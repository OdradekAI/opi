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
mod tests {
    use super::fixtures;
    use super::verify;
    use super::worker::spawn_sign_worker;
    use super::*;
    use rustls::SignatureScheme;
    use std::sync::atomic::AtomicBool;
    use std::task::Context;
    use std::time::Duration;
    use tokio::sync::oneshot;

    fn owner() -> SigningOwner {
        SigningOwner::aws_lc()
    }

    fn load_pem(pem: &str) -> Result<OwnedSigningKey, CryptoKeyError> {
        owner().load_private_key_pem(pem.as_bytes())
    }

    #[test]
    fn rsa_profile_accepts_two_prime_2048_through_8192_in_all_four_encodings() {
        // 2048 across PEM/DER x PKCS#1/PKCS#8 proves encoding-independent
        // admission; PKCS#1 v1.5 determinism proves the encodings describe
        // the same key, not merely any accepting path.
        let mut signatures = Vec::new();
        for input in [
            fixtures::RSA_2048_E65537_PKCS8_PEM,
            fixtures::RSA_2048_E65537_PKCS1_PEM,
        ] {
            let key = load_pem(input).expect("2048-bit two-prime key admitted");
            assert_eq!(key.algorithm(), SignatureAlgorithm::RSA);
            assert!(
                key.offered_schemes()
                    .contains(&SignatureScheme::RSA_PKCS1_SHA256)
            );
            signatures.push(key.sign_rs256(fixtures::MESSAGE_BIN).expect("native RS256"));
        }
        for der in [
            fixtures::RSA_2048_E65537_PKCS8_DER,
            fixtures::RSA_2048_E65537_PKCS1_DER,
        ] {
            let key = owner()
                .load_private_key_der(der)
                .expect("2048-bit DER key admitted");
            signatures.push(key.sign_rs256(fixtures::MESSAGE_BIN).expect("native RS256"));
        }
        let first = &signatures[0];
        assert!(
            signatures.iter().all(|sig| sig == first),
            "same key in different encodings must produce the identical PKCS#1 v1.5 signature"
        );
        assert_eq!(first.len(), 256);

        // 3072/4096/8192: modulus-sized signatures, upper profile bound.
        let key_3072 = load_pem(fixtures::RSA_3072_E65537_PKCS8_PEM).expect("3072 admitted");
        let sig_3072 = key_3072
            .sign_rs256(fixtures::MESSAGE_BIN)
            .expect("native RS256");
        assert_eq!(sig_3072.len(), 384);
        let key_4096 = load_pem(fixtures::RSA_4096_E65537_PKCS8_PEM).expect("4096 admitted");
        let sig_4096 = key_4096
            .sign_rs256(fixtures::MESSAGE_BIN)
            .expect("native RS256");
        assert_eq!(sig_4096.len(), 512);
        let key_8192 = load_pem(fixtures::RSA_8192_E65537_PKCS8_PEM).expect("8192 admitted");
        let sig_8192 = key_8192
            .sign_rs256(fixtures::MESSAGE_BIN)
            .expect("native RS256");
        assert_eq!(sig_8192.len(), 1024);
    }

    #[test]
    fn rsa_e3_small_exponent_keys_are_admitted() {
        let from_pem = load_pem(fixtures::RSA_2048_E3_PKCS8_PEM).expect("e3 PKCS#8 PEM admitted");
        let from_der = owner()
            .load_private_key_der(fixtures::RSA_2048_E3_PKCS1_DER)
            .expect("e3 PKCS#1 DER admitted");
        let a = from_pem
            .sign_rs256(fixtures::MESSAGE_BIN)
            .expect("native RS256 with e=3");
        let b = from_der
            .sign_rs256(fixtures::MESSAGE_BIN)
            .expect("native RS256 with e=3");
        assert_eq!(a, b);
        assert_eq!(a.len(), 256);
    }

    #[test]
    fn p521_profile_accepts_pkcs8_and_sec1_in_pem_and_der() {
        for input in [fixtures::P521_PKCS8_PEM, fixtures::P521_SEC1_PEM] {
            let key = load_pem(input).expect("P-521 PEM key admitted");
            assert_eq!(key.algorithm(), SignatureAlgorithm::ECDSA);
            assert_eq!(
                key.offered_schemes(),
                &[SignatureScheme::ECDSA_NISTP521_SHA512]
            );
            let sig = key
                .sign_p521_sha512(fixtures::MESSAGE_BIN)
                .expect("native P-521 signature");
            assert!(
                (130..=141).contains(&sig.len()),
                "ASN.1 DER ECDSA signature"
            );
            verify::verify_ecdsa_p521_sha512(
                fixtures::P521_PUBLIC_POINT_UNCOMPRESSED,
                fixtures::MESSAGE_BIN,
                &sig,
            )
            .expect("independent P-521 verification");
        }
        for der in [fixtures::P521_PKCS8_DER, fixtures::P521_SEC1_DER] {
            let key = owner()
                .load_private_key_der(der)
                .expect("P-521 DER key admitted");
            let sig = key
                .sign_p521_sha512(fixtures::MESSAGE_BIN)
                .expect("native P-521 signature");
            verify::verify_ecdsa_p521_sha512(
                fixtures::P521_PUBLIC_POINT_UNCOMPRESSED,
                fixtures::MESSAGE_BIN,
                &sig,
            )
            .expect("independent P-521 verification");
        }
    }

    #[test]
    fn rsa_1024_and_multi_prime_keys_are_rejected_at_the_backend_load_seam() {
        for input in [
            fixtures::RSA_1024_E65537_PKCS8_PEM,
            fixtures::RSA_2048_THREEPRIME_PKCS8_PEM,
            fixtures::RSA_2048_THREEPRIME_PKCS1_PEM,
        ] {
            let err =
                load_pem(input).expect_err("declared out-of-profile key must not be admitted");
            assert!(matches!(err, CryptoKeyError::BackendLoad), "got {err:?}");
        }
        let err = owner()
            .load_private_key_der(fixtures::RSA_1024_E65537_PKCS1_DER)
            .expect_err("RSA-1024 DER must not be admitted");
        assert!(matches!(err, CryptoKeyError::BackendLoad), "got {err:?}");
    }

    #[test]
    fn encrypted_truncated_malformed_and_public_only_inputs_never_admit() {
        for input in [
            fixtures::ENCRYPTED_PKCS8_PEM,
            fixtures::TRUNCATED_PKCS8_PEM,
            fixtures::MALFORMED_PRIVATE_PEM,
            fixtures::WRONGTYPE_PUBLIC_PEM,
            "",
        ] {
            load_pem(input).expect_err("invalid input must fail closed");
        }
        for der in [
            fixtures::TRUNCATED_PKCS8_DER,
            fixtures::MALFORMED_PRIVATE_DER,
            b"" as &[u8],
        ] {
            owner()
                .load_private_key_der(der)
                .expect_err("invalid DER input must fail closed");
        }
    }

    #[test]
    fn non_profile_curves_and_wrong_algorithm_requests_are_typed_failures() {
        // P-256 loads and classifies as ECDSA but offers no profile scheme.
        let err = load_pem(fixtures::P256_PKCS8_PEM)
            .expect_err("P-256 is outside the admitted EC profile");
        assert!(
            matches!(err, CryptoKeyError::UnsupportedScheme(_)),
            "got {err:?}"
        );

        // Requesting a foreign scheme on an admitted key never silently
        // downgrades to another algorithm or scheme.
        let rsa = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
        assert!(matches!(
            rsa.sign(
                SignatureScheme::ECDSA_NISTP521_SHA512,
                fixtures::MESSAGE_BIN
            ),
            Err(CryptoSignError::SchemeNotOffered)
        ));
        let p521 = load_pem(fixtures::P521_PKCS8_PEM).expect("p521 admitted");
        assert!(matches!(
            p521.sign(SignatureScheme::RSA_PKCS1_SHA256, fixtures::MESSAGE_BIN),
            Err(CryptoSignError::SchemeNotOffered)
        ));
    }

    #[test]
    fn independent_verification_accepts_valid_and_rejects_tampered_signatures() {
        let key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
        let sig = key.sign_rs256(fixtures::MESSAGE_BIN).expect("native RS256");
        verify::verify_rsa_pkcs1_sha256(
            fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
            fixtures::MESSAGE_BIN,
            &sig,
        )
        .expect("valid signature verifies independently");

        // Tampered signature.
        let mut bad = sig.clone();
        bad[0] ^= 0xff;
        assert!(
            verify::verify_rsa_pkcs1_sha256(
                fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
                fixtures::MESSAGE_BIN,
                &bad,
            )
            .is_err()
        );
        // Tampered message.
        let mut msg = fixtures::MESSAGE_BIN.to_vec();
        msg[0] ^= 0xff;
        assert!(verify::verify_rsa_pkcs1_sha256(
            fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
            &msg,
            &sig,
        )
        .is_err());
        // Wrong key (independent 3072-bit public key).
        assert!(
            verify::verify_rsa_pkcs1_sha256(
                fixtures::RSA_3072_E65537_PUBLIC_PKCS1_DER,
                fixtures::MESSAGE_BIN,
                &sig,
            )
            .is_err()
        );
        // Malformed public key fails closed.
        assert!(
            verify::verify_rsa_pkcs1_sha256(b"not-a-key", fixtures::MESSAGE_BIN, &sig,).is_err()
        );

        let p521 = load_pem(fixtures::P521_PKCS8_PEM).expect("p521 admitted");
        let sig = p521
            .sign_p521_sha512(fixtures::MESSAGE_BIN)
            .expect("native P-521");
        let mut bad = sig.clone();
        let last = bad.len() - 1;
        bad[last] ^= 0xff;
        assert!(
            verify::verify_ecdsa_p521_sha512(
                fixtures::P521_PUBLIC_POINT_UNCOMPRESSED,
                fixtures::MESSAGE_BIN,
                &bad,
            )
            .is_err()
        );
    }

    #[test]
    fn admission_errors_never_expose_key_material() {
        // A canary run from the accepted PKCS#8 PEM body: if any error text
        // ever echoes input material, this substring appears.
        let body = fixtures::RSA_2048_E65537_PKCS8_PEM
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<String>();
        let canary = &body[20..80];
        let rejections = [
            load_pem(fixtures::ENCRYPTED_PKCS8_PEM).unwrap_err(),
            load_pem(fixtures::TRUNCATED_PKCS8_PEM).unwrap_err(),
            load_pem(fixtures::MALFORMED_PRIVATE_PEM).unwrap_err(),
            load_pem(fixtures::WRONGTYPE_PUBLIC_PEM).unwrap_err(),
            load_pem(fixtures::RSA_1024_E65537_PKCS8_PEM).unwrap_err(),
            load_pem(fixtures::RSA_2048_THREEPRIME_PKCS8_PEM).unwrap_err(),
            load_pem(fixtures::P256_PKCS8_PEM).unwrap_err(),
        ];
        for err in &rejections {
            let text = format!("{err} {err:?}");
            assert!(
                !text.contains("PRIVATE"),
                "PEM markers must not leak: {text}"
            );
            assert!(!text.contains(canary), "key material must not leak");
        }
    }

    #[tokio::test]
    async fn worker_signing_publishes_a_verifiable_signature() {
        let key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
        let signature = key
            .sign_on_worker(
                SignatureScheme::RSA_PKCS1_SHA256,
                fixtures::MESSAGE_BIN.to_vec(),
            )
            .await
            .expect("worker signing publishes after native completion");
        verify::verify_rsa_pkcs1_sha256(
            fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
            fixtures::MESSAGE_BIN,
            &signature,
        )
        .expect("published signature verifies independently");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn caller_drop_retains_owned_key_until_worker_joins_and_suppresses_late_results() {
        let key = load_pem(fixtures::RSA_8192_E65537_PKCS8_PEM).expect("8192 admitted");
        let weak = key.weak_key();
        let mut op = key.sign_on_worker(
            SignatureScheme::RSA_PKCS1_SHA256,
            fixtures::MESSAGE_BIN.to_vec(),
        );
        drop(key);
        // Poll once so the blocking worker exists and owns its key clone.
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let _ = std::pin::Pin::new(&mut op).poll(&mut cx);
        // Cancel between operations. The native call in flight (if any) is
        // not preempted; the owned key stays live until the worker joins.
        drop(op);
        tokio::time::timeout(Duration::from_secs(20), async {
            while weak.upgrade().is_some() {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("worker joins after caller cancellation");
        // Nothing was published: the only receiver was dropped with the
        // operation, and the weak reference dying proves the worker returned.
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancellation_before_the_native_call_skips_it_entirely() {
        let key = load_pem(fixtures::RSA_8192_E65537_PKCS8_PEM).expect("8192 admitted");
        let weak = key.weak_key();
        // Pre-set cancellation: the worker observes it between operations and
        // must neither run the native call nor send anything.
        let cancelled = std::sync::Arc::new(AtomicBool::new(true));
        let (tx, mut rx) = oneshot::channel();
        let worker = spawn_sign_worker(
            key.key.clone(),
            SignatureScheme::RSA_PKCS1_SHA256,
            fixtures::MESSAGE_BIN.to_vec(),
            cancelled,
            tx,
        );
        drop(key);
        worker.await.expect("worker joins");
        assert!(
            matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)),
            "cancelled worker sends nothing"
        );
        assert!(weak.upgrade().is_none(), "owned key released after join");
    }
}
