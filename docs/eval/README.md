# opi eval results

This directory stores regression evaluation results produced by the `opi-eval`
skill (`.claude/skills/opi-eval/SKILL.md`).

## Contents

- `history.jsonl` -- append-only log of eval runs, one JSON object per line.
  Used for trend analysis across versions.
- `<version>-<date>-<model>.md` -- individual eval reports with per-case
  verdicts and runtime trace highlights.
- `pi-baseline.jsonl` -- (future) baseline results from pi for comparison.

## history.jsonl schema

Each line is a self-contained JSON object:

```json
{
  "version": "0.7.0",
  "commit": "abc1234",
  "date": "2026-07-07",
  "model": "anthropic:claude-sonnet-4-5-20250514",
  "cases": {
    "candy": {
      "verdict": "PASS",
      "tokens_input": 1200,
      "tokens_output": 450,
      "time_ms": 5600,
      "tool_calls": 0
    },
    "tool_chain": {
      "verdict": "PASS",
      "tokens_input": 800,
      "tokens_output": 300,
      "time_ms": 8200,
      "tool_calls": 2
    },
    "context_retention": {
      "verdict": "PASS",
      "tokens_input": 2400,
      "tokens_output": 600,
      "time_ms": 7100,
      "tool_calls": 0
    }
  },
  "overall": "PASS",
  "evaluator": "cursor-subagent",
  "compaction_triggered": false,
  "retries": 0
}
```

### Field reference

| Field | Type | Description |
|-------|------|-------------|
| `version` | string | Workspace semver at time of eval |
| `commit` | string | Short git hash of the tested binary |
| `date` | string | ISO 8601 date (UTC) |
| `model` | string | `provider:model` spec used for the eval |
| `cases` | object | Per-case results keyed by case name |
| `cases.<name>.verdict` | enum | `PASS`, `DEGRADED`, `FAIL`, or `ERROR` |
| `cases.<name>.tokens_input` | number | Input tokens consumed |
| `cases.<name>.tokens_output` | number | Output tokens consumed |
| `cases.<name>.time_ms` | number | Wall-clock milliseconds |
| `cases.<name>.tool_calls` | number | Count of tool executions |
| `overall` | enum | `PASS`, `DEGRADED`, or `REGRESSION` |
| `evaluator` | string | Type of evaluator subagent used |
| `compaction_triggered` | boolean | Whether any case triggered compaction |
| `retries` | number | Total auto-retry count across all cases |

## Usage

To run an eval, invoke the `opi-eval` skill from any compatible agent
(Cursor, Claude Code, Codex). Example:

```
opi-eval model=anthropic:claude-sonnet-4-5-20250514
```

Results will be written here automatically.
