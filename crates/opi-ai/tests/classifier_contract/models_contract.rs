//! Typed Models registration, call ownership, authentication and telemetry.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use opi_ai::AuthProvenanceSource;
use opi_ai::api::system_one::{TypesafeSystemOneApi, TypesafeSystemOneOptions};
use opi_ai::auth::AuthScheme;
use opi_ai::classifier::{ClassifierCall, ClassifierModelRecord, ClassifierStopReason};
use opi_ai::http::HttpClient;
use opi_ai::models::{Models, ModelsError};
use opi_ai::provider::ProviderError;
use opi_ai::types::ApiId;
use opi_telemetry::memory::Recorder;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

use super::fixtures::classify_context;
use super::pending_auth;
use super::typesafe_system_one::{
    MissingKeyResolver, classifier_models, classifier_pricing, classifier_record, ok_response,
};

#[tokio::test]
async fn classifier_auth_lifecycle_preaborted_never_starts_resolution() {
    let resolver = Arc::new(pending_auth::PendingAuth::default());
    let record = classifier_record("http://127.0.0.1:1".into());
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = ClassifierCall::default();
    call.cancel.cancel();
    let result = models.classify(record, classify_context(), call);
    tokio::pin!(result);
    let outcome = futures_util::poll!(&mut result);
    assert_eq!(
        resolver.calls.load(AtomicOrdering::SeqCst),
        0,
        "pre-aborted classification must not start auth"
    );
    let std::task::Poll::Ready(result) = outcome else {
        panic!("pre-aborted classifier call must settle")
    };
    assert_eq!(result.stop_reason, ClassifierStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
}

#[tokio::test]
async fn classifier_auth_lifecycle_cancel_releases_pending_resolution() {
    let resolver = Arc::new(pending_auth::PendingAuth::default());
    let record = classifier_record("http://127.0.0.1:1".into());
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            resolver.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let call = ClassifierCall::default();
    let cancel = call.cancel.clone();
    let result = models.classify(record, classify_context(), call);
    tokio::pin!(result);
    assert!(futures_util::poll!(&mut result).is_pending());
    assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 1);
    cancel.cancel();
    let std::task::Poll::Ready(result) = futures_util::poll!(&mut result) else {
        panic!("cancellation must interrupt pending classifier auth")
    };
    assert_eq!(result.stop_reason, ClassifierStopReason::Aborted);
    assert!(matches!(result.error, Some(ProviderError::Cancelled)));
    assert!(resolver.released.load(AtomicOrdering::SeqCst));
}

#[test]
fn classifier_registration_preserves_arbitrary_exact_api_ids_without_auth() {
    for id in ["normal", "", " \t", "api/β\0id"] {
        let mut models = Models::new();
        let resolver = Arc::new(pending_auth::PendingAuth::default());
        let mut record = classifier_record("http://127.0.0.1:1".into());
        record.api = ApiId::new(id);
        let handle = models
            .register_classifier_api(
                Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .expect("ApiId has no nonempty or whitespace grammar");
        assert_eq!(handle.api_id().as_str(), id);
        assert_eq!(
            models
                .classifier_model("typesafe", &record.id)
                .unwrap()
                .api
                .as_str(),
            id
        );
        record.provider_id = " \t".into();
        assert!(matches!(
            models.register_classifier_api(
                Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record)),
                resolver.clone(),
                AuthProvenanceSource::Static,
            ),
            Err(opi_ai::registry::RegistrationError::EmptyProviderId)
        ));
        assert_eq!(resolver.calls.load(AtomicOrdering::SeqCst), 0);
    }
}

#[tokio::test]
async fn classifier_selection_rejects_unknown_before_auth_and_resolves_registered() {
    let server = MockServer::start().await;
    let record = classifier_record(server.uri());
    let counter = Arc::new(AtomicU32::new(0));
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::test_support::CountingAuthResolver::new(
                counter.clone(),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register classifier api");

    // Unknown id: typed rejection.
    match models.classifier_model("typesafe", "unknown-model") {
        Err(ModelsError::NotAClassifierModel { provider, id }) => {
            assert_eq!(
                (provider.as_str(), id.as_str()),
                ("typesafe", "unknown-model")
            );
        }
        other => panic!("expected NotAClassifierModel, got {other:?}"),
    }
    // A chat-model id never matches classifier selection either.
    assert!(matches!(
        models.classifier_model("typesafe", "jev-chat-model"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    // The registered record resolves, and selection never resolves auth.
    let resolved = models
        .classifier_model("typesafe", "jev-latest")
        .expect("registered record resolves");
    assert_eq!(resolved.id, "jev-latest");
    assert_eq!(counter.load(AtomicOrdering::SeqCst), 0);
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn classify_without_registration_fails_without_dispatch() {
    let models = Models::new();
    let record = ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        compat: None,
        provider_id: "typesafe".into(),
        id: "jev-latest".into(),
        name: "Jev".into(),
        api: ApiId::new("typesafe-system-one"),
        base_url: "http://127.0.0.1:9".into(),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: Some(classifier_pricing()),
    };
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("no classifier implementation for api"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn classifier_handle_options_survive_re_registration_and_reject_mismatch() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (mut models, handle) = classifier_models(&record);
    let options = handle.options(TypesafeSystemOneOptions::default());

    // A compatible re-registration keeps handle-created options valid.
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("re-register");
    let first = models
        .classify_native(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;
    assert_eq!(first.stop_reason, ClassifierStopReason::Stop);

    // Options tagged for another api id are a typed pre-dispatch failure.
    let mut other_record = classifier_record("http://127.0.0.1:9".into());
    other_record.api = ApiId::new("llama-cpp-classify");
    let other_handle = models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), other_record)),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register other api");
    let mismatched = other_handle.options(TypesafeSystemOneOptions::default());
    let second = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            mismatched,
        )
        .await;
    assert_eq!(second.stop_reason, ClassifierStopReason::Error);
    assert!(
        second
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("do not match the registration"),
        "message was {:?}",
        second.error_message
    );
}

#[tokio::test]
async fn classify_passes_frozen_telemetry_context() {
    let recorder = Recorder::default();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ok_response())
        .expect(1)
        .mount(&server)
        .await;

    let record = classifier_record(server.uri());
    let (models, _handle) = classifier_models(&record);
    let call = ClassifierCall {
        telemetry_context: Some(recorder.context()),
        ..Default::default()
    };
    let result = models.classify(record, classify_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(
        recorder.spans().is_empty(),
        "classification carries telemetry context but does not install a pi.ai span"
    );
}

#[tokio::test]
async fn classifier_call_owns_binding_before_first_poll() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ok_response())
        .expect(2)
        .mount(&server)
        .await;
    let record = classifier_record(server.uri());
    let (mut models, handle) = classifier_models(&record);
    let ordinary = models.classify(
        record.clone(),
        classify_context(),
        ClassifierCall::default(),
    );
    let native = models.classify_native(
        record.clone(),
        classify_context(),
        ClassifierCall::default(),
        handle.options(TypesafeSystemOneOptions::default()),
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(HttpClient::new(), record)),
            Arc::new(MissingKeyResolver),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    drop(models);
    for result in [ordinary.await, native.await] {
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
    }
}
