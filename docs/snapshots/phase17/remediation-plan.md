# Phase 17 Remediation Plan

**Date**: 2026-08-15
**Mode**: Plan only; execution remains user-gated
**Finding sources**:

- Consumed `docs/snapshots/phase17/audit.codex.md` (`audit`); its normalized blocks report `source_path: docs/snapshots/phase17/audit.gpt5.md`, `source_model: gpt5`, and `independence: unknown`. The filename/path mismatch is retained as source metadata, not silently repaired.
- Consumed `docs/snapshots/phase17/audit.deepseek-v4-pro.md` (`audit`); normalized blocks report the same source path, `source_model: deepseek-v4-pro`, and `independence: independent-family`.
- Consumed `docs/snapshots/phase17/audit.glm5.3.md` (`audit`); its normalized blocks report `source_path: docs/snapshots/phase17/audit.glm5.2.md`, `source_model: glm5.2`, and `independence: fresh-context-same-family`. The filename/path mismatch is retained as source metadata, not silently repaired.

**Commit range**: `41464d8c92313285ce53d9fc40b3cecb153e40e7..a4cfa4ddc74b4dfac59b4305d4657599af866480`
**Verification HEAD**: `877c41fd6c7b0c7850839f41c8fd2824e90436a6`
**Design specs**: `docs/opi-spec.md`; `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`

Source aliases below preserve the normalized-block identity:

- **G** = `docs/snapshots/phase17/audit.gpt5.md` / audit / gpt5 / unknown, consumed from `audit.codex.md`.
- **D** = `docs/snapshots/phase17/audit.deepseek-v4-pro.md` / audit / deepseek-v4-pro / independent-family.
- **L** = `docs/snapshots/phase17/audit.glm5.2.md` / audit / glm5.2 / fresh-context-same-family, consumed from `audit.glm5.3.md`.

---

## Finding cross-reference summary

Fifty-nine normalized findings were ingested and reduced to eighteen behavioral clusters. Coverage counts independent source families, not report files; gpt5's `unknown` and glm5.2's `fresh-context-same-family` metadata provide degraded corroboration but do not create independent votes.

| Cluster | Theme | Sources | Independence | Coverage | Source severity range | Final severity + rationale | Verification |
|---------|-------|---------|--------------|----------|-----------------------|----------------------------|--------------|
| C1 | Trusted tool origin and scoped `command.execute` authorization | G:S01, G:SEC01; L:P7 | unknown + fresh-context-same-family | Correlated/degraded overlap | Blocker / Major / Minor | **Blocker**: origin is inferred from an extension-controlled name, while interactive `Ask` can be authorized before an actual adapter/grant is bound; together they permit a forged built-in `bash` registration to reach an arbitrary implementation. | Confirmed in `harness.rs:1217-1305` and `tool_authority.rs:36-285`; the normal routed bash backend has a later broker check, so SEC01's isolated scope is narrower, but the laundering path makes the bypass reachable. |
| C2 | Atomic provider-route replacement, checked registration, and canonical model-spec parsing | G:S02, G:S07; D:divergent-provider-model-parsers | independent-family + unknown | Single independent source with degraded corroboration | Major / Minor | **Major**: provider replacement can retain a stale resolver/provenance; the parser difference is Minor but belongs to the same route-identity boundary. | Confirmed in `provider_collection.rs:336-415`, `registry.rs:165-191,379-391`, `loop_types.rs:92-103`, and `provider_factory.rs:375-383`. |
| C3 | Production auth resolver and dispatchable-route parity across modes/providers | G:P02; D:active-auth-resolver-dropped-noninteractive-rpc; L:P3, L:P8, L:P9 | independent-family + unknown + fresh-context-same-family | Single independent source with degraded corroboration | Blocker / Major / Minor | **Blocker**: non-interactive and RPC drop the active resolver on common production paths and fall back to mock auth; configured OAuth and some inactive routes are also omitted or assembled incorrectly. | Confirmed in `main.rs:721-744,867-885,1052-1069,1207-1215`, `harness.rs:1404-1429`, and `provider_factory.rs:1513-1540,1806-1830`. |
| C4 | Truthful credential source and fallback provenance | G:S03, G:P01; D:auth-provenance-hardcoded-static; L:P1; L:S4 (provider-table subset) | independent-family + unknown + fresh-context-same-family | Single independent source with degraded corroboration | Major / Minor | **Major**: production performs OAuth/keychain/env resolution but records `Static`/`NotAttempted`, so resolved-execution evidence is false. | Confirmed across `credential_store.rs`, `provider_factory.rs:175-206,1571-1679`, `provider_collection.rs:385-415`, and `evidence.rs:457-491`. |
| C5 | Cancellation-aware auth preparation and drop-safe attempt ownership | G:I02; L:R2 | unknown + fresh-context-same-family | Correlated/degraded overlap | Major / Info | **Major** for uninterruptible credential/OAuth preparation; the dropped-unpolled attempt leak is SDK-edge scope but shares the attempt lifecycle. | Partially confirmed in `provider_collection.rs:385-415,650-705`: auth resolution is awaited outside cancellation and the active slot clears only through stream polling. |
| C6 | Unterminated provider streams must not report success | G:I01; L:R4 (terminal-stream subset) | unknown + fresh-context-same-family | Correlated/degraded overlap | Major / Info | **Major**: a valid but unterminated stream returns an apparently successful Agent turn and leaves incomplete assistant state. | Confirmed in `agent_loop.rs:220-245,714-728`; `drain_to_completion` already rejects the same condition in `provider_collection.rs:580-603`, proving semantic divergence. |
| C7 | Complete-state ownership, dispatchability validation, and compaction transition | G:S06; D:agent-set-model-panics, D:prepare-next-turn-seam-unused-in-production, D:validate-state-misses-dispatchability; L:S3 (state/harness subset) | independent-family + unknown + fresh-context-same-family | Single independent source with degraded corroboration | Major / Minor | **Major**: public piecemeal setters bypass or discard validation, route validation ignores resolver availability, and product compaction still uses post-loop direct message replacement instead of the required complete-state transition. | Partially confirmed: the hook seam itself is required and valid for embedders, but `agent.rs:200-258` and `harness.rs:2390-2452,2810-2856` violate the registered state-ownership/compaction clauses. |
| C8 | Evidence-first authorization ordering and live-health reauthorization | G:P06; D:stale-reauth-reuses-captured-health | independent-family + unknown | Single independent source with degraded corroboration | Major | **Major**: sequential tools execute before prior evidence failures are observed, and the retry path reuses the same stale health snapshot. | Confirmed in `agent_loop.rs:292-365,462-513,1031-1096,1180-1260`. |
| C9 | Typed run/turn/call identity and invocation/session context at authorization | G:S08, G:P08; D:authz-request-string-ids; L:P6 | independent-family + unknown + fresh-context-same-family | Single independent source with degraded corroboration | Minor | **Minor**: authorization still occurs, but its records cannot be joined reliably to the evidence graph and lack the specified session context. | Confirmed in `authority.rs:237-253` and `agent_loop.rs:88-1045` (`run_id: None` and loop-local strings despite minted evidence IDs). |
| C10 | Per-provider-request trusted tool projection | D:aut-008-projection-snapshot-test-missing | independent-family | Single independent source | Minor | **Minor**: projection is computed before the turn loop and lacks the mandated consecutive-request snapshot proof. | Confirmed in `harness.rs:1304-1305` and `agent_loop.rs:80`; policy exclusion at initial registration works, so the finding is limited to freshness and coverage. |
| C11 | Strict, current-run manifest identity and resolved-execution truthfulness | G:S05, G:P04; L:P2, L:P4, L:P5; L:S4 (Debug-digest subset) | unknown + fresh-context-same-family | Correlated/degraded overlap | Major / Minor | **Major**: arbitrary digest strings and an incomplete `require_complete` gate permit finalized evidence that omits or freezes required actual-route, branch, system/tool-schema, policy, budget, artifact, and outcome facts. | Confirmed in `opi-agent/evidence.rs:319-326,1081-1132`, `opi-coding-agent/evidence.rs:284-330,397-491`, and `harness.rs:1523-1560,2530-2572`. |
| C12 | Run-scoped capture, durable immutable finalization, and compaction finalization | G:S04, G:P05, G:P07, G:X01, G:X02 | unknown | Single unknown-independence source | Major | **Major**: fixed files are truncated/replaced, stale manifests survive failed reuse, flush/finalization failures are discarded, in-memory state crosses prompts, and manual compaction can append after finalization. | Confirmed in `opi-agent/evidence.rs:997-1031`, `opi-coding-agent/evidence.rs:92-166`, and `harness.rs:2230-2333,2497-2572,2810-2856`. |
| C13 | `--trace` behavior across interactive, non-interactive, and RPC modes | G:P03; L:P10 | unknown + fresh-context-same-family | Correlated/degraded overlap | Major / Minor | **Major**: a documented accepted CLI option is silently ignored in two public product modes, contradicting the closed explicit-capture mapping. | Confirmed in `cli.rs:262-269` and `main.rs:949-1088,1090-1222`; only the non-interactive runner receives `cli.trace`. |
| C14 | Preserve typed provider/registry failure classes at the Agent boundary | L:P11 | fresh-context-same-family | Single degraded source | Minor | **Minor**: distinct registry failures collapse into one `RouteNotDispatchable` string variant. | Confirmed in `agent_loop.rs:858-877`. |
| C15 | Neutralize raw upstream stream-error text before model-visible context | L:SEC1 | fresh-context-same-family | Single degraded source | Minor | **Minor**: Gemini/Vertex/Bedrock and malformed-frame paths expose raw upstream text unlike the other provider wires. | Confirmed in `gemini.rs:409-417,746-749`, `bedrock/mod.rs:868-876`, and the Agent stream-error-to-context path. |
| C16 | Resume persistence failures must remain visible | L:R1 | fresh-context-same-family | Single degraded source | Minor | **Minor**: `.ok()` converts reopen failure into a session-less harness that silently stops persisting new turns. | Confirmed in `harness.rs:1492-1510,1918-1931`; the fork/branch path already propagates equivalent failures. |
| C17 | Published API/docs truthfulness and finding-scoped surface cleanup | D:trace-flag-undocumented-in-readmes; L:S1, L:S2, L:S3, L:S4 | independent-family + fresh-context-same-family | Single independent source with degraded corroboration | Minor | **Minor**: bilingual READMEs contain a non-compiling removed API, source docs describe removed ownership, dead Phase 17 surfaces remain, policy digests use unstable `Debug`, and one dependency is unused. Broad duplication-only refactors are excluded. | Partially confirmed by full reads and repository-wide consumer searches; the concrete stale/dead/digest/dependency claims reproduce, while several duplication observations are style-only. |
| C18 | Phase-exit test quality and external-cache workspace gate | G:T01, G:T02; D:evd-005-canary-channels-incomplete, D:fal-001-typed-classes-tautology, D:a05-prepare-cancel-leg-missing, D:plt-002-token-scan; L:T1, L:T2 | independent-family + unknown + fresh-context-same-family | Single independent source with degraded corroboration | Major / Minor | **Major**: the required external Cargo cache makes `doctor_cli` search the wrong binary path, and the named A14/FAL/RBK/canary assurances do not exercise their claimed conjunctions. | Partially confirmed in the cited tests; several behaviors have separate lower-level coverage, but the phase-exit assertions and external-cache lookup remain insufficient. |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|-------------------|----------|-----------|------------|
| D1 | C1 | Preserve registration origin before tool vectors are combined; derive built-in identity only from product-owned construction. Resolve `command.execute` from validated arguments to the actual adapter/path/operation scope, obtain any interactive broker grant, then issue the final `Allow`. | This is the exact order and ownership required by P17-AUT-001/002/003/005 and P17-OUT-003; it adds no policy language. | auto |
| D2 | C2 | Replace split registration state with one atomic route-registration operation and one canonical `provider:model` parser; remove/narrow unchecked compatibility entry points instead of retaining aliases. | Atomic route state is already the registered Phase 17 collection boundary. The user selected strict Phase 17 API closure. | user (choice 1A) |
| D3 | C3 | Carry the active resolver through every public mode and build every configured dispatchable route with its provider-appropriate OAuth/API-key layering; derive Azure fallback deployment only from Azure configuration. | Task 17.5 already requires one dispatch collection containing each configured dispatchable route. | auto |
| D4 | C4 | Make credential resolution return non-secret typed source/fallback provenance together with prepared auth and preserve it unchanged into evidence. | P17-PRV-004/005 require truthful source and fallback facts; credential store decisions already determine them. | auto |
| D5 | C5 | Race preparation against cancellation before/during resolver I/O and make the attempt slot an RAII lease released when the stream is terminal or dropped. | This closes the existing collection lifecycle without adding policy or another dispatch path. | auto |
| D6 | C6 | Treat EOF before a terminal provider event as a typed provider protocol failure in the Agent loop. | `drain_to_completion` already establishes this semantic; the Agent path must match it. | auto |
| D7 | C7 | Remove/narrow piecemeal public state mutation, validate dispatchability before atomic replacement, and route product compaction through `prepare_next_turn`/complete-state replacement. | The registered design explicitly removes compatibility setters and post-loop direct mutation; the user selected strict Phase 17 API closure. | user (choice 1A) |
| D8 | C8 | Process sequential calls as authorize → emit decision → re-read health/reauthorize if changed → execute → emit outcome before the next call; retain the existing launch boundary for already-authorized parallel calls. | This is the fixed P17-AUT/P17-EVD ordering and preserves in-flight actual outcomes. | auto |
| D9 | C9 | Replace optional/string correlation with typed evidence IDs and a trusted opaque invocation/session context on `ToolAuthorizationRequest`. | The request-content contract is explicit. The user accepted strict public-API closure. | user (choice 1A) |
| D10 | C10 | Recompute provider-visible definitions from trusted registrations immediately before every provider request and snapshot consecutive requests in tests. | P17-AUT-008 states both the behavior and mechanical proof. | auto |
| D11 | C11 | Make digest construction validating/fallible and make manifest completeness exhaustive over current-run facts, including actual route, active branch, system/tool schema, permissions/grants, budgets, outcomes, and finalized artifacts. | The manifest field list and strict-gate behavior are normative. Fallible digest construction is a public API correction covered by choice 1A. | user (choice 1A) |
| D12 | C12 | Treat `--trace PATH` as a capture root, allocate one immutable child directory per run, publish its manifest once after durable record completion, and model manual compaction as a separate correlated finalized run. | The user selected immutable per-run capture. Alternative rejected: treat PATH as a single-run destination and reject reuse, which would unnecessarily restrict RPC/multi-prompt sessions. | user (choice 2A) |
| D13 | C13 | Honor `--trace` in all three public modes using the same evidence adapter and complete-evidence policy mapping. | The CLI already accepts the option and the design explicitly maps CLI `--trace` to required-complete evidence. | auto |
| D14 | C14 | Preserve `RegistryError` variants in closed `AgentError` variants instead of collapsing them into a string. | P17-FAL-001 requires distinguishable typed failure classes. | auto |
| D15 | C15 | Replace upstream/provider-controlled stream-error bodies and malformed-frame excerpts with bounded neutral classifications before constructing model-visible events. | This extends the already-established safe posture of the other provider wires. | auto |
| D16 | C16 | Fail the requested resume/reopen operation visibly; do not continue with a session-less harness. | Silent loss of future persistence contradicts INV-007; the existing fork path demonstrates the fail-visible behavior. | auto |
| D17 | C17 | Apply the strict Phase 17 public-surface cleanup, canonicalize digest inputs, remove the unused dependency, and update every English/Chinese counterpart. Do not perform the audit's broad duplication-only refactors. | The user selected strict API closure; documentation and canonical identity fixes are source-of-truth corrections, while unrelated refactors would exceed minimum change. | user (choice 1A) |
| D18 | C18 | Replace tautological/source-token assertions with production-boundary injections and counters, repair external-target discovery, and add the missing cross-mode/cancellation/canary/fallback snapshots. | These are the mechanical proofs already named by the registered criteria; no product semantics are selected. | auto |

User-choice alternatives recorded:

- **Choice 1**: selected strict Phase 17 closure. Rejected alternative: retain/deprecate compatibility surfaces, which would conflict with the registered removal clauses and require returning to shaping.
- **Choice 2**: selected immutable per-run capture. Rejected alternative: make the trace destination single-use and reject later prompt/RPC runs.

## Remediation layers

### Layer 1: `opi-ai` (provider substrate)

**Verification**:

    powershell -File scripts/opi-impl-smoke.ps1 scoped --crate opi-ai --test provider_collection --test per_request_auth --test auth_contracts --test registry
    cargo test -p opi-ai --test gemini_fixtures --test bedrock_fixtures --test azure_openai_fixtures

#### Fix 1.1: Make route registration atomic and model-spec parsing canonical

- **Finding source**: G/audit/gpt5/unknown S02, S07; D/audit/deepseek-v4-pro/independent-family `divergent-provider-model-parsers`.
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider_collection.rs` ~L320-460; `crates/opi-ai/src/registry.rs` ~L150-191, ~L379-391; `crates/opi-ai/tests/provider_collection.rs`; `crates/opi-ai/tests/registry.rs`
- **Change**: Introduce one checked route-registration value/operation that replaces provider, resolver, auth/provenance metadata, probe state, and compatibility lookup as one unit. Remove or narrow registration entry points that can create lookup-only or stale split-map state. Export one canonical trimmed, non-empty `provider:model` parser for downstream crates.
- **Test plan**: Add replacement tests proving no old resolver/provenance/probe survives, empty IDs fail closed, lookup-only providers remain explicitly non-dispatchable, and padded/empty specs produce one result across all callers.

#### Fix 1.2: Make preparation cancellable and attempt ownership drop-safe

- **Finding source**: G/audit/gpt5/unknown I02; L/audit/glm5.2/fresh-context-same-family R2.
- **Cluster**: C5
- **Decision**: D5
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/provider_collection.rs` ~L385-415, ~L630-705; `crates/opi-ai/tests/provider_collection.rs`; `crates/opi-ai/tests/per_request_auth.rs`
- **Change**: Check pre-cancellation before resolver work, select cancellation against in-progress resolution, and return a typed cancellation without starting a provider attempt. Replace the poll-only release closure with an attempt lease owned by the returned stream so terminal completion and drop both release the one-active-attempt slot exactly once.
- **Test plan**: Add pre-cancel/no-auth-I/O, cancellation-during-blocked-resolution, cancellation-during-attempt, unpolled-stream-drop, and second-attempt-after-drop tests.

#### Fix 1.3: Neutralize upstream-controlled stream errors

- **Finding source**: L/audit/glm5.2/fresh-context-same-family SEC1.
- **Cluster**: C15
- **Decision**: D15
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/gemini.rs` ~L400-420, ~L735-755; `crates/opi-ai/src/vertex.rs` shared Gemini mapping; `crates/opi-ai/src/bedrock/mod.rs` ~L855-885; `crates/opi-ai/src/azure_openai.rs` malformed-frame path
- **Change**: Map provider exception frames and malformed upstream data to stable neutral error classes. Keep raw bodies out of `AssistantStreamEvent::Error`, diagnostics, and assistant context; retain only already-classified metadata needed for retry/error typing.
- **Test plan**: Extend Gemini/Vertex/Bedrock/Azure fixtures with secret-bearing error bodies and assert the canary is absent from stream events and serialized provider errors.

### Layer 2: `opi-agent` (Agent Core)

**Verification**:

    powershell -File scripts/opi-impl-smoke.ps1 scoped --crate opi-agent --test phase17_prepare_call --test hooks_queues --test tool_authority --test evidence_runtime --test evidence_contract --test agent_loop_semantics
    cargo test -p opi-agent --test agent_wrapper --test session_facade

#### Fix 2.1: Close complete-state ownership and dispatchability validation

- **Finding source**: G/audit/gpt5/unknown S06; D/audit/deepseek-v4-pro/independent-family `agent-set-model-panics`, `prepare-next-turn-seam-unused-in-production`, `validate-state-misses-dispatchability`; L/audit/glm5.2/fresh-context-same-family S3 (state/harness subset).
- **Cluster**: C7
- **Decision**: D7
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-agent/src/agent.rs` ~L190-270; `crates/opi-agent/src/loop_types.rs` ~L80-110; `crates/opi-agent/src/state.rs`; `crates/opi-agent/src/harness.rs`; `crates/opi-agent/tests/hooks_queues.rs`; `crates/opi-agent/tests/agent_wrapper.rs`
- **Change**: Retain one fallible complete `NextTurnState` replacement operation and remove/narrow piecemeal model/inference/message mutators that panic or discard validation. Use the Layer 1 canonical parser and a collection dispatchability check that verifies a resolver-backed route without performing auth I/O. Remove the orphaned Phase-guard harness/state surfaces named by S3 when repository consumers are absent.
- **Test plan**: Add compile/API audit coverage for removed surfaces; assert invalid/lookup-only route, prepare error, and prepare cancellation leave full state byte-equivalent; assert a valid cross-provider replacement persists through the next public prompt.

#### Fix 2.2: Carry typed evidence identity and trusted invocation context into authorization

- **Finding source**: G/audit/gpt5/unknown S08, P08; D/audit/deepseek-v4-pro/independent-family `authz-request-string-ids`; L/audit/glm5.2/fresh-context-same-family P6.
- **Cluster**: C9
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/authority.rs` ~L230-260; `crates/opi-agent/src/agent_loop.rs` ~L80-100, ~L1031-1096; `crates/opi-agent/tests/tool_authority.rs`; `crates/opi-agent/tests/evidence_runtime.rs`
- **Change**: Replace optional/string run, turn, and call fields with the minted `RunId`, `TurnId`, and `CallId`; add a product-supplied opaque trusted invocation/session context that cannot be populated from model content. Reuse the same IDs for authorization evidence and tool outcome records.
- **Test plan**: Join authorization and evidence records by typed IDs across retries and multiple tools; assert session context is present when supplied, absent only through an explicit no-session variant, and cannot be forged through arguments.

#### Fix 2.3: Recompute the provider-facing tool projection for each request

- **Finding source**: D/audit/deepseek-v4-pro/independent-family `aut-008-projection-snapshot-test-missing`.
- **Cluster**: C10
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` ~L70-95 and provider-request construction; `crates/opi-agent/tests/agent_loop_semantics.rs`; `crates/opi-agent/tests/tool_authority.rs`
- **Change**: Move trusted-registry definition projection from run initialization to immediately before each provider request, after the current complete state/policy is known. Do not cache a projection across consecutive requests.
- **Test plan**: Capture two consecutive mock-provider requests around a policy/registration-state change and assert exact ordered schemas; an excluded registration must be absent from the request in which it is unavailable.

#### Fix 2.4: Enforce evidence-first authorization and live-health launch checks

- **Finding source**: G/audit/gpt5/unknown P06; D/audit/deepseek-v4-pro/independent-family `stale-reauth-reuses-captured-health`.
- **Cluster**: C8
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` ~L270-520, ~L1031-1260; `crates/opi-agent/tests/evidence_runtime.rs`; `crates/opi-agent/tests/tool_authority.rs`
- **Change**: For sequential calls, emit the authorization outcome before execution, re-read live evidence health, rebuild the request when generation changed, and only then cross the launch boundary. Emit each outcome before considering the next call. For parallel batches, authorize deterministically and launch only decisions current at their individual launch boundary; do not retroactively rewrite in-flight outcomes.
- **Test plan**: Add two-tool counters proving tool 2 stays at zero after tool 1 evidence failure, stale `Allow` is reissued against a new generation and denied, authorizer `Err` executes zero tools, and already-launched parallel work retains its actual partial/cleanup outcome.

#### Fix 2.5: Reject unterminated streams and preserve typed collection failures

- **Finding source**: G/audit/gpt5/unknown I01; L/audit/glm5.2/fresh-context-same-family P11 and R4 (terminal-stream subset).
- **Cluster**: C6, C14
- **Decision**: D6, D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` ~L210-245, ~L714-728, ~L836-877; `crates/opi-agent/src/loop_types.rs` ~L1-75; `crates/opi-agent/tests/agent_loop_semantics.rs`; `crates/opi-agent/tests/phase17_prepare_call.rs`
- **Change**: Track whether a terminal provider event was observed and return a typed protocol failure on EOF otherwise. Map invalid spec, unknown provider/model, non-dispatchable route, auth failure, cancellation, and provider protocol failure into distinct closed `AgentError` variants without string collapsing.
- **Test plan**: Drive each production boundary with a mock failure and match the returned class; assert non-terminal deltas followed by EOF fail and never persist a successful assistant turn.

#### Fix 2.6: Validate digest identity, strict manifest completeness, and in-memory run reset

- **Finding source**: G/audit/gpt5/unknown S05, P04, X02; L/audit/glm5.2/fresh-context-same-family P2, P4, P5 and S4 (Debug-digest subset).
- **Cluster**: C11, C12
- **Decision**: D11, D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/evidence.rs` ~L300-330, ~L870-1035, ~L1080-1135; `crates/opi-agent/tests/evidence_contract.rs`; `crates/opi-agent/tests/evidence_runtime.rs`
- **Change**: Make `ContentDigest` accept only canonical SHA-256 hex through a fallible constructor. Make `InMemoryEvidenceSink::setup` clear records, artifacts, manifest, binding, and failure state for one run. Extend `FinalizedManifest::require_complete` to reject `Incomplete` and every missing/unknown field required by P17-EVD-003, while still permitting explicitly typed unknown measurements only where the design allows them.
- **Test plan**: Add malformed digest cases, two-run sink reuse with distinct run IDs, stale-manifest rejection, and a table test that removes each required manifest field one at a time and expects a typed completeness error.

### Layer 3: `opi-coding-agent` (Reference Product)

**Verification**:

    powershell -File scripts/opi-impl-smoke.ps1 scoped --crate opi-coding-agent --test phase17_provider_runtime --test phase17_tool_authority --test phase17_product_evidence --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit --test phase17_artifact_truthfulness --test doctor_cli --test session_runtime
    cargo test -p opi-coding-agent --test phase17_legacy_migration

#### Fix 3.1: Preserve trusted origin and bind command authorization to the reached adapter

- **Finding source**: G/audit/gpt5/unknown S01, SEC01; L/audit/glm5.2/fresh-context-same-family P7 and S4 (tool-inventory subset).
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L1217-1305; `crates/opi-coding-agent/src/tool_authority.rs` ~L30-70, ~L200-285; `crates/opi-coding-agent/src/execution/`; `crates/opi-coding-agent/tests/phase17_tool_authority.rs`
- **Change**: Build product built-ins and extension registrations separately. Assign built-in identity only while traversing the product-owned built-in table; derive extension origin/namespace from `ExtensionRegistry` and exclude unpermitted extension capabilities from projection. For `bash`, run the existing pure router over the validated final arguments, check the reached adapter's permission, obtain an interactive scoped grant through the broker when required, and populate `permission_ref`/`permission_scope` from that actual decision before `Tool::execute`.
- **Test plan**: Add malicious extension tools named `read`, `write`, and `bash`; all must retain extension origin and execute zero times without exact permission. Cover actual non-local adapter Allow/Deny/Ask, stale/mismatched grant, path/operation scope, and headless zero-execution cases.

#### Fix 3.2: Preserve the production resolver in every mode and build all valid routes

- **Finding source**: G/audit/gpt5/unknown P02; D/audit/deepseek-v4-pro/independent-family `active-auth-resolver-dropped-noninteractive-rpc`; L/audit/glm5.2/fresh-context-same-family P3, P8, P9; L:R4 (dummy-resolver subset).
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/main.rs` ~L721-744, ~L754-1088, ~L1090-1222; `crates/opi-coding-agent/src/runner.rs` ~L160-280; `crates/opi-coding-agent/src/rpc.rs` constructors; `crates/opi-coding-agent/src/harness.rs` ~L704-960, ~L1399-1429; `crates/opi-coding-agent/src/provider_factory.rs` ~L1430-1540, ~L1800-1835
- **Change**: Pass `ProviderBundle.auth_resolver` through the non-interactive and RPC constructors rather than dropping it in `with_provider_bundle`; remove production access to the mock-auth fallback. Register configured GitHub Copilot/OpenAI Codex OAuth routes, preserve Anthropic's layered OAuth/API-key resolver when inactive, and reject or derive Azure's deployment solely from Azure-owned configuration/catalog facts.
- **Test plan**: Use a provider that asserts the exact resolved secret/provenance to exercise text, JSON, RPC, and interactive startup; add inactive OAuth/Anthropic/Azure route-switch tests and a production-constructor test proving no `opi-mock-auth` fallback is reachable.

#### Fix 3.3: Carry truthful credential source and fallback facts

- **Finding source**: G/audit/gpt5/unknown S03, P01; D/audit/deepseek-v4-pro/independent-family `auth-provenance-hardcoded-static`; L/audit/glm5.2/fresh-context-same-family P1 and S4 (provider-table subset).
- **Cluster**: C4
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs`; `crates/opi-coding-agent/src/provider_factory.rs` ~L175-206, ~L1570-1680; `crates/opi-coding-agent/src/evidence.rs` ~L450-495; `crates/opi-coding-agent/tests/phase17_provider_runtime.rs`; `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
- **Change**: Convert the credential store's known OAuth/keychain/config/env choice and allowed-fallback result into Layer 1's typed auth resolution. Remove hardcoded `Static`/`NotAttempted` defaults from production routes and preserve the resolver result through provider evidence and final manifest assembly without including secret material.
- **Test plan**: Add positive fixtures for OAuth, keychain, config key, allowed env fallback with typed reason, denied env fallback, and no-fallback; assert evidence distinguishes every source and never contains the credential.

#### Fix 3.4: Build manifests from the current run's resolved facts

- **Finding source**: G/audit/gpt5/unknown P04, X02; L/audit/glm5.2/fresh-context-same-family P2, P4, P5 and S4 (Debug-digest subset).
- **Cluster**: C11
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/evidence.rs` ~L180-335, ~L350-515; `crates/opi-coding-agent/src/harness.rs` ~L1310-1335, ~L1523-1560, ~L2497-2572; `crates/opi-coding-agent/src/session_coordinator.rs`; `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
- **Change**: Rebuild the binding and static facts at each run setup from the applied model/inference state, canonical policy inputs, system prompt, exact trusted tool schemas, known budgets, platform/environment allowlist, and active session branch tip. Capture provider-reported actual provider/model/response-model after the response instead of copying resolved route. Map partial-side-effect and cleanup-unknown execution outcomes, scoped grants, and finalized artifact references into the manifest; make unknown values carry only permitted typed reasons.
- **Test plan**: Reuse one harness across two prompts with a cross-provider/branch switch and assert distinct run IDs/bindings; add mismatched response metadata, system/tool-schema digest, active branch, grant/scope, budget, partial/cleanup, and artifact-finalization fixtures.

#### Fix 3.5: Make capture durable, immutable, and one-run scoped

- **Finding source**: G/audit/gpt5/unknown S04, P05, P07, X01, X02.
- **Cluster**: C12
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/evidence.rs` ~L40-175; `crates/opi-coding-agent/src/harness.rs` ~L2230-2333, ~L2497-2572, ~L2810-2856; `crates/opi-coding-agent/src/rpc.rs`; `crates/opi-coding-agent/tests/phase17_product_evidence.rs`
- **Change**: Make the configured path a capture root and create a unique immutable child for every prompt/continue/retry/RPC/manual-compaction run. Clear/create run-local recorder state before work, propagate write/flush/finalize errors as typed observable failures, flush/sync records before atomically publishing `manifest.json`, and never truncate or replace a prior run. Keep automatic turn compaction before that run's finalization; start and finalize a separate parent-correlated compaction-only run for manual compaction, rejecting any emission after a manifest is published.
- **Test plan**: Run two prompts plus manual compaction through one root and assert three immutable directories, no cross-run records, exact terminal correlations, and unchanged prior bytes. Inject write, flush, sync, atomic-publish, and finalize failures; each must retain the actual execution outcome, mark evidence incomplete, and expose no finalized manifest for the failed run.

#### Fix 3.6: Honor `--trace` in interactive and RPC modes

- **Finding source**: G/audit/gpt5/unknown P03; L/audit/glm5.2/fresh-context-same-family P10.
- **Cluster**: C13
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/cli.rs` ~L260-270; `crates/opi-coding-agent/src/main.rs` ~L949-1222; `crates/opi-coding-agent/src/rpc.rs` ~L120-335; `crates/opi-coding-agent/src/runner.rs` ~L170-270; `crates/opi-coding-agent/tests/phase17_cross_mode.rs`
- **Change**: Thread the same file-capture configuration into interactive and binary RPC construction, make complete-evidence policy true in every traced mode, and surface setup/finalization failures through each mode's existing error/event channel.
- **Test plan**: Run text, JSON, RPC, and interactive launch seams with temporary trace roots; assert identical required-complete behavior and finalized per-run artifacts, plus zero provider/tool calls after setup failure in every mode.

#### Fix 3.7: Fail resume/reopen visibly

- **Finding source**: L/audit/glm5.2/fresh-context-same-family R1.
- **Cluster**: C16
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L1492-1510, ~L1918-1931; `crates/opi-coding-agent/tests/session_runtime.rs`
- **Change**: Replace `.ok()` on requested `SessionCoordinator::open_existing` operations with a propagated typed construction/resume error. Keep optional new-session persistence behavior distinct; a user-requested resume must never degrade to a session-less harness.
- **Test plan**: Inject missing/corrupt/unwritable session reopen failures and assert the operation fails before a new turn/provider call; verify successful resume continues appending to the original session.

#### Fix 3.8: Replace weak Phase 17 assurances with production-boundary tests

- **Finding source**: G/audit/gpt5/unknown T01, T02; D/audit/deepseek-v4-pro/independent-family `evd-005-canary-channels-incomplete`, `fal-001-typed-classes-tautology`, `a05-prepare-cancel-leg-missing`, `plt-002-token-scan`; L/audit/glm5.2/fresh-context-same-family T1, T2.
- **Cluster**: C18
- **Decision**: D18
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_cross_mode.rs`; `phase17_failure_rollback.rs`; `phase17_product_evidence.rs`; `phase17_api_audit.rs`; `phase17_artifact_truthfulness.rs`; `doctor_cli.rs` ~L1270-1290; `crates/opi-agent/tests/hooks_queues.rs`; `crates/opi-agent/tests/tool_authority.rs`; `crates/opi-ai/tests/provider_collection.rs`
- **Change**: Make A14 one auth-aware/evidence-aware conjunction across public modes; replace FAL enum self-matches with real injected boundary failures; cancel an in-flight `prepare_next_turn` and compare the full prior state; add authorizer-`Err`, env-fallback, and mid-attempt cancellation counters; plant canaries in prompt, arguments, environment, provider-error, diagnostic, and artifact-metadata channels; make rollback fixtures touch the actual recorder/policy/session artifacts they claim; replace the network token scan with explicit mock/local transport injection and a fail-on-escape provider seam; resolve the test binary from Cargo's actual target/current executable rather than `<workspace>/target/debug`.
- **Test plan**: Modify the named binaries so each test fails when its production behavior is removed; run every exact modified binary through the Layer 3 scoped gate, including with the repository external Cargo cache active.

### Layer 4: Documentation and metadata

**Verification**:

    python scripts/opi-doc-check.py
    cargo test -p opi-ai --doc
    cargo test -p opi-agent --doc
    rg -n "Provider::stream|AgentHarness|tool registration helpers" README.md README.zh.md crates/opi-ai/README.md crates/opi-ai/README.zh.md crates/opi-agent/README.md crates/opi-agent/README.zh.md crates/opi-ai/src crates/opi-agent/src

#### Fix 4.1: Synchronize the published Phase 17 API and capture documentation

- **Finding source**: D/audit/deepseek-v4-pro/independent-family `trace-flag-undocumented-in-readmes`; L/audit/glm5.2/fresh-context-same-family S1.
- **Cluster**: C17
- **Decision**: D17
- **Verification status**: Confirmed
- **File(s)**: `README.md`; `README.zh.md`; `crates/opi-ai/README.md`; `crates/opi-ai/README.zh.md`; `crates/opi-agent/README.md`; `crates/opi-agent/README.zh.md`; `CHANGELOG.md` under `Unreleased`
- **Change**: Replace the removed `Provider::stream` and tool-registration examples with collection preparation/attempt and trusted-authority APIs; add the `authority` module; document `--trace PATH` as a per-run capture root in every public mode; document legacy route normalization and the intentional Phase 17 breaking removals. Update English/Chinese counterparts in lockstep and record user-visible fixes only under `Unreleased`.
- **Test plan**: Run doc tests and `opi-doc-check`; ensure README examples use only exported current APIs and EN/ZH contract sections remain synchronized.

#### Fix 4.2: Correct stale source docs and remove finding-scoped dead metadata

- **Finding source**: L/audit/glm5.2/fresh-context-same-family S2, S3, S4.
- **Cluster**: C17
- **Decision**: D17
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/auth.rs` ~L170-180; `crates/opi-ai/src/api_mapped.rs`; `crates/opi-agent/src/harness.rs`; `crates/opi-agent/src/loop_types.rs`; `crates/opi-agent/src/agent.rs`; `crates/opi-coding-agent/src/harness.rs` ~L2488-2495; `crates/opi-coding-agent/src/picker.rs`; `crates/opi-ai/Cargo.toml`; EN/ZH crate READMEs
- **Change**: Describe collection-owned auth preparation, unconditional identity minting versus optional emission, current evidence-health behavior, and retained session ownership accurately. Remove documentation/exports for dead surfaces removed in Layers 1-3. Replace policy-identity `Debug` inputs with the canonical values implemented in Fix 3.4 and remove the unused `async-trait` dependency; do not extract unrelated duplication merely for style.
- **Test plan**: Run rustdoc with warnings denied through the affected scoped smoke gates, `cargo metadata --no-deps --format-version 1`, and source scans proving removed names and stale ownership claims are absent.

## Final verification

The remediation is cross-crate and changes public contracts, so the final gate is workspace-wide after every layer-specific gate passes:

    python scripts/opi-cargo-cache.py status
    powershell -File scripts/opi-impl-smoke.ps1 full
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps; Remove-Item Env:RUSTDOCFLAGS
    python scripts/opi-doc-check.py
    cargo test -p opi-coding-agent --test phase17_provider_runtime --test phase17_tool_authority --test phase17_product_evidence --test phase17_cross_mode --test phase17_failure_rollback --test phase17_api_audit --test phase17_artifact_truthfulness --test doctor_cli --test session_runtime

Success means all commands exit zero with the external Cargo cache enabled; no test performs paid-provider or live-network work; all prior trace/session fixtures remain byte-identical; and `git diff -- docs/snapshots/phase17/opi-impl-state.json` is empty.

## Scope exclusions

Findings or subclaims that do not create an additional fix item:

| Finding | Status | Reason |
|---------|--------|--------|
| D:`prepare-next-turn-seam-unused-in-production` (standalone removal recommendation) | Info/No action | The seam is normatively required for Agent embedders. Its real product gap is product compaction bypassing the seam, covered by Fixes 2.1 and 3.5; the seam itself is not removed. |
| L:S4 broad prompt-body/tool-result/evidence-accessor duplication refactors | Info/No action | Full reads confirm duplication, but it is style/maintainability work with no additional behavioral defect. Only the tool-origin table, provider provenance table, canonical digest, and unused dependency subclaims enter finding-scoped fixes. |
| L:R3 adapter timeout/header/cancellation/capability divergences | Info/No action | The normalized finding has `criterion_source: null`, identifies pre-existing adapter differences, and does not establish a Phase 17 neutral-interface failure. |
| L:R4 regex compilation, proxy overflow, Bedrock blocking-I/O/date arithmetic, and RPC event naming | Info/No action | These normalized Info residuals have `criterion_source: null` and are pre-existing. R4's terminal-stream and dummy-resolver subclaims are duplicates of C6 and C3 and are covered there. |

No normalized Blocker, Major, or criterion-backed Minor finding is deferred. The implementation ledger remains read-only and must not be edited by remediation.
