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
//! - **SC9 non-goals** (`phase12_non_goals_not_in_core`) — the six Phase 12
//!   non-goals not superseded by Phase 14 remain absent from core through
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

fn windows_around(haystack: &str, needle: &str, radius: usize) -> Vec<String> {
    let haystack_lower = haystack.to_lowercase();
    let needle_lower = needle.to_lowercase();
    let haystack_chars: Vec<char> = haystack_lower.chars().collect();
    let mut windows = Vec::new();
    let mut start = 0usize;
    while let Some(idx) = haystack_lower[start..].find(&needle_lower) {
        let hit = start + idx;
        let hit_char = haystack_lower[..hit].chars().count();
        let needle_chars = needle_lower.chars().count();
        let from_char = hit_char.saturating_sub(radius);
        let to_char = (hit_char + needle_chars + radius).min(haystack_chars.len());
        windows.push(
            haystack_chars[from_char..to_char]
                .iter()
                .collect::<String>(),
        );
        start = hit + needle_lower.len();
    }
    windows
}

fn mentions_runtime_enforcement_for_assistant_after_tool_result(doc: &str) -> bool {
    let compat_windows = windows_around(doc, "require_assistant_after_tool_result", 320);
    let shared_adapter_windows = windows_around(doc, "shared adapter", 320);
    let shared_adapter_windows_zh = windows_around(doc, "共享适配器", 320);

    let windows = compat_windows
        .into_iter()
        .chain(shared_adapter_windows)
        .chain(shared_adapter_windows_zh);

    windows.into_iter().any(|window| {
        let mentions_runtime = window.contains("runtime")
            || window.contains("运行时")
            || window.contains("runtime check")
            || window.contains("运行时校验");
        let mentions_enforcement = window.contains("enforce")
            || window.contains("enforced")
            || window.contains("强制")
            || window.contains("校验");
        let mentions_extra_turn = window.contains("assistant turn")
            || window.contains("assistant-after-tool-result")
            || window.contains("after tool result")
            || window.contains("extra assistant turn")
            || window.contains("额外的 assistant 轮次")
            || window.contains("assistant 轮次");
        mentions_runtime && mentions_enforcement && mentions_extra_turn
    })
}

fn count_pub_fields_in_struct(source: &str, struct_name: &str) -> usize {
    let start = source
        .find(&format!("pub struct {struct_name}"))
        .unwrap_or_else(|| panic!("missing struct `{struct_name}` source anchor"));
    let body = &source[start..];
    let open = body
        .find('{')
        .unwrap_or_else(|| panic!("missing `{{` for `{struct_name}`"));
    let rest = &body[open + 1..];
    let close = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("missing closing brace for `{struct_name}`"));
    rest[..close]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub "))
        .count()
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
    "endpoint",
    "http",
    "message",
    "model",
    "model_info",
    "openai_codex_responses",
    "openai_responses_shared",
    "provider",
    "provider_collection",
    "registry",
    "retry",
    "stream",
    "test_support",
    "time",
    // Phase 14 infrastructure modules (not first-class providers): per-request
    // auth resolution, credential envelopes, and API-mapped routing/header policy.
    "api_mapped",
    "auth",
    "credential",
    "provider_headers",
];

/// Phase 12 non-goals not superseded by Phase 14, with an English and a
/// Simplified-Chinese token each.
const PHASE12_NON_GOALS: &[(&str, &str)] = &[
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
    let coding_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");
    let root = read_repo_file("README.md");
    let root_zh = read_repo_file("README.zh.md");
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

    // (4) Profile flags: the ten Phase 12 CompatConfig field names verbatim.
    //     Task 14.15 moved their definition to OpenAiCompletionsCompat and
    //     retained CompatConfig as a public alias. `extra_headers`
    //     is intentionally excluded here — it is a per-profile config field /
    //     OpenAiChatProvider constructor concern, not a CompatConfig flag; the
    //     session-affinity check below pins its documentation separately.
    let compat_flags = [
        "system_role_override",
        "max_tokens_field",
        "tool_result_name_field",
        "usage_in_stream",
        "strict_tool_schema",
        "reasoning_effort",
        "cache_key",
        "send_session_affinity_headers",
        "require_assistant_after_tool_result",
        "chat_completions_path",
    ];
    let model_info_src = read_repo_file("crates/opi-ai/src/model_info.rs");
    let openai_chat_src = read_repo_file("crates/opi-ai/src/openai_chat.rs");
    assert_eq!(
        count_pub_fields_in_struct(&model_info_src, "OpenAiCompletionsCompat"),
        compat_flags.len() + 3,
        "OpenAiCompletionsCompat field count changed; update the Phase 12 docs guard and owned docs together"
    );
    for flag in compat_flags {
        assert!(
            contains_ci(&opi_ai, flag),
            "opi-ai README must document profile flag `{flag}`"
        );
        assert!(
            contains_ci(&coding, flag),
            "opi-coding-agent README must document profile flag `{flag}`"
        );
    }
    for (label, surface) in [
        ("README.md", &opi_ai),
        ("README.zh.md", &opi_ai_zh),
        ("coding README.md", &coding),
        ("coding README.zh.md", &coding_zh),
        ("root README.md", &root),
        ("root README.zh.md", &root_zh),
        ("opi-spec.md", &spec),
        ("opi-spec.zh.md", &spec_zh),
    ] {
        assert!(
            !mentions_runtime_enforcement_for_assistant_after_tool_result(surface),
            "{label} must not claim runtime enforcement for `require_assistant_after_tool_result`; it is metadata-only in the shared adapter"
        );
    }
    // ModelCompatOverride (model > provider precedence).
    assert!(
        contains_ci(&opi_ai, "ModelCompatOverride"),
        "opi-ai README must document ModelCompatOverride (model-over-provider precedence)"
    );

    // (4b) `usage_in_stream` docs stay aligned with the shared adapter.
    assert!(
        openai_chat_src.contains("stream_options") && openai_chat_src.contains("include_usage"),
        "openai_chat.rs must source-anchor `usage_in_stream` to stream_options.include_usage"
    );
    for surface in [&opi_ai, &coding, &spec, &root] {
        assert!(
            contains_ci(surface, "include_usage"),
            "owned English docs must name `stream_options.include_usage` for usage_in_stream"
        );
    }
    for surface in [&opi_ai_zh, &coding_zh, &spec_zh, &root_zh] {
        assert!(
            contains_ci(surface, "include_usage"),
            "owned Chinese docs must name `stream_options.include_usage` for usage_in_stream"
        );
    }
    assert!(
        contains_ci(&opi_ai, "any streaming chunk") && contains_ci(&opi_ai_zh, "任意流式 chunk"),
        "opi-ai READMEs must say usage updates are preserved from any streaming chunk"
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
        contains_ci(&opi_ai, "OpenAI Chat captures the ID from any")
            && contains_ci(&opi_ai, "chunk carrying `id`")
            && contains_ci(&opi_ai_zh, "OpenAI Chat 会从任何携带 `id` 的 chunk")
            && contains_ci(&opi_ai_zh, "捕获 response ID")
            && contains_ci(&coding, "OpenAI Chat captures response IDs")
            && contains_ci(&coding, "chunk carrying `id`")
            && contains_ci(&coding_zh, "OpenAI Chat 会从任何携带 `id` 的 chunk")
            && contains_ci(&coding_zh, "捕获 response ID")
            && contains_ci(&spec, "OpenAI Chat captures the ID from any")
            && contains_ci(&spec, "chunk carrying `id`")
            && contains_ci(&spec_zh, "OpenAI Chat 会从任何携带 `id` 的 chunk")
            && contains_ci(&spec_zh, "捕获 response ID")
            && contains_ci(&root, "OpenAI Chat chunk carrying `id`")
            && contains_ci(&root, "response IDs captured")
            && contains_ci(&root_zh, "OpenAI Chat chunk")
            && contains_ci(&root_zh, "携带 `id`")
            && contains_ci(&root_zh, "捕获 response ID"),
        "owned docs must say OpenAI Chat captures response IDs from any chunk carrying `id`"
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
    assert!(
        contains_ci(&opi_ai, "unknown usage")
            && contains_ci(&opi_ai, "cost summaries should therefore be omitted")
            && contains_ci(&opi_ai_zh, "usage 未知")
            && contains_ci(&opi_ai_zh, "费用汇总就应省略")
            && contains_ci(&coding, "session cost summaries")
            && contains_ci(&coding, "omitted when any turn")
            && contains_ci(&coding_zh, "会话费用汇总会被省略")
            && contains_ci(&spec, "session cost summaries")
            && contains_ci(&spec, "omitted")
            && contains_ci(&spec_zh, "会话费用汇总会被省略"),
        "docs must say missing usage stays explicitly unknown and cost summaries are omitted when usage or pricing is unknown"
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

/// Phase 12 non-goals not superseded by Phase 14 are documented (EN+ZH) and
/// absent from core: no forbidden crate dependency, no forbidden opi-ai module,
/// and no positive current-core claim in any owned doc surface.
#[test]
fn phase12_non_goals_not_in_core() {
    let opi_ai = read_repo_file("crates/opi-ai/README.md");
    let opi_ai_zh = read_repo_file("crates/opi-ai/README.zh.md");

    // (1) All remaining non-goals are documented in both languages.
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

    // (4) Structural positive: no browser module.
    let lib_rs = read_repo_file("crates/opi-ai/src/lib.rs");
    assert!(
        !lib_rs.contains("pub mod browser") && !lib_rs.contains("mod browser"),
        "opi-ai must not declare a `browser` module (Phase 12 non-goal)"
    );
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
        if path.is_dir() || path.extension().is_some_and(|e| e != "rs") {
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
    // Tokens are qualified with `::` so write helpers like `remove_var("X")` /
    // `set_var("X")` (which contain the `var("X")` substring) are not mistaken
    // for credential reads. `std::env::var("X")` / `env::var("X")` reads are
    // still caught; `var_os` reads are out of scope (unchanged).
    let real_creds = [
        "::var(\"ANTHROPIC_API_KEY\")",
        "::var(\"OPENAI_API_KEY\")",
        "::var(\"GEMINI_API_KEY\")",
        "::var(\"AZURE_OPENAI_API_KEY\")",
        "::var(\"VERTEX_ACCESS_TOKEN\")",
        "::var(\"OPENROUTER_API_KEY\")",
        "::var(\"MISTRAL_API_KEY\")",
    ];
    let mut scanned_files = 0usize;
    for dir in [
        repo_root().join("crates/opi-ai/tests"),
        repo_root().join("crates/opi-coding-agent/tests"),
    ] {
        for entry in std::fs::read_dir(&dir).expect("read tests dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() || path.extension().is_some_and(|e| e != "rs") {
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
