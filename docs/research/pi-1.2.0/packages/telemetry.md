# telemetry：显式上下文与被动记录契约

分析日期：2026-10-16。对象是 Pi 1.2.0 本地固定快照；本次只做静态源码、测试与文本审读，没有安装依赖、构建、运行测试或访问服务。测试影响：`none`。

## 职责与发布面

`@earendil-works/pi-telemetry` 是不绑定厂商的诊断基础包，提供 callback 管理的 span、共享 NOOP、内存参考 adapter、可序列化 schema 与类型推导，以及独立测试子入口。它不拥有 exporter、采样、flush、后台队列、全局 current span 或业务持久化。包无生产依赖，Node engine 为 `>=22.19.0`；根实现没有 Node 专用上下文 API，`./testing` 才使用 Node assertion。发布资产是 `dist` 与 README，源码统计为 6 个 TypeScript 文件、935 行；测试统计 2 文件、243 行。 [manifest](../../../../.repo/pi-1.2.0/packages/telemetry/package.json#L1)、[公开根入口](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L1)、[测试入口实现](../../../../.repo/pi-1.2.0/packages/telemetry/src/testing/conformance.ts#L1)。

低层 `TelemetryContext.startSpan(options, callback): Promise<T>` 同步且恰好一次接纳 callback；callback 获得的 `TelemetrySpan` 同时是子 span 的显式父 context。记录接口只有 `addEvent`、`setAttributes`、`setStatus`，没有公开 `end()`。业务结果或 rejection 的身份必须保留；同步 throw 转成相同值的 rejected promise。parent 依靠参数传递，不依靠异步局部存储。 [契约类型](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L1)、[README adapter 契约](../../../../.repo/pi-1.2.0/packages/telemetry/README.md#L118)。

## 执行链、状态与生命周期

内存链是 `startSpan → 复制 start 属性并分配 id → 同步 callback → Promise settlement → 标记 settled/endSequence → getSpans 复制快照`。每个 `InMemoryTelemetryContext` 私有持有 span 数组、下一个 id 与结束序号；子 span 共享该 state 并记录 parentId。`getSpans()` 按开始顺序返回 detached snapshot，数组属性也复制；没有时间戳，序号表达确定性次序而非真实耗时。 [memory.ts:44](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L44)、[接纳与 settlement](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L120)、[快照](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L192)。

正常 callback 默认状态 `ok`，throw/rejection 默认变为 `error`；调用者显式设置过状态后，自动错误不会覆盖它，重复显式状态最后一次生效。这允许“正常返回的业务失败”标为 error，也允许显式 ok 后 callback 失败仍保留 ok；这是记录契约，不是异常吞掉。原异常仍向调用者传播。 [settleSpan](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L89)、[setStatus](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L157)、[conformance 状态用例](../../../../.repo/pi-1.2.0/packages/telemetry/src/testing/conformance.ts#L142)。

`setAttributes` 合并 defined 值、忽略 undefined；记录操作先构造完整副本再替换，因此不可读 payload 不留下部分写入。记录失败被吞掉，不影响业务 callback。settled 后记录调用无效；从已结束的父 span 启动子工作仍执行 callback，但退化到 NOOP，不再追加记录。 [属性复制与合并](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L50)、[recording 方法](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L136)、[后结算测试](../../../../.repo/pi-1.2.0/packages/telemetry/src/testing/conformance.ts#L220)。

NOOP 使用同一个 frozen 惰性 span，根和嵌套 callback 都收到它；不读取 options，也不保留 attributes/events/status。其 `Promise.resolve(callback(...))` 在同步调用 callback 后处理异步结果，捕获同步 throw。 [noop.ts](../../../../.repo/pi-1.2.0/packages/telemetry/src/noop.ts#L1)。

资源与并发边界：内存记录无限增长，生命周期由创建 context 的应用决定，没有 delete/reset/flush；每个 callback 可并发启动子任务，树的正确性来自显式 parent 绑定。父 callback 若不 await 子 promise，父结束不会替应用等待子工作；该接口不是 task group。包没有取消、重试、超时或持久恢复机制，相关语义属于业务 callback 与 adapter。

## Schema 与扩展

schema 的 `version/spans/parents/startAttributes/endAttributes/events/status` 是普通可序列化数据；值域仅 primitive scalar/array。`defineTelemetrySchema` 是 identity function。`parents`、sensitive、cardinality 与 status.errorWhen 说明设计意图，不执行运行时约束或脱敏。 [schema 定义](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L26)、[identity](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L71)。

`createTypedSpanStarter(context, schemas)` 通过 overload、exact 属性类型与 literal tuple 的 duplicate-name 检查约束调用；子 starter 绑定 callback span，因此复用相同词汇并保留父子关系。运行时完全不读取 schema 参数，不校验 span 名称、必填字段、值域或 parent 规则；来自 JavaScript、反序列化或类型断言的数据不会自动被拒绝。start 必填性只在创建时检查；end attributes 全部可选，仅用于完成信息 enrichment，随时可在 callback 存活期间设置。 [类型推导](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L112)、[duplicate 与 overload](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L281)、[runtime binder](../../../../.repo/pi-1.2.0/packages/telemetry/src/index.ts#L324)。

`./testing` 的 `createTelemetryAdapterConformance` 生成 runner 无关的分组 case，fixture 用 `await using` 清理，每例创建独立 adapter；测试 backend 应归一化为 RecordedTelemetrySpan，并可在 `getSpans()` 内 flush。它表达同步单次接纳、结果/拒绝身份、显式状态、原子记录、事件次序、后结算惰性、并发 parentage 等可观察契约，不证明真实 exporter 网络可靠性。 [suite](../../../../.repo/pi-1.2.0/packages/telemetry/src/testing/conformance.ts#L12)、[测试接线](../../../../.repo/pi-1.2.0/packages/telemetry/test/conformance.test.ts#L14)。

## 实际集成与文档时效

当前 ai 的 request options 接受 `telemetryContext` 并传递到 provider，但没有因此自动安装 exporter；本包不拥有 AI 或 harness schema。`agent` 当前也没有这些 schema。telemetry README 的通用 context、NOOP、memory、schema 与 conformance 章节能与源码对应；但 README 323–338 与 365–383 的集成示例仍从 `pi-agent-core` 导入 `AGENT_TELEMETRY_SCHEMAS`、`AI_TELEMETRY_SCHEMA`、`HARNESS_TELEMETRY_SCHEMA`、`startAiSpan`、`startHarnessSpan`。全包导出及全 `packages` 符号搜索只在该 README 命中 schema 名，而 agent 1.0.0 发行记录明确删除了 schema。这些具体示例已陈旧，不能作为当前可编译 API 或“未来待实现”解释。 [README 集成](../../../../.repo/pi-1.2.0/packages/telemetry/README.md#L323)、[agent 导出](../../../../.repo/pi-1.2.0/packages/agent/src/index.ts#L1)、[删除记录](../../../../.repo/pi-1.2.0/packages/agent/CHANGELOG.md#L23)。

README 安全与 portability 要求 context/span 不进入消息、快照、deferred handle 或持久记录；schema 没有自动防止敏感内容进入属性。内存 adapter 会复制 Error.name/message，应用仍需按数据策略选择输入。 [安全说明](../../../../.repo/pi-1.2.0/packages/telemetry/README.md#L385)、[automaticErrorStatus](../../../../.repo/pi-1.2.0/packages/telemetry/src/memory.ts#L78)。

全文审读了 README 1–464 与 CHANGELOG 1–47。没有独立规划文档；CHANGELOG 的 0.84.0 新增项在当前实现与 conformance 中兑现，后续空版本标题不能推导新增功能或路线图。当前边界是将 exporter、全局激活、采样和持久性留给 adapter/应用，并非遗漏待办。 [发行记录](../../../../.repo/pi-1.2.0/packages/telemetry/CHANGELOG.md#L43)。

## 测试、升级与阅读顺序

静态核对 `telemetry.test.ts` 的 schema exactness、多 schema child starter、NOOP 同步和异常身份用例，以及 `conformance.test.ts` 的参考 adapter 接线与 detached snapshot 断言；没有实际执行。核心记录语义由通用 suite 覆盖，第三方 adapter 的故障策略与跨运行时兼容仍需其自身验证。 [类型与 starter 用例](../../../../.repo/pi-1.2.0/packages/telemetry/test/telemetry.test.ts#L29)、[NOOP 用例](../../../../.repo/pi-1.2.0/packages/telemetry/test/telemetry.test.ts#L156)。

与旧 1.0.4 清单逐路径/摘要比较，本包只有 package.json 版本与 CHANGELOG 改动，源码、测试、README 内容相同；旧源码目录不在本次工作区，未做逐行旧新 diff。本次对当前源码重读并重新定位行号，保持独立判断；尤其不能因 README 字节未变就继承其 agent schema 集成主张。升级没有新增运行语义。

建议阅读顺序：README context/adapter 契约 → index.ts schema 与 binder → noop.ts/memory.ts → testing/conformance.ts。静态风险集中在无限内存记录、应用未 await 子工作、第三方 adapter 未兑现被动性，以及集成文档中的过期导出。
