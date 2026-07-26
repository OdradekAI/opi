//! Terminal User Interface library with differential rendering.
//!
//! Synchronous ratatui widget library used by `opi-coding-agent`. Core widgets:
//! [`MessageList`], [`InputEditor`], [`StatusBar`], [`ToolCallView`],
//! [`MarkdownView`], [`CodeBlock`], [`DiffView`], composed by [`Shell`].
//! Themes via [`Theme`]/[`resolve_theme`] and configurable [`Keybindings`].

pub mod branch_picker;
pub mod diff_view;
pub mod editor;
pub mod keybindings;
pub mod markdown;
pub mod message_list;
pub mod render;
pub mod select_list;
pub mod status_bar;
pub mod terminal_image;
pub mod theme;
pub mod tool_call;
pub mod trust_prompt;

pub use branch_picker::{BranchItem, BranchPicker, BranchPickerOutcome, BranchPickerState};
pub use diff_view::DiffView;
pub use editor::InputEditor;
pub use keybindings::{Key, KeyCombo, KeyComboParseError, Keybindings, Modifiers};
pub use markdown::{CodeBlock, MarkdownView};
pub use message_list::MessageList;
pub use render::Shell;
pub use select_list::{SelectItem, SelectList, SelectListState, fuzzy_match};
pub use status_bar::StatusBar;
pub use terminal_image::{
    CapabilitySource, ImageData, MediaType, TerminalGraphicsProtocol, detect_graphics_protocol,
    iterm_escape, kitty_escape, sixel_escape, text_fallback,
};
pub use theme::{ColorParseError, THEME_TOKENS, Theme, is_valid_token, parse_color, resolve_theme};
pub use tool_call::ToolCallView;
pub use trust_prompt::{AwaitingTrustState, TrustChoice, TrustPrompt};

use std::fmt;

/// Error type for TUI operations.
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("terminal error: {0}")]
    Terminal(String),
    #[error("render error: {0}")]
    Render(String),
}

/// Role of a conversation message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

/// A single conversation message.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// Optional structured diff payload. When present, the message is
    /// rendered with the `DiffView` widget instead of plain text.
    pub diff: Option<DiffPayload>,
    /// Optional image payload. When present, the message renders an image
    /// using terminal graphics protocol escape sequences or text fallback.
    pub image: Option<ImagePayload>,
}

/// Image payload for terminal rendering.
#[derive(Debug, Clone)]
pub struct ImagePayload {
    pub data: ImageData,
    pub protocol: TerminalGraphicsProtocol,
}

/// Structured before/after content for diff rendering.
#[derive(Debug, Clone)]
pub struct DiffPayload {
    pub path: String,
    pub before: String,
    pub after: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            diff: None,
            image: None,
        }
    }

    /// Build an image-only message for terminal rendering.
    pub fn image(role: Role, payload: ImagePayload) -> Self {
        let fallback = text_fallback(&payload.data);
        Self {
            role,
            content: fallback,
            diff: None,
            image: Some(payload),
        }
    }

    /// Attach an image payload to an existing text message.
    pub fn with_image(mut self, payload: ImagePayload) -> Self {
        self.image = Some(payload);
        self
    }

    /// Build a tool-role message that renders the supplied before/after as a
    /// unified diff via `DiffView`.
    pub fn diff(
        path: impl Into<String>,
        before: impl Into<String>,
        after: impl Into<String>,
    ) -> Self {
        let path = path.into();
        let before = before.into();
        let after = after.into();
        let content = format!("diff: {path}");
        Self {
            role: Role::Tool,
            content,
            diff: Some(DiffPayload {
                path,
                before,
                after,
            }),
            image: None,
        }
    }
}

/// Application state shown in the status bar / driving the trust-prompt phase.
///
/// The [`Self::AwaitingTrust`] variant carries the interactive trust-prompt
/// payload (project path + oneshot responder). Because that payload owns a
/// `oneshot::Sender`, `AppState` is **not** `Copy`/`Clone`/`PartialEq`/`Eq`.
/// Renderers that only need a display label use the [`AppStatus`] projection
/// returned by [`Self::status`].
#[derive(Debug)]
pub enum AppState {
    Idle,
    Thinking,
    Streaming,
    ToolExecuting,
    AwaitingTrust(AwaitingTrustState),
}

impl AppState {
    /// Render-safe, `Copy` projection of this state for status bars / shells
    /// that must not own the `AwaitingTrust` oneshot payload.
    pub fn status(&self) -> AppStatus {
        match self {
            Self::Idle => AppStatus::Idle,
            Self::Thinking => AppStatus::Thinking,
            Self::Streaming => AppStatus::Streaming,
            Self::ToolExecuting => AppStatus::ToolExecuting,
            Self::AwaitingTrust(_) => AppStatus::AwaitingTrust,
        }
    }
}

impl fmt::Display for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status())
    }
}

/// Copyable status label projected from [`AppState`] for renderers (status bar,
/// shell) that must not own the `AwaitingTrust` oneshot payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppStatus {
    #[default]
    Idle,
    Thinking,
    Streaming,
    ToolExecuting,
    AwaitingTrust,
}

impl fmt::Display for AppStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Thinking => write!(f, "thinking..."),
            Self::Streaming => write!(f, "streaming..."),
            Self::ToolExecuting => write!(f, "executing tool..."),
            Self::AwaitingTrust => write!(f, "awaiting trust..."),
        }
    }
}

/// Status of a tool call being displayed.
#[derive(Debug, Clone)]
pub enum ToolCallStatus {
    Running,
    Success,
    Error(String),
}

impl fmt::Display for ToolCallStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Success => write!(f, "success"),
            Self::Error(e) => write!(f, "error: {e}"),
        }
    }
}
