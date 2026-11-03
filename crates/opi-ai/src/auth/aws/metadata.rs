//! ECS and IMDS (instance metadata) credential sources.
//!
//! Fixed-budget owned HTTP metadata legs, per the verified locked-Node
//! corpus:
//!
//! - **ECS** — `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` (host
//!   `169.254.170.2`) is preferred over `AWS_CONTAINER_CREDENTIALS_FULL_URI`;
//!   the token *file* is preferred over the direct token. The modern
//!   provider uses a 1s per-attempt timeout, at most four sends with a
//!   1000ms retry delay; its continue-eligible failure falls to the legacy
//!   container provider (1s, exactly one send, narrower host rules). A
//!   selected ECS segment never falls through to IMDS.
//! - **IMDS** — IMDSv2 token first (`PUT /latest/api/token`, 21600s TTL).
//!   Token 400 stops that request; timeout/403/404/405 enable v1 mode for
//!   subsequent legs; other token statuses attempt a one-shot v1 fallback.
//!   v1 can be forbidden (`AWS_EC2_METADATA_DISABLED`, explicit input, or
//!   profile). A 401 on the role/credential legs clears the token state and
//!   re-fetches. Every leg is 1s with zero retries. The endpoint is the
//!   configured/IPv4/IPv6-mode address; no construction-time probing.
//!
//! Credential stale-extension (static stability) is deliberately not
//! implemented: opi never extends expired credentials, a recorded stricter
//! divergence from the SDK wrapper.

use std::time::Duration;

use secrecy::SecretString;
use time::OffsetDateTime;

use crate::auth::AwsSigV4Credentials;
use crate::auth::aws::source_http::{
    AwsSourceTransport, SourceHttpError, SourceHttpMethod, SourceHttpRequest,
};

/// Per-attempt timeout for metadata legs (fixed by the corpus).
const ATTEMPT_TIMEOUT: Duration = Duration::from_millis(1000);
/// Retry delay between modern ECS attempts (fixed by the corpus).
const ECS_RETRY_DELAY: Duration = Duration::from_millis(1000);
/// Modern ECS send budget: at most four sends.
const ECS_MODERN_MAX_SENDS: usize = 4;
/// Fixed ECS relative-URI host.
const ECS_HOST: &str = "169.254.170.2";
/// IMDS IPv4 endpoint.
const IMDS_IPV4: &str = "169.254.169.254";
/// IMDS IPv6 endpoint.
const IMDS_IPV6: &str = "[fd00:ec2::254]";
/// IMDSv2 token TTL.
const IMDS_TOKEN_TTL_SECONDS: &str = "21600";

/// ECS endpoint inputs (resolved from scoped/ambient environment).
#[derive(Debug, Clone)]
pub(crate) struct EcsInputs {
    /// `AWS_CONTAINER_CREDENTIALS_RELATIVE_URI` (preferred when present).
    pub relative_uri: Option<String>,
    /// `AWS_CONTAINER_CREDENTIALS_FULL_URI`.
    pub full_uri: Option<String>,
    /// `AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE` (preferred).
    pub authorization_token_file: Option<String>,
    /// `AWS_CONTAINER_AUTHORIZATION_TOKEN`.
    pub authorization_token: Option<String>,
}

impl EcsInputs {
    /// Whether any ECS endpoint is configured.
    pub(crate) fn configured(&self) -> bool {
        self.relative_uri.is_some() || self.full_uri.is_some()
    }

    /// The selected endpoint URL: relative wins over full.
    fn endpoint(&self) -> Option<String> {
        if let Some(relative) = self
            .relative_uri
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let relative = if relative.starts_with('/') {
                relative.to_owned()
            } else {
                format!("/{relative}")
            };
            return Some(format!("http://{ECS_HOST}{relative}"));
        }
        self.full_uri
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    }
}

/// Typed ECS segment failures (secret-free).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub(crate) enum EcsError {
    /// The configured full URI violates the host/protocol rules. Terminal
    /// for the whole chain.
    #[error("ECS endpoint violates the host or protocol rules")]
    InvalidEndpoint,
    /// The segment failed at the transport/status level after its fixed
    /// budget. Continue-eligible: the chain ends (no IMDS fallthrough).
    #[error("ECS credentials were unavailable after the fixed budget")]
    Unavailable,
    /// A 2xx response did not satisfy the credential shape.
    #[error("ECS credentials response was invalid")]
    InvalidResponse,
}

/// Validate one full URI against the fixed host/protocol rules.
///
/// `https` allows any host; plain `http` is allowed only for loopback or
/// link-local (`169.254.0.0/16`) hosts.
pub(crate) fn validate_full_uri(uri: &str) -> Result<(), EcsError> {
    let Ok(parsed) = url::Url::parse(uri) else {
        return Err(EcsError::InvalidEndpoint);
    };
    let Some(host) = parsed.host() else {
        return Err(EcsError::InvalidEndpoint);
    };
    match parsed.scheme() {
        "https" => Ok(()),
        "http" => {
            if is_loopback_host(&host)
                || matches!(host, url::Host::Ipv4(address) if address.is_link_local())
            {
                Ok(())
            } else {
                Err(EcsError::InvalidEndpoint)
            }
        }
        _ => Err(EcsError::InvalidEndpoint),
    }
}

fn is_loopback_host(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Domain(name) => *name == "localhost",
        url::Host::Ipv4(address) => address.is_loopback(),
        url::Host::Ipv6(address) => address.is_loopback(),
    }
}

fn is_legacy_host(host: &url::Host<&str>) -> bool {
    is_loopback_host(host)
        || matches!(host, url::Host::Ipv4(address) if matches!(address.octets(), [169, 254, 170, 2 | 23]))
}

/// One metadata credentials payload.
#[derive(Debug, Clone)]
pub(crate) struct MetadataCredentials {
    /// Access key id.
    pub access_key_id: SecretString,
    /// Secret access key.
    pub secret_access_key: SecretString,
    /// Session token.
    pub session_token: Option<SecretString>,
    /// Source-reported expiration.
    pub expiration: Option<OffsetDateTime>,
}

impl MetadataCredentials {
    /// Build SigV4 credentials for `region`.
    pub fn into_credentials(self, region: &str) -> AwsSigV4Credentials {
        AwsSigV4Credentials {
            access_key_id: self.access_key_id,
            secret_access_key: self.secret_access_key,
            session_token: self.session_token,
            region: region.to_owned(),
            expires_at: self.expiration,
        }
    }
}

/// Parse and validate one metadata JSON payload.
pub(crate) fn parse_metadata_credentials(body: &[u8]) -> Option<MetadataCredentials> {
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    let access = value.get("AccessKeyId")?.as_str()?.trim().to_owned();
    let secret = value.get("SecretAccessKey")?.as_str()?.trim().to_owned();
    if access.is_empty() || secret.is_empty() {
        return None;
    }
    let session_token = value
        .get("Token")
        .and_then(|token| token.as_str())
        .filter(|token| !token.trim().is_empty())
        .map(|token| SecretString::from(token.to_owned()));
    let expiration = value
        .get("Expiration")
        .and_then(|exp| exp.as_str())
        .and_then(|exp| {
            OffsetDateTime::parse(exp, &time::format_description::well_known::Rfc3339).ok()
        });
    Some(MetadataCredentials {
        access_key_id: SecretString::from(access),
        secret_access_key: SecretString::from(secret),
        session_token,
        expiration,
    })
}

/// Resolve the ECS segment: modern provider (fixed budget), then the legacy
/// container provider exactly once. Returns the credentials plus whether
/// the legacy provider produced them. `Selected` never falls through to
/// IMDS.
pub(crate) async fn resolve_ecs(
    transport: &dyn AwsSourceTransport,
    inputs: &EcsInputs,
) -> Result<(MetadataCredentials, bool), EcsError> {
    let Some(endpoint) = inputs.endpoint() else {
        return Err(EcsError::Unavailable);
    };
    // Endpoint host/protocol validation is terminal for the whole chain.
    if inputs.relative_uri.is_none() {
        validate_full_uri(&endpoint)?;
    }

    // Token file preferred over the direct token.
    let token = match &inputs.authorization_token_file {
        Some(path) => Some(
            tokio::fs::read_to_string(path)
                .await
                .map_err(|_| EcsError::Unavailable)?
                .trim()
                .to_owned(),
        ),
        None => inputs.authorization_token.as_deref().map(str::to_owned),
    };

    // Modern provider: up to four sends with the fixed delay. Transport
    // failures, 429, and 5xx are retry-eligible; other statuses stop the
    // modern budget early.
    let mut sends = 0;
    loop {
        sends += 1;
        let request = ecs_request(&endpoint, token.as_deref());
        match transport.send(request).await {
            Ok(response) if (200..300).contains(&response.status) => {
                return parse_metadata_credentials(&response.body)
                    .map(|credentials| (credentials, false))
                    .ok_or(EcsError::InvalidResponse);
            }
            Ok(response) if sends < ECS_MODERN_MAX_SENDS && retryable_status(response.status) => {
                tokio::time::sleep(ECS_RETRY_DELAY).await;
            }
            Err(_) if sends < ECS_MODERN_MAX_SENDS => {
                tokio::time::sleep(ECS_RETRY_DELAY).await;
            }
            Ok(_) | Err(_) => break,
        }
    }

    // Legacy container provider: one send, 1s, narrower host rules.
    let legacy_endpoint = match inputs
        .relative_uri
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        Some(_) => endpoint.clone(),
        None => {
            // The legacy provider applies narrower host/protocol checks to
            // full URIs.
            validate_full_uri(&endpoint)?;
            let parsed = url::Url::parse(&endpoint).map_err(|_| EcsError::InvalidEndpoint)?;
            let host = parsed.host().ok_or(EcsError::InvalidEndpoint)?;
            if !is_legacy_host(&host) && parsed.scheme() != "https" {
                return Err(EcsError::InvalidEndpoint);
            }
            endpoint.clone()
        }
    };
    let request = ecs_request(&legacy_endpoint, inputs.authorization_token.as_deref());
    match transport.send(request).await {
        Ok(response) if (200..300).contains(&response.status) => {
            parse_metadata_credentials(&response.body)
                .map(|credentials| (credentials, true))
                .ok_or(EcsError::InvalidResponse)
        }
        Ok(_) => Err(EcsError::Unavailable),
        Err(_) => Err(EcsError::Unavailable),
    }
}

fn ecs_request(endpoint: &str, token: Option<&str>) -> SourceHttpRequest {
    let mut headers = Vec::new();
    if let Some(token) = token.filter(|token| !token.is_empty()) {
        headers.push(("Authorization".to_owned(), token.to_owned()));
    }
    SourceHttpRequest {
        method: SourceHttpMethod::Get,
        url: endpoint.to_owned(),
        headers,
        body: None,
        timeout: Some(ATTEMPT_TIMEOUT),
    }
}

/// Whether one status is retry-eligible within the modern budget.
fn retryable_status(status: u16) -> bool {
    status == 429 || status >= 500
}

/// IMDS endpoint mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ImdsEndpointMode {
    /// `http://169.254.169.254` (default).
    #[default]
    Ipv4,
    /// `http://[fd00:ec2::254]`.
    Ipv6,
}

impl ImdsEndpointMode {
    fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("IPv6") | Some("ipv6") => Self::Ipv6,
            _ => Self::Ipv4,
        }
    }
}

/// IMDS inputs (resolved from scoped/ambient environment).
#[derive(Debug, Clone)]
pub(crate) struct ImdsInputs {
    /// `AWS_METADATA_SERVICE_ENDPOINT` override.
    pub endpoint_override: Option<String>,
    /// `AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE`.
    pub endpoint_mode: Option<String>,
    /// `AWS_EC2_METADATA_DISABLED` (explicit input or env), already parsed.
    pub v1_disabled: bool,
}

impl ImdsInputs {
    fn base_url(&self) -> String {
        if let Some(endpoint) = self
            .endpoint_override
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return endpoint.trim().trim_end_matches('/').to_owned();
        }
        match ImdsEndpointMode::parse(self.endpoint_mode.as_deref()) {
            ImdsEndpointMode::Ipv4 => format!("http://{IMDS_IPV4}"),
            ImdsEndpointMode::Ipv6 => format!("http://{IMDS_IPV6}"),
        }
    }
}

/// Parse `AWS_EC2_METADATA_DISABLED` semantics: non-empty and not the string
/// `false` disables the v1 fallback (and, at the chain level, the default
/// remote branch).
pub(crate) fn parse_metadata_disabled(raw: Option<&str>) -> bool {
    match raw {
        Some(value) => {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed != "false"
        }
        None => false,
    }
}

/// Typed IMDS failures (secret-free).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub(crate) enum ImdsError {
    /// The token or role/credential legs failed within their fixed budgets.
    #[error("IMDS credentials were unavailable")]
    Unavailable,
    /// A 2xx payload did not satisfy the credential shape.
    #[error("IMDS credentials response was invalid")]
    InvalidResponse,
}

enum TokenOutcome {
    Token(String),
    /// 400: stop this request without any v1 fallback.
    Stop,
    /// Timeout/403/404/405: v1 mode persists for subsequent legs.
    EnableV1,
    /// Other token failures: one-shot v1 fallback for this request only.
    OneShotV1,
}

/// Resolve IMDS credentials through the v2/v1 branch tree.
pub(crate) async fn resolve_imds(
    transport: &dyn AwsSourceTransport,
    inputs: &ImdsInputs,
) -> Result<MetadataCredentials, ImdsError> {
    let base = inputs.base_url();
    let mut v1_mode = false;

    for attempt in 0..2 {
        if attempt > 0 {
            // Second pass: after a v1 401 cleared the token state.
            v1_mode = false;
        }
        let token = if v1_mode {
            None
        } else {
            match fetch_token(transport, &base).await {
                TokenOutcome::Token(token) => Some(token),
                TokenOutcome::Stop => return Err(ImdsError::Unavailable),
                TokenOutcome::EnableV1 => {
                    if inputs.v1_disabled {
                        return Err(ImdsError::Unavailable);
                    }
                    v1_mode = true;
                    None
                }
                TokenOutcome::OneShotV1 => {
                    if inputs.v1_disabled {
                        return Err(ImdsError::Unavailable);
                    }
                    None
                }
            }
        };

        // Role list leg.
        let role = match metadata_get(transport, &base, ROLE_LIST_PATH, token.as_deref()).await {
            GetOutcome::Body(body) => first_line(&body).ok_or(ImdsError::InvalidResponse)?,
            GetOutcome::Unauthorized401 if attempt == 0 => {
                // 401 clears the (v1) token state and retries with a fresh
                // token.
                continue;
            }
            _ => return Err(ImdsError::Unavailable),
        };

        // Credentials leg.
        return match metadata_get(
            transport,
            &base,
            &format!("{ROLE_LIST_PATH}{role}"),
            token.as_deref(),
        )
        .await
        {
            GetOutcome::Body(body) => {
                parse_metadata_credentials(&body).ok_or(ImdsError::InvalidResponse)
            }
            GetOutcome::Unauthorized401 if attempt == 0 => continue,
            _ => Err(ImdsError::Unavailable),
        };
    }
    Err(ImdsError::Unavailable)
}

const ROLE_LIST_PATH: &str = "/latest/meta-data/iam/security-credentials/";

enum GetOutcome {
    Body(Vec<u8>),
    Unauthorized401,
    Failed,
}

async fn metadata_get(
    transport: &dyn AwsSourceTransport,
    base: &str,
    path: &str,
    token: Option<&str>,
) -> GetOutcome {
    let mut headers = Vec::new();
    if let Some(token) = token.filter(|token| !token.is_empty()) {
        headers.push(("x-aws-ec2-metadata-token".to_owned(), token.to_owned()));
    }
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Get,
        url: format!("{base}{path}"),
        headers,
        body: None,
        timeout: Some(ATTEMPT_TIMEOUT),
    };
    match transport.send(request).await {
        Ok(response) if response.status == 401 => GetOutcome::Unauthorized401,
        Ok(response) if (200..300).contains(&response.status) => GetOutcome::Body(response.body),
        _ => GetOutcome::Failed,
    }
}

async fn fetch_token(transport: &dyn AwsSourceTransport, base: &str) -> TokenOutcome {
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Put,
        url: format!("{base}/latest/api/token"),
        headers: vec![(
            "x-aws-ec2-metadata-token-ttl-seconds".to_owned(),
            IMDS_TOKEN_TTL_SECONDS.to_owned(),
        )],
        body: None,
        timeout: Some(ATTEMPT_TIMEOUT),
    };
    match transport.send(request).await {
        Ok(response) if response.status == 400 => TokenOutcome::Stop,
        Ok(response) if response.status == 403 => TokenOutcome::EnableV1,
        Ok(response) if response.status == 404 || response.status == 405 => TokenOutcome::EnableV1,
        Ok(response) if (200..300).contains(&response.status) => {
            let token = String::from_utf8_lossy(&response.body).trim().to_owned();
            if token.is_empty() {
                TokenOutcome::OneShotV1
            } else {
                TokenOutcome::Token(token)
            }
        }
        Err(SourceHttpError::Timeout) => TokenOutcome::EnableV1,
        _ => TokenOutcome::OneShotV1,
    }
}

fn first_line(body: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(body);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_uri_host_protocol_rules() {
        assert!(validate_full_uri("https://credentials.example.com/creds").is_ok());
        assert!(validate_full_uri("http://127.0.0.1:8080/creds").is_ok());
        assert!(validate_full_uri("http://[::1]/creds").is_ok());
        assert!(validate_full_uri("http://169.254.170.23/creds").is_ok());
        assert!(validate_full_uri("http://credentials.example.com/creds").is_err());
        assert!(validate_full_uri("ftp://169.254.170.2/creds").is_err());
        assert!(validate_full_uri("not a url").is_err());
    }

    #[test]
    fn relative_uri_wins_and_builds_the_fixed_host() {
        let inputs = EcsInputs {
            relative_uri: Some("/v2/x".to_owned()),
            full_uri: Some("https://full.example.invalid/creds".to_owned()),
            authorization_token_file: None,
            authorization_token: None,
        };
        assert_eq!(
            inputs.endpoint().as_deref(),
            Some("http://169.254.170.2/v2/x")
        );
    }

    #[test]
    fn metadata_disabled_parses_the_corpus_semantics() {
        assert!(parse_metadata_disabled(Some("true")));
        assert!(parse_metadata_disabled(Some("1")));
        assert!(!parse_metadata_disabled(Some("false")));
        assert!(!parse_metadata_disabled(Some("")));
        assert!(!parse_metadata_disabled(None));
    }

    #[test]
    fn imds_endpoint_modes() {
        let ipv4 = ImdsInputs {
            endpoint_override: None,
            endpoint_mode: None,
            v1_disabled: false,
        };
        assert_eq!(ipv4.base_url(), "http://169.254.169.254");
        let ipv6 = ImdsInputs {
            endpoint_override: None,
            endpoint_mode: Some("IPv6".to_owned()),
            v1_disabled: false,
        };
        assert_eq!(ipv6.base_url(), "http://[fd00:ec2::254]");
        let custom = ImdsInputs {
            endpoint_override: Some("http://127.0.0.1:9911".to_owned()),
            endpoint_mode: Some("IPv6".to_owned()),
            v1_disabled: false,
        };
        assert_eq!(custom.base_url(), "http://127.0.0.1:9911");
    }

    #[test]
    fn metadata_credentials_validation() {
        let good = parse_metadata_credentials(
            br#"{"AccessKeyId":"A","SecretAccessKey":"S","Token":"T","Expiration":"2027-01-01T00:00:00Z"}"#,
        )
        .expect("valid payload");
        use secrecy::ExposeSecret;
        assert_eq!(ExposeSecret::expose_secret(&good.access_key_id), "A");
        assert_eq!(
            good.expiration,
            Some(
                OffsetDateTime::parse(
                    "2027-01-01T00:00:00Z",
                    &time::format_description::well_known::Rfc3339
                )
                .unwrap()
            )
        );
        assert!(
            parse_metadata_credentials(br#"{"AccessKeyId":"A"}"#).is_none(),
            "missing secret is invalid"
        );
        assert!(
            parse_metadata_credentials(b"not json").is_none(),
            "non-JSON is invalid"
        );
    }
}
