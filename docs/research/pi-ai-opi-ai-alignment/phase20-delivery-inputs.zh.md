# Phase 20 交付输入基线（冻结）

> **Phase 20 注册交付输入证据 —— 由任务 20.1（DEL-01）于 2026-10-17 冻结。** 本页冻结 Pi 树／运行时／依赖／数据身份、当前 Opi HEAD、完整 AIR/V 身份处置、已发布能力清单及其明确排除项，以及 Phase 20 任务图背后的源码→Rust 测试映射输入。本页是非规范性证据：合同全部由双语交付来源（[spec-draft.zh.md](spec-draft.zh.md) / [spec-draft.md](spec-draft.md)）及其采纳附件拥有，[acceptance-matrix.md](acceptance-matrix.md) 是唯一验收权威。本页不新增或修改任何要求、不主张运行时支持、不记录任何已执行的验收结果。英文对应页为 [phase20-delivery-inputs.md](phase20-delivery-inputs.md)。

## 1. 冻结的 Pi 1.3.0 来源身份

| 项目 | 冻结值 |
|---|---|
| 来源路径 | `.repo/pi-1.3.0`（本地快照；无 `.git` —— 不是上游 tag/npm 身份证明） |
| 树 SHA-256 | `08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2`（1925 个文件） |
| AI 包 | `@earendil-works/pi-ai` 1.3.0 —— 396 个文件、199 个 `src` TS 文件、174 个测试文件 |
| Telemetry 包 | `@earendil-works/pi-telemetry` 1.3.0 —— 12 个文件、6 个 `src` TS 文件、2 个测试文件 |
| 前驱快照 | `.repo/pi-1.2.0`，树 SHA-256 `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10`（1902 个文件；ai 394/200/171，telemetry 12/6/2） |
| 1.2.0→1.3.0 全树差异 | 新增 30／修改 192／删除 7／未变 1703；ai 包 +4/41/2，telemetry +0/2/0 |
| 哈希方法 | 对按 Unicode 码点排序的 POSIX 相对路径做 UTF-8 拼接，每项 `路径 + NUL + 文件 SHA-256 hex + LF` 后整体 SHA-256；排除 `.git`、`node_modules`、`dist`；符号链接策略为全部拒绝 |
| 生成目录数据 | `packages/ai/src/providers/data/*.json` 在两个快照中均缺失；未 hydrate，因此不主张任何生成目录记录身份 |
| 版本解析 | 用户在 `.repo/pi-1.3.0` 之外写了 “1.12.0→1.13.0”；实际包 manifest 与可用目录确立 1.2.0→1.3.0 |

来源：[pi-1.3.0-source-evidence.json](pi-1.3.0-source-evidence.json)（2026-10-16 记录，Opi 输入 `c5ab42ab8888b93f9969652323caee87b7550392`）；工具与逐路径差异见该文件。

## 2. 参考运行时身份（仅离线研究 harness）

| 运行时 | 冻结身份 |
|---|---|
| Node | 24.13.0，内置 Undici 7.18.2 |
| Bun | 1.3.5 `+1e86cebd7`，按 exe SHA 与完整版本输出固定 |
| Schema 实验 | TypeBox 1.3.27 与原生 `jsonschema` 0.28.3，各有独立依赖锁图 |
| WS 实验 | TW/TTW 0.28.0 及独立锁图（已被否定为完整 parity 选择；替代见 §3.2） |

它们固定的是 §9.4 离线差分观察产生时所用的运行时，是研究 harness 身份而非产品运行时主张。来源：[upstream-tracking.md](upstream-tracking.md)、[pi-1.3.0-schema-ws-offline-validation.md](pi-1.3.0-schema-ws-offline-validation.md)。

## 3. 依赖身份

### 3.1 Pi `@earendil-works/pi-ai` 1.3.0 包集合

| 类别 | 依赖 |
|---|---|
| 运行时 | `@anthropic-ai/sdk` 0.129.0、`@aws-sdk/client-bedrock-runtime` 3.1127.0、`@earendil-works/pi-telemetry` 1.3.0、`@google/genai` 2.21.0、`@smithy/node-http-handler` 4.12.1、`http-proxy-agent` 9.1.0、`https-proxy-agent` 9.1.0、`openai` 7.19.0、`partial-json` 0.1.7、`typebox` 1.3.27 |
| dev | `@types/node` 22.19.19、`canvas` 3.2.3、`vitest` 4.1.11 |

telemetry 包未声明依赖。`openai@7.19.0` 已逐包验证 SHA-512 integrity，归档 SHA-256 `597532dcb9051ee8aa5edadedb57b798b42add68cc5aec1d67e994d0cfad07f1`，官方 tag commit `15f6e408a0359df50ee7a996616957cc14988659`（[sdk-target-proposal.md](sdk-target-proposal.md) 附录 A 与 [sdk-auth-provider-migration.md](sdk-auth-provider-migration.md) 的证据）。

### 3.2 设计选定的 Rust 后端集合（选型，不代表集成验收）

| 组 | 选定身份 |
|---|---|
| HTTP | crates.io 原版 `reqwest` 0.12.28 + `Hyper` 1.9.0，明确声明 RDF-22 HTTP parser/storage 接受范围；唯一显式 transport/retry owner；无 HTTP fork/namespace 义务 |
| TLS / ADC 签名 | `rustls` 0.23.40 / `ring` 0.17.14，实例 trust/mTLS 与显式 ALPN；原生默认为固定 Node 参考 CA bundle；ADC RS256 与来源要求的签名/验证/KX 使用私有 OpenSSL 3 `LibCtx`（内置默认 provider、仅内存 EVP C API），经由窄安全 Rust owner 走 rustls 公开 seam |
| WS | 自有严格 Upgrade（动态 headers/tail）+ 受控/打补丁的 Ratchet `ratchet_core`/`ratchet_ext`/deflate 1.2.1，含公开 PMD message-state 适配器与固定有限帧 RSV/length/checked-storage 修正；TW/TTW 0.28.0 退出完整目标 |
| AWS / ADC 来源协议 | Smithy Query/XML 0.62.1、`xmlparser` 0.13.5、`p256` 0.13.2、`sha1` 0.10.6、既有 `hmac`/`sha2` |
| Schema | 私有 TypeBox JSON 方言 IR/evaluator + `regress` 0.12.0 算法与可复现 Unicode16 数据、`serde_json` preserve_order、显式 JS 键枚举、`ryu-js` 1.0.2 |
| Telemetry | `tracing` 0.1.44 / `tracing-core` 0.1.36 |

这些是经 Phase 交付合同采纳、带有明确补丁/来源/数据出处的**选定设计身份**。HTTP 修订（2026-10-19）：HTTP 行采用 RDF-22；普通 reqwest/Hyper 已在产品 Cargo 图中，但完整新 profile、source-owner 组装与包消费者行为仍须实施验收。本次选型不新增编译通过声明。其他固定输入身份与历史计数保留原证据窗口；正式准入仍是独立门槛（[spec-draft.zh.md](spec-draft.zh.md) 后端附件；[pi-1.3.0-design-readiness.md](pi-1.3.0-design-readiness.md)）。

## 4. 数据身份

- `regress` 0.12.0 算法的生成 Unicode16 正则数据；可复现再生成（64/64 固定实例与 11 个合成错误控制，两次生成结果一致）。
- 私有 Unicode16 URL/IDNA/ICU 家族身份：10 个包命名空间与普通 ICU 2.2 共图且不串数据；`ryu-js` 1.0.2 通过 14 点与 29 点检查（43/43 叶）。
- 固定的 Node 参考 CA bundle 作为原生 TLS 默认（scoped/explicit root 归来源所有；host root 为显式选择）。
- 目录协议数据：distribution schema v1 与 AI hydration data v6，目录最低 Pi 版本 1.2.1 单独记录；Rust crate 版本不是 Pi 兼容身份。
- 未 hydrate 的 provider 数据：`packages/ai/src/providers/data/*.json` 在两个冻结快照中缺失；不主张任何内置目录记录身份。

## 5. 当前 Opi HEAD 与既往证据窗口输入

任务 20.1 开始时冻结：HEAD `96b97096226d0b9ef59c82c9eb6bb67f8ed9da2f`，分支 `main`（领先 `origin/main` 15 个提交），工作区干净，Phase 20 任务图确认于 2026-10-17T01:59:31+08:00。

| 证据窗口 | Opi 输入提交 |
|---|---|
| Pi 1.0.0 基线 | `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73` |
| Pi 1.2.0 审查 | `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8` |
| 1.2.0 Rust 工程修订 | `6b81658` |
| 参数修订与 Pi 1.2.0→1.3.0 升级复核 | `c5ab42ab8888b93f9969652323caee87b7550392` |
| 后端缺口专项 | `a37ed70` |
| Schema/WS 离线实验与设计就绪 | `8cb0fd68db528ae3d3c61874458d45e46bffa29c` |

历史窗口保留原始输入；仅当前 HEAD 行是 Phase 20 交付基线。

## 6. AIR 身份处置 —— 67 ＝ 57 active SDK ＋ 9 host-only ＋ 1 retired

- **Active SDK（57）：** AIR-001–AIR-008、AIR-010–AIR-014、AIR-020–AIR-036、AIR-038、AIR-040、AIR-042–AIR-059、AIR-061–AIR-067。
- **Host-only 影响（9）：** AIR-009、AIR-015、AIR-016、AIR-017、AIR-018、AIR-019、AIR-037、AIR-039、AIR-041。
- **退役（1）：** AIR-060（AssistantMessageFrame encoder/reducer 随 Pi 1.3.0 上游移除；退役不是通过，也不产生新义务）。

身份规则：编号不回收；host-only 行保留历史因果与下游影响，既不计 SDK 完成也不计 SDK 失败；混合组只计其 SDK 子边界。枚举权威：[spec-draft.zh.md](spec-draft.zh.md) 与 [acceptance-matrix.md](acceptance-matrix.md) 的 AIR→V 覆盖表。

## 7. V 身份处置 —— 59 ＝ 54 active SDK ＋ 4 host-only ＋ 1 retired

- **Active SDK（54）：** V-01–V-16、V-20–V-32、V-34–V-51、V-53–V-59。
- **Host-only 影响（4）：** V-17、V-18、V-19、V-33。
- **退役（1）：** V-52（随 AIR-060 退役；普通流事件、不可变事件时 partial 与最终 Result 仍归 V-40/V-41）。

同样适用编号不回收与子边界规则。本冻结未实施、未执行任何 active 组。

## 8. 已发布能力清单与排除项

### 8.1 十个 chat API

`openai-completions`、`mistral-conversations`（实际 `POST /v1/chat/completions`）、`openai-responses`、`azure-openai-responses`（provider `azure`）、`openai-codex-responses`、`anthropic-messages`、`bedrock-converse-stream`、`google-generative-ai`、`google-vertex`、`pi-messages`。

### 8.2 一个 image API

`openrouter-images` —— 单一非流式 Chat wire；首 choice 文本在前、有效 data-URL 图像在后；不下载 URL、不解码像素、不写文件。

### 8.3 四个 classifier API

`typesafe-system-one`（bool→`noul`）、`cloudflare-workers-ai-system-one`（Cloudflare `/run` 包装）、`llama-cpp-classify`（本地 tokenize/apply-template/completion 算法；无内置 factory —— 显式 custom provider 复用）、`openai-decisions`（bool→predicate；仅 API-key 可用性；504 不重试）。

### 8.4 四十二个内置 factory profile

| Provider ID | 注册 wire |
|---|---|
| `amazon-bedrock` | bedrock-converse-stream |
| `ant-ling` | openai-completions |
| `anthropic` | anthropic-messages |
| `azure` | azure-openai-responses ＋ openai-completions |
| `baseten` | openai-completions |
| `cerebras` | openai-completions |
| `cloudflare-ai-gateway` | anthropic-messages ＋ openai-completions ＋ openai-responses |
| `cloudflare-workers-ai` | openai-completions ＋ cloudflare-workers-ai-system-one |
| `deepseek` | openai-completions |
| `fireworks` | anthropic-messages ＋ openai-completions |
| `github-copilot` | anthropic-messages ＋ openai-completions ＋ openai-responses |
| `google` | google-generative-ai |
| `google-vertex` | google-vertex |
| `groq` | openai-completions |
| `huggingface` | openai-completions |
| `kimi-coding` | openai-completions（`/coding/v1`） |
| `meta` | openai-responses |
| `minimax` | anthropic-messages |
| `minimax-cn` | anthropic-messages |
| `mistral` | mistral-conversations |
| `moonshotai` | openai-completions |
| `moonshotai-cn` | openai-completions |
| `nvidia` | openai-completions |
| `openai` | openai-responses（默认）＋ openai-completions（显式）＋ openai-decisions |
| `openai-codex` | openai-codex-responses |
| `opencode` | anthropic-messages ＋ google-generative-ai ＋ openai-completions ＋ openai-responses ＋ typesafe-system-one |
| `opencode-go` | anthropic-messages ＋ google-generative-ai ＋ openai-completions ＋ openai-responses |
| `openrouter` | anthropic-messages ＋ openai-completions ＋ openrouter-images ＋ typesafe-system-one |
| `qwen-token-plan` | openai-completions |
| `qwen-token-plan-cn` | openai-completions |
| `qwen-token-plan-individual` | openai-completions |
| `radius` | pi-messages |
| `together` | openai-completions |
| `typesafe` | typesafe-system-one（仅 classifier） |
| `vercel-ai-gateway` | anthropic-messages（＋经 model baseUrl 的 System One） |
| `xai` | openai-responses |
| `xiaomi` | openai-completions |
| `xiaomi-token-plan-ams` | openai-completions |
| `xiaomi-token-plan-cn` | openai-completions |
| `xiaomi-token-plan-sgp` | openai-completions |
| `zai` | openai-completions |
| `zai-coding-cn` | openai-completions |

枚举来源：[pi-1.3.0-sdk-contracts.md](pi-1.3.0-sdk-contracts.md) §3 与 [acceptance-matrix.md](acceptance-matrix.md) 的 profile 表；共享 wire 不抹除 provider 特有 auth/默认/过滤行为，factory 名称本身不构成交付。

### 8.5 九个 OAuth flow 家族

Anthropic、OpenAI Codex legacy、OpenAI ChatGPT（direct）、GitHub Copilot、OpenRouter、Kimi Coding、Meta、xAI、Radius。

### 8.6 明确排除项

- 专用 OpenRouter Image API 与图像流式不是 Pi 1.3.0 已交付 adapter，不是虚构的 parity 要求；`openrouter-images` 是唯一图像面。
- Responses Lite、真实远端 deferred 协议、exporter/自动 domain-span schema 以及投机的通用 context/operation/plugin/transport 框架在注册来源重开之前保持在本候选之外。
- AssistantMessageFrame 与 compact-progress API 已退役（AIR-060 / V-52）；不存在 parity 义务。
- Agent 执行/授权/hook 策略、持久 session v3/子会话迁移/compaction、产品 tool_search/UI/启动默认、完整 MCP/codemode 与 virtual routing 产品保持宿主所有（AIR-009/015–019/037/039/041；V-17–19/V-33 仅追踪影响）。
- TS/Bun/browser/CLI 替代是 [pi-1.3.0-rust-differences.md](pi-1.3.0-rust-differences.md) 记录的有界平台差异，不是新运行时目标。
- 在 provider 数据缺席于冻结快照期间，不主张任何 hydrate/生成的模型目录数据身份。

## 9. 源码→Rust 测试映射输入（不主张运行时支持）

### 9.1 Rust 处置清单（输入）

`crates/opi-ai/tests/` 下 41 个 integration 文件：`anthropic_fixtures.rs`、`anthropic_thinking_roundtrip.rs`、`api_mapped_provider.rs`、`auth_contracts.rs`、`azure_openai_fixtures.rs`、`bedrock_fixtures.rs`、`connection_pooling.rs`、`context_budget.rs`、`custom_provider_registration.rs`、`gemini_fixtures.rs`、`gemini_lifecycle.rs`、`image_input.rs`、`image_provider_serialization.rs`、`mistral_fixtures.rs`、`model_capabilities_migration.rs`、`model_wire_metadata.rs`、`oauth_wire_shape.rs`、`openai_chat_fixtures.rs`、`openai_codex_responses.rs`、`openai_compat_lifecycle.rs`、`openai_responses_fixtures.rs`、`openai_responses_lifecycle.rs`、`openrouter_fixtures.rs`、`output_content_image.rs`、`per_request_auth.rs`、`phase14_remediation.rs`、`provider_collection.rs`、`provider_diagnostics.rs`、`provider_error_classes.rs`、`provider_lifecycle.rs`、`provider_trait.rs`、`proxy_support.rs`、`registry.rs`、`request_enrichment.rs`、`retry_backoff.rs`、`stream_events.rs`、`test_support_tests.rs`、`tool_result_no_leak.rs`、`tool_result_wire.rs`、`usage_cost.rs`、`vertex_fixtures.rs`。

以上保留原始交付输入清单，不代表当前文件数量。其中 remediation 回归现已合并到 `anthropic_fixtures.rs`、`openai_chat_fixtures.rs`、`model_wire_metadata.rs`、`api_mapped_provider.rs`、`provider_collection.rs` 和 `usage_cost.rs`；当前路径见 [acceptance-matrix.md](acceptance-matrix.md)。

7 个含 `cfg(test)` 的 src 文件：`src/anthropic.rs`、`src/endpoint.rs`、`src/openai_responses_shared.rs`、`src/stream.rs`、`src/bedrock/credentials.rs`、`src/bedrock/event_stream.rs`、`src/bedrock/sigv4.rs`。

这 48 个文件的逐断言 `add/update/delete/retain` 处置位于 [acceptance-matrix.md](acceptance-matrix.md)（“2026-10-16 测试迁移补核”）。它们是绑定合同核准与实现变更的未来建议；任务 20.1 不执行其中任何一项、不删除任何内容。

### 9.2 Pi 测试语料差异（1.2.0 → 1.3.0）

AI 包 171 → 174 个 `.test.ts` 文件：新增 4（`json-parse`、`anthropic-error-tool-result-images`、`openai-responses-mid-convo-effort`、`transform-messages-cross-model-thinking`）、修改 21、删除 1（`assistant-message-frame.test.ts`，随 AIR-060/V-52 退役）；telemetry 2 个测试文件摘要不变。文件数不是用例数，未执行任何 suite。

### 9.3 历史已执行基线（保留为历史）

`cargo test --offline --locked -p opi-ai --test gemini_lifecycle --test provider_lifecycle --test provider_collection` → 4 ＋ 8 ＋ 58 ＝ **70 passed, 0 failed**（外部 target 目录；见 [README](README.md)）。该基线早于 Phase 20 目标，不证明任何新目标。

### 9.4 已执行的离线研究观察（研究 harness，非产品测试）

| 组 | 冻结观察计数 |
|---|---|
| Schema（首窗口） | 85 个 Pi 公开 utility 用例；276 个原生观察 |
| Schema（设计就绪） | 64/64 固定实例 ＋ 11 个合成错误控制，两次生成一致；ryu-js 14 点与 URL 29 点检查（43/43 叶）；10 个包命名空间 ＋ 普通 ICU 2.2 共图 |
| WS（首窗口） | 60 个参考 constructor、15 个原生 loopback、16 个 reader、15 个 partial-codec 观察 |
| WS（length/storage 修正） | 14 个纯内存测试，含 12 个旧 codec fixture；未执行 socket/request |
| Transport/auth | 112 条主记录、16 个 Pi-proxy 观察、20 组 header 对照；ALPN/H2 与 TLS 负例；有来源的 metadata 超时（0/3000 ms） |
| Crypto | 11 个 RSA ＋ 3 个输入负例 ＋ poison ＝ 15 个观察；另 3 条 P521/e3mTLS 差异记录 |
| Telemetry | 固定安全 poll/drop 载体的 7 个场景 |
| Catalog | 42×3 条人工 helper 记录归并为 7 个观察 |

来源：[pi-1.3.0-schema-ws-offline-validation.md](pi-1.3.0-schema-ws-offline-validation.md)、[pi-1.3.0-design-readiness.md](pi-1.3.0-design-readiness.md) 与 [acceptance-matrix.md](acceptance-matrix.md) 末节。这些观察仅支撑设计选择；它们不是产品测试，不关闭七个覆盖组中的任何一个。

### 9.5 边界

任务 20.1 不运行任何运行时测试、不主张运行时支持。本节输入供后续 DEL 行（binding/streams、context、transport、auth、operations、telemetry、迁移）使用；执行证据属于那些任务，且必须对照唯一验收矩阵记录。
