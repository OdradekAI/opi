# Phase 17 Remediation Plan

**Date**: 2026-08-21
**Finding sources**:
- `docs/snapshots/phase17/audit.codex.md` (audit, codex, 2026-08-20, verdict FAIL 0B/4M/3m; working-tree version supersedes the committed 2026-08-15 report retitled by `a680c5d`)
- `docs/snapshots/phase17/audit.glm5.3.md` (audit, glm5.3, 2026-08-21, verdict PASS-WITH-FINDINGS 0B/4M/24m/28I; working-tree version supersedes the committed 2026-08-15 glm5.2 report retitled by `a680c5d`)

**Degraded-input note**: the codex report's normalized blocks declare
`source_path: docs/snapshots/phase17/audit.codex-gpt5.md`, but the file lives at
`audit.codex.md`. IDs are treated as stable within that file; no fields were
repaired in the source reports. Both sources self-declare
`fresh-context-same-family` (reduced independence versus the implementation);
they are distinct model families from each other, so cross-source overlap counts
as independent coverage.

**Commit range**: task commits `41464d8..a4cfa4d`; audit target `a680c5d`
(5 unpushed commits `211aba8..a680c5d` beyond `origin/main` = `877c41f`).
**Design spec**: `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`, `docs/opi-spec.md`

**Verification method**: all 66 findings (7 codex + 59 glm5.3) were verified
against code at `a680c5d` by six parallel read-only verification agents plus
inline `git`/`gh` checks. Tally: 57 Confirmed, 6 Partially confirmed
(AUD-17-001/006/007, MIN-23/24, INFO-04), 0 Refuted, 0 Cannot-confirm.

---

## Finding cross-reference summary

Majors:

| Cluster | Theme | Sources | Coverage | Source severity | Final severity + rationale | Verification |
|---------|-------|---------|----------|-----------------|----------------------------|-------------|
| C1 | No CI or recorded full-suite validation covers audit HEAD (5 unpushed commits incl. ~15k source + ~8k test rewrite) | glm5.3 M1+M2 | single | Major / Major | **Major** — confirmed inline: `origin/main..HEAD` = 5 commits; `gh run list --commit a680c5d` = 0 runs; newest run 31803930575 at 2026-08-14 | Confirmed |
| C2 | `set_model_validated` persists a `model_change` entry its own runtime application rejects (lookup-only provider: registry-resolvable, not dispatchable) | glm5.3 M3 | single | Major | **Major** — persist→apply order verified; no compensating rollback; reachable via RPC `set_model` | Confirmed |
| C3 | `apply_recorded_model` panics via `.expect` on a recorded non-dispatchable route (resume/fork/branch-switch/builder-resume) | glm5.3 M4 | single | Major | **Major** — `.expect` verified; a 4th call site (builder resume, harness.rs:1873) found beyond the audit's 3 | Confirmed |
| C4 | Bare model selection canonicalizes to the active provider when multiple dispatchable routes share the model id | codex AUD-17-005 | single | Major | **Major** — violates the design-spec general selection invariant ("only when the product can prove exactly one valid route") and P17-PRV-002; behavior pinned by an intentional test (spec-vs-implementation conflict) | Confirmed |
| C5 | Pre-dispatch evidence-emission failure does not stop the not-yet-launched provider request | codex AUD-17-001 | single | Major | **Minor (pin + clarify)** — code facts confirmed; conformant reading wins: EVD-009's stated mechanism is the trusted authorizer, the module contract documents "fail-open for the run", verification column is authorization-side | Partially confirmed |
| C6 | Incomplete evidence still advertises unavailable tools in later provider requests | codex AUD-17-006 | single | Major | **Minor (pin + clarify)** — code facts confirmed; conformant reading wins: AUT-008 recomputation is registration-composition by construction (EffectiveUserPolicy stores active-tool selection only in its digest); health denial is the designed authorization-boundary stable code | Partially confirmed |
| C7 | Public Agent events carry raw user/tool-result content with zero secret scrubbing into NDJSON/RPC | codex AUD-17-007 | single | Major | **Minor (harden)** — passthrough mechanics and canary-test gap confirmed (RPC canary test literally discards the leaking `AgentEnd` line); conversation echo is a recorded in-repo decision and FAL-004/A10 scope to diagnostics/evidence; real residual = recognized credentials cross unscrubbed | Partially confirmed |

Minors (all single-source unless noted; every one verified **Confirmed** except MIN-23/24 partially):

| Cluster | Theme | Sources | Final severity | Verification |
|---------|-------|---------|----------------|-------------|
| C8 | Phase/task history in production source comments | glm5.3 MIN-01 + codex AUD-17-003 (**full 2/2 overlap**) | Minor | Confirmed — 15 sites (audits' 14 + `provider_factory.rs:1902` both missed) |
| C9 | `#[non_exhaustive]` on closed state enums | codex AUD-17-002 | Minor | Confirmed — 2 of 4 mechanically removable; 2 are documented fail-closed conversion design |
| C10 | `FileEvidenceSink` public test-only accessors | codex AUD-17-004 | Minor | Confirmed — `dir()` has zero callers anywhere (understated by audit) |
| C11 | Six adapters attach auth secret unconditionally (no AuthScheme check) | glm5.3 MIN-07 | Minor | Confirmed — check is behavior-preserving for every in-repo caller |
| C12 | AUT-003 "arguments never consulted" phrasing falsifiable | glm5.3 MIN-08 | Minor | Confirmed inline (comment vs `command.authorize(&request.arguments)`); behavior design-conformant |
| C13 | A09 test's rejection leg weakened; ledger rows still cite it | glm5.3 MIN-09 | Minor | Confirmed — and the `ManifestCandidate::validate` ActiveSnapshot branch (evidence.rs:2538-2543) has zero coverage at HEAD |
| C14 | Archived criteria_trace citations stale vs HEAD | glm5.3 MIN-10 (+INFO-24/28 sub-findings) | Minor | Confirmed — staleness confined to the archived snapshot; live ledger clean |
| C15 | CLAUDE.md/AGENTS.md lockstep contract self-false | glm5.3 MIN-11 | Minor | Confirmed — mechanism is a git **symlink** (mode 120000) from `eb5e316`; doc-check guard now vacuous |
| C16 | In-band stream `Error` terminal converted into normally completed turn | glm5.3 MIN-12 | Minor (behavioral) | Confirmed — behavior additionally pinned by `mock_e2e.rs:298-332` |
| C17 | INV-007 required/ignorable entry classification unimplemented | glm5.3 MIN-13 | Minor (defer) | Confirmed — clause tightened post-exit (93d75f4, 2026-08-20) |
| C18 | Unsupported-session-version rejection untested | glm5.3 MIN-14 | Minor | Confirmed |
| C19 | No registration-order permutation tests | glm5.3 MIN-15 | Minor (defer) | Confirmed inline — only `hooks_called_in_registration_order` exists (ordering, not invariance) |
| C20 | Project-trust resolver conflicts resolve first-wins | glm5.3 MIN-16 | Minor (defer) | Confirmed — post-exit tightening; pinned by its own test; latent |
| C21 | Bounded-channel overflow test never saturates | glm5.3 MIN-17 | Minor | Confirmed |
| C22 | Emit-before-setup divergence between sinks unpinned | glm5.3 MIN-18 | Minor | Confirmed + strengthened — two shared contracts, both happy-path-only |
| C23 | Stale-allow reauthorization recovery leg untested | glm5.3 MIN-19 | Minor | Confirmed |
| C24 | Prepare-failure preservation tests weak (len proxy, default priors) | glm5.3 MIN-20 | Minor | Confirmed |
| C25 | `binding_variants` not-normalizable assertion tautological | glm5.3 MIN-21 | Minor | Confirmed |
| C26 | Invalid extra-route configs dropped silently | glm5.3 MIN-22 | Minor | Confirmed — downstream "unknown model" failure traced |
| C27 | Embedder-facing builder/`set_model` panic on invalid model | glm5.3 MIN-23 | Minor→**Major-leaning** | Partially confirmed — production CLI does NOT pre-validate startup model: `opi --model provider:typo` panics at build |
| C28 | Duplicated code: agent-loop arms / InMemory sink / adoption tails / telescoping constructors | glm5.3 MIN-02/03/04/05 | Minor | Confirmed (MIN-05 deferred: public-API churn without behavioral defect) |
| C29 | Speculative `register_extension_tools` seam + discarded parameter | glm5.3 MIN-06 | Minor | Confirmed + dead producer chain (`filter_extension_tools` feeds only the discard) |

Info findings (28, glm5.3): all verified (INFO-04 partially — inventory
corrected: 4 adapters run the preflight, **bedrock** is a 4th skipper,
`api_mapped` performs the equivalent model-resolved check). Disposition in
Scope exclusions.

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|-------------------|----------|-----------|------------|
| D1 | C2, C3 (M3/M4) | Add `validate_dispatchable_route` to `try_configure_model` after the registry check; replace the `apply_recorded_model` `.expect` with the existing typed-diagnostic branch | Single verified root cause (resolvability-vs-dispatchability predicate divergence); matches the collection's documented lookup-only semantics; one fix covers all four resume/fork call sites and feeds the existing `CODE_SESSION_RESUME_MODEL_INCOMPATIBLE` branch | auto |
| D2 | C4 (AUD-17-005) | **Enforce spec**: bare input in `set_model_validated` runs the same unique-dispatchable-route enumeration as the legacy path; 0 matches → typed missing error, >1 → typed ambiguity error, before persist; update the pinned test; bare-on-non-active-provider gets typed remediation instead of a generic parse error | Design spec is explicit ("only when the product can prove exactly one valid route"; PRV-002 "MUST fail"); enumeration helper already exists on the read path; alternatives (amend spec) recorded | **user** (option a) |
| D3 | C5 (AUD-17-001) | **Accept conformant reading + pin**: keep "fail-open for the run"; add a test pinning that the provider attempt proceeds and later launches fail closed after a pre-dispatch emission failure; add one clarifying sentence to the design-spec EVD-009 clause | The spec's stated mechanism is the trusted authorizer; verification column is authorization-side; gating the provider call would make the authorizer mechanism nearly vacuous and contradicts the documented module contract | **user** (accept + pin) |
| D4 | C6 (AUD-17-006) | **Accept conformant reading + pin**: keep registration-only projection; extend the cited product test to assert the post-failure request still advertises the tool AND its launch yields the `evidence_incomplete` denial; add one clarifying sentence to the design-spec AUT-008 clause | Projection is registration-composition by construction (policy stores active-tool selection only in its digest); the design wants the model to receive the visible stable-code denial rather than silent omission | **user** (accept + pin) |
| D5 | C7 (AUD-17-007) | **Echo + secret scrub**: keep the conversation echo as intended; run `Message::User` content, `ToolResultMessage.content`, and `ToolExecutionEnd.result` through value-pattern secret scrubbing (NOT Summary redaction — preserves TUI images/transcripts); add NDJSON/RPC canary tests for `AgentEnd`/`TurnEnd`/`ToolExecutionEnd` | Recorded in-repo decision + pi precedent support the echo; the verified real risk is recognized live credentials crossing unscrubbed; full redaction (alternative) breaks product output | **user** (echo + scrub) |
| D6 | C16 (MIN-12) | **Fail closed**: distinguish the `Error` terminal at the seam; the run fails with a typed provider error (non-retryable), the partial assistant message remains in durable context; update the pinned `mock_e2e` test | Matches the phase's FAL-003 posture; retryability classification (alternative c) would require adapters to surface a redacted retryable class — larger change, not taken | **user** (fail closed) |
| D7 | C27 (MIN-23) | CLI-side typed validation of the startup model spec before `builder.build()`; document the builder/`set_model` panic contract in doc comments (leave the public signatures unchanged) | Production-reachable panic from a config typo contradicts the repo's typed-failure posture; CLI validation is the narrowest fix; `build() -> Result` (alternative) is broader public-API churn not required by the finding | auto |
| D8 | C8 (MIN-01) | Rewrite exactly the 15 verified Phase-17 comment sites; do NOT sweep the 119 repo-wide phase-14..17 references | Minimum change: every changed line must trace to the verified finding; the repo-wide sweep is out of remediation scope (recorded as deferred) | auto |
| D9 | C15 (MIN-11) | Commit to the symlink: revise the AGENTS.md lockstep sentence to the single-content policy, de-vacuate `check_root_guidance_lockstep`, update the opi-document skill line | Truthful doc wording; restoring a flavored CLAUDE.md would undo the deliberate `eb5e316` centralization (alternative recorded) | auto |
| D10 | C9 (AUD-17-002) | Remove `#[non_exhaustive]` from `AuthorizationDecision` and `TerminalOutcome` (same-crate, closed, no wildcard arms); keep it on `AuthProvenanceSource`/`AuthFallback` | The two auth enums' wildcards (evidence.rs:1140/1201) are attribute-forced and documented as intentional fail-closed conversion design ("future unsupported variants cannot silently become static/no-fallback facts") | auto |
| D11 | C10 (AUD-17-004) | Remove `dir()` outright; keep `completed_run_dirs()` with a doc contract as a verification-tooling inspection seam | `dir()` is fully dead; `completed_run_dirs` is required by external integration tests (`#[cfg(test)]` gating would not work for `tests/` binaries) | auto |
| D12 | C26 (MIN-22) | Surface per-route skip diagnostics in `ProviderBundle` diagnostics; keep fail-open-at-startup semantics | Diagnostics point later "unknown model" failures at the broken config without changing startup posture; fail-the-switch (alternative) is a product-behavior change not required by the finding | auto |
| D13 | C29 (MIN-06) | Delete `register_extension_tools`, drop the `_extension_tools` parameter, delete the `filter_extension_tools` collection chain; update the one test | Speculative seam with zero production callers; production currently pays to collect and filter tools it then discards | auto |
| D14 | C17/C19/C20 (MIN-13/15/16) | Defer to the spec-owning flow; record the divergences in the dated addendum (Fix 4.3) | All three clauses were tightened post-exit (93d75f4/aff8875, 2026-08-20); Phase 17's registered contract predates them; MIN-13 needs a durable-format change contradicting the documented additive session policy. MIN-14 (one fixture) is cheap and in-scope | auto |
| D15 | C28-partial (MIN-24) | Defer; record the residual (incl. `output_began` blindness to tool-only prefixes) in the addendum | Discard is a documented intentional rollback contract; a real fix needs new mechanism (retry-time idempotency or read-only retained turns), which is new design, not remediation | auto |

## Remediation layers

### Layer 1: opi-ai (substrate)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-ai --all-targets -- -D warnings
    cargo test -p opi-ai --test per_request_auth
    cargo test -p opi-ai --test <new scheme-mismatch test binary>

#### Fix 1.1: Fail-closed AuthScheme attachment in six adapter paths

- **Finding source**: audit.glm5.3.md (audit, glm5.3), MIN-07
- **Cluster**: C11; **Decision**: D10-adjacent (auto — fail-closed adapter-boundary validation, matches the Anthropic/Bedrock positive pattern)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_chat.rs` ~L1145-1154; `openai_responses.rs` ~L359-365; `openai_codex_responses.rs` ~L154-159; `azure_openai.rs` ~L221, 275-278; `gemini.rs` ~L860, 708-711; `vertex.rs` ~L158, 200-204
- **Change**: before attaching `resolved.secret`, match the prepared `AuthScheme` and reject mismatches with a typed `ProviderError::Config` mirroring `anthropic.rs:957-972`. Accepted sets: Bearer-only for `openai_chat`, `openai_responses`, `openai_codex_responses`, `vertex`; ApiKey-only for `azure_openai`, `gemini`. `api_mapped` needs no own check (pure delegation; delegated routes check).
- **Test plan**: new mismatched-scheme rejection test per wire (extend `per_request_auth.rs` or the adapter fixture suites); existing suites must stay green (verified: no in-repo caller prepares a mismatched scheme — config validation already forbids the only possible mismatch).
- **CHANGELOG**: `[Unreleased]` entry (embedder-visible adapter behavior change).

### Layer 2: opi-agent (core)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-agent --all-targets -- -D warnings
    cargo test -p opi-agent --test evidence_contract --test evidence_runtime --test tool_authority --test hooks_queues --test streaming_proxy --test session_storage

#### Fix 2.1: In-band stream `Error` terminal fails closed (public behavior)

- **Finding source**: audit.glm5.3.md, MIN-12; **Decision**: D6 (user)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` ~L1277-1278 (seam `process_stream_event`), ~L411-429 (consumption), ~L889-1028 (terminal/retry gate)
- **Change**: make the seam distinguish the `Error` terminal from `Done` (e.g. return the terminal reason or a small enum); on an `Error` terminal, preserve the partial assistant message in durable context and end the run with a typed, **non-retryable** provider failure (reuse the existing `Err` failure path so `retain_strongest_terminal_error`/run-failure surfacing apply). Do not attempt retryability classification (adapters intentionally discard the raw class for redaction).
- **Test plan**: new agent-core test driving an in-band `Error` event through a scripted provider (assert typed run failure + partial message retained + zero retries); update pinned `crates/opi-coding-agent/tests/mock_e2e.rs:298-332` (`e2e_error_response_from_provider`) to expect the typed failure (Layer 3).
- **CHANGELOG**: `[Unreleased]` entry.

#### Fix 2.2: Extract shared tool-result completion pipeline

- **Finding source**: audit.glm5.3.md, MIN-02; **Decision**: auto (behavior-preserving extraction)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` L507-866 (`ToolEvidenceContext` at L522-532, 614-622, 678-688, 845-853)
- **Change**: extract one completion helper (event emission → `ToolResultMessage` assembly → outcome emission → context push) shared by the sequential and parallel arms; bundle the ~10 context parameters in a small struct to avoid `too_many_arguments`; keep the sequential arm's stop/skip logic outside the helper. Pure extraction — no assertion or ordering changes.
- **Test plan**: existing `tool_authority`/`hooks_queues`/`evidence_runtime` suites must stay green unchanged.

#### Fix 2.3: Delete the vestigial `assistant_content` accumulator

- **Finding source**: audit.glm5.3.md, INFO-23; **Decision**: auto (verified dead: 3 occurrences, no read site)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs` L263, 388, 416-419 (and the write arms inside `process_stream_event`)
- **Change**: remove the accumulator and drop the parameter from `process_stream_event`.
- **Test plan**: existing suites green.

#### Fix 2.4: Value-pattern secret scrubbing at the public event boundary

- **Finding source**: audit.codex.md (audit, codex), AUD-17-007; **Decision**: D5 (user)
- **Verification status**: Partially confirmed (mechanics confirmed; violation reading resolved as echo-as-intended + hardening)
- **File(s)**: `crates/opi-agent/src/event.rs` ~L266-268 (`Message::User`), ~L348 (`ToolResultMessage.content`), ~L172 (`ToolExecutionEnd.result`)
- **Change**: run these three passthrough surfaces through value-pattern secret scrubbing (the `SecretRedactor` pattern set: sk-/bearer/JWT shapes) — NOT `redact_public_value`/Summary (which would redact paths and destroy transcripts/TUI image rendering). Scrub string leaves of the structured content values. Add a boundary comment recording the D5 decision (conversation echo intended; recognized-credential scrubbing only).
- **Test plan**: Layer-3 canary tests (Fix 3.5) prove absence on NDJSON/RPC terminal shapes; add a focused unit test in opi-agent asserting a secret-shaped canary in user content/tool result is scrubbed while ordinary paths/text pass through.
- **CHANGELOG**: `[Unreleased]` entry.

#### Fix 2.5: `InMemoryEvidenceSink::emit` fails closed before setup

- **Finding source**: audit.glm5.3.md, MIN-18; **Decision**: auto (align oracle with the file adapter's documented fail-closed phase gate)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/evidence.rs` ~L2405-2424
- **Change**: `emit` (and `finalize_*`) reject with the typed `EvidenceError` when the sink has no bound run (no `setup` observed), matching `FileEvidenceSink::emit`.
- **Test plan**: add a before-setup leg to the shared conformance contract `noop_and_in_memory_satisfy_one_lifecycle_conformance_contract` (`crates/opi-agent/tests/evidence_contract.rs:1500-1522`); second contract extension in Layer 3 (Fix 3.6).

#### Fix 2.6: Inherent `InMemoryEvidenceSink` query methods delegate to the trait impl

- **Finding source**: audit.glm5.3.md, MIN-03; **Decision**: auto
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/evidence.rs` L2364-2386 vs L2509-2522
- **Change**: inherent `records()`/`has_failure()`/`completed_manifest()` delegate to the explicitly-qualified `EvidenceRecorder` impl.
- **Test plan**: existing evidence suites green.

#### Fix 2.7: Remove `#[non_exhaustive]` from the two same-crate closed enums

- **Finding source**: audit.codex.md, AUD-17-002; **Decision**: D10 (auto, split)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/authority.rs` ~L253 (`AuthorizationDecision`); `crates/opi-agent/src/evidence.rs` ~L1777 (`TerminalOutcome`)
- **Change**: delete the two attributes. KEEP the attributes on `AuthProvenanceSource`/`AuthFallback` (`crates/opi-ai/src/auth.rs:194,253`) — their wildcard conversion arms (`evidence.rs:1140/1201`) are documented intentional fail-closed design; record this in the plan exclusions.
- **Test plan**: `cargo clippy --workspace --all-targets -- -D warnings` must stay clean (no newly-dead wildcard arms in opi-agent — verified none exist for these two enums); CHANGELOG `[Unreleased]` entry (published-API relaxation).

#### Fix 2.8: Comment-contract rewrites (opi-agent share of C8)

- **Finding source**: audit.glm5.3.md MIN-01 + audit.codex.md AUD-17-003; **Decision**: D8 (auto)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-agent/src/hooks.rs` L21, 52, 63, 124; `agent.rs` L572; `evidence.rs` L133, 226, 1792; `extension.rs` L246, 676; `sdk.rs` L118 (11 sites)
- **Change**: rewrite each comment to state the current contract only, dropping `Phase 17.x` / `(17.x)` tokens (keep the invariant text).
- **Test plan**: `python scripts/opi-doc-check.py` (verified: no doc-guard pins these strings).

#### Fix 2.9: Test-strength fixes in opi-agent suites

- **Finding source**: MIN-19 (C23), MIN-20 + INFO-12 (C24), MIN-21 (C25), MIN-14 (C18), MIN-17 (C21), MIN-09 core half (C13); **Decision**: auto
- **Verification status**: all Confirmed
- **File(s) / changes / tests**:
  - `crates/opi-agent/tests/tool_authority.rs` (+ `tests/common/mod.rs`): new stale-first/fresh-second authorizer fixture with a request counter — assert the tool executes exactly once and `authorize` was called exactly twice; optionally pin `stable_code: "authorization_stale"` on the still-stale leg. (MIN-19)
  - `crates/opi-agent/tests/hooks_queues.rs` L1452-1489, L1536-1574: install the non-default baseline (max_tokens 1111, temperature 0.25, extra assistant message) and assert via `assert_prior_state_plus_prompt` instead of `len() == 1`. (MIN-20)
  - same file ~L1405-1412: tighten the A04 context leg from `len >= 3` to exact equality matching the sibling helper. (INFO-12, rides along in the same file)
  - `crates/opi-agent/tests/evidence_contract.rs` L737-751: replace the test-local `kind()` tautology with the render + `BTreeSet` uniqueness pattern from `measurement_origins_are_distinct` (L789-806). (MIN-21)
  - new test beside `ManifestCandidate::validate` coverage: an `ActiveSnapshot`-bound candidate through `validate` → `Err` (closes the only untested enforcement branch of "a direct run must not claim an ActiveSnapshot", evidence.rs:2538-2543). (MIN-09)
  - session tests: write a header with `version: 2` (and `0`) and assert `read_with_recovery` returns `InvalidData`. (MIN-14)
  - `crates/opi-agent/tests/streaming_proxy.rs` L488-507: make the bounded-channel test actually saturate (handler emits > capacity events during one command) and assert the delivered count; drop semantics stay as-is (consumer-visible overflow signal = recorded design decision, not taken). (MIN-17)

### Layer 3: opi-coding-agent (product)

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test phase17_provider_runtime --test phase17_product_evidence --test phase17_tool_authority --test mock_e2e --test json_mode --test rpc_jsonl --test phase17_cross_mode --test phase17_failure_rollback

#### Fix 3.1: Validate dispatchability in `try_configure_model` (fixes M3 + M4)

- **Finding source**: audit.glm5.3.md, M3 + M4; **Decision**: D1 (auto)
- **Verification status**: Confirmed (both)
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L2021-2046 (`try_configure_model`), ~L2368-2369 (`apply_recorded_model`)
- **Change**:
  1. In `try_configure_model`, after the `model_info` resolve, call `self.model_registry.validate_dispatchable_route(model_spec)` and map failures to the existing error type; update the doc comment ("accepted as long as it resolves to a registered route" → dispatchable route).
  2. Replace `apply_recorded_model`'s `.expect("recorded model was validated before application")` with a `match` feeding the same `CODE_SESSION_RESUME_MODEL_INCOMPATIBLE` diagnostic branch used one step earlier (belt-and-suspenders; behavior-preserving for every currently-surviving input).
- **Test plan**: (a) `set_model_validated` with a lookup-only extension provider registered → `Err` AND the session unchanged (no appended `model_change`); (b) resume/fork of a session whose latest `model_change` records a lookup-only route → typed diagnostic, no panic. Covers all four call sites (incl. builder resume at harness.rs:1873).

#### Fix 3.2: Bare-model unique-route enumeration on the write path

- **Finding source**: audit.codex.md, AUD-17-005; **Decision**: D2 (user — enforce spec)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L1995-2033 (`set_model_validated` bare branch + `try_configure_model` fallback)
- **Change**: for bare input, enumerate dispatchable routes serving the model id (reuse the read path's `dispatchable_provider_ids` enumeration from `normalize_recorded_route` ~L2378-2395): exactly one → canonicalize + persist with `BareNormalized`; zero → typed missing-model error; more than one → typed ambiguity error — both before any durable write. Bare ids served only by a non-active provider now get the typed remediation instead of the generic `parse_model_spec` error.
- **Test plan**: update the pinned test `phase17_provider_runtime.rs:398-441` (currently asserts active-provider normalization) to assert ambiguity rejection; add a two-provider/same-model test asserting `Err`, unchanged state, zero dispatches.
- **CHANGELOG**: `[Unreleased]` entry (user-visible behavior change).

#### Fix 3.3: CLI startup model validation (production panic)

- **Finding source**: audit.glm5.3.md, MIN-23; **Decision**: D7 (auto)
- **Verification status**: Partially confirmed (production reachability is worse than the audit stated)
- **File(s)**: `crates/opi-coding-agent/src/main.rs` ~L1229-1265 (startup assembly); `harness.rs` ~L1718-1728 (builder doc comment)
- **Change**: validate the configured startup model spec against the provider catalog (registry resolve + dispatchability) before `builder.build()`; exit with a typed diagnostic on failure (`opi --model anthropic:not-a-real-model` currently panics). Add a doc-comment line on `CodingHarness` builder `build()` stating the panic contract for embedders.
- **Test plan**: CLI/binary test (or builder-level test mirroring the validation) asserting a typed startup error for an invalid configured model.

#### Fix 3.4: Session-adoption tail extraction

- **Finding source**: audit.glm5.3.md, MIN-04; **Decision**: auto
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` L2258-2300, 2451-2487, 2562-2594
- **Change**: extract a private `adopt_session_entries` helper parameterized on (path, session_id, entries, message_count, error-context string, clear-resume-error flag); preserve per-site differences (error strings; only `resume_session_id` clears `session_resume_error`; fork's returned session_id; `open_existing` after `apply_recorded_model`).
- **Test plan**: existing resume/fork/branch suites green unchanged.

#### Fix 3.5: Public-event canary coverage (NDJSON + RPC terminal shapes)

- **Finding source**: audit.codex.md, AUD-17-007; **Decision**: D5 (user)
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/tests/json_mode.rs` (new test); `crates/opi-coding-agent/tests/rpc_jsonl.rs` (extend `phase17_canary_is_absent_from_rpc_jsonl:4816-4872`)
- **Change**: NDJSON test plants a secret-shaped canary in a tool result and the prompt and scans `AgentEnd`/`TurnEnd`/`ToolExecutionEnd` lines for it (must be absent post-Fix-2.4); the RPC test must scan the full event stream instead of discarding it via `recv_until_agent_end`.
- **Test plan**: the tests themselves.

#### Fix 3.6: Extend the product shared conformance contract with a before-setup leg

- **Finding source**: audit.glm5.3.md, MIN-18 (second contract found by verification)
- **Decision**: auto; **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_product_evidence.rs:790-809` (`assert_complete_recorder_lifecycle`)
- **Change**: add the before-setup leg (emit without setup → typed error) for both file and in-memory recorders.
- **Test plan**: the contract itself.

#### Fix 3.7: Restore the manifest-rejection leg of the A09 product test

- **Finding source**: audit.glm5.3.md, MIN-09 (product half); **Decision**: auto
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_product_evidence.rs:2063-2075`
- **Change**: beside the digest-constructor check, construct an `ActiveSnapshot`-bound `ManifestCandidate` and assert `validate` fails (restores the test's name-claim at the now-correct level).
- **Test plan**: the test itself (complements Fix 2.9's core-level test).

#### Fix 3.8: Pin EVD-009 and AUT-008 conformant semantics

- **Finding source**: audit.codex.md, AUD-17-001 + AUD-17-006; **Decision**: D3 + D4 (user — accept + pin)
- **Verification status**: Partialially confirmed (both)
- **File(s)**: `crates/opi-agent/tests/evidence_runtime.rs:1070-1127` (extend); `crates/opi-coding-agent/tests/phase17_product_evidence.rs:1633-1691` (extend)
- **Change**: (a) emission-failure test additionally asserts the provider attempt proceeded and completed (pinning "fail-open for the run" deliberately) while later launches fail closed; (b) product test additionally asserts the second request still advertises `write` AND the launch is denied with stable code `evidence_incomplete`.
- **Test plan**: the tests themselves.

#### Fix 3.9: Surface silently-dropped extra-route diagnostics

- **Finding source**: audit.glm5.3.md, MIN-22; **Decision**: D12 (auto)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs` L1526, 1544-1575
- **Change**: collect per-route non-secret skip reasons into `ProviderBundle` diagnostics (three `if let Ok(route)` sites); startup remains fail-open.
- **Test plan**: new test asserting a broken proxy/profile config yields a startup diagnostic naming the dropped provider.

#### Fix 3.10: Remove the speculative extension-tool registration seam

- **Finding source**: audit.glm5.3.md, MIN-06; **Decision**: D13 (auto)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool_authority.rs` L96-116, 123-128; `crates/opi-coding-agent/src/harness.rs` ~L1442, 1477 (`filter_extension_tools` chain)
- **Change**: delete `register_extension_tools` (update its one test consumer `tests/phase17_tool_authority.rs:467`), drop the discarded `_extension_tools` parameter from `register_product_tools`, delete the now-dead `filter_extension_tools` collection chain.
- **Test plan**: affected suites green; CHANGELOG `[Unreleased]` (public item removal, 0.x).

#### Fix 3.11: `FileEvidenceSink` accessor cleanup

- **Finding source**: audit.codex.md, AUD-17-004; **Decision**: D11 (auto)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/evidence.rs` L114-122
- **Change**: remove `dir()` (zero callers anywhere); keep `completed_run_dirs()` with a doc contract naming it a verification-tooling inspection seam.
- **Test plan**: existing suites green; CHANGELOG `[Unreleased]`.

#### Fix 3.12: Truthful tool-authority comments + spec-citation fixes

- **Finding source**: audit.glm5.3.md, MIN-08 + INFO-03; **Decision**: auto (truthful doc wording)
- **Verification status**: Confirmed (both)
- **File(s)**: `crates/opi-coding-agent/src/tool_authority.rs` L11-16, L468-470 (AUT-003 phrasing), L58, L146-147, L152 (spec line-number citations)
- **Change**: reword to "arguments select the adapter binding inside the trusted boundary; no permission fact derives from argument content"; replace the three `(spec lines …)` citations with the normative concept names (capability map / effective-policy sections).
- **Test plan**: `python scripts/opi-doc-check.py`.

#### Fix 3.13: Comment-contract rewrites (opi-coding-agent + opi-ai share of C8)

- **Finding source**: MIN-01 + AUD-17-003; **Decision**: D8 (auto)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/registry.rs:277`; `crates/opi-coding-agent/src/adapter_extension.rs:497`; `crates/opi-coding-agent/src/execution/permission.rs:79`; `crates/opi-coding-agent/src/provider_factory.rs:1902`
- **Change**: drop the phase/task tokens, keep contract text.
- **Test plan**: doc-check.

#### Fix 3.14: Redact the RPC `tree_read_error` field

- **Finding source**: audit.glm5.3.md, INFO-27; **Decision**: auto (matches sibling fields' established pattern)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/rpc.rs` L956-960
- **Change**: wrap the formatted error in `redact_text(..., RedactionMode::Summary)` like the sibling `tree_recovery`/`branch_summary` fields.
- **Test plan**: extend an RPC session_info test asserting the path-bearing error text is summarized.

#### Fix 3.15: Write measured values into the 17.9 artifact bundle

- **Finding source**: audit.glm5.3.md, INFO-16; **Decision**: auto (artifact-truthfulness convention)
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase17_cross_mode.rs` L797-853
- **Change**: `provider-assertion.json` / `tool-execution-counts.json` / `exit-code.txt` write the measured values (`calls1.len()`, real exit codes, real counters) instead of literals, so `RUN_SUMMARY` "verified" rows are truthful.
- **Test plan**: the suite itself (in-test assertions unchanged).

#### Fix 3.16: Remove the unused `tracing-subscriber` dependency

- **Finding source**: verification pass discovery adjacent to INFO-17 (declared in `crates/opi-coding-agent/Cargo.toml:45`, zero uses workspace-wide)
- **Decision**: auto (supply-chain hygiene: dependency changes are reviewed code; unused dep removal is invisible)
- **Verification status**: Confirmed by verification agent grep
- **File(s)**: `crates/opi-coding-agent/Cargo.toml` (+ regenerate `Cargo.lock` via Cargo, never hand-edited)
- **Change**: drop the unused declaration.
- **Test plan**: `cargo check -p opi-coding-agent`; INFO-17 itself (verbatim stderr at debug level) is **no action** — no subscriber is installed in shipped binaries.

### Layer 4: Documentation

**Verification**:

    python scripts/opi-doc-check.py

#### Fix 4.1: Make the guidance-lockstep contract truthful (symlink policy)

- **Finding source**: audit.glm5.3.md, MIN-11; **Decision**: D9 (auto)
- **Verification status**: Confirmed
- **File(s)**: `AGENTS.md` L33 (lockstep sentence); `scripts/opi-doc-check.py` `check_root_guidance_lockstep` (L279-300, currently vacuous); `.agents/skills/opi-document/SKILL.md` L58 (and the `.claude/skills/` counterpart if present)
- **Change**: state the single-content policy (CLAUDE.md is a symlink to AGENTS.md; keep them one file); replace the vacuous four-flavor normalization with a check that CLAUDE.md resolves to AGENTS.md (fails if a real divergent file reappears); update the skill line. Note the Windows-clone portability caveat (symlinks materialize as text without symlink support).
- **Test plan**: doc-check.

#### Fix 4.2: One-sentence spec clarifications (user-approved via D3/D4)

- **Finding source**: audit.codex.md, AUD-17-001 + AUD-17-006; **Decision**: D3 + D4 (user)
- **Verification status**: Partialially confirmed
- **File(s)**: `docs/superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md` (EVD-009 and AUT-008 rows)
- **Change**: EVD-009: clarify that provider model requests are not authorization-gated side effects; fail-closed applies at the tool launch boundary. AUT-008: clarify that projection recomputation means composition from trusted registrations; evidence-health denial surfaces as the authorization boundary's stable code. (Spec-status rules respected: no implementation status recorded.)
- **Test plan**: doc-check; spec edit re-syncs the live ledger spec hash only via the owning workflow if required (do not hand-edit `.opi-impl-state.json`).

#### Fix 4.3: Dated citation addendum for the archived ledger

- **Finding source**: audit.glm5.3.md, MIN-10 + INFO-24 + INFO-28 + INFO-05 + INFO-06 + INFO-13 + INFO-15 (+ the C17/C19/C20 divergence records from D14)
- **Decision**: auto; **Verification status**: Confirmed
- **File(s)**: new `docs/snapshots/phase17/citation-addendum-2026-08-21.md`
- **Change**: map stale criteria_trace citations to their HEAD locations (agent.rs collection field, agent_loop prepare/apply at 1065-1193, preflight/execute split, UUIDv7 `IdentityAllocator` replacing `RUN_ID_COUNTER`, `ManifestCandidate::validate`/`FinalizedManifest` replacing `require_complete`); record the corrected residuals (PRV-005 narrowed at HEAD; cross-mode `--trace` asymmetries closed; A03 "one call identity" wording; PRV-006 wiremock drives four adapters); record the deferred tightened-invariant divergences (MIN-13 classification, MIN-15 permutation tests, MIN-16 project-trust first-wins, MIN-24 rollback residual). The archived ledger itself is NOT rewritten.
- **Test plan**: doc-check.

#### Fix 4.4: CHANGELOG `[Unreleased]` entries

- **File(s)**: `CHANGELOG.md`
- **Change**: one consolidated entry set covering: adapter AuthScheme fail-closed attachment; bare-model ambiguity/missing typed errors; in-band stream error → typed failure; secret-scrubbing at the public event boundary; CLI startup model validation; removed public items (`FileEvidenceSink::dir`, `register_extension_tools`, `_extension_tools` parameter, two `#[non_exhaustive]` markers). Read the full `[Unreleased]` section first; reuse existing subsections.

## Final verification

    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings   # at CI scope, after fmt
    cargo test --workspace --all-targets                     # per-target on this host (disk constraints), union at CI scope
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Then (C1 / M1+M2 closure — requires explicit user authorization to push):

1. Commit the remediation (explicit paths only; Conventional Commits).
2. Push `main` (all unpushed commits, `211aba8..HEAD`).
3. Confirm the three-platform CI matrix green at the final SHA (`phase17_acceptance` + workspace jobs) and record the run URL in the citation addendum.
4. Until then, the archived ledger's "ran green" rows remain historical (the glm5.3 audit's local HEAD validation — 621 focused tests, fmt, clippy-lib, doc-check — is the only validation attaching to `a680c5d`).

## Scope exclusions

| Finding | Status | Reason |
|---------|--------|--------|
| glm5.3 MIN-13 | Deferred | INV-007 entry classification tightened post-exit (93d75f4); needs a durable-format change contradicting the documented additive session policy — route to the spec-owning flow (D14) |
| glm5.3 MIN-15 | Deferred | Post-exit PRIN-004/INV-005 permutation-matrix verification route; route to a tasked phase (D14) |
| glm5.3 MIN-16 | Deferred | Post-exit tightening; first-wins is pinned by its own documented embedder test; latent (standard CLI registers no resolvers); divergence recorded in addendum (D14) |
| glm5.3 MIN-24 | Deferred | Documented intentional rollback contract; fix needs new mechanism (retry idempotency / read-only retained turns); `output_began` blindness noted in addendum (D15) |
| glm5.3 MIN-05 | Deferred | Telescoping-constructor consolidation is public-API churn without a behavioral defect; verified fix shape (params struct per runner family) recorded for a dedicated cleanup |
| glm5.3 MIN-23 (builder/`set_model` half) | Partially deferred | CLI-side fix taken (Fix 3.3); `build() -> Result` and typed `set_model` are broader public-API changes recorded as alternatives, not taken (D7) |
| codex AUD-17-001 / AUD-17-006 (semantic-change option) | Excluded by decision | User accepted the conformant reading (D3/D4); fixes are pinning tests + spec clarifications only |
| codex AUD-17-007 (full-redaction option) | Excluded by decision | User chose echo + secret-scrub (D5); full redaction breaks TUI images/transcripts and exceeds spec scope |
| glm5.3 INFO-01 | Info/No action | Both `facts()` and `Deref` have distinct real consumer sets |
| glm5.3 INFO-02 | Info/No action | Double parse is real; the tightening side effect (trimmed canonical ids) is a behavior change not requested |
| glm5.3 INFO-04 | Info/No action | Collection-level validation covers all dispatched calls; corrected inventory recorded in addendum (4 adapters preflight; bedrock also skips; api_mapped equivalent) |
| glm5.3 INFO-07 | Info/No action | Design-conformant (RPC `trace` command requires the recorder); recorded in addendum |
| glm5.3 INFO-08 | Info/No action | Both call sites guarded (second structurally); typed-error conversion noted as future hardening |
| glm5.3 INFO-09 | Info/No action | Latent (no phase17 tests exist in the three unscanned crates today); noted |
| glm5.3 INFO-10 | Info/No action | Future work (Promotion-Controller world); no dual durable owner exists today |
| glm5.3 INFO-11 | Info/No action | Mid-run re-setup divergence remains (MIN-18 fix covers emit-before-setup only); within a single lifecycle EVD-008 holds in both |
| glm5.3 INFO-14 | Info/No action | Behavioral proof exists in the runtime test; contract-level test kept |
| glm5.3 INFO-17 | Info/No action | No subscriber installed in shipped binaries; the unused dependency is removed instead (Fix 3.16) |
| glm5.3 INFO-18 / INFO-19 | Info/No action | Memory-hygiene defense-in-depth; no output path exposes them (sigv4.rs scope noted in addendum) |
| glm5.3 INFO-20 | Info/No action | Latent (no production `{:?}` print path); helper retained |
| glm5.3 INFO-21 | Info/No action | Durability-vs-latency design decision; per-record flush + finalize fsync retained |
| glm5.3 INFO-22 | Info/No action | Bounded cost (PathBufs per completed run); noted |
| glm5.3 INFO-25 / INFO-26 | Info/No action | Latent; no in-repo caller/resolver hits the paths |
| Repo-wide phase-history sweep (119 phase-14..17 refs beyond the 15 verified sites) | Deferred | Out of finding scope (D8); minimum-change rule |
| Restoring a flavored CLAUDE.md file | Excluded by decision | Would undo the deliberate `eb5e316` centralization; symlink policy adopted instead (D9) |
