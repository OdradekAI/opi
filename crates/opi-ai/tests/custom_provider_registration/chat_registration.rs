//! Custom Chat API identity and the distinction between routes and model lookup.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use opi_ai::auth::AuthProvenanceSource;
use opi_ai::models::ChatModelRecord;
use opi_ai::types::ApiId;
use opi_ai::{Models, WireApi};

use super::fixtures::{
    EchoChatApi, EchoOptions, Observed, TypedChatApi, assistant_text, counting_resolver,
    custom_model, custom_provider, echo_record_info, models_request, observed_call,
};

#[test]
fn chat_registration_preserves_arbitrary_exact_api_ids_without_auth() {
    for id in ["normal", "", " \t", "api/β\0id"] {
        let mut models = Models::new();
        let resolver = counting_resolver();
        let observed: Observed = Arc::default();
        let handle = models
            .register_chat_api(
                Arc::new(EchoChatApi {
                    api: ApiId::new(id),
                    provider_id: "custom".into(),
                    record: echo_record_info(2_048),
                    text: "registered".into(),
                    observed: observed.clone(),
                }),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .expect("ApiId has no nonempty or whitespace grammar");
        assert_eq!(handle.api_id().as_str(), id);
        assert!(matches!(
            models.register_chat_api(
                Arc::new(TypedChatApi {
                    api: ApiId::new(id),
                    provider_id: " \t".into(),
                    record: echo_record_info(2_048),
                }),
                resolver.clone(),
                AuthProvenanceSource::Static,
            ),
            Err(opi_ai::registry::RegistrationError::EmptyProviderId)
        ));
        assert_eq!(resolver.calls.load(Ordering::SeqCst), 0);
        assert!(observed.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn models_register_chat_api_keeps_record_out_of_catalog() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "api response".into(),
        observed: observed.clone(),
    };
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Exact ApiId identity is preserved on the registration handle.
    assert_eq!(handle.api_id().as_str(), "custom-chat-v1");
    // The custom complete record stays absent from every catalog read.
    assert_eq!(models.get_model("custom", "echo-model"), None);
    assert!(
        models
            .registry()
            .all_models()
            .iter()
            .all(|(provider, _)| *provider != "custom")
    );

    // ... yet it is callable on the registered provider/wire.
    let record = ChatModelRecord::new("custom", record_info);
    let message = models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "native".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&message), "api response");
    let call = observed_call(&observed);
    assert_eq!(call.request_model, "custom:echo-model");
    assert_eq!(call.options_label, "native");
    assert_eq!(call.auth_secret, "resolver-secret");
}

#[tokio::test]
async fn models_get_model_returns_owned_record_with_exact_wire() {
    let mut models = Models::new();
    let provider = custom_provider("owned", vec![custom_model("owned-model", "Owned Model")]);
    models
        .register_route(
            provider,
            counting_resolver(),
            AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();

    let owned = models
        .get_model("owned", "owned-model")
        .expect("catalog record");
    assert_eq!(owned.id, "owned-model");
    assert_eq!(owned.wire_api, WireApi::OpenAiCompletions);
    assert_eq!(models.get_model("owned", "missing"), None);
    assert_eq!(models.get_model("missing", "owned-model"), None);
}
