//! Header validation and separation for mapped provider routes.

use std::collections::BTreeSet;
use std::str::FromStr;

use reqwest::header::{HeaderName, HeaderValue};

use crate::provider::ProviderError;

/// Canonical reserved provider-managed header names shared by every wire.
/// `ProviderHeaders::try_new` and `validate_extra_headers` both gate on this
/// single list so a future provider cannot accidentally inherit a narrower
/// gate.
pub(crate) const RESERVED_PROVIDER_HEADERS: &[&str] = &[
    "authorization",
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
];

/// Validated provider-configured headers kept separate from route-managed and
/// per-request headers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProviderHeaders {
    configured: Vec<(String, String)>,
}

impl ProviderHeaders {
    /// Validate headers supplied by provider configuration.
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

    /// Merge route-managed headers with validated per-request headers.
    ///
    /// Route headers are trusted production values but still pass HTTP
    /// name/value parsing. Request headers cannot use a provider-managed name
    /// or override a configured or route-managed header.
    pub fn merge_request(
        &self,
        route_headers: &[(String, String)],
        request_headers: &[(String, String)],
    ) -> Result<Vec<(String, String)>, ProviderError> {
        let mut occupied = BTreeSet::new();
        let mut merged =
            Vec::with_capacity(self.configured.len() + route_headers.len() + request_headers.len());
        for (name, value) in self.configured.iter().chain(route_headers) {
            validate_pair(name, value).map_err(provider_header_error)?;
            occupied.insert(name.to_ascii_lowercase());
            merged.push((name.clone(), value.clone()));
        }
        for (name, value) in request_headers {
            validate_pair(name, value).map_err(provider_header_error)?;
            let normalized = name.to_ascii_lowercase();
            if RESERVED_PROVIDER_HEADERS.contains(&normalized.as_str()) {
                return Err(ProviderError::RequestFailed(format!(
                    "request header '{name}' is reserved for provider-managed routing"
                )));
            }
            if !occupied.insert(normalized) {
                return Err(ProviderError::RequestFailed(format!(
                    "request header '{name}' cannot override a provider-managed header"
                )));
            }
            merged.push((name.clone(), value.clone()));
        }
        Ok(merged)
    }
}

fn validate_pair(name: &str, value: &str) -> Result<(), ProviderHeadersError> {
    if name.trim().is_empty() {
        return Err(ProviderHeadersError::InvalidName(name.into()));
    }
    HeaderName::from_str(name).map_err(|_| ProviderHeadersError::InvalidName(name.into()))?;
    HeaderValue::from_str(value)
        .map_err(|_| ProviderHeadersError::InvalidValue { name: name.into() })?;
    Ok(())
}

fn provider_header_error(error: ProviderHeadersError) -> ProviderError {
    ProviderError::RequestFailed(error.to_string())
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
