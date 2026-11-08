# Provider Wire 与操作参考

本文说明完成注册和认证选择后的当前 Provider 行为。工厂、catalog、凭据和 OAuth
规则见[Provider、Catalog、认证与刷新参考](provider-auth.zh.md)；共享 HTTP、TLS、
WebSocket 和 cleanup 行为见[Transport、协议与资源生命周期](transport.zh.md)。

## Native Chat Adapter

当 compatibility profile 无法表达 Provider 行为时，native adapter 保留 Provider
专属 option 和 wire 形状。公开注册缝是 `Models::register_chat_api`；类型化 option
携带自己的 `ApiId`，可在兼容注册之间使用。

Anthropic Messages 保留 tool 时序、cache control、thinking/signature block、inline
tool 变更以及 Provider 对错误 result 图像的位置规则。Google Generative AI 和 Vertex
保留 content part、safety 设置、thinking level、usage 是否出现以及 finish-reason 映射。
Bedrock Converse 使用选定的 AWS 或 bearer 认证路径和 event-stream 终态。Mistral、
Codex 与 pi-messages 保留自己的 request envelope 和流式终态，不强行使用通用 chat
serializer。

## Image 操作

图像输入使用带顺序的 `InputContent::Image`，并携带显式 media type 和 source metadata。
图像生成是独立操作，拥有自己的 `ImageModelRecord`、`ImagesContext`、`ImagesOutput`
和类型化结果。Provider failure 返回类型化 result failure，不会静默变成 chat content。
OpenRouter image adapter 使用共享的 timeout、retry、callback 和 cancellation 边界。

## Classification 操作

Classification 是独立操作和 catalog 类型。公共 classifier adapter 保留 Provider answer
形状，同时映射为 `ClassifierOutput`；缺失、畸形、拒答和非 finite answer 都报告为类型化
失败。Decisions、System One 和 llama.cpp route 使用不同 wire envelope，不对 score、
choice 或 boolean 语义做未文档化的共用假设。

## Reasoning 与 Native Option

simple chat 路径携带 `ThinkingLevel`；native adapter 可以通过类型化 option 暴露 Provider
专属 reasoning。Provider 区分省略、零和 false 时，省略值保持省略。streaming thinking
和 redacted-thinking block 保持顺序；route 无法保留时不会提升为可见文本。

Stop reason 归一到公共词汇（`Stop`、`Length`、`ToolUse`、`Error`、`Aborted`、`Deferred`、
`Pending`）；Provider raw reason 只通过类型化 failure 或 adapter diagnostic 暴露。

## OpenAI-Compatible Provider Profile

`CompatConfig` 是叠加在 adapter provider 推断之上的显式覆盖。它控制 instruction role、
thinking format、store 行为、extra header 及所选 profile 文档化的字段。Profile 不能新增
未支持的 wire 能力、提高 output budget 或替换 Provider 管理的认证 header。

Azure 对 Chat 与 Responses 两条 wire 保持显式 deployment/resource/api-version 映射。
OpenRouter、Mistral、Together、Qwen、xAI 等 compatibility profile 保留各自 source 字段
和省略规则。注册规则见 provider/auth 专题，代码以精确 rustdoc option 为准。

## Direct Adapter

当操作 envelope 或响应不能由通用 chat 表达时，Direct adapter 提供类型化公共 API，
包括 pi-messages、OpenAI Decisions、System One、llama.cpp classification、OpenRouter
images 以及 Provider 专属的 Vertex/Bedrock/Mistral route。每个 adapter 在首次请求前
冻结 model/base/auth identity，并使用共享 callback、timeout、cancellation 和类型化错误边界。

## Provider 专属限制

Provider catalog 是有限集合，不代表自动注册、登录或网络访问。构造阶段不做 source I/O；
认证和 catalog refresh 只在文档化的调用或 refresh 边界发生。不支持的 option、未知模型、
非法 tool/schema 组合、不可用 operation 和 Provider protocol failure 会在成功 event
发布前失败。
