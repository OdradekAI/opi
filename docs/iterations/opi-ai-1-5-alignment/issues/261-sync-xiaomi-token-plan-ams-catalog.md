# #261 同步 Xiaomi Token Plan AMS 目录

Status: done

本票按固定 Pi 1.5.0 artifact（`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`）
同步 AMS 入口。目录从 4 条收敛为固定的 `mimo-v2.6-flash` 和
`mimo-v2.6-pro` 两条，撤下 `mimo-v2.5` 与 `mimo-v2.5-pro`。两条记录均为
1048576 上下文和零价格目录记录。

## 三项验收条件

1. 作者源、bundle、Catalog、Models 列表、可用列表、按 ID 查询和运行工厂逐条
   暴露固定两条 OpenAI Completions 记录。
2. 回环服务验证 `POST /v1/chat/completions`、Bearer
   `XIAOMI_TOKEN_PLAN_AMS_API_KEY`、`max_completion_tokens` 和
   `thinking.type=enabled`，不发送 `max_tokens` 或 `reasoning_effort`。
3. 两个下线 ID 在所有入口返回 `UnknownModel`，派发前不产生请求。

## 实施结果

- 作者源：`5c94d24b63659f18a172c9329a7f54a4a425361407318a82af48b17bde90130f`
- 全库 bundle：`b54a03b771ca33a4c72c58dde1942fc3f1719774d4691a6dc211974cfab8ea44`
- manifest：`79e3b858065b07f68ffcdb4e582af1b71dff0e773b4fc27f1f23725bd72852f`
- 全库差分：1,711 对 1,712 条，1,710 相等，0 条仅 opi，1 条仅参考，1 条字段差异。
- 固定 Pi 源：catalog `aaee6ae9d411b205cfc7b137f46b50690d22e4b89a0539e48115c76d9dbf66a2`，
  provider `59d03fc70eca720f8ca9a9f676aedaf241524dac26c179a78e2e30aabe116e0f`，
  data `34fe47b0a79fec7843a01d542cb64b736e651a3e6691cc851183e5ce3eae0118`。

## 测试回执

- `provider_collection::xiaomi_token_plan_ams`：3 项通过。
- 全量 provider_collection、catalog artifact、catalog operations、OpenAI chat fixtures、
  build_catalog、build_catalog check、归档 consumer、pack、fmt、文档和架构边界检查通过。
- clippy lib 通过，保留 3 个既有警告。

Test impact — `add` AMS 目录、Bearer 请求和下线入口回归；`update` bundle 和文档；
`retain` 共享 OpenAI Completions 回归；`none` 兼容行为。
