# 确定 telemetry 的归属与依赖方向
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](01-upstream-contracts.md), [核实 opi-ai 工程、测试与独立接入的现状](02-current-structure-tests.md), [确定独立接入与行为对齐的公共契约](04-public-contract.md)

## Question

通用遥测契约是否独立为 opi-telemetry，哪些 AI 专属 instrumentation 留在 opi-ai，exporter 与配置由谁持有？如何保留可观察行为、最小依赖、上下文传播和跨 agent/coding-agent 的共享契约，而不制造循环或无价值抽象？

## Comments

2026-11-06 首轮建议；以下 Comments 保留讨论历史，当前已确认边界见 Answer。

倾向把中立的 telemetry 契约放入独立 opi-telemetry：上下文、span 生命周期、事件/属性/状态、metrics、传播与 links、no-op 及适配器一致性验证。分离理由是这些能力不属于模型业务，并且上游 ai/durable/chord 的复用已发生；不是为了增加 crate 数量。

opi-ai 保留 AI 专属 instrumentation：模型请求/轮询的 span 名称、provider/model 属性、token/cost 计量、首 chunk 与块完成事件。exporter、采样、后端初始化、flush/close 配置由宿主或可选适配器持有，不能使所有使用者都隐式安装全局 subscriber。

当前 opi-ai telemetry::TelemetryContext 是冻结 tracing Span+Dispatch 的进程内 carrier；上游 TelemetryContext 是后端中立、callback-based 的业务生命周期契约，二者不能因同名判为等价。迁移需要补契约或明确适配层，而不是把 telemetry.rs 机械移动到新 crate。

待决定：是否完整移植通用 schema/metrics/store/render 能力，哪些由独立 telemetry 交付、哪些只是 opi-ai 必需子集；tracing bridge 的归属；旧公开路径的迁移。新 crate 不应反向依赖 opi-ai，也不应强制所有用户依赖完整 exporter 栈。

### 继续讨论：拆分边界草案

2026-11-06，用户已确认公共对齐政策，并选择继续讨论本票。以下内容仍是建议，不是批准的 crate 拆分或已实现能力。

#### 候选所有权

| 所有者 | 拥有 | 边界 |
| --- | --- | --- |
| opi-telemetry 核心 | context/span 生命周期、attributes/events/status/links、可选 metrics/propagation 的共同契约、no-op、schema 描述数据与类型辅助 | 不包含 provider/model/token/cost 等 AI 业务定义；不拥有网络 exporter、全局初始化或后台采集任务 |
| opi-ai::telemetry | AI_TELEMETRY_SCHEMA 的 Rust 映射、请求/poll 的埋点、首 chunk/块完成、usage/cost、停止原因与计量规则 | 观察既有请求转发路径，生命周期由 AI 调用结果驱动；无需向宿主暴露内部协议状态 |
| 可选 tracing 适配 | 把中立 context/span 契约接到现有 tracing，保存/恢复后端上下文 | 第一版可作为 opi-telemetry 的可选 feature/module；是否另拆 crate 由独立消费者决定 |
| 应用宿主 | 选择 adapter、exporter 和采集配置、脱敏策略、缓冲/采样、flush/shutdown | 库正常使用无需初始化全局 subscriber；具体后端依赖由选择它的宿主承担 |

#### 范围建议

本轮优先交付支撑 opi-ai 完整对齐的中立契约，并提供可离线使用的轻量记录 adapter 和共享 conformance 测试。Schema 的描述数据是 AI_TELEMETRY_SCHEMA 的公共能力组成部分，不能因为是“元数据”而漏掉；Rust 的类型辅助不要求机械照搬 TS 条件类型，也不应新增上游未提供的强制运行时 schema 校验。

TelemetryStore/query/render、数据库后端和完整 exporter 产品是另一个范围：本轮建议不自动纳入，不宣称 opi-telemetry 已完整复刻整个上游包。后续真有查询/渲染消费者时再纳入对应契约。轻量记录 adapter 用于观察 span/事件/metrics，不等于承诺实现上游所有存储 API。

上表列的是职责归属。跨进程传播和 links 也属于中立层，但当前 AI observer 不直接调用它们；其本轮具体交付范围需按公开接口映射和真实消费者决定，不因未来可能引入 durable/chord 就提前实现完整跨进程设施。

核心建议不强制依赖 Tokio、HTTP 或 exporter SDK；不以“先架一个通用遥测平台”为目标。保留请求显式携带上下文的方式；tracing 的现有 carrier 可以成为适配器实现的一部分，其冻结 dispatcher/poll/drop 语义需单独映射和验证。

#### 挪动代码前需要守住的行为

- span 的开始/结束时点由契约定义，不能因 Rust Future 惰性或普通作用域结束自动改成另一个时点；AI request span 必须按具体入口的结果路径结算。
- 普通请求与 deferred poll 的计量是 AI 域规则，留在 opi-ai。上游 aborted span 保持 ok；pending/deferred poll 不累计 token/cost，块长度按 UTF-16。它们不能被通用适配器按自身惯例覆盖。
- 上游无操作 adapter 仍调用业务 callback，并保留其结果；现有 opi 无操作 carrier 还通过 Dispatch::none 静音作用域内 tracing。两者需区分：中立 no-op 不应被默认为全局日志静音策略。
- 通用 adapter 契约不保证吞掉后端失败。错误传播、observer 失败和 exporter 缓冲/flush 失败分别处理，具体 Rust 映射在边界确认后讨论。
- 已有 AI instrumentation 的入口范围应保持可追踪，不能从“请求类型可以携 telemetry”推导所有 image/classify/direct API 都自动埋同一 span。

#### 已由用户确认的两项（保留原问题）

1. 本轮 opi-telemetry 是否采用“完整覆盖 opi-ai 所需契约 + no-op/轻量记录与一致性测试”的范围，将 Store/query/render 与具体 exporter 留待真实需求？推荐采用；若用户希望通用遥测包本身也全量对齐，需要另列完整公开清单，不能作为隐含工作。
2. 无论能力范围如何，是否保持公开契约后端中立，让 tracing 作为可选适配而不是所有调用者的强制公共类型？推荐采用；Rust 原生接口不等于必须把某个观测库的类型暴露给所有消费者。

用户随后确认这两项。边界结论见 Answer；生命周期与错误映射转入独立决策，公共路径迁移由模块结构决策继续处理。

#### 本轮追加源码核验

以下路径相对仓库；仅静态核验，未运行测试。

- AI 所需 schema/typed metrics：`.pi/pi-1.5.0/packages/ai/src/telemetry.ts:1–10,15–105,109–153`；schema 经 `src/models.ts:73` 与 `src/index.ts:30` 公开。typed starter 不做运行时验证：`packages/telemetry/src/index.ts:448–456`；typed metrics 将定义传给 adapter：`src/metrics.ts:129–143`。
- Store/query/render：`packages/telemetry/src/index.ts:32–94`、`src/render.ts:10`；AI observer 无调用。它们是独立包的额外范围，不自动从 pi-ai 对齐目标推出。
- No-op：Pi `packages/telemetry/src/noop.ts:4–5` 直接执行 callback；opi `crates/opi-ai/src/telemetry.rs:113–119,139–152` 在 scope/poll/drop 安装无操作 dispatcher。当前 provider 缺省 None 分支与显式 noop 也不同（`openai_chat.rs:3175–3178`）。若保留静音行为，应作为独立后端策略/差异记录。
- 请求生命周期：`packages/ai/src/models.ts:907–917` 在 setup 前建立 observer；`api/lazy.ts:37–82` 从转发路径观察并等待结果；`telemetry.ts:139–153,192–207` 通过 finish 结算、aborted 保持 ok、排除 pending/deferred poll 的 token/cost。Rust stream 返回不等于请求结算。

## Answer

2026-11-06 用户回复“同意，继续推进”，确认前轮两项推荐。已确定：

1. 将中立遥测能力放入独立 opi-telemetry；本轮交付范围是完整支撑 opi-ai 的能力映射，包括所需 context/span、attributes/events/status、metrics、schema 描述、no-op、轻量内存记录器及共享契约测试。
2. AI 专属 schema、请求/poll 生命周期、首 chunk/块事件、token/cost 和停止原因规则仍由 opi-ai 拥有。opi-telemetry 不反向依赖 opi-ai、agent 或 CLI。
3. 公开契约保持后端中立。tracing 是可选适配模块，可复用既有 carrier 的机制；不要求第三方使用者接受 tracing 类型作为中立公共接口，不预先拆额外 tracing crate。
4. Store/query/render、数据库和具体 exporter 不自动纳入本轮；不承诺本轮完整复刻 pi-telemetry 独立包。传播/links 的实际实现范围按公开映射和消费者登记，不因未来可能引入 durable/chord 而扩张。
5. 宿主负责后端选择、exporter、采样/脱敏、初始化、flush/shutdown；独立库的正常调用无需全局 subscriber 初始化。

该确认是架构与范围决策，不是生产代码实施授权。具体生命周期/错误/drop 映射在[确定遥测生命周期与失败语义](11-telemetry-lifecycle.md)处理；旧公开路径迁移在[确定 opi-ai 的模块结构与公开入口](06-module-structure.md)处理。当前没有运行行为测试，也没有声明任何遥测实现已通过等价验收。
