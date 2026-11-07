//! Injectable authentication source access.
//! The default helper snapshot captures cwd on demand; construction does no I/O.

/// Typed failure of injectable [`AuthContext`] source access.
///
/// Source access is separated from credential values: an error here means the
/// *source* could not be consulted (non-unicode environment bytes, a filesystem
/// probe failure), never that a credential was missing. Missing values are
/// `Ok(None)` so callers never conflate "unconfigured" with "source failed".
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuthContextError {
    /// The named environment variable holds non-unicode bytes.
    #[error("environment variable '{name}' is not valid unicode")]
    EnvNotUnicode {
        /// Variable whose value could not be decoded.
        name: String,
    },
    /// An injected environment source failed for an implementation-specific
    /// reason.
    #[error("environment access failed for '{name}': {reason}")]
    EnvAccess {
        /// Variable that was being read.
        name: String,
        /// Stable, non-secret failure reason.
        reason: String,
    },
    /// A file-existence probe failed for a reason other than the file being
    /// absent (permissions, I/O error). A missing file is `Ok(false)`, not an
    /// error.
    #[error("file existence probe failed for '{path}': {reason}")]
    FileProbe {
        /// Path that was being probed.
        path: String,
        /// Stable, non-secret failure reason.
        reason: String,
    },
    /// The native helper's owned process context could not be captured.
    #[error("auth process context could not be captured: {reason}")]
    ProcessSnapshot {
        /// Fixed, secret-free failure reason.
        reason: &'static str,
    },
}

/// Owned environment and working directory for one selected auth helper.
///
/// Captured only when a source selects a native helper, never during
/// construction, registration, configuration checks, or static lookups.
/// Environment values can contain credentials and are omitted from Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthProcessSnapshot {
    /// Exact child environment; nothing else is inherited at spawn time.
    pub env: std::collections::BTreeMap<String, String>,
    /// Working directory frozen with the selected command and environment.
    pub cwd: std::path::PathBuf,
}

impl std::fmt::Debug for AuthProcessSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthProcessSnapshot")
            .field("env", &"<redacted>")
            .field("cwd", &self.cwd)
            .finish()
    }
}

/// Injectable ambient-source access for authentication (AIR-063/AIR-066).
///
/// This is a distinct context responsibility from the serializable model
/// Context/transcript (see [`crate::transcript`]) and from telemetry: it answers only
/// "what does the ambient environment say". [`Models`](crate::Models) construction,
/// registration, and static catalog lookup perform **zero** source access;
/// the context is consulted on demand by the management operations
/// ([`Models::check_auth`](crate::Models::check_auth),
/// [`get_auth`](crate::Models::get_auth),
/// [`get_available`](crate::Models::get_available)). Injecting an
/// implementation replaces ambient access entirely — the default
/// [`super::AmbientAuthContext`] is never consulted once a source is injected.
///
/// The contract never reads a credential to "verify" a source:
/// [`file_exists`](Self::file_exists) checks path existence only.
pub trait AuthContext: Send + Sync {
    /// Resolve the native home-directory source after explicit environment
    /// choices have been exhausted. Injected contexts default to no fallback,
    /// so native helpers never consult the real user's home implicitly.
    fn home_dir(&self) -> Result<Option<std::path::PathBuf>, AuthContextError> {
        Ok(None)
    }

    /// Read one named environment variable on demand.
    ///
    /// Returns `Ok(None)` when the variable is unset or empty after trimming.
    /// Non-unicode bytes and injected-source failures are typed errors; a
    /// missing value is never an error.
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError>;

    /// Probe whether a file exists, expanding a leading `~` to the user's home
    /// directory.
    ///
    /// Returns `Ok(false)` for a missing file, an unresolvable `~` expansion,
    /// or a platform without filesystem access (an injected source may answer
    /// `false` explicitly). Probe failures other than absence are typed
    /// errors. This method never reads file contents.
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError>;

    /// Freeze the native helper's environment and cwd at source selection.
    ///
    /// The default isolates injected contexts from the real environment.
    /// Override it to supply helper environment values explicitly. The cwd
    /// is captured on this call; merely constructing a context does no I/O.
    fn process_snapshot(&self) -> Result<AuthProcessSnapshot, AuthContextError> {
        Ok(AuthProcessSnapshot {
            env: std::collections::BTreeMap::new(),
            cwd: std::env::current_dir().map_err(|_| AuthContextError::ProcessSnapshot {
                reason: "working directory unavailable",
            })?,
        })
    }
}
