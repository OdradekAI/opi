use super::fixtures::stored_api_key;
use super::*;

#[test]
fn construction_register_and_static_lookup_perform_zero_source_io() {
    let context = std::sync::Arc::new(CountingMapContext::new([]));
    let mut collection = opi_ai::Models::new();

    // Construction and registration happen before any context exists; attach
    // one afterwards and verify lookups still consult nothing.
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-a"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let _ = collection.get_model("prov-a", "whatever");
    let _ = collection.provider_ids();
    let _ = collection.auth_descriptor("prov-a");

    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "construction/register/static lookup must not touch any source"
    );
    assert_eq!(context.file_probes.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn check_auth_reads_env_only_on_demand_through_the_injected_context() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_PRESENT_7C31",
        "sk-value-irrelevant",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-env"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_PRESENT_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    collection
        .register(
            provider_with_model("prov-unset"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "lazy: no read yet"
    );
    let check = collection
        .check_auth("prov-env")
        .await
        .expect("check")
        .expect("configured");
    assert_eq!(check.source.as_deref(), Some("OPI_TEST_PRESENT_7C31"));
    assert_eq!(check.kind, opi_ai::AuthCheckKind::ApiKey);
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 1);

    // Unset env var: unconfigured, not an error.
    assert!(
        collection.check_auth("prov-unset").await.unwrap().is_none(),
        "unset env var must report unconfigured"
    );
    // Unknown provider: None, zero source access.
    let before = context.env_reads.load(Ordering::SeqCst);
    assert!(
        collection
            .check_auth("no-such-prov")
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), before);
}

#[tokio::test]
async fn injected_source_replaces_ambient_for_checks_and_resolution() {
    // The variable is deliberately unset in the ambient environment; only the
    // injected source can answer, proving ambient is never consulted.
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-replace"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_AMBIENT_UNSET_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let ambient = collection
        .check_auth("prov-replace")
        .await
        .expect("ambient check");
    assert!(ambient.is_none(), "ambient env is unset, so unconfigured");

    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_AMBIENT_UNSET_7C31",
        "sk-injected",
    )]));
    collection.set_auth_context(context);
    let check = collection
        .check_auth("prov-replace")
        .await
        .expect("injected check")
        .expect("configured through the injected source");
    assert_eq!(check.source.as_deref(), Some("OPI_TEST_AMBIENT_UNSET_7C31"));

    let resolved = collection
        .get_auth("prov-replace", Default::default())
        .await
        .expect("resolve")
        .expect("resolved through the injected source");
    assert_eq!(resolved.secret.expose_secret(), "sk-injected");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Environment {
            name: "OPI_TEST_AMBIENT_UNSET_7C31".to_owned()
        }
    );
}

#[tokio::test]
async fn explicit_per_call_key_wins_over_every_source() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_7C31",
        "sk-from-env",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-override"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_OVERRIDE_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: Some(SecretString::from("sk-explicit-per-call")),
        bearer: None,
        aws_sigv4: None,
    };
    let resolved = collection
        .get_auth("prov-override", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    assert_eq!(resolved.scheme, AuthScheme::ApiKey);
    assert_eq!(resolved.secret.expose_secret(), "sk-explicit-per-call");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "explicit override must not touch any source"
    );
}

#[tokio::test]
async fn explicit_per_call_typed_sigv4_credentials_win_without_source_access() {
    struct CountingResolver {
        resolves: std::sync::atomic::AtomicUsize,
    }
    impl AuthResolver for CountingResolver {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::auth::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>>
        {
            self.resolves.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Ok(ResolvedAuth {
                    env: Default::default(),
                    scheme: AuthScheme::ApiKey,
                    secret: SecretString::from("route-resolved"),
                    base_url: None,
                    account_id: None,
                    provenance: Default::default(),
                })
            })
        }
    }
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_SIGV4_7C31",
        "sk-from-env",
    )]));
    let resolver = std::sync::Arc::new(CountingResolver {
        resolves: std::sync::atomic::AtomicUsize::new(0),
    });
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register_route(
            provider_with_model("prov-override-sigv4"),
            resolver.clone(),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .expect("register");

    let credentials = opi_ai::AwsSigV4Credentials {
        access_key_id: SecretString::from("AKIAPERCALL"),
        secret_access_key: SecretString::from("per-call-secret"),
        session_token: None,
        region: "eu-central-1".to_owned(),
        expires_at: None,
    };
    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: None,
        bearer: None,
        aws_sigv4: Some(credentials),
    };
    let resolved = collection
        .get_auth("prov-override-sigv4", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    match resolved.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(credentials.access_key_id.expose_secret(), "AKIAPERCALL");
            assert_eq!(
                credentials.secret_access_key.expose_secret(),
                "per-call-secret"
            );
            assert_eq!(credentials.region, "eu-central-1");
        }
        other => panic!("expected typed AWS SigV4 override, got {other:?}"),
    }
    assert_eq!(resolved.secret.expose_secret(), "");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 0);
    assert_eq!(
        resolver.resolves.load(Ordering::SeqCst),
        0,
        "typed per-call override must not invoke the route resolver"
    );
}

#[tokio::test]
async fn explicit_per_call_bearer_token_wins_without_source_access() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_OVERRIDE_BEARER_7C31",
        "sk-from-env",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    collection
        .register(
            provider_with_model("prov-override-bearer"),
            opi_ai::AuthDescriptor::EnvApiKey {
                env_var: "OPI_TEST_OVERRIDE_BEARER_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let overrides = opi_ai::AuthResolutionOverrides {
        api_key: None,
        bearer: Some(SecretString::from("bedrock-bearer-per-call")),
        aws_sigv4: None,
    };
    let resolved = collection
        .get_auth("prov-override-bearer", overrides)
        .await
        .expect("override resolution")
        .expect("resolved");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), "bedrock-bearer-per-call");
    assert_eq!(
        resolved.provenance.source,
        opi_ai::AuthProvenanceSource::Explicit
    );
    assert_eq!(context.env_reads.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mutually_exclusive_explicit_overrides_are_a_typed_error() {
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-override-conflict"),
            opi_ai::AuthDescriptor::StaticApiKey {
                value: opi_ai::SecretKey::new("static-key"),
            },
            Default::default(),
        )
        .expect("register");

    let sigv4 = || opi_ai::AwsSigV4Credentials {
        access_key_id: SecretString::from("AKIAPERCALL"),
        secret_access_key: SecretString::from("per-call-secret"),
        session_token: None,
        region: "eu-central-1".to_owned(),
        expires_at: None,
    };
    for (index, overrides) in [
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: Some(SecretString::from("token")),
            aws_sigv4: None,
        },
        opi_ai::AuthResolutionOverrides {
            api_key: None,
            bearer: Some(SecretString::from("token")),
            aws_sigv4: Some(sigv4()),
        },
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: None,
            aws_sigv4: Some(sigv4()),
        },
        // All three set is still exactly one typed configuration error.
        opi_ai::AuthResolutionOverrides {
            api_key: Some(SecretString::from("key")),
            bearer: Some(SecretString::from("token")),
            aws_sigv4: Some(sigv4()),
        },
    ]
    .into_iter()
    .enumerate()
    {
        let error = collection
            .get_auth("prov-override-conflict", overrides)
            .await
            .unwrap_err();
        assert!(
            matches!(error, opi_ai::ModelsError::ConflictingAuthOverrides { .. }),
            "case {index} must fail as a typed override conflict"
        );
    }
}

#[tokio::test]
async fn check_reports_expired_oauth_configured_without_refresh() {
    let expired = opi_ai::Credential::OAuthToken {
        access: SecretString::from("expired-access"),
        refresh: SecretString::from("refresh-material"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(3)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let store = std::sync::Arc::new(RecordingStore::new([("prov-oauth", expired)]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection
        .register(
            provider_with_model("prov-oauth"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-oauth".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let check = collection
        .check_auth("prov-oauth")
        .await
        .expect("check")
        .expect("expired OAuth is still configured");
    assert_eq!(check.kind, opi_ai::AuthCheckKind::OAuth);
    assert_eq!(check.source.as_deref(), Some("OAuth"));
    assert_eq!(
        store.reads.load(Ordering::SeqCst),
        1,
        "one read, no retry loop"
    );
    assert_eq!(
        store.writes.load(Ordering::SeqCst),
        0,
        "check must never refresh or write"
    );
}

#[tokio::test]
async fn store_and_check_errors_are_typed_not_empty() {
    // Live store read failure surfaces as a typed error.
    let store = std::sync::Arc::new(RecordingStore {
        entries: std::collections::HashMap::new(),
        reads: AtomicUsize::new(0),
        writes: AtomicUsize::new(0),
        deletes: std::sync::Mutex::new(Vec::new()),
        fail_reads: true,
    });
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store);
    collection
        .register(
            provider_with_model("prov-storefail"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-storefail".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let error = collection
        .check_auth("prov-storefail")
        .await
        .expect_err("store failure must be an error");
    match &error {
        opi_ai::ModelsError::CredentialStore(opi_ai::CredentialStoreError::Backend {
            reason,
            ..
        }) => assert!(reason.contains("keychain daemon")),
        other => panic!("expected typed CredentialStore::Backend, got {other:?}"),
    }
    let rendered = format!("{error}");
    assert!(
        rendered.contains("keychain daemon unreachable"),
        "{rendered}"
    );

    // Probe-snapshot backend-unavailable also surfaces typed, never empty.
    let mut snapshot_collection = opi_ai::Models::new();
    snapshot_collection
        .register(
            provider_with_model("prov-snapshot"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-snapshot".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    snapshot_collection.set_probe(
        "prov-snapshot",
        opi_ai::CredentialSource::BackendUnavailable {
            reason: "no secret service".to_owned(),
        },
    );
    let error = snapshot_collection
        .check_auth("prov-snapshot")
        .await
        .expect_err("backend-unavailable snapshot must be an error");
    assert!(matches!(
        &error,
        opi_ai::ModelsError::CredentialStore(
            opi_ai::CredentialStoreError::BackendUnavailable { reason, .. }
        ) if reason == "no secret service"
    ));
}

#[tokio::test]
async fn explicit_store_absence_and_errors_override_present_probe_metadata() {
    for fail_reads in [false, true] {
        let mut store = RecordingStore::new([]);
        store.fail_reads = fail_reads;
        let store = Arc::new(store);
        let context = Arc::new(CountingMapContext::new([]));
        let mut models = opi_ai::Models::new();
        models.set_auth_context(context.clone());
        models.set_credential_store(store.clone());
        models
            .register(
                provider_with_model("probe-owner"),
                opi_ai::AuthDescriptor::StoreCredential {
                    key: "probe-owner".into(),
                    display_source: "explicit store".into(),
                },
                Default::default(),
            )
            .unwrap();
        models.set_probe(
            "probe-owner",
            opi_ai::CredentialSource::Present {
                label: "label-only metadata".into(),
            },
        );
        let check = models.check_auth("probe-owner").await;
        let auth = models.get_auth("probe-owner", Default::default()).await;
        if fail_reads {
            assert!(matches!(
                check,
                Err(opi_ai::ModelsError::CredentialStore(_))
            ));
            assert!(matches!(auth, Err(opi_ai::ModelsError::CredentialStore(_))));
        } else {
            assert!(check.unwrap().is_none());
            assert!(
                auth.unwrap().is_none(),
                "a probe cannot supply authentication"
            );
        }
        assert_eq!(store.reads.load(Ordering::SeqCst), 2);
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);
        assert_eq!(context.env_reads.load(Ordering::SeqCst), 0);
        assert_eq!(context.file_probes.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn selected_source_failure_never_falls_back() {
    // The route's selected source fails; an env var for the same provider is
    // present in the context, and a static descriptor also exists — none may
    // be tried. get_auth must return the typed failure.
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_FALLBACK_TRAP_7C31",
        "sk-must-not-be-used",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    let resolver = std::sync::Arc::new(FailingResolver {
        provider_id: "prov-selected",
        calls: AtomicUsize::new(0),
    });
    collection
        .register_route(
            provider_with_model("prov-selected"),
            resolver.clone(),
            opi_ai::AuthProvenanceSource::Environment {
                name: "OPI_TEST_FALLBACK_TRAP_7C31".to_owned(),
            },
            Default::default(),
        )
        .expect("register route");

    let error = collection
        .get_auth("prov-selected", Default::default())
        .await
        .expect_err("selected-source failure must propagate");
    match &error {
        opi_ai::ModelsError::AuthResolution { provider, error } => {
            assert_eq!(provider, "prov-selected");
            assert!(
                matches!(error.as_ref(), ProviderError::AuthFailed(_)),
                "the selected source's typed failure is carried unchanged: {error:?}"
            );
            assert!(format!("{error}").contains("auth"));
        }
        other => panic!("expected AuthResolution, got {other:?}"),
    }
    assert_eq!(
        resolver.calls.load(Ordering::SeqCst),
        1,
        "the selected source ran exactly once"
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "no fallback source may be consulted"
    );

    // CredentialNeeded from the selected source is "unconfigured" (None), not
    // a fallback trigger either.
    collection
        .register_route(
            provider_with_model("prov-needed"),
            std::sync::Arc::new(NeededResolver),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .expect("register route");
    assert!(
        collection
            .get_auth("prov-needed", Default::default())
            .await
            .expect("resolve")
            .is_none()
    );
}

#[tokio::test]
async fn logout_acks_only_the_selected_local_deletion() {
    let store = std::sync::Arc::new(RecordingStore::new([
        (
            "prov-logout",
            opi_ai::Credential::ApiKey(SecretString::from("sk-a").into()),
        ),
        (
            "prov-other",
            opi_ai::Credential::ApiKey(SecretString::from("sk-b").into()),
        ),
    ]));
    let context = std::sync::Arc::new(CountingMapContext::new([]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection.set_auth_context(context.clone());

    collection
        .logout(store.as_ref(), "prov-logout")
        .await
        .expect("logout ACK");
    assert_eq!(
        store.deletes.lock().unwrap().as_slice(),
        ["prov-logout".to_owned()].as_slice()
    );
    assert!(
        store.entries.contains_key("prov-other"),
        "other providers' entries must stay untouched"
    );
    assert_eq!(
        context.env_reads.load(Ordering::SeqCst),
        0,
        "logout must not touch the environment"
    );

    // A failing store stays typed instead of faking an ACK.
    struct FailingDeleteStore;
    impl opi_ai::CredentialStore for FailingDeleteStore {
        fn read<'a>(
            &'a self,
            provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<
            'a,
            Result<Option<opi_ai::Credential>, opi_ai::CredentialStoreError>,
        > {
            Box::pin(async move {
                Err(opi_ai::CredentialStoreError::CorruptMarker {
                    provider: provider_id.to_owned(),
                })
            })
        }
        fn write<'a>(
            &'a self,
            _provider_id: &'a str,
            _cred: &'a opi_ai::Credential,
        ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
            Box::pin(async { Ok(()) })
        }
        fn delete<'a>(
            &'a self,
            provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<'a, Result<(), opi_ai::CredentialStoreError>> {
            Box::pin(async {
                Err(opi_ai::CredentialStoreError::Backend {
                    provider: provider_id.to_owned(),
                    reason: "lock unavailable".to_owned(),
                })
            })
        }
        fn probe<'a>(
            &'a self,
            _provider_id: &'a str,
        ) -> opi_ai::BoxAuthFuture<'a, opi_ai::CredentialSource> {
            Box::pin(async { opi_ai::CredentialSource::Absent })
        }
    }
    let error = opi_ai::Models::new()
        .logout(&FailingDeleteStore, "prov-logout")
        .await
        .expect_err("failing store must surface typed");
    assert!(matches!(
        &error,
        opi_ai::ModelsError::CredentialStore(opi_ai::CredentialStoreError::Backend { reason, .. })
            if reason == "lock unavailable"
    ));
    // The inline default also skips corrupt reads for unconditional replace,
    // while updates still reject the same corruption before their callback.
    let written = FailingDeleteStore
        .modify(
            "prov-logout",
            opi_ai::write_mutation(stored_api_key("replacement")),
        )
        .await
        .unwrap();
    assert!(matches!(written.outcome, MutationOutcome::Written { .. }));
    let error = FailingDeleteStore
        .modify(
            "prov-logout",
            opi_ai::CredentialMutationOp::Update(Box::new(|_, _| {
                panic!("update callback must not run after a corrupt read")
            })),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        opi_ai::CredentialStoreError::CorruptMarker { .. }
    ));
}

#[tokio::test]
async fn get_available_lists_only_configured_providers() {
    let context = std::sync::Arc::new(CountingMapContext::new([(
        "OPI_TEST_AVAILABLE_7C31",
        "sk-present",
    )]));
    let mut collection = opi_ai::Models::new();
    collection.set_auth_context(context.clone());
    for (id, env) in [
        ("prov-a", Some("OPI_TEST_AVAILABLE_7C31")),
        ("prov-b", None),
    ] {
        collection
            .register(
                provider_with_model(id),
                opi_ai::AuthDescriptor::EnvApiKey {
                    env_var: env.unwrap_or("OPI_TEST_UNSET_7C31").to_owned(),
                },
                Default::default(),
            )
            .expect("register");
    }

    let available = collection.get_available(None).await.expect("available");
    let providers: Vec<&str> = available.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        providers,
        ["prov-a"],
        "only configured providers are available"
    );
    assert_eq!(available[0].1.id, "prov-a-model");

    let filtered = collection
        .get_available(Some("prov-b"))
        .await
        .expect("filtered");
    assert!(filtered.is_empty(), "prov-b is unconfigured");
}

#[tokio::test]
async fn ambient_context_env_and_file_exists_semantics() {
    use opi_ai::auth::AuthContext as _;
    let ambient = opi_ai::AmbientAuthContext;

    // Existing / missing files (serialized against ambient-env mutation).
    {
        let _lock = AMBIENT_ENV_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let present = dir.path().join("present.txt");
        std::fs::write(&present, b"x").unwrap();
        assert!(ambient.file_exists(present.to_str().unwrap()).unwrap());
        let missing = dir.path().join("missing.txt");
        assert!(!ambient.file_exists(missing.to_str().unwrap()).unwrap());
    }

    // env(): unset is None, not an error.
    assert_eq!(
        ambient.env("OPI_TEST_DEFINITELY_UNSET_7C31").unwrap(),
        None,
        "unset env is Ok(None)"
    );
}

#[tokio::test]
async fn get_auth_resolves_descriptor_sources_without_refresh() {
    // Stored OAuth (expired) resolves as stored, refresh-free.
    let expired = opi_ai::Credential::OAuthToken {
        access: SecretString::from("stored-access"),
        refresh: SecretString::from("stored-refresh"),
        expires_at: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1)),
        base_url: Some("https://copilot.example.com".to_owned()),
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let store = std::sync::Arc::new(RecordingStore::new([("prov-store", expired)]));
    let mut collection = opi_ai::Models::new();
    collection.set_credential_store(store.clone());
    collection
        .register(
            provider_with_model("prov-store"),
            opi_ai::AuthDescriptor::StoreCredential {
                key: "prov-store".to_owned(),
                display_source: "keychain test".to_owned(),
            },
            Default::default(),
        )
        .expect("register");

    let resolved = collection
        .get_auth("prov-store", Default::default())
        .await
        .expect("resolve")
        .expect("stored credential resolves");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.secret.expose_secret(), "stored-access");
    assert_eq!(
        resolved.base_url.as_deref(),
        Some("https://copilot.example.com")
    );
    assert_eq!(
        store.writes.load(Ordering::SeqCst),
        0,
        "management resolution must not refresh"
    );

    // Static descriptor resolves its baked value; label-only does not resolve.
    let mut collection = opi_ai::Models::new();
    collection
        .register(
            provider_with_model("prov-static"),
            opi_ai::AuthDescriptor::StaticApiKey {
                value: opi_ai::SecretKey::new("sk-static-value"),
            },
            Default::default(),
        )
        .expect("register");
    collection
        .register(
            provider_with_model("prov-label"),
            opi_ai::AuthDescriptor::Resolved {
                source: "route-owned".to_owned(),
            },
            Default::default(),
        )
        .expect("register");
    let resolved = collection
        .get_auth("prov-static", Default::default())
        .await
        .expect("static resolve")
        .expect("static key present");
    assert_eq!(resolved.secret.expose_secret(), "sk-static-value");
    assert!(
        collection
            .get_auth("prov-label", Default::default())
            .await
            .expect("label resolve")
            .is_none(),
        "label-only descriptor has no secret to resolve"
    );
}

// ---------------------------------------------------------------------------
// P20-STORE (task 20.22) — store-owned serialized mutations, receipts, and
// the synchronous mutation barrier.
// ---------------------------------------------------------------------------
