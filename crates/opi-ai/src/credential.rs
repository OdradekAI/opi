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
//! Production native-store selection remains owned by `opi-coding-agent`,
//! which installs Windows Credential Manager, macOS Keychain Services, or
//! Freedesktop Secret Service before credential-aware startup paths.
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
/// on drop. Raw values are exposed only at the concrete provider HTTP boundary
/// and at the protected keychain-serialization boundary owned by
/// `opi-coding-agent` (see [`secrecy::ExposeSecret`]); serialized and
/// intermediate buffers are zeroized there.
///
/// Secret material never appears in [`Debug`](std::fmt::Debug) output: the
/// manual impl redacts every secret field, mirroring the legacy
/// [`crate::SecretKey`] redacting impl.
#[derive(Clone)]
pub enum Credential {
    /// A static API key (Anthropic, OpenAI, Mistral, ...).
    ApiKey(SecretString),
    /// An OAuth token envelope. `base_url` preserves provider-specific
    /// endpoints (e.g. a Copilot enterprise host). `opi-coding-agent` owns the
    /// concrete `OAuthCredential`/`ResolvedAuth` flow and live HTTP refresh
    /// bridge; this crate owns the persistence contract.
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
        /// Provider account identity, when required by a concrete wire.
        /// Non-secret and optional for version-1 decode compatibility.
        account_id: Option<String>,
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
                account_id,
                ..
            } => f
                .debug_struct("Credential::OAuthToken")
                .field("access", &"<redacted>")
                .field("refresh", &"<redacted>")
                .field("expires_at", expires_at)
                .field("base_url", base_url)
                .field("account_id", account_id)
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
    /// The native credential service is not reachable on this host.
    #[error("credential backend unavailable for '{provider}': {reason}")]
    BackendUnavailable { provider: String, reason: String },
    /// The credential backend could not be reached or rejected the operation.
    #[error("credential backend error for '{provider}': {reason}")]
    Backend { provider: String, reason: String },
    /// The stored envelope could not be parsed as the expected JSON shape.
    #[error("malformed credential envelope for '{provider}': {reason}")]
    MalformedEnvelope { provider: String, reason: String },
    /// The non-secret presence/kind marker is missing required structure, has
    /// an unknown closed-set value, or disagrees with the protected entry.
    /// Raw marker bytes are intentionally never retained in this error.
    #[error("corrupt credential marker for '{provider}'")]
    CorruptMarker { provider: String },
    /// The stored envelope uses an unknown version or credential kind.
    #[error("unknown credential envelope for '{provider}': version={version:?}, field={field:?}")]
    UnknownEnvelope {
        provider: String,
        version: Option<u32>,
        field: UnknownEnvelopeField,
    },
    /// A provider entry exists, but its credential kind is not valid for this path.
    #[error("unexpected credential kind for '{provider}': expected {expected}, found {actual}")]
    UnexpectedCredentialKind {
        provider: String,
        expected: &'static str,
        actual: &'static str,
    },
}

/// Closed, non-secret classification for an unknown envelope discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownEnvelopeField {
    /// The numeric envelope version is unsupported.
    Version,
    /// The credential kind discriminator is unsupported.
    Kind,
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
