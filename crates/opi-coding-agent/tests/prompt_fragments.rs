//! Integration tests for prompt fragments/templates with progressive discovery (task 4.7.2).
//!
//! Covers: frontmatter parsing, argument expansion, precedence-based discovery,
//! missing/invalid resources, progressive disclosure, registry operations,
//! and localized documentation updates.

use std::fs;

use opi_coding_agent::prompt_fragment::{
    FragmentArgument, FragmentDiscoveryError, FragmentManifest, FragmentRegistry,
    discover_fragments, expand_fragment_body,
};
use opi_coding_agent::resource::{DiscoveryLayer, DiscoveryLayerKind};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a fragment directory with a FRAGMENT.md file in `parent`.
fn write_fragment(parent: &std::path::Path, dir_name: &str, frontmatter: &str, body: &str) {
    let dir = parent.join(dir_name);
    fs::create_dir_all(&dir).unwrap();
    let content = format!("---\n{frontmatter}\n---\n{body}");
    fs::write(dir.join("FRAGMENT.md"), content).unwrap();
}

/// Build a single discovery layer at `root/subdirectory` with given precedence.
fn layer(root: &std::path::Path, subdirectory: &str, precedence: u32) -> DiscoveryLayer {
    DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: root.to_path_buf(),
        subdirectory: Some(subdirectory.to_string()),
        precedence,
    }
}

// ---------------------------------------------------------------------------
// 1. Manifest parsing — basic
// ---------------------------------------------------------------------------

#[test]
fn test_parse_valid_frontmatter_no_arguments() {
    let content = "---\nname: hello\ndescription: Says hello.\n---\nHello, world!";
    let manifest =
        FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.name, "hello");
    assert_eq!(manifest.description, "Says hello.");
    assert!(manifest.arguments.is_empty());
}

#[test]
fn test_parse_valid_frontmatter_with_required_arguments() {
    let content =
        "---\nname: greet\ndescription: Greets a person.\narguments: name\n---\nHello, {{name}}!";
    let manifest =
        FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.name, "greet");
    assert_eq!(manifest.arguments.len(), 1);
    assert_eq!(manifest.arguments[0].name, "name");
    assert!(manifest.arguments[0].required);
    assert!(manifest.arguments[0].default.is_none());
}

#[test]
fn test_parse_frontmatter_with_default_argument() {
    let content = "---\nname: summarize\ndescription: Summarizes text.\narguments: input, format=markdown\n---\nSummarize {{input}} in {{format}}.";
    let manifest =
        FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.arguments.len(), 2);

    assert_eq!(manifest.arguments[0].name, "input");
    assert!(manifest.arguments[0].required);
    assert!(manifest.arguments[0].default.is_none());

    assert_eq!(manifest.arguments[1].name, "format");
    assert!(!manifest.arguments[1].required);
    assert_eq!(manifest.arguments[1].default.as_deref(), Some("markdown"));
}

#[test]
fn test_parse_frontmatter_multiple_defaults() {
    let content = "---\nname: translate\ndescription: Translates text.\narguments: text, from=en, to=fr\n---\nTranslate {{text}} from {{from}} to {{to}}.";
    let manifest =
        FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.arguments.len(), 3);

    assert_eq!(manifest.arguments[0].name, "text");
    assert!(manifest.arguments[0].required);

    assert_eq!(manifest.arguments[1].name, "from");
    assert!(!manifest.arguments[1].required);
    assert_eq!(manifest.arguments[1].default.as_deref(), Some("en"));

    assert_eq!(manifest.arguments[2].name, "to");
    assert!(!manifest.arguments[2].required);
    assert_eq!(manifest.arguments[2].default.as_deref(), Some("fr"));
}

// ---------------------------------------------------------------------------
// 2. Manifest parsing — validation errors
// ---------------------------------------------------------------------------

#[test]
fn test_parse_missing_name() {
    let content = "---\ndescription: No name.\n---\nBody";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::MissingField { field, .. } => assert_eq!(field, "name"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_missing_description() {
    let content = "---\nname: no-desc\n---\nBody";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::MissingField { field, .. } => assert_eq!(field, "description"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_empty_name() {
    let content = "---\nname: \"\"\ndescription: Empty name.\n---\n";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::MissingField { field, .. } => assert_eq!(field, "name"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_invalid_name_characters() {
    let content = "---\nname: Invalid Name!\ndescription: Bad chars.\n---\n";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidName { .. } => {}
        other => panic!("expected InvalidName, got: {other}"),
    }
}

#[test]
fn test_parse_name_too_long() {
    let long_name = "a".repeat(65);
    let content = format!("---\nname: {long_name}\ndescription: Too long.\n---\n");
    let err = FragmentManifest::from_fragment_md(&content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidName { .. } => {}
        other => panic!("expected InvalidName, got: {other}"),
    }
}

#[test]
fn test_parse_name_at_max_length() {
    let name = "a".repeat(64);
    let content = format!("---\nname: {name}\ndescription: Max ok.\n---\n");
    let manifest =
        FragmentManifest::from_fragment_md(&content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.name.len(), 64);
}

#[test]
fn test_parse_description_too_long() {
    let long_desc = "x".repeat(1025);
    let content = format!("---\nname: ok\ndescription: {long_desc}\n---\n");
    let err = FragmentManifest::from_fragment_md(&content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidDescription { .. } => {}
        other => panic!("expected InvalidDescription, got: {other}"),
    }
}

#[test]
fn test_parse_description_at_max_length() {
    let desc = "x".repeat(1024);
    let content = format!("---\nname: ok\ndescription: {desc}\n---\n");
    let manifest =
        FragmentManifest::from_fragment_md(&content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.description.len(), 1024);
}

#[test]
fn test_parse_no_frontmatter_delimiters() {
    let content = "Just some markdown content without frontmatter.";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidFrontmatter { .. } => {}
        other => panic!("expected InvalidFrontmatter, got: {other}"),
    }
}

#[test]
fn test_parse_valid_name_with_hyphens_and_digits() {
    let content = "---\nname: my-fragment-v2\ndescription: Valid.\n---\n";
    let manifest =
        FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md")).unwrap();
    assert_eq!(manifest.name, "my-fragment-v2");
}

#[test]
fn test_parse_invalid_argument_name() {
    let content = "---\nname: bad-arg\ndescription: Bad arg.\narguments: invalid name!\n---\nBody";
    let err = FragmentManifest::from_fragment_md(content, std::path::Path::new("FRAGMENT.md"))
        .unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidArgument { .. } => {}
        other => panic!("expected InvalidArgument, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Argument expansion
// ---------------------------------------------------------------------------

#[test]
fn test_expand_with_required_args() {
    let body = "Hello, {{name}}!";
    let args = vec![FragmentArgument {
        name: "name".into(),
        required: true,
        default: None,
    }];
    let values = std::collections::HashMap::from([("name".to_string(), "world".to_string())]);
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Hello, world!");
}

#[test]
fn test_expand_with_default_arg_not_provided() {
    let body = "Format: {{format}}";
    let args = vec![FragmentArgument {
        name: "format".into(),
        required: false,
        default: Some("json".into()),
    }];
    let values = std::collections::HashMap::new();
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Format: json");
}

#[test]
fn test_expand_with_default_arg_overridden() {
    let body = "Format: {{format}}";
    let args = vec![FragmentArgument {
        name: "format".into(),
        required: false,
        default: Some("json".into()),
    }];
    let values = std::collections::HashMap::from([("format".to_string(), "xml".to_string())]);
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Format: xml");
}

#[test]
fn test_expand_missing_required_arg() {
    let body = "Hello, {{name}}!";
    let args = vec![FragmentArgument {
        name: "name".into(),
        required: true,
        default: None,
    }];
    let values = std::collections::HashMap::new();
    let err = expand_fragment_body(body, &args, &values).unwrap_err();
    match err {
        FragmentDiscoveryError::MissingArgument { argument, .. } => {
            assert_eq!(argument, "name");
        }
        other => panic!("expected MissingArgument, got: {other}"),
    }
}

#[test]
fn test_expand_multiple_args_mixed() {
    let body = "Translate {{text}} from {{from}} to {{to}}.";
    let args = vec![
        FragmentArgument {
            name: "text".into(),
            required: true,
            default: None,
        },
        FragmentArgument {
            name: "from".into(),
            required: false,
            default: Some("en".into()),
        },
        FragmentArgument {
            name: "to".into(),
            required: false,
            default: Some("fr".into()),
        },
    ];
    let values = std::collections::HashMap::from([("text".to_string(), "hello".to_string())]);
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Translate hello from en to fr.");
}

#[test]
fn test_expand_no_args_needed() {
    let body = "No placeholders here.";
    let args = vec![];
    let values = std::collections::HashMap::new();
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "No placeholders here.");
}

#[test]
fn test_expand_extra_args_ignored() {
    let body = "Hello!";
    let args = vec![];
    let values = std::collections::HashMap::from([("extra".to_string(), "value".to_string())]);
    // Extra args are silently ignored — not an error.
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Hello!");
}

#[test]
fn test_expand_unexpected_placeholder_in_body() {
    // A placeholder in the body that doesn't match any declared argument.
    // Should be left as-is (no error, just not replaced).
    let body = "Hello, {{unknown}}!";
    let args = vec![];
    let values = std::collections::HashMap::new();
    let result = expand_fragment_body(body, &args, &values).unwrap();
    assert_eq!(result, "Hello, {{unknown}}!");
}

// ---------------------------------------------------------------------------
// 4. Discovery — basic
// ---------------------------------------------------------------------------

#[test]
fn test_discover_single_fragment() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "hello",
        "name: hello\ndescription: Says hello.",
        "Hello!",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "hello");
    assert_eq!(resources[0].manifest.description, "Says hello.");
}

#[test]
fn test_discover_multiple_fragments() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(&dir, "alpha", "name: alpha\ndescription: First.", "A");
    write_fragment(&dir, "beta", "name: beta\ndescription: Second.", "B");

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].manifest.name, "alpha");
    assert_eq!(resources[1].manifest.name, "beta");
}

#[test]
fn test_discover_skips_dirs_without_fragment_md() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    fs::create_dir_all(dir.join("no-fragment")).unwrap();
    write_fragment(&dir, "real", "name: real\ndescription: Real.", "R");

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "real");
}

#[test]
fn test_discover_skips_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    fs::write(dir.join("readme.txt"), "not a fragment").unwrap();
    write_fragment(&dir, "valid", "name: valid\ndescription: Valid.", "V");

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 1);
}

// ---------------------------------------------------------------------------
// 5. Discovery — precedence
// ---------------------------------------------------------------------------

#[test]
fn test_discover_precedence_higher_wins() {
    let tmp = tempfile::tempdir().unwrap();

    let low = tmp.path().join("low");
    fs::create_dir_all(&low).unwrap();
    write_fragment(
        &low,
        "tool-a",
        "name: tool-a\ndescription: low version",
        "Low body.",
    );

    let high = tmp.path().join("high");
    fs::create_dir_all(&high).unwrap();
    write_fragment(
        &high,
        "tool-a",
        "name: tool-a\ndescription: high version",
        "High body.",
    );

    let layers = vec![layer(tmp.path(), "low", 0), layer(tmp.path(), "high", 1)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.description, "high version");
    assert_eq!(resources[0].layer_precedence, 1);
}

#[test]
fn test_discover_precedence_mixed_names() {
    let tmp = tempfile::tempdir().unwrap();

    let low = tmp.path().join("low");
    fs::create_dir_all(&low).unwrap();
    write_fragment(&low, "frag-a", "name: frag-a\ndescription: low-a", "A");
    write_fragment(&low, "frag-b", "name: frag-b\ndescription: low-b", "B");

    let high = tmp.path().join("high");
    fs::create_dir_all(&high).unwrap();
    write_fragment(&high, "frag-b", "name: frag-b\ndescription: high-b", "B2");
    write_fragment(&high, "frag-c", "name: frag-c\ndescription: high-c", "C");

    let layers = vec![layer(tmp.path(), "low", 0), layer(tmp.path(), "high", 1)];
    let resources = discover_fragments(&layers).unwrap();

    assert_eq!(resources.len(), 3);
    let names: Vec<&str> = resources.iter().map(|r| r.manifest.name.as_str()).collect();
    assert_eq!(names, vec!["frag-a", "frag-b", "frag-c"]);

    let frag_b = resources
        .iter()
        .find(|r| r.manifest.name == "frag-b")
        .unwrap();
    assert_eq!(frag_b.manifest.description, "high-b");
}

#[test]
fn test_discover_duplicate_name_same_layer_returns_error() {
    let tmp = tempfile::tempdir().unwrap();

    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();
    write_fragment(
        &dir,
        "first",
        "name: shared-frag\ndescription: First",
        "First body.",
    );
    write_fragment(
        &dir,
        "second",
        "name: shared-frag\ndescription: Second",
        "Second body.",
    );

    let err = discover_fragments(&[layer(tmp.path(), "fragments", 0)]).unwrap_err();
    assert!(matches!(
        err,
        FragmentDiscoveryError::DuplicateName { ref name, .. } if name == "shared-frag"
    ));
}

// ---------------------------------------------------------------------------
// 6. Discovery — missing / invalid resources
// ---------------------------------------------------------------------------

#[test]
fn test_discover_missing_scan_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let layers = vec![layer(tmp.path(), "nonexistent", 0)];
    let resources = discover_fragments(&layers).unwrap();
    assert!(resources.is_empty());
}

#[test]
fn test_discover_invalid_manifest_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    let bad_dir = dir.join("broken");
    fs::create_dir_all(&bad_dir).unwrap();
    fs::write(bad_dir.join("FRAGMENT.md"), "No frontmatter here.").unwrap();

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let err = discover_fragments(&layers).unwrap_err();
    match err {
        FragmentDiscoveryError::InvalidFrontmatter { .. } => {}
        other => panic!("expected InvalidFrontmatter, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 7. Progressive disclosure
// ---------------------------------------------------------------------------

#[test]
fn test_progressive_disclosure_metadata_without_body() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "lazy",
        "name: lazy\ndescription: Load on demand.",
        "# Full Template\n\nComplex instructions.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    assert_eq!(resources.len(), 1);

    assert_eq!(resources[0].manifest.name, "lazy");
    assert_eq!(resources[0].manifest.description, "Load on demand.");

    let body = resources[0].load_body().unwrap();
    assert!(body.contains("# Full Template"));
    assert!(body.contains("Complex instructions."));
}

#[test]
fn test_progressive_disclosure_load_body_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "ephemeral",
        "name: ephemeral\ndescription: Will be deleted.",
        "Original body.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();

    fs::remove_file(dir.join("ephemeral").join("FRAGMENT.md")).unwrap();

    let result = resources[0].load_body();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 8. FragmentRegistry
// ---------------------------------------------------------------------------

#[test]
fn test_registry_from_discovered_fragments() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "summarize",
        "name: summarize\ndescription: Summarize text.",
        "Summarize.",
    );
    write_fragment(
        &dir,
        "translate",
        "name: translate\ndescription: Translate text.",
        "Translate.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    let registry = FragmentRegistry::from_resources(resources);

    let names = registry.names();
    assert_eq!(names, vec!["summarize", "translate"]);

    let meta = registry.get("summarize").unwrap();
    assert_eq!(meta.manifest.description, "Summarize text.");
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn test_registry_format_for_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "review",
        "name: review\ndescription: Review code.",
        "Review.",
    );
    write_fragment(
        &dir,
        "greet",
        "name: greet\ndescription: Greet someone.\narguments: name",
        "Hello {{name}}.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    let registry = FragmentRegistry::from_resources(resources);

    let prompt = registry.format_for_prompt();
    assert!(prompt.contains("greet"));
    assert!(prompt.contains("Greet someone."));
    assert!(prompt.contains("review"));
    assert!(prompt.contains("Review code."));
}

#[test]
fn test_registry_format_for_rpc_metadata() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "translate",
        "name: translate\ndescription: Translate.\narguments: text, from=en, to=fr",
        "Translate {{text}} from {{from}} to {{to}}.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    let registry = FragmentRegistry::from_resources(resources);

    let rpc = registry.format_for_rpc_metadata();
    assert!(rpc.contains("translate"));
    assert!(rpc.contains("text"));
    assert!(rpc.contains("from"));
    assert!(rpc.contains("to"));
    // Should include default values in metadata.
    assert!(rpc.contains("en"));
    assert!(rpc.contains("fr"));
}

#[test]
fn test_registry_empty() {
    let registry = FragmentRegistry::from_resources(vec![]);
    assert!(registry.names().is_empty());
    assert!(registry.format_for_prompt().is_empty());
    assert!(registry.format_for_rpc_metadata().is_empty());
}

#[test]
fn test_registry_load_body() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "deep",
        "name: deep\ndescription: Has deep instructions.",
        "# Deep Fragment\n\nStep 1: Analyze.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    let registry = FragmentRegistry::from_resources(resources);

    let body = registry.load_body("deep").unwrap().unwrap();
    assert!(body.contains("# Deep Fragment"));
}

#[test]
fn test_registry_expand() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("fragments");
    fs::create_dir_all(&dir).unwrap();

    write_fragment(
        &dir,
        "translate",
        "name: translate\ndescription: Translate.\narguments: text, from=en, to=fr",
        "Translate {{text}} from {{from}} to {{to}}.",
    );

    let layers = vec![layer(tmp.path(), "fragments", 0)];
    let resources = discover_fragments(&layers).unwrap();
    let registry = FragmentRegistry::from_resources(resources);

    let values = std::collections::HashMap::from([
        ("text".to_string(), "hello".to_string()),
        ("to".to_string(), "de".to_string()),
    ]);
    let expanded = registry.expand("translate", &values).unwrap().unwrap();
    assert_eq!(expanded, "Translate hello from en to de.");
}

#[test]
fn test_registry_expand_unknown_fragment() {
    let registry = FragmentRegistry::from_resources(vec![]);
    let values = std::collections::HashMap::new();
    let result = registry.expand("nonexistent", &values);
    assert!(result.is_none());
}
