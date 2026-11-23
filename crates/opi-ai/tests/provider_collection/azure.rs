//! Azure OpenAI endpoint and deployment selection through built-in assembly.

use super::*;

#[tokio::test]
async fn azure_builtin_registers_with_configured_endpoint() {
    let mut models = Models::new();
    // Fixture host per the acceptance-source hermeticity contract.
    let overrides = BuiltinOverrides {
        azure_endpoint: Some("https://azure.example.com".into()),
        ..BuiltinOverrides::default()
    };
    let report = register_builtin(
        &mut models,
        map_context(&[("AZURE_OPENAI_API_KEY", "azure-key")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .expect("builtin registration succeeds");
    assert!(report.registered.iter().any(|id| id == "azure"));
    let provider_ids = models.provider_ids();
    assert!(provider_ids.contains(&"azure"), "provider stays `azure`");
}

#[tokio::test]
async fn builtin_azure_chat_and_responses_resolve_source_endpoint_and_deployment() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct AzureEnv(String);
    impl opi_ai::auth::AuthContext for AzureEnv {
        fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(match name {
                "AZURE_OPENAI_API_KEY" => Some("azure-fixture".into()),
                "AZURE_OPENAI_BASE_URL" => Some(self.0.clone()),
                "AZURE_OPENAI_API_VERSION" => Some("fixture-version".into()),
                "AZURE_OPENAI_DEPLOYMENT_NAME_MAP" => Some("catalog-id=actual-deployment".into()),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            Ok(false)
        }
    }
    for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let unused = MockServer::start().await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            Arc::new(AzureEnv(format!("{}/openai/v1", server.uri()))),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let model = ModelInfo::new(
            "catalog-id",
            "Catalog identity",
            wire,
            ModelCapabilities::new(2048, 128),
        )
        .with_base_url(unused.uri());
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("azure", model),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(output.provider, "azure");
        assert_eq!(output.model, "catalog-id");
        assert!(unused.received_requests().await.unwrap().is_empty());
        let sent = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(body["model"], "actual-deployment");
        if wire == WireApi::OpenAiCompletions {
            assert_eq!(sent[0].url.path(), "/openai/v1/chat/completions");
            assert_eq!(sent[0].headers["authorization"], "Bearer azure-fixture");
        } else {
            assert_eq!(sent[0].url.path(), "/openai/v1/responses");
            assert!(
                sent[0]
                    .url
                    .query_pairs()
                    .any(|(key, value)| key == "api-version" && value == "fixture-version")
            );
            assert_eq!(sent[0].headers["api-key"], "azure-fixture");
        }
    }
}

#[tokio::test]
async fn builtin_azure_freezes_endpoint_deployment_and_payload_across_retries() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    struct ChangingSource {
        bases: [String; 2],
        reads: AtomicUsize,
        model: String,
    }
    impl opi_ai::auth::AuthContext for ChangingSource {
        fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(match name {
                "AZURE_OPENAI_API_KEY" => Some("isolated-key".into()),
                "AZURE_OPENAI_BASE_URL" => {
                    Some(self.bases[self.reads.fetch_add(1, Ordering::SeqCst).min(1)].clone())
                }
                "AZURE_OPENAI_DEPLOYMENT_NAME_MAP" => Some(format!(
                    "{}=deployment-{}",
                    self.model,
                    self.reads.load(Ordering::SeqCst)
                )),
                _ => None,
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            panic!("unselected source")
        }
    }
    for wire in [WireApi::OpenAiCompletions, WireApi::AzureOpenAiResponses] {
        for advanced in [false, true] {
            let first = MockServer::start().await;
            let second = MockServer::start().await;
            let sends = Arc::new(AtomicUsize::new(0));
            let count = sends.clone();
            Mock::given(method("POST"))
                .respond_with(move |_: &wiremock::Request| {
                    if count.fetch_add(1, Ordering::SeqCst) == 0 && advanced {
                        ResponseTemplate::new(429).insert_header("retry-after", "0")
                    } else {
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(node5_reply(wire))
                    }
                })
                .expect(if advanced { 2 } else { 1 })
                .mount(&first)
                .await;
            let catalog = Catalog::builtin().unwrap();
            let entry = catalog
                .models("azure", CatalogOperation::Chat)
                .unwrap()
                .find(|entry| entry.api == wire.to_string())
                .unwrap();
            let source = Arc::new(ChangingSource {
                bases: [first.uri(), second.uri()],
                reads: AtomicUsize::new(0),
                model: entry.id.clone(),
            });
            let mut models = Models::new();
            let overrides = BuiltinOverrides {
                azure_endpoint: Some(first.uri()),
                ..Default::default()
            };
            register_builtin(
                &mut models,
                source.clone(),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            let spec = format!("azure:{}", entry.id);
            let mut request = minimal_request(&spec);
            let payloads = Arc::new(AtomicUsize::new(0));
            let calls = payloads.clone();
            request.callbacks =
                opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    async { Ok(None) }
                });
            if advanced {
                let prepared = models
                    .prepare_call_with_policy(
                        &spec,
                        request,
                        opi_ai::retry::AcquisitionPolicy {
                            max_retries: 1,
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap();
                assert!(
                    opi_ai::ChatStream::new(prepared.start_attempt().unwrap())
                        .result()
                        .await
                        .is_err()
                );
                assert!(prepared.retry_admission().is_some());
                opi_ai::ChatStream::new(prepared.start_attempt().unwrap())
                    .result()
                    .await
                    .unwrap();
            } else {
                let model = models.get_model("azure", &entry.id).unwrap();
                models
                    .complete_simple(
                        opi_ai::ChatModelRecord::new("azure", model),
                        request,
                        Default::default(),
                    )
                    .await
                    .unwrap();
            }
            assert_eq!(source.reads.load(Ordering::SeqCst), 1, "{wire}/{advanced}");
            assert_eq!(payloads.load(Ordering::SeqCst), 1);
            assert!(second.received_requests().await.unwrap().is_empty());
            let sent = first.received_requests().await.unwrap();
            for request in &sent {
                let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(body["model"], "deployment-1");
            }
            if advanced {
                assert_eq!(sent[0].body, sent[1].body);
            }
        }
    }
}
