# #144 Migrate the OpenRouter Provider profile into `providers`

Status: implemented; canonical namespace enforced; baseline verification caveat recorded below
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Make the first concrete namespace migration from the current provider-specific
root modules to the responsibility-oriented layout agreed in the module
structure decision. Move the OpenRouter Provider profile and catalog factory to
`crates/opi-ai/src/providers/openrouter.rs`, remove the obsolete
`opi_ai::openrouter` root module, and prove that the canonical path works in
the existing provider tests and an external package consumer.

This slice was deliberately limited to the OpenRouter profile. The shared
OpenAI Chat protocol adapter and OpenRouter usage-accounting owner were moved
in subsequent canonical-only slices; this record remains the profile
migration boundary.

## Current contract and migration

- `openrouter.rs` contains Provider profile construction and model metadata;
  it does not own a distinct wire protocol.
- The OpenRouter route continues to reuse the OpenAI Chat adapter and its
  existing feature closure.
- The canonical path is
  `opi_ai::providers::openrouter::{openrouter_provider,model_catalog}`. The
  obsolete root module is removed so new code cannot silently depend on the
  pre-migration namespace.
- No registration, authentication, header, catalog, billing, streaming or
  retry behavior changes are intended.

## Acceptance

1. The implementation source lives under `src/providers/openrouter.rs` and is
   selected only by `provider-openrouter`.
2. The obsolete root module and file are removed; no compatibility shim is
   retained for the migrated namespace.
3. Existing OpenRouter fixtures, cost tests, image serialization, provider
   profile and package-facing tests remain green.
4. A workspace-external consumer built from the actual crate archive can call
   the canonical `providers::openrouter` path.
5. The provider feature profile and warnings-denied Clippy remain green.
6. `git diff --check` and the documentation checker pass.

## Work and verification

- Add the issue and record the ownership boundary.
- Move the implementation, add the `providers` submodule, and remove the root
  module.
- Update only affected internal imports and profile probes if needed.
- Run focused OpenRouter/provider tests, affected Clippy, and the external
  package consumer.

Completed in this slice:

- `crates/opi-ai/src/providers/openrouter.rs` now owns the OpenRouter profile
  factory and model catalog. The obsolete root module and file were removed.
- The provider profile integration probe and the package consumer exercise
  `opi_ai::providers::openrouter` only.
- The canonical-only external proof is recorded in
  [`issue-144-openrouter-package-consumer-20261117-r2.json`](../evidence/issue-144-openrouter-package-consumer-20261117-r2.json).

Verification results:

- PASS: `cargo fmt --all -- --check`.
- PASS: focused OpenRouter tests (`openrouter_fixtures` 55, `openrouter_cost`
  6, `image_provider_serialization` 15, `provider_openrouter_profile` 1).
- PASS: the minimal `--no-default-features --features provider-openrouter`
  profile test (1 test), confirming the new module is selected by the intended
  feature closure.
- PASS: `cargo test -p opi-coding-agent --test provider_factory` (58 tests).
- PASS: package consumer proof from the actual crate archive; the canonical
  namespace compiled and ran with the expected provider feature closure.
- PASS: provider package harness unit tests (6 tests) and task-owned whitespace
  checks.
- PASS with a baseline exception: `cargo clippy -p opi-ai --all-targets
  -- -D warnings -A clippy::too_many_arguments`. The strict command remains
  blocked by the pre-existing `api/openai_responses.rs` function shape in the
  carried-in deferred work; this migration adds no Clippy warning.
- PASS for task-owned paths: `git diff --check`. The repository-wide command
  still reports carried-in CRLF trailing whitespace in unrelated deferred
  changes.
- PASS: `python scripts/opi-doc-check.py`.

## Test impact

`add`: the canonical-path profile assertion and external consumer probe;
`update`: provider module/profile and package-consumer imports;
`delete`: obsolete root module; `retain`: existing OpenRouter fixtures, cost, image, lifecycle,
feature-matrix and historical Pi receipts.
