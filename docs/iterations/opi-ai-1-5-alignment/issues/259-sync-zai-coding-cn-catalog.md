# #259 同步 Z.AI Coding CN 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Z.AI Coding CN 的 SDK 内置聊天目录。
参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
作者目录、builtin bundle、Catalog 查询和 Models 运行入口继续使用同一份
opi 目录源。

固定目录和当前目录都包含 4 条记录。本批只修正
`glm-4.6v.cost.cacheRead`，从 0 更新为 0.05。固定目录没有 retired ID，
入口闭合测试使用目录外的未知模型验证拒绝路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Z.AI Coding CN 记录逐条相等。
   `Catalog::builtin`、Models 列表、可用列表、按 ID 查询和运行工厂只暴露固定的
   4 条 OpenAI Completions 记录，价格和上下文窗口与发布目录一致。
2. **实际请求路径、认证和参数一致。** 回环服务验证
   `POST /api/coding/paas/v4/chat/completions`、Bearer
   `ZAI_CODING_CN_API_KEY` 认证、`max_tokens` 输出上限字段，以及不发送
   `max_completion_tokens` 和 `reasoning_effort`。
3. **入口集合封闭。** 目录外模型不出现在 Catalog、列表或查询结果中，
   `prepare_call` 在派发前返回 `UnknownModel`，回环服务收到零请求。
   `build_catalog check`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Z.AI Coding CN 作者源 SHA-256：
  `b335901611ad018680e5bb7fe7ba12a9f1be28e9d1530c783cbd6f4567a29a92`
- 全库 bundle SHA-256：
  `68749db42a6bb4ec6542ead0d7c0b1fbb357ec4be150fa9d70af82ae73af76fa`
- manifest SHA-256：
  `d0b1a12323e0bbe1abe307070d59085c5dfc249a8e9d8fa32678bdd6a8c975e9`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/zai-coding-cn.ts`：
  `261df5914d6499deffb48b7de8044743d4681d616701fcd4d84772276fcb3a20`
- `.pi/pi-1.5.0/packages/ai/src/providers/zai-coding-cn.ts`：
  `89ff82b1f7b1a7e3427498598ce03574d76217f1c38b0d1731be82b6d8ff87a7`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`
- `.pi/pi-1.5.0/packages/ai/src/providers/data/zai-coding-cn.json`：
  `b335901611ad018680e5bb7fe7ba12a9f1be28e9d1530c783cbd6f4567a29a92`

固定目录全库比较结果为 1,720 条 opi、1,712 条参考记录，1,710 条完全相等，
9 条仅 opi，1 条仅参考，1 条共有记录仍有字段差异。Z.AI Coding CN 已从差异
provider 集合中移除，剩余 5 个 provider 有差异。

## 测试回执

- `provider_collection::zai_coding_cn`：3 项通过。
- `provider_collection` 全目标：180 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `build_catalog`：9 项通过。
- `build_catalog check`、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。
- `clippy --lib`：通过，保留 3 个既有警告。

Test impact — `add` Z.AI Coding CN 目录、Bearer OpenAI Completions 请求和入口
集合回归；`update` builtin catalog bundle、缓存读取价格与双语 provider 文档；
`retain` OpenAI Completions、catalog 和归档消费者回归；`none` 兼容行为和
coding-agent 入口。
