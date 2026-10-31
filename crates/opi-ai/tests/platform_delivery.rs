//! Task 20.58 platform-delivery freeze (DEL-14, P20-S58, ONB-06-20.58).
//!
//! These tests pin the final delivered supply-chain identity of the workspace
//! so that any silent drift - a member rename, an unpinned native dependency,
//! a new `build.rs`, a new `unsafe` block, a repackaged lockfile - fails the
//! ordinary workspace gate instead of shipping. They read repository files
//! relative to `CARGO_MANIFEST_DIR`; they never touch the network, the
//! registry or user runtime data.
//!
//! Native behavioral conformance is this binary itself executing on the
//! declared hosts: the suite is part of `cargo test -p opi-ai`, which ran on
//! Linux x86_64 (rustc 1.97.1, exactly the declared MSRV) and on Windows
//! x86_64-pc-windows-msvc during task 20.58 evidence collection. macOS has no
//! owning host here and stays an explicit CI claim, not local proof.
//!
//! Predecessor ownership is unchanged: per-adapter behavioral suites, the
//! assembled `external_sdk` matrix (20.57), packaging proofs
//! (`run_readme_examples.py`, `package_slice.py --family ws`) and the schema
//! data profile (`run_early_package_proof.py`, RDF-21) keep their evidence
//! roles; nothing here re-proves those identities.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository root (crates/opi-ai/../..).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("required delivery input {rel} missing: {e}"))
}

/// Recursively collects `*.rs` files under `dir`, sorted for stable output.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The exact seven-member workspace graph frozen by task 20.58.
const FROZEN_MEMBERS: &[&str] = &[
    "crates/opi-ai",
    "crates/opi-agent",
    "crates/opi-coding-agent",
    "crates/opi-eval",
    "crates/opi-protocol",
    "crates/opi-sandbox",
    "crates/opi-tui",
];

/// Security-critical pinned closure (RDF-21/22/23/24 selections): every crate
/// the root manifest pins exactly, frozen at its Cargo-generated lock version
/// and registry checksum. An upgrade that changes any row requires the
/// documented re-review of the affected profile.
const FROZEN_PINNED_CLOSURE: &[(&str, &str, &str)] = &[
    // Private WS profile (RDF-24): exact libz-rs-sys/zlib-rs/flate2 sources.
    (
        "libz-rs-sys",
        "0.6.8",
        "434324ee61a8e4a05dd3fdf03d51c073ea7fdaf5a35b2ca33c4ca9f3f37def4a",
    ),
    (
        "flate2",
        "1.1.10",
        "6e634e2e0ebac1ee034020da1ca582e17ffe4e0f5e985823721e168928136dcb",
    ),
    // Upstream schema data profile (RDF-21).
    (
        "regress",
        "0.12.0",
        "32eef8b209c3c1c15dbad02c1f30f9539f00dc7253e0cbcdaae442a50a09d7c1",
    ),
    (
        "ryu-js",
        "1.0.2",
        "dd29631678d6fb0903b69223673e122c32e9ae559d0960a38d574695ebc0ea15",
    ),
    (
        "unicode-normalization",
        "0.1.26",
        "75cd3fb03d5288b3ba9a1ee103ac7e9cbb59f0b03d64f8ddfcc04dd42758a85d",
    ),
    // Upstream HTTP transport profile (RDF-22).
    (
        "reqwest",
        "0.12.28",
        "eddd3ca559203180a307f12d114c268abf583f59b03cb906fd0b3ff8646c1147",
    ),
    // Upstream crypto profile (RDF-23): AWS-LC via vendored C, no global
    // provider installed by opi-ai.
    (
        "rustls",
        "0.23.40",
        "ef86cd5876211988985292b91c96a8f2d298df24e75989a43a3c73f2d4d8168b",
    ),
    (
        "aws-lc-rs",
        "1.18.2",
        "faac5829c2b74c28f830747e7818ccfb684261b5f48a1118b1e2a13d36dfab13",
    ),
    (
        "aws-lc-sys",
        "0.46.0",
        "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
    ),
    // Private AWS STS protocol codecs (rust-backends 3.3).
    (
        "aws-smithy-query",
        "0.62.1",
        "f1d1d71f6562be974caa85442ecd90194c40fdb5df045f182a6c2e872ce95056",
    ),
    (
        "aws-smithy-xml",
        "0.62.1",
        "0b932c8d6dc127fc980eecd78f8694ae9b9551b69a93a7def2a199c1c0033daf",
    ),
    (
        "xmlparser",
        "0.13.5",
        "4d25c75bf9ea12c4040a97f829154768bbbce366287e2dc044af160cd79a13fd",
    ),
    // Owned AWS SSO/SignIn crypto.
    (
        "p256",
        "0.13.2",
        "c9863ad85fa8f4460f9c48cb909d38a0d689dba1f6f6988a5e3e0d31071bcd4b",
    ),
    (
        "sha1",
        "0.10.6",
        "e3bf829a2d51ab4a5ddf1352d8470c140cadc8301b2ae1789db023f01cedd6ba",
    ),
    // Exact transitive pin for protocol-NACK loopback regressions.
    (
        "h2",
        "0.4.14",
        "171fefbc92fe4a4de27e0698d6a5b392d6a0e333506bc49133760b3bcf948733",
    ),
];

/// The exact workspace `src/` files that contain the `unsafe` token, frozen
/// per task 20.58 audit. Every entry owns a documented safety argument; any
/// new file or removal here is a deliberate delivery change.
const FROZEN_UNSAFE_FILES: &[&str] = &[
    "crates/opi-agent/src/harness.rs",
    "crates/opi-ai/src/auth/aws/process/windows.rs",
    "crates/opi-ai/src/schema/ir.rs",
    "crates/opi-ai/src/schema/refs.rs",
    "crates/opi-ai/src/telemetry.rs",
    "crates/opi-ai/src/ws/io.rs",
    "crates/opi-ai/src/ws/pmd.rs",
    "crates/opi-coding-agent/src/execution/contribution.rs",
    "crates/opi-coding-agent/src/main.rs",
    "crates/opi-coding-agent/src/provider_factory.rs",
    "crates/opi-coding-agent/src/tool/process_tree.rs",
    "crates/opi-eval/src/benchmark/process.rs",
    "crates/opi-eval/src/lib.rs",
    "crates/opi-eval/src/native_capture_runtime.rs",
    "crates/opi-eval/src/process/tree.rs",
    "crates/opi-eval/src/regrade.rs",
    "crates/opi-eval/src/runner/experiment.rs",
    "crates/opi-eval/src/runner/material.rs",
    "crates/opi-sandbox/src/platform/linux.rs",
    "crates/opi-sandbox/src/platform/macos.rs",
    "crates/opi-sandbox/src/platform/mod.rs",
    "crates/opi-sandbox/src/process_tree.rs",
];

/// `true` when the file contains an `unsafe` token (word boundary), matching
/// the audit's grep. Comment-only mentions are intentionally included: the
/// freeze is textual so that even a doc mention cannot drift silently.
fn contains_unsafe_token(path: &Path) -> bool {
    let Ok(src) = fs::read_to_string(path) else {
        return false;
    };
    let bytes = src.as_bytes();
    let needle = b"unsafe";
    for i in 0..bytes.len().saturating_sub(5) {
        if &bytes[i..i + 6] != needle {
            continue;
        }
        let before_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let after = i + 6;
        let after_ok =
            after >= bytes.len() || !(bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Parses Cargo.lock into (name, version, checksum) package stanzas.
fn lock_packages() -> Vec<(String, String, Option<String>)> {
    let src = read("Cargo.lock");
    src.split("[[package]]")
        .skip(1)
        .map(|stanza| {
            let name = stanza
                .lines()
                .find_map(|l| l.strip_prefix("name = "))
                .map(|v| v.trim_matches('"').to_string())
                .expect("lock stanza with name");
            let version = stanza
                .lines()
                .find_map(|l| l.strip_prefix("version = "))
                .map(|v| v.trim_matches('"').to_string())
                .expect("lock stanza with version");
            let checksum = stanza
                .lines()
                .find_map(|l| l.strip_prefix("checksum = "))
                .map(|v| v.trim_matches('"').to_string());
            (name, version, checksum)
        })
        .collect()
}

#[test]
fn root_workspace_graph_is_frozen() {
    let src = read("Cargo.toml");

    let members_block = src
        .split("[workspace.package]")
        .next()
        .expect("workspace section");
    for member in FROZEN_MEMBERS {
        assert!(
            members_block.contains(&format!("\"{member}\"")),
            "frozen member {member} missing from workspace graph"
        );
    }
    assert_eq!(
        members_block.matches("crates/").count(),
        FROZEN_MEMBERS.len(),
        "workspace member set changed; update the 20.58 freeze deliberately"
    );
    assert!(members_block.contains("resolver = \"2\""));

    for field in [
        "version = \"0.8.3\"",
        "edition = \"2024\"",
        "rust-version = \"1.97\"",
        "license = \"MIT\"",
        "repository = \"https://github.com/OdradekAI/opi\"",
    ] {
        assert!(
            src.contains(field),
            "workspace.package field drifted: {field}"
        );
    }

    // Internal dependency edges carry both the path and the published
    // version, in lockstep with workspace.package.version.
    for dep in [
        "opi-ai",
        "opi-agent",
        "opi-coding-agent",
        "opi-protocol",
        "opi-tui",
    ] {
        assert!(
            src.contains(&format!(
                "{dep} = {{ path = \"crates/{dep}\", version = \"0.8.3\" }}"
            )),
            "internal dep {dep} lost its lockstep path+version edge"
        );
    }
}

#[test]
fn opi_ai_manifest_has_a_single_feature_union() {
    let src = read("crates/opi-ai/Cargo.toml");

    // The SDK publishes exactly one feature union: default. Any optional
    // dependency or feature would fork the delivered API surface and must be
    // a deliberate, documented delivery change.
    assert!(
        !src.contains("[features]"),
        "opi-ai grew a [features] table; single-union contract broken"
    );
    assert!(
        !src.contains("optional = true"),
        "opi-ai grew an optional dependency; single-union contract broken"
    );

    // Everything except the documented cfg(windows) target table resolves
    // through the root workspace so the root graph stays the only source of
    // versions and features.
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('[') || trimmed.is_empty() {
            continue; // comments and section headers are not dependency lines
        }
        if trimmed.contains("= {") {
            assert!(
                trimmed.contains("workspace = true"),
                "opi-ai declares a non-workspace dependency line: {trimmed}"
            );
        }
    }
    assert!(
        src.contains("[target.'cfg(windows)'.dependencies]"),
        "cfg(windows) target table expected"
    );
    // Package metadata ships the license and the onboarding manual with every
    // packaged artifact.
    assert!(src.contains("license.workspace = true"));
    assert!(src.contains("readme = \"README.md\""));
}

#[test]
fn lockfile_is_cargo_generated_and_members_are_unique() {
    let src = read("Cargo.lock");
    assert!(
        src.starts_with("# This file is automatically @generated by Cargo."),
        "lockfile lost its Cargo-generation marker"
    );
    assert!(
        src.contains("# It is not intended for manual editing"),
        "lockfile manual-edit warning marker missing"
    );
    assert!(
        src.contains("version = 4"),
        "lockfile format version drifted"
    );

    let packages = lock_packages();

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (name, _, _) in &packages {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }
    for member in [
        "opi-ai",
        "opi-agent",
        "opi-coding-agent",
        "opi-eval",
        "opi-protocol",
        "opi-sandbox",
        "opi-tui",
    ] {
        let count = counts.get(member).copied().unwrap_or(0);
        assert_eq!(count, 1, "member {member} appears {count}x in lockfile");
        let (_, version, _) = packages
            .iter()
            .find(|(n, _, _)| n == member)
            .expect("member in lockfile");
        assert_eq!(version, "0.8.3", "member {member} version drift");
    }
}

#[test]
fn security_pinned_closure_checksums_are_frozen() {
    let packages = lock_packages();
    for (name, version, checksum) in FROZEN_PINNED_CLOSURE {
        let found = packages
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("pinned crate {name} missing from lockfile"));
        assert_eq!(&found.1, version, "pin drift for {name}");
        let frozen = checksum.to_string();
        assert_eq!(
            found.2.as_deref(),
            Some(frozen.as_str()),
            "checksum drift for {name} {version}; a pinned-source upgrade must be re-reviewed"
        );
    }
}

#[test]
fn build_scripts_are_limited_to_the_recorded_owners() {
    let root = repo_root();
    let mut found = Vec::new();
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).expect("crates walk");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "build.rs") {
                found.push(path);
            }
        }
    }
    found.sort();
    let expected = vec![
        root.join("crates/opi-coding-agent/build.rs"),
        root.join("crates/opi-sandbox/build.rs"),
    ];
    assert_eq!(found, expected, "build-script ownership changed");
    // The SDK itself stays build-script-free: its native closure is entirely
    // upstream (aws-lc-sys vendored C via cmake, libz-rs-sys Rust).
    assert!(!root.join("crates/opi-ai/build.rs").exists());
}

#[test]
fn unsafe_inventory_is_frozen() {
    let root = repo_root();
    let mut found: Vec<String> = Vec::new();
    for member in FROZEN_MEMBERS {
        let mut files = Vec::new();
        rust_files(&root.join(member).join("src"), &mut files);
        for path in files.iter().filter(|p| contains_unsafe_token(p)) {
            found.push(
                path.strip_prefix(&root)
                    .expect("workspace-relative")
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    found.sort();
    let expected: Vec<String> = FROZEN_UNSAFE_FILES.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        found, expected,
        "unsafe inventory changed; every new entry needs a documented safety owner"
    );
}

#[test]
fn proc_macro_crates_are_transitive_only() {
    // No member ships a proc-macro crate type, and the only direct
    // proc-macro-implementation dependencies in the workspace are
    // opi-sandbox test fixtures (proc-macro2/syn under [dev-dependencies]).
    for member in FROZEN_MEMBERS {
        let src = read(&format!("{member}/Cargo.toml"));
        assert!(
            !src.contains("proc-macro = true"),
            "{member} declares a proc-macro crate type"
        );
        let production = src
            .split("[dev-dependencies]")
            .next()
            .expect("manifest head");
        for token in ["proc-macro2", "syn =", "quote"] {
            assert!(
                !production.contains(token),
                "{member} has a production proc-macro dependency ({token})"
            );
        }
    }
    let sandbox = read("crates/opi-sandbox/Cargo.toml");
    let dev = sandbox
        .split("[dev-dependencies]")
        .nth(1)
        .expect("dev deps");
    assert!(dev.contains("proc-macro2 = { workspace = true }"));
    assert!(dev.contains("syn = { workspace = true }"));

    // The only feature union in the workspace is the coding-agent
    // execution-backend-test-fixture flag, an empty test-only selector.
    let coding = read("crates/opi-coding-agent/Cargo.toml");
    assert!(coding.contains("[features]"));
    assert!(coding.contains("execution-backend-test-fixture = []"));
    let coding_feature_block = coding
        .split("[features]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("feature block");
    let declared: Vec<&str> = coding_feature_block
        .lines()
        .filter(|l| l.contains('=') && !l.trim_start().starts_with('#'))
        .collect();
    assert_eq!(
        declared,
        vec!["execution-backend-test-fixture = []"],
        "workspace feature-union set changed"
    );
}

#[test]
fn license_and_notice_ship_with_the_sdk() {
    // Root license governs the workspace; the SDK additionally ships an
    // explicit LICENSE + NOTICE pair that cargo packages into the .crate
    // (byte-exact inclusion proven by the packaging harness in 20.57).
    let root_license = read("LICENSE");
    assert!(
        root_license.contains("MIT"),
        "root license drifted from MIT"
    );
    let license = read("crates/opi-ai/LICENSE");
    assert_eq!(license, root_license, "SDK LICENSE diverged from root");
    let notice = read("crates/opi-ai/NOTICE");
    assert!(
        notice.lines().any(|l| !l.trim().is_empty()),
        "SDK NOTICE is empty"
    );
}

#[test]
fn native_platform_conformance_runs_on_declared_hosts() {
    // This assertion block executes natively on every host that runs the
    // workspace test gate; during 20.58 evidence collection that was Linux
    // x86_64-unknown-linux-gnu (rustc exactly 1.97.1, the declared MSRV) and
    // Windows x86_64-pc-windows-msvc. macOS has no owning host in that
    // evidence set and remains an explicit CI claim.
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let env_abi = if cfg!(target_env = "gnu") {
        "gnu"
    } else if cfg!(target_env = "msvc") {
        "msvc"
    } else {
        ""
    };

    match os {
        "linux" => {
            assert_eq!(arch, "x86_64");
            assert_eq!(env_abi, "gnu");
        }
        "windows" => {
            assert!(arch == "x86_64" || arch == "aarch64");
            assert_eq!(env_abi, "msvc");
        }
        "macos" => {
            assert_eq!(env_abi, "");
        }
        other => panic!("executed on undeclared platform {other}"),
    }

    // The declared MSRV is a workspace fact, not a per-crate option: every
    // member inherits the same rust-version through `workspace = true`.
    let opi_ai = read("crates/opi-ai/Cargo.toml");
    assert!(opi_ai.contains("rust-version.workspace = true"));
}
