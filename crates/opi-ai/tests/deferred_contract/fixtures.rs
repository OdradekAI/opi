//! Deferred contract setup shared within this test target.

use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::test_support::{self, ScriptedDeferredBackend};
use opi_ai::types::deferred::DeferredHandle;
use opi_ai::{AuthProvenanceSource, Models};
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;

pub(super) fn pending_handle() -> DeferredHandle {
    DeferredHandle {
        provider: "test".into(),
        model_id: "mock-model".into(),
        api: "scripted-deferred".into(),
        id: "deferred-1".into(),
        expires_at_ms: None,
        poll_after_ms: None,
        data: None,
    }
}

pub(super) fn chat_request() -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "mock-model".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text { text: "Hi".into() }],
                    timestamp_ms: 0,
                },
            )],
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
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn chat_record(models: &Models) -> opi_ai::models::ChatModelRecord {
    let (_provider, model) = models
        .resolve("test:mock-model")
        .expect("mock route resolves");
    opi_ai::models::ChatModelRecord::new("test", model.clone())
}

pub(super) fn registered_collection(
    backend: &std::sync::Arc<ScriptedDeferredBackend>,
) -> (Models, opi_ai::models::ChatModelRecord) {
    let mut collection = test_support::single_route_collection(Box::new(
        test_support::MockProvider::new("test", vec![test_support::text_response("x")]),
    ));
    collection
        .register_deferred_api(
            std::sync::Arc::clone(backend)
                as std::sync::Arc<dyn opi_ai::api::contract::DeferredApi<()>>,
            std::sync::Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let record = chat_record(&collection);
    (collection, record)
}
