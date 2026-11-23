//! Shared Vertex request, stream and provider fixtures.

use super::*;

pub(crate) fn direct_cause(error: &ProviderError) -> &ProviderError {
    match error {
        ProviderError::StreamFailure(failure) => failure.cause(),
        cause => cause,
    }
}

pub(crate) fn accepted_failure_cause<'a>(
    error: &'a ProviderError,
    request: &Request,
) -> &'a ProviderError {
    use sha2::Digest as _;
    let ProviderError::StreamFailure(failure) = error else {
        panic!("accepted projection must retain its failure carrier: {error:?}");
    };
    let facts = failure
        .message()
        .projection
        .as_ref()
        .expect("accepted projection is known");
    let bytes = serde_json::to_vec(request.context.messages()).unwrap();
    assert_eq!(
        facts.source_prefix.message_count,
        request.context.messages().len()
    );
    assert_eq!(
        facts.source_prefix.sha256,
        format!("{:x}", sha2::Sha256::digest(bytes))
    );
    assert_eq!(
        facts.temporal.mode,
        opi_ai::transcript::ProjectionMode::Lossless
    );
    assert!(facts.temporal.affected_ranges.is_empty());
    assert!(facts.replay.losses.is_empty());
    failure.cause()
}

pub(crate) fn vertex_provider() -> VertexProvider {
    VertexProvider::new("my-project".into(), "us-central1".into(), None)
}

pub(crate) async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

pub(crate) fn text_sse_fixture() -> String {
    concat!(
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"Hello\"}]},\"index\":0}]}\n\n",
        "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"\"}]},\"finishReason\":\"STOP\",\"index\":0}],\"usageMetadata\":{\"promptTokenCount\":42,\"candidatesTokenCount\":13,\"totalTokenCount\":55}}\n\n",
    ).into()
}

pub(crate) fn tool_call_sse_fixture() -> String {
    let data = serde_json::json!({
        "candidates": [{
            "content": {
                "role": "model",
                "parts": [{
                    "functionCall": {
                        "name": "read_file",
                        "args": {"path": "foo.rs"}
                    }
                }]
            },
            "index": 0
        }],
        "usageMetadata": {
            "promptTokenCount": 20,
            "candidatesTokenCount": 10,
            "totalTokenCount": 30
        }
    });
    format!("data: {data}\n\n")
}

pub(crate) fn lifecycle_text_request() -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: "vertex:gemini-2.5-flash".into(),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: Some("You are helpful.".into()),
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "Hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
        })
        .expect("valid context declarations"),
        max_tokens: Some(1024),
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
