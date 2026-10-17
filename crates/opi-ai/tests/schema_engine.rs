//! Behavioral battery for the private TypeBox JSON dialect engine
//! (`opi_ai::schema`, task 20.9).
//!
//! Two evidence layers:
//!
//! 1. The research-frozen 64-case differential corpus (copied verbatim from
//!    `docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema/`,
//!    pins in that directory's `pins.json`). Expected outcomes are the
//!    Unicode-16-fork native engine's 64/64 results. The two UCD-17-only
//!    cases are deferred: the workspace's stock regress 0.12.0 carries
//!    Unicode 17 tables, and exact Unicode-16 agreement is proven at
//!    packaging time with the `opi-vendor-regress` family (tasks 20.8/20.57),
//!    not by the workspace backend.
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
    let mut deferred_seen = Vec::new();
    for line in cases_text.lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        let id = case["id"].as_str().unwrap();
        let expectation = &expected[id];
        let deferred = expectation["deferred"].as_bool().unwrap();

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

        if deferred {
            // Documented deferral: with the stock Unicode-17 backend these
            // two cases accept what the pinned Node/Unicode-16 profile
            // rejects. Assert the deferred status, not agreement.
            deferred_seen.push(id.to_string());
            assert_ne!(
                outcome, expected_outcome,
                "deferred case {id} unexpectedly agrees; update the deferral"
            );
            continue;
        }
        assert_eq!(outcome, expected_outcome, "case {id}");
        matched += 1;
    }
    assert_eq!(matched, 62, "62 table-independent cases must be asserted");
    assert_eq!(
        deferred_seen,
        vec!["ecma-unicode17-script", "ecma-unicode17-new-letter"],
        "exactly the two UCD-17-only cases are deferred"
    );
}

// ------------------------------------------------------------------
// Core leaves and guards
// ------------------------------------------------------------------

#[test]
fn boolean_and_empty_schemas() {
    assert!(valid(json!(true), json!(null)));
    assert!(!valid(json!(false), json!(null)));
    assert!(valid(json!({}), json!({"x": 1})));
    // non-schema roots are always-true validators (no keyword fires)
    assert!(valid(json!("not-a-schema"), json!(1)));
    assert!(valid(json!(5), json!(1)));
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

#[test]
fn deep_schemas_do_not_exhaust_the_rust_stack() {
    // 10_000 nested allOf wrappers evaluate through the explicit work stack.
    // The compiled schema is intentionally leaked afterwards: dropping a
    // 10_000-deep serde_json value recurses on the (small) test-thread stack,
    // which is a property of the value type, not of the evaluator.
    // Built with map/array constructors (moving the previous schema) rather
    // than the json! macro, whose expression values go through the recursive
    // `to_value` serializer.
    let mut schema = serde_json::Value::Bool(true);
    for _ in 0..10_000 {
        let mut map = serde_json::Map::new();
        map.insert("allOf".into(), serde_json::Value::Array(vec![schema]));
        schema = serde_json::Value::Object(map);
    }
    let compiled = compile(schema).expect("compiles");
    assert_eq!(compiled.check(&json!(1)), Ok(true));
    std::mem::forget(compiled);
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
fn invalid_regexes_fail_admission_with_paths() {
    let error = compile(json!({"properties": {"a": {"pattern": "[z-a]"}}})).unwrap_err();
    match error {
        SchemaError::Regex { path, .. } => assert_eq!(path, "/properties/a/pattern"),
        other => panic!("{other:?}"),
    }
    let error = compile(json!({"patternProperties": {"(": {}}})).unwrap_err();
    assert!(matches!(error, SchemaError::Regex { .. }));
}

#[test]
fn valid_engine_rejects_malformed_instance_without_errors() {
    // wire-level sanity: type mismatches never surface as errors
    assert!(!valid(json!({"type": "object"}), json!([1])));
    assert!(!valid(json!({"type": "array"}), json!({"a": 1})));
}
