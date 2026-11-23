//! Typed provider codes and redacted diagnostic extraction.

use opi_ai::provider::{ProviderError, ProviderErrorSummary, extract_diagnostic_error};

#[test]
fn extract_diagnostic_error_preserves_service_fault_typed_codes() {
    let error = ProviderError::ServiceFault {
        status: 529,
        error_name: Some("overloaded_error".to_owned()),
        retry_after_ms: Some(1_000),
        detail: ProviderErrorSummary::redacted(),
    };
    let info = extract_diagnostic_error(&error);
    assert_eq!(info.name, "ServiceFault");
    assert_eq!(info.http_status, Some(529));
    assert_eq!(info.error_name.as_deref(), Some("overloaded_error"));
    assert_eq!(info.message, error.to_string());
    assert!(info.message.contains("529"));
}

#[test]
fn extract_diagnostic_error_stays_secret_safe() {
    let error = ProviderError::AuthFailed(ProviderErrorSummary::from_untrusted(
        "Bearer sk-super-secret-canary",
    ));
    let info = extract_diagnostic_error(&error);
    assert_eq!(info.name, "AuthFailed");
    assert_eq!(info.http_status, None);
    assert_eq!(info.error_name, None);
    assert_eq!(info.message, error.to_string());
    assert!(!info.message.contains("sk-super-secret-canary"));
}

#[test]
fn extract_diagnostic_error_covers_untyped_variants() {
    let timeout = extract_diagnostic_error(&ProviderError::Timeout);
    assert_eq!(timeout.name, "Timeout");
    assert_eq!(timeout.http_status, None);
    assert_eq!(timeout.error_name, None);
    assert_eq!(timeout.message, ProviderError::Timeout.to_string());
    let rate_limited = extract_diagnostic_error(&ProviderError::RateLimited {
        retry_after_ms: Some(500),
    });
    assert_eq!(rate_limited.name, "RateLimited");
    assert_eq!(
        rate_limited.message,
        ProviderError::RateLimited {
            retry_after_ms: Some(500)
        }
        .to_string()
    );
    let cancelled = extract_diagnostic_error(&ProviderError::Cancelled);
    assert_eq!(cancelled.name, "Cancelled");
}
