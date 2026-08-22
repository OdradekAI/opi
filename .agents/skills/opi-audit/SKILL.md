---
name: opi-audit
disable-model-invocation: true
description: Audit one registered Opi phase against committed HEAD and emit immutable normalized findings.
---

# Opi Audit

Run an independent, read-only conformance audit of one registered Phase. The
workflow is a sealed audit: freeze the evidence endpoint and proof obligations
before reading prior audit conclusions, then report immutable evidence that a
later remediation run can consume without losing identity.

## Inputs and outputs

Require `phase=<N>`. Accept an optional narrow `focus=<area>` only as an
ordering hint; it never reduces the Phase proof obligations.

At start, record:

- `audit_head`: the full committed `git rev-parse HEAD` SHA;
- reviewer/model identity and actual independence class;
- registered Phase requirements and completion state;
- separate staged, unstaged, and untracked path inventories as contamination
  evidence.

Write two new siblings under `docs/snapshots/phase<N>/`:

- `audit.<model>.<head7>.<run-id>.md`
- `audit.<model>.<head7>.<run-id>.findings.jsonl`

Derive `<model>` by lowercasing the reported reviewer identity and replacing
non-alphanumeric runs with `-`; use `unknown` only when the identity is truly
unknown. `<head7>` is the first seven hex characters of `audit_head`.
`<run-id>` is the lowercase UTC timestamp `yyyymmddthhmmssz`; if that path
already exists, append the first free `-N` suffix. Keep the full reported model
identity and SHA inside the artifact.

Never overwrite or append to an older audit artifact. Do not edit production
code, tests, specifications, the implementation ledger, or prior findings.

## Required references

Read before auditing:

1. `../../../README.md` and `../../../AGENTS.md` for workflow authority;
2. `../../../docs/opi-spec.md`, the registered supplemental sources, and the
   Phase snapshot/state for the requested Phase;
3. `references/audit-proof-obligations.md` for the proof obligation matrix and
   anti-vacuity rules;
4. `../_shared/references/finding-contract.md` and
   `references/finding-template.md` for output.

Load other sources only when a proof obligation points to them. Historical
audits are lineage evidence, not the current acceptance baseline.

## Workflow

### 1. Seal the audit endpoint

Confirm the Phase exists and capture `audit_head`. Inventory dirty paths, then
classify each as carried-in, audit-owned, or conflicting. Audit committed HEAD;
do not treat uncommitted work as evidence that a requirement is met. If a dirty
path overlaps evidence, record contamination and use `git show <audit_head>:<path>`
or stop when committed evidence cannot be isolated.

Run every build, test, or reproduction command from an isolated checkout of `audit_head`;
execution in the dirty live worktree is not audit evidence. Keep
the audit report writes in the original worktree and do not copy them into the
isolated checkout.

Build the Phase completion criterion from every registered mandatory
requirement. The criterion must be exhaustive before implementation inspection.

### 2. Build the proof obligation matrix

For every mandatory requirement, record the criterion citation, expected
observable behavior, current implementation surfaces, test/fixture evidence,
and verification command. Assign one requirement state:

- `met`
- `partially-met`
- `not-met`
- `not-assessable`

Run the Standards and Spec axes separately. Also inspect security boundaries,
invariants, integration paths, test quality, residual placeholders, and the
minimum-change conformance matrix defined in the reference.

### 3. Verify the complete current implementation

Inspect the full committed implementation relevant to each obligation, not
only a historical Phase diff. Trace declarations through production consumers,
durable formats, adapters, commands, and tests. Execute the narrowest sufficient
checks. A passing test is evidence only if its assertions can fail when the
claimed behavior is absent; apply the anti-vacuity checks in the reference.

Before accepting a Blocker or Major finding, perform the required
Blocker/Major refutation: search for counter-evidence, alternate paths, and
existing coverage, then record why they do not refute the claim.

### 4. Seal findings before history comparison

Write the current requirement matrix and candidate findings before reading old
audit conclusions. Only then compare immutable prior findings for recurrence or
coverage gaps. Do not copy an old claim into the new run without current
evidence at `audit_head`. Once sealed, history comparison may annotate lineage
or coverage but must not add, remove, or rewrite current finding records.

Each finding must use the normalized contract, keep `status: unverified`, and
cite a falsifiable claim plus observed evidence. Validate the sidecar:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py findings <path>
```

### 5. Derive the verdict from requirements

Severity ranks remediation urgency; it does not determine conformance.

- `FAIL`: any mandatory requirement is `not-met`, `partially-met`, or
  `not-assessable`.
- `PASS-WITH-FINDINGS`: every mandatory requirement is `met`, but actionable
  non-conformance findings remain.
- `PASS`: every mandatory requirement is `met` and no actionable finding
  remains.

Report missing or malformed evidence explicitly. Never manufacture consensus,
independence, source model identity, or a passing conclusion.

## Completion criterion

The run is complete only when the audit endpoint remains unchanged, every
mandatory proof obligation has a state, every actionable claim has a valid
immutable finding record, the validator passes, the verdict follows the
requirement-state rule, and no non-audit file was modified. Any HEAD drift
invalidates the run: publish no verdict and restart with a new endpoint/run ID.
