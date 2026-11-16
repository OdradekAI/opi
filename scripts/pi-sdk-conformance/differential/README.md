# Controlled lifecycle differential

The TypeScript oracle loads the Pi candidate bound to the material record.
The replay uses Pi's compiled public `./models` entry and Rust's public
`Models` with controlled providers. Named barriers expose provider dispatch,
source EOF, Rust source Drop and result timing for chat. The deferred cases
sample dispatch arguments, public outcomes and recorder state for fetch and cancel.

## Checks

Run on the native Windows toolchain used by this candidate:

```powershell
python -B docs/iterations/opi-ai-1-5-alignment/tools/test_acceptance_materials.py -v
node --experimental-import-meta-resolve --test docs/iterations/opi-ai-1-5-alignment/tools/test_inspect_acceptance_materials.mjs
python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p "test_*.py" -v
```

The driver test builds once through the shared Cargo-cache wrapper, then runs
the compiler-reported executable with a separate watchdog. Provider credentials
and proxy variables are not inherited by either runtime probe. Cargo builds
retain their own normal environment and shared incremental cache.

## Record a new candidate

Use new output paths; both writers reject overwriting historical evidence.
The freezer is offline and verifies the saved raw catalog against its pin.
It inventories dist/source data, resolves nested dependencies from the actual
AI importer and distinguishes internal validity from pinned identity. Use
`--snapshot` to bind an isolated upstream candidate. The default upstream tree
is `.pi/pi-1.5.0` under `--repo`, which always identifies the Rust workspace.

```powershell
python -B docs/iterations/opi-ai-1-5-alignment/tools/freeze_acceptance_materials.py `
  --repo D:/Luiz/Odradek/opi `
  --snapshot C:/Temp/opi-ai-frozen-lock-20261111 `
  --output C:/Temp/opi-materials-next.json

python -B scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py `
  --repo D:/Luiz/Odradek/opi `
  --materials C:/Temp/opi-materials-next.json `
  --output C:/Temp/opi-lifecycle-next.json
```

Use a fresh schema-v3 materials manifest. It records the absolute upstream path
and hashes the files and trees at that location. The runner takes the upstream
path from `--materials` and checks the manifest bytes and material identities
before and after replay. Earlier manifests remain records of their original
inputs and tool versions. Every TS case has three recorded repetitions.
Inconsistent observations fail. Build identity, exact commands, stdout/stderr,
raw traces and comparisons are retained.

## Interpretation

- `FAIL` / exit 1: actual mismatch or failed probe. Known source-tail failures
  remain failures even when other material gates are blocked.
- `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit 2: observations were recorded but
  required material provenance is unresolved.
- `DIFFERENCES_REQUIRE_DECISION` / exit 0: the replay detected differences.
  The decision records identify accepted differences and their scope.
  [#43 setup decisions](../../../docs/iterations/opi-ai-1-5-alignment/issues/43-match-public-models-completion.md#accepted-decisions) ·
  [#44 result timing decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/44-observe-public-result-before-source-eof.md#accepted-decision) ·
  [#45 terminal timing decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/45-observe-public-terminal-and-span-before-source-eof.md#accepted-decision) ·
  [#47 result attribute write decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/47-finish-request-span-after-result-attribute-write-failure.md#decision) ·
  [#48 first event write decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/48-observe-first-chunk-event-write-failure.md#accepted-decision) ·
  [#50 histogram factory failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/50-create-ai-instruments-before-starting-span.md#accepted-decision) ·
  [#51 request span start failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/51-observe-request-span-start-failure.md#accepted-decision) ·
  [#52 first chunk duration recording failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/52-observe-first-chunk-duration-record-failure.md#accepted-decision) ·
  [#54 request duration recording failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/54-finish-request-span-after-duration-record-failure.md#decision)
- `SHARED_OBSERVATIONS_PASS` / exit 0: the recorded checks passed in every
  case and the replay reported zero differences.
- Stale/changed materials stop replay or invalidate its result.

## Retained partials

The [partial replay](replay_partial_aliases.py) compares gated OpenAI Chat and
Faux events against a sealed `partial_aliases` Rust test executable. Build the
test with `test -p opi-ai --test partial_aliases --no-run --locked --offline`
through the shared Cargo-cache wrapper. Preserve the executable reported by
Cargo and its SHA-256. Supply the #102 materials, local Pi Git repository,
executable, SHA-256 and a new evidence path to the replay. The program
verifies the original lock and compiled Pi tree, checks each selected source
against the pinned Pi Git revision, and records three runs per side.
[Case and observations](../../../docs/iterations/opi-ai-1-5-alignment/issues/103-compare-stream-partial-aliases.md)

The #103 receipt records the pre-#104 Chat order and uses its archived test
source and executable. The live Rust alias test follows the repaired order.

## OpenAI Chat interleaving

The [interleaving replay](replay_chat_interleaving.py) checks one fixture with
ordinary and whitespace-only text after a tool delta. Supply `--before-materials`
and `--after-materials`, their sealed Rust `--before-build` and `--after-build`
receipts, `--pi-repo`, and a fresh `--output`. Each Rust receipt identifies the
cache-wrapper command and Cargo-reported executable, the executable's SHA-256,
and the bound mapper, test and fixture hashes. The Pi snapshot and its original
lock are shared. The report retains public events, both Chat HTTP request
bodies, the Anthropic assistant payload and the frozen Durable projection.
[#104 case and receipts](../../../docs/iterations/opi-ai-1-5-alignment/issues/104-align-openai-chat-interleaved-text.md)

## OpenAI Chat output limits

The [output-limit replay](replay_chat_output_limits.py) runs the same model,
messages, compatibility settings and loopback replies through native, simple
and injected-auth calls. Build `openai_chat_output_limits` through the shared
Cargo-cache wrapper with `test -p opi-ai --test openai_chat_output_limits
--no-run --locked --offline`. Supply the #104 `--materials`, the recorded
`--rust-build`, the local `--pi-repo`, and a new `--output`. The replay checks
both Chat limit field names, all 42 complete JSON bodies, resolver use, and the
result of each call. See the issue for the build receipt fields and source
identities. [#105 case and receipts](../../../docs/iterations/opi-ai-1-5-alignment/issues/105-compare-chat-output-limits.md)

## Lifecycle cases

`models-no-consumption` and `models-stop-consumption` compare the provider
dispatch checkpoint at public call return and at the source gate before any
consumer poll. In the second case Pi returns the outer iterator after
`start`, and Rust stops polling after `Start` while retaining the call.
Each checks one dispatch, a source terminal after gate release, source EOF
and the public result after EOF. Separate Rust example tests drop the
consumer and observe source completion.

`models-result-before-source-eof` starts observing the public result while
the source terminal is held. After the source emits `Done`, a second gate
holds source EOF. The Pi driver checks the promise state at a named event-loop
checkpoint and the Rust driver polls its pinned result future. Both release
EOF and collect the same result operation. The trace records the state at
the gate and checks the final stop reason and request identity.

`models-terminal-and-span-before-source-eof` reads public `Start` and
observes one next operation across both gates. Each driver samples an
in-memory request span while the source terminal is held and while source
EOF is held. After release, it checks actual source EOF, public `Done`,
outer EOF and the matching result. Pi awaits the delegated request span's
promise before reading the settled span. Rust uses `Recorder::spans()`
after source Drop and public EOF. The held-EOF checkpoint records terminal
readiness and span settlement.

`models-result-attribute-write-failure` injects one failed `pi.ai.request`
attribute write carrying `stop` and `usage.input`. The actual recorder
backs each test adapter, and the write hook samples source EOF at the
fault. The drivers check public `Done`, outer EOF and the matching result.
Pi records host `unhandledRejection` events and span state through a
post-fault event-loop checkpoint. Rust samples the span after execution
and records finish calls, wrapper Drop and tracing diagnostics. With this
fault, Rust reports one write failure and ends the span through the
in-memory backend. A separate Rust test also fails the finish attempt
and checks both warnings and the successful model result.
[#46 original fault trace](../../../docs/iterations/opi-ai-1-5-alignment/issues/46-observe-public-result-attribute-write-failure.md) ·
[#47 span completion](../../../docs/iterations/opi-ai-1-5-alignment/issues/47-finish-request-span-after-result-attribute-write-failure.md)

`models-first-chunk-event-write-failure` injects one failed
`pi.ai.first_chunk` event write while observing public `Start`. With the
provider terminal held, both drivers sample the next public item and the
request span. The Pi trace records iterator `return()` completion, generator
`finally`, natural EOF and source `result()` calls. The Rust trace records
natural EOF, source Drop, span completion and the tracing warning. The
comparison reports the difference in business outcome as well as source
release, telemetry and host diagnostics.
The accepted difference for this single write failure preserves the provider
outcome in Rust and reports the observer failure to the host.
[#48 first event write decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/48-observe-first-chunk-event-write-failure.md#accepted-decision)

`models-request-duration-histogram-creation-failure` fails creation of
`pi.ai.request.duration` once on a fresh recorder. The traces sample
dispatch and request span state at the factory fault, call boundary and
final checkpoint. Pi throws the injected error synchronously and records
zero dispatches and no request span. Rust reports a startup WARN, returns a
handle and runs the provider through source EOF, Drop and a successful
result. The initial Rust trace recorded an open span and zero finish calls.
After moving instrument creation ahead of span start, both drivers record
zero request spans at every checkpoint. The accepted difference covers
public call delivery, subsequent provider execution and the host
diagnostic for this single factory fault.
[#49 startup failure observation](../../../docs/iterations/opi-ai-1-5-alignment/issues/49-observe-request-duration-histogram-creation-failure.md) ·
[#50 startup order and decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/50-create-ai-instruments-before-starting-span.md#accepted-decision)

`models-request-span-start-failure` fails one `pi.ai.request` start after
four successful instrument creations. Both calls return a handle, dispatch
once and record zero request spans. Pi closes the controlled source iterator
and delivers an error from accessing `span.addEvent`. The host records the
injected start rejection and a separate `setAttributes` TypeError. Rust
reports one startup WARN and reaches natural source EOF and Drop, public
`Done` and a matching successful result. The accepted difference covers
the public business result, source exit and host diagnostics for this single
pre-dispatch span start fault.
[#51 span start failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/51-observe-request-span-start-failure.md#accepted-decision)

`models-first-chunk-duration-record-failure` emits `Done` as the first
source event and fails one `pi.ai.first_chunk.duration` record after the
span's first chunk event. Both public calls deliver `Done`, outer EOF and
the matching successful result. Pi completes source iterator `return()`
and settles the request span with `stop: error`. Rust records a WARN,
finishes the span with no `stop` and keeps the first public item pending
until natural source EOF and Drop. The accepted difference covers terminal
timing, source exit, span status and attributes, duration measurements and
host diagnostics for this single typed record error on a terminal first
event.
[#52 recording failure decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/52-observe-first-chunk-duration-record-failure.md#accepted-decision)

`models-request-duration-record-failure` injects one failed
`pi.ai.request.duration` record after the real source EOF and final span
attributes. The traces check `stop` and `usage.input` at the fault and
retain source cleanup, recorded metrics, span state and host diagnostics.
Both sides deliver the successful `Done` and matching result. The #53
baseline records both request spans open. The #54 replay records the Rust
span settled with one backend finish, the original stop and the original
WARN. Pi remains open at the bounded checkpoint. The accepted difference
covers span settlement and host diagnostics for this single typed fault.
[#53 recording failure baseline](../../../docs/iterations/opi-ai-1-5-alignment/issues/53-observe-request-duration-record-failure.md) ·
[#54 completion repair](../../../docs/iterations/opi-ai-1-5-alignment/issues/54-finish-request-span-after-duration-record-failure.md#decision)

`models-missing-provider` requests the same absent route on both sides.
`models-auth-setup-failure` registers a provider on each side and releases a
pre-dispatch auth resolver to the same controlled failure. The Pi probe passes
an in-memory API key to select the resolver before a store await. Each setup
case checks one error, EOF, and a final result carrying the observed failure.
Both failure messages must carry the requested provider and model. The traces
retain resolver entry timing, event and result channels, and diagnostics.

`models-deferred-wait0-pending-then-ready` calls the public fetch helper
twice with one handle and an explicit zero wait. It checks one dispatch
per call, the pending handle and ready message, and one `pi.ai.poll`
child of the caller span per call. Both sides record two duration samples per
poll and add token and cost counters only for the ready result. The
traces retain the span state and measurement names at result return and
the settled records at the following checkpoint. Pi completes its
telemetry after result return in this case. Rust completes it before
return. #96 retains the timing difference for the finite zero-wait case.
Result-only callers can use `fetch_deferred_ready` on Tokio.
[#55 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/55-observe-deferred-wait0-pending-then-ready.md) ·
[#96 decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/96-decide-deferred-wait0-poll-settlement.md)

`models-deferred-result-before-source-eof` uses one ready zero-wait
fetch with gates before source `Done` and before EOF. After `Done` Pi
has the ready result while the same Rust fetch future stays pending.
Both have an open poll span and the first-chunk duration sample at
that checkpoint. EOF release yields matching results and settled
measurements, and the Rust driver observes source Drop. The result
timing is an intentional difference for the direct completion entry.
The `models-deferred-ready-before-source-eof` control returns the
result at the held EOF gate on both sides. #97 replays both cases and
the Durable Harness consumer on the original-lock #89 snapshot.
[#56 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/56-observe-deferred-result-before-source-eof.md) ·
[#97 decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/97-decide-deferred-fetch-held-eof.md)

`models-deferred-local-cancel-after-done` cancels the local fetch at
the same held EOF gate. Both backends receive the cancellation state
and their scripted source waits for the gate. Rust releases the source
and settles the poll with its cached ready result. The Pi source and
poll settle after the driver releases EOF. Both keep the remote cancel
entry unused and return the ready message on an explicit refetch with
the original handle. #98 retains the local cleanup and poll timing
difference for cancellation after a successful cached terminal.
[#57 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/57-observe-deferred-local-cancel-after-done.md) ·
[#98 decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/98-decide-deferred-local-cancel-after-done.md)

`models-deferred-local-cancel-before-done` cancels one zero-wait fetch
after `Start`, with the successful terminal gate still closed. The Pi
fixture observes the forwarded signal and emits an `aborted` terminal.
Pi fulfills the fetch with that message. Rust cancels the pending source
and returns a typed `Cancelled` failure carrying an `Aborted` message.
The settled poll records agree. Pi reads the source result and finishes
the span after the public await. Rust drops the source and settles the
span before returning. Both leave remote cancel unused and explicitly
refetch the ready result on the original handle. #99 retains the typed
result channel, diagnostic and local cleanup differences and verifies
the Durable application abort path on the same Pi snapshot.
[#58 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/58-observe-deferred-local-cancel-before-done.md) ·
[#99 decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/99-decide-deferred-local-cancel-before-done.md)

`models-deferred-remote-cancel-then-fetch` obtains a handle from the compiled
Pi Faux provider and pairs it with Rust's scripted deferred backend. Each
backend receives a pending zero-wait fetch, one explicit cancel with caller
telemetry, then one failed fetch of the same handle. The cancel emits no SDK
AI telemetry. The first replay fails on missing provider and model IDs in
the Rust error message. The repair uses the handle as the identity for a
synthesized failure before any partial. The replay compares the repaired
payload, final poll attributes, request duration and counters. The #59 replay
captures the first-chunk difference before the #60 repair. #93 replays the
current case and checks how Durable handles both diagnostics.
[#59 replay and repair](../../../docs/iterations/opi-ai-1-5-alignment/issues/59-verify-deferred-remote-cancel-then-fetch.md)

`models-deferred-auth-setup-failure` holds authentication before dispatch
for one zero-wait fetch. Each side reaches the resolver once with an open
poll span and no measurements. Releasing the gate yields the error outcome
with zero fetch or cancel dispatches. Pi records first-chunk telemetry in
both this case and the refetch after remote cancel. The repaired Rust
observer records one first-chunk event and duration sample on the first
typed failure. The comparator requires both observations. Result channels,
typed causes and diagnostics remain visible as differences.
[#60 replay and repair](../../../docs/iterations/opi-ai-1-5-alignment/issues/60-align-first-chunk-on-typed-failure.md)

#95 reproduces this case on the #89 materials and checks the disposition
of all 24 frozen cases. #100 retains the typed `AuthFailed` result and
current diagnostic as intentional differences. The pinned Durable Harness
records each diagnostic and settles the #60 input as `unanswered/model_error`
with zero retries. The resolver 503 control retries once under either
diagnostic. The SDK and Durable receipts preserve the distinct error text.
[#95 targeted replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/95-replay-deferred-auth-setup-failure.md) ·
[#100 caller verification and decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/100-decide-deferred-auth-setup-failure.md)

`models-deferred-positive-wait` passes a `50` ms wait and an independent
`25` ms handle hint to a controlled backend. Each public call dispatches
once and stays pending while its backend source holds the first event.
The first release returns `deferred` with the original handle. A second
explicit call returns the ready message after its own release. The paired
case checks both settled poll records and zero remote cancel calls.
[#61 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/61-verify-deferred-positive-wait.md)

`models-deferred-handle-expiry` uses a ready job with expiry metadata and
three explicit zero-wait fetches. Just before expiry and with the Rust
clock omitted, both SDKs resolve auth, dispatch once and return the same
ready payload and settled poll measurements. At equality Rust returns a
`Config` failure before auth or dispatch. Frozen Pi forwards the handle
after auth and Faux returns the ready payload. The targeted #91 replay
reuses the #89 materials and matches its expiry case exactly. Opi
delegates expiry by default and preserves the caller's opt-in cutoff.
[#62 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/62-observe-deferred-handle-expiry.md) ·
[#91 expiry decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/91-defer-expiry-to-provider-by-default.md)

`models-deferred-handle-route-mismatch` changes the provider, model and
API independently on one job handle. Pi resolves auth and dispatches each
fetch before Faux rejects it. Rust rejects each invalid handle before auth
and dispatch. The direct Rust validator returns `Unsupported` for provider
and API and `RouteMismatch` for model. The fourth fetch uses the original
handle and returns the same ready payload on both sides. The rejection
stage, result channel, diagnosis and route labels remain recorded.
#92 replays the same fetch case and compares explicit cancel. Faux
accepts all three mismatched cancels and changes the original job's
state. Opi rejects them before auth and the original handle stays usable.
[#63 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/63-compare-deferred-handle-route-mismatch.md) ·
[#92 route decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/92-validate-deferred-routes-before-dispatch.md)

`models-deferred-unknown-job-id` changes only the job ID. The Rust handle
validator accepts the route and both SDKs resolve auth and dispatch the
zero-wait fetch once. Both backends report a missing job, with matching
error payload fields and settled poll measurements. The original handle
still returns the ready payload on a second explicit fetch. The result
channel, fixture cause and diagnostic remain in the raw comparison.
[#64 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/64-compare-deferred-unknown-job-id.md)

#93 repeats #64 and #59 on the #89 materials. The frozen Durable Harness
classifies the four exact diagnostics as `model_error` with no retry or
remote cancel. Rust's public result preserves the scripted backend's
cause and its message classifies as terminal. The `Config` diagnostic
reflects the fixture's mapping of a backend job state.
[#93 classification decision](../../../docs/iterations/opi-ai-1-5-alignment/issues/93-assess-deferred-job-failure-classification.md)

The #65 replay of both cases uses the same original-lock Pi snapshot.
The Rust provider and API mismatch rows name the missing registration
in the fetch error and poll status. The model mismatch, original handle
refetch, and complete unknown-job-ID case match #64. The public Rust
contract test also checks the registration diagnostic on explicit cancel.
[#65 replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/65-clarify-deferred-api-lookup-diagnostic.md)

In `models-deferred-stream-source-cleanup`, Pi's eager entry and Rust's
`.start()` deliver `Done` with source EOF held. Pi also delivers public EOF
and continues forwarding, then settles the poll after source release.
The started Rust task continues draining with the poll open. Public EOF
and the result follow source release and settlement. The started controls
also cover cleanup after consumer Drop and starting after a direct `Done`.
The direct Rust collector's final `next()` drives the retained tail and
waits at the gate. Local cancellation can release that pending tail and
return public EOF with the ready result cached.
[#67 baseline](../../../docs/iterations/opi-ai-1-5-alignment/issues/67-compare-deferred-stream-source-cleanup.md) ·
[#74 started Done replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/74-deliver-started-deferred-done-before-source-release.md)

#101 reproduces the #89 stream cleanup case and checks the started result
future at the held tail. Keep the started `result()` cleanup wait and the
direct collector's caller-owned release. #102 settles the direct poll
from the cached `Done` after dropping the source, including after the
tail is polled and pending. The expanded Rust trace records one backend
finish with the source already released in both cases and after normal
EOF. The comparator retains support for the original frozen trace.
The Python subprocess tests can use the sealed repaired driver through
`OPI_LIFECYCLE_DRIVER`.
[#101 replay and decisions](../../../docs/iterations/opi-ai-1-5-alignment/issues/101-decide-deferred-stream-result-and-drop.md) ·
[#102 repair and before/after replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/102-settle-deferred-poll-on-direct-drop.md)

The result-only consumer probe runs Pi Durable against the same frozen
source snapshot. It holds the pending and ready fetch tails while the
application schedules the next poll and writes the answer. Run it with
`node --conditions=source --experimental-strip-types` followed by the probe
path and `--snapshot <path>`.
[#75 consumer proof](../../../docs/iterations/opi-ai-1-5-alignment/issues/75-review-deferred-result-consumer.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_deferred_result_consumer.mjs)

`models-deferred-ready-before-source-eof` reuses the result timing fixture
and calls `Models::fetch_deferred_ready` on Rust. The comparator requires
both outcomes at the held EOF gate with the poll open, then source release
and settled telemetry. The original result timing case exercises the
cleanup wait in `Models::fetch_deferred`.
[#76 paired replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/76-return-deferred-outcome-before-source-release.md)

The deferred failure consumer probe runs the pinned Durable Harness on the
same snapshot. It exercises transient and quota errors, two overflow paths,
a provider abort and an application abort during a fetch. Use the same Node
flags and `--snapshot` argument as the result-only consumer probe. The public
Rust failure-input test emits the corresponding `CallFailure` observations
with `--nocapture`.
[#78 classification proof](../../../docs/iterations/opi-ai-1-5-alignment/issues/78-review-deferred-failure-classification.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_deferred_failure_consumer.mjs)

The overflow alignment checks both diagnostics through the public predicate,
deferred failure input and whole-call retry tests. The lifecycle replay
binds the updated Rust source to the same original-lock snapshot.
[#79 overflow alignment](../../../docs/iterations/opi-ai-1-5-alignment/issues/79-align-context-overflow-diagnostics.md)

The legacy provider abort control compares the preserved `Aborted` message
with the same frozen Durable observation. The Rust readiness test checks
the caller's live token and poll settlement after the held tail releases.
[#80 abort alignment](../../../docs/iterations/opi-ai-1-5-alignment/issues/80-preserve-legacy-provider-abort.md)

The retry classification proof evaluates the same diagnostic inputs with
the frozen Pi predicate and overflow gate. The Rust deferred controls read
the canonical message classification and the typed cause retry flag.
[#81 retry classification](../../../docs/iterations/opi-ai-1-5-alignment/issues/81-align-whole-call-retry-classification.md)

The Responses failure probe feeds the same synthetic `response.failed`
fixtures through the pinned Pi OpenAI, Azure and Codex SSE entries. Run
`node --conditions=source` followed by the probe path and `--snapshot <path>`.
The Rust wire control records the explicit whole-call decision, prepared
acquisition seal, partial message and telemetry settlement.
[#82 Responses failures](../../../docs/iterations/opi-ai-1-5-alignment/issues/82-preserve-responses-stream-failure-classification.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_responses_failed.mjs)

The top-level SSE error probe uses the same snapshot and Node flags. It
exercises named and data-only SDK errors and the Codex field precedence.
The Rust fixtures check public failures, whole-call request counts, and
Codex WebSocket reconnect and continuation boundaries.
[#83 SSE error classification](../../../docs/iterations/opi-ai-1-5-alignment/issues/83-preserve-responses-sse-error-classification.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_responses_error.mjs)

The Codex connection-limit probe reuses the same snapshot and flags.
The shared fixture records the sends before SSE handoff, the cached
increment and full input, the subsequent session call and the failed
POST budget. The Rust loopback controls exercise post-Upgrade acquisition
and cancellation of the active fallback.
[#84 Codex fallback](../../../docs/iterations/opi-ai-1-5-alignment/issues/84-fallback-after-codex-connection-limit.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_connection_limit.mjs)

The Codex failed-response probe replays the same nested recovery frames
as the Rust loopback test. It records the shared retry budgets, cached
input, the final failure status and the distinct outcomes after a public
text delta. Use the same Node flags and snapshot.
[#85 Codex recovery](../../../docs/iterations/opi-ai-1-5-alignment/issues/85-recover-codex-response-failed-codes.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_failed_recovery.mjs)

The Codex terminal probe replays 16 shared frame sequences on both
transports with the same Node flags and snapshot. The paired Rust
controls exercise terminal status, usage, response identity, diagnostic
round trips, SSE release and cached WebSocket continuation.
[#86 Codex terminals](../../../docs/iterations/opi-ai-1-5-alignment/issues/86-align-codex-terminal-frames.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_terminal.mjs)

The Codex WebSocket idle-input probe uses the native Node WebSocket
against a loopback peer. It batches the first terminal with its tail
and verifies the next result, raw callbacks, Ping/Pong order, and
cached input. Run it with the same Node flags and snapshot.
[#87 pooled Codex input](../../../docs/iterations/opi-ai-1-5-alignment/issues/87-isolate-codex-websocket-idle-input.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_websocket_idle.mjs)

The Codex WebSocket identity probe delivers the old terminal after the
peer receives the second create. It records the nested response ID,
usage, raw callback order and next-turn continuation. Run it with the
same Node flags and snapshot.
[#88 post-send terminals](../../../docs/iterations/opi-ai-1-5-alignment/issues/88-attribute-codex-websocket-terminals.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_websocket_identity.mjs)

The Codex WebSocket idle-timeout probe uses a loopback peer with continuous
Ping or Pong, silence, progress events, and a slow raw callback. It records
the Close code and reason, current terminal, and the next session route.
Run it with the same Node flags and snapshot.
[#89 application idle timeout](../../../docs/iterations/opi-ai-1-5-alignment/issues/89-align-codex-websocket-application-idle-timeout.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_websocket_idle_timeout.mjs)

The Codex session cleanup probe reuses the #89 frames with the same
snapshot and Node flags. It calls the public package cleanup export after
two sessions fall back to SSE. A healthy pooled connection controls for
cleanup scope. The Rust test awaits `Models::cleanup_session_resources`
and records the effects, next requests and results.
[#90 session cleanup](../../../docs/iterations/opi-ai-1-5-alignment/issues/90-preserve-codex-session-cleanup-recovery.md) ·
[Probe](../../../docs/iterations/opi-ai-1-5-alignment/tools/observe_codex_session_fallback_cleanup.mjs)

When the caller stops at `Done`, Pi's forwarder continues, while the
Rust direct collector releases the unfinished source and leaves the
in-memory poll open. The Rust stop after public EOF now observes a
completed source drain. The controls compare ready result fields and
check dispatch with the original handle, zero wait and zero remote
cancels.
[#69 stop baseline](../../../docs/iterations/opi-ai-1-5-alignment/issues/69-observe-deferred-direct-consumer-stop.md) ·
[#70 direct EOF drain](../../../docs/iterations/opi-ai-1-5-alignment/issues/70-drain-deferred-source-before-direct-eof.md)

The Rust resumption controls drop a pending final `next()` future while
keeping the direct collector. Both `result()` and `.start()` poll the same
held tail before the EOF gate is released. The started worker also does
so after its consumer is dropped. The controls check natural EOF, source
release, one settled poll and the cached ready fields where observable.
[#71 resumption replay](../../../docs/iterations/opi-ai-1-5-alignment/issues/71-resume-deferred-tail-after-pending-next.md)

Use `observe_upstream.mjs --case <case> --snapshot <path>` for the raw
`lazyStream` cases `lazy-no-consumption`, `lazy-stop-consumption` and
`lazy-setup-failure-phases`. The raw construction counter measures the
`lazyStream` setup callback. The public replay measures provider dispatch
in the completion cases and resolver entry in the controlled auth failure.
See the [source-tail repair report](../../../docs/iterations/opi-ai-1-5-alignment/phase5-ordinary-source-tail.md)
for the historical completion traces and
[#43](../../../docs/iterations/opi-ai-1-5-alignment/issues/43-match-public-models-completion.md)
for the paired public replay.

The Chat finish probes hold `finish_reason`, usage, `[DONE]` and HTTP EOF
separately. `replay_chat_finish_timing.py` checks eight matched request pairs
on sealed Rust/Pi inputs; `replay_chat_finish_consumers.py` checks a real Opi
Agent call and recorded-event projections into the frozen real Durable
Harness. Both repeat observations three times and refuse to overwrite an
existing receipt. Their historical #107 verdict was **repair-required**, with
`strict_alignment=false`; passing these observation checks does not accept
the timing differences. The consumer runner rejects source drift against
the Agent build receipt before merging current observation inputs.
See [#107](../../../docs/iterations/opi-ai-1-5-alignment/issues/107-observe-chat-finish-terminal-boundary.md)
for the original tool/storage consequences and scope limits.


### Repaired Chat terminal boundary (#108)

The Chat timing and consumer replay commands now verify the repaired contract:
finish/usage remain pending; sentinel or valid EOF settles; late failures
suppress tools; the peer is released at the terminal. The only accepted
block-end sequence difference in the eight pairs is cancellation: Opi
preempts queued events without synthetic ends, while frozen Pi can close
blocks during SDK abort unwind. The report retains `strict_alignment=false`
and scopes its comparison explicitly; partial aliasing is not hidden by
normalization or claimed as aligned.

Use a fresh material manifest and source-bound executable receipt, then run
`replay_chat_finish_timing.py` and `replay_chat_finish_consumers.py` with new
output paths. Historical `phase5-chat-finish-*` receipts remain #107's
pre-repair evidence and intentionally fail the repaired comparator. The
unit suite checks that rejection, leaked responses, early tools/storage,
wrong results, cancellation ends, source drift and overwrite refusal.
See [#108](../../../docs/iterations/opi-ai-1-5-alignment/issues/108-repair-chat-terminal-boundary.md)
for current proof and the original-source archive mapping.
