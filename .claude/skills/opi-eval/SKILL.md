---
name: opi-eval
disable-model-invocation: true
---

# opi-eval

End-to-end regression eval for the opi runtime. Compiles opi, runs structured
test cases against a real LLM provider, collects NDJSON runtime traces, and
dispatches an independent evaluator subagent to detect fidelity degradation.

## Inputs

```text
model=<provider:model>   # optional; defaults to user's configured default
cases=<name,...>         # optional; comma-separated case names, or "all" (default)
```

If no model is specified, use opi's default resolution. The model used is always
recorded in the report regardless of how it was resolved.

## Step 1: Build

Clean-compile the opi binary in release mode.

```powershell
cargo clean -p opi-coding-agent
cargo build --release -p opi-coding-agent
```

On Unix the binary is `target/release/opi`; on Windows `target/release/opi.exe`.

**Completion criterion**: both commands exit 0 and the binary file exists.

If the build fails, stop and report the error. Do not proceed with stale
binaries.

## Step 2: Execute test cases

Read `references/test-cases.md` for the full test case definitions. For each
selected case:

1. Create an isolated temp directory as the workspace for that case.
2. If the case requires fixture files (noted in its definition), create them in
   the temp workspace.
3. Run:

```
<opi-binary> --json --model <model> --no-builtin-tools "<prompt>"
```

   Or with tools enabled when the case requires tool access:

```
<opi-binary> --json --model <model> --allow-mutating "<prompt>"
```

4. Capture stdout (NDJSON) to `<temp>/output.ndjson`.
5. Record wall-clock duration and exit code.

**Completion criterion**: every selected test case has a corresponding
`output.ndjson` file and recorded exit code. Cases that crash (non-zero exit
without output) are marked `ERROR` rather than skipped.

## Step 3: Parse and extract

For each test case's `output.ndjson`, extract:

| Signal | Source event(s) |
|--------|----------------|
| Tool calls | `Agent.event.type == "ToolExecutionStart"` / `"ToolExecutionEnd"` |
| Tool call arguments | `ToolExecutionStart.args` |
| Tool call results | `ToolExecutionEnd.result`, `.is_error`, `.truncated` |
| Compaction | Top-level `CompactionStart` / `CompactionEnd` |
| Auto-retry | Top-level `AutoRetryStart` / `AutoRetryEnd` |
| Final answer | Last `Agent.event.type == "MessageEnd"` -> `message.content` |
| Token usage | `session_summary.tokens` |
| Cost | `session_summary.cost_usd` (if present) |
| Diagnostics | `session_summary.diagnostics`, `StartupDiagnostics` |

Produce a structured extraction (JSON or inline markdown) per case containing
the above signals.

**Completion criterion**: extracted signals exist for every case that produced
output. Cases marked `ERROR` get a minimal extraction noting the failure.

## Step 4: Evaluate

Dispatch a **readonly** evaluator subagent. The evaluator is independent of the
opi binary under test -- it receives only data and criteria.

Feed the evaluator:
- The test case definitions (from `references/test-cases.md`)
- The extracted runtime signals from Step 3
- The evaluation dimensions and scoring protocol from `references/evaluator-prompt.md`

Read `references/evaluator-prompt.md` now and include its full content as the
evaluator's task prompt, appending the runtime data.

The evaluator produces per-case verdicts across six dimensions and an overall
assessment. Wait for its response before proceeding.

**Completion criterion**: evaluator returns structured verdicts for all
dimensions on all evaluated cases.

## Step 5: Report

Write results to `docs/eval/`.

### Report file

Filename: `<version>-<date>-<model-short>.md`
- `version`: from workspace `Cargo.toml` version field
- `date`: `YYYY-MM-DD`
- `model-short`: provider and model name, colons replaced with dashes

Use the format from `references/report-template.md`.

### History log

Append one JSON line to `docs/eval/history.jsonl` with:
```json
{
  "version": "<semver>",
  "commit": "<short hash>",
  "date": "<YYYY-MM-DD>",
  "model": "<provider:model>",
  "cases": { "<name>": { "verdict": "<PASS|DEGRADED|FAIL|ERROR>", ... } },
  "overall": "<PASS|REGRESSION|DEGRADED>",
  "evaluator": "<subagent-type>"
}
```

### Delta analysis

If `history.jsonl` contains prior entries for the same or adjacent versions,
include a "Version Delta" section comparing key metrics (pass rate, token usage,
tool call count).

### pi comparison (reserved)

If a pi baseline file exists at `docs/eval/pi-baseline.jsonl`, include a
comparison section. Otherwise omit the section entirely.

**Completion criterion**: report markdown file written, `history.jsonl` updated.

## Evaluation dimensions

Six dimensions, applied to every test case:

### 1. Answer correctness

Does the final output solve the problem? Checked against the expected answer
defined in the test case. Scoring:
- PASS: correct answer present
- DEGRADED: partially correct or correct with extraneous errors
- FAIL: wrong answer or no answer

### 2. Tool call correctness

Were the right tools called with valid arguments? Were results handled properly?
- PASS: correct tools, correct args, results used appropriately
- DEGRADED: unnecessary extra calls, minor arg issues, unused results
- FAIL: wrong tools, malformed args, critical results ignored
- N/A: test case does not involve tools

### 3. Context integrity

Is information preserved across the conversation? Particularly after compaction.
- PASS: all relevant information retained and used
- DEGRADED: minor detail loss that did not affect the answer
- FAIL: critical information lost, answer affected

### 4. Chain efficiency

Is the execution path efficient? No dead loops, no redundant operations.
- PASS: direct path to solution
- DEGRADED: minor redundancy (1-2 unnecessary steps)
- FAIL: loops, repeated failures, excessive steps

### 5. Resource consumption

Token usage and timing within expected bounds for the task complexity.
- PASS: within 1.5x of expected baseline
- DEGRADED: 1.5x-3x expected
- FAIL: >3x expected or timeout

### 6. Error handling

Does the runtime handle errors gracefully?
- PASS: no errors, or errors handled with recovery
- DEGRADED: errors occurred but runtime continued correctly
- FAIL: crash, hang, or unhandled error corrupting output

## Guardrails

- This skill consumes real API credits. Never fire without user invocation.
- Always record the model in every output artifact.
- The evaluator subagent must be readonly -- it analyzes, never executes.
- Test fixtures use isolated temp directories. Never write fixtures into the
  workspace root.
- Do not commit results unless the user explicitly asks.
- If a test case fails to run (opi crashes), record it as ERROR and continue
  with remaining cases rather than aborting the entire eval.
- Do not modify opi source code. This skill observes and reports only.

## References

- Read `references/test-cases.md` for test case definitions (prompts, expected
  answers, fixture requirements, evaluation criteria).
- Read `references/evaluator-prompt.md` for the evaluator's full task prompt
  and scoring protocol.
- Read `references/report-template.md` for the output report format.
