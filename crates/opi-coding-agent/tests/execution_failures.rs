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

/// SC16-14 "actionable remediation": the 14 stable codes each carry DISTINCT
/// remediation text (not one generic phrase), and every distinct code resolves to
/// a distinct remediation string. Guards a regression collapsing
/// `ExecutionFailure::remediation()` to a shared phrase.
#[test]
fn remediation_is_distinct_across_all_14_codes() {
    let all = one_of_each();
    assert_eq!(
        all.len(),
        14,
        "one_of_each must cover the full stable-code set"
    );
    let mut codes: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut remediation_values: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for f in &all {
        let code = f.code();
        assert!(
            codes.insert(code),
            "duplicate stable code in one_of_each: {code}"
        );
        let remediation = f.remediation();
        assert!(
            !remediation.trim().is_empty(),
            "empty remediation for {code}"
        );
        // Compare remediation VALUES, not code keys: two codes collapsing to
        // identical remediation text must fail here (remediation_values.len()
        // < 14).
        assert!(
            remediation_values.insert(remediation.clone()),
            "two distinct codes share identical remediation text `{remediation}`"
        );
    }
    assert_eq!(
        remediation_values.len(),
        14,
        "each of the 14 codes must carry distinct actionable remediation"
    );
}

/// The mode-aware `PermissionRequired` remediation (D.2 must-fix): interactive
/// mode gives the persistent USER-config allowance path only (no "run
/// interactively" nudge — the user already is — and no prompt promise, which
/// cannot be relied on in the startup-omission/fail-closed cases); headless
/// modes point at persistent allowance or running interactively. Both branches
/// and their divergence are pinned so a fragment swap/typo regresses.
#[test]
fn permission_required_remediation_is_mode_aware_and_divergent() {
    let interactive = ExecutionFailure::PermissionRequired {
        adapter_id: "opi-sandbox".into(),
        mode: ExecutionRunMode::Interactive,
    };
    let headless = ExecutionFailure::PermissionRequired {
        adapter_id: "opi-sandbox".into(),
        mode: ExecutionRunMode::Rpc,
    };
    let interactive_remediation = interactive.remediation();
    let headless_remediation = headless.remediation();
    assert!(
        interactive_remediation.contains("Allow it persistently in your USER config"),
        "interactive permission_required must give persistent-allowance guidance: {interactive_remediation}"
    );
    assert!(
        !interactive_remediation.contains("run interactively"),
        "interactive permission_required must NOT say 'run interactively': {interactive_remediation}"
    );
    assert!(
        !interactive_remediation.contains("cannot be granted non-interactively"),
        "interactive permission_required must NOT claim it cannot be granted non-interactively: {interactive_remediation}"
    );
    assert!(
        headless_remediation.contains("run interactively"),
        "headless permission_required must say 'run interactively': {headless_remediation}"
    );
    assert!(
        headless_remediation.contains("cannot be granted non-interactively"),
        "headless permission_required must explain non-interactive cannot grant: {headless_remediation}"
    );
    assert!(
        interactive_remediation != headless_remediation,
        "interactive and headless permission_required remediation must differ"
    );
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
