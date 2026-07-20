# Phase 14 Provider & Auth -- Independent Code Audit

**Auditor**: opus4.6 (independent, no prior audit reports consulted)
**Date**: 2026-07-20
**Scope**: Tasks 14.1--14.21, commits `d9f21a9..8364e74`
**Method**: Full-source deep read via six parallel agents (opi-ai sources, opi-ai
tests, opi-coding-agent sources, opi-coding-agent tests, opi-agent changes,
docs/config), followed by targeted verification of specific findings against
source code. Seven audit dimensions applied: correctness, security/redaction,
test quality, spec compliance, invariants, cross-task integration, residuals.

---

## 1. Executive Summary

**Verdict: PASS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 0     |
| Minor    | 2     |
| Info     | 4     |

Phase 14 is a large, security-sensitive phase (21 tasks, 3 crates, credential
storage, OAuth flows, per-request auth, multi-wire provider routing, Usage/Cost
accounting). The implementation is thorough and closely follows the spec through
three corrective iterations (14.1-14.7, 14.8-14.13, 14.14-14.21). Test coverage
is extensive (100+ OAuth tests, 12 outer-TUI auth tests, comprehensive factory
and doctor tests) with strong assertion quality including secret-canary leak
detection, wiremock HTTP capture, and error-variant pattern matching. All eight
success criteria (SC1-SC8) are met, all Non-Goals are preserved, and the
load-bearing construction-ownership invariant is intact.

### Per-task summary

| Task  | Title                                               | Verdict |
|-------|-----------------------------------------------------|---------|
| 14.1  | Credential store model                              | PASS    |
| 14.2  | OAuth architecture and per-request auth re-resolution | PASS |
| 14.3  | Request scalars and session-affinity production path | PASS    |
| 14.4  | Model capabilities and Anthropic cache markers      | PASS    |
| 14.5  | Usage and cost cache/reasoning accounting           | PASS    |
| 14.6  | Dynamic provider model refresh                      | PASS    |
| 14.7  | Provider/auth docs, non-goal guards, and final gates | PASS   |
| 14.8  | Native keyring and production probes                | PASS    |
| 14.9  | Login/logout dispatcher and persistence             | PASS    |
| 14.10 | Live auth and session interaction                   | PASS    |
| 14.11 | Factory-built Anthropic cache markers               | PASS    |
| 14.12 | Usage and cost contract                             | PASS    |
| 14.13 | Documentation, verification, and residual closure   | PASS    |
| 14.14 | Native keyring host selection                       | PASS    |
| 14.15 | WireApi, model metadata, pricing, thinking, IDs     | PASS    |
| 14.16 | ApiMappedProvider and TOML custom providers          | PASS    |
| 14.17 | GitHub Copilot three-wire catalog                   | PASS    |
| 14.18 | OpenAI Codex dedicated wire, catalog, and dual login | PASS   |
| 14.19 | Concrete OAuth dispatcher vertical path             | PASS    |
| 14.20 | Outer TUI credential retry                          | PASS    |
| 14.21 | Documentation, acceptance artifacts, and Phase F    | PASS    |

---

## 2. Correctness Findings

### 2.1 INFO: Usage parent/child type asymmetry (u32 vs Option<u64>)

**File:** `crates/opi-ai/src/stream.rs`
**Lines:** 38--52
**Cause:** Parent token buckets (`input_tokens`, `output_tokens`,
`cache_read_tokens`, `cache_write_tokens`) remain `u32`, while new child
subsets (`cache_write_1h_tokens`, `reasoning_tokens`) are `Option<u64>`.
Subset validation (`cache_write_1h > cache_write_tokens`) widens the parent
to `u64` for comparison, so correctness is preserved. `CumulativeUsage`
tracks all buckets as internal `u64` with `saturating_add`, preventing
accumulation overflow.
**Impact:** None in practice. The theoretical edge case: `as_usage()`
truncates the parent to `u32` via `saturating_usage_bucket`, then clamps
the `u64` child to the truncated parent via `.min()`. If cumulative parent
exceeds `u32::MAX` over many turns, child precision could silently decrease.
No LLM response produces anywhere near 4 billion tokens per turn, so this
is academic.
**Fix:** No fix needed. The type asymmetry is an intentional scope cap --
the spec changes only the new child fields to `Option<u64>`, not the
existing parent fields. A future full-`u64` migration would eliminate the
asymmetry.

### 2.2 INFO: CumulativeUsage child-to-parent clamping on as_usage()

**File:** `crates/opi-ai/src/stream.rs`
**Lines:** 226--231
**Cause:** `as_usage()` produces a `Usage` where each child is clamped to
its parent: `self.cache_write_1h_tokens.map(|t| t.min(u64::from(cache_write_tokens)))`.
This is defensive -- after many turns of `saturating_add`, the `u64` child
total could exceed the `u32`-saturated parent in the output `Usage`, so
clamping restores the subset invariant.
**Impact:** Correct defensive behavior. The clamped value may undercount
the true 1h or reasoning total in the (impossible) `u32::MAX` saturation
scenario. Tests cover valid subset semantics through the accumulation path.

---

## 3. Security / Redaction Findings

### 3.1 MINOR: session_id used as HTTP header without HeaderValue validation

**File:** `crates/opi-ai/src/openai_responses.rs`
**Lines:** 385--388
**File:** `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** 171
**Cause:** Both Responses and Codex Responses paths pass `session_id`
directly to reqwest's `.header("session_id", session_id)` without prior
`HeaderValue::from_str()` validation. In contrast, the OpenAI Chat path
constructs session affinity headers into a `Vec<(String, String)>` that
flows through `ProviderHeaders::merge_request()`, which validates via
`HeaderValue::from_str()`.
**Impact:** If a session_id containing non-visible ASCII or control
characters reaches these paths, reqwest would panic rather than returning
a typed `ProviderError::RequestFailed`. Low practical risk because
session_ids originate from `SessionCoordinator` (UUID-based), but
inconsistent with the spec's statement that "Header values are constructed
with `HeaderValue::from_str`; invalid values return
`ProviderError::RequestFailed` and never panic."
**Fix:** Validate `session_id` through `HeaderValue::from_str()` before
passing to reqwest in both `openai_responses.rs` and
`openai_codex_responses.rs`, returning `ProviderError::RequestFailed` on
failure. This aligns all three OpenAI paths.

---

## 4. Test Quality Findings

### 4.1 MINOR: Incomplete extra_headers reserved-name rejection test coverage

**File:** `crates/opi-ai/tests/` (extra_headers test modules)
**Lines:** (various)
**Cause:** The `validate_extra_headers` function rejects five reserved
header names (`authorization`, `x-api-key`, `api-key`,
`anthropic-version`, `content-type`). Tests exercise rejection for a
subset of these names but do not individually cover all five.
**Impact:** Low -- the rejection logic is a single `RESERVED.contains()`
check, so covering one name validates the mechanism. However, a regression
that accidentally removes a name from the constant would not be caught for
the untested entries.
**Fix:** Add test cases for all five reserved names. This is a one-line-
per-name extension of existing test helpers.

### 4.2 INFO: Codex provider catalog test lacks HTTP routing verification

**File:** `crates/opi-coding-agent/tests/openai_codex_provider.rs`
**Lines:** 1--168
**Cause:** The test verifies the pi-0.80.6 catalog fixture (7 models,
capabilities, pricing tiers) but does not perform wiremock HTTP routing to
confirm the dedicated `/codex/responses` endpoint, headers, or body shape.
**Impact:** None -- HTTP routing for the Codex wire is verified in
`provider_factory.rs` tests (9-family wire route coverage) and the
`custom_provider_map.rs` auth-rotation tests. The catalog test's scope is
appropriately narrow.

---

## 5. Spec Compliance Findings

No deviations found. All eight success criteria (SC1-SC8) are traced to
production-path evidence. All corrective obligations (F14-01 through F14-04,
api-map, B01-B20) are met. All eight Non-Goals remain preserved. Provider ids
are canonical (`github-copilot`, `openai-codex`) with no legacy alias.
`api-map` is `implemented` with task, fixture, and test citations.

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| opi-agent must not construct providers or own auth config | `agent.rs` receives `Box<dyn Provider>`, propagates credential errors; no provider construction or auth config anywhere in opi-agent | `session_id_reaches_every_request` verifies opaque propagation; load-bearing invariant preserved |
| Usage child is subset of parent | `anthropic.rs:181` validates `cache_write_1h <= cache_write`; `openai_chat.rs` and `openai_responses.rs` validate `reasoning_tokens <= output_tokens`; violations return `StreamError` | Provider-level validation tests cover zero, equality, absence, and exceeding cases |
| Credential store mutation uses LockCoordinator | `credential_store.rs` `LockCoordinator` wraps all `write`/`delete`; OAuth refresh holds lock across read-HTTP-write; `login_oauth`/`logout_credential` use the same coordinator | `mutation_lock_serializes_concurrent_writers` uses Arc-shared backend + timing assertions; lock timeout test covers contention |
| Non-reentrant refresh lock | `CredentialResolver` refresh path acquires the public lock once; internal `write_unlocked` / `delete_unlocked` methods bypass re-acquisition | `oauth_auth.rs` refresh tests verify lock-during-HTTP, post-failure re-read, and no partial write |
| No plaintext credential file | `KeychainCredentialStore` writes only to OS keychain; only `credential.lock` exists on disk | `redaction_invariant` tests recursively scan temp-root + stdout/stderr/diagnostics/formatted errors for seeded secret canaries |
| Reserved auth headers rejected | `validate_extra_headers` rejects 5 reserved names before provider construction | Extra-headers validation tests; `ProviderHeaders::merge_request` rejects per-provider managed headers |
| CredentialNeeded retry is single-shot | `PromptAuthStateMachine` `may_arm_retry` flag; `output_began` AtomicBool guard | 12 outer-TUI tests cover success retry, all negative gates, midstream revocation, and repeat-credential-needed guard |

---

## 7. Cross-task Integration Findings

No issues found. The 21-task integration across three crates is well-
coordinated:

- **Shared test infrastructure**: `common/mod.rs` and
  `common/phase14_auth_runtime.rs` provide reusable mock presenters,
  credential runners, and subprocess capture utilities.
- **Centralized construction**: `provider_factory.rs` is the single entry
  point for all provider construction. The `provider_policy_is_centralized`
  test scans source tokens to enforce this.
- **Consistent auth source model**: `AuthSource` (Baked/Store/EnvOAuthToken/
  Layered) bridges `credential_store.rs` ↔ `provider_factory.rs` ↔ concrete
  providers cleanly.
- **pi-0.80.6 fixture provenance**: GitHub Copilot (25 models) and OpenAI
  Codex (7 models) catalogs use checked-in fixtures with version/SHA-256
  provenance, compared field-by-field in tests.

---

## 8. Residuals and Recommendations

### 8.1 INFO: refresh_models substrate has no production trigger

**Cause:** `Provider::refresh_models()` defaults to `Ok(None)`.
`ProviderCollection::refresh()` implements atomic replacement. No CLI,
doctor, RPC, TUI, or startup path invokes refresh in Phase 14.
**Impact:** None -- this is explicitly substrate-only per SC7. The design
is sound (deterministic ordering, atomic batch, error leaves prior state).
A future phase must add a real dynamic provider and trigger before claiming
runtime refresh.

### Priority recommendations

1. Validate `session_id` through `HeaderValue::from_str` in `openai_responses.rs`
   and `openai_codex_responses.rs` to align with the OpenAI Chat path and
   eliminate the (theoretical) panic-on-invalid-value gap (finding 3.1).
2. Extend extra_headers reserved-name rejection tests to cover all five names
   individually (finding 4.1).
3. No code changes needed for the Info findings; they are design observations
   for future consideration.
