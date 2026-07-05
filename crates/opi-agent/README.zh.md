# opi-agent

[![Crates.io](https://img.shields.io/crates/v/opi-agent.svg)](https://crates.io/crates/opi-agent)
[![Docs.rs](https://docs.rs/opi-agent/badge.svg)](https://docs.rs/opi-agent)

> [opi](https://github.com/OdradekAI/opi) 使用的 Provider 无关 Agent 运行时。

[English](README.md) | [opi workspace](../../README.zh.md)

## 当前状态

当前 crate 版本是 `0.6.3`，继承自 workspace 包版本。

`opi-agent` 负责 Agent 主循环和运行时基础能力：工具契约、JSON Schema 参数校验、
并行/串行工具执行、生命周期 hooks、事件输出、steering/follow-up 队列、会话
JSONL 存储、分支重建、上下文压缩、SDK/RPC 类型、扩展、本地诊断、已脱敏 trace
envelope，以及 streaming proxy。

已发布 `0.6.3` crate 之后的当前 workspace 变更仍保持运行时契约聚焦，不新增核心
工作流。Phase 11 把 `truncated` 和工具自有结构化诊断加入工具契约。Agent 主循环会
把这些诊断提升为共享 diagnostic/trace 记录，并在公共 `ToolExecutionEnd` 事件上暴露；
面向 provider 的工具结果消息仍只携带 LLM 可见内容和失败状态。

Phase 12 使用既有运行时表面承载 provider 正确性：来自 `opi-ai` 的
`ProviderErrorCategory` 会映射为已脱敏 diagnostics 和 trace 记录；provider 返回的
cancellation 会表现为 `AgentError::Cancelled`；retry diagnostics 会区分重试预算耗尽与
已有部分 provider 输出后的重试抑制；provider metadata 在公共 event、session、JSON、
RPC 和 trace 边界保持有界。

它依赖 `opi-ai` 的 Provider 和消息类型。它不实现 `opi` CLI、终端 UI 或具体的
文件/ shell 内置工具；这些能力分别位于 `opi-coding-agent` 和 `opi-tui`。

## 核心抽象

| 项 | 作用 |
|----|------|
| `Agent` | 对主循环的有状态封装，提供 prompt、continue、abort、subscribe、steering、follow-up、模型切换和工具注册辅助。 |
| `Tool` | 基于 JSON Schema 的工具契约，支持取消和可选进度更新。 |
| `ExecutionMode` | 控制工具能否进入并行批次，或是否强制串行执行。 |
| `AgentHooks` | 覆盖上下文转换、LLM 转换、工具策略/结果、停止判断和下一轮准备的生命周期 hooks。 |
| `AgentEvent` | 运行时事件流，覆盖生命周期、流式文本、工具调用、队列、重试、压缩和结束状态。 |
| `AgentSessionEvent` | `opi --json` 使用的会话级事件协议。 |
| `AgentLoopConfig` | 主循环限制、重试配置、压缩配置和相关运行时设置。 |

## 主循环形状

主循环按固定的运行时事件顺序执行。`AgentStart` 在首轮之前仅触发一次，
`AgentEnd` 在每条终止路径上仅触发一次（正常停止、Hook 停止、terminate 标志、
取消或错误）。每个 turn（`0..max_turns`）内：

```text
agent_start                              # 仅一次，首轮之前
  对每个 turn：
    cancel check                          # 被取消 -> AgentEnd, AgentError::Cancelled
    turn_start
    transform_context                     # AgentHooks::transform_context
    convert_to_llm                        # AgentHooks::convert_to_llm
    validate request capabilities         # 失败 -> AgentEnd, AgentError::Provider
    provider.stream(Request)
      message_start                       # assistant 流 Start
      message_update                      # 每个文本/思考 delta
      message_end                         # 完整的 assistant 消息
      若存在 tool call：
        validate tool args (jsonschema)
        tool_execution_start              # 每个 tool call
        before_tool_call                  # AgentHooks::before_tool_call（可阻止）
        tool.execute                      # 并行批次；若任一工具为 Sequential 则整批串行
        after_tool_call                   # AgentHooks::after_tool_call（可替换结果）
        tool_execution_end                # 每个 tool call
        turn_end                          # assistant 消息 + tool_results
        若所有结果都 terminate -> AgentEnd, return Ok
        should_stop_after_turn            # true -> AgentEnd, return Ok（压缩停止）
      否则：
        turn_end                          # assistant 消息，无 tool_results
        should_stop_after_turn            # true -> AgentEnd, return Ok
    prepare_next_turn                     # AgentHooks::prepare_next_turn；在终止的
                                         # should_stop_after_turn 之后被跳过；可注入消息
    drain steering queue                  # 非空 -> QueueUpdate，追加，进入下一 turn
    若无待处理工具：
      pop follow-up queue                 # 非空 -> QueueUpdate，追加，进入下一 turn
      否则 -> 停止
agent_end                                 # 仅一次，终止时
```

边界：

- `should_stop_after_turn` 在 `turn_end` 之后、`prepare_next_turn` 及任何队列
  轮询之前执行。压缩协调器在此返回 `true` 以在下一 turn 之前停止；终止停止
  之后不会运行 `prepare_next_turn`，也不会轮询 steering/follow-up。
- `prepare_next_turn` 仅在 `should_stop_after_turn` 允许继续时执行，且早于
  steering/follow-up 轮询；注入的消息会进入下一次 provider 请求。
- Steering 先于 follow-up 被排空。仅当无待处理工具且 steering 队列为空时，
  才弹出 follow-up。
- `CompactionEngine` 只是上下文大小的原语；将压缩与持久化 CLI 会话相连的
  高层协调器位于 `opi-coding-agent`，并通过 `should_stop_after_turn` 停止主循环。

Rate limit 和 timeout 等可重试 Provider 错误可通过 `AgentLoopConfig.retry` 处理。
重试开始/结束会通过 `AgentEvent` 暴露。

## Hook 语义

`AgentHooks` 用于定制主循环。六个方法按以下顺序执行，效果如下：

| Hook | 顺序 / 效果 |
|------|------------|
| `transform_context` | 在 Provider 转换之前运行；可改写应用层消息。 |
| `convert_to_llm` | 将应用消息转换为 Provider 消息，并过滤仅会话状态。 |
| `before_tool_call` | 在 JSON Schema 参数校验之后、`tool.execute` 之前运行；可 `Deny` 阻止执行（拒绝原因成为工具错误）。 |
| `after_tool_call` | 在执行之后、最终的 `ToolExecutionEnd` 事件之前运行；可 `Replace` 结果，使替换后的结果成为被发出和持久化的值。 |
| `should_stop_after_turn` | 在 `turn_end` 之后、steering/follow-up 轮询之前运行；返回 `true` 会在下一 turn 之前停止，并跳过 `prepare_next_turn`。 |
| `prepare_next_turn` | 仅在 `should_stop_after_turn` 允许继续时运行，且早于 steering/follow-up 轮询；可向下一次 provider 请求注入消息。 |

扩展组合：`ExtensionRegistry::wrap_hooks` 先运行基础 `AgentHooks` 方法，再按注册顺序依次运行每个扩展。
扩展的 `on_before_tool_call` 返回 `Block` 会在首个 block 处中断链路；后续扩展不会被调用。
扩展的 `on_after_tool_call` 观察者不能修改结果；只有基础 hook 可以 `Replace`。

当 adapter 或扩展只实现了部分 hook 时，在启用 verbose trace 的情况下，被跳过的 hook 会以
`trace::TraceKind::HookSkipped` 记录写入 trace。运行时会在每次运行之前通过
`Extension::set_trace_collector` 把本次运行的 `TraceCollector` 下发给每个扩展（运行结束后清空），
从而使短路了未声明 hook 的 adapter 能够记录该跳过。

## 工具调度

调度器会把一条 assistant 消息携带的工具调用收集为一个批次，并按以下规则执行：

- 全局默认执行模式为 `Parallel`。工具可通过实现 `Tool::execution_mode` 返回
  `Sequential` 来覆盖默认值。
- 若批次中任意工具调用声明为 `Sequential`，则整个批次串行执行；否则并行执行。
- 串行批次严格按 assistant 源顺序执行工具调用：每个调用先启动、执行、完成，
  之后下一个才开始。
- 并行批次会在等待任意结果之前为每个工具发出 `ToolExecutionStart`，并用
  `join_all` 收集结果（保留源顺序）。因此当前运行时按源顺序发出
  `ToolExecutionEnd`；契约允许按完成顺序发出，因此观察者不应依赖并行工具之间
  的具体结束事件顺序。
- 无论串行还是并行，持久化的 `ToolResult` 消息都按 assistant 源顺序排列，
  与完成顺序无关。
- 仅当批次中每一个已完成的工具结果都设置 `terminate` 时，运行才提前终止。
  只要有一个非终止结果，运行就继续到下一 turn。

参数校验在 `before_tool_call` 和 `Tool::execute` 之前执行。校验失败是正常的
运行时结果，而非循环错误：会持久化一个错误 `ToolResult`（`is_error = true`、
`terminate = false`）并继续运行；hook 不会执行，工具也不会执行。

## 工具结果与诊断

`ToolResult` 是内置工具、自定义工具和扩展工具共享的运行时结果契约：

| 字段 | 含义 |
|---|---|
| `content` | LLM 可见的文本或图片输出。 |
| `details` | 面向运行时、UI、JSON/RPC 和 trace 边界的可选结构化元数据。 |
| `is_error` | 该结果是否表示工具失败。 |
| `terminate` | 当同一批次中的每个结果也终止时，该结果是否可以结束运行。 |
| `truncated` | 输出是否被缩短或受界限限制。 |
| `diagnostics` | 工具自有结构化原因记录（`code`、`message`、`context`）。 |

Agent 主循环会在 `after_tool_call` 之后读取 diagnostics，因此替换后的结果也能替换
诊断上下文。每个 `ToolDiagnostic` 会提升为共享 `Diagnostic` 和 diagnostic-linked
trace 记录。公共事件在发出前会脱敏；provider 请求只通过
`opi_ai::message::ToolResultMessage` 接收工具结果 content、`is_error`、
`truncated` 和时间戳字段。

## 取消（Cancellation）

取消在所有路径上共享同一个可观察契约——provider 流、工具、adapter 尽力取消
（best-effort cancel）、RPC abort、交互式 abort 以及 shutdown。内部机制各不相同，
但结果一致：被取消的工作会发出终止事件或诊断，不会留下挂起的 run，且会话存储
只记录已 finalized 的状态。

在 `agent_loop` 中，每个 turn 会在三处检查同一个 `CancellationToken`：turn 开始
之前、provider 流式过程中、以及重试退避期间。一旦观察到取消，循环会记录一条信息级
的 `agent cancelled` 诊断（标注生命周期阶段），发出携带已 finalized 消息缓冲区的终止
`AgentEnd` 事件，并返回 `Err(AgentError::Cancelled)`。in-flight assistant 消息累积的
部分流式内容会被丢弃：只有当流的 `Done` 事件到达时才会被推入消息缓冲区，因此流式
过程中取消不会写入任何部分 assistant 消息。

Trace 消费方必须容忍 provider 提前退出时留下的 open turn。Provider failure 和
provider-stream cancellation 可能发出 `TurnStarted` 而没有匹配的 `TurnEnded`；
这些路径的终止边界是 `AgentEnd`、trace `RunEnded` 以及关联的诊断。

`Agent::abort`（以及 harness 的 `cancel` / `cancel_token` 辅助方法）会取消活跃 run
的 token；token 会在下一 turn 之前被重置，因此被取消的运行时会回到 idle 并接受新的
prompt。观察到自身 `CancellationToken` 的工具会立即返回——进程 adapter 工具在向 adapter
子进程尽力派发一条 `cancel` 消息后返回 `ToolError::Cancelled`——其结果会成为一个已
finalized 的错误工具结果，而非挂起。RPC abort、交互式 abort 与 shutdown 都归约为同一个
token 原语，因此可观察契约在嵌入方边界之间是一致的。

会话持久化对每条已 finalized 的 `AgentMessage::Llm` 条目进行 append-only 写入，而其
run 返回 `Err(AgentError::Cancelled)` 的 turn 根本不会被持久化，因此存储中永远不会
出现部分 assistant 消息或半应用的工具结果。

## 会话与压缩

会话存储使用 append-only JSONL：

- 第一行：`SessionHeader`。
- 条目：`MessageEntry`、`CompactionEntry`、`LeafEntry` 和 `ExtensionStateEntry`
  （`SessionEntry` 枚举为 `#[non_exhaustive]`；0.x 内可能新增附加变体）。
- Reader 恢复时会跳过损坏条目和末尾截断行。
- `session_branch::SessionTree` 根据 `parent_id` 链接和最新 `LeafEntry` 重建活跃分支。

压缩基础能力包括 threshold/manual/overflow 原因、
`CompactionEngine::should_compact`、`CompactionEngine::compact`，以及用于自定义摘要
生成的 `CompactionHooks`。`opi-coding-agent` 负责把这些基础能力连接到 CLI 会话
持久化。

## SDK 与 RPC 命令契约

`sdk`（`SDK_SCHEMA_VERSION = 3`，RPC 侧再导出为 `RPC_SCHEMA_VERSION`）定义了 RPC
JSONL 模式与嵌入方共享的不稳定 0.x 命令集合。每条命令携带可选的 `id`，并在其响应中
回显；RPC 对每条命令只输出一个 `response`，包含 `command`、`success`、可选的
`id`/`error`、可选的结构化 `error_code`（如 `unsupported_trace_request`），以及可选的
`data`。

结构化 `error_code` 只用于运行时契约失败：

| `error_code` | 含义 |
|---|---|
| `unsupported_trace_request` | 会话没有 trace sink 时请求了 `trace`。 |
| `agent_busy` | 已有 run 处于活跃状态，或运行中尝试执行运行时状态修改。 |
| `harness_unavailable` | RPC runner 没有附着 `CodingHarness`。 |
| `compaction_failed` | 手动压缩返回错误。 |
| `extension_command_not_handled` | 没有已注册扩展处理该命令。 |

`set_model` 和 `set_thinking_level` 的空闲态能力错误仍是自由文本验证失败，不携带
`error_code`。

命令状态契约（运行时守卫，而非解析层）：

| 命令 | 空闲时 | 运行中 |
|---|---|---|
| `prompt` / `continue` | 接受 → 启动一次运行；随后是异步事件 | 拒绝（`agent is already running; use steer or follow_up to queue messages`） |
| `abort` | 成功的空操作 | 取消活跃运行，成功 |
| `steer` | 进入 harness 队列 | 进入活跃 control handle 队列 |
| `follow_up` | 进入 harness 队列 | 进入活跃 control handle 队列 |
| `set_model` | 校验（同 provider、已知 model、重新校验 thinking） | 拒绝（`cannot change model while agent is running`） |
| `set_thinking_level` | 校验（`off|low|medium|high`、model 支持 / 预算） | 拒绝（`cannot change thinking level while agent is running`） |
| `compact` | 手动压缩（结果 + 诊断） | 拒绝（`cannot compact while agent is running`） |
| `session_info` | 返回 model / resources / session_id | 拒绝（`cannot query session info while agent is running`） |
| `extension_command` | 派发到注册表（data / `not handled` / error） | 拒绝（`cannot dispatch extension command while agent is running`） |
| `trace` | 返回版本化的脱敏信封，或 `unsupported_trace_request` | 允许（按运行的快照） |
| `quit` | 成功 + 关闭 | 成功 + 关闭（等待活跃运行清理完成） |

- 被拒绝的变更命令会被丢弃，绝不入队或部分应用：运行中的
  `set_model` / `set_thinking_level` / `compact` 不会改动正在运行的轮次或其配置。
- 只有 `steer` 和 `follow_up` 会在运行中入队；`steer` 在下一次 provider 请求前投递，
  `follow_up` 在 agent 本应停止时投递。
- 格式错误或未知的命令以结构化的 `parse` 响应失败，而不是被静默丢弃。
- 运行中 `abort` 与交互式 abort、关闭共享同一可观测的取消语义（见“取消”）。

## SDK、扩展、诊断与 Proxy

- `sdk` 定义 RPC JSONL 模式和嵌入方共享的带 schema version 的命令/响应类型。
  `SDK_SCHEMA_VERSION` 是 `3`。
- `extension` 提供 `Extension` 和 `ExtensionRegistry`，支持生命周期 hooks、自定义
  工具、自定义命令、事件观察器、扩展状态、自定义 Provider 和模型覆盖。
- `diagnostic` 和 `diagnostic_sink` 提供类型化诊断，以及面向公共 JSON/text 边界的
  脱敏辅助。
- `trace` 在调用方显式启用时保存最新运行的本地、已脱敏 trace envelope。
- `streaming_proxy` 可在任意 `BufRead`/`Write` 传输上转发 JSONL 命令/事件，输出
  `proxy_ready` header，提供事件缓冲、取消，并默认脱敏常见密钥模式。

所有 SDK/RPC/proxy 表面都是不稳定的 0.x API。客户端应检查 schema version，并在
需要时固定精确 crate 版本。

## API 表面分类

`opi-agent` 是 0.x crate。公共项分为三档：

| 档位 | 含义 |
|---|---|
| 支持的 0.x | 已文档化且经契约测试；在 0.x 内仍可能变动，并附带 changelog 条目。 |
| 不稳定内部 | 仅因 crate 布局需要而公开；文档告诫消费者固定版本。 |
| 候选移除 | 在更强的 API 承诺之前应隐藏、迁移或移除。 |

| 表面 | 档位 | 说明 |
|---|---|---|
| `Agent` | 支持的 0.x | 对主循环的有状态封装；经契约测试。 |
| `agent_loop` | 支持的 0.x | 核心异步入口；运行时事件顺序契约已测试。 |
| `AgentHooks` | 支持的 0.x | 六个生命周期 hooks；hook 顺序与失败契约已测试。 |
| `AgentLoopConfig`、`AgentLoopContext`、`AgentError`、`AgentMessage` | 支持的 0.x | 受支持的底层 `agent_loop` 入口所需的类型。 |
| `Tool`、`ToolDef`、`ToolResult`、`ToolError`、`ExecutionMode` | 支持的 0.x | JSON-Schema 工具契约，以及嵌入方使用的结果、错误和调度类型。 |
| `AgentEvent`、`AgentEventSink` | 支持的 0.x | 进程内运行时事件流；`AgentEvent` 是 `#[non_exhaustive]`，因为 0.x 内可能新增变体。 |
| `AgentSessionEvent` | 不稳定内部 | `opi --json` 线协议（`NDJSON_SCHEMA_VERSION = 2`，由 `opi-coding-agent` 拥有）；`#[non_exhaustive]`。请检查 schema 版本。 |
| `SessionEntry` | 不稳定内部 | 会话 JSONL 存储布局；位于 `session::SessionEntry`，未在 crate root 重新导出；`#[non_exhaustive]`。 |
| `Extension`、`ExtensionCommand`、`ExtensionError`、`ExtensionHookResult`、`ExtensionRegistry` | 不稳定内部 | 扩展生命周期与组合表面；`extension` 模块标注为 `# Unstable`。 |
| `SdkCommand`、`SdkResponse`、`SDK_SCHEMA_VERSION` | 不稳定内部 | RPC/SDK 命令模型（`SDK_SCHEMA_VERSION = 3`）；`sdk` 模块标注为不稳定 0.x。 |
| `StreamingProxy`、`ProxyConfig`、`ProxyEvent`、`ProxyHandler`、`SecretRedactor`、`StreamingProxyError` | 不稳定内部 | streaming-proxy 原语；`streaming_proxy` 模块标注为不稳定 0.x。 |
| `Diagnostic`、`DiagnosticPayload`、`RedactionMode`、`Severity`、`redact`、`redact_text`、`DiagnosticSink`、`NullSink`、`RecordingSink` | 不稳定内部 | 运行时表面使用的诊断 payload 与 sink plumbing；当前契约是 redaction/schema-version 行为，不是稳定 API 形状。 |
| `FileTraceSink`、`RecordingTraceSink`、`TRACE_SCHEMA_VERSION`、`TraceCollector`、`TraceError`、`TraceKind`、`TraceRecord`、`TraceSink` | 不稳定内部 | 本地 trace envelope plumbing；`trace` 模块标注为不稳定 0.x，并携带 `TRACE_SCHEMA_VERSION = 1`。 |
| `AgentState` | 不稳定内部 | 为 crate 布局与 harness 集成暴露的运行时状态持有器；不是受支持的嵌入方契约。 |
| `AgentHarness`、`Phase`、`HarnessError`、`HarnessResult`、`HarnessSnapshot`、`HarnessSession`、`HarnessRuntimeConfig`、`HarnessRuntimeConfigBuilder`、`SavePoint`、`PendingWriteQueue`、`PendingWrite`、`PendingWriteKind`、`SessionRepo`、`SessionFacade`、`JsonlHarnessSession`、`JsonlSessionRepo` | 不稳定内部 | 位于 `Agent` 之上的通用 agent-harness/session-facade 编排 seam（Phase 10，Workstream 10.2/10.3）；经契约测试，但尚未自行驱动主循环。`harness` 模块标注为不稳定 0.x。 |

本次审查没有发现候选移除的 crate-root re-export。`src/lib.rs` 中的每个
crate-root `pub use` 都已在上表点名。公共模块可能还会通过模块路径暴露其他项；
除非这些项在这里被点名为支持的 0.x 表面，否则它们都属于不稳定内部 0.x API。

不会给出稳定 1.0 API 承诺。当前稳定性由 `AgentEvent`、`AgentSessionEvent`、
`SessionEntry` 及 trace/hook 结果枚举上的 `#[non_exhaustive]`，以及 `sdk`、
`streaming_proxy`、`extension` 和 `trace` 模块级的 `# Unstable` / 不稳定 0.x 说明来
约束。没有 `#[doc(hidden)]` 或 `#[unstable]` feature gate，因此嵌入方应固定精确
crate 版本。本地 trace envelope 携带 `TRACE_SCHEMA_VERSION = 1`。

## 非目标（Non-Goals）

运行时自 `0.5.4` 起已稳定；crate 维持 0.x，Phase 10 的 `harness` seam 仅为内部使用。以下明确不在范围内，不作声明：

- 不声明稳定 1.0 公共 API 承诺（表面保持 0.x）。
- 不得引入 TypeScript 扩展 API 兼容。
- 不得引入 package 生态扩张或 package 市场。
- 除 `process-jsonl`（`opi-extension-jsonl-v1`）外不得引入新 adapter 类型。
- 不得添加 Web UI 产品工作。
- 不得添加供应商 OAuth 登录工作。
- 不得添加内核 plan mode、sub-agent、todo、权限弹窗或 MCP 运行时。
- 不得引入共享 `opi-types` crate。
- 不得在 crate 之间无理由地迁移公共类型。
- 除非契约测试证明当前形状无法满足所需行为，否则不得重写整个 agent loop。

## 公共模块

`agent`、`compaction`、`diagnostic`、`diagnostic_sink`、`event`、`extension`、
`harness`、`hooks`、`loop_types`、`message`、`sdk`、`session`、`session_branch`、
`session_event`、`state`、`streaming_proxy`、`tool`、`trace` 和 `validation`。

crate root 重新导出了常用运行时类型，包括 `Agent`、`Tool`、`ToolResult`、
`ToolError`、`ExecutionMode`、`AgentHooks`、`AgentEvent`、`AgentSessionEvent`、
`AgentLoopConfig`、`SdkCommand`、`SdkResponse` 和 `ToolDef`。

## 测试支持

确定性主循环测试可使用 `opi_ai::test_support::MockProvider` 搭配自定义 `Tool`
实现。涉及会话存储的测试应使用隔离临时目录。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。
