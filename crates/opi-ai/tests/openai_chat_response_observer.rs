//! OpenAI-compatible Chat response observer failure and release contract.

use opi_ai::message::{InputContent, Message, UserMessage};
use opi_ai::provider::{Provider, Request};
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::ChatStream;
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{CallbackError, callbacks::RequestCallbacks};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &str = include_str!("data/openai-chat-response-observer.json");

fn request() -> Request {
    Request {
        model: "openai:gpt-4o".into(),
        context: normalize_context(Context {
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "observer fixture".into(),
                }],
                timestamp_ms: 1,
            })],
            ..Default::default()
        })
        .unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: Some(128),
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
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

const SUCCESS_SSE: &str = concat!(
    "data: {\"id\":\"observer-1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
    "data: {\"id\":\"observer-1\",\"object\":\"chat.completion.chunk\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":1,\"total_tokens\":3}}\n\n",
    "data: [DONE]\n\n",
);

#[tokio::test]
async fn response_observer_failure_releases_provider_for_next_request() {
    let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
    assert_eq!(
        fixture["expected"]["observer_statuses"],
        serde_json::json!([200, 200])
    );
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(SUCCESS_SSE),
        )
        .expect(2)
        .mount(&server)
        .await;

    let provider = OpenAiChatProvider::new(Some(server.uri()));
    let failures = Arc::new(AtomicUsize::new(0));
    let observed = failures.clone();
    let mut first_request = request();
    first_request.callbacks = RequestCallbacks::new().with_async_on_response(move |status, _| {
        let observed = observed.clone();
        async move {
            assert_eq!(status, 200);
            if observed.fetch_add(1, Ordering::SeqCst) == 0 {
                Err(CallbackError::new("response observer fixture failure"))
            } else {
                Ok(())
            }
        }
    });

    let first = ChatStream::new(
        provider.stream_prepared(first_request, opi_ai::test_support::resolved_bearer_auth()),
    )
    .result()
    .await
    .expect_err("response observer must fail the first call");
    assert!(matches!(
        first.cause(),
        opi_ai::provider::ProviderError::CallbackPhase(_)
    ));
    assert_eq!(
        first.message().stop_reason,
        opi_ai::stream::StopReason::Error
    );

    let second = ChatStream::new(
        provider.stream_prepared(request(), opi_ai::test_support::resolved_bearer_auth()),
    )
    .result()
    .await
    .expect("the response body must be released for a later request");
    assert_eq!(second.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(second.usage.input_tokens, 2);
    assert_eq!(second.usage.output_tokens, 1);
    assert_eq!(failures.load(Ordering::SeqCst), 1);
    server.verify().await;
    println!(
        "{}",
        serde_json::json!({
            "case": fixture["id"],
            "observer_statuses": [200, 200],
            "first": {"stop": first.message().stop_reason, "event_types": ["error"]},
            "second": {"stop": second.stop_reason, "usage": {"input": second.usage.input_tokens, "output": second.usage.output_tokens, "total": second.usage.total_tokens}},
            "requests": 2,
        })
    );
}
