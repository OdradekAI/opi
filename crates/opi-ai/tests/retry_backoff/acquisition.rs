//! Prepared-call acquisition through registered providers and OpenAI Chat peers.

use std::sync::Arc;

use futures_util::StreamExt;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::HttpClientBuilder;
use opi_ai::message::AssistantMessage;
use opi_ai::provider::{CacheRetention, EventStream, Provider, ProviderError};
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::retry::{AcquisitionPolicy, AcquisitionSeal};
use opi_ai::stream::ChatStream;
use opi_ai::test_support::{self, MockProvider, MockResponse, text_response};
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
        projection: Default::default(),
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
        telemetry_context: None,
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
            opi_ai::message::AssistantContent::Text { text, .. } => Some(text.clone()),
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
        .prepare_call("mock:mock-model", request.clone())
        .await
        .unwrap();
    let mut attempt = call.start_attempt().unwrap();
    cancel.cancel();
    let error = attempt
        .next()
        .await
        .expect("one cancellation")
        .expect_err("cancelled lease must fail closed");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    assert_eq!(failure.cause(), &ProviderError::Cancelled);
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    use sha2::Digest as _;
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
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
        projection: Default::default(),
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
        telemetry_context: None,
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
