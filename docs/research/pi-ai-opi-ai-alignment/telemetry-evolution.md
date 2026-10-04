# pi-telemetry 的设计意图与演进核验

> 非规范性研究，供候选 spec 决策使用；不是实施准入、上游路线图或运行时验证报告。访问日期：2026-10-04。本文只研究 pi 的一手材料；Rust `tracing` 的机制和最终 opi 选择由本目录的决策及 spec 负责。本文为新增中文证据附件，没有既有英文 counterpart，不需同步既有双语文档。

核心结论是：pi-telemetry 的通用、显式、被动诊断契约已经发布；pi-ai 的当前职责是传播请求上下文。旧 Agent harness 的领域 schema 和接线规划不是 1.0 的现行公共能力。独立包、callback、typed schema 各有明确目的，但不能据此推导 opi 必须新建 crate、复制 TypeScript API，或默认导出遥测。

## TE-01：基线与可重复性

| 对象 | 固定依据 | 本次实际核验 |
|---|---|---|
| 本地 pi 快照 | `.repo/pi-1.0.0/packages/telemetry/`；声明版本 `1.0.0` | 读取全部 12 个文件；对官方 Git tree 返回的每个 blob 计算本地 Git blob SHA-1，12/12 匹配，缺失及额外文件均为 0 |
| 官方 v0.84.0 | `a5f43bf8aff3c55752432655f7334e3dafd1e256` | tag 指向 commit；GitHub Release 于 2026-08-06 11:07:05 UTC 发布，非 draft、非 prerelease |
| 官方 v1.0.0 | `a13d35a742c6ef8462812a28fbe1d8c8b7431c32` | tag 指向 commit；GitHub Release 于 2026-10-01 19:20:55 UTC 发布，非 draft、非 prerelease |
| 官方 v1.0.1 | `a7229ddc21810d6245105978033b7df645ecc2f7` | tag 指向 commit；GitHub Release 于 2026-10-03 16:14:00 UTC 发布，非 draft、非 prerelease |
| 本次读取时 main | `83692682f095528f8b71652ddacff7075e36e893`，提交时间 2026-10-03 13:23:44 UTC | 与 v1.0.0 比较 31 个提交；telemetry 包只改 package version、CHANGELOG，未发现新增运行时接线 |

来源：[v0.84.0 Release](https://github.com/earendil-works/pi/releases/tag/v0.84.0)、[v1.0.0 Release](https://github.com/earendil-works/pi/releases/tag/v1.0.0)、[v1.0.1 Release](https://github.com/earendil-works/pi/releases/tag/v1.0.1)、[固定 main 比较](https://github.com/earendil-works/pi/compare/a13d35a742c6ef8462812a28fbe1d8c8b7431c32...83692682f095528f8b71652ddacff7075e36e893)。这里的“已发布”指已核验的仓库 tag/Release；未另外下载或验证 npm 发布产物。

blob 比对使用 `gh api 'repos/earendil-works/pi/git/trees/a13d35a742c6ef8462812a28fbe1d8c8b7431c32?recursive=1'`，返回 `truncated=false`；筛选 `packages/telemetry/` 下 `type=blob`，按 `SHA1(UTF8("blob " + byteLength + NUL) + fileBytes)` 计算。关键结果：README `0c00dfd713ef47f704256a988e571d4b46fec0bb`；`src/index.ts` `3d39097917cefd905fa5e9b8bb8c9909a013d802`；`src/memory.ts` `fd5985a6d9111e22bbd1fa4661f5c5f6c9cbfe2f`。该比对只证明 telemetry 包的 12 个文件，不把它扩大成整个 monorepo 与 tag 完全一致的证明。此前 ai 包的独立基线见 [README](README.md)。

## TE-02：本地材料实际位置与文档有效性

本地 1.0.0 快照中没有单独保存的 telemetry RFC。当前材料和从官方历史恢复读取的材料必须分开：

| 材料及定位 | 主张与适用范围 | 文档主张状态 / 实现状态 |
|---|---|---|
| `packages/telemetry/README.md:3–13,62–132` | 通用 context/span、无默认 exporter、callback 所有权、诊断被动性 | 当前有效 / 已实现，见 TE-04、TE-05 |
| 同 README `:174–209,334–363` | memory adapter、conformance、typed schema 的用途及边界 | 当前有效 / 已实现；测试本次未执行 |
| 同 README `:326–332,369–383` | Agent root 仍导出 AI/harness schemas、helpers | **部分过时 / 相关 Agent 导出已移除**；通用包及 ai pass-through 部分仍有效 |
| 同 README `:387–391` | context 不持久化；敏感字段须遵循调用方数据策略；core 不使用 runtime-specific ambient API | 当前有效 / 已实现；策略声明不等于自动脱敏实现 |
| `packages/telemetry/CHANGELOG.md:31–35` | 0.84.0 引入 callback contract、NOOP、memory adapter、conformance、typed schema 和 composition | 当前有效 / 已实现；不是“0.84.0 以后才把 manual end 改 callback” |
| `packages/ai/src/types.ts:132–135`、`src/api/simple-options.ts:32`、`test/telemetry-options.test.ts:68–166` | 通用请求选项传播 parent | 当前有效 / 已实现；不证明 built-in adapter 创建 span |
| `packages/agent/CHANGELOG.md:7`、当前 `src/index.ts`、`package.json` | 1.0 去掉 experimental harness、schemas、telemetry reexports | 当前有效 / 已移除 |
| `packages/coding-agent/CHANGELOG.md:749` | 0.84 历史上加入 schema/helpers；链接至现已删除的 schema 文档 | 作为历史记录当前有效；作为当前导航部分过时 / 对应 API 已移除 |
| 历史 `packages/agent/docs/harness-v2.md`，固定 `6b461b75…` §18 | 拆包理由、schema owner、callback、无 ambient；明确当时尚无 runtime spans | 设计史证据有效；其整体 harness 目标已被替代 / 当时部分实现，现相关实验已移除 |
| 历史 `packages/agent/docs/telemetry.md`，固定 `5503486b…` | 每次调用的不可变 context、共享 receiver 不保留默认 caller、RPC trace 仍需设计 | 历史意图有效；整体文档已被替代 / 实验，1.0 已移除 |
| `packages/chord/README.md:59`、`PLANNING.md:87–89` | generic context keys 可承载应用 telemetry，Chord 本身不内建它 | 当前有效 / 没有据此证明 Pi 的 exporter 或 span 接线 |

telemetry 包 12 文件包括 README、CHANGELOG、package manifest、build tsconfig；`src/index.ts`、`noop.ts`、`memory.ts`；`src/testing/{index,types,conformance}.ts`；`test/{telemetry,conformance}.test.ts`。研究另读取 ai 的请求选项与测试、Agent 导出及 changelog、coding-agent 安装统计实现，并搜索 Agent、durable、coding-agent、ai、Chord 的当前生产源代码。没有将测试或 README 示例当生产消费者。

## TE-03：按已验证时间重建演进

| 时间（UTC）与提交 | 变化 | 当时落地程度与后续 |
|---|---|---|
| 2026-08-05 20:03:33，[`04d6447f7c492aafac97e2d2450b532650a85556`](https://github.com/earendil-works/pi/commit/04d6447f7c492aafac97e2d2450b532650a85556) | Mario Zechner 加入 typed telemetry contracts；通用契约最初位于 ai，harness schema 位于 Agent | 原始 `packages/ai/src/telemetry.ts:14–21` 已是 callback `startSpan`，无 public `end()`；契约、schema、NOOP、选项传播落地，业务 runtime instrumentation 未完成 |
| 2026-08-05 21:23:12，[`6b461b75b39b5a19b378dc42fbfbd1655bc446a6`](https://github.com/earendil-works/pi/commit/6b461b75b39b5a19b378dc42fbfbd1655bc446a6) | Mario 提取独立 `pi-telemetry`；通用契约不再归 ai；AI-request schema 移至 Agent | 同次修订明确 generic contract 与 domain schema 分工；ai 接受 parent，为后续 adapter 使用留入口，但不自行发 span |
| 2026-08-06 09:44:58，[`35f5c265d3d0af7eea3889dbfde03e77e23b8f11`](https://github.com/earendil-works/pi/commit/35f5c265d3d0af7eea3889dbfde03e77e23b8f11) | Mario 加入 reference adapter、runner-independent conformance、跨 schema typed starter 及显式 child starter | 核心契约可以由 stateful reference adapter 验证；独立 README 补充边界。旧 harness-v2 文档中“尚无 recording adapter”的段落未同步，不能继续照抄 |
| 2026-08-06，[v0.84.0](https://github.com/earendil-works/pi/releases/tag/v0.84.0) | 上述三次提交进入该 release | 提取时 package 暂仍写 `0.83.0` 是 release 前工作树版本，不足以声称 0.83 已发布此包 |
| 2026-08-06 至 08-12，[PR #7713](https://github.com/earendil-works/pi/pull/7713) | `StreamAssistant` / config 与 telemetry 的 harness-v2 接线尝试 | 曾有人类 approve，但 `merged=false`；作者 Cristina 于 08-12 明确因新设计而关闭，不能计入已合并能力 |
| 2026-08-17，[`397673eb26954984c0b18cc4ac019d8b60a5a736`](https://github.com/earendil-works/pi/commit/397673eb26954984c0b18cc4ac019d8b60a5a736)、[`5503486bdcd4b65f4b58d25be305f24fa5c7f911`](https://github.com/earendil-works/pi/commit/5503486bdcd4b65f4b58d25be305f24fa5c7f911) | invocation context / RPC / telemetry 设计说明及状态修正 | 原文标记 design input；共享 harness 不再持有默认 caller context；部分 tool hook span 已接入，其他 runtime spans 和跨进程 trace 尚未完成 |
| 2026-09-24 至 09-28，#10006/#10009、#10084/#10085、#10118 | exporter、classic Agent request spans、delivery metadata 提案 | 自动关闭、未获维护者确认的候选，不是路线图；详见 TE-09 |
| 2026-10-01 12:45:01，[`7fd478a2e888ebc28869566f33a186303d372838`](https://github.com/earendil-works/pi/commit/7fd478a2e888ebc28869566f33a186303d372838) | Mario 删除 experimental harness，连同 telemetry schema、generator、文档、测试及 Agent root reexports | 当日晚发布 v1.0.0；generic telemetry 包及 ai parent propagation 保留。durable sessions 迁往 pi-durable，但不能据此声称旧 telemetry schemas 一并迁入 |
| 2026-10-03，v1.0.1 及固定 main | telemetry package version/changelog 更新 | 本次比较窗口没有该包语义更新或 classic Agent 接线交付证据 |

对上述四个关键提交查询 GitHub `commits/{sha}/pulls` 均返回空列表。这只说明 API 未关联到 PR；其实际作者与仓库提交、变更内容可核，不应编造对应设计 issue。callback 并非本轮能观察到的“后续纠正”；真正的后续变化是 reference/conformance 与多 schema composition、再到 invocation context 的所有权调整。

## TE-04：为什么独立包、为什么不依赖 ambient context

**一手设计意图。** 提取提交的 [harness-v2.md:2766–2778](https://github.com/earendil-works/pi/blob/6b461b75b39b5a19b378dc42fbfbd1655bc446a6/packages/agent/docs/harness-v2.md#L2766) 明确分开三件事：通用 telemetry 契约；pi-owned 的领域 vocabulary；特定 backend 的导出与 trace/context 对象。ai 只依赖通用 parent 类型。该文档直接解释不以 `AsyncLocalStorage` 或某个 runtime 的 current-span API 作为 core 抽象的原因：Pi 同时面向 Node、Bun、browser、worker，不能让父子关系依赖某一运行时的 ambient 能力。

**当前实现。** `packages/telemetry/package.json:8–16,43–46` 导出 root 和 `/testing`，无 runtime dependency。`src/index.ts:14–22` 使用显式 context，`src/index.ts:324–353` 将 child starter 绑定到 callback 的 span。README `:132` 允许 adapter 内部激活 backend-native ambient context，以便 HTTP 自动 instrumentation；禁止的是 Pi core 靠隐含全局对象决定 parent，不是禁止所有 backend 的 ambient 工具。manifest 声明 Node engine 下限，README 的跨 runtime 声明不等于本次实际做了所有平台兼容验证。

**重要的归因限制。** 能确认的拆包意图是避免 generic contract 与 AI/harness domain 耦合，并把 backend 实现留给应用 adapter；“为了更快加载”“为性能减少依赖树”不是该提取提交明说的目的。无 runtime dependency 是事实，性能收益未测。独立 npm package 也不直接证明 Rust 必须独立 crate：需要保留的是职责、依赖方向及显式所有权；包装形式属于 Rust 目标设计。

**状态：** 文档主张当前有效；通用契约已实现；分类为稳定核心（相对于本次 1.0 基线），置信度高。这不是承诺未来 API 永不变化。

## TE-05：callback context 的可观测契约

依据 [当前 README:62–132](https://github.com/earendil-works/pi/blob/a13d35a742c6ef8462812a28fbe1d8c8b7431c32/packages/telemetry/README.md#L62)、`src/noop.ts:3–20`、`src/memory.ts:86–185` 和公开 conformance suite，callback 用于把 span 的生命周期与业务调用绑定，避免调用方管理手动 `end()`。它同时保证在 span 创建失败时，业务 callback 仍恰好执行一次。

| 可观测语义 | 实现与已有离线测试边界 | 不可扩大解释 |
|---|---|---|
| callback 同步准入、恰好一次，返回 `Promise` | NOOP `:3–9`；memory `:168–185`；conformance `:65–140` | Rust Future 的惰性执行不必模仿 JS 调用时机；必须另定相应 Rust 准入/完成边界 |
| 原业务返回值与失败值保持；同步 throw 变为同一失败值的 rejection | 同上；`test/telemetry.test.ts:172–183` | 不吞业务错误，不为了生成 span 重跑业务 |
| callback 所返回 promise 完成后 span settled | memory `:94–101,175–185` | callback 仅返回一个 stream 对象时，不能自动证明 span 覆盖后续全部 stream 生命周期；新 instrumentation 必须绑定实际完成边界 |
| 默认成功/失败状态；显式状态最后写入优先，自动完成不覆盖它 | memory `:94–98,158–165`；conformance `:142–181` | “返回错误对象”不等于 throw；调用方须显式标状态。不能把某次显式 `ok` 后业务失败判为 library 自动改写 |
| attributes 合并，undefined 不删除；事件有序 | memory `:61–68,141–156`；conformance `:184–218` | 不证明未知字段自动验证、敏感字段自动脱敏 |
| telemetry recording 失败无副作用、按一次记录原子忽略 | memory `:128–166`；conformance `:206–218,274–320` | 这是 adapter 合规义务；ai pass-through 没有替所有任意外部 adapter 提供防错包装 |
| settled 后记录无效；此后 start child 仍运行业务 callback，但不再记录新 child | memory `:126`；conformance `:220–244` | 不是拒绝或取消迟到业务，也不是让它复活父 span |
| 显式 parent 支持并发 sibling，各自 parentId 与 settlement 顺序可观察 | conformance `:246–272` | 此 suite 的并发父子测试不等于任意共享 product receiver 的上下文隔离已验证 |

NOOP 复用 frozen inert object，memory 使用自增数字 ID、end sequence、没有 wall-clock timestamps，并无限累积 process-local spans。这些是具体适配器及测试实现选择，不能把它们当生产遥测存储策略。memory 返回 detached snapshots，适合确定性测试，不是持久化日志。

**状态：** 通用 adapter 文档当前有效 / 已实现；已有测试只作静态核读，本轮没有执行。通用 lifecycle/passivity 是稳定核心；共享 JS 对象、数字 ID、具体 type-helper 形式属于实现选择。

## TE-06：typed schema 的目的与边界

`src/index.ts:27–79,123–321,345–353` 定义 serializable schema 和 TypeScript 推导；`defineTelemetrySchema` 是 identity function，`createTypedSpanStarter` 不保留、遍历或运行时验证传入 schema。其作用是让 span names、必需 start/event attributes、closed-set values、optional completion enrichment、跨 schema 同名冲突在类型层可检验，并让文档/测试复用同一 vocabulary。见 [README:334–363](https://github.com/earendil-works/pi/blob/a13d35a742c6ef8462812a28fbe1d8c8b7431c32/packages/telemetry/README.md#L334)。

`test/telemetry.test.ts:31–153` 包含 inference、JSON serialization、unknown/missing/closed-set、duplicate-name 的负向类型案例与 child parentage 案例。`@ts-expect-error` 的存在不等于本次已运行 typecheck。`sensitive`、`cardinality` 是元数据；parent rules 是描述性数据；都不自动执行数据策略或约束运行时 parent。

历史 [harness-v2.md:2778](https://github.com/earendil-works/pi/blob/6b461b75b39b5a19b378dc42fbfbd1655bc446a6/packages/agent/docs/harness-v2.md#L2778) 说明 pi-owned vocabulary 与 backend semantic conventions 分开，以免 backend 约定变化迫使 core vocabulary 跟着变化。可以借鉴这一职责分离；不能从中得出旧 `pi.harness.*`/`pi.session.*` schema 必须在 opi 复建。那些具体领域 schema 已随实验删除。

**状态：** generic schema 工具当前有效 / 已实现；旧 AI/harness schema 的现行 API 主张已被替代 / 已移除。置信度高。

## TE-07：pi-ai 及真实产品路径究竟接入了什么

| 边界 | 当前可证事实 | 验证与缺口 |
|---|---|---|
| SDK request options | `ProviderRequestOptions.telemetryContext?: TelemetryContext`，stream/deferred/image/classifier options 继承它；simple-options 显式保留同一对象 | `types.ts:132–135,187,241–250,324,333`、`api/simple-options.ts:32` |
| Provider / Models / direct image dispatch | mock adapter 收到 caller 的同一个 parent object | `test/telemetry-options.test.ts:68–166` 覆盖 stream、simple stream、fetch/cancel deferred、direct/Models image；该文件没有 classifier dispatch 专项断言 |
| built-in provider instrumentation | 本次在所有 `packages/ai/src` 搜索只找到声明和传播；没有 `startSpan`、schema/helper 或 exporter 调用 | 这是当前内建源码接线结论，不否认自定义 provider 可消费此上下文 |
| classic Agent、coding-agent SDK/CLI、pi-durable | 当前生产源中没有 `startAiSpan`/`startHarnessSpan`/`TelemetryContext` 的 callback telemetry 接线；pi-durable 没有接收旧 Agent schema 导出 | 已读各包当前源搜索与 Agent root/manifests；包外第三方消费者未枚举 |
| telemetry package 自身 | 独立公共契约、NOOP、memory adapter、typed starter、conformance 都可供外部 SDK 使用 | “产品不默认接线”不等于“能力不存在” |
| coding-agent 安装统计 | `src/core/telemetry.ts`、interactive startup、provider attribution 实际消费的是 install telemetry 设置 | 与 callback span contract 是两套行为；见 TE-10 |

这是稳定的 SDK 扩展契约加已交付传播路径，不能降格为“只有文件、没有能力”。但若目标是在 opi-ai 自动创建 logical-call/attempt spans，须把它明确写成本次选择的 instrumentation 范围、字段与完成条件；不能声称这是 pi 1.0 内建 adapter 的默认同构行为。

## TE-08：后续 invocation context 改动及 1.0 删除的含义

历史 [telemetry.md:7–14,56–71](https://github.com/earendil-works/pi/blob/5503486bdcd4b65f4b58d25be305f24fa5c7f911/packages/agent/docs/telemetry.md#L7) 直接解释后续问题：同一个 harness/session/lane receiver 会服务并发本地或 RPC 调用，receiver 上的 mutable/default caller context 无法同时表示多个 parent。方案是每次调用传不可变 context，派生 child context，业务共享对象不保留默认 caller；当时 `AgentHarnessOptions.telemetryContext` 已移除。

这条历史支持“并发调用需要明确 parent owner”，不支持“所有 SDK 操作必须引入旧 harness 的通用 Context 容器”。该文档自己在 `:3` 标明设计输入及落地状态；`:129–133` 只确认部分 tool hooks 的 child spans，其余 hooks、events、session write spans 仍待做；`:152,202–219,255–259` 将 links 和 trace carrier owner 列为尚未解决设计，不是已实现 W3C 传播。

[删除提交](https://github.com/earendil-works/pi/commit/7fd478a2e888ebc28869566f33a186303d372838) 明确删除 `docs/telemetry.md`、`docs/telemetry-schema.md`、schema generator、`src/harness/telemetry.ts` 及对应 tests，并改变 Agent 包范围。该动作可证明旧 experimental harness 退出，不能证明作者反对一切 typed schema、被动诊断或显式上下文：telemetry 通用包与 ai parent 传播都保留。

README 当前关于 Agent schema 的段落因此部分过时。反过来，旧 released CHANGELOG 应视为历史记录；不该为了当前目标而改写旧 release 记录，也不应把其失效链接误当当前导出。置信度高。

## TE-09：官方 issue / PR 的接纳状态

本次读取下列 GitHub API 的正文、全部 issue comments、相关 PR reviews；未发送任何消息。提案人的自述测试数字未被本研究当作实际测试结果。

| 来源 | 提案/反馈与当前状态（访问日） | 对目标的约束 |
|---|---|---|
| [#7713](https://github.com/earendil-works/pi/pull/7713) | collaborator 的 harness-v2 StreamAssistant telemetry 接线；存在 vegarsti 的 APPROVED review，作者 Cristina 后来明确 [“Closing in favor of new design”](https://github.com/earendil-works/pi/pull/7713#issuecomment-5265813015)；closed、`merged=false` | 曾被看好不等于合并；不能采信后来提案人声称“因 merge conflicts 关闭”的二手描述 |
| [#10006](https://github.com/earendil-works/pi/issues/10006)、[#10009](https://github.com/earendil-works/pi/pull/10009) | 第三方提出独立 pi-otel exporter、OTEL 环境变量/批处理/flush。issue 和 PR 均自动关闭；PR `merged=false`、`merged_at=null`，无 review；comments 只有机器人和提案人 | 未获维护者明确接受；不是官方计划、不是现成 exporter。提案人解释自身分包原因不等于原作者意图 |
| [#10084](https://github.com/earendil-works/pi/issues/10084)、[#10085](https://github.com/earendil-works/pi/pull/10085) | 第三方为 classic Agent、createAgentSession 增加 context 注入与 `pi.ai.request` spans。issue/PR 自动关闭，PR `merged=false`；唯一 review 是 [Copilot bot COMMENTED](https://github.com/earendil-works/pi/pull/10085#pullrequestreview-5327456217)，指出 child propagation/chunk counting 问题 | 没有人类维护者接受方向的证据；其逻辑请求 instrumentation 不能标为已发布或已承诺 |
| [#10118](https://github.com/earendil-works/pi/issues/10118) | 第三方请求 delivery-context attributes；自动关闭，comments 只有机器人，`state_reason=not_planned` | 仅提案，不建立新的 metadata framework 目标 |
| [#9704](https://github.com/earendil-works/pi/issues/9704) | 第三方请求 transport-neutral rate-limit hook；自动关闭，comments 只有机器人 | rate-limit hook 的存在需求不证明 callback telemetry 包已提供此操作 |
| [#3028](https://github.com/earendil-works/pi/issues/3028)、[#3032](https://github.com/earendil-works/pi/issues/3032)、[#3036](https://github.com/earendil-works/pi/issues/3036) | 2026-04-11 第三方 OTEL task series 中的 types/Agent/SDK 任务；API 标 closed/completed，但评论显示 OSS weekend 机器人自动关闭 | 不能用 `completed` 状态证明交付，也未找到该任务系列与 8 月 package extraction 的作者因果关系 |

对 10006/10084/10118 等，`not_planned` 是当前 API 状态；机器人解释的流程性 auto-close 不能等同维护者对技术方案作出的实质否决。相应地，没有回复也不能当默认批准。四个实际核心提交没有关联 PR，当前可核的作者意图来自提交及随提交修订的设计文档，而不是上述未接受提案。

**状态：** 提案状态可确认 / 仅提案；不能确认未来会采用或何时采用。置信度高；缺口是未找到进一步的维护者接受说明。

## TE-10：同名官方 RFC 是安装统计，不是 span 契约设计书

[官方 Pi RFC 索引](https://rfc.earendil.com/keyword/pi/) 实际列出 [RFC 0019 Pi Telemetry](https://rfc.earendil.com/0019/) 与 [RFC 0038 Pi Analytics](https://rfc.earendil.com/0038/)。两者作者均为 Armin Ronacher，公开页面的创建/更新时间分别为 2026-04-14、2026-05-29；页面没有本次可验证的 Git revision，因此只记录访问日，不伪造固定 commit。

| RFC | 主张、实现证据与有效性 |
|---|---|
| 0019，页面标签 Implemented | 为了解安装/升级采用情况，在 startup/changelog 检查旁发送 version ping。正文显式说已被 0038 取代。当前 `interactive-mode.ts:1325–1362` 确有安装/升级 ping、offline/setting gate、5 秒超时与忽略失败，但实际 endpoint 是 `https://pi.dev/api/report-install`，不是旧 RFC 的 `/install`。所以原文作为现行完整规范已被替代；基础行为已实现但有修订 |
| 0038，页面标签 Discussion | 提议 opt-in analytics ID、可变实验、`/privacy` 展示及设置控制，并讨论匿名 update 请求。当前源码的 install ping 和 User-Agent 不能证明该整套 opt-in 机制已实现。本次未找到它承诺 pi-telemetry callback API、exporter 或 Agent span 的证据；文档是可确认提案，其整套实现状态证据不足 |

`coding-agent/src/core/telemetry.ts:8–12` 的 `PI_TELEMETRY` 控制安装统计；`core/provider-attribution.ts:40` 也使用该设置决定归因 headers。它不是 generic span context 的全局开关。候选 spec 不应因为对齐 pi-telemetry 而自动引入安装跟踪、外发 endpoint、用户 ID、OTEL env 或 `/privacy` 产品流程。

## TE-11：当前规划可说什么、不可说什么

固定 main `83692682…` 的比较窗口仅确认 telemetry 包仍保留原有通用契约，README 的旧 Agent 导出说明也没有随 1.0 修复。没有发现本次窗口中新合并的 exporter、classic Agent spans、distributed carrier 或 schema owner 恢复。不能将目录仍存在、提案已有代码、PR 可得到 merge commit 值，或 issue 关闭，推导为接受/发布。

有一手依据的演进方向是：通用契约从 AI 包抽离；领域 schema 曾归 runtime owner；上下文趋向每次调用显式传递；旧 experimental harness 后来被移除。这是已发生的历史。推断“上游最终会重新接通所有 spans”“下一版将有 pi-otel”或“旧 Agent schema 会全部迁到 durable”均没有足够证据，本报告不作这些承诺。

## TE-12：对 opi 候选目标的研究含义

以下是证据支持的判断，不是新增无条件 MUST，也不代替 [decisions.md](decisions.md) 的人类确认。

| 值得保留/对齐的意图 | 可接受的 Rust 差异及不能自动升级的事项 |
|---|---|
| 调用方显式提供 parent；同一实例上的并发请求互不改写 parent | 可用 Rust 的显式 context/ownership 表达；不必复制旧 harness 的 `ContextKey` 容器、JS callback 签名或独立 npm 包边界 |
| 诊断不能决定业务准入、返回值、失败、持久化结果；导出失败不能重试业务 | 应单独定义 Rust 错误/panic/cancel 的适用边界；不能声称任意外部 observer 的故障已由 pi pass-through 自动隔离 |
| span 覆盖真实 operation 生命周期，child context 随并发任务显式传播 | 流构造完成不是流终止；retry attempt 与 logical call 区别需要候选 spec 明确。新增自动 spans 是本次目标选择，不能伪称 pi 已默认交付 |
| generic observation 与业务 payload、durable state、backend exporter 分离 | backend/collector 的安装配置、发送、flush 由明确 owner 负责；不默认新增 exporter、paid service 或网络行为 |
| 记录 vocabulary、类型、敏感性边界和机械验收 | 可用 Rust 类型与本地 subscriber/fixture 验证；不需要照搬 TypeScript schema compiler，也不能将旧已移除领域 schema 当权威 vocabulary |

最小可借用的测试思想包括：无 telemetry 时业务照常；parent/child 与交叉并发隔离；正常完成、业务错误、取消、异常 EOF 仅结束对应作用域；禁止在 logging/export failure 时重复调用 provider；默认诊断不包含 prompt、tool payload、credentials；上下文不进入 transcript/持久化。pi 自带 conformance 直接证明的是它自己的 callback adapter 语义；Rust 流取消、subscriber 及 spawn 行为需要独立测试，本文没有验证。

## TE-13：执行记录、限制与测试影响

实际完成的只读检查：

- `agent-reach doctor --json`：exit 0；GitHub `gh` 可执行、有配置但 doctor 不做 auth 实时核验。随后本任务 GitHub API 请求成功。
- `firecrawl --status`：exit 0；本地 CLI 可执行，account-info fetch 失败；未发任何 scrape/search/extract 请求，未消耗抓取额度、安装依赖或生成凭据。
- `gh api 'repos/earendil-works/pi/commits?path=packages/telemetry&per_page=100'` 及 `packages/ai/src/telemetry.ts`、`packages/agent/src/harness/telemetry.ts`、`packages/agent/docs/telemetry.md` 的 path history：读取提交、时间及变更。
- `gh api repos/earendil-works/pi/commits/{sha}`、`commits/{sha}/pulls`、`contents/{path}?ref={sha}`：核验本报告固定提交和历史设计文本；没有 checkout、pull 或 Git 写入。
- `gh api repos/earendil-works/pi/git/ref/tags/{tag}`、`releases/tags/{tag}`、`compare/{base}...{head}`：核验三个 tag/Release 及快照后变化。
- `gh api 'search/issues?q=repo:earendil-works/pi+telemetry+in:title&per_page=100'`、相关 issue/PR/comments/reviews：核验 TE-09。一次组合 reviews 命令的 PowerShell 引号错误导致 exit 1，随后改为逐 endpoint 读取全部成功；没有把失败请求当空 review 证据。
- 官方 RFC 索引及 0019/0038 经 web 只读获取；本地 `rg` 检索及完整源码阅读；telemetry 12 文件 Git blob 对比通过。
- `python scripts/opi-doc-check.py`：exit 0，`opi documentation contracts: PASS`。
- 本轮 `git diff --check`：exit 2；唯一诊断路径为本任务未修改的 `docs/research/pi-1.0.0/source-manifest.json`，11720 行 trailing-whitespace 诊断，原文件保留。对本文执行 `git diff --no-index --check -- /dev/null docs/research/pi-ai-opi-ai-alignment/telemetry-evolution.md` 没有 whitespace 诊断；exit 1 表示新文件存在差异。没有将全局检查记录为通过。

没有运行 JS/Rust 测试、typecheck、build、实时模型请求或 exporter。上述测试描述全是静态证据/未来验收设计。本次测试影响：`none`（只新增文档）；候选实现的测试增删改由主 acceptance matrix 记录，本文不创建第二份验收权威。

剩余缺口：未核 npm tarball；没有浏览或请求非公开 RFC；未枚举第三方生态 adapters/consumers；没有找到 package extraction 的另一个原作者解释 issue；RFC 0038 完整落地状态未作全面产品审计。它们不影响“核心 contract 与旧 harness 目标必须分开”的结论，也不允许把未知后续写成已承诺计划。

当前目标指针（2026-10-04）：Q37 已由人类确认，见 [D-37](decisions.md#d-37q37pi-的显式-telemetry-传播以-rust-tracing-模块落实) 与 [telemetry-design.md](telemetry-design.md)。本批采用 opi-ai 显式传播加 Rust tracing 模块；宿主拥有领域 instrumentation/backend，不新增自动 SDK call/attempt spans、旧实验 Agent schema、exporter 或 schema DSL。TE-12 对新增 instrumentation 的讨论是未来改范围时的条件，不再代表本批仍待回答的问题；AIR-054–056 和 V-46–48 是本批唯一目标与验收入口。
