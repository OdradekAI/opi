//! Public dynamic-catalog sources feeding [`crate::Models::refresh`].
//!
//! A [`DynamicCatalogSource`] supplies complete per-provider dynamic
//! catalogs. Sources are an explicit input: the SDK registers none by
//! default, never auto-fetches, and has no pi.dev/default startup policy.
//! Hosts plug their own sources through the trait, or use the provided
//! [`HttpCatalogSource`], which fetches one distribution-schema-v1 document
//! over HTTP and is the reference real-network implementation.

use std::collections::HashMap;

use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

use super::ModelsStoreEntry;
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};

use super::wire;

/// Typed failure decoding a distribution-schema-v1 catalog document.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CatalogDocumentError {
    /// The document violated the distribution-schema-v1 contract.
    #[error("invalid catalog document: {message}")]
    Invalid {
        /// Redacted description of the violated contract.
        message: String,
    },
}

/// A source of complete dynamic catalogs, queried during an explicit
/// `refresh(true)` round.
///
/// Implementations must be cheap to clone or wrapped in an `Arc` when
/// registered; fetches observe the refresh cancellation token and must
/// return without publishing when it fires.
pub trait DynamicCatalogSource: Send + Sync {
    /// Stable identifier reported in refresh outcomes.
    fn id(&self) -> &str;

    /// Fetch the complete dynamic catalogs this source publishes, keyed by
    /// provider id. An error fails only this source: the refresh report
    /// records it and prior data for its providers is retained.
    fn fetch(
        &self,
        token: CancellationToken,
    ) -> BoxFuture<'static, Result<HashMap<String, ModelsStoreEntry>, ProviderError>>;
}

/// Fetches one distribution-schema-v1 `providers` document from a fixed URL.
///
/// The document shape matches [`crate::catalog::ModelsStore`] snapshots and
/// the Pi distribution schema; non-2xx responses and transport failures
/// surface as redacted [`ProviderError`]s. Register instances via `Arc` —
/// the source is not `Clone` because it owns its pooled [`HttpClient`].
#[derive(Debug)]
pub struct HttpCatalogSource {
    id: String,
    url: String,
    client: HttpClient,
}

impl HttpCatalogSource {
    /// Create a source that GETs `url`. The reported source id is
    /// `"http-catalog"`.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            id: "http-catalog".to_owned(),
            url: url.into(),
            client: HttpClient::new(),
        }
    }

    /// Decode a distribution-schema-v1 providers document into catalogs.
    ///
    /// Public so hosts can validate documents out of band; the HTTP fetch
    /// path uses the same decoder.
    pub fn decode_document(
        document: &serde_json::Value,
    ) -> Result<HashMap<String, ModelsStoreEntry>, CatalogDocumentError> {
        let invalid = |message: String| CatalogDocumentError::Invalid { message };
        let providers = document
            .get("providers")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| invalid("document requires a providers object".to_owned()))?;
        let mut catalogs = HashMap::with_capacity(providers.len());
        for (provider_id, provider) in providers {
            let decoded = wire::decode_provider_entry(provider_id, provider)
                .map_err(|message| invalid(format!("provider '{provider_id}': {message}")))?;
            catalogs.insert(provider_id.clone(), decoded);
        }
        Ok(catalogs)
    }
}

impl DynamicCatalogSource for HttpCatalogSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn fetch(
        &self,
        token: CancellationToken,
    ) -> BoxFuture<'static, Result<HashMap<String, ModelsStoreEntry>, ProviderError>> {
        let url = self.url.clone();
        let client = self.client.client().clone();
        Box::pin(async move {
            let request = client
                .get(&url)
                .timeout(std::time::Duration::from_secs(30))
                .send();
            let response = tokio::select! {
                response = request => response.map_err(|_| {
                    ProviderError::RequestFailed(ProviderErrorSummary::redacted())
                })?,
                _ = token.cancelled() => return Err(ProviderError::Cancelled),
            };
            let status = response.status();
            let body = tokio::select! {
                body = response.bytes() => body.map_err(|_| {
                    ProviderError::RequestFailed(ProviderErrorSummary::redacted())
                })?,
                _ = token.cancelled() => return Err(ProviderError::Cancelled),
            };
            if !status.is_success() {
                return Err(ProviderError::RequestFailed(
                    ProviderErrorSummary::from_untrusted(status.to_string()),
                ));
            }
            let document: serde_json::Value = serde_json::from_slice(&body)
                .map_err(|_| ProviderError::RequestFailed(ProviderErrorSummary::redacted()))?;
            HttpCatalogSource::decode_document(&document).map_err(|error| {
                ProviderError::RequestFailed(ProviderErrorSummary::from_untrusted(
                    error.to_string(),
                ))
            })
        })
    }
}
