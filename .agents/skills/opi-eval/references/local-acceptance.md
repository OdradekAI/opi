# Six-trial local acceptance

Read for task 19.11 / `mode=acceptance`, and when verifying an existing
`opi-eval-local-acceptance/1` root. The registered
[delivery specification, sections 7–8](../../../../docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md)
owns acceptance meaning; task 19.11 in `.opi-impl-state.json` owns the current
handoff requirement. This reference produces evidence, not task completion.

Real execution is a separately authorized operator activity outside the
`opi-implement` harness. That harness may only consume redacted evidence offline;
invoking this skill from it does not waive its ban on provider calls or real
auth-file reads. Finish non-live preparation and hand off the operator scope
if the current execution context cannot perform the authorized live run.

## Fixed acceptance scope

| Family | Subjects per same-task pair | Required native evidence |
|---|---|---|
| `terminal-bench-2.1` | Opi and pi | Actual task-side tool effects and grading of that trial environment. |
| `terminal-bench-3.0` | Opi and pi | Declared collection and the task's separate-verifier handoff. |
| `deepswe-v1.1` | Opi and pi | Base revision, committed patch, collection and pristine-verifier attribution. |

Subject IDs are experiment-local labels, including fresh replacement IDs.
The verifier derives each subject's product from its sealed experiment and
checks the trial assignment and experiment identity against run-control.
Renaming a subject never permits a different Agent to claim its result.

The first connection is provider `zhipu`, model `glm-5.3-flash`, API
`anthropic-messages`, base URL `https://open.bigmodel.cn/api/anthropic`.
This is the registered acceptance target, not a claim about current account
access. Both Agents use that same connection. Other models, endpoints, one
software or fewer families can be ordinary benchmark evidence but cannot close
this acceptance requirement. Do not silently switch modes on conflicting input.

Freeze one locally feasible task per family and one repetition per subject:
three family plans, two trials each, six total. Set `max_trials = 2` per family
and show the combined count before spending. Default the proposed concurrency
ceiling to 1; preserve the actual chosen ceiling in evidence. The researched
candidates are `openssl-selfsigned-cert`, `batched-eval-parity`, and
`abs-module-cache-flags`; recommend them only after current pinned inventory,
lifecycle and resource checks. A different feasible task needs an explicit
selection/reason and must still exercise its family's required handoff.

Resolve an omitted Opi version from the current local working tree under
[Opi source selection](native-run.md#opi-source-selection); verify its actual
package version, source snapshot and staged executable against the admitted
client profile. An explicit version still selects that version, and an
arbitrary version string does not admit a new integration. Apply the
[model-control defaults](native-run.md#model-controls) unless overridden.
Preserve requested/effective controls, runner/task/
material/image identities, usage/time scope and operator authorization before
launch. Use [native run](native-run.md) for the concrete CLI steps.

## What counts

Each accepted trial must preserve a real selected Agent's provider/tool loop,
task-side effects, official handoff, attributable native grading and seals.
Valid native zero and benchmark-valid Agent failure can count; positive score,
statistical superiority, full-corpus completion and eighteen protocol trials
are not requirements. Provider/auth, infrastructure, missing tool loop,
identity mismatch, invalid handoff and fixture/oracle-only results cannot count.

The current offline verifier consumes exactly six manifest trial rows, one
Opi and one pi per family. Preserve every prior failed attempt and replacement
relationship outside that accepted six-row selection, with their own original
roots, reports and identities. Do not remove a failure from a run root or
invent a successful exit to satisfy the verifier. If current replacement
layout/exit requirements prevent handoff, report the explicit blocker.

Run [evidence staging and offline checks](evidence-root.md). Check real tool
loops and native score attribution against preserved artifacts in addition to
the verifier's structural checks: its success alone does not establish model
access, task semantics, complete privacy scanning or every Phase criterion.
The Opi-only generic auditor also cannot verify pi/Harbor/Pier provenance or
substitute for cross-Agent/native evidence verification.

## Handoff boundary

Report `ready-for-offline-review` only when all six qualifying trials, command
records, digests, required artifact paths, privacy review, offline acceptance
verification and the non-empty exhaustive Opi capture audit pass. Otherwise
report `blocked` with trial-level reasons and keep all live acceptance gaps open.

Write the operator's bilingual conclusion report in `docs/eval/` under the
[benchmark report](benchmark-report.md) deliverable shape; the full report
stays beside the external evidence root. Return the evidence root
and its `manifest.json` SHA-256 so the user can separately resume
`opi-implement 19.11`, and append the staged root's record to
`docs/eval/evidence-roots.jsonl` as the [evidence-root
reference](evidence-root.md#evidence-root-registry) requires. That workflow
owns the final workspace verification union and
`docs/audits/phase19-local-native-acceptance.md` / `.zh.md`, verifies the
supplied evidence offline, and retains its independent Phase archive gate.
Do not write its ledger, final audit or archive status from this skill.
