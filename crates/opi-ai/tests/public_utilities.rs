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
