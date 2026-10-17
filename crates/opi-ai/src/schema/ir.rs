//! Admission of a trusted JSON schema into the dialect IR.
//!
//! The schema stays a `serde_json::Value` (insertion-ordered via the
//! `preserve_order` feature); admission compiles every structurally-selected
//! regex leaf (`pattern` strings and `patternProperties` keys, walked the way
//! reference resolution walks: every object/array except `const`/`enum`
//! subtrees) and detects whether any `unevaluatedItems`/`unevaluatedProperties`
//! keyword selects the reducer evaluation mode. Invalid regexes anywhere in
//! that walk fail admission with the JSON-pointer path — a deliberately
//! fail-closed divergence from the upstream, which only throws when a leaf is
//! actually visited.

use std::collections::HashMap;

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

impl std::fmt::Debug for Compiled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiled")
            .field("reducer", &self.reducer)
            .field("regex_nodes", &self.regexes.len())
            .finish_non_exhaustive()
    }
}

/// The admission result. The schema is heap-pinned so node pointer
/// identities captured during the admission walk stay valid for the
/// evaluator's regex cache.
pub(crate) struct Compiled {
    pub(crate) schema: Box<Value>,
    pub(crate) reducer: bool,
    pub(crate) regexes: HashMap<*const Value, NodeRegexes>,
}

// SAFETY: the raw-pointer keys in `regexes` are pure identity tokens for
// nodes inside `schema`; they are only hashed and compared, never
// dereferenced, and `Compiled` owns the schema they point into. Moving the
// struct across threads does not create competing access paths.
unsafe impl Send for Compiled {}
unsafe impl Sync for Compiled {}

pub(crate) fn compile(schema: Value) -> Result<Compiled, SchemaError> {
    let mut compiled = Compiled {
        schema: Box::new(schema),
        reducer: false,
        regexes: HashMap::new(),
    };
    let Compiled {
        schema,
        reducer,
        regexes,
    } = &mut compiled;
    walk(schema, String::new(), regexes, reducer)?;
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

fn walk(
    root: &Value,
    root_path: String,
    regexes: &mut HashMap<*const Value, NodeRegexes>,
    reducer: &mut bool,
) -> Result<(), SchemaError> {
    // Iterative pre-order traversal: schema depth never touches the Rust
    // call stack (children are pushed in reverse so errors surface in the
    // same pre-order a recursive walk would report).
    let mut stack: Vec<(&Value, String)> = vec![(root, root_path)];
    while let Some((value, path)) = stack.pop() {
        let Value::Object(map) = value else {
            if let Value::Array(items) = value {
                for (index, item) in items.iter().enumerate() {
                    stack.push((item, format!("{path}/{index}")));
                }
            }
            continue;
        };
        let mut node = NodeRegexes {
            pattern: None,
            pattern_properties: Vec::new(),
        };
        if guards::is_pattern(value) {
            let pattern = guards::get_str(value, "pattern").unwrap();
            let regex = compile_regex(pattern, &format!("{path}/pattern"))?;
            node.pattern = Some(regex);
        }
        if guards::is_pattern_properties(value) {
            let patterns = guards::get(value, "patternProperties")
                .and_then(Value::as_object)
                .unwrap();
            for key in js_keys(patterns) {
                let regex = compile_regex(
                    key,
                    &format!("{}/patternProperties/{}", path, escape_token(key)),
                )?;
                node.pattern_properties.push((key.to_string(), regex));
            }
        }
        if node.pattern.is_some() || !node.pattern_properties.is_empty() {
            regexes.insert(value as *const Value, node);
        }
        if guards::is_unevaluated_items(value) || guards::is_unevaluated_properties(value) {
            *reducer = true;
        }
        for key in js_keys(map).into_iter().rev() {
            if key == "const" || key == "enum" {
                continue;
            }
            stack.push((&map[key], format!("{}/{}", path, escape_token(key))));
        }
    }
    Ok(())
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
        // Deliberate fail-closed divergence from upstream (documented in the
        // module docs): structurally-selected regexes compile at admission.
        let error =
            compile(json!({"anyOf": [{"const": 1}, {"pattern": "\\u{110000}"}]})).unwrap_err();
        assert!(matches!(error, SchemaError::Regex { .. }));
    }

    #[test]
    fn regexes_inside_const_and_enum_are_not_selected() {
        let compiled = compile_ok(json!({"const": {"pattern": "[z-a]"}}));
        assert_eq!(compiled.regexes.len(), 0);
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
        let node = compiled.regexes.values().next().unwrap();
        let keys: Vec<&str> = node
            .pattern_properties
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, vec!["^b", "^a"]);
    }
}
