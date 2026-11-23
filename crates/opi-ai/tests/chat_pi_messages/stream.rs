//! Pi Messages SSE and terminal mapping assertions.

use super::*;

// ---------------------------------------------------------------------------
// SSE consumption: blocks, sparse indices, terminals
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sse_text_thinking_tool_with_sparse_indices() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\n\n",
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"Hel\"}\n\n",
                "data: {\"type\":\"text_end\",\"contentIndex\":0,\"content\":\"Hello\",\"contentSignature\":\"sig-t\"}\n\n",
                "data: {\"type\":\"toolcall_start\",\"contentIndex\":2,\"id\":\"call-9\",\"toolName\":\"lookup\"}\n\n",
                "data: {\"type\":\"toolcall_delta\",\"contentIndex\":2,\"delta\":\"{\\\"q\\\":\"}\n\n",
                "data: {\"type\":\"toolcall_delta\",\"contentIndex\":2,\"delta\":\"\\\"x\\\"}\"}\n\n",
                "data: {\"type\":\"toolcall_end\",\"contentIndex\":2,\"toolCall\":{\"id\":\"call-9\",\"name\":\"lookup\",\"arguments\":{\"q\":\"x\"}}}\n\n",
                "data: {\"type\":\"done\",\"reason\":\"toolUse\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2},\"responseId\":\"resp-2\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;

    assert!(
        events
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Start { .. })))
    );
    let end = events.iter().find_map(|item| match item {
        Ok(AssistantStreamEvent::TextEnd { content, .. }) => Some(content.clone()),
        _ => None,
    });
    assert_eq!(end.as_deref(), Some("Hello"));
    let tool_end = events.iter().find_map(|item| match item {
        Ok(AssistantStreamEvent::ToolCallEnd { tool_call, .. }) => Some(tool_call.clone()),
        _ => None,
    });
    let tool_call = tool_end.expect("tool call completes");
    assert_eq!(tool_call.id, "call-9");
    assert_eq!(tool_call.name, "lookup");
    assert_eq!(tool_call.arguments, "{\"q\":\"x\"}");

    let message = done_message_44(&events).await;
    assert_eq!(message.stop_reason, StopReason::ToolUse);
    assert_eq!(message.response_id.as_deref(), Some("resp-2"));
    let content = &message.content;
    assert_eq!(
        content.len(),
        2,
        "the unstarted hole at index 1 stays empty"
    );
    assert!(matches!(
        &content[0],
        opi_ai::message::AssistantContent::Text { text, signature }
            if text == "Hello" && signature.as_deref() == Some("sig-t")
    ));
    assert!(matches!(
        &content[1],
        opi_ai::message::AssistantContent::ToolCall { tool_call }
            if tool_call.id == "call-9" && tool_call.arguments == "{\"q\":\"x\"}"
    ));
    server.verify().await;
}

#[tokio::test]
async fn thinking_blocks_carry_signature_and_redacted() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"thinking_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"thinking_delta\",\"contentIndex\":0,\"delta\":\"deep\"}\n\n",
                "data: {\"type\":\"thinking_end\",\"contentIndex\":0,\"content\":\"deep thought\",\"contentSignature\":\"sig-h\"}\n\n",
                "data: {\"type\":\"thinking_start\",\"contentIndex\":1}\n\n",
                "data: {\"type\":\"thinking_end\",\"contentIndex\":1,\"content\":\"\",\"contentSignature\":\"enc\",\"redacted\":true}\n\n",
                "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let message = done_message_44(&events).await;
    assert_eq!(message.content.len(), 2);
    assert!(matches!(
        &message.content[0],
        opi_ai::message::AssistantContent::Thinking { thinking, signature }
            if thinking == "deep thought" && signature.as_deref() == Some("sig-h")
    ));
    assert!(matches!(
        &message.content[1],
        opi_ai::message::AssistantContent::RedactedThinking { data }
            if data == "enc"
    ));
    server.verify().await;
}

#[tokio::test]
async fn done_maps_usage_levels_rewrite_and_stops() {
    for (reason, expected) in [("stop", StopReason::Stop), ("length", StopReason::Length)] {
        let server = MockServer::start().await;
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(format!(
                    concat!(
                        "data: {{\"type\":\"done\",\"reason\":\"{}\",",
                        "\"usage\":{{\"input\":10,\"output\":5,\"cacheRead\":3,\"cacheWrite\":2,",
                        "\"cacheWrite1h\":1,\"reasoning\":4,\"totalTokens\":20,",
                        "\"cost\":{{\"input\":0.1,\"output\":0.2,\"cacheRead\":0.01,\"cacheWrite\":0.02,\"total\":0.33}}}},",
                        "\"responseId\":\"resp-3\",\"responseModel\":\"radius-final\",\"providerThinkingLevel\":\"high\",",
                        "\"rewrite\":{{\"policyId\":\"pol-1\",\"policyVersion\":2,\"changed\":true,",
                        "\"tokenCountChange\":-5,\"messageCountChange\":0,\"systemPromptChanged\":false}}}}\n\n",
                    ),
                    reason
                ))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, None, false));
        let events = run_44(&provider, req_44("hi")).await;
        let message = done_message_44(&events).await;

        assert_eq!(message.stop_reason, expected);
        assert_eq!(message.response_id.as_deref(), Some("resp-3"));
        assert_eq!(message.response_model.as_deref(), Some("radius-final"));
        assert_eq!(message.provider_thinking_level.as_deref(), Some("high"));
        assert_eq!(message.usage.input_tokens, 10);
        assert_eq!(message.usage.output_tokens, 5);
        assert_eq!(message.usage.cache_read_tokens, 3);
        assert_eq!(message.usage.cache_write_tokens, 2);
        assert_eq!(message.usage.cache_write_1h_tokens, Some(1));
        assert_eq!(message.usage.reasoning_tokens, Some(4));
        assert_eq!(message.usage.total_tokens, Some(20));
        assert_eq!(message.diagnostics.len(), 1);
        assert_eq!(message.diagnostics[0].kind, "pi_messages_rewrite");
        server.verify().await;
    }
}

#[tokio::test]
async fn terminal_raw_observer_failure_releases_response_and_repeated_done_is_ignored() {
    let server = MockServer::start().await;
    let body = concat!(
        "data: {\"type\":\"start\"}\n\n",
        "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
        "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"partial\"}\n\n",
        "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":10,\"output\":5,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":15},\"responseModel\":\"radius-final\"}\n\n",
        "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":99,\"output\":99,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":198}}\n\n",
    );
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let observed = seen.clone();
    let mut failing = req_44("first");
    failing.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        let kind = event["type"].as_str().unwrap_or("?").to_owned();
        observed.lock().unwrap().push(kind.clone());
        if kind == "done" {
            Err(opi_ai::callbacks::CallbackError::new(
                "terminal observer failed",
            ))
        } else {
            Ok(())
        }
    }));
    let failed = run_44(&provider, failing).await;
    assert!(failed.iter().any(|item| {
        matches!(item, Err(ProviderError::StreamFailure(failure)) if matches!(failure.cause(), ProviderError::CallbackPhase(_)))
    }));
    assert!(
        !failed
            .iter()
            .any(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
    );
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["start", "text_start", "text_delta", "done"]
    );

    let success = run_44(&provider, req_44("second")).await;
    let done = done_message_44(&success).await;
    assert_eq!(done.response_model.as_deref(), Some("radius-final"));
    assert_eq!(done.usage.total_tokens, Some(15));
    assert_eq!(
        success
            .iter()
            .filter(|item| matches!(item, Ok(AssistantStreamEvent::Done { .. })))
            .count(),
        1
    );
    server.verify().await;
}

#[tokio::test]
async fn error_event_fails_typed_with_message() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"partial\"}\n\n",
                "data: {\"type\":\"error\",\"reason\":\"error\",\"usage\":{\"input\":1,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":2},\"errorMessage\":\"upstream exploded\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("upstream exploded"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn aborted_event_maps_cancelled() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"error\",\"reason\":\"aborted\",",
                "\"usage\":{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(matches!(error, ProviderError::Cancelled), "got {error}");
    server.verify().await;
}

#[tokio::test]
async fn missing_terminal_event_fails() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"text_start\",\"contentIndex\":0}\n\n",
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"orphan\"}\n\n",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    match error {
        ProviderError::StreamError(summary) => {
            let text = summary.to_string();
            assert!(text.contains("terminal"), "got {text}");
        }
        other => panic!("expected StreamError, got {other}"),
    }
    server.verify().await;
}

#[tokio::test]
async fn done_marker_crlf_and_trailing_frame_parse() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(concat!(
                "data: {\"type\":\"start\"}\r\n\r\n",
                "data: [DONE]\r\n\r\n",
                "data: {\"type\":\"done\",\"reason\":\"stop\",\"usage\":{\"input\":2,\"output\":1,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":3}}",
            ))
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let message = done_message_44(&events).await;
    assert_eq!(message.usage.total_tokens, Some(3));
    server.verify().await;
}

#[tokio::test]
async fn invalid_events_fail_typed() {
    for bad in [
        "data: {not json}\n\n",
        "data: {\"type\":\"mystery\"}\n\n",
        "data: 42\n\n",
    ] {
        let server = MockServer::start().await;
        mount_44(
            &server,
            ResponseTemplate::new(200)
                .set_body_string(format!(
                    "{bad}data: {{\"type\":\"done\",\"reason\":\"stop\",\"usage\":{{\"input\":0,\"output\":0,\"cacheRead\":0,\"cacheWrite\":0,\"totalTokens\":0}}}}\n\n"
                ))
                .insert_header("content-type", "text/event-stream"),
        )
        .await;
        let provider = PiMessagesProvider::new(Some(server.uri()))
            .with_models(vec![model_44(MODEL_44)])
            .with_native_options(PiMessagesOptions::native(None, None, false));
        let events = run_44(&provider, req_44("hi")).await;
        let error = first_error_44(&events).await;
        match error {
            ProviderError::StreamError(summary) => {
                let text = summary.to_string();
                assert!(
                    text.contains("Invalid pi-messages streaming event"),
                    "got {text}"
                );
            }
            other => panic!("expected StreamError, got {other}"),
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn delta_without_start_fails_closed() {
    let server = MockServer::start().await;
    mount_44(
        &server,
        ResponseTemplate::new(200)
            .set_body_string(
                "data: {\"type\":\"text_delta\",\"contentIndex\":0,\"delta\":\"ghost\"}\n\n",
            )
            .insert_header("content-type", "text/event-stream"),
    )
    .await;
    let provider = PiMessagesProvider::new(Some(server.uri()))
        .with_models(vec![model_44(MODEL_44)])
        .with_native_options(PiMessagesOptions::native(None, None, false));
    let events = run_44(&provider, req_44("hi")).await;
    let error = first_error_44(&events).await;
    assert!(
        matches!(error, ProviderError::StreamError(_)),
        "got {error}"
    );
    server.verify().await;
}
