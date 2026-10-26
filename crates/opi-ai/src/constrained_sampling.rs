//! Constrained sampling: strict JSON-schema wire projection and grammar
//! tools (task 20.11, DEL-04/V-34/AIR-040).
//!
//! Pinned reference: `.repo/pi-1.3.0/packages/ai/src/api/constrained-sampling.ts`
//! (Pi AI 1.3.0). This module owns the provider-neutral strict/grammar
//! resolution: converting a **private wire schema** for provider constrained
//! sampling never mutates the trusted [`ToolDef::input_schema`]; prefer
//! projects non-strict when the wire or schema cannot support strict mode,
//! while require fails typed before any dispatch; grammar tools prefer lark
//! over regex variants, wrap their single string input property as an
//! incremental JSON object, and fail typed before dispatch when a supported
//! target has no usable variant or schema shape.
//!
//! Adapters own their wire flags (strict support, grammar support) and call
//! these helpers at body-build time. Validation of returned arguments stays
//! with the public schema engine; [`normalize_wire_arguments_nulls`] maps the
//! strict wire's forced-nullable output back onto the original trusted
//! schema's optional positions.

use serde_json::Value;

use crate::message::ToolDef;

/// Per-tool constrained-sampling intent (AIR-040).
///
/// `JsonSchema` selects provider-side structured outputs: `Prefer` projects
/// strict when the wire and the schema support it and silently stays
/// non-strict otherwise; `Require` fails typed before dispatch when strict
/// mode is unavailable or the schema cannot be converted.
///
/// `Grammar` selects OpenAI-style grammar-constrained custom tools; the
/// variants carry lark and regex definitions, lark preferred.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ConstrainedSampling {
    #[serde(rename = "json_schema")]
    JsonSchema { strict: StrictPreference },
    #[serde(rename_all = "camelCase")]
    Grammar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        openai_lark: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        openai_regex: Option<String>,
    },
}

/// Whether a tool's JSON-schema constrained sampling is preferred or
/// required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrictPreference {
    Prefer,
    Require,
}

/// Typed failure of the strict/grammar resolution: always a before-dispatch
/// failure of the call, never retried and never a silent constraint drop.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ConstrainedSamplingError(String);

impl ConstrainedSamplingError {
    fn new(detail: impl Into<String>) -> Self {
        Self(detail.into())
    }

    /// The redacted-safe detail line.
    pub fn detail(&self) -> &str {
        &self.0
    }
}

/// A keyword/value pair a provider's strict mode rejects (Pi
/// `UnsupportedStrictSchemaKeywordCheck`). Implementations only inspect the
/// present keyword and its value; the shared conversion owns the structural
/// subset (`$ref`, unions, tuples, …).
pub trait StrictKeywordAdmission {
    /// Returns `true` when the provider's strict mode rejects this keyword
    /// with this value.
    fn is_unsupported(&self, key: &str, value: &Value) -> bool;
}

/// Anthropic strict-tool keyword admission: the documented JSON-schema
/// limitations of Claude structured outputs (numeric bounds, some collection
/// limits, and all string formats outside the supported set are rejected;
/// `minItems` is admissible only as 0 or 1).
#[derive(Debug, Clone, Copy, Default)]
pub struct AnthropicStrictKeywordAdmission;

const ANTHROPIC_STRICT_UNSUPPORTED_KEYWORDS: &[&str] = &[
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "maxItems",
    "uniqueItems",
    "minContains",
    "maxContains",
    "minProperties",
    "maxProperties",
];

const ANTHROPIC_STRICT_STRING_FORMATS: &[&str] = &[
    "date-time",
    "time",
    "date",
    "duration",
    "email",
    "hostname",
    "uri",
    "ipv4",
    "ipv6",
    "uuid",
];

/// The OpenAI strict subset: no per-keyword rejection beyond the shared
/// structural conversion (Pi passes no `isUnsupportedKeyword` for
/// OpenAI-family wires).
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRejectedKeywords;

impl StrictKeywordAdmission for NoRejectedKeywords {
    fn is_unsupported(&self, _key: &str, _value: &Value) -> bool {
        false
    }
}

impl StrictKeywordAdmission for AnthropicStrictKeywordAdmission {
    fn is_unsupported(&self, key: &str, value: &Value) -> bool {
        if ANTHROPIC_STRICT_UNSUPPORTED_KEYWORDS.contains(&key) {
            return true;
        }
        if key == "minItems" {
            return !matches!(value.as_u64(), Some(0 | 1));
        }
        if key == "format" {
            return !value
                .as_str()
                .is_some_and(|format| ANTHROPIC_STRICT_STRING_FORMATS.contains(&format));
        }
        false
    }
}

/// Schema keywords no provider's strict subset in this module accepts
/// (structural subset shared by the conversion; per-wire admission adds more).
const UNSUPPORTED_STRICT_SCHEMA_KEYS: &[&str] = &[
    "$ref",
    "$defs",
    "definitions",
    "allOf",
    "oneOf",
    "patternProperties",
    "dependentSchemas",
    "dependencies",
    "unevaluatedProperties",
    "propertyNames",
    "contains",
    "prefixItems",
    "not",
    "if",
    "then",
    "else",
];

fn is_json_schema_object(value: &Value) -> bool {
    value.is_object()
}

fn schema_type_strings(schema: &Value) -> Vec<&str> {
    match schema.get("type") {
        Some(Value::String(single)) => vec![single.as_str()],
        Some(Value::Array(many)) => many.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        _ => Vec::new(),
    }
}

fn is_structured_schema(schema: &Value) -> bool {
    if !is_json_schema_object(schema) {
        return false;
    }
    let types = schema_type_strings(schema);
    types.iter().any(|t| *t == "object" || *t == "array")
        || schema.get("properties").is_some()
        || schema.get("items").is_some()
}

fn schema_allows_null(schema: &Value) -> bool {
    if !is_json_schema_object(schema) {
        return false;
    }
    let types = schema_type_strings(schema);
    if types.contains(&"null") {
        return true;
    }
    if schema.get("const") == Some(&Value::Null) {
        return true;
    }
    if let Some(Value::Array(variants)) = schema.get("enum")
        && variants.contains(&Value::Null)
    {
        return true;
    }
    schema
        .get("anyOf")
        .and_then(Value::as_array)
        .is_some_and(|variants| variants.iter().any(schema_allows_null))
}

fn make_strict_node(
    schema: &mut Value,
    admission: &dyn StrictKeywordAdmission,
    error: &mut String,
) {
    if !is_json_schema_object(schema) {
        *error = "boolean schemas are unsupported".into();
        return;
    }
    let object = schema.as_object().expect("checked above");
    for key in UNSUPPORTED_STRICT_SCHEMA_KEYS {
        if let Some(value) = object.get(*key)
            && !value.is_null()
        {
            *error = format!("{key} schemas are unsupported");
            return;
        }
    }
    for (key, value) in object {
        if admission.is_unsupported(key, value) {
            *error = format!("{key} is unsupported by this wire's strict mode");
            return;
        }
    }

    if let Some(any_of) = schema.get_mut("anyOf") {
        let Some(variants) = any_of.as_array_mut() else {
            *error = "anyOf must contain at least one schema".into();
            return;
        };
        if variants.is_empty() {
            *error = "anyOf must contain at least one schema".into();
            return;
        }
        for variant in variants.iter_mut() {
            if is_structured_schema(variant) {
                *error = "object and array unions are unsupported".into();
                return;
            }
            make_strict_node(variant, admission, error);
            if !error.is_empty() {
                return;
            }
        }
    }

    if let Some(items) = schema.get_mut("items") {
        if items.is_array() {
            *error = "tuple schemas are unsupported".into();
            return;
        }
        make_strict_node(items, admission, error);
        if !error.is_empty() {
            return;
        }
    }

    let is_object_schema = schema.get("type") == Some(&Value::String("object".into()));
    if schema.get("properties").is_some() && !is_object_schema {
        *error = "properties require type object".into();
        return;
    }
    if !is_object_schema {
        return;
    }
    if let Some(additional) = schema.get("additionalProperties")
        && additional != &Value::Bool(false)
    {
        *error = "schema-valued or true additionalProperties is unsupported".into();
        return;
    }
    // Extract the validated facts before borrowing the property map
    // mutably: property names, the required set, and each property's
    // original null-admission.
    let (property_names, _required, allows_null): (Vec<String>, Vec<String>, Vec<bool>) =
        match schema.get("properties") {
            None => (Vec::new(), Vec::new(), Vec::new()),
            Some(properties) => {
                if !is_json_schema_object(properties) {
                    *error = "object properties must be a schema map".into();
                    return;
                }
                if let Some(Value::Array(entries)) = schema.get("required")
                    && !entries.iter().all(Value::is_string)
                {
                    *error = "object required must be a string array".into();
                    return;
                }
                let required: Vec<String> = schema
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let names: Vec<String> = properties
                    .as_object()
                    .expect("checked above")
                    .keys()
                    .cloned()
                    .collect();
                if required.iter().any(|key| !names.contains(key)) {
                    *error = "required contains an unknown property".into();
                    return;
                }
                let allows_null: Vec<bool> = names
                    .iter()
                    .map(|name| {
                        required.contains(name)
                            || schema_allows_null(
                                properties.get(name).expect("name came from the map"),
                            )
                    })
                    .collect();
                (names, required, allows_null)
            }
        };
    if schema.get("properties").is_none() {
        // No properties: an empty object schema converts vacuously.
        finalize_object(schema, &property_names);
        return;
    }
    let properties = schema
        .get_mut("properties")
        .expect("properties existence checked above");
    for (index, key) in property_names.iter().enumerate() {
        let property = properties.get_mut(key).expect("key came from the map");
        make_strict_node(property, admission, error);
        if !error.is_empty() {
            return;
        }
        if !allows_null[index] {
            *property = serde_json::json!({"anyOf": [property.clone(), {"type": "null"}]});
        }
    }
    finalize_object(schema, &property_names);
}

fn finalize_object(schema: &mut Value, property_names: &[String]) {
    let object = schema.as_object_mut().expect("object schema");
    object.insert(
        "required".into(),
        Value::Array(
            property_names
                .iter()
                .map(|name| Value::String(name.clone()))
                .collect(),
        ),
    );
    object.insert("additionalProperties".into(), Value::Bool(false));
}

/// Convert a tool's trusted input schema to the strict subset expected by
/// provider constrained sampling (Pi `makeStrictJsonSchema`).
///
/// The conversion clones: the trusted schema document is never modified. The
/// root must be a JSON object schema; structural keywords outside the strict
/// subset, per-wire rejected keywords (via `admission`), boolean schemas,
/// tuple `items`, structured unions and non-`false` `additionalProperties`
/// are typed failures. Every property becomes required; optional properties
/// that do not already admit null are wrapped `anyOf: [property, null]`.
pub fn make_strict_json_schema(
    schema: &Value,
    admission: &dyn StrictKeywordAdmission,
) -> Result<Value, ConstrainedSamplingError> {
    let mut cloned = schema.clone();
    if !is_json_schema_object(&cloned) {
        return Err(ConstrainedSamplingError::new(
            "root schema must have type object",
        ));
    }
    let mut error = String::new();
    make_strict_node(&mut cloned, admission, &mut error);
    if !error.is_empty() {
        return Err(ConstrainedSamplingError::new(format!(
            "schema cannot be projected strict: {error}"
        )));
    }
    if cloned.get("type") != Some(&Value::String("object".into())) {
        return Err(ConstrainedSamplingError::new(
            "root schema must have type object",
        ));
    }
    Ok(cloned)
}

/// Decide whether a JSON-schema tool is sent in strict mode (Pi
/// `resolveJsonSchemaStrictSampling`).
///
/// - No `json_schema` config: `Ok(None)` (never strict through this path).
/// - Wire supports strict and the schema converts: `Ok(Some(true))`.
/// - Wire supports strict but conversion fails: `Prefer` yields `Ok(None)`
///   (non-strict projection), `Require` fails typed.
/// - Wire does not support strict: `Prefer` yields `Ok(None)`, `Require`
///   fails typed before dispatch.
pub fn resolve_json_schema_strict_sampling(
    tool: &ToolDef,
    supports_strict_mode: bool,
    admission: &dyn StrictKeywordAdmission,
) -> Result<Option<bool>, ConstrainedSamplingError> {
    let Some(ConstrainedSampling::JsonSchema { strict }) = &tool.constrained_sampling else {
        return Ok(None);
    };
    let tool_label = |detail: &str| {
        format!(
            "tool {:?} requires JSON-schema constrained sampling, but {detail}",
            tool.name
        )
    };
    if supports_strict_mode {
        return match make_strict_json_schema(&tool.input_schema, admission) {
            Ok(_) => Ok(Some(true)),
            Err(error) => match strict {
                StrictPreference::Prefer => Ok(None),
                StrictPreference::Require => {
                    Err(ConstrainedSamplingError::new(tool_label(error.detail())))
                }
            },
        };
    }
    match strict {
        StrictPreference::Prefer => Ok(None),
        StrictPreference::Require => Err(ConstrainedSamplingError::new(
            "strict tools are unsupported on this wire",
        )),
    }
}

/// A resolved grammar-constrained tool definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarConstrainedSampling {
    /// `lark` or `regex`.
    pub format: &'static str,
    pub definition: String,
    pub input_property: String,
}

fn infer_grammar_input_property(tool: &ToolDef) -> Result<String, ConstrainedSamplingError> {
    let fail = |detail: String| {
        ConstrainedSamplingError::new(format!(
            "tool {:?} cannot use grammar constrained sampling: {detail}",
            tool.name
        ))
    };
    let schema = &tool.input_schema;
    if schema.get("type") != Some(&Value::String("object".into())) {
        return Err(fail("grammar requires an object parameter schema".into()));
    }
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("grammar requires exactly one required string property".into()))?;
    if required.len() != 1 || !required[0].is_string() {
        return Err(fail(
            "grammar requires exactly one required string property".into(),
        ));
    }
    let input_property = required[0].as_str().expect("checked above").to_owned();
    let property = schema
        .pointer(&format!("/properties/{input_property}"))
        .ok_or_else(|| {
            fail(format!(
                "grammar requires a properties entry for {input_property}"
            ))
        })?;
    if property.get("type") != Some(&Value::String("string".into())) {
        return Err(fail(format!(
            "grammar property {input_property} must have type string"
        )));
    }
    Ok(input_property)
}

/// Resolve a grammar-constrained tool (Pi `resolveGrammarConstrainedSampling`).
///
/// lark is preferred over regex. When the target wire does not support
/// grammar tools the tool projects as a plain JSON function tool
/// (`Ok(None)`); a supported target with no usable variant or an invalid
/// schema shape fails typed before dispatch.
pub fn resolve_grammar_constrained_sampling(
    tool: &ToolDef,
    supports_grammar_tools: bool,
) -> Result<Option<GrammarConstrainedSampling>, ConstrainedSamplingError> {
    let Some(ConstrainedSampling::Grammar {
        openai_lark,
        openai_regex,
    }) = &tool.constrained_sampling
    else {
        return Ok(None);
    };
    if !supports_grammar_tools {
        return Ok(None);
    }
    let has_lark = openai_lark
        .as_deref()
        .is_some_and(|definition| !definition.trim().is_empty());
    let has_regex = openai_regex
        .as_deref()
        .is_some_and(|definition| !definition.trim().is_empty());
    if !has_lark && !has_regex {
        return Err(ConstrainedSamplingError::new(format!(
            "tool {:?} cannot use grammar constrained sampling: no supported grammar variant was provided",
            tool.name
        )));
    }
    let input_property = infer_grammar_input_property(tool)?;
    Ok(Some(if has_lark {
        GrammarConstrainedSampling {
            format: "lark",
            definition: openai_lark.clone().expect("has_lark checked"),
            input_property,
        }
    } else {
        GrammarConstrainedSampling {
            format: "regex",
            definition: openai_regex.clone().expect("!has_lark implies has_regex"),
            input_property,
        }
    }))
}

/// The per-call grammar-input wrapping buffer (Pi
/// `GrammarToolInputJsonBuffer`): tracks the raw input string, whether the
/// JSON object wrapper opened, and whether it closed.
#[derive(Debug, Clone)]
pub struct GrammarInputBuffer {
    input_property: String,
    input: String,
    started: bool,
    closed: bool,
}

impl GrammarInputBuffer {
    /// A fresh buffer for the tool's input property.
    pub fn new(input_property: String, initial_input: String) -> Self {
        let started = !initial_input.is_empty();
        Self {
            input_property,
            input: initial_input,
            // A non-empty initial input (replayed custom calls) counts as
            // already started; the wrapper opens with the first delta.
            started,
            closed: false,
        }
    }

    /// The raw input observed so far.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// Whether the JSON wrapper already closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

/// Append one grammar-input delta (Pi `appendGrammarToolInputJsonDelta`).
///
/// The wire's raw string input becomes the JSON object text
/// `{"<property>":"<input>"}` incrementally: the first delta opens the
/// wrapper, deltas append the JSON-escaped input increment, and `close`
/// terminates the string and object. Input must grow monotonically; a
/// changed-after-close input is a typed failure. An empty non-closing delta
/// yields `Ok(None)` (nothing to emit).
pub fn append_grammar_tool_input_delta(
    buffer: &mut GrammarInputBuffer,
    next_input: &str,
    close: bool,
) -> Result<Option<String>, ConstrainedSamplingError> {
    let fail = |detail: String| {
        ConstrainedSamplingError::new(format!(
            "grammar tool input for property {:?} {detail}",
            buffer.input_property
        ))
    };
    if buffer.closed {
        if close && next_input == buffer.input {
            return Ok(None);
        }
        return Err(fail("changed after it was closed".into()));
    }
    if !next_input.starts_with(buffer.input.as_str()) {
        return Err(fail("changed non-monotonically".into()));
    }
    let input_delta = &next_input[buffer.input.len()..];
    if !close && input_delta.is_empty() {
        return Ok(None);
    }
    let mut delta = String::new();
    if !buffer.started {
        delta.push_str(&format!(
            "{{{}:\"",
            serde_json::to_string(&buffer.input_property).expect("property name serializes")
        ));
        buffer.started = true;
    }
    let escaped = serde_json::to_string(input_delta).expect("input delta serializes");
    delta.push_str(&escaped[1..escaped.len() - 1]);
    buffer.input = next_input.to_owned();
    if close {
        delta.push_str("\"}");
        buffer.closed = true;
    }
    Ok(Some(delta))
}

/// Fail-closed constrained-sampling admission for wires that do not
/// (yet) project strict tools (task 20.11, AIR-040): a `require` intent is
/// never silently dropped — it fails typed before dispatch. `prefer`
/// projects non-strict; grammar intents follow the same rule as an
/// unsupported target (plain JSON function tool).
pub fn assert_supported_constraints(
    tools: &[ToolDef],
    strict_supported: bool,
) -> Result<(), ConstrainedSamplingError> {
    if strict_supported {
        return Ok(());
    }
    for tool in tools {
        if let Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }) = &tool.constrained_sampling
        {
            return Err(ConstrainedSamplingError::new(format!(
                "tool {:?} requires JSON-schema constrained sampling, but strict tools are not wired on this wire",
                tool.name
            )));
        }
    }
    Ok(())
}

/// Normalize strict-wire tool arguments back onto the original trusted
/// schema (V-34: returned null normalizes per the original schema).
///
/// The strict projection forces every property required and nullable, so the
/// model may answer an explicitly-null for a property the original schema
/// declares optional. Such nulls are dropped: the original optional position
/// is expressed by absence. Positions that already allowed null in the
/// original schema keep their null.
pub fn normalize_wire_arguments_nulls(original_schema: &Value, arguments: &mut Value) {
    let Some(map) = arguments.as_object_mut() else {
        return;
    };
    if original_schema.get("type") != Some(&Value::String("object".into())) {
        return;
    }
    let required: Vec<String> = original_schema
        .get("required")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let Some(properties) = original_schema.get("properties") else {
        return;
    };
    map.retain(|key, value| {
        if !value.is_null() {
            return true;
        }
        if required.contains(key) {
            return true;
        }
        let Some(property) = properties.get(key) else {
            // A null for a position the original schema never declared:
            // additional data; strict mode could not have produced it from
            // this projection, so keep the caller's evidence.
            return true;
        };
        schema_allows_null(property)
    });
}
