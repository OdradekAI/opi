//! Operation-qualified records and source freshness metadata.

use std::fmt;

/// One catalog operation a model entry is qualified for.
///
/// Mirrors Pi's `type:id` entry identity: the same bare model id may appear
/// once per operation for one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CatalogOperation {
    /// Tool-capable chat model (`type: "chat"`).
    Chat,
    /// Image generation model (`type: "image"`).
    Image,
    /// Classifier model (`type: "classifier"`).
    Classifier,
}

impl CatalogOperation {
    /// The artifact's `type` string for this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            CatalogOperation::Chat => "chat",
            CatalogOperation::Image => "image",
            CatalogOperation::Classifier => "classifier",
        }
    }
}

impl fmt::Display for CatalogOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One owned executable record in a persisted provider catalog.
/// The operation and bare id together identify an entry; an image and a chat
/// model may therefore have the same bare id without replacing each other.
#[derive(Debug, Clone, PartialEq)]
pub enum CatalogRecord {
    /// Complete chat metadata, boxed to keep mixed catalogs compact.
    Chat(Box<crate::types::model::ModelInfo>),
    /// Complete image-generation metadata.
    Image(crate::types::images::ImageModelRecord),
    /// Complete classifier metadata.
    Classifier(crate::types::classifier::ClassifierModelRecord),
}

impl CatalogRecord {
    /// The entry's operation identity.
    pub fn operation(&self) -> CatalogOperation {
        match self {
            Self::Chat(_) => CatalogOperation::Chat,
            Self::Image(_) => CatalogOperation::Image,
            Self::Classifier(_) => CatalogOperation::Classifier,
        }
    }

    /// The bare model id, without a provider or operation prefix.
    pub fn id(&self) -> &str {
        match self {
            Self::Chat(model) => &model.id,
            Self::Image(model) => &model.id,
            Self::Classifier(model) => &model.id,
        }
    }
}

impl From<crate::types::model::ModelInfo> for CatalogRecord {
    fn from(model: crate::types::model::ModelInfo) -> Self {
        Self::Chat(Box::new(model))
    }
}

/// One provider's complete dynamic catalog and source freshness facts.
/// Timestamps are Unix milliseconds, matching the source's `Date.now()` and
/// `Date.parse()`. Missing values stay missing; the SDK does not invent a
/// refresh schedule or interpret the opaque ETag.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelsStoreEntry {
    /// Complete operation-qualified models, in source order. Empty is an
    /// authoritative publication, distinct from no provider entry.
    pub models: Vec<CatalogRecord>,
    /// Source modification time in Unix milliseconds, when known.
    pub last_modified: Option<i64>,
    /// Last source check in Unix milliseconds, when supplied by the caller.
    pub checked_at: Option<i64>,
    /// Opaque source validator, including any quotes or weak prefix.
    pub etag: Option<String>,
}
