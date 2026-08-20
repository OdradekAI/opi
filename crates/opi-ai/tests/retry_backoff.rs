//! Retry/backoff/rate-limit tests (task 2.15).
//!
//! DoD: "exponential backoff, rate-limit header parsing (Retry-After,
//!       x-ratelimit-*), AutoRetry events, max attempts from config"
//!
//! This file tests: header parsing, backoff calculation, RetryConfig,
//! MockProvider error injection, ProviderError retryability.
//!
//! Agent-level retry integration tests are in opi-agent/tests/retry_agent.rs.

use opi_ai::Provider;
use opi_ai::provider::{CacheRetention, ProviderError, ProviderErrorSummary};
use opi_ai::retry::{
    RetryConfig, calculate_backoff_delay, parse_http_date_delay, parse_retry_after,
};
use opi_ai::test_support::{self, MockProvider, MockResponse};

// ---------------------------------------------------------------------------
// Retry-After header parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_retry_after_seconds_value() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "30".parse().unwrap());
    let ms = parse_retry_after(&headers);
    assert_eq!(ms, Some(30_000));
}

#[test]
fn parse_retry_after_no_header_returns_none() {
    let headers = reqwest::header::HeaderMap::new();
    assert_eq!(parse_retry_after(&headers), None);
}

#[test]
fn parse_retry_after_x_ratelimit_reset() {
    let mut headers = reqwest::header::HeaderMap::new();
    let future_ts = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 60) as f64;
    headers.insert("x-ratelimit-reset", future_ts.to_string().parse().unwrap());
    let ms = parse_retry_after(&headers);
    let ms = ms.expect("should parse x-ratelimit-reset");
    assert!(
        (55_000..=65_000).contains(&ms),
        "expected ~60000ms, got {ms}"
    );
}

#[test]
fn parse_retry_after_prefers_retry_after_over_ratelimit() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "10".parse().unwrap());
    headers.insert("x-ratelimit-reset", "9999999999".parse().unwrap());
    assert_eq!(parse_retry_after(&headers), Some(10_000));
}

#[test]
fn parse_retry_after_invalid_value_returns_none() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "not-a-number".parse().unwrap());
    assert_eq!(parse_retry_after(&headers), None);
}

// ---------------------------------------------------------------------------
// Exponential backoff calculation
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// RetryConfig
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// MockProvider error injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn mock_provider_returns_error_response() {
    let error = ProviderError::RateLimited {
        retry_after_ms: Some(5000),
    };
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![
            MockResponse::Error(error),
            MockResponse::Events(test_support::text_response("success")),
        ],
    );
    let request = opi_ai::provider::Request {
        model: "mock-model".into(),
        system: None,
        messages: vec![],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    };
    use futures_util::StreamExt;
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let first = stream.next().await.unwrap();
    assert!(first.is_err(), "first call should return error");
    match first.unwrap_err() {
        ProviderError::RateLimited { retry_after_ms } => {
            assert_eq!(retry_after_ms, Some(5000));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// ProviderError retryability
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HTTP-date parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_http_date_valid_future_date() {
    // Construct a date ~120 seconds from now
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 120;
    let date_str = unix_to_http_date(future);
    let delay = parse_http_date_delay(&date_str).expect("should parse future HTTP-date");
    assert!(
        (110_000..=130_000).contains(&delay),
        "expected ~120000ms, got {delay}"
    );
}

#[test]
fn parse_http_date_past_returns_none() {
    // 01 Jan 2020 is in the past
    let date_str = "Wed, 01 Jan 2020 00:00:00 GMT";
    assert_eq!(parse_http_date_delay(date_str), None);
}

#[test]
fn parse_http_date_invalid_format_returns_none() {
    assert_eq!(parse_http_date_delay("not a date"), None);
    assert_eq!(parse_http_date_delay("Fri May 23 12:00:00 2026"), None);
    assert_eq!(parse_http_date_delay("Fri, 23 May 2026"), None);
}

#[test]
fn parse_http_date_wrong_timezone_returns_none() {
    assert_eq!(parse_http_date_delay("Fri, 23 May 2026 12:00:00 EST"), None);
}

#[test]
fn parse_retry_after_with_http_date_header() {
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 30;
    let date_str = unix_to_http_date(future);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", date_str.parse().unwrap());
    let ms = parse_retry_after(&headers);
    let ms = ms.expect("should parse HTTP-date Retry-After header");
    assert!(
        (25_000..=35_000).contains(&ms),
        "expected ~30000ms, got {ms}"
    );
}

// Helper: convert Unix timestamp to IMF-fixdate string.
fn unix_to_http_date(ts: u64) -> String {
    let days = ts / 86400;
    let time_secs = ts % 86400;
    let hour = time_secs / 3600;
    let minute = (time_secs % 3600) / 60;
    let second = time_secs % 60;

    // Civil date from days since epoch (Howard Hinnant algorithm)
    let z = days as i64 + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    let weekday = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days % 7) as usize];
    let month_name = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][m as usize];

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday, d, month_name, y, hour, minute, second
    )
}
