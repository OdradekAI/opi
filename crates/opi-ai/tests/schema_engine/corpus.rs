//! The retained schema differential corpus and its approved outcomes.

use opi_ai::schema::compile;
use serde_json::Value;

#[test]
fn frozen_differential_corpus_matches_expected_outcomes() {
    let cases_text = include_str!("../data/schema-cases.jsonl");
    let expected: serde_json::Map<String, Value> =
        serde_json::from_str(include_str!("../data/schema-expected.json")).unwrap();

    let mut matched = 0;
    for line in cases_text.lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        let id = case["id"].as_str().unwrap();
        let expectation = &expected[id];

        let outcome = match compile(case["schema"].clone()) {
            Err(_) => "mechanism-error".to_string(),
            Ok(compiled) => match compiled.check(&case["arguments"]) {
                Ok(true) => "evaluated-true".to_string(),
                Ok(false) => "evaluated-false".to_string(),
                Err(_) => "mechanism-error".to_string(),
            },
        };
        let expected_outcome = match expectation["outcome"].as_str().unwrap() {
            "mechanism-error" => "mechanism-error".to_string(),
            _ => format!(
                "evaluated-{}",
                expectation["valid"].as_bool().unwrap_or(false)
            ),
        };

        assert_eq!(outcome, expected_outcome, "case {id}");
        matched += 1;
    }
    assert_eq!(matched, 64, "the complete frozen corpus must be asserted");
}
