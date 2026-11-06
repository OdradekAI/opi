# Engineering

This is coding guidance, not a capability roadmap or frozen crate graph.
Cargo manifests own workspace topology, versions, edition, MSRV and dependencies.
Owning source, tests, schemas and READMEs describe current interfaces.

## Structure

- Prefer deep modules: substantial behavior behind a small interface. The
  interface includes invariants, ordering, cancellation, errors and resource
  ownership, not just method signatures. Keep those facts local and testable.
- Search existing owners and consumers before adding a module. A public seam
  needs intrinsic state-machine value or real variation with shared conformance;
  do not invent adapters, traits or crates to anticipate unknown consumers.
- Separate mechanism from policy. Keep reusable runtime code independent of
  terminal interaction, product defaults and optional workflow policy. Explain
  new dependency direction from actual callers, not old layer names alone.
- Improve the structure touched by the task, not the whole repository. Before
  deleting/moving an interface, trace production, configuration, wire, persisted
  and external use. Remove superseded glue only after replacement behavior is
  proven. Preserve intentional behavior unless the user authorizes its change.
- Record expensive, non-obvious trade-offs in a focused ADR. Routine factoring
  needs neither an ADR nor a placement ceremony. Use [context alignment](../agents/context.md)
  when an existing decision conflicts with the requested change.

## Rust interfaces and ownership

- Use enums, newtypes and controlled construction when they encode meaningful
  states or invariants. Validate external inputs and fail closed at trust
  boundaries. Internal code may rely only on invariants enforced by every
  construction/deserialization path. Keep fields private when mutation could
  invalidate those guarantees.
- Expose typed errors where callers need to classify or recover from failures;
  preserve causes and useful context. Follow existing error conventions,
  including `thiserror`, before adding a dependency. Application/reporting
  boundaries may erase error types when only a report is needed; `anyhow` is an
  option, not a requirement for every binary or test. Represent expected input/I/O
  failures with `Result` or the owning typed outcome; reserve panics for violated
  programming invariants.
- Start with concrete types. Use generics for needed static variation and
  trait objects for runtime heterogeneity or an established behavior boundary.
  Choose borrowing, ownership transfer or shared ownership from actual lifetimes
  and sharing needs. Cloning is appropriate when it simplifies ownership at an
  acceptable cost; measure before adding performance-driven complexity.
- Prefer safe Rust. Isolate necessary `unsafe` behind a small boundary, justify
  its use, and document why its safety conditions hold, including lifetime and
  aliasing requirements. Safe APIs must preserve those conditions for all
  permitted callers.
- Document affected public contracts with useful examples and their error,
  panic and safety behavior. Helpers should clarify intent, contain invariants
  or hide complexity; call count alone does not determine their value.

## Async and resource lifecycle

- Identify who owns each spawned task/resource, observes completion, requests
  cancellation and performs cleanup. Define partial effects at cancellation
  points and bounds/backpressure for concurrent work and queues. Sending a
  cancellation request does not prove completion; join or reap owned work as
  required by its contract before reporting a terminal result.
- Keep synchronous lock guards out of `.await` points. Short, low-contention
  data access may use a synchronous mutex; asynchronous I/O state may need an
  async mutex or an owning task with message passing. Keep blocking or long
  CPU-bound work off async executor workers using the appropriate runtime boundary.

## Dependencies and compatibility

- Internal dependencies use root `[workspace.dependencies]` and
  `{ workspace = true }`; publishable paths also need versions. Keep lockstep
  versioning and workspace-owned metadata in root manifests.
- Review manifests, feature changes, lockfile, build scripts, native code and
  proc macros as code. Check APIs against the exact locked version. Generate
  `Cargo.lock` with Cargo and review unexpected transitive changes. Explain
  portability, authority and supply-chain effects before broadening defaults.
- Resolve dependency/API mismatches without dropping required behavior or
  suppressing diagnostics wholesale. Any lint exception needs a narrow scope
  and a reason tied to the affected code.
- Scoped 0.x breaks update Unreleased, callers, schemas, fixtures, tests and
  existing bilingual docs together. Add compatibility only for a real owning
  contract; revisit this policy before 1.0.
- Preserve user-owned authority: a model decision cannot grant permission,
  silently widen policy or turn credential availability into execution consent.

## Reference standards

Use these references for an unresolved question or an affected contract; read
only the relevant sections and reuse still-current context. They inform design
and review. Applying them requires the project's actual consumers, constraints
and accepted decisions; they do not authorize new dependencies, broad rewrites
or compatibility changes by themselves.

| When | Reference and focus |
|---|---|
| Designing or reviewing a public Rust API | [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html): naming, standard traits, type safety, errors, documentation and evolution |
| Introducing or changing an unsafe/FFI boundary | [Rust Reference](https://doc.rust-lang.org/reference/unsafe-keyword.html): safety conditions and proof obligations |
| Changing async state or shutdown | Tokio's [shared state](https://tokio.rs/tokio/tutorial/shared-state) and [graceful shutdown](https://tokio.rs/tokio/topics/shutdown): locks, task ownership, cancellation and completion |
| Changing public APIs, features, MSRV or platform support | [Cargo SemVer Compatibility](https://doc.rust-lang.org/cargo/reference/semver.html), interpreted with this project's 0.x policy |
| Choosing or overriding lints | [Clippy](https://doc.rust-lang.org/clippy/usage.html): select applicable checks and justify exceptions; do not enable all restriction lints |
| Seeking additional library/application design guidance | [Microsoft Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/): evaluate individual recommendations; crate splitting, allocators and error representations remain project decisions |

## Verification

Inventory touched surfaces and select their check union under
[change scope and check selection](../../.agents/skills/_shared/references/change-scope-and-check-selection.md).
That reference selects checks, not scope/admission. Reuse applicable proof;
rerun when source, toolchain, features, target or material inputs invalidate it.

- Docs/skills/metadata: `python scripts/opi-doc-check.py` and `git diff --check`.
  Script/checker changes also run focused tests and a CLI/subprocess negative path.
- Runtime: reproduce bugs, test invalid inputs, and exercise changed behavior at
  the consumer interface. Run every created/modified test binary or filter;
  zero selected/executed tests are not a passing behavioral check. Use applicable
  integration, snapshot and conformance checks at public/durable seams.
- Cross-crate product runtime and CI/release: applicable product gates below.
  A local one-crate change uses focused tests and affected-target clippy unless
  its public impact requires broader proof. [CI policy](../ci.md) owns platform
  selection and major-iteration/native evidence, not the task's filename.

```text
cargo fmt --check --all
cargo clippy --workspace --exclude opi-eval --all-targets -- -D warnings
cargo test --workspace --exclude opi-eval --all-targets
cargo test --workspace --exclude opi-eval --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --exclude opi-eval --no-deps
```

`opi-eval` is unpublished development calibration; changes get scoped local
checks, not automatic product CI/release admission. Tests use
`opi_ai::test_support::MockProvider`, local fixtures and isolated temp dirs;
serialize environment mutation. No paid provider/live credentials are required,
and tests must not activate live calls merely because credentials exist.

Before Cargo builds, inspect the selected target under the
[Cargo cache lifecycle](../../.agents/skills/_shared/references/cargo-cache-lifecycle.md).
Keep incremental compilation and the shared external cache. Cache deletion is
separately authorized maintenance, never task cleanup.
