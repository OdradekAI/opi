//! Phase 7 task 7.2 — opi-coding-agent diagnostic bridges.
//!
//! The coding agent owns package and config error families that live downstream
//! of `opi-agent` (so they cannot host a `From` impl for the foreign
//! `Diagnostic` type without hitting the orphan rule). These bridges are free
//! functions in `diagnostic_bridge` that map `PackageDiagnostic` and
//! `ConfigError` into the shared `Diagnostic` vocabulary with stable
//! `code`/`severity`/`source` tuples.
//!
//! The package layer already carries its own structured diagnostic (with a
//! dynamic `code: String`); since `Diagnostic.code` is a `&'static str`, the
//! granular package code is preserved in `details.package_code` and the shared
//! diagnostic carries a stable `package_diagnostic` code.

use std::collections::HashMap;
use std::path::PathBuf;

use opi_agent::diagnostic::code::*;
use opi_agent::diagnostic::{SOURCE_CONFIG, SOURCE_PACKAGE, Severity};
use opi_coding_agent::config::ConfigError;
use opi_coding_agent::diagnostic_bridge::{diagnostic_from_config, diagnostic_from_package};
use opi_coding_agent::doctor::{DoctorContext, DoctorScope, run_doctor};
use opi_coding_agent::package_activation::PackageActivationStore;
use opi_coding_agent::package_resolver::{
    InstalledPackageScope, PackageDiagnostic, PackageDiagnosticSeverity,
};

// ---------------------------------------------------------------------------
// PackageDiagnostic -> Diagnostic
// ---------------------------------------------------------------------------

fn pkg(severity: PackageDiagnosticSeverity, code: &str) -> PackageDiagnostic {
    PackageDiagnostic {
        scope: InstalledPackageScope::Project,
        source: "my-package".to_string(),
        severity,
        code: code.to_string(),
        message: format!("{code} occurred"),
    }
}

#[test]
fn package_error_diagnostic_maps_severity_and_carries_granular_code() {
    let diag = diagnostic_from_package(&pkg(
        PackageDiagnosticSeverity::Error,
        "manifest_hash_failed",
    ));
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_PACKAGE_DIAGNOSTIC);
    assert_eq!(diag.source, SOURCE_PACKAGE);
    let details = diag.details.as_ref().expect("carries structured details");
    assert_eq!(details["package_code"], "manifest_hash_failed");
    assert_eq!(details["package_source"], "my-package");
    assert_eq!(details["scope"], "project");
}

#[test]
fn package_warning_and_info_severities_map_through() {
    assert_eq!(
        diagnostic_from_package(&pkg(PackageDiagnosticSeverity::Warning, "discovery_failed"))
            .severity,
        Severity::Warning
    );
    assert_eq!(
        diagnostic_from_package(&pkg(PackageDiagnosticSeverity::Info, "loaded")).severity,
        Severity::Info
    );
}

// ---------------------------------------------------------------------------
// ConfigError -> Diagnostic
// ---------------------------------------------------------------------------

fn parse_err() -> toml::de::Error {
    toml::from_str::<toml::Value>("[unterminated").unwrap_err()
}

#[test]
fn config_parse_error_classifies_as_config_parse_failed() {
    let err = ConfigError::Parse {
        path: PathBuf::from("/secret/proj/.opi/config.toml"),
        source: Box::new(parse_err()),
    };
    let diag = diagnostic_from_config(&err);
    assert_eq!(diag.severity, Severity::Error);
    assert_eq!(diag.code, CODE_CONFIG_PARSE_FAILED);
    assert_eq!(diag.source, SOURCE_CONFIG);
    // Path lives in details (redactable), not in the message.
    let details = diag.details.as_ref().expect("carries path in details");
    assert_eq!(details["path"], "/secret/proj/.opi/config.toml");
}

#[test]
fn config_read_error_classifies_as_config_read_failed() {
    let err = ConfigError::Read {
        path: PathBuf::from("/etc/opi/config.toml"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    };
    let diag = diagnostic_from_config(&err);
    assert_eq!(diag.code, CODE_CONFIG_READ_FAILED);
    assert_eq!(diag.source, SOURCE_CONFIG);
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn config_diagnostics_carry_a_remediation_action() {
    let err = ConfigError::Parse {
        path: PathBuf::from("config.toml"),
        source: Box::new(parse_err()),
    };
    assert!(diagnostic_from_config(&err).action.is_some());
}

#[test]
fn doctor_surfaces_corrupt_activation_store_without_synthesizing_untrusted_packages() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let store = PackageActivationStore::global(user.path().to_path_buf());
    std::fs::write(store.store().trust_path(), "not = [valid toml").unwrap();
    let config = opi_coding_agent::config::OpiConfig::default();
    let store_probe = HashMap::new();
    let env_var = |_name: &str| None;
    let ctx = DoctorContext {
        config: &config,
        config_error: None,
        workspace_root: workspace.path(),
        user_config_dir: user.path(),
        sessions_dir: workspace.path(),
        term: None,
        term_program: None,
        term_features: None,
        no_color: false,
        colorterm: None,
        env_var: &env_var,
        store_probe: &store_probe,
    };

    let report = run_doctor(&[DoctorScope::Package], &ctx);
    assert!(
        report
            .entries
            .iter()
            .any(|entry| entry.diagnostic.code == "adapter_unavailable"),
        "activation-store failure must be surfaced truthfully: {:?}",
        report.entries
    );
    assert!(
        report
            .entries
            .iter()
            .all(|entry| entry.diagnostic.code != "package_untrusted"),
        "a read error must not synthesize untrusted package records: {:?}",
        report.entries
    );
}
