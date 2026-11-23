//! AWS Smithy configuration, failure classification, jitter, quota and rate limiting.

use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use opi_ai::retry::smithy::{
    AdaptiveRateLimiter, AwsRetryConfigError, MaxAttemptsSource, ScopedRetryInputs, SmithyClock,
    SmithyErrorClass, SmithyRetryConfig, SmithyRetryMode, SmithyRetryQuota,
    classify_service_failure, parse_smithy_retry_after, read_profile_retry_entries,
    resolve_smithy_retry_config, smithy_backoff_delay_ms, smithy_capacity_cost,
};
use opi_ai::retry::{AcquisitionPolicy, RetrySourcePolicy};

#[test]
fn config_defaults_standard_legacy_three() {
    let config = resolve_smithy_retry_config(ScopedRetryInputs::default())
        .expect("empty inputs resolve defaults");
    assert_eq!(config, SmithyRetryConfig::default());
    assert_eq!(config.mode, SmithyRetryMode::Standard);
    assert!(!config.smithy_2026);
    assert_eq!(config.max_attempts, 3, "source default is 3 total attempts");
}

#[test]
fn config_env_wins_over_profile_then_default() {
    let config = resolve_smithy_retry_config(ScopedRetryInputs {
        env_retry_mode: Some("adaptive"),
        env_max_attempts: Some("5"),
        profile_retry_mode: Some("standard"),
        profile_max_attempts: Some("7"),
    })
    .expect("resolve");
    assert_eq!(config.mode, SmithyRetryMode::Adaptive);
    assert_eq!(config.max_attempts, 5);

    let profile_only = resolve_smithy_retry_config(ScopedRetryInputs {
        env_retry_mode: None,
        env_max_attempts: None,
        profile_retry_mode: Some("adaptive"),
        profile_max_attempts: Some("9"),
    })
    .expect("resolve");
    assert_eq!(profile_only.mode, SmithyRetryMode::Adaptive);
    assert_eq!(profile_only.max_attempts, 9);
}

#[test]
fn config_unknown_mode_fails_typed_never_silent_standard() {
    let error = resolve_smithy_retry_config(ScopedRetryInputs {
        env_retry_mode: Some("experimental-v3"),
        ..ScopedRetryInputs::default()
    })
    .expect_err("unknown mode must fail");
    assert_eq!(
        error,
        AwsRetryConfigError::UnknownMode("experimental-v3".to_owned())
    );
    let from_profile = resolve_smithy_retry_config(ScopedRetryInputs {
        profile_retry_mode: Some("legacy"),
        ..ScopedRetryInputs::default()
    })
    .expect_err("unknown profile mode must fail");
    assert_eq!(
        from_profile,
        AwsRetryConfigError::UnknownMode("legacy".into())
    );
}

#[test]
fn config_bad_max_attempts_fails_typed() {
    let error = resolve_smithy_retry_config(ScopedRetryInputs {
        env_max_attempts: Some("soon"),
        ..ScopedRetryInputs::default()
    })
    .expect_err("unparsable attempts must fail");
    assert_eq!(
        error,
        AwsRetryConfigError::InvalidMaxAttempts {
            source: MaxAttemptsSource::ScopedEnvironment,
            value: "soon".to_owned(),
        }
    );
    let profile_error = resolve_smithy_retry_config(ScopedRetryInputs {
        profile_max_attempts: Some("many"),
        ..ScopedRetryInputs::default()
    })
    .expect_err("unparsable profile attempts must fail");
    assert_eq!(
        profile_error,
        AwsRetryConfigError::InvalidMaxAttempts {
            source: MaxAttemptsSource::SharedProfile,
            value: "many".to_owned(),
        }
    );
}

#[test]
fn config_max_attempts_parses_leading_integer_like_source() {
    // The source's parseInt semantics: "3x" is 3; empty values skip.
    let config = resolve_smithy_retry_config(ScopedRetryInputs {
        env_max_attempts: Some("3x"),
        ..ScopedRetryInputs::default()
    })
    .expect("leading digits parse");
    assert_eq!(config.max_attempts, 3);
    let skipped = resolve_smithy_retry_config(ScopedRetryInputs {
        env_max_attempts: Some("  "),
        profile_max_attempts: Some("2"),
        ..ScopedRetryInputs::default()
    })
    .expect("empty env value skips to the profile");
    assert_eq!(skipped.max_attempts, 2);
}

#[test]
fn mode_parse_accepts_only_published_names() {
    assert_eq!(
        SmithyRetryMode::parse("standard"),
        Ok(SmithyRetryMode::Standard)
    );
    assert_eq!(
        SmithyRetryMode::parse(" adaptive "),
        Ok(SmithyRetryMode::Adaptive)
    );
    assert!(SmithyRetryMode::parse("ADAPTIVE").is_err());
}

#[test]
fn classification_table_matches_source() {
    use SmithyErrorClass::*;
    let cases: &[(u16, Option<&str>, SmithyErrorClass)] = &[
        (429, None, Throttling),
        (429, Some("ThrottlingException"), Throttling),
        (400, Some("ThrottlingException"), Throttling),
        (429, Some("RequestThrottled"), Throttling),
        (500, Some("InternalServerException"), Transient),
        (500, None, Transient),
        (502, None, Transient),
        (503, Some("ServiceUnavailableException"), Transient),
        (504, None, Transient),
        (400, Some("RequestTimeout"), Transient),
        (408, Some("TimeoutError"), Transient),
        (400, Some("ModelNotReadyException"), Transient),
        (408, Some("ModelTimeoutException"), ClientError),
        (501, None, ServerError),
        (505, None, ServerError),
        (566, None, ServerError),
        // The name alone is not transient: only status 500 (and the
        // name list above) classifies InternalServerException.
        (501, Some("InternalServerException"), ServerError),
        (400, Some("ValidationException"), ClientError),
        (409, Some("ConflictException"), ClientError),
        (403, Some("AccessDeniedException"), ClientError),
        (402, Some("ServiceQuotaExceededException"), ClientError),
        (500, Some("ModelTimeoutException"), Transient),
    ];
    for (status, name, expected) in cases {
        assert_eq!(
            classify_service_failure(*status, *name),
            *expected,
            "status {status} name {name:?}"
        );
    }
}

fn sanitized_network() -> ProviderError {
    ProviderError::Network(ProviderErrorSummary::from_untrusted("fixture"))
}

#[test]
fn transport_classification_maps_timeout_network_rate_limited() {
    use SmithyErrorClass::*;
    use opi_ai::retry::smithy::classify_attempt_failure;
    assert_eq!(classify_attempt_failure(&ProviderError::Timeout), Transient);
    assert_eq!(classify_attempt_failure(&sanitized_network()), Transient);
    assert_eq!(
        classify_attempt_failure(&ProviderError::RateLimited {
            retry_after_ms: None
        }),
        Throttling
    );
    assert_eq!(
        classify_attempt_failure(&ProviderError::AuthFailed(
            ProviderErrorSummary::from_untrusted("denied")
        )),
        ClientError
    );
    assert_eq!(
        classify_attempt_failure(&ProviderError::Cancelled),
        ClientError
    );
}

#[test]
fn backoff_exact_values_for_seeded_unit() {
    // V-05 reference vectors: U=0.5 first transient retry is 50ms.
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, None, 0.5),
        50
    );
    assert_eq!(
        smithy_backoff_delay_ms(1, SmithyErrorClass::Transient, false, None, 0.5),
        100
    );
    // Cap at 20s: 100ms * 2^10 = 102400 > cap.
    assert_eq!(
        smithy_backoff_delay_ms(10, SmithyErrorClass::Transient, false, None, 0.5),
        10_000
    );
    assert_eq!(
        smithy_backoff_delay_ms(10, SmithyErrorClass::Transient, false, None, 1.0),
        20_000
    );
    // Throttling base is 500ms legacy / 1000ms Smithy2026; transient
    // base is 100ms legacy / 50ms 2026.
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Throttling, false, None, 0.5),
        250
    );
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Throttling, true, None, 0.5),
        500
    );
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, true, None, 0.5),
        25
    );
}

#[test]
fn backoff_hint_bounded_to_five_second_headroom() {
    // delay=50, hint 10s -> 5050 (V-05); 90s hint -> still 5050.
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, Some(10_000), 0.5),
        5_050
    );
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, Some(90_000), 0.5),
        5_050
    );
    // Smaller hints win over the backoff but stay under the headroom.
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, Some(200), 0.5),
        200
    );
    // Zero/past hints never shorten the computed delay.
    assert_eq!(
        smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, Some(0), 0.5),
        50
    );
}

#[test]
fn quota_costs_and_refunds_per_flavor() {
    assert_eq!(smithy_capacity_cost(SmithyErrorClass::Throttling, false), 5);
    assert_eq!(smithy_capacity_cost(SmithyErrorClass::Transient, false), 10);
    assert_eq!(smithy_capacity_cost(SmithyErrorClass::Throttling, true), 5);
    assert_eq!(smithy_capacity_cost(SmithyErrorClass::Transient, true), 14);

    // 500 tokens at cost 5: exactly one hundred transient-free throttling
    // retries exhaust the per-call quota.
    let mut quota = SmithyRetryQuota::default();
    assert_eq!(quota.capacity(), 500);
    for _ in 0..100 {
        assert!(quota.has_capacity(5));
        quota.acquire(5);
    }
    assert_eq!(quota.capacity(), 0);
    assert!(!quota.has_capacity(5), "quota exhaustion refuses admission");
    // A success refunds the last cost, capped at the initial capacity.
    quota.refund(Some(5));
    assert_eq!(quota.capacity(), 5);
    quota.refund(None);
    assert_eq!(quota.capacity(), 6, "no-retry increment is 1");
}

#[test]
fn hint_headers_parse_seconds_date_and_amz_milliseconds() {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("retry-after", "5".parse().unwrap());
    assert_eq!(parse_smithy_retry_after(&headers, 1_000), Some(5_000));

    let mut date_headers = reqwest::header::HeaderMap::new();
    date_headers.insert(
        "retry-after",
        "Fri, 23 May 2026 12:00:10 GMT".parse().unwrap(),
    );
    assert_eq!(
        parse_smithy_retry_after(&date_headers, 1_780_000_000),
        None,
        "unparseable-future IMF date (not 2026) yields none"
    );
    let mut valid_date = reqwest::header::HeaderMap::new();
    valid_date.insert(
        "retry-after",
        "Sat, 10 Jan 2026 12:00:10 GMT".parse().unwrap(),
    );
    // Sat, 10 Jan 2026 12:00:10 GMT = 1768051210.
    assert_eq!(
        parse_smithy_retry_after(&valid_date, 1_768_046_400),
        Some(10_000),
        "IMF-fixdate delta in milliseconds"
    );

    // Trailing " GMT, <seconds>" form and x-amz-retry-after milliseconds.
    let mut suffixed = reqwest::header::HeaderMap::new();
    suffixed.insert(
        "retry-after",
        "Wed, 21 Oct 2026 07:28:00 GMT, 120".parse().unwrap(),
    );
    assert_eq!(parse_smithy_retry_after(&suffixed, 0), Some(120_000));

    let mut amz = reqwest::header::HeaderMap::new();
    amz.insert("x-amz-retry-after", "250".parse().unwrap());
    assert_eq!(parse_smithy_retry_after(&amz, 0), Some(250));

    // An unparsable retry-after falls through to x-amz-retry-after like
    // the source's per-header loop.
    let mut both = reqwest::header::HeaderMap::new();
    both.insert("retry-after", "tomorrow".parse().unwrap());
    both.insert("x-amz-retry-after", "500".parse().unwrap());
    assert_eq!(parse_smithy_retry_after(&both, 0), Some(500));
}

#[test]
fn profile_reader_extracts_retry_entries() {
    let text = "[default]\nretry_mode = adaptive\nmax_attempts = 4\n\n[profile corp]\nretry_mode = standard\nmax_attempts = 8\n";
    let (mode, attempts) = read_profile_retry_entries(text, None);
    assert_eq!(mode.as_deref(), Some("adaptive"));
    assert_eq!(attempts.as_deref(), Some("4"));
    let (mode, attempts) = read_profile_retry_entries(text, Some("corp"));
    assert_eq!(mode.as_deref(), Some("standard"));
    assert_eq!(attempts.as_deref(), Some("8"));
    let (mode, attempts) = read_profile_retry_entries(text, Some("missing"));
    assert_eq!(mode, None);
    assert_eq!(attempts, None);
}

#[test]
fn smithy_policy_maps_total_attempts_to_extra_sends() {
    let policy = SmithyRetryConfig {
        mode: SmithyRetryMode::Adaptive,
        smithy_2026: true,
        max_attempts: 5,
    }
    .acquisition_policy();
    assert_eq!(policy.max_retries, 4, "5 total attempts = 4 extra sends");
    assert!(matches!(policy.source, RetrySourcePolicy::Smithy(_)));
    assert_eq!(
        AcquisitionPolicy::default().source,
        RetrySourcePolicy::Helper
    );
}

#[test]
fn seeded_policy_preview_matches_first_unit_draw() {
    for seed in [0u64, 1, 42, 999] {
        let unit = opi_ai::retry::smithy_unit_jitter_preview(seed);
        assert!(
            (0.0..1.0).contains(&unit),
            "unit jitter must stay in [0,1), got {unit}"
        );
        assert_eq!(
            smithy_backoff_delay_ms(0, SmithyErrorClass::Transient, false, None, unit),
            (unit * 100.0).floor() as u64
        );
    }
}

#[test]
fn adaptive_limiter_disabled_until_first_throttle() {
    let mut limiter = AdaptiveRateLimiter::new(SmithyClock::manual([100.0]));
    assert!(!limiter.is_enabled());
    assert_eq!(
        limiter.send_token_wait_ms(100.0),
        0,
        "pre-throttle sends are free"
    );
    assert_eq!(
        limiter.available_tokens(),
        0.0,
        "free sends consume nothing"
    );
    // A success outcome never enables the bucket.
    limiter.update_client_sending_rate(false);
    assert!(!limiter.is_enabled());
}

#[test]
fn adaptive_limiter_cut_and_recovery_math() {
    // Scripted clock: construction consumes two ticks (throttle time and
    // bucket floor), then one tick per update_measured_rate and one per
    // update_token_bucket_rate per outcome.
    let mut limiter = AdaptiveRateLimiter::new(SmithyClock::manual([
        0.0, 0.0, // construction
        0.5, 0.5, // first outcome (throttle): bucket advances -> rate measured
        10.0, 10.0, // second outcome (success): cubic recovery
    ]));
    // Seed a measured rate: each success outcome consumes one
    // update_measured_rate tick and one update_token_bucket_rate tick.
    // Update 1 (t=0.5): the bucket advanced from 0, so
    // measured = 0.8 * (1 / 0.5) = 1.6. Updates 2-5 freeze at t=10
    // (repeated final tick), so only update 2 advances the bucket:
    // measured = 0.8 * (1 / 9.5) + 0.2 * 1.6 = 0.40421053.
    for _ in 0..5 {
        limiter.update_client_sending_rate(false);
    }
    // First throttle: cut to beta * measured = 0.28294737, then floor at
    // the minimum fill rate 0.5 and enable the bucket.
    limiter.update_client_sending_rate(true);
    assert!(limiter.is_enabled());
    assert_eq!(limiter.measured_tx_rate(), 0.40421053);
    assert_eq!(limiter.fill_rate(), 0.5, "min fill rate floors the cut");
    // A send at t=10.5 first refills half a second at 0.5/s (0.25
    // tokens), so the deficit is 0.75 tokens: a 1.5s wait.
    let wait = limiter.send_token_wait_ms(10.5);
    assert_eq!(wait, 1_500, "deficit 0.75 tokens at 0.5/s is a 1.5s wait");
}
