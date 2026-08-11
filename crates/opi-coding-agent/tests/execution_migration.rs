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
//! - `docs/snapshots/phase16/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
//!   `## Migration from Phase 15`: "`[sandbox]`, `--sandbox`, and
//!   `--sandbox-require` ... No compatibility aliases are added."; "Corrected
//!   L0 supervision remains in core."; non-goal "Preserving unreleased Phase
//!   15 sandbox configuration aliases."
//! - `### Supervision`: L0 supervision "is reported only as `supervised`" and
//!   applies to local target processes; it stays after native policy removal.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use clap::Parser;

use opi_agent::diagnostic::Severity;
use opi_agent::diagnostic::code::CODE_CONFIG_PARSE_FAILED;

use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::{ConfigError, ConfigSource, load_config_file, resolve_config};
use opi_coding_agent::diagnostic_bridge::diagnostic_from_config;
use opi_coding_agent::diagnostics::LEGACY_SANDBOX_REMEDIATION;

/// Stable remediation needles every legacy-sandbox rejection must surface so a
/// user (or embedder matching output) can find the replacement surface. The
/// needles name the execution-backend config block / CLI flag and the package
/// workflow.
const REMEDIATION_NEEDLES: &[&str] = &["--execution-backend", "[execution]", "opi package"];

struct TempConfig {
    _owner: tempfile::TempDir,
    path: PathBuf,
}

impl TempConfig {
    fn path(&self) -> &Path {
        &self.path
    }
}

/// Write a TOML config body to an owned temporary directory.
fn write_temp_config(body: &str) -> TempConfig {
    let owner = tempfile::tempdir().expect("create temp config directory");
    let path = owner.path().join("config.toml");
    std::fs::write(&path, body).expect("write temp config");
    TempConfig {
        _owner: owner,
        path,
    }
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

/// Every present legacy `[sandbox]` shape takes the dedicated removed-surface
/// error path, whose public text is the exact stable remediation contract.
fn assert_exact_legacy_sandbox_rejection(body: &str) {
    let path = write_temp_config(body);
    let err = load_config_file(path.path()).expect_err("[sandbox] must be rejected");
    assert!(
        matches!(&err, ConfigError::LegacySandboxSection),
        "present [sandbox] must use the stable removed-surface error: {err:?}"
    );
    assert_eq!(err.to_string(), LEGACY_SANDBOX_REMEDIATION);
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
    assert_exact_legacy_sandbox_rejection("[sandbox]\nmode = \"off\"\n");
}

#[test]
fn sandbox_mode_strict_table_is_rejected() {
    assert_exact_legacy_sandbox_rejection("[sandbox]\nmode = \"strict\"\n");
}

#[test]
fn sandbox_require_toggle_is_rejected() {
    assert_exact_legacy_sandbox_rejection("[sandbox]\nrequire = true\n");
}

#[test]
fn sandbox_layer_toggles_are_rejected() {
    for toggle in ["fs = true", "network = false", "syscalls = true"] {
        assert_exact_legacy_sandbox_rejection(&format!("[sandbox]\n{toggle}\n"));
    }
}

#[test]
fn empty_sandbox_table_is_rejected_with_exact_remediation() {
    assert_exact_legacy_sandbox_rejection("[sandbox]\n");
}

#[test]
fn unknown_only_sandbox_table_is_rejected_with_exact_remediation() {
    assert_exact_legacy_sandbox_rejection("[sandbox]\nfuture = true\n");
}

#[test]
fn malformed_known_sandbox_field_is_rejected_with_exact_remediation() {
    assert_exact_legacy_sandbox_rejection("[sandbox]\nrequire = \"not-bool\"\n");
}

#[test]
fn non_table_sandbox_value_remains_a_parse_error() {
    let path = write_temp_config("sandbox = \"strict\"\n");
    let err = load_config_file(path.path()).expect_err("non-table sandbox value must be rejected");
    assert!(
        matches!(err, ConfigError::Parse { .. }),
        "only a present sandbox table maps to the removed-surface remediation: {err:?}"
    );
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_user_layer() {
    let user = write_temp_config("[sandbox]\nmode = \"strict\"\n");
    let source = ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: Some(user.path().to_path_buf()),
    };
    resolve_config(source).expect_err("user-layer [sandbox] must be rejected");
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_explicit_layer() {
    let explicit = write_temp_config("[sandbox]\nmode = \"off\"\n");
    let source = ConfigSource {
        cli_model: None,
        config_path: Some(explicit.path().to_path_buf()),
        env_model: None,
        project_dir: None,
        user_config_path: None,
    };
    resolve_config(source).expect_err("explicit-layer [sandbox] must be rejected");
}

#[test]
fn resolve_config_rejects_legacy_sandbox_in_project_layer() {
    // A trusted project still cannot reintroduce the removed sandbox section.
    let project_dir = tempfile::tempdir().expect("create project directory");
    let opi_dir = project_dir.path().join(".opi");
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
        project_dir: Some(project_dir.path().to_path_buf()),
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
    let config = load_config_file(path.path()).expect("fixed/local execution config must load");
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
    assert_has_remediation(action);
    let remediation = diagnostic
        .details
        .as_ref()
        .and_then(|details| details.get("remediation"))
        .and_then(serde_json::Value::as_str)
        .expect("details carry the remediation text");
    assert_has_remediation(remediation);
}

#[test]
fn legacy_sandbox_public_remediation_is_byte_identical_across_fields() {
    let error = ConfigError::LegacySandboxSection;
    let diagnostic = diagnostic_from_config(&error);
    let action = diagnostic.action.as_deref().expect("action remediation");
    let details = diagnostic
        .details
        .as_ref()
        .and_then(|value| value.get("remediation"))
        .and_then(serde_json::Value::as_str)
        .expect("details remediation");

    assert_eq!(action, details);
    assert_eq!(details, error.to_string());
    assert_eq!(details, LEGACY_SANDBOX_REMEDIATION);
    assert_has_remediation(action);
}
