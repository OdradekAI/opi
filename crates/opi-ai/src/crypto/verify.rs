//! Independent public-key signature verification.
//!
//! Verification uses the AWS-LC public verification parameters (a separate
//! path from the rustls signing seams) over caller-supplied public key
//! material, so signing regressions prove round trips without trusting the
//! signer's own intermediate state. Malformed keys, wrong keys and tampered
//! messages/signatures all fail closed with one redacted error; no key or
//! message material is retained.

use aws_lc_rs::signature::{ECDSA_P521_SHA512_ASN1, RSA_PKCS1_2048_8192_SHA256};

/// Redacted verification failure. Never echoes key, message or signature
/// bytes.
#[derive(Debug, thiserror::Error)]
#[error("signature verification failed")]
pub(crate) struct CryptoVerifyError;

/// Verifies an RSA PKCS#1 v1.5 SHA-256 signature against a DER-encoded
/// ASN.1 `RSAPublicKey` (RFC 3447 A.1.1) structure.
pub(crate) fn verify_rsa_pkcs1_sha256(
    public_key_pkcs1_der: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), CryptoVerifyError> {
    let key = aws_lc_rs::signature::UnparsedPublicKey::new(
        &RSA_PKCS1_2048_8192_SHA256,
        public_key_pkcs1_der,
    );
    key.verify(message, signature)
        .map_err(|_| CryptoVerifyError)
}

/// Verifies an ECDSA P-521 SHA-512 signature (ASN.1 DER encoded, as produced
/// by the rustls signing seam) against an uncompressed P-521 public point
/// (`0x04 || X || Y`).
pub(crate) fn verify_ecdsa_p521_sha512(
    public_point_uncompressed: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), CryptoVerifyError> {
    let key = aws_lc_rs::signature::UnparsedPublicKey::new(
        &ECDSA_P521_SHA512_ASN1,
        public_point_uncompressed,
    );
    key.verify(message, signature)
        .map_err(|_| CryptoVerifyError)
}
