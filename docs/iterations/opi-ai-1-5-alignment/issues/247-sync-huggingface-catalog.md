# #247 同步 Hugging Face 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票只处理 provider=huggingface 的 SDK builtin 目录和现有 OpenAI
Completions 路由。固定 Pi 1.5 发布 artifact SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
模型集合以该 artifact 为准，Pi 源码用于核对 Hugging Face provider 和请求参数。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Hugging Face OpenAI Completions 聊天模型 | 78 → 78 | 3 | 3 | 4 |

新增记录为 MiniMaxAI/MiniMax-M3.1-Flash、Qwen/Qwen3-8B 和
deepseek-ai/DeepSeek-V4.1-Flash-1017。
撤下记录为 Qwen/Qwen3-Next-80B-A3B-Thinking、XiaomiMiMo/MiMo-V2-Flash
和 zai-org/GLM-4.5。
更新记录覆盖 deepseek-ai/DeepSeek-R1-0528、
Qwen/Qwen3-235B-A22B-Thinking-2507、Qwen/Qwen3-Coder-480B-A35B-Instruct
和 Qwen/Qwen3-Next-80B-A3B-Instruct 的 maxTokens。

Hugging Face 复用现有 OpenAI Completions 适配器和 Bearer 认证，不新增
provider 专属 Rust 工厂。opi-coding-agent 没有 Hugging Face 内置入口，
因此本票不扩展 CLI 范围。本票没有新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle、manifest 和固定 Pi 1.5 Hugging Face
   记录逐条相等。Catalog::builtin、注册后的 Models 列表、可用列表和按 ID
   查询均只暴露 78 个固定模型，3 个新增 ID 均可查询。
2. **实际请求参数一致。** 通过回环服务验证 Bearer
   POST /v1/chat/completions。deepseek-ai/DeepSeek-R1-0528 请求发送
   更新后的 32768 输出上限，OpenAI Completions wire 使用
   max_completion_tokens，推理请求发送 reasoning_effort=high。
3. **入口集合封闭。** 3 个下线 ID 在 builtin 目录、列表、查询和运行派发前
   均不可见。未知 ID 返回 UnknownModel，回环服务收到零请求。
   build_catalog check、目录差分和归档 catalog consumer 通过。

## 实施结果

- Hugging Face 作者源 SHA-256：
  44353d7c84278d6921d10c1c817b172e04b221f10423884a5e03e3dc826dc085
- 全库 bundle SHA-256：
  2fcfd38f04f6d73403a8662a7abec5f089d96adb3eb96f77569a8cd78c9f11a5
- manifest SHA-256：
  e7e04c384ada9043640ac65cbff917c20d8bd779abb4d666c6bf6166e83577c0

固定 Pi 行为参考文件的 SHA-256：

- .pi/pi-1.5.0/packages/ai/catalog/providers/huggingface.ts：
  fb8b413cd7f45d3f4b1e2033edcb9515eff2159d9d1d91f57c0ce07201cc8791
- .pi/pi-1.5.0/packages/ai/src/providers/huggingface.ts：
  51c4af194c9989ab033c5b3303831ec9e2df999f51d9fa40ab6d28f632e8ff43
- .pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts：
  e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a

## 测试回执

- cargo test -p opi-ai --offline --locked --test provider_collection --features builtins huggingface：3 项通过。
- 受影响回归 union：320 项通过，包含 provider collection 143、catalog artifact 14、catalog operations 27、OpenAI Chat fixtures 116、external SDK 11 和 build_catalog example 9。
- build_catalog check、Clippy、目录差分和归档 catalog consumer：通过。

Test impact — add Hugging Face 目录、OpenAI Completions 请求和入口集合回归；
update builtin catalog bundle；retain OpenAI Chat、catalog 和归档消费者回归；
none coding-agent 入口。
