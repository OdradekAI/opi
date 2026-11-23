//! Google 4.0 Gemini adapter assertions.

use super::*;
use opi_ai::WireApi;
use opi_ai::api::gemini::{
    GeminiBodyBuilder, GoogleApiThinkingLevel, GoogleGenerativeAiChatApi,
    GoogleGenerativeAiOptions, GoogleThinkingConfig, GoogleToolChoice,
};
use opi_ai::message::{AssistantContent, OutputContent, ToolCall, ToolDef, ToolResultMessage};
use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMapping};
use opi_ai::providers::gemini::GeminiProvider;
use opi_ai::stream::StopReason;
use opi_ai::transcript::Context;

fn model_40(id: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        WireApi::GoogleGenerativeAi,
        ModelCapabilities::new(1_000_000, 65_536)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

fn plain_request_40(model_id: &str, thinking: ThinkingConfig) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("gemini:{model_id}"),
        context: opi_ai::transcript::normalize_context(Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context"),
        max_tokens: Some(64),
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

fn native_40(
    thinking: Option<GoogleThinkingConfig>,
    tool_choice: Option<GoogleToolChoice>,
) -> GoogleGenerativeAiOptions {
    GoogleGenerativeAiOptions::native(thinking, tool_choice)
}

fn body_40(provider: &impl GeminiBodyBuilder, request: &Request) -> serde_json::Value {
    provider
        .build_request_body(request)
        .expect("google body compiles")
}

fn thinking_config_of(body: &serde_json::Value) -> &serde_json::Value {
    body["generationConfig"]
        .get("thinkingConfig")
        .expect("thinkingConfig is present")
}

#[test]
fn google_native_level_mode_uses_thinking_level_enum() {
    let provider = GeminiProvider::new(None)
        .with_models(vec![model_40("gemini-3-pro-preview")])
        .with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: None,
                level: Some(GoogleApiThinkingLevel::High),
            }),
            None,
        ));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({
            "includeThoughts": true,
            "thinkingLevel": "HIGH",
        })
    );
}

#[test]
fn google_native_budget_mode_sends_budget_with_dynamic_minus_one() {
    for (budget, expected) in [(-1i64, -1i64), (0, 0), (2048, 2048)] {
        let provider = GeminiProvider::new(None).with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(budget),
                level: None,
            }),
            None,
        ));
        let body = body_40(
            &provider,
            &plain_request_40("gemini-2.5-flash", ThinkingConfig::default()),
        );
        assert_eq!(
            thinking_config_of(&body),
            &serde_json::json!({
                "includeThoughts": true,
                "thinkingBudget": expected,
            }),
            "budget {budget}"
        );
    }
}

#[test]
fn google_native_level_takes_precedence_over_budget() {
    let provider = GeminiProvider::new(None)
        .with_models(vec![model_40("gemini-3-flash-preview")])
        .with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: true,
                budget_tokens: Some(4096),
                level: Some(GoogleApiThinkingLevel::Low),
            }),
            None,
        ));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-3-flash-preview", ThinkingConfig::default()),
    );
    let config = thinking_config_of(&body);
    assert_eq!(config.get("thinkingLevel"), Some(&serde_json::json!("LOW")));
    assert!(
        config.get("thinkingBudget").is_none(),
        "budget is not sent alongside a level"
    );
}

#[test]
fn google_native_disabled_budget_mode_sends_zero() {
    let provider = GeminiProvider::new(None).with_native_options(native_40(
        Some(GoogleThinkingConfig {
            enabled: false,
            budget_tokens: None,
            level: None,
        }),
        None,
    ));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-2.5-flash", ThinkingConfig::default()),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({ "thinkingBudget": 0 })
    );
}

#[test]
fn google_native_disabled_level_mode_uses_off_fallback_level() {
    // Without an off mapping the pi fallback is budget zero.
    let provider = GeminiProvider::new(None)
        .with_models(vec![model_40("gemini-3-pro-preview")])
        .with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: None,
            }),
            None,
        ));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({ "thinkingBudget": 0 })
    );

    // A mapped off level takes the level wire instead.
    let model = model_40("gemini-3-pro-preview").with_thinking_level_map(
        opi_ai::model_info::ThinkingLevelMap::reasoning_default().with_mapping(
            ThinkingLevel::None,
            ThinkingLevelMapping::Mapped("minimal".into()),
        ),
    );
    let provider = GeminiProvider::new(None)
        .with_models(vec![model])
        .with_native_options(native_40(
            Some(GoogleThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: None,
            }),
            None,
        ));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-3-pro-preview", ThinkingConfig::default()),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({ "thinkingLevel": "MINIMAL" })
    );
}

#[test]
fn google_native_omitted_thinking_sends_no_config() {
    let provider = GeminiProvider::new(None).with_native_options(native_40(None, None));
    let body = body_40(
        &provider,
        &plain_request_40("gemini-2.5-pro", ThinkingConfig::default()),
    );
    assert!(
        body["generationConfig"].get("thinkingConfig").is_none(),
        "native omission leaves the wire without thinkingConfig: {body}"
    );
}

#[test]
fn google_native_overrides_request_thinking() {
    let provider = GeminiProvider::new(None).with_native_options(native_40(
        Some(GoogleThinkingConfig {
            enabled: true,
            budget_tokens: Some(2048),
            level: None,
        }),
        None,
    ));
    let body = body_40(
        &provider,
        &plain_request_40(
            "gemini-2.5-flash",
            ThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: ThinkingLevel::None,
            },
        ),
    );
    assert_eq!(
        thinking_config_of(&body).get("includeThoughts"),
        Some(&serde_json::json!(true))
    );
}

fn tool_request_40(model_id: &str, tools: Vec<ToolDef>) -> Request {
    let mut request = plain_request_40(model_id, ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text { text: "go".into() }],
            timestamp_ms: 0,
        })],
        tools,
    })
    .expect("valid context");
    request
}

fn plain_tool_40() -> ToolDef {
    ToolDef {
        name: "lookup".into(),
        description: "look".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "note": {"type": "string"},
            },
            "required": ["path"],
        }),
        constrained_sampling: None,
    }
}

#[test]
fn google_native_tool_choice_modes_reach_the_wire() {
    for (choice, mode) in [
        (GoogleToolChoice::Auto, "AUTO"),
        (GoogleToolChoice::None_, "NONE"),
        (GoogleToolChoice::Any, "ANY"),
    ] {
        let provider = GeminiProvider::new(None).with_native_options(native_40(None, Some(choice)));
        let body = body_40(
            &provider,
            &tool_request_40("gemini-2.5-flash", vec![plain_tool_40()]),
        );
        assert_eq!(
            body.pointer("/toolConfig/functionCallingConfig/mode"),
            Some(&serde_json::json!(mode)),
            "tool choice {mode}"
        );
    }
}

#[test]
fn google_no_tool_choice_without_strict_sends_no_mode() {
    let provider = GeminiProvider::new(None);
    let body = body_40(
        &provider,
        &tool_request_40("gemini-2.5-flash", vec![plain_tool_40()]),
    );
    assert!(
        body.get("toolConfig").is_none(),
        "plain tools carry no forced mode: {body}"
    );
}

#[test]
fn google_tools_carry_the_original_schema_json() {
    let tool = ToolDef {
        name: "shaped".into(),
        description: "d".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "pick": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "number"},
                    ]
                }
            },
            "required": ["pick"],
        }),
        constrained_sampling: None,
    };
    let provider = GeminiProvider::new(None);
    let body = body_40(&provider, &tool_request_40("gemini-2.5-flash", vec![tool]));
    let declaration = &body["tools"][0]["functionDeclarations"][0];
    assert!(
        declaration.get("parameters").is_none(),
        "legacy OpenAPI parameters stay off the public Google wire"
    );
    let schema = declaration
        .get("parametersJsonSchema")
        .expect("original schema rides parametersJsonSchema");
    assert!(
        schema["properties"]["pick"].get("anyOf").is_some(),
        "original schema survives unmodified: {schema}"
    );
}

#[test]
fn google_gemini3_strict_prefer_selects_validated_mode() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    let mut tool = plain_tool_40();
    tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
        strict: StrictPreference::Prefer,
    });
    let provider = GeminiProvider::new(None);
    let body = body_40(
        &provider,
        &tool_request_40("gemini-3-flash-preview", vec![tool]),
    );
    assert_eq!(
        body.pointer("/toolConfig/functionCallingConfig/mode"),
        Some(&serde_json::json!("VALIDATED"))
    );
    let schema = &body["tools"][0]["functionDeclarations"][0]
        .get("parametersJsonSchema")
        .expect("strict projection keeps the original-schema carrier");
    assert_eq!(
        schema["properties"]["note"]["anyOf"],
        serde_json::json!([
            {"type": "string"},
            {"type": "null"},
        ]),
        "strict projection wraps optional properties with a null arm"
    );
    assert_eq!(
        schema["required"],
        serde_json::json!(["path", "note"]),
        "strict projection forces every property required"
    );
}

#[test]
fn google_gemini2_strict_prefer_stays_downgraded() {
    // Existing wire safety (task 20.11): gemini-2 models have no
    // validated calling mode; prefer downgrades silently.
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    let mut tool = plain_tool_40();
    tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
        strict: StrictPreference::Prefer,
    });
    let provider = GeminiProvider::new(None);
    let body = body_40(&provider, &tool_request_40("gemini-2.5-flash", vec![tool]));
    assert!(
        body.get("toolConfig").is_none(),
        "gemini-2 has no validated mode and prefers silently downgrade"
    );
}

fn history_request_40(model_id: &str, messages: Vec<Message>) -> Request {
    let mut request = plain_request_40(model_id, ThinkingConfig::default());
    request.context = opi_ai::transcript::normalize_context(Context {
        system_prompt: None,
        messages,
        tools: vec![],
    })
    .expect("valid context");
    request
}

fn tool_result_40(id: &str, name: &str, text: &str, is_error: bool) -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: id.into(),
        tool_name: name.into(),
        content: vec![OutputContent::Text { text: text.into() }],
        details: None,
        is_error,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    })
}

fn assistant_call_40(id: &str, name: &str) -> AssistantContent {
    AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: "{}".into(),
            arguments_error: None,
            signature: None,
        },
    }
}

#[test]
fn google_function_responses_use_output_and_error_keys() {
    let messages = vec![
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![
                assistant_call_40("c1", "lookup"),
                assistant_call_40("c2", "lookup"),
            ],
            ..assistant_shell_40("gemini-2.5-flash")
        }),
        tool_result_40("c1", "lookup", "found it", false),
        tool_result_40("c2", "lookup", "boom", true),
        Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "next".into(),
            }],
            timestamp_ms: 0,
        }),
    ];
    let provider = GeminiProvider::new(None);
    let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
    let contents = body["contents"].as_array().expect("contents");
    // Consecutive tool results merge into one user turn (pi merges
    // function responses into a single user message).
    let merged = contents
        .iter()
        .filter(|c| {
            c["role"] == "user"
                && c["parts"]
                    .as_array()
                    .is_some_and(|parts| parts.iter().any(|p| p.get("functionResponse").is_some()))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        merged.len(),
        1,
        "consecutive function responses share one user turn: {contents:?}"
    );
    let responses: Vec<&serde_json::Value> = merged[0]["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .filter_map(|p| p.get("functionResponse"))
        .collect();
    assert_eq!(responses.len(), 2);
    assert_eq!(
        responses[0]["response"],
        serde_json::json!({"output": "found it"}),
        "success uses the output key"
    );
    assert_eq!(
        responses[1]["response"],
        serde_json::json!({"error": "boom"}),
        "failure uses the error key with the text"
    );
}

fn assistant_shell_40(model: &str) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        projection: None,
        content: vec![],
        api: opi_ai::ApiKind::Google,
        provider: "gemini".into(),
        deferred: None,
        model: model.into(),
        response_model: None,
        wire: Some(WireApi::GoogleGenerativeAi),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
    }
}

#[test]
fn google_gemini3_tool_calls_and_responses_carry_normalized_ids() {
    let messages = vec![
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![assistant_call_40("call id/with spaces!!", "lookup")],
            ..assistant_shell_40("gemini-3-pro-preview")
        }),
        tool_result_40("call id/with spaces!!", "lookup", "ok", false),
    ];
    let provider = GeminiProvider::new(None);
    let body = body_40(
        &provider,
        &history_request_40("gemini-3-pro-preview", messages),
    );
    let call_part = &body["contents"][0]["parts"][0]["functionCall"];
    assert_eq!(
        call_part.get("id"),
        Some(&serde_json::json!("call_id_with_spaces__")),
        "gemini-3 normalizes tool ids to [a-zA-Z0-9_-] within 64"
    );
    let response_part = &body["contents"][1]["parts"][0]["functionResponse"];
    assert_eq!(
        response_part.get("id"),
        Some(&serde_json::json!("call_id_with_spaces__"))
    );

    // Gemini 2 keeps the legacy shape: no ids on either side.
    let messages = vec![
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![assistant_call_40("c1", "lookup")],
            ..assistant_shell_40("gemini-2.5-flash")
        }),
        tool_result_40("c1", "lookup", "ok", false),
    ];
    let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
    assert!(
        body["contents"][0]["parts"][0]["functionCall"]
            .get("id")
            .is_none(),
        "gemini-2 does not require tool call ids"
    );
    assert!(
        body["contents"][1]["parts"][0]["functionResponse"]
            .get("id")
            .is_none()
    );
}

fn image_result_40(id: &str, name: &str) -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: id.into(),
        tool_name: name.into(),
        content: vec![OutputContent::Image {
            media_type: opi_ai::message::MediaType::Png,
            source: opi_ai::message::ImageSource::Base64 {
                data: "aGk=".into(),
            },
        }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    })
}

#[test]
fn google_gemini3_nests_images_inside_function_responses() {
    let messages = vec![
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![assistant_call_40("c1", "lookup")],
            ..assistant_shell_40("gemini-3-flash-preview")
        }),
        image_result_40("c1", "lookup"),
    ];
    let provider = GeminiProvider::new(None);
    let body = body_40(
        &provider,
        &history_request_40("gemini-3-flash-preview", messages),
    );
    let response = &body["contents"][1]["parts"][0]["functionResponse"];
    assert!(
        response
            .get("parts")
            .is_some_and(|p| p.as_array().is_some_and(|a| !a.is_empty())),
        "gemini-3 multimodal function responses nest image parts: {response}"
    );
    assert_eq!(
        body["contents"].as_array().expect("contents").len(),
        2,
        "no separate trailing image turn on gemini-3: {body}"
    );
}

#[test]
fn google_gemini2_images_ride_a_separate_user_turn() {
    let messages = vec![
        Message::Assistant(opi_ai::message::AssistantMessage {
            content: vec![assistant_call_40("c1", "lookup")],
            ..assistant_shell_40("gemini-2.5-flash")
        }),
        image_result_40("c1", "lookup"),
    ];
    let provider = GeminiProvider::new(None);
    let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
    let contents = body["contents"].as_array().expect("contents");
    let image_turn = contents
        .iter()
        .find(|c| {
            c["parts"].as_array().is_some_and(|parts| {
                parts
                    .iter()
                    .any(|p| p.get("text") == Some(&serde_json::json!("Tool result image:")))
            })
        })
        .expect("separate image turn follows the results");
    assert!(
        image_turn["parts"]
            .as_array()
            .expect("parts")
            .iter()
            .any(|p| p.get("inlineData").is_some()),
        "image data rides the separate turn"
    );
}

#[test]
fn google_thought_signatures_replay_same_model_only() {
    let same = opi_ai::message::AssistantMessage {
        content: vec![
            AssistantContent::Text {
                text: "answer".into(),
                signature: Some("c2lnMQ==".into()),
            },
            AssistantContent::Thinking {
                thinking: "hm".into(),
                signature: Some("c2lnMg==".into()),
            },
            AssistantContent::ToolCall {
                tool_call: ToolCall {
                    id: "c1".into(),
                    name: "lookup".into(),
                    arguments: "{}".into(),
                    arguments_error: None,
                    signature: Some("c2lnMw==".into()),
                },
            },
        ],
        ..assistant_shell_40("gemini-2.5-flash")
    };
    let messages = vec![
        Message::Assistant(same),
        tool_result_40("c1", "lookup", "ok", false),
    ];
    let provider = GeminiProvider::new(None);
    let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
    let parts = body["contents"][0]["parts"].as_array().expect("parts");
    assert_eq!(parts.len(), 3, "text, thinking and call all ride parts");
    assert_eq!(
        parts[0].get("thoughtSignature"),
        Some(&serde_json::json!("c2lnMQ=="))
    );
    assert_eq!(parts[1].get("thought"), Some(&serde_json::json!(true)));
    assert_eq!(
        parts[1].get("thoughtSignature"),
        Some(&serde_json::json!("c2lnMg=="))
    );
    assert_eq!(
        parts[2].get("thoughtSignature"),
        Some(&serde_json::json!("c2lnMw==")),
        "the signature rides the part beside functionCall"
    );

    // Cross-model history drops all thinking and unusable signatures.
    let foreign = opi_ai::message::AssistantMessage {
        content: vec![
            AssistantContent::Text {
                text: "answer".into(),
                signature: Some("c2lnMQ==".into()),
            },
            AssistantContent::Thinking {
                thinking: "hm".into(),
                signature: Some("c2lnMg==".into()),
            },
        ],
        ..assistant_shell_40("gemini-2.5-pro")
    };
    let messages = vec![Message::Assistant(foreign)];
    let body = body_40(&provider, &history_request_40("gemini-2.5-flash", messages));
    let parts = body["contents"][0]["parts"].as_array().expect("parts");
    // Cross-identity thinking is dropped by the canonical replay
    // projection (task 20.7 contract) before this compiler runs; the
    // text part remains without its unusable signature.
    assert_eq!(parts.len(), 1);
    assert!(parts.iter().all(|p| p.get("thoughtSignature").is_none()));
    assert!(parts[0].get("thought").is_none());

    // Invalid base64 signatures never reach the wire even same-model.
    let invalid = opi_ai::message::AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "answer".into(),
            signature: Some("not base64!!".into()),
        }],
        ..assistant_shell_40("gemini-2.5-flash")
    };
    let body = body_40(
        &provider,
        &history_request_40("gemini-2.5-flash", vec![Message::Assistant(invalid)]),
    );
    assert!(
        body["contents"][0]["parts"][0]
            .get("thoughtSignature")
            .is_none(),
        "non-base64 signatures are dropped"
    );

    // Empty text with a signature is preserved (pi keeps signature-only
    // parts; dropping them breaks the reasoning chain).
    let empty = opi_ai::message::AssistantMessage {
        content: vec![AssistantContent::Text {
            text: "  ".into(),
            signature: Some("c2lnNA==".into()),
        }],
        ..assistant_shell_40("gemini-2.5-flash")
    };
    let body = body_40(
        &provider,
        &history_request_40("gemini-2.5-flash", vec![Message::Assistant(empty)]),
    );
    assert!(
        body["contents"][0]["parts"][0]
            .get("thoughtSignature")
            .is_some(),
        "a signature-only part survives"
    );
}

#[test]
fn google_neutral_unresolvable_level_fails_typed() {
    let model = model_40("gemini-2.5-flash")
        .with_thinking_level_map(opi_ai::model_info::ThinkingLevelMap::disabled());
    let provider = GeminiProvider::new(None).with_models(vec![model]);
    let error = provider
        .build_request_body(&plain_request_40(
            "gemini-2.5-flash",
            ThinkingConfig {
                enabled: true,
                budget_tokens: None,
                level: ThinkingLevel::Medium,
            },
        ))
        .expect_err("unsupported mapping fails before dispatch");
    assert!(
        matches!(error, ProviderError::Config { .. }),
        "got {error:?}"
    );
}

#[test]
fn google_neutral_level_mode_projects_resolved_level() {
    let provider = GeminiProvider::new(None).with_models(vec![model_40("gemini-3-pro-preview")]);
    let body = body_40(
        &provider,
        &plain_request_40(
            "gemini-3-pro-preview",
            ThinkingConfig {
                enabled: true,
                budget_tokens: None,
                level: ThinkingLevel::Medium,
            },
        ),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({
            "includeThoughts": true,
            "thinkingLevel": "MEDIUM",
        })
    );
}

#[test]
fn google_neutral_budget_mode_uses_model_tables_and_dynamic_fallback() {
    for (model_id, level, expected) in [
        ("gemini-2.5-pro", ThinkingLevel::High, 32768i64),
        ("gemini-2.5-pro", ThinkingLevel::Minimal, 128),
        ("gemini-2.5-flash", ThinkingLevel::Medium, 8192),
        ("gemini-2.5-flash-lite", ThinkingLevel::High, 24576),
        ("gemini-2.5-flash-lite", ThinkingLevel::Minimal, 512),
        // Budget-mode models outside the 2.5 families fall back to the
        // dynamic `-1` (level-mode models like gemini-9 never reach the
        // budget table).
        ("gemini-2.0-flash", ThinkingLevel::Medium, -1),
    ] {
        let provider = table_provider_40();
        let body = body_40(
            &provider,
            &plain_request_40(
                model_id,
                ThinkingConfig {
                    enabled: true,
                    budget_tokens: None,
                    level,
                },
            ),
        );
        assert_eq!(
            thinking_config_of(&body).get("thinkingBudget"),
            Some(&serde_json::json!(expected)),
            "{model_id} {level:?}"
        );
    }

    // An explicit neutral budget wins over the table.
    let body = body_40(
        &provider_neutral(),
        &plain_request_40(
            "gemini-2.5-flash",
            ThinkingConfig {
                enabled: true,
                budget_tokens: Some(1234),
                level: ThinkingLevel::Medium,
            },
        ),
    );
    assert_eq!(
        thinking_config_of(&body).get("thinkingBudget"),
        Some(&serde_json::json!(1234))
    );
}

fn provider_neutral() -> GeminiProvider {
    GeminiProvider::new(None)
}

fn table_provider_40() -> GeminiProvider {
    let ids = [
        "gemini-2.5-pro",
        "gemini-2.5-flash",
        "gemini-2.5-flash-lite",
        "gemini-2.0-flash",
    ];
    GeminiProvider::new(None).with_models(ids.iter().map(|id| model_40(id)).collect())
}

#[test]
fn google_neutral_disabled_on_reasoning_sends_disabled_config() {
    let provider = GeminiProvider::new(None);
    let body = body_40(
        &provider,
        &plain_request_40(
            "gemini-2.5-flash",
            ThinkingConfig {
                enabled: false,
                budget_tokens: None,
                level: ThinkingLevel::None,
            },
        ),
    );
    assert_eq!(
        thinking_config_of(&body),
        &serde_json::json!({ "thinkingBudget": 0 })
    );
}

#[test]
fn google_from_simple_maps_tool_choice_only() {
    let simple = opi_ai::api::contract::SimpleOptions {
        tool_choice: Some(opi_ai::api::contract::SimpleToolChoice::Auto),
        ..Default::default()
    };
    let options = GoogleGenerativeAiOptions::from_simple(&simple);
    assert!(options.is_simple());
    assert!(matches!(options.tool_choice, Some(GoogleToolChoice::Auto)));
    assert!(options.thinking.is_none());
}

#[test]
fn google_direct_adapter_rejects_wrong_wire_records() {
    let wrong = ModelInfo::new(
        "gpt-4o",
        "GPT-4o",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(1000, 100),
    );
    let provider = std::sync::Arc::new(GeminiProvider::new(None));
    let error = GoogleGenerativeAiChatApi::new(provider, wrong).expect_err("wrong wire");
    assert!(matches!(error, ProviderError::Config { .. }));
}

fn text_sse_40() -> String {
    let chunk = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{"text": "hi"}],
            },
            "finishReason": "STOP",
        }],
        "usageMetadata": {
            "promptTokenCount": 10,
            "candidatesTokenCount": 3,
            "thoughtsTokenCount": 7,
            "cachedContentTokenCount": 4,
            "totalTokenCount": 20,
        },
        "responseId": "resp-1",
    });
    format!("data: {chunk}\n\n")
}

async fn drive_40(provider: GeminiProvider, request: Request) -> Vec<AssistantStreamEvent> {
    let mut stream = provider.stream_prepared(request, opi_ai::test_support::resolved_auth());
    let mut events = Vec::new();
    while let Some(result) = stream.next().await {
        events.push(result.expect("stream event"));
    }
    events
}

fn stream_request_40() -> Request {
    plain_request_40("gemini-2.5-flash", ThinkingConfig::default())
}

#[tokio::test]
async fn google_stream_counts_thoughts_in_output_and_reasoning() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_40())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let events = drive_40(provider, stream_request_40()).await;
    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("done");
    // input = prompt - cached; output = candidates + thoughts;
    // cacheRead = cached; reasoning = thoughts.
    assert_eq!(done.usage.input_tokens, 6);
    assert_eq!(done.usage.output_tokens, 10);
    assert_eq!(done.usage.cache_read_tokens, 4);
    assert_eq!(done.usage.reasoning_tokens, Some(7));
    assert_eq!(done.response_id.as_deref(), Some("resp-1"));
}

fn sse_40(chunks: &[serde_json::Value]) -> String {
    chunks.iter().map(|c| format!("data: {c}\n\n")).collect()
}

#[tokio::test]
async fn google_unsigned_and_duplicate_tool_call_ids_are_regenerated() {
    let chunk = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [
                    {"functionCall": {"name": "lookup", "args": {"a": 1}}},
                    {"functionCall": {"name": "lookup", "args": {"b": 2}}},
                ],
            },
            "finishReason": "STOP",
        }],
    });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_40(&[chunk]))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let events = drive_40(provider, stream_request_40()).await;
    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("done");
    let ids: Vec<&str> = done
        .content
        .iter()
        .filter_map(|c| match c {
            AssistantContent::ToolCall { tool_call } => Some(tool_call.id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1], "duplicate or missing ids never collide");
    assert!(
        ids.iter().all(|id| id.starts_with("lookup_")),
        "generated ids derive from the tool name: {ids:?}"
    );
    // Tool calls plus STOP map to toolUse.
    assert_eq!(done.stop_reason, StopReason::ToolUse);
}

#[tokio::test]
async fn google_thought_parts_stream_as_thinking_with_signature() {
    let first = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [
                    {"text": "ponder", "thought": true, "thoughtSignature": "c2lnMQ=="},
                ],
            },
        }],
    });
    let second = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [
                    {"text": " more", "thought": true},
                ],
            },
            "finishReason": "STOP",
        }],
    });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_40(&[first, second]))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let events = drive_40(provider, stream_request_40()).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::ThinkingStart { .. })),
        "thought parts open thinking blocks"
    );
    let done = events
        .iter()
        .find_map(|e| match e {
            AssistantStreamEvent::Done { message, .. } => Some(message.clone()),
            _ => None,
        })
        .expect("done");
    match &done.content[0] {
        AssistantContent::Thinking {
            thinking,
            signature,
        } => {
            assert_eq!(thinking, "ponder more");
            // The signature from the first delta survives later deltas
            // that omit it.
            assert_eq!(signature.as_deref(), Some("c2lnMQ=="));
        }
        other => panic!("expected a thinking block, got {other:?}"),
    }
}

#[tokio::test]
async fn google_unknown_finish_reason_is_error_with_raw_text() {
    let chunk = serde_json::json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{"text": "partial"}]},
            "finishReason": "SAFETY",
        }],
    });
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_40(&[chunk]))
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;
    let provider = GeminiProvider::new(Some(server.uri()));
    let mut stream =
        provider.stream_prepared(stream_request_40(), opi_ai::test_support::resolved_auth());
    let mut failure = None;
    while let Some(result) = stream.next().await {
        if let Err(error) = result {
            failure = Some(error);
            break;
        }
    }
    let error = failure.expect("typed failure for an error finish");
    assert!(
        error.to_string().contains("SAFETY"),
        "raw finish reason rides the failure text: {error}"
    );
}

#[tokio::test]
async fn google_native_lifecycle_reaches_the_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(body_partial_json(serde_json::json!({
            "generationConfig": { "thinkingConfig": {
                "includeThoughts": true,
                "thinkingBudget": 2048,
            } }
        })))
        .and(query_param("alt", "sse"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_40())
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let model = model_40("gemini-2.5-flash").with_base_url(server.uri());
    let provider = std::sync::Arc::new(
        GeminiProvider::new(None)
            .with_models(vec![model.clone()])
            .with_base_url(server.uri()),
    );
    let options = native_40(
        Some(GoogleThinkingConfig {
            enabled: true,
            budget_tokens: Some(2048),
            level: None,
        }),
        None,
    );
    let api = GoogleGenerativeAiChatApi::new(provider, model.clone()).expect("valid record");
    let mut models = opi_ai::models::Models::new();
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

    let message = models
        .complete(
            opi_ai::models::ChatModelRecord::new("gemini", model),
            stream_request_40(),
            handle.options(options),
        )
        .await
        .expect("native lifecycle completes");
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.response_id.as_deref(), Some("resp-1"));
}

#[cfg(feature = "telemetry-tracing")]
#[tokio::test]
async fn google_telemetry_carrier_instruments_the_producer() {
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
    let context = opi_ai::telemetry::TelemetryContext::from_span(parent);

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(text_sse_40())
                .insert_header("content-type", "text/event-stream"),
        )
        .mount(&server)
        .await;

    let model = model_40("gemini-2.5-flash").with_base_url(server.uri());
    let provider = GeminiProvider::new(None)
        .with_models(vec![model.clone()])
        .with_base_url(server.uri());
    let seen = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = seen.clone();
    let mut request = stream_request_40();
    request.telemetry_context = Some(context);
    request.callbacks =
        opi_ai::callbacks::RequestCallbacks::new().with_async_on_payload(move |_| {
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                tracing::info_span!("payload_probe_40").in_scope(|| ());
                Ok(None)
            }
        });

    let events = drive_40(provider, request).await;
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AssistantStreamEvent::Done { .. }))
    );
    assert_eq!(seen.load(std::sync::atomic::Ordering::SeqCst), 1);
    let state = recorder.lock().unwrap();
    assert!(
        state
            .spans
            .iter()
            .any(|(name, _)| *name == "payload_probe_40"),
        "producer telemetry flows to the frozen carrier: {:?}",
        state.spans
    );
}

#[tokio::test]
async fn node5_google_explicit_model_base_is_not_given_an_extra_version() {
    for suffix in ["", "/proxy/custom", "/v1", "/v1beta/"] {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\n").insert_header("content-type","text/event-stream")).expect(1).mount(&server).await;
        let mut model = model_40("gemini-3-pro");
        model.base_url = Some(format!("{}{suffix}", server.uri()));
        let provider = std::sync::Arc::new(
            GeminiProvider::new(Some(format!("{}/factory", server.uri())))
                .with_models(vec![model.clone()]),
        );
        let api = GoogleGenerativeAiChatApi::new(provider, model.clone()).unwrap();
        let mut models = opi_ai::models::Models::new();
        let handle = models
            .register_chat_api(
                std::sync::Arc::new(api),
                std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    opi_ai::auth::AuthScheme::ApiKey,
                    secrecy::SecretString::from("fixture"),
                )),
                opi_ai::AuthProvenanceSource::Static,
            )
            .unwrap();
        models
            .complete(
                opi_ai::models::ChatModelRecord::new("gemini", model),
                plain_request_40("gemini-3-pro", Default::default()),
                handle.options(native_40(None, None)),
            )
            .await
            .unwrap();
        let sent = server.received_requests().await.unwrap();
        assert_eq!(
            sent[0].url.path(),
            format!(
                "{}/models/gemini-3-pro:streamGenerateContent",
                suffix.trim_end_matches('/')
            )
        );
    }
}

#[tokio::test]
async fn node5_google_direct_host_constructor_keeps_default_version() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(wiremock::matchers::path("/v1beta/models/gemini-2.5-flash:streamGenerateContent")).respond_with(ResponseTemplate::new(200).set_body_string("data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"ok\"}]},\"finishReason\":\"STOP\"}]}\n\n").insert_header("content-type","text/event-stream")).expect(1).mount(&server).await;
    let events: Vec<_> = GeminiProvider::new(Some(server.uri()))
        .stream_prepared(gemini_http_request(), opi_ai::test_support::resolved_auth())
        .collect()
        .await;
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Ok(AssistantStreamEvent::Done { .. })))
    );
    server.verify().await;
}

#[tokio::test]
async fn node5_google_tool_signature_capture_reload_and_real_replay() {
    let server = MockServer::start().await;
    let raw = serde_json::json!({"candidates":[{"content":{"parts":[{"functionCall":{"id":"call-1","name":"lookup","args":{}},"thoughtSignature":"c2ln"}]},"finishReason":"STOP"}]});
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(format!("data: {raw}\n\n"))
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(2)
        .mount(&server)
        .await;
    let model = model_40("gemini-3-pro");
    let api = GoogleGenerativeAiChatApi::new(
        std::sync::Arc::new(
            GeminiProvider::new(Some(server.uri())).with_models(vec![model.clone()]),
        ),
        model.clone(),
    )
    .unwrap();
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("fixture"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let first = models
        .complete(
            opi_ai::models::ChatModelRecord::new("gemini", model.clone()),
            plain_request_40("gemini-3-pro", Default::default()),
            handle.options(native_40(None, None)),
        )
        .await
        .unwrap();
    let AssistantContent::ToolCall { tool_call } = &first.content[0] else {
        panic!("captured tool")
    };
    assert_eq!(tool_call.signature.as_deref(), Some("c2ln"));
    let reload = serde_json::from_slice(&serde_json::to_vec(&first).unwrap()).unwrap();
    let mut request = plain_request_40("gemini-3-pro", Default::default());
    request.context = opi_ai::transcript::normalize_context(Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "go".into() }],
                timestamp_ms: 0,
            }),
            Message::Assistant(reload),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "lookup".into(),
                content: vec![],
                details: None,
                is_error: false,
                not_executed: None,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
    })
    .unwrap();
    models
        .complete_simple(
            opi_ai::models::ChatModelRecord::new("gemini", model),
            request,
            Default::default(),
        )
        .await
        .unwrap();
    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[1].body).unwrap();
    let parts = body["contents"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|content| content["parts"].as_array().unwrap());
    assert!(
        parts
            .into_iter()
            .any(|part| part.get("functionCall").is_some() && part["thoughtSignature"] == "c2ln")
    );
}
