//! Lifecycle contract tests for OpenAI-Chat-compatible profiles (openai_chat,
//! mistral, openrouter) -- all use `OpenAiChatProvider` over the Chat
//! Completions SSE dialect. Merged from the three per-profile lifecycle
//! binaries to cut integration-binary count (Candidate 2); each profile keeps
//! its own helpers/tests in a submodule.

mod openai_chat_profile {

    use futures_util::StreamExt;
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_ai::openai_chat::OpenAiChatProvider;
    use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
    use opi_ai::stream::AssistantStreamEvent;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_request(cancel: CancellationToken) -> Request {
        Request {
            model: "openai:gpt-4o".into(),
            system: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            thinking: ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel,
        }
    }

    fn text_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
     data: [DONE]\n\n"
    }

    fn incomplete_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Partial\"},\"finish_reason\":null}]}\n\n"
    }

    // ---------------------------------------------------------------------------
    // Happy path: SSE streaming
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let is_terminal = event.is_terminal();
                    events.push(event);
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
            "should have Start event"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
            "should have TextDelta event"
        );

        let done = events
            .iter()
            .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
            .expect("should have Done event");
        if let AssistantStreamEvent::Done { reason, message } = done {
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            let text: String = message
                .content
                .iter()
                .filter_map(|c| match c {
                    opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, "Hello");
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 401 -> AuthFailed
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string(r#"{"error":{"message":"invalid api key"}}"#),
            )
            .mount(&server)
            .await;

        let provider = OpenAiChatProvider::new("bad-key".into(), Some(server.uri()));
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::AuthFailed(msg)) => {
                assert!(
                    msg.contains("authentication failed"),
                    "should mention auth failure: {msg}"
                );
            }
            other => panic!("expected AuthFailed, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 429 -> RateLimited
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"{"error":{"message":"too many requests"}}"#)
                    .insert_header("retry-after", "5"),
            )
            .mount(&server)
            .await;

        let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::RateLimited { retry_after_ms }) => {
                assert!(retry_after_ms.is_some(), "should parse retry-after header");
                assert_eq!(retry_after_ms.unwrap(), 5000);
            }
            other => panic!("expected RateLimited, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // No terminal event -> StreamError
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_no_terminal_event() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(incomplete_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut saw_stream_error = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    if event.is_terminal() {
                        break;
                    }
                }
                Err(ProviderError::StreamError(msg)) if msg.contains("terminal event") => {
                    saw_stream_error = true;
                    break;
                }
                Err(_) => break,
            }
        }
        assert!(
            saw_stream_error,
            "should produce StreamError about missing terminal event"
        );
    }

    #[tokio::test]
    async fn stream_custom_chat_completions_path() {
        use opi_ai::openai_chat::CompatConfig;

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/paas/v4/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        // Real signature is (api_key, base_url, provider_id, compat, extra_headers, models).
        let provider = OpenAiChatProvider::new_for_profile(
            "test-key".into(),
            server.uri(),
            "zai".into(),
            CompatConfig {
                chat_completions_path: "/api/paas/v4/chat/completions".into(),
                ..CompatConfig::default()
            },
            Vec::new(),
            Vec::new(),
        );

        let mut stream = provider.stream(make_request(CancellationToken::new()));
        let first = stream
            .next()
            .await
            .expect("event produced")
            .expect("stream ok");
        assert!(
            matches!(first, AssistantStreamEvent::Start { .. }),
            "expected Start event, got {first:?}"
        );
    }

    #[tokio::test]
    async fn stream_default_chat_completions_path_stays_v1() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = OpenAiChatProvider::new("test-key".into(), Some(server.uri()));
        let mut stream = provider.stream(make_request(CancellationToken::new()));
        let first = stream
            .next()
            .await
            .expect("event produced")
            .expect("stream ok");
        assert!(
            matches!(first, AssistantStreamEvent::Start { .. }),
            "expected Start event, got {first:?}"
        );
    }
}

mod mistral_profile {

    use futures_util::StreamExt;
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_ai::openai_chat::{CompatConfig, OpenAiChatProvider};
    use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
    use opi_ai::stream::AssistantStreamEvent;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_request(cancel: CancellationToken) -> Request {
        Request {
            model: "mistral:mistral-large-latest".into(),
            system: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            thinking: ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel,
        }
    }

    fn make_provider(server_uri: String) -> OpenAiChatProvider {
        let compat = CompatConfig::default();
        OpenAiChatProvider::new_for_profile(
            "test-key".into(),
            server_uri,
            "mistral".into(),
            compat,
            vec![],
            vec![],
        )
    }

    fn text_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"mistral-large-latest\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"mistral-large-latest\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"mistral-large-latest\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
     data: [DONE]\n\n"
    }

    fn incomplete_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"mistral-large-latest\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"mistral-large-latest\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Partial\"},\"finish_reason\":null}]}\n\n"
    }

    // ---------------------------------------------------------------------------
    // Happy path: SSE streaming
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let is_terminal = event.is_terminal();
                    events.push(event);
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
            "should have Start event"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
            "should have TextDelta event"
        );

        let done = events
            .iter()
            .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
            .expect("should have Done event");
        if let AssistantStreamEvent::Done { reason, message } = done {
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            let text: String = message
                .content
                .iter()
                .filter_map(|c| match c {
                    opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, "Hello");
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 401 -> AuthFailed
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string(r#"{"error":{"message":"invalid api key"}}"#),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::AuthFailed(msg)) => {
                assert!(
                    msg.contains("authentication failed"),
                    "should mention auth failure: {msg}"
                );
            }
            other => panic!("expected AuthFailed, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 429 -> RateLimited
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"{"error":{"message":"too many requests"}}"#)
                    .insert_header("retry-after", "5"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::RateLimited { retry_after_ms }) => {
                assert!(retry_after_ms.is_some(), "should parse retry-after header");
                assert_eq!(retry_after_ms.unwrap(), 5000);
            }
            other => panic!("expected RateLimited, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // No terminal event -> StreamError
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_no_terminal_event() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(incomplete_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut saw_stream_error = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    if event.is_terminal() {
                        break;
                    }
                }
                Err(ProviderError::StreamError(msg)) if msg.contains("terminal event") => {
                    saw_stream_error = true;
                    break;
                }
                Err(_) => break,
            }
        }
        assert!(
            saw_stream_error,
            "should produce StreamError about missing terminal event"
        );
    }
}

mod openrouter_profile {

    use futures_util::StreamExt;
    use opi_ai::message::{InputContent, Message, UserMessage};
    use opi_ai::openai_chat::{CompatConfig, OpenAiChatProvider};
    use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
    use opi_ai::stream::AssistantStreamEvent;
    use tokio_util::sync::CancellationToken;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_request(cancel: CancellationToken) -> Request {
        Request {
            model: "openrouter:gpt-4o".into(),
            system: None,
            messages: vec![Message::User(UserMessage {
                content: vec![InputContent::Text {
                    text: "hello".into(),
                }],
                timestamp_ms: 0,
            })],
            tools: vec![],
            max_tokens: None,
            temperature: None,
            thinking: ThinkingConfig::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel,
        }
    }

    fn make_provider(server_uri: String) -> OpenAiChatProvider {
        let compat = CompatConfig::default();
        OpenAiChatProvider::new_for_profile(
            "test-key".into(),
            server_uri,
            "openrouter".into(),
            compat,
            vec![
                ("HTTP-Referer".into(), "https://opi.dev".into()),
                ("X-Title".into(), "opi".into()),
            ],
            vec![],
        )
    }

    fn text_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
     data: [DONE]\n\n"
    }

    fn incomplete_sse_fixture() -> &'static str {
        "data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n\
     data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Partial\"},\"finish_reason\":null}]}\n\n"
    }

    // ---------------------------------------------------------------------------
    // Happy path: SSE streaming
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let is_terminal = event.is_terminal();
                    events.push(event);
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => panic!("unexpected error: {e}"),
            }
        }

        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::Start { .. })),
            "should have Start event"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::TextDelta { .. })),
            "should have TextDelta event"
        );

        let done = events
            .iter()
            .find(|e| matches!(e, AssistantStreamEvent::Done { .. }))
            .expect("should have Done event");
        if let AssistantStreamEvent::Done { reason, message } = done {
            assert_eq!(*reason, opi_ai::stream::StopReason::Stop);
            let text: String = message
                .content
                .iter()
                .filter_map(|c| match c {
                    opi_ai::message::AssistantContent::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(text, "Hello");
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 401 -> AuthFailed
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string(r#"{"error":{"message":"invalid api key"}}"#),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::AuthFailed(msg)) => {
                assert!(
                    msg.contains("authentication failed"),
                    "should mention auth failure: {msg}"
                );
            }
            other => panic!("expected AuthFailed, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // HTTP error mapping: 429 -> RateLimited
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"{"error":{"message":"too many requests"}}"#)
                    .insert_header("retry-after", "5"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let result = stream.next().await.expect("should have event");
        match result {
            Err(ProviderError::RateLimited { retry_after_ms }) => {
                assert!(retry_after_ms.is_some(), "should parse retry-after header");
                assert_eq!(retry_after_ms.unwrap(), 5000);
            }
            other => panic!("expected RateLimited, got: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // No terminal event -> StreamError
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_no_terminal_event() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(incomplete_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        let mut saw_stream_error = false;
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    if event.is_terminal() {
                        break;
                    }
                }
                Err(ProviderError::StreamError(msg)) if msg.contains("terminal event") => {
                    saw_stream_error = true;
                    break;
                }
                Err(_) => break,
            }
        }
        assert!(
            saw_stream_error,
            "should produce StreamError about missing terminal event"
        );
    }

    // ---------------------------------------------------------------------------
    // Extra headers: HTTP-Referer and X-Title are sent with the request
    // ---------------------------------------------------------------------------

    #[tokio::test]
    async fn stream_sends_extra_headers() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("HTTP-Referer", "https://opi.dev"))
            .and(header("X-Title", "opi"))
            .and(header("authorization", "Bearer test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(text_sse_fixture())
                    .insert_header("content-type", "text/event-stream"),
            )
            .mount(&server)
            .await;

        let provider = make_provider(server.uri());
        let mut stream = provider.stream(make_request(CancellationToken::new()));

        // Consume the stream -- the mock will only match if both extra headers are present.
        let mut events = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => {
                    let is_terminal = event.is_terminal();
                    events.push(event);
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => panic!("unexpected error (extra headers likely not sent): {e}"),
            }
        }

        // If we get here without a timeout or mock mismatch, the headers were sent.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, AssistantStreamEvent::Done { .. })),
            "should have Done event, confirming extra headers were accepted by the mock"
        );
    }
}
