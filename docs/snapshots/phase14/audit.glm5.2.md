# Phase 14 Provider & Auth -- Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports consulted)
**Date**: 2026-07-22
**Scope**: Tasks 14.1--14.21, commits `079b5d2..3ef05d1` (HEAD). Spans the
original T1--T3 implementation, the 14.8--14.13 exit remediation, the
14.14--14.21 pi-0.80.6 alignment revision, and five post-phase-exit audit
remediation commits (`9263114`, `b27905a`, `47400ee`, `560d8ed`, plus their
doc refreshes). Phase exit recorded at `8364e74` (2026-07-19); the audit target
is the current post-remediation HEAD.
**Spec sources**:
`docs/opi-spec.md`,
`docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`,
`docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.
**Method**: Six parallel fresh-eyes deep-read passes (general-purpose subagents),
each fully reading one subsystem's source + tests against the spec DoDs and
invariants, plus lead-auditor independent re-verification of every
contamination-sensitive area and a targeted test run at HEAD.

---

## 0. Contamination disclosure

This auditor's auto-loaded session memory carried a summary of *prior* Phase 14
audits performed at earlier HEADs, naming a **Blocker C-2.1** (Responses/Codex
SSE decoder broken on real data-only wire, masked by synthetic `event:` test
fixtures) and a **Major C-3.2** (`AccountIdMissing` typed-auth loss). This was
disclosed up front and treated as contamination. To protect independence:

- No `docs/snapshots/phase14/audit.*.md`, `remediation-plan.md`, or
  `target/opi-artifacts/` evaluator/phase-exit content was read during analysis.
  Two subagents reported incidental grep line-number hits in those files during
  broad searches; neither opened or relied on them. The prior
  `audit.glm5.2.md` was touched only as a mechanical overwrite precondition
  after all findings were independently finalized.
- Every contamination-flagged area was re-derived from source and re-verified by
  the lead auditor independently (see section 4 and the invariant matrix).

**Outcome of re-verification at the audited HEAD (`3ef05d1`)**: both C-2.1 and
C-3.2 are **resolved**. The remediation is real and is confirmed by fresh eyes
reading current code, not by trusting prior verdicts. Details in findings 4.1
and 4.2.

---

## 1. Executive Summary

**Verdict: PASS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 0     |
| Minor    | 7     |
| Info     | 9     |

Phase 14 is substantially complete and correct at the audited HEAD. The
credential-store / OAuth / auth / live-auth / request / cache-marker /
usage-cost / wire / mapped-provider subsystems all trace cleanly to their spec
DoDs and invariants, with strong, non-vacuous, production-path test evidence
(real `CodingHarness` session-id tracing through create/resume/fork;
factory-built wiremock stream captures with exact marker position/TTL
assertions; offline provenance-pinned catalog fixtures; end-to-end usage-subset
persistence/resume round-trips). Redaction is verified by seeded-canary absence
scans, not absence-of-panic. No defect rises above Minor. The two highest-impact
issues from prior audits are independently confirmed fixed.

The top residual concerns are a **test-coverage blind spot that is the same
class as the original C-2.1 masking** (the *standard* Responses lifecycle tests
still feed synthetic `event:`+`data:` pairs rather than realistic data-only
wire -- finding 5.1), and a small set of defense-in-depth gaps and cross-module
inconsistencies in the Codex wire (2.1, 2.2).

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1  | Credential store model                          | PASS |
| 14.2  | OAuth architecture & per-request auth re-resolve| PASS |
| 14.3  | Request scalars & session-affinity path         | PASS (Info 6.1, 6.2) |
| 14.4  | Model capabilities & Anthropic cache markers    | PASS |
| 14.5  | Usage & cost cache/reasoning accounting         | PASS |
| 14.6  | Dynamic provider model refresh (substrate)      | PASS |
| 14.7  | Provider/auth docs, non-goal guards, gates      | PASS |
| 14.8  | Native keyring & production probes              | PASS (Minor 3.2) |
| 14.9  | Login/logout dispatcher & persistence           | PASS |
| 14.10 | Live auth & session interaction                 | PASS |
| 14.11 | Factory-built Anthropic cache markers           | PASS |
| 14.12 | Usage & cost contract                           | PASS (Minor 2.3) |
| 14.13 | Documentation, verification, residual closure   | PASS |
| 14.14 | Native keyring host selection                   | PASS |
| 14.15 | WireApi, model metadata, canonical IDs          | PASS |
| 14.16 | ApiMappedProvider & TOML custom providers       | PASS |
| 14.17 | GitHub Copilot three-wire catalog               | PASS |
| 14.18 | OpenAI Codex dedicated wire/catalog/dual login  | PASS (Minor 2.1, 2.2) |
| 14.19 | Concrete OAuth dispatcher vertical path         | PASS |
| 14.20 | Outer TUI credential retry                      | PASS |
| 14.21 | Documentation, acceptance artifacts, Phase F    | PASS |

---

## 2. Correctness findings

### 2.1 Minor: Codex `build_request_body` strips any `provider:model` prefix while `stream()` strips only `openai-codex:`

**File:** `crates/opi-ai/src/openai_codex_responses.rs:67-72` vs `:277-281`
**Cause:** `build_request_body` computes `model_id` via
`request.model.split_once(':').map(|(_, id)| id).unwrap_or(&request.model)`,
stripping any prefix before the first colon. `stream()` was hardened (the `C9`
comment) to strip *only* `openai-codex:` via `strip_prefix`. The two paths now
disagree.
**Impact:** Harmless on the production path today: `stream()` resolves
`model_id` with the canonical strip, runs the `model_known` guard, and returns
`UnknownModel` before HTTP for any foreign spec, discarding the body. But if the
`model_known` guard were ever removed or reordered, a spec like `acme:gpt-5.4`
would send a body whose `"model"` field is `"gpt-5.4"` while the provider
believed it was serving `acme:gpt-5.4` -- a silent wrong-model wire call. The
fix applied to `stream()` was not propagated to the body builder.
**Fix:** Make `build_request_body` use the same `strip_prefix("openai-codex:")`
pattern, or have `stream()` pass the already-resolved model id into the body
builder rather than re-deriving it.

### 2.2 Minor: Codex `build_request_body` thinking-level fallback bypasses the model thinking map

**File:** `crates/opi-ai/src/openai_codex_responses.rs:111-123`
**Cause:** After `model.thinking_level_map.resolve(request.thinking.level)`, the
Codex body builder appends
`.or_else(|| request.thinking.level.wire_name().map(str::to_owned))`, so when
the model's map rejects the level it falls back to the raw requested level's
wire name. The standard Responses provider (`openai_responses.rs:321-326`) does
not have this fallback.
**Impact:** Unreachable through the production path:
`validate_request_capabilities` runs first in `stream()` (`:271`) and rejects
unsupported levels before the body is built. But `build_request_body` is `pub`,
so a future caller invoking it directly (without the preflight) would silently
emit a `reasoning` field the upstream model map said to reject. Defense-in-depth
asymmetry vs standard Responses.
**Fix:** Remove the `.or_else(...)` fallback to mirror standard Responses, or
document why Codex intentionally permits raw level names.

### 2.3 Minor: Strict-subset usage error collapsed to a generic "malformed SSE frame" string in OpenAI Chat/Responses

**File:** `crates/opi-ai/src/openai_chat.rs` (stream/HTTP malformed arms);
`crates/opi-ai/src/openai_responses.rs:339-345, 448-453`
**Cause:** When `validate_usage_subset`/`parse_response_usage` trips the
strict-subset invariant, the rich error string (e.g.
`"reasoning_tokens (800) exceeds completion_tokens (500)"`) is collapsed to the
hardcoded `ProviderError::StreamError("malformed OpenAI Chat/Responses SSE
frame")`. Anthropic preserves the detailed message via `ParsedEvent::UsageError`.
**Impact:** No contract violation: the error is still `StreamError`,
non-retryable, and no invalid `Usage` event is emitted. The locally-computed
subset message contains no upstream-secret fragment, so surfacing it is safe;
losing it only degrades debuggability. The inconsistency is why tests pass while
asserting only the variant.
**Fix:** In the `ParsedEvent::Malformed { error, .. }` arm, prefer
`ProviderError::StreamError(error)` when `error` carries the locally-generated
subset message, or thread a `ParsedEvent::UsageError(ProviderError)` variant
through Chat/Responses mirroring Anthropic.

---

## 3. Security / redaction findings

### 3.1 Minor: `encode_credential` intermediate envelope fields are not `Zeroizing<String>`

**File:** `crates/opi-coding-agent/src/credential_store.rs:428-442, 527, 540-541, 559-561`
**Cause:** `encode_credential` extracts secrets via `expose_secret().to_owned()`
into plain `Option<String>` fields on `EnvelopeFields`, then manually zeroizes
them after serialization. The serialized envelope is wrapped in
`Zeroizing<String>`, but the intermediate `EnvelopeFields` are not. If a panic
occurs between exposure (`:527/540/541`) and the manual zeroize (`:559-561`),
the secret-bearing `String`s sit in heap memory until the allocator reclaims
them. The decode side has the same shape (`ApiKeyEnvelopeV1`/`OAuthEnvelopeV1`
hold raw `String`).
**Impact:** Defense-in-depth gap, not an exploitable leak in normal operation.
The window is microseconds, but it expands the secret's in-memory footprint
beyond what the spec promises ("serialized JSON *and intermediate envelope
fields* are zeroized after the backend call"). `expose_secret()` is otherwise
correctly confined to this serialization boundary and the HTTP boundary.
**Fix:** Change `EnvelopeFields::{api_key, access, refresh}` (and the decode
envelope structs) from `Option<String>` to `Option<Zeroizing<String>>`, or build
the JSON field-by-field through a `Zeroizing<String>` buffer. (Note: serde does
not serialize `Zeroizing<String>` transparently, so a custom serializer is
needed.)

### 3.2 Minor: Linux Secret Service daemon-absence detection is heuristic substring matching

**File:** `crates/opi-coding-agent/src/credential_store.rs:269-274`
**Cause:** `secret_service_is_unavailable` classifies a missing DBus daemon by
matching lowercased substrings `"serviceunknown"`, `"namehasnoowner"`,
`"connection refused"`. The classification is correct and deliberately narrow
(`"secrets"` alone is *not* treated as absence, and it is pinned by tests
`explicit_secret_service_absence_signatures_are_unavailable`,
`secret_service_name_alone_never_means_daemon_unavailable`), but it couples the
headless API-key fallback path to the wording emitted by
`zbus-secret-service-keyring-store`.
**Impact:** An upstream wording change in a future `zbus`/secret-service
release could silently misclassify a real daemon-absence as
`BackendError::Other` (fail-closed) instead of `BackendUnavailable` (env
fallback), breaking headless API-key login. The dep is currently pinned (`=
"1.0"`), which mitigates but does not structurally prevent this.
**Fix:** Broaden the signature set, or version-couple the matcher to the pinned
dep and re-evaluate on every bump.

### 3.3 Info: `present_provider_error` distinguishes credential-store errors by string prefix

**File:** `crates/opi-coding-agent/src/interactive_auth.rs:282-284`
**Cause:** Credential-store `Config` errors are told apart from generic `Config`
errors via `message.starts_with("credential store")`.
**Impact:** Cosmetic -- a future wording change in `credential_store.rs` would
produce the wrong user-facing label, not a security or correctness issue.
**Fix:** Add a typed `ProviderError::CredentialStore` variant or carry
`CredentialStoreError` through instead of flattening into `Config(String)`.

---

## 4. Previously-flagged issues -- independent re-verification

### 4.1 RESOLVED (was C-2.1): Codex/Responses SSE decoder on real data-only wire

**Files:** `crates/opi-ai/src/openai_responses_shared.rs:209-244, 801-811`;
`crates/opi-ai/src/openai_codex_responses.rs:250-256`
**Independent verification:** `ResponsesEvent::try_from_frame` handles the
`[DONE]` sentinel as a no-op (`:214-216`) and resolves the event name by
preferring the JSON `data.r#type` field, falling back to the SSE `event:` name
only when no JSON type is present (`:231-235`, with an explicit comment that
canonical Responses/Codex SSE is data-only). `parse_sse_frames` defaults an
absent `event:` line to `"message"` (never a real Responses event, so typeless
data-only frames fall through to the ignore arm). `drain_sse_frames` retains the
trailing partial frame across reads (`\n\n`-delimited drain). The post-stream
`if !mapper.saw_done` guard emits a typed `"stream ended without a terminal
event"` error. The regression is pinned by
`dedicated_codex_data_only_frames_stream_to_completion` and two sibling
data-only tests; a targeted run at HEAD (`cargo test -p opi-ai --test
openai_codex_responses --test openai_responses_lifecycle`) returned 16/16 green.
The remaining residual is the *standard*-Responses test fixture shape -- see
5.1.

### 4.2 RESOLVED (was C-3.2): `AccountIdMissing` typed-auth loss

**Files:** `crates/opi-ai/src/provider.rs:330-345, 378-383, 391-396`;
`crates/opi-ai/src/openai_codex_responses.rs:152-158, 401-403`
**Independent verification:** `ProviderError::AccountIdMissing { provider_id }`
is a distinct variant, grouped with `CredentialNeeded`/`CredentialRevoked` under
`ProviderErrorCategory::Auth` (`:394-396`), and `is_retryable()` returns `true`
only for `RateLimited`/`Timeout`/`Network` (`:378-383`), so it is non-retryable.
It is emitted at the Codex HTTP boundary when the persisted `chatgpt-account-id`
is absent (`:152-158`); a 401/403 maps to `CredentialRevoked` (`:401`). The
variant is threaded through the runner/rpc event emission and the interactive
retry state machine. The only nuance (finding 7.3) is that non-interactive modes
surface it under the `CredentialNeeded` *event type* -- a deliberate choice to
drive the single `/login <provider>` remediation, spec-compliant.

---

## 5. Test-quality findings

### 5.1 Minor: Standard-Responses lifecycle tests use synthetic `event:`+`data:` fixtures, not realistic data-only wire

**Files:** `crates/opi-ai/tests/openai_responses_lifecycle.rs:40-66`;
`crates/opi-ai/tests/openai_responses_fixtures.rs:404-578`
**Cause:** Every standard-Responses fixture pairs an `event: <name>` line with a
`data: {"type":<name>,...}` line. Real OpenAI Responses streams are data-only
(no `event:` line). The shared parser handles data-only correctly (proven by the
Codex `dedicated_codex_data_only_frames_stream_to_completion` test), but these
standard-Responses tests would *still pass* if the JSON-`type`-preference in
`try_from_frame` regressed, because they would fall through to the `event:`
name.
**Impact:** This is precisely the test-shape blind spot that originally masked
the Codex C-2.1 bug. The bug class was fixed and regression-pinned for the Codex
wire, but the same weakness persists for the standard Responses wire. A future
regression in the JSON-type dispatch would be caught by the Codex suite but not
by the standard-Responses suite.
**Fix:** Add at least one data-only HTTP-level fixture (no `event:` lines) to
`openai_responses_lifecycle.rs` or `openai_responses_fixtures.rs`.

### 5.2 Minor: No test exercises partial-chunk SSE buffering across reads

**Files:** `crates/opi-ai/tests/` (no match for chunk-boundary/split-chunk feeds)
**Cause:** `drain_sse_frames` is designed to retain an incomplete trailing frame
in its buffer between reads. No test feeds a stream whose chunks split a frame
mid-`data:` line (e.g. `b"data: {\"type\":"` then
`b"\"response.completed\",...}\n\n"`).
**Impact:** Regression risk: a future refactor that broke buffer retention would
not be caught. The parsing logic is correct; the coverage is missing.
**Fix:** Add a two-chunk test that splits a frame mid-line and asserts the
completed event still fires.

---

## 6. Spec-compliance findings (all defensible implementation evolution)

### 6.1 Info: `Request::cache_retention` is `CacheRetention`, not `Option<CacheRetention>`

**File:** `crates/opi-ai/src/provider.rs:104`
The field is typed `CacheRetention` (non-Option) with a `None` variant meaning
"provider default", rather than the literal `Option<CacheRetention>` in the T3
§3a design text. Functionally identical; `CacheRetention::None` == "provider
default". `Some(Disabled)` semantics are preserved.

### 6.2 Info: `Request::extra_headers` is `Vec<(String,String)>`, not `HeaderMap`

**File:** `crates/opi-ai/src/provider.rs:102`
The literal spec reads `HeaderMap`. The implementation uses
`Vec<(String,String)>` but enforces `HeaderName::from_str`/`HeaderValue::from_str`
at the `validate_extra_headers` boundary (`provider.rs:177-209`), so invalid
values still return `ProviderError::RequestFailed` rather than panicking.
Behaviorally equivalent.

### 6.3 Info: Codex synthesizes affinity-header UUIDs even when `session_id` is empty

**File:** `crates/opi-ai/src/openai_codex_responses.rs:293-308`
The general session-affinity rule says mapping occurs "only when `session_id`
is non-empty". Codex synthesizes a UUID v7 for `session-id` and
`x-client-request-id` when `session_id` is empty (but suppresses both under
`CacheRetention::Disabled`, `C7`). The more-specific Codex wire contract
("emits `session-id` + `x-client-request-id`") and the Codex SSE decoder's
header dependence are the controlling authority, so this is consistent -- but
the divergence from the general rule is worth a cross-reference.

---

## 7. Cross-task integration & residuals

### 7.1 Info: `Usage::reported` does not self-validate the subset invariant

**File:** `crates/opi-ai/src/stream.rs:74-91`
The subset invariant (child buckets are subsets of their parents) is documented
but enforced only in the three provider mappers. Bedrock/Gemini/Copilot-via-
Anthropic always pass `None, None`, so they cannot trip it, and the
mapper-enforced path is sufficient on the production wire. External 0.x
consumers can nonetheless construct a `Usage` that violates the documented
invariant. Optional defense-in-depth: a `Usage::validated` constructor or an
explicit "caller enforces the subset" note.

### 7.2 Info: `CumulativeUsage::as_usage` saturates children to `u32` boundaries

**File:** `crates/opi-ai/src/stream.rs:218-234`
Parent buckets saturate to `u32::MAX` and children are clamped to the saturated
parent, so the public view loses exact child totals above ~4.3B tokens. Cost
calculation is unaffected (it operates on raw `u64`). Behavior is test-pinned as
intended; flagged only because the `as_usage` docstring does not state the
saturation.

### 7.3 Info: `AccountIdMissing` surfaces under the `CredentialNeeded` event type in non-interactive modes

**Files:** `crates/opi-coding-agent/src/runner.rs:706-710`; `crates/opi-coding-agent/src/rpc.rs:1031-1041`
JSON/RPC/text modes emit `type: "CredentialNeeded"` for `AccountIdMissing`,
carrying the AccountIdMissing diagnostic and `/login <provider>` remediation.
This matches the spec's single-remediation contract, but embedders that
classify purely on the JSON `type` field lose the AccountIdMissing distinction.
Documented inline; not a spec violation.

### 7.4 Info: Credential lock path is `<user_config_dir>/credential.lock`, not `<user_config_dir>/opi/credential.lock`

**File:** `crates/opi-coding-agent/src/credential_store.rs:649-652`
opi's `user_config_dir()` already returns the opi-anchored path
(`~/.config/opi` / `%APPDATA%/opi`), so adding another `opi` segment would
double it. The comment documents this; the resulting production path matches
spec intent.

### 7.5 Info: `FakeKeyringBackend::set` uses blocking `thread::sleep` in an async context (test-only)

**File:** `crates/opi-coding-agent/src/credential_store.rs:387`
Test-only; relies on the multi-thread test runtime. No production impact.

---

## 8. Invariant verification

| Invariant | Code evidence | Test coverage |
|-----------|---------------|---------------|
| Construction-ownership split: `opi-agent` constructs no providers, owns no auth config | `agent_loop.rs` calls `context.provider.stream(request)`; all `CredentialStore`/`OAuthProvider`/`AuthResolver`/`AuthSource` impls live in `opi-coding-agent`; abstract types in `opi-ai` | structural guards in `phase14_provider_auth_docs.rs` (no OAuth/credential/token fields in `session.rs` or `provider.rs`) |
| No opi-managed plaintext credential file | only on-disk artifact is the secret-free `credential.lock`; `encode_credential` -> `Zeroizing<String>` -> keychain backend, never disk | `redaction_only_secret_free_lock_exists_outside_fake_keyring`; canary-absence scans |
| Provider-managed auth headers reserved; no per-call override | `RESERVED_PROVIDER_HEADERS` (`provider_headers.rs:14-28`) enforced by `try_new` and `validate_extra_headers`; override -> `RequestFailed` | `provider_headers_reject_all_reserved_configured_and_request_names`; `dedicated_codex_rejects_managed_header_overrides_before_http` |
| Usage strict subsets, no double count, no message-stored cost | `total_tokens` adds parents only; `calculate_cost_totals` charges 1h at 2x input, remainder at cache-write rate, reasoning inclusive; `Cost` absent from messages | `usage_cost.rs`; `!raw.contains("\"cost\"")` resume assertion; subset-preservation tests across all three wires |
| `ApiMappedProvider` construction-time route validation; pre-HTTP typed errors | `try_new` validates catalog/route shape; `stream` -> `UnknownModel`/`MissingWireRoute` before HTTP | `api_mapped_provider.rs` (unknown model, missing route, mismatch, subset/superset) |
| Refresh is substrate-only; no production trigger | `refresh_models` default `Ok(None)`; `ProviderCollection::refresh` atomic replace/rollback | structural guard: no `refresh_models(`/`ProviderCollection::refresh` in `opi-coding-agent/src`; six substrate tests |
| Same-turn retry: exactly one user message, two provider calls, one retry; negatives zero retries | `PromptAuthStateMachine` (`interactive.rs:289-429`); `route_auth_outcome` retries only on matching `LoggedIn` | `outer_tui_same_provider_login_retries_pending_turn_once` + eight negative paths |
| No auto-relogin mid-stream; `CredentialRevoked` non-retryable | `is_retryable()` true only for RateLimited/Timeout/Network; agent-loop retry gate requires `is_retryable()` | `anthropic_oauth_revoked_stops_turn_without_retry_or_relogin`; all-three-provider revocation test |
| Two-entry keychain persistence fail-closed | marker-first write, envelope-second; mid-transition -> typed error, no env fallback | `kind_change_is_fail_closed_between_marker_and_protected_writes`; marker-only-state tests |
| Refresh double-checked locking + bounded HTTP under lock | fast-path no-lock; slow-path acquire->re-read->HTTP->write->release; 30s `tokio::time::timeout` | `refresh_timeout_releases_lock_and_preserves_prior_credential`; concurrent-coalesce test |
| Flow-specific manual semantics (Copilot/Codex-Device never `await_manual_code`) | `run_pkce_login` races callback/manual/cancel; Copilot + Codex-Device call only `present_device_code` | `copilot_login_does_not_call_await_manual_code`; `openai_codex_device_code_never_calls_await_manual_code` |
| Per-stream auth resolution; no baked secret | providers hold `Arc<dyn AuthResolver>`; resolve on first poll; `CredentialNeeded` before HTTP | `factory_built_approved_profiles_resolve_auth_inside_each_stream`; `factory_stream_reresolves_after_store_change` |
| Anthropic cache markers only at capable positions/TTL; custom/unknown off | `build_request_body` gates on `supports_cache_control`; `1h` requires `supports_long_cache_retention && Long`; `Disabled` suppresses | factory-built wiremock test: exact count/position/TTL + custom/unknown negatives |
| Wire identity exact; Codex not constructed via Responses flags | dedicated `openai_codex_responses.rs`; `build_codex_oauth` never references `OpenAiResponsesProvider` | `dedicated_codex_request_uses_exact_base_path_body_and_headers` |

All load-bearing invariants hold with test coverage. None are unverified.

---

## 9. Residuals and recommendations

### Priority recommendations
1. **Close the standard-Responses data-only test gap (5.1).** This is the same
   blind-spot class that produced the original C-2.1 Blocker; the Codex wire is
   now protected but the standard Responses wire is not. One data-only fixture
   in `openai_responses_lifecycle.rs` closes it.
2. **Align the Codex body builder with its stream path (2.1).** Propagate the
   `C9` `strip_prefix("openai-codex:")` fix into `build_request_body` so the two
   model-id derivations cannot diverge.
3. **Add a partial-chunk SSE buffering test (5.2).** Cheap, removes a real
   regression gap in the buffering logic.

### Lower priority
4. Tighten `encode_credential` intermediate fields to `Zeroizing<String>` (3.1)
   to fully honor the spec's intermediate-field zeroization promise.
5. Decide whether the Codex thinking-level `.or_else()` fallback (2.2) is
   intentional; if not, remove it to match standard Responses.
6. Re-evaluate `secret_service_is_unavailable` (3.2) on every
   `zbus-secret-service-keyring-store` bump.

### Carry-forwards (unchanged from spec Non-Goals, not defects)
- End-to-end `SecretString`-through-provider-construction refactor (deferred
  follow-up; the `expose_secret` boundary is otherwise tight).
- Per-call credential override / `onPayload`-`onResponse` hooks / production
  refresh trigger -- all correctly out of scope and substrate-only as specified.

---

## 10. Method note and limitations

- Six subsystem deep-reads were performed by fresh-eyes subagents that did not
  share the lead auditor's memory contamination. The lead auditor independently
  re-verified every contamination-sensitive claim (4.1, 4.2), the top candidate
  findings (2.1, 2.2, 3.1, 5.1), and ran the contamination-sensitive test suites
  at HEAD (16/16 green).
- Full-workspace `cargo test` was *not* run to completion: this host has hit
  disk exhaustion from full-workspace smoke in prior sessions (~106 GB
  `target/`). Verification was scoped to targeted crate-level suites plus the
  recorded-green gate evidence in `.opi-impl-state.json`. The conclusions rest
  on static code/test reading and targeted execution, not a full workspace run.
- This audit stands alone; any overlap with `phase_exit.evaluator_summary` items
  is coincidental and re-derived here from source.
