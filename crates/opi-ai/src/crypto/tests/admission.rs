//! Key admission profiles, encodings, typed rejection and redaction.

use super::super::{CryptoKeyError, CryptoSignError, fixtures, verify};
use super::{load_pem, owner};
use rustls::{SignatureAlgorithm, SignatureScheme};

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
        let err = load_pem(input).expect_err("declared out-of-profile key must not be admitted");
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
    let err =
        load_pem(fixtures::P256_PKCS8_PEM).expect_err("P-256 is outside the admitted EC profile");
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
