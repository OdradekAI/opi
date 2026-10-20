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
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
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
        _other => panic!("expected RateLimited"),
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

// ---------------------------------------------------------------------------
// Acquisition owner (task 20.4, AIR-007 / P20-ACQUISITION closure tests)
// ---------------------------------------------------------------------------

/// One executable acquisition attempt at a time; frozen auth/model/account;
/// per-payload-variant send counters; cancellable source-qualified waiting;
/// sealed post-handoff/unknown-dispatch states — proven through `Models`,
/// the real OpenAI Chat adapter where the wire matters, and registered mock
/// providers where the mechanism matters.
mod acquisition {
    use std::sync::Arc;

    use futures_util::StreamExt;
    use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
    use opi_ai::http::HttpClientBuilder;
    use opi_ai::message::AssistantMessage;
    use opi_ai::openai_chat::OpenAiChatProvider;
    use opi_ai::provider::{CacheRetention, EventStream, Provider, ProviderError};
    use opi_ai::retry::{AcquisitionPolicy, AcquisitionSeal};
    use opi_ai::stream::ChatStream;
    use opi_ai::test_support::{MockProvider, MockResponse, text_response};
    use opi_ai::{Models, ModelsError};
    use secrecy::SecretString;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SSE_SUCCESS: &str = concat!(
        "data: {\"id\":\"chatcmpl-r\",\"object\":\"chat.completion.chunk\",\"created\":1,",
        "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"recovered\"},",
        "\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-r\",\"object\":\"chat.completion.chunk\",\"created\":1,",
        "\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],",
        "\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    );

    async fn real_adapter_models(server_uri: &str) -> Models {
        let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
        let provider = OpenAiChatProvider::with_client(
            Some(server_uri.to_owned()),
            "openai".to_owned(),
            vec![],
            client,
        );
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    SecretString::from("acquisition-contract-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata {
                    openai_compatible: true,
                    profile: None,
                },
            )
            .expect("register real adapter route");
        models
    }

    fn chat_request() -> opi_ai::provider::Request {
        opi_ai::provider::Request {
            model: "openai:gpt-4o".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: None,
            temperature: None,
            thinking: Default::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
        }
    }

    async fn mount_429(server: &MockServer, times: usize, retry_after: Option<&'static str>) {
        let mut template = ResponseTemplate::new(429);
        if let Some(value) = retry_after {
            template = template.insert_header("retry-after", value);
        }
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(template)
            .up_to_n_times(times as u64)
            .mount(server)
            .await;
    }

    async fn mount_success(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(SSE_SUCCESS),
            )
            .mount(server)
            .await;
    }

    fn assistant_text(message: &AssistantMessage) -> String {
        message
            .content
            .iter()
            .filter_map(|content| match content {
                opi_ai::message::AssistantContent::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    async fn drain_attempt(stream: EventStream) -> Result<AssistantMessage, ProviderError> {
        ChatStream::new(stream)
            .result()
            .await
            .map_err(|failure| failure.into_parts().1)
    }

    /// One acquisition episode over the real OpenAI Chat adapter: the 429
    /// hint is honored as the source-qualified wait, an early start is
    /// refused with zero dispatch, the admitted retry reuses the frozen
    /// request and credential, and the episode performs exactly 1+N sends.
    #[tokio::test]
    async fn real_adapter_retry_honors_hint_enforces_readiness_and_freezes_payload() {
        let server = MockServer::start().await;
        mount_429(&server, 1, Some("0.1")).await; // 0.1s -> 100ms hint
        mount_success(&server).await;
        let models = real_adapter_models(&server.uri()).await;

        let prepared = models
            .prepare_call_with_policy(
                "openai:gpt-4o",
                chat_request(),
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .expect("prepare");

        let first = drain_attempt(prepared.start_attempt().expect("first attempt"))
            .await
            .expect_err("scripted 429");
        assert!(
            matches!(first, ProviderError::RateLimited { .. }),
            "expected RateLimited, got {first:?}"
        );

        let admission = prepared
            .retry_admission()
            .expect("retryable failure with budget admits the retry");
        assert_eq!(admission.retry_index, 1);
        assert_eq!(admission.delay_ms, 100, "server hint wins over backoff");

        // Early start: typed refusal with zero dispatch (the server has seen
        // exactly one request so far).
        match prepared.start_attempt() {
            Err(ModelsError::RetryNotReady { retry_ready_in_ms }) => {
                assert!(retry_ready_in_ms > 0 && retry_ready_in_ms <= 100);
            }
            _other => panic!("expected RetryNotReady, got a dispatched stream instead"),
        }
        assert_eq!(server.received_requests().await.unwrap().len(), 1);

        // Cancellable wait, then the admitted retry.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let message = drain_attempt(prepared.start_attempt().expect("admitted retry"))
            .await
            .expect("retry succeeds");

        // Frozen payload: both sends carried the identical body, model, and
        // the same frozen bearer credential (auth resolved once at prepare).
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2, "episode must perform exactly 1+N sends");
        let first_body = requests[0].body.clone();
        let retry_body = requests[1].body.clone();
        assert_eq!(
            first_body, retry_body,
            "retry must reuse the frozen payload"
        );
        for request in &requests {
            assert_eq!(
                request
                    .headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok()),
                Some("Bearer acquisition-contract-key"),
                "frozen auth must be reused across attempts"
            );
        }
        assert_eq!(
            message.response_id.as_deref(),
            Some("chatcmpl-r"),
            "the retry's response is delivered"
        );
    }

    /// Without a hint the SDK computes the helper-family backoff with
    /// jitter: `min(500ms * 2^i, 8000ms) * [0.75, 1.0)`.
    #[tokio::test]
    async fn real_adapter_retry_without_hint_uses_family_backoff_range() {
        let server = MockServer::start().await;
        mount_429(&server, 1, None).await;
        mount_success(&server).await;
        let models = real_adapter_models(&server.uri()).await;

        let prepared = models
            .prepare_call_with_policy(
                "openai:gpt-4o",
                chat_request(),
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .expect("prepare");

        let first = drain_attempt(prepared.start_attempt().expect("first attempt"))
            .await
            .expect_err("scripted 429");
        assert!(matches!(first, ProviderError::RateLimited { .. }));

        let admission = prepared
            .retry_admission()
            .expect("no-hint failure still admits the retry");
        assert_eq!(admission.retry_index, 1);
        assert!(
            (375..=500).contains(&admission.delay_ms),
            "index-0 family backoff must land in [375, 500]ms, got {}",
            admission.delay_ms
        );

        tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms + 20)).await;
        let message = drain_attempt(prepared.start_attempt().expect("admitted retry"))
            .await
            .expect("retry succeeds");
        assert_eq!(message.response_id.as_deref(), Some("chatcmpl-r"));
        assert_eq!(server.received_requests().await.unwrap().len(), 2);
    }

    /// The frozen per-variant budget caps the episode at 1+N sends; the
    /// exhausted call is sealed and refuses further starts.
    #[tokio::test]
    async fn real_adapter_budget_caps_sends_and_seals_exhausted() {
        let server = MockServer::start().await;
        mount_429(&server, 10, Some("0.001")).await; // 1ms hint keeps the test fast
        let models = real_adapter_models(&server.uri()).await;

        let prepared = models
            .prepare_call_with_policy(
                "openai:gpt-4o",
                chat_request(),
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .expect("prepare");

        drain_attempt(prepared.start_attempt().expect("send 1"))
            .await
            .expect_err("first 429");
        assert!(prepared.retry_admission().is_some(), "first failure admits");
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        drain_attempt(
            prepared
                .start_attempt()
                .expect("send 2 (the last budgeted)"),
        )
        .await
        .expect_err("second 429");

        assert!(
            prepared.retry_admission().is_none(),
            "budget of 1+1 sends is spent; no third send may be admitted"
        );
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::Exhausted),
            "the exhausted seal is observable"
        );
        match prepared.start_attempt() {
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::Exhausted,
            }) => {}
            _other => panic!("expected CallSealed(Exhausted), got a dispatched stream instead"),
        }
        assert_eq!(
            server.received_requests().await.unwrap().len(),
            2,
            "exactly 1+N sends hit the wire"
        );
    }

    // ------------------------------------------------------------------
    // Mechanism tests over registered mock providers
    // ------------------------------------------------------------------

    fn mock_models(
        responses: Vec<MockResponse>,
    ) -> (
        Models,
        Arc<std::sync::Mutex<Vec<opi_ai::provider::Request>>>,
    ) {
        let provider = MockProvider::new_with_errors("mock", responses);
        let handle = provider.call_log_handle();
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    SecretString::from("mechanism-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata::default(),
            )
            .expect("register route");
        (models, handle)
    }

    fn mock_request() -> opi_ai::provider::Request {
        let mut request = chat_request();
        request.model = "mock:mock-model".into();
        request
    }

    /// A retryable failure after streaming-response handoff is final: the
    /// post-handoff seal suppresses the remaining budget with zero further
    /// sends.
    #[tokio::test]
    async fn post_handoff_failure_is_sealed_not_retried() {
        let mut partial = text_response("partial content");
        partial.pop(); // Start + TextDelta, no Done
        let (models, sends) = mock_models(vec![MockResponse::EventsThenError(
            partial,
            ProviderError::RateLimited {
                retry_after_ms: Some(0),
            },
        )]);
        let prepared = models
            .prepare_call_with_policy(
                "mock:mock-model",
                mock_request(),
                AcquisitionPolicy::with_max_retries(3),
            )
            .await
            .expect("prepare");

        let failure = drain_attempt(prepared.start_attempt().expect("attempt"))
            .await
            .expect_err("mid-stream failure");
        assert!(matches!(
            failure,
            ProviderError::RateLimited { .. } | ProviderError::StreamFailure(_)
        ));

        assert!(
            prepared.retry_admission().is_none(),
            "post-handoff retryable failure must not admit another send"
        );
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::PostHandoff)
        );
        match prepared.start_attempt() {
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::PostHandoff,
            }) => {}
            _other => panic!("expected CallSealed(PostHandoff), got a dispatched stream instead"),
        }
        assert_eq!(
            sends.lock().unwrap().len(),
            1,
            "exactly one send: handoff sealed the episode"
        );
    }

    /// Dropping an attempt after dispatch without an observed closed
    /// outcome records the unknown effect and seals further scheduling;
    /// dropping before any poll releases the lease without sealing.
    #[tokio::test]
    async fn dispatched_drop_records_unknown_dispatch_and_seals() {
        let mut models = Models::new();
        models
            .register_route(
                Box::new(PendingProvider),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::ApiKey,
                    SecretString::from("mechanism-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata::default(),
            )
            .expect("register route");
        let mut request = mock_request();
        request.model = "pending:pending-model".into();
        let prepared = models
            .prepare_call("pending:pending-model", request)
            .await
            .expect("prepare");

        // Dropped before any poll: the lease releases without a send being
        // consumed and the call stays unsealed.
        {
            let _unpolled = prepared.start_attempt().expect("lease acquired");
        }
        // One synchronous poll proves dispatch without ever yielding an
        // item; the pending stream stays pending.
        let polled = {
            let mut attempt = prepared.start_attempt().expect("re-acquired lease");
            futures_util::FutureExt::now_or_never(futures_util::future::poll_fn(|cx| {
                StreamExt::poll_next_unpin(&mut attempt, cx)
            }))
        };
        assert!(polled.is_none(), "the pending provider dispatched no item");
        // The dispatched attempt handle (with its still-pending stream) drops
        // here without an observed outcome: unknown effect, sealed.
        // The dispatched attempt handle drops here without an observed
        // outcome: unknown effect, sealed.
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::UnknownDispatch),
            "dropping a dispatched attempt must record the unknown effect"
        );
        match prepared.start_attempt() {
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::UnknownDispatch,
            }) => {}
            _other => {
                panic!("expected CallSealed(UnknownDispatch), got a dispatched stream instead")
            }
        }
    }

    /// A successful terminal outcome seals the call: the reopenable
    /// successful lease is gone and a new logical call must re-prepare.
    #[tokio::test]
    async fn successful_terminal_seals_the_call() {
        let (models, sends) = mock_models(vec![MockResponse::Events(text_response("done"))]);
        let prepared = models
            .prepare_call("mock:mock-model", mock_request())
            .await
            .expect("prepare");

        let message = drain_attempt(prepared.start_attempt().expect("attempt"))
            .await
            .expect("success");
        assert_eq!(assistant_text(&message), "done");

        // The first delivered event sealed the call (sticky first reason);
        // success keeps it sealed against re-dispatch.
        assert_eq!(
            prepared.acquisition_seal(),
            Some(AcquisitionSeal::PostHandoff),
            "success must seal the call against re-dispatch"
        );
        match prepared.start_attempt() {
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::PostHandoff,
            }) => {}
            _other => panic!("expected CallSealed(PostHandoff), got a dispatched stream instead"),
        }
        assert_eq!(sends.lock().unwrap().len(), 1);

        // A stream that ends without delivering any event seals with the
        // finished reason instead (no handoff was ever observed).
        let (models, _sends) = mock_models(vec![MockResponse::Events(Vec::new())]);
        let prepared = models
            .prepare_call("mock:mock-model", mock_request())
            .await
            .expect("prepare");
        let _ = drain_attempt(prepared.start_attempt().expect("attempt"))
            .await
            .expect_err("empty stream fails with the synthesized protocol error");
        assert_eq!(prepared.acquisition_seal(), Some(AcquisitionSeal::Finished));
        match prepared.start_attempt() {
            Err(ModelsError::CallSealed { .. }) => {}
            _other => panic!("expected CallSealed after a finished empty stream"),
        }
    }

    /// Cancelling the shared token forbids any further attempt, including
    /// after an admitted retry.
    #[tokio::test]
    async fn cancelled_token_forbids_attempts_after_admission() {
        let request = mock_request();
        let token = request.cancel.clone();
        let (models, _sends) = mock_models(vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(10_000),
            }),
            MockResponse::Events(text_response("never reached")),
        ]);
        let prepared = models
            .prepare_call_with_policy(
                "mock:mock-model",
                request,
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .expect("prepare");

        drain_attempt(prepared.start_attempt().expect("attempt"))
            .await
            .expect_err("scripted rate limit");
        assert!(prepared.retry_admission().is_some());

        token.cancel();
        match prepared.start_attempt() {
            Err(ModelsError::CallCancelled) => {}
            _other => {
                panic!("expected CallCancelled after token cancel, got a dispatched stream instead")
            }
        }
    }

    /// A provider whose streams never resolve, for the unknown-dispatch
    /// mechanism test.
    struct PendingProvider;

    impl Provider for PendingProvider {
        fn id(&self) -> &str {
            "pending"
        }
        fn models(&self) -> &[opi_ai::model_info::ModelInfo] {
            static MODELS: std::sync::OnceLock<Vec<opi_ai::model_info::ModelInfo>> =
                std::sync::OnceLock::new();
            MODELS.get_or_init(|| {
                vec![opi_ai::model_info::ModelInfo::new(
                    "pending-model",
                    "Pending",
                    opi_ai::model_info::WireApi::OpenAiCompletions,
                    opi_ai::model_info::ModelCapabilities::new(100_000, 4_096),
                )]
            })
        }
        fn stream_prepared(
            &self,
            _request: opi_ai::provider::Request,
            _auth: opi_ai::auth::ResolvedAuth,
        ) -> EventStream {
            Box::pin(futures_util::stream::pending())
        }
    }
}
