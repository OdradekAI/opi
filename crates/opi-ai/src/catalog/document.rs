//! Offline distribution document decoding shared by sources and hosts.

use super::{ModelsStoreEntry, wire};
use std::collections::HashMap;

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
