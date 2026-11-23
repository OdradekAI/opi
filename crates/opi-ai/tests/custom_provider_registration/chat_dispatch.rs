//! Record, options and authentication passed to an already registered custom adapter.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use opi_ai::api::contract::SimpleOptions;
use opi_ai::auth::{AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::models::ChatModelRecord;
use opi_ai::provider::ModelInfo;
use opi_ai::registry::ModelCapabilities;
use opi_ai::types::ApiId;
use opi_ai::{Models, WireApi};
use secrecy::SecretString;

use super::fixtures::{
    EchoChatApi, EchoOptions, Observed, assistant_text, counting_resolver, custom_provider,
    echo_record_info, models_request, observed_call,
};

#[tokio::test]
async fn models_frozen_record_is_not_replaced_by_same_id_catalog_data() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(1_024);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "frozen".into(),
        observed: observed.clone(),
    };
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // A catalog model with the SAME provider+model id but a larger output
    // limit exists; it must not replace the caller's complete record. The
    // route registration replaces the adapter's catalog-invisible route
    // provider while the typed ApiId registration keeps serving the native
    // entry.
    let catalog_model = ModelInfo::new(
        "echo-model",
        "Catalog Echo",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(200_000, 8_192).with_streaming(true),
    );
    models
        .register_route(
            custom_provider("custom", vec![catalog_model]),
            counting_resolver(),
            AuthProvenanceSource::Static,
            opi_ai::CompatMetadata::default(),
        )
        .unwrap();
    let catalog = models
        .get_model("custom", "echo-model")
        .expect("catalog record present");
    assert_eq!(catalog.capabilities.max_output_tokens, 8_192);

    let record = ChatModelRecord::new("custom", record_info);
    models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "native".into(),
            }),
        )
        .await
        .unwrap();
    let call = observed_call(&observed);
    // The complete record reaches the adapter without filling a native
    // omission from either the caller's or the catalog's output capacity.
    assert_eq!(call.selected_model.capabilities.max_output_tokens, 1_024);
    assert_eq!(call.max_tokens, None);
}

#[tokio::test]
async fn models_simple_entry_uses_neutral_options_and_same_binding() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "simple response".into(),
        observed: observed.clone(),
    };
    models
        .register_chat_api(
            Arc::new(adapter),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("custom", record_info);
    let message = models
        .complete_simple(
            record,
            models_request(),
            SimpleOptions {
                temperature: Some(0.25),
                ..SimpleOptions::default()
            },
        )
        .await
        .unwrap();
    let call = observed_call(&observed);
    assert_eq!(call.options_label, "simple");
    assert_eq!(call.temperature, Some(0.25));
    assert_eq!(call.request_model, "custom:echo-model");
    assert_eq!(assistant_text(&message), "simple response");
    assert_eq!(call.auth_secret, "resolver-secret");
}

#[tokio::test]
async fn models_direct_entry_uses_injected_auth_without_models_auth() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let adapter = EchoChatApi {
        api: ApiId::new("custom-chat-v1"),
        provider_id: "custom".into(),
        record: record_info.clone(),
        text: "direct response".into(),
        observed: observed.clone(),
    };
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(adapter),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let auth = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::Bearer,
        secret: SecretString::from("direct-secret"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    };
    let record = ChatModelRecord::new("custom", record_info);
    models
        .complete_direct(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "direct".into(),
            }),
            auth,
        )
        .await
        .unwrap();

    let call = observed_call(&observed);
    assert_eq!(call.auth_secret, "direct-secret");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn models_native_options_portable_across_instances() {
    let record_info = echo_record_info(2_048);
    let mut first = Models::new();
    let observed_a: Observed = Arc::default();
    let handle = first
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "instance-a".into(),
                observed: observed_a.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let mut second = Models::new();
    let observed_b: Observed = Arc::default();
    second
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "instance-b".into(),
                observed: observed_b.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The same option value (created from the FIRST instance's handle)
    // crosses to the second instance with a compatible registration.
    let options = handle.options(EchoOptions {
        label: "portable".into(),
    });
    let record = ChatModelRecord::new("custom", record_info.clone());
    let a = first
        .complete(record.clone(), models_request(), options.clone())
        .await
        .unwrap();
    let b = second
        .complete(record, models_request(), options)
        .await
        .unwrap();
    assert_eq!(assistant_text(&a), "instance-a");
    assert_eq!(assistant_text(&b), "instance-b");
    assert_eq!(observed_call(&observed_a).options_label, "portable");
    assert_eq!(observed_call(&observed_b).options_label, "portable");
}
