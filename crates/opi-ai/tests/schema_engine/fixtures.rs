//! Compile and check helpers shared within the schema_engine target.

use opi_ai::schema::{SchemaError, compile};
use serde_json::Value;

pub(super) fn check(schema: Value, value: Value) -> Result<bool, SchemaError> {
    compile(schema)?.check(&value)
}

pub(super) fn valid(schema: Value, value: Value) -> bool {
    check(schema, value).expect("engine error")
}
