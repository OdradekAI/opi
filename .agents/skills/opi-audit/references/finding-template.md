# Audit report template

Create immutable siblings:

- `audit.<model>.<head7>.<run-id>.md`
- `audit.<model>.<head7>.<run-id>.findings.jsonl`

The JSONL sibling is the source of truth for normalized findings. The Markdown
report references finding IDs and adds context; it must not duplicate a second
editable copy of the machine records.

```markdown
# Phase <N> Audit

**Audit head**: `<full committed SHA>`
**Reviewer/model**: <reported identity>
**Independence**: <class + rationale>
**Run ID**: <immutable run identity>
**Contamination**: <none, or dirty-path isolation details>
**Verdict**: PASS | PASS-WITH-FINDINGS | FAIL

## Requirement Conformance

| Requirement | Criterion source | Evidence | Requirement state | Finding IDs |
|---|---|---|---|---|
| ... | ... | ... | `met` / `partially-met` / `not-met` / `not-assessable` | ... |

## Standards Review

<standards evidence and findings>

## Spec Review

<spec evidence and findings>

## Security, Invariants, Integration, Test Quality, and Residuals

<evidence grouped by the owning axis>

## Minimum-change Conformance

| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |
|---|---|---|---|---|---|---|---|

## Findings

### <finding ID>: <title>

- Axis: `standards` | `spec` | `security` | `test-quality` | `invariants` | `integration` | `residuals`
- Severity: Blocker | Major | Minor | Info
- Claim: <falsifiable problem statement>
- Evidence: <locations and observed details>
- Criterion: <registered rule or none>
- Refutation attempted: <counter-evidence and result; required for Blocker/Major>
- Suggested closure: <behavioral outcome, not a prescribed patch unless necessary>

## Verification Commands

| Command | Result | Obligation/finding |
|---|---|---|
| ... | PASS / FAIL / NOT RUN | ... |

## Verdict Rationale

<derive the verdict from mandatory Requirement state values, independently of severity>
```

When there are no findings, create an empty sidecar. Do not invent an
informational finding merely to make the file non-empty.
