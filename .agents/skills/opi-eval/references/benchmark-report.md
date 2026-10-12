# Native benchmark report

Use for `benchmark`, `acceptance`, and `verify`. Native JSON remains owned by
Companion `report`; write the human summary from that verified output and
preserved artifacts. Use `report-template.md` only for fidelity canaries.

Choose a unique stem
`<UTC-date-time>-<software-versions>-<model-short>-<run-id>` with filesystem-safe
characters. Keep bilingual reports and machine evidence under the external
evaluation's `reports/` directory. Configured operations return operator
summaries `report.md`, `report.zh.md` and `report.json`; their native records and
checks remain authoritative. Preserve these generated artifacts unchanged and
write any fuller human interpretation as fresh sibling reports using the sections
below. A summary cannot prove missing checks. Preserve the exact full
identities even when a filename uses abbreviations. Each evaluation has a fresh
dated root; never overwrite an earlier evaluation's report.

When a configured execution finishes, the operator automatically exports a
full bilingual benchmark report to the project's
`docs/eval/<stem>.md` and `<stem>.zh.md`: the locked identity block
(provider:model@api, thinking/output-token controls, corpus commit, runner
and Companion digests), coverage, the native metric, per-task outcome and
reward tables, and, for Opi candidates, the paired comparison against the
selected pi baseline (per-task deltas, pairing coverage). When the optional
`storage.exports` root is configured (for example a `/mnt/<drive>/…` path),
the same completion step copies the compact machine-evidence set (result
handoff, generated reports, snapshots, comparisons, native machine reports)
to `<exports>/<evaluation-id>/` so Windows-side consumers reach it without
WSL path knowledge. The export is best-effort: failures record a typed gap
in `export-handoff.json` and never change execution results. It is not an
execution prerequisite or a catalogue update; sealed evidence and
verification output stay external.

Start with the result and paths, then fill the following sections. Missing
evidence yields `blocked`/`incomplete` with reasons; omit unsupported scores.
No native run is appended to the case-based fidelity `history.jsonl` schema.
For configured baseline/candidate operations, link the original `handoff.json`,
snapshot and comparison manifests from [the operator](operator.md). Describe
baseline selection as `matched`, `missing` or `incomparable`, with its frozen
identity, freshness, selection/storage status and eligible coverage. Historical
score validity stays separate from those current selection dimensions. A
successful standalone candidate does not imply that a pi comparison was possible.

## Run and evidence identity

- Mode and operator scope; `completed`, `incomplete` or `blocked` execution;
  `verified`, `rejected` or `not-run` evidence checks; for acceptance only,
  `ready-for-offline-review` or `blocked` handoff.
- Evidence-root path, manifest SHA-256 where applicable, machine-report and
  verification-report paths; link exact artifacts for claims.
- External evaluation ID and record path, relevant import/lifecycle receipt,
  archived/active/missing storage state and pin/retirement decisions. Catalogue
  metadata locates originals; it never substitutes for original verification.
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
excluded pairs/reasons and repetition control pairs. Cross-run pairs against
a reused sealed pi baseline follow [sealed baseline](sealed-baseline.md):
label every pair cross-run with the baseline root and bundle digests, keep
repetition control pairs same-run only, and never present a reused baseline
as a same-run edge. A six-trial smoke is not
a statistical superiority claim or a full benchmark result.

## Trial traceability

| Trial / family / task / repetition | Subject / actual job | Settlement / native outcome | Tool loop / task effect | Official handoff / verifier input | Bundle / artifacts |
|---|---|---|---|---|---|
| <actual identity> | <actual attribution> | <typed result> | <artifact pointers> | <identity and artifact pointers> | <path and digest> |

Include every planned/attempted trial, failed attempts and replacement links.
Use the operator's frozen logical slots to report task/repetition/seed and all
attempts, selecting the first valid native settlement. Keep earlier failed
attempts and their consumption visible; do not sum valid retry results into a
larger benchmark denominator.
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
Include the rolling retention result: selected group, retained/latest count,
removed owned evaluations and protected extras with reasons. Three is the default
target, not a promise to remove pins, references, active work or unknown cleanup.
Link the returned `retention` pin and its `retention_status` (`applied`,
`protected_excess` or `refused`) separately from the frozen result handoff.
Standalone comparison output remains caller-owned and is not a deletion target.

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
