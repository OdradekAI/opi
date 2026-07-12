//! Abstract credential store contract (Phase 14.1).
//!
//! IO-free types owned by [`crate`]. The concrete keychain, env, resolver, and
//! lock implementations live in `opi-coding-agent`; `opi-ai` defines only the
//! abstract contract so the agent runtime and provider layer never depend on a
//! concrete backend.
//!
//! The [`CredentialStore`] trait is object-safe and returns [`BoxAuthFuture`]
//! boxed futures, so a heterogeneous registry can hold `Arc<dyn
//! CredentialStore>` and refresh-on-read stays asynchronous without an
//! `async-trait` dependency. Native `async fn` trait methods remain available
//! to monomorphized internal helpers, but not to provider-held trait objects.
//!
//! Backend failures are preserved as distinct from missing entries: a missing
//! keychain entry ([`CredentialSource::Absent`]) is not collapsed into a
//! backend failure ([`CredentialSource::BackendUnavailable`]), so doctor and
//! `--list-models` can distinguish "no stored credential" from "no keychain
//! daemon".
//!
//! # Unstable
//!
//! This surface is part of the **unstable 0.x extension substrate**. Breaking
//! changes may occur between minor versions without a major version bump.

use std::future::Future;
use std::pin::Pin;

use secrecy::SecretString;
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// Credential value
// ---------------------------------------------------------------------------

/// A persisted credential. Both API keys and OAuth tokens live in the OS
/// keychain envelope. Every secret field uses [`SecretString`], which zeroizes
/// on drop; the only place the raw value is exposed is the concrete provider's
/// HTTP boundary (see [`secrecy::ExposeSecret`]).
///
/// Secret material never appears in [`Debug`](std::fmt::Debug) output: the
/// manual impl redacts every secret field, mirroring the legacy
/// [`crate::SecretKey`] redacting impl.
#[derive(Clone)]
pub enum Credential {
    /// A static API key (Anthropic, OpenAI, Mistral, ...).
    ApiKey(SecretString),
    /// An OAuth token envelope. `base_url` preserves provider-specific
    /// endpoints (e.g. a Copilot enterprise host). T2 owns the concrete
    /// `OAuthCredential`/`ResolvedAuth` and the live HTTP refresh bridge; T1
    /// only persists and probes this envelope.
    OAuthToken {
        /// Bearer/access token. Redacted in all diagnostics.
        access: SecretString,
        /// Refresh token. Redacted in all diagnostics.
        refresh: SecretString,
        /// Token expiry, if known.
        expires_at: Option<OffsetDateTime>,
        /// Provider-specific base URL preserved across refresh (e.g. Copilot
        /// enterprise). Non-secret.
        base_url: Option<String>,
    },
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Manual impl so a future secrecy version that changes SecretString's
        // Debug cannot leak secret material here. Mirrors SecretKey's redacting
        // Debug in crate::provider_collection.
        match self {
            Credential::ApiKey(_) => f.write_str("Credential::ApiKey(<redacted>)"),
            Credential::OAuthToken {
                expires_at,
                base_url,
                ..
            } => f
                .debug_struct("Credential::OAuthToken")
                .field("access", &"<redacted>")
                .field("refresh", &"<redacted>")
                .field("expires_at", expires_at)
                .field("base_url", base_url)
                .finish(),
        }
    }
}

// ---------------------------------------------------------------------------
// Probe result
// ---------------------------------------------------------------------------

/// Three-state result of probing a credential store without reading the secret.
///
/// Used by doctor and `--list-models` to report redacted credential presence
/// and to gate the (non-live) listing dispatch path. Carries no secret
/// material: [`Present`](Self::Present) only carries a non-secret display
/// label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialSource {
    /// An entry exists for the provider. `label` is a non-secret display
    /// source string (e.g. `"keychain opi:anthropic"`).
    Present { label: String },
    /// No entry exists for the provider. Distinct from a backend failure.
    Absent,
    /// The credential backend could not be reached (no keychain daemon, no
    /// compiled native store, platform error). API keys may fall back to env;
    /// persisted OAuth remains keychain-required because refresh tokens must
    /// persist somewhere durable.
    BackendUnavailable { reason: String },
}

impl CredentialSource {
    /// Non-secret display label suitable for doctor / `--list-models`.
    pub fn display_source(&self) -> String {
        match self {
            CredentialSource::Present { label } => label.clone(),
            CredentialSource::Absent => "absent".to_owned(),
            CredentialSource::BackendUnavailable { reason } => {
                format!("backend unavailable: {reason}")
            }
        }
    }

    /// Whether the probe found a stored entry.
    pub fn is_present(&self) -> bool {
        matches!(self, CredentialSource::Present { .. })
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors from credential store operations.
///
/// Backend failures stay distinct from missing entries: a malformed or
/// unknown envelope, or an unreachable backend, is never collapsed into
/// [`CredentialSource::Absent`] or an env fallback, so a corrupt store is
/// surfaced explicitly rather than silently re-prompting for a credential.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CredentialStoreError {
    /// The credential backend could not be reached or rejected the operation.
    #[error("credential backend error for '{provider}': {reason}")]
    Backend { provider: String, reason: String },
    /// The stored envelope could not be parsed as the expected JSON shape.
    #[error("malformed credential envelope for '{provider}': {reason}")]
    MalformedEnvelope { provider: String, reason: String },
    /// The stored envelope uses an unknown version or credential kind.
    #[error("unknown credential envelope for '{provider}': version={version:?}, kind={kind:?}")]
    UnknownEnvelope {
        provider: String,
        version: Option<u32>,
        kind: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Object-safe store trait
// ---------------------------------------------------------------------------

/// Boxed, object-safe future used by [`CredentialStore`].
///
/// `Pin<Box<dyn Future<Output = T> + Send + 'a>>` lets the trait stay
/// object-safe for `Arc<dyn CredentialStore>` while keeping refresh-on-read
/// asynchronous and `Send` for the multi-thread tokio runtime.
pub type BoxAuthFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Object-safe credential store contract.
///
/// All methods are IO-free from `opi-ai`'s point of view: the concrete
/// keychain/env implementations and the cross-process mutation lock live in
/// `opi-coding-agent`. `read`/`write`/`delete`/`probe` return boxed futures
/// so the trait is usable behind `dyn CredentialStore`.
///
/// * `read` returns `Ok(None)` for a missing entry and an `Err` for a backend
///   failure — the two are never collapsed.
/// * `probe` returns a redacted [`CredentialSource`] without reading the
///   secret, for doctor / `--list-models`.
/// * `write`/`delete` are mutations; concrete implementations wrap them in the
///   shared cross-process lock (acquire-then-re-read).
pub trait CredentialStore: Send + Sync {
    /// Read the credential for `provider_id`, if present.
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>>;

    /// Persist `cred` under `provider_id`, replacing any existing entry.
    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>>;

    /// Delete the credential for `provider_id`, if present.
    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>>;

    /// Probe for a credential without reading the secret.
    fn probe<'a>(&'a self, provider_id: &'a str) -> BoxAuthFuture<'a, CredentialSource>;
}
