---
name: opi-audit
disable-model-invocation: true
description: Independently audit one committed Phase and install its validated assurance report.
---

# Opi Audit

Audit one registered Phase without changing production code. The Phase selects
scope; the latest committed normative sources select expected behavior. Install
this peer's report independently, without waiting for another reviewer.

## Invocation and contract ownership

Require `phase=<N>`, `reviewer=<reviewer-id>`, `model=<model-id>`,
`reviewer_model_id=<exact-runtime-model>`,
`model_identity_source=runtime-attested|request-config|operator-declared`, and
`member=<member-directory>` naming a private directory outside the live
assurance directory that will hold this peer's four suffixed files. Accept
optional narrow `focus=<area>` as an ordering hint; it never reduces Phase
proof obligations. `model` is a file-identity slug for the model already
selected by the runtime, never a model selector.

The shared `audit-set-contract.md` owns the index, member schemas, verdicts,
rotation, installation, and recovery. This skill owns one peer's execution.

After rotation admission, read:

1. `../../../AGENTS.md` for repository rules;
2. `../_shared/references/audit-set-contract.md` for the live-set contract;
3. `references/audit-proof-obligations.md` before sealing requirements.

Obtain the committed root and pointed Phase scope through the Step 2 reader,
then read the latest committed `docs/opi-spec.md` and every registered
supplemental source. After sealing obligations, follow their implementation
surfaces and relevant current-product documentation.

## Context boundary

Eligible evidence comes from a sealed committed baseline and its implementation
surfaces. Begin without prior audit/remediation conclusions in context. If the
task already contains an earlier report or conclusion, stop with
`AUDIT-INCOMPLETE: history-contaminated context` and request a fresh audit task.

Keep prior Phase-level audit/remediation content, its Git history, copied
conclusions, and every sibling peer's staged or installed output outside the
run. Exposure invalidates this peer and requires a fresh context.

Read ledgers only through `scripts/read_audit_scope.py`, including in the
source export; exclude raw ledgers from searches. Filtering after tool output
cannot restore independence. Reader failure or missing scope is
`AUDIT-INCOMPLETE`; repair the reader in a separate maintenance context rather
than exposing excluded fields.

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

From the repository root, run:

```text
python .agents/skills/opi-audit/scripts/read_audit_scope.py --phase <N>
```

Use the reader's full `audit_head`; it resolves HEAD once. Pass
`--head <committed-ancestor>` only for an operator-selected baseline. Its
allowlisted projection supplies registrations, snapshot pointer and target-Phase
requirements, scenarios, verification declarations and planning notes. These
declarations establish scope, not conformance.

Use its ordered `baseline_sources` and `stored_hash_mismatches`. Baseline hashes
cover complete committed bytes, never the projection or a reserialized ledger.
The reader compares stored v2 hashes with the ledger's CRLF-normalized
convention; mismatches are current metadata evidence. Keep the projection as
read-only scope input.

Record `audit_head`, reviewer and model identity, `model_identity_source`,
independence class, and a unique Phase-scoped `audit_run_id`. Never infer an
exact model identity. The head must satisfy
`git merge-base --is-ancestor <audit_head> HEAD` at completion; mixed heads
across the live set are valid.

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

Apply the review axes and anti-vacuity rules from
`references/audit-proof-obligations.md`, keeping Standards and Spec separate.
Run the narrowest sufficient checks in the export. Completion requires:

- Every sealed requirement has current evidence or an explicit limitation.
- Every durable or public seam is traced to production consumers and
  discriminating tests, or its absence is recorded.
- Every Blocker or Major finding includes current refutation evidence and why
  that evidence does not defeat the claim.

### 5. Render, validate, and install

Read `../_shared/references/finding-contract.md` for finding records and
`references/finding-template.md` for rendering. Write findings only from
`audit_head` evidence, producing this peer's LF-only
`audit.<reviewer-id>.<model-id>.*` four-file group in the member directory.

Install the completed report into the live set under the assurance lock:

```text
python .agents/skills/_shared/scripts/assurance_set.py complete docs/snapshots/phase<N> <member-directory> --reviewer <reviewer-id> --model <model-id>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```

`complete` validates and installs under the shared contract. On interruption,
run `assurance_set.py recover` before retrying; never copy active files
manually. Failed validation or installation produces `AUDIT-INCOMPLETE`.

## Completion criterion

This peer completes when its sealed `audit_head` remains a committed ancestor
of HEAD, every sealed requirement meets the Step 4 evidence criteria, and its
four files are installed in the live `audit.index.json`. The live set is
consumable at any membership count; only the validator's fail-dominant
aggregate describes the set. Otherwise report `AUDIT-INCOMPLETE` with the
exact blocking evidence; do not report an aggregate verdict.
