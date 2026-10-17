//! Public tool-argument normalizer/validator battery (task 20.10).
//!
//! The differential corpus is the frozen research evidence
//! `docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema/results/original/pi.jsonl`
//! (sha256 `f310f81c81869c9fe2a65a3267882b414decd742724da6fcb623d2df760d7f01`,
//! pinned in that directory's artifact set): the pinned Pi 1.3.0 public
//! `validateToolArguments` utility ran over the readiness corpus and its
//! recorded `normalized` outputs are the expectations here — the conversions
//! are fixed by that JS corpus, not guessed from Rust parsing.
//!
//! One recorded case asserts the mandated RDF-09 correctness fix instead of
//! the recorded output: `root-correctness` records Pi returning the raw
//! unvalidated value when a coerced root primitive fails the final check;
//! the adopted design requires the final original-schema validation to
//! reject it.

use std::path::Path;

use opi_ai::schema::{ToolArgumentOutcome, compile, validate_tool_arguments, validate_tool_call};
use serde_json::Value;

fn rows_by_id(id: &str) -> Option<Value> {
    let data = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/schema-public-expected.json"),
    )
    .expect("schema-public-expected.json is committed");
    let rows: Vec<Value> = serde_json::from_str(&data).expect("battery parses");
    rows.into_iter().find(|row| row["id"].as_str() == Some(id))
}

fn cases() -> Vec<(String, Value, Value, Value, String)> {
    // (id, schema, arguments, pi_normalized, expected)
    let data = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/schema-public-expected.json"),
    )
    .expect("schema-public-expected.json is committed");
    let rows: Vec<Value> = serde_json::from_str(&data).expect("battery parses");
    rows.into_iter()
        .map(|row| {
            (
                row["id"].as_str().expect("id").to_owned(),
                row["schema"].clone(),
                row["arguments"].clone(),
                row["pi_normalized"].clone(),
                row["expected"].as_str().expect("expected").to_owned(),
            )
        })
        .collect()
}

#[test]
fn frozen_public_pi_corpus_matches_recorded_normalized_outputs() {
    for (id, schema, arguments, pi_normalized, expected) in cases() {
        let compiled = compile(schema.clone()).expect("schema admits");
        let outcome = validate_tool_arguments(&compiled, &arguments, None)
            .unwrap_or_else(|err| panic!("{id}: engine error {err}"));
        match expected.as_str() {
            "valid" => match outcome {
                ToolArgumentOutcome::Valid { normalized } => {
                    assert_eq!(
                        normalized, pi_normalized,
                        "{id}: normalized value diverges from the recorded Pi output"
                    );
                }
                other => panic!("{id}: expected Valid, got {other:?}"),
            },
            // RDF-09: the recorded Pi output reflects JavaScript JSON.parse
            // collapsing 2^53+1 to 2^53; the port keeps the precise JSON
            // integer instead of replicating the parse-time loss.
            "valid-exact-integer" => match outcome {
                ToolArgumentOutcome::Valid { normalized } => {
                    let row = rows_by_id(&id).expect("row present");
                    assert_eq!(normalized, row["opi_normalized"], "{id}: exact integer");
                }
                other => panic!("{id}: expected Valid, got {other:?}"),
            },
            "invalid" => match outcome {
                ToolArgumentOutcome::Invalid { failures } => {
                    assert!(!failures.is_empty(), "{id}: failures must be present");
                }
                other => panic!("{id}: expected Invalid, got {other:?}"),
            },
            other => panic!("{id}: unknown expectation {other}"),
        }
    }
}

#[test]
fn root_correctness_fix_rejects_unvalidated_raw() {
    // RDF-09: Pi returns the raw string "2" (which fails {"type":"integer",
    // "minimum":10}); the port must run the final original-schema check on
    // the coerced value and report Invalid.
    let (row, _) = cases()
        .into_iter()
        .partition::<Vec<_>, _>(|c| c.0 == "root-correctness");
    let (_, schema, arguments, pi_normalized, _) = row.into_iter().next().expect("case present");
    assert_eq!(pi_normalized, Value::String("2".to_owned())); // the recorded bug
    let compiled = compile(schema).expect("admits");
    let outcome = validate_tool_arguments(&compiled, &arguments, None).unwrap();
    match outcome {
        ToolArgumentOutcome::Invalid { failures } => {
            assert_eq!(failures[0].keyword, "minimum");
            assert_eq!(failures[0].instance_path, "");
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn arguments_error_rejects_before_coercion() {
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "type": "integer" } },
        "required": ["n"]
    }))
    .unwrap();
    let outcome = validate_tool_arguments(
        &compiled,
        &serde_json::json!({"n": "0x10"}),
        Some("malformed"),
    )
    .unwrap();
    assert_eq!(outcome, ToolArgumentOutcome::ArgumentsError);
}

#[test]
fn raw_input_is_never_mutated() {
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "s": { "type": "string" } },
        "required": ["s"]
    }))
    .unwrap();
    let raw = serde_json::json!({"s": 12});
    let before = raw.clone();
    let outcome = validate_tool_arguments(&compiled, &raw, None).unwrap();
    assert_eq!(raw, before, "clone-before-normalize keeps the raw value");
    match outcome {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"s": "12"}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

#[test]
fn optional_null_deletion_and_required_null_coercion() {
    // Optional null whose sub-schema rejects null is deleted.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "s": { "type": "string" } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"s": null}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
    // Required null stays and coerces per the declared type.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "type": "integer" } },
        "required": ["n"]
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"n": null}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"n": 0}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

#[test]
fn optional_null_kept_when_subschema_accepts_null_or_fails_to_compile() {
    // Sub-schema accepts null: key survives normalization.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "s": { "type": ["string", "null"] } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"s": null}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"s": null}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
    // $ref properties are skipped by optional-null deletion (upstream guard).
    let compiled = compile(serde_json::json!({
        "$defs": { "text": { "type": "string" } },
        "type": "object",
        "properties": { "s": { "$ref": "#/$defs/text" } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"s": null}), None).unwrap() {
        ToolArgumentOutcome::Invalid { .. } => {}
        other => panic!("expected Invalid (null fails the referenced string), got {other:?}"),
    }
}

#[test]
fn union_original_value_wins_before_clone_per_arm() {
    // "12" already matches the string arm: no integer coercion.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "type": ["integer", "string"] } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"n": "12"}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"n": "12"}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
    // anyOf: original value first, then coerced candidates per arm.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "anyOf": [{ "type": "integer" }, { "type": "string" }] } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"n": "12"}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"n": "12"}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
    // A numeric string that only passes after coercion converts.
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "anyOf": [{ "type": "integer", "minimum": 15 }] } }
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!({"n": "0x10"}), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"n": 16}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

#[test]
fn additional_properties_and_tuple_items_coercion() {
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "a": { "type": "boolean" } },
        "additionalProperties": { "type": "number" }
    }))
    .unwrap();
    let outcome = validate_tool_arguments(
        &compiled,
        &serde_json::json!({"a": null, "extra": "12"}),
        None,
    )
    .unwrap();
    match outcome {
        ToolArgumentOutcome::Valid { normalized } => {
            // optional-null deletion removes `a` (not required, sub-schema
            // rejects null) before coercion, exactly like the upstream.
            assert_eq!(normalized, serde_json::json!({"extra": 12}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
    // Tuple coercion follows the upstream: only an `items` ARRAY coerces
    // index-wise (`prefixItems` is not consulted by the coercion walk).
    let compiled = compile(serde_json::json!({
        "type": "array",
        "items": [{ "type": "string" }, { "type": "boolean" }],
        "minItems": 2,
        "maxItems": 2
    }))
    .unwrap();
    match validate_tool_arguments(&compiled, &serde_json::json!([12, "true"]), None).unwrap() {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!(["12", true]));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}

#[test]
fn failures_are_redacted_structured_reports() {
    let compiled = compile(serde_json::json!({
        "type": "object",
        "properties": { "name": { "type": "string", "minLength": 5 } },
        "required": ["name"],
        "additionalProperties": false
    }))
    .unwrap();
    let secret = "abc";
    match validate_tool_arguments(&compiled, &serde_json::json!({"name": secret}), None).unwrap() {
        ToolArgumentOutcome::Invalid { failures } => {
            let report = format!("{failures:?}");
            assert!(!report.contains(secret), "instance leaked: {report}");
            assert_eq!(failures[0].instance_path, "/name");
            assert_eq!(failures[0].keyword, "minLength");
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn admission_failure_is_a_typed_engine_error() {
    // A structurally selected invalid regex fails admission (fail-closed),
    // surface as SchemaError rather than a validation verdict.
    let err = compile(serde_json::json!({
        "type": "object",
        "properties": { "s": { "type": "string", "pattern": "(" } }
    }))
    .unwrap_err();
    assert!(matches!(err, opi_ai::schema::SchemaError::Regex { .. }));
}

#[test]
fn validate_tool_call_resolves_by_name() {
    let int_schema = compile(serde_json::json!({
        "type": "object",
        "properties": { "n": { "type": "integer" } },
        "required": ["n"]
    }))
    .unwrap();
    let text_schema = compile(serde_json::json!({
        "type": "object",
        "properties": { "s": { "type": "string" } },
        "required": ["s"]
    }))
    .unwrap();
    let tools = [("add", &int_schema), ("echo", &text_schema)];
    let unknown = validate_tool_call(&tools, "nope", &serde_json::json!({}), None)
        .unwrap()
        .unwrap_err();
    assert!(unknown.contains("not found"));
    match validate_tool_call(&tools, "add", &serde_json::json!({"n": "7"}), None)
        .unwrap()
        .unwrap()
    {
        ToolArgumentOutcome::Valid { normalized } => {
            assert_eq!(normalized, serde_json::json!({"n": 7}));
        }
        other => panic!("expected Valid, got {other:?}"),
    }
}
