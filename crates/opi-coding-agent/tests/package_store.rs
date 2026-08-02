//! Integration tests for the package store and source model (task 5.1).

use std::path::PathBuf;

use opi_coding_agent::package_store::{
    PackageDeclaration, PackageFilters, PackageLockEntry, PackageSource, PackageStore,
    PackageStoreError, PackageStoreScope,
};

// ---------------------------------------------------------------------------
// Source parsing
// ---------------------------------------------------------------------------

#[test]
fn parses_local_relative_source() {
    let source = PackageSource::parse("./vendor/todo").expect("parse source");
    assert!(matches!(source, PackageSource::Local { .. }));
    assert_eq!(source.identity_key().kind, "local");
}

#[test]
fn parses_local_absolute_source() {
    // Use a platform-appropriate absolute path
    let abs = if cfg!(windows) {
        r"C:\packages\review"
    } else {
        "/opt/packages/review"
    };
    let source = PackageSource::parse(abs).expect("parse source");
    assert!(matches!(source, PackageSource::Local { .. }));
    if let PackageSource::Local { path } = &source {
        assert!(path.is_absolute(), "expected absolute path, got {path:?}");
    } else {
        panic!("expected local source");
    }
}

#[test]
fn parses_github_shorthand_with_ref() {
    let source = PackageSource::parse("git:github.com/user/repo@v1").expect("parse source");
    match source {
        PackageSource::Git { url, refspec } => {
            assert_eq!(url, "https://github.com/user/repo");
            assert_eq!(refspec.as_deref(), Some("v1"));
        }
        PackageSource::Local { .. } => panic!("expected git source"),
    }
}

#[test]
fn parses_git_url_with_ref() {
    let source =
        PackageSource::parse("git:https://gitlab.com/org/pkg@refs/tags/v2.0").expect("parse");
    match source {
        PackageSource::Git { url, refspec } => {
            assert_eq!(url, "https://gitlab.com/org/pkg");
            assert_eq!(refspec.as_deref(), Some("refs/tags/v2.0"));
        }
        _ => panic!("expected git source"),
    }
}

#[test]
fn parses_git_url_without_ref() {
    let source = PackageSource::parse("git:https://example.com/pkg.git").expect("parse");
    match source {
        PackageSource::Git { url, refspec } => {
            assert_eq!(url, "https://example.com/pkg.git");
            assert!(refspec.is_none());
        }
        _ => panic!("expected git source"),
    }
}

#[test]
fn parses_ssh_git_url_without_ref() {
    let source = PackageSource::parse("git:ssh://git@github.com/user/repo").expect("parse");
    match source {
        PackageSource::Git { url, refspec } => {
            assert_eq!(url, "ssh://git@github.com/user/repo");
            assert!(refspec.is_none());
        }
        _ => panic!("expected git source"),
    }
}

#[test]
fn parses_ssh_git_url_with_ref() {
    let source = PackageSource::parse("git:ssh://git@github.com/user/repo@main").expect("parse");
    match source {
        PackageSource::Git { url, refspec } => {
            assert_eq!(url, "ssh://git@github.com/user/repo");
            assert_eq!(refspec.as_deref(), Some("main"));
        }
        _ => panic!("expected git source"),
    }
}

#[test]
fn rejects_scp_like_git_source_until_explicitly_supported() {
    let err = PackageSource::parse("git:git@github.com:user/repo@main").unwrap_err();
    assert!(
        err.to_string()
            .contains("scp-like git sources are not supported"),
        "expected scp-like guidance, got: {err}"
    );
}

#[test]
fn rejects_empty_source() {
    let err = PackageSource::parse("").unwrap_err();
    assert!(
        matches!(err, PackageStoreError::InvalidSource { .. }),
        "expected InvalidSource, got {err:?}"
    );
}

#[test]
fn rejects_unknown_prefix() {
    let err = PackageSource::parse("npm:some-package").unwrap_err();
    assert!(
        matches!(err, PackageStoreError::InvalidSource { .. }),
        "expected InvalidSource, got {err:?}"
    );
}

#[test]
fn git_source_identity_key_is_git() {
    let source = PackageSource::parse("git:github.com/a/b").expect("parse");
    assert_eq!(source.identity_key().kind, "git");
}

// ---------------------------------------------------------------------------
// Store scope paths
// ---------------------------------------------------------------------------

#[test]
fn project_scope_paths_live_under_dot_opi() {
    let root = PathBuf::from(r"C:\work\opi");
    let scope = PackageStoreScope::Project {
        workspace_root: root.clone(),
    };
    assert_eq!(scope.config_path(), root.join(".opi").join("packages.toml"));
    assert_eq!(
        scope.lock_path(),
        root.join(".opi").join("package-lock.toml")
    );
}

#[test]
fn global_scope_paths_live_under_config_dir() {
    let config = PathBuf::from("/home/user/.config/opi");
    let scope = PackageStoreScope::Global {
        user_config_dir: config.clone(),
    };
    assert_eq!(scope.config_path(), config.join("packages.toml"));
    assert_eq!(scope.lock_path(), config.join("package-lock.toml"));
}

#[test]
fn project_cache_dir_is_under_dot_opi() {
    let root = PathBuf::from("/tmp/workspace");
    let scope = PackageStoreScope::Project {
        workspace_root: root.clone(),
    };
    assert_eq!(scope.cache_dir(), root.join(".opi").join("package-cache"));
}

#[test]
fn global_cache_dir_is_under_config_dir() {
    let config = PathBuf::from("/home/user/.config/opi");
    let scope = PackageStoreScope::Global {
        user_config_dir: config.clone(),
    };
    assert_eq!(scope.cache_dir(), config.join("package-cache"));
}

// ---------------------------------------------------------------------------
// Declaration and lock read/write
// ---------------------------------------------------------------------------

#[test]
fn writes_and_reads_project_declarations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    store
        .write_declarations(&[PackageDeclaration {
            source: "./examples/todo".into(),
            filters: PackageFilters::default(),
        }])
        .expect("write declarations");
    let loaded = store.read_declarations().expect("read declarations");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].source, "./examples/todo");
}

#[test]
fn writes_and_reads_lock_entries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let entry = PackageLockEntry {
        identity_kind: "local".into(),
        identity_value: dir.path().join("pkg").display().to_string(),
        source: "./pkg".into(),
        package_root: dir.path().join("pkg"),
        cache_path: None,
        git_commit: None,
        manifest_sha256: "abc123".into(),
        contributions: Vec::new(),
    };
    store.write_lock(&[entry]).expect("write lock");
    let loaded = store.read_lock().expect("read lock");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].manifest_sha256, "abc123");
    assert!(loaded[0].git_commit.is_none());
}

#[test]
fn pre_phase16_lock_file_parses_with_empty_contributions() {
    // A pre-Phase-16 lock file has no `contributions` key; it must still parse
    // (forward compatibility for existing non-execution packages).
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let lock_path = dir.path().join(".opi").join("package-lock.toml");
    std::fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    std::fs::write(
        &lock_path,
        "[[lock]]\n\
         identity_kind = \"local\"\n\
         identity_value = \"/abs/pkg\"\n\
         source = \"./pkg\"\n\
         package_root = \"/abs/pkg\"\n\
         manifest_sha256 = \"abc\"\n",
    )
    .unwrap();
    let loaded = store.read_lock().expect("read lock");
    assert_eq!(loaded.len(), 1);
    assert!(
        loaded[0].contributions.is_empty(),
        "pre-Phase-16 lock parses with empty contributions"
    );
}

#[test]
fn lock_entry_with_git_commit_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let entry = PackageLockEntry {
        identity_kind: "git".into(),
        identity_value: "https://github.com/user/repo".into(),
        source: "git:github.com/user/repo@v1".into(),
        package_root: dir.path().join("cache").join("repo"),
        cache_path: Some(dir.path().join("cache")),
        git_commit: Some("a1b2c3d4e5f6".into()),
        manifest_sha256: "deadbeef".into(),
        contributions: Vec::new(),
    };
    store.write_lock(&[entry]).expect("write lock");
    let loaded = store.read_lock().expect("read lock");
    assert_eq!(loaded[0].git_commit.as_deref(), Some("a1b2c3d4e5f6"));
    assert_eq!(
        loaded[0].cache_path.as_ref().unwrap(),
        &dir.path().join("cache")
    );
}

#[test]
fn read_declarations_returns_empty_when_file_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let loaded = store.read_declarations().expect("read");
    assert!(loaded.is_empty());
}

#[test]
fn read_lock_returns_empty_when_file_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let loaded = store.read_lock().expect("read");
    assert!(loaded.is_empty());
}

#[test]
fn declaration_with_filters_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: dir.path().to_path_buf(),
    });
    let decl = PackageDeclaration {
        source: "./vendor/pkg".into(),
        filters: PackageFilters {
            extensions: Some(vec!["my-ext".into()]),
            skills: Some(vec!["review".into()]),
            fragments: None,
            themes: None,
        },
    };
    store
        .write_declarations(std::slice::from_ref(&decl))
        .expect("write");
    let loaded = store.read_declarations().expect("read");
    let ext: Vec<String> = vec!["my-ext".into()];
    let skl: Vec<String> = vec!["review".into()];
    assert_eq!(
        loaded[0].filters.extensions.as_deref(),
        Some(ext.as_slice())
    );
    assert_eq!(loaded[0].filters.skills.as_deref(), Some(skl.as_slice()));
    assert!(loaded[0].filters.fragments.is_none());
}

// ---------------------------------------------------------------------------
// Windows-style path coverage
// ---------------------------------------------------------------------------

#[test]
fn windows_backslash_source_parses_as_local() {
    let source = PackageSource::parse(r".\vendor\todo").expect("parse backslash");
    assert!(matches!(source, PackageSource::Local { .. }));
}

#[test]
fn windows_absolute_source_parses() {
    let source = PackageSource::parse(r"D:\packages\my-tool").expect("parse windows abs");
    if let PackageSource::Local { path } = &source {
        // `D:\` is only recognized as a drive-absolute root on Windows; on
        // Unix the same bytes parse as a relative PathBuf, so assert the
        // drive-absolute property on Windows and a plain round-trip elsewhere.
        #[cfg(windows)]
        assert!(path.is_absolute());
        #[cfg(not(windows))]
        assert_eq!(path, std::path::Path::new(r"D:\packages\my-tool"));
    } else {
        panic!("expected local source");
    }
}

// ---------------------------------------------------------------------------
// Bare git repository clone/ref-pin fixture
// ---------------------------------------------------------------------------

#[test]
fn git_clone_and_ref_pin_from_bare_repo() {
    // Create a bare git repository with a known commit, then verify that
    // PackageStore can clone it and pin to a ref without touching real user
    // config or network.
    let tmp = tempfile::tempdir().expect("tempdir");
    let bare_dir = tmp.path().join("bare.git");
    std::fs::create_dir_all(&bare_dir).expect("create bare dir");

    // Initialize bare repo
    let status = std::process::Command::new("git")
        .args(["init", "--bare"])
        .arg(&bare_dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@test.com")
        .output()
        .expect("git init --bare");
    assert!(status.status.success(), "git init --bare failed");

    // Create a working clone, add a file, push to bare
    let work_dir = tmp.path().join("work");
    std::fs::create_dir_all(&work_dir).expect("create work dir");

    let git = |args: &[&str]| -> std::process::Output {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&work_dir)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .expect("git command")
    };

    let out = git(&["init"]);
    assert!(
        out.status.success(),
        "git init work failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Write a minimal package.toml
    let pkg_dir = work_dir.join("my-pkg");
    std::fs::create_dir_all(&pkg_dir).expect("create pkg dir");
    std::fs::write(
        pkg_dir.join("package.toml"),
        r#"name = "my-pkg"
description = "Test package"
"#,
    )
    .expect("write package.toml");

    let out = git(&["add", "."]);
    assert!(
        out.status.success(),
        "git add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = git(&["commit", "-m", "initial"]);
    assert!(
        out.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Get the commit SHA
    let rev_out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&work_dir)
        .output()
        .expect("git rev-parse");
    let commit_sha = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();
    assert!(!commit_sha.is_empty(), "empty commit sha");

    // Push to bare repo
    let bare_url = format!(
        "file:///{}",
        bare_dir.display().to_string().replace('\\', "/")
    );
    let out = git(&["remote", "add", "origin", &bare_url]);
    assert!(
        out.status.success(),
        "git remote add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = git(&["push", "-u", "origin", "master"]);
    if !out.status.success() {
        // try main branch
        let out2 = git(&["push", "-u", "origin", "main"]);
        // One of these should succeed
        assert!(
            out.status.success() || out2.status.success(),
            "git push failed: {} / {}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out2.stderr)
        );
    }

    // Now use PackageStore to clone the bare repo with a ref pin
    let store_dir = tmp.path().join("store-workspace");
    std::fs::create_dir_all(&store_dir).expect("create store workspace");

    let store = PackageStore::new(PackageStoreScope::Project {
        workspace_root: store_dir.clone(),
    });

    let clone_dir = store.cache_dir().join("my-pkg");
    let result = store.git_clone(&bare_url, Some(&commit_sha), &clone_dir);
    assert!(result.is_ok(), "git_clone failed: {:?}", result.err());

    // Verify the cloned repo has the package.toml
    let cloned_toml = clone_dir.join("my-pkg").join("package.toml");
    assert!(
        cloned_toml.exists(),
        "cloned package.toml not found at {cloned_toml:?}"
    );

    // Verify the checkout is at the pinned commit
    let head_out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&clone_dir)
        .output()
        .expect("git rev-parse in clone");
    let head_sha = String::from_utf8_lossy(&head_out.stdout).trim().to_string();
    assert_eq!(head_sha, commit_sha, "HEAD does not match pinned commit");
}
