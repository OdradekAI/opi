use std::sync::Arc;
use std::time::Duration;

use opi_ai::auth::{OAuthCredential, OAuthProvider};
use opi_ai::models::Models;
use secrecy::ExposeSecret;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub fn register_oauth_assembly_route(
    models: &mut Models,
    flow: Arc<dyn OAuthProvider>,
    wire: opi_ai::WireApi,
    base_url: String,
    client: Arc<opi_ai::http::HttpClient>,
) {
    use opi_ai::provider::Provider;
    let id = flow.id().to_owned();
    let catalog = vec![opi_ai::ModelInfo::new(
        "auth-proof",
        "Auth proof",
        wire,
        opi_ai::ModelCapabilities::new(16_384, 1024).with_streaming(true),
    )];
    let provider: Box<dyn Provider> = match wire {
        opi_ai::WireApi::AnthropicMessages => {
            Box::new(opi_ai::providers::anthropic::AnthropicProvider::for_route(
                id,
                catalog,
                Some(base_url),
                Default::default(),
                client,
                true,
            ))
        }
        opi_ai::WireApi::OpenAiCompletions => {
            let provider = opi_ai::providers::openai::OpenAiChatProvider::for_route(
                Some(base_url),
                id.clone(),
                Default::default(),
                catalog,
                client,
            );
            Box::new(if id == "github-copilot" {
                provider.with_copilot_initiator()
            } else {
                provider
            })
        }
        opi_ai::WireApi::OpenAiResponses => Box::new(
            opi_ai::providers::openai_responses::OpenAiResponsesProvider::for_route(
                Some(base_url),
                id,
                Default::default(),
                catalog,
                client,
            ),
        ),
        opi_ai::WireApi::OpenAiCodexResponses => Box::new(
            opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider::new(
                Some(base_url),
                catalog,
                client,
            ),
        ),
        _ => panic!("unexpected authentication proof wire"),
    };
    models
        .register_oauth_route(provider, flow, Default::default())
        .expect("OAuth route registration");
}

pub fn oauth_assembly_request(id: &str) -> opi_ai::provider::Request {
    opi_ai::provider::Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("{id}:auth-proof"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: vec![],
            tools: vec![],
        })
        .expect("context"),
        max_tokens: Some(32),
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: Some(Duration::from_secs(5)),
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub fn oauth_assembly_sse(wire: opi_ai::WireApi) -> &'static str {
    match wire {
        opi_ai::WireApi::AnthropicMessages => concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"proof\",\"role\":\"assistant\",\"model\":\"auth-proof\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":0}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ),
        opi_ai::WireApi::OpenAiCompletions => concat!(
            "data: {\"id\":\"proof\",\"model\":\"auth-proof\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"proof\",\"model\":\"auth-proof\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n",
        ),
        _ => {
            "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"proof\",\"model\":\"auth-proof\",\"status\":\"completed\",\"output\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n"
        }
    }
}

pub async fn assert_oauth_identity(
    models: &Models,
    id: &str,
    credential: &OAuthCredential,
) -> opi_ai::auth::ResolvedAuth {
    let resolved = models
        .get_auth(id, Default::default())
        .await
        .expect("resolve selected OAuth")
        .expect("selected identity");
    assert!(matches!(&resolved.scheme, opi_ai::auth::AuthScheme::Bearer));
    assert_eq!(
        resolved.secret.expose_secret(),
        credential.access.expose_secret()
    );
    assert_eq!(resolved.account_id, credential.account_id);
    assert!(
        matches!(&resolved.provenance.source, opi_ai::auth::AuthProvenanceSource::OAuth { kind } if kind == id)
    );
    resolved
}

pub async fn assert_oauth_adapter_request(
    models: &Models,
    id: &str,
    credential: &OAuthCredential,
    server: &MockServer,
    wire: opi_ai::WireApi,
) {
    let _ = assert_oauth_identity(models, id, credential).await;
    let endpoint = match wire {
        opi_ai::WireApi::AnthropicMessages => "/v1/messages",
        opi_ai::WireApi::OpenAiCompletions => "/v1/chat/completions",
        opi_ai::WireApi::OpenAiCodexResponses => "/codex/responses",
        _ => "/v1/responses",
    };
    Mock::given(method("POST"))
        .and(path(endpoint))
        .and(wiremock::matchers::header(
            "authorization",
            format!("Bearer {}", credential.access.expose_secret()),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(oauth_assembly_sse(wire)),
        )
        .expect(1)
        .mount(server)
        .await;
    let spec = format!("{id}:auth-proof");
    let call = models
        .prepare_call(&spec, oauth_assembly_request(id))
        .await
        .expect("prepare selected identity");
    opi_ai::models::drain_to_completion(call.start_attempt().expect("adapter attempt"))
        .await
        .expect("local adapter completed");
    server.verify().await;
    let requests = server.received_requests().await.expect("requests");
    let request = requests
        .iter()
        .find(|request| request.url.path() == endpoint)
        .expect("actual adapter request");
    assert!(!request.headers.contains_key("x-api-key"));
    if id == "anthropic" {
        assert_eq!(
            request.headers["anthropic-beta"],
            "claude-code-20250219,oauth-2025-04-20"
        );
    }
    if id == "openai-codex" {
        assert_eq!(
            request.headers["chatgpt-account-id"],
            credential.account_id.as_deref().unwrap()
        );
    }
}
