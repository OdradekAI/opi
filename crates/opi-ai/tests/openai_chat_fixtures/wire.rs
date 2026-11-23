//! OpenAI Chat parser, request wire, projection and transport contract tests.

use super::fixtures::*;
use super::*;

#[test]
fn sse_parse_empty_input_yields_no_events() {
    let events = collect_valid_events("");
    assert!(events.is_empty());
}

#[test]
fn sse_parse_skips_non_json_lines() {
    let input = "data: [DONE]\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

#[test]
fn sse_parse_ignores_comments() {
    let input = ": this is a comment\n\n";
    let events = collect_valid_events(input);
    assert!(events.is_empty());
}

#[test]
fn text_fixture_yields_all_events() {
    let events = collect_valid_events(text_fixture());
    // role delta, "Hello" delta, " world" delta, finish_reason delta
    assert_eq!(events.len(), 4);
}

#[test]
fn text_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(text_fixture());

    // Start, TextStart, TextDelta("Hello"), TextDelta(" world"), TextEnd, Done
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::TextStart { .. }
    ));

    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[2] {
        assert_eq!(delta, "Hello");
    } else {
        panic!("expected TextDelta at index 2");
    }

    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[3] {
        assert_eq!(delta, " world");
    } else {
        panic!("expected TextDelta at index 3");
    }

    assert!(matches!(
        stream_events[4],
        AssistantStreamEvent::TextEnd { .. }
    ));

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done at index 5");
    }
}

#[test]
fn text_fixture_done_event_has_full_content() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        let text_content: Vec<_> = message
            .content
            .iter()
            .filter_map(|c| match c {
                AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text_content, vec!["Hello world"]);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn tool_call_fixture_yields_tool_events() {
    let events = collect_valid_events(tool_call_fixture());
    // role delta, tool_call start, arg chunk 1, arg chunk 2, finish_reason
    assert_eq!(events.len(), 5);
}

#[test]
fn tool_call_fixture_maps_to_stream_events() {
    let stream_events = map_fixture(tool_call_fixture());

    // Start, ToolCallStart, ToolCallDelta, ToolCallDelta, ToolCallEnd, Done(tool_use)
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::ToolCallStart { .. }
    ));

    if let AssistantStreamEvent::ToolCallDelta { delta, .. } = &stream_events[2] {
        assert!(delta.contains("path"));
    } else {
        panic!("expected ToolCallDelta at index 2");
    }

    if let AssistantStreamEvent::ToolCallEnd { tool_call, .. } = &stream_events[4] {
        assert_eq!(tool_call.name, "read_file");
        assert_eq!(tool_call.id, "call_abc");
        assert!(tool_call.arguments.contains("path"));
    } else {
        panic!("expected ToolCallEnd at index 4");
    }

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done at index 5");
    }
}

#[test]
fn usage_captured_from_final_chunk() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert!(message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 25);
        assert_eq!(message.usage.output_tokens, 8);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn tool_call_usage_tracked() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert!(message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 50);
        assert_eq!(message.usage.output_tokens, 30);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn chat_chunk_id_round_trips_into_response_id() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { message, .. } = &stream_events[5] {
        assert_eq!(
            message.response_id,
            Some("chatcmpl-abc123".to_string()),
            "OpenAI Chat chunk id must round-trip into AssistantMessage::response_id instead of being dropped"
        );
    } else {
        panic!("expected Done event");
    }
}

#[tokio::test]
async fn content_first_chunk_id_round_trips_into_response_id() {
    let provider = OpenAiChatProvider::new(None);
    let sse = r#"data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-content-first","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    assert!(
        matches!(events.first(), Some(AssistantStreamEvent::Start { .. })),
        "content-first chunks must still emit Start before TextStart/TextDelta: {events:?}"
    );

    let (response_id, model) = events
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some((
                message.response_id,
                message.response_model.unwrap_or_default(),
            )),
            _ => None,
        })
        .expect("Done event");

    assert_eq!(response_id.as_deref(), Some("chatcmpl-content-first"));
    // The provider-reported model is a separate fact from the requested
    // identity (AIR-010/RP-02): it now rides response_model.
    assert_eq!(model, "gpt-4o");
}

#[tokio::test]
async fn non_terminal_usage_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new(None);
    let sse = r#"data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":0}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}],"usage":{"prompt_tokens":7,"completion_tokens":1}}

data: {"id":"chatcmpl-usage-early","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let usage = events
        .into_iter()
        .find_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage),
            _ => None,
        })
        .expect("done event");

    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.output_tokens, 1);
}

#[tokio::test]
async fn usage_only_empty_choices_chunk_updates_done_usage() {
    let provider = OpenAiChatProvider::new(None);
    let sse = r#"data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: {"id":"chatcmpl-usage-only","object":"chat.completion.chunk","created":1,"model":"gpt-4o","choices":[],"usage":{"prompt_tokens":7,"completion_tokens":2}}

"#;

    let events = collect_stream(provider.stream_from_sse(sse, CancellationToken::new())).await;
    let usage = events
        .into_iter()
        .filter_map(|event| match event {
            AssistantStreamEvent::Done { message, .. } => Some(message.usage),
            _ => None,
        })
        .next_back()
        .expect("done event");

    assert_eq!(usage.input_tokens, 7);
    assert_eq!(usage.output_tokens, 2);
}

#[test]
fn cache_tokens_captured_from_final_chunk() {
    let stream_events = map_fixture(cache_tokens_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(
            message.usage.cache_read_tokens, 400,
            "prompt_tokens_details.cached_tokens must map to cache_read_tokens"
        );
    }
}

#[test]
fn reasoning_tokens_captured_from_final_chunk() {
    let stream_events = map_fixture(reasoning_tokens_fixture());
    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert_eq!(message.usage.output_tokens, 500);
        assert_eq!(message.usage.reasoning_tokens, Some(300));
        assert!(message.usage.reasoning_tokens.unwrap() <= u64::from(message.usage.output_tokens));
    }
}

#[test]
fn reasoning_absent_zero_and_equality_are_preserved() {
    let absent = reasoning_tokens_fixture().replace(
        ",\"completion_tokens_details\":{\"reasoning_tokens\":300}",
        "",
    );
    let zero =
        reasoning_tokens_fixture().replace("\"reasoning_tokens\":300", "\"reasoning_tokens\":0");
    let equal =
        reasoning_tokens_fixture().replace("\"reasoning_tokens\":300", "\"reasoning_tokens\":500");

    for (fixture, expected) in [
        (absent.as_str(), None),
        (zero.as_str(), Some(0)),
        (equal.as_str(), Some(500)),
    ] {
        let stream_events = map_fixture(fixture);
        let usage = stream_events
            .iter()
            .find_map(|event| match event {
                AssistantStreamEvent::Done { message, .. } => Some(&message.usage),
                _ => None,
            })
            .expect("expected Done event");
        assert_eq!(usage.reasoning_tokens, expected);
    }
}

#[tokio::test]
async fn reasoning_malformed_subset_stops_production_stream_with_non_retryable_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(reasoning_malformed_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let results: Vec<_> = provider
        .stream_prepared(
            make_test_request(),
            opi_ai::test_support::resolved_bearer_auth(),
        )
        .collect()
        .await;
    let errors: Vec<_> = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect();
    assert_eq!(errors.len(), 1, "invalid usage must emit one error");
    assert!(errors[0].category() == opi_ai::provider::ProviderErrorCategory::Stream);
    assert_eq!(
        errors[0].to_string(),
        "stream error: reasoning_tokens (800) exceeds completion_tokens (500)"
    );
    assert!(!errors[0].is_retryable());
    assert!(matches!(
        results.last(),
        Some(Err(cause)) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream
    ));
    assert!(
        !results.iter().any(|result| matches!(
            result,
            Ok(AssistantStreamEvent::Done { .. } | AssistantStreamEvent::Error { .. })
        )),
        "no completion event may follow malformed usage"
    );
}

#[test]
fn missing_usage_yields_graceful_zero_tokens() {
    let stream_events = map_fixture(missing_usage_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        // No usage chunk in the stream -> zero usage, no panic. This path is shared
        // by OpenRouter/Mistral/Azure, which inherit the OpenAI Chat mapper.
        assert!(!message.usage.is_reported());
        assert_eq!(message.usage.input_tokens, 0);
        assert_eq!(message.usage.output_tokens, 0);
        assert_eq!(message.usage.cache_read_tokens, 0);
        assert_eq!(message.usage.cache_write_tokens, 0);
    }
}

#[test]
fn reported_zero_usage_remains_reported() {
    let stream_events = map_fixture(reported_zero_usage_fixture());

    let done = stream_events
        .iter()
        .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
        .expect("expected Done event");
    if let AssistantStreamEvent::Done { message, .. } = done {
        assert!(
            message.usage.is_reported(),
            "provider-reported zero usage must stay distinct from missing usage"
        );
        assert_eq!(message.usage.input_tokens, 0);
        assert_eq!(message.usage.output_tokens, 0);
        assert_eq!(message.usage.cache_read_tokens, 0);
        assert_eq!(message.usage.cache_write_tokens, 0);
    }
}

#[test]
fn error_fixture_parsed_as_error() {
    let events = collect_valid_events(error_fixture());
    assert!(matches!(events[0], OpenAiChatEvent::Error { .. }));
}

#[test]
fn error_event_maps_to_stream_error() {
    let events: Vec<OpenAiChatEvent> = parse_sse_events(error_fixture())
        .flat_map(|p| match p {
            ParsedEvent::Valid(evts) => evts,
            ParsedEvent::Done | ParsedEvent::UsageError(_) => Vec::new(),
            ParsedEvent::Malformed { .. } => Vec::new(),
        })
        .collect();
    let mut mapper = OpenAiChatMapper::new(
        opi_ai::ApiKind::OpenAi,
        "openai",
        opi_ai::model_info::WireApi::OpenAiCompletions,
        "",
    );
    let items: Vec<_> = events.into_iter().flat_map(|e| mapper.process(e)).collect();

    assert_eq!(items.len(), 1);
    // Current producers stop the dual terminal format: the in-band error
    // event is a stream failure with a bounded typed cause.
    match &items[0] {
        Err(cause) if cause.category() == opi_ai::provider::ProviderErrorCategory::Stream => {
            let summary = cause.to_string();
            assert!(
                summary.contains("openai chat stream error"),
                "cause must be the neutral literal, got: {summary}"
            );
            assert!(
                !summary.contains("Rate limit"),
                "raw upstream error text must not leak into the cause: {summary}"
            );
        }
        other => panic!("expected stream error item, got {other:?}"),
    }
}

#[test]
fn stop_reason_stop_maps_correctly() {
    let stream_events = map_fixture(text_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done with StopReason::Stop");
    }
}

#[test]
fn stop_reason_tool_calls_maps_correctly() {
    let stream_events = map_fixture(tool_call_fixture());

    if let AssistantStreamEvent::Done { reason, .. } = &stream_events[5] {
        assert_eq!(*reason, StopReason::ToolUse);
    } else {
        panic!("expected Done with StopReason::ToolUse");
    }
}

#[test]
fn content_null_delta_without_tool_calls_produces_start_then_text() {
    let input = r#"data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null},"finish_reason":null}]}

data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-null","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}

data: [DONE]

"#;
    let stream_events = map_fixture(input);

    // Start, TextStart, TextDelta("Hello"), TextEnd, Done
    assert!(matches!(
        stream_events[0],
        AssistantStreamEvent::Start { .. }
    ));
    assert!(matches!(
        stream_events[1],
        AssistantStreamEvent::TextStart { .. }
    ));
    if let AssistantStreamEvent::TextDelta { delta, .. } = &stream_events[2] {
        assert_eq!(delta, "Hello");
    } else {
        panic!("expected TextDelta at index 2");
    }
    if let Some(AssistantStreamEvent::Done { reason, .. }) = stream_events.last() {
        assert_eq!(*reason, StopReason::Stop);
    } else {
        panic!("expected Done with StopReason::Stop");
    }
    // No empty TextDelta events
    let empty_deltas: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::TextDelta { delta, .. } if delta.is_empty()))
        .collect();
    assert!(
        empty_deltas.is_empty(),
        "should not emit empty TextDelta events"
    );
}

#[test]
fn stop_reason_length_maps_to_length() {
    let input = r#"data: {"id":"chatcmpl-len","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":"Hi"},"finish_reason":null}]}

data: {"id":"chatcmpl-len","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"length"}]}

data: [DONE]

"#;
    let stream_events = map_fixture(input);

    if let Some(AssistantStreamEvent::Done { reason, .. }) = stream_events.last() {
        assert_eq!(*reason, StopReason::Length);
    } else {
        panic!("expected Done with StopReason::Length");
    }
}

#[test]
fn mixed_fixture_produces_text_then_tool_call() {
    let stream_events = map_fixture(mixed_fixture());

    assert!(matches!(
        stream_events.as_slice(),
        [
            AssistantStreamEvent::Start { .. },
            AssistantStreamEvent::TextStart {
                content_index: 0,
                ..
            },
            AssistantStreamEvent::TextDelta {
                content_index: 0,
                ..
            },
            AssistantStreamEvent::ToolCallStart { .. },
            ..,
            AssistantStreamEvent::TextEnd {
                content_index: 0,
                ..
            },
            AssistantStreamEvent::ToolCallEnd { .. },
            AssistantStreamEvent::Done { .. }
        ]
    ));

    if let Some(AssistantStreamEvent::Done { message, reason }) = stream_events.last() {
        assert_eq!(*reason, StopReason::ToolUse);
        assert_eq!(message.content.len(), 2);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn tool_deltas_keep_the_chat_text_block_open() {
    let events = map_fixture(include_str!("../data/openai-chat-interleaved.sse"));
    assert!(
        matches!(
            events.as_slice(),
            [
                AssistantStreamEvent::Start { .. },
                AssistantStreamEvent::TextStart { content_index: 0, .. },
                AssistantStreamEvent::TextDelta { content_index: 0, .. },
                AssistantStreamEvent::ToolCallStart { content_index: 1, .. },
                AssistantStreamEvent::ToolCallDelta { content_index: 1, .. },
                AssistantStreamEvent::TextDelta { content_index: 0, .. },
                AssistantStreamEvent::ToolCallDelta { content_index: 1, .. },
                AssistantStreamEvent::ToolCallDelta { content_index: 1, .. },
                AssistantStreamEvent::TextEnd { content_index: 0, content, .. },
                AssistantStreamEvent::ToolCallEnd { content_index: 1, .. },
                AssistantStreamEvent::Done { message, reason: StopReason::ToolUse }
            ] if content == "ABCD😀EF"
                && matches!(message.content.as_slice(),
                    [AssistantContent::Text { text, .. }, AssistantContent::ToolCall { tool_call }]
                    if text == content && tool_call.id == "call_interleaved"
                        && tool_call.arguments == r#"{"a":"BCDEF","n":2}"#)
        ),
        "{events:?}"
    );
}

#[test]
fn chat_continuation_preserves_whitespace_after_a_tool_delta() {
    use opi_ai::message::{Message, OutputContent, ToolResultMessage};
    use opi_ai::transcript::{Context, normalize_context};

    let fixture = include_str!("../data/openai-chat-interleaved.sse").replace("😀EF", " \\n");
    let events = map_fixture(&fixture);
    let Some(AssistantStreamEvent::Done { message, .. }) = events.into_iter().last() else {
        panic!("expected Done event");
    };
    let mut request = make_test_request();
    request.context = normalize_context(Context {
        messages: vec![
            Message::Assistant(message),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "call_interleaved".into(),
                tool_name: "record".into(),
                content: vec![OutputContent::Text { text: "ok".into() }],
                details: None,
                is_error: false,
                not_executed: None,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
        ..Default::default()
    })
    .unwrap();
    let body = OpenAiChatProvider::new(None)
        .build_request_body(&request)
        .unwrap();
    assert_eq!(body["messages"][0]["content"], "ABCD \n");
    assert_eq!(
        body["messages"][0]["tool_calls"][0]["id"],
        "call_interleaved"
    );
}

#[test]
fn malformed_sse_data_produces_malformed_event() {
    let input = "data: {invalid json here}\n\n";
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 1);
    assert!(
        matches!(&parsed[0], ParsedEvent::Malformed { .. }),
        "expected Malformed event for invalid JSON data"
    );
}

#[test]
fn malformed_and_valid_events_coexist() {
    let input = "data: {bad json}\n\ndata: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n";
    let parsed: Vec<_> = parse_sse_events(input).collect();
    assert_eq!(parsed.len(), 2);
    assert!(matches!(parsed[0], ParsedEvent::Malformed { .. }));
    assert!(matches!(parsed[1], ParsedEvent::Valid(_))); // Vec<OpenAiChatEvent>
}

#[test]
fn sse_parse_handles_crlf_line_endings() {
    let input = "data: {\"id\":\"chatcmpl-crlf\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"Hi\"},\"finish_reason\":null}]}\r\n\r\n";
    let events = collect_valid_events(input);
    assert_eq!(events.len(), 2);
    assert!(matches!(events[0], OpenAiChatEvent::RoleDelta { .. }));
    assert!(matches!(&events[1], OpenAiChatEvent::ContentDelta { content, .. } if content == "Hi"));
}

#[test]
fn openai_chat_provider_id() {
    let provider = OpenAiChatProvider::new(None);
    assert_eq!(provider.id(), "openai");
}

#[test]
fn openai_chat_provider_models_not_empty() {
    let provider = OpenAiChatProvider::new(None);
    assert!(!provider.models().is_empty());
}

#[tokio::test]
async fn stream_from_sse_produces_events() {
    let provider = OpenAiChatProvider::new(None);
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut stream = provider.stream_from_sse(text_fixture(), cancel);

    let first = stream.next().await.expect("should have an event");
    assert!(first.is_ok());
    assert!(matches!(first.unwrap(), AssistantStreamEvent::Start { .. }));
}

#[test]
fn build_request_body_standard_openai_uses_max_completion_tokens() {
    // Pi 1.3.0 baseline: the standard OpenAI endpoint sends
    // `max_completion_tokens`; `max_tokens` remains the explicit override for
    // legacy-compatible endpoints (deepseek/zai/together/…).
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 4096);
}

#[test]
fn build_request_body_with_compat_max_completion_tokens() {
    let config = CompatConfig {
        max_tokens_field: Some("max_completion_tokens".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("max_tokens").is_none());
    assert_eq!(body["max_completion_tokens"], 4096);
}

#[test]
fn build_request_body_system_role_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    // Default: system message uses "system" role
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
}

#[test]
fn build_request_body_developer_role_override() {
    let config = CompatConfig {
        instruction_role: Some(ChatInstructionRole::Developer),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "developer");
}

#[test]
fn build_request_body_usage_disabled_omits_stream_options() {
    let provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "compat".into(),
        CompatConfig {
            usage_in_stream: Some(false),
            ..CompatConfig::default()
        },
        vec![],
        vec![ModelInfo::new(
            "model",
            "model",
            opi_ai::WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 4096).with_streaming(true),
        )],
    );

    let mut request = make_test_request();
    request.model = "compat:model".into();
    let body = provider.build_request_body(&request).unwrap();
    assert!(body.get("stream_options").is_none());
}

#[test]
fn chat_zai_format_projects_thinking_toggle_and_mapped_effort() {
    // zai is inference-driven: provider id zai selects the zai format.
    let provider = profile_provider(
        "zai",
        Default::default(),
        vec![thinking_chat_model("glm-4.6", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "zai",
            "glm-4.6",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(
        body["thinking"],
        serde_json::json!({"type": "enabled", "clear_thinking": false})
    );
    // The zai baseline never sends the standard effort field.
    assert!(body.get("reasoning_effort").is_none());

    let body = provider
        .build_request_body(&thinking_request("zai", "glm-4.6", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));
}

#[test]
fn chat_qwen_format_projects_enable_thinking_toggle() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Qwen),
        ..Default::default()
    };
    let provider = profile_provider(
        "qwen-host",
        compat,
        vec![thinking_chat_model("qwen3", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "qwen-host",
            "qwen3",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["enable_thinking"], serde_json::json!(true));
    // Standard endpoints accept the effort field alongside the toggle.
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    let body = provider
        .build_request_body(&thinking_request("qwen-host", "qwen3", None))
        .unwrap();
    assert_eq!(body["enable_thinking"], serde_json::json!(false));
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn chat_qwen_chat_template_format_projects_fixed_kwargs() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::QwenChatTemplate),
        ..Default::default()
    };
    let provider = profile_provider(
        "qwen-ct",
        compat,
        vec![thinking_chat_model("qwen3-ct", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "qwen-ct",
            "qwen3-ct",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": true, "preserve_thinking": true})
    );

    let body = provider
        .build_request_body(&thinking_request("qwen-ct", "qwen3-ct", None))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": false, "preserve_thinking": true})
    );
}

#[test]
fn chat_template_format_resolves_dollar_var_kwargs() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar, ChatThinkingFormat};
    let mut kwargs = std::collections::BTreeMap::new();
    kwargs.insert(
        "enable_thinking".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEnabled,
            omit_when_off: false,
        },
    );
    kwargs.insert(
        "effort".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEffort,
            omit_when_off: true,
        },
    );
    kwargs.insert(
        "static".into(),
        ChatTemplateKwargValue::Literal(serde_json::json!(42)),
    );
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        chat_template_kwargs: Some(kwargs),
        ..Default::default()
    };
    let provider = profile_provider("ct", compat, vec![thinking_chat_model("ct-model", true)]);

    let body = provider
        .build_request_body(&thinking_request(
            "ct",
            "ct-model",
            Some(opi_ai::ThinkingLevel::Medium),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": true, "effort": "medium", "static": 42})
    );

    // Off drops the omitWhenOff key and disables the toggle.
    let body = provider
        .build_request_body(&thinking_request("ct", "ct-model", None))
        .unwrap();
    assert_eq!(
        body["chat_template_kwargs"],
        serde_json::json!({"enable_thinking": false, "static": 42})
    );
}

#[test]
fn chat_baseten_format_projects_template_args_and_off_effort() {
    use opi_ai::model_info::{ChatTemplateKwargValue, ChatTemplateKwargVar, ChatThinkingFormat};
    let mut args = std::collections::BTreeMap::new();
    args.insert(
        "thinking".into(),
        ChatTemplateKwargValue::Var {
            var: ChatTemplateKwargVar::ThinkingEnabled,
            omit_when_off: false,
        },
    );
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Baseten),
        chat_template_args: Some(args),
        ..Default::default()
    };
    let model = thinking_chat_model("b1", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Mapped("off-value".into()),
        ),
    );
    let provider = profile_provider("baseten", compat, vec![model]);

    let body = provider
        .build_request_body(&thinking_request(
            "baseten",
            "b1",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert_eq!(
        body["chat_template_args"],
        serde_json::json!({"thinking": true})
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("low"));

    let body = provider
        .build_request_body(&thinking_request("baseten", "b1", None))
        .unwrap();
    assert_eq!(
        body["chat_template_args"],
        serde_json::json!({"thinking": false})
    );
    // Off sends the model's declared off value.
    assert_eq!(body["reasoning_effort"], serde_json::json!("off-value"));
}

#[test]
fn chat_deepseek_format_gates_disabled_on_off_mapping() {
    // deepseek format via inference; the reasoning_content replay marker is
    // covered separately.
    let provider = profile_provider(
        "deepseek",
        Default::default(),
        vec![thinking_chat_model("r1", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "deepseek",
            "r1",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "enabled"}));
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    // reasoning_default maps off to identity (no explicit off value): the
    // disabled marker is sent.
    let body = provider
        .build_request_body(&thinking_request("deepseek", "r1", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!({"type": "disabled"}));

    // A model whose map cannot represent off (Pi `off: null`) never receives
    // the disable marker.
    let null_off = thinking_chat_model("r1-null", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Unsupported,
        ),
    );
    let provider = profile_provider("deepseek", Default::default(), vec![null_off]);
    let body = provider
        .build_request_body(&thinking_request("deepseek", "r1-null", None))
        .unwrap();
    assert!(body.get("thinking").is_none(), "off-null never disables");
}

#[test]
fn chat_openrouter_format_projects_nested_reasoning_effort() {
    let provider = profile_provider(
        "openrouter",
        Default::default(),
        vec![thinking_chat_model("some/model", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "openrouter",
            "some/model",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "high"}));
    assert!(body.get("reasoning_effort").is_none());

    // Off with no declared off value falls back to the format default.
    let body = provider
        .build_request_body(&thinking_request("openrouter", "some/model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "none"}));

    // An explicit off value wins over the default.
    let mapped = thinking_chat_model("mapped/model", true).with_thinking_level_map(
        opi_ai::ThinkingLevelMap::reasoning_default().with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Mapped("disable".into()),
        ),
    );
    let provider = profile_provider("openrouter", Default::default(), vec![mapped]);
    let body = provider
        .build_request_body(&thinking_request("openrouter", "mapped/model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "disable"}));
}

#[test]
fn chat_ant_ling_format_only_sends_mapped_effort() {
    let provider = profile_provider(
        "ant-ling",
        Default::default(),
        vec![thinking_chat_model("ant-1", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "ant-ling",
            "ant-1",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"effort": "high"}));

    // Off sends nothing: the ant-ling branch is effort-gated.
    let body = provider
        .build_request_body(&thinking_request("ant-ling", "ant-1", None))
        .unwrap();
    assert!(body.get("reasoning").is_none());
}

#[test]
fn chat_together_format_projects_reasoning_enabled() {
    let provider = profile_provider(
        "together",
        Default::default(),
        vec![thinking_chat_model("t-model", true)],
    );

    let body = provider
        .build_request_body(&thinking_request(
            "together",
            "t-model",
            Some(opi_ai::ThinkingLevel::High),
        ))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"enabled": true}));
    // The together baseline never sends the standard effort field.
    assert!(body.get("reasoning_effort").is_none());

    let body = provider
        .build_request_body(&thinking_request("together", "t-model", None))
        .unwrap();
    assert_eq!(body["reasoning"], serde_json::json!({"enabled": false}));
}

#[test]
fn chat_string_thinking_format_projects_thinking_string() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::StringThinking),
        ..Default::default()
    };
    let provider = profile_provider("st", compat, vec![thinking_chat_model("st-model", true)]);

    let body = provider
        .build_request_body(&thinking_request(
            "st",
            "st-model",
            Some(opi_ai::ThinkingLevel::Low),
        ))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!("low"));

    let body = provider
        .build_request_body(&thinking_request("st", "st-model", None))
        .unwrap();
    assert_eq!(body["thinking"], serde_json::json!("none"));
}

#[test]
fn chat_thinking_token_budget_field_is_explicit_and_answer_clamped() {
    use opi_ai::model_info::{ChatThinkingFormat, ChatThinkingTokenBudgetField};
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::Qwen),
        thinking_token_budget_field: Some(ChatThinkingTokenBudgetField::ThinkingBudget),
        ..Default::default()
    };
    let provider = profile_provider("qwen-budget", compat, vec![thinking_chat_model("q3", true)]);

    let mut request = thinking_request("qwen-budget", "q3", Some(opi_ai::ThinkingLevel::High));
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    // high defaults to 16384, clamped to 8192-1024.
    assert_eq!(body["thinking_budget"], serde_json::json!(7168));

    // Explicit per-call budget wins over the level default.
    let mut request = thinking_request("qwen-budget", "q3", Some(opi_ai::ThinkingLevel::High));
    request.max_tokens = Some(8192);
    request.thinking.budget_tokens = Some(2000);
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["thinking_budget"], serde_json::json!(2000));

    // No effort intent: no budget field even when configured.
    let mut request = thinking_request("qwen-budget", "q3", None);
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    assert!(body.get("thinking_budget").is_none());
}

#[test]
fn chat_thinking_token_budget_alias_selects_vllm_field() {
    use opi_ai::model_info::ChatThinkingFormat;
    let compat = CompatConfig {
        thinking_format: Some(ChatThinkingFormat::ChatTemplate),
        supports_thinking_token_budget: true,
        ..Default::default()
    };
    let provider = profile_provider("vllm", compat, vec![thinking_chat_model("v1", true)]);
    let mut request = thinking_request("vllm", "v1", Some(opi_ai::ThinkingLevel::Medium));
    request.max_tokens = Some(8192);
    let body = provider.build_request_body(&request).unwrap();
    // medium defaults to 8192 and the shared 8192 ceiling leaves 1024 for the
    // answer: the alias field carries the clamped 7168 budget.
    assert_eq!(body["thinking_token_budget"], serde_json::json!(7168));
}

#[test]
fn chat_zai_tool_stream_and_vllm_priority_are_explicit_only() {
    use opi_ai::model_info::ChatThinkingFormat;
    let mut request = make_tool_request_for("zai-ts");
    request.thinking = ThinkingConfig::default();

    // Off by default: no tool_stream, no priority.
    let plain = profile_provider(
        "zai-ts",
        Default::default(),
        vec![thinking_chat_model("glm", true)],
    );
    let body = plain.build_request_body(&request).unwrap();
    assert!(body.get("tool_stream").is_none());
    assert!(body.get("priority").is_none());

    let compat = CompatConfig {
        zai_tool_stream: true,
        vllm_priority: Some(3.into()),
        thinking_format: Some(ChatThinkingFormat::Qwen),
        ..Default::default()
    };
    let provider = profile_provider("zai-ts", compat, vec![thinking_chat_model("glm", true)]);
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(body["tool_stream"], serde_json::json!(true));
    assert_eq!(body["priority"], serde_json::json!(3));
}

#[test]
fn chat_baseline_inference_standard_openai_wire_defaults() {
    // Frozen SE-01 inference for the standard OpenAI endpoint: `store:false`,
    // developer instruction role only for reasoning models, the modern
    // `max_completion_tokens` field and streaming usage accounting.
    let provider = OpenAiChatProvider::new(None);

    let mut reasoning = request_for("openai", "o3");
    reasoning.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&reasoning).unwrap();
    assert_eq!(
        body["store"],
        serde_json::json!(false),
        "store:false on SendFalse"
    );
    assert_eq!(body["max_completion_tokens"], serde_json::json!(4096));
    assert!(body.get("max_tokens").is_none());
    assert_eq!(
        body["stream_options"]["include_usage"],
        serde_json::json!(true)
    );
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "reasoning model uses developer"
    );
    assert_eq!(body["reasoning_effort"], serde_json::json!("high"));

    let plain = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = plain["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "non-reasoning model keeps system"
    );
    assert_eq!(
        plain["store"],
        serde_json::json!(false),
        "store is model-independent"
    );
}

#[test]
fn chat_baseline_inference_non_standard_endpoint_omits_store_and_effort() {
    // zai is a frozen non-standard endpoint: no `store` field, no standard
    // `reasoning_effort` field, the legacy `max_tokens` field name, system role.
    let provider = profile_provider(
        "zai",
        Default::default(),
        vec![thinking_chat_model("glm-4.6", true)],
    );
    let mut request = request_for("zai", "glm-4.6");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("store").is_none(),
        "non-standard endpoint omits store"
    );
    assert!(
        body.get("reasoning_effort").is_none(),
        "zai baseline never sends the standard effort field"
    );
    assert_eq!(body["max_tokens"], serde_json::json!(4096));
    assert!(body.get("max_completion_tokens").is_none());
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "system");
}

#[test]
fn chat_explicit_store_and_role_overrides_beat_inference() {
    let compat = opi_ai::providers::openai::CompatConfig {
        store: Some(ChatStoreField::Omit),
        instruction_role: Some(ChatInstructionRole::System),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("o3", true)]);
    let mut request = request_for("openai", "o3");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::High,
    };
    let body = provider.build_request_body(&request).unwrap();
    assert!(
        body.get("store").is_none(),
        "explicit Omit wins over SendFalse inference"
    );
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "explicit System wins for a reasoning model"
    );

    let developer_fixed = opi_ai::providers::openai::CompatConfig {
        instruction_role: Some(ChatInstructionRole::Developer),
        ..Default::default()
    };
    let provider = profile_provider(
        "openai",
        developer_fixed,
        vec![thinking_chat_model("gpt-4o", false)],
    );
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "explicit Developer is unconditional"
    );
}

#[test]
fn chat_usage_in_stream_explicit_false_omits_stream_options() {
    let compat = opi_ai::providers::openai::CompatConfig {
        usage_in_stream: Some(false),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("gpt-4o", false)]);
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    assert!(
        body.get("stream_options").is_none(),
        "explicit false disables usage accounting"
    );
}

#[test]
fn chat_max_tokens_field_explicit_override_beats_inference() {
    let compat = opi_ai::providers::openai::CompatConfig {
        max_tokens_field: Some("max_tokens".into()),
        ..Default::default()
    };
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("gpt-4o", false)]);
    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    assert_eq!(body["max_tokens"], serde_json::json!(4096));
    assert!(body.get("max_completion_tokens").is_none());
}

#[test]
fn chat_model_level_override_beats_profile_and_inference() {
    use std::collections::HashMap;

    let compat = opi_ai::providers::openai::CompatConfig {
        instruction_role: Some(ChatInstructionRole::System),
        ..Default::default()
    };
    let mut overrides = HashMap::new();
    overrides.insert(
        "o3".into(),
        opi_ai::providers::openai::ModelCompatOverride {
            instruction_role: Some(ChatInstructionRole::Developer),
            ..Default::default()
        },
    );
    let provider = profile_provider("openai", compat, vec![thinking_chat_model("o3", true)])
        .with_model_overrides(overrides);

    let body = provider
        .build_request_body(&request_for("openai", "o3"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "developer",
        "model override wins over profile"
    );

    let body = provider
        .build_request_body(&request_for("openai", "gpt-4o"))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[0]["role"], "system",
        "profile default applies without override"
    );
}

#[test]
fn build_request_body_strict_capability_preserves_unconstrained_false() {
    let config = CompatConfig {
        strict_tool_schema: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_tool_request()).unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"]["strict"] == false,
        "unconstrained strict:false must be emitted on a capable wire: {tools:?}"
    );
}

#[test]
fn build_request_body_strict_tool_schema_off_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_tool_request()).unwrap();
    let tools = body["tools"].as_array().expect("tools array present");
    assert!(
        tools[0]["function"].get("strict").is_none(),
        "strict must be absent by default: {tools:?}"
    );
}

#[test]
fn build_request_body_reasoning_effort_absent_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("reasoning_effort").is_none());
}

#[test]
fn chat_reasoning_effort_uses_request_thinking_and_model_map() {
    use opi_ai::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};

    let provider = OpenAiChatProvider::new(None);
    let mut identity = make_test_request();
    identity.model = "openai:o3".into();
    identity.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: ThinkingLevel::High,
    };
    assert_eq!(
        provider.build_request_body(&identity).unwrap()["reasoning_effort"],
        "high"
    );

    let mapped_model = ModelInfo::new(
        "mapped",
        "Mapped",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::reasoning_default().with_mapping(
        ThinkingLevel::High,
        ThinkingLevelMapping::Mapped("provider-high".into()),
    ));
    let mapped_provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "mapped".into(),
        CompatConfig::default(),
        vec![],
        vec![mapped_model],
    );
    let mut remapped = identity;
    remapped.model = "mapped:mapped".into();
    assert_eq!(
        mapped_provider.build_request_body(&remapped).unwrap()["reasoning_effort"],
        "provider-high"
    );

    let mut disabled = remapped;
    disabled.thinking.enabled = false;
    assert!(
        mapped_provider
            .build_request_body(&disabled)
            .unwrap()
            .get("reasoning_effort")
            .is_none()
    );

    let mut off = disabled;
    off.thinking.enabled = true;
    off.thinking.level = ThinkingLevel::None;
    assert!(
        mapped_provider
            .build_request_body(&off)
            .unwrap()
            .get("reasoning_effort")
            .is_none()
    );

    let unsupported_model = ModelInfo::new(
        "unsupported",
        "Unsupported",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(128_000, 16_384)
            .with_streaming(true)
            .with_thinking(true),
    )
    .with_thinking_level_map(ThinkingLevelMap::disabled());
    let unsupported_provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "unsupported".into(),
        Default::default(),
        vec![],
        vec![unsupported_model],
    );
    let mut unsupported = off;
    unsupported.model = "unsupported:unsupported".into();
    unsupported.thinking.level = ThinkingLevel::High;
    assert!(
        unsupported_provider
            .build_request_body(&unsupported)
            .unwrap()
            .get("reasoning_effort")
            .is_none()
    );
}

#[test]
fn build_request_body_cache_key_emits_prompt_cache_key() {
    let config = CompatConfig {
        cache_key: Some("sess-abc".into()),
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert_eq!(body["prompt_cache_key"], "sess-abc");
}

#[test]
fn build_request_body_cache_key_absent_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider.build_request_body(&make_test_request()).unwrap();
    assert!(body.get("prompt_cache_key").is_none());
}

#[test]
fn chat_assistant_bridge_inserted_between_tool_result_and_user() {
    let compat = CompatConfig {
        require_assistant_after_tool_result: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, compat);
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "a.rs".into(),
            }],
            false,
            true,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "assistant", "user"],
        "fixed assistant bridge lands between the tool result and the next user"
    );
    let bridge = &messages[3];
    assert_eq!(bridge["content"], "I have processed the tool results.");
    // The tool-call assistant turn keeps content "" (not null) on bridge wires.
    assert_eq!(messages[1]["content"], "");
}

#[test]
fn chat_no_bridge_by_default_between_tool_result_and_user() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "a.rs".into(),
            }],
            false,
            true,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, vec!["user", "assistant", "tool", "user"]);
    // No bridge flag: tool-call assistant turns keep content null.
    assert_eq!(messages[1]["content"], serde_json::Value::Null);
}

#[test]
fn chat_tool_result_images_project_to_trailing_user_message() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&tool_loop_request(image_tool_content(), false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "user"],
        "image parts ride a trailing user message, not the tool message"
    );
    let tool_msg = &messages[2];
    assert_eq!(
        tool_msg["content"], "chart data",
        "text stays on the tool turn"
    );
    assert!(
        tool_msg.get("content").and_then(|c| c.as_array()).is_none(),
        "no image parts inline on the tool message"
    );
    let image_user = &messages[3];
    let parts = image_user["content"].as_array().unwrap();
    assert_eq!(parts.len(), 2, "label text plus one image part");
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[0]["text"], "Attached image(s) from tool result:");
    assert_eq!(parts[1]["type"], "image_url");
    assert_eq!(parts[1]["image_url"]["url"], "data:image/png;base64,aW1n");
}

#[test]
fn chat_tool_result_image_bridge_wire_orders_tool_bridge_then_image_user() {
    let compat = CompatConfig {
        require_assistant_after_tool_result: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, compat);
    let body = provider
        .build_request_body(&tool_loop_request(image_tool_content(), false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "tool", "assistant", "user"]
    );
    assert_eq!(messages[3]["content"], "I have processed the tool results.");
    assert_eq!(
        messages[4]["content"][0]["text"],
        "Attached image(s) from tool result:"
    );
}

#[test]
fn chat_tool_result_placeholders_follow_source_semantics() {
    let provider = OpenAiChatProvider::new(None);

    // Only images: text placeholder names the attachment, not a MIME stub.
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Image {
                source: ImageSource::Base64 {
                    data: "aW1n".into(),
                },
                media_type: MediaType::Png,
            }],
            false,
            false,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "(see attached image)");

    // No content at all: explicit no-output placeholder.
    let body = provider
        .build_request_body(&tool_loop_request(vec![], false, false))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "(no tool output)");

    // Error results keep the deterministic failure marker before the text.
    let body = provider
        .build_request_body(&tool_loop_request(
            vec![OutputContent::Text {
                text: "boom".into(),
            }],
            true,
            false,
        ))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[2]["content"], "[tool_error] boom");
}

#[test]
fn chat_assistant_thinking_replay_uses_signature_reasoning_field() {
    let provider = OpenAiChatProvider::new(None);
    let mut request = make_test_request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "think".into(),
                }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![
                    AssistantContent::Thinking {
                        thinking: "step one".into(),
                        signature: Some("reasoning_content".into()),
                    },
                    AssistantContent::Text {
                        text: "answer".into(),
                        signature: None,
                    },
                ],
                api: opi_ai::ApiKind::OpenAi,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    let replayed = &messages[1];
    assert_eq!(replayed["role"], "assistant");
    assert_eq!(
        replayed["content"], "answer",
        "answer text stays plain string"
    );
    assert_eq!(
        replayed["reasoning_content"], "step one",
        "signed thinking rides its declared reasoning field"
    );
}

#[test]
fn chat_empty_assistant_turns_are_skipped() {
    let provider = OpenAiChatProvider::new(None);
    let mut request = make_test_request();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![],
                api: opi_ai::ApiKind::OpenAi,
                provider: "openai".into(),
                model: "gpt-4o".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    let roles: Vec<&str> = messages
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "user"],
        "content/tool-call-free turns drop"
    );
}

#[test]
fn chat_deepseek_baseline_adds_empty_reasoning_content_on_assistant_messages() {
    let provider = profile_provider(
        "deepseek",
        Default::default(),
        vec![thinking_chat_model("deepseek-reasoner", true)],
    );
    let mut request = request_for("deepseek", "deepseek-reasoner");
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::Assistant(AssistantMessage {
                projection: None,
                content: vec![AssistantContent::Text {
                    text: "answer".into(),
                    signature: None,
                }],
                api: opi_ai::ApiKind::OpenAi,
                provider: "deepseek".into(),
                model: "deepseek-reasoner".into(),
                response_model: None,
                wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
                response_id: None,
                usage: Default::default(),
                stop_reason: opi_ai::stream::StopReason::Stop,
                error_message: None,
                timestamp_ms: 0,
                diagnostics: Vec::new(),
                duration_ms: None,
                raw_stop_reason: None,
                provider_thinking_level: None,

                deferred: None,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "again".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid context declarations");
    let body = provider.build_request_body(&request).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(
        messages[1]["reasoning_content"], "",
        "deepseek baseline marks replayed assistant turns for reasoning replay"
    );
}

#[test]
fn build_request_body_tool_result_name_field_wire_shape() {
    let config = CompatConfig {
        tool_result_name_field: true,
        ..Default::default()
    };
    let provider = OpenAiChatProvider::new_with_compat(None, config);
    let body = provider
        .build_request_body(&make_tool_result_request())
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message present");
    assert_eq!(tool_msg["tool_call_id"], "call_1");
    assert_eq!(
        tool_msg["name"], "list_dir",
        "name field must be present when tool_result_name_field is set"
    );
    assert_eq!(tool_msg["content"], "a.rs");
}

#[test]
fn build_request_body_tool_result_name_field_absent_by_default() {
    let provider = OpenAiChatProvider::new(None);
    let body = provider
        .build_request_body(&make_tool_result_request())
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message present");
    assert!(
        tool_msg.get("name").is_none(),
        "name field must be absent by default"
    );
}

#[test]
fn validate_rejects_image_on_text_only_openai_compatible_profile() {
    // A config-driven OpenAI-compatible profile advertising a text-only model
    // rejects image input through the shared capability preflight before the
    // live call. This proves the unsupported-capability diagnostic flows from
    // the shared profile path (DoD), not a per-adapter special case.
    let text_only = ModelInfo::new(
        "text-only-model",
        "Text Only",
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(8000, 1024).with_streaming(true),
    );
    let provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "compatprof".into(),
        CompatConfig::default(),
        vec![],
        vec![text_only],
    );
    let image_request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "compatprof:text-only-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Image {
                    source: ImageSource::Url {
                        url: "https://example.test/i.png".into(),
                    },
                    media_type: MediaType::Png,
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let err = validate_request_capabilities(&provider, &image_request)
        .expect_err("text-only profile model must reject image preflight");
    assert_eq!(err.category(), ProviderErrorCategory::Capability);
    assert!(
        matches!(err, ProviderError::UnsupportedCapability(_)),
        "must be UnsupportedCapability: {err:?}"
    );
}

#[test]
fn model_level_override_takes_precedence_over_provider_profile() {
    // Phase 12 task 12.3 + task 20.36 normalization: provider/model override
    // precedence. The profile sets a provider-level default (role, max tokens
    // field). A model-level override wins for the model that declares it;
    // other models inherit the profile default. Provider-level flags
    // (strict_tool_schema) apply to all models.
    use opi_ai::model_info::ChatInstructionRole;
    use opi_ai::providers::openai::ModelCompatOverride;
    let base = CompatConfig {
        instruction_role: Some(ChatInstructionRole::System),
        max_tokens_field: Some("max_tokens".into()),
        strict_tool_schema: true,
        ..Default::default()
    };
    let mut overrides = std::collections::HashMap::new();
    overrides.insert(
        "o3".into(),
        ModelCompatOverride {
            instruction_role: Some(ChatInstructionRole::Developer),
            max_tokens_field: Some("max_completion_tokens".into()),
            ..Default::default()
        },
    );
    let provider = OpenAiChatProvider::new_for_profile(
        "https://example.test".into(),
        "prof".into(),
        base,
        vec![],
        vec![],
    )
    .with_model_overrides(overrides);

    // o3: model-level override wins for role + max_tokens field.
    let mut req_o3 = make_tool_request();
    req_o3.model = "prof:o3".into();
    let body = provider.build_request_body(&req_o3).unwrap();
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages[0]["role"], "developer", "model override wins");
    assert!(
        body.get("max_completion_tokens").is_some(),
        "model override selects max_completion_tokens"
    );
    assert!(body.get("max_tokens").is_none());
    // Provider-level strict_tool_schema still applies under the override.
    let tools = body["tools"].as_array().expect("tools present");
    assert!(
        tools[0]["function"]["strict"] == false,
        "provider-level strict still applies"
    );

    // gpt-4o: no model override -> profile default applies.
    let mut req_plain = make_tool_request();
    req_plain.model = "prof:gpt-4o".into();
    let body2 = provider.build_request_body(&req_plain).unwrap();
    let messages2 = body2["messages"].as_array().unwrap();
    assert_eq!(
        messages2[0]["role"], "system",
        "profile default applies when no model override"
    );
    assert!(
        body2.get("max_tokens").is_some(),
        "profile default max_tokens_field applies"
    );
}

#[test]
fn multi_tool_fixture_produces_two_tool_calls() {
    let stream_events = map_fixture(multi_tool_fixture());

    // Start, ToolCallStart(0), ToolCallStart(1), ToolCallDelta(0), ToolCallDelta(1),
    // ToolCallEnd(0), ToolCallEnd(1), Done
    let tool_starts: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallStart { .. }))
        .collect();
    assert_eq!(tool_starts.len(), 2);

    let tool_ends: Vec<_> = stream_events
        .iter()
        .filter(|e| matches!(e, AssistantStreamEvent::ToolCallEnd { .. }))
        .collect();
    assert_eq!(tool_ends.len(), 2);

    if let Some(AssistantStreamEvent::Done { message, .. }) = stream_events.last() {
        assert_eq!(message.content.len(), 2);
    } else {
        panic!("expected Done event");
    }
}

#[test]
fn malformed_tool_args_pass_raw_string_without_panic() {
    let stream_events = map_fixture(malformed_tool_args_fixture());

    let end = stream_events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::ToolCallEnd { tool_call, .. } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("ToolCallEnd emitted despite malformed argument JSON");
    assert_eq!(end.arguments, "{not-json");
    assert_eq!(end.id, "call_bad");
    assert_eq!(end.name, "read_file");
}

#[tokio::test]
async fn stream_sends_text_request_body_and_auth_through_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(body_partial_json(serde_json::json!({
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": "Hello"}
            ],
            "max_completion_tokens": 1024
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    let mut saw_done = false;
    while let Some(result) = stream.next().await {
        match result.expect("matched text request must stream successfully") {
            AssistantStreamEvent::Done { reason, .. } => {
                assert_eq!(reason, opi_ai::stream::StopReason::Stop);
                saw_done = true;
            }
            AssistantStreamEvent::Error { .. } => {
                panic!("text fixture must complete with Done");
            }
            _ => {}
        }
    }
    assert!(saw_done, "text fixture must produce a terminal Done event");

    // verify() confirms the production request carried the OpenAI chat body
    // (system + user messages, max_tokens), the Bearer auth header, and the
    // /v1/chat/completions path.
    server.verify().await;
}

#[tokio::test]
async fn profile_extra_headers_reach_the_http_wire() {
    // Phase 12 task 12.3 (DoD "headers ... from the shared profile path"):
    // a config-driven OpenAI-compatible profile's session-affinity headers,
    // threaded through `new_for_profile` as `extra_headers`, are attached to
    // every outbound HTTP request by the shared adapter.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .and(header("X-Session-Id", "sess-1"))
        .and(header("X-Affinity", "region-eu"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new_for_profile(
        server.uri(),
        "affinityprof".into(),
        CompatConfig::default(),
        vec![
            ("X-Session-Id".into(), "sess-1".into()),
            ("X-Affinity".into(), "region-eu".into()),
        ],
        vec![],
    );
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "affinityprof:any-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(result) = stream.next().await {
        match result {
            Ok(event) if event.is_terminal() => break,
            Err(_) => break,
            _ => {}
        }
    }

    // verify() confirms both profile-declared session-affinity headers were
    // sent on the production HTTP request through the shared adapter path.
    server.verify().await;
}

#[tokio::test]
async fn stream_cancellation_aborts_before_completion() {
    // The CancellationToken is threaded into the OpenAI Chat adapter's HTTP
    // body-stream loop (api/openai_completions.rs `cancel.cancelled()` select arm).
    // This fixture sends one Start-producing chunk, then stalls for a full
    // second before the terminal chunk. Cancellation must close the stream well
    // before that delayed terminal data could arrive.
    let server = spawn_stalled_openai_chat_server().await;

    let cancel = CancellationToken::new();
    let provider = OpenAiChatProvider::new(Some(server));
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "openai:gpt-4o".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: cancel.clone(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());

    let first = stream
        .next()
        .await
        .expect("stream should produce at least one event")
        .expect("first event should be valid");
    assert!(matches!(first, AssistantStreamEvent::Start { .. }));
    cancel.cancel();

    let next = tokio::time::timeout(std::time::Duration::from_millis(200), stream.next())
        .await
        .expect("stream must surface cancellation before the delayed terminal fixture completes");
    match next {
        Some(Err(cause))
            if cause.category() == opi_ai::provider::ProviderErrorCategory::Cancelled =>
        { /* typed cancellation, as contracted */ }
        other => {
            panic!("expected Err(ProviderError::Cancelled) on cancellation, got: {other:?}")
        }
    }
}

#[tokio::test]
async fn chat_final_tool_arguments_empty_input_normalizes_to_object() {
    let tool_call = tool_call_of(chat_final_tool_call("").await);
    assert_eq!(tool_call.arguments, "{}");
    assert_eq!(tool_call.arguments_error, None);
}

#[tokio::test]
async fn chat_final_tool_arguments_invalid_escape_repairs_literal_text() {
    let tool_call = tool_call_of(chat_final_tool_call(r#"{"cmd":"\usepackage{x}","n":2}"#).await);
    assert_eq!(tool_call.arguments_error, None);
    let parsed: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&tool_call.arguments).expect("repaired arguments parse");
    assert_eq!(parsed["cmd"], r#"\usepackage{x}"#);
    assert_eq!(parsed["n"], 2);
}

#[tokio::test]
async fn chat_final_tool_arguments_truncated_keeps_raw_and_marks_error() {
    let raw = r#"{"path": "trunc"#;
    let tool_call = tool_call_of(chat_final_tool_call(raw).await);
    assert_eq!(tool_call.arguments, raw);
    assert_eq!(
        tool_call.arguments_error.as_deref(),
        Some("final tool arguments were not a complete JSON object")
    );
}

#[tokio::test]
async fn chat_keepalive_frames_are_ignored_between_events() {
    // Covered inline by chat_tool_fixture's keepalive frame: the fixture above
    // parses to completion with the keepalive line present.
    let events: Vec<_> = parse_sse_events(&chat_tool_fixture("{}"))
        .flat_map(|parsed| match parsed {
            ParsedEvent::Valid(evts) => evts,
            _ => Vec::new(),
        })
        .collect();
    assert!(!events.is_empty(), "keepalive frame does not break parsing");
}

#[test]
fn initial_chat_tool_call_arguments_are_emitted_after_start() {
    let chunk = serde_json::json!({
        "id": "response-1",
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "read", "arguments": "{\"path\":"}
                }]
            }
        }]
    });
    let sse = format!("data: {chunk}\n\n");
    let events = match opi_ai::api::openai_completions::parse_sse_events(&sse)
        .next()
        .expect("one frame")
    {
        opi_ai::api::openai_completions::ParsedEvent::Valid(events) => events,
        opi_ai::api::openai_completions::ParsedEvent::Done => panic!("expected a data frame"),
        opi_ai::api::openai_completions::ParsedEvent::UsageError(error) => panic!("{error}"),
        opi_ai::api::openai_completions::ParsedEvent::Malformed { error, .. } => panic!("{error}"),
    };

    assert!(matches!(
        events.first(),
        Some(OpenAiChatEvent::ToolCallStart { .. })
    ));
    assert!(matches!(
        events.get(1),
        Some(OpenAiChatEvent::ToolCallDelta { arguments, .. }) if arguments == "{\"path\":"
    ));
}

#[tokio::test]
async fn chat_payload_callback_sees_merged_sampling_and_wire_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let seen: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.temperature = Some(0.5);
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(move |body| {
        recorder.lock().unwrap().push(body.clone());
        Ok(None)
    }));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            panic!("unexpected error: {error}");
        }
    }

    let bodies = seen.lock().unwrap().clone();
    assert_eq!(bodies.len(), 1);
    let sent = server.received_requests().await.unwrap()[0].clone();
    let wire_body: serde_json::Value =
        serde_json::from_slice(&sent.body).expect("captured body is JSON");
    assert_eq!(
        bodies[0], wire_body,
        "payload callback observes the exact wire body after sampling merge"
    );
    assert_eq!(bodies[0]["top_p"], serde_json::json!(0.9));
    assert_eq!(bodies[0]["temperature"], serde_json::json!(0.5));
}

#[tokio::test]
async fn chat_payload_replacement_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::body_partial_json(
            serde_json::json!({"chat_marker": "replaced"}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.callbacks = RequestCallbacks::new().with_on_payload(std::sync::Arc::new(|body| {
        let mut replacement = body.clone();
        replacement["chat_marker"] = serde_json::json!("replaced");
        Ok(Some(replacement))
    }));

    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if let Err(error) = item {
            panic!("unexpected error: {error}");
        }
    }
    server.verify().await;
}

#[tokio::test]
async fn chat_async_response_and_raw_callbacks_fire_in_source_order() {
    let order: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let raw_recorder = order.clone();
    let response_recorder = order.clone();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(chat_callbacks_sse())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let mut request = chat_callbacks_request();
    request.callbacks = RequestCallbacks::new()
        .with_async_header_transform(|headers| async move {
            tokio::task::yield_now().await;
            Ok(headers)
        })
        .with_async_on_payload(|_| async {
            tokio::task::yield_now().await;
            Ok(None)
        })
        .with_async_on_response(move |status, _headers| {
            let response_recorder = response_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                response_recorder
                    .lock()
                    .unwrap()
                    .push(format!("response:{status}"));
                Ok(())
            }
        })
        .with_async_on_raw(move |event| {
            let raw_recorder = raw_recorder.clone();
            async move {
                tokio::task::yield_now().await;
                // The sentinel `[DONE]` frame is not JSON and is never a raw event.
                raw_recorder
                    .lock()
                    .unwrap()
                    .push(format!("raw:{}", event["object"].as_str().unwrap_or("?")));
                Ok(())
            }
        });

    let norm_recorder = order.clone();
    let mut stream =
        provider.stream_prepared(request, opi_ai::test_support::resolved_bearer_auth());
    while let Some(item) = stream.next().await {
        if item.is_ok() {
            norm_recorder.lock().unwrap().push("norm".into());
        }
    }

    let order = order.lock().unwrap();
    let markers = order.iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(markers.first(), Some(&"response:200"));
    // Three JSON chunks, each raw before its normalized emission.
    let raw_count = markers
        .iter()
        .filter(|m| **m == "raw:chat.completion.chunk")
        .count();
    assert_eq!(raw_count, 3);
    assert!(markers.iter().filter(|m| **m == "norm").count() >= 3);
    assert!(
        markers
            .iter()
            .position(|m| *m == "raw:chat.completion.chunk")
            .unwrap()
            < markers.iter().position(|m| *m == "norm").unwrap(),
        "raw precedes normalization"
    );
}
