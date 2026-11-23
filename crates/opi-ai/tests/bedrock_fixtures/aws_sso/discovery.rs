//! AWS SSO discovery tests.

use super::*;

#[tokio::test]
async fn explicit_sso_absent_continues_to_ini_static() {
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[default]\naws_access_key_id = INI_ACCESS\naws_secret_access_key = INI_SECRET\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        Some(&credentials),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::ProfileFile);
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn explicit_sso_incomplete_is_terminal_with_zero_sends() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv {
            sso_start_url: Some("https://start.example".to_owned()),
            ..ScopedAwsEnv::empty()
        },
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete explicit SSO selection terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::IncompleteSsoConfiguration,
            ..
        } => {}
        other => panic!("expected incomplete SSO terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn explicit_sso_resolves_with_getrole_wire() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let auth = resolve_local_aws_auth(&input).await.expect("SSO resolves");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!("expected SigV4 provenance");
    };
    assert_eq!(source, AwsCredentialSource::Sso { profile: None });
    assert_eq!(transport.send_count(), 1);
    let send = transport.send_at(0);
    assert_eq!(
        send.method,
        opi_ai::auth::aws::source_http::SourceHttpMethod::Get
    );
    assert_eq!(
        send.url,
        "https://portal.sso.us-east-1.amazonaws.com/federation/credentials?role_name=Engineering&account_id=123456789012"
    );
    assert_eq!(bearer_of(&send), "SSO_TOKEN");
}

#[tokio::test]
async fn sso_shared_cache_keeps_roles_distinct() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(
            200,
            role_credentials_body(1_900_000_000_000).replace("SSO_AKID", "OTHER_ROLE_AKID"),
        ),
    ]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    for (role, expected_key) in [
        ("Engineering", "SSO_AKID"),
        ("OtherRole", "OTHER_ROLE_AKID"),
    ] {
        let mut scoped = sso_scoped("https://start.example", Some("sess-a"));
        scoped.sso_role_name = Some(role.to_owned());
        let input = chain_input(
            &ambient,
            None,
            None,
            &runner,
            AwsAuthSelection::Auto,
            scoped,
        )
        .with_source_transport(&transport)
        .with_source_cache(Some(&cache))
        .with_clock(&clock)
        .with_dpop_jti(&jti)
        .with_sso_cache_dir(Some(dir.path()));
        let resolved = resolve_local_aws_auth(&input).await.unwrap();
        assert_eq!(
            sigv4_of(&resolved).access_key_id.expose_secret(),
            expected_key
        );
    }
    assert_eq!(transport.send_count(), 2);
    assert!(transport.send_at(0).url.contains("role_name=Engineering"));
    assert!(transport.send_at(1).url.contains("role_name=OtherRole"));
}

#[tokio::test]
async fn assume_role_accepts_sso_source_profile() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=sso\nregion=us-west-2\n\
         [profile sso]\nsso_session=sess-a\nsso_account_id=123456789012\nsso_role_name=Engineering\n\
         [sso-session sess-a]\nsso_start_url=https://start.example\nsso_region=us-east-1\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(
            200,
            sts_success_xml(
                "OUTER",
                "arn:aws:sts::123456789012:assumed-role/outer/session",
            ),
        ),
    ]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_sso_cache_dir(Some(dir.path()))
    .with_clock(&clock);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 2);
    assert!(transport.send_at(0).url.contains("/federation/credentials"));
    assert_sigv4_signed_with(&transport.send_at(1), "SSO_AKID");
    assert!(
        transport
            .send_at(1)
            .url
            .contains("sts.us-west-2.amazonaws.com")
    );
    let request = transport.send_at(1);
    let authorization = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .unwrap()
        .1
        .as_str();
    assert!(authorization.contains("/us-west-2/sts/aws4_request"));
}

#[tokio::test]
async fn assume_role_accepts_login_source_profile() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=login\nregion=us-east-1\n\
         [profile login]\nlogin_session=my-session\n",
    )
    .unwrap();
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(
            r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#,
        ),
        Some("2030-01-01T04:00:00Z"),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "OUTER",
            "arn:aws:sts::123456789012:assumed-role/outer/session",
        ),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_login_cache_dir(Some(dir.path()))
    .with_clock(&clock);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&transport.send_at(0), "LOGIN_AKID");
}

#[tokio::test]
async fn ini_modern_sso_resolves_via_sso_session_section() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\nsso_session = sess-a\nsso_account_id = 123456789012\nsso_role_name = Engineering\n\
         [sso-session sess-a]\nsso_start_url = https://start.example\nsso_region = us-east-1\n",
    )
    .unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::Sso {
            profile: Some("corp".to_owned())
        }
    );
}

#[tokio::test]
async fn ini_sso_incomplete_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nsso_account_id = 123456789012\n").unwrap();
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete profile SSO terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::IncompleteSsoConfiguration,
            ..
        } => {}
        other => panic!("expected incomplete SSO terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn sso_getrole_denial_is_typed_terminal_with_zero_generation_sends() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(403, "{}")]);
    let cache = AwsSourceCache::new();
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let jti = CountingJti(std::sync::atomic::AtomicUsize::new(0));
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://start.example", Some("sess-a")),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("selected SSO denial never discovers another identity");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::SsoDenied {
                status: Some(403), ..
            },
            ..
        } => {}
        other => panic!("expected SSO denial terminal, got {other:?}"),
    }
    assert_eq!(
        transport.send_count(),
        1,
        "exactly the denied federation GET"
    );
}
