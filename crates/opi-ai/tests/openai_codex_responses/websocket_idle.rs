use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::http::{HttpClientBuilder, RedirectPolicy};
use opi_ai::message::{AssistantContent, AssistantMessage, InputContent, Message, UserMessage};
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::provider::{Provider, Request, TransportPreference};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{ChatStream, RequestCallbacks};
use serde_json::{Value, json};

use super::ws_continuation::second_turn_request;
use super::ws_path::{CodexPeer, PeerCapture, header, read_frame, response_101, write_frame};
use super::{auth, model, request};

fn event_frame(output: &mut Vec<u8>, event: &Value) {
    write_frame(output, 1, event.to_string().as_bytes());
}

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

async fn call(provider: &OpenAiCodexResponsesProvider, request: Request) -> Value {
    observe_call(provider, request).await.0
}

async fn observe_call(
    provider: &OpenAiCodexResponsesProvider,
    mut request: Request,
) -> (Value, AssistantMessage) {
    let raw = Arc::new(Mutex::new(Vec::new()));
    let observed = raw.clone();
    request.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        observed.lock().unwrap().push(event["type"].clone());
        Ok(())
    }));
    let mut source = ChatStream::new(
        provider.stream_prepared(request, auth("fixture", None, Some("acct-1".into()))),
    );
    let (terminal, outcome) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut terminal = Vec::new();
        while let Some(item) = source.next().await {
            match item {
                Ok(AssistantStreamEvent::Done { .. }) => terminal.push("done"),
                Err(_) => terminal.push("error"),
                _ => {}
            }
        }
        (terminal, source.result().await)
    })
    .await
    .expect("Codex fixture produces a terminal");
    let message = outcome.unwrap_or_else(|failure| failure.message().clone());
    let text: String = message
        .content
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let observation = json!({
        "stop": message.stop_reason, "responseId": message.response_id, "text": text,
        "totalTokens": message.usage.total_tokens, "endTurn": message.end_turn(),
        "terminal": terminal, "raw": *raw.lock().unwrap(),
    });
    (observation, message)
}

#[tokio::test]
async fn pooled_connection_does_not_deliver_idle_frames_to_the_next_call() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-websocket-idle.json")).unwrap();
    let second_frames = fixture["second"].as_array().unwrap();
    let mut reply = Vec::new();
    for event in second_frames {
        event_frame(&mut reply, event);
    }
    let expected_second = json!({
        "stop": "stop", "responseId": "resp_second", "text": "second answer",
        "totalTokens": 8, "endTurn": true, "terminal": ["done"],
        "raw": second_frames.iter().map(|event| &event["type"]).collect::<Vec<_>>(),
    });
    let mut differences = Vec::new();
    for row in fixture["cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let mut first_write = Vec::new();
        event_frame(&mut first_write, &fixture["first"]);
        let tail = row["tail"].as_array().unwrap();
        for item in tail {
            let key = item.as_str().unwrap();
            if key == "ping" {
                write_frame(&mut first_write, 9, b"idle");
            } else {
                event_frame(&mut first_write, &fixture["late"][key]);
            }
        }
        for (transport, preference) in [
            ("auto", TransportPreference::Auto),
            ("websocket-cached", TransportPreference::WebsocketCached),
            ("websocket", TransportPreference::Websocket),
        ] {
            let first_write = first_write.clone();
            let reply = reply.clone();
            let client_frames = Arc::new(Mutex::new(Vec::new()));
            let observed_frames = client_frames.clone();
            let peer = CodexPeer::spawn(move |mut socket, head, capture| {
                assert!(header(&head, "upgrade").is_some());
                socket.write_all(&response_101(&head, &[])).unwrap();
                for response in [&first_write, &reply] {
                    loop {
                        let (opcode, data) = read_frame(&mut socket)?;
                        if opcode == 10 {
                            assert_eq!(data, b"idle");
                            observed_frames.lock().unwrap().push("pong:idle");
                            continue;
                        }
                        assert_eq!(opcode, 1);
                        observed_frames.lock().unwrap().push("create");
                        capture
                            .lock()
                            .unwrap()
                            .frames
                            .push(serde_json::from_slice(&data).unwrap());
                        break;
                    }
                    // Both the terminal and its tail are in the same peer write.
                    if let Err(error) = socket.write_all(response)
                        && !matches!(
                            error.kind(),
                            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                        )
                    {
                        return Err(error.to_string());
                    }
                }
                while let Ok((opcode, data)) = read_frame(&mut socket) {
                    if opcode == 8 {
                        let mut close = Vec::new();
                        write_frame(&mut close, 8, &data);
                        let _ = socket.write_all(&close);
                        break;
                    }
                    assert_eq!(opcode, 10, "the fixture accepts exactly two creates");
                    assert_eq!(data, b"idle");
                    observed_frames.lock().unwrap().push("pong:idle");
                }
                Ok(())
            });
            let provider = provider(&peer);
            let mut first_request = request();
            first_request.transport = preference;
            let first = call(&provider, first_request).await;
            let mut second_request = second_turn_request();
            second_request.transport = preference;
            let second = call(&provider, second_request).await;
            drop(provider);
            let capture = tokio::task::spawn_blocking(move || peer.finish())
                .await
                .unwrap()
                .unwrap();
            let capture = capture.lock().unwrap();
            let sent: Vec<_> = capture
                .frames
                .iter()
                .map(|frame| {
                    assert_eq!(frame["type"], "response.create");
                    json!({
                        "connection": 1, "previous": frame["previous_response_id"],
                        "items": frame["input"].as_array().unwrap().len(),
                    })
                })
                .collect();
            let client_frames = client_frames.lock().unwrap();
            println!(
                "codex_websocket_idle {}",
                json!({
                    "name": name, "transport": transport, "sockets": capture.heads.len(),
                    "sse": capture.posts.len(), "sent": sent, "clientFrames": *client_frames,
                    "first": first, "second": second,
                })
            );
            assert_eq!(
                first,
                json!({
                    "stop": "stop", "responseId": "resp_first", "text": "", "totalTokens": 5,
                    "endTurn": false, "terminal": ["done"], "raw": ["response.done"],
                }),
                "{name}/{transport}"
            );
            let cached = preference != TransportPreference::Websocket;
            assert_eq!(
                sent,
                [
                    json!({"connection": 1, "previous": null, "items": 1}),
                    json!({"connection": 1, "previous": cached.then_some("resp_first"), "items": if cached { 1 } else { 2 }}),
                ],
                "{name}/{transport}"
            );
            assert_eq!((capture.heads.len(), capture.posts.len()), (1, 0));
            let expected_frames = if tail.iter().any(|item| item == "ping") {
                vec!["create", "pong:idle", "create"]
            } else {
                vec!["create", "create"]
            };
            if second != expected_second || *client_frames != expected_frames {
                differences.push(format!("{name}/{transport}"));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "idle frames crossed into the next call: {differences:?}"
    );
}

#[tokio::test]
async fn unsafe_idle_input_replaces_the_socket_before_the_next_create() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-websocket-idle.json")).unwrap();
    for case in ["partial_header", "peer_close", "idle_message_limit"] {
        let mut first = Vec::new();
        event_frame(&mut first, &fixture["first"]);
        match case {
            "partial_header" => first.push(0x81),
            "peer_close" => write_frame(&mut first, 8, &1000_u16.to_be_bytes()),
            "idle_message_limit" => {
                for _ in 0..16 {
                    event_frame(&mut first, &json!({"type":"response.in_progress"}));
                }
            }
            _ => unreachable!(),
        }
        let mut second = Vec::new();
        for event in fixture["second"].as_array().unwrap() {
            event_frame(&mut second, event);
        }
        let serve =
            move |mut socket: TcpStream, head: Vec<String>, capture: &Arc<Mutex<PeerCapture>>| {
                socket.write_all(&response_101(&head, &[])).unwrap();
                let (opcode, data) = read_frame(&mut socket)?;
                assert_eq!(opcode, 1);
                let connection = {
                    let mut capture = capture.lock().unwrap();
                    capture.frames.push(serde_json::from_slice(&data).unwrap());
                    capture.heads.len()
                };
                if connection == 1 {
                    socket.write_all(&first).unwrap();
                    if case == "peer_close" {
                        let (opcode, _) = read_frame(&mut socket)?;
                        assert_eq!(opcode, 8, "acknowledge Close before replacing the socket");
                    } else {
                        assert!(
                            read_frame(&mut socket).is_err(),
                            "old socket receives one create"
                        );
                    }
                } else {
                    socket.write_all(&second).unwrap();
                    assert!(
                        read_frame(&mut socket).is_err(),
                        "new socket receives one create"
                    );
                }
                Ok(())
            };
        let peer = CodexPeer::spawn_many(vec![serve.clone(), serve], 2);
        let provider = provider(&peer);
        let mut first_request = request();
        first_request.transport = TransportPreference::WebsocketCached;
        let first = call(&provider, first_request).await;
        let mut second_request = second_turn_request();
        second_request.transport = TransportPreference::WebsocketCached;
        let full = provider.build_request_body(&second_request).unwrap();
        let second = call(&provider, second_request).await;
        drop(provider);
        let capture = tokio::task::spawn_blocking(move || peer.finish())
            .await
            .unwrap()
            .unwrap();
        let capture = capture.lock().unwrap();
        assert_eq!(
            (
                capture.heads.len(),
                capture.frames.len(),
                capture.posts.len()
            ),
            (2, 2, 0),
            "{case}"
        );
        let mut sent = capture.frames[1].clone();
        assert_eq!(
            sent.as_object_mut().unwrap().remove("type").unwrap(),
            "response.create"
        );
        assert_eq!(
            sent, full,
            "{case}: the new socket sends full input exactly once"
        );
        assert_eq!(first["responseId"], "resp_first", "{case}");
        assert_eq!(second["responseId"], "resp_second", "{case}");
        assert_eq!(second["text"], "second answer", "{case}");
        assert_eq!(second["terminal"], json!(["done"]), "{case}");
        println!(
            "codex_websocket_idle_retirement {}",
            json!({
                "name": case, "sockets": 2, "creates": 2, "sse": 0,
                "secondItems": sent["input"].as_array().unwrap().len(),
                "previous": sent["previous_response_id"], "second": second,
            })
        );
    }
}

fn identity_frames<'a>(fixture: &'a Value, groups: &[Value]) -> Vec<&'a Value> {
    groups
        .iter()
        .flat_map(|group| {
            fixture["frames"][group.as_str().unwrap()]
                .as_array()
                .unwrap()
        })
        .collect()
}

fn expected_identity_call(frames: &[&Value]) -> Value {
    let terminal = frames.last().unwrap();
    let failed = terminal["type"] == "response.failed";
    let id = frames
        .iter()
        .rev()
        .find_map(|event| event["response"]["id"].as_str());
    let text: String = frames
        .iter()
        .filter_map(|event| event["delta"].as_str())
        .collect();
    json!({
        "stop": if failed { "error" } else { "stop" },
        "responseId": id, "text": text,
        "totalTokens": terminal["response"]["usage"]["total_tokens"],
        "endTurn": terminal["response"]["end_turn"],
        "terminal": [if failed { "error" } else { "done" }],
        "raw": frames.iter().map(|event| &event["type"]).collect::<Vec<_>>(),
    })
}

#[tokio::test]
async fn pooled_connection_attributes_post_send_terminals_by_response_identity() {
    let fixture: Value =
        serde_json::from_str(include_str!("../data/codex-websocket-identity.json")).unwrap();
    let first_frames = identity_frames(&fixture, &[json!("first")]);
    let third_frames = identity_frames(&fixture, &[json!("third")]);
    let mut differences = Vec::new();
    for row in fixture["cases"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let follow_up = row["followUp"] == true;
        let second_frames = identity_frames(&fixture, row["second"].as_array().unwrap());
        let accepted = identity_frames(
            &fixture,
            row["accepted"]
                .as_array()
                .unwrap_or_else(|| row["second"].as_array().unwrap()),
        );
        let responses: Vec<_> = [&first_frames, &second_frames, &third_frames]
            .into_iter()
            .take(if follow_up { 3 } else { 2 })
            .map(|events| {
                let mut frames = Vec::new();
                for event in events {
                    event_frame(&mut frames, event);
                }
                frames
            })
            .collect();
        for (transport, preference) in [
            ("auto", TransportPreference::Auto),
            ("websocket-cached", TransportPreference::WebsocketCached),
            ("websocket", TransportPreference::Websocket),
        ] {
            let responses = responses.clone();
            let peer = CodexPeer::spawn(move |mut socket, head, capture| {
                assert!(header(&head, "upgrade").is_some());
                socket.write_all(&response_101(&head, &[])).unwrap();
                for response in &responses {
                    let (opcode, data) = read_frame(&mut socket)?;
                    assert_eq!(opcode, 1);
                    capture
                        .lock()
                        .unwrap()
                        .frames
                        .push(serde_json::from_slice(&data).unwrap());
                    // Every injected frame in the second write follows receipt of its create.
                    socket
                        .write_all(response)
                        .map_err(|error| error.to_string())?;
                }
                if let Ok((opcode, data)) = read_frame(&mut socket) {
                    assert_eq!(
                        opcode, 8,
                        "the peer received an unexpected additional frame"
                    );
                    let mut close = Vec::new();
                    write_frame(&mut close, 8, &data);
                    let _ = socket.write_all(&close);
                }
                Ok(())
            });
            let provider = provider(&peer);
            let mut first_request = request();
            first_request.transport = preference;
            let first = call(&provider, first_request).await;
            let mut second_request = second_turn_request();
            second_request.transport = preference;
            let (second, second_message) = observe_call(&provider, second_request.clone()).await;
            let third = if follow_up {
                let mut messages = second_request.context.into_messages();
                messages.push(Message::Assistant(second_message));
                messages.push(Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "third turn".into(),
                    }],
                    timestamp_ms: 0,
                }));
                second_request.context = normalize_context(Context {
                    messages,
                    ..Default::default()
                })
                .unwrap();
                second_request.cancel = Default::default();
                Some(call(&provider, second_request).await)
            } else {
                None
            };
            drop(provider);
            let capture = tokio::task::spawn_blocking(move || peer.finish())
                .await
                .unwrap()
                .unwrap();
            let capture = capture.lock().unwrap();
            let sent: Vec<_> = capture
                .frames
                .iter()
                .map(|frame| {
                    assert_eq!(frame["type"], "response.create");
                    json!({
                        "connection": 1, "previous": frame["previous_response_id"],
                        "items": frame["input"].as_array().unwrap().len(),
                    })
                })
                .collect();
            println!(
                "codex_websocket_identity {}",
                json!({
                    "name": name, "transport": transport, "sockets": capture.heads.len(),
                    "sse": capture.posts.len(), "sent": sent, "first": first,
                    "second": second, "third": third,
                })
            );
            assert_eq!(
                (capture.heads.len(), capture.posts.len()),
                (1, 0),
                "{name}/{transport}"
            );
            assert_eq!(
                first,
                expected_identity_call(&first_frames),
                "{name}/{transport}"
            );
            let cached = preference != TransportPreference::Websocket;
            let mut expected_sent = vec![
                json!({"connection": 1, "previous": null, "items": 1}),
                json!({"connection": 1, "previous": cached.then_some("resp_first"), "items": if cached { 1 } else { 2 }}),
            ];
            if follow_up {
                expected_sent.push(json!({
                    "connection": 1, "previous": cached.then_some("resp_second"),
                    "items": if cached { 1 } else { 4 },
                }));
                assert_eq!(
                    third,
                    Some(expected_identity_call(&third_frames)),
                    "{name}/{transport}"
                );
            }
            if second != expected_identity_call(&accepted) || sent != expected_sent {
                differences.push(format!("{name}/{transport}"));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "post-send terminal attribution: {differences:?}"
    );
}
