# 公共工具与数据契约

这些工具之所以公开，是因为它们定义了跨 Provider 或 consumer 边界传递的值。精确
签名和 trait bounds 以 rustdoc 为准。

## Tool 参数归一化与校验

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

`provider_headers::merge_provider_header_sources` 做大小写不敏感且有序的 set/remove 合并。
后来源胜出并移至末尾；`None` 删除 key。Provider 管理的认证 header 不能被任意 request
header 静默替换。

Provider environment 通过注入的 `AuthContext` 读取。SDK 在构造模型时不要求 ambient
process read；scoped source 不能修改其他 Provider 的 environment policy。

## Identity、Overflow 与类型化诊断

公共 identity 工具提供稳定 short hash 和 UUIDv7，并校验 timestamp/sequence 边界。overflow
和 length predicate 是纯函数，返回类型化诊断，不调用 compaction、retry 或网络。

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
callback failure。

`RequestCallbacks` 观察 payload 和 response 边界。payload callback 在 projection 和校验
后运行；response callback 在 source 定义的 response boundary 运行。callback 失败是类型化
功能失败，会脱敏且不可重试。callback 顺序确定，不会安装隐藏的全局 transport policy。
