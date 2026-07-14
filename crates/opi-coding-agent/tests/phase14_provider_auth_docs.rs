//! Phase 14 provider/auth documentation and forbidden-scope guards (task 14.7).

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn assert_claims(path: &str, content: &str, claims: &[&str]) {
    let normalized = normalize_whitespace(content);
    for claim in claims {
        assert!(
            normalized.contains(&normalize_whitespace(claim)),
            "{path} must contain the exact Phase 14 claim `{claim}`"
        );
    }
}

fn rust_sources_under(relative: &str) -> String {
    fn visit(path: &Path, output: &mut String) {
        for entry in std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        {
            let path = entry.expect("source directory entry").path();
            if path.is_dir() {
                visit(&path, output);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                output.push_str(
                    &std::fs::read_to_string(&path).unwrap_or_else(|error| {
                        panic!("failed to read {}: {error}", path.display())
                    }),
                );
            }
        }
    }

    let mut output = String::new();
    visit(&repo_root().join(relative), &mut output);
    output
}

#[test]
fn localized_docs_pin_exact_phase14_claims_and_acceptance_rows() {
    let root = read_repo_file("README.md");
    let root_zh = read_repo_file("README.zh.md");
    let ai = read_repo_file("crates/opi-ai/README.md");
    let ai_zh = read_repo_file("crates/opi-ai/README.zh.md");
    let agent = read_repo_file("crates/opi-agent/README.md");
    let agent_zh = read_repo_file("crates/opi-agent/README.zh.md");
    let coding = read_repo_file("crates/opi-coding-agent/README.md");
    let coding_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    assert_claims(
        "README.md",
        &root,
        &[
            "Interactive `/login <provider>` and `/logout <provider>` support Anthropic PKCE, GitHub Copilot device-code, and OpenAI Codex PKCE.",
            "Non-interactive, JSON, and RPC modes instead report the provider plus `/login <provider>` and fail without starting OAuth.",
            "Provider::refresh_models` and collection refresh are substrate-only with no production trigger.",
        ],
    );
    assert_claims(
        "README.zh.md",
        &root_zh,
        &[
            "交互式 `/login <provider>` 与 `/logout <provider>` 支持 Anthropic PKCE、GitHub Copilot device-code 和 OpenAI Codex PKCE。",
            "非交互、JSON 和 RPC 模式只报告 provider 与 `/login <provider>` 修复提示，并在不启动 OAuth 的情况下失败。",
            "`Provider::refresh_models` 和 collection refresh 仅为基底、无生产触发。",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.md",
        &ai,
        &[
            "The three approved live auth paths—Anthropic Messages, Copilot-compatible OpenAI Chat, and Codex-compatible OpenAI Responses—resolve `AuthResolver` inside the returned stream, immediately before HTTP.",
            "Per-call credentials remain out of scope: `extra_headers` rejects provider-managed auth headers.",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.zh.md",
        &ai_zh,
        &[
            "三个获批的真实鉴权路径——Anthropic Messages、Copilot-compatible OpenAI Chat 与 Codex-compatible OpenAI Responses——都在返回的 stream 内、紧邻 HTTP 之前解析 `AuthResolver`。",
            "按调用凭据仍不在范围内：`extra_headers` 会拒绝 Provider 管理的鉴权 header。",
        ],
    );
    assert_claims(
        "crates/opi-agent/README.md",
        &agent,
        &[
            "`opi-agent` does not perform credential IO or construct OAuth providers.",
            "The agent also carries an opaque `session_id` from `Agent` through `AgentLoopContext` into every provider `Request`.",
        ],
    );
    assert_claims(
        "crates/opi-agent/README.zh.md",
        &agent_zh,
        &[
            "`opi-agent` 不执行凭据 IO，也不构造 OAuth provider。",
            "Agent 还把不透明 `session_id` 从 `Agent` 经 `AgentLoopContext` 携带到每个 Provider `Request`。",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.md",
        &coding,
        &[
            "Auth is re-resolved inside the three approved Anthropic, Copilot, and Codex provider streams.",
            "Non-interactive, JSON, and RPC modes do not prompt: they report the provider and `/login anthropic`-style remediation, then fail.",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.zh.md",
        &coding_zh,
        &[
            "只有三个获批的 Anthropic、Copilot 与 Codex Provider stream 会重新解析鉴权。",
            "非交互、JSON 与 RPC 模式不提示：它们报告 provider 和 `/login anthropic` 形式的修复提示后失败。",
        ],
    );
    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "Status: implemented.",
            "held only by the approved Anthropic Messages, Copilot-compatible Chat, and Codex-compatible Responses paths",
            "Dynamic refresh has mock collection coverage but no Phase 14 production trigger and therefore closes no product acceptance path.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "状态：已实现。",
            "只有获批的 Anthropic Messages、Copilot-compatible Chat 与 Codex-compatible Responses 路径持有 `Arc<dyn AuthResolver>`",
            "动态 refresh 只有 mock collection 覆盖，第十四阶段不增加生产触发点，也不以它关闭产品验收路径。",
        ],
    );

    let exact_en_rows = [
        "| SC1 credential storage and probes | 14.1 | `credential_store`, `doctor_cli`, and `list_models` fake-backend integration tests exercise production construction and redacted probe surfaces. |",
        "| SC2 OAuth product flows | 14.2 | `oauth_auth` exercises the production registry, `/login`, `/logout`, PKCE/device-code flows, persistence, exact provider profiles, and `interactive::oauth_login_restores_terminal_after_flow_failure`. |",
        "| SC3 live auth and session interaction | 14.2 | `oauth_auth`, `non_interactive`, and `oauth_auth::rpc_credential_needed_fails_without_blocking` exercise per-stream resolution on the three approved auth paths, typed retry/remediation, revocation, structured RPC failure, and no auto-login paths. |",
        "| SC4 Request and session affinity | 14.3 | `agent_loop_mock::session_id_reaches_every_request`, `session_runtime::phase14_session_affinity_tracks_new_resume_and_fork`, and `request_enrichment::session_affinity_wire_mappings` trace production propagation and exact positive/negative wire mappings. |",
        "| SC5 capabilities and cache markers | 14.4 | `model_capabilities_migration` and Anthropic fixtures prove the nested capability model and capability-gated marker positions/TTL. |",
        "| SC6 usage and cost | 14.5 | `usage_cost`, provider fixtures, and session runtime tests preserve `cache_write_1h_tokens` and `reasoning_tokens` subset semantics without double counting. |",
        "| SC7 dynamic refresh substrate | 14.6 | `provider_collection` and `provider_trait` mock tests prove deterministic atomic replacement; this is substrate-only with no production trigger. |",
        "| SC8 documentation and guards | 14.7 | `phase14_provider_auth_docs`, `oauth_auth::login_logout_commands_are_discoverable`, and `non_interactive::credential_needed_fails_without_prompt` pin localized docs, runtime help, and remediation. |",
    ];
    let exact_zh_rows = [
        "| SC1 凭据存储与 probe | 14.1 | `credential_store`、`doctor_cli` 与 `list_models` fake-backend 集成测试覆盖生产构造和已脱敏 probe 表面。 |",
        "| SC2 OAuth 产品 flow | 14.2 | `oauth_auth` 覆盖生产 registry、`/login`、`/logout`、PKCE/device-code、持久化、精确 Provider profile，以及 `interactive::oauth_login_restores_terminal_after_flow_failure`。 |",
        "| SC3 真实鉴权与会话交互 | 14.2 | `oauth_auth`、`non_interactive` 与 `oauth_auth::rpc_credential_needed_fails_without_blocking` 覆盖三个获批鉴权路径的按 stream 解析、类型化重试/修复、撤销、结构化 RPC 失败与禁止自动登录。 |",
        "| SC4 Request 与会话亲和 | 14.3 | `agent_loop_mock::session_id_reaches_every_request`、`session_runtime::phase14_session_affinity_tracks_new_resume_and_fork` 与 `request_enrichment::session_affinity_wire_mappings` 追踪生产传播和精确的正负 wire 映射。 |",
        "| SC5 能力与 cache marker | 14.4 | `model_capabilities_migration` 与 Anthropic fixture 证明嵌套能力模型及能力门控 marker 位置/TTL。 |",
        "| SC6 用量与费用 | 14.5 | `usage_cost`、Provider fixture 与 session runtime 测试保留 `cache_write_1h_tokens` 和 `reasoning_tokens` 子集语义且不重复计算。 |",
        "| SC7 动态 refresh 基底 | 14.6 | `provider_collection` 与 `provider_trait` mock 测试证明确定性原子替换；它仅为基底、无生产触发。 |",
        "| SC8 文档与 guard | 14.7 | `phase14_provider_auth_docs`、`oauth_auth::login_logout_commands_are_discoverable` 与 `non_interactive::credential_needed_fails_without_prompt` 固定本地化文档、运行时 help 与修复提示。 |",
    ];
    for row in exact_en_rows {
        assert!(
            spec.contains(row),
            "English spec must retain exact row `{row}`"
        );
    }
    for row in exact_zh_rows {
        assert!(
            spec_zh.contains(row),
            "localized spec must retain exact row `{row}`"
        );
    }
}

#[test]
fn every_phase14_non_goal_has_documented_and_structural_evidence() {
    let design =
        read_repo_file("docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md");
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    assert_claims(
        "registered Phase 14 design",
        &design,
        &[
            "No opi-managed plaintext credential file",
            "No auto-relogin mid-stream",
            "No per-call credential (`apiKey` / `env`) or provider-managed auth-header override",
            "No `onPayload` / `onResponse` streaming hooks",
            "No `maxRetries` / `maxRetryDelay` on `Request`",
            "No end-to-end `SecretString`-through-provider-construction refactor",
            "No new OAuth providers beyond the three pi ships",
            "No session-schema or context-reconstruction changes.",
        ],
    );
    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "no opi-managed plaintext credential file",
            "no auto-relogin mid-stream",
            "no per-call credential (`apiKey`/`env`) or provider-managed auth-header override",
            "no `onPayload`/`onResponse` streaming hooks",
            "no `maxRetries`/`maxRetryDelay` on `Request`",
            "no end-to-end `SecretString` provider-construction migration",
            "no OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex",
            "no session-schema or context-reconstruction changes",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "不创建 opi 管理的明文凭据文件",
            "不在 stream 中途自动重新登录",
            "不允许按调用覆盖凭据（`apiKey`/`env`）或 Provider 管理的鉴权 header",
            "不增加 `onPayload`/`onResponse` 流式钩子",
            "不在 `Request` 上增加 `maxRetries`/`maxRetryDelay`",
            "不进行贯穿 Provider 构造的端到端 `SecretString` 迁移",
            "不增加 Anthropic、GitHub Copilot 与 OpenAI Codex 之外的 OAuth Provider",
            "不修改 session schema 或 context reconstruction",
        ],
    );

    let coding_sources = rust_sources_under("crates/opi-coding-agent/src");
    for plaintext_file in ["credentials.json", "credentials.toml", "auth.json"] {
        assert!(
            !coding_sources.contains(plaintext_file),
            "product sources must not introduce plaintext store `{plaintext_file}`"
        );
    }

    let interactive = read_repo_file("crates/opi-coding-agent/src/interactive.rs");
    let rpc = read_repo_file("crates/opi-coding-agent/src/rpc.rs");
    let runner = read_repo_file("crates/opi-coding-agent/src/runner.rs");
    assert_eq!(interactive.matches("oauth::login_oauth(").count(), 2);
    assert_eq!(
        interactive
            .matches("match with_login_terminal_suspended(terminal, ||")
            .count(),
        2,
        "both interactive login paths must suspend raw/alternate-screen mode"
    );
    for (path, source) in [("rpc.rs", &rpc), ("runner.rs", &runner)] {
        assert!(
            !source.contains("login_oauth("),
            "{path} must not auto-start OAuth"
        );
    }
    assert!(rpc.contains("\"type\": \"CredentialNeeded\""));
    assert!(rpc.contains("\"remediation\": format!(\"/login {provider_id}\")"));

    let provider = read_repo_file("crates/opi-ai/src/provider.rs");
    for forbidden in [
        "pub api_key:",
        "pub env:",
        "pub headers:",
        "pub max_retries:",
        "pub max_retry_delay:",
        "pub on_payload:",
        "pub on_response:",
    ] {
        assert!(
            !provider.contains(forbidden),
            "opi_ai::Request must not gain forbidden field `{forbidden}`"
        );
    }
    for required in [
        "pub extra_headers: Vec<(String, String)>",
        "\"authorization\"",
        "\"x-api-key\"",
        "reserved for provider-managed auth",
    ] {
        assert!(
            provider.contains(required),
            "provider guard missing `{required}`"
        );
    }

    for path in [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/src/openai_chat.rs",
        "crates/opi-ai/src/openai_responses.rs",
    ] {
        let source = read_repo_file(path);
        assert!(
            source.contains("pub fn new(api_key: String"),
            "{path} must retain the scoped String constructor boundary"
        );
    }

    let oauth = read_repo_file("crates/opi-coding-agent/src/oauth.rs");
    let registry = oauth
        .split_once("pub fn registry_with_builtins() -> Self")
        .expect("built-in OAuth registry")
        .1
        .split_once("impl Default for OAuthProviderRegistry")
        .expect("end of built-in OAuth registry")
        .0;
    assert_eq!(registry.matches(".register(").count(), 3);
    for provider_type in [
        "AnthropicOAuthProvider::new(",
        "CodexOAuthProvider::new(",
        "CopilotOAuthProvider::new(",
    ] {
        assert_eq!(registry.matches(provider_type).count(), 1);
    }

    let session = read_repo_file("crates/opi-agent/src/session.rs");
    for forbidden in ["OAuth", "Credential", "access_token", "refresh_token"] {
        assert!(
            !session.contains(forbidden),
            "session schema must not acquire auth field `{forbidden}`"
        );
    }
}

#[test]
fn oauth_profiles_and_wire_constants_match_reviewed_pi_values() {
    let oauth = read_repo_file("crates/opi-coding-agent/src/oauth.rs");
    for exact in [
        "https://claude.ai/oauth/authorize",
        "https://platform.claude.com/v1/oauth/token",
        "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
        "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload",
        "https://auth.openai.com/oauth/authorize",
        "https://auth.openai.com/oauth/token",
        "app_EMoamEEZ73f0CkXaXp7hrann",
        "id_token_add_organizations",
        "codex_cli_simplified_flow",
        "https://github.com/login/device/code",
        "https://github.com/login/oauth/access_token",
        "https://api.github.com/copilot_internal/v2/token",
        "Iv1.b507a08c87ecfe98",
        "GitHubCopilotChat/0.35.0",
        "vscode/1.107.0",
        "copilot-chat/0.35.0",
        "vscode-chat",
    ] {
        assert!(oauth.contains(exact), "OAuth source must pin `{exact}`");
    }

    let anthropic = read_repo_file("crates/opi-ai/src/anthropic.rs");
    assert!(anthropic.contains(
        "const ANTHROPIC_OAUTH_BETA_HEADER: &str = \"claude-code-20250219,oauth-2025-04-20\";"
    ));
    let chat = read_repo_file("crates/opi-ai/src/openai_chat.rs");
    assert!(chat.contains("extra_headers.push((\"X-Initiator\".into(), initiator.into()))"));
    let factory = read_repo_file("crates/opi-coding-agent/src/provider_factory.rs");
    assert!(factory.contains(".with_copilot_initiator()"));

    let combined = format!("{oauth}\n{anthropic}\n{chat}\n{factory}").to_lowercase();
    for stale in ["residual-unverified", "must re-confirm", "deferred runtime"] {
        assert!(
            !combined.contains(stale),
            "reviewed auth source must not retain stale qualifier `{stale}`"
        );
    }
}

#[test]
fn changelog_and_refresh_docs_remain_truthful() {
    let changelog = read_repo_file("CHANGELOG.md");
    let unreleased = changelog
        .split("## [0.7.0]")
        .next()
        .expect("Unreleased section");
    for marker in [
        "CredentialStore",
        "OAuthProvider",
        "CredentialNeeded",
        "ModelInfo",
        "cache_write_1h_tokens",
        "reasoning_tokens",
        "refresh_models",
        "/login",
        "/logout",
    ] {
        assert!(
            unreleased.contains(marker),
            "Unreleased must contain `{marker}`"
        );
    }

    for path in [
        "README.md",
        "crates/opi-ai/README.md",
        "crates/opi-coding-agent/README.md",
        "docs/opi-spec.md",
    ] {
        assert_claims(
            path,
            &read_repo_file(path),
            &["substrate-only", "no production trigger"],
        );
    }
    for path in [
        "README.zh.md",
        "crates/opi-ai/README.zh.md",
        "crates/opi-coding-agent/README.zh.md",
        "docs/opi-spec.zh.md",
    ] {
        assert_claims(path, &read_repo_file(path), &["仅为基底", "无生产触发"]);
    }

    let coding_sources = rust_sources_under("crates/opi-coding-agent/src");
    assert!(!coding_sources.contains("refresh_models("));
    assert!(!coding_sources.contains("ProviderCollection::refresh"));
}
