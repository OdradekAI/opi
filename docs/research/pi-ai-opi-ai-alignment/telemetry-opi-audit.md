# opi 遥测 SDK 架构审计

> 非规范性研究；2026-10-04。本文不修改现行规范、不构成实施准入，也不表示候选能力已经实现。所有结论来自本地静态阅读；本轮没有运行测试、联网、安装依赖、调用服务或修改运行时代码。仓库与 pi 快照来源沿用本目录基线记录，不把本次工作区文件称为已发布 opi 行为。本文是新研究材料，没有既有中英文 counterpart；不修改原 pi 独立研究。

> **后续确认：Q37／D-37 已选择 pi 职责划分与 Rust tracing 模块。** 当前目标以 [决策文档](decisions.md) D-37、[英文候选](spec-draft.md)／[中文候选](spec-draft.zh.md) AIR-054–056 及 [验收矩阵](acceptance-matrix.md) V-46–48 为准：显式 `from_span`／`noop`，匹配 Span＋Dispatch，供 custom provider 使用的 `in_scope`／lazy `instrument`，可信宿主 backend 边界；本轮不新增自动 SDK call/attempt spans、generic schema DSL、独立 telemetry crate 或 exporter。下文自动 instrumentation 与强故障抑制的待决比较是确认前的研究记录，已不构成当前候选的未决阻碍。

## 结论与审计边界

建议在 `opi-ai` 内补一个小的显式遥测上下文模块，优先复用已锁定的 `tracing`，由 `opi-agent` 沿既有依赖复用。现有证据不要求立即新增 `opi-telemetry` crate，更不要求 OTel、全局 subscriber 或默认 exporter。该建议以 SDK 可独立接入、父上下文稳定和真实生命周期为依据，不以旧规范的包归属限制 pi 已发布能力。

这一选择有三个必须公开的限制：`tracing::Span` 必须与来源 `Dispatch` 一起捕获；业务完成与原生 span 最后一次引用释放不是同一件事；任意宿主 Subscriber 可以同步阻塞或 panic，直接使用 tracing 不能无条件提供 pi 适配器的故障抑制保证。若目标要求任意 backend 故障也不干扰业务，必须另行界定适配器隔离边界，不能只给类型换名。

pi-ai 1.0.0 已有显式上下文透传，但本次检索没有发现其内置 provider 自动建立 logical-call/attempt spans 或统一 AI 域 schema。新增这些 span 是合理的 opi 候选目标，仍需范围确认；不能写成“照搬 pi 已交付 instrumentation”。

本次覆盖 `opi-ai` 的公共请求、prepared 生命周期、错误与诊断；`opi-agent` 的调用重试、diagnostics、事件扇出、evidence；coding harness、RPC trace 和必要的日志接线。对照本地 pi telemetry 包及 ai 的透传测试。Rust 事实使用本地缓存中与 `Cargo.lock` 对应的 `tracing 0.1.44`、`tracing-core 0.1.36` 源码；没有使用未锁定最新 Rustdoc。

## 证据索引

以下行号是本次阅读定位，不是要求未来保留的目录结构。`TSRC` 指 `C:/Users/Luiz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tracing-0.1.44/src/`；`TCORE` 指同一 registry 下 `tracing-core-0.1.36/src/`。

| ID | 静态事实及定位 | 能证明与不能证明的内容 |
|---|---|---|
| TO-01 | `Cargo.toml:79`、`crates/opi-ai/Cargo.toml:24`、`crates/opi-agent/Cargo.toml:23`、`crates/opi-coding-agent/Cargo.toml:43` 均已有 tracing；`Cargo.lock:3567–3595` 锁定上述版本 | 在这三个 crate 中检索 tracing 调用只发现 TO-02 的少量日志；未找到 SDK span/context 或 subscriber 初始化。锁文件未命中 tracing-subscriber/opentelemetry；不能推断外部宿主没有安装自己的 subscriber |
| TO-02 | `opi-agent/src/streaming_proxy.rs:255,394`；`opi-coding-agent/src/adapter_host.rs:232`、`execution/protocol_host.rs:950–965,1144` | 现有 warn/debug 是队列、正则、adapter、执行后端诊断。它们不是调用 span 树；错误正则日志包含 pattern/error，不能直接视为全部字段已通过统一敏感信息策略 |
| TO-03 | `crates/opi-ai/src/provider.rs:22–44,78–79,102–134`；`provider_collection.rs:416,507–550,764–815,871–899` | SDK 已有 Send stream、owned Request、冻结 auth/route 和 active attempt lease；Request 没有 telemetry 字段。当前 prepared 生命周期可作为接线位置，但不能从其存在推断未来 Models/retry 目标已实现 |
| TO-04 | `crates/opi-agent/src/diagnostic_sink.rs:23–41,68–101`；`agent_loop.rs:2690–2695`；`opi-coding-agent/src/harness.rs:1777–1779,1899–1903`、`runner.rs:313–315` | DiagnosticSink 同步无返回值；RecordingSink 是有序、无界的进程内 Vec；产品非交互路径真实启用 severity 计数。sink 文档“production NullSink today”不能代表全部当前产品路径。调用点直接执行 record，没有 panic 隔离 |
| TO-05 | `crates/opi-agent/src/agent.rs:28–46,56–73`；`tests/tool_event_redaction.rs:152–177,222,347`；`opi-coding-agent/src/main.rs:792` | 公共事件统一 redaction 后同步扇出，回调执行时不持 subscriber mutex；测试明确要求 subscriber panic 对调用方可见。该通道有产品输出用途，不能改造成可丢失遥测来“统一”接口 |
| TO-06 | `crates/opi-agent/src/evidence.rs:10–28,46–83,2267–2290`；`agent.rs:77–101,201–230`；`opi-coding-agent/src/rpc.rs:998–1020` | EvidenceSink 有 setup/emission/finalize/abandon 可失败事务和明确健康状态；RPC `trace` 返回这些已脱敏 evidence records，并非 tracing span 导出。RunId/TurnId/CallId 是证据身份，不是某个 exporter 的 span ID |
| TO-07 | `crates/opi-agent/src/agent_loop.rs:1091–1165`、`loop_types.rs:458`；候选 `spec-draft.md` AIR-007/039 及 retry annex | 当前 Agent 重试同一 PreparedCall，发 AutoRetryStart 和诊断，分配 Retry evidence CallId；候选改为 SDK 单一 acquisition retry owner 与 Agent 高层新 call recovery。遥测必须依最终实际 owner 接线，不能延续两层同名 retry 造成重复计数 |
| TO-08 | `.repo/pi-1.0.0/packages/telemetry/README.md:3–13,62,103,118–132,147–176,209`；`src/noop.ts`；`src/memory.ts:89–99,120–185` | pi 提供 vendor-neutral 显式 context、noop、memory、conformance；adapter 负责 exporter/flush/sampling，录制失败不影响 callback。memory 真实抑制记录失败、settlement 后 inert；memory 本身无界，不是生产容量模板 |
| TO-09 | `.repo/pi-1.0.0/packages/ai/src/types.ts:1,134–135`、`src/api/simple-options.ts:32`、`test/telemetry-options.test.ts:68–119,126–165`；`CHANGELOG.md:276,345` | 0.84.0（2026-08-06）发布记录说明 context 透传；1.0.0 本地源仍保留。测试检查 stream/simple/deferred/images 同一对象传递；不证明内置 provider 已创建 spans，亦不证明任意 exporter 故障已通过 live 验证 |
| TO-10 | `TSRC/span.rs:485–508,1363–1371,1431–1439`；`instrument.rs:202–221,383–388` | child_of 的父参数转换为 Id，仍选当前默认 Dispatch；可用公开 Span::with_subscriber 取来源 Dispatch。线程 scoped default 不自动跨 Tokio 线程；需显式捕获/恢复。不要依赖 doc-hidden child_of_with |
| TO-11 | `TSRC/span.rs:628–663,1455–1463`；`instrument.rs:236–252,274–285,315–321,383–388` | enter guard 不应跨 await；Instrumented Future 每次 poll/drop 进入 span；本锁版本没有 Instrumented Stream 实现。WithDispatch 只在 Future::poll 设置 default，未提供同样的 Drop 守卫；Span Drop 调用其来源 subscriber.try_close |
| TO-12 | `TCORE/dispatcher.rs:555–556,607–611,699–700`；`TSRC/subscriber.rs:33–42` | subscriber new_span/event/close 是同步直接调用，没有内建 catch/超时。tracing 明确告诫 library 不应 set_global_default；配置与 exporter 生命周期属于 host |
| TO-13 | `crates/opi-ai/src/provider.rs:419,479,557–599`、`http.rs:361`；`crates/opi-agent/src/diagnostic.rs:54–66,149–154` | 现有 typed error 分类、safe_excerpt、Summary redaction 可复用为观测事实源；safe_excerpt 的已知凭证模式并非对任意 prompt/tool/URL 内容的通用隐私证明 |
| TO-14 | `README.md:126,675`；`README.zh.md:592`；`crates/opi-coding-agent/src/cli.rs:259`、`main.rs:23–25` | verbose 文档称 debug tracing，当前 main 只输出 debug banner；未找到该开关初始化 subscriber。产品同时声明不收集 telemetry/analytics。未来接线需要澄清本地 opt-in 与对外传输，不可暗启网络或把当前帮助文字当实现证据 |
| TO-15 | `docs/opi-spec.md:236–259,269–270,438,508–523`；`docs/CONTEXT.md:9–32,41–44,76–79` | 显式 context、域词汇、无 exporter 绑定及脱敏有现行条款支持；evidence/session 权威与被动观测分离。独立 crate 不等于 Independent Companion 或 Standalone Project。按人类确认的新目标重新评价归属，旧门禁不是否决 pi 能力的依据 |

## 当前责任与消费者

| 通路 | 所有者与真实消费者 | 保留/补齐建议 |
|---|---|---|
| 普通 tracing 日志 | 各模块产生；输出取决于外部 host subscriber，当前二进制未检出初始化 | 保留有用日志；统一敏感字段审查。不要声称有完整链路追踪，也不要为 telemetry 单独建全局 logger |
| ProviderError / Diagnostic | SDK 产生 typed error；Agent 分类；coding harness 收集 severity；产品桥接 config/package/execution 错误 | 保留错误分类和诊断语义。允许从同一个事实构造独立遥测字段，不能把业务错误改为 telemetry error 或依赖 exporter 返回值 |
| AgentEvent | Agent 单一公共 redaction/fanout；UI/JSON/RPC 等同步消费者 | 保留其功能协议及现有 panic 可见性。只可增加只读适配消费，不能吞掉/采样功能事件来满足遥测被动性 |
| EvidenceSink / RPC trace | Agent owns correlation、health、artifact 生命周期；产品 file/memory recorder；evidence 消费者 | 保留可失败的证据事务。它失败可能使 run 不完整，这是既有业务契约；遥测掉队不得借此伪造成功 evidence，也不能使 evidence 事务失败 |
| 新显式 telemetry context | 候选 `opi-ai` owns 中立 SDK seam；Agent 复用；host owns backend | 补齐独立 SDK 接入。普通调用不要求 Agent、session、durable binding、evidence recorder 或产品配置 |

`DiagnosticSink`、`AgentEventSink`、`EvidenceSink` 名称相近不构成冗余证明。三者分别服务诊断计数、可观察产品协议、可验证证据生命周期，有真实消费者与不同失败规则。可合并的是重复的“从同一已分类事实提取安全字段”逻辑，不是三条通路本身。`diagnostic_bridge.rs:3–12` 的 product-owned error → Agent diagnostic 映射也有现有依赖/orphan-rule 原因，不应仅为遥测引入反向依赖。

## 关键审计发现

### TO-16：standalone SDK 的遥测接入缺口成立，但不能据此制造产品前提

条件：外部 Rust 项目只依赖 opi-ai，希望把一个调用挂到自身请求上下文并区分重试。TO-03 的 API 可以执行 provider 请求，却没有可传递的统一 context；当前少量日志也没有该关系。

反证：外部 caller 可以自行包住 complete future，或在消费 Stream 时建立自己的 span；所以不是“完全不能观测”。但这样无法稳定区分库内部 auth、实际 acquisition、backoff，尤其 upcoming owned/lazy handle 在其他任务中轮询时容易靠 ambient 猜父关系。

推荐：补齐 optional owned context，并保证所有 direct/simple/prepared 和 typed-operation 路径复用同一入口传递；默认明确 noop。若启用自动 instrumentation，负责调用状态机的模块唯一产生 call/attempt 事实，provider 只产生它真正拥有的 transport 细节。是否纳入自动 spans 与字段 schema 仍是待确认范围。置信度高。

保留现状的合理性是 caller 只需外层 duration 时已可满足，零公开 API 成本；代价是无法提供上述一致内部语义。不能把完整 Agent 域 schema 设为 standalone SDK 的必需依赖。

### TO-17：只保存 Span 或默认借 ambient，会破坏多实例父关系

TO-10 证明 `Span::child_of(&parent, ...)` 并不自动选择 parent 的 Dispatch。父 span 来自 subscriber A，而 handle 在 subscriber B 下首次 poll 时，直接创建 child 可能把 A 的 Id 交给 B。普通 single-global-subscriber 测试会掩盖问题。

推荐：显式上下文持有来源 span 与 Dispatch；需要 root recording 的 caller 显式提供自己的 root/context。无 context、disabled span、显式 noop 都短路录制，不回退 `Span::current()`。注意 `parent: None` 是“创建 root span”，不是 noop；省略 parent 才使用 ambient。两种形式都不能代替 noop。

同一 Dispatch 应覆盖 child 创建、poll、event/status 记录、finish 与 Drop 中属于该上下文的录制；不覆盖调用方其他代码。同步 guard 在一次 poll 返回前释放，不能跨 await 保存。跨线程传播只携带 Send/Sync owned handle；不引入全局 current-span singleton。置信度高；这是采用 tracing 后的正确性条件，不要求新 crate。

### TO-18：原生 span close、业务 terminal、资源 quiescence 是三个时点

TO-11 说明 `Span` clone 持有独立引用，最终 close 由 subscriber 生命周期决定；它不是 SDK 公共 terminal 的可靠替代。候选 AIR-003/004/005/035 又有取消优先级、未轮询 handle、已交付 terminal 不变和异步 cleanup 边界。

推荐目标（待范围确认）：

| 触发 | 语义录制 | 不变量/失败边界 |
|---|---|---|
| 创建 lazy handle | 捕获 owned input/context；不开始 call span 或计时 | drop-before-first-poll 零 source/network 工作；不伪造一次已执行 call |
| 首次 poll | 与 call-generation 登记同步开始逻辑 call 观测，再进入 auth/transform await | auth/preflight failure 可观测为此 call 的结果；遥测失败不触发 auth 或第二次调用 |
| 每个已开始 acquisition | 同一 call 下独立 attempt 序号；backoff 是调度事实 | 序号及发送计数从实际 retry owner 读取，不另建 telemetry retry policy；提前拒绝的 attempt 不伪造 HTTP send |
| 一个公共终止结果 | 以真实 typed result 进行一次 semantic settlement | 完成、失败、取消、异常 EOF 不被“future 正常返回”统称 success；Result::Err 原值和 partial/usage 不被录制改写 |
| 活跃 handle 被丢弃 | 一次 abandoned/cancel-observed/unknown-effect 分类，依据实际 call 状态 | 不报告正常成功，不新增请求；不以 telemetry context clone 保活业务任务 |
| 已 terminal 后 Drop | 不重复 settlement | 尚未释放的本地任务由既有 cleanup owner 处理，telemetry 不成为释放屏障 |
| 显式 cleanup | 可记录一个单独的管理结果与已知 effects | 成功只表示既有契约的本地 quiescence；不等价 exporter flush、远程取消或退款；ManagementFailure 仍是业务结果 |

“abandoned”和“caller cancelled”必须不同：没有观察到取消 token 的 Drop 不能杜撰用户取消。advanced AttemptOutcome 的 retryable acquisition failure 只终止 attempt，不提前终止 logical call；高层 recovery 建立新的 logical call，不复用旧 span 冒充同次请求。TO-07 的现有 evidence Retry CallId 也不能直接当作目标 acquisition-span 身份。

锁定版 `Instrumented<T>` 只实现 Future；标准 Stream 需要私有 wrapper 在每次 poll_next 的短同步范围恢复 context。`WithDispatch` 只对 poll 设置 default；不要把组合 `.instrument(...).with_subscriber(...)` 当成 Drop 全覆盖证明。Drop 中需要录制的动作也应由同一小模块明确包住。无需为此公开新的 stream trait、第二 shutdown 入口或泛型 lifecycle framework。置信度高。

### TO-19：被动性可以是适配器契约，不能靠 tracing API 名称自动成立

pi 的保证见 TO-08：录制失败被忽略、callback 恰好执行一次、原值/拒绝不变。Rust tracing 的直接 subscriber 调用见 TO-12；一个任意 Subscriber 可以在 new_span、event、enter/exit、try_close 中阻塞或 panic。给 call 外层加一个 catch_unwind 既不能中断阻塞，也可能混淆业务 panic；panic=abort 亦不受 catch 保护。

建议优先采用可信宿主适配器边界：库不主动安装 subscriber、不做 exporter I/O/flush、不创建后台 exporter、不把录制错误提升为 ProviderError；宿主适配器声明同步录制应短且不 panic，内部 exporter 失败自行抑制，缓冲/丢弃/flush 由 host 决定。库提供 noop 与测试 recorder/适配器 conformance 验证它能控制的行为。这与当前 DiagnosticSink 的“调用应廉价且正常输入不 panic”同类，但不是对任意第三方 subscriber 的无条件隔离保证。

若人类要求更强的可故障注入保证，可考虑 backend-neutral、对象安全的 context/record adapter seam，库在单个录制操作外围抑制可捕获失败，callback/Future 的业务执行保持在捕获范围外。仍需声明不阻塞契约和 panic=abort 边界；不能声称能隔离恶意或永不返回的宿主代码。先决定保证，再选类型；新 trait/new crate 本身不解决此问题。

所有 `catch_unwind`、异步 exporter 隊列、额外工作线程都不是本轮默认建议。尤其不能为“被动”而丢弃 AgentEvent 或忽略 EvidenceSink 失败。置信度高。

### TO-20：tracing-backed context 与 pi generic schema 并非逐 API 等价

tracing 的 span/event metadata 和字段通常由静态 callsite 声明；pi 低层 API 接受任意 name/attribute bag，并有显式 last-write-wins status 与 settlement 后 inert。直接公开 `Span` 不会自动得到这套契约。

推荐缩小公开面：opaque context 与明确、版本化的域字段；内部复用 tracing 处理宿主集成。预先声明可晚填字段与 outcome/status，失败时只记录 typed category、明确安全的 code 和未知原因；不 Debug 整个 Request/Error/Context。如果需要重置/覆盖属性，必须说明后端能观察到的逻辑值，不能假设 tracing subscriber 都用相同 merge 方式。不要机械移植 TypeScript mapped-type/schema-builder 系统。

已有 redaction 和 usage 分类可共享事实，但 unknown 不能填 0；raw prompt、工具内容、credential、URL query、原始 provider body 默认不采集。high-cardinality ID 不宜自动作为指标 label。只有显式批准的字段进入 schema，基于已有 typed value 转换，而非复制整个诊断 JSON。置信度高。

### TO-21：独立 crate 当前没有必要，但存在明确重新评估条件

| 方案 | 当前收益 | 当前代价/限制 | 建议 |
|---|---|---|---|
| 只用 ambient tracing 宏 | 最少代码；Rust host 生态熟悉 | 不能满足 TO-17 的显式传播/noop；无法单独表达稳定 completion | 不采用为完整方案 |
| `opi-ai::telemetry` 的 tracing-backed opaque context + 私有生命周期实现 | 已有依赖；SDK 单独可用；Agent 已依赖 ai；隐藏 Dispatch/poll/drop 复杂度 | API 明确依赖 tracing 生态；强故障隔离与任意动态 schema 不自动成立 | 当前推荐，先确认 TO-19 保证与自动 spans 范围 |
| `opi-ai` 中 backend-neutral adapter trait | 可描述 noop/memory/其他 backend 的共同语义 | 新公开实现责任与 conformance；未证明现有 direct tracing 集成需求不足；不能解决任意阻塞 | 若要求 backend-independent adapter/fault semantics，再比较 |
| 独立 `opi-telemetry` crate | 无 LLM 依赖的领域可单独使用；有机会独立版本化 | 新 crate/API/version/依赖方向维护；当前所查三 crate 已有 ai 依赖，没有循环需要拆分 | 延后，不以 pi package 数量作为拆 crate 证据 |

重新评估触发：出现一个真实消费者，必须获得同一 telemetry contract 却不应拉入 LLM SDK 依赖，并且现有 opi-ai 消费方与其共享可执行 conformance；或出现真实多 backend 适配器要求无法由当前 opaque context 清楚实现。把 `opi-eval` 当成自动第二消费者不成立：本轮没有证明它需要接入运行中的 SDK context，读独立 evidence 也不等于依赖此库。将来提取时保留契约和迁移路径，不能永久保留两套互相转发 owner。置信度中高。

## 与现行参考规范的关系

1. `opi-spec §6.1:238–241`、CTRL-001 与显式 context、无 exporter 绑定一致；可复用既有 run/turn/call 关联作为安全属性，不要求 SDK mint Agent evidence ID。若需要修改域词汇或将最低 context owner 明确放在 opi-ai，应同步规范说明，不声称当前实现已满足。
2. CTRL-002/003 和 INV-007/008 使 evidence/persistence 具有独立权威。将这些失败吞掉以满足 telemetry passivity 会实质冲突；本建议保留它们，不新增普通 SDK durable gate。
3. §8.1 的真实 seam、生命周期与最小权限可帮助比较方案；按本轮人类重新确认的 pi Rust 实现目标，它们不是拒绝 pi 已发布能力的自动依据。新增独立 crate 时需要审视 workspace 图、manifests、公共依赖及文档，不应从“Independent Companion”术语推出必须拆包。
4. README 的“不收集 telemetry/analytics”需要在未来接线时精确定义：提供可由宿主启用的本地观测 contract，不等于默认向外传数据。`--verbose` 当前接线缺口只需修正真实承诺；本次并未授权自动添加 exporter 或后台上传。

## 验证设计与当前测试边界

以下全部是未来验收设计，不是本轮通过记录。

| 检查 | 机械可观察结果 | 防止误证 |
|---|---|---|
| standalone 外部 fixture 仅依赖 opi-ai | direct/simple/complete/stream/result/advanced 使用同一 optional context；不依赖 opi-agent/coding-agent/session/evidence | crate 编译与行为测试分别证明依赖和语义；不能仅通过 re-export 名称判断 |
| 两个 recording Subscriber A/B，A 下创建父/context，B 下 poll/drop | 所有 SDK 子事件归 A 的父；B 零错误子事件；两个 Models 同 ID 不串 | 只测单一 global subscriber 不够；覆盖首次 poll、pending 后迁移线程和 Drop |
| 无 context、显式 noop、disabled parent | 不因 ambient recorder 而产生 SDK 域 span；不构造/序列化敏感 payload | explicit root 与 noop 分开测试；不能以采样后“没看到”代替 noop 路径证明 |
| Future/Stream 生命周期 | unpolled drop 无开始；Pending 不结束；终止后一次 finish；poll 永久 EOF；Drop 分类准确 | 错误/取消/异常 EOF 与 success 分别验证；native Span close 延迟不能改变 semantic finish 次数 |
| retry/recovery/cleanup | call 与 attempt/new-call 数目匹配真实 sends/预算；无重复起调度；cleanup 部分失败保留业务结果 | 不让 telemetry 自己算新的 retry index/预算；不要要求 exporter flush 才完成 cleanup |
| 被动适配器 conformance | noop 与正常 recorder 下业务返回/错误/请求数量一致；可失败 exporter 不改变业务；禁二次 callback/poll | 故障边界按 TO-19 决策测试；不能据此宣称任意 subscriber blocking/panic 被隔离 |
| 安全字段 canary | 请求、headers、凭据、工具、路径、provider 原始错误 canary 不进入默认 telemetry；unknown 保留缺失/原因 | 已有 safe_excerpt 的有限样本不是全部数据敏感性证明；不靠采样掩盖泄漏 |
| host ownership | 库不调用 global-init、不启动 exporter网络/线程、不清空host subscriber、不释放host外部资源 | 静态扫描配合独立实例 fixture；合法 host 安装 subscriber 不算库全局副作用 |
| 既有通路不退化 | AgentEvent 仍单次红化/功能顺序；EvidenceSink 原失败/health/manifest行为保留；诊断计数一致 | 必须保留现有功能协议语义，不因“统一遥测”重写测试使错误行为通过 |

现有 `opi-ai/tests/provider_diagnostics.rs:29–167,198–319` 可证明 typed taxonomy 和具体 safe_excerpt 输入的预期；`opi-agent/tests/diagnostics_runtime.rs:242–275,669–894` 覆盖记录、重试、取消等诊断；`tool_event_redaction.rs:152–177,222,347` 固化事件边界；evidence 测试及 RPC trace 测试验证其自身合同。它们均不证明显式 tracing parent、cross-Dispatch、自动 SDK span、subscriber passivity 或 Stream settlement。pi 的 telemetry conformance 可以作为语义对照，但应保留 Rust lazy poll/Drop 的有意差异，不能复制 JS callback 的 eager timing。

测试影响建议：`add` SDK context/生命周期/隔离 conformance；`update` 未来被单一 retry owner 改写的诊断/事件对应关系和实际 verbose 帮助；`retain` 当前 typed errors、redaction、AgentEvent、EvidenceSink/trace tests；`delete` 当前没有已证明可删的测试。此列表是候选实施影响，本次实际测试变更为 `none`。

## 真正待决项

1. **范围**：仅对齐 pi 已发布的 explicit context 透传，还是同时增加 SDK logical-call/attempt 与 Agent recovery 的自动固定域 spans？推荐后者有实际可诊断价值，但属于新增目标；应固定最小字段、owner 与完成规则，不一次纳入所有 provider/turn/tool/compaction/session 自动 instrumentation。
2. **保证与后端耦合**：接受 tracing-backed context，采用符合契约的 host Subscriber 边界并明确任意阻塞/panic 不受库隔离；还是要求 backend-neutral adapter 和可捕获录制失败的强 conformance？推荐先者作为 Rust 集成的最小一致设计，但不能宣称与 pi 的强适配器 passivity 完全等价。若强 passivity 是硬性对齐目标，需先选择后者的明确故障边界，再决定是否仍放在 opi-ai 模块；它并不自动需要新 crate。

其余事实不需用户决定：来源 Dispatch、Future/Stream/Drop 差异、native close 不等于业务完成、当前生产 trace 是 evidence、现有消费者不能无依据删除，均可从本地源码查明。

## 本轮检查记录

执行了只读 `rg` 与 PowerShell `Get-Content`，检查上述源码、manifests、lock、测试、候选 spec 与本地锁定 dependency 源。两次导航假设不存在（`opi-coding-agent/src/summary.rs`、`opi-agent/src/retry.rs`），随后定位到 `runner.rs`/harness 与 `opi-ai::retry`、Agent loop；pi testing 实际位于 `src/testing/`。未把缺失文件当作缺失能力。

没有运行 Rust/Node 测试、doc checker 或编译 probe；本任务只产出此研究文档，最终文档检查由主 Agent 对本轮组合变更执行。没有修改现行规范或共享候选 spec。

## Q37 确认后独立复核

已重新读取中英文候选 AIR-054–056、telemetry annex、D-37 和 V-46–48。未发现需要阻止该方案进入后续正式规范协调的实质矛盾：noop 的 scoped disabled dispatcher 仍执行同步业务一次、Future 不多 poll 并恢复宿主上下文；非 noop 从原 Span 取其 Dispatch，不以轮询时 ambient 替代；Future/Stream/Drop 与 producer handoff 分别约束，未把 locked tracing 的 Future combinator 误当全部覆盖。helper 不创建 domain span/status，native parent 生命周期由宿主拥有；任意 subscriber panic/block 不被虚假承诺隔离。既有 DiagnosticSink、AgentEvents、EvidenceSink 与 session acknowledgement 保留原失败及权威语义，中英文对此一致。

这是对可实施契约的静态复核，不是 runtime 实现通过。禁用 scoped tracing dispatcher 约束的是本 context 下的 tracing 观测；宿主自建的独立日志系统、显式使用其他 Span/Dispatch 或主动替换 scoped subscriber 不属于该机制可强制控制的范围。当前工作区锁定 tracing 未启用 `log` 依赖；未来宿主 feature unification 启用 `log-always` 时，不应把 noop 文档扩张为“静默整个宿主所有日志”的保证（locked `tracing/src/lib.rs:647–661`）。本次只更新本文件的当前决策指针与复核结论，未运行代码或修改其他文档。
