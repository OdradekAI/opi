//! External ADC credential source assertions.

use futures_util::StreamExt;
use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
};
use opi_ai::auth::{
    AdcCredentialSource, AuthContext, AuthContextError, AuthResolver, ExternalAccountSupplier,
};
use opi_ai::models::CompatMetadata;
use opi_ai::providers::vertex::VertexProvider;
use opi_ai::registry::ProviderRegistry;
use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const AUDIENCE: &str =
    "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o";
const SUBJECT_TOKEN_TYPE: &str = "urn:ietf:params:oauth:token-type:jwt";

struct MapContext {
    env: HashMap<String, String>,
    process_dir: tempfile::TempDir,
}

impl Default for MapContext {
    fn default() -> Self {
        Self {
            env: HashMap::new(),
            process_dir: tempfile::tempdir().unwrap(),
        }
    }
}

impl MapContext {
    fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl AuthContext for MapContext {
    fn process_snapshot(&self) -> Result<opi_ai::auth::AuthProcessSnapshot, AuthContextError> {
        Ok(opi_ai::test_support::portable_auth_process_snapshot(
            self.process_dir.path(),
        ))
    }
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.env.get(name).cloned())
    }
    fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

struct AdcRouteResolver {
    owner: Arc<AdcNativeOwner>,
}

impl AuthResolver for AdcRouteResolver {
    fn resolve<'a>(
        &'a self,
    ) -> BoxAuthFuture<'a, Result<ResolvedAuth, opi_ai::provider::ProviderError>> {
        Box::pin(async move {
            let token = self.owner.resolve_token().await.map_err(map_adc_error)?;
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: opi_ai::auth::AuthScheme::Bearer,
                secret: token.access_token,
                base_url: None,
                account_id: None,
                provenance: AuthProvenance {
                    source: AuthProvenanceSource::Adc {
                        source: token.source,
                    },
                    fallback: opi_ai::auth::AuthFallback::NotAttempted,
                },
            })
        })
    }
}

fn map_adc_error(
    error: opi_ai::auth::adc::native::AdcResolutionError,
) -> opi_ai::provider::ProviderError {
    use opi_ai::auth::adc::native::AdcResolutionError;
    use opi_ai::provider::{ProviderError, ProviderErrorSummary};
    match error {
        AdcResolutionError::NotConfigured => ProviderError::CredentialNeeded {
            provider_id: "vertex".to_owned(),
        },
        other => ProviderError::Config(ProviderErrorSummary::from_untrusted(other.safe_summary())),
    }
}

fn owner_for(
    selection: AdcAuthSelection,
    ambient: Arc<dyn AuthContext>,
    token_endpoint: Option<String>,
) -> Arc<AdcNativeOwner> {
    Arc::new(
        AdcNativeOwner::new(
            selection,
            Arc::new(ReqwestAdcTransport::new()),
            Arc::new(SystemAdcClock),
        )
        .with_ambient(ambient)
        .with_token_endpoint(token_endpoint),
    )
}

fn vertex_models(provider: VertexProvider, resolver: AdcRouteResolver) -> Models {
    let mut registry = ProviderRegistry::new();
    registry
        .register_provider(Box::new(VertexProvider::new(
            "p1".into(),
            "us-central1".into(),
            None,
        )))
        .unwrap();
    let mut models = Models::from_registry(registry);
    models
        .register_route(
            Box::new(provider),
            Arc::new(resolver),
            AuthProvenanceSource::Adc {
                source: AdcCredentialSource::ServiceAccount,
            },
            CompatMetadata::default(),
        )
        .unwrap();
    models
}

fn sse_ok() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string(super::text_sse_fixture())
        .insert_header("content-type", "text/event-stream")
}

fn external_json(credential_source: &str) -> String {
    format!(
        r#"{{"type":"external_account","audience":"{AUDIENCE}","subject_token_type":"{SUBJECT_TOKEN_TYPE}","credential_source":{credential_source}}}"#
    )
}

async fn drain_to_done(models: &Models) {
    let mut request = super::lifecycle_text_request();
    request.model = "vertex:gemini-2.5-flash".into();
    request.cancel = CancellationToken::new();
    let prepared = models
        .prepare_call("vertex:gemini-2.5-flash", request)
        .await
        .expect("prepare call");
    let mut attempt = prepared.start_attempt().expect("attempt starts");
    while let Some(item) = attempt.next().await {
        let event = item.expect("stream item");
        if matches!(
            event,
            opi_ai::stream::AssistantStreamEvent::Done { .. }
                | opi_ai::stream::AssistantStreamEvent::Error { .. }
        ) {
            assert!(!matches!(
                event,
                opi_ai::stream::AssistantStreamEvent::Error { .. }
            ));
            break;
        }
    }
}

#[tokio::test]
async fn external_file_supplier_streams_bearer() {
    let dir = tempfile::tempdir().unwrap();
    let subject_file = dir.path().join("subject.txt");
    std::fs::write(&subject_file, "k8s-projected-token").unwrap();
    let creds = dir.path().join("external.json");
    std::fs::write(
        &creds,
        external_json(&format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&subject_file).unwrap()
        )),
    )
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/token"))
        .and(body_string_contains(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Atoken-exchange",
        ))
        .and(body_string_contains("audience=%2F%2Fiam.googleapis.com"))
        .and(body_string_contains("subject_token=k8s-projected-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("authorization", "Bearer fedtok"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&server)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/v1/token", server.uri())),
            ),
        },
    );

    let resolved = models
        .get_auth("vertex", Default::default())
        .await
        .expect("configured")
        .expect("resolves");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::Adc {
            source: AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::File,
                workforce_user_project: None,
            }
        }
    );
    assert_eq!(resolved.secret.expose_secret(), "fedtok");

    drain_to_done(&models).await;
}

#[tokio::test]
async fn external_sts_failure_sends_zero_generation() {
    let dir = tempfile::tempdir().unwrap();
    let subject_file = dir.path().join("subject.txt");
    std::fs::write(&subject_file, "s").unwrap();
    let creds = dir.path().join("external.json");
    std::fs::write(
        &creds,
        external_json(&format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&subject_file).unwrap()
        )),
    )
    .unwrap();

    let sts = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_grant"}"#))
        .mount(&sts)
        .await;
    let generation = MockServer::start().await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/v1/token", sts.uri())),
            ),
        },
    );

    let error = models
        .get_auth("vertex", Default::default())
        .await
        .expect_err("failed supplier/STS fails the resolution");
    assert!(matches!(
        error,
        opi_ai::models::ModelsError::AuthResolution { .. }
    ));
    assert!(generation.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn external_executable_streams_bearer() {
    let dir = tempfile::tempdir().unwrap();
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let response = format!(
        r#"{{"version":1,"success":true,"token_type":"{SUBJECT_TOKEN_TYPE}","id_token":"pluggable-token","expiration_time":{expires}}}"#
    );
    // Platform bodies stay plain strings; the executable command is a
    // direct (quote-aware) argv, never a shell re-parse.
    let unix = format!("printf '%s' '{}'", response);
    let windows = format!("Write-Host -NoNewline '{}'", response);
    let command =
        opi_ai::test_support::portable_executable_script(dir.path(), "helper", &unix, &windows);
    let creds = dir.path().join("external.json");
    std::fs::write(
        &creds,
        external_json(&format!(
            r#"{{"executable":{{"command":"{}"}}}}"#,
            command.replace('\\', "\\\\").replace('"', "\\\"")
        )),
    )
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/token"))
        .and(body_string_contains("subject_token=pluggable-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"exetok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("authorization", "Bearer exetok"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&server)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(
                    MapContext::default()
                        .with_env("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1"),
                ),
                Some(format!("{}/v1/token", server.uri())),
            ),
        },
    );

    let resolved = models
        .get_auth("vertex", Default::default())
        .await
        .expect("configured")
        .expect("resolves");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::Adc {
            source: AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Executable,
                workforce_user_project: None,
            }
        }
    );
    drain_to_done(&models).await;
}

#[tokio::test]
async fn external_aws_env_streams_bearer() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("external.json");
    std::fs::write(
        &creds,
        external_json(
            r#"{"environment_id":"aws1","regional_cred_verification_url":"https://sts.{region}.example.dev?Action=GetCallerIdentity&Version=2011-06-15"}"#,
        ),
    )
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/token"))
        .and(body_string_contains("GetCallerIdentity"))
        .and(body_string_contains(urlencoded(AUDIENCE)))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"awstok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("authorization", "Bearer awstok"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&server)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(
                    MapContext::default()
                        .with_env("AWS_REGION", "us-east-1")
                        .with_env("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE")
                        .with_env("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI")
                        .with_env("AWS_SESSION_TOKEN", "session-token"),
                ),
                Some(format!("{}/v1/token", server.uri())),
            ),
        },
    );

    let resolved = models
        .get_auth("vertex", Default::default())
        .await
        .expect("configured")
        .expect("resolves");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::Adc {
            source: AdcCredentialSource::ExternalAccount {
                supplier: ExternalAccountSupplier::Aws,
                workforce_user_project: None,
            }
        }
    );
    drain_to_done(&models).await;
}

fn urlencoded(value: &str) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .append_pair("", value)
        .finish()
        .trim_start_matches('=')
        .to_owned()
}
