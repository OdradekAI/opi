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
use std::rc::Rc;

use serde_json::Value;

use super::SchemaError;
use super::guards;
use super::js::js_keys;
use super::proof::{Orbit, ProofContext, ReferenceUrl as Url};
use super::pump::Equality;

const DEFAULT_BASE: &str = "https://json-schema.org";

/// Snapshot of the traversal state handed to resolution (source
/// `_Stack_StackFrame`). Owns cloned anchor/ids vectors so the stack can be
/// mutated while the snapshot is live.
pub(crate) struct StackFrame<'a> {
    pub(crate) context: &'a serde_json::Map<String, Value>,
    pub(crate) root: &'a Value,
    pub(crate) ids: Vec<&'a Value>,
    pub(crate) lexical_schema: &'a Value,
    pub(crate) lexical_base: Url,
    pub(crate) reference_base: Url,
    pub(crate) resource_base: Url,
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
    pub(crate) base: Url,
    pub(crate) root: &'a Value,
}

pub(crate) struct ResolvedResource<'a> {
    pub(crate) target: &'a Value,
    pub(crate) resource: &'a Value,
}

#[derive(Clone)]
struct RetrievedFrameEntry<'a> {
    root: &'a Value,
    base: Url,
    id_depth: usize,
    resource_depth: usize,
}

/// Traversal stack (source `Stack`).
#[derive(Clone)]
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
    /// The source Map retains one record per target; re-registration can
    /// replace that record after applying another resource-stack effect.
    resolved_resources: Vec<(*const Value, &'a Value)>,
    pending_resource: bool,
    proof: Option<Rc<StackProof<'a>>>,
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
            proof: None,
        }
    }

    pub(crate) fn lexical_base_url(&self) -> Result<String, SchemaError> {
        self.lexical_url().map(|url| url.to_string())
    }

    pub(crate) fn lexical_url(&self) -> Result<Url, SchemaError> {
        self.base_url(false)
    }

    /// Source `_Stack_BuildBase`: resolve the scoped `$id` chain over the
    /// current retrieved frame's base (or the default base).
    fn build_base(&self, stack: &[&'a Value]) -> Result<Url, SchemaError> {
        let frame = self.retrieved_frames.last();
        let mut url = match frame {
            Some(frame) => frame.base.clone(),
            None => parse_url(DEFAULT_BASE)?,
        };
        let scoped: &[&'a Value] = match frame {
            Some(frame) => stack.get(frame.id_depth..).unwrap_or_default(),
            None => stack,
        };
        for schema in scoped {
            if let Some(id) = guards::get_str(schema, "$id") {
                url = join_url(&url, id)?;
            }
        }
        Ok(url)
    }

    fn resource_base_url(&self) -> Result<Url, SchemaError> {
        self.base_url(true)
    }

    fn base_url(&self, resource: bool) -> Result<Url, SchemaError> {
        if let Some(proof) = &self.proof {
            if let Some(orbit) = proof.base(self, resource) {
                return Ok(Url::proven(orbit, proof.context.clone()));
            }
            proof.context.fail();
        }
        let stack = if resource {
            &self.resource_ids
        } else {
            &self.ids
        };
        let frame = self.retrieved_frames.last();
        match frame {
            Some(frame) if resource => {
                let mut url = frame.base.clone();
                // Source Array.slice returns empty when an older resolved
                // record has consumed IDs below the retrieved frame depth.
                for schema in stack.get(frame.resource_depth..).unwrap_or_default() {
                    if let Some(id) = guards::get_str(schema, "$id") {
                        url = join_url(&url, id)?;
                    }
                }
                Ok(url)
            }
            _ => self.build_base(stack),
        }
    }

    fn reference_base_url(&self) -> Result<Url, SchemaError> {
        if !self.retrieved_frames.is_empty() {
            return self.resource_base_url();
        }
        if let Some(lexical) = self.ids.last().copied()
            && let Some(id) = guards::get_str(lexical, "$id")
            && !has_scheme(id)
        {
            return self.lexical_url();
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
            lexical_base: self.lexical_url()?,
            reference_base: self.reference_base_url()?,
            resource_base: self.resource_base_url()?,
            recursive_anchors: self.recursive_anchors.clone(),
            dynamic_anchors: self.dynamic_anchors.clone(),
            in_retrieved_frame: !self.retrieved_frames.is_empty(),
        })
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
        self.check_proof_prefix();
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
        self.check_proof_prefix();
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
        let is_resource = self
            .resource_ids
            .last()
            .is_some_and(|last| std::ptr::eq(*last, schema));
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
        // Source Map.set replaces the record without undoing the preceding
        // registration. A later Pop consumes that target's sole record once.
        if let Some((_, current)) = self
            .resolved_resources
            .iter_mut()
            .find(|(key, _)| std::ptr::eq(*key, target))
        {
            *current = resource;
        } else {
            self.resolved_resources
                .push((target as *const Value, resource));
        }
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

    pub(crate) fn proof_replay(&self, next: &Self, context: Rc<ProofContext>) -> Option<Self> {
        let proof = Rc::new(StackProof::new(self, next, context.clone())?);
        let mut replay = self.clone();
        replay.proof = Some(proof);
        for (key, entry) in &mut replay.retrieved_resources {
            let other = next.retrieved_resources.get(key)?;
            let orbit = Orbit::between(entry.base.actual(), other.base.actual(), &context.pump)?;
            entry.base = Url::proven(orbit, context.clone());
        }
        Some(replay)
    }

    pub(crate) fn proof_closed(&self) -> bool {
        self.proof.as_ref().is_some_and(|proof| proof.closed(self))
    }

    fn check_proof_prefix(&self) {
        if let Some(proof) = &self.proof
            && !proof.prefix_intact(self)
        {
            proof.context.fail();
        }
    }
}

/// Symbolic stack at iteration n: the protected prefix followed by n copies
/// of the observed extension and the current iteration's ordinary suffix.
/// Replay may only change the suffix. URL folds prove the effect of repeating
/// an ID word; retrieved bases carry their own exact path orbits.
struct StackProof<'a> {
    before: RefStack<'a>,
    after: RefStack<'a>,
    lexical: Orbit,
    resource: Orbit,
    context: Rc<ProofContext>,
}

fn same_nodes(left: &[&Value], right: &[&Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| std::ptr::eq(*left, *right))
}

fn node_extension<'a>(before: &[&'a Value], after: &[&'a Value]) -> Option<Vec<&'a Value>> {
    if after.len() < before.len() || !same_nodes(before, &after[..before.len()]) {
        return None;
    }
    let extension = &after[before.len()..];
    // Presence and first-anchor lookup cannot change in any repeated block.
    if extension
        .iter()
        .any(|node| !before.iter().any(|old| std::ptr::eq(*old, *node)))
    {
        return None;
    }
    Some(extension.to_vec())
}

fn same_frame(left: &RetrievedFrameEntry<'_>, right: &RetrievedFrameEntry<'_>) -> bool {
    std::ptr::eq(left.root, right.root)
        && left.base.actual() == right.base.actual()
        && left.id_depth == right.id_depth
        && left.resource_depth == right.resource_depth
}

fn fold_ids(mut orbit: Orbit, ids: &[&Value]) -> Option<Orbit> {
    for schema in ids {
        orbit = orbit.join(guards::get_str(schema, "$id")?)?;
    }
    Some(orbit)
}

fn repeated_ids(
    initial: Orbit,
    prefix: &[&Value],
    extension: &[&Value],
    preferred: &super::pump::PathPump,
) -> Option<Orbit> {
    let initial = fold_ids(initial, prefix)?;
    if extension.is_empty() {
        return Some(initial);
    }
    if initial.pump.is_some() {
        return None;
    }
    let once = fold_ids(initial.clone(), extension)?;
    if once.value == initial.value {
        // An exact fixed point is invariant under any further repetitions.
        return Some(initial);
    }
    let growing = Orbit::between(&initial.value, &once.value, preferred)?;
    let next = fold_ids(growing.clone(), extension)?;
    next.equivalent(&growing.next()?).then_some(growing)
}

impl<'a> StackProof<'a> {
    fn new(before: &RefStack<'a>, after: &RefStack<'a>, context: Rc<ProofContext>) -> Option<Self> {
        if !before.context.is_empty()
            || before.proof.is_some()
            || after.proof.is_some()
            || before.pending_resource != after.pending_resource
            || !std::ptr::eq(before.schema, after.schema)
            || !std::ptr::eq(before.lexical_schema(), after.lexical_schema())
        {
            return None;
        }
        let ids = node_extension(&before.ids, &after.ids)?;
        let resource_ids = node_extension(&before.resource_ids, &after.resource_ids)?;
        node_extension(&before.anchors, &after.anchors)?;
        node_extension(&before.recursive_anchors, &after.recursive_anchors)?;
        node_extension(&before.dynamic_anchors, &after.dynamic_anchors)?;
        if before.ids.last().map(|node| *node as *const Value)
            != after.ids.last().map(|node| *node as *const Value)
            || before.resource_ids.last().map(|node| *node as *const Value)
                != after.resource_ids.last().map(|node| *node as *const Value)
            || before.retrieved_resources.len() != after.retrieved_resources.len()
            || before.resolved_resources.len() != after.resolved_resources.len()
            || before.retrieved_frames.len() > after.retrieved_frames.len()
            || !before
                .retrieved_frames
                .iter()
                .zip(&after.retrieved_frames)
                .all(|(a, b)| same_frame(a, b))
        {
            return None;
        }
        for (key, entry) in &before.retrieved_resources {
            let other = after.retrieved_resources.get(key)?;
            if !std::ptr::eq(entry.root, other.root) {
                return None;
            }
            Orbit::between(entry.base.actual(), other.base.actual(), &context.pump)?;
        }
        if before.resolved_resources.iter().any(|(key, resource)| {
            !after
                .resolved_resources
                .iter()
                .any(|(other_key, other)| key == other_key && std::ptr::eq(*resource, *other))
        }) {
            return None;
        }
        let lexical = Self::initial_base(before, after, false, &ids, &context)?;
        let resource = Self::initial_base(before, after, true, &resource_ids, &context)?;
        Some(Self {
            before: before.clone(),
            after: after.clone(),
            lexical,
            resource,
            context,
        })
    }

    fn initial_base(
        before: &RefStack<'a>,
        after: &RefStack<'a>,
        resource: bool,
        extension: &[&Value],
        context: &ProofContext,
    ) -> Option<Orbit> {
        let ids = if resource {
            &before.resource_ids
        } else {
            &before.ids
        };
        let next_ids = if resource {
            &after.resource_ids
        } else {
            &after.ids
        };
        let (initial, prefix) = match (
            before.retrieved_frames.last(),
            after.retrieved_frames.last(),
        ) {
            (None, None) => (
                Orbit::fixed(url::Url::parse(DEFAULT_BASE).ok()?),
                ids.as_slice(),
            ),
            (Some(frame), Some(next))
                if before.retrieved_frames.len() == after.retrieved_frames.len() =>
            {
                if !same_frame(frame, next) {
                    return None;
                }
                let depth = if resource {
                    frame.resource_depth
                } else {
                    frame.id_depth
                };
                (Orbit::fixed(frame.base.actual().clone()), ids.get(depth..)?)
            }
            (Some(frame), Some(next)) => {
                let depth = if resource {
                    frame.resource_depth
                } else {
                    frame.id_depth
                };
                let next_depth = if resource {
                    next.resource_depth
                } else {
                    next.id_depth
                };
                if !std::ptr::eq(frame.root, next.root)
                    || next_depth != depth.checked_add(extension.len())?
                    || !same_nodes(ids.get(depth..)?, next_ids.get(next_depth..)?)
                {
                    return None;
                }
                let initial =
                    Orbit::between(frame.base.actual(), next.base.actual(), &context.pump)?;
                let result = fold_ids(initial, ids.get(depth..)?)?;
                return Self::verify_base(before, after, resource, result);
            }
            _ => return None,
        };
        let result = repeated_ids(initial, prefix, extension, &context.pump)?;
        Self::verify_base(before, after, resource, result)
    }

    fn verify_base(
        before: &RefStack<'a>,
        after: &RefStack<'a>,
        resource: bool,
        result: Orbit,
    ) -> Option<Orbit> {
        (before.base_url(resource).ok()?.actual() == &result.value
            && after.base_url(resource).ok()?.actual() == &result.next()?.value)
            .then_some(result)
    }

    fn prefix_intact(&self, current: &RefStack<'a>) -> bool {
        for (prefix, stack) in [
            (&self.before.ids, &current.ids),
            (&self.before.resource_ids, &current.resource_ids),
            (&self.before.anchors, &current.anchors),
            (&self.before.recursive_anchors, &current.recursive_anchors),
            (&self.before.dynamic_anchors, &current.dynamic_anchors),
        ] {
            if stack.len() < prefix.len() || !same_nodes(prefix, &stack[..prefix.len()]) {
                return false;
            }
        }
        current.retrieved_frames.len() >= self.before.retrieved_frames.len()
            && self
                .before
                .retrieved_frames
                .iter()
                .zip(&current.retrieved_frames)
                .all(|(a, b)| same_frame(a, b))
    }

    fn base(&self, current: &RefStack<'a>, resource: bool) -> Option<Orbit> {
        if !self.prefix_intact(current) {
            return None;
        }
        let ids = if resource {
            &current.resource_ids
        } else {
            &current.ids
        };
        if current.retrieved_frames.len() > self.before.retrieved_frames.len() {
            let frame = current.retrieved_frames.last()?;
            let depth = if resource {
                frame.resource_depth
            } else {
                frame.id_depth
            };
            let prefix_length = if resource {
                self.before.resource_ids.len()
            } else {
                self.before.ids.len()
            };
            if depth < prefix_length {
                return None;
            }
            return fold_ids(frame.base.orbit.clone()?, ids.get(depth..)?);
        }
        let (initial, depth) = if resource {
            (self.resource.clone(), self.before.resource_ids.len())
        } else {
            (self.lexical.clone(), self.before.ids.len())
        };
        fold_ids(initial, ids.get(depth..)?)
    }

    fn closed(&self, current: &RefStack<'a>) -> bool {
        if self.context.failed()
            || !self.prefix_intact(current)
            || current.pending_resource != self.before.pending_resource
            || !same_nodes(&current.ids, &self.after.ids)
            || !same_nodes(&current.resource_ids, &self.after.resource_ids)
            || !same_nodes(&current.anchors, &self.after.anchors)
            || !same_nodes(&current.recursive_anchors, &self.after.recursive_anchors)
            || !same_nodes(&current.dynamic_anchors, &self.after.dynamic_anchors)
            || current.retrieved_frames.len() != self.after.retrieved_frames.len()
            || current.retrieved_resources.len() != self.before.retrieved_resources.len()
            || current.resolved_resources.len() != self.before.resolved_resources.len()
        {
            return false;
        }
        for (key, old) in &self.before.retrieved_resources {
            let Some(expected) = self.after.retrieved_resources.get(key) else {
                return false;
            };
            let Some(actual) = current.retrieved_resources.get(key) else {
                return false;
            };
            let Some(next) = Orbit::between(
                old.base.actual(),
                expected.base.actual(),
                &self.context.pump,
            )
            .and_then(|orbit| orbit.next()) else {
                return false;
            };
            if !std::ptr::eq(actual.root, old.root)
                || !actual
                    .base
                    .orbit
                    .as_ref()
                    .is_some_and(|orbit| orbit.equivalent(&next))
            {
                return false;
            }
        }
        if current.resolved_resources.iter().any(|(key, resource)| {
            !self
                .before
                .resolved_resources
                .iter()
                .any(|(other_key, other)| key == other_key && std::ptr::eq(*resource, *other))
        }) {
            return false;
        }
        if current.retrieved_frames.len() > self.before.retrieved_frames.len() {
            let Some(old) = self.before.retrieved_frames.last() else {
                return false;
            };
            let Some(next) = current.retrieved_frames.last() else {
                return false;
            };
            let Some(expected) = self.after.retrieved_frames.last() else {
                return false;
            };
            let Some(orbit) = Orbit::between(
                old.base.actual(),
                expected.base.actual(),
                &self.context.pump,
            )
            .and_then(|orbit| orbit.next()) else {
                return false;
            };
            if !same_frame(next, expected)
                || !next
                    .base
                    .orbit
                    .as_ref()
                    .is_some_and(|actual| actual.equivalent(&orbit))
            {
                return false;
            }
        }
        true
    }
}

// ------------------------------------------------------------------
// Resolve (resolve.mjs port)
// ------------------------------------------------------------------

fn parse_url(text: &str) -> Result<Url, SchemaError> {
    Url::parse(text)
}

fn join_url(base: &Url, input: &str) -> Result<Url, SchemaError> {
    base.join(input)
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
fn canonical_href(url: &Url) -> Url {
    url.without_fragment()
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
fn find_base<'a>(schema: &'a Value, base: &Url, target: &'a Value) -> Option<Url> {
    if std::ptr::eq(schema, target) {
        return Some(base.clone());
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
    let absolute = base.join_url(reference).ok()?;
    if base.equal(&absolute, Equality::Path) {
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
    if absolute_anchor.equal(reference, Equality::Full) {
        Some(schema)
    } else {
        None
    }
}

fn match_dynamic_anchor<'a>(schema: &'a Value, base: &Url, reference: &Url) -> Option<&'a Value> {
    let anchor = guards::get_str(schema, "$dynamicAnchor")?;
    let absolute_anchor = base.join(&format!("#{anchor}")).ok()?;
    if absolute_anchor.equal(reference, Equality::Full) {
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

#[derive(Clone, Copy)]
enum Reference<'a> {
    Literal(&'a str),
    Parsed(&'a Url),
}

impl Reference<'_> {
    fn text(&self) -> &str {
        match self {
            Self::Literal(text) => text,
            Self::Parsed(url) => url.as_str(),
        }
    }
}

/// Source `Ref(remotes, schema, base, ref, applySchemaId=false)`.
fn resolve_reference<'a>(
    context: &'a serde_json::Map<String, Value>,
    schema: &'a Value,
    base: &Url,
    reference: Reference<'_>,
    apply_schema_id: bool,
) -> Result<Option<&'a Value>, SchemaError> {
    let initial_base = base.clone();
    let resolved_base = if apply_schema_id && guards::is_id(schema) {
        guards::get_str(schema, "$id")
            .map(|id| join_url(&initial_base, id))
            .transpose()?
            .unwrap_or(initial_base)
    } else {
        initial_base
    };
    let initial_ref = match reference {
        Reference::Literal(text) => join_url(&resolved_base, text)?,
        Reference::Parsed(url) => resolved_base.join_url(url)?,
    };
    // Phase 1: context lookup by the raw reference string.
    if let Some(remote) = context.get(reference.text()) {
        return Ok(Some(remote));
    }
    // Phase 2: local tree search.
    if let Some(local) = from_value(schema, &resolved_base, &initial_ref) {
        return Ok(Some(local));
    }
    // Phase 3: remote document in context.
    let canonical = canonical_href(&initial_ref);
    if canonical.equal(&canonical_href(&resolved_base), Equality::Full) {
        return Ok(None);
    }
    let Some(remote_schema) = context.get(canonical.as_str()) else {
        return Ok(None);
    };
    let canonical_url = canonical;
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
        Reference::Literal(reference),
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
    let reference_url = join_url(&frame.reference_base, reference)?;
    let canonical = canonical_href(&reference_url);
    let is_remote = !canonical.equal(&frame.resource_base, Equality::Full);

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
    canonical: &Url,
    schema: &'a Value,
) -> Option<RetrievedResource<'a>> {
    let remote_root = context.get(canonical.as_str())?;
    if !remote_root.is_object() {
        return None;
    }
    Some(RetrievedResource {
        target: schema,
        base: canonical.clone(),
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
    let reference_base = frame.reference_base.clone();
    let target_base = find_base(frame.lexical_schema, &reference_base, schema)?;
    if target_base.equal(&frame.reference_base, Equality::Full) {
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
    canonical: &Url,
    schema: &'a Value,
) -> Option<ResolvedResource<'a>> {
    if guards::is_id(schema) {
        return None;
    }
    let reference_base = frame.reference_base.clone();
    let resource = resource(frame.context, frame.root, &reference_base, canonical)?;
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
    base: &Url,
    reference: &Url,
) -> Option<&'a Value> {
    let result = resolve_reference(context, schema, base, Reference::Parsed(reference), false)
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
        Reference::Literal(recursive_ref),
        false,
    )
}

/// Source `ResolveDynamicRef` (`DynamicRef`).
fn resolve_dynamic_ref<'a>(
    frame: &StackFrame<'a>,
    dynamic_ref: &str,
) -> Result<Option<&'a Value>, SchemaError> {
    let initial_base = frame.lexical_base.clone();
    let fragment_root = if dynamic_ref.starts_with('#') {
        frame.lexical_schema
    } else {
        frame.root
    };
    let fragment_target = resolve_reference(
        frame.context,
        fragment_root,
        &frame.lexical_base,
        Reference::Literal(dynamic_ref),
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
