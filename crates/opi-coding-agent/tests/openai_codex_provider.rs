use std::sync::Arc;
use std::time::Duration;

use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::credential::{Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig, TransportPreference};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::{ChatModelRecord, Models, ModelsError, RegistryError, ThinkingLevel, WireApi};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
use opi_coding_agent::openai_codex::{OPENAI_CODEX_DEFAULT_BASE_URL, openai_codex_catalog};
use opi_coding_agent::provider_factory::{
    build_collection_for_listing_with_store, build_harness_collection, build_provider_bundle,
};
use serde_json::json;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PROVIDER: &str = "openai-codex";
const TERRA: &str = "gpt-6.1-terra";
const RETIRED: [&str; 4] = ["gpt-5.3-codex-spark", "gpt-5.4", "gpt-5.4-mini", "gpt-5.5"];

struct EmptySources;

impl AuthContext for EmptySources {
    fn env(&self, _: &str) -> Result<Option<String>, AuthContextError> {
        Ok(None)
    }

    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

fn credential(base_url: String, available_model_ids: Option<Vec<String>>) -> Credential {
    Credential::OAuthToken {
        access: "codex-fixture-access".into(),
        refresh: "codex-fixture-refresh".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        base_url: Some(base_url),
        account_id: Some("codex-fixture-account".into()),
        client_id: None,
        enterprise_domain: None,
        available_model_ids,
        scopes: None,
    }
}

async fn sdk_models(credential: &Credential) -> Models {
    let store = Arc::new(InMemoryCredentialStore::new());
    store.write(PROVIDER, credential).await.unwrap();
    let mut models = Models::new();
    models.set_credential_store(store);
    register_builtin(
        &mut models,
        Arc::new(EmptySources),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models
}

async fn runtime_models(
    credential: &Credential,
) -> (TempDir, Arc<KeychainCredentialStore>, Arc<Models>) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = OpiConfig::default();
    config.defaults.model = format!("{PROVIDER}:{TERRA}");
    let bundle = build_provider_bundle(
        &config,
        dir.path().to_path_buf(),
        Box::new(|| Box::new(FakeKeyringBackend::new())),
    )
    .await
    .unwrap();
    bundle.store.write(PROVIDER, credential).await.unwrap();
    let mut routes = vec![(bundle.provider, bundle.auth_resolver)];
    routes.extend(bundle.extra_routes);
    let models = build_harness_collection(
        routes,
        Vec::new(),
        Vec::new(),
        PROVIDER,
        Some(bundle.store.clone()),
    );
    (dir, bundle.store, models)
}

fn request(id: &str, level: ThinkingLevel) -> Request {
    Request {
        projection: Default::default(),
        transport: TransportPreference::Sse,
        model: format!("{PROVIDER}:{id}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            ..Default::default()
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig {
            enabled: level != ThinkingLevel::None,
            budget_tokens: None,
            level,
        },
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: Default::default(),
        timeout: Some(Duration::from_secs(5)),
        extra_headers: Vec::new(),
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn openai_codex_catalog_uses_the_published_chat_records() {
    let cli = openai_codex_catalog();
    assert_eq!(cli.len(), 8, "CLI Codex catalog count");
    let embedded: Vec<_> = Catalog::builtin()
        .unwrap()
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .map(|record| record.to_chat_model_info().unwrap())
        .collect();
    assert_eq!(cli, embedded);
    assert_eq!(
        embedded
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        [
            "gpt-5.6-luna",
            "gpt-5.6-sol",
            "gpt-5.6-terra",
            "gpt-6-astra",
            "gpt-6-luna",
            "gpt-6-sol",
            "gpt-6.1-sol",
            TERRA,
        ]
    );

    let credential = credential("https://credential.invalid".into(), None);
    let sdk = sdk_models(&credential).await;
    let (_dir, _store, runtime) = runtime_models(&credential).await;
    assert_eq!(sdk.get_models(Some(PROVIDER)), embedded);
    assert_eq!(runtime.get_models(Some(PROVIDER)), embedded);
    for model in embedded {
        assert_eq!(model.wire_api, WireApi::OpenAiCodexResponses);
        assert_eq!(
            model.base_url.as_deref(),
            Some(OPENAI_CODEX_DEFAULT_BASE_URL)
        );
        for models in [&sdk, runtime.as_ref()] {
            assert_eq!(models.get_model(PROVIDER, &model.id).as_ref(), Some(&model));
        }
    }
}

#[tokio::test]
async fn openai_codex_terra_uses_the_model_endpoint_and_published_effort() {
    let endpoint = MockServer::start().await;
    let credential_endpoint = MockServer::start().await;
    let credential = credential(credential_endpoint.uri(), None);
    let sdk = sdk_models(&credential).await;
    let (_dir, _store, runtime) = runtime_models(&credential).await;

    Mock::given(method("POST"))
        .and(path("/codex/responses"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(
                concat!(
                    "event: response.completed\n",
                    r#"data: {"type":"response.completed","response":{"id":"terra-fixture","status":"completed","model":"gpt-6.1-terra","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
                    "\n\n",
                ),
                "text/event-stream",
            ),
        )
        .expect(6)
        .mount(&endpoint)
        .await;

    let cases = [
        (ThinkingLevel::None, json!({"effort":"none"})),
        (
            ThinkingLevel::Minimal,
            json!({"effort":"low","summary":"auto"}),
        ),
        (ThinkingLevel::Max, json!({"effort":"max","summary":"auto"})),
    ];
    for models in [&sdk, runtime.as_ref()] {
        let record = models.get_model(PROVIDER, TERRA).unwrap();
        let model = ChatModelRecord::new(PROVIDER, record.with_base_url(endpoint.uri()));
        for (level, _) in &cases {
            let result = models
                .complete_simple(model.clone(), request(TERRA, *level), Default::default())
                .await
                .unwrap();
            assert_eq!(result.stop_reason, opi_ai::stream::StopReason::Stop);
            assert_eq!(result.model, TERRA);
        }
    }

    let sent = endpoint.received_requests().await.unwrap();
    assert_eq!(sent.len(), 6);
    for (request, (_, reasoning)) in sent.iter().zip(cases.iter().cycle()) {
        assert_eq!(request.url.path(), "/codex/responses");
        assert_eq!(
            request.headers["authorization"],
            "Bearer codex-fixture-access"
        );
        assert_eq!(
            request.headers["chatgpt-account-id"],
            "codex-fixture-account"
        );
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], TERRA);
        assert_eq!(body["stream"], true);
        assert_eq!(&body["reasoning"], reasoning);
    }
    assert!(
        credential_endpoint
            .received_requests()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn openai_codex_retired_models_are_unavailable_and_rejected_before_dispatch() {
    let server = MockServer::start().await;
    let account_models = [TERRA]
        .into_iter()
        .chain(RETIRED)
        .map(str::to_owned)
        .collect();
    let credential = credential(server.uri(), Some(account_models));
    let sdk = sdk_models(&credential).await;
    let (_dir, store, runtime) = runtime_models(&credential).await;
    let listing = build_collection_for_listing_with_store(&OpiConfig::default(), store.as_ref())
        .await
        .unwrap();
    let catalog = Catalog::builtin().unwrap();

    let available = sdk.get_available(Some(PROVIDER)).await.unwrap();
    assert_eq!(
        available,
        vec![(PROVIDER.into(), sdk.get_model(PROVIDER, TERRA).unwrap())]
    );
    for id in RETIRED {
        assert!(
            catalog
                .model(PROVIDER, CatalogOperation::Chat, id)
                .is_none(),
            "{id}"
        );
        assert!(listing.get_model(PROVIDER, id).is_none(), "{id}");
        for models in [&sdk, runtime.as_ref()] {
            let call = request(id, ThinkingLevel::None);
            let spec = call.model.clone();
            assert!(models.get_model(PROVIDER, id).is_none(), "{spec}");
            assert!(
                matches!(
                    models.prepare_call(&spec, call).await,
                    Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                        if provider == PROVIDER && model == id
                ),
                "{spec} must fail during registry lookup"
            );
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn openai_codex_pricing_tiers_keep_equality_on_base_rate() {
    for model in openai_codex_catalog() {
        let pricing = model.pricing.unwrap();
        assert_eq!(pricing.effective(272_000), pricing.base, "{}", model.id);
        assert_ne!(pricing.effective(272_001), pricing.base, "{}", model.id);
    }
}
