//! Authentication contract target index.
//!
//! Each topic owns one authentication boundary while Cargo keeps one executable.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use opi_ai::auth::credential::BoxAuthFuture;
use opi_ai::auth::credential::Credential;
use opi_ai::auth::{
    AuthResolver, AuthScheme, LoginPresenter, OAuthCredential, OAuthProvider, ResolvedAuth,
    StaticAuthResolver,
};
use opi_ai::provider::{ProviderError, ProviderErrorCategory};
use opi_ai::{CredentialStore, MutationOutcome};
use secrecy::{ExposeSecret, SecretString};

#[path = "auth_contracts/adc_external.rs"]
mod adc_external;
#[path = "auth_contracts/adc_impersonation.rs"]
mod adc_impersonation;
#[path = "auth_contracts/adc_native.rs"]
mod adc_native;
#[path = "auth_contracts/errors.rs"]
mod errors;
#[path = "auth_contracts/fixtures.rs"]
mod fixtures;
#[path = "auth_contracts/mutations.rs"]
mod mutations;
#[path = "auth_contracts/resolution.rs"]
mod resolution;
#[path = "auth_contracts/scoped.rs"]
mod scoped;

pub(crate) use fixtures::*;
