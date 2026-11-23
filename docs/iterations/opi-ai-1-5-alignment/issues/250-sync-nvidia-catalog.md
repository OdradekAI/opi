# #250 同步 NVIDIA 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 NVIDIA 的聊天目录。参考
artifact 的 SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
SDK builtin、Models 列表、查询和运行路由继续读取同一份 opi 作者目录。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| NVIDIA OpenAI Completions 聊天模型 | 19 → 21 | 3 | 1 | 1 |

新增 `minimaxai/minimax-m3.1-flash`、
`nvidia/nemotron-3.5-super-vl-120b-a12b` 和
`stepfun-ai/step-5-preview`。撤下 `z-ai/glm-5.3`。更新
`nvidia/nemotron-3-ultra-550b-a55b` 的价格字段为固定目录中的全零值。

NVIDIA 使用现有 OpenAI Completions 适配器和 `NVIDIA_API_KEY` Bearer
认证。固定记录使用 `max_tokens`，不声明 `reasoning_effort`。DeepSeek
V4.1 Flash 的 High 选择发送 `thinking: {"type":"enabled"}`，新增
Nemotron 记录没有该 thinking envelope。所有 NVIDIA 请求都带目录声明的
`NVCF-POLL-SECONDS: 3600`。本票不新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 NVIDIA 记录逐条
   相等。`Catalog::builtin`、Models 列表、可用列表和按 ID 查询均只暴露
   21 条固定记录，三个新增 ID 均可查询。
2. **实际请求路径、认证和推理参数一致。** 回环服务验证 Bearer
   `POST /v1/chat/completions`、NVCF 轮询头、`max_tokens` 字段以及
   DeepSeek 与 Nemotron 的目录推理 envelope。请求不发送
   `reasoning_effort` 或 `max_completion_tokens`。
3. **入口集合封闭。** `z-ai/glm-5.3` 不出现在内置目录、列表或查询中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   build_catalog、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- NVIDIA 作者源 SHA-256：
  `de0afd8bca38c648c1bbddf09160f6fa11e5865d0c8a376d9a4af2c207335616`
- 全库 bundle SHA-256：
  `880849b43c2f77ff624101046f7d763ea099f56788c4c4eeb548afde0c4dc63c`
- manifest SHA-256：
  `83cf36a99968561e9c0fbe9b48de721fce353778032bfdf295331b61613d94b9`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/nvidia.ts`：
  `1a082e47d1f0a48dac9c974b970384e4c0ab3e549d06c3c449809993897a473d`
- `.pi/pi-1.5.0/packages/ai/src/providers/nvidia.ts`：
  `73c981d27b364f83a32983d19f4ded346acf6af7eb4a653334622f891aedf860`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`

固定目录全库比较结果为 1,711 条 opi、1,712 条参考记录，1,692 条完全
相等，9 条仅 opi，10 条仅参考，10 条共有记录仍有字段差异。NVIDIA 已从
差异 provider 集合中移除，剩余 14 个 provider 有差异。

## 测试回执

- `provider_collection::nvidia`：3 项通过。
- `provider_collection` 全目标：153 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `external_sdk`：11 项通过。
- `build_catalog`：9 项通过。
- build_catalog check、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。

Test impact — add NVIDIA catalog, request-path, header, reasoning and closed
entrypoint coverage; update the builtin catalog bundle; retain OpenAI
Completions, catalog and archive consumer regression; none coding-agent entry.
