//! Header admission, per-call transforms, and protected request identity.

use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::provider::{Provider, ProviderError, validate_extra_headers};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{accepted_failure_cause, drain, header_value, make_anthropic_request};

fn anthropic_sse_ok() -> String {
    "event: message_start\r\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"claude\",\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\r\n\r\nevent: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n".into()
}

#[test]
fn extra_headers_reject_auth_header_names() {
    let auth_names = [
        "authorization",
        "Authorization",
        "AUTHORIZATION",
        "x-api-key",
        "X-Api-Key",
        "api-key",
        "Api-Key",
        "anthropic-version",
        "Anthropic-Version",
        "content-type",
        "Content-Type",
    ];
    for name in &auth_names {
        let headers = vec![(name.to_string(), "secret".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("auth") || msg.contains("reserved") || msg.contains("forbidden"),
            "expected auth rejection for header '{name}', got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_control_characters_in_name() {
    for bad in &["X-\nHeader", "X-\rHeader", "X:\tHeader"] {
        let headers = vec![(bad.to_string(), "value".into())];
        let err = validate_extra_headers(&headers).unwrap_err();
        assert!(
            matches!(err, ProviderError::RequestFailed(_)),
            "expected RequestFailed for header {bad:?}, got: {err}"
        );
    }
}

#[test]
fn extra_headers_reject_empty_name() {
    let headers = vec![("".to_string(), "value".into())];
    let err = validate_extra_headers(&headers).unwrap_err();
    assert!(matches!(err, ProviderError::RequestFailed(_)));
}

#[test]
fn extra_headers_accept_valid_names() {
    let headers = vec![
        ("X-Custom".to_string(), "val".into()),
        ("x-request-id".to_string(), "abc123".into()),
        ("X-Trace-Id".to_string(), "t-1".into()),
    ];
    validate_extra_headers(&headers).unwrap();
}

#[tokio::test]
async fn extra_headers_reach_anthropic_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("X-Custom", "my-value"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "event: message_stop\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n",
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    let cancel = CancellationToken::new();
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(cancel, None);
    request.extra_headers = vec![("X-Custom".to_string(), "my-value".to_string())];

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(item) = stream.next().await {
        match item {
            Ok(_) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}

#[tokio::test]
async fn sdk_headers_omission_set_and_removal_reach_anthropic_wire() {
    let server = MockServer::start().await;
    let body = anthropic_sse_ok();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.extra_headers = vec![
        ("X-Set".to_string(), "keep".to_string()),
        ("X-Drop".to_string(), "gone".to_string()),
    ];
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(|merged| {
        let mut out: Vec<(String, String)> = merged
            .iter()
            .filter(|(name, _)| !name.eq_ignore_ascii_case("x-drop"))
            .cloned()
            .collect();
        out.push(("X-Added".to_string(), "by-transform".to_string()));
        Ok(out)
    }));

    drain(provider.stream_prepared(request, opi_ai::test_support::resolved_auth())).await;

    let sent = &server.received_requests().await.unwrap()[0];
    // Omission: an unset header never reaches the wire.
    assert!(header_value(sent, "x-opi-never").is_none());
    // Set and transform-added headers reach the wire; removed ones do not.
    assert_eq!(header_value(sent, "x-set"), Some("keep"));
    assert!(header_value(sent, "x-drop").is_none());
    assert_eq!(header_value(sent, "x-added"), Some("by-transform"));
    // Protected selected identity stays attached and unchanged.
    assert_eq!(header_value(sent, "x-api-key"), Some("test-key"));
    server.verify().await;
}

#[tokio::test]
async fn header_transform_touching_protected_header_fails_typed_before_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(|merged| {
        let mut out = merged.to_vec();
        out.push(("Authorization".to_string(), "Bearer attacker".to_string()));
        Ok(out)
    }));

    let mut stream =
        provider.stream_prepared(request.clone(), opi_ai::test_support::resolved_auth());
    let mut saw_callback_phase = false;
    while let Some(item) = stream.next().await {
        if let Err(ref error) = item {
            assert!(matches!(
                accepted_failure_cause(error, &request),
                ProviderError::CallbackPhase(_)
            ));
            saw_callback_phase = true;
        }
    }
    assert!(
        saw_callback_phase,
        "transform touching a protected header must fail typed"
    );
    server.verify().await;
}

#[test]
fn header_transform_failure_is_typed_functional_failure() {
    // Unit-level: the transform's CallbackError becomes a typed
    // CallbackPhase error, not a silent ignore and not a retryable cause.
    let error = ProviderError::CallbackPhase(opi_ai::provider::ProviderErrorSummary::sanitized(
        "header transform failed",
    ));
    assert!(!error.is_retryable());
    assert_eq!(
        error.category(),
        opi_ai::provider::ProviderErrorCategory::Request
    );
}

#[tokio::test]
async fn transform_runs_once_per_dispatch_on_the_merged_per_call_set() {
    let server = MockServer::start().await;
    let body = anthropic_sse_ok();
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    type HeaderSets = Arc<Mutex<Vec<Vec<(String, String)>>>>;
    let seen_inputs: HeaderSets = Arc::new(Mutex::new(Vec::new()));
    let recorder = Arc::clone(&seen_inputs);
    let provider = opi_ai::providers::anthropic::AnthropicProvider::new(Some(server.uri()));
    let mut request = make_anthropic_request(CancellationToken::new(), None);
    request.extra_headers = vec![("X-Seed".to_string(), "1".to_string())];
    request.callbacks = RequestCallbacks::new().with_header_transform(Arc::new(move |merged| {
        recorder.lock().unwrap().push(merged.to_vec());
        Ok(merged.to_vec())
    }));

    drain(provider.stream_prepared(request, opi_ai::test_support::resolved_auth())).await;

    let seen = seen_inputs.lock().unwrap();
    assert_eq!(seen.len(), 1, "transform applies exactly once per dispatch");
    assert_eq!(
        seen[0],
        vec![("X-Seed".to_string(), "1".to_string())],
        "transform sees the merged per-call set, never the protected auth headers"
    );
}

#[test]
fn header_transform_cannot_introduce_protected_route_identity() {
    for name in [
        "Host",
        "X-Goog-Api-Key",
        "X-Goog-User-Project",
        "X-Amz-Date",
        "X-Amz-Security-Token",
    ] {
        let callbacks = RequestCallbacks::new()
            .with_header_transform(Arc::new(move |_| Ok(vec![(name.into(), "changed".into())])));
        assert!(
            matches!(
                callbacks.apply_header_transform(&[]),
                Err(ProviderError::CallbackPhase(_))
            ),
            "protected transform header {name} must fail"
        );
    }
}

#[test]
fn header_identity_transform_preserves_existing_protected_values() {
    let original = vec![
        ("X-Initiator".into(), "user".into()),
        ("x-custom".into(), "value".into()),
    ];
    let callbacks =
        RequestCallbacks::new().with_header_transform(Arc::new(|headers| Ok(headers.to_vec())));
    assert_eq!(
        callbacks.apply_header_transform(&original).unwrap(),
        original
    );
    let callbacks = RequestCallbacks::new().with_header_transform(Arc::new(|_| Ok(vec![])));
    assert!(matches!(
        callbacks.apply_header_transform(&original),
        Err(ProviderError::CallbackPhase(_))
    ));
}
