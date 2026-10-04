# pi-ai 独立 SDK 调用方审查

> 非规范性研究与候选条款建议。审查日期：2026-10-04。仅静态阅读本地 pi 1.0.0 快照及当前 44 AIR 英文候选 spec；没有运行 pi 测试、模型请求、安装或生成脚本。本文件不批准新目标，不授权运行时实施。

本轮采用 D-18 的 pi Rust 实现定位，从“只依赖 SDK 的开发者能否完成一次真实工作”反查目标，而非再次盘点 adapter。使用了 `research`、`codebase-design` 和 `writing-for-agents` 的一手证据、公开接口测试及完成条件方法。本文为新增中文研究笔记，没有既有英文 counterpart；正式候选 spec 的双语同步由主文档负责。

主要结论：现有候选已经覆盖模型绑定、鉴权、流终止、transcript、工具执行安全和多数高级机制；但独立 SDK 的**完整聊天结果、公开事件重建、手工工具循环、通用/原生选项入口**仍没有闭合为可执行验收。新增图片/分类的非流式方法不能替代最基本的聊天 `complete` 使用路径。Rust 不需要复制四个 TypeScript 方法名、共享可变 `partial` 或 Promise，但需要提供等价且不会被有界队列阻塞的使用方式。

## 范围及证据使用

下文源路径相对 `.repo/pi-1.0.0/packages/ai/`；每个引用给出行号和拥有行为的符号。被审查目标为 [spec-draft.md](spec-draft.md)，内容哈希在本次开始时为 `2bf33e9be643b0226ac9709d1690cea06a5f2950e42843d545ef03ae133d7679`。已有 [TP](transcript-target-proposal.md)、[R](replay-contracts.md) 和 [SP](sdk-target-proposal.md) 用于定位已讨论决策，不代替此处源码。

本包没有查到独立 `examples/` 目录；可核验的独立使用材料是 README 代码示例、`models-entry.test.ts` 的包导出调用、`models-runtime.test.ts` 的实例调用，以及离线 faux/事件/帧测试。README 示例不是已运行证明。`stream.test.ts`、`cross-provider-handoff.test.ts` 是带真实鉴权/模型调用的测试路径，本轮未执行；前者和不少 faux 测试仍从 `compat` 导入，不能当作新 Models 全路径证明。本次没有扩展调查第三方外部应用，因此不声称发现独立 SDK 的外部生产消费者。

## 开发者的实际工作路径

```text
createModels（空实例）或显式 builtinModels（内置 providers）
  → 注册/选择 provider 与 chat model
  → Context：messages + 可选初始 system/tools
  → stream 或 complete；通用选项或 API 原生选项
  → owning provider auth → normalize 后 transcript → adapter
  → 可选消费 block 事件 + 获取最终 AssistantMessage
  → 调用方保存 assistant；显式验证/授权/执行工具
  → 调用方追加 toolResult；再次调用或切换模型
```

### SC-01：实例、模型选择与默认操作

**事实。** `src/index.ts:4–8,33–48` 明确以无全局 registry 副作用的核心为入口，导出 Models、类型、faux、transcript、validation 和 frames。`createModels` 默认持有内存 credential/model store（`src/models.ts:362–397,985–987`），创建时不注册 provider；`builtinModels` 才显式枚举并注册全部内置 provider（`src/providers/all.ts:183–189`）。这不是“默认联网发现所有模型”。

模型读取有三种：无操作限定的 `getModels/getModel/getAvailable` 面向 chat，`*OfType` 面向指定操作，`getAll*` 面向全部操作（`src/models.ts:235–290`）。Provider 内 model ID 在操作类型内唯一；chat 可省略 `type`，同 provider+ID 的 image/classifier 是另一个操作记录（`src/models.ts:160–174,998–1001`；`README.md:294–316`）。Models 按 `model.provider` 选择 provider，`createProvider` 再按 `model.api` 选择实现（`src/models.ts:824–835,1063–1075`）。

**使用含义。** SDK 用户不需 coding-agent 配置文件；可以只注册一个 provider，也可显式选择全内置集合。列出模型、已配置鉴权、实际可分发是不同结果。现有 AIR-001/002/025/034 与此方向一致，但未用独立 chat caller 的完整构造/调用测试证明替代 AIR-021 删除的外壳仍可用。

**证据边界。** `test/models-entry.test.ts:30–57` 从 `@earendil-works/pi-ai/models` 与 faux 子路径组装实例、断言空目录、执行 complete，并拦截 TypeBox/SDK/catalog 重加载（禁止项在 `:9–27`）；这是离线测试设计，未运行。Rust 可以用模块、builder、feature 和类型化枚举表达相同选择，不需要 npm 子路径、动态 import 或可省略的 JSON discriminator。

### SC-02：配置、auth 与自定义 provider 是 SDK 接口

**事实。** `createProvider` 接受身份、auth、模型、chat API/图片/分类实现，至少有一个实际实现（`src/models.ts:989–1046`）。它既接受单一 chat 实现，也接受按 API 的实现映射。`README.md:1211–1273` 用无需真实 key 的 Ollama 自定义 provider 和混合 API gateway 展示这一接口；不是只允许修改产品中的内置模型。

`applyAuth` 在 owning provider 上解析一次本次调用的 auth，然后应用请求覆盖；模型的 auth-derived endpoint 进入实际请求模型（`src/models.ts:837–868`）。`transformHeaders` 在合并完成后调用一次，并从 provider options 删除（`:858–866`）。`checkAuth/getAvailable` 与会刷新凭据的 `getAuth` 职责不同（`:276–302`）。默认 credential store 在内存，持久化由应用注入（`README.md:428–440`）。

**证据。** `test/models-runtime.test.ts:1090–1149` 使用假 provider 捕获最终选项/endpoint，检查字段覆盖和单次 header transform；`:1152–1172` 检查未知 provider 通过错误流返回、成功实例流为 start/done。这些是源码中已有断言，未执行。

**目标判断。** AIR-001/021/023/042 已确定身份冻结与更严格的受保护鉴权约束。pi 的显式 header 覆盖不能反向推翻这些已确认选择。需要在公开接口验收中证实 SDK 可注册自定义模型/adapter/auth 并通过同一 preparation；无需复制 `createProvider` 的 TypeScript 签名或新增平行工厂层。

### SC-03：chat 的 stream 与 complete 是同一语义的两种消费方式

**事实。** `Models.stream` 先 `normalizeContext`，返回包住异步 auth/provider setup 的流；`complete` 直接返回该流的 `result()`（`src/models.ts:871–909`）。Simple 版本同理。`complete` 不是另一个供应商非流式 wire API，不是 Agent loop，也不会自动把 assistant 加回 Context。

`EventStream.result()` 是独立保存的最终 Promise。终止事件入队时就解析它，事件消费者可以随后才消费队列（`src/utils/event-stream.ts:30–49,72–103`）。`test/event-stream.test.ts:6–23` 先等 result，再遍历，断言结果和事件顺序；`models-entry.test.ts:45–47` 只调用 complete。普通失败/取消由终止 assistant 承载，最终结果保留部分内容、usage、身份和失败信息。

**重要 Rust 差异。** pi 的队列无容量限制；“不消费 events，只等 result”不会被队列背压卡住，但这不证明该内存设计适合 Rust。候选 AIR-005 已选择容量 64。Rust 的 final-only 方法必须主动驱动/排空底层流或使用不会依赖未消费队列的内部结果路径；不能简单添加一个被同一有界事件队列挡住的等待 future。可以使用消费所有权的 `collect/complete`、已消费流保存 terminal outcome 等结构。精确方法数量和 Promise 可重复等待语义属于语言表示选择。

**当前缺口。** AIR-003–007 定义终止和释放，却没有要求普通 SDK 调用者在不写 Agent/事件收集器时获得完整 chat outcome，也没有要求事件终态与 final-only 结果一致；AIR-026 只涉及图片/分类。

### SC-04：公开事件不仅是字符串 delta

**事实。** `src/types.ts:751–783` 的协议是：成功 start→updates→done，开始后失败 start→updates→error，setup 失败可直接 error；done/updates 不得在 start 前。不同内容块可交错，必须用 `contentIndex` 关联。`toolcall_end` 是完成的提议，不代表已按执行 schema 验证（`README.md:703–724`）。

Text/thinking 的 end 是权威完成内容；签名、redaction、工具 ID/name/arguments/namespace 可能只在完成块中确定。`partial` 是 JS 共享实时对象，排队的旧事件可以看到更晚的内容；不是事件时快照。源类型也明确 redacted thinking 可以在 start 已完整且没有 delta（`src/types.ts:760–765`）。

**目标判断。** AIR-003–006 和 AIR-038 覆盖部分终态/opaque，但仍缺公开 block 生命周期、交错、end 覆盖及无 delta 的有效内容契约。它们决定普通 SDK 用户能否渲染/收集正确答案，应补入目标验收。Rust 可以用 owned 不可变事件、增量加完成块或明确借用视图，无需复制共享可变 partial；不得为了形式统一丢掉 end-only 内容、签名或 block 顺序。

### SC-05：错误是一种带历史信息的结果，不只有抛出异常

**事实。** `AssistantMessage` 同时携带有序内容、provider/API/请求模型、可选报告模型/response ID、usage、stopReason 和 errorMessage（`src/types.ts:546–570`）。`AssistantMessageEventStream` 将 done.message 或 error.error 作为同一个最终结果（`src/utils/event-stream.ts:91–103`）。`lazyStream` 把 setup rejection 转为无 start 的 error message（`src/api/lazy.ts:4–22,41–61`）。直接调用 API 的 `streamSimple` 则允许在缺 auth 时同步抛出（`src/types.ts:360–370`）。

**目标判断。** Rust 可以把错误改为 `Result` 或封闭 outcome，区分 setup/transport/provider/cancel；但错误通道必须仍能交付已知部分 assistant/usage/来源，且与最终流事件一致。当前 AIR-003 的 typed Err 与 provider error payload 区分正确，尚需说明 final-only 调用怎么获得这些信息。`Result<Assistant, Error>` 若 Error 只剩字符串，会破坏 pi 提供的可恢复历史行为。

**不应机械复制。** pi 的 setup error 把 usage 填零（`src/api/lazy.ts:11–18`），以及 `end()` 没有结果可能留下未解析的 result Promise（`event-stream.ts:60–69,86–88`），是当前实现限制；AIR-003 的异常 EOF 和 AIR-024 的 unknown/zero 已选择更可靠语义。README 的 stop reason 列表漏列 deferred（`:1080–1087`），源码联合包含 deferred（`types.ts:450,778–783`）。

### SC-06：只依赖 SDK 时工具循环由调用方持有

**事实。** SDK `Tool` 只有 name、description、parameters、constrainedSampling，没有 execute（`src/types.ts:715–720`）。SDK 返回 ToolCall；调用方保存 assistant，选择实现，验证/执行，追加带 toolCallId/toolName/isError 的 ToolResult，然后再调用 SDK。结果可含文本与图片，details/nestedCalls 不等于 wire 内容（`types.ts:594–607`；完整手工继续示例 `README.md:187–217`）。

校验是公开可独立调用的 `validateToolCall/validateToolArguments`，由根入口导出（`src/index.ts:48`；`src/utils/validation.ts:302–349`）。它返回校验/转换后的参数，不自动调用工具。`README.md:667–699` 明确要求手工循环调用该 helper。

**目标判断。** 已确认 plain-JSON 规范化必须能由 `opi-ai` 单独复用，供嵌入式调用方和 `opi-agent` 使用；不能只藏在 Agent executor。SDK 的收到/显示 toolcall 不等于授权执行；原始模型提议与 normalized/execution 的区分仍成立。嵌入方负责自己的执行策略；选择 `opi-agent` 才进入现有 User Policy/Capability Permission runtime，不应强制普通 SDK 请求构造产品 session/registry。

**反证及边界。** README 小示例直接在 `toolcall_end` 执行工具，未等待整个 assistant 的 stopReason；Quick Start 也没有完整异常处理。这是教学简化，不能推翻已确认 Length/Error/Aborted 零执行。完整 Agent 目标已有 AIR-013/041。SDK manual-loop 验收应采用“先完成 outcome，再验证/授权/执行”，而不是把示例中的抢先执行升格为目标。

### SC-07：可序列化 Context 与 SDK 层的自动 replay

**事实。** `Context` 是 messages 加初始 system/tools 简写；normalize 返回只含 messages 的 TranscriptContext（`src/types.ts:726–748`；`src/utils/transcript.ts:10–34`）。当前 system/tools 的重建 helper 从 transcript 读取，SDK 调用方可插入后续 system 变更（`README.md:1559–1590`）。Context 和 Model 都是可序列化数据（`:1592–1623`），不用先引入产品 SessionManager。

Replay 转换发生于 adapter 输入，独立 SDK 调用者也能在同一历史上切换模型：按 provider+API+requested model 判断签名/opaque 是否可复用；跨模型可读 thinking 转普通文本，redacted 不透明内容丢弃，tool ID 依目标 wire 调整，缺失结果合成，Error/Aborted assistant 不送回 wire（`src/api/transform-messages.ts:92–145,158–219`）。已确认 AIR-010–013 的更精确事实/失败规则应应用于 SDK 路径，不限产品恢复。

**文档偏差。** README `:1523–1526` 声称同 provider/API 保留、跨 provider thinking 带 `<thinking>`，比源码条件宽且包装描述不符；源码还比较 requested model，转换为普通 text。README `:1151–1173` 允许把 aborted assistant 保存后继续，不表示其 partial 内容一定被下一请求送回模型。SDK 示例/验收应同时检查原始保存与 wire 省略，避免误承诺“继续被中断的文本”。

**目标判断。** 现有 AIR-008/010–013/020 和第 62 行已覆盖大部分行为；需要补一个只依赖 SDK 的序列化→恢复→换 adapter 场景，把不依赖产品 session v3 作为完成条件。Rust serde 格式无须字节兼容 TS JSON；消息顺序、实际来源、opaque bytes 和 known/unknown 语义须保留。

### SC-08：通用与原生请求选项是有意分层

**事实。** `streamSimple/completeSimple` 接受统一 reasoning、thinkingBudgets、toolChoice、deferred；`stream/complete` 接受已选 API 的完整 options（`src/types.ts:274–292,350–357`；`README.md:986–1053`）。通用 ToolChoice **只有 auto/none**（`types.ts:84`）；不能因为一个 cast 测试传 required 就宣布中立类型支持 required。原生 Azure 测试分别检查 `stream(...toolChoice:required)` 和 `streamSimple(...none)`，且保留工具声明（`test/azure-openai-tool-choice.test.ts:20–79`）。

公共共享选项还有 temperature/maxTokens、samplingParams、cacheRetention/sessionId、timeout/retry、fetch/env/headers、metadata、payload/response/provider-stream 观察（`src/types.ts:131–237`）。`samplingParams` 是特定 OpenAI-compatible adapter 的透传并覆盖命名参数，不是全 adapter 统一保证；`fetch` 不控制 WS；不支持的 provider-event observer 不被调用。这些能力不可仅靠“公共 JSON 配置”或一个 payload hook 代替明确契约。

**目标判断。** 候选已规定 reasoning、strict/grammar、部分 transport/retry 和 payload/response hook，但没有显式规定普通 chat `toolChoice`、通用/原生 option 路径及默认省略语义。至少需要列出保留的中立控制项、真实 adapter 原生能力，以及 unsupported/ignored/rejected 的规则。Rust 可以用一个请求 builder 加类型化 API 扩展表达，不必复制四方法或开放 `Record<string,unknown>`。是否提供 header transform/provider-event observer 的独立便捷接口应作为明确覆盖选择，不能默认宣称 AIR-042 已覆盖全部。

### SC-09：frames 是已导出的 SDK 能力，但不是最终结算

**事实。** root 导出 `AssistantMessageFrameEncoder/reduceAssistantMessageFrames`（`src/index.ts:38`）。Frame 包含 block start/delta/end、工具 JSON checkpoint 和 authoritative metadata，故意排除 terminal settlement（`src/utils/assistant-message-frame.ts:4–33,144–161`）。Reducer 单次遍历、不改输入，end 可替换已积累内容与签名（`:368–409`）。`README.md:726–749` 给出持久化 frames 与单独保存 finalMessage 的用法。

这是当前发布公开接口，不是仅实验内部 helper。`test/assistant-message-frame.test.ts:147–228,230–358,489–545` 分别覆盖 end-only 内容、已提前增长的 live partial、交错及拷贝；`:545–578` 明确 terminal 不成 frame、无 start 返回 undefined。测试未运行。

**目标判断。** 现有候选未纳入公开 frame codec，也未明确延后。它不是必须逐类复制的结构：若 Rust owned 事件已经具备可持久化的紧凑语义和纯 reducer，可直接满足同一使用价值；JS 的 queued-live-partial 去重只在 Rust 也暴露该语义时需要。若本轮只做最终消息 persistence，则应明确把 SDK 进度帧能力列为延后及复评触发，不能把 AIR-019 的产品持久化当作已覆盖。

### SC-10：compat 与独立测试的迁移地位

**事实。** `src/compat.ts:1–10` 声明临时维持旧全局 API，并把退出挂到 coding-agent ModelManager 迁移；模块全局 registry 在 `:102`，导入时注册在 `:215–217`。`README.md:1820–1845` 要求新代码改用 Models/provider，并说明平行 ImagesModels 已退出。

`test/stream.test.ts:7,21–27` 导入 compat 并在模块级解析 OAuth；`:267–340` 是最多五轮的人工工具循环。`test/faux-provider.test.ts:2–11,30–46` 同样主要验证旧 compat 路径；`cross-provider-handoff.test.ts:352–373` 受凭据控制且真实生成输入。这些测试存在不意味着独立新 Models path 已全部验证，更不授权本轮执行。

**目标判断。** opi 不需新建 pi 式全局 compat。应保留语义等价的实例 API 和 SDK 直接调用测试，把当前 AIR-021 外壳退出与 replacement 实际可调用联系起来。Rust 的单实例所有权、静态 feature 和独立测试入口是合理差异。

## 建议补入候选的闭合条款

以下是研究者建议，尚未新增为已确认 AIR；下方 Q32 补充记录人类随后明确选择的 Rust 错误表面。优先级表示对当前 SDK 使用契约的影响，不是实施工单或进度。

| 建议 ID / 优先级 | 责任方、触发、目标行为和失败 | 现有覆盖及必须明确的缺口 | 可机械验证的验收 |
|---|---|---|---|
| SC-G01 / P1 | SDK chat owner；调用方要完整结果。提供 final-only 调用及 stream 后取最终结果的路径，使用同一绑定/adapter/终态归约；失败携带已知 partial/usage/identity。无终止时按 AIR-003 返回协议失败。 | AIR-003–007 没有 chat complete/result 契约；不能只新增图片/分类方法。Rust 可使用 consuming collect，无须可克隆 Promise。 | 只依赖 opi-ai 的 fixture 分别跑 final-only 与流消费；超过 64 个事件仍完成；比较内容/usage/终止/错误/opaque。setup error、部分失败、取消、异常 EOF 全部终结，不死锁，不二次请求。 |
| SC-G02 / P1 | SDK event protocol/adapter；流到达调用者。规定 start、每块开始/增量/结束、允许交错、块身份和权威完成内容；setup 可无 start 失败；partial 所有权明确。 | AIR-003–006/038 只部分约束；没有完整的可重建公开协议。 | 两个真实 adapter 的伪 transport 产生交错 text/thinking/工具、end-only 内容、空签名 thinking、partial JSON；公共 consumer/reducer 得到正确顺序和最终 metadata；非法 lifecycle 明确报错。 |
| SC-G03 / P1 | SDK validation + SDK caller/Agent；收到完整工具提议。SDK 公开声明与提议、独立可复用 plain-JSON normalize/validate；执行由调用方拥有，Agent 复用同一验证并执行已确认权限流程。 | AIR-024/041 未明确只依赖 SDK 的执行前 helper。不能把自动工具执行或产品 session 作为取得模型结果的条件。 | 独立 caller 两轮：保存 assistant→规范化/验证→假工具→追加 result→完成；未知工具、无效参数、Length/Error/Aborted 零执行；JSON 往返保留关联，运行依赖不含 opi-agent/coding-agent。 |
| SC-G04 / P1 | SDK options/adapter；调用者设置或省略中立/原生选项。明确通用 toolChoice auto/none、reasoning 及基础采样/输出控制的公开路径；原生项只传往匹配 API，默认省略遵守已固定 adapter 语义。 | AIR-022/040/042 不能代替所有 chat options。无需四个命名入口；需要对每个现有字段记录 retain/map/reject/defer，避免 agent自行丢弃。 | 假 transport 捕获 omitted/auto/none，证明 none 没有删除声明或变成权限；选择两种真实 API 验证 neutral 与 native 映射及错误路由；只走公开 SDK。 |
| SC-G05 / P2 | SDK composition/replay；注册 provider/模型、恢复历史、换模型。公开组装一个内置或自定义 provider、注入 auth/transport，并使用序列化 Context 完成恢复；无需产品配置/SessionManager。 | AIR-001/020/021/025 和第 62 行已有原则，缺完整 replacement 使用场景。 | 最小外部 crate/compile fixture 只依赖 opi-ai；空实例、显式注册、chat 默认读取、错操作类型、同源/跨模型 replay；Fake auth/HTTP，全程零自动工具执行/文件/网络。 |
| SC-G06 / P2 | SDK frames/observer；调用者需要持久进度或 provider 诊断。明确已导出的 pi 进度帧、header transform 和 provider-event observer 本轮如何映射或延后。 | 当前 spec 既未覆盖也未列为 gap。Frame 不是 final settlement；payload hook 不是 header/event observer。 | 若纳入：纯 reducer 重建进度，terminal 单独结算；只读 observer 顺序、异常传播/脱敏；header 最后合并但不越过已确认身份保护。若延后：范围/触发明确，不声称完整覆盖。 |

SC-G01–05 的可观测目标可从 pi 当前公开接口查明；无需向用户重新询问“是否允许 SDK 自己调用 chat”。公开方法命名、owned handle/collect 的 Rust 表示和 private accumulator 是实现设计选择，但应在候选中选定一种责任明确的方式，不能把双重驱动和背压问题留给实施者猜测。SC-G06 涉及本轮覆盖取舍，需要由主 Agent 根据已确认定位作显式范围判断；本文没有替人确认。

## Q32 补充：已确认的 Rust 结果表面及退出条件

在本轮调查中，人类通过主 Agent 的 Q32 明确选择 Rust `Result`，失败保留完整 assistant message。它是**已确认的 Rust 公共表面差异**，不是声称 pi 返回相同结构：pi 的 `complete/result` 返回同时容纳 error/aborted 的 `AssistantMessage`；Rust 目标为 `Result<AssistantMessage, CallFailure>`，其中 `CallFailure` 同时拥有完整 `message` 和类型化脱敏 cause。

据该选择收敛的公开流目标为 `Item = Result<AssistantStreamEvent, CallFailure>`：Stop/Length/ToolUse/Deferred 对应正常完成结果，Error/Aborted 对应 Err；失败不再同时出现于 `Ok(AssistantStreamEvent::Error)`。Length 为 Ok 只表示模型正常返回了截断结果，仍不能执行其工具；Deferred 为 Ok 仍是等待 handle，不变成业务完成。取消和异常 EOF 也走这一失败表面，保留已经收到的 partial、已知 usage 和来源。

`ChatStream.result(self)` 主动驱动剩余流并返回与 `complete` 一致的 Result，是主 Agent 当前拟采用的所有权设计。此方案比给有界事件队列外挂一个被动 result future 更适合 Rust：调用者可以先逐项读取，再消费所有权取得终态；不能同时由两个所有者驱动同一个流。精确公开 Models 名称及四方法收敛由主文档记录，本文不把尚未记录的命名方案另行批准。

### 现状与迁移依赖

此补充额外只读检查了 opi 的结果类型及直接消费者；不扩展为运行时审计，也没有修改它们。

| 现有责任与真实依赖者 | 处置原因与替代 | 保留/改变及范围 | 退出证明与测试影响 |
|---|---|---|---|
| `crates/opi-ai/src/provider_collection.rs:218–233,718–735`：`drain_to_completion -> Result<CompletedRequest, ProviderError>`，provider 错误为 `Ok(CompletedRequest::Error)`、transport 为外层 Err；`src/lib.rs:55` 导出 | 用统一 `Result<AssistantMessage, CallFailure>` 替换双层错误。完整 message 与 typed cause 各有唯一职责 | 保留 final-only drain、部分结果、usage、route；改变公开返回类型/匹配模式。不是再新增一个重复 complete 并永久保留旧结果入口 | **update** `tests/provider_collection.rs:332–335,424–444,578–581` 的结果断言；成功、provider error、transport error、EOF 各仅一种 terminal disposition。活跃公开 export/调用者中旧 CompletedRequest 归零后退出 |
| `crates/opi-ai/src/stream.rs:325–397,420–421`：事件同时有 Error variant、外层 ProviderError；多个 adapter/test_support 产出前者 | 公开流失败只用 `Err(CallFailure)`，不再两个分支让消费者判断 | 保留原 provider error assistant 与脱敏 cause，传输失败也带累积上下文。影响 adapter、MockProvider、fixture、外部 SDK pattern match；不擅改低层 provider wire 格式 | **update** 各 adapter fixture 和 `tests/stream_events.rs`，**retain** 原错误内容/usage 语义。公开 normal path 不再产出/匹配旧 Error variant，首次 Err 后永久 EOF |
| `crates/opi-agent/src/agent_loop.rs:1615–1631`：摘要调用分别处理 stream Err 和 Error message；`src/event.rs:487–501`：事件脱敏及反序列化；`crates/opi-coding-agent/tests/failed_run_usage.rs:17–26`：失败 message 用量 | 同步所有消费者，使 provider/transport/cancel 失败都读取 failure.message 和 cause，再走其各自 policy | 保留 failed-run usage、部分历史、来源和秘密保护。改变 Agent event/protocol 所引用的 SDK 类型时，必须盘点 schema、fixture、JSON 输出及历史 reader；不能只靠代码编译宣称迁移完成 | **add/update** provider 错误与中途 transport 错误的相同失败用量/历史可见性、redaction、summary 失败检查。旧持久化 `type:error` 是否存在及其导入需求需先核对；必要的窄历史 reader 可保留，不能据 Q32 安全删除历史事实 |

### 不丢信息与不重复驱动的验收

1. final-only `complete`、从未读取事件的 `result(self)`、读取前半流后的 `result(self)`、已读取 terminal 后的 `result(self)`，在同一人工 fixture 上返回相同 Result（归一化无关调用标识后）；每种使用方式仅一个底层模型请求。已消费 Err 后缓存返回同一失败事实，不是创建第二次失败或重试。
2. 至少 65 个内容事件加终态，final-only 和部分读取后收集都完成；保留 AIR-005 的容量 64，不以无限队列解锁。取消/EOF/错误路径同样能结束，清理证明仍独立于结果可用。
3. setup 失败、部分文本后 HTTP/SSE/协议失败、provider Error、取消分别得到一个 typed cause 与完整 message；后续 poll 恒为 EOF。没有已生成内容的 setup failure 使用明确的空内容及 known/unknown accounting，不为填满 message 复制 pi 的人工零用量。
4. `failure.message` 保留已知内容顺序、opaque/签名、请求/报告模型、usage 和 StopReason::Error/Aborted；公共成功返回不得包含 Error/Aborted。cause 与 message 的终止分类相容，序列化/恢复/脱敏不丢上述字段。
5. 单纯取得 Result 不自动执行工具或追加会话。独立 SDK 与 Agent consumer 都验证 Length 零执行、Deferred handle 仍 pending、失败部分内容留存但 replay 按 AIR-013 投影。
6. 在公开 SDK seam 验证一次失败归约，不只给旧 provider Error 和 transport Err 各套一层名字。旧路径的静态清除范围应排除明确保留的历史 reader 与标注为历史的研究文档；这些范围须在迁移盘点中列清。

这些是 Q32 的必要验收细化；历史 schema/持久化 reader 的完整依赖扫描尚未在本轮完成，不能据本文件立即删除旧 serde variant。未发现需要为了 `Result` 再新增一个 error registry、配置开关或永久兼容执行器的理由。

## 保留的有意差异与不能据此复制的行为

| 差异 | 判断与依据 |
|---|---|
| Rust owned event / final outcome / serde 类型代替 JS live partial / Promise / TypeBox | 合理语言差异；前提是 SC-03–07 的部分内容、顺序、最终 metadata、错误和恢复语义完整。 |
| 有界队列及 drop/取消清理 | 保留 AIR-004/005；pi 的 result 不消费事件依赖无界队列，不能作为绕开 Rust 容量的理由。 |
| 正确分类 unknown/zero 与异常 EOF | 保留 AIR-003/024；不复制 setup error 人工零用量或 result 永不完成。 |
| 明确注册/feature 与实例所有权代替兼容全局 API | 保留；compat 是迁移机制，有导入副作用，不是新 SDK 默认设计。 |
| model-schema 验证与 trusted hook/最终授权分层 | 已确认 D-22；裸 SDK 仍只提供数据/验证能力，不强制采用完整 Agent runtime。 |
| 最终 assistant 才决定能否执行工具 | 保留 AIR-013/041；README toolcall_end 教学示例缺完整终止检查，不构成抢先执行承诺。 |
| 更严格的 protected-auth/body binding | 已确认目标，不能用 pi header/samplingParams 的宽松覆盖默认撤销。需记录为有意差异，不误称逐字段 pi 兼容。 |
| 宽松接受 null/missing content | pi 有兼容历史非类型化输入的专门测试：`test/lax-message-content.test.ts:1–6,30–58`。Rust typed construction 可不产生该状态；反序列化旧 JSON 是否恢复为空是导入策略，不能靠“Rust 类型安全”自动推断。 |

## 检查状态与交接

本轮实际执行：`rg --files`/`rg -n` 查找，PowerShell/Python UTF-8 只读定位源码和候选 spec，写入本文件后检查 LF/尾随空白。所有测试分析均为静态；SC-Gxx 的验收是未来设计。未使用真实凭据或 live transport，未修改源码、既有测试或共享候选文件。

测试影响建议：**add** 独立 SDK final-only/stream/manual-tool-loop/serializable replay 的公开接口 fixture；**update** 新结果/事件/选项类型改变所影响的消费者测试；**retain** 已有 adapter wire、取消/EOF/权限/opaque 测试；本轮没有理由 **delete** SDK 行为测试。实际运行时测试影响：**none**。

后续主文档整合时先关闭 SC-G01–04，再用 SC-G05 的独立 caller 验证组合后的 interface。SC-G06 明确映射或延后即可；不能把仅有类型、方法或文档示例计为完成。上游快照后的 SDK 变化不在本轮只读复核范围，未据此提出新的确定规划。
