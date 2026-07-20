# Phase 14 Remediation Plan

**Date**: 2026-07-20
**Audit sources**: `docs/snapshots/phase14/audit.codex.md`, `audit.glm5.2.md`, `audit.opus4.6.md`
**Audited baseline**: `9263114` (post-exit-remediation HEAD; same baseline all three auditors reviewed)
**Commit range**: `d9f21a9..8364e74` (phase work), remediated by `9263114`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`, `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`
**Method**: 3-way cross-reference (codex/glm5.2/opus4.6) → 7 adversarial read-only verification agents → Phase D decisions.

## Headline

All three auditors reviewed the same baseline. They split sharply: **codex FAIL (10 Major / 4 Minor)**, **glm5.2 PASS (6 Minor)**, **opus4.6 PASS (2 Minor)**. Phase C verified **all 21 distinct findings as real defects — 0 refuted, 0 cannot-confirm.** Codex was correct on the facts; the disagreement with GLM/Opus was severity and reachability (GLM/Opus missed all 10 of codex's majors). GLM's explicit refutations targeted *construction-time* laziness and *failed-turn persistence*, which are compatible with — not contradictions of — codex's *stream-time* and *next-turn-accounting* claims.

Final tally after verification: **7 MAJOR** (C4, C5, C6, C7, C8, C9, C10) + **11 Minor** (C1, C2, C3 downgraded from Major; C11–C21). C3 is latent (production never sets `Request::timeout`).

User decisions (Phase D): **C8 → restore rejection**; **scope → all confirmed findings**.

---

## Audit cross-reference summary

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C1 | stream `tokio::spawn` does work before first poll | codex 2.1 | Unique (1/3) | Minor (was Major) | Confirmed (prod-masked) |
| C2 | cancellation yields clean EOF, not `Cancelled` | codex 2.2 | Unique | Minor (was Major) | Confirmed (prod-masked) |
| C3 | post-header body errors lose `Timeout` type | codex 2.3 | Unique | Minor/Info (latent) | Partially confirmed |
| C4 | Codex device poller maps 403/404 → Pending before parsing terminal OAuth code | codex 2.4 | Unique | Major | Confirmed |
| C5 | failed-turn + new prompt → `turns` counter diverges live vs resumed | codex 2.5 | Unique | Major (soft) | Confirmed (turns only; not cost) |
| C6 | raw upstream SSE error text leaks into session JSONL + NDJSON | codex 3.1 | Unique | Major (security) | Confirmed |
| C7 | `CacheRetention::Disabled` still emits Codex affinity headers | codex 3.2 | Unique | Major | Confirmed |
| C8 | Chat/Responses silently omit unsupported thinking (regression from `9263114`) | codex 4.1 | Unique | Major | Confirmed; norm conflict → restore rejection |
| C9 | dedicated Codex route bypasses `UnknownModel` pre-I/O validation | codex 4.2 | Unique | Major (bounded) | Confirmed |
| C10 | acceptance artifacts: 3 zero-test filters, 5 dead paths, phrase-only guard | codex 5.1 | Unique | Major (process) | Confirmed (no missing coverage) |
| C11 | `session-id` skips `HeaderValue::from_str` (Responses + Codex) | glm5.2 2.1, opus4.6 3.1 | Majority (2/3) | Minor | Confirmed |
| C12 | `opi-spec.zh.md` corrupt preamble `e'x#` | codex 5.2 | Unique | Minor | Confirmed |
| C13 | specs still describe credentials as build-time validation | codex 5.3 | Unique | Minor | Confirmed |
| C14 | `ApiMappedProvider` cannot enforce shared-resolver invariant | codex 6.1 | Unique | Minor | Confirmed |
| C15 | dynamic refresh skips catalog ID/duplicate validation | codex 6.2 | Unique | Minor | Confirmed |
| C16 | Copilot managed-header override-rejection test covers 4/7 names | glm5.2 4.1 | Unique | Minor | Confirmed |
| C17 | `Copilot-Vision-Request` not asserted on `/v1/messages` with image | glm5.2 4.2 | Unique | Minor | Confirmed |
| C18 | 401/403 revocation tests don't assert exactly one HTTP request | glm5.2 4.3 | Unique | Minor | Confirmed |
| C19 | bare `copilot`/`codex` → generic error, no rename hint | glm5.2 5.1 | Unique | Minor | Confirmed |
| C20 | doctor `provider_proxy_url` ignores `providers.custom` | glm5.2 7.1 | Unique | Minor | Confirmed |
| C21 | reserved-header rejection test covers 2/5 names | opus4.6 4.1 | Unique | Minor | Confirmed |

## Decision record

| ID | Cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C4 | Parse recognized terminal OAuth codes before status-based Pending fallback; treat 403/404 as pending only when no recognized terminal code | Terminal `access_denied`/`expired_token` on 403/404 must not hang ~15 min then misreport as `Timeout` | auto |
| D2 | C7 | Omit both `session-id` and `x-client-request-id` when `CacheRetention::Disabled`; invert the test to assert absence | Spec (design `:490-492,521,528-530`) requires suppression; test codified the wrong behavior | auto |
| D3 | C8 | **Restore rejection**: drop the Chat/Responses exemption in `provider.rs:276-284`, delete the silent-omit paragraph added by `9263114` at `opi-spec.md:1700-1704` (+ `.zh.md`), convert the two `..._reaches_http_without_reasoning` tests to expect `UnsupportedCapability` | The exemption was introduced by `9263114` and contradicts the registered 14.15 DoD (`exit-remediation-design.md:1078`, untouched); pre-`9263114` code rejected unconditionally. Silent success on unsupported thinking is an undetectable user-facing regression | **user** (restore rejection) |
| D4 | C9 | Add `ProviderError::UnknownModel` stream-entry guard + restrict prefix-strip to the `openai-codex:` prefix in the dedicated Codex provider | Matches the `ApiMappedProvider` boundary; closes the pre-I/O unknown/cross-provider hole | auto |
| D5 | C5 | Expose `Agent::rewind_to(index)`; harness rewinds the in-memory message list to the persisted boundary (`turn_offset`) at new-prompt entry (not on `retry_last_turn`) so abandoned failed-turn User messages don't leak into the next slice | Surgical; preserves the retry path; closes the live-vs-resume `turns` divergence and the analogous cancel leak | auto |
| D6 | C6 | Provider-side neutral error literals (root cause, matches the Codex precedent) for anthropic/chat/responses/shared + malformed-SSE; apply `redact_text` to `error_message` at the `redact_assistant_message` chokepoint (NDJSON defense-in-depth); add sentinel-secret canary tests for the three missing families; update the chat fixture that currently asserts raw-text preservation | Root cause closes both JSONL and NDJSON leaks; chokepoint is one-line insurance against future provider regressions; canaries lock the invariant | auto |
| D7 | C10 | Replace phrase-only check with an executable manifest validator (enumerate each cargo-test filter, assert ≥1 match; validate each declared test path exists); fix the plan doc's broken historical command names; correct the 5 dead `behavioral_tests` paths in the archived snapshot | Evidence-integrity: acceptance rows must actually execute tests | auto |
| D8 | C1, C2, C3 | C2: return `Err(ProviderError::Cancelled)` on the cancel branch (4 providers) + update the EOF test. C3: classify body errors with the same timeout/connect/network mapping as request errors. C1: wrap each spawned task body in `select! { _ = tx_clone.closed() => return Ok(()), result = <body> => result }` for rx-drop cancellation + unpolled-drop tests | Real stream-contract defects; C2/C3 localized; C1 hardest (Minor, prod-masked) — rx-drop via `Sender::closed()` is the localized approach | auto |
| D9 | C11 | Validate `session_id` with `HeaderValue::from_str` before applying the header on Responses + Codex; return `ProviderError::RequestFailed` on failure (mirror Chat's `validate_pair`) | Aligns the two Responses-family wires with the T3 3a mandate; converts a retryable `Network` edge into clean non-retryable `RequestFailed` | auto |
| D10 | C14 | Strengthen the `ApiMappedProvider` shared-resolver doc to a MUST with a correct-usage example; **defer** the enforceable API change (constructor takes route specs + shared resolver) | Enforcement requires a breaking opi-ai public-API change disproportionate to a Minor, production-correct invariant; revisit if an embedder misuse surfaces | auto (partial; enforcement deferred) |
| D11 | C15 | Apply the same canonical-ID + uniqueness validation to the refresh candidate before `replace_all_dynamic_catalogs`; preserve the previous catalog on validation failure; add malformed/duplicate refresh fixtures | Asymmetric validation gap; additive | auto |
| D12 | C12, C13, C16, C17, C18, C19, C20, C21 | Minor doc/test fixes as described per fix item | Each is a one- to four-line, single-direction fix | auto |

---

## Remediation layers

Workspace dependency graph: opi-ai, opi-tui (Layer 1) → opi-agent (Layer 2) → opi-coding-agent (Layer 3) → Documentation (Layer 4).

### Layer 1: opi-ai (substrate)

**Verification**:

```sh
cargo fmt --all
cargo clippy -p opi-ai --all-targets -- -D warnings
cargo test -p opi-ai --all-targets
```

#### Fix 1.1: C2 — cancellation returns `ProviderError::Cancelled`

- **Audit source**: codex 2.2 (Cluster C2, Decision D8)
- **Verification status**: Confirmed (prod-masked; minor)
- **File(s)**: `crates/opi-ai/src/anthropic.rs:973-977`; `openai_chat.rs:1198-1202`; `openai_responses.rs:417-419`; `openai_codex_responses.rs:200-202`
- **Change**: In each provider's `stream_http` cancel branch, `return Err(ProviderError::Cancelled);` instead of `return Ok(());`. The spawned wrapper already forwards `Err`, so consumers observe the typed error.
- **Test plan**: modify `stream_cancellation_aborts_before_completion` (`tests/openai_chat_fixtures.rs:1631-1679`) to expect a `ProviderError::Cancelled` event rather than `next.is_none()`; add cancel-returns-typed-error tests for anthropic/responses/codex.

#### Fix 1.2: C3 — classify body-read errors with timeout/connect/network mapping

- **Audit source**: codex 2.3 (Cluster C3, Decision D8)
- **Verification status**: Partially confirmed (latent; no production caller sets `Request::timeout`)
- **File(s)**: `crates/opi-ai/src/anthropic.rs:986`; `openai_chat.rs:1211`; `openai_responses.rs:425`; `openai_codex_responses.rs:208`; classifier at `provider.rs:363-367`
- **Change**: Route body-read errors through the same classifier used for initial request errors (timeout/is-connect/is-request → `Timeout`/`Network`/`RequestFailed`), only falling back to `StreamError` when no classification matches. Keep `StreamError` for genuinely unclassifiable body errors.
- **Test plan**: add a "headers then stall" fixture per family (send headers, then never end the body) with a per-request `timeout` set, asserting the surfaced error is `ProviderError::Timeout` (retryable), not `StreamError`.

#### Fix 1.3: C1 — rx-drop cancellation of spawned stream tasks

- **Audit source**: codex 2.1 (Cluster C1, Decision D8)
- **Verification status**: Confirmed (prod-masked; minor; **highest implementation risk**)
- **File(s)**: `crates/opi-ai/src/anthropic.rs:1276-1318`; `openai_chat.rs:1495-1537`; `openai_responses.rs:541-585`; `openai_codex_responses.rs:278-318`
- **Change**: Inside each spawned task, clone the `mpsc::Sender` and `tokio::select! { biased; _ = tx_clone.closed() => return Ok(()), result = <existing task body> => result }`. `Sender::closed()` resolves when all receivers drop, so dropping an unpolled/abandoned stream aborts auth resolution and HTTP. This is additive to the existing `tx.send().is_err()` checks.
- **Test plan**: add an unpolled-drop test per family: construct the stream, drop it without polling, assert zero `AuthResolver` calls and zero HTTP requests (wiremock `received_requests().is_empty()`). **Flag**: this is the one Minor fix with non-trivial risk in a security-sensitive area; if it destabilizes the stream architecture during execution, escalate before forcing it through.

#### Fix 1.4: C6(a) — provider-side neutral error literals + canaries

- **Audit source**: codex 3.1 (Cluster C6, Decision D6)
- **Verification status**: Confirmed (Major; security)
- **File(s)**: `anthropic.rs:591-598`; `openai_chat.rs:690-697, 1121-1124, 1225-1228`; `openai_responses.rs:340-342, 436-439`; `openai_responses_shared.rs:512-519`; precedent `openai_codex_responses.rs:213-218, 225-229, 334-349`
- **Change**: Replace raw upstream `error.message` in the `AssistantStreamEvent::Error` path with fixed provider-neutral literals (e.g. `UPSTREAM_STREAM_ERROR`), mirroring the Codex path. Replace the malformed-SSE `format!("malformed SSE data: {error} (data: {data:.80})")` with a fixed literal (drop the 80-char raw fragment). Optionally log the raw detail via `tracing::debug!` only (never exposed).
- **Test plan**: modify `tests/openai_chat_fixtures.rs:611-648` (currently asserts `"Rate limit exceeded"` preserved verbatim — the opposite of a canary) to expect the neutral literal. Add sentinel-secret canary tests for anthropic/chat/responses modeled on the Codex canary at `oauth_auth.rs:3215-3257` (inject a secret-bearing SSE error frame, assert it does not appear in the stream error / public event / persisted message).

#### Fix 1.5: C7 — omit Codex affinity headers when retention Disabled

- **Audit source**: codex 3.2 (Cluster C7, Decision D2)
- **Verification status**: Confirmed (Major)
- **File(s)**: `crates/opi-ai/src/openai_codex_responses.rs:171-172, 269-275`
- **Change**: Pass `session_id` and `request_id` as `Option<String>` into `stream_http`; apply `.header("session-id", …)` / `.header("x-client-request-id", …)` only when retention is not `Disabled` (and the value is present). Stop synthesizing the UUID fallback in the Disabled branch.
- **Test plan**: invert `dedicated_codex_disabled_affinity_omits_user_session_everywhere` (`tests/openai_codex_responses.rs:271-316`) to assert `session-id` and `x-client-request-id` are **absent** from the captured request when retention is `Disabled`; add the positive (present when `Short`/`Long`) counterpart if not already covered.

#### Fix 1.6: C8(a) — drop the Chat/Responses thinking exemption (restore rejection)

- **Audit source**: codex 4.1 (Cluster C8, Decision D3 — **user**)
- **Verification status**: Confirmed (Major; reverts a regression introduced by `9263114`)
- **File(s)**: `crates/opi-ai/src/provider.rs:276-284`
- **Change**: Remove the `if !matches!(model.wire_api, OpenAiCompletions | OpenAiResponses)` guard so the `thinking_level_map.resolve(...)? → UnsupportedCapability` rejection applies to **all** wires unconditionally (the pre-`9263114` behavior). The request builders' `let Ok(Some(effort))` arms become unreachable for unsupported levels (request construction is never reached).
- **Test plan**: convert `chat_unsupported_thinking_level_reaches_http_without_reasoning` and `responses_unsupported_thinking_level_reaches_http_without_reasoning` (`tests/model_wire_metadata.rs:213-286`) to expect `ProviderError::UnsupportedCapability` and `server.received_requests().is_empty()` (same shape as the existing `strict_wire_unsupported_thinking_level_is_rejected_before_http`). Rename them away from `_reaches_http_without_reasoning`. Grep for any other test asserting silent Chat/Responses thinking omission and convert it.

#### Fix 1.7: C9(a) — `UnknownModel` guard + prefix restriction for the dedicated Codex route

- **Audit source**: codex 4.2 (Cluster C9, Decision D4)
- **Verification status**: Confirmed (Major, bounded)
- **File(s)**: `crates/opi-ai/src/openai_codex_responses.rs:256-265` (stream entry); contrast `api_mapped.rs:154-193`
- **Change**: At the start of `OpenAiCodexResponsesProvider::stream`, after deriving `model_id`, return `Err(ProviderError::UnknownModel { provider_id, model_id })` when `model_id` is not present in `self.models`. Restrict the prefix-strip from `split_once(':')` (strips any prefix) to stripping only the `openai-codex:` prefix; a non-matching prefix yields `UnknownModel`.
- **Test plan**: add zero-resolver/zero-HTTP tests: unknown bare id (`gpt-5-typo`), cross-provider prefix (`anthropic:foo` → reject, do not strip to `foo`), and a valid id (positive control). Assert `received_requests().is_empty()` and zero auth-resolver calls on the rejection paths.

#### Fix 1.8: C11 — `HeaderValue::from_str` validation for `session-id`

- **Audit source**: glm5.2 2.1 + opus4.6 3.1 (Cluster C11, Decision D9)
- **Verification status**: Confirmed (Major-consensus Minor)
- **File(s)**: `openai_responses.rs:385-388`; `openai_codex_responses.rs:171`; correct path `openai_chat.rs` → `provider_headers.rs:62-99`
- **Change**: Before applying the `session-id` header, validate the value with `HeaderValue::from_str`; on failure return `ProviderError::RequestFailed` (non-retryable). (Reqwest 0.12.28 stores the conversion failure into the request `Result` and surfaces it at `.send().await` as a retryable `Network` — this fix converts that to the spec-mandated non-retryable `RequestFailed`.) Optionally route through `ProviderHeaders::merge_request` to reuse `validate_pair`.
- **Test plan**: add a test injecting `Request { session_id: Some("bad\nvalue".into()), .. }` on both wires, asserting `ProviderError::RequestFailed` and `!is_retryable()`.

#### Fix 1.9: C15 — validate dynamic-refresh catalog candidates

- **Audit source**: codex 6.2 (Cluster C15, Decision D11)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `provider_collection.rs:442-468`; `registry.rs:339-353`; existing validation `registry.rs:184-214`
- **Change**: Before `replace_all_dynamic_catalogs`, validate each refreshed `ModelInfo` with the same checks as `register_model` (non-empty canonical id, no duplicates within the candidate, `model.validate()`). On validation failure, preserve the previous catalog and return an error.
- **Test plan**: add fixtures for a malformed-id refresh candidate and a duplicate-id refresh candidate; assert the previous catalog is retained and the error is typed.

#### Fix 1.10: C18(a) — assert single HTTP request on Codex revocation

- **Audit source**: glm5.2 4.3 (Cluster C18, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `crates/opi-oding-agent` N/A — this is `crates/opi-ai/tests/openai_codex_responses.rs:499-531`
- **Change**: After the existing `drain()` in `dedicated_codex_401_and_403_are_revoked_and_redacted`, assert `server.received_requests().len() == 1` to lock the no-follow-up-request invariant.
- **Test plan**: same test (modification).

#### Fix 1.11: C21 — reserved-header rejection test covers all 5 names

- **Audit source**: opus4.6 4.1 (Cluster C21, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: const `RESERVED` at `provider.rs:168-174`; test `request_enrichment.rs:107-125`
- **Change**: Extend `extra_headers_reject_auth_header_names` to iterate all 5 reserved names (`authorization`, `x-api-key`, `api-key`, `anthropic-version`, `content-type`) individually (currently only `authorization` + `x-api-key`).
- **Test plan**: same test (modification).

### Layer 2: opi-agent

**Verification**:

```sh
cargo fmt --all
cargo clippy -p opi-agent --all-targets -- -D warnings
cargo test -p opi-agent --all-targets
```

#### Fix 2.1: C6(b) — redact `error_message` at the public-event chokepoint

- **Audit source**: codex 3.1 (Cluster C6, Decision D6)
- **Verification status**: Confirmed (Major; security defense-in-depth)
- **File(s)**: `crates/opi-agent/src/event.rs:234-252`
- **Change**: In `redact_assistant_message`, apply `redact_text(message.error_message.as_deref(), RedactionMode::Summary)` (or map to `None`) instead of cloning `error_message` unchanged. This closes the NDJSON `--json` leak independently of provider discipline.
- **Test plan**: add a unit test feeding an `AssistantMessage` whose `error_message` carries a sentinel secret through `redacted_for_public()`, asserting the secret is absent in the serialized event. (Pairs with the Layer 1 canaries.)

#### Fix 2.2: C5(a) — expose `Agent::rewind_to` for failed-turn rollback

- **Audit source**: codex 2.5 (Cluster C5, Decision D5)
- **Verification status**: Confirmed (Major, soft)
- **File(s)**: `crates/opi-agent/src/agent.rs` (near `prompt`/`prompt_with_content`/`continue_` at `:166-215`)
- **Change**: Add `pub fn rewind_to(&mut self, index: usize)` (or equivalent) that truncates `self.messages` to `index`, used by the harness at new-prompt entry. Does **not** change `retry_last_turn` semantics.
- **Test plan**: unit test that a failed-turn User push followed by `rewind_to(pre-turn-index)` drops exactly the unpersisted message.

### Layer 3: opi-coding-agent

**Verification**:

```sh
cargo fmt --all
cargo clippy -p opi-coding-agent --all-targets -- -D warnings
cargo test -p opi-coding-agent --all-targets
```

#### Fix 3.1: C4 — parse terminal OAuth codes before status-based fallback

- **Audit source**: codex 2.4 (Cluster C4, Decision D1)
- **Verification status**: Confirmed (Major)
- **File(s)**: `crates/opi-coding-agent/src/oauth.rs:1080-1094`
- **Change**: Restructure the Codex device poller to call `codex_device_error_code(&body)` first and classify terminal codes (`access_denied`, `expired_token`, …) before any status check; fall back to status-based `Pending` only when the body has no recognized terminal code.
- **Test plan**: add 403 and 404 variants to the denial/expiry tests (currently HTTP-400-only at `tests/oauth_auth.rs:2787-2829`): mount `403 {"error":"access_denied"}` and `404 {"error":"expired_token"}`, assert `CredentialRevoked`-typed outcome promptly (not the ~15 min `Timeout`).

#### Fix 3.2: C5(b) — harness rewinds to persisted boundary on new prompt

- **Audit source**: codex 2.5 (Cluster C5, Decision D5)
- **Verification status**: Confirmed (Major, soft)
- **File(s)**: `crates/opi-coding-agent/src/harness.rs:1468-1481` (and the new-prompt entry path; coordinate with `interactive.rs:1083-1091`)
- **Change**: At the start of a new prompt (not `retry_last_turn`), call `self.agent.rewind_to(self.turn_offset)` before pushing the new User message, so an abandoned failed-turn (`CredentialNeeded`) or cancelled-turn User message does not get absorbed into the next successful persistence slice. `turn_offset` is the persisted boundary.
- **Test plan**: add a production session-runtime test: fail auth (CredentialNeeded) → submit a *different* prompt → persist → resume → assert the resumed `CumulativeUsage.turns` and `SessionSummary.turns` equal the live process's value (1, not 2). Also cover the cancel-then-new-prompt variant.

#### Fix 3.3: C6(c) — redact `error_message` at the JSONL persistence chokepoint (defense-in-depth)

- **Audit source**: codex 3.1 (Cluster C6, Decision D6)
- **Verification status**: Confirmed (Major; security defense-in-depth; **already closed by Fix 1.4 root cause**)
- **File(s)**: `crates/opi-coding-agent/src/session_coordinator.rs:653-663` (`message_for_session`)
- **Change**: Optional. Apply `redact_text` to `error_message` before persisting the `AssistantMessage` to JSONL. With Fix 1.4 the raw text never reaches this path; this is belt-and-suspenders for future regressions. Include if low-risk; otherwise note as covered.
- **Test plan**: extend the Layer 1 canaries to assert the persisted JSONL `SessionEntry` omits the sentinel secret.

#### Fix 3.4: C9(b) — factory surfaces Codex model validation (if needed beyond 1.7)

- **Audit source**: codex 4.2 (Cluster C9, Decision D4)
- **Verification status**: Confirmed (Major, bounded)
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs:1375-1439`
- **Change**: Likely no change required once Fix 1.7 guards at stream entry. If initial harness construction should reject an unknown `config.defaults.model` for `openai-codex` before any turn, add a catalog-membership check in `build_codex_oauth`. Decide during execution based on whether the stream-entry guard fully satisfies the pre-I/O invariant for the startup path.
- **Test plan**: if changed, add a startup-rejection test for an unknown codex model id.

#### Fix 3.5: C10 — executable acceptance-manifest validator + correct artifacts

- **Audit source**: codex 5.1 (Cluster C10, Decision D7)
- **Verification status**: Confirmed (Major, process)
- **File(s)**: `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs:191-216`; `docs/superpowers/plans/2026-07-17-phase14-pi-0806-alignment.md:1331-1399`; `docs/snapshots/phase14/opi-impl-state.json` (behavioral_tests paths)
- **Change**: (a) Replace the phrase-only `"58-row acceptance manifest"` assertion with a validator that parses the manifest, enumerates each `cargo test … <filter>` row, and asserts each filter matches ≥1 test fn (via source grep / `--list`), and that each declared test path exists. (b) Fix the plan doc: retire the broken historical command names — `unsupported_thinking_level_is_rejected_before_request_build` → `strict_wire_unsupported_thinking_level_is_rejected_before_http`; `oauth_login_restores_terminal_after_flow_failure` → `dispatcher_restores_terminal_once_on_every_concrete_exit` in `--test interactive_auth` (not `--lib`); `interactive_explicit_login_retries_pending_turn_once` → `outer_tui_same_provider_login_retries_pending_turn_once` in `--test interactive_tui_auth` (not `interactive_auth`). (c) Correct the 5 dead `behavioral_tests` paths in the archived snapshot (`doctor.rs`→`doctor_cli.rs`, `request_enrichment_wiring.rs`→`crates/opi-ai/tests/request_enrichment.rs`, `anthropic.rs`→`anthropic_fixtures.rs`, `model_capabilities_wiring.rs`→`crates/opi-ai/tests/model_capabilities_migration.rs`, `usage_cost_wiring.rs`→`crates/opi-ai/tests/usage_cost.rs`).
- **Test plan**: the validator test itself; verify it fails when given a zero-match filter (negative case).

#### Fix 3.6: C16 — Copilot managed-header rejection test covers all 7 names

- **Audit source**: glm5.2 4.1 (Cluster C16, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `tests/github_copilot_provider.rs:399-416`; const `GITHUB_COPILOT_MANAGED_HEADERS` (`crates/opi-ai/src/provider.rs:116-124`)
- **Change**: Extend the iteration list to include `Editor-Version`, `Editor-Plugin-Version`, `Copilot-Integration-Id` (currently only `User-Agent`, `X-Initiator`, `Openai-Intent`, `Copilot-Vision-Request`).
- **Test plan**: same test (modification).

#### Fix 3.7: C17 — assert `Copilot-Vision-Request: true` on `/v1/messages` with image

- **Audit source**: glm5.2 4.2 (Cluster C17, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `tests/github_copilot_provider.rs:472-537`
- **Change**: Add a case sending an image-bearing `UserMessage` to `claude-sonnet-4.5` (`/v1/messages`) and assert `Copilot-Vision-Request: true` on the captured request (currently the Anthropic route is exercised text-only and asserts header *absence*).
- **Test plan**: same test (modification).

#### Fix 3.8: C18(b) — assert single HTTP request on Copilot revocation

- **Audit source**: glm5.2 4.3 (Cluster C18, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `tests/github_copilot_provider.rs:582-607`
- **Change**: After `drain()`, assert `server.received_requests().len() == 1`.
- **Test plan**: same test (modification).

#### Fix 3.9: C19 — canonical remediation hint for deprecated `copilot`/`codex` ids

- **Audit source**: glm5.2 5.1 (Cluster C19, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `crates/opi-coding-agent/src/provider_factory.rs:1809-1828`
- **Change**: In `build_runtime_provider`, detect bare `"copilot"` / `"codex"` and return `ProviderBuildError::Config("'copilot' has been renamed; use provider id 'github-copilot' (login: /login github-copilot)")` (and analogously for `codex` → `openai-codex`) ahead of the generic `unknown provider` arm.
- **Test plan**: add a test asserting the rename hint appears in the error for both deprecated ids (existing `tests/provider_identity.rs:40-47` only checks registry absence).

#### Fix 3.10: C20 — doctor `provider_proxy_url` consults `providers.custom`

- **Audit source**: glm5.2 7.1 (Cluster C20, Decision D12)
- **Verification status**: Confirmed (Minor)
- **File(s)**: `crates/opi-coding-agent/src/doctor.rs:699-720`
- **Change**: In the `other =>` arm of `provider_proxy_url`, add `config.providers.custom.get(other).and_then(|p| p.proxy.as_ref().map(|p| p.url.as_str()))` ahead of the `openai_compatible` fallback.
- **Test plan**: add a doctor config-scope test with a `[providers.custom.acme]` proxy and assert `CODE_DOCTOR_CONFIG_PROXY` reports it.

### Layer 4: Documentation

**Verification**:

```sh
cargo test -p opi-coding-agent --test phase4_ledger
cargo test -p opi-coding-agent --test phase6_ledger
cargo test -p opi-coding-agent --test phase14_provider_auth_docs
cargo fmt --all
```

Doc edits to `docs/opi-spec.md` change its CRLF-normalized SHA-256, which `phase4_ledger` and `phase6_ledger` pin against `spec_files_sha256["docs/opi-spec.md"]` in the **snapshot** ledgers (`docs/snapshots/phase4/opi-impl-state.json`, `docs/snapshots/phase6/opi-impl-state.json`). The guards hash only the EN `opi-spec.md`, not `.zh.md`.

#### Fix 4.1: C8(b) + C13 — `opi-spec.md` thinking-policy + credential-lifecycle text (EN)

- **Audit source**: codex 4.1 (C8) + codex 5.3 (C13); Decisions D3, D12
- **Verification status**: Confirmed
- **File(s)**: `docs/opi-spec.md:1700-1704` (C8) and `:1581-1582` (C13)
- **Change**:
  - C8: replace the silent-omit paragraph at `:1700-1704` (added by `9263114`) with rejection language matching the 14.15 DoD, e.g. *"Unsupported thinking levels are rejected before request construction on every wire: if `request.thinking.enabled` and the selected `ModelInfo::thinking_level_map` cannot resolve `request.thinking.level`, the provider returns `ProviderError::UnsupportedCapability` without network I/O. Static `reasoning_effort` fields are legacy compatibility/profile metadata and do not override that selection."*
  - C13: rewrite `:1581-1582` to describe construction as structural/provider-profile validation and credential resolution as a per-stream operation (login/logout/refresh take effect without rebuilding the provider).
- **Test plan**: re-run `phase4_ledger` / `phase6_ledger` after the snapshot re-sync (Fix 4.4).

#### Fix 4.2: C8(b) + C12 + C13 — localized counterparts (`opi-spec.zh.md`)

- **Audit source**: codex 4.1, 5.2, 5.3; Decisions D3, D12
- **Verification status**: Confirmed
- **File(s)**: `docs/opi-spec.zh.md:1` (C12 preamble), the thinking-policy paragraph (C8 counterpart), `:1343-1344` (C13 counterpart)
- **Change**: remove the corrupt `e'x# ` prefix at line 1; update the thinking-policy paragraph to match Fix 4.1 (rejection, not silent-omit); rewrite the credential-lifecycle sentence to match Fix 4.1.
- **Test plan**: optional lightweight H1-structure assertion for both spec variants (prevents recurrence of C12). No hash guard for `.zh.md`.

#### Fix 4.3: C14 — strengthen `ApiMappedProvider` shared-resolver invariant doc

- **Audit source**: codex 6.1 (Cluster C14, Decision D10)
- **Verification status**: Confirmed (Minor; enforcement deferred)
- **File(s)**: `crates/opi-ai/src/api_mapped.rs:24-28, 47-53` (doc only)
- **Change**: Promote the module/`try_new` doc from "callers should" to "callers MUST construct every route from the same lazy `AuthResolver`" with a one-line correct-usage example and a note that the type cannot enforce this at the trait-object boundary.
- **Test plan**: doc-only (no guard). Enforcement via constructor API change is a deferred residual (see exclusions).

#### Fix 4.4: snapshot spec-hash re-sync (required by Fix 4.1)

- **Audit source**: operational consequence of C8/C13 doc edits (per `phase4_ledger` / `phase6_ledger` guards)
- **Verification status**: N/A (mechanical)
- **File(s)**: `docs/snapshots/phase4/opi-impl-state.json`; `docs/snapshots/phase6/opi-impl-state.json`
- **Change**: after Fix 4.1 lands, recompute the CRLF-normalized SHA-256 of `docs/opi-spec.md` (`spec.replace("\r\n","\n")` then `Sha256::digest`) and write it to `spec_files_sha256["docs/opi-spec.md"]` in **both** snapshot ledgers.
- **Test plan**: `cargo test -p opi-coding-agent --test phase4_ledger` and `--test phase6_ledger` pass.
- **Flag**: the **live** `.opi-impl-state.json` `spec_files_sha256` will also drift; no test guards it, and opi-remediate does not edit the live ledger. Surface this to the user for an opi-implement re-sync.

---

## Final verification

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Note (host): the full-workspace gate is disk-heavy (~106 GB `target/`); reclaim with `cargo clean` if the 452 GB `D:` drive fills. Per-crate layer gates are authoritative during layer work.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| C14 (enforcement half) | Deferred | Strengthened doc (Fix 4.3) addresses the documentation gap; the enforceable fix requires a breaking `ApiMappedProvider` constructor API change disproportionate to a Minor, production-correct invariant. Revisit if an embedder misuse surfaces. |
| C10 (Fix 3.5) | Deferred | Process-integrity finding on already-archived/historical artifacts (plan doc + frozen snapshot ledger). The concrete defect (3 stale command names, 5 dead `behavioral_tests` paths) lives in frozen Phase-14 artifacts with zero product impact; the executable manifest validator is a test-infrastructure feature addition disproportionate to a process finding on completed work. Revisit if Phase 14 acceptance is ever re-executed. |
| C6(c) JSONL chokepoint | Optional / covered | Fix 1.4 (provider-side neutralization) is the root cause and closes the JSONL leak; Fix 3.3 is optional belt-and-suspenders. |
| Live `.opi-impl-state.json` spec-hash | Flagged (out of scope) | opi-remediate must not edit the live ledger; drift has no test consequence. opi-implement should re-sync. |
| Gemini / Vertex SSE-error redaction | Not in scope | C6 named only anthropic/chat/responses. Not audited for the same pattern; recommend a follow-up audit rather than an unverified fix. |
| `opi-spec.zh.md` hash re-sync | N/A | No guard hashes the ZH spec; only EN counterparts in C8/C13 require the snapshot re-sync. |

## Relationship to the live ledger

This plan does **not** read or write `.opi-impl-state.json`. It writes only: source/test files in the three crates, `docs/opi-spec.md` / `.zh.md`, the phase-14 plan doc, the archived `docs/snapshots/phase14/opi-impl-state.json` (behavioral_tests paths — a doc artifact, not the live ledger), and the phase4/phase6 snapshot ledger `spec_files_sha256` values (required by the guard tests after the spec edit).
