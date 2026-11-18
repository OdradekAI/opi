use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::provider::{Provider, ProviderError, TransportPreference};
use opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider;
use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{ChatStream, RequestCallbacks};
use serde_json::{Value, json};

use super::ws_path::{CodexPeer, header, read_frame, response_101, sse_response, write_frame};
use super::{auth, model, request};

fn provider(peer: &CodexPeer) -> OpenAiCodexResponsesProvider {
    OpenAiCodexResponsesProvider::new(
        Some(peer.base_url()),
        vec![model(None)],
        Arc::new(
            HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .unwrap(),
        ),
    )
}

#[tokio::test]
async fn codex_terminal_frames_share_status_and_metadata_on_sse_and_websocket() {
    let fixture: Value = serde_json::from_str(include_str!("../data/codex-terminal.json")).unwrap();
    let mut mismatches = Vec::new();
    for row in fixture["cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let content = row["content"].as_str().unwrap_or("text");
        let mut frames = fixture["content"][content].as_array().unwrap().clone();
        let mut response = fixture["response"].as_object().unwrap().clone();
        response.extend(row["fields"].as_object().unwrap().clone());
        frames.push(json!({"type": row["type"], "response": response}));
        let expected_raw: Vec<_> = frames.iter().map(|event| event["type"].clone()).collect();
        if row["late"] == true {
            frames.push(fixture["late"].clone());
        }
        let sse: String = frames
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        let mut websocket = Vec::new();
        for event in &frames {
            write_frame(&mut websocket, 1, event.to_string().as_bytes());
        }
        write_frame(&mut websocket, 8, &1000_u16.to_be_bytes());
        for (transport, preference) in [
            ("sse", TransportPreference::Sse),
            ("websocket", TransportPreference::Websocket),
        ] {
            let websocket = websocket.clone();
            let http = sse_response(&sse, 200);
            let peer = CodexPeer::spawn(move |mut socket, head, capture| {
                if header(&head, "upgrade").is_none() {
                    socket.write_all(&http).unwrap();
                    return Ok(());
                }
                socket.write_all(&response_101(&head, &[])).unwrap();
                let (opcode, body) = read_frame(&mut socket)?;
                assert_eq!(opcode, 1);
                capture
                    .lock()
                    .unwrap()
                    .frames
                    .push(serde_json::from_slice(&body).unwrap());
                socket.write_all(&websocket).unwrap();
                if let Ok((opcode, _)) = read_frame(&mut socket) {
                    assert_eq!(opcode, 8);
                }
                Ok(())
            });
            let provider = provider(&peer);
            let mut call = request();
            call.transport = preference;
            call.session_id = None;
            let expected_body = provider.build_request_body(&call).unwrap();
            let raw = Arc::new(Mutex::new(Vec::<Value>::new()));
            let observed = raw.clone();
            call.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
                observed.lock().unwrap().push(event.clone());
                Ok(())
            }));
            let (outcome, public_events, delivered_text) =
                tokio::time::timeout(Duration::from_secs(5), async {
                    let source = provider
                        .stream_prepared(call, auth("fixture", None, Some("acct-1".into())));
                    let mut source = ChatStream::new(source);
                    let mut events = Vec::new();
                    let mut delivered_text = String::new();
                    while let Some(item) = source.next().await {
                        events.push(match item {
                            Ok(AssistantStreamEvent::Start { .. }) => "start",
                            Ok(AssistantStreamEvent::TextStart { .. }) => "text_start",
                            Ok(AssistantStreamEvent::TextDelta { delta, .. }) => {
                                delivered_text.push_str(&delta);
                                "text_delta"
                            }
                            Ok(AssistantStreamEvent::TextEnd { .. }) => "text_end",
                            Ok(AssistantStreamEvent::ToolCallStart { .. }) => "toolcall_start",
                            Ok(AssistantStreamEvent::ToolCallDelta { .. }) => "toolcall_delta",
                            Ok(AssistantStreamEvent::ToolCallEnd { .. }) => "toolcall_end",
                            Ok(AssistantStreamEvent::Done { .. }) => "done",
                            Err(_) => "error",
                            other => {
                                panic!("{name}/{transport}: unexpected public event {other:?}")
                            }
                        });
                    }
                    (source.result().await, events, delivered_text)
                })
                .await
                .expect("Codex fixture reaches one terminal within the peer deadline");
            let message = outcome.as_ref().unwrap_or_else(|failure| failure.message());
            let text: String = message
                .content
                .iter()
                .filter_map(|part| match part {
                    AssistantContent::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            let tools: Vec<_> = message
                .content
                .iter()
                .filter_map(|part| match part {
                    AssistantContent::ToolCall { tool_call } => Some(json!({
                        "id": tool_call.id, "name": tool_call.name,
                        "args": serde_json::from_str::<Value>(&tool_call.arguments).unwrap(),
                    })),
                    _ => None,
                })
                .collect();
            let end_turn = message.end_turn();
            let decoded: AssistantMessage =
                serde_json::from_value(serde_json::to_value(message).unwrap()).unwrap();
            assert_eq!(decoded.end_turn(), end_turn);
            let raw = raw.lock().unwrap();
            let raw_events: Vec<_> = raw.iter().map(|event| event["type"].clone()).collect();
            let raw_end_turn = &raw[expected_raw.len() - 1]["response"]["end_turn"];
            let capture = peer.finish().unwrap();
            let mut capture = capture.lock().unwrap();
            let ws = capture.frames.len();
            let sse = capture.posts.len();
            let observed = json!({
                "name": name, "transport": transport, "ws": ws, "sse": sse,
                "stop": message.stop_reason, "text": text, "tools": tools,
                "deliveredText": delivered_text, "responseId": message.response_id,
                "reportedModel": message.response_model,
                "usage": {
                    "input": message.usage.input_tokens, "output": message.usage.output_tokens,
                    "cacheRead": message.usage.cache_read_tokens,
                    "reasoning": message.usage.reasoning_tokens, "total": message.usage.total_tokens,
                },
                "rawStop": message.raw_stop_reason, "endTurn": end_turn, "rawEndTurn": raw_end_turn,
                "publicEvents": public_events, "rawEvents": raw_events,
                "diagnostic": message.error_message,
            });
            println!("codex_terminal {observed}");
            let is_error = row["stop"] == "error";
            let mut expected_events = if content == "text" {
                vec!["text_start", "text_delta", "text_end"]
            } else {
                vec!["toolcall_start", "toolcall_delta", "toolcall_end"]
            };
            expected_events.push(if is_error { "error" } else { "done" });
            let expected_tools = if content == "tool" {
                json!([{"id":"call_1|fc_1","name":"read","args":{"path":"README.md"}}])
            } else {
                json!([])
            };
            if observed["stop"] != row["stop"]
                || observed["rawStop"] != row["rawStop"]
                || observed["endTurn"] != row["endTurn"]
                || observed["rawEndTurn"] != row["fields"]["end_turn"]
                || observed["responseId"] != fixture["response"]["id"]
                || observed["reportedModel"] != fixture["response"]["model"]
                || observed["usage"]
                    != json!({"input":7,"output":4,"cacheRead":2,"reasoning":1,"total":13})
                || text != if content == "text" { "partial" } else { "" }
                || delivered_text != text
                || observed["tools"] != expected_tools
                || raw_events != expected_raw
                || public_events != expected_events
                || outcome.is_err() != is_error
            {
                mismatches.push(format!("{name}/{transport}"));
            }
            if let Err(failure) = &outcome {
                assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
                if let Some(diagnostic) = row["diagnostic"].as_str()
                    && !message
                        .error_message
                        .as_deref()
                        .is_some_and(|value| value.ends_with(diagnostic))
                {
                    mismatches.push(format!("{name}/{transport}: diagnostic"));
                }
                if is_error
                    && classify_whole_call_retry(message) != WholeCallRetryClassification::Terminal
                {
                    mismatches.push(format!("{name}/{transport}: retry classification"));
                }
            }
            assert_eq!((ws, sse), if transport == "sse" { (0, 1) } else { (1, 0) });
            for body in &mut capture.frames {
                assert_eq!(
                    body.as_object_mut().unwrap().remove("type").unwrap(),
                    "response.create"
                );
                assert_eq!(body, &expected_body);
            }
            for body in &capture.posts {
                assert_eq!(body, &expected_body);
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "Codex terminal mismatches: {mismatches:?}"
    );
}

#[tokio::test]
async fn prepared_codex_sse_closes_body_at_terminal_before_peer_eof() {
    let peer = CodexPeer::spawn(|mut socket, head, _| {
        assert!(head[0].starts_with("POST "));
        let frames = concat!(
            "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\n",
            "data: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\"}}\n\n",
        );
        write!(
            socket,
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{frames}\r\n",
            frames.len(),
        )
        .unwrap();
        // Leave the SSE body open. The adapter must finish and release it.
        match socket.read(&mut [0; 1]) {
            Ok(0) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => Ok(()),
            other => Err(format!(
                "expected the HTTP client to release the body: {other:?}"
            )),
        }
    });
    let provider = provider(&peer);
    let raw = Arc::new(Mutex::new(Vec::new()));
    let observed = raw.clone();
    let mut call = request();
    call.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        observed.lock().unwrap().push(event["type"].clone());
        Ok(())
    }));
    let mut source = provider.stream_prepared(call, auth("fixture", None, Some("acct-1".into())));
    tokio::time::timeout(Duration::from_secs(2), async {
        assert!(matches!(
            source.next().await,
            Some(Ok(AssistantStreamEvent::Done { .. }))
        ));
        assert!(
            source.next().await.is_none(),
            "source closes after its first terminal"
        );
    })
    .await
    .expect("Codex closes the SSE body while the peer is still open");
    let release = tokio::task::spawn_blocking(move || peer.finish())
        .await
        .unwrap();
    assert_eq!(*raw.lock().unwrap(), [json!("response.completed")]);
    release.unwrap();
}
