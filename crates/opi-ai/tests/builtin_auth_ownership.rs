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
fn issue(models: &Models, record: &Record) -> futures_util::future::BoxFuture<'static, ()> {
    match record.clone() {
        Record::Chat(record) => {
            let call = models.complete_simple(*record, request(), Default::default());
            Box::pin(async move {
                call.await.expect("controlled chat should succeed");
            })
        }
        Record::Image(record) => {
            let call = models.generate_images(record, Default::default(), Default::default());
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
                Default::default(),
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
    let old_replaced = issue(&models, &replaced);
    let old_retained = issue(&models, &retained);
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
    let custom_before = issue(&models, &replaced);
    models.set_credential_store(store(provider, "builtin-new-key").await);
    let custom_after_store = issue(&models, &replaced);
    models.register_oauth_flow(provider, Arc::new(UnusedFlow(provider)));
    let custom_after_flow = issue(&models, &replaced);
    let retained_after = issue(&models, &retained);
    assert_eq!(reads.load(Ordering::SeqCst), 0, "creation is lazy");
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
            request.headers["authorization"]
                .to_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        vec![
            "Bearer builtin-old-key",
            "Bearer builtin-old-key",
            "Bearer custom-fixture-key",
            "Bearer custom-fixture-key",
            "Bearer custom-fixture-key",
            "Bearer builtin-new-key"
        ],
        "operation {replace}"
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
fn request() -> Request {
    Request {
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
