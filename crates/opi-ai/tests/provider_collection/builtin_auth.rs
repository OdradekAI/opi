//! Built-in auth source selection and binding across prepared calls.

use super::*;

#[tokio::test]
async fn ecs_full_uri_parsed_host_guard_runs_before_token_file_or_send() {
    use opi_ai::auth::aws::source_http::{
        AwsSourceTransport, SourceHttpError, SourceHttpRequest, SourceHttpResponse,
    };
    use std::sync::Arc;

    #[derive(Default)]
    struct Source(Mutex<Vec<SourceHttpRequest>>);
    impl AwsSourceTransport for Source {
        fn send(
            &self,
            request: SourceHttpRequest,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<SourceHttpResponse, SourceHttpError>>
        {
            self.0.lock().unwrap().push(request);
            Box::pin(async {
                Ok(SourceHttpResponse {
                    status: 200,
                    body: br#"{"AccessKeyId":"fixture","SecretAccessKey":"fixture"}"#.to_vec(),
                })
            })
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let missing_token = dir.path().join("missing-token");
    // Endpoint classification data is handled only by the injected Source.
    let non_loopback_ipv6_fixture = "http://[::2]/creds";
    let link_local_ipv4_fixture = "http://169.254.0.1/creds";
    let rejected_endpoints = [
        "http://127.attacker.invalid/creds",
        "http://169.254.attacker.invalid/creds",
        "http://credentials.invalid/creds",
        non_loopback_ipv6_fixture,
    ];
    for uri in rejected_endpoints {
        for token_file in [None, Some(missing_token.to_string_lossy().into_owned())] {
            let source = Arc::new(Source::default());
            let ambient = map_context(&[]);
            let mut models = Models::new();
            let mut overrides = BuiltinOverrides::default();
            overrides.aws_auth.region = Some("us-east-1".into());
            overrides.aws_auth.transport = Some(source.clone());
            overrides.aws_auth.scoped.container_credentials_full_uri = Some(uri.into());
            overrides.aws_auth.scoped.container_authorization_token_file = token_file;
            overrides.aws_auth.scoped.container_authorization_token = Some("fixture-token".into());
            register_builtin(
                &mut models,
                ambient,
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let error = models
                .get_auth("amazon-bedrock", Default::default())
                .await
                .unwrap_err();
            assert!(
                matches!(&error, ModelsError::AuthResolution { error, .. } if matches!(error.as_ref(), ProviderError::Config(_)))
                    && error.to_string().contains("container endpoint"),
                "{uri} must fail the endpoint guard before attempting the missing token file: {error:?}"
            );
            assert!(
                source.0.lock().unwrap().is_empty(),
                "{uri} must not send or fall through to IMDS"
            );
        }
    }

    let accepted_endpoints = [
        ("http://127.0.0.1/creds", None),
        ("http://127.255.255.254/creds", None),
        (link_local_ipv4_fixture, None),
        ("http://[::1]/creds", None),
        ("http://localhost/creds", None),
        ("https://credentials.invalid/creds", None),
        ("http://127.attacker.invalid/creds", Some("/preferred")),
    ];
    for (uri, relative) in accepted_endpoints {
        let source = Arc::new(Source::default());
        let mut overrides = BuiltinOverrides::default();
        overrides.aws_auth.region = Some("us-east-1".into());
        overrides.aws_auth.transport = Some(source.clone());
        overrides.aws_auth.scoped.container_credentials_full_uri = Some(uri.into());
        overrides.aws_auth.scoped.container_credentials_relative_uri = relative.map(str::to_owned);
        overrides.aws_auth.scoped.container_authorization_token = Some("fixture-token".into());
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &overrides,
        )
        .unwrap();
        models
            .get_auth("amazon-bedrock", Default::default())
            .await
            .unwrap();
        let sends = source.0.lock().unwrap();
        assert_eq!(sends.len(), 1, "{uri}");
        assert!(
            sends[0]
                .headers
                .iter()
                .any(|(key, value)| key == "Authorization" && value == "fixture-token")
        );
        if relative.is_some() {
            assert_eq!(sends[0].url, "http://169.254.170.2/preferred");
        }
    }
}

#[tokio::test]
async fn builtin_env_resolver_reads_through_injected_context() {
    let resolver = BuiltinEnvAuthResolver::new(
        map_context(&[("DEEPSEEK_API_KEY", "sk-deepseek")]),
        "DEEPSEEK_API_KEY",
        AuthScheme::Bearer,
    );
    let resolved = resolver.resolve().await.expect("mapped key resolves");
    assert_eq!(resolved.scheme, AuthScheme::Bearer);
    assert_eq!(resolved.base_url, None);

    let missing =
        BuiltinEnvAuthResolver::new(map_context(&[]), "DEEPSEEK_API_KEY", AuthScheme::Bearer);
    assert!(matches!(
        missing.resolve().await,
        Err(ProviderError::CredentialNeeded { .. })
    ));
}

#[tokio::test]
async fn builtin_stored_key_wins_and_store_replacement_only_changes_future_calls() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_string(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    )).expect(2).mount(&server).await;
    let store = |key: &'static str| {
        Arc::new(FakeStore {
            entries: Mutex::new(HashMap::from([(
                "groq".into(),
                Credential::ApiKey(secret(key).into()),
            )])),
        }) as Arc<dyn CredentialStore>
    };
    let mut models = Models::new();
    models.set_credential_store(store("stored-first"));
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("GROQ_API_KEY", "ambient-unused")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let id = first_catalog_chat_model("groq");
    let record = opi_ai::ChatModelRecord::new("groq", models.get_model("groq", &id).unwrap());
    let request = minimal_request(&format!("groq:{id}"));
    let frozen = models.complete_simple(record.clone(), request.clone(), Default::default());
    assert!(server.received_requests().await.unwrap().is_empty());
    models.set_credential_store(store("stored-second"));
    frozen.await.expect("old call keeps its source binding");
    models
        .complete_simple(record, request, Default::default())
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].headers["authorization"], "Bearer stored-first");
    assert_eq!(sent[1].headers["authorization"], "Bearer stored-second");
}

#[tokio::test]
async fn builtin_openai_oauth_excludes_decisions_without_falling_back_to_env_key() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    let credential = Credential::OAuthToken {
        access: "fixture-oauth".into(),
        refresh: "unused".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let mut models = Models::new();
    models.set_credential_store(Arc::new(FakeStore {
        entries: Mutex::new(HashMap::from([("openai".into(), credential)])),
    }));
    register_builtin(
        &mut models,
        map_context(&[("OPENAI_API_KEY", "must-not-fallback")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(
        models.check_auth("openai").await.unwrap().unwrap().kind,
        AuthCheckKind::OAuth
    );
    assert!(
        models
            .get_available_classifiers(Some("openai"))
            .await
            .unwrap()
            .is_empty()
    );
    let entry = Catalog::builtin()
        .unwrap()
        .models("openai", CatalogOperation::Classifier)
        .unwrap()
        .next()
        .unwrap();
    let mut record = models.classifier_model("openai", &entry.id).unwrap();
    let server = wiremock::MockServer::start().await;
    record.base_url = server.uri();
    let result = models
        .classify(
            record,
            ClassifierContext::default(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn builtin_oauth_flow_replacement_changes_future_calls_only() {
    use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct Flow(&'static str);
    impl OAuthProvider for Flow {
        fn id(&self) -> &str {
            "openai"
        }
        fn to_auth(&self, _: &OAuthCredential) -> ResolvedAuth {
            ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: self.0.into(),
                base_url: None,
                account_id: None,
                provenance: opi_ai::auth::AuthProvenance {
                    source: AuthProvenanceSource::OAuth {
                        kind: "openai".into(),
                    },
                    fallback: opi_ai::auth::AuthFallback::NotAttempted,
                },
            }
        }
        fn login<'a>(
            &'a self,
            _: &'a dyn LoginPresenter,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("login must not start")
        }
        fn refresh<'a>(
            &'a self,
            _: &'a OAuthCredential,
        ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
            panic!("fresh fixture must not refresh")
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::OpenAiResponses)),
        )
        .expect(2)
        .mount(&server)
        .await;
    let credential = Credential::OAuthToken {
        access: "acknowledged".into(),
        refresh: "unused".into(),
        expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    };
    let mut models = Models::new();
    models.set_credential_store(Arc::new(FakeStore {
        entries: Mutex::new(HashMap::from([("openai".into(), credential)])),
    }));
    models.register_oauth_flow("openai", Arc::new(Flow("old-flow")));
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("openai".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let info = models
        .get_model("openai", &first_catalog_chat_model("openai"))
        .unwrap();
    let record = opi_ai::ChatModelRecord::new("openai", info);
    let old = models.complete_simple(
        record.clone(),
        minimal_request("ignored"),
        Default::default(),
    );
    models.register_oauth_flow("openai", Arc::new(Flow("new-flow")));
    old.await.unwrap();
    models
        .complete_simple(record, minimal_request("ignored"), Default::default())
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].headers["authorization"], "Bearer old-flow");
    assert_eq!(sent[1].headers["authorization"], "Bearer new-flow");
}

#[tokio::test]
async fn builtin_anthropic_header_owned_token_is_not_an_oauth_identity() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("ANTHROPIC_AUTH_TOKEN", "sk-ant-oat-header-owned-fixture"),
            ("ANTHROPIC_OAUTH_TOKEN", "must-not-use"),
            ("ANTHROPIC_API_KEY", "must-not-use"),
        ]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let model = ModelInfo::new(
        "owned",
        "Owned",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(2048, 128),
    )
    .with_base_url(server.uri());
    models
        .complete_simple(
            opi_ai::ChatModelRecord::new("anthropic", model),
            minimal_request("ignored"),
            Default::default(),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(
        sent[0].headers["authorization"],
        "Bearer sk-ant-oat-header-owned-fixture"
    );
    assert!(!sent[0].headers.contains_key("x-api-key"));
    assert!(!sent[0].headers.contains_key("anthropic-beta"));
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert!(body.get("system").is_none());
}
