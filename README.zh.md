# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> 受 [earendil-works/pi](https://github.com/earendil-works/pi) 启发的 Rust AI
> Agent 工具包与终端优先的编程 Agent。

opi 是一个可嵌入的多 Provider 编程 Agent 运行时，可通过交互式 TUI、单次 CLI、NDJSON
事件流或 RPC harness 驱动。

[English](README.md) | [更新日志](CHANGELOG.md)

## 当前状态

`Cargo.toml` 中的 workspace 包版本是 `0.7.0`。当前 checkout 可能包含基于已发布
`0.7.0` crate 的未发布变更；发布相关判断请先查看
[CHANGELOG.md](CHANGELOG.md)。

`opi` 现在既可作为终端编程 Agent 使用，也可作为一组 Rust crate 嵌入其他 Agent
运行时。它用 Rust 重新实现 pi 的部分思路，但不与 pi API 兼容，默认不读取 pi
配置，并使用自己的 TOML 配置和 append-only JSONL 会话格式。

wire protocol、会话/导出格式、package、extension、SDK/RPC 和 trace envelope 仍是
不稳定 0.x 表面。嵌入方需要固定精确 crate 版本。

## 安装

CLI 二进制名为 `opi`，由 `opi-coding-agent` crate 产出。

```sh
cargo install opi-coding-agent
opi --version
```

Linux、macOS 和 Windows 的 x64/arm64 预编译二进制附在
[GitHub Releases](https://github.com/OdradekAI/opi/releases)。

## 快速开始

先设置一个 Provider 的凭据：

```sh
export ANTHROPIC_API_KEY=sk-ant-...
# 或 OPENAI_API_KEY、OPENROUTER_API_KEY、MISTRAL_API_KEY、GEMINI_API_KEY
# 或 AWS 凭据、AZURE_OPENAI_API_KEY、VERTEX_ACCESS_TOKEN
```

启动交互式 TUI：

```sh
opi
```

在 TUI 内显式管理已存储的 OAuth 凭据：

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
```

运行单次提示词：

```sh
opi "列出这个 workspace 中的 Rust crate。"
```

输出 NDJSON 事件：

```sh
opi --json "总结这个仓库。"
```

最终的 `session_summary` 行报告 `turns`（已接受的用户提示词轮数）和 `provider_turns`（provider 请求/响应周期，即 `TurnStart` 事件）。

给第一条提示词附加图片：

```sh
opi --image screenshot.png "审查这个 UI。"
```

使用 `provider:model` 语法选择模型：

```sh
opi -m anthropic:claude-sonnet-4-5-20250514 "解释 crates/opi-agent/src/lib.rs"
opi -m openai:gpt-4o "审查公共 API 形状。"
```

在本地导出已保存会话：

```sh
opi --export-session <ID_OR_PATH> --output session.md
opi --export-session <ID_OR_PATH> --output session.json --format json
```

## Workspace Crates

所有 crate 共享 workspace 的版本、edition、license、repository 和 authors。

| Crate | 作用 |
| --- | --- |
| [`opi-ai`](crates/opi-ai) | Provider 无关 LLM API、流式事件、模型注册表、重试、HTTP/代理、用量和尽力而为的费用辅助。 |
| [`opi-agent`](crates/opi-agent) | Agent 主循环、工具契约、hooks、事件、队列、会话、压缩、SDK/RPC 类型、扩展、诊断和 streaming proxy。 |
| [`opi-tui`](crates/opi-tui) | Ratatui 组件、对话渲染、diff 视图、选择器、终端图片、主题和按键绑定。 |
| [`opi-coding-agent`](crates/opi-coding-agent) | `opi` 二进制、内置编程工具、配置/会话/package 处理和可嵌入 `CodingHarness`。 |

内部依赖形状：

```text
opi-ai
opi-tui
opi-agent -> opi-ai
opi-coding-agent -> opi-ai + opi-agent + opi-tui -> opi binary
```

## 主要 CLI 表面

```sh
opi --help
opi --list-models
opi --list-models --json
opi --generate-completion powershell
opi doctor
opi package list
```

常用模式和会话参数：

| 参数 | 作用 |
| --- | --- |
| `--non-interactive` | 强制单次文本模式。 |
| `--json` | 单次 NDJSON 事件流。 |
| `--json-compact` | 紧凑 `--json`：流式 `text_delta` 更新省略冗余累积快照（长轮次 ~线性字节）。 |
| `--rpc` | 通过 stdin/stdout 运行持久 JSONL 命令/事件协议。 |
| `--resume <ID>` | 恢复已保存会话。 |
| `--fork <ID>` | 将已保存会话 fork 成新会话。 |
| `--export-session <ID_OR_PATH>` | 把已保存会话渲染为本地 markdown 或 JSON 文件。 |
| `--output <PATH>` | `--export-session` 必需的输出路径。 |
| `--format <markdown\|json>` | 导出格式；`md` 可作为 markdown alias。 |
| `--full-tree` | 导出完整会话树，而不是只导出活跃分支。 |
| `--exclude-tool-output` | 从导出中省略工具结果输出。 |
| `--exclude-thinking` | 从导出中省略助手 thinking 内容。 |
| `--redact <summary\|verbose\|none>` | 导出脱敏模式；默认是 `summary`。 |
| `--tools read,grep` | 只启用列出的内置工具。 |
| `--no-tools` | 禁用所有工具。 |
| `--no-builtin-tools` | 关闭内置工具，同时保留 extension/custom 工具可用。 |
| `--allow-mutating` | 在非交互/RPC 运行中允许 `write`、`edit` 和 `bash`。 |
| `--trace <PATH>` | 为非交互或 JSON 运行写入可选、已脱敏的本地 trace envelope。 |

## Provider

Provider 支持在 `opi-ai` 中实现，并接入 `opi-coding-agent`。

| 前缀 | 后端 | 默认凭据 |
| --- | --- | --- |
| `anthropic:` | Anthropic Messages streaming | `ANTHROPIC_API_KEY` |
| `openai:` | OpenAI Chat Completions streaming | `OPENAI_API_KEY` |
| `openai-responses:` | OpenAI Responses streaming | `OPENAI_API_KEY` |
| `openrouter:` | OpenAI-compatible OpenRouter profile | `OPENROUTER_API_KEY` |
| `mistral:` | OpenAI-compatible Mistral profile | `MISTRAL_API_KEY` |
| `gemini:` | Gemini streaming | `GEMINI_API_KEY` |
| `bedrock:` | AWS Bedrock Converse streaming | AWS 环境变量或共享 AWS 配置 |
| `azure:` | Azure OpenAI deployment | `AZURE_OPENAI_API_KEY` 加 endpoint 配置 |
| `vertex:` | Google Vertex AI Gemini streaming | `VERTEX_ACCESS_TOKEN` 加 project/location 配置 |
| `github-copilot:` | 一个经审计的静态 catalog，路由到 Anthropic Messages、OpenAI Completions/Chat 与 OpenAI Responses | 通过 `/login github-copilot` 写入 OS keychain |
| `openai-codex:` | 专用 OpenAI Codex Responses wire | 通过 `/login openai-codex` 写入 OS keychain |
| 已配置 profile | OpenAI-compatible Chat Completions profile | profile 自己的 `api_key_env` |

兼容 OpenAI 风格的服务通常应通过已配置 profile 接入，而不是新增 first-class provider
模块。对 `usage_in_stream`，OpenAI-compatible profile 会请求
`stream_options.include_usage`；从任何 OpenAI Chat chunk 携带 `id` 的位置捕获 response ID，
并回写到 `response_id`。profile 还可以设置 `chat_completions_path`，用于已包含 API 前缀的
base URL。当 usage 或定价未知时，会省略费用汇总。

## 凭据、OAuth 与 Provider 元数据

`opi-ai` 定义无 IO 的 `CredentialStore`、`Credential`、`OAuthProvider` 和
`AuthResolver` 契约。`opi-coding-agent` 提供 OS-keychain
`CredentialResolver`、API key 的环境变量回退，以及一个不含秘密的协调文件
`credential.lock`。opi 不写入自行管理的明文凭据文件。`opi doctor` 与
`--list-models` 只探测已脱敏的凭据状态。Windows、macOS 和 Linux 上的持久化凭据
分别使用 Windows Credential Manager、macOS Keychain Services 和
Freedesktop Secret Service。

GitHub Copilot 使用规范 `github-copilot` identity，以及一个经审计的静态
pi-0.80.6 catalog；该 catalog 覆盖 Anthropic Messages、OpenAI Completions/Chat 与
OpenAI Responses route。这有意不同于在线 account entitlement filtering：
`--list-models` 读取静态 catalog，不读取 OAuth secret，也不请求 entitlement/model-enable
endpoint。

OpenAI Codex 使用规范 `openai-codex` identity、专用
`openai-codex-responses` wire，以及 Browser（默认）和 Device Code 登录。
只有 Browser PKCE flow 会等待手动 code 或 callback；GitHub Copilot 与 OpenAI Codex
Device Code 调用 `present_device_code`，绝不调用 `await_manual_code`。

持久化凭据使用原生 OS keychain；开发期 id `copilot` 与 `codex` 没有 alias 或凭据迁移，因此受影响用户必须使用规范 id 重新登录。
在输出开始前收到 `CredentialNeeded` 后，只有同一 provider 的显式登录成功，outer TUI 才会对同一待处理轮次精确重试一次，且不追加重复 user message。
非交互文本、JSON 与 RPC 模式会输出规范 provider 修复提示并失败，绝不构造 `LoginPresenter`、打开浏览器或等待输入。
`CredentialRevoked` 不可重试；
opi 不会在流中自动重新登录。

`Request` 现在携带 `timeout`、`extra_headers`、`CacheRetention` 和 `session_id`。
本阶段只有 `session_id` 具有生产 harness 生成方；provider 按审查过的 prompt-cache /
会话亲和规则映射它。`ModelInfo` 使用唯一的嵌套 `ModelCapabilities` 值，其中包括
Anthropic cache-control 能力。`cache_write_1h_tokens` 是 cache write 的子集，
`reasoning_tokens` 是 output 的子集，因此 token 总数与费用不会重复计算。
`Provider::refresh_models` 和 collection refresh 仅为基底、无生产触发。

## 内置工具

可用内置工具包括 `read`、`write`、`edit`、`bash`、`grep`、`find`、`ls` 和
`glob`。

| 模式 | 默认工具 |
| --- | --- |
| 交互式 TUI | `read`、`write`、`edit`、`bash` |
| 非交互 / RPC | `read`、`grep`、`find`、`ls`、`glob` |
| 非交互 / RPC 且显式允许修改 | `read`、`write`、`edit`、`bash` |

文件写入和编辑限制在 harness workspace 根目录内。交互式 `read` 可以检查绝对路径和
workspace 外路径。`bash` 从 workspace 根目录启动，但不限制在工作区内。这些都是工具
策略校验，不是操作系统级 sandbox。

工具结果携带 LLM 可见的 `content`、可选结构化 `details`、`is_error`、`terminate`、
`truncated` 和可选 diagnostics。`read` 返回行号/路径元数据，默认预览上限为 2000 行，
并受 64 KiB 字节上限约束。`bash` 运行一个前台命令，合并 stdout/stderr 上限为
64 KiB，并可能把完整输出路径写入 `details.full_output`。

## 配置与会话

配置会合并用户配置、项目配置和显式 `--config` 文件。模型选择优先级如下：

1. `--model`
2. 未传入 `--config` 时的 `OPI_MODEL`
3. `--config <FILE>` 中的 `model`
4. `<CWD>/.opi/config.toml`
5. 用户配置（Windows: `%APPDATA%\opi\config.toml`，Unix:
   `~/.config/opi/config.toml`）
6. 内置默认值

会话会自动写入 append-only JSONL 文件。

| 平台 | 默认会话目录 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Unix | `~/.local/share/opi/sessions/` |

可用 `OPI_SESSIONS_DIR` 覆盖该位置。会话文件属于敏感内容：其中包含提示词、工具输出，
以及可能泄露的密钥。v1 header 保持不变，并新增会话名称、模型变化、thinking
level、label 和 branch summary 的类型化条目。`--resume`、`--fork`、
`--list-sessions`、RPC `session_info` 和 `--export-session` 都通过同一条上下文路径重建
活跃分支。`opi --export-session` 只做本地导出，并默认脱敏。

## 扩展能力

`opi --rpc` 暴露不稳定 0.x JSONL 命令/事件协议。当前 wire 版本：

| 表面 | 当前版本 | 出现位置 |
| --- | --- | --- |
| NDJSON 模式 | `NDJSON_SCHEMA_VERSION = 2` | `opi --json` schema header |
| RPC / SDK | `SDK_SCHEMA_VERSION = 3` | `opi --rpc` 的 `rpc_ready.schema_version` |
| Trace envelope | `TRACE_SCHEMA_VERSION = 1` | `--trace <PATH>` 和 RPC `trace` payload |

RPC 命令包括 `prompt`、`continue`、`steer`、`follow_up`、`abort`、`set_model`、
`set_thinking_level`、`compact`、`session_info`、`extension_command`、`trace` 和
`quit`。

资源发现支持 extensions、packages、skills、prompt fragments 和 themes。
`opi package add/remove/list/doctor` 可用于本地和 git package source。Package manifest
可以启动使用 `opi-extension-jsonl-v1` 协议的 `process-jsonl` adapter；adapter 可以暴露
工具、命令、hooks、事件、状态以及模型/Provider 覆盖。

## 权限与信任边界

`opi` 以启动它的用户和进程的操作系统权限运行。工具选择和修改性工具参数只控制 Agent
可调用哪些内置工具；它们不是操作系统级 sandbox。

- `bash` 会以启动用户的 OS 权限执行命令。
- Package 是受信任代码。Package 可以启动与 `opi` 拥有相同 OS 权限的子进程；
  package 权限声明是元数据，不是强制 sandbox 策略。
- 可观测性是本地且显式的：`opi` 不收集 telemetry 或 analytics，不会自动分享会话，
  `opi doctor` 默认只做本地、无网络检查，`trace` 需要显式启用。
- 生产级子 Agent、permission gate、plan/todo 和 MCP 工作流是 examples/package
  模式，不是内置核心工作流。
- Anthropic、GitHub Copilot 与 OpenAI Codex 之外的 OAuth provider、两个经审计
  pi-0.80.6 snapshot 之外的 provider catalog、图像生成、浏览器自动化、面向 package
  的 provider 流式 adapter 协议、
  默认测试中的付费实时 provider 调用，以及复制 pi 的 provider 专用配置文件格式仍被推迟。
- 不支持从任意 extension 路径动态加载 Rust 插件。

如果需要更强隔离，请在容器、虚拟机或外部 sandbox 中运行 `opi`，并按暴露给它的工具和
凭据选择合适的边界。

## 开发

Workspace 使用 Rust edition 2024；声明 MSRV 为 Rust 1.97 或更新版本。

```sh
cargo build
cargo run -p opi-coding-agent -- --help
cargo test --workspace --all-targets
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

仓库协作规则见 [AGENTS.md](AGENTS.md)，技术规范草案见
[docs/opi-spec.zh.md](docs/opi-spec.zh.md)。

## 许可证

MIT (c) OdradekAI。详见 [LICENSE](LICENSE)。
