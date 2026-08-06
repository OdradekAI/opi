//! Manifest V2 compatibility tests (task 5.3).
//!
//! Covers: adapter manifest parsing, opi_version field, adapter validation,
//! relative/PATH command resolution, opi_version diagnostics, and backward
//! compatibility with existing V1 flat manifests.

use std::path::Path;

use opi_coding_agent::package_discovery::{
    AdapterManifest, OpiVersionDiagnostic, PackageDiscoveryError, PackageManifest,
    resolve_adapter_command, resolve_adapter_command_checked,
};

// ---------------------------------------------------------------------------
// V2 parsing: adapter and opi_version fields
// ---------------------------------------------------------------------------

#[test]
fn parses_manifest_v2_with_adapter_and_opi_version() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "todo"
description = "Todo package"
version = "0.1.0"
opi_version = ">=0.5,<0.7"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "todo-adapter"
args = ["--mode", "todo"]
protocol = "opi-extension-jsonl-v1"
timeout_ms = 30000
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    assert_eq!(manifest.name, "todo");
    assert_eq!(manifest.opi_version.as_deref(), Some(">=0.5,<0.7"));
    let adapter = manifest
        .adapter
        .as_ref()
        .expect("adapter should be present");
    assert_eq!(adapter.kind, "process-jsonl");
    assert_eq!(adapter.command, "todo-adapter");
    assert_eq!(adapter.args, vec!["--mode", "todo"]);
    assert_eq!(adapter.protocol, "opi-extension-jsonl-v1");
    assert_eq!(adapter.timeout_ms, Some(30000));
}

#[test]
fn flat_manifest_v1_without_adapter_still_valid() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "resource-only"
description = "Resource only package"
skills = ["review"]
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    assert_eq!(manifest.name, "resource-only");
    assert!(manifest.adapter.is_none());
    assert!(manifest.opi_version.is_none());
}

#[test]
fn opi_version_without_adapter_is_valid() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "versioned-pkg"
description = "Has version constraint"
opi_version = ">=0.5"
skills = ["review"]
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    assert_eq!(manifest.opi_version.as_deref(), Some(">=0.5"));
    assert!(manifest.adapter.is_none());
}

#[test]
fn adapter_without_opi_version_is_valid() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "adapter-pkg"
description = "Has adapter"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "my-adapter"
args = []
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    assert!(manifest.opi_version.is_none());
    assert!(manifest.adapter.is_some());
}

#[test]
fn adapter_optional_timeout_defaults_to_none() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "no-timeout"
description = "No timeout"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "my-adapter"
args = []
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    let adapter = manifest.adapter.as_ref().unwrap();
    assert!(adapter.timeout_ms.is_none());
}

#[test]
fn adapter_empty_args_parses() {
    let manifest = PackageManifest::from_toml(
        r#"
name = "empty-args"
description = "Empty args"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "my-adapter"
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .expect("parse manifest");

    let adapter = manifest.adapter.as_ref().unwrap();
    assert!(adapter.args.is_empty());
}

// ---------------------------------------------------------------------------
// Adapter validation
// ---------------------------------------------------------------------------

#[test]
fn adapter_rejects_unsupported_kind() {
    let err = PackageManifest::from_toml(
        r#"
name = "bad-kind"
description = "Bad kind"
skills = ["todo"]

[adapter]
kind = "grpc"
command = "my-adapter"
args = []
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .unwrap_err();

    assert!(
        matches!(err, PackageDiscoveryError::InvalidManifest { ref reason, .. }
            if reason.contains("unsupported adapter kind")),
        "expected unsupported adapter kind error, got: {err}"
    );
}

#[test]
fn adapter_rejects_unsupported_protocol() {
    let err = PackageManifest::from_toml(
        r#"
name = "bad-proto"
description = "Bad protocol"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "my-adapter"
args = []
protocol = "mcp-2025-03-26"
"#,
        Path::new("package.toml"),
    )
    .unwrap_err();

    assert!(
        matches!(err, PackageDiscoveryError::InvalidManifest { ref reason, .. }
            if reason.contains("unsupported adapter protocol")),
        "expected unsupported adapter protocol error, got: {err}"
    );
}

#[test]
fn adapter_rejects_empty_command() {
    let err = PackageManifest::from_toml(
        r#"
name = "no-cmd"
description = "No command"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = ""
args = []
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .unwrap_err();

    assert!(
        matches!(err, PackageDiscoveryError::MissingField { ref field, .. }
            if field == "adapter.command"),
        "expected missing adapter.command error, got: {err}"
    );
}

#[test]
fn adapter_rejects_whitespace_only_command() {
    let err = PackageManifest::from_toml(
        r#"
name = "ws-cmd"
description = "Whitespace command"
skills = ["todo"]

[adapter]
kind = "process-jsonl"
command = "   "
args = []
protocol = "opi-extension-jsonl-v1"
"#,
        Path::new("package.toml"),
    )
    .unwrap_err();

    assert!(
        matches!(err, PackageDiscoveryError::MissingField { ref field, .. }
            if field == "adapter.command"),
        "expected missing adapter.command error, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Command resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_command_relative_to_package_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg_dir = tmp.path().join("my-pkg");
    std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");

    let adapter = AdapterManifest {
        kind: "process-jsonl".into(),
        command: "bin/adapter".into(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".into(),
        timeout_ms: None,
    };

    let resolved = resolve_adapter_command(&adapter, &pkg_dir);
    assert_eq!(
        resolved,
        pkg_dir.join("bin").join("adapter"),
        "relative command should resolve against package root"
    );
}

#[test]
fn resolve_command_absolute_used_as_is() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg_dir = tmp.path().join("my-pkg");
    std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");

    let abs = if cfg!(windows) {
        r"C:\tools\adapter.exe"
    } else {
        "/usr/local/bin/adapter"
    };

    let adapter = AdapterManifest {
        kind: "process-jsonl".into(),
        command: abs.into(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".into(),
        timeout_ms: None,
    };

    let resolved = resolve_adapter_command(&adapter, &pkg_dir);
    assert_eq!(
        resolved,
        Path::new(abs),
        "absolute command should be used as-is"
    );
}

#[test]
fn resolve_command_path_lookup_when_no_separators() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let pkg_dir = tmp.path().join("my-pkg");
    std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");

    let adapter = AdapterManifest {
        kind: "process-jsonl".into(),
        command: "my-adapter".into(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".into(),
        timeout_ms: None,
    };

    let resolved = resolve_adapter_command(&adapter, &pkg_dir);
    // PATH lookup means the command is returned as a bare name.
    // It does NOT get resolved against the package root.
    assert_eq!(
        resolved,
        Path::new("my-adapter"),
        "bare command name should be returned as-is for PATH lookup"
    );
}

#[test]
fn adapter_relative_command_cannot_escape_package_root() {
    let pkg_dir = tempfile::tempdir().unwrap();
    let adapter = AdapterManifest {
        kind: "process-jsonl".to_string(),
        command: "../escape".to_string(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".to_string(),
        timeout_ms: None,
    };

    let err = resolve_adapter_command_checked(&adapter, pkg_dir.path()).unwrap_err();

    assert!(
        err.to_string().contains("escapes package root"),
        "expected escape diagnostic, got: {err}"
    );
}

#[cfg(windows)]
#[test]
fn adapter_windows_drive_relative_command_cannot_bypass_containment() {
    let pkg_dir = tempfile::tempdir().unwrap();
    let adapter = AdapterManifest {
        kind: "process-jsonl".to_string(),
        command: "C:adapter.exe".to_string(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".to_string(),
        timeout_ms: None,
    };

    let err = resolve_adapter_command_checked(&adapter, pkg_dir.path()).unwrap_err();

    assert!(
        err.to_string().contains("escapes package root"),
        "expected escape diagnostic, got: {err}"
    );
}

#[test]
fn checked_resolver_allows_relative_command_inside_package_root() {
    let pkg_dir = tempfile::tempdir().unwrap();
    let adapter = AdapterManifest {
        kind: "process-jsonl".to_string(),
        command: "bin/adapter".to_string(),
        args: vec![],
        protocol: "opi-extension-jsonl-v1".to_string(),
        timeout_ms: None,
    };

    let resolved = resolve_adapter_command_checked(&adapter, pkg_dir.path()).unwrap();

    assert_eq!(resolved, pkg_dir.path().join("bin").join("adapter"));
}

// ---------------------------------------------------------------------------
// opi_version diagnostics
// ---------------------------------------------------------------------------

#[test]
fn opi_version_compatible_produces_no_diagnostic() {
    // Use a version range that includes 0.5.x
    let diagnostic = OpiVersionDiagnostic::check(">=0.5,<1.0", "0.5.0");
    assert!(
        diagnostic.is_none(),
        "compatible version should produce no diagnostic"
    );
}

#[test]
fn opi_version_prerelease_is_parsed_with_semver_precedence() {
    let diagnostic = OpiVersionDiagnostic::check(">=0.8.0-0,<0.9.0-0", "0.8.0-rc.1");
    assert!(
        diagnostic.is_none(),
        "a prerelease inside an explicit prerelease-inclusive range must be compatible: {diagnostic:?}"
    );

    let stable_floor = OpiVersionDiagnostic::check(">=0.8,<0.9", "0.8.0-rc.1");
    assert!(
        stable_floor.is_some(),
        "SemVer prereleases must remain lower than the corresponding stable release"
    );
}

#[test]
fn opi_version_build_metadata_is_parsed_and_ignored_for_precedence() {
    let diagnostic = OpiVersionDiagnostic::check(">=0.8,<0.9", "0.8.0+build.17");
    assert!(
        diagnostic.is_none(),
        "build metadata must parse without changing SemVer precedence: {diagnostic:?}"
    );

    let exact = OpiVersionDiagnostic::check("=0.8.0", "0.8.0+build.17");
    assert!(
        exact.is_none(),
        "build metadata must not affect exact SemVer precedence equality: {exact:?}"
    );
}

#[test]
fn opi_version_incompatible_produces_diagnostic() {
    // Use a version range that excludes 0.5.x
    let diagnostic = OpiVersionDiagnostic::check(">=1.0", "0.5.0");
    assert!(
        diagnostic.is_some(),
        "incompatible version should produce diagnostic"
    );
    let d = diagnostic.unwrap();
    assert!(
        d.message.contains("incompatible"),
        "message should mention incompatibility"
    );
    assert!(
        d.message.contains(">=1.0"),
        "message should include the constraint"
    );
}

#[test]
fn opi_version_unparseable_constraint_produces_diagnostic() {
    let diagnostic = OpiVersionDiagnostic::check("not-a-version", "0.5.0");
    assert!(
        diagnostic.is_some(),
        "bad constraint should produce diagnostic"
    );
    let d = diagnostic.unwrap();
    assert!(
        d.message.contains("parse") || d.message.contains("invalid"),
        "message should mention parse failure: {}",
        d.message
    );
}

// ---------------------------------------------------------------------------
// Backward compatibility: existing manifests still parse
// ---------------------------------------------------------------------------

#[test]
fn existing_example_manifests_still_parse() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let examples_with_adapters = [
        "examples/todo/package.toml",
        "examples/permission-gate/package.toml",
        "examples/protected-paths/package.toml",
    ];
    let examples_without_adapters = [
        "examples/sub-agent/package.toml",
        "examples/plan-mode/package.toml",
        "examples/mcp-adapter/package.toml",
    ];

    for relative in examples_with_adapters {
        let path = repo_root.join(relative);
        let toml = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{relative} should be readable: {e}"));
        let manifest = PackageManifest::from_toml(&toml, &path)
            .unwrap_or_else(|err| panic!("{relative} should parse: {err}"));
        assert!(
            manifest.adapter.is_some(),
            "{relative}: should have adapter declaration"
        );
        assert_eq!(manifest.adapter.as_ref().unwrap().kind, "process-jsonl");
        assert_eq!(
            manifest.adapter.as_ref().unwrap().protocol,
            "opi-extension-jsonl-v1"
        );
    }

    for relative in examples_without_adapters {
        let path = repo_root.join(relative);
        let toml = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{relative} should be readable: {e}"));
        let manifest = PackageManifest::from_toml(&toml, &path)
            .unwrap_or_else(|err| panic!("{relative} should parse: {err}"));
        assert!(
            manifest.adapter.is_none(),
            "{relative}: existing examples should not have adapter"
        );
    }
}

#[test]
fn missing_resources_and_path_containment_unchanged() {
    // Verify that existing error paths still work
    let missing_err = PackageDiscoveryError::MissingAsset {
        package_name: "test".into(),
        kind: "skill".into(),
        name: "missing-skill".into(),
    };
    assert!(missing_err.to_string().contains("missing"));

    let security_err = PackageDiscoveryError::SecurityDiagnostic {
        package_name: "test".into(),
        path: Path::new("/evil").to_path_buf(),
        reason: "escape".into(),
    };
    assert!(security_err.to_string().contains("security"));
}
