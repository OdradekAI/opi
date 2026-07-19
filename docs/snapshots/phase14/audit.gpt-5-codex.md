# Phase 14 Independent Audit

**Auditor:** GPT-5 Codex  
**Date:** 2026-07-19  
**Scope:** Tasks 14.1 through 14.21  
**Implementation range:** `d9f21a97d0d93a57c1a84e248b9254ece2ea2bb8..8364e74a9077a194cb4a7fd68db2e3c4b420111a`  
**Audited tree:** `4758c090da55251f9ea74e2d7c90d9ee0d2b2c8c`

## Method and independence

The audit used the frozen Phase 14 ledger snapshot, `docs/opi-spec.md`, both
registered Phase 14 design sources, the implementation diff, current production
source, and owning tests. The commits after the final implementation commit
change ledgers, plans, audit guards, and archived evidence rather than Phase 14
runtime code.

Existing `audit.*.md`, evaluator reports, and target phase-exit report artifacts
were not read before reaching the findings and verdict. Two incidental
contamination events did not supply findings: an exact-term search returned one
line from a realignment document that repeated the normative manual URL-paste
requirement, and a frozen-ledger property dump exposed embedded historical
review notes.

Audit dimensions were correctness, security/redaction, test quality, spec
compliance, explicit invariants, cross-task integration, and residual risk.

## Executive verdict

**FAIL**

| Severity | Count |
|---|---:|
| Blocker | 0 |
| Major | 11 |
| Minor | 5 |
| Info | 0 |

The implementation is broad and heavily tested, and the credential-store,
canonical identity, concrete Copilot/Codex routing, TUI retry state machine,
usage subset validation, and atomic model-refresh foundations are generally
strong. It is not ready to close Phase 14 because several binding production
paths disagree with their contracts:

- the default Anthropic harness path never enables the required cache markers;
- model thinking selections are validated but not applied on Chat/Responses
  wires;
- custom provider auth and session-affinity defaults violate the promised lazy
  and opt-in boundaries;
- the Browser OAuth fallback, timeout, redaction, and diagnostic surfaces have
  material gaps; and
- explicit Codex cache/affinity disablement is ignored.

The major findings span provider serialization, provider construction, OAuth,
diagnostics, and tests. That is systemic rather than one isolated task defect.

## Findings

### 1.1 MAJOR: Anthropic `None` retention disables the required default cache markers

**File:** `crates/opi-ai/src/anthropic.rs`  
**Lines:** 768--794, 1565--1573  
**Spec ref:** `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:276--290`

**Cause:** `build_request_body` enables caching only when retention is neither
`None` nor `Disabled`. The binding task requires default and `Short` retention
to emit ordinary ephemeral markers, while only `Disabled` suppresses them.
An inline test explicitly preserves the incorrect unmarked `None` behavior, and
the factory test at
`crates/opi-coding-agent/tests/anthropic_cache_markers.rs:241--273` omits a
`None` case.

**Impact:** `crates/opi-agent/src/agent_loop.rs:90--104` constructs ordinary
production requests with `CacheRetention::None`. Consequently, capable
Anthropic models receive no cache markers in the normal harness path. Task
14.11's production acceptance and the English/Chinese provider documentation
are false for the default case.

**Fix:** For cache-capable models, map `None` and `Short` to ordinary ephemeral
markers, `Long` to one-hour markers when supported, and `Disabled` to no
markers. Replace the contrary unit test and add a factory-built `None` capture.

### 1.2 MAJOR: Anthropic marker search stops at a trailing role-only message

**File:** `crates/opi-ai/src/anthropic.rs`  
**Lines:** 1200--1229

**Cause:** Both reverse scans break after the first message with the requested
role even when that message contains no text block. A trailing tool-result or
image-only user message therefore prevents searching an earlier user text;
similarly, a tool-call/thinking-only assistant message prevents searching an
earlier assistant text.

**Impact:** Common tool-use conversations omit the reviewed "last user text"
and "last assistant text" cache markers, reducing cache reuse on the workload
where the feature matters most.

**Fix:** Break the outer loop only after a text block has actually been marked.
Add trailing tool-result, image-only, tool-call, and thinking-only cases to the
factory-level capture.

### 1.3 MAJOR: Thinking maps are validated but not applied on Chat or Responses wires

**Files:** `crates/opi-ai/src/provider.rs`,
`crates/opi-ai/src/openai_chat.rs`,
`crates/opi-ai/src/openai_responses.rs`  
**Lines:** `provider.rs:252--268`; `openai_chat.rs:1074--1078`;
`openai_responses.rs:239--291`  
**Spec ref:** `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:777--790`

**Cause:** Capability preflight resolves the selected
`request.thinking.level`, but body construction ignores that resolved mapping.
Chat serializes only the static compatibility `reasoning_effort`; Responses
serializes only its static configured effort. Neither consumes the request's
enabled flag/level and selected model map.

**Impact:** Supported selections pass preflight but are silently omitted or
replaced by a static profile value. This affects direct and mapped
Chat/Responses routes, including compatible subscription profiles.

**Fix:** Resolve the selected level through the chosen `ModelInfo` during body
construction and serialize its wire value. Add positive, remapped, off, and
unsupported captures for both wire families.

### 1.4 MAJOR: Custom Responses profiles enable session affinity by default

**Files:** `crates/opi-coding-agent/src/config.rs`,
`crates/opi-ai/src/model_info.rs`,
`crates/opi-ai/src/openai_responses.rs`  
**Lines:** `config.rs:1213--1219`; `model_info.rs:326--343`;
`openai_responses.rs:205--218,480--501`  
**Spec ref:** `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md:520--537`

**Cause:** TOML lowering uses `send_session_id_header.unwrap_or(true)`, and the
public Responses compatibility default is also `true`. The direct built-in
OpenAI default was reused for custom/proxy metadata even though custom/proxy
profiles must opt in. The owning custom-provider fixture explicitly sets the
flag to `false`, avoiding the omitted/default case.

**Impact:** Arbitrary custom endpoints receive stable session-derived
`prompt_cache_key` and session headers unless users know to opt out. This
violates the privacy/compatibility boundary and can break strict proxies.

**Fix:** Make custom/mapped Responses compatibility default to `false`, and set
`true` explicitly only for the built-in direct OpenAI profile. Test an omitted
flag at the production factory boundary.

### 1.5 MAJOR: Present custom-provider credentials are frozen at construction

**File:** `crates/opi-coding-agent/src/provider_factory.rs`  
**Lines:** 682--699, 784--822  
**Spec ref:** `docs/opi-spec.md:1687--1694`

**Cause:** The resolver-backed factory eagerly resolves an API key. When a
credential is present, `build_custom_provider` wraps it in
`StaticAuthResolver`; only an initially absent credential gets
`EnvAuthResolver`. The owning test removes the environment variable before
construction and sets it afterward, so it proves only the favorable absent-at-
construction branch.

**Impact:** In the normal present-at-startup case, environment/store rotation is
not observed by the next stream. Laziness depends on startup state, contrary to
Task 14.16's shared lazy-auth contract.

**Fix:** Give mapped custom providers a resolver that re-reads the selected
source for every stream. Add a test that starts with a credential present,
changes it between two streams, and verifies both wire captures.

### 1.6 MAJOR: Custom Bearer API keys are misclassified as revoked OAuth credentials

**Files:** `crates/opi-coding-agent/src/config.rs`,
`crates/opi-ai/src/anthropic.rs`,
`crates/opi-ai/src/openai_chat.rs`,
`crates/opi-ai/src/openai_responses.rs`  
**Lines:** `config.rs:931--948`; `anthropic.rs:1045--1071`;
`openai_chat.rs:1263--1284`; `openai_responses.rs:416--434`

**Cause:** Custom providers default to Bearer authentication, and each reusable
wire treats every Bearer 401/403 as `CredentialRevoked`. HTTP header scheme is
being used as a proxy for OAuth lifecycle even though production OAuth is
limited to three canonical providers and custom credentials come from an
`api_key_env`.

**Impact:** An ordinary invalid custom Bearer API key produces
`/login <custom-id>` remediation even though no such OAuth provider exists.
The user cannot follow the suggested recovery; the correct action is replacing
the API key.

**Fix:** Separate wire auth scheme from credential lifecycle. Only approved
OAuth/subscription profiles should produce `CredentialRevoked`; custom API-key
profiles should return a redacted `AuthFailed`.

### 1.7 MAJOR: Dedicated Codex ignores explicit cache/affinity disablement

**Files:** `crates/opi-ai/src/provider.rs`,
`crates/opi-ai/src/openai_codex_responses.rs`  
**Lines:** `provider.rs:48--64`;
`openai_codex_responses.rs:67--88,145--168,249--270`  
**Spec ref:** `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md:487--493,520--537`

**Cause:** The dedicated provider adds `prompt_cache_key` whenever a session id
is non-empty and derives `session-id` from the user session without consulting
`CacheRetention::Disabled`. It always emits `session-id` and
`x-client-request-id`. The Codex tests use `Short` and contain no disabled case,
while standard Chat/Responses tests do cover disabled suppression.

**Impact:** A valid library request that explicitly disables cache/affinity
still transmits a stable session identifier and enables prompt-cache affinity.

**Fix:** Compute effective affinity only when retention is not `Disabled`.
Omit `prompt_cache_key` and user-session-derived headers when disabled. If the
Codex protocol strictly requires a transport session header, use a fresh
request-local UUID and document/test that it is not session affinity.

### 2.1 MAJOR: PKCE manual fallback does not parse redirect URLs or validate their state

**File:** `crates/opi-coding-agent/src/oauth.rs`  
**Lines:** 166--213, 310--365, 1548--1563  
**Spec ref:** `docs/opi-spec.md:1724--1730`;
`docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:528--539,931--936`

**Cause:** Every string returned by `await_manual_code` is treated as a raw
authorization code and submitted verbatim. Redirect URL parsing, percent
decoding, and state validation exist only in the loopback callback path.

**Impact:** When loopback delivery is unavailable, pasting the documented
redirect URL sends the entire URL as the token endpoint's `code`; Browser login
fails. Its state is also never checked.

**Fix:** Normalize manual input into raw-code and redirect-URL forms. Parse and
percent-decode `code`, validate `state` when a URL supplies it, reject malformed
or mismatched inputs, and test the concrete Anthropic and Codex dispatchers.

### 2.2 MAJOR: OAuth login HTTP stages are not bounded by the flow timeout

**File:** `crates/opi-coding-agent/src/oauth.rs`  
**Lines:** 166--213, 279--294, 491--502, 1317--1355, 1400--1407

**Cause:** The shared OAuth client has no request timeout. PKCE's configured
timeout covers only callback/manual-code waiting; token exchange is explicitly
outside it. Copilot creates its total deadline only after device authorization
and performs the final Copilot-token exchange outside that deadline.

**Impact:** A stalled endpoint is unbounded by the application's login budget.
The interactive dispatcher can remain suspended in alternate/raw-terminal
handling until the network future eventually fails or is externally cancelled,
and the intended terminal restoration/persistence completion cannot run.

**Fix:** Configure a bounded request timeout and race every request plus body
decode against one flow deadline. Add delayed mock endpoints for initial
authorization, polling, PKCE exchange, and final Copilot exchange.

### 2.3 MAJOR: Untrusted OAuth `error` values can echo secrets into formatted errors

**File:** `crates/opi-coding-agent/src/oauth.rs`  
**Lines:** 426--445, 1161--1202

**Cause:** `token_endpoint_error` parses an arbitrary server-controlled
`error` string and interpolates it into `ProviderError`. Copilot device polling
does the same for unknown error values. Existing canaries place secrets in
descriptions or other body fields, not in `error` itself.

**Impact:** An OAuth endpoint or intermediary that reflects a submitted
authorization code, verifier, device code, GitHub token, or refresh token in
`error` can place that credential in diagnostics/logs. This violates the
phase's explicit redaction invariant.

**Fix:** Surface only a closed allowlist of recognized OAuth error codes, or a
fixed typed class, and never format arbitrary server strings. Add echo-in-
`error` canaries for every submitted secret class.

### 2.4 MAJOR: Doctor and model listing do not mirror live auth precedence

**Files:** `crates/opi-coding-agent/src/provider_factory.rs`,
`crates/opi-coding-agent/src/doctor.rs`  
**Lines:** `provider_factory.rs:356--365,833--852,1081--1117,1475--1510`;
`doctor.rs:446--451,675--700`

**Cause:** Live Anthropic auth uses stored OAuth, then
`ANTHROPIC_OAUTH_TOKEN`, then API-key environment input. The redacted
descriptor uses a store only when the global keychain option is selected and
otherwise checks only the API-key environment variable. It has no
`github-copilot` or `openai-codex` arms. Listing similarly checks only one
API-key env or a precomputed store marker.

**Impact:** Valid runtime credentials can produce false missing/unknown doctor
diagnostics and incomplete `--list-models` results. Canonical subscription
providers can be reported as unknown, and OAuth-env-only Anthropic can be
omitted.

**Fix:** Centralize a secret-free availability calculation that mirrors each
live provider's precedence. Include all canonical OAuth ids and Anthropic's
OAuth environment source. Test default configuration after `/login`,
OAuth-env-only Anthropic, and both subscription ids.

### 3.1 Minor: Codex invalid header values become retryable network errors

**Files:** `crates/opi-ai/src/provider.rs`,
`crates/opi-ai/src/openai_codex_responses.rs`  
**Lines:** `provider.rs:155--192`;
`openai_codex_responses.rs:160--185,316--319`

**Cause:** The shared validator ignores values and performs incomplete name
syntax validation. Reqwest then rejects the request builder, and Codex maps that
local error to retryable `Network` instead of non-retryable `RequestFailed`.

**Impact:** Invalid public request input can trigger pointless retries and the
wrong diagnostic class.

**Fix:** Parse names and values with `HeaderName`/`HeaderValue` before spawning
HTTP, preferably through `ProviderHeaders`, and add invalid-value/name captures.

### 3.2 Minor: Cumulative usage wraps parent totals above `u32::MAX`

**File:** `crates/opi-ai/src/stream.rs`  
**Lines:** 180--220

**Cause:** The accumulator stores parent totals as `u64` but converts them with
unchecked `as u32` casts in `as_usage`; optional child totals remain `u64`.

**Impact:** Extremely long-lived accumulation silently wraps input/output/cache
parents and can produce inconsistent usage and cost summaries.

**Fix:** Keep aggregate parents as `u64`, use checked/saturating conversion, or
migrate the public parent counters. Add a `u32::MAX + 1` boundary test.

### 3.3 Minor: Replacing a provider retains its previous dynamic catalog

**File:** `crates/opi-ai/src/registry.rs`  
**Lines:** 129--155, 217--234

**Cause:** Both provider replacement paths replace the provider object but do
not clear `dynamic_catalogs[id]`; resolution prefers that dynamic catalog over
the replacement's own models.

**Impact:** A replacement provider can continue listing/resolving the previous
provider's refreshed models.

**Fix:** Remove the id's dynamic catalog on replacement and add a refresh-then-
replace test.

### 3.4 Minor: Initial native-keyring construction is not single-flight

**File:** `crates/opi-coding-agent/src/native_keyring.rs`  
**Lines:** 52--75, 199--227

**Cause:** `install_native_keyring_with` observes zero leases under the mutex,
releases it, then invokes the constructor. Concurrent first callers can both
construct a native store even though only one is installed.

**Impact:** Startup may duplicate native backend initialization and side
effects. The final default-store lease lifecycle remains correct.

**Fix:** Add an initializing state/condition variable or another single-flight
primitive, plus a barrier-based concurrent test.

### 3.5 Minor: `credential.lock` is truncated before lock ownership

**File:** `crates/opi-coding-agent/src/credential_store.rs`  
**Lines:** 549--588

**Cause:** The lock file is opened with `truncate(true)` before `try_lock`
acquires ownership even though its contents are unused.

**Impact:** Contenders mutate the inode before holding the lock, and a stale
symlink/reparse point can cause an unrelated writable file to be truncated.

**Fix:** Remove truncation; optionally reject link/reparse targets and constrain
permissions.

## Per-task assessment

| Task | Verdict | Audit result |
|---|---|---|
| 14.1 | PASS-WITH-FINDINGS | Store/envelope/lock contracts are strong; findings 2.3, 2.4, and 3.5 remain. |
| 14.2 | FAIL | Findings 2.1--2.3 affect the concrete OAuth contract. |
| 14.3 | FAIL | Custom Responses opt-in and Codex disabled mappings are wrong. |
| 14.4 | FAIL | Default retention and trailing non-text marker placement violate the cache contract. |
| 14.5 | PASS-WITH-FINDINGS | Usage subset/weighting behavior passes; aggregate overflow remains. |
| 14.6 | PASS-WITH-FINDINGS | Atomic deterministic refresh passes; provider replacement can retain stale dynamic state. |
| 14.7 | PASS-WITH-FINDINGS | Guards/gates pass but did not cover the behavioral gaps above. |
| 14.8 | FAIL | Probe implementation exists, but doctor/listing do not mirror live auth. |
| 14.9 | FAIL | Persistence and restoration paths are strong; manual fallback, boundedness, and error redaction are incomplete. |
| 14.10 | FAIL | Built-in per-stream resolution passes, but unbounded login can stall interaction and custom Bearer remediation is invalid. |
| 14.11 | FAIL | The default production cache path and tool-use marker placement are not closed. |
| 14.12 | PASS-WITH-FINDINGS | Optional subset and cost contracts pass; aggregate overflow remains. |
| 14.13 | PASS-WITH-FINDINGS | Verification artifacts passed but encode/omit several incorrect boundary cases. |
| 14.14 | PASS-WITH-FINDINGS | Host selection and lease lifetime pass; concurrent first construction is not single-flight. |
| 14.15 | FAIL | Wire/pricing/canonical identity checks pass; thinking maps are not applied. |
| 14.16 | FAIL | Mapping validation passes; custom affinity, auth laziness, and Bearer lifecycle are wrong. |
| 14.17 | PASS | Three-wire catalog, token/base re-resolution, headers, images, and revocation captures were not contradicted. |
| 14.18 | FAIL | Dedicated route/catalog pass; disabled affinity and Browser OAuth gaps remain. |
| 14.19 | FAIL | Concrete registry/dispatcher exists; findings 2.1--2.3 remain. |
| 14.20 | PASS | Same-provider one-retry/no-duplicate and negative gates are well covered. |
| 14.21 | FAIL | Acceptance/docs claim default cache and negative-affinity coverage not present in production. |

## Invariant verification matrix

| Invariant | Code evidence | Test coverage / assessment |
|---|---|---|
| No opi-managed plaintext credential file | Keychain envelope plus `credential.lock`; no secret file writer found | Credential-store recursive secret/artifact scans pass |
| Corrupt/unknown/wrong-kind store data does not become absence/env fallback | Typed envelope and resolver branches preserve backend/malformed errors | Credential-store cases pass |
| Credential mutations use one cross-process lock and refresh re-reads under lock | `credential_store` mutation helpers and refresh double-check | Contention/timeout/prior-credential tests pass; lock truncation is finding 3.5 |
| Auth is resolved per stream for approved built-ins | Anthropic/Copilot/Codex live resolvers execute at stream start | Changed-store captures pass |
| Canonical ids only (`anthropic`, `github-copilot`, `openai-codex`) | Registry/factory/config guards reject old ids | Canonical identity tests pass |
| OAuth secrets never reach output/errors | Secret wrappers and many canary tests | Violated for arbitrary server `error` values (2.3) |
| Login flow is bounded and restores terminal state | RAII restoration exists; some polling/refresh stages have deadlines | Not all HTTP stages share a bound (2.2) |
| `Disabled` suppresses cache/session affinity | Chat and standard Responses gate mappings | Dedicated Codex violates it (1.7) |
| Default/short Anthropic retention emits ephemeral markers | Capability model and `Short` path exist | Default path violates it; factory test omits `None` (1.1) |
| Cache markers target last system/user text/assistant text/tool definition | Serializer post-processing owns all four marker sites | Trailing non-text messages violate user/assistant selection (1.2) |
| Every thinking selection uses the selected model's map | Preflight rejects unsupported levels | Positive mapping is not consumed by Chat/Responses (1.3) |
| Usage child subsets are optional, bounded by parents, and not double-counted | Usage validators and weighted cache cost | Focused usage tests pass; aggregate parent overflow is 3.2 |
| Dynamic refresh is atomic, deterministic, substrate-only | Registry stages all catalogs before commit | Refresh tests pass; replacement invalidation gap is 3.3 |
| Custom mapped routes share one lazy current auth source | `ApiMappedProvider` shares an `AuthResolver` | Factory freezes credentials already present at construction (1.5) |
| TUI retries exactly once only after same-provider explicit login | Outer state machine owns pending turn and retry gates | Positive/negative TUI captures pass |

## Verification executed

The following independent checks passed:

- `cargo fmt --check --all`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p opi-ai` (all unit, integration, and doctests)
- `cargo test -p opi-coding-agent --test credential_store --test oauth_auth --test interactive_auth --test doctor_cli`
- `cargo test -p opi-coding-agent --test anthropic_cache_markers`
- `cargo test -p opi-coding-agent --test custom_provider_map`
- `cargo test -p opi-coding-agent --test github_copilot_provider --test openai_codex_provider`
- `cargo test -p opi-coding-agent --test interactive_tui_auth`
- `cargo test -p opi-coding-agent --test session_runtime`
- focused `opi-ai` auth, OAuth, request-enrichment, model-wire, usage,
  mapped-provider, collection, and Codex suites

The green suite is important evidence, but several tests encode an incorrect
expectation or select only the favorable branch:

- Anthropic's unit test expects `None` to remain unmarked.
- The factory cache test omits `None`.
- The custom lazy-auth test removes the credential before construction.
- The custom Responses fixture explicitly sets affinity to false.
- Thinking tests cover rejection but not positive wire application.
- Codex affinity tests omit `Disabled`.
- OAuth redaction canaries do not place a secret in the server's `error` field.

## Residual recommendations

1. Fix the eleven Major findings before declaring Phase 14 closed.
2. Add one regression test per finding at the highest production boundary
   identified above; do not close them with source-text guards alone.
3. Re-run the Phase 14 acceptance matrix, full workspace all-targets tests,
   clippy, doctests, rustdoc, and smoke gate after repairs.
4. Preserve the accepted non-goals: no plaintext credential fallback, no new
   OAuth providers/aliases, no automatic login, no schema redesign, and no
   production trigger for dynamic refresh.
