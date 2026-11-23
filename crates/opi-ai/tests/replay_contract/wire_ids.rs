//! Preserve call/result pairing through Responses item IDs and wire ID legalization.

use opi_ai::message::Message;
use opi_ai::providers::openai_responses::OpenAiResponsesProvider;
use opi_ai::stream::StopReason;
use serde_json::json;

use super::fixtures::{anthropic, assistant, call, request, result, user};

/// Responses restores the item ID for matching identity and pairs output by call ID.
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

/// Legalize both sides of the pair with the same request-local wire ID.
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
