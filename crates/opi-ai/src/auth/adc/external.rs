//! ADC `external_account` family coordinator (task 20.21.2).
//!
//! Pinned from `google-auth-library@10.6.2`: the configuration requires
//! `audience` and `subject_token_type`; `token_url` defaults to
//! `https://sts.{universe_domain}/v1/token`; `client_id`(+`client_secret`)
//! selects HTTP Basic client authentication; `workforce_pool_user_project`
//! is only valid for workforce audiences and — without client
//! authentication — flows into the non-standard `options` form field as
//! `{"userProject":...}`. The credential source dispatch follows the
//! source: `environment_id` → aws, `executable` → pluggable, else the
//! identity-pool sources (file/url/certificate, mutually exclusive). The
//! RFC8693 token exchange runs under the fixed Gaxios retry budget; a
//! certificate source exchanges over its own mTLS-scoped transport (the
//! identity never leaves the STS leg). `service_account_impersonation_url`
//! is parsed and preserved only (composition lands with task 20.21.3).

use std::path::PathBuf;

use secrecy::{ExposeSecret, SecretString};

use crate::auth::AdcCredentialSource;
use crate::auth::ExternalAccountSupplier;
use crate::auth::adc::executable::retrieve_executable_subject_token;
use crate::auth::adc::native::{
    AdcNativeInput, AdcResolutionError, AdcResolvedToken, AdcSourceTransport, GaxiosBudget,
    ReqwestAdcTransport, checked_expiration, send_with_gaxios_retry,
};
use crate::auth::adc::subject::{
    CertificateSubject, SubjectFormat, aws_region, aws_security_credentials, aws_subject_token,
    certificate_subject_token, file_subject_token, url_subject_token,
};
use crate::tls::TlsOptions;

/// Default STS token endpoint template for `external_account` files.
const DEFAULT_STS_TOKEN_ENDPOINT: &str = "https://sts.{universe_domain}/v1/token";

/// Fixed Gaxios budget for the STS token exchange (the source's
/// `StsCredentials.RETRY_CONFIG` enables the default configuration).
use super::native::CLOUD_PLATFORM_SCOPE;

const STS_BUDGET: GaxiosBudget = GaxiosBudget {
    extra_retries: 3,
    no_response: 2,
};

/// Workforce audience pattern (locations/workforcePools/providers).
fn is_workforce_audience(audience: &str) -> bool {
    // `//iam.googleapis.com/locations/{loc}/workforcePools/{pool}/providers/{id}`
    // with a non-empty provider id (the source regex requires at least one
    // character after `providers/`).
    let rest = audience
        .strip_prefix("//iam.googleapis.com/locations/")
        .unwrap_or("");
    // `providers/.+`: the literal `providers` segment followed by a
    // non-empty provider id (further slashes are part of the id's path).
    let mut parts = rest.split('/');
    matches!(parts.next(), Some(loc) if !loc.is_empty())
        && matches!(parts.next(), Some(pool_kind) if pool_kind == "workforcePools")
        && matches!(parts.next(), Some(pool) if !pool.is_empty())
        && parts.next() == Some("providers")
        && matches!(parts.next(), Some(id) if !id.is_empty())
}

/// One parsed `external_account` credential source.
#[derive(Debug, Clone)]
pub(crate) enum ExternalCredentialSource {
    /// File-sourced subject token.
    File {
        /// Subject-token file path.
        file: PathBuf,
        /// Text or named-JSON-field format.
        format: SubjectFormat,
    },
    /// URL-sourced subject token.
    Url {
        /// Subject-token URL.
        url: String,
        /// Request headers.
        headers: Vec<(String, String)>,
        /// Text or named-JSON-field format.
        format: SubjectFormat,
    },
    /// Certificate-source subject token with mTLS identity.
    Certificate {
        /// `certificate_config_location` (None → default resolution; the
        /// `use_default_certificate_config` flag only selects that mode at
        /// parse time).
        location: Option<String>,
        /// Optional `trust_chain_path`.
        trust_chain_path: Option<PathBuf>,
    },
    /// AWS (aws1) subject credentials.
    Aws {
        /// `region_url`.
        region_url: Option<String>,
        /// security-credentials `url`.
        credentials_url: Option<String>,
        /// `imdsv2_session_token_url`.
        imdsv2_session_token_url: Option<String>,
        /// `regional_cred_verification_url` (required).
        regional_cred_verification_url: String,
    },
    /// Executable (pluggable) supplier.
    Executable {
        /// Command string (quote-aware direct argv).
        command: String,
        /// Configured timeout (5–120 s; default 30 s).
        timeout_millis: u64,
        /// Optional `output_file` cache.
        output_file: Option<String>,
    },
}

/// One parsed `external_account` credentials file.
#[derive(Clone)]
pub(crate) struct ExternalAccountFile {
    /// STS audience (full resource name).
    pub audience: String,
    /// Subject token type URN.
    pub subject_token_type: String,
    /// Resolved token exchange URL.
    pub token_url: String,
    /// Confidential client id.
    pub client_id: Option<String>,
    /// Confidential client secret.
    pub client_secret: Option<SecretString>,
    /// `workforce_pool_user_project` (workforce audiences only).
    pub workforce_pool_user_project: Option<String>,
    /// `service_account_impersonation_url` (validated: ≤ 256 chars with an
    /// extractable target principal). `None` skips the impersonation leg.
    pub service_account_impersonation_url: Option<String>,
    /// `service_account_impersonation.token_lifetime_seconds`
    /// (default 3600 when the URL is set and no lifetime is configured).
    pub impersonation_lifetime_secs: Option<u64>,
    /// Selected credential source.
    pub credential_source: ExternalCredentialSource,
}

fn config_invalid(reason: &'static str) -> AdcResolutionError {
    AdcResolutionError::CredentialSourceInvalid { reason }
}

fn parse_format(source: &serde_json::Value) -> Result<SubjectFormat, AdcResolutionError> {
    let Some(format) = source.get("format") else {
        return Ok(SubjectFormat::Text);
    };
    let format_type = format
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("text");
    match format_type {
        "text" => Ok(SubjectFormat::Text),
        "json" => {
            let field = format
                .get("subject_token_field_name")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .ok_or(config_invalid(
                    "a json credential_source format requires subject_token_field_name",
                ))?;
            Ok(SubjectFormat::Json {
                field: field.to_owned(),
            })
        }
        _ => Err(config_invalid(
            "the credential_source format type is not admitted",
        )),
    }
}

#[allow(clippy::type_complexity)]
fn parse_identity_pool_source(
    source: &serde_json::Value,
) -> Result<ExternalCredentialSource, AdcResolutionError> {
    let file = source
        .get("file")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    let url = source
        .get("url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty());
    let certificate = source.get("certificate");
    let selected = [file.is_some(), url.is_some(), certificate.is_some()]
        .iter()
        .filter(|flag| **flag)
        .count();
    if selected != 1 {
        return Err(config_invalid(
            "exactly one of credential_source file, url, or certificate must be provided",
        ));
    }
    let format = parse_format(source)?;
    if let Some(file) = file {
        return Ok(ExternalCredentialSource::File {
            file: PathBuf::from(file),
            format,
        });
    }
    if let Some(url) = url {
        let headers = source
            .get("headers")
            .and_then(|value| value.as_object())
            .map(|map| {
                map.iter()
                    .map(|(key, value)| {
                        (
                            key.to_ascii_lowercase(),
                            value.as_str().unwrap_or_default().to_owned(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return Ok(ExternalCredentialSource::Url {
            url: url.to_owned(),
            headers,
            format,
        });
    }
    let certificate = certificate.expect("exactly one source is selected");
    let use_default = certificate
        .get("use_default_certificate_config")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let location = certificate
        .get("certificate_config_location")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if use_default == location.is_some() {
        return Err(config_invalid(
            "exactly one of use_default_certificate_config or certificate_config_location must be provided",
        ));
    }
    let trust_chain_path = certificate
        .get("trust_chain_path")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    Ok(ExternalCredentialSource::Certificate {
        location,
        trust_chain_path,
    })
}

fn parse_aws_source(
    source: &serde_json::Value,
) -> Result<ExternalCredentialSource, AdcResolutionError> {
    let environment_id = source
        .get("environment_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(config_invalid(
            "the aws credential_source requires environment_id",
        ))?;
    if environment_id != "aws1" {
        return Err(config_invalid(
            "the environment_id aws version is not supported in the current build",
        ));
    }
    let regional = source
        .get("regional_cred_verification_url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(config_invalid(
            "the aws credential_source requires regional_cred_verification_url",
        ))?
        .to_owned();
    Ok(ExternalCredentialSource::Aws {
        region_url: source
            .get("region_url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        credentials_url: source
            .get("url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        imdsv2_session_token_url: source
            .get("imdsv2_session_token_url")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        regional_cred_verification_url: regional,
    })
}

fn parse_executable_source(
    source: &serde_json::Value,
) -> Result<ExternalCredentialSource, AdcResolutionError> {
    let executable = source
        .get("executable")
        .ok_or(config_invalid("a credential_source must be provided"))?;
    let command = executable
        .get("command")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(config_invalid(
            "the executable credential_source requires a command",
        ))?;
    let timeout_millis = match executable.get("timeout_millis") {
        None => crate::auth::adc::executable::DEFAULT_EXECUTABLE_TIMEOUT_MILLIS,
        Some(value) => {
            let timeout = value.as_u64().ok_or(config_invalid(
                "executable timeout_millis must be a non-negative integer",
            ))?;
            if !(crate::auth::adc::executable::MINIMUM_EXECUTABLE_TIMEOUT_MILLIS
                ..=crate::auth::adc::executable::MAXIMUM_EXECUTABLE_TIMEOUT_MILLIS)
                .contains(&timeout)
            {
                return Err(config_invalid(
                    "executable timeout_millis must be between 5000 and 120000",
                ));
            }
            timeout
        }
    };
    Ok(ExternalCredentialSource::Executable {
        command: command.to_owned(),
        timeout_millis,
        output_file: executable
            .get("output_file")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
    })
}

/// Parse one `external_account` credentials JSON object.
pub(crate) fn parse_external_account(
    json: &serde_json::Value,
) -> Result<ExternalAccountFile, AdcResolutionError> {
    fn required(
        json: &serde_json::Value,
        field: &'static str,
    ) -> Result<String, AdcResolutionError> {
        json.get(field)
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or(AdcResolutionError::MissingField {
                family: "external_account",
                field,
            })
    }
    let audience = required(json, "audience")?;
    let subject_token_type = required(json, "subject_token_type")?;

    let workforce_pool_user_project = json
        .get("workforce_pool_user_project")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if workforce_pool_user_project.is_some() && !is_workforce_audience(&audience) {
        return Err(AdcResolutionError::WorkforceProjectInvalid);
    }

    let universe_domain = json
        .get("universe_domain")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(crate::auth::adc::native::DEFAULT_UNIVERSE_DOMAIN);
    let token_url = match json
        .get("token_url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
    {
        Some(url) => {
            let parsed = url::Url::parse(url)
                .ok()
                .filter(|url| matches!(url.scheme(), "http" | "https"))
                .ok_or(AdcResolutionError::InvalidTokenUrl)?;
            parsed.to_string()
        }
        None => DEFAULT_STS_TOKEN_ENDPOINT.replace("{universe_domain}", universe_domain),
    };

    let client_id = json
        .get("client_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let client_secret = match &client_id {
        Some(_) => {
            let secret = json
                .get("client_secret")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .ok_or(AdcResolutionError::MissingField {
                    family: "external_account",
                    field: "client_secret",
                })?;
            Some(SecretString::from(secret))
        }
        None => json
            .get("client_secret")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(SecretString::from),
    };

    let source = json
        .get("credential_source")
        .ok_or(config_invalid("a credential_source must be provided"))?;
    let credential_source = if source.get("environment_id").is_some() {
        parse_aws_source(source)?
    } else if source.get("executable").is_some() {
        parse_executable_source(source)?
    } else {
        parse_identity_pool_source(source)?
    };

    let service_account_impersonation_url = json
        .get("service_account_impersonation_url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(url) = &service_account_impersonation_url {
        // The corpus bounds the URL (DOS guard) and requires an extractable
        // target principal; admission happens before any network leg.
        if url.len() > 256
            || crate::auth::adc::impersonation::target_principal_from_url(url).is_none()
        {
            return Err(AdcResolutionError::ImpersonationTargetInvalid);
        }
    }
    let impersonation_lifetime_secs = json
        .get("service_account_impersonation")
        .and_then(|value| value.get("token_lifetime_seconds"))
        .and_then(|value| value.as_u64())
        .filter(|secs| *secs > 0);

    Ok(ExternalAccountFile {
        audience,
        subject_token_type,
        token_url,
        client_id,
        client_secret,
        workforce_pool_user_project,
        service_account_impersonation_url,
        impersonation_lifetime_secs,
        credential_source,
    })
}

/// The supplier classification carried into provenance.
pub(crate) fn supplier_of(source: &ExternalCredentialSource) -> ExternalAccountSupplier {
    match source {
        ExternalCredentialSource::File { .. } => ExternalAccountSupplier::File,
        ExternalCredentialSource::Url { .. } => ExternalAccountSupplier::Url,
        ExternalCredentialSource::Certificate { .. } => ExternalAccountSupplier::Certificate,
        ExternalCredentialSource::Aws { .. } => ExternalAccountSupplier::Aws,
        ExternalCredentialSource::Executable { .. } => ExternalAccountSupplier::Executable,
    }
}

fn urlencoded_form(pairs: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in pairs {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

/// Exchange one subject token for a GCP access token over RFC8693.
struct StsClientAuth<'a> {
    client_id: Option<&'a str>,
    client_secret: Option<&'a SecretString>,
    workforce_user_project: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
async fn exchange_sts(
    token_url: &str,
    audience: &str,
    subject_token: &str,
    subject_token_type: &str,
    scopes: &[String],
    auth: &StsClientAuth<'_>,
    transport: &dyn AdcSourceTransport,
) -> Result<(String, Option<i64>), AdcResolutionError> {
    let scope = scopes
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let mut pairs: Vec<(&str, String)> = vec![
        (
            "grant_type",
            "urn:ietf:params:oauth:grant-type:token-exchange".to_owned(),
        ),
        ("audience", audience.to_owned()),
        (
            "requested_token_type",
            "urn:ietf:params:oauth:token-type:access_token".to_owned(),
        ),
        ("subject_token", subject_token.to_owned()),
        ("subject_token_type", subject_token_type.to_owned()),
        ("scope", scope),
    ];
    let mut headers: Vec<(String, String)> = Vec::new();
    if let (Some(id), Some(secret)) = (auth.client_id, auth.client_secret) {
        use base64::Engine as _;
        let basic = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", id, secret.expose_secret()).as_bytes());
        headers.push(("authorization".to_owned(), format!("Basic {basic}")));
    } else if let Some(user_project) = auth.workforce_user_project {
        // Client authentication is prioritized over the userProject option.
        pairs.push((
            "options",
            serde_json::json!({ "userProject": user_project }).to_string(),
        ));
    }
    headers.push((
        "content-type".to_owned(),
        "application/x-www-form-urlencoded".to_owned(),
    ));
    let request = crate::auth::adc::native::AdcSourceRequest {
        method: "POST".to_owned(),
        url: token_url.to_owned(),
        headers,
        body: Some(urlencoded_form(&pairs)),
        timeout: None,
    };
    let (response, sends) = send_with_gaxios_retry(transport, request, &STS_BUDGET).await?;
    if response.status != 200 {
        return Err(AdcResolutionError::ExchangeRejected {
            status: response.status,
            attempts: sends,
        });
    }
    let json: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| AdcResolutionError::InvalidTokenResponse)?;
    let access_token = json
        .get("access_token")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(AdcResolutionError::InvalidTokenResponse)?
        .to_owned();
    let expires_in = json
        .get("expires_in")
        .and_then(|value| value.as_i64())
        .filter(|secs| *secs > 0);
    Ok((access_token, expires_in))
}

/// Resolve one `external_account` credential end to end: subject token →
/// STS exchange → resolved Vertex token.
pub(crate) async fn resolve_external_account(
    file: &ExternalAccountFile,
    input: &AdcNativeInput<'_>,
    scopes: &[String],
) -> Result<AdcResolvedToken, AdcResolutionError> {
    let now = input.clock.now();
    let supplier = supplier_of(&file.credential_source);

    let (subject_token, certificate): (String, Option<CertificateSubject>) =
        match &file.credential_source {
            ExternalCredentialSource::File { file, format } => {
                (file_subject_token(file, format).await?, None)
            }
            ExternalCredentialSource::Url {
                url,
                headers,
                format,
            } => (
                url_subject_token(url, headers, format, input.transport).await?,
                None,
            ),
            ExternalCredentialSource::Certificate {
                location,
                trust_chain_path,
                ..
            } => {
                let certificate = certificate_subject_token(
                    location.as_deref(),
                    trust_chain_path.as_deref(),
                    input.ambient,
                )
                .await?;
                (certificate.subject_token.clone(), Some(certificate))
            }
            ExternalCredentialSource::Aws {
                region_url,
                credentials_url,
                imdsv2_session_token_url,
                regional_cred_verification_url,
            } => {
                let region = aws_region(
                    region_url.as_deref(),
                    imdsv2_session_token_url.as_deref(),
                    input.ambient,
                    input.transport,
                )
                .await?;
                let credentials = aws_security_credentials(
                    credentials_url.as_deref(),
                    imdsv2_session_token_url.as_deref(),
                    input.ambient,
                    input.transport,
                )
                .await?;
                let token = aws_subject_token(
                    &credentials,
                    &region,
                    &file.audience,
                    regional_cred_verification_url,
                    input.clock,
                )?;
                (token, None)
            }
            ExternalCredentialSource::Executable {
                command,
                timeout_millis,
                output_file,
            } => {
                let impersonated_email = file
                    .service_account_impersonation_url
                    .as_deref()
                    .and_then(crate::auth::adc::executable::impersonated_email_from_url);
                let token = retrieve_executable_subject_token(
                    command,
                    *timeout_millis,
                    output_file.as_deref(),
                    &file.audience,
                    &file.subject_token_type,
                    impersonated_email.as_deref(),
                    input.ambient,
                    input.clock,
                )
                .await?;
                (token, None)
            }
        };

    // The canonical-endpoint override (tests and pinned deployments) wins
    // over the file-derived URL for every external source.
    let token_url = input.token_endpoint.unwrap_or(file.token_url.as_str());
    // With optional IAM impersonation the STS exchange runs the forced
    // cloud-platform scope the corpus pins (generateAccessToken requires
    // iam or cloud-platform on the federated token).
    let sts_scopes: Vec<String> = if file.service_account_impersonation_url.is_some() {
        vec![CLOUD_PLATFORM_SCOPE.to_owned()]
    } else {
        scopes.to_vec()
    };
    let (access_token, expires_in) = match certificate {
        None => {
            exchange_sts(
                token_url,
                &file.audience,
                &subject_token,
                &file.subject_token_type,
                &sts_scopes,
                &StsClientAuth {
                    client_id: file.client_id.as_deref(),
                    client_secret: file.client_secret.as_ref(),
                    workforce_user_project: file.workforce_pool_user_project.as_deref(),
                },
                input.transport,
            )
            .await?
        }
        Some(certificate) => {
            // Source-scoped mTLS: the certificate identity rides only the
            // STS leg through a transport built for this exchange.
            let mut options = TlsOptions::new();
            if let Some(extra_roots) = input.certificate_extra_roots_pem {
                options = options.with_extra_roots_pem(extra_roots).map_err(|_| {
                    AdcResolutionError::CertificateSourceUnavailable {
                        reason: "the certificate transport trust roots were rejected",
                    }
                })?;
            }
            // The key and chain admission follow the frozen TLS profile
            // (rejected keys fail typed before any send).
            let options = options
                .with_client_identity_pem(&certificate.chain_pem, &certificate.key_pem)
                .map_err(|_| AdcResolutionError::KeyRejected)?;
            let scoped = ReqwestAdcTransport::with_tls(options).map_err(|_| {
                AdcResolutionError::CertificateSourceUnavailable {
                    reason: "the certificate transport could not be built",
                }
            })?;
            exchange_sts(
                token_url,
                &file.audience,
                &subject_token,
                &file.subject_token_type,
                &sts_scopes,
                &StsClientAuth {
                    client_id: file.client_id.as_deref(),
                    client_secret: file.client_secret.as_ref(),
                    workforce_user_project: file.workforce_pool_user_project.as_deref(),
                },
                &scoped,
            )
            .await?
        }
    };

    let federation_source = AdcCredentialSource::ExternalAccount {
        supplier,
        workforce_user_project: file.workforce_pool_user_project.clone(),
    };
    let Some(impersonation_url) = file.service_account_impersonation_url.as_deref() else {
        return Ok(AdcResolvedToken {
            access_token: SecretString::from(access_token),
            expires_at: expires_in
                .map(|secs| checked_expiration(now, secs))
                .transpose()?,
            quota_project: None,
            rotated_refresh_token: None,
            source: federation_source,
        });
    };

    // Optional IAM impersonation: the federated token exchanges for a
    // service-account token at the configured URL (validated at parse).
    // The external path sends no delegates (the corpus body is scope +
    // lifetime only) and the requested scopes — defaulting to
    // cloud-platform — become the impersonated token's scopes.
    let target_principal =
        crate::auth::adc::impersonation::target_principal_from_url(impersonation_url)
            .expect("validated at parse time")
            .to_owned();
    let url = input
        .impersonation_endpoint
        .map(str::to_owned)
        .unwrap_or_else(|| impersonation_url.to_owned());
    let lifetime = file.impersonation_lifetime_secs.unwrap_or(3600);
    let body = serde_json::json!({
        "scope": scopes,
        "lifetime": format!("{lifetime}s"),
    });
    let request = crate::auth::adc::native::AdcSourceRequest {
        method: "POST".to_owned(),
        url,
        headers: vec![
            ("authorization".to_owned(), format!("Bearer {access_token}")),
            ("content-type".to_owned(), "application/json".to_owned()),
        ],
        body: Some(body.to_string()),
        timeout: None,
    };
    let (response, sends) = send_with_gaxios_retry(input.transport, request, &STS_BUDGET).await?;
    if response.status != 200 {
        return Err(AdcResolutionError::ExchangeRejected {
            status: response.status,
            attempts: sends,
        });
    }
    let json: serde_json::Value = serde_json::from_slice(&response.body).map_err(|_| {
        AdcResolutionError::ImpersonationResponseInvalid {
            reason: "the impersonation response is not valid JSON",
        }
    })?;
    let impersonated_token = json
        .get("accessToken")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(AdcResolutionError::ImpersonationResponseInvalid {
            reason: "the impersonation response carries no accessToken",
        })?;
    let expire_time = json
        .get("expireTime")
        .and_then(|value| value.as_str())
        .ok_or(AdcResolutionError::ImpersonationResponseInvalid {
            reason: "the impersonation response carries no expireTime",
        })?;
    let expires_at =
        time::OffsetDateTime::parse(expire_time, &time::format_description::well_known::Rfc3339)
            .map_err(|_| AdcResolutionError::ImpersonationResponseInvalid {
                reason: "the impersonation expireTime is not valid RFC3339",
            })?;

    Ok(AdcResolvedToken {
        access_token: SecretString::from(impersonated_token),
        expires_at: Some(expires_at),
        quota_project: None,
        rotated_refresh_token: None,
        source: AdcCredentialSource::Impersonated {
            source: Box::new(federation_source),
            target_principal,
            delegates: 0,
            quota_project: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::is_workforce_audience;

    #[test]
    fn workforce_audience_pattern_is_exact() {
        assert!(is_workforce_audience(
            "//iam.googleapis.com/locations/us/workforcePools/wp/providers/oidc"
        ));
        assert!(is_workforce_audience(
            "//iam.googleapis.com/locations/global/workforcePools/p/providers/saml"
        ));
        assert!(!is_workforce_audience(
            "//iam.googleapis.com/projects/123/locations/global/workloadIdentityPools/p/providers/o"
        ));
        assert!(!is_workforce_audience(
            "//iam.googleapis.com/locations/us/workforcePools/wp/providers/"
        ));
    }
}
