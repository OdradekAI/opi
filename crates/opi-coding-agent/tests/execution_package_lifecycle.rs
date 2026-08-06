//! Behavioral tests for task 16.5: Package Trust + enable/disable lifecycle.
//!
//! Drives the REAL production paths:
//! - `handle_package_command(Add)` -> `cmd_add` -> `validate_executable_contributions`
//!   (first production caller) -> lock material persisted + untrusted/disabled
//!   activation record.
//! - `PackageActivationStore` enable/disable/remove/activate seam (enable's
//!   trust-grant logic is exercised with an injected confirmer; the CLI's
//!   non-TTY refusal is exercised through the real `StdinTrustConfirmer`).
//!
//! Covers SC16-03: install-disabled-untrusted; first-enable consent; non-TTY
//! refusal; trust/enablement independence; disable-preserves-trust; remove-
//! deletes-all; drift-invalidates-trust; cross-package collision; pre-spawn
//! revalidation fail-closed; no adapter process started.

use std::path::{Path, PathBuf};

use opi_coding_agent::cli::PackageCommand;
use opi_coding_agent::execution::{PackageSource, validate_executable_contributions};
use opi_coding_agent::package_activation::{
    self, ActivationError, ActivationRecord, TrustConfirmer, TrustDisplay,
};
use opi_coding_agent::package_cli;
use opi_coding_agent::package_discovery::PackageManifest;
use opi_coding_agent::package_store::PackageStore;

const EXE_CONTENT: &[u8] = b"#!/bin/sh\necho hi\n";

fn t_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}

fn make_executable(path: &Path) {
    let _ = path;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// Build a package dir whose `package.toml` declares one executable
/// `command.execute` contribution targeting the running host. Returns the
/// tempdir (caller keeps it alive), its root, and the executable SHA-256.
fn make_execution_package(adapter_id: &str) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let exe = dir.path().join("bin").join(adapter_id);
    std::fs::write(&exe, EXE_CONTENT).unwrap();
    make_executable(&exe);
    let sha = t_sha256(EXE_CONTENT);
    let target = package_activation::host_target_triple();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \">=0.7,<0.8\"\n\
         name = \"{adapter_id}\"\n\
         description = \"test execution backend\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"{adapter_id}\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/{adapter_id}\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    std::fs::write(dir.path().join("package.toml"), toml).unwrap();
    let root = dir.path().to_path_buf();
    (dir, root, sha)
}

/// A confirmer that grants or refuses deterministically.
struct TestConfirmer {
    grant: bool,
    saw_display: bool,
}

impl TrustConfirmer for TestConfirmer {
    fn confirm(&mut self, _display: &TrustDisplay) -> Result<(), String> {
        self.saw_display = true;
        if self.grant {
            Ok(())
        } else {
            Err("test confirmer refused".into())
        }
    }
}

fn add_global(source: &str, workspace: &Path, user: &Path) -> i32 {
    package_cli::handle_package_command(
        &PackageCommand::Add {
            source: source.to_string(),
            local: false,
        },
        workspace.to_path_buf(),
        user.to_path_buf(),
    )
}

fn records(user: &Path) -> Vec<ActivationRecord> {
    package_activation::PackageActivationStore::global(user.to_path_buf())
        .read_records()
        .expect("read records")
}

fn store(user: &Path) -> package_activation::PackageActivationStore {
    package_activation::PackageActivationStore::global(user.to_path_buf())
}

// --- install ---------------------------------------------------------------

#[test]
fn add_global_execution_package_persists_lock_and_untrusted_disabled_record() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    let exit = add_global(root.to_str().unwrap(), workspace.path(), user.path());
    assert_eq!(exit, 0, "global add should succeed");

    // Lock material persisted on the package lock entry.
    let locks = PackageStore::global(user.path().to_path_buf())
        .read_lock()
        .expect("read lock");
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].contributions.len(), 1);
    assert_eq!(locks[0].contributions[0].adapter_id, "opi-sandbox");

    // Activation record is untrusted + disabled.
    let recs = records(user.path());
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].name, "opi-sandbox");
    assert!(!recs[0].trusted);
    assert!(!recs[0].enabled);
}

#[cfg(target_os = "macos")]
#[test]
fn macos_non_empty_snapshot_matches_install_lock_and_activation_revalidation() {
    let (_pkg, root, declared_sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0,
        "macOS package add must hash the non-empty snapshot from byte zero"
    );
    let persisted_lock = PackageStore::global(user.path().to_path_buf())
        .read_lock()
        .expect("read persisted lock");
    let persisted_contribution = &persisted_lock[0].contributions[0];
    assert_eq!(persisted_contribution.executable_sha256, declared_sha);

    let activation = store(user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    activation
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect("enable revalidation must match the declared digest");
    let activated = activation
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect("pre-spawn revalidation must match the persisted lock");
    let validated = &activated.validated[0];

    assert_eq!(validated.lock, *persisted_contribution);
    assert_eq!(validated.lock.executable_sha256, declared_sha);
    assert_eq!(
        std::fs::read(validated.bound_launch_path()).unwrap(),
        EXE_CONTENT,
        "the validated bound descriptor must expose the complete executable"
    );
}

#[test]
fn add_project_local_execution_package_is_rejected() {
    let (_pkg, root, _sha) = make_execution_package("proj-exec");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    let exit = package_cli::handle_package_command(
        &PackageCommand::Add {
            source: root.to_str().unwrap().to_string(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(
        exit, 2,
        "project-local executable contribution must be rejected"
    );

    // Nothing installed and no activation record.
    assert!(
        PackageStore::global(user.path().to_path_buf())
            .read_lock()
            .unwrap()
            .is_empty()
    );
    assert!(records(user.path()).is_empty());
}

#[test]
fn install_rejects_colliding_adapter_id_across_packages() {
    let (_a, root_a, _sha_a) = make_execution_package("dup-adapter");
    let (_b, root_b, _sha_b) = make_execution_package("dup-adapter"); // same adapter id
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    assert_eq!(
        add_global(root_a.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let declaration_path = user.path().join("packages.toml");
    let lock_path = user.path().join("package-lock.toml");
    let trust_path = user.path().join("package-trust.toml");
    let before_declarations = std::fs::read(&declaration_path).unwrap();
    let before_lock = std::fs::read(&lock_path).unwrap();
    let before_trust = std::fs::read(&trust_path).unwrap();
    let exit = add_global(root_b.to_str().unwrap(), workspace.path(), user.path());
    assert_eq!(
        exit, 2,
        "second package with a colliding adapter id must be rejected"
    );
    assert_eq!(
        records(user.path()).len(),
        1,
        "only the first package is recorded"
    );
    assert_eq!(
        std::fs::read(declaration_path).unwrap(),
        before_declarations
    );
    assert_eq!(std::fs::read(lock_path).unwrap(), before_lock);
    assert_eq!(std::fs::read(trust_path).unwrap(), before_trust);
}

fn set_file_readonly(path: &Path, readonly: bool) {
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    set_permissions_readonly(&mut permissions, readonly);
    std::fs::set_permissions(path, permissions).unwrap();
}

#[cfg(unix)]
fn set_permissions_readonly(permissions: &mut std::fs::Permissions, readonly: bool) {
    use std::os::unix::fs::PermissionsExt as _;

    let mode = permissions.mode();
    permissions.set_mode(if readonly {
        mode & !0o222
    } else {
        mode | 0o600
    });
}

#[cfg(not(unix))]
fn set_permissions_readonly(permissions: &mut std::fs::Permissions, readonly: bool) {
    permissions.set_readonly(readonly);
}

#[test]
fn validated_executable_identity_cannot_be_replaced_before_launch() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let manifest_path = root.join("package.toml");
    let raw = std::fs::read(&manifest_path).unwrap();
    let manifest =
        PackageManifest::from_toml(&String::from_utf8_lossy(&raw), &manifest_path).unwrap();
    let validated = validate_executable_contributions(
        &manifest,
        &raw,
        &root,
        PackageSource::Global,
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
    )
    .unwrap();
    let contribution = &validated[0];
    let command = root.join("bin/opi-sandbox");

    #[cfg(unix)]
    {
        let validated_file = root.join("bin/validated-original");
        std::fs::rename(&command, &validated_file).unwrap();
        std::fs::write(&command, b"#!/bin/sh\necho replacement\n").unwrap();
        make_executable(&command);
        assert_eq!(
            std::fs::read(contribution.bound_launch_path()).unwrap(),
            EXE_CONTENT,
            "the launch path stays bound to the validated inode"
        );
    }

    #[cfg(windows)]
    {
        let error = std::fs::write(&command, b"replacement")
            .expect_err("validated handle must deny write/delete replacement sharing");
        assert_ne!(error.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            std::fs::read(contribution.bound_launch_path()).unwrap(),
            EXE_CONTENT,
            "the launch path still names the validated locked file"
        );
    }
}

#[cfg(unix)]
#[test]
fn validated_executable_material_survives_same_inode_rewrite() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let manifest_path = root.join("package.toml");
    let raw = std::fs::read(&manifest_path).unwrap();
    let manifest =
        PackageManifest::from_toml(&String::from_utf8_lossy(&raw), &manifest_path).unwrap();
    let validated = validate_executable_contributions(
        &manifest,
        &raw,
        &root,
        PackageSource::Global,
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
    )
    .unwrap();
    let contribution = &validated[0];

    std::fs::write(root.join("bin/opi-sandbox"), b"changed in place").unwrap();
    assert_eq!(
        std::fs::read(contribution.bound_launch_path()).unwrap(),
        EXE_CONTENT,
        "launch material must be detached from later writes to the package inode"
    );
}

#[cfg(unix)]
#[test]
fn package_doctor_rejects_fifo_executable_without_blocking() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt as _;
    use std::sync::mpsc;
    use std::time::Duration;

    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let executable = root.join("bin/opi-sandbox");
    std::fs::remove_file(&executable).unwrap();
    let path = CString::new(executable.as_os_str().as_bytes()).unwrap();
    // SAFETY: `path` is a live NUL-terminated filesystem path and the mode is valid.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);

    let workspace_path = workspace.path().to_path_buf();
    let user_path = user.path().to_path_buf();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let code = package_cli::handle_package_command(
            &PackageCommand::Doctor { json: true },
            workspace_path,
            user_path,
        );
        let _ = tx.send(code);
    });
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2))
            .expect("doctor must not block opening a FIFO"),
        2
    );
}

#[test]
fn byte_identical_readd_preserves_trust_and_enablement() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();

    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let rec = &records(user.path())[0];
    assert!(rec.trusted && rec.enabled);
}

#[test]
fn changed_executable_readd_invalidates_trust_and_enablement() {
    let (_pkg, root, old_sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();

    let changed = b"#!/bin/sh\necho changed\n";
    std::fs::write(root.join("bin/opi-sandbox"), changed).unwrap();
    make_executable(&root.join("bin/opi-sandbox"));
    let manifest_path = root.join("package.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    std::fs::write(
        &manifest_path,
        manifest.replace(&old_sha, &t_sha256(changed)),
    )
    .unwrap();

    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let rec = &records(user.path())[0];
    assert!(!rec.trusted && !rec.enabled);
}

#[test]
fn removing_all_contributions_on_readd_invalidates_trust_and_enablement() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();

    std::fs::write(
        root.join("package.toml"),
        "version = \"0.8.0\"\nopi_version = \">=0.7,<0.8\"\nname = \"opi-sandbox\"\ndescription = \"no executable contribution\"\n",
    )
    .unwrap();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );
    let rec = &records(user.path())[0];
    assert!(!rec.trusted && !rec.enabled);
}

// --- enable / disable / remove --------------------------------------------

#[test]
fn enable_refuses_without_explicit_confirmation() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let mut confirmer = TestConfirmer {
        grant: false,
        saw_display: false,
    };
    let err = store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect_err("refusing confirmer must not grant trust");
    assert!(matches!(err, ActivationError::Untrusted { .. }));
    // Trust was NOT granted.
    let rec = &records(user.path())[0];
    assert!(!rec.trusted);
    assert!(!rec.enabled);
    // The confirmer was shown identity before deciding.
    assert!(confirmer.saw_display);
}

#[test]
fn enable_grants_trust_and_enables_with_confirmation() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect("granting confirmer enables");
    let rec = &records(user.path())[0];
    assert!(rec.trusted);
    assert!(rec.enabled);
}

#[test]
fn cli_enable_refuses_in_non_tty() {
    // The production `StdinTrustConfirmer` refuses when stdin is not a terminal
    // (cargo test runs without a TTY), proving machine-facing enable cannot
    // grant trust.
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let exit = package_cli::handle_package_command(
        &PackageCommand::Enable {
            name: "opi-sandbox".into(),
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 2, "non-TTY enable must refuse to grant trust");
    let rec = &records(user.path())[0];
    assert!(!rec.trusted, "trust must not be granted in a non-TTY run");
}

#[test]
fn disable_preserves_trust_and_clears_enablement() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();
    store(user.path()).disable("opi-sandbox").unwrap();

    let rec = &records(user.path())[0];
    assert!(rec.trusted, "disable preserves Package Trust");
    assert!(!rec.enabled, "disable clears enablement");
}

#[test]
fn re_enable_after_disable_does_not_re_prompt() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    let s = store(user.path());
    s.enable(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
        &mut confirmer,
    )
    .unwrap();
    s.disable("opi-sandbox").unwrap();
    // Re-enable: already trusted, so the confirmer must NOT be consulted.
    confirmer.saw_display = false;
    s.enable(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
        &mut confirmer,
    )
    .expect("re-enable of trusted package succeeds without prompt");
    assert!(!confirmer.saw_display, "re-enable must not re-prompt");
    let rec = &records(user.path())[0];
    assert!(rec.trusted && rec.enabled);
}

#[test]
fn remove_deletes_lock_and_activation_record() {
    let (pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    assert_eq!(records(user.path()).len(), 1);

    let exit = package_cli::handle_package_command(
        &PackageCommand::Remove {
            name_or_source: "opi-sandbox".into(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(exit, 0);
    assert!(
        records(user.path()).is_empty(),
        "remove deletes the trust record"
    );
    assert!(
        PackageStore::global(user.path().to_path_buf())
            .read_lock()
            .unwrap()
            .is_empty(),
    );
    assert!(
        PackageStore::global(user.path().to_path_buf())
            .read_declarations()
            .unwrap()
            .is_empty(),
        "remove deletes the package declaration"
    );
    drop(pkg);
}

// --- activate (pre-spawn revalidation seam) --------------------------------

#[test]
fn activate_untrusted_returns_untrusted() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());

    let err = store(user.path())
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("untrusted package must not activate");
    assert!(matches!(err, ActivationError::Untrusted { .. }));
}

#[test]
fn activate_disabled_returns_disabled() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();
    store(user.path()).disable("opi-sandbox").unwrap();

    let err = store(user.path())
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("disabled package must not activate");
    assert!(matches!(err, ActivationError::Disabled(_)));
    // Trust is still intact after a disabled activation attempt.
    assert!(records(user.path())[0].trusted);
}

#[test]
fn activate_not_installed() {
    let user = tempfile::tempdir().unwrap();
    let err = store(user.path())
        .activate(
            "no-such-package",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("unknown package must not activate");
    assert!(matches!(err, ActivationError::NotInstalled(_)));
}

#[test]
fn activate_returns_handle_for_trusted_enabled_valid_package() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();

    let activated = store(user.path())
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect("trusted+enabled+valid package activates");
    assert_eq!(activated.name, "opi-sandbox");
    assert_eq!(activated.validated.len(), 1);
    assert_eq!(activated.validated[0].id, "opi-sandbox");
    // Metadata-only handle: no spawn occurs (the protocol host 16.7 owns spawn).
}

#[test]
fn activate_drift_invalidates_trust_durably() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();

    // Tamper with the executable after trust was granted.
    std::fs::write(
        root.join("bin").join("opi-sandbox"),
        b"#!/bin/sh\necho pwned\n",
    )
    .unwrap();

    let err = store(user.path())
        .activate(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect_err("drifted executable must fail closed");
    assert!(matches!(err, ActivationError::Untrusted { .. }));

    // Drift durably invalidated trust (persisted): re-enable must re-prompt.
    let rec = &records(user.path())[0];
    assert!(!rec.trusted, "drift must invalidate persisted trust");
    assert!(!rec.enabled);
}

#[test]
fn usable_identity_resolution_filters_current_target_mismatch() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let activation = store(user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    activation
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();
    assert_eq!(
        activation
            .usable_enabled_identities(
                package_activation::host_target_triple(),
                package_activation::host_opi_version(),
            )
            .len(),
        1
    );

    let manifest_path = root.join("package.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    std::fs::write(
        &manifest_path,
        manifest.replace(
            package_activation::host_target_triple(),
            "mismatched-target",
        ),
    )
    .unwrap();
    assert!(
        activation
            .usable_enabled_identities(
                package_activation::host_target_triple(),
                package_activation::host_opi_version(),
            )
            .is_empty(),
        "a stale target must not become a model-visible candidate"
    );
}

#[test]
fn activate_surfaces_trust_invalidation_write_failure() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    store(user.path())
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .unwrap();
    std::fs::write(root.join("bin/opi-sandbox"), b"changed").unwrap();

    let trust_path = user.path().join("package-trust.toml");
    set_file_readonly(&trust_path, true);
    let result = store(user.path()).activate(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
    );
    set_file_readonly(&trust_path, false);

    assert!(
        matches!(result, Err(ActivationError::Store(_))),
        "durable invalidation write failure must surface: {result:?}"
    );
}

#[test]
fn re_enable_after_drift_requires_reconfirmation() {
    let (_pkg, root, _sha) = make_execution_package("opi-sandbox");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root.to_str().unwrap(), workspace.path(), user.path());
    let s = store(user.path());
    let mut confirmer = TestConfirmer {
        grant: true,
        saw_display: false,
    };
    s.enable(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
        &mut confirmer,
    )
    .unwrap();

    // Tamper with the executable. The next enable's revalidation detects the
    // drift and durably invalidates trust (persisted trusted=false).
    std::fs::write(
        root.join("bin").join("opi-sandbox"),
        b"#!/bin/sh\necho pwned\n",
    )
    .unwrap();
    let err = s
        .enable(
            "opi-sandbox",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut confirmer,
        )
        .expect_err("drifted package must not enable");
    assert!(matches!(err, ActivationError::Untrusted { .. }));
    assert!(
        !records(user.path())[0].trusted,
        "drift invalidated persisted trust"
    );

    // Restore the original bytes so revalidation passes again, then re-enable:
    // trust is now false, so it MUST re-prompt for confirmation.
    std::fs::write(root.join("bin").join("opi-sandbox"), EXE_CONTENT).unwrap();
    confirmer.saw_display = false;
    s.enable(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
        &mut confirmer,
    )
    .expect("re-enable after drift restoration succeeds");
    assert!(
        confirmer.saw_display,
        "re-enable after drift must re-prompt"
    );
}

#[test]
fn activate_resolves_only_the_named_package() {
    // Two installed, trusted+enabled packages: activating one must not touch the
    // other (no full scan / no cross-activation).
    let (_a, root_a, _sha_a) = make_execution_package("adapter-a");
    let (_b, root_b, _sha_b) = make_execution_package("adapter-b");
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    add_global(root_a.to_str().unwrap(), workspace.path(), user.path());
    add_global(root_b.to_str().unwrap(), workspace.path(), user.path());
    let s = store(user.path());
    for name in ["adapter-a", "adapter-b"] {
        let mut c = TestConfirmer {
            grant: true,
            saw_display: false,
        };
        s.enable(
            name,
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
            &mut c,
        )
        .unwrap();
    }
    let activated = s
        .activate(
            "adapter-a",
            package_activation::host_target_triple(),
            package_activation::host_opi_version(),
        )
        .expect("named activation");
    assert_eq!(activated.validated.len(), 1);
    assert_eq!(activated.validated[0].id, "adapter-a");
}

#[test]
fn no_lifecycle_path_starts_an_adapter_process() {
    // The contribution's executable, IF ever spawned, creates a marker file.
    // No package lifecycle path (list, package doctor, top-level doctor, enable,
    // activate) may start the adapter process. This observes the ABSENCE of a
    // spawn side-effect, independent of the cfg(unix) executability gate, so it
    // is meaningful on Windows too.
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let marker = user.path().join("spawned-marker.txt");
    let exe_content = format!("#!/bin/sh\ntouch \"{}\"\n", marker.display());

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("bin")).unwrap();
    let exe = dir.path().join("bin").join("opi-sandbox");
    std::fs::write(&exe, &exe_content).unwrap();
    make_executable(&exe);
    let sha = t_sha256(exe_content.as_bytes());
    let target = package_activation::host_target_triple();
    let toml = format!(
        "version = \"0.8.0\"\n\
         opi_version = \">=0.7,<0.8\"\n\
         name = \"opi-sandbox\"\n\
         description = \"marker backend\"\n\
         \n\
         [[contributions.adapters]]\n\
         capability = \"command.execute\"\n\
         id = \"opi-sandbox\"\n\
         transport = \"process-jsonl\"\n\
         command = \"bin/opi-sandbox\"\n\
         args = [\"backend\", \"--stdio\"]\n\
         protocol = \"command-execution-jsonl-v1\"\n\
         target = \"{target}\"\n\
         sha256 = \"{sha}\"\n\
         handshake_timeout_ms = 5000\n\
         adapter_config = {{}}\n"
    );
    std::fs::write(dir.path().join("package.toml"), toml).unwrap();
    let root = dir.path().to_path_buf();
    assert_eq!(
        add_global(root.to_str().unwrap(), workspace.path(), user.path()),
        0
    );

    // Exercise every read/lifecycle path through its real entry point.
    for cmd in [
        PackageCommand::List { json: true },
        PackageCommand::Doctor { json: true },
        PackageCommand::Enable {
            name: "opi-sandbox".into(),
        },
    ] {
        let _ = package_cli::handle_package_command(
            &cmd,
            workspace.path().to_path_buf(),
            user.path().to_path_buf(),
        );
    }
    // activate (the pre-spawn seam) must not spawn either.
    let _ = store(user.path()).activate(
        "opi-sandbox",
        package_activation::host_target_triple(),
        package_activation::host_opi_version(),
    );

    assert!(
        !marker.exists(),
        "no package lifecycle path may start the adapter process"
    );
}
