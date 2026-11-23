//! Compile-time schema admission and resource traversal.

use opi_ai::schema::{SchemaError, compile};
use serde_json::{Value, json};

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
fn visited_applicators_propagate_uri_errors_and_unvisited_branches_do_not() {
    for (name, schema, value) in [
        ("not", json!({"not": {"$ref": "http://["}}), json!({})),
        (
            "if condition",
            json!({"if": {"$ref": "http://["}}),
            json!({}),
        ),
        (
            "matching patternProperties",
            json!({"patternProperties": {"^hit$": {"$ref": "http://["}}}),
            json!({"hit": 1}),
        ),
        (
            "allOf after true",
            json!({"allOf": [true, {"$ref": "http://["}]}),
            json!({}),
        ),
        (
            "first anyOf",
            json!({"anyOf": [{"$ref": "http://["}, true]}),
            json!({}),
        ),
        (
            "selected then",
            json!({"if": true, "then": {"$ref": "http://["}, "else": true}),
            json!({}),
        ),
        (
            "selected else",
            json!({"if": false, "then": true, "else": {"$ref": "http://["}}),
            json!({}),
        ),
    ] {
        let compiled = compile(schema).unwrap_or_else(|error| {
            panic!("{name}: URI failure must remain check-time: {error:?}")
        });
        let outcome = compiled.check(&value);
        assert!(
            matches!(&outcome, Err(SchemaError::Uri { .. })),
            "{name}: visited leaf must propagate the typed URI error, got {outcome:?}"
        );
    }

    for (name, schema, value) in [
        (
            "anyOf short circuit",
            json!({"anyOf": [true, {"$ref": "http://["}]}),
            json!({}),
        ),
        (
            "false if skips then",
            json!({"if": false, "then": {"$ref": "http://["}}),
            json!({}),
        ),
        (
            "true if skips else",
            json!({"if": true, "else": {"$ref": "http://["}}),
            json!({}),
        ),
        (
            "nonmatching patternProperties",
            json!({"patternProperties": {"^hit$": {"$ref": "http://["}}}),
            json!({"miss": 1}),
        ),
    ] {
        let compiled = compile(schema).unwrap_or_else(|error| {
            panic!("{name}: URI failure must remain check-time: {error:?}")
        });
        assert_eq!(compiled.check(&value), Ok(true), "{name}");
    }
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
