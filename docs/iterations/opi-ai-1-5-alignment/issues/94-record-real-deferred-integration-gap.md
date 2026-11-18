# #94 Record the production deferred acceptance gap

The original Opi worktree at `ee18dd1323a089d536bdc7e293d0b4869777c344` and
frozen Pi 1.5.0 at `f9a26365568127461c47a07a37dcb42c140a3ac5` had no
in-repository route from a production caller to a real deferred backend.
The #143 slice now supplies the first provider-owned route and explicit
coding-agent caller. Local pending/ready session settlement and restart
recovery are covered; real job responses and the remaining production
acceptance rows remain blocked on the authorized live probe.
The SDK surfaces also allow custom backends supplied by hosts.
The audit covers the two source trees and the original-lock Pi build.
[#93 fixture comparison](93-assess-deferred-job-failure-classification.md)

## Provider registration and callers

| Surface | Provider registration | Caller |
| --- | --- | --- |
| Opi SDK and coding agent | `Models::new` and `from_registry` start with an empty deferred registry. `register_deferred_api` is its insertion path. #143 adds the OpenAI Responses Background adapter for `openai-responses:o3`; the provider factory registers it with the ordinary route's auth resolver. | The coding-agent harness exposes explicit submit, poll and cancel calls, persists each normalized poll outcome in the existing assistant-message session record, restores a still-pending handle on resume/fork, and blocks a new turn until explicit poll or cancel. A ready poll clears the durable pending boundary; an acknowledged cancel persists an `Aborted` terminal. Retryable poll failures and any failed cancel retain the handle for explicit retry, while terminal poll failures persist an `Error` terminal without resubmission. The agent loop returns a pending handle at the Deferred terminal and does not auto-continue. |
| Frozen Pi 1.5.0 | The compiled build constructs 42 built-in providers. Zero expose `fetchDeferred` or `cancelDeferred`. Faux exposes both and produces handles backed by a per-instance job map. | Durable implements checkpoint, poll, explicit cancellation and settlement. The issue triage host uses Durable with real Codex and Anthropic ordinary streaming. Coding-agent Models wrappers forward an adapter's available methods. |

In Opi, both the built-in route factory and the coding-agent harness
factory install ordinary routes through `register_route`. That path installs
the provider, auth resolver and compatibility data. The separate deferred
registry now receives the OpenAI Responses adapter from the same production
factory; `ScriptedDeferredBackend` under `test_support` remains the
deterministic fixture used by the external SDK and differential tests. The
remaining accesses to the registry look up, enumerate or remove entries.
Agent and eval handle references copy or validate data.
[Registry construction](../../../../crates/opi-ai/src/models.rs#L612) ·
[Deferred registration](../../../../crates/opi-ai/src/models.rs#L964) ·
[Built-in routes](../../../../crates/opi-ai/src/providers/collection.rs#L1019) ·
[Coding-agent assembly](../../../../crates/opi-coding-agent/src/provider_factory.rs#L166) ·
[Scripted backend](../../../../crates/opi-ai/src/test_support.rs#L860) ·
[External SDK test](../../../../crates/opi-ai/tests/external_sdk.rs#L1584) ·
[Reopening consumer](../../../../crates/opi-ai/tests/deferred_contract.rs#L817)

At the Pi pin, `packages/ai/src/providers/all.ts:135–189` constructs
and installs the built-ins. `packages/ai/src/models.ts:1164–1186` forwards
optional methods from a selected stream implementation.
`packages/ai/src/api/lazy.ts:93–118` exposes them only when their
capability flags are supplied. The in-tree flag use is in
`packages/ai/test/providers.test.ts:547`. Faux constructs the handle,
fetches from its map and marks cancellation at
`packages/ai/src/providers/faux.ts:548–573,591–665,709–719`.

The Pi coding agent's `packages/coding-agent/src/core/model-runtime.ts:763–796`
and `provider-composer.ts:651–657` contain the forwarding paths.
`packages/coding-agent/src/core/sdk.ts:412–427` wires ordinary agent
turns to `streamSimple`. The repository's issue triage host registers
`openaiCodexProvider` and `anthropicProvider`, opens SQLite and resumes
Durable. It submits inputs and reads their settlement. Its stream
settings and adapter registrations do not activate deferred generation.
See `scripts/issues/triage.ts:118–149,232–263`. The deferred poll and
cancel paths are in `packages/durable/src/harness/generation.ts:323–337,582–595`.
A host can supply its own Models at
`packages/durable/src/harness/types.ts:684–701`.

## Evidence needed for the error mapping

The preserved #64 and #59 cases dispatch to Faux and the Rust scripted
backend. Pi fulfills with an error message and Opi returns
`Err(CallFailure)` with the fixture's `Config`. #93 exercises the exact
diagnostics through the pinned Durable code. It records terminal
`model_error` for the current generation, zero retries and zero extra
cancels. This is application-policy evidence for those fixture inputs.
[#64 raw record](../evidence/phase5-deferred-unknown-job-id-differential.json) ·
[#59 raw record](../evidence/phase5-deferred-remote-cancel-then-fetch-differential.json) ·
[#93 receipts](../evidence/phase5-deferred-job-failure-consumer-and-replay.json)

| Acceptance item | Status | Missing evidence |
| --- | --- | --- |
| Real deferred route | In progress; live evidence blocked | The adapter and production registration exist and are covered by the wire-level submit/fetch/cancel test. Local ready-poll settlement and restart recovery are covered. An authorized OpenAI project must still produce a real response id and live pending/ready/cancel receipt. |
| Unknown and cancelled job responses | Blocked | Actual responses for a valid route with an unknown ID and after explicit cancellation of an existing job. Record auth and dispatch, wire status, provider diagnostic or state, and the returned assistant message and typed cause. |
| Production retry and settlement | In progress; live evidence blocked | The local caller policy is now explicit: no automatic retry/resubmission, transient poll/cancel failures retain the handle for caller retry, terminal poll failures and acknowledged cancellation settle durably once. Live provider responses must still verify the policy and receipt on an authorized OpenAI project. |

The first adapter's status mapping is fixed in #143: queued/in-progress stay
`Deferred`, completed maps to the normal assistant result, failed/cancelled/
incomplete and unknown statuses become typed provider failures, and HTTP
unknown/expired responses retain the provider-side status class. The SDK
retains the backend cause in `CallFailure`.
The scripted `Config` and the two raw result-channel differences stay in
the differential evidence. The original 24-case record still reports
`strict_alignment: false`. Current live service availability has its own
endpoint/model/auth evidence under the existing acceptance policy.
[Bound lifecycle record](../evidence/phase5-codex-websocket-idle-timeout-differential.json) ·
[Acceptance policy](../decisions/10-acceptance-policy.md)

## Verification

- Loaded `builtinProviders()` from `packages/ai/dist/providers/all.js`
  and `fauxProvider().provider` from `packages/ai/dist/providers/faux.js`
  in the original-lock snapshot. Counted function-valued
  `fetchDeferred` and `cancelDeferred` under the credential-isolated
  differential environment. The result is 42 built-ins, zero fetch,
  zero cancel, with Faux positive for both. Stdout SHA-256 is
  `58861fa2fa4a1136779bea8bfd2926f7e62b225002c06d400d7db46a2859e101`.
  Material checks before and after show zero drift.
- Checked the historical #64 receipt and both pre-repair and repaired
  #59 receipts against their recorded SHA-256 values. The #93 case
  records still equal the bound #89 cases. All eight paired driver
  receipts match their stdout and SHA-256.
- Documentation and diff checks pass. The bilingual transport guides
  already describe the explicit registration and fixture boundary.
- #143's first live probe is recorded in
  `../evidence/phase9-deferred-openai-probe-20261116.json`; it is blocked
  because `OPENAI_API_KEY` is unavailable, and no network request was sent.

Test impact `retain` for #64, #59 and their Durable and Rust controls.
Worktree handoff `committed=none-for-this-task` and `staged=none`.
Unstaged updates touch the carried-in iteration brief and contract index.
Untracked updates touch the carried-in #93 issue and add this issue.
