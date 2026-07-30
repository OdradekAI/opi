//! Interactive project-trust prompt widget + types (Phase 15 task 15.8.2 / T6).
//!
//! [`TrustChoice`] is the five-way user choice mirroring pi; [`TrustPrompt`] is
//! the ratatui selection widget rendered while [`crate::AppState`] is
//! [`crate::AppState::AwaitingTrust`]; [`AwaitingTrustState`] is the payload
//! carried by that variant (the project path + the oneshot responder).

use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tokio::sync::oneshot;

/// The user's project-trust choice (Phase 15 task 15.8.2 / T6).
///
/// Mirrors pi's five-way prompt. `Trust`/`TrustParent` are durable allow
/// decisions (persisted to `trust.json`); `Deny` is a durable block; the
/// `*Session` variants are session-only and never persisted. Translation to a
/// [`crate::AppStatus`]/`TrustDecision` + persistence is owned by
/// `opi-coding-agent` (see `project_trust::apply_ui_choice`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustChoice {
    /// Persist a durable allow for the current project directory.
    Trust,
    /// Persist a durable allow for the project's parent directory.
    TrustParent,
    /// Allow for this session only (not persisted).
    TrustSession,
    /// Persist a durable deny for the current project directory.
    Deny,
    /// Deny for this session only (not persisted).
    DenySession,
}

const ALL_TRUST_CHOICES: [TrustChoice; 5] = [
    TrustChoice::Trust,
    TrustChoice::TrustParent,
    TrustChoice::TrustSession,
    TrustChoice::Deny,
    TrustChoice::DenySession,
];

const ROOT_TRUST_CHOICES: [TrustChoice; 4] = [
    TrustChoice::Trust,
    TrustChoice::TrustSession,
    TrustChoice::Deny,
    TrustChoice::DenySession,
];

impl TrustChoice {
    /// The five choices in stable render/navigation order.
    pub fn all() -> [TrustChoice; 5] {
        ALL_TRUST_CHOICES
    }

    /// Human-readable label rendered in the prompt widget.
    pub fn label(self) -> &'static str {
        match self {
            TrustChoice::Trust => "Trust",
            TrustChoice::TrustParent => "Trust parent",
            TrustChoice::TrustSession => "Trust for session",
            TrustChoice::Deny => "Deny",
            TrustChoice::DenySession => "Deny for session",
        }
    }

    /// The choice at `all()` position `index`, or `None` if out of range.
    pub fn from_index(index: usize) -> Option<TrustChoice> {
        TrustChoice::all().get(index).copied()
    }
}

/// Payload carried by [`crate::AppState::AwaitingTrust`]: the project being
/// asked about and the oneshot responder that receives the user's choice.
///
/// The interactive trust-prompt phase constructs this, renders the
/// [`TrustPrompt`] widget, and on exactly one key press sends the selected
/// [`TrustChoice`] through `response_tx`.
pub struct AwaitingTrustState {
    /// The project root being asked about.
    pub project_path: PathBuf,
    /// One-shot channel that receives the user's choice.
    pub response_tx: oneshot::Sender<TrustChoice>,
}

impl std::fmt::Debug for AwaitingTrustState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwaitingTrustState")
            .field("project_path", &self.project_path)
            .field("response_tx", &"<oneshot::Sender>")
            .finish()
    }
}

/// Ratatui selection widget over the available [`TrustChoice`] options.
///
/// The cursor starts on [`TrustChoice::Trust`]; `move_next`/`move_prev` advance
/// and clamp at the ends (no wrap). The selected row is prefixed with `> `.
pub struct TrustPrompt {
    cursor: usize,
    choices: &'static [TrustChoice],
}

impl Clone for TrustPrompt {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor,
            choices: self.choices,
        }
    }
}

impl TrustPrompt {
    /// Create the ordinary five-choice prompt.
    pub fn new() -> Self {
        Self {
            cursor: 0,
            choices: &ALL_TRUST_CHOICES,
        }
    }

    /// Create a prompt for a filesystem root, where no parent can be trusted.
    pub fn without_parent() -> Self {
        Self {
            cursor: 0,
            choices: &ROOT_TRUST_CHOICES,
        }
    }

    /// Return the available choice at `index`.
    pub fn choice_at(&self, index: usize) -> Option<TrustChoice> {
        self.choices.get(index).copied()
    }

    /// The currently selected choice.
    pub fn selected(&self) -> TrustChoice {
        self.choice_at(self.cursor).unwrap_or(TrustChoice::Trust)
    }

    /// Advance the cursor to the next choice; clamp at the last.
    pub fn move_next(&mut self) {
        if self.cursor < self.choices.len() - 1 {
            self.cursor += 1;
        }
    }

    /// Move the cursor to the previous choice; clamp at the first.
    pub fn move_prev(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
}

impl Default for TrustPrompt {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for TrustPrompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            " Trust this project? ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let inner = block.inner(area);
        block.render(area, buf);

        let selected_style = Style::default().add_modifier(Modifier::BOLD);
        let lines = self.choices.iter().copied().enumerate().map(|(i, choice)| {
            if i == self.cursor {
                Line::from(vec![
                    Span::styled("> ", selected_style),
                    Span::styled(choice.label(), selected_style),
                ])
            } else {
                Line::from(format!("  {}", choice.label()))
            }
        });
        Paragraph::new(lines.collect::<Vec<_>>()).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, AppStatus};
    use ratatui::{Terminal, backend::TestBackend, widgets::Widget};
    use std::path::PathBuf;

    /// Render a widget to a fixed buffer and return its trimmed text.
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

    #[test]
    fn trust_choice_has_five_variants_in_stable_order() {
        assert_eq!(
            TrustChoice::all(),
            [
                TrustChoice::Trust,
                TrustChoice::TrustParent,
                TrustChoice::TrustSession,
                TrustChoice::Deny,
                TrustChoice::DenySession,
            ]
        );
    }

    #[test]
    fn trust_choice_labels_are_human_readable() {
        assert_eq!(TrustChoice::Trust.label(), "Trust");
        assert_eq!(TrustChoice::TrustParent.label(), "Trust parent");
        assert_eq!(TrustChoice::TrustSession.label(), "Trust for session");
        assert_eq!(TrustChoice::Deny.label(), "Deny");
        assert_eq!(TrustChoice::DenySession.label(), "Deny for session");
    }

    #[test]
    fn trust_choice_from_index_round_trips_all() {
        for (i, choice) in TrustChoice::all().iter().enumerate() {
            assert_eq!(TrustChoice::from_index(i), Some(*choice));
        }
        assert_eq!(TrustChoice::from_index(TrustChoice::all().len()), None);
    }

    #[test]
    fn trust_prompt_starts_on_trust() {
        assert_eq!(TrustPrompt::new().selected(), TrustChoice::Trust);
    }

    #[test]
    fn trust_prompt_cursor_next_advances_and_clamps_at_last() {
        let mut p = TrustPrompt::new();
        p.move_next();
        assert_eq!(p.selected(), TrustChoice::TrustParent);
        // Overadvance clamps at the last choice (DenySession), no wrap.
        for _ in 0..TrustChoice::all().len() + 3 {
            p.move_next();
        }
        assert_eq!(p.selected(), TrustChoice::DenySession);
    }

    #[test]
    fn trust_prompt_cursor_prev_clamps_at_trust() {
        let mut p = TrustPrompt::new();
        p.move_prev();
        assert_eq!(p.selected(), TrustChoice::Trust);
    }

    #[test]
    fn trust_prompt_render_shows_title_all_choices_and_initial_highlight() {
        let p = TrustPrompt::new();
        let text = render(p, 48, 9);
        assert!(
            text.contains("Trust this project?"),
            "missing title: {text}"
        );
        // Every choice label appears.
        for choice in TrustChoice::all() {
            assert!(text.contains(choice.label()), "missing label: {text}");
        }
        // Exactly one selected marker (`>`) is rendered, on the Trust choice.
        // (The title "Trust this project?" also contains "Trust", so key on the
        // unique marker line, and disambiguate the bare Trust row from the
        // longer Trust-parent / Trust-session rows.)
        let marker_lines: Vec<&str> = text.lines().filter(|l| l.contains('>')).collect();
        assert_eq!(marker_lines.len(), 1, "exactly one marker: {text}");
        let marker = marker_lines[0];
        assert!(
            marker.contains("> Trust"),
            "marker on a Trust row: {marker}"
        );
        assert!(
            !marker.contains("parent") && !marker.contains("session"),
            "marker should be on bare Trust, not a parent/session row: {marker}"
        );
    }

    #[test]
    fn trust_prompt_render_moves_highlight_with_cursor() {
        let mut p = TrustPrompt::new();
        p.move_next();
        p.move_next(); // now on TrustSession
        let text = render(p, 48, 9);
        let marker_lines: Vec<&str> = text.lines().filter(|l| l.contains('>')).collect();
        assert_eq!(marker_lines.len(), 1, "exactly one marker: {text}");
        assert!(
            marker_lines[0].contains("> Trust for session"),
            "marker should follow cursor to TrustSession: {}",
            marker_lines[0]
        );
    }

    #[test]
    fn trust_prompt_without_parent_omits_parent_and_reindexes_choices() {
        let mut prompt = TrustPrompt::without_parent();
        assert_eq!(prompt.selected(), TrustChoice::Trust);
        assert_eq!(prompt.choice_at(1), Some(TrustChoice::TrustSession));

        prompt.move_next();
        assert_eq!(prompt.selected(), TrustChoice::TrustSession);

        let text = render(prompt, 48, 8);
        assert!(
            !text.contains("Trust parent"),
            "parent choice leaked: {text}"
        );
        assert!(
            text.contains("Trust for session"),
            "session choice missing: {text}"
        );
    }

    #[test]
    fn trust_prompt_without_parent_clamps_at_fourth_choice() {
        let mut prompt = TrustPrompt::without_parent();
        for _ in 0..TrustChoice::all().len() + 3 {
            prompt.move_next();
        }
        assert_eq!(prompt.selected(), TrustChoice::DenySession);
        assert_eq!(prompt.choice_at(4), None);
    }

    #[test]
    fn awaiting_trust_state_holds_project_path_and_sender() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let s = AwaitingTrustState {
            project_path: PathBuf::from("/proj"),
            response_tx: tx,
        };
        assert_eq!(s.project_path, PathBuf::from("/proj"));
    }

    #[test]
    fn appstate_awaiting_trust_projects_to_awaiting_status() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let st = AppState::AwaitingTrust(crate::AwaitingTrustState {
            project_path: PathBuf::from("/proj"),
            response_tx: tx,
        });
        assert_eq!(st.status(), AppStatus::AwaitingTrust);
    }

    #[test]
    fn appstatus_display_labels_are_stable() {
        assert_eq!(AppStatus::Idle.to_string(), "idle");
        assert_eq!(AppStatus::Thinking.to_string(), "thinking...");
        assert_eq!(AppStatus::Streaming.to_string(), "streaming...");
        assert_eq!(AppStatus::ToolExecuting.to_string(), "executing tool...");
        assert_eq!(AppStatus::AwaitingTrust.to_string(), "awaiting trust...");
    }
}
