//! AWS SSO login tests.

use super::*;

#[tokio::test]
async fn login_session_bad_cache_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    std::fs::write(
        login_dir.join(format!("{}.json", login_cache_file_name("my-session"))),
        "{not json",
    )
    .unwrap();
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
    .with_login_cache_dir(Some(&login_dir));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("selected session with bad cache terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::LoginCacheInvalid,
            ..
        } => {}
        other => panic!("expected login cache terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn login_session_refresh_dpop_wire_and_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    write_login_cache(&login_dir, "my-session", &pem, None, None);
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
        200,
        format!(
            r#"{{"accessToken":{},"expiresIn":28800,"refreshToken":"signin-refresh-2"}}"#,
            serde_json::to_string(access_token_json).unwrap()
        ),
    )]));
    let cache = cache_with_owned_refresh(transport.clone());
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
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::LoginSession {
            session: "my-session".to_owned()
        }
    );
    // The SignIn refresh carried the owned DPoP proof over the fixed wire.
    assert_eq!(transport.send_count(), 1);
    let send = transport.send_at(0);
    assert_eq!(send.url, "https://signin.us-east-1.amazonaws.com/v1/token");
    let proof = send
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("dpop"))
        .map(|(_, value)| value.clone())
        .expect("DPoP header");
    assert_dpop_proof_shape(
        &proof,
        "jti-0",
        1893456000,
        "https://signin.us-east-1.amazonaws.com/v1/token",
    );
    let body: serde_json::Value =
        serde_json::from_slice(send.body.as_ref().expect("body")).unwrap();
    assert_eq!(body["clientId"], "signin-client");
    assert_eq!(body["grantType"], "refresh_token");
    assert_eq!(body["refreshToken"], "signin-refresh");
    // The rotated refresh token was written back to the standard cache.
    let file = read_login_cache_file(&login_dir, "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.refresh_token),
        "signin-refresh-2"
    );
    assert!(file.access_token.is_some());
}

#[tokio::test]
async fn login_started_rotation_survives_waiter_drop_and_is_barrier_visible() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    write_login_cache(dir.path(), "my-session", &pem, None, None);
    let transport = Arc::new(GatedLoginRefreshTransport {
        entered: Notify::new(),
        release: Notify::new(),
        sends: AtomicUsize::new(0),
    });
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: transport.clone(),
            clock: Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let start_waiter = || {
        tokio::spawn({
            let transport = transport.clone();
            let cache = cache.clone();
            let login_dir = dir.path().to_path_buf();
            let config = config.clone();
            async move {
                let ambient = MapAuthContext(std::collections::HashMap::new());
                let runner = ScriptedProcessRunner {
                    calls: std::sync::Mutex::new(Vec::new()),
                    result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
                };
                let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
                let input = chain_input(
                    &ambient,
                    None,
                    Some(&config),
                    &runner,
                    AwsAuthSelection::Profile("corp".to_owned()),
                    ScopedAwsEnv::empty(),
                )
                .with_source_transport(transport.as_ref())
                .with_source_cache(Some(&cache))
                .with_clock(&clock)
                .with_login_cache_dir(Some(&login_dir));
                resolve_local_aws_auth(&input).await
            }
        })
    };
    let waiter = start_waiter();
    transport.entered.notified().await;
    let second_waiter = start_waiter();
    let barrier = queue.barrier();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    transport.release.notify_one();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    assert!(
        !report.captured.is_empty(),
        "the active rotation belongs to the source owner"
    );
    let second = second_waiter.await.unwrap().unwrap();
    assert_eq!(
        sigv4_of(&second).access_key_id.expose_secret(),
        "LOGIN_AKID"
    );
    let file = read_login_cache_file(dir.path(), "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(file.refresh_token.expose_secret(), "signin-refresh-2");
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(
        &CacheIdentity::LoginSession {
            session: "my-session".to_owned(),
        },
        rfc3339("2030-01-01T00:00:00Z"),
    ) else {
        panic!("the acknowledged active rotation remains usable");
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh-2");
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn login_invalidated_rotation_cannot_overwrite_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    write_login_cache(dir.path(), "my-session", &pem, None, None);
    let transport = Arc::new(GatedLoginRefreshTransport {
        entered: Notify::new(),
        release: Notify::new(),
        sends: AtomicUsize::new(0),
    });
    let cache = cache_with_owned_refresh(transport.clone());
    let waiter = tokio::spawn({
        let cache = cache.clone();
        let transport = transport.clone();
        let config = config.clone();
        let login_dir = dir.path().to_path_buf();
        async move {
            let ambient = MapAuthContext(std::collections::HashMap::new());
            let runner = ScriptedProcessRunner {
                calls: std::sync::Mutex::new(Vec::new()),
                result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
            };
            let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
            let input = chain_input(
                &ambient,
                None,
                Some(&config),
                &runner,
                AwsAuthSelection::Profile("corp".to_owned()),
                ScopedAwsEnv::empty(),
            )
            .with_source_transport(transport.as_ref())
            .with_source_cache(Some(&cache))
            .with_clock(&clock)
            .with_login_cache_dir(Some(&login_dir));
            resolve_local_aws_auth(&input).await
        }
    });
    transport.entered.notified().await;
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    cache.invalidate(&identity);
    let replacement = r#"{"accessKeyId":"REPLACEMENT","secretAccessKey":"REPLACEMENT_SECRET","sessionToken":"REPLACEMENT_SESSION"}"#;
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(replacement),
        Some("2030-01-01T08:00:00Z"),
    );
    transport.release.notify_one();
    assert!(matches!(
        waiter.await.unwrap(),
        Err(CredentialResolutionError::Terminal {
            reason: TerminalReason::SourceRefreshSuperseded,
            ..
        })
    ));
    let report = cache.mutation_queue().unwrap().barrier().wait().await;
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Failed { .. }))
    );
    assert!(
        cache
            .peek(&identity, rfc3339("2030-01-01T00:00:00Z"))
            .is_none()
    );
    let file = read_login_cache_file(dir.path(), "my-session")
        .unwrap()
        .unwrap();
    assert_eq!(file.access_token.unwrap().expose_secret(), replacement);
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let input = chain_input(
        &ambient,
        None,
        Some(&config),
        &runner,
        AwsAuthSelection::Profile("corp".to_owned()),
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_login_cache_dir(Some(dir.path()));
    let selected = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(
        sigv4_of(&selected).access_key_id.expose_secret(),
        "REPLACEMENT"
    );
    assert_eq!(transport.sends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn login_access_denied_family_is_terminal() {
    for code in [
        "AccessDeniedException",
        "TokenExpired",
        "UserCredentialsChanged",
        "InsufficientPermissions",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_secret, pem) = login_cache_key_pair();
        let config = dir.path().join("config");
        std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
        let login_dir = dir.path().join("login");
        std::fs::create_dir_all(&login_dir).unwrap();
        write_login_cache(&login_dir, "my-session", &pem, None, None);
        let ambient = MapAuthContext(std::collections::HashMap::new());
        let runner = ScriptedProcessRunner {
            calls: std::sync::Mutex::new(Vec::new()),
            result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
        };
        let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
            403,
            format!(r#"{{"code":"{code}","message":"denied"}}"#),
        )]));
        let cache = cache_with_owned_refresh(transport.clone());
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
        .with_source_transport(transport.as_ref())
        .with_source_cache(Some(&cache))
        .with_clock(&clock)
        .with_dpop_jti(&jti)
        .with_login_cache_dir(Some(&login_dir));
        let error = resolve_local_aws_auth(&input)
            .await
            .unwrap_err_or_ok("denied")
            .expect_err(&format!("terminal AccessDenied family: {code}"));
        match error {
            CredentialResolutionError::Terminal {
                reason:
                    TerminalReason::LoginDenied {
                        status: Some(403),
                        code: Some(observed),
                    },
                ..
            } => assert_eq!(observed, code),
            other => panic!("expected login denial terminal for {code}, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn login_other_failure_with_expired_token_continues_chain() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\nlogin_session = my-session\nregion = us-east-1\n",
    )
    .unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    // Cached access token already expired.
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    write_login_cache(
        &login_dir,
        "my-session",
        &pem,
        Some(access_token_json),
        Some("2029-12-31T23:00:00Z"),
    );
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_EC2_METADATA_DISABLED",
        "true",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Unclassified refresh failure (500 without a code): the expired token
    // raises the default continue-eligible error.
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(500, "{}")]));
    let cache = cache_with_owned_refresh(transport.clone());
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
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("chain advances past the failed login segment");
    // The chain continued to its end: exhaustion, not a terminal error.
    assert!(
        matches!(error, CredentialResolutionError::Exhausted),
        "expected continue-eligible exhaustion, got {error:?}"
    );
    assert_eq!(transport.send_count(), 1, "only the failed refresh ran");
}

#[tokio::test]
async fn login_unexpired_token_refresh_failure_keeps_old() {
    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    let access_token_json = r#"{"accessKeyId":"LOGIN_AKID","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    // Cached access token still valid for 2 minutes (inside the window, but
    // no mutation owner is attached: the refresh failure must keep it).
    write_login_cache(
        &login_dir,
        "my-session",
        &pem,
        Some(access_token_json),
        Some("2030-01-01T00:02:00Z"),
    );
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
    .with_login_cache_dir(Some(&login_dir));
    let source = chain_source(&input).await;
    assert_eq!(
        source,
        CredentialSource::LoginSession {
            session: "my-session".to_owned()
        },
        "the still-valid cached token is used without any send"
    );
    assert_eq!(transport.send_count(), 0);
}

#[tokio::test]
async fn signin_dpop_over_real_http_to_local_server() {
    // Real transport + real ES256 DPoP proof over loopback HTTP; the proof
    // signature is verified against the JWK embedded in the proof header.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local SignIn");
    let addr = listener.local_addr().expect("local SignIn addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept SignIn request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read SignIn request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if let Some(header_end) = find_subslice(&request, b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]).to_string();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if !name.trim().eq_ignore_ascii_case("Content-Length") {
                            return None;
                        }
                        value.trim().parse::<usize>().ok()
                    })
                    .unwrap_or(0);
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        let access_token_json = r#"{"accessKeyId":"LOCALLOGIN","secretAccessKey":"LOGINSECRET","sessionToken":"LOGINSESSION"}"#;
        let response = format!(
            r#"{{"accessToken":{},"expiresIn":28800,"refreshToken":"signin-refresh-2"}}"#,
            serde_json::to_string(access_token_json).unwrap()
        );
        let mut bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .into_bytes();
        bytes.extend_from_slice(response.as_bytes());
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write SignIn response");
        request
    });

    let dir = tempfile::tempdir().unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session = my-session\n").unwrap();
    let login_dir = dir.path().join("login");
    std::fs::create_dir_all(&login_dir).unwrap();
    write_login_cache(&login_dir, "my-session", &pem, None, None);

    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(opi_ai::auth::aws::source_http::ReqwestSourceTransport::new());
    let cache = cache_with_owned_refresh(transport.clone());
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
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_login_cache_dir(Some(&login_dir))
    .with_signin_endpoint(Some(&endpoint));
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("login resolves");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!("expected SigV4 provenance");
    };
    assert_eq!(
        source,
        AwsCredentialSource::LoginSession {
            session: "my-session".to_owned()
        }
    );

    let request_text = String::from_utf8(server.await.expect("server finished")).unwrap();
    let proof = request_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("dpop")
                .then(|| value.trim().to_owned())
        })
        .expect("DPoP header over real HTTP");
    assert_dpop_proof_shape(&proof, "jti-0", 1893456000, &format!("{endpoint}/v1/token"));
    assert!(
        request_text.contains("POST /v1/token"),
        "real SignIn request: {request_text}"
    );
}
