# Fidelity cases

Read this file only for `mode=fidelity` or an explicit `cases=...` request.
Paths beginning `references/` or `../_shared/` below resolve from the skill
directory. Repository command paths resolve from the checkout. These
Opi-only canaries are separate from native benchmark trials and their scores.

Build Opi, execute authorized real-provider cases in isolated workspaces, and
give preserved NDJSON evidence to a readonly evaluator subagent. Public-seam
Rust tests and CI remain the deterministic acceptance baseline; generic
canaries cannot establish acceptance.

## Evidence authority

- `provider-fidelity` cases detect general model/provider regressions and do
  not cite Opi product criteria.
- `runtime-fidelity` cases cover a registered behavior that deterministic
  providers cannot faithfully reproduce. They must cite that criterion or
  acceptance scenario and state the remaining fidelity gap.
- Do not create a behavior-baseline manifest, copy production call sites into
  this skill, or generate eval cases from the implementation ledger.

Every case supplies metadata from `references/test-cases.md`. Historical
comparison is permitted only for the exact identity
`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`.

## Inputs

```text
model=<provider:model>   # optional; uses the skill's evaluation defaults
cases=<name,...>         # optional; comma-separated case names, or "all" (default)
```

Resolve omitted model and budget inputs using
[evaluation defaults](../SKILL.md#evaluation-defaults); explicit inputs or
user-supplied config values win. Pass the resolved model explicitly instead of
using opi's ambient default. Record the model, effective budget, timer and case
attempt allowance before live execution. The batch timer also covers evaluator
dispatch; evaluator model selection still follows Step 4's independence rules.

## Step 1: Build

Record resource ownership and protected locations using
`references/environment-cleanup.md` before builds or cases create scratch.

Respect an existing `CARGO_TARGET_DIR`; otherwise resolve it with
`python scripts/opi-cargo-cache.py resolve` and acquire a resolver-managed
lease for the current process. Read the helper's lease arguments and release
the lease in `finally`/`trap`. Keep incremental compilation enabled and run:

```text
cargo build --release -p opi-coding-agent
```

Preserve persistent Cargo caches; pruning is separate maintenance, outside
eval. Inspect cache state with `python scripts/opi-cargo-cache.py status`.

On Unix the binary is `$CARGO_TARGET_DIR/release/opi`; on Windows
`$env:CARGO_TARGET_DIR\release\opi.exe`.

**Completion criterion**: the build exits 0 and the resolved cached binary
exists. Build failure blocks execution; stale binaries cannot substitute.

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

4. Capture stdout (NDJSON) to `<temp>/output.ndjson`. Give each retry a fresh
   workspace/evidence identity and preserve its link to the failed attempt.
5. Record wall-clock duration and exit code.

Apply the frozen [retry allowance](../SKILL.md#evaluation-defaults) to failed
case execution before continuing. A completed case's quality verdict is not a
reason to rerun it until a better score appears. Preserve and report all attempts.

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

Dispatch a **readonly** evaluator subagent with criteria and evidence only.
Prefer an available different model family and record `independent-family`.
With the same family, use fresh context, record `fresh-context-same-family`,
mark the overall verdict `DEGRADED` and disclose self-grade risk. Unknown model
identity means `unknown` independence and `DEGRADED`; never infer an identity.

Feed the evaluator:
- The test case definitions (from `references/test-cases.md`)
- The extracted runtime signals from Step 3
- The evaluation dimensions and scoring protocol from `references/evaluator-prompt.md`

Read `references/evaluator-prompt.md` now and include its full content as the
evaluator's task prompt, appending the runtime data.

`references/evaluator-prompt.md` owns all six dimensions and scoring rules,
including `record-only` resources and comparable-cohort thresholds.

**Completion criterion**: receive structured verdicts for every dimension and
case before reporting the assessment.

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
  "platform": "<OS/arch>",
  "run_mode": "json",
  "effective_tools": ["<tool-name>"],
  "cases": {
    "<name>": {
      "case_id": "<name>",
      "case_class": "<provider-fidelity|runtime-fidelity>",
      "case_revision": 1,
      "criterion_source": null,
      "comparison_identity": "<case@revision + subject + environment>",
      "comparison_status": "<comparable|incomparable|record-only>",
      "verdict": "<PASS|DEGRADED|FAIL|ERROR>",
      "tokens_input": 0,
      "tokens_output": 0,
      "time_ms": 0,
      "tool_calls": 0
    }
  },
  "overall": "<PASS|REGRESSION|DEGRADED>",
  "evaluator": "<subagent-type>",
  "evaluator_model": "<provider:model of the evaluator>",
  "independence": "<independent-family|fresh-context-same-family|unknown>"
}
```

Record the actual `evaluator_model` and `independence` from Step 4.

### Delta analysis

Compare a case only with prior samples having the same comparison identity.
Mark all other prior samples `incomparable`; do not calculate or narrate a
percentage delta. Opi version and commit remain recorded but are intentionally
outside the identity because cross-version comparison is the purpose.

Preserve case NDJSON, inputs, effects, extractions, evaluator output and command
records outside disposable workspaces before applying
`references/environment-cleanup.md`. Keep failed-case evidence and link the
cleanup record in the report.

**Completion criterion**: report markdown file written, `history.jsonl` updated,
evidence retained, and cleanup completed or remaining residue explicitly recorded.

### Normalized regressions

For every confirmed `FAIL`, `ERROR`, or cross-version regression signal, append
the normalized YAML block from
`../_shared/references/finding-contract.md`. Use:

```text
source_kind = eval
axis = runtime-fidelity
status = unverified
```

The block cites trace events, report artifacts, and the eval case or exact
reproduction command. A source fix follows the
[user-confirmed repair branch](../SKILL.md#user-confirmed-source-repairs).
Only a later independent audit can admit reproduced eval findings into the
live audit set. `opi-remediate` does not ingest eval reports directly.

## Guardrails

- Resolve live scope, budget and stop conditions before spending API credits;
  reuse authorization only within that scope. Record the model in every output.
- Do not commit results unless the user explicitly asks.
- If a test case fails to run (opi crashes), record it as ERROR and continue
  with remaining authorized cases unless an authority, privacy or budget breach
  requires stopping affected execution.
- Opi source changes require the user-confirmed repair branch; the diagnostic
  evaluator subagent remains readonly.
