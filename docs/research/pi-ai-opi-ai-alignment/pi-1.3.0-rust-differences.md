# Pi 1.3.0：Rust 差异注册表

> 本页现行基线为本地 Pi 1.3.0；[逐项升级复核](pi-1.3.0-upgrade-review.md)与[固定来源](pi-1.3.0-source-evidence.json)区分本次重读和同摘要继承。旧 1.2.0 审计仅保留原证据日期；本页原源码轮未执行运行验收。最新[Schema／WS离线实验](pi-1.3.0-schema-ws-offline-validation.md)另列实际执行与候选判定，不代表产品验收。

> Phase 20 当前差异合同附件。目标仅限 `opi-ai`：对齐固定 Pi 1.3.0 的公开能力与可观察语义，采用 Rust 惯用接口，逐项说明差异。目标要求与采纳边界以[双语交付来源](spec-draft.zh.md)（[English](spec-draft.md)）为唯一正文，机械验收以[acceptance matrix](acceptance-matrix.md)为唯一索引；本页不新增 AIR/V、实施准入或进度账本。此前 D/Q 保留其时间和版本限定，旧研究结果不成为当前产品验收。
>
> 本页依据[首轮审查](pi-1.2.0-review.md)、[五问补核](pi-1.2.0-followup.md)、本地 Pi 1.3.0 与当前 Rust 源码静态核对。本轮Opi输入为 `c5ab42ab8888b93f9969652323caee87b7550392`；Rust运行事实沿用旧审查并按未变代码继承，Pi新来源身份见[source evidence](pi-1.3.0-source-evidence.json)。本文的 Rust 契约均为**待实施和证明的目标**，不是当前 API 可编译、测试通过或远端可用的声明。中文附件没有既有英文 counterpart，由主双语 spec 共同引用。

> 本轮[参数与新增限制来源审计](pi-1.2.0-parameter-audit.md)纠正前轮结论：撤回无来源默认值/额外拒绝，backend选择有条件，Schema/transport差异仍未证明。前轮baseline/评审仅对原输入有效，不能替代本轮接受集证明。

## 分类和使用规则

“等价”检查调用方能完成的动作，以及输入、输出、失败、状态、回调、取消、重试、资源与可用性。语法、对象形状或运行时目录相同不构成等价证明；同一最终字符串也不够。正式父规范的 [GOAL-003、§3.2–3.3](../../opi-spec.md#3-first-design-doctrine)允许 Rust 原生设计，同时保留 PRIN-002/003、INV-001/002 和实施准入权威。本附件没有发现需要先修改整个项目定位才能表达的语言障碍。

| 分类 | 判断标准 | 能否关闭能力义务 |
|---|---|---|
| 语言表达 | snake_case、enum、`Result`、owned 值、Future/Stream、对象安全异步边界 | 只有动作及失败信息仍可达，并通过对应验收时，才能作为语义对应；不是自动通过 |
| 安全／资源行为 | 明确收窄接受集、强化验证、保留 unknown、背压、取消或清理保证 | 必须明示外部变化和理由；不能称 Rust 语言强制，也不能归一化掉差异 |
| 范围／平台 | SDK 与宿主责任、Cargo/module 与 npm/Bun、native runtime 支持面 | 可以不复制非目标符号或平台桥，但 SDK 对应功能不能随之消失 |
| 真正能力缺口 | 尚无公开可达实现、协议接线或机械证明 | 保持未完成；不能以“Rust 惯用”“deferred”或历史子集选择关闭 |

以下 RDF-01–20 是固定的差异定位符，不是新要求或验收编号。一个条目可能含两类因素，但必须分别解释。每项的“机械验证”描述未来产品证据；前置源码轮没有执行 Cargo/Node 测试、构建、live/OAuth 请求、依赖安装或生成数据。最新离线harness另列实际执行，不冒充未来opi-ai公开consumer。

| ID | 差异 | 主分类 | 主验收导航 |
|---|---|---|---|
| RDF-01 | Rust 调用形式和 typed 公共边界 | 语言表达 | V-39/42/49/58 |
| RDF-02 | owned 捕获、惰性启动和响应时长 | 语言表达＋资源行为 | V-39/42/53 |
| RDF-03 | `Result` 失败携完整 assistant | 语言表达 | V-03/40/41 |
| RDF-04 | 容量 64、背压和主动 result drain | 资源行为 | V-04/40/41 |
| RDF-05 | 不可变事件时点 partial；compact接口退役 | 语言表达＋范围变化 | V-41；V-52 retired |
| RDF-06 | typed 脱敏失败、边界验证和有限预算 | 安全／资源行为 | V-02/09/13/21–26/50 |
| RDF-07 | usage knownness 与整数精度 | 安全行为＋语言表达 | V-10/13/40/45/54 |
| RDF-08 | headers／payload 扩展受身份约束 | 安全行为 | V-12/34/36/51 |
| RDF-09 | JSON Schema normalization，不复制 TypeBox 私有路径 | 语言表达＋安全行为 | V-11/35/39/41 |
| RDF-10 | OAuth rotation owner 与 caller 等待分离 | 安全／资源行为 | V-12/43/55 |
| RDF-11 | 对象安全的 store 原子更新 | 语言表达＋安全行为 | V-12/42/43/55 |
| RDF-12 | OAuth expiry、输入验证和来源失败关闭 | 安全行为 | V-12/43/55 |
| RDF-13 | 实例可 await cleanup 与 Drop | 资源行为 | V-04/29/42/58 |
| RDF-14 | 显式 tracing context，不复制 telemetry DSL/status | 语言表达＋范围 | V-46–48/58 |
| RDF-15 | faux 的可取消调度 | 资源行为 | V-37/38/49 |
| RDF-16 | 目录 knownness 与 publication 一致性修正 | 安全／资源行为 | V-27/28/54/56 |
| RDF-17 | 唯一 retry owner 与 strict 降级变体 | 安全／资源行为 | V-05/34/36/57 |
| RDF-18 | Cargo/native 平台与 npm/Bun/browser/CLI 边界 | 范围／平台 | V-26/42/49/58 |
| RDF-19 | SDK 与 host-only 集成责任 | 范围 | V-20/39/49/58 |
| RDF-20 | 全公开能力覆盖仍须实施证明 | 真正能力缺口 | V-49–59 及既有对应 V |

## 逐项契约

### RDF-01 · Rust 调用形式和 typed 公共边界

- **Pi 事实：** [`Models`](../../../.repo/pi-1.3.0/packages/ai/src/models.ts#L255)提供实例管理、native/simple stream/complete、完整自定义 model 及按操作查询；[`ApiOptionsMap`／KnownApi](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)区分协议与 provider。provider ID、API/wire、model ID 和 endpoint/deployment 是不同身份。
- **Rust 目标：** 日常Models与advanced共用状态/typed native options；闭合操作、开放精确ApiId、完整custom model不要求catalog membership。RegisteredChatApi<O>/fixed-O dyn与opaque API/type tag是Rust表达；兼容Models间选项可复用，同type替换不单因revision拒绝，注册沿upsert。不添加ID trim/grammar/length或blanket namespace/stale拒绝。旧owned call冻结完整adapter/record；真类型错配typed失败，不公开Any/通用JSON executor。
- **理由与分类：** 命名、enum 和借用／拥有形式属于语言表达；wire mismatch 提前 typed 失败是明确边界行为，不能把任意 TypeScript `Record` 动态值等同于可执行配置。不能为符号对称创建 `opi-context`、`opi-telemetry` 或通用 operation/plugin 框架。
- **外部 caller 影响：** 调用动作对应，源代码和序列化字节不互换。已有 0.x `ProviderCollection`、`CompletedRequest`、Config/Model 壳的消费者需在实际准入后同步迁移；本附件不授权删除。
- **机械验证：** V-39/42/49/58 用 workspace 外 consumer 覆盖查询、注册／替换／移除／清空、custom record、typed native mismatch、手动工具循环和双实例隔离；public export 存在性不能替代 wire/失败行为证明。
- **历史 D 依赖：** D-03/21/28/30/35 提供收敛与 SDK 理由；当前1.3.0覆盖由本轮包内目标要求，不能从旧 D-01/11 的子集或排除项推导。

### RDF-02 · owned 捕获、惰性启动和响应时长

- **Pi 事实：** [`Models.stream`](../../../.repo/pi-1.3.0/packages/ai/src/models.ts#L882)同步 normalize 后进入 [`lazyStream`](../../../.repo/pi-1.3.0/packages/ai/src/api/lazy.ts#L40)，后者立即调用 async setup；这里的 lazy 指模块载入，不表示等 async iterator 首次消费才鉴权。direct adapter 也立即创建响应流并启动异步 body，例如 [OpenAI Chat](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts)。[`AssistantMessageEventStream`](../../../.repo/pi-1.3.0/packages/ai/src/utils/event-stream.ts#L91)从创建起用单调时钟补 terminal `durationMs`，已有值或 message 时间戳早于流时不补。
- **Rust 目标：** 创建 handle／owned Future 时捕获 Context/options/provider/model revision；不能把冻结推迟到首次 poll。首次 poll 前 drop 为零来源／store／网络工作。首次 poll 在任意 auth/transform await 前登记资源 generation。响应 `duration_ms` 从 **adapter response stream 首次 poll／激活** 到 terminal 单调计量；Models 的 auth/loading 在该边界之外，direct adapter 同规则。未激活 response stream 的 setup failure 可无 duration，不伪造 0。已有 duration 和较早 deferred 结果不重计，collector／转发保留同一值；timestamp 与测量来源须清楚。
- **理由与分类：** owned 捕获保证调用创建后 caller 修改不影响此 call；惰性执行符合 Rust Future。两者不是同一时点。计时边界是显式可观察差异，不能宣称“从创建到完成”或精确覆盖整个 Models 操作。duration 包含该响应流激活后的本地等待和消费／背压时间，不是 provider 服务耗时或 benchmark。
- **外部 caller 影响：** 必须 await/poll 或 spawn 才启动工作；在 Future 创建与 poll 之间等待不会计入 response duration。宿主若需 auth/loading/排队的整体时长，应自行包围调用；库不为此自动建 span。
- **机械验证：** V-39/42/53 用可控时钟／gate 固定创建后改输入、首次 poll 前 drop、A 下创建 B 下 poll、auth 阻塞、direct adapter、partial failure/cancel、已有值及较早 deferred；三种 collector 路径保留同值。只断言 timer 变量存在不够。
- **历史 D 依赖：** D-04/30/35/36 的 owned/lifetime 原则；D-31 的统一结果。duration 是 1.2.0 新增可观察面，旧 D 不构成本次单项批准。

### RDF-03 · `Result` 失败携完整 assistant

- **Pi 事实：** [`EventStream.result()`](../../../.repo/pi-1.3.0/packages/ai/src/utils/event-stream.ts#L85)解析终态结果；`done` 返回 message，`error` 也返回 assistant message。setup failure 的构造见 [`createSetupErrorMessage`](../../../.repo/pi-1.3.0/packages/ai/src/api/lazy.ts#L4)，并非普通 rejected result Promise。
- **Rust 目标：** complete 与 `ChatStream::result(self)` 都为 `Result<AssistantMessage, CallFailure>`；Stop/Length/ToolUse/Deferred 为 Ok，Error/Aborted 为 Err。`CallFailure` 保留完整 canonical partial/terminal assistant、来源、诊断、已知 usage 与 typed 脱敏 cause；setup failure 无生成内容且 usage unknown。流只产生一次 Err 后永久 EOF，不再 Ok(Error) 与 transport Err 双写。
- **理由与分类：** 这是 Rust 错误表达差异，保留 Pi 调用者能够取到的生成结果信息。失败不能因 `?` 转成只剩字符串，也不能把 Length／Deferred 视为失败或执行工具许可。
- **外部 caller 影响：** caller 通过匹配 Err 取得 partial，并自行选择记录或恢复；工具不会自动执行，失败流不会隐式拼接另一个 logical call。需要迁移旧双错误处理者。
- **机械验证：** V-03/40/41 固定 setup、partial、取消、异常 EOF、终态后输入、Length/ToolUse/Deferred；complete、直接 result、部分／完整迭代后 result 得到同一 canonical 终态与失败信息。
- **历史 D 依赖：** D-31／Q32 是实际确认的 Rust Result 选择，D-04 为终态正确性；其批准不证明新公共类型已经实现。

### RDF-04 · 容量 64、背压和主动 result drain

- **Pi 事实：** [`FifoQueue`／`EventStream`](../../../.repo/pi-1.3.0/packages/ai/src/utils/event-stream.ts#L3)不设队列上限；final-result Promise 由 producer 推入 terminal 时独立解决，无需先迭代消费队列。
- **Rust 目标：** 保留现opi队列容量64及可取消背压；这是opi内部调度差异，Pi队列无该count check。撤回logical-byte cap、single-event超队列拒绝和terminal envelope阈值。合法大event仍须可交付且不deadlock；single terminal/完整partial与已知usage不依赖queue permits。result(self)主动drain同一结果，不保存无限archive、不重发、不self-cancel；已交付final不撤回。
- **理由与分类：** 有界内存和明确 backpressure 是资源策略，不是 Rust 唯一可能的实现；其完成时机与 Pi 不相同。单独被动等待 final Promise 会在满队列下死锁，不满足同样的 caller 动作。
- **外部 caller 影响：** 暂停消费可能暂停 producer 并延长本地 duration；caller 可直接 await result，不必先启动另一 drain task。drop 表示放弃本地 call/attempt，不承诺远端停止或退款。
- **机械验证：** V-04/40/41 必须超过 64 个事件，分别不消费、消费一半、读到 terminal、满队列取消和 drop；证明终态唯一、永久 EOF、零后续 dispatch 与 producer 退出，而非仅用短流。
- **历史 D 依赖：** D-04/30/31；64 是已选资源值，不能称 Pi 缓冲方式或 Rust 语言要求。

### RDF-05 · 不可变事件时点 partial；compact 接口退役

- **Pi 事实：** 1.3.0 EventStream 的 partial 仍是 live accumulator 引用；compact AssistantMessageFrame/encoder/reducer 及其测试已删除，见[版本差异](pi-1.3.0-upgrade-review.md)。不能继续沿用1.2.0 frames能力要求。
- **Rust目标与理由：** partial 保持owned不可变事件时点视图，可用内部结构共享，不强制每delta深clone或声称性能优势。ordinary start/delta/end、block index/顺序、signature/opaque、完整final Result保持；不因Rust snapshot新增另一套compact格式、reducer或持久化义务。无新增bytes/block/depth默认cap。
- **调用方与验证：** 旧partial不会随未来变动；V-41保留立即/延迟消费、交错blocks、end权威元数据、完整终态和canonical不变。AIR-060/V-52退役不算验收通过，不削弱现有事件/结果义务；不改宿主持久化格式。差分在JS事件发生时抓snapshot，不能归一化掉alias差异。
- **历史：** D-30/31保留owned事件理由；1.2.0 compact细节仅可由Git历史恢复，不再是当前交付合同。

### RDF-06 · typed 脱敏失败、边界验证和有限预算

- **Pi 事实：** [`ModelsError`](../../../.repo/pi-1.3.0/packages/ai/src/utils/models-error.ts)提供有限 code，但 adapter 与 validation 仍可能把上游 body／原始参数拼入错误；例如 [`validateToolArguments`](../../../.repo/pi-1.3.0/packages/ai/src/utils/validation.ts)。图片、classifier、不同 transport 的校验与 timeout 支持并不完全一致。
- **Rust 目标：** 保留typed脱敏错误、unknown/known信息、checked arithmetic/真实allocation失败、协议关联与各adapter真实required/type/finite校验。撤回所有无来源frame/retained/schema/media/classifier/body/depth/count默认cap及统一positive-only/no-unlimited。Pi接收的data URL、text-only/空image结果、跳过非匹配URI、宽classifier finite结果不能被strict base64/signature/四MIME/domain/label-completeness拒绝替代。省略caller resource policy不启用这些政策。source-backed参数与backend不可配置限制详见来源审计；后者保持未关闭coverage，不借typed Unsupported冒称等价。
- **理由与分类：** 错误隐私、fail-closed 和有界资源是明确行为选择，不是 `thiserror` 自动保证。基础型别／安全拒绝不能演变成随意收窄 Pi 的合法输入，或用 Rust 严格性掩盖未实现 adapter。
- **外部 caller 影响：** 既有秘密保护/typed错误表达仍有明确差异；没有新默认容量拒绝、隐式截断或固定总deadline。Rust返回owned bytes不能自行扩大媒体校验；必要时保留原encoded payload与可选decode状态，不能因backend解码更严格丢掉Pi可观察结果。显式host policy只影响选择它的调用，不称默认等价。
- **机械验证：** 对应 V-02/09/13/21–26/50 的负例、边界值和 canary；保留可观察失败类别、已知 partial/usage、取消与作用边界。安全差异 fixture 单列预期，不把 Pi 错误当成功 oracle。
- **历史 D 依赖：** D-04/09/10/16/20/25/28；旧 D-16/25 的预算选择不授权漏掉新的协议支持。

### RDF-07 · usage knownness 与整数精度

- **Pi 事实：** [`Usage`](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)使用 JavaScript number；adapter/setup 常从零桶开始，例如 [Chat output](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts)。[`calculateContextTokens`](../../../.repo/pi-1.3.0/packages/ai/src/utils/estimate.ts#L18)以正 total 或 parent 和为估计 anchor，未单独表达 Rust 的 knownness。
- **Rust 目标：** 缺失／unknown 与 reported zero 分开，setup/未知 attempt 不补 reported zero；独立 reported total 与 parent buckets 均保留，child subset 不重复计费。保留 Rust 可精确表示的整数，不人为复制 JS binary64 解析损失；超出自身整数／资源范围按明确 typed 规则处理，不能静默 wrap。费用仍需 finite/domain 校验，未知价格不推为免费，估计不冒充账单。
- **理由与分类：** knownness 是信息完整性选择；整数表示是语言差异。当前 [Rust Usage](../../../crates/opi-ai/src/stream.rs#L40)已有 reported/unknown 与整数，不能称全部从零新增；当前形状也不是每桶 knownness／独立 total 已完成的证明。
- **外部 caller 影响：** caller 需处理 Option/knownness，不应把缺失当 0；跨语言较大整数 JSON 值可能不同。UTF-16 计量、每 message 取整与 **3.5** 系数仍须对齐 Pi，不能拿 UTF-8 length 当“更 Rust”的替代。
- **机械验证：** V-10/13/40/45/54 覆盖 unknown/zero/positive、total≠sum、部分桶缺失、child subset、大于 JS safe integer 的 raw/effective、溢出与 finite；只允许已声明数值差异，不掩盖内容／顺序／费用元数据差距。
- **历史 D 依赖：** D-10/20/25/31/34；旧 `/4` 估计不是可保留的 Rust 差异。

### RDF-08 · headers／payload 扩展受身份约束

- **Pi 事实：** [`applyAuth`](../../../.repo/pi-1.3.0/packages/ai/src/models.ts#L850)合并 auth 与 caller headers，再运行 `transformHeaders`；[`ProviderHeaders`](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)允许 null remove；payload/response/raw provider event callbacks 是不同入口。Bedrock 对部分保留 header 静默忽略，其他 adapter 的覆盖行为也不能概括成一个规则。
- **Rust 目标：** 保留static set/remove、dynamic headers/payload/response/provider-event callbacks及真实支持面。HTTP header语法有效性与protected auth/routing/identity保留，不增加任意header大小限。显式key/source在auth前选择；callback失败不被telemetry吞掉，普通retry复用输入。撤回blanket redirect:none及ADC same-origin5-hop例外；按具体Pi fetch/SDK/source的method/body/credential-stripping与retry计数对应，真实secret/目标权限边界明确列差异。custom transport不二次包、不反射担保opaque client。
- **理由与分类：** 扩展存在性属于能力；身份保护和拒绝方式属于明确安全差异。当前 [`provider_headers.rs`](../../../crates/opi-ai/src/provider_headers.rs#L9)的保护列表可作既有证据，但其“不覆盖任何 configured header”也不能代替目标合法 set/remove 合同。
- **外部 caller 影响：** 可用动态定制仍保留；依赖 callback 偷换凭据／身份的 Pi 用法在此拒绝，须改用显式来源或另建 provider。observer 数据只读且受脱敏策略，不能从 response callback 重写业务结果。
- **机械验证：** V-12/34/36/51 固定每个 callback 阶段、顺序、次数、取消、async failure、大小写／remove／CRLF／protected canary、未知 adapter 支持；同时证明正常定制实际到 wire，不能只有拒绝测试。
- **历史 D 依赖：** D-09/21/27；Q30 曾选更窄 callback 面，不能用其历史排除关闭 1.3.0 动态 headers/raw events。

### RDF-09 · JSON Schema normalization，不复制 TypeBox 私有路径

- **Pi 事实：** [`validateToolArguments`](../../../.repo/pi-1.3.0/packages/ai/src/utils/validation.ts)clone 参数，normalize optional-null，经过 TypeBox Value.Convert；无 `Symbol.for("TypeBox.Kind")` 时还运行 plain JSON Schema coercion，再 validation。两条路径并非完全相同算法。
- **Rust 目标：** 选定私有JSON-only TypeBox方言IR/evaluator＋regress0.12.0算法与可重建Unicode16数据；jsonschema0.28.3退出本默认路径。结构guard、mixed applicator/ref/evaluated状态、已知format/近似长度/数值与分支Result由方言owner表达，不先套metaschema、不全pattern guard或伪成功。ordered JSON采用preserve_order＋JS整数键枚举；ryu-js1.0.2与受控Unicode16 URL/IDNA/ICU profile保留JS叶语义。10个namespace后端包有registry分发维护合同，不依赖caller rootpatch。clone-before-normalize与raw/effective分离、原schema最终check及root漏洞修正继续保留，零schema I/O。
- **理由与分类：** JSON-only schema 是语言／包边界差异；UTF8公共String不表示孤立UTF16 surrogate，非JSON运行时值不隐式转换为有效JSON：保留raw text/bytes与parse状态并typed失败，不替换后假称同值。保留 raw 与拒绝坏关联是安全选择。并不意味着不做 normalization，也不承诺任意 TypeBox runtime 值在 Rust 可重建。
- **外部 caller 影响：** caller 可在独立 SDK 手动验证工具，不必有 hook/authorizer；拿到 raw 与 normalized 的区别。执行和授权属于宿主，库不会因为验证成功就执行；host-only hook 顺序不作为 SDK 调用前提。
- **机械验证：** V-11/35/39/41 用 plain JSON 的 optional null、union 保留优先、数组／additionalProperties、numeric string/boolean、非法根值及 precision corpus；TypeBox-only cases 作为显式差异，不通过删除失败或统一 coercion 输出掩盖。
- **历史 D 依赖：** D-12/20/22/29/30 与 RA-08/13；D-22 的产品 hook 选择不扩大为本包执行权限。

### RDF-10 · OAuth rotation owner 与 caller 等待分离

- **Pi 事实：** [`refreshStoredOAuthCredential`](../../../.repo/pi-1.3.0/packages/ai/src/auth/resolve.ts#L105)在store锁内复核/refresh/save；caller signal只影响原caller等待。独立15s AbortSignal传给refresh网络调用，不包store锁或modify/save ACK；持久化完成才释放锁。不能把注释中的rotation理解为总15s。
- **Rust 目标：** caller可停止等锁/等结果；已开始rotation由store/source owner持有并在成功前ACK，不丢唯一新token或恢复取消generation。保留runtime活性、crash/remote/store未确认效果与no rollback限制。mutation_barrier同步捕获同store跨Models cutoff/按key未确认receipts，等待不发auth/logout且可取消；无默认wait30s/keys256/ADC60s或ownership-to-ACK15s cap。Pi15s仅网络refresh，其他source按自身timeout；host正常关闭须保持Tokio并核barrier报告。
- **理由与分类：** 完整保存rotation是安全/所有权合同，不是固定事务总时长。10s网络refresh＋6s ACK不因超过15s被新SDK拒绝；网络refresh真正越过来源15s则失败并保留known/pending。wait deadline仅在caller明确选择时生效，不能据timeout声称底层I/O强杀/远端回滚。
- **外部 caller 影响：** 取消的调用及时得到 Aborted，但相同来源后台事务可能仍完成凭据更新；它不产生新的模型请求。后续 logout/login/replace 与该更新串行，不允许晚到 refresh 覆盖新身份。成功本地确认不保证远端撤销；commit 失败如实报告已知／未知效果，绝不捏造回滚。
- **机械验证：** V-12/43/55 用 fake OAuth＋store gates 覆盖等待锁取消、开始后取消/drop、服务已 rotation、提交失败/timeout、并发 refresh、logout/replace、取消 generation 后零 dispatch、owner 退出及 runtime 关闭限制；不能以 sleep 结束或旧 token 仍在证明正确。
- **历史 D 依赖：** D-04/09/23/32/35 的同源与生命周期理由保留；旧 OA-05/D-32/AIR-051 的“commit 前 caller 取消丢弃结果”已被 Pi 1.0.3 事实取代，不作本轮批准。

### RDF-11 · 对象安全的 store 原子更新

- **Pi 事实：** [`CredentialStore.modify`](../../../.repo/pi-1.3.0/packages/ai/src/auth/types.ts#L47)是 serialized read-modify-write，delete 与 modify 串行；[内存 store](../../../.repo/pi-1.3.0/packages/ai/src/auth/credential-store.ts#L4)按 provider 排队，Promise chain 在活动工作 settle 前不释放。Pi 允许 best-effort store；这不等于真实 durable ack。
- **Rust 目标：** Arc<dyn CredentialStore>保持对象安全，用boxed owned callback/Future实现同键serialized modify/ACK；login/logout/replace共享边界，取消等待不能提前释放仍会晚写的mutation。mutation_barrier捕获active及provider/source-generation/resource-key/revision未确认效果，另一key成功不能覆盖；撤回默认key-count准入限制。只回收confirmed/unreferenced记录，同key reconcile才清unknown；不引通用shutdown trait/新crate/实施账本。
- **理由与分类：** boxing 是 Rust trait-object 表达差异；ack 强度和事务所有权是安全合同。SDK 内存 adapter 与产品注入的 persistent adapter 是两个实际目标适配器，使用同一 conformance；SDK 不依赖具体 keychain，不为它再造通用事务框架、crate 或独立账本。存储不支持 durable ack 时只能明示其能力，不能冒充已经持久化。
- **外部 caller 影响：** 自定义 store 需迁移到原子更新语义；只有外部 read→refresh→write 不能保证 token rotation。产品自行提供 native store、跨进程锁及持久化确认；SDK 内存默认仍可独立使用，不要求 keychain 或 session。
- **机械验证：** V-12/42/43/55 编译并运行 dyn store consumer；两 adapter 共用并发 refresh/login/delete/replace、current 二次检查、ack/failure、取消等待及 active mutation 未释放用例。一次 fake callback 通过不证明 persistent adapter。
- **历史 D 依赖：** D-21/30/32/35 的注入与 ack；当前 [`credential.rs`](../../../crates/opi-ai/src/credential.rs#L1)已有对象安全异步 store，但 unconditional write 不等于已实现目标 modify。旧 D 不授权改变所有存储 API 或新增 list 等无真实需要的表面。

### RDF-12 · OAuth expiry、输入验证和来源失败关闭

- **Pi 事实：** [Anthropic](../../../.repo/pi-1.3.0/packages/ai/src/auth/oauth/anthropic.ts#L136)把 expiry 预减 5min，而[统一 resolver](../../../.repo/pi-1.3.0/packages/ai/src/auth/resolve.ts#L176)另做 5min 检查；不同 flow 对 token 必需字段校验强弱不同。stored credential 优先，失败不静默 fallback ambient。Anthropic 53692 是[首选端口再尝试空闲 loopback／手贴](../../../.repo/pi-1.3.0/packages/ai/src/auth/oauth/anthropic.ts#L140)；[`LoginOptions.agentName`](../../../.repo/pi-1.3.0/packages/ai/src/auth/types.ts#L202)是公开输入。
- **Rust 目标：** 保存服务真实 expiry，仅统一施加一次 5min 安全窗口；按 flow 校验非空 token、finite 有效 expiry、state/PKCE/account/endpoint 来源，拒绝部分新凭据写入。保留每个 flow 的真实 login 方法、agentName 对应输入、callback fallback 和 metadata；Codex legacy 的 `opi` originator 是明示客户端身份，不伪称 Pi。已识别 device denied/expired 为 terminal，不把它们当 pending。来源失败、错误 credential kind 或刷新失败不换账号／ambient。
- **理由与分类：** 双扣窗口、运行时字段检查和失败分类是明确安全差异，不能冠以 Rust 必然；callback 首选端口与 fallback 的变化则必须对齐新上游，不能继续保留旧“固定唯一端口”说法。
- **代理能力对应：** Pi [adapter](../../../.repo/pi-1.3.0/packages/ai/src/api/bedrock-converse-stream.ts)与[loopback fixture](../../../.repo/pi-1.3.0/packages/ai/test/bedrock-response-headers.test.ts#L47)公开无真实 AWS 凭据代理和 HTTP/1.1。Rust 显式 `ProxyPlaceholderSigning`＋caller 非标准 endpoint＋HTTP/1.1，保留固定非秘密 placeholder SigV4；不称无签名 NoAuth，不探测或混入 ambient/store/profile/process/metadata/bearer 凭据，不虚构 AWS 账号身份，普通 AWS endpoint 拒绝该模式。这是 typed authority 的安全差异，能力不再延后；V-09/49/58 验证普通 SDK/真实 adapter/本地代理、H1、签名与真实 credential canary 不泄露。
- **外部 caller 影响：** 较短有效期 token 不会被双扣窗口提前刷新；畸形响应更早失败，错误不泄露 body。login 只在选定 store 确认后成功，logout 仅本地删除；没有自动交互登录、远端 revoke 或隐式 identity 探测。
- **机械验证：** V-12/43/55 fake HTTP/clock/presenter 覆盖 expires 边界、字段缺失/NaN、state、首选端口占用与手贴、agentName、denied/expired、来源优先与 canary；九类 OAuth 覆盖另见 RDF-20，不能用三条旧 flow 代替。
- **历史 D 依赖：** D-09/21/32 与 OA/BP 提供协议细节；旧 Q33 只确认三条 flow，不批准排除本轮其余六类，也不证明真实远端客户端注册可用。

### RDF-13 · 实例可 await cleanup 与 Drop

- **Pi 事实：** [`cleanupSessionResources`](../../../.repo/pi-1.3.0/packages/ai/src/session-resources.ts#L1)是 module-global 同步 registry，收集同步 errors；它不 await 本地 producer 退出。Codex cached、temporary、connecting/busy-overflow 路径不应仅凭 helper 名称推定都已覆盖，见 [SDK UX-15](sdk-consumer-contract.md#ux-1115继续复核的管理与资源边界)。
- **Rust 目标：** Models实例owned/awaitable cleanup保持原scope与cutoff，释放cached/temporary/connecting/busy-overflow本地资源，失败仍尝试其他并报告known/pending；只释放自身lease、不碰外部共享client/credential/catalog。撤回2s close默认deadline；host可显式给shutdown deadline，但不能冒称peer ACK/远端退款。None明确All含sessionless；空session ID不能被隐式扩大为All，具体空ID typed处理为保留的安全差异，不能称Pi parity。store barrier与连接namespace独立。
- **理由与分类：** 所有权与可 await cleanup 是资源保证的有意增强。Drop 只能发关闭信号，不能声称已 await；后台任务不得充当公共 owner 无限延长自身。drop Models/remove/replace 不取消已经 owned call；最后 public owner/call guard 释放才触发相应关闭。
- **外部 caller 影响：** 需要确定本地静止的宿主必须 await cleanup；成功 result 可保留 idle cache。cleanup 不删除 credential/catalog，不取消外部 caller 的父 token、其他实例或截止点后的调用，也不证明远端撤销／退款。
- **机械验证：** V-04/29/42/58 以 gates 覆盖同 session 双实例、sessionless/空 ID、prepare/connect/busy/fallback、晚 publication、重入、cleanup Future drop 后再次 await、外部 shared lease、失败继续清理与最后 owner 退出。
- **历史 D 依赖：** D-04/23/35/36；这是已选资源正确性方向的具体落实，不引入产品 SessionManager 或全局 cleanup trait。

### RDF-14 · 显式 tracing context，不复制 telemetry DSL/status

- **Pi 事实：** AI [`ProviderRequestOptions.telemetryContext`](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)显式传播 parent；[`pi-telemetry`](../../../.repo/pi-1.3.0/packages/telemetry/src/index.ts#L14)另外提供 callback-managed `startSpan`、schema inference 与 `SpanStatus`。当前 AI 传播 context 不等于自动创建 SDK call/attempt spans/exporter，见[telemetry 补核](pi-1.2.0-followup.md#1-context-与-telemetry借鉴责任划分避免混成一个大-context)。
- **Rust 目标：** `opi-ai` 小模块提供 owned `TelemetryContext::from_span/noop/clone/in_scope/instrument`，捕获匹配 parent 与来源 Dispatch；Future/Stream poll、producer task、drop 同来源传播，不持 enter guard 跨 await、不用 poll 时偶然 ambient。所有目标 SDK operation/入口保留 frozen context；不序列化或发送到 wire。宿主拥有领域名字／typed fields／status/settlement 与 subscriber/export/sampling/flush；无默认 SDK spans/exporter/global subscriber。
- **理由与分类：** native tracing 的固定 callsite／typed vocabulary 不等同 Pi 通用动态 schema DSL、callback settlement 或 `SpanStatus`；这是跨语言机制与包范围差异，须明示**不承诺该 utility 源码／API 等价**。AI parent 传播目的仍必须完整实现；不为 telemetry 包目录对称新增 crate。
- **外部 caller 影响：** caller 可显式传 `Span::current()` 捕获，但省略时为 noop。`Result`、Length/Deferred/UnknownEffect 如何记录 status 由理解领域的 owner 决定，Span clone 的存在不证明语义仍活跃／已结束。可信 backend 的 recording/export failure 不修改业务；不承诺隔离任意 subscriber panic/block/abort/OOM。
- **机械验证：** V-46–48/58 覆盖 A 下捕获 B 下 poll、跨 task、所有 chat/classifier/image/deferred/direct/advanced 路径、retry/降级、noop、同步 scope 恢复、drop、Serde/wire 排除与敏感 canary；不用上游 identity 单测冒充端到端 tracing。
- **历史 D 依赖：** D-37／Q37 实际选择该职责划分；不授权自动 SDK schema/exporter，也不把 closed 未合并提案当路线图义务。

### RDF-15 · faux 的可取消调度

- **Pi 事实：** [`fauxProvider`](../../../.repo/pi-1.3.0/packages/ai/src/providers/faux.ts#L450)公开 scripted response、调用计数和 deferred 状态；[`scheduleChunk`](../../../.repo/pi-1.3.0/packages/ai/src/providers/faux.ts#L348)使用 microtask／timeout，`streamWithDeltas`在等完之后检查 signal，等待本身不被取消。
- **Rust 目标：** 提供同用途的确定性 faux／scripted provider 能力，包括 partial/error、节流、计数、deferred pending/fetch/cancel；复用统一 Result、64 背压与事件合同。timer、脚本 gate、背压等待可被取消/drop 唤醒，取消后零后续 chunk/dispatch；不模拟 JS microtask 时间表，不借 MockProvider 的名称自动宣称 faux 全能力。
- **理由与分类：** 可取消等待是资源修正，调度精确时序是 runtime 差异；scripted caller 仍可完成相同行为。fixture 可控制 seed/chunk/clock，但不扩成通用模拟框架或对真实远端的证明。
- **外部 caller 影响：** 取消通常更及时；不能依赖 setTimeout 完整走完。faux 的计数、deferred handle 和结果仍可用于独立 SDK consumer，不能只存在库内单测私有 mock。
- **机械验证：** V-37/38/49 让取消发生在长 timer、满队列、script gate 和 deferred fetch；核对计数、partial/terminal、重开 fetch 不 resubmit 和本地任务退出。差分保留结果及事件语义，不要求 wall-clock／microtask 顺序相同。
- **历史 D 依赖：** D-04/23/30；旧 offline faux fixture 不证明真实 remote deferred adapter。

### RDF-16 · 目录 knownness 与 publication 一致性修正

- **Pi 事实：** [OpenRouter price ingestion](../../../.repo/pi-1.3.0/packages/ai/src/providers/openrouter-api.ts#L35)将负 placeholder 基价变成零并解析 tiers；不同 provider 的动态目录 authority 不同。[Radius](../../../.repo/pi-1.3.0/packages/ai/src/providers/radius.ts#L26)动态列表替代 baseline，空也权威；[`personalizeOpenRouterCatalog`](../../../.repo/pi-1.3.0/packages/ai/src/providers/openrouter.ts#L16)是显式 helper，chat-only、使用有效 base URL，并非 builtin factory 默认联网。
- **Rust 目标：** placeholder／未知价格保留 unknown，不当有效负费用或已知免费；tiers、effective-thinking sampling、metadata 和 simple `/3.5` 等真实已知值仍对齐；Pi Messages simple/default max/cache 透传例外不套通用clamp/Short。OpenRouter image成本直接用基价不取tiers；Responses/Codex serviceTier依response/request及各自default/fast规则调整，Azure无倍率，精确行为见SDK附件与V-49/50/54，不以通用共享算法抹掉实际差异。按 provider 保留 overlay/replacement/empty authority 语义，SDK 提供显式 personalization 与 configured endpoint。source invalidation tombstone 为权威，防止无内置 generation 时旧 cache 又恢复；失败／取消／陈旧 generation 不覆盖最后有效 publication。
- **理由与分类：** unknown-price 与 tombstone 是明确正确性差异；不能推广成“所有失败撤下 catalog”或“所有动态源替代 baseline”。有效 metadata 新增属于须对齐的能力，当前 Rust 已有 [ModelPricing/PricingTier](../../../crates/opi-ai/src/model_info.rs#L404)，不重复宣称从零新增。
- **外部 caller 影响：** caller 可见未知价格，不能用零推断免费；显式 SDK refresh/personality 的存在不自动开启产品启动联网。按 key/账号及 endpoint 隔离，保留 refresh report 的部分成功／失败，不吞 store/source failure。
- **机械验证：** V-27/28/54/56 覆盖负 placeholder/zero/unknown/tiers 阈值、Radius empty、OpenRouter chat-only和401/403/404保留、endpoint override、restore/invalidation/publication竞态与账号隔离；不得通过统一目录策略抹平 provider 区别。
- **历史 D 依赖：** D-10/21/23/28/34/35；旧 catalog generation 局限已明示为差异，产品自动刷新政策不是 SDK 完成条件。

### RDF-17 · 唯一 retry owner 与 strict 降级变体

- **Pi 事实：** [`retryProviderRequest`](../../../.repo/pi-1.3.0/packages/ai/src/utils/provider-retry.ts)与 adapter 自身路径／锁定 AWS SDK 共同决定真实重试；公共 options 的注释不能当请求次数。Anthropic [`prefer strict` 降级](../../../.repo/pi-1.3.0/packages/ai/src/api/anthropic-messages.ts)对特定 pre-stream 400 至多一次去掉 prefer、保留 require、重建 payload 并再次调用 onPayload。SDK 另公开 [`retryAssistantCall`](../../../.repo/pi-1.3.0/packages/ai/src/utils/retry.ts)，不是 Models 默认高层恢复。
- **Rust 目标：** 一个 logical call 一个 acquisition-retry owner；显式 max_retries=N 为每语义变体额外最多 N 次，0 关闭 transport retry。strict fallback 是单独有界语义 transition，至多两变体，最多 `2*(1+N)` 个 HTTP acquisition sends；N=0 仍允许一次满足精确 predicate 的降级。相同变体 retry 复用 payload，第二变体重建且再调用 transform；冻结 auth/model/account/telemetry，保留 require、诊断及取消。advanced start_attempt 不重置计数／quota／降级状态；handoff 后不重放。显式 whole-call helper 可用同一 typed contracts，调用方选择启用，不要求 session。 reqwest显式retry(never)即使Cargo feature合并开启H2/H3也保持单owner；当前独立H1不能被误报已隐式重试。AWS owned auth/source sends与generation分账，不运行vendor runtime/默认resolver或其隐藏日志；无I/O codec调用路径升级需重验。
- **理由与分类：** 修复 Bedrock 忽略公共 override、Codex 省略 auto 与显式 auto 的分歧、以及统一 owner，属于行为修正；不能称 Pi 所有路径本来一致。AWS按选定source实现standard/adaptive/Smithy2026及typed overrides，scoped mode/attempt/shared-config读取与auth发现分开，构造不采ambient。源已支持模式不以Unsupported收面；未知mode/坏配置按源typed失败，不静默采用standard。不能以“payload只变一次”抹去 1.2.0 的 strict callback 行为。
- **外部 caller 影响：** callbacks 可能对两个语义变体各执行一次，但不会为同一 transport retry 重跑。只有显式 whole-call helper 才可能新建 logical calls；产品的恢复次数／compaction 是另一层。既有 caller 需看见适用默认、次数和取消边界。
- **机械验证：** V-05/34/36/57 分别计 variant、send、callback、auth、quota；覆盖 N=0/1/2、特定／非特定400、require 保留、取消／deadline、handoff 后 failure、advanced入口、Busy/transient/Bedrock stalled分类和 helper终态/partial。不能只测最终成功或以 nested SDK 重试充数。
- **历史 D 依赖：** D-09/24/27/36 支持分层与冻结；旧 transform“一次”只适用于同一 payload 变体，旧 D 不批准忽略新降级或上游显式 helper。

### RDF-18 · Cargo/native 平台与 npm/Bun/browser/CLI 边界

- **Pi 事实：** [`package.json`](../../../.repo/pi-1.3.0/packages/ai/package.json)公开 root/subpaths、`pi-ai` CLI 和 bun-oauth；[root export](../../../.repo/pi-1.3.0/packages/ai/src/index.ts)强调 core side-effect-free，provider/catalog 分开；[Bun bridge](../../../.repo/pi-1.3.0/packages/ai/src/bun-oauth.ts#L12)为静态打包加载 OAuth。[AuthContext](../../../.repo/pi-1.3.0/packages/ai/src/auth/context.ts#L19)存在 browser fallback，Node-only flow 仍需对应平台能力。
- **Rust 目标：** Cargo/module/resources/host Tokio表达独立SDK，npm/Bun/browser/CLI符号和任意executor/WASM不承诺ABI。确定受控Hyper动态head/trailer存储、ownedUpgrade＋受控Ratchet PMD/长度/checked存储、实例rustls/commonring＋私有OpenSSL EVP signer/verification/KX，以及full-source AWS/ADC owner；不以ring范围/标准only/noPMD收面。native默认trust选固定Node参考bundle，scoped extra/explicit roots按source消费，host roots显式选择；不宣称Bun/OS/webpki同默认。原生C、有限unsafe及fork/数据/registry包维护成本明示；产品MSRV/锁图/三平台/consumer仍OPEN。保留真实executable env gate/host授权、portable well-known与无自动gcloud差异，无虚构预算/零retry/双独立gate。
- **理由与分类：** packaging/runtime 桥属于平台差异；不是删除 OAuth/helper/catalog/独立 SDK 动作的理由。当前 [manifest](../../../crates/opi-ai/Cargo.toml)与[CI](../../../.github/workflows/ci.yml#L40)支持 native 依赖与 Linux/macOS/Windows 验证路线；本候选**不承诺 WASM/browser 或任意 executor 支持**，也不从源码无显式 wasm forbid 推断可用。
- **外部 caller 影响：** Rust caller 自行装配 runtime、Stream/Serde 和 UI；无 Agent/产品 crate 前提。SDK 文档需列 native 支持面、平台相关 OAuth presenter/callback限制和正常依赖。未证明编译大小、lazy link、启动或性能收益，不为 tree-shaking 猜测加 feature。
- **机械验证：** V-26/42/49/58 用离线 package 与 workspace 外 consumer、native 多平台组装／假传输、资源定位、无环境读取构造、公开工具能力映射。不能以 monorepo 内编译证明可发布，也不能用未缓存依赖的自动下载补缺。
- **历史 D 依赖：** D-21/30/35/37 与父规范 GOAL-003；不复制 Bun/npm 形状不等于免除 SDK 功能。

### RDF-19 · SDK 与 host-only 集成责任

- **Pi 事实：** AI 的 Context/transcript/validation、AuthContext/store 与 TelemetryContext 是三种责任，见[并列补核](pi-1.2.0-followup.md#1-context-与-telemetry借鉴责任划分避免混成一个大-context)。coding-agent 的 session、工具执行、启动目录刷新和高层恢复不是裸 Models 的必备前提。
- **Rust 目标：** 仅 `opi-ai` 的 57 项 active SDK 义务由本包承担；AIR-009/015–019/037/039/041 共 **9 项 host-only** 保留集成影响及原责任方，不作为本包等价完成门槛。SDK 提供 chronology、投影／校验、deferred、显式 retry、refresh 等机制；Agent/session/产品仍拥有授权、hook、session v3／迁移／compaction、tool_search 装配、启动政策及产品恢复。因本包公开破坏性变更确实影响仓库调用方时，必要适配仍需同步。
- **理由与分类：** 这是用户“仅 opi-ai”的范围边界，不是对全项目功能的删除或对原有安全责任的豁免；也不能把父规范／实施准入当过时文件覆盖。
- **外部 caller 影响：** 普通 SDK 不需 durable ACK、User Policy、Agent authorizer、产品配置或 Promotion；显式持久化宿主才承担自己的事务。库不执行工具、不自动保存 Context、不自动开高层恢复／目录后台刷新／交互登录。
- **机械验证：** V-20/39/49/58 的外部 SDK consumer、依赖图和 public operation inventory；同一动作不经产品私有入口即可完成。host-only 保留回归不因本附件而删除，但其未来产品验收不冒充本包支持证明。
- **历史 D 依赖：** D-18–29 包含较广产品目标，D-30/35 已有独立 SDK 理由；本轮明确包范围优先，历史 D 不重新授权 host-only 交付。

### RDF-20 · 全公开能力覆盖仍须实施证明

- **Pi 事实：** [`KnownApi/KnownClassifierApi`](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)列 **10 个 chat API、4 个 classifier API、1 个 image API**；[内置工厂](../../../.repo/pi-1.3.0/packages/ai/src/providers/all.ts)为 42 个；[OAuth loader](../../../.repo/pi-1.3.0/packages/ai/src/auth/oauth/load.ts#L16)有 **9 类 flow**。多个 factory 复用 wire；不因此免核身份、auth、endpoint、默认值和 availability。Mistral 历史 API 名 `mistral-conversations`当前[实际走 chat/completions](../../../.repo/pi-1.3.0/packages/ai/src/api/mistral-conversations.ts)，不能据名字实现错误 endpoint。
- **Rust 目标：** 全部已发布能力进入唯一 public-surface inventory，逐项给出公开 Rust 动作、binding／wire、正常与失败语义、差异和验收。包括四 classifier 的 images/modality／结果／usage／retry，九 OAuth 的 login/refresh/metadata，动态 headers/raw observer、duration、metadata/sampling/tiers、catalog personalization、whole-call helper、faux/deferred、custom adapters 和 package 可达性。remote deferred 仅在有真实协议与 fixture 时声明支持；faux 成功不推导远端能力。 R-01–08的设计处置与source-level精确后端选择已写入候选；这只关闭文档设计空白，不关闭任何未实施的SDK能力或V。正式image/classifier placement及全部运行/编译/平台证据仍缺，不靠后端名/下载hash替代。
- **理由与分类：** 这是**真正能力及证明缺口**，不能用 RDF-01–19 关闭。当前 [`lib.rs`](../../../crates/opi-ai/src/lib.rs#L1)的导出与实现尚未交付上述完整目标；旧稿只选 System One、三 OAuth、部分 chat、延后 callbacks 的范围已不足；frames已在1.3.0退役，不恢复旧义务。native Rust 可行性不等于当前功能已存在。
- **外部 caller 影响：** 支持表必须区分 registered、可执行、已实现 adapter、验证层次与 host integration；缺生成 model data 时不能宣称全模型 ID/catalog 数据逐字节等价。不存在的独立 image wire／image streaming 或仅实验计划也不能凭想象加为 Pi 1.3.0 硬要求。
- **机械验证：** V-49–59 与既有 V 的公共面清单逐项闭合；严格离线 fixtures／fake transport 后才进行所需的 consumer/package/native 证明。运行记录固定输入、case/skip/缺失和归一化；live 另获授权。`未实现`、`未执行`、`无协议证据`分别保留，不能一律填“Rust差异已接受”。
- **历史 D 依赖：** D-01/11/23/25/32/33 是历史范围与理由，D-30/31/37 是可保留的 SDK 基础；没有任何旧 D 单项批准本轮所有排除，也没有旧测试执行结果证明 1.2.0 全包等价。

## 差分与核准边界

每个差分 fixture 应标明 RDF（如适用）、主 spec AIR、唯一 V、Pi snapshot/输入摘要、Rust state 和观察结果。可以归一化无关 clock/request ID 与双射工具 ID；不能归一化掉 alias 时点、backpressure、错误分类、partial、knownness、protected identity、callback 次数、取消／重试、publication 或资源完成差异。整数、计时和 runtime 平台差异须按本页列项显式断言。既定差异若使调用者失去本轮要求的公开能力，应重新列为缺口，不能仅增加一段理由就称已等价。

机械验证为未来义务，本轮仅维护文档。测试影响：**none**；没有修改测试或 CI。后续测试处置仍按 acceptance matrix 的 add/update/delete/retain：保留真实 wire/安全/资源回归，更新被明确新契约取代的断言，删除只锁定已正式退出壳的重复断言，新增全公开面／差分／SDK/package/native 证明。历史 D/Q、70 测试及 probe 的结论保留原版本范围，不重写为本次运行。

## Pi 1.3.0 对既有 RDF 的补充

RDF-06/09：arguments_error来自最终文本解析失败，必须保留显式owned原文且在schema coercion前拒绝；默认错误/日志脱敏是Rust差异。该前置修复与TypeBox/jsonschema dialect/format/ref/regex接受集是两个问题，不能据此关闭Schema gap。空白输入、可修复非法escape与partial展示依Pi实际路径，不强行套所有adapter。RDF-01/08的typed字段和callbacks也不能丢新字段。

RDF-01/08/20（typed字段／projection覆盖）与AIR-010/038：不同provider/API/model的所有thinking现在drop，绝不再降级可见文本；同身份签名空块保留。Kimi历史A不能因当前factory改C而洗掉；request-local tool result图像移位和effort marker保留来源/次序，canonical仍不改。实际编号职责以各RDF正文为准，不新增泛型projection框架。

RDF-14（telemetry）：Pi保持options引用，不冻结跨await的parent字段；Rust创建时冻结Span/Dispatch仍是有意owned差异。Noop/parent传播不等于SDK已自动发span，更不提供generic callback DSL同型兼容。旧telemetry README领域schema/import示例没有在1.3.0恢复，不能变成本包义务。

所有backend缺口仍为G；[当前后端附件](pi-1.3.0-rust-backends.md)列出可关闭证据，没有任何一项因上游小版本、typed Unsupported或可覆盖默认而关闭。

## 后端专项核查的差异边界

本轮[专项核查](pi-1.3.0-backend-gap-audit.md)以a37ed70为输入，细化现有RDF而不新增编号。七组仍OPEN，不能把未证明的补救变成已接受的Rust差异：

- RDF-09：前置源码轮的metaschema／grapheme／format／ref／regex差异未由leaf hook关闭。后续实测后，当前设计明确改选source-defined JSON IR、Unicode16 regex与URL数据profile；不是原标准engine已等价。Kind symbol/factory-only边界与root修正仍保留，完整实现V仍OPEN。
- RDF-13/18：WS累计read检查、124-header槽、Connection token-list/extension验证与无压缩分别核。前置源码轮未固定Pi真实global runtime；后续已固定Node24.13.0／Bun1.3.5，实测两者headers调用和PMD可达。显式websocket也会eligible fallback，最终SSE成功不关闭WS；实测from_partially_read不能补PMD，当前无压缩codec须补救／重选。完整provided-stream／自有Upgrade／patch方案仍未交付。
- RDF-06/08/17/18：按source区分proxy/redirect/retry、H2、trust、RS256与mTLS。撤回JWT typ/kid默认；ring signing与verification范围不可混用，也不把RSA签名限制推成所有非RSA mTLS不支持。真实source不支持的key不凭想象增加义务。
- RDF-10/11/12/18：full-chain、standard/adaptive/2026、ADC五家族和metadata timeout分别待证。executable默认保留源侧stdout+stderr合并解析；scoped frozen child env和kill/reap后代是明确安全/生命周期增强，非Node保证。无新的总时限、进程stdio默认cap或第二deny gate。
- RDF-14：owned pinned Future/Stream可在matching Dispatch的短scope内poll/drop，producer也须接线；safe机制草案不等于实际tracing conformance，host语义结束与Span clone寿命分离。
- RDF-18/20：有效Cargo feature union、native/transitive/MSRV、外部package consumer、三平台和hydrated catalog仍须真实证据。source档案不能关闭这些交付面。

每个未来case保留Pi原始观察、Rust公开动作结果、允许差异和对应V；未知参考runtime、实际backend拒绝和未运行三者分开，不能统一归一化为已等价。

## 最小离线实验后的差异判定

[本轮实际结果](pi-1.3.0-schema-ws-offline-validation.md)替代上述源码轮中已被实验覆盖的未知；旧报告／hash仍固定旧输入。RDF身份不增，七组仍OPEN。

- RDF-09：metaschema-before-hook、grapheme双向差异、formats／multipleOf／metadata refs、regex not／if／patternProperties假成功现已复现。raw fixture／JS parse后输入／Pi normalized分层；native消费Pi normalized不是Rustnormalizer。self-ref在固定Node抛RangeError，Rust不复制栈溢出；root-correctness仍按既有原schema最终验证修正。jsonschema仅研究比较候选，需重开选型或先证明受控适配。
- RDF-13/18：固定Node／Bun均实际可用headers overload与PMD；TW没有对应压缩能力，完整组合须补救／重选。125header的稳定差分取Bunexit0与nativeTooManyHeaders，不取Node交付后进程崩溃。参考Connection token-list本轮也拒绝，不能说native此行为已证明不对齐。Bun非法Accept／extension宽松、Node subprotocol异常不是应复制合同；native自身extension校验缺口仍须fail-closed。
- RDF-18/20：独立研究图与Windows编译不是产品图／外部consumer／三平台证明；WS仅构造器调用形状，不是Pi完整provider。已废弃且隔离的Bun默认自动解析诊断不计离线证据，正式Bun使用--no-install。研究执行为add，产品测试／代码为none，没有关闭覆盖缺口。

## 设计就绪与运行覆盖的独立状态

[本轮设计决定](pi-1.3.0-design-readiness.md)取代旧轮“尚待重新选型”的状态；Schema、HTTP、TLS、Auth、telemetry与交付输入已有确定机制和维护合同。WS独立复审的长度／溢出／声明分配问题由最新纯内存探针及受控patch另证，旧80观察不能自动覆盖它。没有新增RDF身份，也没有将真实能力缺口改名为语言差异。

RDF-18的依赖分发必须对普通外部consumer有效：受控源码／数据包采用独立registry身份与精确source/version/checksum；library lock、rootpatch或TEMP path不是发布方案。URL的10包隔离已实测，其他最终产物与发布、全部native/unsafe安全审查仍由既有V验收。原API/Context/telemetry责任不扩成新公共框架。

RDF-09除原root验证修正外明确UTF8表示边界；RDF-18的EC key拒绝错误RS256标签是算法正确性选择，不能用其拒绝源支持的RSA。原生库保留chain/host/time、显式签名与协商约束，无bypass或全局provider/config禁用。所有七组产品coverage保持OPEN，研究实测通过不等于V通过。
