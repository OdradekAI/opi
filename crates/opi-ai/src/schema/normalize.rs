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
            .or_insert_with(|| compile(super::clone_value(schema)).ok().map(Arc::new));
        entry.as_ref().map(|arc| arc.as_ref())
    }
}

// ------------------------------------------------------------------
// normalizeOptionalNulls
// ------------------------------------------------------------------

#[derive(Clone)]
enum ValueSlot {
    Key(String),
    Index(usize),
}

fn take_child(value: &mut Value, slot: &ValueSlot) -> Value {
    match slot {
        ValueSlot::Key(key) => std::mem::take(value.get_mut(key).expect("selected property")),
        ValueSlot::Index(index) => std::mem::take(value.get_mut(*index).expect("selected item")),
    }
}

fn put_child(value: &mut Value, slot: &ValueSlot, child: Value) {
    match slot {
        ValueSlot::Key(key) => *value.get_mut(key).expect("selected property") = child,
        ValueSlot::Index(index) => *value.get_mut(*index).expect("selected item") = child,
    }
}

type NullChild<'a> = (ValueSlot, &'a Value, bool);

enum NullWork<'a> {
    Visit(&'a Value),
    Children {
        parent: Value,
        children: Vec<NullChild<'a>>,
        next: usize,
    },
    Restore {
        parent: Value,
        children: Vec<NullChild<'a>>,
        index: usize,
    },
}

fn optional_null_children<'a>(value: &Value, schema: &'a Value) -> Vec<NullChild<'a>> {
    if let Some(array) = value.as_array() {
        return match schema.get("items") {
            Some(Value::Array(tuple)) => tuple
                .iter()
                .take(array.len())
                .enumerate()
                .map(|(index, schema)| (ValueSlot::Index(index), schema, false))
                .collect(),
            Some(items) => (0..array.len())
                .map(|index| (ValueSlot::Index(index), items, false))
                .collect(),
            None => Vec::new(),
        };
    }
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Vec::new();
    };
    if !value.is_object() {
        return Vec::new();
    }
    let required: HashSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    properties
        .iter()
        .filter(|(key, _)| value.get(*key).is_some())
        .map(|(key, schema)| {
            (
                ValueSlot::Key(key.clone()),
                schema,
                !required.contains(key.as_str())
                    && !matches!(schema.get("$ref"), Some(Value::String(_))),
            )
        })
        .collect()
}

fn normalize_optional_nulls(
    value: &mut Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<(), SchemaError> {
    let mut current = std::mem::take(value);
    let mut work = vec![NullWork::Visit(schema)];
    while let Some(task) = work.pop() {
        match task {
            NullWork::Visit(schema) => {
                let children = optional_null_children(&current, schema);
                if !children.is_empty() {
                    work.push(NullWork::Children {
                        parent: std::mem::take(&mut current),
                        children,
                        next: 0,
                    });
                }
            }
            NullWork::Children {
                mut parent,
                children,
                next,
            } => {
                let Some((slot, schema, optional)) = children.get(next) else {
                    current = parent;
                    continue;
                };
                let entry = match slot {
                    ValueSlot::Key(key) => parent.get(key),
                    ValueSlot::Index(index) => parent.get(*index),
                }
                .expect("selected child");
                if *optional && entry.is_null() && subs.check(schema, &Value::Null)? == Some(false)
                {
                    let ValueSlot::Key(key) = slot else {
                        unreachable!("only properties are optional")
                    };
                    parent.as_object_mut().expect("selected object").remove(key);
                    work.push(NullWork::Children {
                        parent,
                        children,
                        next: next + 1,
                    });
                    continue;
                }
                current = take_child(&mut parent, slot);
                let schema = *schema;
                work.push(NullWork::Restore {
                    parent,
                    children,
                    index: next,
                });
                work.push(NullWork::Visit(schema));
            }
            NullWork::Restore {
                mut parent,
                children,
                index,
            } => {
                put_child(
                    &mut parent,
                    &children[index].0,
                    std::mem::take(&mut current),
                );
                work.push(NullWork::Children {
                    parent,
                    children,
                    next: index + 1,
                });
            }
        }
    }
    *value = current;
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

enum CoerceWork<'a> {
    Schema(&'a Value),
    AllOf {
        schemas: &'a [Value],
        next: usize,
    },
    Union(&'a [Value]),
    CheckUnion {
        original: Value,
        schemas: &'a [Value],
        index: usize,
    },
    Types(&'a Value),
    Children {
        parent: Value,
        children: Vec<(ValueSlot, &'a Value)>,
        next: usize,
    },
    Restore {
        parent: Value,
        children: Vec<(ValueSlot, &'a Value)>,
        index: usize,
    },
}

fn coercion_children<'a>(
    value: &Value,
    schema: &'a Value,
    types: &[&str],
) -> Vec<(ValueSlot, &'a Value)> {
    let mut children = Vec::new();
    if types.contains(&"object")
        && let Some(object) = value.as_object()
    {
        let properties = schema.get("properties").and_then(Value::as_object);
        if let Some(properties) = properties {
            children.extend(
                properties
                    .iter()
                    .filter(|(key, _)| object.contains_key(*key))
                    .map(|(key, schema)| (ValueSlot::Key(key.clone()), schema)),
            );
        }
        if let Some(additional) = schema
            .get("additionalProperties")
            .filter(|schema| schema.is_object())
        {
            children.extend(
                object
                    .keys()
                    .filter(|key| properties.is_none_or(|map| !map.contains_key(*key)))
                    .map(|key| (ValueSlot::Key(key.clone()), additional)),
            );
        }
    }
    if types.contains(&"array")
        && let Some(array) = value.as_array()
    {
        match schema.get("items") {
            Some(Value::Array(items)) => children.extend(
                items
                    .iter()
                    .take(array.len())
                    .enumerate()
                    .map(|(index, schema)| (ValueSlot::Index(index), schema)),
            ),
            Some(items) if items.is_object() => {
                children.extend((0..array.len()).map(|index| (ValueSlot::Index(index), items)))
            }
            _ => {}
        }
    }
    children
}

fn coerce_with_json_schema(
    value: Value,
    schema: &Value,
    subs: &mut SubValidators,
) -> Result<Value, SchemaError> {
    let mut current = value;
    let mut work = vec![CoerceWork::Schema(schema)];
    while let Some(task) = work.pop() {
        match task {
            CoerceWork::Schema(schema) => {
                // Reverse scheduling preserves the source allOf -> anyOf ->
                // oneOf -> primitive -> child-coercion order.
                work.push(CoerceWork::Types(schema));
                if let Some(schemas) = schema.get("oneOf").and_then(Value::as_array) {
                    work.push(CoerceWork::Union(schemas));
                }
                if let Some(schemas) = schema.get("anyOf").and_then(Value::as_array) {
                    work.push(CoerceWork::Union(schemas));
                }
                if let Some(schemas) = schema.get("allOf").and_then(Value::as_array) {
                    work.push(CoerceWork::AllOf { schemas, next: 0 });
                }
            }
            CoerceWork::AllOf { schemas, next } => {
                if let Some(schema) = schemas.get(next) {
                    work.push(CoerceWork::AllOf {
                        schemas,
                        next: next + 1,
                    });
                    work.push(CoerceWork::Schema(schema));
                }
            }
            CoerceWork::Union(schemas) => {
                let mut matches_original = false;
                for arm in schemas {
                    if subs.check(arm, &current)? == Some(true) {
                        matches_original = true;
                        break;
                    }
                }
                if !matches_original && let Some(arm) = schemas.first() {
                    let original = std::mem::take(&mut current);
                    current = super::clone_value(&original);
                    work.push(CoerceWork::CheckUnion {
                        original,
                        schemas,
                        index: 0,
                    });
                    work.push(CoerceWork::Schema(arm));
                }
            }
            CoerceWork::CheckUnion {
                original,
                schemas,
                index,
            } => {
                if subs.check(&schemas[index], &current)? == Some(true) {
                    super::drop_value(original);
                } else if let Some(arm) = schemas.get(index + 1) {
                    super::drop_value(std::mem::take(&mut current));
                    current = super::clone_value(&original);
                    work.push(CoerceWork::CheckUnion {
                        original,
                        schemas,
                        index: index + 1,
                    });
                    work.push(CoerceWork::Schema(arm));
                } else {
                    super::drop_value(std::mem::replace(&mut current, original));
                }
            }
            CoerceWork::Types(schema) => {
                let types = get_schema_types(schema);
                let matches_union_member =
                    types.len() > 1 && types.iter().any(|kind| type_matches(kind, &current));
                if !types.is_empty() && !matches_union_member {
                    for kind in &types {
                        let candidate = coerce_primitive_by_type(&current, kind);
                        if candidate != current {
                            current = candidate;
                            break;
                        }
                    }
                }
                let children = coercion_children(&current, schema, &types);
                if !children.is_empty() {
                    work.push(CoerceWork::Children {
                        parent: std::mem::take(&mut current),
                        children,
                        next: 0,
                    });
                }
            }
            CoerceWork::Children {
                mut parent,
                children,
                next,
            } => {
                let Some((slot, schema)) = children.get(next) else {
                    current = parent;
                    continue;
                };
                current = take_child(&mut parent, slot);
                let schema = *schema;
                work.push(CoerceWork::Restore {
                    parent,
                    children,
                    index: next,
                });
                work.push(CoerceWork::Schema(schema));
            }
            CoerceWork::Restore {
                mut parent,
                children,
                index,
            } => {
                put_child(
                    &mut parent,
                    &children[index].0,
                    std::mem::take(&mut current),
                );
                work.push(CoerceWork::Children {
                    parent,
                    children,
                    next: index + 1,
                });
            }
        }
    }
    Ok(current)
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
