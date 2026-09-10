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
        model: "anthropic:model-alias".into(),
        system: None,
        messages,
        tools: vec![ToolDef {
            name: "read".into(),
            description: "Read a local fixture".into(),
            input_schema: json!({"type":"object","properties":{"path":{"type":"string"}}}),
        }],
        max_tokens: Some(4096),
        temperature: None,
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
        let body = provider.build_request_body(&request(vec![Message::Assistant(message.clone())]));
        let serialized = body.to_string();
        assert!(!serialized.contains("private-"));
        assert_eq!(
            body["messages"][0]["content"],
            json!([{"type":"text","text":"visible reasoning"},{"type":"text","text":"answer"}])
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
        let body =
            provider.build_request_body(&request(vec![Message::Assistant(opaque_only), user]));
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
    let body = provider.build_converse_body(&request(vec![opaque, mixed, user]));
    assert_eq!(
        body["messages"],
        json!([
            {"role":"assistant","content":[{"text":"reasoning"},{"text":"answer"}]},
            {"role":"user","content":[{"text":"continue"}]}
        ])
    );
    assert!(!body.to_string().contains("private-"));
}
