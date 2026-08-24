# Multi-Model Active Audit Design

## Status and authority

This design was approved on 2026-08-24. It is non-normative design input.
The implemented workflow contract remains owned by the shared assurance
contracts, the `opi-audit` and `opi-remediate` skills, their validators, and
the English and Chinese workflow documentation.

## Context

The current assurance contract gives each Phase one active four-file audit
set with fixed names. A later audit replaces that set, and remediation consumes
only that one audit. The report records a reviewer description in metadata,
but the fixed `audit.md` path cannot distinguish peer reviews produced by
different Agent runtimes and models.

Opi needs multiple independent auditors to review the same sealed repository
state concurrently, retain their reports as equal active inputs, and require
remediation to consume every active finding. No auditor's conclusion is a
canonical replacement for the others.

## Goals

- Keep one independently valid four-file group per reviewer/model pair.
- Make all configured groups equal active inputs to one Phase gate.
- Preserve auditor independence while allowing parallel execution.
- Bind every group to the same Phase, registered baselines, and Git head.
- Make membership explicit and fail closed on incomplete or inconsistent data.
- Let one remediation plan close the union of findings without losing source
  identity or disagreement.
- Preserve superseded generations as immutable evidence without using them as
  audit or remediation input.

## Non-goals

- Selecting or launching a provider model from a skill manifest.
- Voting, majority consensus, or synthesizing a canonical `audit.md`.
- Reading prior audit conclusions to influence a new independent audit.
- Deduplicating findings by title, severity, or prose similarity.
- Adding compatibility aliases for the old unsuffixed filenames.

## Considered layouts

Three layouts were considered:

1. Flat reviewer/model file groups plus an authoritative index.
2. Flat file groups discovered solely by filename globbing.
3. One subdirectory per reviewer/model pair.

The first layout is selected. It preserves human-readable names such as
`audit.codex.gpt56.md`, while the index prevents stale, partial, or accidentally
named files from silently becoming remediation inputs. Glob-only discovery
cannot distinguish intentional active membership from residue. Subdirectories
give a stronger physical boundary but do not satisfy the selected report
naming convention.

## Active artifact model

The active assurance directory contains one index and one complete group per
reviewer/model pair:

```text
docs/snapshots/phase<N>/assurance/
├── audit.index.json
├── audit.codex.gpt56.meta.json
├── audit.codex.gpt56.requirements.jsonl
├── audit.codex.gpt56.findings.jsonl
├── audit.codex.gpt56.md
├── audit.claude.glm53.meta.json
├── audit.claude.glm53.requirements.jsonl
├── audit.claude.glm53.findings.jsonl
├── audit.claude.glm53.md
├── remediation.plan.md
├── remediation.plan.dispositions.jsonl
├── remediation.result.md
├── remediation.result.dispositions.jsonl
└── history/
    └── <audit-generation-id>/
        ├── audit.index.json
        ├── audit.<reviewer-id>.<model-id>.*
        └── remediation.*
```

The remediation files remain one generation-wide group. They are present only
after remediation has begun. There is no unsuffixed active `audit.md` and no
single report with greater authority than its peers.

`audit.index.json` is the only active-membership authority. A matching audit
group that is not indexed is an orphan: it is not consumed, and active-set
validation fails rather than ignoring it silently. Files below `history/` are
never active inputs.

## Reviewer and model identity

The filename uses two stable, file-safe slugs:

- `reviewer-id` identifies the Agent or review runtime, such as `codex` or
  `claude`.
- `model-id` identifies the actual configured model, such as `gpt56` or
  `glm53`.

Both slugs match `^[a-z0-9][a-z0-9-]*$`; dots remain structural separators.
The pair must be unique within a generation. If two full model identifiers
would map to the same slug, publication is rejected until distinct slugs are
provided.

The metadata stores, at minimum:

```json
{
  "reviewer_id": "codex",
  "reviewer_identity": "Codex",
  "model_id": "gpt56",
  "reviewer_model_id": "gpt-5.6",
  "model_identity_source": "runtime-attested",
  "audit_generation_id": "phase17-20260824t010203z",
  "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z"
}
```

`model_identity_source` is `runtime-attested`, `request-config`, or
`operator-declared`. A workflow argument carrying `model-id` identifies the
model already selected by the runtime; it is not a skill-level model selector.
The workflow never infers an exact identifier from a model's self-description.
If no reliable or explicitly declared model identity exists, the review may
run diagnostically but cannot publish an active group.

`(reviewer-id, model-id)` names a long-lived active slot. `audit_run_id`
identifies one execution in that slot. Re-auditing the same slot replaces its
active run only as part of a complete new generation.

## Audit index

The active index has this logical shape:

```json
{
  "schema_version": 1,
  "phase": 17,
  "audit_generation_id": "phase17-20260824t010203z",
  "audit_head": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "revision": 3,
  "aggregate_verdict": "FAIL",
  "members": [
    {
      "reviewer_id": "codex",
      "model_id": "gpt56",
      "artifact_stem": "audit.codex.gpt56",
      "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z",
      "verdict": "FAIL",
      "digests": {
        "meta_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "requirements_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "findings_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "report_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
      }
    }
  ]
}
```

Members are sorted by `(reviewer_id, model_id)` for deterministic bytes. Every
member must have exactly four files. The filename, index entry, metadata, and
human-readable report must agree on reviewer/model identity, generation, run,
Phase, head, and verdict. Every member independently validates its complete
registered requirement scope and baseline hashes.

The aggregate verdict is mechanically derived:

```text
FAIL                if any member verdict is FAIL
PASS-WITH-FINDINGS  if no member is FAIL and at least one is PASS-WITH-FINDINGS
PASS                if every member is PASS
```

Missing members, invalid files, inconsistent heads, or an invalid index produce
`AUDIT-INCOMPLETE`, not a verdict. All active members belong to one generation
and one `audit_head`; reports from different repository states cannot be
combined.

## Independent parallel execution

A coordinator creates a staged generation by sealing one committed
`audit_head` and pre-registering the complete reviewer/model roster. Each
auditor receives only the registered sources, the sealed source export, its
own identity, and the generation identity. It must not read prior assurance
content or another auditor's staged or completed conclusions.

The staging manifest mirrors the intended roster and records each slot as
`pending`, `complete`, or `failed`. It is untracked workflow scratch, not a
durable artifact format or an active-membership source. Only the fully validated
active `audit.index.json` is committed.

Auditors inspect and generate their four-file groups in separate temporary
locations. Different reviewer/model slots may run concurrently. Two runs for
the same slot are rejected.

Publication occurs only after every registered group is complete and validates
in staging. Under a repository-scoped assurance lock, the publisher:

1. Revalidates the sealed head, roster, all groups, and the expected prior
   index digest.
2. Writes a recovery journal and copies the complete prior active generation
   and its remediation group to the new history directory.
3. Installs every new audit group and removes the superseded active files.
4. Writes `audit.index.json` last as the logical generation switch.
5. Validates the live active generation, finalizes history, and removes the
   recovery journal before releasing the lock.

Staging registry changes use a monotonic revision and compare-and-swap under
the same lock so one completion cannot erase another. Every assurance consumer
acquires the lock and performs journal recovery before reading active files. If
publication stops before the index switch, recovery restores the copied prior
generation. If it stops after the switch, recovery validates and completes the
new generation or restores the prior generation when validation fails. Thus an
interrupted new generation never becomes a silently partial remediation input.
If no prior generation exists, the Phase has no active verdict until the first
generation publishes completely.

Removing, adding, or changing a reviewer/model slot requires a new complete
generation. Direct deletion of an active group is invalid.

## Remediation aggregation

`opi-remediate` validates and consumes the active index plus every indexed
requirements and findings sidecar. It does not read history or choose one
reviewer as canonical.

Requirement evidence is represented as a reviewer/model matrix. Rows align on
the criterion identity `(path, sha256, citation)` when possible, while each
auditor's original record and state remain intact. Unaligned criteria remain
separate rows. The aggregate gate is determined by member verdicts, not by
overwriting the matrix with one consensus state.

Findings are the strict union of every member's findings. Their identity remains
`(audit_run_id, finding_id)`. Similar titles, shared closure keys, or matching
locations never merge source identities. One behavioral repair and one set of
verification commands may close several related findings, but the disposition
sidecar contains one source-bound record for each finding.

When reviewers disagree:

- The highest source severity controls scheduling priority.
- Every original severity and rationale remains unchanged in its audit group.
- A remediation disposition may revise final severity or mark a finding
  `Refuted` only with independent verification evidence.
- Another reviewer's contrary conclusion is not sufficient evidence to omit or
  close a finding.

The remediation plan records `Audit generation ID` and `Audit index SHA-256`
instead of one audit run ID and one findings digest. Each disposition still
records its source `audit_run_id`, `findings_sha256`, and finding ID. The plan
digest continues to bind the exact plan and disposition bytes. Any index,
member, sidecar, or roster change invalidates approval of the old plan.

## History and independence

On successful generation replacement, the prior index, every prior audit
group, and all prior remediation files move together to
`history/<audit-generation-id>/`. A history generation is immutable evidence.
It is never an input to a new audit, aggregate verdict, remediation plan, or
finding disposition.

Admission and rotation logic may inspect history path shape and Git cleanliness
without reading or comparing prior conclusions. Historical lineage,
prior-occurrence counters, and consensus-derived finding fields remain
forbidden.

## Failure behavior

The workflow fails closed in these cases:

- A filename, index entry, metadata record, or report header disagrees.
- A registered member or one of its four files is absent.
- A member uses a different Phase, generation, head, or baseline.
- Reviewer/model slots are duplicated or normalize ambiguously.
- An indexed digest differs from the exact file bytes.
- An active-style file is present but absent from the index.
- A publisher loses the index compare-and-swap or cannot acquire the slot lock.
- A remediation input or approved plan no longer matches the active index.

Audit-generation failures report `AUDIT-INCOMPLETE` and publish no new verdict.
Remediation validation failures make no production changes. Recovery repairs or
discards staging and retries; it never guesses membership, identity, or the
intended winning write.

## Migration

The old unsuffixed four-file group is a deliberate 0.x format break. The
implementation does not dual-read old and new active layouts.

Migration requires an explicit reviewer and model mapping. Existing evidence
that identifies only a model family, such as `Codex (GPT-5)`, cannot be renamed
to `gpt56` automatically. An operator may provide `model-id` and record
`model_identity_source: operator-declared`, or rerun the audit with an attested
identity. The migration then constructs and validates the first index. Existing
remediation artifacts are archived with their source audit rather than rebound
to new identities.

The implementation updates the `Unreleased` changelog because paths and the
durable assurance format are user-visible. English and Chinese workflow
documentation change together. The durable product specification currently
does not require a single active audit, so this workflow-contract change does
not require a product-scope amendment.

## Implementation surfaces

The implementation is expected to update only the assurance workflow surface:

- `_shared/references/audit-set-contract.md`.
- `_shared/references/remediation-disposition-contract.md`.
- `opi-audit` and `opi-remediate` skill instructions and applicable Agent
  metadata descriptions.
- `_shared/scripts/validate_assurance_artifact.py`.
- `scripts/opi-doc-check.py` and focused assurance/documentation tests.
- English and Chinese workflow README files.
- `CHANGELOG.md` under `Unreleased`.

No Rust runtime, Cargo manifest, provider adapter, or model-selection behavior
is part of this design.

## Verification

Focused fixtures and tests cover:

1. Two `PASS` members derive `PASS`.
2. One `PASS-WITH-FINDINGS` member derives `PASS-WITH-FINDINGS` when none fail.
3. One `FAIL` member derives `FAIL` regardless of other verdicts.
4. Missing, corrupt, orphaned, identity-mismatched, or cross-head members yield
   `AUDIT-INCOMPLETE`.
5. Two findings for one behavior share a repair but receive two dispositions.
6. Severity disagreement uses the highest scheduling priority without changing
   source records.
7. Different slots complete concurrently without lost index entries.
8. Same-slot concurrency and stale compare-and-swap publication are rejected.
9. An incomplete new generation leaves the prior generation active.
10. A complete switch archives the prior audit and remediation group together.
11. Any active-index change invalidates an approved remediation plan.
12. An old unsuffixed set is rejected unless explicitly migrated.

Run the focused Python test files changed by the implementation, including:

```text
python scripts/test_opi_assurance_skills.py
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

No Cargo gate is required unless implementation expands into Rust runtime code.
