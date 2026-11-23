//! Ten Chat routes exercised through one Models instance with local protocol peers.

use std::sync::Arc;

use opi_ai::api::bedrock::event_stream;
use opi_ai::api::contract::SimpleOptions;
use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
};
use opi_ai::auth::aws::local::AwsAuthSelection;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::http::HttpClient;
use opi_ai::models::{ChatModelRecord, Models};
use opi_ai::providers::collection::{BuiltinOverrides, register_builtin};
use opi_ai::{AuthProvenanceSource, AwsSigV4Credentials, WireApi};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::{
    OwnedRecordingContext, anthropic_text_sse, message_text, openai_completions_sse, sse_template,
    user_request,
};

fn openai_responses_sse() -> String {
    concat!(
        "event: response.created\n",
        r#"data: {"type":"response.created","response":{"id":"resp_1","status":"in_progress","model":"gpt-4","output":[]}}"#, "\n\n",
        "event: response.output_item.added\n",
        r#"data: {"type":"response.output_item.added","output_index":0,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}"#, "\n\n",
        "event: response.content_part.added\n",
        r#"data: {"type":"response.content_part.added","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#, "\n\n",
        "event: response.output_text.delta\n",
        r#"data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"Hi there"}"#, "\n\n",
        "event: response.output_text.done\n",
        r#"data: {"type":"response.output_text.done","output_index":0,"content_index":0,"text":"Hi there"}"#, "\n\n",
        "event: response.output_item.done\n",
        r#"data: {"type":"response.output_item.done","output_index":0,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"Hi there"}]}}"#, "\n\n",
        "event: response.completed\n",
        r#"data: {"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-4","output":[{"type":"message","content":[{"type":"output_text","text":"Hi there"}]}],"usage":{"input_tokens":10,"output_tokens":5}}}"#, "\n\n",
    )
    .into()
}

fn codex_completed_sse() -> String {
    concat!(
        "event: response.completed\n",
        r#"data: {"type":"response.completed","response":{"id":"response-1","model":"gpt-5.3-codex-spark","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
        "\n\n",
    )
    .into()
}

fn google_candidates_sse() -> String {
    concat!(
        r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}]}"#,
        "\n\n",
        r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":42,"candidatesTokenCount":13,"totalTokenCount":55}}"#,
        "\n\n",
    )
    .into()
}

fn mistral_sse() -> String {
    concat!(
        r#"data: {"choices":[{"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"content":"Hi there"},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
        "\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

fn pi_messages_sse() -> String {
    concat!(
        r#"data: {"type":"start"}"#, "\n\n",
        r#"data: {"type":"text_start","contentIndex":0}"#, "\n\n",
        r#"data: {"type":"text_delta","contentIndex":0,"delta":"Hel"}"#, "\n\n",
        r#"data: {"type":"text_end","contentIndex":0,"content":"Hello"}"#, "\n\n",
        r#"data: {"type":"done","reason":"stop","usage":{"input":10,"output":5,"cacheRead":0,"cacheWrite":0,"totalTokens":15,"cost":{"input":0.1,"output":0.2,"cacheRead":0,"cacheWrite":0,"total":0.3},"responseId":"resp-1"}}"#,
        "\n\n",
    )
    .into()
}

fn bedrock_converse_stream_body() -> Vec<u8> {
    let events: &[(&str, &str)] = &[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello bedrock"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ];
    let mut body = Vec::new();
    for (event_type, json_payload) in events {
        body.extend_from_slice(&event_stream::build_test_frame(
            event_type,
            "application/json",
            json_payload.as_bytes(),
        ));
    }
    let trailer = event_stream::build_test_frame("metadata", "application/json", b"{}");
    body.extend_from_slice(&trailer);
    body
}

fn anthropic_route_info(model: &str) -> opi_ai::model_info::ModelInfo {
    opi_ai::model_info::ModelInfo::new(
        model,
        "Matrix Anthropic Model",
        WireApi::AnthropicMessages,
        opi_ai::registry::ModelCapabilities::new(200_000, 8_192),
    )
}

struct StaticBearer;

impl AuthResolver for StaticBearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, opi_ai::provider::ProviderError>>
    {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("fake-key"),
                base_url: None,
                account_id: Some("fake-account".into()),
                provenance: Default::default(),
            })
        })
    }
}

#[tokio::test]
async fn assembled_matrix_drives_all_ten_published_chat_wires_through_one_instance() {
    let anthropic_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sse_template(anthropic_text_sse()))
        .expect(1)
        .mount(&anthropic_peer)
        .await;

    let codex_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(codex_completed_sse()))
        .expect(1)
        .mount(&codex_peer)
        .await;

    let openai_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?responses$"))
        .respond_with(sse_template(openai_responses_sse()))
        .expect(1)
        .mount(&openai_peer)
        .await;

    let openrouter_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(api/)?(v1/)?chat/completions$",
        ))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&openrouter_peer)
        .await;

    let azure_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(openai/deployments/[^/]+/)?(v1/)?responses$",
        ))
        .respond_with(sse_template(openai_responses_sse()))
        .expect(1)
        .mount(&azure_peer)
        .await;

    let google_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            r"^/(v1beta/)?models/gemini-2\.5-flash:streamGenerateContent$",
        ))
        .respond_with(sse_template(google_candidates_sse()))
        .expect(1)
        .mount(&google_peer)
        .await;

    let vertex_token_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "vertex-token", "expires_in": 3600, "token_type": "Bearer"
        })))
        .expect(1..=2)
        .mount(&vertex_token_peer)
        .await;
    let vertex_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?projects/"))
        .respond_with(sse_template(google_candidates_sse()))
        .expect(1)
        .mount(&vertex_peer)
        .await;

    let mistral_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?chat/completions$"))
        .respond_with(sse_template(mistral_sse()))
        .expect(1)
        .mount(&mistral_peer)
        .await;

    let radius_peer = MockServer::start().await;
    let radius_model = Catalog::builtin()
        .unwrap()
        .model("radius", CatalogOperation::Chat, "balanced")
        .unwrap();
    Mock::given(method("GET"))
        .and(path("/v1/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "baseUrl": format!("{}/v1", radius_peer.uri()),
            "models": [radius_model.raw],
        })))
        .expect(1)
        .mount(&radius_peer)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sse_template(pi_messages_sse()))
        .expect(1)
        .mount(&radius_peer)
        .await;

    let bedrock_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/model/.+/converse-stream$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            bedrock_converse_stream_body(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(1)
        .mount(&bedrock_peer)
        .await;

    // Scoped ADC file keeps vertex construction offline and isolated.
    let adc_dir = tempfile::tempdir().unwrap();
    let adc_file = adc_dir.path().join("adc.json");
    std::fs::write(
        &adc_file,
        serde_json::to_vec(&serde_json::json!({
            "type":"authorized_user", "client_id":"client-x",
            "client_secret":"secret-x", "refresh_token":"refresh-x",
            "quota_project_id":"quota-x"
        }))
        .unwrap(),
    )
    .unwrap();

    let context = OwnedRecordingContext::new(&[
        ("OPENAI_API_KEY", "sk-openai"),
        ("OPENROUTER_API_KEY", "sk-openrouter"),
        ("AZURE_OPENAI_API_KEY", "sk-azure"),
        ("GEMINI_API_KEY", "sk-google"),
        ("MISTRAL_API_KEY", "sk-mistral"),
        ("RADIUS_API_KEY", "sk-radius"),
        ("GOOGLE_APPLICATION_CREDENTIALS", adc_file.to_str().unwrap()),
        ("GOOGLE_CLOUD_PROJECT", "assembled-project"),
        ("GOOGLE_CLOUD_LOCATION", "assembled-location"),
    ]);

    let client = Arc::new(HttpClient::new());
    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("openai".into(), openai_peer.uri());
    overrides
        .base_urls
        .insert("openrouter".into(), openrouter_peer.uri());
    overrides
        .base_urls
        .insert("google".into(), google_peer.uri());
    overrides
        .base_urls
        .insert("mistral".into(), mistral_peer.uri());
    overrides
        .base_urls
        .insert("radius".into(), radius_peer.uri());
    overrides.azure_endpoint = Some(azure_peer.uri());
    overrides.azure_api_version = Some("2024-06-01".into());
    overrides.aws_auth.base_url = Some(bedrock_peer.uri());
    overrides.aws_auth.selection = Some(AwsAuthSelection::SigV4(AwsSigV4Credentials::new(
        SecretString::from("AKIDASSEMBLED"),
        SecretString::from("secret"),
        "us-east-1",
    )));
    overrides.vertex_auth.base_url = Some(vertex_peer.uri());
    overrides.vertex_auth.adc = Some(
        AdcNativeOwner::new(
            AdcAuthSelection::Auto,
            Arc::new(ReqwestAdcTransport::new()),
            Arc::new(SystemAdcClock),
        )
        .with_token_endpoint(Some(format!("{}/token", vertex_token_peer.uri()))),
    );

    let report = register_builtin(&mut models, context.clone(), client.clone(), &overrides)
        .expect("builtin registration");
    assert_eq!(
        report.registered.len() + report.skipped.len(),
        42,
        "all 42 published provider profiles are accounted for on the assembled instance: {:?}",
        report.skipped
    );
    assert!(
        report
            .skipped
            .iter()
            .all(|(_, reason)| reason.contains("classifier-only")),
        "only classifier-only profiles may skip chat registration: {:?}",
        report.skipped
    );
    assert!(!report.classifier_apis.is_empty());
    assert!(!report.image_apis.is_empty());
    let refresh = models
        .refresh(opi_ai::ModelsRefreshOptions {
            providers: Some(["radius".to_owned()].into_iter().collect()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        refresh.find("radius").unwrap().outcome,
        opi_ai::RefreshOutcome::Published { models: 1 }
    );
    // The two OAuth-only chat routes join the same instance through the
    // public register_route seam with real adapters and static auth; their
    // records are caller-owned complete records (AIR-045), independent of
    // catalog visibility.
    let anthropic_info = anthropic_route_info("claude-sonnet-4-5-20250514");
    models
        .register_route(
            Box::new(opi_ai::providers::anthropic::AnthropicProvider::for_route(
                "anthropic".into(),
                vec![anthropic_info.clone()],
                Some(anthropic_peer.uri()),
                Default::default(),
                client.clone(),
                false,
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-anthropic"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let codex_info = opi_ai::model_info::ModelInfo::new(
        "gpt-5.3-codex-spark",
        "Codex Spark",
        WireApi::OpenAiCodexResponses,
        opi_ai::registry::ModelCapabilities::new(200_000, 32_768),
    );
    models
        .register_route(
            Box::new(
                opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider::new(
                    Some(codex_peer.uri()),
                    vec![codex_info.clone()],
                    client.clone(),
                ),
            ),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    // All ten published chat wires complete through the same instance with
    // complete catalog records and the neutral entry.
    let matrix: [(&str, &str, &str); 10] = [
        ("anthropic", "claude-sonnet-4-5-20250514", "Hello world"),
        ("openai", "gpt-4", "Hi there"),
        ("openrouter", "openai/gpt-4o", "Hello world"),
        ("azure", "gpt-4", "Hi there"),
        ("google", "gemini-2.5-flash", "Hello"),
        ("google-vertex", "gemini-2.5-flash", "Hello"),
        ("mistral", "codestral-2508", "Hi there"),
        ("openai-codex", "gpt-5.3-codex-spark", ""),
        ("radius", "balanced", "Hello"),
        ("amazon-bedrock", "amazon.nova-2-lite-v1:0", "Hello bedrock"),
    ];
    for (provider, model, expected) in matrix {
        let info = models
            .get_model(provider, model)
            .unwrap_or_else(|| panic!("catalog model for {provider}/{model}"));
        let outcome = models
            .complete_simple(
                ChatModelRecord::new(provider, info),
                user_request(model, "Hello"),
                SimpleOptions::default(),
            )
            .await;
        let message = match outcome {
            Ok(message) => message,
            Err(failure) => {
                let hits = [
                    ("anthropic", &anthropic_peer),
                    ("codex", &codex_peer),
                    ("openai", &openai_peer),
                    ("openrouter", &openrouter_peer),
                    ("azure", &azure_peer),
                    ("google", &google_peer),
                    ("vertex", &vertex_peer),
                    ("mistral", &mistral_peer),
                    ("radius", &radius_peer),
                    ("bedrock", &bedrock_peer),
                ]
                .into_iter()
                .map(|(name, server)| async move {
                    let paths = server
                        .received_requests()
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|r| r.url.path().to_owned())
                        .collect::<Vec<_>>();
                    format!("{name}: {paths:?}")
                })
                .collect::<Vec<_>>();
                let all = futures_util::future::join_all(hits).await;
                panic!("{provider} wire failed: {failure:?}; peers saw {all:?}");
            }
        };
        assert_eq!(
            message_text(&message),
            expected,
            "final text for {provider} wire"
        );
    }

    anthropic_peer.verify().await;
    codex_peer.verify().await;
    openai_peer.verify().await;
    openrouter_peer.verify().await;
    azure_peer.verify().await;
    google_peer.verify().await;
    vertex_peer.verify().await;
    mistral_peer.verify().await;
    radius_peer.verify().await;
    bedrock_peer.verify().await;
}
