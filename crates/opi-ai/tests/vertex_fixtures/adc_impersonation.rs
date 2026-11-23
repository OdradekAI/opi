//! ADC service-account impersonation assertions.

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

const TARGET: &str = "target@proj.iam.gserviceaccount.com";

#[derive(Default)]
struct MapContext(HashMap<String, String>);

impl AuthContext for MapContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.0.get(name).cloned())
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
    impersonation_endpoint: Option<String>,
) -> Arc<AdcNativeOwner> {
    Arc::new(
        AdcNativeOwner::new(
            selection,
            Arc::new(ReqwestAdcTransport::new()),
            Arc::new(SystemAdcClock),
        )
        .with_ambient(ambient)
        .with_token_endpoint(token_endpoint)
        .with_impersonation_endpoint(impersonation_endpoint),
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

fn impersonated_creds(path: &std::path::Path, source_json: &str, quota: Option<&str>) {
    let quota_field = quota
        .map(|q| format!(r#","quota_project_id":"{q}""#))
        .unwrap_or_default();
    std::fs::write(
        path,
        format!(
            r#"{{"type":"impersonated_service_account","source_credentials":{source_json},"service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"{quota_field}}}"#
        ),
    )
    .unwrap();
}

#[tokio::test]
async fn impersonated_authorized_user_streams_bearer_with_quota_and_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("imp.json");
    impersonated_creds(
        &creds,
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        Some("quota-proj"),
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/source/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )))
        .and(header("authorization", "Bearer sourcetok"))
        .and(body_string_contains(
            "\"https://www.googleapis.com/auth/cloud-platform\"",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    let generation = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("authorization", "Bearer imptok"))
        .and(header("x-goog-user-project", "quota-proj"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&generation)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/source/token", server.uri())),
                Some(format!(
                    "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                    server.uri()
                )),
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
            source: AdcCredentialSource::Impersonated {
                source: Box::new(AdcCredentialSource::AuthorizedUser {
                    quota_project: None,
                }),
                target_principal: TARGET.to_owned(),
                delegates: 0,
                quota_project: Some("quota-proj".to_owned()),
            }
        }
    );
    assert_eq!(resolved.secret.expose_secret(), "imptok");

    drain_to_done(&models).await;
}

#[tokio::test]
async fn owner_caches_the_impersonated_chain_across_streams() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("imp.json");
    impersonated_creds(
        &creds,
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        None,
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/source/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
        )
        // Two streams, one acquisition chain: the keyed cache serves the
        // second call without a new source refresh or impersonation.
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    let generation = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .respond_with(sse_ok())
        .expect(2)
        .mount(&generation)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/source/token", server.uri())),
                Some(format!(
                    "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                    server.uri()
                )),
            ),
        },
    );

    drain_to_done(&models).await;
    drain_to_done(&models).await;
}

#[tokio::test]
async fn external_federation_then_impersonation_streams_bearer() {
    let dir = tempfile::tempdir().unwrap();
    let subject_file = dir.path().join("subject.txt");
    std::fs::write(&subject_file, "k8s-projected-token").unwrap();
    let creds = dir.path().join("external.json");
    std::fs::write(
        &creds,
        format!(
            r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken","service_account_impersonation":{{"token_lifetime_seconds":2700}},"credential_source":{{"file":{}}}}}"#,
            serde_json::to_string(&subject_file).unwrap()
        ),
    )
    .unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/token"))
        .and(body_string_contains("subject_token=k8s-projected-token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"fedtok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )))
        .and(header("authorization", "Bearer fedtok"))
        .and(body_string_contains("\"lifetime\":\"2700s\""))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"accessToken":"imptok","expireTime":"2035-01-01T00:00:00Z"}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    let generation = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(header("authorization", "Bearer imptok"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&generation)
        .await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/v1/token", server.uri())),
                Some(format!(
                    "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                    server.uri()
                )),
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
            source: AdcCredentialSource::Impersonated {
                source: Box::new(AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::File,
                    workforce_user_project: None,
                }),
                target_principal: TARGET.to_owned(),
                delegates: 0,
                quota_project: None,
            }
        }
    );
    assert_eq!(resolved.secret.expose_secret(), "imptok");

    drain_to_done(&models).await;
}

#[tokio::test]
async fn impersonation_failure_sends_zero_generation() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("imp.json");
    impersonated_creds(
        &creds,
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
        None,
    );

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/source/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"sourcetok","expires_in":3600}"#),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )))
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_string(r#"{"error":{"status":"PERMISSION_DENIED"}}"#),
        )
        .mount(&server)
        .await;
    let generation = MockServer::start().await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/source/token", server.uri())),
                Some(format!(
                    "{}/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken",
                    server.uri()
                )),
            ),
        },
    );

    let resolved = models.get_auth("vertex", Default::default()).await;
    assert!(matches!(
        resolved,
        Err(opi_ai::models::ModelsError::AuthResolution { .. })
    ));
    assert!(generation.received_requests().await.unwrap().is_empty());
}
