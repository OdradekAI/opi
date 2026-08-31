//! Crate-private sealed trial bundle (Phase 18 task 18.5).
//!
//! A [`RunBundle`] is a staged, then sealed, artifact graph — never a shared
//! mutable database. It owns canonical sealing, mutation rejection, and
//! intent-before-effect persistence: the durable intent reservation is on
//! disk before the proof that admits the trial into its effect-pending phase
//! is handed out.

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::fs;

    thread_local! {
        static ATOMIC_WRITE_TRACE: RefCell<Option<Vec<&'static str>>> = const { RefCell::new(None) };
        static FAIL_PARENT_SYNC: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn begin_atomic_write_trace(fail_parent_sync: bool) {
        ATOMIC_WRITE_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        FAIL_PARENT_SYNC.with(|fail| fail.set(fail_parent_sync));
    }

    pub(super) fn record_atomic_write_event(event: &'static str) {
        ATOMIC_WRITE_TRACE.with(|trace| {
            if let Some(events) = trace.borrow_mut().as_mut() {
                events.push(event);
            }
        });
    }

    pub(super) fn parent_sync_should_fail() -> bool {
        FAIL_PARENT_SYNC.with(Cell::get)
    }

    fn take_atomic_write_trace() -> Vec<&'static str> {
        ATOMIC_WRITE_TRACE.with(|trace| trace.borrow_mut().take().unwrap_or_default())
    }

    fn spec(role: ArtifactRole, source: &str, path: &str, bytes: &[u8]) -> ArtifactSpec {
        ArtifactSpec {
            role,
            source: SourceIdentity::new(source).unwrap(),
            path: ArtifactKey::new(path).unwrap(),
            bytes: bytes.to_vec(),
            classification: Sensitivity::Exportable,
        }
    }

    #[test]
    fn typed_insertion_stages_role_source_digest_and_causal_edges() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();

        let stdout = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"hello\n",
                ),
                vec![],
            )
            .unwrap();
        assert_eq!(stdout, ArtifactKey::new("native/stdout.log").unwrap());
        let entry = bundle.entry(&stdout).unwrap();
        assert_eq!(entry.role, ArtifactRole::Native);
        assert_eq!(entry.source.as_str(), "agent-opi");
        assert_eq!(entry.digest, sha256_hex(b"hello\n"));
        assert!(entry.causal.is_empty());
        // The staged bytes are on disk inside the bundle root.
        assert!(tmp.path().join("artifacts/native/stdout.log").is_file());

        let report = bundle
            .insert(
                spec(
                    ArtifactRole::Derived,
                    "report",
                    "derived/report.md",
                    b"# report",
                ),
                vec![stdout.clone()],
            )
            .unwrap();
        let entry = bundle.entry(&report).unwrap();
        assert_eq!(entry.role, ArtifactRole::Derived);
        assert_eq!(entry.causal, vec![stdout]);
        assert_eq!(entry.classification, Sensitivity::Exportable);
    }

    #[test]
    fn insertion_rejects_escape_paths_oversize_dangling_edges_and_duplicates() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();

        // Logical-path grammar: absolute, parent-escape, and backslash keys
        // are rejected at construction, before any filesystem use.
        assert!(ArtifactKey::new("/etc/passwd").is_err());
        assert!(ArtifactKey::new("native/../escape").is_err());
        assert!(ArtifactKey::new("a//b").is_err());
        assert!(ArtifactKey::new("\\\\share\\x").is_err());
        assert!(ArtifactKey::new("native/stdout.log").is_ok());

        // Causal edges must reference already staged artifacts.
        let err = bundle
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "normalize",
                    "normalized/stdout",
                    b"x",
                ),
                vec![ArtifactKey::new("native/missing").unwrap()],
            )
            .unwrap_err();
        assert_eq!(err.boundary(), FailureBoundaryCode::Evidence);
        assert!(matches!(err, BundleError::UnknownCausalEdge { .. }));

        // Duplicate logical path is rejected.
        bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"one",
                ),
                vec![],
            )
            .unwrap();
        let err = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"two",
                ),
                vec![],
            )
            .unwrap_err();
        assert!(matches!(err, BundleError::DuplicateArtifact { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::Evidence);

        // Oversized artifacts are rejected, not truncated.
        let err = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/huge.bin",
                    &vec![0u8; MAX_ARTIFACT_BYTES + 1],
                ),
                vec![],
            )
            .unwrap_err();
        assert!(matches!(err, BundleError::ArtifactTooLarge { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::Evidence);
    }

    #[cfg(windows)]
    fn create_directory_alias(link: &Path, target: &Path) {
        let output = std::process::Command::new("cmd")
            .args([
                "/D",
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &target.to_string_lossy(),
            ])
            .output()
            .expect("spawn mklink for bundle ancestor regression");
        assert!(
            output.status.success(),
            "create bundle ancestor junction: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(unix)]
    fn create_directory_alias(link: &Path, target: &Path) {
        std::os::unix::fs::symlink(target, link).expect("create bundle ancestor symlink");
    }

    #[test]
    fn insertion_rejects_ancestor_directory_alias() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        fs::create_dir(tmp.path().join("artifacts")).unwrap();
        create_directory_alias(&tmp.path().join("artifacts").join("native"), outside.path());

        let err = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"must stay contained\n",
                ),
                vec![],
            )
            .unwrap_err();

        assert!(matches!(err, BundleError::SymlinkEscape { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::Evidence);
        assert!(!outside.path().join("stdout.log").exists());
    }

    #[test]
    fn intent_publication_requires_parent_directory_durability() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        let intent = IntentRecord {
            trial: TrialIdentity::new("trial-1").unwrap(),
            pair: PairIdentity::new("pair-1").unwrap(),
            artifacts: vec![ArtifactKey::new("normalized/expected").unwrap()],
            expected_output: ArtifactKey::new("normalized/expected").unwrap(),
        };

        begin_atomic_write_trace(false);
        bundle.publish_intent(&intent).unwrap();
        assert_eq!(
            take_atomic_write_trace(),
            vec!["write", "file-sync", "rename", "parent-sync"]
        );

        let failed_root = tempfile::tempdir().unwrap();
        let mut failed = RunBundle::create(failed_root.path()).unwrap();
        begin_atomic_write_trace(true);
        let err = failed.publish_intent(&intent).unwrap_err();
        assert!(matches!(err, BundleError::Io { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);
        assert_eq!(
            take_atomic_write_trace(),
            vec!["write", "file-sync", "rename", "parent-sync"]
        );
    }

    #[test]
    fn sealing_publishes_one_complete_manifest_and_freezes_the_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();

        // Sealing before a durable settlement is rejected: the ladder order
        // is intent -> settlement -> seal.
        let err = bundle.seal(&[]).unwrap_err();
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);

        bundle
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![
                    ArtifactKey::new("derived/report.md").unwrap(),
                    ArtifactKey::new("native/stdout.log").unwrap(),
                    ArtifactKey::new("normalized/expected").unwrap(),
                ],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap();
        let err = bundle.seal(&[]).unwrap_err();
        assert!(matches!(err, BundleError::SealWithoutSettlement { .. }));

        let stdout = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        let stdout_key = stdout.clone();
        bundle
            .insert(
                spec(ArtifactRole::Derived, "report", "derived/report.md", b"# r"),
                vec![stdout_key.clone()],
            )
            .unwrap();
        bundle
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "agent-opi",
                    "normalized/expected",
                    b"observed\n",
                ),
                vec![stdout_key],
            )
            .unwrap();
        bundle
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-1").unwrap(),
            })
            .unwrap();

        let receipt = bundle.seal(&[]).unwrap();
        assert!(!receipt.bundle_identity().is_empty());

        // One complete manifest is atomically visible under the final name.
        let manifest_path = tmp.path().join("manifest.json");
        assert!(manifest_path.is_file());
        let manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["format"], "opi-eval-bundle");
        assert_eq!(manifest["identity"], receipt.bundle_identity());
        assert_eq!(manifest["entries"]["native/stdout.log"]["role"], "Native");

        // The identity is content-addressed: identical content in a second
        // bundle seals to the same identity.
        let tmp2 = tempfile::tempdir().unwrap();
        let mut twin = RunBundle::create(tmp2.path()).unwrap();
        twin.publish_intent(&IntentRecord {
            trial: TrialIdentity::new("trial-1").unwrap(),
            pair: PairIdentity::new("pair-1").unwrap(),
            artifacts: vec![
                ArtifactKey::new("derived/report.md").unwrap(),
                ArtifactKey::new("native/stdout.log").unwrap(),
                ArtifactKey::new("normalized/expected").unwrap(),
            ],
            expected_output: ArtifactKey::new("normalized/expected").unwrap(),
        })
        .unwrap();
        let twin_stdout = twin
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        twin.insert(
            spec(ArtifactRole::Derived, "report", "derived/report.md", b"# r"),
            vec![twin_stdout.clone()],
        )
        .unwrap();
        twin.insert(
            spec(
                ArtifactRole::Normalized,
                "agent-opi",
                "normalized/expected",
                b"observed\n",
            ),
            vec![twin_stdout],
        )
        .unwrap();
        twin.record_settlement(&SettlementMarker {
            trial: TrialIdentity::new("trial-1").unwrap(),
        })
        .unwrap();
        let twin_receipt = twin.seal(&[]).unwrap();
        assert_eq!(receipt.bundle_identity(), twin_receipt.bundle_identity());

        // After sealing every mutation is rejected.
        let err = bundle
            .insert(
                spec(ArtifactRole::Native, "agent-opi", "native/late.log", b"x"),
                vec![],
            )
            .unwrap_err();
        assert!(matches!(err, BundleError::SealedMutation { .. }));
        let err = bundle
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap_err();
        assert!(matches!(err, BundleError::SealedMutation { .. }));
    }

    #[test]
    fn seal_time_tamper_fails_as_evidence_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        bundle
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![
                    ArtifactKey::new("native/stdout.log").unwrap(),
                    ArtifactKey::new("normalized/expected").unwrap(),
                ],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap();
        bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        bundle
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "agent-opi",
                    "normalized/expected",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        bundle
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-1").unwrap(),
            })
            .unwrap();

        // Tamper a staged byte between insertion and sealing: sealing is
        // artifact validation and owns the Evidence boundary.
        fs::write(
            tmp.path().join("artifacts/native/stdout.log"),
            b"tampered\n",
        )
        .unwrap();
        let err = bundle.seal(&[]).unwrap_err();
        assert!(matches!(err, BundleError::DigestMismatch { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::Evidence);
        // The partial bundle never published a manifest.
        assert!(!tmp.path().join("manifest.json").is_file());
    }

    #[test]
    fn sealing_enforces_reservation_equality_and_the_expected_output() {
        let tmp = tempfile::tempdir().unwrap();
        let mut bundle = RunBundle::create(tmp.path()).unwrap();
        bundle
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![
                    ArtifactKey::new("native/stdout.log").unwrap(),
                    ArtifactKey::new("normalized/expected").unwrap(),
                ],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap();
        bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        bundle
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-1").unwrap(),
            })
            .unwrap();

        // The reserved expected output was never staged: sealing refuses
        // the incomplete reservation (P18-BND-001).
        let err = bundle.seal(&[]).unwrap_err();
        assert!(matches!(err, BundleError::ReservationBroken { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);

        // Staging the expected output completes the reservation.
        bundle
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "agent-opi",
                    "normalized/expected",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        assert!(bundle.seal(&[]).is_ok());

        // A staged key outside the reservation and the declared produced
        // native evidence refuses sealing even when the rest is complete.
        let tmp2 = tempfile::tempdir().unwrap();
        let mut rogue = RunBundle::create(tmp2.path()).unwrap();
        rogue
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![
                    ArtifactKey::new("native/stdout.log").unwrap(),
                    ArtifactKey::new("normalized/expected").unwrap(),
                ],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap();
        rogue
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        rogue
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "agent-opi",
                    "normalized/expected",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        rogue
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/undeclared.log",
                    b"x\n",
                ),
                vec![],
            )
            .unwrap();
        rogue
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-1").unwrap(),
            })
            .unwrap();
        let err = rogue.seal(&[]).unwrap_err();
        assert!(matches!(err, BundleError::UnreservedArtifact { .. }));
        // Declaring the extra key as produced native evidence admits it.
        assert!(
            rogue
                .seal(&[ArtifactKey::new("native/undeclared.log").unwrap()])
                .is_ok()
        );
    }

    #[test]
    fn post_seal_mutation_invalidates_verification_without_repair() {
        let tmp = tempfile::tempdir().unwrap();
        let (receipt, bundle, stdout_key) = sealed_bundle(tmp.path());

        // A sealed, unmutated bundle re-verifies to the same identity.
        assert_eq!(RunBundle::verify(tmp.path()).unwrap(), receipt);

        // Mutating any covered byte after sealing invalidates verification
        // (P18-BND-003) as a post-seal mutation owned by TrialDurability.
        let artifact = tmp
            .path()
            .join(format!("artifacts/{}", stdout_key.as_str()));
        fs::write(&artifact, b"mutated\n").unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(err, BundleError::DigestMismatch { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);
        // Verification never repairs or rehashes: the same failure repeats.
        assert!(RunBundle::verify(tmp.path()).is_err());
        assert!(
            fs::read_to_string(tmp.path().join("manifest.json"))
                .unwrap()
                .contains(&receipt.bundle_identity().to_owned())
        );

        // A tampered manifest identity is also rejected, never rewritten.
        fs::write(&artifact, b"observed\n").unwrap();
        let mut tampered: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(tmp.path().join("manifest.json")).unwrap())
                .unwrap();
        tampered["identity"] = serde_json::json!("0".repeat(64));
        fs::write(
            tmp.path().join("manifest.json"),
            serde_json::to_string(&tampered).unwrap(),
        )
        .unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);
        assert!(bundle.entry(&stdout_key).is_none());
    }

    #[test]
    fn verify_rejects_unmanifested_files_sidecar_drift_and_missing_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let (receipt, _bundle, stdout_key) = sealed_bundle(tmp.path());
        assert_eq!(RunBundle::verify(tmp.path()).unwrap(), receipt);

        // An unmanifested file under the sealed artifact tree fails
        // verification as a retained-byte closure break.
        fs::write(tmp.path().join("artifacts/native/rogue.txt"), b"rogue\n").unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(err, BundleError::UnmanifestedFile { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);
        fs::remove_file(tmp.path().join("artifacts/native/rogue.txt")).unwrap();
        assert_eq!(RunBundle::verify(tmp.path()).unwrap(), receipt);

        // Durable sidecar drift: unparsable, semantically divergent, or
        // absent control sidecars break the sealed record even though the
        // manifest bytes are untouched.
        let manifest_bytes = fs::read(tmp.path().join("manifest.json")).unwrap();
        let intent_bytes = fs::read(tmp.path().join("intent.json")).unwrap();
        fs::write(tmp.path().join("intent.json"), b"not json").unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(
            err,
            BundleError::SidecarDrift {
                which: "intent",
                ..
            }
        ));
        let mut intent: serde_json::Value = serde_json::from_slice(&intent_bytes).unwrap();
        intent["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!("native/extra.log"));
        fs::write(
            tmp.path().join("intent.json"),
            serde_json::to_vec(&intent).unwrap(),
        )
        .unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(
            err,
            BundleError::SidecarDrift {
                which: "intent",
                ..
            }
        ));
        fs::write(tmp.path().join("intent.json"), &intent_bytes).unwrap();
        fs::remove_file(tmp.path().join("settlement.json")).unwrap();
        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(
            err,
            BundleError::SidecarDrift {
                which: "settlement",
                ..
            }
        ));
        assert_eq!(
            manifest_bytes,
            fs::read(tmp.path().join("manifest.json")).unwrap()
        );

        // A manifest entry whose artifact vanished fails verification.
        fs::remove_file(tmp.path().join("artifacts").join(stdout_key.as_str())).unwrap();
        assert!(RunBundle::verify(tmp.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn verify_rejects_symlinked_covered_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        let (_receipt, _bundle, stdout_key) = sealed_bundle(tmp.path());
        let artifact = tmp
            .path()
            .join(format!("artifacts/{}", stdout_key.as_str()));
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("outside.txt"), b"observed\n").unwrap();
        fs::remove_file(&artifact).unwrap();
        std::os::unix::fs::symlink(outside.path().join("outside.txt"), &artifact).unwrap();

        let err = RunBundle::verify(tmp.path()).unwrap_err();
        assert!(matches!(err, BundleError::SymlinkEscape { .. }));
        assert_eq!(err.boundary(), FailureBoundaryCode::TrialDurability);
    }

    fn sealed_bundle(root: &Path) -> (SealReceipt, RunBundle, ArtifactKey) {
        let mut bundle = RunBundle::create(root).unwrap();
        bundle
            .publish_intent(&IntentRecord {
                trial: TrialIdentity::new("trial-1").unwrap(),
                pair: PairIdentity::new("pair-1").unwrap(),
                artifacts: vec![
                    ArtifactKey::new("native/stdout.log").unwrap(),
                    ArtifactKey::new("normalized/expected").unwrap(),
                ],
                expected_output: ArtifactKey::new("normalized/expected").unwrap(),
            })
            .unwrap();
        let key = bundle
            .insert(
                spec(
                    ArtifactRole::Native,
                    "agent-opi",
                    "native/stdout.log",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        bundle
            .insert(
                spec(
                    ArtifactRole::Normalized,
                    "agent-opi",
                    "normalized/expected",
                    b"observed\n",
                ),
                vec![],
            )
            .unwrap();
        bundle
            .record_settlement(&SettlementMarker {
                trial: TrialIdentity::new("trial-1").unwrap(),
            })
            .unwrap();
        let receipt = bundle.seal(&[]).unwrap();
        (receipt, bundle, key)
    }
}

use crate::failure::FailureBoundaryCode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Durable identity of one trial. Created only from a non-empty trimmed
/// string; compared by value so identities can never be mixed by accident.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct TrialIdentity(String);

/// Durable identity of one Opi/pi pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct PairIdentity(String);

/// Stable logical key of one bundle artifact (workspace-relative logical
/// path, validated by this module before use).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(crate) struct ArtifactKey(String);

/// Identity of the producing source (Agent adapter, grader, runner, report).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SourceIdentity(String);

macro_rules! identity_newtype {
    ($name:ident, $what:literal) => {
        impl $name {
            /// Creates the identity from a non-empty trimmed string.
            pub(crate) fn new(raw: &str) -> Result<Self, IdentityError> {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err(IdentityError::Empty($what));
                }
                Ok(Self(trimmed.to_owned()))
            }
        }
    };
}

identity_newtype!(TrialIdentity, "trial identity");
identity_newtype!(PairIdentity, "pair identity");
identity_newtype!(SourceIdentity, "source identity");

impl TrialIdentity {
    /// The canonical identity string.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
impl SourceIdentity {
    /// The canonical identity string exposed only to bundle unit tests.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl ArtifactKey {
    /// Creates a logical artifact key from a workspace-relative path. The
    /// grammar rejects absolute paths, `..` and `.` components, empty
    /// components, and backslashes, so a key can never escape the staging
    /// root.
    pub(crate) fn new(raw: &str) -> Result<Self, IdentityError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(IdentityError::Empty("artifact key"));
        }
        if trimmed.contains('\\') {
            return Err(IdentityError::InvalidPath("artifact key"));
        }
        if trimmed.starts_with('/') {
            return Err(IdentityError::InvalidPath("artifact key"));
        }
        for segment in trimmed.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(IdentityError::InvalidPath("artifact key"));
            }
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The canonical key string.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// An identity string was empty or only whitespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityError {
    Empty(&'static str),
    /// A logical artifact path violated the staging-root grammar.
    InvalidPath(&'static str),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::Empty(what) => write!(f, "{what} must not be empty"),
            IdentityError::InvalidPath(what) => write!(
                f,
                "{what} must be workspace-relative with no escape or empty component"
            ),
        }
    }
}

/// The durable pre-effect reservation of trial, pair, artifact, and expected
/// output identities (P18-DUR-001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IntentRecord {
    pub(crate) trial: TrialIdentity,
    pub(crate) pair: PairIdentity,
    pub(crate) artifacts: Vec<ArtifactKey>,
    pub(crate) expected_output: ArtifactKey,
}

/// Durable marker that the observed outcome was recorded (P18-DUR-003).
/// Full evidence retention is written as bundle artifacts; this marker only
/// separates effect-unknown from settled during recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SettlementMarker {
    pub(crate) trial: TrialIdentity,
}

/// What a recovered bundle root proves about a crashed trial. Read from the
/// durable files only; no fact is inferred from absence of memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveryObservation {
    pub(crate) intent: Option<IntentRecord>,
    pub(crate) settlement: Option<SettlementMarker>,
    pub(crate) sealed: bool,
}

/// The execution role one artifact plays in the bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ArtifactRole {
    /// Captured directly from the supervised process.
    Native,
    /// Normalized cross-agent view derived from native artifacts.
    Normalized,
    /// Derived grade/report artifact addressed to a sealed bundle.
    Derived,
}

/// Required sensitivity classification of one artifact. Raw credentials,
/// unrestricted environment values, and private raw reasoning must never be
/// classified exportable (P18-BND-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Sensitivity {
    /// Safe to enter an exportable sealed bundle.
    Exportable,
    /// Machine-local only; excluded from any export.
    LocalOnly,
}

/// The typed insertion request: every artifact must declare its role, source
/// identity, logical path, bytes, and sensitivity classification up front.
pub(crate) struct ArtifactSpec {
    pub(crate) role: ArtifactRole,
    pub(crate) source: SourceIdentity,
    pub(crate) path: ArtifactKey,
    pub(crate) bytes: Vec<u8>,
    pub(crate) classification: Sensitivity,
}

/// The recorded view of one staged artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct ArtifactEntryView {
    pub(crate) role: ArtifactRole,
    pub(crate) source: SourceIdentity,
    pub(crate) digest: String,
    pub(crate) classification: Sensitivity,
    pub(crate) causal: Vec<ArtifactKey>,
}

/// One recorded staging entry (internal representation).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactEntry {
    role: ArtifactRole,
    source: SourceIdentity,
    digest: String,
    classification: Sensitivity,
    causal: Vec<ArtifactKey>,
}

/// Lowercase hex SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Receipt returned by canonical sealing. Carries the content-addressed
/// bundle identity for an outer receipt; a receipt is not an artifact spec
/// and cannot be inserted into the bundle it sealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SealReceipt {
    bundle_identity: String,
}

impl SealReceipt {
    /// The content-addressed identity of the sealed bundle.
    pub(crate) fn bundle_identity(&self) -> &str {
        &self.bundle_identity
    }
}

/// Manifest body: every durable fact the identity covers. Serialization is
/// canonical because `entries` is a `BTreeMap` and field order is fixed by
/// serde.
#[derive(Serialize, Deserialize)]
struct ManifestBody {
    format: String,
    version: u32,
    intent: IntentRecord,
    settlement: SettlementMarker,
    entries: BTreeMap<ArtifactKey, ArtifactEntry>,
}

/// The published manifest: the body plus its content-addressed identity.
#[derive(Serialize, Deserialize)]
struct Manifest {
    #[serde(flatten)]
    body: ManifestBody,
    identity: String,
}

/// Proof that a durable intent reservation exists on disk. Constructible
/// only by [`RunBundle::publish_intent`], so the lifecycle cannot enter the
/// effect-pending phase without it.
/// Canonical intent proof.
#[derive(Debug)]
pub(crate) struct DurableIntentProof {
    _private: (),
}

/// Typed bundle failure. Every failure carries the owning boundary.
#[derive(Debug)]
pub(crate) enum BundleError {
    /// The intent reservation already exists; one bundle reserves once.
    IntentAlreadyReserved { boundary: FailureBoundaryCode },
    /// A settlement was recorded before any intent existed.
    IntentNotPublished { boundary: FailureBoundaryCode },
    /// Settlement was already durably recorded for this bundle.
    SettlementAlreadyRecorded { boundary: FailureBoundaryCode },
    /// Sealing was attempted before the intent was durably reserved.
    SealWithoutIntent { boundary: FailureBoundaryCode },
    /// Sealing was attempted before a settlement was durably recorded.
    SealWithoutSettlement { boundary: FailureBoundaryCode },
    /// A staged artifact no longer matches its recorded digest.
    DigestMismatch {
        boundary: FailureBoundaryCode,
        key: ArtifactKey,
    },
    /// A covered path is a symlink; the bundle never follows symlinks.
    SymlinkEscape {
        boundary: FailureBoundaryCode,
        key: ArtifactKey,
    },
    /// The sealed manifest is unparsable or its identity does not cover its
    /// own body.
    ManifestInvalid { boundary: FailureBoundaryCode },
    /// A reserved artifact identity was not staged at sealing time, or the
    /// reserved expected output is absent: the pre-effect reservation and
    /// the staged set disagree.
    ReservationBroken {
        boundary: FailureBoundaryCode,
        key: ArtifactKey,
    },
    /// A staged artifact is covered by neither the reservation nor the
    /// declared produced native evidence.
    UnreservedArtifact {
        boundary: FailureBoundaryCode,
        key: ArtifactKey,
    },
    /// A manifest entry has no artifact on disk, or a reserved artifact
    /// entry is absent from the manifest.
    MissingArtifact {
        boundary: FailureBoundaryCode,
        key: ArtifactKey,
    },
    /// A durable intent or settlement sidecar diverges from the sealed
    /// manifest's own record.
    SidecarDrift {
        boundary: FailureBoundaryCode,
        which: &'static str,
    },
    /// A file exists under the sealed artifact tree without a manifest
    /// entry.
    UnmanifestedFile {
        boundary: FailureBoundaryCode,
        path: String,
    },
    /// The bundle is sealed and immutable.
    SealedMutation { boundary: FailureBoundaryCode },
    /// An artifact exceeded the staging size bound.
    ArtifactTooLarge {
        boundary: FailureBoundaryCode,
        size: usize,
    },
    /// The same logical path was inserted twice while staging.
    DuplicateArtifact { boundary: FailureBoundaryCode },
    /// A causal edge referenced an artifact that is not staged yet.
    UnknownCausalEdge {
        boundary: FailureBoundaryCode,
        edge: ArtifactKey,
    },
    /// A durable file operation failed.
    Io {
        boundary: FailureBoundaryCode,
        source: io::Error,
    },
}

impl BundleError {
    /// The owning failure boundary for this error.
    pub(crate) fn boundary(&self) -> FailureBoundaryCode {
        match self {
            BundleError::IntentAlreadyReserved { boundary }
            | BundleError::IntentNotPublished { boundary }
            | BundleError::SettlementAlreadyRecorded { boundary }
            | BundleError::SealWithoutIntent { boundary }
            | BundleError::SealWithoutSettlement { boundary }
            | BundleError::SealedMutation { boundary }
            | BundleError::ArtifactTooLarge { boundary, .. }
            | BundleError::DuplicateArtifact { boundary }
            | BundleError::UnknownCausalEdge { boundary, .. }
            | BundleError::DigestMismatch { boundary, .. }
            | BundleError::SymlinkEscape { boundary, .. }
            | BundleError::ManifestInvalid { boundary }
            | BundleError::ReservationBroken { boundary, .. }
            | BundleError::UnreservedArtifact { boundary, .. }
            | BundleError::MissingArtifact { boundary, .. }
            | BundleError::SidecarDrift { boundary, .. }
            | BundleError::UnmanifestedFile { boundary, .. }
            | BundleError::Io { boundary, .. } => *boundary,
        }
    }
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::IntentAlreadyReserved { .. } => {
                write!(f, "intent already reserved for this bundle")
            }
            BundleError::IntentNotPublished { .. } => {
                write!(f, "settlement recorded before any intent was published")
            }
            BundleError::SettlementAlreadyRecorded { .. } => {
                write!(f, "settlement already recorded for this bundle")
            }
            BundleError::SealWithoutIntent { .. } => {
                write!(f, "sealing requires a published intent")
            }
            BundleError::SealWithoutSettlement { .. } => {
                write!(f, "sealing requires a recorded settlement")
            }
            BundleError::DigestMismatch { key, .. } => {
                write!(f, "artifact {} no longer matches its digest", key.as_str())
            }
            BundleError::SymlinkEscape { key, .. } => {
                write!(f, "artifact path {} is a symlink", key.as_str())
            }
            BundleError::ManifestInvalid { .. } => {
                write!(f, "sealed manifest is invalid or does not cover its body")
            }
            BundleError::ReservationBroken { key, .. } => {
                write!(
                    f,
                    "reserved artifact {} was not staged before sealing",
                    key.as_str()
                )
            }
            BundleError::UnreservedArtifact { key, .. } => {
                write!(
                    f,
                    "staged artifact {} is not covered by the reservation",
                    key.as_str()
                )
            }
            BundleError::MissingArtifact { key, .. } => {
                write!(f, "manifest artifact {} is missing on disk", key.as_str())
            }
            BundleError::SidecarDrift { which, .. } => {
                write!(
                    f,
                    "durable {which} sidecar diverges from the sealed manifest"
                )
            }
            BundleError::UnmanifestedFile { path, .. } => {
                write!(
                    f,
                    "unmanifested file {path} inside the sealed artifact tree"
                )
            }
            BundleError::SealedMutation { .. } => {
                write!(f, "bundle is sealed and rejects mutation")
            }
            BundleError::ArtifactTooLarge { size, .. } => {
                write!(f, "artifact of {size} bytes exceeds the staging size bound")
            }
            BundleError::DuplicateArtifact { .. } => {
                write!(f, "artifact path already staged in this bundle")
            }
            BundleError::UnknownCausalEdge { edge, .. } => {
                write!(
                    f,
                    "causal edge references unstaged artifact {}",
                    edge.as_str()
                )
            }
            BundleError::Io { source, .. } => write!(f, "bundle i/o failure: {source}"),
        }
    }
}

enum BundleState {
    Staging {
        intent: Option<IntentRecord>,
        settlement: Option<SettlementMarker>,
        entries: BTreeMap<ArtifactKey, ArtifactEntry>,
    },
    Sealed,
}

/// Upper bound for one staged artifact. Oversized artifacts are rejected at
/// insertion rather than silently truncated.
const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

/// A staged or sealed trial bundle rooted at one directory.
pub(crate) struct RunBundle {
    root: PathBuf,
    state: BundleState,
}

impl RunBundle {
    /// Creates a fresh staging bundle rooted at `root`.
    pub(crate) fn create(root: &Path) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            state: BundleState::Staging {
                intent: None,
                settlement: None,
                entries: BTreeMap::new(),
            },
        })
    }

    /// Stages one artifact under its typed role, source identity, digest,
    /// classification, and causal edges. Causal edges must reference already
    /// staged artifacts; the bytes are written atomically inside the staging
    /// root. Rejected after sealing.
    pub(crate) fn insert(
        &mut self,
        spec: ArtifactSpec,
        causal: Vec<ArtifactKey>,
    ) -> Result<ArtifactKey, BundleError> {
        let BundleState::Staging { entries, .. } = &mut self.state else {
            return Err(BundleError::SealedMutation {
                boundary: FailureBoundaryCode::TrialDurability,
            });
        };
        if spec.bytes.len() > MAX_ARTIFACT_BYTES {
            return Err(BundleError::ArtifactTooLarge {
                boundary: FailureBoundaryCode::Evidence,
                size: spec.bytes.len(),
            });
        }
        if entries.contains_key(&spec.path) {
            return Err(BundleError::DuplicateArtifact {
                boundary: FailureBoundaryCode::Evidence,
            });
        }
        for edge in &causal {
            if !entries.contains_key(edge) {
                return Err(BundleError::UnknownCausalEdge {
                    boundary: FailureBoundaryCode::Evidence,
                    edge: edge.clone(),
                });
            }
        }
        let digest = sha256_hex(&spec.bytes);
        let target = self.artifact_path(&spec.path, FailureBoundaryCode::Evidence, true)?;
        let BundleState::Staging { entries, .. } = &mut self.state else {
            unreachable!("sealed state rejected above")
        };
        atomic_write(&target, &spec.bytes)?;
        entries.insert(
            spec.path.clone(),
            ArtifactEntry {
                role: spec.role,
                source: spec.source,
                digest,
                classification: spec.classification,
                causal,
            },
        );
        Ok(spec.path)
    }

    /// The recorded role, source, digest, classification, and causal edges
    /// of one staged artifact.
    #[cfg(test)]
    pub(crate) fn entry(&self, key: &ArtifactKey) -> Option<ArtifactEntryView> {
        match &self.state {
            BundleState::Staging { entries, .. } => entries.get(key).map(|e| ArtifactEntryView {
                role: e.role,
                source: e.source.clone(),
                digest: e.digest.clone(),
                classification: e.classification,
                causal: e.causal.clone(),
            }),
            BundleState::Sealed => None,
        }
    }

    /// Seals the bundle: re-reads every staged artifact and rejects any
    /// digest mismatch or symlink, then enforces the reservation closure —
    /// the staged key set must equal the reserved artifact identities plus
    /// exactly the declared produced native evidence, and the reserved
    /// expected output must be staged — and finally atomically publishes
    /// one complete manifest carrying the content-addressed bundle
    /// identity (P18-DUR-004, P18-BND-001). `produced` names the
    /// adapter-produced native evidence keys that materialized with their
    /// records (agent completion artifacts, verifier streams, and the
    /// native grader report); every other staged key must be reserved.
    /// Requires a published intent and a recorded settlement. After
    /// sealing, every mutation is rejected and the returned receipt is for
    /// an outer record only — it cannot enter the sealed content.
    pub(crate) fn seal(&mut self, produced: &[ArtifactKey]) -> Result<SealReceipt, BundleError> {
        let BundleState::Staging {
            intent,
            settlement,
            entries,
        } = &self.state
        else {
            return Err(BundleError::SealedMutation {
                boundary: FailureBoundaryCode::TrialDurability,
            });
        };
        let Some(intent) = intent else {
            return Err(BundleError::SealWithoutIntent {
                boundary: FailureBoundaryCode::TrialDurability,
            });
        };
        let Some(settlement) = settlement else {
            return Err(BundleError::SealWithoutSettlement {
                boundary: FailureBoundaryCode::TrialDurability,
            });
        };
        for key in entries.keys() {
            let bytes = self.read_covered(key, FailureBoundaryCode::Evidence)?;
            if sha256_hex(&bytes) != entries[key].digest {
                return Err(BundleError::DigestMismatch {
                    boundary: FailureBoundaryCode::Evidence,
                    key: key.clone(),
                });
            }
        }
        // Reservation equality: staged == reserved + declared produced
        // native evidence, and the expected output exists among them.
        let staged: BTreeSet<&ArtifactKey> = entries.keys().collect();
        let allowed: BTreeSet<&ArtifactKey> = intent.artifacts.iter().chain(produced).collect();
        if let Some(key) = staged.difference(&allowed).next() {
            return Err(BundleError::UnreservedArtifact {
                boundary: FailureBoundaryCode::TrialDurability,
                key: (*key).clone(),
            });
        }
        if let Some(key) = intent.artifacts.iter().find(|key| !staged.contains(key)) {
            return Err(BundleError::ReservationBroken {
                boundary: FailureBoundaryCode::TrialDurability,
                key: key.clone(),
            });
        }
        if let Some(key) = produced.iter().find(|key| !staged.contains(key)) {
            return Err(BundleError::ReservationBroken {
                boundary: FailureBoundaryCode::TrialDurability,
                key: key.clone(),
            });
        }
        if !staged.contains(&intent.expected_output) {
            return Err(BundleError::ReservationBroken {
                boundary: FailureBoundaryCode::TrialDurability,
                key: intent.expected_output.clone(),
            });
        }
        let body = ManifestBody {
            format: "opi-eval-bundle".to_owned(),
            version: 1,
            intent: intent.clone(),
            settlement: settlement.clone(),
            entries: entries.clone(),
        };
        let identity = serde_json::to_vec(&body)
            .map(|canonical| sha256_hex(&canonical))
            .map_err(io::Error::other)?;
        let manifest = Manifest {
            body,
            identity: identity.clone(),
        };
        let bytes = serde_json::to_vec(&manifest).map_err(io::Error::other)?;
        atomic_write(&self.root.join("manifest.json"), &bytes)?;
        self.state = BundleState::Sealed;
        Ok(SealReceipt {
            bundle_identity: identity,
        })
    }

    /// Re-verifies a sealed bundle root without mutating it. Recomputes
    /// the manifest identity from the covered body, compares the
    /// manifest's intent and settlement with the durable sidecars
    /// (`intent.json`, `settlement.json`), requires the reserved artifacts
    /// and the expected output among the entries, enumerates the artifact
    /// tree so every retained byte is manifest-covered (rejecting
    /// unmanifested, missing, non-file, or digest-mismatched entries), and
    /// fails on any mutation. It never repairs, rehashes, or rewrites
    /// (P18-BND-003); post-seal mutations own the TrialDurability
    /// boundary.
    pub(crate) fn verify(root: &Path) -> Result<SealReceipt, BundleError> {
        let text =
            fs::read_to_string(root.join("manifest.json")).map_err(|source| BundleError::Io {
                boundary: FailureBoundaryCode::TrialDurability,
                source,
            })?;
        let manifest: Manifest =
            serde_json::from_str(&text).map_err(|_| BundleError::ManifestInvalid {
                boundary: FailureBoundaryCode::TrialDurability,
            })?;
        let canonical =
            serde_json::to_vec(&manifest.body).map_err(|_| BundleError::ManifestInvalid {
                boundary: FailureBoundaryCode::TrialDurability,
            })?;
        if sha256_hex(&canonical) != manifest.identity {
            return Err(BundleError::ManifestInvalid {
                boundary: FailureBoundaryCode::TrialDurability,
            });
        }
        // Durable sidecar equality: the sealed manifest must agree with the
        // reservations on disk, byte-for-byte in semantics.
        let sidecar_intent = read_sidecar::<IntentRecord>(root, "intent.json")?;
        if sidecar_intent != manifest.body.intent {
            return Err(BundleError::SidecarDrift {
                boundary: FailureBoundaryCode::TrialDurability,
                which: "intent",
            });
        }
        let sidecar_settlement = read_sidecar::<SettlementMarker>(root, "settlement.json")?;
        if sidecar_settlement != manifest.body.settlement {
            return Err(BundleError::SidecarDrift {
                boundary: FailureBoundaryCode::TrialDurability,
                which: "settlement",
            });
        }
        let entries = &manifest.body.entries;
        if !entries.contains_key(&manifest.body.intent.expected_output) {
            return Err(BundleError::MissingArtifact {
                boundary: FailureBoundaryCode::TrialDurability,
                key: manifest.body.intent.expected_output.clone(),
            });
        }
        for key in &manifest.body.intent.artifacts {
            if !entries.contains_key(key) {
                return Err(BundleError::MissingArtifact {
                    boundary: FailureBoundaryCode::TrialDurability,
                    key: key.clone(),
                });
            }
        }
        // Retained-byte closure: every file under the artifact tree is a
        // manifest entry, and every manifest entry has its file.
        let mut present: Vec<String> = Vec::new();
        collect_artifact_files(&root.join("artifacts"), Path::new(""), &mut present)?;
        present.sort();
        present.dedup();
        for path in &present {
            if !entries.keys().any(|key| key.as_str() == *path) {
                return Err(BundleError::UnmanifestedFile {
                    boundary: FailureBoundaryCode::TrialDurability,
                    path: path.clone(),
                });
            }
        }
        for key in entries.keys() {
            if !present.iter().any(|path| path == key.as_str()) {
                return Err(BundleError::MissingArtifact {
                    boundary: FailureBoundaryCode::TrialDurability,
                    key: key.clone(),
                });
            }
        }
        let bundle = RunBundle {
            root: root.to_path_buf(),
            state: BundleState::Sealed,
        };
        for key in manifest.body.entries.keys() {
            let bytes = bundle.read_covered(key, FailureBoundaryCode::TrialDurability)?;
            let recorded = &manifest.body.entries[key].digest;
            if sha256_hex(&bytes) != *recorded {
                return Err(BundleError::DigestMismatch {
                    boundary: FailureBoundaryCode::TrialDurability,
                    key: key.clone(),
                });
            }
        }
        Ok(SealReceipt {
            bundle_identity: manifest.identity,
        })
    }

    /// Reads one covered artifact, refusing to follow symlinks. The caller
    /// owns the failure boundary: seal-time validation is Evidence,
    /// post-seal verification is TrialDurability.
    fn read_covered(
        &self,
        key: &ArtifactKey,
        boundary: FailureBoundaryCode,
    ) -> Result<Vec<u8>, BundleError> {
        let path = self.artifact_path(key, boundary, false)?;
        let meta = fs::symlink_metadata(&path).map_err(BundleError::from)?;
        if !meta.is_file() {
            return Err(BundleError::SymlinkEscape {
                boundary,
                key: key.clone(),
            });
        }
        fs::read(&path).map_err(BundleError::from)
    }

    /// Absolute on-disk location of one logical artifact path. The key
    /// grammar already excludes escapes; this returns the nested location
    /// under the reserved `artifacts` directory.
    fn artifact_path(
        &self,
        key: &ArtifactKey,
        boundary: FailureBoundaryCode,
        create_missing: bool,
    ) -> Result<PathBuf, BundleError> {
        let mut components = key.as_str().split('/').peekable();
        let mut path = self.root.join("artifacts");
        ensure_artifact_directory(&path, key, boundary, create_missing)?;
        while let Some(segment) = components.next() {
            path.push(segment);
            if components.peek().is_some() {
                ensure_artifact_directory(&path, key, boundary, create_missing)?;
            }
        }
        Ok(path)
    }

    /// Reads a possibly-crashed bundle root and reports exactly what the
    /// durable files prove. Missing files mean absent facts, not inferred
    /// ones.
    pub(crate) fn recover(root: &Path) -> io::Result<RecoveryObservation> {
        let read_json = |name: &str| -> io::Result<Option<String>> {
            match fs::read_to_string(root.join(name)) {
                Ok(text) => Ok(Some(text)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e),
            }
        };
        let intent = read_json("intent.json")?
            .map(|text| serde_json::from_str(&text))
            .transpose()
            .map_err(io::Error::other)?;
        let settlement = read_json("settlement.json")?
            .map(|text| serde_json::from_str(&text))
            .transpose()
            .map_err(io::Error::other)?;
        Ok(RecoveryObservation {
            intent,
            settlement,
            sealed: root.join("manifest.json").is_file(),
        })
    }

    /// Durably reserves the intent identities before any process effect can
    /// start (P18-DUR-001). The record is fsynced to `intent.json` before
    /// the returned proof exists; calling twice or after sealing fails.
    pub(crate) fn publish_intent(
        &mut self,
        record: &IntentRecord,
    ) -> Result<DurableIntentProof, BundleError> {
        match &self.state {
            BundleState::Staging { intent: None, .. } => {}
            BundleState::Staging {
                intent: Some(_), ..
            } => {
                return Err(BundleError::IntentAlreadyReserved {
                    boundary: FailureBoundaryCode::TrialDurability,
                });
            }
            BundleState::Sealed => {
                return Err(BundleError::SealedMutation {
                    boundary: FailureBoundaryCode::TrialDurability,
                });
            }
        }
        let bytes = serde_json::to_vec(record).map_err(|source| BundleError::Io {
            boundary: FailureBoundaryCode::TrialDurability,
            source: io::Error::other(source),
        })?;
        atomic_write(&self.root.join("intent.json"), &bytes)?;
        match &mut self.state {
            // The leading match proved this arm; only the intent field moves.
            BundleState::Staging { intent, .. } => *intent = Some(record.clone()),
            BundleState::Sealed => unreachable!("sealed state rejected above"),
        }
        Ok(DurableIntentProof { _private: () })
    }

    /// Durably records that the observed outcome was settled. Valid only
    /// after a published intent and before sealing; one settlement per
    /// bundle.
    pub(crate) fn record_settlement(
        &mut self,
        marker: &SettlementMarker,
    ) -> Result<(), BundleError> {
        match &mut self.state {
            BundleState::Staging {
                intent, settlement, ..
            } => {
                if intent.is_none() {
                    return Err(BundleError::IntentNotPublished {
                        boundary: FailureBoundaryCode::TrialDurability,
                    });
                }
                if settlement.is_some() {
                    return Err(BundleError::SettlementAlreadyRecorded {
                        boundary: FailureBoundaryCode::TrialDurability,
                    });
                }
                let bytes = serde_json::to_vec(marker).map_err(io::Error::other)?;
                atomic_write(&self.root.join("settlement.json"), &bytes)?;
                *settlement = Some(marker.clone());
                Ok(())
            }
            BundleState::Sealed => Err(BundleError::SealedMutation {
                boundary: FailureBoundaryCode::TrialDurability,
            }),
        }
    }
}

impl From<io::Error> for BundleError {
    fn from(source: io::Error) -> Self {
        BundleError::Io {
            boundary: FailureBoundaryCode::TrialDurability,
            source,
        }
    }
}

/// Reads one durable control sidecar (`intent.json`, `settlement.json`)
/// for verification. Absent or unparsable bytes are sidecar drift: the
/// sealed manifest can never be verified against a side file it cannot
/// read back.
fn read_sidecar<T: serde::de::DeserializeOwned>(root: &Path, name: &str) -> Result<T, BundleError> {
    let bytes = fs::read(root.join(name)).map_err(|_| BundleError::SidecarDrift {
        boundary: FailureBoundaryCode::TrialDurability,
        which: match name {
            "intent.json" => "intent",
            _ => "settlement",
        },
    })?;
    serde_json::from_slice(&bytes).map_err(|_| BundleError::SidecarDrift {
        boundary: FailureBoundaryCode::TrialDurability,
        which: match name {
            "intent.json" => "intent",
            _ => "settlement",
        },
    })
}

/// Collects every file under the sealed artifact tree, by logical key.
/// Symlinked entries are reported as their own path so the per-entry read
/// rejects them; directories are walked without following symlinks.
fn collect_artifact_files(
    dir: &Path,
    prefix: &Path,
    files: &mut Vec<String>,
) -> Result<(), BundleError> {
    let entries = fs::read_dir(dir).map_err(|source| BundleError::Io {
        boundary: FailureBoundaryCode::TrialDurability,
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| BundleError::Io {
            boundary: FailureBoundaryCode::TrialDurability,
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| BundleError::Io {
            boundary: FailureBoundaryCode::TrialDurability,
            source,
        })?;
        let key = prefix.join(entry.file_name());
        let key = key.to_string_lossy().replace('\\', "/");
        if file_type.is_dir() {
            collect_artifact_files(&entry.path(), Path::new(&key), files)?;
        } else {
            files.push(key);
        }
    }
    Ok(())
}

/// Validate one artifact ancestor without following an alias. Missing
/// directories are created one component at a time only while staging; sealed
/// reads remain read-only. Windows junctions are reparse points even when the
/// standard symlink bit is not set, so both forms are rejected.
fn ensure_artifact_directory(
    path: &Path,
    key: &ArtifactKey,
    boundary: FailureBoundaryCode,
    create_missing: bool,
) -> Result<(), BundleError> {
    let inspect = || -> Result<bool, io::Error> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || metadata_is_windows_reparse_point(&metadata) {
            return Ok(false);
        }
        Ok(metadata.is_dir())
    };

    match inspect() {
        Ok(true) => Ok(()),
        Ok(false) => Err(BundleError::SymlinkEscape {
            boundary,
            key: key.clone(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound && create_missing => {
            match fs::create_dir(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => match inspect() {
                    Ok(true) => Ok(()),
                    Ok(false) => Err(BundleError::SymlinkEscape {
                        boundary,
                        key: key.clone(),
                    }),
                    Err(source) => Err(BundleError::Io { boundary, source }),
                },
                Err(source) => Err(BundleError::Io { boundary, source }),
            }
        }
        Err(source) => Err(BundleError::Io { boundary, source }),
    }
}

#[cfg(windows)]
fn metadata_is_windows_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
}

#[cfg(not(windows))]
fn metadata_is_windows_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

/// Writes `bytes` to a temporary sibling and atomically renames it over
/// `path`, fsyncing the file first so a crash never leaves a partial
/// publication visible under the final name.
fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let mut tmp = path.to_path_buf();
    tmp.set_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        #[cfg(test)]
        tests::record_atomic_write_event("write");
        file.sync_all()?;
        #[cfg(test)]
        tests::record_atomic_write_event("file-sync");
    }
    fs::rename(&tmp, path)?;
    #[cfg(test)]
    tests::record_atomic_write_event("rename");
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "atomic publication requires a containing directory",
        )
    })?;
    #[cfg(test)]
    tests::record_atomic_write_event("parent-sync");
    sync_parent_directory(parent)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    #[cfg(test)]
    if tests::parent_sync_should_fail() {
        return Err(io::Error::other("injected parent-directory sync failure"));
    }
    fs::File::open(parent)?.sync_all()
}

#[cfg(windows)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;

    #[cfg(test)]
    if tests::parent_sync_should_fail() {
        return Err(io::Error::other("injected parent-directory sync failure"));
    }
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
        .open(parent)?
        .sync_all()
}
