use opi_ai::auth::adc::cache::{AdcCacheKey, AdcTokenCache};
use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeInput, AdcNativeOwner, AdcResolutionError, AdcSourceHttpError,
    AdcSourceRequest, AdcSourceResponse, AdcSourceTransport, CLOUD_PLATFORM_SCOPE, FixedAdcClock,
};
use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError, ExternalAccountSupplier};
use secrecy::ExposeSecret;

use super::fixtures::SERVICE_ACCOUNT_PEM;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

const TOKEN_URL: &str = "https://source-exchange.test/token";
const IMP_URL: &str = "https://impersonation-exchange.test/v1/generateAccessToken";
/// The target principal this mod's files impersonate.
const TARGET: &str = "target@proj.iam.gserviceaccount.com";
/// Raw impersonation URL from the credentials file (its host/path is
/// never used as the wire destination: the family reconstructs the
/// iamcredentials endpoint from the universe domain).
const FILE_IMP_URL: &str = "https://iamcredentials.example.com/v1beta/projects/p/serviceAccounts/target@proj.iam.gserviceaccount.com:generateAccessToken";

struct MapContext {
    env: HashMap<String, String>,
}

impl MapContext {
    fn new(env: &[(&str, &str)]) -> Self {
        Self {
            env: env
                .iter()
                .copied()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }
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

struct ScriptedTransport {
    replies: Mutex<Vec<Result<AdcSourceResponse, AdcSourceHttpError>>>,
    requests: Mutex<Vec<AdcSourceRequest>>,
}

impl ScriptedTransport {
    fn new(replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>) -> Self {
        Self {
            replies: Mutex::new(replies),
            requests: Mutex::new(Vec::new()),
        }
    }
    fn text_reply(status: u16, body: &str) -> Result<AdcSourceResponse, AdcSourceHttpError> {
        Ok(AdcSourceResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        })
    }
    fn sent(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
    fn request(&self, index: usize) -> AdcSourceRequest {
        self.requests.lock().unwrap()[index].clone()
    }
}

impl AdcSourceTransport for ScriptedTransport {
    fn send<'a>(
        &'a self,
        request: AdcSourceRequest,
    ) -> opi_ai::auth::credential::BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>>
    {
        self.requests.lock().unwrap().push(request);
        let reply = self.replies.lock().unwrap().first().cloned();
        if reply.is_some() {
            let _ = self.replies.lock().unwrap().remove(0);
        }
        Box::pin(async move { reply.expect("scripted replies exhausted") })
    }
}

/// Transport whose sends block while armed, until released; proves
/// single-flight merging and late-publication fencing without real
/// waiting.
struct GatedTransport {
    gate: Arc<tokio::sync::Notify>,
    armed: std::sync::atomic::AtomicBool,
    inner: ScriptedTransport,
}

impl GatedTransport {
    fn new(
        replies: Vec<Result<AdcSourceResponse, AdcSourceHttpError>>,
    ) -> (Arc<Self>, Arc<tokio::sync::Notify>) {
        let gate = Arc::new(tokio::sync::Notify::new());
        (
            Arc::new(Self {
                gate: gate.clone(),
                armed: std::sync::atomic::AtomicBool::new(true),
                inner: ScriptedTransport::new(replies),
            }),
            gate,
        )
    }
    fn sent(&self) -> usize {
        self.inner.sent()
    }
    /// Disarm the gate and release every waiting send.
    fn release(&self) {
        self.armed.store(false, std::sync::atomic::Ordering::SeqCst);
        self.gate.notify_waiters();
    }
}

impl AdcSourceTransport for GatedTransport {
    fn send<'a>(
        &'a self,
        request: AdcSourceRequest,
    ) -> opi_ai::auth::credential::BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>>
    {
        let gate = self.gate.clone();
        let armed = self.armed.load(std::sync::atomic::Ordering::SeqCst);
        let inner = &self.inner;
        Box::pin(async move {
            if armed {
                gate.notified().await;
            }
            inner.send(request).await
        })
    }
}

fn clock(unix: i64) -> Arc<FixedAdcClock> {
    Arc::new(FixedAdcClock::new(
        OffsetDateTime::from_unix_timestamp(unix).unwrap(),
    ))
}

fn input<'a>(
    selection: AdcAuthSelection,
    ambient: &'a dyn AuthContext,
    transport: &'a dyn AdcSourceTransport,
    clock: &'a FixedAdcClock,
) -> AdcNativeInput<'a> {
    AdcNativeInput::new(selection, &SCOPED, ambient, transport, clock)
        .with_token_endpoint(Some(TOKEN_URL))
}

static SCOPED: std::sync::LazyLock<opi_ai::auth::adc::native::ScopedAdcEnv> =
    std::sync::LazyLock::new(opi_ai::auth::adc::native::ScopedAdcEnv::empty);

fn creds_file(dir: &std::path::Path, name: &str, contents: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn authorized_user_source() -> String {
    r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#
        .to_owned()
}

fn impersonated_json(extras: &str, source: &str) -> String {
    format!(
        r#"{{"type":"impersonated_service_account","source_credentials":{source},"service_account_impersonation_url":"{FILE_IMP_URL}"{extras}}}"#
    )
}

fn imp_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
    ScriptedTransport::text_reply(
        200,
        r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00Z"}"#,
    )
}

fn source_token(expires_in: i64) -> Result<AdcSourceResponse, AdcSourceHttpError> {
    ScriptedTransport::text_reply(
        200,
        &format!(r#"{{"access_token":"source-token","expires_in":{expires_in}}}"#),
    )
}

async fn resolve(
    json: &str,
    transport: &dyn AdcSourceTransport,
    clock: &FixedAdcClock,
) -> Result<Option<opi_ai::auth::adc::native::AdcResolvedToken>, AdcResolutionError> {
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(dir.path(), "impersonated.json", json);
    let ambient = MapContext::new(&[]);
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        transport,
        clock,
    );
    opi_ai::auth::adc::native::resolve_adc_native(&file_input).await
}

fn body_json(request: &AdcSourceRequest) -> serde_json::Value {
    serde_json::from_slice(request.body.as_deref().unwrap().as_bytes()).unwrap()
}

fn header<'a>(request: &'a AdcSourceRequest, name: &str) -> &'a str {
    request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
        .unwrap()
}

// ------------------------------------------------------------------
// Parsing
// ------------------------------------------------------------------

#[tokio::test]
async fn impersonated_missing_source_credentials_is_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let json = r#"{"type":"impersonated_service_account","service_account_impersonation_url":"https://iamcredentials.example.com/v1/projects/-/serviceAccounts/target@proj.iam.gserviceaccount.com:generateAccessToken"}"#;
    let error = resolve(json, &transport, &clock).await.unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::MissingField {
            family: "impersonated_service_account",
            field: "source_credentials"
        }
    ));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn impersonated_missing_impersonation_url_is_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let json = format!(
        r#"{{"type":"impersonated_service_account","source_credentials":{}}}"#,
        authorized_user_source()
    );
    let error = resolve(&json, &transport, &clock).await.unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::MissingField {
            family: "impersonated_service_account",
            field: "service_account_impersonation_url"
        }
    ));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn impersonated_target_principal_unextractable_is_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let bad = "https://iamcredentials.example.com/v1/not-a-token-request";
    let error = resolve(
        &impersonated_json("", &authorized_user_source()).replace(FILE_IMP_URL, bad),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationTargetInvalid
    ));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn impersonated_url_too_long_is_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let mut url = String::from("https://iamcredentials.example.com/v1/projects/-/serviceAccounts/");
    while url.len() <= 256 {
        url.push('a');
    }
    url.push_str(":generateAccessToken");
    let json = format!(
        r#"{{"type":"impersonated_service_account","source_credentials":{},"service_account_impersonation_url":"{url}"}}"#,
        authorized_user_source()
    );
    let error = resolve(&json, &transport, &clock).await.unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationTargetInvalid
    ));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn impersonated_malformed_delegates_and_lifetime_are_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let error = resolve(
        &impersonated_json(r#","delegates":"not-an-array""#, &authorized_user_source()),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationConfigInvalid { .. }
    ));

    let transport = ScriptedTransport::new(vec![]);
    let error = resolve(
        &impersonated_json(r#","lifetime":-5"#, &authorized_user_source()),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationConfigInvalid { .. }
    ));
    assert_eq!(transport.sent(), 0);
}

// ------------------------------------------------------------------
// Wire requests
// ------------------------------------------------------------------

#[tokio::test]
async fn impersonated_reconstructs_the_wire_request_from_the_universe() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
    let token = resolve(
        &impersonated_json(
            r#","delegates":["del@proj.iam.gserviceaccount.com"],"quota_project_id":"quota-proj""#,
            &authorized_user_source(),
        ),
        &transport,
        &clock,
    )
    .await
    .unwrap()
    .unwrap();

    // The file's raw impersonation URL never becomes the destination:
    // the wire request is reconstructed from the universe domain.
    let imp = transport.request(1);
    assert_eq!(
        imp.url,
        format!(
            "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )
    );
    assert_eq!(imp.method, "POST");
    assert_eq!(header(&imp, "authorization"), "Bearer source-token");
    assert_eq!(header(&imp, "content-type"), "application/json");
    let body = body_json(&imp);
    assert_eq!(
        body["delegates"],
        serde_json::json!(["del@proj.iam.gserviceaccount.com"])
    );
    assert_eq!(
        body["scope"],
        serde_json::json!(["https://www.googleapis.com/auth/cloud-platform"])
    );
    assert_eq!(body["lifetime"], serde_json::json!("3600s"));

    // The resolved token is the impersonated one; provenance keeps the
    // source and target identities separate and carries the quota
    // project from the impersonated file.
    assert_eq!(token.access_token.expose_secret(), "imp-token");
    assert_eq!(
        token.expires_at,
        Some(
            OffsetDateTime::parse(
                "2030-06-01T00:00:00Z",
                &time::format_description::well_known::Rfc3339
            )
            .unwrap()
        )
    );
    assert_eq!(token.quota_project.as_deref(), Some("quota-proj"));
    match &token.source {
        AdcCredentialSource::Impersonated {
            source,
            target_principal,
            delegates,
            quota_project,
        } => {
            assert!(matches!(
                source.as_ref(),
                AdcCredentialSource::AuthorizedUser { quota_project }
                if quota_project.is_none()
            ));
            assert_eq!(target_principal, TARGET);
            assert_eq!(*delegates, 1);
            assert_eq!(quota_project.as_deref(), Some("quota-proj"));
        }
        other => panic!("unexpected provenance: {other:?}"),
    }
}

#[tokio::test]
async fn impersonated_lifetime_and_scopes_come_from_the_file() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
    resolve(
        &impersonated_json(
            r#","lifetime":1200,"scopes":["scope-a","scope-b"]"#,
            &authorized_user_source(),
        ),
        &transport,
        &clock,
    )
    .await
    .unwrap()
    .unwrap();

    let body = body_json(&transport.request(1));
    assert_eq!(body["lifetime"], serde_json::json!("1200s"));
    assert_eq!(body["scope"], serde_json::json!(["scope-a", "scope-b"]));
}

#[tokio::test]
async fn impersonated_service_account_source_flows_identity() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
    let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
    let source = format!(
        r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}"}}"#
    );
    let token = resolve(&impersonated_json("", &source), &transport, &clock)
        .await
        .unwrap()
        .unwrap();

    // Source leg is the service-account JWT assertion exchange.
    let assertion = transport.request(0);
    let body = assertion.body.as_deref().unwrap();
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(body.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    assert!(pairs.iter().any(|(k, _)| {
        k == "grant_type"
            && pairs
                .iter()
                .any(|(k2, v2)| k2 == "assertion" && v2.contains('.'))
    }));

    // The impersonated target is the stable wire identity.
    assert_eq!(token.stable_identity(), Some(TARGET));
}

#[tokio::test]
async fn impersonated_recursive_nesting_has_no_new_depth_cap() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok(), imp_ok()]);
    let inner = impersonated_json("", &authorized_user_source());
    let outer = impersonated_json("", &inner);
    let token = resolve(&outer, &transport, &clock).await.unwrap().unwrap();

    // Three legs: source refresh → inner impersonation → outer
    // impersonation; the final token is the outer target's.
    assert_eq!(transport.sent(), 3);
    let AdcCredentialSource::Impersonated {
        source,
        target_principal,
        ..
    } = &token.source
    else {
        panic!("unexpected provenance: {:?}", token.source)
    };
    assert!(matches!(
        source.as_ref(),
        AdcCredentialSource::Impersonated {
            source: inner,
            ..
        } if matches!(inner.as_ref(), AdcCredentialSource::AuthorizedUser { .. })
    ));
    assert_eq!(target_principal, TARGET);
}

#[tokio::test]
async fn impersonated_universe_mismatch_is_typed_and_match_flows() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![]);
    let error = resolve(
        &impersonated_json(
            r#","universe_domain":"u.example""#,
            &authorized_user_source(),
        ),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationUniverseMismatch
    ));
    assert_eq!(transport.sent(), 0);

    // An explicit universe matching the source's flows into the
    // reconstructed wire endpoint.
    let matching_source = r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","universe_domain":"u.example"}"#;
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
    resolve(
        &impersonated_json(r#","universe_domain":"u.example""#, matching_source),
        &transport,
        &clock,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        transport.request(1).url,
        format!(
            "https://iamcredentials.u.example/v1/projects/-/serviceAccounts/{TARGET}:generateAccessToken"
        )
    );
}

#[tokio::test]
async fn impersonated_response_and_rejection_are_typed() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![
        source_token(3600),
        ScriptedTransport::text_reply(
            200,
            r#"{"accessToken":"imp-token","expireTime":"not-rfc3339"}"#,
        ),
    ]);
    let error = resolve(
        &impersonated_json("", &authorized_user_source()),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationResponseInvalid { .. }
    ));

    let transport = ScriptedTransport::new(vec![
        source_token(3600),
        ScriptedTransport::text_reply(200, r#"{}"#),
    ]);
    let error = resolve(
        &impersonated_json("", &authorized_user_source()),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationResponseInvalid { .. }
    ));

    let transport = ScriptedTransport::new(vec![
        source_token(3600),
        ScriptedTransport::text_reply(403, r#"{"error":"denied"}"#),
    ]);
    let error = resolve(
        &impersonated_json("", &authorized_user_source()),
        &transport,
        &clock,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExchangeRejected {
            status: 403,
            attempts: 1
        }
    ));
}

// ------------------------------------------------------------------
// external_account optional IAM impersonation
// ------------------------------------------------------------------

fn subject_file(dir: &std::path::Path) -> std::path::PathBuf {
    creds_file(dir, "subject.txt", "raw-subject-token")
}

fn external_with_impersonation(dir: &std::path::Path, extras: &str) -> String {
    let token_file = subject_file(dir);
    format!(
        r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","service_account_impersonation_url":"{FILE_IMP_URL}"{extras},"credential_source":{{"file":{}}}}}"#,
        serde_json::to_string(&token_file).unwrap()
    )
}

fn sts_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
    ScriptedTransport::text_reply(200, r#"{"access_token":"fed-token","expires_in":3600}"#)
}

#[tokio::test]
async fn external_impersonation_flows_after_sts_with_forced_scope() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![
        sts_ok(),
        ScriptedTransport::text_reply(
            200,
            r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00+00:00"}"#,
        ),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "external.json",
        &external_with_impersonation(
            dir.path(),
            r#","service_account_impersonation":{"token_lifetime_seconds":2700}"#,
        ),
    );
    let ambient = MapContext::new(&[]);
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
        &clock,
    )
    .with_impersonation_endpoint(Some(IMP_URL));
    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();

    // The STS leg runs with the forced cloud-platform scope.
    let sts = transport.request(0);
    let scope = url::form_urlencoded::parse(sts.body.as_deref().unwrap().as_bytes())
        .find(|(k, _)| k == "scope")
        .map(|(_, v)| v.into_owned())
        .unwrap();
    assert_eq!(scope, "https://www.googleapis.com/auth/cloud-platform");

    // The impersonation leg posts to the configured URL with the
    // federated token; no delegates field on the external path.
    let imp = transport.request(1);
    assert_eq!(imp.url, IMP_URL);
    assert_eq!(header(&imp, "authorization"), "Bearer fed-token");
    assert_eq!(header(&imp, "content-type"), "application/json");
    let body = body_json(&imp);
    assert_eq!(
        body["scope"],
        serde_json::json!(["https://www.googleapis.com/auth/cloud-platform"])
    );
    assert_eq!(body["lifetime"], serde_json::json!("2700s"));
    assert!(body.get("delegates").is_none());

    assert_eq!(token.access_token.expose_secret(), "imp-token");
    match &token.source {
        AdcCredentialSource::Impersonated {
            source,
            target_principal,
            delegates,
            quota_project,
        } => {
            assert!(matches!(
                source.as_ref(),
                AdcCredentialSource::ExternalAccount {
                    supplier: ExternalAccountSupplier::File,
                    ..
                }
            ));
            assert_eq!(target_principal, TARGET);
            assert_eq!(*delegates, 0);
            assert!(quota_project.is_none());
        }
        other => panic!("unexpected provenance: {other:?}"),
    }
}

#[tokio::test]
async fn external_impersonation_defaults_and_validates_the_url() {
    let clock = clock(1_700_000_000);
    let transport = ScriptedTransport::new(vec![sts_ok(), imp_ok()]);
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "external.json",
        &external_with_impersonation(dir.path(), ""),
    );
    let ambient = MapContext::new(&[]);
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
        &clock,
    )
    .with_impersonation_endpoint(Some(IMP_URL));
    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();
    let body = body_json(&transport.request(1));
    assert_eq!(body["lifetime"], serde_json::json!("3600s"));
    assert_eq!(token.stable_identity(), Some(TARGET));

    // A URL without an extractable principal fails typed at parse time.
    let bad = external_with_impersonation(dir.path(), "")
        .replace(FILE_IMP_URL, "https://iamcredentials.example.com/v1/nope");
    let path = creds_file(dir.path(), "external-bad.json", &bad);
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
        &clock,
    );
    let error = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ImpersonationTargetInvalid
    ));
}

// ------------------------------------------------------------------
// Keyed access cache
// ------------------------------------------------------------------

#[test]
fn cache_keys_discriminate_every_source_dimension() {
    let delegates = vec!["del@x".to_owned()];
    let scopes = vec![CLOUD_PLATFORM_SCOPE.to_owned()];
    let base = AdcCacheKey::from_parts(
        "config-digest",
        Some("principal@x"),
        &delegates,
        Some("//audience"),
        &scopes,
        Some("project"),
        Some("quota"),
        Some("endpoint"),
        Some("universe"),
        Some("tls-identity"),
    );
    let same = AdcCacheKey::from_parts(
        "config-digest",
        Some("principal@x"),
        &delegates,
        Some("//audience"),
        &scopes,
        Some("project"),
        Some("quota"),
        Some("endpoint"),
        Some("universe"),
        Some("tls-identity"),
    );
    assert_eq!(base, same);

    let variants = [
        AdcCacheKey::from_parts(
            "other-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("other@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &[],
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//other"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &["https://scope.example.test/other".to_owned()],
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("other-project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("other-quota"),
            Some("endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("other-endpoint"),
            Some("universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("other-universe"),
            Some("tls-identity"),
        ),
        AdcCacheKey::from_parts(
            "config-digest",
            Some("principal@x"),
            &delegates,
            Some("//audience"),
            &scopes,
            Some("project"),
            Some("quota"),
            Some("endpoint"),
            Some("universe"),
            Some("other-tls"),
        ),
    ];
    for variant in variants {
        assert_ne!(base, variant);
    }
}

#[tokio::test]
async fn owner_single_flight_merges_concurrent_resolutions() {
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "user.json",
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
    );
    let (transport, _gate) = GatedTransport::new(vec![source_token(3600)]);
    let owner = Arc::new(
        AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            clock(1_700_000_000),
        )
        .with_token_endpoint(Some(TOKEN_URL.to_owned())),
    );

    let mut tasks = Vec::new();
    for _ in 0..5 {
        let owner = owner.clone();
        tasks.push(tokio::spawn(async move { owner.resolve_token().await }));
    }
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    transport.release();
    for task in tasks {
        let token = task.await.unwrap().unwrap();
        assert_eq!(token.access_token.expose_secret(), "source-token");
    }
    // One acquisition served all five logical callers.
    assert_eq!(transport.sent(), 1);
}

#[tokio::test]
async fn owner_short_ttl_token_serves_the_current_call_without_looping() {
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "user.json",
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
    );
    // First acquisition returns a 200 s token (inside the 5-minute
    // eager window); the next acquisition fails through the budget; the
    // one after returns another short token.
    let transport = Arc::new(ScriptedTransport::new(vec![
        source_token(200),
        ScriptedTransport::text_reply(503, ""),
        ScriptedTransport::text_reply(503, ""),
        ScriptedTransport::text_reply(503, ""),
        ScriptedTransport::text_reply(503, ""),
        source_token(200),
    ]));
    let fixed = clock(1_700_000_000);
    let owner = AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(path),
        transport.clone(),
        fixed.clone(),
    )
    .with_token_endpoint(Some(TOKEN_URL.to_owned()));

    let first = owner.resolve_token().await.unwrap();
    // Eager-expired but still valid: one bounded refresh attempt fails;
    // the valid cached token serves the current call.
    let second = owner.resolve_token().await.unwrap();
    assert_eq!(
        second.access_token.expose_secret(),
        first.access_token.expose_secret()
    );
    // A successful refresh that itself returns a short token is served;
    // no loop refreshes again to exceed the eager window.
    let third = owner.resolve_token().await.unwrap();
    assert_eq!(third.access_token.expose_secret(), "source-token");
    // Bounded: exactly the scripted sends were used — one acquisition
    // attempt per logical call, never a loop past the eager window.
    assert_eq!(transport.sent(), 6);
}

#[tokio::test]
async fn owner_late_refresh_does_not_publish_after_invalidation() {
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "user.json",
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
    );
    let (transport, gate) = GatedTransport::new(vec![
        source_token(3600),
        ScriptedTransport::text_reply(200, r#"{"access_token":"late-token","expires_in":3600}"#),
        ScriptedTransport::text_reply(200, r#"{"access_token":"fresh-token","expires_in":3600}"#),
    ]);
    let fixed = clock(1_700_000_000);
    let owner = Arc::new(
        AdcNativeOwner::new(
            AdcAuthSelection::CredentialsFile(path),
            transport.clone(),
            fixed.clone(),
        )
        .with_token_endpoint(Some(TOKEN_URL.to_owned())),
    );

    let first = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    gate.notify_waiters();
    first.await.unwrap().unwrap();

    // Cross the eager window, then start a refresh whose send is gated.
    fixed.set(
        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap() + time::Duration::seconds(3600),
    );
    let refreshing = {
        let owner = owner.clone();
        tokio::spawn(async move { owner.resolve_token().await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // The source is replaced while the refresh is in flight.
    owner.invalidate();
    gate.notify_waiters();
    let late = refreshing.await.unwrap().unwrap();
    assert_eq!(late.access_token.expose_secret(), "late-token");

    // The late result was not published: the next resolve re-exchanges.
    transport.release();
    let next = owner.resolve_token().await.unwrap();
    assert_eq!(next.access_token.expose_secret(), "fresh-token");
    assert_eq!(transport.sent(), 3);
}

#[tokio::test]
async fn stable_identity_reports_the_wire_principal_only() {
    let clock_value = clock(1_700_000_000);
    // Impersonated: the target principal is stable wire evidence.
    let transport = ScriptedTransport::new(vec![source_token(3600), imp_ok()]);
    let token = resolve(
        &impersonated_json("", &authorized_user_source()),
        &transport,
        &clock_value,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(token.stable_identity(), Some(TARGET));

    // Plain authorized_user and external federation files carry no
    // stable account evidence in the file itself.
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(
        dir.path(),
        "user.json",
        r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#,
    );
    let ambient = MapContext::new(&[]);
    let transport = ScriptedTransport::new(vec![source_token(3600)]);
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
        &clock_value,
    );
    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.stable_identity(), None);

    let ambient = MapContext::new(&[]);
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    let path = creds_file(
        dir.path(),
        "external.json",
        &format!(
            r#"{{"type":"external_account","audience":"//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o","subject_token_type":"urn:ietf:params:oauth:token-type:jwt","credential_source":{{"file":{}}}}}"#,
            serde_json::to_string(&subject_file(dir.path())).unwrap()
        ),
    );
    let file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
        &clock_value,
    );
    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.stable_identity(), None);
}

#[tokio::test]
async fn cache_serves_fresh_tokens_and_single_flights_per_key() {
    // Direct cache-level proof of the reuse/fencing contract.
    let cache = AdcTokenCache::new();
    let key = AdcCacheKey::from_parts(
        "config",
        None,
        &[],
        None,
        &["scope".to_owned()],
        None,
        None,
        None,
        None,
        None,
    );
    let now = OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
    let exchanges = Arc::new(Mutex::new(0usize));
    let make = |token: &'static str, exchanges: Arc<Mutex<usize>>| {
        move || {
            let exchanges = exchanges.clone();
            async move {
                *exchanges.lock().unwrap() += 1;
                Ok(opi_ai::auth::adc::native::AdcResolvedToken {
                    access_token: secrecy::SecretString::from(token),
                    expires_at: Some(
                        OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()
                            + time::Duration::seconds(3600),
                    ),
                    quota_project: None,
                    rotated_refresh_token: None,
                    source: AdcCredentialSource::DirectMetadata,
                })
            }
        }
    };

    let first = cache
        .resolve(key, now, make("a", exchanges.clone()))
        .await
        .unwrap();
    assert_eq!(first.access_token.expose_secret(), "a");
    let second = cache
        .resolve(key, now, make("b", exchanges.clone()))
        .await
        .unwrap();
    assert_eq!(second.access_token.expose_secret(), "a");
    assert_eq!(*exchanges.lock().unwrap(), 1);

    // Invalidation fences the next publication.
    cache.invalidate(key);
    let third = cache
        .resolve(key, now, make("c", exchanges.clone()))
        .await
        .unwrap();
    assert_eq!(third.access_token.expose_secret(), "c");
    assert_eq!(*exchanges.lock().unwrap(), 2);
}
