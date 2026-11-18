//! Dynamic catalog and publication ownership for the [`Models`](crate::models::Models) facade.

use std::collections::HashMap;
use std::sync::Arc;

/// Dynamic catalog and publication owner behind the [`Models`] facade.
///
/// Store restoration, source registration, publisher retention and
/// publication generations form one consistency boundary for refresh rounds.
pub(crate) struct CatalogOwner {
    pub(crate) models_store: Option<crate::catalog::ModelsStore>,
    pub(crate) catalog_sources: Vec<std::sync::Arc<dyn crate::catalog::DynamicCatalogSource>>,
    pub(crate) catalog_publishers: HashMap<String, crate::catalog::CatalogPublisher>,
    pub(crate) dynamic_catalogs: HashMap<String, crate::catalog::ModelsStoreEntry>,
    pub(crate) publication_generation: Arc<()>,
}

impl CatalogOwner {
    pub(crate) fn new() -> Self {
        Self {
            models_store: None,
            catalog_sources: Vec::new(),
            catalog_publishers: HashMap::new(),
            dynamic_catalogs: HashMap::new(),
            publication_generation: Arc::new(()),
        }
    }
}
