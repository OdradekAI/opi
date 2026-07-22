# Phase 14 Provider & Auth — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit/remediation/evaluator reports consulted)
**Date**: 2026-07-22
**Scope**: Tasks 14.1–14.21, commits through `HEAD` `8d6e6ca` (post-pass-3-remediation). Phase exit recorded at `8364e74a` (2026-07-19); four remediation commits landed after (`9263114`, `b27905a`, `47400ee`, `8d6e6ca`). Audit target is the post-remediation HEAD.
**Method**: Full read of the opi-ai contracts and security-critical implementations by the auditor (`openai_responses_shared.rs`, `openai_codex_responses.rs`, `stream.rs`, `credential.rs`, `auth.rs`, `provider.rs`, `provider_collection.rs`, `provider_headers.rs`, `credential_store.rs`, `native_keyring.rs`, `openai_chat.rs`, plus targeted verification of `oauth.rs` refresh/dispatch, `provider_factory.rs` Copilot construction, and the opi-agent typed-error mapping). Five parallel evidence-gathering passes covered the remaining source and test breadth (Anthropic markers/usage-cost; WireApi/ApiMapped/Copilot/custom TOML; credential store/keyring/doctor/listing; OAuth providers/dispatcher/refresh; TUI retry/session-affinity/non-interactive modes). Every candidate finding below was re-verified against source by the auditor before writing it up; the elevated candidate (M-1) was independently confirmed by reading `interactive.rs`.

**Contamination note**: The auditor's persistent memory carried a summary of phase-14's prior audit history (pass-3 Blocker C-2.1 on the Responses/Codex SSE decoder; Major C-3.2 on `AccountIdMissing`). That memory was treated as untrusted: every conclusion here is derived from freshly read code, and two candidate findings the memory suggested were discarded after code verification (see §9). The prior `audit.glm5.2.md` was overwritten without being read.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 1     |
| Minor    | 3     |
| Info     | 10    |

Phase 14 is in strong shape. Across 21 tasks and three crates the credential/OAuth/auth cluster is implemented carefully: the OS-keychain store enforces a single cross-process lock with acquire-then-re-read and no recursive locking; refresh-token rotation is serialized under that lock with a bounded timeout, no partial writes, and post-failure re-read; the typed `CredentialNeeded`/`CredentialRevoked`/`AccountIdMissing` errors flow through variant matching (no string parsing) into both `AgentError` and the JSON/RPC/text surfaces; redaction is mechanical and test-backed (only the secret-free `credential.lock` exists outside the fake keychain). The two highest-severity prior findings are genuinely and fully fixed: the Responses/Codex SSE decoder now dispatches on the JSON `type` field (data-only wire) and is covered by a canonical pi-0.80.6 data-only fixture, and `account_id` is validated on both Codex login **and** refresh.

The single Major is a test-coverage gap, not a live defect: every outer-TUI retry test installs a debug-only headless driver that short-circuits `run_interactive_tui` before the real `tui_event_loop`, so the production interactive credential-retry path (and the entire release-mode interactive loop) has no automated coverage. The shared state machine is well-tested; the residual risk is the event-loop→state-machine wiring.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | PASS |
| 14.2 | OAuth architecture and per-request auth re-resolution | PASS |
| 14.3 | Request scalars and session-affinity production path | PASS (minor test gap, m-1) |
| 14.4 | Model capabilities and Anthropic cache markers | PASS |
| 14.5 | Usage and cost cache/reasoning accounting | PASS |
| 14.6 | Dynamic provider model refresh | PASS |
| 14.7 | Provider/auth docs, non-goal guards, final gates | PASS |
| 14.8 | Native keyring and production probes | PASS |
| 14.9 | Login/logout dispatcher and persistence | PASS |
| 14.10 | Live auth and session interaction | PASS |
| 14.11 | Factory-built Anthropic cache markers | PASS |
| 14.12 | Usage and cost contract | PASS |
| 14.13 | Documentation, verification, residual closure | PASS |
| 14.14 | Native keyring host selection | PASS |
| 14.15 | WireApi, model metadata, provider-id migration | PASS |
| 14.16 | ApiMappedProvider and TOML custom providers | PASS |
| 14.17 | GitHub Copilot three-wire catalog | PASS |
| 14.18 | OpenAI Codex dedicated wire, catalog, dual login | PASS |
| 14.19 | Concrete OAuth dispatcher vertical path | PASS |
| 14.20 | Outer TUI credential retry | PASS-WITH-FINDINGS (M-1) |
| 14.21 | Documentation, acceptance artifacts, Phase F | PASS |

---

## 2. Correctness

### 2.1 INFO: Responses/Codex SSE loop does not drain a trailing unterminated frame

**File:** `crates/opi-ai/src/openai_responses_shared.rs:801-811`; consumer `crates/opi-ai/src/openai_codex_responses.rs:227-256`
**Cause:** `drain_sse_frames` only emits frames delimited by `\n\n`. After the byte stream ends, `stream_http` checks `!mapper.saw_done` but does not call `drain_sse_frames` one final time on the residual buffer. A final frame not terminated by a blank line would be dropped and the stream reported `StreamError("stream ended without a terminal event")`.
**Impact:** Negligible in practice — the SSE spec mandates event termination by a blank line, and real Responses/Codex streams (including `[DONE]`) are `\n\n`-terminated. No fixture exercises a trailing non-blank-terminated frame.
**Fix:** Optional — after the loop, drain the residual buffer before the `!saw_done` check, for robustness against non-conformant proxies.

### 2.2 INFO: OpenAI Chat compatible-affinity headers reuse the session id for three distinct headers

**File:** `crates/opi-ai/src/openai_chat.rs:1488-1500`
**Cause:** When `send_session_affinity_headers` is enabled, the same clamped `session_id` value is emitted for `session_id`, `x-client-request-id`, and `x-session-affinity`. The dedicated Codex/Responses paths instead generate a fresh UUID v7 for `x-client-request-id` (`openai_codex_responses.rs:301-305`).
**Impact:** `x-client-request-id` is intended as a per-request tracing id; reusing the session id defeats that, but only on the opt-in compatible-profile path. Defensible.
**Fix:** None required; consider a distinct request id if compatible-profile tracing matters.

### 2.3 INFO: `reasoning_tokens ⊆ output_tokens` enforced at mappers, not in the `Usage` struct

**File:** `crates/opi-ai/src/stream.rs:74-108` (`Usage::reported`, `total_tokens`); enforced at `crates/opi-ai/src/openai_chat.rs:188-204` and `openai_responses_shared.rs:350-385`
**Cause:** The subset contract is asserted in tests and enforced by the Chat/Responses mappers (`validate_usage_subset` / `parse_response_usage` → `StreamError`), but `Usage::reported` and `CumulativeUsage::accumulate` accept arbitrary `reasoning_tokens` without clamping. Anthropic never populates the field (always `None`).
**Impact:** None today — every producing mapper validates. A future mapper that forgets to validate could construct a non-subset `Usage`. The `cache_write_1h` subset is symmetric: enforced in the Anthropic mapper (`anthropic.rs:178-186`).
**Fix:** Optional defense-in-depth — validate in `Usage::reported`, or document the mapper-enforcement contract on the struct.

### 2.4 INFO: Unknown OpenAI Chat `finish_reason` maps to `StopReason::Error`

**File:** `crates/opi-ai/src/openai_chat.rs:710-718`
**Cause:** `map_stop_reason` returns `StopReason::Error` for any value other than `stop`/`length`/`tool_calls`/`content_filter`.
**Impact:** OpenAI's documented set is closed, so this is unreachable today; a newly added finish reason would surface as a stream error rather than `Stop`.
**Fix:** None required; consider mapping unknown to `Stop` if forward-compat is preferred.

---

## 3. Security / Redaction

No Blocker, Major, or Minor security findings. The credential surface is tight:

- `SecretString` wraps every secret; manual `Debug` impls redact on `Credential` (`credential.rs:71-93`), `ResolvedAuth` (`auth.rs:79-90`), `OAuthCredential` (`auth.rs:165-175`), and `SecretKey` (`provider_collection.rs:71-81`).
- `expose_secret()` is called only at the concrete HTTP boundary and the persistence codec; the serialized envelope is `Zeroizing<String>` with explicitly zeroized intermediates (`credential_store.rs:480-521`).
- Probe reads only the non-secret `opi.presence` marker — a test sets a pending error on the protected entry and proves the probe never touches it (`credential_store.rs:1591-1647`).
- Malformed/unknown envelopes and operational backend errors never collapse to absence or env fallback; only `Absent`/`BackendUnavailable` permit API-key env fallback (`credential_store.rs:956-997`), pinned by six redaction/fallback tests.
- The temp-root scan proves only the secret-free `credential.lock` exists outside the fake keychain (`tests/credential_store.rs:1445-1524`).
- Token-endpoint errors pass through a closed `oauth_error_class` map; loopback callback bodies are fixed and secret-free; loopback binds `127.0.0.1` only.

### 3.1 MINOR: Marker/envelope write is a two-step, non-atomic sequence

**File:** `crates/opi-coding-agent/src/credential_store.rs:761-769` (`write_unlocked`)
**Cause:** `write_unlocked` writes the presence marker to `KEYCHAIN_PRESENCE_SERVICE` before the protected envelope to `KEYCHAIN_SERVICE`. The `resolve_oauth` fast path reads lock-free (`credential_store.rs:1022-1034`). Across a kind-**changing** write (e.g. a stored API key replaced by `/login anthropic` OAuth), a concurrent fast-path reader can transiently observe the new-kind marker with the still-old-kind envelope and get a typed `UnexpectedCredentialKind` error (`read_oauth` → `credential_store.rs:1140-1146`). Same-kind writes (every refresh, OAuth→OAuth re-login) are unaffected because the marker value is unchanged and each backend `set` is atomic per entry.
**Impact:** A transient, typed, self-healing error on the next read; no secret leak, no corruption. The OS keychain has no read-refresh-write transaction (acknowledged in the module docs). No test exercises a kind-changing write concurrent with a fast-path read.
**Fix:** Optional — write the protected envelope before the marker so a transitional reader sees old-kind marker + new-kind envelope (same typed-error outcome, narrower window), or document the accepted transient.

### 3.2 INFO: `encode_credential` exposes secrets at the persistence boundary

**File:** `crates/opi-coding-agent/src/credential_store.rs:485, 498-499`
**Cause:** The 2026-07-11 spec phrases `expose_secret()` as "only at the narrow concrete-provider HTTP boundary." Serialization for the OS-keychain write is a second, sanctioned exposure site.
**Impact:** None — the output is `Zeroizing<String>` and intermediates are zeroized; the keychain is the protected persistence boundary.
**Fix:** Spec-wording only — admit "persistence + HTTP boundaries."

---

## 4. Test Quality

### 4.1 MAJOR: The real `tui_event_loop` is never exercised by any test

**File:** `crates/opi-coding-agent/src/interactive.rs:742-753` (short-circuit) and `781-1116` (production loop); tests in `crates/opi-coding-agent/tests/interactive_tui_auth.rs`
**Cause:** When a test driver is installed, `run_interactive_tui` returns early via `run_headless_interactive_tui_driver` (`#[cfg(debug_assertions)]`, lines 742-753) and never reaches `tui_event_loop` (line 763). Every 14.20 success and negative scenario installs such a driver, so the production crossterm event loop — render, key dispatch, auth-command branch, `cancel_token` refresh (line 790/817), exit-time pending-turn cancellation (909-913), and `apply_prompt_completion_to_tui` — has zero automated coverage. The short-circuit is `cfg(debug_assertions)`, so the **release-mode interactive path is entirely untested**.
**Impact:** The shared `PromptAuthStateMachine` (the complex retry/pending-turn logic) is well-tested — the headless driver and the real loop both construct it (`interactive.rs:789`), and `tests/interactive_tui_auth.rs` covers the success path (one user message, two provider calls, one retry) and nine negative paths. The residual unverified risk is specifically the event-loop→state-machine wiring: a regression in crossterm dispatch, auth-command routing, cancel-token handling, or completion application would break interactive credential retry in production while every test passes. This is the same area as prior finding F14-04; the DoD's letter ("tests enter `run_interactive_tui`", "share the same state machine") is met, but the phase-exit spirit of real production-path evidence is not.
**Fix:** Add a test that drives `tui_event_loop` against a fake crossterm event source / in-memory terminal **without** installing the short-circuiting driver (e.g. a `PromptAuth` scenario fed through the real poll loop), or refactor so the headless driver reuses the real event loop's dispatch instead of replacing it. At minimum, assert the real loop is reachable in release builds.

### 4.2 MINOR: No harness-level trace of `session_id` through resume/fork into the provider Request

**File:** `crates/opi-coding-agent/src/harness.rs:1261, 1372` (`sync_session_id` on resume/fork); test `crates/opi-agent/tests/agent_loop_mock.rs:405`
**Cause:** `session_id_reaches_every_request` calls `Agent::set_session_id` directly and asserts the `Request`. The harness binding — `CodingHarness::sync_session_id` reading the `SessionCoordinator` id on resume (`harness.rs:1261`) and fork (`harness.rs:1372`) — is verified only by code inspection.
**Impact:** The Agent substrate is covered; the harness→agent binding on the resume/fork replacement paths is not. Spec 14.3 asks for "a real `SessionCoordinator` id through harness → agent loop → mock provider, cover resume/fork replacement."
**Fix:** Add a harness-level test that resumes/forks a session and asserts the resulting provider `Request.session_id` matches the new coordinator id.

### 4.3 MINOR: No OAuth-shaped malformed-envelope redaction test

**File:** `crates/opi-coding-agent/tests/credential_store.rs:1445-1524`
**Cause:** The temp-root redaction scan seeds an ApiKey-shaped malformed payload and asserts the decode error does not echo it. The symmetric OAuth-shaped case (real access/refresh canaries in a structurally malformed envelope) is not pinned.
**Impact:** Low — `MalformedEnvelope` carries only provider+reason by construction, so the redaction holds; the test gap is symmetry only.
**Fix:** Add an OAuth-shaped malformed-envelope redaction case.

### 4.4 INFO: No direct test that refresh vs write serializes through the same lock

The write/write serialization is directly asserted (`mutation_lock_serializes_concurrent_writers`); refresh-path tests exercise locked refresh but not a concurrent writer against the lock simultaneously. Serialization is implied by the shared `_guard` pattern. Low-impact.

---

## 5. Spec Compliance

All eight Success Criteria are met with production-path evidence; all eight Non-Goals are respected. Spot checks:

- **SC1** native keyring host selection traverses the production `install_native_keyring_with` seam with an injected constructor (`native_keyring.rs:218-246`), proving single-flight construction, default-store install, and refcounted guard lifecycle — not `install_store(mock)` directly.
- **SC2/SC3** OAuth flows are flow-correct (Copilot/Codex device-code call `present_device_code` and never `await_manual_code`, pinned by `tests/oauth_auth.rs:2939, 3588`); the dispatcher uses the real registry and concrete providers (`interactive_auth.rs:208-209`).
- **SC4** request scalars are substrate-only (no config/harness producer; `agent_loop.rs:101-103`); `session_id` is the one producer and traverses harness→agent→request.
- **SC5** cache markers gated on `supports_cache_control` at the exact positions (system, last user/assistant text, last tool def), TTL gated on `supports_long_cache_retention && Long`, custom/unknown default off; factory-built test is the keystone (`tests/anthropic_cache_markers.rs`).
- **SC6** strict Usage subsets, no `cache_write_1h_cost` line, 1h subset at 2× input rate, reasoning inclusive in output.
- **SC7** `ProviderCollection::refresh` collects, validates, atomically replaces (`provider_collection.rs:442-493`); substrate-only, no trigger.
- **SC8** docs/guards in place; provider ids are `github-copilot`/`openai-codex` with old aliases rejected and canonical remediation (`provider_factory.rs:1809-1820`).

No Non-Goal was found implemented: no plaintext credential file, no auto-relogin mid-stream (revocation ends the turn; `interactive.rs:439-442`), no per-call credential override (reserved headers rejected, `provider_headers.rs:14-28`), no `onPayload`/`onResponse`, no retry fields on `Request`, no end-to-end `SecretString` migration, no new OAuth providers, no session-schema change.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| No opi-managed plaintext credential file | Only `credential.lock` (secret-free) written outside keychain (`credential_store.rs:599-623`) | `redaction_only_secret_free_lock_exists_outside_fake_keyring` |
| Malformed/unknown envelope never → env fallback | `decode_credential` errors; `resolve_api_key` propagates non-`BackendUnavailable` (`credential_store.rs:956-997`) | `resolver_corrupt_envelope_never_falls_back_to_env` + 5 siblings |
| Refresh holds one lock across read+HTTP+write (no recursion) | `resolve_oauth` uses `write_unlocked` under `_guard` (`credential_store.rs:1045-1083`) | `refresh_timeout_releases_lock_and_preserves_prior_credential` |
| Refresh timeout drops HTTP, releases lock, no partial write, non-retryable | `tokio::time::timeout` wrap; only `Ok` branch writes (`credential_store.rs:1071-1083`) | same test + `concurrent_near_expiry_resolves_coalesce_to_single_refresh` |
| Auth errors typed, non-retryable, no string matching | `agent_loop.rs:509-530` variant match; `is_retryable` excludes auth (`provider.rs:355-360`) | `account_id_missing_provider_error_maps_to_typed_agent_error`; `factory_built_approved_profiles_map_revocation_without_retry` |
| Codex `account_id` required on login **and** refresh | `require_codex_account_id` on login paths + `CodexOAuthProvider::refresh` (`oauth.rs:1329, 1414, 1445`) | `openai_codex_login_rejects_token_without_chatgpt_account_id` + refresh variant |
| Usage subset invariants (1h ⊆ cache_write, reasoning ⊆ output) | Anthropic `into_usage`; Chat `validate_usage_subset`; Responses `parse_response_usage` | `cache_1h_malformed_subset_stops_production_stream...`; `usage_cost.rs` |
| Cache markers only on capable models at exact positions/TTL | `anthropic.rs:819-880` | `tests/anthropic_cache_markers.rs` (factory-built) |
| `opi-agent` never constructs providers | `agent_loop.rs` calls `context.provider.stream`; construction in `provider_factory.rs` | (architectural; held) |
| Copilot 401/403 → `CredentialRevoked` on all three wires | routes built with `CredentialManaged` (`provider_factory.rs:1338, 1350, 1362`) | `github_copilot_provider.rs:615-646` |

---

## 7. Cross-task Integration

No integration defects. Verified handoffs:

- Factory → providers: the three Copilot routes share one `Arc<dyn AuthResolver>` and per-stream resolution is re-evaluated each stream (`factory_stream_reresolves_after_store_change` proves the second stream after a store mutation uses the new token for Anthropic/Copilot/Codex).
- Harness → agent → request: `session_id` flows through `CodingHarness::sync_session_id` → `Agent::set_session_id` → `AgentLoopContext` → `Request` (`harness.rs:1964-1967`, `agent.rs:140-142, 406`, `agent_loop.rs:104`); provider mappings are compatibility-gated and suppressed under `CacheRetention::Disabled`.
- Provider → mapper: the shared Responses SSE parser is reused by standard Responses, Codex Responses, and (via routes) Copilot Responses consistently; error messages are substituted with provider-neutral literals at every boundary.
- Dispatcher → registry → locked store: `/login`/`/logout` traverse `dispatch_auth_command` → real registry → concrete providers → injected locked store; terminal suspension is RAII with exactly-once restoration across all exit paths.

### 7.1 INFO: `ApiMappedProvider` shared-resolver invariant is the caller's responsibility

**File:** `crates/opi-ai/src/api_mapped.rs:5-9`
**Cause:** `try_new` receives routes as `Box<dyn Provider>` and cannot enforce that every route holds the same `Arc<dyn AuthResolver>`. All three production callers (`build_custom_provider`, `build_openai_compatible_profile`, `build_copilot_oauth`) clone one resolver; a third-party extension using the unstable surface could diverge.
**Fix:** Optional — expose a builder that accepts one resolver + per-wire constructor closures to enforce sharing at the API.

---

## 8. Residuals and Recommendations

### Priority recommendations

1. **(M-1)** Add test coverage for the real `tui_event_loop` (release-mode interactive credential retry is currently untested). This is the only Major and the one item to address before considering the phase fully closed on test evidence.
2. **(m-1)** Add a harness-level resume/fork `session_id` trace test.
3. **(m-2)** Decide on marker/envelope write ordering (or document the accepted transient) for kind-changing credential replacements.
4. **(m-3)** Add the OAuth-shaped malformed-envelope redaction test for symmetry.

### Carry-forward / deferred (unchanged, still out of scope)

The design-spec deferrals remain deferred and were verified **not** implemented in this phase: per-call credential override (`ApiStreamOptions`); `onPayload`/`onResponse` streaming hooks; end-to-end `SecretString`-through-construction refactor; runtime model-refresh trigger (SC7 is deliberately substrate-only); broader Copilot/Codex catalogs beyond the reviewed pi-0.80.6 snapshots. Any deferrals recorded in the phase ledger should be carried forward as-is.

---

## 9. Verification of the two prior-pass findings (independently re-derived)

- **C-2.1 (Responses/Codex SSE decoder on data-only wire) — FIXED.** `ResponsesEvent::try_from_frame` now prefers the JSON `type` field over the SSE `event:` name (`openai_responses_shared.rs:226-235`), treats `data: [DONE]` as a no-op (line 214), and the Codex `stream_http` reuses this parser. A canonical pi-0.80.6 data-only fixture (every frame `data:`-only, terminated by `[DONE]`) streams to a typed Done — `dedicated_codex_data_only_frames_stream_to_completion` (`tests/openai_codex_responses.rs:409-456`), with a comment that it reproduced the pre-fix termination. The masking problem (synthetic `event:` fixtures) is closed by the new data-only fixture.
- **C-3.2 (`AccountIdMissing` typed auth) — FIXED, login and refresh.** Missing/empty/whitespace `account_id` yields `ProviderError::AccountIdMissing { provider_id: "openai-codex" }`, non-retryable, zero HTTP (`openai_codex_responses.rs:152-158`; test at `tests/openai_codex_responses.rs:501-529`). Login wires `require_codex_account_id` on both Browser and Device-Code flows, and `CodexOAuthProvider::refresh` calls `refresh_oauth_token` **then** `require_codex_account_id` — re-extracting `account_id` from the refreshed JWT (overwriting, not carrying forward) (`oauth.rs:1428-1447`). The new `AgentError::AccountIdMissing` variant is wired through the agent loop and diagnostics with `/login <provider>` remediation and no string matching.

Two candidate findings suggested by the auditor's prior-memory summary were **discarded after code verification**: (a) "refresh carries forward the old account_id" was false — the Codex refresh re-validates via `require_codex_account_id`; (b) "reasoning subset is unenforced" was overstated — the Chat and Responses mappers enforce it (only the `Usage` struct itself does not).
