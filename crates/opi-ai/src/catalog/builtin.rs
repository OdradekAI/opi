//! Embedded catalog asset and its process-local validated cache.

use super::{Catalog, CatalogError};
use std::sync::{Arc, OnceLock};

const BUILTIN_CATALOG_BYTES: &[u8] = include_bytes!("../../resources/catalog/builtin/catalog.json");
const BUILTIN_MANIFEST_BYTES: &[u8] =
    include_bytes!("../../resources/catalog/builtin/manifest.json");

impl Catalog {
    /// The builtin offline bundle: parses and validates the embedded
    /// artifact once per process. Performs no network I/O.
    pub fn builtin() -> Result<&'static Arc<Catalog>, &'static CatalogError> {
        static BUILTIN: OnceLock<Result<Arc<Catalog>, CatalogError>> = OnceLock::new();
        match BUILTIN
            .get_or_init(|| Catalog::from_bytes(BUILTIN_CATALOG_BYTES, BUILTIN_MANIFEST_BYTES))
        {
            Ok(catalog) => Ok(catalog),
            Err(error) => Err(error),
        }
    }

    /// The embedded builtin bundle bytes: `(catalog, manifest)`.
    ///
    /// Exposed so callers (and tests) can revalidate, copy, or derive
    /// mutated candidates from the authentic bytes without new I/O.
    pub fn builtin_bundle_bytes() -> (&'static [u8], &'static [u8]) {
        (BUILTIN_CATALOG_BYTES, BUILTIN_MANIFEST_BYTES)
    }
}
