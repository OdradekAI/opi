//! External-account subject-token suppliers (task 20.21.2).
//!
//! Pinned from `google-auth-library@10.6.2`: the file supplier resolves the
//! path (symlinks included) and keeps the raw text or the named JSON field;
//! the URL supplier issues one GET with the configured headers and parses
//! the same formats; the certificate supplier resolves the certificate
//! configuration (`certificate_config_location` →
//! `GOOGLE_API_CERTIFICATE_CONFIG` → the well-known gcloud path), validates
//! the leaf and trust chain, builds the JSON base64(DER) chain subject
//! token, and yields the mTLS identity for the STS leg; the AWS (aws1)
//! supplier resolves the region (`AWS_REGION` > `AWS_DEFAULT_REGION` >
//! `region_url` with the availability-zone suffix stripped) and security
//! credentials (environment first, else the security-credentials endpoint
//! with an optional IMDSv2 session token), then signs a serialized
//! `GetCallerIdentity` request. All IMDS legs are single sends (the source
//! passes no retry configuration to the default supplier).

use std::path::{Path, PathBuf};

use rustls::RootCertStore;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use secrecy::{ExposeSecret, SecretString};

use crate::auth::{AuthContext, AwsSigV4Credentials};
use crate::bedrock::sigv4::sign_request;

use super::native::{
    AdcClock, AdcResolutionError, AdcSourceRequest, AdcSourceResponse, AdcSourceTransport,
    GaxiosBudget, send_with_gaxios_retry,
};

/// Subject-token formats shared by the file and URL suppliers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubjectFormat {
    /// Raw text (default).
    Text,
    /// JSON with the named subject-token field.
    Json { field: String },
}

/// Whether one HTTP status is a success for supplier legs.
fn is_success(status: u16) -> bool {
    (200..=299).contains(&status)
}

// ---------------------------------------------------------------------------
// File supplier
// ---------------------------------------------------------------------------

/// Read the subject token from a resolved file (text or named JSON field).
pub(crate) async fn file_subject_token(
    file: &Path,
    format: &SubjectFormat,
) -> Result<String, AdcResolutionError> {
    let meta = tokio::fs::metadata(file).await.map_err(|error| {
        AdcResolutionError::SubjectTokenUnavailable {
            reason: format!(
                "the file at {} does not exist, or it is not a file: {error}",
                file.display()
            ),
        }
    })?;
    if !meta.is_file() {
        return Err(AdcResolutionError::SubjectTokenUnavailable {
            reason: format!(
                "the file at {} does not exist, or it is not a file",
                file.display()
            ),
        });
    }
    let raw = tokio::fs::read_to_string(file).await.map_err(|error| {
        AdcResolutionError::SubjectTokenUnavailable {
            reason: format!("failed to read {}: {error}", file.display()),
        }
    })?;
    extract_subject_token(&raw, format, "file")
}

fn extract_subject_token(
    raw: &str,
    format: &SubjectFormat,
    origin: &str,
) -> Result<String, AdcResolutionError> {
    let token = match format {
        SubjectFormat::Text => Some(raw.to_owned()),
        SubjectFormat::Json { field } => serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|json| {
                json.get(field)
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            }),
    };
    match token.filter(|token| !token.is_empty()) {
        Some(token) => Ok(token.to_owned()),
        None => Err(AdcResolutionError::SubjectTokenUnavailable {
            reason: format!(
                "unable to parse the subject_token from the credential_source {origin}"
            ),
        }),
    }
}

// ---------------------------------------------------------------------------
// URL supplier
// ---------------------------------------------------------------------------

/// GET the subject token from the configured URL (fixed Gaxios retry budget
/// for supplier legs, redirects terminal).
pub(crate) async fn url_subject_token(
    url: &str,
    headers: &[(String, String)],
    format: &SubjectFormat,
    transport: &dyn AdcSourceTransport,
) -> Result<String, AdcResolutionError> {
    let request = AdcSourceRequest {
        method: "GET".to_owned(),
        url: url.to_owned(),
        headers: headers.to_vec(),
        body: None,
        timeout: None,
    };
    // The identity-pool client passes its retry configuration to the URL
    // supplier: the fixed Gaxios default budget.
    static URL_BUDGET: GaxiosBudget = GaxiosBudget {
        extra_retries: 3,
        no_response: 2,
    };
    let (response, _sends) = send_with_gaxios_retry(transport, request, &URL_BUDGET).await?;
    if !is_success(response.status) {
        return Err(AdcResolutionError::SubjectTokenUnavailable {
            reason: format!("credential_source url returned status {}", response.status),
        });
    }
    let body = String::from_utf8_lossy(&response.body).into_owned();
    extract_subject_token(&body, format, "url")
}

// ---------------------------------------------------------------------------
// Certificate supplier
// ---------------------------------------------------------------------------

/// Well-known certificate configuration file name (gcloud layout).
const WELL_KNOWN_CERTIFICATE_CONFIG_FILE: &str = "certificate_config.json";

/// Resolved certificate source material: the JSON chain subject token plus
/// the leaf-first PEM identity for the source-scoped mTLS STS leg.
pub(crate) struct CertificateSubject {
    /// `JSON.stringify([base64(DER), ...])` subject token.
    pub subject_token: String,
    /// Leaf-first certificate chain PEM for the mTLS identity.
    pub chain_pem: Vec<u8>,
    /// Private key PEM (unencrypted).
    pub key_pem: Vec<u8>,
}

/// Resolve the certificate configuration path: explicit location →
/// `GOOGLE_API_CERTIFICATE_CONFIG` → the well-known gcloud path.
async fn resolve_certificate_config_path(
    location: Option<&str>,
    ambient: &dyn AuthContext,
) -> Result<PathBuf, AdcResolutionError> {
    if let Some(location) = location {
        let path = Path::new(location);
        return match tokio::fs::metadata(path).await {
            Ok(meta) if meta.is_file() => Ok(path.to_path_buf()),
            _ => Err(AdcResolutionError::CertificateSourceUnavailable {
                reason: "provided certificate config path is invalid",
            }),
        };
    }
    if let Some(env_path) = ambient.env("GOOGLE_API_CERTIFICATE_CONFIG").ok().flatten() {
        let path = Path::new(&env_path);
        return match tokio::fs::metadata(path).await {
            Ok(meta) if meta.is_file() => Ok(path.to_path_buf()),
            _ => Err(AdcResolutionError::CertificateSourceUnavailable {
                reason: "the GOOGLE_API_CERTIFICATE_CONFIG path is invalid",
            }),
        };
    }
    let config_dir = ambient.env("CLOUDSDK_CONFIG").ok().flatten().or_else(|| {
        if cfg!(windows) {
            ambient.env("APPDATA").ok().flatten()
        } else {
            ambient.env("HOME").ok().flatten().map(|home| {
                // The gcloud well-known layout on unix.
                format!("{home}/.config")
            })
        }
    });
    let Some(config_dir) = config_dir else {
        return Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "no certificate configuration location is configured",
        });
    };
    let well_known = Path::new(&config_dir)
        .join("gcloud")
        .join(WELL_KNOWN_CERTIFICATE_CONFIG_FILE);
    match tokio::fs::metadata(&well_known).await {
        Ok(meta) if meta.is_file() => Ok(well_known),
        _ => Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "no certificate configuration file was found",
        }),
    }
}

fn parse_certificate_ders(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, AdcResolutionError> {
    let ders: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AdcResolutionError::CertificateSourceUnavailable {
            reason: "a CERTIFICATE block could not be decoded",
        })?;
    if ders.is_empty() {
        return Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "no CERTIFICATE blocks were parsed",
        });
    }
    // Structural X.509 validation through the rustls parser (the same
    // validation the TLS stack applies), without admitting a new dependency.
    let mut store = RootCertStore::empty();
    for der in &ders {
        store
            .add(der.clone())
            .map_err(|_| AdcResolutionError::CertificateSourceUnavailable {
                reason: "a certificate could not be parsed",
            })?;
    }
    Ok(ders)
}

/// Order the leaf and trust chain into the subject-token DER list following
/// the source rules: leaf absent → prepended; leaf first → kept; leaf
/// present later → typed failure.
pub fn assemble_certificate_chain(
    leaf_der: &[u8],
    chain_pem: &[u8],
) -> Result<Vec<Vec<u8>>, AdcResolutionError> {
    if chain_pem.is_empty() {
        return Ok(vec![leaf_der.to_vec()]);
    }
    let chain = parse_certificate_ders(chain_pem)?;
    let leaf_index = chain.iter().position(|cert| cert.as_ref() == leaf_der);
    match leaf_index {
        None => {
            let mut ordered = vec![leaf_der.to_vec()];
            ordered.extend(chain.iter().map(|cert| cert.as_ref().to_vec()));
            Ok(ordered)
        }
        Some(0) => Ok(chain.iter().map(|cert| cert.as_ref().to_vec()).collect()),
        Some(_) => Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "the leaf certificate exists in the trust chain but is not the first entry",
        }),
    }
}

/// Resolve the certificate source: configuration file → leaf/key paths →
/// validated chain subject token plus the mTLS identity material.
pub(crate) async fn certificate_subject_token(
    location: Option<&str>,
    trust_chain_path: Option<&Path>,
    ambient: &dyn AuthContext,
) -> Result<CertificateSubject, AdcResolutionError> {
    let config_path = resolve_certificate_config_path(location, ambient).await?;
    let config_raw = tokio::fs::read_to_string(&config_path).await.map_err(|_| {
        AdcResolutionError::CertificateSourceUnavailable {
            reason: "failed to read the certificate config file",
        }
    })?;
    let config: serde_json::Value = serde_json::from_str(&config_raw).map_err(|_| {
        AdcResolutionError::CertificateSourceUnavailable {
            reason: "failed to parse the certificate config file",
        }
    })?;
    let workload = config.get("cert_configs").and_then(|v| v.get("workload"));
    let path_of = |field: &str| -> Option<String> {
        workload
            .and_then(|w| w.get(field))
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
    };
    let (Some(cert_path), Some(key_path)) = (path_of("cert_path"), path_of("key_path")) else {
        return Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "the certificate config is missing cert_path or key_path",
        });
    };

    let leaf_pem = tokio::fs::read(&cert_path).await.map_err(|_| {
        AdcResolutionError::CertificateSourceUnavailable {
            reason: "failed to read the certificate file",
        }
    })?;
    let key_pem = tokio::fs::read(&key_path).await.map_err(|_| {
        AdcResolutionError::CertificateSourceUnavailable {
            reason: "failed to read the private key file",
        }
    })?;

    let mut leaf_ders = parse_certificate_ders(&leaf_pem)?;
    if leaf_ders.len() != 1 {
        return Err(AdcResolutionError::CertificateSourceUnavailable {
            reason: "the certificate file must contain exactly one leaf certificate",
        });
    }
    let leaf = leaf_ders.remove(0);

    let chain_pem = match trust_chain_path {
        Some(path) => tokio::fs::read(path).await.map_err(|_| {
            AdcResolutionError::CertificateSourceUnavailable {
                reason: "failed to read the trust chain file",
            }
        })?,
        None => Vec::new(),
    };
    let ordered = assemble_certificate_chain(leaf.as_ref(), &chain_pem)?;

    use base64::Engine as _;
    let token_b64: Vec<String> = ordered
        .iter()
        .map(|der| base64::engine::general_purpose::STANDARD.encode(der))
        .collect();
    let subject_token = serde_json::to_string(&token_b64).expect("chain token serializes");

    // Identity chain PEM: leaf first, then any remaining chain entries
    // (re-encoded from the validated DERs so the identity matches the
    // subject-token order exactly).
    let mut identity_pem = Vec::new();
    for der in &ordered {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(der);
        identity_pem.extend_from_slice(b"-----BEGIN CERTIFICATE-----\n");
        for chunk in encoded.as_bytes().chunks(64) {
            identity_pem.extend_from_slice(chunk);
            identity_pem.extend_from_slice(b"\n");
        }
        identity_pem.extend_from_slice(b"-----END CERTIFICATE-----\n");
    }

    Ok(CertificateSubject {
        subject_token,
        chain_pem: identity_pem,
        key_pem,
    })
}

// ---------------------------------------------------------------------------
// AWS (aws1) supplier
// ---------------------------------------------------------------------------

/// AWS security credentials for one subject-token signing.
pub(crate) struct AwsSubjectCredentials {
    /// Access key identifier.
    pub access_key_id: SecretString,
    /// Secret access key.
    pub secret_access_key: SecretString,
    /// Optional session token.
    pub session_token: Option<SecretString>,
}

/// Send one single-attempt supplier GET/PUT (the source passes no retry
/// configuration to the default AWS supplier).
async fn single_send(
    transport: &dyn AdcSourceTransport,
    request: AdcSourceRequest,
) -> Result<AdcSourceResponse, AdcResolutionError> {
    static NO_RETRY: GaxiosBudget = GaxiosBudget {
        extra_retries: 0,
        no_response: 0,
    };
    let (response, _sends) = send_with_gaxios_retry(transport, request, &NO_RETRY).await?;
    Ok(response)
}

fn aws_unavailable(reason: &'static str) -> AdcResolutionError {
    AdcResolutionError::AwsSubjectUnavailable { reason }
}

/// `PUT` an IMDSv2 session token (fixed 300 s TTL) when configured.
async fn imds_v2_token(
    token_url: Option<&str>,
    transport: &dyn AdcSourceTransport,
) -> Result<Option<String>, AdcResolutionError> {
    let Some(url) = token_url else {
        return Ok(None);
    };
    let request = AdcSourceRequest {
        method: "PUT".to_owned(),
        url: url.to_owned(),
        headers: vec![(
            "x-aws-ec2-metadata-token-ttl-seconds".to_owned(),
            "300".to_owned(),
        )],
        body: None,
        timeout: None,
    };
    let response = single_send(transport, request).await?;
    if !is_success(response.status) {
        return Err(aws_unavailable("the IMDSv2 session token request failed"));
    }
    Ok(Some(String::from_utf8_lossy(&response.body).into_owned()))
}

fn metadata_headers(token: &Option<String>) -> Vec<(String, String)> {
    match token {
        Some(token) => vec![("x-aws-ec2-metadata-token".to_owned(), token.clone())],
        None => Vec::new(),
    }
}

/// Resolve the AWS region: `AWS_REGION` > `AWS_DEFAULT_REGION` >
/// `region_url` (availability-zone string with the final character
/// stripped).
pub(crate) async fn aws_region(
    region_url: Option<&str>,
    imdsv2_url: Option<&str>,
    ambient: &dyn AuthContext,
    transport: &dyn AdcSourceTransport,
) -> Result<String, AdcResolutionError> {
    if let Some(region) = ambient
        .env("AWS_REGION")
        .ok()
        .flatten()
        .or_else(|| ambient.env("AWS_DEFAULT_REGION").ok().flatten())
    {
        return Ok(region);
    }
    let token = imds_v2_token(imdsv2_url, transport).await?;
    let Some(url) = region_url else {
        return Err(aws_unavailable(
            "the AWS region is not in the environment and no region_url is configured",
        ));
    };
    let request = AdcSourceRequest {
        method: "GET".to_owned(),
        url: url.to_owned(),
        headers: metadata_headers(&token),
        body: None,
        timeout: None,
    };
    let response = single_send(transport, request).await?;
    if !is_success(response.status) {
        return Err(aws_unavailable("the region request failed"));
    }
    let zone = String::from_utf8_lossy(&response.body).into_owned();
    if zone.is_empty() {
        return Err(aws_unavailable("the region response was empty"));
    }
    // Strip the availability-zone suffix (`us-east-2b` → `us-east-2`).
    Ok(zone[..zone.len() - 1].to_owned())
}

/// Resolve AWS security credentials: environment variables first
/// (`AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY`, optional
/// `AWS_SESSION_TOKEN`), else the security-credentials endpoint (role list
/// then `{url}/{role}`). The role is never cached.
pub(crate) async fn aws_security_credentials(
    credentials_url: Option<&str>,
    imdsv2_url: Option<&str>,
    ambient: &dyn AuthContext,
    transport: &dyn AdcSourceTransport,
) -> Result<AwsSubjectCredentials, AdcResolutionError> {
    let env_id = ambient.env("AWS_ACCESS_KEY_ID").ok().flatten();
    let env_secret = ambient.env("AWS_SECRET_ACCESS_KEY").ok().flatten();
    if let (Some(access_key_id), Some(secret_access_key)) = (env_id, env_secret) {
        return Ok(AwsSubjectCredentials {
            access_key_id: SecretString::from(access_key_id),
            secret_access_key: SecretString::from(secret_access_key),
            session_token: ambient
                .env("AWS_SESSION_TOKEN")
                .ok()
                .flatten()
                .map(SecretString::from),
        });
    }
    let token = imds_v2_token(imdsv2_url, transport).await?;
    let headers = metadata_headers(&token);
    let Some(url) = credentials_url else {
        return Err(aws_unavailable(
            "AWS credentials are not in the environment and no credential_source url is configured",
        ));
    };
    let role_request = AdcSourceRequest {
        method: "GET".to_owned(),
        url: url.to_owned(),
        headers: headers.clone(),
        body: None,
        timeout: None,
    };
    let role_response = single_send(transport, role_request).await?;
    if !is_success(role_response.status) {
        return Err(aws_unavailable("the role list request failed"));
    }
    let role = String::from_utf8_lossy(&role_response.body).into_owned();
    if role.is_empty() {
        return Err(aws_unavailable("the role list response was empty"));
    }
    let creds_request = AdcSourceRequest {
        method: "GET".to_owned(),
        url: format!("{url}/{role}"),
        headers,
        body: None,
        timeout: None,
    };
    let creds_response = single_send(transport, creds_request).await?;
    if !is_success(creds_response.status) {
        return Err(aws_unavailable("the security credentials request failed"));
    }
    let json: serde_json::Value = serde_json::from_slice(&creds_response.body)
        .map_err(|_| aws_unavailable("the security credentials response was malformed"))?;
    let field = |name: &str| -> Option<String> {
        json.get(name)
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    };
    let (Some(access_key_id), Some(secret_access_key)) =
        (field("AccessKeyId"), field("SecretAccessKey"))
    else {
        return Err(aws_unavailable(
            "the security credentials response is missing AccessKeyId or SecretAccessKey",
        ));
    };
    Ok(AwsSubjectCredentials {
        access_key_id: SecretString::from(access_key_id),
        secret_access_key: SecretString::from(secret_access_key),
        session_token: field("Token").map(SecretString::from),
    })
}

/// JS `encodeURIComponent`: unreserved set plus `-_.!~*'()`.
pub fn js_encode_uri_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            );
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `YYYYMMDDTHHMMSSZ` from the clock (the source derives both date forms
/// from the same instant).
fn amz_timestamps(now: &time::OffsetDateTime) -> (String, String) {
    let (year, month, day, hour, minute, second) = (
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    );
    (
        format!("{year:04}{month:02}{day:02}"),
        format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z"),
    )
}

/// Sign the regional `GetCallerIdentity` request with SigV4 and serialize
/// the source-defined subject token
/// (`encodeURIComponent(JSON.stringify({url, method, headers}))`).
///
/// `x-goog-cloud-target-resource` is merged **after** signing (source
/// truth) and therefore is not part of `SignedHeaders`.
pub(crate) fn aws_subject_token(
    credentials: &AwsSubjectCredentials,
    region: &str,
    audience: &str,
    regional_url: &str,
    clock: &dyn AdcClock,
) -> Result<String, AdcResolutionError> {
    let url = regional_url.replace("{region}", region);
    let parsed =
        url::Url::parse(&url).map_err(|_| AdcResolutionError::CredentialSourceInvalid {
            reason: "regional_cred_verification_url is not a valid URL",
        })?;
    let host = parsed
        .host_str()
        .ok_or(AdcResolutionError::CredentialSourceInvalid {
            reason: "regional_cred_verification_url has no host",
        })?
        .to_owned();
    let path = parsed.path().to_owned();
    let query = parsed.query().unwrap_or_default().to_owned();
    let now = clock.now();
    let (date_stamp, amz_date) = amz_timestamps(&now);
    // The service name is the host's first label (`sts.{region}...` → sts).
    let service = host.split('.').next().unwrap_or_default().to_owned();

    let sigv4_credentials = AwsSigV4Credentials::new(
        credentials.access_key_id.clone(),
        credentials.secret_access_key.clone(),
        region.to_owned(),
    )
    .with_session_token(credentials.session_token.clone());
    let signed = sign_request(
        "POST",
        &path,
        &query,
        &[("host", host.as_str()), ("x-amz-date", amz_date.as_str())],
        b"",
        &sigv4_credentials,
        &service,
        &date_stamp,
        &amz_date,
    );

    // Serialized header order: the deterministic host / x-amz-date /
    // authorization / optional session token set, then the unsigned
    // target-resource header merged after signing. The STS endpoint
    // re-canonicalizes from the array, so only the set must match.
    let mut headers: Vec<(String, String)> = vec![
        ("host".to_owned(), host.clone()),
        ("x-amz-date".to_owned(), amz_date.clone()),
        (
            "authorization".to_owned(),
            signed.authorization.expose_secret().to_owned(),
        ),
    ];
    if let Some(token) = &credentials.session_token {
        headers.push((
            "x-amz-security-token".to_owned(),
            token.expose_secret().to_owned(),
        ));
    }
    headers.push((
        "x-goog-cloud-target-resource".to_owned(),
        audience.to_owned(),
    ));

    let serialized = serde_json::json!({
        "url": url,
        "method": "POST",
        "headers": headers
            .into_iter()
            .map(|(key, value)| serde_json::json!({"key": key, "value": value}))
            .collect::<Vec<_>>(),
    });
    Ok(js_encode_uri_component(&serialized.to_string()))
}
