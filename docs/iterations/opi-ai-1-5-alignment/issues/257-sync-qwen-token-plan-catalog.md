# #257 同步 Qwen Token Plan 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Qwen Token Plan 全球入口的 SDK
内置聊天目录。参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

当前目录包含 20 条记录，固定目录包含 21 条记录。本批新增
`deepseek-v4.1-flash-1017`。该记录的上下文窗口为 1000000，价格字段按
固定目录保留为零值。固定目录没有 retired ID，因此入口闭合测试使用目录外
的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Qwen Token Plan 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   21 条 OpenAI Completions 记录，新增模型的上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /compatible-mode/v1/chat/completions`、Bearer `QWEN_TOKEN_PLAN_API_KEY`
   认证、`max_completion_tokens` 输出上限、`enable_thinking=true` 和
   `reasoning_effort=high`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Qwen Token Plan 作者源 SHA-256：
  `a4cee03de7d85eee69a17e5c4f44b2fac3e4a68685b6db1c0751e8264c500c03`
- 仅包含本入口新增记录的 bundle SHA-256：
  `fedba215c1792fb058494b164b218679bcca623b612ab5f4bb2ce685db438451`
- 对应 manifest SHA-256：
  `a255a6b8e5a9718f7cf3744e4d2f99d7ec412fc6325630111abd2aa93db2a956`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/qwen-token-plan.ts`：
  `67e1cce48ca567e67d1303afbecdeaae7471ad9fce93bbd961ae1fd966ba5626`
- `.pi/pi-1.5.0/packages/ai/src/providers/qwen-token-plan.ts`：
  `c26b85ab529779c4c2e61f6dd749592b515ec25b121b49dc81989d8a767d51f5`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/qwen-token-plan.json`：
  `0630bf7fea9d8579c7ac990e32c40c3685f9db20b6dbfaf745b361fe5203113c`

固定目录全库比较结果在仅加入全球入口记录后为 1,719 条 opi、1,712 条参考
记录，1,708 条完全相等，9 条仅 opi，2 条仅参考，2 条共有记录仍有字段差异。
Qwen Token Plan 已从差异 provider 集合中移除，剩余 7 个 provider 有差异。

## 测试回执

- `provider_collection::qwen_token_plan`：3 项通过。
- 当前双 Qwen 入口全量 `provider_collection`：177 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- 仅全球入口 bundle 的归档 catalog consumer、`build_catalog check`、目录差分、
  fmt、文档检查和架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Qwen Token Plan 目录、Bearer OpenAI Completions 请求和入口
集合回归；`update` builtin catalog bundle、上下文窗口字段和双语 provider 文档；
`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和额外模型别名。
