//! Built-in classifier and image selection, authentication and dispatch.

use super::*;

#[tokio::test]
async fn classifier_and_image_builtins_register_from_the_catalog() {
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        map_context(&[
            ("TYPESAFE_API_KEY", "ts-key"),
            ("OPENAI_API_KEY", "sk-openai"),
            ("CLOUDFLARE_API_KEY", "cf-key"),
        ]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert!(
        report
            .classifier_apis
            .iter()
            .any(|api| api == "typesafe-system-one")
    );
    assert!(
        report
            .classifier_apis
            .iter()
            .any(|api| api == "openai-decisions")
    );
    assert!(
        report
            .image_apis
            .iter()
            .any(|api| api == "openrouter-images")
    );

    let typesafe_record = models
        .classifier_model("typesafe", "jev-latest")
        .expect("catalog classifier record registers");
    assert_eq!(typesafe_record.provider_id, "typesafe");
    let catalog = Catalog::builtin().unwrap();
    let openrouter_image = catalog
        .models("openrouter", CatalogOperation::Image)
        .unwrap()
        .next()
        .unwrap()
        .id
        .clone();
    let image = models
        .image_model("openrouter", &openrouter_image)
        .expect("catalog image record registers");
    assert_eq!(image.provider_id, "openrouter");
}

#[tokio::test]
async fn classifier_same_api_keeps_provider_auth_bindings() {
    use opi_ai::api::system_one::{TypesafeSystemOneApi, default_typesafe_classifier_record};
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method},
    };
    let mut models = Models::new();
    let mut records = Vec::new();
    let mut servers = Vec::new();
    for (provider, key) in [("provider-a", "fixture-a"), ("provider-b", "fixture-b")] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("authorization", format!("Bearer {key}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"answers":{}})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut record = default_typesafe_classifier_record();
        record.provider_id = provider.into();
        record.base_url = server.uri();
        models
            .register_classifier_api(
                Arc::new(TypesafeSystemOneApi::new(
                    opi_ai::http::HttpClient::new(),
                    record.clone(),
                )),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from(key),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
        records.push(record);
        servers.push(server);
    }
    for record in records {
        assert!(
            models
                .classifier_model(&record.provider_id, &record.id)
                .is_ok()
        );
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall::default(),
            )
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
    }
    for server in servers {
        server.verify().await;
    }
}

#[tokio::test]
async fn classifier_model_modality_is_rejected_before_authentication() {
    use opi_ai::api::openai_decisions::{OpenAiDecisionsApi, default_decisions_classifier_record};
    use opi_ai::classifier::{
        ClassifierCall, ClassifierContext, ClassifierImage, ClassifierInputModality,
        ClassifierStopReason,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Probe(Arc<AtomicUsize>);
    impl AuthResolver for Probe {
        fn resolve<'a>(
            &'a self,
        ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {
                Err(ProviderError::CredentialNeeded {
                    provider_id: "fixture".into(),
                })
            })
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    let mut record = default_decisions_classifier_record();
    record.input = vec![ClassifierInputModality::Text];
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(OpenAiDecisionsApi::new(
                opi_ai::http::HttpClient::new(),
                record.clone(),
            )),
            Arc::new(Probe(count.clone())),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let result = models
        .classify(
            record,
            ClassifierContext {
                images: Some(vec![ClassifierImage {
                    mime_type: "image/png".into(),
                    data: "eA==".into(),
                }]),
                ..Default::default()
            },
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability(_))
    ));
}

#[test]
fn builtin_all_operation_records_are_available_for_selection() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let catalog = Catalog::builtin().unwrap();
    let mut missing = Vec::new();
    for provider in catalog.providers() {
        for operation in [CatalogOperation::Classifier, CatalogOperation::Image] {
            for entry in catalog.models(provider, operation).unwrap() {
                let found = match operation {
                    CatalogOperation::Classifier => {
                        models.classifier_model(provider, &entry.id).is_ok()
                    }
                    CatalogOperation::Image => models.image_model(provider, &entry.id).is_ok(),
                    CatalogOperation::Chat => unreachable!(),
                };
                if !found {
                    missing.push(format!("{provider}:{}:{operation:?}", entry.id));
                }
            }
        }
    }
    assert!(missing.is_empty(), "missing operation records: {missing:?}");
}

#[tokio::test]
async fn builtin_openrouter_uses_synced_chat_and_image_routes() {
    use opi_ai::images::{ImagesContext, ImagesInput, ImagesOutput, ImagesStopReason};
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "fixture-openrouter"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer fixture-openrouter"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"images": [{"image_url": "data:image/png;base64,QUJD"}]}}],
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("openrouter".into(), server.uri());
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("OPENROUTER_API_KEY", "fixture-openrouter")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    assert!(
        models
            .get_model("openrouter", "deepseek/deepseek-r1")
            .is_none()
    );
    let opus = "~anthropic/claude-opus-latest";
    let haiku = "~anthropic/claude-haiku-latest";
    for (id, enabled) in [(opus, true), (haiku, false)] {
        let model = models.get_model("openrouter", id).unwrap();
        assert_eq!(model.wire_api, WireApi::AnthropicMessages);
        let mut request = minimal_request(id);
        request.thinking.enabled = enabled;
        request.thinking.level = opi_ai::ThinkingLevel::Low;
        let reply = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("openrouter", model),
                request,
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(assistant_text(&reply), "hi");
    }

    let flux = "black-forest-labs/flux-3-image-klein";
    let record = models.image_model("openrouter", flux).unwrap();
    let generated = models
        .generate_images(
            record,
            ImagesContext {
                input: vec![ImagesInput::Text {
                    text: "a red square".into(),
                }],
            },
            Default::default(),
        )
        .await;
    assert_eq!(
        generated.stop_reason,
        ImagesStopReason::Stop,
        "{:?}",
        generated.error
    );
    assert_eq!(
        generated.output,
        [ImagesOutput::Image {
            mime_type: "image/png".into(),
            data: "QUJD".into(),
        }]
    );
    let sent = server.received_requests().await.unwrap();
    let chat: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(chat["model"], opus);
    assert_eq!(chat["thinking"]["type"], "adaptive");
    assert_eq!(chat["output_config"]["effort"], "low");
    let disabled: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(disabled["model"], haiku);
    assert_eq!(disabled["thinking"]["type"], "disabled");
    assert!(disabled.get("output_config").is_none());
    let image: serde_json::Value = serde_json::from_slice(&sent[2].body).unwrap();
    assert_eq!(image["model"], flux);
    assert_eq!(image["modalities"], json!(["image"]));
}

#[tokio::test]
async fn builtin_classifier_profiles_send_the_selected_record_with_stored_auth() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let catalog = Catalog::builtin().unwrap();
    for provider in [
        "typesafe",
        "openrouter",
        "opencode",
        "vercel-ai-gateway",
        "cloudflare-workers-ai",
        "openai",
    ] {
        let server = MockServer::start().await;
        let response = match provider {
            "cloudflare-workers-ai" => {
                serde_json::json!({"success":true,"result":{"answers":{},"usage":{"input_tokens":2,"output_tokens":1}}})
            }
            "openai" => {
                serde_json::json!({"answers":[],"usage":{"input_tokens":2,"output_tokens":1}})
            }
            _ => serde_json::json!({"answers":{},"usage":{"input_tokens":2,"output_tokens":1}}),
        };
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .expect(1)
            .mount(&server)
            .await;
        let store = Arc::new(FakeStore {
            entries: Mutex::new(HashMap::from([(
                provider.into(),
                Credential::ApiKey("stored-classifier".into()),
            )])),
        });
        let mut models = Models::new();
        models.set_credential_store(store);
        register_builtin(
            &mut models,
            map_context(&[("CLOUDFLARE_ACCOUNT_ID", "fixture")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let entry = catalog
            .models(provider, CatalogOperation::Classifier)
            .unwrap()
            .last()
            .unwrap();
        let mut record = models.classifier_model(provider, &entry.id).unwrap();
        record.base_url = format!("{}/custom-classifier", server.uri());
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall::default(),
            )
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{provider}: {:?}",
            result.error
        );
        assert_eq!(result.provider, provider);
        assert_eq!(result.model, entry.id);
        assert_eq!(result.usage.unwrap().input_tokens, 2);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent[0].headers["authorization"], "Bearer stored-classifier");
        assert!(sent[0].url.path().starts_with("/custom-classifier/"));
        assert!(
            models
                .get_auth(provider, Default::default())
                .await
                .unwrap()
                .is_some(),
            "{provider} management auth must expose its configured source"
        );
    }
}

#[tokio::test]
async fn builtin_openrouter_classifier_sends_images_before_state() {
    use opi_ai::classifier::{
        ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImage, ClassifierQuestion,
        ClassifierStopReason,
    };
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/systemone"))
        .and(header("authorization", "Bearer fixture-openrouter"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {"red": {"type": "noul", "noul": 0.92}},
            "usage": {"input_tokens": 12, "output_tokens": 1},
        })))
        .expect(1)
        .mount(&server)
        .await;
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("openrouter".into(), format!("{}/api/v1", server.uri()));
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("OPENROUTER_API_KEY", "fixture-openrouter")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    let state = json!([{"task": "judge the first image"}, "fixture"]);
    let red = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4z8AAAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
    let blue = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGNgYPgPAAEDAQAIicLsAAAAAElFTkSuQmCC";
    let context = ClassifierContext {
        state: state.clone(),
        images: Some(
            [red, blue]
                .into_iter()
                .map(|data| ClassifierImage {
                    mime_type: "image/png".into(),
                    data: data.into(),
                })
                .collect(),
        ),
        questions: [(
            "red".into(),
            ClassifierQuestion::Bool {
                instructions: "Is the first image red?".into(),
                true_criteria: "Red".into(),
                false_criteria: "Other color".into(),
            },
        )]
        .into_iter()
        .collect(),
    };
    let record = models
        .classifier_model("openrouter", "typesafe/jev-1.14")
        .unwrap();
    let result = models
        .classify(record, context.clone(), ClassifierCall::default())
        .await;
    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "{:?}",
        result.error
    );
    assert_eq!(
        result.answers["red"],
        ClassifierAnswer::Bool { probability: 0.92 }
    );
    assert_eq!(result.usage.unwrap().input_tokens, 12);

    let text_only = models
        .classifier_model("openrouter", "respan/span-01")
        .unwrap();
    let result = models
        .classify(text_only, context, ClassifierCall::default())
        .await;
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability(_))
    ));

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["model"], "typesafe/jev-1.14");
    assert_eq!(body["questions"]["red"]["type"], "noul");
    assert_eq!(
        body["state"],
        json!([
            {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{red}")}},
            {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{blue}")}},
            state,
        ])
    );
    assert!(body.get("assets").is_none() && body.get("images").is_none());
    assert!(!sent[0].headers.contains_key("typesafe-danger-zone-images"));
}

#[tokio::test]
async fn builtin_nonstream_operations_reuse_the_injected_redirect_policy() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use opi_ai::images::{ImagesCall, ImagesContext, ImagesStopReason};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let origin = MockServer::start().await;
    let redirected = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(307)
                .insert_header("location", format!("{}/destination", redirected.uri())),
        )
        .mount(&origin)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"answers":{},"choices":[]})),
        )
        .mount(&redirected)
        .await;
    let client = opi_ai::http::HttpClientBuilder::new()
        .redirects(opi_ai::http::RedirectPolicy::None)
        .build()
        .unwrap();
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[
            ("TYPESAFE_API_KEY", "fixture"),
            ("OPENROUTER_API_KEY", "fixture"),
        ]),
        Arc::new(client),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let mut classifier = models.classifier_model("typesafe", "jev-latest").unwrap();
    classifier.base_url = origin.uri();
    let classified = models
        .classify(
            classifier,
            ClassifierContext::default(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(classified.stop_reason, ClassifierStopReason::Error);
    let entry = Catalog::builtin()
        .unwrap()
        .models("openrouter", CatalogOperation::Image)
        .unwrap()
        .last()
        .unwrap();
    let mut record = models.image_model("openrouter", &entry.id).unwrap();
    record.base_url = origin.uri();
    let generated = models
        .generate_images(record, ImagesContext::default(), ImagesCall::default())
        .await;
    assert_eq!(generated.stop_reason, ImagesStopReason::Error);
    assert_eq!(origin.received_requests().await.unwrap().len(), 2);
    assert!(
        redirected.received_requests().await.unwrap().is_empty(),
        "source policy forbids redirect transport sends"
    );
}
