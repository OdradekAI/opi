# #253 同步 Cerebras 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Cerebras 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

固定目录和当前目录都包含 2 条记录。本批没有新增或撤下模型，只更新
`gpt-oss-120b` 的 `cost.cacheRead`，从 0 修正为 0.35。固定目录没有
retired ID，因此入口闭合测试使用目录外的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Cerebras 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露
   `gpt-oss-120b`、`qwen-3.8-27b` 两条 OpenAI Completions 记录，
   `gpt-oss-120b` 的 cache-read 价格为 0.35。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /v1/chat/completions`、Bearer `CEREBRAS_API_KEY` 认证、
   `max_completion_tokens` 输出上限字段和 `reasoning_effort=high`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Cerebras 作者源 SHA-256：
  `ad410711f936277adce646b1adf5c6db746860e130a37f80f75f3d36c356bda4`
- 全库 bundle SHA-256：
  `c919045012f7bb36be0c8663cd460136ed0246e148bfcb6bde61732b5f73e45b`
- manifest SHA-256：
  `b19b4082f4c007c142bb372ae8234743b3b029166a1077ebbf8f6b6dd473acb7`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/cerebras.ts`：
  `205c913bf1289768b862bdb414aea4259c21da22f981efb3e8ef0701ab62b5fe`
- `.pi/pi-1.5.0/packages/ai/src/providers/cerebras.ts`：
  `457fc35499abea92be4ad7add4f08502d069293380be61bf05b11ae5352bce4f`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/cerebras.json`：
  `2eb969b2956844f9a641d3de52c19f1791293c58f607282fb3b0ebfe85ff66f8`

固定目录全库比较结果为 1,714 条 opi、1,712 条参考记录，1,700 条完全
相等，9 条仅 opi，7 条仅参考，5 条共有记录仍有字段差异。Cerebras 已从
差异 provider 集合中移除，剩余 11 个 provider 有差异。

## 测试回执

- `provider_collection::cerebras`：3 项通过。
- `provider_collection` 全目标：162 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Cerebras 目录、Bearer OpenAI Completions 请求和入口集合
回归；`update` builtin catalog bundle、Cerebras 价格字段和双语 provider 文档；
`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和
coding-agent 入口。
