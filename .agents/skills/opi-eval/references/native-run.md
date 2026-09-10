# Native operator run

Use for `benchmark` and `acceptance`. The current sources of command and format
truth are [main.rs](../../../../crates/opi-eval/src/main.rs),
[experiment.rs](../../../../crates/opi-eval/src/experiment.rs),
[preparation](../../../../crates/opi-eval/src/runner/prepare.rs), and the
[local walkthrough](../../../../crates/opi-eval/examples/local-evaluation/README.md).
Check the selected executable's `--help` before assembling commands; the
Companion has no `--version` flag, so record its build/package identity.

## Platform and builds

Run native benchmarks inside WSL2/Linux x86_64 with the local Docker Linux
engine. Translate Windows paths once, record the Linux cwd, and use Linux
executables. Windows compilation or an installed Docker client does not prove
task feasibility. Keep evaluation storage outside the source checkout.

For Cargo builds, respect an existing `CARGO_TARGET_DIR`; otherwise use
`python scripts/opi-cargo-cache.py resolve`, start a resolver-managed lease for
the invoking process, and release it in `finally`/`trap`. Read the cache helper's
help for lease arguments. Keep incremental compilation enabled and shared
caches intact; inspect with `python scripts/opi-cargo-cache.py status`.

Build the Companion from the checkout, recording the compiler-artifact
executable path instead of assuming `target/release`:

```sh
cargo build --locked --release -p opi-eval --bin opi-eval --message-format=json-render-diagnostics
```

Record commit, dirty state, package version, Cargo.lock hash, toolchain, target,
features and executable SHA-256. If a build fails, stop that path; stale
binaries are not a substitute. A supplied Companion is usable after its
identity and supported CLI are verified.

For both subjects, inspect and use
[build-agent-artifacts.sh](../../../../crates/opi-eval/scripts/build-agent-artifacts.sh):

```sh
bash crates/opi-eval/scripts/build-agent-artifacts.sh --opi-source "$OPI_SOURCE" --pi-source "$PI_SOURCE" --out "$BUILD_OUT"
```

Run it with cwd equal to `OPI_SOURCE`: the script builds Opi from its cwd and
uses the argument to record provenance. The script requires both source paths,
emits identities and locator files, and does **not** install `cache/agents/*`.
For a single subject, use only that product's locked build/locator procedure;
do not run this two-product script or require an unselected Opi installation.
Compare requested versions with the source package and actual executable;
never rewrite the version field to make a mismatched build pass.

Stage the exact selected executable at `cache/agents/<product>` or the complete
verified standalone runtime package at that directory, with entrypoint
`cache/agents/<product>/<product>`. Verify its digest against build evidence.
`prepare` binds the complete package digest and relative entrypoint; both native
integrations verify and transfer those resources. Official pi standalone builds
need their adjacent package metadata and themes; a binary-only copy is incomplete.
Preserve pi's Node runtime, bundle, dependencies
and their identities as well as the actual staged executable hash: a Node hash
or bundle hash alone is not the launcher hash. Check that the pinned integration
can execute that artifact in the task image. Missing interpreters/libraries or
a launcher referring to host-only paths are preparation failures. Do not invent
a wrapper, change task images or broaden mounts to mask an unsupported build.

Seed the pinned lock/scripts/profiles/integrations exactly as the walkthrough
describes. Inspect [external locks](../../../../crates/opi-eval/external-locks/static/linux-x86_64.json)
and the selected [benchmark profiles](../../../../crates/opi-eval/profiles/benchmarks/).
Let `prepare` verify/acquire materials through its pinned materializer; retain
the acquisition plan, log and download identities. A missing runner/package/
image produces a named prerequisite, not a GitHub receipt or invented digest.

## Input to experiment mapping

Create one TOML document per family with a unique `experiment_id`. Use the
[experiment fixture](../../../../crates/opi-eval/tests/fixtures/experiment/local-paired.toml)
only to inspect syntax. Replace its synthetic identity, connection, controls,
subjects and selection; fixture digests/tasks are never live evidence.

| Skill input | Experiment / evidence |
|---|---|
| `software`, `version` | `[[subjects]]` with `id`, `product`, `version`; source/build identity recorded alongside. |
| Single software | One subject, no artificial `[[edges]]`. |
| Opi + pi comparison | `baseline-pi` -> `candidate-opi` edge; same task/repetition/connection/controls. |
| `bench` | `[benchmark].name`, `.revision`, `.dataset` from the selected admitted material. `all` expands to three documents. |
| `model`, `api`, `base_url` | Top-level `connection` selecting a `[[connections]]` id with `provider`, `api`, `base_url`, `model`. |
| Credential reference | Exactly one `credential_env` or `credential_file` on the connection; no inline value. Prefer an environment name to avoid personal paths in frozen config. |
| `tasks`, `scope`, `repetitions` | `[selection]` with `scope`, explicit `task_ids` for subset, `reason`, positive `repetitions`, optional per-task `seeds`. Remove `[[trials]]` when using selection. |
| `concurrency`, budget | `[execution]` with explicit `mode = "live"`, concurrency, `max_trials` and enforceable bounds. |
| `cache`, workspace/capture | `[storage].cache`, `.results`, `.capture_policy`; `[environment]` records Linux architecture and isolated cwd policy. |

All five `[model_controls]` keys are explicit: `temperature`, `top_p`,
`max_output_tokens`, `context_window`, `reasoning`, each a supported value or
an honest `omitted`/`unknown` marker. `required` lists controls that must be
enforced. Show adapter limitations before approval; an unsupported required
control blocks the run. Never infer a provider from its API protocol.

`max_trials` bounds plan expansion; `max_output_tokens` requires the equal
pinned model control and its supported client projection. Native trials remain
serial even with a larger concurrency ceiling. `max_total_tokens`, `max_cost`,
and monetary/usage declarations under `[execution.caps]` reject.

Check enforcement in the selected build, not just accepted config/help text.
In the current source, `time_limit_seconds`, `agent_timeout_seconds`,
`verifier_timeout_seconds` and `max_output_bytes` reach configuration/plan
preview but are not applied by native dispatch; it uses pinned adapter/task
limits. Do not promise those requested overrides. Record the actual pinned
limits and their execution-path evidence; a required smaller limit blocks live
execution until the user explicitly accepts the actual bounds or an admitted
fix supplies enforcement. Do not improvise an outer timeout that bypasses owned
cleanup. Wall time and per-response output limits are not total-spend
guarantees. Record estimates, provider usage, billed cost and unknowns
separately. Finish preparation before requesting revised unsupported bounds;
never silently omit the user's requirement.

## Commands and completion

In the independent directory, bind the following paths to resolved absolute
paths: `EVAL` (built Companion), `CONFIG`, `MATERIAL_DIR`, `PLAN`, `RUN_ROOT`,
`REPORT_A`, `REPORT_B`. Material/plan/run/report destinations must be fresh;
report paths live outside run roots. Save stdout/stderr and the actual exit of
each command separately; a logging pipe's exit is not the Companion's exit.

```sh
"$EVAL" doctor --config "$CONFIG"
"$EVAL" prepare --config "$CONFIG" --out "$MATERIAL_DIR"
"$EVAL" validate --config "$CONFIG" --native-material "$MATERIAL_DIR/material.json"
```

The native `validate` output prints `native_integrity=<digest>`; it does not
edit the config. Pin that exact derived value as `[benchmark].integrity_digest`
in the **unfrozen operator config**, replacing any synthetic fixture digest.
Save the derivation output, re-run native validation and confirm the pinned
value equals its newly derived digest. Only then freeze:

```sh
"$EVAL" validate --config "$CONFIG" --native-material "$MATERIAL_DIR/material.json"
"$EVAL" plan --config "$CONFIG" --native-material "$MATERIAL_DIR/material.json" --out "$PLAN"
```

Missing or mismatched integrity rejects `run` even if `plan` was written.
Do not patch a frozen plan to insert it; regenerate the plan from the corrected
config before approving live scope.

Check each exit before continuing and inspect doctor's observations even when
it exits zero. Read the plan itself: exact selected IDs, material identity,
feasibility, total trials and effective controls must match the authorized
request. A failed preflight remains an explicit missing prerequisite.

After live authorization, execute each family's plan:

```sh
mkdir -p "$UNUSED_FIXTURES"
"$EVAL" run --plan "$PLAN" --root "$RUN_ROOT" --fixtures "$UNUSED_FIXTURES"
"$EVAL" verify --root "$RUN_ROOT"
"$EVAL" report --root "$RUN_ROOT" --out "$REPORT_A"
"$EVAL" report --root "$RUN_ROOT" --out "$REPORT_B"
cmp "$REPORT_A" "$REPORT_B"
```

`UNUSED_FIXTURES` is an empty directory inside the independent workspace. The
current CLI parser still requires `run --fixtures`; the native plan supplies
the actual materials and does not consume repository fixtures. Supply native
material when **creating** the plan; `run --plan` loads its recorded material
and must not also receive `--native-material` (that argument's current branch
expects `--config`). Verify the plan actually carries native material before
launch; never drop it to obtain a fixture run.

When the operator supplies a private canary file, append `--canaries <path>`
to `run` and both `report` calls. This is a leak rejection gate, not a general
redaction exporter. Keep the file outside evidence and never print its contents.

A nonzero run exit still requires inspection of receipts, settlement and
coverage; run read-only verification where evidence exists and report failures.
Do not let a shell `set -e` erase the failed run's handoff. Conversely, failed
prepare/validate/plan blocks live execution entirely.

`--recover` launches no Agent but rewrites `run-report.json`. If classification
is needed beyond the preserved receipts/journal, first copy the complete run
tree to a fresh private `RECOVERY_ROOT`, leaving the original evidence intact:

```sh
"$EVAL" run --config "$CONFIG" --root "$RECOVERY_ROOT" --fixtures "$UNUSED_FIXTURES" --native-material "$MATERIAL_DIR/material.json" --recover
```

Keep that classification separate from the original native report. The current
`--replacement-for <trial-id>` flow resolves explicit `[[trials]]` before
selection expansion, so it cannot replace a trial from this `[selection]`
recipe; combining replacement with `--plan` also changes its frozen identity.
Report this limitation when a retry is needed. A new operator-approved
experiment must use fresh identities and retain the failed pair and replacement
relationship; do not rewrite the old plan, trial IDs or run-control to force
acceptance. Explicit-trial replacement and its family-root layout require their
own checked command scope before spending.

**Completion criterion**: the preserved command record and sealed settlements
account for every planned trial, and each published native report verifies and
reconstructs byte-identically. Platform, material, access and privacy failures
remain visible when that criterion cannot be met.
