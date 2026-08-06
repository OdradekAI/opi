//! JSON Schema validation for tool arguments.

/// Error produced when arguments fail schema validation.
#[derive(Debug, thiserror::Error)]
#[error("schema validation failed: {}", errors.join(", "))]
pub struct ValidationError {
    pub errors: Vec<String>,
}

/// Validate `args` against a JSON Schema.
pub fn validate(
    schema: &serde_json::Value,
    args: &serde_json::Value,
) -> Result<(), ValidationError> {
    match jsonschema::validate(schema, args) {
        Ok(()) => Ok(()),
        Err(e) => Err(ValidationError {
            errors: vec![e.masked_with("<redacted>").to_string()],
        }),
    }
}
