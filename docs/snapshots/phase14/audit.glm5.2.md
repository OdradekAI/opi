# Phase 14 Provider & Auth — Independent Code Audit

**Auditor**: glm5.2 (independent; no prior audit reports or evaluator transcripts consulted)
**Date**: 2026-07-19
**Scope**: Tasks 14.1–14.21, commits `d9f21a97..8364e74` (≈36 commits); cross-checked against current HEAD `72555e0`
**Method**: Read both normative design specs in full; first-hand full reads of the highest-risk files (`credential.rs`, `auth.rs`, `native_keyring.rs`, `credential_store.rs`, `provider.rs`, `openai_responses.rs`, `anthropic.rs`, `openai_chat.rs` revocation paths, `agent_loop.rs`); an 11-group read-only analyst workflow (9 groups returned, 2 re-run after a 429 stampede) with 3 adversarial verifiers; plus targeted first-hand confirmation of every Blocker/Major candidate. Spot-checked the test suite on the light `opi-ai` crate (`usage_cost` 25/25, `api_mapped_provider`/`openai_codex_responses`/`request_enrichment` green); the heavy `opi-coding-agent` binary-crate tests were not built because the host disk sat at ~9.6 G free (98 %) with a 93 G `target/`.

---

## 1. Executive Summary

**Verdict: PASS-WITH-FINDINGS**

| Severity | Count |
|----------|-------|
| Blocker  | 0     |
| Major    | 2     |
| Minor    | 22    |
| Info     | ~15   |

Phase 14 is a genuinely well-built security-critical phase. The credential/OAuth core — the part most likely to leak secrets or corrupt state — is the strongest area: a secret-free cross-process lock, package-private unlocked backend operations so refresh holds one lock without recursing, double-checked refresh with bounded HTTP timeout and write-only-on-success, a two-service keychain split (`opi` protected / `opi.presence` non-secret marker) proven by a test that sets a pending error on the protected entry to show `probe` never reads it, strict versioned-envelope decoding that never collapses corruption to absence, manual redacting `Debug` impls on every secret-bearing type, and `expose_secret()` confined to four concrete HTTP boundaries. The non-goals (NG1–NG8) are all preserved, the construction-ownership invariant holds structurally (verified: `opi-agent` depends only on `opi-ai`; zero grep hits for concrete providers/keyring/oauth in its source), and the usage/cost subset math is correct and survives session resume.

The two Majors are real but bounded spec deviations, not safety failures: (1) the OpenAI Responses provider over-gates session-affinity emission, and a test locks the wrong behavior in; (2) OAuth login cancellation (`Ctrl+C`) is wired for only 1 of 4 flows despite the spec requiring it and the infrastructure existing. Neither loses data, leaks secrets, or crashes. Around them sits a cluster of precision minors in the `CredentialRevoked`/`AuthFailed` mapping and a population of test-realism and doc minors typical of a 21-task phase.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1  | Credential store model                       | PASS (1 minor: probe misclassifies corrupt marker) |
| 14.2  | OAuth + per-request auth re-resolution       | **PASS-WITH-FINDINGS** (MAJOR: cancellation in 3/4 flows; minors: store-write-failure silent, error echo, revocation-variant precision) |
| 14.3  | Request scalars + session affinity           | **PASS-WITH-FINDINGS** (MAJOR: Responses over-gating; minor: header-value validation) |
| 14.4  | Model capabilities + cache markers           | PASS (1 minor: non-positive limits not validated) |
| 14.5  | Usage/cost cache+reasoning accounting        | PASS (clean; 25/25 tests) |
| 14.6  | Dynamic model refresh substrate              | PASS (1 minor: comment/impl drift) |
| 14.7  | Docs, non-goal guards, final gates           | PASS |
| 14.8  | Native keyring + production probes           | PASS (1 minor: doctor ignores `[providers.custom]`) |
| 14.9  | Login/logout dispatcher + persistence        | PASS-WITH-FINDINGS (minor: store-write failure not surfaced) |
| 14.10 | Live auth + session interaction              | PASS (minors: revocation-variant precision) |
| 14.11 | Factory-built Anthropic cache markers        | PASS |
| 14.12 | Usage/cost contract                          | PASS |
| 14.13 | Documentation + residual closure             | PASS |
| 14.14 | Native keyring host selection                | PASS (F14-01 satisfied; verified first-hand) |
| 14.15 | WireApi, model metadata, pricing, canonical IDs | PASS (1 minor: non-positive limits) |
| 14.16 | ApiMappedProvider + TOML custom providers    | PASS (test-quality minors) |
| 14.17 | GitHub Copilot three-wire catalog            | PASS (1 minor: over-broad CredentialRevoked) |
| 14.18 | OpenAI Codex wire/catalog/dual login         | PASS-WITH-FINDINGS (MAJOR cancellation: Codex Browser PKCE; minor: device error echo) |
| 14.19 | Concrete OAuth dispatcher vertical path      | PASS |
| 14.20 | Outer TUI credential retry                   | PASS (test-quality minors; provider_id stripped in TUI messages) |
| 14.21 | Documentation, acceptance, Phase F           | PASS |

---

## 2. Security / Redaction Findings

This dimension is the phase's center of gravity and is, on the whole, the strongest part of the implementation.

### 2.1 INFO (positive): the redaction architecture is sound and behaviorally proven

The redaction invariants hold and are tested with real canaries, not vacuous asserts:

- Every secret-bearing type (`Credential`, `OAuthCredential`, `ResolvedAuth`, legacy `SecretKey`) has a **manual `Debug` impl** that redacts each `SecretString` field, so a future `secrecy` version changing `SecretString`'s `Debug` cannot leak (`credential.rs:71-93`, `auth.rs:54-65`, `auth.rs:140-150`).
- `expose_secret()` is called at exactly four concrete HTTP boundaries (`anthropic.rs:907`, `openai_chat.rs:1144`, `openai_responses.rs:335`, `openai_codex_responses.rs:160`) — confirmed by grep absence in the substrate. The OAuth layer never passes the secret `device_code` to the presenter; `present_device_code` receives only the public `user_code` + `verification_uri` (`auth.rs:252-256`).
- The credential store uses a two-service split: `KEYCHAIN_SERVICE="opi"` (protected envelope) vs `KEYCHAIN_PRESENCE_SERVICE="opi.presence"` (non-secret kind marker). `probe` reads only the marker. `keyring_core_backend_probe_reads_only_nonsecret_marker_entry` (`credential_store.rs:1487-1542`) sets a pending error on the protected entry and proves `probe` returns `Present` without touching it.
- NG1 (no plaintext credential file) is enforced and tested: `redaction_only_secret_free_lock_exists_outside_fake_keyring` asserts `vec![OsString::from("credential.lock")]` is the only on-disk artifact and scans the lock contents for `API_KEY`/`ACCESS`/`REFRESH`.
- A bounded redaction scenario (`oauth_auth.rs:1838-2057`) seeds real secrets into the store/envelope and recursively scans captured URLs, form bodies, SSE events, error `Debug`/`Display`, diagnostics, presenter `notify_failure` reasons, NDJSON, doctor text+JSON, and the temp root.

### 2.2 MINOR (security): `safe_excerpt` is regex-based and can miss non-standard API-key prefixes echoed in 401 bodies

**File:** `crates/opi-ai/src/http.rs:253-308`; consumed at `anthropic.rs:1064-1071`, `openai_chat.rs:1277-1284`, and replicated across `azure_openai.rs`, `gemini.rs`, `bedrock/mod.rs`, `vertex.rs`
**Cause:** The API-key 401/403 path does `ProviderError::AuthFailed(format!("authentication failed: {}", safe_excerpt(body)))`. `safe_excerpt` scrubs only fixed-pattern regexes (`sk-`, `ghp_/gho_/…`, `github_pat_`, `AIza`, `eyJ…` JWTs, bearer tokens, credentialed URLs, a fixed query-secret key list). The Bearer/OAuth 401 arm deliberately **drops** the body for exactly this reason; the API-key arm does not. (Confirmed first-hand at `anthropic.rs:1060-1071`: the Bearer arm has `// Body intentionally dropped.`)
**Impact:** Native Anthropic `sk-ant-…` keys are scrubbed. But an OpenAI-compatible custom proxy whose key uses a non-standard prefix, echoed back in a 401 body (e.g. `invalid key: customprefix-XYZ`), reaches the `AuthFailed` message and onward into diagnostics/trace. The independent verifier expanded the scope: this is a **shared pattern across all six providers**, not Anthropic-only.
**Fix:** Either drop the body on the API-key 401/403 arms too (parity with the Bearer arm, losing diagnostic detail), or scrub the actual submitted secret string per-request. At minimum, document the residual. (See also §3.3 — the API-key 401 returning `AuthFailed` instead of `CredentialRevoked` is what keeps this body-surfacing path alive.)

### 2.3 MINOR (security): Copilot `poll_device_token` echoes unknown OAuth `error` strings verbatim into a `Config` error

**File:** `crates/opi-coding-agent/src/oauth.rs:1196-1202`
**Cause:** `Some(other) => Err(ProviderError::Config(format!("device authorization error: {other}")))` interpolates the raw upstream `error` code. The polling form body carries `device_code` + `client_id`. Compare `poll_codex_device_token` (`oauth.rs:874-876`) which uses only the HTTP status.
**Impact:** Low realistic risk — GitHub's OAuth error vocabulary is fixed — but the unknown-error branch is exercised by no redaction test (the existing Copilot leak test uses only `access_denied`). An unexpected `error` value that echoed submitted material would pass straight into a `Config` message.
**Fix:** Drop `other` from the message (`format!("device authorization error ({status})")`) or apply an OAuth-error-code allowlist before interpolating. Add a test injecting `{"error":"weird_echo_dc_xy"}` and asserting neither the device code nor the echo appears.

---

## 3. Correctness Findings

### 3.1 MAJOR: OAuth login cancellation (`Ctrl+C`) is wired for only 1 of 4 flows

**Files:** `crates/opi-coding-agent/src/oauth.rs:166-178` (PKCE runner) and `oauth.rs:1316-1425` (Copilot device-code)
**Cause:** The `LoginPresenter::await_login_cancelled` trait method exists (`auth.rs:263-265`), the `TuiLoginPresenter` implements it via `tokio::signal::ctrl_c` (`oauth.rs:1566-1572`), and `run_codex_device_login` races it in a `biased` select (`oauth.rs:1017-1036`). But:
- `run_pkce_login`'s `tokio::select!` has only three arms — `accept_one_callback`, `presenter.await_manual_code()`, `tokio::time::sleep(config.timeout)`. There is **no** `await_login_cancelled` arm. This covers **Anthropic PKCE** and **Codex Browser PKCE** (which delegates to the same runner, `oauth.rs:1070`).
- `CopilotOAuthProvider::login` races each poll and inter-poll sleep only against the 5-minute `total_budget`; there is no outer wrap against `await_login_cancelled`.

Git history confirms `await_login_cancelled` was introduced in a single commit scoped to Codex Device-Code only — the infrastructure was built but never propagated to the other three flows.
**Spec ref:** Task 14.2 DoD requires "PKCE verifies S256/state, loopback-only binding, timeout, mismatched-state rejection, and **callback/manual cancellation races**"; device polling "covers pending, slow-down, denial, expiry, timeout, and **cancellation**." SC2 was traced `met` with evidence "concrete Browser/Device dispatcher and persistence rows," but SC2's own text includes "cancellation" as a required sub-item — so this is an unmet SC2 sub-claim, not merely a polish item.
**Impact:** A user pressing `Ctrl+C` during an Anthropic/Codex-Browser/Copilot login cannot abort the in-flight flow; it only stops when the 5-minute `LOGIN_TIMEOUT` elapses or the callback/manual arm resolves. Bounded (no hang forever, no secret leak, no corruption) but a real interactive-UX and spec gap across three production flows. The `TuiLoginPresenter` ctrl_c handler is dead code for those flows.
**Fix:** Add a biased `await_login_cancelled` arm to `run_pkce_login` (returns `ProviderError::LoginCancelled`, calls `notify_failure`), and wrap `CopilotOAuthProvider::login`'s body in the same `biased` select used by `run_codex_device_login`. Add three parallel cancellation tests (Anthropic PKCE, Codex Browser PKCE, Copilot device-code) asserting `LoginCancelled`, a `notify_failure` reason, no token-exchange request recorded, and an empty store — mirroring `openai_codex_device_code_cancellation_writes_nothing`.

### 3.2 MAJOR: OpenAI Responses gates `prompt_cache_key` and `x-client-request-id` on `send_session_id_header`; spec gates only the `session_id` header

**File:** `crates/opi-ai/src/openai_responses.rs:344-350` and `:496-501`
**Cause:** Confirmed first-hand. `stream_http` does `if config.send_session_id_header && let Some(session_id) = session_id { request.header("session_id", …).header("x-client-request-id", …) }` (both headers gated), and `stream` does `if config.send_session_id_header && let Some(session_id) = … { body["prompt_cache_key"] = … }` (body field gated).
**Spec ref:** Design spec line 531-534: *"Direct OpenAI Responses emits `prompt_cache_key` plus `x-client-request-id`; `ResponsesConfig::send_session_id_header` controls the `session_id` header and defaults `true` for the built-in direct profile. Custom/proxy profiles must opt in."* Only the `session_id` header should be gated.
**Impact:** When a custom/proxy Responses profile sets `send_session_id_header=false` (to suppress the `session_id` header), it **also** silently loses `prompt_cache_key` and `x-client-request-id` — degrading prompt caching and request tracing that the spec says should still be sent. Blast radius is narrow: the built-in direct profile defaults `true`, so the default path is correct; only opt-in custom/proxy profiles are affected. Compounding: `request_enrichment.rs:437-476` asserts all three are absent when `send_session_id_header=false`, **locking the deviation into a passing test** — so the fix requires reconciling the test against the spec.
**Fix:** Drop the `send_session_id_header &&` qualifier from the `prompt_cache_key` write and split the header emission so `x-client-request-id` is always sent when `session_id` is `Some`, with only the `session_id` header gated on the config flag. Update `session_affinity_wire_mappings` so the custom case asserts `session_id` absent but `prompt_cache_key` + `x-client-request-id` present.

### 3.3 MINOR: Anthropic API-key 401/403 returns `AuthFailed`, not the spec-mandated `CredentialRevoked`

**File:** `crates/opi-ai/src/anthropic.rs:1059-1071`
**Cause:** `map_http_status` returns `CredentialRevoked` only for `401|403 if scheme == Bearer`; an Anthropic **API-key** 401/403 (the common direct path, `x-api-key`) falls through to `AuthFailed`. Confirmed first-hand.
**Spec ref:** "Revocation: auth-invalid (401/403) for **Anthropic** / any GitHub Copilot wire / OpenAI Codex => `CredentialRevoked`, non-retryable, ends turn, no second HTTP request, no auto-login (NG2)." Direct Anthropic with an API key is part of "for Anthropic."
**Impact:** `AuthFailed` is also non-retryable (`is_retryable()` is `RateLimited|Timeout|Network` only), so the turn still ends and no auto-login fires — the user-observable behavior is largely preserved. The differences: the diagnostic variant/category is `Auth`-generic rather than the specific `CredentialRevoked` signal, and the body is surfaced via `safe_excerpt` (the §2.2 redaction residual). No test asserts the `CredentialRevoked`-vs-`AuthFailed` split for the API-key path.
**Fix:** Widen the Anthropic arm to `401|403 => CredentialRevoked` (drop the body, parity with the Bearer arm), or document this as an intentional divergence and add a test pinning the API-key classification. Also fix the contradictory doc comment at `anthropic.rs:1046-1051` (it says "403 is `AuthFailed` for both schemes," but the code returns `CredentialRevoked` for Bearer 403).

### 3.4 MINOR: Chat Bearer 401/403 over-broadly maps to `CredentialRevoked`, catching static-Bearer `openai_compatible` profiles

**File:** `crates/opi-ai/src/openai_chat.rs:1270-1276`
**Cause:** `401|403 if scheme == AuthScheme::Bearer => CredentialRevoked`. The factory builds `openai_compatible` profiles (OpenRouter, Mistral) with `AuthScheme::Bearer` **and** a static API key (`provider_factory.rs:663-666`). The arm's own comment says "Bearer (OAuth) credential — e.g. Copilot," but the code cannot distinguish OAuth-Bearer from static-API-key-Bearer.
**Spec ref:** `CredentialRevoked` is reserved for Anthropic / GitHub Copilot / OpenAI Codex only. OpenRouter/Mistral are not in that set; pre-Phase-14 they returned `AuthFailed` with a safe excerpt.
**Impact:** An OpenRouter/Mistral 401/403 is now classified `CredentialRevoked` (and the body is dropped, losing diagnostic detail). Non-retryable either way, so no turn-retry harm; the issue is mis-classification + a diagnostic-detail regression for those profiles.
**Fix:** Distinguish OAuth-backed Bearer credentials from static-Bearer credentials — e.g. expose an `is_oauth()`/`credential_kind()` signal on the resolver (it knows), or pass an explicit `revoke_on_auth_invalid: bool` into the provider. Add a test asserting OpenRouter/Mistral 401 stays `AuthFailed`.

### 3.5 MINOR: `validate_extra_headers` checks header names but not values; spec says invalid values => `RequestFailed`

**File:** `crates/opi-ai/src/provider.rs:164-193`
**Cause:** The public validator iterates `for (name, _value) in headers` and inspects only the name. A header with a valid name but a value containing CR/LF/NUL or other bytes outside HTTP field-value grammar passes validation; what happens at the wire layer then depends on `reqwest`.
**Spec ref:** "Header values are constructed with `HeaderValue::from_str`; invalid values return `ProviderError::RequestFailed` and never panic."
**Fix:** Either extend `validate_extra_headers` to validate values against field-value grammar (visible ASCII + tab), or wrap the per-header wire application in a helper that converts `HeaderValue::from_str` errors to `RequestFailed` at every provider's header-emission site. (`provider_headers.rs` already does `HeaderValue::from_str` for configured/merged headers; the gap is the raw `request.extra_headers` path.)

### 3.6 MINOR: `doctor` provider probe ignores `[providers.custom.<id>]`, emits a misleading warning

**File:** `crates/opi-coding-agent/src/doctor.rs:675-741, 446-452`
**Cause:** `provider_credential_probe` looks up only built-in descriptors and `config.providers.openai_compatible`; it never consults `config.providers.custom`. For a custom provider both lookups return `None`, so `provider_diagnostics` pushes a warning that the provider "is not a known built-in or configured profile" — even though it IS configured and was validated by `config.rs`. The asymmetry is visible because `--list-models` *does* list custom providers.
**Fix:** Consult `config.providers.custom.get(provider)` and probe its `api_key_env`, returning an `EnvApiKey` `CredentialProbe`; fall through to the normal Present/Absent path instead of the unknown-profile warning.

### 3.7 MINOR: `probe` reports a corrupt marker as `BackendUnavailable`, misclassifying the failure mode

**File:** `crates/opi-coding-agent/src/credential_store.rs:754-774`
**Cause:** `Err(CorruptMarker { .. }) => CredentialSource::BackendUnavailable { reason: "credential marker is corrupt" }`. The marker entry exists and is readable, so the backend is in fact available; the data is corrupt.
**Impact:** `doctor`/`--list-models` conflate data corruption with backend unavailability — a user with a corrupt marker is told the keychain is unreachable when it is reachable but the entry is unreadable. The reason string partially mitigates the conflation.
**Fix:** Either add a `CredentialSource::Corrupt { reason }` variant so `probe` can return a faithful category, or rename the reason (e.g. `"credential marker corrupt (backend available)"`) and document the conflation in the `probe` doc.

### 3.8 MINOR: `ModelCapabilities` accepts non-positive `context_window`/`max_output_tokens` with no validation

**File:** `crates/opi-ai/src/model_info.rs:30-40, 538-551`
**Cause:** `ModelCapabilities::new(context_window, max_output_tokens)` takes `u64` with no `> 0` check; `ModelInfo::validate()` checks wire/compat match and pricing only — not limits. The spec lists "non-positive token limits" among construction/IO rejections. Pricing tiers ARE validated.
**Impact:** A `ModelInfo` with `context_window=0` passes `validate()` and `ApiMappedProvider::try_new`. If the TOML layer doesn't catch it, a misconfigured model enters the catalog silently. (May be intentional as an "unknown-limit" sentinel; if so, document it.)
**Fix:** If non-positive limits are invalid, add the check in `ModelCapabilities::new` or `ModelInfo::validate` and a guard test. If zero is an allowed sentinel, document it explicitly.

### 3.9 MINOR: `login_oauth` swallows store-write failures without `notify_failure`

**File:** `crates/opi-coding-agent/src/oauth.rs:1783-1791`
**Cause:** `DeferredSuccessPresenter` suppresses `notify_success` so it fires only after `store.write` succeeds — but the error path uses `?` and returns without calling `presenter.notify_failure(...)`. The OAuth provider has already internally succeeded (its `notify_success` was suppressed), so after a successful exchange that the keychain refuses to persist, the user gets **neither** success nor failure feedback.
**Impact:** Silent UX failure on a (rare) keychain-write error post-exchange. The existing test `login_oauth_store_failure_stays_typed_and_does_not_report_success` only asserts `notify_success_count == 0`; it does not assert a failure was surfaced.
**Fix:** Replace `?` with `match`; call `presenter.notify_failure("credential store write failed")` before returning the error. Extend the test to assert `notify_failure_reasons` contains that reason.

### 3.10 MINOR: `present_provider_error` strips `provider_id` from all TUI auth-failure messages

**File:** `crates/opi-coding-agent/src/interactive_auth.rs:265-291`
**Cause:** Every match arm returns a generic static string with no `provider_id` interpolation ("credential is still required", "credential was denied or expired", "authentication failed"). The underlying `ProviderError`/`AgentError` carry `provider_id`, and the `apply_prompt_completion_to_tui` path does format it into a `[credential needed for …]` system message — but `present_provider_error`'s own output does not.
**Spec ref:** "Errors name provider_id, NEVER an env-var name." Satisfied at the error-type layer; partially lost at this display layer.
**Fix:** Thread `provider_id` into the formatted strings (e.g. `format!("credential for {provider_id} was denied or expired")`), or document the display-layer divergence.

### 3.11 MINOR: OAuth refresh-timeout bypasses the post-failure re-read the spec describes

**File:** `crates/opi-coding-agent/src/credential_store.rs:961-999`
**Cause:** On timeout, the `?` after `map_err(|_| AuthFailed(...))` short-circuits out of `resolve_oauth`, so the inner `Err(refresh_err)` re-read at lines 985-998 is never entered on the timeout path.
**Impact:** Functionally moot: the mutation lock is held by this caller across the HTTP attempt, so no concurrent writer could have produced a fresher token for the re-read to find — it would observe the same still-expired credential. Spec-letter deviation only.
**Fix:** Bind the timeout outcome without `?` and route both the timeout case and the inner `Err` case through the same re-read block. (Low value; the practical behavior is already correct.)

### 3.12 MINOR: `ProviderCollection::refresh` comment contradicts the atomic-replace implementation

**File:** `crates/opi-ai/src/provider_collection.rs:458-475`
**Cause:** The `Ok(None)` arm comment says "Static provider — keep whatever was there (no change)," but the code unconditionally calls `replace_all_dynamic_catalogs(new_catalogs)`, which does `self.dynamic_catalogs = catalogs` — i.e. it clears the map and installs only the new batch.
**Impact:** If a provider ever transitions from dynamic (returned `Some` previously) to static (`Ok(None)` next), its prior dynamic catalog disappears and the registry falls back to built-ins. Unlikely in practice.
**Fix:** Fix the comment, or — if preserving prior dynamic state for static-returning providers is intended — change the implementation to merge rather than replace.

### 3.13 MINOR: `ProviderRegistry::register_model` does not call `ModelInfo::validate`

**File:** `crates/opi-ai/src/registry.rs:170-194`
**Cause:** `register_model` checks only empty id and override-layer duplicate; it does not call `model.validate()`. A `ModelInfo` with `wire_api=OpenAiCompletions` but `compat=AnthropicMessages` can be registered as an override without error.
**Impact:** `ApiMappedProvider::try_new` catches wire/compat mismatch for mapped catalogs, but the registry override layer (used by extensions) does not. Extension authors can install invalid models that fail at request-build rather than registration. Not a Phase 14 core-path issue (extension surface).
**Fix:** Call `model.validate()` in `register_model` and surface a typed `RegistrationError` on failure. Low priority.

---

## 4. Test-Quality Findings

Test realism is the phase's redemption story: the original phase-exit accepted 20 blockers largely because acceptance commands selected zero tests or named nonexistent targets, and the remediation fixed exactly that. The rebuilt suite genuinely traverses production paths — `run_interactive_tui`, `dispatch_auth_command`, `OAuthProviderRegistry::registry_with_builtins()`, `build_provider_with_oauth`, factory-built concrete streams through `CodingHarness::prompt` — and asserts exact URLs/headers/form-bodies/counts. PKCE S256 is verified against an RFC 7636 test vector. The findings below are refinements, not a return of the vacuous-test problem.

### 4.1 MINOR: `build_provider_production_returns_store_owning_bundle` is a compiles-only test with an empty assertion body

**File:** `crates/opi-coding-agent/tests/provider_factory.rs:399-418`
**Cause:** `fn assert_bundle_output<F>(_: F) where F: Future<Output=…> {}` (empty body); the production function is passed but never awaited/polled — only its return type is checked. The path argument is literally `"unused-unpolled-keyring-path"`.
**Fix:** Rename to make the type-pinning intent explicit, or actually `await` inside a `tokio::runtime` block and assert the bundle owns a store.

### 4.2 MINOR: `assert_optional_u64` helper is a no-op function named `assert_*`

**File:** `crates/opi-ai/tests/usage_cost.rs:11`
**Cause:** `fn assert_optional_u64(_: Option<u64>) {}` discards its argument. The real assertions are the adjacent `assert_eq!(u.cache_write_1h_tokens, None)`, so the invariant IS verified — but the helper is misleading.
**Fix:** Replace with `let _field_type: Option<u64> = …;` or remove the helper and rely on the adjacent `assert_eq!` calls.

### 4.3 MINOR: child>parent `StreamError` rejection not in `usage_cost.rs`; coverage lives only in fixture files

**File:** `crates/opi-ai/tests/usage_cost.rs:1-353`
**Cause:** `usage_cost.rs` tests the data-structure math and accepts the subset relation by construction; the actual child>parent rejection at the provider-mapper layer is in `anthropic_fixtures.rs:404-449`, `openai_chat_fixtures.rs:513-546`, `openai_responses_fixtures.rs:601-624`. Coverage exists but is split across files.
**Fix:** Add a `usage_cost.rs::provider_mapper_rejects_child_greater_than_parent_via_stream_error` test, or a doc comment pointing to the fixture files that own the rejection invariant.

### 4.4 MINOR: `ProviderHeaders` reserved-name test exercises only 6 of 13 reserved names

**File:** `crates/opi-ai/tests/api_mapped_provider.rs:335-349`
**Cause:** The test loops over 6 reserved names; `RESERVED_PROVIDER_HEADERS` also contains `anthropic-beta`, `openai-beta`, `session-id`, `session_id`, `x-client-request-id`, `x-session-affinity`, `x-initiator` — all 7 unpinned.
**Fix:** Iterate over `RESERVED_PROVIDER_HEADERS` directly (expose it `pub` if needed) or expand the literal list to all 13.

### 4.5 MINOR: no negative test for `ApiMapError::RouteCatalogMismatch`

**File:** `crates/opi-ai/tests/api_mapped_provider.rs:244-322`
**Cause:** Construction-rejection tests cover `MissingRoute`, `WireCompatMismatch`, `DuplicateModel`/`DuplicateRoute`/`RouteProviderIdMismatch`. `RouteCatalogMismatch` — the most complex check (route model set must equal the catalog subset for that wire, full `ModelInfo` equality) — has no dedicated failure test.
**Fix:** Add a test constructing a route with a superset/subset of the wire's models, or same ids with different capabilities/wire.

### 4.6 MINOR: mapped-provider dispatch tests use a `RecordingRoute` test double, not real Chat/Anthropic/Responses routes

**File:** `crates/opi-ai/tests/api_mapped_provider.rs:39-94, 162-220`
**Cause:** The dispatch/shared-auth/unknown-model/refresh tests use `RecordingRoute`. Real-route composition (model-id parsing in the inner route, `model_base_url` precedence, `catalog_compat` resolution, real wire-build) is exercised for the `AnthropicMessages` wire at one site only.
**Fix:** Add a wiremock test driving `ApiMappedProvider::stream` through a real `OpenAiChatProvider::for_route` Chat route and verify body/header composition.

### 4.7 MINOR: `LoginTerminalGuard` suspend-failure path has zero test coverage

**File:** `crates/opi-coding-agent/src/interactive_auth.rs:122-136, 236-244`
**Cause:** The "suspend failed but resume succeeded" path (`TerminalGuardError::Suspension` → `Failed` outcome, TUI continues) and "suspend failed and resume also failed" path (`Restore` → loop exits) are implemented but untested.
**Fix:** Add an `outer_tui_terminal_suspend_failure_outcome` test injecting `InteractiveTuiTestTerminalFailure::Suspend` and asserting the correct outcome/exit per path.

### 4.8 MINOR: `oauth_login_restores_terminal_after_flow_failure` test is misleading and asserts nothing about failure recovery

**File:** `crates/opi-coding-agent/src/interactive_auth.rs:314-325`
**Cause:** The `flow_result` binding is a free-floating literal never wired to the guard; the test only verifies guard suspend/resume ordering on a happy path despite its name.
**Fix:** Rename to `login_terminal_guard_orders_suspend_then_resume` and drop the unused binding, or actually drive `dispatch_auth_command` with a failing OAuth and assert the `Failed` outcome.

### 4.9 MINOR: JSON `CredentialNeeded` duplicate-emission not pinned (test uses `find`, not `count`)

**File:** `crates/opi-coding-agent/src/main.rs:1840-1848` (and `:1941-1946`)
**Cause:** The test uses `.find(|line| line["type"]=="CredentialNeeded")` (first match), not a count. Because `run_json` subscribes a session-event listener AND calls `append_credential_remediation` on the `Err` path, a duplicate would still pass. No duplicate is produced today.
**Fix:** Change `find` to a `filter(...).count() == 1` assertion.

### 4.10 INFO: cancellation is tested only for Codex Device-Code (mirrors §3.1)

Once §3.1 is fixed, add parallel cancellation tests for Anthropic PKCE, Codex Browser PKCE, and Copilot device-code.

### Other test-quality infos
- Subprocess-only RPC stdio children are intentionally `#[ignore]`'d across three files (`interactive_auth.rs:1386`, `interactive_tui_auth.rs:832`, `phase14_provider_auth_docs.rs:659`) — **correct pattern**, surfaced only because the audit flags `#[ignore]`.
- `phase14_provider_auth_docs.rs` is a source-scan doc guard, brittle by design; unlike `provider_factory.rs::provider_policy_is_centralized` it has no vacuous-allowlist guard.
- No structural test pins that `expose_secret()` is called only at HTTP boundaries (covered behaviorally by redaction canaries, which is wide but not exhaustive per call site).
- `Agent::retry_last_turn` has no direct `opi-agent` unit test (covered transitively via harness/TUI end-to-end).

---

## 5. Spec-Compliance Summary

| Criterion | Status | Evidence |
|-----------|--------|----------|
| SC1 credential storage + probes | MET | secret-free lock, two-service probe, corrupt-never-absent; minor: corrupt-marker misclassified as BackendUnavailable (§3.7) |
| SC2 OAuth product flows | MET-WITH-GAP | PKCE S256 (RFC 7636), device-code states, `/login`+`/logout` persistence all verified; **cancellation implemented for 1/4 flows** (§3.1) — SC2's "cancellation" sub-claim is only partially met |
| SC3 live auth + session interaction | MET | per-stream auth resolve before HTTP, typed CredentialNeeded/CredentialRevoked non-retryable, outer-TUI same-turn retry exactly-once; minors: revocation-variant precision (§3.3/§3.4) |
| SC4 request + session affinity | MET-WITH-GAP | session_id traverses harness→agent→request, resume/fork replacement verified; **OpenAI Responses over-gates affinity** (§3.2) |
| SC5 capabilities + cache markers | MET | nested `ModelCapabilities`, exact marker positions/TTL, factory-built stream test; minor: non-positive limits unvalidated (§3.8) |
| SC6 usage + cost | MET | subset math, child>parent rejection, no double count, resume round-trip; 25/25 `usage_cost` tests pass |
| SC7 dynamic refresh substrate | MET | deterministic atomic replace, rollback on error, substrate-only (no trigger claimed) |
| SC8 docs + guards | MET | paired EN/ZH docs, source guards, runtime help via subprocess; minor doc/comment drift |

| Non-Goal | Preserved? | Note |
|----------|-----------|------|
| NG1 no plaintext cred file | YES | enforced + tested |
| NG2 no auto-relogin mid-stream | YES | `CredentialRevoked` non-retryable, ends turn |
| NG3 no per-call credential/auth-header override | YES | `validate_extra_headers` rejects reserved names; minor: values not validated (§3.5) |
| NG4 no onPayload/onResponse hooks | YES | absent |
| NG5 no maxRetries/maxRetryDelay on Request | YES | absent (source-guarded) |
| NG6 no end-to-end SecretString migration | YES | deferred as designed |
| NG7 no OAuth providers beyond Anthropic/Copilot/Codex | YES | exactly 3 registered |
| NG8 no session-schema/context-reconstruction redesign | YES | additive fields only |

---

## 6. Invariant Verification

| Invariant | Code evidence | Test coverage |
|-----------|--------------|---------------|
| `CredentialStore` object-safe behind `Arc<dyn CredentialStore>` | `credential.rs:213-235` all methods return `BoxAuthFuture` | provider_collection.rs mock-store test |
| BackendUnavailable distinct from Absent; corrupt never collapses to env | `credential.rs:105-117,147-178`; `credential_store.rs:846-887` | resolver_*_never_falls_back_to_env; unknown_envelope_kind_never_appears_in_display_or_debug |
| OS keychain primary; API-key env fallback only on absence/backend-unavail; OAuth keychain-required | `credential_store.rs:846-1000` | headless_api_key_env_fallback; resolve_oauth_absent_yields_credential_needed |
| Secret-free cross-process lock + acquire-then-re-read + non-reentrant | `credential_store.rs:520-598, 730-752, 677-712` (`write_unlocked`/`delete_unlocked`/`acquire_lock` are `pub(crate)`) | mutation_lock_serializes_concurrent_writers; mutation_lock_times_out_under_contention |
| Refresh: double-check + bounded HTTP + no partial write + preserve prior | `credential_store.rs:907-1000`; `OAUTH_REFRESH_TIMEOUT=30s` | refresh_timeout_releases_lock_and_preserves_prior_credential |
| `expose_secret()` only at HTTP boundary | 4 call sites only (grep-confirmed) | behavioral redaction canaries (wide, not per-call-site exhaustive) |
| Per-stream auth re-resolution; construction never bakes secret | auth.resolve() inside spawned task before HTTP in all 4 providers | factory_built_approved_profiles_resolve_auth_inside_each_stream; factory_stream_reresolves_after_store_change |
| `CredentialNeeded`/`CredentialRevoked` non-retryable + typed (no string match) | `provider.rs:347-353`; `agent_loop.rs:509-525` | provider_error_*_is_non_retryable_auth; factory_built_approved_profiles_map_revocation_without_retry |
| Same-provider retry exactly-once, one user message, two provider calls | `interactive.rs:373-399, 426-431`; `harness.rs:1512`; `agent.rs:223-227` | outer_tui_same_provider_login_retries_pending_turn_once; outer_tui_retry_credential_needed_does_not_rearm_or_retry_twice |
| RAII terminal restore exactly once on every path | `interactive_auth.rs:117-151` (`LoginTerminalGuard` Drop) | dispatcher_restores_terminal_once_on_every_concrete_exit (8 paths); gap: suspend-failure path untested (§4.7) |
| OAuth login cancellation | `await_login_cancelled` wired in `run_codex_device_login` only | **only Codex Device-Code** — gap in 3 flows (§3.1) |
| Construction-ownership: opi-agent constructs no providers, owns no auth | `opi-agent/Cargo.toml` deps only `opi-ai`; grep for concrete providers/keyring/oauth = 0 hits | structural (type system + phase14_provider_auth_docs.rs:505-511 source guard) |
| session_id traverses harness→agent→request; replaced on resume/fork | `harness.rs:931,1251,1362,1946-1949`; `agent.rs:140,394`; `agent_loop.rs:90-105` | phase14_session_affinity_tracks_new_resume_and_fork; session_id_reaches_every_request |
| `ApiMappedProvider` validates route graph at construction; resolves in own catalog; no mapped-layer re-resolve | `api_mapped.rs:99-145, 172-194` | missing_route_fails_at_construction; mapped_routes_share_one_lazy_auth_resolver; gap: RouteCatalogMismatch untested (§4.5) |
| Codex dedicated wire (not Responses flags); account_id required | `openai_codex_responses.rs:22-36, 148-154` | dedicated_codex_request_uses_exact_base_path_body_and_headers; openai_codex_*_rejects_token_without_chatgpt_account_id |
| Usage subset semantics; child>parent rejected; None vs Some(0) preserved | `stream.rs:39-50, 102-107, 185-194, 235-269`; per-mapper rejection | usage_cost.rs 25/25 + fixture-file rejection tests (§4.3) |
| `refresh_models` atomic replace-not-append; substrate-only | `provider.rs:39-41`; `provider_collection.rs:448-475` | refresh_models_is_atomic_substrate; refresh_models_atomic_rollback_on_error; refresh_models_deterministic_ordering |

---

## 7. Cross-Task Integration + Residuals

**Integration is clean.** The `AuthSource` enum (`Baked`/`Store`/`EnvOAuthToken`/`Layered`) composes `CredentialResolver` and `OAuthProvider` correctly across the factory, the four concrete providers, and the dispatcher; the `Layered` precedence (stored OAuth > `ANTHROPIC_OAUTH_TOKEN` > API-key env) is wired at `credential_store.rs:1179-1217` and tested for precedence. `ApiMappedProvider` shares one `Arc<dyn AuthResolver>` across its routes by factory convention (`provider_factory.rs:899-965` Arc::clone per route) — the struct itself doesn't enforce it, but the design explicitly delegates auth to route providers ("mapped layer does not resolve a second time"), so this is intentional layering, not a defect (an independent verifier refuted the "should hold the resolver structurally" candidate). The opi-agent ↔ opi-coding-agent session-affinity handoff (`set_session_id` → `AgentLoopContext` → `Request`) is consistent and resume/fork-aware.

**Residuals and recommendations (priority order):**

1. **(Major) Wire OAuth cancellation into the remaining 3 flows** (§3.1). The trait method, TUI impl, and Codex Device-Code pattern all exist — this is propagation, not new design. While there, reconcile SC2's "cancellation" sub-claim.
2. **(Major) Fix OpenAI Responses session-affinity gating** (§3.2) and update `request_enrichment.rs:437-476`, which currently locks the deviation in.
3. **(Minor cluster) Reconcile the `CredentialRevoked`/`AuthFailed` mapping** with the spec across Anthropic API-key (§3.3) and static-Bearer `openai_compatible` (§3.4). Resolving §3.3 (make Anthropic API-key 401 drop the body → `CredentialRevoked`) also closes the §2.2 redaction residual for that path.
4. **(Minor) `login_oauth` store-write failure surfacing** (§3.9) — one `notify_failure` call + test assertion.
5. **(Minor) doctor `[providers.custom]` probe** (§3.6) — user-facing misleading diagnostic.
6. **(Test-quality)** the §4.1/§4.2 compiles-only and no-op-`assert_*` tests are the closest descendants of the vacuous-test problem that triggered the whole remediation; cheap to fix and they keep the suite honest.
7. **(Info/cleanup)** dead `let _ = &mut fields;` (`credential_store.rs:435`), the contradictory `anthropic.rs:1046-1051` and `provider_collection.rs:458-475` comments, the `build_provider` legacy sync path that cannot dispatch `github-copilot`/`openai-codex` (§G8 — document or delegate), and the listing-path build-and-discard `HttpClient` for proxy validation.

**Non-blocking carry-forwards (do not block archive):** §3.5 (header-value validation), §3.7 (corrupt-marker classification), §3.8 (non-positive limits), §3.10 (TUI provider_id in messages), §3.11 (refresh-timeout re-read — moot), §3.12 (refresh comment), §3.13 (extension `register_model` validation), §2.3 (Copilot device error echo), and the test-coverage gaps in §4.3–§4.6.

**Overall assessment.** The two Majors are genuine spec deviations worth fixing before archive, but neither is a security vulnerability, data-loss path, or crash; the credential/OAuth security core — the part that matters most — is the strongest part of the phase and is tested with real redaction canaries rather than vacuous asserts. The remediation's central lesson (no zero-selection acceptance commands) has held: the rebuilt suite traverses production paths and asserts exact wire shapes. **PASS-WITH-FINDINGS.**
