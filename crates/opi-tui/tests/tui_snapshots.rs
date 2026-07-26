//! Snapshot tests for TUI components (task 1.12).
//!
//! DoD: "fixed-size render snapshots"

use opi_tui::{
    AppStatus, InputEditor, Message, MessageList, Role, Shell, StatusBar, ToolCallStatus,
    ToolCallView,
};
use ratatui::{Terminal, backend::TestBackend, widgets::Widget};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn render<W: Widget>(widget: W, w: u16, h: u16) -> String {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(widget, f.area()))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// MessageList
// ---------------------------------------------------------------------------

#[test]
fn message_list_with_messages_80x24() {
    let messages = vec![
        Message::new(Role::User, "Hello, how are you?"),
        Message::new(Role::Assistant, "I'm doing well, thanks!"),
        Message::new(Role::User, "Can you help me with Rust?"),
    ];
    let widget = MessageList::new(messages);
    insta::assert_snapshot!("message_list_with_messages_80x24", render(widget, 80, 24));
}

#[test]
fn message_list_with_messages_120x40() {
    let messages = vec![
        Message::new(Role::User, "Hello!"),
        Message::new(Role::Assistant, "Hi there!"),
    ];
    let widget = MessageList::new(messages);
    insta::assert_snapshot!("message_list_with_messages_120x40", render(widget, 120, 40));
}

#[test]
fn message_list_empty() {
    let widget = MessageList::new(vec![]);
    insta::assert_snapshot!("message_list_empty_80x10", render(widget, 80, 10));
}

// ---------------------------------------------------------------------------
// InputEditor
// ---------------------------------------------------------------------------

#[test]
fn input_editor_with_text() {
    let editor = InputEditor::new("help me refactor this code".into());
    insta::assert_snapshot!("input_editor_with_text_80x3", render(editor, 80, 3));
}

#[test]
fn input_editor_empty() {
    let editor = InputEditor::empty();
    insta::assert_snapshot!("input_editor_empty_80x3", render(editor, 80, 3));
}

// ---------------------------------------------------------------------------
// StatusBar
// ---------------------------------------------------------------------------

#[test]
fn status_bar_idle() {
    let bar = StatusBar::new("claude-sonnet-4-5-20250514".into(), AppStatus::Idle, None);
    insta::assert_snapshot!("status_bar_idle_80x1", render(bar, 80, 1));
}

#[test]
fn status_bar_thinking_with_tokens() {
    let bar = StatusBar::new(
        "claude-sonnet-4-5-20250514".into(),
        AppStatus::Thinking,
        Some(150),
    );
    insta::assert_snapshot!("status_bar_thinking_80x1", render(bar, 80, 1));
}

#[test]
fn status_bar_tool_executing() {
    let bar = StatusBar::new(
        "claude-sonnet-4-5-20250514".into(),
        AppStatus::ToolExecuting,
        Some(350),
    );
    insta::assert_snapshot!("status_bar_tool_executing_80x1", render(bar, 80, 1));
}

// ---------------------------------------------------------------------------
// ToolCallView
// ---------------------------------------------------------------------------

#[test]
fn tool_call_running() {
    let tc = ToolCallView::new(
        "grep".into(),
        r#"{"pattern": "TODO"}"#.into(),
        ToolCallStatus::Running,
    );
    insta::assert_snapshot!("tool_call_running_80x5", render(tc, 80, 5));
}

#[test]
fn tool_call_success() {
    let tc = ToolCallView::new(
        "read".into(),
        r#"{"path": "/src/main.rs"}"#.into(),
        ToolCallStatus::Success,
    );
    insta::assert_snapshot!("tool_call_success_80x5", render(tc, 80, 5));
}

#[test]
fn tool_call_error() {
    let tc = ToolCallView::new(
        "bash".into(),
        r#"{"command": "rm -rf /"}"#.into(),
        ToolCallStatus::Error("permission denied".into()),
    );
    insta::assert_snapshot!("tool_call_error_80x5", render(tc, 80, 5));
}

// ---------------------------------------------------------------------------
// Shell (full layout)
// ---------------------------------------------------------------------------

#[test]
fn shell_idle_empty_80x24() {
    let shell = Shell::new("claude-sonnet-4-5-20250514".into());
    insta::assert_snapshot!("shell_idle_empty_80x24", render(shell, 80, 24));
}

#[test]
fn shell_idle_with_messages_80x24() {
    let shell = Shell::new("claude-sonnet-4-5-20250514".into())
        .messages(vec![
            Message::new(Role::User, "What is the capital of France?"),
            Message::new(Role::Assistant, "The capital of France is Paris."),
            Message::new(Role::User, "And Germany?"),
            Message::new(Role::Assistant, "The capital of Germany is Berlin."),
        ])
        .input_text("Tell me more about Berlin.".into());
    insta::assert_snapshot!("shell_idle_with_messages_80x24", render(shell, 80, 24));
}

#[test]
fn shell_thinking_with_tool_80x24() {
    let shell = Shell::new("claude-sonnet-4-5-20250514".into())
        .state(AppStatus::Thinking)
        .token_count(250)
        .messages(vec![Message::new(
            Role::User,
            "Search for TODOs in the codebase.",
        )])
        .active_tool(
            "grep".into(),
            r#"{"pattern": "TODO"}"#.into(),
            ToolCallStatus::Running,
        );
    insta::assert_snapshot!("shell_thinking_with_tool_80x24", render(shell, 80, 24));
}

#[test]
fn shell_with_conversation_120x40() {
    let shell = Shell::new("claude-sonnet-4-5-20250514".into())
        .state(AppStatus::Idle)
        .token_count(520)
        .messages(vec![
            Message::new(Role::User, "Hello!"),
            Message::new(Role::Assistant, "Hi! How can I help?"),
            Message::new(Role::User, "Can you search for all Rust files?"),
            Message::new(Role::Assistant, "I found 15 Rust files in the workspace."),
            Message::new(Role::User, "Show me the main one."),
        ])
        .input_text("What does the main function do?".into());
    insta::assert_snapshot!("shell_with_conversation_120x40", render(shell, 120, 40));
}
