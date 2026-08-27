//! Crate-private admission contract for Phase 18 external execution locks.
//!
//! [`ExternalArtifactLock`] validates the committed static lock candidates
//! (`crates/opi-eval/external-locks/static/`) and admits the resolved lock a
//! later Linux materialization run produces against exactly those candidates.
//! Admission is pure validation of supplied bytes: it never executes the
//! materialization workflow, performs network access, or trusts a claim that
//! is not bound by digest.
//!
//! All identity digests are SHA-256 over LF-normalized bytes (`\r\n` replaced
//! by `\n`) so three-platform checkouts of one pinned artifact agree, matching
//! the repository's spec-ledger hashing convention. Hex identities are
//! lowercase; Git object identities are 40 hex characters; registry digests
//! use the `sha256:<64 hex>` reference form. Timestamps are canonical UTC
//! `YYYY-MM-DDTHH:MM:SSZ` strings, which compare chronologically as strings.
//!
//! This module is not part of the crate's provisional public entry seam: it is
//! consumed only inside this crate by runner and adapter modules.

use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use thiserror::Error;

/// Schema identity accepted for a committed static lock document.
pub(crate) const STATIC_LOCK_SCHEMA: &str = "phase18-external-lock/static/1";
/// Schema identity accepted for a run-produced resolved lock document.
pub(crate) const RESOLVED_LOCK_SCHEMA: &str = "phase18-external-lock/resolved/1";
/// Lock identity of the Phase 18 Linux x86_64 external execution lock.
pub(crate) const LOCK_ID: &str = "phase18-linux-x86_64";
/// Platform scope of this lock family.
pub(crate) const PLATFORM: &str = "linux-x86_64";

/// Fail-closed admission failures for an external lock document.
#[derive(Debug, Error)]
pub(crate) enum ExternalLockError {
    /// The document is not valid JSON.
    #[error("external lock document is not valid JSON: {0}")]
    Json(String),
    /// The document contains a field this contract does not define.
    #[error("unknown field in external lock document: {0}")]
    UnknownField(String),
    /// A required field is missing.
    #[error("missing field in external lock document: {0}")]
    MissingField(String),
    /// The schema identity is missing or unsupported.
    #[error("unsupported external lock schema: {0}")]
    UnsupportedSchema(String),
    /// A digest is not a lowercase 64-hex SHA-256 value.
    #[error("malformed SHA-256 digest for {field}: {value}")]
    MalformedDigest {
        /// Field the digest was found in.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// A registry digest is not in `sha256:<64 hex>` form.
    #[error("malformed registry digest for {field}: {value}")]
    MalformedRegistryDigest {
        /// Field the digest was found in.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// A Git identity is not 40 lowercase hex characters.
    #[error("malformed Git identity for {field}: {value}")]
    MalformedGitIdentity {
        /// Field the identity was found in.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// A field is empty where the contract requires content.
    #[error("empty field in external lock document: {0}")]
    Empty(&'static str),
    /// Two entries share an identity that must be unique.
    #[error("duplicate {kind}: {value}")]
    Duplicate {
        /// What kind of identity collided.
        kind: &'static str,
        /// The colliding value.
        value: String,
    },
    /// A closed vocabulary received an unknown value.
    #[error("invalid {field}: {value}")]
    InvalidValue {
        /// Field the value was found in.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// Two values that must agree do not.
    #[error("mismatch for {field}: expected {expected}, found {actual}")]
    Mismatch {
        /// Field that mismatched.
        field: &'static str,
        /// The value pinned by the static lock.
        expected: String,
        /// The value found in the resolved lock.
        actual: String,
    },
    /// The resolved artifact has expired.
    #[error("resolved lock expired at {expired} (checked at {now})")]
    Expired {
        /// Recorded expiry timestamp.
        expired: String,
        /// Caller-supplied check time.
        now: String,
    },
    /// A timestamp is not canonical UTC `YYYY-MM-DDTHH:MM:SSZ`.
    #[error("malformed canonical timestamp for {field}: {value}")]
    MalformedTimestamp {
        /// Field the timestamp was found in.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// A timestamp ordering invariant was violated.
    #[error("timestamp invariant violated: {detail}")]
    TimestampOrder {
        /// Human-readable invariant description.
        detail: String,
    },
    /// The resolved lock does not bind the static lock it claims.
    #[error("resolved lock does not bind its static lock: {0}")]
    StaticBinding(String),
    /// Producer identities do not satisfy the pinned producer set.
    #[error("producer binding violated: {0}")]
    ProducerBinding(String),
    /// An unresolved output slot is missing, unexpected, or misowned.
    #[error("unresolved slot contract violated: {0}")]
    UnresolvedSlots(String),
}

/// A validated static lock plus its LF-normalized byte digest.
///
/// Constructed only through [`ExternalArtifactLock::from_static_bytes`], which
/// fails closed on every schema, provenance, digest, and authority rule below.
#[derive(Debug, Clone)]
pub(crate) struct ExternalArtifactLock {
    doc: StaticLockDoc,
    digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StaticLockDoc {
    schema: String,
    lock_id: String,
    platform: String,
    authority: AuthorityDoc,
    subjects: Vec<SubjectDoc>,
    tools: Vec<ToolDoc>,
    images: Vec<ImageDoc>,
    closures: Vec<ClosureDoc>,
    adapter_policy: Vec<AdapterPolicyDoc>,
    workspace_input: WorkspaceInputDoc,
    unresolved: Vec<UnresolvedDoc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityDoc {
    trigger: String,
    admission: String,
    workflow: WorkflowPinDoc,
    producers: Vec<ProducerDoc>,
    actions: Vec<ActionDoc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowPinDoc {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProducerDoc {
    path: String,
    role: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionDoc {
    name: String,
    version: String,
    commit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubjectDoc {
    id: String,
    kind: String,
    repository_url: String,
    commit: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    version_anchor: Option<String>,
    #[serde(default)]
    tasks_tree: Option<String>,
    #[serde(default)]
    blobs: Vec<BlobDoc>,
    #[serde(default)]
    task: Option<TaskDoc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct BlobDoc {
    path: String,
    git_blob: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskDoc {
    id: String,
    tree: String,
    #[serde(default)]
    files: Vec<TaskFileDoc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskFileDoc {
    path: String,
    mode: String,
    size: u64,
    sha256: String,
    git_blob: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolDoc {
    id: String,
    kind: String,
    version: String,
    #[serde(default)]
    commit: Option<String>,
    #[serde(default)]
    archive: Option<ArchiveDoc>,
    #[serde(default)]
    uv_lock: Option<GitBlobPinDoc>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArchiveDoc {
    url: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GitBlobPinDoc {
    git_blob: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageDoc {
    id: String,
    role: String,
    reference: String,
    manifest: String,
    #[serde(default)]
    config: Option<String>,
    #[serde(default)]
    layers: Option<Vec<String>>,
    #[serde(default)]
    observed_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosureDoc {
    id: String,
    task_id: String,
    apt_epoch: String,
    indexes: Vec<AptIndexDoc>,
    packages: Vec<AptPackageDoc>,
    uv: UvClosureDoc,
    wheels: Vec<WheelDoc>,
    environment: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AptIndexDoc {
    suite: String,
    archive: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AptPackageDoc {
    name: String,
    version: String,
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct UvClosureDoc {
    installer: ArchiveDoc,
    archive: ArchiveDoc,
    uv_sha256: String,
    uvx_sha256: String,
    source_commit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WheelDoc {
    distribution: String,
    version: String,
    url: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterPolicyDoc {
    id: String,
    producer: String,
    identity: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceInputDoc {
    cargo_lock_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnresolvedDoc {
    id: String,
    owner_task: String,
}

/// An admitted resolved lock: the parsed document plus its digest.
#[derive(Debug, Clone)]
pub(crate) struct AdmittedLock {
    digest: String,
    expires_at: String,
}

impl AdmittedLock {
    /// LF-normalized SHA-256 digest of the admitted resolved bytes.
    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    /// Recorded expiry timestamp of the backing artifact.
    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedLockDoc {
    schema: String,
    lock_id: String,
    platform: String,
    resolved_by_task: String,
    static_lock: StaticBindingDoc,
    workflow: ResolvedWorkflowDoc,
    producers: Vec<ResolvedProducerDoc>,
    run: RunDoc,
    artifact: ArtifactDoc,
    resolved_at: String,
    images: Vec<PulledImageDoc>,
    closure: ResolvedClosureDoc,
    oracle: OracleDoc,
    resolved: Vec<ResolvedSlotDoc>,
    future: Vec<UnresolvedDoc>,
    authority: ResolvedAuthorityDoc,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StaticBindingDoc {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedWorkflowDoc {
    #[serde(rename = "ref")]
    reference: String,
    sha: String,
    path: String,
    bytes_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedProducerDoc {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunDoc {
    id: u64,
    attempt: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactDoc {
    name: String,
    id: u64,
    digest: String,
    url: String,
    expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PulledImageDoc {
    id: String,
    manifest: String,
    config: String,
    layers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedClosureDoc {
    manifest_sha256: String,
    file_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleDoc {
    status: String,
    reward: f64,
    ctrf_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedSlotDoc {
    id: String,
    identity: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolvedAuthorityDoc {
    admission: String,
}

impl ExternalArtifactLock {
    /// Validate a committed static lock document.
    ///
    /// Fails closed on an unsupported schema, a field outside the contract,
    /// a malformed digest or Git identity, a mutable-tag admission claim, a
    /// workflow pin outside `.github/workflows/`, a producer set without a
    /// materializer, duplicate identities, unknown closed-vocabulary values,
    /// a closure without the four offline uv environment controls, and
    /// unresolved slots without a well-formed owning task id.
    pub(crate) fn from_static_bytes(bytes: &[u8]) -> Result<Self, ExternalLockError> {
        let doc: StaticLockDoc =
            serde_json::from_slice(&normalize_lf(bytes)).map_err(map_json_error)?;
        validate_static(&doc)?;
        let digest = lf_sha256_hex(bytes);
        Ok(Self { doc, digest })
    }

    /// LF-normalized SHA-256 digest of the exact static bytes admitted here.
    pub(crate) fn static_digest(&self) -> &str {
        &self.digest
    }

    /// Pinned workflow path of the materialization authority.
    pub(crate) fn workflow_path(&self) -> &str {
        &self.doc.authority.workflow.path
    }

    /// Admit a run-produced resolved lock against this static contract.
    ///
    /// `now_utc` is the caller's check time in canonical UTC form; a resolved
    /// lock whose artifact expiry is not strictly in the future is rejected as
    /// stale. Admission validates structure and binding only: it does not
    /// re-download, re-hash, or re-execute anything.
    pub(crate) fn admit(
        &self,
        resolved_bytes: &[u8],
        now_utc: &str,
    ) -> Result<AdmittedLock, ExternalLockError> {
        let doc: ResolvedLockDoc =
            serde_json::from_slice(&normalize_lf(resolved_bytes)).map_err(map_json_error)?;
        self.validate_resolved(&doc, now_utc)?;
        Ok(AdmittedLock {
            digest: lf_sha256_hex(resolved_bytes),
            expires_at: doc.artifact.expires_at,
        })
    }

    fn validate_resolved(
        &self,
        doc: &ResolvedLockDoc,
        now_utc: &str,
    ) -> Result<(), ExternalLockError> {
        if doc.schema != RESOLVED_LOCK_SCHEMA {
            return Err(ExternalLockError::UnsupportedSchema(doc.schema.clone()));
        }
        if doc.lock_id != self.doc.lock_id {
            return Err(ExternalLockError::Mismatch {
                field: "lock_id",
                expected: self.doc.lock_id.clone(),
                actual: doc.lock_id.clone(),
            });
        }
        if doc.platform != self.doc.platform {
            return Err(ExternalLockError::Mismatch {
                field: "platform",
                expected: self.doc.platform.clone(),
                actual: doc.platform.clone(),
            });
        }
        if !is_task_owner(&doc.resolved_by_task) {
            return Err(ExternalLockError::UnresolvedSlots(format!(
                "resolved_by_task is not a Phase 18 task id: {}",
                doc.resolved_by_task
            )));
        }

        if doc.static_lock.sha256 != self.digest {
            return Err(ExternalLockError::StaticBinding(format!(
                "recorded static digest {} does not match the admitted static lock {}",
                doc.static_lock.sha256, self.digest
            )));
        }
        if doc.static_lock.path.trim().is_empty() {
            return Err(ExternalLockError::Empty("static_lock.path"));
        }

        if doc.workflow.path != self.doc.authority.workflow.path {
            return Err(ExternalLockError::Mismatch {
                field: "workflow.path",
                expected: self.doc.authority.workflow.path.clone(),
                actual: doc.workflow.path.clone(),
            });
        }
        if doc.workflow.bytes_sha256 != self.doc.authority.workflow.sha256 {
            return Err(ExternalLockError::Mismatch {
                field: "workflow.bytes_sha256",
                expected: self.doc.authority.workflow.sha256.clone(),
                actual: doc.workflow.bytes_sha256.clone(),
            });
        }
        if !doc.workflow.reference.starts_with("refs/") {
            return Err(ExternalLockError::InvalidValue {
                field: "workflow.ref",
                value: doc.workflow.reference.clone(),
            });
        }
        require_git_identity("workflow.sha", &doc.workflow.sha)?;

        if doc.producers.len() != self.doc.authority.producers.len() {
            return Err(ExternalLockError::ProducerBinding(format!(
                "producer count {} does not match the pinned {}",
                doc.producers.len(),
                self.doc.authority.producers.len()
            )));
        }
        for producer in &doc.producers {
            let Some(pinned) = self
                .doc
                .authority
                .producers
                .iter()
                .find(|p| p.path == producer.path)
            else {
                return Err(ExternalLockError::ProducerBinding(format!(
                    "producer {} is not pinned by the static lock",
                    producer.path
                )));
            };
            if producer.sha256 != pinned.sha256 {
                return Err(ExternalLockError::ProducerBinding(format!(
                    "producer {} digest {} does not match the pinned {}",
                    producer.path, producer.sha256, pinned.sha256
                )));
            }
        }

        if doc.run.id == 0 {
            return Err(ExternalLockError::InvalidValue {
                field: "run.id",
                value: doc.run.id.to_string(),
            });
        }
        if doc.run.attempt == 0 {
            return Err(ExternalLockError::InvalidValue {
                field: "run.attempt",
                value: doc.run.attempt.to_string(),
            });
        }

        if doc.artifact.name.trim().is_empty() {
            return Err(ExternalLockError::Empty("artifact.name"));
        }
        require_registry_digest("artifact.digest", &doc.artifact.digest)?;
        require_https("artifact.url", &doc.artifact.url)?;
        require_timestamp("artifact.expires_at", &doc.artifact.expires_at)?;
        require_timestamp("resolved_at", &doc.resolved_at)?;
        if doc.resolved_at >= doc.artifact.expires_at {
            return Err(ExternalLockError::TimestampOrder {
                detail: format!(
                    "resolved_at {} is not before expires_at {}",
                    doc.resolved_at, doc.artifact.expires_at
                ),
            });
        }
        require_timestamp("now", now_utc)?;
        if now_utc >= doc.artifact.expires_at.as_str() {
            return Err(ExternalLockError::Expired {
                expired: doc.artifact.expires_at.clone(),
                now: now_utc.to_owned(),
            });
        }

        for image in &doc.images {
            let Some(pinned) = self.doc.images.iter().find(|i| i.id == image.id) else {
                return Err(ExternalLockError::InvalidValue {
                    field: "images[].id",
                    value: image.id.clone(),
                });
            };
            if image.manifest != pinned.manifest {
                return Err(ExternalLockError::Mismatch {
                    field: "images[].manifest",
                    expected: pinned.manifest.clone(),
                    actual: image.manifest.clone(),
                });
            }
            require_registry_digest("images[].config", &image.config)?;
            if image.layers.is_empty() {
                return Err(ExternalLockError::Empty("images[].layers"));
            }
            for layer in &image.layers {
                require_registry_digest("images[].layers[]", layer)?;
            }
        }

        require_sha256("closure.manifest_sha256", &doc.closure.manifest_sha256)?;
        if doc.closure.file_count == 0 {
            return Err(ExternalLockError::InvalidValue {
                field: "closure.file_count",
                value: doc.closure.file_count.to_string(),
            });
        }

        require_value("oracle.status", &doc.oracle.status, "passed")?;
        if !doc.oracle.reward.is_finite() || !(0.0..=1.0).contains(&doc.oracle.reward) {
            return Err(ExternalLockError::InvalidValue {
                field: "oracle.reward",
                value: doc.oracle.reward.to_string(),
            });
        }
        require_sha256("oracle.ctrf_sha256", &doc.oracle.ctrf_sha256)?;
        require_value("authority.admission", &doc.authority.admission, "digest")?;

        let declared = &self.doc.unresolved;
        let mut resolved_ids: Vec<&str> = Vec::new();
        for slot in &doc.resolved {
            let Some(declared_slot) = declared.iter().find(|d| d.id == slot.id) else {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "resolved slot {} is not declared by the static lock",
                    slot.id
                )));
            };
            if declared_slot.owner_task != doc.resolved_by_task {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "resolved slot {} is owned by {}, not by the resolving task {}",
                    slot.id, declared_slot.owner_task, doc.resolved_by_task
                )));
            }
            if slot.identity.trim().is_empty() {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "resolved slot {} carries no identity",
                    slot.id
                )));
            }
            if resolved_ids.contains(&slot.id.as_str()) {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "resolved slot {} appears twice",
                    slot.id
                )));
            }
            resolved_ids.push(slot.id.as_str());
        }
        let mut future_ids: Vec<&str> = Vec::new();
        for slot in &doc.future {
            let Some(declared_slot) = declared.iter().find(|d| d.id == slot.id) else {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "future slot {} is not declared by the static lock",
                    slot.id
                )));
            };
            if declared_slot.owner_task != slot.owner_task {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "future slot {} declares owner {}, but the static lock pins {}",
                    slot.id, slot.owner_task, declared_slot.owner_task
                )));
            }
            if future_ids.contains(&slot.id.as_str()) {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "future slot {} appears twice",
                    slot.id
                )));
            }
            future_ids.push(slot.id.as_str());
        }
        for slot in declared {
            let in_resolved = resolved_ids.contains(&slot.id.as_str());
            let in_future = future_ids.contains(&slot.id.as_str());
            if in_resolved && in_future {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "slot {} appears in both resolved and future",
                    slot.id
                )));
            }
            if slot.owner_task == doc.resolved_by_task && !in_resolved {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "slot {} is owned by the resolving task {} but is not resolved",
                    slot.id, doc.resolved_by_task
                )));
            }
            if slot.owner_task != doc.resolved_by_task && !in_future {
                return Err(ExternalLockError::UnresolvedSlots(format!(
                    "slot {} is owned by {} but is not listed as future",
                    slot.id, slot.owner_task
                )));
            }
        }

        Ok(())
    }
}

fn normalize_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            i += 1;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn lf_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalize_lf(bytes));
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn map_json_error(error: serde_json::Error) -> ExternalLockError {
    let text = error.to_string();
    if text.contains("unknown field") {
        ExternalLockError::UnknownField(text)
    } else if text.contains("missing field") {
        ExternalLockError::MissingField(text)
    } else {
        ExternalLockError::Json(text)
    }
}

fn is_hex(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && is_hex(value.as_bytes())
}

fn is_sha256_digest_ref(value: &str) -> bool {
    value.len() == 71 && value.starts_with("sha256:") && is_sha256_hex(&value[7..])
}

fn is_git_hex(value: &str) -> bool {
    value.len() == 40 && is_hex(value.as_bytes())
}

fn is_task_owner(value: &str) -> bool {
    let mut parts = value.split('.');
    let phase = parts.next().unwrap_or_default();
    let first = parts.next().unwrap_or_default();
    let rest: Vec<&str> = parts.collect();
    !first.is_empty()
        && first.bytes().all(|b| b.is_ascii_digit())
        && rest
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && phase == "18"
        && rest.len() <= 1
}

fn require_sha256(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if is_sha256_hex(value) {
        Ok(())
    } else {
        Err(ExternalLockError::MalformedDigest {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_registry_digest(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if is_sha256_digest_ref(value) {
        Ok(())
    } else {
        Err(ExternalLockError::MalformedRegistryDigest {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_git_identity(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if is_git_hex(value) {
        Ok(())
    } else {
        Err(ExternalLockError::MalformedGitIdentity {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_value(
    field: &'static str,
    value: &str,
    expected: &str,
) -> Result<(), ExternalLockError> {
    if value == expected {
        Ok(())
    } else {
        Err(ExternalLockError::InvalidValue {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_https(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if value.starts_with("https://") {
        Ok(())
    } else {
        Err(ExternalLockError::InvalidValue {
            field,
            value: value.to_owned(),
        })
    }
}

fn require_workflow_path(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if value.starts_with(".github/workflows/")
        && value.ends_with(".yml")
        && !value.contains("..")
        && !value.starts_with('/')
    {
        Ok(())
    } else {
        Err(ExternalLockError::InvalidValue {
            field,
            value: value.to_owned(),
        })
    }
}

fn is_canonical_timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
        && b[11..13].iter().all(|c| c.is_ascii_digit())
        && b[14..16].iter().all(|c| c.is_ascii_digit())
        && b[17..19].iter().all(|c| c.is_ascii_digit())
        && {
            let year: u32 = value[0..4].parse().unwrap_or(0);
            let month: u32 = value[5..7].parse().unwrap_or(0);
            let day: u32 = value[8..10].parse().unwrap_or(0);
            let hour: u32 = value[11..13].parse().unwrap_or(99);
            let minute: u32 = value[14..16].parse().unwrap_or(99);
            let second: u32 = value[17..19].parse().unwrap_or(99);
            let leap =
                year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
            let days_in_month = match month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if leap {
                        29
                    } else {
                        28
                    }
                }
                _ => 0,
            };
            (1..=12).contains(&month)
                && day >= 1
                && day <= days_in_month
                && hour <= 23
                && minute <= 59
                && second <= 59
        }
}

fn require_timestamp(field: &'static str, value: &str) -> Result<(), ExternalLockError> {
    if is_canonical_timestamp(value) {
        Ok(())
    } else {
        Err(ExternalLockError::MalformedTimestamp {
            field,
            value: value.to_owned(),
        })
    }
}

fn validate_static(doc: &StaticLockDoc) -> Result<(), ExternalLockError> {
    if doc.schema != STATIC_LOCK_SCHEMA {
        return Err(ExternalLockError::UnsupportedSchema(doc.schema.clone()));
    }
    if doc.lock_id != LOCK_ID {
        return Err(ExternalLockError::Mismatch {
            field: "lock_id",
            expected: LOCK_ID.to_owned(),
            actual: doc.lock_id.clone(),
        });
    }
    if doc.platform != PLATFORM {
        return Err(ExternalLockError::Mismatch {
            field: "platform",
            expected: PLATFORM.to_owned(),
            actual: doc.platform.clone(),
        });
    }

    require_value(
        "authority.trigger",
        &doc.authority.trigger,
        "workflow_dispatch",
    )?;
    require_value("authority.admission", &doc.authority.admission, "digest")?;
    require_workflow_path("authority.workflow.path", &doc.authority.workflow.path)?;
    require_sha256("authority.workflow.sha256", &doc.authority.workflow.sha256)?;

    let mut producer_paths: Vec<&str> = Vec::new();
    let mut has_materializer = false;
    for producer in &doc.authority.producers {
        require_value(
            "authority.producers[].role",
            &producer.role,
            if producer.role == "materializer" || producer.role == "verifier" {
                &producer.role
            } else {
                "materializer|verifier"
            },
        )?;
        if producer.role == "materializer" {
            has_materializer = true;
        }
        require_sha256("authority.producers[].sha256", &producer.sha256)?;
        if producer.path.starts_with('/') || producer.path.trim().is_empty() {
            return Err(ExternalLockError::InvalidValue {
                field: "authority.producers[].path",
                value: producer.path.clone(),
            });
        }
        if producer_paths.contains(&producer.path.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "producer path",
                value: producer.path.clone(),
            });
        }
        producer_paths.push(producer.path.as_str());
    }
    if !has_materializer {
        return Err(ExternalLockError::InvalidValue {
            field: "authority.producers.materializer",
            value: "missing".to_owned(),
        });
    }

    let mut action_pins: Vec<&str> = Vec::new();
    for action in &doc.authority.actions {
        if !action.name.contains('/') {
            return Err(ExternalLockError::InvalidValue {
                field: "authority.actions[].name",
                value: action.name.clone(),
            });
        }
        if action.version.trim().is_empty() {
            return Err(ExternalLockError::Empty("authority.actions[].version"));
        }
        require_git_identity("authority.actions[].commit", &action.commit)?;
        let pin = format!("{}@{}", action.name, action.commit);
        if action_pins.contains(&pin.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "action pin",
                value: pin,
            });
        }
        action_pins.push(pin.leak() as &str);
    }

    if doc.subjects.is_empty() {
        return Err(ExternalLockError::Empty("subjects"));
    }
    let mut subject_ids: Vec<&str> = Vec::new();
    for subject in &doc.subjects {
        let kind_ok = matches!(subject.kind.as_str(), "agent-source" | "benchmark-source");
        if !kind_ok {
            return Err(ExternalLockError::InvalidValue {
                field: "subjects[].kind",
                value: subject.kind.clone(),
            });
        }
        require_https("subjects[].repository_url", &subject.repository_url)?;
        require_git_identity("subjects[].commit", &subject.commit)?;
        if let Some(tag) = &subject.tag
            && tag.trim().is_empty()
        {
            return Err(ExternalLockError::Empty("subjects[].tag"));
        }
        if let Some(anchor) = &subject.version_anchor {
            require_git_identity("subjects[].version_anchor", anchor)?;
        }
        if let Some(tree) = &subject.tasks_tree {
            require_git_identity("subjects[].tasks_tree", tree)?;
        }
        let mut blob_paths: Vec<&str> = Vec::new();
        for blob in &subject.blobs {
            require_git_identity("subjects[].blobs[].git_blob", &blob.git_blob)?;
            require_sha256("subjects[].blobs[].sha256", &blob.sha256)?;
            if blob_paths.contains(&blob.path.as_str()) {
                return Err(ExternalLockError::Duplicate {
                    kind: "subject blob path",
                    value: blob.path.clone(),
                });
            }
            blob_paths.push(blob.path.as_str());
        }
        if subject.kind == "benchmark-source" {
            let Some(task) = &subject.task else {
                return Err(ExternalLockError::MissingField(
                    "subjects[benchmark-source].task".to_owned(),
                ));
            };
            require_git_identity("subjects[].task.tree", &task.tree)?;
            let mut file_paths: Vec<&str> = Vec::new();
            for file in &task.files {
                if file.mode != "100644" && file.mode != "100755" {
                    return Err(ExternalLockError::InvalidValue {
                        field: "subjects[].task.files[].git mode",
                        value: file.mode.clone(),
                    });
                }
                require_sha256("subjects[].task.files[].sha256", &file.sha256)?;
                require_git_identity("subjects[].task.files[].git_blob", &file.git_blob)?;
                if file_paths.contains(&file.path.as_str()) {
                    return Err(ExternalLockError::Duplicate {
                        kind: "task file path",
                        value: file.path.clone(),
                    });
                }
                file_paths.push(file.path.as_str());
            }
        }
        if subject_ids.contains(&subject.id.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "subject id",
                value: subject.id.clone(),
            });
        }
        subject_ids.push(subject.id.as_str());
    }

    if doc.tools.is_empty() {
        return Err(ExternalLockError::Empty("tools"));
    }
    let mut tool_ids: Vec<&str> = Vec::new();
    for tool in &doc.tools {
        match tool.kind.as_str() {
            "node" | "uv" => {
                let Some(archive) = &tool.archive else {
                    return Err(ExternalLockError::MissingField(
                        "tools[node|uv].archive".to_owned(),
                    ));
                };
                require_https("tools[].archive.url", &archive.url)?;
                require_sha256("tools[].archive.sha256", &archive.sha256)?;
            }
            "harbor" | "pier" => {
                require_git_identity(
                    "tools[harbor|pier].commit",
                    &tool.commit.clone().unwrap_or_default(),
                )?;
                let Some(lock) = &tool.uv_lock else {
                    return Err(ExternalLockError::MissingField(
                        "tools[harbor|pier].uv_lock".to_owned(),
                    ));
                };
                require_git_identity("tools[].uv_lock.git_blob", &lock.git_blob)?;
                require_sha256("tools[].uv_lock.sha256", &lock.sha256)?;
            }
            _ => {
                return Err(ExternalLockError::InvalidValue {
                    field: "tools[].kind",
                    value: tool.kind.clone(),
                });
            }
        }
        if tool.version.trim().is_empty() {
            return Err(ExternalLockError::Empty("tools[].version"));
        }
        if tool_ids.contains(&tool.id.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "tool id",
                value: tool.id.clone(),
            });
        }
        tool_ids.push(tool.id.as_str());
    }

    if doc.images.is_empty() {
        return Err(ExternalLockError::Empty("images"));
    }
    let mut image_ids: Vec<&str> = Vec::new();
    for image in &doc.images {
        if !matches!(
            image.role.as_str(),
            "task" | "build-input" | "provenance-only" | "builder"
        ) {
            return Err(ExternalLockError::InvalidValue {
                field: "images[].role",
                value: image.role.clone(),
            });
        }
        require_registry_digest("images[].manifest", &image.manifest)?;
        if let Some(config) = &image.config {
            require_registry_digest("images[].config", config)?;
        }
        if let Some(layers) = &image.layers {
            if layers.is_empty() {
                return Err(ExternalLockError::Empty("images[].layers"));
            }
            for layer in layers {
                require_registry_digest("images[].layers[]", layer)?;
            }
        }
        if image_ids.contains(&image.id.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "image id",
                value: image.id.clone(),
            });
        }
        image_ids.push(image.id.as_str());
    }

    for closure in &doc.closures {
        if !(closure.apt_epoch.len() == 16
            && closure.apt_epoch.as_bytes()[8] == b'T'
            && closure.apt_epoch.as_bytes()[15] == b'Z'
            && closure.apt_epoch.as_bytes()[..8]
                .iter()
                .all(|b| b.is_ascii_digit())
            && closure.apt_epoch.as_bytes()[9..15]
                .iter()
                .all(|b| b.is_ascii_digit()))
        {
            return Err(ExternalLockError::InvalidValue {
                field: "closures[].apt_epoch",
                value: closure.apt_epoch.clone(),
            });
        }
        if closure.indexes.is_empty() {
            return Err(ExternalLockError::Empty("closures[].indexes"));
        }
        for index in &closure.indexes {
            if !matches!(index.archive.as_str(), "debian" | "debian-security") {
                return Err(ExternalLockError::InvalidValue {
                    field: "closures[].indexes[].archive",
                    value: index.archive.clone(),
                });
            }
            require_sha256("closures[].indexes[].sha256", &index.sha256)?;
        }
        let mut package_names: Vec<&str> = Vec::new();
        for package in &closure.packages {
            require_sha256("closures[].packages[].sha256", &package.sha256)?;
            if package_names.contains(&package.name.as_str()) {
                return Err(ExternalLockError::Duplicate {
                    kind: "apt package name",
                    value: package.name.clone(),
                });
            }
            package_names.push(package.name.as_str());
        }
        require_https("closures[].uv.installer.url", &closure.uv.installer.url)?;
        require_sha256(
            "closures[].uv.installer.sha256",
            &closure.uv.installer.sha256,
        )?;
        require_https("closures[].uv.archive.url", &closure.uv.archive.url)?;
        require_sha256("closures[].uv.archive.sha256", &closure.uv.archive.sha256)?;
        require_sha256("closures[].uv.uv_sha256", &closure.uv.uv_sha256)?;
        require_sha256("closures[].uv.uvx_sha256", &closure.uv.uvx_sha256)?;
        require_git_identity("closures[].uv.source_commit", &closure.uv.source_commit)?;
        let mut wheel_names: Vec<&str> = Vec::new();
        for wheel in &closure.wheels {
            require_https("closures[].wheels[].url", &wheel.url)?;
            require_sha256("closures[].wheels[].sha256", &wheel.sha256)?;
            if wheel_names.contains(&wheel.distribution.as_str()) {
                return Err(ExternalLockError::Duplicate {
                    kind: "wheel distribution",
                    value: wheel.distribution.clone(),
                });
            }
            wheel_names.push(wheel.distribution.as_str());
        }
        for control in [
            "UV_DOWNLOAD_URL",
            "UV_FIND_LINKS",
            "UV_OFFLINE",
            "UV_PYTHON_DOWNLOADS",
        ] {
            if !closure.environment.contains_key(control) {
                return Err(ExternalLockError::InvalidValue {
                    field: "closures[].environment",
                    value: format!("missing {control}"),
                });
            }
        }
    }

    for policy in &doc.adapter_policy {
        if !producer_paths.contains(&policy.producer.as_str()) {
            return Err(ExternalLockError::InvalidValue {
                field: "adapter_policy[].producer",
                value: policy.producer.clone(),
            });
        }
        require_value(
            "adapter_policy[].identity",
            &policy.identity,
            "receipt-recorded",
        )?;
    }

    require_sha256(
        "workspace_input.cargo_lock_sha256",
        &doc.workspace_input.cargo_lock_sha256,
    )?;

    if doc.unresolved.is_empty() {
        return Err(ExternalLockError::Empty("unresolved"));
    }
    let mut unresolved_ids: Vec<&str> = Vec::new();
    for slot in &doc.unresolved {
        if !is_task_owner(&slot.owner_task) {
            return Err(ExternalLockError::InvalidValue {
                field: "unresolved[].owner_task",
                value: slot.owner_task.clone(),
            });
        }
        if unresolved_ids.contains(&slot.id.as_str()) {
            return Err(ExternalLockError::Duplicate {
                kind: "unresolved id",
                value: slot.id.clone(),
            });
        }
        unresolved_ids.push(slot.id.as_str());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const VALID_STATIC: &str =
        include_str!("../tests/fixtures/external-locks/static/valid-static.json");

    /// Set a value inside a copy of the valid fixture by dotted path; numeric
    /// path segments index arrays.
    fn mutated(path: &str, value: Value) -> String {
        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        let segments: Vec<&str> = path.split('.').collect();
        let mut cursor = &mut doc;
        for (index, segment) in segments.iter().enumerate() {
            let last = index + 1 == segments.len();
            if let Ok(position) = segment.parse::<usize>() {
                cursor = cursor
                    .as_array_mut()
                    .expect("array parent")
                    .get_mut(position)
                    .expect("index in range");
            } else {
                cursor = cursor
                    .as_object_mut()
                    .expect("object parent")
                    .get_mut(*segment)
                    .expect("field exists");
            }
            if last {
                *cursor = value.clone();
            }
        }
        serde_json::to_string(&doc).expect("mutation serializes")
    }

    fn static_rejects(document: String) -> ExternalLockError {
        ExternalArtifactLock::from_static_bytes(document.as_bytes())
            .err()
            .expect("static lock must be rejected")
    }

    #[test]
    fn valid_static_fixture_admits_with_stable_digest() {
        let lock = ExternalArtifactLock::from_static_bytes(VALID_STATIC.as_bytes())
            .expect("valid fixture admits");
        assert_eq!(lock.static_digest(), lf_sha256_hex(VALID_STATIC.as_bytes()));
        assert_eq!(
            lock.workflow_path(),
            ".github/workflows/phase18-lock-materialization.yml"
        );
    }

    #[test]
    fn rejects_unsupported_schema() {
        let error = static_rejects(mutated(
            "schema",
            Value::String("phase18-external-lock/static/2".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::UnsupportedSchema(_)),
            "{error}"
        );
    }

    #[test]
    fn rejects_unknown_field() {
        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        doc.as_object_mut()
            .expect("root")
            .insert("surprise".to_owned(), Value::Bool(true));
        let error = static_rejects(serde_json::to_string(&doc).expect("serializes"));
        assert!(
            matches!(error, ExternalLockError::UnknownField(_)),
            "{error}"
        );
    }

    #[test]
    fn rejects_wrong_lock_identity_or_platform() {
        for path in ["lock_id", "platform"] {
            let error = static_rejects(mutated(path, Value::String("other".to_owned())));
            assert!(
                matches!(error, ExternalLockError::Mismatch { field, .. } if field.contains(path)),
                "{path}: {error}"
            );
        }
    }

    #[test]
    fn rejects_mutable_authority_admission() {
        let error = static_rejects(mutated(
            "authority.admission",
            Value::String("tag".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("admission")),
            "{error}"
        );
    }

    #[test]
    fn rejects_non_dispatch_trigger() {
        let error = static_rejects(mutated(
            "authority.trigger",
            Value::String("push".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("trigger")),
            "{error}"
        );
    }

    #[test]
    fn rejects_malformed_workflow_digest() {
        for value in [
            "sha256:aaaa".to_owned(), // registry form
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned(), // uppercase
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(), // 63 chars
        ] {
            let error = static_rejects(mutated(
                "authority.workflow.sha256",
                Value::String(value.clone()),
            ));
            assert!(
                matches!(error, ExternalLockError::MalformedDigest { .. }),
                "{value}: {error}"
            );
        }
    }

    #[test]
    fn rejects_workflow_pin_outside_workflows_directory() {
        for value in [
            "scripts/phase18-lock-materialization.yml".to_owned(),
            "/abs/.github/workflows/x.yml".to_owned(),
            ".github/workflows/../phase18.yml".to_owned(),
        ] {
            let error = static_rejects(mutated(
                "authority.workflow.path",
                Value::String(value.clone()),
            ));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("workflow path") || field.contains("path")),
                "{value}: {error}"
            );
        }
    }

    #[test]
    fn rejects_producer_set_without_materializer() {
        let error = static_rejects(mutated(
            "authority.producers.0.role",
            Value::String("verifier".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("materializer")),
            "{error}"
        );
    }

    #[test]
    fn rejects_duplicate_producer_path() {
        let error = static_rejects(mutated(
            "authority.producers.1.path",
            Value::String("scripts/phase18-materialize-locks.sh".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "producer path"),
            "{error}"
        );
    }

    #[test]
    fn rejects_mutable_action_pin() {
        let error = static_rejects(mutated(
            "authority.actions.0.commit",
            Value::String("v4.2.2".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedGitIdentity { .. }),
            "{error}"
        );
    }

    #[test]
    fn rejects_subject_identity_violations() {
        let error = static_rejects(mutated(
            "subjects.0.commit",
            Value::String("4e58f324".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedGitIdentity { .. }),
            "{error}"
        );

        let error = static_rejects(mutated(
            "subjects.0.repository_url",
            Value::String("http://github.com/earendil-works/pi".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { .. }),
            "{error}"
        );

        let error = static_rejects(mutated("subjects.1.id", Value::String("pi".to_owned())));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "subject id"),
            "{error}"
        );

        let error = static_rejects(mutated(
            "subjects.0.kind",
            Value::String("agent".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("kind")),
            "{error}"
        );
    }

    #[test]
    fn rejects_benchmark_subject_without_task() {
        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        doc.as_object_mut().expect("root")["subjects"]
            .as_array_mut()
            .expect("subjects")[1]
            .as_object_mut()
            .expect("subject")
            .remove("task");
        let error = static_rejects(serde_json::to_string(&doc).expect("serializes"));
        assert!(
            matches!(error, ExternalLockError::MissingField(_)),
            "{error}"
        );
    }

    #[test]
    fn rejects_task_file_contract_violations() {
        let error = static_rejects(mutated(
            "subjects.1.task.files.0.mode",
            Value::String("100666".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("git mode")),
            "{error}"
        );

        let error = static_rejects(mutated(
            "subjects.1.task.files.0.git_blob",
            Value::String("xyz".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedGitIdentity { .. }),
            "{error}"
        );

        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        let files = doc.as_object_mut().expect("root")["subjects"]
            .as_array_mut()
            .expect("subjects")[1]["task"]
            .as_object_mut()
            .expect("task")["files"]
            .as_array_mut()
            .expect("files");
        files.push(files[0].clone());
        let error = static_rejects(serde_json::to_string(&doc).expect("serializes"));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "task file path"),
            "{error}"
        );
    }

    #[test]
    fn rejects_tool_contract_violations() {
        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        doc.as_object_mut().expect("root")["tools"]
            .as_array_mut()
            .expect("tools")[0]
            .as_object_mut()
            .expect("tool")
            .remove("archive");
        let error = static_rejects(serde_json::to_string(&doc).expect("serializes"));
        assert!(
            matches!(error, ExternalLockError::MissingField(_)),
            "{error}"
        );

        let error = static_rejects(mutated(
            "tools.2.uv_lock.git_blob",
            Value::String("1c3995fe".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedGitIdentity { .. }),
            "{error}"
        );

        let error = static_rejects(mutated("tools.3.id", Value::String("harbor".to_owned())));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "tool id"),
            "{error}"
        );

        let error = static_rejects(mutated("tools.1.kind", Value::String("pip".to_owned())));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("kind")),
            "{error}"
        );
    }

    #[test]
    fn rejects_image_contract_violations() {
        let error = static_rejects(mutated(
            "images.0.manifest",
            Value::String(
                "4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12".to_owned(),
            ),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedRegistryDigest { .. }),
            "{error}"
        );

        let error = static_rejects(mutated(
            "images.0.role",
            Value::String("optional".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("role")),
            "{error}"
        );

        let error = static_rejects(mutated(
            "images.1.id",
            Value::String("tb21-openssl-selfsigned-cert-task".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "image id"),
            "{error}"
        );

        let error = static_rejects(mutated(
            "images.0.layers.0",
            Value::String(
                "040d34121c27906c4ff9ac152a30d52bf2c5d328d3bb748916bb3d2743c02528".to_owned(),
            ),
        ));
        assert!(
            matches!(error, ExternalLockError::MalformedRegistryDigest { .. }),
            "{error}"
        );
    }

    #[test]
    fn rejects_closure_contract_violations() {
        let error = static_rejects(mutated(
            "closures.0.apt_epoch",
            Value::String("2025-08-22".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("apt_epoch")),
            "{error}"
        );

        let error = static_rejects(mutated(
            "closures.0.packages.1.name",
            Value::String("curl".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "apt package name"),
            "{error}"
        );

        let mut doc: Value = serde_json::from_str(VALID_STATIC).expect("fixture parses");
        doc.as_object_mut().expect("root")["closures"]
            .as_array_mut()
            .expect("closures")[0]
            .as_object_mut()
            .expect("closure")["environment"]
            .as_object_mut()
            .expect("environment")
            .remove("UV_OFFLINE");
        let error = static_rejects(serde_json::to_string(&doc).expect("serializes"));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("environment")),
            "{error}"
        );
    }

    #[test]
    fn rejects_adapter_policy_without_pinned_producer() {
        let error = static_rejects(mutated(
            "adapter_policy.0.producer",
            Value::String("scripts/not-a-pinned-producer.sh".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("producer")),
            "{error}"
        );
    }

    #[test]
    fn rejects_unresolved_slot_contract_violations() {
        let error = static_rejects(mutated(
            "unresolved.0.owner_task",
            Value::String("18.x".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("owner_task")),
            "{error}"
        );

        let error = static_rejects(mutated(
            "unresolved.1.id",
            Value::String("materializer-tools".to_owned()),
        ));
        assert!(
            matches!(error, ExternalLockError::Duplicate { kind, .. } if kind == "unresolved id"),
            "{error}"
        );
    }

    #[test]
    fn rejects_empty_required_collections() {
        for path in ["subjects", "tools", "images", "unresolved"] {
            let error = static_rejects(mutated(path, Value::Array(Vec::new())));
            assert!(
                matches!(
                    error,
                    ExternalLockError::Empty(_) | ExternalLockError::MissingField(_)
                ),
                "{path}: {error}"
            );
        }
    }

    #[cfg(test)]
    mod resolved_tests {
        use super::*;
        use serde_json::Value;

        const VALID_RESOLVED: &str =
            include_str!("../tests/fixtures/external-locks/static/valid-resolved.json");
        const CHECK_NOW: &str = "2030-06-01T00:00:00Z";

        fn static_lock() -> ExternalArtifactLock {
            ExternalArtifactLock::from_static_bytes(VALID_STATIC.as_bytes())
                .expect("valid static fixture admits")
        }

        /// The committed fixture carries a placeholder static digest; bind it
        /// to the exact static fixture bytes before each admission.
        fn bound_resolved() -> String {
            let mut doc: Value = serde_json::from_str(VALID_RESOLVED).expect("fixture parses");
            doc["static_lock"]["sha256"] = Value::String(static_lock().static_digest().to_owned());
            serde_json::to_string(&doc).expect("serializes")
        }

        fn resolved_mutated(path: &str, value: Value) -> String {
            let mut doc: Value =
                serde_json::from_str(&bound_resolved()).expect("bound fixture parses");
            let segments: Vec<&str> = path.split('.').collect();
            let mut cursor = &mut doc;
            for (index, segment) in segments.iter().enumerate() {
                let last = index + 1 == segments.len();
                if let Ok(position) = segment.parse::<usize>() {
                    cursor = cursor
                        .as_array_mut()
                        .expect("array parent")
                        .get_mut(position)
                        .expect("index in range");
                } else {
                    cursor = cursor
                        .as_object_mut()
                        .expect("object parent")
                        .get_mut(*segment)
                        .expect("field exists");
                }
                if last {
                    *cursor = value.clone();
                }
            }
            serde_json::to_string(&doc).expect("mutation serializes")
        }

        fn admit_rejects(document: String) -> ExternalLockError {
            static_lock()
                .admit(document.as_bytes(), CHECK_NOW)
                .err()
                .expect("resolved lock must be rejected")
        }

        #[test]
        fn valid_resolved_fixture_admits_before_expiry() {
            let admitted = static_lock()
                .admit(bound_resolved().as_bytes(), CHECK_NOW)
                .expect("valid resolved fixture admits");
            assert_eq!(admitted.expires_at(), "2031-01-01T00:00:00Z");
            assert_eq!(
                admitted.digest(),
                lf_sha256_hex(bound_resolved().as_bytes())
            );
        }

        #[test]
        fn rejects_unsupported_resolved_schema() {
            let error = admit_rejects(resolved_mutated(
                "schema",
                Value::String("phase18-external-lock/resolved/2".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::UnsupportedSchema(_)),
                "{error}"
            );
        }

        #[test]
        fn rejects_static_binding_mismatch() {
            let error = admit_rejects(resolved_mutated(
                "static_lock.sha256",
                Value::String("0".repeat(64)),
            ));
            assert!(
                matches!(error, ExternalLockError::StaticBinding(_)),
                "{error}"
            );
        }

        #[test]
        fn rejects_workflow_drift_or_path_mismatch() {
            let error = admit_rejects(resolved_mutated(
                "workflow.bytes_sha256",
                Value::String("f".repeat(64)),
            ));
            assert!(
                matches!(error, ExternalLockError::Mismatch { field, .. } if field.contains("bytes_sha256")),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated(
                "workflow.path",
                Value::String(".github/workflows/other.yml".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::Mismatch { field, .. } if field.contains("path")),
                "{error}"
            );
        }

        #[test]
        fn rejects_producer_binding_violations() {
            let mut doc: Value =
                serde_json::from_str(&bound_resolved()).expect("bound fixture parses");
            doc["producers"].as_array_mut().expect("producers").pop();
            let error = admit_rejects(serde_json::to_string(&doc).expect("serializes"));
            assert!(
                matches!(error, ExternalLockError::ProducerBinding(_)),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated(
                "producers.0.sha256",
                Value::String("e".repeat(64)),
            ));
            assert!(
                matches!(error, ExternalLockError::ProducerBinding(_)),
                "{error}"
            );
        }

        #[test]
        fn rejects_expired_artifact() {
            let error = static_lock()
                .admit(bound_resolved().as_bytes(), "2031-06-01T00:00:00Z")
                .err()
                .expect("expired admission must be rejected");
            assert!(
                matches!(error, ExternalLockError::Expired { .. }),
                "{error}"
            );
        }

        #[test]
        fn rejects_malformed_or_unordered_timestamps() {
            for bad in [
                "2031-01-01 00:00:00Z".to_owned(),
                "2031-13-01T00:00:00Z".to_owned(),
                "2030-02-30T00:00:00Z".to_owned(),
                "2031-01-01T24:00:00Z".to_owned(),
            ] {
                let error = admit_rejects(resolved_mutated(
                    "artifact.expires_at",
                    Value::String(bad.clone()),
                ));
                assert!(
                    matches!(error, ExternalLockError::MalformedTimestamp { .. }),
                    "{bad}: {error}"
                );
            }

            let error = admit_rejects(resolved_mutated(
                "resolved_at",
                Value::String("2031-06-01T00:00:00Z".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::TimestampOrder { .. }),
                "{error}"
            );
        }

        #[test]
        fn rejects_invalid_now() {
            let error = static_lock()
                .admit(bound_resolved().as_bytes(), "yesterday")
                .err()
                .expect("malformed now must be rejected");
            assert!(
                matches!(error, ExternalLockError::MalformedTimestamp { .. }),
                "{error}"
            );
        }

        #[test]
        fn rejects_run_or_artifact_identity_violations() {
            let error = admit_rejects(resolved_mutated("run.id", Value::from(0u64)));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("run.id")),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated(
                "artifact.digest",
                Value::String("dddd".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::MalformedRegistryDigest { .. }),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated(
                "artifact.url",
                Value::String("http://github.com/example/opi".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("url")),
                "{error}"
            );
        }

        #[test]
        fn rejects_pulled_image_binding_violations() {
            let error = admit_rejects(resolved_mutated(
                "images.0.manifest",
                Value::String(format!("sha256:{}", "f".repeat(64))),
            ));
            assert!(
                matches!(error, ExternalLockError::Mismatch { field, .. } if field.contains("manifest")),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated(
                "images.0.id",
                Value::String("not-a-static-image".to_owned()),
            ));
            assert!(
                matches!(
                    error,
                    ExternalLockError::UnresolvedSlots(_) | ExternalLockError::InvalidValue { .. }
                ),
                "{error}"
            );

            let mut doc: Value =
                serde_json::from_str(&bound_resolved()).expect("bound fixture parses");
            doc["images"][0]["layers"]
                .as_array_mut()
                .expect("layers")
                .clear();
            let error = admit_rejects(serde_json::to_string(&doc).expect("serializes"));
            assert!(
                matches!(error, ExternalLockError::Empty("images[].layers")),
                "{error}"
            );
        }

        #[test]
        fn rejects_oracle_or_closure_violations() {
            let error = admit_rejects(resolved_mutated(
                "oracle.status",
                Value::String("failed".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("oracle.status")),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated("oracle.reward", Value::from(1.5f64)));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("reward")),
                "{error}"
            );

            let error = admit_rejects(resolved_mutated("closure.file_count", Value::from(0u64)));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("file_count")),
                "{error}"
            );
        }

        #[test]
        fn rejects_unresolved_partition_violations() {
            // A slot owned by the resolving task is missing from `resolved`.
            let mut doc: Value =
                serde_json::from_str(&bound_resolved()).expect("bound fixture parses");
            doc["resolved"].as_array_mut().expect("resolved").pop();
            let error = admit_rejects(serde_json::to_string(&doc).expect("serializes"));
            assert!(
                matches!(error, ExternalLockError::UnresolvedSlots(_)),
                "{error}"
            );

            // A future-owned slot appears in `resolved`.
            let mut doc: Value =
                serde_json::from_str(&bound_resolved()).expect("bound fixture parses");
            doc["resolved"]
                .as_array_mut()
                .expect("resolved")
                .push(serde_json::json!({
                    "id": "opi-executable",
                    "identity": "sha256:".to_owned() + &"9".repeat(64)
                }));
            doc["future"].as_array_mut().expect("future").pop();
            let error = admit_rejects(serde_json::to_string(&doc).expect("serializes"));
            assert!(
                matches!(error, ExternalLockError::UnresolvedSlots(_)),
                "{error}"
            );

            // An unknown slot id appears in `resolved`.
            let error = admit_rejects(resolved_mutated(
                "resolved.0.id",
                Value::String("never-declared".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::UnresolvedSlots(_)),
                "{error}"
            );

            // A future slot declares a different owner than the static lock.
            let error = admit_rejects(resolved_mutated(
                "future.0.owner_task",
                Value::String("18.14".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::UnresolvedSlots(_)),
                "{error}"
            );
        }

        #[test]
        fn rejects_wrong_resolved_by_task() {
            let error = admit_rejects(resolved_mutated(
                "resolved_by_task",
                Value::String("18.99".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::UnresolvedSlots(_)),
                "{error}"
            );
        }

        #[test]
        fn rejects_non_digest_resolved_authority() {
            let error = admit_rejects(resolved_mutated(
                "authority.admission",
                Value::String("tag".to_owned()),
            ));
            assert!(
                matches!(error, ExternalLockError::InvalidValue { field, .. } if field.contains("admission")),
                "{error}"
            );
        }
    }

    #[cfg(test)]
    mod committed_artifact_tests {
        use super::*;
        use std::path::{Path, PathBuf};

        fn repo_root() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("crate grandparent is the repository root")
                .to_path_buf()
        }

        fn committed_static_bytes() -> Vec<u8> {
            std::fs::read(
                repo_root().join("crates/opi-eval/external-locks/static/linux-x86_64.json"),
            )
            .expect("committed static lock exists")
        }

        #[test]
        fn committed_static_lock_validates() {
            let lock = ExternalArtifactLock::from_static_bytes(&committed_static_bytes())
                .expect("committed static lock validates");
            assert_eq!(
                lock.workflow_path(),
                ".github/workflows/phase18-lock-materialization.yml"
            );
        }

        #[test]
        fn committed_resolved_linux_lock_admits_against_the_static_contract() {
            let static_lock = ExternalArtifactLock::from_static_bytes(&committed_static_bytes())
                .expect("committed static lock validates");
            let resolved_bytes = std::fs::read(
                repo_root().join("crates/opi-eval/external-locks/resolved/linux-x86_64.json"),
            )
            .expect("committed resolved Linux lock exists");
            // The check time is the receipt's own resolution timestamp: a
            // deterministic instant strictly before the recorded artifact
            // expiry, so admission never depends on wall-clock state.
            let admitted = static_lock
                .admit(&resolved_bytes, "2026-08-27T08:15:23Z")
                .expect("committed resolved Linux lock admits");
            assert_eq!(admitted.expires_at(), "2026-09-26T08:15:23Z");
            assert_eq!(
                admitted.digest(),
                lf_sha256_hex(&resolved_bytes),
                "admitted digest must be the LF-normalized digest of the committed bytes"
            );

            // The committed run-produced receipt is durable re-audit evidence:
            // its recorded identities must agree with the admitted lock.
            let receipt: serde_json::Value = serde_json::from_str(include_str!(
                "../tests/fixtures/external-locks/materialization/receipt-linux-x86_64.json"
            ))
            .expect("materialization receipt fixture parses");
            let lock: serde_json::Value =
                serde_json::from_slice(&resolved_bytes).expect("resolved lock parses");
            assert_eq!(receipt["run"]["id"], lock["run"]["id"]);
            assert_eq!(
                receipt["candidate_commit"],
                serde_json::json!("f4648d90c5c2434cf825c0a0c615ebef9e757ed4")
            );
            assert_eq!(
                receipt["closure"]["manifest_sha256"],
                lock["closure"]["manifest_sha256"]
            );
            assert_eq!(
                receipt["oracle"]["ctrf_sha256"],
                lock["oracle"]["ctrf_sha256"]
            );
            assert_eq!(receipt["expires_at"], lock["artifact"]["expires_at"]);
            assert_eq!(
                receipt["images"][0]["manifest"],
                lock["images"][0]["manifest"]
            );
        }

        #[test]
        fn committed_workflow_and_producers_match_recorded_digests() {
            let lock = ExternalArtifactLock::from_static_bytes(&committed_static_bytes())
                .expect("committed static lock validates");
            let root = repo_root();
            let mut pinned: Vec<(String, String)> =
                vec![(lock.workflow_path().to_owned(), "workflow".to_owned())];
            // Re-read the pin list through the public fixture-independent path:
            // the committed lock's producers are validated by admission, so
            // assert their bytes directly against the recorded digests.
            let doc: serde_json::Value =
                serde_json::from_slice(&committed_static_bytes()).expect("committed lock parses");
            for producer in doc["authority"]["producers"]
                .as_array()
                .expect("producers pinned")
            {
                pinned.push((
                    producer["path"].as_str().expect("path").to_owned(),
                    producer["role"].as_str().expect("role").to_owned(),
                ));
            }
            assert!(!pinned.is_empty());
            let doc: serde_json::Value =
                serde_json::from_slice(&committed_static_bytes()).expect("committed lock parses");
            for (path, role) in pinned {
                let bytes = std::fs::read(root.join(&path))
                    .unwrap_or_else(|e| panic!("pinned {role} file {path} must exist: {e}"));
                let recorded = if role == "workflow" {
                    doc["authority"]["workflow"]["sha256"]
                        .as_str()
                        .expect("workflow digest recorded")
                        .to_owned()
                } else {
                    doc["authority"]["producers"]
                        .as_array()
                        .expect("producers")
                        .iter()
                        .find(|p| p["path"].as_str() == Some(path.as_str()))
                        .unwrap_or_else(|| panic!("producer {path} pinned"))["sha256"]
                        .as_str()
                        .expect("digest recorded")
                        .to_owned()
                };
                assert_eq!(
                    lf_sha256_hex(&bytes),
                    recorded,
                    "pinned {role} {path} drifted from the committed static lock"
                );
            }
        }
    }

    #[test]
    fn digest_helpers_reject_and_normalize() {
        assert!(!is_sha256_hex("AAAA"));
        assert!(!is_sha256_hex(&"a".repeat(63)));
        assert!(is_sha256_hex(&"a".repeat(64)));
        assert!(is_sha256_digest_ref(&format!("sha256:{}", "0".repeat(64))));
        assert!(!is_sha256_digest_ref("sha256:0123abcd"));
        assert!(!is_sha256_digest_ref("0123abcd"));
        assert_eq!(lf_sha256_hex(b"a\r\nb\r\n"), lf_sha256_hex(b"a\nb\n"));
    }
}
