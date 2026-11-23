//! Delay calculation, attempt budgets and provider error disposition.

use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use opi_ai::retry::{RetryConfig, calculate_backoff_delay};

#[test]
fn backoff_doubles_each_attempt() {
    let base = 1000;
    let max = 60_000;
    assert_eq!(calculate_backoff_delay(0, base, max), 1000);
    assert_eq!(calculate_backoff_delay(1, base, max), 2000);
    assert_eq!(calculate_backoff_delay(2, base, max), 4000);
    assert_eq!(calculate_backoff_delay(3, base, max), 8000);
}

#[test]
fn backoff_capped_at_max_delay() {
    assert_eq!(calculate_backoff_delay(3, 1000, 5000), 5000);
}

#[test]
fn backoff_with_large_attempt_stays_at_max() {
    assert_eq!(calculate_backoff_delay(10, 1000, 60_000), 60_000);
}

#[test]
fn backoff_zero_base_stays_zero() {
    assert_eq!(calculate_backoff_delay(5, 0, 60_000), 0);
}

#[test]
fn retry_config_default_values() {
    let config = RetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.initial_delay_ms, 1000);
    assert_eq!(config.max_delay_ms, 60_000);
}

#[test]
fn retry_config_delay_for_attempt_respects_retry_after() {
    let config = RetryConfig::default();
    let delay = config.delay_for_attempt(0, Some(5000));
    assert_eq!(delay, 5000);
}

#[test]
fn retry_config_delay_for_attempt_uses_backoff_when_no_retry_after() {
    let config = RetryConfig {
        max_attempts: 5,
        initial_delay_ms: 1000,
        max_delay_ms: 60_000,
    };
    assert_eq!(config.delay_for_attempt(0, None), 1000);
    assert_eq!(config.delay_for_attempt(1, None), 2000);
    assert_eq!(config.delay_for_attempt(2, None), 4000);
}

#[test]
fn retry_config_delay_respects_max_even_with_retry_after() {
    let config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 5000,
    };
    let delay = config.delay_for_attempt(0, Some(10000));
    assert_eq!(delay, 5000);
}

#[test]
fn retry_config_should_retry_within_max_attempts() {
    let config = RetryConfig::default();
    assert!(config.should_retry(0));
    assert!(config.should_retry(1));
    assert!(config.should_retry(2));
    assert!(!config.should_retry(3));
}

#[test]
fn rate_limited_is_retryable() {
    assert!(
        ProviderError::RateLimited {
            retry_after_ms: None
        }
        .is_retryable()
    );
}

#[test]
fn timeout_is_retryable() {
    assert!(ProviderError::Timeout.is_retryable());
}

#[test]
fn network_error_is_retryable() {
    // Transient transport failures (DNS/TLS/connection) classify as Network
    // and are retryable alongside RateLimited and Timeout.
    assert!(ProviderError::Network(ProviderErrorSummary::redacted()).is_retryable());
}

#[test]
fn auth_failed_is_not_retryable() {
    assert!(
        !ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected()).is_retryable()
    );
}

#[test]
fn request_failed_is_not_retryable() {
    assert!(!ProviderError::RequestFailed(ProviderErrorSummary::redacted()).is_retryable());
}

#[test]
fn config_error_is_not_retryable() {
    // Bad endpoint / unsupported model / bad profile is terminal: retrying will
    // not change the outcome. (Phase 12 task 12.7 DoD clause 3: config.)
    assert!(!ProviderError::Config(ProviderErrorSummary::redacted()).is_retryable());
}

#[test]
fn unsupported_capability_is_not_retryable() {
    // Preflight validation rejecting an unsupported image/tool/thinking
    // capability is terminal. (Phase 12 task 12.7 DoD clause 3: validation.)
    assert!(!ProviderError::UnsupportedCapability(ProviderErrorSummary::redacted()).is_retryable());
}

#[test]
fn provider_side_error_is_not_retryable() {
    // A non-rate-limit 5xx maps to ProviderSide and is not retried.
    assert!(!ProviderError::ProviderSide(ProviderErrorSummary::redacted()).is_retryable());
}
