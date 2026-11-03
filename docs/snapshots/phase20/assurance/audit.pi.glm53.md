# Phase 20 Audit

**Audit run ID**: `phase20-pi-glm53-44374ec-20261103t160703z`
**Audit head**: `44374ec3c70b82125a183757dde2017b46299c45`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent harness, fresh independent session
**Reviewer model ID**: `glm53`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family — sealed git-archive export of the committed baseline; no prior audit, remediation, history or sibling-peer content ingested before installation
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `15c7f977d0c2cd7aee098836d5d47c195b402e1e2fb9a82a16fb69c89f52f975` | current committed source |
| `docs/snapshots/phase20/opi-impl-state.json` | `222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8` | current committed source |
| `docs/opi-spec.md` | `b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3` | current committed source |
| `docs/opi-spec.zh.md` | `e9da7696be0772e2ca72f74943c0b3fbba398850abc751d58726d31068cbb13c` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` | `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/spec-draft.zh.md` | `a841575a91ab4a3895648c1c66ce2c6c7d9afd4685fb1d408e550d39b3f62415` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` | `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` | `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` | `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md` | `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d` | current committed source; stored v2 hash in both ledgers is stale (see finding P20-AUD-006) |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md` | `b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` | `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md` | `61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` | `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md` | `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md` | `25a4234507ab1c4c12d483c56bb4083172bf4d5c35dfec1dc92bc98c77a5d94e` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/replay-contracts.md` | `223b21e1aa590d0e4646d99cd7f6792a694c7d8bf0f34b31dd6380979d324fa4` | current committed source |
| `docs/research/pi-ai-opi-ai-alignment/sdk-target-proposal.md` | `4f84b93681a6dce3bea494d0553e81a1f5cb3aec5ad62635eda93a8ee1f5a75c` | current committed source |

## Requirement Conformance

257 sealed requirement records (one per unique registered acceptance scenario across the 68 task registrations; 16 scenario ids dual-registered on 20.57 keep one record citing both owners). 236 `met`, 21 `partially-met` (all mandatory), 0 `not-met`, 0 `not-assessable`; the 7 `P20-GAP-*` rows are the registered OPEN coverage-group trackers (optional; honest OPEN recording verified). Full machine records with per-record checks live in `audit.pi.glm53.requirements.jsonl`.

### Partially-met (mandatory) — findings linked

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P20-S12 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-S12` | dedicated default-client twenty-hop acceptance test fails deterministically on this host (test-harness keep-alive fidelity; see P20-AUD-001) | partially-met | P20-AUD-001 |
| V-44 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-44` | request_enrichment acceptance contains a failing session-affinity test contradicting pinned source (P20-AUD-003) | partially-met | P20-AUD-003 |
| AIR-052 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#AIR-052` | same binary family as V-44 | partially-met | P20-AUD-003 |
| V-05 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-05` | prepare-path credential failures surface as StreamFailure(CredentialNeeded); classification clause observably violated at the product boundary (P20-AUD-004) | partially-met | P20-AUD-004 |
| AIR-007 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#AIR-007` | same family as V-05 | partially-met | P20-AUD-004 |
| V-11 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-11` | compat strict/role/store semantics implemented per source; committed fixture acceptance contradicts pinned source and fails (P20-AUD-002) | partially-met | P20-AUD-002 |
| P20-S43 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-S43` | Conversations strict default diverges from pinned source (P20-AUD-005); mistral compat strict acceptance test fails (P20-AUD-002) | partially-met | P20-AUD-002, P20-AUD-005 |
| ONB-03-20.43 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#ONB-03-20.43` | same family as P20-S43 | partially-met | P20-AUD-002, P20-AUD-005 |
| P20-S51 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-S51` | mistral_fixtures binary contains the failing shared-compat strict test | partially-met | P20-AUD-002 |
| ONB-04-20.51 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#ONB-04-20.51` | mistral_fixtures binary contains the failing shared-compat strict test | partially-met | P20-AUD-002 |
| P20-RDF-21 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md#P20-RDF-21` | external_sdk matrix passes at head; run_early_package_proof recorded at prior tree identity, not rerun (P20-AUD-007) | partially-met | P20-AUD-007 |
| ONB-02-20.57 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#ONB-02-20.57` | readme/package proofs recorded at 20.57 identity; not rerun at audit_head (P20-AUD-007) | partially-met | P20-AUD-007 |
| V-58 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-58` | Linux/MSRV slice re-verified at audit_head (platform_delivery 9/9, suite green except findings); recorded Windows/package evidence predates audit_head production changes; macOS stays OPEN (P20-AUD-007) | partially-met | P20-AUD-007 |
| P20-S58 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-S58` | freeze tests pass at head; native/package evidence staleness and macOS OPEN (P20-AUD-007) | partially-met | P20-AUD-007 |
| P20-EXIT-06 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-EXIT-06` | Linux consumer/package proof refreshed at head; cross-platform and CI dispatch not re-proven at head (P20-AUD-007) | partially-met | P20-AUD-007 |
| ONB-06-20.58 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#ONB-06-20.58` | README claims checked statically + Linux artifacts; Windows evidence stale-by-tree, macOS OPEN (P20-AUD-007) | partially-met | P20-AUD-007 |
| V-59 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-59` | registered-source hash drift (acceptance-matrix.md) unreconciled in both ledgers (P20-AUD-006); doc-check PASS | partially-met | P20-AUD-006 |
| AIR-067 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#AIR-067` | same family as V-59 | partially-met | P20-AUD-006 |
| P20-S59 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#P20-S59` | P.0/P.2/P.4 reconciliation pending (P20-AUD-006); evidence handoff partially stale vs audit_head (P20-AUD-007) | partially-met | P20-AUD-006, P20-AUD-007 |
| DEL-14 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#DEL-14` | documentary handoff references evidence identities that predate audit_head production changes (P20-AUD-007) | partially-met | P20-AUD-007 |
| ONB-07-20.59 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#ONB-07-20.59` | same family as P20-S59 | partially-met | P20-AUD-006, P20-AUD-007 |

### Met requirements

All remaining 236 records are `met`. Evidence classes executed at audit_head (WSL2 Linux x86_64, rustc/cargo 1.97.1 — exactly the declared MSRV; sealed export; shared marked Cargo cache):

- `cargo test -p opi-ai --all-targets --no-fail-fast`: 73 targets, 2374 passed / 4 failed / 0 ignored; the 4 failing tests belong to findings P20-AUD-001..003 (opi-ai) — every other target green, including `external_sdk` 10/10, `platform_delivery` 9/9, `ws_upgrade` 16/16, lib 367/367.
- `cargo test -p opi-agent --all-targets --no-fail-fast`: 33 targets, 605 passed / 1 failed (`continuation_contract`, finding P20-AUD-004).
- `cargo test -p opi-coding-agent --test product_evidence --test credential_store --test oauth_auth --test product_source_barrier --test bedrock_provider_wiring --test vertex_provider_wiring --test provider_factory --test interactive_startup_diagnostics --test tool_authority`: 313 passed / 0 failed.
- `cargo test -p opi-ai --doc`: 3 passed. `cargo fmt --check --all`: PASS. `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `python3 scripts/opi-doc-check.py`: PASS. `python3 -B -m unittest discover -s scripts/pi-sdk-conformance`: 82 tests OK. `git diff --check` on the clean tree at audit_head: PASS.

Per-record evidence, executed commands, hosts and limitations are in the requirements sidecar; records citing a binary that contains an unrelated failing test carry an explicit limitation note and remain assessed on their own criterion.

## Standards Review

`cargo fmt --check --all` and `cargo clippy --workspace --all-targets -- -D warnings` pass at audit_head on the MSRV toolchain. Workspace topology matches the registered layout (7 members, no standalone WS crate, `rust-version = 1.97`, edition 2024); asserted live by `platform_delivery` (9/9). Internal dependency edges, single feature union for opi-ai and the frozen pinned closure are exercised by the same freeze tests. Doc lockstep (`opi-doc-check.py`) passes; bilingual README counterparts exist. CHANGELOG carries an Unreleased section.

## Spec Review

Mechanism-level source conformance was verified by executing the registered per-task verification routes (above) and by differential inspection against the pinned `.repo/pi-1.3.0` source for the four failing surfaces: shared-compat strict projection and session-affinity headers match the pinned source (tests contradict it — P20-AUD-002/003); the redirect budget mechanism is source-conformant while its dedicated default-client acceptance test is broken on honest keep-alive servers (P20-AUD-001); the Mistral Conversations strict default and the prepare-path credential classification diverge from the pinned/registered contracts (P20-AUD-005/004).

## Security, Invariants, Integration, Test Quality, and Residuals

- **Security/authority**: no live calls, credentials or paid providers were used; all executed evidence is fixture/native-local. Boundary tests for header protection, payload validation-before-dispatch, schema coercion refusal and typed redacted failures passed within the suite.
- **Invariants/integration**: `StreamFailure` pre-handoff classification intent is violated at the prepare boundary (P20-AUD-004), breaking the product login-remediation consumers. Registered-source hash drift is pending its designed reconciliation (P20-AUD-006).
- **Test quality**: three committed acceptance tests contradict the pinned source or the transport reality and fail at audit_head (P20-AUD-001..003), so the declared full workspace gate cannot pass; recorded delivery evidence therefore no longer describes the delivered tree (P20-AUD-007).
- **Residuals**: the retired `companions/ws` layout and planned `ws_codec`/`crypto-family package_slice` entries survive only as registration planning notes; current-code evidence covers them via the consolidated suites and the freeze tests. No speculative public seams were observed (single feature union; no public codec/test-only export).

## Minimum-change Conformance

Per-task historical planning intent is not assessable from the sealed projection (statuses excluded by design); conformance was inspected on current surfaces: workspace membership, dependency direction, the ws consolidation (private modules, no new crate), the absence of a second SDK facade and the frozen supply-chain identity all hold at audit_head and are asserted by executing tests rather than prose. No unrelated staged/unstaged work existed (clean tree at audit_head).

## Findings

### P20-AUD-001: Default-client redirect-budget acceptance test fails on keep-alive-dead pooled connections

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-S12
- Claim: The dedicated acceptance test for the source-qualified default twenty-hop redirect budget fails deterministically on this audit host: the scripted redirect server closes each connection without advertising 'Connection: close', so the pooling default client (max_idle_per_host=10) reuses a dead keep-alive connection on a later 307 hop and the non-idempotent POST is not retried, surfacing Network('error sending request') instead of completing the chain. The explicit Limited(20) client passes the identical chain only because plain_client disables pooling. The test therefore conflates a test-harness keep-alive fidelity gap with the redirect-budget boundary and breaks the declared workspace gate at audit_head.
- Evidence:
  - `crates/opi-ai/tests/http_acceptance.rs:536-575` — failing test source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary; panic '20 redirects must succeed; explicit=false'
  - `crates/opi-ai/tests/http_acceptance.rs:378-380` — redirect_response emits no 'Connection: close' header while Script::coalesced hard-closes the socket (serve_scripted lines 152-158)
  - `crates/opi-ai/tests/http_acceptance.rs:478-485` — plain_client sets max_idle_per_host(0), avoiding the dead-pool reuse that the default HttpClient (pool default 10) hits
  - `audit execution log ( opi-ai suite, 2 runs + isolated rerun )` — 20 passed / 1 failed in-suite; isolated rerun with RUST_BACKTRACE=1 failed identically (deterministic)
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md#RDF-22 (upstream HTTP acceptance profile; redirect budget twenty hops with typed refusal)`
- Reproduction: `cargo test -p opi-ai --test http_acceptance source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary -- --exact`
- Refutation attempted: searched the redirect implementation and default policy (Limited(20) default asserted by a passing test; explicit client completes the identical 20-hop chain); no product-side counter-evidence — the default client fails only against a server that closes without advertising Connection: close.
- Confidence: high

### P20-AUD-002: Mistral/OpenRouter compat fixture tests assert strict:true, contradicting the pinned source and the shipped implementation

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-S43, ONB-03-20.43, V-11, P20-S51, ONB-04-20.51
- Claim: mistral_profile_inherits_shared_compat_flags and openrouter_profile_inherits_shared_compat_flags assert tools[0].function.strict == true for an unannotated tool on a strict_tool_schema profile. The pinned Pi 1.3.0 compat chat wire sends 'strict: strict ?? false' (false for unannotated tools), the opi-ai README documents 'unconstrained tools send strict: false on capable wires', and the shared adapter implements exactly that (default_strict: Some(false)). Both committed tests fail at audit_head, so the declared workspace gate cannot pass; the tests encode a misreading of strict_tool_schema as a per-tool default rather than a capability flag.
- Evidence:
  - `crates/opi-ai/tests/mistral_fixtures.rs:537-540` — assertion 'Mistral inherits strict-tool-schema from the shared compat path' expects strict == true; probe shows wire strict:false
  - `crates/opi-ai/tests/openrouter_fixtures.rs:490-493` — same assertion for OpenRouter
  - `.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts:1507-1515` — pinned source: '...(compat.supportsStrictMode !== false && { strict: strict ?? false })'
  - `crates/opi-ai/src/openai_chat.rs:2845-2850` — chat_tool_options sets default_strict: Some(false); openai_responses_shared.rs:720 'strict = per_tool_strict.or(options.default_strict)'
  - `crates/opi-ai/README.md (compat table)` — documents 'unconstrained tools send strict: false on capable wires'
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md#compat-schema (supportsStrictMode wire rules; strict projection per pinned source)`
- Reproduction: `cargo test -p opi-ai --test mistral_fixtures mistral_profile_inherits_shared_compat_flags -- --exact`; `cargo test -p opi-ai --test openrouter_fixtures openrouter_profile_inherits_shared_compat_flags -- --exact`
- Refutation attempted: checked the pinned Pi source, the shipped adapter and the README contract — all three agree on strict:false for unannotated tools; no registered difference authorizes strict:true on compat wires.
- Confidence: high

### P20-AUD-003: Session-affinity mapping test expects retention-gated suppression that neither the pinned source nor the implementation provides

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: V-44, AIR-052
- Claim: session_affinity_wire_mappings expects session-affinity headers (session_id / x-client-request-id / x-session-affinity) to be suppressed when cache_retention is Disabled. The pinned Pi 1.3.0 source emits those headers whenever a session id exists and sendSessionAffinityHeaders is set, and only the prompt-cache key is retention-gated (cacheSessionId = retention === 'none' ? undefined : sessionId); the registered compat rows likewise state affinity format is not equivalent to the cache key. The opi-ai implementation matches the source; the committed test contradicts it and fails at audit_head (observed: all three headers carried 'session-disabled').
- Evidence:
  - `crates/opi-ai/tests/request_enrichment.rs:427-435` — asserts header_value(chat_requests[3], name) == None; probe shows Some("session-disabled") for all three headers
  - `.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts:778-788` — affinity headers emitted on sessionId && sendSessionAffinityHeaders, independent of cache retention
  - `.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts:347-348` — cacheSessionId (prompt cache key) is the only retention-gated session use
  - `crates/opi-ai/src/openai_chat.rs:3034-3065` — affinity header emission unconditional on retention; prompt_cache_key gated by cache_retention != Disabled (lines 1869-1881)
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md#compat-schema (sessionAffinityFormat不可等同cache key)`
- Reproduction: `cargo test -p opi-ai --test request_enrichment session_affinity_wire_mappings -- --exact`
- Refutation attempted: checked the pinned Pi source and the registered compat rows (affinity format is not the cache key); implementation matches both; no registered record requires retention-gated header suppression.
- Confidence: high

### P20-AUD-004: Prepare-path credential failures reach consumers as StreamFailure(CredentialNeeded), defeating typed credential classification and login remediation

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: V-05, AIR-007
- Claim: When authentication resolution fails during prepared-call setup, models.rs wraps the raw CredentialNeeded cause with with_projection/with_partial into ProviderError::StreamFailure(CredentialNeeded) (response_received=false). opi-agent surfaces that wrapped shape to callers, and the product harness provider_auth_failure matches only the unwrapped ProviderError::CredentialNeeded, so interactive login remediation (interactive.rs), RPC auth mapping (rpc.rs) and runner auth exit codes miss prepare-time credential failures. opi-ai itself treats pre-handoff StreamFailure as preserving credential classification (EventStream bookkeeping unwraps it; ChatStream::into_stream_parts unwraps it), so the prepare-boundary wrap escapes that intent. The committed opi-agent acceptance test public_retry_retains_initial_steering_after_auth_preparation_failure fails at audit_head with exactly this shape.
- Evidence:
  - `crates/opi-ai/src/models.rs:1845-1849` — resolve_route_auth error mapped through models_error_cause(error).with_projection(&projection)
  - `crates/opi-ai/src/provider.rs:899-908` — with_partial wraps any non-StreamFailure cause into StreamFailure{response_received:false}
  - `crates/opi-ai/src/models.rs:3545-3560` — EventStream classification unwraps pre-handoff StreamFailure to retain credential classification — the intent the prepare boundary violates
  - `crates/opi-agent/src/agent_loop.rs:620-632,1338` — prepare/start_attempt errors become AgentError::Provider(AgentProviderFailure{error}) unchanged; AgentError::from never maps to AgentError::CredentialNeeded
  - `crates/opi-coding-agent/src/harness.rs:100-118` — provider_auth_failure matches only unwrapped ProviderError::CredentialNeeded/CredentialRevoked/AccountIdMissing
  - `crates/opi-agent/tests/continuation_contract.rs:121-141` — failing test; observed error AgentProviderFailure{error: StreamFailure(CredentialNeeded{provider_id:"mock"})}
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#V-05 (typed acquisition/auth failure classification preserved to callers)`
- Reproduction: `cargo test -p opi-agent --test continuation_contract public_retry_retains_initial_steering_after_auth_preparation_failure -- --exact`
- Refutation attempted: searched all AgentError conversions (none map to the dedicated CredentialNeeded variant from provider errors) and all product consumers of the classification (interactive/rpc/runner); none unwrap StreamFailure; opi-ai's own EventStream/ChatStream layers demonstrate the intended unwrapped classification.
- Confidence: high

### P20-AUD-005: Mistral Conversations adapter sends strict:true with converted schema for unannotated tools, diverging from the pinned Pi 1.3.0 wire

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-S43, ONB-03-20.43
- Claim: MistralConversationsProvider::function_tool_value defaults unannotated tools to strict:true (unwrap_or(true)) and sends the strict-converted schema, while the pinned Pi 1.3.0 mistral-conversations toFunctionTools sends 'strict: strict ?? false' (false) with unconverted parameters for unannotated tools. The Rust doc comment misattributes the true default to Pi ('pi toFunctionTools: the strict projection defaults on'), the crate README documents the true default as opi behavior, and a committed test (tools_project_strict_default_true) locks the divergence in. No registered rust-differences record authorizes this wire deviation, so every unannotated tool on this adapter is enforced differently from the pinned source.
- Evidence:
  - `crates/opi-ai/src/mistral.rs:446-475` — function_tool_value: resolve_json_schema_strict_sampling(...).unwrap_or(true); parameters strict-converted; doc comment misattributes Pi
  - `.repo/pi-1.3.0/packages/ai/src/api/mistral-conversations.ts:777-789` — pinned source: 'const strict = resolveJsonSchemaStrictSampling(tool, true); ... strict: strict ?? false'
  - `crates/opi-ai/tests/mistral_fixtures.rs:1135-1167` — tools_project_strict_default_true asserts function.strict == true (locks divergence, passes)
  - `crates/opi-ai/README.md (Mistral Conversations section)` — documents 'Tools project strict: true by default with the converted schema' as opi behavior
  - `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` — no registered difference records a strict-default deviation for mistral-conversations
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md#mistral-conversations (pinned adapter wire semantics)`
- Reproduction: `cargo test -p opi-ai --test mistral_fixtures tools_project_strict_default_true -- --exact (locks the divergence)`
- Refutation attempted: verified the pinned mistral-conversations source sends strict:false; searched rust-differences for a registered deviation (none); the README presents strict:true as opi behavior, so the divergence is real and unregistered rather than source-conformant.
- Confidence: high

### P20-AUD-006: Registered-source hash drift: acceptance-matrix.md committed bytes no longer match either ledger's stored hash

- Axis: invariants
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: V-59, AIR-067, P20-S59, ONB-07-20.59
- Claim: The scope reader reports acceptance-matrix.md stored_sha256 0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0 versus actual c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d in both .opi-impl-state.json and docs/snapshots/phase20/opi-impl-state.json. The matrix last changed in commit b3f1810 while the ledger was last written at 6adacc3, so the P.0/P.2/P.4 confirmed reconciliation required before task 20.59 passes or F.1a runs is pending. This is the workflow's designed stop state, recorded here so the pending reconciliation is explicit.
- Evidence:
  - `.opi-impl-state.json + docs/snapshots/phase20/opi-impl-state.json (spec_files_sha256)` — stored hash 0c05c6a7... vs committed bytes c8315863... (reader stored_hash_mismatches)
  - `git history (paths only)` — acceptance-matrix.md last touched by b3f1810; ledgers last written at 6adacc3 (earlier)
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#DEL-14 (registered-source drift reconciliation P.0/P.2/P.4 before passing/F.1a)`
- Reproduction: `python .agents/skills/opi-audit/scripts/read_audit_scope.py --phase 20 (observe stored_hash_mismatches)`
- Confidence: high

### P20-AUD-007: Delivery-evidence baseline is stale relative to audit_head: full-gate/native/package outcomes were recorded on superseded trees and the current tree fails its own suite

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: V-58, P20-S58, P20-EXIT-06, ONB-06-20.58, P20-RDF-21, ONB-02-20.57, DEL-14, P20-S59, ONB-07-20.59
- Claim: phase20-delivery-evidence.md records its outcomes against the tree at the 20.57 checkpoint plus 20.58's changes, including a full workspace smoke PASS and Windows 2629/2630. After that evidence was recorded, production code in crates/opi-ai changed again (commit b3f1810 also touches the matrix), and at audit_head cargo test -p opi-ai fails 4 targets and opi-agent fails 1 (findings 001-004) — so the recorded full-gate/package/native outcomes no longer describe the delivered bytes. The Linux/MSRV slice was re-verified during this audit (platform_delivery 9/9, external_sdk 10/10, fmt/clippy/doc gates green on rustc 1.97.1), but the Windows/package evidence remains tied to the superseded tree, macOS stays OPEN, and the recorded 'criteria met' exit state cannot hold for the current bytes without rerunning the affected evidence.
- Evidence:
  - `docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md (Input identity table)` — evidence tree = 20.57 checkpoint 34936a5 + that task's changes; full smoke PASS and Windows run recorded there
  - `git history (paths only)` — b3f1810 changed crates/ and the matrix after the evidence runs and after the ledger's last write
  - `audit execution logs` — at audit_head: opi-ai 2374p/4f (4 targets), opi-agent 605p/1f (continuation_contract); fmt/clippy/doc-check/doc-tests/unittest green
- Criterion source: `docs/research/pi-ai-opi-ai-alignment/spec-draft.md#DEL-14 (actual final identities; executed evidence must describe the delivered tree)`
- Reproduction: `cargo test -p opi-ai --all-targets --no-fail-fast`; `cargo test -p opi-agent --all-targets --no-fail-fast`
- Refutation attempted: re-executed the Linux/MSRV slice at audit_head (green except findings 001-004), confirming the stale evidence is not merely missing local proof; the recorded Windows/package outcomes are tied to superseded trees by their own input-identity table.
- Confidence: high

## Verification Commands

| `cargo test -p opi-ai --all-targets --no-fail-fast (export, MSRV 1.97.1)` | FAIL — 2374 passed / 4 failed | P20-AUD-001, P20-AUD-002, P20-AUD-003; most met requirements |
| `cargo test -p opi-ai --test http_acceptance <redirect test> -- --exact (x2)` | FAIL (deterministic) | P20-AUD-001 |
| `cargo test -p opi-ai --test mistral_fixtures mistral_profile_inherits_shared_compat_flags -- --exact` | FAIL | P20-AUD-002 |
| `cargo test -p opi-ai --test openrouter_fixtures openrouter_profile_inherits_shared_compat_flags -- --exact` | FAIL | P20-AUD-002 |
| `cargo test -p opi-ai --test request_enrichment session_affinity_wire_mappings -- --exact` | FAIL | P20-AUD-003 |
| `cargo test -p opi-agent --all-targets --no-fail-fast` | FAIL — 605 passed / 1 failed | P20-AUD-004 |
| `cargo test -p opi-coding-agent (9 named targets)` | PASS — 313/313 | 20.34/20.57 consumer rows |
| `cargo test -p opi-ai --doc` | PASS — 3/3 | 20.56+ rows |
| `cargo fmt --check --all` | PASS | standards |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | standards |
| `python3 scripts/opi-doc-check.py` | PASS | 20.59 rows |
| `python3 -B -m unittest discover -s scripts/pi-sdk-conformance` | PASS — 82 OK | AIR-036, P20-S16, ONB-03-20.16 |
| `git diff --check (clean tree at audit_head)` | PASS | 20.59 rows |
| `package_slice / run_early_package_proof / run_readme_examples` | NOT RUN (recorded evidence predates audit_head production changes) | P20-AUD-007; P20-RDF-21, ONB-02-20.57, V-58 |
| `native macOS / Windows execution at audit_head` | NOT RUN (no owning host / not authorized) | V-58, P20-S58, P20-EXIT-06, ONB-06-20.58 |

## Verdict Rationale

The mechanical member verdict is **FAIL**: 21 mandatory requirement records are `partially-met` (no mandatory record is `not-met` or `not-assessable`), each linked to at least one reciprocal finding. Five blocking Major findings stand on current, reproduced audit_head evidence: three committed acceptance tests contradict the pinned source or transport reality and fail deterministically (P20-AUD-001..003), the prepare-path credential classification breaks the product login-remediation contract (P20-AUD-004), and the Mistral Conversations strict default diverges from the pinned wire without a registered difference (P20-AUD-005). Two further findings record the pending registered-source reconciliation (P20-AUD-006, advisory) and delivery evidence that no longer describes the delivered tree (P20-AUD-007, blocking). The 236 `met` records reflect broad, genuinely executed source-conformant delivery — including the assembled SDK matrix, platform freeze, WS consolidation, auth/OAuth families and telemetry — on the exact declared MSRV toolchain; the phase is close, but its own gates cannot pass at this head.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "44374ec3c70b82125a183757dde2017b46299c45",
  "audit\u005frun\u005fid": "phase20-pi-glm53-44374ec-20261103t160703z",
  "comparison": {
    "comparisons": [
      {
        "cause": "The prior OPEN item was never repaired (no macOS runner/CI dispatch executed), and later post-evidence production changes invalidated the recorded Windows/package evidence's applicability to the delivered tree.",
        "current\u005fevidence": "macOS remains OPEN with no owning host and CI dispatch remains unexecuted at 44374ec3 (delivery-evidence macOS section). The situation worsened after the prior run: production code changed (b3f1810) after the recorded full-gate/package/native evidence was collected, and the current tree fails its own suites (4 opi-ai + 1 opi-agent targets), so the recorded evidence no longer describes the delivered bytes; severity escalated Info-\u003eMajor blocking under P20-AUD-007.",
        "current\u005ffinding\u005fid": "P20-AUD-007",
        "prior\u005fclaim": "Prior P20-AUD-001 (Info, advisory): 'At audit\u005fhead no native macOS or aarch64 platform/MSRV/conformance run and no actual CI dispatch have been executed; the registered delivery contract itself records these as OPEN and assigns full DEL-14/delivery-group closure to the independent F.1a phase-exit gate.' Old closure framing: F.1a-owned OPEN, not a current defect.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
            "finding\u005fid": "P20-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "input-identity table (tree at 20.57 checkpoint plus 20.58 changes) and macOS OPEN section read at audit\u005fhead",
            "limitations": "document identity cited from the committed tree; no rerun of Windows/package proofs was possible in this context",
            "path": "docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md (committed bytes at audit\u005fhead)",
            "sha256": "5217b9f0228d1dad64aa19e23ad4fc591923ba7059b8e3a839046085f8447585"
          }
        ],
        "relationship": "persistent",
        "repair\u005fcommits": [],
        "verification": "Execute the macOS/aarch64 matrix and a real CI dispatch on the current tree, rerun the full workspace gates and packaging proofs at the remediation head, then re-record delivery evidence against the delivered bytes."
      },
      {
        "cause": "The prior audit's task-level aggregate evidence did not discriminate this test; the keep-alive dead-pool failure mode is also host-timing sensitive, but the strict/affinity content failures (P20-AUD-002/003) prove the prior aggregate claim cannot be wholly accurate regardless of host.",
        "current\u005fevidence": "The failing test is deterministic on this host at audit\u005fhead (in-suite plus isolated reruns). Isolated execution of the same test on a git-archive export of the prior audit\u005fhead 6adacc37 also FAILS (0 passed / 1 failed), so the defect predates the prior report and was omitted by it; the prior aggregate '0 failed' claim is not reproducible on this host.",
        "current\u005ffinding\u005fid": "P20-AUD-001",
        "prior\u005fclaim": "No prior finding covered the default-client redirect-budget acceptance test. The prior run recorded T20.12 as met with 'aggregate 2629 passed / 0 failed' for opi-ai integration suites at 6adacc37.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "cargo test -p opi-ai --test http\u005facceptance source\u005fqualified\u005fdefault\u005fredirect\u005fbudget\u005fhas\u005fexact\u005ftwenty\u005fhop\u005fboundary -- --exact on the 6adacc37 export",
            "limitations": "single-host (WSL2 Linux) execution; raw log retained privately under /tmp",
            "path": "/tmp/opi-p20-prior-redirect.log",
            "sha256": "ecd6a0e031e78b45f4a607b92f3c30c5a472e797833d540a2f0a92c309376f29"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Fix the scripted server to advertise Connection: close (or disable pooling for the default-client leg), then rerun the target on both native hosts."
      },
      {
        "cause": "The tests contradict the pinned Pi source (strict: strict ?? false), the shipped adapter and the README; they were committed before the prior audit and the prior audit's aggregate evidence did not execute or did not report them faithfully.",
        "current\u005fevidence": "Both content assertions (tools[0].function.strict == true) fail deterministically at audit\u005fhead and also fail on an isolated export of prior audit\u005fhead 6adacc37 (0 passed / 1 failed each). These are host-independent content assertions: the prior '0 failed' aggregate cannot have included green runs of these tests at that head.",
        "current\u005ffinding\u005fid": "P20-AUD-002",
        "prior\u005fclaim": "No prior finding covered the shared-compat strict projection tests. The prior run recorded T20.43/T20.51/T20.34 as met with 'aggregate 2629 passed / 0 failed'.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "cargo test -p opi-ai --test mistral\u005ffixtures mistral\u005fprofile\u005finherits\u005fshared\u005fcompat\u005fflags -- --exact on the 6adacc37 export",
            "limitations": "raw log retained privately under /tmp",
            "path": "/tmp/opi-p20-prior-mistral.log",
            "sha256": "c6edbe3326bdabb95450d54459246654840b862bf19a14cc7e978ee3f9855b8f"
          },
          {
            "checked": "cargo test -p opi-ai --test openrouter\u005ffixtures openrouter\u005fprofile\u005finherits\u005fshared\u005fcompat\u005fflags -- --exact on the 6adacc37 export",
            "limitations": "raw log retained privately under /tmp",
            "path": "/tmp/opi-p20-prior-or.log",
            "sha256": "570877798d85e973f1017228e43db372062d9e5e0cde6249e7eb37c21dbe4914"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Update both fixture tests to the source-conformant strict:false expectation (or gate on explicit constrained-sampling annotations) and rerun the two targets."
      },
      {
        "cause": "The test encodes an opi-invented coupling (retention gates affinity headers) that neither the pinned source (headers unconditional; only cacheSessionId gated) nor the registered compat rows (affinity format is not the cache key) support; committed before the prior audit and not discriminated by it.",
        "current\u005fevidence": "session\u005faffinity\u005fwire\u005fmappings fails deterministically at audit\u005fhead (probe: all three affinity headers carry the session id under CacheRetention::Disabled) and also fails on the isolated 6adacc37 export (0 passed / 1 failed). Host-independent content assertion; the prior aggregate claim is contradicted.",
        "current\u005ffinding\u005fid": "P20-AUD-003",
        "prior\u005fclaim": "No prior finding covered the session-affinity mapping test. The prior run recorded T20.34 as met with 'aggregate 2629 passed / 0 failed'.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "cargo test -p opi-ai --test request\u005fenrichment session\u005faffinity\u005fwire\u005fmappings -- --exact on the 6adacc37 export",
            "limitations": "raw log retained privately under /tmp",
            "path": "/tmp/opi-p20-prior-aff.log",
            "sha256": "f91d23b99fa0ce4e1ae3625919dd0d0b0bdd60bdf0140daa83e2bd59322468bc"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Align the test with the pinned source (headers retained under Disabled retention; only prompt\u005fcache\u005fkey suppressed) and rerun the target."
      },
      {
        "cause": "with\u005fprojection/with\u005fpartial wrapping at the prepare boundary escaped the pre-handoff unwrapping intent that opi-ai itself implements elsewhere; the prior audit did not execute the opi-agent consumer suite that discriminates it.",
        "current\u005fevidence": "public\u005fretry\u005fretains\u005finitial\u005fsteering\u005fafter\u005fauth\u005fpreparation\u005ffailure fails at audit\u005fhead (observed AgentProviderFailure{StreamFailure(CredentialNeeded)}) and also fails on the isolated 6adacc37 export (0 passed / 1 failed), so the prepare-boundary wrap predates the prior report.",
        "current\u005ffinding\u005fid": "P20-AUD-004",
        "prior\u005fclaim": "No prior finding covered prepare-path credential classification. The prior run recorded T20.2 as met; its checks cited opi-ai aggregates and did not exercise the opi-agent continuation contract.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "cargo test -p opi-agent --test continuation\u005fcontract public\u005fretry\u005fretains\u005finitial\u005fsteering\u005fafter\u005fauth\u005fpreparation\u005ffailure -- --exact on the 6adacc37 export",
            "limitations": "raw log retained privately under /tmp",
            "path": "/tmp/opi-p20-prior-agent.log",
            "sha256": "d98d8811c4e10832798c4c227065110524ad41fa273857f2dc5a6e01c720abc0"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Unwrap pre-handoff StreamFailure at the prepare boundary (or map credential causes to the dedicated AgentError variants) and rerun the opi-agent suite plus the harness login-remediation paths."
      },
      {
        "cause": "A deliberate opi default (README-documented, test-locked) was never registered as a rust-differences deviation, and its doc comment misattributes the pinned source; the prior audit accepted the adapter family on aggregate evidence without differential wire inspection of this default.",
        "current\u005fevidence": "crates/opi-ai/src/mistral.rs function\u005ftool\u005fvalue contains .unwrap\u005for(true) with the Pi-misattributing doc comment at both audit\u005fhead and the prior head 6adacc37 (verified by reading the prior export), while the pinned Pi 1.3.0 mistral-conversations toFunctionTools sends strict: strict ?? false; the divergence and its locking test (tools\u005fproject\u005fstrict\u005fdefault\u005ftrue, passing) predate the prior report.",
        "current\u005ffinding\u005fid": "P20-AUD-005",
        "prior\u005fclaim": "No prior finding covered the Mistral Conversations strict default; the prior run recorded T20.43 as met.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "grep of the prior export at line 472 confirming unwrap\u005for(true) with the same doc comment",
            "limitations": "prior export retained privately under /tmp; content read only for this bounded comparison",
            "path": "/tmp/opi-audit-p20-prior.wTdpFW/crates/opi-ai/src/mistral.rs (prior-head export copy)",
            "sha256": "b5e0f624646bf2e8ecc3dae4cec2449d3ccc3fd341c84f5c4e1d87324d411310"
          }
        ],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Either align the Conversations strict default to the pinned source or register the deviation in rust-differences and correct the misattributing doc comment, then rerun mistral\u005ffixtures."
      },
      {
        "cause": "b3f1810 appended matrix evidence after the last ledger reconciliation without the DoD's P.0/P.2/P.4 confirmation loop.",
        "current\u005fevidence": "At audit\u005fhead 44374ec3 the committed acceptance-matrix.md hashes to c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d while both ledgers still store 0c05c6a7...; the matrix last changed in b3f1810 after the ledgers' last write (6adacc3 archive). A previously consistent state became drifted.",
        "current\u005ffinding\u005fid": "P20-AUD-006",
        "prior\u005fclaim": "No prior finding covered registered-source hash drift: at 6adacc37 the ledger's stored hash for acceptance-matrix.md (0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0) matched that head's committed bytes (verified by hashing git-show output), i.e. the earlier P.0/P.2/P.4 reconciliation had installed fresh hashes.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [
          {
            "checked": "stored ledger hash versus SHA-256 of the prior head's committed matrix bytes: equal",
            "limitations": "prior-head git-show verification only; current-head mismatch established by the sealed scope reader",
            "path": "git show 6adacc37:docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md (prior-head committed bytes)",
            "sha256": "0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0"
          }
        ],
        "relationship": "regression",
        "repair\u005fcommits": [],
        "verification": "Run the registered P.0/P.2/P.4 drift reconciliation against the current matrix bytes before task 20.59 passes or F.1a runs."
      }
    ],
    "limitations": "- The superseded run's prior P20-AUD-002 (V-59 phase tally 'delete = 0' versus a scoped assertion deletion) was not re-assessed by the current core; it is recorded here as a bounded lead for fresh independent verification, not as a current finding.\n- Prior-head executions used the shared marked Cargo cache and WSL2 Linux host; the redirect failure is timing-sensitive, but the strict/affinity/credential failures are host-independent content assertions, so the prior aggregate '0 failed' claim is contradicted regardless of host.\n- No other history records (other pairs' runs, older runs of this pair, remediation material) were read; raw prior-head logs and exports are retained privately under /tmp and are not part of the repository.",
    "schema": "opi-audit-history/1",
    "scope": "Compared the immediately superseded run phase20-pi-glm53-6adacc37-20261101t164203z of the same (pi, glm53) pair (audit\u005fhead 6adacc379df15ff2e5fa66ddb2afd3abeef5bb89, verdict PASS-WITH-FINDINGS, 68 task-level requirement records all met, 2 advisory findings) against this member's current core sealed at 44374ec3c70b82125a183757dde2017b46299c45. Only that history record was read; the concurrent codex/gpt61 history and older runs stayed outside the comparison. Current behavior remains anchored to audit\u005fhead; prior-head facts were established by executing the prior tree in an isolated export, not by trusting prior conclusions.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
        "commit": "703d8bd8f5033e5ecc89fc1950fc6b7615b64068",
        "id": "prior-meta",
        "path": "docs/snapshots/phase20/assurance/history/phase20-pi-glm53-6adacc37-20261101t164203z/audit.pi.glm53.meta.json",
        "sha256": "7d449f0419ad10484a993b43822e7aa0b9c015a172b1e1134e0e31dcdfb67a4a"
      },
      {
        "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
        "commit": "703d8bd8f5033e5ecc89fc1950fc6b7615b64068",
        "id": "prior-requirements",
        "path": "docs/snapshots/phase20/assurance/history/phase20-pi-glm53-6adacc37-20261101t164203z/audit.pi.glm53.requirements.jsonl",
        "sha256": "3f790100e39e3164bc8248d20b84d273640e41978d66dcc81875f81705898b5d"
      },
      {
        "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
        "commit": "703d8bd8f5033e5ecc89fc1950fc6b7615b64068",
        "id": "prior-findings",
        "path": "docs/snapshots/phase20/assurance/history/phase20-pi-glm53-6adacc37-20261101t164203z/audit.pi.glm53.findings.jsonl",
        "sha256": "fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a"
      },
      {
        "audit\u005frun\u005fid": "phase20-pi-glm53-6adacc37-20261101t164203z",
        "commit": "703d8bd8f5033e5ecc89fc1950fc6b7615b64068",
        "id": "prior-report",
        "path": "docs/snapshots/phase20/assurance/history/phase20-pi-glm53-6adacc37-20261101t164203z/audit.pi.glm53.md",
        "sha256": "233f1d52d075bd463036d20e1d2b6e55cce83dbffab6bad8fc125178ab6d3781"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990",
    "meta\u005fsha256": "f4c4c595f2b4d745a8cf767ee6e2799753040083a5f6566e688175e29ab5f32f",
    "report\u005fbytes": 31641,
    "report\u005fsha256": "5b6ecdf3f072e162cc1433de0d4a35419b6224413b04c89f1bc89868fac614fa",
    "requirements\u005fsha256": "4ffa18d273639122681a2285fe29d51cc2022b5330d54ddd9ac9bbab79bb7bf8"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase20&#45;pi&#45;glm53&#45;44374ec&#45;20261103t160703z
- Audit head: 44374ec3c70b82125a183757dde2017b46299c45
- Core report bytes: 31641
- Core report SHA-256: 5b6ecdf3f072e162cc1433de0d4a35419b6224413b04c89f1bc89868fac614fa
- Current findings SHA-256: f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990
- Comparison status: compared
- Inspected history: Compared the immediately superseded run phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z of the same &#40;pi, glm53&#41; pair &#40;audit&#95;head 6adacc379df15ff2e5fa66ddb2afd3abeef5bb89, verdict PASS&#45;WITH&#45;FINDINGS, 68 task&#45;level requirement records all met, 2 advisory findings&#41; against this member&#39;s current core sealed at 44374ec3c70b82125a183757dde2017b46299c45&#46; Only that history record was read; the concurrent codex/gpt61 history and older runs stayed outside the comparison&#46; Current behavior remains anchored to audit&#95;head; prior&#45;head facts were established by executing the prior tree in an isolated export, not by trusting prior conclusions&#46;
- Limitations: &#45; The superseded run&#39;s prior P20&#45;AUD&#45;002 &#40;V&#45;59 phase tally &#39;delete = 0&#39; versus a scoped assertion deletion&#41; was not re&#45;assessed by the current core; it is recorded here as a bounded lead for fresh independent verification, not as a current finding&#46;<br>&#45; Prior&#45;head executions used the shared marked Cargo cache and WSL2 Linux host; the redirect failure is timing&#45;sensitive, but the strict/affinity/credential failures are host&#45;independent content assertions, so the prior aggregate &#39;0 failed&#39; claim is contradicted regardless of host&#46;<br>&#45; No other history records &#40;other pairs&#39; runs, older runs of this pair, remediation material&#41; were read; raw prior&#45;head logs and exports are retained privately under /tmp and are not part of the repository&#46;

### Pinned historical sources

- prior&#45;meta: docs/snapshots/phase20/assurance/history/phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z/audit&#46;pi&#46;glm53&#46;meta&#46;json; commit 703d8bd8f5033e5ecc89fc1950fc6b7615b64068; SHA-256 7d449f0419ad10484a993b43822e7aa0b9c015a172b1e1134e0e31dcdfb67a4a; audit run phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z
- prior&#45;requirements: docs/snapshots/phase20/assurance/history/phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z/audit&#46;pi&#46;glm53&#46;requirements&#46;jsonl; commit 703d8bd8f5033e5ecc89fc1950fc6b7615b64068; SHA-256 3f790100e39e3164bc8248d20b84d273640e41978d66dcc81875f81705898b5d; audit run phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z
- prior&#45;findings: docs/snapshots/phase20/assurance/history/phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z/audit&#46;pi&#46;glm53&#46;findings&#46;jsonl; commit 703d8bd8f5033e5ecc89fc1950fc6b7615b64068; SHA-256 fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a; audit run phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z
- prior&#45;report: docs/snapshots/phase20/assurance/history/phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z/audit&#46;pi&#46;glm53&#46;md; commit 703d8bd8f5033e5ecc89fc1950fc6b7615b64068; SHA-256 233f1d52d075bd463036d20e1d2b6e55cce83dbffab6bad8fc125178ab6d3781; audit run phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z

### Comparison 1: P20&#45;AUD&#45;007

- Relationship: persistent
- Prior claim (attributed): Prior P20&#45;AUD&#45;001 &#40;Info, advisory&#41;: &#39;At audit&#95;head no native macOS or aarch64 platform/MSRV/conformance run and no actual CI dispatch have been executed; the registered delivery contract itself records these as OPEN and assigns full DEL&#45;14/delivery&#45;group closure to the independent F&#46;1a phase&#45;exit gate&#46;&#39; Old closure framing: F&#46;1a&#45;owned OPEN, not a current defect&#46;
- Current evidence: macOS remains OPEN with no owning host and CI dispatch remains unexecuted at 44374ec3 &#40;delivery&#45;evidence macOS section&#41;&#46; The situation worsened after the prior run: production code changed &#40;b3f1810&#41; after the recorded full&#45;gate/package/native evidence was collected, and the current tree fails its own suites &#40;4 opi&#45;ai &#43; 1 opi&#45;agent targets&#41;, so the recorded evidence no longer describes the delivered bytes; severity escalated Info&#45;&#62;Major blocking under P20&#45;AUD&#45;007&#46;
- Cause / missed boundary: The prior OPEN item was never repaired &#40;no macOS runner/CI dispatch executed&#41;, and later post&#45;evidence production changes invalidated the recorded Windows/package evidence&#39;s applicability to the delivered tree&#46;
- Suggested current verification: Execute the macOS/aarch64 matrix and a real CI dispatch on the current tree, rerun the full workspace gates and packaging proofs at the remediation head, then re&#45;record delivery evidence against the delivered bytes&#46;
- Prior finding: phase20&#45;pi&#45;glm53&#45;6adacc37&#45;20261101t164203z / P20&#45;AUD&#45;001; source prior&#45;findings
- Candidate raw proof: docs/research/pi&#45;ai&#45;opi&#45;ai&#45;alignment/phase20&#45;delivery&#45;evidence&#46;md &#40;committed bytes at audit&#95;head&#41;; SHA-256 5217b9f0228d1dad64aa19e23ad4fc591923ba7059b8e3a839046085f8447585; checked: input&#45;identity table &#40;tree at 20&#46;57 checkpoint plus 20&#46;58 changes&#41; and macOS OPEN section read at audit&#95;head; limits: document identity cited from the committed tree; no rerun of Windows/package proofs was possible in this context

### Comparison 2: P20&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding covered the default&#45;client redirect&#45;budget acceptance test&#46; The prior run recorded T20&#46;12 as met with &#39;aggregate 2629 passed / 0 failed&#39; for opi&#45;ai integration suites at 6adacc37&#46;
- Current evidence: The failing test is deterministic on this host at audit&#95;head &#40;in&#45;suite plus isolated reruns&#41;&#46; Isolated execution of the same test on a git&#45;archive export of the prior audit&#95;head 6adacc37 also FAILS &#40;0 passed / 1 failed&#41;, so the defect predates the prior report and was omitted by it; the prior aggregate &#39;0 failed&#39; claim is not reproducible on this host&#46;
- Cause / missed boundary: The prior audit&#39;s task&#45;level aggregate evidence did not discriminate this test; the keep&#45;alive dead&#45;pool failure mode is also host&#45;timing sensitive, but the strict/affinity content failures &#40;P20&#45;AUD&#45;002/003&#41; prove the prior aggregate claim cannot be wholly accurate regardless of host&#46;
- Suggested current verification: Fix the scripted server to advertise Connection: close &#40;or disable pooling for the default&#45;client leg&#41;, then rerun the target on both native hosts&#46;
- Candidate raw proof: /tmp/opi&#45;p20&#45;prior&#45;redirect&#46;log; SHA-256 ecd6a0e031e78b45f4a607b92f3c30c5a472e797833d540a2f0a92c309376f29; checked: cargo test &#45;p opi&#45;ai &#45;&#45;test http&#95;acceptance source&#95;qualified&#95;default&#95;redirect&#95;budget&#95;has&#95;exact&#95;twenty&#95;hop&#95;boundary &#45;&#45; &#45;&#45;exact on the 6adacc37 export; limits: single&#45;host &#40;WSL2 Linux&#41; execution; raw log retained privately under /tmp

### Comparison 3: P20&#45;AUD&#45;002

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding covered the shared&#45;compat strict projection tests&#46; The prior run recorded T20&#46;43/T20&#46;51/T20&#46;34 as met with &#39;aggregate 2629 passed / 0 failed&#39;&#46;
- Current evidence: Both content assertions &#40;tools&#91;0&#93;&#46;function&#46;strict == true&#41; fail deterministically at audit&#95;head and also fail on an isolated export of prior audit&#95;head 6adacc37 &#40;0 passed / 1 failed each&#41;&#46; These are host&#45;independent content assertions: the prior &#39;0 failed&#39; aggregate cannot have included green runs of these tests at that head&#46;
- Cause / missed boundary: The tests contradict the pinned Pi source &#40;strict: strict ?? false&#41;, the shipped adapter and the README; they were committed before the prior audit and the prior audit&#39;s aggregate evidence did not execute or did not report them faithfully&#46;
- Suggested current verification: Update both fixture tests to the source&#45;conformant strict:false expectation &#40;or gate on explicit constrained&#45;sampling annotations&#41; and rerun the two targets&#46;
- Candidate raw proof: /tmp/opi&#45;p20&#45;prior&#45;mistral&#46;log; SHA-256 c6edbe3326bdabb95450d54459246654840b862bf19a14cc7e978ee3f9855b8f; checked: cargo test &#45;p opi&#45;ai &#45;&#45;test mistral&#95;fixtures mistral&#95;profile&#95;inherits&#95;shared&#95;compat&#95;flags &#45;&#45; &#45;&#45;exact on the 6adacc37 export; limits: raw log retained privately under /tmp
- Candidate raw proof: /tmp/opi&#45;p20&#45;prior&#45;or&#46;log; SHA-256 570877798d85e973f1017228e43db372062d9e5e0cde6249e7eb37c21dbe4914; checked: cargo test &#45;p opi&#45;ai &#45;&#45;test openrouter&#95;fixtures openrouter&#95;profile&#95;inherits&#95;shared&#95;compat&#95;flags &#45;&#45; &#45;&#45;exact on the 6adacc37 export; limits: raw log retained privately under /tmp

### Comparison 4: P20&#45;AUD&#45;003

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding covered the session&#45;affinity mapping test&#46; The prior run recorded T20&#46;34 as met with &#39;aggregate 2629 passed / 0 failed&#39;&#46;
- Current evidence: session&#95;affinity&#95;wire&#95;mappings fails deterministically at audit&#95;head &#40;probe: all three affinity headers carry the session id under CacheRetention::Disabled&#41; and also fails on the isolated 6adacc37 export &#40;0 passed / 1 failed&#41;&#46; Host&#45;independent content assertion; the prior aggregate claim is contradicted&#46;
- Cause / missed boundary: The test encodes an opi&#45;invented coupling &#40;retention gates affinity headers&#41; that neither the pinned source &#40;headers unconditional; only cacheSessionId gated&#41; nor the registered compat rows &#40;affinity format is not the cache key&#41; support; committed before the prior audit and not discriminated by it&#46;
- Suggested current verification: Align the test with the pinned source &#40;headers retained under Disabled retention; only prompt&#95;cache&#95;key suppressed&#41; and rerun the target&#46;
- Candidate raw proof: /tmp/opi&#45;p20&#45;prior&#45;aff&#46;log; SHA-256 f91d23b99fa0ce4e1ae3625919dd0d0b0bdd60bdf0140daa83e2bd59322468bc; checked: cargo test &#45;p opi&#45;ai &#45;&#45;test request&#95;enrichment session&#95;affinity&#95;wire&#95;mappings &#45;&#45; &#45;&#45;exact on the 6adacc37 export; limits: raw log retained privately under /tmp

### Comparison 5: P20&#45;AUD&#45;004

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding covered prepare&#45;path credential classification&#46; The prior run recorded T20&#46;2 as met; its checks cited opi&#45;ai aggregates and did not exercise the opi&#45;agent continuation contract&#46;
- Current evidence: public&#95;retry&#95;retains&#95;initial&#95;steering&#95;after&#95;auth&#95;preparation&#95;failure fails at audit&#95;head &#40;observed AgentProviderFailure&#123;StreamFailure&#40;CredentialNeeded&#41;&#125;&#41; and also fails on the isolated 6adacc37 export &#40;0 passed / 1 failed&#41;, so the prepare&#45;boundary wrap predates the prior report&#46;
- Cause / missed boundary: with&#95;projection/with&#95;partial wrapping at the prepare boundary escaped the pre&#45;handoff unwrapping intent that opi&#45;ai itself implements elsewhere; the prior audit did not execute the opi&#45;agent consumer suite that discriminates it&#46;
- Suggested current verification: Unwrap pre&#45;handoff StreamFailure at the prepare boundary &#40;or map credential causes to the dedicated AgentError variants&#41; and rerun the opi&#45;agent suite plus the harness login&#45;remediation paths&#46;
- Candidate raw proof: /tmp/opi&#45;p20&#45;prior&#45;agent&#46;log; SHA-256 d98d8811c4e10832798c4c227065110524ad41fa273857f2dc5a6e01c720abc0; checked: cargo test &#45;p opi&#45;agent &#45;&#45;test continuation&#95;contract public&#95;retry&#95;retains&#95;initial&#95;steering&#95;after&#95;auth&#95;preparation&#95;failure &#45;&#45; &#45;&#45;exact on the 6adacc37 export; limits: raw log retained privately under /tmp

### Comparison 6: P20&#45;AUD&#45;005

- Relationship: new&#45;omission
- Prior claim (attributed): No prior finding covered the Mistral Conversations strict default; the prior run recorded T20&#46;43 as met&#46;
- Current evidence: crates/opi&#45;ai/src/mistral&#46;rs function&#95;tool&#95;value contains &#46;unwrap&#95;or&#40;true&#41; with the Pi&#45;misattributing doc comment at both audit&#95;head and the prior head 6adacc37 &#40;verified by reading the prior export&#41;, while the pinned Pi 1&#46;3&#46;0 mistral&#45;conversations toFunctionTools sends strict: strict ?? false; the divergence and its locking test &#40;tools&#95;project&#95;strict&#95;default&#95;true, passing&#41; predate the prior report&#46;
- Cause / missed boundary: A deliberate opi default &#40;README&#45;documented, test&#45;locked&#41; was never registered as a rust&#45;differences deviation, and its doc comment misattributes the pinned source; the prior audit accepted the adapter family on aggregate evidence without differential wire inspection of this default&#46;
- Suggested current verification: Either align the Conversations strict default to the pinned source or register the deviation in rust&#45;differences and correct the misattributing doc comment, then rerun mistral&#95;fixtures&#46;
- Candidate raw proof: /tmp/opi&#45;audit&#45;p20&#45;prior&#46;wTdpFW/crates/opi&#45;ai/src/mistral&#46;rs &#40;prior&#45;head export copy&#41;; SHA-256 b5e0f624646bf2e8ecc3dae4cec2449d3ccc3fd341c84f5c4e1d87324d411310; checked: grep of the prior export at line 472 confirming unwrap&#95;or&#40;true&#41; with the same doc comment; limits: prior export retained privately under /tmp; content read only for this bounded comparison

### Comparison 7: P20&#45;AUD&#45;006

- Relationship: regression
- Prior claim (attributed): No prior finding covered registered&#45;source hash drift: at 6adacc37 the ledger&#39;s stored hash for acceptance&#45;matrix&#46;md &#40;0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0&#41; matched that head&#39;s committed bytes &#40;verified by hashing git&#45;show output&#41;, i&#46;e&#46; the earlier P&#46;0/P&#46;2/P&#46;4 reconciliation had installed fresh hashes&#46;
- Current evidence: At audit&#95;head 44374ec3 the committed acceptance&#45;matrix&#46;md hashes to c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d while both ledgers still store 0c05c6a7&#46;&#46;&#46;; the matrix last changed in b3f1810 after the ledgers&#39; last write &#40;6adacc3 archive&#41;&#46; A previously consistent state became drifted&#46;
- Cause / missed boundary: b3f1810 appended matrix evidence after the last ledger reconciliation without the DoD&#39;s P&#46;0/P&#46;2/P&#46;4 confirmation loop&#46;
- Suggested current verification: Run the registered P&#46;0/P&#46;2/P&#46;4 drift reconciliation against the current matrix bytes before task 20&#46;59 passes or F&#46;1a runs&#46;
- Candidate raw proof: git show 6adacc37:docs/research/pi&#45;ai&#45;opi&#45;ai&#45;alignment/acceptance&#45;matrix&#46;md &#40;prior&#45;head committed bytes&#41;; SHA-256 0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0; checked: stored ledger hash versus SHA&#45;256 of the prior head&#39;s committed matrix bytes: equal; limits: prior&#45;head git&#45;show verification only; current&#45;head mismatch established by the sealed scope reader
