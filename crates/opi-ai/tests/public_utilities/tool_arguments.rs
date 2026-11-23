//! JSON repair, argument previews and finalization, and portable enum schemas.

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
fn streaming_preview_keeps_complete_trailing_literals() {
    // A delimiter may arrive in a later chunk. A complete literal must keep
    // its value both before that chunk and after the container closes.
    for literal in ["true", "false", "null"] {
        let value: serde_json::Value = serde_json::from_str(literal).unwrap();
        for (prefix, suffix, expected) in [
            (
                format!("{{\"a\":{literal}"),
                "}",
                serde_json::json!({"a": value}),
            ),
            (format!("[1,{literal}"), "]", serde_json::json!([1, value])),
            (
                format!("{{\"a\":[{literal}"),
                "]}",
                serde_json::json!({"a": [value]}),
            ),
        ] {
            assert_eq!(parse_streaming_json(&prefix), expected, "{prefix}");
            assert_eq!(parse_streaming_json(&(prefix + suffix)), expected);
        }
    }
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
