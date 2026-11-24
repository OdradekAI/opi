//! AWS credential-source metadata tests.

use super::*;

#[tokio::test]
async fn credential_source_routes_environment_ecs_and_imds_sources() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        (
            "AWS_CONTAINER_CREDENTIALS_FULL_URI",
            "http://169.254.170.2/v2/creds",
        ),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(&credentials, "").unwrap();

    // Environment credential_source: the ambient pair signs the exchange.
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile env-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Environment\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        sts_success_xml(
            "ENVROLE",
            "arn:aws:sts::123456789012:assumed-role/fixture/s",
        ),
    )]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("env-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("environment credential_source resolves");
    assert_eq!(transport.send_count(), 1);
    assert_sigv4_signed_with(&recorded_send(&transport, 0), "ENV_ACCESS");

    // EcsContainer credential_source: the ECS provider feeds the signing pair.
    std::fs::write(
        &config,
        "[profile ecs-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=EcsContainer\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![
        source_response(
            200,
            r#"{"AccessKeyId":"ECSACCESS","SecretAccessKey":"ECSSECRET","Token":"ECSSESSION","Expiration":"2027-08-01T00:00:00Z"}"#,
        ),
        source_response(
            200,
            sts_success_xml(
                "ECSROLE",
                "arn:aws:sts::123456789012:assumed-role/fixture/s",
            ),
        ),
    ]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("ecs-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ECS credential_source resolves");
    assert_eq!(transport.send_count(), 2);
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://169.254.170.2/v2/creds"
    );
    assert_sigv4_signed_with(&recorded_send(&transport, 1), "ECSACCESS");
    assert!(
        matches!(
            auth.provenance.source,
            AuthProvenanceSource::AwsSigV4 {
                source: AwsCredentialSource::AssumeRole { .. }
            }
        ),
        "assume-role provenance wraps the resolved source"
    );

    // Ec2InstanceMetadata credential_source: the IMDS provider feeds the pair.
    std::fs::write(
        &config,
        "[profile imds-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Ec2InstanceMetadata\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "imds-token"),
        source_response(200, "imds-role\n"),
        source_response(
            200,
            r#"{"Code":"Success","AccessKeyId":"IMDSACCESS","SecretAccessKey":"IMDSSECRET","Token":"IMDSSESSION","Expiration":"2027-09-01T00:00:00Z"}"#,
        ),
        source_response(
            200,
            sts_success_xml(
                "IMDSROLE",
                "arn:aws:sts::123456789012:assumed-role/fixture/s",
            ),
        ),
    ]);
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("imds-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("IMDS credential_source resolves");
    assert_eq!(transport.send_count(), 4);
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://169.254.169.254/latest/api/token"
    );
    assert_sigv4_signed_with(&recorded_send(&transport, 3), "IMDSACCESS");

    // Unsupported credential_source values are continue-eligible.
    std::fs::write(
        &config,
        "[profile bad-role]\nrole_arn=arn:aws:iam::123456789012:role/fixture\ncredential_source=Unsupported\nregion=us-west-2\n",
    )
    .unwrap();
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        Some(credentials.as_path()),
        Some(config.as_path()),
        &runner,
        AwsAuthSelection::Profile("bad-role".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("unsupported credential_source continues");
    assert!(matches!(error, CredentialResolutionError::Exhausted));
}

#[tokio::test]
async fn ecs_relative_uri_and_token_file_take_precedence() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        (
            "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
            "/v2/fixture-creds",
        ),
        (
            "AWS_CONTAINER_CREDENTIALS_FULL_URI",
            "https://full.example.invalid/creds",
        ),
        ("AWS_CONTAINER_AUTHORIZATION_TOKEN", "direct-token"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("ecs-token");
    std::fs::write(&token_file, "file-token\n").unwrap();
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"AccessKeyId":"ECSACCESS","SecretAccessKey":"ECSSECRET","Token":"ECSSESSION","Expiration":"2027-10-01T00:00:00Z"}"#,
    )]);

    let scoped = ScopedAwsEnv {
        container_authorization_token_file: Some(token_file.display().to_string()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("ECS credentials resolve");

    assert_eq!(transport.send_count(), 1);
    let send = recorded_send(&transport, 0);
    assert_eq!(send.url, "http://169.254.170.2/v2/fixture-creds");
    assert_eq!(header_of(&send, "Authorization").unwrap(), "file-token");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::EcsContainer { legacy: false }
        }
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "ECSACCESS");
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-10-01T00:00:00Z")),
        "ECS expiration is retained"
    );
}

#[tokio::test]
async fn ecs_full_uri_https_is_allowed_and_http_requires_metadata_or_loopback_host() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    // https full URI: any host is allowed.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "https://credentials.example.invalid/creds",
    )]));
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"AccessKeyId":"HTTPSACCESS","SecretAccessKey":"HTTPSSECRET"}"#,
    )]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("https full URI resolves");
    assert_eq!(transport.send_count(), 1);

    // Plain http to an arbitrary host is a typed terminal rejection.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        "http://credentials.example.invalid/creds",
    )]));
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("http full URI to a non-metadata host is terminal");
    assert!(
        matches!(
            error,
            CredentialResolutionError::Terminal {
                reason: TerminalReason::InvalidContainerEndpoint,
                ..
            }
        ),
        "got {error:?}"
    );
    assert_eq!(transport.send_count(), 0, "no sends for invalid endpoints");
}

#[tokio::test]
async fn ecs_modern_budget_then_legacy_single_send_and_no_imds_fallthrough() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "/v2/creds",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Modern provider: three transport failures then success on send 4.
    let transport = FakeSourceTransport::new(vec![
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        source_response(
            200,
            r#"{"AccessKeyId":"RETRYACCESS","SecretAccessKey":"RETRYSECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("modern ECS retries within its fixed budget");
    assert_eq!(
        transport.send_count(),
        4,
        "modern budget is at most four sends"
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "RETRYACCESS");

    // Modern exhaustion falls to legacy exactly once; legacy failure ends the
    // chain without consulting IMDS.
    let transport = FakeSourceTransport::new(vec![
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
        Err(SourceHttpError::Transport),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("ECS exhaustion ends the chain");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(
        transport.send_count(),
        5,
        "four modern sends plus exactly one legacy send, and no IMDS leg"
    );
    for send in transport.sends_snapshot() {
        assert!(
            !send.url.contains("169.254.169.254"),
            "ECS failure must not fall through to IMDS: {}",
            send.url
        );
    }
}

#[tokio::test]
async fn imds_v2_token_role_and_credential_legs_resolve() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "fixture-imds-token"),
        source_response(200, "imds-role\nother-role\n"),
        source_response(
            200,
            r#"{"Code":"Success","Type":"AWS-HMAC","AccessKeyId":"IMDSACCESS","SecretAccessKey":"IMDSSECRET","Token":"IMDSSESSION","Expiration":"2027-11-01T00:00:00Z"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input).await.expect("IMDS resolves");

    assert_eq!(transport.send_count(), 3);
    let token_put = recorded_send(&transport, 0);
    assert_eq!(token_put.method, SourceHttpMethod::Put);
    assert_eq!(token_put.url, "http://169.254.169.254/latest/api/token");
    assert_eq!(
        header_of(&token_put, "x-aws-ec2-metadata-token-ttl-seconds").unwrap(),
        "21600"
    );
    let role_get = recorded_send(&transport, 1);
    assert_eq!(
        role_get.url,
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/"
    );
    assert_eq!(
        header_of(&role_get, "x-aws-ec2-metadata-token").unwrap(),
        "fixture-imds-token"
    );
    let creds_get = recorded_send(&transport, 2);
    assert_eq!(
        creds_get.url,
        "http://169.254.169.254/latest/meta-data/iam/security-credentials/imds-role"
    );
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::AwsSigV4 {
            source: AwsCredentialSource::Ec2InstanceMetadata
        }
    );
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "IMDSACCESS");
    assert_eq!(
        sigv4_of(&auth).expires_at,
        Some(rfc3339("2027-11-01T00:00:00Z"))
    );
}

#[tokio::test]
async fn imds_token_404_enables_v1_mode_for_subsequent_legs() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),
        source_response(200, "v1-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"V1ACCESS","SecretAccessKey":"V1SECRET","Token":"V1SESSION","Expiration":"2027-12-01T00:00:00Z"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("v1 fallback resolves");

    assert_eq!(transport.send_count(), 3);
    for leg in [1_usize, 2] {
        let send = recorded_send(&transport, leg);
        assert!(
            header_of(&send, "x-aws-ec2-metadata-token").is_none(),
            "v1 legs carry no token header"
        );
    }
    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "V1ACCESS");
}

#[tokio::test]
async fn imds_v1_disable_and_token_400_branches() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    // The disable env gates the default remote branch entirely: no IMDS leg
    // runs at all (the fixed node chain behavior).
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let transport = FakeSourceTransport::default_never();
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("a disabled remote segment ends the chain");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(
        transport.send_count(),
        0,
        "no remote leg runs when disabled"
    );

    // Token 400 stops that request: no v1 fallback, no further legs.
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let transport = FakeSourceTransport::new(vec![source_response(400, "")]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("token 400 stops the request");
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "{error:?}"
    );
    assert_eq!(transport.send_count(), 1);

    // "false" does not disable IMDS.
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "false",
    )]));
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),
        source_response(200, "v1-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"FALSEACCESS","SecretAccessKey":"FALSESECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("AWS_EC2_METADATA_DISABLED=false keeps IMDS enabled");
}

#[tokio::test]
async fn imds_v1_role_401_clears_token_state_and_refetches() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(404, ""),            // token miss -> v1 mode
        source_response(401, ""),            // v1 role list denied
        source_response(200, "fresh-token"), // re-fetch v2 token
        source_response(200, "imds-role"),
        source_response(
            200,
            r#"{"AccessKeyId":"REFRESHACCESS","SecretAccessKey":"REFRESHSECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("401 clears the token state and retries with a fresh token");

    assert_eq!(transport.send_count(), 5);
    let role_retry = recorded_send(&transport, 3);
    assert_eq!(
        header_of(&role_retry, "x-aws-ec2-metadata-token").unwrap(),
        "fresh-token"
    );
    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "REFRESHACCESS"
    );
}

#[tokio::test]
async fn imds_ipv6_endpoint_mode_is_selected() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_SERVICE_ENDPOINT_MODE",
        "IPv6",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = FakeSourceTransport::new(vec![
        source_response(200, "token"),
        source_response(200, "role"),
        source_response(
            200,
            r#"{"AccessKeyId":"V6ACCESS","SecretAccessKey":"V6SECRET"}"#,
        ),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);
    resolve_local_aws_auth(&input)
        .await
        .expect("IPv6 IMDS resolves");
    assert_eq!(
        recorded_send(&transport, 0).url,
        "http://[fd00:ec2::254]/latest/api/token"
    );
}
