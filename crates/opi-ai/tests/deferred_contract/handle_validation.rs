//! Deferred handle serialization, exact registration and pre-dispatch validation.

use opi_ai::api::contract::DeferredFetchCall;
use opi_ai::auth::{AuthScheme, ResolvedAuth, StaticAuthResolver};
use opi_ai::stream::StopReason;
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use opi_ai::types::deferred::{DeferredError, DeferredHandle};
use opi_ai::{AuthProvenanceSource, Models};
use secrecy::SecretString;
use serde_json::json;

use super::fixtures::pending_handle;
use super::registration_pending_auth;

#[test]
fn deferred_registration_preserves_arbitrary_exact_api_ids_without_auth() {
    use opi_ai::api::contract::{DeferredApi, DeferredCancelCall};
    use opi_ai::provider::{EventStream, ProviderError};
    use opi_ai::types::ApiId;
    use std::sync::Arc;

    struct RegistrationOnlyApi {
        api: ApiId,
        provider: &'static str,
    }
    impl DeferredApi<()> for RegistrationOnlyApi {
        fn api_id(&self) -> &ApiId {
            &self.api
        }
        fn provider_id(&self) -> &str {
            self.provider
        }
        fn model_id(&self) -> &str {
            "mock-model"
        }
        fn default_options(&self) {
            panic!("registration must not prepare a call")
        }
        fn fetch_prepared(
            &self,
            _: &DeferredHandle,
            _: &(),
            _: ResolvedAuth,
            _: DeferredFetchCall,
        ) -> EventStream {
            panic!("registration must not dispatch")
        }
        fn cancel_prepared<'a>(
            &'a self,
            _: &'a DeferredHandle,
            _: &'a (),
            _: ResolvedAuth,
            _: DeferredCancelCall,
        ) -> futures_util::future::BoxFuture<'a, Result<(), ProviderError>> {
            panic!("registration must not dispatch")
        }
    }
    for id in ["normal", "", " \t", "api/β\0id"] {
        let mut models = Models::new();
        let resolver = Arc::new(registration_pending_auth::PendingAuth::default());
        let handle = models
            .register_deferred_api(
                Arc::new(RegistrationOnlyApi {
                    api: ApiId::new(id),
                    provider: "test",
                }),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .expect("ApiId has no nonempty or whitespace grammar");
        assert_eq!(handle.api_id.as_str(), id);
        let mut pending = pending_handle();
        pending.api = id.into();
        models.validate_deferred_handle(&pending, None).unwrap();
        assert!(matches!(
            models.register_deferred_api(
                Arc::new(RegistrationOnlyApi {
                    api: ApiId::new(id),
                    provider: " \t"
                }),
                resolver.clone(),
                AuthProvenanceSource::Static,
            ),
            Err(opi_ai::registry::RegistrationError::EmptyProviderId)
        ));
        assert_eq!(resolver.calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
}

#[test]
fn deferred_handle_serializes_pi_camel_case() {
    let handle = DeferredHandle {
        provider: "p".into(),
        model_id: "m".into(),
        api: "anthropic-messages".into(),
        id: "resp_123".into(),
        expires_at_ms: Some(1_700_000_000_000),
        poll_after_ms: Some(5_000),
        data: Some(json!({ "row": 2 })),
    };
    let value = serde_json::to_value(&handle).expect("serialize");
    assert_eq!(
        value,
        json!({
            "provider": "p",
            "modelId": "m",
            "api": "anthropic-messages",
            "id": "resp_123",
            "expiresAt": 1_700_000_000_000u64,
            "pollAfterMs": 5_000u64,
            "data": { "row": 2 },
        })
    );
    let round: DeferredHandle = serde_json::from_value(value).expect("deserialize");
    assert_eq!(round, handle);
}

#[test]
fn deferred_handle_minimal_roundtrip_omits_optional_fields() {
    let handle = pending_handle();
    let value = serde_json::to_value(&handle).expect("serialize");
    assert!(value.get("expiresAt").is_none());
    assert!(value.get("pollAfterMs").is_none());
    assert!(value.get("data").is_none());
    let round: DeferredHandle = serde_json::from_value(value).expect("deserialize");
    assert_eq!(round, handle);
}

#[test]
fn deferred_handle_identity_mismatch_is_typed() {
    let handle = pending_handle();
    let wrong = handle
        .validate_identity("other-provider", "mock-model", "scripted-deferred")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    let wrong = handle
        .validate_identity("test", "other-model", "scripted-deferred")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    let wrong = handle
        .validate_identity("test", "mock-model", "other-api")
        .unwrap_err();
    assert!(matches!(wrong, DeferredError::RouteMismatch { .. }));
    handle
        .validate_identity("test", "mock-model", "scripted-deferred")
        .expect("matching identity validates");
}

#[test]
fn deferred_handle_expiry_is_typed_against_injected_clock() {
    let mut handle = pending_handle();
    handle.expires_at_ms = Some(1_000);
    handle.validate_expiry(999).expect("not yet expired");
    let expired = handle.validate_expiry(1_000).unwrap_err();
    assert!(matches!(expired, DeferredError::Expired { .. }));
}

#[tokio::test]
async fn unregistered_deferred_api_names_the_lookup_in_fetch_and_cancel() {
    use opi_ai::provider::ProviderError;
    use opi_ai::test_support::CountingAuthResolver;
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    let auth_calls = Arc::new(AtomicU32::new(0));
    let backend = Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut models = Models::new();
    models
        .register_deferred_api(
            backend.clone(),
            Arc::new(CountingAuthResolver::new(auth_calls.clone())),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let handle = DeferredHandle {
        api: "other-api".into(),
        ..pending_handle()
    };
    assert_eq!(
        models.validate_deferred_handle(&handle, None),
        Err(DeferredError::Unsupported {
            provider: "test".into(),
        })
    );

    let failure = models
        .fetch_deferred(&handle, Default::default())
        .await
        .unwrap_err();
    let ProviderError::Config(summary) = failure.cause() else {
        panic!(
            "expected a local configuration error, got {:?}",
            failure.cause()
        );
    };
    assert_eq!(
        summary.as_str(),
        "no deferred backend is registered for provider 'test' and API 'other-api'"
    );
    assert_eq!(failure.message().stop_reason, StopReason::Error);
    assert_eq!(
        failure.message().error_message.as_deref(),
        Some(failure.cause().to_string().as_str())
    );
    assert_eq!(
        models
            .cancel_deferred(&handle, Default::default())
            .await
            .unwrap_err(),
        *failure.cause()
    );
    assert_eq!(auth_calls.load(Ordering::SeqCst), 0);
    assert_eq!(backend.fetch_count(), 0);
    assert!(backend.cancelled_handles().is_empty());
}

#[tokio::test]
async fn expired_handle_fails_typed_before_auth_or_dispatch() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
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
    let handle =
        backend.submit_with_expiry(test_support::text_response("late"), 0, None, Some(1_000));
    // Typed validation rejects at the SDK boundary with the injected clock.
    let error = collection
        .validate_deferred_handle(&handle, Some(1_000))
        .unwrap_err();
    assert!(matches!(error, DeferredError::Expired { .. }));
    // Fetch with the same clock reading never dispatches.
    let call = DeferredFetchCall {
        now_ms: Some(1_000),
        ..Default::default()
    };
    let failure = collection.fetch_deferred(&handle, call).await.unwrap_err();
    assert_eq!(failure.message().provider, handle.provider);
    assert_eq!(failure.message().model, handle.model_id);
    assert!(
        failure
            .message()
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("expired")
    );
    assert_eq!(backend.fetch_count(), 0);
}

#[tokio::test]
async fn cancel_checks_all_deferred_route_fields_before_auth_or_dispatch() {
    use opi_ai::message::AssistantContent;
    use opi_ai::provider::ProviderError;
    use opi_ai::test_support::CountingAuthResolver;
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    for name in [
        "provider-mismatch",
        "model-mismatch",
        "api-mismatch",
        "unknown-job-id",
        "matching-handle",
    ] {
        let backend = Arc::new(ScriptedDeferredBackend::new(
            "lifecycle-differential",
            "mock-model",
        ));
        let original = backend.submit(test_support::text_response("ready"), 0, Some(25));
        let auth_calls = Arc::new(AtomicU32::new(0));
        let mut models = Models::new();
        models
            .register_deferred_api(
                backend.clone(),
                Arc::new(CountingAuthResolver::new(auth_calls.clone())),
                AuthProvenanceSource::Static,
            )
            .unwrap();

        let mut input = original.clone();
        match name {
            "provider-mismatch" => input.provider = "other-provider".into(),
            "model-mismatch" => input.model_id = "other-model".into(),
            "api-mismatch" => input.api = "other-api".into(),
            "unknown-job-id" => input.id = "deferred-missing".into(),
            "matching-handle" => {}
            _ => unreachable!(),
        }
        let validation = models.validate_deferred_handle(&input, None);
        let direct_validation = match (&validation, name) {
            (Err(DeferredError::Unsupported { .. }), "provider-mismatch" | "api-mismatch") => {
                "Unsupported"
            }
            (Err(DeferredError::RouteMismatch(_)), "model-mismatch") => "RouteMismatch",
            (Ok(()), "unknown-job-id" | "matching-handle") => "ok",
            _ => panic!("unexpected validation for {name}: {validation:?}"),
        };
        assert_eq!(auth_calls.load(Ordering::SeqCst), 0);

        let result = models.cancel_deferred(&input, Default::default()).await;
        let admitted = validation.is_ok();
        let (delivery, cause, diagnostic) = match result {
            Ok(()) if admitted => ("ok", None, None),
            Err(error @ ProviderError::Config(_)) if !admitted => {
                ("typed-err", Some("Config"), Some(error.to_string()))
            }
            other => panic!("unexpected cancellation for {name}: {other:?}"),
        };
        let cancel_auth_calls = auth_calls.load(Ordering::SeqCst);
        let dispatches = backend.cancelled_handles();
        assert_eq!(cancel_auth_calls, u32::from(admitted), "{name}");
        assert_eq!(
            dispatches.as_slice(),
            if admitted {
                std::slice::from_ref(&input)
            } else {
                &[]
            },
            "{name}"
        );
        assert_eq!(backend.fetch_count(), 0);
        let cancel = json!({
            "auth_calls": cancel_auth_calls, "dispatch_count": dispatches.len(), "dispatches": dispatches,
            "result_delivery": delivery, "typed_cause": cause, "error_message": diagnostic,
        });

        let refetch = models
            .fetch_deferred(&original, DeferredFetchCall::default())
            .await;
        let (message, refetch_delivery, refetch_cause) = match &refetch {
            Ok(message) if name != "matching-handle" => (message, "ok", None),
            Err(failure) if name == "matching-handle" => {
                assert!(matches!(failure.cause(), ProviderError::Config(_)));
                assert!(failure.cause().to_string().contains("was cancelled"));
                (failure.message(), "typed-err", Some("Config"))
            }
            other => panic!("unexpected original refetch for {name}: {other:?}"),
        };
        let text: String = message
            .content
            .iter()
            .filter_map(|part| match part {
                AssistantContent::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        let cancelled = name == "matching-handle";
        assert_eq!(
            message.stop_reason,
            if cancelled {
                StopReason::Error
            } else {
                StopReason::Stop
            }
        );
        assert_eq!(text, if cancelled { "" } else { "ready" });
        let refetch_auth_calls = auth_calls.load(Ordering::SeqCst) - cancel_auth_calls;
        assert_eq!(refetch_auth_calls, 1, "{name}");
        assert_eq!(backend.fetch_count(), 1, "{name}");
        let refetch_original = json!({
            "auth_calls": refetch_auth_calls, "dispatch_count": backend.fetch_count(),
            "result_delivery": refetch_delivery, "typed_cause": refetch_cause, "stop": message.stop_reason, "text": text,
            "error_message": message.error_message,
        });
        println!(
            "deferred_cancel_route {}",
            json!({
                "name": name,
                "route": {"provider": original.provider, "modelId": original.model_id, "api": original.api},
                "original": original, "request_handle": input,
                "direct_validation": direct_validation, "cancel": cancel, "refetch_original": refetch_original,
            })
        );
    }
}
