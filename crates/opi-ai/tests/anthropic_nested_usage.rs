use opi_ai::anthropic::{AnthropicMapper, ParsedEvent, parse_sse_events};
use opi_ai::provider::ProviderError;
use opi_ai::stream::AssistantStreamEvent;
use serde_json::{Value, json};

fn fixture(ending: &str) -> String {
    let frames = [
        json!({"type":"message_start","message":{"id":"nested-response","model":"nested-model","usage":{"input_tokens":100,"output_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":200,"cache_creation":{"ephemeral_1h_input_tokens":150},"output_tokens_details":{"thinking_tokens":0}}}}),
        json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":20,"cache_creation":{"ephemeral_1h_input_tokens":75},"output_tokens_details":{"thinking_tokens":6}}}),
        if ending == "wire-error" {
            json!({"type":"error","error":{"type":"overloaded_error","message":"nested fixture"}})
        } else {
            json!({"type":"message_stop","message":{}})
        },
    ];
    frames
        .iter()
        .map(|frame| {
            format!(
                "event: {}\ndata: {frame}\n\n",
                frame["type"].as_str().unwrap()
            )
        })
        .collect()
}

fn view(message: &opi_ai::message::AssistantMessage) -> Value {
    json!({
        "usage": [
            message.usage.input_tokens,
            message.usage.output_tokens,
            message.usage.cache_read_tokens,
            message.usage.cache_write_tokens,
            message.usage.cache_write_1h_tokens,
            message.usage.reasoning_tokens,
        ],
        "stop": message.stop_reason,
        "raw_stop": message.raw_stop_reason,
        "cost": message.usage.cost.total_cost(),
    })
}

fn run(ending: &str) -> Value {
    let mut mapper = AnthropicMapper::new();
    let mut terminal = None;
    for parsed in parse_sse_events(&fixture(ending)) {
        let ParsedEvent::Valid(event) = parsed else {
            panic!("nested fixture must parse");
        };
        for item in mapper.process(event) {
            match item {
                Ok(AssistantStreamEvent::Done { message, .. }) => {
                    terminal = Some(view(&message));
                }
                Err(ProviderError::StreamFailure(failure)) => {
                    terminal = Some(view(failure.message()));
                }
                Ok(_) => {}
                other => panic!("unexpected nested terminal event: {other:?}"),
            }
        }
    }
    terminal.expect("nested fixture must produce a terminal result")
}

#[test]
fn nested_usage_trace() {
    let success = run("success");
    assert_eq!(success["usage"], json!([100, 20, 0, 200, 75, 6]));
    assert_eq!(success["stop"], "stop");
    assert_eq!(success["raw_stop"], "end_turn");
    let failure = run("wire-error");
    assert_eq!(failure["usage"], json!([100, 20, 0, 200, 75, 6]));
    assert_eq!(failure["stop"], "stop");
    assert_eq!(failure["raw_stop"], "end_turn");
    println!(
        "{}",
        json!({"cases":[
            {"ending":"success","result":success},
            {"ending":"wire-error","result":failure},
        ]})
    );
}
