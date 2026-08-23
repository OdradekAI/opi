# Active Audit Set Contract

Each Phase has at most one active assurance set:

```text
docs/snapshots/phase<N>/assurance/
├── audit.meta.json
├── audit.requirements.jsonl
├── audit.findings.jsonl
└── audit.md
```

Remediation may add the four fixed siblings documented in
`remediation-disposition-contract.md`. A later audit replaces the audit group
and removes the remediation group. Git history, not sibling filenames, retains
superseded sets.

## Independent input boundary

`phase=<N>` locates the root `.opi-impl-state.json`, the pointed Phase state,
and registered scope. The audit verdict evaluates the implementation at the
committed `audit_head` against the latest committed `docs/opi-spec.md` and each
currently registered supplemental source. A stored historical source hash is
metadata evidence, not permission to substitute an older source revision.

Before reading requirements, run `rotation` against the Phase directory. It
inspects Git path state only. It never reads the previous audit or remediation
content. Audit must not read, search, summarize, or compare any earlier
`audit*`, `remediation*`, or `assurance/` content.

Inspect tracked source and run checks in an isolated temporary export of
`audit_head`. Do not use uncommitted tracked files from the live worktree as
audit evidence. The live worktree receives only a staged and validated four-file
replacement.

## `audit.meta.json`

The run root contains:

```json
{
  "schema_version": 1,
  "audit_run_id": "phase17-136c380-20260824t010203z",
  "phase": 17,
  "audit_head": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "reviewer_model": "reported reviewer identity",
  "independence": "fresh-context-same-family",
  "baseline_policy": "latest-committed-spec",
  "baseline_sources": [
    {"path": ".opi-impl-state.json", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
    {"path": "docs/snapshots/phase17/opi-impl-state.json", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
    {"path": "docs/opi-spec.md", "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}
  ],
  "requirements_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "verdict": "FAIL"
}
```

Hash each baseline source and each JSONL sidecar over its exact committed/raw
bytes with SHA-256. `audit.md` repeats the exact `Audit run ID`, `Audit head`,
and `Verdict` headers so human and machine views cannot drift.

## Requirement records

Write one JSONL record per independently decidable requirement before reading
production implementation:

```json
{
  "audit_run_id": "phase17-136c380-20260824t010203z",
  "id": "P17-A1",
  "mandatory": true,
  "criterion_source": {
    "path": "docs/opi-spec.md",
    "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
    "citation": "P17-A1"
  },
  "observable_behavior": "The registered behavior is present.",
  "production_surfaces": ["crates/opi-agent/src/lib.rs"],
  "test_evidence": ["phase17_api_audit"],
  "checks": [{"command": "cargo test -p opi-agent phase17_api_audit", "observed": "PASS"}],
  "state": "met",
  "finding_ids": []
}
```

State is `met`, `partially-met`, `not-met`, or `not-assessable`. Missing or
contaminated mandatory evidence is `not-assessable`, never `met`. Every
non-met mandatory requirement has at least one reciprocal finding.

## Verdict derivation

```text
FAIL                if any mandatory state is not met
PASS-WITH-FINDINGS  if all mandatory states are met and a non-Info finding exists
PASS                if all mandatory states are met and no non-Info finding exists
```

Severity ranks urgency; the requirement state determines mandatory
conformance. The finding conformance-effect rules prevent a blocking finding
from coexisting with a linked `met` requirement.

## Publication and rotation

Generate the four audit files in a unique temporary directory and validate
them there. On failure, report `AUDIT-INCOMPLETE`, publish no verdict, and keep
the prior committed active set unchanged. On success, replace the four audit
files and remove the fixed remediation group in one controlled change, then
validate the live set again.

`rotation` passes only when the prior active set is absent or completely
tracked and clean at HEAD, and no legacy Phase-level `audit*` or `remediation*`
path exists in HEAD, the index, the worktree, or Git status. It examines names
and Git state, not old report contents. This requires fixes and assurance
artifacts to be materialized before another audit overwrites them.

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase<N>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```
