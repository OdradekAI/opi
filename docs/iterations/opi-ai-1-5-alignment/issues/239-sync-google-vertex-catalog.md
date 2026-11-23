# #239 同步 Google Vertex 发布目录与两种请求路径

Status: done

承接 #238 的三项验收。使用 Pi 1.5.0 的 `nix/model-catalog.json` 固定发布目录 SHA-256 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`，并按 #226 三方同步流程以 Pi 1.3 发布 blob 为旧基线。Vertex 从 14 条变为 13 条，撤下 `gemini-3.1-flash-lite-image`，13 条共有记录逐字段一致。 [#238](238-sync-google-catalog.md) · [固定发布目录](../evidence/phase5-model-catalog.json) · [#226](226-sync-provider-catalog-three-way.md)

独立工厂和 CLI 列表从审核后的 Vertex 作者文件取得完整记录。独立工厂与 SDK 装配使用相同的类型化投影，单 `provider-vertex` feature 能读取目录。统一后的内置配置拒绝 `[providers.google-vertex].models` 和 `[providers.vertex].models`，默认选择写入 `[defaults].model`，例如 `google-vertex:gemini-3.1-pro-preview`。SDK 的显式自定义模型通过自己的注册接口提供。 [Vertex profile](../../../../crates/opi-ai/src/providers/vertex.rs)

## 三项验收

1. **目录与入口。** 13 条 `google-vertex` 聊天记录的字段和顺序与发布归档一致。SDK、独立 Express／区域工厂、CLI 元数据列表和运行工厂的类型化记录逐条相等。列表用 Express key、静态 Bearer token、ADC 文件存在性三条来源验收，读取存储凭据零次。
2. **实际请求。** SDK 和 CLI 工厂各发出 6 个回环生成请求。Express 发出 `POST /v1/publishers/google/models/{id}:streamGenerateContent?alt=sse`，通过 `x-goog-api-key` 认证。Bearer 发出 `POST /v1/projects/{project}/locations/{location}/publishers/google/models/{id}:streamGenerateContent?alt=sse`，使用 `Authorization: Bearer`。SDK 的 Bearer 测试经过一次回环 ADC 交换并验证 `x-goog-user-project`，CLI 使用配置的静态 token。两条路径均覆盖实际代理前缀、结果的 `google-vertex` 身份和模型 ID。请求体核对温度 `0.25`、输出上限 `53`，及下表三种推理配置。独立 URL 夹具还覆盖 `us`、`eu` 的 `aiplatform.{location}.rep.googleapis.com`，以及缺项目的 Bearer 请求失败。
3. **下线。** SDK 目录、独立 Express／区域工厂、CLI 列表和运行工厂均无法选出 `gemini-3.1-flash-lite-image`。直接调用返回 `UnknownModel`，回环服务收到零个下线请求。CLI 非交互和 RPC 启动返回配置错误，交互入口使用同一个启动校验。

| 模型 | Simple 推理选择 | `thinkingConfig` |
| --- | --- | --- |
| `gemini-3.1-pro-preview` | Max | `thinkingLevel: "HIGH"`，`includeThoughts: true` |
| `gemini-3.1-flash-lite` | 省略 | `thinkingLevel: "MINIMAL"` |
| `gemini-2.5-flash-lite` | Minimal | `thinkingBudget: 128`，`includeThoughts: true` |

固定 Pi 源路径是 `.pi/pi-1.5.0/packages/ai/src/api/google-vertex.ts`，SHA-256 为 `942d6c99256afafaed777a28f5b15cdfbfc5648af3e7a35652fba0b7454ce841`。通过固定的 `@google/genai` 2.21.0 在 fetch 边界捕获七次请求，验证 Express 全局域名、Bearer 的 global／区域／多区域域名、两类显式模型端点、推理参数及配额项目头。opi SDK 的 `BuiltinVertexConfig::base_url` 用于代理主机或路径前缀，显式模型端点可选择 publisher 集合及 API 版本。

## 目录产物

三方同步零冲突，其他 41 份作者文件逐字节一致。Vertex 作者源 SHA-256 为 `b0caec39083c8ed58c562fa5e6ab60713dd03c8f850249077a28cadfb296beff`，bundle 为 `cbb59f1490ef66fd52c5807f16ab08f01a4c2b2ba221d9d12bc4649fc20b6417`。两次离线生成逐字节一致，`build_catalog check` 通过。

全库有 1,696 条记录，含 1,584 条 chat、48 条 classifier、64 条 image。17 个 provider 与固定发布目录完全一致。剩余 25 个 provider，待新增／撤下／更新为 60／44／59。

## 测试回执

- 第一轮回归复现独立工厂仅暴露 2 条模型。最终 SDK 12 个目标共 378 项通过，包含 `vertex_fixtures` 76 项、ADC owner 22 项、`builtin_scoped_env` 5 项、`provider_collection` 118 项和 Gemini／Vertex 共享生命周期 19 项。共享生命周期夹具分别使用各自的 provider 前缀。
- coding-agent 的 `vertex_provider_wiring` 16 项、`provider_factory` 62 项、`list_models` 18 项、binary 41 项、`config_tests` 58 项、`doctor_cli` 47 项通过。库测试 234 项通过，1 项忽略。
- `catalog_artifact` 14 项、目录生成器 8 项、独立 `provider-vertex` feature 1 项通过。同步脚本 14 项、归档脚本 36 项通过。格式、文档、架构、测试清单和 diff 检查通过。定向 Clippy 通过并报告三条既有库警告。
- 独立 `catalog-bundled` 和 `provider-vertex` 消费都通过。两份实际 `.crate` 的 SHA-256 为 `0e440f33ecc47691783a0d549fdba1b2840736c65ede72a9c04d75762e603b14`，其中 Vertex 作者源、bundle 和 manifest 与工作区逐字节一致。
- 附加 `test_hermeticity` 为 14 项通过、1 项失败。静态检查将既有 OpenCode 夹具中两处 `opencode.ai` 字面量判为外网调用。两处夹具均在发送前替换为回环地址，该文件与本批开始时逐字节一致。 [OpenCode 夹具](../../../../crates/opi-ai/tests/provider_collection.rs)

Test impact — `add` 双认证请求、目录一致性、列表凭据来源及下线回归。 `update` CLI 旧 models 配置验收、独立归档消费和共享生命周期夹具。 `retain` 显式模型端点与 ADC 回归。

本批验收基于工作区 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0`。暂存 0 项、未暂存 161 项、未跟踪 126 项。29 个本批路径中有 24 个带入已有改动，5 个仅归本批。其他带入改动的字节哈希与开始时一致。
