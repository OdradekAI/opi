//! Request scalar defaults, explicit values, and timeout outcomes on the wire.

use std::time::Duration;

use futures_util::StreamExt;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{accepted_failure_cause, make_anthropic_request};

#[test]
fn cache_retention_default_is_none() {
    let cr = CacheRetention::default();
    assert_eq!(cr, CacheRetention::None);
}

#[test]
fn cache_retention_variants_exist() {
    let none = CacheRetention::None;
    let disabled = CacheRetention::Disabled;
    let short = CacheRetention::Short;
    let long = CacheRetention::Long;

    assert_ne!(none, disabled);
    assert_ne!(none, short);
    assert_ne!(none, long);
    assert_ne!(disabled, short);
    assert_ne!(disabled, long);
    assert_ne!(short, long);
}

#[test]
fn request_new_scalars_default_to_none_empty() {
    let cancel = CancellationToken::new();
    let req = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    assert!(req.timeout.is_none());
    assert!(req.extra_headers.is_empty());
    assert_eq!(req.cache_retention, CacheRetention::None);
    assert!(req.session_id.is_none());
}

#[test]
fn request_scalars_carry_explicit_values() {
    let cancel = CancellationToken::new();
    let timeout = Duration::from_secs(30);
    let headers = vec![("X-Test".to_string(), "value".to_string())];
    let req = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel,
        timeout: Some(timeout),
        extra_headers: headers.clone(),
        cache_retention: CacheRetention::Long,
        session_id: Some("sess-abc".into()),
        telemetry_context: None,
        callbacks: Default::default(),
    };
    assert_eq!(req.timeout, Some(Duration::from_secs(30)));
    assert_eq!(req.extra_headers, vec![("X-Test".into(), "value".into())]);
    assert_eq!(req.cache_retention, CacheRetention::Long);
    assert_eq!(req.session_id.as_deref(), Some("sess-abc"));
}

#[tokio::test]
async fn provider_timeout_produces_timeout_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let request = make_anthropic_request(cancel, Some(Duration::from_millis(100)));

    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());
    let item = stream.next().await.unwrap();
    match item
        .as_ref()
        .map_err(|error| accepted_failure_cause(error, &request))
    {
        Err(ProviderError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(stream.next().await.is_none(), "timeout must be terminal");
}

#[tokio::test]
async fn provider_no_timeout_completes_normally() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let request = make_anthropic_request(cancel, None);

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    // Drain until we get a Done event (or error).
    let mut saw_done = false;
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => saw_done = true,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
    assert!(saw_done, "stream should produce at least one event");
}

#[test]
fn request_sampling_params_default_absent_and_explicit_maps_are_carried() {
    let req = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "test".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        sampling_params: Default::default(),
    };
    assert!(req.sampling_params.is_none());

    let params: opi_ai::model_info::SamplingParams = serde_json::json!({"top_p": 0.9})
        .as_object()
        .cloned()
        .unwrap();
    let mut req = req;
    req.sampling_params = Some(Default::default());
    assert_eq!(req.sampling_params, Some(Default::default()));
    req.sampling_params = Some(params);
    assert_eq!(
        req.sampling_params.as_ref().unwrap()["top_p"],
        serde_json::json!(0.9)
    );
}
