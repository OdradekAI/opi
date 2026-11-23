# #258 同步 Qwen Token Plan CN 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Qwen Token Plan CN 入口的 SDK
内置聊天目录。参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

当前目录包含 20 条记录，固定目录包含 21 条记录。本批新增
`deepseek-v4.1-flash-1017`。该记录的上下文窗口为 1000000，价格字段按
固定目录保留为零值。固定目录没有 retired ID，因此入口闭合测试使用目录外
的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Qwen Token Plan CN 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   21 条 OpenAI Completions 记录，新增模型的上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /compatible-mode/v1/chat/completions`、Bearer `QWEN_TOKEN_PLAN_CN_API_KEY`
   认证、`max_completion_tokens` 输出上限、`enable_thinking=true` 和
   `reasoning_effort=high`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Qwen Token Plan CN 作者源 SHA-256：
  `e1a26d4a358d57f80b0f0b4cdb17b72494098185c22a8203828302d6ee1a6be7`
- 全库 bundle SHA-256：
  `7bf093a3625cfa6cecd48f25efcad4c675d2287a893e3ddaf21fdecb5304efac`
- manifest SHA-256：
  `36dbe21ee2c975e7e585c65a13392d24b39347a61d99860d397bd622c5ea19ec`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/qwen-token-plan-cn.ts`：
  `2ffe161e63b63e86780b1b668a7d6f4de7c831f5a730429fe2dae41577346e59`
- `.pi/pi-1.5.0/packages/ai/src/providers/qwen-token-plan-cn.ts`：
  `1869aabbff4baaa12faef00071aab0a5b1b1d38d43f16ce735137fc0e13fa897`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/qwen-token-plan-cn.json`：
  `80824536f0c3cd83ab27424df2d664738cbe3f971b4e568a625432288fa53229`

固定目录全库比较结果为 1,720 条 opi、1,712 条参考记录，1,709 条完全
相等，9 条仅 opi，1 条仅参考，2 条共有记录仍有字段差异。Qwen Token Plan CN
已从差异 provider 集合中移除，剩余 6 个 provider 有差异。

## 测试回执

- `provider_collection::qwen_token_plan_cn`：3 项通过。
- `provider_collection` 全目标：177 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Qwen Token Plan CN 目录、Bearer OpenAI Completions 请求和入口
集合回归；`update` builtin catalog bundle、上下文窗口字段和双语 provider 文档；
`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和额外模型别名。
