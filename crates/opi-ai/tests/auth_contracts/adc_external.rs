use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeInput, AdcResolutionError, AdcSourceHttpError, AdcSourceRequest,
    AdcSourceResponse, AdcSourceTransport, ScopedAdcEnv,
};
use opi_ai::auth::credential::BoxAuthFuture;
use opi_ai::auth::{AdcCredentialSource, AuthContext, AuthContextError, ExternalAccountSupplier};
use secrecy::ExposeSecret;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

// ------------------------------------------------------------------
// Shared local helpers
// ------------------------------------------------------------------

struct MapContext {
    env: HashMap<String, String>,
    process_dir: tempfile::TempDir,
}

impl MapContext {
    fn new(env: &[(&str, &str)]) -> Self {
        Self {
            process_dir: tempfile::tempdir().unwrap(),
            env: env
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        }
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
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(Path::new(path).is_file())
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
    ) -> BoxAuthFuture<'a, Result<AdcSourceResponse, AdcSourceHttpError>> {
        self.requests.lock().unwrap().push(request);
        let reply = self.replies.lock().unwrap().first().cloned();
        if reply.is_some() {
            let _ = self.replies.lock().unwrap().remove(0);
        }
        Box::pin(async move { reply.expect("scripted replies exhausted") })
    }
}

fn clock() -> opi_ai::auth::adc::native::FixedAdcClock {
    opi_ai::auth::adc::native::FixedAdcClock::new(
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap(),
    )
}

/// A leaked frozen clock so `AdcNativeInput` helpers can borrow
/// `&'static dyn AdcClock`.
fn leaked_clock() -> &'static opi_ai::auth::adc::native::FixedAdcClock {
    static FROZEN: std::sync::LazyLock<opi_ai::auth::adc::native::FixedAdcClock> =
        std::sync::LazyLock::new(clock);
    &FROZEN
}

fn input<'a>(
    selection: AdcAuthSelection,
    ambient: &'a dyn AuthContext,
    transport: &'a dyn AdcSourceTransport,
) -> AdcNativeInput<'a> {
    static EMPTY_SCOPED: std::sync::LazyLock<ScopedAdcEnv> =
        std::sync::LazyLock::new(ScopedAdcEnv::empty);
    AdcNativeInput::new(selection, &EMPTY_SCOPED, ambient, transport, leaked_clock())
}

fn creds_file(dir: &std::path::Path, name: &str, contents: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn sts_ok() -> Result<AdcSourceResponse, AdcSourceHttpError> {
    ScriptedTransport::text_reply(200, r#"{"access_token":"fed-token","expires_in":3600}"#)
}

fn form_pairs(body: &str) -> Vec<(String, String)> {
    url::form_urlencoded::parse(body.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn form_value(body: &str, key: &str) -> String {
    form_pairs(body)
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// JS `decodeURIComponent` subset (percent + nothing else) for decoding
/// serialized AWS subject tokens in assertions.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap();
            out.push(u8::from_str_radix(hex, 16).unwrap());
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

fn audience() -> &'static str {
    "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o"
}

fn subject_token_type() -> &'static str {
    "urn:ietf:params:oauth:token-type:jwt"
}

fn external_json(credential_source: &str) -> String {
    format!(
        r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}","credential_source":{}}}"#,
        audience(),
        subject_token_type(),
        credential_source
    )
}

/// Full file JSON with extra top-level fields (`universe_domain`,
/// `token_url`, `workforce_pool_user_project`, ...).
fn external_full_json(extras: &str, credential_source: &str) -> String {
    format!(
        r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}"{extras},"credential_source":{credential_source}}}"#,
        audience(),
        subject_token_type(),
    )
}

async fn resolve(
    json: &str,
    ambient: &dyn AuthContext,
    transport: &dyn AdcSourceTransport,
) -> Result<Option<opi_ai::auth::adc::native::AdcResolvedToken>, AdcResolutionError> {
    let dir = tempfile::tempdir().unwrap();
    let path = creds_file(dir.path(), "external.json", json);
    let file_input = input(AdcAuthSelection::CredentialsFile(path), ambient, transport);
    opi_ai::auth::adc::native::resolve_adc_native(&file_input).await
}

// ------------------------------------------------------------------
// Configuration and identity-pool parsing
// ------------------------------------------------------------------

#[tokio::test]
async fn external_file_supplier_text_exchanges_at_sts() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "raw-subject-token");
    let json = external_json(&format!(
        r#"{{"file":{}}}"#,
        serde_json::to_string(&token_file).unwrap()
    ));
    let ambient = MapContext::new(&[]);
    let transport = ScriptedTransport::new(vec![sts_ok()]);

    let token = resolve(&json, &ambient, &transport).await.unwrap().unwrap();

    let sts = transport.request(0);
    assert_eq!(sts.method, "POST");
    assert_eq!(sts.url, "https://sts.googleapis.com/v1/token");
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "grant_type"),
        "urn:ietf:params:oauth:grant-type:token-exchange"
    );
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "requested_token_type"),
        "urn:ietf:params:oauth:token-type:access_token"
    );
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "audience"),
        audience()
    );
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "subject_token"),
        "raw-subject-token"
    );
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "subject_token_type"),
        subject_token_type()
    );
    assert_eq!(
        form_value(sts.body.as_deref().unwrap(), "scope"),
        "https://www.googleapis.com/auth/cloud-platform"
    );
    assert_eq!(token.access_token.expose_secret(), "fed-token");
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::File,
            workforce_user_project: None,
        }
    );
}

#[tokio::test]
async fn external_file_supplier_json_field_and_raw_text_no_trim() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(
        dir.path(),
        "subject.json",
        r#"{"unrelated":"x","the_token":"  spaced  "}"#,
    );
    let json = external_json(&format!(
        r#"{{"file":{},"format":{{"type":"json","subject_token_field_name":"the_token"}}}}"#,
        serde_json::to_string(&token_file).unwrap()
    ));
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    let token = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    // The file supplier keeps the raw text (no trim) for text format;
    // JSON format extracts the named field exactly.
    assert_eq!(
        form_value(
            transport.request(0).body.as_deref().unwrap(),
            "subject_token"
        ),
        "  spaced  "
    );
    assert_eq!(token.access_token.expose_secret(), "fed-token");
}

#[tokio::test]
async fn external_file_text_keeps_raw_bytes_without_trim() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "\n  padded-token \n");
    let json = external_json(&format!(
        r#"{{"file":{}}}"#,
        serde_json::to_string(&token_file).unwrap()
    ));
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        form_value(
            transport.request(0).body.as_deref().unwrap(),
            "subject_token"
        ),
        "\n  padded-token \n"
    );
}

#[tokio::test]
async fn external_universe_domain_and_explicit_token_url() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = external_full_json(
        r#","universe_domain":"example.net""#,
        &format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&token_file).unwrap()
        ),
    );
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(transport.request(0).url, "https://sts.example.net/v1/token");

    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = external_full_json(
        r#","token_url":"https://sts.example.org/exchange""#,
        &format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&token_file).unwrap()
        ),
    );
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(transport.request(0).url, "https://sts.example.org/exchange");
}

#[tokio::test]
async fn external_confidential_client_uses_basic() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = format!(
        r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}","client_id":"ecid","client_secret":"ecsec","credential_source":{{"file":{}}}}}"#,
        audience(),
        subject_token_type(),
        serde_json::to_string(&token_file).unwrap()
    );
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    use base64::Engine as _;
    let expected = base64::engine::general_purpose::STANDARD.encode(b"ecid:ecsec");
    assert!(
        transport
            .request(0)
            .headers
            .iter()
            .any(|(k, v)| k == "authorization" && v == &format!("Basic {expected}"))
    );
    // Confidential clients pass no userProject options field.
    assert!(
        !transport
            .request(0)
            .body
            .as_deref()
            .unwrap()
            .contains("options=")
    );
}

#[tokio::test]
async fn external_workforce_user_project_flows_into_options_field() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = format!(
        r#"{{"type":"external_account","audience":"//iam.googleapis.com/locations/us/workforcePools/wp/providers/oidc","subject_token_type":"{}","workforce_pool_user_project":"wp-proj","credential_source":{{"file":{}}}}}"#,
        subject_token_type(),
        serde_json::to_string(&token_file).unwrap()
    );
    let transport = ScriptedTransport::new(vec![sts_ok()]);
    let token = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        form_value(transport.request(0).body.as_deref().unwrap(), "options"),
        r#"{"userProject":"wp-proj"}"#
    );
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::File,
            workforce_user_project: Some("wp-proj".to_owned()),
        }
    );
}

#[tokio::test]
async fn external_workforce_project_on_non_workforce_audience_is_typed() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = external_full_json(
        r#","workforce_pool_user_project":"wp""#,
        &format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&token_file).unwrap()
        ),
    );
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AdcResolutionError::WorkforceProjectInvalid));
}

#[tokio::test]
async fn external_missing_audience_is_missing_field() {
    let json = r#"{"type":"external_account","subject_token_type":"st","credential_source":{"file":"/nope"}}"#;
    let error = resolve(
        json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::MissingField {
            family: "external_account",
            field: "audience"
        }
    ));
}

#[tokio::test]
async fn external_token_url_invalid_is_typed() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = external_full_json(
        r#","token_url":"not a url""#,
        &format!(
            r#"{{"file":{}}}"#,
            serde_json::to_string(&token_file).unwrap()
        ),
    );
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AdcResolutionError::InvalidTokenUrl));
}

#[tokio::test]
async fn external_identity_pool_source_exclusivity_is_typed() {
    let json = external_json(r#"{"file":"/a","url":"https://x.example/t"}"#);
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));

    let json = external_json("{}");
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));
}

#[tokio::test]
async fn external_json_format_requires_field_name() {
    let json = external_json(r#"{"file":"/a","format":{"type":"json"}}"#);
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));

    let json = external_json(r#"{"file":"/a","format":{"type":"cbor"}}"#);
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));
}

#[tokio::test]
async fn external_sts_rejection_and_retry_budget() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = creds_file(dir.path(), "subject.txt", "t");
    let json = external_json(&format!(
        r#"{{"file":{}}}"#,
        serde_json::to_string(&token_file).unwrap()
    ));

    // Non-retryable status is terminal after one send.
    let transport =
        ScriptedTransport::new(vec![ScriptedTransport::text_reply(400, "nope"), sts_ok()]);
    let error = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExchangeRejected {
            status: 400,
            attempts: 1
        }
    ));
    assert_eq!(transport.sent(), 1);

    // Retry-eligible status retries under the fixed Gaxios budget.
    let transport = ScriptedTransport::new(vec![ScriptedTransport::text_reply(503, ""), sts_ok()]);
    let token = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.access_token.expose_secret(), "fed-token");
    assert_eq!(transport.sent(), 2);
}

// The `impersonated_service_account` family gate test from 20.21.2 was
// superseded by task 20.21.3: the family is admitted and owned by the
// `adc_impersonation` module (19 tests: parsing, wire reconstruction,
// recursion, universe agreement, external composition, keyed cache).

// ------------------------------------------------------------------
// URL supplier
// ------------------------------------------------------------------

#[tokio::test]
async fn external_url_supplier_get_with_headers_then_sts() {
    let json = external_json(
        r#"{"url":"https://subject.example/token","headers":{"X-Custom":"yes"},"format":{"type":"json","subject_token_field_name":"tok"}}"#,
    );
    let transport = ScriptedTransport::new(vec![
        ScriptedTransport::text_reply(200, r#"{"tok":"url-subject"}"#),
        sts_ok(),
    ]);
    let token = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();

    let get = transport.request(0);
    assert_eq!(get.method, "GET");
    assert_eq!(get.url, "https://subject.example/token");
    assert!(
        get.headers
            .iter()
            .any(|(k, v)| k == "x-custom" && v == "yes")
    );
    assert_eq!(
        form_value(
            transport.request(1).body.as_deref().unwrap(),
            "subject_token"
        ),
        "url-subject"
    );
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::Url,
            workforce_user_project: None,
        }
    );
}

#[tokio::test]
async fn external_url_supplier_terminal_status_is_typed_with_zero_sts() {
    let json = external_json(r#"{"url":"https://subject.example/token"}"#);
    let transport =
        ScriptedTransport::new(vec![ScriptedTransport::text_reply(401, "denied"), sts_ok()]);
    let error = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::SubjectTokenUnavailable { .. }
    ));
    assert_eq!(transport.sent(), 1);
}

// ------------------------------------------------------------------
// AWS (aws1) supplier
// ------------------------------------------------------------------

#[tokio::test]
async fn external_aws_env_credentials_and_region_fixed_vector() {
    let json = external_json(
        r#"{"environment_id":"aws1","regional_cred_verification_url":"https://sts.{region}.example.dev?Action=GetCallerIdentity&Version=2011-06-15"}"#,
    );
    let ambient = MapContext::new(&[
        ("AWS_REGION", "us-east-1"),
        ("AWS_ACCESS_KEY_ID", "AKIDEXAMPLE"),
        (
            "AWS_SECRET_ACCESS_KEY",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        ),
        ("AWS_SESSION_TOKEN", "session-token"),
    ]);
    let transport = ScriptedTransport::new(vec![sts_ok()]);

    let token = resolve(&json, &ambient, &transport).await.unwrap().unwrap();
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::Aws,
            workforce_user_project: None,
        }
    );

    let subject = percent_decode(&form_value(
        transport.request(0).body.as_deref().unwrap(),
        "subject_token",
    ));
    let parsed: serde_json::Value = serde_json::from_str(&subject).unwrap();
    assert_eq!(
        parsed["url"].as_str().unwrap(),
        "https://sts.us-east-1.example.dev?Action=GetCallerIdentity&Version=2011-06-15"
    );
    assert_eq!(parsed["method"].as_str().unwrap(), "POST");
    let headers = parsed["headers"].as_array().unwrap();
    let header = |name: &str| -> String {
        headers
            .iter()
            .find(|h| h["key"] == name)
            .map(|h| h["value"].as_str().unwrap().to_owned())
            .unwrap()
    };
    assert_eq!(header("host"), "sts.us-east-1.example.dev");
    assert!(header("x-amz-date").ends_with('Z'));
    assert_eq!(header("x-amz-security-token"), "session-token");
    assert_eq!(header("x-goog-cloud-target-resource"), audience());
    let authorization = header("authorization");
    assert!(authorization.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/"));
    assert!(authorization.contains("/us-east-1/sts/aws4_request"));
    // The target-resource header is merged AFTER signing (source truth)
    // and must not appear in SignedHeaders.
    let signed_headers = authorization
        .split("SignedHeaders=")
        .nth(1)
        .unwrap()
        .split(',')
        .next()
        .unwrap();
    assert!(!signed_headers.contains("x-goog-cloud-target-resource"));
    assert!(signed_headers.contains("x-amz-security-token"));
}

#[tokio::test]
async fn external_aws_imds_flow_with_v2_token_and_region_strip() {
    let json = external_json(
        r#"{"environment_id":"aws1","region_url":"https://imdsv2.example/region","url":"https://imdsv2.example/creds","imdsv2_session_token_url":"https://imdsv2.example/token","regional_cred_verification_url":"https://sts.{region}.amazonaws.com?Action=GetCallerIdentity&Version=2011-06-15"}"#,
    );
    let transport = ScriptedTransport::new(vec![
        ScriptedTransport::text_reply(200, "imds-token"),
        ScriptedTransport::text_reply(200, "us-east-1a"),
        ScriptedTransport::text_reply(200, "imds-token"),
        ScriptedTransport::text_reply(200, "role-list"),
        ScriptedTransport::text_reply(
            200,
            r#"{"AccessKeyId":"AKIDROLE","SecretAccessKey":"role-secret","Token":"role-session"}"#,
        ),
        sts_ok(),
    ]);
    let token = resolve(&json, &MapContext::new(&[]), &transport)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.access_token.expose_secret(), "fed-token");

    // PUT session token with the fixed TTL header.
    let put = transport.request(0);
    assert_eq!(put.method, "PUT");
    assert_eq!(put.url, "https://imdsv2.example/token");
    assert!(
        put.headers
            .iter()
            .any(|(k, v)| k == "x-aws-ec2-metadata-token-ttl-seconds" && v == "300")
    );

    // Region GET strips the availability-zone suffix.
    let region_get = transport.request(1);
    assert_eq!(region_get.method, "GET");
    assert_eq!(region_get.url, "https://imdsv2.example/region");
    assert!(
        region_get
            .headers
            .iter()
            .any(|(k, v)| k == "x-aws-ec2-metadata-token" && v == "imds-token")
    );

    // Role list, then credentials for `{url}/{role}`; the STS subject is
    // signed at the stripped region.
    let role_get = transport.request(3);
    assert_eq!(role_get.url, "https://imdsv2.example/creds");
    let creds_get = transport.request(4);
    assert_eq!(creds_get.url, "https://imdsv2.example/creds/role-list");
    let subject = percent_decode(&form_value(
        transport.request(5).body.as_deref().unwrap(),
        "subject_token",
    ));
    assert!(subject.contains("sts.us-east-1.amazonaws.com"));
    assert!(subject.contains("AKIDROLE"));
}

#[tokio::test]
async fn external_aws_env_id_v2_is_typed_unsupported() {
    let json = external_json(
        r#"{"environment_id":"aws2","regional_cred_verification_url":"https://sts.{region}.example.dev"}"#,
    );
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));
}

#[tokio::test]
async fn external_aws_missing_regional_url_is_typed() {
    let json = external_json(r#"{"environment_id":"aws1"}"#);
    let error = resolve(
        &json,
        &MapContext::new(&[]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));
}

#[tokio::test]
async fn external_aws_missing_region_url_when_needed_is_typed() {
    let json = external_json(
        r#"{"environment_id":"aws1","url":"https://imds.example/creds","regional_cred_verification_url":"https://sts.{region}.example.dev"}"#,
    );
    let ambient = MapContext::new(&[
        ("AWS_ACCESS_KEY_ID", "AKID"),
        ("AWS_SECRET_ACCESS_KEY", "SK"),
    ]);
    let error = resolve(&json, &ambient, &ScriptedTransport::new(vec![sts_ok()]))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::AwsSubjectUnavailable { reason: _ }
    ));
}

// ------------------------------------------------------------------
// Certificate supplier (source-scoped mTLS)
// ------------------------------------------------------------------

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/adc-external");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(FIXTURES).join(name)).unwrap()
}

fn pem_first_der(pem: &[u8]) -> Vec<u8> {
    let text = std::str::from_utf8(pem).unwrap();
    let body = text
        .split("-----BEGIN CERTIFICATE-----")
        .nth(1)
        .unwrap()
        .split("-----END CERTIFICATE-----")
        .next()
        .unwrap();
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(body.lines().collect::<String>())
        .unwrap()
}

/// One-shot mTLS loopback STS endpoint: requires a client certificate
/// chaining to the fixture CA, captures the exchanged form, and replies
/// with a fixed access token.
async fn mtls_sts_server() -> (
    String,
    std::sync::Arc<std::sync::Mutex<Option<(String, String)>>>,
) {
    use rustls::RootCertStore;
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::server::WebPkiClientVerifier;
    use std::sync::Arc as Arc2;

    let provider = rustls::crypto::aws_lc_rs::default_provider();
    let provider = std::sync::Arc::new(provider);
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_file(std::path::Path::new(FIXTURES).join("ca.pem")).unwrap())
        .unwrap();
    let verifier = WebPkiClientVerifier::builder_with_provider(Arc2::new(roots), provider.clone())
        .build()
        .unwrap();
    let config = rustls::ServerConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![
                CertificateDer::from_pem_file(std::path::Path::new(FIXTURES).join("server.pem"))
                    .unwrap(),
            ],
            PrivateKeyDer::from_pem_file(std::path::Path::new(FIXTURES).join("server.key"))
                .unwrap(),
        )
        .unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured: Arc2<Mutex<Option<(String, String)>>> = Arc2::new(Mutex::new(None));
    let captured_task = captured.clone();
    let config = Arc2::new(config);

    tokio::spawn(async move {
        // Serve a handful of one-shot exchanges so multi-part tests can
        // resolve more than once.
        for _ in 0..4 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut tls = tokio_rustls::TlsAcceptor::from(config.clone())
                .accept(stream)
                .await
                .expect("client must present a CA-chained certificate");
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = tls.read(&mut chunk).await.unwrap();
                if n == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buffer);
                if let Some(header_end) = text.find("\r\n\r\n") {
                    let content_length = text
                        .split("\r\n")
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().ok())
                        })
                        .flatten()
                        .unwrap_or(0);
                    if buffer.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&buffer).into_owned();
            let body = request
                .split("\r\n\r\n")
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            *captured_task.lock().unwrap() = Some((request, body));
            let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 51\r\n\r\n{\"access_token\":\"mtls-fed-token\",\"expires_in\":3600}   ";
            tls.write_all(response.as_bytes()).await.unwrap();
            tls.shutdown().await.unwrap();
        }
    });

    let scheme = "https";
    (format!("{scheme}://{addr}/v1/token"), captured)
}

fn certificate_creds(dir: &std::path::Path, config_name: &str) -> String {
    let config = format!(
        r#"{{"cert_configs":{{"workload":{{"cert_path":{},"key_path":{}}}}}}}"#,
        serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.pem")).unwrap(),
        serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.key")).unwrap(),
    );
    creds_file(dir, config_name, &config);
    external_json(&format!(
        r#"{{"certificate":{{"certificate_config_location":{}}}}}"#,
        serde_json::to_string(&dir.join(config_name)).unwrap()
    ))
}

#[tokio::test]
async fn external_certificate_mtls_exchange() {
    let (sts_url, captured) = mtls_sts_server().await;
    let dir = tempfile::tempdir().unwrap();
    let json = certificate_creds(dir.path(), "certificate_config.json");

    let ambient = MapContext::new(&[]);
    let transport = ScriptedTransport::new(Vec::new());
    let path = creds_file(dir.path(), "external.json", &json);
    let mut file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &transport,
    );
    let extra_roots = fixture("ca.pem");
    file_input.certificate_extra_roots_pem = Some(&extra_roots);
    // The certificate source exchanges over its own mTLS-scoped
    // transport at the loopback endpoint.
    file_input.token_endpoint = Some(&sts_url);

    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::Certificate,
            workforce_user_project: None,
        }
    );
    // The plain transport is never used for the certificate STS leg.
    assert_eq!(transport.sent(), 0);

    let (request, body) = captured.lock().unwrap().clone().unwrap();
    assert!(request.starts_with("POST /v1/token"), "{request}");
    assert!(
        request
            .to_lowercase()
            .contains("content-type: application/x-www-form-urlencoded")
    );
    assert_eq!(
        form_value(&body, "grant_type"),
        "urn:ietf:params:oauth:grant-type:token-exchange"
    );
    // The subject token is the JSON certificate chain built from the
    // leaf (no trust chain configured -> leaf only).
    let subject = form_value(&body, "subject_token");
    let chain: Vec<String> = serde_json::from_str(&subject).unwrap();
    assert_eq!(chain.len(), 1);
    use base64::Engine as _;
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(&chain[0])
            .unwrap(),
        pem_first_der(&fixture("client-leaf.pem"))
    );
}

#[tokio::test]
async fn external_certificate_config_resolution_order_and_xor() {
    // Neither flag nor location is typed at parse time.
    let error = resolve(
        &external_json(r#"{"certificate":{}}"#),
        &MapContext::new(&[]),
        &ScriptedTransport::new(Vec::new()),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));

    // Both at once is typed as well.
    let dir = tempfile::tempdir().unwrap();
    let error = resolve(
            &external_json(
                r#"{"certificate":{"use_default_certificate_config":true,"certificate_config_location":"/x.json"}}"#,
            ),
            &MapContext::new(&[]),
            &ScriptedTransport::new(Vec::new()),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CredentialSourceInvalid { .. }
    ));
    drop(dir);
}

#[tokio::test]
async fn external_certificate_env_and_well_known_resolution() {
    let (sts_url, _captured) = mtls_sts_server().await;
    let config_dir = tempfile::tempdir().unwrap();
    let config_dir_path = config_dir.path().to_path_buf();

    // GOOGLE_API_CERTIFICATE_CONFIG wins over the well-known path.
    let env_config = config_dir_path.join("env-certificate_config.json");
    let well_known_dir = config_dir_path.join("gcloud");
    std::fs::create_dir_all(&well_known_dir).unwrap();
    std::fs::write(
        &env_config,
        format!(
            r#"{{"cert_configs":{{"workload":{{"cert_path":{},"key_path":{}}}}}}}"#,
            serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.pem")).unwrap(),
            serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.key")).unwrap(),
        ),
    )
    .unwrap();
    std::fs::write(
        well_known_dir.join("certificate_config.json"),
        "not-read-when-env-is-set",
    )
    .unwrap();

    let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
    let ambient = MapContext::new(&[
        (
            "GOOGLE_API_CERTIFICATE_CONFIG",
            env_config.to_str().unwrap(),
        ),
        ("HOME", config_dir_path.to_str().unwrap()),
    ]);
    let path = creds_file(config_dir_path.as_path(), "external.json", &json);
    let empty_transport = ScriptedTransport::new(Vec::new());
    let mut file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &empty_transport,
    );
    file_input.token_endpoint = Some(&sts_url);
    let extra_roots = fixture("ca.pem");
    file_input.certificate_extra_roots_pem = Some(&extra_roots);

    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");

    // Without the env var the well-known gcloud path is used
    // (CLOUDSDK_CONFIG pins the gcloud directory).
    let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
    let ambient = MapContext::new(&[("CLOUDSDK_CONFIG", config_dir_path.to_str().unwrap())]);
    let path = creds_file(config_dir_path.as_path(), "external2.json", &json);
    let empty_transport2 = ScriptedTransport::new(Vec::new());
    let well_known_config = format!(
        r#"{{"cert_configs":{{"workload":{{"cert_path":{},"key_path":{}}}}}}}"#,
        serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.pem")).unwrap(),
        serde_json::to_string(&std::path::Path::new(FIXTURES).join("client-leaf.key")).unwrap(),
    );
    std::fs::write(
        well_known_dir.join("certificate_config.json"),
        well_known_config,
    )
    .unwrap();
    let mut file_input = input(
        AdcAuthSelection::CredentialsFile(path),
        &ambient,
        &empty_transport2,
    );
    file_input.token_endpoint = Some(&sts_url);
    let extra_roots2 = fixture("ca.pem");
    file_input.certificate_extra_roots_pem = Some(&extra_roots2);

    let token = opi_ai::auth::adc::native::resolve_adc_native(&file_input)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(token.access_token.expose_secret(), "mtls-fed-token");
}

#[tokio::test]
async fn external_certificate_missing_everywhere_is_typed() {
    let json = external_json(r#"{"certificate":{"use_default_certificate_config":true}}"#);
    let error = resolve(
        &json,
        &MapContext::new(&[("HOME", "/nonexistent-adc-test-home")]),
        &ScriptedTransport::new(Vec::new()),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CertificateSourceUnavailable { .. }
    ));
}

#[test]
fn external_certificate_chain_leaf_order_rules() {
    let leaf = pem_first_der(&fixture("client-leaf.pem"));
    let other = pem_first_der(&fixture("other-leaf.pem"));

    // Leaf absent from the chain: prepended.
    let chain = other_leaf_chain_pem();
    let ordered = opi_ai::auth::adc::subject::assemble_certificate_chain(&leaf, &chain).unwrap();
    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0], leaf);
    assert_eq!(ordered[1], other);

    // Leaf already first: order preserved.
    let mut leaf_first = fixture("client-leaf.pem");
    leaf_first.extend_from_slice(&fixture("other-leaf.pem"));
    let ordered =
        opi_ai::auth::adc::subject::assemble_certificate_chain(&leaf, &leaf_first).unwrap();
    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0], leaf);

    // Leaf present but not first: typed failure.
    let mut chain = fixture("client-leaf.pem");
    chain.extend_from_slice(&fixture("other-leaf.pem"));
    let error = opi_ai::auth::adc::subject::assemble_certificate_chain(&other, &chain).unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::CertificateSourceUnavailable { .. }
    ));
}

fn other_leaf_chain_pem() -> Vec<u8> {
    fixture("other-leaf.pem")
}

// ------------------------------------------------------------------
// Executable supplier
// ------------------------------------------------------------------

/// Write a helper script for the current platform (bodies for both
/// platforms stay plain strings so the acceptance source carries no
/// conditional compilation) and return the executable command string.
fn helper(dir: &std::path::Path, name: &str, unix: &str, windows: &str) -> String {
    opi_ai::test_support::portable_executable_script(dir, name, unix, windows)
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn executable_json(command: &str, output_file: Option<&std::path::Path>) -> String {
    let command = json_escape(command);
    match output_file {
        Some(file) => external_json(&format!(
            r#"{{"executable":{{"command":"{command}","output_file":{}}}}}"#,
            serde_json::to_string(&file).unwrap()
        )),
        None => external_json(&format!(r#"{{"executable":{{"command":"{command}"}}}}"#)),
    }
}

fn ok_response() -> String {
    let expires = 1_700_000_000 + 3600;
    format!(
        r#"{{"version":1,"success":true,"token_type":"{}","id_token":"helper-id-token","expiration_time":{expires}}}"#,
        subject_token_type()
    )
}

fn spawn_helper(dir: &std::path::Path, env_dump: &std::path::Path, response: &str) -> String {
    let unix = format!(
        "env > \"{}\"\nprintf '%s' '{}'",
        env_dump.display(),
        response
    );
    let windows = format!(
        "Get-ChildItem env: | ForEach-Object {{ \"$($_.Name)=$($_.Value)\" }} | Out-File -Encoding utf8 \"{}\"\nWrite-Host -NoNewline '{}'",
        env_dump.display(),
        response.replace('\'', "\"")
    );
    helper(dir, "helper", &unix, &windows)
}

#[test]
fn parse_command_quotes_and_segments() {
    use opi_ai::auth::adc::executable::parse_command;

    let argv = parse_command("helper --flag \"quoted value\" tail").unwrap();
    assert_eq!(argv, vec!["helper", "--flag", "quoted value", "tail"]);

    // Mixed unquoted and quoted segments concatenate into one argument
    // with inner quotes preserved (source regex behavior).
    let argv = parse_command("a\"b c\"d").unwrap();
    assert_eq!(argv, vec![r#"a"b c"d"#]);

    // Only a fully-quoted argument loses its outer quotes.
    let argv = parse_command("\"x y\" z").unwrap();
    assert_eq!(argv, vec!["x y", "z"]);

    assert!(parse_command("   ").is_none());
}

#[tokio::test]
async fn executable_gate_precedes_cache_and_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let output_file = dir.path().join("cache.json");
    std::fs::write(&output_file, ok_response()).unwrap();
    let marker = dir.path().join("marker");
    let unix = format!("printf x > \"{}\"", marker.display());
    let windows = format!("Set-Content -Path \"{}\" -Value x", marker.display());
    let command = helper(dir.path(), "gate-helper", &unix, &windows);

    let json = executable_json(&command, Some(&output_file));
    // Gate not set (and not "1"): typed failure, cache never read, the
    // executable never spawns.
    for gate in [None, Some("0"), Some("yes")] {
        let env: Vec<(&str, &str)> = gate
            .map(|v| vec![("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", v)])
            .unwrap_or_default();
        let error = resolve(
            &json,
            &MapContext::new(&env),
            &ScriptedTransport::new(vec![sts_ok()]),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, AdcResolutionError::ExecutablesNotAllowed),
            "gate {gate:?}"
        );
    }
    assert!(!marker.exists(), "the gate must precede any spawn");
}

#[tokio::test]
async fn executable_fresh_cache_avoids_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let output_file = dir.path().join("cache.json");
    std::fs::write(&output_file, ok_response()).unwrap();
    let env_dump = dir.path().join("env.txt");
    let command = spawn_helper(dir.path(), &env_dump, &ok_response());
    let json = executable_json(&command, Some(&output_file));

    let transport = ScriptedTransport::new(vec![sts_ok()]);
    let token = resolve(
        &json,
        &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
        &transport,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(!env_dump.exists(), "fresh cache must not spawn");
    assert_eq!(
        form_value(
            transport.request(0).body.as_deref().unwrap(),
            "subject_token"
        ),
        "helper-id-token"
    );
    assert_eq!(
        token.source,
        AdcCredentialSource::ExternalAccount {
            supplier: ExternalAccountSupplier::Executable,
            workforce_user_project: None,
        }
    );
}

#[tokio::test]
async fn executable_stale_cache_spawns_with_source_env() {
    let dir = tempfile::tempdir().unwrap();
    let output_file = dir.path().join("cache.json");
    // Expired response: stale cache is a miss, the executable runs.
    let stale = ok_response().replace(
        &format!("\"expiration_time\":{}", 1_700_000_000 + 3600),
        "\"expiration_time\":1000",
    );
    std::fs::write(&output_file, stale).unwrap();
    let env_dump = dir.path().join("env.txt");
    let command = spawn_helper(dir.path(), &env_dump, &ok_response());
    let impersonation = ",\"service_account_impersonation_url\":\"https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/sa@proj.iam.gserviceaccount.com:generateAccessToken\"";
    let json = format!(
        r#"{{"type":"external_account","audience":"{}","subject_token_type":"{}"{impersonation},"credential_source":{{"executable":{{"command":"{}","output_file":{}}}}}}}"#,
        audience(),
        subject_token_type(),
        json_escape(&command),
        serde_json::to_string(&output_file).unwrap()
    );

    // 20.21.3: the file's impersonation URL now activates the IAM leg
    // after the STS exchange, so the transport scripts both replies.
    let transport = ScriptedTransport::new(vec![
        sts_ok(),
        ScriptedTransport::text_reply(
            200,
            r#"{"accessToken":"imp-token","expireTime":"2030-06-01T00:00:00Z"}"#,
        ),
    ]);
    resolve(
        &json,
        &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
        &transport,
    )
    .await
    .unwrap()
    .unwrap();

    let dumped = std::fs::read_to_string(&env_dump).unwrap();
    assert!(dumped.contains("GOOGLE_EXTERNAL_ACCOUNT_AUDIENCE="));
    assert!(dumped.contains(&format!(
        "GOOGLE_EXTERNAL_ACCOUNT_TOKEN_TYPE={}",
        subject_token_type()
    )));
    assert!(dumped.contains("GOOGLE_EXTERNAL_ACCOUNT_INTERACTIVE=0"));
    assert!(dumped.contains(&format!(
        "GOOGLE_EXTERNAL_ACCOUNT_OUTPUT_FILE={}",
        output_file.display()
    )));
    assert!(
        dumped
            .contains("GOOGLE_EXTERNAL_ACCOUNT_IMPERSONATED_EMAIL=sa@proj.iam.gserviceaccount.com")
    );
    assert_eq!(
        form_value(
            transport.request(0).body.as_deref().unwrap(),
            "subject_token"
        ),
        "helper-id-token"
    );
}

#[tokio::test]
async fn executable_malformed_cache_is_typed_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let output_file = dir.path().join("cache.json");
    std::fs::write(&output_file, "not json at all").unwrap();
    let env_dump = dir.path().join("env.txt");
    let command = spawn_helper(dir.path(), &env_dump, &ok_response());
    let json = executable_json(&command, Some(&output_file));

    let error = resolve(
        &json,
        &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableResponseInvalid { .. }
    ));
    // A malformed cache is terminal: the executable never runs.
    assert!(!env_dump.exists());
}

#[tokio::test]
async fn executable_failure_and_validation_rules() {
    let gate = &[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")];

    // Non-success response carries code + message.
    let dir = tempfile::tempdir().unwrap();
    let env_dump = dir.path().join("env.txt");
    let failure =
        r#"{"version":1,"success":false,"code":"401","message":"helper denied"}"#.to_owned();
    let command = spawn_helper(dir.path(), &env_dump, &failure);
    let error = resolve(
        &executable_json(&command, None),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableFailed { ref code, .. } if code.as_str() == "[REDACTED]"
    ));

    // Invalid token_type URN.
    let bad_type = ok_response().replace(subject_token_type(), "urn:custom:token");
    let dir = tempfile::tempdir().unwrap();
    let env_dump = dir.path().join("env.txt");
    let command = spawn_helper(dir.path(), &env_dump, &bad_type);
    let error = resolve(
        &executable_json(&command, None),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableResponseInvalid { .. }
    ));

    // Unsupported version.
    let dir = tempfile::tempdir().unwrap();
    let env_dump = dir.path().join("env.txt");
    let v2 = ok_response().replace("\"version\":1", "\"version\":2");
    let command = spawn_helper(dir.path(), &env_dump, &v2);
    let error = resolve(
        &executable_json(&command, None),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableResponseInvalid { .. }
    ));

    // Missing expiration_time with an output_file configured.
    let dir = tempfile::tempdir().unwrap();
    let env_dump = dir.path().join("env.txt");
    let no_expiry = ok_response().replace(
        &format!(",\"expiration_time\":{}", 1_700_000_000 + 3600),
        "",
    );
    let command = spawn_helper(dir.path(), &env_dump, &no_expiry);
    let error = resolve(
        &executable_json(&command, Some(&dir.path().join("unused-cache.json"))),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableResponseInvalid { .. }
    ));

    // Non-zero exit merges output and reports the code.
    let dir = tempfile::tempdir().unwrap();
    let unix = "printf '%s' 'boom on stderr' >&2\nexit 3";
    let windows = "Write-Error 'boom on stderr'\nexit 3";
    let command = helper(dir.path(), "failing", unix, windows);
    let error = resolve(
        &executable_json(&command, None),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableFailed { ref code, ref message }
            if code.as_str() == "3" && message.as_str() == "[REDACTED]"
    ));

    // Expired response (fresh spawn) is typed.
    let dir = tempfile::tempdir().unwrap();
    let env_dump = dir.path().join("env.txt");
    let expired = ok_response().replace(
        &format!("\"expiration_time\":{}", 1_700_000_000 + 3600),
        "\"expiration_time\":1000",
    );
    let command = spawn_helper(dir.path(), &env_dump, &expired);
    let error = resolve(
        &executable_json(&command, None),
        &MapContext::new(gate),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        AdcResolutionError::ExecutableResponseExpired
    ));
}

#[tokio::test]
async fn executable_timeout_kills_the_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("marker");
    let unix = format!(
        "while :; do printf x >> \"{}\"; sleep 0.1; done",
        marker.display()
    );
    let windows = format!(
        "while ($true) {{ Add-Content -Path \"{}\" -Value x; Start-Sleep -Milliseconds 100 }}",
        marker.display()
    );
    let command = helper(dir.path(), "loop", &unix, &windows);
    let json = external_json(&format!(
        r#"{{"executable":{{"command":"{}","timeout_millis":5000}}}}"#,
        json_escape(&command)
    ));

    let start = std::time::Instant::now();
    let error = resolve(
        &json,
        &MapContext::new(&[("GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES", "1")]),
        &ScriptedTransport::new(vec![sts_ok()]),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AdcResolutionError::ExecutableTimeout));
    assert!(start.elapsed() >= std::time::Duration::from_secs(4));

    // The whole process group died: the marker stops growing.
    let size_after_kill = std::fs::metadata(&marker).map(|m| m.len()).unwrap_or(0);
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    let size_later = std::fs::metadata(&marker).map(|m| m.len()).unwrap_or(0);
    assert_eq!(size_after_kill, size_later, "process group must be reaped");
    drop(json);
}

#[tokio::test]
async fn executable_timeout_range_is_parse_time() {
    // Below the 5s floor and above the 120s ceiling are typed at parse.
    for timeout in [4_999u64, 120_001] {
        let json = external_json(&format!(
            r#"{{"executable":{{"command":"sh \"/nonexistent/helper\"","timeout_millis":{timeout}}}}}"#
        ));
        let error = resolve(
            &json,
            &MapContext::new(&[]),
            &ScriptedTransport::new(Vec::new()),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, AdcResolutionError::CredentialSourceInvalid { .. }),
            "{timeout}"
        );
    }
}
