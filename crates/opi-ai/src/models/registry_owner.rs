//! Registry and session-resource ownership for the [`Models`](crate::models::Models) facade.

use std::collections::HashMap;
use std::sync::Arc;

use super::ProviderRegistry;

/// Registry and resource-lifetime owner behind the public [`Models`] facade.
///
/// Provider lookup, registration serialization and session-resource
/// namespaces change together, so they stay behind one private owner instead
/// of being scattered across the facade's auth and catalog state.
pub(crate) struct RegistryOwner {
    pub(crate) registry: ProviderRegistry,
    pub(crate) resource_owners:
        HashMap<String, Arc<dyn crate::codex_resources::SessionResourceOwner>>,
    pub(crate) all_resource_owners:
        Arc<std::sync::Mutex<Vec<Arc<dyn crate::codex_resources::SessionResourceOwner>>>>,
    pub(crate) registration_gate: Arc<std::sync::Mutex<()>>,
}

impl RegistryOwner {
    pub(crate) fn new(registry: ProviderRegistry) -> Self {
        let resource_owners: HashMap<_, _> = registry
            .providers()
            .iter()
            .filter_map(|provider| {
                provider
                    .session_resources()
                    .map(|owner| (provider.id().to_owned(), owner.new_namespace()))
            })
            .collect();
        let all_resource_owners = Arc::new(std::sync::Mutex::new(
            resource_owners.values().cloned().collect(),
        ));
        Self {
            registry,
            resource_owners,
            all_resource_owners,
            registration_gate: Arc::default(),
        }
    }
}
