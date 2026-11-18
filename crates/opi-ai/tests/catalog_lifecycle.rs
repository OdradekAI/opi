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

mod sampling_presence {
    use opi_ai::catalog::{
        Catalog, CatalogOperation, CatalogRecord, ModelsStore, ModelsStoreEntry,
    };
    use serde_json::{Value, json};

    async fn saved(sampling: Option<Value>) -> Value {
        saved_with_extension(sampling, None).await
    }

    fn source_record(
        sampling: Option<Value>,
        extension: Option<Value>,
    ) -> opi_ai::catalog::CatalogModel {
        let mut record = Catalog::builtin()
            .unwrap()
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .unwrap()
            .clone();
        record.raw.as_object_mut().unwrap().remove("samplingParams");
        if let Some(extension) = record.raw.get_mut("x-opi").and_then(Value::as_object_mut) {
            extension.remove("samplingParams");
        }
        if let Some(value) = sampling {
            record.raw["samplingParams"] = value;
        }
        if let Some(value) = extension {
            record.raw["x-opi"]["samplingParams"] = value;
        }
        record
    }

    async fn saved_with_extension(sampling: Option<Value>, extension: Option<Value>) -> Value {
        let expected = extension
            .as_ref()
            .or(sampling.as_ref())
            .map(|value| value.as_object().unwrap().clone());
        let record = source_record(sampling, extension);
        let info = record.to_chat_model_info().unwrap();
        assert_eq!(info.sampling_params, expected);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("models.json");
        let store = ModelsStore::open(&path).unwrap();
        store
            .begin()
            .commit(
                [(
                    record.provider,
                    ModelsStoreEntry {
                        models: vec![CatalogRecord::from(info)],
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )
            .await
            .unwrap();
        let reopened = ModelsStore::open(&path).unwrap().snapshot();
        let CatalogRecord::Chat(restored) = &reopened.catalogs["anthropic"].models[0] else {
            panic!("stored chat record");
        };
        assert_eq!(restored.sampling_params, expected);
        let document: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(
            document["providers"]["anthropic"]["models"][0]["x-opi"].get("samplingParams"),
            expected
                .as_ref()
                .map(|params| Value::Object(params.clone()))
                .as_ref()
        );
        document["providers"]["anthropic"]["models"][0].clone()
    }

    #[tokio::test]
    async fn c018_missing_sampling_stays_absent_after_store_roundtrip() {
        let model = saved(None).await;
        assert!(
            model.get("samplingParams").is_none() && model["x-opi"].get("samplingParams").is_none(),
            "absent source sampling metadata must remain absent"
        );
    }

    #[tokio::test]
    async fn c018_explicit_empty_sampling_survives_store_roundtrip_control() {
        let model = saved(Some(json!({}))).await;
        assert_eq!(
            model
                .get("samplingParams")
                .or_else(|| model["x-opi"].get("samplingParams")),
            Some(&json!({}))
        );
    }

    #[tokio::test]
    async fn c018_nonempty_sampling_survives_store_roundtrip_control() {
        let model = saved(Some(json!({"temperature":0.25,"nested":{"key":true}}))).await;
        assert_eq!(
            model
                .get("samplingParams")
                .or_else(|| model["x-opi"].get("samplingParams")),
            Some(&json!({"temperature":0.25,"nested":{"key":true}}))
        );
    }

    #[tokio::test]
    async fn source_and_extension_sampling_presence_survive_owned_store_reopen() {
        for source in [
            None,
            Some(json!({})),
            Some(json!({"source":1,"nested":{"a":1}})),
        ] {
            for extension in [
                None,
                Some(json!({})),
                Some(json!({"extension":2,"nested":{"b":2}})),
            ] {
                let expected = extension.as_ref().or(source.as_ref()).cloned();
                let model = saved_with_extension(source.clone(), extension).await;
                assert_eq!(model["x-opi"].get("samplingParams"), expected.as_ref());
                assert!(model.get("samplingParams").is_none());
            }
        }
    }

    #[test]
    fn sampling_nonobject_values_remain_typed_errors() {
        for invalid in [Value::Null, json!(true), json!(3), json!("map"), json!([])] {
            for record in [
                source_record(Some(invalid.clone()), None),
                source_record(Some(json!({"source":1})), Some(invalid)),
            ] {
                assert!(matches!(
                    record.to_chat_model_info(),
                    Err(opi_ai::catalog::CatalogError::InvalidEntry { detail, .. })
                    if detail == "x-opi.samplingParams must be an object"
                ));
            }
        }
        let record = source_record(Some(json!([])), Some(json!({})));
        assert_eq!(
            record.to_chat_model_info().unwrap().sampling_params,
            Some(Default::default()),
            "a present valid extension takes precedence over the source value"
        );
    }
}

struct SwitchSource(Arc<AtomicBool>);

impl DynamicCatalogSource for SwitchSource {
    fn id(&self) -> &str {
        "fixture-source"
    }

    fn provider_ids(&self) -> &[String] {
        static IDS: std::sync::LazyLock<Vec<String>> =
            std::sync::LazyLock::new(|| vec!["fixture".into()]);
        &IDS
    }

    fn fetch(
        &self,
        _: opi_ai::catalog::CatalogRefreshContext,
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
    models.refresh(Default::default()).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_some());
    store.begin().commit(HashMap::new()).await.unwrap();
    failed.store(true, Ordering::SeqCst);
    models
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    models.refresh(Default::default()).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    let mut reopened = collection(ModelsStore::open(store.path()).unwrap(), source);
    reopened
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(reopened.get_model("fixture", "remote").is_none());
}

#[tokio::test]
async fn explicit_source_tombstone_survives_failure_reopen_and_late_guard() {
    let temp = tempfile::tempdir().unwrap();
    let store = ModelsStore::open(temp.path().join("models.json")).unwrap();
    let failed = Arc::new(AtomicBool::new(false));
    let source = Arc::new(SwitchSource(failed.clone()));
    let mut models = collection(store.clone(), source.clone());
    models.refresh(Default::default()).await.unwrap();
    let old = store.snapshot().catalogs;
    let late = store.begin();
    store.invalidate_source("fixture-source").await.unwrap();
    assert!(matches!(
        late.commit(old).await,
        Err(opi_ai::catalog::StoreCommitError::Superseded { .. })
    ));
    failed.store(true, Ordering::SeqCst);
    models.refresh(Default::default()).await.unwrap();
    assert!(models.get_model("fixture", "remote").is_none());
    let reopened_store = ModelsStore::open(store.path()).unwrap();
    assert!(
        reopened_store
            .snapshot()
            .invalidated_sources
            .contains("fixture-source")
    );
    let mut reopened = collection(reopened_store.clone(), source);
    reopened
        .refresh(opi_ai::ModelsRefreshOptions {
            allow_network: false,
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(reopened.get_model("fixture", "remote").is_none());
    failed.store(false, Ordering::SeqCst);
    reopened.refresh(Default::default()).await.unwrap();
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
    compat.chat_template_kwargs.get_or_insert_default().insert(
        "literal-reference".into(),
        opi_ai::model_info::ChatTemplateKwargValue::Literal(
            serde_json::json!({"$var": "thinking.enabled", "omitWhenOff": true}),
        ),
    );
    compat.chat_template_kwargs.get_or_insert_default().insert(
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
    use opi_ai::api::openai::OpenAiCompletionsChatApi;
    use opi_ai::auth::{AuthScheme, StaticAuthResolver};
    use opi_ai::providers::openai::OpenAiChatProvider;
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
            projection: Default::default(),
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
