# Phase 20 delivery-input baseline (frozen)

> **REGISTERED PHASE 20 DELIVERY-INPUT EVIDENCE — frozen by task 20.1 (DEL-01), 2026-10-17.** This page freezes the Pi tree/runtime/dependency/data identities, the current Opi HEAD, the full AIR/V identity disposition, the published capability inventory with its explicit exclusions, and the source-to-Rust test-mapping inputs behind the Phase 20 task graph. It is non-normative evidence: the bilingual delivery source ([spec-draft.md](spec-draft.md) / [spec-draft.zh.md](spec-draft.zh.md)) plus its adopted annexes own every contract, and [acceptance-matrix.md](acceptance-matrix.md) is the sole acceptance authority. Nothing here adds or amends a requirement, claims runtime support, or records an executed acceptance result. The Chinese counterpart is [phase20-delivery-inputs.zh.md](phase20-delivery-inputs.zh.md).

## 1. Frozen Pi 1.3.0 source identity

| Item | Frozen value |
|---|---|
| Source path | `.repo/pi-1.3.0` (local snapshot; `.git` absent — not an upstream tag/npm identity proof) |
| Tree SHA-256 | `08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2` (1925 files) |
| AI package | `@earendil-works/pi-ai` 1.3.0 — 396 files, 199 `src` TS files, 174 test files |
| Telemetry package | `@earendil-works/pi-telemetry` 1.3.0 — 12 files, 6 `src` TS files, 2 test files |
| Precursor snapshot | `.repo/pi-1.2.0`, tree SHA-256 `a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10` (1902 files; ai 394/200/171, telemetry 12/6/2) |
| Whole-tree delta 1.2.0→1.3.0 | 30 added / 192 modified / 7 removed / 1703 unchanged; ai package +4/41/2, telemetry +0/2/0 |
| Hash method | SHA-256 over the UTF-8 concatenation of POSIX relative paths sorted by Unicode code point, each `path + NUL + file SHA-256 hex + LF`; excludes `.git`, `node_modules`, `dist`; symlink policy reject-all |
| Generated catalog data | `packages/ai/src/providers/data/*.json` absent in both snapshots; not hydrated, so no generated catalog record identity is claimed |
| Version resolution | The user wrote “1.12.0→1.13.0” alongside `.repo/pi-1.3.0`; the actual package manifests and available directories establish 1.2.0→1.3.0 |

Source: [pi-1.3.0-source-evidence.json](pi-1.3.0-source-evidence.json) (recorded 2026-10-16 with Opi input `c5ab42ab8888b93f9969652323caee87b7550392`); the tool and per-path deltas live there.

## 2. Reference runtime identities (offline research harness only)

| Runtime | Frozen identity |
|---|---|
| Node | 24.13.0 with built-in Undici 7.18.2 |
| Bun | 1.3.5 `+1e86cebd7`, fixed by exe SHA and full version output |
| Schema experiments | TypeBox 1.3.27 and native `jsonschema` 0.28.3, each with an independent dependency lock graph |
| WS experiments | TW/TTW 0.28.0 with an independent lock graph (disqualified as the complete parity choice; §3.2 selects the replacement) |

These pin the runtimes under which the offline differential observations in §9.4 were produced. They are research-harness identities, not product runtime claims. Sources: [upstream-tracking.md](upstream-tracking.md), [pi-1.3.0-schema-ws-offline-validation.md](pi-1.3.0-schema-ws-offline-validation.md).

## 3. Dependency identities

### 3.1 Pi `@earendil-works/pi-ai` 1.3.0 package set

| Kind | Dependencies |
|---|---|
| runtime | `@anthropic-ai/sdk` 0.129.0, `@aws-sdk/client-bedrock-runtime` 3.1127.0, `@earendil-works/pi-telemetry` 1.3.0, `@google/genai` 2.21.0, `@smithy/node-http-handler` 4.12.1, `http-proxy-agent` 9.1.0, `https-proxy-agent` 9.1.0, `openai` 7.19.0, `partial-json` 0.1.7, `typebox` 1.3.27 |
| dev | `@types/node` 22.19.19, `canvas` 3.2.3, `vitest` 4.1.11 |

The telemetry package declares no dependencies. `openai@7.19.0` was verified per-package SHA-512 integrity with archive SHA-256 `597532dcb9051ee8aa5edadedb57b798b42add68cc5aec1d67e994d0cfad07f1` and official tag commit `15f6e408a0359df50ee7a996616957cc14988659` (appendix A evidence in [sdk-target-proposal.md](sdk-target-proposal.md) and [sdk-auth-provider-migration.md](sdk-auth-provider-migration.md)).

### 3.2 Design-selected Rust backend set (selection, not integration acceptance)

| Group | Selected identity |
|---|---|
| HTTP | unmodified crates.io `reqwest` 0.12.28 + `Hyper` 1.9.0, with the declared RDF-22 HTTP parser/storage acceptance profile; one explicit transport/retry owner; no HTTP fork/namespace obligation |
| TLS / ADC signing | `rustls` 0.23.40 / `ring` 0.17.14 with instance trust/mTLS and explicit ALPN; native default is the pinned Node reference CA bundle; ADC RS256 and source-required signing/verification/KX use a private OpenSSL 3 `LibCtx` (built-in default provider, memory-only EVP C APIs) behind a narrow safe Rust owner through rustls public seams |
| WS | owned strict Upgrade (dynamic headers/tail) + controlled/patched Ratchet `ratchet_core`/`ratchet_ext`/deflate 1.2.1 with the public PMD message-state adapter and pinned finite frame RSV/length/checked-storage correction; TW/TTW 0.28.0 exits the complete target |
| AWS / ADC source protocols | Smithy Query/XML 0.62.1, `xmlparser` 0.13.5, `p256` 0.13.2, `sha1` 0.10.6, existing `hmac`/`sha2` |
| Schema | private TypeBox JSON-dialect IR/evaluator + `regress` 0.12.0 algorithm with reproducible Unicode16 data, `serde_json` preserve_order, explicit JS key enumeration, `ryu-js` 1.0.2 |
| Telemetry | `tracing` 0.1.44 / `tracing-core` 0.1.36 |

These are selected design identities with declared patch/source/data provenance, adopted through the Phase delivery contract. HTTP amendment (2026-10-19): the HTTP row follows RDF-22; ordinary reqwest/Hyper already occur in the product Cargo graph, while the complete new profile, source-owner assembly and packaged-consumer behavior still require implementation acceptance. This selection makes no new compilation claim. Other fixed input identities and historical counts retain their original evidence window; formal admission remains a separate gate ([spec-draft.md](spec-draft.md) backend annex; [pi-1.3.0-design-readiness.md](pi-1.3.0-design-readiness.md)).

## 4. Data identities

- Generated Unicode16 regex data for the `regress` 0.12.0 algorithm; regenerated reproducibly (64/64 fixed instances and 11 synthetic error controls produced identical results on two generations).
- Private Unicode16 URL/IDNA/ICU family identities: 10 package namespaces sharing the ordinary ICU 2.2 common graph without cross-contamination; `ryu-js` 1.0.2 checked at 14 and 29 points (43/43 leaf).
- Pinned Node reference CA bundle as the native TLS default (scoped/explicit roots are source-owned; host roots opt-in).
- Catalog protocol data: distribution schema v1 and AI hydration data v6, with the catalog minimum Pi version 1.2.1 recorded separately; a Rust crate version is not a Pi compatibility identity.
- Unhydrated provider data: `packages/ai/src/providers/data/*.json` absent in both frozen snapshots; no bundled catalog record identity is claimed.

## 5. Current Opi HEAD and prior evidence-window inputs

Frozen at task 20.1 start: HEAD `96b97096226d0b9ef59c82c9eb6bb67f8ed9da2f`, branch `main` (ahead 15 of `origin/main`), clean worktree, Phase 20 task graph confirmed 2026-10-17T01:59:31+08:00.

| Evidence window | Opi input commit |
|---|---|
| Pi 1.0.0 baseline | `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73` |
| Pi 1.2.0 review | `e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8` |
| 1.2.0 Rust engineering revision | `6b81658` |
| Parameter revision and Pi 1.2.0→1.3.0 upgrade review | `c5ab42ab8888b93f9969652323caee87b7550392` |
| Backend gap audit | `a37ed70` |
| Schema/WS offline experiments and design readiness | `8cb0fd68db528ae3d3c61874458d45e46bffa29c` |

Historical windows keep their original inputs; only the current-HEAD row is the Phase 20 delivery baseline.

## 6. AIR identity disposition — 67 = 57 active SDK + 9 host-only + 1 retired

- **Active SDK (57):** AIR-001–AIR-008, AIR-010–AIR-014, AIR-020–AIR-036, AIR-038, AIR-040, AIR-042–AIR-059, AIR-061–AIR-067.
- **Host-only impact (9):** AIR-009, AIR-015, AIR-016, AIR-017, AIR-018, AIR-019, AIR-037, AIR-039, AIR-041.
- **Retired (1):** AIR-060 (AssistantMessageFrame encoder/reducer removed upstream with Pi 1.3.0; retirement is not a passed gate and creates no new obligation).

Identity rules: numbers are never recycled; host-only rows keep historical causation and downstream impact and never count as SDK completion or SDK failure; mixed groups contribute only their SDK sub-boundary. Enumeration authority: [spec-draft.md](spec-draft.md) and the AIR→V coverage table in [acceptance-matrix.md](acceptance-matrix.md).

## 7. V identity disposition — 59 = 54 active SDK + 4 host-only + 1 retired

- **Active SDK (54):** V-01–V-16, V-20–V-32, V-34–V-51, V-53–V-59.
- **Host-only impact (4):** V-17, V-18, V-19, V-33.
- **Retired (1):** V-52 (retired with AIR-060; ordinary stream events, immutable event-time partials and the final Result remain under V-40/V-41).

The same non-recycling and sub-boundary rules apply. None of the 54 active groups is implemented or executed by this freeze.

## 8. Published capability inventory and exclusions

### 8.1 Ten chat APIs

`openai-completions`, `mistral-conversations` (actual `POST /v1/chat/completions`), `openai-responses`, `azure-openai-responses` (provider `azure`), `openai-codex-responses`, `anthropic-messages`, `bedrock-converse-stream`, `google-generative-ai`, `google-vertex`, `pi-messages`.

### 8.2 One image API

`openrouter-images` — one non-streaming Chat-wire; first-choice text then valid data-URL images; no URL fetch, pixel decode or file writes.

### 8.3 Four classifier APIs

`typesafe-system-one` (bool→`noul`), `cloudflare-workers-ai-system-one` (Cloudflare `/run` envelope), `llama-cpp-classify` (local tokenize/apply-template/completion algorithm; no builtin factory — explicit custom-provider reuse), `openai-decisions` (bool→predicate; API-key-only availability; 504 never retried).

### 8.4 Forty-two builtin factory profiles

| Provider ID | Registered wire(s) |
|---|---|
| `amazon-bedrock` | bedrock-converse-stream |
| `ant-ling` | openai-completions |
| `anthropic` | anthropic-messages |
| `azure` | azure-openai-responses + openai-completions |
| `baseten` | openai-completions |
| `cerebras` | openai-completions |
| `cloudflare-ai-gateway` | anthropic-messages + openai-completions + openai-responses |
| `cloudflare-workers-ai` | openai-completions + cloudflare-workers-ai-system-one |
| `deepseek` | openai-completions |
| `fireworks` | anthropic-messages + openai-completions |
| `github-copilot` | anthropic-messages + openai-completions + openai-responses |
| `google` | google-generative-ai |
| `google-vertex` | google-vertex |
| `groq` | openai-completions |
| `huggingface` | openai-completions |
| `kimi-coding` | openai-completions (`/coding/v1`) |
| `meta` | openai-responses |
| `minimax` | anthropic-messages |
| `minimax-cn` | anthropic-messages |
| `mistral` | mistral-conversations |
| `moonshotai` | openai-completions |
| `moonshotai-cn` | openai-completions |
| `nvidia` | openai-completions |
| `openai` | openai-responses (default) + openai-completions (explicit) + openai-decisions |
| `openai-codex` | openai-codex-responses |
| `opencode` | anthropic-messages + google-generative-ai + openai-completions + openai-responses + typesafe-system-one |
| `opencode-go` | anthropic-messages + google-generative-ai + openai-completions + openai-responses |
| `openrouter` | anthropic-messages + openai-completions + openrouter-images + typesafe-system-one |
| `qwen-token-plan` | openai-completions |
| `qwen-token-plan-cn` | openai-completions |
| `qwen-token-plan-individual` | openai-completions |
| `radius` | pi-messages |
| `together` | openai-completions |
| `typesafe` | typesafe-system-one (classifier-only) |
| `vercel-ai-gateway` | anthropic-messages (+ System One via model baseUrl) |
| `xai` | openai-responses |
| `xiaomi` | openai-completions |
| `xiaomi-token-plan-ams` | openai-completions |
| `xiaomi-token-plan-cn` | openai-completions |
| `xiaomi-token-plan-sgp` | openai-completions |
| `zai` | openai-completions |
| `zai-coding-cn` | openai-completions |

Enumeration source: [pi-1.3.0-sdk-contracts.md](pi-1.3.0-sdk-contracts.md) §3 and the profile table in [acceptance-matrix.md](acceptance-matrix.md); shared wires do not erase provider-specific auth/default/filter behavior, and a factory name alone is not delivery.

### 8.5 Nine OAuth flow families

Anthropic, OpenAI Codex legacy, OpenAI ChatGPT (direct), GitHub Copilot, OpenRouter, Kimi Coding, Meta, xAI, Radius.

### 8.6 Explicit exclusions

- A dedicated OpenRouter Image API and image streaming are not delivered Pi 1.3.0 adapters and are not invented parity requirements; `openrouter-images` is the sole image surface.
- Responses Lite, a real remote-deferred protocol, exporter/automatic domain-span schemas, and speculative generic context/operation/plugin/transport frameworks stay outside the candidate until reopened by the registered source.
- AssistantMessageFrame and the compact-progress API are retired (AIR-060 / V-52); no parity obligation exists.
- Agent execution/authorization/hook policy, durable session v3/child migration/compaction, product tool_search/UI/startup defaults, and full MCP/codemode or virtual routing products remain host-owned (AIR-009/015–019/037/039/041; V-17–19/V-33 trace impact only).
- TS/Bun/browser/CLI substitutions are bounded platform differences recorded in [pi-1.3.0-rust-differences.md](pi-1.3.0-rust-differences.md), not new runtime targets.
- No hydrated/generated model-catalog data identity is claimed while provider data is absent from the frozen snapshot.

## 9. Source-to-Rust test-mapping inputs (no runtime claim)

### 9.1 Rust disposition inventory (inputs)

41 integration files under `crates/opi-ai/tests/`: `anthropic_fixtures.rs`, `anthropic_thinking_roundtrip.rs`, `api_mapped_provider.rs`, `auth_contracts.rs`, `azure_openai_fixtures.rs`, `bedrock_fixtures.rs`, `connection_pooling.rs`, `context_budget.rs`, `custom_provider_registration.rs`, `gemini_fixtures.rs`, `gemini_lifecycle.rs`, `image_input.rs`, `image_provider_serialization.rs`, `mistral_fixtures.rs`, `model_capabilities_migration.rs`, `model_wire_metadata.rs`, `oauth_wire_shape.rs`, `openai_chat_fixtures.rs`, `openai_codex_responses.rs`, `openai_compat_lifecycle.rs`, `openai_responses_fixtures.rs`, `openai_responses_lifecycle.rs`, `openrouter_fixtures.rs`, `output_content_image.rs`, `per_request_auth.rs`, `phase14_remediation.rs`, `provider_collection.rs`, `provider_diagnostics.rs`, `provider_error_classes.rs`, `provider_lifecycle.rs`, `provider_trait.rs`, `proxy_support.rs`, `registry.rs`, `request_enrichment.rs`, `retry_backoff.rs`, `stream_events.rs`, `test_support_tests.rs`, `tool_result_no_leak.rs`, `tool_result_wire.rs`, `usage_cost.rs`, `vertex_fixtures.rs`.

This is the original delivery-input inventory, not a current file count. The remediation tests listed above have since been merged into `anthropic_fixtures.rs`, `openai_chat_fixtures.rs`, `model_wire_metadata.rs`, `api_mapped_provider.rs`, `provider_collection.rs` and `usage_cost.rs`; use the current links in [acceptance-matrix.md](acceptance-matrix.md).

7 src files with `cfg(test)` units: `src/anthropic.rs`, `src/endpoint.rs`, `src/openai_responses_shared.rs`, `src/stream.rs`, `src/bedrock/credentials.rs`, `src/bedrock/event_stream.rs`, `src/bedrock/sigv4.rs`.

The per-assertion `add/update/delete/retain` dispositions for these 48 files live in [acceptance-matrix.md](acceptance-matrix.md) (“2026-10-16 测试迁移补核”). They are future recommendations tied to contract approval and implementation change; task 20.1 executes none of them and deletes nothing.

### 9.2 Pi test-corpus delta (1.2.0 → 1.3.0)

AI package 171 → 174 `.test.ts` files: 4 added (`json-parse`, `anthropic-error-tool-result-images`, `openai-responses-mid-convo-effort`, `transform-messages-cross-model-thinking`), 21 modified, 1 removed (`assistant-message-frame.test.ts`, retired with AIR-060/V-52); the 2 telemetry test files are unchanged in summary. File counts are not case counts, and no suite was executed.

### 9.3 Historical executed baseline (retained as history)

`cargo test --offline --locked -p opi-ai --test gemini_lifecycle --test provider_lifecycle --test provider_collection` → 4 + 8 + 58 = **70 passed, 0 failed** (external target dir; see [README](README.md)). This baseline predates the Phase 20 targets and proves nothing about them.

### 9.4 Executed offline-research observations (research harness, not product tests)

| Group | Frozen observation counts |
|---|---|
| Schema (first window) | 85 Pi public-utility cases; 276 native observations |
| Schema (design readiness) | 64/64 fixed instances + 11 synthetic error controls, generated twice with identical results; ryu-js 14-point and URL 29-point checks (43/43 leaf); 10 package namespaces + ordinary ICU 2.2 common graph |
| WS (first window) | 60 reference-constructor, 15 native-loopback, 16 reader, 15 partial-codec observations |
| WS (length/storage correction) | 14 pure-memory tests including 12 old codec fixtures; no socket/request executed |
| Transport/auth | 112 main records, 16 Pi-proxy observations, 20 header comparisons; ALPN/H2 and TLS negatives; sourced metadata timeouts (0/3000 ms) |
| Crypto | 11 RSA + 3 input negatives + poison = 15 observations; plus 3 P521/e3mTLS difference records |
| Telemetry | pinned safe poll/drop carrier across 7 scenarios |
| Catalog | 42×3 manual helper records distilled into 7 observations |

Sources: [pi-1.3.0-schema-ws-offline-validation.md](pi-1.3.0-schema-ws-offline-validation.md), [pi-1.3.0-design-readiness.md](pi-1.3.0-design-readiness.md), and the closing sections of [acceptance-matrix.md](acceptance-matrix.md). These observations justify design selections only; they are not product tests and close none of the seven coverage groups.

### 9.5 Boundary

Task 20.1 runs no runtime test and claims no runtime support. The inputs in this section feed the later DEL rows (binding/streams, context, transport, auth, operations, telemetry, migration); execution evidence belongs to those tasks and must be recorded against the sole acceptance matrix.
