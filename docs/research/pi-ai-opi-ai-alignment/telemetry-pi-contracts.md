# pi telemetry 契约、接入边界与 Rust 选择

> 非规范性研究。日期：2026-10-04。范围为本地 `.repo/pi-1.0.0` 的一手源码、README、注释、CHANGELOG 和测试定义。本轮没有运行 pi 测试、构建、模型请求、联网查询或安装；所有实现结论均为静态分析。本文不批准新目标，不修改正式规范，也不授予实施准入。

> **确认状态更新：Q37 已实际选择“采用 pi 职责划分＋Rust tracing 模块”。** 当前唯一目标见 [decisions.md 的 D-37](decisions.md)、[spec-draft.md 的 AIR-054–056 与 telemetry annex](spec-draft.md) 及 [V-46–48](acceptance-matrix.md)。TC-01–10 保留源码证据；TC-11–12 保留确认前的候选分析，不能把其中未选建议或问题继续当作当前未决项。已选范围是 opi-ai 的显式 context 传播、Rust tracing helpers、宿主拥有领域/backend；没有自动 SDK call/attempt spans、通用 pi callback/schema API 兼容或 exporter。确认不证明实施或验收已完成。

本轮使用已实际读取的 `research`、`codebase-design` 和 `writing-for-agents` 方法：先确定契约与真实边界，再判断证据能够支持的目标。本文是新增中文研究，没有既有英文 counterpart；候选 spec 的双语同步另由主文档负责。

主要结论是：**pi-telemetry 提供显式、被动的 span 契约和参考实现；pi-ai 当前接受、传递该 context，但没有自动生成 AI span；旧 agent 领域 schema 已随实验 harness 移除。** 因此，opi 可以保留这种中立、显式、不会改变业务结果的契约，但不能把“全链路 AI tracing 已在 pi 1.0.0 默认落地”作为实施依据。产品安装上报、attribution headers、实验 analytics 偏好与此契约分开判断。

## 范围、可信度与导航

完整阅读 `packages/telemetry/` 的 12 个现有文件：`package.json`、`README.md`、`CHANGELOG.md`、`tsconfig.build.json`、`src/index.ts`、`src/noop.ts`、`src/memory.ts`、`src/testing/{index,types,conformance}.ts`、`test/{telemetry,conformance}.test.ts`。随后检查 AI options/Models/lazy API/测试，以及 agent、durable、codemode、mcp、coding-agent 的必要边界。检索覆盖 `packages/` 中源码、Markdown 与包 manifest；无命中只用于限定“本地检查范围内没有找到”，不推出第三方不能接入。

版本声明来自 [telemetry package.json:2](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/package.json:2) 与本地 CHANGELOG。本文不凭目录名断言发布 tarball 与此快照逐字一致；不把本轮未核验的上游后续规划写成承诺。整个研究的 revision/快照来源继续以 [README](README.md) 中的基线为准。

```text
应用 / 独立 SDK 调用方
  ├─ 可选显式 TelemetryContext
  │    └─ pi-ai options → Models / provider / API adapter
  │         └─ 当前仅传递；内置 adapter 没有自动 startSpan
  ├─ 可直接调用 context.startSpan(callback)
  │    ├─ 通用契约：pi-telemetry
  │    ├─ 本地参考：noop / in-memory
  │    └─ 自有 backend adapter：export、flush、sampling、backend IDs
  └─ 自有领域 instrumentation / schema
       └─ 旧 pi-agent-core schema 已在 1.0.0 移除；未找到当前替代导出

coding-agent 产品的另一组路径
  ├─ enableInstallTelemetry / PI_TELEMETRY → 安装或版本上报
  ├─ 同一开关 → 部分供应商 attribution headers
  └─ enableAnalytics / trackingId → 实验 onboarding 偏好与持久化
       └─ 本轮未找到对应使用数据 sender；不是 pi-telemetry exporter
```

## TC-01：包边界与依赖不是同一层事实

**当前事实。** [telemetry/package.json:8](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/package.json:8) 公开根入口和 `/testing`；没有 runtime dependencies，只有开发依赖。根实现不导入 Node API；`/testing` 使用 [Node assert](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:1)。manifest 的 Node engine 是 `>=22.19.0`，不等于根源码每条路径依赖 Node ambient context。README 的跨 runtime 定位和 backend 自负兼容性见 [README:391](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:391)。本轮没有构建或在浏览器/Bun/worker 执行来验证这一可移植性声明。

AI manifest 把 `@earendil-works/pi-telemetry: ^1.0.0` 列为生产依赖（[ai/package.json:69](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/package.json:69)），但 AI 源码使用的是 [types.ts:1 的 `import type`](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/types.ts:1)。全 AI `src` 的明确 telemetry 引用只有此类型导入、options 字段和 `buildBaseOptions` 转交。应区分：npm 安装依赖存在、公开 `.d.ts` 需要其类型、当前 AI 源码没有 telemetry 的运行时 value import。没有检查已发布编译产物，不能据此宣称某个 bundler 的实测零成本。

**意图与结论。** [README:5](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:5) 明确无 exporter、全局 current-span 或 backend 依赖。源码支持这个主张。它不提供 metrics、日志收集、配置发现、上传地址或退出 flush 协议。置信度：高。

## TC-02：核心状态机是 callback 所有权，而非公开 end

[核心接口 index.ts:1–22](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/index.ts:1) 的范围很小：

- `AttributeValue` 是 string/number/boolean 或各自同类数组；属性字典允许 `undefined`。
- `SpanOptions` 只有 name 和可选 attributes；`SpanStatus` 为 ok 或 error，后者可带 name/message。
- `TelemetryContext.startSpan<T>(options, callback)` 返回 `Promise<T>`。
- callback 收到的 `TelemetrySpan` 同时可作为子 span context，并提供同步 `addEvent`、`setAttributes`、`setStatus`。
- 没有公开 `end()`，也没有 signal、timeout、cancel、flush、serialize 或 backend trace-ID 接口。

**预期契约。** [README:103](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:103)、[adapter contract:120–132](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:120) 要求同步恰好一次进入业务 callback；保留正常结果及拒绝值身份；同步 throw 变为带同一值的 rejected Promise；等待 callback 返回的 Promise 结算；默认正常为 ok、throw/reject 为 error，但显式 status 优先且最后一次有效写入胜出。后续非 undefined 属性覆盖旧值；记录失败应原子忽略、不抛出、不阻止业务；结算后记录操作无效。

**实现与反证。** memory/noop 实现这些主要路径，TC-03/04 给出定位；第三方 `TelemetryContext` 由调用方提供，而 typed helper [index.ts:324–353](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/index.ts:324) 直接调用它，没有外层保护。因此被动性是 adapter 的义务，不是对任意恶意/有缺陷实现的强制隔离保证。也没有性能上界：同步 exporter 若阻塞，类型签名不能使其不阻塞。

**重要边界。** 显式 `setStatus(ok)` 后 callback 再抛错，业务仍抛原错，但 span 保持 ok；这由 [conformance:142–181](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:142) 特意测试，不能当成偶然 bug 自动“修正”。业务返回 `{ ok: false }` 也不会被通用层自动识别；领域 instrumentation 负责标错。置信度：高。

## TC-03：noop 真正不检查 payload

[noop.ts:3–20](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/noop.ts:3) 立即执行 callback，`Promise.resolve` 包装结果，catch 仅把同步 throw 变为同值 rejection。所有层级使用同一个冻结 inert span，记录方法为空，不读取 name、attributes、status。

[telemetry.test.ts:156–197](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/test/telemetry.test.ts:156) 的断言覆盖同步进入、共享冻结身份、拒绝身份及不可读取 payload 不触发异常。本轮未执行。由此可保留“禁用观测不检查/保留敏感 payload、不改变业务结果”的语义；无需在 Rust 中复制 JavaScript 单例对象身份。noop 也不建立可观测 parentage 或计时数据。置信度：高。

## TC-04：memory 是无界参考 recorder，非生产 exporter

源码 [memory.ts:11–48](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:11) 记录递增 ID、parent ID、name、attributes、events、status、settled 和可选 endSequence；没有时间戳或耗时。状态由每个 `InMemoryTelemetryContext` 实例独有。属性值数组复制一层；合法元素是 primitive，因此足以切断合法数据的数组别名。

[memory.ts:89–185](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:89) 由 callback 的返回 Promise 结算一次；错误自动检查也被 try/catch 包住。创建记录失败则通过 noop 仍执行 callback。事件/属性/status 更新先构造可用新值，再发布；不可读取数据失败被吞掉。显式 status 只有成功复制后才标记为已指定。

[getSpans:203–217](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:203) 返回按开始顺序排列的 detached snapshots，包含尚未结算的 span。readonly 是 TypeScript 约束，返回对象没有被冻结；允许外部修改副本而不改变内部记录。这一点由 [conformance.test.ts:23–44](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/test/conformance.test.ts:23) 明确断言。

**必须保留的限定。** [README:174–176](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:174) 明说存储无界、process-local，要求调用者控制记录范围和数据政策。不得把它误认为已有有界后台队列、采样、flush 或 redaction。

父 span 已结算后再从它启动子 callback，会执行 callback，但走 noop 而不新增记录（[memory.ts:126](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:126)）。已启动但未 await 的子工作没有被父 callback 自动 join；父是否等待它取决于 callback 的 Promise。conformance 的并发 parentage 用例显式 await 两个子调用（[conformance:246–271](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:246)），不能推出结构化并发或子任务取消保证。

**静态发现的边界缺陷。** `copyAttributes` 和 `mergeAttributes` 使用普通 `{}` 加动态赋值（[memory.ts:54–68](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:54)）。对包含 own enumerable `"__proto__"` 的合法字符串键字典，例如 JSON 解析所得的 `{"__proto__":"x"}`，赋值会命中普通对象的特殊 setter，不能忠实保存该属性；数组值还可能改变这个局部字典对象的原型。公共属性字典没有禁止该键，现有 conformance 没有这个用例。这是局部数据记录/字典忠实性问题，不是“全局 Object.prototype 被污染”的证据。本轮未执行复现；建议 Rust 字典按普通字符串键保留，不复制这个 JavaScript 对象偶然。置信度：静态判断高，运行验证未做。

## TC-05：schema 属于声明与编译期约束，不是 runtime policy

[index.ts:26–74](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/index.ts:26) 定义可序列化 schema：独立 version、span 名、description、parent 关系、start/end attributes、event attributes、默认 status 和说明性的 `errorWhen`。属性 metadata 有 `sensitive`、低/高 `cardinality`、封闭 values/elementValues 和 examples。start/event 属性各自声明 required；end 属性总是可选。

`defineTelemetrySchema` 是 identity；不冻结、不验证。`createTypedSpanStarter` 的 schema 参数只参与类型推断，不读取或保留 schema，运行时只创建绑定指定 parent 的 helper（[index.ts:345–353](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/index.ts:345)）。[README:294–338](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:294) 明确 end attribute 是普通 attributes 的完成补充，可在 callback 活跃时任意时点补充，不要求一定调用 `setAttributes`。parent metadata 不执行 runtime enforcement。

编译期 helper 限制 name 与对应属性的关联、必填字段、未知键、event 名、封闭取值、跨 schema 重名；union name 先缩窄，多 schema 保留独立版本。证据为 [index.ts:76 起的类型](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/index.ts:76) 和 [telemetry.test.ts:60–71,138–151](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/test/telemetry.test.ts:60)。后者的 `@ts-expect-error` 声明只有真正运行对应类型检查时才形成通过证据，不能用 Vitest 文件存在来代替。

**反证与安全边界。** `sensitive: true` 不自动擦除数据，`cardinality` 不限流，`errorWhen` 不识别正常返回的失败，schema version 不触发迁移。memory 自动错误状态会复制原始 Error name/message（[memory.ts:71–86](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/memory.ts:71)）；因此 README 的“不应记录 prompt/tool args/headers/credentials/自由错误详情，除非 schema 和政策允许”是领域层责任（[README:387–389](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:387)），不是库已经完成脱敏的证明。置信度：高。

## TC-06：conformance 能证明什么

`/testing` 公开 runner-independent cases；fixture 必须每个 case 独立创建，提供 context 和 normalized snapshots，并实现 AsyncDisposable。`getSpans` 可由 backend adapter 自行 flush 后返回（[testing/types.ts:4–18](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/types.ts:4)、[conformance.ts:12–24](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:12)、[README:209](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:209)）。这些不是根 context 的生命周期管理 API。

| 现有测试设计 | 主要定位 | 不应扩大成的证明 |
|---|---|---|
| 同步单次 admission，返回对象身份 | [conformance:65–85](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:65) | 不证明不会阻塞业务线程 |
| 同步/异步拒绝身份，包括 undefined 和不可读取对象 | [conformance:87–140](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:87) | 不证明 Rust panic/abort/drop 语义 |
| 自动/显式 status，最后显式值不被 throw 覆盖 | [conformance:142–182](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:142) | 不识别领域 Result 内容 |
| 属性合并、undefined 忽略、事件顺序、失败原子性 | [conformance:184–218](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:184) | 不覆盖特殊字典键、有限数值或运行时 schema 检查 |
| 结算后记录无效，晚启动子 callback 仍执行但不记录 | [conformance:220–244](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:220) | 不证明自动 join 子任务 |
| 嵌套/并发 parentage 与结束顺序 | [conformance:246–272](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:246) | 不证明所有 parent 一定晚于所有独立子任务结束 |
| 不可读取 options/recording/status 的失败隔离 | [conformance:274–313](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/src/testing/conformance.ts:274) | 不是对真实 exporter 断网、满队列、死锁、重入失败的完整测试 |

仓库将该 suite 实际注册到 **memory adapter**（[test/conformance.test.ts:5–20](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/test/conformance.test.ts:5)）；noop 有单独测试。没有找到真实 OpenTelemetry/Sentry/log exporter 实现或其 conformance 结果。README 列出这些 backend 是接入可能性，不是已交付 adapter。所有上述测试本轮均未执行。置信度：高。

## TC-07：AI option 传播完整地图

公共 parent 字段定义在 [ProviderRequestOptions:131–135](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/types.ts:131)，注释称“this logical request”的显式 parent；这没有定义 span 名、attempt 数量或 auth 子 span。`StreamOptions`、`DeferredFetchOptions`/`DeferredCancelOptions`、`ClassifierOptions`、`ImagesOptions` 均继承该字段；`SimpleStreamOptions` 再继承 StreamOptions（[types.ts:187](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/types.ts:187)、[types.ts:241](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/types.ts:241)、[types.ts:324–350](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/types.ts:324)）。

| 调用路径 | 当前传递证据 | 已有专门 telemetry 断言（均未运行） |
|---|---|---|
| native stream / complete | [Models:871–893](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:871)；[createProvider:1116](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:1116)；[lazyApi:73–78](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/lazy.ts:73) | fake provider 收到同一 context；complete 通过 stream.result 的源码委托，不是单独 telemetry 用例 |
| simple stream / complete | [Models:895–909](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:895)；[buildBaseOptions:21–48](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/simple-options.ts:21) 显式保留字段 | helper 和 fake provider 两层 identity 断言 |
| deferred fetch / cancel | [Models:912–945](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:912)；[createProvider:1121–1144](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:1121)；[lazyApi:81–94](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/lazy.ts:81) | provider 和 Models 各覆盖 fetch/cancel；并未把 context 写入 durable handle |
| image | [Models:948–963](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:948)；[createProvider:1146–1159](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:1146)；[openrouter-images.lazy:3–5](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/openrouter-images.lazy.ts:3) | legacy 全局 registry 的 [images:19–25](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/images.ts:19) 和 Models 两条路径 |
| classifier | [Models:966–980](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:966)；[createProvider:1161–1170](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:1161)；[typesafe lazy:3–5](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/typesafe-system-one.lazy.ts:3)、[llama lazy:3–5](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.lazy.ts:3) | 专门 telemetry-options 测试没有 classifier case；源码传递存在，不能说能力不存在 |
| auth resolution | [applyAuth:848–852](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:848) 只给 getAuth apiKey/env/signal；[864–866](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/models.ts:864) 才把其余 options 原样保留给 provider | 不存在 auth telemetry 传播断言；不能把 request 字段解释为 auth 已 instrumented |

专门测试是 [ai/test/telemetry-options.test.ts:68–165](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/test/telemetry-options.test.ts:68)，通过自定义 API/provider 捕获 options；不是发给真实供应商、生成 span、redaction 或完整流协议的验证。TypeSafe 与 OpenRouter classifier 共用 System One API（[typesafe.ts:14–15](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/providers/typesafe.ts:14)、[openrouter.ts:33–34](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/src/providers/openrouter.ts:33)）；不能把它们计成两种 telemetry backend。

**容易产生的错误目标。** `Models.complete` 返回 `stream.result()`；图片/分类 catch 后返回领域错误结果，许多错误并不 reject（上表源码）。因此应用把它们直接包在通用 `startSpan` 中，正常返回 error/aborted 也会按 TC-02 默认得到 ok，除非领域包装层显式 setStatus。这是被动通用契约与 AI 返回约定之间的必要衔接，不是已经存在的自动 AI instrumentation。置信度：高。

## TC-08：真实 Agent/其它包消费者要区分直接使用、继承传递与可注入

**低层 agent loop 能传递。** [AgentLoopConfig:193](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/src/types.ts:193) 继承 `SimpleStreamOptions`；[agent-loop.ts:397–407](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/src/agent-loop.ts:397) 将 `...config` 传给 streamFunction。因此即使文件里没有字面 `telemetryContext`，调用者可通过低层 config 传入；“全文无字段命中 ⇒ 没有传递能力”是错误结论。

**高层 Agent 没有对应配置入口。** [AgentOptions:114 起](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/src/agent.ts:114) 提供 streamFn、payload/response/provider-event observer 等；[createLoopConfig:467–504](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/src/agent.ts:467) 逐字段组装，没有 telemetry parent。可注入的 streamFn 能由应用自行包 span/context；这不是 Agent 内建 automatic instrumentation。

**durable 也不能据此推为自动接入。** [ConversationStreamOptions:349–360](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/durable/src/harness/types.ts:349) 是精选且可持久化的请求选项，没有 telemetry context；[generation:201–205](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/durable/src/harness/generation.ts:201) 和 [compaction:164–172](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/durable/src/harness/compaction.ts:164) 由这些字段构造 SimpleStreamOptions，不创建 span。不能为了补 tracing 而把进程局部 context 塞进 durable checkpoint。

**codemode/mcp。** 检查两包 README、公开 index、manifest 和源码 telemetry/schema 关键字，没有找到 pi-telemetry import、context 选项或领域 schema 导出。[codemode manifest:57](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/codemode/package.json:57) 依赖 QuickJS runtime；[mcp manifest:53](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/mcp/package.json:53) 依赖 cross-spawn。mcp transport 的 `onEvent` 是内部 SSE 消息解析 callback（[streamable-http.ts:28](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/mcp/src/transports/streamable-http.ts:28)），codemode “stack traces” 是执行错误定位；名称不能证明 tracing 接入。未查明第三方消费者，不能声称这些库无法由调用方包 span。

目前可具体核验的 telemetry 消费者是参考实现测试、typed-helper 测试和 AI options 传递测试；低层 loop 是真实可用传递 seam，SDK/产品可注入 wrapper 是能力。未找到默认产品自动创建 span 的证据。置信度：高（本地源码范围），外部生态未知。

## TC-09：领域 schema 演进及 README 的局部失效

| 文档主张或变更 | 时间/版本证据 | 当前判断 |
|---|---|---|
| 引入独立 callback contract、noop、memory、conformance、typed schema | [telemetry CHANGELOG:31–35](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/CHANGELOG.md:31)，0.84.0 / 2026-08-06 | 当前仍实现；公开契约，不是仅提案 |
| AI 明确传递 stream/deferred/image 的 TelemetryContext | [ai CHANGELOG:276,345](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/ai/CHANGELOG.md:345)，0.84.0 / 2026-08-06 | 当前有效；classifier 后来通过公共父类型/dispatch 继承，当前测试未补专门 case |
| agent 增加 AI-request/harness schema、combined tuple、helpers、generated reference | [agent CHANGELOG:102–115](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/CHANGELOG.md:102)，0.84.0 / 2026-08-06；同节称 harness scaffold 未完成路径 reject `HarnessNotImplemented` | 历史实现记录，不能推为完成默认产品集成 |
| 1.0.0 移除 experimental harness、telemetry schemas 和 pi-telemetry reexports | [agent CHANGELOG:3–7](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/CHANGELOG.md:3)，2026-10-01 | 当前源码与导出吻合；移除事实明确 |
| telemetry README 称 pi-agent-core 当前导出 `AI_TELEMETRY_SCHEMA`、`HARNESS_TELEMETRY_SCHEMA`、`AGENT_TELEMETRY_SCHEMAS`、start helpers | [README:323–334](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:323)、[367–383](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:367) | **部分过时**：这些 integration 导出已被移除；不能执行该示例或照抄其 owner |
| 通用 schema 应由领域包定义，backend 不拥有业务 vocabulary | [README:13](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:13)、[363](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/telemetry/README.md:363) | 设计意图仍合理且未见撤回；但没有当前实际领域 schema 可以直接迁移 |

已检查反证：不是仅凭 grep 缺失断言移除；CHANGELOG 明确列出移除对象，[agent/src/index.ts:1–5](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/agent/src/index.ts:1) 当前仅导出 Agent/loop/proxy/default stream/types，manifest 也没有 pi-telemetry 直接依赖。`durable` 承接 durable sessions 的说明不等于 telemetry schema 已迁入 durable；未找到相应定义/导出。

可重建的演进链是“通用被动上下文 + 实验 harness 领域 vocabulary → AI 保留传递点 → 1.0 精简 agent，移除旧 harness/schema → 通用 telemetry 包仍保留，但 integration README 未完全同步”。为什么移除、是否未来恢复、将放哪个包，本轮没有当前维护者计划证据。不能从这一链推断必须恢复旧 `pi.ai.*`/`pi.harness.*`/`pi.session.*` 全套或承诺恢复日期。置信度：变更与 README 局部失效高；未来方向未知。

## TC-10：产品安装上报、attribution、analytics 分开处理

| 路径 | 实际 owner/触发/行为 | 与通用 telemetry 的关系 |
|---|---|---|
| 安装/版本上报 | coding-agent interactive startup 在新安装或出现新 changelog entries 时调用；恢复已有消息的会话跳过。`PI_OFFLINE` 非空则跳过；否则 `enableInstallTelemetry` 默认 true，可由 `PI_TELEMETRY` 显式覆盖。向 pi.dev 发送 version query 和 User-Agent，5 秒 AbortSignal timeout，fire-and-forget 且吞失败 | 独立产品网络行为，不使用 TelemetryContext，不是 exporter |
| 部分 provider attribution headers | `createAgentSession` 的请求 wrapper 调用 mergeProviderAttributionHeaders；同一 install telemetry 开关控制 OpenRouter/NVIDIA/Cloudflare 默认 attribution；请求 headers 可覆盖 | SDK 产品 wrapper 的 header 策略，不是 span 数据；不能复制到中立 pi-ai/opi-ai 当遥测基础要求 |
| opencode session headers | 有 sessionId 且 provider/host 匹配时添加 x-opencode-session/client；这个分支独立于 install telemetry 开关 | 不应宣称关闭 install telemetry 会去掉全部产品相关 headers |
| analytics 偏好 | enableAnalytics 缺省 false；首次启用生成 trackingId 并保存。实验首次设置 UI 收集偏好，且初选为分享 | 未找到使用该 trackingId 的数据 sender 或 `/privacy` 实现；有配置/UI 不证明完整 analytics 交付，也不证明未来永远不会启用 |

安装路径见 [interactive-mode:1319–1362](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/interactive-mode.ts:1319)；开关语义见 [core/telemetry.ts:3–12](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/core/telemetry.ts:3)，仅 `1`、大小写无关 true/yes 为真；设置默认值和 trackingId 生成见 [settings-manager:1141–1167](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/core/settings-manager.ts:1141)。这些是源码事实，不是本轮实际发出网络请求。

attribution 的真实接入见 [sdk.ts:316–340](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts:316) 和 [provider-attribution.ts:36–96](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/core/provider-attribution.ts:36)。[sdk-openrouter-attribution.test.ts:142–209](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/test/sdk-openrouter-attribution.test.ts:142) 使用假调用检查默认/关闭/覆盖规则，未运行。OpenRouter 使用 provider 名或 baseUrl substring，而 NVIDIA/Cloudflare 用 hostname 比较；substring 行为甚至有专门 legacy 测试（同文件 :168–173）。这是有证据的兼容选择，不因“看起来不够统一”就归为 bug。

实验 analytics gating 见 [startup-ui:124–147](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/cli/startup-ui.ts:124)：官方 distribution、实验开关、默认 agent 目录且没有 settings；interactive main 才调用（[main:672–676](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/main.ts:672)）。UI 文案声称可以 `/privacy` 查看共享数据（[first-time-setup:72–77](D:/Luiz/Odradek/opi/.repo/pi-1.0.0/packages/coding-agent/src/modes/interactive/components/first-time-setup.ts:72)），但本轮源码检索没有找到对应命令/发送器，应标为**未证实的产品文案主张**；不能把它转换成 opi 的默认 analytics 要求。置信度：已定位实现高，sender 未找到的结论仅限本地检查范围。

## TC-11：建议保留的 Rust 语义与应避免的误移植

本节均为待决建议，不把语言适配或新增 instrumentation 当作已批准目标。

| 契约/选择 | 建议及理由 | 离线验收方向 |
|---|---|---|
| 显式 optional parent，默认无记录 | 保留到 chat native/simple、image、classifier 等已纳入操作的公共 options；自定义 adapter 也能收到同一逻辑 parent。context 是进程局部能力，不进入 transcript、模型记录、持久化消息或 deferred handle | fake adapter 分别捕获各入口 parent；缺省路径无 recorder side effects；序列化 schema 不含 context/backend 对象 |
| 通用 mechanism 与领域 vocabulary 分离 | 通用层保留 span/event/status/parent；请求名、attempt、工具调用、usage、错误标签由掌握真实领域结果的 owner 定义。不要恢复已移除旧 agent schema 并宣称这是现行 pi parity | 编译期闭合事件/属性组合；正常领域失败正确标 status，且不改业务 Result/partial/usage |
| 生命周期由 operation wrapper/guard 所有 | Rust future 的创建不等于执行。span 开始应和已确认调用 admission 一致，Future 未 poll/drop/cancel 的结算须明定；记录层不能 spawn 或继续驱动已取消业务。不能照抄 JS “同步进入 callback”作为未 poll Rust async fn 的要求 | pending future、首 poll、正常/失败、取消、drop、晚记录；每个录入 span 最多结算一次；无业务额外 poll |
| 被动性和结果保留 | 普通 backend 错误、满队列/记录失败不得把成功调用变失败或覆盖原业务错误；记录应原子丢弃。Rust panic 是另一安全边界，不能凭 JS catch 声称已覆盖 unwind/abort；优先构造不抛业务错误的 recorder API，明确 trusted backend 的 panic 契约 | 注入 backend 创建/记录/结算失败，业务执行次数、结果、原错误及取消保持；坏观测不增加模型 dispatch |
| 泛型 callback 不直接复制为动态 trait 方法 | TS 的泛型 `startSpan<T>` 不适合作为 Rust `dyn` 接口原样移植。可用具体 context handle + object-safe recording backend + 普通泛型 wrapper 表达；这只是实现候选，不据此先建公共 crate/trait 系统 | custom backend 和 noop 共用 observable conformance；调用方无需暴露 backend 类型或到处添加泛型 |
| status 与敏感错误 | 保留“正常返回值不自动识别业务失败”的通用边界；在 opi 的已确认 Rust Result facade，由领域 wrapper 明确把 Err 标错并只记录已允许的 typed/redacted 标签。是否保留显式 status 最后写入优先，应按 TC-02 作为可观测 pi 契约讨论，不能静默覆盖 | success/typed failure/cancel；明确 ok 后业务 Err 的指定规则；确认凭据、原始 header/body、prompt/tool args 不进入默认记录 |
| bounded 与 exporter | 默认 noop；memory 明确作为测试/受控范围工具。若新增生产 exporter，由其 owner 定义容量、drop/flush、线程与 shutdown；pi 核心没有这些参数，不能借 parity 擅自造后台任务和配置 | 参考 memory detached snapshots/顺序；生产 exporter 若进入范围另验满容量和关闭，不用网络作为默认验收 |
| schema 类型安全 | Rust enums/typed attributes 可保留封闭 vocabulary；JSON schema metadata 可作为文档数据。无需复制 TS 重载/union 技巧，也不自动增加运行时 schema engine；sensitive metadata 不替代实际数据政策 | compile-fail 类型样例或等效静态检查 + recorder allowlist；无未知字段静默发送 |

其中 clone/所有权、Future poll/drop 和 dyn-safe 表达是 Rust 必要差异；自动添加哪些领域 spans、是否包 auth/attempt/retry、是否拥有后台 exporter、是否跟随产品 install analytics，都是产品或范围选择。不能用“pi 是 Rust 实现”替代这些仍缺少当前 pi 默认行为依据的决定。

## TC-12：确认前的取舍输入（历史）

以下三项是 Q37 前的访谈输入。Q37/D-37 已选择传播职责与 Rust native context helpers；helpers 不提供通用 callback status/settlement，不要求新增领域 span，因此第三项不再由实施 Agent 自行决定一套 SDK span 终态政策。需要额外 instrumentation/backend 时按 D-37 重新选定范围，不能把下述推荐自动纳入本批。

1. **本阶段只对齐通用 context/传播，还是同时交付选定领域 instrumentation？** 推荐先把两者写成明确独立范围：传播覆盖已确认 SDK 操作；若用户要实际可见 traces，再明确本轮业务 span 边界和 owner。代价是仅传递 context 不会自动产生 trace；另一方案必须设计当前 pi 已无可复用现成 schema 的 vocabulary 和 lifecycle。不能让实施 Agent 自行决定一个 span 对应 logical call、attempt、传输还是完整工具循环。
2. **backend 交付到哪一层？** 推荐本轮最多 noop、可离线验收的 memory 与外部 adapter 接口，不默认绑定 OpenTelemetry/Sentry 或开启远端上传。若用户要求可部署 exporter，需要单独确认 backend、数据许可、容量/关闭语义；pi 核心提供的是扩展能力，没有现成默认方案可以不经选择直接移植。
3. **Rust 取消/drop 与显式 status 的交叉规则。** 推荐明确区分业务 terminal 与观测结算：未执行无完成 span，已开始后取消/drop 应结算且不伪称业务成功；保留原业务返回/typed failure。pi 的 last-explicit-status-wins 是明确契约，却没有 Rust drop 对应行为；若决定终态强制覆盖显式 ok，需标为有意差异。这个问题可以在选择 scope 后一次闭合，不需要为每个 backend 另问。

产品安装上报和 attribution 不在上述任一选择中自动获得授权。无需为保持 telemetry 的技术能力而新增 analytics 配置或复制 pi.dev 上报。

## 检查记录与剩余缺口

本轮实际执行的是只读 `rg` 检索、Python UTF-8 文件读取/行号输出及本文写入后的文本结构检查；没有运行 Vitest、TypeScript 类型检查、build 或任何模型/live 测试。部分初始猜测路径（如 `ai/src/stream.ts`、`coding-agent/src/core/first-time-setup.ts`）不存在，后续已用文件清单定位真实 `compat.ts`/UI 文件；不以失败读取证明能力缺失。

建议的未来离线补验包括 classifier context 传播、auth 是否纳入 parent 的明确契约、真实 backend 故障隔离、有界 recorder 如被选中的容量行为，以及 `__proto__` 字典例子的忠实性。它们均是验证设计，本文不宣称通过。现有 TS conformance 不证明 Rust Future/drop、线程安全、panic、强制脱敏或 production exporter。

没有核验快照之后上游 telemetry 变化、维护者恢复领域 schema 的计划、发布 tarball 内容或第三方 exporter 生态；没有证据支持确定路线图。本文测试影响为 `none`（未修改测试）；若上述目标被确认，后续预计 `add` 跨入口传播和生命周期 conformance，`retain` 业务结果/取消/流既有测试；具体 update/delete 需随目标 owner 和 API 迁移确定。

Q37 后已只读复核 D-37、AIR-054–056、公开 SDK telemetry annex 与 V-46–48，并补读锁定 `tracing 0.1.44` 的 `Span::with_subscriber`、`Instrumented` poll/drop、`WithDispatch::poll` 与 subscriber scope 源码。目标明确排除自动 SDK domain spans、通用 schema/callback/status 等价声明、context 序列化和 poll-time ambient 替换；公开 `from_span`、`noop`、`in_scope`、`instrument` 为 custom provider 留有可调用路径，并保留业务输出。没有发现需阻断本批候选收敛的 P1/P2 证据或 SDK 语义矛盾。该结论是文档/源码独立静态评审，不是新能力已编译或运行的证明；实际验收继续按 V-46–48 待执行，测试修改仍为 `none`。
