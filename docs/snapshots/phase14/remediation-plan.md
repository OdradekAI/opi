# Phase 14 Remediation Plan

**Date**: 2026-07-20
**Audit sources**: `docs/snapshots/phase14/audit.codex.md`, `docs/snapshots/phase14/audit.glm5.2.md` (both re-issued against HEAD `b27905a`)
**Baseline under review**: `b27905a` (current HEAD; no uncommitted code — only the two audit `.md` files are modified in the working tree)
**Phase 14 task range**: `d9f21a9..8364e74` (phase exit) plus post-exit remediation `9263114..b27905a`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`, `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`
**Verification method**: 14 adversarial code-verification subagents + 7 independent cross-check subagents (Majors), each reading cited files in full, tracing the cited path, and re-rating severity by production reachability.

---

## Auditor split and trust model

Two independent auditors reviewed the same HEAD `b27905a` and split sharply:

- **codex**: FAIL — 7 Major, 2 Minor.
- **glm5.2**: PASS-WITH-FINDINGS — 0 Major, 4 Minor, 9 Info; explicitly claims the prior C1–C21 set is "genuinely fixed at HEAD".

This is the same disagreement pattern documented in the prior phase-14 remediation pass (codex FAIL vs glm5.2/opus4.6 PASS). The established lesson: **do not majority-vote** — the PASS auditor can miss whole defect classes. Every codex Major was verified against actual code; all 14 findings were **Confirmed as real** (0 refuted). Severity was then re-rated by production reachability (the opi binary's four run modes all route through `agent_loop`, which masks several defects; library-consumer paths were evaluated separately).

---

## Audit cross-reference summary

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---------|-------|----------|-----------|------------------|--------------|
| CL1 | Responses/Codex SSE data-only decode | codex 2.1 | unique (codex) | **Blocker** | Confirmed + xcheck agree |
| CL2 | Anthropic block-event underflow panic | codex 2.2 | unique (codex) | Minor | Confirmed + xcheck agree |
| CL3 | OAuth duration fields panic | codex 2.3 | unique (codex) | Minor | Confirmed + xcheck agree |
| CL4 | Session cost reprices historical usage | codex 2.4 | unique (codex) | Minor | Confirmed + xcheck agree |
| CL5 | Thinking-level enforcement gap | codex 3.1, glm5.2 5.1 | **disagree** (codex Major; glm5.2 Info) | Minor | Confirmed + xcheck (Major→Minor) |
| CL6 | `AccountIdMissing` loses typed semantics | codex 3.2 | unique (codex) | **Major** | Confirmed + xcheck agree |
| CL7 | Cancellation not observed pre-code/pre-header | codex 3.3 | unique (codex) | Minor | PartiallyConfirmed + xcheck agree |
| CL8 | Invalid PKCE callback returns 200 | codex 4.1 | unique (codex) | Minor | Confirmed |
| CL9 | Large manual input blocks pipe | codex 4.2 | unique (codex) | Minor | Confirmed |
| CL10 | Envelope encode buffer not zeroized | glm5.2 3.1 | unique (glm5.2) | Minor | Confirmed |
| CL11 | Keyring lease guard debug-only | glm5.2 3.2 | unique (glm5.2) | Info | PartiallyConfirmed (no-action) |
| CL12 | Three divergent reserved-header lists | glm5.2 3.3 | unique (glm5.2) | Minor | Confirmed |
| CL13 | `refresh_models` determinism test asserts nothing | glm5.2 4.1 | unique (glm5.2) | Minor | Confirmed |
| CL14 | Codex subset diagnostics collapse to generic | glm5.2 5.4 | unique (glm5.2) | Info | Confirmed (no-action) |

Severity re-rating rationale (production reachability, the dominant factor):

- **CL1 Blocker (upgraded from codex Major)**: Canonical Codex SSE is data-only (`.repo/pi-0.80.6` fixture unambiguous). opi's parser dispatches on the SSE `event:` field only; a data-only frame defaults to `message` → catch-all → `ResponsesEvent::Error` → terminal on frame 1. Standard Responses is also broken mid-stream (only 9 of the real event names are handled). No masking: parsing runs inside the provider stream task and the Error flows through the channel to `agent_loop` in all four run modes. Tests pass only because every fixture injects a synthetic `event:` line.
- **CL6 Major (kept)**: `AccountIdMissing` is reachable in all four modes; `agent_loop:106` preflight inspects model capabilities only, not auth, so it does not preempt the error. The catch-all at `agent_loop:509-525` drops the typed semantics; embedders using NDJSON/RPC to drive re-auth lose the trigger. Diagnostic substrate preserves the type, but exit code (4 vs 3), NDJSON/RPC events, and interactive retry arming do not.
- **CL5 Minor (downgraded from codex Major)**: The C8 silent-omit regression *was* fixed by `b27905a` (the `9263114` Chat/Responses exemption was reverted). `agent_loop:106` preflights `validate_request_capabilities` before `provider.stream` at `:122` for every wire, so the opi binary cannot reach any silent-omit/fallback path. Residual is a published-library API gap (consumer bypassing the documented validator) + a validator doc-contradiction + a harness UX papercut.
- **CL2/CL3 Minor (downgraded from codex Major)**: Real panics on untrusted input, but constrained threat models (Anthropic malformed block ordering; trusted first-party OAuth endpoints over HTTPS) — defense-in-depth, not remotely-exploitable DoS.
- **CL4 Minor (downgraded from codex Major)**: Real correctness defect (mixed-model resume/fork reprices historical usage), but scope is best-effort cost display only — drives no `agent_loop` decision, no tool, no request, no billing.
- **CL7 Minor (PartiallyConfirmed)**: Production provider-stream cancellation is masked (`agent_loop` drops the stream on cancel → receiver-drop aborts the task). Residual: PKCE Ctrl-C lag during login (UX) and library-consumer reach.
- **CL11 Info / CL14 Info (no-action)**: CL11's underflow is precluded by the existing `self.leased` invariant (zero reach); CL14 is display-only message polish.

---

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|----|-------------------|----------|-----------|------------|
| D1 | CL1 (C-2.1) | Decode the JSON `type` field in `try_from_frame`; add the missing event handlers; change the catch-all to ignore unknown non-error types instead of erroring; update fixtures to canonical data-only framing. | Single clear fix; verifier and cross-check converge. Code matches audit exactly; both remediation commits never touched the dispatch logic. | auto |
| D2 | CL6 (C-3.2) | Add a new `AgentError::AccountIdMissing { provider_id }` variant in `loop_types.rs`; add the `agent_loop` mapping arm; add matching arms in `runner.rs`, `rpc.rs`, `interactive.rs`. | User chose option (c) over (a) `AuthFailed` and (b) reuse `CredentialRevoked`. (c) is the only option that preserves the original message, keeps "missing account id" distinct from `CredentialRevoked` per exit-remediation 1014 vs 1017, and yields full downstream integration (remediation event + RPC event + retry arming). It is a breaking change at 0.x (the enum is not `#[non_exhaustive]`) → recorded under `### Breaking Changes`. | user |
| D3 | CL5 (C-3.1) | Fix only the cheap correctness parts now: (a) gate `thinking_level_map.resolve` on `request.thinking.enabled` in `validate_request_capabilities`; (b) add a `thinking_level_map` check to harness thinking-change paths. **Defer** the `Provider::stream` dispatch-seam refactor to a carry-over. | Production is fully masked by the `agent_loop` preflight; the residual is a published-library API gap, not a binary defect. The dispatch-seam refactor is the substantive/low-ROI part; the doc-contradiction and harness papercut are trivial. Scope decision: Critical + cheap Minors. | auto + user (scope) |
| D4 | CL2, CL3, CL8, CL9, CL10, CL12, CL13 | Apply the convergent minimal fix for each (safe indexing, checked OAuth arithmetic, parse-before-200, concurrent drain, `Zeroizing` buffer, header-list unification, determinism assertion). | Each has a single reasonable fix; verifiers and cross-checkers converge. All confirmed real. | auto |
| D5 | CL4, CL7, CL11, CL14 | **Defer** CL4 (best-effort display-cost redesign), CL7 (production-masked cancellation), CL11 (no-action Info), CL14 (display-only polish). | Scope decision: Critical + cheap Minors. Each deferral rationale recorded in *Scope exclusions*. | user (scope) |

---

## Remediation layers

### Layer 1: opi-ai (substrate)

**Verification**:

    cargo fmt -p opi-ai
    cargo clippy -p opi-ai --all-targets -- -D warnings
    cargo test -p opi-ai --all-targets
    cargo test -p opi-ai --doc

#### Fix 1.1: Responses/Codex SSE decoder — dispatch on JSON `type` (CL1, Blocker)

- **Audit source**: codex 2.1
- **Cluster**: CL1
- **Decision**: D1
- **Verification status**: Confirmed (+ cross-check agree)
- **File(s)**: `crates/opi-ai/src/openai_responses_shared.rs` ~L49-53, L199-330, L326-328, L529-539; `crates/opi-ai/src/openai_responses.rs` ~L439-459; `crates/opi-ai/src/openai_codex_responses.rs` ~L227-253
- **Change**: In `ResponsesEvent::try_from_frame`, parse the top-level JSON `type` from `frame.data` and dispatch on it; use the SSE `event:` name only as a validated fallback. Add handlers for `response.in_progress`, `response.content_part.done`, `response.function_call_arguments.done` (finalize tool args), `response.incomplete` (terminal `Done` with `stop_reason=Length`, surface `incomplete_details.reason`), `response.failed` (terminal Error), and `response.reasoning_summary_text.delta/.done` + `response.reasoning_text.delta/.done` (map to thinking deltas). Change the catch-all at L326-328 from emitting `ResponsesEvent::Error` to ignoring unknown non-error types (Codex emits protocol extensions). Keep `error` as the only SSE type that yields `ResponsesEvent::Error`. Drop the `"message"` default at L49-53 so an absent `event:` leaves it empty and the JSON-type dispatch is authoritative. No change to the C6 redaction constant (`OPENAI_RESPONSES_STREAM_ERROR`) — the error path is preserved for genuine errors.
- **Test plan**: Update fixtures in `tests/openai_responses_fixtures.rs`, `tests/openai_responses_lifecycle.rs`, `tests/openai_codex_responses.rs` to canonical data-only framing (mirror `.repo/pi-0.80.6/packages/ai/test/openai-codex-stream.test.ts:48-86`); add a data-only full-sequence fixture exercising `response.in_progress` → content/reasoning deltas → `response.function_call_arguments.done` → `response.completed`, and a `response.incomplete` length-termination fixture asserting a non-error `Done`. The existing `dedicated_codex_valid_error_sse_never_surfaces_message_or_event_name` C6 redaction test must be retargeted to a genuine `error` SSE frame. Tests stay network-free (fixture-based).

#### Fix 1.2: Anthropic block-event safe indexing (CL2, Minor)

- **Audit source**: codex 2.2
- **Cluster**: CL2
- **Decision**: D4
- **Verification status**: Confirmed (+ cross-check agree)
- **File(s)**: `crates/opi-ai/src/anthropic.rs` ~L468-469, L517-518
- **Change**: In the `ContentBlockDelta` and `ContentBlockStop` arms, use the upstream `index` with `self.blocks.get_mut(index)` / `self.partial.content.get_mut(index)` instead of `self.blocks.len() - 1`. On missing, out-of-range, or type-mismatched block, emit no stream event (or push a non-retryable `ProviderError::StreamError` for stricter contract enforcement).
- **Test plan**: New tests in `tests/anthropic_*.rs` for delta-before-start, stop-before-start, wrong-index, and interleaved-index event sequences (fixture-based, no network).

#### Fix 1.3: Thinking validator `enabled` gate (CL5 cheap part, Minor)

- **Audit source**: codex 3.1
- **Cluster**: CL5
- **Decision**: D3
- **Verification status**: Confirmed (downgraded Major→Minor)
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L264-281 (`validate_request_capabilities`)
- **Change**: Gate `thinking_level_map.resolve` on `request.thinking.enabled` so a disabled request with a stale level is not rejected, matching the helper's own doc-comment.
- **Test plan**: New unit test: disabled thinking with an unmappable level passes the validator; enabled thinking with the same level is rejected before network.
- **Note**: The `Provider::stream` dispatch-seam refactor (the public-API library gap) is **deferred** — see *Scope exclusions* CL5b.

#### Fix 1.4: Unify reserved-header lists (CL12, Minor)

- **Audit source**: glm5.2 3.3
- **Cluster**: CL12
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider.rs` ~L166-205 (`validate_extra_headers`, 5-name `RESERVED`); `crates/opi-ai/src/openai_codex_responses.rs` ~L380 call site
- **Change**: Delete the self-admitted "non-exhaustive" `validate_extra_headers`/`RESERVED`; route the Codex call site through `ProviderHeaders::merge_request` with `MANAGED_HEADERS` folded into the route set, leaving `RESERVED_PROVIDER_HEADERS` (`provider_headers.rs:10-24`) as the single canonical reserved list. No live vulnerability today; this removes the refactor hazard where a future provider inherits the weak 5-name gate.
- **Test plan**: Existing Codex header-injection negative tests must still reject `chatgpt-account-id`, `session-id`, `x-client-request-id`, `openai-beta`, `originator`; add a test asserting the canonical list is the sole gate.

#### Fix 1.5: `refresh_models` determinism assertion (CL13, Minor, test-only)

- **Audit source**: glm5.2 4.1
- **Cluster**: CL13
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/provider_collection.rs` ~L1112-1138
- **Change**: Add `assert_eq!(collection.provider_ids(), vec!["alpha", "mike", "zulu"]);` after the register loop and/or after `refresh()` so a regression to insertion order would be caught.
- **Test plan**: The modified test itself.

### Layer 2: opi-agent (→ opi-ai)

**Verification**:

    cargo fmt -p opi-agent
    cargo clippy -p opi-agent --all-targets -- -D warnings
    cargo test -p opi-agent --all-targets
    cargo test -p opi-agent --doc

#### Fix 2.1: `AccountIdMissing` typed `AgentError` variant (CL6, Major — upstream half)

- **Audit source**: codex 3.2
- **Cluster**: CL6
- **Decision**: D2
- **Verification status**: Confirmed (+ cross-check agree)
- **File(s)**: `crates/opi-agent/src/loop_types.rs` (enum + `Display`); `crates/opi-agent/src/agent_loop.rs` ~L509-525
- **Change**: Add `AgentError::AccountIdMissing { provider_id: String }` with a `#[error("...")]` message that preserves the "missing account id" wording. In the stream-error match at `agent_loop.rs:509-525`, add an arm mapping `opi_ai::provider::ProviderError::AccountIdMissing { provider_id }` → `AgentError::AccountIdMissing { provider_id }` (before the catch-all `_ => AgentError::Provider(e.to_string())` at L524).
- **Test plan**: New `agent_loop`-level integration test (using `opi_ai::test_support::MockProvider` configured to return `ProviderError::AccountIdMissing`) asserting the surfaced `AgentError` variant. The provider-local test at `openai_codex_responses.rs:404-432` already defends the provider side.
- **Breaking change**: record under `### Breaking Changes` in `CHANGELOG.md` (Layer 4).

### Layer 3: opi-coding-agent (→ opi-ai, opi-agent)

**Verification**:

    cargo fmt -p opi-coding-agent
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets
    cargo test -p opi-coding-agent --doc

#### Fix 3.1: `AccountIdMissing` downstream integration (CL6, Major — downstream half)

- **Audit source**: codex 3.2
- **Cluster**: CL6
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/runner.rs` ~L690-703, L725-735; `crates/opi-coding-agent/src/rpc.rs` ~L1000-1032; `crates/opi-coding-agent/src/interactive.rs` ~L419-448
- **Change**: Add the new variant to: `append_credential_remediation` (emit canonical provider id + `/login <provider>` — meets exit-remediation 1002-1004), `exit_code_for_agent_error` (→ `ExitCode::AuthFailure`/3), `handle_agent_result` (emit the typed RPC/NDJSON auth event), and `finish_pending` (arm the same-provider credential retry). This is the spec-required JSON/RPC/text remediation behavior.
- **Test plan**: Integration tests (MockProvider-driven) asserting exit code 3, the NDJSON remediation event, the RPC typed event, and the interactive retry arming when the stream returns `AccountIdMissing`. Shared helpers go in `tests/common/mod.rs` if needed (per workspace convention).

#### Fix 3.2: Harness thinking-change `thinking_level_map` check (CL5 cheap part, Minor)

- **Audit source**: codex 3.1
- **Cluster**: CL5
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L1037-1078 (`prepare_thinking_change`), L1189-1199 (`validate_current_thinking_for_model`)
- **Change**: Add a `thinking_level_map.resolve` check so `set_thinking_level` / model-switch / resume reject an unmappable level (e.g. `xhigh`/`max` for a model that will reject it) up-front, instead of persisting a setting that deterministically fails the next prompt's preflight.
- **Test plan**: New test: setting an unmappable level against a reasoning-default model is rejected at the harness seam.

#### Fix 3.3: OAuth checked duration arithmetic (CL3, Minor)

- **Audit source**: codex 2.3
- **Cluster**: CL3
- **Decision**: D4
- **Verification status**: Confirmed (+ cross-check agree)
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L427-433, L534-536, L1028-1035, L1179-1240, L1717-1720, L1763-1767
- **Change**: Replace unchecked `OffsetDateTime::now_utc() + time::Duration::seconds(secs)` at L433/L536 with `checked_add(...).ok_or_else(|| ProviderError::Config("token expires_in out of range".into()))`. Cap the device poll interval (Codex `codex_device_interval` L1028-1035; Copilot L1717-1720) at a sane RFC 8628 bound (e.g. 600s), rejecting larger values with `ProviderError::Config`. Replace `interval += Duration::from_secs(5)` slow_down arms (L1240, L1767) with `interval.checked_add(...).ok_or_else(|| ProviderError::Config("device poll interval overflow".into()))?`. The safe `checked_add` pattern is already in-file at `FlowBudget::new` (L65-67).
- **Test plan**: Adversarial tests in `tests/oauth_auth.rs` (or inline): `expires_in: i64::MAX`, `i64::MIN`, `0`/negative; device `interval: u64::MAX` and a large positive value with repeated `slow_down` to exercise the `AddAssign` overflow path — each asserting a typed error, not a panic.

#### Fix 3.4: PKCE callback parse-before-200 (CL8, Minor)

- **Audit source**: codex 4.1
- **Cluster**: CL8
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L439-484 (`accept_one_callback`)
- **Change**: Read bytes → parse UTF-8 → `normalize_pkce_input` → validate CSRF state **before** writing the HTTP response. Write `200` + "Login complete" only on valid state/code extraction; write a fixed secret-free `400` body (e.g. "Login failed, return to the terminal.") on parse/validation failure. Preserve the existing write-completion-before-channel-send ordering rationale.
- **Test plan**: New tests: malformed request body and state-mismatched callback each receive a 400 and fail credential acquisition; only a valid callback receives 200 and succeeds.

#### Fix 3.5: Manual input concurrent drain + line cap (CL9, Minor)

- **Audit source**: codex 4.2
- **Cluster**: CL9
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs` ~L1893-1915
- **Change**: Drain the child's piped stdout concurrently with awaiting the child (`tokio::join!` of `child.wait()` and a stdout drain), and enforce a strict maximum line length (e.g. 8 KiB) that errors on oversized input before the pipe fills. Removes the child-write/parent-exit deadlock for oversized manual input.
- **Test plan**: New test: a manual input exceeding the pipe capacity is rejected rather than deadlocking until the OAuth deadline.

#### Fix 3.6: Zeroizing envelope encode buffer (CL10, Minor)

- **Audit source**: glm5.2 3.1
- **Cluster**: CL10
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/credential_store.rs` ~L479-507 (`encode_credential`)
- **Change**: Wrap the intermediate secret-bearing `EnvelopeFields` strings and the final serialized buffer in `zeroize::Zeroizing<String>` so the serialized secret is wiped after `backend.set_password`. Add `zeroize` via `[workspace.dependencies]` if not already present (reuse the one `secrecy` re-exports if suitable).
- **Test plan**: Existing redaction/scan tests must still pass; add an assertion that the encode path does not leave a recoverable plaintext copy (defense-in-depth; best-effort where feasible).

### Layer 4: Documentation

**Verification**: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` (final gate).

#### Fix 4.1: CHANGELOG breaking-change entry for the new `AgentError` variant

- **Audit source**: codex 3.2 (follow-on of D2)
- **Change**: Under `## [Unreleased]` → `### Breaking Changes` in `CHANGELOG.md`, record that `AgentError` gained an `AccountIdMissing { provider_id }` variant (exhaustive enum; 0.x breaking change). Optional brief `### Fixed` note cross-referencing the typed-auth remediation.
- **Note**: No `opi-spec.md` edit is required — the code is being brought into compliance with the existing spec text (`opi-spec.md:1702-1705`, `1721`; exit-remediation `946-948`, `1002-1004`, `1014`). Avoiding a spec edit avoids the phase4/phase6 snapshot spec-hash re-sync cascade.

---

## Final verification

    CARGO_INCREMENTAL=0 cargo test --workspace --all-targets
    CARGO_INCREMENTAL=0 cargo test --workspace --doc
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

Workspace smoke is disk-heavy on this host (~106 GB `target/`); run crate-scoped gates per layer first, then the workspace smoke once with `CARGO_INCREMENTAL=0` and reclaim space with `cargo clean` if needed. New tests must stay network-free (fixture / `wiremock` / `tempfile` / `MockProvider`); any `#[cfg(unix)]` test will not execute locally on Windows and must be audited before pushing.

---

## Scope exclusions

| Finding | Cluster | Status | Reason |
|---------|---------|--------|--------|
| C-2.1 deferred? | CL1 | — | Not deferred; **in scope** (Blocker). Listed here only to confirm it is NOT excluded. |
| C-3.1 dispatch-seam (public `Provider::stream` enforcement) | CL5b | Deferred (carry-over) | Production opi binary is fully masked by the `agent_loop` preflight; residual is a published-library API gap (consumer bypassing the documented `validate_request_capabilities`). The cheap correctness parts (Fix 1.3 + Fix 3.2) are in scope; moving the validator into the public dispatch seam is a low-ROI public-API refactor deferred to a future pass. |
| C-2.4 mixed-model cost reprice | CL4 | Deferred (carry-over) | Real correctness defect but scope is best-effort cost **display** only (CLAUDE.md: "best-effort cost tracking"); drives no `agent_loop` decision, tool, request, or billing. Fix is a Medium redesign (per-turn priced-usage ledger + resume/fork replay + test rewrite). Documented for a future pass. |
| C-3.3 cancellation not observed pre-code/pre-header | CL7 | Deferred (carry-over) | Provider-stream cancellation is production-masked (`agent_loop` drops the stream on cancel → receiver-drop aborts the task). Residual is PKCE Ctrl-C responsiveness during login (Minor UX) and library-consumer reach. Cross-cutting fix (one cancellation future through every pre-code stage + cancel-select around auth+send+body) deferred. |
| G-3.2 keyring lease guard debug-only | CL11 | Info / No action | Zero production reach — the `self.leased` flag already precludes the underflow the `debug_assert!` guards. Gold-plating to harden further. |
| G-5.4 Codex subset diagnostics collapse | CL14 | Info / No action | Display-only message polish; error type and rejection behavior are already correct. |

### Out-of-skill carry-over (not a Phase 14 code defect)

The live `.opi-impl-state.json` `spec_files_sha256` for `docs/opi-spec.md` drifts from the checked-in file (the phase4/phase6 snapshot ledgers were re-synced to a prior opi-spec hash during earlier remediation, but the live ledger was not). This is a process item for `opi-implement` to re-sync; `opi-remediate` does not touch the implementation ledger. This plan makes no `opi-spec.md` edit, so it does not perturb the drift.

---

## Relationship to prior remediation passes

This is the third phase-14 remediation pass. The first (`9263114`) addressed the original audit's C1–C21 but introduced the C8 silent-omit regression (and a self-serving spec edit). The second (`b27905a`) reverted the C8 regression but did not reach the decoder, panic-safety, typed-auth-propagation, or several Minor defects re-surfaced here. Per the established lesson, the PASS auditor (glm5.2) again missed the stream-contract and panic-safety classes; the codex findings were factually correct on all 14, with severity re-rated here by production reachability rather than taken at face value.
