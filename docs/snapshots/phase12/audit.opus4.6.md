# Phase 12 Provider Correctness -- Independent Code Audit

**Auditor model:** Claude Opus 4.6 (Cursor)  
**Date:** 2026-07-03  
**Commit range:** `f3a61f32..9c94b47d` (9 commits, 54 files, +6228/-132 lines)  
**Scope:** Tasks 12.1--12.9 as defined in `docs/superpowers/specs/2026-06-24-phase12-provider-correctness-design.md`  
**Contamination status:** No existing review reports, AI evaluations, or human review records were read during this audit.

---

## 1. Executive Summary

Phase 12 delivers a well-structured provider correctness layer across 9 commits.
The error taxonomy, retry/cancel guards, safe-excerpt redaction, and fixture
infrastructure are sound in design and largely correct in implementation.

**Verdict:** PASS with 2 MAJOR, 5 MEDIUM, 8+ MINOR, and 5+ INFO findings.

| Severity | Count | Summary |
|----------|-------|---------|
| MAJOR | 2 | Bedrock stream termination bug; retry-exhausted mislabel |
| MEDIUM | 5 | Bedrock ToolCallDelta sync; Cancelled routing; Responses Completed dead loop; CompatConfig dead fields; profile flags lack wire tests |
| MINOR | 8+ | Regex pattern gaps; response_id edge case; cancel test vacuousness; proxy test shallowness; etc. |
| INFO | 5+ | Reserved variants without producers; test_support compilation scope; doc guard brittleness |

---

## 2. Correctness Findings

### 2.1 MAJOR: Bedrock `stream_http` does not flush pending Done on metadata absence

**File:** `crates/opi-ai/src/bedrock/mod.rs`  
**Lines:** 364--370 vs 164--167  
**Cause:** The `stream_from_fixture` path (L164--167) calls `mapper.flush_pending()` after the stream loop to emit any deferred `Done` event. The production `stream_http` path (L364--370) only checks `!mapper.saw_done` and emits a `StreamError` if the stream ended without a terminal event, but does NOT call `flush_pending()`. When the Bedrock stream delivers `messageStop` (setting `saw_done = true` and storing a `pending_done`) but the subsequent `metadata` event never arrives, `pending_done` is never flushed -- the caller never receives a `Done` event.  
**Impact:** Callers waiting for `Done` (the agent loop's `process_stream_event` returning `Some(msg)`) may never see the complete assistant message. The stream terminates silently without error or completion signal.  
**Fix:** Add `mapper.flush_pending(&tx).await;` after the stream loop in `stream_http`, before the `!mapper.saw_done` check.

### 2.2 MAJOR: Retry-exhausted mislabel when partial-content guard fires after prior retries

**File:** `crates/opi-agent/src/agent_loop.rs`  
**Lines:** 454--477 vs 396--399  
**Cause:** The retry failure path (L454--477) emits `AutoRetryEnd { success: false }` and diagnostic `CODE_PROVIDER_RETRY_EXHAUSTED` whenever `retry_attempt > 0` and the retry condition is not met. This fires when `stream_delivered_content == true` blocks a retry on the second attempt, even though retry attempts were NOT exhausted -- the partial-content guard prevented it.  
**Impact:** Diagnostic consumers (NDJSON, RPC, traces) receive `provider_retry_exhausted` when the actual cause is `partial_content_no_retry`. Misleading for debugging and monitoring.  
**Fix:** Split the L454--477 block into two paths: one for `retry_attempt >= max_attempts` (genuinely exhausted) and one for `stream_delivered_content` (partial-content guard), emitting a distinct diagnostic code for the latter.

### 2.3 MEDIUM: Bedrock ToolCallDelta does not update `partial.content` arguments

**File:** `crates/opi-ai/src/bedrock/mod.rs`  
**Lines:** 762--774 vs text delta handling  
**Cause:** Text deltas update both `blocks[].partial_text` AND `partial.content` (the public-facing accumulator). Tool call deltas at L762--774 only update `blocks[idx].partial_input`, not the corresponding `ToolCall.arguments` in `partial.content`.  
**Impact:** Mid-stream consumers reading `partial.content` see stale tool call arguments until `contentBlockStop` triggers the final write-back. This is unlike the Anthropic adapter which accumulates arguments incrementally.  
**Fix:** After updating `blocks[idx].partial_input`, also update the corresponding `partial.content` ToolCall entry's `arguments` field.

### 2.4 MEDIUM: ProviderError::Cancelled routes to AgentError::Provider

**File:** `crates/opi-agent/src/agent_loop.rs`  
**Lines:** 485--490  
**Cause:** The `Err(e)` branch at L485--490 handles `AuthFailed` specially, but all other variants (including `Cancelled`) fall through to `AgentError::Provider(e.to_string())`. The harness-initiated cancel path (via `cancel.cancelled()`) correctly returns `AgentError::Cancelled`, but a provider that returns `ProviderError::Cancelled` (e.g., a custom provider detecting external cancellation) gets classified as a generic provider error.  
**Impact:** A provider-returned cancellation appears as a failure rather than a graceful cancel in the agent's error hierarchy. The diagnostic code would be `provider_error` instead of `agent_cancelled`.  
**Fix:** Add a match arm for `ProviderError::Cancelled` returning `AgentError::Cancelled`.

### 2.5 MEDIUM: Responses `Completed` handler does not emit ToolCallEnd

**File:** `crates/opi-ai/src/openai_responses.rs`  
**Lines:** 514--541  
**Cause:** The `Completed` event handler iterates over unclosed tool calls and updates `partial.content`, but never emits a `ToolCallEnd` event. This loop is documented as a fallback for streams that end without `output_item.done`, but it only updates internal state without notifying downstream consumers.  
**Impact:** If `output_item.done` is missing (e.g., a truncated Responses stream), the tool call appears permanently open to the agent loop. The agent may not attempt to execute the tool.  
**Fix:** Either emit `ToolCallEnd` for each finalized tool call in the loop, or remove the dead loop and rely on the subsequent `StreamError` for incomplete streams.

### 2.6 MEDIUM: CompatConfig contains two dead fields

**File:** `crates/opi-ai/src/openai_chat.rs`  
**Lines:** 588, 596 (definitions); 605, 609 (defaults)  
**Fields:** `usage_in_stream: bool`, `require_assistant_after_tool_result: bool`  
**Cause:** Both fields are defined, documented, defaulted, and configurable from TOML, but have zero runtime consumers. No code path reads these values after construction.  
**Impact:** Configuration changes to these fields silently have no effect. Users may believe they are activating behavior that does not exist.  
**Fix:** Either implement the runtime semantics (e.g., `usage_in_stream` controlling whether usage chunks are expected/merged during streaming) or remove the fields and update the CompatConfig documentation and TOML schema.

### 2.7 MEDIUM: Phase 12 profile flags lack wire-level factory tests

**File:** `crates/opi-coding-agent/tests/provider_factory.rs`  
**Lines:** 472--552  
**Cause:** The `openai_compatible_profile_overrides` test group verifies config parsing and that `build_provider` succeeds, but does not use wiremock to assert that `strict_tool_schema`, `reasoning_effort`, `cache_key`, or `extra_headers` actually appear in the outgoing HTTP request body/headers.  
**Impact:** A regression that breaks the config-to-wire path for these flags would not be caught by the factory test suite. The wire assertions exist only in `openai_chat_fixtures.rs` at the provider-unit level.  
**Fix:** Add at least one wiremock-based factory integration test that configures a profile with these flags and asserts the HTTP request body shape.

---

## 3. Security / Redaction Findings

### 3.1 MINOR: Regex pattern divergence between safe_excerpt and SecretRedactor

**Files:** `crates/opi-ai/src/http.rs` L252--271; `crates/opi-agent/src/streaming_proxy.rs` L334--351  

| Pattern | safe_excerpt | SecretRedactor |
|---------|:---:|:---:|
| `sk-[A-Za-z0-9-]{20,}` | yes | yes |
| `sk-ant-[a-zA-Z0-9-]{20,}` | no (covered by generic sk-) | yes |
| `gh[opsu]_` | yes | no |
| `gh[pousr]_` | no | yes |
| `AIza[0-9A-Za-z_-]{35,}` | yes | **no** |
| Credentialed URL | yes | yes |
| JWT | yes | yes |

**Specific gaps:**
- `safe_excerpt` uses `gh[opsu]_` -- misses `ghr_` (GitHub refresh tokens).
- `SecretRedactor` uses `gh[pousr]_` -- misses GitHub App server tokens (`ghs_`) vs the safe_excerpt pattern.
- `AIza` (Google API keys) exists only in `safe_excerpt`. A Google API key appearing in a diagnostic message (not an HTTP response body) would not be caught by the agent-layer redaction.

**Mitigation:** The Gemini provider sends API keys via `x-goog-api-key` header, not in URLs. Network errors from reqwest do not include request headers. The risk is theoretical but the inconsistency should be unified.

### 3.2 MINOR: Network error messages bypass safe_excerpt

**File:** `crates/opi-ai/src/anthropic.rs` L744; `openai_chat.rs` L914; `gemini.rs` L701  
**Pattern:** `ProviderError::Network(e.to_string())` where `e` is a `reqwest::Error`.  
**Cause:** `safe_excerpt` is applied only to HTTP response bodies (AuthFailed, ProviderSide paths). Network errors use reqwest's Display impl directly. If a proxy URL with embedded credentials causes a connection error, reqwest may include the URL in its error message.  
**Mitigation:** The `redacted_for_public` layer applies `redact_text` (via `SecretRedactor`) which catches credentialed URL patterns. Defense-in-depth is present but not at the adapter layer.

### 3.3 INFO: redacted_for_public catch-all passes through 3 event types

**File:** `crates/opi-agent/src/event.rs` L198  
**Pattern:** `other => other.clone()` matches `AgentStart`, `TurnStart`, `QueueUpdate`, `CompactionStart`, `CompactionEnd`, `SessionPersistError`.  
**Risk:** `CompactionEnd.error_message` and `SessionPersistError.message` are unredacted. These contain filesystem/session errors, not provider secrets.  
**Status:** Documented in 12.2 session notes as intentional (not provider errors). Acceptable for Phase 12 scope.

---

## 4. Test Quality Assessment

### 4.1 Coverage Matrix

| Provider | Lifecycle | Request body | Error classes | Tool calls | Usage/cache | Thinking | Cancel | Retry |
|----------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| Anthropic | yes | yes | yes | yes (deep) | yes | yes | yes (2s timeout) | yes |
| OpenAI Chat | yes | yes | yes | yes (deep) | yes | N/A | yes | yes |
| OpenAI Responses | yes | yes | yes | yes | yes | N/A | yes | yes |
| OpenRouter | yes | yes | yes | inherited | inherited | N/A | yes | yes |
| Mistral | yes | yes | yes | inherited | inherited | N/A | yes | yes |
| Gemini | yes | yes | yes | N/A | yes | N/A | yes | yes |
| Azure | yes | yes | yes | inherited | inherited | N/A | yes | yes |
| Bedrock | partial | yes | yes | yes | yes | negative | negative | yes |
| Vertex | yes | yes | yes | inherited | inherited | N/A | yes | yes |

### 4.2 MINOR: provider_lifecycle.rs is misleadingly named

**File:** `crates/opi-ai/tests/provider_lifecycle.rs`  
**Issue:** File name suggests cross-provider lifecycle coverage but only tests Anthropic. Other providers have separate `*_lifecycle.rs` files; Bedrock/Azure/Vertex have none -- their lifecycle coverage is in `*_fixtures.rs` only.

### 4.3 MINOR: Cancel test has no effective assertion

**File:** `crates/opi-ai/tests/provider_lifecycle.rs` L332--370  
**Issue:** The cancellation test at L367--369 uses `let _ = got_terminal;` -- it does not assert whether cancellation was observed or whether the stream terminated cleanly. This is a "does not panic" test, not a behavioral assertion.

### 4.4 MINOR: EventsThenError lacks direct unit test

**File:** `crates/opi-ai/tests/test_support_tests.rs`  
**Issue:** Tests cover `MockResponse::Events` and `error_response()` but never directly test `EventsThenError`. The variant is only exercised transitively through `retry_agent.rs`. A regression in the chain construction would not be caught at the unit level.

### 4.5 MINOR: proxy_support.rs tests only configuration, not behavior

**File:** `crates/opi-ai/tests/proxy_support.rs`  
**Issue:** Tests verify `proxy_config()` returns the expected `ProxyConfig` struct. No test verifies that HTTP requests are actually routed through the proxy, or that `NO_PROXY` patterns correctly bypass it.

### 4.6 MINOR: usage_cost.rs is disconnected from provider normalization

**File:** `crates/opi-ai/tests/usage_cost.rs`  
**Issue:** Tests cover `Usage` struct arithmetic, `CumulativeUsage`, and `Pricing` calculations, but do not test provider-side usage extraction from SSE streams. Provider usage normalization is tested only in the per-provider `*_fixtures.rs` files. The file name suggests broader coverage than it provides.

### 4.7 INFO: retry_backoff.rs has wall-clock-dependent assertions

**File:** `crates/opi-ai/tests/retry_backoff.rs` L37--50, L251--263, L286--300  
**Issue:** Several tests use range assertions on durations computed from wall-clock time (e.g., `55_000..=65_000` ms). Windows are wide enough for normal CI but could flake under extreme load.

### 4.8 Mock Fidelity Assessment

**Strengths:**
- Wiremock-based tests use realistic SSE format matching production providers
- Anthropic fixtures include CRLF handling, malformed events, multi-tool streaming
- Tool result wire tests use byte-level assertions on serialized request bodies

**Weaknesses:**
- `provider_error_classes.rs` uses a single `method("POST")` matcher without path discrimination (L52)
- Bedrock binary event-stream format is not fixture-mountable; cancel test is a named negative

---

## 5. Spec Compliance

### 5.1 Success Criteria Trace

| SC# | Spec requirement | Status | Evidence |
|-----|-----------------|--------|----------|
| SC1 | Every provider has fixture coverage for request, streaming, and error mapping | MET | 9 families in `*_fixtures.rs` + `provider_lifecycle.rs` |
| SC2 | Provider errors map into documented taxonomy | MET | 9-class `ProviderErrorCategory`; `From<&ProviderError> for Diagnostic` |
| SC3 | Tool call conversion tested for tool-capable providers | MET | 4 distinct-wire adapters (Anthropic/Chat/Responses/Bedrock); inheritance families share path |
| SC4 | Thinking and image capability checks tested and documented | MET | Anthropic thinking lifecycle; Bedrock negative; image preflight |
| SC5 | Retry behavior covered without live calls | MET | `retry_backoff.rs` + `retry_agent.rs` + per-family cancel fixtures |
| SC6 | OpenAI-compatible profiles have fixture coverage | MET | CompatConfig 8 flags + ResponsesConfig + wire fixtures |
| SC7 | Config-driven profiles remain preferred path | MET | `config_driven_compatible_profiles_are_preferred` + `first_class_provider_guard` |
| SC8 | Diagnostics/traces include error class and safe metadata | MET | `trace_envelope.rs` + JSON/RPC redaction tests |
| SC9 | No OAuth, subscription auth, image generation, or broad catalog expansion | MET | Non-goal guards + structural absence checks |

### 5.2 Documented Residuals (from impl-state, verified present in code)

| Residual | Status | Notes |
|----------|--------|-------|
| SC3 inheritance: azure/openrouter/mistral tool-call coverage via shared adapter | Accepted | Design choice; shared openai_chat wire |
| Bedrock cancel is named negative | Accepted | Binary event-stream not fixture-mountable |
| rpc_jsonl parallel flake | Pre-existing | Mitigated with `--test-threads=1` |
| RequestFailed has no producer | Accepted | Reserved for local pre-request validation |
| CompactionEnd/SessionPersistError unredacted | Accepted | Not provider errors |

### 5.3 Undocumented Gaps (not in residuals but found by this audit)

| Gap | Severity | Location |
|-----|----------|----------|
| Bedrock `stream_http` pending_done not flushed | MAJOR | `bedrock/mod.rs` L364--370 |
| Retry-exhausted mislabel on partial-content guard | MAJOR | `agent_loop.rs` L454--477 |
| ProviderError::Cancelled misrouted to AgentError::Provider | MEDIUM | `agent_loop.rs` L485--490 |
| Responses Completed ToolCallEnd not emitted | MEDIUM | `openai_responses.rs` L514--541 |
| CompatConfig dead fields mislead users | MEDIUM | `openai_chat.rs` L588, L596 |
| Profile flags lack wire factory test | MEDIUM | `provider_factory.rs` tests |
| response_id lost when first chunk has content but no role | MINOR | `openai_chat.rs` L279--291 |

---

## 6. Residuals and Recommendations

### 6.1 Priority Fixes (recommended before Phase 13)

1. **Bedrock flush_pending** -- Add `mapper.flush_pending(&tx).await;` after the stream loop in `stream_http`. Risk: low (single call site, test already exists for fixture path).

2. **Retry mislabel** -- Introduce a `partial_content_no_retry` diagnostic code distinct from `retry_exhausted`. Risk: low (additive diagnostic code).

3. **ProviderError::Cancelled routing** -- Match `ProviderError::Cancelled` explicitly in agent_loop error path. Risk: low.

### 6.2 Recommended Improvements (non-blocking)

4. **Responses Completed ToolCallEnd** -- Either emit events or remove the dead loop.
5. **CompatConfig dead fields** -- Document as metadata-only or implement runtime semantics.
6. **Profile wire factory test** -- Add one wiremock test through the full config-to-HTTP path.
7. **Regex unification** -- Align `safe_excerpt` and `SecretRedactor` GitHub token patterns; add `AIza` to SecretRedactor.
8. **response_id resilience** -- Extract `raw.id` on content-only chunks (not just role chunks) in OpenAI Chat mapper.
9. **Cancel test assertion** -- Replace `let _ = got_terminal;` with an actual assert or timeout-bounded drain.
10. **EventsThenError unit test** -- Add a direct stream-shape assertion in `test_support_tests.rs`.

### 6.3 Accepted As-Is

- `RequestFailed` / `Cancelled` having no adapter producers (taxonomy reservations for future use)
- `test_support` compiled into release (accepted for cross-crate test ergonomics; `#[doc(hidden)]`)
- Wall-clock assertions in retry tests (wide windows, acceptable CI risk)
- Azure 404 -> Config mapping (intentional Azure-specific behavior)
- proxy tests limited to configuration (behavioral proxy testing requires external proxy fixture)

---

## Appendix A: Files Reviewed

### Source (production)
- `crates/opi-ai/src/provider.rs` (full)
- `crates/opi-ai/src/http.rs` (full)
- `crates/opi-ai/src/openai_chat.rs` (full)
- `crates/opi-ai/src/openai_responses.rs` (full)
- `crates/opi-ai/src/azure_openai.rs` (full)
- `crates/opi-ai/src/bedrock/mod.rs` (full)
- `crates/opi-ai/src/gemini.rs` (grep + key sections)
- `crates/opi-ai/src/vertex.rs` (grep)
- `crates/opi-ai/src/anthropic.rs` (grep + error paths)
- `crates/opi-ai/src/test_support.rs` (full)
- `crates/opi-ai/src/lib.rs` (public API surface)
- `crates/opi-agent/src/agent_loop.rs` (L65--520 retry/cancel/stream)
- `crates/opi-agent/src/diagnostic.rs` (full)
- `crates/opi-agent/src/event.rs` (full)
- `crates/opi-agent/src/streaming_proxy.rs` (SecretRedactor)
- `crates/opi-coding-agent/src/provider_factory.rs` (full)
- `crates/opi-coding-agent/src/config.rs` (profile sections)

### Tests
- `crates/opi-ai/tests/provider_lifecycle.rs` (full)
- `crates/opi-ai/tests/anthropic_fixtures.rs` (full)
- `crates/opi-ai/tests/retry_backoff.rs` (full)
- `crates/opi-ai/tests/proxy_support.rs` (full)
- `crates/opi-ai/tests/provider_error_classes.rs` (full)
- `crates/opi-ai/tests/tool_result_wire.rs` (full)
- `crates/opi-ai/tests/usage_cost.rs` (full)
- `crates/opi-ai/tests/test_support_tests.rs` (referenced)
- `crates/opi-coding-agent/tests/provider_factory.rs` (full)
- `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs` (full)

### Documentation and Config
- `docs/superpowers/specs/2026-06-24-phase12-provider-correctness-design.md` (full)
- `docs/snapshots/phase12/opi-impl-state.json` (full)
- Git log and diff stats for commit range

## Appendix B: Methodology

1. Read the design spec and implementation state in full.
2. Dispatched 6 parallel read-only exploration agents covering:
   - Error taxonomy (`provider.rs`, `http.rs`)
   - Adapter implementations (openai_chat, openai_responses, azure, bedrock)
   - Agent loop diagnostics and retry (`agent_loop.rs`, `diagnostic.rs`, `event.rs`)
   - Test fixture quality (7 test files)
   - Factory and config wiring (`provider_factory.rs`, `config.rs`, guard tests)
   - Mock infrastructure (`test_support.rs`, `lib.rs`)
3. Direct grep/read verification of key code paths (redaction chain, cancel timing, retry logic).
4. Cross-referenced findings against spec success criteria and documented residuals.
5. Classified findings by severity using the project's existing convention (BLOCKER/MAJOR/MINOR/INFO).
6. No existing review reports were consulted; conclusions derive solely from code, tests, config, and diff.
