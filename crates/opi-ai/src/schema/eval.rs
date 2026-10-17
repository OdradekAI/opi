//! The dialect evaluator: an explicit work-stack machine porting the pinned
//! TypeBox 1.3.27 `Compile`/`Build` evaluation semantics (tree SHA
//! `ebf39b14…`).
//!
//! Two modes, selected at admission by whether any `unevaluatedItems` /
//! `unevaluatedProperties` keyword is structurally present (upstream
//! `HasUnevaluated`):
//!
//! * fast mode — pure boolean evaluation; `allOf`/`anyOf` short-circuit like
//!   the emitted `&&`/`||` chains, `oneOf` counts all arms, and no evaluated
//!   facts are recorded.
//! * reducer mode — every container-child node pushes an evaluated-facts
//!   frame; `allOf`/`anyOf`/`oneOf`/`not` evaluate every arm in a fresh
//!   context and merge the successful arms' facts (upstream `_reducer.mjs` /
//!   `_context.mjs`); `contains`/`minContains` record matched indices.
//!
//! Traversal uses heap frames so schema depth cannot exhaust the Rust call
//! stack. Re-entering the exact `(schema node, instance location, reference
//! frame)` state that is still active on the current path is a typed
//! `NoProgress` failure — productive recursion (instance descent or frame
//! change) continues, matching the upstream's RangeError boundary without
//! crashing. There are no depth, node, arm or error-count limits and no I/O.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::SchemaError;
use super::formats;
use super::grapheme;
use super::guards;
use super::hashing;
use super::ir::{Compiled, NodeRegexes};
use super::js::{js_deep_equal, js_keys, type_matches};
use super::refs::RefStack;

const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
const FNV_PRIME: u64 = 1_099_511_628_211;

fn hash_mix(state: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *state ^= u64::from(*byte);
        *state = state.wrapping_mul(FNV_PRIME);
    }
}

#[derive(Default, Clone)]
struct CtxFrame {
    indices: HashSet<u64>,
    keys: HashSet<String>,
}

/// Synthetic child values (object keys validated as strings).
type ValueRef<'a> = Cow<'a, Value>;

fn as_value<'v>(value: &'v ValueRef<'_>) -> &'v Value {
    value.as_ref()
}

/// Input delivered to the top frame on each step.
enum Input {
    Start,
    Child(bool),
}

enum Step<'a> {
    Done(bool),
    Child(Frame<'a>),
}

/// First failing leaf observed during evaluation (public failure reports).
///
/// Recorded for observation only: the verdict logic is untouched, so the
/// acceptance set of the engine is unchanged. `path` is the JSON pointer of
/// the failing instance location ("" is the root); `reason` is redacted —
/// schema-derived bounds only, never instance content.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FailureSite {
    pub(crate) path: String,
    pub(crate) keyword: &'static str,
    pub(crate) reason: String,
}

/// JSON-pointer token escaping (RFC 6901 `~0`/`~1`).
fn pointer_escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn child_pointer(base: &str, token: &str) -> String {
    format!("{}/{}", base, pointer_escape(token))
}

fn index_pointer(base: &str, idx: usize) -> String {
    format!("{base}/{idx}")
}

// ------------------------------------------------------------------
// Frames
// ------------------------------------------------------------------

/// Keyword pipeline for one schema node.
struct NodeFrame<'a> {
    schema: &'a Value,
    value: ValueRef<'a>,
    /// JSON pointer of this node's instance location ("" is the root);
    /// used only for failure reporting.
    path: String,
    stages: Vec<Stage<'a>>,
    stage_idx: usize,
    visit_key: Option<u64>,
    ctx_pushed: bool,
    /// Container-child nodes push an evaluated-facts frame (upstream
    /// `BuildSchemaPushStack` sites: properties, items, prefixItems,
    /// additionalProperties, additionalItems, patternProperties).
    ctx_push: bool,
}

#[derive(Clone)]
enum Stage<'a> {
    // leaf stages (evaluated inline during the scan)
    TypeLeaf,
    RequiredLeaf,
    DependentRequiredLeaf,
    MinPropertiesLeaf,
    MaxPropertiesLeaf,
    MaxItemsLeaf,
    MinItemsLeaf,
    UniqueItemsLeaf,
    MaxLengthLeaf,
    MinLengthLeaf,
    FormatLeaf,
    PatternLeaf,
    ExclusiveMaximumLeaf,
    ExclusiveMinimumLeaf,
    MaximumLeaf,
    MinimumLeaf,
    MultipleOfLeaf,
    ConstLeaf,
    EnumLeaf,
    // child stages
    AdditionalProperties {
        node: &'a Value,
        schema: &'a Value,
    },
    Dependencies {
        entries: Vec<(&'a str, &'a Value)>,
        empty_shortcut: bool,
    },
    PatternProperties {
        node: &'a Value,
        patterns: &'a Vec<(String, regress::Regex)>,
    },
    Properties {
        entries: Vec<(&'a str, &'a Value)>,
    },
    PropertyNames {
        schema: &'a Value,
        keys: Vec<String>,
    },
    AdditionalItems {
        schema: &'a Value,
        items_len: usize,
    },
    Contains {
        schema: &'a Value,
        min_contains_zero: bool,
    },
    Items {
        node: &'a Value,
    },
    MinContains {
        schema: &'a Value,
        limit: f64,
    },
    MaxContains {
        schema: &'a Value,
        limit: f64,
    },
    PrefixItems {
        items: &'a [Value],
    },
    IfThenElse {
        schema: &'a Value,
    },
    ResolveRef,
    ResolveRecursiveRef,
    ResolveDynamicRef,
    Not,
    Arms {
        kind: ArmKind,
        arms: &'a [Value],
    },
    UnevaluatedItems {
        schema: &'a Value,
    },
    UnevaluatedProperties {
        schema: &'a Value,
        keys: Vec<String>,
    },
}

/// Upstream keyword names kept verbatim (`allOf`/`anyOf`/`oneOf`).
#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, PartialEq)]
enum ArmKind {
    AllOf,
    AnyOf,
    OneOf,
}

struct ArmsFrame<'a> {
    /// Instance pointer of the arms' location (failure reporting only).
    base_path: String,
    kind: ArmKind,
    arms: &'a [Value],
    value: ValueRef<'a>,
    arm_idx: usize,
    passed: usize,
    passed_facts: Vec<CtxFrame>,
    saved_ctx: Option<Vec<CtxFrame>>,
    /// `not` semantics over a single arm (negation; no fact merge).
    not_mode: bool,
}

struct RefFrame<'a> {
    /// Instance pointer of the reference's location (failure reporting only).
    base_path: String,
    target: &'a Value,
    value: ValueRef<'a>,
    saved_ctx: Vec<CtxFrame>,
}

struct ContainsFrame<'a> {
    /// Instance pointer of the containing array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    idx: usize,
    matched: bool,
    record_indices: bool,
}

struct MinContainsFrame<'a> {
    /// Instance pointer of the containing array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    idx: usize,
    limit: f64,
    count: u64,
    record_indices: bool,
}

struct MaxContainsFrame<'a> {
    /// Instance pointer of the containing array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    idx: usize,
    limit: f64,
    count: u64,
}

struct ItemsSizedFrame<'a> {
    /// Instance pointer of the array (failure reporting only).
    base_path: String,
    items: &'a [Value],
    array: &'a [Value],
    idx: usize,
}

struct ItemsUnsizedFrame<'a> {
    /// Instance pointer of the array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    idx: usize,
}

struct AdditionalItemsFrame<'a> {
    /// Instance pointer of the array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    items_len: usize,
    idx: usize,
}

struct PrefixItemsFrame<'a> {
    /// Instance pointer of the array (failure reporting only).
    base_path: String,
    items: &'a [Value],
    array: &'a [Value],
    idx: usize,
}

struct PropertiesFrame<'a> {
    /// Instance pointer of the object (failure reporting only).
    base_path: String,
    entries: Vec<(&'a str, &'a Value)>,
    map: &'a serde_json::Map<String, Value>,
    idx: usize,
}

struct PatternPropertiesFrame<'a> {
    /// Instance pointer of the object (failure reporting only).
    base_path: String,
    patterns: &'a Vec<(String, regress::Regex)>,
    pattern_map: &'a serde_json::Map<String, Value>,
    value_map: &'a serde_json::Map<String, Value>,
    pattern_idx: usize,
    key_idx: usize,
    keys: Vec<String>,
}

struct AdditionalPropsFrame<'a> {
    /// Instance pointer of the object (failure reporting only).
    base_path: String,
    node: &'a Value,
    schema: &'a Value,
    value_map: &'a serde_json::Map<String, Value>,
    keys: Vec<String>,
    idx: usize,
}

struct PropertyNamesFrame<'a> {
    /// Instance pointer of the object (failure reporting only; name-string
    /// children report at the object location, matching the upstream).
    base_path: String,
    schema: &'a Value,
    keys: Vec<String>,
    idx: usize,
}

struct DepsFrame<'a> {
    /// Instance pointer of the object (failure reporting only).
    base_path: String,
    entries: Vec<(&'a str, &'a Value)>,
    value: ValueRef<'a>,
    idx: usize,
}

struct IfThenElseFrame<'a> {
    /// Instance pointer of the condition's location (failure reporting only).
    base_path: String,
    schema: &'a Value,
    value: ValueRef<'a>,
    phase: IfPhase,
}

enum IfPhase {
    Condition,
    Branch,
}

struct UnevaluatedItemsFrame<'a> {
    /// Instance pointer of the array (failure reporting only).
    base_path: String,
    schema: &'a Value,
    array: &'a [Value],
    idx: usize,
}

struct UnevaluatedPropsFrame<'a> {
    /// Instance pointer of the object (failure reporting only).
    base_path: String,
    schema: &'a Value,
    value_map: &'a serde_json::Map<String, Value>,
    keys: Vec<String>,
    idx: usize,
}

enum Frame<'a> {
    Node(NodeFrame<'a>),
    Arms(ArmsFrame<'a>),
    Ref(RefFrame<'a>),
    Contains(ContainsFrame<'a>),
    MinContains(MinContainsFrame<'a>),
    MaxContains(MaxContainsFrame<'a>),
    ItemsSized(ItemsSizedFrame<'a>),
    ItemsUnsized(ItemsUnsizedFrame<'a>),
    AdditionalItems(AdditionalItemsFrame<'a>),
    PrefixItems(PrefixItemsFrame<'a>),
    Properties(PropertiesFrame<'a>),
    PatternProperties(PatternPropertiesFrame<'a>),
    AdditionalProps(AdditionalPropsFrame<'a>),
    PropertyNames(PropertyNamesFrame<'a>),
    Deps(DepsFrame<'a>),
    IfThenElse(IfThenElseFrame<'a>),
    UnevaluatedItems(UnevaluatedItemsFrame<'a>),
    UnevaluatedProps(UnevaluatedPropsFrame<'a>),
}

// ------------------------------------------------------------------
// Evaluator
// ------------------------------------------------------------------

pub(crate) struct Evaluator<'a> {
    /// First observed failure (observation only; verdicts are unchanged).
    pub(crate) first_failure: Option<FailureSite>,
    root: &'a Value,
    stack: RefStack<'a>,
    reducer: bool,
    regexes: &'a HashMap<*const Value, NodeRegexes>,
    ctx: Vec<CtxFrame>,
    active: HashMap<u64, usize>,
    work: Vec<Frame<'a>>,
}

impl<'a> Evaluator<'a> {
    pub(crate) fn new(compiled: &'a Compiled) -> Self {
        let mut ctx = Vec::new();
        if compiled.reducer {
            ctx.push(CtxFrame::default());
        }
        Self {
            first_failure: None,
            root: &compiled.schema,
            stack: RefStack::new(empty_remotes(), &compiled.schema),
            reducer: compiled.reducer,
            regexes: &compiled.regexes,
            ctx,
            active: HashMap::new(),
            work: Vec::new(),
        }
    }

    pub(crate) fn run(&mut self, value: &'a Value) -> Result<bool, SchemaError> {
        let root_schema = self.root;
        self.work.push(Frame::Node(NodeFrame {
            schema: root_schema,
            value: Cow::Borrowed(value),
            path: String::new(),
            stages: Vec::new(),
            stage_idx: 0,
            visit_key: None,
            ctx_pushed: false,
            ctx_push: false,
        }));
        let mut input = Input::Start;
        loop {
            let mut frame = self.work.pop().expect("work stack cannot be empty at step");
            let step = frame.step(self, input)?;
            match step {
                Step::Child(child) => {
                    self.work.push(frame);
                    self.work.push(child);
                    input = Input::Start;
                }
                Step::Done(result) => {
                    if self.work.is_empty() {
                        return Ok(result);
                    }
                    input = Input::Child(result);
                }
            }
        }
    }

    // -- context helpers ---------------------------------------------------

    fn add_key(&mut self, key: &str) {
        if self.reducer
            && let Some(top) = self.ctx.last_mut()
        {
            top.keys.insert(key.to_string());
        }
    }

    fn add_index(&mut self, index: u64) {
        if self.reducer
            && let Some(top) = self.ctx.last_mut()
        {
            top.indices.insert(index);
        }
    }

    fn top_indices(&self) -> Option<&HashSet<u64>> {
        self.ctx.last().map(|frame| &frame.indices)
    }

    fn top_keys(&self) -> Option<&HashSet<String>> {
        self.ctx.last().map(|frame| &frame.keys)
    }

    // -- ancestry ----------------------------------------------------------

    /// Record the first observed failure site (later sites keep the first,
    /// which is the deterministic failure the verdict traversal produced).
    pub(crate) fn note_failure(&mut self, path: &str, keyword: &'static str, reason: String) {
        if self.first_failure.is_none() {
            self.first_failure = Some(FailureSite {
                path: path.to_owned(),
                keyword,
                reason,
            });
        }
    }

    fn visit_key(&self, schema: &Value, value: &ValueRef<'_>) -> u64 {
        let mut state = FNV_OFFSET;
        hash_mix(&mut state, &(schema as *const Value as usize).to_le_bytes());
        match value {
            Cow::Borrowed(v) => {
                hash_mix(&mut state, &(*v as *const Value as usize).to_le_bytes());
            }
            Cow::Owned(v) => {
                // Synthetic values (validated object keys) use content
                // identity so repeated key strings collapse like upstream
                // value identity would.
                hash_mix(&mut state, v.to_string().as_bytes());
            }
        }
        self.stack.fingerprint(&mut state);
        state
    }

    fn enter_visit(&mut self, key: u64) -> Result<(), SchemaError> {
        let count = self.active.entry(key).or_insert(0);
        *count += 1;
        if *count > 1 {
            return Err(SchemaError::NoProgress);
        }
        Ok(())
    }

    fn exit_visit(&mut self, key: u64) {
        if let Some(count) = self.active.get_mut(&key) {
            *count -= 1;
            if *count == 0 {
                self.active.remove(&key);
            }
        }
    }

    fn node_regexes(&self, node: &Value) -> Option<&'a NodeRegexes> {
        self.regexes.get(&(node as *const Value))
    }
}

fn empty_remotes() -> &'static serde_json::Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Box<serde_json::Map<String, Value>>> =
        std::sync::OnceLock::new();
    EMPTY.get_or_init(Box::default)
}

// ------------------------------------------------------------------
// Stage construction and leaf evaluation
// ------------------------------------------------------------------

fn is_multiple_of(dividend: f64, divisor: f64) -> bool {
    // Source Guard.IsMultipleOf (number branch; BigInt unreachable for JSON).
    let tolerance = 1e-10;
    let dividend_is_integer = dividend.is_finite() && dividend.fract() == 0.0;
    if dividend_is_integer && (1.0 / divisor).fract() == 0.0 {
        return true;
    }
    let rem = dividend % divisor;
    rem.abs()
        .min((rem - divisor).abs())
        .min((rem + divisor).abs())
        < tolerance
}

fn covered_key(ev: &Evaluator<'_>, node: &Value, key: &str) -> bool {
    if let Some(props) = guards::get(node, "properties").and_then(Value::as_object)
        && props.contains_key(key)
    {
        return true;
    }
    ev.node_regexes(node)
        .map(|regexes| {
            regexes
                .pattern_properties
                .iter()
                .any(|(_, regex)| regex.find(key).is_some())
        })
        .unwrap_or(false)
}

fn build_stages<'a>(ev: &Evaluator<'a>, schema: &'a Value, value: &Value) -> Vec<Stage<'a>> {
    fn owned_keys(map: &serde_json::Map<String, Value>) -> Vec<String> {
        js_keys(map).into_iter().map(str::to_string).collect()
    }
    let mut stages: Vec<Stage<'a>> = Vec::new();
    if guards::is_type(schema) {
        stages.push(Stage::TypeLeaf);
    }
    if let Some(map) = value.as_object() {
        if guards::is_required(schema) {
            stages.push(Stage::RequiredLeaf);
        }
        if guards::is_additional_properties(schema) {
            stages.push(Stage::AdditionalProperties {
                node: schema,
                schema: guards::get(schema, "additionalProperties").unwrap(),
            });
        }
        if guards::is_dependencies(schema) {
            stages.push(Stage::Dependencies {
                entries: dep_entries(guards::get(schema, "dependencies").unwrap()),
                empty_shortcut: true,
            });
        }
        if guards::is_dependent_required(schema) {
            stages.push(Stage::DependentRequiredLeaf);
        }
        if guards::is_dependent_schemas(schema) {
            stages.push(Stage::Dependencies {
                entries: dep_entries(guards::get(schema, "dependentSchemas").unwrap()),
                empty_shortcut: true,
            });
        }
        if guards::is_pattern_properties(schema) {
            stages.push(Stage::PatternProperties {
                node: schema,
                patterns: pattern_list(ev, schema),
            });
        }
        if guards::is_properties(schema) {
            stages.push(Stage::Properties {
                entries: prop_entries(guards::get(schema, "properties").unwrap()),
            });
        }
        if guards::is_property_names(schema) {
            stages.push(Stage::PropertyNames {
                schema: guards::get(schema, "propertyNames").unwrap(),
                keys: owned_keys(map),
            });
        }
        if guards::is_min_properties(schema) {
            stages.push(Stage::MinPropertiesLeaf);
        }
        if guards::is_max_properties(schema) {
            stages.push(Stage::MaxPropertiesLeaf);
        }
    }
    if let Some(array) = value.as_array() {
        let _ = array;
        if guards::is_additional_items(schema) && guards::is_items_sized(schema) {
            stages.push(Stage::AdditionalItems {
                schema: guards::get(schema, "additionalItems").unwrap(),
                items_len: guards::get(schema, "items")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0),
            });
        }
        if guards::is_contains(schema) {
            let min_zero = guards::is_min_contains(schema)
                && guards::get_f64(schema, "minContains") == Some(0.0);
            stages.push(Stage::Contains {
                schema: guards::get(schema, "contains").unwrap(),
                min_contains_zero: min_zero,
            });
        }
        if guards::is_items(schema) {
            stages.push(Stage::Items { node: schema });
        }
        if guards::is_max_contains(schema) && guards::is_contains(schema) {
            stages.push(Stage::MaxContains {
                schema: guards::get(schema, "contains").unwrap(),
                limit: guards::get_f64(schema, "maxContains").unwrap(),
            });
        }
        if guards::is_max_items(schema) {
            stages.push(Stage::MaxItemsLeaf);
        }
        if guards::is_min_contains(schema) && guards::is_contains(schema) {
            stages.push(Stage::MinContains {
                schema: guards::get(schema, "contains").unwrap(),
                limit: guards::get_f64(schema, "minContains").unwrap(),
            });
        }
        if guards::is_min_items(schema) {
            stages.push(Stage::MinItemsLeaf);
        }
        if guards::is_prefix_items(schema) {
            stages.push(Stage::PrefixItems {
                items: guards::get(schema, "prefixItems")
                    .and_then(Value::as_array)
                    .unwrap(),
            });
        }
        if guards::is_unique_items(schema) {
            stages.push(Stage::UniqueItemsLeaf);
        }
    }
    if value.is_string() {
        if guards::is_max_length(schema) {
            stages.push(Stage::MaxLengthLeaf);
        }
        if guards::is_min_length(schema) {
            stages.push(Stage::MinLengthLeaf);
        }
        if guards::is_format(schema) {
            stages.push(Stage::FormatLeaf);
        }
        if guards::is_pattern(schema) {
            stages.push(Stage::PatternLeaf);
        }
    }
    if value.is_number() {
        if guards::is_exclusive_maximum(schema) {
            stages.push(Stage::ExclusiveMaximumLeaf);
        }
        if guards::is_exclusive_minimum(schema) {
            stages.push(Stage::ExclusiveMinimumLeaf);
        }
        if guards::is_maximum(schema) {
            stages.push(Stage::MaximumLeaf);
        }
        if guards::is_minimum(schema) {
            stages.push(Stage::MinimumLeaf);
        }
        if guards::is_multiple_of(schema) {
            stages.push(Stage::MultipleOfLeaf);
        }
    }
    if guards::is_ref(schema) {
        stages.push(Stage::ResolveRef);
    }
    if guards::is_recursive_ref(schema) {
        stages.push(Stage::ResolveRecursiveRef);
    }
    if guards::is_dynamic_ref(schema) {
        stages.push(Stage::ResolveDynamicRef);
    }
    if guards::is_const(schema) {
        stages.push(Stage::ConstLeaf);
    }
    if guards::is_enum(schema) {
        stages.push(Stage::EnumLeaf);
    }
    if guards::is_if(schema) {
        stages.push(Stage::IfThenElse { schema });
    }
    if guards::is_not(schema) {
        stages.push(Stage::Not);
    }
    if guards::is_all_of(schema) {
        stages.push(Stage::Arms {
            kind: ArmKind::AllOf,
            arms: guards::get(schema, "allOf")
                .and_then(Value::as_array)
                .unwrap(),
        });
    }
    if guards::is_any_of(schema) {
        stages.push(Stage::Arms {
            kind: ArmKind::AnyOf,
            arms: guards::get(schema, "anyOf")
                .and_then(Value::as_array)
                .unwrap(),
        });
    }
    if guards::is_one_of(schema) {
        stages.push(Stage::Arms {
            kind: ArmKind::OneOf,
            arms: guards::get(schema, "oneOf")
                .and_then(Value::as_array)
                .unwrap(),
        });
    }
    if guards::is_unevaluated_items(schema) && value.is_array() {
        stages.push(Stage::UnevaluatedItems {
            schema: guards::get(schema, "unevaluatedItems").unwrap(),
        });
    }
    if guards::is_unevaluated_properties(schema)
        && let Some(map) = value.as_object()
    {
        stages.push(Stage::UnevaluatedProperties {
            schema: guards::get(schema, "unevaluatedProperties").unwrap(),
            keys: owned_keys(map),
        });
    }
    stages
}

fn dep_entries(deps: &Value) -> Vec<(&str, &Value)> {
    let Some(map) = deps.as_object() else {
        return Vec::new();
    };
    js_keys(map)
        .into_iter()
        .map(|key| (key, &map[key]))
        .collect()
}

fn prop_entries(props: &Value) -> Vec<(&str, &Value)> {
    let Some(map) = props.as_object() else {
        return Vec::new();
    };
    js_keys(map)
        .into_iter()
        .map(|key| (key, &map[key]))
        .collect()
}

fn pattern_list<'a>(ev: &Evaluator<'a>, node: &'a Value) -> &'a Vec<(String, regress::Regex)> {
    const EMPTY: &Vec<(String, regress::Regex)> = &Vec::new();
    ev.node_regexes(node)
        .map(|r| &r.pattern_properties)
        .unwrap_or(EMPTY)
}

// ------------------------------------------------------------------
// NodeFrame
// ------------------------------------------------------------------

fn node_child<'a>(
    schema: &'a Value,
    value: ValueRef<'a>,
    ctx_push: bool,
    path: String,
) -> Frame<'a> {
    Frame::Node(NodeFrame {
        schema,
        value,
        path,
        stages: Vec::new(),
        stage_idx: 0,
        visit_key: None,
        ctx_pushed: false,
        ctx_push,
    })
}

fn eval_leaf(ev: &Evaluator<'_>, stage: &Stage<'_>, schema: &Value, value: &Value) -> bool {
    match stage {
        Stage::TypeLeaf => match guards::get(schema, "type") {
            Some(Value::String(name)) => type_matches(name, value),
            Some(Value::Array(names)) => names
                .iter()
                .filter_map(Value::as_str)
                .any(|name| type_matches(name, value)),
            _ => true,
        },
        Stage::RequiredLeaf => guards::required_list(schema)
            .map(|required| {
                required
                    .iter()
                    .filter_map(Value::as_str)
                    .all(|key| value.as_object().is_some_and(|map| map.contains_key(key)))
            })
            .unwrap_or(true),
        Stage::DependentRequiredLeaf => {
            let Some(map) = value.as_object() else {
                return true;
            };
            let Some(deps) = guards::get(schema, "dependentRequired").and_then(Value::as_object)
            else {
                return true;
            };
            if map.is_empty() {
                return true;
            }
            js_keys(deps).into_iter().all(|key| {
                !map.contains_key(key)
                    || deps[key]
                        .as_array()
                        .map(|needed| {
                            needed
                                .iter()
                                .filter_map(Value::as_str)
                                .all(|needed| map.contains_key(needed))
                        })
                        .unwrap_or(true)
            })
        }
        Stage::MinPropertiesLeaf => {
            let count = value.as_object().map(|map| js_keys(map).len()).unwrap_or(0);
            (count as f64) >= guards::get_f64(schema, "minProperties").unwrap_or(f64::NEG_INFINITY)
        }
        Stage::MaxPropertiesLeaf => {
            let count = value.as_object().map(|map| js_keys(map).len()).unwrap_or(0);
            (count as f64) <= guards::get_f64(schema, "maxProperties").unwrap_or(f64::INFINITY)
        }
        Stage::MinItemsLeaf => {
            let count = value.as_array().map_or(0, Vec::len);
            (count as f64) >= guards::get_f64(schema, "minItems").unwrap_or(f64::NEG_INFINITY)
        }
        Stage::MaxItemsLeaf => {
            let count = value.as_array().map_or(0, Vec::len);
            (count as f64) <= guards::get_f64(schema, "maxItems").unwrap_or(f64::INFINITY)
        }
        Stage::UniqueItemsLeaf => match guards::get(schema, "uniqueItems") {
            Some(Value::Bool(false)) => true,
            _ => value
                .as_array()
                .is_none_or(|items| hashing::unique_items(items)),
        },
        Stage::MaxLengthLeaf => {
            let Some(text) = value.as_str() else {
                return true;
            };
            let limit = guards::get_f64(schema, "maxLength").unwrap_or(f64::INFINITY);
            grapheme::is_max_length(text, limit)
        }
        Stage::MinLengthLeaf => {
            let Some(text) = value.as_str() else {
                return true;
            };
            let limit = guards::get_f64(schema, "minLength").unwrap_or(f64::NEG_INFINITY);
            grapheme::is_min_length(text, limit)
        }
        Stage::FormatLeaf => {
            let Some(name) = guards::get_str(schema, "format") else {
                return true;
            };
            formats::format_test(name, value)
        }
        Stage::PatternLeaf => {
            let Some(text) = value.as_str() else {
                return true;
            };
            ev.node_regexes(schema)
                .and_then(|regexes| regexes.pattern.as_ref())
                .map(|regex| regex.find(text).is_some())
                .unwrap_or(true)
        }
        Stage::ExclusiveMaximumLeaf => {
            let n = value.as_f64().unwrap_or(f64::NAN);
            let limit = guards::get_f64(schema, "exclusiveMaximum").unwrap_or(f64::NAN);
            n < limit
        }
        Stage::ExclusiveMinimumLeaf => {
            let n = value.as_f64().unwrap_or(f64::NAN);
            let limit = guards::get_f64(schema, "exclusiveMinimum").unwrap_or(f64::NAN);
            n > limit
        }
        Stage::MaximumLeaf => {
            let n = value.as_f64().unwrap_or(f64::NAN);
            let limit = guards::get_f64(schema, "maximum").unwrap_or(f64::NAN);
            n <= limit
        }
        Stage::MinimumLeaf => {
            let n = value.as_f64().unwrap_or(f64::NAN);
            let limit = guards::get_f64(schema, "minimum").unwrap_or(f64::NAN);
            n >= limit
        }
        Stage::MultipleOfLeaf => {
            let n = value.as_f64().unwrap_or(f64::NAN);
            let divisor = guards::get_f64(schema, "multipleOf").unwrap_or(f64::NAN);
            is_multiple_of(n, divisor)
        }
        Stage::ConstLeaf => {
            let Some(expected) = guards::get(schema, "const") else {
                return true;
            };
            js_deep_equal(value, expected)
        }
        Stage::EnumLeaf => {
            let Some(options) = guards::get(schema, "enum").and_then(Value::as_array) else {
                return true;
            };
            options.iter().any(|option| js_deep_equal(value, option))
        }
        _ => true,
    }
}

/// Redacted `(keyword, reason)` for a failing leaf stage. Reasons carry
/// schema-derived bounds only — never instance content.
fn leaf_failure(stage: &Stage<'_>, schema: &Value) -> (&'static str, String) {
    match stage {
        Stage::TypeLeaf => {
            let expected = match guards::get(schema, "type") {
                Some(Value::String(name)) => name.clone(),
                Some(Value::Array(names)) => names
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => String::new(),
            };
            ("type", format!("instance is not of type {expected}"))
        }
        Stage::RequiredLeaf => ("required", "required property is missing".to_owned()),
        Stage::DependentRequiredLeaf => (
            "dependentRequired",
            "required dependent property is missing".to_owned(),
        ),
        Stage::MinPropertiesLeaf => ("minProperties", "object has too few properties".to_owned()),
        Stage::MaxPropertiesLeaf => ("maxProperties", "object has too many properties".to_owned()),
        Stage::MinItemsLeaf => ("minItems", "array has too few items".to_owned()),
        Stage::MaxItemsLeaf => ("maxItems", "array has too many items".to_owned()),
        Stage::UniqueItemsLeaf => ("uniqueItems", "array items are not unique".to_owned()),
        Stage::MaxLengthLeaf => ("maxLength", "string is too long".to_owned()),
        Stage::MinLengthLeaf => ("minLength", "string is too short".to_owned()),
        Stage::FormatLeaf => {
            let name = guards::get_str(schema, "format").unwrap_or_default();
            ("format", format!("string does not match format `{name}`"))
        }
        Stage::PatternLeaf => (
            "pattern",
            "string does not match the schema pattern".to_owned(),
        ),
        Stage::ExclusiveMaximumLeaf => (
            "exclusiveMaximum",
            "number is not below the bound".to_owned(),
        ),
        Stage::ExclusiveMinimumLeaf => (
            "exclusiveMinimum",
            "number is not above the bound".to_owned(),
        ),
        Stage::MaximumLeaf => ("maximum", "number exceeds the bound".to_owned()),
        Stage::MinimumLeaf => ("minimum", "number is below the bound".to_owned()),
        Stage::MultipleOfLeaf => (
            "multipleOf",
            "number is not a multiple of the divisor".to_owned(),
        ),
        Stage::ConstLeaf => ("const", "value does not equal the schema const".to_owned()),
        Stage::EnumLeaf => ("enum", "value does not equal any enum option".to_owned()),
        _ => ("schema", "value failed the schema".to_owned()),
    }
}

impl<'a> NodeFrame<'a> {
    /// The node's instance value when it is a borrowed reference. Container
    /// stages (arrays/objects) only exist for borrowed values; synthesized
    /// values are the propertyNames key strings.
    fn borrowed_value(&self) -> Result<&'a Value, SchemaError> {
        match &self.value {
            Cow::Borrowed(value) => Ok(value),
            Cow::Owned(_) => Err(SchemaError::Internal(
                "container stage reached with a synthesized value",
            )),
        }
    }

    fn enter(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        if let Value::Bool(result) = self.schema {
            return Ok(Step::Done(*result));
        }
        ev.stack.push(self.schema);
        let key = ev.visit_key(self.schema, &self.value);
        ev.enter_visit(key)?;
        self.visit_key = Some(key);
        if self.ctx_push && ev.reducer {
            ev.ctx.push(CtxFrame::default());
            self.ctx_pushed = true;
        }
        self.stages = build_stages(ev, self.schema, as_value(&self.value));
        self.advance(ev)
    }

    fn finish(&mut self, ev: &mut Evaluator<'a>, valid: bool) -> Step<'a> {
        if self.ctx_pushed {
            ev.ctx.pop();
        }
        if let Some(key) = self.visit_key.take() {
            ev.exit_visit(key);
        }
        ev.stack.pop(self.schema);
        Step::Done(valid)
    }

    fn advance(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.stage_idx < self.stages.len() {
            let stage = self.stages[self.stage_idx].clone();
            match &stage {
                Stage::AdditionalProperties { node, schema } => {
                    // Upstream Compile-path optimizations:
                    // 1. fast exact-shape check when additionalProperties is
                    //    false, properties/required lengths agree and no
                    //    patternProperties (BuildAdditionalPropertiesFast:
                    //    key count must equal required length).
                    // 2. true-like additionalProperties ignored outside
                    //    reducer mode.
                    if can_additional_properties_fast(node) {
                        let required_len = guards::required_list(node).map_or(0, Vec::len);
                        let count = self
                            .value
                            .as_ref()
                            .as_object()
                            .map(|map| js_keys(map).len())
                            .unwrap_or(0);
                        if count != required_len {
                            ev.note_failure(
                                &self.path,
                                "additionalProperties",
                                "object has properties not allowed by the schema".to_owned(),
                            );
                            return Ok(self.finish(ev, false));
                        }
                    } else if !ev.reducer && additional_properties_ignored(schema) {
                        // ignored
                    } else {
                        let value = self.borrowed_value()?;
                        let Some(map) = value.as_object() else {
                            return Err(SchemaError::Internal(
                                "additionalProperties stage without object",
                            ));
                        };
                        return Ok(Step::Child(Frame::AdditionalProps(AdditionalPropsFrame {
                            base_path: self.path.clone(),
                            node,
                            schema,
                            value_map: map,
                            keys: js_keys(map).into_iter().map(str::to_string).collect(),
                            idx: 0,
                        })));
                    }
                }
                Stage::Dependencies {
                    entries,
                    empty_shortcut,
                } => {
                    let value = self.borrowed_value()?;
                    let is_empty = value.as_object().is_some_and(serde_json::Map::is_empty);
                    if *empty_shortcut && is_empty {
                        // shortcut: empty object passes
                    } else {
                        return Ok(Step::Child(Frame::Deps(DepsFrame {
                            base_path: self.path.clone(),
                            entries: entries.clone(),
                            value: self.value.clone(),
                            idx: 0,
                        })));
                    }
                }
                Stage::PatternProperties { node, patterns } => {
                    let node_value = self.borrowed_value()?;
                    let Some(value_map) = node_value.as_object() else {
                        return Err(SchemaError::Internal(
                            "patternProperties stage without object",
                        ));
                    };
                    let Some(pattern_map) =
                        guards::get(node, "patternProperties").and_then(Value::as_object)
                    else {
                        return Err(SchemaError::Internal("patternProperties map missing"));
                    };
                    return Ok(Step::Child(Frame::PatternProperties(
                        PatternPropertiesFrame {
                            base_path: self.path.clone(),
                            patterns,
                            pattern_map,
                            value_map,
                            pattern_idx: 0,
                            key_idx: 0,
                            keys: js_keys(value_map).into_iter().map(str::to_string).collect(),
                        },
                    )));
                }
                Stage::Properties { entries } => {
                    let value = self.borrowed_value()?;
                    let Some(map) = value.as_object() else {
                        return Err(SchemaError::Internal("properties stage without object"));
                    };
                    return Ok(Step::Child(Frame::Properties(PropertiesFrame {
                        base_path: self.path.clone(),
                        entries: entries.clone(),
                        map,
                        idx: 0,
                    })));
                }
                Stage::PropertyNames { schema, keys } => {
                    return Ok(Step::Child(Frame::PropertyNames(PropertyNamesFrame {
                        base_path: self.path.clone(),
                        schema,
                        keys: keys.clone(),
                        idx: 0,
                    })));
                }
                Stage::AdditionalItems { schema, items_len } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal("additionalItems stage without array"));
                    };
                    return Ok(Step::Child(Frame::AdditionalItems(AdditionalItemsFrame {
                        base_path: self.path.clone(),
                        schema,
                        array,
                        items_len: *items_len,
                        idx: 0,
                    })));
                }
                Stage::Contains {
                    schema,
                    min_contains_zero,
                } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal("contains stage without array"));
                    };
                    if *min_contains_zero {
                        // inert: minContains === 0 disables contains
                    } else {
                        return Ok(Step::Child(Frame::Contains(ContainsFrame {
                            base_path: self.path.clone(),
                            schema,
                            array,
                            idx: 0,
                            matched: false,
                            record_indices: ev.reducer,
                        })));
                    }
                }
                Stage::Items { node } => {
                    let node_value = self.borrowed_value()?;
                    let Some(array) = node_value.as_array() else {
                        return Err(SchemaError::Internal("items stage without array"));
                    };
                    if guards::is_items_sized(node) {
                        let items = guards::get(node, "items")
                            .and_then(Value::as_array)
                            .unwrap();
                        return Ok(Step::Child(Frame::ItemsSized(ItemsSizedFrame {
                            base_path: self.path.clone(),
                            items,
                            array,
                            idx: 0,
                        })));
                    }
                    let offset = if guards::is_prefix_items(node) {
                        guards::get(node, "prefixItems")
                            .and_then(Value::as_array)
                            .map_or(0, Vec::len)
                    } else {
                        0
                    };
                    let schema = guards::get(node, "items").unwrap();
                    return Ok(Step::Child(Frame::ItemsUnsized(ItemsUnsizedFrame {
                        base_path: self.path.clone(),
                        schema,
                        array,
                        idx: offset,
                    })));
                }
                Stage::MinContains { schema, limit } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal("minContains stage without array"));
                    };
                    return Ok(Step::Child(Frame::MinContains(MinContainsFrame {
                        base_path: self.path.clone(),
                        schema,
                        array,
                        idx: 0,
                        limit: *limit,
                        count: 0,
                        record_indices: ev.reducer,
                    })));
                }
                Stage::MaxContains { schema, limit } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal("maxContains stage without array"));
                    };
                    return Ok(Step::Child(Frame::MaxContains(MaxContainsFrame {
                        base_path: self.path.clone(),
                        schema,
                        array,
                        idx: 0,
                        limit: *limit,
                        count: 0,
                    })));
                }
                Stage::PrefixItems { items } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal("prefixItems stage without array"));
                    };
                    if !array.is_empty() {
                        return Ok(Step::Child(Frame::PrefixItems(PrefixItemsFrame {
                            base_path: self.path.clone(),
                            items,
                            array,
                            idx: 0,
                        })));
                    }
                }
                Stage::IfThenElse { schema } => {
                    return Ok(Step::Child(Frame::IfThenElse(IfThenElseFrame {
                        base_path: self.path.clone(),
                        schema,
                        value: self.value.clone(),
                        phase: IfPhase::Condition,
                    })));
                }
                Stage::ResolveRef => {
                    let reference = guards::get_str(self.schema, "$ref").unwrap_or_default();
                    let target = ev.stack.apply_ref(reference)?;
                    match target {
                        None => {
                            ev.note_failure(&self.path, "$ref", "unresolved reference".to_owned());
                            return Ok(self.finish(ev, false));
                        }
                        Some(target) => {
                            if ev.reducer {
                                return Ok(Step::Child(Frame::Ref(RefFrame {
                                    base_path: self.path.clone(),
                                    target,
                                    value: self.value.clone(),
                                    saved_ctx: Vec::new(),
                                })));
                            }
                            return Ok(Step::Child(node_child(
                                target,
                                self.value.clone(),
                                false,
                                self.path.clone(),
                            )));
                        }
                    }
                }
                Stage::ResolveRecursiveRef => {
                    let reference =
                        guards::get_str(self.schema, "$recursiveRef").unwrap_or_default();
                    let target = ev.stack.apply_recursive_ref(reference)?;
                    match target {
                        None => {
                            ev.note_failure(
                                &self.path,
                                "$recursiveRef",
                                "unresolved reference".to_owned(),
                            );
                            return Ok(self.finish(ev, false));
                        }
                        Some(target) => {
                            return Ok(Step::Child(node_child(
                                target,
                                self.value.clone(),
                                false,
                                self.path.clone(),
                            )));
                        }
                    }
                }
                Stage::ResolveDynamicRef => {
                    let reference = guards::get_str(self.schema, "$dynamicRef").unwrap_or_default();
                    let target = ev.stack.apply_dynamic_ref(reference)?;
                    match target {
                        None => {
                            ev.note_failure(
                                &self.path,
                                "$dynamicRef",
                                "unresolved reference".to_owned(),
                            );
                            return Ok(self.finish(ev, false));
                        }
                        Some(target) => {
                            return Ok(Step::Child(node_child(
                                target,
                                self.value.clone(),
                                false,
                                self.path.clone(),
                            )));
                        }
                    }
                }
                Stage::Not => {
                    let inner = guards::get(self.schema, "not").unwrap();
                    return Ok(Step::Child(Frame::Arms(ArmsFrame {
                        base_path: self.path.clone(),
                        kind: ArmKind::AllOf,
                        arms: std::slice::from_ref(inner),
                        value: self.value.clone(),
                        arm_idx: 0,
                        passed: 0,
                        passed_facts: Vec::new(),
                        saved_ctx: None,
                        not_mode: true,
                    })));
                }
                Stage::Arms { kind, arms } => {
                    return Ok(Step::Child(Frame::Arms(ArmsFrame {
                        base_path: self.path.clone(),
                        kind: *kind,
                        arms,
                        value: self.value.clone(),
                        arm_idx: 0,
                        passed: 0,
                        passed_facts: Vec::new(),
                        saved_ctx: None,
                        not_mode: false,
                    })));
                }
                Stage::UnevaluatedItems { schema } => {
                    let value = self.borrowed_value()?;
                    let Some(array) = value.as_array() else {
                        return Err(SchemaError::Internal(
                            "unevaluatedItems stage without array",
                        ));
                    };
                    return Ok(Step::Child(Frame::UnevaluatedItems(
                        UnevaluatedItemsFrame {
                            base_path: self.path.clone(),
                            schema,
                            array,
                            idx: 0,
                        },
                    )));
                }
                Stage::UnevaluatedProperties { schema, keys } => {
                    let value = self.borrowed_value()?;
                    let Some(map) = value.as_object() else {
                        return Err(SchemaError::Internal(
                            "unevaluatedProperties stage without object",
                        ));
                    };
                    return Ok(Step::Child(Frame::UnevaluatedProps(
                        UnevaluatedPropsFrame {
                            base_path: self.path.clone(),
                            schema,
                            value_map: map,
                            keys: keys.clone(),
                            idx: 0,
                        },
                    )));
                }
                leaf => {
                    if !eval_leaf(ev, leaf, self.schema, as_value(&self.value)) {
                        let (keyword, reason) = leaf_failure(leaf, self.schema);
                        ev.note_failure(&self.path, keyword, reason);
                        return Ok(self.finish(ev, false));
                    }
                }
            }
            self.stage_idx += 1;
        }
        Ok(self.finish(ev, true))
    }
}

fn can_additional_properties_fast(node: &Value) -> bool {
    guards::is_required(node)
        && guards::is_properties(node)
        && !guards::is_pattern_properties(node)
        && guards::get(node, "additionalProperties") == Some(&Value::Bool(false))
        && guards::get(node, "properties")
            .and_then(Value::as_object)
            .map_or(0, serde_json::Map::len)
            == guards::required_list(node).map_or(0, Vec::len)
}

fn additional_properties_ignored(schema: &Value) -> bool {
    *schema == Value::Bool(true) || schema.as_object().is_some_and(|map| map.is_empty())
}

// ------------------------------------------------------------------
// Frame dispatch
// ------------------------------------------------------------------

impl<'a> Frame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match self {
            Frame::Node(frame) => match input {
                Input::Start => frame.enter(ev),
                Input::Child(result) => {
                    if !result {
                        return Ok(frame.finish(ev, false));
                    }
                    frame.stage_idx += 1;
                    frame.advance(ev)
                }
            },
            Frame::Arms(frame) => frame.step(ev, input),
            Frame::Ref(frame) => frame.step(ev, input),
            Frame::Contains(frame) => frame.step(ev, input),
            Frame::MinContains(frame) => frame.step(ev, input),
            Frame::MaxContains(frame) => frame.step(ev, input),
            Frame::ItemsSized(frame) => frame.step(ev, input),
            Frame::ItemsUnsized(frame) => frame.step(ev, input),
            Frame::AdditionalItems(frame) => frame.step(ev, input),
            Frame::PrefixItems(frame) => frame.step(ev, input),
            Frame::Properties(frame) => frame.step(ev, input),
            Frame::PatternProperties(frame) => frame.step(ev, input),
            Frame::AdditionalProps(frame) => frame.step(ev, input),
            Frame::PropertyNames(frame) => frame.step(ev, input),
            Frame::Deps(frame) => frame.step(ev, input),
            Frame::IfThenElse(frame) => frame.step(ev, input),
            Frame::UnevaluatedItems(frame) => frame.step(ev, input),
            Frame::UnevaluatedProps(frame) => frame.step(ev, input),
        }
    }
}

impl<'a> ArmsFrame<'a> {
    fn begin_next_arm(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        if self.arm_idx < self.arms.len() {
            let arm = &self.arms[self.arm_idx];
            if ev.reducer && self.saved_ctx.is_none() {
                self.saved_ctx = Some(std::mem::take(&mut ev.ctx));
            }
            if ev.reducer {
                ev.ctx = vec![CtxFrame::default()];
            }
            return Ok(Step::Child(node_child(
                arm,
                self.value.clone(),
                false,
                self.base_path.clone(),
            )));
        }
        self.finish(ev)
    }

    fn finish(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        if self.not_mode {
            // Reducer: arm passing -> false without merge; arm failing ->
            // true with an empty merge (upstream `check && Merge(results)`).
            if let Some(saved) = self.saved_ctx.take() {
                ev.ctx = saved;
            }
            return Ok(Step::Done(self.passed == 0));
        }
        let check = match self.kind {
            ArmKind::AllOf => self.passed == self.arms.len(),
            ArmKind::AnyOf => self.passed > 0,
            ArmKind::OneOf => self.passed == 1,
        };
        if !check {
            let (keyword, reason) = if self.not_mode {
                ("not", "instance matches the negated schema".to_owned())
            } else {
                match self.kind {
                    ArmKind::AllOf => (
                        "allOf",
                        "instance failed at least one allOf branch".to_owned(),
                    ),
                    ArmKind::AnyOf => ("anyOf", "instance matched no anyOf branch".to_owned()),
                    ArmKind::OneOf => (
                        "oneOf",
                        "instance did not match exactly one oneOf branch".to_owned(),
                    ),
                }
            };
            ev.note_failure(&self.base_path, keyword, reason);
        }
        if ev.reducer {
            let Some(mut saved) = self.saved_ctx.take() else {
                return Ok(Step::Done(check));
            };
            if check && let Some(top) = saved.last_mut() {
                for facts in &self.passed_facts {
                    top.indices.extend(facts.indices.iter().copied());
                    top.keys.extend(facts.keys.iter().cloned());
                }
            }
            ev.ctx = saved;
        }
        Ok(Step::Done(check))
    }

    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.begin_next_arm(ev),
            Input::Child(result) => {
                if ev.reducer {
                    let arm_ctx = std::mem::take(&mut ev.ctx);
                    if result {
                        self.passed += 1;
                        if let Some(top) = arm_ctx.last() {
                            self.passed_facts.push(top.clone());
                        }
                    }
                } else if result {
                    self.passed += 1;
                }
                self.arm_idx += 1;
                // fast-mode short-circuits
                if !ev.reducer {
                    if self.not_mode {
                        return Ok(Step::Done(!result));
                    }
                    match self.kind {
                        ArmKind::AllOf if !result => return Ok(Step::Done(false)),
                        ArmKind::AnyOf if result => return Ok(Step::Done(true)),
                        _ => {}
                    }
                }
                self.begin_next_arm(ev)
            }
        }
    }
}

impl<'a> RefFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                self.saved_ctx = std::mem::take(&mut ev.ctx);
                ev.ctx = vec![CtxFrame::default()];
                Ok(Step::Child(node_child(
                    self.target,
                    self.value.clone(),
                    false,
                    self.base_path.clone(),
                )))
            }
            Input::Child(result) => {
                let ref_ctx = std::mem::take(&mut ev.ctx);
                ev.ctx = std::mem::take(&mut self.saved_ctx);
                if result && let (Some(top), Some(facts)) = (ev.ctx.last_mut(), ref_ctx.last()) {
                    top.indices.extend(facts.indices.iter().copied());
                    top.keys.extend(facts.keys.iter().cloned());
                }
                Ok(Step::Done(result))
            }
        }
    }
}

impl<'a> ContainsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.array.is_empty() {
                    ev.note_failure(
                        &self.base_path,
                        "contains",
                        "array has no element matching the contains schema".to_owned(),
                    );
                    return Ok(Step::Done(false));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if result {
                    self.matched = true;
                    if self.record_indices {
                        ev.add_index(self.idx as u64);
                    }
                    if !ev.reducer {
                        // fast mode: Some short-circuits on first success
                        return Ok(Step::Done(true));
                    }
                }
                self.idx += 1;
                if self.idx >= self.array.len() {
                    if !self.matched {
                        ev.note_failure(
                            &self.base_path,
                            "contains",
                            "array has no element matching the contains schema".to_owned(),
                        );
                    }
                    return Ok(Step::Done(self.matched));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> MinContainsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.array.is_empty() {
                    let ok = (self.count as f64) >= self.limit;
                    if !ok {
                        ev.note_failure(
                            &self.base_path,
                            "minContains",
                            format!("fewer matching elements than minContains {}", self.limit),
                        );
                    }
                    return Ok(Step::Done(ok));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if result {
                    self.count += 1;
                    if self.record_indices {
                        ev.add_index(self.idx as u64);
                    }
                }
                self.idx += 1;
                if self.idx >= self.array.len() {
                    let ok = (self.count as f64) >= self.limit;
                    if !ok {
                        ev.note_failure(
                            &self.base_path,
                            "minContains",
                            format!("fewer matching elements than minContains {}", self.limit),
                        );
                    }
                    return Ok(Step::Done(ok));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> MaxContainsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.array.is_empty() {
                    return Ok(Step::Done((self.count as f64) <= self.limit));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if result {
                    self.count += 1;
                }
                self.idx += 1;
                if self.idx >= self.array.len() {
                    let ok = (self.count as f64) <= self.limit;
                    if !ok {
                        ev.note_failure(
                            &self.base_path,
                            "maxContains",
                            format!("more matching elements than maxContains {}", self.limit),
                        );
                    }
                    return Ok(Step::Done(ok));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    false,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> ItemsSizedFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.idx >= self.items.len() || self.array.len() <= self.idx {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    &self.items[self.idx],
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                ev.add_index(self.idx as u64);
                self.idx += 1;
                if self.idx >= self.items.len() || self.array.len() <= self.idx {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    &self.items[self.idx],
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> ItemsUnsizedFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.idx >= self.array.len() {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                ev.add_index(self.idx as u64);
                self.idx += 1;
                if self.idx >= self.array.len() {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    self.schema,
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> AdditionalItemsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                ev.add_index(self.idx as u64);
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, _ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        // Items covered by the tuple form were validated by the items stage.
        self.idx = self.idx.max(self.items_len);
        if self.idx >= self.array.len() {
            return Ok(Step::Done(true));
        }
        Ok(Step::Child(node_child(
            self.schema,
            Cow::Borrowed(&self.array[self.idx]),
            true,
            index_pointer(&self.base_path, self.idx),
        )))
    }
}

impl<'a> PrefixItemsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => {
                if self.idx >= self.items.len() || self.array.len() <= self.idx {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    &self.items[self.idx],
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                ev.add_index(self.idx as u64);
                self.idx += 1;
                if self.array.len() <= self.idx || self.idx >= self.items.len() {
                    return Ok(Step::Done(true));
                }
                Ok(Step::Child(node_child(
                    &self.items[self.idx],
                    Cow::Borrowed(&self.array[self.idx]),
                    true,
                    index_pointer(&self.base_path, self.idx),
                )))
            }
        }
    }
}

impl<'a> PropertiesFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                let (key, _) = self.entries[self.idx];
                ev.add_key(key);
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, _ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.idx < self.entries.len() {
            let (key, child) = self.entries[self.idx];
            if !self.map.contains_key(key) {
                self.idx += 1;
                continue;
            }
            let value = &self.map[key];
            let path = child_pointer(&self.base_path, key);
            return Ok(Step::Child(node_child(
                child,
                Cow::Borrowed(value),
                true,
                path,
            )));
        }
        Ok(Step::Done(true))
    }
}

impl<'a> PatternPropertiesFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                let key = self.keys[self.key_idx].as_str();
                ev.add_key(key);
                self.key_idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, _ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.pattern_idx < self.patterns.len() {
            let (pattern, regex) = &self.patterns[self.pattern_idx];
            while self.key_idx < self.keys.len() {
                let key = self.keys[self.key_idx].as_str();
                if regex.find(key).is_none() {
                    self.key_idx += 1;
                    continue;
                }
                let child = self
                    .pattern_map
                    .get(pattern.as_str())
                    .expect("pattern key present in patternProperties");
                let value = &self.value_map[key];
                let path = child_pointer(&self.base_path, key);
                return Ok(Step::Child(node_child(
                    child,
                    Cow::Borrowed(value),
                    true,
                    path,
                )));
            }
            self.pattern_idx += 1;
            self.key_idx = 0;
        }
        Ok(Step::Done(true))
    }
}

impl<'a> AdditionalPropsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                let key = self.keys[self.idx].as_str();
                ev.add_key(key);
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.idx < self.keys.len() {
            let key = self.keys[self.idx].as_str();
            if covered_key(ev, self.node, key) {
                self.idx += 1;
                continue;
            }
            let value = &self.value_map[key];
            let path = child_pointer(&self.base_path, key);
            return Ok(Step::Child(node_child(
                self.schema,
                Cow::Borrowed(value),
                true,
                path,
            )));
        }
        Ok(Step::Done(true))
    }
}

impl<'a> PropertyNamesFrame<'a> {
    fn step(&mut self, _ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                self.idx += 1;
                self.advance()
            }
        }
    }

    fn advance(&mut self) -> Result<Step<'a>, SchemaError> {
        if self.idx < self.keys.len() {
            let key = self.keys[self.idx].clone();
            return Ok(Step::Child(node_child(
                self.schema,
                Cow::Owned(Value::String(key)),
                false,
                self.base_path.clone(),
            )));
        }
        Ok(Step::Done(true))
    }
}

impl<'a> DepsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        let Some(map) = as_value(&self.value).as_object() else {
            return Ok(Step::Done(true));
        };
        while self.idx < self.entries.len() {
            let (key, dep) = self.entries[self.idx];
            if !map.contains_key(key) {
                self.idx += 1;
                continue;
            }
            if let Some(needed) = dep.as_array() {
                // dependencies array form: every listed key must be present
                let ok = needed
                    .iter()
                    .filter_map(Value::as_str)
                    .all(|needed| map.contains_key(needed));
                if !ok {
                    ev.note_failure(
                        &self.base_path,
                        "dependencies",
                        "required dependency property is missing".to_owned(),
                    );
                    return Ok(Step::Done(false));
                }
                self.idx += 1;
                continue;
            }
            // Schema form validates the whole instance object.
            return Ok(Step::Child(node_child(
                dep,
                self.value.clone(),
                false,
                self.base_path.clone(),
            )));
        }
        Ok(Step::Done(true))
    }
}

impl<'a> IfThenElseFrame<'a> {
    fn step(&mut self, _ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match (&self.phase, input) {
            (IfPhase::Condition, Input::Start) => {
                let condition = guards::get(self.schema, "if").unwrap();
                Ok(Step::Child(node_child(
                    condition,
                    self.value.clone(),
                    false,
                    self.base_path.clone(),
                )))
            }
            (IfPhase::Condition, Input::Child(result)) => {
                let branch: &'a Value = if result {
                    if guards::is_then(self.schema) {
                        guards::get(self.schema, "then").unwrap()
                    } else {
                        &TRUE_SCHEMA
                    }
                } else if guards::is_else(self.schema) {
                    guards::get(self.schema, "else").unwrap()
                } else {
                    &TRUE_SCHEMA
                };
                self.phase = IfPhase::Branch;
                Ok(Step::Child(node_child(
                    branch,
                    self.value.clone(),
                    false,
                    self.base_path.clone(),
                )))
            }
            (IfPhase::Branch, Input::Child(result)) => Ok(Step::Done(result)),
            (IfPhase::Branch, Input::Start) => {
                unreachable!("branch phase always begins with a child result")
            }
        }
    }
}

static TRUE_SCHEMA: Value = Value::Bool(true);

impl<'a> UnevaluatedItemsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                ev.add_index(self.idx as u64);
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.idx < self.array.len() {
            if ev
                .top_indices()
                .is_some_and(|indices| indices.contains(&(self.idx as u64)))
            {
                ev.add_index(self.idx as u64);
                self.idx += 1;
                continue;
            }
            return Ok(Step::Child(node_child(
                self.schema,
                Cow::Borrowed(&self.array[self.idx]),
                false,
                index_pointer(&self.base_path, self.idx),
            )));
        }
        Ok(Step::Done(true))
    }
}

impl<'a> UnevaluatedPropsFrame<'a> {
    fn step(&mut self, ev: &mut Evaluator<'a>, input: Input) -> Result<Step<'a>, SchemaError> {
        match input {
            Input::Start => self.advance(ev),
            Input::Child(result) => {
                if !result {
                    return Ok(Step::Done(false));
                }
                let key = self.keys[self.idx].as_str();
                ev.add_key(key);
                self.idx += 1;
                self.advance(ev)
            }
        }
    }

    fn advance(&mut self, ev: &mut Evaluator<'a>) -> Result<Step<'a>, SchemaError> {
        while self.idx < self.keys.len() {
            let key = self.keys[self.idx].as_str();
            if ev.top_keys().is_some_and(|keys| keys.contains(key)) {
                ev.add_key(key);
                self.idx += 1;
                continue;
            }
            let value = &self.value_map[key];
            let path = child_pointer(&self.base_path, key);
            return Ok(Step::Child(node_child(
                self.schema,
                Cow::Borrowed(value),
                false,
                path,
            )));
        }
        Ok(Step::Done(true))
    }
}
