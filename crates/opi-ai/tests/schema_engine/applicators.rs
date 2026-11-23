//! Branch selection and propagation of evaluated properties and items.

use serde_json::json;

use super::fixtures::valid;

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
