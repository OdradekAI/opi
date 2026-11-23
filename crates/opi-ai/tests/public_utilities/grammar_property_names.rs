//! Literal grammar field names at the helper and Responses request-builder boundaries.

use opi_ai::constrained_sampling::{
    ConstrainedSampling, GrammarInputBuffer, append_grammar_tool_input_delta,
    resolve_grammar_constrained_sampling,
};
use opi_ai::message::ToolDef;
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::providers::openai_responses::{OpenAiResponsesProvider, ResponsesConfig};
use opi_ai::transcript::{Context, get_current_tools, normalize_context};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const LARK: &str = "start: \"hello\"";
const KEYS: [&str; 4] = ["input", "x/y", "x~1y", "x~y"];

fn grammar_tool(key: &str, present: bool) -> ToolDef {
    let properties = if present {
        json!({key: {"type": "string"}})
    } else {
        match key {
            // These entries satisfy a JSON Pointer lookup, but do not
            // declare the required literal property name.
            "x/y" => json!({"x": {"y": {"type": "string"}}}),
            "x~1y" => json!({"x/y": {"type": "string"}}),
            _ => json!({"other": {"type": "string"}}),
        }
    };
    ToolDef {
        name: "grammar-tool".to_owned(),
        description: "Accept a greeting".to_owned(),
        input_schema: json!({
            "type": "object", "properties": properties, "required": [key]
        }),
        constrained_sampling: Some(ConstrainedSampling::Grammar {
            openai_lark: Some(LARK.to_owned()),
            openai_regex: None,
        }),
    }
}

fn request(tool: ToolDef) -> Request {
    Request {
        projection: Default::default(),
        model: "openai-responses:gpt-4o".to_owned(),
        context: normalize_context(Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: vec![tool],
        })
        .unwrap(),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
        transport: Default::default(),
    }
}

fn provider() -> OpenAiResponsesProvider {
    OpenAiResponsesProvider::new_with_config(
        None,
        ResponsesConfig {
            source: opi_ai::model_info::OpenAiResponsesSourceCompat {
                supports_grammar_tools: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

#[test]
fn helper_resolves_and_wraps_literal_property_names() {
    for key in KEYS {
        let tool = grammar_tool(key, true);
        let original = tool.input_schema.clone();
        let grammar = resolve_grammar_constrained_sampling(&tool, true)
            .unwrap_or_else(|error| panic!("{key:?}: {error}"))
            .unwrap();
        assert_eq!(grammar.input_property, key);
        assert_eq!(grammar.definition, LARK);
        let mut buffer = GrammarInputBuffer::new(grammar.input_property, String::new());
        let first = append_grammar_tool_input_delta(&mut buffer, "he", false)
            .unwrap()
            .unwrap();
        let last = append_grammar_tool_input_delta(&mut buffer, "hello", true)
            .unwrap()
            .unwrap();
        let wrapped: Value = serde_json::from_str(&(first + &last)).unwrap();
        assert_eq!(wrapped, json!({key: "hello"}));
        assert_eq!(tool.input_schema, original);
    }
}

#[test]
fn helper_rejects_missing_literal_property_despite_pointer_alias() {
    for key in KEYS {
        let tool = grammar_tool(key, false);
        let original = tool.input_schema.clone();
        assert!(
            resolve_grammar_constrained_sampling(&tool, true).is_err(),
            "missing literal property {key:?} must fail"
        );
        assert_eq!(tool.input_schema, original);
    }
}

#[test]
fn responses_builder_accepts_literal_grammar_property_names() {
    let provider = provider();
    for key in KEYS {
        let tool = grammar_tool(key, true);
        let original = tool.input_schema.clone();
        let request = request(tool);
        let body = provider
            .build_request_body(&request)
            .unwrap_or_else(|error| panic!("{key:?}: {error}"));
        assert_eq!(body["tools"][0]["type"], "custom");
        assert_eq!(body["tools"][0]["name"], "grammar-tool");
        assert_eq!(
            body["tools"][0]["format"],
            json!({"type": "grammar", "syntax": "lark", "definition": LARK})
        );
        assert_eq!(
            get_current_tools(request.context.messages())[0].input_schema,
            original
        );
    }
}

#[test]
fn responses_builder_rejects_missing_literal_grammar_property() {
    let provider = provider();
    for key in KEYS {
        let tool = grammar_tool(key, false);
        let original = tool.input_schema.clone();
        let request = request(tool);
        assert!(provider.build_request_body(&request).is_err(), "{key:?}");
        assert_eq!(
            get_current_tools(request.context.messages())[0].input_schema,
            original
        );
    }
}
