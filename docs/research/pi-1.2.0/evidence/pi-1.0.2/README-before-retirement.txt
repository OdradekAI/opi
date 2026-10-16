# Pi 1.0.2 整体源码分析报告

> 对象：本地 `.repo/pi-1.0.2` 源码快照。版本升级、源码及文档时效复核日期：2026-10-05。
> 范围：仓库总体结构、13 个一级包、默认产品与实验路径、状态及错误边界、构建发布和测试体系，以及设计文档、后续方向与已被替代的历史方案。
> 方法：只读静态分析，交叉检查实现、manifest、入口、测试及随仓库文档。未安装依赖、运行项目、构建、执行项目测试、调用模型或连接外部服务。报告以中文新建，不修改或替代既有双语文档。
> 本文和子报告只分析 Pi 自身，不包含对其他项目的映射、迁移方案或实现建议。

文档与规划的集中结论见 [设计文档与规划时效核查](documentation-audit.md)；125 份本地文档的逐文件清单和审读口径见 [文档索引](documentation-index.md)。13 份包报告均包含实现分析、规划时效及本次版本核查。新增 [1.0.0 → 1.0.2 升级分析](version-changes.md)，汇总变化、未变化项及逐文件差异。

本目录自2026-10-07起作为历史报告保留；当前 Pi 源码分析基准为 [Pi 1.0.4](../pi-1.0.4/README.md)。下文维持1.0.2的分析对象、结论和原核查日期，不因导航更新而视为重新核验。1.0.2源码当前不在工作区，原源码链接需恢复相同快照后使用；旧清单、审读范围及更早报告恢复位置仍见 [历史证据](evidence/pi-1.0.0/README.md)。

## 1 核心结论

相对 1.0.0，本版增加按 thinking level 配置采样参数、durable 持久 provider session 身份、MCP 项目覆盖与 CIMD、独立工具 renderer、codemode 输出累计限制，以及终端图片修正。交付层删除 npm-shrinkwrap，新增 Nix 构建和模型 catalog pin 流程；默认与实验运行时的结构分工保持。具体边界和全部变化文件见 [升级分析](version-changes.md)。

Pi 的源码不只是一个“向模型发请求并执行 shell”的 CLI，而是若干可独立使用的库与一个组装它们的终端应用。理解它最有效的切入点是状态和生命周期归属：谁持有消息，谁决定下一轮，谁写入存储，谁拥有工具进程，谁负责把结果展示给用户。

默认产品的主链为：

```text
pi CLI / 嵌入 SDK
  → coding-agent 的 AgentSession
  → agent 的 Agent 与 agent-loop
  → ai 的模型目录、认证、Provider API 与流式消息
  → coding-agent 的工具、扩展、会话持久化与模式层展示
```

默认 SessionManager 保存 JSONL 会话树，Agent 在内存中执行 turn/tool loop。另有实验性服务链：

```text
experimental CLI / client
  → client + protocol + server
  → Session worker + Chord services/facets
  → durable Harness + SQLite + 持久化任务
  → ai
```

第二条链不是第一条链更换一个传输协议之后的同一个运行时。两者的状态模型、完成语义、工具执行实现、持久化格式与入口条件不同。把所有目录连成一张没有条件的“整体架构图”，会掩盖这个版本最重要的事实。[默认 CLI 入口](../../../.repo/pi-1.0.2/packages/coding-agent/src/cli.ts#L1)、[SDK 创建入口](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/sdk.ts#L175)、[实验 Session worker](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/session-worker.ts#L784)。

另外有四组容易混淆的边界：

| 容易混淆的概念 | 源码实际区分 |
|---|---|
| Provider 与 API | Provider 包含模型、认证、发现等策略；API 负责具体请求与响应格式，多家 Provider 可以共用一种 API |
| default RPC 与 experimental protocol | 前者是 stdin/stdout JSONL 命令事件；后者是 CBOR frame 中的 Session/Chord service 消息 |
| extension、facet、skill | 默认扩展是同进程可执行代码；Chord facet 是服务装配单元；skill 是被发现和加载的指令资源 |
| VM 隔离与工具权限 | codemode 隔离生成的 JavaScript；宿主工具仍具有宿主授予的文件、进程和网络能力 |

这四组边界分别见 [ai](packages/ai.md)、[coding-agent](packages/coding-agent.md)、[chord](packages/chord.md)、[codemode](packages/codemode.md) 子报告。

## 2 快照身份、覆盖口径和阅读方式

当前目录下 13 个一级包的 manifest 版本都是 `1.0.2`；根 workspace 自身是 private，版本为 `0.0.3`。本报告以这些本地文件为事实依据，未把目录名当成已经验证过的上游 Git tag，也未将外部网站最新文档混入版本结论。[根 manifest](../../../.repo/pi-1.0.2/package.json)。

快照清单收录 **1,859 个文件**，排除名为 `.git`、`node_modules`、`dist` 的目录。文件级 SHA-256 和整个清单的确定性摘要保存在 [source-manifest.json](source-manifest.json)：

```text
f0309990522b4c47b96e19ec684c53fd1c8fd8a5bc7918ba70ec3289bf3f13fb
```

摘要算法是对按 Unicode 码点排序的 POSIX 相对路径，拼接 `path + NUL + file_sha256_hex + LF` 后计算 SHA-256。这用于定位本报告针对的内容，不证明供应方身份、发布包签名或来源真实性。

覆盖方式分为四层：

1. **全包枚举**：manifest、文件规模、源码入口和内部依赖覆盖所有一级包。
2. **关键实现追踪**：逐链分析请求、工具执行、会话、存储、取消、恢复、协议、UI 和扩展的关键函数及状态转换。
3. **契约交叉检查**：用测试与文档核实边界，区分“已有测试断言”和“本次实际执行过的验证”。大包中未逐分支审计的安装器、供应商适配器和平台代码，在子报告说明。
4. **文档与演进对照**：清点本地文档，分离设计规划、当前契约、历史记录、示例和测试夹具；对规划条目比较原始理由、当前实现与替代记录，不把旧的“将实现”描述直接视为未来工作。审读深度与结论置信边界单独记录在文档索引和时效核查中。

每个子报告可独立阅读，包含定位、结构/API、关键流程、状态及失败行为、测试、取舍和阅读入口。源码引用使用本地相对链接及 `#L` 行号；若 Markdown 阅读器不支持行号跳转，链接标签或上下文仍可用于定位。文件枚举见 [source-index.md](source-index.md)，列出一个文件不代表逐行审计过它。


本版建立在旧版分析和摘要清单之上：1,705 个文件字节一致，138 个修改、16 个新增、5 个删除。旧源码目录已不在工作区，因此变化识别来自路径与 SHA-256 比对，语义判断来自新源码、测试和发行记录；不声称已完成旧源码的逐行 diff。对变化文件重新引用当前实现，对一致部分继承旧版阅读证据。[升级核查](version-changes.md) 给出具体边界。

## 3 全部包与规模

| 一级目录 / 子报告 | npm 包名 | 主要职责 | src TS 文件 / 行数 | test/spec 文件 / 行数 |
|---|---|---|---:|---:|
| [ai](packages/ai.md) | `@earendil-works/pi-ai` | 模型、认证、Provider API、消息流、目录缓存 | 193 / 26,341 | 164 / 41,376 |
| [agent](packages/agent.md) | `@earendil-works/pi-agent-core` | 通用内存 Agent、工具轮次、队列与事件 | 6 / 2,513 | 4 / 3,943 |
| [coding-agent](packages/coding-agent.md) | `@earendil-works/pi-coding-agent` | CLI/SDK、会话、工具、扩展、终端产品、实验宿主 | 298 / 84,990 | 311 / 66,745 |
| [tui](packages/tui.md) | `@earendil-works/pi-tui` | 输入、差分渲染、布局、编辑器、Markdown、图片 | 45 / 19,277 | 42 / 18,719 |
| [telemetry](packages/telemetry.md) | `@earendil-works/pi-telemetry` | 观测契约、schema、内存参考实现与 conformance | 6 / 935 | 2 / 243 |
| [codemode](packages/codemode.md) | `@earendil-works/pi-codemode` | Worker + QuickJS/WASI 执行和宿主工具桥接 | 10 / 1,678 | 3 / 1,021 |
| [mcp](packages/mcp.md) | `@earendil-works/pi-mcp` | MCP 客户端、stdio/HTTP/SSE、OAuth | 18 / 3,167 | 5 / 1,398 |
| [chord](packages/chord.md) | `@earendil-works/chord` | Facet 装配、服务、状态复制、delta、Context | 29 / 8,817 | 21 / 6,794 |
| [durable](packages/durable.md) | `@earendil-works/pi-durable` | 持久化 conversation、task、document 和恢复 | 61 / 17,716 | 43 / 23,796 |
| [protocol](packages/protocol.md) | `@earendil-works/pi-protocol` | Session wire envelope、校验与 CBOR framing | 8 / 869 | 3 / 567 |
| [client](packages/client.md) | `@earendil-works/pi-client` | 握手、请求关联、Session 绑定、远程服务源 | 8 / 1,135 | 3 / 715 |
| [server](packages/server.md) | `@earendil-works/pi-server` | Router、共享 Session handle、连接租约及 listener | 16 / 1,958 | 6 / 1,054 |
| [evals](packages/evals.md) | `@earendil-works/pi-evals` | private 文档效用评测、隔离执行和成对汇总 | 5 / 1,446 | 6 / 815 |

源码统计限定在各一级包直属的 `src/` 子树，包含实验目录，不计嵌套示例项目自己的 src。行数包含空行、注释、类型和生成代码，不是逻辑代码行，也不能直接比较包的复杂度。测试统计按整个包子树的文件名模式计算，不能推出用例数或覆盖率。

根 README 的包表只列出其中 7 个包，而 `packages/*` 实际有 13 个 manifest，所以不能仅按 README 的概览决定分析范围。除此之外，root workspaces 还显式包含多个 coding-agent 扩展示例，它们在本报告归入应用包的扩展示例与交付分析，不另当作一级产品包。[README 包表](../../../.repo/pi-1.0.2/README.md)、[workspace 配置](../../../.repo/pi-1.0.2/package.json)。

## 4 静态依赖图与实际运行图

以下实线只表示一级包 manifest 的 production dependencies，箭头由声明依赖者指向被依赖包。图不包含外部 npm 包或开发依赖，也不根据源码中值导入和类型导入的区别删改 manifest 声明的边。

```mermaid
flowchart TD
  coding["coding-agent"] --> agent["agent"]
  coding --> ai["ai"]
  coding --> tui["tui"]
  coding --> codemode["codemode"]
  coding --> mcp["mcp"]
  coding --> chord["chord"]
  agent --> ai
  ai --> telemetry["telemetry"]
  durable["durable"] --> ai
  durable --> chord
  protocol["protocol"] --> chord
  client["client"] --> protocol
  client --> chord
  server["server"] --> protocol
  server --> chord
  evals["evals：private，无内部 production dependency"]
```

具体边由 [快照清单中的 dependencies](source-manifest.json) 记录，可与各包 package.json 交叉核对。这里有三个必须补充的条件：

- `coding-agent` 对 `client/protocol/server` 的声明在 devDependencies；它们参与实验源码路径，不能据此宣称默认 CLI 请求都经过网络 server。
- 实验实现以 `@earendil-works/pi-durable` 及子路径导入 durable，源码 resolver 根据 root tsconfig 解析；应用包 manifest 没有 durable production dependency。实验目录被 build 排除，不能按默认发布物读取这条依赖。
- `evals` 通过 devDependencies 使用 ai/coding-agent，它本身 private，未进入根普通 build 的构建序列。

依据：[coding-agent manifest](../../../.repo/pi-1.0.2/packages/coding-agent/package.json)、[build 排除项](../../../.repo/pi-1.0.2/packages/coding-agent/tsconfig.build.json#L1)、[实验 source resolver](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/source-resolver.ts#L44)、[实验 Harness 导入](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/durable/harness-setup.ts#L3)、[evals manifest](../../../.repo/pi-1.0.2/packages/evals/package.json)。

API 导出面也不同于整个目录：ai 有 root、models、compat、providers、api、utils 等入口；Chord 将 context/delta/node/bundler 分开；durable 将平台中立存储协议与 Node 适配器分开；coding-agent 的 `./client` 等仅有 source condition，而普通 CLI bin 指向编译后的 bundle。源码可达、包内发布、exports 可导入、默认运行会触发，是四个独立问题。

## 5 系统中的权威状态

| 层 | 拥有的状态 | 何时对外可见 | 不负责什么 |
|---|---|---|---|
| ai Models | Provider 实例、模型目录、认证可用性与缓存视图 | refresh/配置变更的发布点 | 不决定会话下一轮或保存完整 transcript |
| ai stream | 本次 assistant 的内容块、usage、stopReason | 增量事件和最终结果 | 不执行工具 |
| Agent | 本次内存消息、isStreaming、tool batch、steering/follow-up | Agent events；结束监听器完成后 idle | 不承诺跨进程恢复 |
| AgentSession | 当前模型、扩展、压缩、工具负载、用户输入与运行协调 | 会话事件及 UI/RPC | 不等于 OS 权限边界 |
| SessionManager | append-only JSONL entries、parent 链和 active leaf | 追加成功及投影重建 | 不提供通用事务性任务数据库 |
| durable Session/Harness | commits、entries、documents、任务状态和提交队列 | 存储提交之后发布 | 不保证外部副作用 exactly-once |
| Chord provider / state source | provider 管实例注册和订阅；MutableReplicatedState 或 attached source 持有状态权威 | change/adopt 后；远程订阅有激活屏障 | 不定义应用 Session 协议或存储介质 |
| Chord consumer | 本地 replica、ready/unready、序号与路径字典 | hydrate/reset/update | 不是可以任意改写的权威对象 |
| TUI | focus、viewport、editor、frame、终端能力与图片缓存 | terminal write | 不决定模型上下文或持久化完成 |
| telemetry | 观测上下文和 adapter 管理的记录 | adapter 自定义 | 不参与业务决策或自动脱敏 |

默认会话中的“磁盘日志”“模型上下文”“UI 消息”不是完全相同的数组。分支选择、context edit、compaction、custom messages、用户 shell 输入都会改变它们之间的投影关系。分析数据丢失或重放问题时，必须先明确正在讨论哪个表示。[会话投影](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/session-manager.ts#L476)、[消息转换](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/messages.ts#L148)。

## 6 路径一：默认 CLI 从启动到一次回答

### 6.1 启动装配

`cli.ts` 执行 setupCli 后调用 main。main 处理命令、运行模式、设置、项目资源与信任、模型和认证，再创建会话服务及 AgentSession。SDK 调用方可以更细地注入工具、存储、资源加载器和模型服务；CLI 的默认装配不是 SDK 隐含的一切行为。[cli.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/cli.ts#L1)、[main.ts:730](../../../.repo/pi-1.0.2/packages/coding-agent/src/main.ts#L730)、[sdk.ts:175](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/sdk.ts#L175)。

内置扩展中包含 llama.cpp、codemode、tool-search 和 MCP 接线。它们的“工厂注册”“工具是否激活”“是否发生实际连接/请求”不同；例如 codemode 已注册不代表默认立刻启用，SDK createAgentSession 也不会无条件复制 CLI 的全部 built-in extensions。细节见 [coding-agent 的扩展和启动分析](packages/coding-agent.md)。

### 6.2 输入、模型与工具循环

AgentSession.prompt 先尝试扩展命令处理，再发 input hooks，之后展开 skill/template 并构造会话消息，最后进入 Agent.prompt；扩展命令或 input hook 可以提前处理输入而不启动模型运行。Agent 检查是否正在运行，建立本轮 context/config，再由 agent-loop 发出 agent_start、turn_start、message events。[输入顺序](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)。

每次模型调用前都有准备层：会话策略可整理/压缩上下文或选择模型；Agent 将自己的消息类型转换成 LLM Context；ai 的具体 API adapter 将其映射成 Provider 请求。认证可在临近请求时重新获取，避免工具执行耗时导致旧 token 被长期固定。[Agent.prompt](../../../.repo/pi-1.0.2/packages/agent/src/agent.ts#L370)、[agent-loop](../../../.repo/pi-1.0.2/packages/agent/src/agent-loop.ts#L102)。

模型输出作为 text/thinking/toolCall 等内容块流入统一事件。assistant 完成后，Agent 根据 stopReason 决定终止还是执行工具。工具调用不是纯函数名分发：包括 lookup、参数校验、before hook、执行、输出更新、错误结果转换和 after hook。

工具批次默认可并行：先完成串行 preflight，再启动各 execute。任一工具要求 sequential 时，该批次整体按串行路径执行。事件里的 tool_execution_end 可以按实际完成顺序出现，但进入 transcript 的 toolResult 保持原工具调用顺序。收到 steering 不会跳过当前批次尚未执行的工具。[agent 子报告的工具调度](packages/agent.md)。

工具结果进入消息序列，再触发下一次模型调用；没有自然续轮时才处理 follow-up。finishTurn 可以显式要求 continue/end。底层 agent_end 发出后还要等待异步 listeners，Agent 才真正 idle。上层 AgentSession 随后仍可能重试、压缩并续跑、处理新队列或执行 agent_before_settle；其较外层的 agent_settled 才表达这轮会话运行的收束。因此底层结束事件、Agent idle 和产品会话 settled 是三个不同观察点。[会话后处理](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)、[before-settle 边界](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)。

### 6.3 记录与呈现

AgentSession 订阅 Agent events，联动 SessionManager、扩展以及当前模式。interactive 用 TUI 展示，print 输出文本/事件，稳定 RPC 用 JSONL 命令事件协议与宿主沟通。SessionManager 不需要等回答成功才首次保存用户消息；第一条 user/assistant 出现时即可物化文件并写出前面积累的元数据。[session-manager.ts:1166](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/session-manager.ts#L1166)。

```mermaid
sequenceDiagram
  participant U as 用户或SDK调用方
  participant S as AgentSession
  participant A as Agent loop
  participant P as ai Provider API
  participant T as 工具或扩展
  participant J as SessionManager
  U->>S: prompt
  S->>A: 已处理的输入
  A-->>S: message / turn events
  S->>J: 追加会话记录
  A->>P: stream(context, options)
  P-->>A: delta / assistant terminal
  A->>T: 校验、hooks、execute
  T-->>A: update / result
  A->>P: 带工具结果的下一轮
  P-->>A: 最终 assistant
  A-->>S: agent_end
  S->>J: 必要的最终记录
  S-->>A: listener 完成
  A-->>S: 底层 prompt 完成
  opt 会话策略要求继续
    S->>A: 重试、压缩后续跑或边界继续
    A-->>S: 新一段运行的事件与结束
  end
  S-->>U: agent_settled / 会话调用完成
```

图省略扩展短路、失败、重试和压缩，表示典型工具续轮。它不是把所有磁盘写入都放在流末尾，也不表示每个 delta 单独写一条 JSONL。

## 7 路径二：会话分支、上下文编辑和压缩

默认 JSONL 中每个条目有标识及父指针，active leaf 决定当前路径。导航到旧节点并继续只是更换 parent 位置，旧分支仍可存在；创建 branched session 则产生新会话文件和标识，并重建所选路径。日志按追加保存历史，当前模型上下文由投影算法形成。[session-manager.ts:1579](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/session-manager.ts#L1579)、[session-manager.ts:1632](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/session-manager.ts#L1632)。

context edit 可替换或省略某条历史消息对模型的呈现，而不直接删除原历史。compaction 将较早的上下文变为摘要，并保留适当的最近片段；切点需要保护 assistant toolCall 与 toolResult 的对应关系。分支摘要与压缩摘要也有不同触发条件。

默认预算使用 reserveTokens=16,384、keepRecentTokens=20,000，但估算并不是所有 Provider tokenizer 的准确实现：它优先使用有效 usage，再估算后缀，排除 error/aborted 等不适合当锚点的记录。模型变化或旧 usage 被新压缩覆盖时，要防止错误地反复压缩。[compaction.ts:126](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/compaction/compaction.ts#L126)、[compaction.ts:446](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/compaction/compaction.ts#L446)。

超出上下文后的压缩重试与正常回答后的阈值压缩也不同：前者可能需要重发模型请求，后者不能重放已经完成的回答。手工 compact 先终止当前运行，允许 before_compact hook 取消或替换结果，成功才追加 compaction entry 并重建上下文。摘要失败不代表上下文替换已经提交。[agent-session.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)、[agent-session.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)。

因此默认 JSONL 的价值在于可审计的会话树与可重建的上下文，而非“把最后显示的聊天数组直接存下来”。其解析允许跳过损坏 JSON 行及处理不完整尾行；这是一套容错日志策略，不等于跨文件事务或完整的恶意数据 schema 校验。

## 8 路径三：durable 的提交、任务和恢复

durable 是独立包，README 明确标记 experimental。它使用 conversation 组织不可变 transcript entry，使用 document 保存可更新的 agent/live/inbox/usage 等 JSON 状态，使用 task 保存可恢复的状态机。Harness 将这些元素组合成 Agent 工作流。[durable README](../../../.repo/pi-1.0.2/packages/durable/README.md)。

1.0.2 还以 `pi.provider` document 持久保存每个 conversation 的 provider session UUID。它与 conversation ID 分开；fork/child 使用新身份，同一 conversation 重开保持身份，旧会话在首次请求前提交补建。generation 与 compaction 都向 AI 传递该值，但 compaction 仍设置 `cacheRetention: "none"`，因此身份稳定并不保证缓存命中。[ProviderDoc](../../../.repo/pi-1.0.2/packages/durable/src/harness/provider.ts#L6)、[generation](../../../.repo/pi-1.0.2/packages/durable/src/harness/generation.ts#L205)、[compaction](../../../.repo/pi-1.0.2/packages/durable/src/harness/compaction.ts#L168)。

它最重要的顺序是：**生成候选变化 → 原子提交到 Storage → 更新内存权威状态 → 发布可观察变化**。conversation.commit 可以在一个提交里添加 entry、修改 document 和创建 task，从而减少“UI 已显示但磁盘尚未记下”的不一致。

输入 submit 先持久化接纳记录，再启动 generation task；generation 保存请求阶段、模型输出和工具子任务；工具意图先提交，再调用外部 execute；结果被存储后，父 generation 才进一步推进。恢复依赖 task checkpoint 和 registry 中可解析的任务实现，不是重放整个交互式 CLI 进程。[durable 子报告](packages/durable.md)。

这不产生普遍的 exactly-once 外部副作用：进程可能在外部工具已产生效果、结果尚未提交时退出。只有持久化意图中的 replay 与恢复时当前工具的 replay 都为 `"safe"`，并且当前工具仍可解析，才允许自动重跑；恢复时不满足条件就形成 interrupted 结果，包括当前工具已不可用的情况。调用方的安全重放声明是契约，运行时不能凭任意 shell 内容自行证明幂等。[工具恢复分支](../../../.repo/pi-1.0.2/packages/durable/src/harness/tool.ts#L95)。

requestId 在会话内查询已有提交：type 相同则返回已有 ID，不比较 payload；type 不同则拒绝。它不是全局请求键，也不保证相同键的新正文重新生效。wait 的 Context 取消只取消等待，不等于取消已接纳工作；任务取消、关闭 Harness 和等待结果是不同操作。Registry 的代码不写入存储，conversation 保存的是名称和配置，所以跨进程恢复还依赖宿主安装兼容的实现。[接纳与去重](../../../.repo/pi-1.0.2/packages/durable/src/harness/submissions.ts#L155)。

Storage 有 Memory、JSONL、SQLite 等实现，Node 文件系统/SQLite adapter 与平台中立接口分开。一个包宣称可在浏览器 bundle 中使用，表示相应平台中立入口不拉入 Node 依赖，不表示浏览器自动拥有 Node 文件系统或 SQLite driver。[浏览器入口检查](../../../.repo/pi-1.0.2/scripts/check-browser-smoke.mjs#L51)。

“Storage 提交成功”也不能无条件翻译成“断电后绝不丢失”。JSONL 默认 fsync=false；启用时，普通提交先 flush sidecars 再追加 main marker，只有存在 sidecar 回收工作时才在回收前 flush main。该顺序有明确测试，不能仅凭选项名声称每次提交都 fsync 主日志。未知存储失败会使 Session poisoned，要求停止继续使用这个内存视图；这又不同于保证未写入的 StorageRejected。[JSONL commit](../../../.repo/pi-1.0.2/packages/durable/src/storage/jsonl/storage.ts#L278)、[回收前 flush](../../../.repo/pi-1.0.2/packages/durable/src/storage/jsonl/storage.ts#L507)、[不确定失败测试](../../../.repo/pi-1.0.2/packages/durable/test/session-documents.test.ts#L440)。

两套工具循环连参数校验时点也有区别：Agent 的 beforeToolCall 获得已验证但可修改的参数，修改后不再校验；durable 的 beforeTool hooks 可以返回参数替换，执行意图提交前会再校验一次。这是代码层的实际差异，不能把一套 hook 的信任约束推及另一套。[Agent 参数修改测试](../../../.repo/pi-1.0.2/packages/agent/test/agent-loop.test.ts#L487)、[durable 二次校验](../../../.repo/pi-1.0.2/packages/durable/src/harness/tool.ts#L82)。

## 9 路径四：Chord 服务经 Session 协议暴露

Chord 自身不定义 socket、HTTP endpoint 或 Session envelope。它定义 singleton/keyed service、RPC 和 replicated state 的服务语义，由应用提供 remote service adapter。协议包则定义外层请求、响应、事件、握手和 CBOR framing，client/server 负责连接上的生命周期。

典型顺序是：连接建立并完成协议版本握手 → 选择/打开 Session → 查询服务 catalogue → 发起服务 subscription → 取得 snapshot → 激活订阅并按序交付 update。snapshot 与 start activation 两个屏障避免 snapshot 产生后、调用方准备好之前的更新丢失。[protocol](packages/protocol.md)、[client](packages/client.md)、[server](packages/server.md)、[chord](packages/chord.md)。

每个订阅拥有独立 decoder registry，其中每个实例 generation/member 状态流分别维护 delta 的 path dictionary 和序号。断线、实例替换、重新 hydration 或显式 reset 时需要重建相应状态；不能将多个订阅共用一个 decoder，也不能丢弃某些 delta 后继续接收后面的编码流。[state-codec.ts:27](../../../.repo/pi-1.0.2/packages/chord/src/services/state-codec.ts#L27)。

Chord provider 每个订阅最多保留 100 个待交付 update，不包含正在交付的一条。激活前或同步重入使队列加入第 101 条时，用包含当前全部状态及实例成员的 reset snapshot 替代待发队列。这个界限只属于该层队列，不会自动限制 client、socket 或业务消费者的另一个异步队列，也不能直接视作慢网络连接的总积压上限。[Chord 远程订阅说明](../../../.repo/pi-1.0.2/packages/chord/README.md#L88)。

strict JSON 也有分层：defineService 的远程约束主要在 TypeScript，parseServiceCall 检查 envelope 形状，不替代整棵数据树的 JSON 校验。此处 protocol codec 才对 wire 对象执行 isJsonValue；本地 loopback 的参数和结果可保留对象引用。因此“服务可远程暴露”不等于所有本地调用都自动 clone 并验证。[protocol codec](../../../.repo/pi-1.0.2/packages/protocol/src/codec.ts#L20)、[Chord 子报告](packages/chord.md)。

server 对同一个 Session 可以保持多个连接租约；某一连接离开不直接关闭共享 Session handle。接纳串行也不等于整个异步 RPC 全程串行。client 的 reconnect 方法本身不是持续自动重试、恢复 Session 和重建所有业务订阅的调度器；实验产品的 Radius reconnect 是另一层代码。

握手中的版本确认不构成身份认证。授权需要 listener/ByteConnection 或应用宿主提供；这套默认抽象本身不应被当作可直接暴露到公共网络的鉴权服务。[server 子报告的权限边界](packages/server.md)。

## 10 路径五：MCP 和 codemode 扩展工具能力

### 10.1 MCP

mcp 是不依赖其他 Pi 包的 JSON-RPC 客户端，提供 stdio 子进程和 Streamable HTTP/SSE 等传输、请求关联、能力协商及 OAuth。默认 coding-agent 通过内置扩展使用它；它与 experimental CBOR Session protocol 不是同一套线协议。

stdio 的风险与成本包括启动进程、环境继承、输入输出 framing、退出与取消。HTTP/SSE 还涉及 session ID、事件恢复、认证刷新及错误体处理。包里的入站大小限制不是所有读取路径的全局内存上限：普通 HTTP JSON 与某些 error text 路径要分别分析。[mcp 子报告](packages/mcp.md)。

OAuth 模块处理并发 401 的 refresh 单飞、旧 token 引起的迟到 401 和 scope step-up。协议校验、OAuth URL 验证和宿主是否允许访问目标网络，是三类不同约束，不能由“支持 OAuth”推出所有 endpoint 都经过同等严格的地址限制。

本版的 CIMD 配置提供另一种 client identity 路径；可信项目的 MCP 覆盖条目还可调整用户级服务器的启用和曝光策略。工具显示通过独立 renderer 注册覆盖未连接或尚未注册的工具，恢复会话和 HTML 导出由此获得稳定的呈现入口；显示能力不等于工具已可执行。底层协议与产品政策分别见 [MCP](packages/mcp.md) 和 [coding-agent](packages/coding-agent.md)。

### 10.2 codemode

codemode 将生成的 JavaScript 放入独立 Worker 中的 QuickJS/WASI VM，显式注入 tool bridges、输出和 store 接口。每次执行创建新的 VM/worker，不是长期共享一个隐式 JavaScript 全局环境。

工具参数和结果跨 bridge 以 JSON 交换；由 schema 生成 TypeScript declarations 提供编程提示，不等于宿主执行时完成 schema validation。实际工具验证仍需要所属工具系统完成。

输出另有每次脚本 16 Mi 字符与 100,000 项的累计限制，覆盖文本和图片 base64，按 `string.length` 的 UTF-16 code units 计数；超过限制先结算失败再抛异常。它限制的是进入宿主输出集合的内容，并非整个进程内存，也不替代 VM 限额或工具自身的结果限制。[输出常量](../../../.repo/pi-1.0.2/packages/codemode/src/runtime/prelude-source.ts#L30)、[失败顺序](packages/codemode.md)。

VM 自己的 CPU/内存/时间中断，与 host tool 的 AbortSignal 分属两个域。执行正常结束后仍未 await 的 host calls 会收到 abort，但忽略 signal 的宿主实现仍可能继续产生效果。产品扩展还覆盖库默认 timeout，因而不能把库默认限制直接写成 CLI 对用户的限制。[codemode 子报告](packages/codemode.md)。

## 11 并发、取消、完成和重试的统一对照

| 边界 | 并发/完成规则 | 取消或故障后果 |
|---|---|---|
| ai 模型目录 refresh | Provider generation、abort、发布队列约束过期结果 | abort 不等于已完成网络操作可回滚；旧 generation 不应覆盖新目录 |
| ai Provider stream | 增量内容与 terminal result | adapter 负责转译 abort/error；已流出内容不因重试自动撤销 |
| Agent tool batch | preflight 后可并行，结果按 call 顺序 | steering 等当前批次；合作式 signal 不能强停任意工具 |
| Agent end | await 结束 listeners 后 idle | listener 生命周期影响 prompt 的真正完成 |
| AgentSession compact | 有独立 busy/abort 状态和提交点 | 失败/取消不能当成已完成的摘要替换 |
| durable commit | 提交队列串行维护 authority | 存储成功是发布前提；外部副作用仍有未知窗口 |
| durable wait | 观察持久化 submission/task | 取消等待不取消工作 |
| Chord public subscription | 各 subscriber 串行，可合并待交付值 | 不是每条底层操作的审计流水 |
| Chord remote subscription | snapshot/activation、顺序 delta、显式 reset | provider overflow reset；下游队列仍须独立管理 |
| client request | requestId 关联 pending | 取消先拒绝 waiter，相关记录保留到后续响应收束 |
| server connection | 租约保护共享 Session | 单连接断开不代表 Session 被关闭 |
| TUI render | normal 帧合并，input 请求更及时刷新 | 同步 render/write，不提供通用输出流背压协议 |

这张表是跨包解释，具体状态与异常转换以各子报告引用的函数为准。源码里的所有“重试”也不是同一种策略：HTTP、OAuth refresh、模型超时、会话 retry、上下文溢出后压缩、WebSocket/SSE 回退、SSE resume 和 durable replay 分属不同层。

特别是 ai 的通用 HTTP retry 默认实际为 0；Codex WebSocket 只有开始输出前才允许回退 SSE。将默认值或兜底分支抽象成“整个项目自动可靠重试”会丢失重复计费、重复输出及工具副作用的边界。[ai 子报告](packages/ai.md)。

## 12 扩展机制与信任边界

| 机制 | 输入形态 | 执行位置 | 状态/权限特点 |
|---|---|---|---|
| 默认 extension | TS/JS factory、hooks、commands、tools | coding-agent 进程 | 拥有宿主进程能力，可改工具和请求流程 |
| skill / prompt template | Markdown、frontmatter、参数 | 转换为模型输入 | 影响模型行为，不建立 OS 隔离 |
| MCP server | 外部进程或 HTTP 服务 | 独立进程/远端 | 权限由服务与启动配置决定；客户端持有连接及认证 |
| codemode | 模型生成 JS + 显式桥接 | Worker 内 VM | VM 隔离不延伸为全部 host tool 的 OS 沙箱 |
| Chord facet/plugin | 服务声明、激活/销毁逻辑、bundle | 指定宿主环境 | 依赖图约束装配，代码执行环境决定权限 |
| durable extension | tools/tasks/hooks/sections/wraps | Harness 宿主 | Registry 是代码，持久化的是名称及配置 |

项目 trust 主要决定是否加载大多数项目设置和资源，不能与逐次工具批准或路径白名单混同。它还有明确例外：sessionDir 在启动的信任决定前已经读取；上下文指令文件不因拒绝信任而自动跳过；SDK SettingsManager 默认 trusted，调用方需要明确传递政策。[安全文档](../../../.repo/pi-1.0.2/packages/coding-agent/docs/security.md#L31)、[SettingsManager](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/settings-manager.ts#L442)。

配置中的 `!command` 可执行 shell 获取值；扩展导入本身执行代码；模型可调用的 bash 工具也是实际进程权限。它们都是本地信任模型中的真实能力。[配置求值](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/resolve-config-value.ts#L159)、[资源加载](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/resource-loader.ts#L497)。

仓库 README 与 SECURITY 明确表示 Pi 默认以启动它的用户权限运行，没有内建的文件系统、进程、网络及凭据权限限制系统；需要更强边界时由容器、VM 或其他沙箱承担。[README 权限说明](../../../.repo/pi-1.0.2/README.md)、[SECURITY.md](../../../.repo/pi-1.0.2/SECURITY.md#L1)。

这描述项目的安全设计边界，不是漏洞判定。JSON 校验、参数 schema、CBOR 长度限制、取消和资源清理可以减少错误输入与生命周期故障，但并不改变受信任代码与宿主在同一权限域中的事实。

## 13 终端产品为什么占据大量代码

TUI 的复杂度来自真实终端状态，不只是 UI 组件数量。输入被拆成字节片段，ESC 可能是独立按键或控制序列开头；不同终端的增强键盘、鼠标、颜色、图片与剪贴板能力不同。输出需要维护字素宽度、ANSI 状态、光标和终端滚动历史。

main-screen renderer 使用普通屏幕和 scrollback，能在内容局部变化时输出 diff；alt-screen renderer 维护全屏布局、嵌套滚动、选择、搜索及图片生命周期。Component.render 返回字符串行，但 renderer 必须理解其中的零宽 cursor marker、链接与图片控制信息。[tui 子报告](packages/tui.md)。

Editor 又拥有一套独立状态：逻辑文本坐标、视觉折行、undo、kill ring、历史、大段粘贴引用和异步补全。Markdown 的流式 fence 处理、图片 cell 尺寸和 native clipboard 都需要专门分支。它们与 Agent turn loop 基本正交，却直接决定终端产品是否稳定可用。

这也解释源码行数：Agent 包核心只有 6 个生产 TS 文件，而 TUI 有 45 个，coding-agent 的 src 在业务状态、模式、工具、UI 与实验实现合计后达 298 个。不能以核心循环短小推断整个产品简单，也不能以 UI 大包反推底层 Agent 抽象过重。

## 14 观测与评测分别测量什么

telemetry 定义 vendor-neutral tracer/span/schema/context 契约，提供 NOOP、内存参考 adapter 和 conformance suite。span 生命周期由回调管理；记录失败不能改变业务回调执行次数、结果或异常身份。敏感属性标记是 schema 元数据，不会自动脱敏，内存实现也不是有容量与导出的生产后端。[telemetry 子报告](packages/telemetry.md)。

ai 中主要是 telemetry 类型和参数透传，不能因为其依赖 telemetry 就宣称每次网络请求自动产生完整 trace。这里还有明确的文档滞后：telemetry README 声称 Agent 导出业务 schemas 和 helpers，但 Agent 1.0.0 CHANGELOG 已记录移除 experimental harness、telemetry schemas 和再导出，当前 src 与根导出也没有这些符号。该段应作为旧包边界的残留说明，不作为已接通链路或待实现承诺。[telemetry README:365](../../../.repo/pi-1.0.2/packages/telemetry/README.md#L365)、[Agent 1.0.0 移除记录](../../../.repo/pi-1.0.2/packages/agent/CHANGELOG.md)、[Agent index.ts:1](../../../.repo/pi-1.0.2/packages/agent/src/index.ts#L1)。

evals 主要研究文档是否改善任务执行，不是覆盖全部 Agent 能力的综合排行榜。它在 Docker 中对同一 case/model/run 的有文档与无文档条件进行成对执行，控制顺序，严格解析产物，再计算配对指标。未形成有效 pair 时，不能把缺失当零分，汇总还会阻止产生不完整的 headline。[evals 子报告](packages/evals.md)。

评测隔离包含只读挂载、非 root 身份、有限工具和凭据处理，但有模型请求与网络需求；不能仅见容器就假定完全离线。评分规则、模型 judge 与样本量也限制结论外推。本快照分析没有运行 evals，也没有给出项目胜率、成本或性能排名。

## 15 工具链、构建与发布

### 15.1 TypeScript 与工作区

项目使用 ESM、npm workspaces 和直接 `.ts` 相对导入。根 manifest 声明 Node `>=22.19.0`、TypeScript `7.0.2`。base tsconfig 使用 ES2024、strict、erasableSyntaxOnly、Node16 resolution，并在输出时重写相对 `.ts` 扩展；根 noEmit 检查覆盖配置改用 NodeNext，并通过 paths 把包名映射到源码。[package.json](../../../.repo/pi-1.0.2/package.json)、[tsconfig.base.json](../../../.repo/pi-1.0.2/tsconfig.base.json#L1)、[tsconfig.json](../../../.repo/pi-1.0.2/tsconfig.json#L1)。

源码运行、根类型检查、单包编译和安装后导入不是同一个 resolver 环境。尤其 release bundler 显式关闭 monorepo paths 别名，要求编译产物按真正安装包的入口解析，防止只在工作区能成功的隐藏依赖。[build-coding-agent-bundle.mjs:100](../../../.repo/pi-1.0.2/scripts/build-coding-agent-bundle.mjs#L100)。

### 15.2 构建顺序与模型数据

根 build 显式依次构建 chord、tui、telemetry、codemode、mcp、ai、durable、agent、protocol、client、server、coding-agent。ai 的普通 build 先生成/刷新模型数据；build:offline 依赖已存在并通过检查的数据。因而“编译源码”也可能牵涉外部模型目录，必须区别普通 build 与 offline build。[根 build scripts](../../../.repo/pi-1.0.2/package.json)、[ai scripts](../../../.repo/pi-1.0.2/packages/ai/package.json)。

模型元数据、模型上下文 prompt cache、Provider 连接复用和本地会话缓存各有用途。更新模型目录主要改变可用模型/能力/价格描述，不等于修改用户 transcript 或重用模型生成结果。

### 15.3 交付物与发布检查

默认 npm CLI 指向打包后的入口；SDK 保留相应库出口，部分依赖及原生资产仍要随安装包提供。Node bundle 脚本维护 external allowlist，并对 jiti 等做惰性装载处理。独立二进制由 Bun compile 构建，显式包含图片和 codemode worker，关闭 cwd bunfig 自动装载，并带运行资产；不能把一个 executable 文件名等同于无外部资产的交付物。[bundle 脚本](../../../.repo/pi-1.0.2/scripts/build-coding-agent-bundle.mjs#L17)、[build-binaries.sh:129](../../../.repo/pi-1.0.2/scripts/build-binaries.sh#L129)。

直接外部依赖锁精确版本，内部 workspace 依赖保留范围。根 package-lock 固定工作区依赖，coding-agent 的独立 install lock 固定托管安装的传递依赖；1.0.1 起删除了 npm-shrinkwrap 及其生成器，普通 npm 安装不再锁住整个传递依赖树。发布前的 consumer 检查关注“打包并在工作区外安装后能否工作”，与源码别名下的单测形成互补。[README 供应链说明](../../../.repo/pi-1.0.2/README.md)、[coding-agent-consumer.mjs](../../../.repo/pi-1.0.2/scripts/coding-agent-consumer.mjs#L1)。

release 脚本检查干净 worktree 和 npm 注册情况，按需刷新 Nix catalog pin，再更新版本/changelog/模型数据/锁文件，运行 checks、offline build、隔离测试与 consumer 检查，再提交、打 tag、增加下一轮 Unreleased 并 push。publish 则先校验 pack，跳过已经发布版本，使用 provenance 和 ignore-scripts 发布。这些是脚本定义的流程，不代表本次执行过发布或验证了上游 registry 中的状态。[release.mjs](../../../.repo/pi-1.0.2/scripts/release.mjs)、[publish.mjs:38](../../../.repo/pi-1.0.2/scripts/publish.mjs#L38)。

### 15.4 Nix 与固定模型目录

新 flake 通过 root lock 构建 workspace、用固定 SHA-256 的 catalog hydrate 模型数据，然后将 CLI 需要的八个包替换进 install-lock。构建阶段使用 offline build，获取固定输入仍可能联网；运行时也可以更新模型目录。Nix 声明 Linux/macOS 各两种架构，CI 实际构建三种，Intel macOS 仅参与全系统求值。pin 更新、stable 快进条件和依赖审计策略详见 [升级分析第 4 节](version-changes.md)。

## 16 测试体系及其证据强度

测试不是单一框架：ai/agent 使用 Vitest，多个独立包及终端层使用 node:test，应用包还有 fixture、快照、端到端式模式测试和 provider 环境相关测试。评测 evals 另有独立执行成本与语义，不能替代确定性单元和协议测试。

| 检查层 | 检查目标 | 不应从通过中推导什么 |
|---|---|---|
| 类型/lint | TS 类型、格式和静态规则 | 不证明执行时 schema 输入合法 |
| runtime dependency | 发布构建中值导入已声明，排除源码未被重新拉入 | 不覆盖任意计算式动态 import |
| entry graph budget | 指定入口的可达文件数与禁入模块 | 没有预算的入口仍可能很大；不是完整性能基准 |
| browser smoke/tree-shake | 平台中立入口不带 Node adapter，仅选 Provider 不拉全目录 | 不证明所有浏览器实际运行正常 |
| deterministic behavior tests | loop、delta、队列、存储、渲染与回归契约 | 不证明每个外部 Provider 当前服务可用 |
| native/terminal tests | 特定平台与协议分支 | skip 的环境相关路径不等于通过 |
| packed consumer | 打包安装后的 CLI/SDK 入口及资产 | 不证明全部用户平台和网络环境 |
| MCP conformance | pinned suite 与本地 baseline 的差异 | 不等于所有外部 MCP server 都兼容 |
| evals | 指定任务与模型条件下的配对效果 | 不证明总体智能水平或所有任务收益 |

根 `npm run check` 的第一项是 `biome check --write`，会改格式，不是纯只读检查。本次没有执行它。入口图检查也只对列入 BUDGETS 的部分 ai/durable 入口设限；代码注释表达的愿景不能替代实际检查集合。[package.json](../../../.repo/pi-1.0.2/package.json)、[check-entry-graphs.mjs](../../../.repo/pi-1.0.2/scripts/check-entry-graphs.mjs)、[check-runtime-deps.mjs:29](../../../.repo/pi-1.0.2/scripts/check-runtime-deps.mjs#L29)。

`test.sh` 创建隔离 HOME/TMP/config/cache，用 `env -i` 建立允许列表，移除通常从环境继承的 API keys，并禁用本地 LLM 与 EC2 metadata 探测。它与直接在开发者环境中执行 npm test 不同；项目内凭据相关 skip 和测试路径仍需要分别阅读。[test.sh:1](../../../.repo/pi-1.0.2/test.sh#L1)。

CI 的主要 job 在 Ubuntu/Node 22 上 install、build、check、test；MCP conformance 是独立 job。新增 Nix 工作流另覆盖 x86_64-linux、aarch64-linux、aarch64-darwin 三个构建目标；声明支持的 x86_64-darwin 参与 all-systems 求值，但不在构建矩阵中。不能把声明支持、静态求值和实际构建覆盖混为一谈。[ci.yml](../../../.repo/pi-1.0.2/.github/workflows/ci.yml#L1)。

## 17 主要设计取舍

**核心循环小，产品策略集中在会话层。** Agent 通过 hooks、conversion 和配置接受宿主策略；coding-agent 负责模型选择、扩展、压缩、文件日志和模式。这让 Agent 可被复用，但 AgentSession 的接线和事件生命周期因此很重要，不能只读核心循环就理解产品。

**源码重用与发布边界同时存在。** 实验目录可以共享内部实现和 source resolver，却不因此成为稳定 npm 产品的一部分。优点是允许较快探索另一套运行时，代价是读者必须时刻核对入口、exports 和构建排除条件。

**跨 Provider 统一形态，保留特定 API 差异。** 统一 Message、Tool、Usage、Event 接口减少上层分支；签名、thinking、cache、toolCall ID、usage 和错误仍由 adapter 精细处理。统一不代表所有能力都能无损跨 Provider 转换。

**日志可读性与事务式恢复是两种存储目标。** 默认 JSONL 会话树偏向对话导航与上下文重建；durable 将提交和任务 checkpoint 放在更核心的位置。两者各自有清晰用途，不能据同为 JSONL 或同有 conversation 就互换恢复承诺。

**状态复制重视原子版本与按订阅编码。** Chord 将状态权威、consumer replica、事务 draft 和线上的 path dictionary 分开；以 reset 处理 provider 激活前或同步重入造成的待交付队列溢出，而不静默丢 delta。public value 订阅另有 coalescing，transport 异步队列须独立管理。代价是有较多必须遵守的 ownership、generation 与顺序契约。

**不可变是所有权约定，不总是 runtime freeze。** Chord 将部分输入根对象直接转移所有权，公开值可以结构共享。调用方在发布后继续修改容器会破坏约定；“类型上 readonly”不能代替跨不可信边界的 clone/serialize。[Chord README ownership](../../../.repo/pi-1.0.2/packages/chord/README.md#L129)。

**扩展能力强，隔离默认由外部承担。** hooks、shell 配置、动态代码、MCP 和工具使产品能广泛扩展，同时扩大宿主必须信任的代码与资源集合。项目选择把这作为公开信任前提，而不是在每层伪造一个隐式权限系统。

**终端正确性依赖长期回归积累。** 宽字符、图片、resize、scrollback、鼠标和按键碎片是结构性复杂度，测试往往对应具体平台历史问题。测试密度说明维护重点，却不是未经运行即可宣称没有兼容问题的证据。

## 18 推荐源码阅读路线

按问题阅读比从目录第一行读到最后一行更有效。以下路线都能以一次完整状态变化作为终点：

| 想弄清的问题 | 推荐顺序 | 阅读完成的判断标准 |
|---|---|---|
| 一条输入如何变成工具和回答 | agent types → agent.ts → agent-loop → ai stream/API → SDK 接线 | 能画出 turn、工具、队列及结束事件时序 |
| 模型和认证如何变化 | ai models/providers/auth → coding-agent ModelRuntime/AuthStorage | 能区分目录刷新、凭据提交与可用性发布 |
| 历史如何影响当前 prompt | SessionManager entry/projection → messages → compaction → AgentSession | 能解释当前 leaf、原历史与模型上下文的差异 |
| 扩展如何改变行为 | extension types/loader/runner → SDK hooks → built-in extensions | 能指出每个 hook 的时机、可变数据和失败路径 |
| 工具有什么真实权限 | tools 的 operations → bash/env → trust/resource config → MCP/codemode | 能标出执行代码所在进程及取消边界 |
| 为什么终端局部更新不会错位 | tui Component → terminal/stdin-buffer → utils → renderer → Editor | 能区分字节、UTF-16、字素与终端列 |
| 断线后如何恢复显示 | chord service/state → protocol → client/server → experimental host | 能解释 snapshot/activation/reset/generation |
| 崩溃后如何恢复工作 | durable Session/commit/storage → TaskRuntime → generation/tool → Harness | 能指出 durable intent、外部副作用与结果提交的窗口 |
| 发布后是否仍可独立使用 | manifest/exports → build config → bundle → consumer install checks | 能区分源码别名、编译输出和安装包可达图 |

完整逐文件导航见 [源码索引](source-index.md)。各包子报告末尾提供更细的函数与测试阅读路线。

## 19 结论的适用边界与后续可验证问题

本报告确认的是本地快照的实现结构和静态契约，没有宣称项目构建、测试、模型连通、原生装载或基准已经通过。尤其以下问题需要专项执行才能得出经验结论：

- 不同 Provider 对统一消息转换、thinking、cache、工具调用和错误回退的实际兼容程度。
- durable 存储在指定文件系统/SQLite adapter 上的故障注入结果，以及外部工具中断后的实际副作用。
- Chord/protocol/client/server 在高更新率、慢消费者和断连重连下的 CPU、内存与队列增长。
- TUI 在具体终端、SSH/multiplexer、字体和 native helper 组合下的输入延迟、图片行为与清理效果。
- 大会话、多扩展、多模型目录条件下的进程内存、启动成本与长期资源释放。
- 实际 npm pack、独立二进制和各平台资产是否与这份源码快照完全一致。
- evals 的真实样本量、模型版本、费用、统计波动与可重复性。

这些条目是证据空白，不是漏洞或失败清单。能够直接由实现确认的具体限制、需要调用方维护的契约和测试中已有的回归场景，均放在对应包报告中，并附源码定位。整套报告的用途是提供一张能回到代码验证的系统地图。

## 20 设计规划如何补充实现视角

1.0.0 报告已在 2026-10-04 补齐设计、规划与历史演进。本版保留这层分析，并通过文件摘要与新实现复核“为什么这样设计、曾打算怎样推进、哪些方向尚未落地”；它不是仅替换版本号或重复列出 CHANGELOG。

最需要防止混读的四组材料是：

| 材料 | 它补充的设计信息 | 当前应如何理解 |
|---|---|---|
| Chord `PLANNING.md` | services、producer/consumer、delta、facet 装配与分阶段工作包 | 多个工作包已完成；对称 `RpcPeer`、structural reload 等仍需区分具体未实现范围，旧 Plugin 名称不能照抄为当前 API；见 [Chord 报告](packages/chord.md) |
| durable `spec.md` 与 handoff | 提交、恢复、任务所有权、模块边界和多轮方案更替 | “Pico5 未实现”已与现状冲突；handoff 同时含已执行的改版决策，不能整体判废或顺序照做；见 [durable 报告](packages/durable.md) |
| `tui-plan.md` | 主屏/全屏分工、布局状态、输入命中、图片与退出输出、未来方向 | 主要布局能力及曾延期的滚动条已完成；虚拟化、overlay 独立受约束布局等仍是文档明确提出而未完整落实的方向；见 [TUI 报告](packages/tui.md) |
| coding-agent 实验 README 与使用文档 | 实验服务集成、未统一的产品行为、扩展组合和 SDK 使用 | 某些 TODO 的迁移前提已过时，部分能力仍有限制；`plan-mode` 示例是用户工作流能力，不是项目路线图；见 [coding-agent 报告](packages/coding-agent.md) |

其余包也逐一核查文档。没有独立路线图的包明确记录这一事实；保留其文档中的职责边界、非目标与扩展点，不补写未经作者说明的未来计划。telemetry 的过时跨包示例、MCP 的 OAuth 示例边界等也归入“文档是否仍可直接采用”的分析。

本次核查以 1.0.2 文件快照为时间参照。根 README 和 CONTRIBUTING 指向外部 RFC，且后者说明并非所有 RFC 公开；外部材料未纳入此轮本地分析，因此这里不是对全部上游路线图或未来发布承诺的认证。[README 的规划入口](../../../.repo/pi-1.0.2/README.md)、[CONTRIBUTING 的说明](../../../.repo/pi-1.0.2/CONTRIBUTING.md#L99)。逐项状态、证据与文档审读范围见 [专项核查](documentation-audit.md) 和 [完整文档索引](documentation-index.md)。
