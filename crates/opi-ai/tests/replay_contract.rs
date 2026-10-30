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

#[path = "replay_contract/object_wire.rs"]
mod object_wire;

use std::sync::Arc;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::bedrock::BedrockProvider;
use opi_ai::message::{
    AssistantContent, AssistantMessage, InputContent, MediaType, Message, OutputContent, ToolCall,
    ToolNotExecutedReason, ToolResultMessage, UserMessage,
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
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,

        deferred: None,
    })
}

fn call(id: &str, name: &str) -> AssistantContent {
    AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: name.into(),
            arguments: "{\"path\":\"a.rs\"}".into(),
            arguments_error: None,
            signature: None,
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
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    })
}

fn not_executed_result(id: &str, name: &str, reason: ToolNotExecutedReason) -> Message {
    let mut message = result(id, name, "private execution-boundary diagnostic");
    if let Message::ToolResult(result) = &mut message {
        result.is_error = true;
        result.not_executed = Some(reason);
    }
    message
}

fn request(model: &str, messages: Vec<Message>) -> Request {
    Request {
        transport: Default::default(),
        model: model.into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: ThinkingConfig::default(),
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
    let canonical = serde_json::to_string(
        &opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: messages.clone(),
            tools: vec![],
        })
        .expect("valid context declarations"),
    )
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
    let replayed = serde_json::to_string(
        &opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .expect("valid context declarations"),
    )
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
            not_executed_result(
                "f_1",
                "read",
                ToolNotExecutedReason::InterruptedBeforeExecution,
            ),
            assistant(
                vec![AssistantContent::Text {
                    text: "recovered".into(),
                    signature: None,
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

/// AIR-012/013 and RP-04/09: omitting a failed proposal cannot erase the
/// unknown effect of a missing result. It is ordinary user context, since
/// there is no wire call to which a synthetic tool result could attach.
#[test]
fn failed_turn_missing_results_keep_unknown_effect_across_batch_boundaries() {
    use opi_ai::replay::{ReplayError, ReplayLoss, ReplayProjection, ReplayRoute, project_replay};
    use opi_ai::transcript::SystemMessage;

    let route = ReplayRoute {
        wire: opi_ai::model_info::WireApi::AnthropicMessages,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        supports_images: true,
    };
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        for boundary in ["end", "user", "assistant", "system", "partial"] {
            let mut missing = call("private-missing-id", "read");
            if let AssistantContent::ToolCall { tool_call } = &mut missing {
                tool_call.arguments = "private-raw-arguments".into();
                tool_call.arguments_error = Some("private-parse-error".into());
            }
            let mut calls = vec![missing];
            if boundary == "partial" {
                calls.insert(0, call("private-known-id", "read"));
            }
            let mut messages = vec![user("q"), assistant(calls, stop_reason)];
            if boundary == "partial" {
                messages.push(not_executed_result(
                    "private-known-id",
                    "read",
                    ToolNotExecutedReason::InterruptedBeforeExecution,
                ));
            }
            if matches!(boundary, "system" | "partial") {
                messages.push(Message::System(SystemMessage {
                    content: "updated instructions".into(),
                    ..Default::default()
                }));
            }
            match boundary {
                "user" | "partial" => messages.push(user("continue")),
                "assistant" => messages.push(assistant(
                    vec![AssistantContent::Text {
                        text: "recovered".into(),
                        signature: None,
                    }],
                    StopReason::Stop,
                )),
                _ => {}
            }
            let mut req = request("anthropic:claude-sonnet-4-5-20250514", messages);
            let canonical = serde_json::to_string(&req.context).unwrap();
            req.context = serde_json::from_str(&canonical).unwrap();
            let out = project_replay(
                req.context.messages(),
                &route,
                ReplayProjection::NativeOrCollapse,
            )
            .unwrap();
            let missing_ids: Vec<_> = out
                .facts
                .losses
                .iter()
                .filter_map(|loss| match loss {
                    ReplayLoss::SynthesizedMissingResult { tool_call_id } => {
                        Some(tool_call_id.as_str())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                missing_ids,
                ["private-missing-id"],
                "{stop_reason:?}/{boundary}: only the missing result needs closure"
            );
            let failure_position = out
                .messages
                .iter()
                .position(|message| match message {
                    Message::User(user) => user.content.iter().any(|content| {
                        matches!(content, InputContent::Text { text } if text.contains("Result unavailable") && text.contains("execution effect is unknown"))
                    }),
                    _ => false,
                })
                .expect("failed missing result remains as ordinary User failure context");
            assert!(out.messages.iter().all(|message| {
                !matches!(message, Message::ToolResult(_))
                    && !matches!(message, Message::Assistant(assistant) if assistant.content.iter().any(|block| matches!(block, AssistantContent::ToolCall { .. })))
            }));
            if let Some(system_position) = out
                .messages
                .iter()
                .position(|message| matches!(message, Message::System(_)))
            {
                assert!(
                    failure_position < system_position,
                    "system stays batch-transparent"
                );
            }
            let ReplayError::StrictLoss(losses) =
                project_replay(req.context.messages(), &route, ReplayProjection::Strict)
                    .expect_err("Strict rejects both omission and missing-result synthesis")
            else {
                panic!("expected StrictLoss");
            };
            assert!(
                losses
                    .iter()
                    .any(|loss| matches!(loss, ReplayLoss::OmittedFailedAssistant { .. }))
            );
            assert_eq!(
                losses.iter().filter(|loss| matches!(loss, ReplayLoss::SynthesizedMissingResult { tool_call_id } if tool_call_id == "private-missing-id")).count(),
                1
            );
            let anthropic_body = anthropic().build_request_body(&req).unwrap();
            let chat_body = OpenAiChatProvider::new(None)
                .build_request_body(&req)
                .unwrap();
            for body in [anthropic_body, chat_body] {
                let serialized = body.to_string();
                assert!(serialized.contains("Result unavailable"));
                assert!(serialized.contains("execution effect is unknown"));
                for private in [
                    "private-missing-id",
                    "private-known-id",
                    "private-raw-arguments",
                    "private-parse-error",
                    "private execution-boundary diagnostic",
                    "tool_use",
                    "tool_calls",
                    "not-executed",
                    "not executed",
                ] {
                    assert!(
                        !serialized.contains(private),
                        "{stop_reason:?}/{boundary}: {private}"
                    );
                }
            }
            assert_eq!(serde_json::to_string(&req.context).unwrap(), canonical);
        }
    }
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
            signature: None,
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
                signature: None,
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
            signature: None,
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
        not_executed_result("tc_bad", "read", ToolNotExecutedReason::InvalidJson),
    ];
    let body = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            anthropic_history,
        ))
        .unwrap();
    let serialized = body.to_string();
    assert!(serialized.contains("[opi:tool-call-not-executed:v1]"));
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
            not_executed: None,
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
            not_executed: None,
            truncated: false,
            timestamp_ms: 0,
        }),
    ];
    let body = anthropic()
        .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
        .unwrap();
    let messages = body["messages"].as_array().unwrap();
    // Task 20.39 (pinned splitErrorToolResultImages + convertMessages): the
    // placeholder replaces the image run as a text block, and the relocated
    // caption + image follow all tool results in the SAME user message.
    let error_block = &messages[2]["content"][0];
    assert_eq!(error_block["type"], "tool_result");
    assert_eq!(error_block["is_error"], json!(true));
    assert_eq!(
        error_block["content"],
        json!([{"type": "text", "text": "(image attached after the tool results)"}])
    );
    let follow_up = &messages[2]["content"];
    assert_eq!(follow_up[1]["type"], "text");
    assert!(
        follow_up[1]["text"]
            .as_str()
            .unwrap()
            .contains("Images from the error result of tool call img_2")
    );
    assert_eq!(follow_up[2]["type"], "image");

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
                signature: None,
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
        diagnostics: Vec::new(),
        duration_ms: None,
        provider_thinking_level: None,

        deferred: None,
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

#[test]
fn failed_turns_reject_results_without_non_execution_facts() {
    for stop_reason in [StopReason::Error, StopReason::Aborted] {
        for (is_error, not_executed) in [
            (false, None),
            (true, None),
            (
                false,
                Some(ToolNotExecutedReason::InterruptedBeforeExecution),
            ),
        ] {
            let mut recorded = result("failed", "read", "tool changed external state");
            if let Message::ToolResult(result) = &mut recorded {
                result.is_error = is_error;
                result.not_executed = not_executed;
            }
            let error = anthropic()
                .build_request_body(&request(
                    "anthropic:claude-sonnet-4-5-20250514",
                    vec![
                        user("q"),
                        assistant(vec![call("failed", "read")], stop_reason),
                        recorded,
                    ],
                ))
                .expect_err("failed assistant projection must not erase possible effects");
            assert!(matches!(error, ProviderError::Config(_)));
        }
    }
}

#[test]
fn malformed_object_wire_does_not_infer_non_execution_from_error_text() {
    let malformed = AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: "malformed".into(),
            name: "read".into(),
            arguments: "[1]".into(),
            arguments_error: Some("sensitive parse diagnostic".into()),
            signature: None,
        },
    };
    let mut recorded = result("malformed", "read", "was not executed");
    if let Message::ToolResult(result) = &mut recorded {
        result.is_error = true;
    }
    let error = anthropic()
        .build_request_body(&request(
            "anthropic:claude-sonnet-4-5-20250514",
            vec![
                user("q"),
                assistant(vec![malformed], StopReason::ToolUse),
                recorded,
            ],
        ))
        .expect_err("ordinary error text cannot supply a non-execution fact");
    assert!(matches!(error, ProviderError::Config(_)));
}

#[test]
fn legalized_wire_ids_remain_bijective_when_a_source_id_matches_a_generated_id() {
    let messages = vec![
        user("q"),
        assistant(
            vec![call("x/y", "read"), call("x-y_becfa9197f2d6333", "read")],
            StopReason::ToolUse,
        ),
        result("x/y", "read", "first"),
        result("x-y_becfa9197f2d6333", "read", "second"),
    ];
    let req = request("anthropic:claude-sonnet-4-5-20250514", messages);
    let canonical = serde_json::to_value(&req.context).unwrap();
    let first = anthropic().build_request_body(&req).unwrap();
    let second = anthropic().build_request_body(&req).unwrap();
    assert_eq!(first, second, "retry mapping is stable");
    let blocks = first["messages"][1]["content"].as_array().unwrap();
    let ids: Vec<_> = blocks
        .iter()
        .filter_map(|block| block["id"].as_str())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(
        ids[0], ids[1],
        "different source calls keep different wire IDs"
    );
    let results: Vec<_> = first["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|message| message["content"].as_array().unwrap())
        .filter(|block| block["type"] == "tool_result")
        .collect();
    assert_eq!(results[0]["tool_use_id"], ids[0]);
    assert_eq!(results[1]["tool_use_id"], ids[1]);
    assert_eq!(serde_json::to_value(&req.context).unwrap(), canonical);
}

#[test]
fn omitted_batches_still_validate_association() {
    for stop_reason in [StopReason::Error, StopReason::Aborted, StopReason::ToolUse] {
        let proposal = || {
            let mut proposal = call("x", "read");
            if stop_reason == StopReason::ToolUse
                && let AssistantContent::ToolCall { tool_call } = &mut proposal
            {
                tool_call.arguments = "[1]".into();
                tool_call.arguments_error = Some("non-object".into());
            }
            proposal
        };
        let trusted = || not_executed_result("x", "read", ToolNotExecutedReason::InvalidJson);
        for (calls, results, expected) in [
            (
                vec![proposal(), proposal()],
                vec![],
                "duplicate tool call id",
            ),
            (
                vec![proposal()],
                vec![trusted(), trusted()],
                "duplicate tool result",
            ),
            (
                vec![proposal()],
                vec![not_executed_result(
                    "x",
                    "other",
                    ToolNotExecutedReason::InvalidJson,
                )],
                "names",
            ),
            (
                vec![proposal()],
                vec![not_executed_result(
                    "orphan",
                    "read",
                    ToolNotExecutedReason::InvalidJson,
                )],
                "does not belong",
            ),
        ] {
            let mut messages = vec![user("q"), assistant(calls, stop_reason)];
            messages.extend(results);
            let error = anthropic()
                .build_request_body(&request("anthropic:claude-sonnet-4-5-20250514", messages))
                .expect_err("omission cannot bypass association checks");
            assert!(matches!(error, ProviderError::Config(_)));
            assert!(
                error.to_string().contains(expected),
                "{stop_reason:?}: {error}"
            );
        }
    }
}

#[test]
fn malformed_non_execution_context_is_one_sanitized_user_block_after_serde() {
    let malformed = |id: &str, arguments: &str| AssistantContent::ToolCall {
        tool_call: ToolCall {
            id: id.into(),
            name: "read\"\n".into(),
            arguments: arguments.into(),
            arguments_error: Some("private-error-canary".into()),
            signature: None,
        },
    };
    let req = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![
            user("canonical input"),
            assistant(
                vec![
                    malformed("private-id-canary-a", "{\"private-argument-canary"),
                    call("valid", "read"),
                    malformed("private-id-canary-b", "[1]"),
                ],
                StopReason::ToolUse,
            ),
            not_executed_result(
                "private-id-canary-a",
                "read\"\n",
                ToolNotExecutedReason::InvalidJson,
            ),
            result("valid", "read", "real recorded result"),
            not_executed_result(
                "private-id-canary-b",
                "read\"\n",
                ToolNotExecutedReason::NonObjectArguments,
            ),
        ],
    );
    let encoded = serde_json::to_string(&req.context).unwrap();
    let mut restored = req.clone();
    restored.context = serde_json::from_str(&encoded).unwrap();
    let body = anthropic().build_request_body(&restored).unwrap();
    let encoded_body = body.to_string();
    for private in [
        "private-error-canary",
        "private-argument-canary",
        "private-id-canary",
        "private execution-boundary diagnostic",
    ] {
        assert!(!encoded_body.contains(private), "{private}");
    }
    let contexts: Vec<_> = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|message| {
            message["content"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(move |block| {
                    block["text"]
                        .as_str()
                        .filter(|text| text.starts_with("[opi:tool-call-not-executed:v1]"))
                        .map(|text| (&message["role"], text))
                })
        })
        .collect();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].0, "user");
    let payload: serde_json::Value =
        serde_json::from_str(contexts[0].1.split_once(' ').unwrap().1).unwrap();
    assert_eq!(
        payload,
        json!({"calls": [
            {"batch_call":0,"tool_name":"read\"\n","reason_code":"invalid_json"},
            {"batch_call":2,"tool_name":"read\"\n","reason_code":"non_object_arguments"}
        ]})
    );
    assert!(encoded_body.contains("real recorded result"));
    assert!(encoded_body.contains("tool_use_id"));
    assert_eq!(serde_json::to_string(&req.context).unwrap(), encoded);
}

#[test]
fn malformed_missing_result_keeps_unknown_effect_distinct_from_non_execution() {
    let mut malformed = call("missing", "read");
    if let AssistantContent::ToolCall { tool_call } = &mut malformed {
        tool_call.arguments = "null".into();
        tool_call.arguments_error = Some("private diagnostic".into());
    }
    let req = request(
        "anthropic:claude-sonnet-4-5-20250514",
        vec![user("q"), assistant(vec![malformed], StopReason::ToolUse)],
    );
    let body = anthropic().build_request_body(&req).unwrap().to_string();
    assert!(body.contains("Result unavailable"));
    assert!(body.contains("execution effect is unknown"));
    assert!(!body.contains("not-executed"));
    assert!(!body.contains("not executed"));
    assert!(!body.contains("private diagnostic"));
    assert!(!body.contains("tool_use"));
}

#[test]
fn legacy_result_serde_has_no_non_execution_fact() {
    let message = result("legacy", "read", "was not executed");
    let mut encoded = serde_json::to_value(message).unwrap();
    encoded.as_object_mut().unwrap().remove("not_executed");
    let Message::ToolResult(restored) = serde_json::from_value::<Message>(encoded).unwrap() else {
        panic!("tool result expected");
    };
    assert_eq!(restored.not_executed, None);
}

// --- task 20.11: grammar tool replay association (V-34, ctc_ items) ---

mod grammar_replay_tests {
    use super::*;
    use opi_ai::constrained_sampling::ConstrainedSampling;
    use opi_ai::message::ToolDef;
    use opi_ai::openai_responses::ResponsesConfig;

    fn grammar_tool() -> ToolDef {
        ToolDef {
            name: "grammar-tool".into(),
            description: "grammar constrained".into(),
            input_schema: json!({
                "type": "object",
                "properties": {"input": {"type": "string"}},
                "required": ["input"]
            }),
            constrained_sampling: Some(ConstrainedSampling::Grammar {
                openai_lark: Some("start: WORD".into()),
                openai_regex: None,
            }),
        }
    }

    fn grammar_request(messages: Vec<Message>) -> Request {
        let mut request = request("openai-responses:gpt-4o", messages);
        request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: request
                .context
                .messages()
                .iter()
                .filter(|message| !matches!(message, opi_ai::message::Message::System(_)))
                .cloned()
                .collect(),
            tools: vec![grammar_tool()],
        })
        .expect("valid context declarations");
        request
    }

    #[test]
    fn grammar_call_and_result_replay_as_custom_items_with_ctc_ids() {
        let mut same_identity_call = call("call_g|ctc_1", "grammar-tool");
        if let AssistantContent::ToolCall { tool_call } = &mut same_identity_call {
            tool_call.arguments = json!({"input": "raw input"}).to_string();
        }
        let mut assistant_message = assistant(vec![same_identity_call], StopReason::ToolUse);
        if let Message::Assistant(inner) = &mut assistant_message {
            inner.provider = "openai-responses".into();
            inner.model = "gpt-4o".into();
            inner.wire = Some(opi_ai::model_info::WireApi::OpenAiResponses);
        }
        let messages = vec![
            user("q"),
            assistant_message,
            result("call_g|ctc_1", "grammar-tool", "raw grammar output"),
        ];
        let provider = OpenAiResponsesProvider::new_with_config(
            None,
            ResponsesConfig {
                supports_grammar_tools: true,
                ..Default::default()
            },
        );
        let body = provider
            .build_request_body(&grammar_request(messages))
            .expect("grammar replay body builds");
        let serialized = body.to_string();
        assert!(
            serialized.contains("\"type\":\"custom_tool_call\""),
            "the grammar call replays as a custom call item: {serialized}"
        );
        assert!(
            serialized.contains("\"id\":\"ctc_1\""),
            "the packed ctc_ item id restores on replay: {serialized}"
        );
        assert!(
            serialized.contains("\"type\":\"custom_tool_call_output\""),
            "the grammar result replays as a custom output item: {serialized}"
        );
        assert!(
            !serialized.contains("\"type\":\"function_call\""),
            "grammar tools never replay as function items: {serialized}"
        );
    }
}
