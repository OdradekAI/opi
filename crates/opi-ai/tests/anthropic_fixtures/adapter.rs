//! Anthropic assembled adapter and Models lifecycle fixtures.

use super::text_fixture;
use futures_util::StreamExt;
use opi_ai::api::anthropic::{
    AnthropicMessagesChatApi, AnthropicMessagesOptions, AnthropicMessagesWireAdapter,
    AnthropicThinkingDisplay, AnthropicToolChoice, ParsedEvent, parse_sse_events,
};
use opi_ai::api::contract::SimpleOptions;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::http::HttpClient;
use opi_ai::message::{AssistantContent, InputContent, Message, ToolDef, UserMessage};
use opi_ai::model_info::{
    AnthropicMessagesCompat, ModelCapabilities, ModelInfo, WireApi, WireCompat,
};
use opi_ai::models::Models;
use opi_ai::provider::{CacheRetention, Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::stream::{AssistantStreamEvent, StopReason};
#[cfg(feature = "telemetry-tracing")]
use opi_ai::telemetry::TelemetryContext;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn caps_39() -> ModelCapabilities {
    ModelCapabilities::new(200_000, 64_000)
        .with_images(true)
        .with_streaming(true)
        .with_thinking(true)
}

fn compat_39(
    force_adaptive: bool,
    mid_convo_system: bool,
    mid_convo_tools: bool,
) -> AnthropicMessagesCompat {
    AnthropicMessagesCompat {
        supports_eager_tool_input_streaming: false,
        force_adaptive_thinking: force_adaptive,
        supports_temperature: true,
        supports_strict_tools: false,
        supports_mid_convo_system_messages: mid_convo_system,
        supports_mid_convo_tool_changes: mid_convo_tools,
        ..Default::default()
    }
}

fn model_39(id: &str, compat: AnthropicMessagesCompat) -> ModelInfo {
    ModelInfo::new(id, id, WireApi::AnthropicMessages, caps_39())
        .with_compat(WireCompat::AnthropicMessages(compat))
        .expect("matching wire compat")
}

fn profile_39(model: &ModelInfo) -> AnthropicProvider {
    AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    )
}

fn provider_39(
    model: &ModelInfo,
    options: AnthropicMessagesOptions,
) -> AnthropicMessagesWireAdapter {
    AnthropicMessagesWireAdapter::with_options(profile_39(model), options)
}

fn native_options_39() -> AnthropicMessagesOptions {
    AnthropicMessagesOptions::default()
}

fn request_39(thinking: ThinkingConfig) -> Request {
    request_39_for("anthropic:claude-sonnet-4-5", thinking)
}

fn request_39_for(model: &str, thinking: ThinkingConfig) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: Default::default(),
        thinking,
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

trait BodyBuilder39 {
    fn build_body_39(&self, request: &Request) -> serde_json::Value;
}

impl BodyBuilder39 for AnthropicProvider {
    fn build_body_39(&self, request: &Request) -> serde_json::Value {
        self.build_request_body(request).expect("body compiles")
    }
}

impl BodyBuilder39 for AnthropicMessagesWireAdapter {
    fn build_body_39(&self, request: &Request) -> serde_json::Value {
        self.build_request_body(request).expect("body compiles")
    }
}

fn body_39(provider: &impl BodyBuilder39, request: &Request) -> serde_json::Value {
    provider.build_body_39(request)
}

#[test]
fn native_enabled_without_budget_uses_source_default_1024_and_summarized() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    let thinking = &body["thinking"];
    assert_eq!(thinking["type"], "enabled");
    assert_eq!(thinking["budget_tokens"], 1024, "source native default");
    assert_eq!(
        thinking["display"], "summarized",
        "enabled thinking defaults to the summarized display"
    );
}

#[test]
fn native_zero_budget_falls_back_to_1024_per_source_or_semantics() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.thinking_budget_tokens = Some(0);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["budget_tokens"], 1024);
}

#[test]
fn native_explicit_budget_and_detailed_display_reach_the_wire() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.thinking_budget_tokens = Some(5000);
    options.thinking_display = Some(AnthropicThinkingDisplay::Detailed);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["budget_tokens"], 5000);
    assert_eq!(body["thinking"]["display"], "detailed");
}

#[test]
fn native_disabled_projects_typed_disabled_on_reasoning_models() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(false);
    let provider = provider_39(&model, options);
    let body = body_39(&provider, &request_39(ThinkingConfig::default()));

    assert_eq!(body["thinking"]["type"], "disabled");
    assert!(
        body.get("betas")
            .is_none_or(|b| b
                .as_array()
                .is_none_or(|betas| !betas.iter().any(|beta| beta
                    .as_str()
                    .is_some_and(|beta| beta.starts_with("interleaved"))))),
        "disabled thinking never requests the interleaved beta"
    );
}

#[test]
fn native_omitted_thinking_keeps_neutral_request_path() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = provider_39(&model, native_options_39());
    let request = request_39(ThinkingConfig {
        enabled: true,
        budget_tokens: Some(5000),
        level: opi_ai::ThinkingLevel::High,
    });
    let body = body_39(&provider, &request);

    // Omitted native tri-state: the neutral request drives the wire and the
    // native default does not override an explicit neutral budget.
    assert_eq!(body["thinking"]["budget_tokens"], 5000);
    assert_eq!(body["thinking"]["display"], "summarized");
}

#[test]
fn interleaved_beta_defaults_true_explicit_false_omits_adaptive_never() {
    let numeric = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let mut on = native_options_39();
    on.thinking_enabled = Some(true);
    let body = body_39(
        &provider_39(&numeric, on),
        &request_39(ThinkingConfig::default()),
    );
    let betas = body["betas"].as_array().expect("default interleaved beta");
    assert!(
        betas
            .iter()
            .any(|beta| beta == "interleaved-thinking-2025-05-14")
    );

    let mut off = native_options_39();
    off.thinking_enabled = Some(true);
    off.interleaved_thinking = Some(false);
    let body = body_39(
        &provider_39(&numeric, off),
        &request_39(ThinkingConfig::default()),
    );
    assert!(
        body.get("betas").is_none(),
        "explicit interleaved=false must omit the beta"
    );

    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut adaptive_on = native_options_39();
    adaptive_on.thinking_enabled = Some(true);
    let body = body_39(
        &provider_39(&adaptive, adaptive_on),
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );
    assert!(
        body.get("betas").is_none(),
        "adaptive thinking models ignore the interleaved beta"
    );
}

#[test]
fn adaptive_native_effort_uses_output_config_and_display_without_budget() {
    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    options.effort = Some(opi_ai::ThinkingLevel::High);
    let provider = provider_39(&adaptive, options);
    let body = body_39(
        &provider,
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );

    assert_eq!(body["thinking"]["type"], "adaptive");
    assert_eq!(body["thinking"]["display"], "summarized");
    assert_eq!(body["output_config"]["effort"], "high");
    assert!(
        body["thinking"].get("budget_tokens").is_none(),
        "adaptive thinking carries no budget"
    );
}

#[test]
fn adaptive_native_without_effort_omits_output_config() {
    let adaptive = model_39("claude-opus-4-7", compat_39(true, false, false));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let provider = provider_39(&adaptive, options);
    let body = body_39(
        &provider,
        &request_39_for("anthropic:claude-opus-4-7", ThinkingConfig::default()),
    );

    assert_eq!(body["thinking"]["type"], "adaptive");
    assert!(body.get("output_config").is_none());
}

#[test]
fn native_tool_choice_variants_reach_the_wire() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let request = request_39(ThinkingConfig::default());
    for (choice, expected) in [
        (
            AnthropicToolChoice::Auto,
            serde_json::json!({"type": "auto"}),
        ),
        (AnthropicToolChoice::Any, serde_json::json!({"type": "any"})),
        (
            AnthropicToolChoice::None_,
            serde_json::json!({"type": "none"}),
        ),
        (
            AnthropicToolChoice::Named("get_weather".into()),
            serde_json::json!({"type": "tool", "name": "get_weather"}),
        ),
    ] {
        let mut options = native_options_39();
        options.tool_choice = Some(choice);
        let provider = provider_39(&model, options);
        let body = body_39(&provider, &request);
        assert_eq!(body["tool_choice"], expected);
    }
}

fn tool_def_39(name: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: format!("tool {name}"),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"x": {"type": "string"}},
            "required": ["x"]
        }),
        constrained_sampling: None,
    }
}

fn inline_tools_context_39() -> opi_ai::transcript::Context {
    opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::System(opi_ai::transcript::SystemMessage {
                content: "Base prompt.".into(),
                sections: vec![],
                tools_added: vec![tool_def_39("initial_tool")],
                tools_removed: vec![],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 1,
            }),
            Message::System(opi_ai::transcript::SystemMessage {
                content: "Update.".into(),
                sections: vec![],
                tools_added: vec![
                    tool_def_39("added_tool"),
                    ToolDef {
                        description: "redefined initial tool".into(),
                        ..tool_def_39("initial_tool")
                    },
                ],
                tools_removed: vec![opi_ai::transcript::ToolReference {
                    name: "removed_tool".into(),
                }],
                timestamp_ms: 2,
            }),
        ],
        tools: vec![],
    }
}

#[test]
fn inline_tools_placeholder_anchors_initial_tools_and_declares_the_beta() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, true, true));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    let betas = body["betas"].as_array().expect("inline-tools beta");
    assert!(betas.iter().any(|beta| beta == "inline-tools-2026-09-15"));

    let tools = body["tools"].as_array().expect("tools list");
    assert_eq!(tools.len(), 2, "initial tools plus the placeholder");
    assert_eq!(tools[0]["name"], "initial_tool");
    assert_eq!(tools[1]["name"], "__pi_deferred_placeholder__");
    assert_eq!(tools[1]["defer_loading"], true);
}

#[test]
fn inline_tools_by_value_additions_and_same_name_redefinitions() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, true, true));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    // The update system message carries by-value tool_addition blocks (the
    // redefinition of initial_tool arrives as a fresh definition, no removal)
    // plus a tool_removal reference for the genuinely removed name.
    let system = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|message| message["role"] == "system")
        .expect("mid-convo system message stays native");
    let blocks = system["content"].as_array().unwrap();
    let additions: Vec<&serde_json::Value> = blocks
        .iter()
        .filter(|block| block["type"] == "tool_addition")
        .collect();
    assert_eq!(additions.len(), 2, "added_tool and the redefinition");
    assert_eq!(additions[0]["tool"]["definition"]["name"], "added_tool");
    assert_eq!(
        additions[1]["tool"]["definition"]["name"], "initial_tool",
        "same-name redefinition is a by-value addition"
    );
    assert_eq!(
        additions[1]["tool"]["definition"]["description"],
        "redefined initial tool"
    );
    let removals: Vec<&serde_json::Value> = blocks
        .iter()
        .filter(|block| block["type"] == "tool_removal")
        .collect();
    assert_eq!(removals.len(), 1, "redefined names are not removed");
    assert_eq!(removals[0]["tool"]["type"], "tool_reference");
    assert_eq!(removals[0]["tool"]["name"], "removed_tool");
}

#[test]
fn inline_tools_stay_off_without_the_compat_flags() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = provider_39(&model, native_options_39());
    let mut request = request_39(ThinkingConfig::default());
    request.context =
        opi_ai::transcript::normalize_context(inline_tools_context_39()).expect("valid");
    let body = body_39(&provider, &request);

    assert!(
        body.get("betas").is_none(),
        "no inline-tools beta without the compat flags"
    );
    let tools = body["tools"].as_array().expect("collapsed current list");
    assert_eq!(
        tools.len(),
        2,
        "current tool set: added_tool + redefined initial"
    );
    assert!(
        !tools
            .iter()
            .any(|tool| tool["name"] == "__pi_deferred_placeholder__")
    );
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["role"] != "system"),
        "system changes collapse without native mid-convo support"
    );
}

#[test]
fn error_tool_result_images_follow_all_results_in_one_user_message() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let png = opi_ai::message::ImageSource::Base64 {
        data: "aWNn".into(),
    };
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(opi_ai::message::AssistantMessage {
            projection: None,
            content: vec![
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_1".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_2".into(),
                        name: "broken".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
            ],
            api: opi_ai::ApiKind::Anthropic,
            deferred: None,
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            response_model: None,
            wire: Some(WireApi::AnthropicMessages),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "ok".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 1,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_2".into(),
            tool_name: "broken".into(),
            content: vec![
                opi_ai::message::OutputContent::Text {
                    text: "boom ".into(),
                },
                opi_ai::message::OutputContent::Image {
                    source: png,
                    media_type: opi_ai::message::MediaType::Png,
                },
            ],
            details: None,
            is_error: true,
            not_executed: None,
            truncated: false,
            timestamp_ms: 2,
        }),
    ];
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid");
    let body = body_39(&provider, &request);

    // One user message carries both tool results, then the relocated error
    // images (caption first) in the same message (pinned #10794 shape).
    let user_messages: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "user")
        .collect();
    let tool_turn = user_messages
        .iter()
        .find(|message| {
            message["content"]
                .as_array()
                .is_some_and(|blocks| blocks.iter().any(|block| block["type"] == "tool_result"))
        })
        .expect("tool turn");
    let blocks = tool_turn["content"].as_array().unwrap();
    assert_eq!(
        blocks.len(),
        4,
        "two tool results, a caption, and the image in one user message"
    );
    assert_eq!(blocks[0]["tool_use_id"], "call_1");
    assert_eq!(blocks[1]["tool_use_id"], "call_2");
    assert_eq!(blocks[1]["is_error"], true);
    assert_eq!(
        blocks[1]["content"],
        serde_json::json!([
            {"type": "text", "text": "boom "},
            {"type": "text", "text": "(image attached after the tool results)"}
        ]),
        "error text is preserved and images become a placeholder"
    );
    assert_eq!(blocks[2]["type"], "text");
    assert_eq!(
        blocks[2]["text"],
        "Images from the error result of tool call call_2 (broken):"
    );
    assert_eq!(blocks[3]["type"], "image");
}

#[test]
fn consecutive_success_tool_results_group_into_one_user_message() {
    let model = model_39("claude-sonnet-4-5", compat_39(false, false, false));
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let messages = vec![
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "q".into() }],
            timestamp_ms: 0,
        }),
        Message::Assistant(opi_ai::message::AssistantMessage {
            projection: None,
            content: vec![
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_1".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
                opi_ai::message::AssistantContent::ToolCall {
                    tool_call: opi_ai::message::ToolCall {
                        id: "call_2".into(),
                        name: "probe".into(),
                        arguments: "{}".into(),
                        arguments_error: None,
                        signature: None,
                    },
                },
            ],
            api: opi_ai::ApiKind::Anthropic,
            deferred: None,
            provider: "anthropic".into(),
            model: "claude-sonnet-4-5".into(),
            response_model: None,
            wire: Some(WireApi::AnthropicMessages),
            response_id: None,
            usage: Default::default(),
            stop_reason: StopReason::ToolUse,
            error_message: None,
            timestamp_ms: 0,
            diagnostics: vec![],
            duration_ms: None,
            raw_stop_reason: None,
            provider_thinking_level: None,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_1".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "a".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 1,
        }),
        Message::ToolResult(opi_ai::message::ToolResultMessage {
            tool_call_id: "call_2".into(),
            tool_name: "probe".into(),
            content: vec![opi_ai::message::OutputContent::Text { text: "b".into() }],
            details: None,
            is_error: false,
            not_executed: None,
            truncated: false,
            timestamp_ms: 2,
        }),
    ];
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid");
    let body = body_39(&provider, &request);

    let user_messages: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "user")
        .collect();
    let tool_blocks: Vec<&serde_json::Value> = user_messages
        .iter()
        .flat_map(|message| message["content"].as_array().into_iter().flatten())
        .filter(|block| block["type"] == "tool_result")
        .collect();
    assert_eq!(
        user_messages
            .iter()
            .filter(|message| message["content"]
                .as_array()
                .is_some_and(|blocks| blocks.iter().any(|b| b["type"] == "tool_result")))
            .count(),
        1,
        "consecutive tool results share one user message"
    );
    assert_eq!(tool_blocks.len(), 2);
}

#[tokio::test]
async fn simple_numeric_claude_adds_thinking_before_model_and_context_caps() {
    #[derive(Debug, Clone, Copy)]
    enum Mode {
        Numeric,
        Disabled,
        Adaptive,
        Native,
    }

    for (mode, model_cap, context_window, expected_limit, expected_budget) in [
        (Mode::Numeric, 64_000, 200_000, 6_144, Some(2_048)),
        (Mode::Numeric, 5_000, 200_000, 5_000, Some(2_048)),
        (Mode::Numeric, 64_000, 6_000, 1_904, Some(880)),
        (Mode::Numeric, 1, 200_000, 1, Some(1_024)),
        (Mode::Numeric, 1_024, 200_000, 1_024, Some(1_024)),
        (Mode::Numeric, 2_000, 200_000, 2_000, Some(976)),
        (Mode::Disabled, 64_000, 200_000, 4_096, None),
        (Mode::Adaptive, 64_000, 200_000, 4_096, None),
        (Mode::Native, 64_000, 200_000, 4_096, Some(2_048)),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut model = model_39(
            "claude-sonnet-4-5",
            compat_39(matches!(mode, Mode::Adaptive), false, false),
        )
        .with_base_url(server.uri());
        model.capabilities.context_window = context_window;
        model.capabilities.max_output_tokens = model_cap;
        let provider = std::sync::Arc::new(AnthropicProvider::for_route(
            "anthropic".into(),
            vec![model.clone()],
            None,
            Default::default(),
            std::sync::Arc::new(HttpClient::new()),
            false,
        ));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("test-key"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut request = request_39(ThinkingConfig {
            enabled: matches!(mode, Mode::Numeric | Mode::Adaptive),
            budget_tokens: matches!(mode, Mode::Numeric).then_some(2_048),
            level: opi_ai::ThinkingLevel::Low,
        });
        request.context =
            opi_ai::transcript::normalize_context(opi_ai::transcript::Context::default()).unwrap();
        let record = opi_ai::models::ChatModelRecord::new("anthropic", model);
        let message = if matches!(mode, Mode::Native) {
            let mut options = AnthropicMessagesOptions::default();
            options.thinking_enabled = Some(true);
            options.thinking_budget_tokens = Some(2_048);
            let options = handle.options(options);
            models.complete(record, request, options).await
        } else {
            models
                .complete_simple(record, request, SimpleOptions::default())
                .await
        }
        .expect("one request completes");
        assert_eq!(message.stop_reason, StopReason::Stop);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&sent[0].body).unwrap();
        assert_eq!(
            body["max_tokens"], expected_limit,
            "{mode:?}, cap={model_cap}"
        );
        assert_eq!(
            body["thinking"].get("budget_tokens"),
            expected_budget
                .map(|budget| serde_json::json!(budget))
                .as_ref(),
            "{mode:?}, cap={model_cap}, context={context_window}"
        );
        if matches!(mode, Mode::Adaptive) {
            assert_eq!(body["thinking"]["type"], "adaptive");
        } else if matches!(mode, Mode::Disabled) {
            assert!(body.get("thinking").is_none());
        } else {
            assert_eq!(body["thinking"]["type"], "enabled");
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn native_options_from_simple_map_neutral_tool_choice_only() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "tool_choice": {"type": "none"},
            "thinking": {
                "type": "enabled",
                "budget_tokens": 8192,
                "display": "summarized"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    // The neutral entry maps only the tool choice; the neutral level-budget
    // policy (8192 for medium) stays distinct from the native no-budget
    // default 1024.
    let mut request = request_39(ThinkingConfig {
        enabled: true,
        budget_tokens: None,
        level: opi_ai::ThinkingLevel::Medium,
    });
    // No explicit output cap: the compiler uses the model cap, so the
    // medium level budget (8192) survives the answer-room clamp.
    request.max_tokens = None;
    let message = models
        .complete_simple(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            SimpleOptions {
                tool_choice: Some(opi_ai::api::contract::SimpleToolChoice::None),
                ..Default::default()
            },
        )
        .await
        .expect("simple entry completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

#[tokio::test]
async fn anthropic_direct_adapter_native_lifecycle_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(body_partial_json(serde_json::json!({
            "thinking": {
                "type": "enabled",
                "budget_tokens": 1024,
                "display": "summarized"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let mut options = native_options_39();
    options.thinking_enabled = Some(true);
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let options = handle.options(options);
    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request_39(ThinkingConfig::default()),
            options,
        )
        .await
        .expect("native lifecycle completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

async fn native_defaults_body(thinking_enabled: bool) -> serde_json::Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string(text_fixture()))
        .expect(1)
        .mount(&server)
        .await;
    let mut model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    model.capabilities.max_output_tokens = 12_345;
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("offline-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = request_39(ThinkingConfig::default());
    request.max_tokens = None;
    request.temperature = Some(0.3);
    let mut options = native_options_39();
    options.thinking_enabled = Some(thinking_enabled);
    models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            handle.options(options),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    server.verify().await;
    serde_json::from_slice(&sent[0].body).unwrap()
}

#[tokio::test]
async fn native_absent_max_tokens_uses_the_selected_model_cap() {
    let body = native_defaults_body(false).await;
    assert_eq!(body["max_tokens"], 12_345);
    assert_eq!(body["temperature"], serde_json::json!(0.3));
}

#[tokio::test]
async fn native_enabled_thinking_suppresses_temperature() {
    let body = native_defaults_body(true).await;
    assert_eq!(body["thinking"]["budget_tokens"], 1024);
    assert!(body.get("temperature").is_none());
}

#[test]
fn anthropic_direct_adapter_rejects_wrong_wire_records() {
    let wrong = ModelInfo::new(
        "gpt-4o",
        "GPT-4o",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let provider = std::sync::Arc::new(AnthropicProvider::new(None));
    let error = AnthropicMessagesChatApi::new(provider, wrong).expect_err("wrong wire");
    assert!(matches!(error, ProviderError::Config { .. }));
}

#[tokio::test]
async fn anthropic_setup_failure_before_activation_is_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(
            r#"{"type":"error","error":{"type":"authentication_error","message":"bad key"}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let error = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request_39(ThinkingConfig::default()),
            handle.options(native_options_39()),
        )
        .await
        .expect_err("401 setup failure is typed");
    assert!(
        !format!("{error:?}").contains("bad key"),
        "owned failure detail"
    );
}

#[tokio::test]
async fn anthropic_direct_adapter_replays_same_identity_signed_thinking() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "q"}]},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "prior", "signature": "sig-1"}
                ]},
                {"role": "user", "content": [{"type": "text", "text": "next"}]}
            ]
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = std::sync::Arc::new(AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model.clone()],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    ));
    let api = AnthropicMessagesChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register");

    let history = Message::Assistant(opi_ai::message::AssistantMessage {
        projection: None,
        content: vec![opi_ai::message::AssistantContent::Thinking {
            thinking: "prior".into(),
            signature: Some("sig-1".into()),
        }],
        api: opi_ai::ApiKind::Anthropic,
        deferred: None,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5".into(),
        response_model: None,
        wire: Some(WireApi::AnthropicMessages),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
    });
    let mut request = request_39(ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "q".into() }],
                timestamp_ms: 0,
            }),
            history,
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "next".into(),
                }],
                timestamp_ms: 1,
            }),
        ],
        tools: vec![],
    })
    .expect("valid");

    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("anthropic", model),
            request,
            handle.options(native_options_39()),
        )
        .await
        .expect("replay call completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
}

#[cfg(feature = "telemetry-tracing")]
#[tokio::test]
async fn anthropic_telemetry_carrier_instruments_the_producer() {
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tracing_core::span::{Attributes, Id, Record};
    use tracing_core::{Dispatch, Event, Metadata, Subscriber, dispatcher};

    #[derive(Default)]
    struct Recording {
        next: u64,
        spans: Vec<(&'static str, Option<u64>)>,
        entered: HashMap<std::thread::ThreadId, Vec<Id>>,
    }
    struct Recorder(std::sync::Arc<Mutex<Recording>>);

    impl Subscriber for Recorder {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, attributes: &Attributes<'_>) -> Id {
            let mut state = self.0.lock().unwrap();
            state.next += 1;
            let id = Id::from_u64(state.next);
            let parent = attributes.parent().cloned().or_else(|| {
                attributes
                    .is_contextual()
                    .then(|| {
                        state
                            .entered
                            .get(&std::thread::current().id())
                            .and_then(|stack| stack.last())
                            .cloned()
                    })
                    .flatten()
            });
            state
                .spans
                .push((attributes.metadata().name(), parent.map(|id| id.into_u64())));
            id
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, id: &Id) {
            self.0
                .lock()
                .unwrap()
                .entered
                .entry(std::thread::current().id())
                .or_default()
                .push(id.clone());
        }
        fn exit(&self, id: &Id) {
            self.0
                .lock()
                .unwrap()
                .entered
                .entry(std::thread::current().id())
                .or_default()
                .pop();
            let _ = id;
        }
    }

    let recorder = std::sync::Arc::new(Mutex::new(Recording::default()));
    let source = Dispatch::new(Recorder(recorder.clone()));
    let parent = dispatcher::with_default(&source, || tracing::info_span!("source_parent"));
    let context = TelemetryContext::from_span(parent);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_fixture())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model =
        model_39("claude-sonnet-4-5", compat_39(false, false, false)).with_base_url(server.uri());
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![model],
        None,
        Default::default(),
        std::sync::Arc::new(HttpClient::new()),
        false,
    );
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = seen.clone();
    let mut request = request_39(ThinkingConfig::default());
    request.telemetry_context = Some(context);
    request.callbacks = RequestCallbacks::new().with_async_on_payload(move |_| {
        let counter = counter.clone();
        async move {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tracing::info_span!("payload_probe").in_scope(|| ());
            Ok(None)
        }
    });

    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    while let Some(result) = stream.next().await {
        result.expect("stream completes under the carrier");
    }
    assert_eq!(
        seen.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "payload callback ran exactly once"
    );

    // The producer (and its payload callback) ran under the frozen source
    // dispatch: the probe span is recorded on that carrier.
    let state = recorder.lock().unwrap();
    assert!(
        state.spans.iter().any(|(name, _)| *name == "payload_probe"),
        "producer telemetry flows to the frozen carrier: {:?}",
        state.spans
    );
}

#[tokio::test]
async fn direct_unknown_model_without_max_tokens_fails_typed_before_send() {
    let server = MockServer::start().await;
    let provider = AnthropicProvider::new(Some(server.uri()));
    let mut request = request_39(ThinkingConfig::default());
    request.model = "anthropic:unknown-model".into();
    request.max_tokens = None;
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let error = stream
        .next()
        .await
        .expect("preflight result")
        .expect_err("unknown output limit");
    let ProviderError::StreamFailure(failure) = &error else {
        panic!("accepted projection retains its failure carrier: {error:?}")
    };
    assert!(failure.message().projection.is_some());
    let cause = failure.cause();
    assert!(matches!(cause, ProviderError::Config(_)));
    assert!(stream.next().await.is_none());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn oauth_key_projects_identity_inline_tools_and_capture_reload_replay() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::message::{AssistantContent, AssistantMessage, OutputContent, ToolResultMessage};
    use std::sync::{Arc, Mutex};
    let server = MockServer::start().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let reply = [
        serde_json::json!({"type":"message_start","message":{"id":"oauth-fixture","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":0}}}),
        serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"call-read","name":"Read","input":{}}}),
        serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"x\":\"fixture\"}"}}),
        serde_json::json!({"type":"content_block_stop","index":0}),
        serde_json::json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":1}}),
        serde_json::json!({"type":"message_stop"}),
    ].into_iter().map(|event| format!("event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap())).collect::<String>();
    let count = calls.clone();
    Mock::given(method("POST")).respond_with(move |_: &wiremock::Request| {
        if count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"type":"error","error":{"type":"invalid_request_error","message":"Schemas contains too many parameters with union types (fixture)"}}))
        } else { ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(reply.clone()) }
    }).mount(&server).await;
    let mut compat = compat_39(false, true, true);
    compat.supports_strict_tools = true;
    let model = model_39("claude-sonnet-4-5", compat).with_base_url(server.uri());
    let api = AnthropicMessagesChatApi::new(Arc::new(profile_39(&model)), model.clone()).unwrap();
    let mut models = Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                "sk-ant-oat-fixture-only".into(),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let strict = |name: &str, preference| ToolDef {
        constrained_sampling: Some(ConstrainedSampling::JsonSchema { strict: preference }),
        ..tool_def_39(name)
    };
    let context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages: vec![
            Message::System(opi_ai::transcript::SystemMessage {
                content: "caller prompt".into(),
                sections: vec![],
                tools_added: vec![
                    strict("read", StrictPreference::Prefer),
                    tool_def_39("write"),
                ],
                tools_removed: vec![],
                timestamp_ms: 0,
            }),
            Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "inspect".into(),
                }],
                timestamp_ms: 1,
            }),
            Message::System(opi_ai::transcript::SystemMessage {
                content: "later prompt".into(),
                sections: vec![],
                tools_added: vec![
                    strict("read", StrictPreference::Prefer),
                    strict("bash", StrictPreference::Require),
                ],
                tools_removed: vec![opi_ai::transcript::ToolReference {
                    name: "write".into(),
                }],
                timestamp_ms: 2,
            }),
        ],
        tools: vec![],
    })
    .unwrap();
    let raw_names = Arc::new(Mutex::new(Vec::new()));
    let recorder = raw_names.clone();
    let mut request = request_39(ThinkingConfig::default());
    request.context = context.clone();
    request.callbacks = RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
        if let Some(name) = event["content_block"]["name"].as_str() {
            recorder.lock().unwrap().push(name.to_owned());
        }
        Ok(())
    }));
    let result = models
        .complete(
            opi_ai::ChatModelRecord::new("anthropic", model.clone()),
            request,
            handle.options(native_options_39()),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 2, "one lawful strict variant");
    assert_eq!(
        sent[0]
            .headers
            .get("authorization")
            .map(|value| value.to_str().unwrap()),
        Some("Bearer sk-ant-oat-fixture-only")
    );
    assert!(!sent[0].headers.contains_key("x-api-key"));
    assert_eq!(sent[0].headers["user-agent"], "claude-cli/2.1.280");
    assert_eq!(sent[0].headers["x-app"], "cli");
    for (index, sent) in sent.iter().enumerate() {
        let body: serde_json::Value = serde_json::from_slice(&sent.body).unwrap();
        assert_eq!(
            body["system"][0]["text"],
            "You are Claude Code, Anthropic's official CLI for Claude."
        );
        assert_eq!(body["system"][1]["text"], "caller prompt");
        assert_eq!(body["tools"][0]["name"], "Read");
        assert_eq!(
            body["tools"][0]
                .get("strict")
                .and_then(|value| value.as_bool()),
            (index == 0).then_some(true)
        );
        let blocks: Vec<_> = body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|message| message["content"].as_array().into_iter().flatten())
            .collect();
        assert!(
            blocks
                .iter()
                .any(|block| block["type"] == "tool_removal" && block["tool"]["name"] == "Write")
        );
        assert!(blocks.iter().any(|block| block["type"] == "tool_addition"
            && block["tool"]["definition"]["name"] == "Bash"
            && block["tool"]["definition"]["strict"] == true));
    }
    assert_eq!(*raw_names.lock().unwrap(), vec!["Read"]);
    let tool_call = result
        .content
        .iter()
        .find_map(|block| match block {
            AssistantContent::ToolCall { tool_call } => Some(tool_call),
            _ => None,
        })
        .unwrap();
    assert_eq!(tool_call.name, "read");
    let reloaded: AssistantMessage =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
    let mut messages = context.messages().to_vec();
    messages.push(Message::Assistant(reloaded));
    messages.push(Message::ToolResult(ToolResultMessage {
        tool_call_id: "call-read".into(),
        tool_name: "read".into(),
        content: vec![OutputContent::Text {
            text: "done".into(),
        }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 3,
    }));
    let mut next = request_39(ThinkingConfig::default());
    next.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .unwrap();
    models
        .complete(
            opi_ai::ChatModelRecord::new("anthropic", model),
            next,
            handle.options(native_options_39()),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[2].body).unwrap();
    assert!(
        body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|message| message["content"].as_array().into_iter().flatten())
            .any(|block| block["type"] == "tool_use" && block["name"] == "Read")
    );
}

mod federation_fixtures {
    use super::*;
    use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
    use opi_ai::credential::BoxAuthFuture;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct FederationAuth(ResolvedAuth);
    impl AuthResolver for FederationAuth {
        fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(self.0.clone()) })
        }
    }

    fn prepared(file: &std::path::Path) -> ResolvedAuth {
        let mut auth = opi_ai::test_support::resolved_auth();
        auth.scheme = AuthScheme::AnthropicFederation;
        auth.secret = "".into();
        auth.env = [
            ("ANTHROPIC_FEDERATION_RULE_ID", "rule-fixture".to_owned()),
            ("ANTHROPIC_ORGANIZATION_ID", "org-fixture".to_owned()),
            (
                "ANTHROPIC_IDENTITY_TOKEN_FILE",
                file.to_string_lossy().into_owned(),
            ),
            ("ANTHROPIC_SERVICE_ACCOUNT_ID", "service-fixture".to_owned()),
            ("ANTHROPIC_WORKSPACE_ID", "workspace-fixture".to_owned()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.into()))
        .collect();
        auth
    }

    fn setup(server: &MockServer, auth: ResolvedAuth) -> (Models, opi_ai::ChatModelRecord) {
        let model = model_39("claude-sonnet-4-5", compat_39(false, false, false))
            .with_base_url(server.uri());
        let provider = Arc::new(profile_39(&model));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(api),
                Arc::new(FederationAuth(auth)),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        (models, opi_ai::ChatModelRecord::new("anthropic", model))
    }

    fn success() -> ResponseTemplate {
        let body = [
            serde_json::json!({"type":"message_start","message":{"id":"federated","model":"claude-sonnet-4-5","usage":{"input_tokens":1,"output_tokens":0}}}),
            serde_json::json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":"ok"}}),
            serde_json::json!({"type":"content_block_stop","index":0}),
            serde_json::json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":1}}),
            serde_json::json!({"type":"message_stop"}),
        ].into_iter().map(|e| format!("event: {}\ndata: {e}\n\n",e["type"].as_str().unwrap())).collect::<String>();
        ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(body)
    }

    #[tokio::test]
    async fn federation_session_affinity_reaches_generation_only_with_disabled_cache() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity.jwt");
        std::fs::write(&file, "fixture-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let mut model = record.info().clone();
        let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
            unreachable!()
        };
        compat.send_session_affinity_headers = Some(true);
        let record = opi_ai::ChatModelRecord::new("anthropic", model);
        let mut request = request_39(Default::default());
        request.session_id = Some("federated-session".into());
        request.cache_retention = CacheRetention::Disabled;
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token":"federated-access", "expires_in":3600
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer federated-access"))
            .and(header("x-session-affinity", "federated-session"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        let outcome = models
            .complete_simple(record, request, Default::default())
            .await
            .unwrap();
        assert_eq!(outcome.stop_reason, StopReason::Stop);
        assert_eq!(outcome.usage.input_tokens, 1);
        assert_eq!(outcome.usage.output_tokens, 1);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
        let token = requests
            .iter()
            .find(|request| request.url.path() == "/v1/oauth/token")
            .unwrap();
        assert!(!token.headers.contains_key("x-session-affinity"));
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_real_models_exchanges_caches_and_keeps_non_subscription_wire() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity.jwt");
        let (models, record) = setup(&server, prepared(&file));
        let call = models.complete_simple(
            record.clone(),
            request_39(Default::default()),
            SimpleOptions::default(),
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        std::fs::write(&file, "  fixture-identity-jwt\n").unwrap();
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(wiremock::matchers::headers("anthropic-beta", vec!["oauth-2025-04-20", "oidc-federation-2026-04-01"]))
            .and(body_partial_json(serde_json::json!({"grant_type":"urn:ietf:params:oauth:grant-type:jwt-bearer","assertion":"fixture-identity-jwt","federation_rule_id":"rule-fixture","organization_id":"org-fixture","service_account_id":"service-fixture","workspace_id":"workspace-fixture"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"federated-access","expires_in":3600,"token_type":"Bearer"})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer federated-access"))
            .respond_with(success())
            .expect(2)
            .mount(&server)
            .await;
        call.await.expect("real federation exchange and generation");
        std::fs::remove_file(&file).unwrap();
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("cached token needs no identity file");
        let requests = server.received_requests().await.unwrap();
        let generation = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/messages")
            .collect::<Vec<_>>();
        assert_eq!(generation.len(), 2);
        for request in generation {
            assert_eq!(request.headers["anthropic-beta"], "oauth-2025-04-20");
            assert!(!request.headers.contains_key("x-api-key"));
            assert!(!request.headers.contains_key("x-app"));
            let body = String::from_utf8(request.body.clone()).unwrap();
            assert!(!body.contains("Claude Code"));
            assert!(!body.contains("fixture-identity-jwt"));
        }
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_401_invalidates_next_call_without_hidden_generation_retry() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity.jwt");
        std::fs::write(&file, "first-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let exchanges = Arc::new(AtomicUsize::new(0));
        let count = exchanges.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":format!("access-{n}"),"expires_in":3600}),
                )
            })
            .expect(2)
            .mount(&server)
            .await;
        let sends = Arc::new(AtomicUsize::new(0));
        let count = sends.clone();
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(move |_: &wiremock::Request| {
                if count.fetch_add(1, Ordering::SeqCst) == 1 {
                    ResponseTemplate::new(401)
                } else {
                    success()
                }
            })
            .expect(3)
            .mount(&server)
            .await;
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let failure = models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(
            failure.cause().category(),
            opi_ai::provider::ProviderErrorCategory::Auth
        );
        assert_eq!(sends.load(Ordering::SeqCst), 2);
        assert_eq!(exchanges.load(Ordering::SeqCst), 1);
        std::fs::write(&file, "second-identity").unwrap();
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .unwrap();
        let requests = server.received_requests().await.unwrap();
        let auths = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/messages")
            .map(|r| r.headers["authorization"].to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            auths,
            vec!["Bearer access-1", "Bearer access-1", "Bearer access-2"]
        );
        let tokens = requests
            .iter()
            .filter(|r| r.url.path() == "/v1/oauth/token")
            .map(|r| {
                serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["assertion"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(tokens, vec!["first-identity", "second-identity"]);
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_expiry_and_complete_model_base_select_fresh_exchange() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "rotating-1").unwrap();
        let (models, record) = setup(&first, prepared(&file));
        for server in [&first, &second] {
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"short-token","expires_in":0}),
                ))
                .mount(server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .mount(server)
                .await;
        }
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        std::fs::write(&file, "rotating-2").unwrap();
        models
            .complete_simple(
                record.clone(),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let mut changed = record.info().clone();
        changed.base_url = Some(second.uri());
        models
            .complete_simple(
                opi_ai::ChatModelRecord::new("anthropic", changed),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap();
        let sent = first.received_requests().await.unwrap();
        let assertions = sent
            .iter()
            .filter(|r| r.url.path() == "/v1/oauth/token")
            .map(|r| {
                serde_json::from_slice::<serde_json::Value>(&r.body).unwrap()["assertion"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(assertions, vec!["rotating-1", "rotating-2"]);
        let second = second.received_requests().await.unwrap();
        assert_eq!(
            second
                .iter()
                .filter(|r| r.url.path() == "/v1/oauth/token")
                .count(),
            1
        );
        assert_eq!(
            second
                .iter()
                .filter(|r| r.url.path() == "/v1/messages")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn federation_callback_failure_and_unsafe_base_do_no_auth_io() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let (models, record) = setup(&server, prepared(&missing));
        let mut request = request_39(Default::default());
        request.callbacks = opi_ai::RequestCallbacks::new().with_on_payload(Arc::new(|_| {
            Err(opi_ai::CallbackError::new("fixture callback failed"))
        }));
        let failure = models
            .complete_simple(record.clone(), request, Default::default())
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
        let mut unsafe_model = record.info().clone();
        unsafe_model.base_url = Some("http://unsafe-federation.invalid".into());
        let failure = models
            .complete_simple(
                opi_ai::ChatModelRecord::new("anthropic", unsafe_model),
                request_39(Default::default()),
                Default::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn federation_failed_exchange_never_generates_or_exposes_identity_material() {
        for (status, response) in [
            (
                401,
                serde_json::json!({"assertion":"identity-secret-canary","access_token":"access-secret-canary"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"access-secret-canary","expires_in":"not-finite"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"access-secret-canary","expires_in":3600,"token_type":"Mac"}),
            ),
            (
                200,
                serde_json::json!({"access_token":"","expires_in":3600}),
            ),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, "identity-secret-canary").unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .insert_header("request-id", "federation-request-fixture")
                        .set_body_json(response),
                )
                .expect(1)
                .mount(&server)
                .await;
            let failure = models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await
                .unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::AuthFailed(_)));
            let diagnostic = format!("{}", failure.cause());
            assert!(diagnostic.contains(&status.to_string()), "{diagnostic}");
            assert!(
                diagnostic.contains("federation-request-fixture"),
                "{diagnostic}"
            );
            for text in [format!("{}", failure.cause()), format!("{failure:?}")] {
                assert!(!text.contains("identity-secret-canary"));
                assert!(!text.contains("access-secret-canary"));
            }
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_wait_can_be_cancelled_without_generation_send() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let exchanged = Arc::new(tokio::sync::Notify::new());
        let seen = exchanged.clone();
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(move |_: &wiremock::Request| {
                seen.notify_one();
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_secs(60))
                    .set_body_json(serde_json::json!({"access_token":"delayed","expires_in":3600}))
            })
            .expect(1)
            .mount(&server)
            .await;
        let request = request_39(Default::default());
        let cancel = request.cancel.clone();
        let call = models.complete_simple(record, request, Default::default());
        let caller = tokio::spawn(call);
        tokio::time::timeout(std::time::Duration::from_secs(3), exchanged.notified())
            .await
            .unwrap();
        cancel.cancel();
        let failure = tokio::time::timeout(std::time::Duration::from_secs(3), caller)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Cancelled));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn federation_builtin_env_only_credential_selects_then_generates_without_early_file_io() {
        use opi_ai::credential::CredentialStore;
        struct EmptyContext;
        impl opi_ai::auth::AuthContext for EmptyContext {
            fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
                Ok(None)
            }
            fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
                panic!("configuration checks must not inspect federation identity file")
            }
        }
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-created-until-generation");
        let env = [
            ("ANTHROPIC_FEDERATION_RULE_ID", "stored-rule".to_owned()),
            ("ANTHROPIC_ORGANIZATION_ID", "stored-org".to_owned()),
            (
                "ANTHROPIC_IDENTITY_TOKEN_FILE",
                file.to_string_lossy().into_owned(),
            ),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.into()))
        .collect();
        let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());
        store
            .write(
                "anthropic",
                &opi_ai::Credential::ApiKey(opi_ai::ApiKeyCredential::new(None, env)),
            )
            .await
            .unwrap();
        let mut models = Models::new();
        models.set_credential_store(store);
        let mut overrides = opi_ai::providers::collection::BuiltinOverrides::default();
        overrides.base_urls.insert("anthropic".into(), server.uri());
        opi_ai::providers::collection::register_builtin(
            &mut models,
            Arc::new(EmptyContext),
            Arc::new(HttpClient::new()),
            &overrides,
        )
        .unwrap();
        let available = models.get_available(Some("anthropic")).await.unwrap();
        assert!(!available.is_empty());
        let auth = models
            .get_auth("anthropic", Default::default())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(auth.scheme, AuthScheme::AnthropicFederation);
        assert!(!file.exists());
        assert!(server.received_requests().await.unwrap().is_empty());
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(body_partial_json(serde_json::json!({"assertion":"stored-identity","federation_rule_id":"stored-rule","organization_id":"stored-org"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"access_token":"stored-fed-token","expires_in":3600})))
            .expect(1).mount(&server).await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer stored-fed-token"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        let (provider, info) = available.into_iter().next().unwrap();
        let call = models.complete_simple(
            opi_ai::ChatModelRecord::new(provider, info),
            request_39(Default::default()),
            Default::default(),
        );
        std::fs::write(&file, "stored-identity").unwrap();
        call.await.expect("real federation exchange and generation");
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_identity_file_empty_and_source_utf16_assertion_limit_are_observable() {
        for (identity, allowed) in [
            (" \n\t".to_owned(), false),
            ("x".repeat(16 * 1024 + 1), false),
            ("\u{1f600}".repeat(8192), true),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, identity).unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"accepted-token","expires_in":"3600"}),
                ))
                .expect(if allowed { 1 } else { 0 })
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .expect(if allowed { 1 } else { 0 })
                .mount(&server)
                .await;
            let result = models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await;
            if allowed {
                result.unwrap();
            } else {
                assert!(matches!(
                    result.unwrap_err().cause(),
                    ProviderError::AuthFailed(_)
                ));
            }
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_identity_uses_node_utf8_replacement_and_javascript_trim() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        let mut bytes = b"\xef\xbb\xbf \tidentity-".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b"-token\xef\xbb\xbf\n");
        std::fs::write(&file, bytes).unwrap();
        let (models, record) = setup(&server, prepared(&file));
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .and(body_partial_json(
                serde_json::json!({"assertion":"identity-\u{fffd}-token"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token":"decoded-token","expires_in":3600}),
            ))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("Node UTF8 and JS trim identity projection");
        server.verify().await;
    }

    #[tokio::test]
    async fn federation_expires_in_uses_source_javascript_number_conversion() {
        for expires in [
            serde_json::json!("0xE10"),
            serde_json::json!("0b111000010000"),
            serde_json::json!("0o7020"),
            serde_json::json!("\u{feff}3600\u{feff}"),
            serde_json::json!(null),
            serde_json::json!(false),
            serde_json::json!([]),
            serde_json::json!([null]),
            serde_json::json!(["3600"]),
            serde_json::json!([["3600"]]),
        ] {
            let server = MockServer::start().await;
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("identity");
            std::fs::write(&file, "numeric-fixture-identity").unwrap();
            let (models, record) = setup(&server, prepared(&file));
            Mock::given(method("POST"))
                .and(path("/v1/oauth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token":"numeric-token","expires_in":expires}),
                ))
                .expect(1)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path("/v1/messages"))
                .respond_with(success())
                .expect(1)
                .mount(&server)
                .await;
            models
                .complete_simple(record, request_39(Default::default()), Default::default())
                .await
                .expect("source Number(expires_in) remains finite");
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn federation_token_response_uses_source_utf8_decoder() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identity");
        std::fs::write(&file, "fixture-identity").unwrap();
        let (models, record) = setup(&server, prepared(&file));
        let mut response=b"\xef\xbb\xbf{\"access_token\":\"decoded-token\",\"expires_in\":3600,\"annotation\":\"".to_vec();
        response.push(0xff);
        response.extend_from_slice(b"\"}");
        Mock::given(method("POST"))
            .and(path("/v1/oauth/token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(response, "application/json"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("authorization", "Bearer decoded-token"))
            .respond_with(success())
            .expect(1)
            .mount(&server)
            .await;
        models
            .complete_simple(record, request_39(Default::default()), Default::default())
            .await
            .expect("source TextDecoder response projection");
        server.verify().await;
    }
}

mod anthropic_cache_source_compat {
    use super::*;
    use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation};
    use opi_ai::message::{AssistantMessage, ToolDef};
    use serde_json::{Value, json};
    use std::sync::Arc;

    fn fireworks() -> CatalogModel {
        Catalog::builtin()
            .unwrap()
            .model(
                "fireworks",
                CatalogOperation::Chat,
                "accounts/fireworks/models/deepseek-v4p1-flash",
            )
            .unwrap()
            .clone()
    }

    fn user() -> Message {
        Message::User(UserMessage {
            content: vec![InputContent::Text { text: "Hi".into() }],
            timestamp_ms: 0,
        })
    }

    fn request(tools: bool) -> Request {
        let mut request = request_39(Default::default());
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages: vec![user()],
            tools: if tools {
                vec![ToolDef {
                    name: "read".into(),
                    description: "Read".into(),
                    input_schema: json!({"type":"object", "properties":{}}),
                    constrained_sampling: None,
                }]
            } else {
                vec![]
            },
        })
        .unwrap();
        request.max_tokens = Some(4096);
        request.cache_retention = CacheRetention::Long;
        request.session_id = Some("session-complete-id".into());
        request
    }

    async fn capture(
        record: CatalogModel,
        mut request: Request,
        generic: Option<(bool, bool)>,
    ) -> (wiremock::Request, AssistantMessage) {
        let server = MockServer::start().await;
        let start = json!({"type":"message_start", "message": {
            "id":"received-id", "model": record.id,
            "usage":{"input_tokens":100, "output_tokens":0}
        }});
        let response = format!(
            "event: message_start\ndata: {start}\n\nevent: message_delta\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":10}}}}\n\nevent: message_stop\ndata: {{\"type\":\"message_stop\"}}\n\n"
        );
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut model = record.to_chat_model_info().unwrap();
        model.base_url = Some(server.uri());
        if let Some((cache, long)) = generic {
            model.capabilities.supports_cache_control = cache;
            model.capabilities.supports_long_cache_retention = long;
        }
        request.model = model.id.clone();
        let provider = Arc::new(AnthropicProvider::for_route(
            record.provider.clone(),
            vec![model.clone()],
            None,
            Default::default(),
            Arc::new(opi_ai::http::HttpClient::new()),
            false,
        ));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    "synthetic-key".into(),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let outcome = models
            .complete_simple(
                opi_ai::ChatModelRecord::new(record.provider, model),
                request,
                Default::default(),
            )
            .await
            .expect("local valid SSE fixture completes");
        assert_eq!(outcome.stop_reason, StopReason::Stop);
        assert_eq!(outcome.usage.input_tokens, 100);
        assert_eq!(outcome.usage.output_tokens, 10);
        let mut sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        server.verify().await;
        (sent.remove(0), outcome)
    }

    #[tokio::test]
    async fn tool_cache_false_suppresses_only_tool_marker() {
        let (sent, _) = capture(fireworks(), request(true), Some((true, true))).await;
        let body: Value = sent.body_json().unwrap();
        assert!(body["system"][0].get("cache_control").is_some());
        assert!(body["tools"][0].get("cache_control").is_none());
    }

    #[tokio::test]
    async fn long_cache_false_omits_ttl() {
        let (sent, _) = capture(fireworks(), request(false), Some((true, true))).await;
        let body: Value = sent.body_json().unwrap();
        assert_eq!(body["system"][0]["cache_control"]["type"], "ephemeral");
        assert!(body["system"][0]["cache_control"].get("ttl").is_none());
    }

    fn source(tool_cache: Option<bool>, long_cache: Option<bool>) -> CatalogModel {
        let mut record = fireworks();
        record.raw["compat"] = json!({});
        if let Some(enabled) = tool_cache {
            record.raw["compat"]["supportsCacheControlOnTools"] = json!(enabled);
        }
        if let Some(enabled) = long_cache {
            record.raw["compat"]["supportsLongCacheRetention"] = json!(enabled);
        }
        record
    }

    fn has_marker(value: &Value) -> bool {
        match value {
            Value::Object(object) => {
                object.contains_key("cache_control") || object.values().any(has_marker)
            }
            Value::Array(values) => values.iter().any(has_marker),
            _ => false,
        }
    }

    #[tokio::test]
    async fn tool_cache_and_long_ttl_follow_independent_flags_and_retention() {
        for tool_cache in [false, true] {
            for long_cache in [false, true] {
                for retention in [
                    CacheRetention::None,
                    CacheRetention::Short,
                    CacheRetention::Long,
                    CacheRetention::Disabled,
                ] {
                    let mut request = request(true);
                    request.cache_retention = retention;
                    let (sent, _) = capture(
                        source(Some(tool_cache), Some(long_cache)),
                        request,
                        Some((true, true)),
                    )
                    .await;
                    let body: Value = sent.body_json().unwrap();
                    let enabled = retention != CacheRetention::Disabled;
                    let ttl = enabled && long_cache && retention == CacheRetention::Long;
                    for content in [&body["system"][0], &body["messages"][0]["content"][0]] {
                        assert_eq!(content.get("cache_control").is_some(), enabled);
                        if enabled {
                            assert_eq!(content["cache_control"]["type"], "ephemeral");
                            assert_eq!(content["cache_control"].get("ttl").is_some(), ttl);
                            if ttl {
                                assert_eq!(content["cache_control"]["ttl"], "1h");
                            }
                        }
                    }
                    let tool = &body["tools"][0];
                    assert_eq!(tool.get("cache_control").is_some(), enabled && tool_cache);
                    assert_eq!(
                        tool["cache_control"].get("ttl").is_some(),
                        ttl && tool_cache
                    );
                    if !enabled {
                        assert!(!has_marker(&body));
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn plain_source_defaults_emit_long_markers_but_owned_generic_opt_out_suppresses_all() {
        let mut plain = source(None, None);
        plain.raw.as_object_mut().unwrap().remove("compat");
        let (sent, _) = capture(plain.clone(), request(true), None).await;
        let body: Value = sent.body_json().unwrap();
        for content in [
            &body["system"][0],
            &body["messages"][0]["content"][0],
            &body["tools"][0],
        ] {
            assert_eq!(
                content["cache_control"],
                json!({"type":"ephemeral", "ttl":"1h"})
            );
        }
        plain.raw["x-opi"] = json!({"cacheControl":false, "longCacheRetention":false});
        let (sent, _) = capture(plain, request(true), None).await;
        let body: Value = sent.body_json().unwrap();
        assert!(!has_marker(&body));
    }

    #[tokio::test]
    async fn generic_long_opt_out_and_source_long_false_preserve_ephemeral_history_markers() {
        for (long_cache, generic_long) in [(true, false), (false, true)] {
            let record = source(Some(false), Some(long_cache));
            let mut assistant = opi_ai::test_support::base_assistant();
            assistant.provider = record.provider.clone();
            assistant.model = record.id.clone();
            assistant.wire = Some(opi_ai::WireApi::AnthropicMessages);
            assistant.content = vec![AssistantContent::Text {
                text: "prior response".into(),
                signature: None,
            }];
            let mut request = request(false);
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: Some("system".into()),
                messages: vec![user(), Message::Assistant(assistant), user()],
                tools: vec![],
            })
            .unwrap();
            let (sent, _) = capture(record, request, Some((true, generic_long))).await;
            let body: Value = sent.body_json().unwrap();
            assert_eq!(body["messages"][1]["role"], "assistant");
            assert_eq!(body["messages"][1]["content"][0]["text"], "prior response");
            for content in [
                &body["system"][0],
                &body["messages"][1]["content"][0],
                &body["messages"][2]["content"][0],
            ] {
                assert_eq!(content["cache_control"], json!({"type":"ephemeral"}));
            }
        }
    }

    #[tokio::test]
    async fn inline_tools_mark_only_last_initial_definition_before_the_placeholder() {
        for tool_cache in [false, true] {
            let mut record = source(Some(tool_cache), Some(true));
            record.raw["compat"]["supportsMidConvoSystemMessages"] = json!(true);
            record.raw["compat"]["supportsMidConvoToolChanges"] = json!(true);
            let mut context = inline_tools_context_39();
            let Message::System(initial) = &mut context.messages[0] else {
                unreachable!()
            };
            initial.tools_added.push(tool_def_39("second_initial"));
            let mut request = request(false);
            request.context = opi_ai::transcript::normalize_context(context).unwrap();
            let (sent, _) = capture(record, request, Some((true, true))).await;
            let body: Value = sent.body_json().unwrap();
            let tools = body["tools"].as_array().unwrap();
            assert_eq!(tools.len(), 3);
            assert_eq!(tools[0]["name"], "initial_tool");
            assert!(tools[0].get("cache_control").is_none());
            assert_eq!(tools[1]["name"], "second_initial");
            assert_eq!(tools[1].get("cache_control").is_some(), tool_cache);
            if tool_cache {
                assert_eq!(
                    tools[1]["cache_control"],
                    json!({"type":"ephemeral", "ttl":"1h"})
                );
            }
            assert_eq!(tools[2]["name"], "__pi_deferred_placeholder__");
            assert!(tools[2].get("cache_control").is_none());
            let additions: Vec<_> = body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|message| message["content"].as_array().into_iter().flatten())
                .filter(|block| block["type"] == "tool_addition")
                .collect();
            assert_eq!(additions.len(), 2);
            for addition in additions {
                assert!(
                    addition["tool"]["definition"]
                        .get("cache_control")
                        .is_none()
                );
            }
        }
    }
}

mod server_fallback_source_compat {
    use super::*;
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::message::AssistantMessage;
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    fn source() -> ModelInfo {
        Catalog::builtin()
            .unwrap()
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .unwrap()
            .to_chat_model_info()
            .unwrap()
    }

    fn compat(model: &mut ModelInfo) -> &mut AnthropicMessagesCompat {
        let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
            unreachable!()
        };
        compat
    }

    struct Capture {
        body: Value,
        events: Vec<AssistantStreamEvent>,
        raw: Vec<Value>,
        outcome: Result<AssistantMessage, opi_ai::CallFailure>,
    }

    async fn invoke(mut model: ModelInfo, reported: Option<&str>, blocks: Vec<Value>) -> Capture {
        let server = MockServer::start().await;
        let mut message =
            json!({"id":"received-id", "usage":{"input_tokens":100,"output_tokens":0}});
        if let Some(reported) = reported {
            message["model"] = json!(reported);
        }
        let mut events = vec![json!({"type":"message_start","message":message})];
        events.extend(blocks);
        events.extend([
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}),
            json!({"type":"message_stop"}),
        ]);
        let response = events
            .into_iter()
            .map(|event| {
                format!(
                    "event: {}\ndata: {event}\n\n",
                    event["type"].as_str().unwrap()
                )
            })
            .collect::<String>();
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response),
            )
            .expect(1)
            .mount(&server)
            .await;
        model.base_url = Some(server.uri());
        model.capabilities.supports_cache_control = true;
        model.capabilities.supports_long_cache_retention = true;
        let mut input = request_39_for(&model.id, ThinkingConfig::default());
        input.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "Hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap();
        input.cache_retention = CacheRetention::Long;
        input.session_id = Some("session-complete-id".into());
        let raw = Arc::new(Mutex::new(Vec::new()));
        let observed = raw.clone();
        input.callbacks = opi_ai::RequestCallbacks::new().with_on_raw(Arc::new(move |event| {
            observed.lock().unwrap().push(event.clone());
            Ok(())
        }));
        let canonical = serde_json::to_vec(&input.context).unwrap();
        let provider = Arc::new(AnthropicProvider::for_route(
            "anthropic".into(),
            vec![model.clone()],
            None,
            Default::default(),
            Arc::new(HttpClient::new()),
            false,
        ));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    "synthetic-key".into(),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let mut call = models.stream_simple(
            opi_ai::ChatModelRecord::new("anthropic", model),
            input.clone(),
            Default::default(),
        );
        let mut events = Vec::new();
        while let Some(item) = call.next().await {
            if let Ok(event) = item {
                events.push(event);
            }
        }
        let outcome = call.result().await;
        if let Ok(message) = &outcome {
            assert_eq!(message.stop_reason, StopReason::Stop);
            assert_eq!(message.usage.input_tokens, 100);
            assert_eq!(message.usage.output_tokens, 10);
        }
        assert_eq!(serde_json::to_vec(&input.context).unwrap(), canonical);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), 1);
        server.verify().await;
        let raw = raw.lock().unwrap().clone();
        Capture {
            body: sent[0].body_json().unwrap(),
            events,
            raw,
            outcome,
        }
    }

    fn fallback(index: usize) -> Value {
        json!({"type":"content_block_start","index":index,"content_block":{"type":"fallback"}})
    }

    #[tokio::test]
    async fn c009_allowed_fallbacks_reach_request() {
        let model = source();
        let WireCompat::AnthropicMessages(compat) = &model.compat else {
            unreachable!()
        };
        assert_eq!(compat.allowed_fallback_models.as_ref().unwrap().len(), 2);
        let captured = invoke(model, None, vec![]).await;
        captured.outcome.unwrap();
        assert_eq!(
            captured.body["fallbacks"],
            json!([{"model":"claude-opus-4-8"},{"model":"claude-opus-5"}])
        );
    }

    #[tokio::test]
    async fn c009_leading_fallback_block_is_not_a_parse_failure() {
        let captured = invoke(source(), Some("claude-opus-4-8"), vec![fallback(0)]).await;
        let reply = captured
            .outcome
            .expect("source accepts fallback marker before output");
        assert_eq!(reply.response_model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(reply.response_id.as_deref(), Some("received-id"));
        assert!(reply.content.is_empty());
    }

    #[tokio::test]
    async fn fallback_targets_are_ordered_model_only_and_gate_only_their_beta() {
        for count in [None, Some(0), Some(1), Some(3)] {
            let mut model = source();
            let template = compat(&mut model).allowed_fallback_models.as_ref().unwrap()[0].clone();
            let entries = count.map(|count| {
                (0..count)
                    .map(|index| {
                        let mut entry = template.clone();
                        entry.provider = format!("local-provider-{index}");
                        entry.model = format!("target-{index}");
                        entry
                    })
                    .collect::<Vec<_>>()
            });
            compat(&mut model).allowed_fallback_models = entries;
            compat(&mut model).supports_mid_convo_effort = false;
            let captured = invoke(model, None, vec![]).await;
            captured.outcome.unwrap();
            if count.is_some_and(|count| count > 0) {
                let expected: Vec<_> = (0..count.unwrap())
                    .map(|index| json!({"model":format!("target-{index}")}))
                    .collect();
                assert_eq!(captured.body["fallbacks"], json!(expected));
                assert_eq!(
                    captured.body["betas"],
                    json!(["server-side-fallback-2026-07-01"])
                );
            } else {
                assert!(captured.body.get("fallbacks").is_none());
                assert!(captured.body.get("betas").is_none());
            }
        }
    }

    fn text(index: usize, value: &str) -> Vec<Value> {
        vec![
            json!({"type":"content_block_start","index":index,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":index,"delta":{"type":"text_delta","text":value}}),
            json!({"type":"content_block_stop","index":index}),
        ]
    }

    #[tokio::test]
    async fn leading_markers_preserve_dense_text_tool_and_thinking_indices_and_raw_observation() {
        let mut blocks = vec![
            fallback(0),
            json!({"type":"content_block_stop","index":0}),
            fallback(1),
        ];
        blocks.extend(text(4, "answer"));
        blocks.extend([
            json!({"type":"content_block_start","index":9,"content_block":{"type":"tool_use","id":"tool-1","name":"read","input":{}}}),
            json!({"type":"content_block_delta","index":9,"delta":{"type":"input_json_delta","partial_json":"{\"x\":"}}),
            json!({"type":"content_block_delta","index":9,"delta":{"type":"input_json_delta","partial_json":"\"fixture\"}"}}),
            json!({"type":"content_block_stop","index":9}),
            json!({"type":"content_block_start","index":11,"content_block":{"type":"thinking","thinking":"reasoning","signature":""}}),
            json!({"type":"content_block_delta","index":11,"delta":{"type":"thinking_delta","thinking":" continued"}}),
            json!({"type":"content_block_delta","index":11,"delta":{"type":"signature_delta","signature":"opaque-signature"}}),
            json!({"type":"content_block_stop","index":11}),
            json!({"type":"content_block_start","index":13,"content_block":{"type":"redacted_thinking","data":"opaque-data"}}),
            json!({"type":"content_block_stop","index":13}),
        ]);
        let captured = invoke(source(), Some("claude-opus-4-8"), blocks).await;
        assert_eq!(captured.raw[1], fallback(0));
        assert_eq!(captured.raw[3], fallback(1));
        let indices: Vec<_> = captured
            .events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::TextStart { content_index, .. }
                | AssistantStreamEvent::ToolCallStart { content_index, .. }
                | AssistantStreamEvent::ThinkingStart { content_index, .. } => Some(*content_index),
                _ => None,
            })
            .collect();
        assert_eq!(indices, [0, 1, 2]);
        assert!(captured.events.iter().any(|event| matches!(event,
            AssistantStreamEvent::ToolCallEnd { content_index: 1, tool_call, .. }
            if tool_call.arguments == "{\"x\":\"fixture\"}" && tool_call.arguments_error.is_none())));
        let reply = captured.outcome.unwrap();
        assert_eq!(reply.model, "claude-fable-5");
        assert_eq!(reply.response_model.as_deref(), Some("claude-opus-4-8"));
        assert_eq!(reply.wire, Some(WireApi::AnthropicMessages));
        assert_eq!(reply.content.len(), 4);
        assert!(
            matches!(&reply.content[0], AssistantContent::Text { text, .. } if text == "answer")
        );
        assert!(
            matches!(&reply.content[2], AssistantContent::Thinking { thinking, signature }
            if thinking == "reasoning continued" && signature.as_deref() == Some("opaque-signature"))
        );
        assert!(
            matches!(&reply.content[3], AssistantContent::RedactedThinking { data } if data == "opaque-data")
        );
        assert!(source().calculate_response_cost(&reply).is_some());
    }

    #[tokio::test]
    async fn duplicate_raw_indices_keep_the_first_output_association() {
        let mut blocks = vec![fallback(0)];
        blocks.extend([
            json!({"type":"content_block_start","index":7,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_start","index":7,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":7,"delta":{"type":"text_delta","text":"first"}}),
            json!({"type":"content_block_stop","index":7}),
        ]);
        let captured = invoke(source(), None, blocks).await;
        let reply = captured.outcome.unwrap();
        assert_eq!(reply.content.len(), 2);
        assert!(
            matches!(&reply.content[0], AssistantContent::Text { text, .. } if text == "first")
        );
        assert!(
            matches!(&reply.content[1], AssistantContent::Text { text, .. } if text.is_empty())
        );
        assert!(captured.events.iter().any(|event| matches!(event,
            AssistantStreamEvent::TextDelta { content_index: 0, delta, .. } if delta == "first")));
    }

    #[tokio::test]
    async fn transport_index_reuse_after_stop_targets_the_new_output_block() {
        let mut blocks = vec![fallback(0)];
        blocks.extend(text(7, "A"));
        blocks.extend(text(7, "B"));
        blocks.extend([
            json!({"type":"content_block_delta","index":7,"delta":{"type":"text_delta","text":"after-stop"}}),
            json!({"type":"content_block_stop","index":7}),
        ]);
        let captured = invoke(source(), None, blocks).await;
        let reply = captured.outcome.unwrap();
        assert_eq!(
            serde_json::to_value(reply.content).unwrap(),
            json!([
                {"type":"text","text":"A"}, {"type":"text","text":"B"}
            ])
        );
        let deltas: Vec<_> = captured
            .events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::TextDelta {
                    content_index,
                    delta,
                    ..
                } => Some((*content_index, delta.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, [(0, "A"), (1, "B")]);
        let ends: Vec<_> = captured
            .events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::TextEnd {
                    content_index,
                    content,
                    ..
                } => Some((*content_index, content.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(ends, [(0, "A"), (1, "B")]);
    }

    #[tokio::test]
    async fn transport_index_duplicate_active_blocks_advance_after_the_first_stop() {
        let blocks = vec![
            fallback(0),
            json!({"type":"content_block_start","index":7,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_start","index":7,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":7,"delta":{"type":"text_delta","text":"A"}}),
            json!({"type":"content_block_stop","index":7}),
            json!({"type":"content_block_delta","index":7,"delta":{"type":"text_delta","text":"B"}}),
            json!({"type":"content_block_stop","index":7}),
            json!({"type":"content_block_delta","index":7,"delta":{"type":"text_delta","text":"after-stop"}}),
            json!({"type":"content_block_stop","index":7}),
        ];
        let captured = invoke(source(), None, blocks).await;
        let reply = captured.outcome.unwrap();
        assert_eq!(
            serde_json::to_value(reply.content).unwrap(),
            json!([
                {"type":"text","text":"A"}, {"type":"text","text":"B"}
            ])
        );
        let deltas: Vec<_> = captured
            .events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::TextDelta {
                    content_index,
                    delta,
                    ..
                } => Some((*content_index, delta.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, [(0, "A"), (1, "B")]);
        let ends: Vec<_> = captured
            .events
            .iter()
            .filter_map(|event| match event {
                AssistantStreamEvent::TextEnd {
                    content_index,
                    content,
                    ..
                } => Some((*content_index, content.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(ends, [(0, "A"), (1, "B")]);
    }

    fn assert_mid_output_failure(captured: Capture) -> AssistantMessage {
        assert_eq!(
            captured.raw.last().unwrap()["content_block"]["type"],
            "fallback"
        );
        assert!(
            !captured
                .events
                .iter()
                .any(AssistantStreamEvent::is_terminal)
        );
        let failure = captured.outcome.unwrap_err();
        let ProviderError::StreamError(summary) = failure.cause() else {
            panic!("mid-output fallback must retain its typed semantic cause");
        };
        assert_eq!(
            summary.as_str(),
            "Anthropic performed an unsupported mid-output model fallback"
        );
        assert_eq!(failure.message().model, "claude-fable-5");
        assert_eq!(
            failure.message().response_model.as_deref(),
            Some("claude-opus-4-8")
        );
        assert_eq!(failure.message().stop_reason, StopReason::Error);
        assert_eq!(failure.message().usage.input_tokens, 100);
        assert_eq!(failure.message().usage.output_tokens, 0);
        failure.message().clone()
    }

    #[tokio::test]
    async fn c009_mid_output_fallback_preserves_partial_failure_control() {
        let mut blocks = text(0, "partial-prefix");
        blocks.push(fallback(1));
        let partial =
            assert_mid_output_failure(invoke(source(), Some("claude-opus-4-8"), blocks).await);
        assert!(partial.content.iter().any(|content| matches!(content,
            AssistantContent::Text { text, .. } if text == "partial-prefix")));
    }

    #[tokio::test]
    async fn fallback_after_any_started_output_block_is_a_semantic_failure() {
        for content in [
            json!({"type":"text","text":""}),
            json!({"type":"tool_use","id":"tool-1","name":"read","input":{}}),
            json!({"type":"thinking","thinking":"","signature":""}),
            json!({"type":"redacted_thinking","data":"opaque-data"}),
        ] {
            let blocks = vec![
                json!({"type":"content_block_start","index":0,"content_block":content}),
                fallback(1),
            ];
            let partial =
                assert_mid_output_failure(invoke(source(), Some("claude-opus-4-8"), blocks).await);
            assert_eq!(partial.content.len(), 1);
        }
    }

    #[tokio::test]
    async fn malformed_fallback_carriers_still_fail_as_malformed_frames() {
        for invalid in [
            json!({"type":"content_block_start","content_block":{"type":"fallback"}}),
            json!({"type":"content_block_start","index":-1,"content_block":{"type":"fallback"}}),
            json!({"type":"content_block_start","index":"0","content_block":{"type":"fallback"}}),
            json!({"type":"content_block_start","index":0.5,"content_block":{"type":"fallback"}}),
        ] {
            let captured = invoke(source(), None, vec![invalid.clone()]).await;
            assert!(captured.raw.contains(&invalid));
            let failure = captured.outcome.unwrap_err();
            let ProviderError::StreamError(summary) = failure.cause() else {
                panic!("typed malformed error")
            };
            assert_eq!(
                summary.as_str(),
                "malformed Anthropic SSE event 'content_block_start'"
            );
            assert!(failure.message().content.is_empty());
        }
    }

    #[tokio::test]
    async fn public_low_level_parser_boundary_is_retained_while_stream_helper_handles_transport_marker()
     {
        let frame = format!("event: content_block_start\ndata: {}\n\n", fallback(0));
        assert!(matches!(
            parse_sse_events(&frame).next(),
            Some(ParsedEvent::Malformed { .. })
        ));
        let (parsed, raw) = opi_ai::api::anthropic::parse_sse_events_with_raw(&frame)
            .next()
            .unwrap();
        assert!(matches!(parsed, ParsedEvent::Malformed { .. }));
        assert_eq!(raw, Some(fallback(0)));
        let mut response = frame;
        for event in text(3, "answer")
            .into_iter()
            .chain([json!({"type":"message_stop"})])
        {
            response.push_str(&format!(
                "event: {}\ndata: {event}\n\n",
                event["type"].as_str().unwrap()
            ));
        }
        let events: Vec<_> = AnthropicProvider::new(None)
            .stream_from_sse(&response, Default::default())
            .collect()
            .await;
        let done = events.last().unwrap().as_ref().unwrap();
        assert!(matches!(done, AssistantStreamEvent::Done { message, .. }
            if message.content.len() == 1 && matches!(&message.content[0], AssistantContent::Text { text, .. } if text == "answer")));
    }
}

mod managed_effort_source_compat {
    use super::*;
    use opi_ai::catalog::{Catalog, CatalogOperation};
    use opi_ai::message::AssistantMessage;
    use serde_json::{Value, json};
    use std::sync::Arc;

    fn source() -> ModelInfo {
        let record = Catalog::builtin()
            .unwrap()
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5-1")
            .unwrap()
            .clone();
        assert_eq!(record.raw["compat"]["supportsMidConvoEffort"], true);
        record.to_chat_model_info().unwrap()
    }

    fn request() -> Request {
        let mut request = request_39_for(
            "anthropic:claude-fable-5-1",
            ThinkingConfig {
                enabled: true,
                budget_tokens: Some(2048),
                level: opi_ai::ThinkingLevel::Low,
            },
        );
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("system".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "Hi".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .unwrap();
        request.cache_retention = CacheRetention::Long;
        request.session_id = Some("session-complete-id".into());
        request
    }

    struct Capture {
        body: Value,
        events: Vec<AssistantStreamEvent>,
        outcome: Result<AssistantMessage, opi_ai::CallFailure>,
    }

    async fn capture(
        model: ModelInfo,
        request: Request,
        native: Option<AnthropicMessagesOptions>,
        response: Option<String>,
    ) -> Capture {
        capture_with_sends(model, request, native, response, 1).await
    }

    async fn capture_with_sends(
        mut model: ModelInfo,
        mut request: Request,
        native: Option<AnthropicMessagesOptions>,
        response: Option<String>,
        expected_sends: u64,
    ) -> Capture {
        let server = MockServer::start().await;
        let response = response.unwrap_or_else(|| {
            [
                json!({"type":"message_start","message":{"id":"received-id","model":model.id,
                "usage":{"input_tokens":100,"output_tokens":0}}}),
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},
                "usage":{"output_tokens":10}}),
                json!({"type":"message_stop"}),
            ]
            .into_iter()
            .map(|event| {
                format!(
                    "event: {}\ndata: {event}\n\n",
                    event["type"].as_str().unwrap()
                )
            })
            .collect()
        });
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(response),
            )
            .expect(expected_sends)
            .mount(&server)
            .await;
        model.base_url = Some(server.uri());
        model.capabilities.supports_cache_control = true;
        model.capabilities.supports_long_cache_retention = true;
        request.model = model.id.clone();
        let canonical = serde_json::to_vec(&request.context).unwrap();
        let provider = Arc::new(AnthropicProvider::for_route(
            "anthropic".into(),
            vec![model.clone()],
            None,
            Default::default(),
            Arc::new(HttpClient::new()),
            false,
        ));
        let api = AnthropicMessagesChatApi::new(provider, model.clone()).unwrap();
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    "synthetic-key".into(),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        let record = opi_ai::ChatModelRecord::new("anthropic", model);
        let mut call = if let Some(native) = native {
            models.stream(record, request.clone(), handle.options(native))
        } else {
            models.stream_simple(record, request.clone(), Default::default())
        };
        let mut events = Vec::new();
        while let Some(item) = call.next().await {
            if let Ok(event) = item {
                events.push(event);
            }
        }
        let outcome = call.result().await;
        if let Ok(message) = &outcome {
            assert_eq!(message.stop_reason, StopReason::Stop);
            assert_eq!(message.usage.input_tokens, 100);
            assert_eq!(message.usage.output_tokens, 10);
        }
        assert_eq!(serde_json::to_vec(&request.context).unwrap(), canonical);
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len() as u64, expected_sends);
        server.verify().await;
        Capture {
            body: sent
                .first()
                .map(|sent| sent.body_json().unwrap())
                .unwrap_or(Value::Null),
            events,
            outcome,
        }
    }

    #[tokio::test]
    async fn c009_mid_effort_emits_binding_and_stamps_response() {
        let captured = capture(source(), request(), None, None).await;
        assert_eq!(
            captured.body["thinking"]["block_binding"]["prefix_mismatch_behavior"],
            "drop_block"
        );
        assert_eq!(
            captured.outcome.unwrap().provider_thinking_level.as_deref(),
            Some("low")
        );
    }

    fn compat(model: &mut ModelInfo) -> &mut AnthropicMessagesCompat {
        let WireCompat::AnthropicMessages(compat) = &mut model.compat else {
            unreachable!()
        };
        compat
    }

    fn assert_managed(captured: &Capture, effort: &str) {
        assert_eq!(
            captured.body["thinking"],
            json!({"type":"adaptive", "display":"summarized",
            "block_binding":{"prefix_mismatch_behavior":"drop_block"}})
        );
        assert_eq!(captured.body["output_config"], json!({"effort":"high"}));
        let messages = captured.body["messages"].as_array().unwrap();
        assert_eq!(
            messages.last().unwrap(),
            &json!({"role":"system", "content":[],
            "output_config":{"effort":effort}})
        );
        let betas = captured.body["betas"].as_array().unwrap();
        for beta in [
            "mid-conversation-output-config-2026-07-01",
            "thinking-binding-controls-2026-08-01",
        ] {
            assert_eq!(betas.iter().filter(|value| *value == beta).count(), 1);
        }
        let mut starts = 0;
        let mut terminals = 0;
        for event in &captured.events {
            match event {
                AssistantStreamEvent::Start { partial } => {
                    starts += 1;
                    assert_eq!(partial.provider_thinking_level.as_deref(), Some(effort));
                }
                AssistantStreamEvent::Done { message, .. } => {
                    terminals += 1;
                    assert_eq!(message.provider_thinking_level.as_deref(), Some(effort));
                }
                _ => {}
            }
        }
        assert_eq!((starts, terminals), (1, 1));
        assert_eq!(
            captured
                .outcome
                .as_ref()
                .unwrap()
                .provider_thinking_level
                .as_deref(),
            Some(effort)
        );
    }

    #[tokio::test]
    async fn native_current_effort_and_omitted_high_are_compiled_independently_of_wire_high() {
        use opi_ai::{ThinkingLevel, ThinkingLevelMapping};
        for (effort, expected) in [
            (Some(ThinkingLevel::Low), "low"),
            (Some(ThinkingLevel::Medium), "medium"),
            (Some(ThinkingLevel::High), "high"),
            (Some(ThinkingLevel::XHigh), "xhigh"),
            (Some(ThinkingLevel::Max), "max"),
            (None, "high"),
        ] {
            let mut model = source();
            for level in [ThinkingLevel::XHigh, ThinkingLevel::Max] {
                model.thinking_level_map = model
                    .thinking_level_map
                    .with_mapping(level, ThinkingLevelMapping::Identity);
            }
            let mut options = AnthropicMessagesOptions::default();
            options.effort = effort;
            let mut input = request();
            input.temperature = Some(0.25);
            let captured = capture(model, input, Some(options), None).await;
            assert_managed(&captured, expected);
            assert!(captured.body.get("temperature").is_none());
        }
        for native in [false, true] {
            let mut model = source();
            model.thinking_level_map = model.thinking_level_map.with_mapping(
                ThinkingLevel::Low,
                ThinkingLevelMapping::Mapped("source-defined-effort".into()),
            );
            let mut options = AnthropicMessagesOptions::default();
            options.effort = Some(ThinkingLevel::Low);
            let options = native.then_some(options);
            let captured = capture(model, request(), options, None).await;
            assert_managed(&captured, "source-defined-effort");
        }
    }

    #[tokio::test]
    async fn simple_adaptive_translation_and_numeric_high_follow_the_source_entry() {
        use opi_ai::{ThinkingLevel, ThinkingLevelMap, ThinkingLevelMapping};
        for (level, expected) in [
            (ThinkingLevel::None, "high"),
            (ThinkingLevel::Minimal, "low"),
            (ThinkingLevel::Low, "low"),
            (ThinkingLevel::Medium, "medium"),
            (ThinkingLevel::High, "high"),
            (ThinkingLevel::XHigh, "high"),
            (ThinkingLevel::Max, "high"),
        ] {
            let mut model = source();
            model.thinking_level_map = ThinkingLevelMap::reasoning_default()
                .with_mapping(ThinkingLevel::XHigh, ThinkingLevelMapping::Identity)
                .with_mapping(ThinkingLevel::Max, ThinkingLevelMapping::Identity);
            let mut input = request();
            input.thinking.level = level;
            let captured = capture(model, input, None, None).await;
            assert_managed(&captured, expected);
        }
        let mut model = source();
        compat(&mut model).force_adaptive_thinking = false;
        let captured = capture(model, request(), None, None).await;
        assert_managed(&captured, "high");
    }

    #[tokio::test]
    async fn native_disabled_and_simple_off_respect_reasoning_and_nullable_off_mapping() {
        use opi_ai::{ThinkingLevel, ThinkingLevelMapping};
        for flagged in [false, true] {
            for off_allowed in [false, true] {
                for enabled in [None, Some(false), Some(true)] {
                    let mut model = source();
                    compat(&mut model).supports_mid_convo_effort = flagged;
                    model.thinking_level_map = model.thinking_level_map.with_mapping(
                        ThinkingLevel::None,
                        if off_allowed {
                            ThinkingLevelMapping::Identity
                        } else {
                            ThinkingLevelMapping::Unsupported
                        },
                    );
                    let mut input = request();
                    input.temperature = Some(0.25);
                    let mut options = AnthropicMessagesOptions::default();
                    options.thinking_enabled = enabled;
                    let captured = capture(model, input, Some(options), None).await;
                    let disabled = enabled == Some(false) && off_allowed;
                    let managed = flagged && !disabled;
                    let body = &captured.body;
                    if disabled {
                        assert_eq!(body["thinking"], json!({"type":"disabled"}));
                    } else if managed {
                        assert_managed(&captured, "high");
                    } else if enabled == Some(true) {
                        assert_eq!(
                            body["thinking"],
                            json!({"type":"adaptive", "display":"summarized"})
                        );
                    } else {
                        assert!(body.get("thinking").is_none());
                    }
                    assert_eq!(
                        body.get("temperature").is_some(),
                        !flagged && enabled != Some(true)
                    );
                    assert_eq!(body.get("betas").is_some(), flagged);
                    let markers = body["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|message| message.get("output_config").is_some())
                        .count();
                    assert_eq!(markers, usize::from(managed));
                    if !managed {
                        assert!(body.get("output_config").is_none());
                        assert!(captured.outcome.unwrap().provider_thinking_level.is_none());
                    }
                }
            }
        }
        for off_allowed in [false, true] {
            let mut model = source();
            model.thinking_level_map = model.thinking_level_map.with_mapping(
                ThinkingLevel::None,
                if off_allowed {
                    ThinkingLevelMapping::Identity
                } else {
                    ThinkingLevelMapping::Unsupported
                },
            );
            let mut input = request();
            input.thinking = ThinkingConfig::default();
            let captured = capture(model, input, None, None).await;
            if off_allowed {
                assert_eq!(captured.body["thinking"], json!({"type":"disabled"}));
                assert!(captured.outcome.unwrap().provider_thinking_level.is_none());
                assert!(
                    captured.body["messages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|message| message.get("output_config").is_none())
                );
            } else {
                assert_managed(&captured, "high");
            }
        }
        let mut model = source();
        model.capabilities.supports_thinking = false;
        let mut options = AnthropicMessagesOptions::default();
        options.thinking_enabled = Some(false);
        let captured = capture(model, request(), Some(options), None).await;
        assert_managed(&captured, "high");
    }

    fn history(level: Option<&str>, identity: &str) -> Message {
        let mut message = opi_ai::test_support::base_assistant();
        message.provider = "anthropic".into();
        message.model = "different-historical-model".into();
        message.wire = Some(WireApi::AnthropicMessages);
        message.content = vec![AssistantContent::Text {
            text: "prior answer".into(),
            signature: None,
        }];
        message.provider_thinking_level = level.map(str::to_owned);
        match identity {
            "same" => {}
            "coarse-api" => message.api = opi_ai::ApiKind::OpenAi,
            "provider" => message.provider = "other".into(),
            "wire" => message.wire = Some(WireApi::OpenAiCompletions),
            "legacy" => message.wire = None,
            _ => unreachable!(),
        }
        Message::Assistant(message)
    }

    #[test]
    fn flagged_off_null_native_false_does_not_inherit_interleaved_beta_from_neutral_true() {
        let mut model = source();
        compat(&mut model).force_adaptive_thinking = false;
        model.thinking_level_map = model.thinking_level_map.with_mapping(
            opi_ai::ThinkingLevel::None,
            opi_ai::ThinkingLevelMapping::Unsupported,
        );
        let mut options = AnthropicMessagesOptions::default();
        options.thinking_enabled = Some(false);
        let mut input = request();
        input.temperature = Some(0.25);
        assert!(input.thinking.enabled);
        let body = provider_39(&model, options)
            .build_request_body(&input)
            .unwrap();
        assert_eq!(
            body["thinking"],
            json!({"type":"adaptive","display":"summarized",
            "block_binding":{"prefix_mismatch_behavior":"drop_block"}})
        );
        assert_eq!(body["output_config"], json!({"effort":"high"}));
        assert_eq!(
            body["messages"].as_array().unwrap().last().unwrap(),
            &json!({"role":"system","content":[],"output_config":{"effort":"high"}})
        );
        assert!(body.get("temperature").is_none());
        assert_eq!(
            body["betas"],
            json!([
                "mid-conversation-output-config-2026-07-01",
                "thinking-binding-controls-2026-08-01"
            ])
        );
        compat(&mut model).supports_mid_convo_effort = false;
        let mut options = AnthropicMessagesOptions::default();
        options.thinking_enabled = Some(false);
        let body = provider_39(&model, options)
            .build_request_body(&input)
            .unwrap();
        assert_eq!(body["betas"], json!(["interleaved-thinking-2025-05-14"]));
    }

    #[tokio::test]
    async fn historical_effort_uses_exact_wire_and_provider_without_model_or_coarse_api_match() {
        for level in [
            Some("low"),
            Some("medium"),
            Some("high"),
            Some("xhigh"),
            Some("max"),
            None,
            Some(""),
            Some("minimal"),
            Some("off"),
            Some("HIGH"),
            Some(" high "),
        ] {
            for identity in ["same", "coarse-api", "provider", "wire", "legacy"] {
                let mut input = request();
                let mut messages = input.context.messages().to_vec();
                messages.push(history(level, identity));
                input.context =
                    opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                        messages,
                        ..Default::default()
                    })
                    .unwrap();
                let captured = capture(source(), input, None, None).await;
                assert_managed(&captured, "low");
                let messages = captured.body["messages"].as_array().unwrap();
                let historical = matches!(level, Some("low" | "medium" | "high" | "xhigh" | "max"))
                    && matches!(identity, "same" | "coarse-api");
                assert_eq!(messages.len(), if historical { 4 } else { 3 });
                if historical {
                    assert_eq!(
                        messages[1],
                        json!({"role":"system","content":[],"output_config":{"effort":level.unwrap()}})
                    );
                    assert_eq!(messages[2]["role"], "assistant");
                } else {
                    assert_eq!(messages[1]["role"], "assistant");
                }
                assert!(
                    captured
                        .outcome
                        .unwrap()
                        .projection
                        .unwrap()
                        .replay
                        .losses
                        .is_empty()
                );
            }
        }
    }

    #[tokio::test]
    async fn managed_display_and_omitted_wire_assistant_do_not_create_orphan_history_markers() {
        let mut model = source();
        compat(&mut model).allow_empty_signature = true;
        let mut blank = history(Some("max"), "same");
        let Message::Assistant(message) = &mut blank else {
            unreachable!()
        };
        message.model = model.id.clone();
        message.content = vec![AssistantContent::Thinking {
            thinking: " \t".into(),
            signature: Some("".into()),
        }];
        let mut input = request();
        let mut messages = input.context.messages().to_vec();
        messages.push(blank);
        input.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        let mut options = AnthropicMessagesOptions::default();
        options.effort = Some(opi_ai::ThinkingLevel::Low);
        options.thinking_display = Some(AnthropicThinkingDisplay::Detailed);
        let captured = capture(model, input, Some(options), None).await;
        assert_eq!(
            captured.body["thinking"],
            json!({"type":"adaptive", "display":"detailed",
            "block_binding":{"prefix_mismatch_behavior":"drop_block"}})
        );
        assert_eq!(captured.body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(captured.body["messages"][0]["role"], "user");
        assert_eq!(
            captured.body["messages"][1],
            json!({"role":"system","content":[],"output_config":{"effort":"low"}})
        );
        let outcome = captured.outcome.unwrap();
        assert_eq!(outcome.provider_thinking_level.as_deref(), Some("low"));
        assert!(outcome.projection.unwrap().replay.losses.is_empty());
    }

    #[tokio::test]
    async fn managed_markers_preserve_tool_result_adjacency_and_delayed_system_order() {
        let mut model = source();
        compat(&mut model).supports_mid_convo_system_messages = true;
        compat(&mut model).supports_mid_convo_tool_changes = true;
        let mut context = inline_tools_context_39();
        let mut call = opi_ai::test_support::base_assistant();
        call.provider = "anthropic".into();
        call.model = model.id.clone();
        call.wire = Some(WireApi::AnthropicMessages);
        call.content = vec![AssistantContent::ToolCall {
            tool_call: opi_ai::message::ToolCall {
                id: "tool-turn".into(),
                name: "initial_tool".into(),
                arguments: "{\"x\":\"fixture\"}".into(),
                arguments_error: None,
                signature: None,
            },
        }];
        context.messages.insert(2, Message::Assistant(call));
        context
            .messages
            .push(Message::ToolResult(opi_ai::message::ToolResultMessage {
                tool_call_id: "tool-turn".into(),
                tool_name: "initial_tool".into(),
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "result".into(),
                }],
                details: None,
                is_error: false,
                truncated: false,
                timestamp_ms: 3,
                not_executed: None,
            }));
        context.messages.push(history(Some("medium"), "same"));
        let mut input = request();
        input.context = opi_ai::transcript::normalize_context(context).unwrap();
        let captured = capture(model, input, None, None).await;
        assert_managed(&captured, "low");
        let messages = captured.body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 7);
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["content"][0]["type"], "tool_use");
        assert_eq!(messages[2]["role"], "user");
        assert_eq!(messages[2]["content"][0]["type"], "tool_result");
        assert_eq!(messages[3]["role"], "system");
        assert_eq!(messages[3]["content"][0]["text"], "Update.");
        assert_eq!(
            messages[4],
            json!({"role":"system","content":[],"output_config":{"effort":"medium"}})
        );
        assert_eq!(messages[5]["role"], "assistant");
        assert_eq!(messages[6]["output_config"]["effort"], "low");
    }

    #[tokio::test]
    async fn compiled_effort_survives_payload_rewrite_and_stream_failure() {
        for failure in [false, true] {
            let mut input = request();
            input.callbacks =
                opi_ai::RequestCallbacks::new().with_on_payload(Arc::new(|body| {
                    let mut body = body.clone();
                    body["messages"].as_array_mut().unwrap().last_mut().unwrap()["output_config"]
                        ["effort"] = json!("max");
                    Ok(Some(body))
                }));
            let response = failure.then(|| concat!(
                "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"received-id\",\"model\":\"reported-model\",\"usage\":{\"input_tokens\":100,\"output_tokens\":0}}}\n\n",
                "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial\"}}\n\n"
            ).to_owned());
            let captured = capture(source(), input, None, response).await;
            assert_eq!(
                captured.body["messages"]
                    .as_array()
                    .unwrap()
                    .last()
                    .unwrap()["output_config"]["effort"],
                "max"
            );
            assert!(
                matches!(&captured.events[0], AssistantStreamEvent::Start { partial }
                if partial.provider_thinking_level.as_deref() == Some("low"))
            );
            if failure {
                let failure = captured.outcome.unwrap_err();
                assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
                assert_eq!(
                    failure.message().provider_thinking_level.as_deref(),
                    Some("low")
                );
                assert_eq!(failure.message().model, "claude-fable-5-1");
                assert_eq!(
                    failure.message().response_model.as_deref(),
                    Some("reported-model")
                );
                assert!(
                    matches!(&failure.message().content[0], AssistantContent::Text { text, .. } if text == "partial")
                );
            } else {
                assert_eq!(
                    captured.outcome.unwrap().provider_thinking_level.as_deref(),
                    Some("low")
                );
            }
        }
    }

    #[tokio::test]
    async fn compiled_stamp_precedes_payload_and_response_callback_failures() {
        for response_phase in [false, true] {
            let mut input = request();
            input.callbacks = if response_phase {
                opi_ai::RequestCallbacks::new().with_on_response(Arc::new(|_, _| {
                    Err(opi_ai::callbacks::CallbackError::new(
                        "synthetic response rejection",
                    ))
                }))
            } else {
                opi_ai::RequestCallbacks::new().with_on_payload(Arc::new(|_| {
                    Err(opi_ai::callbacks::CallbackError::new(
                        "synthetic payload rejection",
                    ))
                }))
            };
            let captured =
                capture_with_sends(source(), input, None, None, u64::from(response_phase)).await;
            assert!(captured.events.is_empty());
            let failure = captured.outcome.unwrap_err();
            assert!(matches!(failure.cause(), ProviderError::CallbackPhase(_)));
            assert_eq!(
                failure.message().provider_thinking_level.as_deref(),
                Some("low")
            );
            assert_eq!(failure.message().model, "claude-fable-5-1");
            assert!(failure.message().content.is_empty());
        }
    }

    #[tokio::test]
    async fn historical_marker_follows_pending_native_system_and_precedes_its_assistant() {
        let mut model = source();
        compat(&mut model).supports_mid_convo_system_messages = true;
        compat(&mut model).supports_mid_convo_tool_changes = true;
        let mut context = inline_tools_context_39();
        context.messages.push(history(Some("medium"), "same"));
        let mut input = request();
        input.context = opi_ai::transcript::normalize_context(context).unwrap();
        let captured = capture(model, input, None, None).await;
        assert_managed(&captured, "low");
        let messages = captured.body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "system");
        assert_eq!(messages[1]["content"][0]["text"], "Update.");
        assert_eq!(
            messages[2],
            json!({"role":"system","content":[],"output_config":{"effort":"medium"}})
        );
        assert_eq!(messages[3]["role"], "assistant");
        assert_eq!(messages[4]["output_config"]["effort"], "low");
        assert_eq!(
            captured.body["betas"],
            json!([
                "mid-conversation-output-config-2026-07-01",
                "thinking-binding-controls-2026-08-01",
                "inline-tools-2026-09-15"
            ])
        );
    }
}

mod anthropic_affinity_source_compat {
    use super::*;
    use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
    use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation};
    use serde_json::json;
    use std::sync::Arc;

    #[derive(Clone)]
    struct FixtureAuth(ResolvedAuth);
    impl AuthResolver for FixtureAuth {
        fn resolve(&self) -> opi_ai::BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
            Box::pin(async { Ok(self.0.clone()) })
        }
    }

    struct Fixture {
        record: CatalogModel,
        request: Request,
        auth: ResolvedAuth,
        configured: Vec<(String, String)>,
        base_suffix: &'static str,
        copilot: bool,
        direct_oauth: bool,
    }

    impl Fixture {
        fn new() -> Self {
            let record = Catalog::builtin()
                .unwrap()
                .model(
                    "fireworks",
                    CatalogOperation::Chat,
                    "accounts/fireworks/models/deepseek-v4p1-flash",
                )
                .unwrap()
                .clone();
            let mut request = request_39(Default::default());
            request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: Some("system".into()),
                messages: vec![Message::User(UserMessage {
                    content: vec![InputContent::Text { text: "Hi".into() }],
                    timestamp_ms: 0,
                })],
                tools: vec![],
            })
            .unwrap();
            request.max_tokens = Some(4096);
            request.cache_retention = CacheRetention::Long;
            request.session_id = Some("session-complete-id".into());
            Self {
                record,
                request,
                auth: ResolvedAuth {
                    scheme: AuthScheme::ApiKey,
                    secret: "synthetic-key".into(),
                    base_url: None,
                    account_id: None,
                    env: Default::default(),
                    provenance: Default::default(),
                },
                configured: vec![],
                base_suffix: "",
                copilot: false,
                direct_oauth: false,
            }
        }

        fn source(&mut self, provider: &str, enabled: Option<bool>, router_format: bool) {
            self.record.provider = provider.into();
            self.record.raw["provider"] = json!(provider);
            let compat = self.record.raw["compat"].as_object_mut().unwrap();
            compat.remove("sendSessionAffinityHeaders");
            compat.remove("sessionAffinityFormat");
            if let Some(enabled) = enabled {
                compat.insert("sendSessionAffinityHeaders".into(), json!(enabled));
            }
            if router_format {
                compat.insert("sessionAffinityFormat".into(), json!("openrouter"));
            }
        }

        async fn capture(mut self) -> wiremock::Request {
            let server = MockServer::start().await;
            let events = [
                json!({"type":"message_start", "message": {"id":"received-id",
                    "model":self.record.id, "usage":{"input_tokens":100, "output_tokens":0}}}),
                json!({"type":"message_delta", "delta":{"stop_reason":"end_turn"},
                    "usage":{"output_tokens":10}}),
                json!({"type":"message_stop"}),
            ];
            let response = events
                .into_iter()
                .map(|event| {
                    format!(
                        "event: {}\ndata: {event}\n\n",
                        event["type"].as_str().unwrap()
                    )
                })
                .collect::<String>();
            Mock::given(method("POST"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(response),
                )
                .expect(1)
                .mount(&server)
                .await;
            let mut model = self.record.to_chat_model_info().unwrap();
            model.base_url = Some(format!("{}{}", server.uri(), self.base_suffix));
            model.capabilities.supports_cache_control = true;
            model.capabilities.supports_long_cache_retention = true;
            self.request.model = model.id.clone();
            let mut provider = AnthropicProvider::for_route(
                self.record.provider.clone(),
                vec![model.clone()],
                None,
                opi_ai::api::shared::headers::ProviderHeaders::try_new(self.configured).unwrap(),
                Arc::new(opi_ai::http::HttpClient::new()),
                self.direct_oauth,
            );
            if self.copilot {
                provider = provider.with_copilot_headers();
            }
            let api = AnthropicMessagesChatApi::new(Arc::new(provider), model.clone()).unwrap();
            let mut models = Models::new();
            models
                .register_chat_api(
                    Arc::new(api),
                    Arc::new(FixtureAuth(self.auth)),
                    opi_ai::AuthProvenanceSource::Static,
                )
                .unwrap();
            let outcome = models
                .complete_simple(
                    opi_ai::ChatModelRecord::new(self.record.provider, model),
                    self.request,
                    Default::default(),
                )
                .await
                .expect("actual successful terminal");
            assert_eq!(outcome.stop_reason, StopReason::Stop);
            assert_eq!(outcome.usage.input_tokens, 100);
            assert_eq!(outcome.usage.output_tokens, 10);
            let mut sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), 1);
            server.verify().await;
            sent.remove(0)
        }
    }

    #[tokio::test]
    async fn explicit_affinity_sends_full_session_id() {
        let sent = Fixture::new().capture().await;
        assert_eq!(
            sent.headers
                .get("x-session-affinity")
                .and_then(|v| v.to_str().ok()),
            Some("session-complete-id")
        );
    }

    fn assert_affinity(sent: &wiremock::Request, affinity: Option<&str>, router: Option<&str>) {
        for (name, expected) in [("x-session-affinity", affinity), ("x-session-id", router)] {
            assert_eq!(
                sent.headers.get(name).and_then(|value| value.to_str().ok()),
                expected,
                "{name}"
            );
            assert_eq!(
                sent.headers.get_all(name).iter().count(),
                usize::from(expected.is_some()),
                "{name}"
            );
        }
        for name in ["session_id", "x-client-request-id"] {
            assert!(
                !sent.headers.contains_key(name),
                "no Responses affinity: {name}"
            );
        }
    }

    #[tokio::test]
    async fn provider_and_source_url_inference_respect_explicit_enable_and_format() {
        for (provider, suffix, enabled, format, family) in [
            ("fireworks", "", None, false, None),
            ("openrouter", "", None, false, Some("router")),
            ("alias", "/openrouter.ai", None, false, Some("router")),
            ("openrouter-proxy", "", None, false, None),
            ("alias", "/OPENROUTER.AI", None, false, None),
            ("openrouter", "", Some(false), false, None),
            ("alias", "/openrouter.ai", Some(false), false, None),
            ("fireworks", "", Some(true), true, Some("router")),
            ("fireworks", "", None, true, None),
            ("fireworks", "", Some(true), false, Some("affinity")),
        ] {
            let mut fixture = Fixture::new();
            fixture.source(provider, enabled, format);
            fixture.base_suffix = suffix;
            let sent = fixture.capture().await;
            assert_affinity(
                &sent,
                (family == Some("affinity")).then_some("session-complete-id"),
                (family == Some("router")).then_some("session-complete-id"),
            );
        }
    }

    #[tokio::test]
    async fn full_nonempty_session_affinity_is_independent_of_retention() {
        let session = "source-session-full-".repeat(8);
        for router in [false, true] {
            for retention in [
                CacheRetention::None,
                CacheRetention::Short,
                CacheRetention::Long,
                CacheRetention::Disabled,
            ] {
                let mut fixture = Fixture::new();
                fixture.source("fireworks", Some(true), router);
                fixture.request.session_id = Some(session.clone());
                fixture.request.cache_retention = retention;
                let sent = fixture.capture().await;
                assert_affinity(
                    &sent,
                    (!router).then_some(session.as_str()),
                    router.then_some(session.as_str()),
                );
            }
        }
    }

    #[tokio::test]
    async fn explicit_false_and_absent_or_empty_sessions_emit_no_generated_affinity() {
        for router in [false, true] {
            let mut fixture = Fixture::new();
            fixture.source("fireworks", Some(false), router);
            assert_affinity(&fixture.capture().await, None, None);
            for session in [None, Some("")] {
                let mut fixture = Fixture::new();
                fixture.source("openrouter", Some(true), router);
                fixture.request.session_id = session.map(str::to_owned);
                assert_affinity(&fixture.capture().await, None, None);
            }
        }
    }

    #[tokio::test]
    async fn lawful_openrouter_header_overrides_and_tombstones_win_before_transform() {
        for mode in [
            "generated",
            "model",
            "caller",
            "remove",
            "remove-then-caller",
            "transform",
        ] {
            let mut fixture = Fixture::new();
            fixture.source("fireworks", Some(true), true);
            fixture.request.cache_retention = CacheRetention::Disabled;
            fixture.configured = vec![("X-Session-Id".into(), "configured-value".into())];
            let model = matches!(mode, "model" | "caller" | "transform");
            if model {
                fixture.record.raw["headers"] = json!({"X-Session-Id":"model-value"});
            }
            if matches!(mode, "remove" | "remove-then-caller") {
                fixture.record.raw["headers"] = json!({"x-session-id":null});
            }
            if matches!(mode, "caller" | "remove-then-caller") {
                fixture
                    .request
                    .extra_headers
                    .push(("x-session-id".into(), "caller-value".into()));
            }
            let before = match mode {
                "model" | "transform" => Some("model-value"),
                "caller" | "remove-then-caller" => Some("caller-value"),
                "remove" => None,
                _ => Some("session-complete-id"),
            };
            let seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let counter = seen.clone();
            fixture.request.callbacks = opi_ai::RequestCallbacks::new()
                .with_async_header_transform(move |mut headers| {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let actual: Vec<_> = headers
                            .iter()
                            .filter(|(name, _)| name.eq_ignore_ascii_case("x-session-id"))
                            .map(|(_, value)| value.as_str())
                            .collect();
                        assert_eq!(actual, before.into_iter().collect::<Vec<_>>());
                        if mode == "transform" {
                            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("x-session-id"));
                            headers.push(("x-session-id".into(), "transformed-value".into()));
                        }
                        Ok(headers)
                    }
                });
            let sent = fixture.capture().await;
            assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert_affinity(
                &sent,
                None,
                if mode == "transform" {
                    Some("transformed-value")
                } else {
                    before
                },
            );
        }
    }

    #[tokio::test]
    async fn oauth_and_copilot_omit_generated_affinity_but_header_owned_auth_retains_it() {
        for mode in ["oauth", "header-owned", "copilot"] {
            let mut fixture = Fixture::new();
            fixture.auth.scheme = AuthScheme::Bearer;
            fixture.auth.secret = "sk-ant-oat-fixture-only".into();
            fixture.direct_oauth = true;
            if mode == "header-owned" {
                fixture.auth.provenance.source = opi_ai::AuthProvenanceSource::Environment {
                    name: "ANTHROPIC_AUTH_TOKEN".into(),
                };
            }
            if mode == "copilot" {
                fixture.source("github-copilot", Some(true), false);
                fixture.copilot = true;
            }
            let sent = fixture.capture().await;
            assert_affinity(
                &sent,
                (mode == "header-owned").then_some("session-complete-id"),
                None,
            );
            assert_eq!(sent.headers.contains_key("anthropic-beta"), mode == "oauth");
            assert_eq!(
                sent.headers["authorization"],
                "Bearer sk-ant-oat-fixture-only"
            );
            if mode == "copilot" {
                assert_eq!(sent.headers["x-initiator"], "user");
            }
        }
    }

    #[tokio::test]
    async fn protected_default_affinity_override_and_invalid_generated_value_still_fail_closed() {
        for mode in ["protected-override", "invalid-value"] {
            let server = MockServer::start().await;
            let mut fixture = Fixture::new();
            if mode == "protected-override" {
                fixture
                    .request
                    .extra_headers
                    .push(("x-session-affinity".into(), "caller-value".into()));
            } else {
                fixture.request.session_id = Some("invalid\nvalue".into());
            }
            let mut model = fixture.record.to_chat_model_info().unwrap();
            model.base_url = Some(server.uri());
            fixture.request.model = model.id.clone();
            let provider = AnthropicProvider::for_route(
                fixture.record.provider,
                vec![model],
                None,
                Default::default(),
                Arc::new(opi_ai::http::HttpClient::new()),
                false,
            );
            let failure =
                opi_ai::ChatStream::new(provider.stream_prepared(fixture.request, fixture.auth))
                    .result()
                    .await
                    .expect_err("invalid headers fail before dispatch");
            assert!(
                matches!(failure.cause(), ProviderError::RequestFailed(_)),
                "{mode}: {:?}",
                failure.cause()
            );
            assert!(server.received_requests().await.unwrap().is_empty());
        }
    }
}
