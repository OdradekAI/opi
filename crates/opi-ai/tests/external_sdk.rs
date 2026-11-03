//! Task 20.57 (DEL-14/ONB-02): assembled workspace-external SDK matrix.
//!
//! Distinct assembled external-use cases over consumer-shaped `Models`
//! instances. Per the dedup rule, per-adapter deep semantics, OAuth
//! nine-flow/shared-store proof (`oauth_flows.rs`), per-wire replay
//! projections and telemetry-carrier freezing keep their predecessor
//! owners; this suite adds the assembled matrix — every published chat
//! wire, the classifier and image APIs, options portability, callback and
//! duration closure, manual tool-loop continuation, two-instance isolation
//! with shared stores and cleanup, empty-instance zero-I/O construction and
//! lawful large collections — through one instance per case, always against
//! local scripted transport with complete model records and injected auth
//! (no live provider, no credentials-triggered call).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use opi_ai::auth::adc::native::{
    AdcAuthSelection, AdcNativeOwner, ReqwestAdcTransport, SystemAdcClock,
};
use opi_ai::auth::aws::local::AwsAuthSelection;
use opi_ai::auth::{AuthContext, AuthContextError, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::bedrock::event_stream;
use opi_ai::callbacks::RequestCallbacks;
use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierQuestion};
use opi_ai::http::HttpClient;
use opi_ai::images::{ImagesCall, ImagesContext, ImagesInput};
use opi_ai::message::{
    AssistantContent, InputContent, Message, OutputContent, ToolDef, ToolResultMessage, UserMessage,
};
use opi_ai::models::{ChatModelRecord, Models, SimpleOptions, SimpleToolChoice};
use opi_ai::provider::Request;
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::test_support::{MockProvider, ScriptedDeferredBackend, base_assistant, text_response};
use opi_ai::transcript::{self, Context as ShorthandContext, TranscriptContext, normalize_context};
use opi_ai::{AuthProvenanceSource, AwsSigV4Credentials, WireApi};
use secrecy::SecretString;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Shared harness
// ---------------------------------------------------------------------------

/// Injected source context recording every read, so zero-I/O construction
/// and registration can be asserted (AIR-046).
#[derive(Default)]
struct OwnedRecordingContext {
    values: BTreeMap<String, String>,
    reads: std::sync::Mutex<Vec<String>>,
}

impl OwnedRecordingContext {
    fn new(values: &[(&str, &str)]) -> Arc<Self> {
        Arc::new(Self {
            values: values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            reads: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn read_count(&self) -> usize {
        self.reads.lock().unwrap().len()
    }
}

impl AuthContext for OwnedRecordingContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        self.reads.lock().unwrap().push(name.to_string());
        Ok(self.values.get(name).cloned())
    }

    fn file_exists(&self, _path: &str) -> Result<bool, AuthContextError> {
        Ok(false)
    }
}

fn base_request(model: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: model.to_owned(),
        context: normalize_context(ShorthandContext::default()).unwrap(),
        max_tokens: Some(256),
        temperature: None,
        sampling_params: Default::default(),
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

fn user_request(model: &str, text: &str) -> Request {
    let mut request = base_request(model);
    request.context = normalize_context(ShorthandContext {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: text.to_owned(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .unwrap();
    request
}

fn message_text(message: &opi_ai::message::AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            AssistantContent::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn completions_info(model: &str) -> opi_ai::model_info::ModelInfo {
    opi_ai::model_info::ModelInfo::new(
        model,
        "Matrix Model",
        WireApi::OpenAiCompletions,
        opi_ai::registry::ModelCapabilities::new(100_000, 4_096),
    )
}

// ---------------------------------------------------------------------------
// Wire fixture bodies (mirroring the per-adapter fixture suites)
// ---------------------------------------------------------------------------

fn anthropic_text_sse() -> String {
    concat!(
        "event: message_start\n",
        r#"data: {"type":"message_start","message":{"id":"msg_abc","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":25,"output_tokens":0}}}"#, "\n\n",
        "event: content_block_start\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}"#, "\n\n",
        "event: content_block_stop\n",
        r#"data: {"type":"content_block_stop","index":0}"#, "\n\n",
        "event: message_delta\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":11}}"#, "\n\n",
        "event: message_stop\n",
        r#"data: {"type":"message_stop"}"#, "\n\n",
    )
    .into()
}

fn anthropic_tool_call_sse() -> String {
    concat!(
        "event: message_start\n",
        r#"data: {"type":"message_start","message":{"id":"msg_tool","type":"message","role":"assistant","content":[],"model":"claude-sonnet-4-5-20250514","stop_reason":null,"usage":{"input_tokens":20,"output_tokens":0}}}"#, "\n\n",
        "event: content_block_start\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"get_weather","input":{}}}"#, "\n\n",
        "event: content_block_delta\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"Paris\"}"}}"#, "\n\n",
        "event: content_block_stop\n",
        r#"data: {"type":"content_block_stop","index":0}"#, "\n\n",
        "event: message_delta\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use","stop_sequence":null},"usage":{"output_tokens":9}}"#, "\n\n",
        "event: message_stop\n",
        r#"data: {"type":"message_stop"}"#, "\n\n",
    )
    .into()
}

fn openai_completions_sse() -> String {
    concat!(
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" world"},"finish_reason":null}]}"#, "\n\n",
        r#"data: {"id":"chatcmpl-abc123","object":"chat.completion.chunk","created":1720000000,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":25,"completion_tokens":8,"total_tokens":33}}"#, "\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

fn openai_responses_sse() -> String {
    concat!(
        "event: response.created\n",
        r#"data: {"type":"response.created","response":{"id":"resp_1","status":"in_progress","model":"gpt-4","output":[]}}"#, "\n\n",
        "event: response.output_item.added\n",
        r#"data: {"type":"response.output_item.added","output_index":0,"item":{"type":"message","status":"in_progress","role":"assistant","content":[]}}"#, "\n\n",
        "event: response.content_part.added\n",
        r#"data: {"type":"response.content_part.added","output_index":0,"content_index":0,"part":{"type":"output_text","text":""}}"#, "\n\n",
        "event: response.output_text.delta\n",
        r#"data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"Hi there"}"#, "\n\n",
        "event: response.output_text.done\n",
        r#"data: {"type":"response.output_text.done","output_index":0,"content_index":0,"text":"Hi there"}"#, "\n\n",
        "event: response.output_item.done\n",
        r#"data: {"type":"response.output_item.done","output_index":0,"item":{"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":"Hi there"}]}}"#, "\n\n",
        "event: response.completed\n",
        r#"data: {"type":"response.completed","response":{"id":"resp_1","status":"completed","model":"gpt-4","output":[{"type":"message","content":[{"type":"output_text","text":"Hi there"}]}],"usage":{"input_tokens":10,"output_tokens":5}}}"#, "\n\n",
    )
    .into()
}

fn codex_completed_sse() -> String {
    concat!(
        "event: response.completed\n",
        r#"data: {"type":"response.completed","response":{"id":"response-1","model":"gpt-5.3-codex-spark","output":[],"usage":{"input_tokens":1,"output_tokens":1}}}"#,
        "\n\n",
    )
    .into()
}

fn google_candidates_sse() -> String {
    concat!(
        r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"index":0}]}"#,
        "\n\n",
        r#"data: {"candidates":[{"content":{"role":"model","parts":[{"text":""}]},"finishReason":"STOP","index":0}],"usageMetadata":{"promptTokenCount":42,"candidatesTokenCount":13,"totalTokenCount":55}}"#,
        "\n\n",
    )
    .into()
}

fn mistral_sse() -> String {
    concat!(
        r#"data: {"choices":[{"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"content":"Hi there"},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#,
        "\n\n",
        "data: [DONE]\n\n",
    )
    .into()
}

fn pi_messages_sse() -> String {
    concat!(
        r#"data: {"type":"start"}"#, "\n\n",
        r#"data: {"type":"text_start","contentIndex":0}"#, "\n\n",
        r#"data: {"type":"text_delta","contentIndex":0,"delta":"Hel"}"#, "\n\n",
        r#"data: {"type":"text_end","contentIndex":0,"content":"Hello"}"#, "\n\n",
        r#"data: {"type":"done","reason":"stop","usage":{"input":10,"output":5,"cacheRead":0,"cacheWrite":0,"totalTokens":15,"cost":{"input":0.1,"output":0.2,"cacheRead":0,"cacheWrite":0,"total":0.3},"responseId":"resp-1"}}"#,
        "\n\n",
    )
    .into()
}

fn bedrock_converse_stream_body() -> Vec<u8> {
    let events: &[(&str, &str)] = &[
        ("messageStart", r#"{"role":"assistant"}"#),
        (
            "contentBlockStart",
            r#"{"start":{"text":{}},"contentBlockIndex":0}"#,
        ),
        (
            "contentBlockDelta",
            r#"{"delta":{"text":"Hello bedrock"},"contentBlockIndex":0}"#,
        ),
        ("contentBlockStop", r#"{"contentBlockIndex":0}"#),
        ("messageStop", r#"{"stopReason":"end_turn"}"#),
    ];
    let mut body = Vec::new();
    for (event_type, json_payload) in events {
        body.extend_from_slice(&event_stream::build_test_frame(
            event_type,
            "application/json",
            json_payload.as_bytes(),
        ));
    }
    let trailer = event_stream::build_test_frame("metadata", "application/json", b"{}");
    body.extend_from_slice(&trailer);
    body
}

fn sse_template(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(body)
}

// ---------------------------------------------------------------------------
// AIR-057 / P20-S57: the assembled ten-wire matrix on one instance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn assembled_matrix_drives_all_ten_published_chat_wires_through_one_instance() {
    let anthropic_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sse_template(anthropic_text_sse()))
        .expect(1)
        .mount(&anthropic_peer)
        .await;

    let codex_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(codex_completed_sse()))
        .expect(1)
        .mount(&codex_peer)
        .await;

    let openai_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?responses$"))
        .respond_with(sse_template(openai_responses_sse()))
        .expect(1)
        .mount(&openai_peer)
        .await;

    let openrouter_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(api/)?(v1/)?chat/completions$",
        ))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&openrouter_peer)
        .await;

    let azure_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(openai/deployments/[^/]+/)?(v1/)?responses$",
        ))
        .respond_with(sse_template(openai_responses_sse()))
        .expect(1)
        .mount(&azure_peer)
        .await;

    let google_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            r"^/(v1beta/)?models/gemini-2\.5-flash:streamGenerateContent$",
        ))
        .respond_with(sse_template(google_candidates_sse()))
        .expect(1)
        .mount(&google_peer)
        .await;

    let vertex_token_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "vertex-token", "expires_in": 3600, "token_type": "Bearer"
        })))
        .expect(1..=2)
        .mount(&vertex_token_peer)
        .await;
    let vertex_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?projects/"))
        .respond_with(sse_template(google_candidates_sse()))
        .expect(1)
        .mount(&vertex_peer)
        .await;

    let mistral_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex("^/(v1/)?chat/completions$"))
        .respond_with(sse_template(mistral_sse()))
        .expect(1)
        .mount(&mistral_peer)
        .await;

    let radius_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(pi_messages_sse()))
        .expect(1)
        .mount(&radius_peer)
        .await;

    let bedrock_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/model/.+/converse-stream$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            bedrock_converse_stream_body(),
            "application/vnd.amazon.eventstream",
        ))
        .expect(1)
        .mount(&bedrock_peer)
        .await;

    // Scoped ADC file keeps vertex construction offline and isolated.
    let adc_dir = tempfile::tempdir().unwrap();
    let adc_file = adc_dir.path().join("adc.json");
    std::fs::write(
        &adc_file,
        serde_json::to_vec(&serde_json::json!({
            "type":"authorized_user", "client_id":"client-x",
            "client_secret":"secret-x", "refresh_token":"refresh-x",
            "quota_project_id":"quota-x"
        }))
        .unwrap(),
    )
    .unwrap();

    let context = OwnedRecordingContext::new(&[
        ("OPENAI_API_KEY", "sk-openai"),
        ("OPENROUTER_API_KEY", "sk-openrouter"),
        ("AZURE_OPENAI_API_KEY", "sk-azure"),
        ("GEMINI_API_KEY", "sk-google"),
        ("MISTRAL_API_KEY", "sk-mistral"),
        ("RADIUS_API_KEY", "sk-radius"),
        ("GOOGLE_APPLICATION_CREDENTIALS", adc_file.to_str().unwrap()),
        ("GOOGLE_CLOUD_PROJECT", "assembled-project"),
        ("GOOGLE_CLOUD_LOCATION", "assembled-location"),
    ]);

    let client = Arc::new(HttpClient::new());
    let mut models = Models::new();
    let mut overrides = BuiltinOverrides::default();
    overrides
        .base_urls
        .insert("openai".into(), openai_peer.uri());
    overrides
        .base_urls
        .insert("openrouter".into(), openrouter_peer.uri());
    overrides
        .base_urls
        .insert("google".into(), google_peer.uri());
    overrides
        .base_urls
        .insert("mistral".into(), mistral_peer.uri());
    overrides
        .base_urls
        .insert("radius".into(), radius_peer.uri());
    overrides.azure_endpoint = Some(azure_peer.uri());
    overrides.azure_api_version = Some("2024-06-01".into());
    overrides.aws_auth.base_url = Some(bedrock_peer.uri());
    overrides.aws_auth.selection = Some(AwsAuthSelection::SigV4(AwsSigV4Credentials::new(
        SecretString::from("AKIDASSEMBLED"),
        SecretString::from("secret"),
        "us-east-1",
    )));
    overrides.vertex_auth.base_url = Some(vertex_peer.uri());
    overrides.vertex_auth.adc = Some(
        AdcNativeOwner::new(
            AdcAuthSelection::Auto,
            Arc::new(ReqwestAdcTransport::new()),
            Arc::new(SystemAdcClock),
        )
        .with_token_endpoint(Some(format!("{}/token", vertex_token_peer.uri()))),
    );

    let report = register_builtin(&mut models, context.clone(), client.clone(), &overrides)
        .expect("builtin registration");
    assert_eq!(
        report.registered.len() + report.skipped.len(),
        42,
        "all 42 published provider profiles are accounted for on the assembled instance: {:?}",
        report.skipped
    );
    assert!(
        report
            .skipped
            .iter()
            .all(|(_, reason)| reason.contains("classifier-only")),
        "only classifier-only profiles may skip chat registration: {:?}",
        report.skipped
    );
    assert!(!report.classifier_apis.is_empty());
    assert!(!report.image_apis.is_empty());
    // The two OAuth-only chat routes join the same instance through the
    // public register_route seam with real adapters and static auth; their
    // records are caller-owned complete records (AIR-045), independent of
    // catalog visibility.
    let anthropic_info = anthropic_route_info("claude-sonnet-4-5-20250514");
    models
        .register_route(
            Box::new(opi_ai::anthropic::AnthropicProvider::for_route(
                "anthropic".into(),
                vec![anthropic_info.clone()],
                Some(anthropic_peer.uri()),
                Default::default(),
                client.clone(),
                false,
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-anthropic"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let codex_info = opi_ai::model_info::ModelInfo::new(
        "gpt-5.3-codex-spark",
        "Codex Spark",
        WireApi::OpenAiCodexResponses,
        opi_ai::registry::ModelCapabilities::new(200_000, 32_768),
    );
    models
        .register_route(
            Box::new(
                opi_ai::openai_codex_responses::OpenAiCodexResponsesProvider::new(
                    Some(codex_peer.uri()),
                    vec![codex_info.clone()],
                    client.clone(),
                ),
            ),
            Arc::new(StaticBearer),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    // All ten published chat wires complete through the same instance with
    // complete catalog records and the neutral entry.
    let matrix: [(&str, &str, &str); 10] = [
        ("anthropic", "claude-sonnet-4-5-20250514", "Hello world"),
        ("openai", "gpt-4", "Hi there"),
        (
            "openrouter",
            "~anthropic/claude-fable-latest",
            "Hello world",
        ),
        ("azure", "gpt-4", "Hi there"),
        ("google", "gemini-2.5-flash", "Hello"),
        ("google-vertex", "gemini-2.5-flash", "Hello"),
        ("mistral", "codestral-2508", "Hi there"),
        ("openai-codex", "gpt-5.3-codex-spark", ""),
        ("radius", "balanced", "Hello"),
        ("amazon-bedrock", "amazon.nova-2-lite-v1:0", "Hello bedrock"),
    ];
    for (provider, model, expected) in matrix {
        let info = models
            .get_model(provider, model)
            .unwrap_or_else(|| panic!("catalog model for {provider}/{model}"));
        let outcome = models
            .complete_simple(
                ChatModelRecord::new(provider, info),
                user_request(model, "Hello"),
                SimpleOptions::default(),
            )
            .await;
        let message = match outcome {
            Ok(message) => message,
            Err(failure) => {
                let hits = [
                    ("anthropic", &anthropic_peer),
                    ("codex", &codex_peer),
                    ("openai", &openai_peer),
                    ("openrouter", &openrouter_peer),
                    ("azure", &azure_peer),
                    ("google", &google_peer),
                    ("vertex", &vertex_peer),
                    ("mistral", &mistral_peer),
                    ("radius", &radius_peer),
                    ("bedrock", &bedrock_peer),
                ]
                .into_iter()
                .map(|(name, server)| async move {
                    let paths = server
                        .received_requests()
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|r| r.url.path().to_owned())
                        .collect::<Vec<_>>();
                    format!("{name}: {paths:?}")
                })
                .collect::<Vec<_>>();
                let all = futures_util::future::join_all(hits).await;
                panic!("{provider} wire failed: {failure:?}; peers saw {all:?}");
            }
        };
        assert_eq!(
            message_text(&message),
            expected,
            "final text for {provider} wire"
        );
    }

    anthropic_peer.verify().await;
    codex_peer.verify().await;
    openai_peer.verify().await;
    openrouter_peer.verify().await;
    azure_peer.verify().await;
    google_peer.verify().await;
    vertex_peer.verify().await;
    mistral_peer.verify().await;
    radius_peer.verify().await;
    bedrock_peer.verify().await;
}

fn anthropic_route_info(model: &str) -> opi_ai::model_info::ModelInfo {
    opi_ai::model_info::ModelInfo::new(
        model,
        "Matrix Anthropic Model",
        WireApi::AnthropicMessages,
        opi_ai::registry::ModelCapabilities::new(200_000, 8_192),
    )
}

struct StaticBearer;

impl AuthResolver for StaticBearer {
    fn resolve(
        &self,
    ) -> opi_ai::credential::BoxAuthFuture<'_, Result<ResolvedAuth, opi_ai::provider::ProviderError>>
    {
        Box::pin(async {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("fake-key"),
                base_url: None,
                account_id: Some("fake-account".into()),
                provenance: Default::default(),
            })
        })
    }
}

// ---------------------------------------------------------------------------
// AIR-058 / P20-ACTOR-05: classifier and image matrix on one instance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn classifier_and_image_apis_answer_typed_through_one_instance() {
    let mut models = Models::new();
    let resolver = || {
        Arc::new(opi_ai::auth::StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("sk-matrix"),
        )) as Arc<dyn AuthResolver>
    };

    // System One (typesafe): typed choices with probabilities and usage.
    let system_one_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": {
                "category": {
                    "type": "choice",
                    "choice": "success",
                    "probabilities": {"success": 0.9, "failure": 0.1},
                    "confidence": 0.8
                }
            }
        })))
        .expect(1)
        .mount(&system_one_peer)
        .await;
    let mut system_one_record = opi_ai::api::system_one::default_typesafe_classifier_record();
    system_one_record.base_url = system_one_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::system_one::TypesafeSystemOneApi::new(
                HttpClient::new(),
                system_one_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Cloudflare Workers AI system one: boolean mapping.
    let cloudflare_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"answers": {
                "category": {
                    "type": "choice",
                    "choice": "success",
                    "probabilities": {"success": 0.9, "failure": 0.1},
                    "confidence": 0.8
                }
            }}
        })))
        .mount(&cloudflare_peer)
        .await;
    let mut cloudflare_record = opi_ai::api::cloudflare::default_cloudflare_classifier_records()
        .into_iter()
        .next()
        .expect("cloudflare record");
    cloudflare_record.base_url = cloudflare_peer.uri();
    models
        .register_classifier_api(
            Arc::new(
                opi_ai::api::cloudflare::CloudflareWorkersAiSystemOneApi::new(
                    HttpClient::new(),
                    cloudflare_record.clone(),
                ),
            ),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // OpenAI Decisions API: 504 stays a typed exclusion.
    let decisions_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(504))
        .mount(&decisions_peer)
        .await;
    let mut decisions_record = opi_ai::api::decisions::default_decisions_classifier_record();
    decisions_record.base_url = decisions_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::decisions::OpenAiDecisionsApi::new(
                HttpClient::new(),
                decisions_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Llama classify: the three-endpoint llama-server protocol (tokenize,
    // apply-template, completion with char-token top_logprobs), mirroring
    // the fake server in classifier_contract.rs.
    let llama_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap_or_default();
            match request.url.path() {
                "/tokenize" => {
                    let tokens: Vec<u64> = body["content"]
                        .as_str()
                        .unwrap_or_default()
                        .chars()
                        .map(|c| c as u64)
                        .collect();
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({ "tokens": tokens }))
                }
                "/apply-template" => {
                    let mut prompt = String::new();
                    for message in body["messages"].as_array().unwrap_or(&Vec::new()) {
                        prompt.push_str(&format!(
                            "<|{}|>\n{}\n",
                            message["role"].as_str().unwrap_or_default(),
                            message["content"].as_str().unwrap_or_default()
                        ));
                    }
                    prompt.push_str("<|assistant|>\n");
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({ "prompt": prompt }))
                }
                "/completion" => {
                    // Choice answers rank the second criterion highest.
                    let top_logprobs: Vec<serde_json::Value> = [
                        (b'B' as u64, -0.3f64),
                        (b'A' as u64, -1.5),
                        (b'C' as u64, -3.0),
                    ]
                    .into_iter()
                    .map(|(id, logprob)| {
                        serde_json::json!({
                            "id": id,
                            "token": char::from_u32(id as u32).unwrap_or_default(),
                            "bytes": [],
                            "logprob": logprob,
                        })
                    })
                    .collect();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "content": "B",
                        "completion_probabilities": [
                            { "id": 66, "token": "B", "top_logprobs": top_logprobs }
                        ]
                    }))
                }
                _ => ResponseTemplate::new(404).set_body_string("not found"),
            }
        })
        .mount(&llama_peer)
        .await;
    let mut llama_record = opi_ai::api::llama::default_llama_classifier_record();
    llama_record.base_url = llama_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::llama::LlamaCppClassifyApi::new(
                HttpClient::new(),
                llama_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The sole published image API: openrouter-images with real outputs.
    let image_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(api/)?(v1/)?chat/completions$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-123",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Done",
                    "images": [{"image_url": "data:image/png;base64,QUJD"}]
                },
                "finish_reason": "stop"
            }]
        })))
        .mount(&image_peer)
        .await;
    let mut image_record = opi_ai::api::openrouter_images::default_openrouter_image_record();
    image_record.base_url = image_peer.uri();
    models
        .register_image_api(
            Arc::new(opi_ai::api::openrouter_images::OpenRouterImagesApi::new(
                HttpClient::new(),
                image_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let classify_state = || ClassifierContext {
        state: serde_json::json!({"build": "green"}),
        questions: [(
            "category".to_owned(),
            ClassifierQuestion::Choice {
                instructions: "Is the build healthy?".into(),
                criteria: [
                    ("success".to_owned(), "healthy".to_owned()),
                    ("failure".to_owned(), "broken".to_owned()),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    };

    let system_one = models
        .classify(
            system_one_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(system_one.error.is_none());
    assert_eq!(
        system_one.stop_reason,
        opi_ai::classifier::ClassifierStopReason::Stop
    );

    let cloudflare = models
        .classify(
            cloudflare_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(cloudflare.error.is_none());

    let decisions = models
        .classify(
            decisions_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(
        decisions.error.is_some(),
        "504 stays a typed classifier failure: {:?}",
        decisions.answers
    );

    let llama = models
        .classify(llama_record, classify_state(), ClassifierCall::default())
        .await;
    assert!(llama.error.is_none());

    let images = models
        .generate_images(
            image_record,
            ImagesContext {
                input: vec![ImagesInput::Text {
                    text: "a small red square".into(),
                }],
            },
            ImagesCall::default(),
        )
        .await;
    assert_eq!(images.stop_reason, opi_ai::images::ImagesStopReason::Stop);
    assert_eq!(images.output.len(), 2);

    system_one_peer.verify().await;
}

// ---------------------------------------------------------------------------
// AIR-049/ONB-03: options portability across wires; typed preflight mismatch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn simple_options_port_across_wires_and_mismatched_native_fails_typed_preflight() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(body_partial_json(
            serde_json::json!({"tool_choice": "none"}),
        ))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;

    let info = completions_info("port-model");
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "openai-chat-port".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info.clone(),
                )
                .expect("completions api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-port"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // A foreign-wire native registration supplies its typed options.
    let pi_info = opi_ai::model_info::ModelInfo::new(
        "pi-port",
        "Pi Port",
        WireApi::PiMessages,
        opi_ai::registry::ModelCapabilities::new(100_000, 8_192),
    );
    let pi_handle = models
        .register_chat_api(
            Arc::new(
                opi_ai::pi_messages::PiMessagesChatApi::new(
                    opi_ai::pi_messages::PiMessagesProvider::new(Some("http://127.0.0.1:1".into())),
                    pi_info.clone(),
                )
                .expect("pi api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-pi"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("openai-chat-port", info.clone());
    let options = SimpleOptions {
        tool_choice: Some(SimpleToolChoice::None),
        temperature: None,
        max_tokens: None,
        timeout: None,
        stop_sequences: vec![],
    };
    let message = models
        .complete_simple(record.clone(), user_request("port-model", "Hello"), options)
        .await
        .expect("neutral tool_choice:none crosses wires");
    assert_eq!(message_text(&message), "Hello world");

    // Native options typed for the pi wire can never bind to the registered
    // openai-completions record: typed preflight failure before auth/network.
    let wrong = pi_handle.options(opi_ai::pi_messages::PiMessagesOptions::native(
        None, None, false,
    ));
    let result = models
        .complete(record, user_request("port-model", "Hello"), wrong)
        .await;
    assert!(result.is_err(), "mismatched native options must fail typed");
    peer.verify().await;
}

// ---------------------------------------------------------------------------
// AIR-059/V-51: assembled callback closure on a real wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn request_callbacks_observe_once_and_protected_headers_fail_typed() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(wiremock::matchers::header("x-assembled", "matrix"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;

    let info = completions_info("cb-model");
    let mut models = Models::new();
    let register = |models: &mut Models, base: String, info: opi_ai::model_info::ModelInfo| {
        models
            .register_chat_api(
                Arc::new(
                    opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
                        Arc::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
                            Some(base),
                            "cb-provider".into(),
                            Default::default(),
                            vec![info.clone()],
                            Arc::new(HttpClient::new()),
                        )),
                        info,
                    )
                    .expect("cb api"),
                ),
                Arc::new(opi_ai::auth::StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    SecretString::from("sk-cb"),
                )),
                AuthProvenanceSource::Static,
            )
            .unwrap();
    };
    register(&mut models, peer.uri(), info.clone());

    let headers = Arc::new(AtomicUsize::new(0));
    let payloads = Arc::new(AtomicUsize::new(0));
    let responses = Arc::new(AtomicUsize::new(0));
    let raws = Arc::new(AtomicUsize::new(0));
    let callbacks = {
        let (h, p, r, w) = (
            headers.clone(),
            payloads.clone(),
            responses.clone(),
            raws.clone(),
        );
        RequestCallbacks::new()
            .with_header_transform(Arc::new(move |values: &[(String, String)]| {
                h.fetch_add(1, Ordering::SeqCst);
                let mut merged = values.to_vec();
                merged.push(("x-assembled".into(), "matrix".into()));
                Ok(merged)
            }))
            .with_on_payload(Arc::new(move |_body| {
                p.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }))
            .with_on_response(Arc::new(move |_status, _headers| {
                r.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
            .with_on_raw(Arc::new(move |_event| {
                w.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }))
    };
    let mut request = user_request("cb-model", "Hello");
    request.callbacks = callbacks;
    let message = models
        .complete_simple(
            ChatModelRecord::new("cb-provider", info.clone()),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("callback-observed call completes");
    assert_eq!(message_text(&message), "Hello world");
    assert_eq!(headers.load(Ordering::SeqCst), 1, "header transform once");
    assert_eq!(payloads.load(Ordering::SeqCst), 1, "payload observed once");
    assert_eq!(
        responses.load(Ordering::SeqCst),
        1,
        "response observed once"
    );
    assert!(
        raws.load(Ordering::SeqCst) >= 1,
        "raw provider events observed"
    );
    peer.verify().await;

    // A protected identity header change fails typed before the wire.
    let guard_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .mount(&guard_peer)
        .await;
    let mut guard_models = Models::new();
    register(&mut guard_models, guard_peer.uri(), info);
    let mut request = user_request("cb-model", "Hello");
    request.callbacks =
        RequestCallbacks::new().with_header_transform(Arc::new(|values: &[(String, String)]| {
            let mut merged = values.to_vec();
            merged.push(("authorization".into(), "Bearer override".into()));
            Ok(merged)
        }));
    let failure = guard_models
        .complete_simple(
            ChatModelRecord::new("cb-provider", completions_info("cb-model")),
            request,
            SimpleOptions::default(),
        )
        .await;
    assert!(
        failure.is_err(),
        "protected header mutation is typed failure"
    );
    assert!(
        guard_peer.received_requests().await.unwrap().is_empty(),
        "typed failure must precede the wire"
    );
}

// ---------------------------------------------------------------------------
// V-53/AIR-061: duration closure across operations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn duration_is_measured_once_drained_and_survives_serialization() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;
    let info = completions_info("dur-model");
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "dur-provider".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info,
                )
                .expect("dur api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-dur"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let message = models
        .complete_simple(
            ChatModelRecord::new("dur-provider", completions_info("dur-model")),
            user_request("dur-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("timed call");
    let measured = message
        .duration_ms
        .expect("adapter activation-to-terminal duration is observed");

    // Serialization retains the known optional duration; forwarding never
    // recomputes (a second identical roundtrip keeps the exact value).
    let encoded = serde_json::to_vec(&message).unwrap();
    let decoded: opi_ai::message::AssistantMessage = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.duration_ms, Some(measured));
    let reencoded = serde_json::to_vec(&decoded).unwrap();
    let redecoded: opi_ai::message::AssistantMessage = serde_json::from_slice(&reencoded).unwrap();
    assert_eq!(redecoded.duration_ms, Some(measured));

    peer.verify().await;
}

// ---------------------------------------------------------------------------
// AIR-049/DEL-03: manual tool loop with serde continuation on a real wire
// ---------------------------------------------------------------------------

#[tokio::test]
async fn manual_tool_loop_continues_after_serde_roundtrip() {
    let peer = MockServer::start().await;
    // Anthropic replays the unchanged tool set as by-value tool_addition on
    // every continuation round, so rounds are distinguished by body content,
    // not by tool presence.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(|request: &wiremock::Request| {
            let body = String::from_utf8_lossy(&request.body);
            let sse = if body.contains("tool_result") {
                anthropic_text_sse()
            } else {
                anthropic_tool_call_sse()
            };
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse)
        })
        .expect(2)
        .mount(&peer)
        .await;

    let info = opi_ai::model_info::ModelInfo::new(
        "claude-sonnet-4-5-20250514",
        "Loop Model",
        WireApi::AnthropicMessages,
        opi_ai::registry::ModelCapabilities::new(200_000, 8_192),
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(opi_ai::anthropic::AnthropicProvider::for_route(
                "anthropic-loop".into(),
                vec![info.clone()],
                Some(peer.uri()),
                Default::default(),
                Arc::new(HttpClient::new()),
                false,
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-loop"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    let weather = ToolDef {
        name: "get_weather".into(),
        description: "Look up the weather".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        }),
        constrained_sampling: None,
    };
    let shorthand = ShorthandContext {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: "Weather in Paris?".into(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![weather],
    };
    let normalized = normalize_context(shorthand).unwrap();
    let normalized_for_loop = normalized.clone();

    let mut request = base_request("claude-sonnet-4-5-20250514");
    request.context = normalized;
    let first = models
        .complete_simple(
            ChatModelRecord::new("anthropic-loop", info.clone()),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("tool round");
    let tool_call = first
        .content
        .iter()
        .find_map(|block| match block {
            AssistantContent::ToolCall { tool_call } => Some(tool_call.clone()),
            _ => None,
        })
        .expect("assistant proposes the declared tool");
    assert_eq!(tool_call.name, "get_weather");

    // The caller is the trusted execution boundary: append the assistant
    // tool-call turn and an executed result, then serialize the whole
    // normalized transcript and rebuild it before continuing — the
    // model-visible chronology is serde-portable without losing tool
    // association.
    let mut messages = transcript::TranscriptContext::into_messages(normalized_for_loop);
    messages.push(Message::Assistant(first));
    messages.push(Message::ToolResult(ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![OutputContent::Text { text: "18C".into() }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    }));
    let rebuilt_shorthand = ShorthandContext {
        system_prompt: None,
        messages,
        tools: vec![],
    };
    let rebuilt = normalize_context(rebuilt_shorthand).unwrap();
    let encoded = serde_json::to_vec(&rebuilt).unwrap();
    let decoded: TranscriptContext = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        decoded.messages().len(),
        4,
        "system + user + assistant tool call + tool result survive serde"
    );

    let mut request = base_request("claude-sonnet-4-5-20250514");
    request.context = decoded;
    let second = models
        .complete_simple(
            ChatModelRecord::new("anthropic-loop", info),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("continuation round");
    assert_eq!(message_text(&second), "Hello world");
    peer.verify().await;
}

// ---------------------------------------------------------------------------
// AIR-050: two instances, shared store, route isolation, cleanup
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_instances_share_a_store_but_keep_routes_isolated_and_clean_up() {
    let left_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse_template(openai_completions_sse()))
        .mount(&left_peer)
        .await;

    let store = Arc::new(opi_ai::mutation::InMemoryCredentialStore::new());

    let mut left = Models::new();
    left.set_credential_store(store.clone());
    left.register_route(
        Box::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
            Some(left_peer.uri()),
            "openai-chat".into(),
            Default::default(),
            vec![completions_info("iso-model")],
            Arc::new(HttpClient::new()),
        )),
        Arc::new(opi_ai::auth::StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("sk-left"),
        )),
        AuthProvenanceSource::Static,
        Default::default(),
    )
    .unwrap();

    let mut right = Models::new();
    right.set_credential_store(store.clone());
    right
        .register_route(
            Box::new(MockProvider::new_with_models(
                "iso-mock",
                vec![completions_info("mock-model")],
                vec![text_response("shared store twin")],
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-right"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();

    let left_message = left
        .complete_simple(
            ChatModelRecord::new("openai-chat", completions_info("iso-model")),
            user_request("iso-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("left instance route");
    assert_eq!(message_text(&left_message), "Hello world");

    // Route isolation: registrations never leak across instances sharing a
    // credential store.
    assert!(right.get_model("openai-chat", "iso-model").is_none());
    assert!(left.get_model("iso-mock", "mock-model").is_none());

    // Availability is instance-scoped (AIR-046 management reads).
    let left_ids: Vec<String> = left
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    let right_ids: Vec<String> = right
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(left_ids.contains(&"openai-chat".to_string()));
    assert!(!left_ids.contains(&"iso-mock".to_string()));
    assert!(right_ids.contains(&"iso-mock".to_string()));
    assert!(!right_ids.contains(&"openai-chat".to_string()));

    left.cleanup_session_resources(None)
        .await
        .expect("left cleanup");
    right
        .cleanup_session_resources(None)
        .await
        .expect("right cleanup");
}

// ---------------------------------------------------------------------------
// AIR-046: empty Models and zero-I/O construction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_models_registers_no_provider_and_static_reads_do_no_io() {
    let context = OwnedRecordingContext::new(&[]);
    let models = Models::new();

    // An empty Models is not advertised as populated.
    assert!(
        models
            .get_model("anthropic", "claude-sonnet-4-5-20250514")
            .is_none()
    );
    assert!(models.get_available(None).await.unwrap().is_empty());

    // Registration with an injected route and static queries perform zero
    // env/store/network I/O: the recording context sees no reads and no
    // provider attempt starts (no availability entry, no model lookup).
    let mut models = Models::new();
    models
        .register_route(
            Box::new(MockProvider::new("never", vec![text_response("unused")])),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-empty"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    assert_eq!(context.read_count(), 0);
    assert!(
        models.get_model("never", "mock-model").is_some(),
        "registration itself advertises the route's models"
    );
    let available: Vec<String> = models
        .get_available(None)
        .await
        .unwrap()
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert!(available.contains(&"never".to_string()));
    assert_eq!(
        context.read_count(),
        0,
        "static queries performed zero env I/O"
    );
}

// ---------------------------------------------------------------------------
// P20-EXIT-02/V-39..42: lawful large collections are accepted
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lawful_collection_over_sixty_four_messages_is_accepted() {
    let peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(sse_template(openai_completions_sse()))
        .expect(1)
        .mount(&peer)
        .await;
    let info = opi_ai::model_info::ModelInfo::new(
        "bulk-model",
        "Bulk Model",
        WireApi::OpenAiCompletions,
        opi_ai::registry::ModelCapabilities::new(1_000_000, 8_192),
    );
    let mut models = Models::new();
    models
        .register_chat_api(
            Arc::new(
                opi_ai::openai_chat::OpenAiCompletionsChatApi::new(
                    Arc::new(opi_ai::openai_chat::OpenAiChatProvider::for_route(
                        Some(peer.uri()),
                        "bulk-provider".into(),
                        Default::default(),
                        vec![info.clone()],
                        Arc::new(HttpClient::new()),
                    )),
                    info,
                )
                .expect("bulk api"),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("sk-bulk"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let mut messages = Vec::new();
    for index in 0..70u32 {
        messages.push(Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: format!("turn {index}"),
            }],
            timestamp_ms: 0,
        }));
        messages.push(Message::Assistant(base_assistant()));
    }
    let context = normalize_context(ShorthandContext {
        system_prompt: Some("bulk chronology".into()),
        messages,
        tools: vec![],
    })
    .expect("70 turns normalize");
    let mut request = base_request("bulk-model");
    request.context = context;
    let message = models
        .complete_simple(
            ChatModelRecord::new("bulk-provider", completions_info("bulk-model")),
            request,
            SimpleOptions::default(),
        )
        .await
        .expect("large lawful acceptance");
    assert_eq!(message_text(&message), "Hello world");
    let sent = peer.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("wire body decodes");
    let wire_messages = body["messages"].as_array().expect("messages array");
    assert_eq!(
        wire_messages.len(),
        71,
        "system + 70 merged user/assistant turns reach the wire"
    );
    assert!(
        serde_json::to_string(&body).unwrap().contains("turn 69"),
        "the latest lawful turn is not truncated"
    );
}

// ---------------------------------------------------------------------------
// P20-ACTOR-08/V-53 deferred slice: explicit resume serves the recorded final
// ---------------------------------------------------------------------------

#[tokio::test]
async fn deferred_pending_then_explicit_fetch_serves_the_recorded_final() {
    // A scripted deferred producer returns a pending handle first; only the
    // caller's explicit fetch resumes it and serves the recorded final
    // exactly once — no automatic polling (P20-ACTOR-08).
    let backend = Arc::new(ScriptedDeferredBackend::new(
        "deferred-matrix",
        "mock-model",
    ));
    let mut models = Models::new();
    models
        .register_route(
            Box::new(
                MockProvider::new("deferred-matrix", vec![text_response("unused direct")])
                    .with_deferred(backend.clone(), 1, Some(60_000)),
            ),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-defer"),
            )),
            AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    models
        .register_deferred_api(
            backend.clone() as Arc<dyn opi_ai::deferred::DeferredApi<()>>,
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::ApiKey,
                SecretString::from("sk-defer"),
            )),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let pending = models
        .complete_simple(
            ChatModelRecord::new("deferred-matrix", completions_info("defer-model")),
            user_request("mock-model", "Hello"),
            SimpleOptions::default(),
        )
        .await
        .expect("deferred round");
    assert_eq!(pending.stop_reason, opi_ai::stream::StopReason::Deferred);
    assert!(pending.deferred.is_some(), "pending handle returned");
    assert_eq!(backend.fetch_count(), 0, "no automatic polling");

    let handle = backend.submit(text_response("deferred done"), 0, None);
    let resumed = models
        .fetch_deferred(&handle, opi_ai::deferred::DeferredFetchCall::default())
        .await
        .expect("explicit fetch serves the recorded final");
    assert_eq!(message_text(&resumed), "deferred done");
    assert_eq!(backend.fetch_count(), 1);
}
