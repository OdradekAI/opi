//! Gated Chat finish, usage, sentinel and HTTP EOF observations on a real peer.
use futures_util::StreamExt;
use opi_ai::message::{
    AssistantMessage, InputContent, Message, ToolDef, UserMessage, parse_streaming_json,
};
use opi_ai::models::CompatMetadata;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::{Provider, Request};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, StaticAuthResolver};
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
const FIXTURE: &str = include_str!("data/openai-chat-finish-timing.json");
fn request(model: &str, messages: Vec<Message>) -> Request {
    Request {
        model: model.into(),
        context: normalize_context(Context {
            messages,
            tools: vec![ToolDef {
                name: "record".into(),
                description: "Records fixture arguments".into(),
                input_schema: json!({
                    "type": "object", "properties": {"a": {"type": "string"}, "n": {"type": "integer"}},
                    "required": ["a", "n"],
                }),
                constrained_sampling: None,
            }],
            ..Default::default()
        })
        .unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

fn user() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "fixture".into(),
        }],
        timestamp_ms: 1,
    })
}

// Keep exact deltas and event indexes. Use a neutral representation for the
// message fields so the Pi Harness can consume these snapshots as a control.
fn message_view(message: &AssistantMessage) -> Value {
    let content = serde_json::to_value(&message.content).unwrap();
    let content: Vec<Value> = content
        .as_array()
        .unwrap()
        .iter()
        .map(|block| match block["type"].as_str() {
            Some("text") => json!({"type": "text", "text": block["text"]}),
            Some("thinking") => json!({"type":"thinking", "thinking":block["thinking"]}),
            Some("tool_call") => {
                let call = &block["tool_call"];
                let raw = call["arguments"].as_str().expect("raw arguments");
                json!({
                    "type": "toolCall", "id": call["id"], "name": call["name"],
                    "arguments": parse_streaming_json(raw), "raw": raw,
                })
            }
            _ => panic!("unexpected fixture block: {block}"),
        })
        .collect();
    json!({
        "content": content, "stop": message.stop_reason,
        "raw_stop": message.raw_stop_reason, "response_id": message.response_id,
        "input_tokens": message.usage.input_tokens, "output_tokens": message.usage.output_tokens,
    })
}

fn event_view(event: &AssistantStreamEvent) -> Value {
    use AssistantStreamEvent::*;
    let (kind, index, delta, content, message) = match event {
        Start { partial } => ("start", None, None, None, partial),
        TextStart {
            content_index,
            partial,
        } => ("text_start", Some(*content_index), None, None, partial),
        TextDelta {
            content_index,
            delta,
            partial,
        } => (
            "text_delta",
            Some(*content_index),
            Some(delta),
            None,
            partial,
        ),
        TextEnd {
            content_index,
            content,
            partial,
        } => (
            "text_end",
            Some(*content_index),
            None,
            Some(content),
            partial,
        ),
        ThinkingStart {
            content_index,
            partial,
        } => ("thinking_start", Some(*content_index), None, None, partial),
        ThinkingDelta {
            content_index,
            delta,
            partial,
        } => (
            "thinking_delta",
            Some(*content_index),
            Some(delta),
            None,
            partial,
        ),
        ThinkingEnd {
            content_index,
            content,
            partial,
        } => (
            "thinking_end",
            Some(*content_index),
            None,
            Some(content),
            partial,
        ),
        ToolCallStart {
            content_index,
            partial,
        } => ("toolcall_start", Some(*content_index), None, None, partial),
        ToolCallDelta {
            content_index,
            delta,
            partial,
        } => (
            "toolcall_delta",
            Some(*content_index),
            Some(delta),
            None,
            partial,
        ),
        ToolCallEnd {
            content_index,
            partial,
            ..
        } => ("toolcall_end", Some(*content_index), None, None, partial),
        Done { message, .. } => ("done", None, None, None, message),
        other => panic!("unexpected fixture event: {other:?}"),
    };
    json!({"kind": kind, "index": index, "delta": delta, "content": content, "message": message_view(message)})
}

#[derive(Default)]
struct Observation {
    events: Vec<Value>,
    result: Option<Value>,
    eof: bool,
}

fn snapshot(state: &Mutex<Observation>) -> Value {
    let state = state.lock().unwrap();
    json!({"events": state.events, "result": state.result, "eof": state.eof})
}

async fn send(tx: &mpsc::Sender<(String, oneshot::Sender<()>)>, body: String) {
    let (ack, received) = oneshot::channel();
    tx.send((body, ack)).await.unwrap();
    received.await.unwrap();
}

fn frame(value: &Value) -> String {
    format!("data: {value}\n\n")
}

async fn observe(fixture: &Value, case: &Value, route: &str) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, mut rx) = mpsc::channel::<(String, oneshot::Sender<()>)>(1);
    let peer_closed = Arc::new(AtomicBool::new(false));
    let closed = peer_closed.clone();
    let azure = route == "azure";
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let end = loop {
            let byte = socket.read_u8().await.unwrap();
            request.push(byte);
            if request.ends_with(b"\r\n\r\n") {
                break request.len();
            }
        };
        let headers = String::from_utf8(request.clone()).unwrap();
        assert!(headers.starts_with(if azure {
            "POST /openai/deployments/gpt-4o/chat/completions?api-version="
        } else {
            "POST /v1/chat/completions HTTP/1.1"
        }));
        assert!(headers.to_lowercase().contains(if azure {
            "api-key: fixture-key"
        } else {
            "authorization: bearer fixture-key"
        }));
        let size: usize = headers
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length: ")
                    .map(|n| n.parse().unwrap())
            })
            .unwrap();
        request.resize(end + size, 0);
        socket.read_exact(&mut request[end..]).await.unwrap();
        let body: Value = serde_json::from_slice(&request[end..]).unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await.unwrap();
        loop {
            let next = tokio::select! {
                next = rx.recv() => next,
                read = socket.read_u8(), if !closed.load(Ordering::SeqCst) => {
                    assert!(read.is_err(), "unexpected bytes after the request");
                    closed.store(true, Ordering::SeqCst);
                    continue;
                }
            };
            let Some((data, ack)) = next else {
                break;
            };
            let chunk = if data == "__broken_chunk__" {
                "not-a-chunk\r\n".into()
            } else {
                format!("{:x}\r\n{}\r\n", data.len(), data)
            };
            // The SDK may already have released its response after a terminal.
            let _ = socket.write_all(chunk.as_bytes()).await;
            let _ = ack.send(());
        }
        let _ = socket.write_all(b"0\r\n\r\n").await;
        body
    });
    let provider: Box<dyn Provider> = match route {
        "openai" => Box::new(OpenAiChatProvider::new_with_compat(
            Some(base),
            opi_ai::openai_chat::CompatConfig {
                strict_tool_schema: case["arguments"] == "strict",
                supports_grammar_tools: (case["arguments"] == "grammar").then_some(true),
                supports_finish_reason: case["supports_finish_reason"].as_bool(),
                ..Default::default()
            },
        )),
        "openrouter" => Box::new(opi_ai::openrouter::openrouter_provider(Some(base))),
        "mistral" => Box::new(opi_ai::mistral::mistral_provider(Some(base))),
        "azure" => Box::new(
            opi_ai::azure_openai::AzureOpenAIProvider::new(Some(base), "gpt-4o".into(), None)
                .unwrap()
                .with_compat(opi_ai::openai_chat::CompatConfig {
                    usage_in_stream: case["supports_finish_reason"].as_bool().map(|_| false),
                    supports_finish_reason: case["supports_finish_reason"].as_bool(),
                    ..Default::default()
                }),
        ),
        _ => unreachable!(),
    };
    let model = provider.models()[0].clone();
    let model_id = model.id.clone();
    let mut models = Models::new();
    models
        .register_route(
            provider,
            Arc::new(StaticAuthResolver::new(
                if azure {
                    AuthScheme::ApiKey
                } else {
                    AuthScheme::Bearer
                },
                SecretString::from("fixture-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let mut selected_model = if route == "openai" {
        models.get_model("openai", "gpt-4o").unwrap()
    } else {
        model
    };
    if let opi_ai::WireCompat::OpenAiCompletions(compat) = &mut selected_model.compat {
        if let Some(supports) = case["supports_finish_reason"].as_bool() {
            compat.supports_finish_reason = Some(supports);
        }
        if case["arguments"] == "strict" {
            compat.strict_tool_schema = true;
        }
        if case["arguments"] == "grammar" {
            compat.supports_grammar_tools = Some(true);
        }
    }
    let record = ChatModelRecord::new(route, selected_model);
    let mut req = request(
        &format!(
            "{route}:{}",
            if route == "openai" {
                "gpt-4o"
            } else {
                &model_id
            }
        ),
        vec![user()],
    );
    if let Some(arguments) = case["arguments"].as_str() {
        use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
        let tool = ToolDef {
            name: "record".into(),
            description: "Records fixture arguments".into(),
            input_schema: if arguments == "grammar" {
                json!({"type":"object","properties":{"input":{"type":"string"}},"required":["input"]})
            } else {
                json!({"type":"object","properties":{"a":{"type":"string"},"optional":{"type":"integer"}},"required":["a"]})
            },
            constrained_sampling: match arguments {
                "strict" => Some(ConstrainedSampling::JsonSchema {
                    strict: StrictPreference::Require,
                }),
                "grammar" => Some(ConstrainedSampling::Grammar {
                    openai_lark: Some("start: /.+/".into()),
                    openai_regex: None,
                }),
                _ => None,
            },
        };
        req.context = normalize_context(Context {
            messages: vec![user()],
            tools: vec![tool],
            ..Default::default()
        })
        .unwrap();
    }
    let recorder = opi_telemetry::memory::Recorder::default();
    req.telemetry_context = Some(recorder.context());
    let raw_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen_raw = raw_count.clone();
    let cancel = req.cancel.clone();
    let (raw_tx, mut raw_rx) = mpsc::channel(1);
    req.callbacks = req.callbacks.with_async_on_raw(move |raw| {
        let tx = raw_tx.clone();
        let seen_raw = seen_raw.clone();
        async move {
            seen_raw.fetch_add(1, Ordering::SeqCst);
            if raw["callback_failure"] == true {
                return Err(opi_ai::callbacks::CallbackError::new(
                    "fixture observer failure",
                ));
            }
            if let Some(name) = raw["checkpoint"].as_str() {
                let (release, wait) = oneshot::channel();
                tx.send((name.to_owned(), release)).await.unwrap();
                let _ = wait.await;
            }
            Ok(())
        }
    });
    let mut call = models.stream_simple(record, req, Default::default());
    let state = Arc::new(Mutex::new(Observation::default()));
    let observed = state.clone();
    let collect = tokio::spawn(async move {
        while let Some(event) = call.next().await {
            let view = match event {
                Ok(event) => event_view(&event),
                Err(error) => json!({"kind":"error", "message":message_view(error.message())}),
            };
            observed.lock().unwrap().events.push(view);
        }
        let result = match call.result().await {
            Ok(message) => message_view(&message),
            Err(error) => message_view(error.message()),
        };
        let mut observed = observed.lock().unwrap();
        observed.result = Some(result);
        observed.eof = true;
    });
    let mut prefix: String = fixture["prefix"]
        .as_array()
        .unwrap()
        .iter()
        .map(frame)
        .collect();
    let mut finish = fixture["finish"].clone();
    if case["usage"] == "inline" {
        finish["usage"] = fixture["usage"]["usage"].clone();
    }
    prefix.push_str(&frame(&finish));
    send(&tx, prefix).await;
    let mut checkpoints = serde_json::Map::new();
    for name in ["after_finish", "after_usage"] {
        if name == "after_usage" && case["usage"] == "trailer" {
            send(&tx, frame(&fixture["usage"])).await;
        }
        send(&tx, frame(&json!({"choices":[],"checkpoint":name}))).await;
        let gate = tokio::time::timeout(Duration::from_millis(200), raw_rx.recv())
            .await
            .ok()
            .flatten();
        // The raw gate proves the preceding frame was mapped. Let the public
        // execution drain its already queued events before taking a snapshot.
        tokio::time::sleep(Duration::from_millis(20)).await;
        if let Some((seen, release)) = gate {
            assert_eq!(seen, name);
            checkpoints.insert(name.into(), snapshot(&state));
            let _ = release.send(());
        } else {
            assert!(
                state.lock().unwrap().eof,
                "missing raw gate without settlement: {name}"
            );
            checkpoints.insert(name.into(), snapshot(&state));
        }
        checkpoints.get_mut(name).unwrap()["peer_closed"] =
            json!(peer_closed.load(Ordering::SeqCst));
    }
    match case["tail"].as_str().unwrap() {
        "done" => send(&tx, "data: [DONE]\n\n".into()).await,
        "error" => send(&tx, frame(&fixture["error"])).await,
        "fragmented_done" => {
            for piece in ["data: [DO", "NE]\r", "\n\r", "\n"] {
                send(&tx, piece.into()).await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
        "cancel" => cancel.cancel(),
        "transport_error" => send(&tx, "__broken_chunk__".into()).await,
        "malformed" => send(&tx, "data: {broken\n\n".into()).await,
        "invalid_usage" => send(&tx, frame(&json!({"choices":[],"usage":{"prompt_tokens":1,"completion_tokens":1,"completion_tokens_details":{"reasoning_tokens":2}}}))).await,
        "callback" => send(&tx, frame(&json!({"choices":[],"callback_failure":true}))).await,
        "done_then_error" => send(&tx, format!("data: [DONE]\n\n{}", frame(&fixture["error"]))).await,
        "eof" => {}
        other => panic!("unknown tail: {other}"),
    }
    // A bounded negative observation while the peer deliberately withholds EOF.
    tokio::time::sleep(Duration::from_millis(200)).await;
    checkpoints.insert("held_http_eof".into(), snapshot(&state));
    checkpoints.get_mut("held_http_eof").unwrap()["peer_closed"] =
        json!(peer_closed.load(Ordering::SeqCst));
    let raw_at_terminal = raw_count.load(Ordering::SeqCst);
    drop(tx);
    let request = peer.await.unwrap();
    collect.await.unwrap();
    let failed = snapshot(&state)["result"]["stop"] != "tool_use";
    let spans = recorder.spans();
    assert_eq!(spans.len(), 1, "one request settlement: {route}/{case}");
    assert!(spans[0].end_sequence.is_some(), "request span must finish");
    let final_stop = snapshot(&state)["result"]["stop"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        spans[0].attributes.get("stop"),
        Some(&opi_telemetry::AttributeValue::String(
            if final_stop == "tool_use" {
                "toolUse".into()
            } else {
                final_stop
            }
        ))
    );
    assert_eq!(
        recorder
            .measurements()
            .iter()
            .filter(|m| m.definition.name == "pi.ai.request.duration")
            .count(),
        1
    );
    let blocks = spans[0]
        .events
        .iter()
        .filter(|event| event.name == opi_ai::telemetry::AI_BLOCK)
        .count();
    assert_eq!(
        blocks,
        if failed {
            0
        } else {
            snapshot(&state)["result"]["content"]
                .as_array()
                .unwrap()
                .len()
        }
    );
    if case["tail"] == "done_then_error" {
        assert_eq!(
            raw_at_terminal,
            fixture["prefix"].as_array().unwrap().len() + 4,
            "sentinel and trailing error must not reach the raw callback"
        );
    }
    json!({"case":case["id"], "request":request, "checkpoints":checkpoints, "final":snapshot(&state)})
}

#[tokio::test]
async fn observe_chat_finish_timing() {
    let trace = tokio::time::timeout(Duration::from_secs(20), async {
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let mut cases = Vec::new();
        for case in fixture["cases"].as_array().unwrap() {
            cases.push(observe(&fixture, case, "openai").await);
        }
        cases
    })
    .await
    .expect("finish timing probe");
    println!("\n{}", json!(trace));
    for row in trace {
        for name in ["after_finish", "after_usage"] {
            let checkpoint = &row["checkpoints"][name];
            assert_eq!(
                checkpoint["events"].as_array().unwrap().len(),
                5,
                "blocks must remain open until the protocol terminal: {}/{name}",
                row["case"]
            );
            assert_eq!(checkpoint["result"], Value::Null);
            assert_eq!(checkpoint["eof"], false);
            assert_eq!(checkpoint["peer_closed"], false);
        }
        let held = &row["checkpoints"]["held_http_eof"];
        let settled = row["case"] != "eof_without_done";
        assert_eq!(held["eof"], settled, "terminal must not wait for HTTP EOF");
        assert_eq!(
            held["peer_closed"], settled,
            "terminal must release its response"
        );
        let name = row["case"].as_str().unwrap();
        let stop = if name.starts_with("error") {
            "error"
        } else if name.starts_with("cancel") {
            "aborted"
        } else {
            "tool_use"
        };
        assert_eq!(row["final"]["result"]["stop"], stop);
    }
}

#[tokio::test]
async fn shared_chat_routes_keep_tail_failures_and_release_at_sentinel() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        // Tool before text, then late reasoning: terminal order follows content.
        fixture["prefix"].as_array_mut().unwrap().swap(1, 2);
        fixture["prefix"]
            .as_array_mut()
            .unwrap()
            .push(json!({"choices":[{"index":0,"delta":{"reasoning_content":"inspect"}}]}));
        for route in ["openai", "openrouter", "mistral", "azure"] {
            for tail in [
                "done",
                "done_then_error",
                "fragmented_done",
                "eof",
                "error",
                "cancel",
                "malformed",
                "invalid_usage",
                "callback",
                "transport_error",
            ] {
                let case = json!({"id":tail,"usage":"trailer","tail":tail});
                let row = observe(&fixture, &case, route).await;
                for name in ["after_finish", "after_usage"] {
                    let checkpoint = &row["checkpoints"][name];
                    assert_eq!(checkpoint["result"], Value::Null, "{route}/{tail}/{name}");
                    assert!(
                        !checkpoint["events"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|e| e["kind"].as_str().unwrap().ends_with("_end"))
                    );
                }
                let success =
                    matches!(tail, "done" | "done_then_error" | "fragmented_done" | "eof");
                assert_eq!(
                    row["checkpoints"]["held_http_eof"]["eof"],
                    tail != "eof",
                    "{route}/{tail}"
                );
                assert_eq!(
                    row["checkpoints"]["held_http_eof"]["peer_closed"],
                    tail != "eof"
                );
                assert_eq!(
                    row["final"]["result"]["stop"],
                    if success {
                        "tool_use"
                    } else if tail == "cancel" {
                        "aborted"
                    } else {
                        "error"
                    }
                );
                let ends: Vec<_> = row["final"]["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e["kind"].as_str().unwrap().ends_with("_end"))
                    .map(|e| e["kind"].as_str().unwrap())
                    .collect();
                assert_eq!(
                    ends,
                    if success {
                        vec!["toolcall_end", "text_end", "thinking_end"]
                    } else {
                        vec![]
                    }
                );
            }
        }
    })
    .await
    .expect("shared Chat routes settle");
}

#[tokio::test]
async fn chat_arguments_finalize_only_at_successful_terminal() {
    tokio::time::timeout(Duration::from_secs(20), async {
        for arguments in ["strict", "grammar", "incomplete"] {
            for tail in ["done", "error", "cancel"] {
                let mut fixture: Value = serde_json::from_str(FIXTURE).unwrap();
                let call = &mut fixture["prefix"][2]["choices"][0]["delta"]["tool_calls"][0];
                if arguments == "grammar" {
                    call.as_object_mut().unwrap().remove("function");
                    call["type"] = json!("custom");
                    call["custom"] = json!({"name":"record","input":"hello"});
                } else {
                    call["function"]["arguments"] = json!(if arguments == "strict" {
                        r#"{"a":"ok","optional":null}"#
                    } else {
                        r#"{"a":"ok""#
                    });
                }
                let case =
                    json!({"id":arguments,"arguments":arguments,"usage":"trailer","tail":tail});
                let row = observe(&fixture, &case, "openai").await;
                assert_eq!(
                    row["checkpoints"]["after_usage"]["events"]
                        .as_array()
                        .unwrap()
                        .len(),
                    5
                );
                let raw = row["final"]["result"]["content"][1]["raw"]
                    .as_str()
                    .unwrap();
                match (arguments, tail) {
                    ("strict", "done") => assert_eq!(
                        serde_json::from_str::<Value>(raw).unwrap(),
                        json!({"a":"ok"})
                    ),
                    ("strict", _) => assert_eq!(raw, r#"{"a":"ok","optional":null}"#),
                    ("grammar", "done") => assert_eq!(
                        serde_json::from_str::<Value>(raw).unwrap(),
                        json!({"input":"hello"})
                    ),
                    ("grammar", _) => assert!(
                        serde_json::from_str::<Value>(raw).is_err(),
                        "failed grammar partial must not be closed"
                    ),
                    ("incomplete", _) => assert_eq!(raw, r#"{"a":"ok""#),
                    _ => unreachable!(),
                }
                assert_eq!(
                    row["final"]["result"]["stop"],
                    if tail == "done" {
                        "tool_use"
                    } else if tail == "cancel" {
                        "aborted"
                    } else {
                        "error"
                    }
                );
            }
        }
    })
    .await
    .expect("argument terminal cases");
}

#[tokio::test]
async fn chat_finishless_sentinel_and_eof_obey_route_compatibility() {
    tokio::time::timeout(Duration::from_secs(20), async {
        for route in ["openai", "azure"] {
            for supports in [true, false] {
                for tail in ["done", "eof"] {
                    let mut fixture: Value = serde_json::from_str(FIXTURE).unwrap();
                    fixture["finish"]["choices"][0]["finish_reason"] = Value::Null;
                    let case = json!({"id":"finishless","supports_finish_reason":supports,"usage":"trailer","tail":tail});
                    let row = observe(&fixture, &case, route).await;
                    assert_eq!(row["final"]["result"]["stop"], if supports { "error" } else { "tool_use" }, "{route}/{supports}/{tail}");
                    assert_eq!(row["final"]["result"]["raw_stop"], Value::Null);
                    if route == "azure" { assert!(row["request"].get("stream_options").is_none()); }
                    assert_eq!(row["checkpoints"]["held_http_eof"]["eof"], tail == "done");
                    assert_eq!(row["checkpoints"]["held_http_eof"]["peer_closed"], tail == "done");
                }
            }
        }
    }).await.expect("finishless compatibility cases");
}

#[tokio::test]
async fn fixture_entries_stop_on_sentinel_and_never_succeed_after_failure() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut prefix: String = fixture["prefix"]
        .as_array()
        .unwrap()
        .iter()
        .map(frame)
        .collect();
    prefix.push_str(&frame(&fixture["finish"]));
    prefix.push_str(&frame(&fixture["usage"]));
    for azure in [false, true] {
        for (tail, success) in [
            (
                format!("data: [DONE]\n\n{}", frame(&fixture["error"])),
                true,
            ),
            (
                format!("{}data: [DONE]\n\n", frame(&fixture["error"])),
                false,
            ),
            ("data: {malformed\n\ndata: [DONE]\n\n".into(), false),
            ("".into(), true),
            // The standalone parser accepts a final unterminated frame.
            ("data: {malformed".into(), false),
        ] {
            let body = format!("{prefix}{tail}");
            let stream = if azure {
                opi_ai::azure_openai::AzureOpenAIProvider::new(
                    Some("https://fixture.invalid".into()),
                    "fixture".into(),
                    None,
                )
                .unwrap()
                .stream_from_sse(&body, Default::default())
            } else {
                OpenAiChatProvider::new(None).stream_from_sse(&body, Default::default())
            };
            let events = stream.collect::<Vec<_>>().await;
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.as_ref().is_ok_and(|e| e.is_terminal()))
                    .count(),
                usize::from(success)
            );
            assert_eq!(
                events.iter().filter(|e| e.is_err()).count(),
                usize::from(!success)
            );
            if !success {
                assert!(!events.iter().any(|e| matches!(
                    e,
                    Ok(AssistantStreamEvent::TextEnd { .. }
                        | AssistantStreamEvent::ToolCallEnd { .. })
                )));
            }
        }
    }
}
