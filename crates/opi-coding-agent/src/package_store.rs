//! Package store and source model.
//!
//! Handles package source parsing (local paths and git URLs), declaration
//! files (`packages.toml`), lock files (`package-lock.toml`), and the store
//! operations that read/write them.
//!
//! # Store Scopes
//!
//! - **Global**: user-level config directory (`~/.config/opi/` on Unix,
//!   `%APPDATA%\opi\` on Windows).
//! - **Project**: workspace-local `.opi/` directory.
//!
//! # File Format
//!
//! `packages.toml` — user-facing declaration of desired packages:
//!
//! ```toml
//! [[package]]
//! source = "./vendor/todo"
//!
//! [[package]]
//! source = "git:github.com/user/repo@v1"
//! filters = { extensions = ["my-ext"] }
//! ```
//!
//! `package-lock.toml` — machine-generated lock state:
//!
//! ```toml
//! [[lock]]
//! identity_kind = "local"
//! identity_value = "/abs/path/to/pkg"
//! source = "./vendor/todo"
//! package_root = "/abs/path/to/pkg"
//! manifest_sha256 = "abcdef..."
//! ```
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors from package store operations.
#[derive(Debug, thiserror::Error)]
pub enum PackageStoreError {
    /// A package source string could not be parsed.
    #[error("invalid package source '{input}': {reason}")]
    InvalidSource { input: String, reason: String },
    /// An I/O error occurred during store operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// A TOML deserialization error.
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    /// A TOML serialization error.
    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
    /// A git operation failed.
    #[error("git operation failed: {0}")]
    Git(String),
    /// A package lifecycle operation failed.
    #[error("package error: {0}")]
    Package(String),
}

// ---------------------------------------------------------------------------
// Source types
// ---------------------------------------------------------------------------

/// A parsed package source: either a local path or a git URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    /// A local filesystem path (relative or absolute).
    Local { path: PathBuf },
    /// A git repository URL with an optional ref spec.
    Git {
        url: String,
        refspec: Option<String>,
    },
}

impl PackageSource {
    /// Parse a source string into a [`PackageSource`].
    ///
    /// Supported formats:
    /// - `./relative/path` or `/absolute/path` or `.\relative\path` or `D:\path` — local
    /// - `git:github.com/user/repo@ref` — GitHub shorthand (becomes `https://github.com/user/repo`)
    /// - `git:https://example.com/repo.git@ref` — full git URL with optional ref
    pub fn parse(raw: &str) -> Result<Self, PackageStoreError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(PackageStoreError::InvalidSource {
                input: raw.into(),
                reason: "source string is empty".into(),
            });
        }

        if let Some(rest) = trimmed.strip_prefix("git:") {
            // Git source
            if is_scp_like_git_source(rest) {
                return Err(PackageStoreError::InvalidSource {
                    input: raw.into(),
                    reason:
                        "scp-like git sources are not supported; use git:ssh://git@host/path@ref"
                            .into(),
                });
            }
            let (url, refspec) = split_git_url_and_ref(rest);
            let url = url.to_string();
            let refspec = refspec.map(str::to_string);

            if url.is_empty() {
                return Err(PackageStoreError::InvalidSource {
                    input: raw.into(),
                    reason: "git URL is empty after 'git:' prefix".into(),
                });
            }

            // Expand GitHub shorthand: no scheme and no "://" → prepend https://
            let expanded_url = if url.contains("://") {
                url
            } else if url.starts_with("github.com/") {
                format!("https://{url}")
            } else {
                return Err(PackageStoreError::InvalidSource {
                    input: raw.into(),
                    reason: format!(
                        "unrecognized git source '{url}': must be a URL with scheme or \
                         github.com shorthand"
                    ),
                });
            };

            Ok(PackageSource::Git {
                url: expanded_url,
                refspec,
            })
        } else if is_local_path(trimmed) {
            Ok(PackageSource::Local {
                path: PathBuf::from(trimmed),
            })
        } else {
            Err(PackageStoreError::InvalidSource {
                input: raw.into(),
                reason: "unrecognized source format: must start with './', '/', a drive letter, \
                     or 'git:' prefix"
                    .to_string(),
            })
        }
    }

    /// Return an identity key for this source.
    ///
    /// The `kind` field is `"local"` or `"git"`. The `value` field is the path
    /// or URL respectively.
    pub fn identity_key(&self) -> PackageIdentity {
        match self {
            PackageSource::Local { path } => PackageIdentity {
                kind: "local".into(),
                value: path.display().to_string(),
            },
            PackageSource::Git { url, .. } => PackageIdentity {
                kind: "git".into(),
                value: url.clone(),
            },
        }
    }
}

/// An identity key derived from a package source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageIdentity {
    /// Source kind: `"local"` or `"git"`.
    pub kind: String,
    /// Source value: path for local, URL for git.
    pub value: String,
}

// ---------------------------------------------------------------------------
// Store scope
// ---------------------------------------------------------------------------

/// The scope of a package store: global (user config) or project (workspace).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageStoreScope {
    /// User-level global store rooted in the config directory.
    Global { user_config_dir: PathBuf },
    /// Project-level store rooted in the workspace `.opi/` directory.
    Project { workspace_root: PathBuf },
}

impl PackageStoreScope {
    /// Path to the `packages.toml` declaration file.
    pub fn config_path(&self) -> PathBuf {
        match self {
            PackageStoreScope::Global { user_config_dir } => user_config_dir.join("packages.toml"),
            PackageStoreScope::Project { workspace_root } => {
                workspace_root.join(".opi").join("packages.toml")
            }
        }
    }

    /// Path to the `package-lock.toml` lock file.
    pub fn lock_path(&self) -> PathBuf {
        match self {
            PackageStoreScope::Global { user_config_dir } => {
                user_config_dir.join("package-lock.toml")
            }
            PackageStoreScope::Project { workspace_root } => {
                workspace_root.join(".opi").join("package-lock.toml")
            }
        }
    }

    /// Path to the package cache directory.
    pub fn cache_dir(&self) -> PathBuf {
        match self {
            PackageStoreScope::Global { user_config_dir } => user_config_dir.join("package-cache"),
            PackageStoreScope::Project { workspace_root } => {
                workspace_root.join(".opi").join("package-cache")
            }
        }
    }

    /// Path to the machine-owned `package-trust.toml` file (Phase 16.5).
    ///
    /// Holds Package Trust + enablement records for executable
    /// `command.execute` contributions. Execution packages are global-only, so
    /// only the Global path is used in practice; the Project path exists for
    /// symmetry. This file is never user-edited (unlike `packages.toml`).
    pub fn trust_path(&self) -> PathBuf {
        match self {
            PackageStoreScope::Global { user_config_dir } => {
                user_config_dir.join("package-trust.toml")
            }
            PackageStoreScope::Project { workspace_root } => {
                workspace_root.join(".opi").join("package-trust.toml")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Declaration types (packages.toml)
// ---------------------------------------------------------------------------

/// A package declaration entry from `packages.toml`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageDeclaration {
    /// Source string for the package (local path or git URL).
    pub source: String,
    /// Optional resource filters.
    #[serde(default)]
    pub filters: PackageFilters,
}

/// Resource filters for a package declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageFilters {
    /// Include only these extensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
    /// Include only these skills.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    /// Include only these prompt fragments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragments: Option<Vec<String>>,
    /// Include only these themes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<String>>,
}

/// Top-level TOML structure for `packages.toml`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct PackagesFile {
    #[serde(default, rename = "package", skip_serializing_if = "Vec::is_empty")]
    packages: Vec<PackageDeclaration>,
}

// ---------------------------------------------------------------------------
// Lock entry types (package-lock.toml)
// ---------------------------------------------------------------------------

/// A lock entry recording the resolved state of an installed package.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageLockEntry {
    /// Source kind: `"local"` or `"git"`.
    pub identity_kind: String,
    /// Source value: path or URL.
    pub identity_value: String,
    /// Original source string from the declaration.
    pub source: String,
    /// Absolute path to the resolved package root.
    pub package_root: PathBuf,
    /// Optional path to the local cache directory (for git sources).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_path: Option<PathBuf>,
    /// Git commit SHA (for git sources).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    /// SHA-256 hash of the package manifest (`package.toml`).
    pub manifest_sha256: String,
    /// Phase 16: validated executable `command.execute` contributions locked
    /// for this package. Empty for non-execution packages (extensions, themes,
    /// fragments). `#[serde(default)]` keeps pre-Phase-16 lock files parsing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributions: Vec<crate::execution::LockMaterial>,
}

/// A cache replacement that can be committed after package metadata is written.
#[derive(Debug)]
pub struct PendingCacheReplacement {
    target: PathBuf,
    backup: Option<PathBuf>,
    committed: bool,
}

impl PendingCacheReplacement {
    /// Keep the newly installed cache directory and remove the old backup.
    pub fn commit(mut self) {
        if let Some(backup) = &self.backup {
            let _ = remove_path(backup);
        }
        self.committed = true;
    }

    /// Restore the cache directory that was present before replacement.
    pub fn rollback(mut self) -> Result<(), PackageStoreError> {
        rollback_cache_replacement(&self.target, self.backup.as_deref())?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for PendingCacheReplacement {
    fn drop(&mut self) {
        if !self.committed {
            let _ = rollback_cache_replacement(&self.target, self.backup.as_deref());
            self.committed = true;
        }
    }
}

/// Top-level TOML structure for `package-lock.toml`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct PackageLockFile {
    #[serde(default, rename = "lock", skip_serializing_if = "Vec::is_empty")]
    locks: Vec<PackageLockEntry>,
}

// ---------------------------------------------------------------------------
// Package store
// ---------------------------------------------------------------------------

/// The package store, providing read/write access to declarations and lock
/// files within a given scope.
#[derive(Debug, Clone)]
pub struct PackageStore {
    scope: PackageStoreScope,
}

impl PackageStore {
    /// Create a new store for the given scope.
    pub fn new(scope: PackageStoreScope) -> Self {
        Self { scope }
    }

    /// Convenience constructor for a project-scoped store.
    pub fn project(workspace_root: PathBuf) -> Self {
        Self::new(PackageStoreScope::Project { workspace_root })
    }

    /// Convenience constructor for a global-scoped store.
    pub fn global(user_config_dir: PathBuf) -> Self {
        Self::new(PackageStoreScope::Global { user_config_dir })
    }

    /// Read package declarations from `packages.toml`.
    ///
    /// Returns an empty vector if the file does not exist.
    pub fn read_declarations(&self) -> Result<Vec<PackageDeclaration>, PackageStoreError> {
        let path = self.scope.config_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let file: PackagesFile = toml::from_str(&content)?;
        Ok(file.packages)
    }

    /// Write package declarations to `packages.toml`, creating parent
    /// directories as needed.
    pub fn write_declarations(
        &self,
        declarations: &[PackageDeclaration],
    ) -> Result<(), PackageStoreError> {
        let path = self.scope.config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = PackagesFile {
            packages: declarations.to_vec(),
        };
        let content = toml::to_string_pretty(&file)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Read lock entries from `package-lock.toml`.
    ///
    /// Returns an empty vector if the file does not exist.
    pub fn read_lock(&self) -> Result<Vec<PackageLockEntry>, PackageStoreError> {
        let path = self.scope.lock_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let file: PackageLockFile = toml::from_str(&content)?;
        Ok(file.locks)
    }

    /// Write lock entries to `package-lock.toml`, creating parent directories
    /// as needed.
    pub fn write_lock(&self, entries: &[PackageLockEntry]) -> Result<(), PackageStoreError> {
        let path = self.scope.lock_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = PackageLockFile {
            locks: entries.to_vec(),
        };
        let content = toml::to_string_pretty(&file)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Return the cache directory path for this store's scope.
    pub fn cache_dir(&self) -> PathBuf {
        self.scope.cache_dir()
    }

    /// Return the machine-owned `package-trust.toml` path for this store's
    /// scope (Phase 16.5).
    pub fn trust_path(&self) -> PathBuf {
        self.scope.trust_path()
    }

    /// Clone a git repository and check out a specific commit.
    ///
    /// Uses `git clone` and `git checkout` via `std::process::Command`.
    /// Does not add `git2` or `gix` dependencies.
    pub fn git_clone(
        &self,
        url: &str,
        refspec: Option<&str>,
        target: &Path,
    ) -> Result<(), PackageStoreError> {
        let staging = self.git_clone_to_staging(url, refspec, target)?;
        let replacement = self.stage_cache_replacement(target, &staging)?;
        replacement.commit();
        Ok(())
    }

    /// Clone a git repository into a temporary staging directory under the
    /// target cache parent. The final target is not touched.
    pub fn git_clone_to_staging(
        &self,
        url: &str,
        refspec: Option<&str>,
        target: &Path,
    ) -> Result<PathBuf, PackageStoreError> {
        // Validate that the target is within the store's cache directory.
        let cache_dir = self.scope.cache_dir();
        // Canonicalize cache_dir (must already exist or be creatable).
        // Since target may not exist yet, canonicalize the parent and verify
        // the final component does not escape via `..`.
        let target_normalized = normalize_path(target);
        let cache_normalized = normalize_path(&cache_dir);
        if !target_normalized.starts_with(&cache_normalized) {
            return Err(PackageStoreError::Git(format!(
                "clone target {target:?} is outside the store cache directory {cache_dir:?}"
            )));
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        } else {
            return Err(PackageStoreError::Git(format!(
                "clone target {target:?} has no parent directory"
            )));
        }
        let staging = staging_dir_for(target);
        if staging.exists() {
            remove_path(&staging)?;
        }

        // Clone the repository
        let output = std::process::Command::new("git")
            .args(["clone", url])
            .arg(&staging)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| PackageStoreError::Git(format!("failed to execute git clone: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = remove_path(&staging);
            return Err(PackageStoreError::Git(format!(
                "git clone failed: {stderr}"
            )));
        }

        if let Some(refspec) = refspec {
            let checkout_output = std::process::Command::new("git")
                .args(["checkout", refspec])
                .current_dir(&staging)
                .env("GIT_TERMINAL_PROMPT", "0")
                .output()
                .map_err(|e| {
                    PackageStoreError::Git(format!("failed to execute git checkout: {e}"))
                })?;

            if !checkout_output.status.success() {
                let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                let _ = remove_path(&staging);
                return Err(PackageStoreError::Git(format!(
                    "git checkout {refspec} failed: {stderr}"
                )));
            }
        }

        Ok(staging)
    }

    /// Replace a cache directory with a previously validated staging directory.
    pub fn replace_cache_dir(
        &self,
        target: &Path,
        staging: &Path,
    ) -> Result<(), PackageStoreError> {
        let replacement = self.stage_cache_replacement(target, staging)?;
        replacement.commit();
        Ok(())
    }

    /// Replace a cache directory with a staging directory, keeping the old
    /// cache available until the caller commits or rolls back.
    pub fn stage_cache_replacement(
        &self,
        target: &Path,
        staging: &Path,
    ) -> Result<PendingCacheReplacement, PackageStoreError> {
        let cache_dir = self.scope.cache_dir();
        let target_normalized = normalize_path(target);
        let staging_normalized = normalize_path(staging);
        let cache_normalized = normalize_path(&cache_dir);
        if !target_normalized.starts_with(&cache_normalized) {
            return Err(PackageStoreError::Git(format!(
                "replace target {target:?} is outside the store cache directory {cache_dir:?}"
            )));
        }
        if !staging_normalized.starts_with(&cache_normalized) {
            return Err(PackageStoreError::Git(format!(
                "staging directory {staging:?} is outside the store cache directory {cache_dir:?}"
            )));
        }
        if !staging.is_dir() {
            return Err(PackageStoreError::Git(format!(
                "staging directory does not exist: {}",
                staging.display()
            )));
        }

        let backup = temporary_cache_dir_for(target, "backup");
        if backup.exists() {
            remove_path(&backup)?;
        }

        let had_target = target.exists();
        if had_target {
            std::fs::rename(target, &backup)?;
        }

        match std::fs::rename(staging, target) {
            Ok(()) => Ok(PendingCacheReplacement {
                target: target.to_path_buf(),
                backup: had_target.then_some(backup),
                committed: false,
            }),
            Err(e) => {
                if had_target {
                    let _ = std::fs::rename(&backup, target);
                }
                Err(PackageStoreError::Io(e))
            }
        }
    }

    /// Return the current HEAD commit for a cloned repository.
    pub fn git_rev_parse_head(&self, repo: &Path) -> Result<String, PackageStoreError> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|e| PackageStoreError::Git(format!("failed to execute git rev-parse: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PackageStoreError::Git(format!(
                "git rev-parse HEAD failed: {stderr}"
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

fn split_git_url_and_ref(rest: &str) -> (&str, Option<&str>) {
    let Some(at_pos) = rest.rfind('@') else {
        return (rest, None);
    };

    if let Some(scheme_pos) = rest.find("://") {
        let path_start = rest[scheme_pos + 3..]
            .find('/')
            .map(|offset| scheme_pos + 3 + offset);
        if path_start.is_some_and(|slash| at_pos > slash) {
            return (&rest[..at_pos], Some(&rest[at_pos + 1..]));
        }
        return (rest, None);
    }

    (&rest[..at_pos], Some(&rest[at_pos + 1..]))
}

fn is_scp_like_git_source(rest: &str) -> bool {
    if rest.contains("://") || rest.starts_with("github.com/") {
        return false;
    }
    let Some(first_slash) = rest.find('/') else {
        return rest.contains('@') && rest.contains(':');
    };
    let authority = &rest[..first_slash];
    authority.contains('@') && authority.contains(':')
}

// ---------------------------------------------------------------------------
// Path detection helpers
// ---------------------------------------------------------------------------

/// Determine if a source string looks like a local filesystem path.
///
/// Recognizes:
/// - Relative paths starting with `.`, `./`, `.\`, `..`
/// - Unix absolute paths starting with `/`
/// - Windows UNC paths starting with `\\`
/// - Windows drive-letter paths like `C:\` or `D:/`
///
/// Rejects anything with a `:` that isn't a single-character drive letter
/// followed by a path separator (so `npm:foo`, `http://...` etc. are not
/// treated as local paths).
fn is_local_path(s: &str) -> bool {
    if s.starts_with('.') || s.starts_with('/') {
        return true;
    }
    // Windows UNC
    if s.starts_with('\\') {
        return true;
    }
    // Windows drive letter: exactly one ASCII letter, then ':', then '/' or '\'
    let bytes = s.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    false
}

/// Normalize a path by resolving `.` and `..` components without touching the
/// filesystem. This avoids `canonicalize` which requires the path to exist.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if components
                    .last()
                    .is_some_and(|c| !matches!(c, std::path::Component::ParentDir))
                {
                    components.pop();
                } else {
                    components.push(comp);
                }
            }
            _ => components.push(comp),
        }
    }
    components.iter().collect()
}

fn staging_dir_for(target: &Path) -> PathBuf {
    temporary_cache_dir_for(target, "staging")
}

fn temporary_cache_dir_for(target: &Path, kind: &str) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("package-cache");
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    parent.join(format!(".{name}.{kind}.{}.{}", std::process::id(), nanos))
}

fn remove_path(path: &Path) -> Result<(), PackageStoreError> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn rollback_cache_replacement(
    target: &Path,
    backup: Option<&Path>,
) -> Result<(), PackageStoreError> {
    if target.exists() {
        remove_path(target)?;
    }
    if let Some(backup) = backup
        && backup.exists()
    {
        std::fs::rename(backup, target)?;
    }
    Ok(())
}
