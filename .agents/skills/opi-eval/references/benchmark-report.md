# Native benchmark report

Use for `benchmark`, `acceptance`, and `verify`. Native JSON remains owned by
Companion `report`; write the human summary from that verified output and
preserved artifacts. Use `report-template.md` only for fidelity canaries.

Choose a unique stem
`<UTC-date-time>-<software-versions>-<model-short>-<run-id>` with filesystem-safe
characters. Write `docs/eval/<stem>.md` and `<stem>.zh.md`, with counterpart
links. Preserve the exact full identities in the report even when the filename
uses abbreviations. Never overwrite an earlier report. For an independent
operator directory without a checkout, keep both reports beside the evidence
and name their intended `docs/eval/` handoff paths.

Start with the result and paths, then fill the following sections. Missing
evidence yields `blocked`/`incomplete` with reasons; omit unsupported scores.
No native run is appended to the case-based fidelity `history.jsonl` schema.

## Run and evidence identity

- Mode and operator scope; `completed`, `incomplete` or `blocked` execution;
  `verified`, `rejected` or `not-run` evidence checks; for acceptance only,
  `ready-for-offline-review` or `blocked` handoff.
- Evidence-root path, manifest SHA-256 where applicable, machine-report and
  verification-report paths; link exact artifacts for claims.
- Tested software/version, full source commits, dirty state, executable/runtime
  and dependency hashes; Companion version/build/digest recorded separately.
- WSL/Linux/Docker/platform/image identities and observed resources.
- Requested and actual provider/model/API/endpoint, effective controls and
  origin/unsupported reasons; no credentials or secret excerpts.
- Experiment/plan/material identities, full/subset selection and reasons,
  repetitions/seeds, total planned trials, concurrency and actual enforced
  limits. Disclose unenforceable budget requests and any explicitly revised
  scope without implying guaranteed spend.

## Native outcomes and coverage

One row per benchmark and subject; copy native metric definitions and counts
from verified JSON. Never average different benchmarks into a headline score.

| Benchmark/revision | Subject/version | Scope | Native metric/value | Denominator | Corpus/selected | Planned/attempted | Valid/Agent failure | Invalid/unexecuted |
|---|---|---|---|---|---|---|---|---|
| <observed> | <observed> | <full/subset> | <native meaning> | <graded count> | <counts> | <counts> | <counts> | <counts> |

Keep native zero and benchmark-defined valid Agent timeouts in the appropriate
denominator. Separate provider, access, infrastructure, adapter, identity,
invalid-grading and effect-unknown outcomes. Show all missing comparison sides,
excluded pairs/reasons and repetition control pairs. A six-trial smoke is not
a statistical superiority claim or a full benchmark result.

## Trial traceability

| Trial / family / task / repetition | Subject / actual job | Settlement / native outcome | Tool loop / task effect | Official handoff / verifier input | Bundle / artifacts |
|---|---|---|---|---|---|
| <actual identity> | <actual attribution> | <typed result> | <artifact pointers> | <identity and artifact pointers> | <path and digest> |

Include every planned/attempted trial, failed attempts and replacement links.
For acceptance, identify the exact six selected rows separately while keeping
the complete attempt history visible. Copying a manifest's claim is not a
traceability check; cite the originating native records.

## Usage, time and cost

Record per-trial wall/critical-path time, provider/tool calls, retry/compaction
and token/cache observations when available, with evidence origin. Missing
telemetry is `unknown` plus reason, never zero. Keep estimates, provider-reported
usage and billed cost distinct. An unexplained measurement gap limits the
associated claim; it does not manufacture an Agent outcome.

## Verification and privacy

| Check | Exact command / cwd | Exit / result | Evidence |
|---|---|---|---|
| Diagnose / prepare / validate / freeze | <executed> | <actual> | <logs/identities> |
| Native run, each family | <executed> | <actual> | <settlements> |
| Original seal verification | <executed> | <actual> | <JSON> |
| Offline report A/B/original equality | <executed> | <actual> | <hashes and byte comparison> |
| Acceptance verifier, if applicable | <executed or not-run reason> | <actual> | <JSON> |
| Opi scoped audit, if applicable | <executed or not-run reason> | <actual, non-empty coverage> | <JSON and capture binding> |
| Privacy/canary review | <scope and method> | <pass/blocked> | <safe record, no secret bytes> |
| Native regrade, only if requested | <executed or not-requested> | <actual> | <derived records> |

## Environment cleanup

Apply [environment cleanup](environment-cleanup.md) before final handoff.
Report its status and inspected scope, link the before/after inventories and
action record, and summarize removed resources, measured reclaimed space (or
`unknown`), retained reusable materials/evidence and unresolved residue with
reasons. Include the post-cleanup preservation/check results. Keep this record
outside original sealed evidence; cleanup status cannot upgrade eval outcomes.

## Acceptance trace and remaining work

For the six-trial route, map `P19-AC-001`, the real portions of
`P19-AC-003`–`P19-AC-006`, and `P19-AC-012` to exact artifacts, as
`evidenced`, `missing` or `rejected`. List any required broader/native witness
or workspace gates as unverified here unless actually run and preserved.
Return the root for independent task 19.11 verification; do not announce Phase
exit or write the implementation ledger from an operator report.

For confirmed diagnostic findings, use the shared finding vocabulary with
`source_kind: eval`, `axis: runtime-fidelity`, `status: unverified`, source
identity, evidence and reproduction. Native graders own scores; an optional
readonly narrative reviewer is labelled diagnostic, with truthful
`evaluator_model` and `independence`, and cannot replace native grading or
silently install findings into the live audit set.

End with only the actionable gaps, safe retained locations and the exact next
operator/verifier action. All reported checks must have actual execution
evidence; a blocked preparation run still deserves a concise gap report.
