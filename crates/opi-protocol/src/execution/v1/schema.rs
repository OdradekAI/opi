//! Deterministic JSON Schema generation for the `v1` wire frames.
//!
//! [`schema`] generates a Draft 2020-12 document covering every
//! `command-execution-jsonl-v1` frame in both wire directions, stamped with a
//! stable `$id`, a root `description`, and a `$comment` carrying the literal
//! wire identity. The output is byte-stable for a fixed `schemars` version; the
//! reviewed insta snapshot under `tests/snapshots/` pins it and must not be
//! updated via `INSTA_UPDATE` without human review.

use schemars::JsonSchema;
use schemars::generate::SchemaSettings;
use serde_json::{Value, json};

use super::WIRE_IDENTITY;
use super::bounds::Bounds;
use super::frames::{BackendToHost, HostToBackend};

/// Stable canonical `$id` for the `command-execution-jsonl-v1` schema.
pub const SCHEMA_ID_URL: &str = "https://odradek.ai/schemas/command-execution-jsonl-v1.json";

/// Root `description` for the generated schema.
pub const SCHEMA_DESCRIPTION: &str =
    "JSON Schema for command-execution-jsonl-v1 wire frames (host-to-backend and backend-to-host).";

/// Schema-root wrapper bundling both wire directions so a single generated
/// document describes every frame. Used only for schema generation, not on the
/// wire; its fields are never read  --  they exist only so the `JsonSchema` derive
/// references both enums and lands them under `$defs`.
#[derive(JsonSchema)]
#[allow(dead_code)]
struct SchemaRoot {
    host_to_backend: HostToBackend,
    backend_to_host: BackendToHost,
}

/// Generate the normative JSON Schema for the `v1` frames.
///
/// The root is a `oneOf` over the two wire directions; each direction's schema
/// (and all sub-types) live under `$defs`. `NativeString` appears as
/// `{"type":"string"}` (with the escape-convention description), and
/// `Base64Bytes` carries `contentEncoding: "base64"`, because both use manual
/// `JsonSchema` impls.
pub fn schema() -> Value {
    schema_with_bounds(Bounds::DEFAULT)
}

/// Generate the normative JSON Schema with message lengths derived from
/// `bounds`.
///
/// JSON Schema `maxLength` counts Unicode characters, while the codec's
/// `max_diagnostics_size` enforcement counts UTF-8 bytes. The schema bound is
/// therefore a necessary character-count limit; the codec remains authoritative
/// for multibyte messages.
pub fn schema_with_bounds(bounds: Bounds) -> Value {
    let generated = SchemaSettings::draft2020_12()
        .into_generator()
        .into_root_schema_for::<SchemaRoot>();
    let mut value = serde_json::to_value(&generated).expect("generated schema is serializable");
    let obj = value
        .as_object_mut()
        .expect("generated schema root is an object");

    // Rewrite the object-typed wrapper root into a oneOf over the two
    // directions; $defs (already populated by into_root_schema_for) is retained.
    obj.remove("type");
    obj.remove("properties");
    obj.remove("required");
    obj.remove("additionalProperties");
    obj.remove("title");

    let definitions = obj
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
        .expect("generated schema has definitions");
    for definition in ["FailedPayload", "Diagnostic", "DiagnosticPayload"] {
        let message = definitions
            .get_mut(definition)
            .and_then(Value::as_object_mut)
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(Value::as_object_mut)
            .and_then(|properties| properties.get_mut("message"))
            .and_then(Value::as_object_mut)
            .expect("diagnostic message schema is an object");
        message.insert("maxLength".to_string(), json!(bounds.max_diagnostics_size));
    }
    obj.insert(
        "oneOf".to_string(),
        json!([
            { "$ref": "#/$defs/HostToBackend" },
            { "$ref": "#/$defs/BackendToHost" }
        ]),
    );
    obj.insert("$id".to_string(), json!(SCHEMA_ID_URL));
    obj.insert("description".to_string(), json!(SCHEMA_DESCRIPTION));
    obj.insert(
        "$comment".to_string(),
        json!(format!("wire identity: {WIRE_IDENTITY}")),
    );
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_deterministic() {
        assert_eq!(schema(), schema());
    }

    #[test]
    fn schema_has_id_wire_identity_and_oneof() {
        let s = schema();
        assert_eq!(s["$id"], json!(SCHEMA_ID_URL));
        assert!(s["$comment"].as_str().unwrap().contains(WIRE_IDENTITY));
        assert!(s.get("oneOf").is_some(), "root must be a oneOf");
        assert!(s.get("$defs").is_some(), "$defs must be present");
    }
}
