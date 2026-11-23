//! Shared Azure OpenAI Responses fixture builders and local peers.

use super::*;

pub(crate) fn azure_responses_record(id: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        WireApi::AzureOpenAiResponses,
        ModelCapabilities::new(200_000, 100_000)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

pub(crate) fn azure_responses_provider(endpoint: String) -> std::sync::Arc<AzureOpenAIProvider> {
    std::sync::Arc::new(
        AzureOpenAIProvider::for_records(
            Some(endpoint),
            vec![azure_responses_record("my-gpt4o")],
            None,
        )
        .unwrap(),
    )
}

pub(crate) fn azure_responses_request(
    model: &str,
    thinking: Option<ThinkingLevel>,
) -> opi_ai::provider::Request {
    let mut request = text_request();
    request.model = format!("azure:{model}");
    request.thinking = match thinking {
        Some(level) => opi_ai::provider::ThinkingConfig {
            enabled: true,
            budget_tokens: None,
            level,
        },
        None => Default::default(),
    };
    request
}

pub(crate) fn azure_responses_sse_text() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_az\",\"status\":\"in_progress\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_az\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"Hi\"}]}],\"usage\":{\"input_tokens\":7,\"output_tokens\":3}}}\n\n",
    )
    .to_owned()
}

pub(crate) fn azure_responses_sse_tool() -> String {
    concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp_az\",\"status\":\"in_progress\",\"model\":\"my-gpt4o\",\"output\":[]}}\n\n",
        "data: {\"type\":\"response.output_item.added\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"\"}}\n\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"output_index\":0,\"item_id\":\"fc_1\",\"call_id\":\"call_1\",\"delta\":\"{\\\"city\\\":\\\"London\\\"}\"}\n\n",
        "data: {\"type\":\"response.output_item.done\",\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"London\\\"}\"}}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp_az\",\"status\":\"completed\",\"model\":\"my-gpt4o\",\"output\":[{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"get_weather\",\"arguments\":\"{\\\"city\\\":\\\"London\\\"}\"}],\"usage\":{\"input_tokens\":20,\"output_tokens\":10}}}\n\n",
    )
    .to_owned()
}

pub(crate) fn azure_api_key_auth() -> opi_ai::auth::ResolvedAuth {
    opi_ai::auth::ResolvedAuth {
        env: Default::default(),
        scheme: opi_ai::auth::AuthScheme::ApiKey,
        secret: secrecy::SecretString::from("offline-azure-key"),
        base_url: None,
        account_id: None,
        provenance: opi_ai::AuthProvenance::default(),
    }
}

pub(crate) async fn mount_azure_responses_mock(server: &wiremock::MockServer, body: String) {
    use wiremock::matchers::{method, path, query_param};
    wiremock::Mock::given(method("POST"))
        .and(path("/openai/v1/responses"))
        .and(query_param("api-version", "v1"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_string(body)
                .insert_header("content-type", "text/event-stream"),
        )
        .expect(1)
        .mount(server)
        .await;
}

pub(crate) async fn sent_body(server: &wiremock::MockServer) -> serde_json::Value {
    serde_json::from_slice(&server.received_requests().await.unwrap()[0].body).unwrap()
}

pub(crate) async fn collect_azure_events(
    stream: &mut opi_ai::provider::EventStream,
) -> Vec<AssistantStreamEvent> {
    use futures_util::StreamExt;
    let mut events = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                let terminal = event.is_terminal();
                events.push(event);
                if terminal {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    events
}

pub(crate) async fn registered_azure_responses_models(
    server_uri: String,
    model: ModelInfo,
) -> (
    opi_ai::models::Models,
    opi_ai::api::contract::ChatApiHandle<AzureOpenAIOptions>,
) {
    let provider = azure_responses_provider(format!("{server_uri}/openai/v1"));
    let api = AzureOpenAIResponsesChatApi::new(provider, model).expect("valid azure record");
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::ApiKey,
                secrecy::SecretString::from("offline-azure-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .expect("register azure responses api");
    (models, handle)
}

pub(crate) async fn collect_azure_items(
    stream: &mut opi_ai::provider::EventStream,
) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
    use futures_util::StreamExt;
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        let terminal = match &item {
            Ok(event) => event.is_terminal(),
            Err(_) => true,
        };
        items.push(item);
        if terminal {
            break;
        }
    }
    items
}

pub(crate) fn azure_history_assistant(
    content: Vec<opi_ai::message::AssistantContent>,
    wire: Option<WireApi>,
    provider: &str,
) -> opi_ai::message::AssistantMessage {
    opi_ai::message::AssistantMessage {
        projection: None,
        content,
        api: opi_ai::ApiKind::OpenAi,
        provider: provider.into(),
        model: "my-gpt4o".into(),
        response_model: None,
        wire,
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    }
}

pub(crate) async fn azure_duration_on_peer(
    body_delay: std::time::Duration,
    setup_delay: std::time::Duration,
) -> (opi_ai::message::AssistantMessage, std::time::Duration) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut bytes = [0; 4096];
        loop {
            let read = socket.read(&mut bytes).await.unwrap();
            assert!(read > 0, "request terminated before its full body");
            request.extend_from_slice(&bytes[..read]);
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if request.len() >= end + 4 + length {
                    assert!(
                        headers.starts_with("post /openai/v1/responses?api-version=v1 http/1.1"),
                        "{headers}"
                    );
                    assert!(headers.contains("\r\napi-key: offline-azure-key"));
                    assert!(!headers.contains("\r\nauthorization:"));
                    let body: serde_json::Value =
                        serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                    assert_eq!(body["model"], "my-gpt4o");
                    break;
                }
            }
        }
        tokio::time::sleep(setup_delay).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        socket.flush().await.unwrap();
        tokio::time::sleep(body_delay).await;
        let body = azure_responses_sse_text().replacen(
            "data: {\"type\":\"response.completed\"",
            "data: {\"type\":\"response.output_text.delta\",\"output_index\":0,\"content_index\":0,\"delta\":\"Hi\"}\n\ndata: {\"type\":\"response.completed\"",
            1,
        );
        socket
            .write_all(format!("{:x}\r\n{}\r\n0\r\n\r\n", body.len(), body).as_bytes())
            .await
            .unwrap();
        socket.flush().await.unwrap();
    });

    struct DelayedAuth {
        delay: std::time::Duration,
        calls: Arc<AtomicUsize>,
    }
    impl opi_ai::auth::AuthResolver for DelayedAuth {
        fn resolve(
            &self,
        ) -> opi_ai::credential::BoxAuthFuture<'_, Result<opi_ai::ResolvedAuth, ProviderError>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                tokio::time::sleep(self.delay).await;
                Ok(azure_api_key_auth())
            })
        }
    }
    let provider = Arc::try_unwrap(azure_responses_provider(format!(
        "http://{address}/openai/v1"
    )))
    .unwrap_or_else(|_| panic!("fresh provider must be uniquely owned"));
    let model = provider
        .models()
        .iter()
        .find(|model| model.id == "my-gpt4o")
        .unwrap()
        .clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut models = opi_ai::Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(DelayedAuth {
                delay: setup_delay,
                calls: calls.clone(),
            }),
            opi_ai::AuthProvenanceSource::Static,
            Default::default(),
        )
        .unwrap();
    let mut request = azure_responses_request("my-gpt4o", None);
    let (terminal_tx, terminal_rx) = tokio::sync::oneshot::channel();
    let terminal_tx = std::sync::Mutex::new(Some(terminal_tx));
    request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_async_on_response(move |status, _| async move {
            assert_eq!(status, 200);
            tokio::time::sleep(setup_delay).await;
            Ok(())
        })
        .with_on_raw(Arc::new(move |event| {
            if event["type"] == "response.completed" {
                terminal_tx
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .send(())
                    .unwrap();
            }
            Ok(())
        }));
    let started = std::time::Instant::now();
    let call = models.stream_simple(
        opi_ai::ChatModelRecord::new("azure", model),
        request,
        Default::default(),
    );
    tokio::time::timeout(std::time::Duration::from_secs(5), terminal_rx)
        .await
        .expect("unconsumed execution receives the raw terminal")
        .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "execution authenticates without consumption"
    );
    // The source proceeds past the terminal independently while the consumer
    // waits. This interval must not inflate the recorded body duration.
    tokio::time::sleep(setup_delay).await;
    let message = call
        .result()
        .await
        .expect("controlled real adapter succeeds");
    let elapsed = started.elapsed();
    peer.await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(message.stop_reason, opi_ai::stream::StopReason::Stop);
    assert_eq!(message.usage.input_tokens, 7);
    assert_eq!(message.usage.output_tokens, 3);
    (message, elapsed)
}
