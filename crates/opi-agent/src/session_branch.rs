//! Session branch reconstruction (task 4.9).
//!
//! Reconstructs the branch tree from session JSONL entries without modifying
//! the append-only session file. A [`SessionTree`] is built from
//! [`SessionEntry`] data using the `parent_id` graph and `Leaf` pointer
//! entries.

use std::collections::{HashMap, HashSet};

use crate::session::SessionEntry;

/// Metadata for a single branch within a session tree.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// The entry ID at the tip (leaf-most entry) of this branch.
    pub tip_id: String,
    /// Number of edges from the root entry to the tip.
    pub depth: usize,
    /// Number of Message/Compaction entries on this branch.
    pub entry_count: usize,
    /// Timestamp of the tip entry.
    pub timestamp: String,
    /// Short summary from the last message on the branch (first 80 chars).
    pub summary: Option<String>,
    /// Entry ID where this branch forks off from the trunk.
    /// `None` for the trunk (first/root branch).
    pub fork_point: Option<String>,
}

/// A reconstructed session tree with branch metadata.
///
/// Built from session entries via [`SessionTree::from_entries`]. The tree
/// identifies distinct branches in the conversation by detecting nodes with
/// multiple children in the `parent_id` graph. The "trunk" is the first
/// branch (root to the first fork point or to the tip if no fork exists).
/// Each child branch starts from a fork point and extends to its own tip.
///
/// The **active branch** is determined by the last `Leaf` entry's
/// `entry_id`. Without leaves, the active branch is the trunk.
#[derive(Debug, Clone)]
pub struct SessionTree {
    branches: Vec<BranchInfo>,
    leaf_tip: Option<String>,
    /// Map from entry ID to entry metadata (for branch containment lookups).
    entries_by_id: HashMap<String, EntryMeta>,
}

/// Lightweight metadata extracted from a session entry.
#[derive(Debug, Clone)]
struct EntryMeta {
    parent_id: Option<String>,
    timestamp: String,
    summary: Option<String>,
}

impl SessionTree {
    /// Reconstruct the session tree from a slice of session entries.
    ///
    /// This is a read-only operation; it does not modify session storage.
    pub fn from_entries(entries: &[SessionEntry]) -> Self {
        let mut meta_by_id: HashMap<String, EntryMeta> = HashMap::new();
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        let mut last_leaf_tip: Option<String> = None;

        for entry in entries {
            match entry {
                SessionEntry::Message(m) => {
                    let summary = extract_message_summary(&m.message);
                    let meta = EntryMeta {
                        parent_id: m.parent_id.clone(),
                        timestamp: m.timestamp.clone(),
                        summary,
                    };
                    if let Some(ref pid) = m.parent_id {
                        children.entry(pid.clone()).or_default().push(m.id.clone());
                    }
                    meta_by_id.insert(m.id.clone(), meta);
                }
                SessionEntry::Compaction(c) => {
                    let meta = EntryMeta {
                        parent_id: c.parent_id.clone(),
                        timestamp: c.timestamp.clone(),
                        summary: Some(c.summary.clone()),
                    };
                    if let Some(ref pid) = c.parent_id {
                        children.entry(pid.clone()).or_default().push(c.id.clone());
                    }
                    meta_by_id.insert(c.id.clone(), meta);
                }
                SessionEntry::Leaf(l) => {
                    last_leaf_tip = Some(l.entry_id.clone());
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
        for child_ids in children.values_mut() {
            child_ids.sort();
        }

        // Find roots: entries whose parent is None or whose parent doesn't
        // exist in the graph. These are the starting points for branch walks.
        let all_ids: HashSet<&str> = meta_by_id.keys().map(|s| s.as_str()).collect();
        let has_valid_parent: HashSet<&str> = meta_by_id
            .iter()
            .filter(|(_, meta)| {
                meta.parent_id
                    .as_deref()
                    .is_some_and(|pid| all_ids.contains(pid))
            })
            .map(|(id, _)| id.as_str())
            .collect();

        let mut roots: Vec<&str> = all_ids
            .iter()
            .filter(|id| !has_valid_parent.contains(*id))
            .copied()
            .collect();
        roots.sort_unstable();

        // Walk from each root to discover branches.
        let mut branches: Vec<BranchInfo> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();

        for &root_id in &roots {
            discover_branches(
                root_id,
                None,
                0,
                &meta_by_id,
                &children,
                &mut branches,
                &mut visited,
            );
        }

        // Fallback: if no roots found but we have orphaned entries.
        if roots.is_empty() && !meta_by_id.is_empty() {
            let mut ids = meta_by_id.keys().collect::<Vec<_>>();
            ids.sort();
            for id in ids {
                let meta = meta_by_id.get(id).expect("sorted id exists");
                if visited.insert(id.clone()) {
                    branches.push(BranchInfo {
                        tip_id: id.clone(),
                        depth: 0,
                        entry_count: 1,
                        timestamp: meta.timestamp.clone(),
                        summary: meta.summary.clone(),
                        fork_point: None,
                    });
                }
            }
        }

        Self {
            branches,
            leaf_tip: last_leaf_tip,
            entries_by_id: meta_by_id,
        }
    }

    /// Return the list of discovered branches.
    pub fn branches(&self) -> &[BranchInfo] {
        &self.branches
    }

    /// Return the entry ID at the tip of the active branch.
    ///
    /// Determined by the last `Leaf` entry's `entry_id`. If no leaf entries
    /// exist (legacy linear sessions), falls back to the trunk branch tip.
    /// Returns `None` only when the session has no content entries.
    pub fn active_tip(&self) -> Option<&str> {
        // If a leaf tip exists and is a valid entry, use it.
        if let Some(ref tip) = self.leaf_tip
            && self.entries_by_id.contains_key(tip.as_str())
        {
            return Some(tip.as_str());
        }
        // Fall back to the trunk (first branch) tip.
        self.branches.first().map(|b| b.tip_id.as_str())
    }

    /// Return the index of the active branch in the branches list.
    ///
    /// If no leaf entries exist, returns index 0 (trunk).
    pub fn active_branch_index(&self) -> Option<usize> {
        match &self.leaf_tip {
            Some(tip) if self.entries_by_id.contains_key(tip.as_str()) => self
                .branches
                .iter()
                .position(|b| b.tip_id == *tip)
                .or_else(|| {
                    self.branches
                        .iter()
                        .position(|b| self.tip_is_on_branch(tip, &b.tip_id))
                }),
            _ => {
                if self.branches.is_empty() {
                    None
                } else {
                    Some(0)
                }
            }
        }
    }

    /// Return the branch at the given index, or `None` if out of bounds.
    pub fn branch_at(&self, index: usize) -> Option<&BranchInfo> {
        self.branches.get(index)
    }

    /// Check whether `candidate_id` is on the branch ending at `tip_id`.
    fn tip_is_on_branch(&self, candidate_id: &str, tip_id: &str) -> bool {
        let mut visited = HashSet::new();
        let mut cursor = Some(tip_id);
        while let Some(id) = cursor {
            if id == candidate_id {
                return true;
            }
            if !visited.insert(id) {
                break;
            }
            cursor = self
                .entries_by_id
                .get(id)
                .and_then(|m| m.parent_id.as_deref());
        }
        false
    }
}

/// Walk from `start_id` to its tip, recursing at fork points.
fn discover_branches(
    start_id: &str,
    fork_point: Option<&str>,
    base_depth: usize,
    meta_by_id: &HashMap<String, EntryMeta>,
    children: &HashMap<String, Vec<String>>,
    branches: &mut Vec<BranchInfo>,
    visited: &mut HashSet<String>,
) {
    let mut cursor = start_id.to_owned();
    let mut depth = base_depth;
    let mut entry_count = 0;

    loop {
        entry_count += 1;

        let child_count = children.get(&cursor).map(|v| v.len()).unwrap_or(0);

        if child_count == 0 {
            // Tip of this branch.
            if visited.insert(cursor.clone()) {
                let m = meta_by_id.get(&cursor);
                branches.push(BranchInfo {
                    tip_id: cursor,
                    depth,
                    entry_count,
                    timestamp: m.map(|m| m.timestamp.clone()).unwrap_or_default(),
                    summary: m.and_then(|m| m.summary.clone()),
                    fork_point: fork_point.map(|s| s.to_owned()),
                });
            }
            return;
        }

        if child_count == 1 {
            // Single child — continue along the chain.
            depth += 1;
            cursor = children.get(&cursor).unwrap()[0].clone();
        } else {
            // Fork point — record trunk up to here, recurse into children.
            if visited.insert(cursor.clone()) {
                let m = meta_by_id.get(&cursor);
                branches.push(BranchInfo {
                    tip_id: cursor.clone(),
                    depth,
                    entry_count,
                    timestamp: m.map(|m| m.timestamp.clone()).unwrap_or_default(),
                    summary: m.and_then(|m| m.summary.clone()),
                    fork_point: fork_point.map(|s| s.to_owned()),
                });
            }
            for child in children.get(&cursor).unwrap() {
                discover_branches(
                    child,
                    Some(&cursor),
                    depth + 1,
                    meta_by_id,
                    children,
                    branches,
                    visited,
                );
            }
            return;
        }
    }
}

/// Extract a short summary from a message (first 80 chars of text content).
fn extract_message_summary(message: &opi_ai::message::Message) -> Option<String> {
    use opi_ai::message::{AssistantContent, InputContent, Message};

    match message {
        Message::User(u) => u.content.iter().find_map(|c| match c {
            InputContent::Text { text } => {
                let summary = if text.len() > 80 {
                    format!("{}...", &text[..text.floor_char_boundary(80)])
                } else {
                    text.clone()
                };
                Some(summary)
            }
            _ => None,
        }),
        Message::Assistant(a) => a.content.iter().find_map(|c| match c {
            AssistantContent::Text { text, .. } => {
                let summary = if text.len() > 80 {
                    format!("{}...", &text[..text.floor_char_boundary(80)])
                } else {
                    text.clone()
                };
                Some(summary)
            }
            _ => None,
        }),
        _ => None,
    }
}
