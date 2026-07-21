# Phase 14 Provider & Auth -- Independent Code Audit

**Auditor**: codex (independent, no prior audit reports consulted)
**Date**: 2026-07-20
**Scope**: Tasks 14.1--14.21; task commits `d9f21a9` through `8364e74`; current implementation assessed at `b27905a`
**Method**: Full read of the Phase 14 ledger, all three normative specifications, affected source/tests/docs, and checked-in pi-0.80.6 fixtures; task/criterion/invariant tracing; three independent file-group reviews; fresh workspace and focused verification.

No `docs/snapshots/phase14/audit.*.md`, remediation plan, evaluator report, or review transcript was consulted. The ledger's short `phase_exit.evaluator_summary` was used only as structural metadata.

---

## 1. Executive Summary

**Verdict: FAIL**

| Severity | Count |
|----------|------:|
| Blocker  | 0 |
| Major    | 7 |
| Minor    | 2 |
| Info     | 0 |

Phase 14 has substantial, well-tested substrate, and the full workspace gates are green. However, seven Major findings affect normal Responses/Codex streaming, panic safety for untrusted provider data, cancellation, thinking enforcement, mixed-model cost accounting, and typed Codex authentication remediation. These span several core Phase 14 success criteria, so the phase should not be treated as closed despite the absence of a direct credential disclosure.

### Per-task summary

| Task | Title | Verdict |
|------|-------|---------|
| 14.1 | Credential store model | PASS |
| 14.2 | OAuth architecture and per-request auth re-resolution | MAJOR findings 2.3, 3.3 |
| 14.3 | Request scalars and session-affinity production path | PASS |
| 14.4 | Model capabilities and Anthropic cache markers | MAJOR findings 2.2, 3.1 |
| 14.5 | Usage and cost cache/reasoning accounting | MAJOR finding 2.4 |
| 14.6 | Dynamic provider model refresh | PASS |
| 14.7 | Provider/auth docs, non-goal guards, and final gates | PASS with test-quality gaps |
| 14.8 | Native keyring and production probes | PASS |
| 14.9 | Login/logout dispatcher and persistence | MAJOR/MINOR findings 2.3, 3.3, 4.1, 4.2 |
| 14.10 | Live auth and session interaction | MAJOR findings 3.2, 3.3 |
| 14.11 | Factory-built Anthropic cache markers | MAJOR finding 2.2 |
| 14.12 | Usage and cost contract | MAJOR finding 2.4 |
| 14.13 | Documentation, verification, and residual closure | PASS with test-quality gaps |
| 14.14 | Native keyring host selection | PASS |
| 14.15 | WireApi, model metadata, pricing, thinking, canonical IDs | MAJOR findings 2.4, 3.1 |
| 14.16 | ApiMappedProvider and TOML custom providers | MAJOR finding 3.1 |
| 14.17 | GitHub Copilot three-wire catalog | MAJOR findings 2.1, 3.1, 3.3 |
| 14.18 | OpenAI Codex dedicated wire, catalog, dual login | MAJOR findings 2.1, 2.3, 3.1--3.3 |
| 14.19 | Concrete OAuth dispatcher vertical path | MAJOR/MINOR findings 2.3, 3.3, 4.1, 4.2 |
| 14.20 | Outer TUI credential retry | MAJOR finding 3.2 |
| 14.21 | Documentation, acceptance artifacts, Phase F | Acceptance did not expose the Major findings |

---

## 2. Correctness and Protocol Findings

### 2.1 MAJOR: Responses/Codex normal SSE frames become terminal errors

**Files:** `crates/opi-ai/src/openai_responses_shared.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** shared `49--53`, `199--209`, `326--328`, `529--539`; Responses `439--459`; Codex `227--253`
**Spec refs:** Tasks 14.17--14.18; Phase 14 pi-0.80.6 wire alignment

**Cause:** `ResponsesEvent::try_from_frame` dispatches exclusively on the SSE `event:` field. A data-only frame therefore defaults to event type `message`, and every unrecognized type becomes `ResponsesEvent::Error`, which immediately marks the mapper terminal. Canonical Codex SSE is data-only and carries the actual type in JSON (`{"type":"response.output_item.added", ...}`), as demonstrated by `.repo/pi-0.80.6/packages/ai/test/openai-codex-stream.test.ts:48--86`.

Even when a server supplies `event:`, the decoder treats normal protocol events such as `response.in_progress`, `response.content_part.done`, `response.function_call_arguments.done`, reasoning summary/text events, `response.incomplete`, and Codex `response.done` as errors. The checked-in pi handler parses JSON `type`, handles reasoning and function-call completion, normalizes terminal variants, and ignores benign lifecycle extensions.

**Impact:** A normal Codex response can fail on its first frame. Standard Responses/Copilot routes can terminate on ordinary lifecycle events, lose reasoning or final tool arguments, and map incomplete/length termination to an error.

**Fix:** Decode the JSON `type` field, using the SSE event name only as a validated fallback. Explicitly handle function-call completion, reasoning, incomplete/failed, and Codex terminal forms; ignore unknown non-error extensions. Add data-only, realistic full-sequence fixtures for both HTTP paths.

### 2.2 MAJOR: Anthropic block events can underflow and panic

**File:** `crates/opi-ai/src/anthropic.rs`
**Lines:** `468--469`, `517--518`

**Cause:** `ContentBlockDelta` and `ContentBlockStop` compute `self.blocks.len() - 1`. A syntactically valid event delivered before any block start underflows `usize`. Both arms also discard the upstream `index` and mutate the last block, so out-of-order/interleaved indices can update the wrong content.

**Impact:** Malformed or reordered upstream input can panic a direct mapper caller. In the HTTP task it can terminate the producer without a typed terminal error, violating the provider failure-in-band contract.

**Fix:** Use the event index with safe `get`/`get_mut`; return a non-retryable `StreamError` for missing, out-of-range, or type-mismatched blocks. Add delta-before-start, stop-before-start, wrong-index, and interleaved-index tests.

### 2.3 MAJOR: Provider-controlled OAuth duration fields can panic

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** `427--433`, `534--536`, `1028--1035`, `1179--1240`, `1717--1720`, `1763--1767`
**Spec refs:** `docs/opi-spec.md:61`, `1754--1758`; exit-remediation design `946--948`

**Cause:** Server-controlled `expires_in: i64` is added to `OffsetDateTime` with the panicking `Add` implementation. The locked `time 0.3.47` implementation uses `checked_add(...).expect(...)`. Device endpoints can also supply `u64::MAX` as the polling interval; a later `slow_down` performs unchecked `interval += 5s` (and an enormous interval may already overflow timer construction).

**Impact:** A malformed or malicious OAuth response can crash the process instead of returning a typed, redacted provider error. This affects login, refresh, GitHub Copilot Device Code, and OpenAI Codex Device Code paths.

**Fix:** Validate duration bounds and use checked date/duration arithmetic. Reject unrepresentable values with fixed typed errors. Add adversarial `i64::{MIN,MAX}` and `u64::MAX` login/refresh/device tests.

### 2.4 MAJOR: Session cost reprices historical usage with the current model

**Files:** `crates/opi-coding-agent/src/harness.rs`, `crates/opi-coding-agent/src/session_coordinator.rs`, `crates/opi-coding-agent/tests/session_runtime.rs`
**Lines:** harness `1173--1178`; coordinator `609--624`, `646--650`; test `1280--1295`, `1371--1428`
**Spec refs:** Tasks 14.5, 14.12, 14.15; SC6

**Cause:** `sync_session_cost_model` replaces one session-wide model/pricing value. `cost_summary` then applies that current price to all cumulative usage. It also selects a pricing tier using cumulative session input tokens, although a model threshold applies to each request.

The lifecycle test encodes the defect: after recording $18 of Sonnet usage, selecting Opus performs no request but changes the asserted total to $42. The test named `embedded_model_pricing_updates_on_model_switch_and_resume` switches before recording any usage and therefore never tests mixed-model history.

**Impact:** Model selection retroactively changes already-incurred cost. Mixed-model resume/fork summaries are wrong, and several individually below-threshold calls can cumulatively cross a tier and reprice the entire session.

**Fix:** Compute and accumulate cost per completed turn using that turn's model and input-token tier. For resume/fork, replay usage against initial-model/model-change segments or persist an additive non-authoritative priced-usage breakdown consistent with the session compatibility constraints. Replace the retroactive assertions with invariance tests.

---

## 3. Contract and Cross-Task Integration Findings

### 3.1 MAJOR: Thinking-level enforcement is optional and inconsistent

**Files:** `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-coding-agent/src/harness.rs`
**Lines:** provider `225--284`; Chat `1096--1101`; Responses `321--326`; Codex `111--125`; harness `1016--1078`, `1189--1199`
**Spec refs:** exit-remediation design `1055--1081`; `docs/opi-spec.md:1702--1706`

**Cause:** Unsupported-level validation lives in a standalone helper that public `Provider::stream`, `ApiMappedProvider`, and `ProviderCollection` do not enforce. Chat/Responses silently omit reasoning after a mapping error, and Codex falls back to the raw level name. The tests' `run_production_request` helper manually calls the validator first, masking the public stream behavior.

The helper also resolves the level when `thinking.enabled == false`, contrary to its own contract. Conversely, `CodingHarness::set_thinking_level` checks only broad thinking capability and token budget, not `thinking_level_map`, so it can accept and persist `xhigh`/`max` for a model that will reject the next prompt.

**Impact:** Direct provider/mapped-provider callers can perform HTTP instead of receiving the required pre-network `UnsupportedCapability`. The coding-agent can report a successful, persisted setting that deterministically fails later, while a disabled request with a stale level can be rejected unnecessarily.

**Fix:** Introduce one checked dispatch seam used by every public stream/collection path. Resolve a model's level only when thinking is enabled. Reuse the same map validation in harness set/model-switch/resume paths. Test `Provider::stream` directly and assert zero auth/HTTP calls.

### 3.2 MAJOR: `AccountIdMissing` loses its typed authentication semantics

**Files:** `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/provider.rs`, `crates/opi-agent/src/agent_loop.rs`, `crates/opi-coding-agent/src/runner.rs`, `crates/opi-coding-agent/src/rpc.rs`
**Lines:** Codex `151--158`; provider `317--320`, `372--377`; agent loop `508--524`; runner `690--703`, `725--735`; RPC `1000--1032`
**Spec ref:** exit-remediation design `1002--1022`

**Cause:** Codex correctly returns `ProviderError::AccountIdMissing` before HTTP, and `opi-ai` classifies it as authentication. `agent_loop` has no matching `AgentError` mapping, so the catch-all converts it to generic `AgentError::Provider`.

**Impact:** Text/JSON exits with `ProviderFailure` instead of `AuthFailure`; typed `/login openai-codex` remediation is lost; RPC emits no authentication event; interactive handling takes the generic failure path.

**Fix:** Preserve a typed agent error (or deliberately normalize to a typed reauthentication error) through diagnostics, interactive mode, runner events/exit codes, JSON, and RPC. Add provider-to-mode integration tests, not only provider-local tests.

### 3.3 MAJOR: Cancellation is not observed across the full Phase 14 operation

**Files:** `crates/opi-coding-agent/src/oauth.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_codex_responses.rs`
**Lines:** OAuth `219--271`; Anthropic auth `1305` vs cancellation `982`; Chat auth `1527` vs cancellation `1209`; Responses auth `568` vs cancellation `423`; Codex auth `339` vs cancellation `211`
**Spec refs:** `docs/opi-spec.md:1754--1758`; provider contract `crates/opi-ai/src/provider.rs:29--32`, `80--84`

**Cause:** PKCE cancellation is first created/polled only after loopback bind and `present_auth_url`; a pre-cancelled or blocked-presenter flow therefore ignores cancellation until the absolute deadline. Separately, provider stream tasks resolve lazy credentials and await HTTP response headers before their byte-loop selects `Request::cancel`.

The main agent path is partly protected because it drops the stream on cancellation and the receiver-drop branch aborts the task. A direct public `Provider`/collection caller that retains and polls the stream is not protected and can refresh credentials or initiate HTTP after cancellation.

**Impact:** Ctrl-C can be unresponsive during pre-code PKCE stages. Public provider consumers can wait indefinitely when no request timeout is set, and cancellation can fail to prevent credential/network work.

**Fix:** Create one cancellation future before PKCE bind and select it through every pre-code stage. In provider implementations, select cancellation and receiver closure around the complete auth-plus-send-plus-body operation, returning exactly one typed `Cancelled` terminal outcome. Add ready-before-start, pending-presenter, pending-resolver, and pending-header tests.

---

## 4. Minor Findings

### 4.1 Minor: Invalid PKCE callbacks receive a success page

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** `439--484`

**Cause:** The loopback handler writes and flushes HTTP 200 with “Login complete” before parsing the request or validating the CSRF state.

**Impact:** A malformed or state-mismatched callback correctly fails credential acquisition, but the browser has already told the user that login succeeded.

**Fix:** Parse and validate first. Return a fixed secret-free 400 response for invalid input and 200 only after valid state/code extraction.

### 4.2 Minor: Large manual input can block the child/parent pipe

**File:** `crates/opi-coding-agent/src/oauth.rs`
**Lines:** `1893--1915`
**Spec ref:** `docs/opi-spec.md:1759--1762`

**Cause:** The parent waits for the cooked-line child to exit before draining piped stdout. A line larger than the pipe capacity blocks the child on write while the parent blocks on exit.

**Impact:** Manual fallback stalls until the OAuth deadline/cancellation path kills and reaps the child. Normal short codes are unaffected.

**Fix:** Drain stdout concurrently with a strict maximum line length, then await/reap the child.

---

## 5. Test Quality Assessment

All fresh workspace gates pass, but several green tests normalize or omit the failing behavior:

- Responses/Codex fixtures add explicit `event:` fields and omit canonical data-only/lifecycle/reasoning/function-done sequences.
- Thinking “production” tests call `validate_request_capabilities` outside `Provider::stream`; body-builder tests explicitly accept silent omission of unsupported reasoning.
- The recorded 14.15 filter `embedded_model_pricing_` excludes the actual harness switch/resume/fork lifecycle test.
- The included pricing test switches before any usage; the excluded lifecycle test asserts the incorrect $18-to-$42 retroactive reprice.
- Account-id tests stop at the provider boundary and never exercise agent/text/JSON/RPC classification.
- OAuth tests do not inject extreme duration values or cancellation before bind/presentation.
- `thinking_integration.rs` checks enabled/budget but does not assert that the selected level reaches the request.
- Several Non-Goal guards are exact substring/source-shape checks. No actual Non-Goal violation was found, but these guards are weaker than behavioral or compiled API checks.

### Fresh verification

| Command | Result |
|---------|--------|
| `cargo test --workspace --all-targets` | PASS |
| `cargo test --workspace --doc` | PASS |
| `cargo fmt --check --all` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS |
| Focused opi-ai auth/wire/Responses/Codex/model tests | PASS |
| Focused coding-agent credential/OAuth/TUI/provider tests | 253 passed, 2 subprocess-only ignored |
| Exact mixed-model lifecycle test | PASS, while asserting the incorrect retroactive reprice |

---

## 6. Success Criteria and Non-Goals

| Criterion | Assessment | Independent evidence |
|-----------|------------|----------------------|
| SC1 credential storage/probes | MET | Native selector, marker-only probes, fail-closed resolver, locking, and no-plaintext checks are substantive. |
| SC2 OAuth product flows | NOT MET | Normal flows exist, but provider-controlled durations can panic and PKCE cancellation omits pre-code stages. |
| SC3 live auth/session interaction | NOT MET | Lazy auth exists; public cancellation and `AccountIdMissing` mode propagation are incomplete. |
| SC4 request/session affinity | MET | New/resume/fork session IDs reach requests and reviewed wire mappings. |
| SC5 capabilities/cache markers | PARTIAL | Cache/capability placement is strong; thinking-map enforcement is not part of every public dispatch path. |
| SC6 usage/metadata/cost | NOT MET | Strict usage subsets pass, but mixed-model and cumulative-tier cost summaries are incorrect. |
| SC7 refresh/api-map substrate | PARTIAL | Atomic refresh and routing pass; mapped/public dispatch does not guarantee thinking preflight. |
| SC8 docs/guards | PARTIAL | English/Chinese claims are aligned, but final acceptance fixtures/filters miss core normal-path failures. |

All eight binding Non-Goals remain respected:

1. no opi-managed plaintext credential file;
2. no automatic mid-stream re-login;
3. no per-call credential or provider-managed auth-header override;
4. no `onPayload`/`onResponse` core hooks;
5. no `maxRetries`/`maxRetryDelay` fields on `Request`;
6. no end-to-end `SecretString` provider-construction migration;
7. no OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex;
8. no session-schema/context-reconstruction redesign.

---

## 7. Invariant Verification

| Invariant | Code evidence | Test coverage / result |
|-----------|---------------|------------------------|
| Credentials are not stored in opi plaintext files | Native keyring backend plus non-secret lock/marker | Strong fake-backend/temp-root/redaction coverage; MET |
| Missing/backend/corrupt credential states remain distinct | Marker and protected-envelope resolver paths fail closed | Strong positive and negative tests; MET |
| Refresh is locked, double-checked, bounded, and preserves the prior credential on failure | `CredentialResolver` lock/re-read/timeout flow | Strong concurrency/failure tests; MET |
| OAuth invalid responses fail in-band, not by panic | OAuth duration arithmetic | No extreme-value tests; NOT MET |
| Cancellation is accepted throughout pre-code login and public request work | Cancellation starts after PKCE presentation and after stream auth/send | Existing tests begin too late; NOT MET |
| Provider streams yield one meaningful terminal outcome | Responses unknown normal events become Error and transport loops do not terminate at protocol completion | Fixtures are non-realistic; NOT MET |
| Unsupported thinking is rejected before network on every public wire | Standalone helper only | Tests manually compose helper; NOT MET |
| Reserved provider/auth headers cannot be overridden | Shared provider-header validation | Strong negative coverage; MET |
| Session affinity is stable across new/resume/fork | Agent/session coordinator propagation | Production-path tests; MET |
| Usage children never exceed parents | Provider mappers and `Usage::validate` | Absence/zero/equality/malformed fixtures; MET |
| Historical cost is invariant under later model selection | One mutable session price over cumulative usage | Test asserts repricing; NOT MET |
| Dynamic catalog replacement is deterministic and atomic | Collection gathers all snapshots before install | Rollback/order tests; MET |
| No automatic re-login after revocation | Typed `CredentialRevoked` ends the turn | Product-mode negative tests; MET |

---

## 8. Residuals and Recommendations

### Priority recommendations

1. Repair the Responses/Codex decoder against data-only, complete pi-0.80.6 event sequences before relying on those providers.
2. Eliminate both untrusted-input panic families and add adversarial mapper/OAuth fixtures.
3. Preserve cancellation and typed authentication semantics across every crate and run mode.
4. Replace session-wide repricing with per-turn/per-model cost accounting, including per-request tier selection and mixed-model resume/fork tests.
5. Centralize thinking validation in the public dispatch contract and reuse it in coding-agent model/thinking lifecycle commands.
6. Correct the two PKCE/manual-input edge cases and strengthen acceptance filters/fixtures so they exercise the actual product boundary.

No source, test, specification, or product documentation was changed by this audit. The only audit-owned write is this report. The pre-existing deletion of `docs/snapshots/phase14/remediation-plan.md` was preserved.
