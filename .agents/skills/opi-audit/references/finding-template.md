# Active Audit Set Template

Create these four files for this peer in its private member directory, then
let `assurance_set.py` validate the member and install it into the live
indexed assurance set:

```text
audit.<reviewer-id>.<model-id>.meta.json
audit.<reviewer-id>.<model-id>.requirements.jsonl
audit.<reviewer-id>.<model-id>.findings.jsonl
audit.<reviewer-id>.<model-id>.md
```

Use the exact JSON schemas from
`../../_shared/references/audit-set-contract.md` and
`../../_shared/references/finding-contract.md`. An empty findings sidecar is
valid; do not invent an Info finding to make it non-empty. Every file must use
LF line endings only.

The Markdown report references machine record IDs and adds explanation. It is
not a second editable copy of the JSONL records. The template below is the
independent core; staged input to `complete` must not contain a historical
appendix or its reserved envelope.

```markdown
# Phase <N> Audit

**Audit run ID**: `<phase-scoped run identity>`
**Audit head**: `<full committed SHA>`
**Reviewer ID**: `<reviewer-id>`
**Model ID**: `<model-id>`
**Reviewer identity**: <human-readable reviewer/runtime identity>
**Reviewer model ID**: `<exact runtime model identity, or the operator-supplied model slug by default>`
**Model identity source**: runtime-attested | request-config | operator-declared
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

If member validation or installation fails, do not copy any staged file
manually and do not put a PASS/FAIL set verdict in the user-facing completion
message. Report `AUDIT-INCOMPLETE` with the validator errors and, on
interruption, run `assurance_set.py recover` before retrying. No member may
read a sibling peer report during the independent stages.

After successful `complete`, use the
[shared historical-reference contract](../../_shared/references/audit-set-contract.md#post-audit-historical-reference)
to compile comparison JSON and run `append-history`. The tool adds the terminal
section **Post-audit historical reference — advisory** and its core-bound
envelope; do not render or edit that section by hand. Record a bounded
`no-history` result when appropriate. If the supplement cannot be completed,
preserve the installed core and report `HISTORY-SUPPLEMENT-INCOMPLETE` separately
from its unchanged verdict. The full workflow is complete only after a validated
appendix is installed; existing reports without one remain valid inputs.
