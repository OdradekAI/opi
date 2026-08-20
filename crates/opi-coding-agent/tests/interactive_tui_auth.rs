use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthLoginMethod};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::message::Message;
use opi_ai::provider::{ProviderError, ProviderErrorSummary};
use opi_ai::stream::AssistantStreamEvent;
use opi_ai::test_support::{self, MockProvider, MockResponse};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::credential_store::{
    FakeKeyringBackend, KEYCHAIN_SERVICE, KeychainCredentialStore,
};
use opi_coding_agent::harness::CodingHarness;
use opi_coding_agent::interactive::{
    InteractiveTuiTestAuthServices, InteractiveTuiTestCapture, InteractiveTuiTestTerminalFailure,
    install_interactive_tui_test_driver, install_interactive_tui_test_driver_with_auth,
    interactive_tui_presenter_construction_count,
    reset_interactive_tui_presenter_construction_count, run_interactive_tui,
};
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::rpc::RpcRunner;
use opi_coding_agent::runner::NonInteractiveRunner;
use opi_tui::Keybindings;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(dead_code)]
#[path = "common/phase14_auth_runtime.rs"]
mod phase14_auth_runtime;
use phase14_auth_runtime::{
    run_json_credential_capture, run_rpc_stdio_capture, run_text_credential_capture,
};

static TEST_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

#[derive(Clone)]
struct TestPresenter {
    method: Option<OAuthLoginMethod>,
    manual_code: Option<String>,
    fail_present: bool,
    evidence: Arc<PresenterEvidence>,
}

#[derive(Default)]
struct PresenterEvidence {
    selections: AtomicUsize,
    auth_urls: AtomicUsize,
    manual_codes: AtomicUsize,
    successes: AtomicUsize,
    failures: AtomicUsize,
}

impl TestPresenter {
    fn browser(code: &str) -> Self {
        Self {
            method: Some(OAuthLoginMethod::Browser),
            manual_code: Some(code.to_owned()),
            fail_present: false,
            evidence: Arc::new(PresenterEvidence::default()),
        }
    }

    fn cancelled() -> Self {
        Self {
            method: None,
            manual_code: None,
            fail_present: false,
            evidence: Arc::new(PresenterEvidence::default()),
        }
    }

    fn failing_presenter() -> Self {
        Self {
            fail_present: true,
            ..Self::browser("unused")
        }
    }

    fn evidence(&self) -> Arc<PresenterEvidence> {
        self.evidence.clone()
    }
}

impl LoginPresenter for TestPresenter {
    fn select_login_method<'a>(
        &'a self,
        provider_id: &'a str,
        _methods: &'a [OAuthLoginMethod],
        _default: OAuthLoginMethod,
    ) -> BoxAuthFuture<'a, Result<OAuthLoginMethod, ProviderError>> {
        self.evidence.selections.fetch_add(1, Ordering::SeqCst);
        let method = self.method;
        let provider_id = provider_id.to_owned();
        Box::pin(async move { method.ok_or(ProviderError::LoginCancelled { provider_id }) })
    }

    fn present_auth_url<'a>(
        &'a self,
        _url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        self.evidence.auth_urls.fetch_add(1, Ordering::SeqCst);
        let fail = self.fail_present;
        Box::pin(async move {
            if fail {
                Err(ProviderError::Config(ProviderErrorSummary::from_untrusted(
                    "presenter failed",
                )))
            } else {
                Ok(())
            }
        })
    }

    fn present_device_code<'a>(
        &'a self,
        _user_code: &'a str,
        _verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        Box::pin(async { Ok(()) })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        self.evidence.manual_codes.fetch_add(1, Ordering::SeqCst);
        let code = self.manual_code.clone();
        Box::pin(async move { code.ok_or(ProviderError::Timeout) })
    }

    fn notify_success(&self) {
        self.evidence.successes.fetch_add(1, Ordering::SeqCst);
    }

    fn notify_failure(&self, _reason: &str) {
        self.evidence.failures.fetch_add(1, Ordering::SeqCst);
    }
}

fn test_store(root: &std::path::Path) -> KeychainCredentialStore {
    KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        root.to_path_buf(),
        Duration::from_millis(100),
    )
}

fn auth_services(presenter: TestPresenter, server: &MockServer) -> InteractiveTuiTestAuthServices {
    InteractiveTuiTestAuthServices::new(
        Arc::new(presenter),
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
        server.uri(),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
}

async fn mount_anthropic_token(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path("/anthropic/token"))
        .respond_with(
            ResponseTemplate::new(status).set_body_json(if status == 200 {
                json!({
                    "access_token": "anthropic-access",
                    "refresh_token": "anthropic-refresh",
                    "expires_in": 3600
                })
            } else {
                json!({"error":"invalid_grant"})
            }),
        )
        .mount(server)
        .await;
}

fn codex_jwt(account_id: &str) -> String {
    use base64::Engine;
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": account_id,
            }
        }))
        .unwrap(),
    );
    format!("{header}.{payload}.signature")
}

async fn mount_codex_token(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/codex/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": codex_jwt("different-provider"),
            "refresh_token": "codex-refresh",
            "expires_in": 3600
        })))
        .mount(server)
        .await;
}

struct OuterResult {
    result: Result<(), Box<dyn std::error::Error>>,
    capture: InteractiveTuiTestCapture,
    calls: Vec<Vec<Message>>,
    store_writes: usize,
    stored_anthropic: bool,
    stored_codex: bool,
}

async fn run_outer(
    responses: Vec<MockResponse>,
    inputs: &[&str],
    auth: InteractiveTuiTestAuthServices,
    failing_store: bool,
) -> OuterResult {
    let provider = MockProvider::new_with_errors("mock", responses);
    let call_log = provider.call_log_handle();
    let workspace = tempfile::tempdir().unwrap();
    let backend = FakeKeyringBackend::new();
    let observed_backend = backend.clone();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .tool_selection(ToolSelection::Disabled)
    .build();
    harness.credential_store = Some(Arc::new(KeychainCredentialStore::with_lock_timeout(
        Box::new(if failing_store {
            backend.with_unavailable()
        } else {
            backend
        }),
        workspace.path().to_path_buf(),
        Duration::from_millis(100),
    )));

    let driver = install_interactive_tui_test_driver_with_auth(inputs.iter().copied(), auth)
        .expect("install one scripted outer-TUI adapter");
    let result = run_interactive_tui(
        harness,
        "mock:mock-model".into(),
        "default",
        Keybindings::default(),
    )
    .await;
    let capture = driver.capture();
    let calls = call_log
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .map(|request| request.messages.clone())
        .collect();
    OuterResult {
        result,
        capture,
        calls,
        store_writes: observed_backend.set_windows().len(),
        stored_anthropic: observed_backend
            .raw_entry(KEYCHAIN_SERVICE, "anthropic")
            .is_some(),
        stored_codex: observed_backend
            .raw_entry(KEYCHAIN_SERVICE, "openai-codex")
            .is_some(),
    }
}

fn assert_one_user_message(calls: &[Vec<Message>]) {
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0]
            .iter()
            .filter(|message| matches!(message, Message::User(_)))
            .count(),
        1
    );
}

fn assert_negative(result: &OuterResult) {
    assert_eq!(result.capture.user_messages, 1);
    assert_eq!(result.capture.provider_calls, 1);
    assert_eq!(result.capture.retries, 0);
    assert_one_user_message(&result.calls);
}

fn assert_last_system_message(result: &OuterResult, expected: &str) {
    assert_eq!(
        result.capture.system_messages.last().map(String::as_str),
        Some(expected)
    );
}

fn presenter_counts(evidence: &PresenterEvidence) -> (usize, usize, usize, usize, usize) {
    (
        evidence.selections.load(Ordering::SeqCst),
        evidence.auth_urls.load(Ordering::SeqCst),
        evidence.manual_codes.load(Ordering::SeqCst),
        evidence.successes.load(Ordering::SeqCst),
        evidence.failures.load(Ordering::SeqCst),
    )
}

fn credential_needed_then_success() -> Vec<MockResponse> {
    vec![
        MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        }),
        MockResponse::Events(test_support::text_response("recovered")),
    ]
}

fn tracked_credential_runner(
    workspace: &std::path::Path,
) -> (
    NonInteractiveRunner,
    Arc<Mutex<Vec<opi_ai::provider::Request>>>,
) {
    let provider = MockProvider::new_with_errors(
        "anthropic",
        vec![MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        })],
    );
    let calls = provider.call_log_handle();
    (
        NonInteractiveRunner::new(
            Box::new(provider),
            // Phase 17.5: prepare_call strictly resolves the spec against the
            // mock catalog (only "mock-model"); the CredentialNeeded provider_id
            // is driven by the injected mock error, not the model spec.
            "anthropic:mock-model".into(),
            OpiConfig::default(),
            workspace.to_path_buf(),
            false,
            None,
            Vec::new(),
            opi_coding_agent::project_trust::TrustDecision::Trusted,
        ),
        calls,
    )
}

fn assert_one_noninteractive_user_message(calls: &Arc<Mutex<Vec<opi_ai::provider::Request>>>) {
    let calls = calls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(calls.len(), 1, "credential failure must not retry");
    assert_eq!(
        calls[0]
            .messages
            .iter()
            .filter(|message| matches!(message, Message::User(_)))
            .count(),
        1
    );
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_same_provider_login_retries_pending_turn_once() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("auth-code");
    let evidence = presenter.evidence();

    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("same-provider login recovers the turn");
    assert_eq!(result.capture.user_messages, 1);
    assert_eq!(result.capture.provider_calls, 2);
    assert_eq!(result.capture.retries, 1);
    assert_eq!(result.capture.presenter_constructions, 1);
    assert_last_system_message(&result, "[/login: anthropic succeeded]");
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 1, 0));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    assert_eq!(result.store_writes, 1);
    assert!(result.stored_anthropic);
    assert_eq!(result.calls.len(), 2);
    for request in &result.calls {
        assert_eq!(
            request
                .iter()
                .filter(|message| matches!(message, Message::User(_)))
                .count(),
            1
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_different_provider_login_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_codex_token(&server).await;
    let presenter = TestPresenter::browser("codex-code");
    let evidence = presenter.evidence();

    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login openai-codex", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("different-provider login still succeeds");
    assert_negative(&result);
    assert_last_system_message(&result, "[/login: openai-codex succeeded]");
    assert_eq!(presenter_counts(&evidence), (1, 1, 1, 1, 0));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    assert_eq!(result.store_writes, 1);
    assert!(result.stored_codex);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/codex/token");
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_login_selection_cancel_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    let presenter = TestPresenter::cancelled();
    let evidence = presenter.evidence();
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login openai-codex", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("selection cancellation returns to the TUI");
    assert_negative(&result);
    assert_last_system_message(
        &result,
        "[authentication command failed: authentication cancelled for provider 'openai-codex']",
    );
    assert_eq!(presenter_counts(&evidence), (1, 0, 0, 0, 1));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(result.store_writes, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_presenter_failure_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    let presenter = TestPresenter::failing_presenter();
    let evidence = presenter.evidence();
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("presenter failure returns to the TUI");
    assert_negative(&result);
    assert_last_system_message(
        &result,
        "[authentication command failed: authentication configuration failed]",
    );
    assert_eq!(presenter_counts(&evidence), (0, 1, 0, 0, 0));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(result.store_writes, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_oauth_failure_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 500).await;
    let presenter = TestPresenter::browser("bad-code");
    let evidence = presenter.evidence();
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("OAuth failure returns to the TUI");
    assert_negative(&result);
    assert_last_system_message(
        &result,
        "[authentication command failed: authentication failed]",
    );
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 0, 1));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/anthropic/token");
    assert_eq!(result.store_writes, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_store_failure_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("store-code");
    let evidence = presenter.evidence();
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        true,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("store failure returns to the TUI");
    assert_negative(&result);
    assert_last_system_message(
        &result,
        "[authentication command failed: authentication configuration failed]",
    );
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 0, 1));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/anthropic/token");
    assert_eq!(result.store_writes, 0);
    assert!(!result.stored_anthropic);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_terminal_suspension_failure_stops_before_oauth_or_store_work() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    let presenter = TestPresenter::browser("must-not-be-used");
    let evidence = presenter.evidence();
    let auth = auth_services(presenter, &server)
        .with_terminal_failure(InteractiveTuiTestTerminalFailure::Suspend);
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth,
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("terminal suspension failure returns to the TUI");
    assert_negative(&result);
    assert_last_system_message(
        &result,
        "[authentication command failed: terminal suspension failed]",
    );
    assert_eq!(presenter_counts(&evidence), (0, 0, 0, 0, 0));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(result.store_writes, 0);
    assert!(!result.stored_anthropic);
    assert!(!result.stored_codex);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_terminal_restore_failure_does_not_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("terminal-code");
    let evidence = presenter.evidence();
    let auth = auth_services(presenter, &server)
        .with_terminal_failure(InteractiveTuiTestTerminalFailure::Resume);
    let result = run_outer(
        credential_needed_then_success(),
        &["normal prompt", "/login anthropic", "exit"],
        auth,
        false,
    )
    .await;

    assert_eq!(
        result.result.as_ref().unwrap_err().to_string(),
        "terminal restore failed"
    );
    assert_negative(&result);
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 1, 0));
    assert_eq!(result.capture.terminal_transitions, ["suspend", "resume"]);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url.path(), "/anthropic/token");
    assert_eq!(result.store_writes, 1);
    assert!(result.stored_anthropic);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_second_normal_prompt_invalidates_pending_credential_turn() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("second-prompt-code");
    let evidence = presenter.evidence();
    let result = run_outer(
        vec![
            MockResponse::Error(ProviderError::CredentialNeeded {
                provider_id: "anthropic".into(),
            }),
            MockResponse::Events(test_support::text_response("second prompt succeeded")),
        ],
        &["first prompt", "second prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("second prompt and later explicit login complete");
    assert_eq!(result.capture.user_messages, 2);
    assert_eq!(result.capture.provider_calls, 2);
    assert_eq!(result.capture.retries, 0);
    assert_eq!(result.calls.len(), 2);
    assert_eq!(
        result.calls[0]
            .iter()
            .filter(|message| matches!(message, Message::User(_)))
            .count(),
        1
    );
    assert_eq!(
        result.calls[1]
            .iter()
            .filter(|message| matches!(message, Message::User(_)))
            .count(),
        // C5: the failed "first prompt" turn is rewound before the second
        // prompt is submitted, so the second provider call carries only the
        // new prompt (not the abandoned credential-needed user message).
        1
    );
    assert_last_system_message(&result, "[/login: anthropic succeeded]");
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 1, 0));
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_retry_credential_needed_does_not_rearm_or_retry_twice() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("retry-still-needs-credential");
    let evidence = presenter.evidence();
    let result = run_outer(
        vec![
            MockResponse::Error(ProviderError::CredentialNeeded {
                provider_id: "anthropic".into(),
            }),
            MockResponse::Error(ProviderError::CredentialNeeded {
                provider_id: "anthropic".into(),
            }),
        ],
        &[
            "normal prompt",
            "/login anthropic",
            "/login anthropic",
            "exit",
        ],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("second explicit login does not re-run consumed turn");
    assert_eq!(result.capture.user_messages, 1);
    assert_eq!(result.capture.provider_calls, 2);
    assert_eq!(result.capture.retries, 1);
    assert_eq!(result.calls.len(), 2);
    for request in &result.calls {
        assert_eq!(
            request
                .iter()
                .filter(|message| matches!(message, Message::User(_)))
                .count(),
            1
        );
    }
    assert_eq!(
        result
            .capture
            .system_messages
            .iter()
            .filter(|message| message.as_str() == "[/login: anthropic succeeded]")
            .count(),
        2
    );
    assert_eq!(
        result
            .capture
            .system_messages
            .iter()
            .filter(|message| message.starts_with("[credential needed"))
            .count(),
        1,
        "the retry failure must not re-arm credential remediation"
    );
    assert_eq!(presenter_counts(&evidence), (0, 2, 2, 2, 0));
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_non_text_output_before_credential_needed_prevents_retry() {
    let _lock = test_lock().await;
    let server = MockServer::start().await;
    mount_anthropic_token(&server, 200).await;
    let presenter = TestPresenter::browser("post-output-code");
    let evidence = presenter.evidence();
    let result = run_outer(
        vec![MockResponse::EventsThenError(
            vec![
                AssistantStreamEvent::Start {
                    partial: test_support::base_assistant(),
                },
                AssistantStreamEvent::ThinkingDelta {
                    content_index: 0,
                    delta: "non-text output".into(),
                    partial: test_support::base_assistant(),
                },
            ],
            ProviderError::CredentialNeeded {
                provider_id: "anthropic".into(),
            },
        )],
        &["normal prompt", "/login anthropic", "exit"],
        auth_services(presenter, &server),
        false,
    )
    .await;

    result
        .result
        .as_ref()
        .expect("post-output failure and explicit login return to the TUI");
    assert_eq!(result.capture.user_messages, 1);
    assert_eq!(result.capture.provider_calls, 1);
    assert_eq!(result.capture.retries, 0);
    assert_one_user_message(&result.calls);
    assert!(
        result
            .capture
            .system_messages
            .iter()
            .all(|message| !message.starts_with("[credential needed"))
    );
    assert_last_system_message(&result, "[/login: anthropic succeeded]");
    assert_eq!(presenter_counts(&evidence), (0, 1, 1, 1, 0));
}

#[tokio::test(flavor = "current_thread")]
async fn outer_tui_midstream_revocation_never_opens_login_or_retries() {
    let _lock = test_lock().await;
    let provider = MockProvider::new_with_errors(
        "mock",
        vec![MockResponse::EventsThenError(
            vec![
                AssistantStreamEvent::Start {
                    partial: test_support::base_assistant(),
                },
                AssistantStreamEvent::TextDelta {
                    content_index: 0,
                    delta: "partial".into(),
                    partial: test_support::base_assistant(),
                },
            ],
            ProviderError::CredentialRevoked {
                provider_id: "anthropic".into(),
            },
        )],
    );
    let call_log = provider.call_log_handle();
    let workspace = tempfile::tempdir().unwrap();
    let mut harness = CodingHarness::builder(
        Box::new(provider),
        "mock:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .tool_selection(ToolSelection::Disabled)
    .build();
    harness.credential_store = Some(Arc::new(test_store(workspace.path())));
    let driver = install_interactive_tui_test_driver(["normal prompt", "exit"]).unwrap();

    run_interactive_tui(
        harness,
        "mock:mock-model".into(),
        "default",
        Keybindings::default(),
    )
    .await
    .expect("midstream revocation returns to the TUI");

    let capture = driver.capture();
    assert_eq!(capture.user_messages, 1);
    assert_eq!(capture.provider_calls, 1);
    assert_eq!(capture.retries, 0);
    assert_eq!(capture.presenter_constructions, 0);
    let recorded_messages = call_log
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .map(|request| request.messages.clone())
        .collect::<Vec<_>>();
    assert_one_user_message(&recorded_messages);
}

#[tokio::test(flavor = "current_thread")]
async fn json_rpc_and_text_credential_needed_never_construct_presenter() {
    let _lock = test_lock().await;
    reset_interactive_tui_presenter_construction_count();
    let workspace = tempfile::tempdir().unwrap();

    let (json_runner, json_calls) = tracked_credential_runner(workspace.path());
    let json = run_json_credential_capture(json_runner).await;
    let (text_runner, text_calls) = tracked_credential_runner(workspace.path());
    let text = run_text_credential_capture(text_runner).await;
    let rpc = run_rpc_stdio_capture("phase14_outer_tui_rpc_credential_child");

    assert_ne!(json["exit_code"], 0);
    assert_ne!(text["exit_code"], 0);
    assert!(
        rpc.iter()
            .any(|record| record["type"] == "CredentialNeeded")
    );
    assert_one_noninteractive_user_message(&json_calls);
    assert_one_noninteractive_user_message(&text_calls);
    assert_eq!(interactive_tui_presenter_construction_count(), 0);
}

#[tokio::test]
#[ignore = "subprocess-only RPC stdio entry point"]
async fn phase14_outer_tui_rpc_credential_child() {
    reset_interactive_tui_presenter_construction_count();
    let workspace = tempfile::tempdir().expect("RPC child workspace");
    let provider = MockProvider::new_with_errors(
        "anthropic",
        vec![MockResponse::Error(ProviderError::CredentialNeeded {
            provider_id: "anthropic".into(),
        })],
    );
    let calls = provider.call_log_handle();
    let mut runner = RpcRunner::new(
        Box::new(provider),
        "anthropic:mock-model".into(),
        OpiConfig::default(),
        workspace.path().to_path_buf(),
        false,
        ToolSelection::Disabled,
        None,
        Vec::new(),
        opi_coding_agent::project_trust::TrustDecision::Trusted,
    )
    .expect("construct RPC runner");
    assert_eq!(runner.run().await, 0);
    assert_one_noninteractive_user_message(&calls);
    assert_eq!(interactive_tui_presenter_construction_count(), 0);
}
