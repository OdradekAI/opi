# Pi 1.0.2 设计文档与规划时效核查

> 核查日期：2026-10-05。对象：本地 `.repo/pi-1.0.2` 固定快照。
> 本文补充 [整体报告](README.md) 与 13 份包报告，只分析 Pi 自身。文档中的未来意图以本快照为准，不代替上游后续版本或外部 RFC 的核查。

## 1 本版的复核目标

1.0.0 报告已在上一轮补齐设计规划与演进分析。本次更新到 1.0.2：先确认材料是否变化，再检查实现变化是否使旧结论失效，继续分别分析设计动机、已兑现计划、尚未完成方向、替代决策、非目标和历史实验。

本轮有 **26 份文档内容变化、99 份字节一致**。一致材料继承旧版审读证据，不声称本轮全部重新逐行阅读；关联代码变化时仍重新判断时效。例如 MCP 测试指南没有变化，但“不使用 CIMD”的说明已被新实现超越。版本比对的方法和限制见 [升级分析](version-changes.md)。

## 2 文档范围与证据方法

本地文档共 **125 份、33,781 行**，其中 packages 下 **111 份**。扫描 `.md/.mdx/.rst/.txt`，排除 `.git/node_modules/dist`。清点和变化识别覆盖全部文件；变化说明文档重新审读，长 CHANGELOG 重点读新增版本和相关历史段落。继承和复核深度逐文件列于 [文档索引](documentation-index.md)，不将“列入清单”写成“逐行审计”。

六份核心材料共 **8,150 行**；durable spec 增至 4,628 行，另外五份字节一致。沿用上一轮全文阅读与逐项分析，本轮重新检查 spec 的现行契约及变化实现：

| 文件 | 行数 | 独立价值 |
|---|---:|---|
| [Chord PLANNING](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L1) | 859 | 位置透明 services、replicated state、facet 生命周期、WP0–WP8 与未完成 peer/structural reload |
| [durable spec](../../../.repo/pi-1.0.2/packages/durable/docs/spec.md) | 4,628 | 提交权威、所有权、records/documents/tasks、恢复、storage 契约及非目标 |
| [durable handoff](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L1) | 917 | 分阶段落实和后续替代决策；阶段编号不是 npm 包编号 |
| [durable Chord usage](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-chord-usage.md#L1) | 387 | 从持久文档到服务观察面的接线、生命周期与可执行示例 |
| [durable delta findings](../../../.repo/pi-1.0.2/packages/durable/docs/chord-delta-findings.md#L1) | 358 | 被否决实验、所有权/别名问题、性能测量方法与重试研究的证据要求 |
| [根目录 tui-plan](../../../.repo/pi-1.0.2/tui-plan.md#L1) | 1,001 | 两种屏幕模式的物理约束、布局和输入契约、阶段验收与后续方向 |

coding-agent 的三个 experimental README 字节一致，保留上一轮逐项 TODO 核查；本轮另全文复核变化的 README 和七份使用文档，重读新增发行记录，并追踪会被新实现影响的旧说明。其余示例、测试夹具和使用指南的历史阅读深度逐项保留，不把继承数量当作本轮重新阅读数量。没有执行文档里的模型调用、构建、benchmark 或测试命令。

### 2.1 不用单一“过时”标签

| 状态 | 判定要求 | 不应推导的结论 |
|---|---|---|
| 已实现／已兑现 | 当前入口、实现与相关测试预期存在；必要时用发行记录确认演进 | 不等于本次执行过测试，也不等于适用于所有平台 |
| 部分实现 | 把已经存在的机制和未满足的具体部分拆开 | 不把整个包都说成未完成 |
| 尚未实现的文档方向 | 文档明确提出，快照没有对应完整机制 | 不自动变成近期交付承诺或下一版本内容 |
| 已被替代／局部陈旧 | 有后续设计、删除记录、实际导出或实现与其冲突 | 不给整份文档判废 |
| 明确非目标／职责边界 | 文档明确排除或交给调用方、adapter、宿主 | 不把“没有”直接写成缺陷或待办 |
| 历史实验／性能目标 | 记录某次条件下的测量、被拒绝方案或验收目标 | 不当作当前性能实测、服务等级保证或仍存在的 bug |
| 证据不足 | 缺当前实现、结果、环境或后续作者确认 | 不擅自选择“仍承诺”或“已经放弃” |

实现回答“现在如何运行”，设计文档回答“原本想解决什么”，CHANGELOG 回答“何时宣称改变”。测试源码说明预期行为，只有实际执行结果才能说明本次验证通过。三类材料出现冲突时，报告保留冲突并指出依据，不使用文件名、mtime、版本号或单个 Done 复选框代替判断。

## 3 13 个包的文档核查结果

| 包 | 包内文档数 | 设计／规划覆盖与主要结论 | 详细报告 |
|---|---:|---|---|
| ai | 2 | 新增分 thinking level 采样与 provider 修正；Provider/Models、SystemMessage、多操作模型重构已实现；compat 移除仍是无日期未来意图，旧 replay 概括需按实现限定 | [ai](packages/ai.md) |
| agent | 2 | 当前循环契约和历史边界调整；1.0.0 已移除 Harness/pico3/telemetry schemas；旧 API 和角色 filter 不能照抄；没有独立未发布路线图 | [agent](packages/agent.md) |
| chord | 3 | PLANNING 的 WP0–WP8 完整矩阵；state/service/facet/bundle 主体已落地；对称 peer 和完整 structural generation replacement 仍未完成；多处旧术语/概括需按具体实现限定 | [chord](packages/chord.md) |
| client | 2 | 当前 Session client 手册与早期 transport/client 历史；没有独立路线图；旧 client API 的演进不等于当前遗漏，重连与恢复边界交给宿主 | [client](packages/client.md) |
| codemode | 2 | 现行 VM、host capability、store 契约新增输出字符/项数限制；没有独立路线图，无 timer/fetch/process 等仍是刻意边界 | [codemode](packages/codemode.md) |
| coding-agent | 79 | 使用文档加入 MCP 项目覆盖/CIMD、工具 renderer 和采样参数；实验 TODO 仍有局部过时，CIMD 测试指南也落后；SDK 快速参考旧导入仍需限定 | [coding-agent](packages/coding-agent.md) |
| durable | 6 | spec 新增持久 provider session；handoff 1–23 和历史实验保留，尚未实现旧状态句仍陈旧；额外工具/缓存友好摘要方向未因此完成 | [durable](packages/durable.md) |
| evals | 1 | README 描述已实现的成对实验协议；无独立路线图；统计、网络与隔离限制不是未来承诺 | [evals](packages/evals.md) |
| mcp | 3 | README、发行记录与许可证；OAuth 示例未转交 `iss`，在特定 issuer 校验条件下不完整；supported surface 遗漏已实现 resources；无独立路线图 | [mcp](packages/mcp.md) |
| protocol | 2 | wire 协议使用说明和移除旧协议的历史；没有独立路线图；传输中立与版本检查属于当前边界 | [protocol](packages/protocol.md) |
| server | 2 | router/SessionHandle/Node adapter 分工与发行历史；没有独立路线图；接入 runtime、认证策略和重建 handle 主要由宿主负责 | [server](packages/server.md) |
| telemetry | 2 | vendor-neutral 契约仍有效；Agent schema 所有权说明已被 1.0.0 移除记录证伪；backend/exporter 交由 adapter，不是 core 待开发项 | [telemetry](packages/telemetry.md) |
| tui | 5 | 包 README/CHANGELOG/native 文档，加包外 tui-plan；主要布局和曾延期的滚动条已落地，virtualization 和独立 overlay layout 等仍留在后续方向 | [tui](packages/tui.md) |

包内文档数量合计 111；TUI 所依赖的根计划另计入根目录 14 份。大量说明文档提供的是当前接口、使用边界和设计动机，而不是未来功能清单。即使一个包没有 `PLANNING.md`，它的 README 和 CHANGELOG 仍有必要分析。

## 4 最重要的时效修正

### 4.1 durable 的“尚未实现”不是当前事实

spec §2.2 仍写 Pico5 尚未实现，同时 handoff 顶部已经称阶段 1–23 完成，包的 1.0.0 CHANGELOG 与 Harness 源码也已存在。应判定**这一状态句陈旧，核心设计并未失效**。handoff 里“pico3 remains”的说明则被 Agent 1.0.0 的明确移除记录替代。[spec 状态句](../../../.repo/pi-1.0.2/packages/durable/docs/spec.md)、[handoff 状态](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L10)、[Harness](../../../.repo/pi-1.0.2/packages/durable/src/harness/harness.ts)、[移除记录](../../../.repo/pi-1.0.2/packages/agent/CHANGELOG.md)。

还必须按后续修订解释前面的阶段：第 19 阶段以 structured concurrency 替换 `after/pi.post-tools`；第 21 阶段以 named extensions、`pi.agent` 和 live settings 替换旧 registry/config 接口。当前代码采用后者，因此旧阶段仍解释历史问题，但不能作为当前 API 实施清单。[阶段 19](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L606)、[阶段 21](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L825)、[当前 Agent 接口](../../../.repo/pi-1.0.2/packages/durable/src/harness/agent.ts#L36)。

handoff 第 22 阶段的 bug 列表也不能直接当作现存问题：now/report 的调用期检查、migration-only publication 和 open 失败清理都有后续实现与针对性测试。逐条闭合证据已放在 durable 子报告中。

### 4.2 TUI 的未来清单已经部分兑现

计划说可见 scrollbar 以后再做，但 0.84.0 已记录滚动条和跨屏选择，当前 ScrollView 与 renderer 中都有相应代码。计划保守的“先只支持可见区域选择”也已被边缘自动滚动扩展。因此仍有研究价值的是布局动机、状态归属和尚未完成的后续方向，而不是继续把 scrollbar 列为待办。[旧预留](../../../.repo/pi-1.0.2/tui-plan.md#L205)、[发布记录](../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md)、[ScrollView](../../../.repo/pi-1.0.2/packages/tui/src/components/scroll-view.ts#L1)。

另一方面，ScrollView 仍测量并保存完整内容行，屏幕裁剪不是虚拟化；overlay 仍调用普通 `render(width)` 后合成，并未为每个 overlay 建立自己的受约束 frame。这两项不能因基础布局已经完成而一并标成实现。[虚拟化方向](../../../.repo/pi-1.0.2/tui-plan.md#L99)、[内容测量](../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L136)、[overlay 后续阶段](../../../.repo/pi-1.0.2/tui-plan.md#L631)、[现行 overlay](../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L1339)。

计划还帮助发现了一个具体实现差距：`contain` 原本应停止剩余滚动量传播，但当前路由在 break 后仍可将余量交给尚未访问的 primary。非 primary 的 contain 区域到达边界、且 primary 尚可滚动时，不能保证外层完全不动。这是静态可见偏差，报告保留证据；本轮未运行复现，也未修改代码。[contain 要求](../../../.repo/pi-1.0.2/tui-plan.md#L531)、[路由分支](../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L986)。

### 4.3 Chord 的未完成项有明确范围

当前 README 仍把 symmetric RPC peer 写成 remote service boundary 的一种可选计划实现。现有 services 通过 `RemoteServiceTransport` 工作，不能因 Pi client/server 有请求关联和取消，就认定 Chord WP2 已经实现。[README](../../../.repo/pi-1.0.2/packages/chord/README.md#L47)、[WP2](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L649)、[公开导出](../../../.repo/pi-1.0.2/packages/chord/src/index.ts#L1)。

现有 reload 要求相同 facet ID 和依赖/服务形状；规划中的新增、删除及重建服务图仍不是公开支持的 structural replacement。该缺口不同于“不能热重载任何代码”。同样，旧 `PluginLoader` 草案已经演变为 `FacetLoader`，文件拆分不同不应误判成能力缺失。[结构替换计划](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L229)、[实际 reload](../../../.repo/pi-1.0.2/packages/chord/src/facets/host.ts#L423)、[当前类型](../../../.repo/pi-1.0.2/packages/chord/src/types.ts#L303)。

### 4.4 文档局部矛盾不只出现在规划稿

telemetry 的 Agent schema 导入示例与已经删除的导出冲突；Agent README 低层 filter 示例没有保留现在已存在的 system 消息；AI README 对跨 provider thinking/partial 的概括落后于实际 transform 条件。这些材料可以解释演进，但不能直接复制为当前调用代码。[telemetry 旧导出](../../../.repo/pi-1.0.2/packages/telemetry/README.md#L365)、[Agent 删除记录](../../../.repo/pi-1.0.2/packages/agent/CHANGELOG.md)、[Agent 旧 filter](../../../.repo/pi-1.0.2/packages/agent/README.md#L541)、[AI transform](../../../.repo/pi-1.0.2/packages/ai/src/api/transform-messages.ts#L64)。

durable README 将“100 ms 节流”解释为“崩溃最多丢失该窗口”也过强：generation 受在途提交和存储速度影响，tool output 还按输出量限制写入频率。这是文档保证范围的问题，不能提升为固定的恢复点上限。[README](../../../.repo/pi-1.0.2/packages/durable/README.md)、[generation](../../../.repo/pi-1.0.2/packages/durable/src/harness/generation.ts)、[output](../../../.repo/pi-1.0.2/packages/durable/src/harness/output.ts#L274)。

### 4.5 历史实验保留方法，不保留为现版数字

`chord-delta-findings.md` 记录了被移除的 graph tracker 实验以及当时 path tracker 的 clone/cache 方案。当前 Chord 已是 immutable revision、revocable draft、prepare/adopt 的模型；trusted root、placement copy 与 alias-free tree 契约也改变了问题边界。旧 heap/耗时数字不能直接当作现版性能，旧 alias 问题不能脱离当前输入契约判断。[历史实验](../../../.repo/pi-1.0.2/packages/durable/docs/chord-delta-findings.md#L3)、[当前 ownership](../../../.repo/pi-1.0.2/packages/chord/src/delta/README.md#L96)、[当前生命周期](../../../.repo/pi-1.0.2/packages/chord/src/delta/README.md#L124)。

该文档仍应保留在分析中，因为它说明为什么一次更复杂的方案被拒绝，以及下一轮实验必须测量初始化、读取放大、编辑、序列化、回收和 identity，而不仅是最后输出是否相等。[后续研究的证据要求](../../../.repo/pi-1.0.2/packages/durable/docs/chord-delta-findings.md#L347)。

### 4.6 1.0.2 新增的契约与新暴露的旧说明

**durable：规范更新和旧状态句并存。** `pi.provider` 保存独立于 conversation ID 的持久 provider session UUID，fork 使用 initial、checkpoint 保留、旧会话在请求前补建。generation/compaction 都接入，但 compaction 的 `cacheRetention: "none"` 与 handoff 的缓存友好摘要候选仍然存在；不能把 session affinity 改进等同于该规划已经完成。[新文档实现](../../../.repo/pi-1.0.2/packages/durable/src/harness/provider.ts#L6)、[新发行条目](../../../.repo/pi-1.0.2/packages/durable/CHANGELOG.md#L7)、[逐项分析](packages/durable.md)。

**MCP：CIMD 已是可选实现，旧 conformance 说明不能代表当前能力。** 产品文档和实现提供 `oauth.clientRegistration: "cimd"`，但测试指南仍写 Pi 不使用 Client ID Metadata Documents。它可以描述旧 baseline 的形成背景，不能继续用作本版不支持 CIMD 的证据。相反，MCP 包 README 中只解构 code 的回调示例仍未转交 issuer；callback 能返回 iss 不会替调用方自动传入 authorize。[新配置文档](../../../.repo/pi-1.0.2/packages/coding-agent/docs/mcp.md)、[旧测试说明](../../../.repo/pi-1.0.2/packages/coding-agent/test/mcp-conformance/README.md#L42)、[包示例](../../../.repo/pi-1.0.2/packages/mcp/README.md#L99)、[分析](packages/mcp.md)。

**codemode：README 里的输出上限是现行约束。** 16 Mi 字符与 100,000 项限制已有 prelude 检查和失败测试源码；它们不再是分析者指出的空白，也不等于宿主整体内存有硬上限。[输出约束](../../../.repo/pi-1.0.2/packages/codemode/src/runtime/prelude-source.ts#L30)、[包报告](packages/codemode.md)。

**TUI：格式支持说明缺少 Kitty 转码前提。** README 列出 PNG/JPEG/GIF/WebP，但未说明 Kitty 路径的非 PNG 图片依赖已注册的 `setImageTranscoder`；库独立使用时无 converter 或返回 null 会退回占位显示。产品已接线与库在任意宿主中自动支持四种格式是不同结论。[README](../../../.repo/pi-1.0.2/packages/tui/README.md#L682)、[转换分支](../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts#L101)、[包报告](packages/tui.md)。

**安装文档：普通 npm 与托管安装已分开。** shrinkwrap 被移除，install-lock 仍服务托管安装和 Nix；Nix 的固定 catalog 构建输入与运行时目录更新也不同。README 使用 `v1.0.0` 作为固定 tag 的示例，不应仅凭该示例数字认定整页陈旧。Nix workflow 三个平台实际构建与四个平台声明支持的差别见 [交付分析](version-changes.md)。

## 5 文档明确留下的后续方向

下表只汇总有文档依据的方向、未支持项或延期候选；它们**不是统一的已批准待办列表，也没有共同的完成日期**。包报告进一步说明具体实现、已有相邻能力和不确定性。

| 方向 | 当前状态 | 文档依据与限制 |
|---|---|---|
| AI compat 退出 | shim 与产品消费路径仍存在 | README 明确未来移除；没有具体版本承诺。[README](../../../.repo/pi-1.0.2/packages/ai/README.md)、[compat 注释](../../../.repo/pi-1.0.2/packages/ai/src/compat.ts#L1) |
| Chord symmetric RpcPeer | 没有独立通用 peer 实现；服务 transport 可替换 | 明确 planned，但不再是当前服务可用性的前置条件。[README](../../../.repo/pi-1.0.2/packages/chord/README.md#L47) |
| Chord structural generation replacement | 当前只支持 shape-preserving reload | 候选图验证、添加/移除 facet 与保留外部 facade 的完整切换仍在计划层。[PLANNING](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L229) |
| Chord 通用 peer metadata、协商及部分背压决策 | 尚无完整对应 generic peer 表面 | 随未实现 peer 留下的设计问题；不能把宿主 transport 已做的限制算成通用层交付。[待决项](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L830) |
| TUI transcript virtualization | 当前完整内容测量与屏幕裁剪 | 文档明确 future use、首轮非目标。[tui-plan](../../../.repo/pi-1.0.2/tui-plan.md#L92) |
| TUI 独立 overlay layout、多窗格键盘 active region | overlay 行合成和 primary scroll 已有，完整新策略未有 | 属后续布局与交互方向，不等于 overlay 或多 ScrollView 不可用。[键盘方向](../../../.repo/pi-1.0.2/tui-plan.md#L548)、[overlay 方向](../../../.repo/pi-1.0.2/tui-plan.md#L631) |
| TUI unread-line indicator | 有 jump-to-end indicator，未见独立未读行计数契约 | 相邻能力存在，不能直接把两者视为同一承诺。[规划](../../../.repo/pi-1.0.2/tui-plan.md#L98)、[已实现 indicator](../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md) |
| durable image read | 当前工具明确不支持图片 | README 使用 not supported yet，但未承诺时间。[README](../../../.repo/pi-1.0.2/packages/durable/README.md) |
| durable 更多 coding tools | 内置组目前只含 read/write/edit/bash | handoff 明确 grep/find/ls 未移植；不代表默认产品也缺这三个工具。[handoff](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L825) |
| durable 缓存友好的摘要请求 | 当前 compaction 已有，精确重发上一请求的替代方案不在 v1 | 明确延期候选，不等于 compaction 尚未实现。[handoff](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L729) |
| coding-agent 服务的身份与授权 | 已有共享 Session 目录和管理操作，尚无 README 描述的身份投影、workspace/plugin 授权 | 明确以 authenticated identity 为前提；现有 token/scope 不等价于该策略。[services README](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/services/README.md#L16) |
| coding-agent 跨 worker reload、Provider/auth composition 插件化 | 单 worker reload 和 Models facet 已有，跨 worker 协调及完整组成权下移未完成 | 不应因已有 FacetHost 就把这些延伸一并标为完成。[services README](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/services/README.md#L19) |
| coding-agent 服务端 tree navigation、子 conversation keyed services、历史分页 | durable 底层机制已有；当前服务主要固定在 root conversation | 原“等待移到 durable”的迁移前提已失效，这些服务层差距仍在；显式 resume 则已被启动时自动恢复替代。[TODO](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/services/README.md#L40)、[分项核查](packages/coding-agent.md) |
| coding-agent hard request limits | `maxRequestBytes` 等 metadata 可配置，但文档明确当前不重写或拒绝既有历史 | 属明确未执行的策略；不能由字段存在推出 enforcement，也没有确定实施日期。[models.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/models.md) |
| delta 的下一次性能／正确性研究 | 没有选定下一代 graph API | findings 提供证据要求和被否决方案，属于研究方向。[findings](../../../.repo/pi-1.0.2/packages/durable/docs/chord-delta-findings.md#L347) |

明确非目标另行保留：Chord 的 CRDT/离线多写者/持久历史，durable 的任意 JS 栈恢复与进程内强停，telemetry 的内置 exporter，codemode 的 Node 全局环境，TUI 的完整 CSS 布局，都不应由本报告擅自转为后续开发计划。它们解释包的边界，有时比未来功能更能说明整体架构。

## 6 文档所揭示的整体演进

跨包材料共同说明三个变化，而不只是增加了若干功能：

1. **Agent 与 durable 的职责分离已发生。** Agent 历史文档里的 Harness 内容不能继续套用到当前小型循环库；durable 已独立形成提交、任务和恢复体系，但默认 coding-agent 仍不是全面切换到这套实验运行时。
2. **Chord 已先交付可替换传输之上的服务机制。** 尚未交付的通用 peer 与 structural reload 不妨碍现有服务使用；架构图应画出已发生的依赖，另标规划，而不是把计划层次当成现行必经链。
3. **TUI 在保留组件模型的同时增加了受约束渲染。** 设计稿保留了“为什么主屏与全屏分开、为什么状态与几何寿命不同、为什么退出要重渲染全文”的理由；已完成的计划仍是理解代码的重要材料。

这些结论分别由 [Agent](packages/agent.md)、[durable](packages/durable.md)、[Chord](packages/chord.md)、[TUI](packages/tui.md) 和 [coding-agent](packages/coding-agent.md) 的规划对照章节展开。仅从目录大小或依赖图，难以识别这几轮替换的意图。

## 7 阅读顺序与本轮边界

先从当前 README 和公共导出确认可用表面，再读设计规范的 invariants、所有权和非目标；随后用较新的 handoff 阶段和 CHANGELOG 确认替代关系，最后回看旧方案与实验记录。直接按长 handoff 从第一阶段开始照做，会误把已被删除的设计重新当成现行目标。

本轮确认的是**文档相对于本地 1.0.2 实现的时效**，并没有确认维护者今天还承诺实现全部剩余项。根 README 与 CONTRIBUTING 指向外部 RFC，且后者说明部分 RFC 不公开；这些外部材料未被读取，不能宣称全部长期规划已穷尽。[规划入口](../../../.repo/pi-1.0.2/README.md)、[RFC 可见性说明](../../../.repo/pi-1.0.2/CONTRIBUTING.md#L99)。

源码、源码清单和原始设计文件均不因本轮分析而修改。报告校验只检查文档结构、引用、覆盖和快照一致性；项目测试、性能目标、物理掉电恢复、真实终端和远端 provider 行为仍不属于本次执行证据。测试影响为 `none`。
