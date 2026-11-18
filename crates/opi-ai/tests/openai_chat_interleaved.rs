//! Replay the shared wire fixture through the public SDK and into the next turn.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{FutureExt, StreamExt};
use opi_ai::message::{
    AssistantMessage, InputContent, Message, OutputContent, ToolDef, ToolResultMessage,
    UserMessage, parse_streaming_json,
};
use opi_ai::models::CompatMetadata;
use opi_ai::provider::Request;
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, StaticAuthResolver};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SSE: &str = include_str!("data/openai-chat-interleaved.sse");
const CHAT_MODEL: &str = "gpt-4o";

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

fn tool_result() -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: "call_interleaved".into(),
        tool_name: "record".into(),
        content: vec![OutputContent::Text { text: "ok".into() }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
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

async fn observe(sse: String) -> Value {
    let frames: Vec<Value> = sse
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|line| *line != "[DONE]")
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer fixture-key"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse, "text/event-stream"))
        .expect(2)
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("fixture-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let record = ChatModelRecord::new("openai", models.get_model("openai", CHAT_MODEL).unwrap());
    let mut initial = request("openai:gpt-4o", vec![user()]);
    let (seen, mut raw) = mpsc::channel(1);
    initial.callbacks = initial.callbacks.with_async_on_raw(move |chunk| {
        let seen = seen.clone();
        async move {
            let (release, gate) = oneshot::channel();
            seen.send((chunk, release)).await.expect("raw receiver");
            gate.await.expect("raw release");
            Ok(())
        }
    });
    let mut call = models.stream_simple(record.clone(), initial, Default::default());
    let mut events = Vec::new();
    let mut checkpoints = Vec::new();
    let mut next = raw.recv().await.expect("first raw frame");
    for frame in &frames[..frames.len() - 1] {
        assert_eq!(&next.0, frame);
        next.1.send(()).expect("raw gate");
        next = raw.recv().await.expect("next raw frame");
        while let Some(item) = call.next().now_or_never() {
            events.push(event_view(&item.expect("stream before finish").unwrap()));
        }
        checkpoints.push(events.len());
    }
    assert_eq!(&next.0, frames.last().unwrap());
    next.1.send(()).expect("release usage");
    let mut terminal = None;
    while let Some(event) = call.next().await {
        let event = event.unwrap();
        events.push(event_view(&event));
        if let AssistantStreamEvent::Done { message, .. } = event {
            assert!(terminal.replace(message).is_none());
        }
    }
    checkpoints.push(events.len());
    let result = call.result().await.unwrap();
    assert_eq!(terminal.as_ref(), Some(&result));

    let messages = vec![user(), Message::Assistant(result.clone()), tool_result()];
    models
        .stream_simple(
            record,
            request("openai:gpt-4o", messages.clone()),
            Default::default(),
        )
        .result()
        .await
        .unwrap();
    let requests: Vec<Value> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|req| serde_json::from_slice(&req.body).unwrap())
        .collect();
    assert_eq!(requests.len(), 2);
    let anthropic = AnthropicProvider::new(None)
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250929", messages))
        .unwrap();
    let anthropic_assistant = anthropic["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|msg| msg["role"] == "assistant")
        .expect("assistant replay");
    json!({
        "events": events, "checkpoints": checkpoints, "result": message_view(&result),
        "chat_requests": requests, "anthropic_replay": anthropic_assistant["content"],
    })
}

#[tokio::test]
async fn observe_interleaved_chat() {
    let trace = tokio::time::timeout(Duration::from_secs(15), async {
        json!({
            "resumed_text": observe(SSE.into()).await,
            "resumed_whitespace": observe(SSE.replace("😀EF", " \\n")).await,
        })
    })
    .await
    .expect("OpenAI Chat interleaving checkpoints");
    println!("\n{trace}");
}
