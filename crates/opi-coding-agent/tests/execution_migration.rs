//! Phase 16 task 16.16.1 — migration contract guard.
//!
//! After 16.16.1 removes the built-in native sandbox from core, the legacy
//! Phase 15 inputs are REJECTED (not aliased) with stable, actionable
//! remediation pointing at the execution-backend surface and the package
//! workflow, while default local execution retains policy-neutral L0
//! supervision. This file inverts the deleted `sandbox_config.rs` acceptance
//! suite into rejection + migration-target acceptance.
//!
//! Design references:
//! - `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
//!   `## Migration from Phase 15`: "`[sandbox]`, `--sandbox`, and
//!   `--sandbox-require` ... No compatibility aliases are added."; "Corrected
//!   L0 supervision remains in core."; non-goal "Preserving unreleased Phase
//!   15 sandbox configuration aliases."
//! - `### Supervision`: L0 supervision "is reported only as `supervised`" and
//!   applies to local target processes; it stays after native policy removal.

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use opi_agent::diagnostic::Severity;
use opi_agent::diagnostic::code::CODE_CONFIG_PARSE_FAILED;

use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::{ConfigError, ConfigSource, load_config_file, resolve_config};
use opi_coding_agent::diagnostic_bridge::diagnostic_from_config;

/// Stable remediation needles every legacy-sandbox rejection must surface so a
/// user (or embedder matching output) can find the replacement surface. The
/// needles name the execution-backend config block / CLI flag and the package
/// workflow.
const REMEDIATION_NEEDLES: &[&str] = &["--execution-backend", "[execution]", "opi package"];

/// Write a TOML config body to a fresh temp file and return its path.
fn write_temp_config(body: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "opi-execution-migration-{}-{}.toml",
        std::process::id(),
        // Vary across calls within one test binary so independent fixtures do
        // not collide. `Math/random`-free: use a monotonic counter via a file
        // count probe is overkill; the pid + an atomic-ish suffix suffices.
        unique_suffix(),
    ));
    let mut file = std::fs::File::create(&path).expect("create temp config");
    file.write_all(body.as_bytes()).expect("write temp config");
    path
}

fn unique_suffix() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed).to_string()
}

/// The remediation text every legacy-sandbox rejection carries names the
/// replacement execution-backend surface and the package workflow, and is
/// stable enough to match programmatically.
fn assert_has_remediation(message: &str) {
    for needle in REMEDIATION_NEEDLES {
        assert!(
            message.contains(needle),
            "legacy-sandbox rejection must name remediation `{needle}`; got: {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// CLI rejection (legacy flags are no longer accepted)
// ---------------------------------------------------------------------------

#[test]
fn cli_sandbox_off_is_rejected() {
    let result = Cli::try_parse_from(["opi", "--sandbox", "off"]);
    let err = result.expect_err("--sandbox off must be rejected");
    assert_has_remediation(&err.to_string());
}

#[test]
fn cli_sandbox_strict_is_rejected() {
    let result = Cli::try_parse_from(["opi", "--sandbox", "strict"]);
    let err = result.expect_err("--sandbox strict must be rejected");
    assert_has_remediation(&err.to_string());
}

#[test]
fn cli_sandbox_require_is_rejected() {
    // The legacy flag was bare (`--sandbox-require`); it must be rejected with
    // remediation regardless of whether a value follows.
    let err = Cli::try_parse_from(["opi", "--sandbox-require"])
        .expect_err("--sandbox-require must be rejected");
    assert_has_remediation(&err.to_string());
}

#[test]
fn cli_sandbox_bare_is_rejected() {
    // `--sandbox` took a value historically, but a BARE invocation (or one
    // followed by another flag) must also reject through the hidden value
    // parser with the remediation text — not clap's stock "a value is
    // required" error.
    let err =
        Cli::try_parse_from(["opi", "--sandbox"]).expect_err("bare --sandbox must be rejected");
    assert_has_remediation(&err.to_string());
}

#[test]
fn cli_no_longer_advertises_a_sandbox_flag() {
    // `--help` is the user-facing flag inventory. A removed flag must not appear
    // in the long help text (the Sandbox policy block is gone with the flag).
    let result = Cli::try_parse_from(["opi", "--help"]);
    let err = result.expect_err("--help exits clap");
    let help = err.to_string();
    assert!(
        !help.contains("--sandbox"),
        "removed --sandbox flag must not appear in help: {help}"
    );
}

// ---------------------------------------------------------------------------
// TOML rejection (legacy [sandbox] table is rejected in every layer)
// ---------------------------------------------------------------------------

#[test]
fn sandbox_mode_off_table_is_rejected() {
    let path = write_temp_config("[sandbox]\nmode = \"off\"\n");
    let result = load_config_file(&path);
    let err = result.expect_err("[sandbox] mode=\"off\" must be rejected");
    assert_has_remediation(&err.to_string());
}

#[test]
fn sandbox_mode_strict_table_is_rejected() {
    let path = write_temp_config("[sandbox]\nmode = \"strict\"\n");
    load_config_file(&path).expect_err("[sandbox] mode=\"strict\" must be rejected");
}

#[test]
fn sandbox_require_toggle_is_rejected() {
    let path = write_temp_config("[sandbox]\nrequire = true\n");
    load_config_file(&path).expect_err("[sandbox] require must be rejected");
}

#[test]
fn sandbox_layer_toggles_are_rejected() {
    for toggle in ["fs = true", "network = false", "syscalls = true"] {
        let path = write_temp_config(&format!("[sandbox]\n{toggle}\n"));
        load_config_file(&path).expect_err(&format!("[sandbox] {toggle} must be rejected"));
    }
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_user_layer() {
    let user = write_temp_config("[sandbox]\nmode = \"strict\"\n");
    let source = ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: Some(user),
    };
    resolve_config(source).expect_err("user-layer [sandbox] must be rejected");
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_explicit_layer() {
    let explicit = write_temp_config("[sandbox]\nmode = \"off\"\n");
    let source = ConfigSource {
        cli_model: None,
        config_path: Some(explicit),
        env_model: None,
        project_dir: None,
        user_config_path: None,
    };
    resolve_config(source).expect_err("explicit-layer [sandbox] must be rejected");
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_project_layer() {
    // A trusted project still cannot reintroduce the removed sandbox section.
    let project_dir = std::env::temp_dir().join(format!(
        "opi-migration-project-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let opi_dir = project_dir.join(".opi");
    std::fs::create_dir_all(&opi_dir).expect("create project .opi dir");
    std::fs::write(
        opi_dir.join("config.toml"),
        "[sandbox]\nmode = \"strict\"\n",
    )
    .expect("write project config");
    let source = ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: None,
    };
    resolve_config(source).expect_err("project-layer [sandbox] must be rejected");
}

// ---------------------------------------------------------------------------
// Migration target acceptance (the replacement surface works)
// ---------------------------------------------------------------------------

#[test]
fn execution_backend_local_flag_is_accepted() {
    Cli::try_parse_from(["opi", "--execution-backend", "local"])
        .expect("--execution-backend local must parse");
}

#[test]
fn execution_strategy_fixed_local_is_accepted() {
    let path = write_temp_config("[execution]\nstrategy = \"fixed\"\nbackend = \"local\"\n");
    let config = load_config_file(&path).expect("fixed/local execution config must load");
    assert_eq!(config.execution.backend, "local");
}

#[test]
fn legacy_sandbox_section_maps_to_stable_config_diagnostic() {
    // The `[sandbox]` rejection surfaces through the production
    // `diagnostic_from_config` bridge (wired into the `opi doctor` config
    // scope) with the stable parse-failed code, Error severity, and an
    // actionable remediation — mirroring the InvalidExecutionConfig arm.
    let diagnostic = diagnostic_from_config(&ConfigError::LegacySandboxSection);
    assert_eq!(diagnostic.code, CODE_CONFIG_PARSE_FAILED);
    assert_eq!(diagnostic.severity, Severity::Error);
    let action = diagnostic.action.as_deref().unwrap_or_default();
    assert!(
        action.contains("remove [sandbox]"),
        "action must carry remediation: {action}"
    );
    let remediation = diagnostic
        .details
        .as_ref()
        .and_then(|details| details.get("remediation"))
        .and_then(serde_json::Value::as_str)
        .expect("details carry the remediation text");
    assert_has_remediation(remediation);
}
