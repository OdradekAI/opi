//! Resource discovery tests: themes, skills, and extension resources.
//!
//! Merged from theme_discovery + skills_discovery + extension_resources to
//! cut integration-binary count (Candidate 4).

use std::fs;
use std::path::Path;

use opi_coding_agent::resource::{
    DiscoveryLayer, DiscoveryLayerKind, ResourceDiscoveryError, discover_extension_resources,
};
use opi_coding_agent::skill::{SkillDiscoveryError, SkillManifest, SkillRegistry, discover_skills};
use opi_coding_agent::theme_discovery::{
    ThemeDiscoveryError, ThemeManifest, ThemeRegistry, discover_themes,
};
use opi_tui::{THEME_TOKENS, Theme, is_valid_token, parse_color};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_theme(dir: &Path, name: &str, toml_content: &str) -> std::path::PathBuf {
    let theme_dir = dir.join(name);
    std::fs::create_dir_all(&theme_dir).unwrap();
    let path = theme_dir.join("theme.toml");
    std::fs::write(&path, toml_content).unwrap();
    path
}

fn layer(root: &Path, subdirectory: Option<&str>, precedence: u32) -> DiscoveryLayer {
    DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: root.to_path_buf(),
        subdirectory: subdirectory.map(String::from),
        precedence,
    }
}

/// A minimal valid theme.toml with all color tokens specified.
fn full_theme_toml(name: &str, description: &str) -> String {
    format!(
        r#"
name = "{name}"
description = "{description}"

[colors]
role_user = "Green"
role_assistant = "Cyan"
role_system = "Yellow"
role_tool = "Magenta"
status_bg = "DarkGray"
status_idle = "White"
status_thinking = "Yellow"
status_streaming = "Green"
status_tool = "Magenta"
status_tokens = "DarkGray"
editor_title = "Yellow"
editor_placeholder = "DarkGray"
code_title = "Yellow"
code_content = "Gray"
heading_h1 = "Cyan"
heading_h2 = "Yellow"
heading_h3 = "White"
italic = "Cyan"
diff_border = "Cyan"
diff_header = "Blue"
diff_context = "Gray"
diff_added = "Green"
diff_removed = "Red"
diff_no_changes = "DarkGray"
tool_running = "Yellow"
tool_success = "Green"
tool_error = "Red"
picker_title = "Cyan"
picker_selected_bg = "DarkGray"
picker_selected_fg = "White"
picker_filter = "Yellow"
picker_metadata = "DarkGray"
picker_empty = "DarkGray"
"#
    )
}

/// A theme.toml with only two color tokens (partial theme).
fn partial_theme_toml(name: &str, description: &str) -> String {
    format!(
        r##"
name = "{name}"
description = "{description}"

[colors]
role_user = "Red"
status_bg = "#1a1a2e"
"##
    )
}

// ===========================================================================
// 1. ThemeManifest parsing
// ===========================================================================

mod manifest_parsing {
    use super::*;

    #[test]
    fn parse_valid_minimal_manifest() {
        let toml = r#"
name = "my-theme"
description = "A test theme."
"#;
        let path = Path::new("my-theme/theme.toml");
        let manifest = ThemeManifest::from_toml(toml, path).unwrap();
        assert_eq!(manifest.name, "my-theme");
        assert_eq!(manifest.description, "A test theme.");
    }

    #[test]
    fn parse_manifest_with_colors_section() {
        let toml = r#"
name = "ocean"
description = "Ocean blues."

[colors]
role_user = "Cyan"
role_assistant = "Blue"
"#;
        let path = Path::new("ocean/theme.toml");
        let manifest = ThemeManifest::from_toml(toml, path).unwrap();
        assert_eq!(manifest.name, "ocean");
        assert_eq!(manifest.description, "Ocean blues.");
    }

    #[test]
    fn parse_manifest_missing_name() {
        let toml = r#"
description = "No name."
"#;
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(
            err,
            ThemeDiscoveryError::MissingField { ref field, .. } if field == "name"
        ));
    }

    #[test]
    fn parse_manifest_missing_description() {
        let toml = r#"
name = "no-desc"
"#;
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(
            err,
            ThemeDiscoveryError::MissingField { ref field, .. } if field == "description"
        ));
    }

    #[test]
    fn parse_manifest_empty_name() {
        let toml = r#"
name = ""
description = "Empty name."
"#;
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(err, ThemeDiscoveryError::MissingField { .. }));
    }

    #[test]
    fn parse_manifest_invalid_toml() {
        let toml = "this is not valid toml [[[";
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(err, ThemeDiscoveryError::InvalidManifest { .. }));
    }

    #[test]
    fn parse_manifest_name_validation_too_long() {
        let long_name = "a".repeat(65);
        let toml = format!(
            r#"
name = "{long_name}"
description = "Too long."
"#
        );
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(&toml, path).unwrap_err();
        assert!(matches!(err, ThemeDiscoveryError::InvalidName { .. }));
    }

    #[test]
    fn parse_manifest_name_validation_invalid_chars() {
        let toml = r#"
name = "My Theme!"
description = "Bad chars."
"#;
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(err, ThemeDiscoveryError::InvalidName { .. }));
    }

    #[test]
    fn parse_manifest_description_too_long() {
        let long_desc = "x".repeat(1025);
        let toml = format!(
            r#"
name = "ok-name"
description = "{long_desc}"
"#
        );
        let path = Path::new("x/theme.toml");
        let err = ThemeManifest::from_toml(&toml, path).unwrap_err();
        assert!(matches!(
            err,
            ThemeDiscoveryError::InvalidDescription { .. }
        ));
    }
}

// ===========================================================================
// 2. Color parsing
// ===========================================================================

mod color_parsing {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn parse_named_color() {
        assert_eq!(parse_color("Red").unwrap(), Color::Red);
        assert_eq!(parse_color("Green").unwrap(), Color::Green);
        assert_eq!(parse_color("Cyan").unwrap(), Color::Cyan);
        assert_eq!(parse_color("DarkGray").unwrap(), Color::DarkGray);
        assert_eq!(parse_color("LightCyan").unwrap(), Color::LightCyan);
        assert_eq!(parse_color("White").unwrap(), Color::White);
    }

    #[test]
    fn parse_hex_color() {
        assert_eq!(parse_color("#ff6600").unwrap(), Color::Rgb(255, 102, 0));
        assert_eq!(parse_color("#000000").unwrap(), Color::Rgb(0, 0, 0));
        assert_eq!(parse_color("#ffffff").unwrap(), Color::Rgb(255, 255, 255));
        assert_eq!(parse_color("#a6e22e").unwrap(), Color::Rgb(166, 226, 46));
    }

    #[test]
    fn parse_hex_color_case_insensitive() {
        assert_eq!(parse_color("#FF6600").unwrap(), Color::Rgb(255, 102, 0));
        assert_eq!(parse_color("#Ff66Aa").unwrap(), Color::Rgb(255, 102, 170));
    }

    #[test]
    fn parse_color_invalid() {
        assert!(parse_color("NotAColor").is_err());
        assert!(parse_color("#gggggg").is_err());
        assert!(parse_color("#12345").is_err()); // too short
        assert!(parse_color("").is_err());
    }
}

// ===========================================================================
// 3. Theme token schema
// ===========================================================================

mod token_schema {
    use super::*;

    #[test]
    fn theme_tokens_contains_all_known_fields() {
        // These are the 27 color fields from Theme struct (minus `name`)
        let expected = [
            "role_user",
            "role_assistant",
            "role_system",
            "role_tool",
            "status_bg",
            "status_idle",
            "status_thinking",
            "status_streaming",
            "status_tool",
            "status_tokens",
            "editor_title",
            "editor_placeholder",
            "code_title",
            "code_content",
            "heading_h1",
            "heading_h2",
            "heading_h3",
            "italic",
            "diff_border",
            "diff_header",
            "diff_context",
            "diff_added",
            "diff_removed",
            "diff_no_changes",
            "tool_running",
            "tool_success",
            "tool_error",
            "picker_title",
            "picker_selected_bg",
            "picker_selected_fg",
            "picker_filter",
            "picker_metadata",
            "picker_empty",
        ];
        for token in &expected {
            assert!(
                THEME_TOKENS.contains(token),
                "THEME_TOKENS missing: {token}"
            );
        }
    }

    #[test]
    fn theme_tokens_rejects_unknown() {
        assert!(!is_valid_token("nonexistent_token"));
        assert!(!is_valid_token("name"));
    }
}

// ===========================================================================
// 4. Discovery basic
// ===========================================================================

mod discovery_basic {
    use super::*;

    #[test]
    fn discover_from_single_layer() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "ocean",
            &full_theme_toml("ocean", "Ocean blues"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.name, "ocean");
    }

    #[test]
    fn discover_multiple_themes() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(&themes_dir, "alpha", &full_theme_toml("alpha", "A"));
        write_theme(&themes_dir, "beta", &full_theme_toml("beta", "B"));

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        assert_eq!(resources.len(), 2);
        // Sorted by name
        assert_eq!(resources[0].manifest.name, "alpha");
        assert_eq!(resources[1].manifest.name, "beta");
    }

    #[test]
    fn discover_skips_non_theme_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        // A directory without theme.toml
        let other_dir = themes_dir.join("not-a-theme");
        std::fs::create_dir_all(&other_dir).unwrap();

        // A file at top level (not a directory)
        std::fs::write(themes_dir.join("readme.txt"), "not a theme").unwrap();

        write_theme(
            &themes_dir,
            "real-theme",
            &full_theme_toml("real-theme", "Real"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.name, "real-theme");
    }

    #[test]
    fn discover_missing_scan_dir_returns_empty() {
        let layers = vec![layer(Path::new("/nonexistent/path"), None, 0)];
        let resources = discover_themes(&layers).unwrap();
        assert!(resources.is_empty());
    }
}

// ===========================================================================
// 5. Discovery precedence
// ===========================================================================

mod discovery_precedence {
    use super::*;

    #[test]
    fn higher_precedence_wins_on_name_collision() {
        let tmp = tempfile::tempdir().unwrap();

        let user_dir = tmp.path().join("user-themes");
        let project_dir = tmp.path().join("project-themes");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_dir).unwrap();

        write_theme(&user_dir, "ocean", &full_theme_toml("ocean", "User ocean"));
        write_theme(
            &project_dir,
            "ocean",
            &full_theme_toml("ocean", "Project ocean"),
        );

        let layers = vec![layer(&user_dir, None, 0), layer(&project_dir, None, 1)];
        let resources = discover_themes(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.description, "Project ocean");
        assert_eq!(resources[0].layer_precedence, 1);
    }

    #[test]
    fn lower_precedence_kept_when_no_collision() {
        let tmp = tempfile::tempdir().unwrap();

        let user_dir = tmp.path().join("user-themes");
        let project_dir = tmp.path().join("project-themes");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_dir).unwrap();

        write_theme(
            &user_dir,
            "user-only",
            &full_theme_toml("user-only", "User theme"),
        );
        write_theme(
            &project_dir,
            "project-only",
            &full_theme_toml("project-only", "Project theme"),
        );

        let layers = vec![layer(&user_dir, None, 0), layer(&project_dir, None, 1)];
        let resources = discover_themes(&layers).unwrap();
        assert_eq!(resources.len(), 2);
    }

    #[test]
    fn duplicate_name_in_same_layer_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(&themes_dir, "first", &full_theme_toml("shared", "First"));
        write_theme(&themes_dir, "second", &full_theme_toml("shared", "Second"));

        let err = discover_themes(&[layer(&themes_dir, None, 0)]).unwrap_err();
        assert!(matches!(
            err,
            ThemeDiscoveryError::DuplicateName { ref name, .. } if name == "shared"
        ));
    }
}

// ===========================================================================
// 6. Discovery errors
// ===========================================================================

mod discovery_errors {
    use super::*;

    #[test]
    fn discover_invalid_theme_toml_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(&themes_dir, "bad", "this is not valid toml [[[");

        let layers = vec![layer(&themes_dir, None, 0)];
        let result = discover_themes(&layers);
        assert!(result.is_err());
    }

    #[test]
    fn load_theme_with_invalid_color_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "bad-color",
            r#"
name = "bad-color"
description = "Has invalid color"

[colors]
role_user = "NotARealColor"
"#,
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        // Discovery succeeds (metadata is valid)
        let resources = discover_themes(&layers).unwrap();
        // Loading fails (color is invalid)
        let result = resources[0].load_theme();
        assert!(result.is_err());
    }

    #[test]
    fn load_theme_with_unknown_token_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "unknown-token",
            r#"
name = "unknown-token"
description = "Has unknown token"

[colors]
nonexistent_token = "Red"
"#,
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        // Discovery succeeds (metadata is valid)
        let resources = discover_themes(&layers).unwrap();
        // Loading fails (token is unknown)
        let result = resources[0].load_theme();
        assert!(result.is_err());
    }
}

// ===========================================================================
// 7. Progressive disclosure
// ===========================================================================

mod progressive_disclosure {
    use super::*;

    #[test]
    fn metadata_available_without_loading_colors() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "ocean",
            &full_theme_toml("ocean", "Ocean theme"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        let resource = &resources[0];

        // Metadata is available immediately
        assert_eq!(resource.manifest.name, "ocean");
        assert_eq!(resource.manifest.description, "Ocean theme");
    }

    #[test]
    fn load_theme_on_demand() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "ocean",
            &full_theme_toml("ocean", "Ocean theme"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        let resource = &resources[0];

        // Full theme loaded on demand
        let theme = resource.load_theme().unwrap();
        assert_eq!(theme.name, "ocean");
        assert_eq!(theme.role_user, ratatui::style::Color::Green);
    }

    #[test]
    fn partial_theme_fills_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "partial",
            &partial_theme_toml("partial", "Only two tokens"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        let resource = &resources[0];

        let theme = resource.load_theme().unwrap();
        assert_eq!(theme.name, "partial");
        // Specified tokens override
        assert_eq!(theme.role_user, ratatui::style::Color::Red);
        assert_eq!(theme.status_bg, ratatui::style::Color::Rgb(26, 26, 46));
        // Unspecified tokens inherit from default
        let default = Theme::default();
        assert_eq!(theme.role_assistant, default.role_assistant);
        assert_eq!(theme.status_idle, default.status_idle);
        assert_eq!(theme.diff_added, default.diff_added);
    }
}

// ===========================================================================
// 8. ThemeRegistry
// ===========================================================================

mod theme_registry {
    use super::*;

    fn setup_registry() -> (tempfile::TempDir, ThemeRegistry) {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();

        write_theme(
            &themes_dir,
            "alpha",
            &full_theme_toml("alpha", "First theme"),
        );
        write_theme(
            &themes_dir,
            "beta",
            &partial_theme_toml("beta", "Second theme"),
        );

        let layers = vec![layer(&themes_dir, None, 0)];
        let resources = discover_themes(&layers).unwrap();
        let registry = ThemeRegistry::from_resources(resources);
        (tmp, registry)
    }

    #[test]
    fn registry_names_returns_sorted() {
        let (_tmp, registry) = setup_registry();
        let names = registry.names();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn registry_get_returns_metadata() {
        let (_tmp, registry) = setup_registry();
        let resource = registry.get("alpha").unwrap();
        assert_eq!(resource.manifest.name, "alpha");
        assert_eq!(resource.manifest.description, "First theme");
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let (_tmp, registry) = setup_registry();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_load_theme() {
        let (_tmp, registry) = setup_registry();
        let theme = registry.load_theme("alpha").unwrap().unwrap();
        assert_eq!(theme.name, "alpha");
    }

    #[test]
    fn registry_resolve_theme_found() {
        let (_tmp, registry) = setup_registry();
        let theme = registry.resolve_theme("beta").unwrap();
        assert_eq!(theme.name, "beta");
    }

    #[test]
    fn registry_resolve_theme_falls_back_to_default() {
        let (_tmp, registry) = setup_registry();
        let theme = registry.resolve_theme("nonexistent").unwrap();
        assert_eq!(theme.name, "default");
    }

    #[test]
    fn registry_resolve_theme_built_in_monokai() {
        let (_tmp, registry) = setup_registry();
        let theme = registry.resolve_theme("monokai").unwrap();
        assert_eq!(theme.name, "monokai");
    }

    #[test]
    fn registry_format_for_prompt() {
        let (_tmp, registry) = setup_registry();
        let prompt = registry.format_for_prompt();
        assert!(prompt.contains("alpha"));
        assert!(prompt.contains("First theme"));
        assert!(prompt.contains("beta"));
        assert!(prompt.contains("Second theme"));
    }

    #[test]
    fn registry_empty_format_returns_empty_string() {
        let registry = ThemeRegistry::from_resources(vec![]);
        assert!(registry.format_for_prompt().is_empty());
    }
}
// ---------------------------------------------------------------------------
// Skills progressive discovery — merged from skills_discovery.rs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a skill directory with a SKILL.md file in `parent`.
fn write_skill(parent: &std::path::Path, dir_name: &str, frontmatter: &str, body: &str) {
    let dir = parent.join(dir_name);
    fs::create_dir_all(&dir).unwrap();
    let content = format!("---\n{frontmatter}\n---\n{body}");
    fs::write(dir.join("SKILL.md"), content).unwrap();
}

/// Build a single discovery layer at `root/subdirectory` with given precedence.
fn skill_layer(root: &std::path::Path, subdirectory: &str, precedence: u32) -> DiscoveryLayer {
    DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: root.to_path_buf(),
        subdirectory: Some(subdirectory.to_string()),
        precedence,
    }
}

// ---------------------------------------------------------------------------
// 1. Manifest parsing
// ---------------------------------------------------------------------------

#[test]
fn test_parse_valid_frontmatter() {
    let content = "---\nname: my-skill\ndescription: Does things.\n---\nBody here.";
    let manifest = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap();
    assert_eq!(manifest.name, "my-skill");
    assert_eq!(manifest.description, "Does things.");
    assert!(!manifest.disable_model_invocation);
}

#[test]
fn test_parse_with_disable_model_invocation() {
    let content =
        "---\nname: manual-only\ndescription: Manual only.\ndisable-model-invocation: true\n---\n";
    let manifest = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap();
    assert!(manifest.disable_model_invocation);
}

#[test]
fn test_parse_disable_model_invocation_false_explicit() {
    let content =
        "---\nname: auto-skill\ndescription: Auto.\ndisable-model-invocation: false\n---\n";
    let manifest = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap();
    assert!(!manifest.disable_model_invocation);
}

#[test]
fn test_parse_missing_name() {
    let content = "---\ndescription: No name.\n---\nBody";
    let err = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::MissingField { field, .. } => assert_eq!(field, "name"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_missing_description() {
    let content = "---\nname: no-desc\n---\nBody";
    let err = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::MissingField { field, .. } => assert_eq!(field, "description"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_empty_name() {
    let content = "---\nname: \"\"\ndescription: Empty name.\n---\n";
    let err = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::MissingField { field, .. } => assert_eq!(field, "name"),
        other => panic!("expected MissingField, got: {other}"),
    }
}

#[test]
fn test_parse_invalid_name_characters() {
    let content = "---\nname: Invalid Name!\ndescription: Bad chars.\n---\n";
    let err = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::InvalidName { .. } => {}
        other => panic!("expected InvalidName, got: {other}"),
    }
}

#[test]
fn test_parse_name_too_long() {
    let long_name = "a".repeat(65);
    let content = format!("---\nname: {long_name}\ndescription: Too long.\n---\n");
    let err = SkillManifest::from_skill_md(&content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::InvalidName { .. } => {}
        other => panic!("expected InvalidName, got: {other}"),
    }
}

#[test]
fn test_parse_name_at_max_length() {
    let name = "a".repeat(64);
    let content = format!("---\nname: {name}\ndescription: Max length ok.\n---\n");
    let manifest =
        SkillManifest::from_skill_md(&content, std::path::Path::new("SKILL.md")).unwrap();
    assert_eq!(manifest.name.len(), 64);
}

#[test]
fn test_parse_description_too_long() {
    let long_desc = "x".repeat(1025);
    let content = format!("---\nname: ok\ndescription: {long_desc}\n---\n");
    let err = SkillManifest::from_skill_md(&content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::InvalidDescription { .. } => {}
        other => panic!("expected InvalidDescription, got: {other}"),
    }
}

#[test]
fn test_parse_description_at_max_length() {
    let desc = "x".repeat(1024);
    let content = format!("---\nname: ok\ndescription: {desc}\n---\n");
    let manifest =
        SkillManifest::from_skill_md(&content, std::path::Path::new("SKILL.md")).unwrap();
    assert_eq!(manifest.description.len(), 1024);
}

#[test]
fn test_parse_no_frontmatter_delimiters() {
    let content = "Just some markdown content without frontmatter.";
    let err = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap_err();
    match err {
        SkillDiscoveryError::InvalidFrontmatter { .. } => {}
        other => panic!("expected InvalidFrontmatter, got: {other}"),
    }
}

#[test]
fn test_parse_valid_name_with_hyphens_and_digits() {
    let content = "---\nname: my-skill-v2\ndescription: Valid name.\n---\n";
    let manifest = SkillManifest::from_skill_md(content, std::path::Path::new("SKILL.md")).unwrap();
    assert_eq!(manifest.name, "my-skill-v2");
}

// ---------------------------------------------------------------------------
// 2. Discovery — basic
// ---------------------------------------------------------------------------

#[test]
fn test_discover_single_skill() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "my-skill",
        "name: my-skill\ndescription: A skill.",
        "Do the thing.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "my-skill");
    assert_eq!(resources[0].manifest.description, "A skill.");
}

#[test]
fn test_discover_multiple_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "alpha",
        "name: alpha\ndescription: First.",
        "A",
    );
    write_skill(&skills_dir, "beta", "name: beta\ndescription: Second.", "B");

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();

    // Results are sorted by name.
    assert_eq!(resources.len(), 2);
    assert_eq!(resources[0].manifest.name, "alpha");
    assert_eq!(resources[1].manifest.name, "beta");
}

#[test]
fn test_discover_skips_dirs_without_skill_md() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Create a directory without SKILL.md — should be skipped.
    fs::create_dir_all(skills_dir.join("no-skill")).unwrap();
    // Create a valid skill.
    write_skill(
        &skills_dir,
        "real-skill",
        "name: real-skill\ndescription: Real.",
        "R",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "real-skill");
}

#[test]
fn test_discover_skips_files() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();
    // A regular file in the scan dir should be skipped.
    fs::write(skills_dir.join("readme.txt"), "not a skill").unwrap();
    write_skill(
        &skills_dir,
        "valid",
        "name: valid\ndescription: Valid.",
        "V",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();

    assert_eq!(resources.len(), 1);
}

// ---------------------------------------------------------------------------
// 3. Discovery — precedence
// ---------------------------------------------------------------------------

#[test]
fn test_discover_precedence_higher_wins() {
    let tmp = tempfile::tempdir().unwrap();

    // User layer (precedence 0) has "tool-a" with description "user version"
    let user_dir = tmp.path().join("user-skills");
    fs::create_dir_all(&user_dir).unwrap();
    write_skill(
        &user_dir,
        "tool-a",
        "name: tool-a\ndescription: user version",
        "User body.",
    );

    // Project layer (precedence 1) has "tool-a" with description "project version"
    let proj_dir = tmp.path().join("proj-skills");
    fs::create_dir_all(&proj_dir).unwrap();
    write_skill(
        &proj_dir,
        "tool-a",
        "name: tool-a\ndescription: project version",
        "Project body.",
    );

    let layers = vec![
        skill_layer(tmp.path(), "user-skills", 0),
        skill_layer(tmp.path(), "proj-skills", 1),
    ];
    let resources = discover_skills(&layers).unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.description, "project version");
    assert_eq!(resources[0].layer_precedence, 1);
}

#[test]
fn test_discover_precedence_mixed_names() {
    let tmp = tempfile::tempdir().unwrap();

    // Layer 0: skill-a, skill-b
    let low = tmp.path().join("low");
    fs::create_dir_all(&low).unwrap();
    write_skill(&low, "skill-a", "name: skill-a\ndescription: low-a", "A");
    write_skill(&low, "skill-b", "name: skill-b\ndescription: low-b", "B");

    // Layer 1: skill-b (overrides), skill-c (new)
    let high = tmp.path().join("high");
    fs::create_dir_all(&high).unwrap();
    write_skill(&high, "skill-b", "name: skill-b\ndescription: high-b", "B2");
    write_skill(&high, "skill-c", "name: skill-c\ndescription: high-c", "C");

    let layers = vec![
        skill_layer(tmp.path(), "low", 0),
        skill_layer(tmp.path(), "high", 1),
    ];
    let resources = discover_skills(&layers).unwrap();

    // 3 unique names: skill-a from low, skill-b from high, skill-c from high.
    assert_eq!(resources.len(), 3);
    let names: Vec<&str> = resources.iter().map(|r| r.manifest.name.as_str()).collect();
    assert_eq!(names, vec!["skill-a", "skill-b", "skill-c"]);
    // skill-b is from high precedence layer.
    let skill_b = resources
        .iter()
        .find(|r| r.manifest.name == "skill-b")
        .unwrap();
    assert_eq!(skill_b.manifest.description, "high-b");
}

#[test]
fn test_discover_duplicate_name_same_layer_returns_error() {
    let tmp = tempfile::tempdir().unwrap();

    let dir = tmp.path().join("skills");
    fs::create_dir_all(&dir).unwrap();
    write_skill(
        &dir,
        "first",
        "name: shared-skill\ndescription: First",
        "First body.",
    );
    write_skill(
        &dir,
        "second",
        "name: shared-skill\ndescription: Second",
        "Second body.",
    );

    let err = discover_skills(&[skill_layer(tmp.path(), "skills", 0)]).unwrap_err();
    assert!(matches!(
        err,
        SkillDiscoveryError::DuplicateName { ref name, .. } if name == "shared-skill"
    ));
}

// ---------------------------------------------------------------------------
// 4. Discovery — missing / invalid resources
// ---------------------------------------------------------------------------

#[test]
fn test_discover_missing_scan_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let layers = vec![skill_layer(tmp.path(), "nonexistent", 0)];
    let resources = discover_skills(&layers).unwrap();
    assert!(resources.is_empty());
}

#[test]
fn test_discover_invalid_manifest_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    // Write a SKILL.md with invalid frontmatter.
    let bad_dir = skills_dir.join("broken");
    fs::create_dir_all(&bad_dir).unwrap();
    fs::write(bad_dir.join("SKILL.md"), "No frontmatter here.").unwrap();

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let err = discover_skills(&layers).unwrap_err();
    match err {
        SkillDiscoveryError::InvalidFrontmatter { .. } => {}
        other => panic!("expected InvalidFrontmatter, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Progressive disclosure
// ---------------------------------------------------------------------------

#[test]
fn test_progressive_disclosure_metadata_without_body() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "lazy",
        "name: lazy\ndescription: Load on demand.",
        "# Full Instructions\n\nDo the complex thing step by step.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();
    assert_eq!(resources.len(), 1);

    // Metadata is available.
    assert_eq!(resources[0].manifest.name, "lazy");
    assert_eq!(resources[0].manifest.description, "Load on demand.");

    // Body is NOT eagerly loaded into manifest.
    // The body can be loaded on demand.
    let body = resources[0].load_body().unwrap();
    assert!(body.contains("# Full Instructions"));
    assert!(body.contains("Do the complex thing step by step."));
}

#[test]
fn test_progressive_disclosure_load_body_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "ephemeral",
        "name: ephemeral\ndescription: Will be deleted.",
        "Original body.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();

    // Delete the SKILL.md after discovery.
    fs::remove_file(skills_dir.join("ephemeral").join("SKILL.md")).unwrap();

    // Loading body should fail gracefully.
    let result = resources[0].load_body();
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// 6. SkillRegistry
// ---------------------------------------------------------------------------

#[test]
fn test_registry_from_discovered_skills() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "summarize",
        "name: summarize\ndescription: Summarize text.",
        "Summarize.",
    );
    write_skill(
        &skills_dir,
        "translate",
        "name: translate\ndescription: Translate text.",
        "Translate.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();
    let registry = SkillRegistry::from_resources(resources);

    // List names — sorted alphabetically.
    let names = registry.names();
    assert_eq!(names, vec!["summarize", "translate"]);

    // Get metadata.
    let meta = registry.get("summarize").unwrap();
    assert_eq!(meta.manifest.description, "Summarize text.");
    assert!(!meta.manifest.disable_model_invocation);

    // Missing skill returns None.
    assert!(registry.get("nonexistent").is_none());
}

#[test]
fn test_registry_format_for_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "review",
        "name: review\ndescription: Review code.",
        "Review code for bugs.",
    );
    write_skill(
        &skills_dir,
        "manual",
        "name: manual\ndescription: Manual only.\ndisable-model-invocation: true",
        "For human use only.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();
    let registry = SkillRegistry::from_resources(resources);

    // Format for prompt should include all skill metadata.
    let prompt = registry.format_for_prompt();
    assert!(prompt.contains("review"));
    assert!(prompt.contains("Review code."));
    assert!(prompt.contains("manual"));
    assert!(prompt.contains("Manual only."));
}

#[test]
fn test_registry_disable_model_invocation_excluded_from_auto_listing() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "auto-skill",
        "name: auto-skill\ndescription: Auto invoked.",
        "Auto.",
    );
    write_skill(
        &skills_dir,
        "manual-skill",
        "name: manual-skill\ndescription: Manual only.\ndisable-model-invocation: true",
        "Manual.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();
    let registry = SkillRegistry::from_resources(resources);

    // auto_invocable() returns only skills without disable-model-invocation.
    let auto = registry.auto_invocable();
    let auto_names: Vec<&str> = auto.iter().map(|s| s.manifest.name.as_str()).collect();
    assert_eq!(auto_names, vec!["auto-skill"]);

    // all() still returns everything.
    assert_eq!(registry.names().len(), 2);
}

#[test]
fn test_registry_empty() {
    let registry = SkillRegistry::from_resources(vec![]);
    assert!(registry.names().is_empty());
    assert!(registry.format_for_prompt().is_empty());
}

// ---------------------------------------------------------------------------
// 7. Integration — load body through registry
// ---------------------------------------------------------------------------

#[test]
fn test_registry_load_body() {
    let tmp = tempfile::tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    fs::create_dir_all(&skills_dir).unwrap();

    write_skill(
        &skills_dir,
        "deep-skill",
        "name: deep-skill\ndescription: Has deep instructions.",
        "# Deep Skill\n\nStep 1: Analyze.\nStep 2: Execute.",
    );

    let layers = vec![skill_layer(tmp.path(), "skills", 0)];
    let resources = discover_skills(&layers).unwrap();
    let registry = SkillRegistry::from_resources(resources);

    let body = registry.load_body("deep-skill").unwrap().unwrap();
    assert!(body.contains("# Deep Skill"));
    assert!(body.contains("Step 1: Analyze."));
}

#[test]
fn test_registry_load_body_unknown_skill() {
    let registry = SkillRegistry::from_resources(vec![]);
    let result = registry.load_body("nonexistent");
    assert!(result.is_none());
}
// ---------------------------------------------------------------------------
// Extension resource discovery — merged from extension_resources.rs
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create an extension.toml manifest in the given directory.
fn write_manifest(dir: &std::path::Path, name: &str, version: &str, description: &str) {
    fs::create_dir_all(dir).unwrap();
    let content = format!(
        r#"[extension]
name = "{name}"
version = "{version}"
description = "{description}"
"#
    );
    fs::write(dir.join("extension.toml"), content).unwrap();
}

/// Create a minimal valid manifest with just a name.
fn write_minimal_manifest(dir: &std::path::Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    let content = format!(
        r#"[extension]
name = "{name}"
"#
    );
    fs::write(dir.join("extension.toml"), content).unwrap();
}

/// Write an invalid TOML file to the given path.
fn write_invalid_manifest(dir: &std::path::Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("extension.toml"), "not valid toml {{{{").unwrap();
}

/// Write a manifest with missing required name field.
fn write_manifest_missing_name(dir: &std::path::Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("extension.toml"),
        r#"[extension]
version = "1.0.0"
"#,
    )
    .unwrap();
}

#[cfg(unix)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

// ---------------------------------------------------------------------------
// 1. Basic discovery from single layers
// ---------------------------------------------------------------------------

#[test]
fn discover_from_project_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let project_ext_dir = tmp.path().join(".opi").join("extensions");
    write_manifest(
        &project_ext_dir.join("my-ext"),
        "my-ext",
        "1.0.0",
        "A test extension",
    );

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "my-ext");
    assert_eq!(resources[0].manifest.version.as_deref(), Some("1.0.0"));
    assert_eq!(
        resources[0].manifest.description.as_deref(),
        Some("A test extension")
    );
    assert_eq!(resources[0].layer_precedence, 0);
}

#[test]
fn discover_from_user_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let user_ext_dir = tmp.path().join("extensions");
    write_manifest(
        &user_ext_dir.join("user-ext"),
        "user-ext",
        "2.0.0",
        "User extension",
    );

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "user-ext");
}

#[test]
fn discover_from_explicit_path() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("my-extensions");
    write_manifest(
        &ext_dir.join("explicit-ext"),
        "explicit-ext",
        "1.0.0",
        "Explicit",
    );

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: ext_dir,
        subdirectory: None,
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "explicit-ext");
}

#[test]
fn discover_multiple_extensions_in_single_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join(".opi").join("extensions");
    write_manifest(&ext_dir.join("ext-a"), "ext-a", "1.0.0", "A");
    write_manifest(&ext_dir.join("ext-b"), "ext-b", "1.0.0", "B");
    write_manifest(&ext_dir.join("ext-c"), "ext-c", "1.0.0", "C");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 3);
    let names: Vec<&str> = resources.iter().map(|r| r.manifest.name.as_str()).collect();
    assert!(names.contains(&"ext-a"));
    assert!(names.contains(&"ext-b"));
    assert!(names.contains(&"ext-c"));
}

#[test]
fn duplicate_name_in_same_layer_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join(".opi").join("extensions");
    write_manifest(&ext_dir.join("first"), "shared", "1.0.0", "First");
    write_manifest(&ext_dir.join("second"), "shared", "1.0.0", "Second");

    let err = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap_err();

    assert!(matches!(
        err,
        ResourceDiscoveryError::DuplicateName { ref name, .. } if name == "shared"
    ));
}

#[test]
fn symlinked_extension_directory_is_canonicalized() {
    let tmp = tempfile::tempdir().unwrap();
    let scan_dir = tmp.path().join(".opi").join("extensions");
    fs::create_dir_all(&scan_dir).unwrap();

    let target_dir = tmp.path().join("external-target");
    write_manifest(&target_dir, "linked-ext", "1.0.0", "Linked");

    let link_dir = scan_dir.join("linked-ext");
    if let Err(err) = symlink_dir(&target_dir, &link_dir) {
        eprintln!("skipping symlink test; symlink creation failed: {err}");
        return;
    }

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "linked-ext");
    assert_eq!(resources[0].path, target_dir.canonicalize().unwrap());
}

// ---------------------------------------------------------------------------
// 2. Precedence model
// ---------------------------------------------------------------------------

#[test]
fn higher_precedence_overrides_lower() {
    let user_tmp = tempfile::tempdir().unwrap();
    let project_tmp = tempfile::tempdir().unwrap();

    // Same extension name in both user and project dirs.
    let user_ext_dir = user_tmp.path().join("extensions");
    write_manifest(
        &user_ext_dir.join("shared"),
        "shared",
        "1.0.0",
        "User version",
    );

    let proj_ext_dir = project_tmp.path().join(".opi").join("extensions");
    write_manifest(
        &proj_ext_dir.join("shared"),
        "shared",
        "2.0.0",
        "Project version",
    );

    let resources = discover_extension_resources(&[
        DiscoveryLayer {
            kind: DiscoveryLayerKind::User,
            root: user_tmp.path().to_path_buf(),
            subdirectory: Some("extensions".into()),
            precedence: 0, // lower
        },
        DiscoveryLayer {
            kind: DiscoveryLayerKind::Project,
            root: project_tmp.path().to_path_buf(),
            subdirectory: Some(".opi/extensions".into()),
            precedence: 1, // higher
        },
    ])
    .unwrap();

    // Should have exactly one entry (deduplicated by name).
    assert_eq!(resources.len(), 1);
    // Higher precedence wins, so we get the project version.
    assert_eq!(resources[0].manifest.version.as_deref(), Some("2.0.0"));
    assert_eq!(
        resources[0].manifest.description.as_deref(),
        Some("Project version")
    );
}

#[test]
fn explicit_path_has_highest_precedence() {
    let user_tmp = tempfile::tempdir().unwrap();
    let project_tmp = tempfile::tempdir().unwrap();
    let explicit_tmp = tempfile::tempdir().unwrap();

    // Same extension name across all three layers.
    let user_ext_dir = user_tmp.path().join("extensions");
    write_manifest(&user_ext_dir.join("shared"), "shared", "1.0.0", "User");

    let proj_ext_dir = project_tmp.path().join(".opi").join("extensions");
    write_manifest(&proj_ext_dir.join("shared"), "shared", "2.0.0", "Project");

    let explicit_dir = explicit_tmp.path().join("ext");
    write_manifest(&explicit_dir.join("shared"), "shared", "3.0.0", "Explicit");

    let resources = discover_extension_resources(&[
        DiscoveryLayer {
            kind: DiscoveryLayerKind::User,
            root: user_tmp.path().to_path_buf(),
            subdirectory: Some("extensions".into()),
            precedence: 0,
        },
        DiscoveryLayer {
            kind: DiscoveryLayerKind::Project,
            root: project_tmp.path().to_path_buf(),
            subdirectory: Some(".opi/extensions".into()),
            precedence: 1,
        },
        DiscoveryLayer {
            kind: DiscoveryLayerKind::Explicit,
            root: explicit_dir,
            subdirectory: None,
            precedence: 2,
        },
    ])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.version.as_deref(), Some("3.0.0"));
    assert_eq!(
        resources[0].manifest.description.as_deref(),
        Some("Explicit")
    );
}

// ---------------------------------------------------------------------------
// 3. Missing resources
// ---------------------------------------------------------------------------

#[test]
fn missing_directory_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let nonexistent = tmp.path().join("does-not-exist");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: nonexistent,
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert!(resources.is_empty());
}

#[test]
fn empty_directory_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    fs::create_dir_all(&ext_dir).unwrap();

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert!(resources.is_empty());
}

#[test]
fn directory_without_manifest_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join(".opi").join("extensions");
    // Create a directory without extension.toml
    fs::create_dir_all(ext_dir.join("no-manifest")).unwrap();
    // Create a valid one alongside
    write_manifest(&ext_dir.join("valid"), "valid", "1.0.0", "Valid");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "valid");
}

// ---------------------------------------------------------------------------
// 4. Invalid manifests
// ---------------------------------------------------------------------------

#[test]
fn invalid_toml_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    write_invalid_manifest(&ext_dir.join("bad-ext"));

    let result = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }]);

    assert!(result.is_err());
    match result.unwrap_err() {
        ResourceDiscoveryError::InvalidManifest { path, .. } => {
            assert!(path.to_string_lossy().contains("bad-ext"));
        }
        other => panic!("expected InvalidManifest, got: {other}"),
    }
}

#[test]
fn manifest_missing_name_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    write_manifest_missing_name(&ext_dir.join("nameless"));

    let result = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }]);

    assert!(result.is_err());
    match result.unwrap_err() {
        ResourceDiscoveryError::MissingField { field, path } => {
            assert_eq!(field, "name");
            assert!(path.to_string_lossy().contains("nameless"));
        }
        other => panic!("expected MissingField, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Path normalization
// ---------------------------------------------------------------------------

#[test]
fn paths_are_normalized_to_canonical() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join(".opi").join("extensions");
    write_manifest(&ext_dir.join("norm-ext"), "norm-ext", "1.0.0", "Normalized");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Project,
        root: tmp.path().to_path_buf(),
        subdirectory: Some(".opi/extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    // The path field should be the resolved directory path.
    assert!(resources[0].path.is_absolute());
}

#[test]
fn empty_name_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    fs::create_dir_all(ext_dir.join("empty-name")).unwrap();
    fs::write(
        ext_dir.join("empty-name").join("extension.toml"),
        r#"[extension]
name = ""
"#,
    )
    .unwrap();

    let result = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }]);

    assert!(result.is_err());
    match result.unwrap_err() {
        ResourceDiscoveryError::MissingField { field, .. } => {
            assert_eq!(field, "name");
        }
        other => panic!("expected MissingField, got: {other}"),
    }
}

// ---------------------------------------------------------------------------
// 6. Minimal manifest (only name required)
// ---------------------------------------------------------------------------

#[test]
fn minimal_manifest_with_only_name_is_valid() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    write_minimal_manifest(&ext_dir.join("minimal"), "minimal");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "minimal");
    assert!(resources[0].manifest.version.is_none());
    assert!(resources[0].manifest.description.is_none());
}

// ---------------------------------------------------------------------------
// 7. ExtensionResource structure
// ---------------------------------------------------------------------------

#[test]
fn resource_tracks_source_path_and_precedence() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("ext");
    write_manifest(&ext_dir.join("tracked"), "tracked", "1.0.0", "Tracked");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: ext_dir,
        subdirectory: None,
        precedence: 42,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert!(resources[0].path.ends_with("tracked"));
    assert_eq!(resources[0].layer_precedence, 42);
}

// ---------------------------------------------------------------------------
// 8. Integration with ExtensionManifest fields
// ---------------------------------------------------------------------------

#[test]
fn manifest_parses_all_optional_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("ext");
    fs::create_dir_all(ext_dir.join("full-ext")).unwrap();
    fs::write(
        ext_dir.join("full-ext").join("extension.toml"),
        r#"[extension]
name = "full-ext"
version = "2.3.1"
description = "A fully specified extension"
"#,
    )
    .unwrap();

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: ext_dir,
        subdirectory: None,
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    let m = &resources[0].manifest;
    assert_eq!(m.name, "full-ext");
    assert_eq!(m.version.as_deref(), Some("2.3.1"));
    assert_eq!(
        m.description.as_deref(),
        Some("A fully specified extension")
    );
}

// ---------------------------------------------------------------------------
// 9. No layers returns empty
// ---------------------------------------------------------------------------

#[test]
fn no_layers_returns_empty() {
    let resources = discover_extension_resources(&[]).unwrap();
    assert!(resources.is_empty());
}

// ---------------------------------------------------------------------------
// 10. Files (non-directories) in extension dir are skipped
// ---------------------------------------------------------------------------

#[test]
fn non_directory_entries_are_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let ext_dir = tmp.path().join("extensions");
    fs::create_dir_all(&ext_dir).unwrap();
    // A plain file, not a directory
    fs::write(ext_dir.join("readme.md"), "not an extension").unwrap();
    // A valid extension
    write_manifest(&ext_dir.join("real-ext"), "real-ext", "1.0.0", "Real");

    let resources = discover_extension_resources(&[DiscoveryLayer {
        kind: DiscoveryLayerKind::User,
        root: tmp.path().to_path_buf(),
        subdirectory: Some("extensions".into()),
        precedence: 0,
    }])
    .unwrap();

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].manifest.name, "real-ext");
}
