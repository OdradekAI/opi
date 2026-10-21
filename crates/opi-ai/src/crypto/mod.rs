//! Private upstream key-admission and signing owner (RDF-23 substrate).
//!
//! Owns opi-ai's use of the unmodified crates.io crypto baseline
//! (`rustls` 0.23.40 with the `aws-lc-rs` provider; `aws-lc-rs` 1.18.2 on
//! `aws-lc-sys` 0.46.0) exclusively through their public safe Rust seams:
//! `KeyProvider::load_private_key`, `SigningKey::algorithm`/`choose_scheme`
//! and `Signer::sign`. There is no local FFI, crypto fork, private crypto
//! namespace or new public crypto API.
//!
//! The admission profile accepts unencrypted two-prime RSA-2048..8192 keys
//! (PKCS#1/PKCS#8, PEM/DER, including e=3) and P-521 keys (PKCS#8/SEC1).
//! RSA-1024, multi-prime RSA, encrypted containers, malformed inputs,
//! public-key-only inputs and algorithms outside the profile are typed,
//! redacted rejections; they never produce false success, downgrade or
//! identity rediscovery, and error values never carry key material.
//!
//! The AWS-LC crypto provider is instantiated explicitly per owner and is
//! never installed process-globally; transitive `ring` (via the reqwest
//! rustls-tls graph) coexists untouched, and existing HMAC/SHA/SigV4
//! signing is unchanged.
//!
//! Synchronous native signing is not hard-preemptible. Cancellation is
//! observed between operations only: owned keys and resources are retained
//! until the blocking signing worker joins, late results are suppressed
//! instead of published, and no claim is made about preempting a single
//! native call.
//!
//! This module is private production substrate for the later TLS/mTLS host
//! wiring (task 20.14) and the real ADC callers (task 20.21.1). It is not
//! re-exported and closes no product acceptance scenario by itself.

use std::sync::Arc;

use rustls::pki_types::{PrivateKeyDer, pem::PemObject};
use rustls::{SignatureAlgorithm, SignatureScheme};

mod verify;
mod worker;

#[cfg(test)]
mod fixtures;

/// Profile-limited RSA PKCS#1 v1.5 schemes, in preference order. RS256 is the
/// only mandatory member (JWT/ADC shape).
const RSA_SCHEMES: &[SignatureScheme] = &[
    SignatureScheme::RSA_PKCS1_SHA256,
    SignatureScheme::RSA_PKCS1_SHA384,
    SignatureScheme::RSA_PKCS1_SHA512,
];

/// The only admitted ECDSA scheme (P-521 keys; P-256/P-384 are typed
/// rejections at admission, not silent narrower signing).
const ECDSA_P521_SCHEME: SignatureScheme = SignatureScheme::ECDSA_NISTP521_SHA512;

/// Redacted, stage-typed private-key admission failure. Variants never retain
/// or echo input bytes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CryptoKeyError {
    /// The input is not decodable as an unencrypted private key (PEM/DER
    /// structure, missing key body, public-key-only or empty input).
    #[error("private key input could not be decoded")]
    Decode,
    /// The upstream key provider rejected the key: unsupported key profile
    /// (RSA below 2048 bits, multi-prime RSA), encrypted container, or
    /// malformed backend structure.
    #[error("upstream key provider rejected the private key")]
    BackendLoad,
    /// The key algorithm is outside the accepted profile.
    #[error("key algorithm outside the supported profile")]
    UnsupportedAlgorithm(SignatureAlgorithm),
    /// The key parses and loads but offers no admitted signature scheme
    /// (for example a P-256/P-384 ECDSA key).
    #[error("key offers no supported signature scheme")]
    UnsupportedScheme(SignatureAlgorithm),
}

/// Redacted signing failure. No key or message material is retained.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CryptoSignError {
    /// The requested scheme is not one of the schemes admitted for this key.
    #[error("signature scheme not offered for this key")]
    SchemeNotOffered,
    /// The synchronous native signing call failed.
    #[error("native signing operation failed")]
    Native,
    /// The result was suppressed because the caller cancelled between
    /// operations; it is never delivered late.
    #[error("signing result suppressed after cancellation")]
    Suppressed,
}

/// Explicit, instance-scoped signing owner over the upstream AWS-LC
/// provider. Constructing an owner never installs a process-global
/// provider, mutates global configuration, or reads ambient credentials.
pub(crate) struct SigningOwner {
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl SigningOwner {
    /// Builds the owner over the explicit `aws-lc-rs` provider instance.
    pub(crate) fn aws_lc() -> Self {
        // Explicit instance only: `CryptoProvider::install_default` is never
        // called anywhere in this crate.
        Self {
            provider: Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        }
    }

    /// Admits an unencrypted private key from PEM input through the public
    /// `KeyProvider::load_private_key` seam and the profile gates below.
    pub(crate) fn load_private_key_pem(
        &self,
        pem: &[u8],
    ) -> Result<OwnedSigningKey, CryptoKeyError> {
        let key = PrivateKeyDer::from_pem_slice(pem).map_err(|_| CryptoKeyError::Decode)?;
        self.load(key)
    }

    /// Admits an unencrypted private key from DER input (PKCS#1, PKCS#8 or
    /// SEC1 structure is auto-detected by the upstream decoder).
    pub(crate) fn load_private_key_der(
        &self,
        der: &[u8],
    ) -> Result<OwnedSigningKey, CryptoKeyError> {
        let key = PrivateKeyDer::try_from(der).map_err(|_| CryptoKeyError::Decode)?;
        self.load(key)
    }

    fn load(&self, key: PrivateKeyDer<'_>) -> Result<OwnedSigningKey, CryptoKeyError> {
        // Upstream seam: the backend rejects RSA below 2048 bits,
        // multi-prime RSA and encrypted containers here. `clone_key` erases
        // the borrow lifetime for the owned upstream key.
        let loaded = self
            .provider
            .key_provider
            .load_private_key(key.clone_key())
            .map_err(|_| CryptoKeyError::BackendLoad)?;
        let algorithm = loaded.algorithm();
        let offered = match algorithm {
            SignatureAlgorithm::RSA => {
                let offered = RSA_SCHEMES
                    .iter()
                    .copied()
                    .filter(|scheme| loaded.choose_scheme(std::slice::from_ref(scheme)).is_some())
                    .collect::<Vec<_>>();
                // RS256 is the mandatory profile floor for RSA admission.
                if !offered.contains(&SignatureScheme::RSA_PKCS1_SHA256) {
                    return Err(CryptoKeyError::UnsupportedScheme(algorithm));
                }
                offered
            }
            SignatureAlgorithm::ECDSA => {
                // Fail closed for every EC curve except the admitted P-521.
                if loaded
                    .choose_scheme(std::slice::from_ref(&ECDSA_P521_SCHEME))
                    .is_some()
                {
                    vec![ECDSA_P521_SCHEME]
                } else {
                    return Err(CryptoKeyError::UnsupportedScheme(algorithm));
                }
            }
            other => return Err(CryptoKeyError::UnsupportedAlgorithm(other)),
        };
        Ok(OwnedSigningKey {
            key: loaded,
            algorithm,
            offered,
        })
    }
}

/// An admitted, profile-gated private signing key owned by this crate.
///
/// Holds the upstream `Arc<dyn SigningKey>`; dropping every owner and worker
/// reference releases the native key material.
pub(crate) struct OwnedSigningKey {
    key: Arc<dyn rustls::sign::SigningKey>,
    algorithm: SignatureAlgorithm,
    offered: Vec<SignatureScheme>,
}

impl std::fmt::Debug for OwnedSigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redacted: classification only, never key material.
        f.debug_struct("OwnedSigningKey")
            .field("algorithm", &self.algorithm)
            .field("offered", &self.offered)
            .finish()
    }
}

impl OwnedSigningKey {
    /// The key's upstream algorithm classification.
    pub(crate) fn algorithm(&self) -> SignatureAlgorithm {
        self.algorithm
    }

    /// Schemes this key may sign with under the profile.
    pub(crate) fn offered_schemes(&self) -> &[SignatureScheme] {
        &self.offered
    }

    /// Synchronous native signing through `choose_scheme` + `Signer::sign`.
    /// This call is not hard-preemptible; use [`Self::sign_on_worker`] for
    /// cancellation-aware owned-worker signing.
    pub(crate) fn sign(
        &self,
        scheme: SignatureScheme,
        message: &[u8],
    ) -> Result<Vec<u8>, CryptoSignError> {
        sign_native(&self.key, scheme, message)
    }

    /// Signs with RS256 (RSA PKCS#1 v1.5 SHA-256; JWT/ADC shape).
    pub(crate) fn sign_rs256(&self, message: &[u8]) -> Result<Vec<u8>, CryptoSignError> {
        self.sign(SignatureScheme::RSA_PKCS1_SHA256, message)
    }

    /// Signs with ECDSA P-521 SHA-512 (ASN.1 DER signature).
    pub(crate) fn sign_p521_sha512(&self, message: &[u8]) -> Result<Vec<u8>, CryptoSignError> {
        self.sign(ECDSA_P521_SCHEME, message)
    }

    /// Starts cancellation-aware signing on a dedicated blocking worker.
    /// See [`worker`] for the ownership and suppression contract.
    pub(crate) fn sign_on_worker(
        &self,
        scheme: SignatureScheme,
        message: Vec<u8>,
    ) -> worker::WorkerSigning {
        worker::WorkerSigning::new(self.key.clone(), scheme, message)
    }

    /// Weak reference to the upstream key owner; used by the worker-lifecycle
    /// regressions to observe retention until join.
    #[cfg(test)]
    pub(crate) fn weak_key(&self) -> std::sync::Weak<dyn rustls::sign::SigningKey> {
        std::sync::Arc::downgrade(&self.key)
    }
}

/// Shared native signing step used by both the synchronous and worker paths.
pub(crate) fn sign_native(
    key: &Arc<dyn rustls::sign::SigningKey>,
    scheme: SignatureScheme,
    message: &[u8],
) -> Result<Vec<u8>, CryptoSignError> {
    let signer = key
        .choose_scheme(std::slice::from_ref(&scheme))
        .ok_or(CryptoSignError::SchemeNotOffered)?;
    signer.sign(message).map_err(|_| CryptoSignError::Native)
}

#[cfg(test)]
mod tests;
