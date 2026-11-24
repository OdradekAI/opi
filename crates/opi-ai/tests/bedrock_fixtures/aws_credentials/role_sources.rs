//! AWS credential-source role sources tests.

use super::*;

#[tokio::test]
async fn ini_assume_role_wins_over_static_and_signs_source_profile_credentials() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "ROLE",
            "arn:aws:sts::123456789012:assumed-role/fixture/fixture-session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let (credentials, config) = write_role_profiles(dir.path());

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("assume-role resolves");

    assert_eq!(transport.send_count(), 1, "exactly one STS exchange");
    let send = recorded_send(&transport, 0);
    assert_eq!(send.method, SourceHttpMethod::Post);
    assert_eq!(send.url, "https://sts.us-west-2.amazonaws.com/");
    assert_eq!(
        header_of(&send, "Content-Type").unwrap(),
        "application/x-www-form-urlencoded"
    );
    let body = body_of(&send);
    for expected in [
        "Action=AssumeRole",
        "Version=2011-06-15",
        "RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Ffixture",
        "RoleSessionName=fixture-session",
        "ExternalId=fixture-external",
        "DurationSeconds=3600",
    ] {
        assert!(
            body.contains(expected),
            "form body missing {expected}: {body}"
        );
    }
    assert!(
        !body.contains("ROLE_OWN_STATIC"),
        "role profile's own static keys must not be sent: {body}"
    );
    assert_sigv4_signed_with(&send, "SOURCE_ACCESS");

    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::AssumeRole {
                profile: "role-prof".to_owned()
            }
        }
    );
    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "ROLEACCESS",
        "temporary credentials replace the source pair"
    );
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-06-01T12:00:00Z")),
        "STS expiration is retained"
    );
    assert_eq!(auth.account_id.as_deref(), Some("123456789012"));
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "process must not run for a resolved role"
    );
}

#[tokio::test]
async fn assume_role_requires_exactly_one_of_source_profile_or_credential_source() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[both]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\ncredential_source=Environment\n\
         [neither]\nrole_arn=arn:aws:iam::123456789012:role/fixture\n",
    )
    .unwrap();

    for profile in ["both", "neither"] {
        let input = chain_input(
            &ambient,
            Some(credentials.as_path()),
            None,
            &runner,
            AwsAuthSelection::Profile(profile.to_owned()),
            ScopedAwsEnv::empty(),
        )
        .with_source_transport(&transport);
        let error = resolve_local_aws_auth(&input)
            .await
            .expect_err("ambiguous role configuration continues and exhausts");
        assert!(
            matches!(error, CredentialResolutionError::Exhausted),
            "expected typed exhaustion, got {error:?}"
        );
    }
    assert_eq!(
        transport.send_count(),
        0,
        "no STS sends for ambiguous roles"
    );
}

#[tokio::test]
async fn nested_source_profiles_resolve_and_cycles_continue() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(
            200,
            sts_success_xml(
                "INNER",
                "arn:aws:sts::123456789012:assumed-role/inner/inner-session",
            ),
        ),
        source_response(
            200,
            sts_success_xml(
                "OUTER",
                "arn:aws:sts::123456789012:assumed-role/outer/outer-session",
            ),
        ),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[base]\naws_access_key_id=BASE_ACCESS\naws_secret_access_key=BASE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=inner\nregion=us-west-2\n\
         [profile inner]\nrole_arn=arn:aws:iam::123456789012:role/inner\nsource_profile=base\n\
         [profile loop-a]\nrole_arn=arn:aws:iam::123456789012:role/a\nsource_profile=loop-b\n\
         [profile loop-b]\nrole_arn=arn:aws:iam::123456789012:role/b\nsource_profile=loop-a\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("nested assume-role resolves");
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "OUTERACCESS");
    assert_eq!(transport.send_count(), 2, "inner then outer STS exchanges");
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "BASE_ACCESS");
    assert_sigv4_signed_with(&recorded_send(&transport, 1), "INNERACCESS");

    let cycling_ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let cycling = chain_input(
        &cycling_ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("loop-a".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&cycling)
        .await
        .expect_err("profile cycles are continue-eligible");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
    assert_eq!(transport.send_count(), 2, "cycles send nothing");
}

#[tokio::test]
async fn mfa_serial_without_presenter_is_typed_terminal() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nmfa_serial=arn:aws:iam::123456789012:mfa/fixture\ncredential_process=fixture-process\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("MFA without presenter must terminate");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::MfaPresenterMissing,
                ..
            }
        ),
        "expected MFA terminal, got {error:?}"
    );
    assert_eq!(transport.send_count(), 0);
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "terminal role failure must not fall through to credential_process"
    );
}

#[tokio::test]
async fn mfa_presenter_supplies_serial_number_and_token_code() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "MFA",
            "arn:aws:sts::123456789012:assumed-role/fixture/fixture-session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nmfa_serial=arn:aws:iam::123456789012:mfa/fixture\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport)
    .with_mfa(Some(&FixtureMfa));
    resolve_local_aws_auth(&input)
        .await
        .expect("MFA-presented assume-role resolves");

    let body = body_of(&recorded_send(&transport, 0));
    assert!(
        body.contains("SerialNumber=arn%3Aaws%3Aiam%3A%3A123456789012%3Amfa%2Ffixture"),
        "MFA serial must be encoded: {body}"
    );
    assert!(
        body.contains("TokenCode=123456"),
        "presenter token code: {body}"
    );
}

#[tokio::test]
async fn sts_denial_is_typed_terminal_with_code_and_request_id() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport =
        FakeSourceTransport::new(vec![source_response(403, sts_error_xml("AccessDenied"))]);
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\ncredential_process=fixture-process\nregion=us-west-2\n",
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("role-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("STS denial is terminal");
    let rendered = error.to_string();
    match error {
        CredentialResolutionError::Terminal { reason, .. } => match reason {
            TerminalReason::StsDenied {
                status,
                code,
                request_id,
            } => {
                assert_eq!(status, Some(403));
                assert_eq!(code.as_deref(), Some("AccessDenied"));
                assert_eq!(request_id.as_deref(), Some("sts-request-denied"));
            }
            other => panic!("expected StsDenied, got {other:?}"),
        },
        other => panic!("expected Terminal, got {other:?}"),
    }
    assert!(rendered.contains("AccessDenied"), "{rendered}");
    assert!(
        !rendered.contains("denied detail"),
        "raw STS message must not reach diagnostics: {rendered}"
    );
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "STS denial must not fall through"
    );
}

#[tokio::test]
async fn web_identity_from_ini_profile_reads_token_file_and_omits_source_sigv4() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        web_identity_success_xml("fixture-subject"),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("token.jwt");
    std::fs::write(&token_file, "fixture-web-identity-token\n").unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nweb_identity_token_file=REPLACED_BY_TEST\nregion=us-west-2\n",
    )
    .unwrap();
    // Token file path with an absolute path in the config.
    std::fs::write(
        &config,
        format!(
            "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nrole_session_name=web-session\nweb_identity_token_file={}\nregion=us-west-2\n",
            token_file.display()
        ),
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("web-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("web identity resolves");

    assert_eq!(transport.send_count(), 1);
    let send = recorded_send(&transport, 0);
    let body = body_of(&send);
    for expected in [
        "Action=AssumeRoleWithWebIdentity",
        "Version=2011-06-15",
        "RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Fweb",
        "RoleSessionName=web-session",
        "WebIdentityToken=fixture-web-identity-token",
    ] {
        assert!(body.contains(expected), "web identity body: {body}");
    }
    assert!(
        header_of(&send, "Authorization").is_none(),
        "web identity never signs with source SigV4"
    );
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::WebIdentity {
                route: WebIdentityRoute::Profile("web-prof".to_owned())
            }
        }
    );
    assert_eq!(auth.account_id.as_deref(), Some("123456789012"));
    assert!(
        runner.calls.lock().unwrap().is_empty(),
        "web identity precedes credential_process"
    );
}

#[tokio::test]
async fn web_identity_token_file_failure_is_terminal() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::default_never();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        format!(
            "[profile web-prof]\nrole_arn=arn:aws:iam::123456789012:role/web\nweb_identity_token_file={}\ncredential_process=fixture-process\nregion=us-west-2\n",
            dir.path().join("missing-token.jwt").display()
        ),
    )
    .unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("web-prof".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("unreadable token file is terminal");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::TokenFileUnreadable,
                ..
            }
        ),
        "got {error:?}"
    );
    assert_eq!(transport.send_count(), 0);
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ambient_web_identity_env_fields_exchange_after_ini_misses() {
    let token_dir = tempfile::tempdir().unwrap();
    let token_path = token_dir.path().join("ambient-token.jwt");
    std::fs::write(&token_path, "ambient-token").unwrap();
    let leaked_token_path: &'static str = Box::leak(
        token_path
            .to_str()
            .expect("utf-8 path")
            .to_owned()
            .into_boxed_str(),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_WEB_IDENTITY_TOKEN_FILE", leaked_token_path),
        ("AWS_ROLE_ARN", "arn:aws:iam::123456789012:role/env"),
        ("AWS_ROLE_SESSION_NAME", "env-session"),
        ("AWS_REGION", "us-west-2"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        web_identity_success_xml("env-subject"),
    )]);
    let dir = tempfile::tempdir().unwrap();
    // Default profile has only a region: the INI family misses, then the
    // chain-level ambient web identity segment runs.
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[default]\nregion=us-west-2\n").unwrap();

    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ambient web identity resolves");

    assert_eq!(transport.send_count(), 1);
    let body = body_of(&recorded_send(&transport, 0));
    assert!(body.contains("WebIdentityToken=ambient-token"), "{body}");
    assert!(body.contains("RoleSessionName=env-session"), "{body}");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::WebIdentity {
                route: WebIdentityRoute::Environment
            }
        }
    );

    // Missing role configuration is continue-eligible: no exchange, exhaustion.
    let ambient_missing = MapAuthContext(std::collections::HashMap::from([
        ("AWS_WEB_IDENTITY_TOKEN_FILE", "/nonexistent/token"),
        ("AWS_REGION", "us-west-2"),
        ("AWS_EC2_METADATA_DISABLED", "true"),
    ]));
    let transport_missing = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient_missing,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport_missing);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("incomplete ambient web identity continues");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
    assert_eq!(transport_missing.send_count(), 0);
}

#[tokio::test]
async fn nested_source_profiles_assume_from_inner_to_outer() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(
        ["INNER", "MIDDLE", "OUTER"]
            .into_iter()
            .map(|name| {
                source_response(
                    200,
                    sts_success_xml(
                        name,
                        &format!("arn:aws:sts::123456789012:assumed-role/{name}/session"),
                    ),
                )
            })
            .collect(),
    );
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[base]\naws_access_key_id=BASE_ACCESS\naws_secret_access_key=BASE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=middle\nregion=us-west-2\n\
         [profile middle]\nrole_arn=arn:aws:iam::123456789012:role/middle\nsource_profile=inner\n\
         [profile inner]\nrole_arn=arn:aws:iam::123456789012:role/inner\nsource_profile=base\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(&credentials),
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "OUTERACCESS"
    );
    assert_eq!(transport.send_count(), 3);
    for (index, (role, source_key)) in [
        ("inner", "BASE_ACCESS"),
        ("middle", "INNERACCESS"),
        ("outer", "MIDDLEACCESS"),
    ]
    .into_iter()
    .enumerate()
    {
        let request = recorded_send(&transport, index);
        assert!(body_of(&request).contains(&format!("role%2F{role}")));
        assert_sigv4_signed_with(&request, source_key);
    }
}

#[tokio::test]
async fn assume_role_accepts_process_source_profile() {
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
            "ROLE",
            "arn:aws:sts::123456789012:assumed-role/outer/session",
        ),
    )]);
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile outer]\nrole_arn=arn:aws:iam::123456789012:role/outer\nsource_profile=process\nregion=us-west-2\n\
         [profile process]\ncredential_process=synthetic-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("outer".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let resolved = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&resolved).access_key_id.expose_secret(),
        "ROLEACCESS"
    );
    assert_eq!(runner.calls.lock().unwrap().len(), 1);
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "PROCESS_ACCESS");
}
