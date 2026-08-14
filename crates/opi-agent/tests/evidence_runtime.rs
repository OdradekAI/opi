//! Phase 17 task 17.6 — Agent evidence runtime over stable identities.
//!
//! Drives the additive `EvidenceSink` lifecycle into the Agent/`agent_loop`
//! seam through the PUBLIC Agent: a bound `InMemoryEvidenceSink` reconstructs
//! the ordered call graph (run/turn/call/parent/sequence correlation + CallKind)
//! as the loop runs. This is the Agent Core evidence substrate — the legacy
//! `TraceSink` capture path was removed in Phase 17.7 and no file
//! adapter/exporter/Eval is introduced here.
//!
//! Slices: (1) a provider-only turn emits a correlated Provider record; (2) a
//! tool call emits a Tool record parented to the provider call; (3) a retry
//! emits a Retry record parented to the provider call, over the one immutable
//! route resolved once per turn.

mod common;

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_agent::agent::Agent;
use opi_agent::evidence::{
    CallKind, EvidenceError, EvidencePayload, EvidenceSink, InMemoryEvidenceSink,
};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::Message;
use opi_ai::message::{AssistantContent, AssistantMessage, OutputContent, ToolCall, ToolDef};
use opi_ai::provider::ProviderError;
use opi_ai::retry::RetryConfig;
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::test_support::{
    MockProvider, MockResponse, single_route_collection, text_response, tool_call_response,
};
use tokio_util::sync::CancellationToken;

struct FailOnEmission {
    emissions: AtomicUsize,
    fail_on: usize,
}

impl EvidenceSink for FailOnEmission {
    fn setup(
        &self,
        _binding: &opi_agent::evidence::RuntimeInputBinding,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn emit(&self, _record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        let emission = self.emissions.fetch_add(1, Ordering::SeqCst) + 1;
        if emission == self.fail_on {
            return Err(EvidenceError::Emission {
                detail: "injected ordered emission failure".to_owned(),
            });
        }
        Ok(())
    }

    fn finalize_artifact(
        &self,
        _artifact: &opi_agent::evidence::ArtifactReference,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn finalize_run(
        &self,
        _manifest: &opi_agent::evidence::FinalizedManifest,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }
}

struct SequentialCountingTool {
    name: String,
    executions: Arc<AtomicUsize>,
}

impl Tool for SequentialCountingTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.name.clone(),
            description: "sequential counter".to_owned(),
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
        let executions = self.executions.clone();
        Box::pin(async move {
            executions.fetch_add(1, Ordering::SeqCst);
            Ok(ToolResult {
                content: vec![OutputContent::Text { text: "ok".into() }],
                details: None,
                is_error: false,
                terminate: false,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

fn two_tool_response() -> Vec<AssistantStreamEvent> {
    let mut message = AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::OpenAi,
        provider: "mock".into(),
        model: "mock-model".into(),
        response_model: None,
        response_id: None,
        usage: Usage::unknown(),
        stop_reason: StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
    };
    let calls = [
        ToolCall {
            id: "c1".into(),
            name: "first".into(),
            arguments: "{}".into(),
        },
        ToolCall {
            id: "c2".into(),
            name: "second".into(),
            arguments: "{}".into(),
        },
    ];
    message.content = calls
        .iter()
        .cloned()
        .map(|tool_call| AssistantContent::ToolCall { tool_call })
        .collect();
    vec![
        AssistantStreamEvent::Start {
            partial: message.clone(),
        },
        AssistantStreamEvent::Done {
            reason: StopReason::ToolUse,
            message,
        },
    ]
}

struct TestHooks;
impl AgentHooks for TestHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|m| match m {
                AgentMessage::Llm(m) => Some(m.clone()),
                _ => None,
            })
            .collect())
    }
    fn should_stop_after_turn(
        &self,
        _ctx: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn std::future::Future<Output = bool> + Send>> {
        Box::pin(async { false })
    }
    fn before_tool_call(
        &self,
        _ctx: BeforeToolCallContext,
    ) -> Pin<Box<dyn std::future::Future<Output = BeforeToolCallResult> + Send>> {
        Box::pin(async { BeforeToolCallResult::Continue })
    }
}

/// Build an Agent bound to `sink` over a single-route mock collection. `responses`
/// is the scripted provider response sequence (events and/or errors); `retry`
/// configures provider retry. The sink is installed through the public
/// `Agent::set_evidence_sink` seam (default is capture-disabled no-op); `sink`
/// is also held by the caller for record inspection.
fn make_agent(
    responses: Vec<MockResponse>,
    registrations: Vec<opi_agent::authority::RegisteredTool>,
    authorizer: Option<Arc<dyn opi_agent::authority::ToolAuthorizer>>,
    retry: Option<RetryConfig>,
    sink: Arc<InMemoryEvidenceSink>,
) -> Agent {
    let provider = MockProvider::new_with_errors("mock", responses);
    let collection = Arc::new(single_route_collection(Box::new(provider)));
    let mut agent = Agent::new(
        collection,
        registrations,
        authorizer,
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            retry,
        },
        Box::new(TestHooks),
    )
    .expect("agent builds");
    let dyn_sink: Arc<dyn EvidenceSink> = sink;
    agent.set_evidence_sink(Some(dyn_sink));
    agent
}

/// Zero-delay retry config so retry tests run without sleeping.
fn fast_retry() -> RetryConfig {
    RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 0,
        max_delay_ms: 0,
    }
}

// ===========================================================================
// P17-EVD-001 / P17-EVD-002 — provider turn emits a correlated record
// ===========================================================================

#[tokio::test]
async fn provider_turn_emits_correlated_provider_record() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let _ = agent.prompt("go").await;

    let records = sink.records();
    assert!(
        !records.is_empty(),
        "a provider turn must emit evidence records"
    );

    // Every record shares one stable, non-reused run identity (P17-EVD-001).
    let run = records[0].run;
    assert!(
        records.iter().all(|r| r.run == run),
        "all records share one run identity"
    );

    // The provider dispatch emits a Provider-kind record carrying its turn
    // (P17-EVD-002).
    let provider = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record is emitted for the provider dispatch");
    assert!(
        provider.turn.is_some(),
        "the provider record carries its turn identity"
    );

    // The provider record attaches the requested and resolved route facts
    // (P17-EVD-002: requested/resolved/actual route correlation).
    let value = match &provider.payload {
        EvidencePayload::Structured(rv) => rv.as_value(),
        _ => panic!("provider payload must be structured"),
    };
    let obj = value
        .as_object()
        .expect("provider payload is a JSON object");
    assert_eq!(obj["requested_route"], "mock:mock-model", "requested route");
    assert_eq!(
        obj["resolved"]["provider"], "mock",
        "resolved provider route"
    );
    assert_eq!(
        obj["resolved"]["model"], "mock-model",
        "resolved model route"
    );
    // The actual route is provider-reported and only known after the response;
    // the pre-dispatch record marks it unknown (empty) rather than copying
    // `resolved` (P17-PRV-005 / P17-EVD-004).
    assert_eq!(obj["actual"]["provider"], "", "actual provider is unknown");
    assert_eq!(obj["actual"]["model"], "", "actual model is unknown");
    assert_eq!(
        obj["actual_reason"], "not_reported",
        "the empty actual carries a typed reason"
    );

    // Sequence is strictly monotonic across emission order (P17-EVD-001).
    assert!(
        records.windows(2).all(|w| w[0].sequence < w[1].sequence),
        "sequence is strictly monotonic"
    );
}

// ===========================================================================
// P17-EVD-002 — a tool call emits a Tool record after the Provider record
// ===========================================================================

#[tokio::test]
async fn tool_turn_emits_provider_then_tool_records_in_order() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let count = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![Box::new(common::RecordingTool::new(
        "mytool",
        count.clone(),
    ))]);
    let mut agent = make_agent(
        vec![
            MockResponse::Events(tool_call_response("c1", "mytool", "{}")),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(common::permissive_authorizer()),
        None,
        sink.clone(),
    );
    let _ = agent.prompt("go").await;

    assert_eq!(
        common::RecordingTool::count_of(&count),
        1,
        "the tool executes once through the real call site"
    );

    let records = sink.records();
    let provider = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record is emitted");
    let tool = records
        .iter()
        .find(|r| r.kind == CallKind::Tool)
        .expect("a Tool record is emitted for the tool call");

    assert!(
        provider.sequence < tool.sequence,
        "the provider record precedes the tool record"
    );
    assert_eq!(provider.turn, tool.turn, "provider and tool share the turn");
    assert_ne!(
        provider.call, tool.call,
        "the tool has its own call identity"
    );
    assert_eq!(
        tool.parent,
        Some(provider.call),
        "the tool record's parent is the provider call"
    );

    // The Tool record carries the authorization identity facts the 17.4 chain
    // resolved: registration id, capability, and the Allow/Deny decision.
    let tool_value = match &tool.payload {
        EvidencePayload::Structured(rv) => rv.as_value(),
        _ => panic!("tool payload must be structured"),
    };
    let tool_obj = tool_value
        .as_object()
        .expect("tool payload is a JSON object");
    assert_eq!(
        tool_obj["registration_id"], "test-mytool",
        "registration id attached"
    );
    assert!(
        tool_obj["capability"].is_string(),
        "capability label attached"
    );
    assert!(
        tool_obj["decision"] == "allow",
        "the Allow decision is attached"
    );
}

// ===========================================================================
// P17-EVD-002 — a retry emits a Retry record parented to the provider call,
// over the one immutable route resolved once per turn
// ===========================================================================

#[tokio::test]
async fn retry_emits_retry_record_parented_to_provider_call() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(0),
            }),
            MockResponse::Events(text_response("recovered")),
        ],
        Vec::new(),
        None,
        Some(fast_retry()),
        sink.clone(),
    );
    let _ = agent.prompt("go").await;

    let records = sink.records();
    let provider = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record is emitted");
    let retry = records
        .iter()
        .find(|r| r.kind == CallKind::Retry)
        .expect("a Retry record is emitted for the retry attempt");

    // The retry attempt is parented to the provider call and follows it in the
    // call graph.
    assert_eq!(
        retry.parent,
        Some(provider.call),
        "the retry record's parent is the provider call"
    );
    assert!(
        provider.sequence < retry.sequence,
        "the provider record precedes the retry record"
    );
    // Exactly one Provider record: the route was resolved once per turn and
    // reused across the retry (prepare_call is not re-invoked), proving the one
    // immutable provider route across retries.
    assert_eq!(
        records
            .iter()
            .filter(|r| r.kind == CallKind::Provider)
            .count(),
        1,
        "exactly one Provider record — one immutable route across retries"
    );
}

// ===========================================================================
// P17-EVD-008 + DoD: an emission failure advances versioned EvidenceHealth,
// and the advanced generation is copied live into the next authorization
// ===========================================================================

#[tokio::test]
async fn emission_failure_advances_health_copied_into_authorization() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    // Inject an emission failure so the provider record's emission advances the
    // run's evidence health to Incomplete at generation 1.
    sink.inject_failure(EvidenceError::Emission {
        detail: "test emission failure".to_owned(),
    });
    let count = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![Box::new(common::RecordingTool::new(
        "mytool",
        count.clone(),
    ))]);
    // StaleGenerationAuthorizer stamps the INITIAL generation (0) on every Allow.
    let mut agent = make_agent(
        vec![
            MockResponse::Events(tool_call_response("c1", "mytool", "{}")),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(Arc::new(common::StaleGenerationAuthorizer::default())),
        None,
        sink.clone(),
    );
    let _ = agent.prompt("go").await;

    assert!(
        sink.has_failure(),
        "the injected emission failure was observed by the sink"
    );
    // The provider emission advanced health to generation 1. The authorizer
    // stamps generation 0; the freshness gate detects the mismatch against the
    // LIVE generation 1 and denies with zero execution. Had health remained the
    // frozen run-start generation 0, the stale Allow would match and the tool
    // would execute — so the denial proves the advanced generation reached the
    // authorization boundary. The decision is made against a per-request COPY
    // of health (authorize_and_verify takes EvidenceHealth by value), so
    // authorization never shares mutable health with the sink.
    assert_eq!(
        common::RecordingTool::count_of(&count),
        0,
        "the advanced health generation (1) reaches authorization and mismatches the stale Allow (0)"
    );
}

#[tokio::test]
async fn sequential_tool_outcome_evidence_precedes_the_next_authorization() {
    let executions = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![
        Box::new(SequentialCountingTool {
            name: "first".into(),
            executions: executions.clone(),
        }),
        Box::new(SequentialCountingTool {
            name: "second".into(),
            executions: executions.clone(),
        }),
    ]);
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(two_tool_response()),
            MockResponse::Events(text_response("done")),
        ],
    );
    let collection = Arc::new(single_route_collection(Box::new(provider)));
    let mut agent = Agent::new(
        collection,
        registrations,
        Some(Arc::new(common::StaleGenerationAuthorizer::default())),
        "mock:mock-model".into(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig::default(),
        Box::new(TestHooks),
    )
    .unwrap();
    agent.set_evidence_sink(Some(Arc::new(FailOnEmission {
        emissions: AtomicUsize::new(0),
        fail_on: 3,
    })));

    let _ = agent.prompt("go").await;

    assert_eq!(
        executions.load(Ordering::SeqCst),
        1,
        "the second sequential tool must not launch after the first outcome's evidence failure"
    );
}

// ===========================================================================
// P17-EVD-002 — post-run compaction emits a correlated Compaction record
// ===========================================================================

#[tokio::test]
async fn compaction_emits_correlated_evidence_record() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let _ = agent.prompt("go").await;

    // Post-run compaction (harness-side) mints a correlated Compaction record
    // in the same run.
    agent
        .emit_compaction_evidence("manual")
        .expect("compaction evidence emits");

    let records = sink.records();
    let provider = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record is emitted");
    let compaction = records
        .iter()
        .find(|r| r.kind == CallKind::Compaction)
        .expect("a Compaction record is emitted");

    // The Compaction record shares the run identity and follows the provider
    // record in sequence order (P17-EVD-001/EVD-002).
    assert_eq!(
        compaction.run, provider.run,
        "compaction shares the run identity"
    );
    assert!(
        provider.sequence < compaction.sequence,
        "compaction follows the provider record in sequence order"
    );

    // The Compaction payload is a structured value carrying the reason.
    let value = match &compaction.payload {
        EvidencePayload::Structured(rv) => rv.as_value(),
        _ => panic!("compaction payload must be structured"),
    };
    assert_eq!(
        value["reason"], "manual",
        "the compaction reason is recorded"
    );
}
