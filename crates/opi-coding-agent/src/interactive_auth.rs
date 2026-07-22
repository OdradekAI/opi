//! Interactive `/login` and `/logout` command dispatch.

use std::io;

use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use opi_ai::auth::LoginPresenter;
use opi_ai::provider::ProviderError;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::credential_store::KeychainCredentialStore;
use crate::oauth::{self, OAuthEndpointConfig, OAuthProviderRegistry};

pub const AUTH_HELP: &[(&str, &str)] = &[
    (
        "/login <provider>",
        "authenticate and persist an OAuth credential",
    ),
    ("/logout <provider>", "delete the persisted credential"),
];

pub trait LoginTerminalControl {
    fn suspend_for_login(&mut self) -> io::Result<()>;
    fn resume_after_login(&mut self) -> io::Result<()>;
}

impl LoginTerminalControl for Terminal<CrosstermBackend<io::Stdout>> {
    fn suspend_for_login(&mut self) -> io::Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(self.backend_mut(), LeaveAlternateScreen)?;
        self.show_cursor()
    }

    fn resume_after_login(&mut self) -> io::Result<()> {
        crossterm::execute!(self.backend_mut(), EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
        self.hide_cursor()?;
        self.clear()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthCommandOutcome {
    NotHandled,
    Usage(String),
    LoggedIn { provider_id: String },
    LoggedOut { provider_id: String },
    Failed { message: String },
}

pub struct AuthCommandServices<'a> {
    pub store: &'a KeychainCredentialStore,
    pub presenter: &'a dyn LoginPresenter,
    endpoints: OAuthEndpointConfig,
    client: reqwest::Client,
}

impl<'a> AuthCommandServices<'a> {
    pub(crate) fn new(
        store: &'a KeychainCredentialStore,
        presenter: &'a dyn LoginPresenter,
        endpoints: OAuthEndpointConfig,
        client: reqwest::Client,
    ) -> Self {
        Self {
            store,
            presenter,
            endpoints,
            client,
        }
    }

    #[doc(hidden)]
    pub fn with_test_services(
        store: &'a KeychainCredentialStore,
        presenter: &'a dyn LoginPresenter,
        client: reqwest::Client,
        endpoint_base_url: String,
        login_timeout: std::time::Duration,
        codex_device_timeout: std::time::Duration,
    ) -> Self {
        Self::new(
            store,
            presenter,
            OAuthEndpointConfig::with_test_base_url(
                endpoint_base_url,
                login_timeout,
                codex_device_timeout,
            ),
            client,
        )
    }
}

const TERMINAL_RESTORE_FAILURE: &str = "terminal restore failed";

pub(crate) fn is_terminal_restore_failure(outcome: &AuthCommandOutcome) -> bool {
    matches!(
        outcome,
        AuthCommandOutcome::Failed { message } if message == TERMINAL_RESTORE_FAILURE
    )
}

fn terminal_restore_failure() -> AuthCommandOutcome {
    AuthCommandOutcome::Failed {
        message: TERMINAL_RESTORE_FAILURE.to_owned(),
    }
}

#[derive(Debug)]
enum TerminalGuardError {
    Suspension,
    Restore,
}

struct LoginTerminalGuard<'a, T: LoginTerminalControl> {
    terminal: &'a mut T,
    resume_required: bool,
}

impl<'a, T: LoginTerminalControl> LoginTerminalGuard<'a, T> {
    fn new(terminal: &'a mut T) -> Result<Self, TerminalGuardError> {
        if terminal.suspend_for_login().is_err() {
            // Suspension can fail after raw mode was disabled or after the
            // alternate screen was left. Make one best-effort restore call.
            return match terminal.resume_after_login() {
                Ok(()) => Err(TerminalGuardError::Suspension),
                Err(_) => Err(TerminalGuardError::Restore),
            };
        }
        Ok(Self {
            terminal,
            resume_required: true,
        })
    }

    fn resume(mut self) -> io::Result<()> {
        self.resume_required = false;
        self.terminal.resume_after_login()
    }
}

impl<T: LoginTerminalControl> Drop for LoginTerminalGuard<'_, T> {
    fn drop(&mut self) {
        if self.resume_required {
            self.resume_required = false;
            let _ = self.terminal.resume_after_login();
        }
    }
}

enum ParsedAuthCommand<'a> {
    Login(&'a str),
    Logout(&'a str),
    Help,
    Usage(&'static str),
    NotHandled,
}

fn parse_auth_command(input: &str) -> ParsedAuthCommand<'_> {
    let input = input.trim();
    if input == "/help" {
        return ParsedAuthCommand::Help;
    }
    for (command, usage, login) in [
        ("/login", "usage: /login <provider>", true),
        ("/logout", "usage: /logout <provider>", false),
    ] {
        if input == command {
            return ParsedAuthCommand::Usage(usage);
        }
        if let Some(rest) = input.strip_prefix(command)
            && rest.chars().next().is_some_and(char::is_whitespace)
        {
            let provider_id = rest.trim();
            if provider_id.is_empty() {
                return ParsedAuthCommand::Usage(usage);
            }
            return if login {
                ParsedAuthCommand::Login(provider_id)
            } else {
                ParsedAuthCommand::Logout(provider_id)
            };
        }
    }
    ParsedAuthCommand::NotHandled
}

pub(crate) fn is_auth_command(input: &str) -> bool {
    !matches!(parse_auth_command(input), ParsedAuthCommand::NotHandled)
}

pub(crate) fn auth_command_requires_presenter(input: &str) -> bool {
    matches!(parse_auth_command(input), ParsedAuthCommand::Login(_))
}

/// Dispatch an interactive authentication slash command.
///
/// Only login suspends the terminal. The guard restores it on every normal
/// return and when the in-flight future is cancelled and dropped.
pub async fn dispatch_auth_command<T: LoginTerminalControl>(
    input: &str,
    terminal: &mut T,
    services: AuthCommandServices<'_>,
) -> AuthCommandOutcome {
    let registry =
        OAuthProviderRegistry::registry_with_services(&services.endpoints, services.client.clone());
    match parse_auth_command(input) {
        ParsedAuthCommand::NotHandled => AuthCommandOutcome::NotHandled,
        ParsedAuthCommand::Help => AuthCommandOutcome::Usage(
            AUTH_HELP
                .iter()
                .map(|(command, description)| format!("{command}  {description}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        ParsedAuthCommand::Usage(usage) => AuthCommandOutcome::Usage(usage.to_owned()),
        ParsedAuthCommand::Logout(provider_id) => {
            if registry.lookup(provider_id).is_none() {
                return AuthCommandOutcome::Failed {
                    message: "unknown OAuth provider".to_owned(),
                };
            }
            match oauth::logout_credential(provider_id, services.store).await {
                Ok(()) => AuthCommandOutcome::LoggedOut {
                    provider_id: provider_id.to_owned(),
                },
                Err(error) => AuthCommandOutcome::Failed {
                    message: present_provider_error(&error),
                },
            }
        }
        ParsedAuthCommand::Login(provider_id) => {
            let guard = match LoginTerminalGuard::new(terminal) {
                Ok(guard) => guard,
                Err(TerminalGuardError::Suspension) => {
                    return AuthCommandOutcome::Failed {
                        message: "terminal suspension failed".to_owned(),
                    };
                }
                Err(TerminalGuardError::Restore) => return terminal_restore_failure(),
            };
            let login_result =
                oauth::login_oauth(provider_id, &registry, services.store, services.presenter)
                    .await;
            let resume_result = guard.resume();

            if resume_result.is_err() {
                return terminal_restore_failure();
            }
            match login_result {
                Ok(()) => AuthCommandOutcome::LoggedIn {
                    provider_id: provider_id.to_owned(),
                },
                Err(error) => AuthCommandOutcome::Failed {
                    message: present_provider_error(&error),
                },
            }
        }
    }
}

fn present_provider_error(error: &ProviderError) -> String {
    match error {
        ProviderError::RateLimited { .. } => "authentication was rate limited",
        ProviderError::Timeout => "authentication timed out",
        ProviderError::RequestFailed(_) | ProviderError::UnknownModel { .. } => {
            "authentication request failed"
        }
        ProviderError::StreamError(_) => "authentication stream failed",
        ProviderError::AuthFailed(_) => "authentication failed",
        ProviderError::CredentialNeeded { provider_id } => {
            return format!("credential is still required for provider '{provider_id}'");
        }
        ProviderError::CredentialRevoked { provider_id } => {
            return format!("credential was denied or expired for provider '{provider_id}'");
        }
        ProviderError::AccountIdMissing { provider_id } => {
            return format!("credential for provider '{provider_id}' is missing its account id");
        }
        ProviderError::Network(_) => "authentication network request failed",
        ProviderError::Config(message) if message.starts_with("credential store") => {
            "credential store operation failed"
        }
        ProviderError::Config(_)
        | ProviderError::MissingWireRoute { .. }
        | ProviderError::WireCompatMismatch { .. } => "authentication configuration failed",
        ProviderError::ProviderSide(_) => "authentication provider failed",
        ProviderError::UnsupportedCapability(_) => "authentication is not supported",
        ProviderError::Cancelled => "authentication cancelled",
        ProviderError::LoginCancelled { provider_id } => {
            return format!("authentication cancelled for provider '{provider_id}'");
        }
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingTerminal {
        transitions: Vec<&'static str>,
    }

    impl LoginTerminalControl for RecordingTerminal {
        fn suspend_for_login(&mut self) -> io::Result<()> {
            self.transitions.push("suspend");
            Ok(())
        }

        fn resume_after_login(&mut self) -> io::Result<()> {
            self.transitions.push("resume");
            Ok(())
        }
    }

    #[tokio::test]
    async fn login_terminal_guard_suspends_then_resumes_once() {
        let mut terminal = RecordingTerminal::default();
        let guard = LoginTerminalGuard::new(&mut terminal).unwrap();
        let resume_result = guard.resume();

        assert!(resume_result.is_ok());
        assert_eq!(terminal.transitions, ["suspend", "resume"]);
    }

    #[test]
    fn provider_auth_errors_name_the_canonical_provider_without_secrets() {
        const ENV_CANARY: &str = "OPI_AUTH_ERROR_ENV_CANARY";
        const SECRET_CANARY: &str = "auth-error-secret-canary";

        let cases = [
            (
                ProviderError::CredentialNeeded {
                    provider_id: "anthropic".into(),
                },
                "credential is still required for provider 'anthropic'",
            ),
            (
                ProviderError::CredentialRevoked {
                    provider_id: "openai-codex".into(),
                },
                "credential was denied or expired for provider 'openai-codex'",
            ),
            (
                ProviderError::AccountIdMissing {
                    provider_id: "github-copilot".into(),
                },
                "credential for provider 'github-copilot' is missing its account id",
            ),
            (
                ProviderError::LoginCancelled {
                    provider_id: "openai-codex".into(),
                },
                "authentication cancelled for provider 'openai-codex'",
            ),
        ];

        for (error, expected) in cases {
            let message = present_provider_error(&error);
            assert_eq!(message, expected);
            assert!(!message.contains(ENV_CANARY), "{message}");
            assert!(!message.contains(SECRET_CANARY), "{message}");
        }
    }
}
