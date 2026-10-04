# pi-ai 设计演进与文档有效性

> **目标解释更新：** 本页基线事实与一手定位保留；首轮目标建议须与 [决策重审](decision-reassessment.md)及 decisions 的第二轮确认一起阅读。D-18以 pi 的 Rust 实现为方向，不能用旧准入或当前 opi 未接线单独否决已落地行为。


> 非规范性研究。研究/上游访问日期：2026-10-03（Asia/Shanghai）。不授权产品实施，不把上游计划视为 opi 目标。本文只维护演进证据；目标取舍以本目录决策文档中的人类确认状态为准。
>
> 本文为新的中文研究材料，没有既有英文 counterpart；不修改 `docs/research/pi-1.0.0/` 的独立 pi 研究。候选 spec 的中英文同步由 spec 文件负责。

## 基线、来源和证据强度

**EVO-001 — pi-ai 包已核验为官方 v1.0.0 tag 的同内容快照。** 本地 [package.json](../../../.repo/pi-1.0.0/packages/ai/package.json) 声明 `@earendil-works/pi-ai` 1.0.0。[官方 tag API](https://api.github.com/repos/earendil-works/pi/git/ref/tags/v1.0.0) 返回 commit `a13d35a742c6ef8462812a28fbe1d8c8b7431c32`；[release](https://github.com/earendil-works/pi/releases/tag/v1.0.0) 发布于 2026-10-01T19:20:55Z。不是仅凭目录名推断版本。

本次执行 `gh api repos/earendil-works/pi/git/trees/a13d35a742c6ef8462812a28fbe1d8c8b7431c32?recursive=1`，确认 `truncated=false`；对每个 `packages/ai/` blob 用本地字节计算 Git 对象哈希 `SHA1("blob " + byte_length + NUL + content)`，与官方树逐项比较：**378 个匹配，0 个不同，0 个缺失，0 个本地额外文件**。这证明 ai 包内容等同该 tag；不证明整个 monorepo、npm 发布包、二进制、ignored 模型数据或依赖安装结果与 tag 等同。主报告另核验独立研究的 source manifest；它与官方来源核验是不同证据。

上游时间窗口固定为查询到的 main `11449730c8a733953ce1bcce70e066bccaa778a5`（提交时间 2026-10-03T11:54:18Z）。[官方 compare](https://github.com/earendil-works/pi/compare/a13d35a742c6ef8462812a28fbe1d8c8b7431c32...11449730c8a733953ce1bcce70e066bccaa778a5) 返回 ahead 26、behind 0；`gh release list` 当次最新仍为 v1.0.0。下文“已合并未发布”只适用于这个窗口，不能替代未来查询。

证据等级：源码/导出/测试断言为静态核验；Git blob 对比和 GitHub 元数据为本次实际执行核验；上游 PR 中的 live-test 描述仅是维护者报告，**不是本次运行结果**。本研究没有执行 pi-ai 测试、构建、模型调用或性能基准，没有安装依赖或运行模型生成脚本。

## 材料盘点与阅读次序

本地 `packages/ai/` 的主设计叙述实际位于 [README](../../../.repo/pi-1.0.0/packages/ai/README.md)、[CHANGELOG](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md)、类型/接口注释、测试断言。0.80.0 和 0.84.0 的迁移说明嵌在 changelog 内；本次文件清单搜索没有找到独立的 pi-ai `DESIGN.md`、`MIGRATION.md` 或统一 roadmap。根 `tui-plan.md` 属于 TUI，不用来推断 ai 方向。

跨包材料限于解释实际消费者所需的 coding-agent `ModelRuntime`、`ModelRegistry`、provider composition、SDK 接线、codemode、虚拟模型和目录刷新；durable 只用于确认轻量入口的真实需求。没有审计整个应用、持久化或扩展系统。

官方根 README 指向 [Earendil RFC 站点](https://rfc.earendil.com/)。其中有独立但非统一路线图的 [RFC 0039：Dynamic Model Configuration](https://rfc.earendil.com/0039/)、[RFC 0054：Responses Lite Investigation](https://rfc.earendil.com/0054/)、[RFC 0019](https://rfc.earendil.com/0019/) 与替代它的 [RFC 0038](https://rfc.earendil.com/0038/)。站点状态本身不能证明落地；需逐项与 tag 源码对照。

## 文档主张有效性索引

“文档状态”和“实现状态”独立判断。`当前有效` 指主张仍能描述所注明版本，不意味着值得照搬；`已实现` 不意味着本研究执行通过其测试。

| ID / 主张 | 适用版本、时间与来源 | 文档状态 | 实现状态 | 验证与限制 |
|---|---|---|---|---|
| EVO-002 全局 API 向实例化 Models/provider 迁移 | 0.80.0，2026-06-23；ai changelog、README Providers and Models | 当前有效 | 已实现 | `models.ts` 的 provider map/auth/dispatch，root/compat 导出，coding-agent ModelRuntime；迁移不等于 compat 已退出 |
| EVO-003 compat 是临时机制，并将随 ModelManager 迁移删除 | 2026-06-10 commit、当前 `compat.ts` 注释 | 部分过时 | 部分实现 | 临时入口仍在且有生产消费者；ModelRuntime 已落地而删除条件未兑现。无已核验删除版本/日期 |
| EVO-004 Provider 拥有 auth/catalog/API dispatch，Models 负责集合和请求装配 | 0.80.0 → 0.80.8（2026-07-16） | 当前有效 | 已实现 | auth 从旧全局 registry/产品辅助器收敛到 provider；产品配置覆盖仍在 coding-agent，并未全移入库 |
| EVO-005a 动态目录分离类型与数据，离线 fallback + cached remote + 用户覆盖 | RFC0039，2026-05-29；0.81.0 后迁移、1.0.0 源码 | 当前有效（这些核心目标） | 已实现 | model-catalog/provider 数据入口与 coding-agent remote catalog 已有对应实现；不等于 RFC 全文完成 |
| EVO-005b 目录版本协商与选中模型 analytics 的提议细节 | 同一 RFC0039；展示状态仍为 Discussion | 无法确认（这些细节的当前契约地位） | 部分实现（RFC 整体；上述细节未逐项证实） | 缺少逐条实现或替代证据；尚未证明交付不等于已过时，不以页面状态推断已接受 |
| EVO-006 目录刷新取消并拒绝过期 generation 发布 | 0.84.0，2026-08-06 | 当前有效 | 已实现 | `context.publish()`、`refreshGenerations`、失败保留既有目录和离线测试断言；非合作存储的副作用不能由接口名证明可回滚 |
| EVO-007 用 DeveloperMessage / additional_tools 改善有序上下文，暂不做完整 Lite | RFC0054，2026-07-11 | 部分过时 | 部分实现 | additional_tools 已用；DeveloperMessage 原型被更一般的 SystemMessage 取代。完整 Lite 未由这份提案证明交付 |
| EVO-008 system/tools 状态属于 transcript；按能力原生回放或 collapse | PR9548 合并 2026-09-16，发布 0.86.0 | 当前有效（1.0.0 语义） | 已实现 | 类型、normalize/replay/collapse、adapter 测试及产品记录；具体 Anthropic wire 在 tag 后已替代 |
| EVO-009 image/classifier 共用 Provider/Models，而非平行 ImagesModels | PR9948 合并 2026-09-23，发布 0.99.0 | 当前有效 | 已实现 | 新模型类型/目录/adapter/codemode 接线均在；虚拟路由是另外的实验用途 |
| EVO-010 root 无副作用且轻量，/models 更小；SDK 延迟加载 | 0.80.0、0.99.2 | 当前有效（须区分入口） | 已实现 | root 仍加载 TypeBox；/models 测试禁止 TypeBox/catalog/SDK。未测毫秒或内存，不能把拆入口直接换算为性能收益 |
| EVO-011 旧 install telemetry 与新 opt-in analytics 是同一当前方案 | RFC0019 → RFC0038，2026-04/05 | 已被替代（0019 的设计地位）/无法确认（0038 全面落地） | 部分实现 | 0019 页面自称被0038替代，却仍标 Implemented；产品保留 install telemetry。不能与库中的 TelemetryContext 混为一谈 |
| EVO-012 tag 后 Anthropic inline tools、Clef classifier 和边界修复 | 2026-10-02 main commits | 当前有效（仅该 main 窗口） | 已实现（已合并未发布） | commit diff/受影响测试文件已查；没有在本地执行或并入1.0.0结论 |
| EVO-013 telemetry 中立包与旧实验 Agent schemas的分开演进 | 0.84.0→1.0.0，补核2026-10-04 | 中立契约当前有效；README Agent导出说明部分过时 | 通用包/传播已实现；旧schema已移除；exporter新接线仅提案 | [TE-01–13](telemetry-evolution.md)验证提取/移除提交、作者设计文本、PR状态及telemetry12blobs；Q37固定opi传播责任，不复制旧实验 |

## EVO-002 / EVO-003：实例所有权取代隐式全局，但兼容并未退出

演进链：全局 catalog/API registry 和环境鉴权便利入口 → 多消费者、bundle 隔离、provider-owned auth 的需求 → [Models 初始实现 `f63095c`](https://github.com/earendil-works/pi/commit/f63095cfff0d4df5239eec8a789d3ad91078b8d5) → [provider factories `fec0c3d`](https://github.com/earendil-works/pi/commit/fec0c3d12f4156d7be6a4254968222b7d1fc655d) → [core-only root 与 compat `8a0903e`](https://github.com/earendil-works/pi/commit/8a0903ebf2db9d708b3d733119638f7407cdef49) → 0.80.0 发布。

关键并非改名字。`ProviderId` 表示身份，`Provider` 是拥有行为的运行时单元；`model.api` 选择 wire adapter，一个 provider 可有多个 API；`Models` 持有 provider 集合并统一 auth/请求装配。[models.ts](../../../.repo/pi-1.0.0/packages/ai/src/models.ts) 是实际所有权与路由证据。

随后 [同步读取/显式异步 refresh `6e98573`](https://github.com/earendil-works/pi/commit/6e98573f24a09c858bbdfcf1171e39f37c2bc3d1) 明确指出：sync-or-async union 会让同步调用方的隐含假设直到 dynamic provider 出现才出错；全部改成异步又破坏同步 extension API。最终同步读取 last-known snapshot，refresh 单独显式进行。这个取舍有真实消费者原因，不能解释为“TypeScript 喜欢全局函数”。

compat 的旧入口确实有注册副作用，[package sideEffects](../../../.repo/pi-1.0.0/packages/ai/package.json) 明示 `compat.js`、images registration 等。它是迁移桥，不是新调用方的推荐核心；但是不能根据 [compat.ts 顶部注释](../../../.repo/pi-1.0.0/packages/ai/src/compat.ts) 写“已经淘汰”。反证包括：

- [ModelRuntime](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts) 已在构造中调用 `createModels()`；[2026-07-14 提交](https://github.com/earendil-works/pi/commit/9993c96907bb0c97260d2c353c31a3464f211122) 已把 model registry 换成 runtime 并保留 extension facade。
- [sdk.ts](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/sdk.ts) 仍注册 compat `streamSimple` 为默认 stream function，但 `createAgentSession()` 又明确注入 `modelRuntime.streamSimple()`。**导入 compat 不能证明默认产品主路径直接以全局 API 路由。**
- [provider-composer.ts](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/provider-composer.ts) 仍通过 `getApiProvider(model.api)` 处理兼容 adapter；[ModelRegistry](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-registry.ts) 注释自称同步 extension compatibility facade。

当前判断：实例 runtime 是有明确意图、真实产品接入的核心；compat 是尚在服役的迁移负担，不是任意可删代码。注释把退出绑定到 ModelManager 迁移过于粗糙，当前证据没有完整退出清单或期限。置信度高（现状/历史），中（未来退出时点）。opi 应借鉴单一权威及显式所有权，不应为了接口对齐复制 pi 的临时 registry/全局桥；是否删除 opi 的任何入口仍须证明其消费者与替代路径。

## EVO-004 / EVO-005 / EVO-006：鉴权、目录与刷新责任逐步明确

Provider 的 auth 能力、凭据存储、目录数据、产品覆盖不是同一责任。0.80.8 将 auth/check/login/logout 纳入 Models，废弃旧 OAuth 全局 registry；[auth resolve](../../../.repo/pi-1.0.0/packages/ai/src/auth/resolve.ts)、[credential store](../../../.repo/pi-1.0.0/packages/ai/src/auth/credential-store.ts) 与 `Provider.auth` 协作。凭据 store 管持久化和序列化更新，provider 定义鉴权机制；Models 做请求时解析和 auth/model/explicit headers 装配。显式调用 `getAuth()` 再流请求可能重复解析，README 因此提供 Models-only `transformHeaders` 并要求 dispatch 前消费移除。

动态目录的原始问题不是“缺少足够通用的 registry”，而是巨型生成 TS 值文件频繁变化、目录修订绑在产品发版上。[RFC0039](https://rfc.earendil.com/0039/) 提议按 provider 发布 JSON、保留离线 fallback 和类型提示、overlay cached/catalog/user overrides。其展示状态仍是 Discussion；实现已走在状态标签前面。

当前数据/类型分离可在 [model-catalog.ts](../../../.repo/pi-1.0.0/packages/ai/src/model-catalog.ts)、provider `.models.ts`、[model-data 脚本](../../../.repo/pi-1.0.0/packages/ai/scripts/model-data.ts) 找到。**pi.dev 远端更新策略在 coding-agent 的 [remote-catalog-provider.ts](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/remote-catalog-provider.ts)**：URL、缓存新鲜度、ETag、类型过滤及产品 overlay 不全属于中立 ai 库。文件中构造 `types=chat,image,classifier` 的目录请求；不能把 RFC 未最终确定的所有版本协商字段与选中模型 analytics 视为现有保证。核心目标仍有效；后两项缺少足够逐条核验，不能因实施证据不全而判为过时（EVO-005a/005b）。

早期 refresh 的 in-flight dedupe 和 mutable store access 暴露取消/过期发布问题；[2026-08-03 `fed6009`](https://github.com/earendil-works/pi/commit/fed6009cc9ccfb15088bc55244120918b5fe23f2) 和 0.84.0 迁移将它改为 caller-owned cancellation、generation 检查、`context.stored` + `context.publish()`。`Models` 在取消时停止等待，即使 provider 不合作；后继 generation 可开始；旧 generation 的可见状态发布被拒绝。

核验边界：`publishProviderModels()` 在写存储前后检查 generation，把同 provider publication 串行化；这不是任意自定义 storage 外部副作用的事务回滚证明。`Models.refresh()` 未传 signal 时仍可能无界等待，是 changelog 明说的契约，不能赞誉为“全部有超时”。[models-runtime.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/models-runtime.test.ts) 有取消非合作 provider、拒绝 late publication、取消 auth/store wait、OAuth refresh 单次化、保留凭据的断言；本次只阅读，未运行。

选择影响：可借鉴 last-known snapshot、显式 refresh、过期结果拒绝、请求时解析 auth 的可观测语义；不要为了模仿 pi 先引入远端模型市场、背景网络刷新、OAuth persistence 或同构 store trait。opi 是否已有至少两个真实消费者以及失效边界，须由设计审计判断。置信度高（历史和源码责任），中（未审计远端服务的完整目录协商）。

## EVO-007 / EVO-008：从 deferred tool 标记到 transcript 状态转换

原始限制：把 `systemPrompt` 和 `tools` 作为请求顶层快照不能说明它们何时变化；修改它们会改写缓存前缀；把工具激活绑在某个 tool result 的 `addedToolNames` 上无法表达一般状态变化。

[RFC0054](https://rfc.earendil.com/0054/)（2026-07-11、Discussion）研究 Responses Lite，但认为完整 Lite 的模型特定复杂度不值得；提出先用 `additional_tools` 和 text-only DeveloperMessage prototype。**这不是承诺将采用 Lite 的路线图。** 0.84.2 已偏向 message-anchored `additional_tools`，仍保留 tool-search/top-level fallback。

[PR9548](https://github.com/earendil-works/pi/pull/9548) 的 merge commit 为 `9e05370b298d0a6b8d9bc2c02e4bfae189ef1616`，合并 2026-09-16，0.86.0（2026-09-19）发布。正式落地采用 `SystemMessage`（含 sections patch、toolsAdded、toolsRemoved），不是 RFC 的 text-only DeveloperMessage；所有 provider 下游收 `TranscriptContext`，入口统一 normalize。重放得到当前 prompt/tools；支持的 adapter 原生定位更新，不支持则 collapse 为前导当前状态。移除 `addedToolNames` 是职责替换，不只是加一种消息。

一手实现与反证：

- [types.ts](../../../.repo/pi-1.0.0/packages/ai/src/types.ts) 的 `SystemMessage` / `Message` / `TranscriptContext`；[transcript.ts](../../../.repo/pi-1.0.0/packages/ai/src/utils/transcript.ts) 的 normalize、getCurrentTools、getCurrentSystemMessage、collapseSystemMessages。
- [transcript-tool-changes.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/transcript-tool-changes.test.ts) 断言 Anthropic native/fallback、OpenAI additional_tools/tool-search/removal fallback、Kimi 特定工具消息及普通兼容端点 collapse；它证明可测试边界存在，不证明所有远端模型接受。
- PR 中维护者描述了 live 场景和 cache 观测；本次没有重跑，不能将“cached prefix stays intact”当所有 provider 的经验结论。能力 flag、初始工具是否存在、同名重定义和 removal 都影响实际降级。

这一演进把“按顺序保留状态变化”与“供应商允许如何发送”分开，值得作为目标讨论。但 opi 若采用 transcript 需同时决定持久化、重放、迁移、历史 tool schema 与 agent/product 的责任；只修改 opi-ai Context 类型不能完成该语义。是否进入范围由用户决定，不能以 pi 已做为授权。

tag 后 [b271b0a](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db)（2026-10-02）又将 Anthropic native additions 改为 `inline-tools-2026-09-15` beta 的按值定义，保持顶层初始工具，并废弃 `hasToolRedefinitions()`。这是已合并未发布的实现，相关 adapter、types、README、测试、SDK/lockfile 一起改动。故 1.0.0 README 的具体 Anthropic发送策略仍是该基线事实，但不再描述 main。稳定目标应是顺序、可重放、降级和不丢状态，不是 beta header 或 wire block 名称。置信度高。

## EVO-009：多操作模型是近期整合，不能等同“所有调用都是 chat”

历史链：[2026-06-10 `827fe1e`](https://github.com/earendil-works/pi/commit/827fe1e2c48bcb6534807ce0eba5241fd5438fcb) 为 image 建立平行 `ImagesModels/ImagesProvider`，共享 auth resolve → 重复集合和 provider infrastructure → [PR9948](https://github.com/earendil-works/pi/pull/9948) / `a328aa89ad6e6dc5c5628ff896769532ed3d29df` 合并 2026-09-23 → 0.99.0（2026-09-29）删除平行 image 集合，合并到 `Models/Provider`，增加 classifier。

当前公开模型有 operation type；chat 的未标注 type 被视为 chat 以保留旧调用。相同 provider/upstream ID 可以按 type 各有一条目录项；无类型限定读取仍返回 chat。`generateImages()` 和 `classify()` 是不同输入输出契约，不能通过 chat stream 使用；Models 验证类型并做 provider auth。相关证据为 [types.ts](../../../.repo/pi-1.0.0/packages/ai/src/types.ts)、[models.ts](../../../.repo/pi-1.0.0/packages/ai/src/models.ts)、[images-models.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/images-models.test.ts)、[classifier-models.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/classifier-models.test.ts)、[llama-cpp-classify.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/llama-cpp-classify.test.ts)。

真实消费者不只样例：[coding-agent ModelRuntime](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/model-runtime.ts) 对接 image/classifier；[codemode execute](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/codemode/execute.ts) 把二者暴露给执行中的脚本，并有自身并发限制。此处证明产品能力接线，不表示普通 chat 每轮自动调用 classifier/image。

另一条用途是 [虚拟模型](../../../.repo/pi-1.0.0/packages/coding-agent/docs/virtual-models.md) 选择物理模型；[引入提交 `540e174`](https://github.com/earendil-works/pi/commit/540e174c72cd793170b73db8160c6d5f6c5a2236) 明言 experimental，`jev-router.ts` 是示例。不能把“classifier 库实现已存在”和“自动模型路由是稳定默认策略”混为一谈。

当前分类：Provider/Models 多操作整合已发布且有消费者；image/classifier 是近期新增能力；虚拟路由策略为实验。opi 的目标取舍已由本次人类访谈独立确认，见 [D-01/D-13](decisions.md)：图片生成与分类纳入 SDK／嵌入式 Agent 候选目标，归入 opi-ai，并提出有界规范修订。原先“只有 chat 需求时保留 chat-only”的条件推理不再是本次范围建议。选定这些操作不自动认可 pi 的 `AnyModel` 形状、全部多操作抽象、自动路由策略或 auth 扩展；对应责任和验收由候选决策明确。置信度高（pi 当前实现）；不确定的是长期 API 稳定性，而非现有代码是否存在。

## EVO-010：入口隔离与 SDK 成本是可检验边界，不是语言移植清单

0.80.0 的 root/core-only 解决隐式全量 catalog/registry/OAuth 导入；provider factory 只带自身 catalog 和 lazy API wrapper；`providers/all` 明确是全量入口。root 仍 `export { Type } from "typebox"`，所以“root side-effect free”不等于“没有依赖装载成本”。

[2026-09-29 `ba7d5fb`](https://github.com/earendil-works/pi/commit/ba7d5fbedb1864c1d6e1b26910b95b1634a665c9) 为 durable 使用增加 `/models`（0.99.2 发布），避免 TypeBox、catalog、SDK 的加载；[models-entry.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/models-entry.test.ts) 的子进程 loader hook 明确拒绝这些路径，并通过 faux completion 检验实际使用而非只 grep 导入。

0.84.2 还将 Mistral SDK 替换为原生 Chat Completions HTTP stream（changelog），表明上游不会为了“统一使用供应商 SDK”保留无收益的生成客户端和 schema runtime。但它没有删除所有 SDK：1.0.0 package 仍依赖 Anthropic、OpenAI、Google、Bedrock SDK。

因此可对齐的是模块/依赖可达性、显式成本和能力边界测试，不是 TS 子路径形状。Rust 中 crate/features/linking/编译成本与 JS 首次导入成本不同；无测量不声称 opi 当前慢，也不以此单独要求拆 crate、加 feature 或替换 HTTP。置信度高（入口与依赖事实），中（未测性能收益）。

## EVO-011：不要混淆产品 analytics 与库级 telemetry

[RFC0019](https://rfc.earendil.com/0019/) 虽显示 Implemented，却在正文明确由 [RFC0038](https://rfc.earendil.com/0038/) 替代；后者仍是 Discussion，提出更细的 opt-in analytics。1.0.0 产品仍有 [isInstallTelemetryEnabled](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/telemetry.ts)。本次没有核验所有 analytics 出站路径，故只确认该配置和替代关系，不宣布0038已全面实施或已撤回。

另一个独立事实是 0.84.0 新增库级 `TelemetryContext`，依赖 `pi-telemetry` 的 vendor-neutral contract；这是请求上下文传播与观测接口，不能仅因同名 telemetry 推断上报内容或用户 consent 策略。前稿只研究到这一点，未完成包架构及消费者审计；2026-10-04补核见[EVO-013/TE](telemetry-evolution.md)。作者实际在0.84发布前提取中立包并补reference/conformance，1.0删除旧Agent schemas，但通用包保留；README integration局部过时。opi以D-37/AIR-054–056对齐显式传播，利用Rust tracing模块，不新增crate/exporter或自动SDKspans。安装量统计不是本批目标。

本页以下EVO-012仍是2026-10-03的固定历史窗口，其“当次最新release1.0.0”不是本日状态。新telemetry补核已确认v1.0.1发布与main `83692682f095528f8b71652ddacff7075e36e893`，只对本主题核验差异，不据此宣称已重新审计全部pi-ai后续变化或恢复旧schemas。时间、版本、官方链接及proposal状态都集中在TE。

## EVO-012：基线之后可核验的进展

以下均在固定 main 窗口内已合并、当次最新 release 仍为1.0.0，**不是1.0.0行为，也不是未来承诺**。

| commit / UTC 时间 | 已核验变化 | 对本研究的实际作用 |
|---|---|---|
| [b271b0a524b29e13c0c9e748aea0d34e1597f2db](https://github.com/earendil-works/pi/commit/b271b0a524b29e13c0c9e748aea0d34e1597f2db) / 2026-10-02 10:30:48 | Anthropic inline tool definitions；废弃 hasToolRedefinitions；SDK 与测试同步更新 | transcript 语义应独立于 beta wire 机制 |
| [4812cb268efc3f0846337cdfb8f00ce51b7b8d5a](https://github.com/earendil-works/pi/commit/4812cb268efc3f0846337cdfb8f00ce51b7b8d5a) / 2026-10-02 14:52:58 | Cloudflare Clef classifier，adapter/generator/test 更新 | 多操作扩展正在发生；不能据此预建无限通用操作框架 |
| [eeac84ca92498ac18b6832754d01aef1d3c5f654](https://github.com/earendil-works/pi/commit/eeac84ca92498ac18b6832754d01aef1d3c5f654) / 2026-10-02 15:44:05 | ChatGPT OAuth callback 端口占用时先失败，取消错误的 paste fallback | OAuth fallback 是有具体 provider 条件的失败契约，不是通用“总能换方式” |
| [4665fafb4ddc8cf066484fbe14d0c5825f4d3a1d](https://github.com/earendil-works/pi/commit/4665fafb4ddc8cf066484fbe14d0c5825f4d3a1d) / 2026-10-02 16:25:43 | Bedrock pricing tiers 从 models.dev 保留到生成结果 | catalog 存在不证明价格语义完整；需端到端 fixture |
| [69f0be6f0a6ca1bf66a2a9cf59c821b632e6bca1](https://github.com/earendil-works/pi/commit/69f0be6f0a6ca1bf66a2a9cf59c821b632e6bca1) / 2026-10-02 20:38:43 | Bedrock 部分 Claude 回放 drop stale thinking blocks，按模型/区域 gate | thinking 签名/绑定跨请求语义继续修正，不能全量无条件回放 |

计划分类：维护者明确意图仅包括 RFC/迁移说明中可读出的方向；上述 main commit 是实施事实；RFC Discussion 并不等于已接受；“更偏向实例所有权、按操作分型、adapter局部兼容”是从历史抽取的研究归纳，不是维护者承诺。未发现能支持具体 compat 删除时间、统一公开 pi-ai 路线图、将 Lite 全面纳入、或将虚拟路由升级默认稳定策略的已核验承诺。

## 对候选决策的约束与证据缺口

可带入决策讨论的核心行为：显式实例所有权、provider/API 分离、last-known catalog 与明确 refresh、取消与过期结果控制、可重放状态变化、操作类型边界、入口依赖边界。需谨慎处理的迁移/偶然形状：compat 全局 registry、chat 默认缺省 type、双套 image globals、供应商 beta headers、TS barrel/subpath 切分。后者不自动是错误，但没有理由直接成为 Rust 目标。

opi 是否纳入 transcript 与非 chat operation 已由人类确认，后续迁移、投影和 authority 细则的状态以 [decisions.md](decisions.md) 为准；本文中的 pi 历史不代替这些决定，也不授权实施。任何 opi API 淘汰仍须保留消费者、替代和退出证据；远端目录及 OAuth 持久化会扩大 authority/生命周期；pi 的 live 兼容、缓存收益与 SDK性能不能从静态材料宣称通过。

本次实际执行的研究检查：`gh api` tag/trees/commits/compare、`gh release list`、`gh pr view 9548`、`gh pr view 9948`，均返回有效一手结果；本地 ai blob 对比378/378匹配；官方 RFC 页面已读。若命令中的文件路径初次检索不存在，后续通过 `rg --files` 定位正确路径，未把路径不存在记为能力缺失。`agent-reach check-update` 返回当前v1.5.0已最新。未运行运行时测试；测试影响 **none**（仅读既有断言）。文档检查由主 Agent 合并全文后统一执行。

## SDK 继续复核的文档有效性（2026-10-04，仍以固定快照为准）

| 主张／一手来源 | 文档主张状态／实现状态 | 冲突、演进与目标影响 |
|---|---|---|
| README 的 builtin env 鉴权与 Models 管理说明；models.ts:269–308,393–396,645–821、auth/context.ts:23–27 | 当前有效；已实现惰性来源、配置检查、解析、目录刷新、login/store与local logout。读取源码而非运行证据 | 构造即cache restore是旧候选的误解，不是pi宣称；返回/副作用由UX-11–13核定，D-35依此修正 |
| README:1814要求SDK结束后cleanup；CHANGELOG:993,1002记0.73.0（2026-05-04）；index.ts:36与session-resources.ts:12–23 | 当前有效；公开helper已实现，是稳定SDK生命周期能力，不是compat待删项 | 旧问题是session资源阻止host退出；同步全局callback方案清缓存socket，却遗漏connecting/temporary/overflow，可能触发fallback。Rust实例await/代际取消是有意正确性加强，不冒充上游承诺本地全部quiescence |
| Codex stream默认auto；src/api/openai-codex-responses.ts:294,1518及公开transport variants | 默认主张当前有效；WS已有真实实现/SDK测试断言，省略值delta行为部分不同 | pi选择WS而非仅产品adapter；原options eligibility未统一是实现偶然。D-35采用effective auto一致性并保留显式SSE/WS/cached；无cache临时连接不等于禁WS |
| StreamOptions public retry字段；types.ts:177–184、provider-retry.ts:97–124及Codex/Bedrock send | Server-delay准入语义当前有效；Bedrock连接这些公共字段仅部分实现，无法确认全adapter文档范围，不仅因代码缺读点判文档过时 | 锁定SDK让Bedrock默认standard最多2额外，与helper/Codex0不同；Q36选择真实默认及override接线。Codex宽catch和AWS非可取消timer为修正项；不把SDK env/adaptive实验当已确认范围 |

以上链条通过公开导出/具体实现/静态测试/消费者核验，不以版本较旧自动撤销文档，不以测试文件存在声称通过。精确路径与依赖归档身份集中在 sdk-consumer-contract UX-11–16及SDK annex；本轮未再联网查询后续main，不将这些材料当当前上游路线图。公开Models/retry形状若后续变化需重新核定语义，不按文件布局搬运Rust代码。
