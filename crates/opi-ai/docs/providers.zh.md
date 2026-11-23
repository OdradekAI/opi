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

## Amazon Bedrock

`amazon-bedrock` 内置路由使用目录中所选的 Converse 模型 ID，支持区域推理
profile。流式请求向所选 runtime 端点发送 `POST /model/{id}/converse-stream`。
工具请求 JSON Schema 约束时采用所选模型的严格模式能力。启用严格模式后，请求
发送转换后的 schema 和 `toolSpec.strict: true`。Claude adaptive thinking 通过
`additionalModelRequestFields.output_config.effort` 发送目录声明的映射值。

独立工厂、coding-agent 与聚合内置目录共用同一份 Bedrock 模型数据。独立路由根据
端点配置和认证区域选择标准 AWS 端点。配置默认模型时，使用当前目录中的模型 ID。

在 `Models` 调用中，如果请求启用了 `Prefer` 工具的严格模式，Bedrock 在交付流
之前返回命中严格工具限额的 `ValidationException` 时，SDK 重建并发送一次。
第二份请求恢复 `Prefer`
工具的原始 schema，保留 `Require` 和普通工具的请求字段。重建时再次运行
payload 回调并复用已解析的认证。结果携带 `bedrock_strict_tools_fallback` 诊断。

## GitHub Copilot

SDK 与 coding-agent 共用内置的 Copilot 聊天目录。所选模型记录决定请求路径为
`/responses`、`/v1/messages` 或 `/chat/completions`。OAuth 提供账户的基址。
适配器按请求内容生成 `X-Initiator`、`Openai-Intent` 和 `Copilot-Vision-Request`。
存储的 OAuth 凭据声明了可用模型 ID 时，SDK 的可用性查询返回该名单与目录的交集。

## OpenAI Responses

SDK 内置入口、独立 Responses 工厂和 coding-agent 共用 OpenAI 聊天目录。
OpenAI 端点接收 `POST /v1/responses`。SDK 内置入口单独提供 OpenAI Decisions
分类操作。

所选模型决定如何发送对话中途加入的工具。GPT-5.4、GPT-5.4 Mini、GPT-5.4 Pro
和 GPT-5.5 在顶层 `tools` 中发送当前工具集，支持严格 JSON Schema 工具、grammar
工具和中途指令。`gpt-6.1-terra` 等模型支持通过内联 `additional_tools` 加入工具。

## OpenAI Codex

SDK 与 coding-agent 从同一份内置 Codex 目录选择聊天模型。专用适配器使用所选模型
的基址和 `/codex/responses` 路径。已存储的 OAuth 凭据提供 Bearer 认证和 ChatGPT
账户 ID。simple 入口按模型记录映射推理等级。`gpt-6.1-terra` 的 Off、Minimal、
Max 分别发送 `none`、`low`、`max`。

## OpenCode Zen 与 Go

OpenCode Zen（`opencode`）和 OpenCode Go（`opencode-go`）使用所选模型记录中的端点。
Chat Completions 和 Responses 共用各自 provider 的 OpenAI 基址，Messages 和 Gemini
使用各自的协议基址。聊天请求可携带 `X-OpenCode-Session`，simple 入口按模型选择受支持
的推理等级。Zen 的两个 Jev 分类模型通过 System One 端点使用 Bearer 认证。

## Baseten

内置 `baseten` 路由使用固定 Pi 1.5 目录中的 14 条聊天记录。设置
`BASETEN_API_KEY` 后使用 Bearer 认证。聊天请求向
`https://inference.baseten.co/v1` 的 `POST /v1/chat/completions` 发送；目录启用
streaming usage 时同时发送 `stream_options.include_usage`。

Baseten 模型按目录中的 `thinkingLevelMap` 和 compatibility profile 生成推理字段。
OpenAI 格式的记录发送 `reasoning_effort`；Baseten 格式的模型发送
`chat_template_args.enable_thinking` 和映射后的 effort。模型不支持的等级由 neutral
simple 入口按照共享推理等级策略收敛。内置列表遵循固定 Pi 发布目录，下线 ID 在派发前
直接拒绝。

## xAI

内置 `xai` 路由使用固定 Pi 1.5 目录中的 4 条 xAI Responses 记录，向
`/v1/responses` 发送 Bearer 认证。`grok-4.3` 和 `grok-4.5` 的 XHigh 映射为
`reasoning.effort=xhigh`。xAI 推理请求同时声明 `reasoning.encrypted_content`，
以保留 Provider 返回的加密推理内容。

## Fireworks

内置 `fireworks` 路由使用固定 Pi 1.5 目录中的 29 条 Fireworks 聊天记录。
Anthropic Messages 记录向 `/inference/v1/messages` 发送请求，OpenAI Completions
记录向 `/inference/v1/chat/completions` 发送请求。Fireworks 的 Anthropic 请求使用
Provider API key 请求头，OpenAI 兼容请求使用 Bearer 认证。

`qwen-max-latest` 使用 adaptive thinking，并将 XHigh 映射到
`output_config.effort=xhigh`。启用 session affinity 的记录会把请求 session 传入
Provider 要求的 affinity 请求头。

## TypeSafe

内置 `typesafe` 路由使用固定 Pi 1.5 目录中的唯一 `jev-latest` 分类器记录。设置
`TYPESAFE_API_KEY` 后使用 Bearer 认证。文本分类向所选记录基址的
`/v1/systemone` 发送请求，并把公共 boolean 问题编码为 `noul`。

目录声明 TypeSafe 的 PNG 和 JPEG 图片使用 `assets` map。带图片的请求在原始 state
前放置 `<image-N>` 资产名，并发送 `TypeSafe-Danger-Zone-Images: true`。纯文本请求
保留原始 state，同时省略 assets 和 opt-in 请求头。不支持的图片格式会在派发前拒绝。

## Moonshot AI CN

内置 moonshotai-cn 路由使用固定 Pi 1.5 目录中的 6 条 OpenAI Completions
记录，并从 MOONSHOT_API_KEY 取得 Bearer 认证。请求发送到
/v1/chat/completions。Kimi K3.1 Pro 和 HighSpeed 使用目录声明的 max_tokens，
并将 High 映射为 reasoning_effort=high。模型选择严格限制在发布目录的 6 条记录内。

## Moonshot AI

内置 `moonshotai` 路由使用固定 Pi 1.5 目录中的 6 条 OpenAI Completions 记录，
通过 `MOONSHOT_API_KEY` 使用 Bearer 认证，请求发送到 `/v1/chat/completions`。
Kimi K3.1 Pro 和 HighSpeed 将 High 映射为 `reasoning_effort=high`。
K3.1 Pro 使用发布目录中的 input 2、output 10、cache-read 0.2 价格和
1048576 上下文窗口。模型选择严格限制在发布目录的 6 条记录内。

## Qwen Token Plan

内置 `qwen-token-plan` 路由使用固定 Pi 1.5 目录中的 21 条 OpenAI Completions
记录，通过 `QWEN_TOKEN_PLAN_API_KEY` 使用 Bearer 认证，请求发送到
`/compatible-mode/v1/chat/completions`。发布目录中的
`deepseek-v4.1-flash-1017` 上下文窗口为 1000000。High 推理请求使用
`max_completion_tokens`、`enable_thinking=true` 和 `reasoning_effort=high`。
模型选择严格限制在发布目录的 21 条记录内。

## Qwen Token Plan CN

内置 `qwen-token-plan-cn` 路由使用固定 Pi 1.5 目录中的 21 条 OpenAI
Completions 记录，通过 `QWEN_TOKEN_PLAN_CN_API_KEY` 使用 Bearer 认证，请求发送到
`/compatible-mode/v1/chat/completions`，使用相同的发布目录推理和输出上限行为。
模型选择严格限制在发布目录的 21 条记录内。

## Kimi Coding

内置 `kimi-coding` 路由使用固定 Pi 1.5 目录中的 6 条 OpenAI Completions 记录，
通过 `KIMI_API_KEY` 使用 Bearer 认证，请求发送到
`/coding/v1/chat/completions`。Kimi K3.1 Pro 和 HighSpeed 将 High 映射为
`reasoning_effort=high`。`k3-256k` 使用发布目录中的 input 3、output 15、
cache-read 0.3 价格和 262144 上下文窗口。模型选择严格限制在发布目录的 6 条记录内。

## Z.AI Coding CN

内置 `zai-coding-cn` 路由使用固定 Pi 1.5 目录中的 4 条 OpenAI Completions
记录，通过 `ZAI_CODING_CN_API_KEY` 使用 Bearer 认证，请求发送到
`/api/coding/paas/v4/chat/completions`，并使用目录声明的 `max_tokens` 字段。
发布目录中的 `glm-4.6v` 具有 128000 上下文窗口和 0.05 cache-read 价格。
模型选择严格限制在发布目录的 4 条记录内。

## Xiaomi

内置 `xiaomi` 路由使用固定 Pi 1.5 目录中的 3 条 OpenAI Completions 记录，
通过 `XIAOMI_API_KEY` 使用 Bearer 认证，请求发送到 `/v1/chat/completions`，
使用 `max_completion_tokens`，启用推理时发送 `thinking.type=enabled`。发布目录
中的 MiMo 2.6 记录具有 1048576 上下文窗口。模型选择严格限制在发布目录的 3 条
记录内。

## Xiaomi Token Plan AMS

内置 `xiaomi-token-plan-ams` 路由使用固定 Pi 1.5 目录中的 2 条 OpenAI
Completions 记录，通过 `XIAOMI_TOKEN_PLAN_AMS_API_KEY` 使用 Bearer 认证，请求
发送到 `/v1/chat/completions`，使用 `max_completion_tokens`，启用推理时发送
`thinking.type=enabled`。目录记录使用 1048576 上下文窗口和零价格。模型选择严格
限制在发布目录的 2 条记录内。

## Xiaomi Token Plan CN

内置 `xiaomi-token-plan-cn` 路由使用固定 Pi 1.5 目录中的 2 条 OpenAI
Completions 记录，通过 `XIAOMI_TOKEN_PLAN_CN_API_KEY` 使用 Bearer 认证，请求
发送到 `/v1/chat/completions`，使用相同的输出上限和 DeepSeek thinking 投影。
模型选择严格限制在发布目录的 2 条记录内。

## Xiaomi Token Plan SGP

内置 `xiaomi-token-plan-sgp` 路由使用固定 Pi 1.5 目录中的 2 条 OpenAI
Completions 记录，通过 `XIAOMI_TOKEN_PLAN_SGP_API_KEY` 使用 Bearer 认证，请求
发送到 `/v1/chat/completions`，使用相同的输出上限和 DeepSeek thinking 投影。
模型选择严格限制在发布目录的 2 条记录内。

## MiniMax CN

内置 minimax-cn 路由使用固定 Pi 1.5 目录中的 4 条 Anthropic Messages
记录，并从 MINIMAX_CN_API_KEY 取得 API key 认证。请求发送到
/anthropic/v1/messages，使用 x-api-key 请求头。MiniMax-M3.1-Flash 使用
adaptive thinking，并将 XHigh 映射为 output_config.effort=xhigh。模型选择
严格限制在发布目录的 4 条记录内。

## MiniMax

内置 `minimax` 路由使用固定 Pi 1.5 目录中的 4 条 Anthropic Messages 记录，
通过 `MINIMAX_API_KEY` 使用 API key 认证，请求发送到 `/anthropic/v1/messages`，
使用 `x-api-key` 和 `anthropic-version=2023-06-01`。`MiniMax-M3.1-Flash` 使用
adaptive thinking，并将 XHigh 映射为 `output_config.effort=xhigh`。
`MiniMax-M3` 暴露发布目录声明的 524288 输出上限。模型选择严格限制在发布目录的
4 条记录内。

## NVIDIA

内置 `nvidia` 路由使用固定 Pi 1.5 目录中的 21 条 OpenAI Completions
记录，并从 `NVIDIA_API_KEY` 取得 Bearer 认证。请求发送到
`/v1/chat/completions`，同时携带目录声明的 `NVCF-POLL-SECONDS: 3600`
请求头。NVIDIA 记录使用 `max_tokens`，不发送 `reasoning_effort`。
DeepSeek V4.1 Flash 将 High 映射到 `thinking` 请求包体，未声明 thinking
映射的 Nemotron 记录省略该包体。模型选择严格限制在发布目录的 21 条记录内。

## Cerebras

内置 `cerebras` 路由使用固定 Pi 1.5 目录中的 2 条 OpenAI Completions 记录，
通过 `CEREBRAS_API_KEY` 使用 Bearer 认证，请求发送到 `/v1/chat/completions`。
Cerebras 使用标准 `max_completion_tokens` 输出上限字段和
`reasoning_effort` 推理映射。发布目录中的 `gpt-oss-120b` cache-read 价格为
0.35。模型选择严格限制在发布目录的 2 条记录内。

## Groq

内置 `groq` 路由使用固定 Pi 1.5 目录中的 7 条 OpenAI Completions 记录，
通过 `GROQ_API_KEY` 使用 Bearer 认证，请求发送到
`/openai/v1/chat/completions`。`openai/gpt-oss-safeguard-20b` 使用发布目录
中的 0.0375 cache-read 价格，`qwen/qwen3.8-27b` 的上下文窗口为 131072。
模型选择严格限制在发布目录的 7 条记录内。

## Cloudflare AI Gateway

内置 cloudflare-ai-gateway 路由使用固定 Pi 1.5 目录中的 59 条记录，
覆盖 Anthropic Messages、OpenAI Completions 和 OpenAI Responses。Cloudflare
从 CLOUDFLARE_API_KEY、CLOUDFLARE_ACCOUNT_ID 和 CLOUDFLARE_GATEWAY_ID
解析 API key、账户和网关，并发送 cf-aig-authorization 请求头。

Anthropic 透传请求使用 /anthropic/v1/messages，Workers AI OpenAI 兼容请求
使用 /compat/chat/completions，OpenAI Responses 请求使用 /openai/responses。
gpt-6.1-terra 将 XHigh 映射为 reasoning.effort=xhigh，DeepSeek Workers AI
记录使用 max_tokens，claude-sonnet-5-5 使用 adaptive thinking 并将 XHigh
映射为 effort。

## Radius

内置 `radius` 路由使用固定 Pi 1.5 目录中的 28 条 Radius Pi Messages 记录。
默认 gateway 使用内置基线。带 Bearer 的认证 `GET /v1/config` 会用 gateway 目录替换
基线，聊天请求使用 Bearer 向 `POST /v1/messages` 发送 Pi Messages SSE。4 个下线 ID
会在派发前直接拒绝。

## Together

内置 together 路由使用固定 Pi 1.5 目录中的 15 条 Together OpenAI Completions 记录。
Bearer 认证发送到 POST /v1/chat/completions。Qwen3.7-Max 启用 reasoning 并使用发布目录
中的 max_tokens 上限。5 个下线 ID 会在派发前直接拒绝。

## Hugging Face

内置 `huggingface` 路由使用固定 Pi 1.5 目录中的 78 条 Hugging Face OpenAI Completions
记录。Bearer 认证发送到 `/v1/chat/completions`。启用推理时，DeepSeek-R1-0528 使用发布
目录中的 `max_completion_tokens` 上限并发送 `reasoning_effort=high`。3 个下线 ID 会在
派发前直接拒绝。

## Vercel AI Gateway

SDK 内置的 `vercel-ai-gateway` 从内置目录提供 303 个聊天模型和 14 个文本分类器。
两种操作都使用 `AI_GATEWAY_API_KEY`。聊天向 `POST /v1/messages?beta=true`
发送 `x-api-key`，Messages beta 功能通过 `anthropic-beta` 请求头声明。
分类向 `POST /typesafe/v1/systemone` 发送 Bearer 认证。使用
`Models::classifier_model` 选择分类记录。

通过 simple 入口关闭支持 Off 的模型推理时，请求发送 `thinking.type=disabled`。
`openai/gpt-6.1-terra` 的 XHigh 推理预算为 16,384 token，受上下文和输出上限约束。
`moonshotai/kimi-k3.1-pro` 的目录输出上限为 131,072 token。

## Image 操作

图像输入使用带顺序的 `InputContent::Image`，并携带显式 media type 和 source metadata。
图像生成是独立操作，拥有自己的 `ImageModelRecord`、`ImagesContext`、`ImagesOutput`
和类型化结果。Provider failure 返回类型化 result failure，不会静默变成 chat content。
OpenRouter image adapter 使用共享的 timeout、retry、callback 和 cancellation 边界。

## Classification 操作

分类适配器的模块名对应 wire API。从旧版升级时，按下表修改 import。

| Wire API | `opi_ai::api` 下的旧模块 | 当前模块 |
| --- | --- | --- |
| `openai-decisions` | `decisions` | `openai_decisions` |
| `cloudflare-workers-ai-system-one` | `cloudflare` | `cloudflare_workers_ai_system_one` |
| `llama-cpp-classify` | `llama` | `llama_cpp_classify` |

Classification 是独立操作和 catalog 类型。公共 classifier adapter 保留 Provider answer
形状，同时映射为 `ClassifierOutput`；缺失、畸形、拒答和非 finite answer 都报告为类型化
失败。Decisions、System One 和 llama.cpp route 使用不同 wire envelope，不对 score、
choice 或 boolean 语义做未文档化的共用假设。

对于 TypeSafe System One，目录以 `assets` 声明 PNG 和 JPEG 图片输入。适配器在原始
state 前放置 `<image-N>` 资产名，仅在图片请求中发送
`TypeSafe-Danger-Zone-Images: true`，并在派发前拒绝不支持的图片格式。

Cloudflare Workers AI System One 使用账户作用域的 `/run` 路径和 Bearer 认证。
Clef 记录以 `images` 声明图片输入，将最多四张 PNG、JPEG 或 WebP 图片编码成
`input.images` data URL，并放入 Cloudflare 的 `{model, input}` envelope。Jev 记录
继续使用目录声明的 `assets` 格式。Cloudflare 路由不会发送 TypeSafe 图片 opt-in 头。

使用 OpenRouter System One 时，选择声明支持图片的分类模型，并通过
`ClassifierContext::images` 传入图片。适配器将图片按顺序编码为 `image_url` data URL，
放在原始 JSON state 前，一起发送到 `<base>/systemone`。配置代理时，
OpenRouter 的 provider ID 仍会选中该格式。纯文本模型会在发请求前拒绝图片。
[OpenRouter System One API](https://openrouter.ai/docs/api/api-reference/systemone/submit-a-system-one-request)

## Reasoning 与 Native Option

simple chat 路径携带 `ThinkingLevel`；native adapter 可以通过类型化 option 暴露 Provider
专属 reasoning。Provider 区分省略、零和 false 时，省略值保持省略。streaming thinking
和 redacted-thinking block 保持顺序；route 无法保留时不会提升为可见文本。

Gemini 和 Vertex 的离散推理等级模型可以禁止 Off。对这类模型关闭推理时，请求使用最低
支持档，`thinkingConfig` 只填写该等级。例如，通过 simple 入口以默认选项调用 OpenCode Go 的
`gemini-4-argon` 会发送 `thinkingConfig: {"thinkingLevel":"LOW"}`。

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
