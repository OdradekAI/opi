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

use super::ModelsStoreEntry;
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};

pub use super::{CatalogDocumentError, CatalogRefreshContext, DynamicCatalogSource};

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
    provider_ids: Vec<String>,
    client: HttpClient,
}

impl HttpCatalogSource {
    /// Declare provider ownership for a source that GETs `url` on refresh.
    /// Construction performs no source work. The reported source id is
    /// `"http-catalog"`.
    pub fn new(url: impl Into<String>, provider_ids: impl IntoIterator<Item = String>) -> Self {
        Self {
            id: "http-catalog".to_owned(),
            url: url.into(),
            provider_ids: provider_ids.into_iter().collect(),
            client: HttpClient::new(),
        }
    }

    /// Decode an offline document. Prefer [`super::decode_document`] when no HTTP source is needed.
    pub fn decode_document(
        document: &serde_json::Value,
    ) -> Result<HashMap<String, ModelsStoreEntry>, CatalogDocumentError> {
        super::decode_document(document)
    }
}

impl DynamicCatalogSource for HttpCatalogSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider_ids(&self) -> &[String] {
        &self.provider_ids
    }

    fn fetch(
        &self,
        context: CatalogRefreshContext,
    ) -> BoxFuture<'static, Result<HashMap<String, ModelsStoreEntry>, ProviderError>> {
        let url = self.url.clone();
        let client = self.client.client().clone();
        Box::pin(async move {
            if !context.allow_network || context.providers.is_empty() {
                return Ok(HashMap::new());
            }
            let token = context.cancel;
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
            super::decode_document(&document).map_err(|error| {
                ProviderError::RequestFailed(ProviderErrorSummary::from_untrusted(
                    error.to_string(),
                ))
            })
        })
    }
}
