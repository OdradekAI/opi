//! Vertex ADC native source assertions.

use futures_util::StreamExt;
use opi_ai::auth::adc::native::{AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport};
use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError, AuthResolver};
use opi_ai::models::CompatMetadata;
use opi_ai::providers::vertex::VertexProvider;
use opi_ai::registry::ProviderRegistry;
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::{AuthProvenance, AuthProvenanceSource, BoxAuthFuture, Models, ResolvedAuth};
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Non-secret synthetic admission key (RDF-23 sealed corpus; copy of the
/// `pub(crate)` crypto fixture so the integration test can sign through
/// the public owner).
const SERVICE_ACCOUNT_PEM: &str = r###"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCgvXz9l8JQ6kC1
E5/ZBuDBqumYD03J6pq9twlaSZ4cDCSOhadPVn8y9yayRNxEDZ25eWObVr5SrQvJ
QnlnKmN4bIXKThvkFxzGWTX96ESZbBBg2Fbv3EjkaRaE+H8WBUtc/Hfn5av8CfKj
x9GOwSzW3VkBPW4LhBSCbXLB07f4hgQ6tMyTaejx5nNBzYtD8SsTW9LnpW4GE2GX
z+U1axSU8WFLfgUzVIXZMSrHY0rwOSwCcSicgbNGd8uIEIv0juOyakaRrPlIk5Ce
s+BYC2dT/SEDJd7VqH98qAw9KxOMmGw9rD7odHafXIB8VmQsTsm53N0KdrsufhJO
32YWIsWFAgMBAAECggEACR6ES+Z99kq6mztO3zMXcbgBUtM9ccuO9gfqGSv3iHZS
OvQPXajV2b2b/ahP9HFu+xHitNz7OmQJuOQ9+ycfEmWFF/ON/Ab+ptcdVndvWuKU
BvaHGh+90pg5WEJj2As6rxQTYRM5NQ5DFzJz1lzpxGfgOo8yYidfECuRmOMAJd0s
3k8FZ/y6nshQgfQ0Lw96aFGfCu8Ycu+3K7p8mh+wH1Bq5oVeXgExlcdO6DLXjtQO
vq0H52mBEHKTG7uiJRadPwoYL8gNhyMhg8hDMUX0w6cUIUchJkLygEaa6NElisMZ
jnSo+gcvHDl2LGQrknineuwgJT12SQ1mtnhltp//eQKBgQDQR5ssNInh/sMOxD8o
ZbuT1iRYMQYeNZqLZeJjstMtQHy3Ul1NH2REImStR1Y5dJRO3d1vRiGIZZx++SEp
44QXdfKin0IfW9w89+2KrZrxfk6eraWHPbYX6FRCob2cfm84RgI+P9YQ7jNLoiix
264MjWJ59Qoyl+ZxTFKMh6637QKBgQDFkYJQSeOApt2OUqtbozJ6NuiyzqpTJVDd
tx4/gGGwKtNm5c1FhYBF6n3WEq261l3drvDC6ucnoN2f7eItcS7Owv5Vylzx1oAM
8KPr59WM96phpnugAWBrOYa4UyJYUxn1eVyjx9l986IeaSIXdhXa32bVfd220Ouu
CC7Lv/1g+QKBgQCugA/mDW6oRcYvDQTNV868oeF5cjlp/gcpW4c9eImtlCXpgbxh
7TcYTqiC9Q2svZu0J/ZqUBuwJ1Hin9/FNLVq4osS7NN9an0vJ5mBYso6IlTEZUAV
+wIhLanK2a/++X89aopelSVIsgmQCKCHw0IjiiFDhKWylcmgiw1uaA9eUQKBgAaE
gFoJtfmeh5yUWZbSUW36tb6AVtobVwmZauQJZi+bmhHA7v6mIs8rpy8Suqx5OeuA
Q+0KNsn14pnODTIWk8a6ZA2MeXh47QyAO8gig8b8vKAwD8/9e6LN6QXXqEBlrWuN
r2Kvyxr+mJCMQ1WmzZ3E1O2iE1hemAfLuU97a+5xAoGBAKoGwyrOChBroJmIcMc7
RYpC88yq0FXkN4rXD1o/3n0CUzeeb1pad2wlt0efNCWRU5E2dM13aBxdQvg0kGNq
f1JdzC/2b2W3wiseeVHZn7MSsny0JpfbveYtQ9xbrmmRY2amtuMzS/jbnrIKQ+wv
7VCl8xqVmjOIpkF7O/B26nBb
-----END PRIVATE KEY-----"###;

fn service_account_json() -> String {
    let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
    format!(
        r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}"}}"#
    )
}

fn authorized_user_json() -> String {
    r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","quota_project_id":"quota-proj"}"#.to_owned()
}

#[derive(Default)]
struct MapContext {
    env: HashMap<String, String>,
}

impl MapContext {
    fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.insert(name.to_owned(), value.to_owned());
        self
    }
}

impl AuthContext for MapContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.env.get(name).cloned())
    }
    fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

/// Public-path resolver: the ADC owner resolves, provenance carries the
/// selected source identity, and the Vertex adapter consumes the Bearer
/// secret at its wire boundary.
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
            Arc::new(opi_ai::auth::adc::native::SystemAdcClock),
        )
        .with_ambient(ambient)
        .with_token_endpoint(token_endpoint),
    )
}

/// Models with the mock-backed provider as the streaming route and a
/// default-catalog instance for listing. The route provider is the one
/// that streams; it must carry the mock base URL.
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

async fn drain_to_done(models: &Models) -> AssistantStreamEvent {
    let mut request = super::lifecycle_text_request();
    request.model = "vertex:gemini-2.5-flash".into();
    request.cancel = CancellationToken::new();
    let prepared = models
        .prepare_call("vertex:gemini-2.5-flash", request)
        .await
        .expect("prepare call");
    let mut attempt = prepared.start_attempt().expect("attempt starts");
    let mut done = None;
    while let Some(item) = attempt.next().await {
        let event = item.expect("stream item");
        if matches!(event, AssistantStreamEvent::Done { .. }) {
            done = Some(event);
            break;
        }
    }
    done.expect("stream reaches Done")
}

#[tokio::test]
async fn express_api_key_uses_key_header_and_projectless_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/v1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
        ))
        .and(query_param("alt", "sse"))
        .and(header("x-goog-api-key", "express-key"))
        .respond_with(sse_ok())
        .expect(1)
        .mount(&server)
        .await;

    // The static API-key resolver drives the Express route.
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
            Box::new(VertexProvider::express(Some(server.uri()))),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("express-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    drain_to_done(&models).await;
}

#[tokio::test]
async fn adc_authorized_user_end_to_end_bearer_and_quota_header() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("adc.json");
    std::fs::write(&creds, authorized_user_json()).unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("client_id=cid"))
        .and(body_string_contains("client_secret=csec"))
        .and(body_string_contains("refresh_token=rt1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"vtok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer vtok"))
            .and(header("x-goog-user-project", "quota-proj"))
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
                Some(format!("{}/token", server.uri())),
            ),
        },
    );

    // Source identity is captured on the public resolution path.
    let resolved = models
        .get_auth("vertex", Default::default())
        .await
        .expect("configured")
        .expect("resolves");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::Adc {
            source: AdcCredentialSource::AuthorizedUser {
                quota_project: Some("quota-proj".to_owned())
            }
        }
    );
    assert_eq!(resolved.secret.expose_secret(), "vtok");

    drain_to_done(&models).await;
}

#[tokio::test]
async fn adc_source_failure_fails_before_generation() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("bad.json");
    std::fs::write(&creds, r#"{"type":"authorized_user"}"#).unwrap();

    let token_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_string(r#"{"error":"invalid_request"}"#))
        .mount(&token_server)
        .await;
    let generation = MockServer::start().await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/token", token_server.uri())),
            ),
        },
    );
    let error = models
        .get_auth("vertex", Default::default())
        .await
        .expect_err("a selected source error fails the resolution");
    assert!(matches!(
        error,
        opi_ai::models::ModelsError::AuthResolution { .. }
    ));
    // Fail before generation: zero requests to the model endpoint.
    let requests = generation.received_requests().await.unwrap();
    assert!(requests.is_empty(), "no generation send may happen");
}

#[tokio::test]
async fn adc_service_account_loopback_exchange_shape_and_single_resolution() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("sa.json");
    std::fs::write(&creds, service_account_json()).unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"satok","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer satok"))
            .respond_with(sse_ok())
            .expect(2)
            .mount(&server)
            .await;

    let endpoint = format!("{}/token", server.uri());
    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(endpoint.clone()),
            ),
        },
    );

    drain_to_done(&models).await;
    drain_to_done(&models).await;

    // The JWT assertion: canonical grant type, iss/aud from the file and
    // the actually used endpoint, exp = iat + 3600.
    let requests = server.received_requests().await.unwrap();
    let token_post = requests
        .iter()
        .find(|r| r.url.path() == "/token")
        .expect("token exchange observed");
    let body = String::from_utf8(token_post.body.clone()).unwrap();
    let assertion = body
        .strip_prefix("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=")
        .expect("jwt-bearer form");
    let parts: Vec<&str> = assertion.split('.').collect();
    assert_eq!(parts.len(), 3);
    let header = b64url_decode(parts[0]);
    assert_eq!(header, b"{\"alg\":\"RS256\"}");
    let claims: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    assert_eq!(claims["iss"], "sa@proj.iam.gserviceaccount.com");
    assert_eq!(claims["aud"], endpoint);
    assert_eq!(
        claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
        3600
    );

    // One exchange serves both logical calls within the eager window.
    let token_posts = requests.iter().filter(|r| r.url.path() == "/token").count();
    assert_eq!(token_posts, 1);
}

#[tokio::test]
async fn token_503_then_success_retries_with_the_first_fixed_delay() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds.json");
    std::fs::write(&creds, authorized_user_json()).unwrap();

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"rt","expires_in":3600}"#),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .respond_with(sse_ok())
            .mount(&server)
            .await;

    let started = Instant::now();
    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(server.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/token", server.uri())),
            ),
        },
    );
    drain_to_done(&models).await;
    assert!(
        started.elapsed() >= std::time::Duration::from_millis(100),
        "the first retry waits the fixed 100ms gaxios delay"
    );
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests.iter().filter(|r| r.url.path() == "/token").count(),
        2
    );
}

#[tokio::test]
async fn token_redirect_is_terminal_and_generation_is_never_contacted() {
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds.json");
    std::fs::write(&creds, authorized_user_json()).unwrap();

    let token_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/elsewhere"))
        .mount(&token_server)
        .await;
    let generation = MockServer::start().await;

    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::CredentialsFile(creds),
                Arc::new(MapContext::default()),
                Some(format!("{}/token", token_server.uri())),
            ),
        },
    );
    assert!(
        models.get_auth("vertex", Default::default()).await.is_err(),
        "a redirect on a token exchange is terminal"
    );
    let requests = generation.received_requests().await.unwrap();
    assert!(requests.is_empty());
}

#[tokio::test]
async fn direct_metadata_selected_explicitly_streams_bearer() {
    let metadata = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/computeMetadata/v1/instance/service-accounts/default/token",
        ))
        .and(query_param(
            "scopes",
            "https://www.googleapis.com/auth/cloud-platform",
        ))
        .and(header("Metadata-Flavor", "Google"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"access_token":"mdtok","expires_in":3600}"#)
                .insert_header("Metadata-Flavor", "Google"),
        )
        .expect(1)
        .mount(&metadata)
        .await;

    let generation = MockServer::start().await;
    Mock::given(method("POST"))
            .and(path(
                "/v1/projects/p1/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent",
            ))
            .and(header("authorization", "Bearer mdtok"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&generation)
            .await;

    let host = metadata.uri().trim_start_matches("http://").to_owned();
    let models = vertex_models(
        VertexProvider::new("p1".into(), "us-central1".into(), Some(generation.uri())),
        AdcRouteResolver {
            owner: owner_for(
                AdcAuthSelection::DirectMetadata {
                    residency: Some(false),
                },
                Arc::new(MapContext::default().with_env("GCE_METADATA_HOST", &host)),
                None,
            ),
        },
    );
    let resolved = models
        .get_auth("vertex", Default::default())
        .await
        .expect("metadata resolves")
        .expect("configured");
    assert_eq!(
        resolved.provenance.source,
        AuthProvenanceSource::Adc {
            source: AdcCredentialSource::DirectMetadata
        }
    );
    drain_to_done(&models).await;
}

fn b64url_decode(input: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in input.as_bytes() {
        if byte == b'=' {
            continue;
        }
        let value = ALPHABET
            .iter()
            .position(|&a| a == byte)
            .expect("valid base64url byte") as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    out
}
