//! Tool-argument validation through the opi-ai private schema dialect.
//!
//! The agent consumes the SDK-owned normalizer/validator (task 20.10): a
//! clone of the final parsed arguments is normalized (optional-null
//! deletion, limited ECMAScript coercion per the pinned Pi corpus) and the
//! normalized root is checked against the registration's original schema.
//! The exact validated value is the value authorized and executed; failures
//! are redacted (instance pointer, keyword, schema-derived reason only).
//!
//! The compiled IR is generation-owned by the tool registration
//! (`RegisteredTool::schema_validation`); this type only wraps the
//! compile-once result.

use opi_ai::schema::{ToolArgumentOutcome, validate_tool_arguments};

/// Error produced when arguments fail schema validation, or the trusted
/// schema itself cannot be admitted by the dialect engine (fail-closed).
#[derive(Debug, thiserror::Error)]
#[error("schema validation failed: {}", errors.join(", "))]
pub struct ValidationError {
    pub errors: Vec<String>,
}

/// Compile-once schema IR for one tool registration.
pub struct ToolSchemaIr {
    compiled: Result<opi_ai::schema::CompiledSchema, String>,
}

impl ToolSchemaIr {
    /// Admit the trusted schema into the dialect IR. An admission failure
    /// (for example a structurally selected invalid regex) is retained and
    /// fails every validation call closed instead of silently accepting.
    pub fn compile(schema: &serde_json::Value) -> Self {
        Self {
            compiled: opi_ai::schema::compile(schema.clone()).map_err(|error| error.to_string()),
        }
    }

    /// Normalize and validate `args` against the admitted schema. Returns
    /// the normalized value on success; the input is never mutated.
    pub fn validate(&self, args: &serde_json::Value) -> Result<serde_json::Value, ValidationError> {
        let compiled = match &self.compiled {
            Ok(compiled) => compiled,
            Err(reason) => {
                return Err(ValidationError {
                    errors: vec![format!("schema admission failed: {reason}")],
                });
            }
        };
        match validate_tool_arguments(compiled, args, None) {
            Ok(ToolArgumentOutcome::Valid { normalized }) => Ok(normalized),
            Ok(ToolArgumentOutcome::Invalid { failures }) => Err(ValidationError {
                errors: failures
                    .iter()
                    .map(|failure| {
                        let path = if failure.instance_path.is_empty() {
                            "root".to_owned()
                        } else {
                            failure.instance_path.clone()
                        };
                        format!("{path}: {}: {}", failure.keyword, failure.reason)
                    })
                    .collect(),
            }),
            // Unreachable through the agent loop (its parse layer rejects
            // malformed arguments before preflight); keep the arm closed.
            Ok(ToolArgumentOutcome::ArgumentsError) => Err(ValidationError {
                errors: vec!["arguments were rejected before coercion".to_owned()],
            }),
            Err(error) => Err(ValidationError {
                errors: vec![format!("schema engine failure: {error}")],
            }),
        }
    }
}
