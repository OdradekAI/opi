# #256 同步 Moonshot AI 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Moonshot AI 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型目录、内置 bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 作者目录。

当前目录包含 4 条记录，固定目录包含 6 条记录。本批新增
`kimi-k3.1-pro` 和 `kimi-k3.1-pro-highspeed`。新增记录使用固定目录中的
input、output 和 cache-read 价格，K3.1 Pro 的上下文窗口为 1048576。
固定目录没有 retired ID，因此入口闭合测试使用目录外的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Moonshot AI 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   6 条 OpenAI Completions 记录，K3.1 Pro 价格和上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /v1/chat/completions`、Bearer `MOONSHOT_API_KEY` 认证、
   `max_tokens` 输出上限字段以及 High 推理映射 `reasoning_effort=high`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Moonshot AI 作者源 SHA-256：
  `82eaba8206b40e2de476625e35a41d2bd0a73a3898bbe3979fcfcd7b5e91c047`
- 全库 bundle SHA-256：
  `3441f9e32f8307288c33cfc2c4519f2fa923960892b18289ef2d300f7f6acb8b`
- manifest SHA-256：
  `cf2c8498afbb2acb103c49bd05ecae7d3956b1ca18bedaf1ebad9b7fe588f34c`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/moonshotai.ts`：
  `994d88db4c173b06f54584604314640b07f73f1ff2342b6fb942ba3ee8bd4b00`
- `.pi/pi-1.5.0/packages/ai/src/providers/moonshotai.ts`：
  `2e269cca0231ebdbd2a58848ab3d19b0a43697e132b48a679e3c1b0cd4897665`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/moonshotai.json`：
  `f31cdf2e6c81a403c4cb0e6b7e759ceff2820f966b08dbec80cfafa3f80dfc63`

固定目录全库比较结果为 1,718 条 opi、1,712 条参考记录，1,707 条完全
相等，9 条仅 opi，3 条仅参考，2 条共有记录仍有字段差异。Moonshot AI 已从
差异 provider 集合中移除，剩余 8 个 provider 有差异。

## 测试回执

- `provider_collection::moonshotai`：3 项通过。
- `provider_collection` 全目标：171 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Moonshot AI 目录、Bearer OpenAI Completions 请求和入口集合
回归；`update` builtin catalog bundle、Moonshot AI 模型字段和双语 provider 文档；
`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和额外模型
别名。
