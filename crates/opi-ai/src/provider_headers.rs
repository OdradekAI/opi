//! Header validation and separation for mapped provider routes.

use std::collections::BTreeSet;
use std::str::FromStr;

use reqwest::header::{HeaderName, HeaderValue};

use crate::provider::{ProviderError, ProviderErrorSummary};

/// Canonical reserved provider-managed header names shared by every wire.
/// `ProviderHeaders::try_new` and `validate_extra_headers` both gate on this
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

    /// Merge configured defaults, selected-model overrides and per-call headers.
    /// Model `None` removes an earlier ordinary header. Route-managed identity
    /// and authentication headers cannot be changed or removed by either input.
    pub(crate) fn merge_model_request(
        &self,
        model_headers: &[(String, Option<String>)],
        route_headers: &[(String, String)],
        request_headers: &[(String, String)],
    ) -> Result<Vec<(String, String)>, ProviderError> {
        validate_model_headers(model_headers).map_err(provider_header_error)?;
        crate::provider::validate_extra_headers(request_headers)?;
        for (name, value) in route_headers {
            validate_pair(name, value).map_err(provider_header_error)?;
        }
        let defaults: Vec<_> = self
            .configured
            .iter()
            .chain(route_headers)
            .map(|(name, value)| (name.clone(), Some(value.clone())))
            .collect();
        let request: Vec<_> = request_headers
            .iter()
            .map(|(name, value)| (name.clone(), Some(value.clone())))
            .collect();
        Ok(
            merge_provider_header_sources(&[&defaults, model_headers, &request])
                .unwrap_or_default(),
        )
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
                return Err(ProviderError::RequestFailed(
                    ProviderErrorSummary::sanitized(format!(
                        "request header '{name}' is reserved for provider-managed routing"
                    )),
                ));
            }
            if !occupied.insert(normalized) {
                return Err(ProviderError::RequestFailed(
                    ProviderErrorSummary::sanitized(format!(
                        "request header '{name}' cannot override a provider-managed header"
                    )),
                ));
            }
            merged.push((name.clone(), value.clone()));
        }
        Ok(merged)
    }
}

/// Merge provider header override sources case-insensitively (pinned
/// `providerHeadersToRecord`; task 20.53.6).
///
/// Later sources win for the same name (case-insensitively); a winning
/// override re-inserts at the end of the output, preserving the winner's
/// original case, and a [`None`] value deletes the name without inserting.
/// Yields [`None`] when nothing survives. The output is unvalidated config
/// normalization: feed it to [`ProviderHeaders::try_new`] for wire
/// validation; this helper never bypasses [`ProviderHeaders::merge_request`]
/// fail-closed override rejection.
pub fn merge_provider_header_sources(
    sources: &[&[(String, Option<String>)]],
) -> Option<Vec<(String, String)>> {
    let mut merged: Vec<(String, (String, String))> = Vec::new();
    for source in sources {
        for (name, value) in *source {
            let normalized = name.to_ascii_lowercase();
            merged.retain(|(existing, _)| *existing != normalized);
            if let Some(value) = value {
                merged.push((normalized, (name.clone(), value.clone())));
            }
        }
    }
    (!merged.is_empty()).then(|| merged.into_iter().map(|(_, pair)| pair).collect())
}

/// Validate a header-transform output (task 20.51): valid names/values, no
/// duplicates, and unchanged protected identity/auth/signing values. A
/// protected name absent from the assembled input cannot be introduced.
pub(crate) fn validate_transform_output(
    headers: &[(String, String)],
    original: &[(String, String)],
) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for (name, value) in headers {
        if let Err(error) = validate_pair(name, value) {
            return Err(error.to_string());
        }
        let normalized = name.to_ascii_lowercase();
        if is_protected_header(&normalized)
            && !original.iter().any(|(prior_name, prior_value)| {
                prior_name.eq_ignore_ascii_case(name) && prior_value == value
            })
        {
            return Err(format!(
                "transform output touches the protected header '{name}'"
            ));
        }
        if !names.insert(normalized) {
            return Err(format!("transform output duplicates the header '{name}'"));
        }
    }
    for (name, value) in original {
        if is_protected_header(name)
            && !headers.iter().any(|(next_name, next_value)| {
                next_name.eq_ignore_ascii_case(name) && next_value == value
            })
        {
            return Err(format!(
                "transform output removes protected header '{name}'"
            ));
        }
    }
    Ok(())
}

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
    ProviderError::RequestFailed(ProviderErrorSummary::sanitized(error.to_string()))
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
