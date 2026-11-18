//! Authentication and source-resolution ownership for the [`Models`](crate::models::Models) facade.

use std::collections::HashMap;
use std::sync::Arc;

use super::{AuthDescriptor, BuiltinAuthCheck, BuiltinAuthFactory};
use crate::auth::{AuthProvenanceSource, AuthResolver};
use crate::credential::{CredentialSource, CredentialStore};

/// Authentication and source-resolution owner behind [`Models`].
///
/// Resolver maps, OAuth bindings, credential-store state and builtin auth
/// factories share lifecycle and source-ownership rules, so the facade keeps
/// them behind one private owner.
pub(crate) struct AuthOwner {
    pub(crate) auth: HashMap<String, AuthDescriptor>,
    pub(crate) probed: HashMap<String, CredentialSource>,
    pub(crate) resolvers: HashMap<String, Arc<dyn AuthResolver>>,
    pub(crate) sources: HashMap<String, AuthProvenanceSource>,
    pub(crate) oauth_flows: HashMap<String, std::sync::Arc<dyn crate::auth::OAuthProvider>>,
    pub(crate) oauth_routes: std::collections::HashSet<String>,
    pub(crate) builtin_auth_factories: HashMap<String, BuiltinAuthFactory>,
    pub(crate) builtin_auth_checks: HashMap<String, BuiltinAuthCheck>,
    pub(crate) builtin_default_auth: std::collections::HashSet<String>,
    pub(crate) auth_context: std::sync::Arc<dyn crate::auth::AuthContext>,
    pub(crate) credential_store: Option<std::sync::Arc<dyn CredentialStore>>,
    pub(crate) credential_store_explicit: bool,
}

impl AuthOwner {
    pub(crate) fn new() -> Self {
        Self {
            auth: HashMap::new(),
            probed: HashMap::new(),
            resolvers: HashMap::new(),
            sources: HashMap::new(),
            oauth_flows: HashMap::new(),
            oauth_routes: std::collections::HashSet::new(),
            builtin_auth_factories: HashMap::new(),
            builtin_auth_checks: HashMap::new(),
            builtin_default_auth: std::collections::HashSet::new(),
            auth_context: std::sync::Arc::new(crate::auth::AmbientAuthContext),
            credential_store: Some(Arc::new(crate::InMemoryCredentialStore::new())),
            credential_store_explicit: false,
        }
    }
}
