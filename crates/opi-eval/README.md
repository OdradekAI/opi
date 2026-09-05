# opi-eval

> Freeze one evaluation contract, run comparable Agent harness trials, and
> re-verify the sealed evidence offline.

[简体中文](README.zh.md) | [opi workspace](../../README.md)

`opi-eval` compares multiple Agent harnesses under the same explicit model,
environment, task, and trial controls. It resolves those inputs before any
Agent starts, records each trial in a sealed bundle, and builds reports only
from evidence that still verifies.

The crate is an unpublished, Agent-neutral workspace companion. It is not part
of the `opi` runtime and does not change ordinary `opi` behavior.

## When to use it

Use `opi-eval` when you need to:

- validate that an experiment is complete and digest-addressed before running;
- compare baseline and candidate harnesses under shared controls;
- exercise an Agent or benchmark adapter against pinned conformance fixtures;
- re-verify sealed trial evidence without rerunning an Agent or provider;
- produce a normalized, conformance-only report from verified bundles.

The central terms are:

| Term | Meaning |
|------|---------|
| experiment | The frozen subjects, comparison edges, controls, environment, and declared trials. |
| subject | One Agent harness configuration, such as the baseline or candidate. |
| edge | A directed baseline-to-candidate comparison declared by the experiment. |
| trial | One subject running one task as part of a comparison group. |
| sealed bundle | The content-addressed evidence directory for one settled trial. Covered bytes become immutable after sealing. |

Use `opi` directly for ordinary coding-Agent work. The commands below are for
evaluation fixtures and evidence workflows, not general Agent execution.

## Prerequisites

- Run commands from the workspace root.
- Building the crate requires Rust 1.97 or newer.
- The repository checkout supplies the example experiment and fixtures.
- `validate` is cross-platform. The complete `run` -> `regrade` -> `report`
  and fixture-conformance walkthroughs documented here are backed by Unix-only
  acceptance tests that use POSIX helper processes.
- Hermetic fixture mode needs no live credentials or network access and never
  calls a paid provider.

`opi-eval` is unpublished, so invoke it through Cargo rather than installing it
from crates.io.

## Quick start: validate an experiment

This command works without starting an Agent:

```sh
cargo run -p opi-eval -- validate \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml
```

Success prints one summary line containing the experiment id, schema,
canonical manifest digest, and subject, edge, and trial counts. The example
resolves as `local-paired-hermetic` with two subjects, one edge, and two trials.

Exit code 0 means the contract resolved. Invalid or incomplete input exits 1
and writes a typed diagnostic to stderr. `validate` does not create a run root
or execute a trial.

## Complete fixture workflow (Unix)

The following walkthrough is hermetic and fixture-grade. Its helper processes
stand in for `opi`, `pi`, and the native verifier; it does not claim a real
Agent run, provider call, or official benchmark environment.

Choose a temporary directory. The run root and report file are deliberately
left uncreated:

```sh
DEMO_DIR="$(mktemp -d)"
RUN_ROOT="$DEMO_DIR/run"
REPORT_PATH="$DEMO_DIR/report.json"
```

### 1. Validate

```sh
cargo run -p opi-eval -- validate \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml
```

This freezes the experiment identity before any process effect. Continue only
after it exits 0.

### 2. Run

```sh
cargo run -p opi-eval -- run \
  --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml \
  --root "$RUN_ROOT" \
  --fixtures crates/opi-eval/tests/fixtures
```

`run` assembles both declared trials, records durable intent before process
effects, settles each trial, and seals its evidence. It prints one
`opi-eval-run-report/1` JSON object. Exit 0 and `"outcome":"completed"` mean
every declared pair settled comparably.

The run root must be fresh: use an absent or empty path that does not contain
an earlier durable run. The command writes `run-report.json` plus one receipt
and bundle under `trials/<trial-id>/`.

### 3. Regrade sealed bundles

```sh
cargo run -p opi-eval -- regrade --root "$RUN_ROOT"
```

`regrade` reads every `trials/<trial-id>/bundle`, recomputes its identity, and
checks every covered artifact. It does not start an Agent or provider, repair a
bundle, rehash changed bytes, or modify the run root.

Exit 0 and `"outcome":"verified"` mean every sealed bundle still matches its
manifest. A mutation or missing seal exits 1 and remains visible in the JSON
failure list.

### 4. Produce a report

```sh
cargo run -p opi-eval -- report \
  --root "$RUN_ROOT" \
  --out "$REPORT_PATH"
```

`report` re-verifies the sealed inputs, then renders one
`opi-eval-normalized-report/1` JSON report. It prints the report to stdout and,
when `--out` is present, writes the same canonical bytes to that path.

Exit 0 and `"outcome":"published"` mean publication succeeded. `REPORT_PATH`
must be outside `RUN_ROOT` and must not exist: report output never overwrites a
sealed input or earlier report.

## Run one conformance case

`conformance` exercises one registered Agent or benchmark adapter case through
the shared execution driver. This fixture example is Unix-only:

```sh
CONFORMANCE_BASE="$(mktemp -d)"
CONFORMANCE_ROOT="$CONFORMANCE_BASE/run"

cargo run -p opi-eval -- conformance \
  --suite agent \
  --adapter opi \
  --case completed \
  --root "$CONFORMANCE_ROOT" \
  --fixtures crates/opi-eval/tests/fixtures \
  --provider crates/opi-eval/scripts/scripted-provider.py
```

The command prints one `opi-eval-conformance-report/1` JSON object. Exit 0 and
`"met":true` mean the selected adapter met the pinned case expectation. Exit 1
means the case settled but missed that expectation; exit 2 means the selection
or command request was rejected.

Supported suites are `agent` and `benchmark`. Supported adapters are `opi`,
`pi`, `terminal-bench-2.1`, `terminal-bench-3.0`, and `deepswe`. Case ids are
defined by the pinned conformance matrices; an unsupported suite, adapter, or
case fails closed.

## Hermetic and native modes

| Mode | Inputs and behavior | What it proves |
|------|---------------------|----------------|
| Hermetic fixture mode | Default. Uses bounded deterministic helpers and pinned repository fixtures. | Adapter, lifecycle, sealing, reporting, and failure-path conformance. It does not prove real-product or real-provider fidelity. |
| Native-material mode | Add `--native-material <MANIFEST>` to `validate`, `run`, or `conformance`. The manifest resolves exact Agent executables, task packages, verifier/oracle entry points, and the scripted-provider endpoint. | The admitted native execution described by that resolved material identity. `conformance` restricts native mode to its registered native case subset. |

The CLI consumes a resolved native-material manifest; it does not improvise or
silently replace missing native inputs. Native materialization and fidelity
verification belong to the repository's
[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml).
Real-provider behavior evaluations use the explicitly invoked
[opi-eval workflow](../../.agents/skills/opi-eval/SKILL.md), which manages
budget and evidence separately from the hermetic CLI examples.

## Command reference

| Command | Required inputs | Important options | Output and exit behavior |
|---------|-----------------|-------------------|--------------------------|
| `validate` | `--config PATH` | `--native-material PATH` adds native integrity identity. | Summary line; 0 resolved, 1 invalid. |
| `run` | `--config PATH --root PATH --fixtures PATH` | `--recover`, `--replacement-for TRIAL`, `--canaries PATH`, `--native-material PATH`, `--preflight-only`; `--behavior` selects hermetic fault fixtures. | One-line JSON; 0 completed or successful preflight, 1 settled non-success, 2 rejected request. |
| `regrade` | `--root PATH` | None. | One-line JSON; 0 verified, 1 mutation/unsealed failure, 2 rejected command request. |
| `report` | `--root PATH` | `--out PATH`, `--canaries PATH`. | One-line JSON; 0 published, 1 blocked/unverified, 2 rejected command request. |
| `conformance` | `--suite ID --adapter ID --case ID --root PATH --fixtures PATH --provider PATH` | `--native-material PATH`. | One-line JSON; 0 expectation met, 1 expectation missed, 2 unsupported/rejected request. |

Use `cargo run -p opi-eval -- <command> --help` as the complete current flag
reference. The CLI and its formats remain unstable 0.x contracts.

## Outputs and failure recovery

A completed fixture run has this durable shape; additional staging files may
exist outside the sealed bundles:

```text
$RUN_ROOT/
├── run-report.json
└── trials/<trial-id>/
    ├── receipt.json
    └── bundle/
        ├── intent.json
        ├── manifest.json
        └── artifacts/
```

Treat a sealed bundle as immutable. Changing a covered byte causes `regrade`
and `report` verification to fail; neither command repairs or silently rehashes
the evidence. A canary listed one per line in `--canaries PATH` blocks sealing
or publication if it appears in exportable content.

| Symptom | What to do |
|---------|------------|
| Configuration is rejected | Run `validate` first and follow its typed stderr diagnostic. There is no implicit control default or fallback. |
| Run root already contains durable state | Do not delete or edit trial evidence. Use `run --recover` to classify durable states, or follow the explicit replacement flow for a crashed trial group. |
| Native material is rejected | Regenerate and resolve it through the native workflow; do not substitute unpinned executables or task packages. |
| `regrade` reports `mutation-detected` | Preserve the bundle for diagnosis. Do not repair, rewrite, or rehash it. |
| `report` is blocked | Inspect bundle-verification and canary failures, then use a new outside-root `--out` path after correcting the input source. |

`run --replacement-for <TRIAL_ID>` creates fresh identities for the crashed
trial's whole comparison group. It does not reuse the failed trial identity.

## Developer guide

| Area | Owning source |
|------|---------------|
| CLI parsing and exit mapping | [`src/main.rs`](src/main.rs) |
| Experiment assembly and durable lifecycle | [`src/runner/`](src/runner) |
| Agent and benchmark process adapters | [`src/agent/`](src/agent), [`src/benchmark/`](src/benchmark) |
| Admission and sealed evidence | [`src/integrity.rs`](src/integrity.rs), [`src/bundle/`](src/bundle) |
| Offline re-verification and normalized reports | [`src/regrade.rs`](src/regrade.rs), [`src/report.rs`](src/report.rs) |
| Hermetic inputs | [`tests/fixtures/`](tests/fixtures) |
| Assembled run behavior | [`tests/assembled_run.rs`](tests/assembled_run.rs) |
| Regrade/report lifecycle | [`tests/end_to_end_report.rs`](tests/end_to_end_report.rs) |
| Native material and fidelity checks | [`scripts/`](scripts), [native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml) |

Use local fixtures and deterministic providers in tests. Tests must not call a
paid provider or become live merely because credentials exist in the
environment. The Unix end-to-end suites are the acceptance source for the
complete fixture workflow shown above.

## Stability and product boundary

`opi-eval` is `publish = false`, depends on no Opi crate in any dependency
table, and remains an unpublished `0.x` workspace member. Its CLI, schemas,
on-disk formats, and library entry points may change deliberately; breaking
changes are recorded under [`CHANGELOG.md`](../../CHANGELOG.md)'s Unreleased
section. Older experiment or report formats imply no compatibility shim.

No Opi product links, registers, or activates this crate. It registers no
provider, tool, package, command, extension, startup hook, or default capture
path in `opi`. Existing sessions, native evidence, local Eval reports,
configuration, credentials, and user artifacts are not read, rewritten, or
migrated by this crate.

Current crate version: `0.8.2`, inherited from the workspace package version.
