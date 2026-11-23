//! Custom Chat call rejection before authentication or dispatch.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::contract::SimpleOptions;
use opi_ai::auth::{AuthProvenanceSource, AuthResolver};
use opi_ai::models::ChatModelRecord;
use opi_ai::provider::{ModelInfo, ProviderError};
use opi_ai::registry::ModelCapabilities;
use opi_ai::types::ApiId;
use opi_ai::{Models, WireApi};

use super::fixtures::{
    CountingResolver, EchoChatApi, EchoOptions, Observed, OtherOptions, TypedChatApi,
    counting_resolver, custom_provider, echo_record_info, models_request,
};

#[tokio::test]
async fn codex_wire_metadata_does_not_admit_unconsumed_simple_tool_selection() {
    for typed_api in [false, true] {
        let mut models = Models::new();
        let resolver = Arc::new(CountingResolver {
            calls: AtomicUsize::new(0),
        });
        let record = ModelInfo::new(
            "custom-codex",
            "custom",
            WireApi::OpenAiCodexResponses,
            ModelCapabilities::new(100_000, 1_024).with_streaming(true),
        );
        let observed: Observed = Arc::default();
        if typed_api {
            models
                .register_chat_api(
                    Arc::new(EchoChatApi {
                        api: ApiId::new("custom-codex"),
                        provider_id: "custom".into(),
                        record: record.clone(),
                        text: "unused".into(),
                        observed: observed.clone(),
                    }),
                    resolver.clone(),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
        } else {
            models
                .register_route(
                    custom_provider("custom", vec![record.clone()]),
                    resolver.clone(),
                    AuthProvenanceSource::Static,
                    Default::default(),
                )
                .unwrap();
        }
        let failure = models
            .complete_simple(
                ChatModelRecord::new("custom", record),
                models_request(),
                SimpleOptions {
                    tool_choice: Some(opi_ai::SimpleToolChoice::None),
                    ..Default::default()
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            failure.cause(),
            ProviderError::UnsupportedCapability(_)
        ));
        assert_eq!(resolver.calls.load(Ordering::SeqCst), 0);
        assert!(observed.lock().unwrap().is_empty());
    }
}

#[tokio::test]
async fn models_type_mismatch_and_unregistered_api_fail_before_auth() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "unreachable".into(),
                observed: observed.clone(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let record = ChatModelRecord::new("custom", record_info.clone());

    // Upsert the same ApiId with an incompatible options type.
    let replacement_reads = counting_resolver();
    let replacement = models
        .register_chat_api(
            Arc::new(TypedChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
            }),
            replacement_reads.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Options created before the upsert now fail the type check pre-auth.
    let failure = models
        .complete(
            record.clone(),
            models_request(),
            handle.options(EchoOptions {
                label: "stale".into(),
            }),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(failure.cause(), ProviderError::Config(reason)
            if reason.as_str().starts_with("native options type does not match ")),
        "{}",
        failure.cause()
    );
    assert!(observed.lock().unwrap().is_empty());
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
    assert_eq!(replacement_reads.calls.load(Ordering::SeqCst), 0);

    // Options from another instance's handle (different ApiId) fail as
    // unregistered on this instance, also before auth.
    let mut fresh = Models::new();
    let orphan_reads = counting_resolver();
    let orphan = fresh
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("orphan-chat"),
                provider_id: "orphan".into(),
                record: record_info.clone(),
                text: "orphan".into(),
                observed: Arc::default(),
            }),
            orphan_reads.clone(),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let failure = models
        .complete(
            record.clone(),
            models_request(),
            orphan.options(EchoOptions {
                label: "orphan".into(),
            }),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(failure.cause(), ProviderError::Config(reason)
            if reason.as_str() == "chat api 'orphan-chat' is not registered"),
        "{}",
        failure.cause()
    );
    assert_eq!(orphan_reads.calls.load(Ordering::SeqCst), 0);

    // Use matching options to isolate rejection by the provider ownership check.
    let failure = models
        .complete(
            ChatModelRecord::new("elsewhere", record_info),
            models_request(),
            replacement.options(OtherOptions { value: 0 }),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(failure.cause(), ProviderError::Config(reason)
            if reason.as_str().starts_with("record provider 'elsewhere' does not match ")),
        "{}",
        failure.cause()
    );
    assert_eq!(failure.message().provider, "elsewhere");
    assert_eq!(failure.message().model, "echo-model");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
    assert_eq!(replacement_reads.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn models_pre_cancelled_request_fails_before_auth() {
    let mut models = Models::new();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "cancelled".into(),
                observed: Arc::default(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let request = models_request();
    request.cancel.cancel();
    let record = ChatModelRecord::new("custom", record_info);
    let failure = models
        .complete(
            record,
            request,
            handle.options(EchoOptions {
                label: "never".into(),
            }),
        )
        .await
        .unwrap_err();
    // A cancelled logical call fails typed with the cancellation cause
    // (mapped to Aborted by CallFailure), before any auth resolution.
    assert!(matches!(
        failure.cause(),
        ProviderError::Cancelled | ProviderError::LoginCancelled { .. }
    ));
    assert_eq!(failure.message().stop_reason.as_str(), "aborted");
    assert_eq!(failure.message().provider, "custom");
    assert_eq!(failure.message().model, "echo-model");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 0);
}
