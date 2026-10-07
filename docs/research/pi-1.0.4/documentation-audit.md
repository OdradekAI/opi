# Pi 1.0.4 设计文档与规划时效核查

> 核查日期：2026-10-07。对象：本地 `.repo/pi-1.0.4` 固定快照。
> 本文补充 [整体报告](README.md) 与 14 份包报告，只分析 Pi 自身。文档中的未来意图以本快照为准，不代替上游后续版本或外部 RFC 的核查。

## 1 本版的复核目标

本次以1.0.2报告为基线更新到1.0.4：当前文档是否变化与其主张是否仍有效分开判断。保留设计动机、已兑现计划、未完成方向、替代决策、非目标和历史实验，不用单一“过时”标签丢弃整篇材料。

文档有 **4份新增、18份修改、1份删除、106份字节一致**。4份新文档全部来自env；删除项是根`tui-plan.md`。未改文档中的Azure介绍、telemetry跨包示例及实验服务TODO依然需要复核。旧源码缺失时继承的是有身份绑定的审读记录，不是推造原文已重新读取。

## 2 文档范围与证据方法

当前本地文档 **128份、33,270行**：packages内 **115份、32,276行**，根及隐藏目录 **13份、994行**。后缀为.md/.mdx/.rst/.txt，排除.git/node_modules/dist；完整逐项口径见 [文档索引](documentation-index.md) 与 [机器审读记录](documentation-reading.json)。本次新增记录绑定当前source tree及每份文件SHA-256；继承记录同时绑定 [1.0.2证据](evidence/pi-1.0.2/documentation-reading.json)，必要时继续指向1.0.0原记录。

当前五份核心长设计材料合计 **7,213行**；env新增协议/语义179行另列。tui-plan曾有1,001行，现在只保全旧报告中的历史分析，不混入当前文档计数。

| 材料 | 当前行数 | 复核价值与范围 |
|---|---:|---|
| [Chord PLANNING](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md) | 859 | 同内容继承全文基础；重查WP0–WP8、peer/structural reload与当前代码边界 |
| [durable spec](../../../.repo/pi-1.0.4/packages/durable/docs/spec.md) | 4,692 | 新环境/进度/输出契约，连同持久化invariants及保留的旧状态句分别判断 |
| [durable handoff](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md) | 917 | 阶段1–23及后续替代决策；阶段编号不是包版本 |
| [durable Chord usage](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md) | 387 | 持久文档与服务观察面的接线及寿命 |
| [delta findings](../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md) | 358 | 被否决实验与方法要求；旧性能数据不是当前实测 |
| [env protocol](../../../.repo/pi-1.0.4/packages/env/docs/protocol.md) | 114 | framed stdio、取消、队列、reader/watch/exec与断线 |
| [env semantics](../../../.repo/pi-1.0.4/packages/env/docs/semantics.md) | 65 | 与NodeExecutionEnv结果相符的目标及平台、错误、文件竞争细节 |

变化的使用指南重新阅读；长CHANGELOG重点读取新增版本和相关历史，其他材料按实际范围继承或重读。新索引不会把分类筛查、重点阅读、全文阅读和逐项实现审计合并。外部RFC和在线最新文档不在本次本地更新范围，源码里的命令与示例未被执行。

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

## 3 14 个包的文档核查结果

| 包 | 包内文档数 | 设计／规划覆盖与主要结论 | 详细报告 |
|---|---:|---|---|
| ai | 2 | Azure provider改名/双API、OAuth refresh及retry；README的Responses-only局部陈旧，compat退出仍是无日期方向 | [ai](packages/ai.md) |
| agent | 2 | 循环实现不变；历史Harness/schema移除仍成立，旧消息filter示例仍需按system消息现状限定 | [agent](packages/agent.md) |
| chord | 3 | WP0–WP8矩阵保留；服务/delta/facet已实现，generic peer与完整structural reload未兑现；env使用Context不填补这些方向 | [chord](packages/chord.md) |
| client | 2 | 使用契约不变，无独立路线图；重连不自动完成业务恢复 | [client](packages/client.md) |
| codemode | 2 | 输出上限继续有效，新增冻结/消息收束；host能力仍由调用方授予，无独立路线图 | [codemode](packages/codemode.md) |
| coding-agent | 79 | tool patterns、MCP关闭、guidelines、图片与按键更新；experimental TODO和SDK快速参考分项核查 | [coding-agent](packages/coding-agent.md) |
| durable | 6 | 新env/scan/watch/argv/progress/output已实现；spec旧未实现状态、read竞争描述和工具API示例有局部滞后；历史阶段不可顺序照抄 | [durable](packages/durable.md) |
| env | 4 | 四文档全文阅读；协议、语义及SSH部署已有实现，队列/输出单帧保证需收窄，平台承诺与实际CI矩阵分开 | [env](packages/env.md) |
| evals | 1 | 成对实验协议及限制仍有效，无独立路线图；未因依赖升级重新获得评测数据 | [evals](packages/evals.md) |
| mcp | 3 | native/web DCR已实现；旧issuer示例不完整、README能力列表漏resources的结论仍需保留 | [mcp](packages/mcp.md) |
| protocol | 2 | Session wire契约不变，无独立路线图；env协议是另一边界 | [protocol](packages/protocol.md) |
| server | 2 | router/handle/listener职责不变；身份与策略由宿主提供 | [server](packages/server.md) |
| telemetry | 2 | 当前契约不变，Agent导出示例仍陈旧；exporter是adapter职责 | [telemetry](packages/telemetry.md) |
| tui | 5 | Home/End与Ctrl+Home/End重新分工；根计划删除，历史布局动机保全而当前未来承诺未确认 | [tui](packages/tui.md) |

包内115份资料均有索引记录；TUI历史计划不计入当前5份包文档。没有独立路线图不表示没有设计材料，也不授权分析者补写作者未表达的计划。

## 4 最重要的时效修正

### 4.1 durable 的“尚未实现”不是当前事实

spec §2.2 仍写 Pico5 尚未实现，同时 handoff 顶部已经称阶段 1–23 完成，包的 1.0.0 CHANGELOG 与 Harness 源码也已存在。应判定**这一状态句陈旧，核心设计并未失效**。handoff 里“pico3 remains”的说明则被 Agent 1.0.0 的明确移除记录替代。[spec 状态句](../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[handoff 状态](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L10)、[Harness](../../../.repo/pi-1.0.4/packages/durable/src/harness/harness.ts)、[移除记录](../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md)。

还必须按后续修订解释前面的阶段：第 19 阶段以 structured concurrency 替换 `after/pi.post-tools`；第 21 阶段以 named extensions、`pi.agent` 和 live settings 替换旧 registry/config 接口。当前代码采用后者，因此旧阶段仍解释历史问题，但不能作为当前 API 实施清单。[阶段 19](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L606)、[阶段 21](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L825)、[当前 Agent 接口](../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts#L42)。

handoff 第 22 阶段的 bug 列表也不能直接当作现存问题：now/report 的调用期检查、migration-only publication 和 open 失败清理都有后续实现与针对性测试。逐条闭合证据已放在 durable 子报告中。

### 4.2 TUI计划删除后，历史方向与当前事实分开

本地1.0.4已无`tui-plan.md`。旧报告记录的scrollbar、可见区域选择、virtualization、overlay布局、active region和unread indicator仍有历史解释价值，但只能经 [原摘要与旧报告摘录](evidence/pi-1.0.2/README.md#tui-plan) 追溯；本次无法读取已删除原文，也不能仅据删除认定维护者撤回或继续承诺。

当前ScrollView、layout和renderer仍支持滚动条与跨屏选择，内容测量仍处理完整行；屏幕裁剪不等于virtualization，overlay仍按普通render(width)合成。这些是当前实现判断，不依赖历史计划仍有效。[ScrollView](../../../.repo/pi-1.0.4/packages/tui/src/components/scroll-view.ts)、[layout](../../../.repo/pi-1.0.4/packages/tui/src/layout.ts#L136)、[overlay](../../../.repo/pi-1.0.4/packages/tui/src/tui.ts#L1339)。

旧报告指出的contain余量路由静态现象仍存在：routeWheel在contain处break，但随后可把非零余量交给未访问的primary。应称“与历史计划记录的contain意图有差距”，不能把丢失的计划原文当成本版现行规范。[routeWheel](../../../.repo/pi-1.0.4/packages/tui/src/tui-alt-screen.ts#L986)、[TUI细查](packages/tui.md)。

1.0.3明确把Home/End留给编辑器行首尾，transcript首尾改Ctrl+Home/End；当前keybindings、测试和产品文档相符。与该变更无关的旧布局方案不能因新CHANGELOG标题而一并标为实现。[keybindings](../../../.repo/pi-1.0.4/packages/tui/src/keybindings.ts#L98)。

### 4.3 Chord 的未完成项有明确范围

当前 README 仍把 symmetric RPC peer 写成 remote service boundary 的一种可选计划实现。现有 services 通过 `RemoteServiceTransport` 工作，不能因 Pi client/server 有请求关联和取消，就认定 Chord WP2 已经实现。[README](../../../.repo/pi-1.0.4/packages/chord/README.md#L47)、[WP2](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md#L649)、[公开导出](../../../.repo/pi-1.0.4/packages/chord/src/index.ts#L1)。

现有 reload 要求相同 facet ID 和依赖/服务形状；规划中的新增、删除及重建服务图仍不是公开支持的 structural replacement。该缺口不同于“不能热重载任何代码”。同样，旧 `PluginLoader` 草案已经演变为 `FacetLoader`，文件拆分不同不应误判成能力缺失。[结构替换计划](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md#L229)、[实际 reload](../../../.repo/pi-1.0.4/packages/chord/src/facets/host.ts#L423)、[当前类型](../../../.repo/pi-1.0.4/packages/chord/src/types.ts#L303)。

### 4.4 文档局部矛盾不只出现在规划稿

telemetry 的 Agent schema 导入示例与已经删除的导出冲突；Agent README 低层 filter 示例没有保留现在已存在的 system 消息；AI README 对跨 provider thinking/partial 的概括落后于实际 transform 条件。这些材料可以解释演进，但不能直接复制为当前调用代码。[telemetry 旧导出](../../../.repo/pi-1.0.4/packages/telemetry/README.md#L365)、[Agent 删除记录](../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md)、[Agent 旧 filter](../../../.repo/pi-1.0.4/packages/agent/README.md#L541)、[AI transform](../../../.repo/pi-1.0.4/packages/ai/src/api/transform-messages.ts#L64)。

durable README 将“100 ms 节流”解释为“崩溃最多丢失该窗口”也过强：generation 受在途提交和存储速度影响，tool output 还按输出量限制写入频率。这是文档保证范围的问题，不能提升为固定的恢复点上限。[README](../../../.repo/pi-1.0.4/packages/durable/README.md)、[generation](../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)、[output](../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts#L261)。

### 4.5 历史实验保留方法，不保留为现版数字

`chord-delta-findings.md` 记录了被移除的 graph tracker 实验以及当时 path tracker 的 clone/cache 方案。当前 Chord 已是 immutable revision、revocable draft、prepare/adopt 的模型；trusted root、placement copy 与 alias-free tree 契约也改变了问题边界。旧 heap/耗时数字不能直接当作现版性能，旧 alias 问题不能脱离当前输入契约判断。[历史实验](../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L3)、[当前 ownership](../../../.repo/pi-1.0.4/packages/chord/src/delta/README.md#L96)、[当前生命周期](../../../.repo/pi-1.0.4/packages/chord/src/delta/README.md#L124)。

该文档仍应保留在分析中，因为它说明为什么一次更复杂的方案被拒绝，以及下一轮实验必须测量初始化、读取放大、编辑、序列化、回收和 identity，而不仅是最后输出是否相等。[后续研究的证据要求](../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L347)。

### 4.6 1.0.2已实现的方向在本版仍需限定

**durable：持久provider身份与摘要缓存是两件事。** `pi.provider` 保存独立于 conversation ID 的持久 provider session UUID，fork 使用 initial、checkpoint 保留、旧会话在请求前补建。generation/compaction 都接入，但 compaction 的 `cacheRetention: "none"` 与 handoff 的缓存友好摘要候选仍然存在；不能把 session affinity 改进等同于该规划已经完成。[新文档实现](../../../.repo/pi-1.0.4/packages/durable/src/harness/provider.ts#L6)、[1.0.2发行条目](../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md#L55)、[逐项分析](packages/durable.md)。

**MCP：CIMD 已是可选实现，旧 conformance 说明不能代表当前能力。** 产品文档和实现提供 `oauth.clientRegistration: "cimd"`，但测试指南仍写 Pi 不使用 Client ID Metadata Documents。它可以描述旧 baseline 的形成背景，不能继续用作本版不支持 CIMD 的证据。相反，MCP 包 README 中只解构 code 的回调示例仍未转交 issuer；callback 能返回 iss 不会替调用方自动传入 authorize。[新配置文档](../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md)、[旧测试说明](../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-conformance/README.md#L42)、[包示例](../../../.repo/pi-1.0.4/packages/mcp/README.md#L99)、[分析](packages/mcp.md)。

**codemode：README 里的输出上限是现行约束。** 16 Mi 字符与 100,000 项限制已有 prelude 检查和失败测试源码；它们不再是分析者指出的空白，也不等于宿主整体内存有硬上限。[输出约束](../../../.repo/pi-1.0.4/packages/codemode/src/runtime/prelude-source.ts#L40)、[包报告](packages/codemode.md)。

**TUI：格式支持说明缺少 Kitty 转码前提。** README 列出 PNG/JPEG/GIF/WebP，但未说明 Kitty 路径的非 PNG 图片依赖已注册的 `setImageTranscoder`；库独立使用时无 converter 或返回 null 会退回占位显示。产品已接线与库在任意宿主中自动支持四种格式是不同结论。[README](../../../.repo/pi-1.0.4/packages/tui/README.md#L682)、[转换分支](../../../.repo/pi-1.0.4/packages/tui/src/components/image.ts#L101)、[包报告](packages/tui.md)。

**安装文档：普通 npm 与托管安装已分开。** shrinkwrap 被移除，install-lock 仍服务托管安装和 Nix；Nix 的固定 catalog 构建输入与运行时目录更新也不同。README 使用 `v1.0.0` 作为固定 tag 的示例，不应仅凭该示例数字认定整页陈旧。Nix workflow 三个平台实际构建与四个平台声明支持的差别见 [交付分析](version-changes.md)。

### 4.7 1.0.3–1.0.4使不变文档产生的新时效问题

**Azure。** AI README在Azure Provider段仍写只使用Responses，当前azureProvider已有Chat Completions。旧Provider文件删除、API ID保留，意味着“Azure OpenAI Responses”在Provider介绍里可能陈旧，在API列表里仍正确。不能批量改名字；逐项见 [AI报告](packages/ai.md)。

**durable。** spec已经添加ExecutionEnv扫描、watch、argv和progress契约，但read段关于变化就重新读一次的描述落后于1.0.4容忍追加算法；ToolExecutionApi示例未完整呈现skipped/outputWindow。新实现事实、现行接口与历史伪代码需要分开。README的100ms节流也不构成固定故障丢失上界。[spec](../../../.repo/pi-1.0.4/packages/durable/docs/spec.md)、[read](../../../.repo/pi-1.0.4/packages/durable/src/tools/read.ts)、[输出](../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts)、[详细核查](packages/durable.md)。

**env。** protocol说result/error在命令输出前优先，但daemon有意把exec终局结果放bulk队列，保证其先前输出的顺序；“每个命令最多一个未发输出frame”也需按window.rs细化，无skipped时会按stream segment生成多个事件。文档的方向有效，过强的概括不能替代实际调度保证。[protocol](../../../.repo/pi-1.0.4/packages/env/docs/protocol.md#L52)、[respond](../../../.repo/pi-1.0.4/packages/env/daemon/src/main.rs#L347)、[window take](../../../.repo/pi-1.0.4/packages/env/daemon/src/window.rs#L166)。

**平台声明。** env宣称Linux/macOS/Android/Windows双架构，工作流确实构建8个产物，但release运行测试为6个非Android产物；不能称本次或CI已实测所有8种目标。本次更没有执行任何Rust/SSH/平台测试。[env workflow](../../../.repo/pi-1.0.4/.github/workflows/env.yml)、[daemon workflow](../../../.repo/pi-1.0.4/.github/workflows/env-daemons.yml)。

**根开发与本地发布说明。** 未变的根README开发步骤和release skill没有补充新daemon前置；env README明确要求先build:daemon，helper也仅检测存在。普通build和local-release不替代这一步，且test.sh清除PI_ENV_DAEMON覆盖。应判“说明局部不完整，CI已有专门接线”，不判整个发布系统不可用。[根开发步骤](../../../.repo/pi-1.0.4/README.md#L102)、[release说明](../../../.repo/pi-1.0.4/.pi/skills/release.md#L11)、[env开发要求](../../../.repo/pi-1.0.4/packages/env/README.md#L36)、[具体静态链](README.md#16-测试体系及其证据强度)。

## 5 文档明确留下的后续方向

下表汇总当前材料中的方向、未支持项或延期候选；标为历史的TUI条目另有时效限制。它们**不是统一的已批准待办列表，也没有共同的完成日期**。包报告进一步说明具体实现、已有相邻能力和不确定性。

| 方向 | 当前状态 | 文档依据与限制 |
|---|---|---|
| AI compat 退出 | shim 与产品消费路径仍存在 | README 明确未来移除；没有具体版本承诺。[README](../../../.repo/pi-1.0.4/packages/ai/README.md)、[compat 注释](../../../.repo/pi-1.0.4/packages/ai/src/compat.ts#L1) |
| Chord symmetric RpcPeer | 没有独立通用 peer 实现；服务 transport 可替换 | 明确 planned，但不再是当前服务可用性的前置条件。[README](../../../.repo/pi-1.0.4/packages/chord/README.md#L47) |
| Chord structural generation replacement | 当前只支持 shape-preserving reload | 候选图验证、添加/移除 facet 与保留外部 facade 的完整切换仍在计划层。[PLANNING](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md#L229) |
| Chord 通用 peer metadata、协商及部分背压决策 | 尚无完整对应 generic peer 表面 | 随未实现 peer 留下的设计问题；不能把宿主 transport 已做的限制算成通用层交付。[待决项](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md#L830) |
| TUI历史：transcript virtualization | 当前完整内容测量与屏幕裁剪 | 旧报告记录future use、首轮非目标；当前承诺未确认。[tui-plan（历史二手证据）](evidence/pi-1.0.2/README.md#tui-plan) |
| TUI历史：独立 overlay layout、多窗格键盘 active region | overlay 行合成和 primary scroll 已有，完整新策略未有 | 属旧报告记录的后续方向；当前承诺未确认，不等于overlay或多ScrollView不可用。[键盘方向（历史二手证据）](evidence/pi-1.0.2/README.md#tui-plan)、[overlay 方向（历史二手证据）](evidence/pi-1.0.2/README.md#tui-plan) |
| TUI历史：unread-line indicator | 有 jump-to-end indicator，未见独立未读行计数契约 | 相邻能力存在；历史方向与当前承诺不可混同。[规划（历史二手证据）](evidence/pi-1.0.2/README.md#tui-plan)、[已实现 indicator](../../../.repo/pi-1.0.4/packages/tui/CHANGELOG.md) |
| durable image read | 当前durable read仍明确不支持图片；与默认应用的read/codemode分开 | README 使用 not supported yet，但未承诺时间。[README](../../../.repo/pi-1.0.4/packages/durable/README.md) |
| durable 更多 coding tools | 默认createCodingTools仍为read/write/edit/bash；另有PowerShell工厂，不能称包只提供四种工具 | handoff 明确 grep/find/ls 未移植；不代表默认产品也缺这三个工具。[handoff](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L825) |
| durable 缓存友好的摘要请求 | 当前 compaction 已有，精确重发上一请求的替代方案不在 v1 | 明确延期候选，不等于 compaction 尚未实现。[handoff](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L729) |
| coding-agent 服务的身份与授权 | 已有共享 Session 目录和管理操作，尚无 README 描述的身份投影、workspace/plugin 授权 | 明确以 authenticated identity 为前提；现有 token/scope 不等价于该策略。[services README](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L16) |
| coding-agent 跨 worker reload、Provider/auth composition 插件化 | 单 worker reload 和 Models facet 已有，跨 worker 协调及完整组成权下移未完成 | 不应因已有 FacetHost 就把这些延伸一并标为完成。[services README](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L19) |
| coding-agent 服务端 tree navigation、子 conversation keyed services、历史分页 | durable 底层机制已有；当前服务主要固定在 root conversation | 原“等待移到 durable”的迁移前提已失效，这些服务层差距仍在；显式 resume 则已被启动时自动恢复替代。[TODO](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L40)、[分项核查](packages/coding-agent.md) |
| coding-agent hard request limits | `maxRequestBytes` 等 metadata 可配置，但文档明确当前不重写或拒绝既有历史 | 属明确未执行的策略；不能由字段存在推出 enforcement，也没有确定实施日期。[models.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md) |
| delta 的下一次性能／正确性研究 | 没有选定下一代 graph API | findings 提供证据要求和被否决方案，属于研究方向。[findings](../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L347) |

明确非目标另行保留：Chord 的 CRDT/离线多写者/持久历史，durable 的任意 JS 栈恢复与进程内强停，telemetry 的内置 exporter，codemode 的 Node 全局环境，TUI 的完整 CSS 布局，都不应由本报告擅自转为后续开发计划。它们解释包的边界，有时比未来功能更能说明整体架构。

## 6 文档所揭示的整体演进

跨包材料共同说明四类演进，而不只是增加了若干功能：

1. **Agent 与 durable 的职责分离已发生。** Agent 历史文档里的 Harness 内容不能继续套用到当前小型循环库；durable 已独立形成提交、任务和恢复体系，但默认 coding-agent 仍不是全面切换到这套实验运行时。
2. **Chord 已先交付可替换传输之上的服务机制。** 尚未交付的通用 peer 与 structural reload 不妨碍现有服务使用；架构图应画出已发生的依赖，另标规划，而不是把计划层次当成现行必经链。
3. **TUI在保留组件模型的同时增加了受约束渲染。** 旧报告对已删除设计稿的记录保留了“为什么主屏与全屏分开、为什么状态与几何寿命不同、为什么退出要重渲染全文”的理由；已完成的计划仍是理解代码的重要材料。

4. **执行环境独立扩展到远端。** durable先定义可一致性验证的I/O与进程契约，env提供TS/Rust实现；它扩大可装配边界，还没有把默认产品主链改为远端。新增契约的破坏性、部署前提和平台差异同时成为分析对象。

这些结论分别由 [Agent](packages/agent.md)、[durable](packages/durable.md)、[Chord](packages/chord.md)、[TUI](packages/tui.md) 和 [coding-agent](packages/coding-agent.md) 的规划对照章节展开。仅从目录大小或依赖图，难以识别这几轮替换的意图。

## 7 阅读顺序与本轮边界

先从当前 README 和公共导出确认可用表面，再读设计规范的 invariants、所有权和非目标；随后用较新的 handoff 阶段和 CHANGELOG 确认替代关系，最后回看旧方案与实验记录。直接按长 handoff 从第一阶段开始照做，会误把已被删除的设计重新当成现行目标。

本轮确认的是**文档相对于本地 1.0.4 实现的时效**，并没有确认维护者今天还承诺实现全部剩余项。根 README 与 CONTRIBUTING 指向外部 RFC，且后者说明部分 RFC 不公开；这些外部材料未被读取，不能宣称全部长期规划已穷尽。[规划入口](../../../.repo/pi-1.0.4/README.md)、[RFC 可见性说明](../../../.repo/pi-1.0.4/CONTRIBUTING.md#L99)。

源码、源码清单和原始设计文件均不因本轮分析而修改。报告校验只检查文档结构、引用、覆盖和快照一致性；项目测试、性能目标、物理掉电恢复、真实终端和远端 provider 行为仍不属于本次执行证据。测试影响为 `none`。
