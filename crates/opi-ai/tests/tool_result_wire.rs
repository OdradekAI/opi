//! Phase 11.9: provider tool-result `is_error` wire propagation.
//!
//! A failed tool result (`ToolResultMessage.is_error == true`) MUST remain
//! distinguishable from a successful one on every supported provider wire.
//! Before this fix only Bedrock propagated `is_error` (native `toolResult.status`);
//! the other four builders dropped it, making failure indistinguishable from
//! success text.
//!
//! Locked per-provider wire shape (design-stress panel, 5 reviewers):
//! - Anthropic: native `is_error: true` on the `tool_result` content block.
//! - Bedrock: native `toolResult.status` = "error" / "success" (already correct;
//!   pinned here).
//! - OpenAI Chat (incl. Azure / OpenRouter / Mistral via the shared adapter): a
//!   deterministic `[tool_error] ` marker prefixed to the `role:"tool"` content
//!   string. The Chat Completions API has no native error field.
//! - OpenAI Responses: the same `[tool_error] ` marker prefixed to the
//!   `function_call_output.output` string. `status` is NOT client-settable on
//!   input items (server-managed on output items only), so a content marker is
//!   mandatory.
//! - Gemini (incl. Vertex via the shared adapter): an `error: true` key INSIDE
//!   the `functionResponse.response` Struct. The Gemini REST API documents this
//!   as the failure key; a top-level `functionResponse.error` is unsupported.
//!
//! `build_request_body` (and `build_converse_body` for Bedrock) IS the production
//! serializer invoked by each provider's `Provider::stream` path, so a fixture
//! that constructs a `Request` with a failed `ToolResult` and calls it exercises
//! the exact wire serialization -- not a mock. No live provider calls, no network.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::azure_openai::AzureOpenAIProvider;
use opi_ai::bedrock::BedrockProvider;
use opi_ai::bedrock::sigv4::AwsCredentials;
use opi_ai::gemini::GeminiProvider;
use opi_ai::http::HttpClient;
use opi_ai::message::{InputContent, Message, OutputContent, ToolResultMessage, UserMessage};
use opi_ai::mistral::mistral_provider;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::openrouter::openrouter_provider;
use opi_ai::provider::{CacheRetention, Request, ThinkingConfig};
use opi_ai::vertex::VertexProvider;
use tokio_util::sync::CancellationToken;

/// The deterministic failure marker the no-native-field providers prefix onto the
/// rendered tool-result text. Pinned in the test so the DoD word "deterministic"
/// is a byte-for-byte assertion.
const EXPECTED_ERROR_MARKER: &str = "[tool_error] ";

fn test_credentials() -> AwsCredentials {
    AwsCredentials {
        access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
        secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
        session_token: None,
        region: "us-east-1".into(),
    }
}

/// Build a Request whose last message is a ToolResult with the requested
/// `is_error` flag and a known payload text. A leading user message keeps the
/// shape realistic; tool-result assertions navigate to the LAST message.
fn request_with_tool_result(is_error: bool, payload: &str) -> Request {
    Request {
        model: "m".into(),
        system: None,
        messages: vec![
            Message::User(UserMessage {
                content: vec![InputContent::Text { text: "hi".into() }],
                timestamp_ms: 0,
            }),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "tc-1".into(),
                tool_name: "tool".into(),
                content: vec![OutputContent::Text {
                    text: payload.into(),
                }],
                details: None,
                is_error,
                truncated: false,
                timestamp_ms: 0,
            }),
        ],
        tools: vec![],
        max_tokens: Some(128),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

/// Last message object of a body's `messages`/`contents`/`input` array.
fn last_msg<'a>(body: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    body[key]
        .as_array()
        .unwrap_or_else(|| panic!("body has no `{key}` array: {body}"))
        .last()
        .unwrap_or_else(|| panic!("body `{key}` array is empty: {body}"))
}

// --- per-provider assertions ------------------------------------------------

fn anthropic_asserts(is_error: bool, payload: &str) {
    let provider = AnthropicProvider::new("k".into(), None);
    let body = provider.build_request_body(&request_with_tool_result(is_error, payload));
    let block = &last_msg(&body, "messages")["content"][0];
    assert_eq!(block["type"], "tool_result", "anthropic tool_result block");
    assert_eq!(
        block["tool_use_id"], "tc-1",
        "anthropic tool_use_id MUST round-trip the ToolResult tool_call_id"
    );
    assert_eq!(
        block["content"].as_str(),
        Some(payload),
        "anthropic content is the joined payload (no marker for this provider)"
    );
    if is_error {
        assert_eq!(
            block["is_error"].as_bool(),
            Some(true),
            "anthropic MUST set native is_error:true on the tool_result block"
        );
    } else {
        assert!(
            block.get("is_error").is_none(),
            "anthropic is_error:false body must be byte-identical to pre-fix (no is_error key): {block}"
        );
    }
}

fn bedrock_asserts(is_error: bool, payload: &str) {
    let provider = BedrockProvider::new(test_credentials(), None, Arc::new(HttpClient::new()));
    let body = provider.build_converse_body(&request_with_tool_result(is_error, payload));
    let result = &last_msg(&body, "messages")["content"][0]["toolResult"];
    assert_eq!(
        result["toolUseId"], "tc-1",
        "bedrock toolUseId MUST round-trip the ToolResult tool_call_id"
    );
    let status = &result["status"];
    let expected = if is_error { "error" } else { "success" };
    assert_eq!(
        status.as_str(),
        Some(expected),
        "bedrock toolResult.status must be {expected} for is_error={is_error}"
    );
    // Bedrock must NOT carry a content marker; the native status field is the signal.
    let body_text = serde_json::to_string(&body).unwrap();
    assert!(
        !body_text.contains(EXPECTED_ERROR_MARKER.trim()),
        "bedrock must rely on native status, not a content marker: {body_text}"
    );
}

fn openai_chat_asserts(is_error: bool, payload: &str) {
    let provider = OpenAiChatProvider::new("k".into(), None);
    let body = provider.build_request_body(&request_with_tool_result(is_error, payload));
    let msg = last_msg(&body, "messages");
    assert_eq!(msg["role"], "tool", "openai_chat role");
    assert_eq!(
        msg["tool_call_id"], "tc-1",
        "openai_chat tool_call_id MUST round-trip the ToolResult tool_call_id"
    );
    let content = msg["content"].as_str().expect("openai_chat content string");
    if is_error {
        assert!(
            content.starts_with(EXPECTED_ERROR_MARKER),
            "openai_chat MUST prefix the failure marker; got: {content:?}"
        );
        assert!(
            content.ends_with(payload),
            "openai_chat marker must precede the original payload; got: {content:?}"
        );
    } else {
        assert_eq!(
            content, payload,
            "openai_chat is_error:false body must be byte-identical to pre-fix (no marker)"
        );
    }
}

fn openai_responses_asserts(is_error: bool, payload: &str) {
    let provider = OpenAiResponsesProvider::new("k".into(), None);
    let body = provider.build_request_body(&request_with_tool_result(is_error, payload));
    let item = last_msg(&body, "input");
    assert_eq!(
        item["type"], "function_call_output",
        "openai_responses item type"
    );
    assert_eq!(
        item["call_id"], "tc-1",
        "openai_responses call_id MUST round-trip the ToolResult tool_call_id"
    );
    // status is NOT client-settable on input items; its absence is a regression guard.
    assert!(
        item.get("status").is_none(),
        "openai_responses must NOT set status on input function_call_output (not client-settable): {item}"
    );
    let output = item["output"]
        .as_str()
        .expect("openai_responses output string");
    if is_error {
        assert!(
            output.starts_with(EXPECTED_ERROR_MARKER),
            "openai_responses MUST prefix the failure marker; got: {output:?}"
        );
    } else {
        assert_eq!(
            output, payload,
            "openai_responses is_error:false body must be byte-identical to pre-fix (no marker)"
        );
    }
}

fn gemini_asserts(is_error: bool, payload: &str) {
    let provider = GeminiProvider::new("k".into(), None);
    let body = provider.build_request_body(&request_with_tool_result(is_error, payload));
    let response = &last_msg(&body, "contents")["parts"][0]["functionResponse"]["response"];
    assert_eq!(
        response["content"].as_str(),
        Some(payload),
        "gemini response.content is the joined payload (no text marker for this provider)"
    );
    if is_error {
        assert_eq!(
            response["error"].as_bool(),
            Some(true),
            "gemini MUST set response.error:true INSIDE the response Struct on failure: {response}"
        );
    } else {
        assert!(
            response.get("error").is_none(),
            "gemini is_error:false body must be byte-identical to pre-fix (no error key): {response}"
        );
    }
}

// --- inheritance assertions --------------------------------------------------

/// Assert the body (built by any OpenAI-Chat-compatible provider) carries the
/// failure marker on error and is byte-identical to the payload on success.
/// Takes a pre-built body so it is independent of the concrete provider type
/// (Azure is its own type; OpenRouter/Mistral return the shared
/// `OpenAiChatProvider`). Pins BOTH the is_error:true marker AND the
/// is_error:false byte-identity for every inheriting surface.
fn assert_chat_compatible_body(body: &serde_json::Value, is_error: bool, payload: &str) {
    let content = last_msg(body, "messages")["content"]
        .as_str()
        .expect("chat-compatible content string");
    if is_error {
        assert!(
            content.starts_with(EXPECTED_ERROR_MARKER),
            "OpenAI-compatible provider must inherit the [tool_error] marker via the shared adapter; got: {content:?}"
        );
    } else {
        assert_eq!(
            content, payload,
            "OpenAI-compatible is_error:false body must be byte-identical to the payload (no marker): {content:?}"
        );
    }
}

fn vertex_inherits_gemini_shape(is_error: bool, payload: &str) {
    let provider = VertexProvider::new("tok".into(), "proj".into(), "loc".into(), None);
    let body = provider.build_request_body(&request_with_tool_result(is_error, payload));
    let response = &last_msg(&body, "contents")["parts"][0]["functionResponse"]["response"];
    assert_eq!(
        response["content"].as_str(),
        Some(payload),
        "Vertex must inherit the Gemini response.content payload: {response}"
    );
    if is_error {
        assert_eq!(
            response["error"].as_bool(),
            Some(true),
            "Vertex must inherit response.error:true on failure via the shared adapter: {response}"
        );
    } else {
        assert!(
            response.get("error").is_none(),
            "Vertex is_error:false body must be byte-identical to pre-fix (no error key): {response}"
        );
    }
}

/// The single acceptance entry point for scenario
/// `phase11-provider-is-error-propagation`. Exercises every provider family for
/// both failure and success, plus the four inheritance claims, plus the
/// Chat-vs-Responses marker-drift pin.
#[test]
fn tool_result_error_semantics_across_providers() {
    let payload = "ok";

    for is_error in [true, false] {
        anthropic_asserts(is_error, payload);
        bedrock_asserts(is_error, payload);
        openai_chat_asserts(is_error, payload);
        openai_responses_asserts(is_error, payload);
        gemini_asserts(is_error, payload);
    }

    // Inheritance: Azure / OpenRouter / Mistral reuse the OpenAI Chat serializer
    // for BOTH failure (marker) and success (byte-identical payload).
    let azure = AzureOpenAIProvider::new(
        "k".into(),
        Some("https://x.openai.azure.com".into()),
        "dep".into(),
        None,
    )
    .expect("azure provider constructs");
    let openrouter = openrouter_provider("k".into(), None);
    let mistral = mistral_provider("k".into(), None);
    for is_error in [true, false] {
        assert_chat_compatible_body(
            &azure.build_request_body(&request_with_tool_result(is_error, payload)),
            is_error,
            payload,
        );
        assert_chat_compatible_body(
            &openrouter.build_request_body(&request_with_tool_result(is_error, payload)),
            is_error,
            payload,
        );
        assert_chat_compatible_body(
            &mistral.build_request_body(&request_with_tool_result(is_error, payload)),
            is_error,
            payload,
        );
    }
    // Inheritance: Vertex reuses the Gemini serializer for both branches.
    for is_error in [true, false] {
        vertex_inherits_gemini_shape(is_error, payload);
    }

    // Marker-drift pin: the two OpenAI surfaces must emit byte-identical failure
    // output through the shared marker constant.
    let chat = OpenAiChatProvider::new("k".into(), None);
    let responses = OpenAiResponsesProvider::new("k".into(), None);
    let chat_body = chat.build_request_body(&request_with_tool_result(true, payload));
    let responses_body = responses.build_request_body(&request_with_tool_result(true, payload));
    let chat_content = last_msg(&chat_body, "messages")["content"]
        .as_str()
        .unwrap();
    let responses_output = last_msg(&responses_body, "input")["output"]
        .as_str()
        .unwrap();
    assert_eq!(
        chat_content, responses_output,
        "OpenAI Chat and Responses must emit byte-identical failure output"
    );
}

/// Acceptance entry point for scenario `phase11-provider-no-phase12-breadth`.
/// Mechanically proves this Phase 11 wire fix did not add Phase 12 breadth: no
/// new opi-ai provider module, no new opi-ai dependency, and no OAuth /
/// image-generation / catalog / marketplace surface.
#[test]
fn provider_tool_result_error_no_phase12_breadth_guard() {
    let cargo_toml = include_str!("../Cargo.toml");
    let lib_rs = include_str!("../src/lib.rs");

    let cargo_lower = cargo_toml.to_lowercase();
    for forbidden in [
        "oauth",
        "image-gen",
        "image_gen",
        "image_generation",
        "marketplace",
        "catalog",
        "stripe",
        "billing",
    ] {
        assert!(
            !cargo_lower.contains(forbidden),
            "opi-ai Cargo.toml must not add Phase 12 breadth (forbidden dependency/surface '{forbidden}'):\n{cargo_toml}"
        );
    }

    let lib_lower = lib_rs.to_lowercase();
    // Phase 14.2 deliberately added the scoped `auth` module (OAuth auth
    // contracts) to lib.rs; "oauth" is no longer a forbidden token there. The
    // module-freeze below still catches accidental module additions.
    for forbidden in [
        "image_gen",
        "image_generation",
        "marketplace",
        "catalog",
        "stripe",
        "billing",
    ] {
        assert!(
            !lib_lower.contains(forbidden),
            "opi-ai src/lib.rs must not declare a Phase 12 breadth module ('{forbidden}'):\n{lib_rs}"
        );
    }

    // Provider-module freeze: the public module set declared in lib.rs must equal
    // the reviewed Phase 11 baseline. Adding a provider family (Phase 12) requires
    // deliberately updating this baseline; this guard catches an accidental add.
    // (Phase 14.1 added the `credential` module; Phase 14.2 added the `auth`
    // module with scoped OAuth auth contracts.)
    let baseline_modules = vec![
        "anthropic",
        "auth",
        "azure_openai",
        "bedrock",
        "config",
        "credential",
        "gemini",
        "http",
        "message",
        "mistral",
        "model",
        "openai_chat",
        "openai_responses",
        "openrouter",
        "provider",
        "provider_collection",
        "registry",
        "retry",
        "stream",
        "test_support",
        "time",
        "vertex",
    ];
    let declared: Vec<&str> = lib_rs
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("pub mod "))
        .map(|rest| rest.trim_end_matches(';').trim())
        .collect();
    assert_eq!(
        declared, baseline_modules,
        "opi-ai public module set changed from the Phase 11 baseline; if this is an intentional \
         later-phase addition, update the baseline here deliberately. declared={declared:?}"
    );
}
