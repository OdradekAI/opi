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

#[test]
fn annotations_do_not_admit_validation_keywords() {
    for schema in [
        serde_json::json!({"type":"object", "default":{"pattern":"["}}),
        serde_json::json!({"type":"object", "examples":[{"pattern":"["}]}),
        serde_json::json!({"type":"object", "annotation":{"patternProperties":{"[":true}}}),
        serde_json::json!({"additionalItems":{"pattern":"["}}),
        serde_json::json!({"items":true, "additionalItems":{"pattern":"["}}),
        serde_json::json!({"items":[1], "additionalItems":{"pattern":"["}}),
    ] {
        let compiled = compile(schema).expect("annotation data is not a subschema");
        assert!(compiled.check(&serde_json::json!({})).unwrap());
    }
    assert!(
        compile(serde_json::json!({"items":[true], "additionalItems":{"pattern":"["}})).is_err()
    );
    let referenced = compile(serde_json::json!({
        "$id":"https://offline.invalid/root", "metadata":{"$id":"leaf", "pattern":"^ok$"}, "$ref":"leaf"
    })).unwrap();
    assert!(referenced.check(&serde_json::json!("ok")).unwrap());
    assert!(!referenced.check(&serde_json::json!("no")).unwrap());
    assert!(compile(serde_json::json!({
        "$id":"https://offline.invalid/root", "metadata":{"$id":"leaf", "pattern":"["}, "$ref":"leaf"
    })).is_err());
    for keyword in ["const", "enum"] {
        let schema = serde_json::json!({"metadata":{keyword:{"pattern":"^ok$"}}, "$ref":format!("#/metadata/{keyword}")});
        let compiled = compile(schema).unwrap();
        assert!(compiled.check(&serde_json::json!("ok")).unwrap());
        assert!(!compiled.check(&serde_json::json!("bad")).unwrap());
    }
}

#[test]
fn malformed_schema_maps_are_inert_as_a_whole() {
    use serde_json::json;

    for keyword in ["properties", "patternProperties"] {
        for invalid in [json!(1), Value::Null, json!([]), json!("schema")] {
            let schema = json!({keyword: {"a": false, "ignored": invalid}});
            let compiled = compile(schema.clone()).unwrap();
            let raw = json!({"a": 1});
            assert!(compiled.check(&raw).unwrap(), "{schema}");
            assert_eq!(
                validate_tool_arguments(&compiled, &raw, None).unwrap(),
                ToolArgumentOutcome::Valid {
                    normalized: raw.clone()
                },
                "{schema}"
            );

            // An inert map contributes no covered properties to its sibling.
            let mut closed = schema;
            closed["additionalProperties"] = json!(false);
            let compiled = compile(closed).unwrap();
            assert!(!compiled.check(&raw).unwrap());
        }

        let active = compile(json!({keyword: {"a": false, "ok": true}})).unwrap();
        assert!(!active.check(&json!({"a": 1})).unwrap());
        assert!(active.check(&json!({"ok": 1})).unwrap());
    }
}

#[test]
fn malformed_schema_maps_do_not_admit_regexes() {
    use serde_json::json;

    for schema in [
        json!({"patternProperties": {"[": 1}}),
        json!({"patternProperties": {"[": true, "ignored": 1}}),
        json!({"properties": {"a": {"pattern": "["}, "ignored": 1}}),
    ] {
        let compiled = compile(schema.clone()).expect("malformed map is inert");
        let raw = json!({"a": "text"});
        assert!(compiled.check(&raw).unwrap(), "{schema}");
        assert_eq!(
            validate_tool_arguments(&compiled, &raw, None).unwrap(),
            ToolArgumentOutcome::Valid { normalized: raw },
            "{schema}"
        );
    }
    for schema in [
        json!({"patternProperties": {"[": true}}),
        json!({"properties": {"a": {"pattern": "["}, "allowed": true}}),
    ] {
        assert!(
            matches!(
                compile(schema),
                Err(opi_ai::schema::SchemaError::Regex { .. })
            ),
            "a structurally valid map must admit its regexes"
        );
    }
}

#[test]
fn refs_into_const_and_enum_preserve_evaluated_tracking() {
    use serde_json::json;

    for (target, accepted, rejected) in [
        (
            json!({"properties": {"a": true}, "unevaluatedProperties": false}),
            json!({"a": 1}),
            json!({"a": 1, "extra": 2}),
        ),
        (
            json!({"prefixItems": [true], "unevaluatedItems": false}),
            json!([1]),
            json!([1, 2]),
        ),
    ] {
        let direct = compile(target.clone()).unwrap();
        assert!(direct.check(&accepted).unwrap());
        assert!(!direct.check(&rejected).unwrap());
        for schema in [
            json!({"metadata": {"const": target}, "$ref": "#/metadata/const"}),
            json!({"metadata": {"enum": [target]}, "$ref": "#/metadata/enum/0"}),
        ] {
            // Rebuild the document exactly as a saved declaration is restored.
            let restored: Value =
                serde_json::from_slice(&serde_json::to_vec(&schema).unwrap()).unwrap();
            let compiled = compile(restored).unwrap();
            assert!(compiled.check(&accepted).unwrap(), "{schema}");
            assert!(!compiled.check(&rejected).unwrap(), "{schema}");
            assert_eq!(
                validate_tool_arguments(&compiled, &accepted, None).unwrap(),
                ToolArgumentOutcome::Valid {
                    normalized: accepted.clone()
                },
                "{schema}"
            );
            assert!(matches!(
                validate_tool_arguments(&compiled, &rejected, None).unwrap(),
                ToolArgumentOutcome::Invalid { .. }
            ));
        }
    }
}

#[test]
fn unreferenced_const_and_enum_data_remain_data_with_unevaluated_keys() {
    use serde_json::json;

    let data = json!({"pattern": "[", "unevaluatedProperties": false});
    for schema in [json!({"const": data}), json!({"enum": [data]})] {
        let compiled = compile(schema).expect("constant data is not a regex schema");
        assert!(compiled.check(&data).unwrap());
        assert!(!compiled.check(&json!({})).unwrap());
        assert_eq!(
            validate_tool_arguments(&compiled, &data, None).unwrap(),
            ToolArgumentOutcome::Valid {
                normalized: data.clone()
            }
        );
    }
}

#[test]
fn upstream_regex_data_reaches_all_public_schema_paths() {
    use serde_json::json;

    // Known data changes in the supported upstream profile, not a promise to
    // freeze this Unicode release for every future dependency resolution.
    for (script, value) in [
        ("Beria_Erfe", "\u{16ea0}"),
        ("Sidetic", "\u{10940}"),
        ("Tai_Yo", "\u{1e6c0}"),
        ("Tolong_Siki", "\u{11db0}"),
    ] {
        let pattern = format!("^\\p{{Script={script}}}$");
        let compiled = compile(json!({"type":"string", "pattern":pattern})).unwrap();
        assert!(compiled.check(&json!(value)).unwrap(), "{script}");
        assert!(!compiled.check(&json!("a")).unwrap(), "{script}");
        assert_eq!(
            validate_tool_arguments(&compiled, &json!(value), None).unwrap(),
            ToolArgumentOutcome::Valid {
                normalized: json!(value)
            }
        );
    }
    for schema in [
        json!({"propertyNames":{"pattern":"^\\p{Letter}$"}}),
        json!({"patternProperties":{"^\\p{Letter}$":{"const":3}}, "additionalProperties":false}),
    ] {
        let compiled = compile(schema).unwrap();
        assert!(compiled.check(&json!({"\u{16ea0}":3})).unwrap());
        assert!(!compiled.check(&json!({"1":3})).unwrap());
    }
    let regex = compile(json!({"format":"regex"})).unwrap();
    assert!(regex.check(&json!("^\\p{Script=Beria_Erfe}$")).unwrap());
    assert!(!regex.check(&json!("[")).unwrap());
    assert!(compile(json!({"pattern":"["})).is_err());
}

#[test]
fn upstream_domain_data_preserves_format_specific_rejection() {
    use serde_json::json;

    for format in ["url", "iri", "iri-reference"] {
        let compiled = compile(json!({"type":"string", "format":format})).unwrap();
        for value in [
            "http://\u{16ea0}.example/",
            "http://%F0%96%BA%A0.example/",
            "http://xn--zs0f.example/",
            "https://example.com/\u{16ea0}",
        ] {
            assert!(compiled.check(&json!(value)).unwrap(), "{format}: {value}");
        }
        for value in [
            "http://[",
            "http://a\u{200d}.example/",
            "http://xn--7r0f.example/",
        ] {
            assert!(!compiled.check(&json!(value)).unwrap(), "{format}: {value}");
        }
    }
    for (format, good, bad) in [
        ("idn-hostname", "\u{16ea0}.example", "a\u{200d}.example"),
        ("idn-email", "a@\u{16ea0}.example", "a@a\u{200d}.example"),
    ] {
        let compiled = compile(json!({"format":format})).unwrap();
        assert!(compiled.check(&json!(good)).unwrap(), "{format}");
        assert!(!compiled.check(&json!(bad)).unwrap(), "{format}");
    }
    // The URI formats keep their source regex algorithm, separate from URL.
    let uri = compile(json!({"format":"uri"})).unwrap();
    assert!(!uri.check(&json!("http://\u{16ea0}.example/")).unwrap());
    assert!(uri.check(&json!("http://xn--zs0f.example/")).unwrap());
}

#[test]
fn upstream_idna_mapping_survives_schema_roundtrip_and_relative_refs() {
    use serde_json::json;

    for target_host in [
        "\u{16ea0}.example",
        "%F0%96%BA%A0.example",
        "xn--zs0f.example",
    ] {
        for reference_host in [
            "\u{16ea0}.example",
            "%F0%96%BA%A0.example",
            "xn--zs0f.example",
        ] {
            for relative in [false, true] {
                let reference = if relative {
                    "leaf".to_owned()
                } else {
                    format!("http://{reference_host}/dir/leaf")
                };
                let schema = json!({
                    "$id":format!("http://{reference_host}/dir/root"),
                    "metadata":{"$id":format!("http://{target_host}/dir/leaf"),"const":3},
                    "$ref":reference
                });
                let restored =
                    serde_json::from_slice(&serde_json::to_vec(&schema).unwrap()).unwrap();
                let compiled = compile(restored).unwrap();
                assert!(compiled.check(&json!(3)).unwrap(), "{schema}");
                assert!(!compiled.check(&json!(4)).unwrap(), "{schema}");
            }
        }
    }
    let invalid = compile(json!({"$id":"http://xn--7r0f.example/", "$ref":"leaf"})).unwrap();
    assert!(matches!(
        invalid.check(&json!(3)),
        Err(opi_ai::schema::SchemaError::Uri { .. })
    ));
}

#[test]
fn upstream_new_script_admission_keeps_optional_null_and_union_rules() {
    use serde_json::json;

    let property = json!({"type":"string", "pattern":"^\\p{Script=Beria_Erfe}$"});
    let compiled = compile(json!({"type":"object", "properties":{"p":property}})).unwrap();
    let raw = json!({"p":null});
    assert_eq!(
        validate_tool_arguments(&compiled, &raw, None).unwrap(),
        ToolArgumentOutcome::Valid {
            normalized: json!({})
        }
    );
    assert_eq!(raw, json!({"p":null}));
    let required =
        compile(json!({"type":"object", "properties":{"p":property}, "required":["p"]})).unwrap();
    assert!(matches!(
        validate_tool_arguments(&required, &raw, None).unwrap(),
        ToolArgumentOutcome::Invalid { .. }
    ));
    let union = compile(json!({"anyOf":[property, {"const":"fallback"}]})).unwrap();
    assert!(union.check(&json!("\u{16ea0}")).unwrap());
    assert!(union.check(&json!("fallback")).unwrap());
    assert!(!union.check(&json!("unrelated")).unwrap());
}

#[test]
fn public_validation_uses_supported_upstream_data() {
    let letter = compile(serde_json::json!({"type":"string", "pattern":"^\\p{Letter}$"})).unwrap();
    assert!(letter.check(&serde_json::json!("\u{16ea0}")).unwrap());
    assert!(letter.check(&serde_json::json!("\u{105c0}")).unwrap());
    let url = compile(serde_json::json!({"type":"string", "format":"url"})).unwrap();
    for value in ["http://\u{16ea0}.example", "http://%F0%96%BA%A0.example"] {
        assert!(url.check(&serde_json::json!(value)).unwrap(), "{value}");
    }
    assert!(
        url.check(&serde_json::json!("https://example.com/\u{16ea0}"))
            .unwrap()
    );
}

#[test]
fn exact_integer_constraints_do_not_round_into_success() {
    use serde_json::json;
    for (schema, value) in [
        (
            json!({"const":9007199254740992u64}),
            json!(9007199254740993u64),
        ),
        (
            json!({"enum":[9007199254740992u64]}),
            json!(9007199254740993u64),
        ),
        (
            json!({"maximum":9007199254740992u64}),
            json!(9007199254740993u64),
        ),
        (
            json!({"minimum":9007199254740993u64}),
            json!(9007199254740992u64),
        ),
        (
            json!({"exclusiveMinimum":9007199254740992u64}),
            json!(9007199254740993u64),
        ),
        (
            json!({"exclusiveMaximum":9007199254740993u64}),
            json!(9007199254740992u64),
        ),
        (json!({"maximum":i64::MAX}), json!(i64::MAX as u64 + 1)),
        (
            json!({"maximum":-9007199254740993i64}),
            json!(-9007199254740992i64),
        ),
    ] {
        let expected =
            schema.get("exclusiveMinimum").is_some() || schema.get("exclusiveMaximum").is_some();
        let compiled = compile(schema.clone()).unwrap();
        let outcome = validate_tool_arguments(&compiled, &value, None).unwrap();
        assert_eq!(
            matches!(outcome, ToolArgumentOutcome::Valid { .. }),
            expected,
            "{schema} / {value}"
        );
    }
    let unique = compile(json!({"uniqueItems":true})).unwrap();
    assert!(
        unique
            .check(&json!([9007199254740992u64, 9007199254740993u64]))
            .unwrap()
    );
    assert!(!unique.check(&json!([1, 1.0])).unwrap());
}

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
fn optional_null_kept_when_subschema_accepts_null_or_uses_ref() {
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
fn optional_null_subcompile_failure_keeps_the_key_in_inert_properties() {
    use serde_json::json;

    let bad_regex = json!({"pattern": "["});
    assert!(matches!(
        compile(bad_regex.clone()),
        Err(opi_ai::schema::SchemaError::Regex { .. })
    ));
    for (property, required, expected) in [
        (bad_regex.clone(), false, json!({"a": null})),
        (json!({"type": "number"}), false, json!({})),
        (json!({"type": "null"}), false, json!({"a": null})),
        (bad_regex, true, json!({"a": null})),
        (json!({"$ref": "#/$defs/text"}), false, json!({"a": null})),
    ] {
        // The malformed sibling makes the evaluator's whole properties map
        // inert; the normalizer still visits each present property.
        let mut schema = json!({
            "type": "object",
            "$defs": {"text": {"type": "string"}},
            "properties": {"a": property, "ignored": 1}
        });
        if required {
            schema["required"] = json!(["a"]);
        }
        let original_schema = schema.clone();
        let compiled = compile(schema.clone()).expect("malformed properties map is inert");
        let raw = json!({"a": null});
        let original_raw = raw.clone();
        assert!(compiled.check(&raw).unwrap(), "{schema}");
        let expected = ToolArgumentOutcome::Valid {
            normalized: expected,
        };
        assert_eq!(
            validate_tool_arguments(&compiled, &raw, None).unwrap(),
            expected,
            "{schema}"
        );
        assert_eq!(
            validate_tool_call(&[("tool", &compiled)], "tool", &raw, None)
                .unwrap()
                .unwrap(),
            expected,
            "{schema}"
        );
        assert_eq!(raw, original_raw, "raw arguments must remain unchanged");
        assert_eq!(schema, original_schema, "schema must remain unchanged");
    }
}

#[test]
fn union_subcompile_failure_does_not_select_the_failed_arm() {
    use serde_json::json;

    let failed_arm = json!({"type": "string", "pattern": "["});
    assert!(matches!(
        compile(failed_arm.clone()),
        Err(opi_ai::schema::SchemaError::Regex { .. })
    ));
    for keyword in ["anyOf", "oneOf"] {
        let schema = json!({
            "type": "object",
            "properties": {
                "a": {keyword: [failed_arm, {"type": "number"}]},
                "ignored": 1
            }
        });
        let original_schema = schema.clone();
        let compiled = compile(schema.clone()).expect("malformed properties map is inert");
        let raw = json!({"a": true});
        let original_raw = raw.clone();
        // Neither arm accepts the original boolean. The failed string arm's
        // coerced candidate must not win over the number arm's fresh clone.
        let expected = ToolArgumentOutcome::Valid {
            normalized: json!({"a": 1}),
        };
        assert_eq!(
            validate_tool_arguments(&compiled, &raw, None).unwrap(),
            expected,
            "{schema}"
        );
        assert_eq!(
            validate_tool_call(&[("tool", &compiled)], "tool", &raw, None)
                .unwrap()
                .unwrap(),
            expected,
            "{schema}"
        );
        assert_eq!(raw, original_raw, "raw arguments must remain unchanged");
        assert_eq!(schema, original_schema, "schema must remain unchanged");
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

#[test]
fn deep_schema_normalization_and_subvalidator_clone_release_normally() {
    const NAME: &str = "deep_schema_normalization_and_subvalidator_clone_release_normally";
    if std::env::var("OPI_SCHEMA_NORMALIZE_CHILD").as_deref() != Ok(NAME) {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture", "--test-threads=1"])
            .env("OPI_SCHEMA_NORMALIZE_CHILD", NAME)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{:?}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut schema = serde_json::json!({"type":"integer"});
            for _ in 0..10_000 {
                let mut map = serde_json::Map::new();
                map.insert("allOf".into(), Value::Array(vec![schema]));
                schema = Value::Object(map);
            }
            let compiled = compile(schema).unwrap();
            assert_eq!(
                validate_tool_arguments(&compiled, &serde_json::json!(1), None).unwrap(),
                ToolArgumentOutcome::Valid {
                    normalized: serde_json::json!(1)
                }
            );
            drop(compiled);
            // Optional-null admission independently compiles this deep child;
            // unlike the scalar fold above, this exercises the schema clone.
            let mut property = serde_json::json!({"type":"integer"});
            for _ in 0..10_000 {
                let mut map = serde_json::Map::new();
                map.insert("allOf".into(), Value::Array(vec![property]));
                property = Value::Object(map);
            }
            let mut properties = serde_json::Map::new();
            properties.insert("n".into(), property);
            let mut root = serde_json::Map::new();
            root.insert("type".into(), serde_json::json!("object"));
            root.insert("properties".into(), Value::Object(properties));
            let compiled = compile(Value::Object(root)).unwrap();
            assert_eq!(
                validate_tool_arguments(&compiled, &serde_json::json!({"n":null}), None).unwrap(),
                ToolArgumentOutcome::Valid {
                    normalized: serde_json::json!({})
                }
            );
            drop(compiled);
        })
        .unwrap()
        .join()
        .unwrap();
}
