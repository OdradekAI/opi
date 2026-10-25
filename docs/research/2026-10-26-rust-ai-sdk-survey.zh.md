# Rust AI SDK 调研与 opi-ai 选型建议

## 结论

**opi-ai 的 Anthropic、OpenAI、Gemini 直连数据面当前均建议继续使用直接 HTTP adapter。**
官方 Rust SDK 已覆盖 AWS Bedrock 和 Google Cloud / Vertex AI；Anthropic、OpenAI 直连与 Gemini Developer API 本次未发现对应的官方 Rust SDK。
社区 SDK 中优先跟踪 `async-openai`、`threatflux-anthropic-sdk` 和 `gemini-rust`；
`genai` 更适合采用其整套多厂商抽象的独立应用。

核查日期：2026-10-26。本文合并官方支持调查与社区 SDK 适配调查，保留原始版本、来源和源码快照。
本次仅合并整理，不重新调查实时状态。官方 SDK 的存在不等于已经通过 opi-ai 的替换验收；
Bedrock / Vertex 官方 SDK 的 opi-ai 接入一致性不在原调查的验证范围内。

### 官方支持速览

| 厂商与接入目标 | 官方维护的 Rust SDK | 对应项目或包 | 支持边界 |
|---|---|---|---|
| Anthropic Claude API 直连 | 本次未发现；官方客户端清单无 Rust | 无官方 Rust 对应项 | `@anthropic-ai/sdk` 是 TypeScript/JavaScript SDK；社区 crate 不代表厂商支持。 |
| AWS / Amazon Bedrock | 有，AWS SDK for Rust，GA | `awslabs/aws-sdk-rust`；`aws-sdk-bedrockruntime` | Rust 可通过 AWS 官方客户端调用 Bedrock 托管的 Claude 等模型。 |
| Google Cloud / Vertex AI Gemini | 有，Google Cloud Rust client libraries，GA | `googleapis/google-cloud-rust`；`google-cloud-aiplatform-v1` | 官方 Rust 客户端可生成内容，并提供流式生成接口。 |
| Google Gemini Developer API / AI Studio | 本次未发现；官方 GenAI SDK 清单无 Rust | 无已核实的官方 Rust 对应项 | 不能将 Vertex AI 的 Rust SDK 当作 AI Studio 接入的直接对应物。 |
| OpenAI API 直连 | 本次未发现；Rust 被归入社区库 | `64bit/async-openai` 是官方文档收录的社区项目 | 官方文档收录不等于 OpenAI 开发、维护或提供支持。 |

表中“官方”指厂商发布、维护并在其文档中确认为相应产品的 SDK；“GA”指厂商已宣布正式可用。
这不等于所有模型能力、预览接口和各语言 SDK 的功能完全一致，也不代表额外的商业支持 SLA。
下文给出逐项来源及社区交叉核查。

### 社区候选与采用顺序

| 目标 | 最值得进一步评估的候选 | 对 opi-ai 的建议 | 适合的其他场景 |
|---|---|---|---|
| OpenAI | `async-openai` | 优先参考其类型/schema，若要迁移需先解决内部无界 SSE 和 reqwest 版本边界；当前不直接引依赖 | 直接使用 Responses/Chat 的独立应用，本次 OpenAI 候选中优先级最高 |
| Anthropic | `threatflux-anthropic-sdk` | 优先跟踪；API 和流式契约较完整，但 MSRV、HTTP client 所有权与当前 workspace 不合 | Rust 1.99+、接受其 transport 配置的 Claude 专用应用 |
| Gemini 单厂商 | `gemini-rust`，仓库 `flachesis/gemini-rust` | 参考签名、工具和 Interactions 类型；当前继续自有 adapter | 新建 Gemini 应用、独立验证 Interactions |
| 多厂商应用 | `genai`，仓库 `jeremychone/rust-genai` | 作为另一套实现的比较对象；不在 opi-ai 内叠加第二套中立 runtime | 希望直接采用该库路由、Chat/Usage 抽象的上层应用 |

采用价值主要来自减少需要自行维护的正确代码；增加平台 API 数量、少写几行 builder，
不足以抵消回放、失败和流生命周期的重复适配成本。

### 继续 HTTP 的主要原因

- OpenAI：`async-openai` 的发布版 SSE 使用无界后台队列；reqwest 0.13 无法直接复用 opi 当前 0.12 Client。
- Anthropic：ThreatFlux 的流式与回放支持较完整，但 MSRV 1.99 高于 opi 的 1.97，且已查公开路径缺现有 HTTP Client 注入。
- Gemini：单厂商客户端仍有类型及 transport 适配成本；`genai` 又叠加中立 runtime，并具有不同的终态和签名处理语义。

保留 HTTP 的依据是统一认证、传输、重试、流资源及回放所有权；完整证据见后文。

### 保留 HTTP 同样需要补齐的能力

原调研在当时的工作树快照中发现以下覆盖缺口，未在线复现；本次合并未重新核查或修改这些实现：

1. [gemini.rs](../../crates/opi-ai/src/gemini.rs)的 Part 只有 Text/FunctionCall，
   没有 thought/thoughtSignature，函数结构只有 name/args。
   [Google 签名规则](https://ai.google.dev/gemini-api/docs/generate-content/thought-signatures)
   要求相应工具调用的签名原样回到原 part；这是实际的表示/回放缺口。
   当前收到完整函数调用后发出 ToolCallStart/End，不发参数 delta；
   按目前 generateContent 路径，不能只凭这一点判定协议缺陷。
2. [openai_responses_shared.rs](../../crates/opi-ai/src/openai_responses_shared.rs)的 RawOutputItem
   没有 encrypted_content 等完整 reasoning payload，reasoning events 进入 Ignore。
   当前没有证明完整 reasoning continuation 保真；SDK 的更丰富类型可以提供参考，但不会自动补齐中立消息/collector 映射。
3. 任一后端扩展都仍需遵守[规范 §7.1](../opi-spec.md)的公开中立 seam、失败与资源所有权要求。
   继续 HTTP 的建议不授权缩减已选定的能力目标，未来新增 wire 也不能用静默 fallback 替代。

## 官方支持状态与证据

### Anthropic

[官方 SDK 总览](https://platform.claude.com/docs/en/cli-sdks-libraries/overview)
列出的通用客户端为 Python、TypeScript、C#、Go、Java、PHP、Ruby，没有 Rust。
其 [TypeScript 官方仓库](https://github.com/anthropics/anthropic-sdk-typescript/blob/main/README.md)
明确说明 `@anthropic-ai/sdk` 面向 TypeScript/JavaScript；npm 命名空间不能证明存在对应的官方 Rust crate。

官方仓库中的两次讨论提供了历史背景：

- [#747 的回复，2024-11-19](https://github.com/anthropics/anthropic-sdk-python/issues/747#issuecomment-2484949790)
  明确表示当时没有 Rust SDK，并提到其他语言排在 Rust 之前。GitHub 将回复者标记为 CONTRIBUTOR；
  本文不额外推断其雇佣关系，也不把两年前的答复当作当前路线图。
- [#1559，2026-05-17](https://github.com/anthropics/anthropic-sdk-python/issues/1559)
  再次请求官方 Rust SDK；核查时仍为 open，已有评论未包含官方发布承诺。
  用户请求的存在只能证明需求，当前支持结论仍以官方 SDK 清单为准。

社区项目确实存在。例如
[ThreatFlux/anthropic_rust_sdk](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/main/README.md)
明确声明自己是社区维护，且不由 Anthropic 开发、背书或支持；其 Cargo 包名为
`threatflux-anthropic-sdk`。其实现能力和 opi-ai 采用价值见后文的社区 SDK 比较。

结论：Claude API 直连目前没有本次能够核实的官方 Rust SDK。
经 AWS Bedrock 调用 Claude 是另一条官方支持路径，支持方和接口契约属于 AWS。

### AWS

AWS 于 [2023-11-28 宣布 Rust SDK GA](https://aws.amazon.com/blogs/developer/announcing-general-availability-of-the-aws-sdk-for-rust/)，
并说明可用于生产环境。
[官方仓库 README](https://github.com/awslabs/aws-sdk-rust/blob/main/README.md)
说明 SDK 按 AWS 服务提供独立 crate，使用 Smithy 服务模型生成客户端，并提供默认凭据链等基础能力。
仓库位于 `awslabs` 不表示它只是未经支持的实验项目；GA 公告和官方文档直接确认其身份。

[Bedrock API 总览](https://docs.aws.amazon.com/bedrock/latest/userguide/api-reference-overview.html)
区分管理控制面和模型推理数据面，其 Rust 对应项为：

| 使用目的 | Rust crate |
|---|---|
| 模型推理、对话和流式输出 | `aws-sdk-bedrockruntime` |
| Bedrock 管理操作 | `aws-sdk-bedrock` |
| Bedrock Agent 管理与运行 | `aws-sdk-bedrockagent`、`aws-sdk-bedrockagentruntime` |
| 通用配置与凭据加载 | `aws-config`，由官方仓库使用示例说明 |

[官方 Rust 示例](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_bedrock-runtime_code_examples.html)
包含 Claude 的 Converse、ConverseStream 和工具调用。
因此“Rust 没有官方 Anthropic 直连 SDK”并不意味着“Rust 无法使用官方支持的客户端调用 Claude”。
这里实际调用的是 Bedrock 服务，不能据此宣称 Anthropic 自己维护 Rust SDK。

维护渠道也有实际证据：
[Bedrock 问题的维护者回复，2025-10-07](https://github.com/awslabs/aws-sdk-rust/issues/1365#issuecomment-3378355631)
记录了 COLLABORATOR 复现问题并转交服务团队的处理过程。这说明存在官方仓库维护通道；
它不证明该历史问题今天仍存在，也不证明已经修复。

### Google

#### Google Cloud / Vertex AI：有官方 Rust SDK

[Google Cloud Rust SDK v1.0.0 发布](https://github.com/googleapis/google-cloud-rust/releases/tag/v1.0.0)
日期为 2025-09-09，明确是首个 GA 版本，覆盖服务包括 Vertex AI。
因此，继续沿用早期社区帖子中的“Google Cloud 没有官方 Rust SDK”已经不准确。

[官方 Vertex AI Gemini Rust 教程](https://googleapis.github.io/google-cloud-rust/generate_text_using_the_vertex_ai_gemini_api.html)
给出的包是 `google-cloud-aiplatform-v1`，客户端是
`google_cloud_aiplatform_v1::client::PredictionService`，通过 `generate_content()` 调用 Gemini。
教程要求启用 Vertex AI API 的 Google Cloud 项目，并使用包含 project/location 的模型资源名。
其 [当前官方客户端源码](https://github.com/googleapis/google-cloud-rust/blob/main/src/generated/cloud/aiplatform/v1/src/client.rs)
也提供 `stream_generate_content()`。

这是一条已核实的官方 Rust 接入路径，但客户端属于 Google Cloud / Vertex AI，
并不是 Google GenAI 高层 SDK 的 Rust 版本。本文没有证明它与其他语言的高层 SDK 全功能对等。

#### Gemini Developer API / AI Studio：未发现官方 Rust 对应客户端

[Gemini API libraries](https://ai.google.dev/gemini-api/docs/libraries)
页面更新于 2026-10-15，详细语言列表包含 Python、JavaScript/TypeScript、Go、Java、C#，没有 Rust。
JavaScript 对应包为 `@google/genai`。

为避免仅凭高层 SDK 清单就漏掉低层官方客户端，又核查了
[Cloud Rust workspace](https://github.com/googleapis/google-cloud-rust/blob/main/Cargo.toml)
和 [生成库目录](https://github.com/googleapis/google-cloud-rust/blob/main/src/generated/_libraries.json)：
后者核查时包含 249 个 package key，没有发现 `generativelanguage` 或对应的官方 Rust 包。

因此，本次结论为：官方 GenAI 清单与 Cloud Rust 包目录均未发现 Gemini Developer API 的官方 Rust 客户端。
这是有范围和日期的调查结论，不是对所有未来发布或未公开项目的断言。
Vertex AI 路径和 Developer API 路径需要分别评估，不能因模型同名而将 SDK 视为直接替换关系。

### OpenAI

[OpenAI 官方 SDK 文档](https://developers.openai.com/api/docs/libraries)
将 Rust 的 `async-openai by 64bit` 放在 Community libraries 下，
并说明这部分项目由开发者社区维护，OpenAI 不验证其正确性与安全性。
这是判断支持身份的直接证据：文档收录的是社区库，不是 OpenAI 官方 Rust SDK。

[64bit/async-openai README](https://github.com/64bit/async-openai/blob/main/README.md)
也明确自称 unofficial Rust library。其当前文档列出了 Responses、Chat Completions、SSE、
Realtime 等能力，但这些是社区项目自己的能力声明，不应转述为 OpenAI 的支持保证。
基于官方 OpenAPI schema 构建，也不会自动获得官方 SDK 身份。

[OpenAI 官方 Go 仓库 #628](https://github.com/openai/openai-go/issues/628)
于 2026-03-21 提出官方 Rust SDK 请求，核查时仍为 open 且没有评论。
该请求不是 OpenAI 已承诺开发或发布日期的证据。

结论：OpenAI API 直连目前没有本次能够核实的官方 Rust SDK；
`async-openai` 属于官方文档收录、社区自行维护的候选库。

## 社区 SDK 比较

### 发布与维护活跃度

版本/日期实际读取 crates.io registry metadata，并下载发布归档检查 `.cargo_vcs_info.json`。
活跃度结合实质代码、发布及 PR 样本判断，没有使用 stars 或单独的 `pushed_at` 排名。
这是本次 shortlist，不是整个 crates.io 的全量审计或维护响应时间统计。

| Cargo 包 / 仓库 | 最新稳定 crate；发布日期 | 维护证据与判断 |
|---|---|---|
| [async-openai / 64bit](https://crates.io/crates/async-openai/0.42.2) | `0.42.2`；2026-10-10 | 高：9–10 月持续发布 spec、reasoning、tool chunk 和 schema 更新，多位贡献者修复进入发布版。 |
| [openai_dive / tjardoo/openai-client](https://crates.io/crates/openai_dive/1.4.3) | `1.4.3`；2026-05-06 | 中：main 在 8–10 月仍有 reasoning/usage 修复，但最新 crate 尚未包含这些修复。 |
| [openai-api-rs / dongri](https://crates.io/crates/openai-api-rs/10.0.2) | `10.0.2`；2026-10-03 | 间歇：3、4、10 月有发布；10.0.2 新增可观察 transport error 的 `try_` 流接口。 |
| [threatflux-anthropic-sdk / ThreatFlux](https://crates.io/crates/threatflux-anthropic-sdk/0.4.1) | `0.4.1`；2026-10-06 | 较高：10-04 大规模 API/流式补全，10-21 依赖及运行时修复；也有 bot 更新。GitHub `0.4.2` 已 release，但尚未成为最新 crate。 |
| [anthropic / AbdelStark/anthropic-rs](https://crates.io/crates/anthropic/0.0.8) | `0.0.8`；2024-09-03 | 发布滞后：2026-04 main 有现代工具/思考更新，包自称 0.1.0，但未发布；不能算作 0.0.8 的能力。 |
| [clust / mochi-neko](https://crates.io/crates/clust/0.9.0) | `0.9.0`；2024-06-30 | 低：main 最后提交为 2025-03-02，读取样本主要是旧 PR/依赖/README；类型仍属于早期 Claude 3/3.5 时代。 |
| [turboclaude / Epistates](https://crates.io/crates/turboclaude/0.3.0) | `0.3.0`；2026-01-08 | 低：发布后约九个月无主分支更新；README 的广泛 parity 声明需要逐条核实。 |
| [gemini-rust / flachesis](https://crates.io/crates/gemini-rust/2.1.0) | `2.1.0`；2026-09-05 | 中：7 月加入 Interactions，9 月有 finish reason/模型更新并合入外部贡献。 |
| [gemini-rs / Shuflduf](https://crates.io/crates/gemini-rs/2.0.1) | `2.0.1`；2026-07-21 | 稀疏：7 月修复 native TLS 依赖；[thought 类型问题](https://github.com/Shuflduf/gemini-rs/issues/17)自 2025-08 仍 open。 |
| [genai / jeremychone/rust-genai](https://crates.io/crates/genai/0.7.1) | `0.7.1`；2026-10-23 | 高：10-20/23 连续发布，多位贡献者修复 stream/tool/schema；10-24 的 main 测试变化不计入 0.7.1。 |

发布/提交证据：[async-openai releases](https://github.com/64bit/async-openai/releases)、
[openai_dive release](https://github.com/tjardoo/openai-client/releases/tag/1.4.3)、
[openai_dive main](https://github.com/tjardoo/openai-client/commits/master/)、
[openai-api-rs release](https://github.com/dongri/openai-api-rs/releases/tag/v10.0.2)、
[ThreatFlux releases](https://github.com/ThreatFlux/anthropic_rust_sdk/releases)、
[AbdelStark commits](https://github.com/AbdelStark/anthropic-rs/commits/main)、
[clust commits](https://github.com/mochi-neko/clust/commits/main)、
[TurboClaude commits](https://github.com/Epistates/turboclaude/commits/main)、
[gemini-rust commits](https://github.com/flachesis/gemini-rust/commits/main)、
[gemini-rs commits](https://github.com/Shuflduf/gemini-rs/commits/main)、
[genai releases](https://github.com/jeremychone/rust-genai/releases)。
仓库核查时均未归档；这不是未来维护承诺。

命名须同时看 Cargo 包和仓库：`anthropic-rs` crate 指向另一项目 `roushou/mesh`，
不是这里的 AbdelStark `anthropic`；`gemini-rs` 的 Rust import 名是 `gemini_rs`，
也不能与 `gemini-rust` 混淆。

### API、流式与工具能力

表中工具支持指 schema、模型调用、参数表示和结果回传；不将自动执行工具作为 opi-ai 的加分项。
能力依据下文固定版本源码；main 单独标注。普通文本流与工具参数增量分开比较。

| 候选 | 与 opi 相关的 API 覆盖 | 流式与工具调用 | 已见限制 |
|---|---|---|---|
| async-openai 0.42.2 | Chat、Responses、reasoning/加密内容、Azure/兼容服务配置；另有大量平台 API | 文本与工具参数 delta、交错调用身份、tool output、typed error/failed/incomplete | native SSE 无界；未知 event 类型无 catchall；reqwest 0.13 |
| openai_dive 1.4.3 | Chat、Responses、reasoning/encrypted_content | 文本/工具参数流、function output | 无界后台 reader；idle drop 等后续 send；main 修复尚未发布 |
| openai-api-rs 10.0.2 | Chat；Responses 大量使用 raw Value/extra | 新 `try_` 接口可观察 transport errors；Responses 需自行解释 raw event | 逐网络 chunk lossy UTF-8；现成 Responses tools 类型形状需要规避；自建 HTTP client |
| ThreatFlux 0.4.1 | Messages、count、Models、Batches、Files；thinking/redacted/unknown replay | `input_json_delta`、thinking/signature delta；有界队列、Drop abort、严格 collector 终态 | MSRV 1.99；reqwest 0.13；已查公开路径缺现有 Client 注入；collector 不保留 opi partial envelope |
| AbdelStark anthropic | main 有 Messages/count/models/batches、cache/thinking/tools；不可算作已发布 0.0.8 能力 | main 支持工具参数和思考流 | 发布脱节；发布版及 main 都有无界后台 producer |
| clust 0.9.0 | Messages；非流式 ToolUse/ToolResult | 文本 SSE；delta 类型不能表示 input_json 增量 | 没有现代 thinking/redacted/unknown 类型；模型枚举封闭且较旧 |
| turboclaude 0.3.0 | Messages 和较广资源类型 | raw 工具 block/partial_json 存在 | 最终 collector 把工具 block 转 Text，丢 ID/name/type；请求类型不足以完整回放 |
| gemini-rust 2.1.0 | generateContent、Interactions、files/cache/batch/embeddings，多模态及签名 | generateContent SSE、完整工具往返；Interactions 有 ArgumentsDelta | 当前 generateContent 函数类型缺 call-id；ClientBuilder 重建 client；未知字段及错误正文策略不匹配 |
| gemini-rs 2.0.1 | generateContent、模型列表、多模态和函数声明 | 按 poll 增量解析流式 JSON 数组；可读 function_call | typed Part 缺 function_response/thoughtSignature；usage 类型窄，难以支撑完整 Agent 往返 |
| genai 0.7.1 | 多厂商 chat/embeddings，Gemini、Interactions、Vertex adapter | 并行工具、signature、工具回传；Interactions 参数增量 | 另一套中立模型/路由；EOF 合成成功 End、未知 part 丢弃、缺签名 placeholder；reqwest 0.13 |

## opi-ai 的适配依据

### 原调研的比较基线

原调研基线 `HEAD=5340754478af8e1e416ceb7b3854c3f479063b4e`，但工作树已有大量未提交实现变更。
以下能力观察针对核查时的工作树，不冒充该提交本身的行为，也不表示这些实现已经验收。

- [规范 §3.2/3.3、INV-001/002](../opi-spec.md)要求真实调用与 adapter 的可观察语义、资源所有权和有界流；
  backend 默认值、输入收窄或换 transport 不能代替等价性证明。工具执行属于调用方。
- [根 manifest](../../Cargo.toml)声明 MSRV `1.97`；[锁文件](../../Cargo.lock)锁定 reqwest `0.12.28`。
  当前 [HttpClient](../../crates/opi-ai/src/http.rs)共享连接池，显式装配代理、TLS、redirect policy，
  关闭 reqwest 自身协议重试。这里不能笼统写成“所有 redirect 都禁用”。
- [Provider](../../crates/opi-ai/src/provider.rs)消费已经解析且冻结的认证；
  [Models](../../crates/opi-ai/src/models.rs)拥有 acquisition 重试。
  不能由 vendor 默认构造重新发现环境凭据，或偷偷叠加发送、刷新和重试。
- [ChatStream](../../crates/opi-ai/src/stream.rs)要求懒启动、单一结果、取消/失败保留 canonical partial、
  未见终态的 EOF 失败，以及 drop 释放 producer。现有四个主要 HTTP adapter 的事件队列容量均为 64。
- [Replay](../../crates/opi-ai/src/replay.rs)以 wire/provider/model 身份处理签名和工具往返；
  [ToolCall](../../crates/opi-ai/src/message.rs)保留原始参数与最终解析失败事实，
  不把不完整 JSON 前缀当成可执行的成功参数。

外层再套一个有界队列，不能消除 SDK 内部无界后台 reader 的积压。
“返回 Stream”“支持函数定义”“有 tool runner”也分别不证明背压、完整工具往返或符合 opi 的执行所有权。

### OpenAI：async-openai 值得参考，当前 transport 仍保留 HTTP

`async-openai` 是本次通用 OpenAI 应用中最值得优先评估的项目，
但已发布 `0.42.2` 的 [native SSE reader](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/src/client.rs#L820)
明确使用 `unbounded_channel` 和后台 task。
**该版本已经通过 `tx.closed()` 修复了空闲 drop；问题是背压，而不是声称它仍缺该修复。**
Responses 的 `create_stream` 和 BYOT 流最终都经过此路径；BYOT 更换输出类型不会更换 reader。

默认重试可以无 fork 关闭：启用 middleware，以
[`with_http_service(ReqwestService::new(client))`](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/src/executor.rs#L103)
替换默认带 OpenAIRetryLayer 的 executor。这解决重试所有权的一部分，不能解决 SSE 内部队列。
其 [manifest](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/Cargo.toml)
使用 reqwest 0.13，现有 opi 0.12 Client 无法直接传入。
两个版本可以在依赖图共存，但会失去直接复用当前 client 的收益，需另外证明 transport 配置一致。

其 [Responses 事件](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/src/types/responses/stream.rs)
和 [reasoning 类型](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/src/types/responses/response.rs)
比当前 opi 部分映射更丰富。可参考或以后单独验证 types-only features；此次不建议立刻增加依赖。
typed enum 没有 unknown catchall，BYOT Value 可以绕过该限制，但分类和映射责任又回到 opi。
还需审阅其默认环境读取及
[5xx 内部 tracing 原文](https://github.com/64bit/async-openai/blob/c2deb8880566c3db35979654ea9dd45dc3deffd9/async-openai/src/client.rs#L729)，
不能仅在收到 SDK 错误后再脱敏就宣称日志契约得到保留。

`openai_dive` 虽可注入 reqwest 0.12，
[已发布 reader](https://github.com/tjardoo/openai-client/blob/cf663ad43979572f1a72b7a75e28b284ede6d8e6/openai_dive/src/v1/api.rs#L328)
同样是无界后台 reader，drop 在下次发送才被感知；不是更好的生命周期替代。

`openai-api-rs` 的 [Responses reader](https://github.com/dongri/openai-api-rs/blob/e43ef8c0b61b8fe271b682e44c9baf9d6d2b43f9/src/v1/responses/responses_stream.rs#L131)
逐网络 chunk 调用 `from_utf8_lossy`，再拼接 String；跨 chunk 的多字节字符会被替换，
不能用于精确保留模型文本/参数。旧流接口还会将 transport error 打印后当 EOF，必须区分新增 `try_` 接口。
[请求实现](https://github.com/dongri/openai-api-rs/blob/e43ef8c0b61b8fe271b682e44c9baf9d6d2b43f9/src/v1/api.rs)
每次构造 request 自建 HTTP client。
Responses 的 raw Value 虽有灵活性，却没有明显减少 opi 已承担的协议映射工作。

### Anthropic：ThreatFlux 最完整，但当前有硬约束

ThreatFlux 0.4.1 的 [MessageStream](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94/src/streaming/message_stream.rs)
有默认容量 32 的有界队列、Drop abort、严格 `message_stop` 收集和内容/工具输入限制；
[delta 处理](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94/src/streaming/message_stream/content_delta.rs)
及 [replay](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94/src/models/common/replay.rs)
覆盖 input JSON、thinking/signature、redacted 和 unknown payload 策略。
这些是真实的实现能力，不能笼统归为“只有文本流”。

但 [发布版 manifest](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94/Cargo.toml)
要求 Rust 1.99，超过 opi 声明的 1.97；reqwest 为 0.13.5，默认 feature 是 native-tls。
TLS feature 可以重配，**MSRV 不会因此消失**。
[HTTP 实现](https://github.com/ThreatFlux/anthropic_rust_sdk/blob/3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94/src/utils/http.rs)
自行创建 client，已查公开 Client/Config 路径没有注入现有 client 的入口。
即使以后解决这些问题，仍应消费 raw typed events 并保留 opi 的 mapper/partial result owner；
其 collector 和默认内容上限不能自动成为 opi 的新契约。

其他样本的限制更直接：

- [clust delta](https://github.com/mochi-neko/clust/blob/dffe179643bb8c702159f6598a5ffb40a9b25bac/src/messages/message_chunk.rs)
  将 delta 固定为文本类型。虽然非流式有 ToolUse/ToolResult，不能满足现代工具参数流与 thinking 回放。
- [AbdelStark main reader](https://github.com/AbdelStark/anthropic-rs/blob/a3a6eb505cf9c7cb77e02322f64fab525c72f384/anthropic/src/client.rs)
  可注入 reqwest 0.12，但仍为无界 detached producer；main 新能力又尚未进入最新 crate。
- [TurboClaude collector](https://github.com/Epistates/turboclaude/blob/4a943bb09cbc2620ce587028ddbdf46be624d617/crates/turboclaude/src/streaming.rs)
  最终将缓存 block 统一变成 Text，不能保留工具 ID/name/type；
  [请求 content enum](https://github.com/Epistates/turboclaude/blob/4a943bb09cbc2620ce587028ddbdf46be624d617/crates/turboclaude/src/types/content.rs)
  也不足以完整表达 assistant 回放。其 README parity 声明不作为本次等价性结论。

因此 Anthropic 当前保留 HTTP，后续首先观察 ThreatFlux 的版本、注入入口与精确 conformance 成本。

### Gemini：单厂商 SDK 可参考，暂不叠加另一套 runtime

`gemini-rust` 的 [发布版模型](https://github.com/flachesis/gemini-rust/blob/6ac5aa4a550ec7420636cc2d146b50028a21dd48/src/models.rs)
和 [工具模型](https://github.com/flachesis/gemini-rust/blob/6ac5aa4a550ec7420636cc2d146b50028a21dd48/src/tools/model.rs)
包含签名和函数往返，覆盖比当前 opi typed Part 更丰富。
其 [generateContent reader](https://github.com/flachesis/gemini-rust/blob/6ac5aa4a550ec7420636cc2d146b50028a21dd48/src/client.rs)
是按 poll 消费的 SSE，没有上述 OpenAI 库的后台无界队列问题。
但 `with_http_client` 接收的是 ClientBuilder 并重建 client，且 generateContent 的已查函数类型没有 call-id 字段；
错误处理读取完整正文，也没有直接复用 opi 的有界分类、Retry-After 和脱敏路径。

该库将 generateContent 标记 deprecated、优先转向 Interactions，是库自己的 API 选择。
[Google 官方迁移说明](https://ai.google.dev/gemini-api/docs/migrate-to-interactions)
仍明确 generateContent 受支持。
官方迁移示例中的 generateContent 完整 functionCall chunk，与 Interactions 的 ArgumentsDelta
是两种 wire，不能用后者支持参数增量来证明前者能力。
本次未核查所有 Vertex 预览参数流或 Live API，不对这些模式作统一否定。

`genai` 在 [已发布 Gemini reader](https://github.com/jeremychone/rust-genai/blob/2d9aa9d15a1cfebe6f032045230497a5fc9d4aed/src/adapter/adapters/gemini/streamer.rs#L215)
中将没有 finishReason 的 EOF 也转成正常 End。
这与 opi [Gemini adapter](../../crates/opi-ai/src/gemini.rs)和 ChatStream 的缺终态失败契约不同，
需要额外验证原始终态，不能直接把它的 End 映射为 opi Done。
[Gemini mapper](https://github.com/jeremychone/rust-genai/blob/2d9aa9d15a1cfebe6f032045230497a5fc9d4aed/src/adapter/adapters/gemini/adapter_impl.rs)
还会跳过未知 part，在特定缺签名场景补 placeholder。
placeholder 有官方特定迁移场景的用途，但不能因此当成原始签名保真的替代品。
该库适合愿意采用其自身中立层的应用；在 opi-ai 内再包它，会重复 Chat/Usage、路由、回放和终态判断。

`gemini-rs` 的 [Part 类型](https://github.com/Shuflduf/gemini-rs/blob/686978d896142b18e2cb846ed189b6b218e9d9ec/src/types.rs)
缺少 function_response 和 thoughtSignature。存在一个名为 FunctionResponse 的类型、以及执行函数的示例，
不证明 typed Part 已支持完整工具结果回传。其更新频率和窄 usage 类型也不支持将其列为当前底座优选。

## 厂商 AI 社区和技术社区交叉核查

| 渠道 | 已读取的材料 | 能说明什么 |
|---|---|---|
| Anthropic 官方 GitHub 社区 | 上述 #747、#1559 | 历史回应和持续需求；不能推出当前发布计划。 |
| AWS 官方 GitHub 社区 | 上述 Bedrock issue 的维护者回复 | 存在维护和服务团队转交渠道；不能推出历史问题的当前状态。 |
| Google AI Developers Forum | [Rust 用户讨论 Gemini 兼容接口](https://discuss.ai.google.dev/t/invalid-argument-when-using-node-js-fetch/62280)，2025-01/02 | 用户会通过 HTTP/OpenAI 兼容接口接入；帖子本身不承诺官方 Rust SDK。 |
| OpenAI Developer Community | [OpenAI Rust client](https://community.openai.com/t/openai-rust-client/27082)，2022-12 起 | 作者请求将自己的项目列入社区库；厂商论坛出现的项目不自动成为官方 SDK。 |
| Rust 技术社区 | [async-openai 作者分享](https://www.reddit.com/r/rust/comments/zb08o3/async_rust_library_for_openai/)，2022-12；[Gemini Rust 库分享](https://www.reddit.com/r/rust/comments/1jz0ih6/rust_library_for_gemini/)，2025-04 | 社区库的存在和作者自述；不用于判定当前厂商支持状态或评估 SDK 质量。 |

搜索也覆盖 AWS re:Post。未找到改变上述支持分类的额外公告；搜索无结果本身不作为“不存在”的证据。
当前身份结论由厂商文档、官方发布和仓库声明共同支持，较早的社区帖子只提供背景。

## 方法、验证与范围

本文是非规范性选型证据，不修改产品依赖、公共契约、规范或实现台账，不构成实现准入。
中文专题报告没有既有英文 counterpart 需要同步。

- 确认官方身份 -> verify：实际读取厂商 SDK 清单、GitHub README 和 GA release。
- 核对社区能力 -> verify：registry 发布版本与日期、发布归档源码、主分支差异、具体事件和请求类型。
- 对照 opi-ai -> verify：原调研工作树的 manifest、调用/流/回放契约和固定输入哈希。
- 合并研究文档 -> verify：来源链接保留、本地链接和标题层级、文档检查及差异格式检查。

### 后续采用前的验证建议

若后续确有替换收益，应先用离线 fixtures 验证候选，且只按 owning workflow 进入实现：

- 请求所有权 -> verify：构造和未 poll drop 零请求；认证冻结；实际发送计数；handoff 后不重放。
- 流生命周期 -> verify：慢消费者有界、idle/半帧 drop、取消优先于缓冲事件、缺终态 EOF、失败保留 partial。
- 工具与回放 -> verify：交错多工具参数、非完整 JSON、结果图片/错误、签名原 part 往返、跨身份投影。
- Transport -> verify：同池/代理/TLS/redirect、超时、Retry-After、禁 SDK 叠加重试、原文不进入错误及 tracing。
- 实际依赖 -> verify：精确发布版本、MSRV、feature union、native/proc-macro/许可证和真实 opi-ai 产物。

### 本次证据的限制

原调查读取了 registry、官方文档、社区讨论和发布源码；核对了列出的版本、日期与源码身份。
没有编译 SDK、安装依赖、运行候选 conformance 或调用付费模型；没有证明完整服务端功能对等，
也没有测量编译时间、二进制大小和运行性能。旧社区帖子提供背景，不作为当前支持承诺。
后续依赖选型仍需针对具体发布版本和调用能力验证。

本次只合并两份研究文档，原源码观察冻结在下列快照；不将其自动更新为当前工作树的验收结论。
测试影响：`none`。文档检查选用 `python scripts/opi-doc-check.py` 和 `git diff --check`，
另检查合并报告的空白、本地链接和原始来源保留情况。

## 固定源码身份与原调研快照

下列 SHA 从发布包 `.cargo_vcs_info.json` 实际核对；AbdelStark 的现代 main 单独列出。

| 发布包 | 源码 SHA |
|---|---|
| async-openai 0.42.2 | `c2deb8880566c3db35979654ea9dd45dc3deffd9` |
| openai_dive 1.4.3 | `cf663ad43979572f1a72b7a75e28b284ede6d8e6` |
| openai-api-rs 10.0.2 | `e43ef8c0b61b8fe271b682e44c9baf9d6d2b43f9` |
| threatflux-anthropic-sdk 0.4.1 | `3abf3cf3fa58fe8afbe8f451f20b4bac1205fd94` |
| clust 0.9.0 | `dffe179643bb8c702159f6598a5ffb40a9b25bac` |
| turboclaude 0.3.0 | `4a943bb09cbc2620ce587028ddbdf46be624d617` |
| anthropic 0.0.8 | `d26016b3d249352a46178e4acc9f4481a5e95e0d` |
| AbdelStark 未发布 main 0.1.0 | `a3a6eb505cf9c7cb77e02322f64fab525c72f384` |
| gemini-rust 2.1.0 | `6ac5aa4a550ec7420636cc2d146b50028a21dd48` |
| gemini-rs 2.0.1 | `686978d896142b18e2cb846ed189b6b218e9d9ec` |
| genai 0.7.1 | `2d9aa9d15a1cfebe6f032045230497a5fc9d4aed` |

关键本地工作树输入 SHA-256，用于区分上面的 HEAD 与本次实际读取内容：

```text
Cargo.toml                       9671c94f8a5a02e451c3a476e3b398b4aa3797697ead2ee7d5e7a01151eeddfd
Cargo.lock                       d1be38802e71bc5617f5191a5b301335cccbaaf3e240da2eb3ab70a4f07c7f7a
docs/opi-spec.md                  7979f349811d5e8da8ed4b60adb0fa41175c76462a78e449fd326dd4c19cf9e1
opi-ai/src/gemini.rs              98781a5bf75a3056f7ff69cc59a31cc26c0e90d2c96f90d5585556a29323dc8b
opi-ai/src/openai_responses_shared.rs 262b6f055fb7dc99a4a67b2656cc80b05f9d5955543854341043d008afd3faad
opi-ai/src/http.rs                b755515b21d86459d6b3f3724e730665c3dc497219439528cfd3b146d8e857bb
```

上面最后三行的代码路径省略统一的 `crates/` 前缀。
本次合并仅改动两份原报告及合并产物，无提交、暂存、Rust/Cargo、规范或台账修改；
其他带入变更保持原有任务归属。
