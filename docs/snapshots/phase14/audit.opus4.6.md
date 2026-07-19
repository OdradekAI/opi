# Phase 14 Provider & Auth -- Independent Code Audit

**Auditor**: opus4.6 (independent, no prior audit reports consulted)
**Date**: 2026-07-19
**Scope**: Tasks 14.1--14.21, commits `d9f21a97..8364e74a`
**Method**: Full source read of all Phase 14 affected files across opi-ai,
opi-agent, and opi-coding-agent via parallel subagents; test matrix
construction from integration and unit tests; cross-reference against both
design specs.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 2     |
| Minor    | 7     |
| Info     | 5     |

Phase 14 delivers a well-structured credential store, OAuth login/logout,
per-request auth re-resolution, model capabilities migration, usage/cost
accounting, and dynamic model refresh substrate. The architecture cleanly
separates IO-free contracts (opi-ai) from implementations (opi-coding-agent),
and test coverage is strong. The two Major findings concern a silent u64-to-u32
truncation in cumulative usage conversion and a stale module comment that
contradicts the actual OAuth implementation status. Neither is a data-loss or
security vulnerability, but both should be addressed before the next phase.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | Pass |
| 14.2 | OAuth architecture and per-request auth re-resolution | Pass |
| 14.3 | Request scalars and session-affinity production path | Pass |
| 14.4 | Model capabilities and Anthropic cache markers | Pass |
| 14.5 | Usage and cost cache/reasoning accounting | Pass-with-findings |
| 14.6 | Dynamic provider model refresh | Pass |
| 14.7 | Provider/auth docs, non-goal guards, and final Phase 14 gates | Pass |
| 14.8 | Native keyring and production probes | Pass |
| 14.9 | Login/logout dispatcher and persistence | Pass |
| 14.10 | Live auth and session interaction | Pass |
| 14.11 | Factory-built Anthropic cache markers | Pass |
| 14.12 | Usage and cost contract | Pass-with-findings |
| 14.13 | Documentation, verification, and residual closure | Pass |
| 14.14 | Native keyring host selection | Pass |
| 14.15 | WireApi, model metadata, pricing, thinking, and canonical IDs | Pass |
| 14.16 | ApiMappedProvider and TOML custom providers | Pass |
| 14.17 | GitHub Copilot three-wire catalog | Pass |
| 14.18 | OpenAI Codex dedicated wire, catalog, and dual login | Pass |
| 14.19 | Concrete OAuth dispatcher vertical path | Pass |
| 14.20 | Outer TUI credential retry | Pass |
| 14.21 | Documentation, acceptance artifacts, and Phase F | Pass |

---

## 2. Correctness Findings

### 2.1 MAJOR: CumulativeUsage::as_usage() silently truncates u64 to u32

**File:** `crates/opi-ai/src/stream.rs`
**Lines:** 201--222
**Cause:** `CumulativeUsage` accumulates tokens as `u64` (via `saturating_add`),
but `as_usage()` converts back to `Usage` whose parent bucket fields are `u32`,
using bare `as u32` casts. For long-running sessions exceeding 4.29B tokens in
any single bucket, the conversion silently wraps/truncates.
**Impact:** Incorrect usage display, incorrect cost calculation, and misleading
session summaries for extremely long sessions. The u32 limit (~4.29B tokens)
means this would require thousands of max-output turns, so it is unlikely in
current practice but architecturally unsound given `CumulativeUsage` explicitly
chose u64 for accumulation safety.
**Fix:** Either make `Usage` parent fields `u64`, or clamp with
`u32::try_from(self.input_tokens).unwrap_or(u32::MAX)` and document the ceiling.
The `cache_write_1h_tokens` and `reasoning_tokens` fields already use `Option<u64>`
consistently through both types.

### 2.2 MINOR: percent_decode returns empty string on invalid UTF-8

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** 463--479
**Cause:** `String::from_utf8(out).unwrap_or_default()` silently produces an
empty string when decoded bytes are not valid UTF-8. The parsed `state` could
then become `""`, causing a false mismatch error (reported as "state mismatch")
rather than a specific parse error.
**Impact:** Low -- OAuth callbacks from conforming servers always produce valid
UTF-8. But a corrupted callback or malicious redirect produces a misleading
error message rather than a descriptive "invalid UTF-8 in callback" diagnostic.
**Fix:** Return a specific error variant or use `from_utf8_lossy` with a logged
warning when the original bytes are not valid UTF-8.

### 2.3 MINOR: encode_credential panics on serialization failure

**File:** `crates/opi-coding-agent/src/credential_store.rs`
**Lines:** 441
**Cause:** `serde_json::to_string(&envelope).expect("credential envelope serializes")`
will panic if serialization fails. While the `Envelope` struct contains only
`String`/`Option<String>`/`u32` fields (making failure essentially impossible),
the `.expect()` on a non-test path is inconsistent with the crate's error-handling
style.
**Impact:** Theoretical panic in production if the type ever gains a
non-serializable field. Practically zero risk given the current field types.
**Fix:** Replace with `.map_err(|e| CredentialStoreError::Backend(e.to_string()))?`
for consistency, or add a code comment documenting why panic is acceptable here.

### 2.4 INFO: expires_at=None causes every stream to hit refresh slow path

**File:** `crates/opi-ai/src/auth.rs`
**Lines:** 157--162
**Cause:** `OAuthCredential::needs_refresh()` returns `true` when `expires_at`
is `None`. If a provider's token response omits an expiry (which GitHub Copilot
does for its long-lived PAT-style token stored as refresh), every stream()
call enters the lock-acquire + re-read refresh slow path.
**Impact:** Performance: unnecessary lock contention and double-check reads on
every request for providers without explicit expiry. The double-check logic
exits quickly when the stored token is unchanged, so correctness is preserved.
**Fix:** Consider distinguishing "never expires" from "unknown expiry" with an
explicit enum, or caching the last-resolved credential to avoid repeated lock
acquisition when the token has not changed.

---

## 3. Security / Redaction Findings

### 3.1 MINOR: SecretKey uses plain String without zeroize

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 56--85
**Cause:** The legacy `SecretKey` type wraps a plain `String`. Its `Debug` and
`Display` impls correctly redact the value, but the inner `String` is not
zeroized on drop. The newer credential path correctly uses
`secrecy::SecretString` (which zeroizes). Both types coexist: `StaticApiKey`
uses `SecretKey`; `StoreCredential`/`OAuthToken` use `SecretString`.
**Impact:** API keys held via the `StaticApiKey` path remain in freed memory
until overwritten. This is a defense-in-depth gap rather than an exploitable
vulnerability -- an attacker with memory-read capability already has full
process access.
**Fix:** The spec explicitly defers the end-to-end `SecretString` migration as
a Non-Goal. Document this as a known limitation for the `StaticApiKey` path.
No immediate action required within Phase 14 scope.

### 3.2 MINOR: await_manual_code() returns plain String

**File:** `crates/opi-ai/src/auth.rs`
**Lines:** 271
**Cause:** `LoginPresenter::await_manual_code()` returns `String`, not
`SecretString`. The returned value is an OAuth authorization code (short-lived,
single-use) that is immediately exchanged for tokens.
**Impact:** Minimal -- authorization codes are single-use and expire within
seconds. The plain String is consumed immediately in the token exchange and
then goes out of scope. Not a practical exploit path.
**Fix:** No action needed. The authorization code's lifetime is bounded by
immediate consumption.

### 3.3 INFO: Loopback callback does not validate Host/Origin

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** 314--364
**Cause:** The `accept_one_callback` function binds to `127.0.0.1:0` and
accepts the first connection without checking Host or Origin headers. A local
malicious process could connect and either inject a crafted response or race
to receive the callback.
**Impact:** Low -- binding to `127.0.0.1` restricts to localhost; the PKCE
`state` parameter validates that the callback originated from the expected
authorization flow; the callback listener shuts down after one connection.
An attacker would need to both race the legitimate callback AND know the
random `state` value.
**Fix:** No action needed given the existing `state` validation. This is
standard practice for local OAuth callback handlers.

---

## 4. Test Quality Findings

### 4.1 MINOR: PKCE cancellation flow untested for Anthropic and Codex Browser

**File:** `crates/opi-coding-agent/tests/oauth_auth.rs`
**Lines:** (not present)
**Cause:** Only Codex device-code has a dedicated cancellation test
(`openai_codex_device_code_cancellation_writes_nothing`). The PKCE browser
flow's `await_login_cancelled` path is not exercised for Anthropic or Codex
Browser login methods.
**Impact:** If the cancellation select! branch has a resource leak or fails to
clean up the loopback listener, it would not be caught by tests. The
`LoginTerminalGuard` Drop impl provides defense-in-depth, but the actual
cancel-during-PKCE path is unverified.
**Fix:** Add one test per PKCE provider that triggers `await_login_cancelled`
during the select! race and asserts: no credential written, loopback port
released, presenter notified of failure.

### 4.2 MINOR: Usage subset violation untested for secondary providers

**File:** `crates/opi-ai/tests/` (Bedrock, Gemini, Mistral, Vertex fixtures)
**Lines:** (not present)
**Cause:** Child-greater-than-parent rejection is tested only for Anthropic
(`cache_write_1h > cache_write`) and OpenAI Chat/Responses (`reasoning > output`).
Secondary providers (Bedrock, Gemini, Mistral, Vertex) pass `None` for these
optional fields today, but if they ever add subset mappings, no negative test
would catch a missing validation.
**Impact:** Low immediate risk -- these providers don't map the subset fields
today. The gap is a regression safety concern for future provider evolution.
**Fix:** Add one parametric negative test that constructs a Usage with
`cache_write_1h > cache_write` (or `reasoning > output`) and asserts the
validation logic rejects it, independent of provider mapper.

### 4.3 INFO: Cost calculation has no overflow/precision boundary test

**File:** `crates/opi-ai/tests/usage_cost.rs`
**Lines:** (not present)
**Cause:** No test exercises `calculate_cost` with token counts near u32::MAX
or values that would produce f64 precision loss or infinity.
**Impact:** Theoretical -- current real-world token counts are orders of
magnitude below overflow thresholds. But documenting the boundary behavior
(or adding a guard) would prevent silent cost miscalculation if the system
scales to very large batch jobs.
**Fix:** Consider adding one boundary test with u32::MAX tokens and asserting
the result is finite and non-negative.

---

## 5. Spec Compliance Findings

### 5.1 MAJOR: Module comment contradicts OAuth implementation status

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 25--27, 94--97
**Cause:** The module-level doc comment (L25) says `# Future OAuth (not
implemented)` and the `AuthDescriptor` doc (L96--97) says `OAuth is an explicit
future extension point and is not implemented in Phase 10.` However,
`AuthDescriptor::StoreCredential` IS implemented and OAuth flows ARE operational
in Phase 14.
**Impact:** Developer confusion. A new contributor reading the module docs would
conclude OAuth is unimplemented, contradicting the actual shipped behavior.
This is a spec compliance issue because Task 14.7 and 14.13/14.21 require
documentation to describe the final code exactly.
**Fix:** Update the module comment to describe the current three-variant
credential model (static, env, store) and remove the "not implemented" / "future
extension" language. Retain the `#[non_exhaustive]` rationale.

### 5.2 INFO: StoreCredential dispatch_stream without injected probe is non-gating

**File:** `crates/opi-ai/src/provider_collection.rs`
**Lines:** 401--409
**Cause:** When `ProviderCollection::dispatch_stream` encounters a
`StoreCredential` descriptor but no probe has been injected via `set_probe()`,
the dispatch proceeds without gating. The design relies on the factory always
calling `set_probe()` before any dispatch.
**Impact:** If a code path constructs a `ProviderCollection` with
`StoreCredential` auth but forgets to inject probes, the stream would proceed
and fail later at `AuthResolver::resolve()` -- a delayed rather than
immediate error. Tests verify the factory path always injects probes, so the
production path is covered.
**Fix:** No immediate fix needed. The factory-always-injects invariant is
tested. Consider adding a `debug_assert!` at the dispatch site if defense-in-depth
is desired.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| opi-agent does not construct providers or own auth config | `agent_loop.rs:118` calls `context.provider.stream(request)` only; no provider construction in opi-agent | `agent_loop_mock.rs` uses injected MockProvider |
| No opi-managed plaintext credential file | Keychain envelope stored via keyring-core; lock file contains no secret | `credential_store.rs` temp-root scan tests assert no plaintext artifact |
| No auto-relogin mid-stream | `CredentialRevoked` is non-retryable; outer TUI retries only after explicit `/login` success | `interactive_tui_auth.rs` mid-stream Revoked tests |
| Marker/credential writes require lock | `write()` and `delete()` both call `self.lock.acquire()` before unlocked ops | `mutation_lock_serializes_concurrent_writers` |
| OAuth refresh non-reentrant | Refresh uses `write_unlocked` inside already-held lock | `concurrent_near_expiry_resolves_coalesce_to_single_refresh` |
| Backend errors distinct from missing | `CredentialStoreError` variants: `BackendUnavailable`, `Backend`, `MalformedEnvelope` etc. never collapse to `None` | `credential_store.rs` L851--886 and integration tests |
| Env fallback only for absence or BackendUnavailable | `resolve_api_key` falls back only on `None` marker or `BackendUnavailable`; operational/corrupt/unknown errors do not fall through | `operational_backend_error_does_not_fall_back_to_env` and siblings |
| Session affinity propagates through resume/fork | `sync_session_id()` called on new/resume/fork; fork generates new header.id | `phase14_session_affinity_tracks_new_resume_and_fork` |

---

## 7. Cross-Task Integration Findings

### 7.1 MINOR: SecretKey vs SecretString dual-path inconsistency

**File:** `crates/opi-ai/src/provider_collection.rs` (SecretKey L56--85) vs
`crates/opi-ai/src/credential.rs` (SecretString usage)
**Cause:** Two parallel secret-holding types coexist: `SecretKey` (plain String,
no zeroize) for `StaticApiKey`, and `SecretString` (secrecy crate, zeroize on
drop) for `Credential`/`ResolvedAuth`/`OAuthCredential`. Both redact in
Debug/Display but differ in drop behavior.
**Impact:** Inconsistent security posture between legacy and new paths. The spec
explicitly defers unification as a Non-Goal.
**Fix:** No Phase 14 action. Track as a follow-up for the deferred end-to-end
SecretString migration.

### 7.2 INFO: Bedrock CredentialSource naming collision

**File:** `crates/opi-ai/src/bedrock/credentials.rs` vs
`crates/opi-ai/src/credential.rs`
**Cause:** Bedrock defines its own `CredentialSource` enum (AWS credential chain)
in the same crate as the Phase 14 `CredentialSource` (three-state probe result).
They are in different modules and serve different domains.
**Impact:** Potential developer confusion. No functional conflict since Rust's
module system disambiguates. Both are used in their respective modules without
cross-import.
**Fix:** No action needed. The types are domain-separated and never interact.

### 7.3 INFO: cache_retention not propagated from harness to agent loop

**File:** `crates/opi-agent/src/agent_loop.rs`
**Lines:** 101--103
**Cause:** The agent loop hardcodes `cache_retention: CacheRetention::None` in
every Request. The `CacheRetention` type exists and OpenAI Responses providers
check `cache_retention != Disabled` before emitting `prompt_cache_key`, but the
production agent loop never sets a non-None value.
**Impact:** None for Phase 14 -- the spec explicitly states "Phase 14 adds no
config key or CodingHarness producer for the first three fields [timeout,
extra_headers, cache_retention]." These are substrate-only, proven by direct
Request-to-provider captures.
**Fix:** No action needed. This is by-design substrate that will gain a
production producer in a future phase.

---

## 8. Residuals and Recommendations

### Priority recommendations

1. **[Major 2.1]** Fix the u64-to-u32 truncation in `CumulativeUsage::as_usage()`.
   Either widen `Usage` parent fields to `u64` (breaking change tracked in
   CHANGELOG) or add saturating conversion with a ceiling constant.

2. **[Major 5.1]** Update `provider_collection.rs` module doc comment to reflect
   the implemented OAuth status. Remove "Future OAuth (not implemented)" and
   "not implemented in Phase 10" language.

3. **[Minor 4.1]** Add PKCE cancellation tests for Anthropic and Codex Browser
   flows to close the cancel-during-login coverage gap.

### Deferred items (by-design, not action items)

- `api-map` recorded as deferred-by-updated-design with reviewed citation and
  two-wire-family trigger (per evaluator summary).
- End-to-end `SecretString` migration through provider construction deferred
  per spec Non-Goal.
- Production `refresh_models` trigger: substrate-only, no runtime caller by
  design.
- `timeout`, `extra_headers`, `cache_retention` production producers: substrate
  proven, harness wiring deferred to future phase.

### Observations for future phases

- The `NativeKeyringGuard` reference-counting pattern works but adds complexity
  compared to a single process-global initialization. If the guard count ever
  drifts (e.g., due to panics in async contexts), the debug_assert at L37 of
  `native_keyring.rs` would only fire in debug builds.
- The OAuth registry is rebuilt on every `/login`/`/logout` command dispatch.
  This is stateless and correct, but a cached registry could avoid repeated
  endpoint config construction.
- The marker-then-credential dual-write in `KeychainCredentialStore` is not
  atomic. If the second write fails, a stale marker remains. The test suite
  covers this scenario, but no automatic recovery or consistency check exists.
