# TB3 failure diagnosis — 2026-09-11

[中文](20260911t125241z-tb3-failure-diagnosis.zh.md)

The pi failure is explained by the outer native-job time budget. Opi failed
inside its provider stream; the preserved output does not identify the
underlying transport or parsing fault. This diagnosis does not establish a
provider-side outage or an Opi parser defect as the historical cause.

Scope: offline inspection of the
[prior TB3 rerun](20260911t001700z-opi-head-pi0851-glm53flash-tb3-rerun.md)
and nine synthetic IPv4-loopback probes using the exact evaluated Opi binary.
No paid provider calls, real credential reads, runtime edits or regrading.
Opi source HEAD was `55fbc95009e6f47145196dbc539454ebcb1cf692`; the frozen
binary SHA-256 was
`d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60`.
pi remained 0.85.1. Evaluator model identity and independence are `unknown`.

## pi: whole-job deadline interrupted an active agent

The pinned task allows 14,400 seconds for the agent and 900 seconds for the
verifier. The frozen Companion profile has `limits.timeout_secs = 900`, applied
to the **entire Harbor job**, including agent execution, collection and grading
([SpawnSpec](../../crates/opi-eval/src/benchmark/terminal_bench_30.rs#L1013)).
The supervisor terminates the process tree at that deadline. Thus the retained
short acceptance budget did not accommodate this task's native agent allowance.

At termination, pi had completed 16 model responses and 32 tool calls; its last
event was an unfinished tool-call delta. The native aggregate still reported one
running trial, zero completed trials and no finish time. No native score exists.

Two completed responses contained 20,719 and 10,989 output tokens. Their start
times to the next response start span about 779 seconds combined, including
intervening tools. These are not measurements of pure model latency, but show
why a 15-minute outer budget became restrictive with long generations. Completed
responses report 34,908 output tokens in total; interrupted-response usage is
unknown.

The error label is also misleading:
[TIMED_OUT](../../crates/opi-eval/src/benchmark/process.rs#L1178) always maps
to `verifier-timeout` at the grader boundary, although grading had not started.
The persisted settlement additionally has `timed_out: false`. Neither field
should be used to infer that the grader itself exhausted its timeout.

## Opi: stream failure before any task repair was delivered

Opi exited 4 after about 277 seconds of agent execution. Responses 1–9 completed;
response 10 emitted 5,895 thinking deltas totaling 26,769 Unicode characters,
then ended without `MessageEnd` or `TurnEnd`. Stderr was only
`provider stream failed`. This is not a recorded 50-turn or output-token-limit
stop. The incomplete response's usage is absent from the session totals.

The underlying adapter distinguishes HTTP body errors, malformed SSE events,
invalid UTF-8 and EOF without a terminal event
([stream handling](../../crates/opi-ai/src/anthropic.rs#L1060)). The diagnostic
retains a `details.provider_error` value
([mapping](../../crates/opi-agent/src/diagnostic.rs#L695)), but the CLI prints
only its generic message
([renderer](../../crates/opi-coding-agent/src/runner.rs#L982)). The session
summary exposes severity counts, not the error detail. Historical raw HTTP/SSE
bytes and a specific terminal diagnostic were not retained, so the exact
underlying cause cannot be recovered from this evidence.

| Synthetic case | Exact evaluated binary result |
|---|---|
| Normal completion | Exit 0 |
| 5,895 thinking deltas plus normal terminal events | Exit 0 |
| CRLF and UTF-8 split across small writes | Exit 0 |
| Clean HTTP EOF without a terminal SSE event | Exit 4, generic stream error |
| Malformed event JSON | Exit 4, generic stream error |
| Invalid UTF-8 frame | Exit 4, generic stream error |
| Truncated chunked HTTP body | Exit 4, generic stream error |
| Valid upstream SSE error event | Exit 4, but emits MessageEnd/TurnEnd |
| Final SSE delimiter absent | Exit 4, generic stream error |

These are controlled examples, not a replay of the unavailable historical wire
bytes. They disprove event count alone as the cause and demonstrate diagnostic
ambiguity. The valid upstream-error case differs from the historical event
sequence. The HTTP client's 90-second pool-idle setting is not an active-request
timeout.

Opi invoked 6 bash and 17 read tools. All **18 delivered task files match the
initial task files byte for byte**. The native verifier then ran and failed all
five tests, with reward 0; its traceback includes unresolved support-row lookup
errors. The zero reflects the unmodified task after the agent failed, rather
than evidence that a completed repair was judged incorrect. The paired benchmark
comparison remains incomplete because pi was not graded.

## Follow-up repair scope

1. Make the native-job budget explicitly cover the task's agent and verifier
   allowances plus setup/collection/cleanup, then freeze a new plan and material
   identity. Do not edit the historical plan or silently extend its budget.
   Verify with an offline supervisor fixture that agent and grader deadlines
   remain distinct and executable.
2. Correct whole-job timeout attribution and settlement flags. Verify the
   before-grader timeout fixture and normalized report fields.
3. Preserve safe typed stream error details in captured CLI/trace output. Verify
   EOF, malformed-frame and transport failures are distinguishable, with
   credential redaction and partial-output behavior covered. Diagnose the actual
   provider failure from a future authorized run with that evidence before
   choosing a transport/parser repair.

These are proposed changes, not changes made by this diagnosis. Eval findings
remain outside the active audit/remediation set.

## Evidence and checks

- Original WSL root:
  `/home/luiz/opi-eval-runs/20260911t001700z-tb3-rerun`.
- New evidence:
  `E:/opi/evaluations/20260911t125241z-tb3-diagnosis/evidence`.
  `diagnosis.json`, `probe-results.json`, `findings.jsonl`, source excerpts and
  per-case synthetic input/output are retained there.
- New evidence-index SHA-256:
  `5aa6cadbf80b8f8443fc9338453b4e51a861653ac5c3792f1ff6dd1a1dddbea1`.
- All 385 indexed prior evidence files still match; prior index SHA-256 remains
  `e83315c24911f8d3ac9325c5fcd29424a1789fc32d134a89db75463f9cb329a8`.
- `wsl -d Ubuntu -- python3 /mnt/e/opi/evaluations/20260911t125241z-tb3-diagnosis/probe.py`:
  exit 0; all nine expected outcomes passed. For reproduction, assign a fresh
  owned root in the script; it refuses to overwrite an existing root.
- `wsl -d Ubuntu -- python3 /mnt/e/opi/evaluations/20260911t125241z-tb3-diagnosis/collect.py`:
  exit 0; 18 unchanged task files, 385 historical files verified, 88 new evidence
  files indexed.
- Documentation verification: `python scripts/opi-doc-check.py` and
  `git diff --check`; results are retained beside the evidence under `checks/`.
- Test impact: **none** in repository tests; synthetic operator probes only.
  No Rust/Cargo files changed, so no compilation gate was run.
- Cleanup: **completed** for this invocation. All nine server threads closed,
  no probe ports remained listening, and 18 empty scratch directories were
  removed. Configs, session homes and traces remain as reproduction evidence.
  No Docker resources or shared caches were modified. See `cleanup.json`.
