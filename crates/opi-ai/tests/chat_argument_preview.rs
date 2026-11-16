//! Observe raw Chat arguments and the separate display helper at each wire boundary.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{FutureExt, StreamExt};
use opi_ai::message::{InputContent, Message, ToolDef, UserMessage, parse_streaming_json};
use opi_ai::models::CompatMetadata;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::provider::Request;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::streaming_json::ToolCallArgumentsStream;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, StaticAuthResolver};
use secrecy::SecretString;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CASES: &str = include_str!("data/chat-argument-preview.json");
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
                    "type": "object", "properties": {},
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

async fn observe(case: &Value) -> Value {
    let pieces: Vec<&str> = case["pieces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part.as_str().unwrap())
        .collect();
    let mut frames: Vec<Value> = pieces.iter().enumerate().map(|(index, part)| {
        let mut tool = json!({"index": 0, "function": {"arguments": part}});
        if index == 0 {
            tool["id"] = json!("call_preview");
            tool["type"] = json!("function");
            tool["function"]["name"] = json!("record");
        }
        json!({"id": "preview", "choices": [{"index": 0, "delta": {"tool_calls": [tool]}, "finish_reason": null}]})
    }).collect();
    frames.push(json!({"id": "preview", "choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}]}));
    let sse = frames
        .iter()
        .map(|frame| format!("data: {frame}\n\n"))
        .collect::<String>()
        + "data: [DONE]\n\n";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer fixture-key"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse, "text/event-stream"))
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
    let mut call = models.stream_simple(record, initial, Default::default());
    let mut next = raw.recv().await.expect("first raw frame");
    let mut prefix = String::new();
    let mut previews = Vec::new();
    let mut arguments = ToolCallArgumentsStream::new();
    let mut starts = 0;
    for (index, piece) in pieces.iter().enumerate() {
        assert_eq!(next.0, frames[index]);
        next.1.send(()).unwrap();
        next = raw.recv().await.expect("next raw frame");
        prefix.push_str(piece);
        let mut delta_count = 0;
        while let Some(item) = call.next().now_or_never() {
            match item.expect("premature EOF").unwrap() {
                AssistantStreamEvent::Start { .. } => {}
                AssistantStreamEvent::ToolCallStart { content_index, .. } => {
                    assert_eq!(content_index, 0);
                    starts += 1;
                }
                AssistantStreamEvent::ToolCallDelta {
                    content_index,
                    delta,
                    partial,
                } => {
                    assert_eq!(content_index, 0);
                    assert_eq!(delta, *piece);
                    let opi_ai::message::AssistantContent::ToolCall { tool_call: tool } =
                        &partial.content[0]
                    else {
                        panic!("expected tool block")
                    };
                    assert_eq!(tool.arguments, prefix);
                    assert!(tool.arguments_error.is_none());
                    arguments.append(&delta);
                    assert_eq!(arguments.text(), prefix);
                    previews.push(
                        json!({"raw": tool.arguments, "preview": arguments.snapshot(),
                        "legacy_preview": parse_streaming_json(&tool.arguments)}),
                    );
                    delta_count += 1;
                }
                event => panic!("unexpected event before finish: {event:?}"),
            }
        }
        assert_eq!(delta_count, 1);
    }
    assert_eq!(starts, 1);
    assert_eq!(next.0, *frames.last().unwrap());
    next.1.send(()).unwrap();
    let mut ends = 0;
    let mut terminal = None;
    while let Some(item) = call.next().await {
        match item.unwrap() {
            AssistantStreamEvent::ToolCallEnd { .. } => ends += 1,
            AssistantStreamEvent::Done { message, .. } => {
                assert!(terminal.replace(message).is_none())
            }
            event => panic!("unexpected tail: {event:?}"),
        }
    }
    assert_eq!(ends, 1);
    let result = call.result().await.unwrap();
    assert_eq!(terminal.as_ref(), Some(&result));
    let opi_ai::message::AssistantContent::ToolCall { tool_call: tool } = &result.content[0] else {
        panic!("expected final tool block")
    };
    assert!(tool.arguments_error.is_none());
    let final_value: Value = serde_json::from_str(&tool.arguments).unwrap();
    assert_eq!(final_value, case["final"]);
    let (preview_final, preview_error) = arguments.finish();
    assert!(preview_error.is_none());
    assert_eq!(
        serde_json::from_str::<Value>(&preview_final).unwrap(),
        final_value
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    json!({"name": case["name"], "previews": previews, "final": final_value,
        "request": serde_json::from_slice::<Value>(&requests[0].body).unwrap()})
}

#[tokio::test]
async fn observe_chat_argument_preview() {
    let cases: Vec<Value> = serde_json::from_str(CASES).unwrap();
    let mut traces = Vec::new();
    for case in cases {
        traces.push(
            tokio::time::timeout(Duration::from_secs(10), observe(&case))
                .await
                .expect("Chat preview checkpoints"),
        );
    }
    println!("\n{}", json!({"cases": traces}));
}
