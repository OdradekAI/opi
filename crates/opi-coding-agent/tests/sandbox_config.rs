//! Sandbox configuration, CLI override, and fallback-diagnostic contract
//! tests for Phase 15 task 15.3.
//!
//! This task is substrate-only: it owns the `[sandbox]` TOML schema, the
//! `--sandbox`/`--sandbox-require` CLI surface, deterministic CLI-over-TOML
//! resolution via `OpiConfig::apply_sandbox_overrides`, and the stable
//! `CODE_SANDBOX_DEGRADED` / `CODE_SANDBOX_UNAVAILABLE` diagnostic literals
//! with redacted `{layer, reason}` details. Production mode dispatch
//! (`sandbox::prepare` inside `LocalBashOperations::exec`) and CLI startup
//! propagation are owned by 15.5.1; the tests here exercise the resolver and
//! diagnostic constructors directly.

use std::path::PathBuf;

use clap::Parser;
use opi_agent::diagnostic::Severity;
use opi_coding_agent::cli::Cli;
use opi_coding_agent::config::{
    ConfigError, ConfigSource, OpiConfig, SandboxConfig, SandboxMode, load_config_file,
    resolve_config,
};
use opi_coding_agent::diagnostics::{
    CODE_SANDBOX_DEGRADED, CODE_SANDBOX_UNAVAILABLE, SOURCE_SANDBOX, SandboxReason,
    sandbox_degraded_diagnostic, sandbox_unavailable_diagnostic,
};
use serde_json::json;

// --- helpers ---

fn write_temp_config(dir: &std::path::Path, contents: &str) -> PathBuf {
    let path = dir.join("config.toml");
    std::fs::write(&path, contents).unwrap();
    path
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

#[test]
fn cli_sandbox_off_parses() {
    let cli = Cli::try_parse_from(["opi", "--sandbox", "off"]).unwrap();
    assert_eq!(cli.sandbox, Some(SandboxMode::Off));
}

#[test]
fn cli_sandbox_strict_parses() {
    let cli = Cli::try_parse_from(["opi", "--sandbox", "strict"]).unwrap();
    assert_eq!(cli.sandbox, Some(SandboxMode::Strict));
}

#[test]
fn cli_sandbox_defaults_to_none() {
    let cli = Cli::try_parse_from(["opi"]).unwrap();
    assert!(cli.sandbox.is_none());
}

#[test]
fn cli_sandbox_require_flag_parses_true() {
    let cli = Cli::try_parse_from(["opi", "--sandbox-require"]).unwrap();
    assert!(cli.sandbox_require);
}

#[test]
fn cli_sandbox_require_defaults_false() {
    let cli = Cli::try_parse_from(["opi"]).unwrap();
    assert!(!cli.sandbox_require);
}

#[test]
fn cli_sandbox_invalid_value_is_named_parser_error() {
    let result = Cli::try_parse_from(["opi", "--sandbox", "bogus"]);
    let err = result.expect_err("invalid --sandbox value must be rejected");
    // clap surfaces a named parser error (not a panic, not a silent default).
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("sandbox"),
        "parser error should name the sandbox flag: {msg}"
    );
}

// ---------------------------------------------------------------------------
// TOML parsing + defaults
// ---------------------------------------------------------------------------

#[test]
fn sandbox_defaults_to_off_require_false_toggles_absent() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.sandbox.mode, SandboxMode::Off);
    assert!(!config.sandbox.require);
    assert!(config.sandbox.fs.is_none());
    assert!(config.sandbox.network.is_none());
    assert!(config.sandbox.syscalls.is_none());
}

/// DoD `phase15-sandbox-config-production-path` (15.5.1): invalid sandbox
/// configuration exits before provider or command construction. An invalid
/// `[sandbox] mode` in TOML is rejected by `resolve_config` (a named `ConfigError`,
/// returned before any provider is built); an invalid `--sandbox` CLI value is
/// rejected by clap's `ValueEnum` at parse time (companion to
/// `cli_sandbox_invalid_value_is_named_parser_error`). The harness never starts
/// in either case.
#[test]
fn invalid_sandbox_config_exits_before_provider_construction() {
    // Invalid TOML mode -> resolve_config error, no provider built.
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "[sandbox]\nmode = \"garbage\"\n");
    let result = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(path),
        env_model: None,
        project_dir: None,
        user_config_path: None,
    });
    let err = result.expect_err("invalid [sandbox] mode must error before provider construction");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("sandbox") || msg.contains("mode") || msg.contains("garbage"),
        "resolver error should name the sandbox problem: {msg}"
    );

    // Invalid CLI value -> clap rejects at parse, before any provider construction.
    Cli::try_parse_from(["opi", "--sandbox", "nope"])
        .expect_err("invalid --sandbox value must be rejected at parse");
}

#[test]
fn opiconfig_default_sandbox_is_off() {
    let config = OpiConfig::default();
    assert_eq!(config.sandbox.mode, SandboxMode::Off);
    assert!(!config.sandbox.require);
}

#[test]
fn sandbox_parses_strict_with_all_toggles() {
    let toml = "[sandbox]\nmode = \"strict\"\nrequire = true\nfs = true\nnetwork = true\nsyscalls = true\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.sandbox.mode, SandboxMode::Strict);
    assert!(config.sandbox.require);
    assert_eq!(config.sandbox.fs, Some(true));
    assert_eq!(config.sandbox.network, Some(true));
    assert_eq!(config.sandbox.syscalls, Some(true));
}

#[test]
fn sandbox_parses_partial_toggles_leaving_others_absent() {
    let toml = "[sandbox]\nmode = \"strict\"\nfs = false\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let config = load_config_file(&path).unwrap();
    assert_eq!(config.sandbox.mode, SandboxMode::Strict);
    assert_eq!(config.sandbox.fs, Some(false));
    assert!(config.sandbox.network.is_none());
    assert!(config.sandbox.syscalls.is_none());
}

#[test]
fn sandbox_invalid_mode_is_named_parse_error() {
    let toml = "[sandbox]\nmode = \"bogus\"\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let err = load_config_file(&path).expect_err("invalid mode must error");
    // Named parser error (ConfigError::Parse), not a panic or silent default.
    match err {
        ConfigError::Parse { .. } => {}
        other => panic!("expected ConfigError::Parse for invalid mode, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(
        msg.contains("bogus"),
        "parse error should echo the invalid value: {msg}"
    );
}

#[test]
fn sandbox_require_without_mode_parses() {
    let toml = "[sandbox]\nrequire = true\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let config = load_config_file(&path).unwrap();
    // require is independent of mode; default mode stays off.
    assert_eq!(config.sandbox.mode, SandboxMode::Off);
    assert!(config.sandbox.require);
}

// ---------------------------------------------------------------------------
// CLI-over-TOML resolution (substrate hook; 15.5.1 wires main.rs)
// ---------------------------------------------------------------------------

#[test]
fn resolve_config_with_no_sources_keeps_sandbox_off() {
    let config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: None,
        user_config_path: None,
    })
    .unwrap();
    assert_eq!(config.sandbox.mode, SandboxMode::Off);
    assert!(!config.sandbox.require);
}

#[test]
fn apply_sandbox_overrides_mode_beats_toml() {
    let toml = "[sandbox]\nmode = \"strict\"\nrequire = true\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let mut config = load_config_file(&path).unwrap();
    assert_eq!(config.sandbox.mode, SandboxMode::Strict);
    config.apply_sandbox_overrides(Some(SandboxMode::Off), None);
    assert_eq!(config.sandbox.mode, SandboxMode::Off);
    // require untouched when None passed.
    assert!(config.sandbox.require);
}

#[test]
fn apply_sandbox_overrides_require_beats_toml() {
    let toml = "[sandbox]\nrequire = true\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let mut config = load_config_file(&path).unwrap();
    assert!(config.sandbox.require);
    config.apply_sandbox_overrides(None, Some(false));
    assert!(!config.sandbox.require);
}

#[test]
fn apply_sandbox_overrides_none_is_noop() {
    let toml = "[sandbox]\nmode = \"strict\"\nrequire = true\n";
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), toml);
    let mut config = load_config_file(&path).unwrap();
    config.apply_sandbox_overrides(None, None);
    assert_eq!(config.sandbox.mode, SandboxMode::Strict);
    assert!(config.sandbox.require);
}

#[test]
fn apply_sandbox_overrides_constructs_strict_require() {
    // Mirrors the 15.5.1 main.rs wiring: a strict+require CLI override on a
    // default-off config flips both.
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_config(dir.path(), "");
    let mut config = load_config_file(&path).unwrap();
    config.apply_sandbox_overrides(Some(SandboxMode::Strict), Some(true));
    assert_eq!(config.sandbox.mode, SandboxMode::Strict);
    assert!(config.sandbox.require);
}

#[test]
fn sandbox_precedence_is_cli_then_explicit_then_project_then_user() {
    let root = tempfile::tempdir().unwrap();
    let user_dir = root.path().join("user");
    let project_dir = root.path().join("project");
    let explicit_dir = root.path().join("explicit");
    std::fs::create_dir_all(&user_dir).unwrap();
    std::fs::create_dir_all(project_dir.join(".opi")).unwrap();
    std::fs::create_dir_all(&explicit_dir).unwrap();

    let user_config = user_dir.join("config.toml");
    std::fs::write(
        &user_config,
        "[sandbox]\nmode = \"strict\"\nrequire = false\nfs = false\nnetwork = false\nsyscalls = false\n",
    )
    .unwrap();
    std::fs::write(
        project_dir.join(".opi").join("config.toml"),
        "[sandbox]\nrequire = true\nfs = true\nnetwork = true\n",
    )
    .unwrap();
    let explicit_config = explicit_dir.join("config.toml");
    std::fs::write(
        &explicit_config,
        "[sandbox]\nmode = \"off\"\nrequire = false\nfs = false\n",
    )
    .unwrap();

    let mut config = resolve_config(ConfigSource {
        cli_model: None,
        config_path: Some(explicit_config),
        env_model: None,
        project_dir: Some(project_dir),
        user_config_path: Some(user_config),
    })
    .unwrap();
    let cli = Cli::parse_from(["opi", "--sandbox", "strict", "--sandbox-require"]);
    config.apply_sandbox_overrides(cli.sandbox, cli.sandbox_require.then_some(true));

    assert_eq!(
        config.sandbox,
        SandboxConfig {
            // CLI beats explicit `off`.
            mode: SandboxMode::Strict,
            // The real one-way CLI flag beats explicit `false`.
            require: true,
            // Explicit config beats the project value.
            fs: Some(false),
            // Project config beats the user value.
            network: Some(true),
            // The untouched user value survives every higher layer.
            syscalls: Some(false),
        }
    );
}

// ---------------------------------------------------------------------------
// Fallback diagnostics
// ---------------------------------------------------------------------------

#[test]
fn degraded_diagnostic_has_stable_code_source_redacted_details() {
    let d = sandbox_degraded_diagnostic("landlock", SandboxReason::LandlockTcpUnavailable);
    assert_eq!(d.code, CODE_SANDBOX_DEGRADED);
    assert_eq!(d.source, SOURCE_SANDBOX);
    assert_eq!(d.severity, Severity::Warning);
    let obj = d
        .details
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .expect("details object present");
    // Redacted: only layer + reason, never command/env/path/secrets.
    assert_eq!(obj.len(), 2, "details must carry only layer and reason");
    assert_eq!(obj.get("layer"), Some(&json!("landlock")));
    assert_eq!(
        obj.get("reason"),
        Some(&json!("landlock TCP bind/connect unavailable below ABI 4"))
    );
}

#[test]
fn unavailable_diagnostic_has_stable_code_source_redacted_details() {
    let d = sandbox_unavailable_diagnostic(
        "windows-l3",
        SandboxReason::WindowsStrictConfinementUnavailable,
    );
    assert_eq!(d.code, CODE_SANDBOX_UNAVAILABLE);
    assert_eq!(d.source, SOURCE_SANDBOX);
    assert_eq!(d.severity, Severity::Warning);
    let obj = d
        .details
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .expect("details object present");
    assert_eq!(obj.len(), 2, "details must carry only layer and reason");
    assert_eq!(obj.get("layer"), Some(&json!("windows-l3")));
    assert_eq!(
        obj.get("reason"),
        Some(&json!(
            "windows provides no L1-L3 strict confinement (L0 Job-Object only)"
        ))
    );
}

#[test]
fn degraded_and_unavailable_codes_are_distinct() {
    // Temporary layer degradation must be distinguishable from permanent
    // platform unavailability by code alone.
    assert_ne!(CODE_SANDBOX_DEGRADED, CODE_SANDBOX_UNAVAILABLE);
}

#[test]
fn sandbox_diagnostic_constants_are_stable_literals() {
    // Pin the literal values; embedders match these by string.
    assert_eq!(CODE_SANDBOX_DEGRADED, "opi.sandbox.degraded");
    assert_eq!(CODE_SANDBOX_UNAVAILABLE, "opi.sandbox.unavailable");
    assert_eq!(SOURCE_SANDBOX, "sandbox");
}

#[test]
fn sandbox_diagnostic_reason_is_closed_and_redaction_safe() {
    let d = sandbox_degraded_diagnostic("seccomp", SandboxReason::SeccompFilterBuildFailed);
    let obj = d
        .details
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .expect("details object present");
    assert_eq!(obj.len(), 2);
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    assert_eq!(keys, vec!["layer", "reason"]);
    let serialized = serde_json::to_string(&d).unwrap();
    for canary in ["AKIAEXAMPLE", "secret", "/home/u", "command="] {
        assert!(
            !serialized.contains(canary),
            "closed reason leaked canary {canary}: {serialized}"
        );
    }
}

// ---------------------------------------------------------------------------
// SandboxConfig sanity (constructor + equality used by resolver paths)
// ---------------------------------------------------------------------------

#[test]
fn sandbox_config_default_is_off_require_false() {
    let c = SandboxConfig::default();
    assert_eq!(c.mode, SandboxMode::Off);
    assert!(!c.require);
    assert!(c.fs.is_none());
    assert!(c.network.is_none());
    assert!(c.syscalls.is_none());
}
