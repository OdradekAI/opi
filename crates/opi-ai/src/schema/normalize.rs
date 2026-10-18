//! Public tool-argument normalizer/validator seam (DEL-04, task 20.10).
//!
//! Ports the pinned Pi `validateToolArguments` pipeline over the private
//! dialect engine from task 20.9:
//!
//! 1. an owned `arguments_error` marker rejects **before any coercion**
//!    (the transport keeps the raw text; `ToolArgumentOutcome::ArgumentsError`
//!    never grants execution);
//! 2. clone-before-normalize: the caller's raw value is never mutated;
//! 3. `normalizeOptionalNulls` deletes non-required `null` properties whose
//!    independently compiled sub-schema rejects `null` (sub-compile failures
//!    keep the key, `$ref` properties are skipped);
//! 4. TypeBox `Value.Convert` is a no-op for plain JSON dialect schemas (its
//!    dispatch reads the non-JSON `~kind` marker, out of the dialect — a
//!    documented divergence);
//! 5. `coerceWithJsonSchema`: allOf folds in order, anyOf/oneOf try the
//!    original value first and then clone-per-arm checked by independent arm
//!    compiles, type-union primitive coercion in `type`-array order with
//!    exact ECMAScript `Number`/`String` conversions, object `properties`
//!    plus object `additionalProperties`, and tuple/single `items`;
//! 6. the normalized root is **always** checked against the original schema
//!    (RDF-09: the upstream returns unvalidated raw when a coerced root
//!    primitive fails the check; this port reports `Invalid` instead).
//!
//! Invalid outcomes carry redacted failures only (instance pointer, keyword,
//! schema-derived reason); raw argument content never enters diagnostics.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;

use super::js::type_matches;
use super::{CompiledSchema, SchemaError, compile};

/// One redacted validation failure. `instance_path` is a JSON pointer
/// ("" is the root); `reason` carries schema-derived bounds only, never
/// instance content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolArgumentFailure {
    pub instance_path: String,
    pub keyword: String,
    pub reason: String,
}

/// Outcome of validating one tool call's arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolArgumentOutcome {
    /// Coerced and validated: `normalized` passes the original schema. The
    /// caller keeps the untouched raw value it passed in.
    Valid { normalized: Value },
    /// Normalization ran; the normalized value failed the original-schema
    /// check. Failures are redacted. Never grants execution.
    Invalid { failures: Vec<ToolArgumentFailure> },
    /// The transport-owned arguments parse error rejected the call before
    /// any coercion. The raw text stays with the transport.
    ArgumentsError,
}

/// Validate (and normalize) parsed tool-call arguments against one trusted,
/// admitted schema.
///
/// `arguments_error` is the final-parse marker from the message layer; when
/// it is set the call is rejected before coercion without touching the raw
/// text (AIR-048/V-35). The input `arguments` value is never mutated.
pub fn validate_tool_arguments(
    schema: &CompiledSchema,
    arguments: &Value,
    arguments_error: Option<&str>,
) -> Result<ToolArgumentOutcome, SchemaError> {
    if arguments_error.is_some() {
        return Ok(ToolArgumentOutcome::ArgumentsError);
    }
    let root = schema.root_value();
    let mut subs = SubValidators::default();
    let mut args = arguments.clone();
    normalize_optional_nulls(&mut args, root, &mut subs)?;
    let coerced = coerce_with_json_schema(args.clone(), root, &mut subs)?;
    // Upstream identity shortcut: an unchanged value falls straight through
    // to the final check; structural equality is behaviorally equivalent
    // here because equal values produce equal outcomes on both arms.
    let normalized = if coerced == args { args } else { coerced };
    let (valid, failure) = schema.check_detailed(&normalized)?;
    if valid {
        return Ok(ToolArgumentOutcome::Valid { normalized });
    }
    let failures = failure
        .map(|site| {
            vec![ToolArgumentFailure {
                instance_path: site.path,
                keyword: site.keyword.to_owned(),
                reason: site.reason,
            }]
        })
        .unwrap_or_else(|| {
            vec![ToolArgumentFailure {
                instance_path: String::new(),
                keyword: "schema".to_owned(),
                reason: "value failed the original schema check".to_owned(),
            }]
        });
    Ok(ToolArgumentOutcome::Invalid { failures })
}

/// Name-lookup variant mirroring the upstream `validateToolCall`: resolves
/// the tool name against `(name, schema)` pairs and validates its arguments.
/// An unknown tool is a typed rejection, not a validation outcome.
pub fn validate_tool_call<'a>(
    tools: &[(&'a str, &'a CompiledSchema)],
    tool_name: &str,
    arguments: &Value,
    arguments_error: Option<&str>,
) -> Result<Result<ToolArgumentOutcome, String>, SchemaError> {
    let Some((_, schema)) = tools.iter().find(|(name, _)| *name == tool_name) else {
        return Ok(Err(format!("tool \"{tool_name}\" not found")));
    };
    Ok(Ok(validate_tool_arguments(
        schema,
        arguments,
        arguments_error,
    )?))
}

// ------------------------------------------------------------------
// Independent sub-schema compiles (upstream getSubSchemaValidator)
// ------------------------------------------------------------------

/// Per-invocation reuse cache for independently compiled sub-schemas. A
/// compile failure maps to `None` exactly like the upstream `catch` branch;
/// the cache only reuses work and never changes any verdict.
#[derive(Default)]
struct SubValidators {
    cache: HashMap<*const Value, Option<Arc<CompiledSchema>>>,
}

impl SubValidators {
    fn check(&mut self, schema: &Value, value: &Value) -> Result<Option<bool>, SchemaError> {
        match self.get(schema) {
            Some(compiled) => compiled.check(value).map(Some),
            // Optional-null deletion requires an explicit false; union
            // selection requires an explicit true. A failed sub-compile
            // supplies neither, matching upstream optional chaining.
            None => Ok(None),
        }
    }

    fn get(&mut self, schema: &Value) -> Option<&CompiledSchema> {
        let key = schema as *const Value;
        let entry = self
            .cache
            .entry(key)
            .or_insert_with(|| compile(schema.clone()).ok().map(Arc::new));
        entry.as_ref().map(|arc| arc.as_ref())
    }
}

// ------------------------------------------------------------------
// normalizeOptionalNulls
// ------------------------------------------------------------------

fn normalize_optional_nulls(
    value: &mut Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<(), SchemaError> {
    if value.is_array() {
        if let Some(items) = schema.get("items") {
            if let Some(tuple) = items.as_array() {
                let array = value.as_array_mut().expect("checked is_array");
                for (index, item_schema) in tuple.iter().enumerate() {
                    if let Some(item) = array.get_mut(index) {
                        normalize_optional_nulls(item, item_schema, subs)?;
                    }
                }
            } else {
                let array = value.as_array_mut().expect("checked is_array");
                for item in array.iter_mut() {
                    normalize_optional_nulls(item, items, subs)?;
                }
            }
        }
        return Ok(());
    }
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Ok(());
    };
    if !value.is_object() {
        return Ok(());
    }
    let required: HashSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default()
        .into_iter()
        .collect();
    for (key, property_schema) in properties {
        let Some(entry) = value.get(key) else {
            continue;
        };
        let is_ref = matches!(property_schema.get("$ref"), Some(Value::String(_)));
        let delete = entry.is_null()
            && !required.contains(key.as_str())
            && !is_ref
            && subs.check(property_schema, &Value::Null)? == Some(false);
        if delete {
            value
                .as_object_mut()
                .expect("checked is_object")
                .remove(key);
        } else if let Some(child) = value.get_mut(key) {
            normalize_optional_nulls(child, property_schema, subs)?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------------
// coerceWithJsonSchema
// ------------------------------------------------------------------

fn get_schema_types(schema: &Value) -> Vec<&str> {
    match schema.get("type") {
        Some(Value::String(name)) => vec![name.as_str()],
        Some(Value::Array(names)) => names
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>(),
        _ => Vec::new(),
    }
}

fn coerce_with_json_schema(
    value: Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<Value, SchemaError> {
    let mut next = value;
    if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
        for nested in all_of {
            next = coerce_with_json_schema(next, nested, subs)?;
        }
    }
    if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
        next = coerce_with_union_schema(next, any_of, subs)?;
    }
    if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
        next = coerce_with_union_schema(next, one_of, subs)?;
    }
    let types = get_schema_types(schema);
    let matches_union_member = types.len() > 1 && types.iter().any(|t| type_matches(t, &next));
    if !types.is_empty() && !matches_union_member {
        for schema_type in &types {
            let candidate = coerce_primitive_by_type(&next, schema_type);
            if candidate != next {
                next = candidate;
                break;
            }
        }
    }
    if types.contains(&"object") && next.is_object() {
        apply_schema_object_coercion(&mut next, schema, subs)?;
    }
    if types.contains(&"array") && next.is_array() {
        apply_schema_array_coercion(&mut next, schema, subs)?;
    }
    Ok(next)
}

fn coerce_with_union_schema(
    value: Value,
    schemas: &[Value],
    subs: &mut SubValidators,
) -> Result<Value, SchemaError> {
    // Original value first: an arm that already matches wins unchanged.
    for arm in schemas {
        if subs.check(arm, &value)? == Some(true) {
            return Ok(value);
        }
    }
    // Clone-per-arm coercion, first arm whose coerced candidate checks wins.
    for arm in schemas {
        let candidate = coerce_with_json_schema(value.clone(), arm, subs)?;
        if subs.check(arm, &candidate)? == Some(true) {
            return Ok(candidate);
        }
    }
    Ok(value)
}

fn apply_schema_object_coercion(
    value: &mut Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<(), SchemaError> {
    let properties = schema.get("properties").and_then(Value::as_object);
    let defined: HashSet<&str> = properties
        .map(|map| map.keys().map(String::as_str).collect())
        .unwrap_or_default();
    if let Some(properties) = properties {
        for (key, property_schema) in properties {
            if let Some(entry) = value.get_mut(key) {
                let coerced = coerce_with_json_schema(entry.clone(), property_schema, subs)?;
                *entry = coerced;
            }
        }
    }
    if let Some(additional) = schema.get("additionalProperties")
        && additional.is_object()
    {
        let object = value.as_object_mut().expect("caller checked is_object");
        for (key, entry) in object.iter_mut() {
            if !defined.contains(key.as_str()) {
                let coerced = coerce_with_json_schema(entry.clone(), additional, subs)?;
                *entry = coerced;
            }
        }
    }
    Ok(())
}

fn apply_schema_array_coercion(
    value: &mut Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<(), SchemaError> {
    let array = value.as_array_mut().expect("caller checked is_array");
    if let Some(items) = schema.get("items").and_then(Value::as_array) {
        for (index, entry) in array.iter_mut().enumerate() {
            if let Some(item_schema) = items.get(index) {
                let coerced = coerce_with_json_schema(entry.clone(), item_schema, subs)?;
                *entry = coerced;
            }
        }
        return Ok(());
    }
    if let Some(item_schema) = schema.get("items") {
        // Upstream coerces per-item only for an object `items` schema.
        if item_schema.is_object() {
            for entry in array.iter_mut() {
                let coerced = coerce_with_json_schema(entry.clone(), item_schema, subs)?;
                *entry = coerced;
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------
// Primitive coercion (upstream coercePrimitiveByType)
// ------------------------------------------------------------------

fn js_trim_nonempty(text: &str) -> bool {
    // JS String.prototype.trim over the StrWhiteSpace set.
    !super::ecma_number::js_trim(text).is_empty()
}

fn coerce_primitive_by_type(value: &Value, schema_type: &str) -> Value {
    match schema_type {
        "number" => match value {
            Value::Null => Value::Number(0u64.into()),
            Value::String(text) if js_trim_nonempty(text) => {
                let parsed = super::ecma_number::js_to_number(text);
                if parsed.is_finite() {
                    super::ecma_number::js_f64_to_number(parsed)
                } else {
                    value.clone()
                }
            }
            Value::Bool(b) => Value::Number(if *b { 1u64 } else { 0u64 }.into()),
            _ => value.clone(),
        },
        "integer" => match value {
            Value::Null => Value::Number(0u64.into()),
            Value::String(text) if js_trim_nonempty(text) => {
                let parsed = super::ecma_number::js_to_number(text);
                // JS Number.isInteger: finite with no fractional part.
                if parsed.is_finite() && parsed.fract() == 0.0 {
                    super::ecma_number::js_f64_to_number(parsed)
                } else {
                    value.clone()
                }
            }
            Value::Bool(b) => Value::Number(if *b { 1u64 } else { 0u64 }.into()),
            _ => value.clone(),
        },
        "boolean" => match value {
            Value::Null => Value::Bool(false),
            Value::String(text) => match text.as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => value.clone(),
            },
            Value::Number(n) => {
                let is_one =
                    n.as_u64() == Some(1) || n.as_i64() == Some(1) || n.as_f64() == Some(1.0);
                let is_zero =
                    n.as_u64() == Some(0) || n.as_i64() == Some(0) || n.as_f64() == Some(0.0);
                if is_one {
                    Value::Bool(true)
                } else if is_zero {
                    Value::Bool(false)
                } else {
                    value.clone()
                }
            }
            _ => value.clone(),
        },
        "string" => match value {
            Value::Null => Value::String(String::new()),
            Value::Number(_) | Value::Bool(_) => {
                match super::ecma_number::js_value_to_string(value) {
                    Some(text) => Value::String(text),
                    None => value.clone(),
                }
            }
            _ => value.clone(),
        },
        "null" => match value {
            Value::String(text) if text.is_empty() => Value::Null,
            Value::Number(n) if n.as_u64() == Some(0) || n.as_i64() == Some(0) => Value::Null,
            Value::Bool(false) => Value::Null,
            _ => value.clone(),
        },
        _ => value.clone(),
    }
}
