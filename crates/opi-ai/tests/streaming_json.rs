use opi_ai::streaming_json::ToolCallArgumentsStream;
use serde_json::json;

#[test]
fn missing_values_duplicates_and_escapes_keep_the_received_value() {
    let mut args = ToolCallArgumentsStream::new();
    for (delta, expected) in [
        (r#"{"a":1,"a""#, json!({"a":1.0})),
        (":", json!({"a":1.0})),
        ("tru", json!({"a":true})),
        (r#"e,"s":"x\u00"#, json!({"a":true,"s":"x"})),
        (r#"e9\n "#, json!({"a":true,"s":"xé\n "})),
        (r#"\q"}"#, json!({"a":true,"s":"xé\n \\q"})),
    ] {
        args.append(delta);
        assert_eq!(args.snapshot(), expected);
    }
    assert!(args.finish().1.is_none());
}

#[test]
fn numbers_literals_and_non_object_roots_are_display_only() {
    let mut args = ToolCallArgumentsStream::new();
    for (delta, expected) in [
        (r#"{"n":1."#, json!({})),
        ("2e+", json!({"n":1.2})),
        ("2,\"a\":[f", json!({"n":120.0,"a":[false]})),
        ("alse,nu", json!({"n":120.0,"a":[false,null]})),
    ] {
        args.append(delta);
        assert_eq!(args.snapshot(), expected);
    }
    assert!(args.finish().1.is_some());
    args.append("ll]}");
    assert!(args.finish().1.is_none());
    for raw in ["[true]", "42", "null", "\"text\""] {
        assert_eq!(args.replace(raw), None);
        assert_eq!(args.snapshot(), json!({}));
        assert!(args.finish().1.is_some());
    }
}

#[test]
fn replacement_and_snapshots_have_independent_ownership() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"s":"a"#);
    let mut early = args.snapshot();
    assert_eq!(args.replace(r#"{"s":"a"#), Some(""));
    assert_eq!(args.replace(r#"{"s":"ab"}"#), Some(r#"b"}"#));
    let complete = args.snapshot();
    early["s"] = json!("caller edit");
    assert_eq!(args.snapshot(), json!({"s":"ab"}));
    assert_eq!(args.replace(r#"{"other":false}"#), None);
    assert_eq!(args.snapshot(), json!({"other":false}));
    drop(args);
    assert_eq!(early, json!({"s":"caller edit"}));
    assert_eq!(complete, json!({"s":"ab"}));
}

#[test]
fn invalid_final_does_not_become_valid_because_preview_succeeded() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"enabled":true"#);
    assert_eq!(args.snapshot(), json!({"enabled":true}));
    let (raw, error) = args.finish();
    assert_eq!(raw, args.text());
    assert!(error.is_some());
    args.append("}");
    assert!(args.finish().1.is_none());
    args.append(" trailing");
    assert_eq!(args.snapshot(), json!({"enabled":true}));
    assert!(args.finish().1.is_some());
}

#[test]
fn unpaired_surrogates_hold_the_last_representable_snapshot() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"s":"x"#);
    let before = args.snapshot();
    args.append(r#"\ud83d"#);
    assert_eq!(args.snapshot(), before);
    args.append(r#"\ude00!"}"#);
    assert_eq!(args.snapshot(), json!({"s":"x😀!"}));
    assert!(args.finish().1.is_none());
    for raw in [r#"{"s":"\ud800"}"#, r#"{"\udc00":1}"#] {
        args.replace(raw);
        assert_eq!(args.snapshot(), json!({}));
        assert!(args.finish().1.is_some());
    }
}

#[test]
fn interleaved_tools_do_not_share_parser_or_final_state() {
    let mut tools = [
        ToolCallArgumentsStream::new(),
        ToolCallArgumentsStream::new(),
    ];
    tools[0].append(r#"{"a":"left"#);
    tools[1].append(r#"{"b":false"#);
    let first = tools[0].snapshot();
    assert_eq!(tools[1].snapshot(), json!({"b":false}));
    tools[1].append("}");
    tools[0].append(r#" right"}"#);
    assert_eq!(first, json!({"a":"left"}));
    assert_eq!(tools[0].snapshot(), json!({"a":"left right"}));
    assert!(tools.iter().all(|tool| tool.finish().1.is_none()));
}

#[test]
fn open_container_credit_empty_appends_and_repeated_reads() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"a":["#);
    let mut first_lag = None;
    for index in 0..1200 {
        args.append(if index == 0 { "\"x\"" } else { ",\"x\"" });
        let snapshot = args.snapshot();
        if snapshot["a"].as_array().unwrap().len() != index + 1 {
            first_lag = Some(index + 1);
            assert_eq!(args.snapshot(), snapshot);
            args.append("");
            assert!(
                args.snapshot()["a"].as_array().unwrap().len()
                    >= snapshot["a"].as_array().unwrap().len()
            );
            break;
        }
    }
    assert!(
        first_lag.is_some(),
        "large open containers should defer copying"
    );
    args.append("]}");
    assert_eq!(
        args.snapshot()["a"].as_array().unwrap().len(),
        first_lag.unwrap()
    );
    assert!(args.finish().1.is_none());
}

#[test]
fn keys_keep_first_position_and_javascript_index_order() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"b":1,"a":2"#);
    let keys = |value: serde_json::Value| {
        value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(keys(args.snapshot()), ["b", "a"]);
    args.append(
        r#", "a":3,"10":true,"2":true,"01":true,"4294967295":true,"4294967294":true,"0":true"#,
    );
    let expected = ["0", "2", "10", "4294967294", "b", "a", "01", "4294967295"];
    assert_eq!(keys(args.snapshot()), expected);
    args.append("}");
    assert_eq!(keys(args.snapshot()), expected);
}

#[test]
fn non_finite_numbers_hold_the_whole_snapshot_and_final_validation_stays_strict() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"a":[7,1e"#);
    let before = args.snapshot();
    assert_eq!(before, json!({"a":[7.0,1.0]}));
    args.append("400,false]}");
    assert_eq!(args.snapshot(), before);
    assert_eq!(args.snapshot(), before);
    assert_eq!(args.text(), r#"{"a":[7,1e400,false]}"#);
    assert!(args.finish().1.is_some());
    args.replace(r#"{"n":1,"n":1e400,"a":[-1e400,true],"z":-1e-400}"#);
    assert_eq!(args.snapshot(), json!({}));
    args.replace(r#"{"n":1,"n":1e400,"a":[-1e400,true],"n":false,"a":[],"z":-1e-400}"#);
    assert_eq!(args.snapshot(), json!({"n":false,"a":[],"z":-0.0}));
    assert!(args.snapshot()["z"].as_f64().unwrap().is_sign_negative());
    assert!(
        args.finish().1.is_some(),
        "earlier overflow still fails strict parsing"
    );
    args.replace(r#"{"a":[7,8,false]}"#);
    assert_eq!(args.snapshot(), json!({"a":[7.0,8.0,false]}));
    assert!(args.finish().1.is_none());
}

#[test]
fn non_finite_slots_recover_without_losing_reachable_siblings() {
    let mut args = ToolCallArgumentsStream::new();
    args.append(r#"{"old":true,"n":2,"#);
    let before = args.snapshot();
    args.append(r#""n":1e400,"later":true,"#);
    assert_eq!(args.snapshot(), before);
    args.append(r#""n":f"#);
    assert_eq!(args.snapshot(), json!({"old":true,"n":false,"later":true}));
    args.append("alse}");
    assert_eq!(args.snapshot(), json!({"old":true,"n":false,"later":true}));
    assert_eq!(before, json!({"old":true,"n":2.0}));
    assert!(args.finish().1.is_some());
    // Non-finite tokens with no value slot are not reachable preview values.
    args.replace(r#"{1e400,"ok":true}"#);
    assert_eq!(args.snapshot(), json!({"ok":true}));
}

#[test]
fn overflow_is_distinct_from_invalid_number_grammar_and_underflow() {
    let mut args = ToolCallArgumentsStream::new();
    for token in ["NaN", "Infinity", "+1e400", "01e400", "1.e400", "1e400x"] {
        args.replace(&format!("{{\"n\":{token},\"ok\":true}}"));
        assert_eq!(args.snapshot(), json!({"ok":true}), "{token}");
        assert!(args.finish().1.is_some());
    }
    for token in [
        "1e400".to_owned(),
        "-1e400".to_owned(),
        "1e999999999999999999999".to_owned(),
        "9".repeat(400),
    ] {
        args.replace(&format!("{{\"n\":{token},\"ok\":true}}"));
        assert_eq!(args.snapshot(), json!({}), "{token}");
        assert!(args.finish().1.is_some());
    }
    args.replace(r#"{"n":1.7976931348623157e308,"z":-1e-999999999999999999999}"#);
    assert_eq!(args.snapshot()["n"].as_f64(), Some(f64::MAX));
    assert!(args.snapshot()["z"].as_f64().unwrap().is_sign_negative());
}

#[test]
fn deep_inputs_bound_owned_snapshot_depth_in_a_subprocess() {
    const CHILD: &str = "OPI_PREVIEW_DEPTH_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "deep_inputs_bound_owned_snapshot_depth_in_a_subprocess",
            ])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let mut args = ToolCallArgumentsStream::new();
    for count in [127, 128] {
        args.replace(&format!(
            "{}0{}",
            "{\"a\":".repeat(count),
            "}".repeat(count)
        ));
        let snapshot = args.snapshot();
        let mut depth = 0;
        let mut cursor = &snapshot;
        while let Some(value) = cursor.get("a") {
            depth += 1;
            cursor = value;
        }
        assert_eq!(depth, count);
    }
    for depth in [129, 10_000] {
        args.replace(r#"{"old":true,"deep":"#);
        let before = args.snapshot();
        args.append(&"[".repeat(depth));
        assert_eq!(args.snapshot(), before);
        args.append(&format!("0{}", "]".repeat(depth)));
        assert_eq!(args.snapshot(), before);
        assert_eq!(args.snapshot(), before);
        args.append("}");
        assert_eq!(args.snapshot(), before);
        assert!(args.finish().1.is_some());
        args.replace(&format!(
            "{{\"deep\":{}0{},\"deep\":true}}",
            "[".repeat(depth),
            "]".repeat(depth)
        ));
        assert_eq!(args.snapshot(), json!({"deep":true}));
        args.replace(r#"{"shallow":true}"#);
        assert_eq!(args.snapshot(), json!({"shallow":true}));
    }
    drop(args);
}
