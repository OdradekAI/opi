//! AWS credential-source lifecycle tests.

use super::*;

#[tokio::test]
async fn models_route_streams_selected_local_credentials_through_the_real_adapter() {
    let body_bytes = bedrock_text_lifecycle_bytes();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body_bytes, "application/vnd.amazon.eventstream"),
        )
        .mount(&server)
        .await;

    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(LocalChainBedrockResolver {
                ambient: MapAuthContext(std::collections::HashMap::from([
                    ("AWS_ACCESS_KEY_ID", "AKIAMODELSLOCAL"),
                    ("AWS_SECRET_ACCESS_KEY", "models-local-secret"),
                ])),
                credentials_file: None,
                config_file: None,
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Models Bedrock route");

    let mut request = lifecycle_text_request();
    request.model = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0".into();
    let prepared = collection
        .prepare_call(
            "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0",
            request,
        )
        .await
        .expect("prepare local-source call");
    let mut attempt = prepared.start_attempt().expect("start attempt");
    let mut saw_done = false;
    while let Some(item) = attempt.next().await {
        if matches!(item, Ok(AssistantStreamEvent::Done { .. })) {
            saw_done = true;
        }
    }
    drop(attempt);
    assert!(
        saw_done,
        "selected local credentials must stream the lifecycle"
    );

    let received = server.received_requests().await.expect("recorded requests");
    let recorded = &received[0];
    let authorization = recorded
        .headers
        .get("authorization")
        .expect("SigV4 authorization from the selected local source")
        .to_str()
        .expect("authorization is ASCII");
    assert!(authorization.contains("AKIAMODELSLOCAL"));
}

#[tokio::test]
async fn failing_local_source_resolution_sends_zero_generation_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(LocalChainBedrockResolver {
                // A partial scoped pair fails closed and borrows no ambient
                // half; generation must never dispatch.
                ambient: MapAuthContext(std::collections::HashMap::from([(
                    "AWS_SECRET_ACCESS_KEY",
                    "ambient-secret-half",
                )])),
                credentials_file: None,
                config_file: None,
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Models Bedrock route");
    // Give the resolver a partial scoped pair through a tiny wrapper.
    struct PartialScopedResolver;
    impl AuthResolver for PartialScopedResolver {
        fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async {
                let runner = ScriptedProcessRunner {
                    calls: std::sync::Mutex::new(Vec::new()),
                    result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
                };
                let ambient = MapAuthContext(std::collections::HashMap::from([(
                    "AWS_SECRET_ACCESS_KEY",
                    "ambient-secret-half",
                )]));
                let input = AwsLocalChainInput {
                    selection: AwsAuthSelection::Auto,
                    scoped: ScopedAwsEnv {
                        access_key_id: Some(SecretString::from("SCOPED_ACCESS_ONLY")),
                        ..ScopedAwsEnv::empty()
                    },
                    transport: &NEVER_TRANSPORT,
                    mfa: None,
                    sts_endpoint: None,
                    source_cache: None,
                    clock: &SYSTEM_SOURCE_CLOCK,
                    dpop_jti: &RANDOM_DPPO_JTI,
                    sso_cache_dir: None,
                    login_cache_dir: None,
                    sso_endpoint: None,
                    oidc_endpoint: None,
                    signin_endpoint: None,
                    ambient: &ambient,
                    credentials_file: None,
                    config_file: None,
                    process: &runner,
                    region: None,
                    default_region: "us-east-1",
                };
                resolve_local_aws_auth(&input)
                    .await
                    .map_err(|error| ProviderError::Config(error.into_safe_provider_summary()))
            })
        }
    }
    let mut failing = Models::new();
    failing
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(PartialScopedResolver),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register failing Bedrock route");

    let mut request = lifecycle_text_request();
    request.model = "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0".into();
    let error = failing
        .prepare_call(
            "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0",
            request,
        )
        .await
        .expect_err("partial scoped pair must fail preparation");
    assert!(
        error.to_string().contains("incomplete"),
        "typed incomplete-source failure, got {error}"
    );
    assert!(
        !error.to_string().contains("SCOPED_ACCESS_ONLY"),
        "failures stay redacted"
    );
    let received = server.received_requests().await.expect("recorded requests");
    assert!(
        received.is_empty(),
        "a failing local source must mean zero generation sends"
    );
}

#[tokio::test]
async fn cancelling_during_bedrock_resolution_fails_closed_without_dispatch() {
    let server = MockServer::start().await;
    let entered = Arc::new(Notify::new());
    let mut collection = Models::new();
    collection
        .register_route(
            Box::new(BedrockProvider::new(
                Some(server.uri()),
                Arc::new(HttpClient::new()),
            )),
            Arc::new(PendingBedrockResolver {
                entered: entered.clone(),
            }),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .expect("register Bedrock route");
    let collection = Arc::new(collection);
    let cancel = CancellationToken::new();
    let request = prepared_bedrock_request(cancel.clone());
    let expected_request = request.clone();
    let task = tokio::spawn(async move {
        collection
            .prepare_call(
                "bedrock:us.anthropic.claude-sonnet-4-20250514-v1:0",
                request,
            )
            .await
    });

    entered.notified().await;
    cancel.cancel();
    let error = task
        .await
        .expect("preparation task did not panic")
        .expect_err("cancelled resolution must fail");
    assert!(
        error.is_call_cancelled(),
        "logical-call cancellation origin remains known"
    );
    let ModelsError::Provider(ProviderError::StreamFailure(failure)) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    assert_eq!(failure.cause(), &ProviderError::Cancelled);
    assert_direct_projection_prefix(failure.message(), &expected_request);
    assert!(
        server
            .received_requests()
            .await
            .expect("recorded requests")
            .is_empty(),
        "cancelled auth resolution must not dispatch Bedrock"
    );
}

#[test]
fn prepared_sigv4_auth_debug_redacts_access_secret_and_session_values() {
    let access = "AKIA_PREPARED_DEBUG_CANARY";
    let secret = "prepared-secret-debug-canary";
    let session = "prepared-session-debug-canary";
    let auth = ResolvedAuth::aws_sigv4(
        AwsCredentials {
            access_key_id: access.into(),
            secret_access_key: secret.into(),
            session_token: Some(session.into()),
            region: "us-east-1".into(),
            expires_at: None,
        },
        aws_provenance(AwsCredentialSource::ExplicitConfig),
    );
    let debug = format!("{auth:?}");
    for canary in [access, secret, session] {
        assert!(!debug.contains(canary), "credential leaked: {debug}");
    }
    assert!(debug.contains("ExplicitConfig"), "provenance is non-secret");
}

#[tokio::test]
async fn bedrock_wrong_prepared_auth_error_does_not_expose_secret() {
    let secret = "wrong-auth-secret-canary";
    let server = MockServer::start().await;
    let provider = BedrockProvider::new(Some(server.uri()), Arc::new(HttpClient::new()));
    let auth = ResolvedAuth {
        env: Default::default(),
        scheme: AuthScheme::ApiKey,
        secret: SecretString::from(secret),
        base_url: None,
        account_id: None,
        provenance: AuthProvenance::default(),
    };
    let request = prepared_bedrock_request(CancellationToken::new());
    let mut stream = provider.stream_prepared(request.clone(), auth);
    let error = stream
        .next()
        .await
        .expect("wrong auth produces an error")
        .expect_err("wrong auth must fail closed");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let ProviderError::Config(summary) = failure.cause() else {
        panic!("expected exact wrong-auth cause: {error:?}");
    };
    assert_eq!(
        summary.as_str(),
        "Bedrock requires prepared AWS SigV4 or Bearer authentication"
    );
    assert_direct_projection_prefix(failure.message(), &request);
    assert!(stream.next().await.is_none(), "refusal must be terminal");
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(!format!("{error:?} {error}").contains(secret));
}

#[tokio::test]
async fn dropping_resolution_cancels_inflight_source_exchange() {
    struct HangingTransport {
        dropped: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }
    struct DropGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for DropGuard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }
    impl AwsSourceTransport for HangingTransport {
        fn send(
            &self,
            _request: SourceHttpRequest,
        ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>>
        {
            let guard = DropGuard(self.dropped.clone());
            Box::pin(async move {
                let _keep = guard;
                std::future::pending::<()>().await;
                unreachable!("pending never resolves")
            })
        }
    }

    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI",
        "/v2/creds",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let transport = HangingTransport {
        dropped: dropped.clone(),
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    )
    .with_source_transport(&transport);

    // Drive the resolution on a select branch, then abandon it: dropping
    // the owning future must release the in-flight exchange.
    let mut resolution = Box::pin(resolve_local_aws_auth(&input));
    tokio::select! {
        _ = &mut resolution => panic!("hanging transport never resolves"),
        _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
    }
    drop(resolution);
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        dropped.load(std::sync::atomic::Ordering::SeqCst),
        "dropping the resolution future must release the owned source exchange"
    );
}

#[tokio::test]
async fn sts_exchange_over_real_http_to_local_server() {
    // Real transport + real form body + real XML parse over loopback HTTP.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local STS");
    let addr = listener.local_addr().expect("local STS addr");
    let endpoint = format!("http://{addr}");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept STS request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 4096];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read STS request");
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
        let response = sts_success_xml(
            "LOCAL",
            "arn:aws:sts::123456789012:assumed-role/fixture/local",
        );
        let mut bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .into_bytes();
        bytes.extend_from_slice(response.as_bytes());
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write STS response");
        request
    });

    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = ReqwestSourceTransport::new();
    let dir = tempfile::tempdir().unwrap();
    let credentials = dir.path().join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=LOCAL_SOURCE_ACCESS\naws_secret_access_key=LOCAL_SOURCE_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nregion=us-west-2\n",
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
    .with_sts_endpoint(Some(&endpoint));
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("real STS exchange resolves");

    assert_eq!(sigv4_of(&auth).access_key_id.expose_secret(), "LOCALACCESS");
    let request = server.await.expect("server finished");
    let request_text = String::from_utf8_lossy(&request);
    assert!(
        request_text.starts_with("POST / HTTP/1.1"),
        "STS request line: {request_text}"
    );
    assert!(
        request_text.contains("content-type: application/x-www-form-urlencoded")
            || request_text.contains("Content-Type: application/x-www-form-urlencoded"),
        "form content type: {request_text}"
    );
    assert!(
        request_text.contains("Action=AssumeRole&Version=2011-06-15"),
        "query-encoded body: {request_text}"
    );
    assert!(
        request_text
            .to_ascii_lowercase()
            .contains("authorization: aws4-hmac-sha256"),
        "source SigV4 over real HTTP: {request_text}"
    );
}

#[tokio::test]
async fn ecs_full_uri_over_real_http_to_loopback() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local ECS");
    let addr = listener.local_addr().expect("local ECS addr");

    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept ECS request");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];
        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read ECS request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let body = r#"{"AccessKeyId":"LOOPECSACCESS","SecretAccessKey":"LOOPECSSECRET","Token":"LOOPECSSESSION","Expiration":"2028-01-01T00:00:00Z"}"#;
        let bytes = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .into_bytes();
        tokio::io::AsyncWriteExt::write_all(&mut socket, &bytes)
            .await
            .expect("write ECS response");
        request
    });

    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_CONTAINER_CREDENTIALS_FULL_URI",
        // http to loopback is an allowed host per the fixed endpoint rules.
        Box::leak(format!("http://{addr}/ecs-creds").into_boxed_str()) as &'static str,
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let transport = ReqwestSourceTransport::new();
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
        .expect("loopback ECS resolves over real HTTP");

    assert_eq!(
        sigv4_of(&auth).access_key_id.expose_secret(),
        "LOOPECSACCESS"
    );
    let request = server.await.expect("server finished");
    let request_text = String::from_utf8_lossy(&request);
    assert!(
        request_text.starts_with("GET /ecs-creds HTTP/1.1"),
        "ECS request line: {request_text}"
    );
}

#[tokio::test]
async fn terminal_sts_failure_through_models_sends_zero_generation_requests() {
    // Source exchange sends and generation sends are separate: an STS denial
    // may spend source legs but must never dispatch a generation request.
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
        "[source-prof]\naws_access_key_id=ZERO_ACCESS\naws_secret_access_key=ZERO_SECRET\n",
    )
    .unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nsource_profile=source-prof\nregion=us-west-2\n",
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

    // The chain itself is the authority: the typed terminal error from the
    // STS denial must surface before any generation dispatch (the
    // Models-level zero-send behavior for failed local resolution is proven
    // by failing_local_source_resolution_sends_zero_generation_requests).
    let error = resolve_local_aws_auth(&input)
        .await
        .expect_err("STS denial is terminal");
    assert!(matches!(error, CredentialResolutionError::Terminal { .. }));
    assert_eq!(transport.send_count(), 1, "one source leg spent");
}

#[test]
fn sts_endpoint_rules_follow_partition_and_override() {
    assert_eq!(
        sts_endpoint_url("us-west-2", None),
        "https://sts.us-west-2.amazonaws.com"
    );
    assert_eq!(
        sts_endpoint_url("cn-north-1", None),
        "https://sts.cn-north-1.amazonaws.com.cn"
    );
    assert_eq!(
        sts_endpoint_url("us-west-2", Some("https://sts.example.invalid")),
        "https://sts.example.invalid",
        "explicit override wins"
    );
}

#[tokio::test]
async fn selected_process_freezes_context_environment_and_cwd() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::write(
        &config,
        "[profile process]\ncredential_process=selected-command\n",
    )
    .unwrap();
    for fail in [false, true] {
        let context = FrozenProcessContext {
            cwd: dir.path().to_path_buf(),
            fail,
            snapshots: AtomicUsize::new(0),
        };
        let runner = FrozenSpecRunner(std::sync::Mutex::new(Vec::new()));
        let input = chain_input(
            &context,
            None,
            Some(&config),
            &runner,
            AwsAuthSelection::Profile("process".to_owned()),
            ScopedAwsEnv::empty(),
        );
        let result = resolve_local_aws_auth(&input).await;
        assert_eq!(context.snapshots.load(Ordering::SeqCst), 1);
        let invocations = runner.0.lock().unwrap();
        if fail {
            assert!(matches!(
                result,
                Err(CredentialResolutionError::Terminal { .. })
            ));
            assert!(invocations.is_empty());
        } else {
            result.unwrap();
            assert_eq!(invocations.len(), 1);
            assert_eq!(invocations[0].cwd.as_deref(), Some(dir.path()));
            assert_eq!(
                invocations[0].env,
                opi_ai::auth::aws::process::ProcessEnv::Exact(std::collections::BTreeMap::from([
                    ("SOURCE_IDENTITY".to_owned(), "scoped-synthetic".to_owned())
                ]))
            );
        }
    }
}
