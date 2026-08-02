//! Integration tests for the package CLI MVP (task 5.2).

use std::path::{Path, PathBuf};

use clap::Parser;
use opi_coding_agent::cli::{Cli, Command, PackageCommand};
use opi_coding_agent::package_cli::handle_package_command;
use opi_coding_agent::package_resolver::{local_lock_entry, resolve_declared_installed_packages};
use opi_coding_agent::package_store::{PackageDeclaration, PackageStore};

fn write_package(root: &Path, name: &str) {
    write_package_version(root, name, "0.1.0");
}

fn write_package_version(root: &Path, name: &str, version: &str) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("package.toml"),
        format!("name = \"{name}\"\ndescription = \"{name} package\"\nversion = \"{version}\"\n"),
    )
    .unwrap();
}

fn write_duplicate_project_packages(workspace: &Path) {
    let pkg_a = workspace.join("dup-a");
    let pkg_b = workspace.join("dup-b");
    write_package(&pkg_a, "dup");
    write_package(&pkg_b, "dup");

    let store = PackageStore::project(workspace.to_path_buf());
    store
        .write_declarations(&[
            PackageDeclaration {
                source: "./dup-a".into(),
                filters: Default::default(),
            },
            PackageDeclaration {
                source: "./dup-b".into(),
                filters: Default::default(),
            },
        ])
        .unwrap();
    store
        .write_lock(&[
            local_lock_entry("./dup-a".into(), &pkg_a).unwrap(),
            local_lock_entry("./dup-b".into(), &pkg_b).unwrap(),
        ])
        .unwrap();
}

fn git_in(cwd: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@test.com")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git command")
}

fn assert_git_ok(output: std::process::Output, action: &str) -> std::process::Output {
    assert!(
        output.status.success(),
        "{action} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

struct GitPackageRepo {
    _tmp: tempfile::TempDir,
    bare_url: String,
    first_commit: String,
    second_commit: String,
}

fn git_package_repo_with_two_commits(name: &str) -> GitPackageRepo {
    let tmp = tempfile::tempdir().unwrap();
    let bare_dir = tmp.path().join("bare.git");
    let work_dir = tmp.path().join("work");
    std::fs::create_dir_all(&work_dir).unwrap();

    assert_git_ok(
        std::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(&bare_dir)
            .output()
            .expect("git init --bare"),
        "git init --bare",
    );
    assert_git_ok(git_in(&work_dir, &["init"]), "git init");
    write_package_version(&work_dir, name, "0.1.0");
    assert_git_ok(git_in(&work_dir, &["add", "."]), "git add first");
    assert_git_ok(
        git_in(&work_dir, &["commit", "-m", "initial"]),
        "git commit first",
    );
    let first = assert_git_ok(
        git_in(&work_dir, &["rev-parse", "HEAD"]),
        "git rev-parse first",
    );
    let first_commit = String::from_utf8_lossy(&first.stdout).trim().to_string();

    let bare_url = format!(
        "file:///{}",
        bare_dir.display().to_string().replace('\\', "/")
    );
    assert_git_ok(
        git_in(&work_dir, &["remote", "add", "origin", &bare_url]),
        "git remote add",
    );
    assert_git_ok(
        git_in(&work_dir, &["push", "origin", "HEAD:refs/heads/main"]),
        "git push first",
    );

    write_package_version(&work_dir, name, "0.2.0");
    assert_git_ok(git_in(&work_dir, &["add", "."]), "git add second");
    assert_git_ok(
        git_in(&work_dir, &["commit", "-m", "second"]),
        "git commit second",
    );
    let second = assert_git_ok(
        git_in(&work_dir, &["rev-parse", "HEAD"]),
        "git rev-parse second",
    );
    let second_commit = String::from_utf8_lossy(&second.stdout).trim().to_string();
    assert_git_ok(
        git_in(&work_dir, &["push", "origin", "HEAD:refs/heads/main"]),
        "git push second",
    );

    GitPackageRepo {
        _tmp: tmp,
        bare_url,
        first_commit,
        second_commit,
    }
}

fn git_head(repo: &Path) -> String {
    let output = assert_git_ok(git_in(repo, &["rev-parse", "HEAD"]), "git rev-parse HEAD");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn opi_command(opi: &Path, workspace: &Path, user_config_root: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(opi);
    command.current_dir(workspace);
    if cfg!(windows) {
        command.env("APPDATA", user_config_root);
    } else {
        command.env("HOME", user_config_root);
    }
    command
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
    if readonly {
        permissions.set_mode(mode & !0o222);
    } else {
        permissions.set_mode(mode | 0o600);
    }
}

#[cfg(not(unix))]
fn set_permissions_readonly(permissions: &mut std::fs::Permissions, readonly: bool) {
    permissions.set_readonly(readonly);
}

// ---------------------------------------------------------------------------
// CLI parser tests
// ---------------------------------------------------------------------------

#[test]
fn parses_package_add_project_scope() {
    let cli = Cli::parse_from(["opi", "package", "add", "./pkg", "-l"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::Add { source, local },
        }) => {
            assert_eq!(source, "./pkg");
            assert!(local);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_add_global_scope() {
    let cli = Cli::parse_from(["opi", "package", "add", "git:github.com/user/repo@v1"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::Add { source, local },
        }) => {
            assert_eq!(source, "git:github.com/user/repo@v1");
            assert!(!local);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_remove_project_scope() {
    let cli = Cli::parse_from(["opi", "package", "remove", "./pkg", "-l"]);
    match cli.command {
        Some(Command::Package {
            command:
                PackageCommand::Remove {
                    name_or_source,
                    local,
                },
        }) => {
            assert_eq!(name_or_source, "./pkg");
            assert!(local);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_list() {
    let cli = Cli::parse_from(["opi", "package", "list"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::List { json },
        }) => assert!(!json),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_list_json() {
    let cli = Cli::parse_from(["opi", "package", "list", "--json"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::List { json },
        }) => assert!(json),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_doctor() {
    let cli = Cli::parse_from(["opi", "package", "doctor"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::Doctor { json },
        }) => assert!(!json),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn parses_package_doctor_json() {
    let cli = Cli::parse_from(["opi", "package", "doctor", "--json"]);
    match cli.command {
        Some(Command::Package {
            command: PackageCommand::Doctor { json },
        }) => assert!(json),
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn no_package_command_is_none() {
    let cli = Cli::parse_from(["opi", "--list-sessions"]);
    // --list-sessions does not produce a package command
    assert!(cli.command.is_none());
}

// ---------------------------------------------------------------------------
// Behavior tests: add, remove, list, doctor
// ---------------------------------------------------------------------------

#[test]
fn package_add_rejects_missing_local_package() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();

    let code = opi_coding_agent::package_cli::handle_package_command(
        &opi_coding_agent::cli::PackageCommand::Add {
            source: "./missing".to_string(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 2);
    assert!(!workspace.path().join(".opi/packages.toml").exists());
    assert!(!workspace.path().join(".opi/package-lock.toml").exists());
}

#[test]
fn package_add_local_writes_declaration_and_lock() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let pkg = workspace.path().join("vendor/todo");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.toml"),
        "name = \"todo\"\ndescription = \"Todo package\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let code = opi_coding_agent::package_cli::handle_package_command(
        &opi_coding_agent::cli::PackageCommand::Add {
            source: "./vendor/todo".to_string(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 0);
    let decls =
        opi_coding_agent::package_store::PackageStore::project(workspace.path().to_path_buf())
            .read_declarations()
            .unwrap();
    assert_eq!(decls[0].source, "./vendor/todo");

    let locks =
        opi_coding_agent::package_store::PackageStore::project(workspace.path().to_path_buf())
            .read_lock()
            .unwrap();
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].identity_kind, "local");
    assert_eq!(locks[0].package_root, pkg.canonicalize().unwrap());
    assert!(!locks[0].manifest_sha256.is_empty());
}

#[test]
fn package_add_git_writes_declaration_and_lock() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let repo = tempfile::tempdir().unwrap();
    let bare_dir = repo.path().join("bare.git");
    let work_dir = repo.path().join("work");
    std::fs::create_dir_all(&work_dir).unwrap();

    let init_bare = std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare_dir)
        .output()
        .expect("git init --bare");
    assert!(
        init_bare.status.success(),
        "git init --bare failed: {}",
        String::from_utf8_lossy(&init_bare.stderr)
    );

    assert_git_ok(git_in(&work_dir, &["init"]), "git init");
    write_package(&work_dir, "gitpkg");
    assert_git_ok(git_in(&work_dir, &["add", "."]), "git add");
    assert_git_ok(
        git_in(&work_dir, &["commit", "-m", "initial"]),
        "git commit",
    );

    let rev = assert_git_ok(git_in(&work_dir, &["rev-parse", "HEAD"]), "git rev-parse");
    let commit = String::from_utf8_lossy(&rev.stdout).trim().to_string();
    assert!(!commit.is_empty());

    let bare_url = format!(
        "file:///{}",
        bare_dir.display().to_string().replace('\\', "/")
    );
    assert_git_ok(
        git_in(&work_dir, &["remote", "add", "origin", &bare_url]),
        "git remote add",
    );
    assert_git_ok(
        git_in(&work_dir, &["push", "origin", "HEAD:refs/heads/main"]),
        "git push",
    );

    let source = format!("git:{bare_url}@{commit}");
    let code = handle_package_command(
        &PackageCommand::Add {
            source: source.clone(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 0);
    let store = PackageStore::global(user.path().to_path_buf());
    let decls = store.read_declarations().unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].source, source);

    let locks = store.read_lock().unwrap();
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].identity_kind, "git");
    assert_eq!(locks[0].git_commit.as_deref(), Some(commit.as_str()));
    assert!(locks[0].cache_path.is_some());
    assert!(locks[0].package_root.join("package.toml").is_file());
    assert!(!locks[0].manifest_sha256.is_empty());
}

#[test]
fn package_add_git_same_repo_new_ref_updates_declaration_and_lock_sources() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let repo = git_package_repo_with_two_commits("gitpkg");
    let first_source = format!("git:{}@{}", repo.bare_url, repo.first_commit);
    let second_source = format!("git:{}@{}", repo.bare_url, repo.second_commit);

    let first_code = handle_package_command(
        &PackageCommand::Add {
            source: first_source,
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(first_code, 0);

    let second_code = handle_package_command(
        &PackageCommand::Add {
            source: second_source.clone(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(second_code, 0);

    let store = PackageStore::global(user.path().to_path_buf());
    let decls = store.read_declarations().unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].source, second_source);

    let locks = store.read_lock().unwrap();
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].source, second_source);
    assert_eq!(
        locks[0].git_commit.as_deref(),
        Some(repo.second_commit.as_str())
    );
}

#[test]
fn package_add_git_bad_ref_preserves_existing_lock_and_cache() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let repo = git_package_repo_with_two_commits("gitpkg");
    let first_source = format!("git:{}@{}", repo.bare_url, repo.first_commit);
    let bad_source = format!("git:{}@missing-ref", repo.bare_url);

    let first_code = handle_package_command(
        &PackageCommand::Add {
            source: first_source.clone(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(first_code, 0);

    let store = PackageStore::global(user.path().to_path_buf());
    let mut before_locks = store.read_lock().unwrap();
    let before_lock = before_locks.remove(0);
    let package_root = before_lock.package_root.clone();
    assert_eq!(git_head(&package_root), repo.first_commit);
    assert!(package_root.join("package.toml").is_file());

    let bad_code = handle_package_command(
        &PackageCommand::Add {
            source: bad_source,
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(bad_code, 2);

    let decls = store.read_declarations().unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].source, first_source);

    let after_locks = store.read_lock().unwrap();
    assert_eq!(after_locks.len(), 1);
    assert_eq!(after_locks[0].source, before_lock.source);
    assert_eq!(after_locks[0].git_commit, before_lock.git_commit);
    assert_eq!(after_locks[0].manifest_sha256, before_lock.manifest_sha256);
    assert_eq!(git_head(&package_root), repo.first_commit);
    assert!(package_root.join("package.toml").is_file());
}

#[test]
fn package_add_git_metadata_write_failure_preserves_existing_lock_and_cache() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let repo = git_package_repo_with_two_commits("gitpkg");
    let first_source = format!("git:{}@{}", repo.bare_url, repo.first_commit);
    let second_source = format!("git:{}@{}", repo.bare_url, repo.second_commit);

    let first_code = handle_package_command(
        &PackageCommand::Add {
            source: first_source.clone(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(first_code, 0);

    let store = PackageStore::global(user.path().to_path_buf());
    let before_lock = store.read_lock().unwrap().remove(0);
    let package_root = before_lock.package_root.clone();
    assert_eq!(git_head(&package_root), repo.first_commit);

    let config_path = user.path().join("packages.toml");
    set_file_readonly(&config_path, true);
    let second_code = handle_package_command(
        &PackageCommand::Add {
            source: second_source,
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    set_file_readonly(&config_path, false);

    assert_eq!(second_code, 2);
    let decls = store.read_declarations().unwrap();
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].source, first_source);

    let after_locks = store.read_lock().unwrap();
    assert_eq!(after_locks.len(), 1);
    assert_eq!(after_locks[0].source, before_lock.source);
    assert_eq!(after_locks[0].git_commit, before_lock.git_commit);
    assert_eq!(after_locks[0].manifest_sha256, before_lock.manifest_sha256);
    assert_eq!(git_head(&package_root), repo.first_commit);
    assert!(package_root.join("package.toml").is_file());
}

#[test]
fn package_doctor_rejects_manifest_v2_adapter_errors() {
    let workspace = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    let pkg = workspace.path().join("badpkg");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.toml"),
        "name = \"badpkg\"\n\
         description = \"Bad package\"\n\
         version = \"0.1.0\"\n\
         [adapter]\n\
         kind = \"grpc\"\n\
         protocol = \"not-opi\"\n\
         command = \"bad\"\n",
    )
    .unwrap();

    let add_code = opi_coding_agent::package_cli::handle_package_command(
        &opi_coding_agent::cli::PackageCommand::Add {
            source: "./badpkg".to_string(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(add_code, 2);

    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./badpkg".to_string(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry("./badpkg".to_string(), &pkg).unwrap()])
        .unwrap();

    let doctor_code = handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(doctor_code, 2);

    let result = resolve_declared_installed_packages(workspace.path(), user.path()).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "discovery_failed");
    assert!(
        result.diagnostics[0]
            .message
            .contains("unsupported adapter kind 'grpc'"),
        "diagnostic should come from PackageManifest validation: {:?}",
        result.diagnostics[0]
    );
}

#[test]
fn package_add_writes_project_config() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    write_package(&workspace.path().join("pkg"), "pkg");
    let code = handle_package_command(
        &PackageCommand::Add {
            source: "./pkg".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0, "expected exit code 0");
    assert!(
        workspace.path().join(".opi").join("packages.toml").exists(),
        "packages.toml should exist under .opi/"
    );
    // Verify the declaration was written correctly
    let store = PackageStore::project(workspace.path().to_path_buf());
    let decls = store.read_declarations().expect("read declarations");
    assert_eq!(decls.len(), 1);
    assert_eq!(decls[0].source, "./pkg");
}

#[test]
fn package_add_global_scope() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    write_package(&user.path().join("global-pkg"), "global-pkg");
    let code = handle_package_command(
        &PackageCommand::Add {
            source: "./global-pkg".into(),
            local: false,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0);
    assert!(
        user.path().join("packages.toml").exists(),
        "packages.toml should exist in user config dir"
    );
}

#[test]
fn package_add_idempotent() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    write_package(&workspace.path().join("pkg"), "pkg");
    let cmd = PackageCommand::Add {
        source: "./pkg".into(),
        local: true,
    };
    let code1 = handle_package_command(
        &cmd,
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    let code2 = handle_package_command(
        &cmd,
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code1, 0);
    assert_eq!(code2, 0);
    let store = PackageStore::project(workspace.path().to_path_buf());
    let decls = store.read_declarations().expect("read declarations");
    assert_eq!(
        decls.len(),
        1,
        "adding same source twice should be idempotent"
    );
}

#[test]
fn package_remove_deletes_entry() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    write_package(&workspace.path().join("pkg"), "pkg");
    // Add first
    handle_package_command(
        &PackageCommand::Add {
            source: "./pkg".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    // Remove
    let code = handle_package_command(
        &PackageCommand::Remove {
            name_or_source: "./pkg".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0);
    let store = PackageStore::project(workspace.path().to_path_buf());
    let decls = store.read_declarations().expect("read declarations");
    assert!(decls.is_empty(), "declaration should be removed");
}

#[test]
fn package_remove_by_manifest_name_deletes_declaration_and_lock() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    let pkg = workspace.path().join("pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.toml"),
        "name = \"todo\"\ndescription = \"Todo package\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let add_code = handle_package_command(
        &PackageCommand::Add {
            source: "./pkg".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(add_code, 0);

    let code = handle_package_command(
        &PackageCommand::Remove {
            name_or_source: "todo".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 0);
    let store = PackageStore::project(workspace.path().to_path_buf());
    assert!(store.read_declarations().unwrap().is_empty());
    assert!(store.read_lock().unwrap().is_empty());
}

#[test]
fn package_remove_by_manifest_name_rejects_ambiguous_matches() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    for dir in ["pkg-a", "pkg-b"] {
        let pkg = workspace.path().join(dir);
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("package.toml"),
            "name = \"todo\"\ndescription = \"Todo package\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let code = handle_package_command(
            &PackageCommand::Add {
                source: format!("./{dir}"),
                local: true,
            },
            workspace.path().to_path_buf(),
            user.path().to_path_buf(),
        );
        assert_eq!(code, 0);
    }

    let code = handle_package_command(
        &PackageCommand::Remove {
            name_or_source: "todo".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 2);
    let store = PackageStore::project(workspace.path().to_path_buf());
    assert_eq!(store.read_declarations().unwrap().len(), 2);
    assert_eq!(store.read_lock().unwrap().len(), 2);
}

#[test]
fn package_remove_nonexistent_is_ok() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    let code = handle_package_command(
        &PackageCommand::Remove {
            name_or_source: "./nonexistent".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0, "removing nonexistent package should succeed");
}

#[test]
fn package_list_empty_returns_zero() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    let code = handle_package_command(
        &PackageCommand::List { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0);
}

#[test]
fn package_doctor_checks_global_and_project_scopes() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    PackageStore::global(user.path().to_path_buf())
        .write_declarations(&[opi_coding_agent::package_store::PackageDeclaration {
            source: "./missing-global".into(),
            filters: Default::default(),
        }])
        .unwrap();

    let code = handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 2);
}

#[test]
fn package_doctor_reports_missing_manifest_declared_resource_asset() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    let pkg = workspace.path().join("missing-skill-pkg");
    std::fs::create_dir_all(&pkg).unwrap();
    std::fs::write(
        pkg.join("package.toml"),
        "name = \"missing-skill-pkg\"\n\
         description = \"Package with missing declared skill\"\n\
         version = \"0.1.0\"\n\
         skills = [\"review\"]\n",
    )
    .unwrap();
    let store = PackageStore::project(workspace.path().to_path_buf());
    store
        .write_declarations(&[PackageDeclaration {
            source: "./missing-skill-pkg".into(),
            filters: Default::default(),
        }])
        .unwrap();
    store
        .write_lock(&[local_lock_entry("./missing-skill-pkg".into(), &pkg).unwrap()])
        .unwrap();

    let code = handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(code, 2);
    let result = resolve_declared_installed_packages(workspace.path(), user.path()).unwrap();
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "composition_failed");
    assert!(
        result.diagnostics[0]
            .message
            .contains("missing skill 'review'"),
        "diagnostic should identify missing declared skill asset: {:?}",
        result.diagnostics[0]
    );
}

#[test]
fn package_doctor_valid_manifest() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    // Create a valid package with manifest
    let pkg_dir = workspace.path().join("my-pkg");
    std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    std::fs::write(
        pkg_dir.join("package.toml"),
        r#"name = "my-pkg"
description = "Test package"
skills = ["review"]
"#,
    )
    .expect("write manifest");
    let skill_dir = pkg_dir.join("skills").join("review");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: review\ndescription: Review skill\n---\n",
    )
    .expect("write skill");
    // Add the package
    handle_package_command(
        &PackageCommand::Add {
            source: "./my-pkg".into(),
            local: true,
        },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    // Doctor should pass
    let code = handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );
    assert_eq!(code, 0, "doctor should pass for valid package");
}

#[test]
fn package_doctor_reports_duplicate_name() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user = tempfile::tempdir().expect("user tempdir");
    write_duplicate_project_packages(workspace.path());

    let code = handle_package_command(
        &PackageCommand::Doctor { json: false },
        workspace.path().to_path_buf(),
        user.path().to_path_buf(),
    );

    assert_eq!(
        code, 2,
        "doctor must fail when runtime resolution disables a duplicate package"
    );
}

// ---------------------------------------------------------------------------
// Subprocess E2E tests
// ---------------------------------------------------------------------------

fn opi_binary() -> PathBuf {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let workspace_root = PathBuf::from(&crate_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("crate should be in crates/opi-coding-agent")
        .to_path_buf();
    let bin_name = if cfg!(windows) { "opi.exe" } else { "opi" };
    let path = workspace_root.join("target").join("debug").join(bin_name);
    assert!(
        path.exists(),
        "opi binary must be built: run `cargo build -p opi-coding-agent`"
    );
    path
}

#[test]
fn package_cli_subprocess_add_and_list() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    write_package(&workspace.path().join("test-pkg"), "test-pkg");

    // opi package add ./test-pkg -l (from workspace dir)
    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "add", "./test-pkg", "-l"])
        .output()
        .expect("run opi package add");
    assert!(
        output.status.success(),
        "add should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // Verify packages.toml was created
    assert!(workspace.path().join(".opi").join("packages.toml").exists());

    // opi package list
    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "list"])
        .output()
        .expect("run opi package list");
    assert!(
        output.status.success(),
        "list should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("./test-pkg"),
        "list should contain the added package"
    );
}

#[test]
fn package_cli_subprocess_list_json_outputs_installed_package_fields() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    write_package(&workspace.path().join("test-pkg"), "test-pkg");

    let add = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "add", "./test-pkg", "-l"])
        .output()
        .expect("run opi package add");
    assert!(
        add.status.success(),
        "add should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&add.stdout),
        String::from_utf8_lossy(&add.stderr)
    );

    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "list", "--json"])
        .output()
        .expect("run opi package list --json");
    assert!(
        output.status.success(),
        "list --json should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "expected one NDJSON row, got: {stdout}");
    let row: serde_json::Value = serde_json::from_str(lines[0]).unwrap();

    assert_eq!(row["scope"], "project");
    assert_eq!(row["name"], "test-pkg");
    assert_eq!(row["version"], "0.1.0");
    assert_eq!(row["source"], "./test-pkg");
    assert_eq!(row["status"], "ok");
    assert!(
        PathBuf::from(row["package_root"].as_str().unwrap())
            .join("package.toml")
            .is_file()
    );
    assert!(row["adapter_command"].is_null());
    assert!(row["adapter_resolved_command"].is_null());
    assert!(row["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn package_cli_subprocess_list_json_outputs_diagnostics() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    PackageStore::project(workspace.path().to_path_buf())
        .write_declarations(&[PackageDeclaration {
            source: "./missing".into(),
            filters: Default::default(),
        }])
        .unwrap();

    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "list", "--json"])
        .output()
        .expect("run opi package list --json");
    assert!(
        output.status.success(),
        "list --json should keep reporting diagnostics without failing: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "expected one diagnostic row, got: {stdout}");
    let row: serde_json::Value = serde_json::from_str(lines[0]).unwrap();

    assert_eq!(row["scope"], "project");
    assert!(row["name"].is_null());
    assert!(row["version"].is_null());
    assert_eq!(row["source"], "./missing");
    assert_eq!(row["status"], "error");
    assert!(row["package_root"].is_null());
    assert!(row["adapter_command"].is_null());
    assert!(row["adapter_resolved_command"].is_null());
    let diagnostics = row["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["severity"], "error");
    assert_eq!(diagnostics[0]["code"], "source_missing");
}

#[test]
fn package_cli_subprocess_doctor_json() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();

    PackageStore::project(workspace.path().to_path_buf())
        .write_declarations(&[opi_coding_agent::package_store::PackageDeclaration {
            source: "./missing".into(),
            filters: Default::default(),
        }])
        .unwrap();

    // Doctor --json should produce JSON output (non-zero exit when diagnostics found)
    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "doctor", "--json"])
        .output()
        .expect("run opi package doctor");
    // doctor exits non-zero when diagnostics are found
    assert!(
        !output.status.success(),
        "doctor should exit non-zero for missing packages: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("doctor --json output should be valid JSON: {e}\ngot: {stdout}")
    });
    assert!(
        parsed.is_array(),
        "doctor --json should output a JSON array"
    );
    let rows = parsed.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["scope"], "project");
    assert_eq!(row["source"], "./missing");
    assert_eq!(row["name"], serde_json::Value::Null);
    assert_eq!(row["status"], "error");
    let diagnostics = row["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["severity"], "error");
    assert_eq!(diagnostics[0]["code"], "source_missing");
}

#[test]
fn package_cli_subprocess_doctor_json_reports_duplicate_name() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    write_duplicate_project_packages(workspace.path());

    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "doctor", "--json"])
        .output()
        .expect("run opi package doctor");
    assert!(
        !output.status.success(),
        "doctor should exit non-zero for duplicate packages: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let rows: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("doctor --json output should be valid JSON: {e}\ngot: {stdout}")
    });
    let diagnostic_codes: Vec<_> = rows
        .as_array()
        .expect("doctor rows")
        .iter()
        .flat_map(|row| row["diagnostics"].as_array().into_iter().flatten())
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert!(
        diagnostic_codes.contains(&"duplicate_name"),
        "doctor --json must surface duplicate_name diagnostics: {rows}"
    );
}

#[test]
fn package_cli_subprocess_list_json_reports_duplicate_name() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    write_duplicate_project_packages(workspace.path());

    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "list", "--json"])
        .output()
        .expect("run opi package list --json");
    assert!(
        output.status.success(),
        "list --json should report diagnostics without failing: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let rows: Vec<serde_json::Value> = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("valid NDJSON row"))
        .collect();
    assert_eq!(
        rows.iter().filter(|row| row["status"] == "ok").count(),
        1,
        "list --json should keep only one duplicate package active: {stdout}"
    );
    assert!(
        rows.iter().any(|row| {
            row["diagnostics"].as_array().is_some_and(|diagnostics| {
                diagnostics.iter().any(|d| d["code"] == "duplicate_name")
            })
        }),
        "list --json must include a duplicate_name diagnostic row: {stdout}"
    );
}

#[test]
fn package_cli_subprocess_remove() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();
    write_package(&workspace.path().join("pkg"), "pkg");

    // Add then remove
    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "add", "./pkg", "-l"])
        .output()
        .expect("run opi package add");
    assert!(output.status.success());

    let output = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "remove", "./pkg", "-l"])
        .output()
        .expect("run opi package remove");
    assert!(
        output.status.success(),
        "remove should succeed: stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn package_cli_subprocess_list_and_doctor_json_report_execution_lifecycle() {
    // Proves the `package list --json` and `package doctor --json` lifecycle
    // branches (trusted/enabled/contributions/drifted_adapters) through the
    // production `opi` binary against a globally-installed execution package.
    use sha2::{Digest, Sha256};

    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let user_config = tempfile::tempdir().expect("user config tempdir");
    let opi = opi_binary();

    let pkg = workspace.path().join("exec-pkg");
    std::fs::create_dir_all(pkg.join("bin")).unwrap();
    let exe_content: &[u8] = b"#!/bin/sh\necho hi\n";
    let exe = pkg.join("bin").join("opi-sandbox");
    std::fs::write(&exe, exe_content).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let sha = format!("{:x}", Sha256::digest(exe_content));
    let target = opi_coding_agent::package_activation::host_target_triple();
    std::fs::write(
        pkg.join("package.toml"),
        format!(
            "version = \"0.8.0\"\n\
             opi_version = \">=0.7,<0.8\"\n\
             name = \"opi-sandbox\"\n\
             description = \"execution backend\"\n\
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
        ),
    )
    .unwrap();

    let add = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "add"])
        .arg(&pkg)
        .output()
        .expect("run opi package add");
    assert!(
        add.status.success(),
        "add should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&add.stdout),
        String::from_utf8_lossy(&add.stderr)
    );

    // list --json: row carries trusted/enabled/contributions[*].
    let list = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "list", "--json"])
        .output()
        .expect("run opi package list --json");
    assert!(
        list.status.success(),
        "list --json should succeed: stderr={}",
        String::from_utf8_lossy(&list.stderr)
    );
    let list_row: serde_json::Value = serde_json::from_str(
        String::from_utf8_lossy(&list.stdout)
            .lines()
            .next()
            .expect("one NDJSON row"),
    )
    .unwrap();
    assert_eq!(list_row["name"], "opi-sandbox");
    assert_eq!(
        list_row["trusted"], false,
        "freshly added package is untrusted"
    );
    assert_eq!(
        list_row["enabled"], false,
        "freshly added package is disabled"
    );
    let contributions = list_row["contributions"]
        .as_array()
        .expect("contributions array present for execution package");
    assert_eq!(contributions.len(), 1);
    assert_eq!(contributions[0]["adapter_id"], "opi-sandbox");
    assert_eq!(contributions[0]["target"], target);
    assert_eq!(contributions[0]["protocol"], "command-execution-jsonl-v1");
    assert!(contributions[0]["executable_sha256"].is_string());

    // doctor --json: row carries trusted/enabled/drifted_adapters.
    let doc = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "doctor", "--json"])
        .output()
        .expect("run opi package doctor --json");
    let doc_rows: serde_json::Value = serde_json::from_str(
        String::from_utf8_lossy(&doc.stdout)
            .lines()
            .next()
            .expect("one JSON array row"),
    )
    .unwrap();
    let doc_row = &doc_rows.as_array().unwrap()[0];
    assert_eq!(doc_row["name"], "opi-sandbox");
    assert_eq!(doc_row["trusted"], false);
    assert_eq!(doc_row["enabled"], false);
    assert!(
        doc_row["drifted_adapters"]
            .as_array()
            .is_some_and(|a| a.is_empty()),
        "healthy execution package has no drift"
    );

    // doctor (text): reports execution lifecycle; drift surfaces after tamper.
    let doc_text = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "doctor"])
        .output()
        .expect("run opi package doctor (text)");
    let text_out = String::from_utf8_lossy(&doc_text.stdout);
    assert!(
        text_out.contains("ok (untrusted, disabled)"),
        "text doctor reports execution lifecycle: {text_out}"
    );

    // Tamper -> drift must surface in text doctor (exit 2).
    std::fs::write(&exe, b"#!/bin/sh\necho pwned\n").unwrap();
    let doc_text_drift = opi_command(&opi, workspace.path(), user_config.path())
        .args(["package", "doctor"])
        .output()
        .expect("run opi package doctor (text) after drift");
    assert!(
        !doc_text_drift.status.success(),
        "drifted execution package -> doctor exits non-zero"
    );
    let drift_out = String::from_utf8_lossy(&doc_text_drift.stdout);
    assert!(
        drift_out.contains("drifted"),
        "text doctor reports drift: {drift_out}"
    );
}
