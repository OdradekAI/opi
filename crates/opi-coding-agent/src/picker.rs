//! Picker integration: bridges provider registry and session listing to
//! SelectItem for the SelectList widget (task 3.11).

use std::path::Path;

use opi_agent::session_branch::{BranchInfo, SessionTree};
use opi_agent::{RedactionMode, redact_text};
use opi_tui::select_list::SelectItem;

/// Collect SelectItem entries from all registered providers' model lists.
///
/// Each entry's `id` is the fully-qualified `provider:model` spec, `display`
/// is the model's display name, and `metadata` is the provider id.
pub fn model_picker_items(registry: &opi_ai::registry::ProviderRegistry) -> Vec<SelectItem> {
    registry
        .all_models()
        .into_iter()
        .map(|(provider_id, model)| SelectItem {
            id: format!("{provider_id}:{}", model.id),
            display: model.display_name.clone(),
            metadata: provider_id.to_string(),
        })
        .collect()
}

/// Collect SelectItem entries from a reconstructed session branch tree.
pub fn branch_picker_items(tree: &SessionTree) -> Vec<SelectItem> {
    let active_index = tree.active_branch_index();
    tree.branches()
        .iter()
        .enumerate()
        .map(|(index, branch)| branch_picker_item(branch, index, active_index == Some(index)))
        .collect()
}

fn branch_picker_item(branch: &BranchInfo, index: usize, is_active: bool) -> SelectItem {
    let name = if index == 0 && branch.fork_point.is_none() {
        "Trunk".to_owned()
    } else {
        format!("Branch {}", index + 1)
    };
    let display = match branch.summary.as_deref() {
        Some(summary) if !summary.is_empty() => {
            format!("{name}: {}", redact_text(summary, RedactionMode::Summary))
        }
        _ => name,
    };
    let mut metadata = format!(
        "{} entries, depth {}, tip {}",
        branch.entry_count, branch.depth, branch.tip_id
    );
    if is_active {
        metadata.push_str(", active");
    }
    SelectItem {
        id: branch.tip_id.clone(),
        display,
        metadata,
    }
}

/// Collect SelectItem entries from session listing in the given directory.
///
/// Each entry's `id` is the session id, `display` is the cwd (truncated if
/// needed) prefixed by the typed session name when one is set, and `metadata`
/// is the timestamp followed by the active label set in `[l1, l2]` form when
/// labels exist. Name and labels are UI-visible metadata that never enter
/// provider context (Phase 13.6); surfacing them here keeps the session picker
/// consistent with RPC `session_info` and `--list-sessions --json`.
pub fn session_picker_items(dir: &Path) -> Result<Vec<SelectItem>, std::io::Error> {
    let sessions = crate::session_cli::list_sessions(dir).unwrap_or_default();
    Ok(sessions
        .into_iter()
        .map(|s| {
            let cwd_short = if s.cwd.len() > 40 {
                let start = s.cwd.floor_char_boundary(s.cwd.len() - 37);
                format!("...{}", &s.cwd[start..])
            } else {
                s.cwd
            };
            // Phase 13.6: prefix the typed name and append the label set so
            // the picker previews the same metadata the handoff surfaces
            // expose. Empty/missing name falls back to the cwd-only display.
            let display = match s.name.as_deref() {
                Some(name) if !name.is_empty() => format!("{name} - {cwd_short}"),
                _ => cwd_short,
            };
            let mut metadata = s.timestamp.clone();
            if !s.labels.is_empty() {
                metadata.push_str(" [");
                metadata.push_str(&s.labels.join(", "));
                metadata.push(']');
            }
            SelectItem {
                id: s.id,
                display,
                metadata,
            }
        })
        .collect())
}
