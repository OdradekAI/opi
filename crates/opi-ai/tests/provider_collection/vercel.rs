//! Vercel AI Gateway's builtin catalog selection and wire dispatch.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, map_context, register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierInputModality,
    ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::models::ModelsError;
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use opi_ai::stream::{Pricing, StopReason};
use opi_ai::{ChatModelRecord, ThinkingLevel, WireApi};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PROVIDER: &str = "vercel-ai-gateway";
const FIXTURE_KEY: &str = "fixture-vercel-key";

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("AI_GATEWAY_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

#[tokio::test]
async fn builtin_vercel_lists_the_same_chat_and_classifier_records_across_sdk_entries() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: serde_json::Value =
        serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/vercel-ai-gateway.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let chat: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .map(|entry| {
            assert_eq!(entry.api, "anthropic-messages");
            let info = entry.to_chat_model_info().unwrap();
            assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
            assert_eq!(
                models
                    .resolve(&format!("{PROVIDER}:{}", entry.id))
                    .unwrap()
                    .1,
                &info
            );
            CatalogRecord::from(info)
        })
        .collect();
    let classifiers: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Classifier)
        .unwrap()
        .map(|entry| {
            let record = models.classifier_model(PROVIDER, &entry.id).unwrap();
            assert_eq!(record.provider_id, PROVIDER);
            assert_eq!(record.id, entry.id);
            assert_eq!(record.api.as_str(), "typesafe-system-one");
            assert_eq!(record.name, entry.name);
            assert_eq!(record.base_url, entry.base_url);
            assert_eq!(record.input, [ClassifierInputModality::Text]);
            assert_eq!(record.context_window, entry.context_window.unwrap());
            assert!(record.headers.is_empty() && record.input_limits.is_none());
            let pricing = record.cost.as_ref().unwrap();
            let cost = entry.cost.as_ref().unwrap();
            assert_eq!(
                pricing.base,
                Pricing {
                    input_cost_per_mtok: cost.input,
                    output_cost_per_mtok: cost.output,
                    cache_read_cost_per_mtok: cost.cache_read,
                    cache_write_cost_per_mtok: cost.cache_write,
                },
                "{}",
                entry.id
            );
            assert!(pricing.tiers.is_empty());
            CatalogRecord::Classifier(record)
        })
        .collect();
    assert_eq!((chat.len(), classifiers.len()), (303, 14));

    for (operation, expected) in [
        (CatalogOperation::Chat, &chat),
        (CatalogOperation::Classifier, &classifiers),
    ] {
        assert_eq!(
            &models.get_models_of_type(operation, Some(PROVIDER)),
            expected
        );
        assert_eq!(
            &models
                .get_available_of_type(operation, Some(PROVIDER))
                .await
                .unwrap(),
            expected
        );
        for record in expected {
            assert_eq!(
                models
                    .get_model_of_type(operation, PROVIDER, record.id())
                    .as_ref(),
                Some(record)
            );
        }
    }
    assert_eq!(
        models.get_all_models(Some(PROVIDER)),
        [chat, classifiers].concat()
    );
}

/// Redirect a selected catalog endpoint to the fixture while keeping its path.
fn loopback_base(server: &MockServer, published_base: &str) -> String {
    let published = url::Url::parse(published_base).unwrap();
    assert_eq!(published.scheme(), "https");
    assert_eq!(published.host_str(), Some("ai-gateway.vercel.sh"));
    format!("{}{}", server.uri(), published.path().trim_end_matches('/'))
}

#[tokio::test]
async fn builtin_vercel_chat_uses_messages_beta_and_the_selected_thinking_budget() {
    let server = MockServer::start().await;
    let models = builtin_models(&BuiltinOverrides::default());
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(3)
        .mount(&server)
        .await;

    let cases = [
        (
            "openai/gpt-6.1-terra",
            ThinkingLevel::XHigh,
            Some(4_096),
            20_480,
            json!({"type":"enabled","budget_tokens":16_384,"display":"summarized"}),
            Some("interleaved-thinking-2025-05-14"),
        ),
        (
            "openai/gpt-6.1-terra",
            ThinkingLevel::None,
            Some(4_096),
            4_096,
            json!({"type":"disabled"}),
            None,
        ),
        (
            "moonshotai/kimi-k3.1-pro",
            ThinkingLevel::None,
            None,
            131_072,
            json!({"type":"disabled"}),
            None,
        ),
    ];
    for (id, level, limit, ..) in &cases {
        let mut info = models.get_model(PROVIDER, id).unwrap();
        info.base_url = Some(loopback_base(&server, info.base_url.as_deref().unwrap()));
        let mut request = super::minimal_request(&format!("{PROVIDER}:{id}"));
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 1,
            })],
            ..Default::default()
        })
        .unwrap();
        request.max_tokens = *limit;
        request.thinking = ThinkingConfig {
            enabled: *level != ThinkingLevel::None,
            level: *level,
            budget_tokens: None,
        };
        let output = models
            .complete_simple(
                ChatModelRecord::new(PROVIDER, info),
                request,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{id}, {level:?}: {error:?}"));
        assert_eq!(output.provider, PROVIDER);
        assert_eq!(output.model, *id);
        assert_eq!(output.stop_reason, StopReason::Stop);
        assert_eq!(super::assistant_text(&output), "hi");
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), cases.len());
    for (request, (id, level, _, limit, thinking, beta)) in sent.iter().zip(cases) {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            (
                request.url.query(),
                request
                    .headers
                    .get("anthropic-beta")
                    .map(|header| header.to_str().unwrap()),
                body.get("betas"),
                body["max_tokens"].as_u64(),
                &body["thinking"],
            ),
            (Some("beta=true"), beta, None, Some(limit), &thinking),
            "{id}, {level:?}"
        );
        assert_eq!(request.headers["x-api-key"], FIXTURE_KEY);
        assert_eq!(request.headers["anthropic-version"], "2023-06-01");
        assert!(!request.headers.contains_key("authorization"));
        assert_eq!(body["model"], id);
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"][0]["content"][0]["text"], "hello");
        assert!(body["output_config"].get("effort").is_none());
    }
}

#[tokio::test]
async fn builtin_vercel_classifies_on_system_one_with_the_gateway_key() {
    let server = MockServer::start().await;
    let models = builtin_models(&BuiltinOverrides::default());
    Mock::given(method("POST"))
        .and(path("/typesafe/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {"ok": {"type":"noul","noul":0.91}},
            "usage": {"input_tokens":12,"output_tokens":1}
        })))
        .expect(2)
        .mount(&server)
        .await;

    let inputs = ["typesafe-ai/jev-1.14.0", "baseten/elephant-1"];
    let context = ClassifierContext {
        state: json!({"text":"hello"}),
        questions: [(
            "ok".into(),
            ClassifierQuestion::Bool {
                instructions: "Is it a greeting?".into(),
                true_criteria: "Greeting".into(),
                false_criteria: "Other".into(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    };
    for id in inputs {
        let mut record = models.classifier_model(PROVIDER, id).unwrap();
        record.base_url = loopback_base(&server, &record.base_url);
        let result = models
            .classify(record, context.clone(), ClassifierCall::default())
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
        assert_eq!(result.provider, PROVIDER);
        assert_eq!(result.model, id);
        assert_eq!(result.api.as_str(), "typesafe-system-one");
        assert_eq!(
            result.answers["ok"],
            ClassifierAnswer::Bool { probability: 0.91 }
        );
        assert_eq!(result.usage.unwrap().input_tokens, 12);
    }

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), inputs.len());
    for (request, id) in sent.iter().zip(inputs) {
        assert!(request.url.query().is_none());
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {FIXTURE_KEY}")
        );
        assert!(!request.headers.contains_key("x-api-key"));
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            body,
            json!({
                "model": id,
                "state": context.state,
                "questions": {"ok": {
                    "type": "noul",
                    "instructions": "Is it a greeting?",
                    "criteria": {"true": "Greeting", "false": "Other"}
                }}
            })
        );
    }
}

#[tokio::test]
async fn builtin_vercel_retired_models_fail_selection_without_sending_requests() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(PROVIDER.into(), server.uri());
    let models = builtin_models(&overrides);
    let catalog = Catalog::builtin().unwrap();
    let all = models.get_all_models(Some(PROVIDER));
    let retired: [(CatalogOperation, &[&str]); 2] = [
        (
            CatalogOperation::Chat,
            &[
                "anthropic/claude-sonnet-4",
                "arcee-ai/trinity-large-thinking",
                "openai/gpt-3.5-turbo",
                "openai/gpt-4-turbo",
                "openai/gpt-4.1-nano-batch",
                "openai/gpt-4.1-nano-fast",
                "openai/o4-mini-fast",
                "poolside/laguna-s-2.1-free",
                "spacexai/grok-4.1-fast-non-reasoning",
                "spacexai/grok-4.1-fast-reasoning",
            ],
        ),
        (
            CatalogOperation::Classifier,
            &[
                "convaiinnovations/laya-free",
                "perplexity/pplx-decider-v1.1-27b",
            ],
        ),
    ];
    for (operation, ids) in retired {
        let available = models
            .get_available_of_type(operation, Some(PROVIDER))
            .await
            .unwrap();
        for &id in ids {
            assert!(catalog.model(PROVIDER, operation, id).is_none(), "{id}");
            assert!(all.iter().all(|record| record.id() != id), "{id}");
            assert!(available.iter().all(|record| record.id() != id), "{id}");
            assert!(
                models.get_model_of_type(operation, PROVIDER, id).is_none(),
                "{id}"
            );
            if operation == CatalogOperation::Chat {
                let spec = format!("{PROVIDER}:{id}");
                assert!(models.get_model(PROVIDER, id).is_none(), "{id}");
                assert!(
                    matches!(
                        models.prepare_call(&spec, super::minimal_request(&spec)).await,
                        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
                        if provider == PROVIDER && model == id
                    ),
                    "{id}"
                );
            } else {
                assert!(
                    matches!(
                        models.classifier_model(PROVIDER, id),
                        Err(ModelsError::NotAClassifierModel { provider, id: model })
                        if provider == PROVIDER && model == id
                    ),
                    "{id}"
                );
            }
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
