//! Per-request auth resolution and store-owned OAuth coordination.
//!
//! The SDK owns source/flow mechanisms and injectable store coordination.
//! Product policy, native keychain backends and terminal presentation live
//! in `opi-coding-agent`. Object-safe contracts let `Models`
//! resolve authentication once per prepared logical call and reuse it for
//! sequential retries without depending on a concrete backend or making
//! providers generic over the resolver.
//!
//! Three context responsibilities stay separate (AIR-066): the serializable
//! model Context/transcript (see [`crate::transcript`]), this module's
//! injectable [`AuthContext`] source access, and any process-local telemetry
//! observation. Credentials and source-access facts never enter model
//! serialization, and model/transcript data never drives credential
//! resolution.
//!
//! All async trait methods return [`BoxAuthFuture`] boxed futures, so
//! `AuthResolver`, `OAuthProvider`, and `LoginPresenter` are usable behind
//! `dyn` without an `async-trait` dependency.
//!
//! # Unstable
//!
//! Part of the **unstable 0.x extension substrate**. Breaking changes may
//! occur between minor versions without a major version bump.

mod ambient;
mod context;
pub mod credential;
pub mod mutation;
mod oauth_contract;
mod resolver;
mod values;
pub use ambient::AmbientAuthContext;
pub use context::{AuthContext, AuthContextError, AuthProcessSnapshot};
pub use credential::{
    ApiKeyCredential, BoxAuthFuture, Credential, CredentialSource, CredentialStore,
    CredentialStoreError,
};
pub use mutation::{InMemoryCredentialStore, MutationQueue};
pub use oauth_contract::{LoginPresenter, OAuthCredential, OAuthLoginMethod, OAuthProvider};
pub use resolver::{AuthResolver, StaticAuthResolver};
pub use values::*;

/// Store-owned OAuth refresh and request identity assembly.
pub mod store_oauth;

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) mod anthropic_federation;
#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) mod builtin;
#[cfg(feature = "builtins")]
pub mod builtin_aws;
#[cfg(feature = "builtins")]
pub mod builtin_vertex;

/// SDK OAuth flows (Anthropic first): injectable presenter + HTTP + store,
/// pinned to pi 1.3.0 wire shapes.
#[cfg(any(feature = "builtins", feature = "oauth-store"))]
pub mod oauth;

/// Local AWS credential-source chain modules (Bedrock's selected-source
/// coordinator; product-neutral, injectable, no live AWS calls).
#[cfg(any(feature = "builtins", feature = "cloud-auth"))]
pub mod adc {
    /// Cancelable executable (pluggable) supplier.
    pub mod cache;
    pub mod executable;
    /// `external_account` family coordinator (RFC8693 STS exchange).
    pub mod external;
    pub mod impersonation;
    /// Explicitly selected direct GCP metadata source.
    pub mod metadata;
    /// ADC native credential families (service account, authorized user,
    /// external account authorized user) and the chain coordinator.
    pub mod native;
    /// Subject-token suppliers (file/URL/certificate/aws1).
    pub mod subject;
}
#[cfg(any(feature = "builtins", feature = "cloud-auth"))]
pub mod aws {
    /// Regional STS endpoint resolution (partition/override arithmetic).
    pub mod cache;
    pub mod endpoint;
    /// Env/INI/role/web-identity/remote chain coordinator.
    pub mod local;
    pub mod login;
    /// ECS and IMDS metadata credential providers.
    pub mod metadata;
    /// Injectable `credential_process` execution seam.
    pub mod process;
    /// STS `AssumeRole` exchange and pinned Query/XML codecs.
    pub mod role;
    /// AWS SigV4 signing shared by cloud credential exchanges and Bedrock.
    pub mod sigv4;
    /// Injectable auth-source HTTP transport.
    pub mod source_http;
    pub mod sso;
    /// STS `AssumeRoleWithWebIdentity` exchange.
    pub mod web_identity;
}
