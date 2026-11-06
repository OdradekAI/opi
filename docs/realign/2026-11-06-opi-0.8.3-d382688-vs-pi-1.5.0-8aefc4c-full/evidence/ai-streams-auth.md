# Batch AI-STR/AI-AUTH — streams, transport, auth, typed ops (Dims 2, 4, 16)

Batch summary (5 lines):
1. Contracts measured: streaming event protocol + ChatStream seam, retry/fallback/timeout/cancellation/deferred lifecycle, WebSocket transport + session resources, credential store + OAuth flows (9 providers) + ADC/AWS/federation, typed ops (openrouter-images, 4 classifiers, constrained sampling), client identity.
2. Verdict tallies (31 records): equivalent 16, partial 3, different 8, missing 4, opi-only 0, unresolved 0.
3. Biggest gaps: agent-level message-pattern retry classifier absent; no ClientIdentity (User-Agent modes/attribution, env callback host) surface; api-key interactive login absent; System One experimental image input absent; chat-provider retryable-status set narrower than pi.
4. Systematic seam differences: pi in-band `error` event vs opi `Err(CallFailure)`; pi shared-live `partial` vs opi event-time snapshots; event tag spelling (`toolcall_*`→`tool_call_*`, `toolUse`→`tool_use`); provider error text redacted (attested/sanitized) vs pi's 4000-char body passthrough.
5. Uncertainties: bedrock smithy/mistral/pi-messages measured at architecture level only (deep dives were anthropic/openai×3/google per brief); OAuth endpoint behavior verified statically, never against network (offline).

All anchors: pi side relative to `.pi/pi-1.5.0/packages/ai/src/`, opi side relative to `crates/opi-ai/src/`. Review outcome: all self-reviewed (pass 2 pending).

---

## Streaming event contract

### AI-STR-01 — AssistantMessageEvent kinds, payloads, ordering
- Dimension(s): 2
- pi entry: `types.ts:823-839` (`AssistantMessageEvent`: start, text_start/delta/end, thinking_start/delta/end, toolcall_start/delta/end, done{reason,message}, error{reason,error}), ordering contract doc `types.ts:807-820` (public)
- Opi entry: `stream.rs:421-507` (`AssistantStreamEvent`: Start, TextStart/TextDelta/TextEnd, ThinkingStart/Delta/End, ToolCallStart/Delta/End, Done, Error; all adapters emit `Start`: anthropic.rs:718, openai_chat.rs:661, gemini.rs:400)
- Comparison: different
- Observable difference: semantic contract is 1:1 (same 12 kinds, same contentIndex/delta/content fields, same `start`-before-deltas-before-terminal ordering, premature-EOF typed), but wire tag spelling differs: pi `"toolcall_start"` (types.ts:831) vs opi serde `"tool_call_start"` (stream.rs:461); stop reason pi `"toolUse"` (types.ts:489) vs opi `"tool_use"` (stream.rs:20). `Done.reason` in pi is narrowed to stop/length/toolUse/deferred at type level; opi `StopReason` is the shared enum (enforced instead at the `ChatStream` seam, stream.rs:939-955, where a `Done` carrying error/aborted converts to `Err(CallFailure)`).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: serde tag divergence is observable wherever events are serialized (replay/transcript). pi also has a `"pending"` StopReason member (types.ts:489) that opi's enum lacks (opi has no pending state on the terminal enum).

### AI-STR-02 — Terminal/error delivery seam and result()
- Dimension(s): 2
- pi entry: `utils/event-stream.ts:33-131` (`EventStream` push/queue/result; `AssistantMessageEventStream` times finals; post-terminal pushes dropped), `types.ts:811-814` (direct `streamSimple()` throws synchronously on missing auth; failures after stream return encoded in-stream)
- Opi entry: `stream.rs:679-990` (`ChatStream`: `Result<AssistantStreamEvent, CallFailure>`, first-terminal absorption → permanent EOF, premature-EOF typed failure "stream ended without a terminal event", `result()` drives to `Result<AssistantMessage, CallFailure>`; `Error` event kept read-only legacy, converted at the seam stream.rs:941-950)
- Comparison: different
- Observable difference: pi delivers failures as an in-band `{type:"error"}` event on the same async-iterable plus a shared `result()` promise; errors never reject the iterator. opi delivers exactly one `Err(CallFailure)` (typed `ProviderError` cause + canonical partial message with stopReason normalization aborted/error, stream.rs:607-632) and never emits the in-band error from current producers. Setup failures: pi throws synchronously before returning a stream; opi returns a lazy `ChatCall` whose failure surfaces as `Err` when polled (models.rs:1656-1674, auth resolution at models.rs:1300-1320). pi's `EventStream` also buffers events pushed before a consumer attaches; opi is pull-based.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's shape is the deliberate AIR-047 redesign; both satisfy "one terminal, nothing after".

### AI-STR-03 — `partial` snapshot semantics
- Dimension(s): 2
- pi entry: `types.ts:816-819` ("`partial` is the shared live response-so-far helper, not an event-time snapshot")
- Opi entry: `stream.rs:517-524` (`latest_partial` doc: "Every streaming event clones the producer's partial message at event time, so consumers observe immutable event-time snapshots regardless of later stream progress")
- Comparison: different
- Observable difference: a TS consumer holding event N's `partial` sees it mutate as the stream progresses (same object reference); an opi consumer holding event N's `partial` sees the frozen state at event N (clone). Also feeds cancellation: opi's `CancelAwareReceiverStream` retains the last producer snapshot and attaches it to the cancellation failure (stream.rs:1009-1026 + cancellation test stream.rs:1145-1173 retains silent producer state).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: deliberate Rust-ownership divergence.

### AI-STR-04 — Response timing (`durationMs`)
- Dimension(s): 2
- pi entry: `utils/event-stream.ts:91-129` (`AssistantMessageEventStream.#time`: monotonic `performance.now()` from stream creation, applied to done/error and `end()` results, skipped when the message predates the stream — deferred fetches stay untimed)
- Opi entry: `stream.rs` message field `message.rs:72` (`duration_ms: Option<u64>`); assigned only in `openai_chat.rs:1971`, `openai_responses.rs:640`, `gemini.rs:1716`; anthropic.rs (grep `duration_ms` → only `None`), mistral.rs, pi_messages.rs, openai_codex_responses.rs, vertex.rs, azure_openai.rs never assign
- Comparison: partial
- Observable difference: every pi stream's final message carries `durationMs`; in opi only the OpenAI-chat, OpenAI-responses and Gemini wires set `duration_ms` — Anthropic/Mistral/pi-messages/codex/azure/vertex finals report `duration_ms: None` (absence searches: `grep -rn "duration_ms = Some" crates/opi-ai/src/` → 3 hits; `grep -n "duration_ms" crates/opi-ai/src/anthropic.rs` → only `None`).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi has no central timing owner equivalent to `AssistantMessageEventStream`; timing is per-adapter.

### AI-STR-05 — Raw provider-event observer (`onProviderStreamEvent`)
- Dimension(s): 2, 4
- pi entry: `types.ts:230-235` (read-only, adapter-explicit), invoked e.g. `api/anthropic-messages.ts:703`, `api/openai-responses-shared.ts:672`
- Opi entry: `callbacks.rs:24,141,192-196,246-293` (`RequestCallbacks.on_raw`, sync+async, "invoked with each parsed provider wire event, in wire order"); invoked in anthropic.rs:2309, openai_responses.rs:790, gemini.rs:1647, openai_chat.rs:2134, mistral.rs:1016/1039, pi_messages.rs:791/812
- Comparison: equivalent
- Observable difference: none found for the observer itself (read-only parsed wire value, adapter-owned). opi's callback bundle is a superset: `on_payload`/`on_response` (pi `onPayload`/`onResponse`, types.ts:180-190) plus `header_transform` (opi-only).
- Evidence basis: source inspection
- Review outcome: self-reviewed

---

## Request lifecycle

### AI-STR-06 — Provider-level retry (conditions/backoff/hints)
- Dimension(s): 4
- pi entry: `utils/provider-retry.ts:37-45` (retryable: `x-should-retry` override, 408/409/429/>=500, no-status), `:66-79` (delay: `retry-after-ms` → `retry-after`, else `min(0.5s*2^i, 8s)*(1-rand*0.25)`), `:81-89` (server delay > `maxRetryDelayMs` (default 60 s) fails immediately; 0 disables), `:101` (`maxRetries ?? 0`); used by chat SDK calls (openai-completions.ts:379-385) and typed ops (classifier-shared.ts:90-107)
- Opi entry: chat adapters: `retry.rs:714-733` (`retry_delay_ms`: `min(500ms*2^i, 8000ms)` × jitter `[0.75,1.0)`, hint honored, cap `max_retry_delay_ms` 60 s seals Finished when exceeded, retry.rs:644-651), `retry.rs:151-160` (`parse_retry_after`: retry-after + x-ratelimit-reset), default `max_retries: 0` (retry.rs:277-289); but chat status maps type only 429→RateLimited, 408/504→Timeout, else ProviderSide (anthropic.rs:2417-2434, openai_chat.rs:2214-2228, openai_responses.rs:839-851, gemini.rs:1822-1824) and `ProviderError::is_retryable` = RateLimited|Timeout|Network (provider.rs:965-971). Typed-op adapters reproduce the full pi ladder: `api/system_one.rs:252` (`408||409||429||>=500` + `x-should-retry` at :427-436 + no-status network/timeout), decisions.rs, llama.rs, openrouter_images.rs:319
- Comparison: different
- Observable difference: with retries enabled on a chat provider, pi retries 409 and 500/502/503 (and honors `x-should-retry`); opi's chat acquisition never admits a retry for those (ProviderSide is non-retryable), so a 503 with `maxRetries>0` fails once in opi but retries in pi. Backoff formula, hint precedence, 60 s fail-fast cap, and default-0 attempts are equivalent. Classifier/image paths match pi exactly.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi mirror docstring "Mirrors the pinned OpenAI/Anthropic SDK retry policy". opi's helper-family doc (retry.rs:187-198) claims 408/409/429/>=500 semantics, implemented only in the typed-op transport, not the chat acquisition path.

### AI-STR-07 — Agent-level retry loop + message-pattern classification
- Dimension(s): 4
- pi entry: `utils/retry.ts:8-95` (`NON_RETRYABLE_PROVIDER_LIMIT_ERROR_PATTERN`: quota/billing/subscription/Zhipu-1308 text patterns; `RETRYABLE_PROVIDER_ERROR_PATTERN`: ~60 network/gateway/stream patterns), `:186-248` (`retryAssistantCall`: bounded attempts, `baseDelayMs*2^(attempt-1)` capped 60 s, abort normalization, callbacks onRetryScheduled/onRetryAttemptStart/onRetryFinished), `:255-260` (`isRetryableAssistantError`)
- Opi entry: absent in opi-ai. Absence search: `grep -rn "insufficient_quota|quota exceeded|billing|subscription_sharing" crates/opi-ai/src/` → only ADC `quota_project` comments (auth/adc/cache.rs:5, auth/adc/external.rs:714). opi-agent's loop delegates to the SDK acquisition admission (`opi-agent/src/agent_loop.rs:1231` uses `cause.is_retryable()` + `AutoRetry*` events); no message-string classifier exists anywhere in the workspace (`grep -rln "RetryPolicy|retryDelayMs|onRetryScheduled" crates/` → none)
- Comparison: missing
- Observable difference: pi callers get text-classified retryability (e.g. OpenCode "GoUsageLimitError" fails fast, "temporarily unavailable" retries); opi classifies only typed causes (HTTP status/timeout/network), so a 429 body reading as a subscription limit behaves like any other rate limit, and provider-specific transient strings delivered mid-stream never trigger an agent-level restart.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: partially compensated by opi's typed pre-handoff acquisition retry (AI-STR-06) — but pi's classifier specifically distinguishes non-retryable 429 bodies, which the typed path cannot.

### AI-STR-08 — Cancellation (AbortSignal vs CancellationToken)
- Dimension(s): 4
- pi entry: `utils/abort.ts:9,17` (`operationSignal`, `raceWithAbortSignal`), `utils/abort-signals.ts:6` (`combineAbortSignals`), abortable sleeps in retry.ts:150-172 / provider-retry.ts:57-74
- Opi entry: `cancel.rs` (pins those sources explicitly: `operation_token` = one-way child token, `sleep_cancel` armed-timer release, `race_with_cancel` abandoned-promise rule); cancellation priority + buffered-event discard + producer-snapshot retention in `stream.rs:975-1066` (`CancelAwareReceiverStream`, `send_or_cancel` biased select); tests stream.rs:1106-1252 (backpressured producer, pending consumer wake, post-terminal no-cancel)
- Comparison: equivalent
- Observable difference: none material found. Both: cancel wins over queued events (opi discards buffered pre-cancel events, test stream.rs:1205-1213), cancel produces an aborted-terminal message with retained canonical partial, and losing waits release timers. opi requires a Tokio runtime for the mpsc/sync machinery; pi is runtime-free. `combineAbortSignals` has no single opi helper (adapters use `tokio::select!` with per-attempt timeout, e.g. deferred fetch `DeferredFetchCall.timeout + .cancel`).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-STR-09 — Timeout phases
- Dimension(s): 4
- pi entry: `types.ts:196-202` (`timeoutMs`: "OpenAI and Anthropic SDK clients default to 10 minutes"), `types.ts:224-229` (`websocketConnectTimeoutMs`: connect/open handshake only; stream idleness uses `timeoutMs`); classifier per-attempt fresh timeout classifier-shared.ts:90-107; OAuth budgets 30 s token exchange / 15 s refresh (anthropic.ts:85, resolve.ts:78)
- Opi entry: per-request `request.timeout` (provider.rs:281-285, distinct from cancellation; applied only when set: anthropic.rs:2155-2157, openai_chat.rs:2029, gemini.rs:1581, responses/codex/azure/vertex/llama likewise); `http.rs:273-410` (HttpClient has pool idle timeout 90 s, no total-request default); WS connect timeout `openai_codex_responses.rs:652-655,796`; OAuth 15 s refresh `store_oauth.rs:31-32`, classifier per-attempt timeouts `api/system_one.rs`
- Comparison: different
- Observable difference: with no caller `timeoutMs`, a hung pi OpenAI/Anthropic response fails at 10 minutes; the same opi request has no overall deadline (reqwest default none) and can hang until cancelled. All other phases (per-request override, WS handshake vs stream idleness split, per-attempt classifier timeouts, 15 s refresh bound) match.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: deliberate omission of a hidden default, but under the reproduction lens observable.

### AI-STR-10 — Deferred responses (submit/fetch/cancel)
- Dimension(s): 2, 4
- pi entry: `types.ts:364-375` (`DeferredHandle` provider/modelId/api/id/expiresAt/pollAfterMs/data), `:278-302` (`ProviderStreams.fetchDeferred?/cancelDeferred?`, `DeferredFetchOptions.wait` default 0 = one status check), `:435-437` (`SimpleStreamOptions.deferred?: boolean | {window}`); only the faux provider implements it (providers/faux.ts:321-322, 548-563, 591-628); no real adapter implements deferred
- Opi entry: `deferred.rs` (handle schema serde-matching pi field names incl. `expiresAt`, deferred.rs:63-100; identity validation 105-152; `wait_ms` zero-check semantics 168-176; independent cancel budget 179-194; `DeferredApi` registration trait 196-231; models.rs:953-1107 register/validate/stream/fetch/cancel), scripted backends via `test_support.rs:262-280` (`MockProvider::with_deferred`), `StopReason::Deferred` stream.rs:35-42
- Comparison: equivalent
- Observable difference: contract-level none (same handle fields, wait-0 single check, best-effort cancel, `deferred` rides the assistant message, no implicit polling in either SDK). Both ship without a real remote deferred backend. Two nits: pi faux accepts `{window: "15m"|"1h"|"24h"}` (faux.ts:123-125); opi's mock scripting has `pending_fetches`/`poll_after_ms` but no window knob. opi adds route-identity validation and caller-injected expiry clock (`DeferredError::RouteMismatch/Expired/Unsupported`, deferred.rs:158-199) — pi has no SDK-side validation analog.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-STR-11 — WebSocket transport (OpenAI Codex)
- Dimension(s): 2, 4
- pi entry: `api/openai-codex-responses.ts:280-390` (transport `auto|sse|websocket|websocket-cached` — types.ts:147; SSE-fallback latch per session :311-313,381-385; connection-limit retry :358-366; missing-WS-continuation retry :360-363; `OpenAI-Beta: responses=experimental` :1690; `previous_response_id` continuation :1488-1548; cached-socket context :1535)
- Opi entry: `ws.rs` (RFC 6455 handshake: nonce/GUID/accept token, subprotocols, permessage-deflate negotiation), `ws/` (frame.rs, io.rs, pmd.rs owned PMD state + extensive tests incl. python-pmd fixtures), `openai_codex_responses.rs:570-650,780-850` (transport preference, resource lease registration, `codex_websocket_fallback` diagnostic :832, connect timeout :652), `openai_codex_responses/ws_transport.rs:409-422` (previous_response_id strip/inject on cached context), `stream.rs` ServiceTier semantics for codex
- Comparison: equivalent
- Observable difference: none found at the contract level: same transport preference set (provider.rs:239 `TransportPreference`), same SSE fallback with session latch, same connection-limit/continuation retry classes, same beta header (codex native.rs / ws_transport.rs:297-311), same cached-continuation body. opi owns the WS implementation (no SDK dependency) and adds resource-generation cleanup hooks (AI-STR-12).
- Evidence basis: source inspection + test inspection (ws/tests/)
- Review outcome: self-reviewed

### AI-STR-12 — Session resources ownership/cleanup
- Dimension(s): 4
- pi entry: `session-resources.ts:1-24` (module-global Set of cleanups; `cleanupSessionResources(sessionId?)` runs all, throws `AggregateError` on any failure), registered at module load by the codex WS adapter (openai-codex-responses.ts:982 → `closeOpenAICodexWebSocketSessions` :966)
- Opi entry: `codex_resources.rs` (`SessionResourceOwner` trait :113-124 incl. `cleanup_session_resources`, `ResourceRegistry` with generations :155-407, `CallResourceLease` :195-260, `CleanupHandle`/`CleanupReport` merging failures :36-110, `await_exits`); codex registers every WS-family call before auth (openai_codex_responses.rs:582-595); pure-SSE calls unregistered (parity with pi: fallback-latched sessions still cleaned? pi closes all cached sessions per session id; opi `cleanup(session)` matches)
- Comparison: different
- Observable difference: core ownership model matches (session-scoped WS resource cleanup, registration before first use, per-session and global cleanup), but the failure surface differs: pi throws a single `AggregateError` collecting all cleanup errors; opi returns a `CleanupReport` (counts + unconfirmed details) and awaits task exits; opi additionally exposes generation cutoffs, unconfirmed-release records and a `Cancelled` fast-path when a lease has no activity (openai_codex_responses.rs:596-600). Callers distinguish a failed cleanup by exception vs typed report.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's registry is generic (trait + namespaces) like pi's Set, but only codex uses it in both trees.

### AI-STR-13 — Anthropic server-side fallback
- Dimension(s): 4
- pi entry: `api/anthropic-messages.ts:202` (`SERVER_SIDE_FALLBACK_BETA = "server-side-fallback-2026-07-01"`), `:1323` (`params.fallbacks` from allowed fallback models), `:729-732` (mid-output `fallback` block rejected), `:710-716` (fallback cost lookup by responseModel)
- Opi entry: `anthropic.rs:1691-1696` (beta emitted when allowed fallback models non-empty), `:1726-1740` (`fallbacks` array from compat), `:593-601` (mid-output fallback marker → typed error, no canonical content), `stream.rs:24-30` + models cost path for usage model attribution
- Comparison: equivalent
- Observable difference: none found in the request/response contract (beta gating, `fallbacks` param shape, mid-output rejection, cost-attribution via response model).
- Evidence basis: source inspection
- Review outcome: self-reviewed

---

## Typed operations

### AI-STR-14 — Image generation (openrouter-images)
- Dimension(s): 2, 16
- pi entry: `api/openrouter-images.ts:50-130` (POST chat/completions `{model,messages,stream:false,modalities}`, data-URL output decode :113-121, usage with cached/cache-write split :176-190, provider retry, onPayload/onResponse, missing key error)
- Opi entry: `api/openrouter_images.rs` (same body shape :150-190, `{base}/chat/completions` :264, data-URL regex parser :640-665, pi-regex pinned, retry ladder with `x-should-retry` :319+, bearer auth, typed `ImagesResult` images.rs:199-230)
- Comparison: equivalent
- Observable difference: none found (modalities depend on model output capability both sides; text part when present; usage split pricing).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-STR-15 — Classifier core contract (types, usage, pricing)
- Dimension(s): 2, 16
- pi entry: `types.ts:741-786` (ClassifierContext{state,images?,questions}, choice/score/bool questions and answers, stopReason stop/error/aborted, usage optional priced from catalog), `api/classifier-shared.ts:90-131` (bearer POST with onPayload/onResponse, per-attempt fresh timeout, retry `maxRetries ?? 2`, `parseClassifierUsage` malformed→absent)
- Opi entry: `classifier.rs:186-283` (ClassifierQuestion/Answer/StopReason mirroring pi), `:283-470` (record + call + result with failure/in_progress), `api/system_one.rs` transport core (attempt loop, guards, output parser, billed-usage survival on parse failure)
- Comparison: equivalent
- Observable difference: none found at the contract level (question/answer shapes, temperature typed option APIs "cannot apply → ignore", usage pricing from catalog tiers, no usage on malformed bodies).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-STR-16 — System One + Cloudflare Workers AI classifiers (image input)
- Dimension(s): 2, 16
- pi entry: `api/system-one-shared.ts:26-48,127-165` (SystemOneWireRequest; image input `assets` (TypeSafe) vs `images` data-URLs (Clef) via `SystemOneCompat.imageInput`), `:53-54,216` (`TypeSafe-Danger-Zone-Images: true` opt-in header), `api/typesafe-system-one.ts:9` (images "experimental"), `api/cloudflare-workers-ai-system-one.ts` (REST `{model,input}` envelope)
- Opi entry: `api/system_one.rs` (`POST <base>/systemone`, `noul` mapping, shared transport core, doc "Image input is rejected before any request"), `api/cloudflare.rs` (REST run envelope, two response shapes, account-id placeholder); image rejection system_one.rs:291-296,351-355
- Comparison: partial
- Observable difference: pi accepts context images for TypeSafe (assets form, experimental opt-in header) and Clef (data-URL list, ≤ max, PNG/JPEG/WebP); opi rejects any `ClassifierContext.images` before dispatch with a typed error. Absence search: `grep -rn "TypeSafe-Danger-Zone-Images" crates/opi-ai/src/` → none.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Layer H — pi implements an experimental feature (experimental authority wording, implemented); opi's typed surface omits it entirely (see Layer H log).

### AI-STR-17 — llama.cpp classifier
- Dimension(s): 2, 16
- pi entry: `api/llama-cpp-classify.ts:22-208` (label tokens A.., `logit_bias`, softmax over logprobs/temperature, TypeSafe confidence `(n*peak-1)/(n-1)`, choice 2..N / score 2..N level guards)
- Opi entry: `api/llama.rs` (same doc pinning lines 5-9, temperature default 1.0 + positive validation :554-557, `label_probabilities` :210-220, same confidence formula :222, same 2..N guards :735-750, retry ladder :326)
- Comparison: equivalent
- Observable difference: none found.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-STR-18 — OpenAI Decisions classifier
- Dimension(s): 2, 16
- pi entry: `api/openai-decisions.ts:12-60` (`POST /v1/decisions {model,input,questions}`, bool→predicate with meanings appended, ≤128 images, refusal/missing/mistyped answers typed, only OpenAI API keys)
- Opi entry: `api/decisions.rs:1-40` (same wire, predicate mapping "a System One noul and a Decisions predicate are not interchangeable", 128 image cap, billed usage retention, Cloudflare 504 HTML no-retry, empty key fails typed pre-request)
- Comparison: equivalent
- Observable difference: none found (both pin `noRetryStatuses`-equivalent 504 handling; pi exposes `noRetryStatuses` option on `postClassifierRequest`, opi hardcodes the 504 rule for decisions).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-STR-19 — Client identity: User-Agent modes + provider attribution
- Dimension(s): 4, 16
- pi entry: `utils/identity.ts:1-35` (`UserAgentMode` detailed/short/custom; detailed default `pi/<version> (platform; runtime; arch)`; `ClientAttribution` → OpenRouter `HTTP-Referer`/`X-Title`/`X-OpenRouter-Categories`, NVIDIA invoke origin, OpenCode client; `ClientIdentity` on StreamOptions types.ts:204-208), `PI_OAUTH_CALLBACK_HOST` env (auth/oauth/anthropic.ts:17, openrouter.ts:22, openai-codex.ts:41)
- Opi entry: absent as a surface. Absence searches: `grep -rn "User-Agent|user_agent" crates/opi-ai/src/` → no matches (reqwest sends no UA); `grep -rn "X-Title|HTTP-Referer|X-OpenRouter-Categories|attribution" crates/opi-ai/src/` → only `openrouter.rs:30-34` hardcoding `HTTP-Referer: https://github.com/OdradekAI/opi`, `X-Title: "opi"`; `PI_OAUTH_CALLBACK_HOST` appears only in a comment (auth/oauth/openrouter.rs:65 — host is construction-time policy)
- Comparison: missing
- Observable difference: pi sends a detailed UA by default and lets callers configure attribution per request; opi sends no UA at all, has no caller-configurable attribution, and always sends its own OpenRouter identification headers regardless of caller (pi sends none unless identity provided). `PI_OAUTH_CALLBACK_HOST` is honored at runtime by pi; opi requires the host to be threaded through construction.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: agent self-identification is inherently product-specific; under strict lens recorded missing with the OpenRouter hardcode noted as an opi-only substitute.

### AI-STR-20 — Extension OAuth compat types (`compat/extension-oauth-types.ts`, `oauth.ts`)
- Dimension(s): 16
- pi entry: `compat/extension-oauth-types.ts` (type-only legacy `OAuthLoginCallbacks` surface "retained only for coding-agent extension compatibility"), re-exported by `oauth.ts` and declared as package entry points (`package.json`)
- Opi entry: absent. Absence search: `grep -rn "OAuthLoginCallbacks|extension-oauth|extension_oauth" crates/opi-ai/src/ crates/opi-coding-agent/src/` → none
- Comparison: missing
- Observable difference: type-only surface; no runtime behavior in pi either. Relevant only if an opi extension ecosystem needs the same legacy callback shape.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: recorded for completeness of the batch's file list.

---

## Auth

### AI-AUTH-01 — CredentialStore contract + default stores
- Dimension(s): 16
- pi entry: `auth/types.ts:57-82` (`CredentialStore` read/list/modify/delete; modify is the only write path, serialized per provider incl. cross-process where supported), `auth/credential-store.ts:7-67` (`InMemoryCredentialStore`, per-provider promise chains, abort-aware)
- Opi entry: `credential.rs:311-379` (`CredentialStore` read/write/delete/probe/modify; modify = serialized mutation with receipt/outcome semantics, `mutation_barrier`, `mutation_queue`), `mutation.rs:556+` (queue-backed in-memory reference store), production store in `opi-coding-agent/src/credential_store.rs` (OS keychain envelope)
- Comparison: equivalent
- Observable difference: contract shape matches (one credential per provider, serialized write path, read resolves undefined for missing). pi `list()` returns metadata; opi `probe()` returns Present/Absent/BackendUnavailable — same purpose, different tri-state. opi's modify returns a `MutationReceipt` with outcome (Written/Unchanged/Failed/Deleted/Unknown) vs pi's post-write credential; opi adds cross-instance barriers/generations.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: persistence substrate (auth.json file vs keychain) is owned by the coding-agent apps in both trees; pi app persists after login via modify, opi likewise.

### AI-AUTH-02 — Credential envelope (secrets handling)
- Dimension(s): 16
- pi entry: `auth/types.ts:17-36` (`ApiKeyCredential{key?,env?}` | `OAuthCredential{refresh,access,expires,[key:string]:unknown}` — open shape)
- Opi entry: `credential.rs:49-98` (`ApiKeyCredential` with `SecretString`), `:114-175` (`Credential::OAuthToken{access,refresh,expires_at,base_url,account_id,client_id,enterprise_domain,available_model_ids,scopes}` all typed, secrets zeroized, manual redacting Debug), `auth.rs:554-606` (`OAuthCredential.needs_refresh` fixed 5-min skew)
- Comparison: different
- Observable difference: pi's OAuth credential is an open record (extension keys preserved); opi's is a closed typed set — unknown provider fields cannot ride along. opi zeroizes secret material on drop and never prints it; pi stores plain strings in auth.json. Deliberate safety/typing difference, observable to store implementers and anyone reading persisted artifacts.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-AUTH-03 — Env precedence per provider
- Dimension(s): 16
- pi entry: `providers/anthropic.ts:35-52` (ANTHROPIC_AUTH_TOKEN bearer → ANTHROPIC_OAUTH_TOKEN → ANTHROPIC_API_KEY → federation last), `auth/helpers.ts:8-25` (`envApiKeyAuth`: stored key wins, else first set env var), per-provider names: GEMINI (providers/google.ts:11), OPENAI (providers/openai.ts:15), MISTRAL (providers/mistral.ts:11), AZURE_OPENAI (providers/azure.ts:48)
- Opi entry: `auth/builtin.rs:196-212` (anthropic: `ANTHROPIC_AUTH_TOKEN`→Bearer, `ANTHROPIC_OAUTH_TOKEN`, `ANTHROPIC_API_KEY` in that order, then federation), `providers/mod.rs:111,207,303,351,399,495` (api_key_env per profile: AZURE_OPENAI_API_KEY, GEMINI_API_KEY, MISTRAL_API_KEY, OPENAI_API_KEY, OPENROUTER_API_KEY, XAI_API_KEY); stored credential precedence: credential key first, env fallback (builtin.rs resolve path + provider_collection.rs)
- Comparison: equivalent
- Observable difference: none found in the sampled chains (anthropic 3-var ordering, per-provider env names, stored-over-env precedence).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: did not enumerate all ~45 pi provider definitions; sampled the provider set the brief's deep dives cover plus Cloudflare (AI-AUTH-11).

### AI-AUTH-04 — OAuth login/refresh/logout flows (9 providers)
- Dimension(s): 16
- pi entry: `auth/oauth/anthropic.ts` (CLIENT_ID base64 `9d1c250a-...`, claude.ai authorize, platform.claude.com token, port 53692 preferred + free-port fallback, copy-code method, scopes string, 5-min expiry margin, 30 s exchange timeout), `openai-codex.ts` (client `app_EMoamEEZ73f0CkXaXp7hrann`, port 1455 shared, device-code flow), `openai-chatgpt.ts:16-26` (`dynamic_agent_client`, auth.openai.com, `AGENT_NAME_HINT = "Pi"`), `github-copilot.ts` (client `Iv1.b507a08c87ecfe98`, device code + copilot_internal/v2/token), `xai.ts` (client `b1a00492-...`, auth.x.ai device grant, https-verification-URI enforcement), `kimi-coding.ts` (client `17e5f671-...`, 15-min device window, 5 s poll, 3 refresh retries), `openrouter.ts` (code→permanent API key, ephemeral callback port), `radius.ts` (client `pi-gateway`, port 1456, browser + device-code), `meta.ts`; OAuth provider set: 9 (providers/*.ts `lazyOAuth`)
- Opi entry: `auth/oauth/anthropic.rs:25-45,101-121,193` (same client id decoded, URLs, scopes, port 53692 + manual redirect URI, copy-code login), `codex.rs:34-39,478-485` (same client id, fixed 1455 redirect, device flow), `chatgpt.rs:31-32` (`dynamic_agent_client`, `CHATGPT_AGENT_NAME_HINT: "opi"`), `copilot.rs:36,120-122` (same client id, device + v2 token), `xai.rs:31-38` (same constants), `kimi_coding.rs:25-43` (same constants incl. 1 s min poll + slow_down increment), `openrouter.rs` (key-not-token exchange), `radius.rs:54,79,343` (`pi-gateway` client, discovery), `meta.rs`; shared core `auth/oauth/mod.rs` (PKCE S256 :133-160, `run_pkce_login` :452, callback bind fallback :232, `refresh_oauth_token` :921, token-endpoint error mapping :1045, `login_to_store` :1353)
- Comparison: partial
- Observable difference: all 9 flows, client ids, endpoints, ports, PKCE, device-code parameters and refresh margins match; the sole divergence found is the ChatGPT login's default agent-name hint: pi `"Pi"` (openai-chatgpt.ts:17, LoginOptions.agentName "defaults to pi's own name", auth/types.ts:204-215) vs opi `"opi"` (chatgpt.rs:32) — the string sent to OpenAI's authorize endpoint differs unless the app overrides it (opi accepts the override at chatgpt.rs:315).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Layer H — flows verified statically; no network probes (offline constraint).

### AI-AUTH-05 — OAuth refresh under store lock
- Dimension(s): 16
- pi entry: `auth/resolve.ts:71-138` (`refreshStoredOAuthCredential`: modify under lock, needs-refresh recheck inside, 15 s refresh timeout, cancellation stops only the lock wait — started refreshes run to persistence), `:141-167` (double-checked 5-min minimum validity, explicit `minOAuthValidityMs` re-check)
- Opi entry: `auth/store_oauth.rs:24-160` (`StoreOAuthResolver`: refresh inside store's serialized mutation, 15 s network bound excluding lock wait, needs-refresh recheck under exclusion, cancellation-after-refresh-start stops only the waiter `CancelWait`, post-failure same-identity committed-rotation adoption :107-119, expired-before-ACK typed failure)
- Comparison: equivalent
- Observable difference: none material (same lock, same recheck, same 15 s bound, same cancel-only-the-wait rule). opi adds receipt outcomes (Unknown → typed "OAuth mutation outcome unknown") where pi would surface the store error.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-AUTH-06 — Side-effect-free status checks
- Dimension(s): 16
- pi entry: `models.ts:661-699` (`checkAuth`; optional `ApiKeyAuth.check` "side-effect-free… use when resolve() may execute commands", auth/types.ts:123-133) — no provider in 1.5.0 implements `check` (grep `check(` across providers/ → none), so checkAuth falls back to full `resolve()`, which CAN execute ambient work (AWS credential_process, ADC executables)
- Opi entry: `credential.rs:188-234` (`CredentialStore::probe` → Present{label}/Absent/BackendUnavailable, "carries no secret material… doctor and --list-models"), used to gate listing without executing key commands
- Comparison: different
- Observable difference: pi's `checkAuth` on Bedrock may run `credential_process`/profile machinery (side effects possible); opi's probe is guaranteed non-executing. Callers cannot distinguish "configured" from "executable would succeed" identically: pi resolves actual auth, opi checks store presence + env candidates only.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: safer in opi; strict lens → different.

### AI-AUTH-07 — ADC (Google Vertex / Gemini Application Default Credentials)
- Dimension(s): 16
- pi entry: `api/google-vertex.ts:100,366,431` (ADC via google-auth-library `GoogleAuth` with optional `keyFilename`, project/location envs), `api/google-generative-ai.ts` (GEMINI_API_KEY x-goog-api-key)
- Opi entry: `auth/adc/` (native.rs 1762 lines: well-known non-interactive file resolution :710, external accounts external.rs, executables executable.rs, impersonation.rs, metadata server metadata.rs, subject tokens, cache.rs keyed cache), `auth/builtin_vertex.rs` (profile + Express API-key mode), `vertex.rs:45-80,281` (two pinned wire modes: API key → Express x-goog-api-key, ADC → aiplatform project/location URL)
- Comparison: equivalent
- Observable difference: none found at the contract level (same source set: gcloud well-known file, external-account/executable configs, metadata, impersonation delegates; token caching both). pi delegates to the JS library (its behavior incl. scopes owned upstream); opi implements natively — scope strings were not byte-compared (see coverage honesty).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-AUTH-08 — AWS credential chain (Bedrock)
- Dimension(s): 16
- pi entry: `api/bedrock-converse-stream.ts:174-263` (BedrockRuntimeClient config; `getConfiguredBedrockCredentials` :1255 explicit env override; otherwise SDK default chain), SigV4 + retry owned by AWS SDK v3
- Opi entry: `auth/aws/` (local.rs shared-config/profiles 1960 lines, sso.rs, web_identity.rs, process.rs + windows, metadata.rs, source_http.rs container chain, role.rs, cache.rs, endpoint.rs), `bedrock/sigv4.rs`, `bedrock/retry.rs` (Smithy standard/adaptive modes, quota, CUBIC limiter, `+5s` bounded hint), wired via `RetrySourcePolicy::Smithy` (retry.rs:201-260, 558-662)
- Comparison: equivalent
- Observable difference: none found in the source set (env → shared credentials/config → SSO → web identity → process → container/EC2 metadata, sigv4 signing, smithy retry modes). opi implements the chain natively including Windows process quirks; pi inherits it from the AWS SDK.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: exact SSO token-file formats and adaptive-mode constants not byte-compared (see coverage honesty).

### AI-AUTH-09 — Anthropic workload identity federation
- Dimension(s): 16
- pi entry: `api/anthropic-messages.ts:349-372,1082-1100` (envs ANTHROPIC_FEDERATION_RULE_ID/ORGANIZATION_ID/IDENTITY_TOKEN_FILE/WORKSPACE_ID; exchange + refresh performed by the Anthropic SDK via client config; only for provider `anthropic` and only when no key/auth header)
- Opi entry: `auth/anthropic_federation.rs:22-26` (same four env names), `:43-77` (HTTPS/loopback validation, all-three-required), `:267-310` (native token exchange: 16 KiB assertion limit, audience/scope claim, beta header `oauth-2025-04-20,oidc-federation-2026-04-01`), `:104-220` (cached entry, advisory refresh serves cached token on failure)
- Comparison: equivalent
- Observable difference: none found in trigger conditions (anthropic provider only, no resolved auth wins) or env names. Mechanics differ by necessity: pi's SDK does the exchange invisibly; opi implements it (with an explicit cache and failure policy).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-AUTH-10 — API-key interactive login
- Dimension(s): 16
- pi entry: `auth/helpers.ts:8-25` (`envApiKeyAuth.login` prompts for the key; stored via app persist), extended logins prompting provider env e.g. `providers/cloudflare-auth.ts` (key + account id)
- Opi entry: absent. Absence searches: `grep -rn "ApiKeyLogin|api_key_login|LoginApiKey" crates/` → none; `LoginPresenter` (auth.rs:710-800) has `prompt_text`/`await_manual_code` but no secret prompt; `/login` help: "starts login for Anthropic, GitHub Copilot, or OpenAI Codex" (opi-coding-agent/src/cli.rs:115) — OAuth routes only
- Comparison: missing
- Observable difference: a pi user can `/login <any-env-key-provider>` interactively to store an API key (with provider env for Cloudflare); an opi user cannot — keys must be set via env or keychain out-of-band.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-AUTH-11 — Cloudflare auth (Workers AI / AI Gateway env resolution)
- Dimension(s): 16
- pi entry: `providers/cloudflare-auth.ts:31-60` (CLOUDFLARE_API_KEY + CLOUDFLARE_ACCOUNT_ID (+ GATEWAY_ID for gateway) from credential-or-env; all-or-nothing), `api/cloudflare.ts:1-17` (base-URL templates with `{CLOUDFLARE_ACCOUNT_ID}`/`{CLOUDFLARE_GATEWAY_ID}` placeholders)
- Opi entry: `auth/builtin.rs:121-140` (account/gateway required per profile kind, composed base URLs), `:382-404` (`effective_base_url` rewrites the catalog placeholder templates from resolved auth), `anthropic.rs:2160-2161` (gateway bearer via `cf-aig-authorization`), `api/cloudflare.rs` (account placeholder materializes from auth identity or stays literal → request never dispatched)
- Comparison: equivalent
- Observable difference: none found (placeholder substitution, required-field semantics, gateway auth header).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi's `cloudflare-ai-binding.ts` (Workers `env.AI.fetch` passthrough) is a JS-runtime concept with no Rust counterpart — not applicable to a native SDK; recorded here rather than as a verdict.

### AI-AUTH-12 — OAuth callback server / manual-code race / trusted URLs
- Dimension(s): 16
- pi entry: `auth/oauth/callback-server.ts` (loopback HTTP server, redirect-URI, close), `pkce.ts` (S256), `device-code.ts` (RFC 8628 polling incl. slow_down), `trusted-url.ts` (https enforcement on verification URIs)
- Opi entry: `auth/oauth/mod.rs:133-160` (verifier/challenge/state gen), `:232-450` (callback binding with `CallbackBindFallback`, authorize styles, manual-code race against callback), `:817-1019` (code exchange + refresh with `RefreshFieldPolicy`), `:1021-1044` (oauth error classes), per-flow trusted URL checks (`trusted_https_url` xai.rs:67, `trusted_http_url` kimi_coding.rs:60, meta.rs:74), kimi slow-down increment kimi_coding.rs:38
- Comparison: equivalent
- Observable difference: none found (loopback bind + free-port/manual-paste fallback, state mismatch rejection — anthropic.rs:289-310, PKCE S256, device polling semantics).
- Evidence basis: source inspection
- Review outcome: self-reviewed

---

## Layer H log (active design material encountered in scope)

| Claim | Anchor | Authority wording | Observed maturity |
|---|---|---|---|
| TypeSafe System One experimental image input (`assets`/`images` forms, `TypeSafe-Danger-Zone-Images` header) | pi `api/system-one-shared.ts:53-54,216`, `api/typesafe-system-one.ts:9` | "experimental" / "Danger-Zone" | implemented (pi); absent in opi (AI-STR-16) |
| Codex WS `OpenAI-Beta: responses=experimental` | pi `api/openai-codex-responses.ts:1690`; opi `openai_codex_responses.rs:249`, ws_transport.rs:297-311 | beta header value "experimental" (provider-side label) | implemented both |
| Codex WS bun proxy-env TODO | pi `api/openai-codex-responses.ts:1015` ("TODO: remove this when bun supports proxy envs in websocket") | TODO with design content (proxy env passthrough on WS) | pi: workaround present; opi: explicit proxy config only (`http.rs:331-384`, ambient detection disabled) — different mechanism, no TODO |
| Deferred responses ("Ask a capable provider to return a durable handle") | pi `types.ts:435-437`, faux-only; opi `deferred.rs` module doc "No real remote deferred protocol is claimed here; the published seam serves scripted and custom backends" | pi: neutral option doc; opi: explicit non-claim | both scaffold/contract-level; no production backend either side (AI-STR-10) |
| `ApiKeyAuth.check` optional side-effect-free status hook | pi `auth/types.ts:123-133` | normative interface doc | designed + contract, unimplemented by any provider in 1.5.0; opi probe is the implemented analog (AI-AUTH-06) |

## Coverage honesty

Not inspected, and why:
- **bedrock-converse-stream.ts body/event mapping in depth** (53K): only credentials/retry/identity compared; Converse wire fields (thinking config, tool use, guardrails) were not field-by-field compared — outside this batch's four named deep dives; recommend a dedicated Bedrock record in the adapter batch.
- **mistral-conversations.ts, pi-messages.ts event-by-event**: verified presence, API identity, protocol shape (Mistral agents API; pi-messages POST `/messages` SSE of assistant-message events — pi_messages.rs implements the same protocol including `on_raw`), but not every event branch.
- **Azure adapter full matrix**: verified endpoint/deployment/api-version resolution parity (azure-openai-config.ts vs azure_openai.rs:3-9,138-142) but not every compat branch of the Azure responses wire.
- **Model catalogs/pricing** (models.rs 210K vs pi providers/*.models.ts + catalog generators): owned by a different measurement batch; only cost-multiplier/service-tier surface touched (stream.rs:79-93).
- **ADC scope strings, AWS SSO token-file byte formats, Smithy adaptive constants**: architecture and source-set verified; upstream-library-owned details not byte-compared.
- **opi-coding-agent credential_store.rs internals** (keychain envelope): confirmed existence and ownership only; auth.json vs keychain is an app-layer fact.
- **Executed probes**: none (offline constraint); every record is source/test inspection.
- **pi identity.ts full attribution mapping** (NVIDIA/OpenCode header specifics beyond OpenRouter): enumerated absence on opi side; pi side read to line 60 of identity.ts, header emission sites not all visited.
