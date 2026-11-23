# #255 同步 Kimi Coding 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Kimi Coding 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

当前目录包含 4 条记录，固定目录包含 6 条记录。本批新增
`k3.1-pro` 和 `k3.1-pro-highspeed`，并更新 `k3-256k` 的三项价格字段。
其固定价格为 input 3、output 15、cache-read 0.3。`k3-256k` 的上下文窗口
为 262144。固定目录没有 retired ID，因此入口闭合测试使用目录外的未知模型
验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Kimi Coding 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   6 条 OpenAI Completions 记录，`k3-256k` 的价格和上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /coding/v1/chat/completions`、Bearer `KIMI_API_KEY` 认证、
   `max_tokens` 输出上限字段以及 High 推理映射 `reasoning_effort=high`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Kimi Coding 作者源 SHA-256：
  `a8a178276a8cbb1bda5f99062238e8f9961a18b86b7c2091da91cddcbb6e7910`
- 全库 bundle SHA-256：
  `c46d9c076b306af44f8ae9ce8af0a7ed712e5c76f3960bfa3d1e8ef068d93719`
- manifest SHA-256：
  `d539195310610bdc830f6a1a1e87efc83f21303196bcdd32b3c4380263c6b38b`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/kimi-coding.ts`：
  `0b671154d03017b71ca25fd982373ca1c14ff4611d1673905e4dc8db892fe037`
- `.pi/pi-1.5.0/packages/ai/src/providers/kimi-coding.ts`：
  `7b74ceaa07c4b90fbafccaeff5085bc6493bce28a04097540a3589f23cbc14bd`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/kimi-coding.json`：
  `04e7914391e8e986d04b5cc8f84c49267d148a53a2d92bf968b7570dd8ff09b9`

固定目录全库比较结果为 1,716 条 opi、1,712 条参考记录，1,705 条完全
相等，9 条仅 opi，5 条仅参考，2 条共有记录仍有字段差异。Kimi Coding 已从
差异 provider 集合中移除，剩余 9 个 provider 有差异。

## 测试回执

- `provider_collection::kimi_coding`：3 项通过。
- `provider_collection` 全目标：168 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Kimi Coding 目录、Bearer OpenAI Completions 请求和入口集合
回归；`update` builtin catalog bundle、Kimi Coding 价格与模型字段和双语 provider
文档；`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和
额外模型别名。
