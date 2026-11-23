//! Schema guards, scalar constraints and JavaScript equality.

use serde_json::json;

use super::fixtures::valid;

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
fn valid_engine_rejects_malformed_instance_without_errors() {
    // wire-level sanity: type mismatches never surface as errors
    assert!(!valid(json!({"type": "object"}), json!([1])));
    assert!(!valid(json!({"type": "array"}), json!({"a": 1})));
}
