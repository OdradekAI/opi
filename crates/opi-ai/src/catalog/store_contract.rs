//! Snapshot, acknowledgement and error contracts shared with the document codec.

use super::ModelsStoreEntry;
use std::collections::{BTreeSet, HashMap};

/// The publisher of one acknowledged provider snapshot. Missing metadata
/// denotes an older or caller-owned publication with unknown provenance.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CatalogPublisher {
    Native,
    Source { id: String },
}

/// Typed failure while opening or decoding a [`super::ModelsStore`] file.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ModelsStoreError {
    /// The file exists but is not a valid store document.
    #[error("corrupt models store: {message}")]
    Corrupt {
        /// Redacted, non-secret description of the violated contract.
        message: String,
    },
    /// Reading or writing the store file failed.
    #[error("models store IO failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Typed rejection of a [`super::RefreshGuard::commit`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreCommitError {
    /// A newer revision was committed after this guard was created, so this
    /// write arrived late and was dropped without touching published state.
    #[error("superseded: guard revision {guard_revision} predates store revision {store_revision}")]
    Superseded {
        /// Revision the guard was created at.
        guard_revision: u64,
        /// Revision currently published in the store.
        store_revision: u64,
    },
    /// The refresh was cancelled before committing; nothing was persisted.
    #[error("cancelled before commit")]
    Cancelled,
    /// Persisting the snapshot failed.
    #[error(transparent)]
    Store(#[from] ModelsStoreError),
}

/// A read-only view of the store's currently published state.
#[derive(Clone, Debug)]
pub struct StoreSnapshot {
    /// Currently published revision.
    pub revision: u64,
    /// Per-provider dynamic catalogs at that revision.
    pub catalogs: HashMap<String, ModelsStoreEntry>,
    /// Explicitly invalidated source identities. A new successful publication
    /// from that source clears its tombstone.
    pub invalidated_sources: BTreeSet<String>,
}

/// The acknowledged result of a successful [`super::RefreshGuard::commit`].
#[derive(Clone, Debug)]
pub struct StoreCommit {
    /// Revision this commit published.
    pub revision: u64,
    /// The committed catalog map to apply to the in-memory registry.
    pub catalogs: HashMap<String, ModelsStoreEntry>,
}
