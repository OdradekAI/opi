//! Phase 14 provider/auth documentation and forbidden-scope guards (task 14.21).

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
        .replace("\r\n", "\n")
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

fn assert_absent(path: &str, content: &str, claims: &[&str]) {
    let normalized = normalize_whitespace(content);
    for claim in claims {
        assert!(
            !normalized.contains(&normalize_whitespace(claim)),
            "{path} must not retain the superseded Phase 14 claim `{claim}`"
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
            "GitHub Copilot uses the canonical `github-copilot` identity and one audited static pi-0.80.6 catalog across Anthropic Messages, OpenAI Completions/Chat, and OpenAI Responses routes.",
            "OpenAI Codex uses the canonical `openai-codex` identity, the dedicated `openai-codex-responses` wire, and Browser (default) plus Device Code login.",
            "Persisted credentials use the native OS keychain; the development ids `copilot` and `codex` have no alias or credential migration, so affected users must log in again with the canonical id.",
            "Only Browser PKCE flows await a manual code or callback; GitHub Copilot and OpenAI Codex Device Code call `present_device_code` and never `await_manual_code`.",
            "After a pre-output `CredentialNeeded`, a successful explicit login for the same provider makes the outer TUI retry the same pending turn exactly once without appending a duplicate user message.",
            "Non-interactive text, JSON, and RPC modes emit canonical provider remediation and fail without constructing a `LoginPresenter`, opening a browser, or waiting for input.",
            "Provider::refresh_models` and collection refresh are substrate-only with no production trigger.",
        ],
    );
    assert_claims(
        "README.zh.md",
        &root_zh,
        &[
            "GitHub Copilot 使用规范 `github-copilot` identity，以及一个经审计的静态 pi-0.80.6 catalog；该 catalog 覆盖 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses route。",
            "OpenAI Codex 使用规范 `openai-codex` identity、专用 `openai-codex-responses` wire，以及 Browser（默认）和 Device Code 登录。",
            "持久化凭据使用原生 OS keychain；开发期 id `copilot` 与 `codex` 没有 alias 或凭据迁移，因此受影响用户必须使用规范 id 重新登录。",
            "只有 Browser PKCE flow 会等待手动 code 或 callback；GitHub Copilot 与 OpenAI Codex Device Code 调用 `present_device_code`，绝不调用 `await_manual_code`。",
            "在输出开始前收到 `CredentialNeeded` 后，只有同一 provider 的显式登录成功，outer TUI 才会对同一待处理轮次精确重试一次，且不追加重复 user message。",
            "非交互文本、JSON 与 RPC 模式会输出规范 provider 修复提示并失败，绝不构造 `LoginPresenter`、打开浏览器或等待输入。",
            "`Provider::refresh_models` 和 collection refresh 仅为基底、无生产触发。",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.md",
        &ai,
        &[
            "`WireApi` gives every `ModelInfo` one exact request wire, while public `ApiMappedProvider` exposes one provider identity and catalog and validates its `WireApi -> Provider` routes before dispatch.",
            "One mapped provider shares one lazy `AuthResolver` across all routes; provider/model metadata chooses the route before network IO.",
            "GitHub Copilot routes one static catalog through Anthropic Messages, OpenAI Completions/Chat, and OpenAI Responses; OpenAI Codex uses its dedicated Responses provider rather than standard Responses compatibility flags.",
            "Per-call credentials remain out of scope: `extra_headers` rejects provider-managed auth headers.",
            "Capable built-in Anthropic models emit `cache_control` on the system prompt, final user text, final assistant text, and final tool definition.",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.zh.md",
        &ai_zh,
        &[
            "`WireApi` 为每个 `ModelInfo` 指定一个精确请求 wire；公开 `ApiMappedProvider` 暴露一个 provider identity 与 catalog，并在派发前校验其 `WireApi -> Provider` route。",
            "一个 mapped provider 的所有 route 共享一个惰性 `AuthResolver`；provider/model 元数据会在网络 IO 前选择 route。",
            "GitHub Copilot 把一个静态 catalog 路由到 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses；OpenAI Codex 使用专用 Responses provider，而不是标准 Responses 兼容标志。",
            "按调用凭据仍不在范围内：`extra_headers` 会拒绝 Provider 管理的鉴权 header。",
            "具备能力的 Anthropic 内置模型会在 system prompt、最后一段 user text、最后一段 assistant text 和最后一个 tool definition 上发出 `cache_control`。",
        ],
    );
    assert_claims(
        "crates/opi-agent/README.md",
        &agent,
        &[
            "`opi-agent` does not perform credential IO or construct OAuth providers.",
            "The outer interactive product may retry one pre-output pending turn exactly once after a successful explicit login for the same provider; non-interactive products never prompt, and revoked credentials never trigger automatic re-login.",
            "The agent also carries an opaque `session_id` from `Agent` through `AgentLoopContext` into every provider `Request`.",
        ],
    );
    assert_claims(
        "crates/opi-agent/README.zh.md",
        &agent_zh,
        &[
            "`opi-agent` 不执行凭据 IO，也不构造 OAuth provider。",
            "outer 交互产品只能在同一 provider 的显式登录成功后，对一个输出前的待处理轮次精确重试一次；非交互产品绝不提示，撤销凭据也绝不会触发自动重新登录。",
            "Agent 还把不透明 `session_id` 从 `Agent` 经 `AgentLoopContext` 携带到每个 Provider `Request`。",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.md",
        &coding,
        &[
            "`[providers.custom.<id>]` defines one mapped provider with one shared credential source and auth scheme; provider `api` and `base_url` are defaults, while model values take precedence.",
            "Custom models may use only `anthropic-messages`, `openai-completions`, or `openai-responses`; thinking maps encode identity as `true`, unsupported as `false`, or a wire value as a string.",
            "Compatibility metadata is wire-tagged, pricing tiers apply only when input tokens are strictly greater than `input_tokens_above`, and provider-managed authentication headers are reserved.",
            "Non-interactive, JSON, and RPC modes do not prompt or construct a presenter: they report the canonical provider and `/login <provider>` remediation, then fail.",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.zh.md",
        &coding_zh,
        &[
            "`[providers.custom.<id>]` 定义一个 mapped provider，并让所有 route 共享一个凭据 source 与 auth scheme；provider `api` 和 `base_url` 是默认值，model 值优先。",
            "自定义 model 只能使用 `anthropic-messages`、`openai-completions` 或 `openai-responses`；thinking map 用 `true` 表示 identity、`false` 表示 unsupported，或用 string 表示 wire 值。",
            "兼容元数据按 wire 加 tag；只有 input token 严格大于 `input_tokens_above` 时才应用 pricing tier；Provider 管理的鉴权 header 保持保留。",
            "非交互、JSON 与 RPC 模式既不提示也不构造 presenter：它们报告规范 provider 与 `/login <provider>` 修复提示后失败。",
        ],
    );
    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "Status: implemented; pi-0.80.6 alignment complete. Historical design: `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`. Corrective design: `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.",
            "The offline pi-0.80.6 fixtures `github-copilot.models.json` and `openai-codex.models.json` pin catalog provenance, while `mapped_provider_dispatches_one_catalog_across_three_wires`, `mapped_routes_share_one_lazy_auth_resolver`, `custom_provider_api_and_base_url_precedence`, and `invalid_custom_provider_contracts_fail_at_load` pin mapped-provider behavior.",
            "Dynamic refresh has mock collection coverage but no Phase 14 production trigger and therefore closes no product acceptance path.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "状态：已实现；pi-0.80.6 对齐已完成。历史设计： `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`。修复设计： `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`。",
            "离线 pi-0.80.6 fixture `github-copilot.models.json` 与 `openai-codex.models.json` 固定 catalog provenance；`mapped_provider_dispatches_one_catalog_across_three_wires`、`mapped_routes_share_one_lazy_auth_resolver`、`custom_provider_api_and_base_url_precedence` 与 `invalid_custom_provider_contracts_fail_at_load` 固定 mapped-provider 行为。",
            "动态 refresh 只有 mock collection 覆盖，第十四阶段不增加生产触发点，也不以它关闭产品验收路径。",
        ],
    );

    let exact_en_rows = [
        "| SC1 credential storage and probes | 14.1, 14.8, 14.14 | The cfg-gated host-selection test enters the production native-store selector and proves constructor, default-store, and guard lifecycle; async store/doctor/listing tests retain strict redacted resolver behavior. |",
        "| SC2 OAuth product flows | 14.2, 14.9, 14.18, 14.19 | Concrete dispatcher tests cover Anthropic Browser PKCE, GitHub Copilot Device Code, and OpenAI Codex Browser/Device Code through locked persistence and exact terminal restoration. |",
        "| SC3 live auth and session interaction | 14.2, 14.10, 14.17, 14.18, 14.20 | Factory-built provider tests prove lazy auth and revocation on every approved wire; outer `run_interactive_tui` tests prove one same-provider retry and all negative gates; text/JSON/RPC never construct a presenter. |",
        "| SC4 Request and session affinity | 14.3 | `agent_loop_mock::session_id_reaches_every_request`, `session_runtime::phase14_session_affinity_tracks_new_resume_and_fork`, and `request_enrichment::session_affinity_wire_mappings` trace production propagation and exact positive/negative wire mappings. |",
        "| SC5 capabilities and cache markers | 14.4, 14.11, 14.15 | `ModelInfo` carries exact wire/capability metadata, and `anthropic_cache_markers` captures capability-gated marker positions and TTL through a factory-built concrete Anthropic stream. |",
        "| SC6 usage, metadata, and cost | 14.5, 14.12, 14.15, 14.17, 14.18 | Public contracts, pi catalog fixtures, pricing-tier tests, provider fixtures, cost tests, and session resume preserve strict subsets and deterministic model pricing without double counting. |",
        "| SC7 dynamic refresh and api-map substrate | 14.6, 14.16 | `ApiMappedProvider` and custom TOML tests prove checked multi-wire dispatch with shared lazy auth; collection tests retain deterministic atomic refresh, which has no production trigger. |",
        "| SC8 documentation and guards | 14.7, 14.13, 14.21 | Paired public docs, rustdoc, TUI help, runtime remediation tests, the 58-row acceptance manifest, and workspace gates pin current provider/auth truth and api-map implementation. |",
    ];
    let exact_zh_rows = [
        "| SC1 凭据存储与 probe | 14.1, 14.8, 14.14 | cfg-gated host-selection 测试进入生产原生 store selector，证明 constructor、default-store 与 guard 生命周期；异步 store/doctor/listing 测试保留严格且脱敏的 resolver 行为。 |",
        "| SC2 OAuth 产品 flow | 14.2, 14.9, 14.18, 14.19 | 具体 dispatcher 测试覆盖 Anthropic Browser PKCE、GitHub Copilot Device Code 与 OpenAI Codex Browser/Device Code，并贯穿带锁持久化和精确终端恢复。 |",
        "| SC3 真实鉴权与会话交互 | 14.2, 14.10, 14.17, 14.18, 14.20 | Factory-built Provider 测试证明每条获批 wire 的惰性鉴权与撤销；outer `run_interactive_tui` 测试证明一次同 provider 重试和全部负向 gate；文本/JSON/RPC 绝不构造 presenter。 |",
        "| SC4 Request 与会话亲和 | 14.3 | `agent_loop_mock::session_id_reaches_every_request`、`session_runtime::phase14_session_affinity_tracks_new_resume_and_fork` 与 `request_enrichment::session_affinity_wire_mappings` 追踪生产传播和精确的正负 wire 映射。 |",
        "| SC5 能力与 cache marker | 14.4, 14.11, 14.15 | `ModelInfo` 携带精确 wire/能力元数据，`anthropic_cache_markers` 通过 factory-built 具体 Anthropic stream 捕获能力门控的 marker 位置与 TTL。 |",
        "| SC6 用量、元数据与费用 | 14.5, 14.12, 14.15, 14.17, 14.18 | 公开契约、pi catalog fixture、pricing-tier 测试、Provider fixture、费用测试与 session resume 保留严格子集和确定性 model pricing，且不重复计算。 |",
        "| SC7 动态 refresh 与 api-map 基底 | 14.6, 14.16 | `ApiMappedProvider` 与自定义 TOML 测试证明带共享惰性鉴权的 checked multi-wire 派发；collection 测试保留确定性原子 refresh，且无生产触发。 |",
        "| SC8 文档与 guard | 14.7, 14.13, 14.21 | 成对公共文档、rustdoc、TUI help、运行时修复测试、58-row 验收 manifest 与 workspace gate 固定当前 Provider/Auth 真相和 api-map 实现。 |",
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

    for (path, content) in [
        ("README.md", &root),
        ("README.zh.md", &root_zh),
        ("crates/opi-ai/README.md", &ai),
        ("crates/opi-ai/README.zh.md", &ai_zh),
        ("crates/opi-coding-agent/README.md", &coding),
        ("crates/opi-coding-agent/README.zh.md", &coding_zh),
        ("docs/opi-spec.md", &spec),
        ("docs/opi-spec.zh.md", &spec_zh),
    ] {
        assert_absent(
            path,
            content,
            &[
                "`copilot:`",
                "`codex:`",
                "`/login copilot`",
                "`/login codex`",
                "Copilot OpenAI Chat compatibility profile",
                "Codex Responses compatibility profile",
                "Copilot-compatible OpenAI Chat",
                "Codex-compatible OpenAI Responses",
                "broad Copilot multi-wire parity",
                "separate Codex provider type",
                "`api-map`: `deferred-by-updated-design`",
                "`api-map`：依据",
            ],
        );
    }
}

#[test]
fn localized_specs_pin_final_phase14_runtime_semantics() {
    let spec = read_repo_file("docs/opi-spec.md");
    let spec_zh = read_repo_file("docs/opi-spec.zh.md");
    let ai = read_repo_file("crates/opi-ai/README.md");
    let ai_zh = read_repo_file("crates/opi-ai/README.zh.md");
    let coding = read_repo_file("crates/opi-coding-agent/README.md");
    let coding_zh = read_repo_file("crates/opi-coding-agent/README.zh.md");

    assert_claims(
        "docs/opi-spec.md",
        &spec,
        &[
            "OpenAI Chat and Responses emit reasoning wire fields only when `request.thinking` is enabled and the selected `ModelInfo::thinking_level_map` resolves the requested level; static `reasoning_effort` fields are legacy compatibility/profile metadata and do not override that selection.",
            "With an effective session, built-in direct Responses emits `prompt_cache_key` and a fresh `x-client-request-id` on every request; `send_session_id_header` gates only `session_id`. Custom/proxy profiles default all affinity off, and explicit opt-in enables the reviewed full mapping.",
            "`AuthInvalidPolicy` is explicit on constructed Anthropic, OpenAI Chat, and OpenAI Responses routes, including mapped static profiles, and is never inferred from Bearer syntax.",
            "Within those routes, canonical credential-managed profiles may return `CredentialRevoked`, while static custom, OpenRouter, and Mistral profiles return fixed bodyless `AuthFailed`; this body-suppression claim does not extend to Azure, Bedrock, Gemini, or Vertex diagnostics.",
            "One absolute OAuth flow deadline covers every send, response-body decode, wait/poll, and exchange.",
            "Cancellation is accepted only before one-use code/token acquisition for every flow, producing typed `LoginCancelled`, one fixed cancellation notification, no persistence, and terminal restoration; after code/token acquisition cancellation is ignored while the original deadline remains in force.",
            "Manual input uses one serialized, cancellable cooked-line child process; the manually entered authorization code travels through its inherited stdin and captured stdout, is never injected into argv or a new environment variable, and the child is reaped before retry.",
            "Cumulative `Usage` saturates each public `u32` field at `u32::MAX`; child subsets remain bounded by their parents, and the public shape is not widened.",
            "Doctor and credential-gated model-listing paths use secret-free availability and credential-kind probes that mirror live credential precedence and fail closed on operational errors or corrupt markers.",
            "GitHub Copilot and OpenAI Codex subscription catalogs remain unconditional static catalogs and perform no credential probe during listing.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "OpenAI Chat 与 Responses 仅在 `request.thinking` 启用且所选 `ModelInfo::thinking_level_map` 能解析请求级别时发出 reasoning wire 字段；静态 `reasoning_effort` 字段只是遗留 compatibility/profile metadata，不能覆盖该选择。",
            "存在有效 session 时，内置直连 Responses 在每次请求中发出 `prompt_cache_key` 和新的 `x-client-request-id`；`send_session_id_header` 只门控 `session_id`。自定义/proxy profile 默认关闭全部 affinity；显式 opt-in 会启用经审查的完整映射。",
            "`AuthInvalidPolicy` 由构造完成的 Anthropic、OpenAI Chat 与 OpenAI Responses route（包括 mapped static profile）显式指定，绝不从 Bearer 语法推断。",
            "在这些 route 内，规范 credential-managed profile 可以返回 `CredentialRevoked`，而静态 custom、OpenRouter 与 Mistral profile 返回固定且无 body 的 `AuthFailed`；该 body 抑制声明不扩展到 Azure、Bedrock、Gemini 或 Vertex diagnostics。",
            "一个绝对 OAuth flow deadline 覆盖所有 send、response body decode、wait/poll 与 exchange。",
            "所有 flow 只在获取一次性 code/token 前接受取消，并产生类型化 `LoginCancelled`、一条固定取消通知、不持久化任何凭据且恢复终端；获取 code/token 后忽略取消，但原 deadline 继续生效。",
            "手动输入使用一个串行化、可取消的 cooked-line 子进程；手动输入的 authorization code 经由继承的 stdin 与捕获的 stdout 传递，绝不注入 argv 或新增环境变量，并在 retry 前回收子进程。",
            "累计 `Usage` 的每个公开 `u32` 字段都在 `u32::MAX` 饱和；子集保持不超过父项，公开形状不拓宽。",
            "`doctor` 与凭据门控的模型列表路径使用无 secret availability/credential-kind probe；这些 probe 遵循实时凭据优先级，并在操作错误或 marker 损坏时失败关闭。",
            "GitHub Copilot 与 OpenAI Codex subscription catalog 保持为无条件静态 catalog，列表时不执行凭据 probe。",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.md",
        &ai,
        &[
            "For an effective session, direct OpenAI Responses automatically derives `prompt_cache_key` and a fresh `x-client-request-id`; `send_session_id_header` gates only `session_id`. Custom/proxy affinity remains disabled by default and requires explicit opt-in.",
        ],
    );
    assert_claims(
        "crates/opi-ai/README.zh.md",
        &ai_zh,
        &[
            "存在有效 session 时，直连 OpenAI Responses 会自动派生 `prompt_cache_key` 和新的 `x-client-request-id`；`send_session_id_header` 只门控 `session_id`。自定义/proxy affinity 默认关闭，必须显式 opt-in。",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.md",
        &coding,
        &[
            "`opi doctor` and credential-gated `--list-models` paths await secret-free probes and format only redacted present/absent/backend-unavailable state; the unconditional static GitHub Copilot and OpenAI Codex subscription catalogs perform no credential probe during listing.",
        ],
    );
    assert_claims(
        "crates/opi-coding-agent/README.zh.md",
        &coding_zh,
        &[
            "`opi doctor` 与凭据门控的 `--list-models` 路径等待无 secret probe，并只格式化已脱敏的 present/absent/backend-unavailable 状态；无条件静态 GitHub Copilot 与 OpenAI Codex subscription catalog 在列表时不执行凭据 probe。",
        ],
    );
    assert_absent(
        "final Phase 14 docs",
        &format!("{spec}\n{spec_zh}\n{ai}\n{ai_zh}\n{coding}\n{coding_zh}"),
        &[
            "401/403 response bodies are never surfaced",
            "401/403 response body 绝不对外暴露",
            "map that id only through reviewed compatibility flags",
            "只通过审查过的兼容标志映射该 id",
            "`opi doctor` and `--list-models` await a probe",
            "`opi doctor` 与 `--list-models` 等待 probe",
        ],
    );
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
            "`api-map`: `implemented` by Task 14.16.",
            "The public Rust `ApiMappedProvider` contract and `[providers.custom.<id>]` TOML contract route one provider catalog across checked concrete wires with one shared lazy credential source.",
        ],
    );
    assert_claims(
        "docs/opi-spec.zh.md",
        &spec_zh,
        &[
            "`api-map`：由 Task 14.16 标记为 `implemented`。",
            "公开 Rust `ApiMappedProvider` 契约与 `[providers.custom.<id>]` TOML 契约让一个 provider catalog 通过 checked 具体 wire 路由，并共享一个惰性凭据 source。",
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
