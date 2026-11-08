# Acceptance Materials and Lifecycle Differential Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Freeze the available Pi 1.5.0 acceptance inputs and produce the first reproducible TS/Rust differential evidence for request lifecycle behavior.

**Architecture:** Keep the existing upstream TypeScript probe as the source oracle. Add a small Rust example using only the public `opi-ai` lifecycle surface and a controlled provider, then normalize both traces with a Python comparator. Record provenance, missing materials, hashes, commands, and classifications in new iteration evidence files; do not alter production behavior.

**Tech Stack:** Node 24 probe scripts, Rust 1.99-compatible `opi-ai` example, Python 3 standard library, JSON evidence, SHA-256, Tokio/futures controlled streams.

## Global Constraints

- Preserve the Pi snapshot identity and never substitute current Rust catalog data for the pinned catalog response.
- Keep live provider credentials and paid calls disabled; only the immutable catalog revision may be requested.
- Treat `pass`, `intentional-difference`, `fail`, and `blocked` as distinct evidence states.
- Do not claim the full feature/platform/MSRV matrix, full provider coverage, or six-gate acceptance from this slice.
- Preserve all existing evidence files; write new evidence files with unique paths.
- Use the shared Cargo target/cache; do not run `cargo clean` or delete caches.
- Do not commit or publish changes without an explicit user request.

---

### Task 1: Add the acceptance-material freezer

**Files:**
- Create: `docs/iterations/opi-ai-1-5-alignment/tools/freeze_acceptance_materials.py`
- Create: `docs/iterations/opi-ai-1-5-alignment/evidence/phase5-acceptance-materials.json`
- Test: `docs/iterations/opi-ai-1-5-alignment/tools/test_acceptance_materials.py`

**Interfaces:**
- Consumes: repository root, `.pi/pi-1.5.0/nix/model-catalog.json`, upstream source files, `package.json`, `package-lock.json`, workspace `Cargo.toml`/`Cargo.lock`.
- Produces: schema-versioned JSON with `inputs`, `runtime`, `catalog`, `blocked_gates`, and exact command metadata.

- [x] **Step 1: Write the failing freezer tests**

  Test a temporary mini-repository containing a valid pinned revision and a
  small input file. Assert that the freezer records SHA-256, byte length and
  `present` state. Add a second test with a malformed pin and assert a
  non-zero exit plus a `blocked` catalog gate. Use only `tempfile`, `json`,
  `hashlib`, `unittest`, and `subprocess` from the standard library.

- [x] **Step 2: Run the focused tests and verify failure**

  Run:

  ```powershell
  python -B -m unittest docs/iterations/opi-ai-1-5-alignment/tools/test_acceptance_materials.py -v
  ```

  Expected: FAIL because the freezer module and CLI do not exist.

- [x] **Step 3: Implement the freezer**

  Implement a CLI with `--repo`, `--output`, and optional catalog-output path.
  Hash files as raw bytes, report missing files without throwing, parse the
  catalog pin as JSON, and record `node`, `rustc`, `cargo`, platform, arch,
  Git HEAD and `git status --short --branch`. For the catalog request, use
  Python `urllib.request` against
  `https://pi.dev/api/models/revisions/<revision>?types=chat,image,classifier`;
  stream bytes into memory, verify `sha256-<hex>` against the declared revision,
  and save the exact response bytes as a separately hashed evidence artifact.
  Return exit 0 for a valid inventory even when the catalog is unavailable,
  with that gate marked `blocked`.

- [x] **Step 4: Run the focused tests and verify pass**

  Run the same unittest command. Expected: all freezer tests PASS.

- [x] **Step 5: Run the freezer against the real snapshot**

  ```powershell
  python -B docs/iterations/opi-ai-1-5-alignment/tools/freeze_acceptance_materials.py `
    --repo D:/Luiz/Odradek/opi `
    --output D:/Luiz/Odradek/opi/docs/iterations/opi-ai-1-5-alignment/evidence/phase5-acceptance-materials.json
  ```

  Expected: exit 0; all local identity hashes are recorded; catalog status is
  `available` only if the response hash matches, otherwise `blocked` with the
  exact transport/HTTP/hash reason.

### Task 2: Add the Rust lifecycle trace driver

**Files:**
- Create: `crates/opi-ai/examples/lifecycle_differential.rs`
- Create: `scripts/pi-sdk-conformance/differential/README.md`

**Interfaces:**
- Consumes: public `Models`, `ChatCall`, `StartedChatStream`, `Provider`, `Request`, `ChatModelRecord`, and hidden-but-packaged deterministic `test_support` helpers.
- Produces: one JSON object per scenario on stdout with `schemaVersion`, `runtime: "rust"`, `scenario`, `steps`, `shared`, and `runtime` fields.

- [x] **Step 1: Write the driver contract test**

  Add a Python test that invokes `cargo run -p opi-ai --example lifecycle_differential --offline --`
  for `lazy-no-consumption`, `lazy-stop-consumption`, and `lazy-setup-failure`.
  Assert valid JSON, the three scenario IDs, monotonic step order, and a
  terminal observation for each case. The test should fail until the example
  exists.

- [x] **Step 2: Implement the controlled provider and trace cases**

  The example must construct a `Models` collection with
  `test_support::single_route_collection` and a custom `Provider`. The provider
  owns one `ModelInfo`, increments a setup counter in `stream_prepared`, and
  returns a deterministic stream that signals source completion through a
  `oneshot` channel. The no-consumption case constructs `stream_simple`, drops
  the `ChatCall`, and waits for source completion. The stop-consumption case
  receives one event, drops the handle, and waits for source completion. The
  setup-failure case uses an unregistered record and observes the one typed
  `CallFailure`. Bound every wait with a five-second Tokio timeout and serialize
  only deterministic booleans, counts, stop reasons and error categories.

- [x] **Step 3: Run the example contract test**

  ```powershell
  python -B -m unittest scripts/pi-sdk-conformance/differential/test_rust_driver.py -v
  ```

  Expected: all three scenarios emit parseable JSON and complete within the
  watchdog.

### Task 3: Normalize the TypeScript oracle and compare traces

**Files:**
- Modify: `docs/iterations/opi-ai-1-5-alignment/tools/observe_upstream.mjs`
- Create: `scripts/pi-sdk-conformance/differential/compare_lifecycle.py`
- Create: `scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py`
- Create: `scripts/pi-sdk-conformance/differential/test_rust_driver.py`
- Create: `docs/iterations/opi-ai-1-5-alignment/evidence/phase5-lifecycle-differential.json`

**Interfaces:**
- Consumes: upstream probe JSON, Rust driver JSON, source/driver hashes, and the material-freeze evidence.
- Produces: normalized traces and comparison rows classified as `pass`, `intentional-difference`, `fail`, or `blocked`.

- [x] **Step 1: Add the failing comparator tests**

  Create fixtures for one matching pair, one documented startup-timing
  difference, and one unexpected terminal mismatch. Assert the comparator
  returns the expected classification and never treats a mismatch as pass.

- [x] **Step 2: Inspect the oracle and implement normalization**

  Keep the existing upstream assertions intact. The existing probe already
  emits a stable top-level schema and the selected case IDs, so no source edit
  was required. Normalize shared fields to
  `setup_started`, `source_started`, `source_completed`, `consumer_polled`,
  `consumer_dropped`, `terminal_kind`, and `error_category`. Compare only those
  fields; retain raw runtime-specific details under `runtime`.

- [x] **Step 3: Implement the runner**

  The runner executes the Node oracle against `.pi/pi-1.5.0`, runs the Rust
  example once per scenario, loads the material-freeze evidence, and writes
  `phase5-lifecycle-differential.json` with command lines, SHA-256 hashes,
  traces, comparisons, and `blocked_gates`. If Node, Cargo, or required source
  files are missing, emit `blocked` evidence and exit 0 only for an inventory
  report; an unexpected driver failure exits non-zero.

- [x] **Step 4: Run comparator tests and verify pass**

  ```powershell
  python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p "test_*.py" -v
  ```

  Expected: all fixture and driver contract tests PASS.

- [x] **Step 5: Run the full differential slice**

  ```powershell
  python -B scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py `
    --repo D:/Luiz/Odradek/opi `
    --output D:/Luiz/Odradek/opi/docs/iterations/opi-ai-1-5-alignment/evidence/phase5-lifecycle-differential.json
  ```

  Expected: the evidence contains three rows. Shared lifecycle outcomes either
  pass or are explicitly classified as intentional differences; no unexpected
  mismatch is hidden.

### Task 4: Update iteration evidence and verify the slice

**Files:**
- Create: `docs/iterations/opi-ai-1-5-alignment/phase5-materials-and-differential.md`
- Modify: `docs/iterations/opi-ai-1-5-alignment/spec.md`

**Interfaces:**
- Consumes: the two new evidence JSON files and command logs.
- Produces: a concise report linking exact evidence, stating blockers and
  leaving the full-alignment status active.

- [x] **Step 1: Write the report from evidence**

  Record the frozen identities, catalog availability, three scenario results,
  intentional differences, and limits. Do not edit historical phase reports.

- [x] **Step 2: Update the spec's current-status paragraph**

  Add one paragraph linking the new report and preserve the distinction between
  this differential slice and complete provider/feature/TS-Rust alignment.

- [x] **Step 3: Run repository checks**

  ```powershell
  python -B -m unittest discover -s docs/iterations/opi-ai-1-5-alignment/tools -p "test_*.py" -v
  python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p "test_*.py" -v
  python scripts/opi-doc-check.py
  git diff --check
  cargo fmt --all --check
  cargo test -p opi-ai --example lifecycle_differential --offline -j4
  ```

  Expected: focused tests and documentation/format checks PASS. If the full
  Rust workspace retains the two known unrelated failures, report them as
  existing blockers rather than changing their classification in this slice.
