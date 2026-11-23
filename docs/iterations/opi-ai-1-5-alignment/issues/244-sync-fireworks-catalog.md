# #244 同步 Fireworks 目录与双协议请求路径

Status: done

## 范围与固定基线

本票只处理 `provider=fireworks` 的 SDK builtin 目录和共享协议路由。
固定 Pi 1.5 发布 artifact SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型集合以该 artifact 为准，Pi 源码只用于核对 Anthropic Messages 和 OpenAI
Completions 的请求行为。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Fireworks 聊天模型 | 22 → 29 | 7 | 0 | 1 |

更新记录为 `accounts/fireworks/routers/qwen-max-latest` 的
`compat.forceAdaptiveThinking=true` 和 `thinkingLevelMap` 中的 XHigh 映射。
本票不引入固定 artifact 之外的模型，也不保留旧模型别名或参数回退。

Fireworks 沿用现有共享 Anthropic Messages 和 OpenAI Completions 适配器，
不新增 provider 专属 Rust 工厂。`opi-coding-agent` 没有 Fireworks 内置入口，
因此不扩展 CLI 范围。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle、manifest 与固定 Pi 1.5 文件逐条相等。
   `Catalog::builtin`、注册后的 `Models` 列表、可用列表、按 ID 查询和运行工厂
   均只暴露 29 个固定模型，Qwen Max 的 adaptive 与 XHigh 元数据一致。
2. **两条实际请求路径一致。** 通过 builtin Fireworks 入口验证
   `POST /inference/v1/messages` 和 `POST /inference/v1/chat/completions`。
   Qwen Max 请求验证 adaptive thinking、`output_config.effort=xhigh` 和
   `x-session-affinity`。OpenAI Completions 请求验证 Bearer、请求体和
   `session_id`、`x-client-request-id`、`x-session-affinity`。
3. **入口集合封闭。** 固定目录之外的模型在 HTTP 派发前返回
   `UnknownModel`，回环服务收到零请求。`build_catalog check`、目录差分和归档
   catalog consumer 通过。

## 实施结果

- Fireworks 作者源 SHA-256：
  `9005b52ce1292c63cdb3aee24109fc99fb00069aae644988152b06b07fc17a85`
- 全库 bundle SHA-256：
  `836038eef26191b334f87e9235170c5dd38ddcd6715679d2b3bf26ebf9534c8f`
- manifest SHA-256：
  `0f1ecf1c60ac9048539273fb8314ee299dafc67693d89f264271e28ea20dc0b9`

固定 Pi 行为参考文件的 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/fireworks.ts`：
  `8b54992c69de29ad07b770df87c87814d7e3314572a6c087dbc9f8dab8504b91`
- `.pi/pi-1.5.0/packages/ai/src/providers/fireworks.ts`：
  `72dbb53e3bb8b0bc6c42dd7937ec11dd0126efe3176f11153b1d599a0c7776af`
- `.pi/pi-1.5.0/packages/ai/src/api/anthropic-messages.ts`：
  `f1348f1919aa9c4d2ed69767e4f64727e29df0455e8998a7da55773197fce48b`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`

## 测试回执

- `cargo test -p opi-ai --offline --locked --test provider_collection --features builtins fireworks`：3 项通过。
- 受影响回归 union：`provider_collection` 134 项、`catalog_artifact` 14 项、
  `catalog_operations` 27 项、`anthropic_fixtures` 155 项、
  `openai_chat_fixtures` 116 项全部通过。
- `build_catalog check`：通过。
- `run_catalog_bundled_package.py`：实际归档包外部消费者通过，42 providers、
  1589 chat、64 image、54 classifier。
- Anthropic Messages builtin 使用 `x-api-key`，OpenAI Completions builtin 使用
  Bearer。该认证差异由现有内置认证合同决定，没有修改生产认证逻辑。

Test impact — `add` Fireworks 目录、双协议请求和入口集合回归；`update` builtin
catalog bundle；`retain` Anthropic、OpenAI Chat、catalog 和归档消费者回归；
`none` coding-agent 入口。
