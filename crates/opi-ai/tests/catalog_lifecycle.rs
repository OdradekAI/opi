//! Explicit catalog publication and restore through shared store revisions.

#[path = "support/catalog_store.rs"]
mod catalog_fixture;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use opi_ai::catalog::{DynamicCatalogSource, ModelsStore};
use opi_ai::models::{AuthDescriptor, CompatMetadata, Models};
use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use opi_ai::test_support::MockProvider;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tokio_util::sync::CancellationToken;

struct SwitchSource(Arc<AtomicBool>);

impl DynamicCatalogSource for SwitchSource {
    fn id(&self) -> &str {
        "fixture-source"
    }

    fn fetch(
        &self,
        _: CancellationToken,
    ) -> futures_util::future::BoxFuture<
        'static,
        Result<HashMap<String, opi_ai::catalog::ModelsStoreEntry>, ProviderError>,
    > {
        let fail = self.0.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                return Err(ProviderError::RequestFailed(
                    ProviderErrorSummary::sanitized("fixture unavailable"),
                ));
            }
            Ok(catalog_fixture::catalogs(HashMap::from([(
                "fixture".into(),
                vec![ModelInfo::new(
                    "remote",
                    "Remote",
                    WireApi::OpenAiCompletions,
                    ModelCapabilities::new(8192, 1024),
                )],
            )])))
        })
    }
}

fn collection(store: ModelsStore, source: Arc<SwitchSource>) -> Models {
    let mut models = Models::new();
    models
        .register(
            Box::new(MockProvider::new_with_models("fixture", vec![], vec![])),
            AuthDescriptor::Resolved {
                source: "fixture".into(),
            },
            CompatMetadata::default(),
        )
        .unwrap();
    models.set_models_store(store);
    models.register_catalog_source(source);
    models
}

#[tokio::test]
async fn shared_store_new_revision_cannot_restore_deleted_records_from_memory() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let failed = Arc::new(AtomicBool::new(false));
    let source = Arc::new(SwitchSource(failed.clone()));
    let mut models = collection(store.clone(), source.clone());
    models.refresh(true).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_some());
    store.begin().commit(HashMap::new()).await.unwrap();
    failed.store(true, Ordering::SeqCst);
    models.refresh(false).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    models.refresh(true).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    let mut reopened = collection(ModelsStore::open(store.path()).unwrap(), source);
    reopened.refresh(false).await.unwrap();
    assert!(reopened.get_model("fixture", "remote").is_none());
}

#[tokio::test]
async fn explicit_source_tombstone_survives_failure_reopen_and_late_guard() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let failed = Arc::new(AtomicBool::new(false));
    let source = Arc::new(SwitchSource(failed.clone()));
    let mut models = collection(store.clone(), source.clone());
    models.refresh(true).await.unwrap();
    let old = store.snapshot().catalogs;
    let late = store.begin();
    store.invalidate_source("fixture-source").await.unwrap();
    assert!(matches!(
        late.commit(old).await,
        Err(opi_ai::catalog::StoreCommitError::Superseded { .. })
    ));
    failed.store(true, Ordering::SeqCst);
    models.refresh(true).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    let reopened_store = ModelsStore::open(store.path()).unwrap();
    assert!(
        reopened_store
            .snapshot()
            .invalidated_sources
            .contains("fixture-source")
    );
    let mut reopened = collection(reopened_store.clone(), source);
    reopened.refresh(false).await.unwrap();
    assert!(reopened.get_model("fixture", "remote").is_none());
    failed.store(false, Ordering::SeqCst);
    reopened.refresh(true).await.unwrap();
    assert!(reopened.get_model("fixture", "remote").is_some());
    assert!(
        !reopened_store
            .snapshot()
            .invalidated_sources
            .contains("fixture-source")
    );
}

#[tokio::test]
async fn distribution_metadata_survives_public_decode_store_and_reopen() {
    let mut decoded = opi_ai::catalog::HttpCatalogSource::decode_document(&serde_json::json!({
        "providers": {"fixture": {"models": [{
            "id": "model", "api": "openai-completions", "contextWindow": 32000, "maxTokens": 4096,
            "reasoning": true, "thinkingLevelMap": {"off": null, "xhigh": "high"},
            "headers": {"x-model": "chosen", "x-remove": null},
            "inputLimits": {"images": {"maxPerRequest": 2}},
            "samplingParams": {"top_p": 0.8}, "samplingParamsByThinkingLevel": {"high": {"temperature": 0.3}},
            "compat": {"supportsStore": false, "supportsThinkingTokenBudget": true,
                "thinkingTokenBudgetField": "thinking_budget", "chatTemplateArgs": {"enabled": {"$var": "thinking.enabled"}},
                "chatTemplateKwargs": {"effort": {"$var": "thinking.effort", "omitWhenOff": true}, "literal": false}},
            "cost": {"input": 1, "output": 2, "cacheRead": 0, "cacheWrite": 0,
                "tiers": [{"inputTokensAbove": 100, "input": 3, "output": 4, "cacheRead": 0, "cacheWrite": 0}]}
        }]}}
    })).unwrap();
    let original = catalog_fixture::chat(&decoded["fixture"].models[0]);
    let opi_ai::WireCompat::OpenAiCompletions(compat) = &original.compat else {
        panic!("chat compat");
    };
    assert_eq!(
        compat.thinking_token_budget_field,
        Some(opi_ai::model_info::ChatThinkingTokenBudgetField::ThinkingBudget)
    );
    let opi_ai::WireCompat::OpenAiCompletions(compat) =
        &mut catalog_fixture::chat_mut(&mut decoded.get_mut("fixture").unwrap().models[0]).compat
    else {
        unreachable!()
    };
    compat.chat_template_kwargs.insert(
        "literal-reference".into(),
        opi_ai::model_info::ChatTemplateKwargValue::Literal(
            serde_json::json!({"$var": "thinking.enabled", "omitWhenOff": true}),
        ),
    );
    compat.chat_template_kwargs.insert(
        "budget".into(),
        opi_ai::model_info::ChatTemplateKwargValue::Var {
            var: opi_ai::model_info::ChatTemplateKwargVar::ThinkingBudget,
            omit_when_off: false,
        },
    );
    decoded.get_mut("fixture").unwrap().models.push(
        ModelInfo::new(
            "mapless",
            "Mapless",
            WireApi::MistralConversations,
            ModelCapabilities::new(8192, 1024).with_thinking(true),
        )
        .into(),
    );
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store.begin().commit(decoded.clone()).await.unwrap();
    let restored = ModelsStore::open(store.path()).unwrap().snapshot();
    assert!(
        !catalog_fixture::chat(&restored.catalogs["fixture"].models[1])
            .thinking_level_map
            .has_explicit_mappings()
    );
    assert_eq!(restored.catalogs, decoded);

    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
    for (name, pointer, invalid) in [
        (
            "headers",
            "/providers/fixture/models/0/x-opi/headers",
            serde_json::json!([["x-mode", 3]]),
        ),
        (
            "template",
            "/providers/fixture/models/0/x-opi/compat/chatTemplateKwargs",
            serde_json::json!({"key": {"var": "unknown.variable"}}),
        ),
    ] {
        let mut malformed = document.clone();
        *malformed.pointer_mut(pointer).unwrap() = invalid;
        let path = temp.path().join(format!("malformed-{name}.json"));
        std::fs::write(&path, serde_json::to_vec(&malformed).unwrap()).unwrap();
        assert!(matches!(
            ModelsStore::open(path),
            Err(opi_ai::catalog::ModelsStoreError::Corrupt { .. })
        ));
    }
}

#[tokio::test]
async fn ordered_model_headers_survive_store_reopen_and_next_real_request() {
    use opi_ai::auth::{AuthScheme, StaticAuthResolver};
    use opi_ai::openai_chat::{OpenAiChatProvider, OpenAiCompletionsChatApi};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"))
        .expect(2)
        .mount(&server).await;
    let mut original = ModelInfo::new(
        "header-fixture",
        "Header fixture",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024).with_streaming(true),
    )
    .with_base_url(server.uri());
    original.headers = vec![
        ("x-mode".into(), Some("on".into())),
        ("X-Mode".into(), Some("off".into())),
        ("x-mode".into(), None),
    ];
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    store
        .begin()
        .commit(catalog_fixture::catalogs(HashMap::from([(
            "openai".into(),
            vec![original.clone()],
        )])))
        .await
        .unwrap();
    let restored = catalog_fixture::chat(
        &ModelsStore::open(store.path()).unwrap().snapshot().catalogs["openai"].models[0],
    )
    .clone();

    for model in [original, restored] {
        let api = OpenAiCompletionsChatApi::new(
            Arc::new(OpenAiChatProvider::new(Some(server.uri()))),
            model.clone(),
        )
        .unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from("offline-fixture-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let request = opi_ai::provider::Request {
            model: "openai:header-fixture".into(),
            context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
            max_tokens: None,
            temperature: None,
            sampling_params: Default::default(),
            thinking: Default::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: Default::default(),
            session_id: None,
            callbacks: Default::default(),
            telemetry_context: None,
            transport: opi_ai::provider::TransportPreference::Sse,
        };
        models
            .complete(
                opi_ai::ChatModelRecord::new("openai", model),
                request,
                handle.options(Default::default()),
            )
            .await
            .unwrap();
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].headers.contains_key("x-mode"));
    assert!(
        !requests[1].headers.contains_key("x-mode"),
        "reopen must preserve the later case-insensitive removal"
    );
}
