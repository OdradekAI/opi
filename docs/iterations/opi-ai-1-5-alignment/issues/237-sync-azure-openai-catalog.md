# #237 同步 Azure OpenAI 发布目录与运行入口

Status: done

沿用 [#236](236-sync-openai-catalog.md) 的三项验收。使用 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e` 的
[Pi 1.5 发布归档](../evidence/phase5-model-catalog.json)，以
[#226](226-sync-provider-catalog-three-way.md) 固定的 Pi 1.3 Git blob 为三方比较的旧基线。
请求依据为 Pi 1.5.0 的 `packages/ai/src/providers/azure.ts`、
`packages/ai/src/api/azure-openai-config.ts`、
`packages/ai/src/api/azure-openai-responses.ts` 和
`packages/ai/catalog/providers/azure.ts`。

## 目录与配置

- 新增 `chat-latest`、`gpt-5.6-cyber`、`gpt-6.1-terra`。
- 下线 `gpt-5.2-chat-latest`、`gpt-5.3-chat-latest`、`gpt-realtime-2.1`。
- 42 条共有记录逐字段一致。45 条聊天记录中，44 条走 Azure Responses，
  `deepseek-v4-pro` 走 Chat Completions。

SDK 内置装配和 coding-agent 工厂共用 `AzureCatalogProvider`。目录 ID 用于列表、
查询和结果。发送时从 `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` 读取
`model-id=deployment`，将部署名写入请求体 `model`，无匹配时使用目录 ID。
CLI 的显式端点和 API 版本优先于对应的环境变量。端点也可由
`AZURE_OPENAI_BASE_URL` 或 `AZURE_OPENAI_RESOURCE_NAME` 提供。

旧 `[providers.azure].deployments` 配置给出迁移错误。配置默认模型时选择
`azure:<model-id>`，再配置部署映射，例如 `gpt-6.1-terra=terra-prod`。
CLI 的 `doctor` 按端点来源检查必需配置。

## 三项验收

1. **目录和查询。** 比较源文件、生成 bundle 与发布归档的 45 条记录。SDK、
   coding-agent 运行工厂及静态列表的类型化记录完全一致。列表读取凭据零次。
2. **实际请求。** SDK 和 coding-agent 生产工厂各发出 7 次本地 HTTP 请求。
   Responses 使用 `POST /openai/v1/responses?api-version=…` 和 `api-key`，
   分别验证默认 `v1` 和 CLI 显式版本。DeepSeek 使用
   `POST /openai/v1/chat/completions` 和 Bearer。两条协议都核对了请求体的部署名
   和返回结果的目录 ID。Responses 验证 16-token 输出下限、温度、`store: false`
   及 Cyber、Terra 的 Lark 工具格式。Chat Latest Off 发送 `none`，Cyber Off
   省略 `reasoning`、Max 发送 `max`，Terra 的 Off、Minimal、Max 分别发送
   `none`、`low`、`max`。DeepSeek 的 Max 在 simple 入口收敛到
   `reasoning_effort: "high"`，中途指令使用 `system`，省略 `thinking`
   和 prompt-cache 参数。
3. **下线。** 三个下线 ID 无法从 Azure 源目录、SDK 查询、CLI 列表及运行工厂选取。
   即使部署映射含有这些 ID，运行入口也在发请求前返回 `UnknownModel`。
   非交互和 RPC 启动逐个验证配置错误，交互入口使用相同的启动校验。

## 目录产物

三方同步冲突为零，其他 41 份 provider 源逐字节一致。Azure 源 SHA-256 为
`1b008029b81c08faeb49a0c42970f478b07301b7cf83eea336188e1ca2562e68`，bundle 为
`6a06b38969313002a0fb1c38c1c83e65d5b7179fd2df23e0c862a8e777e3088b`。
两次离线重建逐字节一致，`build_catalog check` 通过。

全库共 1,697 条记录，15 个 provider 与固定发布目录一致，剩余 27 个有差异，
涉及新增 62 条、撤下 47 条、更新 59 条。

## 测试回执

- 工厂回归先复现一个下线默认 ID 被构造成唯一可选模型。Azure SDK wire 夹具
  46 项、`model_wire_metadata` 24 项、`provider_collection` 118 项、
  `builtin_scoped_env` 5 项及 transcript 29 项通过。Azure scoped 端点的库测试通过。
- CLI binary 39 项、`provider_factory` 60 项、`list_models` 16 项、
  `azure_openai_provider_wiring` 9 项、`config_tests` 58 项、`doctor_cli` 46 项
  及 `provider_route_diagnostics` 2 项通过。
- `catalog-bundled` 的目录验证 14 项通过。部署名映射 helper 的 feature 门控
  收窄到 `provider-azure-openai` 后，独立 Azure profile 测试通过。
  两种独立 `.crate` 消费均通过。归档 SHA-256 为
  `061e08d405b6fca98e8205fe79db58e026294f5224ad7b390e3c459df891fc35`，
  Azure 源、bundle 和 manifest 在归档与工作区中逐字节一致。
- 格式、架构、测试清单、文档及 diff 检查通过。定向 Clippy 通过并报告三项既有库警告。

Test impact — `add` 双协议实际请求、目录与下线验证、端点诊断及单 provider feature
目录校验。 `update` 工厂及配置迁移夹具。 `retain` SDK scoped env、重试冻结及
显式协议调用回归。

本批工作区验收基于 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0`，未产生提交。
全工作区暂存 0 项、未暂存 153 项、未跟踪 124 项。本批修改 20 个已有改动的路径，
另改 5 个只归属本批的路径。其余原有工作区改动的内容哈希一致。
