//! Catalog input modality values.

/// One input modality declared by a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogInputModality {
    /// Text input.
    Text,
    /// Image input.
    Image,
}
