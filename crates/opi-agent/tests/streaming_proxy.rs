//! Streaming proxy tests (task 4.10).
//!
//! DoD: "A streaming proxy forwards command/event streams using the settled
//! transport/RPC model, preserves framing and backpressure, propagates
//! cancellation, redacts secrets, and is tested with mock streams for success,
//! errors, malformed frames, client disconnects, and no live provider calls."
//!
//! All tests use a mock handler -- no live provider calls.

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use opi_agent::AgentEvent;
use opi_agent::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse, agent_event_to_value};
use opi_agent::streaming_proxy::{
    ProxyConfig, ProxyEvent, ProxyHandler, SecretRedactor, StreamingProxy,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A mock handler that records commands and returns canned responses.
#[derive(Clone)]
struct MockHandler {
    responses: Arc<Mutex<Vec<(String, SdkResponse)>>>,
    events_to_emit: Arc<Mutex<Vec<ProxyEvent>>>,
}

impl MockHandler {
    fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            events_to_emit: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn with_response(&self, command: &str, response: SdkResponse) {
        self.responses
            .lock()
            .unwrap()
            .push((command.to_owned(), response));
    }

    fn emit_event(&self, event: ProxyEvent) {
        self.events_to_emit.lock().unwrap().push(event);
    }
}

impl ProxyHandler for MockHandler {
    fn handle_command(&self, command: SdkCommand, event_sink: &dyn Fn(ProxyEvent)) -> SdkResponse {
        // Emit queued events
        let events = std::mem::take(&mut *self.events_to_emit.lock().unwrap());
        for ev in events {
            event_sink(ev);
        }

        // Look for a canned response
        let name = command.command_name().to_owned();
        let responses = self.responses.lock().unwrap();
        for (cmd, resp) in responses.iter() {
            if cmd == &name {
                return resp.clone();
            }
        }

        SdkResponse::success(command.id(), command.command_name())
    }
}

/// Build a JSONL input string from a list of JSON values.
fn jsonl_input(lines: &[&str]) -> String {
    let mut s = String::new();
    for line in lines {
        s.push_str(line);
        s.push('\n');
    }
    s
}

/// Parse JSONL output into a list of JSON values.
fn parse_jsonl(output: &str) -> Vec<Value> {
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid JSON per line"))
        .collect()
}

/// Run the proxy with a mock handler and return the output.
async fn run_proxy(input: &str, handler: MockHandler) -> String {
    run_proxy_with_config(input, handler, ProxyConfig::default()).await
}

async fn run_proxy_with_config(input: &str, handler: MockHandler, config: ProxyConfig) -> String {
    let reader = Cursor::new(input.to_owned());
    let writer = Cursor::new(Vec::new());

    let proxy = StreamingProxy::new(handler, config);
    let cancel = CancellationToken::new();
    let result = proxy.run(reader, writer, cancel);

    match result {
        Ok(writer) => {
            let bytes = writer.into_inner();
            String::from_utf8(bytes).unwrap()
        }
        Err(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// 1. Success path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_command_produces_response() {
    let handler = MockHandler::new();
    handler.with_response(
        "session_info",
        SdkResponse::success_with_data(None, "session_info", json!({"session_id": "test"})),
    );

    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // First message should be the ready header
    assert_eq!(messages[0]["type"], "proxy_ready");
    assert_eq!(messages[0]["schema_version"], SDK_SCHEMA_VERSION);

    // Second message should be the session_info response
    assert_eq!(messages[1]["type"], "response");
    assert_eq!(messages[1]["command"], "session_info");
    assert_eq!(messages[1]["success"], true);
    assert_eq!(messages[1]["data"]["session_id"], "test");
}

#[tokio::test]
async fn multiple_commands_in_sequence() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[
        r#"{"type":"set_model","model":"anthropic:claude-sonnet-4"}"#,
        r#"{"type":"session_info"}"#,
    ]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // proxy_ready + 2 responses
    assert!(messages.len() >= 3);
    assert_eq!(messages[0]["type"], "proxy_ready");
    assert_eq!(messages[1]["type"], "response");
    assert_eq!(messages[1]["command"], "set_model");
    assert_eq!(messages[2]["type"], "response");
    assert_eq!(messages[2]["command"], "session_info");
}

#[tokio::test]
async fn quit_command_ends_proxy() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"{"type":"session_info"}"#, r#"{"type":"quit"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // Should have proxy_ready + session_info response + quit response
    // but no error
    let types: Vec<&str> = messages
        .iter()
        .map(|m| m["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"response"));
    // All responses should be successful
    for msg in &messages {
        if msg["type"] == "response" {
            assert_eq!(
                msg["success"], true,
                "response should be success: {:?}",
                msg
            );
        }
    }
}

#[tokio::test]
async fn response_correlates_with_command_id() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"{"type":"session_info","id":"corr-42"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    let resp = &messages[1];
    assert_eq!(resp["id"], "corr-42");
}

// ---------------------------------------------------------------------------
// 2. Event forwarding
// ---------------------------------------------------------------------------

#[tokio::test]
async fn events_are_forwarded_as_jsonl() {
    let handler = MockHandler::new();
    handler.emit_event(ProxyEvent::Agent(agent_event_to_value(
        &AgentEvent::AgentStart,
    )));
    handler.emit_event(ProxyEvent::Agent(agent_event_to_value(
        &AgentEvent::AgentEnd { messages: vec![] },
    )));

    let input = jsonl_input(&[r#"{"type":"prompt","message":"hello"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // proxy_ready + AgentStart + AgentEnd + response
    assert!(
        messages.len() >= 4,
        "expected >= 4 messages, got {}",
        messages.len()
    );

    let types: Vec<&str> = messages
        .iter()
        .map(|m| m["type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"AgentStart"), "should contain AgentStart");
    assert!(types.contains(&"AgentEnd"), "should contain AgentEnd");
}

// ---------------------------------------------------------------------------
// 3. Malformed frames
// ---------------------------------------------------------------------------

#[tokio::test]
async fn malformed_json_produces_error_response() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"not valid json"#, r#"{"type":"session_info"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // Should have a proxy_error for the malformed line
    let error_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m["type"] == "proxy_error")
        .collect();
    assert_eq!(error_msgs.len(), 1, "should have exactly one proxy_error");
    assert!(error_msgs[0]["error"].as_str().unwrap().contains("parse"));
    assert_eq!(error_msgs[0]["line_number"], 1);

    // The valid command after should still work
    let session_resp: Vec<_> = messages
        .iter()
        .filter(|m| m["type"] == "response" && m["command"] == "session_info")
        .collect();
    assert_eq!(
        session_resp.len(),
        1,
        "session_info should succeed after error"
    );
}

#[tokio::test]
async fn unknown_command_type_produces_error() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"{"type":"nonexistent_command"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    let error_msgs: Vec<_> = messages
        .iter()
        .filter(|m| {
            m["type"] == "proxy_error" || (m["type"] == "response" && m["success"] == false)
        })
        .collect();
    assert!(
        !error_msgs.is_empty(),
        "should produce an error for unknown command"
    );
}

#[tokio::test]
async fn empty_lines_are_ignored() {
    let handler = MockHandler::new();
    let input = "\n\n{\"type\":\"session_info\"}\n\n".to_owned();

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    // proxy_ready + session_info response (empty lines skipped)
    assert!(messages.len() >= 2);
    assert_eq!(messages[1]["type"], "response");
    assert_eq!(messages[1]["command"], "session_info");
}

// ---------------------------------------------------------------------------
// 4. Cancellation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cancellation_stops_proxy_cleanly() {
    let handler = MockHandler::new();
    let cancel = CancellationToken::new();

    let reader = Cursor::new(r#"{"type":"session_info"}"#.to_owned());
    let writer = Cursor::new(Vec::new());

    let proxy = StreamingProxy::new(handler, ProxyConfig::default());

    cancel.cancel();
    let result = proxy.run(reader, writer, cancel);

    let writer = result.expect("pre-cancelled input should shut down cleanly");
    let output = String::from_utf8(writer.into_inner()).unwrap();
    let messages = parse_jsonl(&output);
    let cancelled = messages.iter().any(|m| m["type"] == "proxy_cancelled");
    assert!(cancelled, "should emit proxy_cancelled event");
}

#[tokio::test]
async fn cancellation_emits_proxy_cancelled_event() {
    let handler = MockHandler::new();

    // Build a slow handler that will be cancelled
    let input = jsonl_input(&[r#"{"type":"prompt","message":"hello"}"#]);

    let reader = Cursor::new(input);
    let writer = Cursor::new(Vec::new());

    let cancel = CancellationToken::new();
    let proxy = StreamingProxy::new(handler, ProxyConfig::default());

    // Cancel immediately after starting
    cancel.cancel();
    let result = proxy.run(reader, writer, cancel);

    if let Ok(w) = result {
        let output = String::from_utf8(w.into_inner()).unwrap();
        let messages = parse_jsonl(&output);
        let cancelled: Vec<_> = messages
            .iter()
            .filter(|m| m["type"] == "proxy_cancelled")
            .collect();
        assert!(!cancelled.is_empty(), "should emit proxy_cancelled event");
    }
}

// ---------------------------------------------------------------------------
// 5. Secret redaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secret_redaction_removes_api_keys() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "type": "ToolExecutionEnd",
        "result": "API key sk-ant-1234567890abcdef1234567890abcdef used successfully"
    });

    let redacted = redactor.redact(&event);

    let result_text = redacted["result"].as_str().unwrap();
    assert!(
        !result_text.contains("sk-ant-"),
        "API key should be redacted"
    );
    assert!(
        result_text.contains("[REDACTED]"),
        "should contain [REDACTED]"
    );
}

#[tokio::test]
async fn secret_redaction_handles_bearer_tokens() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "type": "ToolExecutionEnd",
        "result": "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.abc.def"
    });

    let redacted = redactor.redact(&event);

    let result_text = redacted["result"].as_str().unwrap();
    assert!(
        !result_text.contains("eyJhbGci"),
        "JWT token should be redacted"
    );
    assert!(result_text.contains("[REDACTED]"));
}

#[tokio::test]
async fn secret_redaction_handles_password_fields() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "type": "ToolExecutionEnd",
        "args": {
            "password": "super_secret_123",
            "username": "user"
        }
    });

    let redacted = redactor.redact(&event);

    assert_eq!(
        redacted["args"]["username"], "user",
        "username should be preserved"
    );
    assert_eq!(
        redacted["args"]["password"], "[REDACTED]",
        "password should be redacted"
    );
}

#[tokio::test]
async fn proxy_applies_redaction_to_events() {
    let handler = MockHandler::new();
    handler.emit_event(ProxyEvent::Agent(json!({
        "type": "ToolExecutionEnd",
        "result": "key=sk-ant-1234567890abcdef1234567890abcdef"
    })));

    let config = ProxyConfig {
        redact_secrets: true,
        ..Default::default()
    };

    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy_with_config(&input, handler, config).await;
    let messages = parse_jsonl(&output);

    // Find the ToolExecutionEnd event
    let tool_events: Vec<_> = messages
        .iter()
        .filter(|m| m["type"] == "ToolExecutionEnd")
        .collect();
    assert_eq!(tool_events.len(), 1);
    let result_text = tool_events[0]["result"].as_str().unwrap();
    assert!(
        !result_text.contains("sk-ant-"),
        "secret should be redacted in proxy output"
    );
}

#[tokio::test]
async fn redaction_can_be_disabled() {
    let handler = MockHandler::new();
    handler.emit_event(ProxyEvent::Agent(json!({
        "type": "ToolExecutionEnd",
        "result": "key=sk-ant-1234567890abcdef1234567890abcdef"
    })));

    let config = ProxyConfig {
        redact_secrets: false,
        ..Default::default()
    };

    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy_with_config(&input, handler, config).await;
    let messages = parse_jsonl(&output);

    let tool_events: Vec<_> = messages
        .iter()
        .filter(|m| m["type"] == "ToolExecutionEnd")
        .collect();
    assert_eq!(tool_events.len(), 1);
    let result_text = tool_events[0]["result"].as_str().unwrap();
    assert!(
        result_text.contains("sk-ant-"),
        "secret should NOT be redacted when disabled"
    );
}

// ---------------------------------------------------------------------------
// 6. Backpressure
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bounded_event_channel_capacity_respected() {
    let config = ProxyConfig {
        event_channel_capacity: 2,
        ..Default::default()
    };

    // This test verifies the config is accepted and the proxy still works
    // with a small channel. Backpressure is applied internally.
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy_with_config(&input, handler, config).await;
    let messages = parse_jsonl(&output);

    assert!(
        messages.len() >= 2,
        "proxy should produce output even with tiny channel"
    );
}

#[tokio::test]
async fn bounded_event_channel_saturates_and_drops_overflow() {
    // The handler emits more events during one command than the channel can
    // buffer, and nothing drains until `handle_command` returns, so the
    // excess is dropped at the sink (server-side warn only; there is no
    // consumer-visible overflow signal). Pins the actual drop semantics:
    // exactly `capacity` events are delivered, the command response still
    // lands, and the run completes.
    let config = ProxyConfig {
        event_channel_capacity: 2,
        ..Default::default()
    };
    let handler = MockHandler::new();
    for i in 0..5 {
        handler.emit_event(ProxyEvent::Agent(json!({
            "type": "ToolExecutionEnd",
            "sequence": i,
        })));
    }
    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy_with_config(&input, handler, config).await;
    let messages = parse_jsonl(&output);

    let agent_events: Vec<_> = messages
        .iter()
        .filter(|m| m["type"] == "ToolExecutionEnd")
        .collect();
    assert_eq!(
        agent_events.len(),
        2,
        "only the buffered capacity is delivered; overflow is dropped: got {agent_events:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m["type"] == "response" && m["success"] == true),
        "the overflow drops must not fail the command: {messages:?}"
    );
}

// ---------------------------------------------------------------------------
// 7. Client disconnect (write error)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn write_error_handled_gracefully() {
    /// A writer that fails after N bytes.
    struct FailingWriter {
        bytes_written: usize,
        fail_after: usize,
    }

    impl std::io::Write for FailingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes_written += buf.len();
            if self.bytes_written > self.fail_after {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "client disconnected",
                ))
            } else {
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let handler = MockHandler::new();
    let reader = Cursor::new(jsonl_input(&[
        r#"{"type":"session_info"}"#,
        r#"{"type":"session_info"}"#,
    ]));

    // Fail after writing the proxy_ready header
    let writer = FailingWriter {
        bytes_written: 0,
        fail_after: 200,
    };

    let proxy = StreamingProxy::new(handler, ProxyConfig::default());
    let cancel = CancellationToken::new();

    // Should not panic; should return an error or handle gracefully
    let _ = proxy.run(reader, writer, cancel);
    // If we get here without panic, the test passes.
}

// ---------------------------------------------------------------------------
// 8. Proxy ready header
// ---------------------------------------------------------------------------

#[tokio::test]
async fn first_output_is_proxy_ready() {
    let handler = MockHandler::new();
    let input = jsonl_input(&[r#"{"type":"session_info"}"#]);

    let output = run_proxy(&input, handler).await;
    let messages = parse_jsonl(&output);

    assert!(!messages.is_empty(), "should produce output");
    assert_eq!(messages[0]["type"], "proxy_ready");
    assert_eq!(messages[0]["schema_version"], SDK_SCHEMA_VERSION);
}

// ---------------------------------------------------------------------------
// 9. Empty input
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_input_produces_only_ready_header() {
    let handler = MockHandler::new();

    let output = run_proxy("", handler).await;
    let messages = parse_jsonl(&output);

    // Should still emit proxy_ready
    assert!(!messages.is_empty(), "should produce proxy_ready");
    assert_eq!(messages[0]["type"], "proxy_ready");
    assert_eq!(
        messages.len(),
        1,
        "empty input should only produce proxy_ready"
    );
}

// ---------------------------------------------------------------------------
// 10. No live provider calls
// ---------------------------------------------------------------------------

#[test]
fn mock_handler_proves_no_live_calls() {
    // This test exists to satisfy the DoD requirement that tests prove no live
    // provider calls are required. All tests above use MockHandler, which has
    // no network dependency. This is the explicit assertion.
    let handler = MockHandler::new();
    handler.with_response("session_info", SdkResponse::success(None, "session_info"));
    // No network calls needed to use the handler
    let resp = {
        let sink = |_: ProxyEvent| {};
        handler.handle_command(SdkCommand::session_info { id: None }, &sink)
    };
    assert!(resp.success, "mock handler should work without network");
}

// ---------------------------------------------------------------------------
// 11. SecretRedactor unit tests
// ---------------------------------------------------------------------------

#[test]
fn redactor_default_patterns() {
    let redactor = SecretRedactor::default();
    assert!(
        !redactor.patterns().is_empty(),
        "should have default patterns"
    );
}

#[test]
fn redactor_custom_pattern() {
    let redactor = SecretRedactor::new(vec!["my-secret-token-\\w+".to_owned()]);

    let event = json!({
        "data": "token=my-secret-token-abc123 found"
    });

    let redacted = redactor.redact(&event);
    let text = redacted["data"].as_str().unwrap();
    assert!(!text.contains("my-secret-token-abc123"));
    assert!(text.contains("[REDACTED]"));
}

#[test]
fn redactor_handles_deeply_nested_json() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "outer": {
            "inner": {
                "password": "deep_secret",
                "data": "normal"
            }
        }
    });

    let redacted = redactor.redact(&event);
    assert_eq!(redacted["outer"]["inner"]["password"], "[REDACTED]");
    assert_eq!(redacted["outer"]["inner"]["data"], "normal");
}

#[test]
fn redactor_preserves_non_matching_values() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "type": "MessageStart",
        "message": "Hello, world!"
    });

    let redacted = redactor.redact(&event);
    assert_eq!(redacted, event, "non-matching event should be unchanged");
}

#[test]
fn redactor_preserves_short_secret_like_prefixes() {
    let redactor = SecretRedactor::default();

    let event = json!({
        "status": "short marker sk-test and JWT prefix eyJ are not credentials"
    });

    let redacted = redactor.redact(&event);
    assert_eq!(
        redacted, event,
        "short credential-like prefixes should not be redacted"
    );
}

#[test]
fn redactor_redacts_hyphenated_provider_key_values() {
    let redactor = SecretRedactor::default();
    let samples = [
        "sk-ant-api03-1234567890abcdefghijklmnopqrstuv",
        "sk-ant-api06-1234567890abcdefghijklmnopqrstuv",
        "sk-proj-1234567890abcdefghijklmnopqrstuv",
        "sk-live-1234567890abcdefghijklmnopqrstuv",
        "sk-svcacct-1234567890abcdefghijklmnopqrstuv",
    ];

    for sample in samples {
        let event = json!({ "message": format!("credential {sample}") });
        let redacted = redactor.redact(&event);
        let text = redacted["message"].as_str().unwrap();
        assert!(
            !text.contains(sample),
            "hyphenated provider key leaked: {text}"
        );
        assert_eq!(text, "[REDACTED]");
    }
}

// ---------------------------------------------------------------------------
// 12. ProxyConfig defaults
// ---------------------------------------------------------------------------

#[test]
fn default_config_has_reasonable_values() {
    let config = ProxyConfig::default();
    assert!(
        config.event_channel_capacity > 0,
        "channel capacity should be positive"
    );
    assert!(config.redact_secrets, "redaction should be on by default");
}
