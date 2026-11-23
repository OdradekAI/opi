//! AWS SSO chain tests.

use super::*;

#[tokio::test]
async fn chain_order_env_pair_beats_explicit_sso() {
    let dir = tempfile::tempdir().unwrap();
    modern_sso_token_file(dir.path(), "2030-01-01T04:00:00Z");
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
            access_key_id: Some(secrecy::SecretString::from("ENV_ACCESS")),
            secret_access_key: Some(secrecy::SecretString::from("ENV_SECRET")),
            ..sso_scoped("https://start.example", Some("sess-a"))
        },
    )
    .with_source_transport(&transport)
    .with_source_cache(Some(&cache))
    .with_clock(&clock)
    .with_dpop_jti(&jti)
    .with_sso_cache_dir(Some(dir.path()));
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::ScopedEnvironment);
    assert_eq!(transport.send_count(), 0, "the env pair wins before SSO");
}

#[tokio::test]
async fn chain_order_ini_process_beats_sso_and_login() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile corp]\ncredential_process = echo-process\nsso_session = sess-a\nsso_account_id = 1\nsso_role_name = r\nlogin_session = my-session\n",
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
    .with_dpop_jti(&jti);
    let source = chain_source(&input).await;
    assert_eq!(source, CredentialSource::CredentialProcess);
    assert_eq!(
        transport.send_count(),
        0,
        "process precedes SSO/login in the INI order"
    );
}

#[tokio::test]
async fn sso_throttle_suppresses_immediate_second_passive_refresh() {
    let queue = std::sync::Arc::new(opi_ai::mutation::MutationQueue::new());
    let cache = AwsSourceCache::with_mutation_queue(queue.clone());
    let identity = CacheIdentity::SsoToken {
        cache_key: "k".to_owned(),
    };
    cache.store(
        identity.clone(),
        SourceCacheValue::SsoToken(SsoTokenSnapshot {
            access_token: secrecy::SecretString::from("token"),
            expires_at: rfc3339("2030-01-01T00:02:00Z"),
            refresh: Some(SsoRefreshMaterial {
                client_id: "c".to_owned(),
                client_secret: secrecy::SecretString::from("s"),
                refresh_token: secrecy::SecretString::from("r"),
            }),
            modern: true,
        }),
    );
    let make_slot = || {
        Box::new(move || {
            Box::pin(async { Ok(opi_ai::mutation::MutationApplied::Keep) })
                as opi_ai::BoxAuthFuture<
                    'static,
                    Result<opi_ai::mutation::MutationApplied, opi_ai::mutation::MutationError>,
                >
        }) as opi_ai::mutation::MutationSlot
    };
    let now = rfc3339("2030-01-01T00:00:00Z");
    assert!(cache.maybe_spawn_passive(identity.clone(), now, make_slot()));
    // The 30-second throttle suppresses a same-instant second attempt.
    assert!(!cache.maybe_spawn_passive(identity.clone(), now, make_slot()));
    // Let the first slot settle, then the throttle window governs alone.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    assert!(
        !cache.maybe_spawn_passive(identity.clone(), now, make_slot()),
        "same-instant attempt stays throttled after settle"
    );
    let later = now + time::Duration::seconds(31);
    assert!(cache.maybe_spawn_passive(identity, later, make_slot()));
}
