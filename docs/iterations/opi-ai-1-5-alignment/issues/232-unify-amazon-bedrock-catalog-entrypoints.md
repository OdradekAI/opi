# #232 统一 Amazon Bedrock 的模型目录入口

Status: done

依据 [#231](231-sync-amazon-bedrock-catalog.md) 采纳的 [Pi 1.5 固定发布归档](../evidence/phase5-model-catalog.json)，独立工厂与 coding-agent 读取同一份已经审核的 `amazon-bedrock.json`。单独启用 `provider-bedrock` 时内嵌这份源文件。聚合目录由同一源文件离线生成。独立工厂把标准 AWS 端点交给端点配置和认证区域选择。调用方提供的模型端点按已有路由规则处理。

## 三条旧默认记录与调用方

| 旧 ID | 固定发布目录中的相关记录 |
| --- | --- |
| `anthropic.claude-sonnet-4-20250514-v2:0` | `us.anthropic.claude-sonnet-4-20250514-v1:0` |
| `anthropic.claude-opus-4-20250514-v1:0` | `anthropic.claude-opus-4-1-20250805-v1:0` |
| `anthropic.claude-haiku-4-5-20250514-v1:0` | `anthropic.claude-haiku-4-5-20251001-v1:0` |

三条旧 ID 均不在固定发布目录。独立工厂负责 SDK 默认选集，coding-agent 的运行工厂和认证后的列表分别使用该构造器与 `model_catalog()`。生产代码的三条手写声明在此次撤下。CLI 的 SigV4 生命周期测试和 SDK 的默认目录测试改用发布目录中的 US Sonnet 4。底层签名向量和 RPC 的模拟目录拥有自己的测试记录。旧配置继续按用户填写的 ID 查询，缺失时报告未知模型，由用户选择当前目录中的 ID。

## 验收

1. 197 条模型从同一份发布源解码。比较独立工厂、聚合目录和 coding-agent 里的模型身份及类型化字段。CLI 的列表与查询均覆盖这 197 条，并验证三条旧 ID 已撤下。
2. 运行真实 SDK 和 coding-agent 工厂，在本地抓取 Converse 路径和认证。验证区域 ID 含冒号尾缀时仍完整发出，并验证用户配置的端点和签名区域。
3. 从真实 `.crate` 消费 `provider-bedrock`，核对模型数量和严格工具声明。资源逐字节保持已审核的发布数据。

## 验收证据

- `bedrock_fixtures`、`model_wire_metadata`、`provider_collection` 共 323 项通过。独立目录投影的 197 条记录与聚合目录一致，端点继续按运行时配置和认证区域解析。新增目录测试在改动前以 3 对 197 失败。
- coding-agent 的 `bedrock_provider_wiring`、`list_models`、`provider_factory` 共 80 项通过。列表覆盖 197 条，查询命中 `global.openai.gpt-6.1-terra`，旧 ID 从内置选集撤下。实际工厂发送 SigV4 签名的 `/model/us.anthropic.claude-sonnet-4-20250514-v1:0/converse-stream`。
- `--no-default-features --features provider-bedrock --test provider_bedrock_profile` 通过。真实归档的独立 Rust 消费端同样通过，feature 图确认未选入 `catalog-bundled`。归档 SHA-256 为 `4e561b54e35295d8eac40ebcdfcfde850af60848c2a5c245fc23b6e27606e071`。审核过的 Bedrock 源未变化，SHA-256 为 `d99cfd50a440c4c3e9934b18857209186c57f55311aaf4d34613a31d67f6a8a7`。

Test impact — add 目录一致性与真实归档断言。update 真实 CLI 列表和 Converse 回归、SDK 默认目录测试。retain SigV4 已知签名、显式代理和回放测试。
