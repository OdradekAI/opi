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

#### Normalized regression finding

_Include for each confirmed FAIL, ERROR, or cross-version regression signal._

```yaml
id: <source-stable identifier>
source_kind: eval
source_path: docs/eval/<version>-<date>-<model-short>.md
source_model: <evaluator provider:model>
independence: <independent-family | fresh-context-same-family | unknown>
axis: runtime-fidelity
severity: <Blocker | Major | Minor | Info>
title: <short title>
claim: <falsifiable runtime regression>
evidence:
  - location: <trace event or artifact path>
    detail: <observed evidence>
criterion_source: <test-case criterion or null>
reproduction: [<eval case or exact command>]
confidence: <high | medium | low>
status: unverified
```

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
- Evaluator: <subagent type> on <evaluator provider:model>
- Independence: <independent-family | fresh-context-same-family | unknown>
```

---

## Naming convention

Filename: `<version>-<date>-<model-short>.md`

Examples:
- `0.7.2-2026-07-07-anthropic-claude-sonnet-4.md`
- `0.7.2-2026-07-07-openai-gpt-4o.md`

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
