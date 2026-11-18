//! Silent usage corrections must reach canonical results and their observers.
use futures_util::StreamExt;
use opi_ai::api::anthropic::{AnthropicMapper, ParsedEvent, parse_sse_events};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::model_info::{ModelInfo, ModelPricing, WireApi};
use opi_ai::models::CompatMetadata;
use opi_ai::provider::{Provider, Request};
use opi_ai::providers::anthropic::AnthropicProvider;
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, ChatStream, Pricing};
use opi_ai::{AuthProvenanceSource, AuthScheme, ChatModelRecord, Models, StaticAuthResolver};
use opi_telemetry::memory::Recorder;
use secrecy::SecretString;
use serde_json::{Value, json};
use std::sync::Arc;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug)]
struct EventFaultContext(opi_telemetry::TelemetryContext);
#[derive(Debug)]
struct EventFaultSpan(Arc<dyn opi_telemetry::backend::SpanBackend>);
impl opi_telemetry::backend::ContextBackend for EventFaultContext {
    fn start_span(
        &self,
        options: &opi_telemetry::SpanOptions,
    ) -> opi_telemetry::Result<Arc<dyn opi_telemetry::backend::SpanBackend>> {
        Ok(Arc::new(EventFaultSpan(
            self.0.backend().start_span(options)?,
        )))
    }
    fn metrics(&self) -> Arc<dyn opi_telemetry::backend::MetricsBackend> {
        self.0.backend().metrics()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl opi_telemetry::backend::ContextBackend for EventFaultSpan {
    fn start_span(
        &self,
        options: &opi_telemetry::SpanOptions,
    ) -> opi_telemetry::Result<Arc<dyn opi_telemetry::backend::SpanBackend>> {
        self.0.start_span(options)
    }
    fn metrics(&self) -> Arc<dyn opi_telemetry::backend::MetricsBackend> {
        self.0.metrics()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl opi_telemetry::backend::SpanBackend for EventFaultSpan {
    fn set_attributes(&self, a: &opi_telemetry::Attributes) -> opi_telemetry::Result<()> {
        self.0.set_attributes(a)
    }
    fn add_event(&self, name: &str, a: &opi_telemetry::Attributes) -> opi_telemetry::Result<()> {
        if name == "pi.ai.first_chunk" {
            return Err(opi_telemetry::TelemetryError::new(std::io::Error::other(
                "fixture observer",
            )));
        }
        self.0.add_event(name, a)
    }
    fn set_status(&self, s: opi_telemetry::SpanStatus) -> opi_telemetry::Result<()> {
        self.0.set_status(s)
    }
    fn finish(&self, e: Option<opi_telemetry::ErrorStatus>) -> opi_telemetry::Result<()> {
        self.0.finish(e)
    }
}

fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!("data/anthropic-usage-updates.json")).unwrap()
}

#[test]
fn nested_cache_and_reasoning_usage_reaches_the_canonical_message() {
    let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":100,\"output_tokens\":0,\"cache_creation_input_tokens\":200,\"cache_creation\":{\"ephemeral_1h_input_tokens\":150},\"output_tokens_details\":{\"thinking_tokens\":0}}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":20,\"cache_creation\":{\"ephemeral_1h_input_tokens\":75},\"output_tokens_details\":{\"thinking_tokens\":6}}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
    let mut mapper = AnthropicMapper::new();
    let mut done = None;
    for parsed in parse_sse_events(sse) {
        let ParsedEvent::Valid(event) = parsed else {
            panic!("nested usage fixture must parse");
        };
        for result in mapper.process(event) {
            if let AssistantStreamEvent::Done { message, .. } = result.unwrap() {
                done = Some(message);
            }
        }
    }
    let message = done.expect("message_stop produces Done");
    assert_eq!(message.usage.cache_write_tokens, 200);
    assert_eq!(message.usage.cache_write_1h_tokens, Some(75));
    assert_eq!(message.usage.output_tokens, 20);
    assert_eq!(message.usage.reasoning_tokens, Some(6));
}

fn fixture(case: &Value, ending: &str) -> String {
    let mut frames = vec![
        json!({"type":"message_start","message":{"id":"usage-response","model":"usage-model","usage":{"input_tokens":11,"output_tokens":7,"cache_read_input_tokens":5,"cache_creation_input_tokens":3}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"A"}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"B"}}),
        json!({"type":"content_block_stop","index":0}),
    ];
    if case["after_stop"] == true {
        frames.push(json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{}}));
    }
    let mut delta = json!({"type":"message_delta","delta":{}});
    if case["after_stop"] != true {
        delta["delta"]["stop_reason"] = json!("end_turn");
    }
    if let Some(usage) = case.get("usage") {
        delta["usage"] = usage.clone();
    }
    frames.push(delta);
    frames.push(if ending == "wire-error" {
        json!({"type":"error","error":{"type":"overloaded_error","message":"fixture failure"}})
    } else {
        json!({"type":"message_stop"})
    });
    frames
        .iter()
        .map(|f| format!("event: {}\ndata: {f}\n\n", f["type"].as_str().unwrap()))
        .collect()
}

fn view(message: &AssistantMessage) -> Value {
    let u = &message.usage;
    json!({"text":message.content.iter().filter_map(|b| match b {AssistantContent::Text{text,..}=>Some(text.as_str()),_=>None}).collect::<String>(),
        "usage":[u.input_tokens,u.output_tokens,u.cache_read_tokens,u.cache_write_tokens],
        "cost":u.cost.total_cost(),"stop":message.stop_reason,"raw_stop":message.raw_stop_reason,"response_id":message.response_id})
}

async fn observe(case: &Value, route: &str, ending: &str) -> Value {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(fixture(case, ending)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let info = ModelInfo::new(
        "usage-model",
        "Usage fixture",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000, 4096).with_streaming(true),
    )
    .with_pricing(
        ModelPricing::try_new(
            Pricing {
                input_cost_per_mtok: 1.0,
                output_cost_per_mtok: 2.0,
                cache_read_cost_per_mtok: 3.0,
                cache_write_cost_per_mtok: 4.0,
            },
            vec![],
        )
        .unwrap(),
    )
    .unwrap();
    let provider = AnthropicProvider::for_route(
        "anthropic".into(),
        vec![info.clone()],
        Some(server.uri()),
        Default::default(),
        Arc::new(opi_ai::http::HttpClient::new()),
        false,
    );
    let recorder = Recorder::default();
    let telemetry = if case["observer_fault"] == true {
        opi_telemetry::TelemetryContext::new(Arc::new(EventFaultContext(recorder.context())))
    } else {
        recorder.context()
    };
    let first_seen = Arc::new(tokio::sync::Notify::new());
    let gate = first_seen.clone();
    let fail = ending == "callback-error";
    let request = Request {
        model: "anthropic:usage-model".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: Some(4096),
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: Some(telemetry),
        callbacks: opi_ai::callbacks::RequestCallbacks::new().with_async_on_raw(move |frame| {
            let gate = gate.clone();
            async move {
                if frame["delta"]["text"] == "B" {
                    gate.notified().await;
                }
                if fail && frame["type"] == "message_stop" {
                    return Err(opi_ai::callbacks::CallbackError::new("fixture callback"));
                }
                Ok(())
            }
        }),
    };
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider.clone()),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("fixture-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let record = ChatModelRecord::new("anthropic", info);
    let mut call = match route {
        "simple" => models
            .stream_simple(record, request, Default::default())
            .start(),
        "prepared-observed" => models
            .prepare_call("anthropic:usage-model", request)
            .await
            .unwrap()
            .start_attempt_with_telemetry()
            .unwrap()
            .start(),
        "prepared-raw" => ChatStream::new(
            models
                .prepare_call("anthropic:usage-model", request)
                .await
                .unwrap()
                .start_attempt()
                .unwrap(),
        )
        .start(),
        "direct" => ChatStream::new(
            provider.stream_prepared(request, opi_ai::test_support::resolved_auth()),
        )
        .start(),
        _ => panic!("unknown route"),
    };
    let mut held = None;
    let mut events = vec![];
    while let Some(event) = call.next().await {
        match event {
            Ok(AssistantStreamEvent::TextDelta { delta, partial, .. }) => {
                events.push(json!({"kind":"text_delta","delta":delta,"message":view(&partial)}));
                if held.is_none() {
                    held = Some(partial);
                    first_seen.notify_one();
                }
            }
            Ok(AssistantStreamEvent::TextEnd { partial, .. }) => {
                events.push(json!({"kind":"text_end","message":view(&partial)}))
            }
            Ok(AssistantStreamEvent::Done { message, .. }) => {
                events.push(json!({"kind":"done","message":view(&message)}))
            }
            Err(failure) => events.push(json!({"kind":"error","message":view(failure.message())})),
            _ => {}
        }
    }
    let (result, delivery) = match call.result().await {
        Ok(m) => (m, "ok"),
        Err(f) => (f.message().clone(), "error"),
    };
    let spans = recorder.spans();
    json!({"id":case["id"],"route":route,"ending":ending,"events":events,"result":view(&result),"delivery":delivery,
        "held":view(&held.unwrap()),"spans":spans.iter().map(|s|json!({"name":s.name,"ended":s.end_sequence.is_some(),
            "attributes":s.attributes,"events":s.events.iter().map(|e|json!({"name":e.name,"attributes":e.attributes})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "metrics":recorder.measurements().iter().map(|m|json!({"name":m.definition.name,"value":m.value,"attributes":m.attributes})).collect::<Vec<_>>(),
        "requests":server.received_requests().await.unwrap().len()})
}

async fn observations() -> Vec<Value> {
    let mut rows = vec![];
    for case in cases() {
        for ending in ["success", "wire-error", "callback-error"] {
            for route in ["simple", "prepared-observed", "prepared-raw", "direct"] {
                rows.push(
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        observe(&case, route, ending),
                    )
                    .await
                    .unwrap(),
                );
            }
        }
    }
    rows
}

#[tokio::test]
async fn anthropic_usage_trace() {
    println!("{}", json!({"cases":observations().await}));
}

#[tokio::test]
async fn anthropic_usage_contract() {
    for row in observations().await {
        let case = cases().into_iter().find(|c| c["id"] == row["id"]).unwrap();
        let expected = case["expected"].as_array().unwrap();
        assert_eq!(row["result"]["usage"], case["expected"], "{row}");
        let cost = expected
            .iter()
            .enumerate()
            .map(|(i, v)| v.as_f64().unwrap() * (i + 1) as f64 / 1e6)
            .sum::<f64>();
        assert!(
            (row["result"]["cost"].as_f64().unwrap() - cost).abs() < 1e-12,
            "{row}"
        );
        assert_eq!(row["result"]["text"], "AB");
        assert_eq!(row["held"]["text"], "A");
        assert_eq!(row["held"]["usage"], json!([11, 7, 5, 3]));
        assert!((row["held"]["cost"].as_f64().unwrap() - 0.000052).abs() < 1e-12);
        assert_eq!(row["result"]["raw_stop"], "end_turn");
        assert_eq!(
            row["result"]["stop"],
            if row["ending"] == "success" {
                "stop"
            } else {
                "error"
            }
        );
        assert_eq!(
            row["delivery"],
            if row["ending"] == "success" {
                "ok"
            } else {
                "error"
            }
        );
        let observed = row["route"] == "simple" || row["route"] == "prepared-observed";
        let spans = row["spans"].as_array().unwrap();
        assert_eq!(spans.len(), usize::from(observed));
        if observed {
            assert_eq!(spans[0]["ended"], true);
            if case["observer_fault"] == true {
                assert!(row["metrics"].as_array().unwrap().is_empty());
                assert!(spans[0]["attributes"].get("usage.input").is_none());
                continue;
            }
            assert!((spans[0]["attributes"]["cost.total"].as_f64().unwrap() - cost).abs() < 1e-12);
            let metrics = row["metrics"].as_array().unwrap();
            assert_eq!(metrics.len(), 7);
            for name in [
                "pi.ai.first_chunk.duration",
                "pi.ai.request.duration",
                "pi.ai.cost",
            ] {
                assert_eq!(metrics.iter().filter(|m| m["name"] == name).count(), 1);
            }
            assert!(
                (metrics.iter().find(|m| m["name"] == "pi.ai.cost").unwrap()["value"]
                    .as_f64()
                    .unwrap()
                    - cost)
                    .abs()
                    < 1e-12
            );
            for (i, k) in ["input", "output", "cache_read", "cache_write"]
                .iter()
                .enumerate()
            {
                assert_eq!(
                    spans[0]["attributes"][format!("usage.{k}")].as_f64(),
                    expected[i].as_f64()
                );
                let metric = row["metrics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|m| m["name"] == "pi.ai.tokens" && m["attributes"]["kind"] == *k)
                    .unwrap();
                assert_eq!(metric["value"].as_f64(), expected[i].as_f64());
                assert_eq!(
                    metrics
                        .iter()
                        .filter(|m| m["name"] == "pi.ai.tokens" && m["attributes"]["kind"] == *k)
                        .count(),
                    1
                );
            }
        }
    }
}

#[test]
fn merged_delta_validation_is_atomic_and_cost_zero_has_authority() {
    use opi_ai::api::anthropic::{
        AnthropicEvent, AnthropicMapper, AnthropicUsageDelta, ParsedEvent, parse_sse_events,
        parse_sse_events_for_openrouter,
    };
    let mut mapper = AnthropicMapper::new();
    mapper.process(AnthropicEvent::MessageStart {
        id: None,
        model: None,
        usage: opi_ai::stream::Usage::reported(11, 7, 5, 3, None, None),
    });
    assert!(
        mapper
            .process(AnthropicEvent::MessageDelta {
                stop_reason: Some("end_turn".into()),
                usage: AnthropicUsageDelta {
                    cache_write_1h_tokens: Some(2),
                    ..Default::default()
                }
            })
            .is_empty()
    );
    let failed = mapper
        .process(AnthropicEvent::MessageDelta {
            stop_reason: Some("max_tokens".into()),
            usage: AnthropicUsageDelta {
                input_tokens: Some(0),
                cache_write_tokens: Some(1),
                ..Default::default()
            },
        })
        .pop()
        .unwrap()
        .unwrap_err();
    let opi_ai::provider::ProviderError::StreamFailure(failed) = failed else {
        panic!("canonical failure required")
    };
    assert_eq!(failed.message().usage.input_tokens, 11);
    assert_eq!(failed.message().usage.cache_write_tokens, 3);
    assert_eq!(failed.message().usage.cache_write_1h_tokens, Some(2));
    assert_eq!(
        failed.message().raw_stop_reason.as_deref(),
        Some("end_turn")
    );
    for openrouter in [false, true] {
        let sse = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"cache_creation_input_tokens\":3,\"cache_creation_input_tokens_1h\":2}}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"cache_creation_input_tokens\":0,\"cache_creation_input_tokens_1h\":0}}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{},\"usage\":{\"cost\":0,\"is_byok\":false}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";
        let events: Vec<_> = if openrouter {
            parse_sse_events_for_openrouter(sse).collect()
        } else {
            parse_sse_events(sse).collect()
        };
        let mut mapper = AnthropicMapper::new();
        let mut last = None;
        for event in events {
            let ParsedEvent::Valid(event) = event else {
                panic!("valid frame")
            };
            for item in mapper.process(event) {
                if let AssistantStreamEvent::Done { message, .. } = item.unwrap() {
                    last = Some(message)
                }
            }
        }
        let result = last.unwrap();
        assert_eq!(result.usage.cache_write_tokens, 0);
        assert_eq!(result.usage.cache_write_1h_tokens, Some(0));
        assert_eq!(result.usage.cost.total_from_provider, openrouter);
        assert_eq!(result.usage.cost.total_cost(), 0.0);
    }
}

#[test]
fn reasoning_delta_subset_validation_is_atomic() {
    use opi_ai::api::anthropic::{AnthropicEvent, AnthropicMapper, AnthropicUsageDelta};

    let mut mapper = AnthropicMapper::new();
    mapper.process(AnthropicEvent::MessageStart {
        id: None,
        model: None,
        usage: opi_ai::stream::Usage::reported(11, 7, 5, 3, Some(2), None),
    });
    let failure = mapper
        .process(AnthropicEvent::MessageDelta {
            stop_reason: None,
            usage: AnthropicUsageDelta {
                output_tokens: Some(0),
                reasoning_tokens: Some(8),
                ..Default::default()
            },
        })
        .pop()
        .expect("invalid reasoning update fails")
        .expect_err("invalid reasoning update is a stream failure");
    let opi_ai::provider::ProviderError::StreamFailure(failure) = failure else {
        panic!("canonical failure required");
    };
    assert_eq!(failure.message().usage.output_tokens, 7);
    assert_eq!(failure.message().usage.reasoning_tokens, None);
    assert_eq!(failure.message().usage.cache_write_1h_tokens, Some(2));
}
