//! OpenAI Chat Models lifecycle, timing, callbacks and source-compatibility tests.

use super::fixtures::*;
use super::*;

#[tokio::test]
async fn native_sampling_uses_native_effort_and_request_overrides() {
    for (effort, request_temperature, expected) in [
        (None, None, 0.2),
        (Some(opi_ai::ThinkingLevel::High), None, 0.9),
        (Some(opi_ai::ThinkingLevel::High), Some(0.4), 0.4),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_string(chat_sse_ok()))
            .expect(1)
            .mount(&server)
            .await;
        let model = ModelInfo::new(
            "o3",
            "o3",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(200_000, 100_000).with_thinking(true),
        )
        .with_sampling_params(
            [("temperature".into(), serde_json::json!(0.1))]
                .into_iter()
                .collect(),
        )
        .with_sampling_level_params(
            opi_ai::ThinkingLevel::None,
            [("temperature".into(), serde_json::json!(0.2))]
                .into_iter()
                .collect(),
        )
        .with_sampling_level_params(
            opi_ai::ThinkingLevel::High,
            [("temperature".into(), serde_json::json!(0.9))]
                .into_iter()
                .collect(),
        );
        let (models, handle) = registered_chat_models(server.uri(), model.clone()).await;
        let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
        if let Some(value) = request_temperature {
            request
                .sampling_params
                .get_or_insert_default()
                .insert("temperature".into(), serde_json::json!(value));
        }
        let mut options = OpenAiCompletionsOptions::default();
        options.reasoning_effort = effort;
        models
            .complete(
                ChatModelRecord::new("openai", model),
                request,
                handle.options(options),
            )
            .await
            .expect("native call completes");
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["temperature"], serde_json::json!(expected));
        server.verify().await;
    }
}

#[tokio::test]
async fn models_native_options_reach_the_chat_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let mut budgets = std::collections::BTreeMap::new();
    budgets.insert(opi_ai::ThinkingLevel::High, 512u64);
    let mut value = OpenAiCompletionsOptions::default();
    value.tool_choice = Some(OpenAiToolChoice::Named("list_dir".into()));
    value.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
    value.thinking_budgets = budgets;
    let options = handle.options(value);

    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Minimal));
    // Native options own effort: the neutral field must not leak.
    let message = models
        .complete(o3_record(), request.clone(), options)
        .await
        .expect("native chat call completes");
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);

    let sent = &server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    assert_eq!(
        body["tool_choice"],
        serde_json::json!({"type": "function", "function": {"name": "list_dir"}})
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));
    assert_eq!(
        body["store"],
        serde_json::json!(false),
        "public native path carries the standard OpenAI baseline"
    );
    assert_eq!(
        body["stream_options"]["include_usage"],
        serde_json::json!(true)
    );
    server.verify().await;
}

#[tokio::test]
async fn models_native_unsupported_effort_fails_typed_before_auth() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(0)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let mut value = OpenAiCompletionsOptions::default();
    value.reasoning_effort = Some(opi_ai::ThinkingLevel::XHigh);
    let options = handle.options(value);
    let failure = models
        .complete(o3_record(), chat_public_request("o3", None), options)
        .await
        .expect_err("unsupported native effort fails typed preflight");
    assert!(
        matches!(failure.cause(), opi_ai::provider::ProviderError::Config(_)),
        "typed preflight failure, got {:?}",
        failure.cause()
    );
    server.verify().await;
}

#[tokio::test]
async fn models_simple_neutral_options_and_clamp_reach_the_chat_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, _handle) = registered_chat_models(server.uri(), model).await;

    // Neutral tool choice plus neutral effort through the simple entry.
    let simple = SimpleOptions {
        tool_choice: Some(SimpleToolChoice::Auto),
        ..Default::default()
    };
    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::High));
    models
        .complete_simple(o3_record(), request, simple)
        .await
        .expect("simple chat call completes");

    // XHigh is unsupported on the default map: the neutral entry clamps to
    // the nearest supported level (High) instead of failing.
    let request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::XHigh));
    models
        .complete_simple(o3_record(), request, Default::default())
        .await
        .expect("clamped simple chat call completes");

    let sent = server.received_requests().await.unwrap();
    let first: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(first["tool_choice"], serde_json::json!("auto"));
    assert_eq!(first["reasoning_effort"], serde_json::json!("high"));
    let second: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(
        second["reasoning_effort"],
        serde_json::json!("high"),
        "neutral clamp downgrades XHigh to the nearest supported level"
    );
    server.verify().await;
}

#[tokio::test]
async fn models_direct_adapter_uses_injected_auth_and_returns_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer injected-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 100_000)
            .with_streaming(true)
            .with_thinking(true),
    );
    let (models, handle) = registered_chat_models(server.uri(), model).await;

    let options = handle.options(OpenAiCompletionsOptions::default());
    let mut injected = opi_ai::test_support::resolved_bearer_auth();
    injected.secret = secrecy::SecretString::from("injected-key");
    let message = models
        .complete_direct(
            o3_record(),
            chat_public_request("o3", None),
            options,
            injected,
        )
        .await
        .expect("direct adapter call completes");
    assert_eq!(message.content.len(), 1);
    server.verify().await;
}

#[test]
fn chat_direct_adapter_rejects_wrong_wire_records() {
    let responses_model = ModelInfo::new(
        "gpt-5.4",
        "GPT-5.4",
        opi_ai::WireApi::OpenAiResponses,
        ModelCapabilities::new(200_000, 100_000).with_streaming(true),
    );
    let provider = StdArc::new(OpenAiChatProvider::new(None));
    match OpenAiCompletionsChatApi::new(provider, responses_model) {
        Err(opi_ai::provider::ProviderError::Config(_)) => {}
        other => panic!("wrong-wire rejection is typed, got {other:?}"),
    }
}

#[tokio::test]
async fn chat_done_message_carries_measured_response_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_sse_ok())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut duration = None;
    while let Some(item) = stream.next().await {
        if let Ok(AssistantStreamEvent::Done { message, .. }) = item {
            duration = message.duration_ms;
        }
    }
    assert!(
        duration.is_some(),
        "successful terminal carries the measured response-stream duration"
    );
    server.verify().await;
}

#[tokio::test]
async fn chat_setup_failure_before_activation_has_no_duration() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { .. }) => saw_done = true,
            Ok(_) => {}
            Err(cause) => {
                assert!(
                    !cause.is_retryable(),
                    "unauthorized is a terminal setup failure: {cause}"
                );
            }
        }
    }
    assert!(!saw_done, "setup failure never reaches a terminal Done");
    server.verify().await;
}

#[tokio::test]
async fn chat_sse_frames_split_across_tcp_chunks_reassemble() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let body = chat_sse_ok();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = String::new();
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; 4096];
        // Read until the end of the request headers/body boundary heuristically:
        // read once (headers + small body) then answer.
        let n = socket.read(&mut buf).await.unwrap();
        request.push_str(&String::from_utf8_lossy(&buf[..n]));
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        // Write the SSE body split at awkward byte boundaries.
        for byte in body.bytes() {
            socket.write_all(&[byte]).await.unwrap();
            tokio::task::yield_now().await;
        }
        socket.shutdown().await.unwrap();
    });

    let provider = OpenAiChatProvider::new(Some(format!("http://{addr}")));
    let mut stream = provider.stream_prepared(
        chat_public_request("gpt-4o", None),
        opi_ai::test_support::resolved_bearer_auth(),
    );
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(AssistantStreamEvent::Done { reason, .. }) => {
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                saw_done = true;
            }
            Ok(_) => {}
            Err(cause) => panic!("byte-split frames must reassemble: {cause}"),
        }
    }
    assert!(saw_done, "split-chunk stream still reaches its terminal");
}

#[tokio::test]
async fn multiline_sse_line_endings_survive_http_chunk_splits() {
    async fn run(body: String, split: Option<usize>) -> Vec<AssistantStreamEvent> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 65536];
            let received = socket.read(&mut request).await.unwrap();
            assert!(
                received > 0,
                "client sent request bytes before the response"
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            if let Some(split) = split {
                socket.write_all(&body.as_bytes()[..split]).await.unwrap();
                socket.flush().await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                socket.write_all(&body.as_bytes()[split..]).await.unwrap();
            } else {
                socket.write_all(body.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        });
        let provider = OpenAiChatProvider::new(Some(format!("http://{address}")));
        let results: Vec<_> = provider
            .stream_prepared(
                chat_public_request("gpt-4o", None),
                opi_ai::test_support::resolved_bearer_auth(),
            )
            .collect()
            .await;
        peer.await.unwrap();
        results
            .into_iter()
            .map(|event| event.expect("legal complete multiline event"))
            .collect()
    }

    let body = text_fixture().to_owned();
    for newline in ["\n", "\r\n", "\r"] {
        let multiline = body
            .lines()
            .map(|line| {
                if line.starts_with("data: {") {
                    line.replacen(',', &format!(",{newline}data: "), 1)
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join(newline)
            + newline;
        let split = multiline.find(&format!("{newline}data: ")).unwrap() + 1;
        let whole = run(multiline.clone(), None).await;
        let split = run(multiline, Some(split)).await;
        let kinds = |events: &[AssistantStreamEvent]| {
            events
                .iter()
                .map(|event| serde_json::to_value(event).unwrap()["type"].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&split), kinds(&whole), "newline={newline:?}");
        let final_message = |events: Vec<AssistantStreamEvent>| {
            events
                .into_iter()
                .find_map(|event| match event {
                    AssistantStreamEvent::Done { message, .. } => Some(message),
                    _ => None,
                })
                .expect("terminal outcome")
        };
        let whole = final_message(whole);
        let split = final_message(split);
        assert_eq!(split.content, whole.content);
        assert_eq!(split.stop_reason, whole.stop_reason);
        assert_eq!(split.usage, whole.usage);
        assert!(!split.content.is_empty());
    }
}

#[tokio::test]
async fn chat_duration_includes_pending_first_body_poll() {
    use std::time::Duration;
    let (control, _) = chat_duration_on_peer(Duration::ZERO, Duration::ZERO).await;
    let (delayed, elapsed) =
        chat_duration_on_peer(Duration::from_millis(350), Duration::ZERO).await;
    assert_eq!(delayed.content, control.content);
    assert!(!delayed.content.is_empty());
    assert_eq!(delayed.usage, control.usage);
    assert!(elapsed >= Duration::from_millis(300));
    assert!(
        delayed.duration_ms.unwrap() >= 300,
        "first Pending body poll belongs to response duration: {:?}",
        delayed.duration_ms
    );
}

#[tokio::test]
async fn chat_duration_excludes_auth_setup_callback_and_unpolled_time() {
    use std::time::Duration;
    // Four 150ms intervals: resolver, response headers, response callback,
    // then delayed consumption after the raw terminal. None belongs to body polling.
    let (message, elapsed) =
        chat_duration_on_peer(Duration::ZERO, Duration::from_millis(150)).await;
    let measured = Duration::from_millis(message.duration_ms.unwrap());
    assert!(
        elapsed.saturating_sub(measured) >= Duration::from_millis(500),
        "duration included setup: measured={measured:?}, elapsed={elapsed:?}"
    );
}

#[test]
fn source_compat_chat_cache_markers_and_retention_have_exact_owners() {
    use opi_ai::model_info::CacheControlFormat;
    for retention in [
        CacheRetention::Disabled,
        CacheRetention::Short,
        CacheRetention::Long,
    ] {
        for long in [false, true] {
            let provider = OpenAiChatProvider::new_with_compat(
                None,
                CompatConfig {
                    cache_control_format: Some(CacheControlFormat::Anthropic),
                    supports_long_cache_retention: Some(long),
                    ..Default::default()
                },
            );
            let mut request = make_tool_request();
            request.cache_retention = retention;
            request.session_id = Some("raw-session".repeat(12));
            let canonical = serde_json::to_vec(request.context.messages()).unwrap();
            let body = provider.build_request_body(&request).unwrap();
            if retention == CacheRetention::Disabled {
                assert!(!body.to_string().contains("cache_control"));
                assert!(body.get("prompt_cache_key").is_none());
            } else {
                let marker = if retention == CacheRetention::Long && long {
                    serde_json::json!({"type":"ephemeral","ttl":"1h"})
                } else {
                    serde_json::json!({"type":"ephemeral"})
                };
                assert_eq!(body["messages"][0]["content"][0]["cache_control"], marker);
                assert_eq!(body["messages"][1]["content"][0]["cache_control"], marker);
                assert_eq!(body["tools"][0]["cache_control"], marker);
                assert!(body["tools"][0]["function"].get("cache_control").is_none());
                assert_eq!(
                    body["prompt_cache_key"].as_str().unwrap().chars().count(),
                    64
                );
            }
            assert_eq!(
                body.get("prompt_cache_retention"),
                (retention == CacheRetention::Long && long).then_some(&serde_json::json!("24h"))
            );
            assert_eq!(
                serde_json::to_vec(request.context.messages()).unwrap(),
                canonical
            );
        }
    }
    for (provider_id, expected) in [
        ("together", false),
        ("cloudflare-workers-ai", false),
        ("cloudflare-ai-gateway", false),
        ("nvidia", false),
        ("ant-ling", false),
        ("ordinary", true),
    ] {
        let mut request = make_tool_request();
        request.model = format!("{provider_id}:gpt-4o");
        request.cache_retention = CacheRetention::Long;
        let provider = profile_provider(
            provider_id,
            CompatConfig::default(),
            vec![thinking_chat_model("gpt-4o", false)],
        );
        assert_eq!(
            provider
                .build_request_body(&request)
                .unwrap()
                .get("prompt_cache_retention")
                .is_some(),
            expected,
            "{provider_id}"
        );
    }
}

#[test]
fn source_compat_chat_template_empty_map_clears_and_priority_is_exact() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatThinkingFormat};
    let mut model = thinking_chat_model("clear", true);
    model.compat = opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        chat_template_kwargs: Some(Default::default()),
        vllm_priority: Some(serde_json::Number::from(i64::MAX)),
        ..Default::default()
    });
    let provider = OpenAiChatProvider::for_route(
        Some("http://127.0.0.1:1".into()),
        "custom".into(),
        Default::default(),
        vec![model],
        StdArc::new(opi_ai::http::HttpClient::new()),
    );
    let body = provider
        .build_request_body(&thinking_request(
            "custom",
            "clear",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert!(body.get("chat_template_kwargs").is_none());
    assert_eq!(body["priority"].as_i64(), Some(i64::MAX));
    let kwargs = std::collections::BTreeMap::from([(
        "literal".into(),
        ChatTemplateKwargValue::Literal(
            serde_json::json!({"$var":"thinking.enabled","omitWhenOff":true}),
        ),
    )]);
    let provider = profile_provider(
        "custom",
        CompatConfig {
            thinking_format: Some(ChatThinkingFormat::ChatTemplate),
            chat_template_kwargs: Some(kwargs),
            vllm_priority: Some(serde_json::Number::from_f64(-0.125).unwrap()),
            ..Default::default()
        },
        vec![thinking_chat_model("clear", true)],
    );
    let body = provider
        .build_request_body(&thinking_request("custom", "clear", None))
        .unwrap();
    assert_eq!(body["priority"], -0.125);
    assert_eq!(
        body["chat_template_kwargs"]["literal"],
        serde_json::json!({"$var":"thinking.enabled","omitWhenOff":true})
    );
}

#[tokio::test]
async fn source_compat_chat_affinity_formats_survive_disabled_cache_and_raw_id() {
    use opi_ai::model_info::SessionAffinityFormat;
    for (format, names) in [
        (
            SessionAffinityFormat::OpenAi,
            vec!["session_id", "x-client-request-id", "x-session-affinity"],
        ),
        (
            SessionAffinityFormat::OpenAiNoSession,
            vec!["x-client-request-id", "x-session-affinity"],
        ),
        (SessionAffinityFormat::OpenRouter, vec!["x-session-id"]),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(stalled_openai_chat_terminal_chunk())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let provider = OpenAiChatProvider::new_with_compat(
            Some(server.uri()),
            CompatConfig {
                send_session_affinity_headers: Some(true),
                session_affinity_format: Some(format),
                ..Default::default()
            },
        );
        let mut request = make_test_request();
        request.cache_retention = CacheRetention::Disabled;
        request.session_id = Some("unclamped-session-".repeat(8));
        let id = request.session_id.clone().unwrap();
        let events = provider
            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
            .collect::<Vec<_>>()
            .await;
        assert!(events.iter().all(Result::is_ok));
        let sent = server.received_requests().await.unwrap();
        for name in [
            "session_id",
            "x-client-request-id",
            "x-session-affinity",
            "x-session-id",
        ] {
            if names.contains(&name) {
                assert_eq!(sent[0].headers.get(name).unwrap(), id.as_str());
            } else {
                assert!(!sent[0].headers.contains_key(name));
            }
        }
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert!(body.get("prompt_cache_key").is_none());
    }
}

#[tokio::test]
async fn source_compat_chat_finishless_eof_closes_text_and_tools_without_raw_reason() {
    for supports in [None, Some(true), Some(false)] {
        for tool in [false, true] {
            let delta = if tool {
                serde_json::json!({"role":"assistant","tool_calls":[{"index":0,"id":"call-eof","type":"function","function":{"name":"list_dir","arguments":"{\"path\":"}}]})
            } else {
                serde_json::json!({"role":"assistant","content":"hello"})
            };
            let sse = format!(
                "data: {}\n\ndata: {}\n\n",
                serde_json::json!({"id":"finishless","model":"gpt-4o","choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                serde_json::json!({"choices":[],"usage":{"prompt_tokens":5,"completion_tokens":3}})
            );
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(sse)
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(1)
                .mount(&server)
                .await;
            let provider = OpenAiChatProvider::new_with_compat(
                Some(server.uri()),
                CompatConfig {
                    supports_finish_reason: supports,
                    ..Default::default()
                },
            );
            let request = make_tool_request();
            let cancel = request.cancel.clone();
            let mut stream =
                provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
            let mut items = Vec::new();
            while let Some(item) = stream.next().await {
                items.push(item);
            }
            if supports == Some(false) {
                assert!(items.iter().all(Result::is_ok), "{items:?}");
                let message = items
                    .iter()
                    .find_map(|item| match item {
                        Ok(AssistantStreamEvent::Done { message, .. }) => Some(message),
                        _ => None,
                    })
                    .expect("inferred terminal");
                assert_eq!(
                    message.stop_reason,
                    if tool {
                        StopReason::ToolUse
                    } else {
                        StopReason::Stop
                    }
                );
                assert_eq!(message.raw_stop_reason, None);
                assert!(message.projection.is_some());
                assert!(message.duration_ms.is_some());
                assert_eq!(message.usage.output_tokens, 3);
                if tool {
                    let call = message
                        .content
                        .iter()
                        .find_map(|part| match part {
                            AssistantContent::ToolCall { tool_call } => Some(tool_call),
                            _ => None,
                        })
                        .unwrap();
                    assert!(call.arguments_error.is_some());
                    assert_eq!(call.arguments, "{\"path\":");
                }
            } else {
                assert_eq!(items.iter().filter(|item| item.is_err()).count(), 1);
                assert!(
                    !items
                        .iter()
                        .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
                );
            }
            cancel.cancel();
            assert!(stream.next().await.is_none());
            assert!(stream.next().await.is_none());
        }
    }
}

#[tokio::test]
async fn source_compat_chat_strict_grammar_and_normalizer_controls() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    for strict_cap in [false, true] {
        for intent in [StrictPreference::Prefer, StrictPreference::Require] {
            let mut request = make_tool_request();
            let tool = ToolDef {
                name: "list_dir".into(),
                description: "list".into(),
                input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"optional":{"type":"number"}},"required":["path"]}),
                constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: intent }),
            };
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![tool],
            })
            .unwrap();
            let before = serde_json::to_vec(request.context.messages()).unwrap();
            let provider = OpenAiChatProvider::new_with_compat(
                None,
                CompatConfig {
                    strict_tool_schema: strict_cap,
                    ..Default::default()
                },
            );
            let body = provider.build_request_body(&request);
            if !strict_cap && intent == StrictPreference::Require {
                assert!(matches!(body, Err(ProviderError::Config(_))));
                continue;
            }
            let body = body.unwrap();
            assert_eq!(
                body["tools"][0]["function"].get("strict"),
                strict_cap.then_some(&serde_json::json!(true))
            );
            if strict_cap {
                assert_eq!(
                    body["tools"][0]["function"]["parameters"]["additionalProperties"],
                    false
                );
                assert_eq!(
                    body["tools"][0]["function"]["parameters"]["required"],
                    serde_json::json!(["path", "optional"])
                );
            }
            assert_eq!(
                serde_json::to_vec(request.context.messages()).unwrap(),
                before
            );
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_tool_fixture("{\"path\":\"x\",\"optional\":null}"))
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::new_with_compat(
        Some(server.uri()),
        CompatConfig {
            strict_tool_schema: true,
            ..Default::default()
        },
    );
    let mut request = make_tool_request();
    let tool = ToolDef {
        name: "read_file".into(),
        description: "read".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"optional":{"type":"number"}},"required":["path"]}),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
    };
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![],
        tools: vec![tool],
    })
    .unwrap();
    let events = provider
        .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
        .collect::<Vec<_>>()
        .await;
    let call = events
        .into_iter()
        .find_map(|event| match event.unwrap() {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(call.arguments_error, None);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&call.arguments).unwrap(),
        serde_json::json!({"path":"x"})
    );
    for grammar_cap in [false, true] {
        let mut request = make_tool_request();
        request.context=opi_ai::transcript::normalize_context(opi_ai::transcript::Context{system_prompt:None,messages:vec![],tools:vec![ToolDef{name:"lookup".into(),description:"lookup".into(),input_schema:serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),constrained_sampling:Some(ConstrainedSampling::Grammar{openai_lark:Some("start: /.+/".into()),openai_regex:Some("regex-loses".into())})}]}).unwrap();
        let provider = OpenAiChatProvider::new_with_compat(
            None,
            CompatConfig {
                supports_grammar_tools: Some(grammar_cap),
                ..Default::default()
            },
        );
        let body = provider.build_request_body(&request).unwrap();
        assert_eq!(
            body["tools"][0]["type"],
            if grammar_cap { "custom" } else { "function" }
        );
        if grammar_cap {
            assert_eq!(body["tools"][0]["custom"]["format"]["syntax"], "lark");
        }
    }
}

#[tokio::test]
async fn source_compat_chat_frozen_model_controls_reach_native_and_simple_http() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::model_info::{
        CacheControlFormat, ChatInstructionRole, ChatStoreField, ChatThinkingFormat,
        SessionAffinityFormat,
    };
    let server = MockServer::start().await;
    let sse = format!(
        "data: {}\n\n",
        serde_json::json!({"id":"native-source","model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"done"},"finish_reason":null}]})
    );
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let routing = serde_json::json!({"allow_fallbacks":false,"only":[],"sort":{"by":"latency","partition":null}});
    let compat = CompatConfig {
        store: Some(ChatStoreField::Omit),
        instruction_role: Some(ChatInstructionRole::System),
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        supports_reasoning_effort: Some(false),
        usage_in_stream: Some(false),
        supports_finish_reason: Some(false),
        max_tokens_field: Some("max_tokens".into()),
        supports_grammar_tools: Some(true),
        strict_tool_schema: true,
        open_router_routing: Some(serde_json::from_value(routing.clone()).unwrap()),
        vercel_gateway_routing: Some(
            serde_json::from_value(serde_json::json!({"only":[]})).unwrap(),
        ),
        cache_control_format: Some(CacheControlFormat::Anthropic),
        session_affinity_format: Some(SessionAffinityFormat::OpenAiNoSession),
        send_session_affinity_headers: Some(true),
        supports_long_cache_retention: Some(true),
        vllm_priority: Some(serde_json::Number::from_f64(1.25).unwrap()),
        chat_template_kwargs: Some(std::collections::BTreeMap::from([(
            "zero".into(),
            opi_ai::model_info::ChatTemplateKwargValue::Literal(serde_json::json!(0)),
        )])),
        supports_mid_convo_system_messages: Some(true),
        supports_mid_convo_tool_additions: Some(true),
        ..Default::default()
    };
    let model = ModelInfo::new(
        "o3",
        "o3",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_compat(opi_ai::WireCompat::OpenAiCompletions(compat))
    .unwrap();
    let (models, handle) = registered_chat_models(server.uri(), model.clone()).await;
    let schema_tool = ToolDef {
        name: "structured".into(),
        description: "schema".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
    };
    let grammar_tool = ToolDef {
        name: "grammar".into(),
        description: "grammar".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: /.+/".into()),
            openai_regex: None,
        }),
    };
    let change = opi_ai::transcript::SystemMessage {
        content: "later policy".into(),
        tools_added: vec![grammar_tool],
        ..Default::default()
    };
    let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
    request.cache_retention = CacheRetention::Long;
    request.session_id = Some("native-full-session".repeat(7));
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: Some("initial policy".into()),
        tools: vec![schema_tool],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "before".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::System(change),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "after".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
    })
    .unwrap();
    let canonical = serde_json::to_vec(request.context.messages()).unwrap();
    let mut options = OpenAiCompletionsOptions::default();
    options.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
    let native = models
        .complete(
            ChatModelRecord::new("openai", model.clone()),
            request.clone(),
            handle.options(options),
        )
        .await
        .unwrap();
    let simple = models
        .complete_simple(
            ChatModelRecord::new("openai", model),
            request.clone(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(native.raw_stop_reason, None);
    assert_eq!(simple.raw_stop_reason, None);
    assert!(native.projection.is_some());
    assert!(simple.projection.is_some());
    assert_eq!(
        serde_json::to_vec(request.context.messages()).unwrap(),
        canonical
    );
    for sent in server.received_requests().await.unwrap() {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["provider"], routing);
        assert_eq!(
            body["providerOptions"],
            serde_json::json!({"gateway":{"only":[]}})
        );
        assert_eq!(body["priority"], 1.25);
        assert_eq!(body["chat_template_kwargs"], serde_json::json!({"zero":0}));
        assert!(body.get("store").is_none());
        assert!(body.get("stream_options").is_none());
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("max_tokens").is_some());
        assert!(body.get("max_completion_tokens").is_none());
        assert_eq!(body["prompt_cache_retention"], "24h");
        assert_eq!(body["tools"][0]["function"]["strict"], true);
        assert_eq!(body["messages"][2]["tools"][0]["type"], "custom");
        assert_eq!(body["messages"][3]["role"], "system");
        assert_eq!(body["messages"][3]["content"], "later policy");
        assert_eq!(
            body["messages"][4]["content"][0]["cache_control"]["ttl"],
            "1h"
        );
        assert!(!sent.headers.contains_key("session_id"));
        assert!(!sent.headers.contains_key("x-session-id"));
        assert_eq!(
            sent.headers.get("x-session-affinity").unwrap(),
            request.session_id.as_deref().unwrap()
        );
        assert_eq!(
            sent.headers.get("x-client-request-id").unwrap(),
            request.session_id.as_deref().unwrap()
        );
    }
}

#[test]
fn source_compat_chat_cache_marks_last_eligible_assistant_tool_and_projected_text() {
    use opi_ai::model_info::CacheControlFormat;
    let provider = OpenAiChatProvider::new_with_compat(
        None,
        CompatConfig {
            cache_control_format: Some(CacheControlFormat::Anthropic),
            ..Default::default()
        },
    );
    for images in [false, true] {
        let mut request = tool_loop_request(
            if images {
                image_tool_content()
            } else {
                vec![OutputContent::Text {
                    text: "tool output".into(),
                }]
            },
            false,
            false,
        );
        request.cache_retention = CacheRetention::Short;
        let canonical = serde_json::to_vec(request.context.messages()).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        let messages = body["messages"].as_array().unwrap();
        let last = messages.last().unwrap();
        assert_eq!(last["role"], if images { "user" } else { "tool" });
        let parts = last["content"].as_array().unwrap();
        assert_eq!(
            parts
                .iter()
                .filter(|part| part.get("cache_control").is_some())
                .count(),
            1
        );
        let text = parts
            .iter()
            .rev()
            .find(|part| part["type"] == "text")
            .unwrap();
        assert_eq!(
            text["cache_control"],
            serde_json::json!({"type":"ephemeral"})
        );
        assert_eq!(
            serde_json::to_vec(request.context.messages()).unwrap(),
            canonical
        );
    }
    let mut assistant = opi_ai::test_support::base_assistant();
    assistant.provider = "openai".into();
    assistant.model = "gpt-4o".into();
    assistant.wire = Some(opi_ai::WireApi::OpenAiCompletions);
    assistant.content = vec![AssistantContent::Text {
        text: "cached answer".into(),
        signature: None,
    }];
    let mut request = make_test_request();
    request.cache_retention = CacheRetention::Short;
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        messages: vec![Message::Assistant(assistant)],
        ..Default::default()
    })
    .unwrap();
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["messages"][0]["role"], "assistant");
    assert_eq!(
        body["messages"][0]["content"][0]["cache_control"],
        serde_json::json!({"type":"ephemeral"})
    );
}

#[test]
fn reviewed_chat_plain_thinking_keeps_text_calls_and_independent_required_content() {
    let signature_details =
        serde_json::json!([{"type":"reasoning.text","text":"opaque","id":"trace"}]).to_string();
    for (plain, required, signature) in [
        (true, false, "reasoning_content".to_owned()),
        (false, true, "reasoning".to_owned()),
        (false, true, "reasoning_text".to_owned()),
        (true, true, "reasoning_text".to_owned()),
        (false, true, signature_details.clone()),
        (true, true, signature_details),
    ] {
        let provider = profile_provider(
            "openai",
            CompatConfig {
                requires_thinking_as_text: plain,
                requires_reasoning_content_on_assistant_messages: Some(required),
                ..Default::default()
            },
            vec![thinking_chat_model("gpt-4o", true)],
        );
        let mut assistant = opi_ai::test_support::base_assistant();
        assistant.api = opi_ai::ApiKind::OpenAi;
        assistant.provider = "openai".into();
        assistant.model = "gpt-4o".into();
        assistant.wire = Some(opi_ai::WireApi::OpenAiCompletions);
        assistant.content = vec![
            AssistantContent::Thinking {
                thinking: "plain thought".into(),
                signature: Some(signature.clone()),
            },
            AssistantContent::Text {
                text: "answer".into(),
                signature: None,
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "review-call".into(),
                    name: "read".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: None,
                },
            },
        ];
        let mut request = make_test_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages: vec![Message::Assistant(assistant)],
            ..Default::default()
        })
        .unwrap();
        let canonical = serde_json::to_vec(request.context.messages()).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        let assistant = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["role"] == "assistant")
            .unwrap();
        assert_eq!(assistant["tool_calls"][0]["function"]["name"], "read");
        if plain {
            assert_eq!(
                assistant["content"],
                serde_json::json!([{"type":"text","text":"plain thought"},{"type":"text","text":"answer"}])
            );
            if !required {
                assert!(assistant.get("reasoning_content").is_none());
            }
            assert!(!assistant.to_string().contains("<thinking>"));
        }
        if required {
            assert_eq!(assistant["reasoning_content"], "");
            if signature == "reasoning" || signature == "reasoning_text" {
                if plain {
                    assert!(assistant.get(&signature).is_none());
                } else {
                    assert_eq!(assistant[&signature], "plain thought");
                }
            } else {
                assert_eq!(
                    assistant["reasoning_details"],
                    serde_json::from_str::<serde_json::Value>(&signature).unwrap()
                );
            }
        }
        assert_eq!(
            serde_json::to_vec(request.context.messages()).unwrap(),
            canonical
        );
    }
}

#[tokio::test]
async fn reviewed_chat_zero_max_is_omitted_for_each_field_and_real_entry() {
    for field in ["max_tokens", "max_completion_tokens"] {
        for value in [0, 32] {
            for native in [false, true] {
                let server = MockServer::start().await;
                Mock::given(method("POST"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_string(chat_sse_ok())
                            .insert_header("content-type", "text/event-stream"),
                    )
                    .expect(1)
                    .mount(&server)
                    .await;
                let compat = CompatConfig {
                    max_tokens_field: Some(field.into()),
                    ..Default::default()
                };
                let mut request = chat_public_request("o3", None);
                request.max_tokens = Some(value);
                if native {
                    let model = ModelInfo::new(
                        "o3",
                        "o3",
                        opi_ai::WireApi::OpenAiCompletions,
                        ModelCapabilities::new(8192, 1024).with_streaming(true),
                    )
                    .with_compat(opi_ai::WireCompat::OpenAiCompletions(compat))
                    .unwrap();
                    let (models, handle) =
                        registered_chat_models(server.uri(), model.clone()).await;
                    models
                        .complete(
                            ChatModelRecord::new("openai", model),
                            request,
                            handle.options(Default::default()),
                        )
                        .await
                        .unwrap();
                } else {
                    let provider = OpenAiChatProvider::new_with_compat(Some(server.uri()), compat);
                    let events = provider
                        .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth())
                        .collect::<Vec<_>>()
                        .await;
                    assert!(events.iter().all(Result::is_ok), "{events:?}");
                }
                let sent = server.received_requests().await.unwrap();
                let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
                assert_eq!(
                    body.get(field),
                    (value != 0).then_some(&serde_json::json!(value)),
                    "field={field},value={value},native={native}"
                );
                let other = if field == "max_tokens" {
                    "max_completion_tokens"
                } else {
                    "max_tokens"
                };
                assert!(body.get(other).is_none());
            }
        }
    }
}

#[tokio::test]
async fn reviewed_chat_optional_profile_and_model_precedence_reaches_native_and_simple() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatThinkingFormat};
    for explicit_model in [false, true] {
        let server = MockServer::start().await;
        let peer = if explicit_model {
            chat_sse_ok()
        } else {
            format!(
                "data: {}\n\n",
                serde_json::json!({"model":"o3","choices":[{"index":0,"delta":{"role":"assistant","content":"done"},"finish_reason":null}]})
            )
        };
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(peer)
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(2)
            .mount(&server)
            .await;
        let profile = CompatConfig {
            supports_finish_reason: Some(false),
            supports_grammar_tools: Some(true),
            thinking_format: Some(ChatThinkingFormat::ChatTemplate),
            chat_template_kwargs: Some(std::collections::BTreeMap::from([(
                "profile".into(),
                ChatTemplateKwargValue::Literal(serde_json::json!(false)),
            )])),
            vllm_priority: Some(serde_json::Number::from_f64(1.5).unwrap()),
            ..Default::default()
        };
        let model_compat = if explicit_model {
            CompatConfig {
                supports_finish_reason: Some(true),
                supports_grammar_tools: Some(false),
                chat_template_kwargs: Some(Default::default()),
                vllm_priority: Some(0.into()),
                ..Default::default()
            }
        } else {
            CompatConfig::default()
        };
        let model = ModelInfo::new(
            "o3",
            "o3",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(8192, 1024)
                .with_streaming(true)
                .with_thinking(true),
        )
        .with_compat(opi_ai::WireCompat::OpenAiCompletions(model_compat))
        .unwrap();
        let provider = StdArc::new(OpenAiChatProvider::new_with_compat(
            Some(server.uri()),
            profile,
        ));
        let api = OpenAiCompletionsChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                StdArc::new(api),
                StdArc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::Bearer,
                    secrecy::SecretString::from("offline-profile-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let tool = ToolDef {
            name: "lookup".into(),
            description: "lookup".into(),
            input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
            constrained_sampling: Some(
                opi_ai::constrained_sampling::ConstrainedSampling::Grammar {
                    openai_lark: Some("start: /.+/".into()),
                    openai_regex: None,
                },
            ),
        };
        let mut request = chat_public_request("o3", Some(opi_ai::ThinkingLevel::Low));
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            tools: vec![tool],
            ..Default::default()
        })
        .unwrap();
        let mut options = OpenAiCompletionsOptions::default();
        options.reasoning_effort = Some(opi_ai::ThinkingLevel::High);
        models
            .complete(
                ChatModelRecord::new("openai", model.clone()),
                request.clone(),
                handle.options(options),
            )
            .await
            .unwrap();
        models
            .complete_simple(
                ChatModelRecord::new("openai", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        for sent in server.received_requests().await.unwrap() {
            let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
            assert_eq!(
                body["tools"][0]["type"],
                if explicit_model { "function" } else { "custom" }
            );
            assert_eq!(
                body["priority"],
                if explicit_model {
                    serde_json::json!(0)
                } else {
                    serde_json::json!(1.5)
                }
            );
            if explicit_model {
                assert!(body.get("chat_template_kwargs").is_none());
            } else {
                assert_eq!(
                    body["chat_template_kwargs"],
                    serde_json::json!({"profile":false})
                );
            }
        }
    }
}

#[test]
fn reviewed_chat_model_optional_overrides_apply_to_profile_routes() {
    let model = thinking_chat_model("gpt-4o", false)
        .with_compat(opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
            usage_in_stream: Some(false),
            vllm_priority: Some(0.into()),
            open_router_routing: Some(Default::default()),
            ..Default::default()
        }))
        .unwrap();
    let profile = CompatConfig {
        usage_in_stream: Some(true),
        vllm_priority: Some(3.into()),
        open_router_routing: Some(
            serde_json::from_value(serde_json::json!({"only":["profile"]})).unwrap(),
        ),
        ..Default::default()
    };
    let provider = profile_provider("openai", profile, vec![model]);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("stream_options").is_none());
    assert_eq!(body["priority"], 0);
    assert_eq!(body["provider"], serde_json::json!({}));
}

#[tokio::test]
async fn reviewed_chat_required_constraints_fail_before_dispatch_at_real_entries() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountAuth(StdArc<AtomicUsize>);
    impl opi_ai::auth::AuthResolver for CountAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(opi_ai::test_support::resolved_bearer_auth()) })
        }
    }
    for (cap, schema, valid) in [
        (
            false,
            serde_json::json!({"type":"object","properties":{}}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"array","items":{"type":"string"}}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"object","properties":{},"additionalProperties":true}),
            false,
        ),
        (
            true,
            serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
            true,
        ),
    ] {
        for entry in ["native", "simple", "advanced"] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_string(chat_sse_ok())
                        .insert_header("content-type", "text/event-stream"),
                )
                .expect(u64::from(valid))
                .mount(&server)
                .await;
            let model = ModelInfo::new(
                "o3",
                "o3",
                opi_ai::WireApi::OpenAiCompletions,
                ModelCapabilities::new(8192, 1024).with_streaming(true),
            )
            .with_compat(opi_ai::WireCompat::OpenAiCompletions(CompatConfig {
                strict_tool_schema: cap,
                ..Default::default()
            }))
            .unwrap();
            let auth = StdArc::new(AtomicUsize::new(0));
            let payload = StdArc::new(AtomicUsize::new(0));
            let mut models = Models::new();
            let handle = if entry == "advanced" {
                let provider = OpenAiChatProvider::for_route(
                    Some(server.uri()),
                    "openai".into(),
                    Default::default(),
                    vec![model.clone()],
                    StdArc::new(opi_ai::http::HttpClient::new()),
                );
                models
                    .register_route(
                        Box::new(provider),
                        StdArc::new(CountAuth(auth.clone())),
                        opi_ai::AuthProvenanceSource::Static,
                        Default::default(),
                    )
                    .unwrap();
                None
            } else {
                let api = OpenAiCompletionsChatApi::new(
                    StdArc::new(OpenAiChatProvider::new(Some(server.uri()))),
                    model.clone(),
                )
                .unwrap();
                Some(
                    models
                        .register_chat_api(
                            StdArc::new(api),
                            StdArc::new(CountAuth(auth.clone())),
                            opi_ai::AuthProvenanceSource::Static,
                        )
                        .unwrap(),
                )
            };
            let mut request = chat_public_request("o3", None);
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                tools: vec![ToolDef {
                    name: "required".into(),
                    description: "strict requirement".into(),
                    input_schema: schema.clone(),
                    constrained_sampling: Some(ConstrainedSampling::JsonSchema {
                        strict: StrictPreference::Require,
                    }),
                }],
                ..Default::default()
            })
            .unwrap();
            let sink = payload.clone();
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new().with_on_payload(
                StdArc::new(move |_| {
                    sink.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }),
            );
            assert_eq!(auth.load(Ordering::SeqCst), 0);
            assert_eq!(payload.load(Ordering::SeqCst), 0);
            let result = match entry {
                "native" => {
                    models
                        .complete(
                            ChatModelRecord::new("openai", model),
                            request,
                            handle.as_ref().unwrap().options(Default::default()),
                        )
                        .await
                }
                "simple" => {
                    models
                        .complete_simple(
                            ChatModelRecord::new("openai", model),
                            request,
                            Default::default(),
                        )
                        .await
                }
                _ => {
                    let call = models
                        .prepare_call("openai:o3", request)
                        .await
                        .expect("valid structure prepares the advanced route");
                    opi_ai::stream::ChatStream::new(call.start_attempt().unwrap())
                        .result()
                        .await
                }
            };
            if valid {
                assert!(result.is_ok(), "{entry}: {result:?}");
                assert_eq!(auth.load(Ordering::SeqCst), 1);
                assert_eq!(payload.load(Ordering::SeqCst), 1);
                assert_eq!(server.received_requests().await.unwrap().len(), 1);
            } else {
                let failure =
                    result.expect_err("required constraints never dispatch unsupported tools");
                assert!(
                    matches!(failure.cause(), ProviderError::Config(_)),
                    "{entry}: {:?}",
                    failure.cause()
                );
                assert_eq!(auth.load(Ordering::SeqCst), usize::from(entry != "native"));
                assert_eq!(payload.load(Ordering::SeqCst), 0);
                assert!(server.received_requests().await.unwrap().is_empty());
            }
        }
    }
}
