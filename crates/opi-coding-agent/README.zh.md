# opi-coding-agent

[![Crates.io](https://img.shields.io/crates/v/opi-coding-agent.svg)](https://crates.io/crates/opi-coding-agent)
[![Docs.rs](https://docs.rs/opi-coding-agent/badge.svg)](https://docs.rs/opi-coding-agent)

> `opi` 二进制与可嵌入编程 harness。

[English](README.md) | [opi workspace](../../README.zh.md)

## 当前状态

当前 crate 版本是 `0.6.5`，继承自 workspace 包版本。

本 crate 把 `opi-ai`、`opi-agent` 和 `opi-tui` 连接成终端编程 Agent。它提供：

- `opi` CLI 二进制；
- 交互式 ratatui TUI 模式；
- 单次文本模式和 `--json` NDJSON 模式；
- `--rpc` JSONL 命令/事件模式；
- 模型、会话、分支和会话树选择器；
- 通过 `--image` 和 `/image` 附加图片；
- 会话 list/resume/fork/delete 命令；
- 8 个内置工具；
- 配置、上下文文件加载、会话持久化、压缩、重试、用量、费用摘要、package/资源发现、
  诊断和可选 trace。

workspace 包版本是 `0.6.5`；当前 checkout 也可能包含尚未发布的 Phase 13 会话工作。
近期阶段在不新增核心工作流工具的前提下加固既有行为。Phase 11 让文件系统失败携带
类型化诊断，read/bash 输出截断成为显式状态，write/edit 记录可审计元数据，导航工具
共享有界的 gitignore-aware 遍历，并让失败工具结果保留到 provider adapter。
Phase 12 把 provider 正确性接入 CLI 与 harness：provider 构造会用已脱敏 diagnostics
校验 auth/config，OpenAI-compatible profile 携带已文档化的兼容性标志和
`extra_headers`，provider 错误会进入 text/JSON/RPC diagnostics；当任一轮 usage 未知
或定价未知时，会话费用汇总会被省略。Phase 13 增加类型化会话 metadata、分支摘要、
活跃分支重建诊断、持久化 model/thinking 状态和本地会话导出命令。

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
| `--rpc` | 通过 stdin/stdout 启动双向 JSONL 命令/事件模式。 |
| `--allow-mutating` | 在交互模式之外允许 `write`、`edit` 和 `bash`。 |
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
| `--redact <summary\|verbose\|none>` | `--export-session` 的第 7 阶段脱敏模式。 |
| `--generate-completion <SHELL>` | 为 `bash`、`zsh`、`fish`、`powershell` 或 `elvish` 生成补全。 |
| `--trace <PATH>` | 为非交互/JSON 运行写入可选的、已脱敏本地 trace envelope。 |
| `doctor [--json] [--scope ...]` | 本地、无网络健康检查。 |
| `package <add|remove|list|doctor>` | 管理本地/git extension package。 |

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
| 已配置 profile | OpenAI-compatible profile | profile 自己的 `api_key_env`、`base_url` 和模型列表 |

Provider 凭据环境变量名、base URL、模型列表和代理都可以在配置中覆盖。

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

八个内置工具分为只读和修改性两组。修改性工具仅在已解析策略允许时运
行；其余约束通过工具选择落实，而非操作系统级 sandbox 或交互式权限提示。

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
| 超时 | 默认 30 秒；`timeout_secs` 可覆盖。 |
| 取消 | 取消令牌报告 `cancelled=true` / `timed_out=false`；超时报告 `timed_out=true` / `cancelled=false`。 |
| 路径限制 | 无 —— `bash` 不限制在工作区内。 |
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

- 内置权限弹窗或交互式批准提示
- 持久后台 bash 或 shell 会话
- 远程执行
- IDE 项目索引
- 语言服务器集成
- `write` / `edit` 时自动格式化
- package 生态扩展
- todo、plan mode 或 sub-agents 等工作流工具
- sandbox 实现

修改性工具安全是工具选择校验，不是权限或 sandbox 子系统。

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
| `exit` / `quit` | 退出。 |

### 非交互与 JSON

文本模式把助手文本写到 stdout，把诊断写到 stderr。`--json` 会输出 schema header、
序列化 session/agent 事件，以及最终 `session_summary`，格式为 NDJSON。当前 NDJSON
schema version 是 `NDJSON_SCHEMA_VERSION = 2`。

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
opi package list
opi package list --json
opi package doctor
opi package doctor --json
opi package remove todo
```

Package 可以启动使用 `opi-extension-jsonl-v1` 协议的 `process-jsonl` adapter。该
adapter 协议是不稳定的 0.x 契约。Package 是受信任代码，不会被 package manager
sandbox。

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
- 生产级子 Agent、permission gate、plan/todo 和 MCP 工作流是 examples/package
  模式，不是内置核心工作流。
- OAuth 或订阅登录流程尚未实现。第十二阶段 provider 正确性是正确性阶段，而非宽度
  阶段；以下仍是推迟的产品决策：OAuth 登录、Anthropic / OpenAI Codex / GitHub
  Copilot 订阅鉴权、大范围新增 first-class provider 列表（兼容 provider 保持为
  config-driven 的 OpenAI-compatible profile）、图像生成（图片支持仅为输入侧）、
  浏览器使用、面向 package 的 provider 流式 adapter 协议、默认测试中的付费实时
  provider 调用，以及复制 pi 的 provider 专用配置文件格式。按 provider 的代理配置
  （`proxy.url` / `proxy.no_proxy`，环境变量 `HTTPS_PROXY` > `HTTP_PROXY` >
  `NO_PROXY`）和尽力而为的费用（显式未知值优先于虚假置信）已实现。详见 `opi-ai`
  README 的按 family 行为矩阵、OpenAI-compatible profile 标志（`system_role_override`、
  `max_tokens_field`、`tool_result_name_field`、`usage_in_stream`、
  `strict_tool_schema`、`reasoning_effort`、`cache_key`、
  `require_assistant_after_tool_result`；外加用于静态请求 header 的按 profile
  `extra_headers`，它是 profile 配置字段，不是 `CompatConfig` 标志）、OpenAI Responses
  原生语义（`store` / `reasoning_effort` / `strict_tools` 已实现；
  `previous_response_id` 推迟），以及缓存 / response-ID / 会话亲和行为。具体来说，
  `usage_in_stream` 会请求 `stream_options.include_usage`，OpenAI Chat 会从任何携带 `id` 的 chunk
  捕获 response ID，`require_assistant_after_tool_result` 在共享适配器中保持为纯元数据，而当任一轮
  usage 未知或定价未知时，会话费用汇总会被省略。Phase 13
  会话工作可以依赖经由共享 `opi-ai` 类型传递的 provider-correct usage、model、thinking
  和 error 数据，而无需依赖 provider 专用内部实现。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。
