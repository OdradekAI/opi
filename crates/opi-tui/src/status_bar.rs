//! Status bar component showing model, state, and token info.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Widget},
};

use crate::{AppStatus, theme::Theme};

/// Single-line status bar.
pub struct StatusBar {
    model: String,
    state: AppStatus,
    token_count: Option<u64>,
    cost_usd: Option<f64>,
    theme: Theme,
}

impl StatusBar {
    pub fn new(model: String, state: AppStatus, token_count: Option<u64>) -> Self {
        Self {
            model,
            state,
            token_count,
            cost_usd: None,
            theme: Theme::default(),
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the total accumulated cost in USD. Surfaced next to the token count.
    pub fn cost_usd(mut self, cost: f64) -> Self {
        self.cost_usd = Some(cost);
        self
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let t = &self.theme;
        Block::default()
            .style(Style::default().bg(t.status_bg))
            .render(area, buf);

        let state_style = match self.state {
            AppStatus::Idle => Style::default().fg(t.status_idle),
            AppStatus::Thinking => Style::default().fg(t.status_thinking),
            AppStatus::Streaming => Style::default().fg(t.status_streaming),
            AppStatus::ToolExecuting => Style::default().fg(t.status_tool),
            AppStatus::AwaitingTrust => Style::default().fg(t.status_idle),
            AppStatus::AwaitingPermission => Style::default().fg(t.status_tool),
        };

        let mut spans = vec![
            Span::styled(
                format!(" {} ", self.model),
                Style::default().fg(t.status_idle),
            ),
            Span::styled(format!("[{}]", self.state), state_style),
        ];

        if let Some(count) = self.token_count {
            spans.push(Span::styled(
                format!(" | {count} tokens"),
                Style::default().fg(t.status_tokens),
            ));
        }

        if let Some(cost) = self.cost_usd {
            spans.push(Span::styled(
                format!(" | ${cost:.4}"),
                Style::default().fg(t.status_tokens),
            ));
        }

        Line::from(spans).render(area, buf);
    }
}
