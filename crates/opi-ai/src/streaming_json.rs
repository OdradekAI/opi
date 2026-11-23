//! Incremental, caller-owned tool argument previews.
//!
//! Feed one owner per tool with raw deltas. Parsing and snapshot materialization
//! are separate: appending does not repeatedly parse the accumulated prefix.
//! A snapshot is an independent JSON value, not permission to execute a tool.

use std::collections::HashMap;

use serde_json::{Map, Number, Value};

/// Incremental preview of one tool's textual arguments.
///
/// `append` retains raw UTF-8 text and scans only new UTF-16 code units. Member
/// insertion uses a hash table; parser storage is proportional to received text.
/// `snapshot` visits and copies the visible value and orders JSON object keys.
/// Taking an owned snapshot after every tiny chunk can therefore cost quadratic
/// total output copying. Callers choose their display cadence independently.
///
/// Missing keys/values are omitted, incomplete escapes are held, and malformed
/// text is read tolerantly. Numbers/literals can appear before their delimiter.
/// Preview numbers use finite IEEE-754 values; finalization preserves raw text.
/// Large open containers can exhaust the snapshot copy
/// credit and temporarily return the previous value. Each append earns
/// `256 + 8 * UTF-16 length`; a refresh spends the open containers' member/item
/// counts plus one per container and one for a pending scalar. Empty appends
/// earn credit; repeated snapshots do not. Match append/snapshot cadence when
/// comparing with Pi's argument stream, which requests a snapshot per append.
///
/// Snapshots always contain an object. Non-object roots show `{}`. A value or
/// key containing an unpaired UTF-16 surrogate delays publication until it is
/// representable as Rust Unicode, preserving the previous snapshot meanwhile.
/// Objects retain insertion order, with canonical JavaScript array-index keys
/// first in numeric order. Snapshots exceeding 128 nested containers retain the
/// previous value, bounding recursive clone/drop in the returned JSON type.
/// A reachable non-finite number also retains the entire previous snapshot,
/// preserving array positions. Replacing the offending subtree can resume
/// publication. These representation limits never discard the raw input.
/// Raw text is retained without modification. `finish` uses the existing strict
/// final-argument contract and neither consumes nor resets this owner.
///
/// ```
/// use opi_ai::streaming_json::ToolCallArgumentsStream;
/// let mut args = ToolCallArgumentsStream::new();
/// args.append(r#"{"enabled":"#);
/// let before = args.snapshot();
/// args.append("true}");
/// assert_eq!(before, serde_json::json!({}));
/// assert_eq!(args.snapshot(), serde_json::json!({"enabled": true}));
/// assert!(args.finish().1.is_none());
/// ```
#[derive(Debug, Default)]
pub struct ToolCallArgumentsStream {
    text: String,
    parser: Parser,
}

impl ToolCallArgumentsStream {
    /// Construct an empty per-tool owner with no network or runtime work.
    pub fn new() -> Self {
        Self::default()
    }

    /// The exact text supplied so far, including any ignored trailing input.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Retain and parse a new fragment; request a preview with `snapshot`.
    pub fn append(&mut self, delta: &str) {
        self.text.push_str(delta);
        self.parser.append(delta);
    }

    /// Replace the complete text. An extension returns its raw suffix, including
    /// `Some("")` for identical text. A non-prefix replacement resets all state
    /// and returns `None`. Identical text does not earn append credit.
    pub fn replace<'a>(&mut self, text: &'a str) -> Option<&'a str> {
        if let Some(delta) = text.strip_prefix(&self.text) {
            if !delta.is_empty() {
                self.append(delta);
            }
            Some(delta)
        } else {
            *self = Self::new();
            self.append(text);
            None
        }
    }

    /// Return an independent, display-only JSON object. Later input, replacement
    /// or dropping this owner cannot mutate a retained value; caller edits to
    /// that value cannot mutate this owner.
    pub fn snapshot(&mut self) -> Value {
        self.parser.snapshot()
    }

    /// Strictly finalize the complete raw text, returning `(text, error)` under
    /// [`crate::message::finalize_tool_arguments`]. A partial preview never turns
    /// incomplete/non-object final text into a valid call. May be called again
    /// after more input; the provider's terminal message remains authoritative.
    pub fn finish(&self) -> (String, Option<String>) {
        crate::message::finalize_tool_arguments(self.text.clone())
    }
}

type NodeId = usize;

#[derive(Debug)]
enum Node {
    Scalar(Scalar),
    String(Vec<u16>),
    Object(Members),
    Array(Vec<NodeId>),
}

#[derive(Debug, Clone)]
enum Scalar {
    Json(Value),
    NonFinite,
}

impl Scalar {
    fn json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            Self::NonFinite => None,
        }
    }
}

#[derive(Debug, Default)]
struct Members {
    positions: HashMap<Vec<u16>, usize>,
    entries: Vec<(Vec<u16>, NodeId)>,
}

impl Members {
    fn len(&self) -> usize {
        self.entries.len()
    }

    fn insert(&mut self, key: Vec<u16>, node: NodeId) -> bool {
        if let Some(index) = self.positions.get(&key) {
            self.entries[*index].1 = node;
            false
        } else {
            self.positions.insert(key.clone(), self.entries.len());
            self.entries.push((key, node));
            true
        }
    }
}

#[derive(Debug, Clone)]
enum Slot {
    Root,
    Member(NodeId, Vec<u16>),
    Element(NodeId, usize),
}

#[derive(Debug, PartialEq)]
enum Phase {
    Key,
    Colon,
    Value,
    Comma,
}

#[derive(Debug)]
struct Frame {
    node: NodeId,
    key: Option<Vec<u16>>,
    phase: Phase,
}

#[derive(Debug)]
struct OpenString {
    node: NodeId,
    key: bool,
    visible: bool,
    escape: Vec<u16>,
}

#[derive(Debug)]
struct Parser {
    nodes: Vec<Node>,
    root: Option<NodeId>,
    stack: Vec<Frame>,
    string: Option<OpenString>,
    scalar: Vec<u16>,
    scalar_slot: Option<Slot>,
    path_cost: usize,
    credit: usize,
    dirty: bool,
    done: bool,
    cached: Value,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            stack: Vec::new(),
            string: None,
            scalar: Vec::new(),
            scalar_slot: None,
            path_cost: 0,
            credit: 0,
            dirty: false,
            done: false,
            cached: Value::Object(Map::new()),
        }
    }
}

impl Parser {
    fn append(&mut self, text: &str) {
        self.credit = self
            .credit
            .saturating_add(256)
            .saturating_add(text.encode_utf16().count().saturating_mul(8));
        for unit in text.encode_utf16() {
            if self.done {
                break;
            }
            if let Some(mut open) = self.string.take() {
                if self.read_string(&mut open, unit) {
                    self.string = Some(open);
                }
                continue;
            }
            if !self.scalar.is_empty() {
                if !is_delimiter(unit) {
                    self.scalar.push(unit);
                    self.dirty |= self.scalar_slot.is_some();
                    continue;
                }
                self.end_scalar();
                if self.done {
                    break;
                }
            }
            self.structural(unit);
        }
    }

    fn new_node(&mut self, node: Node) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    fn value_slot(&self) -> Option<Slot> {
        let Some(frame) = self.stack.last() else {
            return self.root.is_none().then_some(Slot::Root);
        };
        match &self.nodes[frame.node] {
            Node::Array(items) => Some(Slot::Element(frame.node, items.len())),
            Node::Object(_) if matches!(frame.phase, Phase::Value | Phase::Colon) => frame
                .key
                .as_ref()
                .map(|key| Slot::Member(frame.node, key.clone())),
            _ => None,
        }
    }

    fn place(&mut self, slot: Option<Slot>, node: NodeId) {
        let Some(slot) = slot else {
            return;
        };
        self.dirty = true;
        match slot {
            Slot::Root => self.root = Some(node),
            Slot::Member(parent, key) => {
                let Node::Object(members) = &mut self.nodes[parent] else {
                    unreachable!()
                };
                if members.insert(key, node) {
                    self.path_cost += 1;
                }
            }
            Slot::Element(parent, index) => {
                let Node::Array(items) = &mut self.nodes[parent] else {
                    unreachable!()
                };
                if index == items.len() {
                    items.push(node);
                    self.path_cost += 1;
                } else {
                    items[index] = node;
                }
            }
        }
    }

    fn structural(&mut self, unit: u16) {
        match unit {
            0x20 | 0x09 | 0x0a | 0x0d => {}
            0x22 => {
                let key = self.stack.last().is_some_and(|frame| {
                    matches!(self.nodes[frame.node], Node::Object(_))
                        && matches!(frame.phase, Phase::Key | Phase::Comma)
                });
                let slot = if key { None } else { self.value_slot() };
                let visible = slot.is_some();
                let node = self.new_node(Node::String(Vec::new()));
                self.place(slot, node);
                self.string = Some(OpenString {
                    node,
                    key,
                    visible,
                    escape: Vec::new(),
                });
            }
            0x7b | 0x5b => {
                let slot = self.value_slot();
                let object = unit == 0x7b;
                let node = self.new_node(if object {
                    Node::Object(Members::default())
                } else {
                    Node::Array(Vec::new())
                });
                self.place(slot, node);
                self.stack.push(Frame {
                    node,
                    key: None,
                    phase: if object { Phase::Key } else { Phase::Value },
                });
                self.path_cost += 1;
                self.dirty = true;
            }
            0x7d | 0x5d => {
                if let Some(frame) = self.stack.pop() {
                    let size = match &self.nodes[frame.node] {
                        Node::Object(members) => members.len(),
                        Node::Array(items) => items.len(),
                        _ => unreachable!(),
                    };
                    self.path_cost -= size + 1;
                    self.dirty = true;
                    self.value_ended();
                }
            }
            0x3a => {
                if let Some(frame) = self.stack.last_mut()
                    && matches!(self.nodes[frame.node], Node::Object(_))
                    && frame.phase == Phase::Colon
                {
                    frame.phase = Phase::Value;
                }
            }
            0x2c => {
                if let Some(frame) = self.stack.last_mut() {
                    frame.phase = if matches!(self.nodes[frame.node], Node::Array(_)) {
                        Phase::Value
                    } else {
                        Phase::Key
                    };
                }
            }
            _ => {
                self.scalar.push(unit);
                self.scalar_slot = self.value_slot();
                self.dirty |= self.scalar_slot.is_some();
            }
        }
    }

    fn value_ended(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.phase = Phase::Comma;
        } else if self.root.is_some() {
            self.done = true;
        }
    }

    fn string_units(&mut self, open: &OpenString, units: &[u16]) {
        let Node::String(text) = &mut self.nodes[open.node] else {
            unreachable!()
        };
        text.extend_from_slice(units);
        self.dirty |= open.visible;
    }

    // Returns whether the string is still open. UTF-16 units remain internal
    // until snapshot conversion, so an escaped pair can span any input boundary.
    fn read_string(&mut self, open: &mut OpenString, unit: u16) -> bool {
        if !open.escape.is_empty() {
            if open.escape.len() == 1 {
                let decoded = match unit {
                    0x22 | 0x5c | 0x2f => Some(unit),
                    0x62 => Some(8),
                    0x66 => Some(12),
                    0x6e => Some(10),
                    0x72 => Some(13),
                    0x74 => Some(9),
                    _ => None,
                };
                if let Some(decoded) = decoded {
                    open.escape.clear();
                    self.string_units(open, &[decoded]);
                    return true;
                }
                if unit == 0x75 {
                    open.escape.push(unit);
                    return true;
                }
            } else if char::from_u32(u32::from(unit)).is_some_and(|ch| ch.is_ascii_hexdigit()) {
                open.escape.push(unit);
                if open.escape.len() == 6 {
                    let mut decoded = 0u16;
                    for hex in &open.escape[2..] {
                        decoded = decoded * 16
                            + char::from_u32(u32::from(*hex))
                                .unwrap()
                                .to_digit(16)
                                .unwrap() as u16;
                    }
                    open.escape.clear();
                    self.string_units(open, &[decoded]);
                }
                return true;
            }
            let literal = std::mem::take(&mut open.escape);
            self.string_units(open, &literal);
        }
        match unit {
            0x22 => {
                if open.key {
                    let Node::String(text) = &mut self.nodes[open.node] else {
                        unreachable!()
                    };
                    if let Some(frame) = self.stack.last_mut() {
                        frame.key = Some(std::mem::take(text));
                        frame.phase = Phase::Colon;
                    }
                } else {
                    self.value_ended();
                }
                false
            }
            0x5c => {
                open.escape.push(unit);
                true
            }
            _ => {
                self.string_units(open, &[unit]);
                true
            }
        }
    }

    fn end_scalar(&mut self) {
        let token = std::mem::take(&mut self.scalar);
        let slot = self.scalar_slot.take();
        if let Some(value) = scalar_value(&token, false) {
            let node = self.new_node(Node::Scalar(value));
            self.place(slot, node);
        }
        self.value_ended();
    }

    fn snapshot(&mut self) -> Value {
        if self.dirty {
            let pending = self.scalar_slot.as_ref().and_then(|slot| {
                (self.scalar.len() <= 64)
                    .then(|| scalar_value(&self.scalar, true))
                    .flatten()
                    .map(|value| (slot, value))
            });
            let cost = self.path_cost + usize::from(pending.is_some());
            if cost <= self.credit {
                self.credit -= cost;
                self.dirty = false;
                if let Some(value) = self.materialize(pending) {
                    self.cached = if value.is_object() {
                        value
                    } else {
                        Value::Object(Map::new())
                    };
                }
            }
        }
        self.cached.clone()
    }

    // An explicit traversal avoids recursion through the mutable parser arena.
    // Only reachable nodes are copied; superseded values do not add copy cost.
    fn materialize(&self, pending: Option<(&Slot, Scalar)>) -> Option<Value> {
        if let Some((Slot::Root, value)) = &pending {
            return Some(value.json()?.clone());
        }
        let Some(root) = self.root else {
            return Some(Value::Object(Map::new()));
        };
        enum Work {
            Visit(NodeId, usize),
            Pending(Scalar),
            Object(Vec<String>),
            Array(NodeId, usize),
        }
        let mut work = vec![Work::Visit(root, 0)];
        let mut values = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(id, depth) => match &self.nodes[id] {
                    Node::Scalar(value) => values.push(value.json()?.clone()),
                    Node::String(text) => {
                        values.push(Value::String(String::from_utf16(text).ok()?))
                    }
                    Node::Object(members) => {
                        if depth == 128 {
                            return None;
                        }
                        let mut entries: Vec<_> = members
                            .entries
                            .iter()
                            .map(|(key, node)| (key, Some(*node)))
                            .collect();
                        if let Some((Slot::Member(parent, key), _)) = &pending
                            && *parent == id
                            && !members.positions.contains_key(key)
                        {
                            entries.push((key, None));
                        }
                        // Stable sorting preserves insertion order for non-index keys.
                        entries.sort_by_key(|(key, _)| array_index(key).unwrap_or(u32::MAX));
                        let keys: Option<Vec<_>> = entries
                            .iter()
                            .map(|(key, _)| String::from_utf16(key).ok())
                            .collect();
                        work.push(Work::Object(keys?));
                        work.extend(entries.iter().rev().map(|(key, node)| {
                            if let Some((Slot::Member(parent, replaced), value)) = &pending
                                && *parent == id
                                && replaced == *key
                            {
                                Work::Pending(value.clone())
                            } else {
                                Work::Visit(node.unwrap(), depth + 1)
                            }
                        }));
                    }
                    Node::Array(items) => {
                        if depth == 128 {
                            return None;
                        }
                        work.push(Work::Array(id, items.len()));
                        work.extend(items.iter().rev().map(|node| Work::Visit(*node, depth + 1)));
                    }
                },
                Work::Pending(value) => values.push(value.json()?.clone()),
                Work::Object(keys) => {
                    let children = values.split_off(values.len() - keys.len());
                    let members: Map<_, _> = keys.into_iter().zip(children).collect();
                    values.push(Value::Object(members));
                }
                Work::Array(id, count) => {
                    let mut children = values.split_off(values.len() - count);
                    if let Some((Slot::Element(parent, index), value)) = &pending
                        && *parent == id
                    {
                        if *index == children.len() {
                            children.push(value.json()?.clone());
                        } else {
                            children[*index] = value.json()?.clone();
                        }
                    }
                    values.push(Value::Array(children));
                }
            }
        }
        values.pop()
    }
}

fn array_index(key: &[u16]) -> Option<u32> {
    if key.len() > 1 && key.first() == Some(&u16::from(b'0')) {
        return None;
    }
    let text = String::from_utf16(key).ok()?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>().ok().filter(|index| *index != u32::MAX)
}

fn is_delimiter(unit: u16) -> bool {
    matches!(
        unit,
        0x20 | 0x09 | 0x0a | 0x0d | 0x2c | 0x3a | 0x7b | 0x7d | 0x5b | 0x5d | 0x22
    )
}

fn scalar_value(token: &[u16], pending: bool) -> Option<Scalar> {
    let token = String::from_utf16(token).ok()?;
    if token.is_empty() {
        return None;
    }
    for (literal, value) in [
        ("true", Value::Bool(true)),
        ("false", Value::Bool(false)),
        ("null", Value::Null),
    ] {
        if token == literal || (pending && literal.starts_with(&token)) {
            return Some(Scalar::Json(value));
        }
    }
    // A partial exponent uses its complete mantissa. A partial fraction does
    // not invent digits. IgnoredAny validates JSON grammar without imposing
    // serde_json Number's finite range; f64 then distinguishes representable
    // values from overflow. Keep overflow in its slot so it cannot shift an
    // array or reveal a superseded member during materialization.
    let candidate = if pending {
        token
            .find(['e', 'E'])
            .filter(|index| matches!(&token[index + 1..], "" | "+" | "-"))
            .map_or(token.as_str(), |index| &token[..index])
    } else {
        &token
    };
    serde_json::from_str::<serde::de::IgnoredAny>(candidate).ok()?;
    Some(match Number::from_f64(candidate.parse().ok()?) {
        Some(number) => Scalar::Json(Value::Number(number)),
        None => Scalar::NonFinite,
    })
}
