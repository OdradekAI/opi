# opi

[![CI](https://github.com/OdradekAI/opi/actions/workflows/ci.yml/badge.svg)](https://github.com/OdradekAI/opi/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> 受 [earendil-works/pi](https://github.com/earendil-works/pi) 启发的 Rust AI
> Agent 工具包与终端优先的编程 Agent。

[English](README.md) | [更新日志](CHANGELOG.md) | [技术规范草案](docs/opi-spec.zh.md)

## 当前状态

`Cargo.toml` 中的 workspace 包版本是 `0.6.3`。`opi` 既可以作为终端编程
Agent 使用，也可以作为一组 Rust crate 嵌入到其他 Agent 运行时中。当前 checkout
可能包含基于已发布 `0.6.3` crate 的未发布变更；当前差异见
[CHANGELOG.md](CHANGELOG.md)。

`opi` 用 Rust 重新实现 pi 的部分思路。它不与 pi API 兼容，默认不读取 pi
配置，并使用自己的 TOML 配置和 JSONL 会话格式。

已发布的 `0.6.3` 版本包含 Phase 10 的 provider collection/auth、通用 harness、
session facade 和运行时 hook 边界 seam，以及 Phase 11 工具质量工作。当前
workspace 还包含未发布的 Phase 12 provider 正确性变更：已有 provider family
具备 fixture-backed 的 request、streaming、tool-call、thinking/image、usage、
retry、cancellation 和错误分类覆盖；OpenAI-compatible profile 行为保持配置驱动并
已文档化。除非 crate README 另有明确说明，wire protocol、extension/package 表面、
trace payload 以及 Phase 10-12 seam 都应视为不稳定 0.x。

## 安装

CLI 二进制名为 `opi`，由 `opi-coding-agent` crate 产出。

```sh
cargo install opi-coding-agent
opi --version
```

Linux、macOS 和 Windows 的 x64/arm64 预编译二进制附在
[GitHub Releases](https://github.com/OdradekAI/opi/releases)。

## 快速开始

先设置要使用的 Provider 凭据：

```sh
export ANTHROPIC_API_KEY=sk-ant-...
# 或 OPENAI_API_KEY、OPENROUTER_API_KEY、MISTRAL_API_KEY、GEMINI_API_KEY
# 或 AWS 凭据、AZURE_OPENAI_API_KEY、VERTEX_ACCESS_TOKEN
```

启动交互式 TUI：

```sh
opi
```

运行单次提示词：

```sh
opi "列出这个 workspace 中的 Rust crate。"
```

为自动化输出 NDJSON 事件：

```sh
opi --json "总结这个仓库。"
```

给第一条提示词附加图片：

```sh
opi --image screenshot.png "审查这个 UI。"
```

使用 `provider:model` 语法选择模型：

```sh
opi -m anthropic:claude-sonnet-4-5-20250514 "解释 crates/opi-agent/src/lib.rs"
opi -m openai:gpt-4o "审查公共 API 形状。"
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

常用模式参数：

| 参数 | 作用 |
|------|------|
| `--non-interactive` | 强制单次文本模式。 |
| `--json` | 单次 NDJSON 事件流。 |
| `--rpc` | 通过 stdin/stdout 运行持久 JSONL 命令/事件协议。 |
| `--resume <ID>` | 恢复已保存会话。 |
| `--fork <ID>` | 将已保存会话 fork 成新会话。 |
| `--tools read,grep` | 只启用列出的内置工具。 |
| `--no-tools` | 禁用所有工具。 |
| `--allow-mutating` | 在非交互/RPC 运行中允许 `write`、`edit` 和 `bash`。 |
| `--trace <PATH>` | 为本次运行写入可选的、已脱敏的本地 trace envelope。 |

## Wire 版本

自动化和嵌入方表面带版本号，但仍是不稳定 0.x：

| 表面 | 当前版本 | 出现位置 |
|------|----------|----------|
| NDJSON 模式 | `NDJSON_SCHEMA_VERSION = 2` | `opi --json` schema header |
| RPC / SDK | `SDK_SCHEMA_VERSION = 3` | `opi --rpc` 的 `rpc_ready.schema_version` |
| Trace envelope | `TRACE_SCHEMA_VERSION = 1` | `--trace <PATH>` 和 RPC `trace` payload |

RPC 运行时状态拒绝可能带稳定的机器可读 `error_code`：`unsupported_trace_request`、
`agent_busy`、`harness_unavailable`、`compaction_failed` 或
`extension_command_not_handled`。`set_model` 和 `set_thinking_level` 的空闲态能力
校验错误仍是自由文本验证错误。

## Provider

Provider 支持在 `opi-ai` 中实现，并接入 `opi-coding-agent`。

| 前缀 | 后端 | 默认凭据 |
|------|------|----------|
| `anthropic:` | Anthropic Messages streaming | `ANTHROPIC_API_KEY` |
| `openai:` | OpenAI Chat Completions streaming | `OPENAI_API_KEY` |
| `openai-responses:` | OpenAI Responses streaming | `OPENAI_API_KEY` |
| `openrouter:` | OpenAI-compatible OpenRouter profile | `OPENROUTER_API_KEY` |
| `mistral:` | OpenAI-compatible Mistral profile | `MISTRAL_API_KEY` |
| `gemini:` | Gemini streaming | `GEMINI_API_KEY` |
| `bedrock:` | AWS Bedrock Converse streaming | AWS 环境变量或共享 AWS 配置 |
| `azure:` | Azure OpenAI deployment | `AZURE_OPENAI_API_KEY` 加 endpoint 配置 |
| `vertex:` | Google Vertex AI Gemini streaming | `VERTEX_ACCESS_TOKEN` 加 project/location 配置 |
| 已配置 profile | OpenAI-compatible Chat Completions profile | profile 自己的 `api_key_env` |

兼容 OpenAI 风格的服务通常应通过已配置 profile 接入，而不是新增 first-class
provider 模块。只有 wire、auth、streaming、tool、image 或能力语义存在实质差异时，
才应添加 first-class adapter。

## 内置工具

可用内置工具包括 `read`、`write`、`edit`、`bash`、`grep`、`find`、`ls` 和
`glob`。

默认启用工具取决于运行模式：

| 模式 | 默认工具 |
|------|----------|
| 交互式 TUI | `read`、`write`、`edit`、`bash` |
| 非交互 / RPC | `read`、`grep`、`find`、`ls`、`glob` |
| 非交互 / RPC 且显式允许修改 | `read`、`write`、`edit`、`bash` |

文件写入和编辑限制在 harness workspace 根目录内。交互式 `read` 可以检查绝对
路径和 workspace 外路径。这些规则是工具策略，不是操作系统级 sandbox。

当前工具结果可检查，但没有扩张内置工具集：

- 内置工具结果携带 `content`、可选 `details`、`is_error`、`terminate`、
  `truncated` 和可选结构化诊断。
- `read` 返回行号和路径元数据，默认省略输出上限为 2000 行，并仍受 64 KiB 字节
  上限约束。
- `bash` 运行一个前台命令，报告 cwd/shell/exit/timeout/cancel 元数据，合并后的
  stdout/stderr 上限为 64 KiB，并可能把完整输出路径写入 `details.full_output`。
- `grep`、`find`、`ls` 和 `glob` 共享 gitignore-aware 遍历、确定性排序、有界结果
  数量和 skipped-file 诊断。
- 工具选择安全不是权限系统。需要操作系统级隔离时，请使用外部容器、虚拟机或
  sandbox。

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
|------|--------------|
| Windows | `%LOCALAPPDATA%\opi\sessions\` |
| Unix | `~/.local/share/opi/sessions/` |

可用 `OPI_SESSIONS_DIR` 覆盖该位置。

## Workspace Crates

所有 crate 共享 workspace 的版本、edition、license、repository 和 authors。

| Crate | 是否发布 | 作用 |
|-------|----------|------|
| [`opi-ai`](crates/opi-ai) | 是 | Provider 无关 LLM API、流式事件、模型注册表、重试、HTTP/代理、用量与费用辅助。 |
| [`opi-agent`](crates/opi-agent) | 是 | Agent 主循环、工具执行、hooks、事件、队列、会话、压缩、SDK 类型、扩展、streaming proxy。 |
| [`opi-tui`](crates/opi-tui) | 是 | Ratatui 组件、对话渲染、diff 视图、选择器、终端图片、主题、按键绑定。 |
| [`opi-coding-agent`](crates/opi-coding-agent) | 是 | `opi` 二进制与可嵌入编程 harness。 |

内部依赖形状：

```text
opi-ai
opi-tui
opi-agent -> opi-ai
opi-coding-agent -> opi-ai + opi-agent + opi-tui -> opi binary
```

## 扩展能力

`opi --rpc` 暴露不稳定的 0.x JSONL 命令/事件协议，客户端必须检查 schema
version。`opi-agent` 也为嵌入方提供共享 SDK 类型和 extension registry 基础能力。
RPC 命令包括 `prompt`、`continue`、`steer`、`follow_up`、`abort`、`set_model`、
`set_thinking_level`、`compact`、`session_info`、`extension_command`、`trace` 和
`quit`。

资源发现支持 extensions、packages、skills、prompt fragments 和 themes。Package
manifest 可以启动 `process-jsonl` adapter，用于暴露自定义工具、命令、hooks、事件
观察器、状态以及模型/Provider 覆盖。

## 权限与信任边界

`opi` 以启动它的用户和进程的操作系统权限运行。工具选择和修改性工具参数只控制
Agent 可调用哪些内置工具；它们不是操作系统级 sandbox。

- 文件写入和编辑限制在 harness workspace 根目录内。`bash` 从 workspace 根目录启动，
  但会以启动用户的 OS 权限执行命令。
- Package 是受信任代码。Package 可以启动与 `opi` 拥有相同 OS 权限的子进程；
  package 权限声明是元数据，不是强制 sandbox 策略。
- 可观测性是本地且显式的：`opi` 不收集 telemetry 或 analytics，不会自动分享会话，
  `opi doctor` 默认只做本地、无网络检查，trace 需要显式启用。
- 生产级子 Agent、permission gate、plan/todo 和 MCP 工作流不内置在核心 CLI 中；
  仓库提供相关 examples 与 package 脚手架。
- Phase 12 provider 正确性已经覆盖已有 provider，而不是扩张 provider 宽度。
  OAuth 登录、Anthropic / OpenAI Codex / GitHub Copilot 订阅鉴权、大范围新增
  first-class provider 列表、图像生成、浏览器使用、面向 package 的 provider 流式
  adapter 协议、默认测试中的付费实时 provider 调用，以及复制 pi 的 provider 专用
  配置文件格式仍被推迟。详见 `opi-ai` README 的按 family 行为矩阵、
  OpenAI-compatible profile 标志、缓存 / response-ID / 会话亲和行为、代理和
  尽力而为的费用。关键 Phase 12 细节包括：`usage_in_stream` 请求
  `stream_options.include_usage`；从任何携带 `id` 的 OpenAI Chat chunk 捕获 response ID
  并回写到 `response_id`；当 usage 或定价未知时省略费用汇总。

- 不支持从任意 extension 路径动态加载 Rust 插件。

如果需要更强隔离，请在容器、虚拟机或外部 sandbox 中运行 `opi`，并按暴露给它的
工具和凭据选择合适的边界。

## 开发

Workspace 使用 Rust edition 2024，因此需要 Rust 1.85 或更新版本。

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
