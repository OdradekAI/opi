//! Snapshot tests for the PermissionPrompt widget (Phase 16 task 16.10).
//!
//! DoD: "The permission prompt and status presentation have deterministic 80x24
//! and 120x40 snapshots that require explicit human review before acceptance."
//!
//! Snapshots are NEW pending files until a human reviews them (red flag #4).
//! The summary rendered here is redaction-safe by construction (adapter id +
//! package name + run-mode label only); the over-long-id test pins the
//! deterministic truncation/clipping policy so a package-derived id cannot make
//! the snapshot brittle.

use opi_tui::{PermissionPrompt, PermissionSummary};
use ratatui::{Terminal, backend::TestBackend, widgets::Widget};

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

fn external_summary() -> PermissionSummary {
    PermissionSummary {
        adapter_id: "opi-sandbox".to_string(),
        package_name: "mock-pkg".to_string(),
        run_mode_label: "interactive".to_string(),
    }
}

#[test]
fn permission_prompt_external_80x24() {
    let p = PermissionPrompt::new(external_summary());
    insta::assert_snapshot!("permission_prompt_external_80x24", render(p, 80, 24));
}

#[test]
fn permission_prompt_external_120x40() {
    let p = PermissionPrompt::new(external_summary());
    insta::assert_snapshot!("permission_prompt_external_120x40", render(p, 120, 40));
}

#[test]
fn permission_prompt_local_no_package_80x24() {
    // `local` has no package name; the package context line is omitted.
    let p = PermissionPrompt::new(PermissionSummary {
        adapter_id: "local".to_string(),
        package_name: String::new(),
        run_mode_label: "interactive".to_string(),
    });
    insta::assert_snapshot!(
        "permission_prompt_local_no_package_80x24",
        render(p, 80, 24)
    );
}

#[test]
fn permission_prompt_local_no_package_120x40() {
    // `local` has no package name; the package context line is omitted.
    let p = PermissionPrompt::new(PermissionSummary {
        adapter_id: "local".to_string(),
        package_name: String::new(),
        run_mode_label: "interactive".to_string(),
    });
    insta::assert_snapshot!(
        "permission_prompt_local_no_package_120x40",
        render(p, 120, 40)
    );
}

/// Determinism guard (audit flag): an over-long adapter id must not panic, wrap,
/// or overflow the fixed buffer — ratatui clips to the area, so the rendered
/// buffer is exactly 80 cols and the title is truncated to the border width.
#[test]
fn permission_prompt_overlong_adapter_id_is_clipped_to_80x24() {
    let p = PermissionPrompt::new(PermissionSummary {
        adapter_id: "x".repeat(200),
        package_name: "mock-pkg".to_string(),
        run_mode_label: "interactive".to_string(),
    });
    let text = render(p, 80, 24);
    // Every rendered line fits within 80 columns (no wrap-induced overflow).
    for line in text.lines() {
        assert!(
            line.chars().count() <= 80,
            "line overflowed 80 cols: {:?}",
            line
        );
    }
    // All three choices remain visible (clipping the title did not drop them).
    for choice in opi_tui::PermissionChoice::all() {
        assert!(text.contains(choice.label()), "choice dropped: {text}");
    }
}
