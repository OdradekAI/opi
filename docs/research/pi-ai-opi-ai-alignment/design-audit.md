# opi-ai 设计审计与候选处置

> **后续重审：** 用户已将本轮基准明确为“pi 的 Rust 实现”，opi-spec 仅作可能过时的参考。本页源码事实保留；涉及原 Q1–Q21 的目标建议和确认状态须先读 [决策重审](decision-reassessment.md)。旧规范不能继续单独否决 pi 已落地能力。

> 非规范性研究。2026-10-03，基线见 README。建议不是人类确认或实施授权。
> 研究结论必须读其触发条件与反证；不因 pi 不同而判 opi 错误。

本审计分两部分：主 Agent 核验流生命周期与离线探针；[独立调查](opi-audit-notes.md)保存路由、目录、鉴权、能力和公开表面的详细证据。发现 ID `A-*` 为主审计，`O-*` 为独立调查导航。统一比较见 [证据矩阵](evidence-matrix.md)，取舍及确认状态见 [决策](decisions.md)。

| 处置主题 | 处置（确认状态见decisions） | 详细证据 |
|---|---|---|
| Registry/Collection/mapped职责、typed auth/error、usage未知 | 保留 | A-01/05/06，O-001/006/011 |
| effective model双authority、framing、terminal、drop/取消 | 修正；收敛重复选择与生命周期机制 | O-002，A-02/03/04 |
| 不连通Config/Model/Error/ProviderKind壳 | Q6确认删除候选；保留持久化ApiKind | O-003，D-08 |
| opi未接线而pi有真实行为的compat字段 | Q24确认补齐真实行为；等价合并并迁移后才退出重复字段 | O-004，RA-01，RS-01，Q24 |
| AWS profile/process discovery | Q24确认SDK可复用完整链与产品配置责任；撤回产品独占迁移 | O-007，RA-02，RS-02，Q24 |
| 当前request snapshot、signed replay | Q22确认完整动态transcript、每call trusted revision、默认native-or-collapse；保留签名来源约束 | O-010，P19，D-05 |
| 工具结果图片 | Q7确认真实发送或提前拒绝 | O-009，D-06 |
| 动态目录production trigger、WS | Q26确认目标与产品接线；按正确性依赖排序，不新建通用框架 | O-005，RA-06，RE-01/02 |
| README无environment IO表述 | 修正文档范围 | O-012 |
| 显式telemetry缺失、sinks语义混淆与verbose接线声明 | Q37补齐opi-ai context模块；保留diagnostic/events/evidence；不新增自动SDKspans或exporter | A-08、TO-01–21、D-37 |

## A-01：保留显式实例及三种不同职责

当前 ProviderRegistry 管目录分层和身份查找，ProviderCollection 管 route/auth 准备与 logical call，ApiMappedProvider 管同一 provider 的多 wire dispatch。Agent 与 coding harness 均使用这些职责；它们的删除会把复杂度移回调用方。证据：provider_collection.rs:297、416、764，api_mapped.rs，产品 provider_factory 与 Agent prepare_call/start_attempt。

**处置：保留职责，修正职责间一致性。** 不建议为了 pi 的 Models 名称把三者物理合并，也不建议新建第四个同职责 facade。具体有效模型状态裂缝由 O-002 记录。反证：接口数多本身不能证明抽象浅；registry-only 查询与 dispatchable route 明确不同。置信度高。保持现状的代价是当前状态同步责任需要补齐，而不是所有分层都不成立。

## A-02：SSE 分块边界会改变有效 UTF-8 文本

**责任与消费者**：HTTP adapter 将网络字节转为 provider-neutral 内容，默认产品和直接 SDK 均消费。Chat 在 [openai_chat.rs:1219](../../../crates/opi-ai/src/openai_chat.rs#L1219) 对每个网络 chunk 单独 `from_utf8_lossy`；Responses:436、Codex:229、Gemini:771、Vertex:286 有同型路径。Anthropic:1040 起保留字节并按完整 SSE 解析，是已检查的反证。

**成立条件与影响**：一个合法多字节字符跨两个 HTTP chunk，单 chunk 并非完整 UTF-8，却被不可逆替换为 U+FFFD。JSON 仍可能有效，因此表现为正常完成但文字、工具参数或签名输入损坏。没有必要依赖异常供应商行为，分块合法且由传输决定。

**实际验证**：[离线探针](offline-probes.md) 对当前 OpenAI Chat 公共 Provider 接口发送分开的“中”字字节，输出 `���`；这不是仅根据源码猜测。其他同型 adapter 仍是静态证据，不称全部已重现。

**候选处置：修正**。目标为任意合法 HTTP 分块都产生相同文本和事件语义；完整 SSE frame 后才做严格 UTF-8/JSON 解码，不吞掉 malformed frame。可在 crate 内共享字节 framing 实现，但无需新增公开 Transport trait。是否统一所有 parser 由行为一致性与现有 framing 差异决定。

保留现状没有语义理由；实现少量简洁是其唯一便利，不能抵偿静默内容损坏。验收设计：按每个字节位置分块的中文、emoji、tool arguments 与 CRLF fixture，断言 payload 等价、事件顺序与 terminal 不被归一化掩盖；无效 UTF-8 明确失败且不回显原始负载。置信度：Chat 高（运行），其他列出的路径高（静态）、待逐 adapter 回归。

## A-03：取消、背压与丢弃流的保障不对称

**状态归属**：各 adapter 启动异步 HTTP producer，容量 64 的 mpsc 连接消费者；Request.cancel 是显式取消权威。Anthropic:1422–1463、Chat:1513 起有 `tx.closed()` 终止整个 producer，但内部 `tx.send().await` 未同时选择取消；满队列且消费者保持但不再 poll 时，取消可能被阻塞。Anthropic 的普通 ReceiverStream 也不优先丢弃已排队输出。

反向证据：Gemini/Vertex/Bedrock 使用 stream.rs:444 起的 CancelAwareReceiverStream 与 send_or_cancel，能在背压时取消并丢弃旧缓冲；其 producer（gemini.rs:887、vertex.rs:186、bedrock/mod.rs:241）却没有整个 HTTP 工作级别的 receiver-close 观察。接收者 drop 后，若 HTTP 正在等下一个 chunk、请求没有 timeout 且 token 未 cancel，producer 仍可等待。

**实际验证**：Gemini 本机 HTTP 发出首帧后保持空闲，消费首事件并 drop 流；500 ms 内连接未关闭。探针自身 3秒 timeout 限制资源；不声称永久泄漏。Vertex/Bedrock 只做静态同型检查。已有 70 个通过用例不能覆盖该 drop 场景。

**候选处置：修正并收敛内部生命周期机制**。保持有界队列和独立取消；所有发送、连接、读取及 producer 等待都能响应取消或接收端关闭。禁止改成 pi 的无界 FIFO。Drop 应结束该 attempt 的本地工作，不应取消准备好的整个 logical call，仅符合 D-36 准入的可重试 acquisition failure 才能后续 retry；dispatch 后未知效果 drop 封存调度，但不取消 caller token。显式共享 token 取消结束整个 call。

Q9已确认首次terminal与取消的优先级：已交付 terminal 不可撤回，尚未交付时已观察取消优先，缓冲 delta 丢弃。测试需要人工同步的屏障，分别验证队列满、idle socket、未 poll 直接 drop，不能只靠 sleeps 判断清理。

保持两套机制的理由是原有 adapter 演进路径不同；代价是调用方必须了解供应商才能正确释放资源。置信度高（已验证 Gemini 场景和显式分支），尚缺全部 adapter 故障注入；属于正确性和生命周期问题，不是代码风格。

## A-04：prepared attempt 的终态只是释放租约，没有封闭事件流

**责任与证据**：[AttemptStream::poll_next](../../../crates/opi-ai/src/provider_collection.rs#L793) 对 Done、Error、Err、None 清除 active，但下一次 poll 仍读 inner；空流直接返回 None。`drain_to_completion`（:718）另行拒绝无 terminal EOF；Agent loop（agent_loop.rs:1222）另行转 ProviderProtocol 错误。

**运行证据**：MockProvider 经真正的 prepare_call/start_attempt，Done 后仍交付追加 Start；空 prepared stream 返回 None，而 drain 同一类输入得到错误。现有成功消费者在 terminal 处 break，是已核查的重要反证：不能说默认产品已展示了 terminal 后垃圾事件。

**影响条件**：第三方或新 SDK 消费者直接驱动 EventStream，或自定义 Provider 返回异常序列时，必须重复实现 terminal/EOF 守卫；第一个 attempt 已释放 active，但旧 inner 仍保留且可 poll，租约不等于资源已退出。

**候选处置：修正** prepared seam 为唯一 attempt 终态守卫，terminal 一次、terminal 后永久 EOF，清理 inner；异常 EOF 一次 typed StreamError，继而 EOF。原稿保留 Done/Error 与 Rust Err 双轨的建议已由 Q32/D-31 替代：当前公共逻辑调用统一 Result/CallFailure，完整消息/usage/cause 保留；高级 acquisition attempt outcome 与最终 public ChatStream error 仍须分层。默认 Agent/drain 可保留防御性拒绝，不能拥有不同成功判定。

需明确 adapter 仍负责取消 HTTP 工作，fuse wrapper 不能单独证明 producer 清理。无需将事件合法序列自动“修复”为成功。保留现状的理由是默认调用方已有防御；代价是公开接口的契约分散。置信度高（运行）；已由Q1/Q9确认纳入近期候选。

## A-05：保留 Rust 的 frozen logical call、typed errors 与隐私边界

prepare_call 在执行前一次固定 request/route/auth，start_attempt 使用相同材料；at-most-one attempt 是独立状态机价值。Agent 负责 retry policy，首个可见事件后不自动重试（含 Start），auth needed/revoked 与共享取消终止 call；不静默 fallback 到另一个 provider。证据：provider_collection.rs:416–550、858–898，agent_loop.rs:460–606、1092。

**处置：保留冻结调用、typed errors与安全边界，补充分块／终止／模型状态一致性。** 首个事件后不 retry 是当前 opi 事实；RA-07已证明 pi 有可观察的高层失败回合恢复，不能再将该全面限制作为无需讨论的 Rust 差异。低层流不重复输出与高层新恢复尝试分别验收。 凭据刷新在新 logical call 的 prepare 阶段进行；不能将普通网络 retry 变成绕过 credential revocation 的刷新重试。70 个现有测试中 prepared call/credential/fallback/atomic catalog 断言已运行通过。反证：API 集合名称不同不能说明错误；该 seam 的状态机价值不依赖找到第二种 UI。

ProviderErrorSummary 通过私有字段与封闭构造丢弃不可信错误内容；typed error 保留可操作分类。pi 注释中的 redacted diagnostics 与实现不完全一致（见 P 笔记），不能成为削弱此边界的理由。保留安全摘要不等于完全没有诊断：类型、状态码、无秘密 route/provenance 应保留。置信度高。

## A-06：预算、超时与使用量需要区分层级

Request.timeout 是一次 HTTP request 的 deadline，prepare_call 的自定义 AuthResolver 等待只受 cancel；默认 None 不承诺端到端截止。名字或“bounded failure”愿景不能证明整体超时已实现。Agent 当前构造 Request.timeout=None；增加整体 wall-clock 默认预算属于策略决策，不应偷偷塞进中立库。

Usage 已有 reported 区分整条 usage 缺报，reasoning/1h 子集用 Option 区分缺失与0，累计使用u64并饱和投影到u32，值得保留。单个父桶仍不是 Option；不应声称其已精确表达每个缺报字段。calculate_cost 将1h子集按2倍input计费，是当前具体规则，是否所有provider适用需要对应定价证据；不得仅因 pi 有 cost 字段就把未知费用变为0。

候选处置：保留可证明的语义、明确当前限制；任何通用计价/整体deadline重设计先确认真实需求。证据：stream.rs:45–350、provider.rs Request、provider_collection.rs:514、agent_loop.rs:478。测试支持：usage_cost/context_budget 既有断言，尚未在本次运行。置信度高（表示法），中（跨供应商定价完整性）。

## A-07：独立 SDK 默认、管理与 retry/resource owner 的收敛

**职责与消费者。** 当前 prepare_call/start_attempt 有独立租约、冻结 request/auth 价值，AgentLoopConfig.retry 在 agent_loop.rs:461,1092–1123 调度同一 prepared call；产品 harness.rs:1774 注入该配置。Summary 引擎 compaction.rs:186,210,300–327 用同一 RetryConfig 另包完整 run_provider_call；OpiConfig.retry/config.rs:29,774,1135–1142 将次数/旧 capped backoff 物化。不是凭字段存在判能力，已检查实际循环和来源读取；当前 SDK providers 自身并无通用自动 retry loop。

**成立条件与影响。** 在本次已选 SDK 默认 retry/唯一 owner/Q36 目标下，照旧保留 Agent 循环会叠加，保留无限 start_attempt 会绕过同 call 次数；把 Summary 同一旧配置继续解释为高层恢复会混淆 counted attempt/new call 与 delay 含义。这是目标迁移必要修正，不能声称当前未加入 SDK retry 的代码已经发生乘法重试。反证是目前循环各有实际消费者，不能以“没有调用方”删除；冻结鉴权/at-most-one 仍保留。

**处置：修正/合并 owner，明确迁移。** SDK call owner 管 acquisition 次数/分类/退避，advanced 单 attempt 只推进同一状态；Agent 不再另包低层重试。Summary 使用已确认公开高层机制和同目标身份规则；产品 model_call/recovery config 分层，legacy retry 键报显式迁移错误，不把旧 capped delay 偷换成 server-delay admission。纯 helper 若仍有独立真实消费者可保留，不强制删除有价值实现。替代是保留两个次数 owner并协调上限，复杂且不能给外部SDK同一默认；成本是公开配置/事件/summary/diagnostic/schema/help/tests迁移，退出以 V-05/20/33 证明。

**SDK资源与管理补核。** UX-11–15 给出惰性 env 来源、管理返回、Codex SDK Auto、公开 cleanup 的一手依据；已有 Rust 独立实例/owned state 是保留理由。pi 全局同步 cleanup 缺 connecting/临时资源，不能作为正确性 oracle；Rust 实例 owner、first-poll generation、内部单向取消和 await 屏障是已选资源目标的落实，非假想通用 transport framework。具体语义集中在 AIR-034–036/046 和 D-35/36。上述新增结论置信高（静态调用链/版本锁定依赖），未来配置/资源/次数 conformance 未执行；不据此声称当前WS leak或性能提升。

## A-08：telemetry 接入缺口成立，既有三条通路不构成冗余

**当前职责、消费者与影响。** [TO-01–21](telemetry-opi-audit.md)核源码：opi-ai有tracing依赖但无显式parent；少量warn/debug不构成span树。DiagnosticSink服务真实诊断计数，AgentEvents服务产品事件且保留回调失败，EvidenceSink/IdentityAllocator服务证据health/artifact/manifest，RPC trace返回该evidence。外部SDK跨任务需要显式parent时，当前只能自包外层future，不能通过options稳定传给provider。这是接入缺口，不证明业务完全不能被观测。

**反证与保留理由。** 外部caller能自行instrument，pi内置AI也没有自动spans；不能以此指控opi违反pi默认。三条sink已有不同消费者/失败规则，名称相近不是冗余证据。现状零新API成本，可满足外层duration；代价是context无法穿过owned/lazy/adapter路径，verbose“debug tracing”声明未找到初始化对应。没有发现遥测导致实际泄漏或吞业务结果的事故。

**处置与替代。** Q37/D-37补齐opi-ai tracing-backed模块和全部已选operation显式传播，保留原diagnostic/events/evidence/ack；不增独立crate、自动SDK call/attempt schema或exporter。Span＋来源Dispatch冻结；no-op不误接ambient，Future/Stream/drop分别验证。宿主domain解释Result，host负责backend/flush；telemetry不成为dispatch/权限gate，也不隐藏EvidenceSink failure。替代的独立generic crate/trait需新增发布/lifecycle/schema适配，目前无已证非LLM独立consumer；只ambient宏无法满足多dispatcher隔离。

**代价、条件与证明。** tracing nativeclose/clone不等于业务settle，任意Subscriber panic/block不能无条件隔离，不能声称pi callback/schema API等价。合规host承担非阻塞录制和exportfailure抑制；方法/body只执行一次，业务Result不改。AIR-054–056/V-46–48验证public传播／序列化排除、Dispatch/poll/drop、secret/结果/evidence独立性。本轮未跑runtime，源码facts置信高、物理模块判断中高；性能/外部ecosystem未知。拆crate需真实consumer/shared conformance，自动spans需明确新范围，不让实施者按旧实验或未接受PR扩展。
