//! Documentation, non-goal, and structure guard tests for Phase 12 provider
//! correctness (task 12.9).
//!
//! These guards implement the Phase 12 design's "Documentation Updates",
//! "Non-Goals", and "Success Criteria 7 + 9":
//!
//! - **Docs/profile-policy sync** (`provider_docs_and_profile_policy_stay_in_sync`)
//!   — root, opi-ai, and opi-coding-agent READMEs (EN+ZH) plus the docs/opi-spec
//!   Phase 12/13 sections state the provider/protocol matrix, the OpenAI-compatible
//!   profile policy and `CompatConfig` flags, OpenAI Responses native semantics
//!   implemented vs explicitly deferred (`previous_response_id`), cache-control,
//!   response-ID round-trip, session-affinity, per-provider thinking/image
//!   behavior, proxy, best-effort cost, and the Phase 13 handoff. Load-bearing
//!   identifiers are source-anchored where a stable name exists.
//! - **SC7 first-class guard** (`first_class_provider_guard`) — the opi-ai
//!   first-class provider module set is exactly the nine built-in families; a
//!   new module cannot appear without updating the allow-list (the "graph
//!   update"). Closes the 12.3 forward-reference.
//! - **SC9 non-goals** (`phase12_non_goals_not_in_core`) — the ten Phase 12
//!   non-goals are documented as deferred and are absent from core through
//!   structural positives (no forbidden crate deps, no forbidden modules).
//! - **Network-free** (`default_provider_tests_are_network_free`) — default
//!   provider tests are fixture/wiremock/MockProvider, carry the no-live-calls
//!   module-doc convention, and do not read real provider credentials outside
//!   `#[ignore]`-gated tests.

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Path/file helpers (match the Phase 6/7/8/11 doc-guard convention).
// ---------------------------------------------------------------------------

/// Read a file relative to the repo root (two levels up from CARGO_MANIFEST_DIR).
fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Case-insensitive substring check.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Doc surfaces task 12.9 owns (English + Simplified-Chinese counterparts).
fn doc_surfaces() -> Vec<&'static str> {
    vec![
        "README.md",
        "README.zh.md",
        "crates/opi-ai/README.md",
        "crates/opi-ai/README.zh.md",
        "crates/opi-coding-agent/README.md",
        "crates/opi-coding-agent/README.zh.md",
        "docs/opi-spec.md",
        "docs/opi-spec.zh.md",
        "docs/pi-alignment-matrix.md",
        "docs/pi-alignment-matrix.zh.md",
    ]
}

/// The nine built-in first-class provider families as `(lib.rs module,
/// provider_factory build fn)`. Mirrors `crates/opi-ai/src/lib.rs` `pub mod`
/// declarations and `crates/opi-coding-agent/src/provider_factory.rs`.
const FIRST_CLASS_PROVIDERS: &[(&str, &str)] = &[
    ("anthropic", "build_anthropic"),
    ("openai_chat", "build_openai"),
    ("openai_responses", "build_openai_responses"),
    ("openrouter", "build_openrouter"),
    ("mistral", "build_mistral"),
    ("gemini", "build_gemini"),
    ("bedrock", "build_bedrock"),
    ("azure_openai", "build_azure"),
    ("vertex", "build_vertex"),
];

/// opi-ai public modules that are NOT providers (excluded from the first-class
/// provider set when parsing lib.rs / scanning opi-ai/src).
const NON_PROVIDER_MODULES: &[&str] = &[
    "config",
    "http",
    "message",
    "model",
    "provider",
    "provider_collection",
    "registry",
    "retry",
    "stream",
    "test_support",
];

/// The ten Phase 12 non-goals (design doc, "Non-Goals") with an English and a
/// Simplified-Chinese token each, used to confirm the opi-ai README non-goal
/// list carries all ten in both languages.
const PHASE12_NON_GOALS: &[(&str, &str)] = &[
    ("OAuth login", "OAuth 登录"),
    ("Anthropic subscription auth", "Anthropic 订阅鉴权"),
    ("OpenAI Codex subscription auth", "OpenAI Codex 订阅鉴权"),
    ("GitHub Copilot auth", "GitHub Copilot 鉴权"),
    (
        "broad new first-class provider list",
        "first-class provider 列表",
    ),
    ("Image generation", "图像生成"),
    ("Browser usage", "浏览器使用"),
    ("streaming-adapter protocol", "流式 adapter 协议"),
    ("Paid live provider calls", "付费实时 provider 调用"),
    (
        "provider-specific config file format",
        "provider 专用配置文件格式",
    ),
];

// ===========================================================================
// Docs + OpenAI-compatible profile policy stay in sync (EN + ZH)
// ===========================================================================

/// README/spec surfaces state the provider matrix, profile policy + flags,
/// OpenAI Responses implemented-vs-deferred semantics, cache/response-ID/
/// session-affinity, thinking/image behavior, proxy, best-effort cost, and the
/// Phase 13 handoff. Load-bearing identifiers are pinned in both languages.
#[test]
fn provider_docs_and_profile_policy_stay_in_sync() {
    let opi_ai = read_repo_file("crates/opi-ai/README.md");
    let opi_ai_zh = read_repo_file("crates/opi-ai/README.zh.md");
    let coding = read_repo_file("crates/opi-coding-agent/README.md");
    let root = read_repo_file("README.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    // (1) All 9 provider modules are listed in the opi-ai README provider table.
    for (module, _) in FIRST_CLASS_PROVIDERS {
        assert!(
            contains_ci(&opi_ai, &format!("`{module}`")),
            "opi-ai README must list provider module `{module}`"
        );
    }
    // Root README lists the 9 provider ids.
    for id in [
        "anthropic",
        "openai",
        "openai-responses",
        "openrouter",
        "mistral",
        "gemini",
        "bedrock",
        "azure",
        "vertex",
    ] {
        assert!(
            contains_ci(&root, id),
            "root README must list provider id `{id}`"
        );
    }

    // (2) Auth method per provider: Bedrock SigV4 is named.
    assert!(
        contains_ci(&opi_ai, "SigV4"),
        "opi-ai README must state Bedrock uses AWS SigV4 signing"
    );

    // (3) OpenAI-compatible profile policy: config-driven is the preferred path.
    assert!(
        contains_ci(&opi_ai, "config-driven") && contains_ci(&opi_ai, "preferred"),
        "opi-ai README must state config-driven profiles are the preferred breadth path"
    );

    // (4) Profile flags: the eight CompatConfig field names verbatim (anchored
    //     to the struct at crates/opi-ai/src/openai_chat.rs). `extra_headers`
    //     is intentionally excluded here — it is a per-profile config field /
    //     OpenAiChatProvider constructor concern, not a CompatConfig flag; the
    //     session-affinity check below pins its documentation separately.
    for flag in [
        "system_role_override",
        "max_tokens_field",
        "tool_result_name_field",
        "usage_in_stream",
        "strict_tool_schema",
        "reasoning_effort",
        "cache_key",
        "require_assistant_after_tool_result",
    ] {
        assert!(
            contains_ci(&opi_ai, flag),
            "opi-ai README must document profile flag `{flag}`"
        );
        assert!(
            contains_ci(&coding, flag),
            "opi-coding-agent README must document profile flag `{flag}`"
        );
    }
    // ModelCompatOverride (model > provider precedence).
    assert!(
        contains_ci(&opi_ai, "ModelCompatOverride"),
        "opi-ai README must document ModelCompatOverride (model-over-provider precedence)"
    );

    // (5) OpenAI Responses native semantics: implemented (ResponsesConfig) AND
    //     explicitly deferred (previous_response_id). Source-anchored: the
    //     deferral comment lives in openai_responses.rs.
    let responses_src = read_repo_file("crates/opi-ai/src/openai_responses.rs");
    assert!(
        responses_src.contains("previous_response_id")
            && responses_src.contains("intentionally absent"),
        "openai_responses.rs must document the previous_response_id deferral (source anchor)"
    );
    for token in [
        "ResponsesConfig",
        "store",
        "strict_tools",
        "previous_response_id",
    ] {
        assert!(
            contains_ci(&opi_ai, token),
            "opi-ai README must name Responses semantic `{token}`"
        );
    }

    // (6) Response-ID round-trip + Bedrock reasoningContent parser limitation.
    assert!(
        contains_ci(&opi_ai, "response_id"),
        "opi-ai README must document response-ID round-trip into response_id"
    );
    assert!(
        contains_ci(&opi_ai, "reasoningContent"),
        "opi-ai README must document the Bedrock reasoningContent parser limitation"
    );
    // Session affinity: extra_headers is a per-profile static-header mechanism
    // (config.rs profile field / OpenAiChatProvider constructor), NOT a
    // CompatConfig flag. Documented in the session-affinity section.
    assert!(
        contains_ci(&opi_ai, "extra_headers"),
        "opi-ai README must document extra_headers as the session-affinity header mechanism"
    );

    // (7) Best-effort cost semantics.
    assert!(
        contains_ci(&opi_ai, "best-effort") || contains_ci(&opi_ai, "best effort"),
        "opi-ai README must state cost is best-effort"
    );

    // (8) Proxy precedence.
    assert!(
        contains_ci(&opi_ai, "HTTPS_PROXY") && contains_ci(&opi_ai, "NO_PROXY"),
        "opi-ai README must document the proxy env precedence"
    );

    // (9) Phase 13 handoff in the spec.
    assert!(
        contains_ci(&spec, "Phase 13 handoff")
            && contains_ci(&spec, "provider-correct")
            && contains_ci(&spec, "provider-specific internals"),
        "docs/opi-spec.md must carry the Phase 13 handoff (sessions rely on provider-correct data, not provider-specific internals)"
    );
    assert!(
        contains_ci(&spec_zh, "Phase 13 交接") || contains_ci(&spec_zh, "第十三阶段"),
        "docs/opi-spec.zh.md must carry a Phase 13 handoff section"
    );

    // (10) Spec Phase 12 section is expanded (not the old 'planned' stub).
    assert!(
        contains_ci(&spec, "CompatConfig") && contains_ci(&spec, "previous_response_id"),
        "docs/opi-spec.md Phase 12 section must name CompatConfig + the previous_response_id deferral"
    );

    // --- ZH mirror carries the same load-bearing identifiers. ---
    for token in [
        "previous_response_id",
        "require_assistant_after_tool_result",
        "response_id",
        "reasoningContent",
        "ResponsesConfig",
        "CompatConfig",
        "ModelCompatOverride",
        "SigV4",
    ] {
        assert!(
            opi_ai_zh.contains(token),
            "opi-ai README.zh must carry the load-bearing identifier `{token}`"
        );
    }
}

// ===========================================================================
// SC7: first-class provider module set is exactly the nine built-ins
// ===========================================================================

/// The opi-ai first-class provider module set is exactly the nine built-in
/// families, declared in lib.rs, present on disk, and built by provider_factory.
/// A tenth module cannot appear without updating this allow-list (the graph
/// update the DoD requires).
#[test]
fn first_class_provider_guard() {
    let lib_rs = read_repo_file("crates/opi-ai/src/lib.rs");

    // (1) Parse `pub mod <name>;` declarations.
    let declared: std::collections::HashSet<String> = lib_rs
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|rest| rest.split(';').next())
        .map(str::trim)
        .map(str::to_owned)
        .collect();

    // (2) Declared provider set = declared - non-provider support modules.
    let non_provider: std::collections::HashSet<&str> =
        NON_PROVIDER_MODULES.iter().copied().collect();
    let declared_providers: std::collections::HashSet<&str> = declared
        .iter()
        .map(String::as_str)
        .filter(|name| !non_provider.contains(*name))
        .collect();

    // (3) Expected provider set (hardcoded allow-list).
    let expected: std::collections::HashSet<&str> =
        FIRST_CLASS_PROVIDERS.iter().map(|(m, _)| *m).collect();
    assert_eq!(
        declared_providers, expected,
        "opi-ai first-class provider module set must be exactly the nine built-ins; \
         adding a tenth requires updating this allow-list (the graph update)"
    );

    // (4) Filesystem matches lib.rs (no undeclared module file/dir, no missing file).
    let opi_ai_src = repo_root().join("crates/opi-ai/src");
    let mut fs_modules: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in std::fs::read_dir(&opi_ai_src).expect("read opi-ai/src") {
        let path = entry.expect("dir entry").path();
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let is_rs = path.extension().is_some_and(|e| e == "rs") && name != "lib";
        let is_dir = path.is_dir();
        if (is_rs || is_dir) && !non_provider.contains(name) {
            fs_modules.insert(name.to_owned());
        }
    }
    let fs_providers: std::collections::HashSet<&str> =
        fs_modules.iter().map(String::as_str).collect();
    assert_eq!(
        fs_providers, expected,
        "opi-ai/src provider files/dirs must match the lib.rs declarations"
    );

    // (5) Factory builds exactly these families (vacuous-allowlist self-check).
    let factory_src = read_repo_file("crates/opi-coding-agent/src/provider_factory.rs");
    let mut missing_build: Vec<&str> = Vec::new();
    for (_, build_fn) in FIRST_CLASS_PROVIDERS {
        let token = format!("fn {build_fn}");
        if !factory_src.contains(&token) {
            missing_build.push(build_fn);
        }
    }
    assert!(
        missing_build.is_empty(),
        "provider_factory.rs is missing build fns {missing_build:?} (allow-list would be vacuous)"
    );
}

// ===========================================================================
// SC9: Phase 12 non-goals documented as deferred and absent from core
// ===========================================================================

/// The ten Phase 12 non-goals are documented (EN+ZH) and are absent from core:
/// no forbidden crate dependency, no forbidden opi-ai module, and no positive
/// current-core claim in any owned doc surface.
#[test]
fn phase12_non_goals_not_in_core() {
    let opi_ai = read_repo_file("crates/opi-ai/README.md");
    let opi_ai_zh = read_repo_file("crates/opi-ai/README.zh.md");

    // (1) All ten non-goals are documented as deferred in both languages.
    for (en, zh) in PHASE12_NON_GOALS {
        assert!(
            contains_ci(&opi_ai, en),
            "opi-ai README must list the Phase 12 non-goal: {en}"
        );
        assert!(
            opi_ai_zh.contains(zh),
            "opi-ai README.zh must list the Phase 12 non-goal: {zh}"
        );
    }

    // (2) No positive current-core claim of a non-goal anywhere in the surfaces.
    let forbidden_positive = [
        "supports oauth",
        "oauth is supported",
        "oauth login is implemented",
        "subscription auth is implemented",
        "subscription auth is supported",
        "image generation is implemented",
        "image generation is supported",
        "browser usage is supported",
        "browser is supported",
    ];
    for path in doc_surfaces() {
        let content = read_repo_file(path);
        for phrase in forbidden_positive {
            assert!(
                !contains_ci(&content, phrase),
                "{path} must not positively claim `{phrase}` as current core behavior"
            );
        }
    }

    // (3) Structural positive: no forbidden crate dependency anywhere in the
    //     workspace. Scans the root and per-crate Cargo.toml files.
    let mut cargo_files: Vec<PathBuf> = vec![repo_root().join("Cargo.toml")];
    let crates_dir = repo_root().join("crates");
    for entry in std::fs::read_dir(&crates_dir).expect("read crates dir") {
        let path = entry.expect("dir entry").path().join("Cargo.toml");
        if path.is_file() {
            cargo_files.push(path);
        }
    }
    let forbidden_crates = [
        "oauth2",
        "openidconnect",
        "copilot",
        "puppeteer",
        "playwright",
        "chromiumoxide",
        "headless-chrome",
        "fantoccini",
    ];
    let mut scanned_cargo = 0usize;
    for path in &cargo_files {
        let cargo = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        scanned_cargo += 1;
        for forbidden in forbidden_crates {
            assert!(
                !cargo.contains(forbidden),
                "{} must not depend on a forbidden Phase 12 non-goal crate ({forbidden})",
                path.display()
            );
        }
    }
    assert!(
        scanned_cargo >= 4,
        "vacuous-guard: cargo scan must visit at least 4 Cargo.toml files (saw {scanned_cargo})"
    );

    // (4) Structural positive: no oauth/subscription/copilot/browser module.
    let lib_rs = read_repo_file("crates/opi-ai/src/lib.rs");
    for forbidden_mod in ["oauth", "subscription", "copilot", "browser"] {
        assert!(
            !lib_rs.contains(&format!("pub mod {forbidden_mod}"))
                && !lib_rs.contains(&format!("mod {forbidden_mod}")),
            "opi-ai must not declare an `{forbidden_mod}` module (Phase 12 non-goal)"
        );
    }
}

// ===========================================================================
// SC9: default provider tests are network-free and credential-free
// ===========================================================================

/// Default provider tests are fixture/wiremock/MockProvider: every opi-ai
/// provider/lifecycle/fixture test module declares a network-free marker in its
/// module doc-comment, and no test reads a real provider credential outside an
/// `#[ignore]`-gated test.
#[test]
fn default_provider_tests_are_network_free() {
    let family_markers = [
        "anthropic",
        "bedrock",
        "gemini",
        "mistral",
        "openai_chat",
        "openai_responses",
        "openrouter",
        "vertex",
        "azure",
        "lifecycle",
        "fixtures",
    ];
    let doc_markers = [
        "no live",
        "no network",
        "without live",
        "wiremock",
        "MockProvider",
        "fixture",
        "no aws",
    ];

    // (1) Marker-doc convention across opi-ai provider/lifecycle/fixture tests.
    let opi_ai_tests = repo_root().join("crates/opi-ai/tests");
    let mut matched = 0usize;
    for entry in std::fs::read_dir(&opi_ai_tests).expect("read opi-ai/tests") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e != "rs") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !family_markers.iter().any(|m| name.contains(m)) {
            continue;
        }
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let header = content.lines().take(20).collect::<Vec<_>>().join("\n");
        assert!(
            doc_markers.iter().any(|m| contains_ci(&header, m)),
            "{} must declare a network-free marker in its module doc-comment",
            path.display()
        );
        matched += 1;
    }
    assert!(
        matched >= 8,
        "vacuous-guard: at least 8 provider test files must declare network-free markers (saw {matched})"
    );

    // (2) No real-credential env::var READ outside #[ignore]-gated tests, across
    //     opi-ai + opi-coding-agent tests. `.env("X", "...")` calls (setting
    //     subprocess env) and base_url config strings are allowed; the forbidden
    //     pattern is `var("X")` (reading host credentials), which a live test
    //     would require. Endpoint-URL presence is not flagged because a real
    //     domain as a config base_url does not imply a live HTTP call.
    let real_creds = [
        "var(\"ANTHROPIC_API_KEY\")",
        "var(\"OPENAI_API_KEY\")",
        "var(\"GEMINI_API_KEY\")",
        "var(\"AZURE_OPENAI_API_KEY\")",
        "var(\"VERTEX_ACCESS_TOKEN\")",
        "var(\"OPENROUTER_API_KEY\")",
        "var(\"MISTRAL_API_KEY\")",
    ];
    let mut scanned_files = 0usize;
    for dir in [
        repo_root().join("crates/opi-ai/tests"),
        repo_root().join("crates/opi-coding-agent/tests"),
    ] {
        for entry in std::fs::read_dir(&dir).expect("read tests dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e != "rs") {
                continue;
            }
            // This guard file legitimately contains the credential literals as
            // scan targets; skip self-scanning.
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem == "phase12_provider_correctness_docs" {
                continue;
            }
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            scanned_files += 1;
            // Blank out comment lines, then track #[ignore] -> fn boundaries.
            let mut code = String::with_capacity(raw.len());
            for line in raw.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    code.push('\n');
                } else {
                    code.push_str(line);
                    code.push('\n');
                }
            }
            let mut next_fn_ignored = false;
            let mut in_ignored_fn = false;
            let mut brace_depth: i32 = 0;
            for line in code.lines() {
                let t = line.trim();
                if t.starts_with("#[ignore") {
                    next_fn_ignored = true;
                    continue;
                }
                let is_fn_boundary = t.starts_with("fn ")
                    || t.starts_with("async fn ")
                    || t.starts_with("pub fn ")
                    || t.starts_with("pub async fn ")
                    || t.starts_with("pub(crate) fn ")
                    || t.starts_with("pub(crate) async fn ");
                if is_fn_boundary {
                    in_ignored_fn = next_fn_ignored;
                    next_fn_ignored = false;
                    brace_depth = 0;
                }
                if in_ignored_fn {
                    for ch in t.chars() {
                        if ch == '{' {
                            brace_depth += 1;
                        } else if ch == '}' {
                            brace_depth -= 1;
                        }
                    }
                    if brace_depth <= 0 && t.contains('}') {
                        in_ignored_fn = false;
                    }
                    continue;
                }
                for cred in real_creds {
                    assert!(
                        !t.contains(cred),
                        "{} must not read real provider credential via `{cred}` outside an #[ignore] test",
                        path.display()
                    );
                }
            }
        }
    }
    assert!(
        scanned_files >= 20,
        "vacuous-guard: network-free scan must visit at least 20 test files (saw {scanned_files})"
    );
}
