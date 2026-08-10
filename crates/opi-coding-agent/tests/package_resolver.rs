use std::fs;

use opi_coding_agent::package_resolver::{
    InstalledPackageScope, PackageDiagnosticSeverity, manifest_sha256, resolve_installed_packages,
    source_identity_for_resolution,
};
use opi_coding_agent::package_store::{
    PackageDeclaration, PackageLockEntry, PackageSource, PackageStore,
};
use tempfile::tempdir;

fn write_package(root: &std::path::Path, name: &str) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("package.toml"),
        format!(
            "name = \"{name}\"\n\
             description = \"{name} package\"\n\
             version = \"0.1.0\"\n"
        ),
    )
    .unwrap();
}

fn write_package_with_opi_version(root: &std::path::Path, name: &str, opi_version: &str) {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("package.toml"),
        format!(
            "name = \"{name}\"\n\
             description = \"{name} package\"\n\
             version = \"0.1.0\"\n\
             opi_version = \"{opi_version}\"\n"
        ),
    )
    .unwrap();
}

#[test]
fn manifest_sha256_returns_lowercase_hex_sha256_digest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("input.txt");
    fs::write(&path, "hello world").unwrap();

    let hash = manifest_sha256(&path).unwrap();

    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn manifest_sha256_distinguishes_exact_line_endings() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("package.toml");
    fs::write(&path, b"name = \"demo\"\n").unwrap();
    let lf = manifest_sha256(&path).unwrap();
    fs::write(&path, b"name = \"demo\"\r\n").unwrap();
    let crlf = manifest_sha256(&path).unwrap();

    assert_ne!(lf, crlf);
}

#[test]
fn source_identity_for_resolution_canonicalizes_local_paths_against_base() {
    let workspace = tempdir().unwrap();
    let package_root = workspace.path().join("vendor").join("todo");
    write_package(&package_root, "todo");
    let source = PackageSource::parse("./vendor/../vendor/todo").unwrap();

    let identity = source_identity_for_resolution(&source, workspace.path()).unwrap();

    assert_eq!(identity.kind, "local");
    assert_eq!(
        identity.value,
        package_root.canonicalize().unwrap().display().to_string()
    );
}

#[test]
fn resolver_reads_project_package_declaration_as_package_resource() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/todo");
    write_package(&package_root, "todo");

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: ".\\vendor\\todo".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[PackageLockEntry {
            identity_kind: "local".to_string(),
            identity_value: package_root.canonicalize().unwrap().display().to_string(),
            source: ".\\vendor\\todo".to_string(),
            package_root: package_root.canonicalize().unwrap(),
            cache_path: None,
            git_commit: None,
            manifest_sha256: opi_coding_agent::package_resolver::manifest_sha256(
                &package_root.join("package.toml"),
            )
            .unwrap(),
            contributions: Vec::new(),
        }])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.diagnostics, []);
    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.packages[0].scope, InstalledPackageScope::Project);
    assert_eq!(result.packages[0].package.manifest.name, "todo");
}

#[test]
fn resolver_reports_missing_local_package_as_error() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./missing".to_string(),
            filters: Default::default(),
        }])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].severity,
        PackageDiagnosticSeverity::Error
    );
    assert_eq!(result.diagnostics[0].code, "source_missing");
}

#[test]
fn resolver_rejects_non_local_lock_entry_for_local_package_source() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/todo");
    write_package(&package_root, "todo");

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/todo".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[PackageLockEntry {
            identity_kind: "git".to_string(),
            identity_value: "https://example.com/todo.git".to_string(),
            source: "./vendor/todo".to_string(),
            package_root: package_root.canonicalize().unwrap(),
            cache_path: None,
            git_commit: Some("abc123".to_string()),
            manifest_sha256: manifest_sha256(&package_root.join("package.toml")).unwrap(),
            contributions: Vec::new(),
        }])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "lock_missing");
}

#[test]
fn resolver_rejects_git_lock_source_that_does_not_match_ref_pinned_declaration() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = user.path().join("cache").join("repo");
    write_package(&package_root, "gitpkg");

    let store = PackageStore::global(user.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "git:https://example.com/repo.git@sha1".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[PackageLockEntry {
            identity_kind: "git".to_string(),
            identity_value: "https://example.com/repo.git".to_string(),
            source: "git:https://example.com/repo.git@sha2".to_string(),
            package_root: package_root.canonicalize().unwrap(),
            cache_path: Some(package_root.canonicalize().unwrap()),
            git_commit: Some("sha2".to_string()),
            manifest_sha256: manifest_sha256(&package_root.join("package.toml")).unwrap(),
            contributions: Vec::new(),
        }])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "lock_source_mismatch");
}

#[test]
fn resolver_reports_missing_manifest_declared_resource_asset() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/todo");
    fs::create_dir_all(&package_root).unwrap();
    fs::write(
        package_root.join("package.toml"),
        "name = \"todo\"\n\
         description = \"todo package\"\n\
         version = \"0.1.0\"\n\
         skills = [\"review\"]\n",
    )
    .unwrap();

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/todo".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            "./vendor/todo".to_string(),
            &package_root,
        )
        .unwrap()])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "composition_failed");
    assert!(
        result.diagnostics[0]
            .message
            .contains("missing skill 'review'"),
        "diagnostic should identify missing declared skill asset: {:?}",
        result.diagnostics[0]
    );
}

#[test]
fn resolver_reports_incompatible_opi_version_as_warning() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/versioned");
    write_package_with_opi_version(&package_root, "versioned", ">=99.0");

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/versioned".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            "./vendor/versioned".to_string(),
            &package_root,
        )
        .unwrap()])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].severity,
        PackageDiagnosticSeverity::Warning
    );
    assert_eq!(result.diagnostics[0].code, "opi_version_incompatible");
    assert!(
        result.diagnostics[0]
            .message
            .contains("incompatible opi version"),
        "diagnostic should identify opi_version incompatibility: {:?}",
        result.diagnostics[0]
    );
}

#[test]
fn resolver_prefers_project_package_over_global_package_with_same_manifest_name() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let global_root = user.path().join("global-todo");
    let project_root = workspace.path().join("project-todo");
    write_package(&global_root, "todo");
    write_package(&project_root, "todo");

    let global = PackageStore::global(user.path().to_path_buf());
    global
        .write_declarations(&[PackageDeclaration {
            source: global_root.display().to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    global
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            global_root.display().to_string(),
            &global_root,
        )
        .unwrap()])
        .unwrap();

    let project = PackageStore::project(workspace.path().to_path_buf());
    project
        .write_declarations(&[PackageDeclaration {
            source: project_root.display().to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    project
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            project_root.display().to_string(),
            &project_root,
        )
        .unwrap()])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.packages[0].scope, InstalledPackageScope::Project);
    assert_eq!(
        result.packages[0].package.path,
        project_root.canonicalize().unwrap()
    );
}

// ---------------------------------------------------------------------------
// Phase 6 (task 6.4) degraded-path diagnostic contract.
//
// The design Error Handling section requires degraded diagnostics to identify
// package source, package scope, manifest name when available, adapter command
// when relevant, expected versus actual values when negotiation/drift fails,
// the timeout surface when a timeout occurs, and whether the package is
// disabled at runtime or only degraded. Source and scope are already carried as
// PackageDiagnostic struct fields by `diagnostic()`; these tests pin the message
// content for the missing-field, lock-drift, missing-lock, and duplicate-name
// degraded paths through the production `resolve_installed_packages` entry
// point. The manifest name is not available for pre-parse failures
// (lock_missing / lock_drifted / invalid manifest), which matches the design's
// "manifest name when available" qualifier.
// ---------------------------------------------------------------------------

#[test]
fn resolver_reports_missing_lock_entry_with_disabled_runtime_state() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/nolock");
    write_package(&package_root, "nolock");

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/nolock".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    // No lock entry written -> lock_missing.

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "lock_missing")
        .expect("lock_missing diagnostic");
    assert_eq!(diag.severity, PackageDiagnosticSeverity::Error);
    assert_eq!(diag.scope, InstalledPackageScope::Project);
    assert!(
        diag.message.contains("disabled at runtime"),
        "lock_missing message must declare disabled-at-runtime state: {:?}",
        diag.message
    );
}

#[test]
fn resolver_reports_lock_drift_with_expected_and_actual_hash_and_disabled_state() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/drift");
    write_package(&package_root, "drift");
    let actual_hash = manifest_sha256(&package_root.join("package.toml")).unwrap();

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/drift".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[PackageLockEntry {
            identity_kind: "local".to_string(),
            identity_value: package_root.canonicalize().unwrap().display().to_string(),
            source: "./vendor/drift".to_string(),
            package_root: package_root.canonicalize().unwrap(),
            cache_path: None,
            git_commit: None,
            manifest_sha256: "0".repeat(64), // intentionally wrong
            contributions: Vec::new(),
        }])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "lock_drifted")
        .expect("lock_drifted diagnostic");
    assert!(
        diag.message.contains("expected"),
        "lock_drifted message must name the expected hash: {:?}",
        diag.message
    );
    assert!(
        diag.message.contains("actual"),
        "lock_drifted message must name the actual hash: {:?}",
        diag.message
    );
    assert!(
        diag.message.contains(&actual_hash),
        "lock_drifted message must include the actual hash value: {:?}",
        diag.message
    );
    assert!(
        diag.message.contains("disabled at runtime"),
        "lock_drifted message must declare disabled-at-runtime state: {:?}",
        diag.message
    );
}

#[test]
fn resolver_reports_missing_top_level_manifest_field_through_resolution() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let package_root = workspace.path().join("vendor/noname");
    std::fs::create_dir_all(&package_root).unwrap();
    // package.toml missing the required top-level `name` field. The manifest
    // hash still computes (it reads bytes), so resolution reaches discovery
    // before the parse failure surfaces as a discovery_failed diagnostic.
    std::fs::write(
        package_root.join("package.toml"),
        "description = \"package with no name\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./vendor/noname".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            "./vendor/noname".to_string(),
            &package_root,
        )
        .unwrap()])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 0);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "discovery_failed")
        .expect("discovery_failed diagnostic");
    assert!(
        diag.message.contains("disabled at runtime"),
        "invalid-manifest discovery_failed message must declare disabled-at-runtime state: {:?}",
        diag.message
    );
}

#[test]
fn resolver_reports_duplicate_manifest_name_within_precedence_layer() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let pkg_a = workspace.path().join("vendor/dup-a");
    let pkg_b = workspace.path().join("vendor/dup-b");
    write_package(&pkg_a, "dup");
    write_package(&pkg_b, "dup"); // identical manifest name, same scope

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[
            PackageDeclaration {
                source: "./vendor/dup-a".to_string(),
                filters: Default::default(),
            },
            PackageDeclaration {
                source: "./vendor/dup-b".to_string(),
                filters: Default::default(),
            },
        ])
        .unwrap();
    store
        .write_lock(&[
            opi_coding_agent::package_resolver::local_lock_entry(
                "./vendor/dup-a".to_string(),
                &pkg_a,
            )
            .unwrap(),
            opi_coding_agent::package_resolver::local_lock_entry(
                "./vendor/dup-b".to_string(),
                &pkg_b,
            )
            .unwrap(),
        ])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(
        result.packages.len(),
        1,
        "one of the same-layer duplicates survives"
    );
    let dup = result
        .diagnostics
        .iter()
        .find(|d| d.code == "duplicate_name")
        .expect("duplicate_name diagnostic");
    assert_eq!(dup.severity, PackageDiagnosticSeverity::Error);
    assert_eq!(dup.scope, InstalledPackageScope::Project);
    assert!(
        dup.message.contains("dup"),
        "duplicate_name message must name the manifest name: {:?}",
        dup.message
    );
    assert!(
        dup.message.contains("disabled at runtime"),
        "duplicate_name message must declare disabled-at-runtime state: {:?}",
        dup.message
    );
    assert!(
        dup.message.contains("./vendor/dup-a"),
        "duplicate_name message must reference the surviving package source: {:?}",
        dup.message
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "duplicate_name")
            .count(),
        1,
        "exactly one duplicate_name diagnostic for the dropped package"
    );
}

#[test]
fn resolver_does_not_report_duplicate_for_project_over_global_override() {
    let workspace = tempdir().unwrap();
    let user = tempdir().unwrap();
    let global_root = user.path().join("global-todo");
    let project_root = workspace.path().join("project-todo");
    write_package(&global_root, "todo");
    write_package(&project_root, "todo");

    let global = PackageStore::global(user.path().to_path_buf());
    global
        .write_declarations(&[PackageDeclaration {
            source: global_root.display().to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    global
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            global_root.display().to_string(),
            &global_root,
        )
        .unwrap()])
        .unwrap();

    let project = PackageStore::project(workspace.path().to_path_buf());
    project
        .write_declarations(&[PackageDeclaration {
            source: project_root.display().to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    project
        .write_lock(&[opi_coding_agent::package_resolver::local_lock_entry(
            project_root.display().to_string(),
            &project_root,
        )
        .unwrap()])
        .unwrap();

    let result = resolve_installed_packages(workspace.path(), user.path()).unwrap();

    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.packages[0].scope, InstalledPackageScope::Project);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.code == "duplicate_name"),
        "project-over-global override is a precedence change, not a same-layer duplicate: {:?}",
        result.diagnostics
    );
}
