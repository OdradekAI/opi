//! Observe real Agent tool execution while the Chat protocol tail is held.
//!
//! This uses the real OpenAI Chat adapter and Models::prepare_call through
//! agent_loop. The only mocked component is a harmless, terminating tool.
//! No assertion selects either the current or proposed terminal timing.

mod common;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_agent::event::AgentEvent;
use opi_agent::hooks::{AgentHooks, ShouldStopAfterTurnContext};
use opi_agent::loop_types::{
    AgentError, AgentLoopConfig, AgentLoopContext, InferenceConfig, ModelSelection, NextTurnState,
};
use opi_agent::message::AgentMessage;
use opi_agent::tool::{ExecutionMode, Tool, ToolError, ToolResult, UpdateCallback};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, OutputContent, ToolDef, UserMessage,
};
use opi_ai::providers::openai::{CompatConfig, OpenAiChatProvider};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{
    AuthProvenanceSource, AuthScheme, CompatMetadata, ModelCapabilities, ModelInfo, Models,
    StaticAuthResolver, WireApi,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

const LIMIT: Duration = Duration::from_secs(10);
const HOLD: Duration = Duration::from_millis(200);

#[derive(Clone, Copy)]
enum Tail {
    Done,
    Error,
    Cancel,
}

impl Tail {
    fn name(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Error => "error",
            Self::Cancel => "cancel",
        }
    }

    fn body(self) -> &'static str {
        match self {
            Self::Done | Self::Cancel => "data: [DONE]\n\n",
            Self::Error => {
                "data: {\"error\":{\"message\":\"fixture late failure\",\"type\":\"server_error\",\"code\":\"fixture_late_error\"}}\n\n"
            }
        }
    }
}

struct Trace {
    phase: &'static str,
    calls: usize,
    rows: Vec<Value>,
}

impl Trace {
    fn push(&mut self, value: Value) {
        self.rows.push(json!({
            "seq": self.rows.len(),
            "phase": self.phase,
            "data": value,
        }));
    }
}

type SharedTrace = Arc<Mutex<Trace>>;

struct RecordTool {
    trace: SharedTrace,
}

impl Tool for RecordTool {
    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "record".into(),
            description: "Records fixture arguments without external effects".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"arg": {"type": "string"}},
                "required": ["arg"],
                "additionalProperties": false,
            }),
            constrained_sampling: None,
        }
    }

    fn execute(
        &self,
        call_id: &str,
        args: Value,
        _signal: CancellationToken,
        _on_update: Option<UpdateCallback>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send>> {
        let trace = self.trace.clone();
        let call_id = call_id.to_owned();
        Box::pin(async move {
            // Fixture integrity, not a terminal-timing expectation.
            assert_eq!(args, json!({"arg": "ok"}));
            {
                let mut trace = trace.lock().unwrap();
                trace.calls += 1;
                trace.push(json!({
                    "event": "tool_body",
                    "call_id": call_id,
                    "args": args,
                    "result": "recorded:ok",
                }));
            }
            Ok(ToolResult {
                content: vec![OutputContent::Text {
                    text: "recorded:ok".into(),
                }],
                details: None,
                is_error: false,
                terminate: true,
                truncated: false,
                diagnostics: vec![],
            })
        })
    }

    fn execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Sequential
    }
}

struct ProbeHooks;

impl AgentHooks for ProbeHooks {
    fn convert_to_llm(&self, messages: &[AgentMessage]) -> Result<Vec<Message>, AgentError> {
        Ok(messages
            .iter()
            .filter_map(|message| match message {
                AgentMessage::Llm(message) => Some(message.clone()),
                _ => None,
            })
            .collect())
    }

    fn should_stop_after_turn(
        &self,
        _context: ShouldStopAfterTurnContext,
    ) -> Pin<Box<dyn Future<Output = bool> + Send>> {
        // Defensive bound: even an unexpectedly tool-free successful result
        // cannot require a second loopback request.
        Box::pin(async { true })
    }
}

fn assistant_view(message: &AssistantMessage) -> Value {
    let text: String = message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let tools: Vec<Value> = message
        .content
        .iter()
        .filter_map(|content| match content {
            AssistantContent::ToolCall { tool_call } => Some(json!({
                "id": tool_call.id,
                "name": tool_call.name,
                "arguments": tool_call.arguments,
                "arguments_error": tool_call.arguments_error,
            })),
            _ => None,
        })
        .collect();
    json!({
        "stop": message.stop_reason,
        "raw_stop": message.raw_stop_reason,
        "text": text,
        "tools": tools,
        "error_message": message.error_message,
        "usage": {
            "reported": message.usage.reported,
            "input": message.usage.input_tokens,
            "output": message.usage.output_tokens,
            "total": message.usage.total_tokens,
        },
    })
}

fn record_event(trace: &SharedTrace, event: AgentEvent) {
    let row = match event {
        AgentEvent::AgentStart => json!({"event": "agent_start"}),
        AgentEvent::TurnStart => json!({"event": "turn_start"}),
        AgentEvent::MessageStart { .. } => json!({"event": "message_start"}),
        AgentEvent::MessageUpdate {
            assistant_event, ..
        } => match *assistant_event {
            AssistantStreamEvent::TextDelta { delta, .. } => {
                json!({"event": "text_delta", "delta": delta})
            }
            other => {
                let value = serde_json::to_value(other).unwrap();
                json!({"event": "message_update", "kind": value["type"]})
            }
        },
        AgentEvent::MessageEnd {
            message: AgentMessage::Llm(Message::Assistant(message)),
        } => json!({"event": "message_end", "message": assistant_view(&message)}),
        AgentEvent::ToolExecutionStart {
            tool_call_id,
            tool_name,
            args,
        } => json!({
            "event": "tool_start",
            "call_id": tool_call_id,
            "name": tool_name,
            "args": args,
        }),
        AgentEvent::ToolExecutionEnd {
            tool_call_id,
            tool_name,
            result,
            is_error,
            ..
        } => json!({
            "event": "tool_end",
            "call_id": tool_call_id,
            "name": tool_name,
            "result": result,
            "is_error": is_error,
        }),
        AgentEvent::TurnEnd { tool_results, .. } => {
            json!({"event": "turn_end", "tool_results": tool_results.len()})
        }
        AgentEvent::AgentEnd { messages } => {
            json!({"event": "agent_end", "messages": messages.len()})
        }
        _ => return,
    };
    trace.lock().unwrap().push(row);
}

fn context(base_url: String, trace: SharedTrace) -> AgentLoopContext {
    let provider = OpenAiChatProvider::new_for_profile(
        base_url,
        "probe".into(),
        CompatConfig::default(),
        vec![],
        vec![ModelInfo::new(
            "mock",
            "Mock",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(100_000, 4_096),
        )],
    );
    let mut models = Models::new();
    // single_route_collection uses ApiKey and cannot authenticate real Chat.
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                "fixture-key".into(),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    AgentLoopContext {
        collection: Arc::new(models),
        registry: common::test_registry(vec![Box::new(RecordTool { trace })]),
        authorizer: Some(common::permissive_authorizer()),
        evidence_health: opi_agent::evidence::EvidenceHealth::healthy(),
        state: NextTurnState::new(
            vec![AgentMessage::Llm(Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "record the fixture".into(),
                }],
                timestamp_ms: 0,
            }))],
            ModelSelection::parse_spec("probe:mock").unwrap(),
            InferenceConfig::default(),
        ),
        system: None,
        steering_queue: None,
        follow_up_queue: None,
        diagnostic_sink: None,
        session_id: None,
        evidence_sink: None,
    }
}

fn prefix() -> String {
    [
        json!({"id":"fixture","model":"mock","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}),
        json!({"id":"fixture","model":"mock","choices":[{"index":0,"delta":{"content":"before tool"},"finish_reason":null}]}),
        json!({"id":"fixture","model":"mock","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_record","type":"function","function":{"name":"record","arguments":"{\"arg\":\"ok\"}"}}]},"finish_reason":null}]}),
        json!({"id":"fixture","model":"mock","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}),
        json!({"id":"fixture","model":"mock","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":3,"total_tokens":10}}),
    ]
    .into_iter()
    .map(|event| format!("data: {event}\n\n"))
    .collect()
}

async fn read_request(socket: &mut TcpStream) -> Value {
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
            break index + 4;
        }
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0, "client ended before the HTTP headers");
        bytes.extend_from_slice(&buffer[..count]);
        assert!(
            bytes.len() < 256 * 1024,
            "fixture request exceeded its bound"
        );
    };
    let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
    assert!(headers.starts_with("POST "));
    let length = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find_map(|(name, value)| {
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .expect("the fixed JSON request carries Content-Length");
    assert!(length < 256 * 1024);
    while bytes.len() - header_end < length {
        let mut buffer = [0; 4096];
        let count = socket.read(&mut buffer).await.unwrap();
        assert_ne!(count, 0, "client ended before its JSON request");
        bytes.extend_from_slice(&buffer[..count]);
    }
    serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap()
}

async fn observe(tail: Tail) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}", listener.local_addr().unwrap());
    let (prefix_sent, prefix_ready) = oneshot::channel();
    let (release_tail, tail_released) = oneshot::channel();
    let mut peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_request(&mut socket).await;
        assert_eq!(request["model"], "mock");
        assert_eq!(request["tools"][0]["function"]["name"], "record");
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        socket.write_all(prefix().as_bytes()).await.unwrap();
        socket.flush().await.unwrap();
        let _ = prefix_sent.send(());
        // Deliberately retain this gate even if an early Done causes the
        // client to close. Socket EOF is not permission to release the tail.
        tail_released.await.expect("test releases the tail gate");
        let write = socket.write_all(tail.body().as_bytes()).await;
        let flushed = if write.is_ok() {
            socket.flush().await.is_ok()
        } else {
            false
        };
        let _ = socket.shutdown().await;
        // OS TCP-close timing can affect acceptance of a late write. Keep
        // that transport observation separate from the semantic event trace.
        json!({
            "tail_gate_released": true,
            "tail_write_accepted": write.is_ok() && flushed,
        })
    });
    let trace = Arc::new(Mutex::new(Trace {
        phase: "before_tail",
        calls: 0,
        rows: vec![],
    }));
    let context = context(base_url, trace.clone());
    let sink_trace = trace.clone();
    let cancel = CancellationToken::new();
    let run_cancel = cancel.clone();
    let mut run = tokio::spawn(async move {
        opi_agent::agent_loop(
            context,
            AgentLoopConfig::default(),
            &ProbeHooks,
            Box::new(move |event| record_event(&sink_trace, event)),
            run_cancel,
        )
        .await
    });
    if tokio::time::timeout(LIMIT, prefix_ready)
        .await
        .ok()
        .and_then(Result::ok)
        .is_none()
    {
        cancel.cancel();
        let _ = release_tail.send(());
        run.abort();
        peer.abort();
        let _ = run.await;
        let _ = peer.await;
        panic!("fixture prefix was not sent within the readiness bound");
    }
    tokio::time::sleep(HOLD).await;
    let before_tail = {
        let mut trace = trace.lock().unwrap();
        let snapshot = json!({
            "agent_finished": run.is_finished(),
            "tool_calls": trace.calls,
            "events": trace.rows.clone(),
        });
        trace.phase = "after_tail_release";
        snapshot
    };
    if matches!(tail, Tail::Cancel) {
        // This is exactly the token passed to agent_loop, not socket teardown.
        cancel.cancel();
    }
    release_tail
        .send(())
        .expect("peer still owns the tail gate");
    let outcome = match tokio::time::timeout(LIMIT, &mut run).await {
        Ok(joined) => joined.expect("agent task panicked"),
        Err(_) => {
            cancel.cancel();
            run.abort();
            peer.abort();
            let _ = run.await;
            let _ = peer.await;
            panic!("agent did not settle after tail release");
        }
    };
    let peer_observation = match tokio::time::timeout(LIMIT, &mut peer).await {
        Ok(joined) => joined.expect("loopback peer panicked"),
        Err(_) => {
            peer.abort();
            let _ = peer.await;
            panic!("loopback peer did not settle after tail release");
        }
    };
    let final_error = match outcome.error() {
        None => Value::Null,
        Some(AgentError::Cancelled) => json!({"kind":"cancelled"}),
        Some(AgentError::Provider(failure)) => json!({
            "kind": "provider",
            "code": failure.code(),
            "category": format!("{:?}", failure.category()),
        }),
        Some(error) => json!({"kind":"other", "diagnostic":error.to_string()}),
    };
    let assistants: Vec<Value> = outcome
        .state()
        .context
        .iter()
        .filter_map(|message| match message {
            AgentMessage::Llm(Message::Assistant(message)) => Some(assistant_view(message)),
            _ => None,
        })
        .collect();
    let trace = trace.lock().unwrap();
    json!({
        "probe": "agent_chat_finish_timing",
        "mode": "observe",
        "case": tail.name(),
        "held_ms": HOLD.as_millis(),
        "before_tail": before_tail,
        "final": {
            "ok": outcome.error().is_none(),
            "error": final_error,
            "tool_calls": trace.calls,
            "assistants": assistants,
            "events": trace.rows,
        },
        "peer": peer_observation,
    })
}

#[tokio::test]
async fn observe_agent_chat_finish_timing() {
    // One test keeps the three JSON records in deterministic case order.
    // No elapsed wall times, random ports, or generated IDs enter the trace.
    for tail in [Tail::Done, Tail::Error, Tail::Cancel] {
        let row = observe(tail).await;
        println!("agent_chat_finish_timing {row}");
        assert_eq!(row["before_tail"]["agent_finished"], false);
        assert_eq!(row["before_tail"]["tool_calls"], 0);
        assert!(
            !row["before_tail"]["events"]
                .as_array()
                .unwrap()
                .iter()
                .any(|event| event["data"]["event"] == "message_end")
        );
        assert_eq!(
            row["final"]["tool_calls"],
            if matches!(tail, Tail::Done) { 1 } else { 0 }
        );
        assert_eq!(row["final"]["ok"], matches!(tail, Tail::Done));
    }
}
