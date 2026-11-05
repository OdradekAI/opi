# Phase 20 Remediation Plan

**Status**: DRAFT-UNRESOLVED
**Audit index SHA-256**: `e1b34739db46331707a93ab0f6f9336ed6386a4564b0215d8488ce288319f0ae`
**Remediation head**: `f5d9573eadbe71e746e2838099491989c8ff30c1`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: only the fixed plan pair was unstaged before this source rebind. The six workflow inputs below are committed at the remediation head. Staged and untracked inventories were empty.
**Workflow inputs**: [{"path":".agents/skills/README.md","sha256":"520eb923cc8f3d6073906fb9a26334a47a2c17916dda04368f71ea224caf5aa0"},{"path":".agents/skills/README.zh.md","sha256":"18f97693fa1bbb9011cf5783710598d64fd2747f903b5018301caeef3ae1598d"},{"path":".agents/skills/opi-remediate/SKILL.md","sha256":"ba2cb8a416b6935cf55536ef62d4fa507c21f9dcf1c3edf148d080327a15a13f"},{"path":".agents/skills/opi-remediate/references/remediation-plan-template.md","sha256":"bc2d13f420d7c31102b1227c94f245fc403e5f80423d8a924a25e5cfda5d0bbf"},{"path":".agents/skills/_shared/references/remediation-disposition-contract.md","sha256":"e15285bc35e02f0ab6d4d6637079d9eb649f1225175b43155248262a80d2830f"},{"path":".agents/skills/opi-remediate/references/execution-protocol.md","sha256":"33dd282c8b0c65c1defaec88228e805abb22a846cfe0c259d9ef5bdea779380b"}]
**Unresolved decisions**: D1, D2, D3

## Current Finding Verification

The current audit set validated before inspection. C and P identify the runs below. Every disposition binds the full source tuple. Source severities and reviewer requirement states are retained. Refutations below address the bounded current claim. The remaining linked delivery predicates are tracked separately.

| Alias | Indexed run | Findings SHA-256 | Keys |
|---|---|---|---:|
| C | `phase20-codex-gpt61-b3f1810-20261103t142820z` | `4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6` | 21 |
| P | `phase20-pi-glm53-44374ec-20261103t160703z` | `f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990` | 7 |

| Source / finding | Verification and current evidence | Severity | Closure key / family | Batches | Decision and residuals |
|---|---|---|---|---|---|
| C001 / P20-AUD-001 | Refuted. Scoped context overlays the stored map before candidate lookup and returns an explicitly empty entry before candidate filtering. Explicit-key selection remains earlier. `crates/opi-ai/src/auth/builtin.rs:50–73,173–221`. | Major retained | `credential.scoped-key-selection` / `auth.scoped-source` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C002 / P20-AUD-002 | Refuted. Both HTTP builders disable automatic Referer, and the default ADC transport uses those builders. A caller-supplied Referer remains caller-owned. `crates/opi-ai/src/http.rs:380–402`; `auth/adc/native.rs:350–379`. | Major retained | `adc.unsolicited-referer` / `http.redirect-audience` | [] | `no-action:refuted-current-source`. D1. |
| C003 / P20-AUD-003 | Refuted. Image failures use a status-based summary. Cloudflare envelope failures use the redacted summary projection. `crates/opi-ai/src/api/openrouter_images.rs:319–334`; `api/cloudflare.rs:173–215`. | Major retained | `nonchat.diagnostic-redaction` / `nonchat.diagnostics` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C004 / P20-AUD-004 | Refuted. The product classifier walks nested StreamFailure causes and matches all three credential variants. The wrapper retains typed failure context. `crates/opi-coding-agent/src/harness.rs:100–127`. | Major retained | `auth.prepared-failure-consumer` / `auth.preparation` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C005 / P20-AUD-005 | Refuted. A contradictory Done becomes one retained failure, and the collector absorbs the inner stream. `crates/opi-ai/src/stream.rs:853–873`. | Major retained | `stream.matching-stop-reasons` / `stream.terminal` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C006 / P20-AUD-006 | Refuted. The acquisition guard refuses a server wait above a nonzero cap before calculating a delay or admitting the send. The zero-cap sentinel remains source-owned. `crates/opi-ai/src/retry.rs:601–613,711–734`. | Major retained | `retry.server-hint-admission` / `retry.acquisition` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C007 / P20-AUD-007 | Refuted. The context overflow veto executes before the transient-pattern match. `crates/opi-ai/src/retry.rs:1017–1037`. | Major retained | `retry.context-overflow-veto` / `retry.whole-call` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C008 / P20-AUD-008 | Refuted. The numeric Claude branch adds answer and thinking before the model and context clamps. The separate zero-limit admission case still needs B_BUDGET. `crates/opi-ai/src/models.rs:3981–4016`. | Major retained | `simple.numeric-claude-addition` / `models.simple-budget` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C009 / P20-AUD-009 | Refuted. Typed compatibility fields round-trip and reach the Anthropic replay, cache, effort, header, fallback and cost paths. `crates/opi-ai/src/model_info.rs:349–412`; `catalog/wire.rs:182–201,623–659,983–1018`; `anthropic.rs:1407–1429,1526,1585,1677–1735,1917–1931,2566`. | Major retained | `anthropic.compat-branches` / `anthropic.compat` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C010 / P20-AUD-010 | Confirmed. The consolidated rehearsal includes all named ordinary source owners and the two real Codex HTTP paths. Source-specific public trust and additional native route combinations remain separate. | Major retained | `proxy.source-qualified-routes` / `http.source-proxy` | PXY_HTTP, PXY_HTTP_OWNERS, PXY_WS, PXY_HTTP_REST, PXY_AWS, PXY_ADC_SOURCE, B_ADC_REDIRECT, B_ADC_CONNECT | `pending:rehearse-source-qualified-proxy`. D1. |
| C011 / P20-AUD-011 | Refuted. The declaration collection uses a Vec with a numeric prefix and insertion-ordered ordinary keys. The Llama path consumes that order. `crates/opi-ai/src/classifier.rs:43–94,192,229`; `api/llama.rs:652,729–736`. | Major retained | `classifier.declaration-order` / `classifier.declarations` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C012 / P20-AUD-012 | Refuted. Both constructors install independent default stores. Login reads the selected store path. `crates/opi-ai/src/models.rs:632,685,2449–2467`. | Major retained | `models.default-memory-store` / `auth.store-default` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C013 / P20-AUD-013 | Refuted. The image request emits image plus text when text is present and image otherwise. Caller modality ordering and duplicates do not determine the wire order. `crates/opi-ai/src/api/openrouter_images.rs:174–178`. | Major retained | `images.source-modalities` / `images.wire` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C014 / P20-AUD-014 | Refuted. Image result parsing precedes the response callback. Failure updates the terminal projection while retaining output and usage. `crates/opi-ai/src/api/openrouter_images.rs:383–411`. | Major retained | `images.observer-retains-result` / `images.diagnostics` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C015 / P20-AUD-015 | Refuted. The numeric branch recognizes floating and signed zero before final validation. `crates/opi-ai/src/schema/normalize.rs:608–615`; `crates/opi-ai/tests/schema_public.rs:589,640`. | Major retained | `schema.float-zero-null` / `schema.coercion` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C016 / P20-AUD-016 | Refuted. The owning fixtures cover capability by per-tool intent and expect Chat affinity under disabled retention. `crates/opi-ai/tests/mistral_fixtures.rs:475–566`; `openrouter_fixtures.rs:429–519`; `request_enrichment.rs:417–439`. | Major retained | `chat.compat-fixture-expectations` / `chat.compat` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| C017 / P20-AUD-017 | Confirmed. The original CLI rejects the SDK family. The actual extracted consumer on the merged source passed all 48 ordinary and 48 caller-union cases with the reviewed dependency graph. | Major retained | `sdk.extracted-full-public-consumer` / `delivery.package` | B_SDK | `fix:execute-extracted-full-sdk-consumer`. Local planning proof complete. |
| C018 / P20-AUD-018 | Partially confirmed. A consolidated runtime and harness candidate now binds the selected verification union. The exact linked acceptance conjunction remains open. | Major retained | `delivery.complete-current-criterion-conjunction` / `delivery.evidence` | B_BUDGET, PXY_HTTP, PXY_HTTP_OWNERS, PXY_WS, PXY_HTTP_REST, PXY_AWS, PXY_ADC_SOURCE, B_ADC_REDIRECT, B_ADC_CONNECT, B_EXPORT, B_SDK, B_RECEIPT, B_SCHEMA, B_NATIVE | `pending:complete-current-criterion-conjunction`. D1, D2, D3. |
| C019 / P20-AUD-019 | Confirmed. Actual typed catalog/store red and 87-test candidate pass prove the chosen metadata repair and consumer paths. | Minor retained | `models.prompt-cache-durations-roundtrip` / `models.metadata` | B_CACHE | `fix:preserve-typed-prompt-cache-durations`. Local planning proof complete. |
| C020 / P20-AUD-020 | Confirmed. Actual OpenRouter and keychain red plus 688-test candidate pass prove the chosen lifetime migration and consumer paths. | Minor retained | `oauth.typed-permanent-lifetime` / `auth.expiry` | B_EXPIRY | `fix:typed-permanent-oauth-lifetime`. Local planning proof complete. |
| C021 / P20-AUD-021 | Refuted at current head. The registered source owner committed the below-cost wording in `8da7950`. | Minor retained | `bedrock.quota-bilingual-boundary` / `docs.quota` | [] | `no-action:corrected-by-registered-source-owner`. The source correction is recorded in the owner commit. |
| P001 / P20-AUD-001 | Refuted. The redirect response now advertises Connection: close to match the per-leg server lifecycle. Exact hop counts remain a runtime predicate. `crates/opi-ai/tests/http_acceptance.rs:382–385`. | Major retained | `http.redirect-test-peer-lifetime` / `http.redirects` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| P002 / P20-AUD-002 | Refuted. The current fixture cases distinguish capability from None, Prefer and Require tool intent. `crates/opi-ai/tests/mistral_fixtures.rs:475–566`; `openrouter_fixtures.rs:429–519`. | Major retained | `chat.tool-strict-fixture-intent` / `chat.compat` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| P003 / P20-AUD-003 | Refuted. The current Chat fixture expects affinity to remain under disabled retention. `crates/opi-ai/tests/request_enrichment.rs:419–438`. | Major retained | `chat.affinity-under-disabled-retention` / `chat.compat` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| P004 / P20-AUD-004 | Refuted. The product classifier recursively walks the wrapper, and interactive, RPC and runner entry points use it. `crates/opi-coding-agent/src/harness.rs:100–127`; `interactive.rs:766`; `rpc.rs:1100`; `runner.rs:840,882,919`. | Major retained | `auth.prepared-failure-consumer` / `auth.preparation` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| P005 / P20-AUD-005 | Refuted. The Mistral Conversations adapter defaults strict to false and retains the original schema in that branch. `crates/opi-ai/src/mistral.rs:447–464`. | Major retained | `mistral.unannotated-tool-wire` / `chat.compat` | [] | `no-action:refuted-current-source`. No distinct repair for this bounded claim. |
| P006 / P20-AUD-006 | Refuted at current head. The live acceptance-matrix pin matches current source. The frozen acceptance-matrix pin matches the source at archive commit `6adacc3`. | Minor retained | `ledger.current-source-pins` / `ledger.authority` | [] | `no-action:refuted-current-source`. The archived source identity remains historical. |
| P007 / P20-AUD-007 | Partially confirmed. The six-row native route and source/graph/material selectors are explicit. Actual native execution and complete family applicability remain open. | Major retained | `delivery.current-native-package-applicability` / `delivery.evidence` | B_NATIVE | `pending:prove-current-delivery-applicability`. D2, D3. |

## Unresolved Decisions

| ID | Required decision or proof | Current evidence and exit condition | Owner or authority |
|---|---|---|---|
| D1 | Finish source-specific route and trust controls | All eleven remaining adapter probes pass. The joined native-family certificate-file probe passed against the anonymous outer proxy and exact inner STS certificate with explicit fixture roots. Actual public builtin with the pinned default trust roots, Vertex ADC and Cloudflare placeholder proxy combinations, and WSS and Bedrock negotiated route controls require their remaining exact evidence. | Transport and downstream test owners. |
| D2 | Bind package and native acceptance to the verified source | The actual merged extracted SDK package passed 48 ordinary and 48 caller-feature-union cases from the same independent lock. Both profiles freshly compiled the extracted SDK and retained their exact test binaries and resolved RDF-21/HTTP graphs. [merged-sdk-proof-1/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/merged-sdk-proof-1/receipt.json>) (`7966d06ab182034fc19382eee901274823bf02e79ed58e9ebf79347444d98936`). Complete the distinct package families and feature/native material inventory. The six real host and toolchain runs use the verified checkpoint after explicit apply. | Package and CI execution owners. |
| D3 | Finish the linked delivery criteria | Existing public Models tests passed for 10s refresh plus 6s ACK, 20s prior lock wait, and 15s network timeout with no late commit. Confirmed receipt reclamation and retention under ordinary same-key success have concrete controls. A positive reconciliation requires authoritative evidence for the exact previous operation and resource. No such production entry point was found in the reviewed store, ADC or AWS queue paths. Complete remaining per-criterion selectors and delivery material. | Runtime, package and Phase exit owners. |

## Planning Evidence Work

- Committed source archive and worktree inventory: [contract-replan-20261105T064655Z-4f969318/baseline.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-replan-20261105T064655Z-4f969318/baseline.json>) (`f1b68845be0c70af00026f1a22df09dab9aad239b661e7fff20506a6fab50893`).
- Package CLI and cache owner/archive controls: [contract-replan-20261105T064655Z-4f969318/package-admission.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-replan-20261105T064655Z-4f969318/package-admission.json>) (`488d52f762d34edf830dd7eeeaec660605647e5c605ddc775caff2d1a1caa127`). The direct-owner check accepted and the archive check rejected before scratch. The `sdk` selector exited 2 before packaging.
- Earlier owner pin projection: [contract-replan-20261105T064655Z-4f969318/owner-pin-projection.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-replan-20261105T064655Z-4f969318/owner-pin-projection.json>) (`dbf4d96c5c3578163a42b48b56cca44a97ab15051823534cb043fd5527036eaa`). The projection compares stored pins with the then-current files. The archived acceptance-matrix pin matches the exact bytes at archive commit `6adacc3`.
- Exact current C018/P007 requirement projection: [contract-replan-20261105T064655Z-4f969318/linked-delivery-criteria.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-replan-20261105T064655Z-4f969318/linked-delivery-criteria.jsonl>) (`c57adab725fbab66b0a0e7afb54e212c77960fb9289f003930dd6ae7b9c133ac`). The 165 records represent 138 criterion identities, with 155 `not-assessable`, 9 `partially-met`, and one `not-met` in the source sidecars. The draft preserves those states and treats remaining proof separately.

The current-source counter-evidence in the verification table is static unless a receipt records execution. Runtime selectors retained for later criterion verification:

- C001: `builtin_auth_ownership / scoped_candidate_`.
- C002: `adc_owner_regressions / default_url_supplier_`.
- C003: `openrouter_fixtures / images_http_failure_diagnostics_do_not_echo_provider_body; classifier_contract / cloudflare_failure_diagnostics_do_not_echo_`.
- C004: `opi-coding-agent --lib / prepare_wrapped_; continuation_contract / public_retry_retains_initial_steering_after_auth_preparation_failure`.
- C005: `stream_events / chat_stream_rejects_mismatched_done_reasons_with_one_retained_failure`.
- C006: `provider_collection / helper_retry_hint_`.
- C007: `whole_call_retry / context_overflow`.
- C008: `anthropic_fixtures / simple_numeric_claude_adds_thinking_before_model_and_context_caps; context_budget / public_prepare_numeric_claude_`.
- C009: `anthropic_fixtures; model_wire_metadata; catalog_operations`.
- C011: `classifier_contract / llama_tied_choice_preserves_declared_first and llama_question_wire_sequence`.
- C012: `external_sdk / constructors_own_default_memory_without_a_runtime; oauth_flows / constructors_login_to_independent_default_memory_and_ack`.
- C013: `openrouter_fixtures / images_modalities_`.
- C014: `openrouter_fixtures / retains_received_result and unknown_usage_stays_unknown_after_failure`.
- C015: `schema_public / null_coercion_`.
- C016: `mistral_fixtures / mistral_profile_inherits_shared_compat_flags; openrouter_fixtures / openrouter_profile_inherits_shared_compat_flags; request_enrichment / session_affinity_wire_mappings`.
- P001: `http_acceptance / redirect budget acceptance`.
- P002: `mistral_fixtures; openrouter_fixtures`.
- P003: `request_enrichment / session_affinity_wire_mappings`.
- P004: `opi-coding-agent --lib / prepare_wrapped_; continuation_contract / public_retry_retains_initial_steering_after_auth_preparation_failure`.
- P005: `mistral_fixtures / mistral_profile_inherits_shared_compat_flags`.


### Continued rehearsal and owner baseline

- Workflow commit `065af6d0e82a5efe78a5c4eaf3979e9140a612a1` changes exactly the four then-bound workflow documents after the Chinese source owner commit. The indexed audit and rehearsal inputs are unchanged across that workflow commit. Trial receipts retain the source commits they actually executed.
- Workflow commit `f5d9573eadbe71e746e2838099491989c8ff30c1` changes only the skill documentation and prompt fixtures. The indexed audit and executable rehearsal inputs match the prior baseline.

- Source owner commit and exact one-path diff from the prior runtime baseline: [contract-continue-20261105T070028Z-405ed662/bedrock-owner-commit.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/bedrock-owner-commit.json>) (`975409e8f8b4ce920f0b724dcf98bfab08c3686154ae9d1ec692f98a5de1ecd4`). Trial receipts keep their actual recorded source identity, including `c6d38728934c30eff30ba9e5956a6fa84b0239ef` for pre-commit exports. This commit changes only the Chinese registered clause; production source, Cargo inputs, harnesses and indexed audit bytes match the trial baseline.
- Exact owner wording and boundary: [contract-continue-20261105T070028Z-405ed662/bedrock-quota-owner-handoff.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/bedrock-quota-owner-handoff.json>) (`b57d9a17da256eaf585117d24dbc359a70c88318e9d0713e2c88c2f5f7363539`). The archived Chinese-source pin matches the exact bytes at `6adacc3`.
- One combined invocation reported `opi_ai fresh=True` after a different export built the same cache output. Its results are excluded. Fresh rebuild evidence binds the isolated input mtimes and runtime source: [contract-continue-20261105T070028Z-405ed662/candidate-fresh-freshness-input.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/candidate-fresh-freshness-input.json>) (`ef1ea2b2e65cc0751a1929be13d80aadf43abbd589cb218d93b15be88c822efc`).
- All 165 source rows remain distinct. The selector leads are source-linked inspection inputs and do not establish full predicates: [contract-continue-20261105T070028Z-405ed662/delivery-selector-leads.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/delivery-selector-leads.jsonl>) (`adb284c94f2422da7e2f4fc2d7bd2f34793a15635496181e0f80e3c855eeb7e1`).
- Exact downstream adapter ownership and remaining path leads: [metadata-trials-8a36672b/d1-adapter-inventory.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/d1-adapter-inventory.md>) (`337bec026338e6fe4447f9a6a6187899d365c44bfbb7ddbc321db10519fd58dc`). The additional selected ordinary routes and ten remaining owner paths are in [ordinary-http-trials-f342aa50/route-evidence.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/ordinary-http-trials-f342aa50/route-evidence.md>) (`9a0eaedef1c38b066e57cd793ae8d32f43933b148081b15a91b3d10f4b2b11d9`).
- Pinned proxy refusal cases: [adc-connect-plan-68e2ca4f/gaxios-connect-refusal.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/gaxios-connect-refusal.jsonl>) (`b780a0492d3982a543524e1eac3da98fcbb687bd45e0826897e4a6bcb8bc041f`). This generic Gaxios probe configured two retries and observed one CONNECT for 407 and three for 502. The actual source family retains the default budget and is independently bound to one and four in B_ADC_CONNECT.
- The standalone outer TLS comparator independently observed reqwest offering the synthetic target certificate to the proxy. That separate anonymous-proxy candidate failed on the clear target with an immediately framed response. The B_ADC_CONNECT trials record a passing nested TLS identity witness and the separate framed-response correction: [https-green/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/https-green/receipt.json>) (`47bdc4d13a899d4cccc1191d770e2a14ded80a3493444e468bbe253ed87cf527`).
- Exact unchanged ordinary CONNECT dependency resolution: [adc-connect-plan-68e2ca4f/exact-graph-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/exact-graph-delta.json>) (`1fa2bc60dfc44697dcf5bec0f16506ada4416aabc11726605d6e986800f9f165`). The separate actual plain CONNECT candidate passed six controls and retained its expected HTTPS red: [plain-green-https-red/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/plain-green-https-red/receipt.json>) (`1ca4d3f44f792582257ad615ecf549ae55660858f3891a5ca694cf9a5a57a2b0`). Preserve inner/outer identity and integrate the same frozen source redirect owner.
- Pinned Gaxios source loopback output: [proxy-plan-h422v3ff/gaxios-probe.stdout.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/gaxios-probe.stdout.jsonl>) (`17d8689182a4f8616c8248d42a7bf8ac79f49e7bbfc2750903dc4d8f4a8bb51e`). Clear HTTP requires CONNECT under the ADC source contract.

### Consolidated contract rehearsal

- The consolidated rehearsal used the `3e6c257b1a317d8d2f54660e03f70b878e2ea4f2` source archive. Its executable source inputs match the remediation head. [delivery-runtime-join-aa9kjgkr/candidate-inputs.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/candidate-inputs.json>) (`accfa26712995403c353a5b1cd8f5f7cc169717a3db309268115776ca7313c0e`). The 86-path candidate and replayable patch bind exact bytes, including the narrow line-ending normalization and the reviewed archive/README merge. All 75 changed Rust files passed per-file rustfmt checks. [Format receipt](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/fmt-candidate.json>) (`f6f08e9fbaa688c804af61f367db190dc8721024ca127b004141985c6749ab70`). [delivery-runtime-join-aa9kjgkr/candidate-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/candidate-delta.json>) (`b851dd9a1daf1d2492b55e837ad0bedbb0345ca75a77bf817684faa3159bdc01`). [delivery-runtime-join-aa9kjgkr/candidate.patch](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/candidate.patch>) (`175e00769408c4b0ef1011d65afecd2bad8e0fdc85405b7bb11f61482b65f425`).
- The merged candidate rebuilt all 57 selected executables and ran 3,023 passing tests, with one existing Windows privilege-dependent test ignored. The two nested TLS control subprocesses are counted separately. [merged-runtime/bound-receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/merged-runtime/bound-receipt.json>) (`c0e21c3014ffb70280399c2a081ff0bd1caec2c84d99e15857ffa64f3ced1363`). The prior merged D1 run and its exact corrected fixture result retain 1,792 passing cases in 25 targets with one existing Windows privilege case ignored. [d1-combined-3e6c-f53nww66/verification-union.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/d1-combined-3e6c-f53nww66/verification-union.json>) (`cf8b4abf5f3f652269dc07c6525d723ce904bf511717776616c24a0fd46a8a70`).
- The joined native-family certificate-file probe passed against the anonymous outer proxy and exact inner STS certificate with explicit fixture roots. The existing private test input admits fixture roots through the same production ADC certificate-file and STS owner. Public builtin retains its fixed default trust, so the local CA witness has that boundary. [adc-certificate-composition-3e6c-ny_xfi4x/fixture-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-certificate-composition-3e6c-ny_xfi4x/fixture-delta.json>) (`3690da7d4b7987bb8918452c35dfef860d0fcde3e2e3a673143ac98bb5c8ea78`).
- Existing public Models tests passed for 10s refresh plus 6s ACK, 20s prior lock wait, and 15s network timeout with no late commit. Ordinary same-key success retains Unknown and Failed records. Positive authoritative reconciliation remains open. A plain key match, replacement ACK or current read alone does not establish a previous remote or durable effect.
- The actual merged extracted SDK package passed 48 ordinary and 48 caller-feature-union cases from the same independent lock. Both profiles freshly compiled the extracted SDK and retained their exact test binaries and resolved RDF-21/HTTP graphs. [merged-sdk-proof-1/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/merged-sdk-proof-1/receipt.json>) (`7966d06ab182034fc19382eee901274823bf02e79ed58e9ebf79347444d98936`). The two nine-row delivery selector sets retain their distinct indexed identities. [delivery-integrated-plan-4155b24d/delivery-criteria-selectors.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/delivery-criteria-selectors.json>) (`394eb8d4caab430381f2997827a17e2e22504547bfe3d1307cb6af4ed6407d5c`). Package, host and Phase exit evidence remain separate.

- All 165 linked criterion rows retain their source identities. 48 have named selector executions in this run. The remaining rows and each full predicate require separate review. [delivery-runtime-join-aa9kjgkr/criterion-observed-leads.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/criterion-observed-leads.jsonl>) (`7362e2f5b6bc94a9ac7647ce9d746c264443d77e76ad6b7c5ac6df70d172a749`).

## Owner Follow-up

The later Chinese-source change creates a live-ledger pin mismatch. The `opi-implement` owner can reconcile it separately through its source-alignment workflow. The archived pin and the current remediation finding each have their governing source identified.

## Closure Batches

### Batch B_BUDGET: Admit lawful zero limits and preserve the simple-call minimum

**Root change**: Admit zero model metadata and apply the registered minimum of one token to the zero-context simple-call output fallback.
**Closure predicate**: Public prepared calls send the source-defined zero-context simple output limit, while native omission, genuine invalid-input rejection and product configuration validation retain their predicates.
**Dependencies**: []
**Verification union**: Re-run the sixteen unfiltered affected binaries in the candidate receipt, the exact public wire case, and affected crate checks after implementation. Rehearse the separate numeric-thinking preflight and remaining wire selectors under D3 before sealing an executable plan.
**Contract rehearsal**: Passed for the bounded zero-limit change and seven-path delta described below. Package/native and separate thinking-budget predicates remain open.

#### Fix B_BUDGET.1: Preserve zero-limit acceptance and downstream validation intent

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: C018 / P20-SDK-MODEL-021 zero-context model-limit admission and simple output minimum. The other portions of C018 remain pending.
- **Decision**: Remove the blanket positive-only model metadata checks and clamp the selected zero-context simple limit to at least one. Keep product TOML validation, native omission, and genuinely invalid model or wire rejection. Update acceptance and rejection fixtures at the observed consumers.
- **Verification status**: Partially confirmed for C018 as a whole.
- **File(s)**: `CHANGELOG.md`, `crates/opi-agent/tests/agent_wrapper.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/api_mapped_provider.rs`, `crates/opi-ai/tests/codex_native_options.rs`, `crates/opi-ai/tests/context_budget.rs`, `crates/opi-ai/tests/custom_provider_registration.rs`, `crates/opi-ai/tests/node4_preparation.rs`, `crates/opi-ai/tests/provider_collection.rs`
- **Change kind**: behavioral.
- **Change**: Implement the rehearsed metadata admission, source-specific simple fallback, and assertion migrations.
- **Closure predicate**: Zero metadata reaches the public API. Native omission remains visible. A simple zero-context call with explicit zero or zero model maximum sends one; a positive default remains positive. The Agent and native API continue to reject truly invalid metadata.
- **Red-before**: Validation-only trial ran 306 tests with five failed assertions. After their migration, the exact public fallback test failed because the outgoing JSON omitted max_completion_tokens for explicit zero.
- **Green-after**: Re-run the exact public fallback test and complete affected-target union against the approved source. Observed: not-run.

| Trial | Observation | Command, input and raw logs |
|---|---|---|
| Baseline | 306 passed across nine binaries | [baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/baseline/receipt.json>) (`adf9f271f2140f556cec500581715802920538b42207557f78ff1db39c079319`) |
| Remove the two zero-admission checks | 301 passed, five failed across four test paths | [candidate/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/candidate/receipt.json>) (`691c750bfa3d04d90467769de48e916796490171749569e7be037cc4dcdc0ffa`) |
| Migrate four test paths and add public fallback probe | 332 passed, one failed on explicit zero | [fallback-red/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/fallback-red/receipt.json>) (`03aae6e0b686e7c7de1cdb3232bf69dbfe8011cad859eef5505c8197eec31344`) |
| Add zero-context simple output minimum | 916 passed across sixteen binaries | [fallback-green/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/fallback-green/receipt.json>) (`5a206d81b8700e4a20a225d6379aa48ea55ee49209a0733e5888de64fb9a67f4`) |

Per-target source, executable and run inventory: [budget-plan-lsgr7phy/executed-targets.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/executed-targets.json>) (`33a8bde62b81d72ad89ce9ebfabafc4f9b0e5b0e1bb4bb3d9536770fe9817ef6`). The raw build events and source-path inventory distinguish the SDK and product targets that share the custom_provider_registration name.

The five migrated assertions were the Agent constructor and idle replacement tests, API-mapped construction, custom model registration, and Codex native construction. The Agent and native rejection fixtures still test invalid inputs. Public SDK acceptance tests now exercise lawful zero input. Product config and custom-provider loader tests remain positive-only controls.

Path reconciliation: the seven modified export paths and their exact before/after hashes are in [budget-plan-lsgr7phy/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/budget-plan-lsgr7phy/trial-delta.json>) (`b4213814ddf2418e2a88310e505ea60b3d41f3e2806608585f160c6af4aa0c01`). They are the seven paths in the first bounded trial. The numeric extension adds `context_budget.rs` and `provider_collection.rs`. The paired SDK README and Unreleased are planned user-facing contract updates. The compound disposition carries the union of its assigned batch paths. Four paths are existing downstream tests. `node4_preparation.rs` gains the public-wire regression case. Test impact is `update` for the four fixtures, `add` for the public case, `retain` for the other assertions, and `delete` none.

Numeric-Claude extension rehearsal: the original guard caused two public cases to fail. The candidate clamps numeric thinking after the output cap, including zero-context zero model cap and custom budget at or above a positive cap. It migrates the existing default-budget rejection test. [before-numeric-reclaim-schema/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/before-numeric-reclaim-schema/receipt.json>) (`5c359c47b49563d2a417f581f3e80576a3c61a6c726d1af5e195ead8416e24b1`). The compiler-selected library rebuilt, with context_budget 16 and node4_preparation 27 passing while independent receipt expectations failed elsewhere. [candidate-fresh/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/candidate-fresh/receipt.json>) (`945d8c32892729d5078bc8e32cfcb1eb52ef0ab039ea9bac9e342fef29893577`). Common trial delta in [contract-continue-20261105T070028Z-405ed662/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/trial-delta.json>) (`b6ba3830573b1de45c3a8a43e61f3b1cbbd1b87dbd9db8dbe26a6c59382a5606`).

### Batch PXY_HTTP: Route prepared builtin Chat through the selected source

**Root change**: Route prepared builtin Chat through the selected source.
**Closure predicate**: Prepared builtin Chat selects the source-qualified route with scoped priority, exact protocol/ALL and bypass handling. Explicit direct ownership wins; malformed selected proxy fails safely without a fallback send.
**Dependencies**: []
**Verification union**: Repeat real prepared Chat scenarios plus the four-target union. Rehearse each remaining source owner separately under D1.
**Contract rehearsal**: 166 passing tests across builtin_scoped_env, http_acceptance, openai_chat_fixtures and proxy_support, including nine real routing cases. [baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/baseline/receipt.json>) (`6c40f4c3e9089c7ab0ebac9ee2865a89c4ae53d6cffba33ab7b541d90b150551`). [candidate/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/candidate/receipt.json>) (`b1b29eb53275335d5248a3892068f97d57f542c8baf62cb374820454002cb18c`). [proxy-plan-h422v3ff/http-candidate-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/http-candidate-delta.json>) (`c33720d08d4e0e0be06c8ae7ae58b489a00629ac8304a7af0620afde0c6bd93b`). This observes the Chat portion of the broader proxy finding.

#### Fix PXY_HTTP.1: Route prepared builtin Chat through the selected source

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`
- **Change kind**: behavioral
- **Change**: Freeze selected request routing in the scoped auth context and bind the prepared Chat acquisition to it.
- **Closure predicate**: Prepared builtin Chat selects the source-qualified route with scoped priority, exact protocol/ALL and bypass handling. Explicit direct ownership wins; malformed selected proxy fails safely without a fallback send.
- **Red-before**: Existing controls passed; the new actual prepared call failed the scoped-over-injected route assertion.
- **Green-after**: Repeat real prepared Chat scenarios plus the four-target union. Rehearse each remaining source owner separately under D1. Observed: not-run.

### Batch PXY_HTTP_OWNERS: Bind selected ordinary adapter clients before acquisition

**Root change**: Bind selected ordinary adapter clients before acquisition.
**Closure predicate**: Actual builtin Anthropic, Responses and Azure Chat/Responses retain the frozen proxy route through selected endpoint and acquisition. Anthropic federation reuses the token under an unchanged policy and re-exchanges on a changed policy.
**Dependencies**: ["PXY_HTTP"]
**Verification union**: Repeat five public route controls plus the eleven selected downstream targets on the integrated source.
**Contract rehearsal**: All five new scenarios passed and eleven unfiltered targets executed 483 passing tests. [red-ordinary-executed/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/ordinary-http-trials-f342aa50/red-ordinary-executed/receipt.json>) (`89f43790b2db3c339e86ae628dad39d32aaefa7e2cee894ec7ab13d7d3e0a311`). [green-ordinary/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/ordinary-http-trials-f342aa50/green-ordinary/receipt.json>) (`f05b6126ed580ea822997871eb99d412446ad120e5aee607978eb9609c08c80a`). Exact eight-path scope and hunk roles: [ordinary-http-trials-f342aa50/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/ordinary-http-trials-f342aa50/trial-delta.json>) (`d0403a991ac12d895171943bda8610eb3d214a3b59cdcf10dd2e7f5bd8640d8b`). Integrate these bytes with the separately rehearsed raw-source changes before a ready seal.

#### Fix PXY_HTTP_OWNERS.1: Bind selected ordinary adapter clients before acquisition

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/auth/anthropic_federation.rs`, `crates/opi-ai/src/azure_openai.rs`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`
- **Change kind**: behavioral
- **Change**: Use the scoped HTTP client at Anthropic generation/federation, Responses and Azure wrapper acquisition. Include the frozen route in the federation token cache identity.
- **Closure predicate**: Actual builtin Anthropic, Responses and Azure Chat/Responses retain the frozen proxy route through selected endpoint and acquisition. Anthropic federation reuses the token under an unchanged policy and re-exchanges on a changed policy.
- **Red-before**: Five actual new public scenarios failed against the inherited Chat-only trial. The inherited Chat route control passed.
- **Green-after**: Repeat five public route controls plus the eleven selected downstream targets on the integrated source. Observed: not-run.

### Batch PXY_WS: Bind Codex WebSocket acquisition to the selected proxy

**Root change**: Bind Codex WebSocket acquisition to the selected proxy.
**Closure predicate**: Actual Codex Upgrade uses the selected source route. A proxy identity change under the same session/account invalidates the old pooled socket and continuation.
**Dependencies**: ["PXY_HTTP"]
**Verification union**: Repeat the real Codex route/pool case and Upgrade suites.
**Contract rehearsal**: [expanded-baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-baseline/receipt.json>) (`e25ec9dae3178cfde91b58d35899406948a50d94b58f5c02f37f8b3cf8523b6f`). Expanded source candidate passed 1,609 tests in 20 binaries, with one existing Windows privilege case ignored. [expanded-reverified-native/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-native/receipt.json>) (`1ed2fed5e63efd1d70f7f96ae531f6b13a38dffba0e10d54536848dec32325f3`). The 26 source/test paths and shared caller migration are in [proxy-plan-h422v3ff/expanded-reverified-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-delta.json>) (`77d3e3376a27c43fadf51c7213775244a6b00d91d5bc426fd631aa62ec89e7bb`).

#### Fix PXY_WS.1: Bind Codex WebSocket acquisition to the selected proxy

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/openai_codex_responses/ws_transport.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`
- **Change kind**: behavioral
- **Change**: Use the frozen route for Codex Upgrade and include its identity in the WebSocket pool/continuation boundary.
- **Closure predicate**: Actual Codex Upgrade uses the selected source route. A proxy identity change under the same session/account invalidates the old pooled socket and continuation.
- **Red-before**: Baseline direct send produced 401 instead of reaching the chosen proxy.
- **Green-after**: Repeat the real Codex route/pool case and Upgrade suites. Observed: not-run.

### Batch PXY_HTTP_REST: Route the remaining selected HTTP adapters and Codex fallback

**Root change**: Bind existing selected HTTP ownership to the remaining real acquisition points.
**Closure predicate**: Gemini, Vertex Express, Mistral, Pi Messages, image and classifier routes send through their selected source profile. Codex direct SSE and Upgrade-to-SSE fallback retain the same proxy. Vertex stored API-key selection reads only allowed proxy keys at send time and never consults ADC credentials or files.
**Dependencies**: ["PXY_HTTP_OWNERS", "PXY_WS"]
**Verification union**: Repeat eleven real route probes and their 14 downstream targets in the final workspace and preserve the Vertex no-ADC invariant.
**Contract rehearsal**: All eleven discriminating probes failed before the changes and passed afterward. The corrected 14-binary union passed 664 tests. Vertex had one stale zero-environment-read expectation during actual generation. The updated fixture allows only raw HTTP proxy keys and rejects every credential or file read. [remaining-http-trials-9db2194e/route-evidence.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/remaining-http-trials-9db2194e/route-evidence.md>) (`5b1bda537dba67e802e8adf7762e7a2bc3f05f9e7e194780a4ab1190649ac229`). [remaining-http-trials-9db2194e/executed-targets.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/remaining-http-trials-9db2194e/executed-targets.json>) (`5efc7a926795cb366e6f0754d5dd5258a08bc8241178c91920f8ba0971b4a056`). [remaining-http-trials-9db2194e/remaining-incremental-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/remaining-http-trials-9db2194e/remaining-incremental-delta.json>) (`c986c5bbe86695cb6db13f2c3ef669e8533c11823a9f391b6e518136b6ab3194`).

#### Fix PXY_HTTP_REST.1: Route the remaining selected HTTP adapters and Codex fallback

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Remaining ordinary prepared adapter routes and Codex fallback for C010 and C018.
- **Decision**: Carry forward the existing scoped HTTP acquisition and retain per-adapter retry and credential owners.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/api/cloudflare.rs`, `crates/opi-ai/src/api/decisions.rs`, `crates/opi-ai/src/api/llama.rs`, `crates/opi-ai/src/api/openrouter_images.rs`, `crates/opi-ai/src/api/system_one.rs`, `crates/opi-ai/src/gemini.rs`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/mistral.rs`, `crates/opi-ai/src/pi_messages.rs`, `crates/opi-ai/src/vertex.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`, `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`, `crates/opi-ai/tests/vertex_fixtures.rs`
- **Change kind**: behavioral
- **Change**: Freeze the selected source profile at the existing adapter acquisition and update the exact Vertex downstream assertion.
- **Closure predicate**: Gemini, Vertex Express, Mistral, Pi Messages, image and classifier routes send through their selected source profile. Codex direct SSE and Upgrade-to-SSE fallback retain the same proxy. Vertex stored API-key selection reads only allowed proxy keys at send time and never consults ADC credentials or files.
- **Red-before**: The eight builtin, one llama.cpp and two Codex real request probes each failed on the inherited source before the route integration.
- **Green-after**: Repeat the real route probes, the complete downstream target union and the packaged caller. Observed: not-run.

### Batch PXY_AWS: Bind builtin Bedrock generation to the selected proxy

**Root change**: Bind builtin Bedrock generation to the selected proxy.
**Closure predicate**: Actual builtin Bedrock generation uses the scoped route while explicit AWS selection keeps its no-discovery contract and stored bearer avoids AWS credential-source reads.
**Dependencies**: ["PXY_HTTP"]
**Verification union**: Repeat the real scoped route and the retained explicit/no-credential-read fixture controls.
**Contract rehearsal**: [expanded-baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-baseline/receipt.json>) (`e25ec9dae3178cfde91b58d35899406948a50d94b58f5c02f37f8b3cf8523b6f`). Expanded source candidate passed 1,609 tests in 20 binaries, with one existing Windows privilege case ignored. [expanded-reverified-native/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-native/receipt.json>) (`1ed2fed5e63efd1d70f7f96ae531f6b13a38dffba0e10d54536848dec32325f3`). The 26 source/test paths and shared caller migration are in [proxy-plan-h422v3ff/expanded-reverified-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-delta.json>) (`77d3e3376a27c43fadf51c7213775244a6b00d91d5bc426fd631aa62ec89e7bb`).

#### Fix PXY_AWS.1: Bind builtin Bedrock generation to the selected proxy

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/bedrock/mod.rs`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`
- **Change kind**: behavioral
- **Change**: Attach the scoped route at Bedrock acquisition and preserve selected source credential isolation.
- **Closure predicate**: Actual builtin Bedrock generation uses the scoped route while explicit AWS selection keeps its no-discovery contract and stored bearer avoids AWS credential-source reads.
- **Red-before**: Baseline produced no proxy sends for the selected Bedrock source.
- **Green-after**: Repeat the real scoped route and the retained explicit/no-credential-read fixture controls. Observed: not-run.

### Batch PXY_ADC_SOURCE: Freeze distinct ADC proxy settings at source selection

**Root change**: Freeze distinct ADC proxy settings at source selection.
**Closure predicate**: Builtin-selected ADC captures raw present-empty versus absent env, retains injected custom transport ownership, and binds token caching to the selected source proxy profile.
**Dependencies**: ["PXY_HTTP"]
**Verification union**: Repeat actual builtin ADC source and custom owner controls, the AuthContext implementer/caller union and embedded README consumer.
**Contract rehearsal**: [expanded-baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-baseline/receipt.json>) (`e25ec9dae3178cfde91b58d35899406948a50d94b58f5c02f37f8b3cf8523b6f`). Expanded source candidate passed 1,609 tests in 20 binaries, with one existing Windows privilege case ignored. [expanded-reverified-native/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-native/receipt.json>) (`1ed2fed5e63efd1d70f7f96ae531f6b13a38dffba0e10d54536848dec32325f3`). The 26 source/test paths and shared caller migration are in [proxy-plan-h422v3ff/expanded-reverified-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-delta.json>) (`77d3e3376a27c43fadf51c7213775244a6b00d91d5bc426fd631aa62ec89e7bb`).

#### Fix PXY_ADC_SOURCE.1: Freeze distinct ADC proxy settings at source selection

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/src/auth/builtin.rs`, `crates/opi-ai/src/auth/builtin_vertex.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/auth_process_lifecycle.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/builtin_native_scoped.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`, `crates/opi-ai/tests/chat_capture_replay.rs`, `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/vertex_fixtures.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `scripts/pi-sdk-conformance/run_readme_examples.py`
- **Change kind**: behavioral
- **Change**: Capture raw ADC proxy environment with distinct absent and empty states and bind the selected route to the token cache.
- **Closure predicate**: Builtin-selected ADC captures raw present-empty versus absent env, retains injected custom transport ownership, and binds token caching to the selected source proxy profile.
- **Red-before**: Baseline source route and raw setting controls did not meet the Gaxios case.
- **Green-after**: Repeat actual builtin ADC source and custom owner controls, the AuthContext implementer/caller union and embedded README consumer. Observed: not-run.

### Batch B_ADC_REDIRECT: Apply the selected ADC source redirect rules

**Root change**: Apply the selected ADC source redirect rules.
**Closure predicate**: The actual authorized-user exchange follows cross-port 302 POST to GET on the initial chosen direct route, retains the admitted Authorization and Cookie headers, avoids synthesized Referer, and observes the bounded redirect/cancellation contract.
**Dependencies**: ["PXY_ADC_SOURCE"]
**Verification union**: Repeat actual source redirect/cache/cancellation and distinct connector/certificate cases on the integrated candidate.
**Contract rehearsal**: [expanded-baseline/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-baseline/receipt.json>) (`e25ec9dae3178cfde91b58d35899406948a50d94b58f5c02f37f8b3cf8523b6f`). Expanded source candidate passed 1,609 tests in 20 binaries, with one existing Windows privilege case ignored. [expanded-reverified-native/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-native/receipt.json>) (`1ed2fed5e63efd1d70f7f96ae531f6b13a38dffba0e10d54536848dec32325f3`). The 26 source/test paths and shared caller migration are in [proxy-plan-h422v3ff/expanded-reverified-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/expanded-reverified-delta.json>) (`77d3e3376a27c43fadf51c7213775244a6b00d91d5bc426fd631aa62ec89e7bb`). The direct-route case passed. CONNECT, outer TLS, certificate STS and protected audience remain D1 integration proof.

#### Fix B_ADC_REDIRECT.1: Apply the selected ADC source redirect rules

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`
- **Change kind**: behavioral
- **Change**: Follow the admitted token-source redirect set with the initial route and bounded redirect/cancellation handling.
- **Closure predicate**: The actual authorized-user exchange follows cross-port 302 POST to GET on the initial chosen direct route, retains the admitted Authorization and Cookie headers, avoids synthesized Referer, and observes the bounded redirect/cancellation contract.
- **Red-before**: Original default ADC refused the tested token POST redirect.
- **Green-after**: Repeat actual source redirect/cache/cancellation and distinct connector/certificate cases on the integrated candidate. Observed: not-run.

### Batch B_ADC_CONNECT: Use the selected ADC tunnel with separate proxy and target identities

**Root change**: Use the selected ADC tunnel with separate proxy and target identities.
**Closure predicate**: Use the selected frozen proxy for actual ADC CONNECT to clear or TLS targets. Anonymous outer proxy TLS and approved STS-only inner identity stay separate. Preserve 407 and 502 refusal status, source retry count, redirect audience, cancellation and no direct fallback.
**Dependencies**: ["B_ADC_REDIRECT"]
**Verification union**: Repeat actual builtin CONNECT, raw setting, redirect and cache probes together with the library refusal, trust, identity, cancellation and audience controls and final dependency/feature checks.
**Contract rehearsal**: Actual builtin scoped integration passed all eight cases, including two-hop CONNECT with frozen raw-empty/cache ownership. The library passed 371 cases, including anonymous outer TLS, exact inner STS identity and cross-origin certificate refusal. The initial refusal witnesses exposed a Windows reset handling assumption and an incorrect retry budget. [focused/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/focused/receipt.json>) (`81b91e2250202f63e5dd2b8ca1ffc1e43f033b886d24108590f1e1ce12d07e75`). [focused-refusal-green/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/focused-refusal-green/receipt.json>) (`7efd0b359f7dfc3ce73bcefcd2b70b045dbef1f28916c56591ca05bde8a35828`). The corrected two-case run passed with freshly compiled source and retained binaries. [focused-source-budget/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/focused-source-budget/receipt.json>) (`552319fe3b5ea7d694d3338878bb968d4545ddd31921eab400f7d5b3d93d37ed`). Pinned actual ExternalAccountAuthorizedUser source returns 407 after one attempt and 502 after four, with no inner send. [proxy-plan-h422v3ff/gaxios-source-connect-refusal.jsonl](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/gaxios-source-connect-refusal.jsonl>) (`2550ae35bfad4d221afd4d29a67d040a7611089c8424e21091d7a72dc0c89185`). In a separate fresh candidate, anonymous HTTPS proxy TLS and CONNECT completed, then the inner connection driver returned UnexpectedEof before the response consumer could read the complete two-byte framed body. Leaving framing completion to the consumer passed the immediate complete response and rejected its one-byte truncation. Nine targeted cases passed. [https-diagnostic/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/https-diagnostic/receipt.json>) (`8f6a18677f329fd62eda6f7e9a6b5963d724cbd3a6d8c5614868e7fee0d44148`). [https-framing-green/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/https-framing-green/receipt.json>) (`5382bce1f36b938c3eac2efe9b34bc4abfaafeabf206c70554fbda9634302974`). Join this exact framing correction and its probes with the builtin candidate, including the complete certificate file-supplier and audience paths. Independent framing experiment patch and exact five-path delta [https-framing-trial.patch](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/https-framing-trial.patch>) (`502d20e59d683827d59898f86eb6ce2e6509ae31d7a12f42cf6c42fa62cfd745`), [https-framing-trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/adc-connect-plan-68e2ca4f/https-framing-trial-delta.json>) (`87be27f4c172f6b194f9ffb24cf897474fedc7eae4f39debc6e0c3dea268386a`). Exact nine-path delta in [integrated-connect/final-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/final-delta.json>) (`e58bbf1dec61650c4948b2b868ff33aa8121b42f29365ce734845b163dd77f5e`). The effective ordinary graph is unchanged; the Windows 193-unit feature union drops only the tokio-rustls `default` marker and retains its constituent leaf features. [integrated-connect/ordinary-graph-diff.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/ordinary-graph-diff.json>) (`72837bc50b05a1ecd54ac7fcf0def4df782aafe5ddf1d2958312d102a5774e6d`). [integrated-connect/windows-graph-union.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/proxy-plan-h422v3ff/integrated-connect/windows-graph-union.json>) (`66cdb2f843ad2e7cfee0af971b39f77459f89b49fe828a7b9addb57311315330`).

#### Fix B_ADC_CONNECT.1: Use the selected ADC tunnel with separate proxy and target identities

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C010, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `Cargo.toml`, `crates/opi-ai/Cargo.toml`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/adc/connect_proxy.rs`, `crates/opi-ai/src/auth/adc/external.rs`, `crates/opi-ai/src/auth/adc/impersonation.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`
- **Change kind**: behavioral
- **Change**: Build an ADC-owned CONNECT tunnel on already resolved HTTP/TLS dependencies. Separate outer proxy and inner STS TLS configuration and retain typed refusal status.
- **Closure predicate**: Use the selected frozen proxy for actual ADC CONNECT to clear or TLS targets. Anonymous outer proxy TLS and approved STS-only inner identity stay separate. Preserve 407 and 502 refusal status, source retry count, redirect audience, cancellation and no direct fallback.
- **Red-before**: A default clear HTTP proxy used forwarding rather than CONNECT. A separate reqwest comparator offered the synthetic target client certificate to the outer proxy. The pinned actual source discriminates one 407 attempt and four 502 attempts with the default budget.
- **Green-after**: Repeat actual builtin CONNECT, raw setting, redirect and cache probes together with the library refusal, trust, identity, cancellation and audience controls and final dependency/feature checks. Observed: not-run.

### Batch B_EXPORT: Admit a bound archive through its actual Cargo cache owner

**Root change**: Admit a bound archive through its actual Cargo cache owner.
**Closure predicate**: The package runner accepts the actual owner or an explicitly bound exact committed export and refuses missing/forged owner or source identity before creating scratch.
**Dependencies**: []
**Verification union**: Run exact owner/archive positives, wrong head, foreign marker, source-tree drift and package runner checks.
**Contract rehearsal**: The archive guard accepts the exact committed source and an explicitly hashed exact isolated delta from the actual cache owner. It rejects implicit owner, wrong head, wrong digest, extra path and missing changed bytes. All three real SDK/Schema/README CLI negative paths stop before scratch and Cargo. [delivery-integrated-plan-4155b24d/pure-receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/pure-receipt.json>) (`ec7091a4c4aacb91282bcb01ff1a686af03e787477d78c37ac41e2934fbb049f`). [delivery-integrated-plan-4155b24d/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/trial-delta.json>) (`00d9dd3c8de4414af6603b4d0cf216438902241994677df743c3b8d0f94718f4`). The merged README harness retains the raw AuthContext fixture as well as archive admission.

#### Fix B_EXPORT.1: Admit a bound archive through its actual Cargo cache owner

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `scripts/pi-sdk-conformance/package_slice.py`, `scripts/pi-sdk-conformance/run_early_package_proof.py`, `scripts/pi-sdk-conformance/run_readme_examples.py`, `scripts/pi-sdk-conformance/test_package_slice.py`
- **Change kind**: behavioral
- **Change**: Add exact committed-export admission to the existing owner-bound package runner and reject invalid material before creating scratch.
- **Closure predicate**: The package runner accepts the actual owner or an explicitly bound exact committed export and refuses missing/forged owner or source identity before creating scratch.
- **Red-before**: Actual owner accepted. The same marker refused the committed archive before scratch.
- **Green-after**: Run exact owner/archive positives, wrong head, foreign marker, source-tree drift and package runner checks. Observed: not-run.

### Batch B_SDK: Execute the assembled public SDK as an actual package consumer

**Root change**: Execute the assembled public SDK as an actual package consumer.
**Closure predicate**: The actual extracted .crate is the only Opi runtime dependency in an independently locked external workspace. The declared SDK inventory executes real public native/simple/direct/custom, typed operations, Send, ownership, and offline-resource cases.
**Dependencies**: ["B_EXPORT"]
**Verification union**: Execute independent real package consumer on final sources with exact test, graph, binary and lock receipts. Wider feature/native coverage is B_NATIVE and D3.
**Contract rehearsal**: The actual merged extracted SDK package passed 48 ordinary and 48 caller-feature-union cases from the same independent lock. Both profiles freshly compiled the extracted SDK and retained their exact test binaries and resolved RDF-21/HTTP graphs. [merged-sdk-proof-1/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-runtime-join-aa9kjgkr/merged-sdk-proof-1/receipt.json>) (`7966d06ab182034fc19382eee901274823bf02e79ed58e9ebf79347444d98936`).

#### Fix B_SDK.1: Execute the assembled public SDK as an actual package consumer

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-017; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C017, C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Partially confirmed
- **File(s)**: `scripts/pi-sdk-conformance/README.md`, `scripts/pi-sdk-conformance/consumer-sdk/Cargo.toml`, `scripts/pi-sdk-conformance/consumer-sdk/tests/entrances.rs`, `scripts/pi-sdk-conformance/package_slice.py`, `scripts/pi-sdk-conformance/test_package_slice.py`
- **Change kind**: behavioral
- **Change**: Run the assembled SDK inventory in the independently locked extracted-package consumer.
- **Closure predicate**: The actual extracted .crate is the only Opi runtime dependency in an independently locked external workspace. The declared SDK inventory executes real public native/simple/direct/custom, typed operations, Send, ownership, and offline-resource cases.
- **Red-before**: Original package runner rejected the sdk selector. First exploratory independent resolution exposed versions outside RDF-22.
- **Green-after**: Execute independent real package consumer on final sources with exact test, graph, binary and lock receipts. Wider feature/native coverage is B_NATIVE and D3. Observed: not-run.

### Batch B_RECEIPT: Reclaim confirmed mutation receipts after observers release them

**Root change**: Reclaim confirmed mutation receipts after observers release them.
**Closure predicate**: Drop confirmed unreferenced records on queue activity. Keep receipts observed by ACKs, cloned barriers, pending reports and late captures. Preserve Failed/Unknown records across ordinary success.
**Dependencies**: []
**Verification union**: Repeat all queue/auth/credential tests and verify pending/late protection and source-scoped reconciliation separately.
**Contract rehearsal**: Five new reclamation tests passed on the freshly rebuilt candidate. [before-numeric-reclaim-schema/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/before-numeric-reclaim-schema/receipt.json>) (`5c359c47b49563d2a417f581f3e80576a3c61a6c726d1af5e195ead8416e24b1`). [candidate-fresh/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/candidate-fresh/receipt.json>) (`945d8c32892729d5078bc8e32cfcb1eb52ef0ab039ea9bac9e342fef29893577`). After migrating seven downstream observer assertions, five unfiltered targets passed all 545 tests. [candidate-verified/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/candidate-verified/receipt.json>) (`107963cf5f52fa412b4f10d9f092b4489e8a1eb7f5e99fb54e1d6b7b5b57999a`). Exact common source/test delta in [contract-continue-20261105T070028Z-405ed662/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/trial-delta.json>) (`b6ba3830573b1de45c3a8a43e61f3b1cbbd1b87dbd9db8dbe26a6c59382a5606`). Same-key authoritative reconciliation remains a separate D3 predicate.

#### Fix B_RECEIPT.1: Reclaim confirmed mutation receipts after observers release them

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/mutation.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/mutation_reclamation.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-coding-agent/README.md`, `crates/opi-coding-agent/README.zh.md`, `crates/opi-coding-agent/tests/credential_store.rs`
- **Change kind**: behavioral
- **Change**: Track which barrier, ACK and late-capture observers retain each receipt. Reclaim only confirmed receipts with no live observer.
- **Closure predicate**: Drop confirmed unreferenced records on queue activity. Keep receipts observed by ACKs, cloned barriers, pending reports and late captures. Preserve Failed/Unknown records across ordinary success.
- **Red-before**: Five public queue reclamation scenarios failed on original production bytes.
- **Green-after**: Repeat all queue/auth/credential tests and verify pending/late protection and source-scoped reconciliation separately. Observed: not-run.

### Batch B_SCHEMA: Correct the selected resource fixture against pinned TypeBox

**Root change**: Correct the selected resource fixture against pinned TypeBox.
**Closure predicate**: The encoded pointer controls retain their original checks. The absolute selected resource fixture accepts the intended value and rejects all wrong-resource values under the actual engine.
**Dependencies**: []
**Verification union**: Repeat schema_engine and schema_public plus package data-profile proofs; this batch changes the fixture only.
**Contract rehearsal**: Pinned TypeBox1.3.27 under Node24.13.0 compared 40 compiled/interpreted cases; the absolute positive and wrong-resource negatives agree. [schema-oracle/reference-variants.receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/schema-oracle/reference-variants.receipt.json>) (`1eb9d27cd1af3f19dd0ae09fd6ba360fc836747633d0c3a11cce15ec4aa95691`). Actual Rust candidate schema_engine 73 and schema_public 28 passed in [candidate-fresh/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/contract-continue-20261105T070028Z-405ed662/candidate-fresh/receipt.json>) (`945d8c32892729d5078bc8e32cfcb1eb52ef0ab039ea9bac9e342fef29893577`); unrelated receipt assertions failed in other binaries.

#### Fix B_SCHEMA.1: Correct the selected resource fixture against pinned TypeBox

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Assigned portion**: Only this batch predicate for C018. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/schema_engine.rs`
- **Change kind**: test-only
- **Change**: Correct the resource fixture identity and retain paired positive and wrong-resource controls, leaving engine behavior as rehearsed.
- **Closure predicate**: The encoded pointer controls retain their original checks. The absolute selected resource fixture accepts the intended value and rejects all wrong-resource values under the actual engine.
- **Red-before**: Rust baseline fixture returned false at the asserted positive. The original TypeBox compiled and interpreted cases also reject the relative form.
- **Green-after**: Repeat schema_engine and schema_public plus package data-profile proofs; this batch changes the fixture only. Observed: not-run.

### Batch B_CACHE: Preserve typed prompt-cache duration metadata

**Root change**: Preserve typed prompt-cache duration metadata.
**Closure predicate**: ModelInfo and the typed catalog store preserve independently optional finite positive seconds, including fractional durations and explicit empty metadata. Invalid writes leave revision and stored bytes unchanged.
**Dependencies**: []
**Verification union**: Repeat the five unfiltered targets and actual extracted package applicability after the approved edits.
**Contract rehearsal**: Observed 87 passing tests across catalog_artifact, catalog_lifecycle, catalog_operations, model_wire_metadata and product list_models. [d4-red/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/d4-red/receipt.json>) (`d9eda1c5dce1e025620d1b1dba55e467cabc999083e11736e73645a3a2f75374`). [d4-green/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/d4-green/receipt.json>) (`a6f5162307c1f36cb74be4ae723734f6e54358c9bf94d83a3f569e76c5767c70`). Exact runtime/test paths in [metadata-trials-8a36672b/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/trial-delta.json>) (`f1ba589518cda5ca404574d90cf8f22855ab281556e7f5865ef85a83651b3922`). User-facing documentation paths in [metadata-trials-8a36672b/docs-candidate.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/docs-candidate.md>) (`efcc4ceccd49df54d8b85918ffe82647880e6be3ba680246aedd96608cab2be1`).

#### Fix B_CACHE.1: Preserve typed prompt-cache duration metadata

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-019
- **Assigned portion**: C019 whole finding
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`
- **Change kind**: behavioral
- **Change**: Carry both independent typed cache-duration tiers through model and catalog storage, with validation before mutation.
- **Closure predicate**: ModelInfo and the typed catalog store preserve independently optional finite positive seconds, including fractional durations and explicit empty metadata. Invalid writes leave revision and stored bytes unchanged.
- **Red-before**: Two discriminating public catalog lifecycle failures on the original code.
- **Green-after**: Repeat the five unfiltered targets and actual extracted package applicability after the approved edits. Observed: not-run.

### Batch B_EXPIRY: Expose typed permanent OAuth lifetime and restore legacy credentials

**Root change**: Expose typed permanent OAuth lifetime and restore legacy credentials.
**Closure predicate**: SDK and credential store distinguish Unknown, finite At, and NonExpiring. Permanent OpenRouter login and refresh preserve typed lifetime, normal finite/unknown behavior remains, product v2 survives reload, and legacy OpenRouter sentinel maps only with empty refresh.
**Dependencies**: []
**Verification union**: Repeat the exact SDK/product candidate union, public examples, durable record and actual package checks after the approved edits.
**Contract rehearsal**: Observed 688 passing tests in ten unfiltered SDK/product targets, zero failed/ignored/filtered. [d5-red/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/oauth/d5-red/receipt.json>) (`1df6fa28b9a7ebdce7daed64845df5877558db5b67d13441fafdf2cc86820968`). [d5-red-v2/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/oauth/red-control/d5-red-v2/receipt.json>) (`206f1df1643d250a3d12f75ac1e724399f288147734a0899f54df18c508a62f0`). [d5-verified/receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/oauth/d5-verified/receipt.json>) (`adb4e4fe73210bfe9d73cbb930ee847f24335b2cca7b15f8033641cdad5de885`). Exact runtime/test paths in [oauth/trial-delta.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/oauth/trial-delta.json>) (`36fc4ac5890966045a56be338976df256a7299a2d1f855f1886f3f0b78f8711a`). Published caller and paired documentation migration in [metadata-trials-8a36672b/docs-candidate.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/metadata-trials-8a36672b/docs-candidate.md>) (`efcc4ceccd49df54d8b85918ffe82647880e6be3ba680246aedd96608cab2be1`).

#### Fix B_EXPIRY.1: Expose typed permanent OAuth lifetime and restore legacy credentials

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-020
- **Assigned portion**: C020 whole finding
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/oauth/chatgpt.rs`, `crates/opi-ai/src/auth/oauth/copilot.rs`, `crates/opi-ai/src/auth/oauth/kimi_coding.rs`, `crates/opi-ai/src/auth/oauth/meta.rs`, `crates/opi-ai/src/auth/oauth/mod.rs`, `crates/opi-ai/src/auth/oauth/openrouter.rs`, `crates/opi-ai/src/auth/oauth/radius.rs`, `crates/opi-ai/src/auth/oauth/xai.rs`, `crates/opi-ai/src/auth/store_oauth.rs`, `crates/opi-ai/src/credential.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-ai/tests/oauth_store_integration.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-coding-agent/README.md`, `crates/opi-coding-agent/README.zh.md`, `crates/opi-coding-agent/src/credential_store.rs`, `crates/opi-coding-agent/tests/credential_store.rs`, `crates/opi-coding-agent/tests/doctor_cli.rs`, `crates/opi-coding-agent/tests/github_copilot_provider.rs`, `crates/opi-coding-agent/tests/list_models.rs`, `crates/opi-coding-agent/tests/oauth_auth.rs`
- **Change kind**: behavioral
- **Change**: Represent OAuth lifetime as Unknown, finite At, or NonExpiring and migrate only the admitted legacy durable representation.
- **Closure predicate**: SDK and credential store distinguish Unknown, finite At, and NonExpiring. Permanent OpenRouter login and refresh preserve typed lifetime, normal finite/unknown behavior remains, product v2 survives reload, and legacy OpenRouter sentinel maps only with empty refresh.
- **Red-before**: Loopback public login produced 9999; corrected marked raw keychain v2 baseline produced UnknownEnvelope. Original unmarked setup failure is excluded from the decoder evidence.
- **Green-after**: Repeat the exact SDK/product candidate union, public examples, durable record and actual package checks after the approved edits. Observed: not-run.

### Batch B_NATIVE: Prepare the six native host/toolchain CI rows

**Root change**: Prepare the six native host/toolchain CI rows.
**Closure predicate**: Linux, macOS and Windows each run stable and exact 1.97.0 from the verified checkpoint, with locked dependency preparation, offline checks and native source/material receipts.
**Dependencies**: ["B_BUDGET", "B_CACHE", "B_EXPIRY", "PXY_HTTP", "PXY_HTTP_OWNERS", "PXY_WS", "PXY_HTTP_REST", "PXY_AWS", "PXY_ADC_SOURCE", "B_ADC_REDIRECT", "B_ADC_CONNECT", "B_EXPORT", "B_SDK", "B_RECEIPT", "B_SCHEMA"]
**Verification union**: After explicit approved apply and local checks, publish only the exact verified checkpoint to the reserved codex/phase20-remediation-ci branch, bind each host/toolchain checkout, and retain all original proof before closure.
**Contract rehearsal**: 90 pure tests and six local command-plan dry runs passed. Each row has 16 verification and 7 dependency-preparation stages. The three reviewed native host architectures and both toolchains remain checkpoint verification. The other advertised release architectures are separate claim-mapping rows. [delivery-integrated-plan-4155b24d/pure-receipt.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/pure-receipt.json>) (`ec7091a4c4aacb91282bcb01ff1a686af03e787477d78c37ac41e2934fbb049f`). [delivery-integrated-plan-4155b24d/six-native-plans.json](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/six-native-plans.json>) (`5c362a7fa99ce0c8e174ef8cf5b46244069a195cfa424fc8b5db57fe9665843a`). [delivery-integrated-plan-4155b24d/delivery-handoff.md](<D:/Luiz/Odradek/opi-remediation-evidence/phase20/delivery-integrated-plan-4155b24d/delivery-handoff.md>) (`7d170ab8ed2182f60a36950cf037f9667d949d2e2459966d96b59c067debfb2c`).

#### Fix B_NATIVE.1: Prepare the six native host/toolchain CI rows

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Assigned portion**: Only this batch predicate for C018, P007. Complete each compound finding through its full ordered batch list.
- **Decision**: Carry the bounded repair forward under this batch predicate.
- **Verification status**: Partially confirmed
- **File(s)**: `.github/workflows/phase20-remediation-native.yml`, `scripts/pi-sdk-conformance/run_phase20_native.py`, `scripts/pi-sdk-conformance/test_run_phase20_native.py`
- **Change kind**: behavioral
- **Change**: Add the six stable/MSRV host rows and native source/material receipt collection to the reserved checkpoint CI route.
- **Closure predicate**: Linux, macOS and Windows each run stable and exact 1.97.0 from the verified checkpoint, with locked dependency preparation, offline checks and native source/material receipts.
- **Red-before**: Current source lacks the executable six-row remediation route for the assembled SDK package and native/material closure.
- **Green-after**: After explicit approved apply and local checks, publish only the exact verified checkpoint to the reserved codex/phase20-remediation-ci branch, bind each host/toolchain checkout, and retain all original proof before closure. Observed: not-run.

## Final Verification

After all decisions are resolved, join each of the 165 linked source records to its actual required criterion evidence and affected test cases. Each caller-visible batch needs the isolated rehearsal and exact path reconciliation before a ready seal. After explicit apply, prove the approved source state with the execution protocol, including the actual package/native provenance and workspace union required by the admitted plan.

## Exclusions

The 22 `Refuted` records above retain the original severities and their bounded counter-evidence. P006 records that the frozen matrix pin matches the archive commit. C021 records the independently corrected registered-source wording. This draft records planning evidence and the known repair paths. Apply verification and the Phase exit verdict require their owning workflows.
