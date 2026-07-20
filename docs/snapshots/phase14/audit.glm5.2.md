# Phase 14 Provider & Auth — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports or the remediation plan were consulted before this report was written)
**Date**: 2026-07-20
**Scope**: Tasks 14.1–14.21, phase commit range `079b5d2..8364e74`; audited at working-tree HEAD `9263114` (`fix(workspace): remediate phase 14 audit findings`, 2026-07-20), which includes the post-exit remediation on top of the phase-exit commit `8364e74`.
**Normative sources**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`, `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.
**Method**: (1) Independent full-file reads of the `opi-ai` abstract contract layer (`credential.rs`, `auth.rs`, `provider_headers.rs`, `model_info.rs`, `api_mapped.rs`, `openai_codex_responses.rs`) and the highest-risk `opi-coding-agent` impl regions (`credential_store.rs` lock + `resolve_oauth` refresh path). (2) A 10-slice parallel deep-read workflow (one high-effort finder per file group) pipelined into one adversarial refuter per candidate finding; 9/10 slices completed, and the test-quality slice was re-run by the auditor directly after a rate-limit failure. (3) Focused test execution: `cargo test -p opi-ai -p opi-coding-agent` → 2284 passed, 0 failed, exit 0.

---

## 1. Executive Summary

**Verdict: PASS** (0 blockers, 0 majors; 6 low-risk minor findings documented below — none block archive or the next phase)

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 0     |
| Minor    | 6     |
| Info     | 3     |

Per the skill's definitions, PASS applies when there are no blockers or majors and all minors are low-risk. PASS-WITH-FINDINGS (majors need attention before the next phase) and FAIL (blockers, or systemic majors) do not apply. The six minors below are worth addressing but are not gating.

Phase 14 is a security-sensitive, structurally sound implementation. The credential/auth contract layer in `opi-ai` is clean (object-safe boxed-future traits, `SecretString` everywhere with manual redacting `Debug`, well-separated error variants, reserved-header enforcement). The single highest-risk region — OAuth refresh double-checked locking with a bounded HTTP timeout — is correct: no deadlock, no partial writes, no double-refresh race, RAII lock release on every path including timeout. All 2284 focused tests pass. The phase-exit Success Criteria SC1–SC8, the four F14 obligations, `api-map`, and Non-Goals NG1–NG8 are all traced and respected.

The six findings are all **minor** and **low-risk**: one narrow spec-wording divergence on header-value validation (production-unreachable), one missing canonical-remediation hint on a deprecated id, one misleading doctor diagnostic for custom-provider proxies, and three test-coverage gaps where an invariant is covered indirectly but not asserted at its strongest point. None block archive or the next phase. Three Info items document non-defects that surfaced during review.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | PASS |
| 14.2 | OAuth architecture and per-request auth re-resolution | PASS |
| 14.3 | Request scalars and session-affinity production path | PASS-WITH-MINOR (F2.1) |
| 14.4 | Model capabilities and Anthropic cache markers | PASS |
| 14.5 | Usage and cost cache/reasoning accounting | PASS |
| 14.6 | Dynamic provider model refresh | PASS (substrate-only, by design) |
| 14.7 | Provider/auth docs, non-goal guards, final gates | PASS |
| 14.8 | Native keyring and production probes | PASS |
| 14.9 | Login/logout dispatcher and persistence | PASS |
| 14.10 | Live auth and session interaction | PASS |
| 14.11 | Factory-built Anthropic cache markers | PASS |
| 14.12 | Usage and cost contract | PASS |
| 14.13 | Documentation, verification, residual closure | PASS-WITH-MINOR (F7.1) |
| 14.14 | Native keyring host selection | PASS |
| 14.15 | WireApi, model metadata, pricing, thinking, canonical IDs | PASS-WITH-MINOR (F5.1) |
| 14.16 | ApiMappedProvider and TOML custom providers | PASS |
| 14.17 | GitHub Copilot three-wire catalog | PASS-WITH-MINOR (F4.1, F4.2) |
| 14.18 | OpenAI Codex dedicated wire, catalog, dual login | PASS-WITH-MINOR (F2.1, F4.3) |
| 14.19 | Concrete OAuth dispatcher vertical path | PASS |
| 14.20 | Outer TUI credential retry | PASS |
| 14.21 | Documentation, acceptance artifacts, Phase F | PASS |

---

## 2. Correctness

### 2.1 MINOR: `session_id` / `session-id` header values skip `HeaderValue::from_str` on the Responses and Codex wires; invalid values surface as retryable `Network` instead of non-retryable `RequestFailed`

**File:** `crates/opi-ai/src/openai_responses.rs` (session_id header application in `stream_http`); `crates/opi-ai/src/openai_codex_responses.rs:171` (`.header("session-id", session_id)`); contrast `crates/opi-ai/src/openai_chat.rs` (routes through `extra_headers` → `ProviderHeaders::merge_request` → `validate_pair` → `HeaderValue::from_str`), which is correct.

**Cause:** On the OpenAI Responses and Codex Responses wires the affinity id is attached with `RequestBuilder::header(name, value)` using a raw `&str` value, not via `HeaderValue::from_str`. The Phase 14 design (T3 3a) mandates: "Header values are constructed with `HeaderValue::from_str`; invalid values return `ProviderError::RequestFailed` and never panic." That mandate is met on the Chat wire but not on the two Responses-family wires. The Codex `validate_headers` (`openai_codex_responses.rs:321-332`) likewise does not pre-check the `session-id` value.

**Correction to the original candidate finding (adversarial verifier):** the finder's claim that `RequestBuilder::header(K, V)` **panics** on an invalid value is **false** for the pinned `reqwest 0.12.28`. In that version `header()` stores the `HeaderValue::try_from` failure inside `self.request: Result<Request, _>`; `.send().await` then surfaces it as a builder error, which the providers map to `ProviderError::Network(...)`. There is **no panic and no silent stream end** — the "never panic" half of the mandate is satisfied.

**What is actually wrong (and was understated by the finder):** the surfaced error is `ProviderError::Network`, which `ProviderError::is_retryable()` (`provider.rs:363-368`) classifies as **retryable**, whereas the mandate specifies `ProviderError::RequestFailed` (**non-retryable**). So an invalid affinity header value can trigger a retry loop instead of failing fast.

**Failure scenario:** An external `opi-ai` library consumer (or a hypothetical future change to session-id generation that emits a control byte / NUL / bare CR-LF) constructs `Request { session_id: Some("bad\nvalue".into()), .. }` and calls `OpenAiResponsesProvider::stream` (or `OpenAiCodexResponsesProvider::stream` with `cache_retention != Disabled`). The request fails after `send()` with a retryable `Network` error rather than a clean non-retryable `RequestFailed`.

**Impact:** Minor. The production `SessionCoordinator` id is a hex timestamp and always HTTP-safe, so the opi binary never trips this today; it only affects external library consumers or a future session-id source that emits invalid bytes. No security exposure (no secret involved), no panic.

**Fix:** In both providers, validate the affinity id with `HeaderValue::from_str` before applying the header, returning `ProviderError::RequestFailed` on failure — mirroring the `validate_pair` path used by OpenAI Chat; or route the session-id header through the same `ProviderHeaders::merge_request` helper.

**Spec ref:** `2026-07-11-phase14-provider-auth-design.md` T3 3a (header construction mandate).

---

## 3. Security / Redaction

No findings. Observed strengths:

- `Credential`, `ResolvedAuth`, and `OAuthCredential` wrap every secret in `secrecy::SecretString` and carry **manual `Debug` impls** that redact `access`/`refresh`/`secret`, so a future `secrecy` change to `SecretString::Debug` cannot leak. (`credential.rs:71-93`, `auth.rs:79-90`, `auth.rs:165-175`.)
- `expose_secret()` is called only at the concrete HTTP boundary; secrets are zeroized on drop. No raw credential value is registered as a `SecretRedactor` pattern (avoids expanding the secret footprint). No opi-managed plaintext credential file exists (NG1 verified structurally by the `phase14_provider_auth_docs` guard, which scans `crates/opi-coding-agent/src` for `credentials.json`/`credentials.toml`/`auth.json`).
- The `fs4` lock file holds **no secret**; the redaction test `redaction_only_secret_free_lock_exists_outside_fake_keyring` recursively scans the injected config root plus read-back `Credential::Debug` output and proves only the secret-free `credential.lock` exists outside the fake keyring.
- `present_device_code` accepts only the public `user_code` + `verification_uri`; the device code (which grants token issuance) is never passed to any presenter method.
- Reserved auth/transport headers (`authorization`, `x-api-key`, `chatgpt-account-id`, `session-id`, etc.) are rejected from both provider-configured headers (`ProviderHeaders::try_new`) and per-request `extra_headers` (`merge_request`), so NG3 (no per-call auth-header override) holds on every wire.
- No authorization code, access/refresh token, device secret, JWT, or keychain envelope appears in captured test output or error paths; corrupt/unknown envelopes return typed errors and **never** fall through to env fallback (B02).

---

## 4. Test Quality

### 4.1 MINOR: Copilot managed-header override-rejection test covers only 4 of 7 protected names

**File:** `crates/opi-coding-agent/tests/github_copilot_provider.rs:399-416`

**Cause:** `github_copilot_headers_match_reviewed_static_contract` iterates the per-request override-rejection check over only `[User-Agent, X-Initiator, Openai-Intent, Copilot-Vision-Request]`. The managed set `GITHUB_COPILOT_MANAGED_HEADERS` (`crates/opi-ai/src/provider.rs:116-124`) has **7** entries: the four above plus `editor-version`, `editor-plugin-version`, and `copilot-integration-id`.

**Failure scenario:** A future refactor weakens the case-insensitive membership check in `github_copilot_route_headers` (e.g. switches to exact-case, or drops one of the three `Editor*`/`Copilot-Integration-Id` entries from the const). A caller then sets `extra_headers = [("Editor-Version", "evil")]` and the override is silently accepted. The test still passes because it never probes those three names on the rejection path.

**Impact:** Minor. Three of seven reviewed Copilot reserved headers have no regression coverage on the override-rejection path — a security-relevant invariant (provider-managed transport headers cannot be overridden per-request).

**Fix:** Extend the iteration list at `github_copilot_provider.rs:399-404` to include `Editor-Version`, `Editor-Plugin-Version`, and `Copilot-Integration-Id` so every entry of `GITHUB_COPILOT_MANAGED_HEADERS` has a negative test.

### 4.2 MINOR: `Copilot-Vision-Request: true` not asserted on the `/v1/messages` (Anthropic) route with image content

**File:** `crates/opi-coding-agent/tests/github_copilot_provider.rs:472-537`

**Cause:** `github_copilot_vision_header_covers_user_and_tool_result_images` asserts `Copilot-Vision-Request: true` only on `/chat/completions` (user image) and `/responses` (tool-result image). The `/v1/messages` Anthropic route is exercised only with text content and asserts the header is **absent**.

**Failure scenario:** A refactor to `AnthropicProvider::stream` (`crates/opi-ai/src/anthropic.rs` ~`:1258`) drops or short-circuits the `copilot_headers` branch on the Anthropic route only. A user sends an image-bearing request to `claude-sonnet-4.5` via the Copilot Anthropic Messages route and `Copilot-Vision-Request` is not emitted. The test still passes because it never sends an image to `/v1/messages`.

**Impact:** Minor. The image-route parity claim for `Copilot-Vision-Request` is under-tested on the Anthropic wire; a single-route regression would not be detected. Production code is currently correct.

**Fix:** Add a fourth case sending an image-bearing `UserMessage` to `claude-sonnet-4.5` (`/v1/messages`) and assert `Copilot-Vision-Request: true` on the captured request.

### 4.3 MINOR: 401/403 revocation tests do not assert exactly one HTTP request

**File:** `crates/opi-coding-agent/tests/github_copilot_provider.rs:582-607` (`github_copilot_401_and_403_are_revoked_on_every_wire`); `crates/opi-ai/tests/openai_codex_responses.rs:500-531` (`dedicated_codex_401_and_403_are_revoked_and_redacted`).

**Cause:** Both tests assert only (a) `ProviderError::CredentialRevoked`, (b) `provider_id` matches, (c) `!error.is_retryable()` (plus redaction for Codex). Neither asserts `received_requests().len() == 1`.

**Failure scenario:** A future change adds a follow-up HTTP request after the auth-invalid response inside the provider stream (e.g. a telemetry call, a revocation-reason GET, or an attempted auto-refresh). The first response still maps to `CredentialRevoked` and `is_retryable() == false`, so both tests still pass — but the "no second HTTP request, no auto-login" guarantee (T2 D5) is silently broken.

**Impact:** Minor. The strongest form of the revocation invariant — ends the turn, performs no second request — is not directly enforced; only its typed-error and retry-class consequences are. Production code is currently correct (`openai_codex_responses.rs:334-342`, `anthropic.rs:1068-1085` map 401/403 directly with no follow-up).

**Fix:** After the `drain()` call in both tests, assert `received_requests().len() == 1` to lock the no-follow-up-request invariant.

---

## 5. Spec Compliance

### 5.1 MINOR: Old `copilot`/`codex` provider ids rejected without canonical remediation message

**File:** `crates/opi-coding-agent/src/provider_factory.rs:1809-1827` (`build_runtime_provider` `_` arm → `ProviderBuildError::Config(format!("unknown provider: {other}"))`); surfaced at `crates/opi-coding-agent/src/main.rs:819-821` (interactive) and `:685-687` (RPC).

**Cause:** Any unrecognized provider id falls through to a generic `"unknown provider: {other}"` message with no detection of the deprecated dev-only `copilot`/`codex` ids and no hint that the canonical ids are now `github-copilot`/`openai-codex`.

**Failure scenario:** An upgrading user runs `opi --model copilot:gpt-4o`. The CLI exits 2 with stderr `opi: unknown provider: copilot` and no hint about the rename; the user must discover `github-copilot` from external docs. `tests/provider_identity.rs:40-47` pins only `registry.lookup("copilot").is_none()`, not remediation text.

**Impact:** Minor UX regression for users migrating off the pre-Phase-14 development-only ids. The task 14.15 DoD (`2026-07-14-phase14-exit-remediation-design.md:1079`) states "old provider ids are rejected with canonical remediation," where "canonical remediation" is used consistently elsewhere to mean emitting the canonical id plus the `/login <canonical-id>` hint.

**Fix:** In `build_runtime_provider` (or the factory's provider match), detect bare `"copilot"` / `"codex"` and return `ProviderBuildError::Config("'copilot' has been renamed; use provider id 'github-copilot' (login: /login github-copilot)")` (and analogously for `codex`).

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| `opi-agent` constructs no provider and owns no auth config (load-bearing) | `crates/opi-agent/src/agent.rs`/`agent_loop.rs` carry only an opaque `session_id` string; all provider construction + auth resolution live in `opi-coding-agent`/`opi-ai`. `session.rs` contains none of `OAuth`/`Credential`/`access_token`/`refresh_token`. | `phase14_provider_auth_docs::every_phase14_non_goal_has_documented_and_structural_evidence` asserts `session.rs` excludes those tokens. |
| `ProviderCollection` stays off the live path (status-gate only) | `provider_collection.rs::dispatch_stream` consumes a precomputed redacted `CredentialSource`; the descriptor performs no IO. The live resolver is inside each provider's `stream`. | `provider_collection` tests; full-file read, 0 findings. |
| Corrupt/unknown envelope never falls through to env | `CredentialStoreError::{MalformedEnvelope, CorruptMarker, UnknownEnvelope}` are distinct `Err`; only `Ok(None)` (missing) and `BackendUnavailable` (API-key only) take the env path. | `credential_store.rs::corrupt_marker_is_redacted_and_blocks_env_fallback` (:708), `oauth_marker_only_state_is_typed_and_never_leaks_secrets` (:786). |
| No auto-relogin mid-stream (NG2) | 401/403 → `CredentialRevoked` (non-retryable); ends turn; never starts login. `AuthInvalidPolicy::CredentialManaged`/`Static` is explicit per route, never inferred from Bearer. | `github_copilot_provider.rs::github_copilot_401_and_403_are_revoked_on_every_wire`; Codex counterpart. (F4.3 notes the single-request half is not directly asserted.) |
| Usage subsets bounded by parent; malformed → error (B16) | `validate_usage_subset` returns `StreamError` when `reasoning > output` or `cache_write_1h > cache_write`; no invalid `Usage` event. `total_tokens` counts each parent once. | Provider fixture tests; session resume preserves subsets. |
| Reserved auth headers rejected (NG3) | `ProviderHeaders::try_new` + `merge_request`; per-wire `validate_headers`. | `provider_headers` unit tests; Copilot managed-header test (F4.1 notes incomplete name coverage). |
| Refresh: no double-refresh race, no partial write, bounded | `resolve_oauth` fast-path read → `needs_refresh` → acquire RAII lock → re-read (double-check) → `tokio::time::timeout(refresh_timeout, refresh)` → `write_unlocked` only on success → post-failure re-read. | `credential_store.rs` refresh/timeout tests; full-file read, 0 findings. |
| `api-map` implemented (not deferred) | `ApiMappedProvider`; `[providers.custom]`; `github-copilot` 3-wire; `openai-codex` dedicated wire. | `custom_provider_map`, `github_copilot_provider`, `openai_codex_provider`, `api_mapped`/`provider_collection` tests. |

---

## 7. Cross-Task Integration

### 7.1 MINOR: Doctor's `provider_proxy_url` ignores `config.providers.custom`

**File:** `crates/opi-coding-agent/src/doctor.rs:699-720`

**Cause:** `provider_proxy_url` inspects the built-in providers and `config.providers.openai_compatible`, but its `other =>` arm has no `providers.custom.get(other)` lookup. `CustomProviderConfig.proxy: Option<ProviderProxyConfig>` exists (`config.rs:118-127`) and is honored at runtime by `build_custom_provider` via `build_proxied_client`.

**Failure scenario:** A user configures `[providers.custom.acme]` with `proxy.url = "http://proxy.internal:3128"`, sets `defaults.model = "acme:foo"`, and runs `opi doctor --scope config`. The diagnostic at `CODE_DOCTOR_CONFIG_PROXY` reports "no explicit proxy configured for selected provider `acme`" even though the custom provider's proxy is real and will be used at runtime.

**Impact:** Minor. Misleading doctor config-scope diagnostic for custom providers; not a binding Phase 14 item, but a real cross-cutting consistency gap (doctor vs factory) surfaced while auditing factory/doctor integration.

**Fix:** Add a `providers.custom.get(other).and_then(|p| p.proxy.as_ref().map(|p| p.url.as_str()))` branch ahead of the `openai_compatible` fallback in `provider_proxy_url`.

---

## 8. Residuals and Recommendations

### Priority recommendations

1. **Close the three test-coverage gaps (F4.1–F4.3).** All three are one- to four-line additions that lock invariants currently held only by code review (full 7-name Copilot managed-header rejection, image-route `Copilot-Vision-Request` on `/v1/messages`, single-request revocation). Cheap, high value.
2. **Add `HeaderValue::from_str` validation for the Responses/Codex session-id header (F2.1).** Aligns the two Responses-family wires with the Chat wire and the T3 3a mandate, and converts a latent retryable-error edge into a clean non-retryable `RequestFailed`.
3. **Emit canonical remediation for deprecated `copilot`/`codex` ids (F5.1)** and **extend doctor's proxy lookup to custom providers (F7.1).** Both are small, user-facing correctness improvements.

### Rejected findings (independently confirmed not defects)

Eight candidate findings were refuted by adversarial verification with concrete evidence. The most substantive:

- *`build_provider` cannot construct OAuth providers / Anthropic not lazy* — framing error; the bare-id `_` arm is not the OAuth path, and `build_anthropic` resolves lazily at stream time.
- *`CredentialNeeded` failed-turn user message not persisted* — the designed symmetric persistence contract (only finalized turns persist), already pinned by `phase8_cancel_persists_only_finalized_state`.
- *`CredentialNeeded` pending turn survives `/fork`* — refuted: `fork_current_session` rebuilds messages from JSONL via `reconstruct_context`, and the unpersisted user message was never written, so it is absent from the fork.
- *CHANGELOG omits the malformed-subset → `StreamError` break* — the "prior behavior" was itself unreleased Phase 14 code, so there is no public break to record.
- *opi-spec SC8 "58-row acceptance manifest" has no artifact* — the manifest **is** the guard test file (`29 alignment + 29 historical` pinned rows in `phase14_provider_auth_docs.rs`).
- *Guard test does not pin Bearer on the Copilot Anthropic Messages route* — verified **not a finding**: `github_copilot_provider.rs:344-355` behaviorally asserts `authorization: Bearer copilot-route-token` and `x-api-key is None` on `/v1/messages`, and `anthropic.rs:940-946` confirms the Bearer path. The invariant is mutation-resistantly locked by a behavioral test; a redundant guard pin is unnecessary.

### Info items (non-defects)

- **I1 — `anthropic_cache_markers` is a single test function but densely comprehensive.** It builds Anthropic through the real factory (`build_provider_with_oauth`) + real `ProviderRegistry`, resolves final `ModelInfo` capabilities, fires 5 captured HTTP requests (Long/Short/Disabled/custom-default-off/unknown), and asserts exact marker positions (system + last-user + last-assistant + last-tool) with `ttl:"1h"` vs ephemeral plus null-marker negatives. SC5 is well covered; the "1 test" count is misleading.
- **I2 — Three `#[ignore]`d tests** across `interactive_auth`, `interactive_tui_auth`, and `phase14_provider_auth_docs` are subprocess-only RPC stdio entry points (e.g. `phase14_docs_rpc_run_stdio_child`), invoked by their parent test rather than skipped. Not a coverage gap.
- **I3 — `openai_codex_responses.rs::map_http_status` discards the upstream error body** (`_body` unused; 4xx/5xx → `ProviderSide("HTTP {code}")`). This is a defensible redaction-by-simplification but loses debugging detail for non-auth upstream errors. Consider preserving a redacted/truncated body excerpt in a `tracing` call only.

### Methodology notes

- The audit workflow's dedicated test-quality slice failed with an upstream rate-limit and was re-performed by the auditor directly via full-file reads of `anthropic_cache_markers.rs`, `interactive_tui_auth.rs`, the `credential_store.rs` redaction tests, and `phase14_provider_auth_docs.rs`; the three confirmed test-quality findings (F4.1–F4.3) originated from the Copilot/Codex parity slice.
- No real keychain, OAuth endpoint, LLM API, browser, terminal, user config directory, or session directory was accessed. All evidence is from source, tests, configuration, and the design specification.
