# Phase 14 Provider & Auth — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior phase-14 `audit.*.md` or evaluator
transcripts consulted)
**Date**: 2026-07-20
**Scope**: Tasks 14.1–14.21, commit range `d9f21a9..8364e74` (phase exit) **plus**
post-exit remediation commits `9263114..b27905a` (HEAD = `b27905a`)
**Method**: Spec-driven, full-file reads of all affected source + tests across
`opi-ai`, `opi-agent`, and `opi-coding-agent`, with five parallel deep-read
subagents organized by file group plus the lead auditor's own independent
verification of the highest-risk paths (thinking rejection, header injection,
SSE redaction, dynamic-refresh validation, the credential probe path, and the
outer-TUI retry state machine). Each Blocker/Major candidate raised by a
subagent was re-verified against the code by the lead before being accepted or
rejected.

**Contamination disclosure**: The lead's auto-loaded session memory contained a
process-state note summarizing prior phase-14 audit outcomes (a 3-way
codex/glm5.2/opus4.6 split and the remediation status). This was not sought out
and is treated as structural process metadata only; all findings below are
derived independently from source, tests, the two normative design specs, and
git history. No `audit.*.md`, evaluator transcript, or `remediation-plan.md`
was read.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 0     |
| Minor    | 4     |
| Info     | 9     |

Phase 14 is a large, security-critical provider/auth cluster (21 tasks, two
crates, credential persistence, OAuth for three providers, per-stream auth
re-resolution, Usage/cost accounting, multi-wire routing). At HEAD `b27905a`
the implementation is **security-correct and spec-compliant**: secrets are
redacted at every formatting site, the cross-process mutation lock is
non-reentrant and bounded, corrupt/unknown credential envelopes never fall
through to env, malformed Usage subsets are rejected rather than clamped,
Anthropic cache markers fire at the exact reviewed positions/TTLs, and the
`ApiMappedProvider` rejects unknown models / missing routes before network IO.

The 20 code-level findings from the prior refreshed audit (C1–C9, C11–C21) are
**genuinely fixed at HEAD**. The lead independently re-verified the four
highest-risk claims: C8 (Chat/Responses unsupported-thinking rejection),
C6 (upstream SSE error-text redaction), C15 (dynamic-refresh candidate
validation), and C-03-class header-injection concerns — all confirmed fixed,
and C-03 is a confirmed false positive (the Codex path defends
`chatgpt-account-id` via its own `MANAGED_HEADERS`). The one deferred item
(C10, process-integrity in frozen archived artifacts) is a process/meta
finding, not a code defect, and its deferral is consistent with the project's
immutability rules for released/archived material.

Surviving findings are Minor and Info: defense-in-depth gaps (a non-zeroizing
envelope encode buffer, a `debug_assert!`-only lease guard), a test that does
not assert the property it names, and several spec-wording-vs-implementation
nuances. None blocks the next phase.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1  | Credential store model                                   | PASS |
| 14.2  | OAuth architecture and per-request auth re-resolution    | PASS |
| 14.3  | Request scalars and session-affinity production path     | PASS |
| 14.4  | Model capabilities and Anthropic cache markers           | PASS |
| 14.5  | Usage and cost cache/reasoning accounting                | PASS |
| 14.6  | Dynamic provider model refresh                           | PASS |
| 14.7  | Provider/auth docs, non-goal guards, final gates         | PASS |
| 14.8  | Native keyring and production probes                     | PASS |
| 14.9  | Login/logout dispatcher and persistence                  | PASS |
| 14.10 | Live auth and session interaction                        | PASS |
| 14.11 | Factory-built Anthropic cache markers                    | PASS |
| 14.12 | Usage and cost contract                                  | PASS |
| 14.13 | Documentation, verification, residual closure            | PASS |
| 14.14 | Native keyring host selection                            | PASS |
| 14.15 | WireApi, model metadata, pricing, thinking, canonical IDs| PASS |
| 14.16 | ApiMappedProvider and TOML custom providers              | PASS |
| 14.17 | GitHub Copilot three-wire catalog                        | PASS |
| 14.18 | OpenAI Codex dedicated wire, catalog, dual login         | PASS |
| 14.19 | Concrete OAuth dispatcher vertical path                  | PASS |
| 14.20 | Outer TUI credential retry                               | PASS |
| 14.21 | Documentation, acceptance artifacts, Phase F             | PASS |

---

## 2. Correctness Findings

No correctness defects found. Subset-validation arithmetic, cost calculation,
cache-marker placement, `ApiMappedProvider` routing, session-affinity
propagation, and the outer-TUI retry state machine were all traced and verified.

**Cost arithmetic (verified).** `calculate_cost` (`stream.rs:264–281`) folds the
1-hour cache-write subset into `cache_write_cost` at 2× input rate
(`cache_write_1h * input_cost_per_mtok * 2.0`) plus the short-cache remainder at
`cache_write_cost_per_mtok`; reasoning stays inside `output_cost`; `total_cost`
sums the four parent lines once. Numeric trace with the `Usage(500k, 250k, 1M,
500k, Some(150k), Some(0))` fixture yields `7.7625`, matching the test
assertion exactly. No double-count.

**Subset rejection (verified).** All three mappers reject child-greater-than-parent
with non-retryable `ProviderError::StreamError` and emit no `Usage` event
carrying the invalid data: Anthropic 1h (`anthropic.rs:181–186`, propagated via
`?` in `from_raw` before event construction), OpenAI Chat reasoning
(`openai_chat.rs:188–204` `validate_usage_subset`, invoked before chunk
materialization), and OpenAI Responses/Codex reasoning
(`openai_responses_shared.rs:260–280` returns `ParsedEvent::Malformed` before
building `Usage`). None clamps or silently drops.

**`ApiMappedProvider` routing (verified).** Construction-time validation
(`api_mapped.rs:50–155`) rejects empty ids, duplicate models, missing routes,
route/provider id mismatch, and route-catalog subset/superset mismatch. On
stream, an unknown model yields `ProviderError::UnknownModel` synchronously via
`stream::once` — zero auth calls, zero HTTP (test
`unknown_model_fails_before_route_or_network` confirms `auth.calls == 0`).

**Outer-TUI retry state machine (verified).** `PromptAuthStateMachine`
(`interactive.rs:309–450`) gates retry on the **same** provider
(`pending_auth_provider == Some(provider_id)`, L375), reuses the pending turn
via `harness.retry_last_prompt()` (no new user message), sets `may_arm_retry:
false` on the retry turn (L394) so the cycle is bounded to one retry, and
requires `may_arm_retry && !output_began` to arm (L426–427). Every other exit
(CredentialRevoked, generic error, JoinError, different-provider login, login
failure) clears `pending_auth_provider` (L432–447) → zero retries. SC3/F14-04
satisfied.

---

## 3. Security / Redaction Findings

### 3.1 Minor: Envelope encode buffer is not zeroized

**File:** `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** 479–507 (`encode_credential`)
**Cause:** `encode_credential` pulls raw secrets via `expose_secret()` into a
plain `EnvelopeFields` (`Option<String>`) and `serde_json::to_string` produces a
`String` containing the live access/refresh tokens. The `secrecy::SecretString`
fields on `Credential` are zeroized on drop, but this derived JSON `String` is
only deallocated, not zeroized.
**Impact:** Defense-in-depth gap only — no normal-path leak. A memory-inspection
attacker (or core dump) on a long-running opi process could recover
recently-encoded envelope strings from deallocated-but-not-zeroized heap pages.
**Fix:** Encode into `zeroize::Zeroizing<String>` (and a `Zeroizing` envelope
buffer), so the serialized secret is wiped after `backend.set_password`.

### 3.2 Minor: `NativeKeyringGuard::Drop` lease-underflow guard is debug-only

**File:** `crates/opi-coding-agent/src/native_keyring.rs`
**Lines:** 37–38
**Cause:** `debug_assert!(state.leases > 0, …); state.leases -= 1;`. In release
builds the assert compiles out. The `self.leased` flag (L33) prevents
double-drop of the *same* guard, but a latent lease-accounting bug elsewhere
would wrap `leases` to `usize::MAX`, leaving the `set_default_store`'d store
installed for the process lifetime (never unset).
**Impact:** Only triggers on a hypothetical lease-accounting bug, not on a
normal path. If triggered, the default keyring store leaks for the process
lifetime and future `install_native_keyring` calls in the same process see
`leases > 0` and skip construction.
**Fix:** Use `saturating_sub` plus a release-mode path that logs and re-derives
the true count, or guard the decrement unconditionally.

### 3.3 Minor: Two parallel reserved-header lists can drift

**File:** `crates/opi-ai/src/provider.rs:166–205` (`validate_extra_headers`,
5-name `RESERVED`); `crates/opi-ai/src/provider_headers.rs:10–24`
(`RESERVED_PROVIDER_HEADERS`, 13 names); `crates/opi-ai/src/openai_codex_responses.rs:27–36`
(`MANAGED_HEADERS`, 8 names)
**Cause:** Three divergent reserved-header lists coexist. The Codex path is
defended by its own `MANAGED_HEADERS` check (`openai_codex_responses.rs:381–388`,
which rejects `chatgpt-account-id`, `session-id`, `x-client-request-id`,
`openai-beta`, `originator`, etc.), and Anthropic/Chat/Responses route
`extra_headers` through the strict `ProviderHeaders::merge_request`
(`anthropic.rs:1285`, `openai_chat.rs:1505`, `openai_responses.rs:531`), so
**there is no live vulnerability** — a user-supplied managed header is rejected
on every wire. But `validate_extra_headers` is self-admitted "non-exhaustive"
(`provider.rs:160–163`) and a future provider wired up with only that 5-name
gate would inherit the weak check.
**Impact:** Refactor hazard, not a present defect. The prior-audit
header-injection concern (Codex `validate_extra_headers` at
`openai_codex_responses.rs:380`) is a **confirmed false positive at Major** — the
`MANAGED_HEADERS` guard immediately below it closes the hole.
**Fix:** Unify on one canonical reserved-header list shared by all providers, or
delete `validate_extra_headers` in favor of `ProviderHeaders::merge_request`
everywhere.

### 3.4 Info: Error `reason` fields have no documented secret-free contract

**File:** `crates/opi-ai/src/credential.rs:152–177`
**Lines:** `BackendUnavailable.reason`, `Backend.reason`, `MalformedEnvelope.reason`
**Cause:** These free-form `reason: String` fields are surfaced via
`Display`/`Debug` into doctor/listing diagnostics. The current
`KeychainCredentialStore` populates them only from fixed string literals or
backend `Display` output (never the payload — verified
`MalformedEnvelope` reasons are literals at `credential_store.rs:519/535/547/555/561`),
so there is **no leak today**. There is, however, no documented contract that
`reason` must stay secret-free.
**Impact:** A future backend or error-path change that echoes the offending
payload (e.g. a malformed JSON snippet containing an embedded token) would leak
via diagnostics.
**Fix:** Add a doc invariant on `CredentialStoreError` that `reason` must never
echo credential payload, and assert it in the redaction canary test.

---

## 4. Test Quality Findings

### 4.1 Minor: `refresh_models_deterministic_ordering` does not assert order

**File:** `crates/opi-ai/tests/provider_collection.rs`
**Lines:** 1112–1138
**Cause:** The test registers providers in non-sorted order (`["zulu", "alpha",
"mike"]`), calls `refresh()`, then resolves each provider's refreshed model.
None of the asserts depend on refresh order — they would pass even if
`registry.provider_ids()` returned insertion order. The determinism guarantee
(`provider_collection.rs:443` uses the sorted `provider_ids()`) is proven only
indirectly through the implementation of `provider_ids()`.
**Impact:** A regression that broke refresh ordering (e.g. switching to
insertion order) would not be caught.
**Fix:** Assert the install/replace order explicitly, or assert
`registry.provider_ids()` is sorted in this test.

### 4.2 Info: No CHANGELOG "Fixed" section for in-phase remediation

**File:** `CHANGELOG.md` (`## [Unreleased]`)
**Cause:** The remediation commits fixed real defects (the C1–C21 set) within
the unreleased phase-14 body. The Unreleased section records the Breaking /
Added / Changed entries accurately but has no `### Fixed` subsection.
**Impact:** Defensible — all phase-14 work is unreleased, and fixes within
unreleased work arguably do not need separate Fixed entries. Noted for
completeness against Keep a Changelog conventions.
**Fix:** Optional — add a brief `### Fixed` subsection citing the
cancellation/error-classification/redaction repairs, or document the omission as
intentional.

---

## 5. Spec Compliance Findings

SC1–SC8 are all **met** at HEAD; NG1–NG8 are all **respected**; F14-01..04 are
closed; and the 2026-07-17 alignment-revision obligations (`WireApi`,
`ApiMappedProvider`, Copilot 3-wire, dedicated `openai-codex-responses` wire +
Browser/Device-Code login, `chatgpt_account_id` persistence) are implemented.

### 5.1 Info: Capability preflight is agent-loop-level, not provider-level

**File:** `crates/opi-agent/src/agent_loop.rs:106`; `crates/opi-ai/src/provider.rs:234–284`
**Cause:** `validate_request_capabilities` is invoked once in the agent loop
before `context.provider.stream()` (`agent_loop.rs:106` → `:122`). It rejects
unsupported thinking levels (via `thinking_level_map.resolve()`) and
text-only-model image input. The HEAD spec text says "the provider returns
`ProviderError::UnsupportedCapability`," but the rejection is actually performed
by the agent loop, not inside each provider. Library consumers that call
`provider.stream()` directly bypass the preflight. Unknown model IDs skip the
thinking preflight by design (`provider.rs:264`, so configured custom deployments
still work).
**Impact:** Behavior matches the spec on the product (agent) path, which is the
only path opi ships. The wording imprecision and the library-user gap are
nominal. C8 (the `9263114` Chat/Responses unsupported-thinking exemption) is
**genuinely reverted at HEAD** — the `9263114` spec softening ("emit when
resolved") was replaced by `b27905a`'s rejection wording, and the code rejects
via the agent loop.
**Fix:** Optional — move the preflight into a shared `Provider::stream` wrapper,
or adjust the spec wording to "the agent loop rejects… before delegating to the
provider."

### 5.2 Info: `Request` field shapes diverge from spec wording

**File:** `crates/opi-ai/src/provider.rs:85–96`
**Cause:** Spec says `extra_headers: HeaderMap` and `cache_retention:
Option<CacheRetention>`. Implementation uses `extra_headers:
Vec<(String,String)>` and `cache_retention: CacheRetention` (with a `None`
default variant). Both are functionally equivalent (`CacheRetention::None`
plays `Option::None`'s role; the `Vec` form is serde-friendly and validated
through `HeaderName`/`HeaderValue::from_str`).
**Impact:** External 0.x consumers reading the spec will write code expecting a
`HeaderMap` API; the actual surface differs structurally. No behavioral defect.
**Fix:** Optional — update the spec to match the shipped `Vec`/enum shapes.

### 5.3 Info: C10 deferral (frozen archived artifacts) — process finding

**Cause:** The refreshed audit's C10 (process-integrity defects in frozen
archived artifacts) is deferred per `docs/snapshots/phase14/remediation-plan.md`.
The commit metadata (`24538f2` "add phase 14 audit reports" → `9ec970b`
"refresh… for post-remediation HEAD") indicates C10 concerns already-frozen
audit/snapshot material, where modification would conflict with the project's
immutability rules for released/archived artifacts (CHANGELOG: "NEVER modify
already-released version sections"; ledger: "do not delete or hand-edit the
canonical file").
**Impact:** No code defect. To preserve audit independence the lead did not read
`remediation-plan.md`, so C10 is not fully characterized here.
**Fix:** Maintainer to confirm the C10 deferral rationale is recorded in the
plan and that no *code* defect was deferred under C10 (only the process/meta
item).

### 5.4 Info: Codex subset-violation diagnostics collapse to a generic message

**File:** `crates/opi-ai/src/openai_codex_responses.rs:242–247`
**Cause:** When the shared Responses parser rejects a reasoning-tokens subset
violation, the Codex provider replaces the parser's specific message with the
generic `MALFORMED_STREAM_ERROR` sentinel. The standard
`OpenAiResponsesProvider` does the same generic replacement. Error type
(`StreamError`) and rejection behavior (no `Usage` event with invalid data) are
correct.
**Impact:** Display-only. An operator debugging a Codex stream sees a generic
"malformed streaming data" message with no hint that a subset invariant tripped.
**Fix:** Optional — thread the parser's structured reason through to the
`StreamError` message for the subset-violation case.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| No opi-managed plaintext credential file (NG1) | All persisted credentials go to the OS keychain via `keyring-core`; env is read-only fallback for API keys | `credential_store.rs` redaction/temp-root scan (L1446–1524) proves only the secret-free `credential.lock` exists outside the fake keyring |
| Corrupt/unknown envelope never falls through to env | `resolve_api_key` (`credential_store.rs:943–984`) returns hard errors for OAuth-kind marker, OAuth-kind protected, marker+no-protected, malformed, unknown version/type; env only on `BackendUnavailable` for API keys | `credential_store.rs` tests L1304–1373 |
| OAuth persistence is keychain-required (no env fallback) | `resolve_oauth` returns `CredentialNeeded` on absence (`credential_store.rs:1016–1020`); never queries env | `oauth_auth.rs` absence/revocation tests |
| Mutation lock is single, non-reentrant, bounded | `LockCoordinator` fs4 exclusive lock at `<user_config_dir>/credential.lock`; `write_unlocked`/`delete_unlocked`/`read_unlocked`/`acquire_lock` are `pub(crate)` and lock-free; refresh holds one lock across re-read→HTTP→write using unlocked ops (no recursive `write`) | `refresh_timeout_releases_lock_and_preserves_prior_credential` (`oauth_auth.rs:5336`); lock-contention tests |
| Refresh HTTP is bounded shorter than lock hold | `OAUTH_REFRESH_TIMEOUT = 30s` (`credential_store.rs:119`); lock-wait 5s production; timeout drops the future, releases the lock, writes no partial credential | same |
| Secrets redacted at every formatting site | Manual redacting `Debug` on `Credential`/`OAuthCredential`/`ResolvedAuth`/`SecretKey`; `expose_secret()` only at the 4 concrete HTTP boundaries; upstream SSE error text neutralized (`openai_chat.rs:694–696`, shared Responses path, Codex `UPSTREAM_STREAM_ERROR`) | multi-stage canary `openai_codex_bounded_redaction_scenario`; `pkce_token_endpoint_unknown_error_code_is_closed_and_redacted`; Copilot/Codex leak canaries |
| Doctor/list-models probes are secret-free | `collect_credential_probes` → `probe_metadata` → `read_marker_kind` reads only the non-secret `opi.presence` marker service, never the protected `opi` envelope | `doctor_cli.rs`, `list_models.rs` inject present/absent/unavailable backends |
| Doctor distinguishes Present/Absent/BackendUnavailable | `provider_diagnostics` (`doctor.rs:407–439`) matches on `probe.state` using only `probe.label` (display_source); uses precomputed redacted `store_probe`, not the laxer `auth_status()` | `doctor_cli.rs` three-state assertions |
| Same-provider-only same-turn retry, no duplicate user message | `PromptAuthStateMachine` same-provider gate (L375), `retry_last_prompt` (L397), `may_arm_retry:false` on retry (L394) | `interactive_tui_auth.rs` one-message/two-calls/one-retry + negative paths |
| `ApiMappedProvider` rejects before network on unknown model / missing route / wire mismatch | `api_mapped.rs:50–155` construction validation; `stream` L175–197 returns `UnknownModel` via `stream::once` pre-network | `api_mapped_provider.rs` construction + `unknown_model_fails_before_route_or_network` |
| Anthropic cache markers only at capable positions/TTL; custom/unknown off | `anthropic.rs:810–820` capability gate (`supports_cache_control.unwrap_or(false)`); markers on system + last user/assistant/tool; `ttl:"1h"` only when `Long && supports_long_cache_retention` | `anthropic.rs` marker tests + `anthropic_cache_markers.rs` factory test (4 markers capable+Long, 0 for custom/unknown) |
| Usage subset semantics; no double count | `stream.rs:97–107` total counts parents only; `calculate_cost` L272–279 | `usage_cost.rs` subset + cost tests |
| `opi-agent` does not construct providers (load-bearing invariant) | Auth/resolver/store/lock live in `opi-coding-agent`; `agent_loop` calls `context.provider.stream()` only | preserved — no new provider construction in `opi-agent` |

---

## 7. Cross-task Integration

No integration defects found. The auth/resolver/store/lock seams are consistent
across `opi-ai` (abstract, IO-free) and `opi-coding-agent` (concrete, IO-bearing).
The factory (`provider_factory.rs`) injects a lazy `AuthResolver`
(`CredentialAuthResolver` / `AuthSource::Store` / `AuthSource::Layered`) into the
approved Anthropic, GitHub Copilot, and OpenAI Codex providers; each holds the
resolver + provider id, **not** the secret, so a changed store credential is
observed by the next stream on the same constructed provider. Old `copilot` /
`codex` ids are rejected with a canonical rename hint
(`provider_factory.rs:1809–1820`) — no silent alias. The single
`PromptAuthStateMachine` is shared by the real `tui_event_loop` and the debug
scripted driver (no duplicated branching).

---

## 8. Residuals and Recommendations

### Priority recommendations

1. **Unify the reserved-header lists** (§3.3). Three divergent lists is the
   single largest maintainability hazard in the auth surface; collapse to one
   canonical list so a future provider cannot accidentally inherit the 5-name
   gate.
2. **Zeroize the envelope encode buffer** (§3.1) and **harden the keyring lease
   guard for release builds** (§3.2). Both are low-cost defense-in-depth fixes.
3. **Strengthen the determinism test** (§4.1) to actually assert refresh order.
4. **Confirm the C10 deferral** (§5.3) is recorded as a process/meta item only,
   with no code defect hidden behind it.

### Carry-forward items (explicitly deferred by the spec, not defects)

- Per-call credential override (`ApiStreamOptions`) — fog.
- `onPayload`/`onResponse` streaming hooks (T3 3d) — fog.
- End-to-end `SecretString`-through-provider-construction refactor (T1 D5 scope
  cap) — the capability preflight being agent-loop-level (§5.1) is related; a
  future refactor could move preflight into providers and close the library-user
  gap at the same time.
- Production trigger for `refresh_models` — substrate-only in Phase 14 by design.

### Notes for the next phase

- The live `.opi-impl-state.json` `spec_files_sha256` now drifts from the
  checked-in `docs/opi-spec.md` hash (the spec was edited during remediation and
  the phase4/phase6 snapshot hashes were re-synced to `0a961b6a`, but the live
  ledger was not). `opi-implement` should re-sync the live ledger raw hash before
  the next phase's guards are exercised; this is a process item, not a Phase 14
  code defect.
