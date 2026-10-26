//! Scoped provider environment lookup (pinned source:
//! `.repo/pi-1.3.0/packages/ai/src/utils/provider-env.ts`; task 20.53.6).
//!
//! Resolution is lazy and side-effect free: no environment is read at
//! construction time and nothing is cached or mutated globally. Proxy and
//! header-identity policy stay with their owning modules ([`crate::http`]
//! proxy resolution, [`crate::auth`] identities) — this helper only answers
//! "which value does this name resolve to right now". The pinned Bun
//! `/proc/self/environ` fallback has no Rust counterpart and is not
//! reproduced.

use std::collections::HashMap;

/// Resolve a provider environment variable from a scoped override map,
/// falling back to the ambient process environment (pinned
/// `getProviderEnvValue(name, env)`).
///
/// Lookup order is: the scoped map first, then [`std::env::var`]. Faithful
/// to the pinned `env?.[name] || process.env[name] || undefined` chain, an
/// empty scoped value falls back to the ambient lookup and an empty ambient
/// value yields `None`. Missing names yield `None`. Names are matched
/// exactly (environment access is case-sensitive on the ambient path, so a
/// scoped override must spell the name the same way).
pub fn provider_env_value(name: &str, scoped: Option<&HashMap<String, String>>) -> Option<String> {
    if let Some(value) = scoped.and_then(|scoped| scoped.get(name))
        && !value.is_empty()
    {
        return Some(value.clone());
    }
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}
