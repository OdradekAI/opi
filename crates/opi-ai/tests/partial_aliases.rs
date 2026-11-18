//! Retain public stream events across later chunks and compare the same reads
//! with the Pi partial-alias probe.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{FutureExt, StreamExt};
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, Message, ToolCall, UserMessage,
    parse_streaming_json,
};
use opi_ai::models::{ChatCall, CompatMetadata};
use opi_ai::provider::Request;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
use opi_ai::test_support::{MockProvider, base_assistant, single_route_collection};
use opi_ai::{AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, StaticAuthResolver};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SSE: &str = include_str!("data/partial-aliases.sse");
const TEXT: &str = "ABCD😀EF";
const ARGS: [&str; 5] = ["{\"a\"", ":\"BC", "DEF\"", ",\"n\"", ":2}"];

fn start_call(models: &Models, provider: &str, model: &str, mut request: Request) -> ChatCall {
    request.model = format!("{provider}:{model}");
    let info = models.get_model(provider, model).expect("registered model");
    let record = ChatModelRecord::new(provider, info);
    models.stream_simple(record, request, Default::default())
}

fn request() -> Request {
    let context = opi_ai::transcript::Context {
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "fixture".into(),
            }],
            timestamp_ms: 1,
        })],
        ..Default::default()
    };
    Request {
        model: String::new(),
        context: opi_ai::transcript::normalize_context(context).unwrap(),
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

fn message_view(message: &AssistantMessage) -> Value {
    json!({
        "content": message.content.iter().map(block_view).collect::<Vec<_>>(),
        "stop": message.stop_reason, "response_id": message.response_id,
        "input_tokens": message.usage.input_tokens,
    })
}

fn block_view(block: &AssistantContent) -> Value {
    match block {
        AssistantContent::Text { text, .. } => json!({"kind": "text", "text": text}),
        AssistantContent::ToolCall { tool_call } => json!({
            "kind": "tool", "id": tool_call.id, "name": tool_call.name,
            "args": args_view(tool_call),
        }),
        other => panic!("unexpected fixture block: {other:?}"),
    }
}

fn args_view(call: &ToolCall) -> Value {
    json!({"raw": call.arguments, "value": parse_streaming_json(&call.arguments)})
}

fn event_fields(event: &AssistantStreamEvent) -> (&str, Option<usize>, Option<&str>, Option<&str>) {
    match event {
        AssistantStreamEvent::Start { .. } => ("start", None, None, None),
        AssistantStreamEvent::TextStart { content_index, .. } => {
            ("text_start", Some(*content_index), None, None)
        }
        AssistantStreamEvent::TextDelta {
            content_index,
            delta,
            ..
        } => ("text_delta", Some(*content_index), Some(delta), None),
        AssistantStreamEvent::TextEnd {
            content_index,
            content,
            ..
        } => ("text_end", Some(*content_index), None, Some(content)),
        AssistantStreamEvent::ToolCallStart { content_index, .. } => {
            ("tool_call_start", Some(*content_index), None, None)
        }
        AssistantStreamEvent::ToolCallDelta {
            content_index,
            delta,
            ..
        } => ("tool_call_delta", Some(*content_index), Some(delta), None),
        AssistantStreamEvent::ToolCallEnd { content_index, .. } => {
            ("tool_call_end", Some(*content_index), None, None)
        }
        AssistantStreamEvent::Done { .. } => ("done", None, None, None),
        other => panic!("unexpected fixture event: {other:?}"),
    }
}

fn message_mut(event: &mut AssistantStreamEvent) -> &mut AssistantMessage {
    match event {
        AssistantStreamEvent::Start { partial }
        | AssistantStreamEvent::TextStart { partial, .. }
        | AssistantStreamEvent::TextDelta { partial, .. }
        | AssistantStreamEvent::TextEnd { partial, .. }
        | AssistantStreamEvent::ToolCallStart { partial, .. }
        | AssistantStreamEvent::ToolCallDelta { partial, .. }
        | AssistantStreamEvent::ToolCallEnd { partial, .. } => partial,
        AssistantStreamEvent::Done { message, .. } => message,
        other => panic!("unexpected fixture event: {other:?}"),
    }
}

fn tool(block: &mut AssistantContent) -> &mut ToolCall {
    let AssistantContent::ToolCall { tool_call } = block else {
        panic!("tool block expected")
    };
    tool_call
}

#[derive(Default)]
struct Observation {
    held: Vec<AssistantStreamEvent>,
    events: Vec<Value>,
    write: Option<Value>,
    tool_delta: usize,
}

impl Observation {
    async fn read(&mut self, call: &mut ChatCall, types: &[&str], mutate: bool) {
        for expected in types {
            let mut event = call.next().await.expect("event").expect("successful event");
            let (kind, index, delta, content) = event_fields(&event);
            assert_eq!(&kind, expected);
            let second_tool_delta = kind == "tool_call_delta" && delta == Some(ARGS[1]);
            let mut entry =
                json!({"kind": kind, "index": index, "delta": delta, "content": content});
            entry["message"] = message_view(message_mut(&mut event));
            self.events.push(entry);
            self.held.push(event);
            if second_tool_delta {
                self.tool_delta = self.held.len() - 1;
                if mutate {
                    let before = self.retained();
                    message_mut(&mut self.held[0]).response_id = Some("consumer-id".into());
                    let AssistantContent::Text { text, .. } =
                        &mut message_mut(&mut self.held[1]).content[0]
                    else {
                        panic!("text block expected")
                    };
                    text.push_str("|consumer|");
                    let partial = message_mut(&mut self.held[self.tool_delta]);
                    tool(&mut partial.content[1]).arguments =
                        r#"{"a":"BC","consumer":true}"#.into();
                    self.write = Some(json!({"before": before, "after": self.retained()}));
                }
            }
        }
    }

    fn retained(&mut self) -> Value {
        let latest = self.held.len() - 1 - usize::from(self.held.last().unwrap().is_terminal());
        json!({
            "start": message_view(message_mut(&mut self.held[0])),
            "text_start": message_view(message_mut(&mut self.held[1])),
            "first_text": message_view(message_mut(&mut self.held[2])),
            "saved_text": block_view(&message_mut(&mut self.held[1]).content[0]),
            "tool_delta": message_view(message_mut(&mut self.held[self.tool_delta])),
            "saved_args": args_view(tool(&mut message_mut(&mut self.held[self.tool_delta]).content[1])),
            "latest": message_view(message_mut(&mut self.held[latest])),
        })
    }

    async fn finish(mut self, mut call: ChatCall, after_usage: Option<Value>) -> Value {
        assert!(call.next().await.is_none());
        let result = call.result().await.expect("successful result");
        assert_eq!(message_mut(self.held.last_mut().unwrap()), &result);
        assert_eq!(result.stop_reason, StopReason::ToolUse);
        assert_eq!(
            result.content[0],
            AssistantContent::Text {
                text: TEXT.into(),
                signature: None
            }
        );
        let AssistantContent::ToolCall { tool_call } = &result.content[1] else {
            panic!("terminal tool block expected")
        };
        assert_eq!(tool_call.id, "call_alias");
        assert_eq!(tool_call.arguments, ARGS.concat());
        assert_eq!(tool_call.arguments_error, None);

        let after_terminal = self.retained();
        if let Some(write) = &self.write {
            for key in [
                "start",
                "text_start",
                "tool_delta",
                "saved_text",
                "saved_args",
            ] {
                assert_eq!(after_terminal[key], write["after"][key], "retained {key}");
            }
            assert_eq!(write["after"]["first_text"], write["before"]["first_text"]);
            assert_ne!(result.response_id.as_deref(), Some("consumer-id"));
        } else {
            for (key, index) in [
                ("start", 0),
                ("text_start", 1),
                ("first_text", 2),
                ("tool_delta", self.tool_delta),
            ] {
                assert_eq!(after_terminal[key], self.events[index]["message"]);
            }
        }
        if let Some(usage) = &after_usage {
            assert_eq!(usage["latest"]["input_tokens"], 0);
            assert_eq!(result.usage.input_tokens, 7);
        }
        json!({
            "events": self.events, "write": self.write, "after_terminal": after_terminal,
            "after_usage": after_usage, "result": message_view(&result),
        })
    }
}

async fn openai(mutate: bool) -> Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer fixture-key"))
        .and(body_partial_json(json!({
            "model": "gpt-4o", "stream": true,
            "messages": [{"role": "user", "content": "fixture"}],
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(SSE, "text/event-stream"))
        .expect(1)
        .mount(&server)
        .await;
    let mut models = Models::new();
    models
        .register_route(
            Box::new(OpenAiChatProvider::new(Some(server.uri()))),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fixture-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let (seen, mut raw) = mpsc::channel(1);
    let mut request = request();
    request.callbacks = request.callbacks.with_async_on_raw(move |chunk| {
        let seen = seen.clone();
        async move {
            let (release, gate) = oneshot::channel();
            seen.send((chunk, release)).await.expect("raw receiver");
            gate.await.expect("raw release");
            Ok(())
        }
    });
    let mut call = start_call(&models, "openai", "gpt-4o", request);
    let frames: Vec<Value> = SSE
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|line| *line != "[DONE]")
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let groups: &[&[&str]] = &[
        &["start"],
        &["text_start", "text_delta"],
        &["text_delta"],
        &["tool_call_start", "tool_call_delta"],
        &["tool_call_delta"],
        &["tool_call_delta"],
        &["tool_call_delta"],
        &["tool_call_delta"],
        &[],
        &[],
        &["text_end", "tool_call_end", "done"],
    ];
    assert_eq!(frames.len(), groups.len());
    let mut observation = Observation::default();
    let mut next = raw.recv().await;
    let mut after_usage = None;
    for (index, group) in groups.iter().enumerate() {
        let (chunk, release) = next.take().expect("raw frame");
        assert_eq!(chunk, frames[index]);
        release.send(()).expect("raw gate");
        if index + 1 < frames.len() {
            // The next callback is strictly before normalization of its frame.
            next = raw.recv().await;
        }
        observation.read(&mut call, group, mutate).await;
        if chunk.get("usage").is_some() {
            assert!(call.next().now_or_never().is_none());
            after_usage = Some(observation.retained());
        }
    }
    assert!(after_usage.is_some());
    let mut trace = observation.finish(call, after_usage).await;
    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    trace["requests"] = json!(requests.len());
    trace["request"] = serde_json::from_slice::<Value>(&requests[0].body).unwrap();
    trace["raw_frames"] = json!(frames.len());
    trace
}

fn mock_events() -> Vec<AssistantStreamEvent> {
    let mut partial = AssistantMessage {
        stop_reason: StopReason::Pending,
        response_id: Some("faux-alias".into()),
        ..base_assistant()
    };
    let mut events = vec![AssistantStreamEvent::Start {
        partial: partial.clone(),
    }];
    partial.content.push(AssistantContent::Text {
        text: String::new(),
        signature: None,
    });
    events.push(AssistantStreamEvent::TextStart {
        content_index: 0,
        partial: partial.clone(),
    });
    for delta in ["ABCD", "😀EF"] {
        let AssistantContent::Text { text, .. } = &mut partial.content[0] else {
            unreachable!()
        };
        text.push_str(delta);
        events.push(AssistantStreamEvent::TextDelta {
            content_index: 0,
            delta: delta.into(),
            partial: partial.clone(),
        });
    }
    events.push(AssistantStreamEvent::TextEnd {
        content_index: 0,
        content: TEXT.into(),
        partial: partial.clone(),
    });
    partial.content.push(AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "call_alias".into(),
            name: "record".into(),
            arguments: String::new(),
            arguments_error: None,
            signature: None,
        },
    });
    events.push(AssistantStreamEvent::ToolCallStart {
        content_index: 1,
        partial: partial.clone(),
    });
    for delta in ARGS {
        tool(&mut partial.content[1]).arguments.push_str(delta);
        events.push(AssistantStreamEvent::ToolCallDelta {
            content_index: 1,
            delta: delta.into(),
            partial: partial.clone(),
        });
    }
    events.push(AssistantStreamEvent::ToolCallEnd {
        content_index: 1,
        tool_call: tool(&mut partial.content[1]).clone(),
        partial: partial.clone(),
    });
    partial.stop_reason = StopReason::ToolUse;
    events.push(AssistantStreamEvent::Done {
        reason: StopReason::ToolUse,
        message: partial,
    });
    events
}

async fn mock(mutate: bool) -> Value {
    let provider = MockProvider::new("mock", vec![mock_events()]);
    let dispatches = provider.call_log_handle();
    let models = single_route_collection(Box::new(provider));
    let mut call = start_call(&models, "mock", "mock-model", request());
    let mut observation = Observation::default();
    observation
        .read(
            &mut call,
            &[
                "start",
                "text_start",
                "text_delta",
                "text_delta",
                "text_end",
                "tool_call_start",
                "tool_call_delta",
                "tool_call_delta",
                "tool_call_delta",
                "tool_call_delta",
                "tool_call_delta",
                "tool_call_end",
                "done",
            ],
            mutate,
        )
        .await;
    let mut trace = observation.finish(call, None).await;
    let count = dispatches.lock().unwrap().len();
    assert_eq!(count, 1);
    trace["requests"] = json!(count);
    trace
}

#[tokio::test]
async fn partial_aliases() {
    let observations = tokio::time::timeout(Duration::from_secs(10), async {
        json!({
            "openai": {"baseline": openai(false).await, "mutated": openai(true).await},
            "faux": {"baseline": mock(false).await, "mutated": mock(true).await},
        })
    })
    .await
    .expect("partial alias checkpoint timed out");
    println!("\n{observations}");
}
