//! Schema generation: determinism, structural keyword properties
//! (`contentEncoding`, `minLength`, NativeString string shape), the reviewed
//! insta snapshot, and fixture validation against the normative schema.

use opi_protocol::execution::v1;

fn fixtures_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn load_json(name: &str) -> serde_json::Value {
    let path = fixtures_dir().join(name);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading fixture {name}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing fixture {name}: {e}"))
}

#[test]
fn schema_snapshot_is_reviewed() {
    // The snapshot is a REVIEWED artifact (see README): it must not be updated
    // via INSTA_UPDATE without human review.
    let pretty = serde_json::to_string_pretty(&v1::schema()).unwrap();
    insta::assert_snapshot!("schema_v1", pretty);
}

#[test]
fn schema_is_deterministic_across_calls() {
    assert_eq!(v1::schema(), v1::schema());
}

#[test]
fn schema_carries_id_and_wire_identity() {
    let s = v1::schema();
    assert_eq!(s["$id"], serde_json::json!(v1::SCHEMA_ID_URL));
    assert!(s["$comment"].as_str().unwrap().contains(v1::WIRE_IDENTITY));
    assert!(s.get("oneOf").is_some(), "root must be a oneOf");
    assert!(s.get("$defs").is_some(), "$defs must be present");
}

#[test]
fn schema_omits_internal_root_title() {
    assert!(
        v1::schema().get("title").is_none(),
        "the internal SchemaRoot wrapper must not leak into the wire schema"
    );
}

#[test]
fn nativestring_schema_is_string_with_escape_description() {
    let s = v1::schema();
    let defs = s["$defs"].as_object().unwrap();
    let ns = &defs["NativeString"];
    assert_eq!(
        ns["type"], "string",
        "NativeString must be a string, not array"
    );
    assert!(
        ns["description"].as_str().unwrap().contains("U+E000"),
        "NativeString description must document the escape"
    );
}

#[test]
fn base64_schema_has_content_encoding() {
    let s = v1::schema();
    let defs = s["$defs"].as_object().unwrap();
    assert_eq!(defs["Base64Bytes"]["contentEncoding"], "base64");
    assert_eq!(defs["Base64Bytes"]["type"], "string");
}

#[test]
fn request_id_schema_has_min_length() {
    let s = v1::schema();
    let defs = s["$defs"].as_object().unwrap();
    assert_eq!(defs["RequestId"]["minLength"], 1);
}

#[test]
fn implementation_id_schema_has_min_length() {
    let s = v1::schema();
    let defs = s["$defs"].as_object().unwrap();
    assert_eq!(defs["ImplementationId"]["type"], "string");
    assert_eq!(defs["ImplementationId"]["minLength"], 1);
}

#[test]
fn protocol_id_schema_has_min_length() {
    let s = v1::schema();
    let defs = s["$defs"].as_object().unwrap();
    assert_eq!(defs["ProtocolId"]["type"], "string");
    assert_eq!(defs["ProtocolId"]["minLength"], 1);
}

#[test]
fn failed_message_schema_has_default_diagnostics_bound() {
    let s = v1::schema();
    assert_eq!(
        s["$defs"]["FailedPayload"]["properties"]["message"]["maxLength"],
        v1::Bounds::DEFAULT.max_diagnostics_size
    );
}

#[test]
fn custom_bounds_apply_to_every_diagnostic_message_schema() {
    let bounds = v1::Bounds {
        max_diagnostics_size: 7,
        ..v1::Bounds::DEFAULT
    };
    let s = v1::schema_with_bounds(bounds);
    for pointer in [
        "/$defs/FailedPayload/properties/message/maxLength",
        "/$defs/Diagnostic/properties/message/maxLength",
        "/$defs/DiagnosticPayload/properties/message/maxLength",
    ] {
        assert_eq!(s.pointer(pointer), Some(&serde_json::json!(7)), "{pointer}");
    }
}

#[test]
fn schema_character_limit_defers_multibyte_byte_limit_to_codec() {
    let bounds = v1::Bounds {
        max_line_size: 4096,
        max_decoded_chunk_size: 8,
        max_configuration_size: 16,
        max_diagnostics_size: 4,
        max_cumulative_output: 64,
    };
    let instance = serde_json::json!({
        "type": "diagnostic",
        "payload": { "request_id": "r1", "message": "ééé" }
    });
    assert!(
        jsonschema::validate(&v1::schema_with_bounds(bounds), &instance).is_ok(),
        "three characters fit the schema maxLength"
    );

    let err = v1::Session::new(bounds)
        .unwrap()
        .feed_backend_line(&serde_json::to_vec(&instance).unwrap())
        .unwrap_err();
    assert!(matches!(
        err,
        v1::SessionError::Codec(v1::CodecError::DiagnosticsTooLarge {
            actual: 6,
            limit: 4
        })
    ));
}

#[test]
fn valid_fixtures_validate_against_schema() {
    let schema = v1::schema();
    let valid = [
        "valid_initialize.json",
        "valid_execute.json",
        "valid_cancel.json",
        "valid_ready.json",
        "valid_accepted.json",
        "valid_started.json",
        "valid_stdout.json",
        "valid_stderr.json",
        "valid_diagnostic.json",
        "valid_completed.json",
        "valid_failed.json",
        "valid_execute_nonutf8.json",
    ];
    for name in valid {
        let instance = load_json(name);
        jsonschema::validate(&schema, &instance).unwrap_or_else(|e| {
            panic!("valid fixture {name} rejected by schema: {e}");
        });
    }
}

#[test]
fn invalid_fixtures_rejected_by_schema() {
    let schema = v1::schema();
    let invalid = [
        "invalid_missing_id.json",
        "invalid_mismatched_id.json",
        "invalid_empty_id.json",
        "invalid_unknown_field.json",
        "invalid_unknown_tag.json",
        "invalid_nested_diagnostic_unknown_field.json",
        "invalid_ready_missing_implementation.json",
        "invalid_ready_empty_implementation.json",
        "invalid_ready_unknown_field.json",
        "invalid_initialize_empty_protocol.json",
        "invalid_initialize_unknown_field.json",
    ];
    for name in invalid {
        let instance = load_json(name);
        assert!(
            jsonschema::validate(&schema, &instance).is_err(),
            "invalid fixture {name} must be rejected by the schema"
        );
    }
}
