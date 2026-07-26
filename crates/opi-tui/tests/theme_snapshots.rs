//! Theme system tests (task 2.12).
//!
//! DoD: "Theme struct and palette model in opi-tui, default + at least one alt theme,
//!       theme loading wired from [defaults].theme config in opi-coding-agent,
//!       snapshot tests at 80x24 and 120x40"

use opi_tui::{AppStatus, Message, Role, Shell, StatusBar, Theme};
use ratatui::style::Color;
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
// Theme struct tests
// ---------------------------------------------------------------------------

#[test]
fn theme_default_has_name() {
    let theme = Theme::default();
    assert_eq!(theme.name, "default");
}

#[test]
fn theme_default_status_bar_colors() {
    let t = Theme::default();
    assert_eq!(t.status_bg, Color::DarkGray);
    assert_eq!(t.status_idle, Color::White);
    assert_eq!(t.status_thinking, Color::Yellow);
    assert_eq!(t.status_streaming, Color::Green);
    assert_eq!(t.status_tool, Color::Magenta);
    assert_eq!(t.status_tokens, Color::DarkGray);
}

#[test]
fn theme_default_role_colors() {
    let t = Theme::default();
    assert_eq!(t.role_user, Color::Green);
    assert_eq!(t.role_assistant, Color::Cyan);
    assert_eq!(t.role_system, Color::Yellow);
    assert_eq!(t.role_tool, Color::Magenta);
}

#[test]
fn theme_default_editor_colors() {
    let t = Theme::default();
    assert_eq!(t.editor_title, Color::Yellow);
    assert_eq!(t.editor_placeholder, Color::DarkGray);
}

#[test]
fn theme_default_markdown_colors() {
    let t = Theme::default();
    assert_eq!(t.code_title, Color::Yellow);
    assert_eq!(t.code_content, Color::Gray);
    assert_eq!(t.heading_h1, Color::Cyan);
    assert_eq!(t.heading_h2, Color::Yellow);
    assert_eq!(t.heading_h3, Color::White);
    assert_eq!(t.italic, Color::Cyan);
}

#[test]
fn theme_default_diff_colors() {
    let t = Theme::default();
    assert_eq!(t.diff_border, Color::Cyan);
    assert_eq!(t.diff_header, Color::Blue);
    assert_eq!(t.diff_context, Color::Gray);
    assert_eq!(t.diff_added, Color::Green);
    assert_eq!(t.diff_removed, Color::Red);
    assert_eq!(t.diff_no_changes, Color::DarkGray);
}

#[test]
fn theme_default_tool_call_colors() {
    let t = Theme::default();
    assert_eq!(t.tool_running, Color::Yellow);
    assert_eq!(t.tool_success, Color::Green);
    assert_eq!(t.tool_error, Color::Red);
}

// ---------------------------------------------------------------------------
// Built-in alternate theme
// ---------------------------------------------------------------------------

#[test]
fn theme_monokai_exists() {
    let theme = Theme::monokai();
    assert_eq!(theme.name, "monokai");
}

#[test]
fn theme_monokai_is_distinct_from_default() {
    let default = Theme::default();
    let monokai = Theme::monokai();
    // At least several colors should differ
    let diffs = [
        default.role_user != monokai.role_user,
        default.status_bg != monokai.status_bg,
        default.diff_added != monokai.diff_added,
        default.heading_h1 != monokai.heading_h1,
        default.editor_title != monokai.editor_title,
    ];
    assert!(
        diffs.iter().filter(|&&d| d).count() >= 3,
        "monokai should differ from default in at least 3 color fields"
    );
}

// ---------------------------------------------------------------------------
// Theme resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_theme_default() {
    let theme = opi_tui::resolve_theme("default");
    assert_eq!(theme.name, "default");
}

#[test]
fn resolve_theme_monokai() {
    let theme = opi_tui::resolve_theme("monokai");
    assert_eq!(theme.name, "monokai");
}

#[test]
fn resolve_theme_unknown_returns_default() {
    let theme = opi_tui::resolve_theme("nonexistent");
    assert_eq!(theme.name, "default");
}

// ---------------------------------------------------------------------------
// Shell theme builder
// ---------------------------------------------------------------------------

#[test]
fn shell_accepts_theme() {
    let theme = Theme::monokai();
    let shell = Shell::new("test-model".into()).theme(theme);
    // Should not panic — verifies the builder compiles and renders
    let output = render(shell, 80, 24);
    assert!(!output.is_empty());
}

// ---------------------------------------------------------------------------
// Snapshot tests at 80x24 and 120x40
// ---------------------------------------------------------------------------

#[test]
fn theme_default_shell_80x24() {
    let theme = Theme::default();
    let shell = Shell::new("test-model".into())
        .theme(theme)
        .state(AppStatus::Idle)
        .messages(vec![
            Message::new(Role::User, "Hello!"),
            Message::new(Role::Assistant, "Hi there!"),
        ]);
    insta::assert_snapshot!("theme_default_shell_80x24", render(shell, 80, 24));
}

#[test]
fn theme_default_shell_120x40() {
    let theme = Theme::default();
    let shell = Shell::new("test-model".into())
        .theme(theme)
        .state(AppStatus::Streaming)
        .token_count(420)
        .messages(vec![
            Message::new(Role::User, "What is the capital of France?"),
            Message::new(Role::Assistant, "The capital of France is Paris."),
        ]);
    insta::assert_snapshot!("theme_default_shell_120x40", render(shell, 120, 40));
}

#[test]
fn theme_monokai_shell_80x24() {
    let theme = Theme::monokai();
    let shell = Shell::new("test-model".into())
        .theme(theme)
        .state(AppStatus::Thinking)
        .messages(vec![
            Message::new(Role::User, "Hello!"),
            Message::new(Role::Assistant, "Hi there!"),
        ]);
    insta::assert_snapshot!("theme_monokai_shell_80x24", render(shell, 80, 24));
}

#[test]
fn theme_monokai_shell_120x40() {
    let theme = Theme::monokai();
    let shell = Shell::new("test-model".into())
        .theme(theme)
        .state(AppStatus::Idle)
        .token_count(99)
        .messages(vec![
            Message::new(Role::User, "Explain recursion."),
            Message::new(
                Role::Assistant,
                "Recursion is when a function calls itself.",
            ),
        ])
        .input_text("Give me an example.".into());
    insta::assert_snapshot!("theme_monokai_shell_120x40", render(shell, 120, 40));
}

// ---------------------------------------------------------------------------
// Color assertion tests — verify theme colors reach the buffer
// ---------------------------------------------------------------------------

fn render_buf<W: Widget>(widget: W, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| f.render_widget(widget, f.area()))
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn status_bar_applies_default_theme_colors() {
    let bar = StatusBar::new("model".into(), AppStatus::Idle, None);
    let buf = render_buf(bar, 80, 1);
    // Status bar background should be DarkGray (default theme)
    assert_eq!(buf.cell((0, 0)).unwrap().bg, Color::DarkGray);
}

#[test]
fn status_bar_applies_monokai_theme_colors() {
    let monokai = Theme::monokai();
    let bar = StatusBar::new("model".into(), AppStatus::Idle, None).theme(monokai);
    let buf = render_buf(bar, 80, 1);
    // Status bar background should be the monokai dark bg color
    assert_eq!(buf.cell((0, 0)).unwrap().bg, Color::Rgb(39, 40, 34));
}

#[test]
fn message_list_applies_theme_role_colors() {
    let theme = Theme::monokai();
    let messages = vec![
        Message::new(Role::User, "hello"),
        Message::new(Role::Assistant, "hi"),
    ];
    let widget = opi_tui::MessageList::new(messages).theme(theme.clone());
    let buf = render_buf(widget, 80, 24);
    // Find "You:" label — should be monokai green
    let user_fg = buf.cell((1, 1)).unwrap().fg;
    assert_eq!(user_fg, theme.role_user);
}

#[test]
fn shell_default_vs_monokai_produces_different_colors() {
    let default_theme = Theme::default();
    let monokai_theme = Theme::monokai();

    let shell_default = Shell::new("m".into())
        .theme(default_theme.clone())
        .state(AppStatus::Idle);
    let shell_monokai = Shell::new("m".into())
        .theme(monokai_theme.clone())
        .state(AppStatus::Idle);

    let buf_default = render_buf(shell_default, 80, 24);
    let buf_monokai = render_buf(shell_monokai, 80, 24);

    // Status bar row (y=20 in 80x24 shell layout) should have different bg
    let default_bg = buf_default.cell((0, 20)).unwrap().bg;
    let monokai_bg = buf_monokai.cell((0, 20)).unwrap().bg;
    assert_ne!(
        default_bg, monokai_bg,
        "default and monokai themes should produce different status bar colors"
    );
}
