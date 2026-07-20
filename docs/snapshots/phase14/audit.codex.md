# Phase 14 Provider Authentication, Wire Routing, and Usage Accounting -- Independent Code Audit

**Auditor**: gpt5 (independent, no prior audit reports consulted)

**Date**: 2026-07-20

**Scope**: Tasks 14.1--14.21, commits `d9f21a97d0d93a57c1a84e248b9254ece2ea2bb8..8364e74a9077a194cb4a7fd68db2e3c4b420111a`

**Audited baseline**: `9263114731b0cdd3706769a001fedbe227da6109`

**Method**: Full reads of the Phase 14 task ledger, both registered Phase 14 design specifications, the current technical specifications, and the affected provider/auth/runtime/test/documentation surfaces; task-DoD and invariant tracing; targeted negative-path analysis; acceptance-manifest inspection; and full workspace build, format, lint, test, doctest, and documentation gates.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|------:|
| Blocker  | 0 |
| Major    | 10 |
| Minor    | 4 |
| Info     | 0 |

The current baseline passes every ordinary workspace gate, and the credential-store, native-keyring, dispatcher, cache-marker, and most catalog/routing paths are well covered. It nevertheless has systemic contract failures at the provider-stream boundary, multiple dedicated Codex edge-case failures, a live-versus-resumed usage-accounting divergence, an upstream-error redaction gap, and acceptance artifacts that silently execute zero tests while claiming Phase 14 closure.

The phase is not ready to be treated as closed. The highest-priority work is to restore the lazy/cancellable/typed stream contract, prevent raw provider payloads from becoming public/session error text, fix Codex terminal OAuth and disabled-affinity behavior, and replace the documentary acceptance manifest with an executable gate that rejects zero-test selections.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | PASS |
| 14.2 | OAuth architecture and per-request auth re-resolution | FAIL |
| 14.3 | Request scalars and session-affinity production path | FAIL |
| 14.4 | Model capabilities and Anthropic cache markers | PASS |
| 14.5 | Usage and cost cache/reasoning accounting | FAIL |
| 14.6 | Dynamic provider model refresh | PASS-WITH-FINDINGS |
| 14.7 | Provider/auth docs, non-goal guards, and final Phase 14 gates | FAIL |
| 14.8 | Native keyring and production probes | PASS |
| 14.9 | Login/logout dispatcher and persistence | PASS |
| 14.10 | Live auth and session interaction | FAIL |
| 14.11 | Factory-built Anthropic cache markers | PASS |
| 14.12 | Usage and cost contract | FAIL |
| 14.13 | Documentation, verification, and residual closure | FAIL |
| 14.14 | Native keyring host selection | PASS |
| 14.15 | WireApi, model metadata, pricing, thinking, and canonical IDs | FAIL |
| 14.16 | ApiMappedProvider and TOML custom providers | PASS-WITH-FINDINGS |
| 14.17 | GitHub Copilot three-wire catalog | FAIL |
| 14.18 | OpenAI Codex dedicated wire, catalog, and dual login | FAIL |
| 14.19 | Concrete OAuth dispatcher vertical path | FAIL |
| 14.20 | Outer TUI credential retry | PASS |
| 14.21 | Documentation, acceptance artifacts, and Phase F | FAIL |

### Verification executed

The following gates passed on the audited baseline:

```text
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo test -p opi-ai --all-targets
cargo test -p opi-coding-agent --test interactive_tui_auth outer_tui_same_provider_login_retries_pending_turn_once
```

Passing these gates does not contradict the findings below: several failures are untested, two current acceptance commands select zero tests, and some existing tests explicitly assert the incorrect behavior.

---

## 2. Correctness and Provider-Lifecycle Findings

### 2.1 MAJOR: Provider streams perform auth resolution and network I/O before first poll

**Files:** `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** `anthropic.rs:1276--1299`; `openai_chat.rs:1495--1517`; `openai_responses.rs:541--563`; `openai_codex_responses.rs:278--300`
**Spec refs:** `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:229,814`; task 14.10 first-poll DoD
**Cause:** Each `Provider::stream` implementation creates a channel and immediately calls `tokio::spawn`. The spawned future resolves credentials and can begin the HTTP request before the returned event stream is polled. Dropping that stream only drops the receiver; it does not cancel the detached task.
**Impact:** Merely constructing and dropping a stream can refresh credentials and transmit user content. The implementation also requires a Tokio runtime at stream construction and can panic outside one, despite the contract defining work as lazy on first poll.
**Fix:** Return a stream that owns and polls the request future. Tie cancellation and drop to that owned future rather than a detached task. Add tests that construct and drop an unpolled stream and assert zero credential-resolver calls and zero HTTP requests.

### 2.2 MAJOR: Request cancellation terminates direct provider streams as clean EOF

**Files:** `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** `provider.rs:82--84`; `anthropic.rs:973--977`; `openai_chat.rs:1198--1202`; `openai_responses.rs:417--425`; `openai_codex_responses.rs:200--208`
**Cause:** The stream contract says cancellation yields `ProviderError::Cancelled`, but the HTTP loops return `Ok(())` when the request cancellation token fires. Their spawned wrappers forward only `Err`, so the sender is dropped and the consumer observes clean EOF. `openai_chat_fixtures.rs:1631--1677` currently enshrines this behavior.
**Impact:** Direct `opi-ai` consumers cannot distinguish cancellation from a provider that ended without a terminal event. The coding-agent loop has an independent cancellation select that masks part of the issue, but the public provider contract remains broken.
**Fix:** Emit or return `ProviderError::Cancelled` on the cancellation branch and add direct-provider tests for all four implementations. The tests must assert the typed error, not only stream termination.

### 2.3 MAJOR: Timeouts after response headers are downgraded to non-retryable stream errors

**Files:** `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/provider.rs`
**Lines:** `anthropic.rs:986`; `openai_chat.rs:1211`; `openai_responses.rs:425`; `openai_codex_responses.rs:208`; `provider.rs:363--367`
**Cause:** Once response headers have arrived, every body-stream error is mapped to `StreamError`. Reqwest's per-request timeout remains active while the response body is read, so a body timeout arrives through this branch but loses its `Timeout` classification. `StreamError` is non-retryable while `Timeout` is retryable.
**Impact:** A stalled SSE body is not retried even though an equivalent pre-header timeout is retried. Provider behavior depends on the exact point at which the same timeout occurs.
**Fix:** Classify body errors with the same timeout/connect/request mapping used for initial request errors. Add fixtures that send headers and then stall the body for each provider family.

### 2.4 MAJOR: Codex device polling treats terminal 403/404 OAuth responses as pending

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** `1080--1090`; tests at `crates/opi-coding-agent/tests/oauth_auth.rs:2715--2730,2787--2829`
**Cause:** The Codex Device Code poller maps every HTTP 403 or 404 to `Pending` before parsing a structured OAuth error code. The denial/expiry tests cover terminal codes only with HTTP 400, while a 403 fixture explicitly asserts pending.
**Impact:** A valid `access_denied` or expiration response delivered with 403/404 waits until the maximum polling timeout, potentially about fifteen minutes, and is then reported as a timeout instead of the real terminal outcome.
**Fix:** Parse recognized terminal OAuth codes before applying status-based fallback. Treat 403/404 as pending only when the payload does not contain a recognized terminal code. Add 403 and 404 denial/expiry tests.

### 2.5 MAJOR: A failed credential turn followed by a different prompt changes usage totals after resume

**Files:** `crates/opi-agent/src/agent.rs`, `crates/opi-coding-agent/src/harness.rs`, `crates/opi-coding-agent/src/session_coordinator.rs`, `crates/opi-ai/src/stream.rs`
**Lines:** `agent.rs:166--177`; `harness.rs:1468--1480`; `session_coordinator.rs:154--158,191--198,244--253,295--303`; `stream.rs:189--215`
**Spec ref:** `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:333--340`
**Cause:** The agent appends the first user prompt before provider execution. On a credential failure, the harness returns without persisting that turn or advancing its persistence offset. If the user submits a different prompt, the next successful persistence slice contains two user messages but `on_turn_end` calls `CumulativeUsage::accumulate` only once. Replay reconstructs `turn_count` by counting both persisted user messages.
**Impact:** The live process reports one turn while the resumed session reports two for the same persisted history. Cost totals and any behavior keyed to cumulative turn count can diverge across restart.
**Fix:** Give abandoned failed turns an explicit persistence/accounting boundary before accepting a fresh prompt, while preserving the existing no-duplicate behavior for `retry_last_turn`. Add an outer-TUI session test that fails auth, submits a different prompt, persists, resumes, and compares the complete cumulative usage structure.

---

## 3. Security, Redaction, and Session-Affinity Findings

### 3.1 MAJOR: Raw upstream SSE errors can be persisted and exposed without redaction

**Files:** `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_responses_shared.rs`, `crates/opi-agent/src/event.rs`, `crates/opi-agent/src/agent_loop.rs`
**Lines:** `anthropic.rs:591--598`; `openai_chat.rs:690--697,1121--1124,1225--1228`; `openai_responses.rs:340--342,436--439`; `openai_responses_shared.rs:512--519`; `event.rs:234--251`; `agent_loop.rs:674--675`
**Cause:** Managed Anthropic, OpenAI Chat/Copilot, and Responses paths copy raw upstream error messages or malformed SSE frame data into assistant stream errors. Public-event redaction clones `error_message` unchanged, and the agent loop promotes that value to final assistant error text. The dedicated Codex path already uses a safer generic mapping, demonstrating that raw payload inclusion is not required.
**Impact:** An upstream proxy or provider that echoes authorization material, request fragments, or user content can place it in terminal output, JSON events, traces, or persisted session content.
**Fix:** Convert provider payloads to bounded, provider-neutral public error messages. Retain sensitive detail only in an explicitly redacted diagnostic channel. Add sentinel-secret tests covering structured upstream errors and malformed SSE data for every provider family.

### 3.2 MAJOR: Disabled cache retention still emits generated Codex affinity identifiers

**File:** `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** `171--172,269--275`; tests at `211--315`
**Spec ref:** `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md:490--492,521`
**Cause:** When `CacheRetention::Disabled` suppresses the derived session affinity, the Codex route falls back to a new UUID and still emits both `session-id` and `x-client-request-id`. Existing tests assert the generated identifiers instead of the specified omission.
**Impact:** An explicit request to disable cache/session affinity is ignored for the dedicated Codex wire, and every request still carries correlation identifiers.
**Fix:** Keep both identifiers optional and omit them when retention is disabled or the mapping is empty. Update the tests to assert header absence.

---

## 4. Model, Wire, and Spec-Compliance Findings

### 4.1 MAJOR: OpenAI Chat and Responses silently omit unsupported thinking levels

**Files:** `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`
**Lines:** `provider.rs:276--284`; `openai_chat.rs:1087--1092`; `openai_responses.rs:321--326`; tests at `crates/opi-ai/tests/model_wire_metadata.rs:213--285`
**Spec ref:** task 14.15 DoD and `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md:1078`
**Cause:** Shared validation exempts OpenAI Chat and Responses from unsupported-thinking rejection. The request builders then omit the unresolved level and continue to HTTP. The wire-metadata tests explicitly assert that the request reaches the server. This follows `docs/opi-spec.md:1700--1704`, but contradicts the registered corrective design and task DoD that require rejection before request construction.
**Impact:** Phase completion claims and production behavior disagree, and a user receives a successful request with silently changed reasoning semantics.
**Fix:** Resolve the normative conflict explicitly. To satisfy the recorded task DoD, reject unsupported levels with a typed pre-I/O error and update the tests. If silent omission is intended product behavior, amend the corrective design and ledger claim rather than leaving contradictory normative sources.

### 4.2 MAJOR: The dedicated Codex provider sends unknown or cross-provider model IDs

**Files:** `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-coding-agent/src/harness.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/api_mapped.rs`
**Lines:** `provider_factory.rs:1375--1394`; `harness.rs:454--469,988--1000`; `openai_codex_responses.rs:256--265`; `api_mapped.rs:172--193`
**Cause:** The factory constructs the dedicated Codex route directly, initial harness construction does not run the model reconfiguration validator, and the route strips any provider prefix before sending. Its catalog lookup only helps choose a base URL and does not reject an unknown model. `ApiMappedProvider` correctly performs typed catalog/wire validation, but the dedicated route bypasses it.
**Impact:** Typos and cross-provider model specs can cause credential resolution and network I/O before failing remotely, violating the pre-I/O unknown-model invariant and potentially sending a request to the wrong wire.
**Fix:** Put the dedicated route behind the same catalog/wire validation boundary or add equivalent validation inside it. Cover unknown bare IDs and cross-provider prefixes with zero-resolver/zero-HTTP tests.

---

## 5. Test-Quality and Documentation Findings

### 5.1 MAJOR: Phase acceptance artifacts claim coverage while selected commands run zero tests

**Files:** `docs/superpowers/plans/2026-07-17-phase14-pi-0806-alignment.md`, `docs/snapshots/phase14/opi-impl-state.json`, `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`
**Lines:** plan `1331,1341,1380`; snapshot `110,401,701,875--876,1010,2086,2298,2363`; docs guard `191--209`
**Cause:** The final plan correctly states that zero selected tests are a failure, but two of its own 58 mandatory commands select zero tests:

```text
cargo test -p opi-ai --test model_wire_metadata unsupported_thinking_level_is_rejected_before_request_build
cargo test -p opi-coding-agent --lib oauth_login_restores_terminal_after_flow_failure
```

The snapshot also carries a third zero-selection filter:

```text
cargo test -p opi-coding-agent --test interactive_auth interactive_explicit_login_retries_pending_turn_once
```

The replacements have different names, including `strict_wire_unsupported_thinking_level_is_rejected_before_http`, `dispatcher_restores_terminal_once_on_every_concrete_exit`, and `outer_tui_same_provider_login_retries_pending_turn_once`. Five snapshot `behavioral_tests` paths do not exist: `doctor.rs`, `request_enrichment_wiring.rs`, `anthropic.rs`, `model_capabilities_wiring.rs`, and `usage_cost_wiring.rs`. The docs guard verifies only the literal phrase `"58-row acceptance manifest"`; it neither parses nor executes the manifest.
**Impact:** Phase-exit evidence can remain green while mandatory acceptance rows execute no tests. The artifact cannot reliably prove task completion or detect renamed/deleted coverage.
**Fix:** Replace prose-only command rows with a machine-readable executable manifest. For every filtered Cargo command, enumerate first and fail unless at least one test matches; also validate every declared test path. Make the documentation test invoke this validator.

### 5.2 Minor: The localized technical specification begins with corrupt preamble bytes

**File:** `docs/opi-spec.zh.md`
**Lines:** `1`
**Cause:** The file begins with the literal characters `e'x#` before the intended title, so it has no valid leading H1.
**Impact:** The rendered localized specification has a malformed title and signals that the localized artifact was not structurally validated.
**Fix:** Remove the stray prefix and add a lightweight Markdown structure check for both technical-spec variants.

### 5.3 Minor: Technical specifications still describe provider credentials as build-time validation

**Files:** `docs/opi-spec.md`, `docs/opi-spec.zh.md`
**Lines:** `docs/opi-spec.md:1580--1582`; `docs/opi-spec.zh.md:1341--1344`
**Cause:** Both documents state that provider construction validates credentials at build time. Phase 14 moved managed credentials to per-stream resolution so login/logout/refresh can take effect without rebuilding the provider.
**Impact:** Embedders can implement the old lifecycle and misunderstand when missing credentials are reported.
**Fix:** Describe construction as structural/provider-profile validation and credential resolution as a per-stream operation. Keep the localized text synchronized.

---

## 6. Cross-Task Invariant Findings

### 6.1 Minor: `ApiMappedProvider` cannot enforce the shared-auth-resolver invariant

**File:** `crates/opi-ai/src/api_mapped.rs`
**Lines:** `24--28,47--53`
**Cause:** `ApiMappedProvider::try_new` accepts already-built boxed route providers. Its documentation says callers `"should"` share one resolver, but the type and constructor cannot verify or establish that invariant.
**Impact:** Current production construction shares the resolver correctly, but embedders can create one logical provider whose wires observe different login/logout state.
**Fix:** Construct routes from a shared resolver inside the mapped provider, or make the resolver identity part of route construction and validate it in `try_new`.

### 6.2 Minor: Dynamic refresh installs model catalogs without validating IDs or duplicates

**Files:** `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/src/registry.rs`
**Lines:** `provider_collection.rs:442--468`; `registry.rs:339--352`
**Cause:** Initial configured model overrides are validated, but refreshed `ModelInfo` values replace the registry catalog directly without equivalent identifier and duplicate checks.
**Impact:** A malformed dynamic catalog can create ambiguous or unreachable models until the next successful refresh. The atomic replacement behavior is sound, but the candidate catalog is not validated before commit.
**Fix:** Apply the same canonical-ID and uniqueness validation to the complete refresh candidate before replacing the active catalog. Preserve the previous catalog on validation failure and add malformed/duplicate refresh fixtures.

---

## 7. Invariant Verification

| Invariant | Code evidence | Test coverage / assessment |
|-----------|---------------|----------------------------|
| Plaintext credential material is not persisted | Credential records store handles/metadata; native secret material remains behind `CredentialStore` | Strong positive and negative coverage; PASS |
| Corrupt credential data fails closed | Credential parsing rejects malformed records rather than guessing | Covered; PASS |
| Native keyring host selection is explicit and deterministic | Platform/native host selection is centralized | Production probes and host-selection tests pass; PASS |
| Doctor/model listing remain secret-free | Diagnostic projections expose availability/state, not secret values | Covered; PASS |
| Login/logout persistence and terminal restoration are centralized | Concrete dispatcher owns persistence and presenter lifecycle | Concrete-exit tests pass; PASS |
| Managed auth is resolved on first stream poll | Provider `stream` methods immediately spawn resolver/request futures | Missing unpolled-drop tests; FAIL (2.1) |
| Request cancellation is a typed `Cancelled` error | Provider contract declares it; HTTP loops return clean success | Existing Chat fixture asserts EOF; FAIL (2.2) |
| Request timeout remains typed and retryable across the whole body | Initial request errors are classified; body errors are flattened | Only pre-header timeout coverage; FAIL (2.3) |
| Provider/public errors do not expose upstream payload secrets | Dedicated Codex genericizes errors; other routes copy raw payloads | No sentinel coverage for SSE error payloads; FAIL (3.1) |
| Disabled retention omits session-affinity identifiers | General affinity mapping can return `None`; Codex generates UUID fallback | Tests assert the wrong fallback; FAIL (3.2) |
| Unknown/cross-provider models fail before auth or HTTP | `ApiMappedProvider` validates the catalog/wire mapping | Dedicated Codex bypasses the boundary; FAIL (4.2) |
| Unsupported thinking levels obey one explicit policy | Some routes reject; Chat/Responses silently omit | Tests and normative specs conflict; FAIL (4.1) |
| Live cumulative usage equals replayed cumulative usage | Coordinator persists turn slices and reconstructs from user entries | Failed-turn/new-prompt path is uncovered; FAIL (2.5) |
| Dynamic refresh is atomic and preserves the old catalog on fetch failure | Candidate fetch completes before registry replacement | Atomicity covered; candidate validation missing; PARTIAL (6.2) |
| All wires of one logical provider share auth state | Production factory passes one resolver | Public mapped-provider constructor cannot enforce it; PARTIAL (6.1) |
| Every acceptance filter executes at least one test | Plan states this invariant | Three stale filters execute zero tests; FAIL (5.1) |
| English and localized technical specifications remain synchronized and valid | Documentation guards compare selected snippets | Structural corruption and stale lifecycle text remain; FAIL (5.2, 5.3) |

---

## 8. Success-Criterion Assessment

| Capability area | Assessment | Evidence |
|-----------------|------------|----------|
| Credential store model and secret-free diagnostics | PASS | Store/keyring/doctor tests and code paths agree with the design |
| OAuth architecture and concrete dispatcher | FAIL | Codex 403/404 terminal codes are classified as pending |
| Per-request auth and stream lifecycle | FAIL | Work begins before first poll; cancellation and body timeout lose typed semantics |
| Request scalars and affinity | FAIL | Dedicated Codex ignores disabled retention |
| Model capabilities and Anthropic cache markers | PASS | Factory-built capability/cache-marker paths are wired and covered |
| Usage and cost accounting | FAIL | Failed-auth/new-prompt history changes `turn_count` after resume |
| Dynamic provider catalogs and wire routing | PARTIAL | Core mapping/atomic refresh work, but Codex pre-I/O validation and refresh validation are incomplete |
| Documentation and Phase F acceptance closure | FAIL | Zero-selection commands, nonexistent ledger paths, malformed/stale localized documentation |

The Phase 14 non-goals remain respected: the implementation does not turn MCP, production subagents, permission gates, or plan/todo workflows into new built-in core workflows.

---

## 9. Residuals and Recommendations

### Priority recommendations

1. Rework all four managed stream implementations around one owned lazy future, with shared typed cancellation, timeout, and sanitized-error classification.
2. Fix the dedicated Codex route: parse terminal device-flow codes before status fallback, omit affinity IDs when disabled, and validate model/wire mapping before auth or I/O.
3. Define one normative unsupported-thinking policy, then align the corrective design, technical spec, implementation, and tests.
4. Add the failed-auth/different-prompt persistence boundary and prove that live and resumed cumulative usage are identical.
5. Replace the documentary 58-row acceptance list with an executable manifest that fails on missing files and zero-test filters.
6. Repair and synchronize both technical specifications, then add structural Markdown and lifecycle-statement guards.

### Exit condition for a follow-up audit

A follow-up should require all fourteen findings to be resolved or explicitly accepted in the normative design, plus:

- all ordinary workspace gates listed in section 1;
- direct tests for unpolled stream drop, typed cancellation, and post-header timeout on every affected provider family;
- sentinel-secret upstream SSE tests;
- 403/404 terminal Codex OAuth tests;
- disabled-retention header-absence tests;
- zero-I/O unknown/cross-provider Codex model tests;
- a live-versus-resume failed-auth/new-prompt usage test; and
- an acceptance-manifest validator demonstrating that every declared command selects at least one test.
