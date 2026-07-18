//! Diagnostic bridges for opi-coding-agent-owned error families (Phase 7 task 7.2).
//!
//! `opi-agent` owns the shared [`opi_agent::Diagnostic`] model, and
//! `opi-coding-agent` depends on `opi-agent`. The package and config error
//! families live here, so they cannot host `impl From<&E> for Diagnostic`
//! without violating the orphan rule (both `From` and `Diagnostic` are foreign).
//! These free functions perform the mapping instead.
//!
//! The package layer already produces structured `PackageDiagnostic` values
//! (with a dynamic `code: String`). Because `Diagnostic.code` is a `&'static
//! str`, the granular package code is preserved in `details.package_code` and
//! the shared diagnostic carries a stable `package_diagnostic` code.

use opi_agent::diagnostic::code::*;
use opi_agent::diagnostic::{Diagnostic, SOURCE_ADAPTER, SOURCE_CONFIG, SOURCE_PACKAGE, Severity};

use crate::config::ConfigError;
use crate::package_resolver::{
    InstalledPackageScope, PackageDiagnostic, PackageDiagnosticSeverity,
};

/// Map a [`PackageDiagnostic`] into the shared [`Diagnostic`] vocabulary.
///
/// Severity is mapped directly. The package's granular `code`, `source`, and
/// `scope` are carried in `details` so the shared `code` stays a stable literal.
pub fn diagnostic_from_package(pd: &PackageDiagnostic) -> Diagnostic {
    let severity = match pd.severity {
        PackageDiagnosticSeverity::Info => Severity::Info,
        PackageDiagnosticSeverity::Warning => Severity::Warning,
        PackageDiagnosticSeverity::Error => Severity::Error,
    };
    let scope = match pd.scope {
        InstalledPackageScope::Global => "global",
        InstalledPackageScope::Project => "project",
    };
    Diagnostic::new(
        severity,
        CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "installed package diagnostic",
    )
    .details(serde_json::json!({
        "package_code": pd.code,
        "package_message": pd.message,
        "package_source": pd.source,
        "scope": scope,
    }))
}

/// Map a fatal installed-package resolution error into the shared vocabulary.
pub fn diagnostic_from_package_resolution_error(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_PACKAGE_RESOLUTION_FAILED,
        SOURCE_PACKAGE,
        "installed package resolution failed",
    )
    .details(serde_json::json!({ "package_error": error.to_string() }))
}

pub fn diagnostic_for_package_discovery_error(error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "package discovery failed",
    )
    .details(serde_json::json!({ "package_error": error.to_string() }))
}

pub fn diagnostic_for_resource_discovery_error(
    resource_kind: &'static str,
    error: impl std::fmt::Display,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "resource discovery failed",
    )
    .details(serde_json::json!({
        "resource_kind": resource_kind,
        "package_error": error.to_string(),
    }))
}

pub fn diagnostic_for_resource_layer_message(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "package resource composition diagnostic",
    )
    .details(serde_json::json!({ "package_message": message.into() }))
}

pub fn diagnostic_for_model_registry_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_PACKAGE_DIAGNOSTIC,
        SOURCE_PACKAGE,
        "model registry diagnostic",
    )
    .details(serde_json::json!({ "package_message": message.into() }))
}

pub fn diagnostic_for_unsupported_adapter_protocol(
    package_name: &str,
    actual_protocol: &str,
    adapter_command: &str,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_ADAPTER_PROTOCOL_UNSUPPORTED,
        SOURCE_ADAPTER,
        "unsupported adapter protocol",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "actual_protocol": actual_protocol,
        "expected_protocol": "opi-extension-jsonl-v1",
        "adapter_command": adapter_command,
        "disabled_at_runtime": true,
    }))
}

pub fn diagnostic_for_unsupported_adapter_kind(
    package_name: &str,
    actual_kind: &str,
    adapter_command: &str,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_ADAPTER_KIND_UNSUPPORTED,
        SOURCE_ADAPTER,
        "unsupported adapter kind",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "actual_kind": actual_kind,
        "expected_kind": "process-jsonl",
        "adapter_command": adapter_command,
        "disabled_at_runtime": true,
    }))
}

pub fn diagnostic_for_adapter_command_invalid(
    package_name: &str,
    adapter_command: &str,
    error: impl std::fmt::Display,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_ADAPTER_COMMAND_INVALID,
        SOURCE_ADAPTER,
        "adapter command invalid",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "adapter_command": adapter_command,
        "adapter_error": error.to_string(),
        "disabled_at_runtime": true,
    }))
}

pub fn diagnostic_for_adapter_startup_failed(
    package_name: &str,
    adapter_command: &str,
    error: impl std::fmt::Display,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_ADAPTER_STARTUP_FAILED,
        SOURCE_ADAPTER,
        "adapter startup failed",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "adapter_command": adapter_command,
        "adapter_error": error.to_string(),
        "disabled_at_runtime": true,
    }))
}

pub fn diagnostic_for_adapter_registration_failed(
    package_name: &str,
    error: impl std::fmt::Display,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Error,
        CODE_ADAPTER_REGISTRATION_FAILED,
        SOURCE_ADAPTER,
        "adapter registration failed",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "adapter_error": error.to_string(),
        "disabled_at_runtime": true,
    }))
}

pub fn diagnostic_for_adapter_host_message(
    package_name: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        Severity::Warning,
        CODE_ADAPTER_HOST_DIAGNOSTIC,
        SOURCE_ADAPTER,
        "adapter host diagnostic",
    )
    .details(serde_json::json!({
        "package_name": package_name,
        "adapter_error": message.into(),
    }))
}

/// Map a [`ConfigError`] into the shared [`Diagnostic`] vocabulary.
///
/// Both variants are startup-fatal, so they are `Error`. The config path is
/// placed in `details` (redactable in summary mode) rather than the message.
pub fn diagnostic_from_config(err: &ConfigError) -> Diagnostic {
    match err {
        ConfigError::Parse { path, .. } => Diagnostic::new(
            Severity::Error,
            CODE_CONFIG_PARSE_FAILED,
            SOURCE_CONFIG,
            "failed to parse config file",
        )
        .details(serde_json::json!({ "path": path.display().to_string() }))
        .action("fix the malformed TOML or remove the file to use defaults"),
        ConfigError::Read { path, .. } => Diagnostic::new(
            Severity::Error,
            CODE_CONFIG_READ_FAILED,
            SOURCE_CONFIG,
            "failed to read config file",
        )
        .details(serde_json::json!({ "path": path.display().to_string() }))
        .action("check file permissions or remove the file to use defaults"),
        ConfigError::InvalidCustomProvider {
            provider,
            model,
            field,
            ..
        } => Diagnostic::new(
            Severity::Error,
            CODE_CONFIG_PARSE_FAILED,
            SOURCE_CONFIG,
            "invalid custom provider configuration",
        )
        .details(serde_json::json!({
            "provider": provider,
            "model": model,
            "field": field,
        }))
        .action("fix the custom provider contract before starting opi"),
    }
}
