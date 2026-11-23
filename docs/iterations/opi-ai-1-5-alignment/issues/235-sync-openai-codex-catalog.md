# #235 同步 OpenAI Codex 发布目录与运行入口

Status: done

沿用 [#234](234-sync-github-copilot-catalog.md) 的三项验收。通过 [#226](226-sync-provider-catalog-three-way.md) 的三方流程，使用其中固定的 Pi 1.3 Git blob 和 SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e` 的 [Pi 1.5 发布归档](../evidence/phase5-model-catalog.json)。请求依据是该版本的 `packages/ai/src/providers/openai-codex.ts` 和 `packages/ai/src/api/openai-codex-responses.ts`。

## 目录与入口

SDK 源由 9 条变为 8 条。新增 `gpt-6.1-terra`，撤下 `gpt-5.3-codex-spark` 和 `gpt-5.5`，其他 7 条身份与字段和发布归档一致。CLI 的旧手写目录有 7 条，其中 `gpt-5.4`、`gpt-5.4-mini` 也需撤下。两份旧目录的下线并集为 4 个 ID。

SDK、coding-agent 静态列表和运行工厂使用内置目录的类型化投影。新 Terra 的上下文与输出上限为 500,000 和 128,000，原先 CLI 手写为 372,000 的三款 GPT-5.6 采用发布目录的 272,000 上限。Codex 请求从所选模型取得端点，由已解析的 OAuth 凭据提供认证和账户 ID。

## 三项验收

1. **目录和查询。** 用固定发布归档逐条核对 8 个模型及完整字段，确认 SDK、CLI 列表和运行工厂的类型化记录一致，静态列举读取凭据零次。两次离线重建结果逐字节一致。
2. **实际请求。** 用 SDK 内置装配及 coding-agent 生产工厂在本地分别发送 3 次 SSE 请求。捕获 `POST /codex/responses`、模型、Bearer 与 `chatgpt-account-id`。Terra 的 Off 发送 `{"effort":"none"}`，Minimal 发送 `{"effort":"low","summary":"auto"}`，Max 发送 `{"effort":"max","summary":"auto"}`。模型记录指向本地接收端，凭据携带的另一个本地端点收到零请求。
3. **下线。** 四个 `openai-codex:<id>` 在 SDK 目录、查询、CLI 静态列表及运行选择中均不可选，准备调用时返回 `UnknownModel`，本地 HTTP 收到零请求。测试账户手工提供四个旧 ID 加 Terra 时，SDK 可用列表只返回 Terra。CLI 的非交互与 RPC 逐个验证配置错误，交互使用同一启动校验。

## 目录产物

三方同步冲突为零。其他 41 份 provider 源文件逐字节一致。Codex 源 SHA-256 为 `b0b75d13a3ae5a18687901853a35dbd8f018eb70791f2d13920e9dcba0423448`，全库 bundle SHA-256 为 `e0801076f4c8c608f901d9740747737edfea1f28d70d5c1ba3c10e0d2090264c`。两次离线重建及 `build_catalog check` 通过。

全库共有 1,697 条记录，包含 1,585 条 chat、48 条 classifier 和 64 条 image。13 个 provider 的目录与固定发布数据一致。剩余 29 个 provider 有差异，待新增 68 条、撤下 53 条、更新 63 条。

## 验收结果

- 目录专项由改动前的 7 条对 8 条失败转为通过，已核对三处入口的 8 个完整模型记录。
- 真实 SDK 与 CLI 工厂的 6 次请求均完成。另将受影响的 OAuth 脱敏夹具接到使用 Terra 与测试 OAuth 凭据的本地 HTTP 入口，确认每次失败都实际发送了请求。四个下线 ID 的查询、发送前拒绝和默认配置错误专项通过。
- `provider_collection` 118 项、`model_wire_metadata` 24 项、`openai_codex_responses` 87 项、`codex_native_options` 21 项及独立 `catalog_artifact` 14 项通过。CLI binary 37 项、`list_models` 14 项、`openai_codex_provider` 4 项、`provider_factory` 58 项和受影响的 `oauth_auth` 3 项通过。
- 实际 `.crate` 的独立 `catalog-bundled` 消费通过，归档与工作区的 bundle、manifest 及 Codex 源文件逐字节一致。归档 SHA-256 为 `f87af59d5eabeeb46f5ad38ec2b08c13f68a22911682b043c7463d54937700d2`。格式、架构、测试清单、文档和 diff 检查通过。定向 Clippy 通过并报告 3 项既有库 lint。

Test impact — `add` 目录一致性、Terra 实际请求及四个下线 ID 回归。 `update` CLI 列表、运行工厂、启动校验和 OAuth 夹具。 `delete` 旧手写目录快照。 `retain` 定价阈值与 Codex 协议回归。
