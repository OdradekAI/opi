---
name: opi-audit
disable-model-invocation: true
description: Run a fresh-context, read-only conformance audit of one registered Phase and install its validated report into the live indexed assurance set.
---

# Opi Audit

Run a fresh-context, read-only conformance audit of one registered Phase. The
Phase selects scope; the latest committed normative sources select expected
behavior. Prior audit and remediation conclusions are outside the evidence
boundary. The completed report installs independently into the live indexed
assurance set; it never waits for another reviewer.

## Invocation and contract ownership

Require `phase=<N>`, `reviewer=<reviewer-id>`, `model=<model-id>`,
`reviewer_model_id=<exact-runtime-model>`,
`model_identity_source=runtime-attested|request-config|operator-declared`, and
`member=<member-directory>` naming a private directory outside the live
assurance directory that will hold this peer's four suffixed files. Accept
optional narrow `focus=<area>` as an ordering hint; it never reduces Phase
proof obligations. `model` is a file-identity slug for the model already
selected by the runtime, never a model selector.

The shared `audit-set-contract.md` owns the index, suffixed member paths,
schemas, verdict derivation, rotation, installation, recovery, and migration.
This skill owns one peer's ordered execution and completion criteria.
`references/finding-template.md` owns Markdown rendering.

After rotation admission, read:

1. `../../../AGENTS.md` and `../../../README.md` for repository authority;
2. `../_shared/references/audit-set-contract.md` for the live-set contract;
3. `references/audit-proof-obligations.md` for the requirement matrix and
   anti-vacuity rules;
4. `../_shared/references/finding-contract.md` for finding records; and
5. `references/finding-template.md` when rendering the member files.

Then read the committed root state, pointed Phase state, latest committed
`docs/opi-spec.md`, and every currently registered supplemental source. Load
implementation files only when a sealed proof obligation points to them.

## Context boundary

Eligible evidence comes from a sealed committed baseline and its implementation
surfaces. Begin without prior audit/remediation conclusions in context. If the
task already contains an earlier report or conclusion, stop with
`AUDIT-INCOMPLETE: history-contaminated context` and request a fresh audit task.

Use the `rotation` validator for prior-set path existence and Git cleanliness.
Keep prior Phase-level audit/remediation content, its Git history, copied
conclusions, and every sibling peer's staged or installed output outside the
run. If a broad search exposes any such conclusion, invalidate this peer and
restart in fresh context.

## Workflow

### 1. Admit rotation

Run:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase<N>
```

Continue only when the validator admits a new installation. It checks path
state without loading prior conclusions. If the live index is a legacy
schema-1 set, the coordinating operator runs `assurance_set.py migrate` once
before any member installs.

### 2. Seal this peer's baseline

Resolve all registered sources from the committed ledgers and hash their exact
committed bytes as required by the live-set contract. A stored hash mismatch
is current metadata evidence; missing or malformed registered scope is
`AUDIT-INCOMPLETE`.

Record the full `audit_head` this peer inspects (the current committed HEAD at
seal time unless the operator names an explicit committed ancestor), reviewer
and model identity, `model_identity_source`, independence class, and a unique
Phase-scoped `audit_run_id`. Never infer an exact model identity. The head
must satisfy `git merge-base --is-ancestor <audit_head> HEAD` at completion;
mixed heads across the live set are expected and valid.

Export `audit_head` to a unique temporary directory with `git archive`.
Do not use a Git worktree. Inspect and run checks only in that export, with
this peer's four files in the separate member directory. Any HEAD drift that
removes the sealed ancestor before installation invalidates the run.

### 3. Seal requirements before implementation inspection

Derive every independently decidable mandatory and optional obligation from
the committed state and registered specifications. Write the complete sealed
requirement identities, criterion evidence, and observable behaviors before
reading production implementation.

Then populate only the evidence fields governed by
`references/audit-proof-obligations.md` and the live-set contract. Keep the
sealed requirement set unchanged in response to implementation discoveries.

### 4. Inspect the complete committed implementation

Run Standards and Spec as separate axes. Inspect security/authority,
invariants, integration, test quality, residuals, and minimum-change
conformance. Trace declarations through production consumers, durable formats,
adapters, commands, and tests.

Run the narrowest sufficient checks in the export and apply the anti-vacuity
rules. This step is complete only when:

- Every sealed requirement has current evidence or an explicit limitation.
- Every durable or public seam is traced to production consumers and
  discriminating tests, or its absence is recorded.
- Every Blocker or Major finding includes current refutation evidence and why
  that evidence does not defeat the claim.

### 5. Render, validate, and install

Write current findings only from `audit_head` evidence. Render exactly this
peer's `audit.<reviewer-id>.<model-id>.*` four-file group in the member
directory with `references/finding-template.md`; let the shared contracts own
record shape, reciprocal links, hashes, and verdict derivation. The staged
files must use LF line endings only.

Install the completed report into the live set under the assurance lock:

```text
python .agents/skills/_shared/scripts/assurance_set.py complete docs/snapshots/phase<N> <member-directory> --reviewer <reviewer-id> --model <model-id>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```

`complete` validates the member, admits it against rotation cleanliness and
the ancestor rule, installs the four files, and advances the index; a re-run
of the same reviewer/model replaces its own entry and archives the superseded
run under `history/<audit-run-id>/`. On interruption, run
`assurance_set.py recover` before retrying. Do not copy active files manually.
Failed member validation produces `AUDIT-INCOMPLETE`; do not report an
aggregate verdict for the set.

## Completion criterion

This peer completes when its sealed `audit_head` remains a committed ancestor
of HEAD, every sealed requirement meets the Step 4 evidence criteria, and its
four files are installed in the live `audit.index.json`. The live set is
consumable at any membership count; only the validator's fail-dominant
aggregate describes the set. Otherwise report `AUDIT-INCOMPLETE` with the
exact blocking evidence; do not report an aggregate verdict.
