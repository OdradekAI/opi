//! FNV-1a-64 structural value hashing ported from the pinned upstream
//! `system/hashing/hash.mjs` (TypeBox 1.3.27, tree SHA `ebf39b14…`).
//!
//! `uniqueItems` compares items by this hash, not by deep equality; porting
//! the exact function preserves the upstream acceptance set including its
//! collision behavior. Object keys hash in JavaScript `Array#sort()` order
//! (UTF-16 code-unit order), and numbers hash their little-endian binary64
//! bytes.

use serde_json::Value;

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

/// Type-marker bytes (source `ByteMarker`).
const MARK_ARRAY: u8 = 0;
const MARK_BOOLEAN: u8 = 2;
const MARK_NULL: u8 = 6;
const MARK_NUMBER: u8 = 7;
const MARK_OBJECT: u8 = 8;
const MARK_STRING: u8 = 10;

fn op(state: &mut u64, byte: u8) {
    *state ^= u64::from(byte);
    *state = state.wrapping_mul(FNV_PRIME);
}

fn hash_string(state: &mut u64, text: &str) {
    op(state, MARK_STRING);
    for &byte in text.as_bytes() {
        op(state, byte);
    }
}

fn hash_number(state: &mut u64, n: f64) {
    op(state, MARK_NUMBER);
    for byte in n.to_le_bytes() {
        op(state, byte);
    }
}

fn hash_value(state: &mut u64, value: &Value) {
    match value {
        Value::Array(items) => {
            op(state, MARK_ARRAY);
            for item in items {
                hash_value(state, item);
            }
        }
        Value::Object(map) => {
            op(state, MARK_OBJECT);
            // JavaScript default sort compares UTF-16 code units; byte-wise
            // Rust string ordering differs around astral characters.
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_by(|a, b| utf16_cmp(a, b));
            for key in keys {
                hash_string(state, key);
                hash_value(state, &map[key]);
            }
        }
        Value::String(text) => hash_string(state, text),
        Value::Number(n) => hash_number(state, n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => {
            op(state, MARK_BOOLEAN);
            op(state, u8::from(*b));
        }
        Value::Null => op(state, MARK_NULL),
    }
}

/// Lexicographic UTF-16 code-unit comparison (JavaScript string sort order).
fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ia = a.encode_utf16();
    let mut ib = b.encode_utf16();
    loop {
        match (ia.next(), ib.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) => match x.cmp(&y) {
                std::cmp::Ordering::Equal => continue,
                other => return other,
            },
        }
    }
}

/// FNV-1a-64 hash of a JSON value (source `Hash`, hex form not needed here).
pub(crate) fn value_hash(value: &Value) -> u64 {
    let mut state = FNV_OFFSET;
    hash_value(&mut state, value);
    state
}

/// Source `CheckUniqueItems`: every item must have a distinct hash.
pub(crate) fn unique_items(items: &[Value]) -> bool {
    let mut seen = std::collections::HashSet::new();
    items.iter().all(|item| seen.insert(value_hash(item)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn integer_and_float_spelling_hash_equal() {
        // JavaScript sees 1 and 1.0 as the same binary64 value.
        assert_eq!(value_hash(&json!(1)), value_hash(&json!(1.0)));
    }

    #[test]
    fn distinct_scalars_hash_distinct() {
        assert_ne!(value_hash(&json!(1)), value_hash(&json!(2)));
        assert_ne!(value_hash(&json!("1")), value_hash(&json!(1)));
        assert_ne!(value_hash(&json!(true)), value_hash(&json!(1)));
        assert_ne!(value_hash(&json!(null)), value_hash(&json!(0)));
    }

    #[test]
    fn object_key_order_does_not_change_the_hash() {
        assert_eq!(
            value_hash(&json!({"a": 1, "b": 2})),
            value_hash(&json!({"b": 2, "a": 1}))
        );
    }

    #[test]
    fn unique_items_detects_duplicates_across_spelling() {
        assert!(unique_items(&[json!(1), json!(2)]));
        assert!(!unique_items(&[json!(1), json!(1.0)]));
        assert!(!unique_items(&[json!({"a": 1}), json!({"a": 1})]));
        assert!(unique_items(&[json!({"a": 1}), json!({"a": 2})]));
    }
}
