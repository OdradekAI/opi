# 确定独立接入与行为对齐的公共契约
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](01-upstream-contracts.md), [核实 opi-ai 工程、测试与独立接入的现状](02-current-structure-tests.md)

## Question

在已确认的独立接入边界内，如何定义 pi-ai 能力完整、可观察行为兼容、Rust 原生接口和允许的有意差异？如何处理 JS 引用/异常/宿主专用入口与 Rust 的冲突，以及阶段交付和最终完整对齐之间的关系？

## Comments

2026-11-06 首轮建议，以下为讨论历史；已确认的决策见 Answer。

已确认的范围是独立 Rust 库及声明的库依赖，无 opi-agent/CLI/私有路径/隐式全局初始化要求。下一步必须区分：能力覆盖、可观察行为等价、Rust 公共 API 的形状。推荐保留完整能力目标，采用 Rust 原生类型和所有权表达；每个原公开入口必须有映射，所有有意行为差异显式登记，不计入严格等价。

需要用具体场景讨论：调用 stream 时还是首次 poll 时消费脚本和启动鉴权；保留 partial 后是否观察到后续修改；取消等待者后 OAuth 轮换是否继续提交。不得仅因 Rust 不便实现就从验收分母移除这些问题。若选择 Rust 原生快照/错误模型，记录输入域、可观察差异、理由和对 agent 的影响。

用户当前指定的 .pi/pi-1.5.0 是目标；验收文档中的 a57438e 不是自动覆盖该目标的权威。先迁移基准证据，再确定契约比较规则。不修改现有 API。

## Answer

2026-11-06 用户确认：

> 采用 Rust 原生接口，保持完整能力映射，可观察行为逐项验收；有意差异单独讨论并记录，不能算严格等价通过。

因此 Rust 的命名、类型和所有权表达可以自然设计；原公开能力仍须完整映射。不能以语言差异为由自动豁免可观察行为，不能把有意改进从差异记录中删除后计作严格等价。

这是对齐政策的确认，不是某项具体差异的批准。对象共享、调用/首次 poll 时点、错误通道、取消后提交等具体合同仍须在所属职责决策中处理；具体发布等级与证据集合留给“确定验收标准的分层与发布判定”。本票不授权修改生产代码。

### 公开 Models 的准备失败错误通道

2026-11-11，用户在收到两项保留建议后回复“继续推进”。据此保留普通
`Models::stream_simple` 的类型化准备失败，并采纳对应的调度决策。
[#43 重放与决策](../issues/43-match-public-models-completion.md) · [调度决策](11-telemetry-lifecycle.md#普通-models-的-resolver-进入时点)

此项覆盖 `models-missing-provider` 的缺失路由和
`models-auth-setup-failure` 的受控 resolver 失败。Pi 的公开
`Models.streamSimple` 发送一次 `error` 事件，`result()` 兑现为该事件的错误消息。
Rust 在流项和 `result()` 中交付值相等的 `Err(CallFailure)`。缺失路由的原因是
`Config`，认证失败的原因是 `AuthFailed`。两侧均在 provider 派发前失败，消息保留请求的
provider/model，流在错误后到达 EOF。状态为**有意差异**。原始诊断文本、故障标记、
终态和结果一致性仍由重放证据记录。
[原始轨迹](../evidence/phase5-public-models-lifecycle-differential.json)

Rust 调用方可根据 `CallFailure::cause()` 区分路由与认证问题，并从
`CallFailure::message()` 读取失败消息。公开 streaming 示例用 `map_err` 传播原因。
Agent 主循环和摘要走 prepared attempt，在该入口处理自己的类型化源流错误。
[#43 调用方核对](../issues/43-match-public-models-completion.md#accepted-decisions)

### 公开 deferred fetch 的本地取消错误通道

#58 中，fetch 已派发且首个终态仍未缓存时，调用方取消本地请求。Pi 兑现带
`aborted` 和 `Request was aborted` 的 assistant 消息。Opi 返回 `Err(CallFailure)`，
保留类型化原因 `Cancelled`、停止原因 `Aborted` 和当前消息内容，缺少原诊断时使用
`cancelled`。保留此错误通道和诊断为有意差异。Durable 风格的调用方先依据任务的中止
状态决定结算，再通过 `CallFailure::message()` 取得需交给分类器的消息。`Aborted`
按停止原因排除重试。显式远端取消由应用按保存的 handle 发起。
[#99 重放与调用方证据](../issues/99-decide-deferred-local-cancel-before-done.md)

### 公开 deferred fetch 的鉴权失败错误通道

#60 的 API-key resolver 在派发前报错。Pi 兑现带 `error` 的 assistant 消息，诊断包含
provider 和 resolver 原因。Opi 返回 `Err(CallFailure)`，`cause()` 保留 `AuthFailed`，
`message()` 的 provider/model 来自请求，诊断保留认证类别与安全的 resolver 原因。
将错误通道和诊断文本登记为有意差异。Durable 用两段原文均零重试并按 `model_error`
结算，同时持久化各自的原文。受控 503 原因在两种诊断下都可触发整次重试。迁移调用方
通过 `CallFailure::message()` 记录失败，并按消息和应用策略判定整次重试。
[#100 SDK 重放与 Durable 证据](../issues/100-decide-deferred-auth-setup-failure.md)

### 流事件的 partial 所有权

[#103](../issues/103-compare-stream-partial-aliases.md) 在 Faux 和 OpenAI Chat 的受控输入中
确认了事件时快照的具体差异。Opi 保留各事件独立拥有的 `partial`。调用方按收到的事件更新
当前进度，可以独立保留或修改任一历史事件，最终提交以 `Done` 或 `result()` 为准。Pi 的
OpenAI Chat 共享整条响应，修改早期事件的内容会传到终态。Faux 分别共享数组中的已有块
和部分子对象，原始轨迹分别登记。

在 Durable 的受控重放中，快照在没有新事件时保持旧用量。下一条事件或终态带来更新后的
用量。进度文本和终态结算在该场景一致。工具参数预览和块结束时点有各自的契约差异。
[#103 原始轨迹](../evidence/phase5-partial-aliases.json)

[#115](../issues/115-preserve-anthropic-usage-updates.md) 在 Anthropic 的真实适配器中补充
该所有权观察：保留的首个文本事件仍是原文本、原用量与原成本；静默用量更新由最终
结果或失败 partial 交付，不回写历史事件。Pi 的保留引用随最终响应变化，差异继续记录。

### OpenAI Chat 的交错文本与工具增量

OpenAI Chat 按到达顺序交付文本和工具增量。整次 completion 的文本增量属于同一个
内容块，续写复用首次文本的 `content_index`。`TextEnd` 携带完整块内容。
[#104](../issues/104-align-openai-chat-interleaved-text.md) 修复了工具开始时的提前闭块，
使下一轮 Chat 请求保留续写空白，Anthropic 的助手内容块及受控 Durable 的进度和历史
顺序也与 Pi 对齐。该历史重放中 Opi 在 `finish_reason` 帧关闭块，Pi 在 provider
迭代结束时关闭；[#108](../issues/108-repair-chat-terminal-boundary.md) 已修复此终结边界。
两边的未完成工具参数预览仍可分别出现 `null` 和缺字段。
[#110](../issues/110-verify-chat-argument-previews-and-retain-literals.md) 修复完整尾部字面量
丢失，并分别核对 Pi 增量解析器、旧 utility 和 Opi helper；其余预览差异仍未裁决。
原始差分继续登记。
[#104 修复前后轨迹](../evidence/phase5-chat-interleaving.json)

### OpenAI Chat 的输出上限

[#105](../issues/105-compare-chat-output-limits.md) 使用相同模型记录对照 native、simple
和直接鉴权入口。两端的 native 与 direct 在上限缺省或显式为零时省略字段，显式正值原样
传递。simple 缺省取模型输出容量，显式正值使用调用方指定的上限，再按扣除 4096 token
余量的上下文空间收缩。空间估算值最低为一，显式零值最终仍省略字段。两种字段名的请求
及终态一致。#103 的探索请求配对用了不同入口，原始轨迹继续保存。
[#105 配对轨迹](../evidence/phase5-chat-output-limits.json)

### 增量参数预览中的 Unicode 映射

2026-11-14，用户明确选择：

> 保留标准 Rust JSON 值，配对后再发布预览；明确记录这项时序差异。

`streaming_json::ToolCallArgumentsStream` 内部保留 UTF-16 增量，原始 UTF-8 文本
不改写；如果当前可见键或值含孤立代理码点，保留上一份可表示的独立 JSON 快照。
后续增量完成配对后再发布。Pi 1.5.0 会立即公开未配对的码元，这项预览时序为已批准的
有意差异，不计入严格等价。严格最终校验维持原合同，不能因为展示可用就执行非法参数。

[#111](../issues/111-map-incremental-argument-previews.md) 的配对控制还记录非有限数字、
128 层安全实体化边界及既有最终 JSON 数字重解码精度差异。它们是明确的剩余范围，
**不在上述 Unicode 决策的批准范围内**。独立 owned JSON 的累计复制成本也单独测量，
未批准数值性能预算；不能声称与 Pi 结构共享快照具有相同成本。

### 非有限数字及快照深度的完整保留

2026-11-14，用户在看到数组 `[1e400,true]` 被省略为 `[true]` 的反例后明确选择：

> 采用完整快照保留与 128 层上限。

可达的非有限数字保留内部槽位，暂缓整份独立快照发布；超过 128 层容器也保留上一份
快照。原始增量不丢弃、不改写，后续可表示的子树覆盖可恢复发布；非前缀替换从空状态
开始。既有严格最终校验保持不变，包括对较早重复成员中溢出数字的拒绝。不得靠省略
数组元素、复用被覆盖成员或转换为 `null` 来伪装对齐。

这些是已批准的表示及时序差异，**不计作严格 Pi 等价**。实现及调用方证据见
[#112](../issues/112-retain-complete-preview-at-representation-boundaries.md)。Pi 的解析器
允许非有限数字，但实际 Chord/Durable JSON 提交会拒绝它们。当前产品没有消费新预览
owner，16/100 ms 的受控采样不代表产品性能验收。既有最终数字 `1e64` 的重解码差异
仍未批准为有意差异，并会影响 Agent 参数及会话序列化；下一切片单独处理。

### 最终数字重解码

2026-11-14，用户要求修复 #112 的最终 `1e64` 精度差异，并覆盖实际工具执行和会话
保存、恢复。SDK 的普通 `serde_json` 依赖启用 `float_roundtrip`，使浮点转换正确舍入；
Cargo 会向使用同一版本的依赖图统一此特性。根 workspace 的独立依赖默认配置不变。
这不是仅作用于预览或单个函数的开关，也不是新解析器或新依赖。

最终参数原始文本继续保留；会话按既有脱敏规则转为规范 JSON。原生 `i64`/`u64` 整数
继续精确表示，不强制改为 JavaScript Number；非有限、孤立代理码点及深层最终值的
拒绝规则保留。历史会话打开不改写字节、不重执行工具；已经写入的错误数值无法可靠
反推原始模型文本，本次不自动迁移旧会话。实现与位值、调用方证据归属
[#113](../issues/113-preserve-final-numeric-decoding.md)。
