# opi-coding-agent

[![Crates.io](https://img.shields.io/crates/v/opi-coding-agent.svg)](https://crates.io/crates/opi-coding-agent)
[![Docs.rs](https://docs.rs/opi-coding-agent/badge.svg)](https://docs.rs/opi-coding-agent)

> `opi` 二进制与可嵌入编程 harness。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-coding-agent` 产出 `opi` CLI 与可嵌入的 `CodingHarness`：交互式 ratatui TUI、单次
文本与 NDJSON 模式、RPC harness、八个内置工具，以及会话/配置/package 处理。

## 当前状态

当前 crate 版本是 `0.8.0`，继承自 workspace 包版本。

本 crate 把 `opi-ai`、`opi-agent`、`opi-protocol` 和 `opi-tui` 连接成终端编程
Agent。它提供：

- `opi` CLI 二进制；
- 交互式 ratatui TUI 模式；
- 单次文本模式和 `--json` NDJSON 模式；
- `--rpc` JSONL 命令/事件模式；
- 模型、会话、分支和会话树选择器；
- 通过 `--image` 和 `/image` 附加图片；
- 会话 list/resume/fork/delete 命令；
- 8 个内置工具；
- 为 `bash` 提供可插拔的 `command.execute` 路由、Capability Permission 与失败关闭的
  外部 adapter；
- 配置、上下文文件加载、会话持久化、压缩、重试、用量、费用摘要、package/资源发现、
  诊断、可选 trace、OS-keychain 凭据 store，以及交互式 OAuth 登录/登出。

workspace 包版本是 `0.8.0`。当前 checkout 也可能包含未发布变更；delta 见
[CHANGELOG.md](../../CHANGELOG.md)。

opi 加固既有表面，而非新增核心工作流：类型化文件系统工具失败、显式 read/bash 截断、
有界的 gitignore-aware 导航、对 provider adapter 可见的失败工具结果、已脱敏的
provider auth/config 诊断，以及当任一轮 usage 未知或定价未知时会话费用汇总会被省略。

本 crate 也可以通过 `CodingHarness` 作为库使用，但多数用户应先从 CLI 开始。

## 安装

```sh
cargo install opi-coding-agent
opi --version
```

预编译二进制附在 [GitHub Releases](https://github.com/OdradekAI/opi/releases)。

## 快速开始

```sh
export ANTHROPIC_API_KEY=sk-ant-...

# 交互式 TUI
opi

# 单次提示词，助手文本输出到 stdout
opi "找出这个仓库中的 TODO 注释。"

# NDJSON 事件流
opi --json "总结这个 workspace。"

# 指定 provider/model
opi -m openai:gpt-4o "解释 crates/opi-coding-agent/src/main.rs"

# 给第一条提示词附加图片
opi --image screenshot.png "审查这张截图。"

# 在非交互自动化中允许 write/edit/bash
opi --allow-mutating "更新 README。"
```

在交互式 TUI 内管理已存储 OAuth 凭据：

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
```

## CLI 命令与参数

运行 `opi --help` 可查看当前精确表面。重要命令和参数：

| 命令 / 参数 | 作用 |
|-------------|------|
| `[PROMPT]...` | 非空位置参数会进入单次文本模式。 |
| `-m, --model <SPEC>` | 模型 spec，例如 `anthropic:claude-sonnet-4-5-20250514`。 |
| `-c, --config <FILE>` | 显式 TOML 配置文件；必须存在。 |
| `-s, --system <FILE>` | 把用户系统提示词文件追加到内置编程提示词。 |
| `--non-interactive` | 强制单次文本模式；仍然需要提示词文本。 |
| `--json` | 向 stdout 输出 NDJSON session/agent 事件。 |
| `--json-compact` | 紧凑 `--json`：流式 `text_delta` 更新省略冗余累积快照（~线性字节）。 |
| `--rpc` | 通过 stdin/stdout 启动双向 JSONL 命令/事件模式。 |
| `--allow-mutating` | 在交互模式之外允许 `write`、`edit` 和 `bash`。 |
| `--execution-strategy <fixed\|rules\|model>` | 选择 `command.execute` 路由策略。 |
| `--execution-backend <local\|ADAPTER_ID>` | 选择固定执行 backend，或覆盖配置中的 backend。 |
| `--trust` / `--no-trust` | 针对本次会话的一次性项目信任覆盖；二者互斥。 |
| `--tools <TOOLS>` | 逗号分隔的内置工具 allowlist。 |
| `--no-tools` | 禁用所有工具。 |
| `--no-builtin-tools` | 禁用内置工具，同时保留 extension/custom 工具可用性。 |
| `--image <PATH>` | 给初始提示词附加一张图片；可重复。 |
| `--list-models` | 列出已配置 Provider 暴露的模型并退出。 |
| `--list-sessions` | 列出已保存会话并退出。 |
| `--resume <ID>` | 恢复已保存会话。 |
| `--fork <ID>` | fork 已保存会话为新会话。 |
| `--delete-session <ID>` | 删除已保存会话并退出。 |
| `--export-session <ID\|PATH>` | 把会话导出为 markdown 或 JSON 到本地文件。 |
| `--format <md\|json>` | `--export-session` 的输出格式。 |
| `--output <FILE>` | `--export-session` 的输出路径。 |
| `--full-tree` | 导出完整会话树，而非仅活跃分支。 |
| `--exclude-tool-output` | 从导出中省略工具输出。 |
| `--exclude-thinking` | 从导出中省略思考内容。 |
| `--redact <summary\|verbose\|none>` | `--export-session` 的脱敏模式。 |
| `--generate-completion <SHELL>` | 为 `bash`、`zsh`、`fish`、`powershell` 或 `elvish` 生成补全。 |
| `--trace <PATH>` | 为非交互/JSON 运行写入可选的证据（evidence.jsonl + manifest.json）。 |
| `-v, --verbose` | 启用调试追踪。 |
| `doctor [--json] [--scope ...]` | 本地、无网络健康检查。 |
| `package <add|remove|list|doctor|enable|disable>` | 管理本地/git extension package 与可执行 package 的激活状态。 |

## Provider

| 前缀 | 后端 | 默认凭据/配置 |
|------|------|---------------|
| `anthropic:` | `AnthropicProvider` | `ANTHROPIC_API_KEY` |
| `openai:` | `OpenAiChatProvider` | `OPENAI_API_KEY` |
| `openai-responses:` | `OpenAiResponsesProvider` | `OPENAI_API_KEY` |
| `openrouter:` | OpenRouter profile | `OPENROUTER_API_KEY` |
| `mistral:` | Mistral profile | `MISTRAL_API_KEY` |
| `gemini:` | `GeminiProvider` | `GEMINI_API_KEY` |
| `bedrock:` | `BedrockProvider` | AWS 环境变量或共享 AWS profile/config |
| `azure:` | `AzureOpenAIProvider` | `AZURE_OPENAI_API_KEY`；endpoint/deployments 在配置中 |
| `vertex:` | `VertexProvider` | `VERTEX_ACCESS_TOKEN`；project/location 在配置中 |
| `github-copilot:` | 一个经审计的静态 catalog，映射到 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses | 通过 `/login github-copilot` 写入 OS keychain |
| `openai-codex:` | 专用 `OpenAiCodexResponsesProvider` | 通过 `/login openai-codex` 写入 OS keychain |
| 已配置 profile | OpenAI-compatible profile | profile 自己的 `api_key_env`、`base_url` 和模型列表 |

Provider 凭据环境变量名、base URL、模型列表和代理都可以在配置中覆盖。

### 自定义 mapped provider

`[providers.custom.<id>]` 定义一个 mapped provider，并让所有 route 共享一个凭据
source 与 auth scheme；provider `api` 和 `base_url` 是默认值，model 值优先。

```toml
[providers.custom.acme]
name = "Acme"
base_url = "https://api.acme.example"
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
api = "openai-completions"

[[providers.custom.acme.models]]
id = "chat"
display_name = "Acme Chat"
context_window = 128000
max_output_tokens = 16384
thinking_level_map = { low = true, high = "high", max = false }
```

自定义 model 只能使用 `anthropic-messages`、`openai-completions` 或 `openai-responses`；thinking map 用 `true` 表示 identity、`false` 表示 unsupported，或用 string 表示 wire 值。
model `api` 与 `base_url` 覆盖 provider 默认值。model 还可
声明 capabilities、按 wire 加 tag 的 `compat`、base pricing 与严格递增的
`pricing.tiers`。兼容元数据按 wire 加 tag；只有 input token 严格大于
`input_tokens_above` 时才应用 pricing tier；Provider 管理的鉴权 header 保持保留。
subscription-only `openai-codex-responses` wire 不能由自定义 TOML 选择。

## 凭据 Store 与 OAuth

`opi-ai` 拥有无 IO 的 `CredentialStore`、`OAuthProvider`、`LoginPresenter` 和
`AuthResolver` 契约。本 crate 拥有 `KeychainCredentialStore`、
`CredentialResolver`、环境变量查找、跨进程 `credential.lock` 与 Provider HTTP
refresh。持久化 API key 和 OAuth envelope 使用 OS keychain；backend 不可用时 API key
可回退到对应 env source。不会创建 opi 自行管理的明文凭据文件。`opi doctor` 与凭据门控的
`--list-models` 路径等待无 secret probe，并只格式化已脱敏的
present/absent/backend-unavailable 状态；无条件静态 GitHub Copilot 与 OpenAI Codex
subscription catalog 在列表时不执行凭据 probe。

生产启动会在任何凭据感知路径构造 entry 前，先在 Windows 上安装 Windows
Credential Manager、在 macOS 上安装 macOS Keychain Services，或在 Linux 上安装
Freedesktop Secret Service。

`github-copilot` 使用一个经审计的静态 pi-0.80.6 catalog 与一个惰性已存储凭据，
并路由到 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses。模型列表
有意不调用在线 account entitlement/model-enable endpoint。`openai-codex` 在
`/codex/responses` 使用专用 `openai-codex-responses` provider。

`/login openai-codex` 提供 Browser（默认）和 Device Code。Anthropic 与 Codex
Browser 是带 callback/manual-paste fallback 的 PKCE flow。GitHub Copilot 与 Codex
Device Code 调用 `present_device_code`，轮询 provider endpoint，绝不调用
`await_manual_code`。持久化凭据使用原生 OS keychain。开发期 id `copilot` 与 `codex`
没有 alias 或凭据迁移；这些开发期条目的用户必须使用规范 id 重新登录。没有已存储凭据时，
`ANTHROPIC_OAUTH_TOKEN` 是优先于 `ANTHROPIC_API_KEY` 的不可 refresh bearer source。

在输出开始前收到 `CredentialNeeded` 后，只有同一 provider 的显式登录成功，outer TUI
才会对同一待处理轮次精确重试一次，且不追加重复 user message。
非交互、JSON 与 RPC 模式既不提示也不构造 presenter：它们报告规范 provider 与 `/login <provider>` 修复提示后失败。
`CredentialRevoked` 不可重试，绝不会造成自动重新登录。
`AccountIdMissing { provider_id }` 同样不可重试，但与撤销不同：已存储凭据缺少所选 wire
要求的 account identity。若在输出开始前发生，交互模式会保留待处理轮次，等待显式
`/login <provider>` 修复；文本模式以 `AuthFailure` 退出，JSON/RPC 发出带
`AccountIdMissing` 诊断的 `CredentialNeeded` 事件。

活跃 `session_id` 从 `CodingHarness` 经 Agent 主循环带入审查过的 Provider
cache-affinity 映射。其它新 `Request` 标量（`timeout`、`extra_headers`、
`CacheRetention`）仍是直接 `opi-ai` request 基底。`cache_write_1h_tokens` 与
`reasoning_tokens` 在会话费用摘要中保持子集记账。`refresh_models` 仍仅为基底、无生产触发；
CLI、TUI、RPC、doctor、模型列表和启动路径都不会调用它。

## 内置工具

工具位于 `src/tool/`。

| 工具 | 参数 | 说明 |
|------|------|------|
| `read` | `path`，可选 `offset`、`limit` | 1-based 行偏移；并行。 |
| `ls` | `path`，可选 `max_entries`、`max_depth` | 确定性目录列表；遵守 gitignore；并行。 |
| `glob` | `pattern` | 遵守 gitignore 的文件发现；并行。 |
| `find` | `pattern`，可选 `path` | 遵守 gitignore 的文件发现，可限制到子目录；并行。 |
| `grep` | `pattern` | 遵守 gitignore 的正则搜索；并行。 |
| `write` | `path`、`content` | 创建父目录；串行；修改性。 |
| `edit` | `path`、`old_string`、`new_string` | 替换唯一精确匹配，并记录 before/after details；串行；修改性。 |
| `bash` | `command`，可选 `timeout_secs` | 在 workspace 根目录运行；Windows 使用 `cmd /C`，Unix 使用 `sh -c`；串行；修改性。 |

`glob` 是 opi 的便利工具；pi-compatible workflow 不应依赖它作为唯一发现路径。

默认启用工具：

| 模式 | 工具 |
|------|------|
| 交互式 | `read`、`write`、`edit`、`bash` |
| 非交互 / RPC | `read`、`grep`、`find`、`ls`、`glob` |
| 非交互 / RPC 且显式允许修改 | `read`、`write`、`edit`、`bash` |

非交互/RPC 模式下，显式 allowlist 如果包含 `write`、`edit` 或 `bash`，必须同时设置
`--allow-mutating` 或 `defaults.allow_mutating_tools = true`。

## 工具结果契约

每个内置工具都返回同一运行时形状：

| 字段 | 含义 |
|---|---|
| `content` | LLM 可见的文本或图片输出。 |
| `details` | 面向 UI、JSON/RPC、会话和 trace 边界的结构化元数据。 |
| `is_error` | 操作失败或 `bash` 非零退出时设置。 |
| `terminate` | 预留给明确结束运行的工具。 |
| `truncated` | 输出因行数、字节或遍历上限被缩短时设置。 |
| `diagnostics` | 会提升为 opi diagnostics 和 traces 的结构化原因记录。 |

失败 details 在公共边界会被界定和脱敏。Provider 请求接收 LLM 可见内容和失败状态，
不会接收原始命令或路径敏感诊断上下文。

## 工具策略

八个内置工具分为只读和修改性两组。修改性工具仅在解析后的工具选择策略允许时运行。
对于 `bash`，`command.execute` 路由与 Capability Permission 是独立门：启用工具不会
授权外部 adapter，授权 adapter 也不会启用工具。这两个门都不是操作系统级 sandbox。

### 只读与修改性

| 工具 | 类别 |
|------|------|
| `read`、`grep`、`find`、`ls`、`glob` | 只读 |
| `write`、`edit`、`bash` | 修改性 |

`write` 与 `edit` 限制在工作区根目录；非交互式 `read` 同样受限，但交互式
`read` 可读取绝对路径与工作区外的路径。`bash` 不受路径限制。各模式默认启用
哪一组工具，以及非交互/RPC 模式下修改性工具对 `--allow-mutating` 的要求，见
上方[内置工具](#内置工具)。

### 参数优先级

工具参数按确定性优先级解析：

`--no-tools` > `--tools <list>` > `--no-builtin-tools` > 默认

`--no-tools` 禁用全部工具；`--tools` 仅保留指定的内置工具；`--no-builtin-tools`
关闭内置工具但保留 extension/custom 工具可用；否则使用模式默认值。

### bash 执行

| 方面 | 行为 |
|------|------|
| Shell | Windows 使用 `cmd /C`，Unix 使用 `sh -c`。 |
| 工作目录 | 工作区根目录。 |
| 执行 backend | 默认使用内置 `local`；`[execution]` 或 execution CLI 参数可以选择合格的外部 `command.execute` adapter。 |
| 超时 | 默认 30 秒；`timeout_secs` 可覆盖。 |
| 取消 | 取消令牌报告 `cancelled=true` / `timed_out=false`；超时报告 `timed_out=true` / `cancelled=false`。 |
| 有效执行契约 | 本地执行报告 `placement=host`、`guarantee=supervised`；外部 placement、guarantee、policy 与 limitations 来自 backend 的 `started` 事件。仅凭 adapter 身份不能证明安全保证。 |
| 路径限制 | `local` backend 不会把 `bash` 限制在工作区内；外部 backend 只受其报告的有效执行契约约束。 |
| 环境 | 继承自父进程，但绝不写入 details：`details.env = { "inheritance": "inherited", "values_included": false }`。只有当命令本身打印某个值时，该值才会暴露。 |
| 退出码 | 记录在 details 中；非零退出码置 `is_error`。进程在退出前被取消或超时时 `exit_code` 为 null。 |
| 输出 | 合并后的 stdout 与 stderr 上限 64 KiB。见[输出截断](#输出截断)。 |

### 输出截断

| 工具 | 上限 | 截断行为 |
|------|------|----------|
| `read` | 默认 2000 行 | 置 `truncated`，追加 `... N lines omitted` 标记，并记录 `details.truncated` / `omitted` / `line_count`。显式 `limit` 不受默认行数上限约束，但仍受 64 KiB 字节上限约束；`limit: 0` 不返回任何行并置 `truncated`。 |
| `bash` | 合并 stdout+stderr 64 KiB | 当总输出超过上限时，预览为合并后 stdout-then-stderr 的前 64 KiB，置 `truncated` 与 `details.truncated`，并尽力把完整合并输出落盘到临时文件，路径报告在 `details.full_output`。若无法创建该文件，则仅置 `truncated`。 |

### 导航边界

`grep`、`find`、`ls` 和 `glob` 使用同一个 gitignore-aware walker，包含 dotfile，
不跟随 symlink，并按确定性路径排序。`grep`、`find` 和 `glob` 每次最多返回 200 个
inline 结果；四个导航工具都会在访问 10,000 个条目后停止遍历。`grep` 还会跳过大于
1 MiB 的文件，并在累计读取 8 MiB 后停止。跳过文件和提前终止会尽量通过 `details`
和 diagnostics 报告。

### 非目标

以下各项刻意不在内置工具范围内（更广的产品边界见[边界](#边界)）：

- `command.execute` adapter 授权以外的通用权限系统
- 持久后台 bash 或 shell 会话
- 远程执行
- IDE 项目索引
- 语言服务器集成
- `write` / `edit` 时自动格式化
- package 生态扩展
- todo、plan mode 或 sub-agents 等工作流工具
- 内置原生限制；原生限制由 `opi-sandbox` 等独立 adapter 提供

修改性工具的可用性仍由工具选择校验。Capability Permission 与之独立，目前只适用于
`command.execute`。

## 命令执行与项目信任

默认的 Minimal Runtime 通过内置 `local` backend 执行 `bash`。
`[execution] strategy = "fixed"|"rules"|"model"` 配合
`backend = "local"|<adapter-id>`，可以改为选择已安装的外部 `command.execute` adapter。

- `--execution-strategy` 与 `--execution-backend` 只覆盖路由；它们绝不会授予 package
  信任或 Capability Permission。
- Installed、Trusted、Enabled、Selected 与 Permitted 是彼此独立的门。
  `opi package enable <NAME>` 在确认后授予 Package Trust 并启用 package。
  `[execution.permissions]` 归用户所有；项目配置不能授予权限。`local` 默认为
  `allow`，外部 adapter 默认为 `ask`。交互模式下，`ask` 提供单次调用授权或仅驻留
  内存的会话授权；无头模式以 `permission_required` 失败。
- 一旦选定外部 adapter，激活、协议、超时及其他执行失败都会失败关闭，不会回退到
  `local`。运行时结果会报告有效执行契约；仅凭 adapter 身份或 package 元数据不能证明
  限制保证。
- Opi 二进制不链接 `opi-sandbox`。已移除的 `[sandbox]`、`--sandbox` 与
  `--sandbox-require` 表面会被拒绝，且不提供别名。原生限制位于
  [`opi-sandbox`](../opi-sandbox) 等独立 package 中；它们可脱离 Opi 复用。
- 项目信任在启动期解析一次，先于任何项目资源或项目配置消费者（包括 `doctor` 与
  `--list-models`）运行。该 store 是位于 `{user_config_dir}/trust.json`（Windows 为
  `%APPDATA%\opi\trust.json`，Unix 为 `~/.config/opi/trust.json`）的扁平
  `Map<canonical_path, bool>`。当项目不受信任时，其 `.opi/config.toml`、
  `.opi/{skills,fragments,themes,extensions}`、项目级 `.opi/packages.toml` adapter 声明，
  以及项目 `AGENTS.md`/`CLAUDE.md` 都不会加载；这些上下文文件仍可通过 `read` 工具读取。
  信任门控的是资源*加载*，而非工具执行。CLI：`--trust` / `--no-trust`；
  `[defaults] default_project_trust = "ask"|"always"|"never"`（默认 `ask`，仅全局生效）。
  没有内置 `/trust` 命令，没有会话进行中的实时信任变更，也没有项目资源重新加载。

## 运行模式

### 交互式

没有提示词参数时，`opi` 启动 ratatui TUI。Slash 命令包括：

| 命令 | 作用 |
|------|------|
| `/model` | 打开当前 Provider 的模型选择器。 |
| `/session` | 打开会话选择器。 |
| `/session info` | 显示会话名称、标签、活跃分支、模型与思考元数据。 |
| `/name <name>` | 设置当前会话的类型化名称（`session_info` 条目）。 |
| `/label <label>` | 为当前会话添加标签（`label` 条目）。 |
| `/unlabel <label>` | 移除当前会话的某个标签。 |
| `/branch` | 打开分支选择器。 |
| `/tree` | 打开会话树选择器。 |
| `/fork` | 把当前活跃分支 fork 成新的父子会话。 |
| `/clone` | 把当前活跃分支 clone 成新的父子会话。 |
| `/image <path>` | 为下一条提示词排队一张图片。 |
| `/help` | 显示已注册的鉴权命令及其说明。 |
| `/login <provider>` | 运行获批 OAuth flow，并把凭据持久化到 OS keychain。 |
| `/logout <provider>` | 删除该 Provider 的已存储凭据。 |
| `exit` / `quit` | 退出。 |

### 非交互与 JSON

文本模式把助手文本写到 stdout，把诊断写到 stderr。`--json` 会输出 schema header、
序列化 session/agent 事件，以及最终 `session_summary`，格式为 NDJSON。当前 NDJSON
schema version 是 `NDJSON_SCHEMA_VERSION = 2`。在 `session_summary` 中，`turns` 统计
已接受的用户提示词轮数，而 `provider_turns` 统计 provider 请求/响应周期（`TurnStart`
事件），因此使用工具的提示词通常 `provider_turns > turns`。
类型化 `CredentialNeeded` 失败以 code `3` 退出，点名 provider 与
`/login <provider>` 修复提示，绝不启动 OAuth flow 或阻塞等待输入。

`--json-compact` 是一个可选标志，使流式 `text_delta` 更新变为固定大小：它省略冗余的
`assistant_event.partial` 快照，并清空这些更新中 `event.message` 的累积文本，从而让
长时间流式轮次在字节上按 ~线性增长。消费者可从增量重建完整文本，或读取终态
`Done`/`MessageEnd` 快照。默认 `--json` 输出和 `NDJSON_SCHEMA_VERSION = 2` 保持不变。

退出码：

| Code | 含义 |
|------|------|
| `0` | 成功 |
| `1` | 运行时失败 |
| `2` | 配置错误 |
| `3` | 鉴权失败 |
| `4` | Provider 失败 |
| `5` | 工具失败 |
| `130` | 被中断 |

### RPC JSONL

`--rpc` 为 IDE、自定义 UI 和其他嵌入方启动持久双向 JSONL 协议。这是不稳定的 0.x
协议；客户端必须检查 `rpc_ready` header 中的 `schema_version`。当前 SDK/RPC
schema version 是 `3`。启动诊断会通过该 ready header 的 `startup_diagnostics`
字段暴露。

命令包括 `prompt`、`continue`、`steer`、`follow_up`、`abort`、`set_model`、
`set_thinking_level`、`compact`、`session_info`、`extension_command`、`trace` 和
`quit`。

运行时状态拒绝响应可能包含 `error_code`：`unsupported_trace_request`、`agent_busy`、
`harness_unavailable`、`compaction_failed` 和 `extension_command_not_handled`。
`set_model` 和 `set_thinking_level` 的空闲态能力校验失败仍是自由文本错误，不带
`error_code`。

## 配置、会话与上下文文件

配置会合并用户配置、项目配置和显式 `--config` 文件。模型优先级依次为
`--model`、未传入 `--config` 时的 `OPI_MODEL`、显式配置、项目 `.opi/config.toml`、
用户配置和内置默认值。

用户配置路径：

- Windows: `%APPDATA%\opi\config.toml`
- Unix: `~/.config/opi/config.toml`

会话是 append-only JSONL 文件。默认位置是 Windows 的
`%LOCALAPPDATA%\opi\sessions\` 和 Unix 的 `~/.local/share/opi/sessions/`，可用
`OPI_SESSIONS_DIR` 覆盖。

旧版会话中的模型记录可能是不带 provider 前缀的裸模型名。resume、fork 或
branch 时，只有当恰好一条可分发路由提供该模型时才会归一化；路由缺失或
有歧义时保持配置的模型并给出类型化的修复诊断，而不是猜测。加载和归一化
永不改写会话文件；旧的 serialize-only trace 文件与 `--trace` 写出的新
schema 证据并存时保持不透明且逐字节不变，新证据也从不覆盖或升级它们。

`CodingHarness` 会从 workspace 祖先目录向上到 git root 加载 `AGENTS.md` 和
`CLAUDE.md`，然后加载用户配置目录中的同名文件。空文件和超过 128 KiB 的文件会被
忽略。`OPI.md` 有意不加载。

## 资源与 Package

资源发现覆盖来自用户、项目、显式和 package 层的 extensions、packages、skills、
prompt fragments 和 themes。高优先级层覆盖低优先级层；同一层内的重复名称会作为
diagnostics 暴露。

Package 命令：

```sh
opi package add ./vendor/todo
opi package add --local ./vendor/todo
opi package add git:github.com/user/pkg@v1
opi package enable todo
opi package disable todo
opi package list
opi package list --json
opi package doctor
opi package doctor --json
opi package remove todo
```

Package 可以通过 `opi-extension-jsonl-v1` 协议启动 `process-jsonl` extension
adapter，或通过 `command-execution-jsonl-v1` 提供可执行的 `command.execute`
adapter。两者都是不稳定的 0.x 契约。安装可执行 package 不等于信任、启用、选择或
许可它；这些生命周期门彼此独立。Package 是以启动用户的操作系统权限运行的受信任代码，
package 权限声明只是元数据，不是强制执行的 sandbox 策略。

## 作为库使用

`CodingHarness` 是嵌入入口。它可以直接构建，也可以通过 `CodingHarness::builder`
构建，并可配置自定义 hooks、会话恢复数据、工具选择、运行时 package 状态、资源
metadata 和启动诊断。

常用方法包括 `prompt`、`prompt_with_content`、`queue_images`、`subscribe`、
`cancel`、`set_model`、`model_picker_items`、`branch_picker_items`、
`resource_metadata`、`resolve_theme` 和 `session`。

## 边界

- `opi` 不收集 telemetry 或 analytics，也不会自动分享会话。
- `opi doctor` 默认不发起付费模型调用，也不做网络检查。
- 修改性工具策略不是操作系统级 sandbox。
- 核心 `opi` 没有原生限制 backend，也不链接 `opi-sandbox`。
- 生产级子 Agent、plan/todo、MCP，以及 `command.execute` 之外的通用 permission gate
  工作流是 examples/package 模式，不是内置核心工作流。
- Anthropic、GitHub Copilot 与 OpenAI Codex 之外的 OAuth provider 仍被推迟。其它推迟的
  产品决策包括经审计的静态 pi-0.80.6 Copilot/Codex snapshot 之外的 provider catalog、
  大范围新增 first-class provider 列表（兼容
  provider 保持为 config-driven 的 OpenAI-compatible profile）、图像生成（图片支持仅为输入侧）、
  已获批的 Anthropic 与 OpenAI Codex OAuth 登录流程之外的浏览器自动化、面向 package 的 provider 流式 adapter 协议、默认测试中的付费实时
  provider 调用，以及复制 pi 的 provider 专用配置文件格式。按 provider 的代理配置
  （`proxy.url` / `proxy.no_proxy`，环境变量 `HTTPS_PROXY` > `HTTP_PROXY` >
  `NO_PROXY`）和尽力而为的费用（显式未知值优先于虚假置信）已实现。详见 `opi-ai`
  README 的按 family 行为矩阵、OpenAI-compatible profile 标志（`system_role_override`、
  `max_tokens_field`、`tool_result_name_field`、`usage_in_stream`、
  `strict_tool_schema`、`reasoning_effort`、`cache_key`、
  `send_session_affinity_headers`、
  `require_assistant_after_tool_result`、`chat_completions_path`；外加用于静态请求 header 的按 profile
  `extra_headers`，它是 profile 配置字段，不是 `CompatConfig` 标志）、OpenAI Responses
  原生语义（`store` / `strict_tools` 已实现；静态 `reasoning_effort` 仅为遗留兼容性/profile
  元数据；`request.thinking` 与所选 `ModelInfo::thinking_level_map` 控制 Chat/Responses
  wire 输出；`previous_response_id` 推迟），以及缓存 / response-ID / 会话亲和行为。具体来说，
  `usage_in_stream` 会请求 `stream_options.include_usage`，OpenAI Chat 会从任何携带 `id` 的 chunk
  捕获 response ID，`require_assistant_after_tool_result` 在共享适配器中保持为纯元数据，而当任一轮
  usage 未知或定价未知时，会话费用汇总会被省略。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。
