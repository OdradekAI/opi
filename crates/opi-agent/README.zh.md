# opi-agent

[![Crates.io](https://img.shields.io/crates/v/opi-agent.svg)](https://crates.io/crates/opi-agent)
[![Docs.rs](https://docs.rs/opi-agent/badge.svg)](https://docs.rs/opi-agent)

> [opi](https://github.com/OdradekAI/opi) 使用的 Provider 无关 Agent 运行时。

[English](README.md) | [opi workspace](../../README.zh.md)

Rust 的 Provider 无关 Agent 运行时：Agent 主循环、JSON-Schema 工具契约、生命周期
hooks、会话存储与压缩、SDK/RPC 类型、扩展，以及 streaming proxy——`opi-coding-agent`
构建于其上的引擎。

```sh
cargo add opi-agent
```

## 当前状态

当前 crate 版本是 `0.8.2`，继承自 workspace 包版本。

`opi-agent` 负责 Agent 主循环和运行时基础能力：工具契约、JSON Schema 参数校验、
并行/串行工具执行、生命周期 hooks、事件输出、steering/follow-up 队列、会话
JSONL 存储、分支重建、上下文压缩、SDK/RPC 类型、扩展、本地诊断、已脱敏证据
记录，以及 streaming proxy。

运行时契约保持聚焦于主循环本身，而非新增工作流。工具结果携带 `truncated` 和工具自有
结构化诊断；主循环把这些诊断提升为共享 diagnostic/evidence 系统，并在公共
`ToolExecutionEnd` 事件上暴露，同时让面向 provider 的工具结果消息只携带 LLM 可见内容
和失败状态。来自 `opi-ai` 的 `ProviderErrorCategory` 会映射为已脱敏 diagnostics 和
evidence 记录；provider 返回的 cancellation 表现为 `AgentError::Cancelled`；retry diagnostics
会区分重试预算耗尽与部分 provider 输出后的重试抑制。append-only 会话存储写入携带精确
运行时输入绑定的版本 2 头部；旧版 1 输入保持只读。metadata 条目不进入 provider 上下文，
但分支摘要会通过 `session_context::reconstruct_context` 进入。

它依赖 `opi-ai` 的 Provider 和消息类型。它不实现 `opi` CLI、终端 UI 或具体的
文件/ shell 内置工具；这些能力分别位于 `opi-coding-agent` 和 `opi-tui`。

## 鉴权与会话亲和边界

`opi-agent` 不执行凭据 IO，也不构造 OAuth provider。它会在真实主循环中保留类型化鉴权
结果：不可重试的 `ProviderError::CredentialNeeded`、
`ProviderError::CredentialRevoked` 与 `ProviderError::AccountIdMissing { provider_id }`
无需字符串匹配即可映射为对应 `AgentError` 变体和已脱敏诊断。缺少 account id 与凭据撤销
是不同情况，产品层使用规范 `/login <provider>` 修复。outer 交互产品只能在同一 provider
的显式登录成功后，对一个输出前的待处理轮次精确重试一次；非交互产品绝不提示，撤销凭据也绝不会触发自动重新登录。
`opi-agent` 只保留类型化
结果与现有 message buffer；coding-agent outer TUI 拥有登录和重试策略。

Agent 还把不透明 `session_id` 从 `Agent` 经 `AgentLoopContext` 携带到每个 Provider
`Request`。它不拥有持久化或 Provider 专用 header 映射：`opi-coding-agent` 提供活跃会话
id，`opi-ai` adapter 决定审查过的 cache-affinity 映射是否消费该值。

## 核心抽象

| 项 | 作用 |
|----|------|
| `Agent` | 对主循环的有状态封装，提供 prompt、continue、abort、subscribe、steering、follow-up 和完整状态的原子替换；可信 `RegisteredTool` 与 `ToolAuthorizer` 在构造时提供，公共 run 操作返回 `AgentRunResult`。 |
| `AgentRunResult` / `AgentLoopResult` | 带 `must_use` 的类型化结果；即使失败也保留实际 state/messages、owning error（如有）、终止结果和 evidence health，而不会丢失生命周期事实。 |
| `Tool` | 基于 JSON Schema 的工具契约，支持取消和可选进度更新。 |
| `RegisteredTool` / `ToolRegistry` / `ToolAuthorizer` | 不可变可信工具注册，以及每次调用都必须经过的 authority 边界。 |
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
    ProviderCollection::prepare_call(Request)
      validate route/request model schema and capabilities
                                         # hook 转换已经完成
      PreparedProviderCall::start_attempt
      message_start                       # assistant 流 Start
      message_update                      # 每个文本/思考 delta
      message_end                         # 完整的 assistant 消息
      若存在 tool call：
        tool_execution_start              # 每个 tool call
        resolve trusted registration
        before_tool_call                  # 可阻止；绝不授予 authority
        validate tool args (jsonschema)
        authorize tool call               # 必须经过可信 ToolAuthorizer
        tool.execute                      # 并行批次；若任一工具为 Sequential 则整批串行
        after_tool_call                   # AgentHooks::after_tool_call（可替换结果）
        tool_execution_end                # 每个 tool call
        turn_end                          # assistant 消息 + tool_results
      否则：
        turn_end                          # assistant 消息，无 tool_results
    prepare_next_turn                     # hook 可返回完整候选状态
    validate candidate as one unit
    apply candidate atomically            # 先于停止判断
    should_stop_after_turn                # 观察已应用状态；terminate 强制为 true
                                         # true -> AgentEnd, 成功的 AgentLoopResult
    drain steering queue                  # 非空 -> QueueUpdate，追加，进入下一 turn
    若已应用候选状态 -> 进入下一 turn
    若无待处理工具：
      pop follow-up queue                 # 非空 -> QueueUpdate，追加，进入下一 turn
      否则 -> 停止
agent_end                                 # 仅一次，终止时
```

边界：

- `transform_context` 与 `convert_to_llm` 先完成，随后
  `ProviderCollection::prepare_call` 才针对已解析模型的 schema、wire 和能力校验
  转换后的请求。
- `prepare_next_turn` 在 live state 之外构造完整候选状态。主循环先把候选作为整体
  校验并原子应用，再执行 `should_stop_after_turn`；取消或校验失败会保留（或恢复）
  先前状态。
- `should_stop_after_turn` 在 `turn_end` 和候选应用之后、任何队列轮询之前观察已应用
  状态。终止停止会阻止 steering/follow-up 轮询；否则，已应用的 prepared candidate
  会在弹出 follow-up 前获得自己的下一次 provider turn。
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
| `before_tool_call` | 在解析可信工具之后、JSON Schema 参数校验与强制 authorization 之前运行；可 `Deny` 阻止执行（拒绝原因成为工具错误）。`Continue` 不是授权。 |
| `after_tool_call` | 在执行之后、最终的 `ToolExecutionEnd` 事件之前运行；可 `Replace` 展示结果，使替换值被发出并持久化，但工具 evidence 仍保留原始的底层执行结果。 |
| `prepare_next_turn` | 在 `turn_end` 之后运行；可返回完整候选状态，该状态会在停止判断前被校验并原子应用。 |
| `should_stop_after_turn` | 在候选准备/应用之后、steering/follow-up 轮询之前运行；返回 `true` 会在下一 turn 之前停止。 |

扩展组合：`ExtensionRegistry::wrap_hooks` 先运行基础 `AgentHooks` 方法，再按注册顺序依次运行每个扩展。
扩展的 `on_before_tool_call` 返回 `Block` 会在首个 block 处中断链路；后续扩展不会被调用。
扩展的 `on_after_tool_call` 观察者不能修改结果；只有基础 hook 可以 `Replace`。

当 adapter 或扩展只实现了部分 hook 时，其余 hook 方法默认为 no-op。

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

对于已解析的工具，`before_tool_call` 先运行；随后参数校验会在强制 authorization 与
`Tool::execute` 之前执行。校验失败是正常的运行时结果，而非循环错误：会持久化一个
错误 `ToolResult`（`is_error = true`、`terminate = false`）并继续运行。此时 hook
已经运行，但 authorizer 与工具不会运行；未知工具则会在 hook 之前被拒绝。

## 工具结果与诊断

`ToolResult` 是内置工具、自定义工具和扩展工具共享的运行时结果契约：

| 字段 | 含义 |
|---|---|
| `content` | LLM 可见的文本或图片输出。 |
| `details` | 面向运行时、UI、JSON/RPC 和 evidence 边界的可选结构化元数据。 |
| `is_error` | 该结果是否表示工具失败。 |
| `terminate` | 当同一批次中的每个结果也终止时，该结果是否可以结束运行。 |
| `truncated` | 输出是否被缩短或受界限限制。 |
| `diagnostics` | 工具自有结构化原因记录（`code`、`message`、`context`）。 |

Agent 主循环会在 `after_tool_call` 之后读取 diagnostics，因此替换后的结果也能替换
诊断上下文。每个 `ToolDiagnostic` 会提升为共享 `Diagnostic` 和 diagnostic-linked
evidence 记录。公共事件在发出前会脱敏；provider 请求只通过
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
`AgentEnd` 事件，并返回 error 为 `Some(AgentError::Cancelled)` 的
`AgentLoopResult`。in-flight assistant 消息累积的
部分流式内容会被丢弃：只有当流的 `Done` 事件到达时才会被推入消息缓冲区，因此流式
过程中取消不会写入任何部分 assistant 消息。

消费方必须容忍 provider 提前退出时留下的 open turn。Provider failure 和
provider-stream cancellation 可能发出 `TurnStarted` 而没有匹配的 `TurnEnded`；
这些路径的终止边界是 `AgentEnd` 以及关联的诊断。

`Agent::abort` 会取消活跃 run 的 token；token 会在下一次操作之前被重置，因此被取消的
运行时会回到 idle 并接受新的 prompt。需要等待产品 preflight 的调用方可使用公共但
doc-hidden 的 `Agent::arm_run`/`ArmedAgentRun` 及配套 `*_armed` 操作：这个不透明值把
取消绑定到恰好一个 Agent 的最新 generation，foreign 或 stale 值会产生类型化
`AgentError::InvalidArmedRun`。观察到自身 `CancellationToken` 的工具会立即返回——进程 adapter 工具在向 adapter
子进程尽力派发一条 `cancel` 消息后返回 `ToolError::Cancelled`——其结果会成为一个已
finalized 的错误工具结果，而非挂起。RPC abort、交互式 abort 与 shutdown 都归约为同一个
token 原语，因此可观察契约在嵌入方边界之间是一致的。

会话持久化对每条已 finalized 的 `AgentMessage::Llm` 条目进行 append-only 写入，而其
类型化 run 结果携带 `AgentError::Cancelled` 的 turn 根本不会被持久化，因此存储中永远不会
出现部分 assistant 消息或半应用的工具结果。

公共 `Agent` run 操作返回带 `must_use` 的 `AgentRunResult`，底层 `agent_loop` 返回
`AgentLoopResult`。二者都暴露保留的 state/messages、owning error、`TerminalOutcome`
与最终 `EvidenceHealth`；`into_execution_result` 是显式兼容转换。`AgentRunResult`
还拥有 loop 后生命周期：报告 `AgentRunLifecyclePhase`，把 `begin_compaction` 与
`finish_compaction` 配对，并通过 `finalize_evidence` 或 `abandon_evidence` 结束证据。

## 会话与压缩

会话存储使用 append-only JSONL：

- 当前写入器生成 `version = 2` 的 `SessionHeader`，其中含必需的精确
  `RuntimeInputBinding`。每个条目都是分类为 `required` 或
  `ignorable_observation` 的 envelope。
- 会话条目包括 `MessageEntry`、`CompactionEntry`、`LeafEntry`、
  `ExtensionStateEntry`、`SessionInfoEntry`（`session_info`）、
  `ModelChangeEntry`（`model_change`）、`ThinkingLevelChangeEntry`
  （`thinking_level_change`）、`LabelEntry`（`label`）和
  `BranchSummaryEntry`（`branch_summary`）。元数据条目不推进内容 tip，也不进入
  provider 上下文；`branch_summary` 由 `session_context::reconstruct_context`
  作为 metadata-parented 消息注入重建的 LLM 上下文。
- 未知的 required 版本 2 envelope、损坏输入和不支持的版本都会 fail closed。未知的
  ignorable-observation envelope 会被跳过；Reader recovery 会记录损坏的中间条目，
  并跳过末尾截断行。
- 版本 1 reader 是历史兼容且只读：绝不改写源字节。Reference Product 仅在能够唯一证明
  路由规范化后才可 resume 或 fork v1 会话，并会在执行前创建并采用一个携带当前精确绑定的
  parented 版本 2 child。缺失或歧义路由会在 provider 或 tool dispatch 前失败。
- `session_branch::SessionTree` 根据 `parent_id` 链接和最新 `LeafEntry` 重建活跃分支。

压缩基础能力包括 threshold/manual/overflow 原因、
`CompactionEngine::should_compact`、`CompactionEngine::compact`，以及用于自定义摘要
生成的 `CompactionHooks`。`opi-coding-agent` 负责把这些基础能力连接到 CLI 会话
持久化。

## SDK 与 RPC 命令契约

`sdk`（`SDK_SCHEMA_VERSION = 3`）定义了 RPC JSONL 模式与嵌入方共享的不稳定 0.x 命令
集合。每条命令携带可选的 `id`，并在其响应中
回显；RPC 对每条命令只输出一个 `response`，包含 `command`、`success`、可选的
`id`/`error`、可选的结构化 `error_code`（如 `unsupported_trace_request`），以及可选的
`data`。

结构化 `error_code` 只用于运行时契约失败：

| `error_code` | 含义 |
|---|---|
| `unsupported_trace_request` | 运行没有 evidence recorder 时请求了 `trace`。 |
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
| `trace` | 返回运行的 evidence records，或 `unsupported_trace_request` | 允许（按运行的快照） |
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
- `evidence` 提供存储无关的证据 sink、健康状态、身份标识，以及在调用方显式启用时
  为最新运行生成 resolved-execution manifest。
- `streaming_proxy` 可在任意 `BufRead`/`Write` 传输上转发 JSONL 命令/事件，输出
  `proxy_ready` header，提供事件缓冲、取消，并默认脱敏常见密钥模式。

Evidence 生命周期是显式的：`EvidenceSink::setup` 先于有序 `emit`，artifact 经过
`finalize_artifact`，只有已校验且不可变的 `FinalizedManifest` 才能进入
`finalize_run`。无法发布的 run 通过 `abandon_run` 关闭；未完成生命周期时，
`AgentRunResult` 会失败关闭地调用该路径。Evidence 与 manifest facts 均为类型化
（包括 route/auth provenance、tool authorization/outcome、session binding、measurement
与 terminal outcome），而 `AssemblyIdentity`、`CapabilityIdentity` 等 product/embedder
identity 保持为经校验的不透明字符串。`RunId` 是不透明 UUIDv7，并按规范带连字符字符串
序列化/解析；run-local turn/call/sequence identity 不暴露构造器。

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
| `AgentRunResult`、`AgentLoopResult`、`AgentRunLifecyclePhase`、`PendingCompaction` | 支持的 0.x | 带 `must_use` 的类型化执行/生命周期结果；调用方在显式转换或 finalization 前检查实际 outcome 与 evidence health。 |
| `AgentHooks` | 支持的 0.x | 六个生命周期 hooks；hook 顺序与失败契约已测试。 |
| `AgentLoopConfig`、`AgentLoopContext`、`AgentError`、`AgentMessage` | 支持的 0.x | 受支持的底层 `agent_loop` 入口所需的类型。 |
| `Tool`、`ToolDef`、`ToolResult`、`ToolError`、`ExecutionMode` | 支持的 0.x | JSON-Schema 工具契约，以及嵌入方使用的结果、错误和调度类型。 |
| `RegisteredTool`、`ToolRegistry`、`ToolAuthorizer`、`CapabilityIdentity` | 支持的 0.x | 产品中立的可信注册、经校验的不透明 capability identity、面向 provider 的投影，以及 fail-closed 的逐调用授权。产品策略和内置 capability 常量由产品/embedder 提供，不属于 Agent Core。 |
| `AgentEvent`、`AgentEventSink` | 支持的 0.x | 进程内运行时事件流；`AgentEvent` 是 `#[non_exhaustive]`，因为 0.x 内可能新增变体。 |
| `AgentSessionEvent` | 不稳定内部 | `opi --json` 线协议（`NDJSON_SCHEMA_VERSION = 2`，由 `opi-coding-agent` 拥有）；`#[non_exhaustive]`。请检查 schema 版本。 |
| `SessionEntry` | 不稳定内部 | 会话 JSONL 存储布局；位于 `session::SessionEntry`，未在 crate root 重新导出；`#[non_exhaustive]`。 |
| `Extension`、`ExtensionCommand`、`ExtensionError`、`ExtensionHookResult`、`ExtensionRegistry` | 不稳定内部 | 扩展生命周期与组合表面；`extension` 模块标注为 `# Unstable`。 |
| `SdkCommand`、`SdkResponse`、`SDK_SCHEMA_VERSION` | 不稳定内部 | RPC/SDK 命令模型（`SDK_SCHEMA_VERSION = 3`）；`sdk` 模块标注为不稳定 0.x。 |
| `StreamingProxy`、`ProxyConfig`、`ProxyEvent`、`ProxyHandler`、`SecretRedactor`、`StreamingProxyError` | 不稳定内部 | streaming-proxy 原语；`streaming_proxy` 模块标注为不稳定 0.x。 |
| `Diagnostic`、`DiagnosticPayload`、`RedactionMode`、`Severity`、`redact`、`redact_text`、`DiagnosticSink`、`NullSink`、`RecordingSink` | 不稳定内部 | 运行时表面使用的诊断 payload 与 sink plumbing；当前契约是 redaction/schema-version 行为，不是稳定 API 形状。 |
| `EvidenceSink`、`EvidenceRecorder`、`InMemoryEvidenceSink`、`NoopEvidenceSink`、`EvidenceRecord`、`ManifestCandidate`、`FinalizedManifest`、`RuntimeInputBinding`、`EvidenceHealth`、`IdentityAllocator`、`RunId` | 不稳定内部 | 产品中立的类型化 evidence 契约：存储中立的 finalize/abandon 生命周期、经校验的 resolved-execution facts、不透明 identity 与规范 UUIDv7 run identity。`evidence` 模块标注为不稳定 0.x。 |
| `agent::ArmedAgentRun` 及 doc-hidden `Agent::{arm_run,control_handle_for_run,*_armed}` | 不稳定内部 | 产品 preflight 使用的单次操作 cancellation generation；stale/foreign generation 返回 `AgentError::InvalidArmedRun`。 |
| `HarnessError`、`HarnessResult`、`SavePoint`、`PendingWriteQueue`、`PendingWrite`、`PendingWriteKind`、`SessionRepo`、`SessionFacade`、`JsonlSessionRepo` | 不稳定内部 | 通用 session-facade/repo 编排 seam；`harness` 模块标注为不稳定 0.x。 |

上表列出了 `src/lib.rs` 中每个受支持的 crate-root `pub use`。公共模块可能还会通过
模块路径暴露其他项；除非这些项在这里被点名为支持的 0.x 表面，否则它们都属于
不稳定内部 0.x API。

不会给出稳定 1.0 API 承诺。当前稳定性由 `AgentEvent`、`AgentSessionEvent`、
`SessionEntry` 及 evidence/hook 结果枚举上的 `#[non_exhaustive]`，以及 `sdk`、
`streaming_proxy`、`extension` 和 `evidence` 模块级的 `# Unstable` / 不稳定 0.x 说明来
约束。上表列出的 preflight 与 armed-run 方法有意标记为 `#[doc(hidden)]` 内部表面；
该属性只控制文档可见性，并不保证 API 稳定。不存在由编译器强制执行的
`#[unstable]` gate，因此嵌入方只应依赖上表标明的受支持 0.x 分类，并固定精确
crate 版本。evidence sink 生命周期是捕获契约。

Agent 状态以一个经校验的完整 `NextTurnState` 原子替换。嵌入方在构造时提供不可变
`RegisteredTool` 与 `ToolAuthorizer`。

## 非目标（Non-Goals）

crate 维持 0.x，`harness` seam 仅为内部使用。以下明确不在范围内，不作声明：

- 不声明稳定 1.0 公共 API 承诺（表面保持 0.x）。
- 不得引入 TypeScript 扩展 API 兼容。
- 不得引入 package 生态扩张或 package 市场。
- `opi-agent` 不提供 `process-jsonl`（`opi-extension-jsonl-v1`）之外的 extension
  adapter 类型；`command.execute` 是独立的 `opi-coding-agent` / `opi-protocol` 表面。
- 不得添加 Web UI 产品工作。
- `opi-agent` 不得实现凭据 store、登录 presenter 或供应商 OAuth flow；这些仍由
  `opi-ai` 契约与 `opi-coding-agent` 产品接线拥有。
- 不得添加内核 plan mode、sub-agent、todo、权限弹窗或 MCP 运行时。
- 不得引入共享 `opi-types` crate。
- 不得在 crate 之间无理由地迁移公共类型。
- 除非契约测试证明当前形状无法满足所需行为，否则不得重写整个 agent loop。

## 公共模块

`agent`、`authority`、`compaction`、`diagnostic`、`diagnostic_sink`、`event`、`extension`、
`harness`、`hooks`、`loop_types`、`message`、`sdk`、`session`、`session_branch`、
`session_context`、`session_event`、`streaming_proxy`、`tool`、
`evidence` 和 `validation`。

crate root 重新导出了常用运行时类型，包括 `Agent`、`Tool`、`ToolResult`、
`ToolError`、`ExecutionMode`、`AgentHooks`、`AgentEvent`、`AgentSessionEvent`、
`AgentLoopConfig`、`SdkCommand`、`SdkResponse` 和 `ToolDef`。

## 测试支持

确定性主循环测试可使用 `opi_ai::test_support::MockProvider` 搭配自定义 `Tool`
实现。涉及会话存储的测试应使用隔离临时目录。

## 许可证

MIT。详见 workspace [LICENSE](../../LICENSE)。
