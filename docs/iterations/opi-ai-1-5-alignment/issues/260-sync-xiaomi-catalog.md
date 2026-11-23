# #260 同步 Xiaomi 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Xiaomi 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
作者目录、builtin bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 目录源。

固定目录从 6 条收敛为 3 条，撤下 `mimo-v2.5`、`mimo-v2.5-pro` 和
`mimo-v2.5-pro-ultraspeed`。保留的 MiMo 2.6 记录具有 1048576 上下文窗口，
`mimo-v2.6-flash` 的 input、output 和 cache-read 价格分别为 0.14、0.28 和
0.0028。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Xiaomi 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   3 条 OpenAI Completions 记录，价格和上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /v1/chat/completions`、Bearer `XIAOMI_API_KEY` 认证、
   `max_completion_tokens` 输出上限、DeepSeek `thinking.type=enabled`，
   且不发送 `max_tokens` 或 `reasoning_effort`。
3. **入口集合封闭。** 3 个固定目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Xiaomi 作者源 SHA-256：
  `f74d752ed9855855c453aa7d138b60dc88f37aaf2aa49789303396a4c8fda47f`
- 全库 bundle SHA-256：
  `b54a03b771ca33a4c72c58dde1942fc3f1719774d4691a6dc211974cfab8ea44`
- manifest SHA-256：
  `79e3b858065b07f68ffcdb4e582af1b71dff0e773b4fc27f1f23725bd72852f`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/xiaomi.ts`：
  `9d5004dc43be1f827cc1318ea3097d5b04c8ed6a671dfac6f76f78f77f8f85b6`
- `.pi/pi-1.5.0/packages/ai/src/providers/xiaomi.ts`：
  `d0d20dbcbcecca8b02948df1506ffa474461565546f7c24dfce44aebebef2344`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/xiaomi.json`：
  `65bc04b2ab6c9e97594de34bcea4ef7e45befd87901b4208d1ea4142b93f0561`

固定目录全库比较结果为 1,711 条 opi、1,712 条参考记录，1,710 条完全相等，
0 条仅 opi，1 条仅参考，1 条共有记录仍有字段差异。Xiaomi 已从差异 provider
集合中移除，剩余 4 个 provider 有差异。

## 测试回执

- `provider_collection::xiaomi`：3 项通过。
- `provider_collection` 全目标：181 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Xiaomi 目录、Bearer OpenAI Completions 请求和入口集合回归；
`update` builtin catalog bundle、模型撤下和双语 provider 文档；`retain` OpenAI
Completions、catalog 和归档消费者回归；`none` 兼容行为和 coding-agent 入口。
