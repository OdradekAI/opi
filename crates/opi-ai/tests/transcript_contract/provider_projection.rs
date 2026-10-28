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
    prompt: "/instructions",
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
            // Codex explicitly emits null; ordinary Responses omits strict.
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
