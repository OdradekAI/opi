# Configured benchmark operator

Read for `baseline`, `candidate`, `compare`, or a caller reusing the benchmark
operation. The skill selects scope, reviews evidence and communicates results;
the evaluation runtime owns configuration resolution, builds, durable operation
records and comparison. The native Companion owns materials, plans, execution,
seals and native scores. This is not an implementation/admission workflow.

**Status:** the decoupling design below is proposed. The existing operator is
still checkout-bound; the commands in the later sections describe that current
implementation. Runtime relocation, new configuration fields and additional
diagnostic capture have not been implemented. Reading newer research does not
change the admitted pi pin or create new benchmark results.

- [Decoupling design](#decoupling-design)
- [Configuration](#one-editable-configuration)
- [Preparation and execution](#prepare-review-and-execute)
- [Baselines and comparison](#baselines-and-offline-comparison)
- [Evidence and caller handoff](#evidence-and-caller-handoff)

## Decoupling design

Keep one workflow with three ownership boundaries:

```text
calling skill / human
  -> opi-eval skill: scope, authorization, diagnostics, interpretation
    -> installed evaluation runtime: prepare, execute, verify, compare, handoff
      -> subject build adapter: pinned pi package / selected Opi working tree
      -> native Companion and benchmark integrations: execution and grading
```

| Owner | Maintained material | Consumed interface |
|---|---|---|
| Skill | Instructions, diagnostic checklist, scope and handoff rules | Runtime entry point, supported contract versions and returned artifact references |
| Evaluation runtime | Existing Python operator, sole config template, snapshot lookup, retries, cancellation, reports and frozen packages | Native CLI and its bundled resources; explicit subject source/package inputs |
| Subject build adapter | Product-specific source inventory, locked build and executable verification | Explicit checkout or pinned package; emits immutable source/build identity |
| Native integration | Pinned clients, benchmark materials, capture and native grader | Prepared subject package and frozen trial plan |

The recommended change moves the existing Python operator into the evaluation
runtime package. Keeping the whole runtime inside a skill preserves the same
coupling; rewriting it in Rust adds migration cost without resolving the
ownership problem. Preserve the existing Python/Rust boundary and avoid a new
crate, generic plugin registry or public distribution promise.

The skill's install location and the subject checkout are independent inputs.
After migration, skill files contain no repository-root inference, Cargo
package selection, private source-module imports, internal resource paths or
mandatory links to a project's research tree. Product-specific knowledge
belongs to the runtime's real pi/Opi adapters. Project repair/admission policy
remains a caller-supplied context, separate from the portable benchmark contract.

### Runtime package and one profile

Use one explicitly selected, self-contained runtime package with a verified
entry point, native executable, operator modules, configuration template, locks,
proof/capture helpers and integrations. Resolve its own resources relative to
the package, never relative to the skill or subject. Record package contents,
dependency pins and executable digests separately from the subject build.
Package metadata exposes supported operation/handoff versions, pi pins,
benchmark families, model controls, enforced bounds and capture capabilities.
This is a provisional local process contract; public packaging requires its
own admission evidence.

Extend the sole private profile in the runtime with these bindings. These are
design fields, **not accepted keys in the current `/1` profile**:

| Binding | Meaning |
|---|---|
| `runtime.package` | Absolute installed runtime package path; verify its content identity during preparation |
| `subjects.opi.source` | Absolute current working-tree path, required for candidates; irrelevant to pi-only and offline operations |
| `storage.index` | Absolute external snapshot-discovery index, independent of project documentation |
| `diagnostics.required` | Selected diagnostic capabilities that must be available before new dispatch; empty means disclose available capture and gaps |

Keep model/API/endpoint, credential source, thinking level/required flag,
retries, concurrency, deadlines, benchmark selection and workspace/cache in
this same profile. Defaults come from the runtime's template; the skill carries
no second executable default table. Explicit typed overrides win and their
origins are frozen. Unknown fields reject. Bump the profile version for these
new bindings; never silently reinterpret an existing `/1` profile.

Every invocation supplies an explicit profile or frozen operation. Resolve
profile-relative file paths against the profile location, then freeze absolute
paths; no implicit search in the skill or current working directory. The
runtime obtains any build-cache coordination through its subject adapter.
Missing required tools produce a named preparation failure.

### Subject and evidence independence

`baseline` prepares the selected admitted pi version without inspecting or
building Opi source. Multiple admitted pi versions can retain independent
snapshots; a new version needs verified acquisition and adapter proof before
dispatch. Update pins explicitly rather than treating a research version or
an ambient executable as an admitted subject.

`candidate` builds the explicitly configured current Opi tree, including
relevant dirty inputs, for every preparation. It freezes source inputs and the
newly reported executable; build failure never falls back to a cached Opi
binary. Keeping the evaluation runtime fixed across candidates makes subject
changes distinguishable from grader/integration changes.

The external index is discovery data, not evidence or a progress ledger. Runtime
operations write their own roots and append discovery references there. Project
conclusion reports and any project registry entry are an explicit caller-owned
handoff, rather than an execution prerequisite. Import old snapshot references
without rewriting originals; verify each original before eligibility. Offline
verification and comparison use the frozen compatible runtime package and
retained evidence, without a subject checkout, build, credential resolution or
network request.

Select and freeze a baseline before live dispatch by admitted pi version,
benchmark/task/grader identities, model/connection/effective controls, tools,
environment and repetition/seed policy. Label saved comparisons `cross-run`
and disclose timestamps, service-version uncertainty and coverage. The same
model name does not establish the same remote model implementation. Runtime
changes require reviewed grading-path equivalence or an `incomparable` result;
do not weaken current eligibility checks to improve the match rate.

### Migration acceptance

Complete runtime migration only when all these observable checks pass:

- Copy the skill to an unrelated directory: it reads its own references and
  uses a supplied runtime without discovering a repository from its location.
- Resolve the sole profile and prepare a synthetic pi baseline with no Opi
  checkout present; no Opi build or source access occurs.
- Prepare an Opi candidate from a relocated dirty checkout: build identity
  changes with relevant inputs, and a failed build produces no reused binary.
- Execute/verify a frozen synthetic operation after removing its source
  checkout and skill installation; the operation's pinned runtime remains
  usable. Reject a changed package, unsupported contract or missing resource.
- Compare retained fixtures from a read-only checkout with credentials absent
  and network disabled; originals remain unchanged and only the configured
  external output/index can be written.
- Preserve the precise retry/deadline/privacy/seal contracts and exercise the
  diagnostic gaps and invalid inputs in [diagnostics](diagnostics.md).
- Test both actual Agent integrations and benchmark-native grader paths with
  offline fixtures and existing exact-client proofs. These checks create no
  benchmark performance claim or permission for paid execution.

## One editable configuration

Current checkout-bound implementation, pending the migration above:

```sh
python3 .agents/skills/opi-eval/scripts/evaluate.py init
python3 .agents/skills/opi-eval/scripts/evaluate.py resolve --config .opi/eval.toml
```

`init` creates the ignored private profile exclusively from
[config.example.toml](../config.example.toml); it never replaces an existing
profile. Edit this one file for model, API, endpoint, credential source, model
controls, thinking, logical trial retries, whole-batch deadline, benchmark
selection and storage. `resolve` validates without resolving credentials or
starting a provider. `--set key=TOML_VALUE` supplies a typed invocation override;
unknown keys, wrong types and contradictory selections reject.

Exactly one credential source is allowed: `credential_env`, `credential_file`,
or `api_key` in the private profile. An inline key is projected to a temporary
environment reference for the selected live launch; the sanitized profile,
generated experiment, plan, reports and command records do not contain it.
Keep the real profile outside shareable evidence and restrict access to it.
Never print or hash its secret-bearing bytes as an exportable identity.

`[model.thinking]` selects `level` and `required` once. The generated experiment
uses `[model_controls].reasoning`; actual native launch and exact-client
loopback proof must agree. The same name does not by itself prove equal wire
settings. Unsupported levels fail when required and remain explicitly
unsupported otherwise. Record per-client mappings, actual observations and
missing capability rather than silently lowering the level.

The template retains the established model and budget defaults. Whole-batch
time starts at first live dispatch and includes graders and retries. A full
corpus request can need a larger explicitly selected deadline; expiry produces
an incomplete operation with all undispatched trials visible. Configured
concurrency remains an upper bound; the current native Companion runs trials
serially. Logical trial retries do not control client-internal request retries.
The Companion's unsupported total-token and monetary caps remain unsupported.
Nonempty task `seeds` currently reject operator preparation: the native route
does not enforce that selection field. An empty table records no seed control.

## Prepare, review and execute

Native preparation and execution run in Linux/WSL x86_64 with Docker Linux.
Configure absolute Linux workspace/cache paths outside the source checkout.
The workspace and material cache must be disjoint. The helper admits at most
10,000 logical trials per family and checks the expanded count before creating
a plan; oversized requests reject without allocating their trial expansion.
Use the shared Cargo target cache and lease workflow in
[native builds](native-run.md#platform-and-builds); each operation freezes its
actual build package and material view so later candidates cannot replace its
runtime bytes.
Newly acquired materials are retained as independent `storage.cache/seeds/`
generations, excluding disposable runner environments. A later preparation
physically copies a matching seed into its own cache and recreates the runner
environment at that final path; existing generations and operation packages
remain unchanged. These seeds are reusable material, not a progress ledger.

```sh
python3 .agents/skills/opi-eval/scripts/evaluate.py baseline --config .opi/eval.toml --bench tb21
python3 .agents/skills/opi-eval/scripts/evaluate.py candidate --config .opi/eval.toml --bench tb21
```

These commands prepare and return the external operation directory and
`handoff.json`. They do not run a live benchmark. Preparation may download
pinned dependencies/images and runs synthetic loopback client proofs; inspect
the full requested task inventory, effective controls, paths, build identities,
exact attempted-count limit and budget enforcement before live dispatch.

`baseline` selects only pi and checks its version against the admitted static
lock. A new upstream release requires an updated, verified pin and supported
adapter; it never substitutes an ambient installation. `candidate` builds the
current Opi working tree with relevant dirty inputs, checks its build inputs
before/after, and freezes the executable. An explicit historical Opi experiment
is a separate benchmark request, not a candidate operation.

With explicit authorization bound to that prepared scope:

```sh
python3 .agents/skills/opi-eval/scripts/evaluate.py baseline --prepared /absolute/operation --execute
python3 .agents/skills/opi-eval/scripts/evaluate.py candidate --prepared /absolute/operation --execute
```

The reviewed operation is the input; the private profile cannot silently alter
its scope. The operator preserves command cwd/argv, start/end, exit and bounded
stdout/stderr outside seals. At the shared deadline it stops dispatch and asks
the Companion to cancel through its supported signal/cleanup route. Cleanup
uncertainty stops further live work and stays a typed gap.

Eligible failed logical trials get new precise plans and attempt roots. Valid
native zero and benchmark-valid timeouts are completed results. The first valid
settlement is selected; earlier attempts, consumption and replacement links
remain visible. Never use whole-scope `--replacement-for` as a single-item retry
or choose the best score from several valid attempts.

## Baselines and offline comparison

A snapshot is an immutable derived index over original run/attempt roots,
verified native reports and bundle identities. It records the full expected
inventory and each logical result; incomplete snapshots are retained but never
labelled full benchmark completion. The current implementation registers baseline evidence through
`docs/eval/evidence-roots.jsonl`, with snapshot path/digest and owning report.
The registry locates evidence; it does not upgrade a missing seal or score.

The candidate freezes its selected baseline before live execution. Use
[sealed baseline](sealed-baseline.md) for the comparison dimensions. Missing or
ineligible baseline evidence leaves a standalone Opi result, never an automatic
pi provider call. Snapshot selection is deterministic and independent of score.

To reconstruct comparison separately:

```sh
python3 .agents/skills/opi-eval/scripts/evaluate.py compare --candidate /absolute/candidate/snapshot.json --out /absolute/fresh-comparison
python3 .agents/skills/opi-eval/scripts/evaluate.py compare --candidate /absolute/candidate/snapshot.json --baseline /absolute/pi/snapshot.json --out /absolute/fresh-comparison
```

Both originals are re-verified; output is outside their evidence roots. Bind
paired task/repetition/seed and bundle identities in one comparison manifest,
labelled cross-run. Per-run native metrics keep their own full denominators;
paired differences use only the verified comparable intersection. Missing,
invalid, mismatched or unavailable telemetry remains explicit. Never compare
an Opi subset's aggregate with pi's whole-corpus average as a paired result.

## Evidence and caller handoff

Keep the whole operation outside the checkout: sanitized inputs, source/build
identities and frozen packages, material/plan/control proof, all run roots and
attempt links, command/check outputs, snapshots, comparisons and `handoff.json`.
The configured `storage.workspace` contains one fresh operation directory:

```text
<baseline-or-candidate>-<bench>-<operation-id>/
  inputs/        sanitized effective profile, experiment, plan, baseline binding
  builds/        source/build identities and exact Companion package
  cache/         this operation's frozen client and native material view
  material/      prepared native material
  commands/      argv, cwd, timing, exits and permitted stdout/stderr
  checks/        control proof, seal checks and reconstructed reports
  runs/          original native attempt roots and sealed bundles
  attempts/      dispatch and precise retry records
  prepared.json  frozen operation contract
  handoff.json   preparation result and exact execution command
  snapshot.json  final verified logical results and coverage
  comparison.json       candidate comparison, when applicable
  comparison-manifest.json  original snapshot and comparison digest bindings
  result-handoff.json   execution result and all evidence pointers
```

`handoff.json` describes preparation; consumers read `result-handoff.json` after
execution. Offline `compare` writes its own `comparison.json`,
`comparison-manifest.json` and `handoff.json`
in the supplied fresh output directory. Contracts are versioned as
`opi-eval-prepared-operation/1`, `opi-eval-baseline-snapshot/1` (with an explicit
pi/Opi product), `opi-eval-cross-run-comparison/1` and `opi-eval-handoff/1`.
Follow [evidence privacy](evidence-root.md) before sharing, and
[environment cleanup](environment-cleanup.md) after preserving evidence.
`full` retains permitted event streams; it does not promise recoverable pi
sessions, private thinking, complete final container filesystems or measured
telemetry absent from the actual client. Such gaps remain in the handoff.

Use the self-contained [diagnostic checklist](diagnostics.md) when assessing
whether a retained run can explain an Opi regression. Its retention requirements
do not depend on a project research checkout. Current permitted streams are
partial diagnostics: complete native sessions, measured system-prompt digests,
per-trial telemetry and final-workspace coverage require adapter evidence,
rather than a stronger interpretation of `capture_policy = "full"`.

Keep `score_validity`, `diagnostic_completeness` and `comparison_eligibility`
separate in the proposed handoff extension. A missing session does not erase
an otherwise verified native score, and unknown telemetry never becomes zero.
If selected required diagnostics are unavailable, stop preparation before a
new dispatch; this policy cannot retroactively invalidate historical scores.
The current `/1` handoff does not yet contain these additional dimensions.

The machine handoff exposes status, effective configuration, candidate
source/build, planned/attempted/valid/missing coverage, selected baseline and
comparison status, attempt roots, evidence/report/check paths and typed gaps.
Consumers read its schema/version and actual outcomes, not human prose or an
exit-zero assumption. Native grade provenance stays with Companion reports.

Another explicitly authorized skill may read this contract and call these
operations with supplied scope and budget. Reading or invoking a parent skill
does not authorize a new paid batch, update pi pins, repair source, regrade,
commit, or publish. A nested call inherits the parent's remaining live budget;
the caller must set that remaining allowance and supervise the original
deadline and supported cancellation route across families. This helper starts
one timer per prepared family; it does not automatically discover or inherit
a parent's timer. A caller without that supervision must not use repeated
operations as one authorized batch. It cannot create a second progress ledger. Eval findings
remain evidence until the owning repair/audit workflow admits them.
