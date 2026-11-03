//! Billing evidence from actual Models calls and loopback Responses-family wires.

use std::sync::Arc;

use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::azure_openai::{AzureOpenAIOptions, AzureOpenAIProvider, AzureOpenAIResponsesChatApi};
use opi_ai::http::HttpClient;
use opi_ai::openai_codex_responses::{
    CodexChatApi, CodexOptions, CodexServiceTier, OpenAiCodexResponsesProvider,
};
use opi_ai::openai_responses::{
    OpenAiResponsesChatApi, OpenAiResponsesOptions, OpenAiResponsesProvider, ResponsesServiceTier,
};
use opi_ai::provider::{ProviderError, Request, TransportPreference};
use opi_ai::stream::{Pricing, ServiceTier, StopReason};
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, ModelPricing, Models,
    WireApi,
};
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

struct FixtureAuth(AuthScheme);

impl AuthResolver for FixtureAuth {
    fn resolve<'a>(&'a self) -> opi_ai::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: self.0.clone(),
                secret: secrecy::SecretString::from("offline-fixture-key"),
                account_id: Some("offline-fixture-account".into()),
                base_url: None,
                provenance: Default::default(),
            })
        })
    }
}

fn pricing() -> ModelPricing {
    ModelPricing::try_new(
        Pricing {
            input_cost_per_mtok: 1.0,
            output_cost_per_mtok: 2.0,
            cache_read_cost_per_mtok: 0.1,
            cache_write_cost_per_mtok: 1.25,
        },
        vec![],
    )
    .unwrap()
}

fn request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![opi_ai::message::Message::User(
                opi_ai::message::UserMessage {
                    content: vec![opi_ai::message::InputContent::Text {
                        text: "price this fixture".into(),
                    }],
                    timestamp_ms: 0,
                },
            )],
            tools: vec![],
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        transport: TransportPreference::Sse,
    }
}

async fn capture(
    wire: WireApi,
    model_id: &str,
    requested: Option<ServiceTier>,
    response: Option<ServiceTier>,
    usage: Option<Value>,
) -> opi_ai::message::AssistantMessage {
    let server = MockServer::start().await;
    let has_usage = usage.is_some();
    let mut terminal = json!({"type":"response.completed","response":{
        "id":"fixture-response","model":model_id,"status":"completed","output":[],
    }});
    if let Some(usage) = usage {
        terminal["response"]["usage"] = usage;
    }
    if let Some(response) = response {
        terminal["response"]["service_tier"] = serde_json::to_value(response).unwrap();
    }
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(format!("data: {terminal}\n\n")),
        )
        .mount(&server)
        .await;
    let info = ModelInfo::new(
        model_id,
        model_id,
        wire,
        ModelCapabilities::new(2_000_000, 4096).with_streaming(true),
    )
    .with_pricing(pricing())
    .unwrap();
    let mut models = Models::new();
    let (provider, expected_path) = match wire {
        WireApi::OpenAiResponses => ("openai-responses", "/v1/responses"),
        WireApi::OpenAiCodexResponses => ("openai-codex", "/codex/responses"),
        WireApi::AzureOpenAiResponses => ("azure", "/openai/v1/responses"),
        _ => unreachable!(),
    };
    let record = ChatModelRecord::new(provider, info.clone());
    let request = request(&format!("{provider}:{model_id}"));
    let message = match wire {
        WireApi::OpenAiResponses => {
            let api = OpenAiResponsesChatApi::new(
                Arc::new(OpenAiResponsesProvider::new(Some(server.uri()))),
                info,
            )
            .unwrap();
            let handle = models
                .register_chat_api(
                    Arc::new(api),
                    Arc::new(FixtureAuth(AuthScheme::Bearer)),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
            let mut options = OpenAiResponsesOptions::default();
            options.service_tier = requested.map(|tier| match tier {
                ServiceTier::Auto => ResponsesServiceTier::Auto,
                ServiceTier::Default => ResponsesServiceTier::Default,
                ServiceTier::Flex => ResponsesServiceTier::Flex,
                ServiceTier::Priority => ResponsesServiceTier::Priority,
                ServiceTier::Fast => ResponsesServiceTier::Fast,
                ServiceTier::Scale => ResponsesServiceTier::Scale,
                ServiceTier::Unknown => panic!("unknown tiers are only response fixtures"),
            });
            models
                .complete(record, request, handle.options(options))
                .await
                .unwrap()
        }
        WireApi::OpenAiCodexResponses => {
            let api = CodexChatApi::new(
                Arc::new(OpenAiCodexResponsesProvider::new(
                    Some(server.uri()),
                    vec![],
                    Arc::new(HttpClient::new()),
                )),
                info,
            )
            .unwrap();
            let handle = models
                .register_chat_api(
                    Arc::new(api),
                    Arc::new(FixtureAuth(AuthScheme::Bearer)),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
            let mut options = CodexOptions::default();
            options.service_tier = requested.map(|tier| match tier {
                ServiceTier::Auto => CodexServiceTier::Auto,
                ServiceTier::Default => CodexServiceTier::Default,
                ServiceTier::Flex => CodexServiceTier::Flex,
                ServiceTier::Priority => CodexServiceTier::Priority,
                ServiceTier::Fast => CodexServiceTier::Fast,
                ServiceTier::Scale => CodexServiceTier::Scale,
                ServiceTier::Unknown => panic!("unknown tiers are only response fixtures"),
            });
            models
                .complete(record, request, handle.options(options))
                .await
                .unwrap()
        }
        WireApi::AzureOpenAiResponses => {
            let provider = AzureOpenAIProvider::for_records(
                Some(format!("{}/openai/v1", server.uri())),
                vec![info.clone()],
                None,
            )
            .unwrap();
            let api = AzureOpenAIResponsesChatApi::new(Arc::new(provider), info).unwrap();
            let handle = models
                .register_chat_api(
                    Arc::new(api),
                    Arc::new(FixtureAuth(AuthScheme::ApiKey)),
                    AuthProvenanceSource::Static,
                )
                .unwrap();
            models
                .complete(
                    record,
                    request,
                    handle.options(AzureOpenAIOptions::default()),
                )
                .await
                .unwrap()
        }
        _ => unreachable!(),
    };
    let captures = server.received_requests().await.unwrap();
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].url.path(), expected_path);
    let body: Value = serde_json::from_slice(&captures[0].body).unwrap();
    assert_eq!(body["model"], model_id);
    assert_eq!(
        body.get("service_tier").cloned(),
        requested.map(|tier| serde_json::to_value(tier).unwrap())
    );
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.wire, Some(wire));
    assert_eq!(
        message.usage.service_tier, response,
        "the original response tier is preserved"
    );
    let expected_effective = match wire {
        WireApi::AzureOpenAiResponses => None,
        WireApi::OpenAiCodexResponses
            if response == Some(ServiceTier::Default)
                && matches!(requested, Some(ServiceTier::Flex | ServiceTier::Priority)) =>
        {
            requested
        }
        _ => response.or(requested),
    };
    assert_eq!(
        message.usage.effective_service_tier,
        has_usage.then_some(expected_effective).flatten()
    );
    let round_trip: opi_ai::message::AssistantMessage =
        serde_json::from_value(serde_json::to_value(&message).unwrap()).unwrap();
    assert_eq!(round_trip.usage, message.usage);
    message
}

fn full_usage() -> Option<Value> {
    Some(json!({"input_tokens":1_000_000,"output_tokens":0}))
}

#[tokio::test]
async fn responses_request_tier_fallback_changes_actual_terminal_cost() {
    let message = capture(
        WireApi::OpenAiResponses,
        "gpt-5.5",
        Some(ServiceTier::Flex),
        None,
        full_usage(),
    )
    .await;
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            message.wire.unwrap()
        )
        .unwrap()
        .total_cost(),
        0.5
    );
}

#[tokio::test]
async fn codex_default_preserves_requested_priority_for_cost() {
    let message = capture(
        WireApi::OpenAiCodexResponses,
        "gpt-5.5",
        Some(ServiceTier::Priority),
        Some(ServiceTier::Default),
        full_usage(),
    )
    .await;
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            message.wire.unwrap()
        )
        .unwrap()
        .total_cost(),
        2.5
    );
}

#[tokio::test]
async fn codex_fast_has_no_responses_multiplier() {
    let message = capture(
        WireApi::OpenAiCodexResponses,
        "gpt-5.5",
        None,
        Some(ServiceTier::Fast),
        full_usage(),
    )
    .await;
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            message.wire.unwrap()
        )
        .unwrap()
        .total_cost(),
        1.0
    );
}

#[tokio::test]
async fn azure_actual_usage_requires_wire_qualified_cost() {
    let message = capture(
        WireApi::AzureOpenAiResponses,
        "gpt-5.5",
        None,
        Some(ServiceTier::Flex),
        full_usage(),
    )
    .await;
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            message.wire.unwrap()
        )
        .unwrap()
        .total_cost(),
        1.0
    );
}

#[tokio::test]
async fn responses_cached_input_is_a_subset_before_costing() {
    let message = capture(
        WireApi::OpenAiResponses,
        "gpt-5.5",
        None,
        None,
        Some(json!({
            "input_tokens":1000,"output_tokens":0,"total_tokens":1000,
            "input_tokens_details":{"cached_tokens":100,"cache_write_tokens":50}
        })),
    )
    .await;
    assert_eq!(message.usage.input_tokens, 850);
    assert_eq!(message.usage.cache_read_tokens, 100);
    assert_eq!(message.usage.cache_write_tokens, 50);
    assert_eq!(message.usage.total_tokens, Some(1000));
    assert!(
        (opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            message.wire.unwrap()
        )
        .unwrap()
        .total_cost()
            - 0.0009225)
            .abs()
            < 1e-12
    );
}

#[tokio::test]
async fn response_precedence_and_exact_model_tier_rules_reach_terminal_cost() {
    for (wire, model, requested, response, expected) in [
        (
            WireApi::OpenAiResponses,
            "gpt-5.5",
            Some(ServiceTier::Flex),
            Some(ServiceTier::Default),
            1.0,
        ),
        (
            WireApi::OpenAiResponses,
            "gpt-5.5",
            None,
            Some(ServiceTier::Fast),
            2.5,
        ),
        (
            WireApi::OpenAiResponses,
            "gpt-5.5-preview",
            None,
            Some(ServiceTier::Fast),
            2.0,
        ),
        (
            WireApi::OpenAiResponses,
            "gpt-5.5",
            Some(ServiceTier::Flex),
            Some(ServiceTier::Auto),
            1.0,
        ),
        (
            WireApi::OpenAiResponses,
            "gpt-5.5",
            Some(ServiceTier::Flex),
            Some(ServiceTier::Scale),
            1.0,
        ),
        (
            WireApi::OpenAiResponses,
            "gpt-5.5",
            Some(ServiceTier::Priority),
            Some(ServiceTier::Flex),
            0.5,
        ),
        (
            WireApi::OpenAiCodexResponses,
            "gpt-5.5-preview",
            Some(ServiceTier::Priority),
            Some(ServiceTier::Default),
            2.0,
        ),
        (
            WireApi::OpenAiCodexResponses,
            "gpt-5.5",
            Some(ServiceTier::Flex),
            Some(ServiceTier::Default),
            0.5,
        ),
        (
            WireApi::OpenAiCodexResponses,
            "gpt-5.5",
            Some(ServiceTier::Fast),
            None,
            1.0,
        ),
        (
            WireApi::AzureOpenAiResponses,
            "gpt-5.5",
            None,
            Some(ServiceTier::Priority),
            1.0,
        ),
    ] {
        let message = capture(wire, model, requested, response, full_usage()).await;
        assert_eq!(
            opi_ai::calculate_cost_tiered(&message.usage, &pricing(), &message.model, wire)
                .unwrap()
                .total_cost(),
            expected
        );
    }
}

#[tokio::test]
async fn missing_usage_and_unknown_tier_stay_unknown_but_known_zero_is_free() {
    let missing = capture(
        WireApi::OpenAiResponses,
        "gpt-5.5",
        Some(ServiceTier::Flex),
        None,
        None,
    )
    .await;
    assert!(!missing.usage.reported);
    assert!(
        opi_ai::calculate_cost_tiered(
            &missing.usage,
            &pricing(),
            &missing.model,
            WireApi::OpenAiResponses
        )
        .is_none()
    );
    let unknown = capture(
        WireApi::OpenAiResponses,
        "gpt-5.5",
        Some(ServiceTier::Flex),
        Some(ServiceTier::Unknown),
        full_usage(),
    )
    .await;
    assert_eq!(
        unknown.usage.effective_service_tier,
        Some(ServiceTier::Unknown)
    );
    assert!(
        opi_ai::calculate_cost_tiered(
            &unknown.usage,
            &pricing(),
            &unknown.model,
            WireApi::OpenAiResponses
        )
        .is_none()
    );
    let zero = capture(
        WireApi::OpenAiResponses,
        "gpt-5.5",
        None,
        None,
        Some(json!({"input_tokens":0,"output_tokens":0})),
    )
    .await;
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &zero.usage,
            &pricing(),
            &zero.model,
            WireApi::OpenAiResponses
        )
        .unwrap()
        .total_cost(),
        0.0
    );
}

use opi_ai::ws;
#[path = "support/ws_peer.rs"]
mod ws_peer;

#[tokio::test]
async fn codex_websocket_keeps_requested_tier_separate_from_response_default() {
    use std::io::{Read, Write};
    let peer = ws_peer::Peer::spawn(
        |mut socket| {
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .map_err(|error| error.to_string())?;
            let mut read_frame = || -> Result<(u8, Vec<u8>), String> {
                let mut head = [0; 2];
                socket
                    .read_exact(&mut head)
                    .map_err(|error| error.to_string())?;
                assert_eq!(head[1] & 0x80, 0x80);
                let size = match head[1] & 0x7f {
                    126 => {
                        let mut size = [0; 2];
                        socket
                            .read_exact(&mut size)
                            .map_err(|error| error.to_string())?;
                        usize::from(u16::from_be_bytes(size))
                    }
                    127 => return Err("unexpected large fixture frame".into()),
                    size => usize::from(size),
                };
                let mut mask = [0; 4];
                socket
                    .read_exact(&mut mask)
                    .map_err(|error| error.to_string())?;
                let mut body = vec![0; size];
                socket
                    .read_exact(&mut body)
                    .map_err(|error| error.to_string())?;
                for (index, byte) in body.iter_mut().enumerate() {
                    *byte ^= mask[index % 4];
                }
                Ok((head[0] & 0x0f, body))
            };
            let (opcode, body) = read_frame()?;
            assert_eq!(opcode, 1);
            let body: Value = serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            assert_eq!(body["service_tier"], "priority");
            let terminal = json!({"type":"response.completed","response":{
                "id":"ws-fixture","model":"gpt-5.5","status":"completed","output":[],"service_tier":"default",
                "usage":{"input_tokens":1_000_000,"output_tokens":0}
            }}).to_string();
            let mut frame = vec![0x81, 126];
            frame.extend_from_slice(&(terminal.len() as u16).to_be_bytes());
            frame.extend_from_slice(terminal.as_bytes());
            socket
                .write_all(&frame)
                .map_err(|error| error.to_string())?;
            // Echo the peer's Close after the temporary generation completes.
            let mut head = [0; 2];
            socket
                .read_exact(&mut head)
                .map_err(|error| error.to_string())?;
            assert_eq!(head[0] & 0x0f, 8);
            let size = usize::from(head[1] & 0x7f);
            let mut mask = [0; 4];
            socket
                .read_exact(&mut mask)
                .map_err(|error| error.to_string())?;
            let mut close = vec![0; size];
            socket
                .read_exact(&mut close)
                .map_err(|error| error.to_string())?;
            for (index, byte) in close.iter_mut().enumerate() {
                *byte ^= mask[index % 4];
            }
            socket
                .write_all(&[0x88, size as u8])
                .map_err(|error| error.to_string())?;
            socket
                .write_all(&close)
                .map_err(|error| error.to_string())?;
            Ok(())
        },
        |head| ws_peer::response_101(head, &[]),
    );
    let info = ModelInfo::new(
        "gpt-5.5",
        "fixture",
        WireApi::OpenAiCodexResponses,
        ModelCapabilities::new(2_000_000, 4096),
    );
    let provider = Arc::new(OpenAiCodexResponsesProvider::new(
        Some(format!("http://{}", peer.url().authority())),
        vec![],
        Arc::new(HttpClient::new()),
    ));
    let mut models = Models::new();
    let api = models
        .register_chat_api(
            Arc::new(CodexChatApi::new(provider, info.clone()).unwrap()),
            Arc::new(FixtureAuth(AuthScheme::Bearer)),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut options = CodexOptions::default();
    options.service_tier = Some(CodexServiceTier::Priority);
    let mut request = request("openai-codex:gpt-5.5");
    request.transport = TransportPreference::Websocket;
    let message = models
        .complete(
            ChatModelRecord::new("openai-codex", info),
            request,
            api.options(options),
        )
        .await
        .unwrap();
    assert_eq!(message.usage.service_tier, Some(ServiceTier::Default));
    assert_eq!(
        message.usage.effective_service_tier,
        Some(ServiceTier::Priority)
    );
    assert_eq!(
        opi_ai::calculate_cost_tiered(
            &message.usage,
            &pricing(),
            &message.model,
            WireApi::OpenAiCodexResponses
        )
        .unwrap()
        .total_cost(),
        2.5
    );
    models.cleanup_session_resources(None).await.unwrap();
    peer.finish().unwrap();
}
