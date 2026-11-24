//! Submission and explicit fetch through the public Models API.

use opi_ai::api::contract::DeferredFetchCall;
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
use opi_ai::stream::StopReason;
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use opi_ai::types::deferred::DeferredError;
use opi_ai::{AuthProvenanceSource, Models};
use secrecy::SecretString;

use super::fixtures::{chat_record, chat_request, pending_handle};

#[tokio::test]
async fn normal_generation_returns_pending_handle_or_final() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut models = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("final answer")])
            .with_deferred(std::sync::Arc::clone(&backend), 1, Some(5_000)),
    ));
    models
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register deferred api");

    // Deferred submission: the same ordinary simple entry returns a pending
    // terminal with the handle instead of final content.
    let pending = models
        .complete_simple(chat_record(&models), chat_request(), Default::default())
        .await
        .expect("pending submission is an Ok outcome");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert!(pending.content.is_empty());
    let handle = *pending.deferred.clone().expect("handle rides the message");
    assert_eq!(handle.provider, "test");
    assert_eq!(handle.model_id, "mock-model");
    assert_eq!(handle.api, "scripted-deferred");
    assert_eq!(handle.poll_after_ms, Some(5_000));
    assert_eq!(backend.fetch_count(), 0);

    // Without deferred scripting the same normal generation stays final.
    let plain = test_support::single_route_collection(Box::new(test_support::MockProvider::new(
        "test",
        vec![test_support::text_response("done")],
    )));
    let final_message = plain
        .complete_simple(chat_record(&plain), chat_request(), Default::default())
        .await
        .expect("final submission");
    assert_eq!(final_message.stop_reason, StopReason::Stop);
    assert!(final_message.deferred.is_none());
}

#[tokio::test]
async fn wait0_fetch_checks_once_and_may_stay_pending() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("ready")]),
    ));
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");

    // Script one submission with one pending fetch before readiness.
    let mut submission_events = test_support::text_response("ready");
    for event in &mut submission_events {
        if let opi_ai::stream::AssistantStreamEvent::Done { message, .. } = event {
            message.provider = "test".into();
            message.model = "mock-model".into();
        }
    }
    let handle = backend.submit(submission_events, 1, None);

    // wait0: exactly one status check; the outcome stays pending with the
    // same handle.
    let pending = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("pending is Ok");
    assert_eq!(pending.stop_reason, StopReason::Deferred);
    assert_eq!(pending.deferred.as_deref(), Some(&handle));
    assert_eq!(backend.fetch_count(), 1);

    // A second explicit fetch is ready: the final assistant message alone.
    let ready = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("ready");
    assert_eq!(ready.stop_reason, StopReason::Stop);
    assert!(ready.deferred.is_none());
    let text = match ready.content.first() {
        Some(opi_ai::message::AssistantContent::Text { text, .. }) => text.clone(),
        other => panic!("expected final text, got {other:?}"),
    };
    assert_eq!(text, "ready");
    assert_eq!(backend.fetch_count(), 2);
}

#[tokio::test]
async fn unsupported_provider_and_unknown_handle_fail_typed() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    // No deferred registration: fetch fails typed before auth.
    let mut stranger = pending_handle();
    stranger.provider = "other".into();
    let error = collection
        .validate_deferred_handle(&stranger, None)
        .unwrap_err();
    assert!(matches!(error, DeferredError::Unsupported { .. }));
    let failure = collection
        .fetch_deferred(&stranger, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert_eq!(failure.message().stop_reason, StopReason::Error);

    // Registered backend, but the handle token is unknown: the scripted
    // backend fails the single fetch attempt.
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let mut unknown = pending_handle();
    unknown.id = "deferred-missing".into();
    let failure = collection
        .fetch_deferred(&unknown, DeferredFetchCall::default())
        .await
        .unwrap_err();
    assert_eq!(backend.fetch_count(), 1);
    assert!(backend.cancelled_handles().is_empty());
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert_eq!(
        failure.message().error_message.as_deref(),
        Some(
            "invalid provider configuration: Unknown scripted deferred response: deferred-missing"
        )
    );
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::Config(_)
    ));
    assert!(!failure.cause().is_retryable());
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Terminal
    );
}

#[tokio::test]
async fn missing_credential_fails_typed_with_remediation_message() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    // A resolver that reports CredentialNeeded, like classify/images.
    struct MissingKey;
    impl opi_ai::auth::AuthResolver for MissingKey {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::credential::BoxAuthFuture<
            'a,
            Result<ResolvedAuth, opi_ai::provider::ProviderError>,
        > {
            Box::pin(async {
                Err(opi_ai::provider::ProviderError::CredentialNeeded {
                    provider_id: "test".into(),
                })
            })
        }
    }
    collection
        .register_deferred_api(
            std::sync::Arc::clone(&backend)
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            std::sync::Arc::new(MissingKey),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let handle = backend.submit(test_support::text_response("never"), 5, None);
    let failure = collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .unwrap_err();
    // The typed cause stays CredentialNeeded (remediation-classified) and
    // the canonical message carries the provider name; nothing dispatched.
    assert!(matches!(
        failure.cause(),
        opi_ai::provider::ProviderError::CredentialNeeded { .. }
    ));
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("test")
    );
    assert_eq!(backend.fetch_count(), 0);
}

#[tokio::test]
async fn deferred_auth_failure_preserves_cause_and_message_retry_input() {
    use opi_ai::errors::{ProviderError, ProviderErrorSummary};
    use std::sync::Arc;

    struct FailedAuth(&'static str);
    impl opi_ai::auth::AuthResolver for FailedAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            let cause = ProviderError::AuthFailed(ProviderErrorSummary::sanitized(self.0));
            Box::pin(async move { Err(cause) })
        }
    }

    for (detail, classification) in [
        (
            "controlled-auth-setup-failure",
            WholeCallRetryClassification::Terminal,
        ),
        (
            "503 Service Unavailable",
            WholeCallRetryClassification::Transient,
        ),
    ] {
        let backend = Arc::new(ScriptedDeferredBackend::new(
            "lifecycle-differential",
            "mock-model",
        ));
        let handle = backend.submit(test_support::text_response("not fetched"), 0, Some(25));
        let mut collection = Models::new();
        collection
            .register_deferred_api(
                Arc::clone(&backend) as Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
                Arc::new(FailedAuth(detail)),
                AuthProvenanceSource::Static,
            )
            .expect("register");
        let failure = collection
            .fetch_deferred(&handle, DeferredFetchCall::default())
            .await
            .expect_err("resolver failure stops fetch");
        assert_eq!(backend.fetch_count(), 0);
        assert!(backend.cancelled_handles().is_empty());
        assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
        assert!(!failure.cause().is_retryable());
        let message = failure.message();
        assert_eq!(message.stop_reason, StopReason::Error);
        assert_eq!(message.provider, handle.provider);
        assert_eq!(message.model, handle.model_id);
        let diagnostic = format!("authentication failed: {detail}");
        assert_eq!(message.error_message.as_deref(), Some(diagnostic.as_str()));
        assert_eq!(classify_whole_call_retry(message), classification);
    }
}
