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

// opi-phase17-acceptance

mod common;

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::agent::{Agent, AgentRunLifecyclePhase};
use opi_agent::authority::{
    AuthorizationDecision, AuthorizationError, CapabilityIdentity, RegistrationId,
    ToolAuthorizationRequest, ToolAuthorizer,
};
use opi_agent::evidence::{
    ActualRoute, AssemblyIdentity, CallKind, CompactionOutcome, CompactionTrigger, ConfigIdentity,
    ContentDigest, EnvironmentFacts, EvidenceCompleteness, EvidenceError, EvidenceHealth,
    EvidencePayload, EvidenceRunObservation, EvidenceSink, ExecutionTrigger, FinalizedManifest,
    IdentityAllocator, InMemoryEvidenceSink, InputIdentity, ManifestCandidate, ManifestCorrelation,
    Measurement, PermissionReference, PermissionScope, PlatformIdentity, PolicyReference,
    ProvenanceFacts, ProviderEvidenceFacts, ProviderInvocationFacts, RequestedRoute, RouteFacts,
    RouteSelection, RuntimeInputBinding, SessionBinding, TerminalOutcome, UnknownReason,
    UsageFacts, UserPolicyFacts,
};
use opi_agent::hooks::{
    AgentHooks, BeforeToolCallContext, BeforeToolCallResult, ShouldStopAfterTurnContext,
};
use opi_agent::loop_types::{AgentError, AgentLoopConfig, InferenceConfig};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult};
use opi_ai::message::Message;
use opi_ai::message::{AssistantContent, AssistantMessage, OutputContent, ToolCall, ToolDef};
use opi_ai::model_info::WireApi;
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

struct FailOnAuthorizationEmission {
    authorizations: AtomicUsize,
    fail_on: usize,
}

#[derive(Clone, Copy)]
enum SecondAuthorizationFailure {
    Deny,
    Error,
    RegistrationMismatch,
    CapabilityMismatch,
    IdentityAndGenerationMismatch,
}

struct FailSecondAuthorizer {
    failure: SecondAuthorizationFailure,
}

struct GatedAllowAuthorizer {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

#[derive(Default)]
struct CountingCompleteEvidenceAuthorizer {
    calls: AtomicUsize,
}

impl CountingCompleteEvidenceAuthorizer {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

struct UncertainTool {
    outcome: UncertainToolOutcome,
}

#[derive(Clone, Copy)]
enum UncertainToolOutcome {
    PartialSideEffect,
    CleanupUnknown,
    Cancelled,
}

impl Tool for UncertainTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "uncertain".to_owned(),
            description: "returns an uncertain external-effect result".to_owned(),
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
        let outcome = self.outcome;
        Box::pin(async move {
            Err(match outcome {
                UncertainToolOutcome::PartialSideEffect => {
                    ToolError::PartialSideEffect("remote mutation may have completed".to_owned())
                }
                UncertainToolOutcome::CleanupUnknown => {
                    ToolError::CleanupUnknown("remote cleanup was not confirmed".to_owned())
                }
                UncertainToolOutcome::Cancelled => ToolError::Cancelled,
            })
        })
    }
}

struct SequentialUncertainTool {
    outcome: UncertainToolOutcome,
    executions: Arc<AtomicUsize>,
}

impl Tool for SequentialUncertainTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "first".to_owned(),
            description: "returns a terminal uncertain outcome".to_owned(),
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
        let outcome = self.outcome;
        let executions = self.executions.clone();
        Box::pin(async move {
            executions.fetch_add(1, Ordering::SeqCst);
            Err(match outcome {
                UncertainToolOutcome::PartialSideEffect => {
                    ToolError::PartialSideEffect("first tool may have mutated".to_owned())
                }
                UncertainToolOutcome::CleanupUnknown => {
                    ToolError::CleanupUnknown("first tool cleanup is unknown".to_owned())
                }
                UncertainToolOutcome::Cancelled => ToolError::Cancelled,
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

struct FinalizationFailureSink {
    abandon_fails: bool,
    abandon_calls: AtomicUsize,
}

struct PriorEmissionAndCleanupFailureSink {
    fail_next_emission: AtomicBool,
    finalization_calls: AtomicUsize,
    abandon_calls: AtomicUsize,
}

#[derive(Default)]
struct AbandonRecordingSink {
    records: Mutex<Vec<opi_agent::evidence::EvidenceRecord>>,
    finalized: AtomicUsize,
    abandoned: Mutex<Vec<TerminalOutcome>>,
}

impl EvidenceSink for AbandonRecordingSink {
    fn setup(&self, _binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn emit(&self, record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        self.records.lock().unwrap().push(record.clone());
        Ok(())
    }

    fn finalize_artifact(
        &self,
        _artifact: &opi_agent::evidence::ArtifactReference,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn finalize_run(&self, _manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        self.finalized.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn abandon_run(&self, outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        self.abandoned.lock().unwrap().push(outcome.clone());
        Ok(())
    }
}

impl EvidenceSink for PriorEmissionAndCleanupFailureSink {
    fn setup(&self, _binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn emit(&self, _record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        if self.fail_next_emission.swap(false, Ordering::SeqCst) {
            Err(EvidenceError::Emission {
                detail: "prior emission failed".to_owned(),
            })
        } else {
            Ok(())
        }
    }

    fn finalize_artifact(
        &self,
        _artifact: &opi_agent::evidence::ArtifactReference,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn finalize_run(&self, _manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        self.finalization_calls.fetch_add(1, Ordering::SeqCst);
        Err(EvidenceError::Finalization {
            detail: "unexpected sink finalization".to_owned(),
        })
    }

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        self.abandon_calls.fetch_add(1, Ordering::SeqCst);
        Err(EvidenceError::Finalization {
            detail: "cleanup confirmation failed".to_owned(),
        })
    }
}

impl EvidenceSink for FinalizationFailureSink {
    fn setup(
        &self,
        _binding: &opi_agent::evidence::RuntimeInputBinding,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn emit(&self, _record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn finalize_artifact(
        &self,
        _artifact: &opi_agent::evidence::ArtifactReference,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn finalize_run(&self, _manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        Err(EvidenceError::Finalization {
            detail: "owning finalization failure".to_owned(),
        })
    }

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        self.abandon_calls.fetch_add(1, Ordering::SeqCst);
        if self.abandon_fails {
            Err(EvidenceError::Finalization {
                detail: "cleanup confirmation failed".to_owned(),
            })
        } else {
            Ok(())
        }
    }
}

impl ToolAuthorizer for FailSecondAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<AuthorizationDecision, AuthorizationError>>
                + Send,
        >,
    > {
        let failure = self.failure;
        Box::pin(async move {
            if request.registration_id.as_str() == "test-second" {
                match failure {
                    SecondAuthorizationFailure::Deny => {
                        return Ok(AuthorizationDecision::Deny {
                            stable_code: "second_denied".to_owned(),
                            redacted_reason: "second authorization denied".to_owned(),
                        });
                    }
                    SecondAuthorizationFailure::Error => {
                        return Err(AuthorizationError::Unavailable(
                            "injected second authorization failure".to_owned(),
                        ));
                    }
                    SecondAuthorizationFailure::RegistrationMismatch => {
                        return Ok(AuthorizationDecision::Allow {
                            policy_ref: PolicyReference::new("test-policy").unwrap(),
                            permission_ref: PermissionReference::new("test-permission").unwrap(),
                            permission_scope: PermissionScope::new("test-scope").unwrap(),
                            scoped_grant_ref: None,
                            registration_id: RegistrationId::new("mismatched-registration"),
                            capability: request.capability,
                            evidence_health_generation: request.evidence_health.generation(),
                        });
                    }
                    SecondAuthorizationFailure::CapabilityMismatch => {
                        return Ok(AuthorizationDecision::Allow {
                            policy_ref: PolicyReference::new("test-policy").unwrap(),
                            permission_ref: PermissionReference::new("test-permission").unwrap(),
                            permission_scope: PermissionScope::new("test-scope").unwrap(),
                            scoped_grant_ref: None,
                            registration_id: request.registration_id,
                            capability: CapabilityIdentity::new("test.mismatched.capability")
                                .unwrap(),
                            evidence_health_generation: request.evidence_health.generation(),
                        });
                    }
                    SecondAuthorizationFailure::IdentityAndGenerationMismatch => {
                        return Ok(AuthorizationDecision::Allow {
                            policy_ref: PolicyReference::new("test-policy").unwrap(),
                            permission_ref: PermissionReference::new("test-permission").unwrap(),
                            permission_scope: PermissionScope::new("test-scope").unwrap(),
                            scoped_grant_ref: None,
                            registration_id: RegistrationId::new("mismatched-registration"),
                            capability: request.capability,
                            evidence_health_generation: request.evidence_health.generation().next(),
                        });
                    }
                }
            }
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id,
                capability: request.capability,
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
}

impl ToolAuthorizer for GatedAllowAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<AuthorizationDecision, AuthorizationError>>
                + Send,
        >,
    > {
        let started = self.started.clone();
        let release = self.release.clone();
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id,
                capability: request.capability,
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
}

impl ToolAuthorizer for CountingCompleteEvidenceAuthorizer {
    fn authorize(
        &self,
        request: ToolAuthorizationRequest,
        _cancel: CancellationToken,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<AuthorizationDecision, AuthorizationError>>
                + Send,
        >,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if !request.evidence_health.is_healthy() {
                return Ok(AuthorizationDecision::Deny {
                    stable_code: "evidence_incomplete".to_owned(),
                    redacted_reason: "complete evidence is required".to_owned(),
                });
            }
            Ok(AuthorizationDecision::Allow {
                policy_ref: PolicyReference::new("test-policy").unwrap(),
                permission_ref: PermissionReference::new("test-permission").unwrap(),
                permission_scope: PermissionScope::new("test-scope").unwrap(),
                scoped_grant_ref: None,
                registration_id: request.registration_id,
                capability: request.capability,
                evidence_health_generation: request.evidence_health.generation(),
            })
        })
    }
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

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        Ok(())
    }
}

impl EvidenceSink for FailOnAuthorizationEmission {
    fn setup(
        &self,
        _binding: &opi_agent::evidence::RuntimeInputBinding,
    ) -> Result<(), EvidenceError> {
        Ok(())
    }

    fn emit(&self, record: &opi_agent::evidence::EvidenceRecord) -> Result<(), EvidenceError> {
        if matches!(
            &record.payload,
            EvidencePayload::Tool(facts)
                if facts.phase() == opi_agent::evidence::ToolEvidencePhase::Authorization
        ) {
            let authorization = self.authorizations.fetch_add(1, Ordering::SeqCst) + 1;
            if authorization == self.fail_on {
                return Err(EvidenceError::Emission {
                    detail: "injected parallel authorization emission failure".to_owned(),
                });
            }
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

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
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

fn tool_call_response_with_model(model: &str) -> Vec<AssistantStreamEvent> {
    let mut events = tool_call_response("c1", "mytool", "{}");
    for event in &mut events {
        match event {
            AssistantStreamEvent::Start { partial } => partial.model = model.to_owned(),
            AssistantStreamEvent::Done { message, .. } => message.model = model.to_owned(),
            _ => {}
        }
    }
    events
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
    make_agent_with_sink(responses, registrations, authorizer, retry, sink)
}

fn make_agent_with_sink(
    responses: Vec<MockResponse>,
    registrations: Vec<opi_agent::authority::RegisteredTool>,
    authorizer: Option<Arc<dyn opi_agent::authority::ToolAuthorizer>>,
    retry: Option<RetryConfig>,
    sink: Arc<dyn EvidenceSink>,
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
    agent.set_evidence_sink(Some(sink.clone()));
    // The runtime-input binding and capture setup are assembly-owned facts:
    // trusted wiring sets the sink up before the first provider call, so the
    // fixture mirrors that contract instead of emitting before setup.
    let digest_byte = "agent-runtime"
        .as_bytes()
        .iter()
        .fold(0_u8, |acc, b| acc ^ b);
    let binding = RuntimeInputBinding::direct(
        ContentDigest::from_hex(format!("{digest_byte:02x}").repeat(32)).expect("valid digest"),
        AssemblyIdentity::new("opi.test.fixture").expect("valid assembly identity"),
    );
    EvidenceSink::setup(&*sink, &binding).expect("the in-memory sink accepts a direct-run binding");
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

fn digest(byte: char) -> ContentDigest {
    ContentDigest::from_hex(byte.to_string().repeat(64)).unwrap()
}

fn standalone_finalized_manifest(
    run: opi_agent::evidence::RunId,
    outcome: TerminalOutcome,
) -> FinalizedManifest {
    let binding =
        RuntimeInputBinding::direct(digest('1'), AssemblyIdentity::new("test.runtime").unwrap());
    let mut identities = IdentityAllocator::new();
    let turn = identities.next_turn();
    let call = identities.next_call();
    let sequence = identities.next_sequence();
    let route = RouteFacts::new(
        RequestedRoute::new("mock", "mock-model").unwrap(),
        RouteSelection::new("mock", "mock-model", WireApi::OpenAiResponses).unwrap(),
        ActualRoute::wire_unknown("mock", "mock-model", UnknownReason::NotReported).unwrap(),
    );
    let provenance = ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance::default()).unwrap();
    let record = opi_agent::evidence::EvidenceRecord {
        run,
        turn: Some(turn),
        call,
        parent: None,
        sequence,
        kind: CallKind::Provider,
        payload: EvidencePayload::Provider(ProviderEvidenceFacts {
            route: route.clone(),
            provenance: provenance.clone(),
        }),
    };
    let candidate = ManifestCandidate {
        correlation: ManifestCorrelation {
            run,
            turn: Some(turn),
            call: Some(call),
            parent: None,
            sequence,
        },
        outcome,
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
            system_digest: None,
            tool_schema_digests: Vec::new(),
        },
        environment: EnvironmentFacts {
            budget: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            trigger: ExecutionTrigger::Invocation,
            time: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            platform: PlatformIdentity::new("test"),
        },
        usage: UsageFacts {
            input_tokens: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            output_tokens: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
        },
        artifacts: Vec::new(),
        completeness: EvidenceCompleteness::Complete,
    };
    candidate
        .validate(EvidenceRunObservation::new(&binding, &[record], &[]))
        .unwrap()
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
        .rev()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a terminal Provider record is emitted for the provider dispatch");
    assert!(
        provider.turn.is_some(),
        "the provider record carries its turn identity"
    );

    // The provider record attaches the requested and resolved route facts
    // (P17-EVD-002: requested/resolved/actual route correlation).
    let facts = match &provider.payload {
        EvidencePayload::Provider(facts) => facts,
        _ => panic!("provider payload must carry typed provider facts"),
    };
    assert_eq!(facts.route.requested().provider_id(), "mock");
    assert_eq!(facts.route.requested().model_id(), "mock-model");
    assert_eq!(facts.route.resolved().provider_id(), "mock");
    assert_eq!(facts.route.resolved().model_id(), "mock-model");
    // Provider/model are reported by the terminal response. The response does
    // not report the exact wire, so the typed actual keeps that unknown reason
    // rather than copying the configured wire (P17-PRV-005 / P17-EVD-004).
    assert!(
        matches!(
            facts.route.actual(),
            opi_agent::evidence::ActualRoute::WireUnknown {
                route,
                reason: opi_agent::evidence::UnknownReason::NotReported
            } if route.provider_id() == "mock" && route.model_id() == "mock-model"
        ),
        "reported actual provider/model retain a typed unknown-wire reason"
    );

    // Sequence is strictly monotonic across emission order (P17-EVD-001).
    assert!(
        records.windows(2).all(|w| w[0].sequence < w[1].sequence),
        "sequence is strictly monotonic"
    );
}

#[tokio::test]
async fn absent_terminal_model_keeps_evidence_healthy_for_tool_authorization() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let count = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![Box::new(common::RecordingTool::new(
        "mytool",
        count.clone(),
    ))]);
    let mut agent = make_agent(
        vec![
            MockResponse::Events(tool_call_response_with_model("")),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(Arc::new(common::StaleGenerationAuthorizer::default())),
        None,
        sink.clone(),
    );

    let run = agent.prompt("go").await;
    assert!(run.error().is_none());

    assert_eq!(
        common::RecordingTool::count_of(&count),
        1,
        "an absent provider-reported model is unknown evidence, not evidence corruption"
    );
    assert!(sink.records().iter().any(|record| {
        matches!(
            &record.payload,
            EvidencePayload::Provider(facts)
                if matches!(
                    facts.route.actual(),
                    opi_agent::evidence::ActualRoute::Unknown {
                        reason: opi_agent::evidence::UnknownReason::NotReported
                    }
                )
        )
    }));
    drop(run);
}

#[tokio::test]
async fn malformed_nonempty_terminal_model_marks_evidence_incomplete() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let count = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![Box::new(common::RecordingTool::new(
        "mytool",
        count.clone(),
    ))]);
    let mut agent = make_agent(
        vec![
            MockResponse::Events(tool_call_response_with_model(" malformed")),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(Arc::new(common::StaleGenerationAuthorizer::default())),
        None,
        sink,
    );

    let _ = agent.prompt("go").await;

    assert_eq!(
        common::RecordingTool::count_of(&count),
        0,
        "non-empty malformed provider facts remain evidence corruption"
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
    let facts = match &tool.payload {
        EvidencePayload::Tool(facts) => facts,
        _ => panic!("tool payload must carry typed tool facts"),
    };
    assert_eq!(
        facts.registration().unwrap().as_str(),
        "test-mytool",
        "registration id attached"
    );
    assert_eq!(
        facts.capability().unwrap().as_str(),
        "opi.workspace.read",
        "parallel evidence retains the exact opaque capability identity"
    );
    assert!(matches!(
        facts.authorization_facts(),
        opi_agent::evidence::ToolAuthorizationFacts::Allowed { .. }
    ));
}

#[tokio::test]
async fn sequential_authorization_evidence_retains_exact_capability_identity() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let executions = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![Box::new(SequentialCountingTool {
        name: "mytool".into(),
        executions,
    })]);
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

    let run = agent.prompt("go").await;
    assert!(run.error().is_none());

    let authorization = sink
        .records()
        .into_iter()
        .find_map(|record| match record.payload {
            EvidencePayload::Tool(facts)
                if facts.phase() == opi_agent::evidence::ToolEvidencePhase::Authorization =>
            {
                Some(facts)
            }
            _ => None,
        })
        .expect("sequential authorization evidence is emitted");
    assert_eq!(
        authorization.capability().unwrap().as_str(),
        "opi.workspace.read",
        "sequential evidence retains the exact opaque capability identity"
    );
    assert!(matches!(
        authorization.invocation(),
        opi_agent::evidence::InvocationBinding::NoSession
    ));
    match authorization.authorization_facts() {
        opi_agent::evidence::ToolAuthorizationFacts::Allowed {
            policy_ref,
            permission_ref,
            permission_scope,
            scoped_grant_ref,
        } => {
            assert_eq!(policy_ref.as_str(), "test-policy");
            assert_eq!(permission_ref.as_str(), "test-permission");
            assert_eq!(permission_scope.as_str(), "test-scope");
            assert!(scoped_grant_ref.is_none());
        }
        other => panic!("expected exact typed allow facts, got {other:?}"),
    }
    drop(run);
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
    // The pre-dispatch and terminal Provider lifecycle records reuse one call
    // identity: the route was resolved once per turn and reused across the
    // retry (prepare_call is not re-invoked).
    assert_eq!(
        records
            .iter()
            .filter(|r| r.kind == CallKind::Provider)
            .map(|r| r.call)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1,
        "one Provider call identity — one immutable route across retries"
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
    let mut run = agent.prompt("go").await;

    assert!(
        sink.has_failure(),
        "the injected emission failure was observed by the sink"
    );
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete {
            first_failure_code: opi_agent::evidence::EvidenceFailureCode::Emission,
            ..
        }
    ));
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
    // The run is deliberately fail-open for provider dispatch: the trusted
    // authorizer at tool launch, not the provider request, is the fail-closed
    // boundary after evidence health becomes incomplete. Both scripted
    // provider attempts therefore proceeded and completed — the tool-call turn
    // and the recovered text turn are both in the run's visible conversation —
    // even though health advanced to incomplete before the first dispatch.
    let messages = run.messages();
    assert!(
        messages.iter().any(|m| matches!(m,
            AgentMessage::Llm(Message::Assistant(a))
                if a.content.iter().any(|c| matches!(c, AssistantContent::ToolCall { .. })))),
        "the first provider attempt proceeded after the failed pre-dispatch emission"
    );
    assert!(
        messages.iter().any(|m| matches!(m,
            AgentMessage::Llm(Message::Assistant(a))
                if a.content.iter().any(|c| matches!(c, AssistantContent::Text { text } if text == "done")))),
        "the second provider attempt also proceeded with health already incomplete"
    );
    let generation = run.evidence_health().generation();
    assert!(run.begin_compaction(CompactionTrigger::Threshold).is_err());
    assert_eq!(
        run.evidence_health().generation(),
        generation,
        "an already-incomplete run rejects compaction before another emission attempt"
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
        // Provider preparation + terminal actual-route records precede the
        // first tool authorization. Fail the first tool outcome record.
        fail_on: 4,
    })));

    let _ = agent.prompt("go").await;

    assert_eq!(
        executions.load(Ordering::SeqCst),
        1,
        "the second sequential tool must not launch after the first outcome's evidence failure"
    );
}

#[tokio::test]
async fn parallel_authorization_record_failure_on_first_or_second_launches_zero_tools() {
    for fail_on in [1, 2] {
        let executions = Arc::new(AtomicUsize::new(0));
        let registrations = common::registrations_from(vec![
            Box::new(common::RecordingTool::new("first", executions.clone())),
            Box::new(common::RecordingTool::new("second", executions.clone())),
        ]);
        let sink = Arc::new(FailOnAuthorizationEmission {
            authorizations: AtomicUsize::new(0),
            fail_on,
        });
        let authorizer = Arc::new(CountingCompleteEvidenceAuthorizer::default());
        let mut agent = make_agent_with_sink(
            vec![
                MockResponse::Events(two_tool_response()),
                MockResponse::Events(text_response("done")),
            ],
            registrations,
            Some(authorizer.clone()),
            None,
            sink.clone(),
        );

        let _ = agent.prompt("go").await;

        assert_eq!(
            executions.load(Ordering::SeqCst),
            0,
            "failure on parallel authorization record {fail_on} must occur before every launch"
        );
        assert_eq!(
            sink.authorizations.load(Ordering::SeqCst),
            3,
            "the changed evidence generation is reauthorized once before later preflight"
        );
        assert_eq!(
            authorizer.calls(),
            3,
            "the first failed authorization record triggers one complete-evidence reauthorization"
        );
    }
}

struct ParallelBarrierTool {
    name: String,
    barrier: Arc<tokio::sync::Barrier>,
    executions: Arc<AtomicUsize>,
}

impl Tool for ParallelBarrierTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: self.name.clone(),
            description: "parallel preflight barrier".to_owned(),
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
        let barrier = self.barrier.clone();
        let executions = self.executions.clone();
        Box::pin(async move {
            executions.fetch_add(1, Ordering::SeqCst);
            barrier.wait().await;
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
}

#[tokio::test]
async fn parallel_allows_preflight_in_source_order_then_launch_concurrently() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let executions = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![
        Box::new(ParallelBarrierTool {
            name: "first".into(),
            barrier: barrier.clone(),
            executions: executions.clone(),
        }),
        Box::new(ParallelBarrierTool {
            name: "second".into(),
            barrier,
            executions: executions.clone(),
        }),
    ]);
    let mut agent = make_agent(
        vec![
            MockResponse::Events(two_tool_response()),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(common::permissive_authorizer()),
        None,
        sink.clone(),
    );

    let run = tokio::time::timeout(std::time::Duration::from_secs(2), agent.prompt("go"))
        .await
        .expect("both prepared parallel calls must cross the launch barrier concurrently");
    assert!(run.error().is_none());

    assert_eq!(executions.load(Ordering::SeqCst), 2);
    let tool_phases: Vec<_> = sink
        .records()
        .into_iter()
        .filter_map(|record| match record.payload {
            EvidencePayload::Tool(facts) => Some((facts.phase(), facts.tool().as_str().to_owned())),
            _ => None,
        })
        .collect();
    assert_eq!(
        tool_phases,
        vec![
            (
                opi_agent::evidence::ToolEvidencePhase::Authorization,
                "first".to_owned(),
            ),
            (
                opi_agent::evidence::ToolEvidencePhase::Authorization,
                "second".to_owned(),
            ),
            (
                opi_agent::evidence::ToolEvidencePhase::Outcome,
                "first".to_owned(),
            ),
            (
                opi_agent::evidence::ToolEvidencePhase::Outcome,
                "second".to_owned(),
            ),
        ],
        "parallel calls use the same typed authorization/outcome phases as sequential calls"
    );
    drop(run);
}

#[tokio::test]
async fn parallel_call_local_authorization_rejection_still_launches_fresh_allowed_sibling() {
    for failure in [
        SecondAuthorizationFailure::Deny,
        SecondAuthorizationFailure::Error,
        SecondAuthorizationFailure::RegistrationMismatch,
        SecondAuthorizationFailure::CapabilityMismatch,
    ] {
        let sink = Arc::new(InMemoryEvidenceSink::new());
        let first_executions = Arc::new(AtomicUsize::new(0));
        let second_executions = Arc::new(AtomicUsize::new(0));
        let registrations = common::registrations_from(vec![
            Box::new(common::RecordingTool::new(
                "first",
                first_executions.clone(),
            )),
            Box::new(common::RecordingTool::new(
                "second",
                second_executions.clone(),
            )),
        ]);
        let mut agent = make_agent(
            vec![
                MockResponse::Events(two_tool_response()),
                MockResponse::Events(text_response("done")),
            ],
            registrations,
            Some(Arc::new(FailSecondAuthorizer { failure })),
            None,
            sink.clone(),
        );

        let _ = agent.prompt("go").await;

        assert_eq!(
            first_executions.load(Ordering::SeqCst),
            1,
            "a call-local denial, authorizer error, or identity mismatch must not invalidate a fresh allowed sibling"
        );
        assert_eq!(
            second_executions.load(Ordering::SeqCst),
            0,
            "the call whose authorization was rejected must retain zero executions"
        );
        let tool_phases: Vec<_> = sink
            .records()
            .into_iter()
            .filter_map(|record| match record.payload {
                EvidencePayload::Tool(facts) => {
                    Some((facts.phase(), facts.tool().as_str().to_owned()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            tool_phases,
            vec![
                (
                    opi_agent::evidence::ToolEvidencePhase::Authorization,
                    "first".to_owned(),
                ),
                (
                    opi_agent::evidence::ToolEvidencePhase::Authorization,
                    "second".to_owned(),
                ),
                (
                    opi_agent::evidence::ToolEvidencePhase::Outcome,
                    "first".to_owned(),
                ),
                (
                    opi_agent::evidence::ToolEvidencePhase::Outcome,
                    "second".to_owned(),
                ),
            ],
            "call-local rejection retains stable source-ordered authorization/outcome records"
        );
    }
}

#[tokio::test]
async fn parallel_generation_mismatch_invalidates_batch_even_with_identity_mismatch() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let first_executions = Arc::new(AtomicUsize::new(0));
    let second_executions = Arc::new(AtomicUsize::new(0));
    let registrations = common::registrations_from(vec![
        Box::new(common::RecordingTool::new(
            "first",
            first_executions.clone(),
        )),
        Box::new(common::RecordingTool::new(
            "second",
            second_executions.clone(),
        )),
    ]);
    let mut agent = make_agent(
        vec![
            MockResponse::Events(two_tool_response()),
            MockResponse::Events(text_response("done")),
        ],
        registrations,
        Some(Arc::new(FailSecondAuthorizer {
            failure: SecondAuthorizationFailure::IdentityAndGenerationMismatch,
        })),
        None,
        sink,
    );

    let _ = agent.prompt("go").await;

    assert_eq!(
        first_executions.load(Ordering::SeqCst),
        0,
        "a persistent evidence-generation mismatch must invalidate an earlier fresh sibling"
    );
    assert_eq!(
        second_executions.load(Ordering::SeqCst),
        0,
        "the mixed identity/generation mismatch call must not execute"
    );
}

async fn join_task_with_timeout<T: Send + 'static>(
    mut task: tokio::task::JoinHandle<T>,
    timeout: std::time::Duration,
    label: &str,
) -> Result<T, String> {
    match tokio::time::timeout(timeout, &mut task).await {
        Ok(joined) => joined.map_err(|error| format!("{label} task failed: {error}")),
        Err(_) => {
            task.abort();
            let cleanup = match tokio::time::timeout(timeout, &mut task).await {
                Ok(Ok(_)) => "task completed while abort was being delivered".to_owned(),
                Ok(Err(error)) => format!("task aborted: {error}"),
                Err(_) => format!("abort cleanup did not finish within {timeout:?}"),
            };
            Err(format!(
                "{label} did not terminate within {timeout:?}; {cleanup}"
            ))
        }
    }
}

#[tokio::test]
async fn bounded_task_join_aborts_pending_future() {
    let handle = tokio::spawn(std::future::pending::<()>());
    let error = join_task_with_timeout(
        handle,
        std::time::Duration::from_millis(10),
        "pending join canary",
    )
    .await
    .expect_err("a pending task must hit the join bound");
    assert!(
        error.contains("pending join canary") && error.contains("task aborted"),
        "timeout reports the task label and bounded abort cleanup: {error}"
    );
}

#[tokio::test]
async fn cancellation_while_authorization_is_pending_prevents_sequential_and_parallel_launch() {
    for execution_mode in [ExecutionMode::Parallel, ExecutionMode::Sequential] {
        let sink = Arc::new(InMemoryEvidenceSink::new());
        let executions = Arc::new(AtomicUsize::new(0));
        let tool: Box<dyn Tool> = match execution_mode {
            ExecutionMode::Parallel => {
                Box::new(common::RecordingTool::new("mytool", executions.clone()))
            }
            ExecutionMode::Sequential => Box::new(SequentialCountingTool {
                name: "mytool".into(),
                executions: executions.clone(),
            }),
        };
        let started = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let mut agent = make_agent(
            vec![MockResponse::Events(tool_call_response(
                "c1", "mytool", "{}",
            ))],
            common::registrations_from(vec![tool]),
            Some(Arc::new(GatedAllowAuthorizer {
                started: started.clone(),
                release: release.clone(),
            })),
            None,
            sink.clone(),
        );
        let cancel = agent.cancel_token();
        let handle = tokio::spawn(async move { agent.prompt("go").await });

        tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
            .await
            .unwrap_or_else(|_| {
                panic!("{execution_mode:?} authorization readiness must be bounded")
            });
        cancel.cancel();
        release.notify_one();

        let result = join_task_with_timeout(
            handle,
            std::time::Duration::from_secs(2),
            &format!("{execution_mode:?} authorization cancellation"),
        )
        .await
        .unwrap_or_else(|error| panic!("{error}"));
        assert!(
            matches!(result.error(), Some(AgentError::Cancelled)),
            "cancellation must win over an Allow released at the same boundary"
        );
        assert_eq!(
            executions.load(Ordering::SeqCst),
            0,
            "a tool that ignores its token must never be called after preflight cancellation"
        );
        assert!(
            sink.records().iter().all(|record| !matches!(
                &record.payload,
                EvidencePayload::Tool(facts)
                    if facts.phase() == opi_agent::evidence::ToolEvidencePhase::Outcome
            )),
            "a never-launched cancelled call must not emit a terminal tool outcome"
        );
    }
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
    let mut run = agent.prompt("go").await;

    // Post-run compaction emits a start before mutation is authorized and a
    // terminal outcome afterward, both correlated to the same call.
    let compaction = run
        .begin_compaction(CompactionTrigger::Manual)
        .expect("compaction start evidence emits");
    run.finish_compaction(&compaction, CompactionOutcome::Succeeded)
        .expect("compaction terminal evidence emits");

    let records = sink.records();
    let provider = records
        .iter()
        .find(|r| r.kind == CallKind::Provider)
        .expect("a Provider record is emitted");
    let compaction: Vec<_> = records
        .iter()
        .filter(|r| r.kind == CallKind::Compaction)
        .collect();
    assert_eq!(
        compaction.len(),
        2,
        "start and terminal records are emitted"
    );

    // The Compaction record shares the run identity and follows the provider
    // record in sequence order (P17-EVD-001/EVD-002).
    assert_eq!(
        compaction[0].run, provider.run,
        "compaction shares the run identity"
    );
    assert!(
        provider.sequence < compaction[0].sequence
            && compaction[0].sequence < compaction[1].sequence,
        "compaction follows the provider record in sequence order"
    );
    assert_eq!(compaction[0].call, compaction[1].call);

    assert!(matches!(
        &compaction[0].payload,
        EvidencePayload::Compaction(facts)
            if facts.trigger() == CompactionTrigger::Manual && facts.outcome().is_none()
    ));
    assert!(matches!(
        &compaction[1].payload,
        EvidencePayload::Compaction(facts)
            if facts.trigger() == CompactionTrigger::Manual
                && facts.outcome() == Some(CompactionOutcome::Succeeded)
    ));
}

#[tokio::test]
async fn compaction_abort_failure_and_cleanup_unknown_remain_typed() {
    for (compaction_outcome, terminal_outcome) in [
        (CompactionOutcome::Aborted, TerminalOutcome::Cancelled),
        (CompactionOutcome::Failed, TerminalOutcome::Failed),
        (
            CompactionOutcome::CleanupUnknown,
            TerminalOutcome::CleanupUnknown,
        ),
    ] {
        let sink = Arc::new(InMemoryEvidenceSink::new());
        let mut agent = make_agent(
            vec![MockResponse::Events(text_response("done"))],
            Vec::new(),
            None,
            None,
            sink.clone(),
        );
        let mut run = agent.prompt("go").await;
        let pending = run.begin_compaction(CompactionTrigger::Manual).unwrap();

        run.finish_compaction(&pending, compaction_outcome).unwrap();

        assert_eq!(run.terminal_outcome(), &terminal_outcome);
        assert!(sink.records().iter().any(|record| {
            matches!(
                &record.payload,
                EvidencePayload::Compaction(facts)
                    if facts.outcome() == Some(compaction_outcome)
            )
        }));
    }
}

#[tokio::test]
async fn run_result_carries_success_and_final_evidence_health() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink,
    );

    let run = agent.prompt("go").await;

    assert!(run.error().is_none());
    assert!(!run.messages().is_empty());
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::Success);
    assert!(run.evidence_health().is_healthy());
}

#[tokio::test]
async fn partial_side_effect_and_cleanup_unknown_survive_tool_and_run_boundaries() {
    for (tool_outcome, expected_terminal, expected_tool) in [
        (
            UncertainToolOutcome::PartialSideEffect,
            TerminalOutcome::PartialSideEffect,
            opi_agent::evidence::ToolExecutionOutcome::PartialSideEffect,
        ),
        (
            UncertainToolOutcome::CleanupUnknown,
            TerminalOutcome::CleanupUnknown,
            opi_agent::evidence::ToolExecutionOutcome::CleanupUnknown,
        ),
    ] {
        let sink = Arc::new(InMemoryEvidenceSink::new());
        let mut agent = make_agent(
            vec![MockResponse::Events(tool_call_response(
                "c1",
                "uncertain",
                "{}",
            ))],
            common::registrations_from(vec![Box::new(UncertainTool {
                outcome: tool_outcome,
            })]),
            Some(Arc::new(common::StaleGenerationAuthorizer::default())),
            None,
            sink.clone(),
        );

        let run = agent.prompt("go").await;

        assert_eq!(run.terminal_outcome(), &expected_terminal);
        assert!(run.error().is_some(), "the owning tool failure is retained");
        assert!(
            agent
                .messages_snapshot()
                .iter()
                .any(|message| { matches!(message, AgentMessage::Llm(Message::ToolResult(_))) })
        );
        assert!(sink.records().iter().any(|record| {
            matches!(
                &record.payload,
                EvidencePayload::Tool(facts)
                    if facts
                        .outcome_facts()
                        .is_some_and(|outcome| outcome.execution == expected_tool)
            )
        }));
    }
}

#[tokio::test]
async fn failed_compaction_start_invalidates_health_without_authorizing_mutation() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    sink.inject_failure(EvidenceError::Emission {
        detail: "compaction start failed".to_owned(),
    });

    let result = run.begin_compaction(CompactionTrigger::Threshold);

    assert!(result.is_err(), "no pending compaction is returned");
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete {
            first_failure_code: opi_agent::evidence::EvidenceFailureCode::Emission,
            ..
        }
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::Success);
}

#[tokio::test]
async fn failed_compaction_terminal_retains_actual_outcome_and_incomplete_health() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let pending = run
        .begin_compaction(CompactionTrigger::Overflow)
        .expect("start succeeds before mutation");
    sink.inject_failure(EvidenceError::Emission {
        detail: "compaction terminal failed".to_owned(),
    });

    let result = run.finish_compaction(&pending, CompactionOutcome::PartialSideEffect);

    assert!(result.is_err());
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::PartialSideEffect);
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete { .. }
    ));
    let generation = run.evidence_health().generation();
    assert!(run.begin_compaction(CompactionTrigger::Manual).is_err());
    assert_eq!(
        run.evidence_health().generation(),
        generation,
        "a failed terminal emission prohibits every later compaction start"
    );
}

#[tokio::test]
async fn finalization_failure_advances_health_and_preserves_execution_outcome() {
    let sink = Arc::new(FinalizationFailureSink {
        abandon_fails: false,
        abandon_calls: AtomicUsize::new(0),
    });
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let manifest = standalone_finalized_manifest(run.run_id(), TerminalOutcome::Success);

    let error = run.finalize_evidence(&manifest).unwrap_err();

    assert!(matches!(
        error,
        EvidenceError::Finalization { ref detail } if detail == "owning finalization failure"
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::Success);
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete {
            first_failure_code: opi_agent::evidence::EvidenceFailureCode::Finalization,
            ..
        }
    ));
    assert_eq!(sink.abandon_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn manifest_terminal_outcome_must_match_the_core_lifecycle() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink,
    );
    let mut run = agent.prompt("go").await;
    let mismatched = standalone_finalized_manifest(run.run_id(), TerminalOutcome::Failed);

    let error = run.finalize_evidence(&mismatched).unwrap_err();

    assert!(matches!(
        error,
        EvidenceError::Finalization { ref detail }
            if detail == "manifest terminal outcome does not match the run lifecycle"
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::Success);
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete {
            first_failure_code: opi_agent::evidence::EvidenceFailureCode::Finalization,
            ..
        }
    ));
}

#[tokio::test]
async fn unconfirmed_finalization_cleanup_is_retained_without_replacing_owning_error() {
    let sink = Arc::new(FinalizationFailureSink {
        abandon_fails: true,
        abandon_calls: AtomicUsize::new(0),
    });
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let manifest = standalone_finalized_manifest(run.run_id(), TerminalOutcome::Success);

    let error = run.finalize_evidence(&manifest).unwrap_err();

    assert!(matches!(
        error,
        EvidenceError::Finalization { ref detail } if detail == "owning finalization failure"
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::CleanupUnknown);
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete { .. }
    ));
    assert!(matches!(
        run.evidence_cleanup_error(),
        Some(EvidenceError::Finalization { detail }) if detail == "cleanup confirmation failed"
    ));
    assert_eq!(sink.abandon_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn dropped_compaction_token_blocks_manifest_and_abandons_the_lifecycle() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    let mut agent = make_agent(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink,
    );
    let mut run = agent.prompt("go").await;
    let token = run.begin_compaction(CompactionTrigger::Manual).unwrap();
    assert_eq!(
        run.lifecycle_phase(),
        AgentRunLifecyclePhase::CompactionPending
    );
    drop(token);

    let error = run
        .finalize_evidence(&standalone_finalized_manifest(
            run.run_id(),
            TerminalOutcome::Success,
        ))
        .unwrap_err();

    assert!(matches!(error, EvidenceError::Finalization { .. }));
    assert_eq!(
        run.lifecycle_phase(),
        AgentRunLifecyclePhase::FinalizedOrAbandoned
    );
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete { .. }
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::CleanupUnknown);
}

#[tokio::test]
async fn compaction_token_must_match_once_and_cannot_cross_run_boundaries() {
    let mut first_agent = make_agent(
        vec![MockResponse::Events(text_response("first"))],
        Vec::new(),
        None,
        None,
        Arc::new(InMemoryEvidenceSink::new()),
    );
    let mut second_agent = make_agent(
        vec![MockResponse::Events(text_response("second"))],
        Vec::new(),
        None,
        None,
        Arc::new(InMemoryEvidenceSink::new()),
    );
    let mut first = first_agent.prompt("go").await;
    let mut second = second_agent.prompt("go").await;
    let first_token = first
        .begin_compaction(CompactionTrigger::Threshold)
        .unwrap();
    let second_token = second
        .begin_compaction(CompactionTrigger::Overflow)
        .unwrap();

    assert!(
        first
            .finish_compaction(&second_token, CompactionOutcome::Succeeded)
            .is_err()
    );
    assert_eq!(
        first.lifecycle_phase(),
        AgentRunLifecyclePhase::CompactionPending
    );
    first
        .finish_compaction(&first_token, CompactionOutcome::Succeeded)
        .unwrap();
    assert_eq!(first.lifecycle_phase(), AgentRunLifecyclePhase::Active);
    assert!(
        first
            .finish_compaction(&first_token, CompactionOutcome::Succeeded)
            .is_err(),
        "a terminal compaction cannot be emitted twice"
    );

    second
        .finish_compaction(&second_token, CompactionOutcome::Aborted)
        .unwrap();
}

#[tokio::test]
async fn finalized_or_abandoned_run_rejects_repeated_finalization_and_compaction() {
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        Arc::new(opi_agent::evidence::NoopEvidenceSink::new()),
    );
    let mut run = agent.prompt("go").await;
    let manifest = standalone_finalized_manifest(run.run_id(), TerminalOutcome::Success);

    run.finalize_evidence(&manifest).unwrap();
    assert_eq!(
        run.lifecycle_phase(),
        AgentRunLifecyclePhase::FinalizedOrAbandoned
    );
    assert!(run.finalize_evidence(&manifest).is_err());
    assert!(run.begin_compaction(CompactionTrigger::Manual).is_err());

    let mut other_agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("other"))],
        Vec::new(),
        None,
        None,
        Arc::new(opi_agent::evidence::NoopEvidenceSink::new()),
    );
    let mut other = other_agent.prompt("go").await;
    let other_token = other.begin_compaction(CompactionTrigger::Manual).unwrap();
    assert!(
        run.finish_compaction(&other_token, CompactionOutcome::Succeeded)
            .is_err()
    );
    other
        .finish_compaction(&other_token, CompactionOutcome::Aborted)
        .unwrap();
}

#[tokio::test]
async fn prior_emission_finalization_and_abandonment_each_advance_health_generation() {
    let sink = Arc::new(PriorEmissionAndCleanupFailureSink {
        fail_next_emission: AtomicBool::new(false),
        finalization_calls: AtomicUsize::new(0),
        abandon_calls: AtomicUsize::new(0),
    });
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    sink.fail_next_emission.store(true, Ordering::SeqCst);
    run.begin_compaction(CompactionTrigger::Threshold)
        .unwrap_err();
    let prior_generation = run.evidence_health().generation();

    let owning_error = run
        .finalize_evidence(&standalone_finalized_manifest(
            run.run_id(),
            TerminalOutcome::Success,
        ))
        .unwrap_err();

    assert!(matches!(
        owning_error,
        EvidenceError::Finalization { ref detail }
            if detail == "evidence is incomplete; manifest finalization withheld"
    ));
    assert_eq!(
        run.evidence_health().generation(),
        prior_generation.next().next(),
        "blocked finalization and failed abandonment advance separately"
    );
    assert_eq!(sink.finalization_calls.load(Ordering::SeqCst), 0);
    assert_eq!(sink.abandon_calls.load(Ordering::SeqCst), 1);
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::CleanupUnknown);
}

#[tokio::test]
async fn pending_compaction_is_abandoned_on_execution_consumption_or_result_drop() {
    for consume in [true, false] {
        let sink = Arc::new(AbandonRecordingSink::default());
        let mut agent = make_agent_with_sink(
            vec![MockResponse::Events(text_response("done"))],
            Vec::new(),
            None,
            None,
            sink.clone(),
        );
        let mut run = agent.prompt("go").await;
        let token = run.begin_compaction(CompactionTrigger::Manual).unwrap();
        drop(token);

        if consume {
            assert!(matches!(
                run.into_execution_result(),
                Err(AgentError::EvidenceFinalization(_))
            ));
        } else {
            drop(run);
        }

        assert_eq!(
            sink.abandoned.lock().unwrap().as_slice(),
            &[TerminalOutcome::CleanupUnknown]
        );
        let compaction: Vec<_> = sink
            .records
            .lock()
            .unwrap()
            .iter()
            .filter(|record| record.kind == CallKind::Compaction)
            .cloned()
            .collect();
        assert_eq!(compaction.len(), 1, "only the start was emitted");
        assert!(matches!(
            &compaction[0].payload,
            EvidencePayload::Compaction(facts) if facts.outcome().is_none()
        ));
        assert_eq!(sink.finalized.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn explicit_pending_compaction_abandon_marks_cleanup_unknown_once() {
    let sink = Arc::new(AbandonRecordingSink::default());
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let pending = run.begin_compaction(CompactionTrigger::Manual).unwrap();
    drop(pending);
    let prior_generation = run.evidence_health().generation();
    let owning = EvidenceError::Emission {
        detail: "owning evidence failure".to_owned(),
    };

    run.abandon_evidence(&owning)
        .expect("cleanup confirmation succeeds");

    assert!(matches!(
        owning,
        EvidenceError::Emission { ref detail } if detail == "owning evidence failure"
    ));
    assert!(matches!(
        run.evidence_health(),
        EvidenceHealth::Incomplete {
            first_failure_code: opi_agent::evidence::EvidenceFailureCode::Emission,
            ..
        }
    ));
    assert_eq!(
        run.evidence_health().generation(),
        prior_generation.next(),
        "the supplied owning failure advances health exactly once"
    );
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::CleanupUnknown);
    assert_eq!(
        run.lifecycle_phase(),
        AgentRunLifecyclePhase::FinalizedOrAbandoned
    );
    assert_eq!(
        sink.abandoned.lock().unwrap().as_slice(),
        &[TerminalOutcome::CleanupUnknown]
    );

    assert!(run.abandon_evidence(&owning).is_err());
    assert_eq!(sink.abandoned.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn explicit_pending_compaction_abandon_retains_cleanup_failure_separately() {
    let sink = Arc::new(FinalizationFailureSink {
        abandon_fails: true,
        abandon_calls: AtomicUsize::new(0),
    });
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let pending = run.begin_compaction(CompactionTrigger::Manual).unwrap();
    drop(pending);
    let prior_generation = run.evidence_health().generation();
    let owning = EvidenceError::Emission {
        detail: "owning evidence failure".to_owned(),
    };

    let cleanup = run.abandon_evidence(&owning).unwrap_err();

    assert!(matches!(
        owning,
        EvidenceError::Emission { ref detail } if detail == "owning evidence failure"
    ));
    assert!(matches!(
        cleanup,
        EvidenceError::Finalization { ref detail } if detail == "cleanup confirmation failed"
    ));
    assert!(matches!(
        run.evidence_cleanup_error(),
        Some(EvidenceError::Finalization { detail }) if detail == "cleanup confirmation failed"
    ));
    assert_eq!(run.terminal_outcome(), &TerminalOutcome::CleanupUnknown);
    assert_eq!(
        run.evidence_health().generation(),
        prior_generation.next().next(),
        "owning and cleanup failures each advance health exactly once"
    );
    assert_eq!(sink.abandon_calls.load(Ordering::SeqCst), 1);
    assert!(run.abandon_evidence(&owning).is_err());
    assert_eq!(sink.abandon_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn successful_active_capture_consumption_abandons_and_fails_without_a_manifest() {
    let sink = Arc::new(AbandonRecordingSink::default());
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );

    let result = agent.prompt("go").await.into_execution_result();

    assert!(matches!(
        result,
        Err(AgentError::EvidenceFinalization(detail))
            if detail == "active evidence run was abandoned before execution result consumption"
    ));
    assert_eq!(sink.finalized.load(Ordering::SeqCst), 0);
    assert_eq!(
        sink.abandoned.lock().unwrap().as_slice(),
        [TerminalOutcome::Success]
    );
}

#[tokio::test]
async fn manifest_for_another_run_is_rejected_before_sink_finalization() {
    let sink = Arc::new(AbandonRecordingSink::default());
    let mut agent = make_agent_with_sink(
        vec![MockResponse::Events(text_response("done"))],
        Vec::new(),
        None,
        None,
        sink.clone(),
    );
    let mut run = agent.prompt("go").await;
    let other_run = IdentityAllocator::new().run_id();
    assert_ne!(other_run, run.run_id());
    let manifest = standalone_finalized_manifest(other_run, TerminalOutcome::Success);

    let error = run.finalize_evidence(&manifest).unwrap_err();

    assert!(matches!(
        error,
        EvidenceError::Finalization { ref detail }
            if detail == "manifest run does not match the run lifecycle"
    ));
    assert_eq!(sink.finalized.load(Ordering::SeqCst), 0);
    assert_eq!(sink.abandoned.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn sequential_uncertain_or_cancelled_tool_stops_every_later_side_effect() {
    for (outcome, expected_terminal) in [
        (
            UncertainToolOutcome::PartialSideEffect,
            TerminalOutcome::PartialSideEffect,
        ),
        (
            UncertainToolOutcome::CleanupUnknown,
            TerminalOutcome::CleanupUnknown,
        ),
        (UncertainToolOutcome::Cancelled, TerminalOutcome::Cancelled),
    ] {
        let first_executions = Arc::new(AtomicUsize::new(0));
        let second_executions = Arc::new(AtomicUsize::new(0));
        let registrations = common::registrations_from(vec![
            Box::new(SequentialUncertainTool {
                outcome,
                executions: first_executions.clone(),
            }),
            Box::new(SequentialCountingTool {
                name: "second".to_owned(),
                executions: second_executions.clone(),
            }),
        ]);
        let mut agent = make_agent(
            vec![MockResponse::Events(two_tool_response())],
            registrations,
            Some(common::permissive_authorizer()),
            None,
            Arc::new(InMemoryEvidenceSink::new()),
        );

        let run = agent.prompt("go").await;

        assert_eq!(run.terminal_outcome(), &expected_terminal);
        assert_eq!(first_executions.load(Ordering::SeqCst), 1);
        assert_eq!(
            second_executions.load(Ordering::SeqCst),
            0,
            "the later sequential tool must never cross its execution boundary"
        );
        let tool_results: Vec<_> = run
            .messages()
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(Message::ToolResult(result)) => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(tool_results.len(), 2, "every proposed call has a result");
        assert_eq!(tool_results[1].tool_name, "second");
        assert!(tool_results[1].is_error);
    }
}

// ===========================================================================
// In-band stream Error terminal: typed non-retryable failure, partial message
// retained, zero retries (never converted into a normally completed turn)
// ===========================================================================

#[tokio::test]
async fn in_band_stream_error_terminal_fails_the_run_without_retry() {
    let sink = Arc::new(InMemoryEvidenceSink::new());
    // Partial assistant payload delivered before the in-band error terminal.
    let mut partial = AssistantMessage {
        content: vec![],
        api: opi_ai::ApiKind::OpenAi,
        provider: "mock".to_owned(),
        model: "mock-model".to_owned(),
        response_model: None,
        response_id: None,
        usage: Usage::default(),
        stop_reason: StopReason::Error,
        error_message: Some("overloaded".to_owned()),
        timestamp_ms: 0,
    };
    partial.content.push(AssistantContent::Text {
        text: "partial before failure".to_owned(),
    });
    let start_partial = AssistantMessage {
        content: vec![],
        ..partial.clone()
    };
    let attempt = vec![
        AssistantStreamEvent::Start {
            partial: start_partial,
        },
        AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: "partial before failure".to_owned(),
            partial: partial.clone(),
        },
        AssistantStreamEvent::Error {
            reason: StopReason::Error,
            message: partial,
        },
    ];
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Events(attempt),
            // A second scripted response proves the failure is not retried: it
            // must never be consumed.
            MockResponse::Events(text_response("recovered")),
        ],
    );
    let calls = provider.call_log_handle();
    let collection = Arc::new(single_route_collection(Box::new(provider)));
    let mut agent = Agent::new(
        collection,
        Vec::new(),
        None,
        "mock:mock-model".to_owned(),
        None,
        InferenceConfig::default(),
        AgentLoopConfig {
            max_turns: 5,
            // Retry is deliberately enabled: an in-band error terminal is a
            // non-retryable provider failure even under a retry policy.
            retry: Some(fast_retry()),
        },
        Box::new(TestHooks),
    )
    .expect("agent builds");
    agent.set_evidence_sink(Some(sink));
    let run = agent.prompt("go").await;

    // The provider's complete terminal message stays visible in the run.
    let assistants: Vec<_> = run
        .messages()
        .iter()
        .filter_map(|message| match message {
            AgentMessage::Llm(Message::Assistant(assistant)) => Some(assistant),
            _ => None,
        })
        .collect();
    assert_eq!(
        assistants.len(),
        1,
        "the partial assistant message is retained by the failed run"
    );
    assert_eq!(assistants[0].error_message.as_deref(), Some("overloaded"));
    assert!(matches!(
        assistants[0].content.first(),
        Some(AssistantContent::Text { text }) if text == "partial before failure"
    ));

    let error = run.into_execution_result().unwrap_err();
    assert!(
        matches!(&error, AgentError::Provider(e) if matches!(
            e.provider_error(),
            ProviderError::StreamError(_)
        )),
        "the in-band error terminal fails the run with a typed stream error, got {error:?}"
    );
    assert!(
        matches!(&error, AgentError::Provider(e) if !e.provider_error().is_retryable()),
        "the typed stream error is non-retryable"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "the in-band error terminal is never retried (second response unconsumed)"
    );
}
