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
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
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
            transport: Default::default(),
            model: "openai:gpt-4o".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: None,
            temperature: None,
            sampling_params: Default::default(),
            thinking: Default::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            callbacks: Default::default(),
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

    #[tokio::test]
    async fn real_adapter_unpolled_attempt_does_not_dispatch() {
        let server = MockServer::start().await;
        mount_success(&server).await;
        let models = real_adapter_models(&server.uri()).await;
        let call = models
            .prepare_call("openai:gpt-4o", chat_request())
            .await
            .unwrap();
        let attempt = call.start_attempt().unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "an unpolled attempt must not start the real adapter producer"
        );
        drop(attempt);
        assert_eq!(call.acquisition_seal(), None);
        drain_attempt(call.start_attempt().unwrap()).await.unwrap();
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn real_adapter_drop_after_send_seals_unknown_dispatch() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(5)))
            .mount(&server)
            .await;
        let models = real_adapter_models(&server.uri()).await;
        let call = models
            .prepare_call("openai:gpt-4o", chat_request())
            .await
            .unwrap();
        let mut attempt = call.start_attempt().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            tokio::select! {
                result = attempt.next() => panic!("response should remain pending: {result:?}"),
                _ = async {
                    while server.received_requests().await.unwrap().is_empty() {
                        tokio::task::yield_now().await;
                    }
                } => {}
            }
        })
        .await
        .unwrap();
        drop(attempt);
        assert_eq!(
            call.acquisition_seal(),
            Some(AcquisitionSeal::UnknownDispatch)
        );
        assert!(matches!(
            call.start_attempt(),
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::UnknownDispatch
            })
        ));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn successful_response_head_seals_retry_before_the_first_event() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uri = format!("http://{}", listener.local_addr().unwrap());
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = socket.read(&mut buffer).await.unwrap();
                assert_ne!(n, 0);
                bytes.extend_from_slice(&buffer[..n]);
                if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&bytes[..at])
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if bytes.len() >= at + 4 + length {
                        break;
                    }
                }
            }
            socket.write_all(b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n").await.unwrap();
            socket.flush().await.unwrap();
            assert!(matches!(socket.read(&mut buffer).await, Ok(0) | Err(_)));
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(75), listener.accept())
                    .await
                    .is_err()
            );
        });
        let models = real_adapter_models(&uri).await;
        let mut request = chat_request();
        request.timeout = Some(std::time::Duration::from_millis(200));
        let call = models
            .prepare_call_with_policy(
                "openai:gpt-4o",
                request,
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .unwrap();
        let failure = ChatStream::new(call.start_attempt().unwrap())
            .result()
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Timeout));
        assert!(!failure.message().usage.reported);
        assert_eq!(call.acquisition_seal(), Some(AcquisitionSeal::PostHandoff));
        assert!(call.retry_admission().is_none());
        assert!(matches!(
            call.start_attempt(),
            Err(ModelsError::CallSealed {
                seal: AcquisitionSeal::PostHandoff
            })
        ));
        peer.await.unwrap();
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

    #[tokio::test]
    async fn cancellation_after_reserving_an_unpolled_lease_never_starts_the_adapter() {
        let (models, log) = mock_models(vec![MockResponse::Events(text_response("unused"))]);
        let request = mock_request();
        let cancel = request.cancel.clone();
        let call = models
            .prepare_call("mock:mock-model", request)
            .await
            .unwrap();
        let mut attempt = call.start_attempt().unwrap();
        cancel.cancel();
        assert!(matches!(
            attempt.next().await,
            Some(Err(ProviderError::Cancelled))
        ));
        assert!(attempt.next().await.is_none());
        assert!(log.lock().unwrap().is_empty());
        assert!(matches!(
            call.start_attempt(),
            Err(ModelsError::CallCancelled)
        ));
    }

    #[tokio::test]
    async fn polling_a_closed_attempt_does_not_seal_the_admitted_retry() {
        let (models, log) = mock_models(vec![
            MockResponse::Error(ProviderError::RateLimited {
                retry_after_ms: Some(0),
            }),
            MockResponse::Events(text_response("recovered")),
        ]);
        let call = models
            .prepare_call_with_policy(
                "mock:mock-model",
                mock_request(),
                AcquisitionPolicy::with_max_retries(1),
            )
            .await
            .unwrap();
        let mut old_attempt = call.start_attempt().unwrap();
        assert!(old_attempt.next().await.unwrap().is_err());
        let next_attempt = call.start_attempt().unwrap();
        assert!(old_attempt.next().await.is_none());
        assert!(old_attempt.next().await.is_none());
        assert!(call.acquisition_seal().is_none());
        drain_attempt(next_attempt).await.unwrap();
        assert_eq!(log.lock().unwrap().len(), 2);
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

// ---------------------------------------------------------------------------
// Task 20.20: AWS Smithy V2 retry modes (standard/adaptive/Smithy2026).
// ---------------------------------------------------------------------------

mod smithy {
    use opi_ai::bedrock::retry::{
        AdaptiveRateLimiter, AwsRetryConfigError, MaxAttemptsSource, ScopedRetryInputs,
        SmithyClock, SmithyErrorClass, SmithyRetryConfig, SmithyRetryMode, SmithyRetryQuota,
        classify_service_failure, parse_smithy_retry_after, read_profile_retry_entries,
        resolve_smithy_retry_config, smithy_backoff_delay_ms, smithy_capacity_cost,
    };
    use opi_ai::provider::{ProviderError, ProviderErrorSummary};
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
        use opi_ai::bedrock::retry::classify_attempt_failure;
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
}
