//! Array and object keyword contracts.

use opi_ai::schema::compile;
use serde_json::json;

use super::fixtures::valid;

#[test]
fn cardinality_min_properties_boundaries() {
    let bounded = compile(json!({"type": "object", "minProperties": 2})).unwrap();
    let inert = compile(json!({"type": "object", "minProperties": "2"})).unwrap();
    for (value, expected) in [
        (json!({}), false),
        (json!({"a": 1}), false),
        (json!({"a": 1, "b": 2}), true),
        (json!({"a": 1, "b": 2, "c": 3}), true),
    ] {
        assert_eq!(bounded.check(&value), Ok(expected), "instance {value}");
        assert_eq!(inert.check(&value), Ok(true), "string limit: {value}");
    }
}

#[test]
fn cardinality_max_properties_boundaries() {
    let bounded = compile(json!({"type": "object", "maxProperties": 2})).unwrap();
    let inert = compile(json!({"type": "object", "maxProperties": "2"})).unwrap();
    for (value, expected) in [
        (json!({}), true),
        (json!({"a": 1}), true),
        (json!({"a": 1, "b": 2}), true),
        (json!({"a": 1, "b": 2, "c": 3}), false),
    ] {
        assert_eq!(bounded.check(&value), Ok(expected), "instance {value}");
        assert_eq!(inert.check(&value), Ok(true), "string limit: {value}");
    }
}

#[test]
fn cardinality_min_items_boundaries() {
    let bounded = compile(json!({"type": "array", "minItems": 2})).unwrap();
    let inert = compile(json!({"type": "array", "minItems": "2"})).unwrap();
    for (value, expected) in [
        (json!([]), false),
        (json!([1]), false),
        (json!([1, 2]), true),
        (json!([1, 2, 3]), true),
    ] {
        assert_eq!(bounded.check(&value), Ok(expected), "instance {value}");
        assert_eq!(inert.check(&value), Ok(true), "string limit: {value}");
    }
}

#[test]
fn cardinality_max_items_boundaries() {
    let bounded = compile(json!({"type": "array", "maxItems": 2})).unwrap();
    let inert = compile(json!({"type": "array", "maxItems": "2"})).unwrap();
    for (value, expected) in [
        (json!([]), true),
        (json!([1]), true),
        (json!([1, 2]), true),
        (json!([1, 2, 3]), false),
    ] {
        assert_eq!(bounded.check(&value), Ok(expected), "instance {value}");
        assert_eq!(inert.check(&value), Ok(true), "string limit: {value}");
    }
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
