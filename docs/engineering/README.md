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

## Rust and dependencies

- Prefer safe Rust, explicit ownership, enums, typed errors, bounded concurrency
  and fail-closed boundary validation. Isolate and justify unavoidable `unsafe`.
  Use `thiserror` in libraries, `anyhow` in binaries/tests, trait objects at
  proven crate seams and generics internally where concrete types are known.
- Internal dependencies use root `[workspace.dependencies]` and
  `{ workspace = true }`; publishable paths also need versions. Keep lockstep
  versioning and workspace-owned metadata in root manifests.
- Review manifests, feature changes, lockfile, build scripts, native code and
  proc macros as code. Check APIs against the exact locked version. Generate
  `Cargo.lock` with Cargo and review unexpected transitive changes. Explain
  portability, authority and supply-chain effects before broadening defaults.
- Scoped 0.x breaks update Unreleased, callers, schemas, fixtures, tests and
  existing bilingual docs together. Add compatibility only for a real owning
  contract; revisit this policy before 1.0.
- Preserve user-owned authority: a model decision cannot grant permission,
  silently widen policy or turn credential availability into execution consent.

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
