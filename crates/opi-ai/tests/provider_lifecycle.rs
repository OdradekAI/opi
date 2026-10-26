//! Contract tests for Anthropic prepared attempts through `stream_prepared` —
//! real HTTP-level verification.
//!
//! Covers C1/H2: POST endpoint, SSE streaming, HTTP error mapping,
//! no-terminal-event detection, and cancellation.
//!
//! Uses wiremock to simulate Anthropic's SSE endpoint without live API calls.

use futures_util::StreamExt;
use opi_ai::anthropic::AnthropicProvider;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request};
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_request(cancel: CancellationToken) -> Request {
    Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4".into(),
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
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
    }
}

fn text_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_lc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}

event: message_stop
data: {"type":"message_stop"}

"#
}

fn tool_call_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_tc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":20,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"read_file","input":{}}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\"/tmp/x\"}"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":30}}

event: message_stop
data: {"type":"message_stop"}

"#
}

fn incomplete_sse_fixture() -> &'static str {
    r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_inc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":5,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Partial..."}}

"#
}

// ---------------------------------------------------------------------------
// Happy path: text streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_text_response_produces_correct_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    assert!(matches!(events[0], AssistantStreamEvent::Start { .. }));
    assert!(events.iter().any(|e| matches!(
        e,
        AssistantStreamEvent::TextDelta { delta, .. } if delta == "Hello"
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        AssistantStreamEvent::TextDelta { delta, .. } if delta == " world"
    )));

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, message } = done {
        assert_eq!(*reason, StopReason::Stop);
        let text: String = message
            .content
            .iter()
            .filter_map(|c| match c {
                opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, "Hello world");
    }
}

// ---------------------------------------------------------------------------
// Happy path: tool call streaming
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_tool_call_response_produces_correct_events() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(tool_call_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                let is_terminal = event.is_terminal();
                events.push(event);
                if is_terminal {
                    break;
                }
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
    );

    let done = events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("should have Done event");
    if let AssistantStreamEvent::Done { reason, .. } = done {
        assert_eq!(*reason, StopReason::ToolUse);
    }
}

// ---------------------------------------------------------------------------
// HTTP error mapping
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_http_401_maps_to_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::AuthFailed(msg)) => {
            assert!(
                msg.contains("provider rejected credentials"),
                "should mention auth failure: {msg}"
            );
        }
        other => panic!("expected AuthFailed, got: {other:?}"),
    }
}

#[tokio::test]
async fn stream_http_429_maps_to_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string(
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"too many requests"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let result = stream.next().await.expect("should have event");
    assert!(
        matches!(result, Err(ProviderError::RateLimited { .. })),
        "expected RateLimited, got: {result:?}"
    );
}

#[tokio::test]
async fn stream_http_500_maps_to_provider_side() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string(
            r#"{"type":"error","error":{"type":"api_error","message":"internal"}}"#,
        ))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let result = stream.next().await.expect("should have event");
    match result {
        Err(ProviderError::ProviderSide(msg)) => {
            assert!(msg.contains("HTTP 500"), "should mention status: {msg}");
        }
        other => panic!("expected ProviderSide, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// No terminal event
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_no_terminal_event_produces_stream_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(incomplete_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );

    let mut saw_stream_error = false;
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                if event.is_terminal() {
                    break;
                }
            }
            Err(error)
                if error.category() == opi_ai::provider::ProviderErrorCategory::Stream
                    && error.to_string().contains("terminal event") =>
            {
                saw_stream_error = true;
                break;
            }
            Err(_) => break,
        }
    }
    assert!(
        saw_stream_error,
        "should produce StreamError about missing terminal event"
    );
}

// ---------------------------------------------------------------------------
// Cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_cancellation_drains_without_hang_after_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    // Read at least one event then cancel
    let first = stream.next().await.expect("should have Start event");
    assert!(first.is_ok(), "first event should be Ok");
    cancel.cancel();

    let drain = async {
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) if event.is_terminal() => break,
                Err(_) => break,
                _ => {}
            }
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(2), drain)
        .await
        .expect("stream must drain promptly after cancellation (no hang)");
}

// ---------------------------------------------------------------------------
// Request contract verification
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stream_sends_correct_headers_and_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key-123"))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new(Some(server.uri()));
    // The mock matches x-api-key "test-key-123", so build a ResolvedAuth whose
    // secret is exactly that (resolved_auth()'s placeholder is "test-key").
    let auth = opi_ai::auth::ResolvedAuth {
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("test-key-123"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::auth::AuthProvenance::default(),
    };
    let mut stream = provider.stream_prepared(make_request(CancellationToken::new()), auth);

    // Consume stream to trigger the request
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // wiremock verifies that all matchers were satisfied
    server.verify().await;
}

// ---------------------------------------------------------------------------
// Task 20.5 (AIR-006/V-02): byte-exact framing, replacement decoding,
// keepalive/large-event acceptance, final tool arguments (AIR-048), and
// producer barriers (V-04/AIR-004/005).
// ---------------------------------------------------------------------------

/// A raw-TCP SSE server: reads the request head+body, then writes the given
/// payload as a chunked response with one byte per chunk. Returns
/// `(base_url, shutdown)`: awaiting `shutdown` completes once the client-side
/// connection is observed closed (EOF on the server socket).
async fn byte_chunked_sse_server(payload: Vec<u8>) -> (String, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut received = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0, "client must send its request before streaming");
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
        let mut response =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n"
                .to_vec();
        for byte in payload {
            response.extend_from_slice(b"1\r\n");
            response.push(byte);
            response.extend_from_slice(b"\r\n");
        }
        response.extend_from_slice(b"0\r\n\r\n");
        let _ = socket.write_all(&response).await;
    });
    (url, server)
}

fn text_with_multibyte_fixture(separator: &str, keepalive: bool) -> Vec<u8> {
    let mut body = String::new();
    if keepalive {
        body.push_str(": keepalive\n\n");
        body.push_str(": ping\n\n");
    }
    for (name, payload) in [
        (
            "message_start",
            r#"{"type":"message_start","message":{"id":"msg_mb","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
        ),
        (
            "content_block_start",
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        ),
        (
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"你好🌍"}}"#,
        ),
        (
            "content_block_delta",
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" trailing"}}"#,
        ),
        (
            "content_block_stop",
            r#"{"type":"content_block_stop","index":0}"#,
        ),
        (
            "message_delta",
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}"#,
        ),
        ("message_stop", r#"{"type":"message_stop"}"#),
    ] {
        body.push_str(&format!("event: {name}\ndata: {payload}\n"));
        body.push_str(separator);
    }
    body.into_bytes()
}

#[tokio::test]
async fn anthropic_every_byte_splits_with_crlf_keepalive_and_multibyte_frame_identically() {
    for separator in ["\n", "\r\n", "\r"] {
        let (url, server) =
            byte_chunked_sse_server(text_with_multibyte_fixture(separator, true)).await;
        let provider = AnthropicProvider::new(Some(url));
        let results: Vec<_> = provider
            .stream_prepared(
                make_request(CancellationToken::new()),
                opi_ai::test_support::resolved_auth(),
            )
            .collect()
            .await;
        let _ = server.await;
        let message = results
            .into_iter()
            .find_map(|result| match result.expect("valid fragmented stream") {
                AssistantStreamEvent::Done { message, .. } => Some(message),
                _ => None,
            })
            .expect("complete fragmented stream reaches its terminal");
        let text = message
            .content
            .iter()
            .find_map(|content| match content {
                opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .expect("text block present");
        assert_eq!(text, "你好🌍 trailing", "separator={separator:?}");
    }
}

#[tokio::test]
async fn anthropic_large_lawful_event_beyond_withdrawn_caps_is_accepted() {
    // A single lawful SSE event far beyond the withdrawn 16 KiB-era defaults
    // must stream without a default size cap. (Single-shot body: the
    // byte-split suites cover framing; this one covers absence of caps.)
    let large_text = "x".repeat(300 * 1024);
    let mut body = String::new();
    body.push_str("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_big\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n");
    body.push_str("event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");
    body.push_str(&format!(
        "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"{large_text}\"}}}}\n\n"
    ));
    body.push_str(
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    );
    body.push_str("event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}\n\n");
    body.push_str("event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    let message = results
        .into_iter()
        .find_map(|result| match result.expect("large lawful stream") {
            AssistantStreamEvent::Done { message, .. } => Some(message),
            _ => None,
        })
        .expect("large lawful event completes");
    let text = message
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::Text { text } => Some(text.len()),
            _ => None,
        })
        .expect("text block present");
    assert_eq!(text, 300 * 1024);
}

#[tokio::test]
async fn anthropic_invalid_utf8_bytes_are_replaced_not_fatal() {
    // AIR-006: preserve the adapter's incremental TextDecoder replacement
    // semantics rather than adding a fatal SSE UTF-8 rejection.
    let mut body: Vec<u8> = b"event: message_start\ndata: ".to_vec();
    body.extend_from_slice(
        br#"{"type":"message_start","message":{"id":"msg_inv","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4","stop_reason":null,"usage":{"input_tokens":1,"output_tokens":0}}}"#.as_slice(),
    );
    body.extend_from_slice(b"\n\n");
    body.extend_from_slice(b"event: content_block_start\ndata: ");
    // A lone continuation byte is invalid UTF-8 anywhere it appears.
    body.extend_from_slice(
        br#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#
            .as_slice(),
    );
    body.extend_from_slice(b"\n\n");
    let mut delta: Vec<u8> =
        br#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"a"#
            .to_vec();
    delta.push(0x80); // invalid lone continuation byte inside the JSON string
    delta.extend_from_slice(br#"b"}}"#.as_slice());
    body.extend_from_slice(b"event: content_block_delta\ndata: ");
    body.extend_from_slice(&delta);
    body.extend_from_slice(b"\n\n");
    body.extend_from_slice(
        b"\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );

    let (url, server) = byte_chunked_sse_server(body).await;
    let provider = AnthropicProvider::new(Some(url));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    let _ = server.await;
    let outcome = results
        .into_iter()
        .find_map(|result| match result {
            Ok(AssistantStreamEvent::Done { message, .. }) => Some(Ok(message)),
            Err(error) => Some(Err(error)),
            Ok(_) => None,
        })
        .expect("stream reaches a terminal or typed failure");
    match outcome {
        Ok(message) => {
            let text = message
                .content
                .iter()
                .find_map(|content| match content {
                    opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .expect("text block present");
            assert_eq!(text, "a\u{FFFD}b", "invalid byte is replaced, not fatal");
        }
        Err(error) => panic!("invalid UTF-8 must be replaced, not fatal: {error}"),
    }
}

fn tool_arguments_fixture(partial_json: &str) -> String {
    let escaped = partial_json.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "event: message_start\ndata: {{\"type\":\"message_start\",\"message\":{{\"id\":\"msg_args\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{{\"input_tokens\":20,\"output_tokens\":0}}}}}}\n\n\
         event: content_block_start\ndata: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"tool_use\",\"id\":\"toolu_args\",\"name\":\"read_file\",\"input\":{{}}}}}}\n\n\
         event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":\"{escaped}\"}}}}\n\n\
         event: content_block_stop\ndata: {{\"type\":\"content_block_stop\",\"index\":0}}\n\n\
         event: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":30}}}}\n\n\
         event: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"
    )
}

async fn anthropic_final_tool_call(partial_json: &str) -> opi_ai::message::ToolCall {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(tool_arguments_fixture(partial_json))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        )
        .collect()
        .await;
    results
        .into_iter()
        .find_map(|result| match result.expect("valid tool stream") {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .expect("tool call terminal event")
}

#[tokio::test]
async fn anthropic_final_tool_arguments_empty_input_normalizes_to_object() {
    let tool_call = anthropic_final_tool_call("").await;
    assert_eq!(tool_call.arguments, "{}");
    assert_eq!(tool_call.arguments_error, None);
}

#[tokio::test]
async fn anthropic_final_tool_arguments_invalid_escape_repairs_literal_text() {
    let tool_call = anthropic_final_tool_call(r#"{"cmd":"\usepackage{x}","n":2}"#).await;
    assert_eq!(tool_call.arguments_error, None);
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&tool_call.arguments).expect("repaired arguments parse");
    assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
    assert_eq!(parsed["n"], 2);
}

#[tokio::test]
async fn anthropic_final_tool_arguments_truncated_keeps_raw_and_marks_error() {
    let raw = r#"{"path": "trunc"#;
    let tool_call = anthropic_final_tool_call(raw).await;
    assert_eq!(tool_call.arguments, raw);
    assert_eq!(
        tool_call.arguments_error.as_deref(),
        Some("final tool arguments were not a complete JSON object")
    );
}

#[tokio::test]
async fn anthropic_final_tool_arguments_non_object_keeps_raw_and_marks_error() {
    let tool_call = anthropic_final_tool_call("[1,2]").await;
    assert_eq!(tool_call.arguments, "[1,2]");
    assert!(tool_call.arguments_error.is_some());
}

// ---------------------------------------------------------------------------
// Producer barriers (V-04/AIR-004/005): cancellation priority over buffered
// events at confirmed wait points, and drop releasing HTTP waits.
// ---------------------------------------------------------------------------

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Staged raw-TCP SSE server: reads the request, writes the prologue bytes,
/// signals `sent`, then holds the connection open until the client closes it
/// (EOF), when it acks on the returned receiver.
async fn staged_sse_server(
    prologue: Vec<u8>,
) -> (
    String,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sent_tx, sent_rx) = tokio::sync::oneshot::channel();
    let (closed_tx, closed_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
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
        let head = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
        let chunked: Vec<u8> = std::iter::once(&head[..])
            .chain(
                prologue
                    .chunks(1)
                    .flat_map(|byte| [b"1\r\n".as_slice(), byte, b"\r\n".as_slice()]),
            )
            .flatten()
            .copied()
            .collect();
        let _ = socket.write_all(&chunked).await;
        let _ = sent_tx.send(());
        let mut sink = Vec::new();
        let _ = socket.read_to_end(&mut sink).await;
        let _ = closed_tx.send(());
    });
    (url, sent_rx, closed_rx)
}

fn many_delta_events_fixture(count: usize) -> Vec<u8> {
    let mut body = String::new();
    body.push_str("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_q\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"claude-sonnet-4\",\"stop_reason\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n");
    body.push_str("event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n");
    for i in 0..count {
        body.push_str(&format!(
            "event: content_block_delta\ndata: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"text_delta\",\"text\":\"d{i}\"}}}}\n\n"
        ));
    }
    body.into_bytes()
}

#[tokio::test]
async fn anthropic_cancel_discards_buffered_events_at_the_queue_barrier() {
    // Fill the bounded queue far beyond its 64 slots while the consumer has
    // not polled, then cancel: the first delivered item must be the typed
    // cancellation, never a pre-cancel buffered event.
    let (url, sent_rx, _closed_rx) = staged_sse_server(many_delta_events_fixture(200)).await;
    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    // Confirm the wait point: the server handed the whole payload to the
    // socket, so the producer has parsed events and is parked on the queue.
    sent_rx.await.expect("server wrote payload");

    // Confirm the queue wait point: one event is demonstrably buffered
    // behind the next, so pre-cancel events exist to discard.
    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("producer delivers into the queue")
        .expect("stream alive");
    assert!(first.is_ok());

    cancel.cancel();
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("cancellation must be visible without draining the queue");
    match first {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled => {}
        other => panic!("cancellation must take priority over buffered events, got {other:?}"),
    }
    assert!(
        stream.next().await.is_none(),
        "stream must stay at EOF after the cancellation barrier"
    );
}

#[tokio::test]
async fn anthropic_drop_ends_delivery_and_a_new_attempt_is_unaffected() {
    // Drop after the first delivered event while the server still streams:
    // the dropped attempt delivers nothing further (its queue and read waits
    // end with the attempt), and a fresh attempt on the same provider
    // completes normally. Socket teardown timing is a transport internal and
    // is not asserted; attempt-slot release at the public ChatStream seam is
    // covered by the stream_events suite.
    let (url, sent_rx, _closed_rx) = staged_sse_server(many_delta_events_fixture(4)).await;
    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream =
        provider.stream_prepared(make_request(cancel), opi_ai::test_support::resolved_auth());

    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("first event arrives")
        .expect("stream alive");
    assert!(first.is_ok());
    drop(stream);
    let _ = sent_rx.await;

    // A fresh logical attempt on the same provider completes normally.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut next = provider.stream_prepared(
        make_request(CancellationToken::new()),
        opi_ai::test_support::resolved_auth(),
    );
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), next.next())
        .await
        .expect("fresh attempt delivers")
        .expect("fresh attempt alive");
    assert!(outcome.is_ok());
}

#[tokio::test]
async fn anthropic_cancel_at_the_connect_barrier_is_immediately_visible() {
    // The server accepts the connection and reads the request but never
    // writes a response head: cancellation during the connect/response wait
    // must surface as the one typed cancellation without any delivery.
    use tokio::io::AsyncReadExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (connected_tx, connected_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut sink = Vec::new();
        let mut buffer = [0u8; 4096];
        // Read the full request head+body, then hold the response open.
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            if n == 0 {
                break;
            }
            sink.extend_from_slice(&buffer[..n]);
            if let Some(end) = sink.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&sink[..end]).unwrap();
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                if sink.len() >= end + 4 + length {
                    break;
                }
            }
        }
        let _ = connected_tx.send(());
        let mut drain = Vec::new();
        let _ = socket.read_to_end(&mut drain).await;
    });

    let provider = AnthropicProvider::new(Some(url));
    let cancel = CancellationToken::new();
    let mut stream = provider.stream_prepared(
        make_request(cancel.clone()),
        opi_ai::test_support::resolved_auth(),
    );

    connected_rx.await.expect("server accepted the request");
    cancel.cancel();
    use futures_util::StreamExt as _;
    let first = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .expect("cancellation must be visible during the connect wait");
    match first {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled => {}
        other => panic!("expected typed cancellation, got {other:?}"),
    }
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn anthropic_failures_preserve_unpublished_mapper_state() {
    use opi_ai::message::AssistantContent;
    use opi_ai::stream::ChatStream;

    let start = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"partial\",\"model\":\"claude\",\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\n";
    let middle = concat!(
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"x\",\"name\":\"effect\",\"input\":{}}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"x\\\":\"}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"redacted_thinking\",\"data\":\"opaque-artificial\"}}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n",
    );
    for ending in [
        "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"secret-error-canary\"}}\n\n",
        "",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{},\"usage\":{\"output_tokens\":-1}}\n\n",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(format!("{start}{middle}{ending}")),
            )
            .mount(&server)
            .await;
        let provider = AnthropicProvider::new(Some(server.uri()));
        let failure = ChatStream::new(provider.stream_prepared(
            make_request(CancellationToken::new()),
            opi_ai::test_support::resolved_auth(),
        ))
        .result()
        .await
        .expect_err("provider error or premature EOF");
        assert_eq!(failure.message().usage.output_tokens, 7);
        assert_eq!(failure.message().usage.input_tokens, 10);
        assert!(
            matches!(&failure.message().content[0], AssistantContent::ToolCall { tool_call } if tool_call.arguments == "{\"x\":")
        );
        assert!(
            matches!(&failure.message().content[1], AssistantContent::RedactedThinking { data } if data == "opaque-artificial")
        );
        assert!(!format!("{:?}", failure.cause()).contains("secret-error-canary"));
    }
}

async fn assert_drop_closes_owned_http_attempt(bedrock: bool, response_started: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let cancel = CancellationToken::new();
    let provider: Box<dyn Provider> = if bedrock {
        Box::new(opi_ai::bedrock::BedrockProvider::new(
            Some(url),
            std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        ))
    } else {
        Box::new(opi_ai::gemini::GeminiProvider::new(Some(url)))
    };
    let auth = if bedrock {
        opi_ai::ResolvedAuth::aws_sigv4(
            opi_ai::AwsSigV4Credentials {
                access_key_id: "offline-public-id".into(),
                secret_access_key: "offline-public-secret".into(),
                session_token: None,
                region: "us-east-1".into(),
                expires_at: None,
            },
            Default::default(),
        )
    } else {
        opi_ai::test_support::resolved_auth()
    };
    let mut request = make_request(cancel.clone());
    request.model = if bedrock {
        "bedrock:amazon.nova-lite-v1:0"
    } else {
        "gemini:gemini-2.5-flash"
    }
    .into();
    let mut stream = provider.stream_prepared(request, auth);
    let (mut socket, _) =
        tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
            .await
            .unwrap()
            .unwrap();
    let mut received = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let count = socket.read(&mut buf).await.unwrap();
        assert_ne!(count, 0);
        received.extend_from_slice(&buf[..count]);
        if let Some(end) = received.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&received[..end]).unwrap();
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if received.len() >= end + 4 + length {
                break;
            }
        }
    }
    if response_started {
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let body = if bedrock {
            opi_ai::bedrock::event_stream::build_test_frame(
                "messageStart",
                "application/json",
                br#"{"role":"assistant"}"#,
            )
        } else {
            b"data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"barrier\"}]}}]}\n\n"
                .to_vec()
        };
        socket
            .write_all(format!("{:x}\r\n", body.len()).as_bytes())
            .await
            .unwrap();
        socket.write_all(&body).await.unwrap();
        socket.write_all(b"\r\n").await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
                .await
                .unwrap()
                .unwrap()
                .is_ok()
        );
    }
    drop(stream);
    let closed =
        tokio::time::timeout(std::time::Duration::from_secs(2), socket.read(&mut buf)).await;
    cancel.cancel(); // also release the producer when this regression fails
    assert!(
        matches!(closed, Ok(Ok(0))),
        "dropped owned attempt must release its HTTP wait: {closed:?}"
    );
}

#[tokio::test]
async fn gemini_drop_releases_connect_and_read_waits() {
    for response_started in [false, true] {
        assert_drop_closes_owned_http_attempt(false, response_started).await;
    }
}

#[tokio::test]
async fn bedrock_drop_releases_connect_and_read_waits() {
    for response_started in [false, true] {
        assert_drop_closes_owned_http_attempt(true, response_started).await;
    }
}
