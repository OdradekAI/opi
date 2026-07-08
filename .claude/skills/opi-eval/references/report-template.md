# Report Template

Output format for eval reports written to `docs/eval/`. The skill runner fills
in the template and writes the result as
`docs/eval/<version>-<date>-<model-short>.md`.

---

```markdown
# opi eval -- <version> / <YYYY-MM-DD>

**Model**: <provider:model>
**Binary**: <git short hash> (release build)
**Duration**: <total wall-clock for all cases>
**Verdict**: <PASS | DEGRADED | REGRESSION>

---

## Summary

| Case | Correctness | Tools | Context | Efficiency | Resources | Errors | Overall |
|------|-------------|-------|---------|------------|-----------|--------|---------|
| <name> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> | <verdict> |
| ... | ... | ... | ... | ... | ... | ... | ... |

**Pass rate**: <X/Y dimensions passed> across <N> cases

---

## Detailed Findings

### Case: <name>

**Prompt** (truncated): <first 100 chars>...
**Expected**: <expected answer summary>
**Actual**: <what opi produced>
**Exit code**: <0 or error>
**Duration**: <ms>
**Tokens**: input=<N> output=<N>

#### Dimension analysis

| Dimension | Verdict | Evidence |
|-----------|---------|----------|
| Correctness | <V> | <evidence> |
| Tool calls | <V> | <evidence> |
| Context | <V> | <evidence> |
| Efficiency | <V> | <evidence> |
| Resources | <V> | <evidence> |
| Errors | <V> | <evidence> |

#### Runtime trace highlights

- Tool calls: <list or "none">
- Compaction: <triggered? reason, tokens_before -> tokens_after>
- Retries: <count, reasons>
- Diagnostics: <any startup or runtime diagnostics>

---

## Version Delta

_Present only when history.jsonl contains prior entries for comparison._

| Metric | Previous (<version>) | Current | Delta |
|--------|---------------------|---------|-------|
| Pass rate | X/Y | X/Y | +/-N |
| Total tokens (candy) | N | N | +/-% |
| Tool calls (tool_chain) | N | N | +/-N |
| Duration (total) | Nms | Nms | +/-% |

### Trend notes

- <observations about direction of change>

---

## pi Comparison

_Present only when docs/eval/pi-baseline.jsonl exists._

| Case | opi verdict | pi verdict | Notes |
|------|-------------|------------|-------|
| ... | ... | ... | ... |

---

## Environment

- OS: <platform>
- Rust: <rustc version>
- opi version: <semver>
- Commit: <full hash>
- Date: <ISO 8601>
- Evaluator: <subagent type and model if known>
```

---

## Naming convention

Filename: `<version>-<date>-<model-short>.md`

Examples:
- `0.6.5-2026-07-07-anthropic-claude-sonnet-4.md`
- `0.6.5-2026-07-07-openai-gpt-4o.md`

Rules:
- Version is the workspace semver from `Cargo.toml`
- Date is UTC `YYYY-MM-DD`
- Model-short: take `provider:model`, replace `:` with `-`, truncate to 40 chars

## Verdict definitions

| Verdict | Meaning |
|---------|---------|
| PASS | All dimensions pass across all cases. No regression signals. |
| DEGRADED | One or more dimensions scored DEGRADED, but no FAIL. Minor regression signals worth monitoring. |
| REGRESSION | One or more dimensions scored FAIL, or a case returned ERROR. Indicates a fidelity problem introduced by recent changes. |
