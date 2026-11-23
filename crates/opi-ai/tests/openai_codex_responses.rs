use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::auth::{AuthScheme, ResolvedAuth};
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::HttpClient;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolDef, UserMessage,
};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason, Usage};
use opi_ai::{ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMap, WireApi};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "openai_codex_responses/replay.rs"]
mod replay;

#[path = "openai_codex_responses/deflate_audit.rs"]
mod deflate_audit;

fn auth(secret: &str, base_url: Option<String>, account_id: Option<String>) -> ResolvedAuth {
    ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from(secret),
        base_url,
        account_id,
        provenance: opi_ai::AuthProvenance::default(),
    }
}

fn model(base_url: Option<String>) -> ModelInfo {
    let mut model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    );
    model.base_url = base_url;
    model
}

fn request() -> Request {
    Request {
        projection: Default::default(),
        transport: opi_ai::provider::TransportPreference::Sse,
        model: "openai-codex:gpt-5.4".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system prompt".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![ToolDef {
                name: "read".into(),
                description: "Read a file".into(),
                input_schema: serde_json::json!({"type":"object"}),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: Some(0.2),
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level: ThinkingLevel::High,
        },
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Short,
        session_id: Some("session-fixed".into()),
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

async fn drain(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> Option<ProviderError> {
    let mut stream = provider.stream_prepared(request, auth);
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => return None,
            Ok(_) => {}
            Err(error) => return Some(direct_error_cause(&error).clone()),
        }
    }
    None
}

async fn capture_stream(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> (Vec<String>, usize, usize) {
    let mut stream = provider.stream_prepared(request, auth);
    let mut captures = Vec::new();
    let mut events = 0;
    let mut errors = 0;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                events += 1;
                captures.push(format!("{event:?}"));
                captures.push(serde_json::to_string(&event).expect("serialize stream event"));
                if event.is_terminal() {
                    break;
                }
            }
            Err(error) => {
                errors += 1;
                captures.push(format!("{error:?} {error}"));
                break;
            }
        }
    }
    (captures, events, errors)
}

async fn typed_terminal(
    provider: &dyn Provider,
    request: Request,
    auth: ResolvedAuth,
) -> AssistantMessage {
    let mut stream = provider.stream_prepared(request, auth);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let mut terminal = None;
        while let Some(result) = stream.next().await {
            let event = result.expect("scripted Codex response must stream successfully");
            assert!(terminal.is_none(), "no event may follow the terminal Done");
            match event {
                AssistantStreamEvent::Done { reason, message } => {
                    assert_eq!(message.stop_reason, reason);
                    terminal = Some(message);
                }
                AssistantStreamEvent::Error { .. } => {
                    panic!("Codex response must finish with Done")
                }
                _ => {}
            }
        }
        terminal.expect("Codex response must produce exactly one Done and EOF")
    })
    .await
    .expect("Codex response must terminate within the peer deadline")
}

fn codex_completed_sse() -> String {
    concat!(
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"response-1\",\"model\":\"gpt-5.4\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
    )
    .into()
}

fn connection_limit_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("data/codex-connection-limit.json")).unwrap()
}

fn direct_error_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        other => other,
    }
}

fn assert_direct_projection_prefix(
    message: &opi_ai::message::AssistantMessage,
    request: &opi_ai::provider::Request,
) {
    use sha2::Digest as _;
    let facts = message
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
}

#[path = "openai_codex_responses/wire.rs"]
mod wire;

#[path = "openai_codex_responses/constrained_sampling.rs"]
mod constrained_sampling;

#[path = "openai_codex_responses/ws_path.rs"]
mod ws_path;

#[path = "openai_codex_responses/ws_continuation.rs"]
mod ws_continuation;

#[path = "openai_codex_responses/ws_branches.rs"]
mod ws_branches;

#[path = "openai_codex_responses/ws_cancellation.rs"]
mod ws_cancellation;

#[path = "openai_codex_responses/ws_resources.rs"]
mod ws_resources;

#[path = "openai_codex_responses/ws_protocol_regressions.rs"]
mod ws_protocol_regressions;

#[path = "openai_codex_responses/terminal.rs"]
mod terminal;

#[path = "openai_codex_responses/websocket_idle.rs"]
mod websocket_idle;

#[path = "openai_codex_responses/websocket_idle_timeout.rs"]
mod websocket_idle_timeout;

#[path = "openai_codex_responses/session_fallback_cleanup.rs"]
mod session_fallback_cleanup;
