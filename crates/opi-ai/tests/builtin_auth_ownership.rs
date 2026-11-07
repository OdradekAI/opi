//! Builtin authentication rebinding preserves explicitly replaced operation owners.
use opi_ai::auth::{
    AuthContext, AuthContextError, AuthProvenanceSource, AuthResolver, LoginPresenter,
    OAuthCredential, OAuthProvider, ResolvedAuth,
};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider::{ProviderError, Request};
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::{ChatModelRecord, Models};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

struct NoAmbient;
impl AuthContext for NoAmbient {
    fn env(&self, _: &str) -> Result<Option<String>, AuthContextError> {
        Ok(None)
    }
    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}
struct CustomAuth {
    reads: Arc<AtomicUsize>,
}
impl AuthResolver for CustomAuth {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            let mut auth = opi_ai::test_support::resolved_bearer_auth();
            auth.secret = "custom-fixture-key".into();
            Ok(auth)
        })
    }
    fn check_configuration(&self) -> Option<BoxAuthFuture<'_, Result<bool, ProviderError>>> {
        Some(Box::pin(async { Ok(true) }))
    }
}
struct UnusedFlow(&'static str);
impl OAuthProvider for UnusedFlow {
    fn id(&self) -> &str {
        self.0
    }
    fn to_auth(&self, _: &OAuthCredential) -> ResolvedAuth {
        panic!("API-key fixture must not use OAuth")
    }
    fn login<'a>(
        &'a self,
        _: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        panic!("no login")
    }
    fn refresh<'a>(
        &'a self,
        _: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        panic!("no OAuth refresh")
    }
}

struct NoLoginInteraction;
impl LoginPresenter for NoLoginInteraction {
    fn present_auth_url<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        panic!("flow preflight must not interact")
    }
    fn present_device_code<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        panic!("flow preflight must not interact")
    }
    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        panic!("flow preflight must not interact")
    }
    fn notify_success(&self) {
        panic!("flow preflight must not interact")
    }
    fn notify_failure(&self, _: &str) {
        panic!("flow preflight must not interact")
    }
}

struct CancelledFlow {
    id: &'static str,
    calls: Arc<AtomicUsize>,
}

impl OAuthProvider for CancelledFlow {
    fn id(&self) -> &str {
        self.id
    }

    fn to_auth(&self, _: &OAuthCredential) -> ResolvedAuth {
        panic!("cancelled login has no credential")
    }

    fn login<'a>(
        &'a self,
        _: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(ProviderError::LoginCancelled {
                provider_id: self.id.into(),
            })
        })
    }

    fn refresh<'a>(
        &'a self,
        _: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        panic!("cancelled login must not refresh")
    }
}

#[tokio::test]
async fn remove_and_clear_keep_auth_only_flows_and_remove_model_owned_flows() {
    fn assert_cancelled(result: Result<OAuthCredential, opi_ai::ModelsError>, id: &str) {
        assert!(matches!(
            result,
            Err(opi_ai::ModelsError::AuthResolution { provider, error })
                if provider == id && matches!(
                    error.as_ref(),
                    ProviderError::LoginCancelled { provider_id } if provider_id == id
                )
        ));
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let flow = |id| {
        Arc::new(CancelledFlow {
            id,
            calls: calls.clone(),
        })
    };
    let mut models = Models::new();
    models.set_auth_context(Arc::new(NoAmbient));
    models.register_oauth_flow("auth-only", flow("auth-only"));
    for id in ["remove-me", "clear-me"] {
        models
            .register(
                Box::new(opi_ai::test_support::MockProvider::new_with_models(
                    id,
                    vec![],
                    vec![],
                )),
                opi_ai::AuthDescriptor::Resolved {
                    source: "fixture".into(),
                },
                Default::default(),
            )
            .unwrap();
        models.register_oauth_flow(id, flow(id));
        assert_cancelled(models.login(id, &NoLoginInteraction).await, id);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(!models.delete_provider("auth-only"));
    assert!(models.delete_provider("remove-me"));
    assert!(matches!(
        models.login("remove-me", &NoLoginInteraction).await,
        Err(opi_ai::ModelsError::UnknownOAuthFlow { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_cancelled(
        models.login("clear-me", &NoLoginInteraction).await,
        "clear-me",
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    models.clear_providers();
    assert!(models.provider_ids().is_empty());
    assert!(matches!(
        models.login("clear-me", &NoLoginInteraction).await,
        Err(opi_ai::ModelsError::UnknownOAuthFlow { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_cancelled(
        models.login("auth-only", &NoLoginInteraction).await,
        "auth-only",
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn removed_builtin_bindings_are_not_recreated_by_store_or_flow_rebinding() {
    use secrecy::ExposeSecret;

    let old_store = store("openrouter", "old-fixture-key").await;
    let new_store = store("openrouter", "new-fixture-key").await;
    let temp = tempfile::tempdir().unwrap();
    let catalog_store =
        opi_ai::catalog::ModelsStore::open(temp.path().join("models.json")).unwrap();
    catalog_store
        .begin()
        .commit(Default::default())
        .await
        .unwrap();
    let catalog_bytes = std::fs::read(catalog_store.path()).unwrap();
    let mut models = Models::new();
    models.set_credential_store(old_store.clone());
    models.set_models_store(catalog_store.clone());
    register_builtin(
        &mut models,
        Arc::new(NoAmbient),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert!(!models.get_models(Some("openrouter")).is_empty());
    assert!(models.delete_provider("openrouter"));
    models.set_credential_store(new_store.clone());
    models.register_oauth_flow("openrouter", Arc::new(UnusedFlow("openrouter")));
    assert!(!models.provider_ids().contains(&"openrouter"));
    assert!(models.get_all_models(Some("openrouter")).is_empty());
    assert!(models.auth_descriptor("openrouter").is_none());
    assert!(
        models
            .get_auth("openrouter", Default::default())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        models
            .get_all_available(Some("openrouter"))
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(old_store.read_count(), 0);
    assert_eq!(new_store.read_count(), 0);
    models.clear_providers();
    models.set_credential_store(old_store.clone());
    assert!(models.provider_ids().is_empty());
    assert!(models.get_all_models(None).is_empty());
    assert!(
        models
            .get_auth("openrouter", Default::default())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(old_store.read_count(), 0);
    assert_eq!(new_store.read_count(), 0);
    assert_eq!(std::fs::read(catalog_store.path()).unwrap(), catalog_bytes);
    for (credentials, expected) in [
        (&old_store, "old-fixture-key"),
        (&new_store, "new-fixture-key"),
    ] {
        let Some(Credential::ApiKey(key)) = credentials.read("openrouter").await.unwrap() else {
            panic!("removal must retain the stored credential");
        };
        assert_eq!(key.key().unwrap().expose_secret(), expected);
    }
}

#[tokio::test]
async fn all_operation_availability_preserves_builtin_oauth_restrictions_and_custom_image_owner() {
    use opi_ai::catalog::{CatalogOperation, CatalogRecord};

    let credentials = Arc::new(InMemoryCredentialStore::new());
    credentials
        .write(
            "openai",
            &Credential::OAuthToken {
                access: "expired-fixture-oauth".into(),
                refresh: "unused-fixture-refresh".into(),
                expires_at: Some(time::OffsetDateTime::UNIX_EPOCH),
                base_url: None,
                account_id: None,
                client_id: None,
                enterprise_domain: None,
                available_model_ids: None,
                scopes: None,
            },
        )
        .await
        .unwrap();
    let mut models = Models::new();
    models.set_credential_store(credentials.clone());
    register_builtin(
        &mut models,
        Arc::new(NoAmbient),
        Arc::new(HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models.register_oauth_flow("openai", Arc::new(UnusedFlow("openai")));
    let custom_reads = Arc::new(AtomicUsize::new(0));
    let mut image = opi_ai::api::openrouter_images::default_openrouter_image_record();
    image.provider_id = "openai".into();
    image.id = "custom-image".into();
    models
        .register_image_api(
            Arc::new(opi_ai::api::openrouter_images::OpenRouterImagesApi::new(
                HttpClient::new(),
                image,
            )),
            Arc::new(CustomAuth {
                reads: custom_reads.clone(),
            }),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    assert_eq!(credentials.read_count(), 0, "registration stays lazy");
    assert!(
        models
            .get_available_of_type(CatalogOperation::Classifier, Some("openai"))
            .await
            .unwrap()
            .is_empty()
    );
    let available = models.get_all_available(Some("openai")).await.unwrap();
    assert!(
        available
            .iter()
            .any(|record| matches!(record, CatalogRecord::Chat(_)))
    );
    assert!(
        available.iter().any(
            |record| matches!(record, CatalogRecord::Image(image) if image.id == "custom-image")
        )
    );
    assert!(available.iter().all(|record| match record {
        CatalogRecord::Chat(model) => model.wire_api == opi_ai::WireApi::OpenAiResponses,
        CatalogRecord::Image(image) => image.id == "custom-image",
        CatalogRecord::Classifier(_) => false,
    }));
    assert_eq!(
        custom_reads.load(Ordering::SeqCst),
        0,
        "availability must not resolve custom generation auth"
    );
}
async fn store(provider: &str, key: &str) -> Arc<InMemoryCredentialStore> {
    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write(provider, &Credential::ApiKey(key.into()))
        .await
        .unwrap();
    store
}

#[derive(Clone)]
enum Record {
    Chat(Box<ChatModelRecord>),
    Image(opi_ai::images::ImageModelRecord),
    Classifier(opi_ai::classifier::ClassifierModelRecord),
}
fn issue(
    models: &Models,
    record: &Record,
    id: &'static str,
) -> futures_util::future::BoxFuture<'static, ()> {
    let headers = vec![("x-test-call".into(), id.into())];
    match record.clone() {
        Record::Chat(record) => {
            let mut request = request();
            request.extra_headers = headers;
            let call = models.complete_simple(*record, request, Default::default());
            Box::pin(async move {
                call.await.expect("controlled chat should succeed");
            })
        }
        Record::Image(record) => {
            let call = models.generate_images(
                record,
                Default::default(),
                opi_ai::images::ImagesCall {
                    extra_headers: headers,
                    ..Default::default()
                },
            );
            Box::pin(async move {
                let result = call.await;
                assert_eq!(
                    result.stop_reason,
                    opi_ai::images::ImagesStopReason::Stop,
                    "{result:?}"
                );
            })
        }
        Record::Classifier(record) => {
            let call = models.classify(
                record,
                opi_ai::classifier::ClassifierContext {
                    state: serde_json::json!({"fixture":true}),
                    ..Default::default()
                },
                opi_ai::classifier::ClassifierCall {
                    extra_headers: headers,
                    ..Default::default()
                },
            );
            Box::pin(async move {
                let result = call.await;
                assert_eq!(
                    result.stop_reason,
                    opi_ai::classifier::ClassifierStopReason::Stop,
                    "{result:?}"
                );
            })
        }
    }
}

async fn exercise(replace: &str) {
    let provider = if replace == "classifier" {
        "openai"
    } else {
        "openrouter"
    };
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(|request:&wiremock::Request| {
        let body:serde_json::Value=serde_json::from_slice(&request.body).unwrap();
        if request.url.path().ends_with("decisions") {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"answers":[]}))
        } else if body["stream"]==true {
            let sse=if request.url.path().ends_with("responses") {
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"fixture\",\"status\":\"completed\",\"output\":[]}}\n\n"
            } else {
                "data: {\"id\":\"fixture\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
            };
            ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string(sse)
        } else {
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id":"image","choices":[]}))
        }
    }).expect(6).mount(&server).await;
    let mut models = Models::new();
    models.set_credential_store(store(provider, "builtin-old-key").await);
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(provider.into(), server.uri());
    register_builtin(
        &mut models,
        Arc::new(NoAmbient),
        Arc::new(HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let info = models
        .registry()
        .all_models()
        .into_iter()
        .find(|(id, _)| *id == provider)
        .unwrap()
        .1
        .clone();
    let chat = Record::Chat(Box::new(ChatModelRecord::new(provider, info.clone())));
    let mut image = opi_ai::api::openrouter_images::default_openrouter_image_record();
    image.base_url = server.uri();
    let image = Record::Image(image);
    let mut classifier = opi_ai::api::decisions::default_decisions_classifier_record();
    classifier.base_url = server.uri();
    let classifier = Record::Classifier(classifier);
    let (replaced, retained) = match replace {
        "chat" => (chat.clone(), image.clone()),
        "image" => (image.clone(), chat.clone()),
        "classifier" => (classifier.clone(), chat.clone()),
        _ => unreachable!(),
    };
    let old_replaced = issue(&models, &replaced, "old_replaced");
    let old_retained = issue(&models, &retained, "old_retained");
    let reads = Arc::new(AtomicUsize::new(0));
    let auth = Arc::new(CustomAuth {
        reads: reads.clone(),
    });
    match &replaced {
        Record::Chat(record) => {
            let api = opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
                Arc::new(opi_ai::openrouter::openrouter_provider(Some(server.uri()))),
                record.info().clone(),
            )
            .unwrap();
            models
                .register_chat_api(Arc::new(api), auth, AuthProvenanceSource::Static)
                .unwrap();
        }
        Record::Image(record) => {
            models
                .register_image_api(
                    Arc::new(opi_ai::api::openrouter_images::OpenRouterImagesApi::new(
                        HttpClient::new(),
                        record.clone(),
                    )),
                    auth,
                    AuthProvenanceSource::Static,
                )
                .unwrap();
        }
        Record::Classifier(record) => {
            models
                .register_classifier_api(
                    Arc::new(opi_ai::api::decisions::OpenAiDecisionsApi::new(
                        HttpClient::new(),
                        record.clone(),
                    )),
                    auth,
                    AuthProvenanceSource::Static,
                )
                .unwrap();
        }
    }
    let custom_before = issue(&models, &replaced, "custom_before");
    models.set_credential_store(store(provider, "builtin-new-key").await);
    let custom_after_store = issue(&models, &replaced, "custom_after_store");
    models.register_oauth_flow(provider, Arc::new(UnusedFlow(provider)));
    let custom_after_flow = issue(&models, &replaced, "custom_after_flow");
    let retained_after = issue(&models, &retained, "retained_after");
    old_replaced.await;
    // Keep six sends: one retained call created before attachment, three
    // custom calls across both mutations, and one current builtin operation.
    old_retained.await;
    custom_before.await;
    custom_after_store.await;
    custom_after_flow.await;
    retained_after.await;
    let requests = server.received_requests().await.unwrap();
    let keys = requests
        .iter()
        .map(|request| {
            (
                request.headers["x-test-call"].to_str().unwrap().to_owned(),
                request.headers["authorization"]
                    .to_str()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        keys,
        std::collections::BTreeMap::from([
            ("old_replaced".into(), "Bearer builtin-old-key".into()),
            ("old_retained".into(), "Bearer builtin-old-key".into()),
            ("custom_before".into(), "Bearer custom-fixture-key".into()),
            (
                "custom_after_store".into(),
                "Bearer custom-fixture-key".into()
            ),
            (
                "custom_after_flow".into(),
                "Bearer custom-fixture-key".into()
            ),
            ("retained_after".into(), "Bearer builtin-new-key".into()),
        ]),
        "operation {replace}: request identity survives independent scheduling"
    );
    assert_eq!(
        reads.load(Ordering::SeqCst),
        3,
        "every custom call retains its resolver"
    );
}

#[tokio::test]
async fn custom_chat_survives_rebinding_while_builtin_image_tracks_new_store() {
    exercise("chat").await
}
#[tokio::test]
async fn custom_image_survives_store_and_flow_rebinding_while_builtin_chat_updates() {
    exercise("image").await
}
#[tokio::test]
async fn custom_classifier_survives_store_and_flow_rebinding_while_builtin_chat_updates() {
    exercise("classifier").await
}

struct AmbientGroqKey;
impl AuthContext for AmbientGroqKey {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok((name == "GROQ_API_KEY").then(|| "synthetic-ambient".into()))
    }
    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

async fn groq_scoped_candidate(
    scoped: Option<&str>,
    key: Option<&str>,
) -> (bool, Vec<wiremock::Request>) {
    use opi_ai::credential::ApiKeyCredential;
    use secrecy::SecretString;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ))
        .mount(&server)
        .await;
    let mut models = Models::new();
    if scoped.is_some() || key.is_some() {
        let credentials = Arc::new(InMemoryCredentialStore::new());
        let env = scoped
            .map(|value| [("GROQ_API_KEY".into(), SecretString::from(value))].into())
            .unwrap_or_default();
        credentials
            .write(
                "groq",
                &Credential::ApiKey(ApiKeyCredential::new(key.map(SecretString::from), env)),
            )
            .await
            .unwrap();
        models.set_credential_store(credentials);
    }
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(
        &mut models,
        Arc::new(AmbientGroqKey),
        Arc::new(HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let info = models.get_models(Some("groq")).into_iter().next().unwrap();
    let result = models
        .complete_simple(
            ChatModelRecord::new("groq", info),
            request(),
            Default::default(),
        )
        .await;
    (result.is_ok(), server.received_requests().await.unwrap())
}

#[tokio::test]
async fn scoped_candidate_preserves_ambient_and_explicit_key_priority() {
    for (scoped, key, expected) in [
        (None, None, "Bearer synthetic-ambient"),
        (
            Some("synthetic-scoped"),
            Some("synthetic-key"),
            "Bearer synthetic-key",
        ),
    ] {
        let (ok, requests) = groq_scoped_candidate(scoped, key).await;
        assert!(ok);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].headers["authorization"], expected);
    }
}

#[tokio::test]
async fn scoped_candidate_key_wins_over_ambient_on_builtin_wire() {
    let (ok, requests) = groq_scoped_candidate(Some("synthetic-scoped"), None).await;
    assert!(ok);
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer synthetic-scoped"
    );
}

#[tokio::test]
async fn scoped_candidate_empty_masks_ambient_without_dispatch() {
    let (ok, requests) = groq_scoped_candidate(Some(""), None).await;
    assert!(
        !ok,
        "empty scoped key must fail closed; sends={}",
        requests.len()
    );
    assert!(requests.is_empty());
}

fn request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "ignored".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: Some(128),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}
