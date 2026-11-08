# opi-telemetry

[简体中文](README.zh.md)

This crate follows the current workspace version `0.8.3`.

Explicit, backend-neutral telemetry for Rust libraries. Use a no-op context,
an in-memory recorder, or the optional tracing adapter without initializing a
global subscriber. The default build has no tracing, async runtime, networking,
AI, agent or CLI dependency.

This crate is introduced in the unreleased workspace. The manifest below applies
when the matching package is available in your registry; registry publication
is a separate release step. You can also consume an extracted `.crate` artifact
with a Cargo path dependency, independently of the Opi workspace.

```toml
[dependencies]
opi-telemetry = { version = "0.8.3", default-features = false }
```

## Record an operation

The complete offline program is [examples/record_request.rs](examples/record_request.rs).
Run it with `cargo run -p opi-telemetry --example record_request` in the workspace,
or copy it as `src/main.rs` into a project using the dependency above.

```rust
use opi_telemetry::memory::Recorder;
use opi_telemetry::{Attributes, SpanOptions};

fn main() -> opi_telemetry::Result<()> {
    let recorder = Recorder::default();
    let context = recorder.context();
    let owner = context.start_span(SpanOptions::new("example.operation"))?;
    let observed = owner.span();
    observed.add_event("work.started", &Attributes::new())?;
    let answer = owner.context().in_scope(|| 42);
    observed.set_attributes(&[("answer".into(), answer.to_string().into())].into())?;
    owner.finish(None)?;
    assert_eq!(recorder.spans()[0].end_sequence, Some(0));
    Ok(())
}
```

## Ownership and failure

`start_span` starts synchronously. Keep its unique `SpanOwner` in the execution
task and call `finish` when that operation ends. The cloneable `TelemetrySpan`
only records observations; its `context()` starts children and provides metrics.
Dropping a context or observation handle neither finishes a logical span nor
cancels business work. Dropping an instrumented future does drop its inner future
in the captured scope; cancellation follows that future's own contract. The
wrapper never finishes a logical span. Dropping an unfinished owner leaves the
record unfinished. The execution layer must keep request ownership separate
from stream consumption.

`finish(None)` retains the latest explicit status, defaulting to `Ok`.
`finish(Some(ErrorStatus { .. }))` supplies an automatic error only if no explicit
status was set. Explicit status wins. Late attributes, events and status changes
are ignored; new children through a settled live context are inert but retain
metrics capability. An already-started child has its own completion lifetime.

Adapter errors return `TelemetryError` with their original source. Factories,
recording and completion are synchronous, independently fallible operations.
There is no implicit retry, panic catch or automatic business-error conversion.
`finish` consumes the owner even if the backend fails; that failure is not a
successful settlement. Backend implementations own serialization with concurrent
recording. Business callback/promise error mapping belongs to the execution layer.

`in_scope` executes its body once and restores the caller's scope on return or
unwind. `instrument` is lazy, polls the inner future once per poll, preserves its
output and `Pending`, and scopes its Drop. No scope guard crosses an await.
No-op disables only this interface's recording and preserves ambient host tracing.

## Metrics and schemas

`context.metrics()` provides synchronous counter, up/down counter and histogram
factories. Supply an `InstrumentDefinition` with name, description, unit and
optional histogram boundaries. The selected factory determines its kind.
Counter increments are nonnegative; measurements are finite; histogram boundaries
are finite and strictly increasing. These are trusted-input preconditions, not
an additional runtime validation policy. Histograms can carry an explicit context.

`schema` contains serializable descriptions for span parents, start/end
attributes, required event attributes, status and metric metadata. Attribute
metadata retains values, examples, sensitivity and cardinality. A
`MetricDefinition::instrument(name)` forwards metadata to a factory. Generic
recording does not enforce schema names or attributes. Domain crates own typed
instrumentation and their schema; no AI names or token/cost rules live here.

The recorder returns detached spans in start order and measurements in recording
order. `end_sequence` records completion order. It retains records until dropped;
use it for tests and bounded diagnostic sessions, not as an unbounded production
exporter. It has no Store/query/render or wire propagation interface.

## Optional tracing adapter

Enable `features = ["tracing"]`, then call
`TelemetryContext::from_span(host_span)`. It freezes the span's own dispatcher,
including across threads, future polls and Drop; it does not capture a different
ambient dispatcher. Disabled spans become neutral no-op contexts.

Neutral child spans use `opi.telemetry.span` with a `telemetry.name` field.
Events and measurements carry metadata in `telemetry.*` debug fields. A
`telemetry.operation = "finish"` event marks logical completion; native tracing
close can wait for an already-entered synchronous scope. This is a tracing
adapter, not a complete exporter. The host owns initialization, filtering,
sampling, redaction, flushing and shutdown.

`opi-ai::telemetry` re-exports these carrier paths. When a caller passes an
explicit context, ordinary `Models` chat installs an AI-owned `pi.ai.request`
observer and deferred fetch installs `pi.ai.poll`; a memory recorder therefore
captures the AI span, events, and usage/cost settlement emitted through this
neutral carrier. Raw `ChatStream` and prepared raw attempts remain manual
boundaries, while image, classifier, and deferred-cancel operations deliberately
carry context without installing an AI span. No context captures no ambient
subscriber. AI schema, token, and cost rules remain in `opi-ai`; this crate owns
the neutral context, backend, and adapter contracts only.

Custom backends implement the contracts in `backend`. Shared public scenarios in
`tests/support` exercise no-op, recording and tracing; backend-specific tests
cover routing. No production test probes are exposed.
