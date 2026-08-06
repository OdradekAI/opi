//! Package Trust and enable/disable lifecycle for executable
//! `command.execute` contributions (Phase 16.5).
//!
//! Execution-adapter packages are **global-only**. `package add` validates the
//! package's `[[contributions.adapters]]` via [`crate::execution::contribution`]
//! and records the exact [`LockMaterial`] in the package lock (`package-lock.toml`,
//! per spec line 363-364), while leaving the package **untrusted and disabled**
//! in a separate machine-owned `package-trust.toml`. The first interactive
//! `enable` displays identity/version/locked-executable-hash/contributions and
//! requires explicit confirmation before granting Package Trust. `disable`
//! clears enablement but preserves the trust record; `remove` deletes all
//! three. Manifest, lock, or executable **drift durably invalidates** Package
//! Trust (spec line 318-320): the record is reset so re-enable forces review.
//!
//! [`PackageActivationStore::activate`] is the pre-spawn revalidation seam the
//! execution protocol host (Phase 16.7) calls immediately before every process
//! start. It resolves ONLY the named package, re-runs the static contribution
//! gates, recomputes the lock material, and either returns an
//! [`ActivatedContribution`] (metadata only — no spawn) or fails closed. The
//! actual process spawn is owned by 16.7; startup wiring by 16.9.
//!
//! Failure codes use 16.5's OWN internal vocabulary ([`ActivationError`]); the
//! architecture-level `ExecutionFailure` stable-code envelope is owned by Phase
//! 16.6, which maps these via `From<ActivationError>`
//! (`NotInstalled` -> `package_not_installed`, `Untrusted` -> `package_untrusted`,
//! `Disabled` -> `contribution_disabled`).
//!
//! # Unstable
//!
//! This module is part of the **unstable 0.x extension API**. Breaking changes
//! may occur between minor versions without a major version bump.

use std::path::{Path, PathBuf};

use crate::execution::{
    ContributionValidationError, EnabledIdentity, LockMaterial, ValidatedExecutableContribution,
    validate_executable_contributions,
};
// The contribution validator's scope enum is named `PackageSource`, which
// collides with `package_store::PackageSource` (Local/Git source KIND). Alias it
// to `ContributionScope` (install SCOPE) everywhere in this module to keep the
// two unrelated enums unambiguous.
use crate::execution::PackageSource as ContributionScope;
use crate::package_discovery::PackageManifest;
use crate::package_store::{PackageLockEntry, PackageStore, PackageStoreError};

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_RECORD_WRITE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn fail_next_record_write_for_test() {
    FAIL_NEXT_RECORD_WRITE.with(|fail| fail.set(true));
}

// ---------------------------------------------------------------------------
// Host identity
// ---------------------------------------------------------------------------

/// The running host's Rust target triple, used for the exact-match target gate.
///
/// Covers the six CI/supported targets. The `unknown` fallback matches no
/// contribution, so every executable contribution fails the target gate closed
/// on unsupported hosts.
pub fn host_target_triple() -> &'static str {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu",
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu",
        ("x86_64", "macos") => "x86_64-apple-darwin",
        ("aarch64", "macos") => "aarch64-apple-darwin",
        ("x86_64", "windows") => "x86_64-pc-windows-msvc",
        ("aarch64", "windows") => "aarch64-pc-windows-msvc",
        _ => "unknown-unknown-unknown",
    }
}

/// The running Opi version, used for the hard opi-range compatibility gate.
pub fn host_opi_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ---------------------------------------------------------------------------
// Persisted record
// ---------------------------------------------------------------------------

/// One package's persisted Package Trust + enablement record.
///
/// Lives in the machine-owned `package-trust.toml` (Global scope only;
/// execution packages are global-only). The validated lock material itself
/// lives in the package LOCK (`PackageLockEntry.contributions`), per spec line
/// 363-364; this record only references the package by `source` so
/// [`PackageActivationStore::activate`] can do a direct lock-entry lookup
/// without scanning every package manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ActivationRecord {
    /// Package name (from the manifest).
    pub name: String,
    /// Declaration source string used to find the matching `PackageLockEntry`.
    pub source: String,
    /// Whether the user has granted Package Trust (survives disable; cleared on
    /// drift or remove).
    #[serde(default)]
    pub trusted: bool,
    /// Whether the contribution is enabled for activation (toggled by
    /// enable/disable).
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct TrustFile {
    #[serde(default, rename = "record", skip_serializing_if = "Vec::is_empty")]
    records: Vec<ActivationRecord>,
}

// ---------------------------------------------------------------------------
// Activation outcome + internal error
// ---------------------------------------------------------------------------

/// A package whose executable contribution was validated and whose trust is
/// current at activation time.
///
/// **Metadata only.** The protocol host (Phase 16.7) re-runs
/// [`PackageActivationStore::activate`] immediately before spawn and reads the
/// executable itself, so no validated-bytes handle or TOCTOU contract is
/// carried here.
#[derive(Debug, Clone)]
pub struct ActivatedContribution {
    pub name: String,
    pub source: String,
    pub validated: Vec<ValidatedExecutableContribution>,
    pub lock: PackageLockEntry,
}

/// Internal activation/trust failure vocabulary (Phase 16.5).
///
/// Variant names are deliberately NOT the architecture-level stable-code
/// strings; Phase 16.6's `ExecutionFailure` is the single source of truth for
/// those and maps these via `From<ActivationError>`.
#[derive(Debug, thiserror::Error)]
pub enum ActivationError {
    /// No installed/trust record resolves for the name
    /// (16.6 -> `package_not_installed`).
    #[error("package {0:?} is not installed")]
    NotInstalled(String),
    /// Present but untrusted, or trust invalidated by drift / a static-gate
    /// re-failure (16.6 -> `package_untrusted`). `detail` is remediation text.
    #[error("package {name:?} is not trusted: {detail}")]
    Untrusted { name: String, detail: String },
    /// Present and trusted but not enabled (16.6 -> `contribution_disabled`).
    #[error("package {0:?} is trusted but not enabled")]
    Disabled(String),
    /// An adapter id collides with another installed/enabled package
    /// (spec line 823).
    #[error("adapter id {adapter_id:?} collides with package {other:?}")]
    CollidingAdapterId { adapter_id: String, other: String },
    /// Store I/O error.
    #[error(transparent)]
    Store(#[from] PackageStoreError),
}

/// Internal revalidation outcome (not public): a static-gate re-failure, a
/// locked-material drift, or an I/O error. All map to [`ActivationError::Untrusted`].
#[derive(Debug)]
enum RevalidationError {
    Store(PackageStoreError),
    Gate(ContributionValidationError),
    Drift { adapter_id: String },
    NoLockedContribution { adapter_id: String },
}

impl RevalidationError {
    fn detail(&self) -> String {
        match self {
            RevalidationError::Store(e) => format!("store error during revalidation: {e}"),
            RevalidationError::Gate(e) => format!("contribution no longer validates: {e}"),
            RevalidationError::Drift { adapter_id } => {
                format!("locked material drift for adapter {adapter_id:?}")
            }
            RevalidationError::NoLockedContribution { adapter_id } => {
                format!("no locked contribution for adapter {adapter_id:?}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trust confirmation seam
// ---------------------------------------------------------------------------

/// Identity/version/hash/contribution summary shown before a first-enable
/// trust grant.
#[derive(Debug, Clone)]
pub struct TrustDisplay {
    pub name: String,
    pub version: Option<String>,
    pub contributions: Vec<ContributionSummary>,
}

/// One contribution's identity shown during trust confirmation.
#[derive(Debug, Clone)]
pub struct ContributionSummary {
    pub adapter_id: String,
    pub executable_rel_path: String,
    pub executable_sha256: String,
    pub target: String,
    pub protocol: String,
}

/// Interactive confirmation seam for first-enable Package Trust grants.
///
/// The production implementation ([`StdinTrustConfirmer`]) refuses on a
/// non-TTY stdin (a machine-facing invocation cannot grant trust) and requires
/// the user to type the exact package name to confirm. Tests inject a
/// deterministic confirmer.
pub trait TrustConfirmer {
    /// Show `display` and decide the trust grant.
    ///
    /// Returns `Ok(())` only if the user explicitly confirmed trust for the
    /// named package. Returns `Err(reason)` if confirmation was refused or
    /// impossible (e.g. non-TTY); `reason` is remediation text.
    fn confirm(&mut self, display: &TrustDisplay) -> Result<(), String>;
}

/// Production stdin-based confirmer: refuses on non-TTY, else requires the
/// user to type the exact package name to grant trust.
pub struct StdinTrustConfirmer;

impl TrustConfirmer for StdinTrustConfirmer {
    fn confirm(&mut self, display: &TrustDisplay) -> Result<(), String> {
        use std::io::{IsTerminal, Write};
        if !std::io::stdin().is_terminal() {
            return Err(
                "enable requires an interactive terminal; refusing to grant trust in a \
                 non-TTY/machine-facing invocation. Run `opi package enable` from a terminal \
                 and confirm the package identity."
                    .into(),
            );
        }
        println!(
            "Package: {} ({})",
            display.name,
            display.version.as_deref().unwrap_or("-")
        );
        for c in &display.contributions {
            println!(
                "  adapter {} -> {} (sha256 {}, target {}, protocol {})",
                c.adapter_id, c.executable_rel_path, c.executable_sha256, c.target, c.protocol
            );
        }
        print!(
            "Enabling this package trusts the executable listed above. Opi does NOT \
             authenticate the publisher. Type the package name `{}` to confirm trust: ",
            display.name
        );
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        match std::io::stdin().read_line(&mut line) {
            Ok(_) => {
                if line.trim() == display.name {
                    Ok(())
                } else {
                    Err(
                        "confirmation text did not match the package name; trust not granted"
                            .into(),
                    )
                }
            }
            Err(e) => Err(format!("could not read confirmation: {e}")),
        }
    }
}

// ---------------------------------------------------------------------------
// PackageActivationStore
// ---------------------------------------------------------------------------

/// The store backing Package Trust + enablement, persisted to
/// `package-trust.toml` in the Global config directory. Execution packages are
/// global-only; the activation/trust logic lives here, over a [`PackageStore`]
/// that owns declaration/lock/trust file I/O.
#[derive(Debug, Clone)]
pub struct PackageActivationStore {
    store: PackageStore,
}

impl PackageActivationStore {
    /// Create a global-scoped activation store rooted in `user_config_dir`.
    pub fn global(user_config_dir: PathBuf) -> Self {
        Self {
            store: PackageStore::global(user_config_dir),
        }
    }

    /// Wrap an existing [`PackageStore`] (must be Global-scoped for execution
    /// packages).
    pub fn from_store(store: PackageStore) -> Self {
        Self { store }
    }

    /// Borrow the underlying package store (for lock/declaration reads).
    pub fn store(&self) -> &PackageStore {
        &self.store
    }

    /// Read all trust/enablement records. Empty if `package-trust.toml` is absent.
    pub fn read_records(&self) -> Result<Vec<ActivationRecord>, PackageStoreError> {
        let path = self.store.trust_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let file: TrustFile = toml::from_str(&content)?;
        Ok(file.records)
    }

    /// The enabled named-package identities resolvable at startup (Phase 16.9).
    ///
    /// Reads the machine `package-trust.toml`, keeps `trusted && enabled`
    /// records, and expands each to one [`EnabledIdentity`] per locked adapter
    /// id. A corrupt or unreadable trust file yields an empty set (mirrors the
    /// tolerant `doctor` read pattern) so an invalid package-store sentinel
    /// never blocks Minimal Runtime startup. The heavier per-package
    /// revalidation (target/version/hash/drift) stays at [`Self::activate`]
    /// time, immediately before every spawn.
    pub fn enabled_identities(&self) -> Vec<EnabledIdentity> {
        let records = self.read_records().unwrap_or_default();
        let mut out = Vec::new();
        for record in records.iter().filter(|r| r.trusted && r.enabled) {
            for adapter_id in self
                .installed_adapter_ids(&record.source)
                .unwrap_or_default()
            {
                out.push(EnabledIdentity {
                    adapter_id,
                    package_name: record.name.clone(),
                });
            }
        }
        out
    }

    /// Resolve the enabled identities that are usable by this exact host.
    /// Each trusted+enabled package is activated once, so target/version,
    /// manifest, lock, executable type, and executable hash are current before
    /// an identity is exposed in a model-visible schema. Invocation-time
    /// activation still repeats the same validation immediately before spawn.
    pub fn usable_enabled_identities(
        &self,
        host_target: &str,
        host_opi_version: &str,
    ) -> Vec<EnabledIdentity> {
        let records = self.read_records().unwrap_or_default();
        let mut out = Vec::new();
        for record in records
            .iter()
            .filter(|record| record.trusted && record.enabled)
        {
            let Ok(activated) = self.activate(&record.name, host_target, host_opi_version) else {
                continue;
            };
            out.extend(
                activated
                    .validated
                    .iter()
                    .map(|contribution| EnabledIdentity {
                        adapter_id: contribution.id.clone(),
                        package_name: record.name.clone(),
                    }),
            );
        }
        out
    }

    /// Write all trust/enablement records, creating parent directories.
    pub fn write_records(&self, records: &[ActivationRecord]) -> Result<(), PackageStoreError> {
        #[cfg(test)]
        if FAIL_NEXT_RECORD_WRITE.with(std::cell::Cell::take) {
            return Err(PackageStoreError::Io(std::io::Error::other(
                "injected one-shot package trust write failure",
            )));
        }
        let path = self.store.trust_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = TrustFile {
            records: records.to_vec(),
        };
        let content = toml::to_string_pretty(&file)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Record a freshly-installed execution package as untrusted + disabled,
    /// after its contributions are validated and the lock written. Rejects if
    /// any contribution adapter id collides with another installed package
    /// (cross-package collision, spec line 823).
    pub fn install(
        &self,
        name: &str,
        source: &str,
        previous_source: Option<&str>,
        adapter_ids: &[String],
    ) -> Result<(), ActivationError> {
        let mut records = self.read_records()?;
        // Cross-package adapter-id collision across installed packages.
        for existing in &records {
            if existing.source == source || previous_source == Some(existing.source.as_str()) {
                continue; // re-install / re-add of the same source: upsert below.
            }
            let existing_ids = self.installed_adapter_ids(&existing.source)?;
            for id in adapter_ids {
                if existing_ids.iter().any(|e| e == id) {
                    return Err(ActivationError::CollidingAdapterId {
                        adapter_id: id.clone(),
                        other: existing.name.clone(),
                    });
                }
            }
        }
        upsert_record(
            &mut records,
            ActivationRecord {
                name: name.to_string(),
                source: source.to_string(),
                trusted: false,
                enabled: false,
            },
            previous_source,
        );
        self.write_records(&records)?;
        Ok(())
    }

    /// Grant trust + enable a package. First enablement (or re-enablement after
    /// drift invalidated trust) requires explicit interactive confirmation via
    /// `confirmer`; a non-TTY/machine-facing confirmer refuses. Re-enablement
    /// of an already-trusted, non-drifted package does not re-prompt. Drift
    /// detected here durably invalidates trust.
    pub fn enable(
        &self,
        name: &str,
        host_target: &str,
        host_opi_version: &str,
        confirmer: &mut dyn TrustConfirmer,
    ) -> Result<(), ActivationError> {
        let mut records = self.read_records()?;
        let idx = records
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))?;
        let source = records[idx].source.clone();
        let lock = self
            .find_lock_by_source(&source)?
            .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))?;

        // Re-validate on every enable (catches drift on the already-trusted path).
        let validated = match self.revalidate_lock(&lock, host_target, host_opi_version) {
            Ok(v) => v,
            Err(re) => {
                // Drift / static-gate re-failure: durably invalidate trust.
                records[idx].trusted = false;
                records[idx].enabled = false;
                self.write_records(&records)?;
                return Err(ActivationError::Untrusted {
                    name: name.to_string(),
                    detail: re.detail(),
                });
            }
        };

        let adapter_ids: Vec<String> = validated.iter().map(|v| v.id.clone()).collect();

        if !records[idx].trusted {
            // First enablement (or re-enablement after invalidation): require
            // explicit interactive confirmation.
            let display = build_trust_display(name, &lock, &validated);
            confirmer
                .confirm(&display)
                .map_err(|reason| ActivationError::Untrusted {
                    name: name.to_string(),
                    detail: format!("trust not granted: {reason}"),
                })?;
            records[idx].trusted = true;
        }

        // Defense-in-depth collision check vs other ENABLED packages.
        for other in &records {
            if other.name == name || !other.enabled {
                continue;
            }
            let other_ids = self
                .installed_adapter_ids(&other.source)
                .unwrap_or_default();
            for id in &adapter_ids {
                if other_ids.iter().any(|o| o == id) {
                    return Err(ActivationError::CollidingAdapterId {
                        adapter_id: id.clone(),
                        other: other.name.clone(),
                    });
                }
            }
        }

        records[idx].enabled = true;
        self.write_records(&records)?;
        Ok(())
    }

    /// Clear enablement while preserving the (unchanged) Package Trust record.
    pub fn disable(&self, name: &str) -> Result<(), ActivationError> {
        let mut records = self.read_records()?;
        let rec = records
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))?;
        rec.enabled = false;
        self.write_records(&records)?;
        Ok(())
    }

    /// Delete the trust/enablement record for a package matched by name or
    /// source. Returns whether a record was removed.
    pub fn remove(&self, name_or_source: &str) -> Result<bool, ActivationError> {
        let mut records = self.read_records()?;
        let before = records.len();
        records.retain(|r| r.name != name_or_source && r.source != name_or_source);
        let removed = records.len() != before;
        if removed {
            self.write_records(&records)?;
        }
        Ok(removed)
    }

    /// The pre-spawn revalidation seam. Resolves ONLY the named package,
    /// revalidates manifest/lock/executable-hash/target/version/protocol/trust,
    /// and returns an [`ActivatedContribution`] (metadata only; no spawn) or
    /// fails closed. Drift durably invalidates trust. Phase 16.7 calls this
    /// immediately before every process start.
    pub fn activate(
        &self,
        name: &str,
        host_target: &str,
        host_opi_version: &str,
    ) -> Result<ActivatedContribution, ActivationError> {
        let records = self.read_records()?;
        let record = records
            .iter()
            .find(|r| r.name == name)
            .cloned()
            .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))?;
        // Resolve the named package's lock entry directly (no full manifest scan).
        let lock = self
            .find_lock_by_source(&record.source)?
            .ok_or_else(|| ActivationError::NotInstalled(name.to_string()))?;
        if !record.trusted {
            return Err(ActivationError::Untrusted {
                name: name.to_string(),
                detail: "package is not trusted".into(),
            });
        }
        if !record.enabled {
            return Err(ActivationError::Disabled(name.to_string()));
        }
        let validated = match self.revalidate_lock(&lock, host_target, host_opi_version) {
            Ok(v) => v,
            Err(re) => {
                // Drift / static-gate re-failure: durably invalidate trust.
                let mut recs = self.read_records()?;
                if let Some(r) = recs.iter_mut().find(|r| r.name == name) {
                    r.trusted = false;
                    r.enabled = false;
                }
                self.write_records(&recs)?;
                return Err(ActivationError::Untrusted {
                    name: name.to_string(),
                    detail: re.detail(),
                });
            }
        };
        Ok(ActivatedContribution {
            name: name.to_string(),
            source: record.source,
            validated,
            lock,
        })
    }

    // -- private helpers ---------------------------------------------------

    /// Find the lock entry whose `source` matches (the named-package lookup).
    fn find_lock_by_source(
        &self,
        source: &str,
    ) -> Result<Option<PackageLockEntry>, PackageStoreError> {
        let locks = self.store.read_lock()?;
        Ok(locks.into_iter().find(|l| l.source == source))
    }

    /// Adapter ids locked for an installed package (by source).
    fn installed_adapter_ids(&self, source: &str) -> Result<Vec<String>, PackageStoreError> {
        let lock = self.find_lock_by_source(source)?;
        Ok(lock
            .map(|l| l.contributions)
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.adapter_id)
            .collect())
    }

    /// Re-run the static contribution gates against the current files and
    /// compare the recomputed lock material to the stored lock. Any static-gate
    /// re-failure OR lock-material drift is a trust invalidation.
    fn revalidate_lock(
        &self,
        lock: &PackageLockEntry,
        host_target: &str,
        host_opi_version: &str,
    ) -> Result<Vec<ValidatedExecutableContribution>, RevalidationError> {
        let manifest_path = lock.package_root.join("package.toml");
        let raw = std::fs::read(&manifest_path)
            .map_err(|e| RevalidationError::Store(PackageStoreError::from(e)))?;
        let manifest =
            PackageManifest::from_toml(&String::from_utf8_lossy(&raw), &manifest_path)
                .map_err(|e| RevalidationError::Store(PackageStoreError::Package(e.to_string())))?;
        let validated = validate_executable_contributions(
            &manifest,
            &raw,
            &lock.package_root,
            ContributionScope::Global,
            host_target,
            host_opi_version,
        )
        .map_err(RevalidationError::Gate)?;
        if validated.len() != lock.contributions.len() {
            let adapter_id = lock
                .contributions
                .iter()
                .find(|stored| !validated.iter().any(|v| v.lock == **stored))
                .map(|stored| stored.adapter_id.clone())
                .unwrap_or_else(|| "contribution-set".to_string());
            return Err(RevalidationError::Drift { adapter_id });
        }
        // Drift: compare recomputed lock material to the stored contributions.
        for v in &validated {
            let stored = lock
                .contributions
                .iter()
                .find(|c| c.adapter_id == v.lock.adapter_id);
            let Some(stored) = stored else {
                return Err(RevalidationError::NoLockedContribution {
                    adapter_id: v.lock.adapter_id.clone(),
                });
            };
            if &v.lock != stored {
                return Err(RevalidationError::Drift {
                    adapter_id: v.lock.adapter_id.clone(),
                });
            }
        }
        Ok(validated)
    }
}

fn upsert_record(
    records: &mut Vec<ActivationRecord>,
    record: ActivationRecord,
    previous_source: Option<&str>,
) {
    if let Some(existing) = records
        .iter_mut()
        .find(|r| r.source == record.source || previous_source == Some(r.source.as_str()))
    {
        existing.name = record.name;
        existing.source = record.source;
        existing.trusted = false;
        existing.enabled = false;
        return;
    }
    records.push(record);
}

fn build_trust_display(
    name: &str,
    lock: &PackageLockEntry,
    validated: &[ValidatedExecutableContribution],
) -> TrustDisplay {
    let version = lock
        .contributions
        .first()
        .map(|c| c.package_version.clone());
    let contributions = validated
        .iter()
        .map(|v| ContributionSummary {
            adapter_id: v.id.clone(),
            executable_rel_path: v.command.display().to_string(),
            executable_sha256: v.lock.executable_sha256.clone(),
            target: v.target.clone(),
            protocol: v.protocol.clone(),
        })
        .collect();
    TrustDisplay {
        name: name.to_string(),
        version,
        contributions,
    }
}

/// Validate a package's contributions and return the lock material to persist.
///
/// Called by `package add`. `package_source` selects the install scope: a
/// project-local package with executable contributions is rejected here
/// (`ProjectLocalExecutableContribution`).
pub fn validate_for_install(
    manifest: &PackageManifest,
    raw_manifest_bytes: &[u8],
    package_root: &Path,
    package_source: ContributionScope,
) -> Result<Vec<LockMaterial>, ContributionValidationError> {
    let validated = validate_executable_contributions(
        manifest,
        raw_manifest_bytes,
        package_root,
        package_source,
        host_target_triple(),
        host_opi_version(),
    )?;
    Ok(validated.into_iter().map(|v| v.lock).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_target_triple_is_a_known_triple_or_fallback() {
        // Catches a corrupted match arm: the result must be one of the six
        // supported triples or the explicit unknown fallback (which fails the
        // target gate closed). A wrong-but-non-empty string fails this.
        assert!(matches!(
            host_target_triple(),
            "x86_64-unknown-linux-gnu"
                | "aarch64-unknown-linux-gnu"
                | "x86_64-apple-darwin"
                | "aarch64-apple-darwin"
                | "x86_64-pc-windows-msvc"
                | "aarch64-pc-windows-msvc"
                | "unknown-unknown-unknown"
        ));
    }

    #[test]
    fn revalidation_error_detail_is_human_readable() {
        let e = RevalidationError::Drift {
            adapter_id: "opi-sandbox".into(),
        };
        assert!(e.detail().contains("opi-sandbox"));
        assert!(e.detail().contains("drift"));
    }

    #[test]
    fn install_always_resets_existing_record_to_untrusted_and_disabled() {
        let user = tempfile::tempdir().expect("user config");
        let activation = PackageActivationStore::global(user.path().to_path_buf());
        activation
            .write_records(&[ActivationRecord {
                name: "adapter".into(),
                source: "./adapter".into(),
                trusted: true,
                enabled: true,
            }])
            .expect("seed trusted record");

        activation
            .install("adapter", "./adapter", None, &["adapter".into()])
            .expect("reinstall record");

        assert_eq!(
            activation.read_records().expect("read record"),
            [ActivationRecord {
                name: "adapter".into(),
                source: "./adapter".into(),
                trusted: false,
                enabled: false,
            }]
        );
    }
}
