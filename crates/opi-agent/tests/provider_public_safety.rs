//! Provider failures must cross the Agent boundary as typed, safe summaries.

mod common;

use std::io::Cursor;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use opi_agent::agent_loop;
use opi_agent::diagnostic::code::{
    CODE_PROVIDER_AUTH_FAILED, CODE_PROVIDER_REQUEST_FAILED, CODE_PROVIDER_SIDE,
};
use opi_agent::diagnostic::{Diagnostic, RedactionMode, Severity, redact_text};
use opi_agent::event::{AgentEvent, AgentEventSink};
use opi_agent::hooks::AgentHooks;
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::streaming_proxy::{ProxyConfig, ProxyEvent, ProxyHandler, StreamingProxy};
use opi_agent::{DiagnosticSink, RecordingSink, SdkCommand, SdkResponse};
use opi_ai::CollectionError;
use opi_ai::WireApi;
use opi_ai::auth::ResolvedAuth;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, EventStream, ModelInfo, Provider, ProviderError, ProviderErrorCategory,
    ProviderErrorSummary, Request, ThinkingConfig,
};
use opi_ai::test_support::{MockProvider, MockResponse, single_route_collection};

const PROVIDER_CANARY: &str = "upstream-opaque-canary-Q7vN4m2L";

struct NoopHooks;

impl AgentHooks for NoopHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }
}

fn user_message(text: &str) -> AgentMessage {
    AgentMessage::Llm(Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: text.to_owned(),
        }],
        timestamp_ms: 0,
    }))
}

fn null_event_sink() -> AgentEventSink {
    Box::new(|_: AgentEvent| {})
}

#[derive(Clone)]
struct DiagnosticProxyHandler {
    diagnostic: serde_json::Value,
}

impl ProxyHandler for DiagnosticProxyHandler {
    fn handle_command(&self, command: SdkCommand, event_sink: &dyn Fn(ProxyEvent)) -> SdkResponse {
        event_sink(ProxyEvent::Agent(self.diagnostic.clone()));
        SdkResponse::success(command.id(), command.command_name())
    }
}

struct CanaryProvider {
    models: Vec<ModelInfo>,
}

impl CanaryProvider {
    fn new() -> Self {
        let model_source = MockProvider::new("canary", Vec::new());
        Self {
            models: model_source.models().to_vec(),
        }
    }
}

impl Provider for CanaryProvider {
    fn id(&self) -> &str {
        "canary"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, _request: Request, _auth: ResolvedAuth) -> EventStream {
        Box::pin(stream::iter([Err(ProviderError::ProviderSide(
            ProviderErrorSummary::from_untrusted(PROVIDER_CANARY),
        ))]))
    }
}

fn request(model: &str) -> Request {
    Request {
        model: model.to_owned(),
        system: None,
        messages: Vec::new(),
        tools: Vec::new(),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[tokio::test]
async fn custom_provider_canary_is_absent_from_agent_error_and_diagnostic_serialization() {
    let collection = single_route_collection(Box::new(CanaryProvider::new()));
    let prepared = collection
        .prepare_call("canary:mock-model", request("canary:mock-model"))
        .await
        .expect("custom provider route should prepare");
    let provider_error = prepared
        .start_attempt()
        .expect("custom provider attempt should start")
        .next()
        .await
        .expect("custom provider should emit one item")
        .expect_err("custom provider should emit its safe provider error");

    let agent_error = AgentError::from(provider_error);
    let failure = match &agent_error {
        AgentError::Provider(failure) => failure,
        other => panic!("expected a provider AgentError, got {other:?}"),
    };
    assert_eq!(failure.category(), ProviderErrorCategory::Provider);
    assert_eq!(
        failure.summary().map(ProviderErrorSummary::as_str),
        Some("[REDACTED]")
    );

    let diagnostic = Diagnostic::from(&agent_error);
    assert_eq!(diagnostic.code, CODE_PROVIDER_SIDE);
    let public_payload = diagnostic.redacted_payload(RedactionMode::Summary);
    let public_json = serde_json::to_string(&public_payload).expect("public diagnostic JSON");
    let proxy = StreamingProxy::new(
        DiagnosticProxyHandler {
            diagnostic: serde_json::to_value(public_payload).expect("diagnostic payload JSON"),
        },
        ProxyConfig::default(),
    );
    let writer = proxy
        .run(
            Cursor::new("{\"type\":\"session_info\"}\n"),
            Cursor::new(Vec::new()),
            tokio_util::sync::CancellationToken::new(),
        )
        .expect("proxy should serialize the diagnostic event");
    let proxy_jsonl =
        String::from_utf8(writer.into_inner()).expect("proxy output must be UTF-8 JSONL");
    assert!(proxy_jsonl.ends_with('\n'));
    assert!(
        proxy_jsonl
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
    );
    for (surface, rendered) in [
        ("AgentError Display", agent_error.to_string()),
        ("Diagnostic Display", diagnostic.to_string()),
        ("public diagnostic JSON", public_json),
        ("proxy diagnostic NDJSON", proxy_jsonl),
    ] {
        assert!(
            !rendered.contains(PROVIDER_CANARY),
            "{surface} leaked the custom provider canary: {rendered}"
        );
    }
}

#[tokio::test]
async fn real_loop_failure_records_one_typed_diagnostic_and_proxy_frames_it_as_jsonl() {
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::Error(ProviderError::RequestFailed(
            ProviderErrorSummary::redacted(),
        ))],
    );
    let sink = Arc::new(RecordingSink::new());
    let context = AgentLoopContext {
        collection: Arc::new(single_route_collection(Box::new(provider))),
        registry: common::test_registry(vec![]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![user_message("hello")],
            ModelSelection::parse_spec("mock:mock-model").expect("canonical model selection"),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: Some(sink.clone() as Arc<dyn DiagnosticSink>),
        session_id: None,
        evidence_sink: None,
    };

    let error = agent_loop(
        context,
        AgentLoopConfig::default(),
        &NoopHooks,
        null_event_sink(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .into_execution_result()
    .expect_err("the provider failure must cross the real loop boundary");
    assert!(matches!(error, AgentError::Provider(_)));

    let expected = Diagnostic::from(&error);
    let snapshot = sink.snapshot();
    assert_eq!(snapshot, vec![expected.clone()]);
    assert_eq!(snapshot[0].code, CODE_PROVIDER_REQUEST_FAILED);
    assert_eq!(snapshot[0].severity, Severity::Error);

    let payload = serde_json::to_value(snapshot[0].redacted_payload(RedactionMode::Summary))
        .expect("diagnostic payload JSON");
    let proxy = StreamingProxy::new(
        DiagnosticProxyHandler {
            diagnostic: payload.clone(),
        },
        ProxyConfig::default(),
    );
    let writer = proxy
        .run(
            Cursor::new("{\"type\":\"session_info\"}\n"),
            Cursor::new(Vec::new()),
            tokio_util::sync::CancellationToken::new(),
        )
        .expect("proxy should serialize the diagnostic event");
    let output = String::from_utf8(writer.into_inner()).expect("proxy output must be UTF-8");
    assert!(output.ends_with('\n'), "JSONL output must end in a newline");
    let frames = output
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid JSONL frame"))
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 3, "ready, response, and diagnostic frames");
    assert_eq!(frames[2], payload);
}

fn provider_metadata(error: &ProviderError) -> serde_json::Value {
    match error {
        ProviderError::RateLimited { retry_after_ms } => {
            serde_json::json!({ "variant": "rate_limited", "retry_after_ms": retry_after_ms })
        }
        ProviderError::Timeout => serde_json::json!({ "variant": "timeout" }),
        ProviderError::RequestFailed(summary) => serde_json::json!({
            "variant": "request_failed",
            "summary": summary.as_str(),
        }),
        ProviderError::StreamError(summary) => serde_json::json!({
            "variant": "stream_error",
            "summary": summary.as_str(),
        }),
        ProviderError::AuthFailed(summary) => serde_json::json!({
            "variant": "auth_failed",
            "summary": summary.as_str(),
        }),
        ProviderError::CredentialNeeded { provider_id } => serde_json::json!({
            "variant": "credential_needed",
            "provider_id": provider_id,
        }),
        ProviderError::CredentialRevoked { provider_id } => serde_json::json!({
            "variant": "credential_revoked",
            "provider_id": provider_id,
        }),
        ProviderError::AccountIdMissing { provider_id } => serde_json::json!({
            "variant": "account_id_missing",
            "provider_id": provider_id,
        }),
        ProviderError::LoginCancelled { provider_id } => serde_json::json!({
            "variant": "login_cancelled",
            "provider_id": provider_id,
        }),
        ProviderError::Network(summary) => serde_json::json!({
            "variant": "network",
            "summary": summary.as_str(),
        }),
        ProviderError::Config(summary) => serde_json::json!({
            "variant": "config",
            "summary": summary.as_str(),
        }),
        ProviderError::UnknownModel {
            provider_id,
            model_id,
        } => serde_json::json!({
            "variant": "unknown_model",
            "provider_id": provider_id,
            "model_id": model_id,
        }),
        ProviderError::MissingWireRoute {
            provider_id,
            wire_api,
        } => serde_json::json!({
            "variant": "missing_wire_route",
            "provider_id": provider_id,
            "wire_api": wire_api.as_str(),
        }),
        ProviderError::WireCompatMismatch {
            model_id,
            wire_api,
            compat_wire,
        } => serde_json::json!({
            "variant": "wire_compat_mismatch",
            "model_id": model_id,
            "wire_api": wire_api.as_str(),
            "compat_wire": compat_wire.as_str(),
        }),
        ProviderError::ProviderSide(summary) => serde_json::json!({
            "variant": "provider_side",
            "summary": summary.as_str(),
        }),
        ProviderError::UnsupportedCapability(summary) => serde_json::json!({
            "variant": "unsupported_capability",
            "summary": summary.as_str(),
        }),
        ProviderError::Cancelled => serde_json::json!({ "variant": "cancelled" }),
    }
}

fn provider_summary(error: &ProviderError) -> Option<&ProviderErrorSummary> {
    match error {
        ProviderError::RequestFailed(summary)
        | ProviderError::StreamError(summary)
        | ProviderError::AuthFailed(summary)
        | ProviderError::Network(summary)
        | ProviderError::Config(summary)
        | ProviderError::ProviderSide(summary)
        | ProviderError::UnsupportedCapability(summary) => Some(summary),
        ProviderError::RateLimited { .. }
        | ProviderError::Timeout
        | ProviderError::CredentialNeeded { .. }
        | ProviderError::CredentialRevoked { .. }
        | ProviderError::AccountIdMissing { .. }
        | ProviderError::LoginCancelled { .. }
        | ProviderError::UnknownModel { .. }
        | ProviderError::MissingWireRoute { .. }
        | ProviderError::WireCompatMismatch { .. }
        | ProviderError::Cancelled => None,
    }
}

#[test]
fn every_provider_error_preserves_category_code_and_safe_typed_metadata() {
    let errors = vec![
        ProviderError::RateLimited {
            retry_after_ms: Some(713),
        },
        ProviderError::Timeout,
        ProviderError::RequestFailed(ProviderErrorSummary::redacted()),
        ProviderError::StreamError(ProviderErrorSummary::redacted()),
        ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected()),
        ProviderError::CredentialNeeded {
            provider_id: "credential-needed-provider".to_owned(),
        },
        ProviderError::CredentialRevoked {
            provider_id: "credential-revoked-provider".to_owned(),
        },
        ProviderError::AccountIdMissing {
            provider_id: "account-id-provider".to_owned(),
        },
        ProviderError::LoginCancelled {
            provider_id: "login-cancelled-provider".to_owned(),
        },
        ProviderError::Network(ProviderErrorSummary::redacted()),
        ProviderError::Config(ProviderErrorSummary::redacted()),
        ProviderError::UnknownModel {
            provider_id: "unknown-model-provider".to_owned(),
            model_id: "unknown-model-id".to_owned(),
        },
        ProviderError::MissingWireRoute {
            provider_id: "missing-wire-provider".to_owned(),
            wire_api: WireApi::OpenAiResponses,
        },
        ProviderError::WireCompatMismatch {
            model_id: "wire-mismatch-model".to_owned(),
            wire_api: WireApi::AnthropicMessages,
            compat_wire: WireApi::OpenAiCompletions,
        },
        ProviderError::ProviderSide(ProviderErrorSummary::redacted()),
        ProviderError::UnsupportedCapability(ProviderErrorSummary::redacted()),
        ProviderError::Cancelled,
    ];

    for provider_error in errors {
        let expected_category = provider_error.category();
        let expected_diagnostic = Diagnostic::from(&provider_error);
        let expected_metadata = provider_metadata(&provider_error);
        let expected_summary =
            provider_summary(&provider_error).map(|summary| summary.as_str().to_owned());

        let agent_error = AgentError::from(provider_error);
        let failure = match &agent_error {
            AgentError::Provider(failure) => failure,
            other => panic!("provider failure was remapped to a different AgentError: {other:?}"),
        };
        assert_eq!(failure.category(), expected_category);
        assert_eq!(failure.code(), expected_diagnostic.code);
        assert_eq!(
            failure.summary().map(ProviderErrorSummary::as_str),
            expected_summary.as_deref()
        );
        assert_eq!(
            provider_metadata(failure.provider_error()),
            expected_metadata
        );
        assert_eq!(Diagnostic::from(&agent_error), expected_diagnostic);
    }
}

#[test]
fn collection_failures_keep_typed_agent_categories_and_diagnostic_codes() {
    let mismatch = AgentError::from(CollectionError::RequestRouteMismatch {
        request_model: "other:model".to_owned(),
        route_provider: "canary".to_owned(),
        route_model: "mock-model".to_owned(),
    });
    assert!(matches!(
        mismatch,
        AgentError::RequestRouteMismatch {
            ref request_model,
            ref route_provider,
            ref route_model,
        } if request_model == "other:model"
            && route_provider == "canary"
            && route_model == "mock-model"
    ));
    assert_eq!(
        Diagnostic::from(&mismatch).code,
        CODE_PROVIDER_REQUEST_FAILED
    );

    let terminated = AgentError::from(CollectionError::CredentialTerminated {
        provider: "canary".to_owned(),
    });
    assert!(matches!(
        terminated,
        AgentError::CredentialTerminated { ref provider } if provider == "canary"
    ));
    assert_eq!(
        Diagnostic::from(&terminated).code,
        CODE_PROVIDER_AUTH_FAILED
    );
}

#[test]
fn agent_query_redaction_matches_provider_credential_vocabulary() {
    let keys = [
        "api_key",
        "api-key",
        "apikey",
        "key",
        "token",
        "access_token",
        "access-token",
        "refresh_token",
        "refresh-token",
        "session_token",
        "session-token",
        "access_key_id",
        "access-key-id",
        "secret_access_key",
        "secret-access-key",
        "secret",
        "password",
        "authorization",
        "proxy_authorization",
        "proxy-authorization",
    ];

    for key in keys {
        let canary = format!("opaque-{key}-value");
        let input = format!("https://example.test/path?{key}={canary}&ok=yes");
        for mode in [RedactionMode::Summary, RedactionMode::Verbose] {
            let output = redact_text(&input, mode);
            assert!(
                !output.contains(&canary),
                "query credential for key '{key}' leaked in {mode:?}: {output}"
            );
        }
    }
}
