//! Independent verification of valid and tampered signatures.

use super::super::{fixtures, verify};
use super::load_pem;

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
    assert!(
        verify::verify_rsa_pkcs1_sha256(fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER, &msg, &sig,)
            .is_err()
    );
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
    assert!(verify::verify_rsa_pkcs1_sha256(b"not-a-key", fixtures::MESSAGE_BIN, &sig,).is_err());

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
