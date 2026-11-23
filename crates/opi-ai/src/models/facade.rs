//! Public construction and defaulting for the [`Models`](crate::models::Models) facade.

use super::*;

impl Models {
    /// Construct an empty driver with an independent memory credential store.
    /// Registers no provider, needs no runtime, and performs no external source
    /// discovery or environment/file/network I/O.
    pub fn new() -> Self {
        Self {
            registry: RegistryOwner::new(ProviderRegistry::new()),
            auth_owner: AuthOwner::new(),
            compat: HashMap::new(),
            catalog_owner: CatalogOwner::new(),
            operation_owner: OperationOwner::new(),
        }
    }

    /// Wrap an existing registry to layer the driver semantics onto providers
    /// constructed by an outer factory. Owns an independent memory credential
    /// store without discovering external credential sources.
    ///
    /// Pre-registered providers are lookup-only until replaced by
    /// [`register_route`](Self::register_route) or
    /// [`register_chat_api`](Self::register_chat_api):
    /// [`Models::check_auth`] returns `Ok(None)` and
    /// [`prepare_call`](Self::prepare_call) returns
    /// [`ModelsError::RouteNotDispatchable`].
    pub fn from_registry(registry: ProviderRegistry) -> Self {
        Self {
            registry: RegistryOwner::new(registry),
            auth_owner: AuthOwner::new(),
            compat: HashMap::new(),
            catalog_owner: CatalogOwner::new(),
            operation_owner: OperationOwner::new(),
        }
    }
}

impl Default for Models {
    fn default() -> Self {
        Self::new()
    }
}
