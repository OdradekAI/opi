# Opi Behavior Baseline Stratification Design

**Date:** 2026-08-11

**Status:** Approved

**Scope:** deterministic acceptance evidence and `opi-eval` runtime-fidelity evidence

## Problem

Opi already has deterministic Rust unit, integration, harness, and subprocess
tests that exercise public production seams. The implementation ledger also
registers acceptance scenarios, production call sites, and behavioral
verification. These are the effective behavior baseline, but the workflow does
not name their authority explicitly.

`opi-eval` serves a different purpose: it executes the release binary against a
real provider and uses another model to evaluate the resulting trace. Its three
current cases (`candy`, `tool_chain`, and `context_retention`) measure general
model/provider behavior rather than registered Opi product criteria. Treating
them as acceptance evidence would make admission depend on provider variance,
evaluator variance, credentials, and spend.

The current resource policy also lets the first run establish a baseline. A
single anomalous run can therefore become the comparison authority, while runs
with different models, platforms, modes, or tool sets can appear comparable.

## Decision

Use two evidence layers with one-way authority:

1. Existing public-seam Rust tests and CI are the sole deterministic acceptance
   baseline. The implementation ledger owns their mapping to registered
   criteria, acceptance scenarios, production call sites, and behavioral
   verification.
2. `opi-eval` is an optional real-provider fidelity layer. It may supplement a
   registered criterion only when that criterion explicitly requires evidence
   that a deterministic provider cannot supply.

Do not add a behavior-baseline manifest, generated registry, Rust interface,
or execution seam. Do not mirror every acceptance scenario into `opi-eval`.

## Authority and Flow

A registered behavior flows through the existing authorities:

```text
registered criterion / acceptance scenario
  -> implementation-ledger verification and production call sites
  -> public-seam Rust test command
  -> deterministic CI result
  -> optional runtime-fidelity case only when the registered source requires it
```

The deterministic result is the normal implementation-admission and audit
evidence. A generic `opi-eval` canary cannot prove or disprove a product
criterion. A runtime-fidelity result affects admission only when the registered
criterion names that evidence as required.

`opi-eval` does not copy production call sites, test commands, or acceptance
prose. A criterion-linked case records only the registered scenario reference;
the implementation ledger remains the source for the full trace.

## Case Taxonomy

`opi-eval` has two case classes:

| Class | Purpose | Criterion relationship |
|---|---|---|
| `provider-fidelity` | Detect general model/provider regressions in reasoning, tool use, or context retention | None; never acceptance evidence |
| `runtime-fidelity` | Exercise a real-provider/runtime property that deterministic tests cannot faithfully reproduce | Must cite a registered criterion or acceptance scenario |

Classify all current cases as `provider-fidelity`:

- `candy`: general answer correctness and reasoning canary;
- `tool_chain`: general real-provider tool-selection and chaining canary;
- `context_retention`: general long-context retention canary.

A future `runtime-fidelity` case is admitted only when it states the fidelity
gap, such as provider streaming behavior, tool-call wire generation, or
provider-dependent context behavior. Convenience or duplication of an existing
deterministic test is not a fidelity gap.

## Case Metadata

Each case definition carries only:

- `case_id`;
- `class`;
- `revision`;
- `criterion/scenario reference`, or `N/A` for a generic canary;
- `fidelity justification`.

Increment `revision` when inputs, assertions, run mode, or effective tool set
change semantically. Editorial changes do not increment it. The metadata lives
with the existing case definitions in `references/test-cases.md`; it is not a
second acceptance registry.

## Comparison Identity

Historical delta analysis is allowed only when the complete comparison identity
matches:

```text
case_id@revision + provider:model + OS/arch + run_mode + effective_tools
```

Record the Opi version and commit in each result, but do not include them in the
identity because cross-version comparison is the purpose of regression
analysis. Runs with a different identity are `incomparable`; the evaluator must
not calculate or narrate percentage deltas between them.

Evaluator-model identity and independence remain report fields. They explain
judgment confidence but do not make unlike subject runs comparable.

## Resource Policy

Token use, elapsed time, and tool-call counts are observations by default, not
acceptance thresholds:

- always record the observed values;
- report the resource dimension as `N/A` with `record-only` status;
- do not let the first run establish a resource baseline;
- do not let the resource dimension change the overall verdict while it is
  record-only.

A resource threshold may be enabled later only when either:

- a registered performance criterion defines the required budget; or
- at least three prior samples with the same comparison identity exist and the
  resource policy is explicitly enabled.

When historical comparison is enabled, use the comparable cohort's median
rather than the first observation. This design records the activation rule but
does not add a statistical execution module.

Answer correctness, tool behavior, context integrity, chain efficiency, and
error handling continue to be judged on every run. Resource warm-up never
suppresses those results.

## Reporting Semantics

Every case result and history entry exposes:

- case id, class, and revision;
- criterion/scenario reference;
- comparison identity and comparison status;
- subject provider/model and Opi version/commit;
- evaluator model and independence status;
- observed metrics and per-dimension verdicts.

The comparison status distinguishes at least `comparable`, `incomparable`, and
`record-only`. Resource observations remain visible when their verdict is
`N/A`.

Generic provider canaries retain their ordinary overall verdict, but reports
must label them as fidelity signals rather than acceptance proof. A
criterion-linked runtime-fidelity failure becomes admission evidence only under
the registered criterion that requested it.

## Files to Change

- `.claude/skills/opi-eval/SKILL.md`: state the two-layer authority, taxonomy,
  comparison identity, and resource policy.
- `.claude/skills/opi-eval/references/test-cases.md`: add minimal metadata,
  classify the three current cases, and define future runtime-fidelity
  admission.
- `.claude/skills/opi-eval/references/evaluator-prompt.md`: prevent generic
  canaries from being treated as acceptance evidence and prohibit incomparable
  deltas.
- `.claude/skills/opi-eval/references/report-template.md`: expose case and
  comparison fields plus record-only resource status.
- `docs/eval/README.md`: synchronize the history schema, including the already
  documented evaluator identity and independence contract.
- `scripts/opi-doc-check.py`: add a focused cross-file behavior-baseline
  contract.
- `scripts/test_opi_doc_check.py`: add happy-path and token-removal mutation
  tests for that contract.

Do not modify Rust code, CI, the implementation ledger, the audit sidecar,
existing history, or `.claude/skills/opi-eval/agents/openai.yaml`. Do not run a
real provider as part of this documentation-only change.

## Verification

Test impact: update documentation-contract tests; no Rust tests.

Run:

```text
python -m unittest scripts.test_opi_doc_check -v
python scripts/opi-doc-check.py
git diff --check -- .claude/skills/opi-eval/SKILL.md
git diff --check -- .claude/skills/opi-eval/references/test-cases.md
git diff --check -- .claude/skills/opi-eval/references/evaluator-prompt.md
git diff --check -- .claude/skills/opi-eval/references/report-template.md
git diff --check -- docs/eval/README.md
git diff --check -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py
```

The checker must detect removal or drift of the authority split, both case
classes, case revision, criterion linkage, comparison identity, incomparable
delta prohibition, record-only resource behavior, evaluator identity fields,
or the no-manifest rule.

## Rollout

The change applies prospectively without migrating runtime state:

- current cases become explicitly classified provider-fidelity canaries;
- existing deterministic tests retain their current execution paths and become
  the named acceptance authority;
- no empty `history.jsonl` is created;
- future history entries use the expanded schema;
- future runtime-fidelity cases require registered-source justification.

## Rejected Alternatives

### One unified behavior-baseline manifest

Rejected because it would duplicate the implementation ledger and test suite,
creating a third authority that could drift without adding execution leverage.

### Treat all `opi-eval` cases as acceptance gates

Rejected because generic model tasks do not trace to Opi criteria and their
outcomes depend on provider and evaluator variance. This would make normal
implementation admission costly and nondeterministic.

### Generate `opi-eval` cases from ledger scenarios

Rejected because deterministic and real-provider evidence answer different
questions. Mechanical generation would copy scenarios without establishing a
real fidelity gap.

### Keep first-run resource baselines

Rejected because a single outlier is not a stable comparison authority and can
misclassify every later run.
