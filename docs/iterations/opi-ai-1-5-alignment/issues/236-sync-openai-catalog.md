# #236 同步 OpenAI 发布目录与 Responses 入口

Status: done

沿用 [#235](235-sync-openai-codex-catalog.md) 的三项验收。目录以 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e` 的
[Pi 1.5 发布归档](../evidence/phase5-model-catalog.json) 为准，按
[#226](226-sync-provider-catalog-three-way.md) 固定的 Pi 1.3 Git blob 执行三方核对。
请求依据为 Pi 1.5.0 的 `packages/ai/src/providers/openai.ts`、
`packages/ai/src/api/openai-responses.ts` 和 `packages/ai/catalog/families/openai.ts`。

## 目录差异

- 新增 `chat-latest`、`gpt-5.6-cyber`、`gpt-6.1-terra`。
- 下线 `gpt-5.2-chat-latest`、`gpt-5.3-chat-latest`、`gpt-realtime-2.1`。
- `gpt-5.4`、`gpt-5.4-mini`、`gpt-5.4-pro`、`gpt-5.5` 移除
  `supportsAdditionalTools` 和 `supportsToolSearch`，保留严格 Schema、grammar 和中途指令能力。

OpenAI 共 45 条记录。其中 44 条聊天记录由 SDK 内置装配、独立 Responses 工厂和
coding-agent 共用。`gpt-6-luna` 的 OpenAI Decisions 分类记录仍按独立操作注册。
coding-agent 的 `openai:` 切到 Responses。已有的 `openai-responses:` 共用目录，
两处配置仍分别提供端点、凭据和代理。

## 三项验收

1. **目录和查询。** 逐字段核对 OpenAI 的 45 条记录。SDK 与 CLI 运行工厂的 44 条
   聊天记录一致，CLI 静态列表逐 ID 核对同样的完整模型数据，凭据读取为零次。
   两次离线重建逐字节一致。
2. **实际请求。** SDK 内置路由和两个 CLI 路由各向本地 HTTP 服务发出七次
   `POST /v1/responses`。捕获三个新增 ID、Bearer 和 SSE 终态。四个能力变更模型将
   晚加入的工具列在顶层 `tools` 中，请求保留 `strict: true`、Lark grammar 和中途指令，
   不含 `additional_tools` 或 `tool_search_*` 项。Terra 的对照请求以内联
   `additional_tools` 发送新增工具。CLI 的裸地址和 `/v1/` 基址得到相同最终路径。
3. **下线。** 三个旧 ID 在 OpenAI 源目录、SDK 查询、两处 CLI 列表和运行选择中撤下。
   三个实际运行入口均在准备调用时返回 `UnknownModel`，未发出 HTTP 请求。两个 CLI
   命名空间的非交互与 RPC 启动均逐 ID 验证配置错误，交互入口使用同一启动校验。

## 目录产物

三方同步冲突为零。OpenAI 源 SHA-256 为
`a1a5310ed8adc10b51c2fece9f8cd213f89cbaad9221a05c15812b0acf2f13c5`，bundle 为
`a743caea0b1824bc8751cdc47e69f03d9769042011622685ebad586cf5d49e82`。
`build_catalog check` 通过。其他 41 份源，包括 Azure，逐字节一致。

全库共 1,697 条记录，含 1,585 条 chat、48 条 classifier 和 64 条 image。42 个
provider 中有 14 个与固定参考一致，剩余 28 个有差异，涉及新增 65 条、撤下 50 条、
更新 59 条。下一批是 Azure。

## 测试回执

- CLI 列表回归先复现 `openai` 的 4 条与预期 44 条不符。调整后 `list_models` 15 项、
  `provider_factory` 59 项、CLI binary 38 项通过。SDK `provider_collection` 118 项、
  `model_wire_metadata` 24 项、`openai_responses_fixtures` 84 项、生命周期 15 项、
  transcript 29 项及全调用重试 23 项通过。
- 单独开启 `catalog-bundled` 时目录测试 14 项通过。单独开启
  `provider-openai-responses` 的 profile 及两种独立归档消费通过。归档 SHA-256 为
  `a1a97dafda23a3caf71c58b2532f5323adb4cfaa3555fc025b0c87b6de5a3ecd`。
  归档内的 OpenAI 源、bundle 和 manifest 与工作区逐字节一致。
- 格式、架构依赖、测试清单、文档及 diff 检查通过。定向 Clippy 通过并报告三项既有库警告。
- `tool_result_error_semantics_across_providers` 通过。同一测试目标的历史
  `provider_tool_result_error_no_phase12_breadth_guard` 仍要求已迁移的 `deferred` 公开模块。
  已核对该用例、`lib.rs` 和 Cargo 清单与本批开始时一致。

Test impact — `add` 目录一致性、实际请求及下线校验。`update` 模型记录、严格工具、Off
映射及请求路由夹具。`retain` 显式 Responses 配置和通用工具结果协议回归。

本批工作区验收基于 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0`，未产生提交。
全工作区暂存 0 项、未暂存 149 项、未跟踪 123 项。本批修改 14 个已有改动的路径，
另改 6 个只归属本批的路径。核对其余原有工作区改动的内容哈希一致。
