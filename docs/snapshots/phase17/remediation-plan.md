# Phase 17 Remediation Plan

**Date**: 2026-08-22
**Finding sources**: `docs/snapshots/phase17/audit.codex.md` (audit, codex), `docs/snapshots/phase17/audit.glm5.3.md` (audit, glm5.3)
**Verification target**: current committed `136c380f0c5eea541190cc1a0f5c1d62f983b4e8`
**Phase-exit provenance**: frozen `docs/snapshots/phase17/opi-impl-state.json`; Tasks 17.1--17.9 were verified at `41464d8`, `0e7ed0b`, `96370ce`, `5ebadac`, `6600cd2`, `b547f45`, `32c79e7`, `4893014`, and `a4cfa4d` respectively. These commits are provenance, not remediation coverage.
**Design spec**: `docs/opi-spec.md`; `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`

This is a plan-only artifact. It does not authorize execution, does not revise either specification, and does not modify `.opi-impl-state.json`.

Success means that every normalized finding is either tied to a verified fix below or appears in Scope exclusions; each fix has a failing-before/passing-after focused check; and the final cross-crate verification union passes without weakening a registered Phase 17 requirement.

---

## Finding cross-reference summary

Both reports' normalized blocks declare `fresh-context-same-family`. Therefore repeated findings are **correlated overlap**, not independent-family consensus. Single-report findings are **single same-family source**. Coverage is descriptive only; the final status below comes from verification against the current committed source.

| Cluster | Theme | Sources | Independence | Coverage | Source severity range | Final severity + rationale | Verification |
|---|---|---|---|---|---|---|---|
| C01 | One dispatchable provider-route owner | codex `P17-CODEX-STD-001`; glm `glm53-007`, `glm53-059` | same family | correlated/related overlap | Major--Info | Major: the split stores and listing-only `Provider` are real and contradict the registered single collection owner; repeated product switches already drift on availability | Confirmed |
| C02 | Auth provenance must not use a valid value as “unset” | codex `P17-CODEX-STD-002`; glm `glm53-024` | same family | correlated overlap | Major--Info | Major: `Static + NotAttempted` is both valid and the overwrite sentinel | Confirmed |
| C03 | Atomic, origin-preserving trusted tool registration | codex `P17-CODEX-STD-003`; glm `glm53-025`, `glm53-039` | same family | correlated/related overlap | Major--Info | Major: registration can retain inconsistent identity/schema, and A07 never exercises the laundering path | Confirmed |
| C04 | Safe typed denial facts | codex `P17-CODEX-STD-004`; glm `glm53-021` | same family | correlated overlap | Major--Minor | Major: arbitrary reason/code text reaches model-visible results and diagnostics | Confirmed |
| C05 | Safe public evidence failures | codex `P17-CODEX-STD-005` | same family | single source | Major | Major: public `EvidenceError` renders unrestricted adapter detail | Confirmed |
| C06 | Classified artifact metadata | codex `P17-CODEX-STD-006` | same family | single source | Major | Major: infallible public strings cannot enforce the producer classification boundary | Confirmed |
| C07 | Content-bearing Agent `Debug` | codex `P17-CODEX-STD-007` | same family | single source | Major | Major: complete message and cleanup content is printed | Confirmed |
| C08 | Secret-bearing configuration and adapter copies | codex `P17-CODEX-STD-008`; glm `glm53-032`, `glm53-033` | same family | related overlap | Major--Minor | Major for externally printable proxy/API-key configuration; Minor for non-zeroizing request-task copies | Confirmed |
| C09 | Canonical model normalization and mutation | codex `P17-CODEX-STD-009`, `P17-CODEX-SPEC-004`; glm `glm53-006`, `glm53-009`, `glm53-010` | same family | correlated overlap | Major--Minor | Major: startup can discard the uniquely proven provider; colon-bearing bare IDs are misclassified; public mutation can panic or persist before full validation | Confirmed |
| C10 | Typed provider assembly and consistent startup | codex `P17-CODEX-STD-011`; glm `glm53-005`, `glm53-023`, `glm53-055` | same family | related overlap | Minor | Minor: duplicated mode classification, extension-reachable panics, eager Anthropic store failure, and route-local fallback visibility are real | Confirmed |
| C11 | Durable session prefix, envelope, and runtime binding | codex `P17-CODEX-SPEC-001` | same family | single source | Major | Major: current v1 reader skips unknown tags and reconstructs no immutable `RuntimeInputBinding`, contrary to `INV-007` | Confirmed |
| C12 | Bounded, observable steering/follow-up queues | codex `P17-CODEX-SPEC-002`; glm `glm53-074` | same family | correlated overlap | Major--Info | Major: parent `INV-006` explicitly requires observable closure/overflow; unbounded `VecDeque` makes the matrix vacuous | Confirmed |
| C13 | Settle previously committed state and queued input | codex `P17-CODEX-SPEC-003`; glm `glm53-019` | same family | related overlap | Major--Minor | Major: an ordinary later error discards a transition already committed before queue polling and can lose drained user input | Confirmed |
| C14 | Complete next-turn intrinsic validation | codex `P17-CODEX-SPEC-005` | same family | single source | Major | Major: model output ceilings and thinking-budget relationships are omitted | Confirmed |
| C15 | Reauthorize after evidence-health generation changes | codex `P17-CODEX-SPEC-006` | same family | single source | Minor | Minor: execution fails closed, but the specified stale-Allow reauthorization is bypassed | Confirmed |
| C16 | Truthful evidence manifest construction | codex `P17-CODEX-SPEC-007`; glm `glm53-014`, `glm53-015`, `glm53-017` | same family | correlated/related overlap | Minor--Info | Major for missing content/authority binding required by `CTRL-001/002` and `P17-OUT-004`; Minor for divergent empty observations and the public empty-record panic | Partially confirmed: empty builder behavior is exact; final authority-reference shape needs user selection |
| C17 | Exact capability, policy, and execution scope facts | codex `P17-CODEX-SPEC-008`, `P17-CODEX-SEC-001`; glm `glm53-003`, `glm53-004`, `glm53-035` | same family | related overlap | Major--Minor | Major: external reads can be recorded as workspace-only; fixed IDs differ from the spec. Digest/list duplication and untyped backend binding are Minor | Confirmed; external-read policy direction needs user selection |
| C18 | Execute the registered rollback profile | codex `P17-CODEX-SPEC-009`; glm `glm53-016` | same family | correlated overlap | Minor--Info | Minor: the frozen evidence explicitly substituted a structural scan for the required pre-Phase runtime profile | Confirmed |
| C19 | Frozen-auth terminal classification | codex `P17-CODEX-INV-001` | same family | single source | Major | Major: `AuthFailed` and `AccountIdMissing` can redispatch the rejected frozen credential | Confirmed |
| C20 | Compaction persistence failure must be public failure | codex `P17-CODEX-INT-001` | same family | single source | Major | Major: `prompt` returns `Ok`/exit zero while evidence records failure or cleanup uncertainty | Confirmed |
| C21 | Session CLI must honor Cargo's resolved binary | codex `P17-CODEX-TST-001`; glm `glm53-065` | same family | correlated overlap | Minor--Info | Minor: hard-coded workspace `target/debug/opi` breaks the repository cache contract and its fallback is unreachable | Confirmed |
| C22 | Bedrock event-stream corruption is terminal | glm `glm53-018` | same family | single source | Major | Major: CRC-invalid complete frames are consumed and silently dropped | Confirmed |
| C23 | Bedrock thinking-history wire semantics | glm `glm53-012` | same family | single source | Minor | Minor: thinking is relabeled as ordinary text; the correct supported wire representation needs protocol confirmation | Partially confirmed |
| C24 | Anthropic malformed-frame terminal ordering | glm `glm53-020` | same family | single source | Minor | Minor: both fixture and HTTP loops can emit `Err` followed by `Ok`/`Done` | Confirmed |
| C25 | Collision-resistant session creation | glm `glm53-022` | same family | single source | Minor | Minor: millisecond-only IDs collide across concurrent creators | Confirmed |
| C26 | Redaction parity across proxy and TUI | glm `glm53-030`, `glm53-031`, `glm53-034` | same family | related overlap | Minor | Minor: raw proxy input/responses, common secret-field spellings, and interactive errors bypass shared redaction | Confirmed |
| C27 | Phase acceptance tests that currently cannot fail for their named rule | glm `glm53-011`, `glm53-040`, `glm53-041`, `glm53-042`, `glm53-043`, `glm53-044` | same family | single-source group | Minor | Minor: each cited fixture or guard misses its stated production/rule path | Confirmed |
| C28 | Product integration failure and disclosure edges | glm `glm53-056`, `glm53-057`, `glm53-058`, `glm53-063`, `glm53-064` | same family | single-source group | Minor | Minor: blocking keyring calls, mid-run trace semantics, absolute-root disclosure, lost diagnostics, and timeout misclassification are present | Confirmed |
| C29 | Current-contract source comments | codex `P17-CODEX-STD-012`; glm `glm53-001`, `glm53-002` | same family | correlated/related overlap | Minor--Info | Minor: Phase/task markers violate `AGENTS.md`, and one test comment states the inverse ordering | Confirmed |
| C30 | Unadmitted provider refresh surface | codex `P17-CODEX-STD-010` | same family | single source | Minor | Minor: no production trigger and only one real adapter; removal is a public API decision | Confirmed |

## Review decisions required

Approval of this plan means selecting the recommended **(a)** option for every user-pending decision: D1, D5--D12, and D15. Any different selection should be recorded here before execution.

1. **D1 — provider route surface**
   - **(a) Recommended:** replace split route maps with one private atomic `RouteEntry`, remove lookup-only `Provider` registration/construction, and expose listing metadata as data rather than a fake provider.
   - **(b):** retain the public lookup-only surface, which requires an explicit registered-spec revision because the current design removes the metadata-only proxy and second owner.
   - Codex recommends consolidation; glm recommends a descriptor row but treats it as a lower-severity smell.
2. **D5 — queue bound/API**
   - **(a) Recommended:** one crate-owned finite default and `Result<(), AgentControlError::{Full, Closed}>` from both `Agent` and `AgentControl`; do not add a user config key.
   - **(b):** add queue capacities to `AgentLoopConfig`, broadening public configuration and test permutations.
   - Codex requires an observable bounded queue; glm would document the owner split unless steering becomes producer-facing. It already is producer-facing, so documentation alone does not satisfy `INV-006`.
3. **D6 — durable session format**
   - **(a) Recommended:** add a new versioned envelope for newly written entries with required/ignorable classification and an immutable runtime-binding header; continue reading legacy v1 byte-for-byte without rewriting it.
   - **(b):** revise `INV-007` to accept best-effort unknown-tag skipping and a non-durable runtime binding.
   - Only codex reported the gap; the parent spec is explicit and favors (a).
4. **D8 — artifact/manifest shape**
   - **(a) Recommended:** make artifact locations validated opaque/relative references, content-address `evidence.jsonl`, and use ordered set-valued authority references so multiple tool calls remain representable.
   - **(b):** bind only the evidence log and narrow the registered manifest-authority requirement in shaping.
   - Codex requires artifact and applicable authority binding; glm considers the current empty set documented but identifies divergent validation inputs.
5. **D9 — interactive outside-workspace reads**
   - **(a) Recommended:** preserve the intentional feature, add a distinct typed external-read permission scope containing a non-secret path identity, and revalidate the resolved relation at execution.
   - **(b):** prohibit outside-workspace reads so the existing `workspace:read` fact becomes truthful.
   - Codex identifies the mismatch; glm did not report it. `AGENTS.md` requires approval before removing intentional behavior, so (b) cannot be automatic.
6. **D12 — provider refresh**
   - **(a) Recommended:** remove the public refresh seam and test-only adapter until a real trigger and second consumer justify it.
   - **(b):** register a real production trigger and second adapter/conformance suite, which expands scope beyond remediation and would need shaping.
   - Codex recommends narrowing/removal; glm notes adjacent dead surfaces but does not independently require refresh.
7. **D15 — Bedrock thinking history**
   - **(a) Recommended:** verify the supported Converse reasoning-content shape against authoritative protocol documentation, map to it when supported, and otherwise omit thinking blocks rather than relabel them as assistant text.
   - **(b):** remove advertised Bedrock thinking support until round-trip semantics exist.
   - Glm recommends a reasoning mapping or deliberate omission; no second report covers this path.
8. **D7 — authority public API**
   - **(a) Recommended:** make registration and denial construction private/fallible and update all callers together.
   - **(b):** retain the old infallible constructors as compatibility shims, which conflicts with the repository's 0.x cleanup policy and leaves the unsafe boundary callable.
   - Codex requires the API closure; glm confirms the unvalidated fields and vacuous laundering test.
9. **D10 — secret-bearing public configuration**
   - **(a) Recommended:** change `ProviderConfig.api_key` to a secrecy-wrapped type, remove secret-emitting derives, and use redacted proxy/config debug.
   - **(b):** preserve the field/serde API and document that debug/serialization can expose credentials, which does not satisfy `P17-FAL-004`.
   - Codex covers proxy configuration; glm covers `ProviderConfig` and request-task copies.
10. **D11 — model-mutation public API**
    - **(a) Recommended:** change `set_model` to return the existing typed `Result` and update callers.
    - **(b):** retain the panicking method and add another checked method, leaving a compatibility path that violates the typed-boundary rule.
    - Both reports identify the panic; codex also identifies lost provider normalization and glm the persist-before-apply divergence.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C01 | Recommended option (a): one atomic route entry and data-only listing metadata | Directly implements the registered single collection owner without a new trait or registry | user (pending plan approval) |
| D2 | C02, C19 | Represent resolver omission explicitly; classify every frozen-auth rejection/expiry as terminal | Closed domain states already determine the correction | auto |
| D3 | C22, C24 | Propagate complete-frame parse failures as one terminal stream error; stop Anthropic after malformed input | No valid consumer semantics permit content after corruption/error | auto |
| D4 | C13, C14 | Persist the loop's last committed state on later failure and validate all resolved model ceilings before commit | The loop already defines the commit point and retains the prior state until then | auto |
| D5 | C12 | Recommended option (a): bounded queue with observable `Full`/`Closed` results and no new config key | Satisfies `INV-006` with the smallest public mechanism | user (pending plan approval) |
| D6 | C11 | Recommended option (a): versioned required/ignorable envelope plus immutable runtime binding for new writes; legacy read-only compatibility | `INV-007` states the durable semantics; the choice affects the on-disk format | user (pending plan approval) |
| D7 | C03, C04, C15 | Private/fallible registration and safe denial types; reauthorize once after evidence-generation change | Existing authority order and closed facts uniquely determine the fix | user (pending plan approval for public API break) |
| D8 | C05, C06, C16 | Recommended option (a): safe public evidence summaries, validated artifact references, content-bound log, set-valued authority refs | Required to make `CTRL-001/002/003` mechanically true; changes public/schema surfaces | user (pending plan approval) |
| D9 | C17 | Recommended option (a): preserve external reads with a distinct typed scope; also use exact fixed IDs, canonical policy serialization, one mutating classifier, and typed backend scope | Keeps intentional behavior while making the recorded authority truthful | user (pending plan approval for external-read meaning) |
| D10 | C07, C08, C26 | Metadata-only `Debug`, secrecy-wrapped credentials, explicit common-field redaction, and TUI/proxy parity | Behavior-preserving boundary hardening; `ProviderConfig` field/serde changes are 0.x breaking | user (pending plan approval for `ProviderConfig` API break) |
| D11 | C09 | One provider-aware canonicalizer; public `set_model` returns `Result`; validate before durable append and make commit infallible | Removes three divergent decisions and prevents panic/durable-live divergence | user (pending plan approval for public API break) |
| D12 | C30 | Recommended option (a): remove unadmitted refresh APIs and their one-off test implementation | Removing intentional public surface requires explicit approval | user (pending plan approval) |
| D13 | C10 | Return typed provider-assembly failures, keep credential resolution lazy, emit fallback facts per route, and share startup classification | Existing typed failures and per-call auth contract determine the correction | auto |
| D14 | C20, C25, C28 | Propagate actual product failures, use UUIDv7-style session IDs, and close the cited disclosure/classification edges | Local behavior has one clear truthful outcome | auto |
| D15 | C23 | Recommended option (a): verified reasoning wire mapping or deliberate omission, never ordinary-text relabeling | Correct mapping depends on the supported external protocol | user (pending plan approval) |
| D16 | C21, C27, C29 | Repair the named tests/comments to exercise and describe current production contracts | Test/document truthfulness is behavior-preserving | auto |
| D17 | C18 | Run the exact pre-Phase rollback profile in an isolated checkout and record results without editing the implementation ledger | This is the mechanical verification required by `P17-RBK-002` | auto |

## Remediation layers

### Layer 1: `opi-ai` (substrate)

**Verification**:

    cargo test -p opi-ai --lib bedrock::event_stream
    cargo test -p opi-ai --test provider_collection --test per_request_auth --test auth_contracts
    cargo test -p opi-ai --test bedrock_fixtures --test anthropic_fixtures

#### Fix 1.1: Make a provider route one atomic owned value

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-001`); `audit.glm5.3.md` (audit/glm5.3 `glm53-007`, `glm53-059`)
- **Cluster**: C01
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider_collection.rs` ~L297--580; `crates/opi-ai/src/registry.rs`
- **Change**: Store provider, resolver, auth descriptor/provenance source, compatibility, probe, and catalog metadata in one private `RouteEntry` inserted/replaced atomically. Remove lookup-only `from_registry`/`register` behavior and expose catalog metadata as data rather than through a non-dispatchable `Provider`.
- **Test plan**: Update `provider_collection` registration/replacement/listing tests to prove one insertion/replacement changes every route fact atomically and no lookup-only provider can be constructed.

#### Fix 1.2: Make auth provenance and terminal state explicit

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-002`, `P17-CODEX-INV-001`); `audit.glm5.3.md` (audit/glm5.3 `glm53-024`)
- **Cluster**: C02, C19
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth.rs` ~L90--220; `crates/opi-ai/src/provider_collection.rs` ~L445--459, L727--827
- **Change**: Replace equality-to-default omission detection with an explicit resolver outcome (`Reported(AuthProvenance)` versus `UseRegisteredSource`). Centralize a closed `terminates_frozen_auth` classification covering `AuthFailed`, `CredentialNeeded`, `CredentialRevoked`, and `AccountIdMissing`; after any such error, every later `start_attempt` returns the terminal error without dispatch.
- **Test plan**: Add a non-static registered route whose resolver truthfully reports default static provenance and assert no overwrite; extend `stream_time_credential_failure_forbids_redispatch` for all four terminal auth classes and a retryable non-auth control.

#### Fix 1.3: Fail closed on Bedrock/Anthropic stream corruption

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-018`, `glm53-020`)
- **Cluster**: C22, C24
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/bedrock/event_stream.rs` ~L18--130, L385; `crates/opi-ai/src/bedrock/mod.rs` stream consumer; `crates/opi-ai/src/anthropic.rs` ~L900--1040
- **Change**: Return typed frame outcomes that distinguish incomplete input from malformed length/header/CRC; consume a complete malformed frame only while emitting one terminal `StreamError`. Break both Anthropic parse loops immediately after malformed input.
- **Test plan**: Replace `parse_frames_ignores_bad_crc_without_panic` with prelude-CRC, message-CRC, header-overflow, and invalid-header tests that assert exactly one error and no later content/Done; add Anthropic fixture and HTTP-stream cases asserting no post-error event.

#### Fix 1.4: Keep provider credentials secrecy-wrapped

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-032`, `glm53-033`)
- **Cluster**: C08
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/config.rs` ~L1--20; `crates/opi-ai/src/gemini.rs` ~L858; `crates/opi-ai/src/azure_openai.rs` ~L220; `crates/opi-ai/src/vertex.rs` ~L157
- **Change**: Change the unused public `ProviderConfig.api_key` to a secrecy-wrapped value, remove secret-emitting derived `Debug`/`Serialize`, and provide only redacted debug plus explicit secret exposure at the wire boundary. Move `ResolvedAuth`/`SecretString` into Gemini, Azure, and Vertex request tasks and call `expose_secret()` only while constructing the header/query.
- **Test plan**: Add compile/runtime safety tests proving `Debug` and ordinary serialization cannot reveal a canary; extend adapter fixture tests with drop/lifetime-safe prepared auth and unchanged wire headers.

#### Fix 1.5: Correct provider-path test depth

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-042`)
- **Cluster**: C27
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/provider_collection.rs` ~L713--760
- **Change**: Replace the misleading `store_credential_dispatch_*` setup with real `AuthDescriptor::StoreCredential` routes and an injected probe; retain separate names for static-resolver tests.
- **Test plan**: Assert probe call count, present/missing/backend-error outcomes, dispatch count, and no probe on static routes.

### Layer 2: `opi-agent` (Agent Core)

**Verification**:

    cargo test -p opi-agent --test agent_wrapper --test hooks_queues --test agent_loop_semantics
    cargo test -p opi-agent --test session_contract --test session_storage --test session_branching --test session_facade
    cargo test -p opi-agent --test tool_authority --test tool_validation
    cargo test -p opi-agent --test evidence_contract --test evidence_runtime
    cargo test -p opi-agent --test streaming_proxy --test provider_public_safety

#### Fix 2.1: Settle committed next-turn state and make queue outcomes observable

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-002`, `P17-CODEX-SPEC-003`); `audit.glm5.3.md` (audit/glm5.3 `glm53-019`, `glm53-074`)
- **Cluster**: C12, C13
- **Decision**: D4, D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent.rs` ~L490--520, L550--615, L925--965; `crates/opi-agent/src/loop_types.rs` ~L430--470; `crates/opi-agent/src/agent_loop.rs` ~L1000--1160, L2089--2110
- **Change**: Always settle the loop's returned state because cancellation/validation before the commit point already restores `prior_state`; this preserves earlier committed transitions and drained queued input on a later ordinary error. Replace raw shared `VecDeque`s with one bounded internal queue type whose producer methods return `Full` or `Closed`, preserve steering priority, and close producers when the owning Agent is dropped.
- **Test plan**: Add a two-turn failure test proving turn-1 state remains the Agent state; add steering and follow-up cases proving drained input is retained exactly once after turn-2 failure. Add boundary tests for capacity, FIFO, steering priority, closure, concurrent producers, and zero provider dispatch on rejected enqueue.

#### Fix 2.2: Validate every intrinsic next-turn limit atomically

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-005`)
- **Cluster**: C14
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/loop_types.rs` ~L342--405; `crates/opi-ai/src/model_info.rs` resolved capability fields
- **Change**: Extend `validate_next_turn_candidate` to reject `max_tokens` above the resolved model ceiling and invalid thinking budgets/levels before `mem::replace`. Keep all checks pure and shared with product pre-persist validation.
- **Test plan**: Add boundary values (equal/above ceiling), enabled-thinking budget relationships, and cancellation tests asserting byte-equivalent prior state plus untouched stop/queues.

#### Fix 2.3: Make the committed session prefix and runtime binding durable truth

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-001`)
- **Cluster**: C11
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/session.rs` ~L1--540; core session entry/envelope and reconstruction types
- **Change**: Introduce a new versioned envelope for newly written entries with `required` versus explicitly `ignorable-observation` semantics. Persist one immutable `RuntimeInputBinding` header for the branch and return it with the validated committed prefix. Fail distinctly on unsupported version, unknown required entry, corruption, and interrupted tail; skip only explicitly ignorable observations. Preserve the core legacy-v1 reader without rewrite behavior; product resume/fork/export adaptation follows in Layer 3.
- **Test plan**: Add required/ignorable/unsupported fixtures, binding mismatch and missing-binding failures, committed-prefix plus tail recovery, and resume/fork/export equality from the same `(prefix, binding)` pair. Retain every legacy byte-immutability fixture.

#### Fix 2.4: Make trusted registration one validated immutable construction

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-003`); `audit.glm5.3.md` (audit/glm5.3 `glm53-025`, `glm53-039`)
- **Cluster**: C03
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/authority.rs` ~L35--160; `crates/opi-agent/src/extension.rs` ~L410--430
- **Change**: Make `RegistrationId` and `RegisteredTool` fields private and construction fallible; validate opaque identity and bind implementation, visible name, schema digest, origin, and capability in the same value. Require trusted assembly to supply origin rather than allowing a later name-based inference.
- **Test plan**: Add empty/control/padded ID rejection, schema/name mismatch, duplicate ID/name, and origin-preservation cases at the core registry boundary. Product one-definition/A07 coverage follows in Layer 3.

#### Fix 2.5: Enforce safe denial facts and stale-Allow reauthorization

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-004`, `P17-CODEX-SPEC-006`); `audit.glm5.3.md` (audit/glm5.3 `glm53-021`)
- **Cluster**: C04, C15
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/authority.rs` ~L270--290; `crates/opi-agent/src/agent_loop.rs` ~L1820--1885, L2070--2085; `crates/opi-agent/src/evidence.rs` denial constructors
- **Change**: Replace denial strings with a validated stable-code identity and redacted safe-summary type shared by the decision, evidence, diagnostic, and tool-result paths. If authorization-evidence emission changes the health generation, rebuild the request with current health and call the authorizer once more; enforce and record the second decision before launch.
- **Test plan**: Add canary code/reason rejection/redaction across model context, diagnostics, events, and evidence; add counting-authorizer tests for unchanged generation, one reauthorization, second denial/error, and zero execution.

#### Fix 2.6: Seal evidence errors, artifact references, and public `Debug`

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-005`, `P17-CODEX-STD-006`, `P17-CODEX-STD-007`)
- **Cluster**: C05, C06, C07
- **Decision**: D8, D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/evidence.rs` ~L290--325, L660--705; `crates/opi-agent/src/agent.rs` ~L95--120
- **Change**: Expose only a private validated/redacted evidence-summary type publicly while retaining raw adapter source/path detail in an internal error chain. Make artifact construction fallible and restrict the public location to the approved opaque/relative form selected in D8. Change `AgentRunResult::Debug` to correlation/count/outcome metadata only and audit adjacent public state debug implementations reached from it.
- **Test plan**: Add secret/path canaries to `Debug`/`Display`, invalid artifact locations/media, serialization, and file-adapter failures; assert metadata-only `AgentRunResult` debug contains no prompt, tool argument/result, or cleanup content.

#### Fix 2.7: Redact every StreamingProxy output path

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-030`, `glm53-031`)
- **Cluster**: C26
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/streaming_proxy.rs` ~L230--305, L330--460
- **Change**: Apply the configured redactor to malformed-input responses and handler `SdkResponse`s, not only Agent events. Add the explicitly identified common sensitive field spellings; do not use broad substring rules that would unpredictably erase normal data.
- **Test plan**: Add raw JSONL, handler response, nested object/array, case/dash/underscore field, opaque-value, and `redact_secrets=false` controls.

#### Fix 2.8: Repair authority/evidence tests that assert the wrong rule

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-040`, `glm53-041`)
- **Cluster**: C27
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/tests/tool_validation.rs` ~L640; `crates/opi-agent/tests/evidence_contract.rs` ~L580--590, L1035--1090
- **Change**: Add a counting authorizer to the invalid-schema precedence case and rebuild graph-rule fixtures so kind/payload pairs are valid and only the named parent/kind/turn/retry relation is invalid.
- **Test plan**: Assert authorizer count zero and downstream execution zero; for each manifest fixture, assert it passes kind/payload validation before the target graph rule rejects it.

### Layer 3: `opi-coding-agent` (Reference Product)

**Verification**:

    cargo test -p opi-coding-agent --lib
    cargo test -p opi-coding-agent --test provider_factory --test phase17_provider_runtime --test phase17_product_evidence
    cargo test -p opi-coding-agent --test phase17_tool_authority --test phase17_failure_rollback --test phase17_legacy_migration
    cargo test -p opi-coding-agent --test phase17_cross_mode --test json_mode --test session_cli
    cargo test -p opi-coding-agent --test credential_store --test rpc_jsonl --test execution_protocol_host
    cargo test -p opi-coding-agent --test tools_glob_grep --test interactive_mock --test config_tests

#### Fix 3.1: Use one provider-aware canonical model decision

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-009`, `P17-CODEX-SPEC-004`); `audit.glm5.3.md` (audit/glm5.3 `glm53-006`, `glm53-009`, `glm53-010`)
- **Cluster**: C09
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L175--210, L1870--1995; `crates/opi-coding-agent/src/provider_factory.rs` startup normalization; `crates/opi-agent/src/harness.rs` model-input provenance
- **Change**: Replace every `contains(':')` heuristic with one collection-aware resolver that evaluates both an exact bare model ID and a parsed registered-provider prefix, fails on two different valid routes, and returns the exact canonical `provider:model` plus source. Make `set_model` return the existing typed error. Run the shared intrinsic next-state validation before appending `model_change`; after a successful append, commit through an infallible validated state operation.
- **Test plan**: Cover unique/ambiguous/missing bare IDs, registered/unregistered colon prefixes, Bedrock IDs containing additional colons, canonical-vs-bare dual matches, no provider dispatch on failure, public non-panicking mutation, and no session append when thinking/output validation fails.

#### Fix 3.2: Make provider assembly single-owned, typed, lazy, and mode-consistent

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-001`, `P17-CODEX-STD-011`); `audit.glm5.3.md` (audit/glm5.3 `glm53-005`, `glm53-007`, `glm53-023`, `glm53-055`, `glm53-059`)
- **Cluster**: C01, C10
- **Decision**: D1, D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs` ~L150--200, L1320--1390, L1620--1830; `crates/opi-coding-agent/src/main.rs` ~L830--1210
- **Change**: Replace the six provider-id switches and `ListingMetadataProvider` with one product descriptor table consumed by construction, listing, environment, auth, availability, and the Layer-1 route API. Return typed registration/config errors instead of `expect`; retain operational credential-store failures in the lazy resolver so startup still permits remediation; emit fallback diagnostics for every constructed route. Factor provider-bundle construction and failure taxonomy into one helper consumed by interactive, noninteractive, and RPC paths, leaving presentation local.
- **Test plan**: Add descriptor parity and listing/runtime availability tests, including keychain-backed `openai_compatible`; add extension provider collision/malformed route inputs with no panic, active Anthropic unavailable/corrupt store startup plus prepare-call failure, extra-route env fallback diagnostics, and a cross-mode table asserting identical typed category/redaction.

#### Fix 3.3: Make product authority facts exact and canonical

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-003`, `P17-CODEX-SPEC-008`, `P17-CODEX-SEC-001`); `audit.glm5.3.md` (audit/glm5.3 `glm53-003`, `glm53-004`, `glm53-039`, `glm53-035`)
- **Cluster**: C03, C17
- **Decision**: D7, D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool_authority.rs` ~L40--70, L130--190, L430--460; `crates/opi-coding-agent/src/harness.rs` ~L1545--1590; `crates/opi-coding-agent/src/policy.rs`; `crates/opi-coding-agent/src/execution/runtime.rs` ~L450--500
- **Change**: Read each product tool definition once, carry origin from the builtin/extension assembly seam into the fallible core registration, and never assign builtin capability from a name alone. Use the exact fixed IDs `workspace.read`, `workspace.write`, and `command.execute`; call the one policy-owned mutating classifier; hash explicitly versioned canonical fields with deterministic package ordering instead of `Debug`; replace string `authorized_backend` with validated `CommandPermissionScope`. Under recommended D9(a), represent external-read permission separately, bind a non-secret path identity/relation, and recheck it immediately before reading.
- **Test plan**: Rewrite A07 so malicious extension `read`/`write`/`bash` tools traverse actual product assembly and execute zero times; add an alternating-definition tool. Update authority/evidence fixtures for exact IDs; add digest golden/permutation/toolchain-independent serialization tests, typed backend mismatch/ask tests, and interactive external-read Allow/Deny/stale-path cases asserting truthful evidence and zero access after relation change.

#### Fix 3.4: Make product evidence finalization truthful

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-007`, `P17-CODEX-INT-001`); `audit.glm5.3.md` (audit/glm5.3 `glm53-014`, `glm53-015`, `glm53-017`)
- **Cluster**: C16, C20
- **Decision**: D8, D14
- **Verification status**: Partially confirmed for final manifest shape; Confirmed for builder/compaction behavior
- **File(s)**: `crates/opi-coding-agent/src/evidence.rs` ~L430--680; `crates/opi-coding-agent/src/harness.rs` ~L2860--3050, L4710--4735; `crates/opi-coding-agent/src/runner.rs` ~L440--460
- **Change**: Pass the recorder's actual artifact observation into the builder and return typed `Finalization` on an empty record graph. Under D8(a), finalize/hash `evidence.jsonl` before manifest validation and include it plus the ordered authority references represented by the run. On automatic-compaction persistence or rollback-cleanup failure, retain the committed turn and truthful terminal evidence but return a typed public error so runners exit nonzero.
- **Test plan**: Add mismatched builder/sink artifact sets, empty records, tampered evidence-log digest, multiple authority refs, and no-artifact runs. Invert `automatic_compaction_rollback_failure_emits_cleanup_unknown_terminal` to require typed `Err`, retained committed content, `CleanupUnknown`, withheld/appropriate manifest, and nonzero print/JSON exit.

#### Fix 3.5: Use collision-safe session IDs and Cargo's test binary

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-001`, `P17-CODEX-TST-001`); `audit.glm5.3.md` (audit/glm5.3 `glm53-022`, `glm53-065`)
- **Cluster**: C11, C21, C25
- **Decision**: D6, D14, D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/session_coordinator.rs` session creation/reconstruction and ~L950--970; `crates/opi-coding-agent/src/session_cli.rs` resume/fork/export; `crates/opi-coding-agent/tests/session_cli.rs` ~L830--855
- **Change**: Adapt product session creation, resume, fork, export, and evidence binding to the Layer-2 `(validated prefix, RuntimeInputBinding)` result while preserving legacy bytes. Generate sortable collision-resistant IDs using the repository's UUIDv7 precedent and create the initial file exclusively. Resolve the integration-test executable from Cargo's `CARGO_BIN_EXE_opi` contract; remove the dead local-build fallback and workspace-target assumption.
- **Test plan**: Extend Phase 17 legacy migration with new required/ignorable/binding fixtures across resume/fork/export. Concurrent same-timestamp/session-process creation yields distinct paths; session CLI passes with the external cache and with no worktree-local `target/debug/opi`.

#### Fix 3.6: Close configuration, TUI, and structured-output disclosure paths

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-008`); `audit.glm5.3.md` (audit/glm5.3 `glm53-034`, `glm53-058`)
- **Cluster**: C08, C26, C28
- **Decision**: D10, D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/config.rs` ~L20--35, L450--465; `crates/opi-ai/src/http.rs` ~L80--95; `crates/opi-coding-agent/src/interactive.rs` ~L760--790, L1120--1145, L1670--1680; `crates/opi-coding-agent/src/tool/grep.rs` ~L185--195; `crates/opi-coding-agent/src/tool/glob.rs` ~L135--145
- **Change**: Parse proxy URLs into a credential-redacting wrapper whose `Debug`/`Display` removes userinfo and sensitive query values. Apply Summary redaction at every interactive error render. Remove absolute `workspace_root` from public grep/glob details and retain only the relation plus workspace-relative result paths.
- **Test plan**: Add proxy userinfo/query canaries across config and HTTP debug, TUI prompt/compaction/session-persist canaries, and JSON/NDJSON tool-result assertions that no absolute root appears.

#### Fix 3.7: Move keyring I/O off async workers

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-056`)
- **Cluster**: C28
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L800--910 and all async backend get/set/delete call sites
- **Change**: Run synchronous backend operations in `spawn_blocking` at the existing async credential-store seam, preserving the refresh lock across the required read/HTTP/write transaction without nesting runtimes.
- **Test plan**: Use a blocking fake backend plus a Tokio heartbeat to prove the runtime stays responsive; cover task join failure and existing marker/credential atomicity.

#### Fix 3.8: Preserve typed RPC/protocol failure semantics

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-057`, `glm53-063`, `glm53-064`)
- **Cluster**: C28
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/rpc.rs` ~L990--1030; `crates/opi-coding-agent/src/execution/protocol_host.rs` ~L270--295, L1320--1350
- **Change**: Guard `trace` with the same `agent_busy` rule as `session_info`; carry accumulated diagnostics through `terminate_and_fail`; introduce a distinct handshake-timeout failure code instead of `protocol_violation` before traffic begins.
- **Test plan**: Add active-run trace rejection and idle trace success; inject diagnostics before a malformed frame and assert retention; distinguish pre-traffic timeout from malformed/out-of-order protocol traffic.

#### Fix 3.9: Repair product acceptance depth and environment isolation

- **Finding source**: `audit.glm5.3.md` (audit/glm5.3 `glm53-011`, `glm53-043`, `glm53-044`)
- **Cluster**: C27
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_product_evidence.rs` ~L1960--1980; `crates/opi-coding-agent/tests/provider_factory.rs` ~L340--390; `crates/opi-coding-agent/tests/json_mode.rs` ~L1540 and sibling case
- **Change**: Make the A01 provider mocks report each dispatched route and assert requested/resolved/actual agreement for both providers. Prepare a real factory-built extra route through `build_harness_collection`. Keep the scoped environment guard and temp directory alive through agent/session persistence.
- **Test plan**: Run the three exact integration binaries; add a negative wrong-actual-route assertion, resolver/prepare call counts for the extra route, and a host-session-directory canary for JSON mode.

### Layer 4: Documentation, comments, and rollback evidence

**Verification**:

    python scripts/opi-doc-check.py
    cargo test -p opi-agent --test hooks_queues
    cargo test -p opi-coding-agent --test phase17_api_audit

#### Fix 4.1: Rewrite historical/stale comments as current contracts

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-STD-012`); `audit.glm5.3.md` (audit/glm5.3 `glm53-001`, `glm53-002`)
- **Cluster**: C29
- **Decision**: D16
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/policy.rs` module/inline comments; `crates/opi-coding-agent/src/execution/permission.rs`; `crates/opi-coding-agent/src/rpc.rs` ~L1266; `crates/opi-agent/tests/hooks_queues.rs` ~L715--722; `crates/opi-coding-agent/tests/phase17_api_audit.rs` marker discovery
- **Change**: Remove Phase/task/workstream history while retaining current invariants. Correct the hook/schema/prepare/stop ordering comment. Replace acceptance discovery based on a production Phase marker with behavior/source facts that do not require historical comments.
- **Test plan**: Run the exact two source-sensitive test binaries and the documentation contract.

#### Fix 4.2: Execute and record the registered rollback profile

- **Finding source**: `audit.codex.md` (audit/codex `P17-CODEX-SPEC-009`); `audit.glm5.3.md` (audit/glm5.3 `glm53-016`)
- **Cluster**: C18
- **Decision**: D17
- **Verification status**: Confirmed
- **File(s)**: isolated temporary Git worktree only; result appended to this remediation artifact or a newly authorized remediation-results artifact, never `docs/snapshots/phase17/opi-impl-state.json`
- **Change**: In an isolated worktree, revert the complete registered Phase 17 range, run the named pre-Phase regression profile, then verify that representative Phase 17 evidence/session files remain preserved and that the pre-Phase binary neither rewrites nor broadens authority. Record exact commits, commands, exit results, and artifact hashes. Do not alter the current branch or user worktree.
- **Test plan**: The rollback commands themselves are the test; require a clean isolated worktree before/after and preserve the generated evidence for review.

#### Fix 4.3: Record deliberate 0.x public/schema changes

- **Finding source**: all confirmed public-surface findings implemented under D1, D5--D12
- **Cluster**: C01, C03, C05, C06, C08, C09, C11, C12, C16, C17, C30
- **Decision**: D1, D5--D12
- **Verification status**: Confirmed/Partially confirmed as recorded above
- **File(s)**: `CHANGELOG.md` complete `## [Unreleased]` section; affected English/Chinese user documentation only if current behavior/API text exists in both
- **Change**: After implementation, read the full Unreleased section, update the existing subsection once for user-visible breaking changes, and synchronize any existing localized counterpart. Do not write phase status or remediation history into normative specs.
- **Test plan**: `python scripts/opi-doc-check.py`; generated `opi --help` snapshot only if a CLI surface changed.

## Final verification

After each layer's focused commands pass, use the actual changed-file inventory to compute the union from `.agents/skills/_shared/references/change-scope-and-check-selection.md`. Because the proposed remediation changes all three runtime crates and public/durable schemas, the expected final union is the Phase 17 workspace gate:

    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Before running Cargo, inspect `python scripts/opi-cargo-cache.py status`; keep incremental compilation and the repository external cache enabled. No test may call a paid/live provider or depend on credentials/network. Expected test impact: `add` and `update`; no deletion unless D12 removal is approved, in which case only refresh-specific tests made obsolete by that removal may be deleted.

## Scope exclusions

Findings not producing a fix item in this plan:

| Finding | Status | Reason |
|---|---|---|
| `glm53-013` | Refuted | `split_once(':')` splits only the provider prefix and preserves every later colon in the returned model ID; the cited Bedrock code does not truncate the ID. C09 still fixes the separate product-level `contains(':')` misclassification. |
| `glm53-008` | Info/No action | Factual dead/misleading surface observations, but the source assigns Info and no registered Phase 17 behavior fails. Remove only items made unused by approved fixes. |
| `glm53-026`, `glm53-027`, `glm53-028`, `glm53-029` | Info/No action | Preflight sink lifecycle asymmetry, test-oracle locking, rollback-of-rollback, and year-zero parsing are verified defense/edge observations without a Phase 17 Major/Minor requirement in the source. |
| `glm53-036`, `glm53-037`, `glm53-038` | Info/No action | Documented conversation-redaction tradeoff and memory-hygiene observations; D10 may incidentally improve shared secrecy storage, but this plan adds no separate work. |
| `glm53-045`, `glm53-046`, `glm53-047`, `glm53-048`, `glm53-049`, `glm53-050`, `glm53-051`, `glm53-052`, `glm53-053` | Info/No action | Low-severity test robustness/coverage observations; none is needed to prove an actionable cluster after C27's named gaps are repaired. |
| `glm53-054` | Returned to shaping | Code inspection confirms credentials-file `credential_process` is ignored, but the cited local criterion does not require parity with that external AWS placement. Admit the compatibility requirement with authoritative protocol evidence before changing resolution semantics. |
| `glm53-060`, `glm53-061`, `glm53-062` | Info/No action | RPC tail emit, Codex default system text, and hypothetical sparse Anthropic indices are informational integration observations outside the selected Phase 17 correction set. |
| `glm53-066`, `glm53-067`, `glm53-068`, `glm53-069`, `glm53-070`, `glm53-071`, `glm53-072`, `glm53-073` | Info/No action | Fork atomicity, login ergonomics, sync navigation I/O, temp cleanup, cosmetic output/casts, dead capture, and tail-scan performance are verified residuals but source-classified Info. |
| `glm53-016` | Duplicate | Covered by C18 / Fix 4.2 with the stronger codex Minor finding. |
| `glm53-017` | Duplicate | Its factual empty-artifact observation is covered by C16 / Fix 3.4; the reports disagree on acceptability, so D8 records the user decision. |
| `glm53-024`, `glm53-025`, `glm53-059`, `glm53-065`, `glm53-074` | Duplicate | Covered respectively by C02, C03, C01, C21, and C12 fixes. |
| `P17-CODEX-STD-010` | Returned to shaping pending D12 | Removal or admission of a public refresh seam requires the user's explicit choice; approval of recommended D12(a) promotes it into Fix 1.1 cleanup and updates tests/changelog. |
| `glm53-012` | Returned to shaping pending D15 | The ordinary-text relabeling is confirmed, but exact Bedrock reasoning wire support must be verified before implementation; approval of D15(a) promotes it into Fix 1.3 as a separate serialization fixture. |
