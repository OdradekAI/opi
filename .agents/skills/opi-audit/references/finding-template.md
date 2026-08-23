# Active Audit Set Template

Create these four files in a unique temporary staging directory, validate them,
then publish the fixed Phase paths together:

```text
audit.meta.json
audit.requirements.jsonl
audit.findings.jsonl
audit.md
```

Use the exact JSON schemas from
`../../_shared/references/audit-set-contract.md` and
`../../_shared/references/finding-contract.md`. An empty findings sidecar is
valid; do not invent an Info finding to make it non-empty.

The Markdown report references machine record IDs and adds explanation. It is
not a second editable copy of the JSONL records.

```markdown
# Phase <N> Audit

**Audit run ID**: `<phase-scoped run identity>`
**Audit head**: `<full committed SHA>`
**Reviewer/model**: <reported identity>
**Independence**: <class and rationale>
**Baseline policy**: latest-committed-spec
**Verdict**: PASS | PASS-WITH-FINDINGS | FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| ... | ... | current committed source / stored-hash mismatch |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| ... | ... | ... | met / partially-met / not-met / not-assessable | ... |

## Standards Review

<current audit_head evidence>

## Spec Review

<current audit_head evidence>

## Security, Invariants, Integration, Test Quality, and Residuals

<current evidence grouped by owning axis>

## Minimum-change Conformance

<current task evidence and status>

## Findings

### <finding ID>: <title>

- Axis: <axis>
- Severity: Blocker | Major | Minor | Info
- Conformance effect: blocks | advisory
- Requirement IDs: <IDs>
- Claim: <falsifiable problem>
- Evidence: <locations and observed details>
- Refutation attempted: <required for Blocker/Major>
- Suggested closure: <behavioral outcome>

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| ... | PASS / FAIL / NOT RUN | ... |

## Verdict Rationale

<derive from mandatory requirement states and actionable current findings>
```

If staging validation fails, do not copy any staged file to the Phase and do
not put a PASS/FAIL verdict in the user-facing completion message. Report
`AUDIT-INCOMPLETE` with validator errors.
