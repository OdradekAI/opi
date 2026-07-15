use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use opi_ai::auth::{LoginPresenter, OAuthCredential, OAuthProvider};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialStore};
use opi_ai::provider::ProviderError;
use opi_coding_agent::credential_store::{FakeKeyringBackend, KeychainCredentialStore};
use opi_coding_agent::interactive_auth::{
    AuthCommandOutcome, AuthCommandServices, LoginTerminalControl, dispatch_auth_command,
};
use opi_coding_agent::oauth::OAuthProviderRegistry;
use secrecy::SecretString;

const PROVIDERS: [&str; 3] = ["anthropic", "copilot", "codex"];
const SECRET_CANARY: &str = "AUTH-DO-NOT-LEAK";

#[derive(Clone, Copy)]
enum LoginBehavior {
    Success,
    ProviderFailure,
    PresenterFailure,
    Pending,
    Timeout,
}

struct FakeOAuthProvider {
    id: &'static str,
    behavior: LoginBehavior,
}

impl OAuthProvider for FakeOAuthProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn login<'a>(
        &'a self,
        presenter: &'a dyn LoginPresenter,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        let id = self.id;
        let behavior = self.behavior;
        Box::pin(async move {
            match behavior {
                LoginBehavior::Success => Ok(oauth_credential(id)),
                LoginBehavior::ProviderFailure => {
                    Err(ProviderError::Network(SECRET_CANARY.to_owned()))
                }
                LoginBehavior::PresenterFailure => {
                    presenter
                        .present_auth_url("https://login.example/authorize")
                        .await?;
                    Ok(oauth_credential(id))
                }
                LoginBehavior::Pending => {
                    std::future::pending::<Result<OAuthCredential, ProviderError>>().await
                }
                LoginBehavior::Timeout => Err(ProviderError::Timeout),
            }
        })
    }

    fn refresh<'a>(
        &'a self,
        _cred: &'a OAuthCredential,
    ) -> BoxAuthFuture<'a, Result<OAuthCredential, ProviderError>> {
        Box::pin(async { Err(ProviderError::Config("unused fake refresh".into())) })
    }
}

fn oauth_credential(provider_id: &str) -> OAuthCredential {
    OAuthCredential {
        access: SecretString::new(format!("access-{provider_id}").into_boxed_str()),
        refresh: SecretString::new(format!("refresh-{provider_id}").into_boxed_str()),
        expires_at: None,
        base_url: None,
    }
}

#[derive(Default)]
struct MockPresenter {
    fail_auth_url: bool,
    success_count: AtomicUsize,
}

impl LoginPresenter for MockPresenter {
    fn present_auth_url<'a>(
        &'a self,
        _url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let fail = self.fail_auth_url;
        Box::pin(async move {
            if fail {
                Err(ProviderError::Config(SECRET_CANARY.to_owned()))
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
        Box::pin(async { std::future::pending::<Result<String, ProviderError>>().await })
    }

    fn notify_success(&self) {
        self.success_count.fetch_add(1, Ordering::SeqCst);
    }

    fn notify_failure(&self, _reason: &str) {}
}

#[derive(Default)]
struct RecordingTerminal {
    transitions: Vec<&'static str>,
    fail_suspend: bool,
    fail_resume: bool,
}

impl LoginTerminalControl for RecordingTerminal {
    fn suspend_for_login(&mut self) -> io::Result<()> {
        self.transitions.push("suspend");
        if self.fail_suspend {
            Err(io::Error::other(SECRET_CANARY))
        } else {
            Ok(())
        }
    }

    fn resume_after_login(&mut self) -> io::Result<()> {
        self.transitions.push("resume");
        if self.fail_resume {
            Err(io::Error::other(SECRET_CANARY))
        } else {
            Ok(())
        }
    }
}

fn registry_with(
    providers: impl IntoIterator<Item = (&'static str, LoginBehavior)>,
) -> OAuthProviderRegistry {
    let mut registry = OAuthProviderRegistry::new();
    for (id, behavior) in providers {
        registry
            .register(Arc::new(FakeOAuthProvider { id, behavior }))
            .unwrap();
    }
    registry
}

fn test_store(root: &std::path::Path, timeout: Duration) -> KeychainCredentialStore {
    KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new()),
        root.to_path_buf(),
        timeout,
    )
}

fn services<'a>(
    store: &'a KeychainCredentialStore,
    registry: &'a OAuthProviderRegistry,
    presenter: &'a dyn LoginPresenter,
) -> AuthCommandServices<'a> {
    AuthCommandServices {
        store,
        registry,
        presenter,
    }
}

#[tokio::test]
async fn interactive_auth_dispatcher_persists_and_deletes_all_profiles() {
    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_secs(1));
    let registry = registry_with(PROVIDERS.map(|id| (id, LoginBehavior::Success)));
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal::default();

    for provider_id in PROVIDERS {
        let outcome = dispatch_auth_command(
            &format!("/login {provider_id}"),
            &mut terminal,
            services(&store, &registry, &presenter),
        )
        .await;
        assert_eq!(
            outcome,
            AuthCommandOutcome::LoggedIn {
                provider_id: provider_id.to_owned(),
            }
        );
        assert!(
            matches!(
                store.read(provider_id).await.unwrap(),
                Some(Credential::OAuthToken { .. })
            ),
            "{provider_id} OAuth envelope was not persisted"
        );

        let outcome = dispatch_auth_command(
            &format!("/logout {provider_id}"),
            &mut terminal,
            services(&store, &registry, &presenter),
        )
        .await;
        assert_eq!(
            outcome,
            AuthCommandOutcome::LoggedOut {
                provider_id: provider_id.to_owned(),
            }
        );
        assert!(store.read(provider_id).await.unwrap().is_none());
    }
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 3);
    assert_eq!(
        terminal.transitions,
        [
            "suspend", "resume", "suspend", "resume", "suspend", "resume"
        ]
    );
}

#[tokio::test]
async fn interactive_auth_dispatcher_restores_terminal_on_every_exit() {
    for (behavior, presenter, expected, expected_successes) in [
        (
            LoginBehavior::Success,
            MockPresenter::default(),
            AuthCommandOutcome::LoggedIn {
                provider_id: "anthropic".to_owned(),
            },
            1,
        ),
        (
            LoginBehavior::ProviderFailure,
            MockPresenter::default(),
            AuthCommandOutcome::Failed {
                message: "authentication network request failed".to_owned(),
            },
            0,
        ),
        (
            LoginBehavior::PresenterFailure,
            MockPresenter {
                fail_auth_url: true,
                success_count: AtomicUsize::new(0),
            },
            AuthCommandOutcome::Failed {
                message: "authentication configuration failed".to_owned(),
            },
            0,
        ),
        (
            LoginBehavior::Timeout,
            MockPresenter::default(),
            AuthCommandOutcome::Failed {
                message: "authentication timed out".to_owned(),
            },
            0,
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let store = test_store(root.path(), Duration::from_secs(1));
        let registry = registry_with([("anthropic", behavior)]);
        let mut terminal = RecordingTerminal::default();
        let outcome = dispatch_auth_command(
            "/login anthropic",
            &mut terminal,
            services(&store, &registry, &presenter),
        )
        .await;

        assert_eq!(outcome, expected);
        if let AuthCommandOutcome::Failed { message } = &outcome {
            assert!(!message.contains(SECRET_CANARY), "secret leaked: {message}");
        }
        assert_eq!(
            presenter.success_count.load(Ordering::SeqCst),
            expected_successes
        );
        assert_eq!(terminal.transitions, ["suspend", "resume"]);
    }

    let root = tempfile::tempdir().unwrap();
    let unavailable_store = KeychainCredentialStore::with_lock_timeout(
        Box::new(FakeKeyringBackend::new().with_unavailable()),
        root.path().to_path_buf(),
        Duration::from_secs(1),
    );
    let registry = registry_with([("anthropic", LoginBehavior::Success)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal::default();
    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        services(&unavailable_store, &registry, &presenter),
    )
    .await;
    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "credential store operation failed".to_owned(),
        }
    );
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 0);
    assert_eq!(terminal.transitions, ["suspend", "resume"]);

    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_secs(1));
    let registry = registry_with([("anthropic", LoginBehavior::Pending)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal::default();
    let mut future = Box::pin(dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        services(&store, &registry, &presenter),
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(25), future.as_mut())
            .await
            .is_err(),
        "pending login unexpectedly completed"
    );
    drop(future);
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}

#[tokio::test]
async fn interactive_auth_dispatcher_reports_terminal_restore_failures_once() {
    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_secs(1));
    let registry = registry_with([("anthropic", LoginBehavior::Success)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal {
        fail_suspend: true,
        fail_resume: true,
        ..Default::default()
    };

    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        services(&store, &registry, &presenter),
    )
    .await;

    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "terminal restore failed".to_owned(),
        }
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 0);

    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_secs(1));
    let registry = registry_with([("anthropic", LoginBehavior::Success)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal {
        fail_resume: true,
        ..Default::default()
    };

    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        services(&store, &registry, &presenter),
    )
    .await;

    assert_eq!(
        outcome,
        AuthCommandOutcome::Failed {
            message: "terminal restore failed".to_owned(),
        }
    );
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn interactive_auth_dispatcher_parses_the_reviewed_command_forms() {
    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_secs(1));
    let registry = registry_with([("anthropic", LoginBehavior::Success)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal::default();
    let cases = [
        (
            "/login",
            AuthCommandOutcome::Usage("usage: /login <provider>".to_owned()),
        ),
        (
            "/logout",
            AuthCommandOutcome::Usage("usage: /logout <provider>".to_owned()),
        ),
        (
            "  /login   anthropic  ",
            AuthCommandOutcome::LoggedIn {
                provider_id: "anthropic".to_owned(),
            },
        ),
        (
            "  /logout   anthropic  ",
            AuthCommandOutcome::LoggedOut {
                provider_id: "anthropic".to_owned(),
            },
        ),
        ("/model", AuthCommandOutcome::NotHandled),
        (
            "/login anthropic extra",
            AuthCommandOutcome::Failed {
                message: "authentication configuration failed".to_owned(),
            },
        ),
    ];

    for (input, expected) in cases {
        let outcome = dispatch_auth_command(
            input,
            &mut terminal,
            services(&store, &registry, &presenter),
        )
        .await;
        assert_eq!(outcome, expected, "input: {input:?}");
    }
    assert_eq!(
        terminal.transitions,
        ["suspend", "resume", "suspend", "resume"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_auth_dispatcher_reports_lock_failure_without_success() {
    let root = tempfile::tempdir().unwrap();
    let store = test_store(root.path(), Duration::from_millis(50));
    let registry = registry_with([("anthropic", LoginBehavior::Success)]);
    let presenter = MockPresenter::default();
    let mut terminal = RecordingTerminal::default();
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(root.path().join("credential.lock"))
        .unwrap();
    fs4::FileExt::lock(&lock_file).unwrap();

    let outcome = dispatch_auth_command(
        "/login anthropic",
        &mut terminal,
        services(&store, &registry, &presenter),
    )
    .await;
    fs4::FileExt::unlock(&lock_file).unwrap();

    let message = match outcome {
        AuthCommandOutcome::Failed { message } => message,
        other => panic!("lock failure must not report success: {other:?}"),
    };
    assert!(message.contains("credential store"), "message: {message}");
    assert!(!message.contains(SECRET_CANARY), "secret leaked: {message}");
    assert_eq!(presenter.success_count.load(Ordering::SeqCst), 0);
    assert!(store.read("anthropic").await.unwrap().is_none());
    assert_eq!(terminal.transitions, ["suspend", "resume"]);
}
