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
    AssemblyIdentity, CallKind, EvidenceError, EvidenceRecorder, EvidenceSink,
    InMemoryEvidenceSink, ProviderInvocationFacts,
};
use opi_ai::test_support::{MockProvider, MockResponse, text_response};
use opi_coding_agent::config::{ExecutionRunMode, OpiConfig};
use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
use opi_coding_agent::harness::{CodingHarness, ResumeInfo};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;

struct RelabelProviderSink {
    inner: Arc<InMemoryEvidenceSink>,
}

impl EvidenceSink for RelabelProviderSink {
    fn setup(
        &self,
        binding: &opi_agent::evidence::RuntimeInputBinding,
    ) -> Result<(), EvidenceError> {
        self.inner.setup(binding)
    }

    fn emit(&self, record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        let mut record = record.clone();
        if matches!(
            record.payload,
            opi_agent::evidence::EvidencePayload::Provider(_)
        ) {
            record.kind = CallKind::Tool;
        }
        self.inner.emit(&record)
    }

    fn finalize_artifact(
        &self,
        artifact: &opi_agent::evidence::ArtifactReference,
    ) -> Result<(), EvidenceError> {
        self.inner.finalize_artifact(artifact)
    }

    fn finalize_run(
        &self,
        manifest: &opi_agent::evidence::FinalizedManifest,
    ) -> Result<(), EvidenceError> {
        self.inner.finalize_run(manifest)
    }

    fn abandon_run(
        &self,
        outcome: &opi_agent::evidence::TerminalOutcome,
    ) -> Result<(), EvidenceError> {
        self.inner.abandon_run(outcome)
    }
}

impl EvidenceRecorder for RelabelProviderSink {
    fn records(&self) -> Vec<opi_agent::evidence::EvidenceRecord> {
        self.inner.records()
    }

    fn has_failure(&self) -> bool {
        self.inner.has_failure()
    }

    fn completed_manifest(&self) -> Option<opi_agent::evidence::FinalizedManifest> {
        self.inner.completed_manifest()
    }
}

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

struct RouteReportingProvider {
    models: Vec<opi_ai::provider::ModelInfo>,
    response_model: Option<String>,
}

impl RouteReportingProvider {
    fn new(response_model: Option<&str>) -> Self {
        Self {
            models: vec![model_info("resolved-model")],
            response_model: response_model.map(str::to_owned),
        }
    }
}

impl opi_ai::provider::Provider for RouteReportingProvider {
    fn id(&self) -> &str {
        "route-reporting"
    }

    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        _request: opi_ai::provider::Request,
        _auth: opi_ai::auth::ResolvedAuth,
    ) -> opi_ai::provider::EventStream {
        let mut events = text_response("done");
        for event in &mut events {
            if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
                message.provider = self.id().to_owned();
                message.model = "resolved-model".to_owned();
                message.response_model = self.response_model.clone();
            }
        }
        Box::pin(futures_util::stream::iter(events.into_iter().map(Ok)))
    }
}

/// Build a `CodingHarness` over a mock provider whose evidence capture is bound
/// to `recorder`. The caller keeps the `Arc<InMemoryEvidenceSink>` so it can
/// inspect records and the finalized manifest after the run.
fn build_harness_with_evidence(
    workspace: &Path,
    user: &Path,
    responses: Vec<MockResponse>,
    recorder: Arc<InMemoryEvidenceSink>,
    source: AssemblyIdentity,
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

fn empty_resume_info(workspace: &Path, sessions: &Path, session_id: &str) -> ResumeInfo {
    let path = sessions.join(format!("{session_id}.jsonl"));
    opi_agent::session::SessionWriter::create(
        &path,
        opi_agent::session::SessionHeader::new_for_test(
            session_id.to_owned(),
            "2026-08-20T00:00:00Z".to_owned(),
            workspace.display().to_string(),
            None,
        ),
    )
    .unwrap();
    ResumeInfo {
        path,
        session_id: session_id.to_owned(),
        entries: Vec::new(),
        original_cwd: workspace.to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    }
}

fn resume_snapshot_from_path(path: &Path) -> (ResumeInfo, Vec<AgentMessage>) {
    let (header, entries) = opi_agent::session::SessionReader::read_all(path).unwrap();
    let reconstructed = opi_agent::session_context::reconstruct_context(
        &entries,
        &opi_agent::session::CrashRecovery::default(),
    );
    (
        ResumeInfo {
            path: path.to_path_buf(),
            session_id: header.id,
            entries,
            original_cwd: Path::new(&header.cwd).to_path_buf(),
            diagnostics: Vec::new(),
            recorded_model: None,
            recorded_thinking: None,
        },
        reconstructed.messages,
    )
}

fn assert_sentinel_message_order(messages: &[Message], tool_sentinel: &str, assistant: &str) {
    let tool_index = messages
        .iter()
        .position(|message| {
            matches!(message, Message::ToolResult(_))
                && serde_json::to_string(message)
                    .unwrap()
                    .contains(tool_sentinel)
        })
        .unwrap_or_else(|| {
            panic!(
                "typed tool-result message retains {tool_sentinel}: {}",
                serde_json::to_string(messages).unwrap()
            )
        });
    let assistant_index = messages
        .iter()
        .position(|message| {
            matches!(message, Message::Assistant(_))
                && serde_json::to_string(message).unwrap().contains(assistant)
        })
        .expect("typed terminal assistant message retains its unique sentinel");
    assert!(
        tool_index < assistant_index,
        "the tool result precedes the terminal assistant outcome"
    );
}

async fn assert_a11_failure_preserves_live_persisted_and_reopened_outcome(
    failure: EvidenceError,
    case: &str,
) {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let tool_sentinel = format!("a11-{case}-tool-result-sentinel");
    let assistant_sentinel = format!("a11-{case}-assistant-sentinel");
    let tool_content_control = format!("a11-{case}-tool-content-control");
    let fixture_name = format!("a11-{case}.txt");
    std::fs::write(workspace.path().join(&fixture_name), tool_content_control).unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    sink.inject_failure(failure);
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(opi_ai::test_support::tool_call_response(
                &tool_sentinel,
                "read",
                &serde_json::json!({ "path": fixture_name }).to_string(),
            )),
            MockResponse::Events(text_response(&assistant_sentinel)),
        ],
    );
    let resume = empty_resume_info(workspace.path(), sessions.path(), &format!("a11-{case}"));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .tool_selection(ToolSelection::Allowlist(vec!["read".to_owned()]))
    .resume(resume)
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::SDK_ASSEMBLY.clone(),
    })
    .build();
    let live_events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured_events = live_events.clone();
    harness.subscribe(Box::new(move |event| {
        captured_events.lock().unwrap().push(event.clone());
    }));

    let prompt = format!("a11-{case}-prompt");
    assert!(matches!(
        harness.prompt(&prompt).await,
        Err(opi_agent::loop_types::AgentError::EvidenceFinalization(_))
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());
    let session_path = harness
        .session()
        .expect("the resumed source was adopted or migrated before the run")
        .session_path()
        .to_path_buf();
    let live_messages = live_events
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|event| match event {
            opi_agent::event::AgentEvent::AgentEnd { messages } => Some(
                messages
                    .iter()
                    .filter_map(|message| match message {
                        AgentMessage::Llm(message) => Some(message.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .expect("the public live subscriber observes the terminal AgentEnd");
    assert_sentinel_message_order(&live_messages, &tool_sentinel, &assistant_sentinel);

    let session_bytes = std::fs::read(&session_path).unwrap();
    assert!(!session_bytes.is_empty());
    let session_text = String::from_utf8(session_bytes.clone()).unwrap();
    assert!(session_text.contains(&tool_sentinel));
    assert!(session_text.contains(&assistant_sentinel));
    let (_, entries) = opi_agent::session::SessionReader::read_all(&session_path).unwrap();
    let persisted_messages = entries
        .iter()
        .filter_map(|entry| match entry {
            opi_agent::session::SessionEntry::Message(entry) => Some(entry.message.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_sentinel_message_order(&persisted_messages, &tool_sentinel, &assistant_sentinel);
    drop(harness);

    let recovery_provider = MockProvider::new("mock", vec![text_response("a11-recovery-control")]);
    let recovery_calls = recovery_provider.call_log_handle();
    let (resume, initial_messages) = resume_snapshot_from_path(&session_path);
    let mut reloader = CodingHarness::builder(
        Box::new(recovery_provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .initial_messages(initial_messages)
    .resume(resume)
    .build();
    let reopen_prompt = format!("a11-{case}-reopen-prompt");
    reloader.prompt(&reopen_prompt).await.unwrap();
    let calls = recovery_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_sentinel_message_order(&calls[0].messages, &tool_sentinel, &assistant_sentinel);
    assert!(
        std::fs::read(&session_path)
            .unwrap()
            .starts_with(&session_bytes)
    );
    assert!(sink.completed_manifest().is_none());
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
        opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    );
    let session_binding = harness
        .session()
        .expect("harness created a durable session")
        .runtime_input_binding()
        .clone();
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

    // A healthy run finalizes exactly one strict manifest bound to the same
    // immutable runtime input as its durable session.
    let manifest = sink
        .completed_manifest()
        .expect("a healthy run finalizes a manifest");
    assert!(
        manifest.binding.is_direct(),
        "direct CLI run binds DirectRuntimeInput"
    );
    assert_eq!(manifest.binding, session_binding);
    assert!(matches!(
        manifest.session,
        opi_agent::evidence::SessionBinding::Branch { .. }
    ));
    assert!(matches!(
        manifest.environment.trigger,
        opi_agent::evidence::ExecutionTrigger::Invocation
    ));
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
    let route = manifest
        .provider
        .route()
        .expect("provider-backed run retains route facts");
    assert!(matches!(
        route.actual(),
        opi_agent::evidence::ActualRoute::WireUnknown { route, reason }
            if route.provider_id() == "mock"
                && route.model_id() == "mock-model"
                && *reason == opi_agent::evidence::UnknownReason::NotReported
    ));
}

#[tokio::test]
async fn product_route_evidence_distinguishes_unreported_and_reported_actual_models() {
    let sessions = tempfile::tempdir().unwrap();

    for (index, (reported, expected_actual)) in [
        (None, "resolved-model"),
        (Some("provider-reported-model"), "provider-reported-model"),
    ]
    .into_iter()
    .enumerate()
    {
        let workspace = tempfile::tempdir().unwrap();
        let user = tempfile::tempdir().unwrap();
        let sink = Arc::new(InMemoryEvidenceSink::new());
        let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
        let mut harness = CodingHarness::builder(
            Box::new(RouteReportingProvider::new(reported)),
            "route-reporting:resolved-model".to_owned(),
            OpiConfig::default(),
            workspace.path().to_path_buf(),
            TrustDecision::Trusted,
        )
        .global_config_dir(user.path().to_path_buf())
        .execution_mode(ExecutionRunMode::Interactive)
        .resume(empty_resume_info(
            workspace.path(),
            sessions.path(),
            &format!("route-evidence-{index}"),
        ))
        .evidence(EvidenceBuilderConfig {
            recorder,
            source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
        })
        .build();

        harness.prompt("hello").await.unwrap();

        let manifest = sink.completed_manifest().expect("manifest finalizes");
        let route = manifest.provider.route().expect("provider route applies");
        assert_eq!(route.requested().model_id(), "resolved-model");
        assert_eq!(route.resolved().model_id(), "resolved-model");
        assert!(matches!(
            route.actual(),
            opi_agent::evidence::ActualRoute::WireUnknown { route, reason }
                if route.provider_id() == "route-reporting"
                    && route.model_id() == expected_actual
                    && *reason == opi_agent::evidence::UnknownReason::NotReported
        ));
        if reported.is_some() {
            assert_ne!(
                route.actual(),
                &opi_agent::evidence::ActualRoute::wire_unknown(
                    "route-reporting",
                    route.resolved().model_id(),
                    opi_agent::evidence::UnknownReason::NotReported,
                )
                .unwrap(),
                "a reported actual model is not rewritten to the resolved model"
            );
        }
    }
}

#[tokio::test]
async fn consecutive_product_turns_reproject_the_current_tool_authority() {
    let sessions = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("projection.txt"),
        "projection-control",
    )
    .unwrap();
    let args = serde_json::json!({ "path": "projection.txt" }).to_string();
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(opi_ai::test_support::tool_call_response(
                "projection-read-1",
                "read",
                &args,
            )),
            MockResponse::Events(opi_ai::test_support::tool_call_response(
                "projection-read-2",
                "read",
                &args,
            )),
            MockResponse::Events(text_response("projection complete")),
        ],
    );
    let calls = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "consecutive-tool-projection",
    ))
    .tool_selection(opi_coding_agent::policy::ToolSelection::Allowlist(vec![
        "read".to_owned(),
    ]))
    .build();

    harness
        .prompt("read the control twice")
        .await
        .expect("both real product tool turns complete");

    let requests = calls.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for (index, request) in requests.iter().enumerate() {
        let names = request
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["read"],
            "provider request {index} must reproject the current trusted allowlist"
        );
    }
    assert_eq!(
        requests[1]
            .messages
            .iter()
            .filter(|message| matches!(message, opi_ai::message::Message::ToolResult(_)))
            .count(),
        1
    );
    assert_eq!(
        requests[2]
            .messages
            .iter()
            .filter(|message| matches!(message, opi_ai::message::Message::ToolResult(_)))
            .count(),
        2
    );
}

// ===========================================================================
// P17-EVD-008 / P17-A11 — emission failure preserves outcome, marks incomplete,
// and produces no finalized manifest
// ===========================================================================

#[tokio::test]
async fn evidence_emission_failure_withholds_manifest_and_preserves_outcome() {
    assert_a11_failure_preserves_live_persisted_and_reopened_outcome(
        EvidenceError::Emission {
            detail: "product emission failure".to_owned(),
        },
        "emission",
    )
    .await;
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
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
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
    assert_a11_failure_preserves_live_persisted_and_reopened_outcome(
        EvidenceError::Finalization {
            detail: "capture finalization failure".to_owned(),
        },
        "finalization",
    )
    .await;
}

// ===========================================================================
// P17-EVD-011 / P17-MIG-003 — the product file adapter satisfies the lifecycle
// contract and writes durable evidence.jsonl + manifest.json
// ===========================================================================

struct EvidenceLifecycleFixture {
    binding: opi_agent::evidence::RuntimeInputBinding,
    record: opi_agent::evidence::EvidenceRecord,
    artifact: opi_agent::evidence::ArtifactReference,
    /// The pre-validation candidate behind `manifest`, kept so negative
    /// validation legs can mutate one field and re-validate.
    candidate: opi_agent::evidence::ManifestCandidate,
    manifest: opi_agent::evidence::FinalizedManifest,
}

fn evidence_lifecycle_fixture() -> EvidenceLifecycleFixture {
    use opi_agent::evidence::{
        ActualRoute, ArtifactLocation, ArtifactReference, ArtifactRole, ConfigIdentity,
        ContentDigest, EnvironmentFacts, EvidenceCompleteness, EvidencePayload, EvidenceRecord,
        EvidenceRunObservation, ExecutionTrigger, FinalizationState, IdentityAllocator,
        InputIdentity, ManifestCandidate, ManifestCorrelation, Measurement, MediaType,
        PlatformIdentity, ProvenanceFacts, ProviderEvidenceFacts, ProviderInvocationFacts,
        RequestedRoute, RouteFacts, RouteSelection, RuntimeInputBinding, SensitivityClassification,
        SessionBinding, TerminalOutcome, UnknownReason, UsageFacts, UserPolicyFacts,
    };

    let digest = |nibble: char| {
        ContentDigest::from_hex(nibble.to_string().repeat(64)).expect("valid sha256 hex")
    };
    let binding = RuntimeInputBinding::direct(
        digest('1'),
        opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    );
    let provenance = ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance {
        source: opi_ai::auth::AuthProvenanceSource::Static,
        fallback: opi_ai::auth::AuthFallback::NotAttempted,
    })
    .unwrap();
    let route = RouteFacts::new(
        RequestedRoute::new("contract", "requested").unwrap(),
        RouteSelection::new("contract", "resolved", opi_ai::WireApi::OpenAiCompletions).unwrap(),
        ActualRoute::unknown(UnknownReason::NotReported),
    );
    let mut identities = IdentityAllocator::new();
    let record = EvidenceRecord {
        run: identities.run_id(),
        turn: Some(identities.next_turn()),
        call: identities.next_call(),
        parent: None,
        sequence: identities.next_sequence(),
        kind: CallKind::Provider,
        payload: EvidencePayload::Provider(ProviderEvidenceFacts {
            route: route.clone(),
            provenance: provenance.clone(),
        }),
    };
    let artifact = ArtifactReference {
        role: ArtifactRole::ProviderBody,
        media_type: MediaType::new("application/json"),
        content_digest: digest('2'),
        location: ArtifactLocation::new("artifact://phase17/finalized-control"),
        sensitivity: SensitivityClassification::Public,
        finalization: FinalizationState::Finalized,
    };
    let candidate = ManifestCandidate {
        correlation: ManifestCorrelation {
            run: record.run,
            turn: record.turn,
            call: Some(record.call),
            parent: record.parent,
            sequence: record.sequence,
        },
        outcome: TerminalOutcome::Success,
        session: SessionBinding::NoSession,
        binding: binding.clone(),
        config: ConfigIdentity {
            harness_digest: digest('3'),
            runtime_digest: digest('4'),
            adapter_digest: digest('5'),
            material_digest: digest('6'),
        },
        provider: ProviderInvocationFacts::applicable(route, provenance),
        policy: UserPolicyFacts {
            policy_digest: digest('7'),
            capability: None,
            permission_ref: None,
            permission_scope: None,
            scoped_grant_ref: None,
        },
        input_identity: InputIdentity {
            prompt_digest: digest('8'),
            system_digest: Some(digest('9')),
            tool_schema_digests: vec![digest('a')],
        },
        environment: EnvironmentFacts {
            budget: Measurement::provider_reported(1),
            trigger: ExecutionTrigger::Invocation,
            time: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            platform: PlatformIdentity::new("test-platform"),
        },
        usage: UsageFacts {
            input_tokens: Measurement::provider_reported(1),
            output_tokens: Measurement::provider_reported(1),
        },
        artifacts: vec![artifact.clone()],
        completeness: EvidenceCompleteness::Complete,
    };
    let manifest = candidate
        .clone()
        .validate(EvidenceRunObservation::new(
            &binding,
            std::slice::from_ref(&record),
            std::slice::from_ref(&artifact),
        ))
        .unwrap();
    EvidenceLifecycleFixture {
        binding,
        record,
        artifact,
        candidate,
        manifest,
    }
}

fn assert_complete_recorder_lifecycle(recorder: &dyn EvidenceRecorder) {
    let fixture = evidence_lifecycle_fixture();
    // Before-setup leg: no lifecycle method may accept work before setup is
    // observed — each fails with its typed error and marks the recorder
    // failed, mirroring the core no-op/in-memory conformance contract. The
    // rejected attempts leave the recorder unstarted, so the subsequent
    // setup still opens a clean run.
    assert!(
        recorder.emit(&fixture.record).is_err(),
        "emit before setup must fail closed"
    );
    assert!(
        recorder.finalize_artifact(&fixture.artifact).is_err(),
        "finalize_artifact before setup must fail closed"
    );
    assert!(
        recorder.finalize_run(&fixture.manifest).is_err(),
        "finalize_run before setup must fail closed"
    );
    assert!(
        recorder.has_failure(),
        "the rejected before-setup attempts mark the lifecycle incomplete"
    );

    recorder.setup(&fixture.binding).unwrap();
    recorder.emit(&fixture.record).unwrap();
    recorder.finalize_artifact(&fixture.artifact).unwrap();
    recorder.finalize_run(&fixture.manifest).unwrap();
    assert_eq!(recorder.records().len(), 1);
    assert!(!recorder.has_failure());
    assert_eq!(recorder.completed_manifest(), Some(fixture.manifest));
}

#[test]
fn file_and_in_memory_recorders_share_complete_artifact_lifecycle() {
    let file_root = tempfile::tempdir().unwrap();
    let file = FileEvidenceSink::new(file_root.path());
    let memory = InMemoryEvidenceSink::new();

    assert_complete_recorder_lifecycle(&file);
    assert_complete_recorder_lifecycle(&memory);

    let run_dir = file.completed_run_dirs().into_iter().next().unwrap();
    let evidence_bytes = std::fs::read(run_dir.join("evidence.jsonl")).unwrap();
    let manifest_bytes = std::fs::read(run_dir.join("manifest.json")).unwrap();
    assert!(!evidence_bytes.is_empty());
    assert!(!manifest_bytes.is_empty());
    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(
        manifest_json["artifacts"][0]["location"],
        "artifact://phase17/finalized-control"
    );
    assert_eq!(memory.artifacts(), [evidence_lifecycle_fixture().artifact]);
}

#[test]
fn file_evidence_setup_failure_poisons_recorder_health() {
    use opi_agent::evidence::{ContentDigest, RuntimeInputBinding};

    let blocked_root = tempfile::NamedTempFile::new().unwrap();
    let sink = FileEvidenceSink::new(blocked_root.path());
    let binding = RuntimeInputBinding::direct(
        ContentDigest::from_hex("d".repeat(64)).unwrap(),
        opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    );

    assert!(matches!(
        sink.setup(&binding),
        Err(EvidenceError::Setup { .. })
    ));
    assert!(
        sink.has_failure(),
        "a real file-adapter setup failure makes the lifecycle incomplete"
    );
    assert!(sink.completed_manifest().is_none());
}

#[test]
fn file_evidence_artifact_failure_poisons_health_and_abandon_recovers() {
    let root = tempfile::tempdir().unwrap();
    let sink = FileEvidenceSink::new(root.path());
    let fixture = evidence_lifecycle_fixture();

    sink.setup(&fixture.binding).unwrap();
    sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
        .unwrap();
    assert!(matches!(
        sink.finalize_artifact(&fixture.artifact),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());

    assert_complete_recorder_lifecycle(&sink);
    assert_eq!(sink.completed_run_dirs().len(), 1);
}

#[test]
fn file_evidence_abandon_failure_poisons_health_and_next_setup_recovers() {
    let root = tempfile::tempdir().unwrap();
    let sink = FileEvidenceSink::new(root.path());
    let fixture = evidence_lifecycle_fixture();

    sink.setup(&fixture.binding).unwrap();
    sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
        .unwrap();
    assert!(matches!(
        sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());

    assert_complete_recorder_lifecycle(&sink);
    assert_eq!(sink.completed_run_dirs().len(), 1);
}

fn assert_artifact_observation_mismatch_is_incomplete(recorder: &dyn EvidenceRecorder) {
    let fixture = evidence_lifecycle_fixture();
    recorder.setup(&fixture.binding).unwrap();
    recorder.emit(&fixture.record).unwrap();
    assert!(matches!(
        recorder.finalize_run(&fixture.manifest),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(recorder.has_failure());
    assert!(recorder.completed_manifest().is_none());
    recorder
        .abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
        .unwrap();
    assert_complete_recorder_lifecycle(recorder);
}

#[test]
fn file_and_in_memory_recorders_reject_artifact_observation_mismatch_and_recover() {
    let file_root = tempfile::tempdir().unwrap();
    let file = FileEvidenceSink::new(file_root.path());
    let memory = InMemoryEvidenceSink::new();

    assert_artifact_observation_mismatch_is_incomplete(&file);
    assert_artifact_observation_mismatch_is_incomplete(&memory);
    assert_eq!(file.completed_run_dirs().len(), 1);
}

#[test]
fn poisoned_file_lifecycle_cannot_publish_a_later_matching_manifest() {
    let root = tempfile::tempdir().unwrap();
    let sink = FileEvidenceSink::new(root.path());
    let fixture = evidence_lifecycle_fixture();

    sink.setup(&fixture.binding).unwrap();
    let mut malformed = fixture.record.clone();
    malformed.kind = CallKind::Tool;
    assert!(matches!(
        sink.emit(&malformed),
        Err(EvidenceError::Emission { .. })
    ));
    assert!(sink.has_failure());
    sink.emit(&fixture.record).unwrap();
    sink.finalize_artifact(&fixture.artifact).unwrap();

    assert!(matches!(
        sink.finalize_run(&fixture.manifest),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());
    assert!(sink.completed_run_dirs().is_empty());
    assert!(
        std::fs::read_dir(root.path()).unwrap().all(|entry| !entry
            .unwrap()
            .path()
            .join("manifest.json")
            .exists())
    );

    sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
        .unwrap();
    assert_complete_recorder_lifecycle(&sink);
    assert_eq!(sink.completed_run_dirs().len(), 1);
}

#[test]
fn file_publish_failure_requires_abandon_and_cleans_temporary_manifest() {
    let root = tempfile::tempdir().unwrap();
    let sink = FileEvidenceSink::new(root.path());
    let fixture = evidence_lifecycle_fixture();

    sink.setup(&fixture.binding).unwrap();
    sink.emit(&fixture.record).unwrap();
    sink.finalize_artifact(&fixture.artifact).unwrap();
    let run_dir = std::fs::read_dir(root.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let manifest_path = run_dir.join("manifest.json");
    let temporary_path = run_dir.join(".manifest.json.tmp");
    std::fs::create_dir(&manifest_path).unwrap();

    assert!(matches!(
        sink.finalize_run(&fixture.manifest),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());
    assert!(sink.completed_run_dirs().is_empty());
    assert!(temporary_path.is_file());
    assert!(manifest_path.is_dir());
    assert!(matches!(
        sink.setup(&fixture.binding),
        Err(EvidenceError::Setup { .. })
    ));
    assert!(temporary_path.is_file());

    sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
        .unwrap();
    assert!(!temporary_path.exists());
    assert_complete_recorder_lifecycle(&sink);
    assert_eq!(sink.completed_run_dirs().len(), 1);
}

#[test]
fn completed_file_lifecycle_rejects_artifacts_without_poisoning_published_result() {
    let root = tempfile::tempdir().unwrap();
    let sink = FileEvidenceSink::new(root.path());
    let fixture = evidence_lifecycle_fixture();

    assert_complete_recorder_lifecycle(&sink);
    let published_manifest = sink.completed_manifest().unwrap();
    let run_dir = sink.completed_run_dirs().into_iter().next().unwrap();
    let manifest_path = run_dir.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path).unwrap();

    assert!(matches!(
        sink.finalize_artifact(&fixture.artifact),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(!sink.has_failure());
    assert_eq!(sink.completed_manifest(), Some(published_manifest));
    assert_eq!(sink.completed_run_dirs(), [run_dir]);
    assert_eq!(std::fs::read(manifest_path).unwrap(), manifest_bytes);
}

#[test]
fn concurrent_artifact_and_run_finalization_leave_one_coherent_file_state() {
    let root = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(root.path()));

    for _ in 0..16 {
        let fixture = evidence_lifecycle_fixture();
        sink.setup(&fixture.binding).unwrap();
        sink.emit(&fixture.record).unwrap();
        let completed_before = sink.completed_run_dirs().len();
        let barrier = Arc::new(std::sync::Barrier::new(3));

        let artifact_sink = sink.clone();
        let artifact = fixture.artifact.clone();
        let artifact_barrier = barrier.clone();
        let artifact_task = std::thread::spawn(move || {
            artifact_barrier.wait();
            artifact_sink.finalize_artifact(&artifact)
        });
        let manifest_sink = sink.clone();
        let manifest = fixture.manifest.clone();
        let manifest_barrier = barrier.clone();
        let manifest_task = std::thread::spawn(move || {
            manifest_barrier.wait();
            manifest_sink.finalize_run(&manifest)
        });
        barrier.wait();

        let artifact_result = artifact_task.join().unwrap();
        let manifest_result = manifest_task.join().unwrap();
        if manifest_result.is_ok() {
            assert!(artifact_result.is_ok());
            assert!(!sink.has_failure());
            assert!(sink.completed_manifest().is_some());
            assert_eq!(sink.completed_run_dirs().len(), completed_before + 1);
        } else {
            assert!(sink.has_failure());
            assert!(sink.completed_manifest().is_none());
            assert_eq!(sink.completed_run_dirs().len(), completed_before);
            sink.abandon_run(&opi_agent::evidence::TerminalOutcome::Failed)
                .unwrap();
        }
        let completed_dirs = sink.completed_run_dirs();
        let disk_dirs = std::fs::read_dir(root.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(
            disk_dirs
                .iter()
                .filter(|dir| dir.join("manifest.json").is_file())
                .count(),
            completed_dirs.len()
        );
        assert!(
            disk_dirs
                .iter()
                .all(|dir| !dir.join(".manifest.json.tmp").exists())
        );
    }
}

#[test]
fn file_evidence_sink_writes_records_and_manifest() {
    use opi_agent::evidence::{
        ActualRoute, AuthFallbackFacts, AuthSourceFacts, ConfigIdentity, ContentDigest,
        EvidencePayload, EvidenceRecord, EvidenceRecorder, ExecutionTrigger, IdentityAllocator,
        Measurement, MeasurementOrigin, ProvenanceFacts, ProviderEvidenceFacts, RequestedRoute,
        RouteFacts, RouteSelection, RuntimeInputBinding, SessionBinding, TerminalOutcome,
        UnknownReason, UsageFacts,
    };
    use opi_coding_agent::evidence::{EvidenceCapture, RunDynamicFacts, build_finalized_manifest};

    let digest = |nibble: char| {
        ContentDigest::from_hex(nibble.to_string().repeat(64)).expect("valid sha256 hex")
    };

    let dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(dir.path()));
    let binding = RuntimeInputBinding::direct(
        digest('d'),
        opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    );
    sink.setup(&binding)
        .expect("setup creates the capture file");

    // Emit one provider record (route facts payload, as the loop does).
    let mut ids = IdentityAllocator::new();
    let run = ids.run_id();
    let provenance = ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance {
        source: opi_ai::auth::AuthProvenanceSource::Environment {
            name: "MOCK_API_KEY".to_owned(),
        },
        fallback: opi_ai::auth::AuthFallback::Used {
            from: opi_ai::auth::AuthProvenanceSource::Static,
            to: opi_ai::auth::AuthProvenanceSource::Environment {
                name: "MOCK_API_KEY".to_owned(),
            },
            reason: "configured source unavailable".to_owned(),
        },
    })
    .unwrap();
    let route = RouteFacts::new(
        RequestedRoute::new("mock", "requested-model").unwrap(),
        RouteSelection::new("mock", "resolved-model", opi_ai::WireApi::OpenAiCompletions).unwrap(),
        ActualRoute::wire_unknown("mock", "actual-model", UnknownReason::NotReported).unwrap(),
    );
    let record = EvidenceRecord {
        run,
        turn: Some(ids.next_turn()),
        call: ids.next_call(),
        parent: None,
        sequence: ids.next_sequence(),
        kind: CallKind::Provider,
        payload: EvidencePayload::Provider(ProviderEvidenceFacts {
            route: route.clone(),
            provenance: provenance.clone(),
        }),
    };

    let malformed_dir = tempfile::tempdir().unwrap();
    let malformed_sink = FileEvidenceSink::new(malformed_dir.path());
    malformed_sink.setup(&binding).unwrap();
    let mut malformed_emit = record.clone();
    malformed_emit.kind = CallKind::Tool;
    assert!(matches!(
        malformed_sink.emit(&malformed_emit),
        Err(EvidenceError::Emission { .. })
    ));
    assert!(malformed_sink.has_failure());
    assert!(malformed_sink.records().is_empty());
    let malformed_run_dir = std::fs::read_dir(malformed_dir.path())
        .unwrap()
        .next()
        .expect("setup allocated one run directory")
        .unwrap()
        .path();
    assert!(
        std::fs::read(malformed_run_dir.join("evidence.jsonl"))
            .unwrap()
            .is_empty(),
        "a rejected record must not reach the file"
    );

    sink.emit(&record).expect("emit appends one JSONL record");

    // The recorder sees the emitted record and no failure.
    assert_eq!(sink.records().len(), 1);
    assert!(!sink.has_failure());

    // Finalize a strict manifest built from the capture + recorded route.
    let capture = EvidenceCapture {
        recorder: sink.clone(),
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
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
            permission_ref: None,
            permission_scope: None,
            scoped_grant_ref: None,
        },
        system_digest: Some(digest('f')),
        tool_schema_digests: vec![digest('9')],
        budget: Measurement::Known {
            value: 50,
            origin: MeasurementOrigin::Quota,
        },
    };
    let dynamic = || RunDynamicFacts {
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
        session: SessionBinding::NoSession,
        prompt_digest: digest('e'),
        trigger: ExecutionTrigger::Invocation,
    };
    assert!(matches!(
        build_finalized_manifest(&capture, &[], dynamic()),
        Err(EvidenceError::Finalization { .. })
    ));
    let mut malformed_record = record.clone();
    malformed_record.payload = EvidencePayload::Digest(digest('0'));
    assert!(
        build_finalized_manifest(&capture, &[malformed_record], dynamic()).is_err(),
        "a provider-kind record without typed provider facts fails closed"
    );

    let mut provider_payload_under_tool_kind = record.clone();
    provider_payload_under_tool_kind.kind = CallKind::Tool;
    assert!(
        build_finalized_manifest(
            &capture,
            std::slice::from_ref(&provider_payload_under_tool_kind),
            dynamic(),
        )
        .is_err(),
        "product assembly must reject a Provider payload under a non-Provider kind"
    );

    let malformed_terminal = EvidenceRecord {
        run,
        turn: record.turn,
        call: ids.next_call(),
        parent: None,
        sequence: ids.next_sequence(),
        kind: CallKind::Provider,
        payload: EvidencePayload::Digest(digest('0')),
    };
    let mixed_records = [record.clone(), malformed_terminal];
    assert!(
        build_finalized_manifest(&capture, &mixed_records, dynamic()).is_err(),
        "a valid Provider record cannot hide a malformed Provider-kind terminal record"
    );

    let manifest = build_finalized_manifest(&capture, &sink.records(), dynamic())
        .expect("typed manifest is complete and correlated");

    fn assert_binding_mismatch_marks_failure(
        recorder: Arc<dyn EvidenceRecorder>,
        binding: &RuntimeInputBinding,
        record: &EvidenceRecord,
        manifest: &opi_agent::evidence::FinalizedManifest,
    ) {
        recorder.setup(binding).unwrap();
        recorder.emit(record).unwrap();
        assert!(matches!(
            recorder.finalize_run(manifest),
            Err(EvidenceError::Finalization { .. })
        ));
        assert!(
            recorder.has_failure(),
            "a finalization observation mismatch must poison every recording adapter"
        );
        assert!(recorder.completed_manifest().is_none());
    }

    let mismatched_binding = RuntimeInputBinding::direct(
        digest('8'),
        opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    );
    assert_binding_mismatch_marks_failure(
        Arc::new(InMemoryEvidenceSink::new()),
        &mismatched_binding,
        &record,
        &manifest,
    );
    let mismatch_dir = tempfile::tempdir().unwrap();
    assert_binding_mismatch_marks_failure(
        Arc::new(FileEvidenceSink::new(mismatch_dir.path())),
        &mismatched_binding,
        &record,
        &manifest,
    );

    // P17-PRV-005: the manifest extracts the real non-secret auth provenance
    // from the provider record, never assumes Static.
    assert_eq!(
        manifest.provider.provenance().unwrap().auth_source(),
        &AuthSourceFacts::Environment {
            name: "MOCK_API_KEY".to_owned()
        },
        "manifest must reflect the record's environment auth source"
    );
    assert_eq!(
        manifest.provider.provenance().unwrap().fallback(),
        &AuthFallbackFacts::Used {
            from: AuthSourceFacts::Static,
            to: AuthSourceFacts::Environment {
                name: "MOCK_API_KEY".to_owned()
            },
            stable_reason: opi_agent::evidence::RedactedEvidenceText::new(
                "configured source unavailable"
            ),
        },
        "manifest retains full fallback source, target, and reason"
    );
    assert!(matches!(
        manifest.environment.trigger,
        ExecutionTrigger::Invocation
    ));
    assert!(matches!(manifest.session, SessionBinding::NoSession));
    let retained_route = manifest.provider.route().unwrap();
    assert_eq!(retained_route.requested().model_id(), "requested-model");
    assert_eq!(retained_route.resolved().model_id(), "resolved-model");
    assert!(matches!(
        retained_route.actual(),
        ActualRoute::WireUnknown { route, reason }
            if route.model_id() == "actual-model" && *reason == UnknownReason::NotReported
    ));
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
    assert_eq!(
        parsed["provider"]["route"]["requested"]["model_id"],
        "requested-model"
    );
    assert_eq!(
        parsed["provider"]["route"]["resolved"]["model_id"],
        "resolved-model"
    );
    assert_eq!(
        parsed["provider"]["route"]["actual"]["model_id"],
        "actual-model"
    );
    assert_eq!(
        parsed["provider"]["route"]["actual"]["reason"],
        "not_reported"
    );
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
use opi_agent::authority::{RegisteredTool, RegistrationId, ToolOrigin};
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
        opi_coding_agent::tool_authority::WORKSPACE_WRITE_CAPABILITY.clone(),
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

    let run = agent.prompt("use write").await;
    let messages = run.messages();

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
    assert!(matches!(
        run.into_execution_result(),
        Err(AgentError::EvidenceFinalization(_))
    ));
    assert!(
        sink.completed_manifest().is_none(),
        "abandoned incomplete evidence produces no finalized manifest"
    );
}

#[tokio::test]
async fn malformed_provider_record_rejection_blocks_later_tool_launch() {
    use opi_coding_agent::execution::permission::PermissionPolicy;
    use opi_coding_agent::tool_authority::{EffectiveUserPolicy, ProductToolAuthorizer};

    let count = Arc::new(AtomicUsize::new(0));
    let write_tool = RegisteredTool::new(
        RegistrationId::new("malformed-write"),
        "write".to_owned(),
        ToolOrigin::Builtin,
        opi_coding_agent::tool_authority::WORKSPACE_WRITE_CAPABILITY.clone(),
        opi_ai::message::ToolDef {
            name: "write".to_owned(),
            description: "write".to_owned(),
            input_schema: serde_json::json!({ "type": "object" }),
        },
        Arc::new(RecordingTool {
            count: count.clone(),
        }),
    );
    let policy = Arc::new(EffectiveUserPolicy::build(
        opi_coding_agent::config::ExecutionRunMode::NonInteractive,
        vec!["write".to_owned()],
        true,
        PermissionPolicy::empty(),
        true,
        "project".to_owned(),
        "package".to_owned(),
        "workspace".to_owned(),
    ));
    let authorizer = Arc::new(ProductToolAuthorizer::new(policy, None));
    let inner = Arc::new(InMemoryEvidenceSink::new());
    let sink = Arc::new(RelabelProviderSink {
        inner: inner.clone(),
    });
    let collection = Arc::new(single_route_collection(Box::new(MockProvider::new(
        "mock",
        vec![
            tool_call_response("malformed-call", "write", "{}"),
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
    .unwrap();
    agent.set_evidence_sink(Some(sink as Arc<dyn EvidenceSink>));

    let _ = agent.prompt("use write").await.into_execution_result();

    assert_eq!(RecordingTool::count_of(&count), 0);
    assert!(inner.has_failure());
    assert!(
        inner
            .records()
            .iter()
            .all(|record| record.validate_kind_payload().is_ok()),
        "the malformed Provider payload must not enter the accepted record set"
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
    let sessions = tempfile::tempdir().unwrap();
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
    let call_log = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::NonInteractive)
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "complete-evidence-denial",
    ))
    .tool_selection(opi_coding_agent::policy::ToolSelection::Allowlist(vec![
        "write".to_owned(),
    ]))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();

    let denial_details: Arc<std::sync::Mutex<Option<serde_json::Value>>> =
        Arc::new(std::sync::Mutex::new(None));
    let denial_capture = denial_details.clone();
    harness.subscribe(Box::new(move |event| {
        if let opi_agent::event::AgentEvent::TurnEnd { tool_results, .. } = event {
            for result in tool_results {
                if result.tool_call_id == "c-write" {
                    *denial_capture.lock().unwrap() = result.details.clone();
                }
            }
        }
    }));

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
    // The denial is the authorization boundary's controlled outcome carrying
    // the owning stable code (the lower-level conformance test pins the same
    // code at the ProductToolAuthorizer seam).
    let denial = denial_details.lock().unwrap().clone();
    assert!(
        denial.as_ref().is_some_and(|details| {
            details
                .get("stable_code")
                .is_some_and(|code| code.as_str() == Some("evidence_incomplete"))
        }),
        "the denied write surfaces the evidence_incomplete stable code, got {denial:?}"
    );

    // Projection stays registration-composed (AUT-008): both provider requests
    // of the run still advertise the trusted `write` registration, while its
    // launch is denied at the authorization boundary.
    let calls = call_log.lock().unwrap().clone();
    assert_eq!(
        calls.len(),
        2,
        "the run consumes both scripted responses (fail-open dispatch, fail-closed launch)"
    );
    for (index, request) in calls.iter().enumerate() {
        assert!(
            request.tools.iter().any(|tool| tool.name == "write"),
            "provider request {} must still advertise the registered write tool: {:?}",
            index + 1,
            request
                .tools
                .iter()
                .map(|tool| tool.name.clone())
                .collect::<Vec<_>>()
        );
    }
}

/// A capture-absent harness maps to `complete_evidence_required = false` (the
/// no-op Minimal Runtime): the same write tool call is allowed and executes.
#[tokio::test]
async fn harness_capture_absent_allows_tool() {
    let sessions = tempfile::tempdir().unwrap();
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
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "capture-absent",
    ))
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
async fn harness_compaction_emits_correlated_evidence_record() {
    let sessions = tempfile::tempdir().unwrap();

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
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "manual-compaction-evidence",
    ))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
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

/// Read the resolved provider id from a typed Provider record.
fn resolved_provider_of(record: &opi_agent::evidence::EvidenceRecord) -> Option<String> {
    let facts = match &record.payload {
        opi_agent::evidence::EvidencePayload::Provider(facts) => facts,
        _ => return None,
    };
    Some(facts.route.resolved().provider_id().to_owned())
}

#[tokio::test]
async fn phase17_harness_switches_providers_with_matching_route_evidence() {
    let sessions = tempfile::tempdir().unwrap();

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
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "provider-switch",
    ))
    .extra_routes(vec![(Box::new(beta), static_resolver())])
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();

    harness.prompt("from alpha").await.unwrap();
    let alpha_manifest = sink
        .completed_manifest()
        .expect("alpha run finalizes independently");
    let alpha_session = harness
        .session()
        .expect("alpha run owns a bound session branch");
    let alpha_session_id = alpha_session.session_id().to_owned();
    let alpha_session_path = alpha_session.session_path().to_path_buf();
    assert_eq!(
        alpha_session.runtime_input_binding(),
        &alpha_manifest.binding,
        "the alpha header and manifest share one exact binding"
    );
    harness.set_model_validated("beta:b1".to_owned()).unwrap();
    harness.prompt("from beta").await.unwrap();

    assert_eq!(
        alpha_manifest
            .provider
            .route()
            .unwrap()
            .resolved()
            .provider_id(),
        "alpha"
    );
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
    let route = manifest.provider.route().unwrap();
    assert_eq!(route.resolved().provider_id(), "beta");
    assert!(matches!(
        route.actual(),
        opi_agent::evidence::ActualRoute::WireUnknown { route, reason }
            if route.provider_id() == "mock"
                && route.model_id() == "mock-model"
                && *reason == opi_agent::evidence::UnknownReason::NotReported
    ));
    let beta_session = harness
        .session()
        .expect("beta run adopts a new bound session branch");
    let beta_session_id = beta_session.session_id().to_owned();
    let beta_session_path = beta_session.session_path().to_path_buf();
    assert_ne!(
        alpha_session_id, beta_session_id,
        "a between-run user model change creates a new session branch"
    );
    assert_ne!(
        alpha_manifest.binding, manifest.binding,
        "the beta branch has a new material-input binding"
    );
    assert_eq!(
        beta_session.runtime_input_binding(),
        &manifest.binding,
        "the beta header and manifest share one exact binding"
    );
    let (alpha_header, _) =
        opi_agent::session::SessionReader::read_all(&alpha_session_path).unwrap();
    let (beta_header, _) = opi_agent::session::SessionReader::read_all(&beta_session_path).unwrap();
    assert_eq!(alpha_header.id, alpha_session_id);
    assert_eq!(beta_header.id, beta_session_id);
    assert_eq!(
        beta_header.parent_session.as_deref(),
        Some(alpha_session_id.as_str())
    );
    assert_eq!(
        beta_header.runtime_input_binding.as_ref(),
        Some(&manifest.binding),
        "the persisted beta header retains the exact manifest binding"
    );
    assert_ne!(
        alpha_manifest.config.adapter_digest, manifest.config.adapter_digest,
        "the adapter identity follows the current route"
    );
    assert_eq!(route.requested().provider_id(), "beta");
    assert_eq!(route.requested().model_id(), "b1");
    assert_eq!(
        manifest.provider.provenance().unwrap().auth_source(),
        &opi_agent::evidence::AuthSourceFacts::Static,
        "the static resolver provenance is recorded"
    );
    assert_eq!(
        manifest.provider.provenance().unwrap().fallback(),
        &opi_agent::evidence::AuthFallbackFacts::NotAttempted,
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
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
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
    // One logical provider call emits pre-dispatch + terminal typed facts on
    // the same call identity; the retry does not prepare a second route.
    let provider_records = records
        .iter()
        .filter(|record| record.kind == CallKind::Provider)
        .collect::<Vec<_>>();
    assert_eq!(provider_records.len(), 2);
    assert_eq!(provider_records[0].call, provider_records[1].call);
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
        source: opi_coding_agent::evidence::SDK_ASSEMBLY.clone(),
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
    let _manifest = sink
        .completed_manifest()
        .expect("a complete run finalizes a manifest");
    // A finalized manifest is the private validated wrapper; callers cannot
    // mutate it into a fabricated ActiveSnapshot after validation.
    use opi_agent::evidence::ContentDigest;

    // Invalid config identity cannot be represented: digest construction
    // rejects missing/non-canonical SHA-256 text before manifest assembly.
    assert!(
        ContentDigest::from_hex("").is_err(),
        "a missing config identity must be rejected at construction"
    );

    // A fabricated ActiveSnapshot binding is rejected at manifest validation
    // itself, before any sink sees the candidate: only a trusted Promotion
    // Controller may supply one, and this direct run cannot. The observation
    // carries the SAME swapped binding so the rejection is attributable to the
    // direct-run clause rather than a binding mismatch.
    let fixture = evidence_lifecycle_fixture();
    let mut snapshot_claim = fixture.candidate.clone();
    snapshot_claim.binding = opi_agent::evidence::RuntimeInputBinding::ActiveSnapshot {
        snapshot_ref: opi_agent::evidence::SnapshotRef::new("fabricated-promotion-snapshot"),
    };
    assert!(
        snapshot_claim
            .clone()
            .validate(opi_agent::evidence::EvidenceRunObservation::new(
                &snapshot_claim.binding,
                std::slice::from_ref(&fixture.record),
                std::slice::from_ref(&fixture.artifact),
            ))
            .is_err(),
        "a direct run must not be able to claim an ActiveSnapshot binding at validation"
    );
}

// ===========================================================================
// P17-A09 (phase-exit closure) — the one-run graph includes the TOOL leg
// through the harness-wired sink: provider + retry + a real built-in tool
// execution reconstruct one ordered graph with one shared run identity.
// ===========================================================================

#[tokio::test]
async fn phase17_one_run_graph_includes_tool_execution_record() {
    let sessions = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let evidence = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.retry.max_attempts = 2;
    config.retry.initial_delay_ms = 0;
    config.retry.max_delay_ms = 0;
    config.compaction.threshold_tokens = 0;
    let sink = Arc::new(FileEvidenceSink::new(evidence.path()));
    // One run, four legs: a retryable provider error (Retry record), a real
    // built-in `read` tool call over a workspace file (Tool record), then the
    // terminal text turn (Provider record), followed by threshold compaction
    // during persistence. The read tool and automatic compaction execute
    // through the production harness path before the one prompt manifest is
    // finalized.
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
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "a09-one-run-graph",
    ))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::SDK_ASSEMBLY.clone(),
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
    let compaction_recs = records
        .iter()
        .filter(|r| r.kind == CallKind::Compaction)
        .collect::<Vec<_>>();
    assert!(!provider_recs.is_empty(), "the run emits Provider records");
    for required in [
        CallKind::Provider,
        CallKind::Retry,
        CallKind::Tool,
        CallKind::Compaction,
    ] {
        assert!(records.iter().any(|record| record.kind == required));
    }
    assert!(
        records.iter().all(|record| matches!(
            record.kind,
            CallKind::Provider | CallKind::Retry | CallKind::Tool | CallKind::Compaction
        )),
        "the single prompt graph contains only the four required evidence kinds"
    );
    assert_eq!(
        compaction_recs.len(),
        2,
        "automatic compaction emits one start and one terminal record"
    );
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
        provider_recs.iter().any(|provider| {
            tool_rec.parent == Some(provider.call) && tool_rec.turn == provider.turn
        }),
        "the Tool record is parented to the exact provider request in its turn"
    );
    assert!(
        records.iter().all(|record| record.run == tool_rec.run),
        "Provider, Retry, Tool, and automatic Compaction share one run identity"
    );
    assert_eq!(compaction_recs[0].call, compaction_recs[1].call);
    assert_eq!(compaction_recs[0].turn, compaction_recs[1].turn);
    assert!(matches!(
        &compaction_recs[0].payload,
        opi_agent::evidence::EvidencePayload::Compaction(facts)
            if facts.trigger() == opi_agent::evidence::CompactionTrigger::Threshold
                && facts.outcome().is_none()
    ));
    assert!(matches!(
        &compaction_recs[1].payload,
        opi_agent::evidence::EvidencePayload::Compaction(facts)
            if facts.trigger() == opi_agent::evidence::CompactionTrigger::Threshold
                && facts.outcome() == Some(opi_agent::evidence::CompactionOutcome::Succeeded)
    ));

    let manifest = sink
        .completed_manifest()
        .expect("the four-leg prompt run finalizes one strict manifest");
    let terminal = records.last().expect("non-empty evidence graph");
    assert_eq!(manifest.correlation.run, tool_rec.run);
    assert_eq!(manifest.correlation.turn, terminal.turn);
    assert_eq!(manifest.correlation.call, Some(terminal.call));
    assert_eq!(manifest.correlation.parent, terminal.parent);
    assert_eq!(manifest.correlation.sequence, terminal.sequence);
    assert!(matches!(
        manifest.provider,
        ProviderInvocationFacts::Applicable { .. }
    ));
    assert!(matches!(
        manifest.environment.trigger,
        opi_agent::evidence::ExecutionTrigger::Invocation
    ));
    assert!(matches!(
        manifest.session,
        opi_agent::evidence::SessionBinding::Branch { .. }
    ));

    let run_dirs = sink.completed_run_dirs();
    assert_eq!(run_dirs.len(), 1, "one prompt publishes one immutable run");
    let records_path = run_dirs[0].join("evidence.jsonl");
    let manifest_path = run_dirs[0].join("manifest.json");
    assert!(
        std::fs::metadata(&records_path).unwrap().len() > 0,
        "the finalized evidence graph exists on disk"
    );
    assert!(
        std::fs::metadata(&manifest_path).unwrap().len() > 0,
        "the finalized manifest artifact exists on disk"
    );
    let persisted_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
    let persisted_records = std::fs::read_to_string(records_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(persisted_records.len(), records.len());
    assert!(
        persisted_records
            .iter()
            .all(|record| record["run"] == manifest.correlation.run.to_string())
    );
    assert_eq!(
        persisted_records
            .iter()
            .map(|record| record["call"].clone())
            .collect::<Vec<_>>(),
        records
            .iter()
            .map(|record| serde_json::to_value(record).unwrap()["call"].clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        persisted_manifest["correlation"]["run"],
        manifest.correlation.run.to_string()
    );
    assert_eq!(
        persisted_manifest["environment"]["trigger"]["kind"],
        "invocation"
    );
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
async fn phase17_canaries_stop_before_sink_file_and_manifest() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let evidence_dir = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(evidence_dir.path()));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let prompt_canary = "sk-canary-prompt-AAAAAAAAAAAAAAAAAAAA";
    let argument_canary = "sk-canary-argument-BBBBBBBBBBBBBBBBBBBB";
    let session_path_canary = "sk-canary-session-path-CCCCCCCCCCCCCCCCCCCC";
    let credential_canary = "sk-canary-credential-DDDDDDDDDDDDDDDDDDDD";
    let provider_error_canary = "sk-canary-provider-error-EEEEEEEEEEEEEEEEEEEE";

    // Exercise prompt, tool-argument, explicit session-path, and credential
    // channels through one real harness run. The built-in read may fail for
    // the canary path; that controlled tool result is followed by a terminal
    // provider response and must still never expose the argument.
    let sessions = tempfile::tempdir().unwrap();
    let canary_sessions = sessions.path().join(session_path_canary);
    std::fs::create_dir_all(&canary_sessions).unwrap();
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
    .resume(empty_resume_info(
        workspace.path(),
        &canary_sessions,
        "canary-success",
    ))
    .auth_resolver(Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from(credential_canary),
    )))
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    let prompt = format!("here is a secret {prompt_canary} please ignore");
    let _ = harness.prompt(&prompt).await.expect("run completes");

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
        session_path_canary,
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
            opi_ai::provider::ProviderError::RequestFailed(
                opi_ai::provider::ProviderErrorSummary::from_untrusted(provider_error_canary),
            ),
        )],
    );
    let mut error_harness = CodingHarness::builder(
        Box::new(error_provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .execution_mode(ExecutionRunMode::Interactive)
    .resume(empty_resume_info(
        workspace.path(),
        &canary_sessions,
        "canary-error",
    ))
    .evidence(EvidenceBuilderConfig {
        recorder: error_recorder,
        source: opi_coding_agent::evidence::CLI_ASSEMBLY.clone(),
    })
    .build();
    assert!(
        error_harness
            .prompt("trigger provider error")
            .await
            .is_err()
    );

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

#[tokio::test]
async fn direct_subscriber_redacts_real_compaction_and_session_persist_events() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let sessions = tempfile::tempdir().unwrap();
    let prompt_canary = "prompt/arbitrary/{phase17-direct}";
    let tool_canary = "tool/arbitrary/{phase17-direct}";
    let path_canary = "path-arbitrary-{phase17-direct}";
    let credential_canary = "credential/arbitrary/{phase17-direct}";
    let fixture_name = format!("{path_canary}-fixture.txt").replace('/', "_");
    std::fs::write(
        workspace.path().join(&fixture_name),
        format!("{tool_canary} {path_canary}"),
    )
    .unwrap();
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(opi_ai::test_support::tool_call_response(
                "direct-canary-read",
                "read",
                &serde_json::json!({ "path": fixture_name }).to_string(),
            )),
            MockResponse::Events(text_response("direct-safe-control")),
        ],
    );
    let mut config = OpiConfig::default();
    config.compaction.threshold_tokens = 0;
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        config,
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(empty_resume_info(
        workspace.path(),
        sessions.path(),
        "subscriber-compaction",
    ))
    .tool_selection(opi_coding_agent::policy::ToolSelection::Allowlist(vec![
        "read".to_owned(),
    ]))
    .auth_resolver(Arc::new(opi_ai::auth::StaticAuthResolver::new(
        opi_ai::auth::AuthScheme::ApiKey,
        secrecy::SecretString::from(credential_canary),
    )))
    .build();
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = events.clone();
    harness.subscribe(Box::new(move |event| {
        captured.lock().unwrap().push(event.clone());
    }));
    harness
        .prompt(&format!(
            "{prompt_canary} credential={credential_canary} read the fixture"
        ))
        .await
        .expect("the automatic compaction run completes");

    let compaction_events = events
        .lock()
        .unwrap()
        .iter()
        .filter(|event| {
            matches!(
                event,
                opi_agent::event::AgentEvent::CompactionStart { .. }
                    | opi_agent::event::AgentEvent::CompactionEnd { .. }
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(compaction_events.len(), 2);
    let compaction_json = serde_json::to_string(&compaction_events).unwrap();
    assert!(compaction_json.contains("CompactionStart"));
    assert!(compaction_json.contains("CompactionEnd"));
    assert!(compaction_json.contains("threshold"));
    assert!(compaction_json.contains("tokens_before"));
    assert!(compaction_json.contains("[REDACTED]"));
    for canary in [prompt_canary, tool_canary, path_canary, credential_canary] {
        assert!(
            !compaction_json.contains(canary),
            "subscriber leaked {canary}"
        );
    }

    let persist_root = tempfile::tempdir().unwrap();
    let persist_dir = persist_root
        .path()
        .join(format!("persist-{path_canary}").replace('/', "_"));
    std::fs::create_dir_all(&persist_dir).unwrap();
    let persist_resume = empty_resume_info(workspace.path(), &persist_dir, "subscriber-persist");
    let missing_path = persist_resume.path.clone();
    let mut persist_harness = CodingHarness::builder(
        Box::new(MockProvider::new(
            "mock",
            vec![
                text_response("persist-prepare-control"),
                text_response("persist-safe-control"),
            ],
        )),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .resume(persist_resume)
    .build();
    let persist_events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let persist_capture = persist_events.clone();
    persist_harness.subscribe(Box::new(move |event| {
        persist_capture.lock().unwrap().push(event.clone());
    }));
    persist_harness
        .prompt("prepare bound session for persistence failure")
        .await
        .expect("the initial bound run completes");
    let active_path = persist_harness
        .session()
        .expect("the initial run keeps an active session")
        .session_path()
        .to_path_buf();
    assert_ne!(
        active_path, missing_path,
        "fixture source migrated to its bound child"
    );
    std::fs::remove_file(&active_path).unwrap();
    assert!(matches!(
        persist_harness
            .prompt(&format!("{prompt_canary} credential={credential_canary}"))
            .await,
        Err(opi_agent::loop_types::AgentError::SessionPersist(_))
    ));
    let persist_json = serde_json::to_string(
        &persist_events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    opi_agent::event::AgentEvent::SessionPersistError { .. }
                )
            })
            .cloned()
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert!(persist_json.contains("SessionPersistError"));
    assert!(persist_json.contains("[REDACTED]"));
    assert!(!persist_json.contains(path_canary));
    assert!(!persist_json.contains(&active_path.display().to_string()));
}
