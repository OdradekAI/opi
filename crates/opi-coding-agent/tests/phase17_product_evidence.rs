//! Phase 17 task 17.7 — Reference Product evidence cutover.
//!
//! Drives the production evidence capture path through `CodingHarness::prompt`:
//! the harness binds an [`opi_agent::evidence::EvidenceRecorder`] (here the
//! in-memory oracle), runs the real agent loop — which emits provider/tool/retry
//! records per task 17.6 — and finalizes one strict `DirectRuntimeInput`-bound
//! manifest. The legacy `TraceSink` capture path is replaced by this evidence
//! lifecycle; this test proves the production call site wires setup → emit →
//! finalize through the public harness, not a helper or unit shim.

use std::path::Path;
use std::sync::Arc;

use opi_agent::evidence::{
    AssemblySource, AuthProvenanceSource, CallKind, EvidenceError, EvidenceRecorder, EvidenceSink,
    InMemoryEvidenceSink,
};
use opi_ai::test_support::{MockProvider, MockResponse, text_response};
use opi_coding_agent::config::{ExecutionRunMode, OpiConfig};
use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::project_trust::TrustDecision;

/// Serialize `OPI_SESSIONS_DIR` mutation across this test binary. ONE lock is
/// shared by every test that redirects the process-global sessions dir —
/// per-function statics of the same name are distinct mutexes and would not
/// serialize against each other.
static SESSION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn static_resolver() -> Arc<dyn opi_ai::auth::AuthResolver> {
    Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from("opi-test-auth"),
    ))
}

fn model_info(id: &str) -> opi_ai::provider::ModelInfo {
    opi_ai::provider::ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        opi_ai::ModelCapabilities::new(100_000, 4_096),
    )
}

/// Build a `CodingHarness` over a mock provider whose evidence capture is bound
/// to `recorder`. The caller keeps the `Arc<InMemoryEvidenceSink>` so it can
/// inspect records and the finalized manifest after the run.
fn build_harness_with_evidence(
    workspace: &Path,
    user: &Path,
    responses: Vec<MockResponse>,
    recorder: Arc<InMemoryEvidenceSink>,
    source: AssemblySource,
) -> CodingHarness {
    let provider = MockProvider::new_with_errors("mock", responses);
    let recorder_dyn: Arc<dyn EvidenceRecorder> = recorder;
    CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_string(),
        OpiConfig::default(),
        workspace.to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .evidence(EvidenceBuilderConfig {
        recorder: recorder_dyn,
        source,
    })
    .build()
}

// ===========================================================================
// P17-EVD-003 / P17-EVD-007 / P17-EVD-008 — setup → emit → finalize lifecycle
// ===========================================================================

#[tokio::test]
async fn evidence_capture_finalizes_direct_runtime_input_manifest() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut harness = build_harness_with_evidence(
        workspace.path(),
        user.path(),
        vec![MockResponse::Events(text_response("done"))],
        sink.clone(),
        AssemblySource::Cli,
    );
    let messages = harness.prompt("hello").await.expect("run completes");
    assert!(!messages.is_empty(), "the run produced assistant output");

    // The production loop emitted provider records through the bound sink
    // (P17-EVD-001/EVD-002 via the 17.6 runtime, here proven on the product
    // path).
    let records = sink.records();
    assert!(
        !records.is_empty(),
        "a provider turn emits evidence records"
    );
    assert!(
        records.iter().any(|r| r.kind == CallKind::Provider),
        "a Provider record is emitted through the production path",
    );

    // A healthy run finalizes exactly one strict manifest bound to the direct
    // runtime-input assembly (P17-EVD-003: a direct run must not claim an
    // ActiveSnapshot).
    let manifest = sink
        .completed_manifest()
        .expect("a healthy run finalizes a manifest");
    assert!(
        manifest.binding.is_direct(),
        "direct CLI run binds DirectRuntimeInput"
    );
    // The strict manifest passes its own completeness gate.
    manifest
        .require_complete()
        .expect("finalized manifest passes the strict completeness gate");
    assert!(
        manifest.input_identity.system_digest.is_some(),
        "the exact resolved system instruction is addressed"
    );
    assert!(
        !manifest.input_identity.tool_schema_digests.is_empty(),
        "the exact trusted tool projection is addressed"
    );
    assert!(
        matches!(
            manifest.environment.budget,
            opi_agent::evidence::Measurement::Known {
                origin: opi_agent::evidence::MeasurementOrigin::Quota,
                ..
            }
        ),
        "the configured run budget is distinguished from unknown"
    );
    assert_eq!(manifest.route.actual.provider_id, "mock");
    assert_eq!(manifest.route.actual.model_id, "mock-model");
    assert_eq!(manifest.route.actual_reason, None);
}

// ===========================================================================
// P17-EVD-008 / P17-A11 — emission failure preserves outcome, marks incomplete,
// and produces no finalized manifest
// ===========================================================================

#[tokio::test]
async fn evidence_emission_failure_withholds_manifest_and_preserves_outcome() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    // The first emit (the provider record) fails, advancing health to
    // incomplete. The run's actual execution outcome is still preserved.
    sink.inject_failure(EvidenceError::Emission {
        detail: "product emission failure".to_owned(),
    });
    let mut harness = build_harness_with_evidence(
        workspace.path(),
        user.path(),
        vec![MockResponse::Events(text_response("done"))],
        sink.clone(),
        AssemblySource::Sdk,
    );
    // Provider execution still occurs, but explicit capture is fail-visible at
    // the public operation boundary.
    let result = harness.prompt("hello").await;
    assert!(matches!(
        result,
        Err(opi_agent::loop_types::AgentError::EvidenceFinalization(_))
    ));

    assert!(sink.has_failure(), "the emission failure advanced health");
    assert!(
        sink.completed_manifest().is_none(),
        "an incomplete run produces no finalized manifest"
    );
}

// ===========================================================================
// P17-EVD-007 — explicit capture setup failure aborts the run before its first
// provider or tool call (fail-closed), proven at the production harness boundary
// ===========================================================================

#[tokio::test]
async fn setup_failure_aborts_before_provider_call() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    sink.inject_failure(EvidenceError::Setup {
        detail: "capture setup failure".to_owned(),
    });
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let provider = MockProvider::new("mock", vec![text_response("done")]);
    let call_log = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    let result = harness.prompt("hello").await;
    assert!(
        matches!(
            result,
            Err(opi_agent::loop_types::AgentError::EvidenceSetup(_))
        ),
        "setup failure aborts the run with EvidenceSetup"
    );
    assert_eq!(
        call_log.lock().unwrap().len(),
        0,
        "no provider call fired before setup failure"
    );
}

// ===========================================================================
// P17-EVD-008 / P17-A11 — a finalization failure (not just emission) withholds
// the finalized manifest through the production harness boundary
// ===========================================================================

#[tokio::test]
async fn finalization_failure_withholds_manifest_through_harness() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    sink.inject_failure(EvidenceError::Finalization {
        detail: "capture finalization failure".to_owned(),
    });
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("done")])),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    // The model work completes, but explicit capture failure is observable at
    // the public operation boundary and the manifest is withheld.
    let result = harness.prompt("hello").await;
    assert!(matches!(
        result,
        Err(opi_agent::loop_types::AgentError::EvidenceFinalization(_))
    ));
    assert!(sink.has_failure(), "finalization failure is recorded");
    assert!(
        sink.completed_manifest().is_none(),
        "the finalized manifest is withheld"
    );
}

// ===========================================================================
// P17-EVD-011 / P17-MIG-003 — the product file adapter satisfies the lifecycle
// contract and writes durable evidence.jsonl + manifest.json
// ===========================================================================

#[test]
fn file_evidence_sink_writes_records_and_manifest() {
    use opi_agent::diagnostic::RedactionMode;
    use opi_agent::evidence::{
        ConfigIdentity, ContentDigest, EvidencePayload, EvidenceRecord, EvidenceRecorder,
        IdentityAllocator, Measurement, MeasurementOrigin, RedactedValue, RuntimeInputBinding,
        TerminalOutcome, UnknownReason, UsageFacts,
    };
    use opi_coding_agent::evidence::{EvidenceCapture, RunDynamicFacts, build_finalized_manifest};

    let digest = |nibble: char| {
        ContentDigest::from_hex(nibble.to_string().repeat(64)).expect("valid sha256 hex")
    };

    let dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(dir.path()));
    let binding = RuntimeInputBinding::direct(digest('d'), AssemblySource::Cli);
    sink.setup(&binding)
        .expect("setup creates the capture file");

    // Emit one provider record (route facts payload, as the loop does).
    let mut ids = IdentityAllocator::new();
    let run = ids.run_id();
    let record = EvidenceRecord {
        run,
        turn: Some(ids.next_turn()),
        call: ids.next_call(),
        parent: None,
        sequence: ids.next_sequence(),
        kind: CallKind::Provider,
        payload: EvidencePayload::Structured(RedactedValue::redacted(
            serde_json::json!({
                "requested_route": "mock:mock-model",
                "resolved": { "provider": "mock", "model": "mock-model", "wire": "openai-completions" },
                "actual": { "provider": "mock", "model": "mock-model", "wire": "openai-completions" },
                "auth_source": "environment",
                "fallback": "used",
            }),
            RedactionMode::Summary,
        )),
    };
    sink.emit(&record).expect("emit appends one JSONL record");

    // The recorder sees the emitted record and no failure.
    assert_eq!(sink.records().len(), 1);
    assert!(!sink.has_failure());

    // Finalize a strict manifest built from the capture + recorded route.
    let capture = EvidenceCapture {
        recorder: sink.clone(),
        source: AssemblySource::Cli,
        binding: binding.clone(),
        config: ConfigIdentity {
            harness_digest: digest('1'),
            runtime_digest: digest('2'),
            adapter_digest: digest('a'),
            material_digest: digest('b'),
        },
        policy: opi_agent::evidence::UserPolicyFacts {
            policy_digest: digest('c'),
            capability: None,
        },
        system_digest: Some(digest('f')),
        tool_schema_digests: vec![digest('9')],
        configured_route: opi_agent::evidence::RouteSelection {
            provider_id: "mock".to_owned(),
            model_id: "mock-model".to_owned(),
            wire: opi_ai::WireApi::OpenAiCompletions,
        },
        budget: Measurement::Known {
            value: 50,
            origin: MeasurementOrigin::Quota,
        },
    };
    let dynamic = RunDynamicFacts {
        outcome: TerminalOutcome::Success,
        usage: UsageFacts {
            input_tokens: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            output_tokens: Measurement::Known {
                value: 0,
                origin: MeasurementOrigin::ProviderReported,
            },
        },
        session_branch: None,
        prompt_digest: digest('e'),
        actual_route: Some(opi_agent::evidence::RouteSelection {
            provider_id: "mock".to_owned(),
            model_id: "mock-model".to_owned(),
            wire: opi_ai::WireApi::OpenAiCompletions,
        }),
    };
    let manifest = build_finalized_manifest(&capture, &sink.records(), dynamic);
    manifest.require_complete().expect("manifest is complete");
    // P17-PRV-005: the manifest extracts the real non-secret auth provenance
    // from the provider record, never assumes Static.
    assert_eq!(
        manifest.provenance.auth_source,
        AuthProvenanceSource::Environment,
        "manifest must reflect the record's environment auth source"
    );
    assert_eq!(
        manifest.provenance.fallback_allowed,
        Some(true),
        "manifest must reflect the record's used-fallback classification"
    );
    sink.finalize_run(&manifest)
        .expect("finalize writes manifest.json");

    assert_eq!(
        sink.completed_manifest()
            .as_ref()
            .map(|m| m.binding.clone()),
        Some(binding.clone()),
        "the file recorder returns the finalized manifest",
    );

    // The configured path is a capture root; this run owns one immutable child.
    let first_run_dir = sink
        .completed_run_dirs()
        .into_iter()
        .next()
        .expect("one finalized run directory");
    let first_records_path = first_run_dir.join("evidence.jsonl");
    let first_manifest_path = first_run_dir.join("manifest.json");
    let first_records_bytes = std::fs::read(&first_records_path).unwrap();
    let first_manifest_bytes = std::fs::read(&first_manifest_path).unwrap();
    let records_json = String::from_utf8(first_records_bytes.clone()).unwrap();
    assert!(!records_json.is_empty(), "evidence.jsonl is non-empty");
    let manifest_json = String::from_utf8(first_manifest_bytes.clone()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&manifest_json).unwrap();
    // manifest.json round-trips and carries the parsed route from the record.
    assert_eq!(parsed["route"]["resolved"]["provider_id"], "mock");
    assert_eq!(parsed["binding"]["kind"], "direct_runtime_input");

    // Reusing the same capture root allocates a new child and cannot replace
    // any bytes from the finalized first run.
    sink.setup(&binding).expect("second run setup");
    sink.emit(&record).expect("second run record");
    sink.finalize_run(&manifest).expect("second run finalizes");
    let run_dirs = sink.completed_run_dirs();
    assert_eq!(run_dirs.len(), 2);
    assert_ne!(run_dirs[0], run_dirs[1]);
    assert_eq!(
        std::fs::read(first_records_path).unwrap(),
        first_records_bytes
    );
    assert_eq!(
        std::fs::read(first_manifest_path).unwrap(),
        first_manifest_bytes
    );
}

// ===========================================================================
// P17-EVD-009 / P17-A12 — required-complete-evidence fails closed: after
// evidence becomes incomplete, a stale/unlaunched tool side effect is denied
// ===========================================================================

use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_agent::agent::Agent;
use opi_agent::authority::{Capability, RegisteredTool, RegistrationId, ToolOrigin};
use opi_agent::evidence::CapabilityClass;
use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{Tool, ToolError, ToolResult};
use opi_ai::message::Message;
use opi_ai::test_support::{single_route_collection, tool_call_response};
use tokio_util::sync::CancellationToken;

struct NoopHooks;
impl AgentHooks for NoopHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

struct RecordingTool {
    count: Arc<AtomicUsize>,
}
impl RecordingTool {
    fn count_of(count: &Arc<AtomicUsize>) -> usize {
        count.load(Ordering::SeqCst)
    }
}
impl Tool for RecordingTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: "write".to_owned(),
            description: "recording test tool".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        }
    }
    fn execute(
        &self,
        _call_id: &str,
        _arguments: serde_json::Value,
        _signal: CancellationToken,
        _on_update: Option<opi_agent::tool::UpdateCallback>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let count = self.count.clone();
        Box::pin(async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "executed".to_owned(),
                }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: Vec::new(),
            })
        })
    }
}

#[tokio::test]
async fn required_evidence_failure_denies_unlaunched_tool_side_effect() {
    use opi_coding_agent::execution::permission::PermissionPolicy;
    use opi_coding_agent::tool_authority::{EffectiveUserPolicy, ProductToolAuthorizer};

    // A `write` tool that would be allowed under a healthy, mutating policy.
    let count = Arc::new(AtomicUsize::new(0));
    let write_tool = RegisteredTool::new(
        RegistrationId::new("test-write"),
        "write".to_owned(),
        ToolOrigin::Builtin,
        Capability::Builtin(CapabilityClass::WorkspaceWrite),
        opi_ai::message::ToolDef {
            name: "write".to_owned(),
            description: "write".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        },
        Arc::new(RecordingTool {
            count: count.clone(),
        }),
    );
    // complete_evidence_required = true (capture configured), mutating allowed.
    let policy = Arc::new(EffectiveUserPolicy::build(
        opi_coding_agent::config::ExecutionRunMode::NonInteractive,
        vec!["write".to_owned()],
        true,
        PermissionPolicy::empty(),
        true, // complete evidence required
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    ));
    assert!(policy.complete_evidence_required());
    let authorizer = Arc::new(ProductToolAuthorizer::new(policy, None));

    let sink = Arc::new(InMemoryEvidenceSink::new());
    // The provider record emission fails, advancing health to incomplete before
    // the tool call is authorized.
    sink.inject_failure(EvidenceError::Emission {
        detail: "incomplete evidence".to_owned(),
    });

    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(
        "mock",
        vec![
            tool_call_response("c-write", "write", "{}"),
            text_response("done"),
        ],
    ))));
    let mut agent = Agent::new(
        collection,
        vec![write_tool],
        Some(authorizer),
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(NoopHooks),
    )
    .expect("agent builds");
    agent.set_evidence_sink(Some(sink.clone() as Arc<dyn EvidenceSink>));

    let messages = agent.prompt("use write").await.expect("turn completes");

    // The unlaunched side effect failed closed: zero executions (P17-EVD-009).
    assert_eq!(
        RecordingTool::count_of(&count),
        0,
        "required-complete-evidence must deny the tool after health became incomplete"
    );
    assert!(
        sink.has_failure(),
        "the emission failure advanced evidence health to incomplete"
    );
    // The denial surfaces as a controlled error tool result carrying the owning
    // stable code, without executing the tool.
    let denial = messages.iter().find_map(|m| match m {
        AgentMessage::Llm(Message::ToolResult(tr)) if tr.tool_call_id == "c-write" => Some(tr),
        _ => None,
    });
    let denial = denial.expect("the denied tool call persists a tool result");
    assert!(denial.is_error, "the denial is an error result");
    assert!(
        denial.details.as_ref().is_some_and(|d| {
            d.get("stable_code")
                .is_some_and(|c| c.as_str() == Some("evidence_incomplete"))
        }),
        "the denial carries the evidence_incomplete stable code"
    );
}

// ===========================================================================
// P17-A12 / P17-EVD-009 — the production `complete_evidence_required =
// build_options.evidence.is_some()` mapping is exercised at its real call site
// ===========================================================================

/// A capture-configured harness maps to `complete_evidence_required = true`
/// (harness.rs:1309). After evidence becomes incomplete, a tool launch is
/// denied at the production harness boundary — not a hardcoded helper policy.
#[tokio::test]
async fn harness_complete_evidence_mapping_denies_unlaunched_tool() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    sink.inject_failure(EvidenceError::Emission {
        detail: "product emission failure".to_owned(),
    });
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response(
                "c-write",
                "write",
                r#"{"path": "should_not_exist.txt", "content": "hello"}"#,
            ),
            text_response("done"),
        ],
    );
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::NonInteractive)
    .tool_selection(opi_coding_agent::policy::ToolSelection::Allowlist(vec![
        "write".to_owned(),
    ]))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    let result = harness.prompt("write a file").await;
    assert!(matches!(
        result,
        Err(opi_agent::loop_types::AgentError::EvidenceFinalization(_))
    ));

    // The write was denied at the harness boundary: no file side effect.
    assert!(
        !workspace.path().join("should_not_exist.txt").exists(),
        "the write tool must not execute when evidence is incomplete"
    );
    // The lower-level authority conformance test above pins the stable
    // `evidence_incomplete` result; this boundary test pins the visible capture
    // failure and absence of a side effect.
}

/// A capture-absent harness maps to `complete_evidence_required = false` (the
/// no-op Minimal Runtime): the same write tool call is allowed and executes.
#[tokio::test]
async fn harness_capture_absent_allows_tool() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let provider = MockProvider::new(
        "mock",
        vec![
            opi_ai::test_support::tool_call_response(
                "c-write",
                "write",
                r#"{"path": "should_exist.txt", "content": "hello"}"#,
            ),
            text_response("done"),
        ],
    );
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::NonInteractive)
    .tool_selection(opi_coding_agent::policy::ToolSelection::Allowlist(vec![
        "write".to_owned(),
    ]))
    .build();

    harness.prompt("write a file").await.expect("run completes");

    assert!(
        workspace.path().join("should_exist.txt").exists(),
        "the write tool must execute when capture is absent (complete_evidence_required=false)"
    );
}

// ===========================================================================
// P17-A09 / P17-EVD-002 — harness-side compaction emits a correlated
// Compaction record through the real production call site
// (compact_with_diagnostic → execute_compaction → emit_compaction_evidence)
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn harness_compaction_emits_correlated_evidence_record() {
    let _lock = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let sessions = tempfile::tempdir().unwrap();
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", sessions.path()) };

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("done")])),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    harness.prompt("first prompt").await.unwrap();
    let provider_run = sink
        .records()
        .into_iter()
        .find(|record| record.kind == CallKind::Provider)
        .expect("the prompt run emits Provider evidence")
        .run;
    let prompt_manifest = sink
        .completed_manifest()
        .expect("the prompt run finalizes before manual compaction");

    // Manual compaction is a new public operation and therefore owns a new
    // immutable evidence run rather than appending after the prompt manifest.
    let result = harness
        .compact(opi_agent::session_event::CompactionReason::Manual)
        .expect("manual compaction succeeds");
    assert!(result.is_some(), "manual compaction produces output");

    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };

    let records = sink.records();
    assert!(
        records
            .iter()
            .all(|record| record.kind != CallKind::Provider),
        "the manual compaction run cannot mutate the finalized prompt run"
    );
    let compaction = records
        .iter()
        .find(|r| r.kind == CallKind::Compaction)
        .expect("a Compaction record is emitted");
    assert_ne!(compaction.run, provider_run);
    let compaction_manifest = sink
        .completed_manifest()
        .expect("manual compaction finalizes its own run");
    assert_ne!(
        compaction_manifest.correlation.run,
        prompt_manifest.correlation.run
    );
}

// ===========================================================================
// P17-A01 — one session selects provider A then B; evidence retains matching
// requested/resolved/actual route facts for each
// ===========================================================================

/// Read the resolved provider id from a Provider record's route payload.
fn resolved_provider_of(record: &opi_agent::evidence::EvidenceRecord) -> Option<String> {
    let payload = match &record.payload {
        opi_agent::evidence::EvidencePayload::Structured(rv) => rv.as_value(),
        _ => return None,
    };
    payload["resolved"]["provider"].as_str().map(str::to_owned)
}

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in awaited dispatch.
async fn phase17_harness_switches_providers_with_matching_route_evidence() {
    let _lock = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let sessions = tempfile::tempdir().unwrap();
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", sessions.path()) };

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![model_info("a1")],
        vec![text_response("alpha-response")],
    );
    let beta = MockProvider::new_with_models(
        "beta",
        vec![model_info("b1")],
        vec![text_response("beta-response")],
    );
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(alpha),
        "alpha:a1".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .extra_routes(vec![(Box::new(beta), static_resolver())])
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();

    harness.prompt("from alpha").await.unwrap();
    let alpha_manifest = sink
        .completed_manifest()
        .expect("alpha run finalizes independently");
    harness.set_model_validated("beta:b1".to_owned()).unwrap();
    harness.prompt("from beta").await.unwrap();

    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };

    assert_eq!(alpha_manifest.route.resolved.provider_id, "alpha");
    let records = sink.records();
    assert_eq!(
        records
            .iter()
            .find(|record| record.kind == CallKind::Provider)
            .and_then(resolved_provider_of)
            .as_deref(),
        Some("beta"),
        "the recorder contains only the current immutable run"
    );

    // The finalized manifest carries the terminal (beta) route facts and the
    // resolved auth provenance (P17-PRV-005: requested/resolved/actual and
    // auth-source/fallback are all distinguishable).
    let manifest = sink.completed_manifest().expect("a finalized manifest");
    assert_eq!(manifest.route.resolved.provider_id, "beta");
    assert_eq!(manifest.route.actual.provider_id, "mock");
    assert_eq!(manifest.route.actual.model_id, "mock-model");
    assert_eq!(manifest.route.actual_reason, None);
    assert_ne!(
        alpha_manifest.binding, manifest.binding,
        "a model switch must produce a fresh current-run binding"
    );
    assert_ne!(
        alpha_manifest.config.adapter_digest, manifest.config.adapter_digest,
        "the adapter identity follows the current route"
    );
    assert_eq!(manifest.route.requested.provider_id, "beta");
    assert_eq!(manifest.route.requested.model_id, "b1");
    assert_eq!(
        manifest.provenance.auth_source,
        AuthProvenanceSource::Static,
        "the static resolver provenance is recorded"
    );
    assert_eq!(
        manifest.provenance.fallback_allowed,
        Some(false),
        "no auth fallback was attempted"
    );
}

// ===========================================================================
// P17-A03 — a retrying provider call keeps one route and a parented retry chain
// ===========================================================================

#[tokio::test]
async fn phase17_retry_keeps_route_parent_and_terminal_evidence() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.retry.max_attempts = 2;
    config.retry.initial_delay_ms = 0;
    config.retry.max_delay_ms = 0;
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(opi_ai::provider::ProviderError::RateLimited {
                retry_after_ms: Some(0),
            }),
            MockResponse::Events(text_response("recovered")),
        ],
    );
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        config,
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    harness.prompt("go").await.unwrap();

    let records = sink.records();
    let provider_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record");
    let retry_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Retry)
        .expect("a Retry record");
    // The retry is parented to the one provider call and follows it.
    assert_eq!(retry_rec.parent, Some(provider_rec.call));
    assert!(provider_rec.sequence < retry_rec.sequence);
    // Exactly one provider route across the retry (prepare_call not re-invoked).
    assert_eq!(
        records
            .iter()
            .filter(|r| r.kind == CallKind::Provider)
            .count(),
        1
    );
}

// ===========================================================================
// P17-A09 — a run reconstructs one ordered graph and the strict manifest
// rejects a missing/wrong runtime-input binding
// ===========================================================================

#[tokio::test]
async fn phase17_complete_run_reconstructs_graph_and_rejects_missing_bindings() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.retry.max_attempts = 2;
    config.retry.initial_delay_ms = 0;
    config.retry.max_delay_ms = 0;
    let sink = Arc::new(InMemoryEvidenceSink::new());
    // A retryable provider error forces a Retry record; the recovered text turn
    // emits the Provider record, so the run reconstructs a multi-record ordered
    // graph (P17-EVD-001) instead of the single-record tautology a text-only
    // turn would produce.
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(opi_ai::provider::ProviderError::RateLimited {
                retry_after_ms: Some(0),
            }),
            MockResponse::Events(text_response("done")),
        ],
    );
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        config,
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Sdk,
    })
    .build();
    harness.prompt("hello").await.unwrap();

    let records = sink.records();
    // The ordering check is non-vacuous only over >= 2 records.
    assert!(
        records.len() >= 2,
        "a retried turn reconstructs a multi-record graph, got {} record(s)",
        records.len()
    );
    assert!(
        records.iter().any(|r| r.kind == CallKind::Provider),
        "a Provider record is present"
    );
    assert!(
        records.iter().any(|r| r.kind == CallKind::Retry),
        "a Retry record is present"
    );
    // The Retry record is parented to the one Provider call (P17-EVD-002), so
    // the graph reconstructs the call correlation rather than an emission-order
    // tautology (sequence is minted monotonically at emission).
    let provider_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record");
    let retry_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Retry)
        .expect("a Retry record");
    assert_eq!(
        retry_rec.parent,
        Some(provider_rec.call),
        "the Retry record is parented to the Provider record"
    );
    assert!(
        records.iter().all(|r| r.run == records[0].run),
        "all records share one run identity"
    );
    let manifest = sink
        .completed_manifest()
        .expect("a complete run finalizes a manifest");
    manifest
        .require_complete()
        .expect("the finalized manifest passes the strict gate");

    // P17-EVD-003 / INV-008: a direct run must not claim an ActiveSnapshot. A
    // manifest bound to ActiveSnapshot is rejected by the strict gate.
    use opi_agent::evidence::{ContentDigest, RuntimeInputBinding, SnapshotRef};
    let mut snapshot_manifest = manifest.clone();
    snapshot_manifest.binding = RuntimeInputBinding::ActiveSnapshot {
        snapshot_ref: SnapshotRef::new("fabricated"),
    };
    assert!(
        snapshot_manifest.require_complete().is_err(),
        "an ActiveSnapshot binding must be rejected for a direct run"
    );

    // Invalid config identity cannot be represented: digest construction
    // rejects missing/non-canonical SHA-256 text before manifest assembly.
    assert!(
        ContentDigest::from_hex("").is_err(),
        "a missing config identity must be rejected at construction"
    );
}

// ===========================================================================
// P17-A09 (phase-exit closure) — the one-run graph includes the TOOL leg
// through the harness-wired sink: provider + retry + a real built-in tool
// execution reconstruct one ordered graph with one shared run identity.
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_one_run_graph_includes_tool_execution_record() {
    // The compaction leg persists session state, so isolate the process-global
    // sessions dir for the whole test (mirrors the compaction test below).
    let _lock = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let sessions = tempfile::tempdir().unwrap();
    // SAFETY: test-only env var mutation serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", sessions.path()) };

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.retry.max_attempts = 2;
    config.retry.initial_delay_ms = 0;
    config.retry.max_delay_ms = 0;
    let sink = Arc::new(InMemoryEvidenceSink::new());
    // One run, three legs: a retryable provider error (Retry record), a real
    // built-in `read` tool call over a workspace file (Tool record), then the
    // terminal text turn (Provider record). The read tool executes through the
    // production harness tool path, so the Tool evidence record is emitted by
    // the product assembly, not just the 17.6 substrate.
    let target = workspace.path().join("graph-fixture.txt");
    std::fs::write(&target, "phase17 one-run graph fixture\n").unwrap();
    let tool_call = opi_ai::test_support::tool_call_response(
        "tc-1",
        "read",
        &format!(
            r#"{{"path":"{}","offset":1,"limit":5}}"#,
            target.display().to_string().replace('\\', "/")
        ),
    );
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(opi_ai::provider::ProviderError::RateLimited {
                retry_after_ms: Some(0),
            }),
            MockResponse::Events(tool_call),
            MockResponse::Events(text_response("done")),
        ],
    );
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        config,
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Sdk,
    })
    .build();
    harness.prompt("read the fixture file").await.unwrap();

    let records = sink.records();
    let provider_recs: Vec<_> = records
        .iter()
        .filter(|r| r.kind == CallKind::Provider)
        .collect();
    let retry_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Retry)
        .expect("a Retry record (the retryable error leg)");
    let tool_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Tool)
        .expect("a Tool record for the real built-in read execution");
    assert!(!provider_recs.is_empty(), "the run emits Provider records");
    // The retry is parented to the provider call it retries.
    assert_eq!(
        retry_rec.parent,
        Some(provider_recs[0].call),
        "the Retry record is parented to the Provider record"
    );
    // The tool call is correlated into the same run graph: same run identity,
    // its own call id, and parented into the provider turn that requested it.
    assert_eq!(
        tool_rec.run, retry_rec.run,
        "Provider, Retry, and Tool records share one run identity"
    );
    assert_ne!(
        tool_rec.call, retry_rec.call,
        "the Tool record has its own call identity"
    );
    assert!(
        tool_rec.parent.is_some(),
        "the Tool record is parented into the call graph"
    );

    let prompt_run = tool_rec.run;
    let prompt_manifest = sink
        .completed_manifest()
        .expect("the tool-bearing prompt run is finalized");

    // Manual compaction is a second immutable run; it never appends to the
    // already finalized provider/retry/tool graph.
    let compacted = harness
        .compact(opi_agent::session_event::CompactionReason::Manual)
        .expect("manual compaction succeeds");
    assert!(compacted.is_some(), "compaction produces output");
    let records = sink.records();
    assert!(
        records
            .iter()
            .all(|record| record.kind == CallKind::Compaction),
        "the recorder contains only the current manual-compaction run"
    );
    let compaction_rec = records
        .iter()
        .find(|r| r.kind == CallKind::Compaction)
        .expect("a Compaction record in the independent run");
    assert_ne!(compaction_rec.run, prompt_run);

    // Both independent operations finalize strict manifests.
    let compaction_manifest = sink
        .completed_manifest()
        .expect("the compaction run finalizes a manifest");
    compaction_manifest
        .require_complete()
        .expect("the compaction run passes the strict completeness gate");
    assert_ne!(
        prompt_manifest.correlation.run,
        compaction_manifest.correlation.run
    );

    // SAFETY: serialized by SESSION_TEST_LOCK.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };
}

// ===========================================================================
// P17-EVD-006 (phase-exit closure) — the DEFAULT Reference Product assembly
// (no explicit capture configuration) runs the no-op Minimal Runtime: no
// evidence is minted or written anywhere; capture exists only when explicitly
// configured (--trace / SDK recorder), never merely because an adapter or
// consumer exists.
// ===========================================================================

#[tokio::test]
async fn phase17_default_harness_emits_no_evidence() {
    fn walk(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, found);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name == "evidence.jsonl" || name == "manifest.json")
            {
                found.push(path);
            }
        }
    }

    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new("mock", vec![text_response("done")])),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .build();
    let messages = harness.prompt("hello").await.expect("run completes");
    assert!(!messages.is_empty(), "the default run completes normally");

    // No evidence artifacts anywhere under the isolated user-config tree: the
    // default assembly wires no recorder, so nothing is minted or written.
    let mut found = Vec::new();
    walk(user.path(), &mut found);
    assert!(
        found.is_empty(),
        "the default (no-capture) assembly writes no evidence artifacts: {found:?}"
    );
}

// ===========================================================================
// P17-A10 / P17-FAL-004 — canary secrets stop before the sink, file, and
// manifest (producer-boundary redaction; evidence never carries raw content)
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serializes the environment-channel canary across awaited dispatch.
async fn phase17_canaries_stop_before_sink_file_and_manifest() {
    let _lock = SESSION_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let evidence_dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(evidence_dir.path()));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let prompt_canary = "sk-canary-prompt-AAAAAAAAAAAAAAAAAAAA";
    let argument_canary = "sk-canary-argument-BBBBBBBBBBBBBBBBBBBB";
    let environment_canary = "sk-canary-environment-CCCCCCCCCCCCCCCCCCCC";
    let credential_canary = "sk-canary-credential-DDDDDDDDDDDDDDDDDDDD";
    let provider_error_canary = "sk-canary-provider-error-EEEEEEEEEEEEEEEEEEEE";

    // Exercise prompt, tool-argument, process-environment, and credential
    // channels through one real harness run. The built-in read may fail for
    // the canary path; that controlled tool result is followed by a terminal
    // provider response and must still never expose the argument.
    let sessions = tempfile::tempdir().unwrap();
    let canary_sessions = sessions.path().join(environment_canary);
    std::fs::create_dir_all(&canary_sessions).unwrap();
    // SAFETY: process-global mutation is serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", &canary_sessions) };
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(tool_call_response(
                "canary-read",
                "read",
                &serde_json::json!({ "path": argument_canary }).to_string(),
            )),
            MockResponse::Events(text_response("done")),
        ],
    );
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .auth_resolver(Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from(credential_canary),
    )))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    let prompt = format!("here is a secret {prompt_canary} please ignore");
    let _ = harness.prompt(&prompt).await.expect("run completes");
    // SAFETY: paired with the serialized override above.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };

    // The run emitted records and finalized a manifest, and the prompt was
    // digested into the manifest (never stored raw): these make the absence
    // assertions below non-vacuous.
    assert!(!sink.records().is_empty(), "the run emits evidence records");
    let manifest = sink
        .completed_manifest()
        .expect("a complete run finalizes a manifest");
    assert!(
        !manifest.input_identity.prompt_digest.as_hex().is_empty(),
        "the prompt is digested into the manifest, never stored raw"
    );

    let input_canaries = [
        prompt_canary,
        argument_canary,
        environment_canary,
        credential_canary,
    ];

    // The in-memory records carry no raw input-channel canary.
    let records_json = serde_json::to_string(&sink.records()).unwrap();
    for canary in input_canaries {
        assert!(
            !records_json.contains(canary),
            "{canary} leaked into evidence records: {records_json}"
        );
    }
    // The durable evidence.jsonl and artifact metadata in manifest.json carry
    // no raw canary from any exercised input channel.
    let run_dir = sink
        .completed_run_dirs()
        .into_iter()
        .next()
        .expect("one immutable trace run directory");
    let evidence_file = std::fs::read_to_string(run_dir.join("evidence.jsonl")).unwrap();
    let manifest_file = std::fs::read_to_string(run_dir.join("manifest.json")).unwrap();
    for canary in input_canaries {
        assert!(
            !evidence_file.contains(canary),
            "{canary} leaked into evidence.jsonl: {evidence_file}"
        );
        assert!(
            !manifest_file.contains(canary),
            "{canary} leaked into manifest artifact metadata: {manifest_file}"
        );
    }

    // Provider-error text is also diagnostic input. Drive a second real run
    // that fails at that boundary, then inspect both its diagnostic evidence
    // and finalized artifact metadata.
    let error_sink = Arc::new(FileEvidenceSink::new(evidence_dir.path()));
    let error_recorder: Arc<dyn EvidenceRecorder> = error_sink.clone();
    let error_provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::Error(
            opi_ai::provider::ProviderError::RequestFailed(provider_error_canary.to_owned()),
        )],
    );
    // SAFETY: process-global mutation is serialized by SESSION_TEST_LOCK.
    unsafe { std::env::set_var("OPI_SESSIONS_DIR", &canary_sessions) };
    let mut error_harness = CodingHarness::builder(
        Box::new(error_provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .evidence(EvidenceBuilderConfig {
        recorder: error_recorder,
        source: AssemblySource::Cli,
    })
    .build();
    assert!(
        error_harness
            .prompt("trigger provider error")
            .await
            .is_err()
    );
    // SAFETY: paired with the serialized override above.
    unsafe { std::env::remove_var("OPI_SESSIONS_DIR") };

    let error_records = serde_json::to_string(&error_sink.records()).unwrap();
    let error_run_dir = error_sink
        .completed_run_dirs()
        .into_iter()
        .next()
        .expect("failed run still finalizes one immutable trace directory");
    let error_file = std::fs::read_to_string(error_run_dir.join("evidence.jsonl")).unwrap();
    let error_manifest = std::fs::read_to_string(error_run_dir.join("manifest.json")).unwrap();
    for output in [&error_records, &error_file, &error_manifest] {
        assert!(
            !output.contains(provider_error_canary),
            "provider-error/diagnostic canary leaked: {output}"
        );
    }
}
