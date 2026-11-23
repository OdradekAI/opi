//! AWS SSO expiry tests.

use super::*;

#[tokio::test]
async fn sso_near_expiry_uses_old_token_and_registers_passive_refresh() {
    let dir = tempfile::tempdir().unwrap();
    // Token expires in 2 minutes: inside the 5-minute window.
    modern_sso_token_file(dir.path(), "2030-01-01T00:02:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Sends: federation GET (old token). The passive OIDC refresh runs on
    // the mutation-owner runtime through its own transport.
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let passive_transport = std::sync::Arc::new(FakeSourceTransport::new(vec![source_response(
        200,
        r#"{"accessToken":"SSO_TOKEN_2","expiresIn":28800,"refreshToken":"oidc-refresh-2","tokenType":"Bearer"}"#,
    )]));
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let clock = FakeClock(rfc3339("2030-01-01T00:00:00Z"));
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: passive_transport.clone(),
            clock: std::sync::Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: std::sync::Arc::new(CountingJti(std::sync::atomic::AtomicUsize::new(0))),
        }),
    );
    let barrier_before = queue.barrier();
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
    let _ = auth;
    // The federation GET used the OLD token (passive refresh stays background).
    assert_eq!(bearer_of(&transport.send_at(0)), "SSO_TOKEN");
    // The passive refresh was registered into the mutation owner and runs
    // to completion even though the caller already left.
    let barrier_after = queue.barrier();
    assert!(barrier_after.cutoff() > barrier_before.cutoff());
    let report = barrier_after.wait().await;
    assert!(report.is_complete());
    assert_eq!(passive_transport.send_count(), 1);
    assert_eq!(
        passive_transport.send_at(0).url,
        "https://oidc.us-east-1.amazonaws.com/token"
    );
    // The standard cache file now holds the refreshed token.
    let file = read_sso_token_file(dir.path(), "sess-a").unwrap().unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.snapshot.access_token),
        "SSO_TOKEN_2"
    );
    let identity = CacheIdentity::SsoToken {
        cache_key: "sess-a".to_owned(),
    };
    let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("the acknowledged refresh must leave a cached token");
    };
    assert_eq!(snapshot.access_token.expose_secret(), "SSO_TOKEN_2");
    assert_eq!(
        snapshot.refresh.unwrap().refresh_token.expose_secret(),
        "oidc-refresh-2"
    );
    assert!(cache.revision(&identity) >= 2);
}

#[tokio::test]
async fn sso_nonpositive_refresh_expiry_never_reaches_federation() {
    for ttl in [0, -1] {
        let (result, federation) = source_refresh_expiry_probe(false, ttl, false).await;
        assert!(
            result.is_err(),
            "already-expired SSO access cannot authorize GetRoleCredentials"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn login_nonpositive_refresh_expiry_never_selects_credentials() {
    for ttl in [0, -1] {
        let (result, _) = source_refresh_expiry_probe(true, ttl, false).await;
        assert!(
            result.is_err(),
            "already-expired login credentials cannot reach a generation attempt"
        );
    }
}

#[tokio::test]
async fn source_refresh_extreme_expiry_is_typed_invalid_response() {
    for login in [false, true] {
        let (result, federation) = source_refresh_expiry_probe(login, i64::MAX, false).await;
        assert!(
            matches!(
                result,
                Err(CredentialResolutionError::Terminal {
                    reason: TerminalReason::InvalidSsoResponse
                        | TerminalReason::InvalidLoginResponse,
                    ..
                })
            ),
            "unrepresentable expiry must have an ordinary typed source failure"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn source_refresh_rechecks_actual_expiry_after_exchange_and_ack() {
    for login in [false, true] {
        let (usable, _) = source_refresh_expiry_probe(login, 1, false).await;
        assert!(usable.is_ok(), "a still-valid short TTL serves this call");
        let (result, federation) = source_refresh_expiry_probe(login, 1, true).await;
        assert!(
            result.is_err(),
            "a token expired before source ACK must not serve the waiting call"
        );
        assert_eq!(federation, 0);
    }
}

#[tokio::test]
async fn login_passive_extreme_expiry_is_failed_receipt_and_keeps_valid_access() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let access = r#"{"accessKeyId":"OLD_LOGIN","secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}"#;
    write_login_cache(
        dir.path(),
        "my-session",
        &pem,
        Some(access),
        Some("2030-01-01T00:02:00Z"),
    );
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(200, serde_json::json!({"accessToken":access,"expiresIn":i64::MAX,"refreshToken":"rotated-refresh"}).to_string())]));
    let cache = cache_with_owned_refresh(transport.clone());
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
        "OLD_LOGIN"
    );
    let report = cache.mutation_queue().unwrap().barrier().wait().await;
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Failed { .. })),
        "invalid expiry is a typed refresh failure rather than a panicked worker's unknown outcome"
    );
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("old access remains valid")
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh");
    assert_eq!(snapshot.access_token.unwrap().expose_secret(), access);
}

#[tokio::test]
async fn sso_failed_file_save_retains_rotated_material_through_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let identity = CacheIdentity::SsoToken {
        cache_key: "sess-a".to_owned(),
    };
    let file = dir.path().join(format!(
        "{}.json",
        opi_ai::auth::aws::sso::sso_cache_file_name("sess-a")
    ));
    std::fs::create_dir(&file).unwrap(); // every standard-file write must fail
    let clock = Arc::new(AdjustableSourceClock(std::sync::Mutex::new(rfc3339(
        "2030-01-01T00:00:00Z",
    ))));
    let owner_transport = Arc::new(FakeSourceTransport::new(vec![
        source_response(
            200,
            r#"{"accessToken":"SSO_TOKEN_2","expiresIn":60,"refreshToken":"oidc-refresh-2"}"#,
        ),
        source_response(
            200,
            r#"{"accessToken":"SSO_TOKEN_3","expiresIn":3600,"refreshToken":"oidc-refresh-3"}"#,
        ),
    ]));
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: owner_transport.clone(),
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: SecretString::from("SSO_TOKEN"),
            expires_at: rfc3339("2030-01-01T00:02:00Z"),
            refresh: Some(SsoRefreshMaterial {
                client_id: "oidc-client".to_owned(),
                client_secret: SecretString::from("oidc-secret"),
                refresh_token: SecretString::from("oidc-refresh"),
            }),
            modern: true,
        }),
    );
    let transport = FakeSourceTransport::new(vec![
        source_response(200, role_credentials_body(1_900_000_000_000)),
        source_response(200, role_credentials_body(1_900_000_000_000)),
    ]);
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
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
    .with_clock(clock.as_ref())
    .with_sso_cache_dir(Some(dir.path()));
    resolve_local_aws_auth(&input).await.unwrap();
    let report = queue.barrier().wait().await;
    assert!(report.is_complete());
    assert!(
        matches!(&report.captured[0], opi_ai::mutation::BarrierEntry::Settled(receipt) if matches!(receipt.outcome, opi_ai::mutation::MutationOutcome::Written { .. }))
    );
    let Some(SourceCacheValue::SsoToken(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("rotated source token stays owned after file failure")
    };
    assert_eq!(snapshot.access_token.expose_secret(), "SSO_TOKEN_2");
    *clock.0.lock().unwrap() = rfc3339("2030-01-01T00:02:01Z");
    resolve_local_aws_auth(&input).await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(owner_transport.send_at(1).body.as_ref().unwrap()).unwrap();
    assert_eq!(
        body["refreshToken"], "oidc-refresh-2",
        "expired access must retain the unique rotated refresh material"
    );
    assert_eq!(bearer_of(&transport.send_at(1)), "SSO_TOKEN_3");
    assert!(
        file.is_dir(),
        "best-effort file failure must not be reported as a saved file"
    );
}

#[tokio::test]
async fn login_failed_file_save_retains_rotated_material_through_expiry() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(&config, "[profile corp]\nlogin_session=my-session\n").unwrap();
    let file = dir
        .path()
        .join(format!("{}.json", login_cache_file_name("my-session")));
    std::fs::create_dir(&file).unwrap();
    let (_secret, pem) = login_cache_key_pair();
    let access = |key: &str| {
        serde_json::json!({"accessKeyId":key,"secretAccessKey":"LOGIN_SAK","sessionToken":"LOGIN_SESSION"}).to_string()
    };
    let clock = Arc::new(AdjustableSourceClock(std::sync::Mutex::new(rfc3339(
        "2030-01-01T00:00:00Z",
    ))));
    let transport = Arc::new(FakeSourceTransport::new(vec![
        source_response(200, serde_json::json!({"accessToken":access("LOGIN_2"),"expiresIn":60,"refreshToken":"signin-refresh-2"}).to_string()),
        source_response(200, serde_json::json!({"accessToken":access("LOGIN_3"),"expiresIn":3600,"refreshToken":"signin-refresh-3"}).to_string()),
    ]));
    let queue = Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(Arc::new(
        PassiveRefreshDeps {
            transport: transport.clone(),
            clock: clock.clone(),
            jti: Arc::new(CountingJti(AtomicUsize::new(0))),
        },
    ));
    let identity = CacheIdentity::LoginSession {
        session: "my-session".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::LoginSession(opi_ai::auth::aws::cache::LoginTokenSnapshot {
            client_id: "signin-client".to_owned(),
            refresh_token: SecretString::from("signin-refresh"),
            signing_key: SecretString::from(pem),
            account: Some("123456789012".to_owned()),
            access_token: Some(SecretString::from(access("LOGIN_1"))),
            expires_at: Some(rfc3339("2030-01-01T00:02:00Z")),
        }),
    );
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
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
    .with_clock(clock.as_ref())
    .with_login_cache_dir(Some(dir.path()));
    let first = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(sigv4_of(&first).access_key_id.expose_secret(), "LOGIN_1");
    let report = queue.barrier().wait().await;
    assert!(report.is_complete());
    let Some(SourceCacheValue::LoginSession(snapshot)) = cache.peek(&identity, clock.now()) else {
        panic!("rotated source token stays owned after file failure")
    };
    assert_eq!(snapshot.refresh_token.expose_secret(), "signin-refresh-2");
    *clock.0.lock().unwrap() = rfc3339("2030-01-01T00:02:01Z");
    let second = resolve_local_aws_auth(&input).await.unwrap();
    assert_eq!(sigv4_of(&second).access_key_id.expose_secret(), "LOGIN_3");
    let body: serde_json::Value =
        serde_json::from_slice(transport.send_at(1).body.as_ref().unwrap()).unwrap();
    assert_eq!(body["refreshToken"], "signin-refresh-2");
    assert!(file.is_dir());
}

#[tokio::test]
async fn sso_passive_refresh_failure_keeps_old_token() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T00:02:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Chain sends: only the federation GET with the old token. The passive
    // OIDC refresh fails (slow_down) on the mutation-owner transport; its
    // failure is swallowed and the old token stays usable.
    let transport = FakeSourceTransport::new(vec![source_response(
        200,
        role_credentials_body(1_900_000_000_000),
    )]);
    let passive_transport = std::sync::Arc::new(FakeSourceTransport::new(vec![source_response(
        400,
        r#"{"error":"slow_down","error_description":"try later"}"#,
    )]));
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone()).with_passive_deps(
        std::sync::Arc::new(PassiveRefreshDeps {
            transport: passive_transport.clone(),
            clock: std::sync::Arc::new(FakeClock(rfc3339("2030-01-01T00:00:00Z"))),
            jti: std::sync::Arc::new(CountingJti(std::sync::atomic::AtomicUsize::new(0))),
        }),
    );
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
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("old token stays usable");
    let _ = auth;
    assert_eq!(transport.send_count(), 1);
    assert_eq!(bearer_of(&transport.send_at(0)), "SSO_TOKEN");
    let barrier = queue.barrier();
    let report = barrier.wait().await;
    assert!(report.is_complete());
    assert_eq!(passive_transport.send_count(), 1);
    // The failed refresh wrote nothing: the cache file keeps the old token.
    let file = read_sso_token_file(dir.path(), "sess-a").unwrap().unwrap();
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&file.snapshot.access_token),
        "SSO_TOKEN"
    );
}

#[tokio::test]
async fn sso_expired_token_refresh_denied_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2029-12-31T23:00:00Z");
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = Arc::new(FakeSourceTransport::new(vec![source_response(
        400,
        r#"{"error":"invalid_grant"}"#,
    )]));
    let cache = cache_with_owned_refresh(transport.clone());
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
    .with_source_transport(transport.as_ref())
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("expired token with denied refresh terminates");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::SsoDenied {
                code: Some(code), ..
            },
            ..
        } => assert_eq!(code, "invalid_grant"),
        other => panic!("expected SSO denial terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 1, "no federation GET after denial");
}

#[tokio::test]
async fn sso_legacy_expired_token_is_terminal_relogin() {
    let dir = tempfile::tempdir().unwrap();
    // Legacy cache file: no refresh material.
    write_sso_token_file(
        dir.path(),
        &"HTTPS://LEGACY.EXAMPLE".to_uppercase(),
        &SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("LEGACY"),
            expires_at: rfc3339("2029-12-31T23:00:00Z"),
            refresh: None,
            modern: false,
        },
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
        None,
        &runner,
        AwsAuthSelection::Auto,
        sso_scoped("https://legacy.example", None),
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("legacy expired token requires re-login");
    match error {
        CredentialResolutionError::Terminal {
            reason: TerminalReason::InvalidSsoToken,
            ..
        } => {}
        other => panic!("expected invalid SSO token terminal, got {other:?}"),
    }
    assert_eq!(transport.send_count(), 0);
}
