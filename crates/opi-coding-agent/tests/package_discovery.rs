//! Package progressive discovery behavioral tests (task 4.7.4).
//!
//! Covers: manifest parsing, resource composition, filtering, precedence,
//! disabled resources, duplicate package identity, missing assets, security
//! diagnostics, progressive disclosure, and PackageRegistry.

use std::path::Path;

use opi_coding_agent::package_discovery::{
    PackageDiscoveryError, PackageManifest, PackageRegistry, ResourceKind, discover_packages,
    package_composed_resource_layers,
};
use opi_coding_agent::prompt_fragment::discover_fragments;
use opi_coding_agent::resource::discover_extension_resources;
use opi_coding_agent::resource::{DiscoveryLayer, DiscoveryLayerKind};
use opi_coding_agent::skill::discover_skills;
use opi_coding_agent::theme_discovery::discover_themes;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn layer(root: &Path, subdirectory: Option<&str>, precedence: u32) -> DiscoveryLayer {
    DiscoveryLayer {
        kind: DiscoveryLayerKind::Explicit,
        root: root.to_path_buf(),
        subdirectory: subdirectory.map(String::from),
        precedence,
    }
}

/// Write a package.toml to a package directory.
fn write_package(dir: &Path, name: &str, toml_content: &str) -> std::path::PathBuf {
    let pkg_dir = dir.join(name);
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let path = pkg_dir.join("package.toml");
    std::fs::write(&path, toml_content).unwrap();
    pkg_dir
}

/// Minimal valid package.toml content.
fn minimal_pkg_toml(name: &str, description: &str) -> String {
    format!(
        r#"
name = "{name}"
description = "{description}"
"#
    )
}

/// Package.toml with version.
fn pkg_toml_with_version(name: &str, description: &str, version: &str) -> String {
    format!(
        r#"
name = "{name}"
description = "{description}"
version = "{version}"
"#
    )
}

/// Package.toml with explicit resource lists and disabled entries.
fn pkg_toml_with_filters(
    name: &str,
    description: &str,
    extensions: &[&str],
    skills: &[&str],
    fragments: &[&str],
    themes: &[&str],
    disabled: &[&str],
) -> String {
    let ext_list = if extensions.is_empty() {
        String::new()
    } else {
        format!(
            "extensions = [{}]",
            extensions
                .iter()
                .map(|e| format!("\"{e}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let skill_list = if skills.is_empty() {
        String::new()
    } else {
        format!(
            "skills = [{}]",
            skills
                .iter()
                .map(|s| format!("\"{s}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let frag_list = if fragments.is_empty() {
        String::new()
    } else {
        format!(
            "fragments = [{}]",
            fragments
                .iter()
                .map(|f| format!("\"{f}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let theme_list = if themes.is_empty() {
        String::new()
    } else {
        format!(
            "themes = [{}]",
            themes
                .iter()
                .map(|t| format!("\"{t}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let disabled_list = if disabled.is_empty() {
        String::new()
    } else {
        format!(
            "disabled = [{}]",
            disabled
                .iter()
                .map(|d| format!("\"{d}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    let mut lines = vec![
        format!(r#"name = "{name}""#),
        format!(r#"description = "{description}""#),
    ];
    if !ext_list.is_empty() {
        lines.push(ext_list);
    }
    if !skill_list.is_empty() {
        lines.push(skill_list);
    }
    if !frag_list.is_empty() {
        lines.push(frag_list);
    }
    if !theme_list.is_empty() {
        lines.push(theme_list);
    }
    if !disabled_list.is_empty() {
        lines.push(disabled_list);
    }
    lines.join("\n")
}

/// Create a resource directory with a marker file inside a package.
fn add_resource(pkg_dir: &Path, type_subdir: &str, name: &str, marker: &str) -> std::path::PathBuf {
    let dir = pkg_dir.join(type_subdir).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(marker), "").unwrap();
    dir
}

/// Create a full package with one of each resource type.
fn create_full_package(parent_dir: &Path, name: &str, description: &str) -> std::path::PathBuf {
    let pkg_dir = write_package(parent_dir, name, &minimal_pkg_toml(name, description));

    add_resource(&pkg_dir, "extensions", "my-ext", "extension.toml");
    add_resource(&pkg_dir, "skills", "my-skill", "SKILL.md");
    add_resource(&pkg_dir, "fragments", "my-frag", "FRAGMENT.md");
    add_resource(&pkg_dir, "themes", "my-theme", "theme.toml");

    pkg_dir
}

fn write_valid_package_resources(pkg_dir: &Path) {
    let ext_dir = pkg_dir.join("extensions").join("pkg-ext");
    std::fs::create_dir_all(&ext_dir).unwrap();
    std::fs::write(
        ext_dir.join("extension.toml"),
        r#"[extension]
name = "pkg-ext"
version = "1.0.0"
description = "Package extension"
"#,
    )
    .unwrap();

    let skill_dir = pkg_dir.join("skills").join("pkg-skill");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: pkg-skill
description: Package skill.
---
Full skill body.
"#,
    )
    .unwrap();

    let fragment_dir = pkg_dir.join("fragments").join("pkg-fragment");
    std::fs::create_dir_all(&fragment_dir).unwrap();
    std::fs::write(
        fragment_dir.join("FRAGMENT.md"),
        r#"---
name: pkg-fragment
description: Package fragment.
arguments: text
---
Full fragment body.
"#,
    )
    .unwrap();

    let theme_dir = pkg_dir.join("themes").join("pkg-theme");
    std::fs::create_dir_all(&theme_dir).unwrap();
    std::fs::write(
        theme_dir.join("theme.toml"),
        r#"
name = "pkg-theme"
description = "Package theme."
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

// ===========================================================================
// 1. Manifest parsing
// ===========================================================================

mod manifest_parsing {
    use super::*;

    #[test]
    fn parse_valid_minimal_manifest() {
        let toml = minimal_pkg_toml("my-pkg", "A test package.");
        let path = Path::new("my-pkg/package.toml");
        let manifest = PackageManifest::from_toml(&toml, path).unwrap();
        assert_eq!(manifest.name, "my-pkg");
        assert_eq!(manifest.description, "A test package.");
        assert!(manifest.version.is_none());
        assert!(manifest.extensions.is_none());
        assert!(manifest.skills.is_none());
        assert!(manifest.fragments.is_none());
        assert!(manifest.themes.is_none());
        assert!(manifest.disabled.is_empty());
    }

    #[test]
    fn parse_manifest_with_version() {
        let toml = pkg_toml_with_version("versioned", "Has a version.", "2.1.0");
        let path = Path::new("versioned/package.toml");
        let manifest = PackageManifest::from_toml(&toml, path).unwrap();
        assert_eq!(manifest.version.as_deref(), Some("2.1.0"));
    }

    #[test]
    fn parse_manifest_with_resource_lists() {
        let toml = pkg_toml_with_filters(
            "filtered",
            "Has filters.",
            &["ext-a"],
            &["skill-b"],
            &["frag-c"],
            &["theme-d"],
            &[],
        );
        let path = Path::new("filtered/package.toml");
        let manifest = PackageManifest::from_toml(&toml, path).unwrap();
        assert_eq!(
            manifest.extensions.as_deref(),
            Some(&["ext-a".to_string()][..])
        );
        assert_eq!(
            manifest.skills.as_deref(),
            Some(&["skill-b".to_string()][..])
        );
        assert_eq!(
            manifest.fragments.as_deref(),
            Some(&["frag-c".to_string()][..])
        );
        assert_eq!(
            manifest.themes.as_deref(),
            Some(&["theme-d".to_string()][..])
        );
    }

    #[test]
    fn parse_manifest_with_disabled() {
        let toml = pkg_toml_with_filters(
            "with-disabled",
            "Has disabled.",
            &[],
            &[],
            &[],
            &[],
            &["old-skill", "deprecated-theme"],
        );
        let path = Path::new("with-disabled/package.toml");
        let manifest = PackageManifest::from_toml(&toml, path).unwrap();
        assert_eq!(
            manifest.disabled,
            vec!["old-skill".to_string(), "deprecated-theme".to_string()]
        );
    }

    #[test]
    fn parse_manifest_missing_name() {
        let toml = r#"description = "No name.""#;
        let path = Path::new("x/package.toml");
        let err = PackageManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(
            err,
            PackageDiscoveryError::MissingField { ref field, .. } if field == "name"
        ));
    }

    #[test]
    fn parse_manifest_missing_description() {
        let toml = r#"name = "no-desc""#;
        let path = Path::new("x/package.toml");
        let err = PackageManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(
            err,
            PackageDiscoveryError::MissingField { ref field, .. } if field == "description"
        ));
    }

    #[test]
    fn parse_manifest_invalid_name() {
        let toml = minimal_pkg_toml("Bad Name!", "Invalid chars.");
        let path = Path::new("x/package.toml");
        let err = PackageManifest::from_toml(&toml, path).unwrap_err();
        assert!(matches!(err, PackageDiscoveryError::InvalidName { .. }));
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
        let path = Path::new("x/package.toml");
        let err = PackageManifest::from_toml(&toml, path).unwrap_err();
        assert!(matches!(
            err,
            PackageDiscoveryError::InvalidDescription { .. }
        ));
    }

    #[test]
    fn parse_manifest_invalid_toml() {
        let toml = "this is not valid toml [[[";
        let path = Path::new("x/package.toml");
        let err = PackageManifest::from_toml(toml, path).unwrap_err();
        assert!(matches!(err, PackageDiscoveryError::InvalidManifest { .. }));
    }

    #[test]
    fn example_package_manifests_parse_with_supported_schema() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.join("../..");
        let examples = [
            "examples/sub-agent/package.toml",
            "examples/plan-mode/package.toml",
            "examples/todo/package.toml",
            "examples/mcp-adapter/package.toml",
        ];

        for relative in examples {
            let path = repo_root.join(relative);
            let toml = std::fs::read_to_string(&path).expect("example package should be readable");
            PackageManifest::from_toml(&toml, &path)
                .unwrap_or_else(|err| panic!("{relative} should parse: {err}"));
        }
    }
}

// ===========================================================================
// 2. Discovery basic
// ===========================================================================

mod discovery_basic {
    use super::*;

    #[test]
    fn discover_from_single_layer() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = tmp.path().join("packages");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        write_package(&pkg_dir, "my-pkg", &minimal_pkg_toml("my-pkg", "A package"));

        let layers = vec![layer(&pkg_dir, None, 0)];
        let resources = discover_packages(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.name, "my-pkg");
    }

    #[test]
    fn discover_multiple_packages() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = tmp.path().join("packages");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        write_package(&pkg_dir, "alpha", &minimal_pkg_toml("alpha", "A"));
        write_package(&pkg_dir, "beta", &minimal_pkg_toml("beta", "B"));

        let layers = vec![layer(&pkg_dir, None, 0)];
        let resources = discover_packages(&layers).unwrap();
        assert_eq!(resources.len(), 2);
        assert_eq!(resources[0].manifest.name, "alpha");
        assert_eq!(resources[1].manifest.name, "beta");
    }

    #[test]
    fn discover_skips_non_package_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = tmp.path().join("packages");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        // A directory without package.toml
        let other = pkg_dir.join("not-a-package");
        std::fs::create_dir_all(&other).unwrap();

        // A file at top level
        std::fs::write(pkg_dir.join("readme.txt"), "not a package").unwrap();

        write_package(&pkg_dir, "real-pkg", &minimal_pkg_toml("real-pkg", "Real"));

        let layers = vec![layer(&pkg_dir, None, 0)];
        let resources = discover_packages(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.name, "real-pkg");
    }

    #[test]
    fn discover_missing_scan_dir_returns_empty() {
        let layers = vec![layer(Path::new("/nonexistent/path"), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        assert!(resources.is_empty());
    }
}

// ===========================================================================
// 3. Discovery precedence
// ===========================================================================

mod discovery_precedence {
    use super::*;

    #[test]
    fn higher_precedence_wins_on_name_collision() {
        let tmp = tempfile::tempdir().unwrap();

        let user_dir = tmp.path().join("user-packages");
        let project_dir = tmp.path().join("project-packages");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_dir).unwrap();

        write_package(
            &user_dir,
            "my-pkg",
            &minimal_pkg_toml("my-pkg", "User version"),
        );
        write_package(
            &project_dir,
            "my-pkg",
            &minimal_pkg_toml("my-pkg", "Project version"),
        );

        let layers = vec![layer(&user_dir, None, 0), layer(&project_dir, None, 1)];
        let resources = discover_packages(&layers).unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.description, "Project version");
        assert_eq!(resources[0].layer_precedence, 1);
    }

    #[test]
    fn lower_precedence_kept_when_no_collision() {
        let tmp = tempfile::tempdir().unwrap();

        let user_dir = tmp.path().join("user-packages");
        let project_dir = tmp.path().join("project-packages");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_dir).unwrap();

        write_package(
            &user_dir,
            "user-only",
            &minimal_pkg_toml("user-only", "User package"),
        );
        write_package(
            &project_dir,
            "project-only",
            &minimal_pkg_toml("project-only", "Project package"),
        );

        let layers = vec![layer(&user_dir, None, 0), layer(&project_dir, None, 1)];
        let resources = discover_packages(&layers).unwrap();
        assert_eq!(resources.len(), 2);
    }
}

// ===========================================================================
// 4. Resource composition
// ===========================================================================

mod resource_composition {
    use super::*;

    #[test]
    fn compose_finds_all_resource_types() {
        let tmp = tempfile::tempdir().unwrap();
        let _pkg_dir = create_full_package(tmp.path(), "full-pkg", "Has everything");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        assert_eq!(composed.len(), 4);
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Extension && r.name == "my-ext")
        );
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Skill && r.name == "my-skill")
        );
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Fragment && r.name == "my-frag")
        );
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Theme && r.name == "my-theme")
        );
    }

    #[test]
    fn compose_with_no_resources() {
        let tmp = tempfile::tempdir().unwrap();
        write_package(
            tmp.path(),
            "empty-pkg",
            &minimal_pkg_toml("empty-pkg", "No resources"),
        );

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();
        assert!(composed.is_empty());
    }

    #[test]
    fn compose_with_include_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "filtered",
            &pkg_toml_with_filters("filtered", "Filtered", &[], &["skill-a"], &[], &[], &[]),
        );

        // Create two skills but only one is in the include list
        add_resource(&pkg_dir, "skills", "skill-a", "SKILL.md");
        add_resource(&pkg_dir, "skills", "skill-b", "SKILL.md");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        // Only skill-a should be included
        let skills: Vec<_> = composed
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "skill-a");
    }

    #[test]
    fn compose_auto_discovers_when_no_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "auto",
            &minimal_pkg_toml("auto", "Auto-discover"),
        );

        add_resource(&pkg_dir, "skills", "skill-a", "SKILL.md");
        add_resource(&pkg_dir, "skills", "skill-b", "SKILL.md");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        let skills: Vec<_> = composed
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .collect();
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn compose_skips_directories_without_marker_files() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(tmp.path(), "sparse", &minimal_pkg_toml("sparse", "Sparse"));

        // Valid skill
        add_resource(&pkg_dir, "skills", "real-skill", "SKILL.md");

        // Directory without SKILL.md (should be skipped in auto mode)
        let fake_dir = pkg_dir.join("skills").join("fake-skill");
        std::fs::create_dir_all(&fake_dir).unwrap();
        std::fs::write(fake_dir.join("README.md"), "not a skill").unwrap();

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        let skills: Vec<_> = composed
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "real-skill");
    }

    #[test]
    fn compose_with_multiple_resource_types_filtered() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "multi-filter",
            &pkg_toml_with_filters(
                "multi-filter",
                "Multiple filters",
                &["ext-1"],
                &["skill-1"],
                &["frag-1"],
                &["theme-1"],
                &[],
            ),
        );

        add_resource(&pkg_dir, "extensions", "ext-1", "extension.toml");
        add_resource(&pkg_dir, "extensions", "ext-2", "extension.toml");
        add_resource(&pkg_dir, "skills", "skill-1", "SKILL.md");
        add_resource(&pkg_dir, "skills", "skill-2", "SKILL.md");
        add_resource(&pkg_dir, "fragments", "frag-1", "FRAGMENT.md");
        add_resource(&pkg_dir, "themes", "theme-1", "theme.toml");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        // Only listed resources should be included
        assert_eq!(composed.len(), 4);
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Extension && r.name == "ext-1")
        );
        assert!(!composed.iter().any(|r| r.name == "ext-2"));
        assert!(
            composed
                .iter()
                .any(|r| r.kind == ResourceKind::Skill && r.name == "skill-1")
        );
        assert!(!composed.iter().any(|r| r.name == "skill-2"));
    }

    #[test]
    fn composed_package_layers_make_resources_discoverable_by_kind() {
        let tmp = tempfile::tempdir().unwrap();
        let package_root = tmp.path().join("packages");
        std::fs::create_dir_all(&package_root).unwrap();
        let pkg_dir = write_package(
            &package_root,
            "resource-suite",
            &minimal_pkg_toml("resource-suite", "Resource suite"),
        );
        write_valid_package_resources(&pkg_dir);

        let packages = discover_packages(&[layer(&package_root, None, 0)]).unwrap();
        let package_layers = package_composed_resource_layers(&packages);

        assert!(package_layers.diagnostics.is_empty());

        let extensions = discover_extension_resources(&package_layers.extensions).unwrap();
        let skills = discover_skills(&package_layers.skills).unwrap();
        let fragments = discover_fragments(&package_layers.fragments).unwrap();
        let themes = discover_themes(&package_layers.themes).unwrap();

        assert_eq!(extensions[0].manifest.name, "pkg-ext");
        assert_eq!(skills[0].manifest.name, "pkg-skill");
        assert_eq!(fragments[0].manifest.name, "pkg-fragment");
        assert_eq!(themes[0].manifest.name, "pkg-theme");
    }

    #[test]
    fn composed_package_layers_do_not_outprioritize_explicit_direct_layers() {
        let tmp = tempfile::tempdir().unwrap();
        let package_root = tmp.path().join("user-packages");
        let explicit_root = tmp.path().join("explicit-extensions");
        std::fs::create_dir_all(&package_root).unwrap();
        std::fs::create_dir_all(&explicit_root).unwrap();

        let pkg_dir = write_package(
            &package_root,
            "resource-suite",
            &minimal_pkg_toml("resource-suite", "Resource suite"),
        );
        let package_ext = add_resource(&pkg_dir, "extensions", "shared-ext", "extension.toml");
        std::fs::write(
            package_ext.join("extension.toml"),
            r#"
[extension]
name = "shared-ext"
description = "From package"
"#,
        )
        .unwrap();

        let explicit_ext = explicit_root.join("shared-ext");
        std::fs::create_dir_all(&explicit_ext).unwrap();
        std::fs::write(
            explicit_ext.join("extension.toml"),
            r#"
[extension]
name = "shared-ext"
description = "From explicit direct layer"
"#,
        )
        .unwrap();

        let packages = discover_packages(&[layer(&package_root, None, 0)]).unwrap();
        let package_layers = package_composed_resource_layers(&packages);

        let mut extension_layers = vec![layer(&explicit_root, None, 2)];
        extension_layers.extend(package_layers.extensions);
        let extensions = discover_extension_resources(&extension_layers).unwrap();

        assert_eq!(extensions.len(), 1);
        assert_eq!(
            extensions[0].manifest.description.as_deref(),
            Some("From explicit direct layer")
        );
        assert_eq!(extensions[0].layer_precedence, 2);
    }
}

// ===========================================================================
// 5. Disabled resources
// ===========================================================================

mod disabled_resources {
    use super::*;

    #[test]
    fn disabled_resources_excluded_in_auto_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "disabled-auto",
            &pkg_toml_with_filters(
                "disabled-auto",
                "Disabled auto",
                &[],
                &[],
                &[],
                &[],
                &["old-skill"],
            ),
        );

        add_resource(&pkg_dir, "skills", "new-skill", "SKILL.md");
        add_resource(&pkg_dir, "skills", "old-skill", "SKILL.md");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        let skills: Vec<_> = composed
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "new-skill");
    }

    #[test]
    fn disabled_overrides_include_list() {
        let tmp = tempfile::tempdir().unwrap();
        // Include lists "both" but disabled also lists "both"
        let pkg_dir = write_package(
            tmp.path(),
            "disabled-override",
            &pkg_toml_with_filters(
                "disabled-override",
                "Disabled override",
                &[],
                &["kept", "both"],
                &[],
                &[],
                &["both"],
            ),
        );

        add_resource(&pkg_dir, "skills", "kept", "SKILL.md");
        add_resource(&pkg_dir, "skills", "both", "SKILL.md");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        let skills: Vec<_> = composed
            .iter()
            .filter(|r| r.kind == ResourceKind::Skill)
            .collect();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "kept");
    }
}

// ===========================================================================
// 6. Duplicate package identity
// ===========================================================================

mod duplicate_identity {
    use super::*;

    #[test]
    fn same_package_deduped_across_layers() {
        let tmp = tempfile::tempdir().unwrap();

        let user_dir = tmp.path().join("user");
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::create_dir_all(&project_dir).unwrap();

        write_package(
            &user_dir,
            "shared",
            &minimal_pkg_toml("shared", "User version"),
        );
        write_package(
            &project_dir,
            "shared",
            &minimal_pkg_toml("shared", "Project version"),
        );

        let layers = vec![layer(&user_dir, None, 0), layer(&project_dir, None, 1)];
        let resources = discover_packages(&layers).unwrap();

        // Only one "shared" package, higher precedence wins
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].manifest.description, "Project version");
    }

    #[test]
    fn duplicate_name_in_same_layer_returns_error() {
        let tmp = tempfile::tempdir().unwrap();

        write_package(
            tmp.path(),
            "first",
            &minimal_pkg_toml("shared", "First version"),
        );
        write_package(
            tmp.path(),
            "second",
            &minimal_pkg_toml("shared", "Second version"),
        );

        let err = discover_packages(&[layer(tmp.path(), None, 0)]).unwrap_err();

        assert!(matches!(
            err,
            PackageDiscoveryError::DuplicateName { ref name, .. } if name == "shared"
        ));
    }
}

// ===========================================================================
// 7. Missing assets
// ===========================================================================

mod missing_assets {
    use super::*;

    #[test]
    fn missing_resource_in_include_list() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "missing",
            &pkg_toml_with_filters("missing", "Missing", &[], &["nonexistent"], &[], &[], &[]),
        );

        // Create the skills dir but not the resource
        std::fs::create_dir_all(pkg_dir.join("skills")).unwrap();

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let result = resources[0].compose();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            PackageDiscoveryError::MissingAsset { ref name, .. } if name == "nonexistent"
        ));
    }

    #[test]
    fn missing_subdirectory_with_include_list() {
        let tmp = tempfile::tempdir().unwrap();
        // Include list references a skill but no skills/ dir exists
        write_package(
            tmp.path(),
            "no-dir",
            &pkg_toml_with_filters("no-dir", "No dir", &[], &["ghost-skill"], &[], &[], &[]),
        );

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let result = resources[0].compose();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PackageDiscoveryError::MissingAsset { .. }
        ));
    }

    #[test]
    fn missing_marker_file_in_include_list() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "no-marker",
            &pkg_toml_with_filters("no-marker", "No marker", &[], &["bad-skill"], &[], &[], &[]),
        );

        // Create directory but without SKILL.md
        let skill_dir = pkg_dir.join("skills").join("bad-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let result = resources[0].compose();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PackageDiscoveryError::MissingAsset { .. }
        ));
    }
}

// ===========================================================================
// 8. Security diagnostics
// ===========================================================================

mod security_diagnostics {
    use super::*;

    #[test]
    fn security_error_variant_exists_and_documented() {
        // Verify the SecurityDiagnostic error variant can be constructed
        // and has the expected fields. The actual path traversal check
        // uses canonicalize() + starts_with(), which is verified in
        // compose_type() at runtime.
        let err = PackageDiscoveryError::SecurityDiagnostic {
            package_name: "test-pkg".into(),
            path: std::path::PathBuf::from("/evil/path"),
            reason: "resource path escapes package directory".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("security"));
        assert!(msg.contains("test-pkg"));
        assert!(msg.contains("evil"));
    }

    #[test]
    fn compose_validates_paths_within_package() {
        // Normal resources within the package directory should work fine.
        // This tests the happy path: all paths are within the package.
        let tmp = tempfile::tempdir().unwrap();
        let _pkg_dir = create_full_package(tmp.path(), "safe-pkg", "Safe package");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let composed = resources[0].compose().unwrap();

        // All 4 resources should pass the security check
        assert_eq!(composed.len(), 4);
    }

    #[test]
    fn compose_rejects_symlinked_resource_escaping_package() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = write_package(
            tmp.path(),
            "escape-pkg",
            &pkg_toml_with_filters(
                "escape-pkg",
                "Escaping resource",
                &["linked-ext"],
                &[],
                &[],
                &[],
                &[],
            ),
        );

        let external_dir = tmp.path().join("outside-extension");
        std::fs::create_dir_all(&external_dir).unwrap();
        std::fs::write(external_dir.join("extension.toml"), "").unwrap();

        let ext_dir = pkg_dir.join("extensions");
        std::fs::create_dir_all(&ext_dir).unwrap();
        let link_dir = ext_dir.join("linked-ext");
        if let Err(err) = symlink_dir(&external_dir, &link_dir) {
            eprintln!("skipping symlink test; symlink creation failed: {err}");
            return;
        }

        let resources = discover_packages(&[layer(tmp.path(), None, 0)]).unwrap();
        let err = resources[0].compose().unwrap_err();

        assert!(matches!(
            err,
            PackageDiscoveryError::SecurityDiagnostic { ref package_name, .. }
                if package_name == "escape-pkg"
        ));
    }
}

// ===========================================================================
// 9. Progressive disclosure
// ===========================================================================

mod progressive_disclosure {
    use super::*;

    #[test]
    fn metadata_available_without_composing() {
        let tmp = tempfile::tempdir().unwrap();
        let _pkg_dir = write_package(
            tmp.path(),
            "lazy",
            &minimal_pkg_toml("lazy", "Lazy package"),
        );

        // Don't create any resource subdirectories

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();

        // Metadata is available immediately
        assert_eq!(resources[0].manifest.name, "lazy");
        assert_eq!(resources[0].manifest.description, "Lazy package");
        assert!(resources[0].path.is_dir());
    }

    #[test]
    fn compose_on_demand() {
        let tmp = tempfile::tempdir().unwrap();
        let _pkg_dir = create_full_package(tmp.path(), "ondemand", "On demand");

        let layers = vec![layer(tmp.path(), None, 0)];
        let resources = discover_packages(&layers).unwrap();

        // Composition happens only when explicitly requested
        let composed = resources[0].compose().unwrap();
        assert_eq!(composed.len(), 4);
    }
}

// ===========================================================================
// 10. PackageRegistry
// ===========================================================================

mod package_registry {
    use super::*;

    fn setup_registry() -> (tempfile::TempDir, PackageRegistry) {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_dir = tmp.path().join("packages");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        write_package(
            &pkg_dir,
            "alpha",
            &minimal_pkg_toml("alpha", "First package"),
        );
        write_package(
            &pkg_dir,
            "beta",
            &pkg_toml_with_version("beta", "Second package", "1.2.0"),
        );

        let layers = vec![layer(&pkg_dir, None, 0)];
        let resources = discover_packages(&layers).unwrap();
        let registry = PackageRegistry::from_resources(resources);
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
        let pkg = registry.get("alpha").unwrap();
        assert_eq!(pkg.manifest.name, "alpha");
        assert_eq!(pkg.manifest.description, "First package");
    }

    #[test]
    fn registry_get_missing_returns_none() {
        let (_tmp, registry) = setup_registry();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_format_for_prompt() {
        let (_tmp, registry) = setup_registry();
        let prompt = registry.format_for_prompt();
        assert!(prompt.contains("alpha"));
        assert!(prompt.contains("First package"));
        assert!(prompt.contains("beta"));
        assert!(prompt.contains("Second package"));
        assert!(prompt.contains("v1.2.0"));
    }

    #[test]
    fn registry_empty_format_returns_empty_string() {
        let registry = PackageRegistry::from_resources(vec![]);
        assert!(registry.format_for_prompt().is_empty());
    }
}
