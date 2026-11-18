# 公共工具与数据契约

这些工具之所以公开，是因为它们定义了跨 Provider 或 consumer 边界传递的值。精确
签名和 trait bounds 以 rustdoc 为准。

## Tool 参数归一化与校验

Chat Completions 路由的 partial `arguments` 保留当前收到的原始文本。增量展示时，
在一次调用内为每个工具保留一个 `streaming_json::ToolCallArgumentsStream`，将
`ToolCallDelta` 文本交给 `append`，按渲染节奏调用 `snapshot`。每次返回的
`serde_json::Value` 都是独立对象，可以保留或修改。调用结束或取消时丢弃该 owner。
工具执行仍以最终 tool call 及其校验结果为准。

`text` 返回未改写的累计输入。`replace` 接收完整前缀：扩展返回新后缀，相同文本返回
`Some("")`，非前缀替换重置状态并返回 `None`。`finish` 委托既有严格最终参数契约，
返回 `(text, error)`，不消耗 owner；预览成功不代表非法最终值可执行。Provider 的
权威最终替换仍须使用。

SDK 启用 `serde_json/float_roundtrip`，让最终 JSON 数字正确舍入，包括 `1e64` 的
整数写法。Cargo 会向使用同一依赖版本的调用方统一此特性，因此 Agent 执行与会话
保存、恢复采用同一解码。最终原始文本不变；会话存储仍应用既有脱敏及序列化规则。
原生 `i64`/`u64` JSON 整数继续精确表示，溢出仍是最终错误；这不会把所有 Rust 整数
转换成 JavaScript 的浮点表示。

缺值字段暂不显示，重复键在新值可用前保留旧值，不完整转义等待后续输入。对象键保留
首次插入位置，标准 JavaScript 数组索引键按数值顺序置于最前。非对象根显示 `{}`。
键或值中尚未配对的 UTF-16 代理码点会暂缓发布，保持上一份可表示快照，配对后继续；
原始文本保留，严格最终校验仍拒绝孤立代理码点。超过 128 层容器的快照也保留旧值，
闭合后仍如此，以限制 JSON clone/drop 的递归深度。可达的非有限数字会保留整份旧快照，
避免省略成员或改变数组位置；有问题的子树被可表示值覆盖后恢复发布，非前缀 `replace`
则从空快照开始。严格最终解析仍拒绝溢出，包括预览中已被覆盖的较早重复成员。
这些 Unicode、深度及数字边界与 Pi 的 JavaScript 值不同。

解析只扫描新增输入。快照发布遵循 Pi 1.5.0 的开放路径复制额度：每次 append 获得
`256 + 8 * UTF-16 长度`，刷新消耗开放容器的成员/元素数、每容器一单位及待定标量
一单位。空 append 增加额度；重复读取和相同 replace 不增加。大型开放容器可能暂时
返回旧快照。独立 JSON 实体化仍需复制可见树；长值每个小块都请求完整快照，累计复制
可能为平方量级。调用方应选择合适的展示频率，不能将增量解析理解为总成本线性保证。

独立的 whole-prefix `message::parse_streaming_json` 助手继续可用。它可能将缺值显示为
`null`；完整 `true`、`false`、`null` 在容器闭合前保持可见。它保留既有行为，与上述
增量 owner 是不同接口。

`schema::validate_tool_arguments` 与 `schema::validate_tool_call` 在构造请求前校验
调用方拥有的 tool 定义。已有的 `arguments_error` 在 coercion 前拒绝；调用方 raw 文本
保持不变。空文本参数归一为 `{}`；不完整或非对象的最终值保留 owned raw 和类型化
错误，不会变成 partial-prefix call。

normalizer 应用支持的 JSON schema coercion，并在授权或网络 I/O 前拒绝非法 root/schema
形状。Provider adapter 可以把已校验的值投影到 wire，但不会通过猜测修复非法 tool call。

## 规范 Replay 投影

`project_replay` 将规范 `Message` 时序转换为 route 局部的 wire 视图。它按批次位置和
source identity 校验 tool 关联，在构造请求前拒绝重复 call/result、孤儿 result 和名称
不匹配。规范历史保持不变。

wire ID 只在请求局部合法化，并保持 call/result 配对同步。支持的 route 保留图像的关联、
顺序和失败状态。`ReplayProjection::Strict` 拒绝有损转换，不会静默记录另一份历史。
Provider adapter 在请求构造开头消费该 projection。

## Context 与 Transcript 时序

`transcript::normalize_context` 将简写 `Context` 折叠为一份规范时序。system content、
具名 section、tool add/remove 按顺序应用；冗余变更被省略，拒绝 append 时此前 transcript
保持不变。

Adapter 在构造请求时解析 transcript。native route 可以保留对话中的 system message；
collapse route 投影出一个 replay header。规范 transcript 由调用方持有，projection 不会
改写它。

## Header 与 scoped environment

`api::shared::headers::merge_provider_header_sources` 做大小写不敏感且有序的 set/remove 合并。
后来源胜出并移至末尾；`None` 删除 key。Provider 管理的认证 header 不能被任意 request
header 静默替换。

Provider environment 通过注入的 `AuthContext` 读取。SDK 在构造模型时不要求 ambient
process read；scoped source 不能修改其他 Provider 的 environment policy。

## Identity、Overflow 与类型化诊断

公共 identity 工具提供稳定 short hash 和 UUIDv7，并校验 timestamp/sequence 边界。

`message::is_context_overflow` 识别上下文窗口和请求大小溢出。错误诊断包括 LiteLLM 的
`Context Window exceeded`，以及诊断开头的 HTTP 413，支持原始状态码和
`<provider> API error (413):` 前缀。限流诊断优先。无正文的 HTTP 400 只在 Cerebras 上
表示溢出。overflow 和 length 判定返回布尔值，压缩与重试由调用方决定。

诊断 summary 会脱敏并限制长度。Provider status、retry hint 和安全 excerpt 保留类型
分类，不回显 secret 或无界 response body。

## JSON Repair 与可移植 Schema 工具

公共 JSON repair helper 保留调用方 raw 文本，只修复允许的 escape 和 delimiter 情形。
repair 结果说明文本是否发生变化；不完整值不会被伪造成 tool call。

schema 工具支持文档化 JSON 方言和 source-free 校验路径。不支持的 dialect marker、非法
reference、无进展递归和资源上限都会产生类型化失败。受支持依赖 profile 的数据版本差异
不会改变 schema admission 规则。

## Text/System 渲染

Text、thinking、redacted-thinking、tool-call 和 tool-result content 在渲染与 replay 中
保留顺序和失败元数据。route 无法保留 private thinking 时，renderer 不会把它转成可见文本。

system-message helper 以稳定顺序维护具名 section 和 tool declaration。tool 重定义是显式
add/change 操作；下线名称表示为 removal，而不是隐式修改。

## Whole-Call Retry 与 Request Callback

`opi_ai::retry_assistant_call` 提供调用方拥有的有界 whole-call retry policy。它只分类
文档化的 transient failure，耗尽后保留最后错误，观察 cancellation，不重试显式 abort 或
callback failure。上下文溢出判定优先于瞬态错误诊断。

对于调用失败，`retry::classify_whole_call_retry(failure.message())` 根据规范消息的
诊断分类。ChatGPT Business 工作区用量上限和 Z.ai 错误码 `1308` 属于配额限制。可重试的
HTTP 状态码按 ASCII 字母和数字边界识别。OpenAI 兼容接口的 429 若带有已识别的配额信息，
会提供脱敏的配额诊断，以及保留重试提示的类型化限流原因。

对于 `response.failed`，OpenAI Responses 和 Azure 读取嵌套的错误码和消息。Codex 的 SSE
和 WebSocket 路径使用错误消息。识别到瞬态失败、配额限制或上下文溢出时，流错误原因、
结果消息和遥测都使用固定诊断。响应成功交接后，prepared acquisition 即封闭。显式的
整次调用重试策略可以为瞬态失败发起新调用。

对于顶层 Responses `error`，OpenAI 和 Azure 的 SDK 错误信封优先使用选中的消息，缺少
消息时取标量错误码和类型。纯 data 的扁平 `type: "error"` 同时使用错误码和消息。
Codex 分别从顶层字符串或嵌套对象中选取字段，再使用非空消息或错误码。结果和遥测用固定
诊断传递恢复判定，分类只读取这些字段。

`RequestCallbacks` 观察 payload 和 response 边界。payload callback 在 projection 和校验
后运行；response callback 在 source 定义的 response boundary 运行。callback 失败是类型化
功能失败，会脱敏且不可重试。callback 顺序确定，不会安装隐藏的全局 transport policy。
