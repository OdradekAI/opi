# Language-native reproduction and architecture review

Full reproduction means mapping every in-scope capability and observable
contract of the supplied snapshot to Opi. Rust-native names, types and module
boundaries are allowed; an observable difference is not erased by calling it
idiomatic, safer, intentional, or outside Opi's current plan. Separate native
Rust usability from any claim that existing JS callers or persisted artifacts
work unchanged; the latter requires actual compatibility evidence.

## Behavioral translation

Compare the complete caller workflow and each public entry level, not only the
highest-level convenience method. Record where setup, authentication, callbacks,
network requests, results and cleanup happen. Preserve meaningful distinctions:

- missing, undefined, null, false, zero, empty values and explicit defaults;
- open provider/API identifiers versus closed state/model-type alternatives;
- numeric precision, negative zero, non-finite input where reachable, threshold
  direction, map/array order and wire serialization;
- UTF-8 bytes, UTF-16 code units, Unicode scalars and chunk boundaries;
- mutable/live partials versus snapshots, aliasing, input mutation and retained
  references. Describe the Rust observation mechanism or record the difference;
- synchronous errors, returned failures, stream errors and errors after effects;
- JS eager work versus Rust Future polling, spawned tasks, drop, cancellation,
  stopping consumption, and remote cancellation. None implies all others;
- durable commit versus in-memory acceptance/publication, and caller cancellation
  versus credential rotation or other work that must finish.

Do not narrow legal inputs or silently discard fields to make tests pass.
Safety/resource improvements require their own acceptance and a visible parity
difference when they change the target behavior. Suspected upstream defects
need evidence too; preserve unresolved source/doc/test conflicts.

## Engineering review

A module owns a coherent responsibility behind an interface; that interface
includes types, ordering, errors, configuration, lifecycle and performance, not
just function signatures. Compare pi's design with Opi's actual implementation
using these checks, without consulting Opi's specification or roadmap:

| Check | Evidence and recommendation criteria |
|---|---|
| Responsibility and depth | Map capability owners and real callers. Prefer one coherent owner over duplicate rules or pass-through layers; show what complexity removal would return to callers. |
| Dependency direction | Inspect production, build and test graphs, target dependencies, features, lockfiles, native/proc-macro/build-script closure. Domain and pure logic need not expose transport/vendor implementation types. |
| State and resources | Identify creator, holder, mutation/commit authority, concurrent access, cancellation, lifetime and cleanup of every relevant task, cache, connection, lock and durable record. Detect competing mutable truths. |
| Real interfaces | Explain which intrinsic state transition or real consumer/adapter variation justifies a public interface. A mock alone does not justify exposing internals. |
| Locality and extensibility | Trace adding a provider sharing a protocol, a custom protocol, a tool or a storage adapter. Show which existing modules/callers change and where semantics would be duplicated. |
| Testability | Public callers and acceptance tests cross the same interface; production and test adapters share applicable contracts. Private mechanism tests remain useful but prove a different scope. |
| Distribution | Check a minimal consumer and full-capability consumer, independent package resolution, host/runtime constraints, optional dependencies and feature unification. A source module is not automatically a separate crate. |
| Authority and failure | Identify actual trust, process/OS isolation and failure boundaries. In-process code and unsandboxed subprocesses do not become safe merely through metadata or an RPC interface. |

Recommend retaining, deepening, merging, moving or splitting a module only with
concrete evidence of behavior, ownership, coupling, consumer needs or operational
cost. Explain the smallest viable change, alternatives, migration, risks and
acceptance. Do not optimize for crate count, file-size symmetry, one-to-one pi
packages, or a predetermined core/plugin split. Keep behavioral preservation
separate from structural improvement; a different layout can already be sound.

Engineering recommendations are conditional design advice, not permission to
change Opi. Where evidence is insufficient, recommend the specific experiment
or measurement that would distinguish alternatives instead of declaring a
preferred architecture proven.
