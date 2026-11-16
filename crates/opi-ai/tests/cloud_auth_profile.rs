use opi_ai::auth::AwsSigV4Credentials;
use opi_ai::auth::aws::{endpoint, sigv4};
use secrecy::{ExposeSecret, SecretString};

#[test]
fn cloud_auth_owner_exposes_source_and_signing_contracts() {
    assert_eq!(
        endpoint::sts_endpoint_url("cn-north-1", None),
        "https://sts.cn-north-1.amazonaws.com.cn"
    );
    assert_eq!(
        endpoint::sts_endpoint_url("us-east-1", Some(" http://127.0.0.1:9 ")),
        "http://127.0.0.1:9"
    );

    let credentials = AwsSigV4Credentials::new(
        SecretString::from("AKIDEXAMPLE"),
        SecretString::from("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY"),
        "us-east-1",
    );
    let signed = sigv4::sign_request(
        "POST",
        "/",
        "Action=GetCallerIdentity&Version=2011-06-15",
        &[("host", "sts.us-east-1.amazonaws.com")],
        b"",
        &credentials,
        "sts",
        "20150830",
        "20150830T123600Z",
    );
    assert!(
        signed
            .authorization
            .expose_secret()
            .starts_with("AWS4-HMAC-SHA256")
    );
    assert_eq!(signed.x_amz_content_sha256.len(), 64);
}
