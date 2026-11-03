use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::bedrock::BedrockProvider;
use opi_ai::gemini::GeminiProvider;
use opi_ai::http::HttpClient;
use opi_ai::message::ToolDef;
use opi_ai::mistral::mistral_provider;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::openrouter::openrouter_provider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::transcript::{
    Context, SystemMessage, SystemSection, ToolReference, TranscriptContext, normalize_context,
};
use opi_ai::vertex::VertexProvider;
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const CURRENT_PROMPT: &str = "Base policy.\n\nUse current declarations.\n\nApply the latest policy.\n\nCurrent scope.\n\nKeep this section.\n\nFinal section.";

fn current_read() -> ToolDef {
    ToolDef {
        name: "read".into(),
        description: "Read selected paths with explicit modes".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "minLength": 1},
                "modes": {
                    "type": "array",
                    "items": {"type": "string", "enum": ["inspect", "apply"]},
                    "minItems": 1,
                    "uniqueItems": true
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "force": {"type": "boolean"},
                        "limit": {"type": "integer", "minimum": 1, "maximum": 10}
                    },
                    "required": ["force"],
                    "additionalProperties": false
                }
            },
            "required": ["path", "modes"],
            "additionalProperties": false
        }),
        constrained_sampling: None,
    }
}

fn current_status() -> ToolDef {
    ToolDef {
        name: "status".into(),
        description: "Inspect the retained status declaration".into(),
        input_schema: json!({
            "type": "object",
            "properties": {"detail": {"type": "string", "enum": ["brief", "full"]}},
            "required": ["detail"],
            "additionalProperties": false
        }),
        constrained_sampling: None,
    }
}

fn current_write() -> ToolDef {
    ToolDef {
        name: "write".into(),
        description: "Write a nonempty batch of explicit changes".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "changes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "minLength": 1},
                            "contents": {"type": "string"}
                        },
                        "required": ["path", "contents"],
                        "additionalProperties": false
                    },
                    "minItems": 1
                },
                "dry_run": {"type": "boolean", "default": true}
            },
            "required": ["changes"],
            "additionalProperties": false
        }),
        constrained_sampling: None,
    }
}

fn updated_chronology() -> TranscriptContext {
    let mut context = normalize_context(Context {
        system_prompt: Some("Base policy.".into()),
        messages: vec![super::user("What changed?")],
        tools: vec![
            super::tool("read", "Obsolete read declaration"),
            super::tool("obsolete", "This tool must leave the request"),
            current_status(),
        ],
    })
    .expect("initial declarations compile");
    context
        .append_system_change(SystemMessage {
            content: "Use current declarations.".into(),
            sections: vec![
                SystemSection {
                    name: "scope".into(),
                    value: Some("Obsolete scope.".into()),
                },
                SystemSection {
                    name: "removed".into(),
                    value: Some("This section must leave the prompt.".into()),
                },
                SystemSection {
                    name: "retained".into(),
                    value: Some("Keep this section.".into()),
                },
            ],
            ..super::change()
        })
        .expect("first update is admitted");
    context
        .append_system_change(SystemMessage {
            content: "Apply the latest policy.".into(),
            sections: vec![
                SystemSection {
                    name: "scope".into(),
                    value: Some("Current scope.".into()),
                },
                SystemSection {
                    name: "removed".into(),
                    value: None,
                },
                SystemSection {
                    name: "final".into(),
                    value: Some("Final section.".into()),
                },
            ],
            tools_removed: vec![ToolReference {
                name: "obsolete".into(),
            }],
            tools_added: vec![current_read(), current_write()],
            ..super::change()
        })
        .expect("replacement and new declarations compile");
    context
}

struct WireView {
    prompt: &'static str,
    tools: &'static str,
    declaration: &'static str,
    schema: &'static str,
    strict: Option<Value>,
}

const CHAT: WireView = WireView {
    prompt: "/messages/0/content",
    tools: "/tools",
    declaration: "/function",
    schema: "/parameters",
    strict: None,
};

const RESPONSES: WireView = WireView {
    prompt: "/input/0/content",
    tools: "/tools",
    declaration: "",
    schema: "/parameters",
    strict: None,
};

const GEMINI: WireView = WireView {
    prompt: "/systemInstruction/parts/0/text",
    tools: "/tools/0/functionDeclarations",
    declaration: "",
    // Task 20.40: the original JSON Schema rides parametersJsonSchema.
    schema: "/parametersJsonSchema",
    strict: None,
};

fn assert_projection(
    context: &TranscriptContext,
    model: &str,
    build: impl FnOnce(&Request) -> Result<Value, ProviderError>,
    view: WireView,
) {
    let request = Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context: context.clone(),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        // Cache-marker placement has separate tests; this matrix isolates
        // chronology and the exact declaration values supplied to the wire.
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    };
    let canonical = serde_json::to_vec(&request.context).unwrap();
    let body = build(&request).unwrap_or_else(|error| panic!("{model}: {error}"));
    assert_eq!(
        serde_json::to_vec(&request.context).unwrap(),
        canonical,
        "{model}: projection must not rewrite the canonical chronology"
    );
    assert_eq!(
        body.pointer(view.prompt),
        Some(&json!(CURRENT_PROMPT)),
        "{model}: append, replacement, removal and section order reach the prompt"
    );
    let tools = body
        .pointer(view.tools)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{model}: current tool declarations are missing: {body}"));
    assert_eq!(tools.len(), 3, "{model}: removed tools must not survive");
    for (actual, expected) in tools
        .iter()
        .zip([current_read(), current_status(), current_write()])
    {
        let declaration = actual
            .pointer(view.declaration)
            .unwrap_or_else(|| panic!("{model}: malformed declaration: {actual}"));
        assert_eq!(declaration["name"], expected.name, "{model}: tool order");
        assert_eq!(
            declaration["description"], expected.description,
            "{model}: current description"
        );
        assert_eq!(
            declaration.pointer(view.schema),
            Some(&expected.input_schema),
            "{model}: complete schema value, including nested declarations"
        );
        assert_eq!(
            declaration.get("strict"),
            view.strict.as_ref(),
            "{model}: preserve the wire's default strict declaration contract"
        );
    }
}

#[test]
fn independent_adapters_project_current_prompt_and_complete_tool_schemas() {
    let context = updated_chronology();
    let client = Arc::new(HttpClient::new());

    // All six current serializers collapse mid-conversation system changes.
    // The prompt oracle is a worked literal, not the transcript renderer.
    assert_projection(
        &context,
        "anthropic:claude-sonnet-4-5-20250514",
        |request| AnthropicProvider::new(None).build_request_body(request),
        WireView {
            prompt: "/system",
            tools: "/tools",
            declaration: "",
            schema: "/input_schema",
            strict: None,
        },
    );
    assert_projection(
        &context,
        "openai:gpt-4o",
        |request| OpenAiChatProvider::new(None).build_request_body(request),
        CHAT,
    );
    assert_projection(
        &context,
        "openai-responses:gpt-4o",
        |request| OpenAiResponsesProvider::new(None).build_request_body(request),
        RESPONSES,
    );
    let codex = OpenAiCodexResponsesProvider::new(
        None,
        vec![ModelInfo::new(
            "gpt-5.4",
            "GPT-5.4",
            WireApi::OpenAiCodexResponses,
            ModelCapabilities::new(272_000, 128_000).with_streaming(true),
        )],
        client.clone(),
    );
    assert_projection(
        &context,
        "openai-codex:gpt-5.4",
        |request| codex.build_request_body(request),
        WireView {
            // Codex keeps initial instructions outside input and emits null strict.
            prompt: "/instructions",
            strict: Some(Value::Null),
            ..RESPONSES
        },
    );
    assert_projection(
        &context,
        "gemini:gemini-2.5-flash",
        |request| GeminiProvider::new(None).build_request_body(request),
        GEMINI,
    );
    assert_projection(
        &context,
        "bedrock:anthropic.claude-sonnet-4",
        |request| BedrockProvider::new(None, client).build_converse_body(request),
        WireView {
            prompt: "/system/0/text",
            tools: "/toolConfig/tools",
            declaration: "/toolSpec",
            schema: "/inputSchema/json",
            strict: None,
        },
    );
}

#[test]
fn actual_facades_and_factories_project_current_prompt_and_complete_tool_schemas() {
    let context = updated_chronology();
    let endpoint = "http://127.0.0.1:1";
    let azure =
        AzureOpenAIProvider::new(Some(endpoint.into()), "projection-deployment".into(), None)
            .expect("explicit Azure deployment is admitted");
    assert_projection(
        &context,
        "azure:projection-deployment",
        |request| azure.build_request_body(request),
        CHAT,
    );
    let vertex = VertexProvider::new(
        "projection-project".into(),
        "us-central1".into(),
        Some(endpoint.into()),
    );
    assert_projection(
        &context,
        "vertex:gemini-2.5-flash",
        |request| vertex.build_request_body(request),
        GEMINI,
    );
    // These are the real profile factories. Their shared serializer does not
    // replace constructing and checking the profile that a caller receives.
    let mistral = mistral_provider(Some(endpoint.into()));
    assert_projection(
        &context,
        "mistral:mistral-large-latest",
        |request| mistral.build_request_body(request),
        CHAT,
    );
    let openrouter = openrouter_provider(Some(endpoint.into()));
    assert_projection(
        &context,
        "openrouter:openai/gpt-4o",
        |request| openrouter.build_request_body(request),
        CHAT,
    );
}

#[derive(Clone, Copy)]
enum NativeToolChange {
    Add,
    Remove,
    Redefine,
}

fn native_tool(name: &str, description: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: description.into(),
        input_schema: json!({"type":"object", "properties":{"query":{"type":"string"}}, "required":["query"], "additionalProperties":false}),
        constrained_sampling: None,
    }
}

fn native_tool_context(change: NativeToolChange, body_change: bool) -> TranscriptContext {
    let (tools_added, tools_removed) = match change {
        NativeToolChange::Add => (vec![native_tool("beta", "new beta")], vec![]),
        NativeToolChange::Remove => (
            vec![],
            vec![ToolReference {
                name: "alpha".into(),
            }],
        ),
        NativeToolChange::Redefine => (vec![native_tool("alpha", "replacement alpha")], vec![]),
    };
    normalize_context(Context {
        system_prompt: None,
        tools: vec![],
        messages: vec![
            opi_ai::message::Message::System(SystemMessage {
                content: "initial policy".into(),
                tools_added: vec![native_tool("alpha", "initial alpha")],
                timestamp_ms: 0,
                ..Default::default()
            }),
            super::user("before addition"),
            opi_ai::message::Message::System(SystemMessage {
                content: if body_change {
                    "later policy".into()
                } else {
                    String::new()
                },
                tools_added,
                tools_removed,
                timestamp_ms: 2,
                ..Default::default()
            }),
            super::user("after addition"),
        ],
    })
    .unwrap()
}

fn native_tool_request(model: &str, context: TranscriptContext) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.into(),
        context,
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

fn chat_with_native_tools(mid_system: Option<bool>, additions: Option<bool>) -> OpenAiChatProvider {
    use opi_ai::model_info::{OpenAiCompletionsCompat, WireCompat};
    let model = ModelInfo::new(
        "native-tool-model",
        "native tools",
        WireApi::OpenAiCompletions,
        ModelCapabilities::new(8192, 1024).with_streaming(true),
    )
    .with_compat(WireCompat::OpenAiCompletions(OpenAiCompletionsCompat {
        supports_mid_convo_system_messages: mid_system,
        supports_mid_convo_tool_additions: additions,
        ..Default::default()
    }))
    .unwrap();
    OpenAiChatProvider::for_route(
        None,
        "native-chat".into(),
        Default::default(),
        vec![model],
        Arc::new(HttpClient::new()),
    )
}

fn chat_tool_names(body: &Value) -> Vec<&str> {
    body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|tool| tool["function"]["name"].as_str().unwrap())
        .collect()
}

fn response_tool_names(body: &Value) -> Vec<&str> {
    body["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect()
}

#[test]
fn native_chat_tool_additions_require_both_flags_and_keep_position() {
    let request = native_tool_request(
        "native-chat:native-tool-model",
        native_tool_context(NativeToolChange::Add, false),
    );
    let canonical = serde_json::to_value(&request.context).unwrap();
    let provider = chat_with_native_tools(Some(true), Some(true));
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(chat_tool_names(&body), ["alpha"]);
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4);
    assert_eq!(
        messages[0],
        json!({"role":"system", "content":"initial policy"})
    );
    assert_eq!(
        messages[1],
        json!({"role":"user", "content":"before addition"})
    );
    assert_eq!(messages[2]["role"], "system");
    assert_eq!(messages[2]["tools"][0]["function"]["name"], "beta");
    assert_eq!(
        messages[2]["tools"][0]["function"]["parameters"],
        native_tool("beta", "new beta").input_schema
    );
    assert_eq!(
        messages[3],
        json!({"role":"user", "content":"after addition"})
    );
    let mut strict = request.clone();
    strict.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
    assert_eq!(provider.build_request_body(&strict).unwrap(), body);
    for (mid, additions) in [
        (None, None),
        (Some(false), Some(true)),
        (Some(true), Some(false)),
    ] {
        let provider = chat_with_native_tools(mid, additions);
        let collapsed = provider.build_request_body(&request).unwrap();
        assert_eq!(chat_tool_names(&collapsed), ["alpha", "beta"]);
        assert_eq!(collapsed["messages"].as_array().unwrap().len(), 3);
        assert!(
            collapsed["messages"]
                .as_array()
                .unwrap()
                .iter()
                .all(|message| message.get("tools").is_none())
        );
        assert!(matches!(
            provider.build_request_body(&strict),
            Err(ProviderError::Config(_))
        ));
    }
    assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
}

#[test]
fn native_chat_non_additive_tool_changes_collapse_or_reject_strict() {
    let provider = chat_with_native_tools(Some(true), Some(true));
    for change in [NativeToolChange::Remove, NativeToolChange::Redefine] {
        let mut request = native_tool_request(
            "native-chat:native-tool-model",
            native_tool_context(change, false),
        );
        let canonical = serde_json::to_value(&request.context).unwrap();
        let body = provider.build_request_body(&request).unwrap();
        assert!(
            body["messages"]
                .as_array()
                .unwrap()
                .iter()
                .all(|message| message.get("tools").is_none())
        );
        match change {
            NativeToolChange::Remove => assert!(chat_tool_names(&body).is_empty()),
            NativeToolChange::Redefine => {
                assert_eq!(chat_tool_names(&body), ["alpha"]);
                assert_eq!(
                    body["tools"][0]["function"]["description"],
                    "replacement alpha"
                );
            }
            NativeToolChange::Add => unreachable!(),
        }
        request.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
        let error = provider.build_request_body(&request).unwrap_err();
        assert!(matches!(error, ProviderError::Config(_)));
        assert!(error.to_string().contains("strict transcript projection"));
        assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
    }
}

#[derive(Clone, Copy)]
enum NativeResponseWire {
    Responses,
    Codex,
}

fn response_source(
    mid_system: bool,
    additional: bool,
    search: bool,
) -> opi_ai::model_info::OpenAiResponsesSourceCompat {
    opi_ai::model_info::OpenAiResponsesSourceCompat {
        supports_mid_convo_system_messages: Some(mid_system),
        supports_additional_tools: Some(additional),
        supports_tool_search: Some(search),
        ..Default::default()
    }
}

fn response_model(
    wire: WireApi,
    source: opi_ai::model_info::OpenAiResponsesSourceCompat,
) -> ModelInfo {
    use opi_ai::model_info::{OpenAiCodexResponsesCompat, OpenAiResponsesCompat, WireCompat};
    let compat = match wire {
        WireApi::OpenAiResponses => WireCompat::OpenAiResponses(OpenAiResponsesCompat {
            source,
            ..Default::default()
        }),
        WireApi::AzureOpenAiResponses => WireCompat::AzureOpenAiResponses(OpenAiResponsesCompat {
            source,
            ..Default::default()
        }),
        WireApi::OpenAiCodexResponses => {
            WireCompat::OpenAiCodexResponses(OpenAiCodexResponsesCompat {
                source,
                ..Default::default()
            })
        }
        _ => unreachable!(),
    };
    ModelInfo::new(
        "native-tool-model",
        "native tools",
        wire,
        ModelCapabilities::new(8192, 1024).with_streaming(true),
    )
    .with_compat(compat)
    .unwrap()
}

fn response_wire_spec(wire: NativeResponseWire) -> &'static str {
    match wire {
        NativeResponseWire::Responses => "openai-responses:native-tool-model",
        NativeResponseWire::Codex => "openai-codex:native-tool-model",
    }
}

fn native_response_body(
    wire: NativeResponseWire,
    source: opi_ai::model_info::OpenAiResponsesSourceCompat,
    request: &Request,
) -> Result<Value, ProviderError> {
    let client = Arc::new(HttpClient::new());
    match wire {
        NativeResponseWire::Responses => OpenAiResponsesProvider::for_route(
            None,
            "openai-responses".into(),
            Default::default(),
            vec![response_model(WireApi::OpenAiResponses, source)],
            client,
        )
        .build_request_body(request),
        NativeResponseWire::Codex => OpenAiCodexResponsesProvider::new(
            None,
            vec![response_model(WireApi::OpenAiCodexResponses, source)],
            client,
        )
        .build_request_body(request),
    }
}

fn assert_response_user(item: &Value, text: &str) {
    assert_eq!(item["role"], "user");
    assert_eq!(item["content"], text);
}

fn response_input<'a>(body: &'a Value, initial_in_input: bool, instructions: &str) -> &'a [Value] {
    let input = body["input"].as_array().unwrap();
    if initial_in_input {
        assert!(
            body.get("instructions").is_none(),
            "ordinary and Azure instructions belong in input"
        );
        assert_eq!(input[0]["role"], "system");
        assert_eq!(input[0]["content"], instructions);
        &input[1..]
    } else {
        assert_eq!(
            body["instructions"], instructions,
            "Codex keeps initial instructions outside input"
        );
        input
    }
}

fn assert_response_addition(
    body: &Value,
    search: bool,
    strict: Option<Value>,
    instructions: &str,
    initial_in_input: bool,
) {
    assert_eq!(response_tool_names(body), ["alpha"]);
    assert_eq!(body["tools"][0].get("strict"), strict.as_ref());
    let input = response_input(body, initial_in_input, instructions);
    assert_response_user(&input[0], "before addition");
    assert_response_user(input.last().unwrap(), "after addition");
    assert!(
        input.iter().all(|item| item["content"] != "initial policy"),
        "initial instructions appear only at their wire-owned location"
    );
    if !search {
        assert_eq!(input.len(), 3);
        assert_eq!(input[1]["type"], "additional_tools");
        assert_eq!(input[1]["role"], "developer");
        assert_eq!(input[1]["tools"][0]["name"], "beta");
        assert_eq!(input[1]["tools"][0].get("strict"), strict.as_ref());
        assert!(input.iter().all(|item| !matches!(
            item["type"].as_str(),
            Some("tool_search_call" | "tool_search_output")
        )));
        assert!(input[1]["tools"][0].get("defer_loading").is_none());
    } else {
        assert_eq!(input.len(), 4);
        let call = &input[1];
        let output = &input[2];
        assert_eq!(call["type"], "tool_search_call");
        assert_eq!(output["type"], "tool_search_output");
        assert_eq!(call["execution"], "client");
        assert_eq!(output["execution"], "client");
        assert_eq!(call["status"], "completed");
        assert_eq!(output["status"], "completed");
        assert_eq!(call["arguments"], json!({"query":"beta", "limit":1}));
        assert_eq!(
            call["call_id"],
            format!(
                "pi_tool_load_{}",
                opi_ai::identity::short_hash("system:1:beta")
            )
        );
        assert_eq!(call["call_id"], output["call_id"]);
        assert_eq!(output["tools"][0]["name"], "beta");
        assert_eq!(output["tools"][0]["defer_loading"], true);
        assert_eq!(output["tools"][0].get("strict"), strict.as_ref());
    }
    assert_eq!(body["tools"][0].get("defer_loading"), None);
}

fn assert_response_non_additive(
    body: &Value,
    change: NativeToolChange,
    strict: Option<Value>,
    initial_in_input: bool,
) {
    let input = response_input(body, initial_in_input, "initial policy");
    assert_eq!(input.len(), 2);
    assert_response_user(&input[0], "before addition");
    assert_response_user(&input[1], "after addition");
    match change {
        NativeToolChange::Remove => assert!(response_tool_names(body).is_empty()),
        NativeToolChange::Redefine => {
            assert_eq!(response_tool_names(body), ["alpha"]);
            assert_eq!(body["tools"][0]["description"], "replacement alpha");
            assert_eq!(body["tools"][0].get("strict"), strict.as_ref());
        }
        NativeToolChange::Add => unreachable!(),
    }
}

#[test]
fn native_responses_and_codex_tool_precedence_search_pairs_and_defaults() {
    for wire in [NativeResponseWire::Responses, NativeResponseWire::Codex] {
        let strict = match wire {
            NativeResponseWire::Responses => None,
            NativeResponseWire::Codex => Some(Value::Null),
        };
        let request = native_tool_request(
            response_wire_spec(wire),
            native_tool_context(NativeToolChange::Add, false),
        );
        let canonical = serde_json::to_value(&request.context).unwrap();
        for (additional, search, expected_search) in [(true, true, false), (false, true, true)] {
            let source = response_source(true, additional, search);
            let first = native_response_body(wire, source.clone(), &request).unwrap();
            let second = native_response_body(wire, source.clone(), &request).unwrap();
            let mut strict_request = request.clone();
            strict_request.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
            assert_eq!(
                native_response_body(wire, source, &strict_request).unwrap(),
                first
            );
            assert_eq!(
                first, second,
                "paired IDs and all wire facts stay deterministic"
            );
            assert_response_addition(
                &first,
                expected_search,
                strict.clone(),
                "initial policy",
                matches!(wire, NativeResponseWire::Responses),
            );
        }
        let collapsed = native_response_body(wire, Default::default(), &request).unwrap();
        assert_eq!(response_tool_names(&collapsed), ["alpha", "beta"]);
        assert_eq!(
            response_input(
                &collapsed,
                matches!(wire, NativeResponseWire::Responses),
                "initial policy"
            )
            .len(),
            2
        );
        for tool in collapsed["tools"].as_array().unwrap() {
            assert_eq!(tool.get("strict"), strict.as_ref());
        }
        assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
    }
}

#[test]
fn native_responses_and_codex_mixed_classes_and_non_additive_strict() {
    for wire in [NativeResponseWire::Responses, NativeResponseWire::Codex] {
        let strict = match wire {
            NativeResponseWire::Responses => None,
            NativeResponseWire::Codex => Some(Value::Null),
        };
        let mut request = native_tool_request(
            response_wire_spec(wire),
            native_tool_context(NativeToolChange::Add, true),
        );
        let canonical = serde_json::to_value(&request.context).unwrap();
        let source = response_source(false, true, true);
        let body = native_response_body(wire, source.clone(), &request).unwrap();
        assert_response_addition(
            &body,
            false,
            strict.clone(),
            "initial policy\n\nlater policy",
            matches!(wire, NativeResponseWire::Responses),
        );
        request.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
        assert!(matches!(
            native_response_body(wire, source, &request),
            Err(ProviderError::Config(_))
        ));
        assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
        for change in [NativeToolChange::Remove, NativeToolChange::Redefine] {
            let mut request =
                native_tool_request(response_wire_spec(wire), native_tool_context(change, false));
            let source = response_source(true, true, true);
            let body = native_response_body(wire, source.clone(), &request).unwrap();
            assert_response_non_additive(
                &body,
                change,
                strict.clone(),
                matches!(wire, NativeResponseWire::Responses),
            );
            request.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
            let error = native_response_body(wire, source, &request).unwrap_err();
            assert!(matches!(error, ProviderError::Config(_)));
            assert!(error.to_string().contains("strict transcript projection"));
        }
    }
}

async fn azure_native_tool_call(
    source: opi_ai::model_info::OpenAiResponsesSourceCompat,
    context: TranscriptContext,
    strict: bool,
) -> (
    Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure>,
    Vec<Value>,
) {
    use opi_ai::azure_openai::{AzureOpenAIOptions, AzureOpenAIResponsesChatApi};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param},
    };
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/openai/v1/responses")).and(query_param("api-version", "v1"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(concat!(
            "data: {\"type\":\"response.created\",\"response\":{\"id\":\"native-response\",\"status\":\"in_progress\",\"model\":\"native-tool-model\",\"output\":[]}}\n\n",
            "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"native-response\",\"status\":\"completed\",\"model\":\"native-tool-model\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}}\n\n"
        ))).mount(&server).await;
    let model = response_model(WireApi::AzureOpenAiResponses, source)
        .with_base_url(format!("{}/openai/v1", server.uri()));
    let provider = Arc::new(
        AzureOpenAIProvider::for_records(
            Some(format!("{}/openai/v1", server.uri())),
            vec![model.clone()],
            Some("v1".into()),
        )
        .unwrap(),
    );
    let api = AzureOpenAIResponsesChatApi::new(provider, model.clone()).unwrap();
    let mut models = opi_ai::Models::new();
    let handle = models
        .register_chat_api(
            Arc::new(api),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("synthetic-azure-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    let mut request = native_tool_request("azure:native-tool-model", context);
    if strict {
        request.projection.temporal = opi_ai::transcript::TranscriptProjection::Strict;
    }
    let canonical = serde_json::to_value(&request.context).unwrap();
    let result = models
        .complete(
            opi_ai::models::ChatModelRecord::new("azure", model),
            request.clone(),
            handle.options(AzureOpenAIOptions::default()),
        )
        .await;
    assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
    let requests = server.received_requests().await.unwrap();
    let bodies = requests
        .iter()
        .map(|request| {
            assert_eq!(request.url.path(), "/openai/v1/responses");
            assert_eq!(
                request.headers.get("api-key").unwrap(),
                "synthetic-azure-key"
            );
            assert!(request.headers.get("authorization").is_none());
            serde_json::from_slice(&request.body).unwrap()
        })
        .collect();
    (result, bodies)
}

#[tokio::test]
async fn native_azure_responses_tool_precedence_search_pairs_and_strict_default() {
    use opi_ai::transcript::ProjectionMode;
    for (additional, search, expected_search) in [(true, true, false), (false, true, true)] {
        let mut source = response_source(true, additional, search);
        source.supports_strict_mode = Some(true);
        let (result, bodies) = azure_native_tool_call(
            source.clone(),
            native_tool_context(NativeToolChange::Add, false),
            false,
        )
        .await;
        let message = result.unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            message.projection.unwrap().temporal.mode,
            ProjectionMode::Native
        );
        assert_response_addition(
            &bodies[0],
            expected_search,
            Some(json!(false)),
            "initial policy",
            true,
        );
        let (strict_result, again) = azure_native_tool_call(
            source,
            native_tool_context(NativeToolChange::Add, false),
            true,
        )
        .await;
        assert_eq!(
            strict_result.unwrap().projection.unwrap().temporal.mode,
            ProjectionMode::Native
        );
        assert_eq!(again.len(), 1);
        assert_eq!(
            again[0], bodies[0],
            "Strict accepts supported native chronology with identical deterministic wire facts"
        );
    }
    for strict_flag in [None, Some(false)] {
        let source = opi_ai::model_info::OpenAiResponsesSourceCompat {
            supports_strict_mode: strict_flag,
            ..Default::default()
        };
        let (result, bodies) = azure_native_tool_call(
            source,
            native_tool_context(NativeToolChange::Add, false),
            false,
        )
        .await;
        result.unwrap();
        assert_eq!(bodies.len(), 1);
        assert_eq!(response_tool_names(&bodies[0]), ["alpha", "beta"]);
        assert_eq!(response_input(&bodies[0], true, "initial policy").len(), 2);
        for tool in bodies[0]["tools"].as_array().unwrap() {
            assert_eq!(
                tool.get("strict"),
                strict_flag.is_none().then_some(&json!(false))
            );
        }
    }
}

#[tokio::test]
async fn native_azure_responses_mixed_classes_and_non_additive_strict() {
    use opi_ai::transcript::{MessageRange, ProjectionMode};
    let mut source = response_source(false, true, true);
    source.supports_strict_mode = Some(true);
    let context = native_tool_context(NativeToolChange::Add, true);
    let (result, bodies) = azure_native_tool_call(source.clone(), context.clone(), false).await;
    let facts = result.unwrap().projection.unwrap();
    assert_eq!(bodies.len(), 1);
    assert_eq!(facts.temporal.mode, ProjectionMode::Collapse);
    assert_eq!(
        facts.temporal.affected_ranges,
        [MessageRange { start: 0, end: 3 }]
    );
    assert_response_addition(
        &bodies[0],
        false,
        Some(json!(false)),
        "initial policy\n\nlater policy",
        true,
    );
    let (result, bodies) = azure_native_tool_call(source, context, true).await;
    assert!(matches!(
        result.unwrap_err().cause(),
        ProviderError::Config(_)
    ));
    assert!(bodies.is_empty());
    for change in [NativeToolChange::Remove, NativeToolChange::Redefine] {
        let mut source = response_source(true, true, true);
        source.supports_strict_mode = Some(true);
        let context = native_tool_context(change, false);
        let (result, bodies) = azure_native_tool_call(source.clone(), context.clone(), false).await;
        result.unwrap();
        assert_eq!(bodies.len(), 1);
        assert_response_non_additive(&bodies[0], change, Some(json!(false)), true);
        let (result, bodies) = azure_native_tool_call(source, context, true).await;
        let failure = result.unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::Config(_)));
        assert!(
            failure
                .cause()
                .to_string()
                .contains("strict transcript projection")
        );
        assert!(bodies.is_empty());
    }
}

#[derive(Clone, Copy, Debug)]
enum InlineMetadataWire {
    Responses,
    Azure,
    Codex,
}

impl InlineMetadataWire {
    fn wire(self) -> WireApi {
        match self {
            Self::Responses => WireApi::OpenAiResponses,
            Self::Azure => WireApi::AzureOpenAiResponses,
            Self::Codex => WireApi::OpenAiCodexResponses,
        }
    }
    fn provider(self) -> &'static str {
        match self {
            Self::Responses => "openai-responses",
            Self::Azure => "azure",
            Self::Codex => "openai-codex",
        }
    }
}

fn inline_metadata_source(grammar: bool) -> opi_ai::model_info::OpenAiResponsesSourceCompat {
    let mut source = response_source(true, true, false);
    source.supports_strict_mode = Some(true);
    source.supports_grammar_tools = Some(grammar);
    source
}

fn inline_grammar_tool() -> ToolDef {
    let mut tool = native_tool("beta", "inline grammar query");
    tool.constrained_sampling = Some(opi_ai::constrained_sampling::ConstrainedSampling::Grammar {
        openai_lark: Some("start: /.+/".into()),
        openai_regex: None,
    });
    tool
}

fn inline_metadata_context(tool: ToolDef) -> TranscriptContext {
    let mut messages = native_tool_context(NativeToolChange::Add, false).into_messages();
    let opi_ai::message::Message::System(change) = &mut messages[2] else {
        unreachable!()
    };
    change.tools_added = vec![tool];
    normalize_context(Context {
        messages,
        ..Default::default()
    })
    .unwrap()
}

fn inline_tool_sse(custom: bool, input: &str) -> String {
    let item = if custom {
        json!({"type":"custom_tool_call", "id":"ctc_inline", "call_id":"call_inline", "name":"beta", "input":input})
    } else {
        json!({"type":"function_call", "id":"fc_inline", "call_id":"call_inline", "name":"beta", "arguments":input})
    };
    let mut added = item.clone();
    added[if custom { "input" } else { "arguments" }] = json!("");
    let delta = if custom {
        json!({"type":"response.custom_tool_call_input.delta", "output_index":0, "delta":input})
    } else {
        json!({"type":"response.function_call_arguments.delta", "output_index":0, "delta":input})
    };
    [
        json!({"type":"response.output_item.added", "output_index":0, "item":added}), delta,
        json!({"type":"response.output_item.done", "output_index":0, "item":item}),
        json!({"type":"response.completed", "response":{"id":"inline-response", "status":"completed", "model":"native-tool-model", "output":[item], "usage":{"input_tokens":3,"output_tokens":2}}}),
    ].iter().map(|event| format!("data: {event}\n\n")).collect()
}

fn inline_text_sse() -> String {
    let event = json!({"type":"response.completed", "response":{"id":"inline-text", "status":"completed", "model":"native-tool-model", "output":[{"type":"message", "content":[{"type":"output_text", "text":"ok"}]}], "usage":{"input_tokens":3,"output_tokens":1}}});
    format!("data: {event}\n\n")
}

async fn inline_metadata_call(
    wire: InlineMetadataWire,
    source: opi_ai::model_info::OpenAiResponsesSourceCompat,
    context: TranscriptContext,
    sse: String,
) -> (
    Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure>,
    Value,
) {
    use opi_ai::provider::Provider;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .expect(1)
        .mount(&server)
        .await;
    let expected_initial_names: Vec<String> =
        opi_ai::transcript::get_initial_system_message(context.messages())
            .into_iter()
            .flat_map(|head| head.tools_added.iter().map(|tool| tool.name.clone()))
            .collect();
    let mut request =
        native_tool_request(&format!("{}:native-tool-model", wire.provider()), context);
    request.transport = opi_ai::provider::TransportPreference::Sse;
    let canonical = serde_json::to_value(&request.context).unwrap();
    let model = response_model(wire.wire(), source);
    let result = match wire {
        InlineMetadataWire::Responses => {
            let provider = OpenAiResponsesProvider::for_route(
                Some(server.uri()),
                wire.provider().into(),
                Default::default(),
                vec![model],
                Arc::new(HttpClient::new()),
            );
            opi_ai::stream::ChatStream::new(provider.stream_prepared(
                request.clone(),
                opi_ai::test_support::resolved_bearer_auth(),
            ))
            .result()
            .await
        }
        InlineMetadataWire::Codex => {
            let provider = OpenAiCodexResponsesProvider::new(
                Some(server.uri()),
                vec![model],
                Arc::new(HttpClient::new()),
            );
            let mut auth = opi_ai::test_support::resolved_bearer_auth();
            auth.account_id = Some("synthetic-account".into());
            opi_ai::stream::ChatStream::new(provider.stream_prepared(request.clone(), auth))
                .result()
                .await
        }
        InlineMetadataWire::Azure => {
            use opi_ai::azure_openai::{AzureOpenAIOptions, AzureOpenAIResponsesChatApi};
            let model = model.with_base_url(format!("{}/openai/v1", server.uri()));
            let provider = Arc::new(
                AzureOpenAIProvider::for_records(
                    Some(format!("{}/openai/v1", server.uri())),
                    vec![model.clone()],
                    Some("v1".into()),
                )
                .unwrap(),
            );
            let api = AzureOpenAIResponsesChatApi::new(provider, model.clone()).unwrap();
            let mut models = opi_ai::Models::new();
            let handle = models
                .register_chat_api(
                    Arc::new(api),
                    Arc::new(opi_ai::auth::StaticAuthResolver::new(
                        opi_ai::auth::AuthScheme::ApiKey,
                        secrecy::SecretString::from("synthetic-azure-key"),
                    )),
                    opi_ai::AuthProvenanceSource::Static,
                )
                .unwrap();
            models
                .complete(
                    opi_ai::models::ChatModelRecord::new("azure", model),
                    request.clone(),
                    handle.options(AzureOpenAIOptions::default()),
                )
                .await
        }
    };
    assert_eq!(serde_json::to_value(&request.context).unwrap(), canonical);
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 1);
    let body: Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(
        response_tool_names(&body),
        expected_initial_names,
        "initial declarations stay separate from the inline beta declaration"
    );
    if expected_initial_names.is_empty() {
        assert!(
            body.get("tools").is_none(),
            "an empty initial toolset does not add a top-level tools field"
        );
    }
    let addition = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["type"] == "additional_tools")
        .expect("actual wire has inline declaration");
    assert_eq!(addition["tools"][0]["name"], "beta");
    server.verify().await;
    (result, body)
}

fn inline_call(message: &opi_ai::message::AssistantMessage) -> &opi_ai::message::ToolCall {
    message
        .content
        .iter()
        .find_map(|content| match content {
            opi_ai::message::AssistantContent::ToolCall { tool_call } => Some(tool_call),
            _ => None,
        })
        .expect("tool proposal")
}

#[tokio::test]
async fn inline_metadata_custom_grammar_decodes_non_default_property() {
    let mut failures = Vec::new();
    for wire in [
        InlineMetadataWire::Responses,
        InlineMetadataWire::Azure,
        InlineMetadataWire::Codex,
    ] {
        let (result, body) = inline_metadata_call(
            wire,
            inline_metadata_source(true),
            inline_metadata_context(inline_grammar_tool()),
            inline_tool_sse(true, "find me"),
        )
        .await;
        assert_eq!(
            response_input(
                &body,
                !matches!(wire, InlineMetadataWire::Codex),
                "initial policy"
            )[1]["tools"][0]["type"],
            "custom"
        );
        match result {
            Ok(message) => {
                let call = inline_call(&message);
                if serde_json::from_str::<Value>(&call.arguments).ok()
                    != Some(json!({"query":"find me"}))
                    || call.arguments_error.is_some()
                {
                    failures.push(format!(
                        "{wire:?}: arguments={:?}, error={:?}",
                        call.arguments, call.arguments_error
                    ));
                }
            }
            Err(failure) => failures.push(format!("{wire:?}: failure={:?}", failure.cause())),
        }
    }
    assert!(
        failures.is_empty(),
        "inline grammar metadata was lost: {failures:?}"
    );
}

#[tokio::test]
async fn inline_metadata_strict_proposal_validation_uses_added_tool_schema() {
    use opi_ai::constrained_sampling::{ConstrainedSampling, StrictPreference};
    use opi_ai::schema::{ToolArgumentOutcome, compile, validate_tool_arguments};
    let mut tool = native_tool("beta", "inline strict query");
    tool.input_schema = json!({
        "type":"object", "properties":{
            "query":{"type":"string", "enum":["allowed"]},
            "optional":{"type":"string"}
        }, "required":["query"], "additionalProperties":false
    });
    tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
        strict: StrictPreference::Require,
    });
    let trusted = compile(tool.input_schema.clone()).unwrap();
    assert!(
        matches!(
            validate_tool_arguments(&trusted, &json!({"query":"blocked"}), None).unwrap(),
            ToolArgumentOutcome::Invalid { .. }
        ),
        "invalid proposals are rejected by the separate public schema engine"
    );
    let mut failures = Vec::new();
    for wire in [
        InlineMetadataWire::Responses,
        InlineMetadataWire::Azure,
        InlineMetadataWire::Codex,
    ] {
        let (result, body) = inline_metadata_call(
            wire,
            inline_metadata_source(true),
            inline_metadata_context(tool.clone()),
            inline_tool_sse(false, r#"{"query":"allowed","optional":null}"#),
        )
        .await;
        let inline = &response_input(
            &body,
            !matches!(wire, InlineMetadataWire::Codex),
            "initial policy",
        )[1]["tools"][0];
        assert_eq!(inline["strict"], true);
        assert!(
            inline["parameters"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!("optional")),
            "the actual strict wire makes the original optional field required"
        );
        match result {
            Ok(message) => {
                let call = inline_call(&message);
                let decoded = serde_json::from_str::<Value>(&call.arguments).unwrap();
                if decoded != json!({"query":"allowed"}) || call.arguments_error.is_some() {
                    failures.push(format!(
                        "{wire:?}: strict optional-null mapping={decoded}, parse_error={:?}",
                        call.arguments_error
                    ));
                }
                assert!(
                    matches!(validate_tool_arguments(&trusted, &decoded, call.arguments_error.as_deref()).unwrap(),
                    ToolArgumentOutcome::Valid { normalized } if normalized == json!({"query":"allowed"})),
                    "a valid decoded proposal passes explicit public validation"
                );
            }
            Err(failure) => failures.push(format!(
                "{wire:?}: unexpected stream failure {:?}",
                failure.cause()
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "inline strict normalization metadata was lost: {failures:?}"
    );
}

#[tokio::test]
async fn inline_metadata_second_turn_replays_custom_call_and_output() {
    use opi_ai::message::{AssistantContent, Message, ToolCall, ToolResultMessage};
    let mut failures = Vec::new();
    for wire in [
        InlineMetadataWire::Responses,
        InlineMetadataWire::Azure,
        InlineMetadataWire::Codex,
    ] {
        let mut assistant = opi_ai::test_support::base_assistant();
        assistant.api = opi_ai::ApiKind::OpenAi;
        assistant.provider = wire.provider().into();
        assistant.model = "native-tool-model".into();
        assistant.wire = Some(wire.wire());
        assistant.stop_reason = opi_ai::stream::StopReason::ToolUse;
        assistant.content = vec![AssistantContent::ToolCall {
            tool_call: ToolCall {
                id: "call_inline|fc_inline".into(),
                name: "beta".into(),
                arguments: r#"{"query":"find me"}"#.into(),
                arguments_error: None,
                signature: None,
            },
        }];
        let mut messages = inline_metadata_context(inline_grammar_tool()).into_messages();
        messages.extend([
            Message::Assistant(assistant),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "call_inline|fc_inline".into(),
                tool_name: "beta".into(),
                content: vec![opi_ai::message::OutputContent::Text {
                    text: "done".into(),
                }],
                details: None,
                is_error: false,
                not_executed: None,
                truncated: false,
                timestamp_ms: 3,
            }),
            super::user("continue"),
        ]);
        let context = normalize_context(Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        let (result, body) = inline_metadata_call(
            wire,
            inline_metadata_source(true),
            context,
            inline_text_sse(),
        )
        .await;
        result.unwrap();
        let input = body["input"].as_array().unwrap();
        let call = input.iter().find(|item| item["type"] == "custom_tool_call");
        let output = input
            .iter()
            .find(|item| item["type"] == "custom_tool_call_output");
        if !matches!((call, output), (Some(call), Some(output)) if call["input"] == "find me" && call["call_id"] == output["call_id"] && output["output"] == "done")
        {
            failures.push(format!("{wire:?}: replay={input:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "inline custom grammar replay metadata was lost: {failures:?}"
    );
}

#[tokio::test]
async fn inline_metadata_unsupported_grammar_keeps_fallback_diagnostics() {
    let mut failures = Vec::new();
    for wire in [
        InlineMetadataWire::Responses,
        InlineMetadataWire::Azure,
        InlineMetadataWire::Codex,
    ] {
        let mut messages = inline_metadata_context(inline_grammar_tool()).into_messages();
        let opi_ai::message::Message::System(head) = &mut messages[0] else {
            unreachable!()
        };
        let mut initial_grammar = inline_grammar_tool();
        initial_grammar.name = "alpha".into();
        head.tools_added = vec![initial_grammar];
        let context = normalize_context(Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        let (result, body) = inline_metadata_call(
            wire,
            inline_metadata_source(false),
            context,
            inline_text_sse(),
        )
        .await;
        assert_eq!(
            response_input(
                &body,
                !matches!(wire, InlineMetadataWire::Codex),
                "initial policy"
            )[1]["tools"][0]["type"],
            "function"
        );
        let message = result.unwrap();
        assert!(
            message
                .diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.reason == "grammar_target_unsupported"
                        && diagnostic.detail.as_deref() == Some("alpha")
                ),
            "initial-tool diagnostic carrier positive control"
        );

        if message
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.kind == "constrained_sampling_fallback"
                    && diagnostic.reason == "grammar_target_unsupported"
                    && diagnostic.detail.as_deref() == Some("beta")
            })
            .count()
            != 1
        {
            failures.push(format!("{wire:?}: diagnostics={:?}", message.diagnostics));
        }
    }
    assert!(
        failures.is_empty(),
        "inline fallback diagnostic metadata was lost: {failures:?}"
    );
}

#[tokio::test]
async fn inline_metadata_empty_initial_tools_keeps_added_grammar_metadata() {
    let mut failures = Vec::new();
    for wire in [
        InlineMetadataWire::Responses,
        InlineMetadataWire::Azure,
        InlineMetadataWire::Codex,
    ] {
        let mut messages = inline_metadata_context(inline_grammar_tool()).into_messages();
        let opi_ai::message::Message::System(head) = &mut messages[0] else {
            unreachable!()
        };
        head.tools_added.clear();
        let context = normalize_context(Context {
            messages,
            ..Default::default()
        })
        .unwrap();
        let (result, body) = inline_metadata_call(
            wire,
            inline_metadata_source(true),
            context,
            inline_tool_sse(true, "find me"),
        )
        .await;
        assert!(body.get("tools").is_none());
        assert_eq!(
            response_input(
                &body,
                !matches!(wire, InlineMetadataWire::Codex),
                "initial policy"
            )[1]["tools"][0]["type"],
            "custom"
        );
        match result {
            Ok(message) => {
                let call = inline_call(&message);
                if serde_json::from_str::<Value>(&call.arguments).ok()
                    != Some(json!({"query":"find me"}))
                    || call.arguments_error.is_some()
                {
                    failures.push(format!(
                        "{wire:?}: arguments={:?}, error={:?}",
                        call.arguments, call.arguments_error
                    ));
                }
            }
            Err(failure) => failures.push(format!("{wire:?}: failure={:?}", failure.cause())),
        }
    }
    assert!(
        failures.is_empty(),
        "empty initial toolset lost inline metadata: {failures:?}"
    );
}
