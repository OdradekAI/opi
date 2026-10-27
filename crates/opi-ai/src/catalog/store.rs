//! Persisted dynamic-catalog store with a single revision-commit owner.
//!
//! [`ModelsStore`] is the one owner that publishes a new dynamic-catalog
//! revision: a [`RefreshGuard::commit`] validates that the guard is still
//! current (not superseded by a newer revision, not cancelled), persists the
//! distribution-schema-v1 snapshot atomically (temporary file plus rename),
//! bumps the revision, and only then returns the committed catalog map that
//! the caller applies to its in-memory registry. Work whose guard has been
//! superseded or cancelled can therefore never publish late, neither to the
//! persisted file nor — because the caller applies exactly what the commit
//! acknowledged — to memory.
//!
//! Opening a store restores the last-known revision from disk before any
//! network work is attempted; `refresh(false)` on a [`crate::Models`]
//! collection restores without touching any provider. The SDK never attaches
//! a store or fetches a remote catalog by default: there is no pi.dev or
//! default startup policy here.
//!
//! The persisted file speaks the Pi *distribution* schema
//! (`https://pi.dev/schemas/models.schema.json`) plus an `opi` revision
//! header; see the private `catalog::wire` codec for the encoding contract. It is a
//! different artifact from the hydration data embedded in the builtin bundle.
//!
//! All operations are bounded, synchronous file work over small documents;
//! the critical section deliberately holds a `std::sync::Mutex` so callers
//! observe strict commit ordering.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::model_info::ModelInfo;

use super::wire;

/// The publisher of one acknowledged provider snapshot. Missing metadata
/// denotes an older or caller-owned publication with unknown provenance.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CatalogPublisher {
    Native,
    Source { id: String },
}

/// Typed failure while opening or decoding a [`ModelsStore`] file.
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

/// Typed rejection of a [`RefreshGuard::commit`].
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
    pub catalogs: HashMap<String, Vec<ModelInfo>>,
}

/// The acknowledged result of a successful [`RefreshGuard::commit`].
#[derive(Clone, Debug)]
pub struct StoreCommit {
    /// Revision this commit published.
    pub revision: u64,
    /// The committed catalog map to apply to the in-memory registry.
    pub catalogs: HashMap<String, Vec<ModelInfo>>,
}

#[derive(Debug, Default)]
struct StoreInner {
    revision: u64,
    catalogs: HashMap<String, Vec<ModelInfo>>,
    publishers: HashMap<String, CatalogPublisher>,
}

/// Handle to the persisted dynamic-catalog store. Cloning shares one inner
/// state, so guards created from any clone validate against the same
/// revision.
#[derive(Clone)]
pub struct ModelsStore {
    inner: Arc<Mutex<StoreInner>>,
    path: PathBuf,
}

impl std::fmt::Debug for ModelsStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let revision = self.inner.lock().map(|inner| inner.revision).unwrap_or(0);
        formatter
            .debug_struct("ModelsStore")
            .field("path", &self.path)
            .field("revision", &revision)
            .finish()
    }
}

impl ModelsStore {
    /// Open (or create) the store at `path`.
    ///
    /// An existing file is restored verbatim; a missing file starts at
    /// revision 0 with no catalogs. Nothing is written and no network work
    /// happens here.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ModelsStoreError> {
        let path = path.into();
        let inner = match std::fs::read(&path) {
            Ok(bytes) => {
                let document: Value =
                    serde_json::from_slice(&bytes).map_err(|error| ModelsStoreError::Corrupt {
                        message: format!("store file is not valid JSON: {error}"),
                    })?;
                let decoded = wire::decode_store_document(&document)?;
                StoreInner {
                    revision: decoded.revision,
                    catalogs: decoded.catalogs.into_iter().collect(),
                    publishers: decoded.publishers.into_iter().collect(),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => StoreInner::default(),
            Err(error) => return Err(ModelsStoreError::Io(error)),
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            path,
        })
    }

    /// Path of the persisted store file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Currently published revision.
    pub fn revision(&self) -> u64 {
        self.inner.lock().expect("store poisoned").revision
    }

    /// Read the currently published state without modifying anything.
    pub fn snapshot(&self) -> StoreSnapshot {
        self.snapshot_with_publishers().0
    }

    pub(crate) fn snapshot_with_publishers(
        &self,
    ) -> (StoreSnapshot, HashMap<String, CatalogPublisher>) {
        let inner = self.inner.lock().expect("store poisoned");
        (
            StoreSnapshot {
                revision: inner.revision,
                catalogs: inner.catalogs.clone(),
            },
            inner.publishers.clone(),
        )
    }

    /// Begin a refresh attempt at the current revision. The returned guard
    /// may commit only while no newer revision has been published.
    pub fn begin(&self) -> RefreshGuard {
        self.begin_inner(None)
    }

    /// Begin a refresh attempt that also observes `token`: a cancelled token
    /// makes the guard's commit fail without touching disk.
    pub fn begin_with(&self, token: CancellationToken) -> RefreshGuard {
        self.begin_inner(Some(token))
    }

    fn begin_inner(&self, token: Option<CancellationToken>) -> RefreshGuard {
        let guard_revision = self.revision();
        RefreshGuard {
            inner: Arc::clone(&self.inner),
            path: self.path.clone(),
            guard_revision,
            token,
        }
    }
}

/// Right to publish one revision, created at [`ModelsStore::begin`] time.
///
/// A guard commits at most once and only while the store has not moved past
/// the revision the guard was created at. Dropping a guard publishes
/// nothing.
pub struct RefreshGuard {
    inner: Arc<Mutex<StoreInner>>,
    path: PathBuf,
    guard_revision: u64,
    token: Option<CancellationToken>,
}

impl RefreshGuard {
    /// Commit `catalogs` as the next store revision.
    ///
    /// On success the snapshot is durably persisted (temporary file plus
    /// atomic rename) and the revision is bumped; the returned
    /// [`StoreCommit`] carries exactly what may be applied to memory. On
    /// rejection nothing is written and the published state is unchanged.
    pub async fn commit(
        self,
        catalogs: HashMap<String, Vec<ModelInfo>>,
    ) -> Result<StoreCommit, StoreCommitError> {
        self.commit_with_publishers(catalogs, HashMap::new())
            .await
            .map(|(commit, _)| commit)
    }

    pub(crate) async fn commit_with_publishers(
        self,
        catalogs: HashMap<String, Vec<ModelInfo>>,
        mut publishers: HashMap<String, CatalogPublisher>,
    ) -> Result<(StoreCommit, HashMap<String, CatalogPublisher>), StoreCommitError> {
        if self
            .token
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            return Err(StoreCommitError::Cancelled);
        }
        // Deterministic provider ordering keeps the serialized bytes stable.
        let ordered: BTreeMap<String, Vec<ModelInfo>> = catalogs.into_iter().collect();
        publishers.retain(|provider, _| ordered.contains_key(provider));
        let document = wire::encode_store_document(self.guard_revision + 1, &ordered, &publishers);
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| ModelsStoreError::Corrupt {
                message: format!("store document failed to encode: {error}"),
            })?
            .to_owned();

        let mut inner = self.inner.lock().expect("store poisoned");
        if inner.revision != self.guard_revision {
            return Err(StoreCommitError::Superseded {
                guard_revision: self.guard_revision,
                store_revision: inner.revision,
            });
        }
        let temp_path = self.path.with_extension("json.committing");
        std::fs::write(&temp_path, &bytes).map_err(ModelsStoreError::Io)?;
        // Rename within the same directory is atomic on the supported
        // platforms; a crash before the rename leaves only the temp file and
        // the previously published revision intact.
        std::fs::rename(&temp_path, &self.path).map_err(ModelsStoreError::Io)?;
        inner.revision += 1;
        inner.catalogs = ordered.into_iter().collect();
        inner.publishers = publishers;
        Ok((
            StoreCommit {
                revision: inner.revision,
                catalogs: inner.catalogs.clone(),
            },
            inner.publishers.clone(),
        ))
    }
}
