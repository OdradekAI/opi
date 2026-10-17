//! Structural keyword guards ported from the pinned TypeBox 1.3.27
//! `build/schema/types/*.mjs` (tree SHA `ebf39b14…`).
//!
//! A keyword whose guard fails is inert for that schema node — it neither
//! constrains the instance nor errors. JSON input cannot carry BigInt,
//! RegExp, symbol or function values, so the upstream guards that accept
//! those types reduce to their number/string/object forms here.

use serde_json::Value;

fn is_finite_number(value: &Value) -> bool {
    value.as_f64().is_some_and(|n| n.is_finite())
}

fn is_schema(value: &Value) -> bool {
    matches!(value, Value::Object(_) | Value::Bool(_))
}

fn is_string_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_string))
}

fn is_schema_array(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(is_schema))
}

fn key<'a>(schema: &'a Value, name: &str) -> Option<&'a Value> {
    schema.as_object().and_then(|map| map.get(name))
}

macro_rules! presence_guard {
    ($name:ident, $keyword:literal) => {
        pub(crate) fn $name(schema: &Value) -> bool {
            key(schema, $keyword).is_some()
        }
    };
}

macro_rules! number_guard {
    ($name:ident, $keyword:literal) => {
        pub(crate) fn $name(schema: &Value) -> bool {
            key(schema, $keyword).is_some_and(is_finite_number)
        }
    };
}

macro_rules! schema_guard {
    ($name:ident, $keyword:literal) => {
        pub(crate) fn $name(schema: &Value) -> bool {
            key(schema, $keyword).is_some_and(is_schema)
        }
    };
}

/// Object-valued guard (`Guard.IsObject` semantics: every value is a schema).
macro_rules! schema_guard_object {
    ($name:ident, $keyword:literal) => {
        pub(crate) fn $name(schema: &Value) -> bool {
            key(schema, $keyword).is_some_and(|value| {
                value
                    .as_object()
                    .is_some_and(|map| map.values().all(is_schema))
            })
        }
    };
}

// boolean/type/const/enum
presence_guard!(is_const, "const");
pub(crate) fn is_enum(schema: &Value) -> bool {
    key(schema, "enum").is_some_and(Value::is_array)
}
pub(crate) fn is_type(schema: &Value) -> bool {
    match key(schema, "type") {
        Some(Value::String(_)) => true,
        Some(Value::Array(items)) => items.iter().all(Value::is_string),
        _ => false,
    }
}
// object keywords
schema_guard!(is_additional_properties, "additionalProperties");
pub(crate) fn is_dependencies(schema: &Value) -> bool {
    key(schema, "dependencies").is_some_and(|deps| {
        deps.as_object().is_some_and(|map| {
            map.values()
                .all(|value| is_schema(value) || is_string_array(value))
        })
    })
}
pub(crate) fn is_dependent_required(schema: &Value) -> bool {
    key(schema, "dependentRequired").is_some_and(|deps| {
        deps.as_object()
            .is_some_and(|map| map.values().all(is_string_array))
    })
}
schema_guard_object!(is_dependent_schemas, "dependentSchemas");

pub(crate) fn is_pattern_properties(schema: &Value) -> bool {
    key(schema, "patternProperties").is_some_and(Value::is_object)
}
pub(crate) fn is_properties(schema: &Value) -> bool {
    key(schema, "properties").is_some_and(Value::is_object)
}
pub(crate) fn is_property_names(schema: &Value) -> bool {
    key(schema, "propertyNames").is_some_and(|value| value.is_object() || is_schema(value))
}
number_guard!(is_min_properties, "minProperties");
number_guard!(is_max_properties, "maxProperties");
pub(crate) fn is_required(schema: &Value) -> bool {
    key(schema, "required").is_some_and(is_string_array)
}
schema_guard!(is_unevaluated_properties, "unevaluatedProperties");

// array keywords
schema_guard!(is_additional_items, "additionalItems");
schema_guard!(is_contains, "contains");
pub(crate) fn is_items(schema: &Value) -> bool {
    key(schema, "items").is_some_and(|value| is_schema(value) || is_schema_array(value))
}
pub(crate) fn is_items_sized(schema: &Value) -> bool {
    key(schema, "items").is_some_and(Value::is_array)
}
number_guard!(is_max_contains, "maxContains");
number_guard!(is_max_items, "maxItems");
number_guard!(is_min_contains, "minContains");
number_guard!(is_min_items, "minItems");
pub(crate) fn is_prefix_items(schema: &Value) -> bool {
    key(schema, "prefixItems").is_some_and(is_schema_array)
}
pub(crate) fn is_unique_items(schema: &Value) -> bool {
    key(schema, "uniqueItems").is_some_and(Value::is_boolean)
}
schema_guard!(is_unevaluated_items, "unevaluatedItems");

// string keywords
number_guard!(is_max_length, "maxLength");
number_guard!(is_min_length, "minLength");
pub(crate) fn is_format(schema: &Value) -> bool {
    key(schema, "format").is_some_and(Value::is_string)
}
pub(crate) fn is_pattern(schema: &Value) -> bool {
    key(schema, "pattern").is_some_and(Value::is_string)
}

// number keywords
number_guard!(is_minimum, "minimum");
number_guard!(is_maximum, "maximum");
number_guard!(is_exclusive_minimum, "exclusiveMinimum");
number_guard!(is_exclusive_maximum, "exclusiveMaximum");
pub(crate) fn is_multiple_of(schema: &Value) -> bool {
    key(schema, "multipleOf").is_some_and(is_finite_number)
}

// reference keywords
pub(crate) fn is_ref(schema: &Value) -> bool {
    key(schema, "$ref").is_some_and(Value::is_string)
}
pub(crate) fn is_recursive_ref(schema: &Value) -> bool {
    key(schema, "$recursiveRef").is_some_and(Value::is_string)
}
pub(crate) fn is_dynamic_ref(schema: &Value) -> bool {
    key(schema, "$dynamicRef").is_some_and(Value::is_string)
}
pub(crate) fn is_id(schema: &Value) -> bool {
    key(schema, "$id").is_some_and(Value::is_string)
}
pub(crate) fn is_anchor(schema: &Value) -> bool {
    key(schema, "$anchor").is_some_and(Value::is_string)
}
pub(crate) fn is_dynamic_anchor(schema: &Value) -> bool {
    key(schema, "$dynamicAnchor").is_some_and(Value::is_string)
}
pub(crate) fn is_recursive_anchor_true(schema: &Value) -> bool {
    key(schema, "$recursiveAnchor").is_some_and(|v| v == &Value::Bool(true))
}

// combinator keywords
schema_guard!(is_if, "if");
schema_guard!(is_then, "then");
schema_guard!(is_else, "else");
schema_guard!(is_not, "not");
pub(crate) fn is_all_of(schema: &Value) -> bool {
    key(schema, "allOf").is_some_and(is_schema_array)
}
pub(crate) fn is_any_of(schema: &Value) -> bool {
    key(schema, "anyOf").is_some_and(is_schema_array)
}
pub(crate) fn is_one_of(schema: &Value) -> bool {
    key(schema, "oneOf").is_some_and(is_schema_array)
}

// accessor helpers shared with the evaluator
pub(crate) fn get<'a>(schema: &'a Value, name: &str) -> Option<&'a Value> {
    schema.as_object().and_then(|map| map.get(name))
}

pub(crate) fn get_str<'a>(schema: &'a Value, name: &str) -> Option<&'a str> {
    get(schema, name).and_then(Value::as_str)
}

pub(crate) fn get_f64(schema: &Value, name: &str) -> Option<f64> {
    get(schema, name).and_then(Value::as_f64)
}

/// The required list when the mechanism's guard holds (array of strings).
pub(crate) fn required_list(schema: &Value) -> Option<&Vec<Value>> {
    get(schema, "required")
        .and_then(Value::as_array)
        .filter(|items| items.iter().all(Value::is_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn malformed_numeric_limits_are_inert() {
        assert!(!is_min_length(&json!({"minLength": "2"})));
        assert!(is_min_length(&json!({"minLength": 2})));
        assert!(is_min_length(&json!({"minLength": 2.5})));
        assert!(is_min_length(&json!({"minLength": -1})));
        assert!(!is_minimum(&json!({"minimum": "10"})));
        assert!(!is_min_properties(&json!({"minProperties": [1]})));
    }

    #[test]
    fn boolean_subschemas_are_schemas() {
        assert!(is_additional_properties(
            &json!({"additionalProperties": false})
        ));
        assert!(is_additional_properties(
            &json!({"additionalProperties": {}})
        ));
        assert!(!is_additional_properties(
            &json!({"additionalProperties": "no"})
        ));
        assert!(!is_additional_properties(
            &json!({"additionalProperties": [true]})
        ));
    }

    #[test]
    fn pattern_guard_requires_a_string() {
        assert!(is_pattern(&json!({"pattern": "^a"})));
        assert!(!is_pattern(&json!({"pattern": false})));
        assert!(!is_pattern(&json!({"pattern": 1})));
    }

    #[test]
    fn required_is_present_but_maybe_invalid() {
        // presence and validity differ: an invalid required stays inert.
        assert!(is_const(&json!({"const": {"nested": 1}})));
        assert!(!is_required(&json!({"required": [1]})));
        assert!(is_required(&json!({"required": ["a"]})));
    }

    #[test]
    fn type_guard_accepts_string_or_all_string_array() {
        assert!(is_type(&json!({"type": "object"})));
        assert!(is_type(&json!({"type": ["object", "null"]})));
        assert!(!is_type(&json!({"type": ["object", 1]})));
        assert!(!is_type(&json!({"type": 1})));
    }
}
