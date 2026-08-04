//! Phase 16 task 16.16.1 — crate/source boundary guard.
//!
//! After 16.16.1 removes the built-in native sandbox from the Opi binary, the
//! resolve graph of `opi-coding-agent` must contain neither the standalone
//! `opi-sandbox` crate nor the native-policy crates (`landlock`, `seccompiler`),
//! and the production source must not own native-restriction symbols. The
//! load-bearing structural proof is `cargo tree -p opi-coding-agent --edges
//! normal`; the source/Cargo.toml tripwires are secondary regression guards.
//!
//! Design references:
//! - `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
//!   Overview: "The Opi binary does not link `opi-sandbox`."
//!   `## Migration from Phase 15`: "Native confinement and its
//!   helper/capability-selection code leave core."
//! - Plan task 16.16 residue check: `cargo tree -p opi-coding-agent` contains
//!   no `opi-sandbox`/`landlock`/`seccompiler`; the source-guard symbol set
//!   (`PreparedSandbox`/`SandboxConfig`/`SandboxMode`/`StrictBackend`/
//!   `CODE_SANDBOX_DEGRADED`/`CODE_SANDBOX_UNAVAILABLE`) does not reappear
//!   (the independent crate name `opi-sandbox` is permitted; the legacy
//!   `--sandbox`/`--sandbox-require` flag strings are excluded from the
//!   tripwire because 16.16.1 keeps them as hidden rejection-trigger args —
//!   their rejection-with-remediation is pinned behaviorally in
//!   `execution_migration.rs`).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::Command;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    manifest_dir().join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// The normal-edge resolve graph of `opi-coding-agent` links NEITHER the
/// standalone `opi-sandbox` crate NOR the native-policy crates `landlock` and
/// `seccompiler`. This is the mechanical proof that the Opi binary owns no
/// native restriction after 16.16.1.
#[test]
fn cargo_tree_proves_no_sandbox_or_native_policy_dependency() {
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
    for forbidden in ["opi-sandbox", "landlock", "seccompiler", "seccomp"] {
        assert!(
            !tree.contains(forbidden),
            "forbidden native-policy/sandbox dependency `{forbidden}` present in opi-coding-agent graph:\n{tree}"
        );
    }
}

/// The native sandbox source tree is gone: no `src/sandbox.rs` file and no
/// `src/sandbox/` module directory remain in the crate.
#[test]
fn no_native_sandbox_module_remains_in_source() {
    let sandbox_file = manifest_dir().join("src/sandbox.rs");
    let sandbox_dir = manifest_dir().join("src/sandbox");
    assert!(
        !sandbox_file.exists(),
        "src/sandbox.rs must be deleted by 16.16.1"
    );
    assert!(
        !sandbox_dir.exists(),
        "src/sandbox/ module directory must be deleted by 16.16.1"
    );
}

/// `src/lib.rs` no longer declares or re-exports a `sandbox` module.
#[test]
fn lib_rs_does_not_reexport_sandbox() {
    let lib = read_repo_file("crates/opi-coding-agent/src/lib.rs");
    let normalized: String = lib.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !normalized.contains("pub mod sandbox"),
        "lib.rs must not declare a sandbox module: found `pub mod sandbox`"
    );
}

/// Static tripwire: NO production source file under `src/` carries a legacy
/// native-sandbox symbol from the source-guard set. This is the secondary
/// regression guard; the load-bearing proof is
/// [`cargo_tree_proves_no_sandbox_or_native_policy_dependency`] (the resolve
/// graph) plus [`no_native_sandbox_module_remains_in_source`] (the module is
/// gone). The walk is restricted to `src/` (production source); integration
/// tests live under `tests/` and are intentionally out of scope here.
#[test]
fn no_legacy_sandbox_symbols_in_production_source() {
    let src = manifest_dir().join("src");
    // `--sandbox` / `--sandbox-require` are INTENTIONALLY absent from the
    // tripwire set: 16.16.1 keeps them as hidden clap args that REJECT with
    // remediation (never aliases), so the flag strings legitimately appear in
    // cli.rs. Their rejection-with-remediation behavior is the load-bearing
    // proof, pinned behaviorally in tests/execution_migration.rs. The TYPE and
    // CODE symbols below are the native-restriction regression tripwires.
    let needles = [
        "PreparedSandbox",
        "SandboxConfig",
        "SandboxMode",
        "StrictBackend",
        "prepare_production",
        "build_tools_with_sandbox",
        "CODE_SANDBOX_DEGRADED",
        "CODE_SANDBOX_UNAVAILABLE",
    ];
    let mut hits = String::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        let read = match std::fs::read_dir(&dir) {
            Ok(read) => read,
            Err(error) => panic!("read src dir {}: {error}", dir.display()),
        };
        for entry in read {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            for needle in needles {
                if content.contains(needle) {
                    hits.push_str(&format!("{}: `{needle}`\n", path.display()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "legacy native-sandbox symbols found in production source:\n{hits}"
    );
}

/// `crates/opi-coding-agent/Cargo.toml` lists neither `landlock` nor
/// `seccompiler` as a dependency (cfg-gated or otherwise).
#[test]
fn cargo_toml_drops_native_policy_dependencies() {
    let cargo_toml = read_repo_file("crates/opi-coding-agent/Cargo.toml");
    let normalized: String = cargo_toml.split_whitespace().collect::<Vec<_>>().join(" ");
    for forbidden in ["landlock", "seccompiler"] {
        assert!(
            !normalized.contains(forbidden),
            "opi-coding-agent Cargo.toml must not depend on `{forbidden}`"
        );
    }
}
