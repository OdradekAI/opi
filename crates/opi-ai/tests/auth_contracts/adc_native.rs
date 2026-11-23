use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::BoxAuthFuture;
use opi_ai::auth::adc::metadata::resolve_direct_metadata_token;
use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeInput, AdcNativeOwner, AdcResolutionError, AdcSourceHttpError,
    AdcSourceRequest, AdcSourceResponse, AdcSourceTransport, CANONICAL_TOKEN_ENDPOINT,
    CLOUD_PLATFORM_SCOPE, DEFAULT_STS_OAUTHTOKEN_ENDPOINT, ScopedAdcEnv,
};
use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError};
use secrecy::ExposeSecret;

use super::fixtures::SERVICE_ACCOUNT_PEM;

fn service_account_json() -> String {
    let escaped = SERVICE_ACCOUNT_PEM.replace('\n', "\\n");
    format!(
        r#"{{"type":"service_account","client_email":"sa@proj.iam.gserviceaccount.com","private_key":"{escaped}","project_id":"proj"}}"#
    )
}

fn authorized_user_json(quota: Option<&str>) -> String {
    match quota {
            Some(q) => format!(
                r#"{{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1","quota_project_id":"{q}"}}"#
            ),
            None => r#"{"type":"authorized_user","client_id":"cid","client_secret":"csec","refresh_token":"rt1"}"#.to_owned(),
        }
}

fn external_authorized_user_json(token_url: Option<&str>, universe: Option<&str>) -> String {
    let mut fields = vec![
        r#""type":"external_account_authorized_user""#.to_owned(),
        r#""client_id":"ecid""#.to_owned(),
        r#""client_secret":"ecsec""#.to_owned(),
        r#""refresh_token":"ert1""#.to_owned(),
    ];
    if let Some(url) = token_url {
        fields.push(format!(r#""token_url":"{url}""#));
    }
    if let Some(u) = universe {
        fields.push(format!(r#""universe_domain":"{u}""#));
    }
    format!("{{{}}}", fields.join(","))
}

/// In-memory ambient source with counted reads.
#[derive(Default)]
struct MapContext {
    env: HashMap<String, String>,
    files: HashMap<String, bool>,
    env_reads: AtomicUsize,
    file_probes: AtomicUsize,
}

impl MapContext {
    fn with_env(mut self, name: &str, value: &str) -> Self {
        self.env.insert(name.to_owned(), value.to_owned());
        self
    }

    fn with_file(mut self, path: &std::path::Path) -> Self {
        self.files.insert(path.to_string_lossy().into_owned(), true);
        self
    }
}

impl AuthContext for MapContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.env_reads.fetch_add(1, Ordering::SeqCst);
        Ok(self.env.get(name).cloned())
    }
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        self.file_probes.fetch_add(1, Ordering::SeqCst);
        Ok(self.files.get(path).copied().unwrap_or(false))
    }
}

/// Scripted transport: replies per attempt and records every request.
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

    fn token_reply(body: &str) -> Result<AdcSourceResponse, AdcSourceHttpError> {
        Ok(AdcSourceResponse {
            status: 200,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        })
    }

    fn status_reply(status: u16) -> Result<AdcSourceResponse, AdcSourceHttpError> {
        Ok(AdcSourceResponse {
            status,
            headers: Vec::new(),
            body: Vec::new(),
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
    ) -> BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>> {
        self.requests.lock().unwrap().push(request);
        let reply = self.replies.lock().unwrap().first().cloned();
        if reply.is_some() {
            let _ = self.replies.lock().unwrap().remove(0);
        }
        Box::pin(async move { reply.expect("scripted replies exhausted") })
    }
}

fn fixed_clock(unix: i64) -> opi_ai::auth::adc::native::FixedAdcClock {
    opi_ai::auth::adc::native::FixedAdcClock::new(
        time::OffsetDateTime::from_unix_timestamp(unix).unwrap(),
    )
}

fn input<'a>(
    selection: AdcAuthSelection,
    scoped: &'a ScopedAdcEnv,
    ambient: &'a dyn AuthContext,
    transport: &'a dyn AdcSourceTransport,
    clock: &'a dyn opi_ai::auth::adc::native::AdcClock,
) -> AdcNativeInput<'a> {
    AdcNativeInput {
        selection,
        scoped,
        ambient,
        transport,
        clock,
        token_endpoint: None,
        universe_domain: None,
        scopes: &[],
        certificate_extra_roots_pem: None,
        impersonation_endpoint: None,
    }
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// -- selection / file semantics ------------------------------------------

#[tokio::test]
async fn adc_auto_without_any_file_is_not_configured_and_sends_nothing() {
    let ctx = MapContext::default();
    let transport = ScriptedTransport::new(Vec::new());
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::Auto,
        &scoped,
        &ctx,
        &transport,
        &clock,
    ))
    .await
    .unwrap();
    assert!(resolved.is_none(), "no file anywhere: not configured");
    assert_eq!(transport.sent(), 0, "missing credentials never dial out");
}

#[tokio::test]
async fn adc_explicit_env_file_parse_failure_is_terminal() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, "{not json").unwrap();
    let ctx =
        MapContext::default().with_env("GOOGLE_APPLICATION_CREDENTIALS", path.to_str().unwrap());
    let transport = ScriptedTransport::new(Vec::new());
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::Auto,
        &scoped,
        &ctx,
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(err, AdcResolutionError::MalformedJson { .. }));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn adc_well_known_platform_path_is_used_when_env_is_unset() {
    let dir = temp_dir();
    let gcloud = if cfg!(windows) {
        dir.path().join("gcloud")
    } else {
        dir.path().join(".config").join("gcloud")
    };
    std::fs::create_dir_all(&gcloud).unwrap();
    let file = gcloud.join("application_default_credentials.json");
    std::fs::write(&file, authorized_user_json(None)).unwrap();
    let ctx = MapContext::default()
        .with_env(
            if cfg!(windows) { "APPDATA" } else { "HOME" },
            dir.path().to_str().unwrap(),
        )
        .with_file(&file);
    let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"tok1","expires_in":3600}"#,
    )]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::Auto,
        &scoped,
        &ctx,
        &transport,
        &clock,
    ))
    .await
    .unwrap()
    .expect("well-known file resolves");
    assert!(matches!(
        resolved.source,
        AdcCredentialSource::AuthorizedUser {
            quota_project: None
        }
    ));
    let request = transport.request(0);
    assert_eq!(request.url, CANONICAL_TOKEN_ENDPOINT);
    assert!(
        request
            .body
            .as_deref()
            .unwrap()
            .contains("grant_type=refresh_token")
    );
    assert!(
        request
            .body
            .as_deref()
            .unwrap()
            .contains("refresh_token=rt1")
    );
    assert_eq!(
        resolved.expires_at,
        Some(time::OffsetDateTime::from_unix_timestamp(1_003_600).unwrap())
    );
}

#[tokio::test]
async fn adc_scoped_env_file_shadows_well_known() {
    let dir = temp_dir();
    let explicit = dir.path().join("explicit.json");
    std::fs::write(&explicit, service_account_json()).unwrap();
    let gcloud = dir.path().join(".config").join("gcloud");
    std::fs::create_dir_all(&gcloud).unwrap();
    std::fs::write(
        gcloud.join("application_default_credentials.json"),
        authorized_user_json(None),
    )
    .unwrap();
    let ctx = MapContext::default()
        .with_env("HOME", dir.path().to_str().unwrap())
        .with_env("GOOGLE_APPLICATION_CREDENTIALS", explicit.to_str().unwrap());
    let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"sa-tok","expires_in":3600}"#,
    )]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::Auto,
        &scoped,
        &ctx,
        &transport,
        &clock,
    ))
    .await
    .unwrap()
    .expect("explicit env file wins");
    assert!(matches!(
        resolved.source,
        AdcCredentialSource::ServiceAccount
    ));
}

#[tokio::test]
async fn adc_unknown_and_future_families_fail_typed() {
    let dir = temp_dir();
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    for (contents, expect_admitted) in [
        // `external_account` is admitted since task 20.21.2; its
        // missing-field failures are typed family errors.
        (r#"{"type":"external_account"}"#.to_owned(), false),
        (r#"{"type":"wat"}"#.to_owned(), false),
    ] {
        let path = dir.path().join("creds.json");
        std::fs::write(&path, &contents).unwrap();
        let transport = ScriptedTransport::new(Vec::new());
        let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
            AdcAuthSelection::CredentialsFile(path),
            &scoped,
            &MapContext::default(),
            &transport,
            &clock,
        ))
        .await
        .unwrap_err();
        assert!(
            matches!(
                err,
                AdcResolutionError::FamilyNotAdmitted { .. }
                    | AdcResolutionError::UnknownType { .. }
                    | AdcResolutionError::MissingField {
                        family: "external_account",
                        ..
                    }
            ),
            "family dispatch must be typed, got {err:?}"
        );
        assert_eq!(transport.sent(), 0);
        assert!(!expect_admitted);
    }
}

#[tokio::test]
async fn adc_missing_required_field_names_the_field() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(
        &path,
        r#"{"type":"authorized_user","client_id":"cid","refresh_token":"rt"}"#,
    )
    .unwrap();
    let transport = ScriptedTransport::new(Vec::new());
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        AdcResolutionError::MissingField { family, field, .. }
            if family == "authorized_user" && field == "client_secret"
    ));
    assert_eq!(transport.sent(), 0);
}

// -- family exchanges -----------------------------------------------------

#[tokio::test]
async fn adc_service_account_jws_shape_and_canonical_exchange() {
    let dir = temp_dir();
    let path = dir.path().join("sa.json");
    std::fs::write(&path, service_account_json()).unwrap();
    let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"sa-tok","expires_in":3600}"#,
    )]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap()
    .expect("service account resolves");
    assert!(matches!(
        resolved.source,
        AdcCredentialSource::ServiceAccount
    ));

    let request = transport.request(0);
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, CANONICAL_TOKEN_ENDPOINT);
    let body = request.body.unwrap();
    assert!(
        body.starts_with(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion="
        ),
        "form is grant_type then assertion, got {body}"
    );
    let assertion = body.trim_start_matches(
        "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=",
    );
    let parts: Vec<&str> = assertion.split('.').collect();
    assert_eq!(parts.len(), 3, "JWS compact form");

    // Header is alg-only (no typ/kid).
    let header = b64url_decode(parts[0]);
    assert_eq!(header, b"{\"alg\":\"RS256\"}");

    // Claims carry the source-pinned shape with iat/exp exactly 3600 apart.
    let claims: serde_json::Value = serde_json::from_slice(&b64url_decode(parts[1])).unwrap();
    assert_eq!(claims["iss"], "sa@proj.iam.gserviceaccount.com");
    assert_eq!(claims["scope"], CLOUD_PLATFORM_SCOPE);
    assert_eq!(claims["aud"], CANONICAL_TOKEN_ENDPOINT);
    assert_eq!(claims["iat"], 1_000_000);
    assert_eq!(claims["exp"], 1_003_600);
    assert_eq!(
        resolved.expires_at,
        Some(time::OffsetDateTime::from_unix_timestamp(1_003_600).unwrap())
    );
}

#[tokio::test]
async fn adc_rejected_private_key_is_a_typed_failure_with_zero_sends() {
    let dir = temp_dir();
    let path = dir.path().join("badkey.json");
    std::fs::write(
        &path,
        r#"{"type":"service_account","client_email":"sa@x","private_key":"not a pem"}"#,
    )
    .unwrap();
    let transport = ScriptedTransport::new(Vec::new());
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(err, AdcResolutionError::KeyRejected));
    assert_eq!(transport.sent(), 0);
}

#[tokio::test]
async fn adc_external_authorized_user_uses_basic_auth_and_default_sts_url() {
    let dir = temp_dir();
    let path = dir.path().join("eau.json");
    std::fs::write(&path, external_authorized_user_json(None, None)).unwrap();
    let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"ext-tok","expires_in":1800,"refresh_token":"ert2"}"#,
    )]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap()
    .expect("external authorized user resolves");
    assert!(matches!(
        resolved.source,
        AdcCredentialSource::ExternalAccountAuthorizedUser { rotated: true, .. }
    ));
    assert_eq!(
        resolved
            .rotated_refresh_token
            .as_ref()
            .map(|s| s.expose_secret()),
        Some("ert2")
    );

    let request = transport.request(0);
    assert_eq!(request.url, DEFAULT_STS_OAUTHTOKEN_ENDPOINT);
    let basic = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .map(|(_, value)| value.clone())
        .expect("basic auth header present");
    assert_eq!(basic, format!("Basic {}", b64_std(b"ecid:ecsec")));
    let body = request.body.unwrap();
    assert!(body.contains("grant_type=refresh_token"));
    assert!(body.contains("refresh_token=ert1"));
}

#[tokio::test]
async fn adc_external_authorized_user_token_url_and_universe_domain() {
    let dir = temp_dir();
    let path = dir.path().join("eau-url.json");
    std::fs::write(
        &path,
        external_authorized_user_json(Some("https://sts.example.invalid/v1/oauthtoken"), None),
    )
    .unwrap();
    let transport = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"t","expires_in":100}"#,
    )]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap();
    assert!(resolved.is_some());
    assert_eq!(
        transport.request(0).url,
        "https://sts.example.invalid/v1/oauthtoken"
    );

    let path2 = dir.path().join("eau-universe.json");
    std::fs::write(
        &path2,
        external_authorized_user_json(None, Some("example.net")),
    )
    .unwrap();
    let transport2 = ScriptedTransport::new(vec![ScriptedTransport::token_reply(
        r#"{"access_token":"t","expires_in":100}"#,
    )]);
    let resolved2 = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path2),
        &ScopedAdcEnv::default(),
        &MapContext::default(),
        &transport2,
        &clock,
    ))
    .await
    .unwrap();
    assert!(resolved2.is_some());
    assert_eq!(
        transport2.request(0).url,
        "https://sts.example.net/v1/oauthtoken"
    );
}

#[tokio::test]
async fn owner_reuses_fresh_tokens_and_refreshes_after_the_eager_window() {
    use std::sync::Arc;
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, authorized_user_json(None)).unwrap();
    let transport = Arc::new(ScriptedTransport::new(vec![
        ScriptedTransport::token_reply(r#"{"access_token":"tok1","expires_in":3600}"#),
        ScriptedTransport::token_reply(r#"{"access_token":"tok2","expires_in":3600}"#),
    ]));
    let clock = Arc::new(fixed_clock(1_000_000));
    let owner = AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(path),
        transport.clone(),
        clock.clone(),
    )
    .with_ambient(Arc::new(MapContext::default()));

    let first = owner.resolve_token().await.unwrap();
    assert_eq!(first.access_token.expose_secret(), "tok1");
    let second = owner.resolve_token().await.unwrap();
    assert_eq!(
        second.access_token.expose_secret(),
        "tok1",
        "fresh token reused"
    );

    // The eager threshold is five minutes: once now + 300s reaches the
    // expiry the owner re-exchanges.
    clock.set(time::OffsetDateTime::from_unix_timestamp(1_003_300).unwrap());
    let third = owner.resolve_token().await.unwrap();
    assert_eq!(third.access_token.expose_secret(), "tok2");
}

#[tokio::test]
async fn owner_rotation_replaces_the_refresh_token_in_place() {
    use std::sync::Arc;
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, external_authorized_user_json(None, None)).unwrap();
    let transport = Arc::new(ScriptedTransport::new(vec![
        ScriptedTransport::token_reply(
            r#"{"access_token":"t1","expires_in":60,"refresh_token":"ert2"}"#,
        ),
        ScriptedTransport::token_reply(r#"{"access_token":"t2","expires_in":60}"#),
    ]));
    let clock = Arc::new(fixed_clock(1_000_000));
    let owner = AdcNativeOwner::new(
        AdcAuthSelection::CredentialsFile(path),
        transport.clone(),
        clock.clone(),
    )
    .with_ambient(Arc::new(MapContext::default()));

    let first = owner.resolve_token().await.unwrap();
    assert_eq!(
        first
            .rotated_refresh_token
            .as_ref()
            .map(|s| s.expose_secret()),
        Some("ert2")
    );
    clock.set(time::OffsetDateTime::from_unix_timestamp(1_000_120).unwrap());
    let second = owner.resolve_token().await.unwrap();
    let body = transport.request(1).body.unwrap();
    assert!(
        body.contains("refresh_token=ert2"),
        "the second exchange sends the rotated refresh token, got {body}"
    );
    assert!(second.rotated_refresh_token.is_none(), "no new rotation");
}

// -- retry budgets ---------------------------------------------------------

#[tokio::test]
async fn adc_status_retry_is_three_extra_with_gaxios_ranges() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, authorized_user_json(None)).unwrap();
    let transport = ScriptedTransport::new(vec![
        ScriptedTransport::status_reply(503),
        ScriptedTransport::status_reply(503),
        ScriptedTransport::token_reply(r#"{"access_token":"ok","expires_in":3600}"#),
    ]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let resolved = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap()
    .expect("503 x2 then 200 succeeds");
    assert!(resolved.access_token.expose_secret() == "ok");
    assert_eq!(transport.sent(), 3);
}

#[tokio::test]
async fn adc_status_exhaustion_and_non_retryable_statuses() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, authorized_user_json(None)).unwrap();
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();

    // 4 sends exhausts the fixed extra-3 budget.
    let transport = ScriptedTransport::new(vec![
        ScriptedTransport::status_reply(500),
        ScriptedTransport::status_reply(500),
        ScriptedTransport::status_reply(500),
        ScriptedTransport::status_reply(500),
    ]);
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path.clone()),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        AdcResolutionError::ExchangeRejected {
            status: 500,
            attempts: 4
        }
    ));
    assert_eq!(transport.sent(), 4);

    // 400 is outside the fixed ranges: terminal on the first send.
    let transport = ScriptedTransport::new(vec![ScriptedTransport::status_reply(400)]);
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        AdcResolutionError::ExchangeRejected {
            status: 400,
            attempts: 1
        }
    ));
    assert_eq!(transport.sent(), 1);
}

#[tokio::test]
async fn adc_no_response_budget_is_two_extra() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, authorized_user_json(None)).unwrap();
    let transport = ScriptedTransport::new(vec![
        Err(AdcSourceHttpError::Network),
        Err(AdcSourceHttpError::Network),
        Err(AdcSourceHttpError::Network),
    ]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        AdcResolutionError::ExchangeNetwork { attempts: 3 }
    ));
}

#[tokio::test]
async fn adc_redirect_on_exchange_is_terminal() {
    let dir = temp_dir();
    let path = dir.path().join("creds.json");
    std::fs::write(&path, authorized_user_json(None)).unwrap();
    let transport = ScriptedTransport::new(vec![Err(AdcSourceHttpError::Redirect)]);
    let clock = fixed_clock(1_000_000);
    let scoped = ScopedAdcEnv::default();
    let err = opi_ai::auth::adc::native::resolve_adc_native(&input(
        AdcAuthSelection::CredentialsFile(path),
        &scoped,
        &MapContext::default(),
        &transport,
        &clock,
    ))
    .await
    .unwrap_err();
    assert!(matches!(err, AdcResolutionError::Redirected));
    assert_eq!(transport.sent(), 1);
}

// -- direct metadata -------------------------------------------------------

#[tokio::test]
async fn metadata_token_carries_flavor_scope_and_expiry() {
    let ctx = MapContext::default();
    let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
        status: 200,
        headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
        body: br#"{"access_token":"md-tok","expires_in":1200}"#.to_vec(),
    })]);
    let clock = fixed_clock(1_000_000);
    let token = resolve_direct_metadata_token(
        &ctx,
        &transport,
        &clock,
        &[CLOUD_PLATFORM_SCOPE.to_owned()],
        Some(false),
    )
    .await
    .unwrap();
    assert!(matches!(token.source, AdcCredentialSource::DirectMetadata));
    assert_eq!(token.access_token.expose_secret(), "md-tok");
    assert_eq!(
        token.expires_at,
        Some(time::OffsetDateTime::from_unix_timestamp(1_001_200).unwrap())
    );

    let request = transport.request(0);
    assert_eq!(request.method, "GET");
    assert_eq!(
        request.url,
        "http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/token?scopes=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcloud-platform"
    );
    assert!(
        request
            .headers
            .iter()
            .any(|(name, value)| name == "Metadata-Flavor" && value == "Google")
    );
    assert_eq!(
        request.timeout,
        Some(std::time::Duration::from_millis(3000)),
        "non-resident metadata uses the fixed 3000ms"
    );
}

#[tokio::test]
async fn metadata_residency_true_disables_the_timeout() {
    let ctx = MapContext::default();
    let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
        status: 200,
        headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
        body: br#"{"access_token":"md","expires_in":60}"#.to_vec(),
    })]);
    let clock = fixed_clock(1_000_000);
    resolve_direct_metadata_token(
        &ctx,
        &transport,
        &clock,
        &[CLOUD_PLATFORM_SCOPE.to_owned()],
        Some(true),
    )
    .await
    .unwrap();
    assert_eq!(transport.request(0).timeout, None);
}

#[tokio::test]
async fn metadata_wrong_flavor_is_typed_and_timeout_is_single_send() {
    let ctx = MapContext::default();
    let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
        status: 200,
        headers: Vec::new(),
        body: br#"{"access_token":"x"}"#.to_vec(),
    })]);
    let clock = fixed_clock(1_000_000);
    let err = resolve_direct_metadata_token(
        &ctx,
        &transport,
        &clock,
        &[CLOUD_PLATFORM_SCOPE.to_owned()],
        Some(false),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AdcResolutionError::MetadataFlavor));

    let transport = ScriptedTransport::new(vec![Err(AdcSourceHttpError::Timeout)]);
    let err = resolve_direct_metadata_token(
        &ctx,
        &transport,
        &clock,
        &[CLOUD_PLATFORM_SCOPE.to_owned()],
        Some(false),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        AdcResolutionError::ExchangeTimeout { attempts: 1 }
    ));
    assert_eq!(transport.sent(), 1);
}

#[tokio::test]
async fn metadata_base_honors_scoped_host_env() {
    let ctx = MapContext::default().with_env("GCE_METADATA_HOST", "127.0.0.1:9999");
    let transport = ScriptedTransport::new(vec![Ok(AdcSourceResponse {
        status: 200,
        headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
        body: br#"{"access_token":"md","expires_in":60}"#.to_vec(),
    })]);
    let clock = fixed_clock(1_000_000);
    resolve_direct_metadata_token(
        &ctx,
        &transport,
        &clock,
        &[CLOUD_PLATFORM_SCOPE.to_owned()],
        Some(false),
    )
    .await
    .unwrap();
    assert_eq!(
        transport.request(0).url,
        "http://127.0.0.1:9999/computeMetadata/v1/instance/service-accounts/default/token?scopes=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcloud-platform"
    );
}

// -- helpers ---------------------------------------------------------------

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

fn b64_std(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}
