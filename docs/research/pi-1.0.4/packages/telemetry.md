# Pi 1.0.4 telemetry 包分析

> 升级核查日期：2026-10-07。以 [1.0.2 历史证据](../evidence/pi-1.0.2/README.md) 与 [当前清单](../source-manifest.json) 比较；旧源码不在现场，不声称取得完整旧新文本 diff。未变正文继承已完成分析，本次重读范围和跨包复核见下文；没有执行 Pi 代码、测试、安装或网络请求。

## 1.0.2 → 1.0.4 升级核查

相对基线：新增 0、修改 2、删除 0、字节一致 10 个文件。

仅 manifest 和 CHANGELOG 变化；6 个生产文件、2 个测试与 README 哈希未变，保留 935 行生产 TypeScript、243 行测试的原分析深度。本次全文读 43 行 CHANGELOG，并重读 README 第 321–400 行的跨包 schema 示例；1.0.3/1.0.4 均无具体发行条目。

Agent 当前根入口仍无 AI/HARNESS/AGENT_TELEMETRY_SCHEMAS，CHANGELOG 1.0.0 的移除记录仍在。README Package Integration 的旧导出说法继续过时；原理与 typed helper 仍有效。新增 env 仅消费 Chord Context 的取消契约，没有把 telemetry context 自动变成跨进程 trace carrier。[Agent 导出](../../../../.repo/pi-1.0.4/packages/agent/src/index.ts#L1)、[移除记录](../../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md#L11)、[env Context](../../../../.repo/pi-1.0.4/packages/env/src/remote-env.ts#L834)。

## 1 定位与结论

`@earendil-works/pi-telemetry` 是一个显式传递上下文、以回调生命周期管理 span 的观测契约包。它提供接口、空实现、内存参考实现、类型化 schema 和适配器一致性用例，不负责发送数据到某个观测平台。它的主要设计目标是让观测保持被动：记录失败不能改变业务回调的执行次数、结果或异常身份。

这是一份针对本地 1.0.4 快照的静态源码报告。继承上一轮对全部生产 TypeScript 文件及两个测试文件的阅读证据，并确认这些文件在 1.0.4 字节未变；未执行测试、连接观测后端或测量性能。类型层约束与运行时约束在下文分别说明。

依据：[核心契约 index.ts:14](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L14)、[参考实现 memory.ts:120](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L120)、[一致性套件 conformance.ts:61](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/conformance.ts#L61)。

## 2 包结构与发布边界

| 项目 | 源码事实 |
|---|---|
| 版本 | `1.0.4` |
| 包格式 | ESM，声明文件与 JavaScript 发布到 `dist` |
| 根入口 | `src/index.ts` 对应 `dist/index.js` |
| 测试入口 | `./testing`，对外提供适配器 conformance 契约 |
| 运行时依赖 | manifest 未声明 `dependencies` |
| 开发依赖 | `@types/node 22.19.19`、`vitest 4.1.11` |
| 引擎声明 | Node `>=22.19.0` |
| 编译 | `tsc -p tsconfig.build.json` |

根实现没有 Node 特有导入，而 testing 入口使用 `node:assert/strict`。因此应区分“核心代码不绑定运行时”和“包引擎、测试辅助要求 Node”，不能把 testing 子路径无条件视为浏览器入口。[package.json:1](../../../../.repo/pi-1.0.4/packages/telemetry/package.json)、[testing/conformance.ts:1](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/conformance.ts#L1)。

| 文件 | 责任 | 阅读重点 |
|---|---|---|
| `src/index.ts` | 低层 API、schema 数据模型与类型推导、typed starter | 运行时实现很薄，主要复杂度在类型系统 |
| `src/noop.ts` | 无观测时的公共替代实现 | 同步进入回调、统一 Promise 返回 |
| `src/memory.ts` | 内存 span 树与脱离内部状态的快照 | 生命周期、错误隔离、顺序与复制 |
| `src/testing/types.ts` | adapter fixture 和 case 接口 | adapter 如何暴露标准化观测结果 |
| `src/testing/conformance.ts` | 可交给不同测试 runner 的用例集合 | 业务语义不可被观测破坏 |
| `test/telemetry.test.ts` | schema 类型约束与 NOOP 行为 | 编译期负例与运行时行为分开 |
| `test/conformance.test.ts` | 内存 adapter 接入公共套件 | 快照不暴露可修改的内部状态 |

## 3 核心数据模型

`AttributeValue` 只表达字符串、数字、布尔值及这些标量的只读数组。`SpanAttributes` 是开放字典，允许 `undefined`；实际记录实现忽略值为 `undefined` 的项。开放名称和属性支持不同业务包共用一个 adapter，不在通用层预设模型、工具、会话等概念。[index.ts:1](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L1)、[memory.ts:54](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L54)。

`TelemetryContext.startSpan(options, callback)` 总是返回 `Promise<T>`。回调参数 `TelemetrySpan` 本身又是一个 `TelemetryContext`，因此父子关系由调用者显式传递。span 没有公开 `end()`，生命周期由回调的返回、resolve 或 reject 决定。这个接口减少了“忘记结束 span”的路径，但也意味着业务必须把需要计入生命周期的异步工作纳入返回的 Promise。[index.ts:14](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L14)。

`SpanStatus` 只有 `ok` 和 `error`；error 可以带 `name/message`，没有独立的取消、超时或重试状态。业务包可以用属性和事件描述这些细节，适配器本身不推断业务分类。[index.ts:12](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L12)。

内存记录包含：

| 字段 | 含义 |
|---|---|
| `id` | 同一 context 内从 1 递增的 span 标识 |
| `parentId` | 显式父节点，根节点为 null |
| `name / attributes / events` | 名称、合并后的属性和按调用顺序保存的事件 |
| `status` | 当前或最终状态 |
| `settled` | 回调是否已经结算 |
| `endSequence` | 结算顺序，未结算时不存在 |

没有起止时间、持续时长、trace ID 或 exporter ID。`endSequence` 可以验证并发结束顺序，不能用于计算耗时。[memory.ts:16](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L16)、[memory.ts:192](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L192)。

## 4 一次 span 的完整生命周期

```text
context.startSpan(options, callback)
  → 复制名称与初始属性，创建记录
  → 同步调用 callback(span)，且只调用一次
  → Promise.resolve(callback 的返回值)
  → 成功：settled=true，分配 endSequence，返回原值
  → 失败：若没有显式 status 则自动标记 error，重新抛出原值
```

参考实现 `startInMemorySpan` 首先检查父 span 是否已结束；已结束则改用 NOOP 执行业务回调。创建记录时如果读取 options 或复制属性失败，也退回 NOOP。这里放弃的是观测记录，业务回调仍然执行。[memory.ts:120](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L120)。

进入回调时发生同步异常，会立即结算记录并返回 `Promise.reject(error)`；返回 Promise 后异步拒绝，则在 rejection 分支结算并 `throw error`。两种路径均保留原始异常对象或任意拒绝值，不将其包装成新的业务错误。[memory.ts:166](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L166)。

显式设置过状态以后，自动异常状态不会覆盖它。比如先 `setStatus({status:'ok'})` 再抛异常，业务 Promise 仍拒绝，但记录状态保持 `ok`。这不是由本报告推断的优先级，而是测试明确要求的语义。调用者必须理解“业务完成方式”和“显式诊断状态”是两套信息。[conformance.ts:146](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/conformance.ts#L146)。

## 5 NOOP 与被动记录

NOOP 使用一个冻结的共享 span；嵌套 span 仍返回这个对象。`startNoopSpan` 不读取 options、不记录名称或属性，在同步调用回调后将其返回值包装为 Promise；同步异常转换为同一拒绝值。[noop.ts:3](../../../../.repo/pi-1.0.4/packages/telemetry/src/noop.ts#L3)。

内存实现的记录方法遵守以下顺序：

1. 已结算则立即返回。
2. 在临时值上完成属性、事件或状态复制。
3. 全部成功后才更新内部记录。
4. 复制或读取异常被捕获，当前记录调用整体放弃。

`mergeAttributes` 先复制当前属性，再遍历新属性，最后赋值回内部记录。因此遇到会抛异常的 getter、Proxy 或数组读取，不会只写入一半属性。`setStatus` 同样在成功复制后才设置 `explicitStatus`。[memory.ts:63](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L63)、[memory.ts:136](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L136)。

“被动”不是任意第三方 adapter 自动获得的保障。接口本身无法阻止自定义 adapter 抛异常；这里的保障来自参考实现和一致性契约。自定义 adapter 需要自己实现并验证这些行为。

## 6 类型化 schema 的作用范围

schema 将每个 span 分成起始属性、结束属性、事件、父节点说明和状态说明。起始属性与事件属性显式声明 required；结束属性始终可选，因为提前失败可能使结果属性不可用。[index.ts:44](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L44)。

关键类型机制包括：

| 机制 | 目的 |
|---|---|
| `AttributeDefinitionValue` | 根据标量/数组定义和枚举值计算实际 TypeScript 类型 |
| `RequiredAttributeNames` | 分离必填和可选属性 |
| `ExactTelemetryAttributes` | 拒绝额外字段，包括通过变量传入的额外字段 |
| `SchemaTelemetrySpan` | 将 addEvent 和 setAttributes 收窄为当前 span 的词汇 |
| `DuplicateTelemetrySpanNames` | 在组合的字面量 schema 元组中发现重名 span |
| `TypedSpanStarter` | 为每个 span 建立关联名称、属性、回调类型的重载 |

依据：[index.ts:76](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L76)、[index.ts:140](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L140)、[index.ts:222](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L222)、[index.ts:278](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L278)。

`defineTelemetrySchema` 是 identity 函数，原样返回对象；`createTypedSpanStarter` 不读取 `_schemas`，仅把 context 绑定到一个转发函数。每次开始 span 时，回调还能收到已绑定到当前 span 的 `startChildSpan`。所有 schema 合并、额外字段限制、枚举范围和名称关联主要发生在编译期。[index.ts:72](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L72)、[index.ts:324](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L324)。

因此以下说法并不成立：

- schema 会在运行时验证来自 JavaScript 的所有输入。
- `parents` 字段会强制检查父子关系。
- `sensitive` 字段会自动脱敏。
- `endAttributes` 保证结束前一定记录该属性。

这些字段表达类型约束或说明信息，adapter 和业务埋点仍负责其实际行为。[index.ts:345](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L345)。

## 7 并发、快照与资源所有权

子 span 的 `parentId` 在创建时绑定到传入的父记录，不依赖异步局部变量或全局当前 span。多个并发子任务可以不同顺序结束，最终记录数组仍按开始顺序保存，`endSequence` 按结束顺序递增。调用者若不等待子任务，父节点可以先结束；本包不会隐式 join 所有子工作。[memory.ts:101](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L101)、[memory.ts:174](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L174)。

`getSpans()` 每次重新复制记录、属性数组、事件和状态，返回值修改不会改变内部采集状态。只读 TypeScript 标注和物理复制是两层不同保证；这里的快照隔离来自复制，而不是深度 freeze。[memory.ts:203](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L203)、[conformance.test.ts:23](../../../../.repo/pi-1.0.4/packages/telemetry/test/conformance.test.ts#L23)。

内存 context 没有 clear、采样、上限、flush 或 dispose API；span 和 event 累积在数组中。长生命周期大量记录会持续占用内存，具体规模未做测量。创建新的 context 可以自然划分采集范围。[memory.ts:192](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L192)。

## 8 与其他包的关系

`pi-ai` 接收、传播 provider 请求选项里的 telemetry context；通用 telemetry 包提供定义 schema 的工具，而没有在生产源码中定义 Agent/AI 业务 span 词汇。

**当前快照存在已能定位演进原因的文档滞后。** telemetry README 的 Package Integration 声称 `pi-agent-core` 导出 `AI_TELEMETRY_SCHEMA`、`HARNESS_TELEMETRY_SCHEMA`、`AGENT_TELEMETRY_SCHEMAS` 及 typed helpers，并给出导入示例。但 Agent 1.0.0 CHANGELOG 明确记录移除了 experimental harness、telemetry schemas 和 telemetry 再导出；本快照 Agent 的 src 与根 index 也没有这些符号。因此该段是旧包边界留下的描述，不是待实现的跨包接线承诺。[README.md:365](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L365)、[Agent 1.0.0 变更](../../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md)、[Agent 根导出](../../../../.repo/pi-1.0.4/packages/agent/src/index.ts#L1)、[通用 typed helper](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L349)。

跨包细节参见 [ai 报告](ai.md) 和 [agent 报告](agent.md)。要核对一次模型请求是否真的发出 span，应沿调用方 helper 追踪，不能仅根据 provider options 中存在 `telemetryContext` 就认定底层 SDK 自带完整 tracing。

## 9 测试证据矩阵

下表描述测试源码中的断言目标，不是本次执行结果。

| 文件或用例组 | 可观察保证 |
|---|---|
| `telemetry.test.ts` schema 部分 | identity、可 JSON 序列化、精确类型、未知事件/属性负例 |
| 同文件 typed starter 部分 | 多 schema、child starter 父子关联、schema 对象无需运行时读取 |
| 同文件 NOOP 部分 | 同步进入、共享冻结 span、相同拒绝值、不检查 payload |
| conformance callback lifecycle | 同步一次进入、返回值和拒绝值身份 |
| conformance status | 显式状态最后写入生效且不被自动状态覆盖 |
| conformance recording | 属性合并、事件顺序、失败原子性、结算后惰性 |
| conformance parentage | 并发与嵌套父子关系、结束顺序 |
| conformance passivity | 不可读对象、状态错误不会改变业务执行 |
| `conformance.test.ts` | 参考实现跑公共契约；快照修改不影响内部状态 |

依据：[telemetry.test.ts:29](../../../../.repo/pi-1.0.4/packages/telemetry/test/telemetry.test.ts#L29)、[telemetry.test.ts:74](../../../../.repo/pi-1.0.4/packages/telemetry/test/telemetry.test.ts#L74)、[telemetry.test.ts:156](../../../../.repo/pi-1.0.4/packages/telemetry/test/telemetry.test.ts#L156)、[conformance.test.ts:5](../../../../.repo/pi-1.0.4/packages/telemetry/test/conformance.test.ts#L5)。

测试套件使用 fixture factory 为每个 case 创建独立 adapter，并通过 `await using` 调用 `Symbol.asyncDispose`。它没有绑定 Vitest 的内部接口，因此其他 runner 可以复用同一组 case。[testing/conformance.ts:12](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/conformance.ts#L12)、[testing/types.ts:5](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/types.ts#L5)。

## 10 设计取舍与边界

- **显式 context 换取清晰父子关系。** 每层调用需要传递参数；作为回报，核心包不依赖某个异步上下文实现。
- **回调结束换取确定生命周期。** 没有手动 end，但脱离返回 Promise 的后台任务不会自动延长父 span。
- **通用低层 API 配合编译期词汇。** adapter 保持开放；typed helper 给业务代码更严格的约束，运行时外部数据验证仍是另一件事。
- **参考实现强调确定性。** 递增 ID 和 endSequence 适合测试，不承担生产性能观测所需的时钟、采样和导出。
- **敏感信息策略不由元数据自动执行。** 本实现复制调用者提供的内容；使用方必须决定哪些内容可以进入 attributes 和 error message。

这些是由接口和实现推导的取舍，不是性能基准结论，也不等同于发现安全漏洞。

## 11 建议阅读顺序与验证空白

推荐按 `index.ts` 顶部接口 → `noop.ts` → `memory.ts` → conformance 用例 → `index.ts` 类型推导后半部分 → `telemetry.test.ts` 中自定义 schema 的顺序阅读。先理解回调是否改变业务行为，再研究类型层如何限制词汇；不要把 README 中当前不存在的 Agent schema 导出当成可用入口。

本次未验证第三方 telemetry adapter、真实 exporter 故障、浏览器打包和大规模内存表现。源码中没有这些实现的测试结果，不应据此宣称它们已被证明可靠或不可靠。

## 12 设计文档、规划与时效核查（2026-10-07）

本包文档包括 [README](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L1) 的 464 行设计契约与用法，以及 [CHANGELOG](../../../../.repo/pi-1.0.4/packages/telemetry/CHANGELOG.md) 的 43 行历史记录；README 字节未变，继承上轮全文审读，本次重读其 321–400 行并全文核对 CHANGELOG。没有独立规划文件。README 的“Applications can provide an adapter”描述现有扩展点，不表示包将内置这些后端。

| 文档主张或方向 | 分类与本快照结论 | 证据 |
|---|---|---|
| vendor-neutral、callback 管理 span、显式传父 context | **现行设计且已实现**；异步上下文机制由 adapter 自行决定 | [设计契约](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L118)、[公共接口](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L1) |
| NOOP、确定性 in-memory adapter、通用 conformance suite | **已发布能力**；0.84.0 已记录引入，不是之后才要建设的测试设施 | [变更记录](../../../../.repo/pi-1.0.4/packages/telemetry/CHANGELOG.md)、[内存实现](../../../../.repo/pi-1.0.4/packages/telemetry/src/memory.ts#L192)、[公共测试入口](../../../../.repo/pi-1.0.4/packages/telemetry/src/testing/conformance.ts#L12) |
| typed schema + 多 schema starter | **已实现的编译期约束**；schema 参数不参与运行时验证 | [README](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L211)、[实现](../../../../.repo/pi-1.0.4/packages/telemetry/src/index.ts#L349) |
| Agent 拥有并导出 AI/harness schemas 与 helper | **过时、被 1.0.0 的包拆分替代**；第 8 节已补齐 CHANGELOG 与根导出两方面证据 | [旧说明](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L365)、[移除记录](../../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md) |
| 多 schema 示例从 Agent 导入 `AGENT_TELEMETRY_SCHEMAS` | **同一旧接口残留**；多 schema 功能有效，但这个具体 import 不成立 | [示例](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L323)、[Agent 当前导出](../../../../.repo/pi-1.0.4/packages/agent/src/index.ts#L1) |
| OpenTelemetry、Sentry、日志后端适配 | **作者说明可由应用实现的适配方向**；本包没有提供对应 exporter，也没有本地文档承诺交付日期 | [范围说明](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L5)、[adapter 职责](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L132) |
| buffering、flush、sampling、backend ID、ambient context | **明确放在 adapter 的职责**；不是 core 缺失的待办清单 | [职责划分](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L132) |
| 时间戳、内存上限与 durable 持久化 | in-memory adapter 明确不记录时间戳、存储无界；telemetry context 明确不应持久化，属于**现行限制与非目标** | [内存限制](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L174)、[安全与可移植性](../../../../.repo/pi-1.0.4/packages/telemetry/README.md#L385) |

该 README 仍保留有价值的设计原则：诊断失败不能改变业务运行次数、返回值或错误身份；生命周期由回调结算决定；schema 元信息与数据处理策略分工明确。应修正的是特定跨包所有权和导入示例，不能因它们过时而否定通用 telemetry 契约。

后续阅读应把“应用可以实现的 adapter”与“Pi 已承诺的产品路线图”分开。本快照只证明适配面与 conformance 工具存在，不证明任何第三方后端已经完成接线；本次也没有把 README 的可移植性描述当作跨运行时执行验证。
