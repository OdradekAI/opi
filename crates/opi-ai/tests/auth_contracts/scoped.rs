use super::*;

#[test]
fn scoped_credential_values_and_resolved_snapshot_are_owned_and_redacted() {
    use opi_ai::ApiKeyCredential;
    const KEY: &str = "api-key-snapshot-canary";
    const VALUE: &str = "env-snapshot-canary";
    let env = std::collections::BTreeMap::from([
        ("SECRET_CONFIG".into(), SecretString::from(VALUE)),
        ("EMPTY".into(), SecretString::from("")),
    ]);
    let credential = ApiKeyCredential::new(Some(SecretString::from(KEY)), env.clone());
    assert_eq!(credential.key().unwrap().expose_secret(), KEY);
    assert_eq!(credential.env()["EMPTY"].expose_secret(), "");
    assert!(!credential.env().contains_key("ABSENT"));
    let mut resolved = opi_ai::test_support::resolved_auth();
    resolved.env = credential.env().clone();
    let frozen = resolved.clone();
    resolved
        .env
        .insert("SECRET_CONFIG".into(), SecretString::from("changed"));
    assert_eq!(frozen.env["SECRET_CONFIG"].expose_secret(), VALUE);
    for debug in [
        format!("{credential:?}"),
        format!("{frozen:?}"),
        format!("{:?}", Credential::ApiKey(credential)),
    ] {
        assert!(!debug.contains(KEY));
        assert!(!debug.contains(VALUE));
    }
    assert!(ApiKeyCredential::new(None, env).key().is_none());
    assert_eq!(
        ApiKeyCredential::from(SecretString::from(""))
            .into_key()
            .unwrap()
            .expose_secret(),
        ""
    );
}
