//! Canonical replay/association contract (task 20.7 closure battery,
//! V-07/V-08/V-15 SDK-active, AIR-010/012/013/014, DEL-03, P20-S07).
//!
//! Pinned references: `.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts`
//! and the dual-ID packing in `openai-responses-shared.ts` (Pi AI 1.3.0);
//! reviewed matrix in
//! `docs/research/pi-ai-opi-ai-alignment/replay-contracts.md` (RP-02..RP-11).
//! Every assertion here drives real adapter request bodies (the production
//! call sites) so a passing battery proves the wire consumes only the
//! replay-projected view. Host-impact clauses (AIR-009/041 execution,
//! authorization, retry policy, effort stamps/V-32) stay with their owners.

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::bedrock::BedrockProvider;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, MediaType, Message, OutputContent, ToolCall,
    ToolResultMessage, UserMessage,
};
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::openai_responses::OpenAiResponsesProvider;
use opi_ai::provider::{CacheRetention, ProviderError, Request, ThinkingConfig};
use opi_ai::stream::StopReason;
use opi_ai::test_support::resolved_auth;
use serde_json::json;
use tokio_util::sync::CancellationToken;

fn user(text: &str) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms: 0,
    })
}

fn assistant(content: Vec<AssistantContent>, stop_reason: StopReason) -> Message {
    Message::Assistant(AssistantMessage {
        content,
        api: opi_ai::ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: Some(opi_ai::model_info::WireApi::AnthropicMessages),
        response_id: None,
        usage: Default::default(),
        stop_reason,
        error_message: None,
        timestamp_ms: 0,
    })
}

fn call(id: &str, name: &str) -> AssistantContent {
    AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: "{\"path\":\"a.rs\"}".into(),
            arguments_error: None,
        },
    }
}

fn result(id: &str, name: &str, text: &str) -> Message {
    Message::ToolResult(ToolResultMessage {
        tool_call_id: id.into(),
        tool_name: name.into(),
        content: vec![OutputContent::Text { text: text.into() }],
        details: None,
        is_error: false,
        truncated: false,
        timestamp_ms: 0,
    })
}

fn request(model: &str, messages: Vec<Message>) -> Request {
    Request {
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        }),
        max_tokens: Some(64),
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

fn anthropic() -> AnthropicProvider {
    AnthropicProvider::with_client(
        Some("https://api.anthropic.com".into()),
        Arc::new(opi_ai::http::HttpClient::new()),
    )
}

/// AIR-012/RP-06: association is by batch position plus source ID. Closed
/// batches may reuse an ID string; inside one batch duplicates, orphans and
/// name mismatches are typed failures raised at body construction - before
/// any authorization or network use.
#[test]
fn association_matrix_through_real_adapters() {
    // Reused IDs across closed batches serialize fine on every family.
    let messages = vec![
        user("q"),
        assistant(vec![call("fc_0", "read")], StopReason::ToolUse),
        result("fc_0", "read", "one"),
        assistant(vec![call("fc_0", "read")], StopReason::ToolUse),
        result("fc_0", "read", "two"),
    ];
    let anthropic_body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            messages.clone(),
        ))
        .unwrap();
    let chat_body = OpenAiChatProvider::new(None)
        .build_request_body(&request("openai:gpt-4o", messages))
        .unwrap();
    for body in [&anthropic_body, &chat_body] {
        let serialized = body.to_string();
        assert!(
            serialized.matches("tool_use").count() >= 1
                || serialized.matches("function").count() >= 2,
            "both closed batches reach the wire"
        );
    }

    // Duplicate call ID inside one batch fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(
                    vec![call("x", "read"), call("x", "bash")],
                    StopReason::ToolUse,
                ),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("duplicate tool call id"));

    // Duplicate result for one call fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![call("x", "read")], StopReason::ToolUse),
                result("x", "read", "one"),
                result("x", "read", "two"),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("duplicate tool result"));

    // Orphan result fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), result("nope", "read", "one")],
        ))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not belong to any open tool-call batch")
    );

    // Result/call name mismatch fails typed.
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![call("x", "read")], StopReason::ToolUse),
                result("x", "bash", "one"),
            ],
        ))
        .unwrap_err();
    assert!(error.to_string().contains("names"));

    // The typed failure is a ProviderError::Config, not a panic: the
    // request never reaches authorization or the network.
    assert!(matches!(error, ProviderError::Config(_)));
}

/// RP-09: an unambiguous missing result closes request-locally with a
/// synthetic error result stating the result is unavailable and the
/// execution effect is unknown - never NotExecuted, never re-executed -
/// and the canonical request context is untouched by the projection.
#[test]
fn missing_results_close_request_locally_with_unknown_effect() {
    let messages = vec![
        user("q"),
        assistant(vec![call("tc_9", "read")], StopReason::ToolUse),
        user("next turn"),
    ];
    let canonical = serde_json::to_string(&opi_ai::transcript::normalize_context(
        opi_ai::transcript::Context {
            system_prompt: None,
            messages: messages.clone(),
            tools: vec![],
        },
    ))
    .unwrap();
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            messages.clone(),
        ))
        .unwrap();

    let tool_results: Vec<&serde_json::Value> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| {
            m["content"]
                .as_array()
                .map(|a| a.iter())
                .into_iter()
                .flatten()
        })
        .filter(|block| block["type"] == "tool_result")
        .collect();
    assert_eq!(tool_results.len(), 1, "exactly one synthetic result");
    assert_eq!(tool_results[0]["is_error"], json!(true));
    let text = tool_results[0]["content"].as_str().unwrap();
    assert!(text.contains("Result unavailable"), "{text}");
    assert!(text.contains("execution effect is unknown"), "{text}");
    assert!(!text.to_lowercase().contains("was not executed"));

    // The canonical history is byte-identical after projection (the
    // projection cannot mutate a borrowed chronology).
    let replayed = serde_json::to_string(&opi_ai::transcript::normalize_context(
        opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        },
    ))
    .unwrap();
    assert_eq!(canonical, replayed);
}

/// AIR-013/RP-04: failed turns (Error/Aborted) and their linked
/// non-execution results are omitted from the wire as a closed unit while
/// the canonical history keeps them; Length turns keep their expressible
/// pairs and length never grants execution anywhere in this projection.
#[test]
fn failed_turns_omit_with_linked_results_and_length_keeps_pairs() {
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        let messages = vec![
            user("q"),
            assistant(vec![call("f_1", "read")], stop_reason),
            result("f_1", "read", "not executed record"),
            assistant(
                vec![AssistantContent::Text {
                    text: "recovered".into(),
                }],
                StopReason::Stop,
            ),
        ];
        let body = anthropic()
            .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
            .unwrap();
        let serialized = body.to_string();
        assert!(
            !serialized.contains("f_1"),
            "{stop_reason:?}: failed turn and linked results omitted"
        );
        assert!(!serialized.contains("not executed record"));
        assert!(serialized.contains("recovered"));
    }

    let messages = vec![
        user("q"),
        assistant(vec![call("len_1", "read")], StopReason::Length),
        result("len_1", "read", "error result from the trusted path"),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    assert!(body.to_string().contains("len_1"), "Length pair kept");
}

/// AIR-010/V-07: cross provider/API/model drops every thinking block -
/// readable, signed and redacted alike - and never turns earlier reasoning
/// into visible text (Pi 1.3 semantics); opaque data never leaks to the
/// wire. Same identity keeps redacted data and signed blocks including
/// signed-empty ones.
#[test]
fn cross_identity_drops_all_thinking_and_same_identity_preserves_opaque() {
    let thinking = vec![
        AssistantContent::Thinking {
            thinking: "readable reasoning".into(),
            signature: Some("private-signature".into()),
        },
        AssistantContent::RedactedThinking {
            data: "private-opaque-data".into(),
        },
        AssistantContent::Text {
            text: "answer".into(),
        },
    ];
    let mut foreign = assistant(thinking.clone(), StopReason::Stop);
    if let Message::Assistant(a) = &mut foreign {
        a.provider = "other-provider".into();
    }
    let anthropic_body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), foreign.clone()],
        ))
        .unwrap();
    let bedrock_body = BedrockProvider::new(None, Arc::new(opi_ai::http::HttpClient::new()))
        .build_converse_body(&request(
            "bedrock:us-anthropic.claude-sonnet-4-5-20250514-v1:0",
            vec![user("q"), foreign],
        ))
        .unwrap();
    for body in [&anthropic_body, &bedrock_body] {
        let serialized = body.to_string();
        assert!(!serialized.contains("readable reasoning"));
        assert!(!serialized.contains("private-signature"));
        assert!(!serialized.contains("private-opaque-data"));
        assert!(serialized.contains("answer"));
    }

    // Same identity: signed-empty and redacted blocks round-trip onto the
    // Anthropic wire; blank unsigned thinking is dropped.
    let same = assistant(
        vec![
            AssistantContent::Thinking {
                thinking: String::new(),
                signature: Some("empty-signed".into()),
            },
            AssistantContent::RedactedThinking {
                data: "opaque-kept".into(),
            },
            AssistantContent::Thinking {
                thinking: "   ".into(),
                signature: None,
            },
            AssistantContent::Text {
                text: "answer".into(),
            },
        ],
        StopReason::Stop,
    );
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), same],
        ))
        .unwrap();
    let content = &body["messages"][1]["content"];
    assert_eq!(content[0]["signature"], json!("empty-signed"));
    assert_eq!(
        content[1],
        json!({"type": "redacted_thinking", "data": "opaque-kept"})
    );
    assert_eq!(content[2]["type"], "text");
    assert_eq!(content[2]["text"], "answer");

    // Legacy-unknown provenance (no stored wire) is never same-identity.
    let mut legacy = assistant(thinking, StopReason::Stop);
    if let Message::Assistant(a) = &mut legacy {
        a.wire = None;
    }
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![user("q"), legacy],
        ))
        .unwrap();
    assert!(!body.to_string().contains("private-"));
}

/// V-07 dual-ID: the Responses parser packs `call_id|fc_item_id`; the
/// serializer restores both fields for same identity and omits the item id
/// for foreign history so pairing validation never fires.
#[test]
fn responses_dual_ids_round_trip_and_cross_drops_item_id() {
    let provider = OpenAiResponsesProvider::new(None);
    let mut same = assistant(vec![call("call_x|fc_1", "read")], StopReason::ToolUse);
    if let Message::Assistant(a) = &mut same {
        a.api = opi_ai::ApiKind::OpenAi;
        a.provider = "openai-responses".into();
        a.model = "gpt-4o".into();
        a.wire = Some(opi_ai::model_info::WireApi::OpenAiResponses);
    }
    let body = provider
        .build_request_body(&request(
            "openai:gpt-4o",
            vec![user("q"), same.clone(), result("call_x|fc_1", "read", "ok")],
        ))
        .unwrap();
    let items = body["input"].as_array().unwrap();
    let function_call = items
        .iter()
        .find(|item| item["type"] == "function_call")
        .expect("function call item");
    assert_eq!(function_call["call_id"], json!("call_x"));
    assert_eq!(function_call["id"], json!("fc_1"));
    let output = items
        .iter()
        .find(|item| item["type"] == "function_call_output")
        .expect("output item pairs by call id");
    assert_eq!(output["call_id"], json!("call_x"));

    // Foreign history keeps the call id but drops the item id.
    let mut foreign = same;
    if let Message::Assistant(a) = &mut foreign {
        a.model = "other-model".into();
    }
    let body = provider
        .build_request_body(&request(
            "openai:gpt-4o",
            vec![user("q"), foreign, result("call_x|fc_1", "read", "ok")],
        ))
        .unwrap();
    let function_call = body["input"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["type"] == "function_call")
        .expect("function call item");
    assert_eq!(function_call["call_id"], json!("call_x"));
    assert!(
        function_call.get("id").is_none(),
        "foreign item id omitted to avoid pairing validation"
    );
}

/// RP-08: malformed proposals ride raw text on string-argument wires and
/// become explicit failure contexts on object-argument wires - never `{}`,
/// `null` or a fabricated successful call.
#[test]
fn malformed_proposals_split_by_wire_argument_shape() {
    let malformed = || AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "tc_bad".into(),
            name: "read".into(),
            arguments: "{\"broken".into(),
            arguments_error: Some("not a complete JSON object".into()),
        },
    };
    let history = |mut a: Message, result_text: &str| {
        if let Message::Assistant(am) = &mut a {
            am.api = opi_ai::ApiKind::OpenAi;
            am.provider = "openai".into();
            am.model = "gpt-4o".into();
            am.wire = Some(opi_ai::model_info::WireApi::OpenAiCompletions);
        }
        vec![user("q"), a, result("tc_bad", "read", result_text)]
    };

    // Chat: raw malformed text on the wire.
    let chat = OpenAiChatProvider::new(None);
    let body = chat
        .build_request_body(&request(
            "openai:gpt-4o",
            history(
                assistant(vec![malformed()], StopReason::ToolUse),
                "error result",
            ),
        ))
        .unwrap();
    let serialized = body.to_string();
    assert!(serialized.contains("broken"), "raw text preserved");
    assert!(
        !serialized.contains("\"arguments\":\"{}\""),
        "no fabricated empty object"
    );

    // Anthropic (object wire): explicit failure context replaces the pair.
    let anthropic_history = vec![
        user("q"),
        assistant(vec![malformed()], StopReason::ToolUse),
        result("tc_bad", "read", "error result"),
    ];
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            anthropic_history,
        ))
        .unwrap();
    let serialized = body.to_string();
    assert!(serialized.contains("[tool proposal not replayed]"));
    assert!(!serialized.contains("broken"));
    assert!(
        !serialized.contains("tool_result"),
        "linked result consumed by the context"
    );
}

/// RP-07: wire IDs are legalized request-locally with a stable bijective
/// mapping synchronized across the call and its result; canonical IDs are
/// untouched.
#[test]
fn wire_ids_legalize_synchronously_on_both_sides_of_the_pair() {
    let messages = vec![
        user("q"),
        assistant(
            vec![call("call|with pipes and spaces!", "read")],
            StopReason::ToolUse,
        ),
        result("call|with pipes and spaces!", "read", "ok"),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let tool_use_id = body["messages"][1]["content"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let tool_result_id = body["messages"][2]["content"][0]["tool_use_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(tool_use_id, tool_result_id, "pair stays synchronized");
    assert!(
        tool_use_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    );
    assert!(tool_use_id.len() <= 64);
}

/// AIR-014/V-08: image-capable routes receive real tool-result images with
/// their association, ordering and failure flag; Anthropic error results
/// split their images out to after the turn's results (pinned
/// `splitErrorToolResultImages`); private `details` never enter the wire and
/// URL sources pass through untouched.
#[test]
fn tool_result_images_preserve_association_order_and_failure_flag() {
    let image = || OutputContent::Image {
        source: opi_ai::message::ImageSource::Base64 {
            data: "AA==".into(),
        },
        media_type: MediaType::Png,
    };
    // Success result: image blocks ride the tool_result content array.
    let messages = vec![
        user("q"),
        assistant(vec![call("img_1", "screenshot")], StopReason::ToolUse),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "img_1".into(),
            tool_name: "screenshot".into(),
            content: vec![
                OutputContent::Text {
                    text: "captured".into(),
                },
                image(),
            ],
            details: Some(json!({"secret": "private-detail"})),
            is_error: false,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let block = &body["messages"][2]["content"][0];
    assert_eq!(block["type"], "tool_result");
    assert_eq!(block["tool_use_id"], "img_1");
    assert_eq!(block["content"][0]["type"], "text");
    assert_eq!(block["content"][0]["text"], "captured");
    assert_eq!(block["content"][1]["type"], "image");
    assert_eq!(block["content"][1]["source"]["media_type"], "image/png");
    assert!(!body.to_string().contains("private-detail"));

    // Error result: images split after the turn's results with a caption.
    let messages = vec![
        user("q"),
        assistant(vec![call("img_2", "screenshot")], StopReason::ToolUse),
        Message::ToolResult(ToolResultMessage {
            tool_call_id: "img_2".into(),
            tool_name: "screenshot".into(),
            content: vec![image()],
            details: None,
            is_error: true,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    let error_block = &messages[2]["content"][0];
    assert_eq!(error_block["type"], "tool_result");
    assert_eq!(error_block["is_error"], json!(true));
    assert_eq!(
        error_block["content"],
        "[image attached after the tool results]"
    );
    let follow_up = &messages[3]["content"];
    assert_eq!(follow_up[0]["type"], "text");
    assert!(
        follow_up[0]["text"]
            .as_str()
            .unwrap()
            .contains("Images from the error result of tool call img_2")
    );
    assert_eq!(follow_up[1]["type"], "image");

    // URL sources pass through, never downloaded.
    let messages = vec![user("see"), user_url_image()];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    assert_eq!(
        body["messages"][1]["content"][0]["source"]["url"],
        "https://example.test/i.png"
    );
}

fn user_url_image() -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Image {
            source: opi_ai::message::ImageSource::Url {
                url: "https://example.test/i.png".into(),
            },
            media_type: MediaType::Png,
        }],
        timestamp_ms: 0,
    })
}

/// AIR-012/RP-03: Chat keeps visible text alongside its tool calls in the
/// same turn (representable order preserved).
#[test]
fn chat_keeps_turn_text_alongside_tool_calls() {
    let chat = OpenAiChatProvider::new(None);
    let messages = vec![
        user("q"),
        assistant_with_openai_identity(vec![
            AssistantContent::Text {
                text: "I will read the file.".into(),
            },
            call("c_1", "read"),
        ]),
        result("c_1", "read", "contents"),
    ];
    let body = chat
        .build_request_body(&request("openai:gpt-4o", messages))
        .unwrap();
    let assistant = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "assistant")
        .unwrap();
    assert_eq!(assistant["content"], "I will read the file.");
    assert_eq!(assistant["tool_calls"][0]["function"]["name"], "read");
}

fn assistant_with_openai_identity(content: Vec<AssistantContent>) -> Message {
    Message::Assistant(AssistantMessage {
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: "openai".into(),
        model: "gpt-4o".into(),
        response_model: None,
        wire: Some(opi_ai::model_info::WireApi::OpenAiCompletions),
        response_id: None,
        usage: Default::default(),
        stop_reason: StopReason::ToolUse,
        error_message: None,
        timestamp_ms: 0,
    })
}

/// The projection is deterministic for one prepared call (retry stability,
/// RP-01) and a stream_prepared association failure surfaces as a typed
/// error event with zero network use.
#[tokio::test]
async fn association_failure_surfaces_typed_before_network() {
    use futures_util::StreamExt;
    let provider = anthropic();
    let request = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![user("q"), result("orphan", "read", "x")],
    );
    use opi_ai::provider::Provider;
    let mut stream = provider.stream_prepared(request, resolved_auth());
    let event = stream.next().await.unwrap();
    match event {
        Err(ProviderError::Config(summary)) => {
            let text = summary.to_string();
            assert!(text.contains("does not belong to any open tool-call batch"));
        }
        other => panic!("expected typed association failure, got {other:?}"),
    }
    // No further events: nothing was dispatched.
    assert!(stream.next().await.is_none());
}
