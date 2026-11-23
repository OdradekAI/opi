//! Capture the native, neutral and injected-auth public Chat requests on loopback.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::api::openai_completions::OpenAiCompletionsChatApi;
use opi_ai::auth::AuthResolver;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::message::{AssistantContent, InputContent, Message, UserMessage};
use opi_ai::model_info::ChatStoreField;
use opi_ai::provider::{CacheRetention, ProviderError, Request};
use opi_ai::providers::openai::{CompatConfig, OpenAiChatProvider};
use opi_ai::stream::Usage;
use opi_ai::test_support::{base_assistant, resolved_bearer_auth};
use opi_ai::transcript::{Context, normalize_context};
use opi_ai::{
    AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, ResolvedAuth,
    SimpleOptions, WireApi, WireCompat,
};
use serde::Deserialize;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &str = include_str!("data/openai-chat-output-limits.json");

#[derive(Deserialize)]
struct Fixture {
    model: Model,
    fields: Vec<String>,
    input: String,
    history: [String; 2],
    cases: Vec<Case>,
    sse: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
    name: String,
    context_window: u64,
    max_tokens: u64,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    anchor: Option<u32>,
    max_tokens: Option<u64>,
    simple_limit: Option<u64>,
}

#[derive(Default)]
struct CountingAuth(AtomicUsize);

impl AuthResolver for CountingAuth {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            let mut auth = resolved_bearer_auth();
            auth.secret = "fixture-route-key".into();
            Ok(auth)
        })
    }
}

fn user(text: &str, timestamp_ms: i64) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text { text: text.into() }],
        timestamp_ms,
    })
}

fn request(fixture: &Fixture, case: &Case) -> Request {
    let mut messages = Vec::new();
    if let Some(anchor) = case.anchor {
        messages.push(user(&fixture.history[0], 1));
        let mut previous = base_assistant();
        previous.api = opi_ai::ApiKind::OpenAi;
        previous.provider = "openai".into();
        previous.model = fixture.model.id.clone();
        previous.wire = Some(WireApi::OpenAiCompletions);
        previous.timestamp_ms = 2;
        previous.content = vec![AssistantContent::Text {
            text: fixture.history[1].clone(),
            signature: None,
        }];
        previous.usage = Usage::reported(anchor - 1, 1, 0, 0, None, None)
            .with_total_tokens(Some(u64::from(anchor)));
        messages.push(Message::Assistant(previous));
    }
    messages.push(user(&fixture.input, 3));
    Request {
        model: format!("openai:{}", fixture.model.id),
        context: normalize_context(Context {
            messages,
            ..Default::default()
        })
        .unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: case.max_tokens,
        temperature: None,
        sampling_params: None,
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: Default::default(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::Disabled,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

#[tokio::test]
async fn observe_chat_output_limits() {
    let fixture: Fixture = serde_json::from_str(FIXTURE).unwrap();
    let server = MockServer::start().await;
    let sse = fixture.sse.clone();
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().unwrap();
            let hit_limit = ["max_completion_tokens", "max_tokens"]
                .iter()
                .any(|field| body[field].as_u64() == Some(1));
            let stop = if hit_limit { "length" } else { "stop" };
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse[stop].clone())
        })
        .expect((fixture.fields.len() * fixture.cases.len() * 3) as u64)
        .mount(&server)
        .await;

    let mut rows = Vec::new();
    let mut requests_seen = 0;
    for field in &fixture.fields {
        let model = ModelInfo::new(
            &fixture.model.id,
            &fixture.model.name,
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(fixture.model.context_window, fixture.model.max_tokens)
                .with_streaming(true),
        )
        .with_base_url(format!("{}/v1", server.uri()))
        .with_compat(WireCompat::OpenAiCompletions(CompatConfig {
            max_tokens_field: Some(field.clone()),
            usage_in_stream: Some(true),
            store: Some(ChatStoreField::SendFalse),
            ..Default::default()
        }))
        .unwrap();
        let record = ChatModelRecord::new("openai", model.clone());
        let provider = Arc::new(OpenAiChatProvider::new(Some(server.uri())));
        let api = OpenAiCompletionsChatApi::new(provider, model).unwrap();
        let resolver = Arc::new(CountingAuth::default());
        let mut models = Models::new();
        let handle = models
            .register_chat_api(
                Arc::new(api),
                resolver.clone(),
                AuthProvenanceSource::Static,
            )
            .unwrap();

        for case in &fixture.cases {
            for entry in ["native", "simple", "direct"] {
                let mut request = request(&fixture, case);
                let estimate =
                    opi_ai::context::estimate_context_tokens(None, request.context.messages(), &[])
                        .tokens;
                let resolves_before = resolver.0.load(Ordering::SeqCst);
                let call = match entry {
                    "native" => {
                        models.stream(record.clone(), request, handle.options(Default::default()))
                    }
                    "simple" => {
                        request.max_tokens = None;
                        models.stream_simple(
                            record.clone(),
                            request,
                            SimpleOptions {
                                max_tokens: case.max_tokens,
                                ..Default::default()
                            },
                        )
                    }
                    "direct" => {
                        let mut auth = resolved_bearer_auth();
                        auth.secret = "fixture-direct-key".into();
                        models.stream_direct(
                            record.clone(),
                            request,
                            handle.options(Default::default()),
                            auth,
                        )
                    }
                    _ => unreachable!(),
                };
                let result = call.result().await.unwrap();
                let sent = server.received_requests().await.unwrap();
                assert_eq!(sent.len(), requests_seen + 1, "{field}/{}/{entry}", case.id);
                let sent = &sent[requests_seen];
                requests_seen += 1;
                let [AssistantContent::Text { text, .. }] = result.content.as_slice() else {
                    panic!("expected a single text block, got {:?}", result.content);
                };
                let body: Value = sent.body_json().unwrap();
                let limit = if entry == "simple" {
                    case.simple_limit
                } else {
                    case.max_tokens.filter(|limit| *limit != 0)
                };
                for name in &fixture.fields {
                    let expected = limit.filter(|_| name == field).map(Value::from);
                    assert_eq!(
                        body.get(name),
                        expected.as_ref(),
                        "{field}/{}/{entry}",
                        case.id
                    );
                }
                rows.push(json!({
                    "field": field,
                    "case": case.id,
                    "entry": entry,
                    "estimate": estimate,
                    "resolver_calls": resolver.0.load(Ordering::SeqCst) - resolves_before,
                    "request": {
                        "method": sent.method.as_str(),
                        "path": sent.url.path(),
                        "authorization": sent.headers["authorization"].to_str().unwrap(),
                        "body": body,
                    },
                    "result": {
                        "text": text,
                        "stop": result.stop_reason.as_str(),
                        "raw_stop": result.raw_stop_reason,
                        "response_id": result.response_id,
                        "input_tokens": result.usage.input_tokens,
                        "output_tokens": result.usage.output_tokens,
                        "total_tokens": result.usage.total_tokens,
                        "error": result.error_message,
                    },
                }));
            }
        }
    }
    server.verify().await;
    println!("{}", json!(rows));
}
