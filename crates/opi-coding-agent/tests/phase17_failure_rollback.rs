//! Phase 17 task 17.9 — composed failure-boundary matrix and rollback fixtures.
//!
//! P17-FAL-001/002/003 are COMPOSED from the owner-task slices (route/auth
//! 17.1+17.5, next-turn 17.2, evidence 17.3+17.6+17.7, authority 17.4; timeout,
//! cancellation, queue closure, and partial effects are owned by Phase 8/12;
//! cleanup-unknown is owned by opi-protocol/opi-sandbox Phase 15/16). This file
//! does not first implement boundary semantics: it proves the classes stay
//! caller-distinguishable, that a failure stops before every later boundary in
//! the fixed order, and that cancellation/evidence failure are not converted
//! into success or an unqualified denial on the assembled product path.
//!
//! P17-RBK-003/004 rollback fixtures: a Phase 17 run's session and evidence
//! bytes survive a subsequent load untouched, and the effective user policy is
//! not widened by run activity or session persistence.

mod common;

#[path = "common/phase17.rs"]
mod phase17;

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::stream;
use opi_agent::authority::{
    AuthorizationDecision, Capability, InvocationContext, RegisteredTool, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer,
};
use opi_agent::evidence::{
    AssemblySource, CapabilityClass, EvidenceError, EvidenceHealth, EvidenceRecorder, EvidenceSink,
    IdentityAllocator, InMemoryEvidenceSink,
};
use opi_agent::hooks::{AgentHooks, BeforeToolCallContext, BeforeToolCallResult};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig, ModelSelection};
use opi_agent::message::AgentMessage;
use opi_agent::{Agent, Tool, ToolError, ToolResult};
use opi_ai::auth::ResolvedAuth;
use opi_ai::message::Message;
use opi_ai::provider::{EventStream, Provider, ProviderError, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use tokio_util::sync::CancellationToken;

use opi_coding_agent::config::{ExecutionRunMode, OpiConfig};
use opi_coding_agent::evidence::{EvidenceBuilderConfig, FileEvidenceSink};
use opi_coding_agent::execution::permission::PermissionPolicy;
use opi_coding_agent::harness::{CodingHarness, ResumeInfo};
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::rpc::{RpcCommand, RpcRunner};
use opi_coding_agent::tool_authority::{EffectiveUserPolicy, ProductToolAuthorizer};

// ===========================================================================
// P17-FAL-001 — every composed boundary exposes caller-distinguishable typed
// failure classes (no string parsing). Each row names its owner-task slice.
// ===========================================================================

#[tokio::test]
async fn requested_resume_open_failure_is_visible_before_provider_dispatch() {
    let workspace = tempfile::tempdir().unwrap();
    let missing_session = workspace.path().join("missing-session.jsonl");
    let provider = MockProvider::new("mock", vec![text_response("must not run")]);
    let calls = provider.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".to_owned(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .resume(ResumeInfo {
        path: missing_session,
        session_id: "requested-session".to_owned(),
        entries: Vec::new(),
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: Vec::new(),
        recorded_model: None,
        recorded_thinking: None,
    })
    .build();

    let result = harness.prompt("continue the requested session").await;
    assert!(matches!(result, Err(AgentError::SessionResume(_))));
    assert_eq!(calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn phase17_failure_boundaries_expose_distinguishable_typed_classes() {
    fn request(model: &str, cancel: CancellationToken) -> Request {
        Request {
            model: model.to_owned(),
            system: None,
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: None,
            temperature: None,
            thinking: opi_ai::provider::ThinkingConfig::default(),
            stop_sequences: Vec::new(),
            metadata: None,
            cancel,
            timeout: None,
            extra_headers: Vec::new(),
            cache_retention: opi_ai::provider::CacheRetention::None,
            session_id: None,
        }
    }

    // A lookup-only provider reaches the real Agent construction gate and is
    // rejected as a typed non-dispatchable route.
    let mut lookup_registry = opi_ai::ProviderRegistry::new();
    lookup_registry
        .register_provider(Box::new(MockProvider::new(
            "lookup",
            vec![text_response("must not dispatch")],
        )))
        .unwrap();
    let agent = Agent::new(
        Arc::new(opi_ai::ProviderCollection::from_registry(lookup_registry)),
        Vec::new(),
        None,
        "lookup:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoopHooks),
    );
    assert!(matches!(
        agent,
        Err(AgentError::RouteNotDispatchable { provider }) if provider == "lookup"
    ));

    // A pre-cancelled real collection call exits before resolver/provider work
    // with its distinct cancellation class.
    let collection = single_route_collection(Box::new(MockProvider::new(
        "mock",
        vec![text_response("must not dispatch")],
    )));
    let cancel = CancellationToken::new();
    cancel.cancel();
    let cancelled = collection
        .prepare_call("mock:mock-model", request("mock:mock-model", cancel))
        .await;
    assert!(matches!(
        cancelled,
        Err(opi_ai::provider_collection::CollectionError::CallCancelled)
    ));

    // Complete-state validation returns a typed registry class and preserves
    // the prior state rather than partially applying the candidate.
    let mut state_agent = Agent::new(
        Arc::new(single_route_collection(Box::new(MockProvider::new(
            "mock",
            vec![text_response("unused")],
        )))),
        Vec::new(),
        None,
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(NoopHooks),
    )
    .unwrap();
    let prior = state_agent.state_snapshot();
    let mut invalid = prior.clone();
    invalid.model_selection = ModelSelection::new("missing", "model");
    assert!(matches!(
        state_agent.replace_state(invalid),
        Err(AgentError::InvalidNextTurnCandidate(detail)) if detail.contains("missing")
    ));
    assert_eq!(
        state_agent.state_snapshot().model_selection,
        prior.model_selection
    );

    // A naturally invalid file-capture root reaches the production adapter's
    // setup boundary and yields EvidenceError::Setup.
    let blocked = tempfile::NamedTempFile::new().unwrap();
    let file_sink = FileEvidenceSink::new(blocked.path());
    let binding = opi_agent::evidence::RuntimeInputBinding::direct(
        opi_agent::evidence::ContentDigest::from_hex("0".repeat(64)).unwrap(),
        AssemblySource::Cli,
    );
    assert!(matches!(
        file_sink.setup(&binding),
        Err(EvidenceError::Setup { .. })
    ));

    // Tool-authorizer Err, emission/finalization, cleanup-unknown, cancellation,
    // and bounded proxy overflow are exercised through their real owner paths
    // by phase17_tool_authority, phase17_product_evidence,
    // protocol_conformance, the cancellation test below, and streaming_proxy.
}

// ===========================================================================
// P17-FAL-002 — a failure stops processing before every later boundary in the
// fixed runtime order (route -> next-turn -> authority -> execution).
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_failure_precedence_stops_before_later_boundaries() {
    // This test prompts a CodingHarness (the evidence-setup row), so it must
    // hold the session lock and isolate OPI_SESSIONS_DIR: the env var is
    // process-global, and without the lock a concurrent lock-holder's
    // directory setting would capture this run's session writes.
    let _lock = phase17::session_lock();
    let isolated_sessions = tempfile::tempdir().unwrap();
    phase17::set_sessions_dir(isolated_sessions.path());

    // --- Route selection failure (owner 17.1/17.5 slice) --------------------
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let alpha = MockProvider::new_with_models(
        "alpha",
        vec![phase17::model_info("alpha-model")],
        Vec::new(),
    );
    let alpha_calls = alpha.call_log_handle();
    let mut harness = CodingHarness::builder(
        Box::new(alpha),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(user.path().to_path_buf())
    .build();
    let rejected = harness.set_model_validated("ghost:ghost-model".to_owned());
    assert!(
        rejected.is_err(),
        "an unknown route is a typed selection failure before any dispatch"
    );
    assert_eq!(
        harness.model_spec(),
        "alpha:alpha-model",
        "the failed selection leaves the applied route unchanged"
    );
    assert!(
        alpha_calls.lock().unwrap().is_empty(),
        "no provider dispatch happened while selecting"
    );

    // --- Authority denial precedes execution (owner 17.4 slice) -------------
    let count = Arc::new(AtomicUsize::new(0));
    let registration = counted_registered("write", count.clone());
    let responses = vec![
        tool_call_response("tc-1", "write", "{}"),
        text_response("denied path"),
    ];
    let provider = MockProvider::new_with_models("mock", vec![phase17::model_info("m")], responses);
    let collection = Arc::new(single_route_collection(Box::new(provider)));
    let mut agent = Agent::new(
        collection,
        vec![registration],
        Some(Arc::new(common::DenyingAuthorizer) as Arc<dyn ToolAuthorizer>),
        "mock:m".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            ..Default::default()
        },
        Box::new(NoopHooks),
    )
    .expect("agent builds");
    let outcome = agent.prompt("use write").await;
    assert!(
        outcome.is_ok(),
        "the denial becomes a redacted tool result, not a run failure: {outcome:?}"
    );
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "a denied authorization executes the tool zero times"
    );

    // --- Evidence setup failure precedes the provider call (owner 17.7) ------
    let ws = tempfile::tempdir().unwrap();
    let usr = tempfile::tempdir().unwrap();
    let blocked = tempfile::tempdir().unwrap();
    // An existing FILE occupies the evidence directory path: setup must fail
    // closed before the run dispatches the provider (17.8's MIG-004 fixture).
    let legacy_file = blocked.path().join("legacy-trace.jsonl");
    std::fs::write(&legacy_file, b"{\"schema_version\":1,\"records\":[]}\n").unwrap();
    let sink = Arc::new(FileEvidenceSink::new(legacy_file.clone()));
    let recorder: Arc<dyn EvidenceRecorder> = sink;
    let dispatched = MockProvider::new_with_models(
        "alpha",
        vec![phase17::model_info("alpha-model")],
        vec![text_response("never reached")],
    );
    let dispatched_calls = dispatched.call_log_handle();
    let mut blocked_harness = CodingHarness::builder(
        Box::new(dispatched),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        ws.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(usr.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    let blocked_result = blocked_harness.prompt("must not dispatch").await;
    assert!(
        blocked_result.is_err(),
        "evidence setup failure fails the run closed: {blocked_result:?}"
    );
    assert!(
        dispatched_calls.lock().unwrap().is_empty(),
        "evidence setup failure stops before the provider boundary"
    );
    assert_eq!(
        std::fs::read(&legacy_file).unwrap(),
        b"{\"schema_version\":1,\"records\":[]}\n",
        "the blocking legacy file is untouched"
    );
    phase17::clear_sessions_dir();
}

// ===========================================================================
// P17-FAL-003 — cancellation and evidence failure are not converted into
// success or an unqualified denial.
// ===========================================================================

/// A provider whose stream never yields: dispatch is observable, completion is
/// not, so a cancelled run provably terminated via cancellation (it cannot
/// have completed).
struct PendingProvider {
    calls: Arc<AtomicUsize>,
}

impl Provider for PendingProvider {
    fn id(&self) -> &str {
        "pending"
    }
    fn models(&self) -> &[opi_ai::provider::ModelInfo] {
        static MODELS: std::sync::OnceLock<Vec<opi_ai::provider::ModelInfo>> =
            std::sync::OnceLock::new();
        MODELS
            .get_or_init(|| vec![phase17::model_info("pending-model")])
            .as_slice()
    }
    fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(stream::pending::<Result<AssistantStreamEvent, ProviderError>>())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_cancellation_and_evidence_failure_are_not_converted_to_success() {
    // Both halves prompt a CodingHarness (directly and through the RPC
    // runner), so hold the session lock and isolate OPI_SESSIONS_DIR for the
    // whole test (process-global env; see FAL-002 above).
    let _lock = phase17::session_lock();
    let isolated_sessions = tempfile::tempdir().unwrap();
    phase17::set_sessions_dir(isolated_sessions.path());

    // --- Evidence emission failure preserves the outcome, withholds the
    // manifest (owner 17.7 A11 slice, recomposed on the product path). -------
    let ws = tempfile::tempdir().unwrap();
    let usr = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    sink.inject_failure(EvidenceError::Emission {
        detail: "capstone emission failure".to_owned(),
    });
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "alpha",
            vec![phase17::model_info("alpha-model")],
            vec![text_response("outcome preserved")],
        )),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        ws.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(usr.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    let messages = harness.prompt("still runs").await;
    assert!(
        matches!(messages, Err(AgentError::EvidenceFinalization(_))),
        "explicit capture failure is visible after execution: {messages:?}"
    );
    assert!(
        sink.completed_manifest().is_none(),
        "incomplete evidence is never finalized into a manifest (not converted to success)"
    );

    // --- Cancellation through the RPC product seam: a dispatched-but-pending
    // provider stream, aborted mid-flight, terminates the run without
    // fabricating completion. ----------------------------------------------
    let ws2 = tempfile::tempdir().unwrap();
    let pending = PendingProvider {
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let calls = pending.calls.clone();
    let mut rpc = RpcRunner::new_with_trace(
        Box::new(pending),
        "pending:pending-model".into(),
        OpiConfig::default(),
        ws2.path().to_path_buf(),
        /* allow_mutating */ true,
        opi_coding_agent::policy::ToolSelection::Disabled,
        None,
        Vec::new(),
        None,
        TrustDecision::Trusted,
    )
    .expect("rpc runner constructs");
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(async move { rpc.run_with_channels(command_rx, output_tx).await });
    command_tx
        .send(RpcCommand::prompt {
            id: Some("cancel-1".into()),
            message: "will be cancelled".into(),
        })
        .unwrap();
    // Wait until the provider has been dispatched, then abort mid-stream.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while calls.load(Ordering::SeqCst) == 0 && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the provider was dispatched"
    );
    command_tx.send(RpcCommand::abort { id: None }).unwrap();
    // Drain until the terminal AgentEnd (bounded), tracking any assistant text.
    let mut saw_agent_end = false;
    let mut saw_assistant_text = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let Ok(maybe) = tokio::time::timeout(Duration::from_millis(200), output_rx.recv()).await
        else {
            break;
        };
        let Some(line) = maybe else { break };
        if line["type"] == "AgentEnd" {
            saw_agent_end = true;
            break;
        }
        if line["type"] == "MessageUpdate" || line["type"] == "MessageStart" {
            saw_assistant_text = true;
        }
    }
    assert!(
        saw_agent_end,
        "the aborted run terminates with one AgentEnd, not a hang"
    );
    assert!(
        !saw_assistant_text,
        "a cancelled stream is not converted into a completed assistant message"
    );
    command_tx.send(RpcCommand::quit { id: None }).unwrap();
    let _ = task.await;
    phase17::clear_sessions_dir();
}

// ===========================================================================
// P17-RBK-003 — a rollback preserves user sessions and new evidence artifacts:
// the files a Phase 17 run wrote survive a subsequent load byte-identically.
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_rollback_preserves_session_and_evidence_bytes() {
    let sessions = tempfile::tempdir().unwrap();
    let _lock = phase17::session_lock();
    phase17::set_sessions_dir(sessions.path());

    let evidence_dir = tempfile::tempdir().unwrap();
    let ws = tempfile::tempdir().unwrap();
    let usr = tempfile::tempdir().unwrap();
    let sink = Arc::new(FileEvidenceSink::new(evidence_dir.path().to_path_buf()));
    let recorder: Arc<dyn EvidenceRecorder> = sink.clone();
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "alpha",
            vec![phase17::model_info("alpha-model")],
            vec![text_response("rollback fixture")],
        )),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        ws.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(usr.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    harness
        .prompt("create artifacts")
        .await
        .expect("the fixture run completes");

    let session_path = phase17::newest_jsonl(sessions.path()).expect("a session was persisted");
    let session_before = std::fs::read(&session_path).unwrap();
    let run_dir = sink
        .completed_run_dirs()
        .into_iter()
        .next()
        .expect("one immutable evidence run");
    let evidence_path = run_dir.join("evidence.jsonl");
    let manifest_path = run_dir.join("manifest.json");
    let evidence_before = std::fs::read(&evidence_path).unwrap();
    let manifest_before = std::fs::read(&manifest_path).unwrap();
    assert!(!evidence_before.is_empty() && !manifest_before.is_empty());

    // A subsequent Phase 17 runtime loading the same session and evidence
    // leaves every artifact byte-identical (a rollback preserves them; it does
    // not rewrite, down-convert, or delete them).
    let session_id = session_path
        .file_stem()
        .and_then(|s| s.to_str())
        .expect("session id from file name");
    let ws2 = tempfile::tempdir().unwrap();
    let usr2 = tempfile::tempdir().unwrap();
    let mut reloader = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "alpha",
            vec![phase17::model_info("alpha-model")],
            Vec::new(),
        )),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        ws2.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(usr2.path().to_path_buf())
    .build();
    reloader
        .resume_session_id(session_id)
        .expect("the session reloads");

    assert_eq!(
        std::fs::read(&session_path).unwrap(),
        session_before,
        "the session file is byte-identical after reload"
    );
    assert_eq!(
        std::fs::read(&evidence_path).unwrap(),
        evidence_before,
        "the evidence file is byte-identical after reload"
    );
    assert_eq!(
        std::fs::read(&manifest_path).unwrap(),
        manifest_before,
        "the finalized manifest is byte-identical after reload"
    );

    phase17::clear_sessions_dir();
}

// ===========================================================================
// P17-RBK-004 — rollback cannot widen User Policy: the effective policy is
// input-addressed and unchanged by run activity or session persistence, and a
// policy that denies a mutating capability still denies it afterward.
// ===========================================================================

#[tokio::test]
#[allow(clippy::await_holding_lock)] // serialized OPI_SESSIONS_DIR mutation; not re-acquired in the awaited dispatch.
async fn phase17_rollback_does_not_widen_user_policy() {
    let build_policy = || {
        EffectiveUserPolicy::build(
            ExecutionRunMode::Interactive,
            vec!["read".to_owned()],
            /* mutating not allowed */ false,
            PermissionPolicy::empty(),
            /* complete evidence not required */ false,
            "project".to_owned(),
            "package".to_owned(),
            "workspace".to_owned(),
        )
    };
    let before = build_policy();
    let digest_before = before.digest().to_owned();

    // Run activity: a real harness prompt (which persists a session and emits
    // evidence) must not feed back into the policy inputs.
    let sessions = tempfile::tempdir().unwrap();
    let _lock = phase17::session_lock();
    phase17::set_sessions_dir(sessions.path());
    let ws = tempfile::tempdir().unwrap();
    let usr = tempfile::tempdir().unwrap();
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let recorder: Arc<dyn EvidenceRecorder> = sink;
    let mut harness = CodingHarness::builder(
        Box::new(MockProvider::new_with_models(
            "alpha",
            vec![phase17::model_info("alpha-model")],
            vec![text_response("policy fixture")],
        )),
        "alpha:alpha-model".into(),
        OpiConfig::default(),
        ws.path().to_path_buf(),
        TrustDecision::Trusted,
    )
    .global_config_dir(usr.path().to_path_buf())
    .evidence(EvidenceBuilderConfig {
        recorder,
        source: AssemblySource::Cli,
    })
    .build();
    harness
        .prompt("try to escalate permissions please")
        .await
        .expect("the run completes");
    phase17::clear_sessions_dir();

    let after = build_policy();
    assert_eq!(
        after.digest(),
        digest_before,
        "the effective policy digest is unchanged by run activity"
    );

    // The denial outcome is unchanged: a WorkspaceWrite request that the
    // pre-run policy denies is still denied by the post-run policy (no
    // widening of capability permission or scope).
    let mut identities = IdentityAllocator::new();
    let request = ToolAuthorizationRequest {
        run_id: identities.run_id(),
        turn_id: identities.next_turn(),
        call_id: identities.next_call(),
        invocation_context: InvocationContext::NoSession,
        registration_id: RegistrationId::new("test-write"),
        capability: Capability::Builtin(CapabilityClass::WorkspaceWrite),
        arguments: serde_json::json!({}),
        evidence_health: EvidenceHealth::healthy(),
    };
    let decide = |policy: &EffectiveUserPolicy| {
        let authorizer = ProductToolAuthorizer::new(Arc::new(policy.clone()), None);
        futures_util::FutureExt::now_or_never(
            authorizer.authorize(request.clone(), CancellationToken::new()),
        )
        .expect("the product authorizer decides synchronously")
        .expect("authorization succeeds")
    };
    let before_decision = decide(&before);
    let after_decision = decide(&after);
    assert!(matches!(
        before_decision,
        AuthorizationDecision::Deny { .. }
    ));
    assert!(matches!(after_decision, AuthorizationDecision::Deny { .. }));
}

// ---------------------------------------------------------------------------
// Local doubles (mirroring task 17.4's local copies).
// ---------------------------------------------------------------------------

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

/// A tool that counts executions; used to prove zero executions on denial.
struct CountingTool {
    name: String,
    count: Arc<AtomicUsize>,
}

impl Tool for CountingTool {
    fn definition(&self) -> opi_ai::message::ToolDef {
        opi_ai::message::ToolDef {
            name: self.name.clone(),
            description: "counting test tool".to_owned(),
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

/// A registered tool with a shared execution counter, for zero-execution
/// assertions on denial. ONE tool instance owns both the definition and the
/// implementation, so the counter observes the registered implementation.
fn counted_registered(name: &str, count: Arc<AtomicUsize>) -> RegisteredTool {
    let tool = CountingTool {
        name: name.to_owned(),
        count,
    };
    let definition = tool.definition();
    RegisteredTool::new(
        RegistrationId::new(format!("test-{name}")),
        name.to_owned(),
        opi_agent::authority::ToolOrigin::Builtin,
        Capability::Builtin(CapabilityClass::WorkspaceWrite),
        definition,
        Arc::from(tool),
    )
}
