# Review deferred failure classification in Durable

The Rust result entry exposes a canonical assistant message and a typed
cause. The typed retryability flag differs from Pi's message policy. The
overflow predicate misses two Pi 1.5.0 rules, and the legacy in-band abort
terminal loses its `Aborted` stop reason. Each gap affects a Durable-style
caller's continuation.
[#76 result timing](76-return-deferred-outcome-before-source-release.md) ·
[Bound observations](../evidence/phase5-deferred-failure-consumer.json)

## Application behavior

Pi `v1.5.0` at `f9a26365568127461c47a07a37dcb42c140a3ac5` awaits
`fetchDeferred` in `packages/durable/src/harness/generation.ts:323–338`.
`classify` first checks the task signal, then uses the returned message.
An `error` can start overflow compaction if there is an eligible cut and
the run has not compacted already. Ordinary retries exclude overflow and
use `isRetryableAssistantError`, the configured attempt budget and backoff.
The retry checkpoint from a returned error has no deferred handle and the
next attempt starts at `prepare`. A provider `aborted` terminal with a live
task signal reaches the `model_error` branch. The task abort handler
separately cancels a retained deferred handle. See `generation.ts:302–320,
390–418,574–596,758–851` and `packages/ai/src/utils/retry.ts:7–104,270–275`.

The probe runs the pinned Harness with `MemoryStorage` and Faux, exercising
the public application paths on the original-lock installation. The first
four cases return provider diagnostics, the fifth returns a provider abort,
and the sixth aborts the conversation after the fetch has reached a gate.

| Fetch outcome | Actual Durable continuation |
| --- | --- |
| `503 Service Unavailable` | Stores the error entry, then submits and fetches a new job. The second answer succeeds. |
| `429 GoUsageLimitError: Monthly usage limit reached` | Stores the error entry and settles `model_error` after one job. |
| `Context Window exceeded for given call.` | With eligible history, stores the error, completes one `overflow` compaction and submits a new job that answers. |
| Copilot-style `API error (413)` | Completes the same overflow compaction path, including a new job. |
| Provider `aborted` with the same 503 text | Stores an `aborted` entry and settles `model_error`. One fetch and zero remote cancels. |
| Application abort during a held fetch | The fetch signal is aborted. The application settles `aborted`, issues one remote cancel for the same handle and records no provider assistant entry in this case. |

The host-restart path has a different checkpoint. `interrupted` retains the
handle if it can retry the interrupted poll and cancels it when the run ends.
The two pinned restart tests pass and confirm that a resumed fetch uses the
same handle and that an exhausted restart path explicitly cancels it.
`packages/durable/src/harness/generation.ts:501–543` and
`packages/durable/test/harness-restart-failure.test.ts:523–572`.

## Rust mapping and reproduced gaps

The `fetch_deferred_ready` test uses the same fixture diagnostics through a
registered backend. It asserts one auth resolution and dispatch, retains
the message's partial and route, and reads both parts of `CallFailure`.
Its local cancellation control verifies `Cancelled` with `Aborted` and a
released source. The controlled backend rejects any implicit remote cancel.
[Failure inputs](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1384) ·
[Local token control](../../../../crates/opi-ai/tests/execution_lifecycle.rs#L1290)

- **Retry.** The in-band 503 retains its diagnostic and gets a generic
  `StreamError`, whose typed retryability flag is false. The typed
  `RateLimited` control with the quota diagnostic retains that text and
  has a true retryability flag. Pi takes the opposite decision on each
  message. `ProviderError::is_retryable` remains useful for typed transport
  decisions, such as the tested `Timeout`. A faithful application retry
  policy needs the canonical message and its available diagnostic as well
  as caller-owned retry state. The case with quota text retained is a
  controlled adapter input. For an adapter that omits or redacts that text,
  equivalent classification requires an available structured fact.
  [Typed predicate](../../../../crates/opi-ai/src/errors.rs#L374)
- **Compaction.** The two diagnostic strings survive `CallFailure` intact.
  Rust recognizes the `prompt is too long` control, but its public
  `is_context_overflow` misses the two new forms. Its table was ported from
  Pi `v1.3.0` at `71023aec1546d6d677a65669c5665c0927ce0dd0`. Pi added
  the LiteLLM pattern in `d3398f826ae2654b52a7988830d533a354754c13` and
  the anchored HTTP 413 pattern in `73f8de80bc87021375f8b525292ac07492253002`,
  both in `packages/ai/src/utils/overflow.ts:39–69`. This is source drift.
  [Rust predicate](../../../../crates/opi-ai/src/types/message.rs#L688)
- **Abort.** Typed `ProviderError::Cancelled` preserves `Aborted`, including
  with an uncancelled request token when the provider supplies the cause.
  The legacy in-band `Error` has `reason == Aborted` and message stop
  `Aborted` in the fixture. The normalizer rewrites it to `Error` and
  `StreamError`. That produces the same stop and text that caused the 503
  retry in Pi. This path dates to Opi's `b2ff2f398b8d2416f0284ac7a77731338cf0a436`
  normalization. A consumer uses its own token and task state to distinguish
  an explicit abort from an `Aborted` message, and can explicitly invoke
  `cancel_deferred` on the abandoned job.
  [Legacy normalization](../../../../crates/opi-ai/src/stream.rs#L229) ·
  [Typed normalization](../../../../crates/opi-ai/src/stream.rs#L204)

## Verification and follow-up

- Run the [Pi probe](../tools/observe_deferred_failure_consumer.mjs) with
  `node --conditions=source --experimental-strip-types` and `--snapshot`
  pointing at the bound snapshot. The six observations match across three
  credential-isolated runs. `node --check` passes and missing arguments
  exit 2. Thirteen pinned source and lock files match Git bytes.
- `cargo test -p opi-ai --test execution_lifecycle -- --nocapture
  --test-threads=1` passes all 40 tests and emits the nine Rust observations.
  Affected-target Clippy with `-D warnings`, Cargo format, the documentation
  checker and `git diff --check` pass.
- The existing #76 materials and 24 shared comparisons revalidate. The
  receipt binds the probe, the Rust test source and executable, and the
  original-lock manifest. It records application observations against a
  controlled provider and SDK contract inputs. Real-provider diagnostics
  and a Rust Durable migration remain open.

Align the two overflow patterns first, with a negative control for a
body that merely mentions 413 and for the throttling exclusion. Then
address legacy abort normalization and verify the application-level retry
predicate against the pinned message policy.

Test impact `add` for the public Rust failure-input test and the six-case
Pi application probe. `retain` for the existing cancellation, host-restart
and paired lifecycle controls. Worktree handoff `committed=none-for-this-task`
and `staged=none`. The Rust test target and owning docs are carried in and
touched. The issue, probe and receipt are new.
