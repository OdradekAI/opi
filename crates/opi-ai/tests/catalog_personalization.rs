//! Real loopback evidence for explicit, account-local OpenRouter personalization.

use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation, personalize_openrouter_catalog};
use opi_ai::http::HttpClient;
use secrecy::SecretString;
use serde_json::json;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn personalization_keeps_finite_negative_and_fractional_price_thresholds() {
    let server = MockServer::start().await;
    let model = records()
        .into_iter()
        .find(|model| model.operation == CatalogOperation::Chat)
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/models/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{
            "id": model.id,
            "pricing": {"prompt": "0.000001", "completion": "0", "input_cache_read": "0", "input_cache_write": "0",
                "overrides": [
                    {"min_prompt_tokens": -2, "prompt": "0.000099"},
                    {"min_prompt_tokens": -0.5, "prompt": "0.000004"},
                    {"min_prompt_tokens": 0.5, "prompt": "0.000006"}
                ]}
        }]})))
        .expect(1)
        .mount(&server)
        .await;
    let models = personalize_openrouter_catalog(
        vec![model],
        &server.uri(),
        Some(&SecretString::from("fixture-key")),
        &HttpClient::new(),
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let tiers = models[0].raw["cost"]["tiers"]
        .as_array()
        .expect("tiers retained");
    assert_eq!(tiers.len(), 3);
    assert_eq!(tiers[0]["inputTokensAbove"], json!(-2));
    assert_eq!(tiers[1]["inputTokensAbove"], json!(-0.5));
    assert_eq!(tiers[2]["inputTokensAbove"], json!(0.5));
    let pricing = models[0].to_chat_model_info().unwrap().pricing.unwrap();
    assert_eq!(pricing.effective(0).input_cost_per_mtok, 4.0);
    assert_eq!(pricing.effective(1).input_cost_per_mtok, 6.0);
}

fn records() -> Vec<CatalogModel> {
    let catalog = Catalog::builtin().unwrap();
    [
        CatalogOperation::Chat,
        CatalogOperation::Image,
        CatalogOperation::Classifier,
    ]
    .into_iter()
    .flat_map(|operation| catalog.models("openrouter", operation).unwrap())
    .cloned()
    .collect()
}

#[tokio::test]
async fn explicit_personalization_uses_configured_endpoint_and_keeps_accounts_and_operations_separate()
 {
    let server = MockServer::start().await;
    let original = records();
    let chats: Vec<_> = original
        .iter()
        .filter(|model| model.operation == CatalogOperation::Chat)
        .take(2)
        .cloned()
        .collect();
    for (key, model, context) in [
        ("fixture-a", &chats[0], 77777_u64),
        ("fixture-b", &chats[1], 88888),
    ] {
        Mock::given(method("GET")).and(path("/region/models/user")).and(header("authorization", format!("Bearer {key}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": model.id, "context_length": 10,
                "top_provider": {"context_length": context, "max_completion_tokens": 1234},
                "pricing": {"prompt": "0.000002", "completion": "0.000003", "input_cache_read": "0", "input_cache_write": "0",
                    "overrides": [{"min_prompt_tokens": 100, "prompt": "0.000004"}, {"min_prompt_tokens": 100, "prompt": "0.000009"}, {"min_prompt_tokens": 1, "utc_start": 2, "prompt": "10"}]}
            }]}))).expect(1).mount(&server).await;
    }
    let client = HttpClient::new();
    for (key, selected, context) in [
        ("fixture-a", &chats[0], 77777_u64),
        ("fixture-b", &chats[1], 88888),
    ] {
        let personalized = personalize_openrouter_catalog(
            original.clone(),
            &format!("{}/region/", server.uri()),
            Some(&SecretString::from(key)),
            &client,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        let chat: Vec<_> = personalized
            .iter()
            .filter(|model| model.operation == CatalogOperation::Chat)
            .collect();
        assert_eq!(chat.len(), 1);
        assert_eq!(chat[0].id, selected.id);
        assert_eq!(chat[0].context_window, Some(context));
        assert_eq!(chat[0].max_tokens, Some(1234));
        let model = chat[0].to_chat_model_info().unwrap();
        let pricing = model.pricing.unwrap();
        assert_eq!(pricing.base.input_cost_per_mtok, 2.0);
        assert_eq!(pricing.tiers.len(), 2);
        assert_eq!(pricing.tiers[0].pricing.input_cost_per_mtok, 4.0);
        assert_eq!(pricing.tiers[1].pricing.input_cost_per_mtok, 9.0);
        let nonchat: Vec<_> = personalized
            .iter()
            .filter(|model| model.operation != CatalogOperation::Chat)
            .map(|model| &model.raw)
            .collect();
        assert_eq!(
            nonchat,
            original
                .iter()
                .filter(|model| model.operation != CatalogOperation::Chat)
                .map(|model| &model.raw)
                .collect::<Vec<_>>()
        );
    }
    assert_eq!(
        original.iter().map(|model| &model.raw).collect::<Vec<_>>(),
        records().iter().map(|model| &model.raw).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn absent_key_and_source_statuses_keep_catalog_while_invalid_or_cancelled_work_fails() {
    let server = MockServer::start().await;
    let original = records();
    let client = HttpClient::new();
    for key in [None, Some(SecretString::from(""))] {
        let result = personalize_openrouter_catalog(
            original.clone(),
            &server.uri(),
            key.as_ref(),
            &client,
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result.len(), original.len());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    for status in [401, 403, 404, 500] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(status))
            .expect(1)
            .mount(&server)
            .await;
        let result = personalize_openrouter_catalog(
            original.clone(),
            &server.uri(),
            Some(&SecretString::from("fixture")),
            &client,
            CancellationToken::new(),
        )
        .await;
        if status == 500 {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap().len(), original.len());
        }
        server.verify().await;
    }
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": {}})))
        .expect(1)
        .mount(&server)
        .await;
    assert!(
        personalize_openrouter_catalog(
            original.clone(),
            &server.uri(),
            Some(&SecretString::from("fixture")),
            &client,
            CancellationToken::new()
        )
        .await
        .is_err()
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(matches!(
        personalize_openrouter_catalog(
            original,
            &server.uri(),
            Some(&SecretString::from("fixture")),
            &client,
            cancel
        )
        .await,
        Err(opi_ai::provider::ProviderError::Cancelled)
    ));
}

#[tokio::test]
async fn personalization_preserves_unknown_prices_and_true_free_prices() {
    let server = MockServer::start().await;
    let model = records()
        .into_iter()
        .find(|model| model.operation == CatalogOperation::Chat)
        .unwrap();
    for (pricing, known) in [
        (json!({"prompt": "-1", "completion": "-1"}), false),
        (json!({}), false),
        (
            json!({"prompt": "0", "completion": "0", "input_cache_read": "0", "input_cache_write": "0"}),
            true,
        ),
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data": [{"id": model.id, "pricing": pricing}]})),
            )
            .expect(1)
            .mount(&server)
            .await;
        let result = personalize_openrouter_catalog(
            vec![model.clone()],
            &server.uri(),
            Some(&SecretString::from("fixture")),
            &HttpClient::new(),
            CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(result[0].cost.is_some(), known);
        assert_eq!(
            result[0].to_chat_model_info().unwrap().pricing.is_some(),
            known
        );
        if known {
            assert_eq!(result[0].cost.unwrap().input, 0.0);
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn personalized_record_drives_the_next_real_models_request() {
    use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
    use opi_ai::provider::Request;
    use opi_ai::{ChatModelRecord, Models, WireApi};
    use std::sync::Arc;

    let server = MockServer::start().await;
    let original = records()
        .into_iter()
        .find(|model| {
            model.operation == CatalogOperation::Chat && model.api == "openai-completions"
        })
        .unwrap();
    Mock::given(method("GET")).and(path("/models/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{
            "id": original.id, "top_provider": {"context_length": 80000, "max_completion_tokens": 1234},
            "pricing": {"prompt": "0.000002", "completion": "0.000003", "input_cache_read": "0", "input_cache_write": "0"}
        }]}))).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/chat/completions"))
        .and(header("authorization", "Bearer personalized-account"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream")
            .set_body_string("data: {\"id\":\"personalized\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ready\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":1,\"total_tokens\":3}}\n\ndata: [DONE]\n\n"))
        .expect(1).mount(&server).await;
    let client = Arc::new(HttpClient::new());
    let personalized = personalize_openrouter_catalog(
        vec![original],
        &server.uri(),
        Some(&SecretString::from("personalized-account")),
        &client,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let mut info = personalized[0].to_chat_model_info().unwrap();
    assert_eq!(info.wire_api, WireApi::OpenAiCompletions);
    info.base_url = Some(server.uri());
    let mut models = Models::new();
    models
        .register_route(
            Box::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
                Some(server.uri()),
                "openrouter".into(),
                Default::default(),
                vec![info.clone()],
                client,
            )),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("personalized-account"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let request = Request {
        transport: Default::default(),
        model: "ignored".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let record = ChatModelRecord::new("openrouter", info.clone());
    let output = models
        .complete_simple(record, request, Default::default())
        .await
        .unwrap();
    assert_eq!(output.model, info.id);
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(body["model"], info.id);
    let limit = body
        .get("max_tokens")
        .or_else(|| body.get("max_completion_tokens"));
    assert_eq!(limit, Some(&json!(1234)));
    assert_eq!(info.pricing.unwrap().base.input_cost_per_mtok, 2.0);
}
