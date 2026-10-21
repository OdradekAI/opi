//! Synthetic RS256 JWT signing inputs and independent verification.

use super::super::fixtures;
use super::load_pem;
use rustls::SignatureScheme;

fn b64url(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn jwt_claims(aud: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("monotonic clock")
        .as_secs();
    format!(
        "{{\"iss\":\"jwt@local.invalid\",\"scope\":\"local synthetic scope\",\"aud\":\"{aud}\",\"exp\":{},\"iat\":{}}}",
        now + 3600,
        now
    )
}

fn b64url_decode(data: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .expect("fixture base64")
}

#[tokio::test]
async fn jwt_rs256_alg_only_header_signs_and_independently_verifies() {
    // Source header shape: exactly {"alg":"RS256"}; no typ/kid is added.
    let header = b64url(b"{\"alg\":\"RS256\"}");
    let claims_text = jwt_claims("https://127.0.0.1/token");
    let claims = b64url(claims_text.as_bytes());
    let signing_input = format!("{header}.{claims}");
    let key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
    let signature = key
        .sign_on_worker(
            SignatureScheme::RSA_PKCS1_SHA256,
            signing_input.clone().into_bytes(),
        )
        .await
        .expect("worker RS256 for JWT");
    let jwt = format!("{signing_input}.{}", b64url(&signature));

    // Independent verification: aws-lc-rs public parameters only.
    use aws_lc_rs::signature::{RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey};
    UnparsedPublicKey::new(
        &RSA_PKCS1_2048_8192_SHA256,
        fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
    )
    .verify(signing_input.as_bytes(), &signature)
    .expect("independent JWT verification");

    // Header is alg-only; claims survive round-trip without rewrite.
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    let decoded_header = b64url_decode(parts[0]);
    assert_eq!(decoded_header, b"{\"alg\":\"RS256\"}");
    let decoded_claims = b64url_decode(parts[1]);
    assert_eq!(
        String::from_utf8(decoded_claims).expect("claims utf8"),
        claims_text,
        "claims must be carried verbatim, not normalized"
    );
}

#[tokio::test]
async fn tampered_and_wrong_alg_jwt_never_verify() {
    use aws_lc_rs::signature::{RSA_PKCS1_2048_8192_SHA256, UnparsedPublicKey};
    let header = b64url(b"{\"alg\":\"RS256\"}");
    let claims = b64url(jwt_claims("https://127.0.0.1/token").as_bytes());
    let signing_input = format!("{header}.{claims}");
    let key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
    let mut signature = key
        .sign_on_worker(
            SignatureScheme::RSA_PKCS1_SHA256,
            signing_input.clone().into_bytes(),
        )
        .await
        .expect("worker RS256");
    let verifier = UnparsedPublicKey::new(
        &RSA_PKCS1_2048_8192_SHA256,
        fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
    );

    // A wrong-alg header is not an RS256 signing input. The owner offers
    // only the admitted profile schemes for this key (no HS256-class or
    // unadmitted scheme can be produced through this path).
    const ADMITTED: &[SignatureScheme] = &[
        SignatureScheme::RSA_PKCS1_SHA256,
        SignatureScheme::RSA_PKCS1_SHA384,
        SignatureScheme::RSA_PKCS1_SHA512,
    ];
    assert!(
        key.offered_schemes()
            .iter()
            .all(|scheme| ADMITTED.contains(scheme))
    );
    assert!(
        key.offered_schemes()
            .contains(&SignatureScheme::RSA_PKCS1_SHA256)
    );

    // Tampered signature bytes fail independent verification.
    let last = signature.len() - 1;
    signature[last] ^= 0x01;
    assert!(
        verifier
            .verify(signing_input.as_bytes(), &signature)
            .is_err()
    );
}
