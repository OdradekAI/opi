//! Source-scoped TLS configuration for [`crate::http::HttpClient`] (RDF-23).
//!
//! This module owns the one typed TLS options value accepted by
//! [`crate::http::HttpClientBuilder::tls`]: source-scoped trust inputs, an
//! optional owned client certificate-chain/key identity, and a
//! source-qualified H2/H1 ALPN preference. All inputs are validated and
//! frozen when the client is built, before any network I/O, with typed
//! redacted failures.
//!
//! Fixed profile (not caller-selectable): an explicit instance `aws-lc-rs`
//! provider is used with TLS 1.3 + TLS 1.2, the provider's default cipher
//! suites and signature schemes, and the declared key-exchange group order
//! `X25519MLKEM768, X25519, secp256r1, secp384r1`. No backend selector,
//! generic crypto trait, raw verifier, verification bypass or test-only
//! signing API is exposed, and no process-global rustls provider or
//! configuration is ever installed or mutated. WebPKI chain, hostname and
//! time verification always stay on.
//!
//! Default trust is the packaged pinned Node reference CA bundle in
//! [`node_trust`]; extra roots extend it, explicit roots replace it, and
//! host-system roots are an explicit opt-in. No ambient credential, trust or
//! proxy discovery exists in this path.

mod node_trust;

use std::sync::Arc;

use rustls::{
    RootCertStore,
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};

use crate::crypto::{self, CryptoKeyError};

/// Declared key-exchange group order (RDF-23).
use rustls::crypto::aws_lc_rs::kx_group::{SECP256R1, SECP384R1, X25519, X25519MLKEM768};

/// Typed, redacted reason a client identity key was rejected at admission.
///
/// Mirrors the private crypto-owner admission taxonomy; no variant retains
/// or echoes key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TlsKeyError {
    /// The input is not decodable as an unencrypted private key (PEM/DER
    /// structure, missing key body, public-key-only or empty input).
    #[error("private key input could not be decoded")]
    Decode,
    /// The upstream key provider rejected the key: unsupported key profile
    /// (RSA below 2048 bits, multi-prime RSA), encrypted container or
    /// malformed backend structure.
    #[error("upstream key provider rejected the private key")]
    BackendLoad,
    /// The key algorithm is outside the accepted profile.
    #[error("key algorithm outside the supported profile")]
    UnsupportedAlgorithm,
    /// The key parses and loads but offers no admitted signature scheme.
    #[error("key offers no supported signature scheme")]
    UnsupportedScheme,
}

impl From<CryptoKeyError> for TlsKeyError {
    fn from(error: CryptoKeyError) -> Self {
        match error {
            CryptoKeyError::Decode => TlsKeyError::Decode,
            CryptoKeyError::BackendLoad => TlsKeyError::BackendLoad,
            CryptoKeyError::UnsupportedAlgorithm(_) => TlsKeyError::UnsupportedAlgorithm,
            CryptoKeyError::UnsupportedScheme(_) => TlsKeyError::UnsupportedScheme,
        }
    }
}

/// Typed, redacted TLS configuration failure. Failures are reported when the
/// client is built, before any I/O; variants never retain or echo
/// certificate or key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TlsConfigError {
    /// A caller-provided root certificate input could not be decoded.
    #[error("root certificate input could not be decoded")]
    RootDecode,
    /// The packaged pinned default root store could not be loaded (internal
    /// invariant; the pinned bundle is malformed for this build).
    #[error("pinned default root store could not be loaded")]
    DefaultRootStore,
    /// The explicitly selected host-system root store is unavailable or
    /// empty. There is no silent fallback to another trust mode.
    #[error("host system root store is unavailable")]
    HostRootsUnavailable,
    /// The client identity key was rejected at admission.
    #[error("client identity key rejected: {0}")]
    KeyAdmission(#[from] TlsKeyError),
    /// The client identity chain could not be decoded, is empty, or could
    /// not be assembled with its key by the TLS backend.
    #[error("client identity could not be assembled")]
    IdentityAssembly,
}

/// Source-scoped trust selection for [`TlsOptions`].
#[derive(Clone, Default)]
pub enum TrustRoots {
    /// The packaged pinned Node reference default CA bundle (the default).
    #[default]
    PinnedNodeDefault,
    /// The pinned default bundle plus explicitly supplied roots.
    PinnedNodeDefaultWithExtra(Vec<CertificateDer<'static>>),
    /// Caller-supplied roots replace the default bundle entirely. An empty
    /// set is rejected when the client is built.
    Explicit(Vec<CertificateDer<'static>>),
    /// Explicit opt-in to the host system root store. No other trust mode
    /// silently blends in.
    HostSystem,
}

impl std::fmt::Debug for TrustRoots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redacted: mode name and counts only, never certificate bytes.
        match self {
            Self::PinnedNodeDefault => f.write_str("PinnedNodeDefault"),
            Self::PinnedNodeDefaultWithExtra(extra) => f
                .debug_struct("PinnedNodeDefaultWithExtra")
                .field("extra_count", &extra.len())
                .finish(),
            Self::Explicit(roots) => f
                .debug_struct("Explicit")
                .field("root_count", &roots.len())
                .finish(),
            Self::HostSystem => f.write_str("HostSystem"),
        }
    }
}

/// Owned client certificate-chain/key identity for mutual TLS.
///
/// The key is admitted through the crate's private safe upstream signing
/// owner (profile-gated: unencrypted two-prime RSA-2048..8192 or P-521)
/// when the identity is set; the stored representation is reused to
/// assemble the TLS identity when the client is built.
pub struct ClientIdentity {
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
}

impl Clone for ClientIdentity {
    fn clone(&self) -> Self {
        Self {
            chain: self.chain.clone(),
            key: self.key.clone_key(),
        }
    }
}

impl std::fmt::Debug for ClientIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Redacted: chain length and key container classification only.
        let key_kind = match &self.key {
            PrivateKeyDer::Pkcs1(_) => "Pkcs1",
            PrivateKeyDer::Sec1(_) => "Sec1",
            PrivateKeyDer::Pkcs8(_) => "Pkcs8",
            _ => "other",
        };
        f.debug_struct("ClientIdentity")
            .field("chain_len", &self.chain.len())
            .field("key_kind", &key_kind)
            .finish()
    }
}

/// Source-qualified HTTP protocol selection expressed through ALPN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlpnPreference {
    /// Offer `h2` and `http/1.1` (in that order); negotiate with the peer.
    #[default]
    H2Preferred,
    /// Offer only `http/1.1`.
    Http1Only,
}

/// The one typed TLS options value accepted by
/// [`crate::http::HttpClientBuilder::tls`].
///
/// Defaults: pinned Node reference root bundle, no client identity,
/// `h2`-preferred ALPN. Construction performs no I/O and reads no ambient
/// state; every input is caller-supplied and validated when the client is
/// built.
#[derive(Debug, Clone, Default)]
pub struct TlsOptions {
    trust: TrustRoots,
    identity: Option<ClientIdentity>,
    alpn: AlpnPreference,
}

impl TlsOptions {
    /// Default options: pinned Node reference trust, no client identity,
    /// H2-preferred ALPN.
    pub fn new() -> Self {
        Self::default()
    }

    /// Extend the pinned default root bundle with additional roots supplied
    /// as PEM (one or more `CERTIFICATE` blocks).
    pub fn with_extra_roots_pem(mut self, pem: &[u8]) -> Result<Self, TlsConfigError> {
        let mut extra = match &mut self.trust {
            TrustRoots::PinnedNodeDefaultWithExtra(extra) => std::mem::take(extra),
            TrustRoots::PinnedNodeDefault => Vec::new(),
            TrustRoots::Explicit(_) | TrustRoots::HostSystem => {
                return Err(TlsConfigError::RootDecode);
            }
        };
        extra.extend(
            CertificateDer::pem_slice_iter(pem)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| TlsConfigError::RootDecode)?,
        );
        if extra.is_empty() {
            return Err(TlsConfigError::RootDecode);
        }
        self.trust = TrustRoots::PinnedNodeDefaultWithExtra(extra);
        Ok(self)
    }

    /// Replace the default bundle with explicit roots supplied as PEM (one
    /// or more `CERTIFICATE` blocks). An empty set is rejected.
    pub fn with_explicit_roots_pem(mut self, pem: &[u8]) -> Result<Self, TlsConfigError> {
        let roots: Vec<_> = CertificateDer::pem_slice_iter(pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| TlsConfigError::RootDecode)?;
        if roots.is_empty() {
            return Err(TlsConfigError::RootDecode);
        }
        self.trust = TrustRoots::Explicit(roots);
        Ok(self)
    }

    /// Explicitly select the host system root store. This is an explicit
    /// opt-in scoped to this client; it never becomes ambient behavior for
    /// other clients.
    pub fn use_host_system_roots(mut self) -> Self {
        self.trust = TrustRoots::HostSystem;
        self
    }

    /// Set the mutual-TLS client identity from a PEM certificate chain (one
    /// or more `CERTIFICATE` blocks, leaf first) and an unencrypted PEM
    /// private key. The key is admitted through the crate's profile-gated
    /// upstream signing owner immediately; unsupported keys (RSA below 2048
    /// bits, multi-prime RSA, non-P-521 curves), encrypted or malformed
    /// inputs fail typed here, before any I/O.
    pub fn with_client_identity_pem(
        mut self,
        chain_pem: &[u8],
        key_pem: &[u8],
    ) -> Result<Self, TlsConfigError> {
        let chain: Vec<_> = CertificateDer::pem_slice_iter(chain_pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| TlsConfigError::IdentityAssembly)?;
        if chain.is_empty() {
            return Err(TlsConfigError::IdentityAssembly);
        }
        // Admission through the private safe upstream owner: decode +
        // backend load + profile gates. The admitted classification is
        // retained only in the redacted classification above.
        crypto::SigningOwner::aws_lc()
            .load_private_key_pem(key_pem)
            .map_err(TlsKeyError::from)?;
        let key = PrivateKeyDer::from_pem_slice(key_pem)
            .map_err(|_| TlsConfigError::KeyAdmission(TlsKeyError::Decode))?;
        self.identity = Some(ClientIdentity { chain, key });
        Ok(self)
    }

    /// Restrict ALPN to `http/1.1` only.
    pub fn http1_only(mut self) -> Self {
        self.alpn = AlpnPreference::Http1Only;
        self
    }

    /// The configured trust mode (redacted view).
    pub fn trust_roots(&self) -> &TrustRoots {
        &self.trust
    }

    /// Whether a client identity is configured (redacted view).
    pub fn has_client_identity(&self) -> bool {
        self.identity.is_some()
    }

    /// The configured ALPN preference.
    pub fn alpn_preference(&self) -> AlpnPreference {
        self.alpn
    }
}

/// The frozen instance `aws-lc-rs` provider with the declared key-exchange
/// group order. Explicit instance only: nothing here is installed
/// process-globally.
pub(crate) fn provider_instance() -> Arc<CryptoProvider> {
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    provider.kx_groups = vec![X25519MLKEM768, X25519, SECP256R1, SECP384R1];
    Arc::new(provider)
}

/// A frozen, validated TLS configuration ready to hand to the HTTP backend.
pub(crate) struct FrozenTls {
    /// Preconfigured rustls client configuration (instance AWS-LC provider,
    /// frozen roots/identity/ALPN).
    pub(crate) config: rustls::ClientConfig,
    /// Whether the HTTP backend must restrict itself to HTTP/1.1.
    pub(crate) http1_only: bool,
}

impl TlsOptions {
    /// Validate and freeze the options into a backend configuration.
    ///
    /// This is the single pre-I/O freeze point: root decoding, host-store
    /// availability, identity assembly and the frozen profile are all
    /// resolved here, returning typed redacted failures.
    pub(crate) fn freeze(&self) -> Result<FrozenTls, TlsConfigError> {
        let mut roots = RootCertStore::empty();
        match &self.trust {
            TrustRoots::PinnedNodeDefault | TrustRoots::PinnedNodeDefaultWithExtra(_) => {
                for cert in CertificateDer::pem_slice_iter(
                    node_trust::NODE_ROOT_CERTIFICATES_PEM.as_bytes(),
                ) {
                    roots
                        .add(cert.map_err(|_| TlsConfigError::DefaultRootStore)?)
                        .map_err(|_| TlsConfigError::DefaultRootStore)?;
                }
                if roots.is_empty() {
                    return Err(TlsConfigError::DefaultRootStore);
                }
                if let TrustRoots::PinnedNodeDefaultWithExtra(extra) = &self.trust {
                    for cert in extra {
                        roots
                            .add(cert.clone())
                            .map_err(|_| TlsConfigError::RootDecode)?;
                    }
                }
            }
            TrustRoots::Explicit(explicit) => {
                if explicit.is_empty() {
                    return Err(TlsConfigError::RootDecode);
                }
                for cert in explicit {
                    roots
                        .add(cert.clone())
                        .map_err(|_| TlsConfigError::RootDecode)?;
                }
            }
            TrustRoots::HostSystem => {
                let native = rustls_native_certs::load_native_certs();
                if native.certs.is_empty() {
                    return Err(TlsConfigError::HostRootsUnavailable);
                }
                for cert in native.certs {
                    roots
                        .add(cert)
                        .map_err(|_| TlsConfigError::HostRootsUnavailable)?;
                }
            }
        }

        let builder = rustls::ClientConfig::builder_with_provider(provider_instance())
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .map_err(|_| TlsConfigError::IdentityAssembly)?
            .with_root_certificates(roots);
        let mut config = match &self.identity {
            Some(identity) => builder
                .with_client_auth_cert(identity.chain.clone(), identity.key.clone_key())
                .map_err(|_| TlsConfigError::IdentityAssembly)?,
            None => builder.with_no_client_auth(),
        };
        config.alpn_protocols = match self.alpn {
            AlpnPreference::H2Preferred => vec![b"h2".to_vec(), b"http/1.1".to_vec()],
            AlpnPreference::Http1Only => vec![b"http/1.1".to_vec()],
        };
        Ok(FrozenTls {
            config,
            http1_only: self.alpn == AlpnPreference::Http1Only,
        })
    }
}
