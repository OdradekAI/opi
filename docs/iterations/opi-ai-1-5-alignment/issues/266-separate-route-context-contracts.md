# #266 收敛 route context 的合同与策略归属

Status: complete
Owner: opi-ai provider contract and protocol adapters
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none
Related: [module structure decision](../decisions/06-module-structure.md), [#213](213-measure-lower-layer-dependencies.md)

## 证据

`src/provider/routes.rs` 当前同时承载 adapter 构造状态、Azure endpoint 规范化、
Bedrock endpoint 校验、OpenAI 兼容字段、Copilot 头策略、Responses affinity 和
多个 provider-specific policy。`check_architecture_boundaries.py` 仍记录 11 个
`api -> provider::routes` 依赖和 9 个经过批准的 `providers -> api` 装配/计费导出
例外。当前没有违规，但批准清单持续增长会让 provider contract 变成策略垃圾桶。

## 目标和范围

- 保留一个小而稳定的 provider-neutral route context 合同，只放身份、已解析端点、
  模型快照和传输资源。
- 将 adapter-native options、Copilot 头策略、Responses affinity、兼容字段和
  provider-specific URL 规范化移回所属 API 或 profile/factory。
- 让 API adapter 只依赖 provider contract、types、transport、auth/catalog contract，
  不读取 profile 类型或 profile policy。

## 独立验收

1. 为 route context 字段建立逐字段 owner 表，策略字段在 `api` 或 `providers` 有
   唯一实现，`provider/routes.rs` 不再定义重复 policy。
2. 架构检查的 approved route/context 例外只保留中性合同引用，profile-to-api
   违规为零，provider-to-provider 违规为零。
3. 所有受影响 wire fixture、provider lifecycle、provider profile matrix 和实际
   package consumer 通过，且公开 `Models`、`Provider` 类型路径保持不变。

## 执行顺序

先迁移纯 URL/endpoint 规范化，再迁移兼容和请求头策略，最后缩减 route context
字段。每一小步都保留对应 wire 的受控请求断言，不以静态扫描替代实际请求验证。

Test impact: update architecture edge baselines and focused wire/feature-profile
tests; retain all request and auth assertions.

## 第一阶段实施记录 · 2026-11-22

本阶段先冻结字段 owner，再迁移 URL/endpoint 规范化。字段表以当前 Rust
构造函数和实际调用者为准，避免仅按目录名判断归属。

| route context 字段或规则 | 当前唯一 owner | 使用者 | 结论 |
| --- | --- | --- | --- |
| Gemini identity、base URL、models、catalog 标记、client、wire | `provider::routes::GeminiRouteContext` | Gemini/Vertex wire 与 profile | 保留中性 route state |
| Vertex project/location、base URL、Express 标记、Gemini inner route | `provider::routes::VertexRouteContext` | Vertex wire 与 Vertex factory | 保留中性 route state；集合 URL 由 `providers::vertex` 构造 |
| Bedrock identity、base URL、models、HTTP clients、Smithy retry | `provider::routes::BedrockRouteContext` | Bedrock wire 与 profile | 保留中性 route state；标准端点判定由 `providers::bedrock` 构造 |
| OpenAI Chat wire、base URL、models、headers、client | `provider::routes::OpenAiRouteContext` | OpenAI-compatible adapters 与 factories | 保留中性 route state；策略由 `OpenAiRoutePolicy` 传入 configured route |
| Mistral、Pi Messages、Anthropic identity/endpoint/catalog/client | 对应 `provider::routes::*RouteContext` | 各自 wire adapter 与 profile | 保留中性 route state |
| Responses config | `provider::policies::ResponsesConfig` | Responses adapter 与 Azure Responses wire | 保留为 typed native contract |
| Responses affinity、Copilot/auth policy、catalog compatibility | `provider::policies::OpenAiResponsesRoutePolicy` | `providers::openai_responses` 构造，`api::openai_responses` 消费 | 不再属于 neutral route context |
| Azure endpoint、API version、deployment map | `providers::azure_openai` | Azure factory 与 Azure adapter | endpoint/version/deployment 解析已从 `provider::routes` 移出 |
| Azure Chat/Responses URL path | `api::azure_openai` | Azure wire adapter 与 direct API | wire URL 构造已从 `provider::routes` 移出 |
| Vertex collection URL | `providers::vertex` | Vertex wire 与 builtin auth | provider endpoint owner，auth 复用同一实现 |
| Bedrock 标准端点判定与 placeholder 校验 | `providers::bedrock` | Bedrock wire、catalog、auth helper | provider endpoint policy owner，adapter 通过 context method 调用 |
| generic slash joining | `api::shared::endpoint` | 多个 wire adapter | 继续保持已解析 URL 的中性拼接职责 |

已完成的 URL/endpoint 迁移如下。

- `provider/routes.rs` 不再定义 Vertex collection、Bedrock endpoint 或 Azure
  endpoint/version/deployment helper。
- API adapter 不再直接导入这些 helper，只依赖 route context 和 provider-owned
  route method。
- Azure Responses URL 构造回到 `api::azure_openai`，因为它属于 wire path。
- Vertex builtin auth 使用 `providers::vertex` 的同一集合 URL，避免两个 endpoint
  算法漂移。

证据文件见
[`issue-266-route-context-owners.json`](../evidence/issue-266-route-context-owners.json)。

三个独立归档包 consumer 也已通过：

- [`Azure package`](../evidence/issue-266-package-azure.json)
- [`Bedrock package`](../evidence/issue-266-package-bedrock.json)
- [`Vertex package`](../evidence/issue-266-package-vertex.json)

受影响的 wire/profile 检查中，Azure fixture 46/46、Vertex fixture 75/75，
三个 provider profile target 均通过。Bedrock fixture 的 endpoint、placeholder
signing 和实际 Converse 请求断言通过，但完整 target 仍有一个 carried-in 的
持久化 redacted-bytes base64 断言失败，该失败不触及本阶段修改的 endpoint owner。

## 第二阶段实施记录 · 2026-11-22

本阶段将 OpenAI Chat 与 Responses 的策略状态从 route context 的逐字段定义
收拢为独立的 typed policy contract。`provider::routes` 现在只引用 policy
contract，不再逐项声明 compat、catalog compatibility、auth-invalid、Copilot
或 Responses affinity 字段。

| 策略合同 | 构造 owner | wire consumer | 归属说明 |
| --- | --- | --- | --- |
| `OpenAiRoutePolicy` | `providers::openai` | `api::openai_completions` | provider factory 选择 compat、model overrides、认证错误分类、缓存与 Copilot initiator；Chat adapter 只解析和消费合同 |
| `OpenAiResponsesRoutePolicy` | `providers::openai_responses` | `api::openai_responses` | provider factory 选择 catalog compatibility、认证错误分类、affinity 与 Copilot headers；Responses adapter 只消费合同 |
| `ResponsesConfig` | `providers::openai_responses` | `api::openai_responses` | native Responses source/store/reasoning/session-header 配置保留为独立 typed contract |

本阶段没有新增兼容 shim，也没有改变公开 `OpenAiChatProvider`、
`OpenAiResponsesProvider`、`Provider` 或 `Models` 路径。策略的值仍由现有构造器
与显式 builder 设置，迁移只改变所有权边界和访问路径。

受影响的 wire/profile/package 检查如下。

- OpenAI Chat fixture 116/116，compat lifecycle 15/15。
- OpenAI Responses fixture 84/84。
- OpenAI Chat 与 Responses 独立 profile target 各 1/1。
- `provider-openai-chat` 与 `provider-openai-responses` 外部归档包 consumer 均为 PASS。
- `check_architecture_boundaries.py --enforce` 通过，`api_provider_edges=0`、
  `unapproved_profile_api_edges=0`、`provider_to_provider_edges=0`、
  `provider_copilot_policy_sites=0`。

证据文件已更新为
[`issue-266-route-context-owners.json`](../evidence/issue-266-route-context-owners.json)。
OpenAI Chat、OpenAI Responses 和 Azure dual-wire 的归档包 consumer 证据分别见
[`OpenAI Chat`](../evidence/issue-266-package-openai-chat.json)、
[`OpenAI Responses`](../evidence/issue-266-package-openai-responses.json) 和
[`Azure policy wrapper`](../evidence/issue-266-package-azure-policy.json)。

三项验收在本阶段收口。`OpenAiRouteContext` 与
`OpenAiResponsesRouteContext` 现在只承载中性路由状态，策略由独立的 configured
route wrapper 持有。Azure dual-wire 的 Chat inner route 也已切换到 configured
wrapper，Responses wire 复用了同一 policy contract。公开 provider alias 名称保持
不变，因此工作区调用方无需改变入口。
