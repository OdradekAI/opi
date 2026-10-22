//! ADC `impersonated_service_account` files and the IAM
//! `generateAccessToken` exchange (task 20.21.3).
//!
//! Pinned to `google-auth-library@10.6.2` (`googleauth.fromImpersonatedJSON`
//! and `auth/impersonated.js`): the family requires `source_credentials`
//! (any other family, resolved recursively through the same adapter with no
//! new depth cap) and a `service_account_impersonation_url` (≤ 256 chars,
//! target principal extracted as the final non-empty path segment before
//! `:generateAccessToken`/`:generateIdToken`). The file's raw URL is never
//! the wire destination: the request is reconstructed from the effective
//! universe domain as
//! `https://iamcredentials.{universe}/v1/projects/-/serviceAccounts/{target}:generateAccessToken`
//! (an explicit `universe_domain` that disagrees with the source's is a
//! typed construction failure, the source's corpus `RangeError`). The body
//! carries `delegates`, the target `scope` array (the requested scopes, or
//! the file's `scopes` when only the default cloud-platform scope was
//! requested), and `lifetime` as `"{seconds}s"` (default 3600); the source
//! access token rides the leg as the Bearer credential, and the response's
//! `expireTime` is RFC3339. Source and target identities stay separate in
//! the resolved provenance.

use secrecy::{ExposeSecret, SecretString};

use super::native::{
    AdcNativeInput, AdcResolutionError, AdcResolvedToken, AdcSourceRequest, CLOUD_PLATFORM_SCOPE,
    DEFAULT_UNIVERSE_DOMAIN, TOKEN_BUDGET, send_with_gaxios_retry,
};
use crate::auth::AdcCredentialSource;

/// One parsed `impersonated_service_account` credentials file.
#[derive(Clone)]
pub(crate) struct ImpersonatedFile {
    /// Raw `source_credentials` JSON (recursively parsed by the native
    /// dispatcher).
    pub source_json: serde_json::Value,
    /// The source file's own `universe_domain`, when set.
    pub source_universe: Option<String>,
    /// Raw `service_account_impersonation_url` (target extraction only; it
    /// never becomes the wire destination).
    #[allow(dead_code)]
    pub impersonation_url: String,
    /// Target service-account email extracted from the URL.
    pub target_principal: String,
    /// Delegation chain (non-empty strings).
    pub delegates: Vec<String>,
    /// Requested token lifetime in seconds (default 3600).
    pub lifetime_secs: u64,
    /// Non-secret `quota_project_id` from the impersonated file.
    pub quota_project: Option<String>,
    /// Explicit `universe_domain` on the impersonated file.
    pub universe_domain: Option<String>,
    /// File-level `scopes` fallback for the target scope array.
    pub scopes: Option<Vec<String>>,
}

/// Extract the target principal exactly like the source regex
/// `(?<target>[^/]+):(generateAccessToken|generateIdToken)$`: the final
/// non-empty path segment before the method suffix.
pub(crate) fn target_principal_from_url(url: &str) -> Option<&str> {
    for suffix in [":generateAccessToken", ":generateIdToken"] {
        if let Some(stripped) = url.strip_suffix(suffix) {
            let segment = stripped.rsplit('/').next().unwrap_or("");
            return (!segment.is_empty()).then_some(segment);
        }
    }
    None
}

fn config_invalid(reason: &'static str) -> AdcResolutionError {
    AdcResolutionError::ImpersonationConfigInvalid { reason }
}

fn optional_string_field(json: &serde_json::Value, field: &str) -> Option<String> {
    json.get(field)
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Parse one `impersonated_service_account` file. Structural validation
/// (including the universe agreement with the embedded source) happens
/// before any network leg.
pub(crate) fn parse_impersonated(
    json: &serde_json::Value,
) -> Result<ImpersonatedFile, AdcResolutionError> {
    const FAMILY: &str = "impersonated_service_account";
    let missing = |field: &'static str| AdcResolutionError::MissingField {
        family: FAMILY,
        field,
    };

    let source = json
        .get("source_credentials")
        .ok_or(missing("source_credentials"))?;
    if !source.is_object() {
        return Err(config_invalid("source_credentials must be a JSON object"));
    }
    let impersonation_url = json
        .get("service_account_impersonation_url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(missing("service_account_impersonation_url"))?
        .to_owned();
    if impersonation_url.len() > 256 {
        return Err(AdcResolutionError::ImpersonationTargetInvalid);
    }
    let target_principal = target_principal_from_url(&impersonation_url)
        .ok_or(AdcResolutionError::ImpersonationTargetInvalid)?
        .to_owned();

    let delegates = match json.get("delegates") {
        None | Some(serde_json::Value::Null) => Vec::new(),
        Some(serde_json::Value::Array(items)) => {
            let mut parsed = Vec::with_capacity(items.len());
            for item in items {
                let delegate = item
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or(config_invalid("delegates must be non-empty strings"))?;
                parsed.push(delegate.to_owned());
            }
            parsed
        }
        Some(_) => {
            return Err(config_invalid(
                "delegates must be an array of non-empty strings",
            ));
        }
    };
    let lifetime_secs = match json.get("lifetime") {
        None | Some(serde_json::Value::Null) => 3600,
        Some(value) => value
            .as_u64()
            .filter(|secs| *secs > 0)
            .ok_or(config_invalid(
                "lifetime must be a positive number of seconds",
            ))?,
    };
    let scopes = match json.get("scopes") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::Array(items)) => {
            let mut parsed = Vec::with_capacity(items.len());
            for item in items {
                let scope = item
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or(config_invalid("scopes must be non-empty strings"))?;
                parsed.push(scope.to_owned());
            }
            Some(parsed)
        }
        Some(_) => {
            return Err(config_invalid(
                "scopes must be an array of non-empty strings",
            ));
        }
    };

    let universe_domain = optional_string_field(json, "universe_domain");
    let source_universe = optional_string_field(source, "universe_domain");
    // The source's universe domain wins unless the file carries an explicit
    // one; an explicit domain that disagrees with the source's is the
    // source's typed construction failure (credential-leak guard).
    if let (Some(explicit), source_value) = (
        universe_domain.as_deref(),
        source_universe
            .as_deref()
            .unwrap_or(DEFAULT_UNIVERSE_DOMAIN),
    ) && explicit != source_value
    {
        return Err(AdcResolutionError::ImpersonationUniverseMismatch);
    }

    Ok(ImpersonatedFile {
        source_json: source.clone(),
        source_universe,
        impersonation_url,
        target_principal,
        delegates,
        lifetime_secs,
        quota_project: optional_string_field(json, "quota_project_id"),
        universe_domain,
        scopes,
    })
}

/// Effective universe domain for the reconstructed endpoint.
fn effective_universe(file: &ImpersonatedFile, input: &AdcNativeInput<'_>) -> String {
    file.universe_domain
        .clone()
        .or_else(|| file.source_universe.clone())
        .or_else(|| input.universe_domain.map(str::to_owned))
        .unwrap_or_else(|| DEFAULT_UNIVERSE_DOMAIN.to_owned())
}

/// The target scope array: requested scopes win; the file's `scopes`
/// substitute only when the request carried the default cloud-platform
/// scope alone (the source's `this.scopes || json.scopes || default`
/// precedence with the SDK's default request).
fn target_scopes(file: &ImpersonatedFile, scopes: &[String]) -> Vec<String> {
    match &file.scopes {
        Some(file_scopes) if scopes.len() == 1 && scopes[0] == CLOUD_PLATFORM_SCOPE => {
            file_scopes.clone()
        }
        _ => scopes.to_vec(),
    }
}

/// Resolve one impersonated file: recursive source token through the same
/// adapter, then the IAM `generateAccessToken` exchange.
pub(crate) async fn resolve_impersonated(
    file: &ImpersonatedFile,
    input: &AdcNativeInput<'_>,
    scopes: &[String],
    refresh_override: Option<&SecretString>,
) -> Result<AdcResolvedToken, AdcResolutionError> {
    let source_token = super::native::exchange_family_for_value(
        &file.source_json,
        refresh_override,
        input,
        scopes,
    )
    .await?;

    let universe = effective_universe(file, input);
    let url = input
        .impersonation_endpoint
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "https://iamcredentials.{universe}/v1/projects/-/serviceAccounts/{}:generateAccessToken",
                file.target_principal
            )
        });
    let body = serde_json::json!({
        "delegates": file.delegates,
        "scope": target_scopes(file, scopes),
        "lifetime": format!("{}s", file.lifetime_secs),
    });
    let request = AdcSourceRequest {
        method: "POST".to_owned(),
        url,
        headers: vec![
            (
                "authorization".to_owned(),
                format!("Bearer {}", source_token.access_token.expose_secret()),
            ),
            ("content-type".to_owned(), "application/json".to_owned()),
        ],
        body: Some(body.to_string()),
        timeout: None,
    };
    let (response, sends) = send_with_gaxios_retry(input.transport, request, &TOKEN_BUDGET).await?;
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
    let access_token = json
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
        access_token: SecretString::from(access_token),
        expires_at: Some(expires_at),
        quota_project: file.quota_project.clone(),
        // A source rotation (embedded `external_account_authorized_user`)
        // propagates to the owning cache exactly like a direct family.
        rotated_refresh_token: source_token.rotated_refresh_token.clone(),
        source: AdcCredentialSource::Impersonated {
            source: Box::new(source_token.source),
            target_principal: file.target_principal.clone(),
            delegates: file.delegates.len(),
            quota_project: file.quota_project.clone(),
        },
    })
}
