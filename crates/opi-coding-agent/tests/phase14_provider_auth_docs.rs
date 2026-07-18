//! Phase 14 provider/auth documentation and forbidden-scope guards (task 14.13).

use std::path::{Path, PathBuf};

use opi_coding_agent::interactive_auth::AUTH_HELP;
use opi_coding_agent::runner::ExitCode;

#[path = "common/phase14_auth_runtime.rs"]
mod phase14_auth_runtime;
use phase14_auth_runtime::{
    credential_runner, run_json_credential_capture, run_rpc_stdio_capture,
    run_text_credential_capture,
};

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
            "Only a successful, user-initiated `/login <provider>` retries a pending interactive turn; `CredentialNeeded` never starts login automatically.",
            "Non-interactive, JSON, and RPC modes instead report the provider plus `/login <provider>` and fail without starting OAuth.",
            "Provider::refresh_models` and collection refresh are substrate-only with no production trigger.",
        ],
    );
    assert_claims(
        "README.zh.md",
        &root_zh,
        &[
            "交互式 `/login <provider>` 与 `/logout <provider>` 支持 Anthropic PKCE、GitHub Copilot device-code 和 OpenAI Codex PKCE。",
            "只有用户显式执行且成功的 `/login <provider>` 才会重试待处理的交互轮次；`CredentialNeeded` 绝不自动启动登录。",
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
            "Capable built-in Anthropic models emit `cache_control` on the system prompt, final user text, final assistant text, and final tool definition.",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.zh.md",
        &ai_zh,
        &[
            "三个获批的真实鉴权路径——Anthropic Messages、Copilot-compatible OpenAI Chat 与 Codex-compatible OpenAI Responses——都在返回的 stream 内、紧邻 HTTP 之前解析 `AuthResolver`。",
            "按调用凭据仍不在范围内：`extra_headers` 会拒绝 Provider 管理的鉴权 header。",
            "具备能力的 Anthropic 内置模型会在 system prompt、最后一段 user text、最后一段 assistant text 和最后一个 tool definition 上发出 `cache_control`。",
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
            "Status: implemented; remediation complete. Historical design: `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`. Corrective design: `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.",
            "held only by the approved Anthropic Messages, Copilot-compatible Chat, and Codex-compatible Responses paths",
            "Dynamic refresh has mock collection coverage but no Phase 14 production trigger and therefore closes no product acceptance path.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "状态：已实现；修复已完成。历史设计： `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`。修复设计： `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`。",
            "只有获批的 Anthropic Messages、Copilot-compatible Chat 与 Codex-compatible Responses 路径持有 `Arc<dyn AuthResolver>`",
            "动态 refresh 只有 mock collection 覆盖，第十四阶段不增加生产触发点，也不以它关闭产品验收路径。",
        ],
    );

    let exact_en_rows = [
        "| SC1 credential storage and probes | 14.1, 14.8 | Native-store selection plus async `credential_store`, `doctor_cli`, and `list_models` fake-backend tests exercise production startup, strict resolver errors, stored-only listing, and redacted probes. |",
        "| SC2 OAuth product flows | 14.2, 14.9 | `interactive_auth` drives the production `/login` and `/logout` dispatcher, locked persistence, terminal suspension/restoration, and all three reviewed OAuth profiles. |",
        "| SC3 live auth and session interaction | 14.2, 14.10 | Factory-built provider, `interactive_auth`, `json_mode`, RPC, and text tests cover lazy per-stream auth, changed credentials, bounded refresh, explicit same-turn retry, revocation, provider-id remediation, and no automatic login. |",
        "| SC4 Request and session affinity | 14.3 | `agent_loop_mock::session_id_reaches_every_request`, `session_runtime::phase14_session_affinity_tracks_new_resume_and_fork`, and `request_enrichment::session_affinity_wire_mappings` trace production propagation and exact positive/negative wire mappings. |",
        "| SC5 capabilities and cache markers | 14.4, 14.11 | `anthropic_cache_markers` captures capability-gated marker positions and TTL through a factory-built concrete Anthropic stream. |",
        "| SC6 usage and cost | 14.5, 14.12 | Public-contract, provider-fixture, cost, and session-resume tests preserve optional `u64` child subsets, reject malformed usage, and prevent double counting. |",
        "| SC7 dynamic refresh substrate | 14.6 | `provider_collection` and `provider_trait` mock tests prove deterministic atomic replacement; this is substrate-only with no production trigger. |",
        "| SC8 documentation and guards | 14.7, 14.13 | `phase14_provider_auth_docs`, production-dispatcher TUI help, `json_mode`, RPC, and text tests pin localized truth, runtime discovery, typed remediation, and the renewed `api-map` disposition. |",
    ];
    let exact_zh_rows = [
        "| SC1 凭据存储与 probe | 14.1, 14.8 | 原生 store 选择以及异步 `credential_store`、`doctor_cli` 与 `list_models` fake-backend 测试覆盖生产启动、严格 resolver 错误、仅已存储凭据的模型列表和脱敏 probe。 |",
        "| SC2 OAuth 产品 flow | 14.2, 14.9 | `interactive_auth` 驱动生产 `/login` 与 `/logout` dispatcher、带锁持久化、终端暂停/恢复以及三个经审查 OAuth profile。 |",
        "| SC3 真实鉴权与会话交互 | 14.2, 14.10 | Factory-built Provider、`interactive_auth`、`json_mode`、RPC 与文本测试覆盖惰性按 stream 鉴权、凭据变更、有界 refresh、显式同轮重试、撤销、provider-id 修复提示和禁止自动登录。 |",
        "| SC4 Request 与会话亲和 | 14.3 | `agent_loop_mock::session_id_reaches_every_request`、`session_runtime::phase14_session_affinity_tracks_new_resume_and_fork` 与 `request_enrichment::session_affinity_wire_mappings` 追踪生产传播和精确的正负 wire 映射。 |",
        "| SC5 能力与 cache marker | 14.4, 14.11 | `anthropic_cache_markers` 通过 factory-built 具体 Anthropic stream 捕获能力门控的 marker 位置与 TTL。 |",
        "| SC6 用量与费用 | 14.5, 14.12 | 公开契约、Provider fixture、费用和 session-resume 测试保留可选 `u64` 子集、拒绝非法用量并防止重复计算。 |",
        "| SC7 动态 refresh 基底 | 14.6 | `provider_collection` 与 `provider_trait` mock 测试证明确定性原子替换；它仅为基底、无生产触发。 |",
        "| SC8 文档与 guard | 14.7, 14.13 | `phase14_provider_auth_docs`、生产 dispatcher TUI help、`json_mode`、RPC 与文本测试固定本地化真相、运行时发现、类型化修复提示以及更新的 `api-map` disposition。 |",
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
fn final_phase14_contracts_native_targets_and_api_map_are_truthful() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");

    for content in [&spec, &spec_zh] {
        for exact in [
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "windows-native-keyring-store",
            "apple-native-keyring-store",
            "zbus-secret-service-keyring-store",
            "Windows Credential Manager",
            "macOS Keychain Services",
            "Freedesktop Secret Service",
        ] {
            assert!(content.contains(exact), "spec must name `{exact}`");
        }
        assert!(!content.contains("~/.local/share/opi/auth/"));
        assert_claims(
            "Phase 14 public signatures",
            content,
            &[
                r#"pub trait Provider: Send + Sync {
                    fn id(&self) -> &str;
                    fn models(&self) -> &[ModelInfo];
                    fn stream(&self, request: Request) -> EventStream;
                    fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>, ProviderError>>;
                }"#,
                r#"pub struct Request {
                    pub model: String,
                    pub system: Option<String>,
                    pub messages: Vec<Message>,
                    pub tools: Vec<ToolDef>,
                    pub max_tokens: Option<u64>,
                    pub temperature: Option<f64>,
                    pub thinking: ThinkingConfig,
                    pub stop_sequences: Vec<String>,
                    pub metadata: Option<serde_json::Value>,
                    pub cancel: CancellationToken,
                    pub timeout: Option<std::time::Duration>,
                    pub extra_headers: Vec<(String, String)>,
                    pub cache_retention: CacheRetention,
                    pub session_id: Option<String>,
                }"#,
                r#"pub struct Usage {
                    pub input_tokens: u32,
                    pub output_tokens: u32,
                    pub cache_read_tokens: u32,
                    pub cache_write_tokens: u32,
                    pub cache_write_1h_tokens: Option<u64>,
                    pub reasoning_tokens: Option<u64>,
                    pub reported: bool,
                }"#,
                r#"pub struct CostBreakdown {
                    pub input_cost: f64,
                    pub output_cost: f64,
                    pub cache_read_cost: f64,
                    pub cache_write_cost: f64,
                }"#,
            ],
        );
    }

    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "`api-map`: `deferred-by-updated-design` under `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.",
            "New trigger: one catalog/provider identity must require at least two concrete wire families and explicit provider profiles must be inadequate. A separate reviewed design must then define model-to-wire selection, per-stream auth, capability routing, and the `ProviderCollection` boundary.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "`api-map`：依据 `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md` 记为 `deferred-by-updated-design`。",
            "新触发条件：一个 catalog/provider identity 必须要求至少两个具体 wire family，且显式 provider profile 必须不足以表达；届时必须由单独审查的设计定义 model-to-wire selection、per-stream auth、capability routing 和 `ProviderCollection` boundary。",
        ],
    );
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
    let interactive_auth = read_repo_file("crates/opi-coding-agent/src/interactive_auth.rs");
    let rpc = read_repo_file("crates/opi-coding-agent/src/rpc.rs");
    let runner = read_repo_file("crates/opi-coding-agent/src/runner.rs");
    assert!(interactive.contains("dispatch_auth_command("));
    assert!(interactive.contains("AuthCommandServices"));
    for forbidden in [
        "with_login_terminal_suspended",
        "oauth::login_oauth(",
        "oauth::logout_credential(",
    ] {
        assert!(
            !interactive.contains(forbidden),
            "interactive.rs must not contain inline auth branch `{forbidden}`"
        );
    }
    assert!(interactive_auth.contains("pub async fn dispatch_auth_command"));
    assert!(interactive_auth.contains("oauth::login_oauth("));
    assert!(interactive_auth.contains("oauth::logout_credential("));
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
    let registry_with_services = oauth
        .split_once("pub(crate) fn registry_with_services(")
        .expect("service-backed OAuth registry")
        .1
        .split_once("/// Register the three production OAuth providers")
        .expect("end of service-backed OAuth registry")
        .0;
    assert_eq!(registry_with_services.matches(".register(").count(), 3);
    for provider_type in [
        "AnthropicOAuthProvider::with_services(",
        "CodexOAuthProvider::with_services(",
        "CopilotOAuthProvider::with_services(",
    ] {
        assert_eq!(registry_with_services.matches(provider_type).count(), 1);
    }

    let registry_with_builtins = oauth
        .split_once("pub fn registry_with_builtins() -> Self")
        .expect("built-in OAuth registry")
        .1
        .split_once("impl Default for OAuthProviderRegistry")
        .expect("end of built-in OAuth registry")
        .0;
    assert_eq!(registry_with_builtins.matches(".register(").count(), 0);
    assert_claims(
        "production OAuth registry",
        registry_with_builtins,
        &[r#"Self::registry_with_services(
            &OAuthEndpointConfig::production(),
            production_oauth_client(),
        )"#],
    );
    assert!(oauth.contains("#[cfg(debug_assertions)]\n    pub(crate) fn with_test_base_url("));
    assert!(
        interactive_auth.contains(
            "#[cfg(debug_assertions)]\n    #[doc(hidden)]\n    pub fn with_test_services("
        )
    );

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn localized_truth_target_executes_declared_runtime_api_map() {
    assert!(
        AUTH_HELP.iter().any(|(command, description)| {
            *command == "/login <provider>"
                && *description == "authenticate and persist an OAuth credential"
        }),
        "localized truth target binds the production auth help table"
    );
    assert!(
        AUTH_HELP.iter().any(|(command, description)| {
            *command == "/logout <provider>" && *description == "delete the persisted credential"
        }),
        "localized truth target binds the production logout help table"
    );

    let workspace = tempfile::tempdir().unwrap();
    let json = run_json_credential_capture(credential_runner(workspace.path())).await;
    assert_eq!(json["exit_code"], ExitCode::AuthFailure as i32);
    assert!(
        json["stdout"]
            .as_str()
            .unwrap()
            .contains("\"type\":\"CredentialNeeded\"")
    );
    assert!(
        json["stdout"]
            .as_str()
            .unwrap()
            .contains("/login anthropic")
    );

    let text = run_text_credential_capture(credential_runner(workspace.path())).await;
    assert_eq!(text["exit_code"], ExitCode::AuthFailure as i32);
    assert!(text["stderr"].as_str().unwrap().contains("anthropic"));
    assert!(
        text["stderr"]
            .as_str()
            .unwrap()
            .contains("/login anthropic")
    );

    let rpc = run_rpc_stdio_capture("phase14_docs_rpc_run_stdio_child");
    let remediation = rpc
        .iter()
        .find(|line| line["type"] == "CredentialNeeded")
        .expect("RpcRunner::run emits typed credential remediation");
    assert_eq!(remediation["provider_id"], "anthropic");
    assert_eq!(remediation["remediation"], "/login anthropic");
}

#[tokio::test]
#[ignore = "subprocess-only RPC stdio entry point"]
async fn phase14_docs_rpc_run_stdio_child() {
    phase14_auth_runtime::run_rpc_stdio_child().await;
}
