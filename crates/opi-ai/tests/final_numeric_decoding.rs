//! The public final text is decoded by downstream callers, including sessions.
use opi_ai::message::finalize_tool_arguments;
use opi_ai::streaming_json::ToolCallArgumentsStream;
use serde_json::{Value, json};

#[test]
fn final_numbers_survive_downstream_decoding_and_roundtrip() {
    let literals = [
        format!("1{}", "0".repeat(64)),
        "1e64".into(),
        "-1e64".into(),
        "1.2345678901234567".into(),
        "2.2250738585072014e-308".into(),
        "2.4703282292062327e-324".into(),
        "2.4703282292062328e-324".into(),
        "1.7976931348623157e308".into(),
        "-1e-400".into(),
        "18446744073709551616".into(),
    ];
    let mut rows = Vec::new();
    for literal in literals {
        let expected = literal.parse::<f64>().unwrap().to_bits();
        let raw = format!(r#"{{"n":{literal}}}"#);
        let mut stream = ToolCallArgumentsStream::new();
        stream.append(&raw);
        let (final_text, error) = stream.finish();
        assert_eq!(final_text, raw);
        assert_eq!(error, None);
        let value: Value = serde_json::from_str(&final_text).unwrap();
        let bits = value["n"].as_f64().unwrap().to_bits();
        assert_eq!(bits, expected, "final decoding of {literal}");
        let serialized = serde_json::to_string(&value).unwrap();
        let restored: Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored["n"].as_f64().unwrap().to_bits(), expected);
        rows.push(json!({"raw":raw,"bits":format!("{bits:016x}"),"serialized":serialized}));
    }
    println!("\n{}", json!({"final_numbers":rows}));
}

#[test]
fn strict_final_rejection_and_exact_rust_integers_are_preserved() {
    for raw in [r#"{"n":1e400}"#, r#"{"n":1e400,"n":2}"#, r#"{"n":01}"#] {
        let (text, error) = finalize_tool_arguments(raw.to_owned());
        assert_eq!(text, raw);
        assert!(error.is_some());
    }
    let raw = r#"{"safe":9007199254740993,"max":18446744073709551615,"min":-9223372036854775808}"#;
    let (text, error) = finalize_tool_arguments(raw.to_owned());
    assert_eq!(error, None);
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["safe"].as_u64(), Some(9_007_199_254_740_993));
    assert_eq!(value["max"].as_u64(), Some(u64::MAX));
    assert_eq!(value["min"].as_i64(), Some(i64::MIN));
    assert_eq!(serde_json::to_string(&value).unwrap(), raw);
}
