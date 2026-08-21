# opi eval results

This directory stores real-provider fidelity results produced by the
`opi-eval` skill (`.claude/skills/opi-eval/SKILL.md`). Generic canaries are
runtime signals, not deterministic acceptance evidence; public-seam tests and
CI remain the acceptance baseline.

## Contents

- `history.jsonl` -- append-only log of eval runs, one JSON object per line.
  Used for trend analysis across versions.
- `<version>-<date>-<model>.md` -- individual eval reports with per-case
  verdicts and runtime trace highlights.

The currently registered cases are provider-fidelity canaries. There is no
registered runtime-fidelity acceptance case yet, so these canaries cannot by
themselves close a product criterion or Phase exit condition.

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
  "independence": "independent-family",
  "compaction_triggered": false,
  "retries": 0
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
| `compaction_triggered` | boolean | Whether any case triggered compaction |
| `retries` | number | Total auto-retry count across all cases |

## Comparison rules

Compare history only when `case_id@case_revision`, subject provider/model,
OS/architecture, run mode, and effective tool set all match. Otherwise set
`comparison_status` to `incomparable` and omit percentage deltas.

Resource metrics are `record-only` by default. They become threshold-bearing
only for a registered performance budget or an explicitly enabled median from
at least three comparable prior samples. Do not create an empty
`history.jsonl`; the first real eval creates it.

## Usage

To run an eval, invoke the `opi-eval` skill from any compatible agent
(Cursor, Claude Code, Codex). Example:

```
opi-eval model=anthropic:claude-sonnet-4-5-20250514
```

Results will be written here automatically.
