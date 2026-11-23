# #231 同步 Amazon Bedrock 目录与 Converse 请求

Status: done

沿用 [#230 OpenCode Go](230-sync-opencode-go-catalog.md) 的三项验收条件。通过 [#226 三方流程](226-sync-provider-catalog-three-way.md) 将 `amazon-bedrock` 从 Pi 1.3 发布基线更新到 [Pi 1.5 固定发布归档](../evidence/phase5-model-catalog.json)。请求行为依据同一版本 `packages/ai/src/providers/amazon-bedrock.ts`、`packages/ai/catalog/providers/amazon-bedrock.ts` 和 `packages/ai/src/api/bedrock-converse-stream.ts`。

## 目录同步与下线

三方合并无冲突。193 条增至 197 条，新增 4 条、下线 0 条。51 条共有记录新增 `compat.supportsStrictMode: true`，严格工具模式覆盖 150 条。所有记录的身份、字段及顺序与固定发布归档一致。已有记录的请求基址、协议和推理映射一致。

## 公开请求验收

用真实 builtin 装配和 `Models::complete_simple` 向本地 HTTP 服务发送 Converse 请求。五次请求分别验证 Bearer 认证、`POST /model/{id}/converse-stream`、事件流结束、严格工具的 `toolSpec.strict: true`、转换后的 JSON Schema 及普通工具原始 schema。

| 模型 | 推理请求 | 线上的字段 |
| --- | --- | --- |
| `anthropic.claude-fable-5`（本轮声明严格工具） | Max | `output_config.effort: "max"`，adaptive thinking 和 block binding |
| `au.anthropic.claude-sonnet-5-5` | Max | `output_config.effort: "max"`，adaptive thinking 和 block binding |
| `global.openai.gpt-6.1-terra`、`openai.gpt-6.1-terra`、`us.openai.gpt-6.1-terra` | XHigh | `reasoning.effort: "xhigh"` |

旧版本复现了已列出的 APAC 推理 profile 被家族校验拒绝，以及 Fable 的严格工具请求在发出前被拒绝。更新目录后抓到 Claude 的显式 `"max":"max"` 映射被错误地当成默认值，实际发成 `"high"`。现在已注册 Converse 模型按完整 ID 通过验证，Claude 序列化通过类型化映射取显式值。197 个目录 ID 均通过路由校验，另核对无模型记录的未知家族仍被拒绝。

## 产物验收

- 其余 41 份 provider 源文件逐字节一致。两次离线生成逐字节一致，`build_catalog check` 通过。
- 源目录 `amazon-bedrock.json` 的 SHA-256 为 `d99cfd50a440c4c3e9934b18857209186c57f55311aaf4d34613a31d67f6a8a7`。聚合 `catalog.json` 的 SHA-256 为 `4c323ee874c62a9dc2a7e58e545a9fe8c6914c1ef8c094f591ec24bba8b1c1d6`。
- 从实际 `opi-ai-0.8.3.crate` 安装的独立 `catalog-bundled` 消费者读出了相同摘要及计数。归档 SHA-256 为 `17e2c8efe2dd849bfe002b91f43b13e1eb0216fbb530462a08fe5988ae8056a7`。

## 执行检查

| 本地检查 | 结果 |
| --- | --- |
| `cargo test -p opi-ai --offline --locked --no-default-features --features builtins --test bedrock_fixtures --test provider_collection --test catalog_operations --test external_sdk` | 180 + 118 + 27 + 11 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --features catalog-bundled --test catalog_artifact` | 14 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --example build_catalog` | 8 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --features provider-bedrock --test provider_bedrock_profile` | 1 项通过 |
| `cargo test -p opi-coding-agent --offline --locked --test bedrock_provider_wiring --test provider_factory --test list_models` | 8 + 58 + 14 项通过 |
| `python -m unittest discover -s scripts/pi-catalog` | 14 项通过 |
| 独立归档消费、定向 Clippy、格式、架构、文档和 diff 检查 | 通过，Clippy 仍有 3 项既有库 lint |

## 待独立验收的行为

独立 `BedrockProvider` 工厂与 coding-agent 在本票结束时共用手写的三个默认 ID。[#232 统一目录入口](232-unify-amazon-bedrock-catalog-entrypoints.md) 核对这些 ID 和调用方。

Pi 对服务端特定的严格工具 schema 限额拒绝会只降低 `Prefer` 工具的严格度并重发一次，见 1.5.0 的 `packages/ai/test/bedrock-strict-tool-fallback.test.ts`。[#233 严格工具限额重发](233-bedrock-preferred-strict-tool-limit-retry.md) 负责第二份 payload、重新签名和重试边界。

## 下次上游基线

Mistral、OpenRouter、OpenCode Zen、OpenCode Go 和 Amazon Bedrock 已采纳固定的 Pi 1.5 发布基线，SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。其余 37 个 provider 使用 [#226 记录的 Pi 1.3 Git blob](226-sync-provider-catalog-three-way.md#mistral-基线)。全库 42 个 provider 共 1,707 条记录。11 个 provider 的身份与字段一致，31 个仍有差异。只读比较得到 70 条仅参考有、65 条仅 opi 有、64 条共有记录字段不同。

Test impact — add 已发布模型 ID 校验及 builtin Converse 请求回归。retain Bedrock 签名、事件流、CLI 装配和归档消费检查。
