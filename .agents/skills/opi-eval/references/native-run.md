# Native operator run

Use for `benchmark` and `acceptance`. Normal benchmark calls use the supplied
[operator](operator.md) and its explicit private profile; the operator owns
source/build adapters, material acquisition and frozen family plans. Inspect
its returned preparation handoff before dispatch. This reference defines the
native contracts needed to review that handoff or drive an explicitly selected
native/acceptance workflow. Check the selected executable's `--help`; the
Companion has no `--version` flag, so record its build/package identity.

## Platform and builds

Run native benchmarks inside WSL2/Linux x86_64 with the local Docker Linux
engine. Translate Windows paths once, record the Linux cwd, and use Linux
executables. Windows compilation or an installed Docker client does not prove
task feasibility. Keep evaluation storage outside the source checkout.

Use the runtime's subject adapters and build-cache coordination. Record the
Companion's package version, source/dirty identity, locked dependencies,
toolchain, target, features and executable digest separately from each subject.
Respect configured persistent Cargo caches and release only invocation-owned
leases; keep incremental compilation and reusable cache bytes intact. A failed
build stops preparation; stale binaries cannot substitute. A supplied Companion
is usable only after its identity and supported CLI are verified. Compare subject
versions with actual source/package/executable records rather than editing a
version field to make a mismatch pass. Project-specific developer build recipes
belong to the runtime's development documentation, outside the portable benchmark
instructions.

### Opi source selection

For each new evaluation without an explicit Opi version/source pin, build from
the current local repository working tree at preparation time, including
build-relevant uncommitted changes. Local means that checkout, without fetching
or switching to a release tag or remote branch. A prior run's executable is
historical evidence, not the default source for a new run. Explicit version
selection, including a pin in a user-supplied config, keeps its requested source.

Record the repository path, HEAD, package version, dirty diff and build-input
inventory, including relevant untracked files; exclude credentials and generated
caches. Run the locked build with the shared Cargo cache and bind its actual
artifact digest to that source snapshot. Recheck the inputs after building;
concurrent source changes require a fresh build identity before freezing.
Use the real package version in `[[subjects]].version` and label the subject as
a local working-tree candidate with its commit/dirty identity. Equal package
versions do not make a local candidate the corresponding release. Preserve the
snapshot evidence and binary used by every frozen trial. A repository that is
unavailable or cannot build is an explicit preparation gap.

### Pi source selection

pi has no working-tree default: a pi subject comes from the runtime's admitted
static-lock commit and blob identities. An explicit `version` must match that pinned
package; `pi:latest` is operator sugar that resolves at preparation to the
lock's current pi pin and records the resolved package version in
`[[subjects]].version` beside its build identity. The lock pin, not an upstream
tag, is the selection source of truth; bumping the lock creates a new pi
identity rather than silently upgrading a subject.

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
Use a per-operation frozen material view. Preserve the actual executable and
runtime bytes used by every run before staging another candidate; updating a
shared `agents/opi` alias would invalidate the old material's bound paths.

Use the supplied runtime's pinned lock/profiles/integrations.
Let `prepare` verify/acquire materials through its pinned materializer; retain
the acquisition plan, log and download identities. A missing runner/package/
image produces a named prerequisite, not a GitHub receipt or invented digest.

## Input to experiment mapping

Resolve the [evaluation defaults](../SKILL.md#evaluation-defaults) before
creating a config. Keep the whole-batch deadline and retry allowance in operator
evidence: the CLI has no corresponding automatic default or retry-budget field.
Unlimited spending is the absence of cumulative usage/cost caps, not an
unsupported `max_cost` value. Per-Agent stage limits retain their own meaning.

The operator creates one TOML document per family with a unique `experiment_id`
under `evaluations/YYYY/MM/<eval-id>/families/<bench>/`. Direct native requests
must use the selected runtime's documented experiment schema; fixture
digests/tasks are never live evidence. `bench=all` shares the selected families'
external record and one first-dispatch deadline, including grading and retries.

| Skill input | Experiment / evidence |
|---|---|
| `software`, `version` | `[[subjects]]` with `id`, `product`, `version`; source/build identity recorded alongside. |
| Single software | One subject, no artificial `[[edges]]`. |
| Opi + pi comparison | `baseline-pi` -> `candidate-opi` edge; same task/repetition/connection/controls. |
| `bench` | `[benchmark].name`, `.revision`, `.dataset` from the selected admitted material. `all` expands to three documents. |
| `model`, `api`, `base_url` | Top-level `connection` selecting a `[[connections]]` id with `provider`, `api`, `base_url`, `model`. |
| `max_output_tokens`, `context_window` | Equal-named `[model_controls]` values for every compared subject; explicit inputs override model-specific defaults. |
| Credential reference | Exactly one `credential_env` or `credential_file` on the connection; no inline value. Prefer an environment name to avoid personal paths in frozen config. |
| `tasks`, `scope`, `repetitions` | `[selection]` with `scope`, explicit `task_ids` for subset, `reason`, positive `repetitions`, optional per-task `seeds`. Remove `[[trials]]` when using selection. |
| `concurrency`, budget | `[execution]` with explicit `mode = "live"`, concurrency, `max_trials` and enforceable bounds. |
| `cache`, workspace/capture | `[storage].cache`, `.results`, `.capture_policy`; `[environment]` records Linux architecture and isolated cwd policy. |

### Model controls

Explicit user inputs override the resolved
[operator profile](operator.md#supplied-entrypoint-and-one-profile). The
supplied operator's maintained template is the single default source
for context/output controls; apply the same required controls to compared
subjects.
The [model documentation](https://docs.bigmodel.cn/cn/guide/models/vlm/glm-5.3-flash)
lists a 1M context window and 128K maximum output; the
[API reference](https://docs.bigmodel.cn/api-reference/模型-api/对话补全)
specifies `max_tokens <= 131072` (checked 2026-09-11). Recheck the selected
endpoint's current support before freezing; an incompatible required value is
a preparation gap rather than permission to silently lower the limit. Other
models resolve their own documented capabilities instead of inheriting these
numbers. Resolve sampling and thinking from the same profile, preserving honest
omitted/unknown markers for controls that were not selected. A required thinking
level needs actual-client proof on the first request and tool continuation;
profile acceptance alone does not establish support.

`max_output_tokens` limits each model response, including thinking, and resets
for the next provider request; it is not the task's cumulative token budget.
`context_window` describes the client's context capacity, not the response
budget or a measurement of server capacity. When an execution output bound is
present, set `[execution].max_output_tokens` to the same response limit.
Preserve exact-client loopback proof of that value on the first request and
tool continuation before native planning. A larger declared capability or an
old 8192-token proof does not establish the new request limit. Changed controls
need fresh config/material/plan identities; prior sealed runs remain unchanged.

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
Native dispatch uses the smaller supplied `time_limit_seconds` and
`agent_timeout_seconds` as the Agent-stage cap, and applies
`verifier_timeout_seconds` as the verifier-stage cap. Existing narrower task
limits remain in force; persisted stage configuration is checked against the
dispatched caps. The pinned adapter profile supplies a separate outer deadline
for each whole native job, not the whole experiment. A positive explicit
`max_output_bytes` rejects native resolution because it has no enforced native
projection. Outer job stdout/stderr use the profile bounds, and native client
capture uses the separate 64 MiB per-stream artifact bound. Record the effective
limits and their execution-path evidence. Do not improvise an outer timeout that
bypasses owned cleanup. Wall time and per-response output limits are not
total-spend guarantees. Record estimates, provider usage, billed cost and unknowns
separately. Finish preparation before requesting revised unsupported bounds;
never silently omit the user's requirement.

### Agent turn budgets

Inspect the frozen client projection and exact-client proof alongside native
time limits. The current projection
omits `[defaults].max_iterations`; current-tree Opi has no default turn-count
ceiling and only the product enforces an explicitly supplied budget. An older
Opi executable may still default to 50: a version label or absent field alone
does not prove the selected executable's behavior. Native task/adapter deadlines
and cancellation still apply. Disclose the observed turn policy and deadlines
in the preview, separately from tokens and pi's actual loop/retry policy.

Before using a new Opi executable/projection combination, verify it with the
runtime's offline `native_client_turn_budgets` conformance check. It must
finish beyond 50 turns using both the default and production projection, while
an explicitly small product budget stops before the next request. A failed
exact-client check is a preparation gap; do not silently substitute a newer
executable or fabricate a limit. Changed projection bytes require fresh output-control
proof, material and plan identities; preserve old candidates and sealed runs.
The machine plan has no dedicated Agent-turn-budget field, so retain the
projection and actual-client check alongside the readable preview.

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
"$EVAL" run --plan "$PLAN" --root "$RUN_ROOT"
"$EVAL" verify --root "$RUN_ROOT"
"$EVAL" report --root "$RUN_ROOT" --out "$REPORT_A"
"$EVAL" report --root "$RUN_ROOT" --out "$REPORT_B"
cmp "$REPORT_A" "$REPORT_B"
```

Native execution does not require `--fixtures`. Supply native material when
**creating** the plan; `run --plan` loads its recorded material. An additional
`--native-material` must match that frozen identity. Verify the plan actually
carries native material before launch; never drop it to obtain a fixture run.

When the operator supplies a private canary file, append `--canaries <path>`
to `run` and both `report` calls. This is a leak rejection gate, not a general
redaction exporter. Keep the file outside evidence and never print its contents.

A nonzero run exit still requires inspection of receipts, settlement and
coverage; run read-only verification where evidence exists and report failures.
Do not let a shell `set -e` erase the failed run's handoff. Conversely, failed
prepare/validate/plan blocks live execution entirely.

`--recover` launches no Agent and performs read-only classification of the
preserved receipts and journal:

```sh
"$EVAL" run --plan "$PLAN" --root "$RUN_ROOT" --recover
```

Save that classification outside the original native run report. The current
`--replacement-for <trial-id>` flow expands selection before assigning fresh
identities to the whole declared scope. It does not isolate one failed logical
trial. Within the standing [retry allowance](../SKILL.md#evaluation-defaults),
prepare a fresh experiment containing only the failed logical trials, and retain
the original failures and explicit attempt relationships. Do not rewrite the
old plan, trial IDs or run-control to force acceptance. Repeated successful
trials or other scope expansion need their own authorization. Every retry
shares the original batch timer and each logical trial's attempt count.

**Completion criterion**: the preserved command record and sealed settlements
account for every planned trial, and each published native report verifies and
reconstructs byte-identically. Platform, material, access and privacy failures
remain visible when that criterion cannot be met.
