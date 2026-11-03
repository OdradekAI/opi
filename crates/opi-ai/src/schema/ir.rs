//! Admission of a trusted JSON schema into the dialect IR.
//!
//! The schema stays a `serde_json::Value` (insertion-ordered via the
//! `preserve_order` feature); admission compiles every structurally-selected
//! regex leaf selected by a source structural guard. Reference discovery may
//! inspect annotation objects, but that does not turn their contents into
//! schemas. Regex slots compile only when their node is admitted or resolved
//! as a schema; the generation owns both successful and failed compilation.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

use super::SchemaError;
use super::guards;
use super::js::js_keys;

/// Compiled regex leaves for one schema node, looked up during evaluation by
/// node identity.
pub(crate) struct NodeRegexes {
    pub(crate) pattern: Option<regress::Regex>,
    pub(crate) pattern_properties: Vec<(String, regress::Regex)>,
}

pub(crate) struct RegexSlot {
    path: String,
    compiled: OnceLock<Result<NodeRegexes, SchemaError>>,
}

impl RegexSlot {
    pub(crate) fn get(&self) -> Option<&NodeRegexes> {
        self.compiled.get().and_then(|result| result.as_ref().ok())
    }

    pub(crate) fn admit(&self, schema: &Value) -> Result<(), SchemaError> {
        self.compiled
            .get_or_init(|| compile_node(schema, &self.path))
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone)
    }
}

impl std::fmt::Debug for Compiled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiled")
            .field("reducer", &self.reducer)
            .field("regex_nodes", &self.regexes.len())
            .field("plan_nodes", &self.plan_nodes.len())
            .finish_non_exhaustive()
    }
}

pub(crate) type PlanId = usize;

/// One source build invocation. A schema node can have several plans when it
/// is built in different lexical/resource frames.
#[derive(Debug, Clone)]
pub(crate) struct PlanNode {
    pub(crate) schema_location: usize,
    pub(crate) children: Vec<PlanChild>,
    /// `$ref`, `$recursiveRef`, then `$dynamicRef`.
    pub(crate) refs: [RefBinding; 3],
}

/// A structurally selected child at its source keyword's build position.
#[derive(Debug, Clone)]
pub(crate) struct PlanChild {
    pub(crate) keyword: &'static str,
    pub(crate) schema: *const Value,
    pub(crate) plan: PlanId,
}

/// Reference resolution frozen at source build time. URI failures remain
/// deferred until the corresponding reference is checked.
#[derive(Debug, Clone)]
pub(crate) enum RefBinding {
    Unresolved,
    Target(PlanId),
    Error(SchemaError),
}

enum SchemaToken {
    Root,
    Key(String),
    Index(usize),
}

/// Flat parent links keep both lookup and destruction off the Rust call stack.
struct SchemaPath {
    parent: Option<usize>,
    token: SchemaToken,
}

/// The admission result. The schema is heap-pinned so node pointer
/// identities captured during the admission walk stay valid for the
/// evaluator's regex cache.
pub(crate) struct Compiled {
    pub(crate) schema: Box<Value>,
    pub(crate) reducer: bool,
    pub(crate) regexes: HashMap<*const Value, RegexSlot>,
    pub(crate) root_plan: PlanId,
    pub(crate) plan_nodes: Vec<PlanNode>,
    schema_paths: Vec<SchemaPath>,
}

// SAFETY: the raw-pointer keys in `regexes` and identities in `plan_nodes`
// refer to nodes inside `schema`; they are only hashed and compared, never
// dereferenced. Compiled owns the immutable schema, and plan lookup traverses
// safe Value references through the flat path arena. Moving the struct across
// threads does not create competing access paths.
unsafe impl Send for Compiled {}
unsafe impl Sync for Compiled {}

impl Drop for Compiled {
    fn drop(&mut self) {
        // Node identities remain pinned for the whole live generation. Detach
        // only during destruction, leaving an empty root for Box's own drop.
        super::drop_value(std::mem::take(self.schema.as_mut()));
    }
}

impl Compiled {
    pub(crate) fn schema_for_plan(&self, plan: PlanId) -> Result<&Value, SchemaError> {
        let location = self
            .plan_nodes
            .get(plan)
            .ok_or(SchemaError::Internal("schema plan does not exist"))?
            .schema_location;
        let mut tokens = Vec::new();
        let mut current = Some(location);
        while let Some(location) = current {
            let path = self
                .schema_paths
                .get(location)
                .ok_or(SchemaError::Internal("schema location does not exist"))?;
            tokens.push(&path.token);
            current = path.parent;
        }
        let mut schema = self.schema.as_ref();
        for token in tokens.into_iter().rev() {
            schema = match token {
                SchemaToken::Root => Some(schema),
                SchemaToken::Key(key) => schema.get(key.as_str()),
                SchemaToken::Index(index) => schema.get(*index),
            }
            .ok_or(SchemaError::Internal(
                "schema location does not match the document",
            ))?;
        }
        Ok(schema)
    }

    pub(crate) fn child_plan(
        &self,
        parent: PlanId,
        keyword: &str,
        schema: &Value,
    ) -> Result<PlanId, SchemaError> {
        self.plan_nodes
            .get(parent)
            .ok_or(SchemaError::Internal("parent schema plan does not exist"))?
            .children
            .iter()
            .find(|child| child.keyword == keyword && std::ptr::eq(child.schema, schema))
            .map(|child| child.plan)
            .ok_or(SchemaError::Internal(
                "structural child plan does not exist",
            ))
    }
}

pub(crate) fn compile(schema: Value) -> Result<Compiled, SchemaError> {
    let mut compiled = Compiled {
        schema: Box::new(schema),
        reducer: false,
        regexes: HashMap::new(),
        root_plan: 0,
        plan_nodes: Vec::new(),
        schema_paths: Vec::new(),
    };
    let Compiled {
        schema,
        reducer,
        regexes,
        plan_nodes,
        schema_paths,
        ..
    } = &mut compiled;
    let mut schema_locations = HashMap::new();
    walk(
        schema,
        regexes,
        reducer,
        &mut schema_locations,
        schema_paths,
    );
    *plan_nodes = super::admission::admit(schema, regexes, &schema_locations)?;
    Ok(compiled)
}

fn escape_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn compile_regex(pattern: &str, path: &str) -> Result<regress::Regex, SchemaError> {
    regress::Regex::with_flags(pattern, "u").map_err(|error| SchemaError::Regex {
        path: path.to_string(),
        message: error.to_string(),
    })
}

fn compile_node(value: &Value, path: &str) -> Result<NodeRegexes, SchemaError> {
    let mut node = NodeRegexes {
        pattern: None,
        pattern_properties: Vec::new(),
    };
    if guards::is_pattern(value) {
        node.pattern = Some(compile_regex(
            guards::get_str(value, "pattern").unwrap(),
            &format!("{path}/pattern"),
        )?);
    }
    if guards::is_pattern_properties(value) {
        let patterns = guards::get(value, "patternProperties")
            .and_then(Value::as_object)
            .unwrap();
        for key in js_keys(patterns) {
            node.pattern_properties.push((
                key.to_owned(),
                compile_regex(
                    key,
                    &format!("{path}/patternProperties/{}", escape_token(key)),
                )?,
            ));
        }
    }
    Ok(node)
}

/// Source-selected subschemas in `BuildSchema` order, split around references.
/// Visiting a child can change the pending resource consumed by a later sibling.
/// Maps and annotations are not themselves schemas.
pub(super) type SchemaChildren<'a> = Vec<(&'static str, &'a Value)>;

pub(super) fn schema_children(schema: &Value) -> (SchemaChildren<'_>, SchemaChildren<'_>) {
    let mut children = Vec::new();
    if guards::is_additional_properties(schema) {
        children.push((
            "additionalProperties",
            guards::get(schema, "additionalProperties").unwrap(),
        ));
    }
    for (keyword, selected) in [
        ("dependencies", guards::is_dependencies(schema)),
        ("dependentSchemas", guards::is_dependent_schemas(schema)),
        ("patternProperties", guards::is_pattern_properties(schema)),
        ("properties", guards::is_properties(schema)),
    ] {
        if selected && let Some(map) = guards::get(schema, keyword).and_then(Value::as_object) {
            children.extend(
                js_keys(map)
                    .into_iter()
                    .map(|key| &map[key])
                    .filter(|value| value.is_object() || value.is_boolean())
                    .map(|value| (keyword, value)),
            );
        }
    }
    for (keyword, selected) in [
        ("propertyNames", guards::is_property_names(schema)),
        (
            "additionalItems",
            guards::is_additional_items(schema)
                && guards::is_items(schema)
                && guards::is_items_sized(schema),
        ),
        (
            "contains",
            guards::is_contains(schema) && guards::get_f64(schema, "minContains") != Some(0.0),
        ),
    ] {
        if selected {
            children.push((keyword, guards::get(schema, keyword).unwrap()));
        }
    }
    if guards::is_items(schema) {
        let items = guards::get(schema, "items").unwrap();
        if let Some(items) = items.as_array() {
            children.extend(items.iter().map(|item| ("items", item)));
        } else {
            children.push(("items", items));
        }
    }
    // Each contains builder traverses its schema separately. Those visits can
    // establish different resource frames even though the node is identical.
    for (keyword, selected) in [
        ("maxContains", guards::is_max_contains(schema)),
        ("minContains", guards::is_min_contains(schema)),
    ] {
        if selected && guards::is_contains(schema) {
            children.push((keyword, guards::get(schema, "contains").unwrap()));
        }
    }
    if guards::is_prefix_items(schema) {
        children.extend(
            guards::get(schema, "prefixItems")
                .and_then(Value::as_array)
                .unwrap()
                .iter()
                .map(|item| ("prefixItems", item)),
        );
    }
    let before_references = children;
    let mut children = Vec::new();
    for (keyword, selected) in [
        ("if", guards::is_if(schema)),
        ("then", guards::is_if(schema) && guards::is_then(schema)),
        ("else", guards::is_if(schema) && guards::is_else(schema)),
        ("not", guards::is_not(schema)),
    ] {
        if selected {
            children.push((keyword, guards::get(schema, keyword).unwrap()));
        }
    }
    for (keyword, selected) in [
        ("allOf", guards::is_all_of(schema)),
        ("anyOf", guards::is_any_of(schema)),
        ("oneOf", guards::is_one_of(schema)),
    ] {
        if selected {
            let value = guards::get(schema, keyword).unwrap();
            if let Some(items) = value.as_array() {
                children.extend(items.iter().map(|item| (keyword, item)));
            } else {
                children.push((keyword, value));
            }
        }
    }
    for (keyword, selected) in [
        ("unevaluatedItems", guards::is_unevaluated_items(schema)),
        (
            "unevaluatedProperties",
            guards::is_unevaluated_properties(schema),
        ),
    ] {
        if selected {
            children.push((keyword, guards::get(schema, keyword).unwrap()));
        }
    }
    (before_references, children)
}

fn walk(
    root: &Value,
    regexes: &mut HashMap<*const Value, RegexSlot>,
    reducer: &mut bool,
    schema_locations: &mut HashMap<*const Value, usize>,
    schema_paths: &mut Vec<SchemaPath>,
) {
    // Every value receives a flat location. Discovery and later path lookup
    // use explicit stacks, so schema depth never consumes the Rust call stack.
    schema_paths.push(SchemaPath {
        parent: None,
        token: SchemaToken::Root,
    });
    let mut stack = vec![(root, 0)];
    while let Some((value, location)) = stack.pop() {
        schema_locations.insert(value as *const Value, location);
        let Value::Object(map) = value else {
            if let Value::Array(items) = value {
                for (index, item) in items.iter().enumerate().rev() {
                    let child_location = schema_paths.len();
                    schema_paths.push(SchemaPath {
                        parent: Some(location),
                        token: SchemaToken::Index(index),
                    });
                    stack.push((item, child_location));
                }
            }
            continue;
        };
        if guards::is_pattern(value) || guards::is_pattern_properties(value) {
            regexes.insert(
                value as *const Value,
                RegexSlot {
                    path: render_path(schema_paths, location),
                    compiled: OnceLock::new(),
                },
            );
        }
        // Source HasUnevaluated inspects every JSON object, including const /
        // enum data that a reference may select. This chooses tracking mode;
        // Admission separately selects which regexes are schemas.
        if guards::is_unevaluated_items(value) || guards::is_unevaluated_properties(value) {
            *reducer = true;
        }
        for key in js_keys(map).into_iter().rev() {
            let child_location = schema_paths.len();
            schema_paths.push(SchemaPath {
                parent: Some(location),
                token: SchemaToken::Key(key.to_owned()),
            });
            stack.push((&map[key], child_location));
        }
    }
}

fn render_path(schema_paths: &[SchemaPath], location: usize) -> String {
    let mut tokens = Vec::new();
    let mut current = Some(location);
    while let Some(location) = current {
        let path = &schema_paths[location];
        tokens.push(&path.token);
        current = path.parent;
    }
    let mut rendered = String::new();
    for token in tokens.into_iter().rev() {
        match token {
            SchemaToken::Root => {}
            SchemaToken::Key(key) => {
                rendered.push('/');
                rendered.push_str(&escape_token(key));
            }
            SchemaToken::Index(index) => {
                rendered.push('/');
                rendered.push_str(&index.to_string());
            }
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn compile_ok(schema: Value) -> Compiled {
        compile(schema).expect("compiles")
    }

    #[test]
    fn valid_patterns_compile_into_the_node_cache() {
        let compiled = compile_ok(json!({"type": "string", "pattern": "^a+$"}));
        assert!(!compiled.reducer);
        assert_eq!(compiled.regexes.len(), 1);
    }

    #[test]
    fn invalid_pattern_fails_admission_with_its_path() {
        let error = compile(json!({"properties": {"a": {"pattern": "[z-a]"}}})).unwrap_err();
        match error {
            SchemaError::Regex { path, .. } => assert_eq!(path, "/properties/a/pattern"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn invalid_pattern_in_an_unvisited_anyof_arm_still_fails_admission() {
        // The source compiler eagerly builds structurally selected arms,
        // including arms whose instance check would not be visited.
        let error =
            compile(json!({"anyOf": [{"const": 1}, {"pattern": "\\u{110000}"}]})).unwrap_err();
        assert!(matches!(error, SchemaError::Regex { .. }));
    }

    #[test]
    fn regexes_inside_const_and_enum_are_not_selected() {
        let compiled = compile_ok(json!({"const": {"pattern": "[z-a]"}}));
        assert!(
            compiled
                .regexes
                .values()
                .all(|slot| slot.compiled.get().is_none())
        );
    }

    #[test]
    fn unevaluated_keywords_select_reducer_mode() {
        let compiled = compile_ok(json!({"properties": {"a": {}}, "unevaluatedProperties": false}));
        assert!(compiled.reducer);
        let compiled = compile_ok(json!({"unevaluatedItems": {}}));
        assert!(compiled.reducer);
        let compiled = compile_ok(json!({"properties": {"a": {}}}));
        assert!(!compiled.reducer);
    }

    #[test]
    fn pattern_properties_keys_compile_in_schema_order() {
        let compiled = compile_ok(json!({"patternProperties": {"^b": {}, "^a": {}}}));
        let node = compiled.regexes.values().next().unwrap().get().unwrap();
        let keys: Vec<&str> = node
            .pattern_properties
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, vec!["^b", "^a"]);
    }
}
