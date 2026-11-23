# #254 同步 Groq 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Groq 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

固定目录和当前目录都包含 7 条记录。本批没有新增或撤下模型，更新了
`openai/gpt-oss-safeguard-20b` 的 `cost.cacheRead`，从 0 修正为 0.0375，
以及 `qwen/qwen3.8-27b` 的上下文窗口，从 131042 修正为 131072。
固定目录没有 retired ID，因此入口闭合测试使用目录外的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Groq 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露
   固定的 7 条 OpenAI Completions 记录，价格和上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /openai/v1/chat/completions`、Bearer `GROQ_API_KEY` 认证、
   `max_completion_tokens` 输出上限字段以及不发送 `max_tokens` 和
   `reasoning_effort`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Groq 作者源 SHA-256：
  `40321de941023c544568c84b918c824ea1a4d59d7ef15029b7b9846ebf62bf2b`
- 全库 bundle SHA-256：
  `4ac1c4da81c005fd2241cbb7555abdf4ad6b6ea80f034ada19ef8f6895eb2b0f`
- manifest SHA-256：
  `6ec930d42a0d07672a9343dc99dabd85ae41268558b4eb62eaf54b5fe852ab34`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/groq.ts`：
  `447a5a438e54d2fc5bfd6e1fe070ee82e2b2283a37b130681d7b955f48666fa5`
- `.pi/pi-1.5.0/packages/ai/src/providers/groq.ts`：
  `a8bbced4fbc37370892fec68fb7f03cf634d08d134d9f003fa63edb5a8b60873`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/groq.json`：
  `087df37206e9eed8b10f1e558e4c496289bef60074b7e8afba4f20fb102b4ec2`

固定目录全库比较结果为 1,714 条 opi、1,712 条参考记录，1,702 条完全
相等，9 条仅 opi，7 条仅参考，3 条共有记录仍有字段差异。Groq 已从
差异 provider 集合中移除，剩余 10 个 provider 有差异。

## 测试回执

- `provider_collection::groq`：3 项通过。
- `provider_collection` 全目标：165 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Groq 目录、Bearer OpenAI Completions 请求和入口集合
回归；`update` builtin catalog bundle、Groq 价格与上下文字段和双语 provider
文档；`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和
coding-agent 入口。
