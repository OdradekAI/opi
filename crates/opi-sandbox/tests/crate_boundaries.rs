//! Crate-boundary invariants for the standalone `opi-sandbox` SDK (Phase 16 task 16.11.1).
//!
//! The DoD requires the crate to depend on neither `opi-agent` nor
//! `opi-coding-agent` and to read no Opi configuration, sessions, or package
//! storage. The strong structural proof is `cargo tree -p opi-sandbox --edges
//! normal` (the resolve graph has no `opi-agent`/`opi-coding-agent` edge; the
//! sole opi-internal dep is the pure-types `opi-protocol`). The secondary guard
//! asserts the library source calls no host-environment-read API except explicit
//! execution/build inputs. That is a necessary condition for reading any
//! `OPI_*` configuration env var.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use proc_macro2::{TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMacro, ExprPath, ImplItem, ImplItemFn, Item, ItemExternCrate,
    ItemFn, ItemMod, ItemUse, Lit, Local, Macro, Meta, StmtMacro, Token, TraitItem, UseTree,
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn host_environment_reads(source: &str, permit_program_path_lookup: bool) -> Vec<String> {
    let syntax = match syn::parse_file(source) {
        Ok(syntax) => syntax,
        Err(error) => return vec![format!("unparseable Rust source: {error}")],
    };
    let mut scanner = HostEnvironmentReadScanner {
        permit_program_path_lookup,
        ..HostEnvironmentReadScanner::default()
    };
    scanner.visit_file(&syntax);
    scanner.hits
}

#[derive(Default)]
struct HostEnvironmentReadScanner {
    permit_program_path_lookup: bool,
    functions: Vec<String>,
    hits: Vec<String>,
}

impl HostEnvironmentReadScanner {
    fn record_path(&mut self, path: &syn::Path) {
        let segments = path_segments(path);
        if forbidden_path_reference(&segments) {
            self.hits.push(segments.join("::"));
        }
    }

    fn is_permitted_path_lookup(&self, call: &ExprCall) -> bool {
        let Expr::Path(function) = call.func.as_ref() else {
            return false;
        };
        let segments = path_segments(&function.path);
        self.permit_program_path_lookup
            && self
                .functions
                .last()
                .is_some_and(|name| name == "resolve_program")
            && segments == ["std", "env", "var_os"]
            && call.args.len() == 1
            && matches!(
                call.args.first(),
                Some(Expr::Lit(literal))
                    if matches!(&literal.lit, Lit::Str(value) if value.value() == "PATH")
            )
    }
}

impl<'ast> Visit<'ast> for HostEnvironmentReadScanner {
    fn visit_item(&mut self, node: &'ast Item) {
        if item_attributes(node).is_some_and(test_only) {
            return;
        }
        visit::visit_item(self, node);
    }

    fn visit_impl_item(&mut self, node: &'ast ImplItem) {
        if impl_item_attributes(node).is_some_and(test_only) {
            return;
        }
        visit::visit_impl_item(self, node);
    }

    fn visit_trait_item(&mut self, node: &'ast TraitItem) {
        if trait_item_attributes(node).is_some_and(test_only) {
            return;
        }
        visit::visit_trait_item(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        if !test_only(&node.attrs) {
            visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if test_only(&node.attrs) {
            return;
        }
        self.functions.push(normalized_ident(&node.sig.ident));
        visit::visit_item_fn(self, node);
        self.functions.pop();
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if test_only(&node.attrs) {
            return;
        }
        self.functions.push(normalized_ident(&node.sig.ident));
        visit::visit_impl_item_fn(self, node);
        self.functions.pop();
    }

    fn visit_item_use(&mut self, node: &'ast ItemUse) {
        if test_only(&node.attrs) {
            return;
        }
        let mut imports = Vec::new();
        flatten_use_tree(&node.tree, &mut Vec::new(), &mut imports);
        for import in imports {
            if forbidden_import(&import) {
                self.hits.push(format!("use {}", import.join("::")));
            }
        }
    }

    fn visit_item_extern_crate(&mut self, node: &'ast ItemExternCrate) {
        if test_only(&node.attrs) {
            return;
        }
        let crate_name = normalized_ident(&node.ident);
        if crate_name == "dotenvy" || (crate_name == "std" && node.rename.is_some()) {
            self.hits.push(format!("extern crate {crate_name}"));
        }
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if self.is_permitted_path_lookup(node) {
            for argument in &node.args {
                self.visit_expr(argument);
            }
            return;
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &'ast ExprPath) {
        self.record_path(&node.path);
        visit::visit_expr_path(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        let path = path_segments(&node.path);
        if forbidden_macro_path(&path) && !permitted_build_env_macro(node) {
            self.hits.push(format!("macro {}", path.join("::")));
        }
        self.hits
            .extend(forbidden_macro_tokens(node.tokens.clone()));
        visit::visit_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast StmtMacro) {
        if !test_only(&node.attrs) {
            visit::visit_stmt_macro(self, node);
        }
    }

    fn visit_local(&mut self, node: &'ast Local) {
        if !test_only(&node.attrs) {
            visit::visit_local(self, node);
        }
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        if !test_only(&node.attrs) {
            visit::visit_expr_macro(self, node);
        }
    }
}

fn item_attributes(item: &Item) -> Option<&[Attribute]> {
    match item {
        Item::Const(item) => Some(&item.attrs),
        Item::Enum(item) => Some(&item.attrs),
        Item::ExternCrate(item) => Some(&item.attrs),
        Item::Fn(item) => Some(&item.attrs),
        Item::ForeignMod(item) => Some(&item.attrs),
        Item::Impl(item) => Some(&item.attrs),
        Item::Macro(item) => Some(&item.attrs),
        Item::Mod(item) => Some(&item.attrs),
        Item::Static(item) => Some(&item.attrs),
        Item::Struct(item) => Some(&item.attrs),
        Item::Trait(item) => Some(&item.attrs),
        Item::TraitAlias(item) => Some(&item.attrs),
        Item::Type(item) => Some(&item.attrs),
        Item::Union(item) => Some(&item.attrs),
        Item::Use(item) => Some(&item.attrs),
        Item::Verbatim(_) => None,
        _ => None,
    }
}

fn impl_item_attributes(item: &ImplItem) -> Option<&[Attribute]> {
    match item {
        ImplItem::Const(item) => Some(&item.attrs),
        ImplItem::Fn(item) => Some(&item.attrs),
        ImplItem::Type(item) => Some(&item.attrs),
        ImplItem::Macro(item) => Some(&item.attrs),
        ImplItem::Verbatim(_) => None,
        _ => None,
    }
}

fn trait_item_attributes(item: &TraitItem) -> Option<&[Attribute]> {
    match item {
        TraitItem::Const(item) => Some(&item.attrs),
        TraitItem::Fn(item) => Some(&item.attrs),
        TraitItem::Type(item) => Some(&item.attrs),
        TraitItem::Macro(item) => Some(&item.attrs),
        TraitItem::Verbatim(_) => None,
        _ => None,
    }
}

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

fn normalized_ident(ident: &proc_macro2::Ident) -> String {
    let name = ident.to_string();
    name.strip_prefix("r#").unwrap_or(&name).to_string()
}

fn forbidden_path_reference(segments: &[String]) -> bool {
    segments
        .last()
        .is_some_and(|name| matches!(name.as_str(), "var" | "vars" | "var_os" | "vars_os"))
        || segments.iter().any(|segment| segment == "dotenvy")
}

fn forbidden_macro_path(segments: &[String]) -> bool {
    forbidden_path_reference(segments)
        || segments
            .last()
            .is_some_and(|name| matches!(name.as_str(), "env" | "option_env"))
}

fn permitted_build_env_macro(node: &Macro) -> bool {
    path_segments(&node.path) == ["env"]
        && syn::parse2::<syn::LitStr>(node.tokens.clone()).is_ok_and(permitted_build_env_key)
}

fn permitted_build_env_key(key: syn::LitStr) -> bool {
    matches!(
        key.value().as_str(),
        "CARGO_PKG_VERSION" | "OPI_SANDBOX_BUILD_TARGET"
    )
}

fn permitted_build_env_tokens(trees: &[TokenTree], index: usize) -> bool {
    if index >= 2
        && trees[index - 2..index]
            .iter()
            .all(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':'))
    {
        return false;
    }
    let Some(TokenTree::Group(arguments)) = trees.get(index + 2) else {
        return false;
    };
    syn::parse2::<syn::LitStr>(arguments.stream()).is_ok_and(permitted_build_env_key)
}

fn forbidden_macro_tokens(tokens: TokenStream) -> Vec<String> {
    // This intentionally does not claim macro expansion. It recursively scans
    // the lexical token tree and rejects suspicious paths/identifiers that can
    // assemble or conceal an ambient-environment read.
    let trees: Vec<TokenTree> = tokens.into_iter().collect();
    let mut hits = Vec::new();
    let mut identifiers = BTreeSet::new();
    collect_macro_identifiers(&trees, &mut identifiers);
    if identifiers.contains("std")
        && identifiers.contains("env")
        && identifiers
            .iter()
            .any(|name| matches!(name.as_str(), "var" | "vars" | "var_os" | "vars_os"))
    {
        hits.push("macro token identifiers assemble std::env::var*".to_string());
    }
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Group(group) => hits.extend(forbidden_macro_tokens(group.stream())),
            TokenTree::Ident(ident) => {
                let name = normalized_ident(ident);
                if name == "dotenvy" {
                    hits.push("macro token dotenvy".to_string());
                }
                if name == "std" && forbidden_std_env_macro_path(&trees, index) {
                    hits.push("macro token std::env".to_string());
                }
                if matches!(name.as_str(), "env" | "option_env")
                    && trees.get(index + 1).is_some_and(
                        |next| matches!(next, TokenTree::Punct(punct) if punct.as_char() == '!'),
                    )
                    && (name != "env" || !permitted_build_env_tokens(&trees, index))
                {
                    hits.push(format!("macro token {name}!"));
                }
                if matches!(name.as_str(), "var" | "vars" | "var_os" | "vars_os")
                    && index >= 2
                    && trees[index - 2..index].iter().all(
                        |previous| matches!(previous, TokenTree::Punct(punct) if punct.as_char() == ':'),
                    )
                {
                    hits.push(format!("macro token ::{name}"));
                }
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
    hits
}

fn collect_macro_identifiers(trees: &[TokenTree], identifiers: &mut BTreeSet<String>) {
    for tree in trees {
        match tree {
            TokenTree::Group(group) => {
                let nested: Vec<TokenTree> = group.stream().into_iter().collect();
                collect_macro_identifiers(&nested, identifiers);
            }
            TokenTree::Ident(ident) => {
                identifiers.insert(normalized_ident(ident));
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => {}
        }
    }
}

fn forbidden_std_env_macro_path(trees: &[TokenTree], index: usize) -> bool {
    let has_std_env_prefix = trees.get(index + 1..index + 3).is_some_and(|separator| {
        separator
            .iter()
            .all(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':'))
    }) && trees
        .get(index + 3)
        .is_some_and(|token| token_ident_is(token, "env"));
    if !has_std_env_prefix {
        return false;
    }

    let has_tail_separator = trees.get(index + 4..index + 6).is_some_and(|separator| {
        separator
            .iter()
            .all(|token| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':'))
    });
    if !has_tail_separator {
        return true;
    }
    !trees.get(index + 6).is_some_and(|token| {
        ["args", "args_os", "consts"]
            .iter()
            .any(|allowed| token_ident_is(token, allowed))
    })
}

fn token_ident_is(token: &TokenTree, expected: &str) -> bool {
    let TokenTree::Ident(ident) = token else {
        return false;
    };
    normalized_ident(ident) == expected
}

fn flatten_use_tree(tree: &UseTree, prefix: &mut Vec<String>, imports: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(normalized_ident(&path.ident));
            flatten_use_tree(&path.tree, prefix, imports);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut import = prefix.clone();
            import.push(normalized_ident(&name.ident));
            imports.push(import);
        }
        UseTree::Rename(rename) => {
            let mut import = prefix.clone();
            import.push(normalized_ident(&rename.ident));
            imports.push(import);
        }
        UseTree::Glob(_) => imports.push(prefix.clone()),
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(item, prefix, imports);
            }
        }
    }
}

fn forbidden_import(segments: &[String]) -> bool {
    if segments.first().is_some_and(|segment| segment == "dotenvy") {
        return true;
    }
    if segments.first().is_some_and(|segment| segment == "std")
        && segments.get(1).is_some_and(|segment| segment == "env")
    {
        return match segments.get(2).map(String::as_str) {
            None | Some("self") => true,
            Some(name) => matches!(name, "var" | "vars" | "var_os" | "vars_os"),
        };
    }
    false
}

fn test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("test") {
            return true;
        }
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        let Meta::List(cfg) = &attribute.meta else {
            return false;
        };
        syn::parse2::<Meta>(cfg.tokens.clone())
            .map(|predicate| !cfg_possibilities(&predicate).can_true)
            .unwrap_or(false)
    })
}

#[derive(Clone, Copy)]
struct Possibilities {
    can_true: bool,
    can_false: bool,
}

fn cfg_possibilities(predicate: &Meta) -> Possibilities {
    match predicate {
        Meta::Path(path) if path.is_ident("test") => Possibilities {
            can_true: false,
            can_false: true,
        },
        Meta::Path(_) | Meta::NameValue(_) => unknown_possibilities(),
        Meta::List(list) if list.path.is_ident("all") => {
            let Some(predicates) = nested_cfg_predicates(list) else {
                return unknown_possibilities();
            };
            predicates.iter().map(cfg_possibilities).fold(
                Possibilities {
                    can_true: true,
                    can_false: false,
                },
                |combined, item| Possibilities {
                    can_true: combined.can_true && item.can_true,
                    can_false: combined.can_false || item.can_false,
                },
            )
        }
        Meta::List(list) if list.path.is_ident("any") => {
            let Some(predicates) = nested_cfg_predicates(list) else {
                return unknown_possibilities();
            };
            predicates.iter().map(cfg_possibilities).fold(
                Possibilities {
                    can_true: false,
                    can_false: true,
                },
                |combined, item| Possibilities {
                    can_true: combined.can_true || item.can_true,
                    can_false: combined.can_false && item.can_false,
                },
            )
        }
        Meta::List(list) if list.path.is_ident("not") => {
            let Some(predicates) = nested_cfg_predicates(list) else {
                return unknown_possibilities();
            };
            if predicates.len() != 1 {
                return unknown_possibilities();
            }
            let predicate = predicates.first().expect("length checked");
            let inner = cfg_possibilities(predicate);
            Possibilities {
                can_true: inner.can_false,
                can_false: inner.can_true,
            }
        }
        Meta::List(_) => unknown_possibilities(),
    }
}

fn nested_cfg_predicates(list: &syn::MetaList) -> Option<Punctuated<Meta, Token![,]>> {
    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()
}

fn unknown_possibilities() -> Possibilities {
    Possibilities {
        can_true: true,
        can_false: true,
    }
}

/// The resolve graph depends on `opi-protocol` and has NO `opi-agent` or
/// `opi-coding-agent` edge (runtime/normal edges; dev-deps excluded).
#[test]
fn depends_only_on_neutral_crates_not_opi_agent_or_coding_agent() {
    let output = Command::new(env!("CARGO"))
        .args(["tree", "-p", env!("CARGO_PKG_NAME"), "--edges", "normal"])
        .current_dir(manifest_dir())
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(
        tree.contains("opi-protocol"),
        "crate must depend on opi-protocol;\n{tree}"
    );
    for forbidden in ["opi-agent", "opi-coding-agent"] {
        assert!(
            !tree.contains(forbidden),
            "forbidden transitive dependency `{forbidden}` present:\n{tree}"
        );
    }
}

/// Static tripwire: no source file under `src/` (library OR binary) calls a
/// runtime host-environment-VAR-read API except the effective inherited
/// `PATH`. The forbidden needles are
/// `env::var`, `env::vars`, `var_os`, `vars_os`, and `dotenvy` — the APIs that
/// read host configuration/state. `std::env::var_os("PATH")` is permitted
/// because inherited PATH resolution is an explicit execution input.
/// `env::args`, `env::args_os` (CLI argument
/// plumbing) and `env::consts` (compile-time constants such as `consts::OS`)
/// are PERMITTED and intentionally absent from the needle set. `env!` is
/// limited to the exact build metadata inputs already used by this crate;
/// other `env!`/`option_env!` paths and suspicious macro token trees are
/// rejected without claiming full macro expansion.
///
/// This is NOT the load-bearing proof that the crate reads no Opi configuration
/// — the structural proof is
/// `depends_only_on_neutral_crates_not_opi_agent_or_coding_agent` (no
/// `opi-agent`/`opi-coding-agent` dependency; the sole opi-internal dep is the
/// pure-types `opi-protocol`). This tripwire catches a DIRECT env-var read
/// (e.g. a future `env::var("OPI_SESSIONS_DIR")`) that the dependency graph
/// cannot see; it walks `src/` recursively so `src/platform/*` is covered
/// (Phase 16 task 16.11.2 audit fold: narrow the needle, not the scope).
#[test]
fn source_calls_no_host_environment_var_read_api() {
    let src = manifest_dir().join("src");
    let mut hits = String::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let permit_program_path_lookup = path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                == Some("runner")
                && path.file_name().and_then(|name| name.to_str()) == Some("preparation.rs");
            for needle in host_environment_reads(&content, permit_program_path_lookup) {
                hits.push_str(&format!("{}: `{needle}`\n", path.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "runtime host-environment-VAR-read API found in source:\n{hits}"
    );
}

#[test]
fn structural_scanner_ignores_comments_test_code_and_benign_strings() {
    let fixture = r#"
// std::env::var_os("OPI_SESSIONS_DIR")
const EXAMPLE: &str = "env::vars_os() is forbidden in production";

#[cfg(test)]
mod tests {
    fn may_read_test_environment() {
        let _ = std::env::var("OPI_TEST_ONLY");
    }
}

#[test]
fn direct_test_may_read_environment() {
    let _ = std::env::var_os("OPI_DIRECT_TEST_ONLY");
}

fn production_cli() {
    let _ = std::env::args_os();
    let _ = env!("CARGO_PKG_VERSION");
}
"#;

    assert!(
        host_environment_reads(fixture, false).is_empty(),
        "comments, strings, test-only code, and argument APIs are not host environment reads"
    );
}

#[test]
fn structural_scanner_catches_aliases_computed_keys_and_wrong_scopes() {
    let fixtures = [
        r#"fn outside_scope() { let _ = std::env::var_os("PATH"); }"#,
        r#"fn aliased() { use std::env as host; let _ = host::var_os("PATH"); }"#,
        r#"fn computed() { let key = "PATH"; let _ = std::env::var_os(key); }"#,
        r#"fn config() { let _ = std::env::var("OPI_SESSIONS_DIR"); }"#,
        r#"fn dotenv() { let _ = dotenvy::dotenv(); }"#,
    ];

    for fixture in fixtures {
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "production host-environment read was not detected: {fixture}"
        );
    }
}

#[test]
fn structural_scanner_catches_imports_function_values_and_parenthesized_calls() {
    let fixtures = [
        r#"fn imported() { use std::env::var_os as read; let _ = read("PATH"); }"#,
        r#"fn function_value() { let read = std::env::var_os; let _ = read("PATH"); }"#,
        r#"fn parenthesized() { let _ = (std::env::var_os)("PATH"); }"#,
        r#"fn module_alias() { use std::env as e; let _ = e::var("X"); }"#,
        r#"fn dotenv_alias() { use dotenvy::dotenv as load; let _ = load(); }"#,
        r#"fn dotenv_direct() { use dotenvy::dotenv; let _ = dotenv(); }"#,
    ];

    for fixture in fixtures {
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "forbidden path reference or import was not detected: {fixture}"
        );
    }
}

#[test]
fn structural_scanner_normalizes_raw_use_tree_identifiers() {
    for fixture in [
        r#"fn imported() { use std::env::r#var_os as read; let _ = read("PATH"); }"#,
        r#"fn imported() { use r#dotenvy::dotenv as load; let _ = load(); }"#,
    ] {
        assert!(
            syn::parse_file(fixture).is_ok(),
            "raw-identifier fixture must be valid Rust: {fixture}"
        );
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "raw use-tree identifier bypassed the scanner: {fixture}"
        );
    }
}

#[test]
fn structural_scanner_catches_extern_crate_aliases() {
    for fixture in [
        r#"extern crate dotenvy as d; fn load() { let _ = d::dotenv(); }"#,
        r#"extern crate dotenvy; fn load() { let _ = dotenvy::dotenv(); }"#,
        r#"extern crate std as s; fn read() { let _ = s::env::var_os("PATH"); }"#,
        r#"extern crate std as s; fn no_read() {}"#,
    ] {
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "forbidden extern-crate import was not detected: {fixture}"
        );
    }

    let test_only = r#"
#[cfg(all(test, unix))]
extern crate dotenvy as d;
"#;
    assert!(
        host_environment_reads(test_only, false).is_empty(),
        "true test-only extern-crate imports must be ignored"
    );
}

#[test]
fn structural_scanner_catches_forbidden_macro_paths_and_tokens() {
    for fixture in [
        r#"macro_rules! read_host { () => { std::env::var_os("PATH") }; }"#,
        r#"macro_rules! read_host { () => { std::env::var("OPI_CONFIG") }; }"#,
        r#"macro_rules! read_host { ($method:ident) => { std::env::$method("OPI_CONFIG") }; }"#,
        r#"macro_rules! read_host { () => { env!("OPI_CONFIG") }; }"#,
        r#"macro_rules! read_host { () => { option_env!("OPI_CONFIG") }; }"#,
        r#"read_host!(std::env::var_os("PATH"));"#,
        r#"read_host!(std::env::var("OPI_CONFIG"));"#,
        r#"read_host!(env!("OPI_CONFIG"));"#,
        r#"read_host!(option_env!("OPI_CONFIG"));"#,
        r#"read_host!(std::env, var);"#,
        r#"read_host!(host::env!("CARGO_PKG_VERSION"));"#,
        r#"const VALUE: &str = env!("OPI_CONFIG");"#,
        r#"const VALUE: Option<&str> = option_env!("OPI_CONFIG");"#,
        r#"macro_rules! load_host { () => { dotenvy::dotenv() }; }"#,
        r#"load_host!(dotenvy::dotenv());"#,
        r#"dotenvy::load!();"#,
        r#"r#dotenvy::load!();"#,
    ] {
        assert!(
            syn::parse_file(fixture).is_ok(),
            "macro scanner fixture must be valid Rust: {fixture}"
        );
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "forbidden macro path or token tree was not detected: {fixture}"
        );
    }
}

#[test]
fn structural_scanner_catches_assembled_macro_identifier_paths() {
    let fixture = r#"
macro_rules! read_host {
    ($root:ident, $module:ident, $function:ident) => {
        $root::$module::$function("OPI_CONFIG")
    };
}
read_host!(std, env, var_os);
"#;
    assert!(syn::parse_file(fixture).is_ok());
    assert!(
        !host_environment_reads(fixture, false).is_empty(),
        "separated std/env/var_os identifiers must be rejected conservatively"
    );
}

#[test]
fn structural_scanner_ignores_benign_and_test_only_macros() {
    let fixture = r#"
// macro_rules! commented { () => { std::env::var_os("PATH") }; }
const EXAMPLE: &str = "option_env!(OPI_CONFIG) and dotenvy::dotenv()";

macro_rules! benign {
    ($variable:ident) => {{
        let $variable = "std::env::var(\"OPI_CONFIG\")";
        module::variable($variable)
    }};
}

#[cfg(all(test, unix))]
macro_rules! test_read {
    () => { std::env::var("OPI_TEST_ONLY") };
}

#[cfg(test)]
test_read!(dotenvy::dotenv());

#[cfg(test)]
test_read!(env!("OPI_TEST_ONLY"));

fn production_function() {
    #[cfg(all(test, unix))]
    test_read!(option_env!("OPI_TEST_ONLY"));
}
"#;

    assert!(
        host_environment_reads(fixture, false).is_empty(),
        "comments, literals, benign macros, and test-only macro items must be ignored"
    );
}

#[test]
fn structural_scanner_allows_args_os_and_skips_test_only_local_macros() {
    let fixture = r#"
macro_rules! command_line {
    () => { std::env::args_os() };
}

fn production_function() {
    let _args = command_line!();

    #[cfg(test)]
    let _local = env!("OPI_TEST_ONLY");

    let _expression = {
        #[cfg(test)]
        env!("OPI_TEST_ONLY")
    };
}
"#;
    assert!(syn::parse_file(fixture).is_ok());
    assert!(
        host_environment_reads(fixture, false).is_empty(),
        "args_os and cfg(test) local/expression macros must not false-positive"
    );
}

#[test]
fn structural_scanner_keeps_cfg_not_test_local_macros_in_scope() {
    for fixture in [
        r#"fn production() { #[cfg(not(test))] let _value = env!("OPI_CONFIG"); }"#,
        r#"fn production() { let _value = { #[cfg(not(test))] env!("OPI_CONFIG") }; }"#,
    ] {
        assert!(
            syn::parse_file(fixture).is_ok(),
            "cfg(not(test)) fixture must be valid Rust: {fixture}"
        );
        assert!(
            !host_environment_reads(fixture, false).is_empty(),
            "cfg(not(test)) macro must remain production-scanned: {fixture}"
        );
    }
}

#[test]
fn structural_scanner_evaluates_test_cfg_predicates_conservatively() {
    for test_only in [
        r#"#[cfg(all(test, unix))] fn helper() { let _ = std::env::var("X"); }"#,
        r#"#[cfg(any(test, test))] fn helper() { let _ = std::env::var("X"); }"#,
        r#"#[cfg(all(test, unix))] const READ: fn(&str) -> Option<std::ffi::OsString> = std::env::var_os;"#,
    ] {
        assert!(
            host_environment_reads(test_only, false).is_empty(),
            "cfg predicate that requires test must be ignored: {test_only}"
        );
    }

    for production_possible in [
        r#"#[cfg(any(test, windows))] fn helper() { let _ = std::env::var("X"); }"#,
        r#"#[cfg(not(test))] fn helper() { let _ = std::env::var("X"); }"#,
    ] {
        assert!(
            !host_environment_reads(production_possible, false).is_empty(),
            "cfg predicate that can compile without test must still be scanned: {production_possible}"
        );
    }
}

#[test]
fn structural_scanner_allows_only_literal_path_in_program_resolution() {
    let allowed = r#"
fn resolve_program() {
    let _ = std::env::var_os("PATH");
}
"#;
    assert!(host_environment_reads(allowed, true).is_empty());
    assert!(
        !host_environment_reads(allowed, false).is_empty(),
        "the PATH exception is scoped to runner/preparation.rs as well as resolve_program"
    );

    let computed = r#"
fn resolve_program() {
    let key = "PATH";
    let _ = std::env::var_os(key);
}
"#;
    assert!(!host_environment_reads(computed, true).is_empty());
}
