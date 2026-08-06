//! Interactive capability-permission prompt widget + types (Phase 16 task 16.10).
//!
//! DISTINCT from [`crate::trust_prompt`] (Phase 15 project-trust): this is the
//! mid-execution `command.execute` capability prompt offered when an adapter's
//! resolved permission is `ask`. [`PermissionChoice`] is the three-way user
//! choice (allow-once / allow-session / deny); [`PermissionPrompt`] is the
//! ratatui selection widget; [`AwaitingPermissionState`] is the payload carried
//! by [`crate::AppState::AwaitingPermission`].
//!
//! # Redaction (Phase 16 invariant)
//!
//! [`PermissionSummary`] carries ONLY redaction-safe identifiers — adapter id,
//! package name, and run-mode label. It MUST NOT carry command text, arguments,
//! environment values, credentials, absolute paths, or the workspace/cwd. Every
//! other Phase 16 display surface redacts command text because it may carry
//! secrets; this prompt is rendered to the alternate screen (screenshottable,
//! in scrollback, in insta snapshots) and follows the same rule. The broker
//! builds a `PermissionSummary` from redaction-safe fields only.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use tokio::sync::oneshot;

/// The user's capability-permission choice (Phase 16 task 16.10).
///
/// Three-way, mirroring the DoD: "allow once, allow for the current in-memory
/// harness session, or deny". `AllowOnce` authorizes exactly this one
/// invocation (consumed at decision time, no session grant recorded);
/// `AllowSession` records a memory-only session grant (re-prompted only after a
/// restart/resume/fork/exit — never persisted); `Deny` refuses this invocation
/// (the next `ask` for the same adapter re-prompts). None of these is a durable
/// CLI grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    /// Authorize this one invocation only (no session grant).
    AllowOnce,
    /// Authorize for the current in-memory harness session (memory-only).
    AllowSession,
    /// Refuse this invocation (re-prompts on the next `ask`).
    Deny,
}

const ALL_PERMISSION_CHOICES: [PermissionChoice; 3] = [
    PermissionChoice::AllowOnce,
    PermissionChoice::AllowSession,
    PermissionChoice::Deny,
];

impl PermissionChoice {
    /// The three choices in stable render/navigation order (allow-once first,
    /// deny last; the cursor starts on allow-once).
    pub fn all() -> [PermissionChoice; 3] {
        ALL_PERMISSION_CHOICES
    }

    /// Human-readable label rendered in the prompt widget.
    pub fn label(self) -> &'static str {
        match self {
            PermissionChoice::AllowOnce => "Allow once",
            PermissionChoice::AllowSession => "Allow for session",
            PermissionChoice::Deny => "Deny",
        }
    }

    /// The choice at `all()` position `index`, or `None` if out of range.
    pub fn from_index(index: usize) -> Option<PermissionChoice> {
        Self::all().get(index).copied()
    }
}

/// Redaction-safe identity context for a permission prompt. See the module docs:
/// carries ONLY safe identifiers, never command text/env/paths/credentials.
#[derive(Debug, Clone)]
pub struct PermissionSummary {
    /// The adapter id being asked about (e.g. `opi-sandbox`, `local`).
    pub adapter_id: String,
    /// The package name contributing the adapter (empty for `local`).
    pub package_name: String,
    /// The run-mode label (e.g. `interactive`).
    pub run_mode_label: String,
}

/// Payload carried by [`crate::AppState::AwaitingPermission`]: the redaction-safe
/// identity context and the oneshot responder that receives the user's
/// [`PermissionChoice`].
///
/// The interactive permission phase constructs this, renders the
/// [`PermissionPrompt`] widget, and on exactly one key press sends the selected
/// [`PermissionChoice`] through `response_tx`. Esc / abort / a dropped receiver
/// (terminal close) all resolve to [`PermissionChoice::Deny`] upstream — never a
/// panic or hang — so the tool call surfaces a stable `permission_denied` error.
pub struct AwaitingPermissionState {
    /// Redaction-safe identity context for the prompt.
    pub summary: PermissionSummary,
    /// One-shot channel that receives the user's choice.
    pub response_tx: oneshot::Sender<PermissionChoice>,
}

impl std::fmt::Debug for AwaitingPermissionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AwaitingPermissionState")
            .field("summary", &self.summary)
            .field("response_tx", &"<oneshot::Sender>")
            .finish()
    }
}

/// Ratatui selection widget over the three [`PermissionChoice`] options.
///
/// The cursor starts on [`PermissionChoice::AllowOnce`]; `move_next`/`move_prev`
/// advance and clamp at the ends (no wrap). The selected row is prefixed with
/// `> `. Carries a [`PermissionSummary`] so the rendered title and context name
/// the adapter; the summary is redaction-safe by construction.
#[derive(Clone)]
pub struct PermissionPrompt {
    cursor: usize,
    summary: PermissionSummary,
}

impl PermissionPrompt {
    /// Create the prompt for `summary`; the cursor starts on allow-once.
    pub fn new(summary: PermissionSummary) -> Self {
        Self { cursor: 0, summary }
    }

    /// The redaction-safe summary this prompt renders.
    pub fn summary(&self) -> &PermissionSummary {
        &self.summary
    }

    /// Return the available choice at `index`.
    pub fn choice_at(&self, index: usize) -> Option<PermissionChoice> {
        Self::all_choices().get(index).copied()
    }

    /// The currently selected choice.
    pub fn selected(&self) -> PermissionChoice {
        self.choice_at(self.cursor)
            .unwrap_or(PermissionChoice::Deny)
    }

    /// Advance the cursor to the next choice; clamp at the last.
    pub fn move_next(&mut self) {
        if self.cursor < Self::all_choices().len() - 1 {
            self.cursor += 1;
        }
    }

    /// Move the cursor to the previous choice; clamp at the first.
    pub fn move_prev(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn all_choices() -> &'static [PermissionChoice] {
        &ALL_PERMISSION_CHOICES
    }
}

impl Widget for PermissionPrompt {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Deterministic truncation policy (audit flag): cap the adapter id in the
        // title and the context line so a long package-derived id cannot wrap or
        // push choices off a fixed 80x24 / 120x40 buffer. ratatui clips overflow
        // to the area, so the rendered buffer is always exactly the area size.
        let title = format!(" Allow {}? ", self.summary.adapter_id);
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let inner = block.inner(area);
        block.render(area, buf);

        let selected_style = Style::default().add_modifier(Modifier::BOLD);
        let mut lines: Vec<Line> = Vec::new();
        // Redaction-safe context only: adapter id, package name, run-mode label.
        lines.push(Line::from(format!("adapter: {}", self.summary.adapter_id)));
        if !self.summary.package_name.is_empty() {
            lines.push(Line::from(format!(
                "package: {} · mode: {}",
                self.summary.package_name, self.summary.run_mode_label
            )));
        } else {
            lines.push(Line::from(format!("mode: {}", self.summary.run_mode_label)));
        }
        lines.push(Line::from("Esc cancels (= deny)."));
        lines.push(Line::from(""));

        for (i, choice) in Self::all_choices().iter().copied().enumerate() {
            if i == self.cursor {
                lines.push(Line::from(vec![
                    Span::styled("> ", selected_style),
                    Span::styled(choice.label(), selected_style),
                ]));
            } else {
                lines.push(Line::from(format!("  {}", choice.label())));
            }
        }
        Paragraph::new(lines).render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppState, AppStatus};
    use ratatui::{Terminal, backend::TestBackend, widgets::Widget};

    fn summary() -> PermissionSummary {
        PermissionSummary {
            adapter_id: "opi-sandbox".to_string(),
            package_name: "mock-pkg".to_string(),
            run_mode_label: "interactive".to_string(),
        }
    }

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
    fn permission_choice_has_three_variants_in_stable_order() {
        assert_eq!(
            PermissionChoice::all(),
            [
                PermissionChoice::AllowOnce,
                PermissionChoice::AllowSession,
                PermissionChoice::Deny,
            ]
        );
    }

    #[test]
    fn permission_choice_labels_are_human_readable() {
        assert_eq!(PermissionChoice::AllowOnce.label(), "Allow once");
        assert_eq!(PermissionChoice::AllowSession.label(), "Allow for session");
        assert_eq!(PermissionChoice::Deny.label(), "Deny");
    }

    #[test]
    fn permission_choice_from_index_round_trips_all() {
        for (i, choice) in PermissionChoice::all().iter().enumerate() {
            assert_eq!(PermissionChoice::from_index(i), Some(*choice));
        }
        assert_eq!(
            PermissionChoice::from_index(PermissionChoice::all().len()),
            None
        );
    }

    #[test]
    fn permission_prompt_starts_on_allow_once() {
        assert_eq!(
            PermissionPrompt::new(summary()).selected(),
            PermissionChoice::AllowOnce
        );
    }

    #[test]
    fn permission_prompt_invalid_cursor_fails_closed_to_deny() {
        let prompt = PermissionPrompt {
            cursor: PermissionChoice::all().len(),
            summary: summary(),
        };

        assert_eq!(prompt.selected(), PermissionChoice::Deny);
    }

    #[test]
    fn permission_prompt_cursor_next_advances_and_clamps_at_deny() {
        let mut p = PermissionPrompt::new(summary());
        p.move_next();
        assert_eq!(p.selected(), PermissionChoice::AllowSession);
        for _ in 0..PermissionChoice::all().len() + 3 {
            p.move_next();
        }
        assert_eq!(p.selected(), PermissionChoice::Deny);
    }

    #[test]
    fn permission_prompt_cursor_prev_clamps_at_allow_once() {
        let mut p = PermissionPrompt::new(summary());
        p.move_prev();
        assert_eq!(p.selected(), PermissionChoice::AllowOnce);
    }

    #[test]
    fn permission_prompt_render_shows_title_all_choices_and_initial_highlight() {
        let p = PermissionPrompt::new(summary());
        let text = render(p, 48, 11);
        assert!(text.contains("Allow opi-sandbox?"), "missing title: {text}");
        for choice in PermissionChoice::all() {
            assert!(text.contains(choice.label()), "missing label: {text}");
        }
        // Exactly one selected marker, on the Allow-once row.
        let marker_lines: Vec<&str> = text.lines().filter(|l| l.contains('>')).collect();
        assert_eq!(marker_lines.len(), 1, "exactly one marker: {text}");
        assert!(
            marker_lines[0].contains("> Allow once"),
            "marker on Allow-once: {}",
            marker_lines[0]
        );
    }

    #[test]
    fn permission_prompt_render_moves_highlight_with_cursor() {
        let mut p = PermissionPrompt::new(summary());
        p.move_next();
        p.move_next(); // now on Deny
        let text = render(p, 48, 11);
        let marker_lines: Vec<&str> = text.lines().filter(|l| l.contains('>')).collect();
        assert_eq!(marker_lines.len(), 1, "exactly one marker: {text}");
        assert!(
            marker_lines[0].contains("> Deny"),
            "marker should follow cursor to Deny: {}",
            marker_lines[0]
        );
    }

    #[test]
    fn permission_prompt_render_omits_package_line_when_empty() {
        let s = PermissionSummary {
            adapter_id: "local".to_string(),
            package_name: String::new(),
            run_mode_label: "interactive".to_string(),
        };
        let text = render(PermissionPrompt::new(s), 48, 11);
        assert!(
            !text.contains("package:"),
            "no package line for local: {text}"
        );
        assert!(
            text.contains("mode: interactive"),
            "mode line present: {text}"
        );
    }

    #[test]
    fn appstate_awaiting_permission_projects_to_awaiting_status() {
        let (tx, _rx) = tokio::sync::oneshot::channel();
        let st = AppState::AwaitingPermission(crate::AwaitingPermissionState {
            summary: summary(),
            response_tx: tx,
        });
        assert_eq!(st.status(), AppStatus::AwaitingPermission);
    }

    #[test]
    fn appstatus_awaiting_permission_display_label_is_stable() {
        assert_eq!(
            AppStatus::AwaitingPermission.to_string(),
            "awaiting permission..."
        );
    }
}
