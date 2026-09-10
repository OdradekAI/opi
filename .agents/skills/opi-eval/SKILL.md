---
name: opi-eval
description: Run native benchmarks or explicit fidelity cases, report evidence, and repair Opi source after user confirmation.
disable-model-invocation: true
---

# opi-eval

Produce evidence through the Rust CLI in `crates/opi-eval`. The skill owns
operator input, authorized execution, private evidence staging, readable
reports, and post-evaluation environment cleanup; the Companion owns frozen
trials, native grading, seals, and offline reconstruction. Opi source fixes follow the
[user-confirmed repair branch](#user-confirmed-source-repairs). Only
`opi-implement` owns `.opi-impl-state.json` and Phase exit.

## Inputs and routing

These are skill inputs, not additional CLI flags. Accept natural-language
software names, versions, benchmark selections, and models, then resolve them
to the following contract. Ask only for unresolved choices that affect the
run; reuse explicit authorization and selections already supplied.

```text
mode=benchmark|acceptance|verify|fidelity
software=opi|pi|opi,pi
version=<version for one software, or opi:<version>,pi:<version>>
bench=terminal-bench-2.1|terminal-bench-3.0|deepswe-v1.1|all
model=<provider:model>  api=<protocol>  base_url=<endpoint>
credential_env=<name> | credential_file=<private absolute path>
tasks=<family:task-id,...>  scope=subset|full  repetitions=<positive integer>
concurrency=<positive integer>  budget=<requested time/usage/spend limits>
workspace=<independent Linux directory>  cache=<local material cache>
config=<existing experiment.toml>  evidence_root=<existing root for verify>
cases=<name,...|all>  # fidelity branch only
```

| Selection | Route |
|---|---|
| Software/bench evaluation, no mode | `benchmark`; native CLI workflow below. |
| Six-trial evidence for task 19.11 / P19-AC-012 | `acceptance`; first read [local acceptance](references/local-acceptance.md). |
| Existing evidence, no new execution | `verify`; go to Step 4 with supplied roots. Never resolve credentials. |
| Explicit canaries or `cases=...` | `fidelity`; use [fidelity cases](references/fidelity-run.md) instead of the native workflow. |

An explicit `mode` wins; reject contradictory inputs instead of combining
branches. A bare invocation collects software, version, bench and model; it
never selects paid trials or a configured default model silently. An existing
config supplies defaults, but conflicting requested overrides require a new
experiment and plan. `verify` derives identities from evidence, not new inputs.

## Step 1: Resolve the operator request

Read the [crate README](../../../crates/opi-eval/README.md),
[local walkthrough](../../../crates/opi-eval/examples/local-evaluation/README.md),
and [native run reference](references/native-run.md). For acceptance, also use
the registered source linked by that branch; inspect task 19.11 read-only to
identify its handoff, never to generate benchmark cases or update progress.

Present the available pinned benchmark families and the selected corpus task
inventory. Bind each software version to its actual source/build/executable
identity; `software` currently admits only Opi and pi native integrations.
An unsupported version/adapter is an explicit preparation gap, not permission
to substitute the current build. A pi-only selection needs no Opi executable.

Resolve one named connection, common controls, full/subset tasks and reasons,
repetitions, concurrency, enforceable bounds, storage, capture, and failure
policy. Explicitly distinguish the evaluated software version from the
Companion build version. Keep credential values out of conversation and reports;
only the selected Agent launch resolves their declared reference.

**Completion criterion**: every input is resolved or named as a blocking input;
the chosen branch and exact software/model/benchmark scope are visible. Missing
live authorization still permits preparation, never a provider call.

## Step 2: Prepare and freeze

Follow [native run](references/native-run.md) for cached builds, WSL/Linux
execution, material acquisition, configuration projection, and exact commands.
Use the built Companion for `doctor`, `prepare`, `validate`, and `plan` from
the independent directory. Each benchmark family has its own experiment,
material manifest and plan. Keep the full inventory and rejected requirements;
resource failures never silently shrink a requested full run.

Before freezing capture, read the privacy section of
[evidence root](references/evidence-root.md); sealed bytes cannot be edited
later to make an unsafe capture shareable.
Before acquisition/builds create resources, record the resource baseline and
protected locations required by [environment cleanup](references/environment-cleanup.md).

Show the resulting plan digests, subjects/builds, task IDs, model connection,
effective/unsupported controls, exact trial count, concurrency, time limits,
capture policy and budget enforcement limits. The current CLI rejects total
token and monetary caps; resolve any required unsupported bound before running.
Honor the user's approval of this exact scope. If authorization is missing,
finish the preview and ask once, citing the remaining live/paid effect.

**Completion criterion**: every selected family has verified local material,
successful validation and a frozen feasible plan; all trials and bounds are
accounted for, with authorization bound to that scope before Step 3.

## Step 3: Run the exact plans

Execute `run --plan` through the Companion's native path as documented in
[native run](references/native-run.md). Preserve each command's cwd, sanitized
argv, start/end time, exit, stdout/stderr, plan and material identity. Let the
selected Agent operate in the task environment and the native grader consume
that same trial's declared artifacts. Shell exit alone is not a score.

Retain every failed, cancelled and unstarted trial and its typed settlement.
On authority, privacy or budget breach, cancel owned execution and stop affected
dispatch. Recovery classifies prior effects; a separately scoped replacement
uses new identities and preserves the original group. No automatic paid retries
or substitution of witnesses for real trials.

**Completion criterion**: every planned trial has a settled result or an explicit
unexecuted/effect-unknown record. An invalid trial remains invalid; completion
of the run operation does not imply acceptance.

## Step 4: Verify and stage shareable evidence

Read [evidence root](references/evidence-root.md). Run Companion `verify` and
`report` for each run root, preserving exits and typed failures. Reconstruct
each successful report twice to distinct external paths and compare bytes.
`regrade` is a separate, explicitly requested native-grader operation; byte
verification and report reconstruction never imply regrading occurred.

Stage privacy-reviewed evidence without changing sealed bytes. Produce the
acceptance manifest, repository-qualified upstream cache, and exact Opi-only
capture view only for the acceptance branch. Then run its offline acceptance
verifier and generic artifact audit against that view. For `mode=verify`, use
the [local acceptance](references/local-acceptance.md) route only when the
supplied manifest has the acceptance schema; otherwise verify the supplied
ordinary run roots. Never reconstruct a missing
trial or approval from a template, source code, or historical run.

**Completion criterion**: all selected roots have explicit verification results;
shareable roots retain their original seals and pass the privacy review. Any
missing/invalid evidence yields a blocked or incomplete report, not a ready
acceptance handoff.

## Step 5: Report and hand off

Use [benchmark report](references/benchmark-report.md) for native/acceptance
and verify results. Write unique English/Chinese reports under `docs/eval/`
and retain machine reports beside the external evidence. Keep native metrics
per benchmark, with denominator, coverage, missing pairs and unknown telemetry.
Report each requested criterion as evidenced, missing or rejected with an
artifact link; acceptance readiness is distinct from Phase completion.

The final handoff gives the evidence-root path, manifest digest, report paths,
exact verification commands/exits, six-trial coverage when applicable, and
remaining gaps. Eval findings follow the shared
[finding contract](../_shared/references/finding-contract.md); only a subsequent
independent audit can admit them to the live remediation set. Do not commit
reports or invoke another user-only workflow without an explicit request.

**Completion criterion**: readable reports agree with verified native JSON and
all attempted/planned coverage; a blocked run has a useful gap report without
invented evidence. Complete [post-evaluation cleanup](#post-evaluation-cleanup)
and include its result before final handoff. The user can locate the evidence
and repeat its offline checks.

## Post-evaluation cleanup

After execution, evidence preservation and report preparation, follow
[environment cleanup](references/environment-cleanup.md). Inspect Docker images
and other evaluation leftovers; retain reusable evaluation materials and all
evidence needed to trace, verify or reproduce results. Automatically remove
only proven run-owned disposable resources under the authorized evaluation
scope. Apply this on success, failure, cancellation, and completed repair
checks; preserve resources needed by an active or resumable run.

This is an operator procedure, not a new Companion CLI command. `mode=verify`
keeps supplied evidence and shared Docker resources read-only; it may clean
only disposable scratch created by that verification invocation.

**Completion criterion**: a retained cleanup record identifies removed resources,
kept resources and reasons, unresolved failures, and post-cleanup evidence
checks. An uncertain resource stays intact with its reason; incomplete cleanup
is explicit and never changes a trial's score or verification status.

## User-confirmed source repairs

In any eval mode, an observed defect or execution blocker may lead to an Opi
source repair within this skill. Present the evidence, intended behavior,
affected source scope and verification plan before seeking confirmation. Once
the user explicitly confirms that repair scope, proceed directly; a separate
`opi-implement plan` or `opi-remediate` invocation is not required. Reuse an
existing clear approval for the same scope; ask again only for a material scope
expansion. Authorization to run trials alone does not authorize source fixes.

1. Reproduce the defect offline where possible and define an observable success
   criterion. Keep the repair consistent with the normative specification;
   product-direction changes still require human-led specification revision.
2. Make the minimum Opi runtime, product or Companion source change needed for
   the approved repair, with affected regression tests and documentation. Use
   [change-scope and check selection](../_shared/references/change-scope-and-check-selection.md)
   to select checks, including workspace gates for cross-crate runtime changes.
   Repair approval covers offline reproduction, builds and verification with
   mocks, synthetic credentials and isolated fixtures.
3. Preserve prior trials, sealed evidence and their frozen executables/materials.
   Build the repair as a separately identified candidate; record source revision,
   dirty diff, executable and affected material identities. Changes to a candidate
   do not retroactively repair or regrade an existing result.
4. Record the approval, reproduction, changed scope, test impact, exact check
   commands/results and remaining gaps in the eval handoff. Repair evidence
   remains outside the live audit set; independent audit admission, implementation
   ledger updates and Phase exit retain their existing owners.
   Apply post-evaluation cleanup to completed repair-check scratch as well,
   preserving the separate candidate and regression evidence.
5. Return to preparation for any subsequent evaluation. Repair approval does not
   authorize additional paid trials, new credential use, commits or publication;
   obtain any missing authorization for the new frozen execution scope under
   Step 2. Do not present offline regression success as a new benchmark result.

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
