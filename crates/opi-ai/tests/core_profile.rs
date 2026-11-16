//! Backend-neutral SDK behavior, also executed from the actual minimum package.
use futures_util::StreamExt;
use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{
    BoxAuthFuture, Credential, CredentialSource, CredentialStore, CredentialStoreError,
};
use opi_ai::message::{AssistantContent, Message, OutputContent, ToolResultMessage};
use opi_ai::models::Models;
use opi_ai::mutation::InMemoryCredentialStore;
use opi_ai::provider::{ProviderError, Request};
use opi_ai::stream::StopReason;
use opi_ai::test_support::{
    MockProvider, single_route_collection, text_response, tool_call_response,
};
use opi_ai::transcript::{Context, normalize_context};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn request(messages: Vec<Message>) -> Request {
    Request {
        model: "faux:mock-model".into(),
        context: normalize_context(Context {
            system_prompt: None,
            messages,
            tools: vec![],
        })
        .unwrap(),
        projection: Default::default(),
        transport: Default::default(),
        max_tokens: None,
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
        callbacks: Default::default(),
        telemetry_context: None,
    }
}

#[tokio::test]
async fn scripted_faux_streams_and_replays_a_tool_result() {
    let provider = MockProvider::new(
        "faux",
        vec![
            tool_call_response("call-1", "number", r#"{"n":1e64}"#),
            text_response("confirmed"),
        ],
    );
    let log = provider.call_log_handle();
    let models = single_route_collection(Box::new(provider));
    let prepared = models
        .prepare_call("faux:mock-model", request(vec![]))
        .await
        .unwrap();
    let mut stream = opi_ai::ChatStream::new(prepared.start_attempt().unwrap());
    let mut events = Vec::new();
    while let Some(event) = stream.next().await {
        events.push(event.unwrap());
    }
    assert!(matches!(
        events.first(),
        Some(opi_ai::stream::AssistantStreamEvent::Start { .. })
    ));
    let first = stream.result().await.unwrap();
    assert_eq!(first.stop_reason, StopReason::ToolUse);
    let AssistantContent::ToolCall { tool_call } = &first.content[0] else {
        panic!("expected tool call")
    };
    let value: serde_json::Value = serde_json::from_str(&tool_call.arguments).unwrap();
    assert_eq!(value["n"].as_f64().unwrap().to_bits(), 0x4d384f03e93ff9f5);
    let result = Message::ToolResult(ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![OutputContent::Text {
            text: serde_json::to_string(&value).unwrap(),
        }],
        details: None,
        is_error: false,
        not_executed: None,
        truncated: false,
        timestamp_ms: 0,
    });
    let messages = vec![Message::Assistant(first), result];
    let prepared = models
        .prepare_call("faux:mock-model", request(messages.clone()))
        .await
        .unwrap();
    let final_message = opi_ai::ChatStream::new(prepared.start_attempt().unwrap())
        .result()
        .await
        .unwrap();
    assert_eq!(
        opi_ai::message::assistant_content_text(&final_message.content),
        "confirmed"
    );
    let log = log.lock().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[1].context.messages(), messages);
    let policy = opi_ai::retry::smithy::SmithyRetryConfig::default().acquisition_policy();
    assert_eq!(policy.max_retries, 2);
}

struct Flow;
impl OAuthProvider for Flow {
    fn id(&self) -> &str {
        "custom"
    }
    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async move {
            presenter.notify_success(); // Must be suppressed until the store acknowledges.
            Ok(OAuthCredential {
                access: "synthetic-access".into(),
                refresh: "synthetic-refresh".into(),
                expires_at: None,
                base_url: None,
                account_id: None,
                client_id: None,
                enterprise_domain: None,
                available_model_ids: None,
                scopes: None,
            })
        })
    }
    fn refresh<'a>(
        &'a self,
        _: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { panic!("login must not refresh") })
    }
}

#[derive(Default)]
struct Presenter {
    success: AtomicUsize,
    failure: AtomicUsize,
}
impl LoginPresenter for Presenter {
    fn present_auth_url<'a>(&'a self, _: &'a str) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }
    fn present_device_code<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }
    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        Box::pin(async { panic!("no manual input") })
    }
    fn notify_success(&self) {
        self.success.fetch_add(1, Ordering::SeqCst);
    }
    fn notify_failure(&self, _: &str) {
        self.failure.fetch_add(1, Ordering::SeqCst);
    }
}

struct GatedStore {
    inner: InMemoryCredentialStore,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    reject: bool,
}
impl CredentialStore for GatedStore {
    fn read<'a>(
        &'a self,
        key: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>> {
        self.inner.read(key)
    }
    fn write<'a>(
        &'a self,
        key: &'a str,
        value: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            if self.reject {
                return Err(CredentialStoreError::Backend {
                    provider: key.into(),
                    reason: "synthetic rejected write".into(),
                });
            }
            self.inner.write(key, value).await
        })
    }
    fn delete<'a>(&'a self, key: &'a str) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>> {
        self.inner.delete(key)
    }
    fn probe<'a>(&'a self, key: &'a str) -> BoxAuthFuture<'a, CredentialSource> {
        self.inner.probe(key)
    }
}

async fn login_ack(reject: bool) {
    let store = Arc::new(GatedStore {
        inner: InMemoryCredentialStore::new(),
        entered: Default::default(),
        release: Default::default(),
        reject,
    });
    let presenter = Arc::new(Presenter::default());
    let mut models = Models::new();
    models.set_credential_store(store.clone());
    models.register_oauth_flow("custom", Arc::new(Flow));
    let task_presenter = presenter.clone();
    let login = tokio::spawn(async move { models.login("custom", task_presenter.as_ref()).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), store.entered.notified())
        .await
        .unwrap();
    assert_eq!(presenter.success.load(Ordering::SeqCst), 0);
    assert!(!login.is_finished());
    store.release.notify_one();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(5), login)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcome.is_ok(), !reject);
    assert_eq!(
        presenter.success.load(Ordering::SeqCst),
        usize::from(!reject)
    );
    assert_eq!(
        presenter.failure.load(Ordering::SeqCst),
        usize::from(reject)
    );
    assert_eq!(store.read("custom").await.unwrap().is_some(), !reject);
}

#[tokio::test]
async fn custom_login_waits_for_store_ack() {
    login_ack(false).await;
}

#[tokio::test]
async fn custom_login_never_announces_a_rejected_write() {
    login_ack(true).await;
}

#[cfg(feature = "telemetry-tracing")]
#[test]
fn tracing_carrier_is_available_independently_of_builtins() {
    let context = opi_ai::telemetry::TelemetryContext::from_span(tracing::Span::none());
    let _call = opi_ai::classifier::ClassifierCall {
        telemetry_context: Some(context),
        ..Default::default()
    };
}
