//! Signed and opaque thinking must survive the stream, storage and tool continuation.

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{AssistantMessage, Message, OutputContent, ToolDef, ToolResultMessage};
use opi_ai::provider::{CacheRetention, Provider, Request, ThinkingConfig};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sse(events: Vec<Value>) -> String {
    events
        .into_iter()
        .map(|event| {
            format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            )
        })
        .collect()
}

fn thinking_tool_stream() -> String {
    sse(vec![
        json!({"type":"message_start","message":{"id":"synthetic","model":"resolved-model"}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Synthetic reasoning."}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"opaque-"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"signature"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"redacted_thinking","data":"opaque-encrypted-data"}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"tool-1","name":"read","input":{}}}),
        json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"fixture\"}"}}),
        json!({"type":"content_block_stop","index":2}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":10}}),
        json!({"type":"message_stop"}),
    ])
}

fn request(messages: Vec<Message>) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "anthropic:model-alias".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![ToolDef {
                name: "read".into(),
                description: "Read a local fixture".into(),
                input_schema: json!({"type":"object","properties":{"path":{"type":"string"}}}),
                constrained_sampling: None,
            }],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: true,
            budget_tokens: Some(1024),
            ..ThinkingConfig::default()
        },
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: Some(std::time::Duration::from_secs(10)),
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn signed_and_redacted_thinking_survive_http_storage_and_tool_continuation() {
    for reported in [true, false] {
        let server = MockServer::start().await;
        let response = if reported {
            thinking_tool_stream()
        } else {
            let original = thinking_tool_stream();
            let without_model = original.replace(",\"model\":\"resolved-model\"", "");
            assert_ne!(original, without_model, "omit the response model field");
            without_model
        };
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(response)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let provider = AnthropicProvider::new(Some(server.uri()));
        let mut stream =
            provider.stream_prepared(request(vec![]), opi_ai::test_support::resolved_auth());
        let mut completed = None;
        while let Some(event) = stream.next().await {
            match event.expect("valid signed/opaque stream must not fail") {
                AssistantStreamEvent::Done { message, reason } => {
                    assert_eq!(reason, StopReason::ToolUse);
                    completed = Some(message);
                }
                AssistantStreamEvent::ThinkingDelta { delta, .. } => {
                    assert_eq!(delta, "Synthetic reasoning.")
                }
                _ => {}
            }
        }
        let message = completed.expect("complete tool turn");
        assert_eq!(message.raw_stop_reason.as_deref(), Some("tool_use"));
        let content = serde_json::to_value(&message.content).unwrap();
        assert_eq!(
            content[0],
            json!({"type":"thinking","thinking":"Synthetic reasoning.","signature":"opaque-signature"})
        );
        assert_eq!(
            content[1],
            json!({"type":"redacted_thinking","data":"opaque-encrypted-data"})
        );
        assert_eq!(
            message.model, "model-alias",
            "replay identity uses requested model, including aliases"
        );
        assert_eq!(
            message.response_model.as_deref(),
            Some(if reported { "resolved-model" } else { "" })
        );
        let stored = serde_json::to_vec(&Message::Assistant(message)).unwrap();
        let restored: Message = serde_json::from_slice(&stored).unwrap();
        let continuation = request(vec![
            restored,
            Message::ToolResult(ToolResultMessage {
                not_executed: None,
                tool_call_id: "tool-1".into(),
                tool_name: "read".into(),
                content: vec![OutputContent::Text {
                    text: "fixture contents".into(),
                }],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 0,
            }),
        ]);
        let results: Vec<_> = provider
            .stream_prepared(continuation, opi_ai::test_support::resolved_auth())
            .collect()
            .await;
        assert!(results.iter().all(Result::is_ok));
        let requests = server.received_requests().await.unwrap();
        let body: Value = serde_json::from_slice(&requests[1].body).unwrap();
        assert_eq!(body["messages"][0]["content"][0], content[0]);
        assert_eq!(body["messages"][0]["content"][1], content[1]);
        assert_eq!(
            body["messages"][0]["content"][2],
            json!({"type":"tool_use","id":"tool-1","name":"read","input":{"path":"fixture"}})
        );
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_result");
    }
}

#[test]
fn foreign_route_does_not_receive_signed_or_opaque_thinking() {
    let provider = AnthropicProvider::new(None);
    for (api, origin, model) in [
        ("Anthropic", "other-provider", "model-alias"),
        ("Anthropic", "anthropic", "other-model"),
    ] {
        let message: AssistantMessage = serde_json::from_value(json!({
            "content":[{"type":"thinking","thinking":"visible reasoning","signature":"private-signature"},
                {"type":"redacted_thinking","data":"private-opaque-data"}, {"type":"text","text":"answer"}],
            "api":api,"provider":origin,"model":model,"response_model":null,"response_id":null,
            "usage":opi_ai::stream::Usage::default(),"stop_reason":"stop","error_message":null,"timestamp_ms":0
        })).unwrap();
        let body = provider
            .build_request_body(&request(vec![Message::Assistant(message.clone())]))
            .unwrap();
        let serialized = body.to_string();
        assert!(!serialized.contains("private-"));
        // Pi 1.3 semantics (V-07/AIR-010): cross-identity thinking is
        // dropped entirely - readable, signed and redacted alike - and is
        // never converted into visible text.
        assert!(!serialized.contains("visible reasoning"));
        assert_eq!(
            body["messages"][0]["content"],
            json!([{"type":"text","text":"answer"}])
        );
        let mut opaque_only = message;
        opaque_only.content = serde_json::from_value(json!([
            {"type":"thinking","thinking":"","signature":"private-signature"},
            {"type":"redacted_thinking","data":"private-opaque-data"}
        ]))
        .unwrap();
        let user: Message = serde_json::from_value(
            json!({"role":"user","content":[{"type":"text","text":"continue"}],"timestamp_ms":0}),
        )
        .unwrap();
        let body = provider
            .build_request_body(&request(vec![Message::Assistant(opaque_only), user]))
            .unwrap();
        assert_eq!(
            body["messages"],
            json!([{"role":"user","content":[{"type":"text","text":"continue"}]}]),
            "omitting foreign opaque blocks must not leave an empty assistant message"
        );
    }
}

#[tokio::test]
async fn malformed_thinking_events_remain_errors_without_echoing_payloads() {
    let provider = AnthropicProvider::new(None);
    for bad in [
        json!({"type":"signature_delta","signature":17,"secret":"synthetic-private-canary"}),
        json!({"type":"synthetic-private-canary"}),
    ] {
        let events = sse(vec![
            json!({"type":"content_block_delta","index":0,"delta":bad}),
        ]);
        let results: Vec<_> = provider
            .stream_from_sse(&events, CancellationToken::new())
            .collect()
            .await;
        assert_eq!(results.len(), 1);
        let error = results[0]
            .as_ref()
            .expect_err("malformed known event must fail");
        assert!(!error.is_retryable());
        assert!(error.to_string().contains("content_block_delta"));
        assert!(!format!("{error:?}").contains("synthetic-private-canary"));
    }
}

#[tokio::test]
async fn signed_thinking_utf8_is_unchanged_across_http_byte_chunks() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for separator in ["\n", "\r\n", "\r"] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut received = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert_ne!(n, 0);
                received.extend_from_slice(&buffer[..n]);
                if let Some(end) = received.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&received[..end]).unwrap();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().unwrap())
                        })
                        .unwrap();
                    if received.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = thinking_tool_stream()
                .replace("Synthetic reasoning.", "推理：你好🌍")
                .replace('\n', separator);
            let mut response = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_vec();
            for byte in body.bytes() {
                response.extend_from_slice(b"1\r\n");
                response.push(byte);
                response.extend_from_slice(b"\r\n");
            }
            response.extend_from_slice(b"0\r\n\r\n");
            socket.write_all(&response).await.unwrap();
        });
        let provider = AnthropicProvider::new(Some(url));
        let results: Vec<_> = provider
            .stream_prepared(request(vec![]), opi_ai::test_support::resolved_auth())
            .collect()
            .await;
        server.await.unwrap();
        let message = results
            .into_iter()
            .find_map(|result| match result.expect("valid fragmented stream") {
                AssistantStreamEvent::Done { message, .. } => Some(message),
                _ => None,
            })
            .expect("complete fragmented stream");
        let content = serde_json::to_value(message.content).unwrap();
        assert_eq!(content[0]["thinking"], "推理：你好🌍");
        assert_eq!(content[0]["signature"], "opaque-signature");
        assert_eq!(content[1]["data"], "opaque-encrypted-data");
    }
}

#[tokio::test]
async fn signatures_stay_with_their_blocks_including_empty_thinking() {
    let body = sse(vec![
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"initial","signature":"prefix-"}}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"thinking","thinking":""}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"signature_delta","signature":"empty-signature"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"suffix"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"message_stop"}),
    ]);
    let provider = AnthropicProvider::new(None);
    let results: Vec<_> = provider
        .stream_from_sse(&body, CancellationToken::new())
        .collect()
        .await;
    let AssistantStreamEvent::Done { message, .. } = results.last().unwrap().as_ref().unwrap()
    else {
        panic!("completed message");
    };
    assert_eq!(
        serde_json::to_value(&message.content).unwrap(),
        json!([
            {"type":"thinking","thinking":"initial","signature":"prefix-suffix"},
            {"type":"thinking","thinking":"","signature":"empty-signature"}
        ])
    );
}

#[test]
fn bedrock_omits_opaque_only_assistants_and_preserves_readable_content() {
    let provider = opi_ai::bedrock::BedrockProvider::new(
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
    let opaque: Message = serde_json::from_value(json!({
        "role":"assistant","content":[{"type":"redacted_thinking","data":"private-opaque"}],
        "api":"Anthropic","provider":"anthropic","model":"model-alias","response_model":null,"response_id":null,
        "usage":opi_ai::stream::Usage::default(),"stop_reason":"stop","error_message":null,"timestamp_ms":0
    })).unwrap();
    let mut mixed = opaque.clone();
    let Message::Assistant(message) = &mut mixed else {
        panic!("assistant");
    };
    message.content.extend(
        serde_json::from_value::<Vec<opi_ai::message::AssistantContent>>(json!([
            {"type":"thinking","thinking":"reasoning","signature":"private-signature"},
            {"type":"text","text":"answer"}
        ]))
        .unwrap(),
    );
    let user: Message = serde_json::from_value(
        json!({"role":"user","content":[{"type":"text","text":"continue"}],"timestamp_ms":0}),
    )
    .unwrap();
    let body = provider
        .build_converse_body(&request(vec![opaque, mixed, user]))
        .unwrap();
    // The history is foreign to this Bedrock route, so replay drops every
    // thinking block (Pi 1.3 semantics) before the wire serializer runs;
    // only the visible answer survives.
    assert_eq!(
        body["messages"],
        json!([
            {"role":"assistant","content":[{"text":"answer"}]},
            {"role":"user","content":[{"text":"continue"}]}
        ])
    );
    assert!(!body.to_string().contains("private-"));
    assert!(!body.to_string().contains("reasoning"));
}

#[tokio::test]
async fn compiled_effort_survives_storage_and_replays_before_the_corresponding_assistant() {
    use opi_ai::anthropic::{AnthropicMessagesChatApi, AnthropicMessagesOptions};
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::message::{InputContent, UserMessage};
    use std::sync::Arc;

    let server = MockServer::start().await;
    let mut model = Catalog::builtin()
        .unwrap()
        .model("anthropic", CatalogOperation::Chat, "claude-fable-5-1")
        .unwrap()
        .to_chat_model_info()
        .unwrap();
    model.base_url = Some(server.uri());
    let response = sse(vec![
        json!({"type":"message_start","message":{"id":"received-id","model":model.id,
            "usage":{"input_tokens":100,"output_tokens":0}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking",
            "thinking":"stored reasoning","signature":"stored-signature"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"answer"}}),
        json!({"type":"content_block_stop","index":1}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}),
        json!({"type":"message_stop"}),
    ]);
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(response),
        )
        .expect(2)
        .mount(&server)
        .await;
    let provider = Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
    let mut models = opi_ai::Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                "synthetic-key".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let record = opi_ai::ChatModelRecord::new("anthropic", model);
    let user = || {
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "continue".into(),
            }],
            timestamp_ms: 0,
        })
    };
    let input = |messages| {
        let mut request = request(vec![]);
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        request
    };
    let mut low = AnthropicMessagesOptions::default();
    low.effort = Some(opi_ai::ThinkingLevel::Low);
    let first = models
        .complete(record.clone(), input(vec![user()]), handle.options(low))
        .await
        .unwrap();
    assert_eq!(first.provider_thinking_level.as_deref(), Some("low"));
    let stored = serde_json::to_vec(&Message::Assistant(first)).unwrap();
    let restored: Message = serde_json::from_slice(&stored).unwrap();
    let Message::Assistant(message) = &restored else {
        unreachable!()
    };
    assert_eq!(message.provider_thinking_level.as_deref(), Some("low"));
    assert_eq!(message.wire, Some(opi_ai::WireApi::AnthropicMessages));
    let mut high = AnthropicMessagesOptions::default();
    high.effort = Some(opi_ai::ThinkingLevel::High);
    let second = models
        .complete(
            record,
            input(vec![user(), restored, user()]),
            handle.options(high),
        )
        .await
        .unwrap();
    assert_eq!(second.provider_thinking_level.as_deref(), Some("high"));
    assert!(second.projection.unwrap().replay.losses.is_empty());
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let body: Value = sent[1].body_json().unwrap();
    assert_eq!(
        body["messages"][1],
        json!({"role":"system","content":[],"output_config":{"effort":"low"}})
    );
    assert_eq!(body["messages"][2]["role"], "assistant");
    assert_eq!(
        body["messages"][2]["content"][0],
        json!({"type":"thinking","thinking":"stored reasoning","signature":"stored-signature"})
    );
    assert_eq!(
        body["messages"][4],
        json!({"role":"system","content":[],"output_config":{"effort":"high"}})
    );
    server.verify().await;
}

mod empty_signature_source_compat {
    use super::*;
    use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
    use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation};
    use opi_ai::message::{AssistantContent, InputContent, UserMessage};
    use opi_ai::provider::ProviderError;
    use opi_ai::replay::ReplayProjection;
    use opi_ai::{ChatModelRecord, Models, WireApi};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct FixtureAuth(AtomicUsize);

    impl AuthResolver for FixtureAuth {
        fn resolve(&self) -> opi_ai::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(ResolvedAuth {
                    scheme: AuthScheme::ApiKey,
                    secret: "synthetic-key".into(),
                    base_url: None,
                    account_id: None,
                    env: Default::default(),
                    provenance: Default::default(),
                })
            })
        }
    }

    fn source(allow_empty: bool) -> CatalogModel {
        let mut record = Catalog::builtin()
            .unwrap()
            .model(
                "fireworks",
                CatalogOperation::Chat,
                "accounts/fireworks/models/deepseek-v4p1-flash",
            )
            .unwrap()
            .clone();
        record.raw["compat"]["allowEmptySignature"] = json!(allow_empty);
        record
    }

    fn user() -> Message {
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        })
    }

    fn assistant(record: &CatalogModel, thinking: &str, signature: Option<&str>) -> Message {
        let mut message = opi_ai::test_support::base_assistant();
        message.provider = record.provider.clone();
        message.model = record.id.clone();
        message.wire = Some(WireApi::AnthropicMessages);
        message.timestamp_ms = 0;
        message.content = vec![AssistantContent::Thinking {
            thinking: thinking.into(),
            signature: signature.map(str::to_owned),
        }];
        Message::Assistant(message)
    }

    fn replay_request(messages: Vec<Message>, policy: ReplayProjection) -> Request {
        let mut input = request(vec![]);
        input.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages,
            tools: vec![],
        })
        .unwrap();
        input.thinking = ThinkingConfig::default();
        input.cache_retention = CacheRetention::Long;
        input.session_id = Some("session-complete-id".into());
        input.projection.replay = policy;
        input
    }

    fn setup(
        record: CatalogModel,
        server: &MockServer,
    ) -> (Models, ChatModelRecord, Arc<FixtureAuth>) {
        let mut model = record.to_chat_model_info().unwrap();
        model.base_url = Some(server.uri());
        model.capabilities.supports_cache_control = true;
        model.capabilities.supports_long_cache_retention = true;
        let provider = Arc::new(AnthropicProvider::for_route(
            record.provider.clone(),
            vec![model.clone()],
            None,
            Default::default(),
            Arc::new(opi_ai::http::HttpClient::new()),
            false,
        ));
        let api =
            opi_ai::anthropic::AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let auth = Arc::new(FixtureAuth::default());
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(api),
                auth.clone(),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        (models, ChatModelRecord::new(record.provider, model), auth)
    }

    async fn capture(
        record: CatalogModel,
        messages: Vec<Message>,
        policy: ReplayProjection,
    ) -> (Value, AssistantMessage) {
        let server = MockServer::start().await;
        let response = sse(vec![
            json!({"type":"message_start", "message":{"id":"received-id", "model":record.id,
                "usage":{"input_tokens":100,"output_tokens":0}}}),
            json!({"type":"message_delta", "delta":{"stop_reason":"end_turn"},
                "usage":{"output_tokens":10}}),
            json!({"type":"message_stop"}),
        ]);
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response),
            )
            .expect(1)
            .mount(&server)
            .await;
        let (models, record, auth) = setup(record, &server);
        let mut input = replay_request(messages, policy);
        input.model = record.model_id().into();
        let canonical = serde_json::to_vec(&input.context).unwrap();
        let outcome = models
            .complete_simple(record, input.clone(), Default::default())
            .await
            .expect("successful terminal fixture");
        assert_eq!(outcome.stop_reason, StopReason::Stop);
        assert_eq!(outcome.usage.input_tokens, 100);
        assert_eq!(outcome.usage.output_tokens, 10);
        assert_eq!(auth.0.load(Ordering::SeqCst), 1);
        assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        server.verify().await;
        (sent[0].body_json().unwrap(), outcome)
    }

    #[tokio::test]
    async fn c009_allow_empty_signature_materializes_empty_wire_signature() {
        let record = source(true);
        let assistant = assistant(&record, "reasoning", None);
        let (body, _) = capture(record, vec![user(), assistant, user()], Default::default()).await;
        assert_eq!(
            body["messages"][1]["content"][0],
            json!({"type":"thinking","thinking":"reasoning","signature":""})
        );
    }

    #[tokio::test]
    async fn c009_disallow_empty_signature_projects_thinking_as_text() {
        let record = source(false);
        let assistant = assistant(&record, "reasoning", None);
        let (body, _) = capture(record, vec![user(), assistant, user()], Default::default()).await;
        assert_eq!(body["messages"][1]["content"][0]["type"], "text");
    }

    #[tokio::test]
    async fn signature_and_blank_thinking_matrix_preserves_only_source_wire_branches() {
        for allow_empty in [false, true] {
            for signature in [
                None,
                Some(""),
                Some(" \t\r\n\u{feff}"),
                Some("  opaque-signature  "),
            ] {
                for thinking in ["", " \t\r\n\u{feff}", "reasoning 🙂 e\u{301}\n"] {
                    let record = source(allow_empty);
                    let assistant = assistant(&record, thinking, signature);
                    let (body, outcome) = capture(
                        record,
                        vec![user(), assistant, user()],
                        ReplayProjection::Strict,
                    )
                    .await;
                    let assistants: Vec<_> = body["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|message| message["role"] == "assistant")
                        .collect();
                    let signed = signature == Some("  opaque-signature  ");
                    let blank = thinking != "reasoning 🙂 e\u{301}\n";
                    if !signed && blank {
                        assert!(
                            assistants.is_empty(),
                            "{allow_empty}/{signature:?}/{thinking:?}"
                        );
                    } else {
                        assert_eq!(assistants.len(), 1);
                        let expected = if signed {
                            json!({"type":"thinking","thinking":thinking,"signature":signature.unwrap()})
                        } else if allow_empty {
                            json!({"type":"thinking","thinking":thinking,"signature":""})
                        } else {
                            json!({"type":"text","text":thinking,
                                "cache_control":{"type":"ephemeral"}})
                        };
                        assert_eq!(assistants[0]["content"], json!([expected]));
                    }
                    assert!(outcome.projection.unwrap().replay.losses.is_empty());
                }
            }
        }
    }

    #[tokio::test]
    async fn redacted_payload_and_signed_whitespace_thinking_are_exact_for_both_flags() {
        for allow_empty in [false, true] {
            let record = source(allow_empty);
            let Message::Assistant(mut message) = assistant(&record, " \t\n", Some("\u{85}"))
            else {
                unreachable!()
            };
            // U+0085 is not ECMAScript trim whitespace: its signature is real.
            message.content.extend([
                AssistantContent::RedactedThinking {
                    data: "opaque+/=秘密".into(),
                },
                AssistantContent::Text {
                    text: "visible".into(),
                    signature: None,
                },
            ]);
            let (body, outcome) = capture(
                record,
                vec![user(), Message::Assistant(message), user()],
                ReplayProjection::Strict,
            )
            .await;
            assert_eq!(
                body["messages"][1]["content"],
                json!([
                    {"type":"thinking","thinking":" \t\n","signature":"\u{85}"},
                    {"type":"redacted_thinking","data":"opaque+/=秘密"},
                    {"type":"text","text":"visible","cache_control":{"type":"ephemeral"}}
                ])
            );
            assert!(outcome.projection.unwrap().replay.losses.is_empty());
        }
    }

    fn foreign_assistant(record: &CatalogModel, identity: &str) -> Message {
        let Message::Assistant(mut message) = assistant(
            record,
            "private thinking canary",
            Some("private signature canary"),
        ) else {
            unreachable!()
        };
        message.content.extend([
            AssistantContent::RedactedThinking {
                data: "private opaque canary".into(),
            },
            AssistantContent::Text {
                text: "visible answer".into(),
                signature: None,
            },
        ]);
        match identity {
            "provider" => message.provider = "different".into(),
            "model" => message.model = "different".into(),
            "wire" => message.wire = Some(WireApi::OpenAiCompletions),
            "legacy" => message.wire = None,
            _ => unreachable!(),
        }
        Message::Assistant(message)
    }

    #[tokio::test]
    async fn cross_identity_drops_private_blocks_with_exact_replay_loss_facts() {
        for allow_empty in [false, true] {
            for identity in ["provider", "model", "wire", "legacy"] {
                let record = source(allow_empty);
                let message = foreign_assistant(&record, identity);
                let (body, outcome) = capture(
                    record,
                    vec![user(), message, user()],
                    ReplayProjection::NativeOrCollapse,
                )
                .await;
                assert_eq!(
                    body["messages"][1]["content"],
                    json!([
                        {"type":"text","text":"visible answer","cache_control":{"type":"ephemeral"}}
                    ])
                );
                assert!(!body.to_string().contains("private"));
                let facts = outcome.projection.unwrap();
                assert_eq!(
                    facts.replay.losses,
                    vec![opi_ai::replay::ReplayLoss::DroppedThinkingBlocks { count: 2 }]
                );
                assert!(!serde_json::to_string(&facts).unwrap().contains("private"));
            }
        }
    }

    #[tokio::test]
    async fn cross_identity_strict_replay_still_refuses_before_auth_and_send() {
        use opi_ai::replay::{ReplayError, ReplayLoss, ReplayRoute};

        for allow_empty in [false, true] {
            for identity in ["provider", "model", "wire", "legacy"] {
                let record = source(allow_empty);
                let mut input = replay_request(
                    vec![user(), foreign_assistant(&record, identity), user()],
                    ReplayProjection::Strict,
                );
                let canonical = serde_json::to_vec(&input.context).unwrap();
                assert_eq!(
                    opi_ai::replay::project_replay(
                        input.context.messages(),
                        &ReplayRoute {
                            wire: WireApi::AnthropicMessages,
                            provider: record.provider.clone(),
                            model: record.id.clone(),
                            supports_images: false,
                        },
                        ReplayProjection::Strict,
                    )
                    .unwrap_err(),
                    ReplayError::StrictLoss(vec![ReplayLoss::DroppedThinkingBlocks { count: 2 }]),
                );
                let server = MockServer::start().await;
                let (models, record, auth) = setup(record, &server);
                input.model = record.model_id().into();
                let failure = models
                    .complete_simple(record, input.clone(), Default::default())
                    .await
                    .unwrap_err();
                let ProviderError::Config(message) = failure.cause() else {
                    panic!(
                        "strict projection must fail with the existing typed configuration boundary"
                    );
                };
                assert!(message.contains("strict replay projection rejected lossy transforms"));
                assert!(!message.contains("private"));
                assert!(failure.message().projection.is_none());
                assert_eq!(auth.0.load(Ordering::SeqCst), 0);
                assert!(server.received_requests().await.unwrap().is_empty());
                assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
            }
        }
    }
}
