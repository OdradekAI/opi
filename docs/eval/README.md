# opi eval results

Every finished configured evaluation automatically exports its full bilingual
benchmark report here: `<UTC-date>_<action>-<families>_<evaluation-id>.md` plus
its `.zh.md` counterpart. The report carries the locked identity block
(provider:model@api, thinking and output-token controls, corpus commit, runner
and Companion digests), coverage, the native metric, per-task outcome and
reward tables, and for Opi candidates the paired comparison against the
selected pi baseline. A subset precision re-run whose controlled-variable
identity matches a full-scope evaluation folds itself into that evaluation's
merged report (sealed and merged metrics side by side, evidence chain
documented) instead of writing a separate file.

The reports are derived views. Authoritative evidence stays in the operator's
external workspace (`<workspace>/evaluations/YYYY/MM/<eval-id>/`: sealed trial
bundles, frozen inputs, sanitized command records, machine reports); when the
private profile configures `storage.exports`, compact machine-evidence copies
land in that Windows-accessible root (`<exports>/<evaluation-id>/`). See the
[`opi-eval` skill](../../.agents/skills/opi-eval/SKILL.md) for the contracts.
This directory is not an execution prerequisite, holds no database, and is
never a default evidence location. Generic canaries are runtime signals,
not deterministic acceptance evidence; public-seam tests and CI remain the
deterministic acceptance baseline.

## Historical note

The legacy machine-readable index `evidence-roots.jsonl` was removed; its
records (2026-09 era evidence roots, some since cleaned) remain available in
Git history and are referenced as prose by the historical audit documents.
The operator still accepts an external JSONL index through an explicit
`import-index --config <profile> --index <path>` request, read-only, without
copying or rewriting originals.

## Native benchmark and acceptance reports

Each new native evaluation writes bilingual and machine reports externally.
An explicit conclusion export uses one bilingual pair
`<UTC-date-time>-<software-versions>-<model-short>-<run-id>.md` and
`.zh.md`: outcome, scope, verified coverage counts, key identities, the
external record/evidence/report paths with digests, and remaining gaps. Full
operator reports, sealed evidence and verification output stay external; the
[report contract](../../.agents/skills/opi-eval/references/benchmark-report.md)
defines both artifacts.

The detailed operator reports written here between 2026-09-10 and 2026-09-29
were removed from the checkout on 2026-10-01 when this conclusion-pair
contract took effect. They remain readable in Git history at the removal
commit; frozen snapshots that reference them resolve them through that
history, and their evidence roots remain historical index entries.

Select software/version, benchmark families and model through `mode=benchmark`.
Use `mode=acceptance` for task 19.11's fixed Opi/pi six-trial scope or
`mode=verify evidence_root=<path>` for offline checks without new Agent calls.
The skill drives the Rust Companion's actual native workflow; native graders
own scores. The [report contract](../../.agents/skills/opi-eval/references/benchmark-report.md)
defines coverage, provenance, failure history, usage/time, privacy review and
exact verification results. `bench=all` groups selected families under one
external record and one first-dispatch deadline.

Historical score validity is independent of baseline freshness, retirement,
selection eligibility and storage availability. Finished operations retain the
latest three per Agent, benchmark combination, model/API/parameters/thinking and
task scope. Pi releases have independent buckets; Opi commits/versions and
Companion builds do not reset counts. Pins, references, active work and unknown
cleanup/dependencies can retain extra entries, which the handoff reports.
Completed preparation failures with grouped non-live proof and stopped offline
comparisons with confirmed command/stream cleanup count toward retention; unknown
or unsettled process cleanup stays protected. Failure summaries and separate
`retention-handoff.json` persist even when the CLI rejects without result JSON.
Manual archive/GC previews bind exact inventory and policy. Restore returns unchanged bytes to the
original canonical path without rewriting seals. Missing cached materials or
images remain offline capability gaps. Rolling retention covers only owned
evaluation trees; caller-owned standalone comparison output and shared caches
stay outside cleanup.

Acceptance reports link the external evidence root, its manifest SHA-256,
per-family native JSON, offline verifier result and exhaustive Opi-only capture
audit. They do not mark task 19.11 complete or substitute for its independent
audit/workspace gates. A blocked run produces a gap report without fabricated
trials. Native runs do not use the case-based `history.jsonl` schema below.

## Fidelity contents

- `history.jsonl` -- legacy append-only case history, one JSON object per line.
- `<version>-<date>-<model>.md` -- historical reports with per-case verdicts and
  runtime trace highlights.

New fidelity invocations keep reports/evidence and the same case history schema
in an explicitly selected external fidelity directory. This is caller-managed;
the baseline/candidate operator catalogue does not run canaries or append fidelity
history. Exporting a conclusion here requires a separate explicit request.

The cases in the fidelity catalogue are provider-fidelity canaries. They cannot
by themselves close a product criterion or Phase exit condition; the native
acceptance route above has its own registered requirements and evidence.

## history.jsonl schema

Each line is a self-contained JSON object:

```json
{
  "version": "0.8.0",
  "commit": "abc1234",
  "date": "2026-07-07",
  "model": "anthropic:claude-sonnet-4-5-20250514",
  "platform": "linux/x86_64",
  "run_mode": "json",
  "effective_tools": [],
  "cases": {
    "candy": {
      "case_id": "candy",
      "case_class": "provider-fidelity",
      "case_revision": 1,
      "criterion_source": null,
      "comparison_identity": "candy@1|anthropic:claude-sonnet-4-5-20250514|linux/x86_64|json|none",
      "comparison_status": "record-only",
      "verdict": "PASS",
      "tokens_input": 1200,
      "tokens_output": 450,
      "time_ms": 5600,
      "tool_calls": 0
    }
  },
  "overall": "PASS",
  "evaluator": "readonly-subagent",
  "evaluator_model": "openai:gpt-5.6",
  "independence": "independent-family"
}
```

### Field reference

Each per-case object records `case_id`, `case_class`, `case_revision`,
`criterion_source`, `comparison_identity`, and `comparison_status`.

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Workspace semver at time of eval; recorded but excluded from comparison identity |
| `commit` | string | Short git hash of the tested binary; recorded but excluded from comparison identity |
| `date` | string | ISO 8601 date (UTC) |
| `model` | string | Subject `provider:model` used for the eval |
| `platform` | string | Subject OS and architecture |
| `run_mode` | string | Opi run mode used by the case |
| `effective_tools` | array | Actual tool names enabled for the case |
| `cases` | object | Per-case results keyed by case id |
| `cases.<name>.case_id` | string | Stable case id |
| `cases.<name>.case_class` | enum | `provider-fidelity` or `runtime-fidelity` |
| `cases.<name>.case_revision` | integer | Positive semantic revision |
| `cases.<name>.criterion_source` | string or null | Registered criterion/scenario reference for runtime-fidelity cases |
| `cases.<name>.comparison_identity` | string | Case revision plus subject and environment identity |
| `cases.<name>.comparison_status` | enum | `comparable`, `incomparable`, or `record-only` |
| `cases.<name>.verdict` | enum | `PASS`, `DEGRADED`, `FAIL`, or `ERROR` |
| `cases.<name>.tokens_input` | number | Input tokens consumed |
| `cases.<name>.tokens_output` | number | Output tokens consumed |
| `cases.<name>.time_ms` | number | Wall-clock milliseconds |
| `cases.<name>.tool_calls` | number | Count of tool executions |
| `overall` | enum | `PASS`, `DEGRADED`, or `REGRESSION` |
| `evaluator` | string | Type of readonly evaluator subagent used |
| `evaluator_model` | string | Evaluator `provider:model` identity |
| `independence` | enum | `independent-family`, `fresh-context-same-family`, or `unknown` |

## Comparison rules

Compare history only when `case_id@case_revision`, subject provider/model,
OS/architecture, run mode, and effective tool set all match. Otherwise set
`comparison_status` to `incomparable` and omit percentage deltas.

Resource metrics are `record-only` by default. They become threshold-bearing
only for a registered performance budget or an explicitly enabled median from
at least three comparable prior samples. Do not create an empty
external `history.jsonl`; the first real eval creates it.

## Usage

To run an eval, invoke the `opi-eval` skill from any compatible agent
(Cursor, Claude Code, Codex). Example:

```
$opi-eval mode=fidelity model=anthropic:claude-sonnet-4-5-20250514
```

Results remain in the selected external fidelity directory. Request a repository
conclusion export explicitly when needed.
