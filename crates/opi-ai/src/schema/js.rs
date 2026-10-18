//! JavaScript value semantics shared by the private schema dialect engine.
//!
//! The pinned upstream (TypeBox 1.3.27, tree SHA `ebf39b14…`, see
//! `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md`)
//! evaluates over already-parsed JavaScript values. This module reproduces the
//! exact JavaScript semantics the dialect depends on: property-key
//! enumeration order, binary64 float semantics, and deep structural equality.
//! Native Rust integers retain their exact value (RDF-09). It is deliberately
//! not a general JSON utility layer.

use std::cmp::Ordering;

use serde_json::{Number, Value};

/// Maximum array-index property key: JS array indices cover `0..=2^32-2`.
const MAX_ARRAY_INDEX: u64 = 4_294_967_294;

/// Returns the array index for `key` when `key` is a canonical JavaScript
/// integer-index property key (`"0"` or `[1-9][0-9]*` without leading zeros,
/// value at most `2^32 - 2`).
pub(crate) fn array_index(key: &str) -> Option<u64> {
    if key.is_empty() || key.len() > 10 {
        return None;
    }
    let bytes = key.as_bytes();
    if bytes[0] == b'0' {
        return if key.len() == 1 { Some(0) } else { None };
    }
    if !bytes.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    if value > MAX_ARRAY_INDEX {
        return None;
    }
    Some(value)
}

/// Returns the object's keys in JavaScript enumeration order: canonical
/// integer-index keys first in ascending numeric order, then the remaining
/// string keys in insertion order (`serde_json/preserve_order` supplies the
/// insertion order).
pub(crate) fn js_keys(map: &serde_json::Map<String, Value>) -> Vec<&str> {
    let mut indexed: Vec<(u64, &str)> = Vec::new();
    let mut strings: Vec<&str> = Vec::new();
    for key in map.keys() {
        match array_index(key) {
            Some(index) => indexed.push((index, key.as_str())),
            None => strings.push(key.as_str()),
        }
    }
    indexed.sort_by_key(|(index, _)| *index);
    indexed
        .into_iter()
        .map(|(_, key)| key)
        .chain(strings)
        .collect()
}

/// Returns `true` when the JSON instance is a JavaScript integer
/// (`Number.isInteger`: finite binary64 with zero fractional part).
pub(crate) fn is_js_integer(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|n| n.is_finite() && n.fract() == 0.0)
}

/// Checks one `type` keyword name against an instance. Unknown names are
/// always-`true` leaves in the upstream engine; names that can never describe
/// a JSON value (`bigint`, `function`, ...) are always false for JSON input.
pub(crate) fn type_matches(name: &str, value: &Value) -> bool {
    match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "boolean" => value.is_boolean(),
        "integer" => is_js_integer(value),
        "number" => value.is_number(),
        "null" => value.is_null(),
        "string" => value.is_string(),
        // xschema names and unknown names: the upstream `CheckTypeName`
        // returns `true` for unrecognized names; JSON instances can never be
        // bigint/constructor/function/symbol/undefined/void, so those names
        // are always false here.
        "bigint" | "constructor" | "function" | "symbol" | "undefined" | "void" => false,
        _ => true,
    }
}

/// The exact value of a native signed or unsigned JSON integer.
pub(crate) fn integer_value(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

/// Compare a native JSON integer with a finite binary64 value without rounding
/// the integer. The integer fits in 64 bits, so saturation of an out-of-range
/// float-to-i128 conversion cannot compare equal to it.
fn integer_float_cmp(integer: i128, float: f64) -> Ordering {
    match integer.cmp(&(float as i128)) {
        Ordering::Equal if float.fract() > 0.0 => Ordering::Less,
        Ordering::Equal if float.fract() < 0.0 => Ordering::Greater,
        ordering => ordering,
    }
}

/// Numeric ordering with exact native integers and binary64 float values.
pub(crate) fn number_cmp(left: &Number, right: &Number) -> Option<Ordering> {
    match (integer_value(left), integer_value(right)) {
        (Some(left), Some(right)) => Some(left.cmp(&right)),
        (Some(left), None) => right.as_f64().map(|right| integer_float_cmp(left, right)),
        (None, Some(right)) => left
            .as_f64()
            .map(|left| integer_float_cmp(right, left).reverse()),
        (None, None) => left.as_f64()?.partial_cmp(&right.as_f64()?),
    }
}

/// JavaScript `===`-based structural equality over JSON values, with the
/// RDF-09 native integer representation difference.
///
/// `1` and `1.0` denote the same value. Native integers beyond `2^53` remain
/// distinct from neighboring integers and rounded floats. `-0.0` equals `0.0`
/// (`===` semantics, not `Object.is`).
pub(crate) fn js_deep_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => number_cmp(a, b) == Some(Ordering::Equal),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Null, Value::Null) => true,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| js_deep_equal(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, x)| b.get(key).is_some_and(|y| js_deep_equal(x, y)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn array_index_keys_follow_canonical_rules() {
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("42"), Some(42));
        assert_eq!(array_index("01"), None);
        assert_eq!(array_index("-1"), None);
        assert_eq!(array_index("4294967294"), Some(4294967294));
        assert_eq!(array_index("4294967295"), None);
        assert_eq!(array_index("a"), None);
        assert_eq!(array_index(""), None);
    }

    #[test]
    fn js_keys_enumerate_integer_indices_first_ascending() {
        let value = json!({"b": 1, "2": 2, "a": 3, "1": 4, "10": 5});
        let keys: Vec<&str> = js_keys(value.as_object().unwrap());
        assert_eq!(keys, vec!["1", "2", "10", "b", "a"]);
    }

    #[test]
    fn js_keys_without_indices_keep_insertion_order() {
        let value = json!({"z": 1, "a": 2});
        let keys: Vec<&str> = js_keys(value.as_object().unwrap());
        assert_eq!(keys, vec!["z", "a"]);
    }

    #[test]
    fn integer_detection_uses_binary64_fraction() {
        assert!(is_js_integer(&json!(3)));
        assert!(is_js_integer(&json!(3.0)));
        assert!(!is_js_integer(&json!(3.5)));
    }

    #[test]
    fn deep_equal_retains_exact_integers_and_numeric_spelling() {
        assert!(js_deep_equal(&json!(1), &json!(1.0)));
        assert!(!js_deep_equal(
            &json!(9007199254740993u64),
            &json!(9007199254740992u64)
        ));
        for (integer, float, equal) in [
            (json!(9007199254740993u64), json!(9007199254740992.0), false),
            (
                json!(-9007199254740993i64),
                json!(-9007199254740992.0),
                false,
            ),
            (json!(u64::MAX), json!(18446744073709551616.0), false),
            (json!(i64::MIN), json!(-9223372036854775808.0), true),
            (json!(i64::MAX), json!(9223372036854775808.0), false),
            (json!(0), json!(-0.0), true),
            (json!(0), json!(0.5), false),
            (json!(0), json!(-0.5), false),
        ] {
            assert_eq!(
                js_deep_equal(&integer, &float),
                equal,
                "{integer} / {float}"
            );
            assert_eq!(
                js_deep_equal(&float, &integer),
                equal,
                "{float} / {integer}"
            );
        }
        assert!(!js_deep_equal(
            &json!({"nested": [9007199254740993u64]}),
            &json!({"nested": [9007199254740992u64]})
        ));
        assert!(!js_deep_equal(&json!(1), &json!(2)));
        assert!(!js_deep_equal(&json!("1"), &json!(1)));
    }

    #[test]
    fn deep_equal_ignores_object_key_order() {
        assert!(js_deep_equal(
            &json!({"a": 1, "b": 2}),
            &json!({"b": 2, "a": 1})
        ));
        assert!(!js_deep_equal(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
    }

    #[test]
    fn unknown_type_names_are_true_leaves() {
        assert!(type_matches("anything", &json!(1)));
        assert!(!type_matches("bigint", &json!(1)));
    }
}
