//! Public text/system rendering utility family (task 20.53.1, P20-S53-1).
//!
//! Pinned source: `.repo/pi-1.3.0/packages/ai/src/utils/text.ts` — visible
//! text extraction in exact block order (never thinking, tool calls or
//! images) plus the transcript-owned system renderers, exercised together
//! against the DoD: canonical body/section chronology and updates in exact
//! order, no canonical input mutation, no thinking exposure, initial/current
//! distinctions preserved.

use opi_ai::message::{
    AssistantContent, ImageSource, InputContent, MediaType, Message, OutputContent, ToolCall,
    assistant_content_text, content_text, output_content_text,
};
use opi_ai::transcript::{
    SystemMessage, SystemSection, get_current_system_message, get_current_system_prompt,
    get_initial_system_message, render_system_message_update, system_message_text,
};

fn text(value: &str) -> InputContent {
    InputContent::Text {
        text: value.to_string(),
    }
}

fn image() -> InputContent {
    InputContent::Image {
        source: ImageSource::Base64 {
            data: "aGk=".to_string(),
        },
        media_type: MediaType::Png,
    }
}

fn system(content: &str, sections: &[(&str, Option<&str>)]) -> Message {
    Message::System(SystemMessage {
        content: content.to_string(),
        sections: sections
            .iter()
            .map(|(name, value)| SystemSection {
                name: (*name).to_string(),
                value: value.map(|value| value.to_string()),
            })
            .collect(),
        tools_added: Vec::new(),
        tools_removed: Vec::new(),
        timestamp_ms: 0,
    })
}

#[test]
fn content_text_extracts_only_text_blocks_in_order() {
    let blocks = [text("first"), image(), text("second"), text("third")];
    assert_eq!(content_text(&blocks), "first\nsecond\nthird");
    assert_eq!(content_text(&[]), "");
    assert_eq!(content_text(&[image()]), "");
}

#[test]
fn assistant_content_text_never_exposes_thinking_or_tool_calls() {
    let blocks = [
        AssistantContent::Thinking {
            thinking: "secret chain".to_string(),
            signature: Some("sig".to_string()),
        },
        AssistantContent::Text {
            text: "visible".to_string(),
        },
        AssistantContent::RedactedThinking {
            data: "opaque".to_string(),
        },
        AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "call_1".to_string(),
                name: "probe".to_string(),
                arguments: "{\"secret\": true}".to_string(),
                arguments_error: None,
            },
        },
        AssistantContent::Text {
            text: "tail".to_string(),
        },
    ];
    let rendered = assistant_content_text(&blocks);
    assert_eq!(rendered, "visible\ntail");
    assert!(!rendered.contains("secret"));
    assert!(!rendered.contains("opaque"));
    // All-invisible content renders nothing, not whitespace.
    let invisible = [AssistantContent::RedactedThinking {
        data: "x".to_string(),
    }];
    assert_eq!(assistant_content_text(&invisible), "");
}

#[test]
fn output_content_text_extracts_only_text_blocks() {
    let blocks = [
        OutputContent::Image {
            source: ImageSource::Base64 {
                data: "aGk=".to_string(),
            },
            media_type: MediaType::Jpeg,
        },
        OutputContent::Text {
            text: "result".to_string(),
        },
    ];
    assert_eq!(output_content_text(&blocks), "result");
    assert_eq!(output_content_text(&[]), "");
}

#[test]
fn system_message_text_renders_body_then_live_sections_in_insertion_order() {
    let message = SystemMessage {
        content: "base prompt".to_string(),
        sections: vec![
            SystemSection {
                name: "alpha".to_string(),
                value: Some("A".to_string()),
            },
            SystemSection {
                name: "removed".to_string(),
                value: None,
            },
            SystemSection {
                name: "empty".to_string(),
                value: Some(String::new()),
            },
            SystemSection {
                name: "beta".to_string(),
                value: Some("B".to_string()),
            },
        ],
        tools_added: Vec::new(),
        tools_removed: Vec::new(),
        timestamp_ms: 0,
    };
    // Null removes a section; an empty value keeps its identity in the
    // canonical data but contributes no rendered text.
    assert_eq!(message.sections.len(), 4);
    assert_eq!(system_message_text(&message), "base prompt\n\nA\n\nB");
}

#[test]
fn system_message_text_omits_empty_parts_entirely() {
    let empty_body = SystemMessage {
        content: String::new(),
        sections: vec![SystemSection {
            name: "only".to_string(),
            value: Some("S".to_string()),
        }],
        ..Default::default()
    };
    assert_eq!(system_message_text(&empty_body), "S");
    let all_empty = SystemMessage::default();
    assert_eq!(system_message_text(&all_empty), "");
}

#[test]
fn render_system_message_update_uses_exact_framing() {
    let message = SystemMessage {
        content: "late instruction".to_string(),
        sections: vec![
            SystemSection {
                name: "alpha".to_string(),
                value: Some("A2".to_string()),
            },
            SystemSection {
                name: "gone".to_string(),
                value: None,
            },
        ],
        ..Default::default()
    };
    assert_eq!(
        render_system_message_update(&message),
        "late instruction\n\nUpdated system prompt section \"alpha\":\n\nA2\n\nRemoved system prompt section \"gone\"."
    );
    // Empty body is omitted; an empty-string update keeps its framing (the
    // identity survives, the rendered body is empty).
    let no_body = SystemMessage {
        content: String::new(),
        sections: vec![SystemSection {
            name: "cleared".to_string(),
            value: Some(String::new()),
        }],
        ..Default::default()
    };
    assert_eq!(
        render_system_message_update(&no_body),
        "Updated system prompt section \"cleared\":\n\n"
    );
}

#[test]
fn replay_renders_chronology_initial_and_current_distinctly() {
    let messages = vec![
        system("base", &[("alpha", Some("A")), ("beta", Some("B"))]),
        system(
            "append",
            &[("alpha", Some("A2")), ("gone", None), ("new", Some("C"))],
        ),
    ];
    // Initial stays the leading entry's own state; current replays every
    // system message in order: content appends join with a blank line,
    // a replacement keeps its position, a null deletes, a new name appends.
    let initial = get_initial_system_message(&messages).expect("initial system");
    assert_eq!(system_message_text(initial), "base\n\nA\n\nB");
    let current = get_current_system_message(&messages).expect("current system");
    assert_eq!(current.content, "base\n\nappend");
    let names: Vec<&str> = current.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["alpha", "beta", "new"]);
    assert_eq!(
        get_current_system_prompt(&messages),
        "base\n\nappend\n\nA2\n\nB\n\nC"
    );
    assert_ne!(
        system_message_text(&current),
        system_message_text(initial),
        "initial/current distinction must survive replay"
    );
}

#[test]
fn replay_delete_then_readd_moves_the_section_to_the_end() {
    let messages = vec![
        system("", &[("alpha", Some("A")), ("beta", Some("B"))]),
        system("", &[("alpha", None)]),
        system("", &[("alpha", Some("A3"))]),
    ];
    let current = get_current_system_message(&messages).expect("current system");
    let names: Vec<&str> = current.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["beta", "alpha"]);
    assert_eq!(system_message_text(&current), "B\n\nA3");
    assert_eq!(get_current_system_prompt(&messages), "B\n\nA3");
}

#[test]
fn rendering_never_mutates_the_canonical_input() {
    let messages = vec![
        system("base", &[("alpha", Some("A"))]),
        system("update", &[("alpha", None)]),
        Message::User(opi_ai::message::UserMessage {
            content: vec![text("question"), image()],
            timestamp_ms: 1,
        }),
    ];
    let before = messages.clone();
    for message in &messages {
        if let Message::System(system) = message {
            let _ = system_message_text(system);
            let _ = render_system_message_update(system);
        }
        if let Message::User(user) = message {
            let _ = content_text(&user.content);
        }
        if let Message::Assistant(assistant) = message {
            let _ = assistant_content_text(&assistant.content);
        }
        if let Message::ToolResult(result) = message {
            let _ = output_content_text(&result.content);
        }
    }
    let _ = get_current_system_prompt(&messages);
    let _ = get_current_system_message(&messages);
    assert_eq!(messages, before, "renderers are read-only projections");
}

// --- task 20.53.2: JSON repair and portable schema construction ---

use opi_ai::message::{finalize_tool_arguments, parse_streaming_json, repair_json, string_enum};

#[test]
fn repair_json_escapes_control_characters_inside_strings() {
    // Raw control characters inside strings become valid JSON escapes;
    // outside strings nothing changes.
    assert_eq!(
        repair_json("{\"a\": \"x\u{0}y\"}"),
        "{\"a\": \"x\\u0000y\"}"
    );
    assert_eq!(repair_json("{\"a\": \"l\0m\"}"), "{\"a\": \"l\\u0000m\"}");
    // The five named escapes map to their short forms; a bare tab becomes
    // \t and other controls use \u00xx.
    assert_eq!(
        repair_json("{\"k\": \"a\tb\u{1}c\"}"),
        "{\"k\": \"a\\tb\\u0001c\"}"
    );
    // Control characters outside strings are left alone.
    assert_eq!(repair_json("\t{\"a\": 1}\n"), "\t{\"a\": 1}\n");
}

#[test]
fn repair_json_doubles_invalid_escapes_and_keeps_valid_ones() {
    let invalid = r#"{"a": "x\qy"}"#;
    assert_eq!(repair_json(invalid), r#"{"a": "x\\qy"}"#);
    // \u without four hex digits is not a valid unicode escape.
    assert_eq!(repair_json(r#"{"a": "\u12zz"}"#), r#"{"a": "\\u12zz"}"#);
    // Truncated \u keeps the same treatment.
    assert_eq!(repair_json(r#"{"a": "\u12"#), r#"{"a": "\\u12"#);
    // Valid escapes pass through untouched, including \uXXXX.
    let valid = r#"{"a": "q\"w\\e\/r\b f\n g\r h\t iA"}"#;
    assert_eq!(repair_json(valid), valid);
    assert_eq!(repair_json(r#"{"a": "é"}"#), r#"{"a": "é"}"#);
    // A trailing backslash inside a string stays literal.
    assert_eq!(repair_json(r#"{"a": "x\"#), r#"{"a": "x\\"#);
}

#[test]
fn parse_streaming_json_returns_empty_object_for_empty_input() {
    assert_eq!(parse_streaming_json(""), serde_json::json!({}));
    assert_eq!(parse_streaming_json("   "), serde_json::json!({}));
}

#[test]
fn parse_streaming_json_prefers_full_parse_with_repair() {
    // Cascade step 1: a complete document is authoritative, repaired when
    // only escapes differ - it wins before any partial completion runs.
    assert_eq!(
        parse_streaming_json(r#"{"a": 1}"#),
        serde_json::json!({"a": 1})
    );
    assert_eq!(
        parse_streaming_json(r#"{"a": "x\qy"}"#),
        serde_json::json!({"a": "x\\qy"})
    );
}

#[test]
fn parse_streaming_json_falls_through_partial_then_repaired_then_empty() {
    // Cascade step 2: partial completion of the raw text.
    assert_eq!(
        parse_streaming_json("{\"a\": \"hel"),
        serde_json::json!({"a": "hel"})
    );
    // Cascade step 3: raw partial cannot parse (invalid escape), the
    // repaired partial can.
    assert_eq!(
        parse_streaming_json(r#"{"a": "x\q"#),
        serde_json::json!({"a": "x\\q"})
    );
    // Cascade step 4: nothing works - display gets a valid empty object.
    assert_eq!(parse_streaming_json("{ ,"), serde_json::json!({}));
}

#[test]
fn partial_completion_fills_dangling_structures() {
    // Dangling key and dangling value complete to null.
    assert_eq!(
        parse_streaming_json("{\"a\""),
        serde_json::json!({"a": null})
    );
    assert_eq!(
        parse_streaming_json("{\"a\": "),
        serde_json::json!({"a": null})
    );
    // Trailing separators are dropped; containers close in order.
    assert_eq!(
        parse_streaming_json("{\"a\": 1,"),
        serde_json::json!({"a": 1})
    );
    assert_eq!(parse_streaming_json("[1, 2"), serde_json::json!([1, 2]));
    assert_eq!(
        parse_streaming_json("{\"a\": {\"b\": \"v"),
        serde_json::json!({"a": {"b": "v"}})
    );
    // Truncated numbers and literals complete conservatively.
    assert_eq!(
        parse_streaming_json("{\"a\": 1."),
        serde_json::json!({"a": 1})
    );
    assert_eq!(
        parse_streaming_json("{\"a\": tru"),
        serde_json::json!({"a": true})
    );
    // An escaped quote inside a partial string does not close the string.
    assert_eq!(
        parse_streaming_json("{\"a\": \"say \\\"hi"),
        serde_json::json!({"a": "say \"hi"})
    );
    // Junk after the first complete top-level value is ignored.
    assert_eq!(
        parse_streaming_json("{\"a\": 1}garbage"),
        serde_json::json!({"a": 1})
    );
    // A top-level bare value still parses for display.
    assert_eq!(parse_streaming_json("12"), serde_json::json!(12));
}

#[test]
fn finalize_tool_arguments_keeps_final_contract_public_and_authoritative() {
    // Empty and whitespace finalize to {} without an error.
    assert_eq!(
        finalize_tool_arguments(String::new()),
        ("{}".to_owned(), None)
    );
    assert_eq!(
        finalize_tool_arguments("  ".to_owned()),
        ("{}".to_owned(), None)
    );
    // A complete object keeps its owned raw text exactly.
    let raw = r#"{"b": 2, "a": 1}"#;
    assert_eq!(
        finalize_tool_arguments(raw.to_owned()),
        (raw.to_owned(), None)
    );
    // Input differing only by invalid escapes is repaired.
    assert_eq!(
        finalize_tool_arguments(r#"{"a": "x\qy"}"#.to_owned()),
        (r#"{"a": "x\\qy"}"#.to_owned(), None)
    );
    // Anything else keeps the raw text and carries a redacted-safe error
    // that never echoes the model-sent text.
    let (arguments, error) = finalize_tool_arguments("{\"a\": ".to_owned());
    assert_eq!(arguments, "{\"a\": ");
    let error = error.expect("incomplete final arguments error");
    assert!(error.contains("not a complete JSON object"));
    assert!(!error.contains("{\"a\": "));
    let (arguments, error) = finalize_tool_arguments("[1, 2]".to_owned());
    assert_eq!(arguments, "[1, 2]");
    assert!(error.is_some());
}

#[test]
fn final_tool_arguments_repairs_complete_objects_without_partial_completion() {
    for raw in [
        r#" {"a":"x\qy"}"#,
        "{\"a\":\"x\ny\"}",
        r#"{"a":"escaped\"quote","b":"x\qy"}"#,
    ] {
        let (arguments, error) = finalize_tool_arguments(raw.to_owned());
        assert!(
            error.is_none(),
            "complete repairable object failed: {error:?}"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&arguments).unwrap(),
            parse_streaming_json(raw)
        );
    }
    let raw = "{\"a\":\"x\ny\",\"unfinished\":";
    let (arguments, error) = finalize_tool_arguments(raw.to_owned());
    assert_eq!(
        arguments, raw,
        "incomplete final text remains available unchanged"
    );
    assert!(
        error.is_some(),
        "display repair cannot authorize a partial final object"
    );
}

#[test]
fn string_enum_emits_portable_schema() {
    let schema = string_enum(&["add", "subtract"], None, None);
    assert_eq!(
        schema,
        serde_json::json!({"type": "string", "enum": ["add", "subtract"]})
    );
    let described = string_enum(
        &["add", "subtract"],
        Some("The operation to perform"),
        Some("add"),
    );
    assert_eq!(
        described,
        serde_json::json!({
            "type": "string",
            "enum": ["add", "subtract"],
            "description": "The operation to perform",
            "default": "add"
        })
    );
    // Portable shape only: no anyOf, const or TypeBox symbols.
    for value in [&schema, &described] {
        let object = value.as_object().expect("object schema");
        assert!(object.len() <= 4);
        assert!(!object.contains_key("anyOf"));
        assert!(!object.contains_key("const"));
    }
}

// --- task 20.53.3: context-overflow / recoverable-Length predicates and
// --- typed provider diagnostics ---

use opi_ai::ApiKind;
use opi_ai::message::{AssistantMessage, is_context_overflow, is_recoverable_length};
use opi_ai::provider::{ProviderError, ProviderErrorSummary, extract_diagnostic_error};
use opi_ai::stream::{StopReason, Usage};

fn diagnostic_message(
    provider: &str,
    stop_reason: StopReason,
    error_message: Option<&str>,
    usage: Usage,
) -> AssistantMessage {
    AssistantMessage {
        content: vec![],
        api: ApiKind::Anthropic,
        provider: provider.to_owned(),
        model: "test-model".to_owned(),
        response_model: None,
        wire: None,
        response_id: None,
        usage,
        stop_reason,
        error_message: error_message.map(str::to_owned),
        timestamp_ms: 0,
        diagnostics: vec![],
    }
}

fn known_usage(input: u32, cache_read: u32, output: u32) -> Usage {
    Usage {
        input_tokens: input,
        cache_read_tokens: cache_read,
        output_tokens: output,
        reported: true,
        ..Usage::unknown()
    }
}

#[test]
fn context_overflow_matches_pinned_provider_error_messages() {
    let error_messages = [
        "prompt is too long: 213462 tokens > 200000 maximum",
        "Prompt too long",
        "Prompt exceeds max length",
        "413 {\"error\":{\"type\":\"request_too_large\",\"message\":\"Request exceeds the maximum size\"}}",
        "input is too long for requested model",
        "Your input exceeds the context window of this model",
        "Requested token count exceeds the model's maximum context length of 131072 tokens",
        "Input length (265330) exceeds maximum context length (262144).",
        "The input token count (1196265) exceeds the maximum number of tokens allowed (1048575)",
        "This model's maximum prompt length is 131072 but the request contains 537812 tokens",
        "Please reduce the length of the messages or completion",
        "This endpoint's maximum context length is 131072 tokens. However, you requested about 140000 tokens",
        "Input length 300000 exceeds the maximum allowed input length of 262144 tokens.",
        "The input (537812 tokens) is longer than the model's context length (131072 tokens).",
        "prompt token count of 537812 exceeds the limit of 131072",
        "the request exceeds the available context size, try increasing it",
        "tokens to keep from the initial prompt is greater than the context length",
        "invalid params, context window exceeds limit",
        "Your request exceeded model token limit: 131072 (requested: 537812)",
        "Prompt contains 537812 tokens ... too large for model with 131072 maximum context length",
        "Prompt has 537,812 tokens, but the configured context size is 131,072 tokens",
        "model_context_window_exceeded",
        "prompt too long; exceeded max context length by 8192 tokens",
        "Range of input length should be [1, 1048575]",
        "context_length_exceeded",
        "context length exceeded",
        "too many tokens",
        "token limit exceeded",
    ];
    for error_message in error_messages {
        let message = diagnostic_message(
            "openai",
            StopReason::Error,
            Some(error_message),
            Usage::unknown(),
        );
        assert!(
            is_context_overflow(&message, None),
            "expected overflow for {error_message:?}"
        );
    }
}

#[test]
fn context_overflow_excludes_non_overflow_patterns() {
    let excluded = [
        "Throttling error: Too many tokens, please wait before trying again.",
        "Service unavailable: too many tokens",
        "rate limit hit while checking too many tokens",
        "Too many requests: token limit exceeded",
    ];
    for error_message in excluded {
        let message = diagnostic_message(
            "bedrock",
            StopReason::Error,
            Some(error_message),
            Usage::unknown(),
        );
        assert!(
            !is_context_overflow(&message, None),
            "expected exclusion for {error_message:?}"
        );
    }
    // Unrelated failures and absent error text never classify as overflow.
    let unrelated = diagnostic_message(
        "openai",
        StopReason::Error,
        Some("internal hiccup"),
        Usage::unknown(),
    );
    assert!(!is_context_overflow(&unrelated, None));
    let silent = diagnostic_message("openai", StopReason::Error, None, Usage::unknown());
    assert!(!is_context_overflow(&silent, None));
}

#[test]
fn context_overflow_cerebras_bodyless_pattern_is_provider_gated() {
    let bodyless = diagnostic_message(
        "cerebras",
        StopReason::Error,
        Some("413 (no body)"),
        Usage::unknown(),
    );
    assert!(is_context_overflow(&bodyless, None));
    let bodyless_status = diagnostic_message(
        "cerebras",
        StopReason::Error,
        Some("400 status code (no body)"),
        Usage::unknown(),
    );
    assert!(is_context_overflow(&bodyless_status, None));
    let other_provider = diagnostic_message(
        "openai",
        StopReason::Error,
        Some("413 (no body)"),
        Usage::unknown(),
    );
    assert!(!is_context_overflow(&other_provider, None));
    let other_status = diagnostic_message(
        "cerebras",
        StopReason::Error,
        Some("500 (no body)"),
        Usage::unknown(),
    );
    assert!(!is_context_overflow(&other_status, None));
}

#[test]
fn context_overflow_silent_stop_branch_requires_known_usage_and_window() {
    let overflow = diagnostic_message("zai", StopReason::Stop, None, known_usage(213_462, 0, 10));
    assert!(is_context_overflow(&overflow, Some(200_000)));
    let with_cache = diagnostic_message(
        "zai",
        StopReason::Stop,
        None,
        known_usage(210_000, 4_000, 10),
    );
    assert!(is_context_overflow(&with_cache, Some(200_000)));
    // No window, zero window, under-window, unknown usage and non-stop
    // stop reasons never fire the silent branch.
    assert!(!is_context_overflow(&overflow, None));
    assert!(!is_context_overflow(&overflow, Some(0)));
    let under = diagnostic_message("zai", StopReason::Stop, None, known_usage(199_999, 0, 10));
    assert!(!is_context_overflow(&under, Some(200_000)));
    let unknown = diagnostic_message("zai", StopReason::Stop, None, Usage::unknown());
    assert!(!is_context_overflow(&unknown, Some(200_000)));
    let errored = diagnostic_message("zai", StopReason::Error, None, known_usage(213_462, 0, 0));
    assert!(!is_context_overflow(&errored, Some(200_000)));
}

#[test]
fn context_overflow_length_stop_branch_detects_filled_window() {
    let filled = diagnostic_message("mimo", StopReason::Length, None, known_usage(99_000, 0, 0));
    assert!(is_context_overflow(&filled, Some(100_000)));
    let exactly_99 = diagnostic_message(
        "mimo",
        StopReason::Length,
        None,
        known_usage(98_000, 1_000, 0),
    );
    assert!(is_context_overflow(&exactly_99, Some(100_000)));
    let below = diagnostic_message("mimo", StopReason::Length, None, known_usage(98_998, 0, 0));
    assert!(!is_context_overflow(&below, Some(100_000)));
    let with_output =
        diagnostic_message("mimo", StopReason::Length, None, known_usage(99_000, 0, 1));
    assert!(!is_context_overflow(&with_output, Some(100_000)));
    let unknown = diagnostic_message("mimo", StopReason::Length, None, Usage::unknown());
    assert!(!is_context_overflow(&unknown, Some(100_000)));
    assert!(!is_context_overflow(&filled, None));
    assert!(!is_context_overflow(&filled, Some(0)));
}

#[test]
fn is_recoverable_length_matches_source_comparison() {
    let truncated = diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 50));
    assert!(is_recoverable_length(&truncated, 100));
    let at_limit = diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 100));
    assert!(!is_recoverable_length(&at_limit, 100));
    let over_limit =
        diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 120));
    assert!(!is_recoverable_length(&over_limit, 100));
    assert!(!is_recoverable_length(&truncated, 0));
    let stopped = diagnostic_message("openai", StopReason::Stop, None, known_usage(100, 0, 50));
    assert!(!is_recoverable_length(&stopped, 100));
    // Unknown usage never converts to zero tokens.
    let unknown = diagnostic_message("openai", StopReason::Length, None, Usage::unknown());
    assert!(!is_recoverable_length(&unknown, 100));
}

#[test]
fn extract_diagnostic_error_preserves_service_fault_typed_codes() {
    let error = ProviderError::ServiceFault {
        status: 529,
        error_name: Some("overloaded_error".to_owned()),
        retry_after_ms: Some(1_000),
        detail: ProviderErrorSummary::redacted(),
    };
    let info = extract_diagnostic_error(&error);
    assert_eq!(info.name, "ServiceFault");
    assert_eq!(info.http_status, Some(529));
    assert_eq!(info.error_name.as_deref(), Some("overloaded_error"));
    assert_eq!(info.message, error.to_string());
    assert!(info.message.contains("529"));
}

#[test]
fn extract_diagnostic_error_stays_secret_safe() {
    let error = ProviderError::AuthFailed(ProviderErrorSummary::from_untrusted(
        "Bearer sk-super-secret-canary",
    ));
    let info = extract_diagnostic_error(&error);
    assert_eq!(info.name, "AuthFailed");
    assert_eq!(info.http_status, None);
    assert_eq!(info.error_name, None);
    assert_eq!(info.message, error.to_string());
    assert!(!info.message.contains("sk-super-secret-canary"));
}

#[test]
fn extract_diagnostic_error_covers_untyped_variants() {
    let timeout = extract_diagnostic_error(&ProviderError::Timeout);
    assert_eq!(timeout.name, "Timeout");
    assert_eq!(timeout.http_status, None);
    assert_eq!(timeout.error_name, None);
    assert_eq!(timeout.message, ProviderError::Timeout.to_string());
    let rate_limited = extract_diagnostic_error(&ProviderError::RateLimited {
        retry_after_ms: Some(500),
    });
    assert_eq!(rate_limited.name, "RateLimited");
    assert_eq!(
        rate_limited.message,
        ProviderError::RateLimited {
            retry_after_ms: Some(500)
        }
        .to_string()
    );
    let cancelled = extract_diagnostic_error(&ProviderError::Cancelled);
    assert_eq!(cancelled.name, "Cancelled");
}

// --- task 20.53.4: compatibility shortHash and UUIDv7 public utilities ---

use opi_ai::identity::{UuidV7Error, UuidV7Generator, short_hash};
use opi_ai::test_support;

fn uuid_v7_bytes(id: &str) -> [u8; 16] {
    let hex: String = id.chars().filter(|c| *c != '-').collect();
    let mut bytes = [0u8; 16];
    for (slot, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    bytes
}

fn uuid_v7_timestamp_ms(id: &str) -> u64 {
    let bytes = uuid_v7_bytes(id);
    u64::from(bytes[0]) << 40
        | u64::from(bytes[1]) << 32
        | u64::from(bytes[2]) << 24
        | u64::from(bytes[3]) << 16
        | u64::from(bytes[4]) << 8
        | u64::from(bytes[5])
}

fn uuid_v7_sequence(id: &str) -> u64 {
    let b = uuid_v7_bytes(id);
    (u64::from(b[6]) & 0x0f) << 37
        | u64::from(b[7]) << 29
        | (u64::from(b[8]) & 0x3f) << 23
        | u64::from(b[9]) << 15
        | u64::from(b[10]) << 7
        | u64::from(b[11]) >> 1
}

#[test]
fn short_hash_matches_pinned_source_fixtures() {
    // Expected values computed from the pinned JavaScript algorithm with
    // strict ToInt32/ToUint32 discipline (UTF-16 code units, Math.imul 32-bit
    // wrapping, self>>>16 vs cross>>>13 final mix, h2-then-h1 base36).
    let fixtures = [
        ("", "k4n83c7h0j2b"),
        ("a", "m8735310ae7sx"),
        ("hello", "1h6qa0qrowduu"),
        ("pi\u{2248}3.14", "1nlj7n2m2u17f"),
        ("\u{1F600}emoji", "a5yrnht4g44f"),
        ("0123456789abcdef0123456789abcdef", "1gcesuz1crg44z"),
    ];
    for (input, expected) in fixtures {
        assert_eq!(short_hash(input), expected, "mismatch for {input:?}");
    }
}

#[test]
fn short_hash_is_deterministic_and_distinct() {
    assert_eq!(short_hash("abc"), short_hash("abc"));
    assert_ne!(short_hash("abc"), short_hash("abd"));
}

#[test]
fn uuidv7_preserves_explicit_timestamp_and_layout() {
    let mut generator = UuidV7Generator::new();
    let id = generator.generate_at(1_700_000_000_123).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&id), 1_700_000_000_123);
    let bytes = uuid_v7_bytes(&id);
    assert_eq!(bytes[6] >> 4, 0x7, "version nibble");
    assert_eq!(bytes[8] >> 6, 0b10, "variant bits");
    assert!(bytes[12..16].iter().any(|byte| *byte != 0), "random tail");
}

#[test]
fn uuidv7_rejects_out_of_range_timestamps() {
    let mut generator = UuidV7Generator::new();
    let max = 0xffff_ffff_ffff;
    assert!(generator.generate_at(max).is_ok());
    assert_eq!(
        generator.generate_at(max + 1).unwrap_err(),
        UuidV7Error::InvalidTimestamp
    );
    assert_eq!(
        generator.generate(max + 1).unwrap_err(),
        UuidV7Error::InvalidTimestamp
    );
}

#[test]
fn uuidv7_ordinary_generation_never_regresses_on_rollback() {
    let mut generator = UuidV7Generator::new();
    let first = generator.generate(5_000).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&first), 5_000);
    let rolled_back = generator.generate(4_000).unwrap();
    assert_eq!(
        uuid_v7_timestamp_ms(&rolled_back),
        5_000,
        "clock rollback must not regress the timestamp"
    );
    let advanced = generator.generate(6_000).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&advanced), 6_000);
}

#[test]
fn uuidv7_explicit_path_does_not_advance_the_rollback_floor() {
    let mut generator = UuidV7Generator::new();
    generator.generate(5_000).unwrap();
    generator.generate_at(9_000).unwrap();
    let after = generator.generate(4_500).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&after), 5_000);
}

#[test]
fn uuidv7_sequence_increments_within_a_generator() {
    let mut generator = UuidV7Generator::new();
    let first = generator.generate_at(1_000).unwrap();
    let second = generator.generate_at(1_000).unwrap();
    assert_eq!(uuid_v7_sequence(&second), uuid_v7_sequence(&first) + 1);
    let ordinary = generator.generate(2_000).unwrap();
    assert_eq!(uuid_v7_sequence(&ordinary), uuid_v7_sequence(&second) + 1);
}

#[test]
fn uuidv7_sequence_exhaustion_is_a_typed_failure() {
    let mut generator = UuidV7Generator::new();
    test_support::force_sequence_exhausted(&mut generator);
    assert_eq!(
        generator.generate(5_000).unwrap_err(),
        UuidV7Error::SequenceExhausted
    );
    assert_eq!(
        generator.generate_at(6_000).unwrap_err(),
        UuidV7Error::SequenceExhausted
    );
}

#[test]
fn uuidv7_generates_unique_well_formed_ids() {
    let mut generator = UuidV7Generator::new();
    let mut seen = std::collections::HashSet::new();
    for index in 0..16u64 {
        let id = generator.generate(10_000 + index).unwrap();
        assert_eq!(id.len(), 36);
        for (slot, dash) in [8usize, 13, 18, 23].iter().enumerate() {
            let _ = slot;
            assert_eq!(id.as_bytes()[*dash], b'-');
        }
        assert!(id.bytes().all(|byte| {
            byte == b'-' || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        }));
        assert!(seen.insert(id), "duplicate id generated");
    }
    for timestamp in [11_000u64, 11_001] {
        let id = generator.generate_at(timestamp).unwrap();
        assert!(seen.insert(id), "duplicate id across paths");
    }
}

// --- task 20.53.6: header merge, scoped provider env, cancellable waits ---

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::cancel::{Cancelled, operation_token, race_with_cancel, sleep_cancel};
use opi_ai::provider_env::provider_env_value;
use opi_ai::provider_headers::merge_provider_header_sources;
use tokio_util::sync::CancellationToken;

/// Serializes tests that mutate the process environment.
static ENV_LOCK_536: Mutex<()> = Mutex::new(());

fn headers(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.map(|value| value.to_owned())))
        .collect()
}

#[test]
fn merge_provider_header_sources_merges_case_insensitively_last_wins_in_reinsert_order() {
    let first = headers(&[("X-Beta", Some("one")), ("Keep-Alive", Some("0"))]);
    let second = headers(&[("x-beta", Some("two"))]);
    let merged = merge_provider_header_sources(&[&first, &second]).unwrap();
    assert_eq!(
        merged,
        vec![
            ("Keep-Alive".to_owned(), "0".to_owned()),
            // The override re-inserts at the end and keeps the winner's case.
            ("x-beta".to_owned(), "two".to_owned()),
        ]
    );
}

#[test]
fn merge_provider_header_sources_none_deletes_and_empty_yields_none() {
    let base = headers(&[("A", Some("1")), ("B", Some("2"))]);
    let delete = headers(&[("a", None)]);
    let merged = merge_provider_header_sources(&[&base, &delete]).unwrap();
    assert_eq!(merged, vec![("B".to_owned(), "2".to_owned())]);

    let delete_all = headers(&[("b", None)]);
    assert_eq!(
        merge_provider_header_sources(&[&base, &delete, &delete_all]),
        None
    );
    // A deletion for a name that is not present adds nothing.
    let only_delete = headers(&[("absent", None)]);
    assert_eq!(merge_provider_header_sources(&[&only_delete]), None);
    assert_eq!(merge_provider_header_sources(&[]), None);
}

#[test]
fn merge_provider_header_sources_preserves_winner_within_and_across_sources() {
    let single = headers(&[("X-Api-Version", Some("v1")), ("x-api-version", Some("v2"))]);
    assert_eq!(
        merge_provider_header_sources(&[&single]).unwrap(),
        vec![("x-api-version".to_owned(), "v2".to_owned())]
    );
    let literal = headers(&[("X-Api-Version", Some("v1"))]);
    assert_eq!(
        merge_provider_header_sources(&[&literal]).unwrap(),
        vec![("X-Api-Version".to_owned(), "v1".to_owned())]
    );
}

#[test]
fn provider_env_scoped_precedes_ambient() {
    let _guard = ENV_LOCK_536.lock().unwrap();
    let name = "OPI_TEST_PROVIDER_ENV_536_A";
    // Safety: single-threaded under ENV_LOCK_536; unique test name.
    unsafe { std::env::set_var(name, "ambient") };
    let mut scoped = HashMap::new();
    scoped.insert(name.to_owned(), "scoped".to_owned());
    assert_eq!(
        provider_env_value(name, Some(&scoped)),
        Some("scoped".to_owned())
    );
    assert_eq!(provider_env_value(name, None), Some("ambient".to_owned()));
    // Safety: see above.
    unsafe { std::env::remove_var(name) };
    assert_eq!(provider_env_value(name, None), None);
}

#[test]
fn provider_env_empty_scoped_falls_back_to_ambient() {
    let _guard = ENV_LOCK_536.lock().unwrap();
    let name = "OPI_TEST_PROVIDER_ENV_536_B";
    // Safety: single-threaded under ENV_LOCK_536; unique test name.
    unsafe { std::env::set_var(name, "ambient") };
    let mut scoped = HashMap::new();
    scoped.insert(name.to_owned(), String::new());
    // Faithful to the pinned `env?.[name] || process.env[name]`: an empty
    // scoped value is falsy and falls back to ambient.
    assert_eq!(
        provider_env_value(name, Some(&scoped)),
        Some("ambient".to_owned())
    );
    // Safety: see above.
    unsafe { std::env::set_var(name, "") };
    // The pinned || chain also treats an empty ambient value as unset.
    assert_eq!(provider_env_value(name, Some(&scoped)), None);
    // Safety: see above.
    unsafe { std::env::remove_var(name) };
    assert_eq!(provider_env_value(name, Some(&scoped)), None);
}

#[tokio::test(start_paused = true)]
async fn sleep_cancel_resolves_after_the_requested_duration() {
    let token = CancellationToken::new();
    let start = tokio::time::Instant::now();
    sleep_cancel(120, &token).await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(120));
}

#[tokio::test(start_paused = true)]
async fn sleep_cancel_fails_immediately_when_pre_cancelled() {
    let token = CancellationToken::new();
    token.cancel();
    let start = tokio::time::Instant::now();
    assert_eq!(sleep_cancel(60_000, &token).await, Err(Cancelled));
    assert!(start.elapsed() < Duration::from_millis(1));
}

#[tokio::test(start_paused = true)]
async fn sleep_cancel_fails_when_cancelled_mid_wait() {
    let token = CancellationToken::new();
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(40)).await;
        cancel_at.cancel();
    });
    let start = tokio::time::Instant::now();
    assert_eq!(sleep_cancel(60_000, &token).await, Err(Cancelled));
    assert!(start.elapsed() < Duration::from_millis(60_000));
}

#[tokio::test(start_paused = true)]
async fn race_with_cancel_returns_output_when_ready_before_cancel() {
    let token = CancellationToken::new();
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel_at.cancel();
    });
    assert_eq!(race_with_cancel(async { 42u32 }, &token).await, Ok(42));
}

#[tokio::test(start_paused = true)]
async fn race_with_cancel_releases_the_operation_future_on_cancel() {
    struct Sentinel(Arc<AtomicBool>);
    impl Drop for Sentinel {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    let token = CancellationToken::new();
    let dropped = Arc::new(AtomicBool::new(false));
    let sentinel = Sentinel(dropped.clone());
    let operation = async move {
        let _keep = sentinel;
        tokio::time::sleep(Duration::from_secs(1_000)).await;
        0u32
    };
    let cancel_at = token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_at.cancel();
    });
    assert_eq!(race_with_cancel(operation, &token).await, Err(Cancelled));
    assert!(
        dropped.load(Ordering::SeqCst),
        "the abandoned operation future must be dropped on cancel"
    );
}

#[tokio::test]
async fn operation_token_links_parent_to_child_one_way() {
    let parent = CancellationToken::new();
    let child = operation_token(Some(&parent));
    child.cancel();
    assert!(child.is_cancelled());
    assert!(
        !parent.is_cancelled(),
        "child cancellation must not propagate to the parent"
    );

    let parent_two = CancellationToken::new();
    let child_two = operation_token(Some(&parent_two));
    parent_two.cancel();
    assert!(
        child_two.is_cancelled(),
        "parent cancellation must reach the child"
    );

    let standalone = operation_token(None);
    standalone.cancel();
    // parent_two was cancelled on purpose above; only parent must stay clean.
    assert!(!parent.is_cancelled());
}
