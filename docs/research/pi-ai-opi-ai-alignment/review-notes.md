# 独立证据评审记录

> 非规范性研究评审，2026-10-03。范围是审计、离线探针和比较证据；**这不是候选 spec 的最终独立评审**。首轮进行时 Q1/Q2 尚未确认；后续决定以 [decisions.md](decisions.md) 为准，不能据本文标为 ready-for-agent 或实施准入。
>
> 本文为新的中文研究材料，没有既有英文 counterpart。首轮评审仅编辑本文件；第二轮另经主 Agent 授权修正本评审者原著的 design-evolution.md。没有修改源码或测试，没有重跑探针。
>
> 后续关闭状态：Q1–Q21 及修订 Q17 的目标选择均已确认，R-001–005 已关闭。下文首轮/第二轮的“待决”仅保留历史时点；最新关闭依据见本文末尾及 [spec 证据评审](spec-evidence-review.md)，不是新的开放问题。

## 范围与方法

全文阅读 [design-audit.md](design-audit.md)、[offline-probes.md](offline-probes.md)、[offline-probe-result.json](offline-probe-result.json)、[opi-audit-notes.md](opi-audit-notes.md)。重点复核 A-02、A-03、A-04、O-002、O-007。通过源码检查反证，不按文件/测试存在推断完成：ProviderCollection 的准备与 attempt wrapper、Registry overlay、ApiMappedProvider、Chat/Anthropic/Gemini/Vertex/Bedrock producer、CancelAwareReceiverStream、产品 factory/harness 的非启动 route 与 Bedrock auth 装配、现行规范 §8.1/§8.2。

执行的独立检查仅为只读检索、文档 Rust 代码块与临时源码的文本比较及 SHA-256 核算。下文的运行结果仍来自主研究保留的 JSON；独立评审没有再次运行它。测试影响：**none**。

## 发现与修订复核

R-001、R-002 均已修复，2026-10-03 复核通过。以下保留首轮问题及当时建议作为修订依据；修订后结论见各项状态段，不能把首轮证据限制套到已经更换的探针上。复核者未重跑探针。

### R-001 [P2] Gemini 探针没有证明“drop 后 500 ms 连接未关闭”

**状态：已修复。** 修订版 `drop-barrier-and-classified-socket-outcome` 在客户端执行 `drop(stream)` 后才通过 std mpsc 发出信号；服务器 `recv_timeout()` 成功后开始计时与 socket read。代码分别识别 EOF、reset/aborted、timeout、unexpected data，未知错误 panic；因此不再把关闭与超时混为 false。保存结果的 compile/run 均 exit 0，`gemini_idle_after_drop=timeout; observation_ms=501`。这支持本次本地场景中 drop 后约 501 ms 观察窗未检测到关闭，不证明永久泄漏或所有 adapter 的清理时限。原问题在 offline-probes.md 中已明确保留为被替代版本的限制。

**受审主张：** A-03 的“消费首事件并 drop 流；500 ms 内连接未关闭”，以及 offline-probes 结尾的“连接未关闭证明此场景下没有及时 drop cleanup”。

**证据问题：** 保存的 Rust 探针在服务器发出首帧后立即执行 `socket.set_read_timeout(500ms)` 和 `matches!(socket.read(...), Ok(0))`。客户端此时还要获得首个事件并执行 drop。探针没有服务器可见的 drop 屏障，因此计时窗口并非从 drop 开始。它还把 `ConnectionReset`、`ConnectionAborted`、timeout、其他 I/O error、读取到字节全部折叠为 false。连接被 abort/reset 也是关闭的可能表现；结果 false 不能区分它们。

**已检查反证：** [gemini.rs](../../../crates/opi-ai/src/gemini.rs) 的 producer 确实直接 await `stream_http()`，读取 select 只有 request cancel 与下一 chunk；[stream.rs](../../../crates/opi-ai/src/stream.rs) 的 CancelAwareReceiverStream drop 不取消 request token。Vertex/Bedrock 同型 producer 也没有整个 HTTP future 的 receiver-close select。相反 Chat/Anthropic 有外层 `tx.closed()`。所以 **A-03 的静态生命周期缺口仍有直接证据**；本问题不否定该缺口，只否定该探针已提供严格连接清理时序证明。

**建议：** 现在把运行观察降级为“本次服务器观察未得到优雅 EOF（Ok(0)）；由于错误分类和计时不同步，无法仅凭此结果判定连接是否在 drop 后关闭”。A-03 的确切机制结论注明静态确认，Gemini 执行结果注明局限；不要写“已验证 drop 失败”。如后续需要行为验收，在 drop 之后通过独立同步通道启动观察，记录具体 read 结果/错误类型，并优先使用 producer 完成或 request future 被丢弃的机械信号。500 ms 不成为目标性能要求。

**置信度：高。** 依据是保存的完整探针代码，不依赖操作系统对连接重置的某个特定行为。

### R-002 [P3] probe_source_sha256 没有说明换行归一化

**状态：已修复。** 结果 JSON 已分为 `source_lf_sha256` 与 `compiled_source_bytes_sha256`，文档解释二者口径。复核者只读重算 Markdown fenced source（含末尾 LF）与编译命令指向的当前源文件原始字节，两者均为 `3084365913ea575eb8ebecb15c4f4ad09e4bc067563f114be15eeac48da474ae`，与记录一致；未重新编译或执行。

**受审主张：** offline-probes 把 `e9547b486dea8480ea648b7cdfd44d44dc95afbb563f6f1912accf68239021dc` 称为“源代码 SHA-256”，结果 JSON 使用 `probe_source_sha256`，同时提供编译时临时路径。

**独立核算：** Markdown Rust code block 加一个结尾 LF 的 SHA-256 正是该值；当前仍存在的编译路径 `probe.rs` 含 102 个 CRLF，其原始字节 SHA-256 为 `6ef2a941ec12cba1a4c685c50099b7df651fcbbd2b8956684ee9c8f5c02724fd`。将 CRLF 归一化为 LF 后得到已记录值；逐行文本比较无差异。

**影响与反证：** 没有发现不同探针逻辑或结果篡改；差异完全由换行解释。但按编译命令对源文件做原始字节核验会失败，影响研究复现。

**建议：** 明确该字段为“LF 归一化、含末尾换行的源码摘要”，或另记原始文件摘要与归一化算法。无需重跑探针，也无需修改运行时代码。

**置信度：高。** 本次独立执行了只读哈希核算。

## 首轮核验通过、应保留的证据限定

本节保存首轮时点的判断；其中未决目标和未实测状态不能覆盖后续人类决定及新增证据。当前 O-002 已另有 [纯 mock 路由探针](route-probe.md) 的反例，仍未运行完整产品切换；当前目标状态见 decisions.md。

| 审计项 | 独立判断 | 核查依据与反证 | 保留的限制 |
|---|---|---|---|
| A-02 UTF-8 chunk 边界损坏 | 同意其正确性结论 | Chat 在每个 byte chunk 上 `from_utf8_lossy` 后再累积 SSE；保存运行结果出现三替换字符。Anthropic 先缓冲字节是直接反证，审计已区分。 | 当前探针仅重现 Chat 的 text payload；tool arguments、签名、所有其他 adapter 的具体损坏是同型实现风险，未运行遍历。建议中的严格 malformed/UTF-8 失败策略仍是待确认目标，不是当前契约。 |
| A-03 取消/背压机制不对称 | 同意源码结论；修订探针支持本次 Gemini drop 观察 | Chat/Anthropic 的 send await 不 select cancel，外层只观察 receiver close；Gemini/Vertex/Bedrock 使用 send_or_cancel 和优先取消 receiver，却缺整个 HTTP future 的 receiver close。修订探针有 drop 屏障和关闭/超时分类，观察为 timeout、501 ms。 | 不宣称永久泄漏；目前探针未证明所有 producer、每个 wait point 或精确取消优先级。target 的 terminal/cancel 竞争规则仍需决策。 |
| A-04 terminal 只释放租约，wrapper 未封闭 | 同意观察和处置候选 | AttemptStream 每次 poll inner；terminal/Err/None 只清 active，未清 inner 或标记 fused。Mock 探针从 Done 后读到追加 Start；drain 对空 EOF 显式报错。 | 不把“wrapper 未守卫”写成默认产品已消费垃圾；Agent/drain 在 terminal break 是重要反证。现行公开 EventStream alias 没有完整合法序列契约，统一守卫是拟议的契约强化，尚非已获批准需求。 |
| O-002 effective metadata 与 dispatch 自有目录分离 | 同意静态数据流结论与高置信度；未实测须保留 | Registry override 保留并优先 resolve；register_route 不清 model override；prepare_call 从有效 ModelInfo 得 wire/preflight，但仅传 Request/auth。ApiMappedProvider 再读自身 models。产品只 materialize active route，跨 provider switch 没有补做。 | 有效情景需同时具备非启动可 dispatch route、extension override 和后续选择。内置 refresh 没生产触发，因此动态 refresh 部分不能叫默认产品故障。 |
| O-007 AWS local discovery/process 的 placement 张力 | 同意发起 Placement Review；不同意在未决前写成无条件迁移要求 | credentials.rs 确有 profile I/O、PowerShell/sh process 与固定优先级；product BedrockAuthResolver 使用它。§8.1 product neutrality/minimal authority 与 §8.2 ADR 条款是直接依据。 | helper 由调用方显式输入后触发，并非导入即执行；已有超时/输出上限/取消清理，SigV4 本身仍属 wire 实现。需讨论的是 discovery/process owner，而不是删除 Bedrock 或撤掉已有 workflow。 |

O-002 进一步核验了不利于发现成立的条件：registry-only listing 是合法需求；active route 已 materialize；只改 max output 的 override 可经 Request 传播。因此保留这三项反证很重要，不能将该发现扩大为“所有 model override 均失效”或“所有三层设计应合并”。

O-007 的规范引用准确，但“把 discovery 移到 product”不是唯一由事实强制推出的选项；现行规范下可能需要保留能力并改变归属，或人类明确修改 admission 解释。最终 spec 需记录所选方案及行为保留/退出条件，不能先把候选 ADR 当已确认。

## 交付前的适用范围

上述 R-001/R-002 是对研究材料证据精度的发现，已通过探针与记录修订解决，没有运行时实施。评审未发现足以否定 A-02、A-04、O-002 的源码反证；它们依然需要范围确认和对应验收契约。本文也不证明候选 spec 的所有 MUST 有 owner/trigger/failure/verification，更未检查中英文 spec 对齐，因为这些文件尚未形成。

待决策完成并生成候选 spec 后，应另做独立评审：检查未确认选择是否被写为要求、各旧路径的替代/迁移/退出证明、R-001 所涉生命周期验收能否机械判定，以及所有目标要求与证据/决策的追踪关系。本轮不能代替该步骤。

## 第二轮：主题覆盖、设计分类与历史证据

本轮只读复核 evidence-matrix.md、pi-contracts-notes.md、design-evolution.md、README.md、decisions.md、operations-design.md，并追踪 retry 源码和测试。研究者曾编写 design-evolution.md，因此对该文件的检查是**作者证据复核**，不声称它是独立作者审查；对其他作者的比较材料是独立复核。候选 spec 在本轮开始时尚未形成，本轮不替代其最终独立评审。

### 15 主题覆盖判断

每项都有可导航的当前行为与限制，不能将“主题已覆盖”解释为供应商穷尽或运行验收通过。本轮新增的实质遗漏集中在 M-06；其余缺口大多已明确声明。

| 用户主题 | 当前索引 | 判断与必须保留的界限 |
|---|---|---|
| 1 身份、provider/API/wire、路由 | M-01，P01/02，O-002 | 有双方机制、消费者及 route mock 反例；后者不是 live wire 或完整 CLI 证明。 |
| 2 公共入口、全局与所有权 | M-02，P01/02/07，EVO-002/003/010 | 已纠正 compat 导入等于默认 dispatch 的误读；轻量入口测试未运行。 |
| 3 transcript/system/tools/replay | M-03，P04/05/19，EVO-007/008 | 有实现与历史，但旧候选推荐须与人类后续决定区分，见 R-003。 |
| 4 流序列、终止与 EOF | M-04，P03/06，A-02/04 | 已区分 wrapper 不变量和 adapter 行为，不要求复制 pi 弱 EOF。 |
| 5 取消、超时、背压与资源 | M-05，P03/07/09/10，A-03 | 有静态与 Gemini 有界观察；不推断所有 adapter 或永久泄漏。 |
| 6 call/attempt/retry/fallback | M-06，P08/13 | 原文漏 assistant-call 与产品回合重试；需按 R-004 修正。 |
| 7 API key/OAuth/存储/刷新 | M-07，P09，O-006/007/012 | 库/产品、显式输入/ambient 与生命周期已分；未执行凭据链。 |
| 8 目录/缓存/刷新/过期结果 | M-08，P10，EVO-005/006 | Rust 独占刷新与 TS generation 不机械对齐；RFC 状态精度按 R-005 修正。 |
| 9 tool schema/partial/strict | M-09，P11/16，O-004/008 | 区分展示解析、执行校验和 wire strict；不声称全 JSON Schema conformance。 |
| 10 thinking/signature/effort | M-10，P12，EVO-012 | 基线和 tag 后修复分开；未将 serde roundtrip 当 live replay。 |
| 11 usage/cost/cache/未知 | M-11，P12，O-011 | 明确 pi 注释与实现矛盾、opi 缺失语义；未核账单。 |
| 12 图像/classifier/deferred | M-12，P14–16，EVO-009 | 操作、输入/结果图像、deferred response/tool loading 分开；不能把真实 codemode 操作与虚拟路由实验混同。 |
| 13 HTTP/SSE/WS/proxy | M-13，P07/13 | Codex 的局部恢复不泛化为统一 fallback；未测吞吐/首 token。 |
| 14 日志/脱敏/telemetry | M-14，P17，EVO-011 | observer/helper、安全边界及产品 analytics 分开；未全面审计 exporter。 |
| 15 测试/公开一致性/产品接入 | M-15，P18，O/A 笔记 | 静态断言、70 项实际测试、隔离 probe、未执行 pi 测试的口径明确。 |

### 官方来源、版本与设计类别

- design-evolution.md 的 14 个唯一 GitHub commit URL 均指向官方 `earendil-works/pi`，使用完整 40 位 SHA；本轮做了链接结构核算。先前研究已通过官方 API/PR/RFC 读取验证其内容，本轮未重复查询移动中的 main，也没有新宣称“截至此刻最新”。
- EVO-001 的来源证明严格限制为 378 个 ai 包 tracked blob；固定 tag、main SHA、查询日期、ahead/behind、当次 release 状态分开。没有把 ignored catalog、整个 monorepo 或 npm 发布物等同于此次树验证。
- EVO-012 的 tag 后内容明示已合并未发布，RFC Discussion 未当成接受，compat 退出无确定日期，趋势归纳明示研究推断。未发现把未核验 latest 写成确定路线图的主张。
- 稳定核心候选有实例所有权、provider/API 分离、明确 refresh 及顺序语义；这指有实现和真实消费的持续设计意图，不等于 1.x 永久 API 承诺。compat/旧 image 全局是迁移与历史负担；virtual models 是明确实验；beta wire、TypeScript barrel/subpath、具体等待常量是局部机制或语言实现选择。多 operation 已发布且有消费，不能仅因近期新增而称为实验；也不能据 pi 消费者证明 opi 准入。

### R-003 [P2] 访谈前的建议被保留成当前待决状态

**状态：已修复，2026-10-03 复核关闭。** review-notes.md 已注明历史时点，design-evolution.md EVO-009/结尾已指当前决定；P19 的旧 snapshot 推荐、M-03/M-05/M-12 与 operations/placement/transcript 的状态均已更新或明确标为访谈前推理。当前目标以 decisions 为准，详见末尾关闭复核。

**受审主张与反证：** P19 仍推荐近期 snapshot、延后 transcript，M-03/M-05/M-12 仍写对应目标待决，evolution 仍把非 chat 与 transcript 是否纳入作为未决范围；而 decisions.md 已记录人类选择完整 transcript、工具图像发送或拒绝、明确流生命周期和新操作归属。旧方案的技术推理并未因此失真，失真的是它仍被呈现为当前推荐或待决事项。

**最小修补：** 保留备选项和反证，明确标为访谈前候选；当前方向只引用 owning decision，不复制不断变化的问答进度。将“范围已确认”与尚未完成的迁移、投影和验收细则分开。无需重开已回答问题或删除不同方案分析。置信度高。

### R-004 [P2] HTTP/传输恢复被过度概括成所有重试的共同语义

**状态：已修复，2026-10-03 复核关闭。** P08 已补公开 helper 和 compaction／产品消费者，M-06 与 D-09 将 opi 的已交付事件禁自动重试明确为有意更保守差异；AIR-007 保留该 opi 行为，不再声称它与 pi 全链等价。测试仍是静态阅读，没有重跑。

**受审主张：** M-06 将 opi 的“任何已交付事件后禁止自动重试”与 pi 写成“输出后不重播，语义部分对齐”；P08 仅记录 provider HTTP helper 和 Codex WS 恢复。这两层的证据不足以支持对 pi 整体重试语义的判断。

**遗漏的一手证据：** [index.ts:43](../../../.repo/pi-1.0.0/packages/ai/src/index.ts#L43) 公开导出 [retryAssistantCall](../../../.repo/pi-1.0.0/packages/ai/src/utils/retry.ts#L185)；它消费最终 AssistantMessage，根据错误字符串分类、启用策略、预算和取消决定重新调用 produce，不检查此前向消费者交付过哪些事件。[compaction.ts:614–638](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/compaction/compaction.ts#L614) 是真实消费者。另一个独立层是 [AgentSession._handlePostAgentRun](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L1806) 与 [_prepareRetry](../../../.repo/pi-1.0.0/packages/coding-agent/src/core/agent-session.ts#L3713)：retryable assistant error 后保留 raw history、用 context edit 从模型 projection 省略该失败回合，退避后继续 agent。该分支没有 opi 的“已交付任何事件即禁止重试”守卫。

**反证与限制：** Codex 在 parsed 输出后不做 WS→SSE fallback 的原始结论仍成立。不能据回合重试存在推断 tool 副作用重复、所有错误均重试，或所有 partial 内容都实际显示过；本轮只确认各层 contract 不同。产品还先排除 context overflow、处理 abort、记录 retry 事件，不是无界盲重试。

**最小修补：** P08 区分四层：HTTP acquisition、同 call 传输恢复、assistant-call helper、产品回合重试；补真实消费者与 [retry.test.ts](../../../.repo/pi-1.0.0/packages/ai/test/retry.test.ts)、[compaction regression](../../../.repo/pi-1.0.0/packages/coding-agent/test/suite/regressions/6647-compaction-retries-transient-stream-drop.test.ts)、[retry events](../../../.repo/pi-1.0.0/packages/coding-agent/test/suite/agent-session-retry-events.test.ts) 的静态断言来源。M-06 将 opi 的严格可见性边界标为自身有意更保守的选择，而非 pi 全链统一契约。既有测试未运行，本轮不新增运行要求。置信度高。

### R-005 [P2] RFC 部分实施不能直接判为文档部分过时

**状态：已修复（作者证据复核）。** 本轮按主 Agent 授权修改 EVO-005 为 EVO-005a/005b：核心目录目标当前有效，版本协商及 analytics 的具体当前契约地位无法确认；整体实施程度与主张有效性独立表述。

**原问题：** EVO-005 将整个 RFC0039 标为“部分过时”，给出的依据主要是目标部分落地、页面仍显示 Discussion、另一些字段未确认为现有保证。该证据没有具体指出哪个主张已被后续决策替代；未完整实现不能证明文档过时。与之不同，EVO-007 的 DeveloperMessage→SystemMessage 和 EVO-011 的0019→0038有明确替代依据，不应一并降级。

**修补结果与限制：** 未新增“RFC整体已接受/已完成”的结论；保留未知项，不虚构撤回或替代日期。后续若要进一步归类版本协商/analytics，需逐项追踪当前导出、实现与维护者修订。本轮不扩大到整个 analytics 出站审计。置信度高。

本轮实际检查：只读源码/测试/文档检索、14 个官方 commit URL 结构核算；未重新请求最新上游、未运行 runtime probe 或测试。文件检查结果由本轮交付报告和主 Agent 合并后的文档检查记录；测试影响 **none**。

## 关闭复核

2026-10-03 对 [spec-evidence-review.md](spec-evidence-review.md) 的修订复核同时关闭本页 R-003/R-004。P19 保留早期方案事实但明确后续人类决定已覆盖；M-03/M-05/M-12、EVO、operations/placement 头注及 transcript Q15/Q19/Q20 状态均不再要求重新决定已确认方向。Q1–Q21 及修订 Q17 已确认，正式规范修订与实施准入仍未办理，两者不得混同。

本页 R-001/R-002 的探针计时/哈希问题已在前轮解决，R-005 的 RFC 文档有效性问题已按核心目标与未核验细节分开处理，本次无反证要求重开。当前 R-001–005 均已关闭；首轮表格中的未实测与未确认文字是当时记录，后续 route probe 和 decisions 已另有明确来源。

本次只更新评审记录，没有执行运行时测试、重跑探针或审查中文 spec；测试影响 **none**。双语结构与语义检查由主任务另行完成，不能由本页的证据问题关闭推导其已通过。
