# Phase 12 (Provider Correctness) — Independent Audit

- **Reviewer:** GLM-5.2 (workflow: 8 dimension reviewers → per-finding adversarial verifiers → completeness critic; 38 subagents, ~1.6M tokens)
- **Date:** 2026-07-03
- **Scope:** opi Phase 12, commits `f3a61f3..31ad709` (v0.6.3 baseline → archive). Nine tasks (12.1–12.9), ~7,940 insertions / 132 deletions across 55 files. Production logic is concentrated in `opi-ai/src/{provider,http,openai_chat,openai_responses,anthropic,gemini,bedrock/mod,azure_openai,vertex,test_support}.rs`, `opi-agent/src/{agent_loop,diagnostic,event}.rs`, and `opi-coding-agent/src/{config,provider_factory}.rs`; the remainder (~6,000 lines) is tests and docs.
- **Sources:** code at HEAD (`31ad709`), tests, `Cargo.toml`/`Cargo.lock`, `docs/opi-spec.md`, `README*`/`crates/*/README*`, the design spec, and `git diff`. **No prior audit/review/acceptance report was read** (see [Containment & Independence Disclosure](#containment--independence-disclosure)).

## Verdict

Phase 12 is **archived-ready on behavior**: every production path I traced is correct on its own terms, the taxonomy is exactly the 9 spec classes, the one material production change (the no-retry-after-partial-content guard) is sound, and the test suite is broad. However the phase ships **two real Major defects** and a long tail of minor test-quality, redaction-defense-in-depth, doc-drift, and bookkeeping gaps. None are crashes, secret leaks, or blockers.

Counts: **0 Blocker · 2 Major · ~16 Minor · ~10 Info.**

The highest-leverage fixes are concentrated in three places: (1) make the `usage_in_stream` config knob honest, (2) add one end-to-end `Network` retry test, and (3) tighten the eight per-family cancellation tests that today prove only "does not hang."

### Headline strengths (independently confirmed)

- **Error taxonomy is exact.** `ProviderErrorCategory` has exactly the 9 spec classes (`provider.rs:198-218`); every variant has a stable `category()` mapping (`provider.rs:165-177`) and a one-arm-per-class `From<&ProviderError> for Diagnostic` bridge (`diagnostic.rs:605-680`). The 4xx-classification concern is resolved correctly: every adapter's `map_http_status` routes 400/404/409/422 into the `code => ProviderSide` default arm (e.g. `anthropic.rs:865-867`, `openai_chat.rs:1026-1027`, `gemini.rs:828-832`, `bedrock/mod.rs:976-979`), so server-side validation errors land in the spec-correct `provider` class, never `request`. OpenRouter/Mistral inherit the shared classifier (`openrouter.rs:8,21`; `mistral.rs:18`).
- **The retry guard is correct.** `stream_delivered_content` (`agent_loop.rs:125`) is set on any delivered stream item (`:146`) and the retry condition requires `!stream_delivered_content` (`:398`); no off-by-one, no cross-attempt reset. Treating `Start` (which emits a public `MessageStart`) as "content delivered" matches the spec intent of avoiding a second `Start` plus duplicated content.
- **response_id genuinely round-trips** into `AssistantMessage::response_id` for OpenAI Chat (`chatcmpl-*` via `RoleDelta`) and Responses (`resp_*` via Created/Completed events).
- **Request-side secrets cannot reach the classifier.** Adapters interpolate only the response body into error messages; `Bearer`/`x-goog-api-key` headers and request URLs never enter `map_http_status`. `safe_excerpt` (`http.rs:281-295`) additionally scrubs `sk-*`/`gh[opsu]_*`/`github_pat_*`/`AIza*`/JWT/`Bearer …`/credentialed-URL userinfo and caps length at 256 chars. AutoRetry `error_message`/`final_error` are scrubbed at the public boundary.

---

## Major Findings

### M1. `usage_in_stream` is dead config — TOML-exposed and README-advertised, with zero wire effect

- **Task / spec:** 12.3, SC6 ("config-driven profiles represent flag-expressible differences")
- **Files:** `crates/opi-ai/src/openai_chat.rs:567,588,605`; `crates/opi-coding-agent/src/config.rs:193`; `crates/opi-coding-agent/src/provider_factory.rs:571`; `crates/opi-ai/README.md:135`; `README.zh.md:127`; test `crates/opi-ai/tests/openai_chat_fixtures.rs:580-587`
- **Root cause:** `CompatConfig.usage_in_stream: bool` is declared (`openai_chat.rs:588`), defaulted to `false` (`:605`), threaded from TOML (`config.rs:193`) through `provider_factory.rs:571`, and **never read by any production code**. The serializer `build_request_body` (`openai_chat.rs:799-866`) and the event handler `OpenAiChatMapper::process` (`:328-525`, Finish handling at `:444-514`) consume usage unconditionally from any usage-bearing event regardless of the flag. Critically, the actual OpenAI wire mechanism `stream_options: {include_usage: true}` appears **nowhere** in `crates/opi-ai/src` (verified by grep).
- **Impact:** Setting `usage_in_stream = true` in a profile changes no behavior. The README row (`:135` "Expect usage deltas in the streaming response") and the struct doc-comment (`:567` "whether usage appears in every chunk vs only the last") both promise an effect that does not exist. Embedders configuring streaming-usage behavior and future maintainers are misled; SC6 is violated for this specific flag. The sole test (`compat_config_usage_in_stream`) is a pure struct round-trip (`assert!(config.usage_in_stream)`) — it would still pass if the field were deleted, so it masks the gap.
- **I confirmed this myself:** grep for `usage_in_stream` in `crates/opi-ai/src` returns only the doc-comment (`:567`), the field decl (`:588`), and the default (`:605`); grep for `stream_options|include_usage` in `crates/opi-ai/src` returns no matches.
- **Fix:** Either (a) make the flag honest — emit `stream_options: {include_usage: true}` when set, with a wire-body test; or (b) reword the README/doc-comment to "Reserved/metadata-only; the modern adapter consumes usage from any event unconditionally" and downgrade the test to a documented-as-inert marker. Option (a) is preferred because the README currently promises a real wire effect.

### M2. `Network` errors became retryable on every provider, but no `agent_loop` test exercises a `Network` retry

- **Task / spec:** 12.2 (taxonomy) interacting with 12.7 (retry behavior); SC5/SC8
- **Files:** `crates/opi-ai/src/provider.rs:152-156` (`is_retryable`); transport-error mapping rewritten on all 7 native families — `anthropic.rs:744`, `openai_chat.rs:914`, `openai_responses.rs:918`, `gemini.rs:701`, `vertex.rs:220`, `azure_openai.rs:258`, `bedrock/mod.rs:316`; `crates/opi-agent/tests/retry_agent.rs`; `crates/opi-ai/tests/retry_backoff.rs:208`
- **Root cause:** Phase 12 expanded `is_retryable()` from `{RateLimited, Timeout}` (baseline `f3a61f3`, `provider.rs:132-136`) to `{RateLimited, Timeout, Network(_)}` (HEAD `provider.rs:152-156`). In the same phase, every native family's transport-error `map_err` site was rewritten from `ProviderError::RequestFailed(...)` (non-retryable) to `ProviderError::Network(...)` (retryable). **Net effect: every DNS/TLS/connection-reset failure on every provider now triggers an automatic `agent_loop` retry where it previously did not.** This is the single most consequential cross-provider behavior change in Phase 12.
- **Impact:** The change is plausibly desirable, but it is verified only by the unit assertion `network_error_is_retryable` (`retry_backoff.rs:208`), which checks a predicate's return value. `retry_agent.rs` programs `MockProvider` exclusively with `RateLimited` (8 sites: `:94,177,180,183,265,294,356,415`) and `Timeout` (`:232`); **zero `Network` occurrences** (verified by grep). A behavioral regression in the `Network` retry path — e.g. the new `stream_delivered_content` guard suppressing a legitimate `Network` retry, or backoff-timing differences between `Network` and `RateLimited` — would ship unobserved.
- **I confirmed this myself:** baseline vs HEAD `is_retryable` diff (above) and the `retry_agent.rs` grep (only `RateLimited`/`Timeout`).
- **Fix:** Add an `agent_loop` retry test that programs `MockProvider` with `MockResponse::Error(ProviderError::Network(...))` followed by a successful response and asserts `AutoRetryStart` / `AutoRetryEnd(success=true)` and the final assistant message. Optionally add an `EventsThenError(Network)` case to confirm the partial-content guard also suppresses retry for `Network`.

---

## Findings by Theme

### Secret redaction (defense-in-depth)

**R1. `CompactionEnd.error_message` and `SessionPersistError.message` bypass `redacted_for_public` and reach NDJSON stdout unredacted** — Minor (security defense-in-depth)
- `crates/opi-agent/src/event.rs:93-100,104-200`; catch-all at `:198`; `crates/opi-coding-agent/src/runner.rs:234-254`
- `redacted_for_public` has explicit arms for `AutoRetryStart.error_message` (`:185`) and `AutoRetryEnd.final_error` (`:194-196`) but `CompactionEnd` and `SessionPersistError` fall through `other => other.clone()` (`:198`) verbatim. The JSON-mode runner subscriber (`runner.rs:244`) then clones `error_message.clone()` into `AgentSessionEvent` and serializes to the NDJSON stdout buffer (`:250-254`) with no re-redaction. (The non-interactive stderr path via `format_persist_errors` at `runner.rs:636` does re-redact, so only the JSON-mode stdout surface is exposed.)
- **Impact:** Today the harness only ever sets these to local-IO strings (e.g. `"compaction persist failed: {e}"` at `harness.rs:1249,1260,1285,1298`), so secret risk is low. But the surface is unguarded: a future change that interpolates a provider response body or credentialed URL into a compaction/persist error would leak straight to public NDJSON, and no test exercises these variants through the public boundary (`trace_envelope.rs:2234` covers only AutoRetry).
- **Fix:** Add explicit `CompactionEnd`/`SessionPersistError` arms in `redacted_for_public` (redact via `redact_text Summary`), and add a test that pushes a secret-bearing `CompactionEnd` through the public boundary. Defense-in-depth: also re-redact in the runner subscriber before serializing.

**R2. `safe_excerpt` is a prefix denylist; opaque / non-prefixed URL-query secrets are not scrubbed** — Minor
- `crates/opi-ai/src/http.rs:252-295`; tests `crates/opi-ai/tests/provider_error_classes.rs:29,51-80`
- `safe_excerpt` scrubs only `sk-*`/`gh[opsu]_*`/`github_pat_*`/`AIza*`/JWT/`Bearer`/credentialed-userinfo. It does **not** redact URL query-string secrets (`?api_key=opaque`, `?token=…`) unless the value matches a known prefix, nor arbitrary header values. The diagnostic-layer Summary redactor uses the same regex family, so defense-in-depth catches the same set, not more.
- **Impact:** Spec line "Do not log secrets from … URLs" is only partially honored (userinfo caught, query secrets not). Blast radius is bounded because providers do not typically echo the request URL in error bodies, and Gemini/Vertex send keys in headers. But `provider_error_classes.rs` only tests 5xx bodies echoing an `sk-`-prefixed key — a body echoing `?key=opaque` would survive.
- **Fix:** Extend `safe_excerpt`/`SecretRedactor` with a query-param redactor for common secret-bearing keys (`api_key`, `token`, `access_token`, `key`, `secret`, `password`), or document as an accepted-denied gap and add a test pinning the current behavior.

**R3. `From<&ProviderError> for Diagnostic` embeds the raw message; safety depends on every caller re-running `redacted_payload`** — Info
- `crates/opi-agent/src/diagnostic.rs:605-680`
- Each arm embeds `.details(json!({ "provider_error": message }))` with the raw message; the bridge does no redaction itself. No current leak (every public caller runs `redacted_payload(Summary)` first, and the trace mirror at `agent_loop.rs:482` routes through a redacting sink), but the bridge is a tempting single-point redaction site that currently does nothing.
- **Fix:** Optionally pre-redact in the bridge, or add a doc comment warning that details are unredacted and callers must run `redacted_payload` at the public boundary.

**R4. `provider_error_classes.rs` redaction coverage is only the 5xx arm with an `sk-`-prefixed secret** — Minor
- `crates/opi-ai/tests/provider_error_classes.rs:51-80`
- All 9 per-family tests reuse `mount_500_echoing_secret` (HTTP 500 only). No per-family test exercises a 401/403 body with a secret (auth arm), a non-prefixed opaque secret, or a `StreamError` mid-flight SSE error chunk. Mutation: removing `safe_excerpt(body)` from any adapter's 401 arm is not caught by this file.
- **Fix:** Add a parametric helper that mounts `{401-with-secret, 5xx-with-secret}` per family and asserts both class and redaction.

### Error taxonomy

**T1. `ProviderError::RequestFailed` / the `request` class has zero production producers** — Info
- `crates/opi-ai/src/provider.rs:131,169`; `crates/opi-agent/src/diagnostic.rs:623`; the only pre-request validator `validate_request_capabilities` (`provider.rs:84-115`) returns only `UnsupportedCapability`.
- The `request` class is documented as "local pre-request schema/validation failure" but is unreachable at runtime; `RequestFailed` is constructed only inside test files (`provider_diagnostics.rs:123`, `retry_backoff.rs:221`, `trace_envelope.rs:1027`).
- **I confirmed this myself:** grep for `RequestFailed` in `crates/opi-ai/src` returns only the decl (`:131`), the category arm (`:169`), the diagnostic bridge arm, and an unrelated `ConfigError` variant at `config.rs:24`.
- **Fix:** Either wire `RequestFailed` to a real pre-request schema validator, or document `request` as an intentionally-reserved slot (mirroring how `cancelled` is documented at `provider.rs:215-217`) and add a code comment so the spec's "exactly 9 classes" stays honest without claiming live coverage.

### Usage / cost / response-id normalization

**U1. Absent provider usage yields cost `$0.0` instead of an explicit unknown** — Minor (spec-principle gap)
- `crates/opi-ai/src/stream.rs:32-151`; `crates/opi-ai/src/message.rs:35`; `crates/opi-coding-agent/src/session_coordinator.rs:455-461`
- `Usage` is a non-Option struct, `AssistantMessage.usage` is non-Option, and `calculate_cost` returns a non-Option `CostBreakdown`. When a provider emits no usage object, mappers leave `Usage::default()` (e.g. `anthropic.rs:270` `unwrap_or_default`, `openai_chat.rs` Finish with `usage=None`), which flows into `calculate_cost` → `CostBreakdown::default()` → `total_cost() == 0.0`. The only `None`-gate in the cost path is `cost_summary`, which returns `None` on unknown **pricing** — not on unknown **usage**. So unknown-pricing → `None` (correct), but unknown-usage → `$0.0` (false confidence). This contradicts the design-spec principle "Incorrect confidence is worse than explicit unknown values."
- **Impact:** Any provider/model that omits the usage object is displayed/accumulated as `$0.0` rather than "cost unknown." Narrow (most providers emit usage), but it directly contradicts the guiding principle.
- **Fix:** Make `AssistantMessage.usage: Option<Usage>` and `calculate_cost: Option<CostBreakdown>` that is `None` when usage is `None`, threading through `CumulativeUsage`; or add a `usage_reported: bool`/`cost_known: bool` flag that cost displays honor. At minimum, document the deferral (see U2).

**U2. `opi-ai/README.md` claims "cost `None` when usage absent" but the implementation returns `$0.0`** — Minor (doc drift)
- `crates/opi-ai/README.md:179-184`; `docs/opi-spec.md:1516-1527`
- The "Best-Effort Cost" section states cost helpers keep "explicit unknowns (cost `None`)" when "pricing **or usage** is absent." This is true for pricing, false for usage. `docs/opi-spec.md` restates the principle without acknowledging the gap, and the "Explicitly deferred" paragraph does not list absent-usage-cost.
- **Fix:** Implement U1 and keep the README text, or amend both docs to read "when **pricing** is absent, cost returns `None`; when **usage** is absent, cost is reported as `$0.0` (a known false-confidence gap deferred to a later phase)."

**U3. Responses `response_id` test passes after removing *either* the Created- or Completed-branch threading alone** — Minor
- `crates/opi-ai/tests/openai_responses_fixtures.rs:444-477`; production `openai_responses.rs:344-346` and `:546-548`
- The fixture carries the same `id:"resp_1"` in both `response.created` and `response.completed`, so the two threading sites are redundant with respect to this single test. Deleting either one alone leaves the test green; only deleting both fails it. The load-bearing claim (response_id reaches `AssistantMessage::response_id`) is verified, but a one-site regression in either branch slips through.
- **Fix:** Add a fixture where `response.created` omits `id` (or uses a different sentinel) and only `response.completed` carries it, so the Completed-branch threading is independently verifiable.

**U4. Anthropic `message.id` → `AssistantMessage::response_id` round-trip has no fixture** — Minor
- `crates/opi-ai/src/anthropic.rs:370` sets `self.partial.response_id = id` from `MessageStart`; `crates/opi-ai/tests/anthropic_fixtures.rs` (zero `response_id` hits); `docs/opi-spec.md` Phase 12 section advertises the Anthropic `message.id` round-trip.
- The spec claim is verified for OpenAI Chat/Responses but not for Anthropic (nor Gemini/Bedrock/Vertex). `anthropic.rs:573` and `:1029` hardcode `response_id: None` in non-production paths, so a regression that overwrites/drops the value would be uncaught.
- **Fix:** Add an anthropic fixture streaming `message_start` with `id:msg_X` to completion, asserting `Done.message.response_id == Some("msg_X")`.

**U5. OpenRouter/Mistral/Azure inherit the OpenAI Chat cache-read mapping but have no dedicated cache-token fixture** — Minor
- `crates/opi-ai/src/openai_chat.rs:207-219` maps `prompt_tokens_details.cached_tokens → cache_read_tokens` once, in the shared path; `crates/opi-ai/tests/openai_chat_fixtures.rs:263-287` is the only cache-token test and uses the bare `openai` provider_id.
- A future change that affects a profile's `RawUsage` deserialization could silently drop `cache_read_tokens` without test failure for the inherited families.
- **Fix:** Add one cache-token assertion per inherited profile (OpenRouter, Mistral, Azure).

### Retry / cancel / proxy

**X1. The eight per-family `stream_cancellation_aborts_before_completion` tests prove only "no hang," not real cancellation** — Minor (overclaimed coverage) *(flagged independently by two dimensions)*
- Tests: `anthropic_fixtures.rs:1268-1328` and the identical pattern in `openai_chat_fixtures.rs:1198`, `openai_responses_fixtures.rs:810`, `openrouter_fixtures.rs:375`, `mistral_fixtures.rs:422`, `gemini_fixtures.rs:535`, `vertex_fixtures.rs:501`, `azure_openai_fixtures.rs:484`; adapter cancel arms e.g. `anthropic.rs:758-768`, `openai_chat.rs:929`.
- Each test mounts a wiremock returning a **fully-buffered** static SSE fixture, reads one event, calls `cancel.cancel()`, then drains inside `timeout(2s)` breaking on `event.is_terminal()`. Because the body is delivered synchronously, the terminal `Done` arrives in milliseconds regardless of cancel; the drain exits via the terminal-event branch, not the cancel branch. Mutation: deleting the adapter's `cancel.cancelled() => return Ok(())` select arm leaves every one of the eight tests green. The bedrock test (`fixture_path_does_not_observe_cancel_documented_http_only_limitation`) is the lone honest named-negative.
- **Impact:** DoD clause 6 ("provider stream cancellation") is overclaimed for these families: a regression removing adapter-level cancellation would not be caught. Real cancellation is only proven at the agent layer (`cancellation_during_retry_backoff_aborts`).
- **Fix:** Serve a slow/streamed body (or one missing the terminal event) so the cancel arm is the only exit before the timeout, and assert termination within a tight bound (e.g. 200 ms) — or downgrade the test name/docstring to `stream_drains_without_hang_after_cancel` and add one deterministic-interrupt test.

**X2. `M1` residual is real: attempt-2+ partial-content failure is mislabeled `AutoRetryEnd(false)`/`RETRY_EXHAUSTED` and has no test** — Minor
- `crates/opi-agent/src/agent_loop.rs:395-478`
- When attempt 1 errors with no content, the guard retries (`retry_attempt` → 1, `AutoRetryStart` emitted). On attempt 2, content is delivered then a retryable error arrives mid-stream; the guard correctly suppresses retry, control falls to `:454`, and because `retry_attempt > 0` it emits `AutoRetryEnd{success:false}` (`:455-462`) plus `CODE_PROVIDER_RETRY_EXHAUSTED` (`:463-477`). This is misleading accounting: retries were **not** exhausted (budget remained) — the retry was deliberately suppressed. Observers counting retry-exhaustion incidents will over-count. The final `Result` is still correct.
- **I confirmed this myself:** read the full retry block (`agent_loop.rs:395-484`); the existing partial-content test puts `EventsThenError` on attempt 1, so `retry_attempt==0` when the guard fires and the mislabel branch is never reached.
- **Fix:** Add a distinct branch/event for the partial-content-suppression case (e.g. `AutoRetryEnd` with `reason:"partial_content"`) and a test with `[Error(RateLimited), EventsThenError([Start,TextDelta], RateLimited)]` asserting correct suppression-not-exhaustion semantics.

**X3. Proxy tests prove config round-trip but never route traffic through a proxy mock** — Minor
- `crates/opi-ai/tests/proxy_support.rs:266-336`; `crates/opi-coding-agent/tests/proxy_config.rs:311-330`
- Every proxy test asserts a configured proxy URL is readable back via `client.proxy_config()`; none mounts a proxy mock that records the CONNECT/proxied request. `proxy_config()` returns the stored field (`http.rs:78-80`), not anything inspected from `reqwest::Client`, so the production wiring at `http.rs:137-143` (correct) is not exercised end-to-end. (The separate `redact_proxy_credentials` tests are sound.) No secret-leak risk is introduced.
- **Fix:** Add one test that starts a wiremock proxy, configures it via `reqwest::Proxy::all`, sends a `stream()` request, and asserts the proxy mock received the request.

### OpenAI-compatible profiles

**P1. Azure `with_compat` is exercised only by a test; `build_provider` never calls it** — Minor
- `crates/opi-ai/src/azure_openai.rs:139-149`; `crates/opi-coding-agent/src/provider_factory.rs:790-815`; sole caller `azure_openai_fixtures.rs:451`
- The factory's `azure` arm builds via `AzureOpenAIProvider::new`/`from_config` + `.with_client(...)` and never `.with_compat(...)`. There is no `[providers.azure] compat` field. So in production Azure always runs `CompatConfig::default()` — developer-role/strict-tool-schema/max-tokens-field/tool-result-name overrides are impossible to enable for the built-in Azure family. The README/doc-comment implies Azure inherits the shared compat path; that is true at the serializer layer only, not the config layer.
- **Fix:** Wire an Azure compat config through the factory arm, or downgrade the README claim to "Azure uses `CompatConfig::default()` at runtime; use an `openai_compatible` profile for per-profile compat."

**P2. `AzureOpenAIProvider::with_compat` reconstruction drops inner `extra_headers` and `models`** — Minor (latent, never fires today)
- `crates/opi-ai/src/azure_openai.rs:140-148`
- `with_compat` rebuilds `self.inner` via `new_for_profile(..., compat, vec![], vec![])` — empty `extra_headers` and `models`. If `with_compat` were ever chained after `from_config`, deployments/headers configured earlier would be silently lost. Today inert because `with_compat` is dead in production (P1).
- **Fix:** Reconstruct preserving `self.inner.extra_headers()`/`models()`, or add an inner `set_compat` that swaps only the compat field.

**P3. OpenRouter/Mistral/responses runtime arms hardcode `CompatConfig::default()`** — Info
- `crates/opi-coding-agent/src/provider_factory.rs:672,706-710,720`; `mistral.rs:18-28`
- Only the `other =>` arm routing to `build_openai_compatible_profile` reads compat from config. This is consistent with SC7 (config-driven profiles are the preferred breadth path) — not a bug, but the README compat table could be read as applying to the built-in OpenRouter/Mistral families.
- **Fix:** No code change; optionally clarify the README that the flag table applies to `openai_compatible` profiles and that built-in OpenRouter/Mistral ship defaults.

**P4. `require_assistant_after_tool_result` README row claims "enforced as a runtime check" — no enforcement exists** — Minor (doc drift)
- `crates/opi-ai/src/openai_chat.rs:596,609`; `crates/opi-ai/README.md:139`
- The flag is honestly documented in the struct doc-comment as compat metadata, but the README row says "enforced as a runtime check, not a wire field," and there is no runtime check. (The struct-level documentation is fine; the published README row overclaims.)
- **Fix:** Reword the README row to "Modeled as compat metadata only; no wire field and no runtime enforcement."

### Tool calls / thinking / images

**C1. Bedrock `event_stream` binary parser has no malformed/adversarial-frame test** — Minor (panic-safety on untrusted bytes)
- `crates/opi-ai/src/bedrock/event_stream.rs:46-145`; tests `:187-275`
- `parse_single_frame` hand-rolls 6 bounds checks (`:76,83,88,91,93,127`) on untrusted network bytes; the loop reads `u32 total_len`/`headers_len` from the wire. Tests cover only well-formed frames, empty buffer, a truncated frame, and a split-then-completed frame. No test feeds corrupt header lengths, an over-large `headers_len`, a bad CRC, or garbage bytes ≥ `MIN_FRAME_SIZE`. A logic slip in the bounds arithmetic would panic the agent loop mid-stream.
- **Fix:** Add tests for `headers_len` overflow, string `val_len` exceeding the header region, a `total_len == PRELUDE_LEN+4` truncated header, and random garbage bytes; confirm no panic.

**C2. No `max_image_count` cap despite spec "verify image size/count limits where configured"** — Info
- `crates/opi-coding-agent/src/image.rs:8,40-49`; `crates/opi-coding-agent/src/config.rs:40,302`
- A per-image byte cap exists (`DEFAULT_MAX_IMAGE_BYTES = 20 MiB`) but there is no count cap anywhere. A user message can attach unbounded image parts; this surfaces as a provider 4xx rather than a client-side diagnostic. The spec's "where configured" wording makes this an accepted-deferred gap — but the deferral is honest only if 12.9 names it.
- **Fix:** Add a `max_image_count` config field validated in `validate_request_capabilities`, or document the deferral explicitly in the 12.9 provider docs.

**C3. Inline `ImageSource::Base64`/`Bytes` skip MIME and base64 validation** — Info
- `crates/opi-coding-agent/src/image.rs:11-58`; `crates/opi-ai/src/message.rs`; `crates/opi-ai/src/provider.rs:84-115`
- Only file-path attachments (`--image`/`/image`) go through MIME detection + byte read. Programmatic callers (extensions/SDK/RPC/replay) that build `InputContent::Image` directly bypass `image.rs` — there is no MIME-vs-bytes consistency check and no base64-alphabet validation before serialization into the request body. CLI/TUI blast radius is small; the gap matters for embedders.
- **Fix:** Add a `validate_image_content` helper in `opi-ai` invoked from `validate_request_capabilities`, or document the inline-source gap in 12.9.

**Note (clean):** The "stream layer never parses tool args" claim was confirmed for all four required mappers (Anthropic/OpenAI Chat/Responses/Bedrock accumulate raw `String`; JSON parsing happens only in `agent_loop`), so malformed tool args route to runtime validation rather than panicking at the adapter. The Bedrock reasoningContent negative assertion is non-vacuous (adding a parser that emitted `ThinkingStart` would fail it).

### Provider factory / list-models / auth

**F1. Interactive auth failure uses a hardcoded exit literal `3` and is unpinned by any test** — Minor
- `crates/opi-coding-agent/src/main.rs:443-457`
- `run_interactive` calls `std::process::exit(3)` on `ProviderBuildError::Auth`, unlike `run_non_interactive` (`:257`) and `run_rpc` (`:372`) which use `ExitCode::AuthFailure as i32`. Behavior is correct today (3 == AuthFailure) but if the enum repr changes, interactive silently diverges. The "deferred as TTY-hard" framing is misleading — the path is already implemented, just unverified.
- **Fix:** Replace the literal with `ExitCode::AuthFailure as i32` and either add an interactive-mode auth subprocess test or reframe the deferral as "unpinned, not unimplemented."

**F2. Precedence wiremock test proves the override wins but does not pin the profile default** — Minor
- `crates/opi-coding-agent/tests/provider_factory.rs:1031-1112`
- `body_partial_json` matches only the overridden keys; it cannot distinguish "override wins" from "profile default was also wrong in the same direction." A regression that replaces the profile default globally for all models would still pass.
- **Fix:** Add a second request under a non-overridden model asserting `role=="system"` and `max_tokens`.

**F3. `--list-models` network-free/no-key claim is structurally enforced but lacks a positive no-network assertion** — Info
- `crates/opi-coding-agent/src/provider_factory.rs:957-992`; `main.rs:551-623`; `model_listing.rs:11-21`
- The listing path only constructs providers and projects `model_entries_from_registry(registry.all_models())`; AuthDescriptor/CompatMetadata and any live connection are unreachable. This is enforced by data shape, not merely asserted-by-absence. Observation only.
- **Fix (optional):** Add a test running `--list-models` against a wiremock `base_url` and asserting `server.received_requests()` is empty.

### Docs & guard tests

**G1. `provider_docs_and_profile_policy_stay_in_sync` pins README substrings, not `CompatConfig` source structure** — Minor
- `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs:126-285`
- The test iterates a hardcoded 8-flag list and asserts `contains_ci` of each flag name in the READMEs; it never reads the `CompatConfig` struct (`openai_chat.rs:580-597`) and asserts the documented count matches the live field count. The `previous_response_id` check (`:204-209`) is source-anchored, showing the author knows how to anchor to source, but the flag-list section is not.
- **Fix:** Read the struct and assert the documented count equals the live field count; or pin each flag to a code anchor.

**G2. `phase12_non_goals_not_in_core` positive-phrase blocklist covers only 9 specific phrasings** — Minor
- `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs:384-404`
- `forbidden_positive` (9 entries) is evadable by rephrasing ("OAuth support", "image generation capability"). It is supplementary — the real structural enforcement is the `Cargo.toml` forbidden-dep scan (`:416-442`) and `lib.rs` forbidden-mod scan (`:446-452`), which catch actual code. Mutation: adding "opi now offers OAuth support for Anthropic." to `README.md` passes the guard.
- **Fix:** Expand the blocklist or invert the check (require any mention of oauth/subscription/copilot/browser/image-generation to appear near a negation token).

**G3. `first_class_provider_guard` lib.rs parser only matches bare `pub mod`** — Minor
- `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs:299-343`
- The parser (`:301`) uses `strip_prefix("pub mod ")` and would miss `pub(crate) mod`, cfg-attribute-prefixed, or inline `pub mod foo {}`. Step (4) filesystem cross-check (`:327-343`) provides defense-in-depth (any new `.rs`/dir surfaces regardless), so a hidden module cannot fully evade the guard; impact is diagnostic-quality only. All current declarations are bare `pub mod` (the file has 19 module declarations, not 16 as the reviewer first guessed).
- **Fix:** Broaden the prefix match (or replace with a regex/syn parse); acceptable as-is given step (4) compensates.

**G4. `default_provider_tests_are_network_free` `#[ignore]`-tracking machinery is dead code** — Minor
- `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs:519-611`
- A brace-depth-tracking `#[ignore]`-fn boundary parser (`:553-596`) is implemented, but a workspace grep for `#[ignore` in provider test dirs returns zero matches outside this file. The load-bearing part is the unconditional `var("<CRED>")` substring ban (`:597-604`). ~45 lines of dead complexity.
- **Fix:** Drop the `#[ignore]` branch and assert unconditionally (live tests are already disallowed by policy in the README non-goals).

**G5. `validate_rejects_thinking_on_non_thinking_model_as_capability` asserts acceptance, not rejection** — Minor
- `crates/opi-ai/tests/provider_diagnostics.rs:357-379`
- The name says "rejects_thinking…as_capability" but the body uses `.expect("thinking preflight is owned by the harness layer, not opi-ai")` — a success path, the inverse of "rejects." The pinned behavior (thinking preflight lives in the harness; opi-ai clamps downstream) is intentional, but the name misleads. Contrast `:332` `validate_rejects_image_on_text_only_model_as_capability`, which correctly uses `.expect_err`.
- **Fix:** Rename to `validate_does_not_reject_thinking_at_opi_ai_layer` (or `thinking_preflight_is_deferred_to_harness`).

### Release bookkeeping

**B1. `CHANGELOG.md` `[Unreleased]` is empty despite four Phase-12 `feat` commits** — Minor (process)
- `CHANGELOG.md:8-12`
- `git diff f3a61f3..HEAD -- CHANGELOG.md` is empty, yet the range contains `feat(opi-ai)` commits for 12.2/12.3/12.6 (12.8 collapsed to test). CLAUDE.md mandates "New entries ALWAYS go under `## [Unreleased]`" with Conventional-Commits categorization. When the next `opi-release` runs, the changelog will not advertise the 9-class taxonomy, CompatConfig flags, response_id round-trip, `safe_excerpt`, or the `Network`-retryability expansion; embedders upgrading `opi-ai` get no breadcrumb for the behavioral change (M2).
- **Fix:** Add an `Unreleased` block (`Added`: taxonomy + `safe_excerpt` + CompatConfig flags + response_id round-trip; `Changed`: Network transport errors are now retryable across all providers; image preflight returns `UnsupportedCapability` instead of `RequestFailed`) before the next release.

### Other minor / info

- **O1 (Info).** `EventsThenError` doc comment misstates agent_loop `Done` semantics — `crates/opi-ai/src/test_support.rs:19-25`. The doc says "the agent loop exits the stream on Done," but the while-let loop (`agent_loop.rs:127`) terminates only when `stream.next()` yields `None`; a `Done` does not break it. No runtime bug (the practical guidance still works), but the rationale is stale and could mislead future test authors. Fix: reword to "build the partial prefix only; do not terminate the prefix with a `Done` that would push a complete assistant message before the trailing error arrives."
- **O2 (Info).** `stream_delivered_content` guard positive branch is covered by exactly one scenario (`retry_agent.rs:339-401`, attempt-1 text content). The tool-call-partial-then-error case and the Start-only case are not covered. The guard logic is simple and sound; these are edge-case gaps only.

---

## Containment & Independence Disclosure

Per your instruction, no prior audit/review/acceptance report was read during this audit. The independence guardrails were applied to every subagent and held, with two disclosed incidents:

1. **No agent read any forbidden file's contents.** During repo-wide greps (e.g. for `RequestFailed`, `usage_in_stream`, `require_assistant_after_tool_result`), the search tool surfaced *filenames* of prior reports that exist under `docs/snapshots/` — including `docs/snapshots/phase12/audit.opus4.6.md` and `docs/snapshots/phase12/audit.codex.md` (both currently untracked), plus earlier-phase audits. The verifiers that hit this **disclosed it explicitly and declined to open those files**; their verdicts rest on their own reading of source/tests/README/diffs.
2. **One verifier halted per the strict guardrail.** The adversarial verifier for the `RequestFailed` dead-variant finding interpreted the contamination rule strictly and returned a `contamination-stop` instead of a substantive verdict. That finding is nonetheless valid: I re-confirmed it independently from code (grep for `RequestFailed` in `crates/opi-ai/src` shows only the declaration, the `category()` arm, the diagnostic-bridge arm, and an unrelated `ConfigError` variant).

**Incidental knowledge in this reviewer's context:** I am now aware that prior Phase-12 audit files exist (`audit.opus4.6.md`, `audit.codex.md`) — but I have **not** read them, and every finding above is supported by code evidence that I or a subagent personally verified at HEAD. The 100%-confirmed / 0%-refuted verifier tally should be read with skepticism: the `RequestFailed` finding's verifier did not actually adjudicate, and adversarial passes that confirm every finding can drift toward rubber-stamping. I therefore personally re-verified the headline findings (M1 `usage_in_stream`, M2 `Network` retry, R1 `CompactionEnd` redaction, T1 `RequestFailed`, the retry guard) by reading the code myself before publishing them.

Findings [16] and [28] from the workflow were the same issue (the eight per-family cancel tests) reported by two dimensions; they are merged here as **X1**. The critic contributed six additional findings; five are folded into the themes above (M2, U4, C1, B1, O1) and one (the `require_assistant` README row) is folded into **P4**.

## Methodology

- Eight dimension reviewers (error taxonomy/redaction; profile flags; usage/response-id/cache; retry/cancel/proxy; tool calls + thinking/image; factory/auth/list-models; docs/guards; cross-cutting test-vacuity), each given only the design-spec DoD text for its tasks, the commit range, and hard contamination guardrails.
- Each finding was adversarially verified by an independent subagent that re-read the cited code/tests, applied the described mutation, and tried to refute.
- A completeness critic then compared the confirmed set against all nine Success Criteria and the spec's failure modes; its six findings are folded in above.
- The audit lead (this reviewer) independently re-verified every headline claim by reading code at HEAD before writing this document.

**Files the audit relied on (selected):** `crates/opi-ai/src/{provider,http,openai_chat,openai_responses,anthropic,gemini,bedrock/{mod,event_stream},azure_openai,vertex,test_support,stream,message}.rs`; `crates/opi-agent/src/{agent_loop,diagnostic,event}.rs`; `crates/opi-coding-agent/src/{config,provider_factory,main,runner,harness,image,session_coordinator}.rs`; `crates/opi-ai/tests/{provider_diagnostics,provider_error_classes,openai_chat_fixtures,openai_responses_fixtures,anthropic_fixtures,bedrock_fixtures,retry_backoff,proxy_support}.rs`; `crates/opi-agent/tests/{retry_agent,tool_validation,trace_envelope}.rs`; `crates/opi-coding-agent/tests/{provider_factory,phase12_provider_correctness_docs,list_models,rpc_jsonl,non_interactive,json_mode}.rs`; `CHANGELOG.md`; `docs/opi-spec.md`; `crates/opi-ai/README.md`.
