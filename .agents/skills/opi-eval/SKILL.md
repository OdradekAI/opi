---
name: opi-eval
description: Preserve pi benchmark baselines, evaluate current-worktree Opi, compare verified evidence, and repair confirmed eval defects.
disable-model-invocation: true
---

# opi-eval

Maintain pi version baselines and measure the current Opi working tree against
eligible saved results. Use the identified native Companion CLI for trials,
grading, seals and offline reconstruction. This skill owns operator scope, execution, evidence
privacy, reports and cleanup. Opi source fixes follow the
[user-confirmed repair branch](#user-confirmed-source-repairs). Only
`opi-implement` owns `.opi-impl-state.json` and Phase exit.

## Inputs and routing

The normal benchmark entry is one of these actions:

```text
$opi-eval baseline bench=tb21 version=pi:1.0.0
$opi-eval candidate bench=tb21
$opi-eval compare run=<operator run directory>
```

Read [operator invocation](references/operator.md) for the explicitly supplied
`EVAL_OPERATOR` entrypoint, single private profile, frozen preparation/run handoff
and return contract. `baseline` records only the admitted pi version; `candidate` always
builds current-worktree Opi and compares with an eligible saved baseline when
available; `compare` is offline. Missing baselines never dispatch pi. These
actions select `mode=benchmark`; acceptance, verify and fidelity retain their
separate contracts below. Configuration and preparation do not authorize live
trials. Reuse explicit authorization for the exact selected scope.

The operator owns external evaluation records, reports, catalogue and lifecycle
receipts. Its source ownership has moved out of the skill; a complete independently
distributable runtime package remains provisional. Read
[diagnostics](references/diagnostics.md) before selecting capture requirements
or explaining a regression; assess score validity, diagnostic completeness and
comparison eligibility separately.

Resolve natural-language requests to these skill inputs, not extra CLI flags.
Reuse supplied selections and authorization; ask only for unresolved choices
that affect the run. Load only the selected branch's references.

```text
mode=benchmark|acceptance|verify|fidelity
action=baseline|candidate|compare  profile=<private operator TOML>
operator=<supplied EVAL_OPERATOR entrypoint>
software=opi|pi|opi,pi
version=<single-product version or per-product entries, e.g. pi:<version>|pi:latest>
bench=terminal-bench-2.1|terminal-bench-3.0|deepswe-v1.1|all
model=<provider:model>  api=<protocol>  base_url=<endpoint>
max_output_tokens=<positive integer>  context_window=<positive integer>
temperature=<number>|omitted|unknown  top_p=<number>|omitted|unknown
thinking=<native level>|omitted  thinking_required=true|false
credential_env=<name> | credential_file=<private absolute path>
tasks=<family:task-id,...>  scope=subset|full  repetitions=<positive integer>
concurrency=<positive integer>  budget=<requested time/usage/spend limits>
workspace=<independent Linux directory>  cache=<local material cache>
config=<existing experiment.toml>  evidence_root=<existing root for verify>
baseline_root=<registered sealed pi baseline root>  # benchmark reuse branch only
cases=<name,...|all>  # fidelity branch only
```

Read [model-control selection](references/operator.md#model-control-selection)
for thinking levels including `max`, omission semantics and the operator's
supported API/sampling bounds. Preserve explicit selections when resolving the
profile and preparing both clients.

| Selection | Route |
|---|---|
| Baseline, candidate or offline comparison | `benchmark`; [operator invocation](references/operator.md), then native/evidence references as needed. |
| Software/bench evaluation, no mode | `benchmark`; resolve the operator profile, then native CLI workflow below. |
| Sealed pi baseline reuse for an Opi-only benchmark | `benchmark`; apply [sealed baseline](references/sealed-baseline.md) before Step 2 approval. |
| Six-trial evidence for task 19.11 / P19-AC-012 | `acceptance`; first read [local acceptance](references/local-acceptance.md). |
| Existing evidence, no new execution | `verify`; go to Step 4 with supplied roots. Never resolve credentials. |
| Explicit canaries or `cases=...` | `fidelity`; use [fidelity cases](references/fidelity-run.md) instead of the native workflow. |

An explicit `mode` wins; reject contradictory inputs instead of combining
branches. For a new evaluation, omitted Opi version/source selection means the
current local repository working tree, including build-relevant uncommitted
changes. Resolve that snapshot under [native builds](references/native-run.md#opi-source-selection);
`candidate` requires this source selection: a historical Opi pin rejects rather
than replacing its current-worktree build. An explicitly requested historical
benchmark outside `candidate` may pin its source. A per-product
version map may pin only pi and leave Opi at this default. Pi has no working-tree
default: an explicit entry pins it, or `pi:latest` resolves at preparation to
the pi subject pinned by the current static lock, recording that resolved
package version in the experiment and report; `latest` never survives as a
frozen identity. A bare invocation collects software and benchmark/case scope;
omitted model and budget inputs use the resolved operator profile, shown in the
resolved request. Explicit inputs and user-supplied config values take
precedence; conflicting requested overrides require a new experiment and plan.
`verify` derives identities from evidence, not new inputs.

## Evaluation defaults

Use the supplied operator's `init --config <private-profile>` to create a private
profile from its single maintained template; explicitly select it for each operation.
Edit that file for model, connection, credential source, thinking depth, retries,
deadline, selection, baseline freshness and retention. Explicit invocation
overrides win over the private profile. Freeze sanitized effective values and origins; do not copy
secret-bearing profile bytes into evidence. Do not keep a second defaults table
in a reference or invent CLI flags for operator-only controls.

The established authorization for default retry/time/spending policy still
applies to an explicitly selected live scope. A modified profile alone does
not authorize new trials or expanded paid scope. The provider-neutral Rust CLI
does not load this profile; the operator resolves it to its existing experiment
contract. `verify` and `compare` stay offline and never resolve credentials.

Reuse this authorization without asking again for these defaults or each
eligible retry. Resolve the selected software/tasks/cases and credential
references before dispatch; selecting this skill or finding credentials alone
does not choose live task scope. The connection defaults apply to the default
subject model; another provider/model selection resolves its own connection.
The subject-model default does not select the independent evaluator's model.
Check supported controls and exact-client proof under
[model controls](references/native-run.md#model-controls). Thinking is requested
once through the profile, projected to both selected clients, verified on first
request and tool continuation, and included in baseline eligibility. A required
unsupported level rejects preparation; never claim a configured value took
effect merely because TOML accepted it.

Before the first live dispatch, freeze the effective defaults/overrides,
logical trial count, maximum attempted count, authorization source and timer
enforcement in the invocation's evidence. Record the batch start/deadline and
use one monotonic timer for the configured effective batch limit
from the first live dispatch, including grading and retries; preparation and
subsequent offline reporting are outside that live interval. At expiry, stop
new dispatch and cancel owned execution
through its supported cleanup path. A new family, replacement root or provider
request never resets the timer. An unenforceable required deadline blocks live
execution; a per-Agent timeout alone does not enforce the whole batch.

Retry failed execution within the frozen scope, remaining attempt allowance
and batch deadline. Preserve each attempt, failure and replacement link with
fresh identities; never overwrite sealed evidence. A valid native score,
including zero, is a completed result rather than a retry trigger. Authority
or privacy breaches stop affected execution. Count trial/case attempts
separately from client/provider request retries; all live activity consumes the
same time budget, and unlimited spending does not authorize unlimited trials.
Generate a fresh plan containing only the eligible failed logical trials,
retaining original identities and attempt links. The native
`--replacement-for` path redispatches its whole original scope and is not an
automatic single-trial retry. The first valid native settlement is the logical
result; do not select the highest-scoring attempt.

## Step 1: Resolve the operator request

For benchmark/acceptance, read [native run](references/native-run.md) for native
execution contracts and inspect the supplied operator's help and handoff.
For acceptance, also read its registered source and
task 19.11 handoff read-only; never derive benchmark cases or update progress
from the ledger.

Present the available pinned benchmark families and the selected corpus task
inventory. Bind each software version to its actual source/build/executable
identity; `software` currently admits only Opi and pi native integrations.
An unsupported version/adapter is an explicit preparation gap, not permission
to substitute the current build. A pi-only selection needs no Opi executable.

Resolve the connection, [model controls](references/native-run.md#model-controls),
task scope/reasons, repetitions, concurrency, enforceable bounds, storage,
capture and failure policy. Distinguish subject and Companion build identities.
Only the selected Agent launch resolves credential references; keep values out
of conversation and reports.

**Completion criterion**: every input is resolved or named as a blocking input;
the chosen branch and exact software/model/benchmark scope are visible. Missing
live authorization still permits preparation, never a provider call.

## Step 2: Prepare and freeze

Follow [native run](references/native-run.md) for builds, materials, projection
and `doctor` / `prepare` / `validate` / `plan` in the independent directory.
Baseline reuse adds the [sealed baseline](references/sealed-baseline.md)
identity checks to this freeze and its disclosure to the approval preview.
Each benchmark family gets its own experiment, material manifest and plan under
one external evaluation record. `bench=all` shares the selected families' first-
dispatch deadline; changing family or attempt never resets it.
Preserve the full requested inventory and rejected requirements; resource
failures do not authorize a smaller run.

Before resources are created, record ownership/protected locations under
[environment cleanup](references/environment-cleanup.md). Before capture is
frozen, apply [evidence privacy](references/evidence-root.md#privacy-without-breaking-seals);
sealed bytes cannot later be edited for sharing.

Show the resulting plan digests, subjects/builds, task IDs, model connection,
effective/unsupported controls, exact trial count, concurrency,
[Agent turn budgets](references/native-run.md#agent-turn-budgets), time limits,
capture policy, [evaluation defaults](#evaluation-defaults), overrides and
budget enforcement limits. The current CLI rejects total
token and monetary caps; resolve any required unsupported bound before running.
Honor the user's approval of this exact scope. If authorization is missing,
finish the preview and ask once, citing the remaining live/paid effect.

**Completion criterion**: every selected family has verified local material,
successful validation and a frozen feasible plan; all trials and bounds are
accounted for, with authorization bound to that scope before Step 3.

## Step 3: Run the exact plans

Execute `run --plan` as documented in [native run](references/native-run.md).
Preserve cwd, sanitized argv, timing, exit, stdout/stderr and plan/material
identity. The selected Agent and native grader must operate on the same trial's
environment and declared artifacts; shell exit alone is not a score.

Retain every failed, cancelled and unstarted trial and its typed settlement.
On authority, privacy or budget breach, cancel owned execution and stop affected
dispatch. Recovery classifies prior effects; replacements follow the frozen
[retry allowance](#evaluation-defaults), use new identities and preserve the
original group. Witnesses cannot substitute for real trials.

**Completion criterion**: every planned trial has a settled result or an explicit
unexecuted/effect-unknown record. An invalid trial remains invalid; completion
of the run operation does not imply acceptance.

## Step 4: Verify and stage shareable evidence

Follow [evidence root](references/evidence-root.md) for privacy and offline
checks. Run Companion `verify` and `report` for each root, preserving exits and
typed failures. Reconstruct each successful report twice to distinct external
paths and compare bytes.
`regrade` is a separate, explicitly requested native-grader operation; byte
verification and report reconstruction never imply regrading occurred.

Stage privacy-reviewed originals unchanged. Acceptance alone requires its
manifest, repository-qualified upstream cache, exact Opi-only capture view,
offline acceptance verifier and generic artifact audit. In `mode=verify`,
use [local acceptance](references/local-acceptance.md) only for a supplied
acceptance-schema manifest; otherwise verify ordinary run roots. Never invent
missing trials or approvals from templates, code or historical runs.

**Completion criterion**: all selected roots have explicit verification results;
shareable roots retain their original seals and pass the privacy review. Any
missing/invalid evidence yields a blocked or incomplete report, not a ready
acceptance handoff.

## Step 5: Report and hand off

Use [benchmark report](references/benchmark-report.md) for native, acceptance
and verify results: unique external bilingual reports and machine evidence,
native metrics/denominators, complete coverage, criterion status and exact
artifact/check links. The operator automatically exports a compact bilingual
conclusion pair to the project's `docs/eval/` and, when `storage.exports` is
configured, machine-evidence copies to that Windows-accessible root when an
execution finishes. Acceptance readiness is separate from Phase exit.
Eval findings use the [finding contract](../_shared/references/finding-contract.md);
only a later independent audit can admit them to the live remediation set.

**Completion criterion**: reports agree with verified native JSON and account
for all planned/attempted trials, including blocked runs. Hand off locatable
evidence, manifest digest where applicable, report paths, exact checks/results,
remaining gaps and [cleanup](#post-evaluation-cleanup). Commits and another
explicit-only workflow still require a user request.

## Post-evaluation cleanup

After preserving evidence and preparing reports, follow
[environment cleanup](references/environment-cleanup.md) on success, failure,
cancellation and completed repair checks. Remove only proven invocation-owned
disposable resources; preserve reusable materials, evidence and active/resumable
work. Finished managed evaluations apply the operator's latest-three rolling
policy; pins, references, active leases and unknown cleanup/dependencies stay
protected, with retained exceptions reported. Manual archive/GC remains separate
explicit plan-based maintenance. Shared material/build caches remain outside both.
`mode=verify` cleans only its own scratch, keeping supplied evidence and
shared Docker state read-only.

**Completion criterion**: the cleanup record accounts for removed/retained
resources, reasons, failures and preservation checks. Uncertain resources stay
intact; cleanup never changes scores or evidence verification status.

## User-confirmed source repairs

In any eval mode, an observed defect or execution blocker may lead to an Opi
source repair within this skill. Present the evidence, intended behavior,
affected source scope and verification plan before seeking confirmation. Once
the user explicitly confirms that repair scope, proceed directly; a separate
`opi-implement plan` or `opi-remediate` invocation is not required. Reuse an
existing clear approval for the same scope; ask again only for a material scope
expansion. Authorization to run trials alone does not authorize source fixes.

1. Reproduce offline where possible and define observable success. Keep the
   repair within the normative specification; direction changes require
   human-led revision.
2. Make the minimum approved runtime, product or Companion repair with affected
   regression tests and documentation. Use
   [change-scope and check selection](../_shared/references/change-scope-and-check-selection.md)
   to select checks, including workspace gates for cross-crate runtime changes.
   Repair approval covers offline reproduction, builds and verification with
   mocks, synthetic credentials and isolated fixtures.
3. Preserve prior trials, seals and frozen executables/materials. Identify the
   repair candidate separately by source revision, dirty diff, executable and
   affected material identities; it cannot retroactively repair/regrade a result.
4. Hand off approval, reproduction, changed scope, test impact, exact checks and
   gaps. Preserve candidate/regression evidence before cleaning repair scratch.
   Audit admission, ledger updates and Phase exit retain their existing owners.
5. Return to Step 2 for a new evaluation. Repair approval covers no additional
   paid trials, new credential use, commits or publication. Reuse authorization
   only for its exact frozen scope; offline regression success is not a new
   benchmark result.

**Completion criterion**: the approved repair has reproducible verification or
explicit remaining failures, its candidate identity is distinct, and prior
evidence is unchanged. Keep further live execution pending unless its exact
scope is already authorized.

## Fidelity authority

For the explicit fidelity branch, public-seam Rust tests and CI remain the
sole deterministic acceptance baseline. `provider-fidelity` canaries are
signals; `runtime-fidelity` cases require a registered criterion and a stated
fidelity gap. Do not create a behavior-baseline manifest or generate cases from
the implementation ledger. Compare only
`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`;
other samples are `incomparable`, resources default to `record-only`, and
`evaluator_model` / `independence` disclose the actual readonly evaluator.
These judge verdicts never replace benchmark-native grading.
