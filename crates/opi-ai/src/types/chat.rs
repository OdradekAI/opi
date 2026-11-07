//! Complete immutable chat-route records.

use super::model::ModelInfo;

/// The complete chat record one ordinary call binds to: provider id, bare
/// model id, and the full [`ModelInfo`] (exact wire, capabilities, compat).
///
/// A record does not need catalog membership: calls dispatch on the record's
/// provider/wire exactly as the caller supplied it, and same-ID catalog data
/// never replaces a frozen record.
#[derive(Debug, Clone)]
pub struct ChatModelRecord {
    provider_id: String,
    info: ModelInfo,
}

impl ChatModelRecord {
    /// Build one complete record.
    pub fn new(provider_id: impl Into<String>, info: ModelInfo) -> Self {
        Self {
            provider_id: provider_id.into(),
            info,
        }
    }

    /// Owning provider id of the record.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Bare model id (no provider prefix).
    pub fn model_id(&self) -> &str {
        &self.info.id
    }

    /// Exact wire API of the record.
    pub fn wire_api(&self) -> crate::types::model::WireApi {
        self.info.wire_api
    }

    /// The full model metadata.
    pub fn info(&self) -> &ModelInfo {
        &self.info
    }

    pub(crate) fn spec(&self) -> String {
        format!("{}:{}", self.provider_id, self.info.id)
    }
}
