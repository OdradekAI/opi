# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/opi-coding-agent.svg)](https://crates.io/crates/opi-coding-agent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Opi 是一个 Rust AI Agent 工具包和终端优先的编程 Agent。你可以把它作为交互式
TUI、单次运行的 CLI、支持 NDJSON/RPC 的自动化进程，或一组可嵌入的 Rust crate
使用。

Opi 延续了 [earendil-works/pi](https://github.com/earendil-works/pi) 所展示的“小核心、
可扩展 harness”设计，但使用自己的 Rust API、TOML 配置和仅追加 JSONL 会话格式。

[English](README.md) | [发布版本](https://github.com/OdradekAI/opi/releases) | [更新日志](CHANGELOG.md)

## 主要特性

- 交互式编程 TUI，支持流式输出、会话与分支导航、模型选择、图片附件、diff、主题和
  终端图片。
- 面向脚本和嵌入方的单次文本、NDJSON 和持久 JSONL RPC 模式。
- 支持 Anthropic、OpenAI、OpenRouter、Mistral、Gemini、Bedrock、Azure OpenAI、
  Vertex AI、GitHub Copilot、OpenAI Codex，以及自定义 OpenAI 兼容 Provider。
- 八个内置编程工具；交互模式与自动化模式采用不同的默认工具集。
- 本地仅追加会话，支持恢复、fork、导出、压缩和崩溃恢复。
- 提供 Provider API、Agent 运行时、终端 UI、编程 harness、命令执行协议和可选进程
  限制能力的 Rust crate。

## 当前状态

`Cargo.toml` 中的 workspace 包版本为 `0.8.0`。当前 checkout 可能包含尚未发布的
变更；在判断发布状态或兼容性前，请查看 [CHANGELOG.md](CHANGELOG.md)。

终端 Agent 已可实际使用。公开 crate 与 wire format 仍处于 0.x 阶段：嵌入 Opi 时
应固定精确版本，并以更新日志记录的协议/API 变更为准。

## 安装

从 crates.io 安装 `opi` 二进制。从源码构建需要 Rust 1.97 或更高版本。

```sh
cargo install opi-coding-agent
opi --version
```

[GitHub Releases](https://github.com/OdradekAI/opi/releases) 提供 Linux、macOS 和
Windows 的 x64/arm64 预编译 `opi` 压缩包。下载对应平台的压缩包，并将 `opi`
可执行文件放入 `PATH`。

`opi-sandbox` 是独立的可选产品，不会随 `opi-coding-agent` 安装，也不会链接进
`opi`。

## 快速开始

Opi 的内置默认配置使用 Anthropic。设置一个 Provider 凭据，然后启动 TUI：

```sh
export ANTHROPIC_API_KEY=sk-ant-...
opi
```

PowerShell：

```powershell
$env:ANTHROPIC_API_KEY = "sk-ant-..."
opi
```

运行一次提示词并输出最终答案：

```sh
opi "总结这个仓库。"
```

使用 `provider:model` 语法选择模型：

```sh
opi -m openai:gpt-4o "审查公开 API。"
```

在 TUI 中，可以直接配置基于 OAuth 的 Provider，无需设置环境变量 API key：

```text
/login anthropic
/login github-copilot
/login openai-codex
/logout <provider>
```

非交互、NDJSON 和 RPC 模式不会启动登录流程；它们只会报告对应 Provider 和
`/login` 修复方式。

## 常用操作

| 目标 | 命令 |
| --- | --- |
| 启动交互式 TUI | `opi` |
| 运行一次提示词 | `opi "PROMPT"` |
| 输出 NDJSON 事件流 | `opi --json "PROMPT"` |
| 减少长 NDJSON 流体积 | `opi --json --json-compact "PROMPT"` |
| 启动 JSONL RPC 进程 | `opi --rpc` |
| 为每次运行捕获不可变证据 | `opi --trace .opi-traces "PROMPT"` |
| 给第一条提示词附加图片 | `opi --image screenshot.png "审查这个 UI"` |
| 列出已配置模型 | `opi --list-models` |
| 检查本地配置与健康状态 | `opi doctor` |
| 生成 shell 补全 | `opi --generate-completion powershell` |

使用 `opi --help` 查看当前完整参数说明。

`--trace PATH` 适用于交互式、非交互文本、NDJSON 和 RPC 模式。`PATH` 是捕获根目录：
每次 prompt、continue、retry 或手动压缩都会创建唯一子目录，其中包含
`evidence.jsonl` 和 `manifest.json`；已 finalized 的运行目录不可变。

### 会话

会话会自动写入仅追加 JSONL 文件。文件可能包含提示词、工具输出、路径和密钥；应像
保护本地开发日志一样保护它们。

```sh
opi --list-sessions
opi --resume <ID>
opi --fork <ID>
opi --delete-session <ID>
opi --export-session <ID_OR_PATH> --output session.md
opi --export-session <ID_OR_PATH> --output session.json --format json
```

导出默认使用 `summary` 脱敏。高级选项包括 `--full-tree`、
`--exclude-tool-output`、`--exclude-thinking` 和
`--redact summary|verbose|none`。

默认会话目录：

| 平台 | 目录 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Linux/macOS | `~/.local/share/opi/sessions/` |

可通过 `OPI_SESSIONS_DIR` 指定其他目录。

## Provider 与模型

配置凭据或 Provider 设置后，运行 `opi --list-models` 查看可用模型。

| 前缀 | 后端 | 默认认证方式 |
| --- | --- | --- |
| `anthropic:` | Anthropic Messages | `ANTHROPIC_API_KEY` 或 `/login anthropic` |
| `openai:` | OpenAI Chat Completions | `OPENAI_API_KEY` |
| `openai-responses:` | OpenAI Responses | `OPENAI_API_KEY` |
| `openrouter:` | OpenRouter | `OPENROUTER_API_KEY` |
| `mistral:` | Mistral | `MISTRAL_API_KEY` |
| `gemini:` | Gemini | `GEMINI_API_KEY` |
| `bedrock:` | AWS Bedrock Converse | AWS 环境变量或共享 profile 凭据 |
| `azure:` | Azure OpenAI | `AZURE_OPENAI_API_KEY` 和 endpoint 配置 |
| `vertex:` | Vertex AI Gemini | `VERTEX_ACCESS_TOKEN` 和 project/location 配置 |
| `github-copilot:` | GitHub Copilot | `/login github-copilot` 和 OS keychain |
| `openai-codex:` | OpenAI Codex Responses | `/login openai-codex` 和 OS keychain |
| 自定义 profile | OpenAI 兼容 Chat Completions | 配置的 `api_key_env` |

OAuth 凭据存储在 Windows Credential Manager、macOS Keychain 或 Freedesktop
Secret Service 中。Opi 不会创建明文凭据文件。

## 配置与项目信任

配置使用 TOML。常用的模型选择优先级如下：

1. `--model`
2. 未传入 `--config` 时的 `OPI_MODEL`
3. 显式 `--config <FILE>` 中的 `model`
4. 已信任项目的 `.opi/config.toml`
5. 用户配置
6. 内置默认值

用户配置路径在 Windows 上为 `%APPDATA%\opi\config.toml`，在 Linux/macOS 上为
`~/.config/opi/config.toml`。Opi 启动时还会加载当前目录的 `.env`；不要提交其中的
凭据。

最小用户配置可以只选择默认模型：

```toml
model = "openai:gpt-4o"

[defaults]
default_project_trust = "ask"
allow_mutating_tools = false
```

项目信任只在启动时解析一次。未信任的项目不会加载 `.opi/config.toml`、项目
package 和资源，也不会注入项目 `AGENTS.md`/`CLAUDE.md`；工具本身不受影响。使用
`--trust` 或 `--no-trust` 可覆盖本次运行。Opi 没有内置 `/trust` 命令，也不会在
会话中途重新加载项目资源。

自定义 Provider profile 和完整配置项请参阅
[编程 Agent crate 指南](crates/opi-coding-agent/README.zh.md)。

## 工具与自动化策略

内置工具包括 `read`、`write`、`edit`、`bash`、`grep`、`find`、`ls` 和
`glob`。

| 模式 | 默认工具 |
| --- | --- |
| 交互式 TUI | `read`、`write`、`edit`、`bash` |
| 非交互 / RPC | `read`、`grep`、`find`、`ls`、`glob` |
| 非交互 / RPC 加 `--allow-mutating` | `read`、`write`、`edit`、`bash` |

使用 `--tools read,grep`、`--no-tools` 或 `--no-builtin-tools` 缩小可用工具集。
`write` 和 `edit` 限制在 workspace 根目录内。`bash` 从 workspace 根目录启动，但
不受路径限制，并以启动用户的操作系统权限运行。

这些设置用于选择 Agent 工具，不是操作系统 sandbox。

## Package 与命令执行

Package 可以提供 skill、prompt fragment、主题、extension 和 `command.execute`
adapter。Extension 可以提供命令与 hook，但其工具贡献目前不会注册到 Reference
Product。

```sh
opi package add <PATH_OR_GIT_URL>
opi package list
opi package doctor
opi package enable <NAME>
opi package disable <NAME>
opi package remove <NAME_OR_SOURCE>
```

可执行贡献经过五个相互独立的状态：Installed、Trusted、Enabled、Selected 和
Permitted。`package add` 只负责安装；第一次执行 `package enable` 时会确认 Package
Trust 并启用贡献。用户执行策略另行决定是否允许每次调用。

Minimal Runtime 默认通过内置 `local` 后端执行 `bash`。
`--execution-strategy fixed|rules|model` 与
`--execution-backend local|ADAPTER_ID` 可以选择已安装的 `command.execute`
adapter。外部 adapter 一旦被选中，任何 adapter 或协议失败都会 fail-closed，绝不
回退到本地执行。

原核心 `[sandbox]`、`--sandbox` 和 `--sandbox-require` 设置已经移除。
`opi-sandbox` 是可选的独立 SDK/CLI 和原生进程限制协议后端。它只依赖
`opi-protocol`；`opi` 二进制不会链接它。

官方 `opi-sandbox` 产物只面向 Linux 和 macOS。Windows 仅提供 L0 Job-Object
进程树监控。Opi 不内置 Docker、VM、SSH 或远程执行 adapter；package 进程拥有与
`opi` 相同的 OS 权限，因此应将 package 视为受信任代码。`opi-sandbox` 只为目标
进程树提供纵深防御，不是完整的安全边界。

## 嵌入与协议

| Crate | 用途 |
| --- | --- |
| [`opi-ai`](crates/opi-ai) | Provider 无关模型 API、流式处理、用量、重试、凭据和模型注册表 |
| [`opi-agent`](crates/opi-agent) | 产品无关 Agent 主循环、工具、hook、队列、会话、压缩、SDK 和 extension |
| [`opi-tui`](crates/opi-tui) | Ratatui 组件、对话与 diff 渲染、选择器、主题和终端图片 |
| [`opi-coding-agent`](crates/opi-coding-agent) | `opi` CLI、内置工具、package/config/session 装配和 `CodingHarness` |
| [`opi-protocol`](crates/opi-protocol) | 版本化 `command-execution-jsonl-v1` 协议类型、schema、codec 和 fixture |
| [`opi-sandbox`](crates/opi-sandbox) | 独立命令限制 SDK/CLI 和协议后端 |

面向机器的表面仍是不稳定 0.x 契约：

| 表面 | 版本 |
| --- | --- |
| NDJSON | `NDJSON_SCHEMA_VERSION = 2` |
| SDK / RPC | `SDK_SCHEMA_VERSION = 3` |

精确契约以 crate 文档和实际输出的 schema/version header 为准，不能从 workspace
包版本推断协议兼容性。

## 开发与贡献

Workspace 使用 Rust edition 2024，并要求 Rust 1.97 或更高版本。

```sh
cargo build
cargo run -p opi-coding-agent -- --help
cargo test --workspace --all-targets
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
```

修改仓库前请阅读 [AGENTS.md](AGENTS.md)，其中包含权威来源索引、架构边界、测试
策略和 Git 安全规则。持久技术方向见
[docs/opi-spec.zh.md](docs/opi-spec.zh.md)。Bug 报告与功能讨论请提交到
[GitHub Issues](https://github.com/OdradekAI/opi/issues)。

## 许可证

[MIT](LICENSE) © OdradekAI。
