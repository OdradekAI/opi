# #246 同步 Together 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票只处理 provider=together 的 SDK builtin 目录和现有 OpenAI
Completions 路由。固定 Pi 1.5 发布 artifact SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
模型集合以该 artifact 为准，Pi 源码用于核对 Together provider 和请求参数。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Together OpenAI Completions 聊天模型 | 17 → 15 | 3 | 5 | 4 |

新增记录为 meta-models/Muse-Spark-Open-816B-A42B、
nvidia/NVIDIA-Nemotron-3.5-Super-VL 和
reflection/Beam-1.0-501B-A23B。
撤下记录为 MiniMaxAI/MiniMax-M2.7、
Qwen/Qwen2.5-7B-Instruct-Turbo、
deepseek-ai/DeepSeek-V4-Flash-0731、
meta-llama/Llama-3.3-70B-Instruct-Turbo 和
nvidia/nemotron-3-ultra-550b-a55b。
更新记录覆盖 MiniMaxAI/MiniMax-M3、
Qwen/Qwen3.6-Plus、Qwen/Qwen3.7-Max 的 maxTokens，
以及 zai-org/GLM-5.3 的 contextWindow。
Qwen/Qwen3.7-Max 同时从非 reasoning 记录变为 reasoning 记录。

Together 复用现有 OpenAI Completions 适配器和 Bearer 认证，不新增
provider 专属 Rust 工厂。opi-coding-agent 没有 Together 内置入口，
因此本票不扩展 CLI 范围。本票没有新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle、manifest 和固定 Pi 1.5 Together
   记录逐条相等。Catalog::builtin、注册后的 Models 列表、可用列表、
   按 ID 查询和运行派发只暴露 15 个固定模型。
2. **实际请求参数一致。** 通过回环服务验证 Bearer
   POST /v1/chat/completions。Qwen/Qwen3.7-Max 请求发送
   reasoning.enabled=true 和更新后的 max_tokens，不发送未声明的
   reasoning_effort。
3. **入口集合封闭。** 5 个下线 ID 在 builtin 目录、列表、查询和运行派发前
   均不可见。未知 ID 返回 UnknownModel，回环服务收到零请求。
   build_catalog check、目录差分和归档 catalog consumer 通过。

## 实施结果

- Together 作者源 SHA-256：
  c5c325cc905566be93d5d1852ff57fedcaed3759af3340a421b3ee924e3517fe
- 全库 bundle SHA-256：
  c9353fe37bb1cd12806221ee0505a02bef06aee2d5cbc40c01de6c89ee568890
- manifest SHA-256：
  16610730632ba38823a968b6edb768a5d50810adccbc15002f353a9224e21bd4

固定 Pi 行为参考文件的 SHA-256：

- .pi/pi-1.5.0/packages/ai/catalog/providers/together.ts：
  d9bc2e64799ac178876456c46dbd63af9f81e18b0f3dbc5bb55d845392fba523
- .pi/pi-1.5.0/packages/ai/src/providers/together.ts：
  cbf232b1bfe5753bba855f0a95e2c4d180db8155121effcbdffc955fe090738c
- .pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts：
  e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a

## 测试回执

- cargo test -p opi-ai --offline --locked --test provider_collection --features builtins together：3 项通过。
- 受影响回归 union：317 项通过，包含 provider collection 140、catalog artifact 14、catalog operations 27、OpenAI Chat fixtures 116、external SDK 11 和 build_catalog example 9。
- build_catalog check、Clippy、目录差分和归档 catalog consumer：通过。

Test impact — add Together 目录、OpenAI Completions 请求和入口集合回归；
update builtin catalog bundle；retain OpenAI Chat、catalog 和归档消费者回归；
none coding-agent 入口。
