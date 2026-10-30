//! Stored scoped configuration through real builtin dispatch, without ambient credentials.
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::credential::{ApiKeyCredential, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider::Request;
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::{ChatModelRecord, Models, WireApi, WireCompat};
use secrecy::{ExposeSecret, SecretString};
use std::collections::BTreeMap;
use std::sync::Arc;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct Context(BTreeMap<String, String>);
impl AuthContext for Context {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.0.get(name).cloned())
    }
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(std::path::Path::new(path).is_file())
    }
}
fn context(values: &[(&str, &str)]) -> Arc<dyn AuthContext> {
    Arc::new(Context(
        values
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    ))
}
async fn stored(
    provider: &str,
    key: Option<&str>,
    env: &[(&str, &str)],
) -> Arc<InMemoryCredentialStore> {
    let store = Arc::new(InMemoryCredentialStore::new());
    store
        .write(
            provider,
            &Credential::ApiKey(ApiKeyCredential::new(
                key.map(SecretString::from),
                env.iter()
                    .map(|(k, v)| (k.to_string(), SecretString::from(*v)))
                    .collect(),
            )),
        )
        .await
        .unwrap();
    store
}
fn models(
    store: Arc<InMemoryCredentialStore>,
    ctx: Arc<dyn AuthContext>,
    overrides: BuiltinOverrides,
) -> Models {
    let mut models = Models::new();
    models.set_credential_store(store);
    register_builtin(&mut models, ctx, Arc::new(HttpClient::new()), &overrides).unwrap();
    models
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
fn sse(wire: WireApi) -> &'static str {
    match wire {
        WireApi::OpenAiCompletions => {
            "data: {\"id\":\"scope\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
        }
        WireApi::AzureOpenAiResponses => {
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"scope\",\"status\":\"completed\",\"output\":[]}}\n\n"
        }
        _ => panic!("fixture wire"),
    }
}
#[tokio::test]
async fn cloudflare_fields_merge_and_empty_scope_masks_ambient() {
    let store = stored(
        "cloudflare-ai-gateway",
        Some("stored-key"),
        &[
            ("CLOUDFLARE_ACCOUNT_ID", "owned-account"),
            ("CLOUDFLARE_GATEWAY_ID", "owned-gateway"),
        ],
    )
    .await;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("cf-aig-authorization", "Bearer stored-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sse(WireApi::OpenAiCompletions)))
        .expect(1)
        .mount(&server)
        .await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(
        "cloudflare-ai-gateway".into(),
        format!("{}/compat", server.uri()),
    );
    let models = models(
        store.clone(),
        context(&[
            ("CLOUDFLARE_API_KEY", "ambient-key"),
            ("CLOUDFLARE_ACCOUNT_ID", "ambient-account"),
            ("CLOUDFLARE_GATEWAY_ID", "ambient-gateway"),
        ]),
        overrides,
    );
    let auth = models
        .get_auth("cloudflare-ai-gateway", Default::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://gateway.ai.cloudflare.com/v1/owned-account/owned-gateway")
    );
    assert_eq!(
        auth.env["CLOUDFLARE_ACCOUNT_ID"].expose_secret(),
        "owned-account"
    );
    let info = models
        .registry()
        .all_models()
        .into_iter()
        .find(|(id, m)| *id == "cloudflare-ai-gateway" && m.wire_api == WireApi::OpenAiCompletions)
        .unwrap()
        .1
        .clone();
    models
        .complete_simple(
            ChatModelRecord::new("cloudflare-ai-gateway", info),
            request(),
            Default::default(),
        )
        .await
        .unwrap();
    store
        .write(
            "cloudflare-ai-gateway",
            &Credential::ApiKey(ApiKeyCredential::new(
                Some("stored-key".into()),
                BTreeMap::from([
                    ("CLOUDFLARE_ACCOUNT_ID".into(), "".into()),
                    ("CLOUDFLARE_GATEWAY_ID".into(), "owned-gateway".into()),
                ]),
            )),
        )
        .await
        .unwrap();
    assert!(
        models
            .get_auth("cloudflare-ai-gateway", Default::default())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
#[tokio::test]
async fn azure_scoped_endpoint_version_deployment_reach_both_real_wires() {
    for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sse(wire)))
            .expect(1)
            .mount(&server)
            .await;
        let base = format!("{}/openai/v1", server.uri());
        let store = stored(
            "azure",
            Some("scoped-key"),
            &[
                ("AZURE_OPENAI_BASE_URL", &base),
                ("AZURE_OPENAI_API_VERSION", "scoped-version"),
                (
                    "AZURE_OPENAI_DEPLOYMENT_NAME_MAP",
                    "gpt-4.1=owned-deployment",
                ),
            ],
        )
        .await;
        let models = models(
            store,
            context(&[
                ("AZURE_OPENAI_BASE_URL", "invalid ambient endpoint"),
                ("AZURE_OPENAI_API_VERSION", "ambient-version"),
                (
                    "AZURE_OPENAI_DEPLOYMENT_NAME_MAP",
                    "gpt-4.1=ambient-deployment",
                ),
            ]),
            Default::default(),
        );
        let mut info = models.get_model("azure", "gpt-4.1").unwrap();
        info.base_url = None;
        info.wire_api = wire;
        info.compat = match wire {
            WireApi::OpenAiCompletions => WireCompat::OpenAiCompletions(Default::default()),
            _ => WireCompat::AzureOpenAiResponses(Default::default()),
        };
        models
            .complete_simple(
                ChatModelRecord::new("azure", info),
                request(),
                Default::default(),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["model"], "owned-deployment");
        assert_eq!(
            sent[0].url.path(),
            if wire == WireApi::OpenAiCompletions {
                "/openai/v1/chat/completions"
            } else {
                "/openai/v1/responses"
            }
        );
        if wire == WireApi::AzureOpenAiResponses {
            assert!(
                sent[0]
                    .url
                    .query_pairs()
                    .any(|(k, v)| k == "api-version" && v == "scoped-version")
            );
        }
    }
}
#[tokio::test]
async fn bedrock_env_only_bearer_and_region_reach_real_transport() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer scoped-bearer"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"message":"controlled fixture"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let store = stored(
        "amazon-bedrock",
        None,
        &[
            ("AWS_BEARER_TOKEN_BEDROCK", "scoped-bearer"),
            ("AWS_REGION", "us-west-2"),
        ],
    )
    .await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("amazon-bedrock".into(), server.uri());
    let models = models(
        store,
        context(&[
            ("AWS_BEARER_TOKEN_BEDROCK", "ambient-bearer"),
            ("AWS_REGION", "us-east-1"),
        ]),
        overrides,
    );
    let auth = models
        .get_auth("amazon-bedrock", Default::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(auth.secret.expose_secret(), "scoped-bearer");
    let info = models
        .registry()
        .all_models()
        .into_iter()
        .find(|(id, _)| *id == "amazon-bedrock")
        .unwrap()
        .1
        .clone();
    assert!(
        models
            .complete_simple(
                ChatModelRecord::new("amazon-bedrock", info),
                request(),
                Default::default()
            )
            .await
            .is_err()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
#[tokio::test]
async fn bedrock_partial_stored_pair_never_borrows_an_ambient_half() {
    let store = stored(
        "amazon-bedrock",
        None,
        &[("AWS_ACCESS_KEY_ID", "scoped-access")],
    )
    .await;
    let models = models(
        store,
        context(&[
            ("AWS_ACCESS_KEY_ID", "ambient-access"),
            ("AWS_SECRET_ACCESS_KEY", "ambient-secret"),
            ("AWS_REGION", "us-east-1"),
        ]),
        Default::default(),
    );
    assert!(
        models
            .get_auth("amazon-bedrock", Default::default())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn vertex_stored_adc_config_stays_offline_until_actual_generation() {
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("owned-adc.json");
    std::fs::write(&file,serde_json::to_vec(&serde_json::json!({"type":"authorized_user","client_id":"fixture-client","client_secret":"fixture-secret","refresh_token":"fixture-refresh","quota_project_id":"owned-quota"})).unwrap()).unwrap();
    Mock::given(method("POST")).and(path("/token")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"owned-adc-token","token_type":"Bearer","expires_in":3600}))).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/v1/projects/owned-project/locations/owned-location/publishers/google/models/gemini-2.5-flash:streamGenerateContent"))
 .and(header("authorization","Bearer owned-adc-token")).and(header("x-goog-user-project","owned-quota"))
 .respond_with(ResponseTemplate::new(200).insert_header("content-type","text/event-stream").set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}]}\n\n")).expect(1).mount(&server).await;
    let store = stored(
        "google-vertex",
        None,
        &[
            ("GOOGLE_CLOUD_PROJECT", "owned-project"),
            ("GOOGLE_CLOUD_LOCATION", "owned-location"),
            ("GOOGLE_APPLICATION_CREDENTIALS", file.to_str().unwrap()),
        ],
    )
    .await;
    let mut overrides = BuiltinOverrides::default();
    overrides.vertex_auth.base_url = Some(server.uri());
    overrides.vertex_auth.adc = Some(
        AdcNativeOwner::new(
            AdcAuthSelection::Auto,
            Arc::new(ReqwestAdcTransport::new()),
            Arc::new(SystemAdcClock),
        )
        .with_token_endpoint(Some(format!("{}/token", server.uri()))),
    );
    let models = models(
        store,
        context(&[
            ("GOOGLE_CLOUD_PROJECT", "ambient-project"),
            ("GOOGLE_CLOUD_LOCATION", "ambient-location"),
            ("GOOGLE_APPLICATION_CREDENTIALS", "missing-ambient-file"),
        ]),
        overrides,
    );
    assert!(
        !models
            .get_available(Some("google-vertex"))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    let info = models
        .get_model("google-vertex", "gemini-2.5-flash")
        .unwrap();
    models
        .complete_simple(
            ChatModelRecord::new("google-vertex", info),
            request(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}
