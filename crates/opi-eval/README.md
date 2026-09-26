# opi-eval

> Freeze one evaluation contract, run comparable Agent harness trials, and
> re-verify the sealed evidence offline.

[简体中文](README.zh.md) | [opi workspace](../../README.md)

`opi-eval` compares multiple Agent harnesses under the same explicit model,
environment, task, and trial controls. It resolves those inputs before any
Agent starts, records each trial in a sealed bundle, and derives scores only
from evidence that still verifies. Reports also retain bound unsealed attempts
as unverified diagnostics, without assigning them scores.

The crate is an unpublished, Agent-neutral workspace companion. It is not part
of the `opi` runtime and does not change ordinary `opi` behavior.

## When to use it

Use `opi-eval` when you need to:

- validate that an experiment is complete and digest-addressed before running;
- compare baseline and candidate harnesses under shared controls;
- exercise an Agent or benchmark adapter against pinned conformance fixtures;
- re-verify sealed trial evidence without rerunning an Agent or provider;
- produce a normalized report with verified results and explicit invalid or
  unexecuted coverage.

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

A cold selected Opi build requires paired `--opi-source /local/git/repository`
and `--opi-commit <full-40-hex>` arguments. Preparation exports only that local
commit, verifies its locked Cargo input and builds from the export; it never
infers HEAD, includes uncommitted files or fetches a missing Opi commit.
Verified warm packages need no source arguments, and pi-only preparation
needs no Opi build. One invocation acquires the selected clients, complete
pinned corpus and locked Harbor/Pier environment. Seed both the materializer
and `build-agent-artifacts.sh` from the pin set. Agent runtime resources and
build identities remain in the verified package.

Pi model data is fetched from `subjects[id=pi].model_data_archive` in the
static lock: package and version must match the source, and the exact archive
URL and SHA-256 are checked before extraction or offline building. Upstream
shrinkwrap SRI is not required. Direct pi builder calls require
`--static-lock /absolute/linux-x86_64.json`; local prepare and native-smoke
pass their declared lock explicitly. Opi-only builds need no pi archive pin.

The local pi package includes the pinned bundle's external chord, photon-node
and jiti packages, plus chord's esbuild and Linux x64 binary. They come from
the locked offline build; installation hooks stay disabled. Preparation checks
package identities and rejects escaping links or missing runtime files, then
runs bounded standalone version, import and native esbuild checks in an isolated
configuration before installing the package in the cache.

The cache holds each complete, clean pinned Git corpus at
`corpora/<adapter>/tasks`. Preparation creates the selected tasks' packages at
`packages/<adapter>/<task-id>` and SHA-256-bound profiles at
`profiles/<adapter>/<task-id>.toml`. Material/5 records the full corpus
root/commit/tasks-tree separately from its selected `tasks` map; each entry
binds its canonical ID, native name, profile and package. Task images and
Dockerfile bases must resolve to immutable digests. With the pinned materializer
seeded, preparation pulls missing selected images by their admitted digest and
verifies them in the same selected engine before freezing. Missing or invalid
pins reject before pulling. Warm preparation reuses verified images. Preview
remains read-only, and preparation never builds images or starts containers.

Corpus metadata links directly beneath `tasks/` may refer to a regular tracked
file in the same pinned commit, such as `tasks/README.md -> ../README.md`.
Preparation verifies the exact link text and target bytes/mode before writing
the inventory; the corpus digest hashes link text. These inert entries are
never tasks or task-package content. Escapes, chains, linked ancestors, links
inside task directories, submodules and special files reject. Runtime and task
packages retain their no-link rule. Relative configuration/cache paths keep
their configured location and produce absolute frozen package identities.

Requirements retain separate Agent and verifier CPU, memory, storage, GPU
count/types, network policy, collection and stage deadlines, including their
pinned defaults and origins. CPU and memory come from the selected Docker
engine. Storage checks cover the cache, output and engine storage namespace.
On Linux/WSL, an already running Docker Desktop can expose storage through a
bounded read-only `wsl.exe` observation when the selected local Unix engine ID
matches exactly one dockerd namespace. The sample binds the engine root,
process and mount identity and rechecks the selected endpoint; it starts no
distribution or container. Remote engines, ambiguous identities, denied access
and failed or timed-out observations remain unknown. An NVIDIA runtime registration does not establish GPU
capacity. A required unknown rejects selection before jobs or models start.
The full inventory and exclusion reasons remain visible for an explicit subset.

Material/4 and run-plan/1 must be regenerated; no implicit conversion is
performed. Existing sealed bundles remain readable without rewriting them.

### 0b. Preview and freeze a selection plan

```sh
cargo run -p opi-eval -- plan --config experiment.toml   --native-material "$DEMO_DIR/material/material.json"   --out "$DEMO_DIR/plan.json"
cargo run -p opi-eval -- run --plan "$DEMO_DIR/plan.json" --root "$DEMO_DIR/run"
```

`run` requires exactly one of `--config` or `--plan`. A native plan reuses its
recorded material; an explicit `--native-material` must match the frozen
manifest digest. Native execution needs no `--fixtures`; hermetic execution
still requires it. Missing or changed plan/material inputs reject before any
run effects.

`plan` enumerates the pinned corpus task inventory from `task.toml`
metadata (no image is pulled or started), expands the document's
`[selection]` (`scope=full|subset`, explicit `task_ids`, recorded
`reason`, `repetitions`, optional per-task `seeds`) into exact trials,
and checks every selected task's declared resource requirements against
the locally observed resources: an unmet or unobservable requirement
rejects the whole request - the set never silently shrinks - while the
preview keeps the full inventory's per-task statuses visible. The
`opi-eval-plan-preview/2` JSON shows subjects, connection, task list, selection, trial
count, concurrency, bounds, and the unmet/unknown preflight summary;
the frozen `opi-eval-run-plan/2` file is digest-bound over the embedded
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
byte-stable normalized report with scores from verified sealed evidence; and
`regrade --root R --native-material M --out D` is the distinct actual
native regrading described below - a byte check never runs under the
regrade name. Every run binds a run-level
`run-control.json` before its first effect (experiment identity,
manifest digest, selection, corpus size, exact planned trials), so the
offline report preserves unstarted trials even when no trial sealed.
The report consumes per-trial settlement records - never stderr tokens or
process exits - for its outcome classes. Unsealed records must agree with the
experiment and planned-trial bindings; they preserve attempted/invalid or
effect-unknown coverage only, never verified status, native facts, or scores.
Missing seals therefore remain visible even when all attempts are unsealed.
Only sealed, verified graded trials
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
reconstruction over identical inputs yields identical bytes. Tampered sealed
evidence and contradictory control records block publication. Unsealed
attempts stay unverified and outside the metric denominator. Native completion
diagnostics recognize verified native grades, and each native headline points
to the exact sealed trial result that supplied its reward.

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
verifier in fresh trial-owned environments under `--out`. The private entry
uses explicit `uv run --locked --offline python -m <module>` arguments and
the pinned native Task, ArtifactHandler and Verifier APIs. It emits an
`opi-eval-native-regrade/2` report and an `opi-eval-regrade-record/2` record
per successful trial, binding the source bundle, original result, restored
inputs, task/package, runner/module, image/build context and fresh native
result digest without rewriting any original score. No Agent executable is
spawned, no credential is resolved, and no provider is contacted: a task whose
verifier declares a network requirement without a permitted offline equivalent
rejects fail-closed, as do tampered bundles, missing or altered Agent
artifacts, a drifted patch base, and drifted task material identity. Exit 0
(`"outcome":"regraded"`) means every sealed native trial regraded; exit 1
names the typed per-trial rejections; exit 2 rejects the request itself (a
root with no native trial, an unreadable material, or an existing output
root, which a regrade never overwrites). Only declared, digest-bound preserved
inputs are restored; missing shared final state cannot be replaced by a
pristine task. Images must already be cached: this operation never builds or
pulls. Cancellation and finite process bounds settle owned effects before
continuing; unconfirmed cleanup blocks success and further dispatch.

### 4. Produce a report

```sh
cargo run -p opi-eval -- report \
  --root "$RUN_ROOT" \
  --out "$REPORT_PATH"
```

`report` re-verifies the sealed inputs, then renders one
`opi-eval-normalized-report/2` JSON report (`opi-eval-reporter/2`). It prints the report to stdout and,
when `--out` is present, writes the same canonical bytes to that path.

The binary task outcome remains 0 or 1: TB2.1 reports native task success,
TB3 reports task success after declared artifact collection, and DeepSWE
reports the resolved task outcome after committed-patch verification. Each
metric names its benchmark revision and counts only valid graded trials;
zero scores and benchmark-defined Agent timeouts remain in that denominator.

Per-trial `native_grader.test_case_fraction` separately reports the native
`passed / total`, with numerator, denominator and the raw result digest. Partial
test success may accompany task reward zero. Missing counters, zero totals and
invalid counters remain explicit unknowns. `native_fields` preserves additional
reward fields under their original names without interpreting abbreviations.
`observation_coverage` counts eligible valid trials, measured fractions and
unknowns. Invalid trials, corpus/subset coverage, planned/attempted/unexecuted
trials and pair coverage remain separate; fractions are not pooled across
trials or benchmark families. Offline reconstruction reads existing sealed
bundles without rewriting them; absent native observations stay unknown.

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
| Native-material mode | Add `--native-material <MANIFEST>` to `validate`, `run`, or `conformance`. The manifest (schema `opi-eval-native-material/5`) resolves exact Agent executables, the pinned integration modules that run each Agent inside the task environment (one per runner family: Harbor for the Terminal-Bench route, Pier for DeepSWE), task packages, verifier/oracle entry points, and the scripted-provider endpoint. | For `run`, one explicitly selected upstream job per trial runs the selected Agent inside the graded task environment and grades that same trial; the imported result must bind to the selected Agent, task, and job identity before anything seals. Terminal-Bench 3.0 runs through its declared separate-verifier handoff: the task's own `task.toml` lifecycle is parsed and bound (unsupported shapes, including multi-step tasks, disabled verifiers, verifier collect hooks on the Harbor route, and non-Linux tasks, reject before any verifier spawn), and for separate-verifier tasks only the declared artifacts cross into the fresh verifier - undeclared, missing, or substituted collections reject and an unrelated canary file stays outside. DeepSWE runs through its declared committed-patch handoff: the Pier task's collect hook extracts the committed patch beyond the task's base revision and the pristine no-network verifier grades exactly that patch (uncommitted work can never be substituted for it); the bound job must record the task's declared lifecycle together with the base revision and environment image, an empty nop patch still collects and grades zero, and undeclared collection rejects. The sealed bundle records the verifier-input identity (`native/verifier-input.json`: the environment mode, the DeepSWE base revision and image, plus every declared artifact with the digests of exactly the collected bytes). `conformance` restricts native mode to its registered native case subset (including the explicitly selected `oracle-witness`/`nop-witness` artifact witnesses for all three families). |

An Agent may carry a `runtime_package` with an absolute `path`, sorted-file
`manifest_sha256`, and relative `entrypoint` bound to its pinned executable.
The native handoff schema is `opi-eval-native-handoff/3`; Harbor and Pier
verify and upload the complete package before launching it with the selected
provider and model. Resource drift, links and entrypoint traversal reject.
For local preparation, `cache/agents/<product>` is either the executable or
a package directory containing the `<product>` executable and its resources.

Local material also binds a `runner_project` to the static runner commit,
lockfile and tracked source bytes. Loading rechecks those identities and the
absolute `UV_PROJECT`, `UV_PROJECT_ENVIRONMENT` and `UV_PYTHON` selections.
Preparation uses the same pinned uv executable
for explicit oracle diagnostics. Pier derives its inference allowlist from the
handoff's selected `inference_endpoint` and scopes the upstream proxy environment
to the Agent process; the separate verifier receives neither.
Opi task processes use an isolated unavailable session-bus address so the pinned
client's environment-credential fallback works in headless containers.

The upstream oracle is an explicitly selected conformance diagnostic
(`conformance --case oracle-witness` / `--case nop-witness`), never a
prerequisite silently run by `run`. Diagnostic jobs select the pinned runner's
registered `oracle` or `nop` name without a custom import path. Result binding
requires that exact name and rejects missing, substituted or dual selectors;
integration Agents retain their pinned import selection.

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

Native admission checks required `temperature` and `top_p` against every
selected client. Opi cannot express either control; pi supports them only on
the OpenAI protocols. Unsupported required values reject validation, planning
and execution; optional requested values remain non-comparable. Omitted and
unknown markers remain distinct and do not establish an enforced value.

Native Opi configuration omits `[defaults].max_iterations`. Current-tree Opi's
core has no intrinsic turn ceiling; its product enforces only explicitly
configured budgets. Native task/adapter deadlines and cancellation still apply.
Older executables may retain a 50-turn default, so record the exact observed
client behavior alongside deadlines in the preview; the machine plan has no
dedicated Agent-turn-budget field. Verify the selected executable with the
production projection using the offline long-loop test:

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=/absolute/clients.json cargo test -p opi-eval --test native_driver \
  native_client_turn_budgets -- --ignored --exact --nocapture
```

Supply a JSON object with absolute `opi` and `pi` executable paths. The loopback
test completes 51 tool turns and a final response with numeric output controls
for Opi's default and projected configurations and the pinned pi client, and checks that
an explicit small budget stops before another request. A custom Opi model with
an omitted output capability still rejects before provider dispatch; the
projection does not invent that capability. A failed check blocks preparation
for this behavior claim instead of silently replacing the selected executable.
Changed projection bytes require new control proof and native
material/plan identities; old candidates and sealed results remain historical.

Numeric output limits are projected into both the model capability and Opi's
explicit `[defaults].max_output_tokens` request setting. This does **not**
establish support in stock Opi 0.8.2, which can silently ignore that setting.
Native `validate`, `plan`, and `run` require exact-client loopback evidence
when `max_output_tokens` is required or an execution output-token bound is
declared. An optional numeric limit without evidence remains an unsupported
comparison control. A version string, declared capability, or low token usage
does not substitute for observed request limits.

Generate the narrowly scoped proof with the selected experiment and locally
supplied executables; the probe replaces only the endpoint and credential with
a loopback listener and literal synthetic authentication. It never resolves
the experiment's credential or contacts its provider. For example, `clients.json`:

```json
{
  "opi": "/absolute/opi",
  "pi": "/absolute/pi-package/pi",
  "pi_package": "/absolute/pi-package",
  "experiment": "/absolute/experiment.toml"
}
```

```sh
OPI_EVAL_NATIVE_CLIENT_MATERIAL=/absolute/clients.json \
OPI_EVAL_OUTPUT_CONTROL_OUT=/absolute/new-proof-directory \
cargo test -p opi-eval --test native_driver \
  native_output_control_client_matrix -- --ignored --exact
```

The output directory must not exist. Native pi is launched verbatim; the
existing `node`/`pi_bundle` input form is also supported. Supply `opi_package`
or `pi_package` whenever native material transports that client's runtime
package. This probe covers the experiment's selected API and numeric limit,
with reasoning omitted or unknown; it does not attest reasoning effort. Both
requests in the deterministic tool loop must carry the exact limit.

Before starting a listener, the probe binds the executed file to a normal
relative entrypoint inside the supplied package, rejecting traversal, links,
and unrelated packages even when executable bytes match. The proof retains
that entrypoint as well as the package digest. The `node`/`pi_bundle` form
requires `pi_package` to contain both the interpreter and bundle; an
interpreter hash alone cannot identify the pi code.

Each `<product>/output-control.json` pins the executable/package identity,
provider/API/model, frozen controls, projected config, and the independent
listener log. Retain the entire proof directory. In **new** native material,
set `agents.<product>.output_control_evidence` to
`{"path":"/absolute/proof/<product>/output-control.json","sha256":"<file SHA-256>"}`
before freezing the plan. The optional field does not grant support by itself:
admission verifies the pinned bytes, regenerates the production projection,
and checks both observed wire limits. Missing, substituted, unsupported, or
mismatched required evidence rejects before dispatch. No existing plan, run,
or seal is rewritten. Each admitted trial seals a self-contained snapshot as
`control/output-control.json`.

The CLI consumes a resolved native-material manifest; it does not improvise or
silently replace missing native inputs. Native materialization and fidelity
verification belong to the repository's
[native-smoke workflow](../../.github/workflows/opi-eval-native-smoke.yml).
Its materialization stage runs the same loopback producer with a required
4096-token limit before pinning material and freezing configs. Opi uses the
located build executable; pi transports the built Node interpreter, package
metadata, distribution and assets behind a relative launcher. The package and
entrypoint bound by the probe are the ones declared in trial material. Failed
or incomplete proof stops materialization; the artifact verifier still requires
a comparable pair. This stage may fetch locked Cargo test dependencies, but
the clients receive only isolated synthetic authentication and a loopback URL.
Real-provider behavior evaluations use the explicitly invoked
[opi-eval workflow](../../.agents/skills/opi-eval/SKILL.md), which manages
budget and evidence separately from the hermetic CLI examples.

## Command reference

| Command | Required inputs | Important options | Output and exit behavior |
|---------|-----------------|-------------------|--------------------------|
| `validate` | `--config PATH` | `--native-material PATH` adds native integrity identity. | Summary line; 0 resolved, 1 invalid. |
| `run` | Exactly one of `--config PATH` or `--plan PATH`, plus `--root PATH`; `--fixtures PATH` for hermetic execution. | `--recover`, `--replacement-for TRIAL`, `--canaries PATH`, `--native-material PATH` (must match a plan's frozen digest); `--behavior` selects hermetic fault fixtures. | One-line JSON; 0 completed, 1 settled non-success, 2 rejected request. |
| `verify` | `--root PATH` | None. | One-line JSON; 0 verified, 1 mutation/unsealed failure, 2 rejected command request. |
| `regrade` | `--root PATH --native-material PATH --out PATH` | None. | One-line JSON; 0 all native trials regraded, 1 typed per-trial rejections, 2 rejected command request. |
| `report` | `--root PATH` | `--out PATH`, `--canaries PATH`. | One-line JSON; 0 published, 1 blocked/unverified, 2 rejected command request. |
| `conformance` | `--suite ID --adapter ID --case ID --root PATH --fixtures PATH --provider PATH` | `--native-material PATH`. | One-line JSON; 0 expectation met, 1 expectation missed, 2 unsupported/rejected request. |

Use `cargo run -p opi-eval -- <command> --help` as the complete current flag
reference. The CLI and its formats remain unstable 0.x contracts.

## Outputs and failure recovery

Material acquisition uses a 3,600-second deadline for the whole invocation
and retains at most 1 MiB per output stream. `acquisition.log` records source
acquisition; `image-acquisition/acquisition.log` records selected-image acquisition.
Both record exit,
timeout/cancellation, truncation and owned cleanup. SIGINT/SIGTERM cancel the
owned process tree; failed or unverified settlement cannot publish material.
Only invocation-owned temporary staging is removed; verified cache entries remain.

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
or publication if it appears in exportable content. A native canary breach also
cancels the run before another trial starts; observed evidence remains private
and unsealed, and undispatched trials remain in planned coverage.
Offline comparisons require the sealed output-control snapshot for a requested
native output limit. Missing proof remains unsupported, including older bundles;
mutable run reports cannot upgrade comparability or supply missing proof.

| Symptom | What to do |
|---------|------------|
| Configuration is rejected | Run `validate` first and follow its typed stderr diagnostic. There is no implicit control default or fallback. |
| Run root already contains durable state | Do not delete or edit trial evidence. Use `run --recover` to classify durable states, or follow the explicit replacement flow for a crashed trial group. |
| Native material is rejected | Regenerate and resolve it through the native workflow; do not substitute unpinned executables or task packages. |
| `verify`/`regrade` reports `mutation-detected` or a typed rejection | Preserve the bundle and preserved artifacts for diagnosis. Do not repair, rewrite, or rehash them. |
| `report` is blocked | Inspect bundle-verification and canary failures, then use a new outside-root `--out` path after correcting the input source. |

`run --recover` prints a read-only classification of the frozen planned trials,
including unstarted trials without directories. It preserves every original
report, control and bundle byte, exposes retained settlement evidence, and
rejects inconsistent identities instead of relabelling them.

`run --replacement-for <TRIAL_ID>` creates fresh identities for the crashed
trial's whole comparison group using one new attempt token per invocation.
The original plan is verified and its selection expanded before replacement;
the new experiment retains its frozen scope, controls and unrelated groups.
Use a fresh output root; original plans and run evidence remain unchanged.

### Local operator acceptance and its offline verifier

The verifier requires sealed `execution.mode = "live"` and the first connection
`zhipu` / `anthropic-messages` / `https://open.bigmodel.cn/api/anthropic` /
`glm-5.3-flash`. Sealed terminal states, client streams, provider responses and
tool events must agree with that claim and the retained build/control evidence.
Loopback output-control probes establish client conformance only.

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

Acceptance requires Python 3.11 or later. Its manifest's
`controls.connection_controls` contains `connection` (the selected `provider`,
`api`, `base_url`, and `model`) and `model_controls` (the frozen resolved object).
The verifier compares these and `execution_concurrency` with every sealed
experiment, then checks the sealed output-control snapshot against the trial's
executable identity and preserved material. It validates the projected numeric
limit and both independently observed request limits; a dictionary-shaped
claim or a seal alone is insufficient. Original host paths inside the probe
record are never followed during acceptance.

### Bounded cancellation

Native Harbor/Pier jobs select the pinned integration's supervised Docker
environment for both Agent and verifier execution. Each command retains its
own process-group identity; stage cancellation waits for an observed command
exit before settling. Uncertain termination stops subsequent dispatch and
blocks sealing. The persisted trial configuration must retain that environment
selection and the effective stage caps. Teardown preserves shared images and
neighboring resources. Changed integration bytes require fresh material and
plan identities; existing source bundles are never rewritten.

Run the explicit no-model stage witness with prepared material/5:

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL=/absolute/material.json \
cargo test -p opi-eval --locked --test native_driver \
  remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
```

Prepare immutable task/helper images and separate-verifier build contexts before
timed dispatch. The three families each require all nine controlled cases for
both subjects; all positive controls run first. Harbor uses a 3-second positive
verifier cap and a 1-second timeout cap against a 4-second delayed command.
Pier times the entire verifier lifecycle and retries once: every case uses a
30-second verifier cap, a 60-second task verifier limit and, for verifier-timeout
cases, a 60-second delayed command. Agent caps stay at one second, with a
0.5-second stricter-task control. Each cancelled command needs an observed exit;
Pier verifier timeouts require two distinct command identities. Native stage
timestamps, command durations, absent delayed completion and owned-resource
cleanup remain separate evidence. Command settlement allows at most five extra
seconds for Pier; paired outer bounds are 270 seconds for Harbor and 510 for
Pier. Host wall-clock regressions remain diagnostic observations; elapsed checks
use monotonic clocks. These fixtures establish cancellation conformance, not real
acceptance.

The pinned Terminal-Bench 3.0 profile gives the whole Harbor job 18,000 seconds:
14,400 for the task's Agent, 900 for grading, two 600-second environment builds,
and 1,500 for remaining setup, collection and cleanup. This outer deadline is
separate from the task's native Agent and verifier deadlines. Profile changes
require new material and plan identities; existing frozen plans keep their
original budgets. A selected native job that reaches its outer deadline is an
ungraded `native-job-timeout` at the infrastructure boundary. Native verifier
timeouts remain grader-owned, and both retain `timed_out: true` in settlement
and receipt classification; benchmark-valid Agent timeouts retain their native
grade.

One run-level cancellation token governs every dispatch. Sending SIGINT or
SIGTERM to a running `run` stops new affected dispatch, cancels only the
owned process trees of the in-flight trials, removes the job containers
bound to each trial's private Compose working directories plus
the temporary credential material, and still settles and renders the
interrupted report: cancelled and never-dispatched trials stay explicit
missing coverage with no fabricated score, and the durable
`integration/owned-resources.json` evidence plus the receipt's cleanup token
name exactly what was cleaned. A successful cancellation never leaves orphan
execution behind; the ignored `native_owned_resource_cancellation` witness
exercises the real Docker removal locally. Unenforceable required bounds
reject before any effect, and a currency cap can never be promised from
unknown prices.

On the pinned mounted Docker routes, Agent streams and Opi traces are written
directly to the trial's log mount, so an outer timeout does not depend on a
final download. After owned writers are removed, full-capture runs retain a
private diagnostic copy limited to 1 MiB per file, 16 MiB total and 256 files,
with bounded traversal and explicit truncation counts. Root-owned capture is
made readable only within the exact daemon-observed log mount; permissions are
not widened. Links and hardlinked files cannot supply captured content. This
partial evidence never supplies a terminal event, native grade, or seal, and
the original timeout and cleanup failures remain visible.

Cleanup also runs after normal upstream exits, since an exit-zero job can
contain a failed setup or teardown. Docker labels and full container IDs must
match the recorded private roots before removal; an unconfirmed removal blocks
successful sealing. `opi-eval-owned-resources/2` records those identities and
outcomes. The temporary `handoff.json`, `config.toml`, and `models.json` are
removed on settlement, while `handoff-identity.json` retains non-secret launch
identity. Native result binding follows the pinned Harbor/Pier layout,
`jobs/<job>/<trial>/result.json`, including valid zero rewards.

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
