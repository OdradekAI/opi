//! Behavioral battery for the private TypeBox JSON dialect engine
//! (`opi_ai::schema`, task 20.9).
//!
//! Two evidence layers:
//!
//! 1. The research-frozen 64-case differential corpus (copied verbatim from
//!    `docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema/`,
//!    pins in that directory's `pins.json`). Current expectations include the
//!    two approved upstream Unicode-data differences (RDF-21). The original
//!    Node and Unicode-16 results remain frozen research evidence.
//! 2. Focused engine tests for reference frames, evaluated sets, typed
//!    no-progress recursion, JavaScript key enumeration and the upstream
//!    structural-inert guards.

use opi_ai::schema::{SchemaError, compile};
use serde_json::{Value, json};

fn check(schema: Value, value: Value) -> Result<bool, SchemaError> {
    compile(schema)?.check(&value)
}

fn valid(schema: Value, value: Value) -> bool {
    check(schema, value).expect("engine error")
}

// ------------------------------------------------------------------
// Frozen differential corpus
// ------------------------------------------------------------------

#[test]
fn frozen_differential_corpus_matches_expected_outcomes() {
    let cases_text = include_str!("data/schema-cases.jsonl");
    let expected: serde_json::Map<String, Value> =
        serde_json::from_str(include_str!("data/schema-expected.json")).unwrap();

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

// ------------------------------------------------------------------
// Core leaves and guards
// ------------------------------------------------------------------

#[test]
fn boolean_and_empty_schemas() {
    assert!(valid(json!(true), json!(null)));
    assert!(!valid(json!(false), json!(null)));
    assert!(valid(json!({}), json!({"x": 1})));
    // Source arrays have no active schema keywords.
    assert!(valid(json!([]), json!(1)));
    assert!(valid(json!([false]), json!(1)));
}

#[test]
fn malformed_limits_are_inert_not_errors() {
    assert!(valid(
        json!({"type": "string", "minLength": "2"}),
        json!("")
    ));
    assert!(valid(json!({"type": "number", "minimum": "10"}), json!(0)));
    assert!(valid(
        json!({"type": "string", "pattern": false}),
        json!("abc")
    ));
    // fractional/negative limits are active numbers with source semantics
    assert!(!valid(
        json!({"type": "string", "minLength": 2.5}),
        json!("ab")
    ));
    assert!(valid(
        json!({"type": "string", "minLength": 2.5}),
        json!("abc")
    ));
}

#[test]
fn malformed_required_and_enum_stay_inert() {
    assert!(valid(json!({"required": [1]}), json!({})));
    assert!(!valid(json!({"required": ["a"]}), json!({})));
    assert!(valid(json!({"enum": "not-an-array"}), json!(1)));
    assert!(valid(json!({"enum": [1, 2]}), json!(2)));
    assert!(!valid(json!({"enum": [1, 2]}), json!(3)));
}

#[test]
fn const_compares_through_javascript_equality() {
    assert!(valid(json!({"const": 1}), json!(1.0)));
    assert!(valid(json!({"const": {"a": [1, 2]}}), json!({"a": [1, 2]})));
    assert!(valid(json!({"const": {"a": 1}}), json!({"a": 1})));
    assert!(!valid(json!({"const": {"a": 1}}), json!({"a": 2})));
    assert!(!valid(json!({"const": 1}), json!("1")));
}

#[test]
fn unknown_type_names_are_true_leaves() {
    // Unknown names match every instance (upstream CheckTypeName
    // falls through to true); a type array is a union, so one unknown
    // member makes every instance valid.
    assert!(valid(json!({"type": "custom"}), json!(1)));
    assert!(valid(json!({"type": ["custom", "integer"]}), json!(2.5)));
    // xschema names that JSON instances can never satisfy
    assert!(!valid(json!({"type": "bigint"}), json!(1)));
}

#[test]
fn integer_type_uses_js_number_semantics() {
    assert!(valid(json!({"type": "integer"}), json!(3)));
    assert!(valid(json!({"type": "integer"}), json!(3.0)));
    assert!(!valid(json!({"type": "integer"}), json!(3.5)));
}

#[test]
fn number_limits_compare_through_binary64() {
    assert!(valid(json!({"minimum": 0.1}), json!(0.1)));
    assert!(!valid(json!({"minimum": 10}), json!(9.999)));
    assert!(valid(json!({"exclusiveMaximum": 10}), json!(9)));
    assert!(!valid(json!({"exclusiveMaximum": 10}), json!(10)));
    assert!(!valid(json!({"maximum": 10}), json!(10.1)));
}

#[test]
fn multiple_of_uses_the_source_tolerance() {
    assert!(valid(
        json!({"type": "number", "multipleOf": 0.1}),
        json!(0.3)
    ));
    assert!(valid(
        json!({"type": "number", "multipleOf": 0.1}),
        json!(3)
    ));
    assert!(!valid(
        json!({"type": "number", "multipleOf": 10}),
        json!(5)
    ));
}

#[test]
fn unique_items_uses_hash_equality() {
    assert!(valid(
        json!({"type": "array", "uniqueItems": true}),
        json!([1, 2])
    ));
    assert!(!valid(
        json!({"type": "array", "uniqueItems": true}),
        json!([1, 1.0])
    ));
    assert!(valid(
        json!({"type": "array", "uniqueItems": false}),
        json!([1, 1])
    ));
    assert!(!valid(
        json!({"type": "array", "uniqueItems": true}),
        json!([{"a": 1}, {"a": 1}])
    ));
}

// ------------------------------------------------------------------
// Array / object keywords
// ------------------------------------------------------------------

#[test]
fn items_and_prefix_items_both_apply() {
    let schema = json!({"type": "array", "items": [{"const": 1}], "prefixItems": [{"const": 1}, {"const": 2}]});
    assert!(valid(schema.clone(), json!([1, 2])));
    assert!(!valid(schema, json!([1, 3])));
    // items tuple only constrains its own positions
    let schema = json!({"items": [{"const": 1}]});
    assert!(valid(schema, json!([1, "anything", 5])));
}

#[test]
fn unsized_items_respect_prefix_offset() {
    let schema = json!({"prefixItems": [{"const": 0}], "items": {"const": 9}});
    assert!(valid(schema.clone(), json!([0, 9, 9])));
    assert!(!valid(schema, json!([0, 9, 8])));
}

#[test]
fn additional_items_apply_after_tuple_positions() {
    let schema = json!({"items": [{"const": 1}], "additionalItems": {"const": 7}});
    assert!(valid(schema.clone(), json!([1, 7, 7])));
    assert!(!valid(schema, json!([1, 7, 8])));
}

#[test]
fn contains_interacts_with_min_and_max() {
    let schema = json!({"contains": {"type": "integer"}});
    assert!(!valid(schema.clone(), json!([])));
    assert!(valid(schema, json!(["a", 1])));
    let schema = json!({"contains": {"type": "integer"}, "minContains": 2});
    assert!(!valid(schema.clone(), json!([1])));
    assert!(valid(schema, json!([1, 2])));
    let schema = json!({"contains": {"type": "integer"}, "maxContains": 1});
    assert!(valid(schema.clone(), json!(["a", 1])));
    assert!(!valid(schema, json!([1, 2])));
    // minContains: 0 disables contains entirely
    let schema = json!({"contains": false, "minContains": 0});
    assert!(valid(schema, json!(["anything"])));
}

#[test]
fn dependencies_support_both_forms() {
    let schema = json!({"dependencies": {"credit": ["billing"]}});
    assert!(valid(schema.clone(), json!({"billing": 1})));
    assert!(!valid(schema, json!({"credit": 1})));
    let schema = json!({"dependencies": {"credit": {"required": ["billing"]}}});
    assert!(!valid(schema.clone(), json!({"credit": 1})));
    assert!(valid(schema.clone(), json!({"credit": 1, "billing": 2})));
    // empty object shortcut
    assert!(valid(schema, json!({})));
}

#[test]
fn dependent_required_and_dependent_schemas() {
    let schema = json!({"dependentRequired": {"a": ["b"]}});
    assert!(!valid(schema.clone(), json!({"a": 1})));
    assert!(valid(schema, json!({"a": 1, "b": 2})));
    let schema = json!({"dependentSchemas": {"a": {"required": ["b"]}}});
    assert!(!valid(schema.clone(), json!({"a": 1})));
    assert!(valid(schema, json!({"a": 1, "b": 2})));
}

#[test]
fn property_names_validates_key_strings() {
    let schema = json!({"propertyNames": {"maxLength": 2}});
    assert!(valid(schema.clone(), json!({"ab": 1})));
    assert!(!valid(schema, json!({"abc": 1})));
}

#[test]
fn pattern_properties_apply_per_matching_key() {
    let schema = json!({"patternProperties": {"^s_": {"type": "integer"}}});
    assert!(valid(schema.clone(), json!({"s_x": 1, "other": "str"})));
    assert!(!valid(schema, json!({"s_x": "str"})));
}

#[test]
fn additional_properties_fast_path_accepts_required_superset_keys() {
    // Upstream Compile-path optimization: with properties/required length
    // agreement and additionalProperties false, the check reduces to the
    // instance key count equaling required length. required may name keys
    // outside properties; the fast path still accepts exact shapes.
    let schema = json!({
        "properties": {"a": {}},
        "required": ["b"],
        "additionalProperties": false
    });
    assert!(valid(schema.clone(), json!({"b": 1})));
    assert!(!valid(schema.clone(), json!({"b": 1, "a": 2})));
    // required failure short-circuits first
    assert!(!valid(schema, json!({"a": 1})));
}

#[test]
fn additional_properties_schema_applies_to_uncovered_keys() {
    let schema = json!({
        "properties": {"a": {}},
        "patternProperties": {"^p_": {}},
        "additionalProperties": false
    });
    assert!(valid(schema.clone(), json!({"a": 1, "p_x": 2})));
    assert!(!valid(schema, json!({"b": 1})));
}

// ------------------------------------------------------------------
// Combinators
// ------------------------------------------------------------------

#[test]
fn if_then_else_uses_default_true_branches() {
    let schema = json!({"if": {"type": "integer"}, "then": {"minimum": 10}});
    assert!(valid(schema.clone(), json!(11)));
    assert!(!valid(schema, json!(5)));
    let schema = json!({"if": {"type": "integer"}, "else": {"const": "x"}});
    assert!(valid(schema.clone(), json!("x")));
    assert!(!valid(schema.clone(), json!("y")));
    assert!(valid(schema, json!(1)));
}

#[test]
fn one_of_requires_exactly_one() {
    let schema = json!({"oneOf": [{"type": "integer"}, {"minimum": 10}]});
    assert!(valid(schema.clone(), json!(5)));
    assert!(valid(schema.clone(), json!(20.5)));
    assert!(!valid(schema.clone(), json!(20)));
    assert!(!valid(schema, json!(5.5)));
}

#[test]
fn any_of_and_all_of_compose() {
    assert!(valid(
        json!({"anyOf": [{"type": "string"}, {"type": "integer"}]}),
        json!(1)
    ));
    assert!(!valid(
        json!({"anyOf": [{"type": "string"}, {"type": "boolean"}]}),
        json!(1)
    ));
    assert!(!valid(
        json!({"allOf": [{"type": "integer"}, {"minimum": 2}]}),
        json!(1)
    ));
    // empty combinator edge cases from ReduceAnd/ReduceOr
    assert!(valid(json!({"allOf": []}), json!(1)));
    assert!(!valid(json!({"anyOf": []}), json!(1)));
    assert!(!valid(json!({"oneOf": []}), json!(1)));
}

#[test]
fn not_negates_without_error_conversion() {
    assert!(valid(json!({"not": {"type": "string"}}), json!(1)));
    assert!(!valid(json!({"not": {"type": "integer"}}), json!(1)));
}

// ------------------------------------------------------------------
// References
// ------------------------------------------------------------------

#[test]
fn local_pointer_refs_and_sibling_keywords() {
    let schema = json!({"$defs": {"n": {"type": "integer", "minimum": 1}}, "$ref": "#/$defs/n"});
    assert!(valid(schema.clone(), json!(2)));
    assert!(!valid(schema.clone(), json!(0)));
    // sibling keywords apply alongside $ref
    let schema = json!({"$defs": {"n": {"type": "integer"}}, "$ref": "#/$defs/n", "minimum": 1});
    assert!(valid(schema.clone(), json!(2)));
    assert!(!valid(schema, json!(0)));
}

#[test]
fn unresolved_refs_are_invalid_not_errors() {
    assert!(!valid(json!({"$ref": "urn:probe:absent"}), json!(1)));
    assert!(!valid(json!({"$ref": "#/missing"}), json!(1)));
}

#[test]
fn anchor_refs_resolve_by_url_identity() {
    let schema = json!({"$defs": {"a": {"$anchor": "leaf", "type": "integer"}}, "$ref": "#leaf"});
    assert!(valid(schema.clone(), json!(1)));
    assert!(!valid(schema, json!("x")));
}

#[test]
fn absolute_id_refs_reach_embedded_metadata_resources() {
    let schema = json!({
        "$id": "https://offline.invalid/root",
        "x-meta": {"$id": "https://offline.invalid/target", "type": "integer", "minimum": 2},
        "$ref": "https://offline.invalid/target"
    });
    assert!(valid(schema.clone(), json!(3)));
    assert!(!valid(schema, json!(0)));
}

#[test]
fn refs_index_into_arrays_by_numeric_pointer() {
    let schema = json!({"$defs": [{"type": "integer"}], "$ref": "#/$defs/0"});
    assert!(valid(schema.clone(), json!(1)));
    assert!(!valid(schema, json!("x")));
}

#[test]
fn recursive_schema_walks_productive_instance_recursion() {
    let schema = json!({
        "type": "object",
        "properties": {"next": {"$ref": "#"}, "value": {"type": "integer"}},
        "additionalProperties": false
    });
    // deep chain: productive recursion, no typed failure, no stack overflow
    // (built with constructors; the json! macro serializes expression
    // values recursively)
    let mut value = serde_json::json!({"value": 0});
    for i in 1..2000 {
        let mut map = serde_json::Map::new();
        map.insert("value".into(), json!(i));
        map.insert("next".into(), value);
        value = serde_json::Value::Object(map);
    }
    assert!(valid(schema.clone(), value));
    assert!(!valid(
        schema,
        json!({"value": 1, "next": {"value": "bad"}})
    ));
}

#[test]
fn unchanged_instance_self_recursion_is_a_typed_failure() {
    let schema = json!({"$ref": "#"});
    let error = check(schema, json!(1)).unwrap_err();
    assert!(matches!(error, SchemaError::NoProgress));
    // allOf-wrapped self-reference behaves the same
    let schema = json!({"allOf": [{"$ref": "#"}]});
    assert!(matches!(
        check(schema, json!(1)),
        Err(SchemaError::NoProgress)
    ));
}

fn ordinary_stack_subprocess(name: &str, test: fn()) {
    if std::env::var("OPI_SCHEMA_LIFECYCLE_CHILD").as_deref() == Ok(name) {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(test)
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env("OPI_SCHEMA_LIFECYCLE_CHILD", name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name}: {:?}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn nested_all_of(mut schema: Value) -> Value {
    for _ in 0..10_000 {
        let mut map = serde_json::Map::new();
        map.insert("allOf".into(), Value::Array(vec![schema]));
        schema = Value::Object(map);
    }
    schema
}

#[test]
fn deep_schemas_do_not_exhaust_the_rust_stack() {
    ordinary_stack_subprocess("deep_schemas_do_not_exhaust_the_rust_stack", || {
        let mut schema = nested_all_of(json!({"type":"integer"}));
        schema
            .as_object_mut()
            .unwrap()
            .insert("$id".into(), json!("https://offline.invalid/root"));
        let compiled = compile(schema).expect("compiles");
        assert_eq!(compiled.check(&json!(1)), Ok(true));
        let last_owner = compiled.clone();
        drop(compiled);
        assert_eq!(last_owner.check(&json!("bad")), Ok(false));
        drop(last_owner);
    });
}

#[test]
fn deep_local_ref_search() {
    ordinary_stack_subprocess("deep_local_ref_search", || {
        for reference in ["#target", "#missing"] {
            let mut schema = nested_all_of(json!({"$anchor":"target", "type":"integer"}));
            schema
                .as_object_mut()
                .unwrap()
                .insert("$ref".into(), json!(reference));
            let compiled = compile(schema).unwrap();
            assert_eq!(compiled.check(&json!(1)), Ok(reference == "#target"));
            drop(compiled);
        }
    });
}

#[test]
fn deep_compile_error_releases_owned_schema() {
    ordinary_stack_subprocess("deep_compile_error_releases_owned_schema", || {
        let schema = nested_all_of(json!({"pattern":"["}));
        assert!(matches!(compile(schema), Err(SchemaError::Regex { .. })));
    });
}

#[test]
fn dynamic_ref_resolves_the_nearest_dynamic_anchor() {
    let schema = json!({
        "$defs": {
            "inner": {
                "$dynamicAnchor": "node",
                "type": "object",
                "properties": {"leaf": {"const": "end"}, "next": {"$dynamicRef": "#node"}}
            }
        },
        "$dynamicAnchor": "node",
        "$ref": "#/$defs/inner"
    });
    assert!(valid(
        schema.clone(),
        json!({"next": {"next": {"leaf": "end"}}})
    ));
    assert!(!valid(schema, json!({"next": {"leaf": "wrong"}})));
}

#[test]
fn recursive_ref_uses_the_recursive_anchor_root() {
    let schema = json!({
        "$recursiveAnchor": true,
        "type": "object",
        "properties": {"child": {"$recursiveRef": "#"}}
    });
    assert!(valid(schema.clone(), json!({"child": {"child": {}}})));
}

// ------------------------------------------------------------------
// Evaluated sets (reducer mode)
// ------------------------------------------------------------------

#[test]
fn unevaluated_properties_tracks_all_of_successes() {
    let schema = json!({
        "allOf": [
            {"properties": {"a": {"type": "integer"}}},
            {"properties": {"b": {"type": "integer"}}}
        ],
        "unevaluatedProperties": false
    });
    assert!(valid(schema.clone(), json!({"a": 1, "b": 2})));
    assert!(!valid(schema, json!({"a": 1, "b": 2, "c": 3})));
}

#[test]
fn unevaluated_properties_only_merges_successful_any_of_arms() {
    let schema = json!({
        "anyOf": [
            {"properties": {"a": {"const": 1}}},
            {"properties": {"b": {"const": 2}}, "required": ["b"]}
        ],
        "unevaluatedProperties": false
    });
    // arm 1 passes and evaluates "a"; arm 2 fails (b missing) and must not
    // leak any facts, so "c" stays unevaluated.
    assert!(valid(schema.clone(), json!({"a": 1})));
    assert!(!valid(schema, json!({"a": 1, "c": 3})));
}

#[test]
fn unevaluated_items_merges_successful_arms() {
    let schema = json!({
        "anyOf": [
            {"items": {"const": 1}, "type": "array"},
            {"prefixItems": [{"const": "x"}]}
        ],
        "unevaluatedItems": false
    });
    assert!(valid(schema.clone(), json!([1, 1])));
    assert!(valid(schema.clone(), json!(["x"])));
    assert!(!valid(schema, json!([1, "x"])));
}

#[test]
fn not_never_contributes_evaluated_facts() {
    let schema = json!({
        "not": {"properties": {"a": {"type": "string"}}},
        "unevaluatedProperties": false
    });
    // "a": 1 makes the not-arm fail (so not passes), but its key is not
    // evaluated by the arm's success state.
    assert!(!valid(schema, json!({"a": 1})));
}

#[test]
fn nested_containers_keep_their_own_evaluated_frames() {
    let schema = json!({
        "properties": {
            "outer": {
                "properties": {"inner": {"type": "integer"}},
                "unevaluatedProperties": false
            }
        }
    });
    assert!(valid(schema.clone(), json!({"outer": {"inner": 1}})));
    assert!(!valid(
        schema.clone(),
        json!({"outer": {"inner": 1, "extra": 2}})
    ));
    // the outer container has no unevaluatedProperties of its own
    assert!(valid(schema, json!({"outer": {"inner": 1}, "sibling": 5})));
}

#[test]
fn ref_targets_merge_evaluated_facts_on_success() {
    let schema = json!({
        "$defs": {"n": {"properties": {"a": {"type": "integer"}}}},
        "$ref": "#/$defs/n",
        "unevaluatedProperties": false
    });
    assert!(valid(schema.clone(), json!({"a": 1})));
    assert!(!valid(schema, json!({"a": 1, "b": 2})));
}

// ------------------------------------------------------------------
// JavaScript enumeration order
// ------------------------------------------------------------------

#[test]
fn integer_index_keys_enumerate_first_in_numeric_order() {
    // additionalProperties sees keys in JavaScript order: "1", "2", "10",
    // then insertion-order strings. Observable through additionalProperties
    // with an order-sensitive... the acceptance only depends on set
    // membership, so exercise the order through a recording pattern: use
    // patternProperties with two patterns where later patterns override the
    // earlier evaluation order visibility via oneOf counting.
    // Simplest observable: duplicate $id resolution uses enumeration order.
    let schema = json!({
        "$id": "https://x.invalid/root",
        "definitions": {
            "dup": {"$id": "https://x.invalid/first", "const": 1},
            "b": {"$id": "https://x.invalid/second", "const": 2}
        },
        "$ref": "https://x.invalid/first"
    });
    assert!(valid(schema.clone(), json!(1)));
    assert!(!valid(schema, json!(2)));
}

#[test]
fn duplicate_ids_last_occurrence_wins_in_enumeration_order() {
    // FromObject/FromArray keep the LAST match in source enumeration order.
    let schema = json!({
        "$defs": {
            "x": {"$id": "https://x.invalid/dup", "const": 10},
            "y": {"$id": "https://x.invalid/dup", "const": 20}
        },
        "$ref": "https://x.invalid/dup"
    });
    assert!(!valid(schema.clone(), json!(10)));
    assert!(valid(schema, json!(20)));
}

// ------------------------------------------------------------------
// Admission errors
// ------------------------------------------------------------------

#[test]
fn admission_rejects_primitive_roots_with_typed_errors() {
    for schema in [json!(null), json!(0), json!("")] {
        assert!(matches!(
            compile(schema),
            Err(SchemaError::InvalidSchema { path }) if path.is_empty()
        ));
    }
}

#[test]
fn admission_retrieved_frames_preserve_consumed_resolved_resources() {
    let schema = json!({
        "$id": "https://offline.invalid/root/",
        "properties": {
            "first": {"$ref": "https://offline.invalid/root/child/#/schemas/P/properties/t"},
            "second": {"$ref": "https://offline.invalid/root/child/#/schemas/P/properties/t"},
            "third": {"$ref": "#/schemas/P"}
        },
        "schemas": {
            "U": {
                "$id": "child/",
                "schemas": {
                    "P": {"properties": {"t": {}}, "$ref": "leaf"}
                }
            }
        }
    });
    let compiled = compile(schema).expect("source permits a consumed resolved-resource frame");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"first": 1})), Ok(true));
    assert_eq!(compiled.check(&json!({"second": 1})), Ok(true));
    assert_eq!(compiled.check(&json!({"third": {}})), Ok(false));
    assert_eq!(compiled.check(&json!({"third": {"t": 1}})), Ok(false));
}

#[test]
fn admission_reference_targets_preserve_null_boolean_and_array_guards() {
    for reference_kind in ["$ref", "$recursiveRef", "$dynamicRef"] {
        for (target, expected) in [
            (json!(null), false),
            (json!(false), false),
            (json!(true), true),
            (json!([]), true),
            (json!([false]), true),
        ] {
            let mut schema = json!({"metadata": {"leaf": target}});
            schema[reference_kind] = json!("#/metadata/leaf");
            let compiled = compile(schema).expect("source compiles the selected target");
            assert_eq!(
                compiled.check(&json!(1)),
                Ok(expected),
                "{reference_kind} target {target}"
            );
        }
    }
}

#[test]
fn admission_rejects_primitive_reference_targets_with_typed_errors() {
    for reference_kind in ["$ref", "$recursiveRef", "$dynamicRef"] {
        for target in [json!(0), json!("")] {
            let mut schema = json!({"metadata": {"leaf": target}});
            schema[reference_kind] = json!("#/metadata/leaf");
            assert!(matches!(
                compile(schema),
                Err(SchemaError::InvalidSchema { path }) if path == "/metadata/leaf"
            ));
        }
    }
}

#[test]
fn invalid_regexes_fail_admission_with_paths() {
    let error = compile(json!({"properties": {"a": {"pattern": "[z-a]"}}})).unwrap_err();
    match error {
        SchemaError::Regex { path, .. } => assert_eq!(path, "/properties/a/pattern"),
        other => panic!("{other:?}"),
    }
    let error = compile(json!({"patternProperties": {"(": {}}})).unwrap_err();
    assert!(matches!(error, SchemaError::Regex { .. }));
}

fn assert_admission_regex_path(schema: Value, expected_path: &str) {
    match compile(schema).expect_err("selected regex must fail during compilation") {
        SchemaError::Regex { path, .. } => assert_eq!(path, expected_path),
        other => panic!("expected regex admission failure, got {other:?}"),
    }
}

#[test]
fn admission_structural_refs_precede_root_refs() {
    for reference_kind in ["$ref", "$recursiveRef", "$dynamicRef"] {
        let mut schema = json!({
            "properties": {"p": {"$id": "https://offline.invalid/branch/p", "$ref": "leaf"}},
            "metadata": {
                "target": {"$id": "https://offline.invalid/target"},
                "leaf": {"$id": "https://offline.invalid/branch/leaf", "pattern": "["}
            }
        });
        schema[reference_kind] = json!("https://offline.invalid/target");
        // Keyword execution order is independent of JSON field insertion.
        for reverse in [false, true] {
            let mut ordered = schema.clone();
            if reverse {
                let fields = ordered.as_object_mut().unwrap();
                *fields = std::mem::take(fields).into_iter().rev().collect();
            }
            assert_admission_regex_path(ordered, "/metadata/leaf/pattern");
        }
    }
}

#[test]
fn admission_relative_refs_ignore_regexes_at_other_bases() {
    let schema = json!({
        "properties": {"p": {"$id": "https://offline.invalid/branch/p", "$ref": "leaf"}},
        "metadata": {
            "target": {"$id": "https://offline.invalid/target"},
            "leaf": {"$id": "https://offline.invalid/branch/leaf", "pattern": "^ok$"},
            "other": {"$id": "https://json-schema.org/leaf", "pattern": "["}
        },
        "$ref": "https://offline.invalid/target"
    });
    let compiled = compile(schema).expect("only the branch resource is selected");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": "ok"})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": "bad"})), Ok(false));
}

#[test]
fn admission_refs_precede_later_applicators() {
    let schema = json!({
        "allOf": [{"$id": "https://offline.invalid/branch/p", "$ref": "leaf"}],
        "metadata": {
            "target": {"$id": "https://offline.invalid/target"},
            "leaf": {"$id": "https://json-schema.org/leaf", "pattern": "["},
            "other": {"$id": "https://offline.invalid/branch/leaf", "pattern": "^ok$"}
        },
        "$ref": "https://offline.invalid/target"
    });
    assert_admission_regex_path(schema, "/metadata/leaf/pattern");
}

#[test]
fn admission_object_keywords_preserve_sibling_resource_order() {
    let schema = json!({
        "properties": {"p": {"$id": "https://offline.invalid/branch/p", "$ref": "leaf"}},
        "metadata": {
            "target": {"$id": "https://offline.invalid/target"},
            "leaf": {"$id": "https://json-schema.org/leaf", "pattern": "["},
            "other": {"$id": "https://offline.invalid/branch/leaf", "pattern": "^ok$"}
        },
        "additionalProperties": {"$ref": "https://offline.invalid/target"}
    });
    assert_admission_regex_path(schema, "/metadata/leaf/pattern");
}

#[test]
fn admission_shared_ref_targets_revisit_lexical_bases() {
    let mut schema = json!({
        "properties": {
            "first": {
                "$id": "https://offline.invalid/first/p",
                "$ref": "https://offline.invalid/missing#/metadata/common"
            },
            "second": {
                "$id": "https://offline.invalid/second/p",
                "$ref": "https://offline.invalid/missing#/metadata/common"
            }
        },
        "metadata": {
            "common": {"$ref": "leaf"},
            "leaf": {"$id": "https://offline.invalid/second/leaf", "pattern": "["}
        }
    });
    // The first call to common has no matching leaf; the second does.
    assert_admission_regex_path(schema.clone(), "/metadata/leaf/pattern");
    schema["metadata"]["leaf"]["pattern"] = json!("^ok$");
    let compiled = compile(schema).expect("valid target in the second lexical base");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"second": "ok"})), Ok(true));
    assert_eq!(compiled.check(&json!({"second": "bad"})), Ok(false));
}

#[test]
fn admission_follows_resolved_and_legacy_retrieved_resources() {
    for reference in [
        "https://offline.invalid/branch/container#/schemas/child",
        "#/metadata/resource/schemas/child",
    ] {
        let schema = json!({
            "properties": {"p": {"$ref": reference}},
            "metadata": {
                "resource": {
                    "$id": "https://offline.invalid/branch/container",
                    "schemas": {"child": {"$ref": "leaf"}}
                },
                "leaf": {"$id": "https://offline.invalid/branch/leaf", "pattern": "["}
            }
        });
        assert_admission_regex_path(schema, "/metadata/leaf/pattern");
    }
}

#[test]
fn admission_keeps_unreferenced_annotations_as_data() {
    let schema = json!({
        "$defs": {"unused": {"pattern": "["}},
        "metadata": {"unused": {"$id": "https://offline.invalid/unused", "pattern": "["}},
        "const": {"pattern": "["},
        "enum": [{"pattern": "["}]
    });
    let compiled = compile(schema).expect("annotation regex text is data");
    assert_eq!(compiled.check(&json!({"pattern": "["})), Ok(true));
    assert_eq!(compiled.check(&json!({"pattern": "ok"})), Ok(false));
    assert_admission_regex_path(
        json!({"anyOf": [true, {"pattern": "["}]}),
        "/anyOf/1/pattern",
    );
}

#[test]
fn admission_preserves_uri_failures_at_check_time() {
    let compiled = compile(json!({"$ref": "http://["})).expect("URI failure is check-time");
    assert!(matches!(
        compiled.check(&json!({})),
        Err(SchemaError::Uri { .. })
    ));
    let compiled = compile(json!({"anyOf": [true, {"$ref": "http://["}]}))
        .expect("unvisited URI failure is not a regex admission error");
    assert_eq!(compiled.check(&json!({})), Ok(true));
}

#[test]
fn admission_growing_relative_id_recursion_is_a_typed_failure() {
    for schema in [
        json!({"$id": "a/", "$ref": "#"}),
        json!({"$id": "a/", "properties": {"p": {"$ref": "leaf"}}, "$ref": "#"}),
    ] {
        assert!(matches!(compile(schema), Err(SchemaError::NoProgress)));
    }
}

#[test]
fn admission_non_http_resource_bases_detect_growing_recursion() {
    for resource in ["file:///a/", "file:///C:/a/", "custom://host/a/"] {
        let schema = json!({
            "$id": resource,
            "properties": {"child": {"$id": "b/", "$ref": "#"}}
        });
        assert!(matches!(compile(schema), Err(SchemaError::NoProgress)));
    }
}

#[test]
fn admission_file_resource_resets_keep_productive_recursion() {
    let compiled = compile(json!({
        "$id": "file:///C:/a/",
        "type": "object",
        "properties": {"p": {
            "$id": "file:///C:/b",
            "$ref": "file:///C:/a/"
        }}
    }))
    .expect("absolute file resource reset gives finite source compilation");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": {}})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": {"p": {}}})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": 1})), Ok(false));

    let compiled = compile(json!({
        "$id": "file:///C:/a/",
        "properties": {"child": {
            "$id": "b/",
            "properties": {
                "p": {"$ref": "leaf#/schemas/t"},
                "q": {"$ref": "leaf#/schemas/t"}
            },
            "$ref": "#"
        }},
        "metadata": {"resource": {"$id": "/a/b/b/leaf", "schemas": {"t": {}}}}
    }))
    .expect("a later rooted file resource can reset growing compilation");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"child": {}})), Ok(true));
    assert_eq!(
        compiled.check(&json!({"child": {"p": 1, "q": false}})),
        Ok(true)
    );
}

#[test]
fn admission_absolute_resets_keep_finite_recursive_compilation() {
    let compiled = compile(json!({
        "$id": "a/",
        "properties": {"p": {
            "$id": "https://offline.invalid/b",
            "$ref": "https://json-schema.org/a/"
        }}
    }))
    .expect("absolute resource reset gives finite source compilation");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": {}})), Ok(false));

    compile(json!({
        "$id": "a/",
        "properties": {
            "p": {"$ref": "leaf#/schemas/t"},
            "q": {"$ref": "leaf#/schemas/t"}
        },
        "metadata": {"resource": {
            "$id": "https://json-schema.org/a/a/leaf",
            "schemas": {"t": {}}
        }},
        "$ref": "#"
    }))
    .expect("a later resolved-resource reset can make source compilation finite");

    let compiled = compile(json!({
        "$id": "https://offline.invalid/root",
        "type": "object",
        "properties": {"value": {"type": "integer"}, "next": {"$ref": "#"}}
    }))
    .expect("productive recursion with an absolute ID remains admitted");
    assert_eq!(
        compiled.check(&json!({"value": 1, "next": {"value": 2}})),
        Ok(true)
    );
    assert_eq!(
        compiled.check(&json!({"value": 1, "next": {"value": "bad"}})),
        Ok(false)
    );
}

#[test]
fn admission_recursive_frames_can_discover_later_regex_targets() {
    for resource in [
        "https://json-schema.org/a/a/leaf",
        "https://json-schema.org/a/a/a/a/a/leaf",
    ] {
        assert_admission_regex_path(
            json!({
                "$id": "a/",
                "properties": {"p": {"$ref": "leaf"}},
                "metadata": {"leaf": {"$id": resource, "pattern": "["}},
                "$ref": "#"
            }),
            "/metadata/leaf/pattern",
        );
    }
    assert_admission_regex_path(
        json!({
            "$id": "file:///C:/a/",
            "properties": {"p": {
                "$id": "b/",
                "properties": {"q": {"$ref": "leaf"}},
                "$ref": "#"
            }},
            "metadata": {"leaf": {"$id": "/a/b/b/b/leaf", "pattern": "["}}
        }),
        "/metadata/leaf/pattern",
    );
}

#[test]
fn admission_resource_ids_compare_node_identity() {
    let mut schema = json!({
        "properties": {"p": {
            "$ref": "https://offline.invalid/branch/resource#/metadata/t"
        }},
        "metadata": {
            "t": {
                "properties": {"same": {"$id": "https://offline.invalid/branch/resource"}},
                "$ref": "leaf"
            },
            "resource": {"$id": "https://offline.invalid/branch/resource"},
            "leaf": {"$id": "https://offline.invalid/branch/leaf", "pattern": "["}
        }
    });
    // Equal JSON values at different nodes are distinct resource identities.
    assert_admission_regex_path(schema.clone(), "/metadata/leaf/pattern");
    schema["metadata"]["leaf"]["pattern"] = json!("^ok$");
    let compiled = compile(schema).expect("valid referenced regex");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": "ok"})), Ok(true));
    assert_eq!(compiled.check(&json!({"p": "bad"})), Ok(false));
}

#[test]
fn admission_overwritten_resolved_targets_preserve_the_sibling_base() {
    let mut schema = json!({
        "properties": {
            "p": {"$ref": "https://offline.invalid/branch/one#/metadata/one/t"},
            "q": {"$ref": "leaf"}
        },
        "metadata": {
            "one": {
                "$id": "https://offline.invalid/branch/one",
                "t": {"$ref": "https://offline.invalid/two#/metadata/one/t"}
            },
            "two": {"$id": "https://offline.invalid/two"},
            "leaf": {"$id": "https://offline.invalid/branch/leaf", "pattern": "["}
        }
    });
    // Source Map.set replaces a target's entry; its later Pop consumes it once.
    assert_admission_regex_path(schema.clone(), "/metadata/leaf/pattern");
    schema["metadata"]["leaf"]["pattern"] = json!("^ok$");
    let compiled = compile(schema).expect("valid referenced regex");
    assert_eq!(compiled.check(&json!({})), Ok(true));
    assert_eq!(compiled.check(&json!({"q": "ok"})), Ok(true));
    assert_eq!(compiled.check(&json!({"q": "bad"})), Ok(false));
}

#[test]
fn admission_dynamic_and_recursive_cycles_can_grow_different_bases() {
    for schema in [
        json!({
            "$id": "a/",
            "$dynamicAnchor": "node",
            "properties": {"p": {"$id": "b/", "$dynamicRef": "#node"}}
        }),
        json!({
            "$id": "a/",
            "$recursiveAnchor": true,
            "properties": {"p": {
                "$id": "b/",
                "$recursiveAnchor": true,
                "$recursiveRef": "#"
            }}
        }),
    ] {
        // Each root re-entry adds a/b/ lexically and a/ to the resource base.
        assert!(matches!(compile(schema), Err(SchemaError::NoProgress)));
    }
}

#[test]
fn admission_growing_cycles_can_complete_auxiliary_ref_functions() {
    for schema in [
        json!({
            "$id": "a/",
            "$dynamicAnchor": "node",
            "properties": {
                "helper": {"$ref": "#/metadata/closed"},
                "p": {"$id": "b/", "$dynamicRef": "#node"}
            },
            "metadata": {"closed": true}
        }),
        json!({
            "$id": "a/",
            "$recursiveAnchor": true,
            "properties": {
                "helper": {"$ref": "#/metadata/closed"},
                "p": {"$id": "b/", "$recursiveAnchor": true, "$recursiveRef": "#"}
            },
            "metadata": {"closed": true}
        }),
    ] {
        let mut finite = schema.clone();
        finite["properties"].as_object_mut().unwrap().remove("p");
        let compiled = compile(finite).expect("the auxiliary ref function returns normally");
        assert_eq!(compiled.check(&json!({"helper": 1})), Ok(true));
        assert!(matches!(compile(schema), Err(SchemaError::NoProgress)));
    }
}

#[test]
fn valid_engine_rejects_malformed_instance_without_errors() {
    // wire-level sanity: type mismatches never surface as errors
    assert!(!valid(json!({"type": "object"}), json!([1])));
    assert!(!valid(json!({"type": "array"}), json!({"a": 1})));
}
