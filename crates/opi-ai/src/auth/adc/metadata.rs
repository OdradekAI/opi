//! Explicitly selected direct GCP metadata source (task 20.21.1).
//!
//! Pinned from `gcp-metadata@8.1.2` + `google-auth-library@10.6.2`
//! (`computeclient`): `GET {base}/computeMetadata/v1/instance/service-accounts/default/token`
//! with `Metadata-Flavor: Google` (both directions), `scopes` query
//! parameter when scopes are requested, and the fixed per-request timeout
//! `getGCPResidency() ? 0 : 3000ms` where `0` means no timeout.
//!
//! Residency is frozen by the caller
//! ([`AdcAuthSelection::DirectMetadata`](crate::auth::adc::native::AdcAuthSelection::DirectMetadata)'s
//! field): an explicit value, or an observation made only after this path was
//! selected. Models construction, availability checks, and credential
//! file-presence probes never select this source.
//!
//! Measured corpus divergences are explicit: the ~3000 ms timeout fails after
//! exactly one send (no budget is derived from the source's no-response
//! retry count), while status failures retry under the fixed Gaxios ranges.

use std::time::Duration;

use secrecy::SecretString;

use crate::auth::{AdcCredentialSource, AuthContext};

use super::native::{
    AdcClock, AdcResolutionError, AdcResolvedToken, AdcSourceRequest, AdcSourceTransport,
    GaxiosBudget, send_with_gaxios_retry,
};

/// Default metadata base (`HOST_ADDRESS` in the source; the secondary
/// DNS name is not a separate source here).
pub const METADATA_BASE_DEFAULT: &str = "http://169.254.169.254";

/// Instance token path under the metadata base.
pub const METADATA_TOKEN_PATH: &str = "/computeMetadata/v1/instance/service-accounts/default/token";

/// Fixed non-resident timeout (the source's `requestTimeout()`).
const NON_RESIDENT_TIMEOUT: Duration = Duration::from_millis(3000);

/// Metadata accessor budget: the source passes `noResponseRetries: 3` on top
/// of the fixed Gaxios defaults.
const METADATA_BUDGET: GaxiosBudget = GaxiosBudget {
    extra_retries: 3,
    no_response: 3,
};

/// Resolve one access token from the explicitly selected direct metadata
/// source. `residency` freezes the GCP-residency input for the per-request
/// timeout (`Some(true)`/resident → none, otherwise 3000 ms).
pub async fn resolve_direct_metadata_token(
    ambient: &dyn AuthContext,
    transport: &dyn AdcSourceTransport,
    clock: &dyn AdcClock,
    scopes: &[String],
    residency: Option<bool>,
) -> Result<AdcResolvedToken, AdcResolutionError> {
    let base = ambient
        .env("GCE_METADATA_IP")
        .ok()
        .flatten()
        .or_else(|| ambient.env("GCE_METADATA_HOST").ok().flatten())
        .unwrap_or_else(|| METADATA_BASE_DEFAULT.to_owned());
    // The source prefixes http:// when no scheme is present.
    let base = if base.starts_with("http://") || base.starts_with("https://") {
        base
    } else {
        format!("http://{base}")
    };
    let mut url = format!("{base}{METADATA_TOKEN_PATH}");
    if !scopes.is_empty() {
        let joined = scopes
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(",");
        url.push_str(&format!(
            "?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("scopes", &joined)
                .finish()
        ));
    }
    let request = AdcSourceRequest {
        method: "GET".to_owned(),
        url,
        headers: vec![("Metadata-Flavor".to_owned(), "Google".to_owned())],
        body: None,
        timeout: if residency.unwrap_or(false) {
            None
        } else {
            Some(NON_RESIDENT_TIMEOUT)
        },
    };
    let (response, sends) = send_with_gaxios_retry(transport, request, &METADATA_BUDGET).await?;
    if response.status != 200 {
        return Err(AdcResolutionError::ExchangeRejected {
            status: response.status,
            attempts: sends,
        });
    }
    // Wrong or absent flavor is the source's RangeError: typed failure.
    let flavor = response
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("metadata-flavor"))
        .map(|(_, value)| value.clone());
    if flavor.as_deref() != Some("Google") {
        return Err(AdcResolutionError::MetadataFlavor);
    }
    let json: serde_json::Value = serde_json::from_slice(&response.body)
        .map_err(|_| AdcResolutionError::InvalidTokenResponse)?;
    let access_token = json
        .get("access_token")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or(AdcResolutionError::InvalidTokenResponse)?
        .to_owned();
    let expires_at = json
        .get("expires_in")
        .and_then(|value| value.as_i64())
        .filter(|secs| *secs > 0)
        .map(|secs| clock.now() + time::Duration::seconds(secs));
    Ok(AdcResolvedToken {
        access_token: SecretString::from(access_token),
        expires_at,
        quota_project: None,
        rotated_refresh_token: None,
        source: AdcCredentialSource::DirectMetadata,
    })
}
