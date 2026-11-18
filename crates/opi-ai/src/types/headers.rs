//! Pure HTTP header syntax, reserved-name rules and provider header config.
//! No client, provider wire or I/O dependency.

use http::header::{HeaderName, HeaderValue};
use std::collections::BTreeSet;
use std::str::FromStr;

/// Canonical reserved provider-managed header names shared by every wire.
/// [`crate::api::shared::headers::ProviderHeaders::try_new`] and
/// `validate_extra_headers` both gate on this
/// single list so a future provider cannot accidentally inherit a narrower
/// gate.
pub(crate) const RESERVED_PROVIDER_HEADERS: &[&str] = &[
    "host",
    "authorization",
    "cf-aig-authorization",
    "x-api-key",
    "api-key",
    "anthropic-version",
    "anthropic-beta",
    "content-type",
    "chatgpt-account-id",
    "openai-beta",
    "session-id",
    "session_id",
    "x-client-request-id",
    "x-session-affinity",
    "x-initiator",
    "x-goog-api-key",
    "x-goog-user-project",
];

pub(crate) fn is_protected_header(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    RESERVED_PROVIDER_HEADERS.contains(&name.as_str())
        || name.starts_with("amz-sdk-")
        || name.starts_with("x-amz-")
}

pub(crate) fn validate_model_headers(
    headers: &[(String, Option<String>)],
) -> Result<(), ProviderHeadersError> {
    for (name, value) in headers {
        validate_pair(name, value.as_deref().unwrap_or(""))?;
        if is_protected_header(name) {
            return Err(ProviderHeadersError::ReservedName(name.clone()));
        }
    }
    Ok(())
}

pub(crate) fn validate_pair(name: &str, value: &str) -> Result<(), ProviderHeadersError> {
    if name.trim().is_empty() {
        return Err(ProviderHeadersError::InvalidName(name.into()));
    }
    HeaderName::from_str(name).map_err(|_| ProviderHeadersError::InvalidName(name.into()))?;
    HeaderValue::from_str(value)
        .map_err(|_| ProviderHeadersError::InvalidValue { name: name.into() })?;
    Ok(())
}

/// Invalid provider-configured HTTP header.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProviderHeadersError {
    #[error("invalid provider header name {0:?}")]
    InvalidName(String),
    #[error("invalid value for provider header '{name}'")]
    InvalidValue { name: String },
    #[error("provider header '{0}' is reserved for route-managed behavior")]
    ReservedName(String),
    #[error("duplicate provider header '{0}'")]
    DuplicateName(String),
}

/// Validated provider-configured HTTP headers.
///
/// This value belongs to the profile/configuration side of the seam. Wire
/// adapters add route-managed and per-request headers through their own merge
/// policy in `api::shared::headers`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderHeaders {
    configured: Vec<(String, String)>,
}

impl ProviderHeaders {
    /// Validate provider-configured header values and names.
    pub fn try_new(headers: Vec<(String, String)>) -> Result<Self, ProviderHeadersError> {
        let mut names = BTreeSet::new();
        for (name, value) in &headers {
            validate_pair(name, value)?;
            let normalized = name.to_ascii_lowercase();
            if RESERVED_PROVIDER_HEADERS.contains(&normalized.as_str()) {
                return Err(ProviderHeadersError::ReservedName(name.clone()));
            }
            if !names.insert(normalized) {
                return Err(ProviderHeadersError::DuplicateName(name.clone()));
            }
        }
        Ok(Self {
            configured: headers,
        })
    }

    /// Return the validated provider-configured header values.
    pub fn configured(&self) -> &[(String, String)] {
        &self.configured
    }
}
