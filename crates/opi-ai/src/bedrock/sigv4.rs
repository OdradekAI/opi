//! AWS Signature Version 4 signing for Bedrock requests.
//!
//! Implements the SigV4 algorithm for signing HTTP requests to AWS services.
//! Uses HMAC-SHA256 throughout. No live AWS dependency.

use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// AWS credentials for SigV4 signing.
pub type AwsCredentials = crate::auth::AwsSigV4Credentials;

/// Result of signing a request.
#[derive(Clone)]
pub struct SignedRequest {
    /// The Authorization header value.
    pub authorization: SecretString,
    /// The X-Amz-Date header value (ISO 8601 basic: YYYYMMDDTHHmmssZ).
    pub x_amz_date: String,
    /// The X-Amz-Security-Token header (present when session token is set).
    pub x_amz_security_token: Option<SecretString>,
    /// The SHA-256 hash of the payload (hex encoded).
    pub x_amz_content_sha256: String,
}

impl std::fmt::Debug for SignedRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignedRequest")
            .field("authorization", &"<redacted>")
            .field("x_amz_date", &self.x_amz_date)
            .field(
                "x_amz_security_token",
                &self.x_amz_security_token.as_ref().map(|_| "<redacted>"),
            )
            .field("x_amz_content_sha256", &self.x_amz_content_sha256)
            .finish()
    }
}

/// Sign an HTTP request using AWS SigV4.
///
/// `date_stamp` is `YYYYMMDD`, `amz_date` is `YYYYMMDDTHHmmssZ`.
/// All parameters are explicit so tests can use fixed values.
#[allow(clippy::too_many_arguments)]
pub fn sign_request(
    method: &str,
    path: &str,
    query: &str,
    header_pairs: &[(&str, &str)],
    payload: &[u8],
    credentials: &AwsCredentials,
    service: &str,
    date_stamp: &str,
    amz_date: &str,
) -> SignedRequest {
    let payload_hash = sha256_hex(payload);
    let (signed_headers, canonical_headers) =
        canonicalize_headers(header_pairs, &credentials.session_token);

    let canonical_request =
        format!("{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}");

    let credential_scope = format!("{date_stamp}/{}/{service}/aws4_request", credentials.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    let signing_key = get_signing_key(
        credentials.secret_access_key.expose_secret(),
        date_stamp,
        &credentials.region,
        service,
    );
    let signature = hex_encode(&hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
        credentials.access_key_id.expose_secret(),
    );

    SignedRequest {
        authorization: SecretString::from(auth_header),
        x_amz_date: amz_date.to_string(),
        x_amz_security_token: credentials.session_token.clone(),
        x_amz_content_sha256: payload_hash,
    }
}

/// Build canonical header string and signed headers list.
fn canonicalize_headers(
    headers: &[(&str, &str)],
    session_token: &Option<SecretString>,
) -> (String, String) {
    let mut all_headers: Vec<(&str, &str)> = headers.to_vec();
    // Session token is added as a header if present
    if let Some(token) = session_token {
        all_headers.push(("x-amz-security-token", token.expose_secret()));
    }
    // Sort by lowercase header name
    all_headers.sort_by_key(|a| a.0.to_lowercase());

    let canonical: String = all_headers
        .iter()
        .map(|(k, v)| format!("{}:{}", k.to_lowercase(), normalize_header_value(v)))
        .collect::<Vec<_>>()
        .join("\n");
    let signed: String = all_headers
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");

    (signed, canonical)
}

fn normalize_header_value(value: &str) -> String {
    value
        .trim_matches([' ', '\t'])
        .split([' ', '\t'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is valid");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex_encode(&hasher.finalize())
}

fn get_signing_key(secret_key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(
        format!("AWS4{secret_key}").as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn hex_encode(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    // AWS SigV4 test vector from the official documentation.
    // See: https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_sigv.html

    #[test]
    fn sha256_empty_string() {
        let hash = sha256_hex(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hello_world() {
        let hash = sha256_hex(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn signing_key_derivation() {
        let key = get_signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        let expected_hex = "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9";
        assert_eq!(hex_encode(&key), expected_hex);
    }

    #[test]
    fn sign_get_request_no_session_token() {
        let creds = AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
            region: "us-east-1".into(),
        };

        let result = sign_request(
            "GET",
            "/",
            "",
            &[("host", "example.amazonaws.com")],
            b"",
            &creds,
            "service",
            "20150830",
            "20150830T123600Z",
        );

        assert!(
            result
                .authorization
                .expose_secret()
                .starts_with("AWS4-HMAC-SHA256")
        );
        assert!(
            result
                .authorization
                .expose_secret()
                .contains("Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request")
        );
        assert!(
            result
                .authorization
                .expose_secret()
                .contains("SignedHeaders=host")
        );
        assert_eq!(result.x_amz_date, "20150830T123600Z");
        assert!(result.x_amz_security_token.is_none());
    }

    #[test]
    fn sign_request_with_session_token() {
        let creds = AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: Some("session-token-123".into()),
            region: "us-east-1".into(),
        };

        let result = sign_request(
            "POST",
            "/model/anthropic.claude-sonnet-4-20250514-v2:0/converse-stream",
            "",
            &[
                ("host", "bedrock-runtime.us-east-1.amazonaws.com"),
                ("content-type", "application/json"),
            ],
            b"{\"messages\":[]}",
            &creds,
            "bedrock",
            "20250526",
            "20250526T120000Z",
        );

        assert!(
            result
                .authorization
                .expose_secret()
                .contains("SignedHeaders=")
        );
        assert!(
            result
                .authorization
                .expose_secret()
                .contains("x-amz-security-token")
        );
        assert_eq!(
            result
                .x_amz_security_token
                .as_ref()
                .map(ExposeSecret::expose_secret),
            Some("session-token-123")
        );
    }

    #[test]
    fn payload_hash_is_sha256_hex() {
        let creds = AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "secret".into(),
            session_token: None,
            region: "us-east-1".into(),
        };

        let result = sign_request(
            "POST",
            "/",
            "",
            &[("host", "example.com")],
            b"test payload",
            &creds,
            "bedrock",
            "20250526",
            "20250526T120000Z",
        );

        assert_eq!(result.x_amz_content_sha256, sha256_hex(b"test payload"));
    }

    #[test]
    fn canonicalize_headers_sorts_case_insensitive() {
        let (signed, canonical) = canonicalize_headers(
            &[
                ("Content-Type", "application/json"),
                ("Host", "example.com"),
            ],
            &None,
        );
        assert_eq!(signed, "content-type;host");
        assert!(canonical.starts_with("content-type:application/json\n"));
    }

    #[test]
    fn sign_request_collapses_sequential_spaces_and_tabs_in_header_values() {
        let credentials = AwsCredentials {
            access_key_id: "AKIDEXAMPLE".into(),
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY".into(),
            session_token: None,
            region: "us-east-1".into(),
        };
        let payload = br#"{"messages":[]}"#;
        let payload_hash = sha256_hex(payload);
        let signed = sign_request(
            "POST",
            "/model/test/converse-stream",
            "",
            &[
                ("host", "bedrock-runtime.us-east-1.amazonaws.com"),
                ("content-type", "application/json"),
                ("x-amz-content-sha256", &payload_hash),
                ("x-amz-date", "20250815T123456Z"),
                ("x-opi-custom", " \talpha   beta\t\tgamma  "),
            ],
            payload,
            &credentials,
            "bedrock",
            "20250815",
            "20250815T123456Z",
        );

        assert_eq!(
            signed.authorization.expose_secret(),
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20250815/us-east-1/bedrock/aws4_request, SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date;x-opi-custom, Signature=e6fa97332c5c3f3c9e8ca56062e2626ea4909ad8fa636260ffa90e35ffd6003d"
        );
    }
}
