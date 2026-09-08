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
- `validate` is cross-platform. The complete `run` -> `verify` -> `report`
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

### 0. Diagnose and prepare local materials

```sh
cargo run -p opi-eval -- doctor --config experiment.toml
cargo run -p opi-eval -- prepare --config experiment.toml --out "$DEMO_DIR/material"
```

`doctor` reports Linux-engine reachability, architecture, WSL/Docker-
observed CPU/memory/GPU/disk, and missing materials with reasons for
unknown observations, starting no model. `prepare` verifies the seeded
local cache (`[storage].cache`) against the pinned static lock, freezes
the selected benchmark, subject executables, and every requested image
identity, checks the frozen task's resource requirements against the
local observations (an unmet or unobservable requirement rejects the
whole request), and emits a local-origin native-material manifest
consumable by `validate` and `run` with `--native-material`. Local
origin is recorded truthfully and never manufactures GitHub provenance:
the upload receipt stays a CI transport claim. A cache miss triggers
supervised acquisition through the pinned materializer when it is
seeded (plan and captured log under the output root); otherwise the
failure names the missing identity and how to acquire it. The full
operator walkthrough lives under `examples/local-evaluation/`.

### 0b. Preview and freeze a selection plan

```sh
cargo run -p opi-eval -- plan --config experiment.toml   --native-material "$DEMO_DIR/material/material.json"   --out "$DEMO_DIR/plan.json"
cargo run -p opi-eval -- run --plan "$DEMO_DIR/plan.json"   --root "$DEMO_DIR/run" --native-material "$DEMO_DIR/material/material.json"
```

`plan` enumerates the pinned corpus task inventory from `task.toml`
metadata (no image is pulled or started), expands the document's
`[selection]` (`scope=full|subset`, explicit `task_ids`, recorded
`reason`, `repetitions`, optional per-task `seeds`) into exact trials,
and checks every selected task's declared resource requirements against
the locally observed resources: an unmet or unobservable requirement
rejects the whole request - the set never silently shrinks - while the
preview keeps the full inventory's per-task statuses visible. The
preview JSON shows subjects, connection, task list, selection, trial
count, concurrency, bounds, and the unmet/unknown preflight summary;
the frozen `opi-eval-run-plan/1` file is digest-bound over the embedded
experiment source, material identity, corpus entries, and exact trials.
`[execution]` bounds are explicit: `concurrency` (default 1, an upper
bound; hermetic trials run in parallel waves up to it, native trials
stay serial), `max_trials` (a larger expansion rejects),
`agent_timeout_seconds`/`verifier_timeout_seconds` (combine by
narrowing), and `max_output_tokens` - enforceable only through the
pinned model control of the same value. `max_total_tokens` and
`max_cost` have no enforcement route in this build, so declaring either
rejects with that reason (never silently ignored). `run --plan`
re-verifies the plan digest, the material bytes, the experiment
identity, and the corpus expansion, then executes exactly the frozen
trials with stage/settlement progress on stderr and a saved structured
result; a single-subject (pi-only or opi-only) experiment needs no
artificial comparison edge. `run --config C` remains the same
resolve/freeze path - a `[selection]` document expands against the same
enumerated corpus at run time.

### 0c. Verify seals and reconstruct the report offline

```sh
cargo run -p opi-eval -- verify --root "$DEMO_DIR/run"
cargo run -p opi-eval -- report --root "$DEMO_DIR/run" --out "$DEMO_DIR/report.json"
```

Three distinct offline operations, none starting an Agent or provider:
`verify --root R` re-checks every sealed trial bundle against its
original seals (exit 0 verified / 1 mutation detected / 2 rejected; no
repair, no rewrite); `report --root R --out O` reconstructs the
byte-stable normalized report from verified sealed evidence alone; and
`regrade --root R --native-material M --out D` is the distinct actual
native regrading described below - a byte check never runs under the
regrade name. Every run binds a run-level
`run-control.json` before its first effect (experiment identity,
manifest digest, selection, corpus size, exact planned trials), so the
offline report preserves unstarted trials even when no trial sealed.
The report consumes the sealed per-trial settlement record - never
stderr tokens or process exits - for its outcome classes: graded trials
(including valid native zero and benchmark-defined Agent timeouts) form
the metric denominator, counted per benchmark with its own metric
definition (never a cross-benchmark average), while
provider/access/identity/infrastructure/invalid-grading failures stay
attempted-but-invalid and effect-unknown trials stay unexecuted.
Coverage counts (`corpus`, `selected`, `planned`, `attempted`,
`valid`, `agent_failure`, `invalid`, `unexecuted`), the
`full|subset` scope label beside the completed/incomplete outcome,
same-task/repetition comparison pairs with missing sides and
exclusions, repetition control pairs (`consistent|divergent|incomplete`
with missing groups), and unknown telemetry reasons all stay visible: a
protocol-group edge (two subjects of one product) never pairs, and a
single-subject report needs no artificial comparison edge. Repeated
reconstruction over identical sealed inputs yields identical bytes;
tampered or unsealed evidence blocks publication instead of upgrading.

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

### 3. Verify sealed bundles, or re-grade them natively offline

```sh
cargo run -p opi-eval -- verify --root "$RUN_ROOT"
cargo run -p opi-eval -- regrade \
  --root "$RUN_ROOT" \
  --native-material "$MATERIAL" \
  --out "$REGRADE_OUT"
```

`verify` reads every `trials/<trial-id>/bundle`, recomputes its identity, and
checks every covered artifact. It does not start an Agent or provider, repair a
bundle, rehash changed bytes, or modify the run root. Exit 0 and
`"outcome":"verified"` mean every sealed bundle still matches its manifest; a
mutation or missing seal exits 1 and remains visible in the JSON failure list.

`regrade` is the distinct actual native regrading (`P19-AC-009`): it verifies
every source bundle first, requires the saved real Agent artifacts under each
trial's preserved job tree to still digest to the sealed verifier-input
identity, then re-grades those preserved artifacts with the pinned native
verifier in fresh trial-owned environments under `--out` - reusing the same
binding readers as initial execution - and persists one derived
`opi-eval-regrade-record/1` grading record per trial that references the
source bundle, the original artifact identities, and the fresh verifier
identity, without rewriting any original score. No Agent executable is
spawned, no credential is resolved, and no provider is contacted: a task whose
verifier declares a network requirement without a permitted offline equivalent
rejects fail-closed, as do tampered bundles, missing or altered Agent
artifacts, a drifted patch base, and drifted task material identity. Exit 0
(`"outcome":"regraded"`) means every sealed native trial regraded; exit 1
names the typed per-trial rejections; exit 2 rejects the request itself (a
root with no native trial, an unreadable material, or an existing output
root, which a regrade never overwrites).

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
| Native-material mode | Add `--native-material <MANIFEST>` to `validate`, `run`, or `conformance`. The manifest (schema `opi-eval-native-material/3`) resolves exact Agent executables, the pinned integration modules that run each Agent inside the task environment (one per runner family: Harbor for the Terminal-Bench route, Pier for DeepSWE), task packages, verifier/oracle entry points, and the scripted-provider endpoint. | For `run`, one explicitly selected upstream job per trial runs the selected Agent inside the graded task environment and grades that same trial; the imported result must bind to the selected Agent, task, and job identity before anything seals. Terminal-Bench 3.0 runs through its declared separate-verifier handoff: the task's own `task.toml` lifecycle is parsed and bound (unsupported shapes, including multi-step tasks, disabled verifiers, verifier collect hooks on the Harbor route, and non-Linux tasks, reject before any verifier spawn), and for separate-verifier tasks only the declared artifacts cross into the fresh verifier - undeclared, missing, or substituted collections reject and an unrelated canary file stays outside. DeepSWE runs through its declared committed-patch handoff: the Pier task's collect hook extracts the committed patch beyond the task's base revision and the pristine no-network verifier grades exactly that patch (uncommitted work can never be substituted for it); the bound job must record the task's declared lifecycle together with the base revision and environment image, an empty nop patch still collects and grades zero, and undeclared collection rejects. The sealed bundle records the verifier-input identity (`native/verifier-input.json`: the environment mode, the DeepSWE base revision and image, plus every declared artifact with the digests of exactly the collected bytes). `conformance` restricts native mode to its registered native case subset (including the explicitly selected `oracle-witness`/`nop-witness` artifact witnesses for all three families). |

The upstream oracle is an explicitly selected conformance diagnostic
(`conformance --case oracle-witness` / `--case nop-witness`), never a
prerequisite silently run by `run`.

Request identity comes from the experiment document's selected named
connection (schema `opi-eval-experiment/2`): the top-level `connection`
selects one `[[connections]]` id carrying `provider`,
`api` (`anthropic-messages` | `openai-completions` | `openai-responses`),
`base_url`, `model`, and exactly one `credential_env`/`credential_file`
reference. There is no provider, protocol, model, or billing-product
fallback: the credential resolves only inside the supervised Agent launch,
absent credentials and required-but-unsupported `[model_controls]` reject
before any model effect, and remote base URLs require the explicit
`[execution].mode = "live"` (loopback routes only otherwise). Requested
monetary/usage caps in `[execution.caps]` are rejected - the native route
cannot enforce them - while the enforceable `time_limit_seconds` and
`max_output_bytes` bounds are recorded with their limits. Both pinned
clients (Opi and pi) are projected from the same connection for all three
protocols by the local
[scripted provider](scripts/scripted-provider.py) (deterministic tool
loops plus model-keyed `auth-fail`, `rate-limit`, `retry`, and
`protocol-error` scripts) through the ignored
`native_protocol_client_matrix` filter with locally supplied client
material. Each adapter classifies typed terminal provider causes
(authentication failure, exhausted rate limiting, protocol error) from the
client's own evidence - including a JSON error at process exit zero - and
never speculates a provider cause the evidence leaves unknown.

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
| `run` | `--config PATH --root PATH --fixtures PATH` | `--recover`, `--replacement-for TRIAL`, `--canaries PATH`, `--native-material PATH`; `--behavior` selects hermetic fault fixtures. | One-line JSON; 0 completed, 1 settled non-success, 2 rejected request. |
| `verify` | `--root PATH` | None. | One-line JSON; 0 verified, 1 mutation/unsealed failure, 2 rejected command request. |
| `regrade` | `--root PATH --native-material PATH --out PATH` | None. | One-line JSON; 0 all native trials regraded, 1 typed per-trial rejections, 2 rejected command request. |
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

Treat a sealed bundle as immutable. Changing a covered byte causes `verify`,
`regrade`, and `report` verification to fail; none of them repairs or silently
rehashes the evidence. A canary listed one per line in `--canaries PATH` blocks sealing
or publication if it appears in exportable content.

| Symptom | What to do |
|---------|------------|
| Configuration is rejected | Run `validate` first and follow its typed stderr diagnostic. There is no implicit control default or fallback. |
| Run root already contains durable state | Do not delete or edit trial evidence. Use `run --recover` to classify durable states, or follow the explicit replacement flow for a crashed trial group. |
| Native material is rejected | Regenerate and resolve it through the native workflow; do not substitute unpinned executables or task packages. |
| `verify`/`regrade` reports `mutation-detected` or a typed rejection | Preserve the bundle and preserved artifacts for diagnosis. Do not repair, rewrite, or rehash them. |
| `report` is blocked | Inspect bundle-verification and canary failures, then use a new outside-root `--out` path after correcting the input source. |

`run --replacement-for <TRIAL_ID>` creates fresh identities for the crashed
trial's whole comparison group. It does not reuse the failed trial identity.

### Local operator acceptance and its offline verifier

Real acceptance is a separately authorized local operator run on
WSL/Linux, outside any development harness: the operator selects and
freezes the tasks, agent builds, material digests, model connection
controls, concurrency, and usage/time budget, connects the first real
credential explicitly, and runs the same public CLI. Default tests and CI
never become live because a credential happens to exist, and no provider
is contacted by any deterministic path. On a budget, authority,
provenance, or privacy breach, stop the affected trials, cancel owned
execution, and roll back by disabling the affected local execution path
or restoring a previously verified Companion build - never by relabeling
old evidence.

The operator hands over one redacted evidence root: the manifest with six
attributable trials across the three benchmark families, the frozen
controls, source/build/material/bundle digests, the preserved command
exits and provider/tool/native artifacts, the Opi-only
`opi-artifact-audit` scoped capture view (byte-bound to the three
original Opi native captures and exhaustive for them), and a cached
upstream repository-identity inventory for the pi/Harbor/Pier source
records. Anyone can verify that evidence offline:

```sh
python3 crates/opi-eval/scripts/verify-local-acceptance.py \
  --root <redacted-evidence-root> \
  --out <verification-report.json> \
  --companion <built opi-eval executable>
```

The verifier never launches an Agent, reads credentials, or contacts a
provider. It rejects substituted, oracle, missing, or fixture-only
evidence, refuses an empty generic-audit success, validates upstream
commits only against the supplied cached identities (never by resolving
commits in this checkout), and invokes the built Companion `verify` and
`report` only against the supplied family run roots. Exit 0 accepts the
evidence; exit 2 prints one typed rejection.

### Bounded cancellation

One run-level cancellation token governs every dispatch. Sending SIGINT or
SIGTERM to a running `run` stops new affected dispatch, cancels only the
owned process trees of the in-flight trials, removes the job containers
recorded under each trial's owned prefix (`opi-eval-owned-<trial-id>-`) plus
the temporary credential material, and still settles and renders the
interrupted report: cancelled and never-dispatched trials stay explicit
missing coverage with no fabricated score, and the durable
`integration/owned-resources.json` evidence plus the receipt's cleanup token
name exactly what was cleaned. A successful cancellation never leaves orphan
execution behind; the ignored `native_owned_resource_cancellation` witness
exercises the real Docker removal locally. Unenforceable required bounds
reject before any effect, and a currency cap can never be promised from
unknown prices.

## Developer guide

| Area | Owning source |
|------|---------------|
| CLI parsing and exit mapping | [`src/main.rs`](src/main.rs) |
| Experiment assembly and durable lifecycle | [`src/runner/`](src/runner) |
| Agent and benchmark process adapters | [`src/agent/`](src/agent), [`src/benchmark/`](src/benchmark) |
| Admission and sealed evidence | [`src/integrity.rs`](src/integrity.rs), [`src/bundle/`](src/bundle) |
| Offline byte verification, native regrading, and normalized reports | [`src/regrade.rs`](src/regrade.rs), [`src/report.rs`](src/report.rs) |
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
