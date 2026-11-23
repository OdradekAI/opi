//! Built-in chat routing and request projection across wire APIs.

use super::*;

#[tokio::test]
async fn real_public_call_through_a_groq_completions_builtin_profile() {
    use wiremock::matchers;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("GROQ_API_KEY", "gsk-groq")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    // groq's first catalog model is a plain non-thinking completions model.
    let model = first_catalog_chat_model("groq");
    let spec = format!("groq:{model}");
    let prepared = models
        .prepare_call(&spec, minimal_request(&spec))
        .await
        .expect("canonical builtin dispatches");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("stream completes");
    assert_eq!(assistant_text(&message), "hi");

    let requests = server.received_requests().await.expect("captured");
    let request = &requests[0];
    assert!(
        request.url.path().ends_with("chat/completions"),
        "openai-completions wire, got {}",
        request.url.path()
    );
    assert_eq!(
        request.headers.get("authorization").unwrap(),
        "Bearer gsk-groq"
    );
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body["model"], model, "the wire model is the catalog id");
}

#[tokio::test]
async fn real_public_call_through_openai_defaults_to_responses_wire() {
    use wiremock::matchers;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let sse = concat!(
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}\n\n",
        "data: [DONE]\n\n",
    );
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse)
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("openai".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("OPENAI_API_KEY", "sk-openai")]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();

    let model = first_catalog_chat_model("openai");
    let spec = format!("openai:{model}");
    let prepared = models
        .prepare_call(&spec, minimal_request(&spec))
        .await
        .expect("openai builtin dispatches");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("stream completes");
    assert_eq!(assistant_text(&message), "hi");

    let requests = server.received_requests().await.expect("captured");
    let request = &requests[0];
    assert!(
        request.url.path().ends_with("/responses"),
        "openai defaults to the Responses wire, got {}",
        request.url.path()
    );
    assert_eq!(
        request.headers.get("authorization").unwrap(),
        "Bearer sk-openai"
    );
}

#[tokio::test]
async fn builtin_mistral_projects_catalog_effort_on_the_public_simple_route() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(WireApi::MistralConversations)),
        )
        .expect(2)
        .mount(&server)
        .await;
    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("mistral".into(), server.uri());
    register_builtin(
        &mut models,
        map_context(&[("MISTRAL_API_KEY", "test-key")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &overrides,
    )
    .unwrap();
    assert!(models.get_model("mistral", "mistral-nemo").is_none());
    let model = models.get_model("mistral", "mistral-large-4").unwrap();
    let mut request = minimal_request(&model.id);
    request.thinking.enabled = true;
    request.thinking.level = opi_ai::ThinkingLevel::Low;
    let result = models
        .complete_simple(
            opi_ai::ChatModelRecord::new("mistral", model),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&result), "hi");
    let added = models.get_model("mistral", "mistral-large-4-0-8").unwrap();
    let mut request = minimal_request(&added.id);
    request.thinking.enabled = false;
    models
        .complete_simple(
            opi_ai::ChatModelRecord::new("mistral", added),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
    assert_eq!(body["model"], "mistral-large-4");
    assert_eq!(body["reasoning_effort"], "low");
    assert!(body.get("prompt_mode").is_none());
    let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    assert_eq!(body["model"], "mistral-large-4-0-8");
    assert!(body.get("reasoning_effort").is_none());
    assert!(body.get("prompt_mode").is_none());
}

#[tokio::test]
async fn builtin_mapped_calls_use_owned_custom_records_and_header_precedence() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire) in [
        ("groq", "GROQ_API_KEY", WireApi::OpenAiCompletions),
        ("openai", "OPENAI_API_KEY", WireApi::OpenAiResponses),
        ("anthropic", "ANTHROPIC_API_KEY", WireApi::AnthropicMessages),
        ("azure", "AZURE_OPENAI_API_KEY", WireApi::OpenAiCompletions),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
        ),
    ] {
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
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let mut info = ModelInfo::new(
            "custom-owned",
            "Custom owned",
            wire,
            ModelCapabilities::new(8192, 129),
        )
        .with_base_url(format!("{}/owned", server.uri()));
        info.headers = vec![
            ("x-model".into(), Some("record".into())),
            ("x-override".into(), Some("record".into())),
            ("x-absent".into(), None),
        ];
        info.sampling_params
            .get_or_insert_default()
            .insert("top_p".into(), serde_json::json!(0.34));
        let mut request = minimal_request("ignored-request-model");
        request.extra_headers = vec![("x-override".into(), "request".into())];
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new(provider, info.clone()),
                request,
                Default::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("{provider}/{wire}: {error:?}"));
        assert_eq!(assistant_text(&output), "hi");
        assert_eq!(output.provider, provider);
        assert_eq!(output.model, "custom-owned");
        let sent = server.received_requests().await.unwrap();
        let sent = &sent[0];
        assert!(
            sent.url.path().starts_with("/owned/"),
            "{provider}: {}",
            sent.url
        );
        assert_eq!(sent.headers["x-model"], "record");
        assert_eq!(sent.headers["x-override"], "request");
        assert!(!sent.headers.contains_key("x-absent"));
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(body["model"], "custom-owned");
        if wire == WireApi::AnthropicMessages {
            assert!(body.get("top_p").is_none());
        } else {
            assert_eq!(body["top_p"], serde_json::json!(0.34), "{provider}/{wire}");
        }
        info.headers
            .push(("authorization".into(), Some("forbidden".into())));
        assert!(
            models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(provider, info),
                    minimal_request("ignored"),
                    Default::default()
                )
                .await
                .is_err()
        );
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn builtin_gateway_all_three_wires_use_only_source_auth_header() {
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for wire in [
        WireApi::OpenAiCompletions,
        WireApi::OpenAiResponses,
        WireApi::AnthropicMessages,
    ] {
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
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[
                ("CLOUDFLARE_API_KEY", "gateway-fixture"),
                ("CLOUDFLARE_ACCOUNT_ID", "account"),
                ("CLOUDFLARE_GATEWAY_ID", "gateway"),
            ]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let info = ModelInfo::new(
            "custom-gateway",
            "Gateway",
            wire,
            ModelCapabilities::new(8192, 128),
        )
        .with_base_url(format!("{}/selected", server.uri()));
        let output = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("cloudflare-ai-gateway", info),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(assistant_text(&output), "hi");
        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent[0].headers["cf-aig-authorization"],
            "Bearer gateway-fixture"
        );
        assert!(!sent[0].headers.contains_key("authorization"));
        assert!(!sent[0].headers.contains_key("x-api-key"));
        assert!(sent[0].url.path().starts_with("/selected/"));
    }
}

#[tokio::test]
async fn builtin_source_profiles_reach_their_registered_wires() {
    use std::collections::BTreeSet;
    use std::sync::Arc;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };
    let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
    let mut checked = BTreeSet::new();
    for profile in opi_ai::providers::builtin_provider_profiles() {
        // Native crypto/ADC transports and classifier-only operation have their
        // own real builtin fixtures; this corpus covers every other profile/wire.
        if ["amazon-bedrock", "google", "google-vertex", "typesafe"].contains(&profile.id) {
            continue;
        }
        let mut wires = BTreeSet::new();
        for entry in catalog
            .models(profile.id, opi_ai::catalog::CatalogOperation::Chat)
            .unwrap()
        {
            if !wires.insert(entry.api.clone()) {
                continue;
            }
            let server = MockServer::start().await;
            let source_path = if profile.id == "azure" {
                "/openai/v1".to_owned()
            } else {
                url::Url::parse(&entry.base_url)
                    .unwrap()
                    .path()
                    .trim_end_matches('/')
                    .to_owned()
            };
            let controlled_base = format!("{}{source_path}", server.uri());
            let mut overrides = BuiltinOverrides::default();
            overrides
                .base_urls
                .insert(profile.id.into(), controlled_base.clone());
            if profile.id == "azure" {
                overrides.azure_endpoint = Some(controlled_base.clone());
            }
            let credential = if profile.id == "openai-codex" {
                Credential::OAuthToken {
                    access: "fixture-codex".into(),
                    refresh: "unused".into(),
                    expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
                    base_url: None,
                    account_id: Some("fixture-account".into()),
                    client_id: None,
                    enterprise_domain: None,
                    available_model_ids: None,
                    scopes: None,
                }
            } else {
                Credential::ApiKey("fixture-source-key".into())
            };
            let mut models = Models::new();
            models.set_credential_store(Arc::new(FakeStore {
                entries: Mutex::new(HashMap::from([(profile.id.into(), credential)])),
            }));
            register_builtin(
                &mut models,
                map_context(&[
                    ("CLOUDFLARE_ACCOUNT_ID", "fixture-account"),
                    ("CLOUDFLARE_GATEWAY_ID", "fixture-gateway"),
                ]),
                Arc::new(opi_ai::http::HttpClient::new()),
                &overrides,
            )
            .unwrap();
            if profile.id == "radius" {
                Mock::given(method("GET"))
                    .and(path("/v1/config"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "baseUrl": controlled_base,
                        "models": [entry.raw],
                    })))
                    .expect(1)
                    .mount(&server)
                    .await;
                let refresh = models
                    .refresh(opi_ai::ModelsRefreshOptions {
                        providers: Some(BTreeSet::from(["radius".to_owned()])),
                        ..Default::default()
                    })
                    .await
                    .unwrap();
                assert_eq!(
                    refresh.find("radius").unwrap().outcome,
                    opi_ai::RefreshOutcome::Published { models: 1 }
                );
            }
            let info = models.get_model(profile.id, &entry.id).unwrap();
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(node5_reply(info.wire_api)),
                )
                .expect(1)
                .mount(&server)
                .await;
            let mut request = minimal_request("owned model selects routing");
            request.transport = opi_ai::provider::TransportPreference::Sse;
            request.max_tokens = Some(32);
            request.session_id = Some("profile-session".into());
            request
                .extra_headers
                .push(("x-call-proof".into(), "profile-wire".into()));
            let output = models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(profile.id, info.clone()),
                    request,
                    Default::default(),
                )
                .await
                .unwrap_or_else(|error| panic!("{}/{}: {error:?}", profile.id, entry.api));
            assert_eq!(output.provider, profile.id, "{}/{}", profile.id, entry.api);
            assert_eq!(output.model, entry.id);
            assert_eq!(assistant_text(&output), "hi");
            let sent = server.received_requests().await.unwrap();
            let sent = if profile.id == "radius" {
                assert_eq!(sent[0].url.path(), "/v1/config");
                &sent[1..]
            } else {
                &sent[..]
            };
            assert_eq!(sent.len(), 1, "{}/{}", profile.id, entry.api);
            let suffix = match info.wire_api {
                WireApi::OpenAiCompletions => Some("/chat/completions"),
                WireApi::MistralConversations => Some("/v1/chat/completions"),
                WireApi::OpenAiResponses | WireApi::AzureOpenAiResponses => Some("/responses"),
                WireApi::OpenAiCodexResponses => Some("/codex/responses"),
                WireApi::AnthropicMessages => Some("/v1/messages"),
                WireApi::PiMessages => Some("/messages"),
                _ => None,
            };
            if let Some(suffix) = suffix {
                assert_eq!(
                    sent[0].url.path(),
                    format!("{source_path}{suffix}"),
                    "{}/{}",
                    profile.id,
                    entry.api
                );
            }

            assert_eq!(sent[0].headers["x-call-proof"], "profile-wire");
            let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
            if info.wire_api == WireApi::GoogleGenerativeAi {
                assert!(sent[0].url.path().contains(&entry.id));
            } else {
                assert_eq!(body["model"], entry.id);
            }
            if ["opencode", "opencode-go"].contains(&profile.id) {
                assert_eq!(sent[0].headers["x-opencode-session"], "profile-session");
            }
            let auth_header = if profile.id == "cloudflare-ai-gateway" {
                "cf-aig-authorization"
            } else if info.wire_api == WireApi::AzureOpenAiResponses {
                "api-key"
            } else if info.wire_api == WireApi::GoogleGenerativeAi {
                "x-goog-api-key"
            } else if info.wire_api == WireApi::AnthropicMessages && profile.id != "github-copilot"
            {
                "x-api-key"
            } else {
                "authorization"
            };
            assert!(
                sent[0].headers.contains_key(auth_header),
                "{}/{} lacks {auth_header}",
                profile.id,
                entry.api
            );
            checked.insert(profile.id);
        }
    }
    assert_eq!(checked.len(), 38);
}

#[tokio::test]
async fn builtin_simple_tool_choice_reaches_each_chat_wire() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire) in [
        ("groq", "GROQ_API_KEY", WireApi::OpenAiCompletions),
        ("openai", "OPENAI_API_KEY", WireApi::OpenAiResponses),
        ("anthropic", "ANTHROPIC_API_KEY", WireApi::AnthropicMessages),
        ("azure", "AZURE_OPENAI_API_KEY", WireApi::OpenAiCompletions),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(node5_reply(wire)),
            )
            .expect(2)
            .mount(&server)
            .await;
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let record = opi_ai::ChatModelRecord::new(
            provider,
            ModelInfo::new(
                "choice-fixture",
                "Choice fixture",
                wire,
                ModelCapabilities::new(8192, 64),
            )
            .with_base_url(server.uri()),
        );
        for choice in [
            opi_ai::SimpleToolChoice::Auto,
            opi_ai::SimpleToolChoice::None,
        ] {
            models
                .complete_simple(
                    record.clone(),
                    minimal_request("ignored"),
                    opi_ai::SimpleOptions {
                        tool_choice: Some(choice),
                        ..Default::default()
                    },
                )
                .await
                .unwrap_or_else(|error| panic!("{provider}/{wire}/{choice:?}: {error:?}"));
        }
        for (request, expected) in server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .zip(["auto", "none"])
        {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            let choice = if wire == WireApi::AnthropicMessages {
                &body["tool_choice"]["type"]
            } else {
                &body["tool_choice"]
            };
            assert_eq!(choice, expected, "{provider}/{wire}");
        }
    }
}

#[tokio::test]
async fn builtin_profile_api_base_paths_reach_exact_endpoints() {
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    for (provider, env, wire, azure_base, suffix) in [
        (
            "groq",
            "GROQ_API_KEY",
            WireApi::OpenAiCompletions,
            false,
            "/chat/completions",
        ),
        (
            "openai",
            "OPENAI_API_KEY",
            WireApi::OpenAiResponses,
            false,
            "/responses",
        ),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::OpenAiCompletions,
            true,
            "/chat/completions",
        ),
        (
            "azure",
            "AZURE_OPENAI_API_KEY",
            WireApi::AzureOpenAiResponses,
            true,
            "/responses",
        ),
    ] {
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
        let mut models = Models::new();
        register_builtin(
            &mut models,
            map_context(&[(env, "fixture-key")]),
            Arc::new(opi_ai::http::HttpClient::new()),
            &BuiltinOverrides::default(),
        )
        .unwrap();
        let catalog = Catalog::builtin().unwrap();
        let entry = catalog
            .models(provider, CatalogOperation::Chat)
            .unwrap()
            .find(|entry| entry.api == wire.to_string())
            .unwrap();
        let mut model = models.get_model(provider, &entry.id).unwrap();
        let source_path = if azure_base {
            "/openai/v1".into()
        } else {
            url::Url::parse(model.base_url.as_deref().unwrap())
                .unwrap()
                .path()
                .trim_end_matches('/')
                .to_owned()
        };
        model.base_url = Some(format!("{}{source_path}", server.uri()));
        models
            .complete_simple(
                opi_ai::ChatModelRecord::new(provider, model),
                minimal_request("ignored"),
                Default::default(),
            )
            .await
            .unwrap();
        assert_eq!(
            server.received_requests().await.unwrap()[0].url.path(),
            format!("{source_path}{suffix}"),
            "{provider}/{wire}"
        );
    }
}
