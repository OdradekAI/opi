# #143 Connect one real Deferred route to a production caller

Status: open
Owner: user
Blocked by: [acceptance policy](../decisions/10-acceptance-policy.md), [lifecycle decision](../decisions/11-telemetry-lifecycle.md)
Resolves: [#94](94-record-real-deferred-integration-gap.md)

## Goal

Close the production integration gap recorded by #94 for one concrete provider
route. The slice must submit a real asynchronous job, return a serializable
`DeferredHandle`, register fetch and remote-cancel operations on the same
production `Models` collection, and let one production caller resume, settle,
retry, or cancel that job.

This is a bounded capability slice. It does not claim that every built-in
provider supports Deferred, and it does not replace the existing Faux,
`ScriptedDeferredBackend`, or frozen differential fixtures. Those controls stay
as deterministic regression coverage.

## Current facts and provider gate

The SDK already owns the generic lifecycle:

- `crates/opi-ai/src/api/contract.rs` defines `DeferredApi` and typed
  fetch/cancel calls; `crates/opi-ai/src/types/deferred.rs` owns the
  serializable `DeferredHandle` and route error values.
- `crates/opi-ai/src/models.rs` exposes
  `Models::register_deferred_api`, `validate_deferred_handle`,
  `stream_deferred`, `fetch_deferred_ready`, `fetch_deferred`, and
  `cancel_deferred`. Registration is separate from ordinary provider routes;
  `Models::new` and `Models::from_registry` start with an empty deferred map.
- `crates/opi-ai/src/test_support.rs` provides only the scripted backend used
  by current tests and package proofs.
- `crates/opi-coding-agent/src/provider_factory.rs` and
  `crates/opi-coding-agent/src/harness.rs` assemble ordinary dispatch routes
  and auth resolvers. The first slice now registers provider-owned Deferred
  adapters through `build_harness_collection`; the harness owns explicit
  submit/poll/cancel calls and persists normalized poll outcomes through the
  active session coordinator.
- `crates/opi-agent/src/agent_loop.rs` turns a `Done` event into an assistant
  message and continues normal tool/turn handling. It has no production branch
  that owns a deferred handle, schedules an explicit fetch, or applies a
  retry/settlement policy. `crates/opi-agent/src/event.rs` already preserves the
  handle across the public redaction boundary.

Before implementation, freeze one provider/API/model whose real service has
all three operations below and a credential-isolated test account or equivalent
non-production endpoint:

1. submit an asynchronous job and return a stable provider job id;
2. fetch the job once, distinguishing pending, ready, unknown, expired and
   provider-failed states;
3. cancel an existing job and report the provider's acknowledgement or failure.

The provider decision must record endpoint, API revision, model, auth source,
wire status/error shapes, rate/retention limits, and whether the call is live,
sandboxed, or otherwise non-production. If no provider satisfies this gate,
the issue remains blocked and #94 stays open; scripted calls cannot be promoted
to real integration evidence.

## Provider decision and first probe

The first candidate is **OpenAI / Responses API / `o3`**:

- endpoint class: `https://api.openai.com/v1`;
- submit: `POST /responses` with `background: true`, `store: true`, and
  `model: "o3"`;
- fetch: `GET /responses/{response_id}`;
- cancel: `POST /responses/{response_id}/cancel`;
- handle id: the provider `response.id`; the persisted handle carries only
  `provider`, `model_id`, `api`, `id`, and optional expiry/poll advice;
- source evidence: [OpenAI Background mode](https://developers.openai.com/api/docs/guides/background)
  and the [Responses create](https://developers.openai.com/api/reference/resources/responses/methods/create),
  [retrieve](https://developers.openai.com/api/reference/resources/responses/methods/retrieve),
  and [cancel](https://developers.openai.com/api/reference/resources/responses/methods/cancel)
  references.

The repeatable live probe is
[`scripts/opi-deferred-provider-probe.ps1`](../../../../scripts/opi-deferred-provider-probe.ps1).
It records only redacted status/error shapes and exits `2` when the selected
credential is unavailable. The first run on 2026-11-16 produced
[`phase9-deferred-openai-probe-20261116.json`](../evidence/phase9-deferred-openai-probe-20261116.json):
the provider contract is selected, but live submit/fetch/cancel evidence is
**blocked** because `OPENAI_API_KEY` is not set. The local `OPI_BIGMODEL_API_KEY`
is intentionally not substituted: the configured BigModel endpoint is not
evidence for the OpenAI Background contract. No network request was sent by
the blocked run.

The implementation can proceed behind this gate with wire-level tests and
production registration, but #143 cannot be closed until the same probe runs
against an authorized, credential-isolated OpenAI project and records a real
response id plus pending/ready/cancel outcomes.

## Implementation started (2026-11-16)

The first production slice is now wired behind the selected route:

- `crates/opi-ai/src/providers/openai_responses.rs` and `crates/opi-ai/src/api/openai_responses.rs` adds
  `OpenAiResponsesDeferredApi` for the `openai-responses:o3` route. It sends
  `POST /responses` with `background: true`, `store: true`, fetches with
  `GET /responses/{id}`, cancels with `POST /responses/{id}/cancel`, and maps
  pending/ready/provider-failure outcomes into the existing Deferred and
  assistant-message contracts.
- `crates/opi-ai/src/provider.rs` and
  `crates/opi-coding-agent/src/provider_factory.rs` expose and register a
  provider-owned Deferred adapter with the same auth resolver and `Models`
  collection as the ordinary route. Lookup and registration do not submit a
  live job.
- `crates/opi-ai/src/models.rs` now has an explicit submit entry point that
  validates the registered provider/model identity before authentication and
  wraps submission in the normal `pi.ai.request` telemetry carrier.
- `crates/opi-coding-agent/src/harness.rs` exposes explicit
  `submit_deferred`, `poll_deferred`, and `cancel_deferred` calls, retaining a
  pending handle in memory and never polling or cancelling from drop paths.
  The existing assistant-message session record now carries that handle
  through resume/fork reconstruction, and the harness restores it before
  allowing another turn. Each explicit poll appends its normalized assistant
  outcome to the live agent/session projection; pending polls update the
  durable handle, while a ready poll clears it so reopening the session does
  not resurrect the stale job.
  A successful cancel now appends an `Aborted` assistant terminal and clears
  the durable pending boundary. A failed cancel retains the handle for an
  explicit retry. Poll failures use the existing typed retryability taxonomy:
  transient network/timeout/rate-limit failures retain the handle for a
  caller-issued retry, while terminal provider failures append an `Error`
  assistant terminal and settle without resubmission.
- `crates/opi-agent/src/agent_loop.rs` treats a successful `Deferred` terminal
  as a pending-state boundary: it requires a handle, returns it in the
  run-result, emits one terminal turn, and does not enter tool dispatch,
  follow-up queue polling, or hook-driven continuation. A caller must poll or
  cancel explicitly before starting another turn.
- `crates/opi-ai/tests/openai_responses_lifecycle.rs` contains a wire-level
  submit/fetch/cancel proof using the real route adapter shape and a redacted
  static credential. This proves registration and mapping only; it is not live
  provider evidence.

The live gate is still blocked by the missing `OPENAI_API_KEY`, as recorded in
the receipt above. The current implementation therefore remains an open
milestone: it is ready for an authorized probe, and local session/loop proofs
now cover pending ownership and restart restoration. It does not claim that
the provider account, retention window, rate limits, retry policy, or full
production caller acceptance have been demonstrated.

## Verification update (2026-11-17)

The current implementation was rechecked against the shared persistent Cargo
target and the selected route's production boundaries:

- `openai_responses_lifecycle::background_deferred_registration_submits_fetches_and_cancels`: 1 passed;
- coding-agent `session_runtime` Deferred cases: 7 passed;
- `provider_factory::build_harness_collection_registers_provider_owned_deferred_route`: 1 passed;
- agent-loop `deferred_terminal_stops_the_loop_and_retains_handle`: 1 passed.

The live probe was rerun with the exact selected route (`openai` /
`openai-responses` / `o3`) and produced
[`phase9-deferred-openai-probe-20261117.json`](../evidence/phase9-deferred-openai-probe-20261117.json).
`OPENAI_API_KEY` was unavailable, so submission, fetch and cancellation are
all explicitly `blocked`; no provider request was sent. The older 2026-11-16
receipt remains unchanged. This is a credential gate, not a wire or local
lifecycle failure.

## Entry points and expected changes

### 1. Provider adapter in `opi-ai`

Add one provider-specific adapter behind the existing `DeferredApi` seam. Keep
generic lifecycle logic in `Models` unless the provider probe exposes a missing
contract. The adapter must:

- own a fixed `ApiId`, provider id, and model id;
- use the existing `AuthResolver`/`ResolvedAuth` path and never place secrets in
  a handle or persisted event;
- map submit responses to a normal chat terminal with
  `StopReason::Deferred` plus a `DeferredHandle`;
- map one fetch to the existing `AssistantStreamEvent` contract, including a
  pending `Deferred` terminal and a ready assistant result;
- map unknown, expired, cancelled, transport, authentication and provider
  failures to typed `ProviderError`/`CallFailure` while preserving the original
  provider diagnostic in the bounded evidence record;
- make remote cancellation best effort and distinguish cancellation requested,
  acknowledged, already terminal, unknown and failed states;
- carry the supplied telemetry context so `pi.ai.poll` and cancellation
  context rules remain testable.

Likely production files are a new provider-specific module under
`crates/opi-ai/src/` plus the provider's existing protocol/auth module. Touch
`crates/opi-ai/src/models.rs` or `api/contract.rs` only if the real wire contract
demonstrates a generic omission; do not fork the public Deferred lifecycle for
one provider.

### 2. Production registration and assembly

Thread the adapter and its resolver through the production construction path:

- expose an optional provider-owned Deferred adapter and register it alongside
  the ordinary route in `build_harness_collection`, preserving one `Models`
  owner, one credential store and one auth resolver;
- keep the registration derivable from the provider route rather than adding a
  parallel `ProviderBundle` credential field;
- thread the registration through `CodingHarnessBuilder` and the shared
  construction used by interactive, non-interactive and RPC startup paths in
  `crates/opi-coding-agent/src/main.rs`, `runner.rs`, `rpc.rs`, and
  `harness.rs`;
- keep `--list-models` and lookup-only construction free of live job submission;
  a Deferred capability may be advertised only when its registration and auth
  policy are actually available;
- add startup diagnostics for unavailable credentials/configuration without
  silently constructing a fake Deferred backend.

### 3. One production caller and durable ownership

Choose one caller as the acceptance owner. The smallest compatible choice is
the coding-agent runtime, with an explicit resume/cancel operation rather than
implicit polling in every turn:

- in `crates/opi-agent/src/agent_loop.rs`, detect a successful terminal whose
  `stop_reason` is `Deferred`; require and retain its handle instead of treating
  it as an ordinary completed turn;
- expose a product-level pending state and explicit `poll`/`cancel` operation
  from `crates/opi-coding-agent/src/harness.rs` (and the corresponding runner or
  RPC surface used by the selected caller);
- persist the handle and provider/model/API identity through the existing
  session/event format, then restore it without credentials in the handle;
- on poll, use `fetch_deferred_ready` for the caller-visible outcome and
  `fetch_deferred` when the caller explicitly requests source/poll settlement;
  preserve pending handles for another poll;
- on ready, feed the final assistant result through the same session projection
  and task-settlement boundary as an ordinary provider result; the local
  ready-poll path is covered, while live provider acceptance remains open;
- on unknown/expired/cancelled/provider failure, apply one documented retry or
  resubmission policy, record whether a new job was submitted, and settle the
  task exactly once. The current coding-agent policy performs no automatic
  retry or resubmission: retryable poll failures return immediately with the
  same durable handle for an explicit caller retry; terminal failures settle
  once as an error assistant;
- on explicit cancel, call `Models::cancel_deferred`, record the remote result,
  persist an acknowledged cancellation as an `Aborted` assistant terminal,
  and leave a failed cancellation pending for an explicit retry.

Do not make stream Drop, consumer Drop, or an unrelated context clone perform a
remote cancel. The existing lifecycle decision requires explicit cancellation
and keeps execution ownership separate from consumption.

## Evidence to add

Create a new machine-readable receipt and retain the raw logs, redacted wire
payloads, environment identity and hashes. The receipt must bind the exact
provider revision, model, endpoint class, auth source, Cargo/features,
workspace/package artifact, test source and runner.

Required evidence rows:

1. **Submission** — one real request creates one job, returns one handle, and
   does not expose credentials; record request count, provider job id mapping,
   `Deferred` terminal, session persistence and `pi.ai.request` settlement.
2. **Pending fetch** — at least one pending response preserves the same handle,
   increments exactly one poll, and leaves the task resumable without an
   implicit second poll.
3. **Ready fetch** — a later poll returns the final assistant result with
   provider/model/usage/stop fields, exactly one task settlement, source EOF and
   `pi.ai.poll` completion.
4. **Unknown/expired job** — use a valid route with a deliberately invalid or
   expired id; record auth, dispatch, wire status, redacted diagnostic, typed
   cause, assistant error projection, retry decision and final settlement.
5. **Remote cancellation** — cancel an existing job and separately attempt a
   cancel after terminal completion; record remote acknowledgement/failure,
   local `Aborted`/pending state, subsequent fetch behavior and any additional
   cancel count.
6. **Provider failure** — inject or capture a real provider-side failure after
   authentication; prove that the caller does not report success, does not
   duplicate settlement, and applies the documented retry/resubmission rule.
7. **Restart/resume** — persist a pending handle, restart the selected caller,
   restore it, poll once, and reach the same final outcome without replaying the
   original submission.
8. **Negative controls** — keep the existing Faux/scripted and missing-route,
   identity-mismatch, expiry and telemetry tests green; prove the real adapter
   is the only path used by the real receipt.

The differential record must separate provider facts from application policy:
Pi's message-shaped failure, Rust's typed `CallFailure`, and any approved
resource/timing difference remain separately reported. A live service result
must be dated and scoped to the exact endpoint/model/auth candidate.

## Acceptance

This issue is complete only when all applicable rows below are `pass` or an
explicitly approved difference with a named decision:

- **Route:** one real provider route submits, fetches and cancels through the
  public `opi-ai` registration path; no `ScriptedDeferredBackend`, Faux or
  source patch participates in the real receipt.
- **Identity:** every handle round-trips through the existing serialization,
  validates provider/model/API identity and expiry, and contains no secret.
- **Assembly:** interactive, non-interactive and the selected RPC/runner caller
  use the same registered adapter and credential store; lookup-only paths do no
  live submission.
- **Caller:** a real production caller can observe pending, resume to ready,
  explicitly cancel, handle unknown/expired/provider failure, and settle once.
- **Retry policy:** retryable poll failures retain the existing handle for an
  explicit caller retry; terminal failures and acknowledged cancellation are
  durable local terminals; no automatic retry or resubmission duplicates a
  remote submission or task settlement.
- **Persistence:** pending handles survive the selected session restart and
  restore without replaying submission or leaking credentials.
- **Telemetry:** request and poll spans/metrics follow the approved lifecycle;
  pending/deferred polls do not add token/cost samples, and observer failures
  remain distinguishable from provider business outcomes.
- **Security/resources:** secrets are redacted, cancel and source cleanup are
  bounded, and no task or network operation remains owned only by a dropped
  consumer.
- **Tests:** focused adapter, caller, persistence, failure and cancellation
  regressions pass, as do the existing deferred contract, lifecycle,
  telemetry, package and docs checks affected by the change.
- **Evidence:** the receipt is self-consistent, hashes current inputs, records
  the exact command/environment, and updates the contract index rows that were
  previously blocked by #94.

The issue must not be marked complete if the provider-selection gate is open,
the route is only scripted, real job responses are unavailable, or the caller
has no recorded retry/settlement decision. If no suitable provider or test
account can be authorized, record the blocker and retain #94 as an explicit
milestone gap instead of weakening these criteria.

## Non-goals

- enabling Deferred for all 42 built-in providers;
- changing the generic `Models` lifecycle or approved Pi/Rust differences;
- replacing deterministic Faux/scripted differential coverage;
- adding implicit polling or remote cancellation to ordinary stream Drop;
- claiming live-service availability beyond the dated provider receipt;
- publishing crates or making paid provider calls as part of issue preparation.

## Verification and handoff

Before claiming the issue, run the affected Rust check union, the external
package consumer for the selected feature profile, `python scripts/opi-doc-check.py`,
`cargo fmt --all -- --check`, and `git diff --check`. Re-run the existing
deferred differential controls after any lifecycle change. Update #94 with the
receipt and disposition, add the receipt/issue to the spec history, and report
the remaining macOS, budget and full-six-gate status separately.
