---
name: opi-slim-tests
description: Cut opi's test integration-binary count to speed compiles — each tests/*.rs is its own binary/link step. Use when the user wants to slim the test suite, cut test compile/build time, or says tests are slow, too numerous, or bloated. Preserves coverage.
---

# opi-slim-tests

Cut the opi test suite's integration-**binary** count to speed up `cargo test` / CI, without losing coverage.

## The lever

Every `tests/*.rs` is a separate integration **binary**: Cargo compiles and **links** the full dependency tree (reqwest, tokio, ratatui, clap, schemars, wiremock) once per file. **File count, not test count, is the compile cost.** Files in `tests/` *subdirectories* are modules, not binaries — a `tests/common/mod.rs` reached by `mod common;` costs zero binaries. Reducing binaries (link steps) is the only lever that moves wall-clock; rearranging tests within one file does nothing for compile time.

## Steps

### 1. Inventory — where the link-step mass is
Count test files and lines per crate; flag **thin binaries** (≈1–13 tests) — they pay a full link for almost nothing and are the cheapest merge fuel. opi-coding-agent (~77 binaries) is historically the centre of mass.
Criterion: a ranked list of binaries by test count, thin binaries marked.

### 2. Classify each candidate — clone / per-X / load-bearing
Read the duplicate test *bodies*; never classify from names alone (same name often masks different coverage).
- **clone** — identical assertions, differing only in a model string / fixture / provider constructor → **merge** or **delete** (the subsuming test must prove identical coverage).
- **per-X** — same name, but each copy tests a *different* thing (find vs ls vs glob; openai vs mistral profile) → **not redundant**. Merge by renaming per-X or wrapping each in a sibling `mod`; **skip** when per-file locality outweighs the link-step win.
- **load-bearing** — release-critical (see step 5) → **hold**; never change without a byte-identity gate + linux CI.
Criterion: every candidate labelled clone / per-X / load-bearing, with evidence quoted from the bodies.

### 3. Merge (clones only)
- **clean merge** — `cat` the files, then strip each per-file `//!` header and duplicate `use` lines at every **seam**; dedup byte-identical helpers; keep one copy where signatures differ.
- **helper-colliding but distinct** — wrap each file verbatim in a sibling `mod` (`mod profile_a { … } mod profile_b { … }`); mod-namespacing removes the collision with zero behavior change.
- **misplaced pure unit test** (tests a library fn, no subprocess) — inline into the owning `src/*.rs` as `#[cfg(test)]`.
- **shared helpers** → `tests/common/mod.rs` with `#![allow(dead_code)]` at the top — the module is compiled per-binary, so a helper unused by one binary trips `dead_code` and CI fails under `-D warnings` without it.
Criterion: merged file compiles — `cargo clippy -p <crate> --test <name> -- -D warnings`.

### 4. Verify — CI parity, reported honestly
`cargo clippy -p <crate> --test <name> -- -D warnings` + `cargo test -p <crate> --test <name>` + **`cargo fmt --check --all`**. The fmt gate bites cat-built merges (rustfmt reflows unified imports and indents `mod` bodies): run `cargo fmt --all`, then re-check.
**cfg(unix) tests are invisible on this Windows host** — they compile out locally; never claim a unix-only test passes from a local run. The linux CI job is authoritative for them.
Criterion: clippy clean, converted tests pass locally, fmt clean; unix-gated coverage explicitly deferred to CI.

### 5. Hold the load-bearing guards
Do not silently change: the **phase4/phase6 ledger** SHA-256 of `docs/opi-spec.md` (CRLF-normalized) vs `docs/snapshots/phase{4,6}/`; `productized_packages_docs` `CARGO_PKG_VERSION` guard; the doc-guards' **two `no_positive_claim` token sets** (Phase5/6 vs Phase7 — never unify into one); the `strip_rust_comments` variants. Touch only with a byte-identity gate (sha256 of helper bodies pre/post) and confirm on linux CI.
Criterion: each load-bearing item either untouched, or verified byte-identical and CI-green.

### 6. Commit
Feature branch (never commit direct to `main`); Conventional Commits (`test(crate): …`); `git add` only your files — never `-A`/`.` (the opi working tree routinely carries other agents' uncommitted changes).
Criterion: clean commit on a feature branch, only your files staged.

## Placement — the publish-leak rule
Shared helpers must not enlarge a published crate's API:
- **opi-coding-agent** publishes to crates.io *and* has a lib target → use `tests/common/mod.rs` (`mod common;`). **Never** `src/test_support.rs`: `#[doc(hidden)]` masks rustdoc only, not the callable public API.
- **pure unit tests** inlined as `#[cfg(test)]` are always publish-safe (cfg-gated out of release builds).
- **opi-ai** already leaks `src/test_support` (MockProvider); extending it adds no *new* surface class, but `tests/common` avoids the leak entirely.

## Host constraints
- Windows host: `cfg(unix)` tests compile out — invisible to local `cargo test`/`clippy`. Grep for bare `extern "C"` / `unsafe` before pushing unix-test changes (edition-2024 `unsafe extern` errors fail linux CI, not local).
- Full-workspace `cargo test --workspace --all-targets` can fill the 452G disk (~106 GB `target/` bloat). Use per-crate / per-`--test` builds.
