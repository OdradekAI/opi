//! Task 16.6: the stable `ExecutionFailure` envelope.
//!
//! DoD item 12 — all 14 stable codes are declared with their exact literal and a
//! non-empty, redacted remediation; the 16.6-owned codes are produced via real
//! failure paths (`From<ActivationError>` + the router); remediation redacts
//! command text, env values, credentials, absolute paths, and raw stderr.
//!
//! DoD item 13 ("no degraded-success state") is enforced by the type system, not
//! a runtime test: `resolve_selection` returns `Result<Selection, ExecutionFailure>`
//! and neither type carries a warning/diagnostic field, and `ExecutionFailure` has
//! no `Degraded`/`Partial` variant. The routing tests in `execution_routing.rs`
//! assert the Ok path carries only `backend` + `mode`.

use opi_coding_agent::config::{ExecutionRunMode, ExecutionStrategy};
use opi_coding_agent::execution::{ExecutionFailure, UnavailableDetail};
use opi_coding_agent::package_activation::ActivationError;
use opi_coding_agent::package_store::PackageStoreError;

/// Construct one instance of every declared variant, so the declaration
/// coverage is exhaustive and a missing variant fails to compile.
fn one_of_each() -> Vec<ExecutionFailure> {
    vec![
        ExecutionFailure::PackageNotInstalled {
            name: "opi-sandbox".into(),
        },
        ExecutionFailure::PackageUntrusted {
            name: "opi-sandbox".into(),
        },
        ExecutionFailure::ContributionDisabled {
            name: "opi-sandbox".into(),
        },
        ExecutionFailure::PolicyDenied {
            adapter_id: "opi-sandbox".into(),
        },
        ExecutionFailure::PermissionRequired {
            adapter_id: "opi-sandbox".into(),
            mode: ExecutionRunMode::Rpc,
        },
        ExecutionFailure::PermissionDenied {
            adapter_id: "opi-sandbox".into(),
        },
        ExecutionFailure::NoEligibleAdapter {
            strategy: ExecutionStrategy::Model,
            mode: ExecutionRunMode::Interactive,
        },
        ExecutionFailure::AdapterNotSelected {
            requested: "ghost".into(),
            strategy: ExecutionStrategy::Fixed,
        },
        ExecutionFailure::AdapterUnavailable {
            adapter_id: Some("opi-sandbox".into()),
            detail: UnavailableDetail::Collision,
        },
        ExecutionFailure::ProtocolIncompatible,
        ExecutionFailure::ProtocolViolation,
        ExecutionFailure::ExecutionFailed,
        ExecutionFailure::ExecutionTimedOut,
        ExecutionFailure::CleanupUnconfirmed,
    ]
}

#[test]
fn all_14_codes_declared_with_stable_literal() {
    let expected = [
        "package_not_installed",
        "package_untrusted",
        "contribution_disabled",
        "policy_denied",
        "permission_required",
        "permission_denied",
        "no_eligible_adapter",
        "adapter_not_selected",
        "adapter_unavailable",
        "protocol_incompatible",
        "protocol_violation",
        "execution_failed",
        "execution_timed_out",
        "cleanup_unconfirmed",
    ];
    let codes: Vec<&str> = one_of_each().iter().map(|f| f.code()).collect();
    assert_eq!(codes, expected);
    assert_eq!(codes.len(), 14, "exactly 14 stable codes");
}

#[test]
fn every_code_has_nonempty_remediation() {
    for f in one_of_each() {
        let r = f.remediation();
        assert!(!r.trim().is_empty(), "empty remediation for {}", f.code());
    }
}

#[test]
fn from_activation_error_honors_pinned_mapping() {
    // The three pinned mappings documented on ActivationError.
    assert!(matches!(
        ExecutionFailure::from(ActivationError::NotInstalled("p".into())),
        ExecutionFailure::PackageNotInstalled { .. }
    ));
    assert!(matches!(
        ExecutionFailure::from(ActivationError::Untrusted {
            name: "p".into(),
            detail: "drift".into()
        }),
        ExecutionFailure::PackageUntrusted { .. }
    ));
    assert!(matches!(
        ExecutionFailure::from(ActivationError::Disabled("p".into())),
        ExecutionFailure::ContributionDisabled { .. }
    ));
    // CollidingAdapterId + Store -> adapter_unavailable (match exhaustiveness).
    assert_eq!(
        ExecutionFailure::from(ActivationError::CollidingAdapterId {
            adapter_id: "opi-sandbox".into(),
            other: "other-pkg".into()
        })
        .code(),
        "adapter_unavailable"
    );
    assert_eq!(
        ExecutionFailure::from(ActivationError::Store(PackageStoreError::Package(
            "store boom".into()
        )))
        .code(),
        "adapter_unavailable"
    );
}

/// A canary that must NEVER appear in any user-facing failure text.
const REDACT_CANARIES: &[&str] = &[
    "AKIAEXAMPLE",      // credential
    "/home/u/secret",   // absolute path
    "PASSWORD=hunter2", // env value
    "rm -rf /",         // command text
];

#[test]
fn redaction_omits_untrusted_detail_abs_path_and_secret() {
    // The untrusted `detail` may carry an absolute path / secret; it must be
    // dropped, never interpolated into remediation, code, or Display.
    let detail = format!(
        "drift at {} env {} cmd {}",
        REDACT_CANARIES[1], REDACT_CANARIES[2], REDACT_CANARIES[3]
    );
    let f = ExecutionFailure::from(ActivationError::Untrusted {
        name: "opi-sandbox".into(),
        detail,
    });
    let remediation = f.remediation();
    let display = format!("{f}");
    for canary in REDACT_CANARIES {
        assert!(
            !remediation.contains(canary),
            "remediation leaked canary {canary:?}: {remediation}"
        );
        assert!(
            !display.contains(canary),
            "Display leaked canary {canary:?}: {display}"
        );
    }
    // Positive: the safe package name IS surfaced (actionable).
    assert!(remediation.contains("opi-sandbox"));
}

#[test]
fn redaction_omits_store_display() {
    // The opaque Store display must be dropped by the From mapping; the mapped
    // AdapterUnavailable remediation is static.
    let store = PackageStoreError::Package(format!(
        "store failure {} {}",
        REDACT_CANARIES[0], REDACT_CANARIES[1]
    ));
    let f = ExecutionFailure::from(ActivationError::Store(store));
    let remediation = f.remediation();
    for canary in REDACT_CANARIES {
        assert!(
            !remediation.contains(canary),
            "store-derived remediation leaked canary {canary:?}: {remediation}"
        );
    }
}

#[test]
fn redaction_safe_across_all_declared_codes() {
    // Negative sweep: no constructed failure leaks any canary, and no remediation
    // interpolates command text / env / creds / abs paths.
    for f in one_of_each() {
        let remediation = f.remediation();
        let display = format!("{f}");
        for canary in REDACT_CANARIES {
            assert!(
                !remediation.contains(canary),
                "{} remediation leaked canary {canary:?}: {remediation}",
                f.code()
            );
            assert!(
                !display.contains(canary),
                "{} Display leaked canary {canary:?}: {display}",
                f.code()
            );
        }
    }
}
