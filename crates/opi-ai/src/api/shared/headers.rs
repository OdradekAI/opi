//! Provider-header validation and merge policy shared by all wire adapters.
//!
//! This is a protocol-neutral wire contract rather than a provider profile:
//! it validates configured/model/request header sources, protects
//! route-managed identity and authentication headers, and produces the final
//! header list consumed by concrete adapters. Provider identity, endpoint and
//! catalog ownership remain in `crate::providers`.

use crate::types::headers::validate_pair;
pub use crate::types::headers::{ProviderHeaders, ProviderHeadersError};
pub(crate) use crate::types::headers::{
    RESERVED_PROVIDER_HEADERS, is_protected_header, validate_model_headers,
};
use std::collections::BTreeSet;

use crate::provider::{ProviderError, ProviderErrorSummary};

impl ProviderHeaders {
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
            .configured()
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
        let mut merged = Vec::with_capacity(
            self.configured().len() + route_headers.len() + request_headers.len(),
        );
        for (name, value) in self.configured().iter().chain(route_headers) {
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

fn provider_header_error(error: ProviderHeadersError) -> ProviderError {
    ProviderError::RequestFailed(ProviderErrorSummary::sanitized(error.to_string()))
}
