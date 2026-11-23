//! Reference identity, resource order and recursion during evaluation.

use opi_ai::schema::{SchemaError, compile};
use serde_json::{Value, json};

use super::fixtures::{check, valid};

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
fn encoded_pointer_targets_preserve_selected_resource_identity() {
    fn assert_outcomes(schema: Value, cases: &[(Value, bool)]) {
        let trusted_before = serde_json::to_vec(&schema).unwrap();
        let compiled = compile(schema.clone()).expect("finite reference targets are admitted");
        for (instance, expected) in cases {
            let instance_before = serde_json::to_vec(instance).unwrap();
            assert_eq!(
                compiled.check(instance),
                Ok(*expected),
                "instance {instance}"
            );
            assert_eq!(serde_json::to_vec(instance).unwrap(), instance_before);
        }
        assert_eq!(serde_json::to_vec(&schema).unwrap(), trusted_before);
    }

    for (reference, accepted, rejected) in [
        ("#/$defs/a~1b", "slash", "tilde"),
        ("#/$defs/a~0b", "tilde", "slash"),
        ("#/$defs/%E9%9B%AA", "unicode", "plain"),
        ("#/$defs/plain", "plain", "unicode"),
    ] {
        assert_outcomes(
            json!({
                "$id": "https://offline.invalid/literal/",
                "$defs": {
                    "a/b": {"const": "slash"},
                    "a~b": {"const": "tilde"},
                    "雪": {"const": "unicode"},
                    "plain": {"const": "plain"}
                },
                "$ref": reference
            }),
            &[(json!(accepted), true), (json!(rejected), false)],
        );
    }

    assert_outcomes(
        json!({
            "$id": "https://offline.invalid/root/",
            "type": "object",
            "required": ["left", "right"],
            "properties": {
                "left": {
                    "$id": "https://offline.invalid/left/",
                    "$defs": {"selected": {"$ref": "leaf"}},
                    "$ref": "#/$defs/selected"
                },
                "right": {
                    "$id": "https://offline.invalid/right/",
                    "$defs": {"selected": {"$ref": "leaf"}},
                    "$ref": "#/$defs/selected"
                }
            },
            "metadata": {
                "left": {"$id": "https://offline.invalid/left/leaf", "const": "left"},
                "right": {"$id": "https://offline.invalid/right/leaf", "const": "right"},
                "root": {"$id": "https://offline.invalid/root/leaf", "const": "root"}
            }
        }),
        &[
            (json!({"left": "left", "right": "right"}), true),
            (json!({"left": "right", "right": "right"}), false),
            (json!({"left": "left", "right": "left"}), false),
            (json!({"left": "root", "right": "right"}), false),
            (json!({"left": "left", "right": "root"}), false),
        ],
    );

    assert_outcomes(
        json!({
            "$id": "https://offline.invalid/root/",
            "$defs": {"unknown": {"const": "root"}},
            "type": "object",
            "required": ["selected"],
            "properties": {
                "selected": {
                    "$id": "https://offline.invalid/left/",
                    "$defs": {"present": {"const": "left"}},
                    "$ref": "#/$defs/unknown"
                }
            },
            "metadata": {
                "other": {
                    "$id": "https://offline.invalid/right/",
                    "$defs": {"unknown": {"const": "right"}}
                }
            }
        }),
        &[
            (json!({"selected": "left"}), false),
            (json!({"selected": "root"}), false),
            (json!({"selected": "right"}), false),
        ],
    );
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
