//! Bedrock builtin route assembly tests.

use super::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request as HttpRequest, Response, body::Incoming, service::service_fn};
use hyper_util::rt::{TokioExecutor, TokioIo};
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::http::{HttpClientBuilder, TlsOptions};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use secrecy::ExposeSecret;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct Sources {
    values: BTreeMap<String, String>,
    reads: Mutex<Vec<String>>,
}
impl AuthContext for Sources {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.reads.lock().unwrap().push(name.into());
        Ok(self.values.get(name).cloned())
    }
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        panic!("unselected file source: {path}")
    }
}

struct UnavailableStore(AtomicUsize);
impl opi_ai::credential::CredentialStore for UnavailableStore {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<
        'a,
        Result<Option<opi_ai::credential::Credential>, opi_ai::credential::CredentialStoreError>,
    > {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Err(
                opi_ai::credential::CredentialStoreError::BackendUnavailable {
                    provider: provider_id.into(),
                    reason: "isolated unavailable store".into(),
                },
            )
        })
    }
    fn write<'a>(
        &'a self,
        _: &'a str,
        _: &'a opi_ai::credential::Credential,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::credential::CredentialStoreError>> {
        Box::pin(async { panic!("unexpected write") })
    }
    fn delete<'a>(
        &'a self,
        _: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), opi_ai::credential::CredentialStoreError>> {
        Box::pin(async { panic!("unexpected delete") })
    }
    fn probe<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, opi_ai::credential::CredentialSource> {
        Box::pin(async { panic!("unexpected probe") })
    }
}

#[tokio::test]
async fn node5_builtin_explicit_aws_selection_never_consults_store_or_env() {
    let server = MockServer::start().await;
    let fallback = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            signed_thinking_stream(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let context = Arc::new(Sources::default());
    let store = Arc::new(UnavailableStore(AtomicUsize::new(0)));
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.selection = Some(opi_ai::auth::aws::local::AwsAuthSelection::SigV4(
        test_credentials(),
    ));
    overrides.aws_auth.base_url = Some(server.uri());
    overrides
        .base_urls
        .insert("amazon-bedrock".into(), fallback.uri());
    register_builtin(
        &mut models,
        context.clone(),
        Arc::new(HttpClient::new()),
        &overrides,
    )
    .unwrap();
    assert!(models.check_auth("amazon-bedrock").await.unwrap().is_some());
    let id = &Catalog::builtin()
        .unwrap()
        .models("amazon-bedrock", CatalogOperation::Chat)
        .unwrap()
        .find(|model| model.id.contains("claude-sonnet-4"))
        .unwrap()
        .id;
    let mut model = models.get_model("amazon-bedrock", id).unwrap();
    assert_eq!(model.base_url.as_deref(), Some(fallback.uri().as_str()));
    model.base_url = None;
    let mut request = text_stream_request();
    request.model = format!("amazon-bedrock:{id}");
    models
        .complete_simple(
            opi_ai::ChatModelRecord::new("amazon-bedrock", model),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(store.0.load(Ordering::SeqCst), 0);
    assert!(fallback.received_requests().await.unwrap().is_empty());
    assert!(context.reads.lock().unwrap().is_empty());
    assert!(
        server.received_requests().await.unwrap()[0].headers["authorization"]
            .to_str()
            .unwrap()
            .contains("AKIAIOSFODNN7EXAMPLE")
    );
}

fn signed_thinking_stream() -> Vec<u8> {
    build_bedrock_stream(&[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"reasoningContent":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"text":"fixture thought"}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"reasoningContent":{"signature":"c2ln"}},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":1}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"answer"},"contentBlockIndex":1}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":1}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
        (
            "metadata",
            r#"{"usage":{"inputTokens":3,"outputTokens":4}}"#,
        ),
    ])
}

#[tokio::test]
async fn node5_placeholder_signing_rejects_selected_standard_model_endpoint_before_payload() {
    let server = MockServer::start().await;
    let client = Arc::new(
        HttpClientBuilder::new()
            .tls(TlsOptions::new().http1_only())
            .build()
            .unwrap(),
    );
    let provider =
        BedrockProvider::new(Some(server.uri()), client.clone()).with_placeholder_client(client);
    let mut model = provider.models()[0].clone();
    model.base_url = Some("https://bedrock-runtime.us-east-1.amazonaws.com".into());
    let mut request = text_stream_request();
    request.model = model.id.clone();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let calls = callbacks.clone();
    // A failing callback is an independent no-network backstop if the
    // endpoint guard regresses; this test never contacts the named host.
    request.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            async {
                Err(opi_ai::callbacks::CallbackError::new(
                    "fixture network backstop",
                ))
            }
        });
    let auth =
        opi_ai::providers::bedrock::proxy_placeholder_auth(&server.uri(), "us-east-1").unwrap();
    let result = collect_outcome(
        provider
            .with_models(vec![model])
            .stream_prepared(request, auth),
    )
    .await;
    assert!(
        matches!(result, Err(ref error) if matches!(direct_error_cause(error), ProviderError::Config(_)))
    );
    assert_eq!(callbacks.load(Ordering::SeqCst), 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn node5_builtin_bedrock_stored_bearer_suppresses_native_sources() {
    use opi_ai::credential::{Credential, CredentialStore};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            signed_thinking_stream(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(1)
        .mount(&server)
        .await;
    let context = Arc::new(Sources::default());
    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
    store
        .write(
            "amazon-bedrock",
            &Credential::ApiKey("stored-bearer".into()),
        )
        .await
        .unwrap();
    let mut models = Models::new();
    models.set_credential_store(store);
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.region = Some("us-east-1".into());
    overrides
        .base_urls
        .insert("amazon-bedrock".into(), server.uri());
    register_builtin(
        &mut models,
        context.clone(),
        Arc::new(HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let id = &Catalog::builtin()
        .unwrap()
        .models("amazon-bedrock", CatalogOperation::Chat)
        .unwrap()
        .find(|model| model.id.contains("claude-sonnet-4"))
        .unwrap()
        .id;
    let model = models.get_model("amazon-bedrock", id).unwrap();
    let mut request = text_stream_request();
    request.model = format!("amazon-bedrock:{id}");
    models
        .complete_simple(
            opi_ai::ChatModelRecord::new("amazon-bedrock", model),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert!(
        context.reads.lock().unwrap().is_empty(),
        "stored bearer and explicit region require no native source discovery"
    );
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].headers["authorization"], "Bearer stored-bearer");
    assert!(!sent[0].headers.contains_key("x-amz-date"));
    assert!(!sent[0].headers.contains_key("x-amz-security-token"));
}

#[tokio::test]
async fn node5_builtin_bedrock_simple_tool_choice_reaches_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            signed_thinking_stream(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(2)
        .mount(&server)
        .await;
    let context = Arc::new(Sources {
        values: BTreeMap::from([("AWS_BEARER_TOKEN_BEDROCK".into(), "fixture-bearer".into())]),
        ..Default::default()
    });
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.region = Some("us-east-1".into());
    overrides
        .base_urls
        .insert("amazon-bedrock".into(), server.uri());
    let mut models = Models::new();
    register_builtin(
        &mut models,
        context,
        Arc::new(HttpClient::new()),
        &overrides,
    )
    .unwrap();
    let id = &Catalog::builtin()
        .unwrap()
        .models("amazon-bedrock", CatalogOperation::Chat)
        .unwrap()
        .find(|model| model.id.contains("claude-sonnet-4"))
        .unwrap()
        .id;
    let model = models.get_model("amazon-bedrock", id).unwrap();
    for choice in [
        opi_ai::SimpleToolChoice::Auto,
        opi_ai::SimpleToolChoice::None,
    ] {
        let mut request = text_stream_request();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: request.context.messages().to_vec(),
            tools: vec![ToolDef {
                name: "lookup".into(),
                description: "fixture".into(),
                input_schema: serde_json::json!({"type":"object","properties":{}}),
                constrained_sampling: None,
            }],
        })
        .unwrap();
        models
            .complete_simple(
                opi_ai::ChatModelRecord::new("amazon-bedrock", model.clone()),
                request,
                opi_ai::SimpleOptions {
                    tool_choice: Some(choice),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    }
    let sent = server.received_requests().await.unwrap();
    let auto: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    let none: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(
        auto["toolConfig"]["toolChoice"],
        serde_json::json!({"auto":{}})
    );
    assert!(none.get("toolConfig").is_none());
}

#[tokio::test]
async fn builtin_bedrock_dispatches_published_models_with_strict_tools_and_effort() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::model_info::ThinkingLevel;
    use opi_ai::provider::ThinkingConfig;
    use serde_json::json;

    let cases = [
        ("anthropic.claude-fable-5", ThinkingLevel::Max),
        ("au.anthropic.claude-sonnet-5-5", ThinkingLevel::Max),
        ("global.openai.gpt-6.1-terra", ThinkingLevel::XHigh),
        ("openai.gpt-6.1-terra", ThinkingLevel::XHigh),
        ("us.openai.gpt-6.1-terra", ThinkingLevel::XHigh),
    ];
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            signed_thinking_stream(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(cases.len() as u64)
        .mount(&server)
        .await;
    let auth = Arc::new(Sources {
        values: BTreeMap::from([("AWS_BEARER_TOKEN_BEDROCK".into(), "fixture-bearer".into())]),
        ..Default::default()
    });
    let mut overrides = BuiltinOverrides::default();
    overrides.aws_auth.region = Some("us-east-1".into());
    overrides
        .base_urls
        .insert("amazon-bedrock".into(), server.uri());
    let mut models = Models::new();
    register_builtin(&mut models, auth, Arc::new(HttpClient::new()), &overrides).unwrap();

    let mut request = text_stream_request();
    let schema = json!({
        "type":"object",
        "properties":{"query":{"type":"string"}}
    });
    let ordinary = ToolDef {
        name: "search".into(),
        description: "Search the fixture".into(),
        input_schema: schema.clone(),
        constrained_sampling: None,
    };
    let strict = ToolDef {
        name: "strict_search".into(),
        constrained_sampling: Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        }),
        ..ordinary.clone()
    };
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: request.context.messages().to_vec(),
        tools: vec![strict, ordinary],
    })
    .unwrap();
    for (id, level) in cases {
        let model = models
            .get_model("amazon-bedrock", id)
            .unwrap_or_else(|| panic!("missing builtin Bedrock model {id}"));
        let mut call = request.clone();
        call.thinking = ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        };
        let result = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("amazon-bedrock", model),
                call,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(result.stop_reason, StopReason::Stop, "{id}");
        assert_eq!(result.provider, "amazon-bedrock");
        assert_eq!(result.model, id);
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), cases.len());
    for (request, (id, level)) in sent.iter().zip(cases) {
        assert_eq!(request.url.path(), format!("/model/{id}/converse-stream"));
        assert_eq!(request.headers["authorization"], "Bearer fixture-bearer");
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let strict = &body["toolConfig"]["tools"][0]["toolSpec"];
        assert_eq!(strict["strict"], true, "{id}");
        assert_eq!(
            strict["inputSchema"]["json"],
            json!({
                "type": "object",
                "additionalProperties": false,
                "required": ["query"],
                "properties": {"query":{"anyOf": [{"type":"string"}, {"type":"null"}]}}
            }),
            "{id}"
        );
        let ordinary = &body["toolConfig"]["tools"][1]["toolSpec"];
        assert!(ordinary.get("strict").is_none(), "{id}");
        assert_eq!(ordinary["inputSchema"]["json"], schema);
        let expected_fields = match level {
            ThinkingLevel::Max => json!({
                "thinking": {
                    "type": "adaptive",
                    "display": "summarized",
                    "block_binding": {"prefix_mismatch_behavior": "drop_block"}
                },
                "output_config": {"effort": "max"},
                "anthropic_beta": ["thinking-binding-controls-2026-08-01"]
            }),
            ThinkingLevel::XHigh => json!({"reasoning":{"effort":"xhigh"}}),
            _ => unreachable!(),
        };
        assert_eq!(
            body["additionalModelRequestFields"], expected_fields,
            "{id}"
        );
    }
}

#[test]
fn bedrock_direct_catalog_uses_all_reviewed_models() {
    use opi_ai::providers::bedrock::{is_standard_bedrock_endpoint, model_catalog};

    let builtin = Catalog::builtin().unwrap();
    let expected = builtin
        .models("amazon-bedrock", CatalogOperation::Chat)
        .unwrap()
        .map(|entry| {
            let mut model = entry.to_chat_model_info().unwrap();
            if model
                .base_url
                .as_deref()
                .is_some_and(is_standard_bedrock_endpoint)
            {
                model.base_url = None;
            }
            model
        })
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 197);
    let direct = BedrockProvider::new(None, Arc::new(HttpClient::new()));
    assert_eq!(direct.models().len(), expected.len());
    assert_eq!(direct.models(), expected);
    assert_eq!(model_catalog(), expected);
}

#[test]
fn builtin_bedrock_catalog_ids_are_admitted_to_their_wire() {
    let catalog = Catalog::builtin().unwrap();
    let models = catalog
        .models("amazon-bedrock", CatalogOperation::Chat)
        .unwrap()
        .map(|entry| entry.to_chat_model_info().unwrap())
        .collect::<Vec<_>>();
    assert!(!models.is_empty());
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new())).with_models(models);
    for model in provider.models() {
        provider
            .validate_model_id(&model.id)
            .unwrap_or_else(|error| {
                panic!("catalog model {} rejected: {error}", model.id);
            });
    }
    assert!(
        provider
            .validate_model_id("unlisted.bedrock-model")
            .is_err()
    );
}

#[tokio::test]
async fn node5_builtin_bedrock_h2_h1_signed_capture_reload_replay() {
    for h2 in [true, false] {
        let fallback = MockServer::start().await;
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let mut config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![
                    CertificateDer::from_pem_slice(
                        builtin_tls_fixtures::CERT_SERVER_RSA.as_bytes(),
                    )
                    .unwrap(),
                ],
                PrivateKeyDer::from_pem_slice(builtin_tls_fixtures::KEY_SERVER_RSA.as_bytes())
                    .unwrap(),
            )
            .unwrap();
        config.alpn_protocols = vec![if h2 {
            b"h2".to_vec()
        } else {
            b"http/1.1".to_vec()
        }];
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}",
            listener.local_addr().unwrap().port()
        );
        let captured = Arc::new(Mutex::new(Vec::new()));
        let received = captured.clone();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    accepted = listener.accept() => {
                        let (tcp, _) = accepted.unwrap();
                        let acceptor = acceptor.clone();
                        let received = received.clone();
                        connections.spawn(async move {
                            let tls = acceptor.accept(tcp).await.unwrap();
                            assert_eq!(tls.get_ref().1.alpn_protocol(), Some(if h2 { b"h2".as_slice() } else { b"http/1.1".as_slice() }));
                            let service = service_fn(move |request: HttpRequest<Incoming>| {
                                let received = received.clone();
                                async move {
                                    let (parts, body) = request.into_parts();
                                    let body = body.collect().await.unwrap().to_bytes();
                                    received.lock().unwrap().push((parts, body));
                                    Ok::<_, std::convert::Infallible>(Response::builder().status(200).header("content-type", "application/vnd.amazon.eventstream").body(Full::new(hyper::body::Bytes::from(signed_thinking_stream()))).unwrap())
                                }
                            });
                            if h2 { let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new()).serve_connection(TokioIo::new(tls), service).await; }
                            else { let _ = hyper::server::conn::http1::Builder::new().keep_alive(false).serve_connection(TokioIo::new(tls), service).await; }
                        });
                    }
                }
            }
            connections.abort_all();
            while connections.join_next().await.is_some() {}
        });
        let context = Arc::new(Sources {
            values: BTreeMap::from([
                ("AWS_ACCESS_KEY_ID".into(), "ISOLATED_ACCESS".into()),
                ("AWS_SECRET_ACCESS_KEY".into(), "isolated-secret".into()),
                ("AWS_SESSION_TOKEN".into(), "isolated-session".into()),
                ("AWS_REGION".into(), "eu-west-1".into()),
            ]),
            ..Default::default()
        });
        let mut overrides = BuiltinOverrides::default();
        overrides.aws_auth.base_url = Some(endpoint);
        overrides
            .base_urls
            .insert("amazon-bedrock".into(), fallback.uri());
        let client = HttpClientBuilder::new()
            .tls(
                TlsOptions::new()
                    .with_explicit_roots_pem(builtin_tls_fixtures::CERT_CA.as_bytes())
                    .unwrap(),
            )
            .build()
            .unwrap();
        let mut models = Models::new();
        register_builtin(&mut models, context.clone(), Arc::new(client), &overrides).unwrap();
        let id = &Catalog::builtin()
            .unwrap()
            .models("amazon-bedrock", CatalogOperation::Chat)
            .unwrap()
            .find(|model| model.id.contains("claude-sonnet-4") && !model.id.starts_with("us."))
            .unwrap()
            .id;
        let mut model = models.get_model("amazon-bedrock", id).unwrap();
        assert_eq!(model.base_url.as_deref(), Some(fallback.uri().as_str()));
        model.base_url = None;
        model.headers = vec![
            ("x-model-proof".into(), Some("selected-record".into())),
            ("x-priority".into(), Some("model".into())),
            ("x-removed".into(), Some("gone".into())),
            ("x-removed".into(), None),
        ];
        assert!(context.reads.lock().unwrap().is_empty());
        let mut request = text_stream_request();
        request.model = format!("amazon-bedrock:{id}");
        request
            .extra_headers
            .push(("x-priority".into(), "request".into()));
        let call = models.complete_simple(
            opi_ai::ChatModelRecord::new("amazon-bedrock", model.clone()),
            request.clone(),
            Default::default(),
        );
        assert!(context.reads.lock().unwrap().is_empty());
        let first = tokio::time::timeout(std::time::Duration::from_secs(10), call)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.provider, "amazon-bedrock");
        assert_eq!(first.raw_stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(first.usage.input_tokens, 3);
        assert!(
            matches!(&first.content[0], AssistantContent::Thinking { thinking, signature } if thinking == "fixture thought" && signature.as_deref() == Some("c2ln"))
        );
        let reloaded = serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            tools: vec![],
            messages: vec![
                Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "go".into() }],
                    timestamp_ms: 0,
                }),
                Message::Assistant(reloaded),
                Message::User(UserMessage {
                    content: vec![InputContent::Text {
                        text: "next".into(),
                    }],
                    timestamp_ms: 0,
                }),
            ],
        })
        .unwrap();
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            models.complete_simple(
                opi_ai::ChatModelRecord::new("amazon-bedrock", model),
                request,
                Default::default(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        let _ = stop_tx.send(());
        server.await.unwrap();
        assert!(fallback.received_requests().await.unwrap().is_empty());
        let received = captured.lock().unwrap();
        assert_eq!(received.len(), 2);
        for (parts, body) in received.iter() {
            assert_eq!(
                parts.version,
                if h2 {
                    hyper::Version::HTTP_2
                } else {
                    hyper::Version::HTTP_11
                }
            );
            assert_eq!(parts.headers["x-model-proof"], "selected-record");
            assert_eq!(parts.headers["x-priority"], "request");
            assert!(!parts.headers.contains_key("x-removed"));
            assert_eq!(parts.headers["x-amz-security-token"], "isolated-session");
            let authorization = parts.headers["authorization"].to_str().unwrap();
            let signed_names = authorization
                .split("SignedHeaders=")
                .nth(1)
                .unwrap()
                .split(',')
                .next()
                .unwrap();
            let headers: Vec<(&str, &str)> = signed_names
                .split(';')
                .filter(|name| *name != "x-amz-security-token")
                .map(|name| {
                    (
                        name,
                        parts
                            .headers
                            .get(name)
                            .map(|value| value.to_str().unwrap())
                            .or_else(|| {
                                (name == "host").then(|| parts.uri.authority().unwrap().as_str())
                            })
                            .unwrap(),
                    )
                })
                .collect();
            let date = parts.headers["x-amz-date"].to_str().unwrap();
            let credentials = AwsCredentials {
                access_key_id: "ISOLATED_ACCESS".into(),
                secret_access_key: "isolated-secret".into(),
                session_token: Some("isolated-session".into()),
                region: "eu-west-1".into(),
                expires_at: None,
            };
            let expected = opi_ai::auth::aws::sigv4::sign_request(
                "POST",
                parts.uri.path(),
                "",
                &headers,
                body,
                &credentials,
                "bedrock",
                &date[..8],
                date,
            );
            assert_eq!(
                authorization,
                expected.authorization.expose_secret(),
                "actual payload and selected headers are signed"
            );
        }
        let replay: serde_json::Value = serde_json::from_slice(&received[1].1).unwrap();
        assert!(
            replay["messages"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|message| message["content"].as_array().unwrap())
                .any(|block| block
                    .pointer("/reasoningContent/reasoningText/signature")
                    .is_some_and(|value| value == "c2ln")),
            "canonical captured thinking is replayed: {replay}"
        );
        let reads = context.reads.lock().unwrap();
        assert!(
            !reads.iter().any(|name| name == "HOME"
                || name.contains("CONTAINER")
                || name.contains("METADATA")
                || name == "AWS_PROFILE"),
            "selected pair must not probe lower sources: {reads:?}"
        );
    }
}
