# Phase 20 Audit

**Audit run ID**: `phase20-pi-glm53-6adacc37-20261101t164203z`
**Audit head**: `6adacc379df15ff2e5fa66ddb2afd3abeef5bb89`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: `pi coding agent harness (operator-named reviewer)`
**Reviewer model ID**: `glm53`

The reviewer model ID is the operator-supplied `model` slug, transcribed verbatim.
**Model identity source**: operator-declared
**Independence**: `fresh-context-same-family` — this peer sealed its baseline and requirements before any implementation inspection; no prior audit/remediation report, sibling peer output, or historical appendix entered Steps 1–5. One boundary deviation is disclosed below.
**Baseline policy**: `latest-committed-spec`
**Verdict**: PASS-WITH-FINDINGS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | `15c7f977d0c2cd7aee098836d5d47c195b402e1e2fb9a82a16fb69c89f52f975` | committed ledger baseline (stored v2 hash match) |
| docs/snapshots/phase20/opi-impl-state.json | `222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8` | committed ledger baseline (stored v2 hash match) |
| docs/opi-spec.md | `b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3` | committed spec (stored v2 hash match) |
| docs/opi-spec.zh.md | `e9da7696be0772e2ca72f74943c0b3fbba398850abc751d58726d31068cbb13c` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/spec-draft.md | `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/spec-draft.zh.md | `a841575a91ab4a3895648c1c66ce2c6c7d9afd4685fb1d408e550d39b3f62415` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md | `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md | `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md | `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md | `0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md | `b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md | `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md | `61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md | `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md | `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md | `25a4234507ab1c4c12d483c56bb4083172bf4d5c35dfec1dc92bc98c77a5d94e` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/replay-contracts.md | `223b21e1aa590d0e4646d99cd7f6792a694c7d8bf0f34b31dd6380979d324fa4` | committed spec (stored v2 hash match) |
| docs/research/pi-ai-opi-ai-alignment/sdk-target-proposal.md | `4f84b93681a6dce3bea494d0553e81a1f5cb3aec5ad62635eda93a8ee1f5a75c` | committed spec (stored v2 hash match) |

All 18 baseline digests match the sealed `git archive` export byte-for-byte; `stored_hash_mismatches` was empty. The registered scope is 68 ledger tasks covering 57 active SDK AIR requirements, 9 host-only impacts and 1 retired identity, plus 54 active V groups, 4 host-only and 1 retired, qualified by RDF-01–24, ONB-01–07 and the seven OPEN product-coverage groups owned by F.1a.

## Requirement Conformance

One record per registered task (independently decidable DoD with its scenario index); all records are mandatory. Full sealed criteria, observable behaviors, surfaces and per-task check evidence are in the requirements sidecar.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| T20.1 | task 20.1 '固定来源、能力与测试迁移基线' (parent row DEL-01; scenarios: P20-S01, P20-EXIT-01, DEL-01) | Registered baselines verified: all 18 baseline-source digests match the sealed export byte-for-byte; opi-do... | met | — |
| T20.2 | task 20.2 '统一最终 Result、不可变事件与 owned stream 生命周期' (parent row DEL-02; scenarios: V-03, V-13, V... | crates/opi-ai/tests/stream_events.rs: executed green, 28 passed / 0 failed (individually observed) | met | — |
| T20.3 | task 20.3 '单一 Models、冻结 binding 与 typed custom chat 调用' (parent row DEL-02; scenarios: AIR-04... | crates/opi-ai/tests/provider_collection.rs: executed green, 96 passed / 0 failed (individually observed) | met | — |
| T20.4 | task 20.4 '唯一 acquisition retry／advanced attempt 状态机' (parent row DEL-02; scenarios: P20-S04) | crates/opi-agent/tests/agent_loop_semantics.rs: executed green, 22 passed / 0 failed | met | — |
| T20.5 | task 20.5 '逐字节 framing、最终参数解析及全部 producer 退出' (parent row DEL-02; scenarios: V-02, V-04, AIR-... | crates/opi-ai/tests/provider_lifecycle.rs: executed green, 21 passed / 0 failed (individually observed) | met | — |
| T20.6 | task 20.6 'Context／Transcript chronology 与单一输入归一化' (parent row DEL-03; scenarios: V-06, V-31,... | crates/opi-ai/tests/transcript_contract.rs: executed green, 13 passed / 0 failed (individually observed) | met | — |
| T20.7 | task 20.7 '工具关联、跨模型 replay、opaque 与 projection' (parent row DEL-03; scenarios: V-07, V-08, AI... | crates/opi-ai/tests/replay_contract.rs: executed green, 22 passed / 0 failed (individually observed) | met | — |
| T20.8 | task 20.8 'Schema 上游依赖 profile 与早期外部 package 证明' (parent row DEL-04; scenarios: (DoD-only)) | crates/opi-ai/tests/schema_engine.rs: executed green, 65 passed / 0 failed (individually observed) | met | — |
| T20.9 | task 20.9 '完整私有 TypeBox JSON 方言 IR／resolver／formats' (parent row DEL-04; scenarios: (DoD-only)) | crates/opi-ai/tests/schema_engine.rs: executed green, 65 passed / 0 failed (individually observed) | met | — |
| T20.10 | task 20.10 '公开参数 normalizer／validator 与 Agent 消费' (parent row DEL-04; scenarios: V-35, AIR-02... | crates/opi-ai/tests/schema_public.rs: executed green, 24 passed / 0 failed (individually observed) | met | — |
| T20.11 | task 20.11 'strict／grammar wire 投影与有限降级变体' (parent row DEL-04; scenarios: V-34, AIR-040, P20-... | crates/opi-ai/tests/anthropic_fixtures.rs: executed green, 113 passed / 0 failed (individually observed) | met | — |
| T20.12 | task 20.12 '源限定 HTTP／H2／proxy／redirect 与上游接受范围 profile' (parent row DEL-05; scenarios: P20-S12) | crates/opi-ai/tests/connection_pooling.rs: executed green (aggregate run) | met | — |
| T20.13 | task 20.13 '上游 rustls／AWS-LC 私有安全签名机制与原生准入' (parent row DEL-06; scenarios: (DoD-only)) | private crypto module unsafe inventory frozen by platform_delivery (6 opi-ai files incl. ws io/pmd, schema ... | met | — |
| T20.14 | task 20.14 '真实 TLS／roots／mTLS／ALPN 与上游 crypto 包消费者' (parent row DEL-06; scenarios: P20-S14) | crates/opi-ai/tests/tls_contract.rs (public HttpClientBuilder::tls(TlsOptions) -> real Models/Chat/HttpClie... | met | — |
| T20.15 | task 20.15 '中立自有 WS／PMD／Close 与严格 Upgrade 接入' (parent row DEL-07; scenarios: (DoD-only)) | historical companion contract preserved (no companions/ dir remains; consolidation per RDF-24 executed in 2... | met | — |
| T20.16 | task 20.16 '真实 Codex WS／Auto／continuation 公共路径' (parent row DEL-07; scenarios: AIR-036, P20-S... | consolidation verified: src/ws/{frame,io,pmd}.rs are pub(crate), public opi_ai::ws Upgrade API preserved, p... | met | — |
| T20.17 | task 20.17 '实例 resource generation／pool／await cleanup' (parent row DEL-07; scenarios: V-29, A... | crates/opi-ai/tests/openai_codex_responses.rs: executed green, 74 passed / 0 failed (individually observed) | met | — |
| T20.18 | task 20.18 '惰性 AuthContext／默认来源与管理结果' (parent row DEL-08; scenarios: P20-S18) | crates/opi-ai/tests/auth_contracts.rs: executed green, 106 passed / 0 failed (individually observed) | met | — |
| T20.19.1 | task 20.19.1 'AWS 本地来源／process 与真实 Bedrock 调用' (parent row DEL-08; scenarios: P20-S19-1) | crates/opi-ai/tests/bedrock_fixtures.rs: executed green, 176 passed / 0 failed (individually observed) | met | — |
| T20.19.2 | task 20.19.2 'AWS STS role／web identity 与 ECS／IMDS' (parent row DEL-08; scenarios: P20-S19-2) | crates/opi-ai/tests/bedrock_fixtures.rs: executed green, 176 passed / 0 failed (individually observed) | met | — |
| T20.19.3 | task 20.19.3 'AWS SSO／OIDC／login 与完整来源链验收' (parent row DEL-08; scenarios: P20-S19, V-09, AIR-... | crates/opi-ai/tests/bedrock_fixtures.rs: executed green, 176 passed / 0 failed (individually observed) | met | — |
| T20.20 | task 20.20 'AWS standard／adaptive／Smithy2026 retry 模式' (parent row DEL-08; scenarios: P20-S20) | crates/opi-ai/tests/bedrock_fixtures.rs: executed green, 176 passed / 0 failed (individually observed) | met | — |
| T20.21.1 | task 20.21.1 'ADC 原生凭据／refresh／direct metadata 与 Vertex 调用' (parent row DEL-08; scenarios: P2... | crates/opi-ai/tests/vertex_fixtures.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.21.2 | task 20.21.2 'ADC external-account federation 与可取消 supplier' (parent row DEL-08; scenarios: P... | crates/opi-ai/tests/vertex_fixtures.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.21.3 | task 20.21.3 'ADC impersonation／递归组合与五家族验收' (parent row DEL-08; scenarios: P20-S21) | crates/opi-ai/tests/vertex_fixtures.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.22 | task 20.22 'store-owned mutation ACK／rotation／barrier 与宿主桥' (parent row DEL-09; scenarios: V-... | crates/opi-ai/tests/auth_contracts.rs: executed green, 106 passed / 0 failed (individually observed) | met | — |
| T20.23 | task 20.23 'SDK OAuth：Anthropic' (parent row DEL-09; scenarios: P20-S23) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.24 | task 20.24 'SDK OAuth：OpenAI Codex legacy' (parent row DEL-09; scenarios: P20-S24) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.25 | task 20.25 'SDK OAuth：OpenAI ChatGPT' (parent row DEL-09; scenarios: P20-S25) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.26 | task 20.26 'SDK OAuth：GitHub Copilot' (parent row DEL-09; scenarios: P20-S26) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.27 | task 20.27 'SDK OAuth：OpenRouter' (parent row DEL-09; scenarios: P20-S27) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.28 | task 20.28 'SDK OAuth：Kimi Coding' (parent row DEL-09; scenarios: P20-S28) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.29 | task 20.29 'SDK OAuth：Meta' (parent row DEL-09; scenarios: P20-S29) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.30 | task 20.30 'SDK OAuth：xAI' (parent row DEL-09; scenarios: P20-S30) | crates/opi-ai/tests/oauth_flows.rs: executed green, 193 passed / 0 failed (individually observed) | met | — |
| T20.31 | task 20.31 'Radius OAuth 与完整认证来源／九 flow 验收' (parent row DEL-09; scenarios: V-43, V-55, AIR-05... | crates/opi-ai/tests/auth_contracts.rs: executed green, 106 passed / 0 failed (individually observed) | met | — |
| T20.32 | task 20.32 '真实 hydration artifact 与离线 bundled catalog' (parent row DEL-10; scenarios: P20-S32) | crates/opi-ai/tests/catalog_artifact.rs: executed green (aggregate run) | met | — |
| T20.33 | task 20.33 'ModelsStore 原子 restore／refresh／personalization' (parent row DEL-10; scenarios: V-... | crates/opi-ai/tests/registry.rs: executed green, 28 passed / 0 failed (individually observed) | met | — |
| T20.34 | task 20.34 '42 profiles、目录／预算集成与 canonical 引用迁移' (parent row DEL-10; scenarios: V-44, AIR-052... | crates/opi-ai/tests/api_mapped_provider.rs: executed green, 17 passed / 0 failed (individually observed) | met | — |
| T20.35 | task 20.35 '3.5 estimator、effort／sampling／pricing 元数据' (parent row DEL-10; scenarios: V-14, V... | crates/opi-ai/tests/context_budget.rs: executed green, 11 passed / 0 failed (individually observed) | met | — |
| T20.36 | task 20.36 '完整聊天 adapter：openai-completions' (parent row DEL-11; scenarios: V-11, AIR-022, P2... | crates/opi-ai/tests/openai_chat_fixtures.rs: executed green, 99 passed / 0 failed (individually observed) | met | — |
| T20.37 | task 20.37 '完整聊天 adapter：openai-responses' (parent row DEL-11; scenarios: P20-S37, ONB-03-20.37) | crates/opi-ai/tests/openai_responses_fixtures.rs: executed green (aggregate run) | met | — |
| T20.38 | task 20.38 '完整聊天 adapter：azure-openai-responses + Azure Chat' (parent row DEL-11; scenarios: ... | crates/opi-ai/tests/azure_openai_fixtures.rs: executed green, 40 passed / 0 failed (individually observed) | met | — |
| T20.39 | task 20.39 '完整聊天 adapter：anthropic-messages' (parent row DEL-11; scenarios: P20-S39, ONB-03-2... | crates/opi-ai/tests/anthropic_fixtures.rs: executed green, 113 passed / 0 failed (individually observed) | met | — |
| T20.40 | task 20.40 '完整聊天 adapter：google-generative-ai' (parent row DEL-11; scenarios: P20-S40, ONB-03... | crates/opi-ai/tests/gemini_fixtures.rs: executed green, 70 passed / 0 failed (individually observed) | met | — |
| T20.41 | task 20.41 '完整聊天 adapter：google-vertex' (parent row DEL-11; scenarios: P20-S41, ONB-03-20.41) | crates/opi-ai/tests/vertex_fixtures.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.42 | task 20.42 '完整聊天 adapter：bedrock-converse-stream' (parent row DEL-11; scenarios: P20-S42, ONB... | crates/opi-ai/tests/bedrock_fixtures.rs: executed green, 176 passed / 0 failed (individually observed) | met | — |
| T20.43 | task 20.43 '完整聊天 adapter：mistral-conversations' (parent row DEL-11; scenarios: P20-S43, ONB-0... | crates/opi-ai/tests/mistral_fixtures.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.44 | task 20.44 '完整聊天 adapter：pi-messages' (parent row DEL-11; scenarios: P20-S44, ONB-03-20.44) | crates/opi-ai/tests/chat_pi_messages.rs: executed green, 36 passed / 0 failed (individually observed) | met | — |
| T20.45 | task 20.45 'typed OpenRouter image／ImageApi 公共路径' (parent row DEL-11; scenarios: V-23, AIR-02... | crates/opi-ai/tests/image_provider_serialization.rs: executed green, 15 passed / 0 failed (individually obs... | met | — |
| T20.46 | task 20.46 'typed classifier：typesafe-system-one' (parent row DEL-11; scenarios: P20-S46, ONB... | crates/opi-ai/tests/classifier_contract.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.47 | task 20.47 'typed classifier：cloudflare-workers-ai-system-one' (parent row DEL-11; scenarios:... | crates/opi-ai/tests/classifier_contract.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.48 | task 20.48 'typed classifier：llama-cpp-classify' (parent row DEL-11; scenarios: P20-S48, ONB-... | crates/opi-ai/tests/classifier_contract.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.49 | task 20.49 'OpenAI Decisions 与四 classifier 共用合同验收' (parent row DEL-11; scenarios: V-24, AIR-0... | crates/opi-ai/tests/classifier_contract.rs: executed green, 72 passed / 0 failed (individually observed) | met | — |
| T20.50 | task 20.50 'deferred handle／显式 fetch-cancel／可重开 consumer' (parent row DEL-11; scenarios: V-37... | crates/opi-ai/tests/deferred_contract.rs: executed green, 14 passed / 0 failed (individually observed) | met | — |
| T20.51 | task 20.51 'headers／payload／response／raw functional callbacks' (parent row DEL-11; scenarios:... | crates/opi-ai/tests/request_enrichment.rs: executed green, 32 passed / 0 failed (individually observed) | met | — |
| T20.52 | task 20.52 '显式 whole-call retry helper' (parent row DEL-11; scenarios: V-57, AIR-065, P20-S52) | crates/opi-ai/tests/whole_call_retry.rs: executed green, 18 passed / 0 failed (individually observed) | met | — |
| T20.53.1 | task 20.53.1 'text／system rendering 公共工具' (parent row DEL-11; scenarios: P20-S53-1) | crates/opi-ai/tests/public_utilities.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.53.2 | task 20.53.2 'JSON repair 与 portable schema 构造工具' (parent row DEL-11; scenarios: P20-S53-2) | crates/opi-ai/tests/public_utilities.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.53.3 | task 20.53.3 'diagnostic／overflow／Length 公共判别' (parent row DEL-11; scenarios: P20-S53-3) | crates/opi-ai/tests/public_utilities.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.53.4 | task 20.53.4 '字节身份、shortHash 与 UUIDv7 公共工具' (parent row DEL-11; scenarios: P20-S53-4, P20-S53-5) | crates/opi-ai/tests/public_utilities.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.53.6 | task 20.53.6 'headers、scoped env 与可取消等待公共工具' (parent row DEL-11; scenarios: P20-S53-6, P20-S5... | crates/opi-ai/tests/public_utilities.rs: executed green, 51 passed / 0 failed (individually observed) | met | — |
| T20.53.8 | task 20.53.8 'MockProvider scripted 多操作测试支撑' (parent row DEL-11; scenarios: (DoD-only)) | crates/opi-ai/tests/test_support_tests.rs: executed green, 23 passed / 0 failed (individually observed) | met | — |
| T20.54 | task 20.54 '安全 TelemetryContext parent＋Dispatch 载体' (parent row DEL-12; scenarios: (DoD-only)) | crates/opi-ai/tests/telemetry_carrier.rs: executed green, 10 passed / 0 failed (individually observed) | met | — |
| T20.55 | task 20.55 '所有真实 SDK 操作 telemetry 传播与被动性' (parent row DEL-12; scenarios: V-46, V-47, V-48, AI... | crates/opi-ai/tests/telemetry_operations.rs: executed green, 7 passed / 0 failed (individually observed) | met | — |
| T20.56 | task 20.56 '逐切片迁移后的旧符号／断言／历史 reader 退出审计' (parent row DEL-13; scenarios: V-10, V-20, AIR-021,... | retired shells zero in opi-ai production/test code (Config/ProviderConfig/config::Error/Model/ProviderKind/... | met | — |
| T20.57 | task 20.57 'workspace 外真实 package consumer 与完整 SDK 矩阵' (parent row DEL-14; scenarios: V-21, V... | external_sdk 10/10 green + read-only execution checker PASS (inventory 10, all executed, no ignored); examp... | met | — |
| T20.58 | task 20.58 '真实 native 平台／MSRV／feature 与供应链交付验收' (parent row DEL-14; scenarios: V-58, P20-S58,... | platform_delivery 9/9 green on this Linux MSRV host (frozen graph/features/checksums/build.rs/unsafe/proc-m... | met | P20-AUD-001 |
| T20.59 | task 20.59 '上游跟进／测试处置与交付证据基线' (parent row DEL-14; scenarios: V-59, AIR-067, P20-S59, P20-ACTO... | opi-doc-check PASS; upstream-tracking 2026-10-30 addendum + acceptance-matrix V-59 addendum verified agains... | met | P20-AUD-002 |

Independent execution evidence backing the `met` states (sealed export, WSL2 x86_64-unknown-linux-gnu, rustc/cargo **1.97.1 = the declared MSRV**, offline, per-worktree resolved target cache):

- `cargo test -p opi-ai --tests --offline` — **2629 passed / 0 failed / 0 ignored** across all 76 result lines (every registered opi-ai behavioral suite, including external_sdk, platform_delivery, oauth_flows 193, bedrock_fixtures 176, anthropic_fixtures 113, auth_contracts 106, schema_engine 65, ws corpus).
- `cargo test -p opi-ai --lib --offline` — 342/0/0 (private unit corpora: ws:: 136, schema IR/refs, stream, bedrock, anthropic, endpoint). `--doc` — 3/0/0.
- Caller suites: opi-agent `agent_loop_semantics` 22, `retry_agent` 14; opi-coding-agent `product_evidence` 29, `tool_authority` 15, `credential_store` 59, `oauth_auth` 124, `provider_factory` 57, `product_source_barrier` 3, `bedrock_provider_wiring` 8, `vertex_provider_wiring` 15, `interactive_startup_diagnostics` 3.
- Read-only libtest execution checker on external_sdk: PASS (inventory 10 = executed 10, identities/counts consistent; inputs hashed).
- `cargo check -p opi-coding-agent` green (necessary workspace callers compile); `cargo fmt --check --all` clean; `cargo clippy -p opi-ai --all-targets -- -D warnings` clean; `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-ai --no-deps` clean; `python scripts/opi-doc-check.py` PASS.

Recorded limitations (evidence class not independently reproducible in this environment, disclosed per the raw-proof preflight): the 2026-10-30 dual-host packaging-harness runs (245/245 byte-identical `.crate` members, `package_slice --family ws`, README runners) and `opi-impl-smoke` full were inspected as recorded delivery evidence and corroborated by the independently executed equivalents above, but were not re-executed by this peer. macOS/aarch64 native execution and CI dispatch are unexecuted everywhere and remain registered OPEN (P20-AUD-001).

## Standards Review

- **Workspace topology/manifests**: 7 members, resolver 2, edition 2024, rust-version 1.97, lockstep 0.8.3 path+version internal edges; `opi-ai` has no internal dependency; `opi-sandbox -> opi-protocol` only; asserted by platform_delivery (9/9) and re-verified statically. Pinned backend closure matches the registered contract (reqwest 0.12.28, rustls 0.23.40 + aws-lc-rs 1.18.2/aws-lc-sys via prebuilt-nasm, libz-rs-sys =0.6.8/flate2 =1.1.10 zlib-rs, regress =0.12.0 std/utf16/prohibit-unsafe, ryu-js =1.0.2, url 2.5.8, smithy query/xml =0.62.1, xmlparser =0.13.5, p256 =0.13.2).
- **Retirement discipline (AIR-021/DEL-13)**: retired shells are absent from opi-ai production and tests; ApiKind retained for persisted consumers; legacy in-band `Error` events converge to one `Err(CallFailure)` terminal; CHANGELOG Unreleased records every scoped 0.x break; bilingual README reorganized onboarding-first with three compiling examples embedded verbatim.
- **Cargo cache workflow honored**: per-worktree cache resolved via `scripts/opi-cargo-cache.py resolve`; no `cargo clean`, no session targets, no cache deletion.

## Spec Review

- **Public SDK contract (AIR-045–049)**: `Models` exposes `stream/complete/stream_simple/complete_simple` (+ direct variants), `get_model`, `login/logout/check_auth/get_auth/refresh`, `prepare_call`, `cleanup_session_resources(Some(nonempty)/None/Some("")->typed EmptySessionId)`, `stream_deferred`; `result(self)` terminal mapping verified (Stop/Length/ToolUse/Deferred→Ok, Error/Aborted→Err with complete partial/usage); duration_ms preserved through serde.
- **Backend profiles (RDF-21..24)**: ws frame/pmd/io are `pub(crate)` under `src/ws/` with the public `opi_ai::ws` Upgrade API preserved and a real production consumer (`openai_codex_responses/ws_transport.rs`); telemetry is a small owned module (`from_span/noop/in_scope/instrument`), no exporter/global subscriber; mutation barrier has no default wait deadline; schema validation uses the private TypeBox-dialect evaluator (regress/url/ryu-js) with no jsonschema on the default path.
- **Coverage groups**: the six P20-GAP mechanism groups and P20-GAP-DELIVERY remain OPEN exactly as registered, owned by F.1a; the delivery evidence keeps executed vs missing outcomes separated and honest.

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: zero-I/O construction/registration/static reads asserted by executed tests (external_sdk `empty_models_registers_no_provider_and_static_reads_do_no_io`); protected identity headers fail typed; secret canaries across oauth wire/diagnostics/proxy suites green; SigV4 header-whitespace vectors in lib tests; fail-closed schema and typed redacted failures exercised by schema_engine/schema_public/provider_diagnostics.
- **Invariants**: bounded queue/backpressure, terminal absorb/permanent EOF, cancellation-before-send, frozen auth/model per logical call, atomic catalog publication with stale-write fencing, store-owned serialized rotation with honest pending/unknown receipts — all covered by green suites (stream/provider_lifecycle/provider_collection/catalog_*/oauth_store_integration/resource_cleanup).
- **Test quality (anti-vacuity spot checks)**: discriminating assertions observed in external_sdk (callback once-per-variant counts, protected-header typed failure, >64 lawful collection, zero-read construction counter), whole_call_retry (18 tests on classification/delay/cancel), retry_backoff (57 incl. hint/quota), resource_cleanup (16 incl. namespace/empty-id/cutoff). No ignored/gated tests were counted as evidence (0 ignored everywhere).
- **Residuals**: no `companions/` dir; no test-only features (platform_delivery asserts the single feature union); no member proc-macro crates; unsafe inventory frozen by test; the historical Phase-12 breadth guard in `tool_result_wire.rs` was deliberately retained-and-updated (documented in-file) rather than deleted, with the formal graph/boundary checks now owned by platform_delivery — permitted, and noted as a residual guard.

## Minimum-change Conformance

- `reuse_search`, `placement`, `surface_necessity`, `simplification_ceiling`, `forbidden_scope` are recorded for all 68 tasks (ledger inference notes, read via the scope reader) — **conforming**; spot-verified against current code (README/examples/suites exist as claimed; no second facade: `provider_collection` is builtin assembly over `Models`).
- Introduced public seams trace to production consumers or registered SDK-caller ownership: `Models` methods (consumed by workspace callers and external_sdk), `ws::Upgrade` (consumed by codex ws_transport), `retry_assistant_call`/`cleanup_session_resources`/`TelemetryContext` are host/SDK-caller seams by design (AIR-065/035/054 require no Agent/session); their absence from host crates is recorded, not a defect (`nonproduction_consumers`: test suites + examples). No `net_deletion` regression found beyond the registered scoped breaks (CHANGELOG-documented).

## Findings

### P20-AUD-001: Native macOS/aarch64 execution and CI dispatch remain OPEN, owned by F.1a

- Axis: spec | Severity: Info | Conformance effect: advisory
- Requirement IDs: T20.58
- Claim: at audit_head no native macOS/aarch64 run and no CI dispatch executed; the contract itself records these as OPEN and assigns DEL-14 closure to F.1a. This is a registered deferral, not an implementation defect; recorded so set consumers see the residual phase-exit blocker.
- Evidence: phase20-delivery-evidence.md macOS row ("No owning host — OPEN … blocks Phase exit until a runner executes"), "no dispatch executed" CI section; platform_delivery asserts configuration identity only.
- Refutation attempted: verified whether any darwin execution claim exists anywhere in the evidence baseline — none does; the deferral is explicitly registered (P20-S58), so this is not a missing requirement.
- Suggested closure: an owning macOS/aarch64 runner (or explicitly authorized CI) executes the registered platform/MSRV/conformance matrix and F.1a judges DEL-14.

### P20-AUD-002: V-59 phase tally records delete = 0 while a scoped assertion deletion occurred

- Axis: standards | Severity: Minor | Conformance effect: advisory
- Requirement IDs: T20.59
- Claim: the 2026-10-30 V-59 addendum's delivery-group tally says `delete = 0`, but task 20.56 deleted the constructor-only `provider_kind_variants` assertion (CHANGELOG-documented, with green typed replacements). Under the matrix's assertion-level disposition convention the phase-level delete count is ≥1; the tally is accurate only when read file-level or as task-20.59-own impact.
- Evidence: acceptance-matrix V-59 addendum tally vs CHANGELOG Unreleased; `provider_kind_variants` absent from current crates/ (grep clean).
- Refutation attempted: checked the file-level reading (no test files deleted — all 41+7 files still present) and the per-round convention ("本轮" tallies elsewhere); the ambiguity remains only for the phase-labeled line.
- Suggested closure: relabel the tally (file-level or task-scoped) or record the assertion-level delete count.

## Context boundary deviation (disclosure)

While verifying P20-AUD-002, this peer ran `grep -rn provider_kind_variants` across the export and one hit returned from the phase ledger file (`docs/snapshots/phase20/opi-impl-state.json`, a task-disposition line). Ledgers are reader-only inputs; direct ledger search was a boundary deviation. The exposed line only **corroborated** a fact already established from two permitted sources (CHANGELOG.md, acceptance-matrix.md) and current code; no prior audit/remediation conclusion, sibling output, or history content was exposed, and no sealed requirement, state, or finding derives from it. The line is excluded from the finding's evidence. The coordinating operator may treat this disclosure as grounds to supersede this member with a fresh-context peer.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| python3 .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 20 | PASS (head 6adacc379df1, 68 tasks, 0 hash mismatches) | baseline |
| python3 .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase20 | PASS | admission |
| cargo test -p opi-ai --tests --offline | PASS 2629/0/0 | T20.* behavioral suites |
| cargo test -p opi-ai --lib --offline | PASS 342/0/0 | T20.9/13/15/16 unit corpora |
| cargo test -p opi-ai --doc --offline | PASS 3/0/0 | docs |
| cargo test -p opi-agent --test agent_loop_semantics / retry_agent | PASS 22 / 14 | T20.4/20.6 |
| cargo test -p opi-coding-agent --test <9 caller suites> | PASS 313 aggregate | T20.4/19.*/20.22/20.31/20.34 |
| check_test_execution.py (external_sdk list/ignored/run) | PASS (10 executed, consistent) | T20.57 |
| cargo check -p opi-coding-agent --offline | PASS | T20.56 caller migration |
| cargo fmt --check --all | PASS | standards |
| cargo clippy -p opi-ai --all-targets -- -D warnings | PASS | standards |
| RUSTDOCFLAGS=-D warnings cargo doc -p opi-ai --no-deps | PASS | standards |
| python3 scripts/opi-doc-check.py | PASS | T20.1/20.59, bilingual lockstep |
| git merge-base --is-ancestor 6adacc379df15ff2e5fa66ddb2afd3abeef5bb89 HEAD | PASS | head validity |
| packaging harness (package_slice/run_readme_examples/run_early_package_proof) | NOT RUN (recorded dual-host evidence inspected) | T20.57/20.58 limitation |
| native macOS/aarch64; CI dispatch | NOT RUN (registered OPEN, F.1a) | P20-AUD-001 |

## Verdict Rationale

All 68 mandatory task-level requirements are `met` on current `audit_head` evidence independently executed by this peer (≈2,971 opi-ai tests + 349 caller tests + contract/lint/doc gates, zero failures, on the declared MSRV toolchain), with recorded limitations confined to evidence classes this environment cannot reproduce (packaging harness re-execution, macOS/aarch64 native, CI dispatch) that are themselves registered OPEN and owned by F.1a. The two findings are advisory (Info, Minor); no Blocker/Major finding survives refutation. Member verdict: **PASS-WITH-FINDINGS** (mechanical: all mandatory met, one non-Info advisory finding).

<!-- opi-audit-history/1
{
  "audit\u005fhead": "6adacc379df15ff2e5fa66ddb2afd3abeef5bb89",
  "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
  "comparison": {
    "comparisons": [],
    "limitations": "None found within the inspected scope: the absence claim is bounded to this Phase's assurance set at installation time (empty set, no history directory).",
    "schema": "opi-audit-history/1",
    "scope": "No superseded same-pair run: this peer's complete installed the first member of the docs/snapshots/phase20 assurance set (revision 1, no prior index, no assurance/history/ directory, no replaced\u005frun\u005fid in the installation journal). The (reviewer=pi, model=glm53) pair had no prior run in this set; no other history records were inspected.",
    "sources": [],
    "status": "no-history"
  },
  "core": {
    "findings\u005fsha256": "fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a",
    "meta\u005fsha256": "7d449f0419ad10484a993b43822e7aa0b9c015a172b1e1134e0e31dcdfb67a4a",
    "report\u005fbytes": 32850,
    "report\u005fsha256": "45860afef2d943c0666db941777ca961c642ac12d55a6c6091e0c17110491fdc",
    "requirements\u005fsha256": "3f790100e39e3164bc8248d20b84d273640e41978d66dcc81875f81705898b5d"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z
- Audit head: 6adacc379df15ff2e5fa66ddb2afd3abeef5bb89
- Core report bytes: 32850
- Core report SHA-256: 45860afef2d943c0666db941777ca961c642ac12d55a6c6091e0c17110491fdc
- Current findings SHA-256: fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a
- Comparison status: no&#45;history
- Inspected history: No superseded same&#45;pair run: this peer&#39;s complete installed the first member of the docs/snapshots/phase20 assurance set &#40;revision 1, no prior index, no assurance/history/ directory, no replaced&#95;run&#95;id in the installation journal&#41;&#46; The &#40;reviewer=pi, model=glm53&#41; pair had no prior run in this set; no other history records were inspected&#46;
- Limitations: None found within the inspected scope: the absence claim is bounded to this Phase&#39;s assurance set at installation time &#40;empty set, no history directory&#41;&#46;

### Pinned historical sources

No relevant historical source found within the stated scope.
