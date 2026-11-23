//! OpenCode Zen and Go catalog selection, requests and prepared-call binding.

use super::*;

#[test]
fn builtin_full_record_keeps_pinned_model_compatibility() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    let model = models.get_model("opencode", "big-pickle").unwrap();
    let opi_ai::model_info::WireCompat::OpenAiCompletions(compat) = model.compat else {
        panic!("chat wire")
    };
    assert_eq!(compat.max_tokens_field.as_deref(), Some("max_tokens"));
    assert_eq!(
        compat.instruction_role,
        Some(opi_ai::model_info::ChatInstructionRole::System)
    );
}

fn opencode_builtin_models() -> Models {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[("OPENCODE_API_KEY", "fixture-opencode")]),
        Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    models
}

async fn capture_opencode_chat(
    models: &Models,
    provider: &str,
    id: &str,
    wire: WireApi,
    request_path: &str,
    level: opi_ai::model_info::ThinkingLevel,
) -> serde_json::Value {
    use opi_ai::model_info::ThinkingLevel;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(request_path))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(node5_reply(wire)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let mut model = models
        .get_model(provider, id)
        .unwrap_or_else(|| panic!("missing {provider}/{id}"));
    assert_eq!(model.wire_api, wire);
    // Keep the endpoint path from the selected builtin model record.
    let suffix = model
        .base_url
        .as_ref()
        .unwrap()
        .strip_prefix("https://opencode.ai/")
        .unwrap();
    model.base_url = Some(format!("{}/{suffix}", server.uri()));
    let mut request = minimal_request(id);
    request.transport = opi_ai::provider::TransportPreference::Sse;
    request.max_tokens = Some(32);
    request.session_id = Some("opencode-session".into());
    request.thinking = ThinkingConfig {
        enabled: level != ThinkingLevel::None,
        budget_tokens: None,
        level,
    };
    let output = models
        .complete_simple(
            opi_ai::ChatModelRecord::new(provider, model),
            request,
            Default::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("{provider}/{id}: {error:?}"));
    assert_eq!(output.provider, provider);
    assert_eq!(output.model, id);
    assert_eq!(assistant_text(&output), "hi");

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1, "{id}");
    let sent = &sent[0];
    assert_eq!(sent.url.path(), request_path);
    assert_eq!(sent.headers["x-opencode-session"], "opencode-session");
    match wire {
        WireApi::AnthropicMessages => assert_eq!(sent.headers["x-api-key"], "fixture-opencode"),
        WireApi::GoogleGenerativeAi => {
            assert_eq!(sent.headers["x-goog-api-key"], "fixture-opencode");
            assert_eq!(sent.url.query(), Some("alt=sse"));
        }
        _ => assert_eq!(sent.headers["authorization"], "Bearer fixture-opencode"),
    }
    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
    if wire != WireApi::GoogleGenerativeAi {
        assert_eq!(body["model"], id);
    }
    body
}

#[test]
fn builtin_opencode_zen_retires_models_removed_from_the_catalog() {
    let models = opencode_builtin_models();
    let catalog = Catalog::builtin().unwrap();
    for id in [
        "cat-jam-free",
        "claude-sonnet-4",
        "deepseek-v4-flash",
        "deepseek-v4-flash-vision-exp",
        "longcat-2.5-preview-free",
        "muse-spark-1.3-contributor-free",
        "nemotron-3-ultra-free",
        "space-bunny-free",
        "step-5-preview-free",
    ] {
        assert!(
            catalog
                .model("opencode", CatalogOperation::Chat, id)
                .is_none(),
            "{id}"
        );
        assert!(models.get_model("opencode", id).is_none(), "{id}");
    }
    assert_eq!(models.get_models(Some("opencode")).len(), 79);
}

#[tokio::test]
async fn builtin_opencode_zen_dispatches_published_routes_and_efforts() {
    use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierStopReason};
    use opi_ai::model_info::ThinkingLevel;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    let models = opencode_builtin_models();

    for (id, wire, request_path, level, effort) in [
        (
            "kimi-k3",
            WireApi::OpenAiCompletions,
            "/inference/openai/v1/chat/completions",
            ThinkingLevel::Medium,
            Some("high"),
        ),
        (
            "qwen3.8-max",
            WireApi::OpenAiCompletions,
            "/inference/openai/v1/chat/completions",
            ThinkingLevel::High,
            Some("xhigh"),
        ),
        (
            "muse-wonder-1.0",
            WireApi::OpenAiResponses,
            "/inference/openai/v1/responses",
            ThinkingLevel::None,
            None,
        ),
        (
            "claude-opus-4-8",
            WireApi::AnthropicMessages,
            "/inference/anthropic/v1/messages",
            ThinkingLevel::None,
            None,
        ),
        (
            "gemini-3.1-pro",
            WireApi::GoogleGenerativeAi,
            "/inference/google/v1beta/models/gemini-3.1-pro:streamGenerateContent",
            ThinkingLevel::None,
            None,
        ),
    ] {
        let body = capture_opencode_chat(&models, "opencode", id, wire, request_path, level).await;
        if let Some(effort) = effort {
            assert_eq!(body["reasoning_effort"], effort, "{id}");
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/inference/systemone/v1/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "answers": {},
            "usage": {"input_tokens": 2, "output_tokens": 1},
        })))
        .expect(2)
        .mount(&server)
        .await;
    let state = json!({"ticket": "test submission"});
    let ids = ["jev-1.13", "jev-1.13-free"];
    for id in ids {
        let mut record = models.classifier_model("opencode", id).unwrap();
        let suffix = record
            .base_url
            .strip_prefix("https://opencode.ai/")
            .unwrap();
        record.base_url = format!("{}/{suffix}", server.uri());
        let result = models
            .classify(
                record,
                ClassifierContext {
                    state: state.clone(),
                    ..Default::default()
                },
                ClassifierCall::default(),
            )
            .await;
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{id}: {:?}",
            result.error
        );
        assert_eq!(result.provider, "opencode");
        assert_eq!(result.model, id);
    }
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), ids.len());
    for (request, id) in sent.iter().zip(ids) {
        assert_eq!(request.url.path(), "/inference/systemone/v1/systemone");
        assert_eq!(request.headers["authorization"], "Bearer fixture-opencode");
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body["model"], id);
        assert_eq!(body["state"], state);
    }
}

#[test]
fn builtin_opencode_go_retires_models_removed_from_the_catalog() {
    let models = opencode_builtin_models();
    let catalog = Catalog::builtin().unwrap();
    let listed = models.get_models_of_type(CatalogOperation::Chat, Some("opencode-go"));
    for id in [
        "cat-jam-free",
        "gemini-4-argon-contributor",
        "grok-4.6",
        "longcat-2.5-preview-free",
        "minimax-m2.7",
        "space-bunny",
        "step-5-preview-free",
    ] {
        assert!(
            catalog
                .model("opencode-go", CatalogOperation::Chat, id)
                .is_none(),
            "{id}"
        );
        assert!(models.get_model("opencode-go", id).is_none(), "{id}");
        assert!(listed.iter().all(|record| record.id() != id), "{id}");
    }
    assert_eq!(listed.len(), 30);

    let kimi = models.get_model("opencode-go", "kimi-k3").unwrap();
    assert_eq!(kimi.pricing.unwrap().base.cache_write_cost_per_mtok, 3.0);
}

#[tokio::test]
async fn builtin_opencode_go_dispatches_published_routes_and_efforts() {
    use opi_ai::model_info::ThinkingLevel;
    use serde_json::json;

    let models = opencode_builtin_models();
    let provider = "opencode-go";
    capture_opencode_chat(
        &models,
        provider,
        "gpt-5.6-luna",
        WireApi::OpenAiResponses,
        "/inference/go/openai/v1/responses",
        ThinkingLevel::None,
    )
    .await;
    capture_opencode_chat(
        &models,
        provider,
        "qwen3.8-flash",
        WireApi::AnthropicMessages,
        "/inference/go/anthropic/v1/messages",
        ThinkingLevel::None,
    )
    .await;

    for (level, expected) in [
        (ThinkingLevel::Medium, Some(json!("high"))),
        (ThinkingLevel::None, None),
    ] {
        let body = capture_opencode_chat(
            &models,
            provider,
            "kimi-k3.1-pro",
            WireApi::OpenAiCompletions,
            "/inference/go/openai/v1/chat/completions",
            level,
        )
        .await;
        assert_eq!(body.get("reasoning_effort"), expected.as_ref(), "{level:?}");
    }

    for (level, expected) in [
        (
            ThinkingLevel::XHigh,
            json!({"includeThoughts": true, "thinkingLevel": "HIGH"}),
        ),
        (ThinkingLevel::None, json!({"thinkingLevel": "LOW"})),
    ] {
        let body = capture_opencode_chat(
            &models,
            provider,
            "gemini-4-argon",
            WireApi::GoogleGenerativeAi,
            "/inference/go/google/v1beta/models/gemini-4-argon:streamGenerateContent",
            level,
        )
        .await;
        assert_eq!(
            body["generationConfig"]["thinkingConfig"], expected,
            "{level:?}"
        );
    }
}

#[tokio::test]
async fn builtin_prepared_opencode_keeps_session_headers_and_caller_precedence() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

    struct CountingContext(Arc<AtomicUsize>);
    impl opi_ai::auth::AuthContext for CountingContext {
        fn env(&self, name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            Ok(if name == "OPENCODE_API_KEY" {
                self.0.fetch_add(1, Ordering::SeqCst);
                Some("frozen-profile-key".into())
            } else {
                None
            })
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            panic!("the configured API key must not discover credential files")
        }
    }

    for provider in ["opencode", "opencode-go"] {
        // Responses returns a prepared replacement; Chat keeps its mapped route.
        for wire in [WireApi::OpenAiResponses, WireApi::OpenAiCompletions] {
            for advanced in [false, true] {
                for (session, caller_value, remove, expected) in [
                    (
                        Some("profile-session"),
                        None,
                        false,
                        Some("profile-session"),
                    ),
                    (None, None, false, None),
                    (Some(""), None, false, None),
                    (
                        Some("profile-session"),
                        Some("caller-value"),
                        false,
                        Some("caller-value"),
                    ),
                    (Some("profile-session"), None, true, None),
                ] {
                    let server = MockServer::start().await;
                    let replacement_server = MockServer::start().await;
                    Mock::given(method("POST"))
                        .respond_with(
                            ResponseTemplate::new(200)
                                .insert_header("content-type", "text/event-stream")
                                .set_body_string(node5_reply(wire)),
                        )
                        .expect(1)
                        .mount(&server)
                        .await;
                    let resolutions = Arc::new(AtomicUsize::new(0));
                    let transforms = Arc::new(AtomicUsize::new(0));
                    let mut models = Models::new();
                    let mut overrides = BuiltinOverrides::default();
                    overrides
                        .base_urls
                        .insert(provider.into(), format!("{}/selected", server.uri()));
                    register_builtin(
                        &mut models,
                        Arc::new(CountingContext(resolutions.clone())),
                        Arc::new(opi_ai::http::HttpClient::new()),
                        &overrides,
                    )
                    .unwrap();
                    let mut info = models
                        .get_models(Some(provider))
                        .into_iter()
                        .find(|model| model.wire_api == wire)
                        .unwrap();
                    let model_id = info.id.clone();
                    let spec = format!("{provider}:{model_id}");
                    let mut request = minimal_request(&spec);
                    request.transport = opi_ai::provider::TransportPreference::Sse;
                    request.session_id = session.map(str::to_owned);
                    request.max_tokens = Some(32);
                    if let Some(value) = caller_value {
                        request
                            .extra_headers
                            .push(("X-OpenCode-Session".into(), value.into()));
                    }
                    let transform_count = transforms.clone();
                    let before_transform =
                        caller_value.or(session.filter(|value| !value.is_empty()));
                    request.callbacks = opi_ai::callbacks::RequestCallbacks::default()
                        .with_header_transform(Arc::new(move |headers| {
                            transform_count.fetch_add(1, Ordering::SeqCst);
                            let actual: Vec<_> = headers
                                .iter()
                                .filter(|(name, _)| name.eq_ignore_ascii_case("x-opencode-session"))
                                .map(|(_, value)| value.as_str())
                                .collect();
                            assert_eq!(actual, before_transform.into_iter().collect::<Vec<_>>());
                            Ok(headers
                                .iter()
                                .filter(|(name, _)| {
                                    !remove || !name.eq_ignore_ascii_case("x-opencode-session")
                                })
                                .cloned()
                                .collect())
                        }));
                    assert_eq!(resolutions.load(Ordering::SeqCst), 0);

                    let replace_registration = |models: &mut Models| {
                        assert!(models.delete_provider(provider));
                        let mut changed = BuiltinOverrides::default();
                        changed.base_urls.insert(
                            provider.into(),
                            format!("{}/replacement", replacement_server.uri()),
                        );
                        register_builtin(
                            models,
                            map_context(&[("OPENCODE_API_KEY", "replacement-key")]),
                            Arc::new(opi_ai::http::HttpClient::new()),
                            &changed,
                        )
                        .unwrap();
                    };
                    let expected_base = if advanced { "/selected" } else { "/owned" };
                    let output = if advanced {
                        let prepared = models.prepare_call(&spec, request).await.unwrap();
                        assert_eq!(prepared.route().provider_id, provider);
                        assert_eq!(prepared.route().model_id, model_id);
                        assert_eq!(resolutions.load(Ordering::SeqCst), 1);
                        replace_registration(&mut models);
                        drain_to_completion(prepared.start_attempt().unwrap())
                            .await
                            .unwrap()
                    } else {
                        // The owned record overrides the catalog endpoint and is
                        // captured before a later registration changes the driver.
                        info.base_url = Some(format!("{}/owned", server.uri()));
                        info.headers
                            .push(("x-owned-record".into(), Some("frozen".into())));
                        let completion = models.complete_simple(
                            opi_ai::ChatModelRecord::new(provider, info),
                            request,
                            Default::default(),
                        );
                        assert_eq!(resolutions.load(Ordering::SeqCst), 0);
                        replace_registration(&mut models);
                        completion.await.unwrap()
                    };
                    assert_eq!(output.provider, provider);
                    assert_eq!(output.model, model_id);
                    assert_eq!(assistant_text(&output), "hi");
                    assert_eq!(resolutions.load(Ordering::SeqCst), 1);
                    assert_eq!(transforms.load(Ordering::SeqCst), 1);
                    let sent = server.received_requests().await.unwrap();
                    assert_eq!(sent.len(), 1);
                    let sent = &sent[0];
                    let suffix = if wire == WireApi::OpenAiResponses {
                        "/responses"
                    } else {
                        "/chat/completions"
                    };
                    assert_eq!(sent.url.path(), format!("{expected_base}{suffix}"));
                    assert_eq!(sent.headers["authorization"], "Bearer frozen-profile-key");
                    let session_values: Vec<_> = sent
                        .headers
                        .get_all("x-opencode-session")
                        .iter()
                        .map(|value| value.to_str().unwrap())
                        .collect();
                    assert_eq!(session_values, expected.into_iter().collect::<Vec<_>>());
                    if !advanced {
                        assert_eq!(sent.headers["x-owned-record"], "frozen");
                    }
                    let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
                    assert_eq!(body["model"], model_id);
                    assert!(
                        replacement_server
                            .received_requests()
                            .await
                            .unwrap()
                            .is_empty()
                    );
                }
            }
        }
    }
}
