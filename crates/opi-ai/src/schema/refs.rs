//! Reference resolution ported from the pinned TypeBox 1.3.27
//! `build/schema/engine/_stack.mjs` and `build/schema/resolve/resolve.mjs`
//! (tree SHA `ebf39b14…`).
//!
//! The stack tracks lexical/resource bases, anchors and retrieved/resolved
//! resource frames during traversal; resolution delegates to the `Resolve`
//! port below with snapshot frames. URL math follows WHATWG
//! `new URL(input, base)` semantics via the `url` crate. A malformed URI or
//! percent-escapes in a fragment is a typed error (the upstream throws); an
//! unresolved reference is not an error — the `$ref` leaf evaluates `false`.

use std::collections::HashMap;

use serde_json::Value;
use url::Url;

use super::SchemaError;
use super::guards;
use super::js::js_keys;

const DEFAULT_BASE: &str = "https://json-schema.org";

/// Snapshot of the traversal state handed to resolution (source
/// `_Stack_StackFrame`). Owns cloned anchor/ids vectors so the stack can be
/// mutated while the snapshot is live.
pub(crate) struct StackFrame<'a> {
    pub(crate) context: &'a serde_json::Map<String, Value>,
    pub(crate) root: &'a Value,
    pub(crate) ids: Vec<&'a Value>,
    pub(crate) lexical_schema: &'a Value,
    pub(crate) lexical_base: String,
    pub(crate) reference_base: String,
    pub(crate) resource_base: String,
    pub(crate) recursive_anchors: Vec<&'a Value>,
    pub(crate) dynamic_anchors: Vec<&'a Value>,
    pub(crate) in_retrieved_frame: bool,
}

/// A `$ref` resolution result (source `ResolveRef` return shape).
pub(crate) struct RefResolution<'a> {
    pub(crate) schema: Option<&'a Value>,
    pub(crate) retrieved_resource: Option<RetrievedResource<'a>>,
    pub(crate) resolved_resource: Option<ResolvedResource<'a>>,
}

pub(crate) struct RetrievedResource<'a> {
    pub(crate) target: &'a Value,
    pub(crate) base: String,
    pub(crate) root: &'a Value,
}

pub(crate) struct ResolvedResource<'a> {
    pub(crate) target: &'a Value,
    pub(crate) resource: &'a Value,
}

struct RetrievedFrameEntry<'a> {
    root: &'a Value,
    base: String,
    id_depth: usize,
    resource_depth: usize,
}

/// Traversal stack (source `Stack`).
pub(crate) struct RefStack<'a> {
    context: &'a serde_json::Map<String, Value>,
    schema: &'a Value,
    ids: Vec<&'a Value>,
    resource_ids: Vec<&'a Value>,
    anchors: Vec<&'a Value>,
    recursive_anchors: Vec<&'a Value>,
    dynamic_anchors: Vec<&'a Value>,
    retrieved_resources: HashMap<*const Value, RetrievedFrameEntry<'a>>,
    retrieved_frames: Vec<RetrievedFrameEntry<'a>>,
    /// The source clones frame records; this port stores frame snapshots
    /// keyed by insertion order and pops symmetrically.
    resolved_resources: Vec<(*const Value, &'a Value)>,
    pending_resource: bool,
}

impl<'a> RefStack<'a> {
    pub(crate) fn new(context: &'a serde_json::Map<String, Value>, schema: &'a Value) -> Self {
        Self {
            context,
            schema,
            ids: Vec::new(),
            resource_ids: Vec::new(),
            anchors: Vec::new(),
            recursive_anchors: Vec::new(),
            dynamic_anchors: Vec::new(),
            retrieved_resources: HashMap::new(),
            retrieved_frames: Vec::new(),
            resolved_resources: Vec::new(),
            pending_resource: true,
        }
    }

    fn lexical_base_url(&self) -> Result<String, SchemaError> {
        self.build_base(&self.ids)
    }

    /// Source `_Stack_BuildBase`: resolve the scoped `$id` chain over the
    /// current retrieved frame's base (or the default base).
    fn build_base(&self, stack: &[&'a Value]) -> Result<String, SchemaError> {
        let frame = self.retrieved_frames.last();
        let mut url = match frame {
            Some(frame) => parse_url(&frame.base)?,
            None => parse_url(DEFAULT_BASE)?,
        };
        let scoped: &[&'a Value] = match frame {
            Some(frame) => &stack[frame.id_depth..],
            None => stack,
        };
        for schema in scoped {
            if let Some(id) = guards::get_str(schema, "$id") {
                url = join_url(&url, id)?;
            }
        }
        Ok(url.to_string())
    }

    fn resource_base_url(&self) -> Result<String, SchemaError> {
        let frame = self.retrieved_frames.last();
        match frame {
            Some(frame) => {
                let mut url = parse_url(&frame.base)?;
                for schema in &self.resource_ids[frame.resource_depth..] {
                    if let Some(id) = guards::get_str(schema, "$id") {
                        url = join_url(&url, id)?;
                    }
                }
                Ok(url.to_string())
            }
            None => self.build_base(&self.resource_ids),
        }
    }

    fn reference_base_url(&self) -> Result<String, SchemaError> {
        if !self.retrieved_frames.is_empty() {
            return self.resource_base_url();
        }
        if let Some(lexical) = self.ids.last().copied()
            && let Some(id) = guards::get_str(lexical, "$id")
            && !has_scheme(id)
        {
            return self.lexical_base_url();
        }
        self.resource_base_url()
    }

    fn lexical_schema(&self) -> &'a Value {
        let frame = self.retrieved_frames.last();
        match frame {
            Some(frame) => {
                if self.ids.len() > frame.id_depth {
                    self.ids[self.ids.len() - 1]
                } else {
                    frame.root
                }
            }
            None => self.ids.last().copied().unwrap_or(self.schema),
        }
    }

    fn stack_frame(&self) -> Result<StackFrame<'a>, SchemaError> {
        Ok(StackFrame {
            context: self.context,
            root: self.schema,
            ids: self.ids.clone(),
            lexical_schema: self.lexical_schema(),
            lexical_base: self.lexical_base_url()?,
            reference_base: self.reference_base_url()?,
            resource_base: self.resource_base_url()?,
            recursive_anchors: self.recursive_anchors.clone(),
            dynamic_anchors: self.dynamic_anchors.clone(),
            in_retrieved_frame: !self.retrieved_frames.is_empty(),
        })
    }

    /// Hash the live reference-frame state into `state` (used for the
    /// evaluator's no-progress detection).
    pub(crate) fn fingerprint(&self, state: &mut u64) {
        fn mix_ptrs(state: &mut u64, ptrs: &[&Value]) {
            for ptr in ptrs {
                let addr = *ptr as *const Value as usize;
                *state ^= (addr as u64).wrapping_mul(1_099_511_628_211);
                *state = state.rotate_left(7);
            }
        }
        fn mix_u64(state: &mut u64, value: u64) {
            *state ^= value.wrapping_mul(1_099_511_628_211);
            *state = state.rotate_left(7);
        }
        fn mix_str(state: &mut u64, text: &str) {
            for byte in text.as_bytes() {
                *state ^= u64::from(*byte);
                *state = state.wrapping_mul(1_099_511_628_211);
            }
        }
        mix_ptrs(state, &self.ids);
        mix_ptrs(state, &self.resource_ids);
        mix_ptrs(state, &self.anchors);
        mix_ptrs(state, &self.recursive_anchors);
        mix_ptrs(state, &self.dynamic_anchors);
        mix_u64(state, self.retrieved_frames.len() as u64);
        for frame in &self.retrieved_frames {
            mix_u64(state, frame.root as *const Value as u64);
            mix_u64(state, frame.id_depth as u64);
            mix_u64(state, frame.resource_depth as u64);
            mix_str(state, &frame.base);
        }
        mix_u64(state, self.resolved_resources.len() as u64);
        for (target, resource) in &self.resolved_resources {
            mix_u64(state, *target as u64);
            mix_u64(state, *resource as *const Value as u64);
        }
        mix_u64(state, u64::from(self.pending_resource));
    }

    // ------------------------------------------------------------------
    // Push / Pop
    // ------------------------------------------------------------------

    pub(crate) fn push(&mut self, schema: &'a Value) {
        if !schema.is_object() {
            return;
        }
        if guards::is_id(schema) {
            self.register_resource(schema);
        }
        if guards::is_anchor(schema) {
            self.anchors.push(schema);
        }
        if guards::is_recursive_anchor_true(schema) {
            self.recursive_anchors.push(schema);
        }
        if guards::is_dynamic_anchor(schema) {
            self.dynamic_anchors.push(schema);
        }
        if let Some(entry) = self.retrieved_resources.get(&(schema as *const Value)) {
            let snapshot = RetrievedFrameEntry {
                root: entry.root,
                base: entry.base.clone(),
                id_depth: self.ids.len(),
                resource_depth: self.resource_ids.len(),
            };
            self.retrieved_frames.push(snapshot);
        }
    }

    pub(crate) fn pop(&mut self, schema: &'a Value) {
        if !schema.is_object() {
            return;
        }
        if guards::is_id(schema) {
            self.unregister_resource(schema);
        }
        if guards::is_anchor(schema) {
            self.anchors.pop();
        }
        if guards::is_recursive_anchor_true(schema) {
            self.recursive_anchors.pop();
        }
        if guards::is_dynamic_anchor(schema) {
            self.dynamic_anchors.pop();
        }
        if self
            .retrieved_resources
            .contains_key(&(schema as *const Value))
        {
            self.retrieved_frames.pop();
        }
        self.exit_resolved_resource(schema as *const Value);
    }

    // ------------------------------------------------------------------
    // Resource registration
    // ------------------------------------------------------------------

    fn register_resource(&mut self, schema: &'a Value) {
        self.ids.push(schema);
        let is_resource = self.pending_resource;
        self.pending_resource = false;
        if is_resource {
            self.resource_ids.push(schema);
        }
        self.register_resource_anchors(schema, true);
    }

    fn unregister_resource(&mut self, schema: &'a Value) {
        self.ids.pop();
        let is_resource =
            !self.resource_ids.is_empty() && self.resource_ids.last().copied() == Some(schema);
        if is_resource {
            self.resource_ids.pop();
        }
        self.unregister_resource_anchors(schema, true);
    }

    fn register_resource_anchors(&mut self, schema: &'a Value, is_root: bool) {
        match schema {
            Value::Bool(_) => {}
            Value::Array(items) => {
                for item in items {
                    self.register_resource_anchors(item, false);
                }
            }
            Value::Object(map) => {
                if !is_root && guards::is_id(schema) {
                    return;
                }
                if !is_root && guards::is_dynamic_anchor(schema) {
                    self.dynamic_anchors.push(schema);
                }
                for key in js_keys(map) {
                    self.register_resource_anchors(&map[key], false);
                }
            }
            _ => {}
        }
    }

    fn unregister_resource_anchors(&mut self, schema: &'a Value, is_root: bool) {
        match schema {
            Value::Bool(_) => {}
            Value::Array(items) => {
                for item in items {
                    self.unregister_resource_anchors(item, false);
                }
            }
            Value::Object(map) => {
                if !is_root && guards::is_id(schema) {
                    return;
                }
                if !is_root && guards::is_dynamic_anchor(schema) {
                    self.dynamic_anchors.pop();
                }
                for key in js_keys(map) {
                    self.unregister_resource_anchors(&map[key], false);
                }
            }
            _ => {}
        }
    }

    fn enter_resolved_resource(&mut self, target: &'a Value, resource: &'a Value) {
        self.register_resource(resource);
        self.resolved_resources
            .push((target as *const Value, resource));
    }

    fn exit_resolved_resource(&mut self, target: *const Value) {
        if let Some(position) = self
            .resolved_resources
            .iter()
            .rposition(|(key, _)| *key == target)
        {
            let (_, resource) = self.resolved_resources.remove(position);
            self.unregister_resource(resource);
        }
    }

    // ------------------------------------------------------------------
    // Ref application
    // ------------------------------------------------------------------

    /// Source `Stack.Ref`: resolve a `$ref` and apply frame effects.
    /// Returns the target schema (`None` when unresolved).
    pub(crate) fn apply_ref(&mut self, reference: &str) -> Result<Option<&'a Value>, SchemaError> {
        let frame = self.stack_frame()?;
        let result = resolve_ref(&frame, reference)?;
        let schema = result.schema;
        self.apply_ref_result(result);
        Ok(schema)
    }

    /// Source `Stack.RecursiveRef`.
    pub(crate) fn apply_recursive_ref(
        &mut self,
        recursive_ref: &str,
    ) -> Result<Option<&'a Value>, SchemaError> {
        let frame = self.stack_frame()?;
        let target = resolve_recursive_ref(&frame, recursive_ref)?;
        if target.is_some() {
            self.pending_resource = true;
        }
        Ok(target)
    }

    /// Source `Stack.DynamicRef`.
    pub(crate) fn apply_dynamic_ref(
        &mut self,
        dynamic_ref: &str,
    ) -> Result<Option<&'a Value>, SchemaError> {
        let frame = self.stack_frame()?;
        let target = resolve_dynamic_ref(&frame, dynamic_ref)?;
        if target.is_some() {
            self.pending_resource = true;
        }
        Ok(target)
    }

    fn apply_ref_result(&mut self, result: RefResolution<'a>) {
        if result.schema.is_some() {
            self.pending_resource = true;
        }
        if let Some(resolved) = result.resolved_resource {
            self.enter_resolved_resource(resolved.target, resolved.resource);
        }
        if let Some(retrieved) = result.retrieved_resource {
            self.retrieved_resources.insert(
                retrieved.target as *const Value,
                RetrievedFrameEntry {
                    root: retrieved.root,
                    base: retrieved.base,
                    id_depth: 0,
                    resource_depth: 0,
                },
            );
        }
    }
}

// ------------------------------------------------------------------
// Resolve (resolve.mjs port)
// ------------------------------------------------------------------

fn parse_url(text: &str) -> Result<Url, SchemaError> {
    Url::parse(text).map_err(|e| SchemaError::Uri {
        detail: e.to_string(),
    })
}

fn join_url(base: &Url, input: &str) -> Result<Url, SchemaError> {
    base.join(input).map_err(|e| SchemaError::Uri {
        detail: e.to_string(),
    })
}

fn has_scheme(id: &str) -> bool {
    // Source: /^[A-Za-z][A-Za-z0-9+.-]*:/
    let bytes = id.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    for byte in &bytes[1..] {
        match byte {
            b':' => return true,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'+' | b'.' | b'-' => continue,
            _ => return false,
        }
    }
    false
}

/// `url.hash` (leading `#` included, empty when absent).
fn js_hash(url: &Url) -> String {
    url.fragment().map(|f| format!("#{f}")).unwrap_or_default()
}

/// `CanonicalHref`: the URL serialized without its fragment.
fn canonical_href(url: &Url) -> String {
    let mut clone = url.clone();
    clone.set_fragment(None);
    clone.to_string()
}

/// Source `FindDynamicAnchor`: first schema carrying the named
/// `$dynamicAnchor`, in traversal order.
fn find_dynamic_anchor<'a>(schema: &'a Value, name: &str) -> Option<&'a Value> {
    if schema.is_object() && guards::get_str(schema, "$dynamicAnchor") == Some(name) {
        return Some(schema);
    }
    match schema {
        Value::Object(map) => {
            for key in js_keys(map) {
                if let Some(result) = find_dynamic_anchor(&map[key], name) {
                    return Some(result);
                }
            }
            None
        }
        Value::Array(items) => {
            for item in items {
                if let Some(result) = find_dynamic_anchor(item, name) {
                    return Some(result);
                }
            }
            None
        }
        _ => None,
    }
}

/// Source `FindBase`: first-match search for the base URL of `target`.
fn find_base<'a>(schema: &'a Value, base: &Url, target: &'a Value) -> Option<String> {
    if std::ptr::eq(schema, target) {
        return Some(base.to_string());
    }
    let next_base: Url = if schema.is_object() && guards::is_id(schema) {
        let id = guards::get_str(schema, "$id")?;
        base.join(id).ok()?
    } else {
        base.clone()
    };
    match schema {
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_base(item, &next_base, target)),
        Value::Object(map) => js_keys(map)
            .into_iter()
            .find_map(|key| find_base(&map[key], &next_base, target)),
        _ => None,
    }
}

/// Source `Pointer.Get` with the upstream `GetIndex` unsafe-key semantics.
fn pointer_get<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    let mut current = value;
    for index in pointer_indices(pointer) {
        if !pointer_has_index(&index, current) {
            return None;
        }
        current = pointer_index(&index, current)?;
    }
    Some(current)
}

fn is_unsafe_key(key: &str) -> bool {
    key == "__proto__" || key == "constructor" || key == "prototype"
}

fn pointer_has_index(index: &str, value: &Value) -> bool {
    // Guard.IsObject covers arrays too; unsafe keys use own-property checks,
    // which for JSON values coincides with normal containment.
    match value {
        Value::Object(map) => map.contains_key(index),
        Value::Array(items) => index
            .parse::<usize>()
            .map(|i| i < items.len())
            .unwrap_or(false),
        _ => false,
    }
}

fn pointer_index<'a>(index: &str, value: &'a Value) -> Option<&'a Value> {
    if is_unsafe_key(index) {
        return None;
    }
    match value {
        Value::Object(map) => map.get(index),
        Value::Array(items) => index.parse::<usize>().ok().and_then(|i| items.get(i)),
        _ => None,
    }
}

/// Source `Pointer.Indices`: split, then `~1` -> `/` then `~0` -> `~`.
fn pointer_indices(pointer: &str) -> Vec<String> {
    let mut indices: Vec<String> = pointer
        .split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect();
    if indices.first().is_some_and(|first| first.is_empty()) {
        indices.remove(0);
    }
    indices
}

/// JS `decodeURIComponent` over a fragment body: UTF-8 percent-decoding,
/// erroring on malformed escapes or non-UTF-8 output.
fn decode_uri_component(text: &str) -> Result<String, SchemaError> {
    fn hex_value(byte: u8) -> Result<u8, SchemaError> {
        match byte {
            b'0'..=b'9' => Ok(byte - b'0'),
            b'a'..=b'f' => Ok(byte - b'a' + 10),
            b'A'..=b'F' => Ok(byte - b'A' + 10),
            _ => Err(SchemaError::Uri {
                detail: "malformed percent escape".to_string(),
            }),
        }
    }
    let malformed = || SchemaError::Uri {
        detail: "malformed percent escape".to_string(),
    };
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(malformed());
            }
            let hi = hex_value(bytes[i + 1])?;
            let lo = hex_value(bytes[i + 2])?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| SchemaError::Uri {
        detail: "fragment is not valid UTF-8".to_string(),
    })
}

// ------------------------------------------------------------------
// Match functions
// ------------------------------------------------------------------

fn match_id<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    let id = guards::get_str(schema, "$id")?;
    // Bare fragment naming this $id directly.
    if id == js_hash(reference) {
        return Some(schema);
    }
    let absolute = base.join(reference.as_str()).ok()?;
    if base.path() == absolute.path() {
        return if js_hash(reference).starts_with('#') {
            match_hash(schema, reference)
        } else {
            Some(schema)
        };
    }
    None
}

fn match_anchor<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    let anchor = guards::get_str(schema, "$anchor")?;
    let absolute_anchor = base.join(&format!("#{anchor}")).ok()?;
    if absolute_anchor.as_str() == reference.as_str() {
        Some(schema)
    } else {
        None
    }
}

fn match_dynamic_anchor<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    let anchor = guards::get_str(schema, "$dynamicAnchor")?;
    let absolute_anchor = base.join(&format!("#{anchor}")).ok()?;
    if absolute_anchor.as_str() == reference.as_str() {
        Some(schema)
    } else {
        None
    }
}

fn match_hash<'a>(schema: &'a Value, reference: &Url) -> Option<&'a Value> {
    if reference.as_str().ends_with('#') {
        return Some(schema);
    }
    let hash = js_hash(reference);
    if !hash.starts_with('#') {
        return None;
    }
    let fragment = decode_uri_component(&hash[1..]).ok()?;
    if !fragment.starts_with('/') {
        return None;
    }
    pointer_get(schema, &fragment)
}

// The if-chain keeps the upstream `Match` structure (id, then anchor, then
// dynamic anchor, then hash) explicit.
#[allow(clippy::collapsible_if)]
fn r#match<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    if guards::is_id(schema) {
        if let Some(result) = match_id(schema, base, reference) {
            return Some(result);
        }
    }
    if guards::is_anchor(schema) {
        if let Some(result) = match_anchor(schema, base, reference) {
            return Some(result);
        }
    }
    if guards::is_dynamic_anchor(schema) {
        if let Some(result) = match_dynamic_anchor(schema, base, reference) {
            return Some(result);
        }
    }
    match_hash(schema, reference)
}

// ------------------------------------------------------------------
// FromValue family (last match wins, upstream enumeration order)
// ------------------------------------------------------------------

fn from_value<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    let next_base = if schema.is_object() && guards::is_id(schema) {
        guards::get_str(schema, "$id")
            .and_then(|id| base.join(id).ok())
            .unwrap_or_else(|| base.clone())
    } else {
        base.clone()
    };
    if schema.is_object()
        && let Some(result) = r#match(schema, &next_base, reference)
    {
        return Some(result);
    }
    match schema {
        Value::Array(items) => {
            let mut result: Option<&'a Value> = None;
            for item in items {
                if let Some(matched) = from_value(item, &next_base, reference) {
                    result = Some(matched);
                }
            }
            result
        }
        Value::Object(map) => {
            let mut result: Option<&'a Value> = None;
            for key in js_keys(map) {
                if key == "const" || key == "enum" {
                    continue;
                }
                if let Some(matched) = from_value(&map[key], &next_base, reference) {
                    result = Some(matched);
                }
            }
            result
        }
        _ => None,
    }
}

// ------------------------------------------------------------------
// Ref entry points
// ------------------------------------------------------------------

/// Source `Ref(remotes, schema, base, ref, applySchemaId=false)`.
fn resolve_reference<'a>(
    context: &'a serde_json::Map<String, Value>,
    schema: &'a Value,
    base: &str,
    reference: &str,
    apply_schema_id: bool,
) -> Result<Option<&'a Value>, SchemaError> {
    let initial_base = parse_url(base_or_dot(base))?;
    let resolved_base = if apply_schema_id && guards::is_id(schema) {
        guards::get_str(schema, "$id")
            .map(|id| join_url(&initial_base, id))
            .transpose()?
            .unwrap_or(initial_base)
    } else {
        initial_base
    };
    let initial_ref = join_url(&resolved_base, reference)?;
    // Phase 1: context lookup by the raw reference string.
    if let Some(remote) = context.get(reference) {
        return Ok(Some(remote));
    }
    // Phase 2: local tree search.
    if let Some(local) = from_value(schema, &resolved_base, &initial_ref) {
        return Ok(Some(local));
    }
    // Phase 3: remote document in context.
    let canonical = canonical_href(&initial_ref);
    if canonical == canonical_href(&resolved_base) {
        return Ok(None);
    }
    let Some(remote_schema) = context.get(&canonical) else {
        return Ok(None);
    };
    let canonical_url = parse_url(&canonical)?;
    let remote_base = if remote_schema.is_object() && guards::is_id(remote_schema) {
        let id = guards::get_str(remote_schema, "$id").unwrap_or_default();
        join_url(&canonical_url, id)?
    } else {
        canonical_url
    };
    if initial_ref.fragment().is_none() {
        return Ok(Some(remote_schema));
    }
    Ok(from_value(remote_schema, &remote_base, &initial_ref))
}

fn base_or_dot(base: &str) -> &str {
    if base.is_empty() { "." } else { base }
}

/// Source `ResolveRef`.
fn resolve_ref<'a>(
    frame: &StackFrame<'a>,
    reference: &str,
) -> Result<RefResolution<'a>, SchemaError> {
    let source = if frame.in_retrieved_frame {
        frame.lexical_schema
    } else {
        frame.root
    };
    let ref_root = if reference.starts_with('#') {
        frame.lexical_schema
    } else {
        source
    };
    let schema = resolve_reference(
        frame.context,
        ref_root,
        &frame.reference_base,
        reference,
        false,
    )?;
    let Some(schema) = schema else {
        return Ok(RefResolution {
            schema: None,
            retrieved_resource: None,
            resolved_resource: None,
        });
    };
    if !schema.is_object() {
        // Boolean target: plain schema result, no frames.
        return Ok(RefResolution {
            schema: Some(schema),
            retrieved_resource: None,
            resolved_resource: None,
        });
    }
    let reference_url =
        parse_url(&frame.reference_base).and_then(|base| join_url(&base, reference))?;
    let canonical = canonical_href(&reference_url);
    let is_remote = canonical != frame.resource_base;

    // FindRetrievedResource: remote frame first, then legacy same-document.
    let retrieved = if is_remote {
        remote_retrieved_resource(frame.context, &canonical, schema)
    } else {
        None
    }
    .or_else(|| legacy_retrieved_resource(frame, reference, schema));

    let resolved = if is_remote {
        find_resolved_resource(frame, &canonical, schema)
    } else {
        None
    };

    Ok(RefResolution {
        schema: Some(schema),
        retrieved_resource: retrieved,
        resolved_resource: resolved,
    })
}

/// Source `RemoteRetrievedResource`.
fn remote_retrieved_resource<'a>(
    context: &'a serde_json::Map<String, Value>,
    canonical: &str,
    schema: &'a Value,
) -> Option<RetrievedResource<'a>> {
    let remote_root = context.get(canonical)?;
    if !remote_root.is_object() {
        return None;
    }
    Some(RetrievedResource {
        target: schema,
        base: canonical.to_string(),
        root: remote_root,
    })
}

/// Source `LegacyRetrievedResource`: draft-4 style in-place base change for
/// schemas without `$schema`.
fn legacy_retrieved_resource<'a>(
    frame: &StackFrame<'a>,
    reference: &str,
    schema: &'a Value,
) -> Option<RetrievedResource<'a>> {
    if !reference.starts_with('#') {
        return None;
    }
    if frame
        .root
        .as_object()
        .is_some_and(|map| map.contains_key("$schema"))
    {
        return None;
    }
    let reference_base = parse_url(&frame.reference_base).ok()?;
    let target_base = find_base(frame.lexical_schema, &reference_base, schema)?;
    if target_base == frame.reference_base {
        return None;
    }
    Some(RetrievedResource {
        target: schema,
        base: target_base,
        root: frame.lexical_schema,
    })
}

/// Source `FindResolvedResource`.
fn find_resolved_resource<'a>(
    frame: &StackFrame<'a>,
    canonical: &str,
    schema: &'a Value,
) -> Option<ResolvedResource<'a>> {
    if guards::is_id(schema) {
        return None;
    }
    let reference_base = parse_url(&frame.reference_base).ok()?;
    let resource = resource(
        frame.context,
        frame.root,
        reference_base.as_ref(),
        canonical,
    )?;
    if frame.ids.iter().any(|id| std::ptr::eq(*id, resource)) {
        return None;
    }
    Some(ResolvedResource {
        target: schema,
        resource,
    })
}

/// Source `Resource`: resolve to a schema object carrying `$id`.
fn resource<'a>(
    context: &'a serde_json::Map<String, Value>,
    schema: &'a Value,
    base: &str,
    reference: &str,
) -> Option<&'a Value> {
    let result = resolve_reference(context, schema, base, reference, false)
        .ok()
        .flatten()?;
    if result.is_object() && guards::is_id(result) {
        Some(result)
    } else {
        None
    }
}

/// Source `ResolveRecursiveRef`.
fn resolve_recursive_ref<'a>(
    frame: &StackFrame<'a>,
    recursive_ref: &str,
) -> Result<Option<&'a Value>, SchemaError> {
    let ref_root = if guards::is_recursive_anchor_true(frame.lexical_schema) {
        frame
            .recursive_anchors
            .first()
            .copied()
            .unwrap_or(frame.lexical_schema)
    } else {
        frame.lexical_schema
    };
    resolve_reference(
        frame.context,
        ref_root,
        &frame.lexical_base,
        recursive_ref,
        false,
    )
}

/// Source `ResolveDynamicRef` (`DynamicRef`).
fn resolve_dynamic_ref<'a>(
    frame: &StackFrame<'a>,
    dynamic_ref: &str,
) -> Result<Option<&'a Value>, SchemaError> {
    let initial_base = parse_url(base_or_dot(&frame.lexical_base))?;
    let fragment_root = if dynamic_ref.starts_with('#') {
        frame.lexical_schema
    } else {
        frame.root
    };
    let fragment_target = resolve_reference(
        frame.context,
        fragment_root,
        &frame.lexical_base,
        dynamic_ref,
        false,
    )?;
    let Some(fragment_target) = fragment_target else {
        let hash = js_hash(&join_url(&initial_base, dynamic_ref)?);
        if !hash.starts_with("#/") && hash.starts_with('#') && hash.len() > 1 {
            let name = decode_uri_component(&hash[1..])?;
            let anchor_target = frame
                .dynamic_anchors
                .iter()
                .copied()
                .find(|anchor| guards::get_str(anchor, "$dynamicAnchor") == Some(name.as_str()))
                .or_else(|| find_dynamic_anchor(frame.root, &name));
            return Ok(anchor_target);
        }
        return Ok(None);
    };
    if !fragment_target.is_object() || !guards::is_dynamic_anchor(fragment_target) {
        return Ok(Some(fragment_target));
    }
    let hash = js_hash(&join_url(&initial_base, dynamic_ref)?);
    if hash.starts_with("#/") {
        return Ok(Some(fragment_target));
    }
    let target_name = guards::get_str(fragment_target, "$dynamicAnchor");
    let anchor_target = frame
        .dynamic_anchors
        .iter()
        .copied()
        .find(|anchor| guards::get_str(anchor, "$dynamicAnchor") == target_name);
    Ok(anchor_target.or(Some(fragment_target)))
}
