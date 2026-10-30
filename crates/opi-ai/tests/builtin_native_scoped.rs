//! Native builtin scoped source selection and cache isolation through public dispatch.
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::credential::{ApiKeyCredential, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider::Request;
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::{ChatModelRecord, Models};
use secrecy::{ExposeSecret, SecretString};
use std::collections::BTreeMap;
use std::sync::Arc;
use wiremock::matchers::{method, path};
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

#[tokio::test]
async fn vertex_scoped_file_account_changes_isolate_tokens_and_reuse_original_cache() {
    use opi_ai::auth::adc::native::{
        AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
    };
    let server = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let files = [dir.path().join("one.json"), dir.path().join("two.json")];
    for (file, name) in files.iter().zip(["one", "two"]) {
        std::fs::write(
            file,
            serde_json::to_vec(&serde_json::json!({
                "type":"authorized_user", "client_id":format!("client-{name}"),
                "client_secret":format!("secret-{name}"), "refresh_token":format!("refresh-{name}"),
                "quota_project_id":format!("quota-{name}")
            }))
            .unwrap(),
        )
        .unwrap();
    }
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(|request: &wiremock::Request| {
            let body = String::from_utf8_lossy(&request.body);
            let name = if body.contains("refresh-one") {
                "one"
            } else {
                assert!(body.contains("refresh-two"));
                "two"
            };
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token":format!("token-{name}"), "expires_in":3600,"token_type":"Bearer"
            }))
        })
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST")).and(wiremock::matchers::path_regex("^/v1/projects/"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type","text/event-stream")
            .set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}],\"role\":\"model\"},\"finishReason\":\"STOP\",\"index\":0}]}\n\n"))
        .expect(3).mount(&server).await;
    let store = stored("google-vertex", None, &[]).await;
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
        store.clone(),
        context(&[
            ("GOOGLE_APPLICATION_CREDENTIALS", "missing-ambient"),
            ("GOOGLE_CLOUD_PROJECT", "ambient-project"),
            ("GOOGLE_CLOUD_LOCATION", "ambient-location"),
        ]),
        overrides,
    );
    for (index, name) in [(0, "one"), (1, "two"), (0, "one")] {
        let env = [
            (
                "GOOGLE_APPLICATION_CREDENTIALS",
                files[index].to_string_lossy().into_owned(),
            ),
            ("GOOGLE_CLOUD_PROJECT", format!("project-{name}")),
            ("GOOGLE_CLOUD_LOCATION", format!("location-{name}")),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), SecretString::from(v)))
        .collect();
        store
            .write(
                "google-vertex",
                &Credential::ApiKey(ApiKeyCredential::new(None, env)),
            )
            .await
            .unwrap();
        assert!(
            !models
                .get_available(Some("google-vertex"))
                .await
                .unwrap()
                .is_empty()
        );
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
    }
    let received = server.received_requests().await.unwrap();
    let generated = received
        .iter()
        .filter(|r| r.url.path() != "/token")
        .collect::<Vec<_>>();
    for (sent, name) in generated.iter().zip(["one", "two", "one"]) {
        assert!(sent.url.path().starts_with(&format!(
            "/v1/projects/project-{name}/locations/location-{name}/"
        )));
        assert_eq!(
            sent.headers.get("authorization").unwrap(),
            &format!("Bearer token-{name}")
        );
        assert_eq!(
            sent.headers.get("x-goog-user-project").unwrap(),
            &format!("quota-{name}")
        );
    }
    assert_eq!(received.len(), 5);
    let auth = models
        .get_auth("google-vertex", Default::default())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        auth.env["GOOGLE_CLOUD_PROJECT"].expose_secret(),
        "project-one"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 5);
}

#[tokio::test]
async fn vertex_empty_scoped_project_masks_ambient_and_explicit_config_wins() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("adc.json");
    std::fs::write(&file, "intentionally-not-parsed-by-availability").unwrap();
    let store = stored(
        "google-vertex",
        None,
        &[
            ("GOOGLE_CLOUD_PROJECT", ""),
            ("GOOGLE_CLOUD_LOCATION", "owned-location"),
            ("GOOGLE_APPLICATION_CREDENTIALS", file.to_str().unwrap()),
        ],
    )
    .await;
    let ctx = context(&[("GOOGLE_CLOUD_PROJECT", "ambient-project")]);
    let mut overrides = BuiltinOverrides::default();
    let unavailable = models(store.clone(), ctx.clone(), overrides.clone());
    assert!(
        unavailable
            .get_available(Some("google-vertex"))
            .await
            .unwrap()
            .is_empty()
    );
    overrides.vertex_auth.project = Some("explicit-project".into());
    let available = models(store, ctx, overrides);
    assert!(
        !available
            .get_available(Some("google-vertex"))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn bedrock_stored_profile_file_owns_credentials_and_region_on_wire() {
    use opi_ai::auth::AuthScheme;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(serde_json::json!({"message":"controlled fixture"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    let config = dir.path().join("config");
    std::fs::write(
        &credentials,
        "[owned]\naws_access_key_id=owned-access\naws_secret_access_key=owned-secret\n",
    )
    .unwrap();
    std::fs::write(&config, "[profile owned]\nregion=us-west-2\n").unwrap();
    let store = stored(
        "amazon-bedrock",
        None,
        &[
            ("AWS_PROFILE", "owned"),
            ("AWS_SHARED_CREDENTIALS_FILE", credentials.to_str().unwrap()),
            ("AWS_CONFIG_FILE", config.to_str().unwrap()),
        ],
    )
    .await;
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.base_url = Some(server.uri());
    overrides.aws_auth.scoped.ec2_metadata_disabled = Some(true);
    let models = models(
        store,
        context(&[
            ("AWS_PROFILE", "ambient-profile"),
            ("AWS_ACCESS_KEY_ID", "ambient-access"),
            ("AWS_SECRET_ACCESS_KEY", "ambient-secret"),
        ]),
        overrides,
    );
    assert!(
        !models
            .get_available(Some("amazon-bedrock"))
            .await
            .unwrap()
            .is_empty()
    );
    let auth = models
        .get_auth("amazon-bedrock", Default::default())
        .await
        .unwrap()
        .unwrap();
    let AuthScheme::AwsSigV4(credentials) = auth.scheme else {
        panic!("selected profile must resolve SigV4")
    };
    assert_eq!(credentials.access_key_id.expose_secret(), "owned-access");
    assert_eq!(credentials.region, "us-west-2");
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
    let sent = server.received_requests().await.unwrap();
    let authorization = sent[0]
        .headers
        .get("authorization")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(authorization.contains("Credential=owned-access/"));
    assert!(authorization.contains("/us-west-2/bedrock/aws4_request"));
}

#[tokio::test]
async fn bedrock_earlier_winners_never_probe_shared_file_paths() {
    use opi_ai::auth::aws::local::AwsAuthSelection;
    use opi_ai::auth::{AuthScheme, AwsSigV4Credentials};
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Canary {
        calls: Arc<AtomicUsize>,
        pair: bool,
    }
    impl AuthContext for Canary {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            if matches!(
                name,
                "AWS_SHARED_CREDENTIALS_FILE"
                    | "AWS_CONFIG_FILE"
                    | "HOME"
                    | "USERPROFILE"
                    | "HOMEPATH"
                    | "HOMEDRIVE"
            ) {
                self.calls.fetch_add(1, Ordering::SeqCst);
                panic!("earlier winner must not consult path configuration");
            }
            Ok(match name {
                "AWS_ACCESS_KEY_ID" if self.pair => Some("ambient-access".into()),
                "AWS_SECRET_ACCESS_KEY" if self.pair => Some("ambient-secret".into()),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            panic!("earlier winner must not probe files")
        }
    }
    for kind in 0..4 {
        let store = stored(
            "amazon-bedrock",
            (kind == 1).then_some("stored-bearer"),
            &[],
        )
        .await;
        let mut overrides = BuiltinOverrides::default();
        overrides.aws_auth.region = Some("us-east-1".into());
        match kind {
            0 => {
                overrides.aws_auth.selection =
                    Some(AwsAuthSelection::SigV4(AwsSigV4Credentials::new(
                        "explicit-access".into(),
                        "explicit-secret".into(),
                        "us-east-1",
                    )))
            }
            2 => overrides.aws_auth.scoped.bearer_token_bedrock = Some("scoped-bearer".into()),
            _ => (),
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let models = models(
            store,
            Arc::new(Canary {
                calls: calls.clone(),
                pair: kind == 3,
            }),
            overrides,
        );
        assert!(
            !models
                .get_available(Some("amazon-bedrock"))
                .await
                .unwrap()
                .is_empty()
        );
        let auth = models
            .get_auth("amazon-bedrock", Default::default())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            (kind, &auth.scheme),
            (0 | 3, AuthScheme::AwsSigV4(_)) | (1 | 2, AuthScheme::Bearer)
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn bedrock_default_and_tilde_paths_use_injected_home_only() {
    use opi_ai::auth::AuthScheme;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct HomeContext {
        home: std::path::PathBuf,
        calls: Arc<AtomicUsize>,
    }
    impl AuthContext for HomeContext {
        fn env(&self, _: &str) -> Result<Option<String>, AuthContextError> {
            Ok(None)
        }
        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            panic!("availability must not probe shared files")
        }
        fn home_dir(&self) -> Result<Option<std::path::PathBuf>, AuthContextError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some(self.home.clone()))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".aws")).unwrap();
    std::fs::write(
        dir.path().join(".aws/credentials"),
        "[owned]\naws_access_key_id=home-access\naws_secret_access_key=home-secret\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".aws/config"),
        "[profile owned]\nregion=us-west-2\n",
    )
    .unwrap();
    for mode in 0..3 {
        let mut env = vec![("AWS_PROFILE", "owned")];
        if mode < 2 {
            env.push(("HOME", dir.path().to_str().unwrap()));
        }
        if mode == 1 {
            env.extend([
                ("AWS_SHARED_CREDENTIALS_FILE", "~/.aws/credentials"),
                ("AWS_CONFIG_FILE", "~/.aws/config"),
            ]);
        }
        let store = stored("amazon-bedrock", None, &env).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let context = Arc::new(HomeContext {
            home: dir.path().into(),
            calls: calls.clone(),
        });
        let mut overrides = BuiltinOverrides::default();
        overrides.aws_auth.scoped.ec2_metadata_disabled = Some(true);
        let models = models(store, context, overrides);
        assert!(
            !models
                .get_available(Some("amazon-bedrock"))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let auth = models
            .get_auth("amazon-bedrock", Default::default())
            .await
            .unwrap()
            .unwrap();
        let AuthScheme::AwsSigV4(credentials) = auth.scheme else {
            panic!("expected file credentials")
        };
        assert_eq!(credentials.access_key_id.expose_secret(), "home-access");
        assert_eq!(credentials.region, "us-west-2");
        if mode < 2 {
            assert_eq!(calls.load(Ordering::SeqCst), 0)
        } else {
            assert!(calls.load(Ordering::SeqCst) > 0)
        }
    }
}

#[tokio::test]
async fn bedrock_shared_path_context_failure_is_typed_and_does_not_select_default_files() {
    struct FailedPath;
    impl AuthContext for FailedPath {
        fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
            if name == "AWS_SHARED_CREDENTIALS_FILE" {
                return Err(AuthContextError::EnvNotUnicode { name: name.into() });
            }
            if matches!(name, "HOME" | "USERPROFILE" | "HOMEPATH" | "HOMEDRIVE") {
                panic!("failed selected path must not fall through to another file");
            }
            Ok(None)
        }
        fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> {
            panic!("no file fallback")
        }
        fn home_dir(&self) -> Result<Option<std::path::PathBuf>, AuthContextError> {
            panic!("no OS fallback")
        }
    }
    let store = stored("amazon-bedrock", None, &[("AWS_PROFILE", "owned")]).await;
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.scoped.ec2_metadata_disabled = Some(true);
    let models = models(store, Arc::new(FailedPath), overrides);
    let error = models
        .get_auth("amazon-bedrock", Default::default())
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        opi_ai::ModelsError::AuthResolution { error, .. }
            if matches!(error.as_ref(), opi_ai::provider::ProviderError::Config(_))
    ));
    assert!(
        error
            .to_string()
            .contains("shared AWS configuration context")
    );
}
