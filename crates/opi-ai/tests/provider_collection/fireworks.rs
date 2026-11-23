//! Fireworks' fixed Pi 1.5 catalog and dual-wire dispatch.

use super::{
    Arc, BuiltinOverrides, Catalog, CatalogOperation, Models, WireApi, map_context,
    register_builtin,
};
use opi_ai::catalog::CatalogRecord;
use opi_ai::models::{ChatModelRecord, ModelsError};
use opi_ai::provider::ThinkingConfig;
use opi_ai::registry::RegistryError;
use opi_ai::{ThinkingLevel, WireCompat};
use serde_json::Value;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

const PROVIDER: &str = "fireworks";
const FIXTURE_KEY: &str = "fixture-fireworks-key";
const PUBLISHED_IDS: [&str; 29] = [
    "accounts/fireworks/models/deepseek-v4p1-flash",
    "accounts/fireworks/models/deepseek-v4p1-flash-1017",
    "accounts/fireworks/models/ember-1",
    "accounts/fireworks/models/glm-5p3",
    "accounts/fireworks/models/glm-5p3-flash",
    "accounts/fireworks/models/gpt-oss-120b",
    "accounts/fireworks/models/inkling",
    "accounts/fireworks/models/kimi-k3",
    "accounts/fireworks/models/minimax-m3",
    "accounts/fireworks/models/minimax-m3p1-flash",
    "accounts/fireworks/models/muse-spark-open",
    "accounts/fireworks/models/nemotron-3-ultra-nvfp4",
    "accounts/fireworks/models/nemotron-lightning-3p5-30b-a3b",
    "accounts/fireworks/models/qwen3p8-2p4t-a95b",
    "accounts/fireworks/models/qwen3p8-max",
    "accounts/fireworks/routers/deepseek-flash-latest",
    "accounts/fireworks/routers/deepseek-v4p1-flash-1017-spot",
    "accounts/fireworks/routers/deepseek-v4p1-flash-spot",
    "accounts/fireworks/routers/glm-5p3-fast",
    "accounts/fireworks/routers/glm-5p3-spot",
    "accounts/fireworks/routers/glm-fast-latest",
    "accounts/fireworks/routers/glm-flash-latest",
    "accounts/fireworks/routers/glm-latest",
    "accounts/fireworks/routers/kimi-fast-latest",
    "accounts/fireworks/routers/kimi-k3-fast",
    "accounts/fireworks/routers/kimi-k3-spot",
    "accounts/fireworks/routers/kimi-latest",
    "accounts/fireworks/routers/minimax-latest",
    "accounts/fireworks/routers/qwen-max-latest",
];

fn builtin_models(overrides: &BuiltinOverrides) -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("FIREWORKS_API_KEY", FIXTURE_KEY)]),
        Arc::new(opi_ai::http::HttpClient::new()),
        overrides,
    )
    .unwrap();
    models
}

fn loopback_base(server: &MockServer, published_base: &str) -> String {
    let published = url::Url::parse(published_base).unwrap();
    assert_eq!(published.host_str(), Some("api.fireworks.ai"));
    format!("{}{}", server.uri(), published.path().trim_end_matches('/'))
}

#[tokio::test]
async fn builtin_fireworks_lists_the_published_dual_wire_records() {
    let catalog = Catalog::builtin().unwrap();
    let models = builtin_models(&BuiltinOverrides::default());
    let raw_bundle: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    let author_source: Value = serde_json::from_slice(include_bytes!(
        "../../resources/catalog/providers/fireworks.json"
    ))
    .unwrap();
    assert_eq!(raw_bundle[PROVIDER], author_source);

    let entries: Vec<_> = catalog
        .models(PROVIDER, CatalogOperation::Chat)
        .unwrap()
        .collect();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        PUBLISHED_IDS
    );
    let expected: Vec<_> = entries
        .iter()
        .map(|entry| {
            assert!(matches!(
                entry.api.as_str(),
                "anthropic-messages" | "openai-completions"
            ));
            let info = entry.to_chat_model_info().unwrap();
            assert_eq!(models.get_model(PROVIDER, &entry.id).as_ref(), Some(&info));
            CatalogRecord::from(info)
        })
        .collect();
    assert_eq!(
        models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER)),
        expected
    );
    assert_eq!(
        models
            .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
            .await
            .unwrap(),
        expected
    );
    for id in PUBLISHED_IDS {
        assert_eq!(
            models
                .get_model_of_type(CatalogOperation::Chat, PROVIDER, id)
                .as_ref(),
            Some(&CatalogRecord::Chat(Box::new(
                models.get_model(PROVIDER, id).unwrap(),
            )))
        );
    }

    let qwen = models
        .get_model(PROVIDER, "accounts/fireworks/routers/qwen-max-latest")
        .unwrap();
    assert_eq!(qwen.wire_api, WireApi::AnthropicMessages);
    assert_eq!(
        qwen.thinking_level_map.resolve(ThinkingLevel::XHigh),
        Ok(Some("xhigh".to_owned()))
    );
    match &qwen.compat {
        WireCompat::AnthropicMessages(compat) => {
            assert!(compat.force_adaptive_thinking);
            assert_eq!(compat.send_session_affinity_headers, Some(true));
        }
        other => panic!("unexpected Qwen Max compatibility {other:?}"),
    }
}

#[tokio::test]
async fn builtin_fireworks_sends_anthropic_and_openai_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/inference/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::AnthropicMessages)),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/inference/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(super::node5_reply(WireApi::OpenAiCompletions)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let models = builtin_models(&BuiltinOverrides::default());
    let qwen_id = "accounts/fireworks/routers/qwen-max-latest";
    let mut qwen = models.get_model(PROVIDER, qwen_id).unwrap();
    qwen.base_url = Some(loopback_base(&server, qwen.base_url.as_deref().unwrap()));
    let qwen_spec = format!("{PROVIDER}:{qwen_id}");
    let mut qwen_request = super::minimal_request(&qwen_spec);
    qwen_request.max_tokens = Some(1_024);
    qwen_request.session_id = Some("fireworks-session".into());
    qwen_request.thinking = ThinkingConfig {
        enabled: true,
        level: ThinkingLevel::XHigh,
        budget_tokens: None,
    };
    let qwen_output = models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, qwen),
            qwen_request,
            Default::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("{qwen_id}: {error:?}"));
    assert_eq!(qwen_output.provider, PROVIDER);
    assert_eq!(qwen_output.model, qwen_id);
    assert_eq!(super::assistant_text(&qwen_output), "hi");

    let openai_id = "accounts/fireworks/routers/kimi-k3-spot";
    let mut openai = models.get_model(PROVIDER, openai_id).unwrap();
    openai.base_url = Some(loopback_base(&server, openai.base_url.as_deref().unwrap()));
    let openai_spec = format!("{PROVIDER}:{openai_id}");
    let mut openai_request = super::minimal_request(&openai_spec);
    openai_request.max_tokens = Some(1_024);
    openai_request.session_id = Some("fireworks-session".into());
    let openai_output = models
        .complete_simple(
            ChatModelRecord::new(PROVIDER, openai),
            openai_request,
            Default::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("{openai_id}: {error:?}"));
    assert_eq!(openai_output.provider, PROVIDER);
    assert_eq!(openai_output.model, openai_id);
    assert_eq!(super::assistant_text(&openai_output), "hi");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2);
    let anthropic = sent
        .iter()
        .find(|request| request.url.path() == "/inference/v1/messages")
        .unwrap();
    assert_eq!(anthropic.headers["x-api-key"], FIXTURE_KEY);
    assert!(!anthropic.headers.contains_key("authorization"));
    assert_eq!(anthropic.headers["x-session-affinity"], "fireworks-session");
    let anthropic_body: Value = serde_json::from_slice(&anthropic.body).unwrap();
    assert_eq!(anthropic_body["model"], qwen_id);
    assert_eq!(anthropic_body["thinking"]["type"], "adaptive");
    assert_eq!(anthropic_body["output_config"]["effort"], "xhigh");

    let openai = sent
        .iter()
        .find(|request| request.url.path() == "/inference/v1/chat/completions")
        .unwrap();
    assert_eq!(
        openai.headers["authorization"],
        format!("Bearer {FIXTURE_KEY}")
    );
    for name in ["session_id", "x-client-request-id", "x-session-affinity"] {
        assert_eq!(openai.headers[name], "fireworks-session");
    }
    let openai_body: Value = serde_json::from_slice(&openai.body).unwrap();
    assert_eq!(openai_body["model"], openai_id);
    assert_eq!(openai_body["stream"], true);
    assert_eq!(openai_body["stream_options"]["include_usage"], true);
}

#[tokio::test]
async fn builtin_fireworks_unknown_models_are_rejected_before_dispatch() {
    let server = MockServer::start().await;
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert(PROVIDER.into(), server.uri());
    let models = builtin_models(&overrides);
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some(PROVIDER));
    let available = models
        .get_available_of_type(CatalogOperation::Chat, Some(PROVIDER))
        .await
        .unwrap();
    assert_eq!(listed.len(), PUBLISHED_IDS.len());
    assert_eq!(available.len(), PUBLISHED_IDS.len());
    assert!(
        catalog
            .model(
                PROVIDER,
                CatalogOperation::Chat,
                "accounts/fireworks/models/retired"
            )
            .is_none()
    );
    assert!(
        listed
            .iter()
            .all(|record| record.id() != "accounts/fireworks/models/retired")
    );
    assert!(
        available
            .iter()
            .all(|record| record.id() != "accounts/fireworks/models/retired")
    );
    assert!(
        models
            .get_model(PROVIDER, "accounts/fireworks/models/retired")
            .is_none()
    );

    let spec = format!("{PROVIDER}:accounts/fireworks/models/retired");
    assert!(matches!(
        models.prepare_call(&spec, super::minimal_request(&spec)).await,
        Err(ModelsError::Registry(RegistryError::UnknownModel { provider, model }))
            if provider == PROVIDER && model == "accounts/fireworks/models/retired"
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
