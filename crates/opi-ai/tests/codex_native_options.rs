//! Native and neutral Codex options exercised through real public dispatch.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::openai_codex_responses::{
    CodexChatApi, CodexOptions, CodexReasoningEffort, CodexReasoningSummary, CodexServiceTier,
    CodexTextVerbosity, CodexToolChoice, OpenAiCodexResponsesProvider,
};
use opi_ai::provider::{ProviderError, Request, ThinkingConfig, TransportPreference};
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, SimpleOptions,
    SimpleToolChoice, ThinkingLevel, ThinkingLevelMapping, WireApi,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use opi_ai::ws;
#[path = "support/ws_peer.rs"]
mod ws_peer;

struct LocalAuth;

struct CountedAuth {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    inner: LocalAuth,
}

impl AuthResolver for CountedAuth {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.inner.resolve()
    }
}

impl AuthResolver for LocalAuth {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: secrecy::SecretString::from("local-token"),
                account_id: Some("local-account".into()),
                base_url: None,
                provenance: Default::default(),
            })
        })
    }
}

fn model(thinking: bool) -> ModelInfo {
    ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(272_000, 128_000)
            .with_streaming(true)
            .with_thinking(thinking),
    )
}

fn request() -> Request {
    Request {
        projection: Default::default(),
        model: "openai-codex:gpt-5.4".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: Some(0),
        temperature: Some(0.0),
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        transport: TransportPreference::Sse,
    }
}

fn completed(id: &str) -> Value {
    json!({"type":"response.completed", "response":{
        "id": id, "model":"gpt-5.4", "status":"completed", "output":[],
        "usage":{"input_tokens":1,"output_tokens":0,"total_tokens":1}
    }})
}

async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {}\n\n", completed("resp-local"))),
        )
        .mount(&server)
        .await;
    server
}

async fn drain(mut call: opi_ai::ChatCall) {
    while let Some(result) = call.next().await {
        result.unwrap();
    }
}

#[tokio::test]
async fn selected_empty_model_id_fails_before_auth_for_native_and_simple() {
    let server = server().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![model(false)],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, model(false)).unwrap()),
            Arc::new(CountedAuth {
                calls: calls.clone(),
                inner: LocalAuth,
            }),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    for id in ["", " \t"] {
        let mut invalid = model(false);
        invalid.id = id.into();
        for native in [false, true] {
            let record = ChatModelRecord::new("openai-codex", invalid.clone());
            let call = if native {
                models.stream(record, request(), handle.options(CodexOptions::default()))
            } else {
                models.stream_simple(record, request(), SimpleOptions::default())
            };
            assert!(matches!(
                call.result().await.unwrap_err().cause(),
                ProviderError::Config(_)
            ));
        }
    }
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn simple_default_reasoning_uses_model_off_mapping_and_preserves_zero() {
    let server = server().await;
    let info = model(true).with_thinking_level_map(model(true).thinking_level_map.with_mapping(
        ThinkingLevel::None,
        ThinkingLevelMapping::Mapped("minimal".into()),
    ));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiCodexResponsesProvider::new(
                Some(server.uri()),
                vec![info.clone()],
                Arc::new(HttpClient::new()),
            )),
            Arc::new(LocalAuth),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    drain(models.stream_simple(
        ChatModelRecord::new("openai-codex", info),
        request(),
        SimpleOptions::default(),
    ))
    .await;
    let captures = server.received_requests().await.unwrap();
    assert_eq!(captures.len(), 1);
    let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
    assert_eq!(body["reasoning"], json!({"effort":"minimal"}));
    assert_eq!(body["temperature"], 0.0);
    assert_eq!(body["store"], false);
    assert!(body.get("max_output_tokens").is_none());
}

async fn capture_native(info: ModelInfo, request: Request, options: CodexOptions) -> Value {
    let server = server().await;
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let api = Arc::new(CodexChatApi::new(provider, info.clone()).unwrap());
    let mut models = Models::new();
    let handle = models
        .register_chat_api(api, Arc::new(LocalAuth), AuthProvenanceSource::Static)
        .unwrap();
    assert_eq!(handle.api_id().as_str(), "openai-codex-responses");
    drain(models.stream(
        ChatModelRecord::new("openai-codex", info),
        request,
        handle.options(options),
    ))
    .await;
    let captures = server.received_requests().await.unwrap();
    assert_eq!(captures.len(), 1);
    serde_json::from_slice(&captures[0].body).unwrap()
}

#[tokio::test]
async fn native_omission_ignores_neutral_effort_and_sends_own_defaults() {
    let mut request = request();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    let body = capture_native(model(true), request, CodexOptions::default()).await;
    assert_eq!(body["reasoning"], json!({"effort":"none"}));
    assert_eq!(body["text"], json!({"verbosity":"low"}));
    assert_eq!(body["tool_choice"], "auto");
    assert!(body.get("service_tier").is_none());
    let body = capture_native(model(false), self::request(), CodexOptions::default()).await;
    assert!(body.get("reasoning").is_none());
}

#[tokio::test]
async fn native_default_ignores_unsupported_neutral_thinking() {
    let mut request = request();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: Some(99_999),
    };
    let body = capture_native(model(false), request, CodexOptions::default()).await;
    assert!(body.get("reasoning").is_none());
}

#[tokio::test]
async fn native_effort_ignores_unmapped_neutral_level() {
    let info = model(true).with_thinking_level_map(
        model(true)
            .thinking_level_map
            .with_mapping(ThinkingLevel::Max, ThinkingLevelMapping::Unsupported),
    );
    let mut request = request();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::Max,
        budget_tokens: None,
    };
    let mut options = CodexOptions::default();
    options.reasoning_effort = Some(CodexReasoningEffort::Low);
    let body = capture_native(info, request, options).await;
    assert_eq!(body["reasoning"], json!({"effort":"low","summary":"auto"}));
}

#[tokio::test]
async fn native_dispatch_uses_callers_same_id_map_and_endpoint() {
    let construction_endpoint = server().await;
    let selected_endpoint = server().await;
    let construction = model(true).with_base_url(construction_endpoint.uri());
    let selected = model(true)
        .with_base_url(selected_endpoint.uri())
        .with_thinking_level_map(model(true).thinking_level_map.with_mapping(
            ThinkingLevel::Low,
            ThinkingLevelMapping::Mapped("minimal".into()),
        ));
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(construction_endpoint.uri()),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, construction).unwrap()),
            Arc::new(LocalAuth),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut options = CodexOptions::default();
    options.reasoning_effort = Some(CodexReasoningEffort::Low);
    let call = models.stream(
        ChatModelRecord::new("openai-codex", selected),
        request(),
        handle.options(options),
    );
    drop(models);
    drain(call).await;
    assert!(
        construction_endpoint
            .received_requests()
            .await
            .unwrap()
            .is_empty()
    );
    let captures = selected_endpoint.received_requests().await.unwrap();
    assert_eq!(captures.len(), 1);
    let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
    assert_eq!(body["model"], "gpt-5.4");
    assert_eq!(
        body["reasoning"],
        json!({"effort":"minimal","summary":"auto"})
    );
}

#[tokio::test]
async fn native_dispatch_uses_distinct_complete_model_id_and_mapping() {
    let endpoint = server().await;
    let construction = model(true);
    let mut selected =
        model(true).with_thinking_level_map(model(true).thinking_level_map.with_mapping(
            ThinkingLevel::High,
            ThinkingLevelMapping::Mapped("medium".into()),
        ));
    selected.id = "complete-custom-codex".into();
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(endpoint.uri()),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, construction).unwrap()),
            Arc::new(LocalAuth),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut options = CodexOptions::default();
    options.reasoning_effort = Some(CodexReasoningEffort::High);
    drain(models.stream(
        ChatModelRecord::new("openai-codex", selected),
        request(),
        handle.options(options),
    ))
    .await;
    let captures = endpoint.received_requests().await.unwrap();
    assert_eq!(captures.len(), 1);
    let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
    assert_eq!(body["model"], "complete-custom-codex");
    assert_eq!(
        body["reasoning"],
        json!({"effort":"medium","summary":"auto"})
    );
}

#[tokio::test]
async fn native_fields_override_common_effort_without_clamping_and_keep_null_tier() {
    let mut options = CodexOptions::default();
    options.reasoning_effort = Some(CodexReasoningEffort::Max);
    options.reasoning_summary = Some(CodexReasoningSummary::Detailed);
    options.service_tier = Some(CodexServiceTier::Null);
    options.text_verbosity = Some(CodexTextVerbosity::High);
    options.tool_choice = Some(CodexToolChoice::Required);
    let mut request = request();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::Low,
        budget_tokens: None,
    };
    let body = capture_native(model(true), request, options).await;
    assert_eq!(
        body["reasoning"],
        json!({"effort":"max","summary":"detailed"})
    );
    assert_eq!(body["service_tier"], Value::Null);
    assert!(body.get("service_tier").is_some());
    assert_eq!(body["text"], json!({"verbosity":"high"}));
    assert_eq!(body["tool_choice"], "required");
    assert_eq!(body["temperature"], 0.0);
    assert!(body.get("max_output_tokens").is_none());
}

#[tokio::test]
async fn native_none_mapping_and_nullable_summary_follow_source_presence_rules() {
    let mut options = CodexOptions::default();
    options.reasoning_summary = Some(CodexReasoningSummary::Off);
    let body = capture_native(model(true), request(), options.clone()).await;
    assert_eq!(body["reasoning"], json!({"effort":"none"}));
    options.reasoning_effort = Some(CodexReasoningEffort::None);
    options.reasoning_summary = Some(CodexReasoningSummary::Null);
    let body = capture_native(model(true), request(), options.clone()).await;
    assert_eq!(body["reasoning"], json!({"effort":"none","summary":"auto"}));
    let info = model(true).with_thinking_level_map(
        model(true)
            .thinking_level_map
            .with_mapping(ThinkingLevel::None, ThinkingLevelMapping::Unsupported),
    );
    let body = capture_native(info.clone(), request(), options.clone()).await;
    assert!(body.get("reasoning").is_none());
    options.reasoning_effort = Some(CodexReasoningEffort::Max);
    let body = capture_native(info, request(), options).await;
    assert_eq!(body["reasoning"], json!({"effort":"max","summary":"auto"}));
}

#[tokio::test]
async fn native_all_admitted_summary_and_service_tier_values_reach_wire() {
    for (summary, wire) in [
        (CodexReasoningSummary::Auto, "auto"),
        (CodexReasoningSummary::Concise, "concise"),
        (CodexReasoningSummary::Detailed, "detailed"),
        (CodexReasoningSummary::Off, "off"),
        (CodexReasoningSummary::On, "on"),
    ] {
        let mut options = CodexOptions::default();
        options.reasoning_effort = Some(CodexReasoningEffort::Low);
        options.reasoning_summary = Some(summary);
        let body = capture_native(model(false), request(), options).await;
        assert_eq!(body["reasoning"], json!({"effort":"low","summary":wire}));
    }
    for (tier, wire) in [
        (CodexServiceTier::Auto, "auto"),
        (CodexServiceTier::Default, "default"),
        (CodexServiceTier::Flex, "flex"),
        (CodexServiceTier::Scale, "scale"),
        (CodexServiceTier::Priority, "priority"),
        (CodexServiceTier::Fast, "fast"),
    ] {
        let mut options = CodexOptions::default();
        options.service_tier = Some(tier);
        let body = capture_native(model(false), request(), options).await;
        assert_eq!(body["service_tier"], wire);
    }
}

#[test]
fn typed_options_reject_wrong_json_types_and_unknown_values() {
    for value in [
        json!(false),
        json!(0),
        Value::Null,
        json!("off"),
        json!("unexpected"),
    ] {
        assert!(serde_json::from_value::<CodexReasoningEffort>(value).is_err());
    }
    for value in [json!(false), json!(0), json!("unexpected")] {
        assert!(serde_json::from_value::<CodexReasoningSummary>(value.clone()).is_err());
        assert!(serde_json::from_value::<CodexServiceTier>(value).is_err());
    }
    assert_eq!(
        serde_json::from_value::<CodexReasoningSummary>(Value::Null).unwrap(),
        CodexReasoningSummary::Null
    );
    assert_eq!(
        serde_json::from_value::<CodexServiceTier>(Value::Null).unwrap(),
        CodexServiceTier::Null
    );
    assert!(serde_json::from_value::<CodexToolChoice>(Value::Null).is_err());
    assert!(serde_json::from_value::<CodexTextVerbosity>(json!(false)).is_err());
}

#[test]
fn native_api_rejects_wrong_wire_and_incomplete_model() {
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        None,
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let wrong = ModelInfo::new(
        "gpt",
        "GPT",
        WireApi::OpenAiResponses,
        ModelCapabilities::new(100, 50),
    );
    assert!(matches!(
        CodexChatApi::new(provider.clone(), wrong),
        Err(ProviderError::Config(_))
    ));
    let mut invalid = model(true);
    invalid.capabilities.context_window = 0;
    assert!(matches!(
        CodexChatApi::new(provider, invalid),
        Err(ProviderError::Config(_))
    ));
}

#[tokio::test]
async fn registered_native_api_simple_route_consumes_neutral_effort() {
    let server = server().await;
    let info = model(true);
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(server.uri()),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, info.clone()).unwrap()),
            Arc::new(LocalAuth),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = request();
    request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::High,
        budget_tokens: None,
    };
    drain(models.stream_simple(
        ChatModelRecord::new("openai-codex", info),
        request,
        SimpleOptions::default(),
    ))
    .await;
    let captures = server.received_requests().await.unwrap();
    let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
    assert_eq!(body["reasoning"], json!({"effort":"high","summary":"auto"}));
}

#[tokio::test]
async fn simple_tool_choice_and_selected_record_reach_provider_and_registered_api() {
    for registered_api in [false, true] {
        let construction_endpoint = server().await;
        let selected_endpoint = server().await;
        let construction = model(true);
        let selected = model(true)
            .with_base_url(selected_endpoint.uri())
            .with_thinking_level_map(model(true).thinking_level_map.with_mapping(
                ThinkingLevel::High,
                ThinkingLevelMapping::Mapped("medium".into()),
            ));
        let provider = OpenAiCodexResponsesProvider::new(
            Some(construction_endpoint.uri()),
            vec![construction.clone()],
            Arc::new(HttpClient::new()),
        );
        let mut models = Models::new();
        if registered_api {
            models
                .register_chat_api(
                    Arc::new(CodexChatApi::new(Arc::new(provider), construction).unwrap()),
                    Arc::new(LocalAuth),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
        } else {
            models
                .register_route(
                    Box::new(provider),
                    Arc::new(LocalAuth),
                    AuthProvenanceSource::Static,
                    Default::default(),
                )
                .unwrap();
        }
        let mut request = request();
        request.thinking = ThinkingConfig {
            enabled: true,
            level: ThinkingLevel::High,
            budget_tokens: None,
        };
        request
            .context
            .append_system_change(opi_ai::transcript::SystemMessage {
                tools_added: vec![opi_ai::message::ToolDef {
                    name: "local_tool".into(),
                    description: "local declaration".into(),
                    input_schema: json!({"type":"object","properties":{}}),
                    constrained_sampling: None,
                }],
                ..Default::default()
            })
            .unwrap();
        drain(models.stream_simple(
            ChatModelRecord::new("openai-codex", selected),
            request,
            SimpleOptions {
                tool_choice: Some(SimpleToolChoice::None),
                ..Default::default()
            },
        ))
        .await;
        assert!(
            construction_endpoint
                .received_requests()
                .await
                .unwrap()
                .is_empty()
        );
        let captures = selected_endpoint.received_requests().await.unwrap();
        assert_eq!(captures.len(), 1);
        let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
        assert_eq!(body["tool_choice"], "none");
        assert_eq!(body["tools"].as_array().unwrap().len(), 1);
        assert_eq!(
            body["reasoning"],
            json!({"effort":"medium","summary":"auto"})
        );
    }
}

mod continuation {
    use super::*;
    use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
    use opi_ai::provider::CacheRetention;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::Mutex;
    use std::time::Duration;

    // A small uncompressed loopback oracle, independent of the private codec.
    fn read_frame(socket: &mut TcpStream) -> Result<(u8, Vec<u8>), String> {
        let mut head = [0; 2];
        socket.read_exact(&mut head).map_err(|e| e.to_string())?;
        if head[0] & 0x70 != 0 || head[1] & 0x80 == 0 {
            return Err("unexpected client frame flags".into());
        }
        let length = match head[1] & 0x7f {
            126 => {
                let mut bytes = [0; 2];
                socket.read_exact(&mut bytes).map_err(|e| e.to_string())?;
                u16::from_be_bytes(bytes) as usize
            }
            127 => return Err("oversize test frame".into()),
            n => usize::from(n),
        };
        let mut mask = [0; 4];
        socket.read_exact(&mut mask).map_err(|e| e.to_string())?;
        let mut body = vec![0; length];
        socket.read_exact(&mut body).map_err(|e| e.to_string())?;
        for (index, byte) in body.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
        Ok((head[0] & 0x0f, body))
    }

    fn write_frame(socket: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<(), String> {
        let mut frame = vec![0x80 | opcode];
        if payload.len() < 126 {
            frame.push(payload.len() as u8);
        } else {
            frame.push(126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        frame.extend_from_slice(payload);
        socket.write_all(&frame).map_err(|e| e.to_string())
    }

    #[tokio::test]
    async fn native_noninput_changes_reset_continuation_and_equal_options_reuse_it() {
        let captured = Arc::new(Mutex::new(Vec::<Value>::new()));
        let frames = captured.clone();
        let peer = ws_peer::Peer::spawn(
            move |mut socket| {
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| e.to_string())?;
                for index in 1..=3 {
                    let (opcode, payload) = read_frame(&mut socket)?;
                    assert_eq!(opcode, 1);
                    frames
                        .lock()
                        .unwrap()
                        .push(serde_json::from_slice(&payload).unwrap());
                    write_frame(
                        &mut socket,
                        1,
                        completed(&format!("response-{index}"))
                            .to_string()
                            .as_bytes(),
                    )?;
                }
                let (opcode, payload) = read_frame(&mut socket)?;
                assert_eq!(opcode, 8);
                write_frame(&mut socket, 8, &payload)?;
                Ok(())
            },
            move |head| ws_peer::response_101(head, &[]),
        );
        let info = model(true);
        let provider = Arc::new(OpenAiCodexResponsesProvider::new(
            Some(format!("http://{}", peer.url().authority())),
            vec![],
            Arc::new(
                HttpClientBuilder::new()
                    .redirects(RedirectPolicy::None)
                    .build()
                    .unwrap(),
            ),
        ));
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(CodexChatApi::new(provider, info.clone()).unwrap()),
                Arc::new(LocalAuth),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        for verbosity in [
            CodexTextVerbosity::Low,
            CodexTextVerbosity::High,
            CodexTextVerbosity::High,
        ] {
            let mut request = request();
            request.transport = TransportPreference::WebsocketCached;
            request.cache_retention = CacheRetention::Short;
            request.session_id = Some("native-continuation".into());
            let mut options = CodexOptions::default();
            options.text_verbosity = Some(verbosity);
            drain(models.stream(
                ChatModelRecord::new("openai-codex", info.clone()),
                request,
                handle.options(options),
            ))
            .await;
        }
        models
            .cleanup_session_resources(Some("native-continuation"))
            .await
            .unwrap();
        peer.finish().unwrap();
        let frames = captured.lock().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0]["text"]["verbosity"], "low");
        assert!(frames[0].get("previous_response_id").is_none());
        assert_eq!(frames[1]["text"]["verbosity"], "high");
        assert!(
            frames[1].get("previous_response_id").is_none(),
            "a noninput native option changed"
        );
        assert_eq!(frames[1]["input"].as_array().unwrap().len(), 1);
        assert_eq!(frames[2]["previous_response_id"], "response-2");
        assert_eq!(frames[2]["input"], json!([]));
    }
}

#[path = "codex_native_options/deadlines.rs"]
mod deadlines;

#[path = "codex_native_options/lifetimes.rs"]
mod lifetimes;
