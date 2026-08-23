---
name: opi-audit
disable-model-invocation: true
description: Independently audit one registered Opi Phase at committed HEAD against the latest committed specifications, then publish one validated active assurance set. Use only when the user explicitly invokes opi-audit.
---

# Opi Audit

Run a fresh-context, read-only conformance audit of one registered Phase. The
Phase selects scope; the latest committed normative sources select expected
behavior. Prior audit and remediation conclusions are never evidence or input.

## Inputs and outputs

Require `phase=<N>`. Accept optional narrow `focus=<area>` as an ordering hint;
it never reduces Phase proof obligations.

The only audit outputs are these fixed paths:

```text
docs/snapshots/phase<N>/assurance/audit.meta.json
docs/snapshots/phase<N>/assurance/audit.requirements.jsonl
docs/snapshots/phase<N>/assurance/audit.findings.jsonl
docs/snapshots/phase<N>/assurance/audit.md
```

A later run replaces this group after materialization. Git history retains the
previous set. Do not create timestamped/model-named siblings or a second audit
ledger. Do not edit production code, tests, specifications, or
`.opi-impl-state.json`.

## Required references

Read these current sources after rotation admission:

1. `../../../AGENTS.md` and `../../../README.md` for repository authority;
2. `../_shared/references/audit-set-contract.md` for baseline, schemas,
   publication, and rotation;
3. `references/audit-proof-obligations.md` for the requirement matrix and
   anti-vacuity rules;
4. `../_shared/references/finding-contract.md` and
   `references/finding-template.md` for findings and report output.

Then read the committed root state, pointed Phase state, latest committed
`docs/opi-spec.md`, and every currently registered supplemental source. Load
implementation files only when a sealed proof obligation points to them.

## History and context boundary

The run must begin without prior audit/remediation conclusions in context. If
the current task already contains old report contents or conclusions, stop with
`AUDIT-INCOMPLETE: history-contaminated context` and request a fresh audit task.

Never read, search, diff, summarize, or compare:

- Phase-level `audit*` or `remediation*` artifacts;
- any existing `docs/snapshots/phase*/assurance/` content;
- Git history of those artifacts;
- conclusions copied into chat, scratch files, or another ledger.

Path existence and Git cleanliness are allowed only through the `rotation`
validator. If a broad search exposes an old conclusion, invalidate the run and
restart in fresh context. There is no post-seal comparison with prior results, recurrence
annotation, or consensus pass.

## Workflow

### 1. Admit rotation before reading requirements

Run:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase<N>
```

Refuse on any legacy audit/remediation sibling or on staged, unstaged, or
untracked active-set change. This proves the prior set and fixes are
materialized before overwrite without loading their content.

### 2. Seal the committed baseline

Record the full `audit_head` from `git rev-parse HEAD`, actual reviewer/model
identity, independence class, and a unique `audit_run_id` beginning with
`phase<N>-`.

Read the root ledger and pointed Phase state from `audit_head`. Resolve the
registered source paths, then use the contents currently committed at those
paths. Hash exact bytes for:

- root `.opi-impl-state.json`;
- `docs/snapshots/phase<N>/opi-impl-state.json`;
- latest committed `docs/opi-spec.md`;
- every currently registered supplemental source.

The metadata records the exact `requirements_sha256` and `findings_sha256` for
the two JSONL sidecars.

If a stored source hash differs, record the mismatch as current metadata
evidence; do not substitute the older revision. Missing or malformed registered
scope is `AUDIT-INCOMPLETE`.

Export `audit_head` to a unique temporary directory with `git archive` and run
all source inspection and commands there. Do not use a Git worktree. Do not use
uncommitted tracked files from the live worktree as evidence. Keep generated
assurance files in a separate unique temporary staging directory.

Any HEAD drift before publication invalidates the run.

### 3. Seal requirements before implementation inspection

Derive every independently decidable mandatory and optional obligation from
the committed state and registered specifications. Write the complete
requirement identities, source hashes/citations, and observable behaviors to
staged `audit.requirements.jsonl` before reading production implementation.

After sealing the set, fill only evidence surfaces, checks, state, and reciprocal
finding IDs. Do not add, merge, or remove requirements in response to what the
implementation happens to contain. Use:

- `met`
- `partially-met`
- `not-met`
- `not-assessable`

Unavailable, contaminated, skipped, or non-discriminating mandatory evidence
is `not-assessable`, never `met`.

Every mandatory requirement that is `partially-met`, `not-met`, or
`not-assessable` must create a reciprocal blocking finding. State that finding
link explicitly whenever reporting the requirement state and verdict.

### 4. Inspect the complete committed implementation

Run Standards and Spec as separate axes. Also inspect security/authority,
invariants, integration, test quality, residuals, and minimum-change
conformance. Trace declarations through production consumers, durable formats,
adapters, commands, and tests.

Execute the narrowest sufficient checks in the exported tree. A passing command
counts only when its assertion can fail if the claimed behavior is absent.
Apply the anti-vacuity checklist. Before retaining a Blocker or Major finding,
search current counter-evidence and alternate paths and record why they do not
refute the claim.

### 5. Write current findings and derive the verdict

Write only evidence observed at `audit_head`. Every finding uses
`(audit_run_id, id)`, the fixed report path, reciprocal requirement IDs, and a
conformance effect. Do not reuse an old claim or preserve lineage.

Derive the verdict mechanically:

- `FAIL` when any mandatory requirement is not `met`;
- `PASS-WITH-FINDINGS` when all mandatory requirements are `met` and a
  non-Info finding exists;
- `PASS` when all mandatory requirements are `met` and no non-Info finding
  exists.

Severity ranks urgency but cannot override requirement state. A blocking
finding cannot link a `met` requirement.

### 6. Stage, validate, and publish atomically

Write all four fixed basenames in the temporary staging directory. Hash the
raw JSONL bytes into `audit.meta.json`; repeat exact Audit run ID, Audit head,
and Verdict headers in `audit.md`. Run `audit-set` against staging.

If validation fails, report `AUDIT-INCOMPLETE`, publish no verdict, and leave
the prior active set unchanged. Repair and revalidate the temporary staging set
or discard it; never publish or retain a partial staged set as the active set.

After staged validation passes, replace the four live audit files and remove
the four fixed remediation files in one controlled change without reading the
old contents. Validate the live directory again:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```

Publish the verdict only after the second validation passes. Never run two
audits concurrently against the same repository.

## Completion criterion

Complete only when `audit_head` is unchanged, all registered requirements have
states, all reciprocal links and raw-byte digests validate, the verdict is
derived rather than asserted, the live set passes `audit-set`, and no
non-assurance file changed. Otherwise report `AUDIT-INCOMPLETE` and the exact
blocking evidence.
