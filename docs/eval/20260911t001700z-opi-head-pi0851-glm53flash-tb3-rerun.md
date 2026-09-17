# TB3 rerun with updated model controls

[简体中文](20260911t001700z-opi-head-pi0851-glm53flash-tb3-rerun.zh.md)

**Both requested trials were attempted and settled; the paired benchmark is incomplete.** pi exhausted the outer native-job budget and has no grade. Opi exited with `provider stream failed`; the native verifier graded its collected environment zero (0/5 checks passed). The Companion records Opi as `graded-native` Agent failure and pi as invalid execution. This is not a successful paired capability comparison.

Evidence is private at [E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence), indexed by [evidence-index.json](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/evidence-index.json), SHA-256 `e83315c24911f8d3ac9325c5fcd29424a1789fc32d134a89db75463f9cb329a8`. The [normalized report](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/terminal-bench-3.0-a.json), [semantic review](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/semantic-review.json), and [telemetry](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/telemetry.json) retain the distinctions. This is a two-trial benchmark subset rerun of the prior acceptance pair; it neither replaces the previous six-trial manifest nor closes a Phase or implementation task.

## Scope and identities

The user requested “复跑tb3” after updating opi-eval. The operation reused the explicit Opi/pi, pi 0.85.1, GLM-5.3-Flash, original task and `.env` credential-reference selections. Omitted Opi version selected the current local working tree. No additional paid retries or source repairs were performed. [Authorization and frozen scope](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/operator-scope.json) retain the exact two trials. Source inputs were identical before/after the locked build; the source snapshot, diff and build records are preserved under [builds](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/builds).

| Item | Identity |
|---|---|
| Opi | Local working-tree candidate, package 0.8.2; HEAD `55fbc95009e6f47145196dbc539454ebcb1cf692`; existing documentation/skill changes retained; no runtime change in this rerun |
| Opi executable SHA-256 | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60`; Linux musl; locked build returned the current compiler artifact; same bytes as prior candidate |
| pi | 0.85.1; source `d981de1229ef899957bbe968bc8dcda02a21f477`; previously acquired standalone package revalidated |
| pi executable / package SHA-256 | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` / `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae` |
| Companion | Separately rebuilt opi-eval 0.8.2, Linux GNU; SHA-256 `c730f1fe85a5eed9cb8c2207572ac551dca7e69d8d1a0feb6537e87465d377ee` |
| TB3 / Harbor | TB3 commit `2b0442c3c583b710ca8da14c8e601b99f2f1f244`; Harbor `4407eb5227a2ff4f0d3f16b2eb48849382fdf276` |
| Experiment digest | `451736416157c883e7de81e2060aee01f975b655e6fa5ceede3de9fa5d581623` |
| Plan digest | `ebec713871be25f4de4c9945e531e5de14cef8012673bddf81b58721ee649de4` |
| Material SHA-256 | `85c306b3cbd987fce1010f0a621dd158a75d897888eb98c8036f19c15852a962` |
| Config SHA-256 | `b7c3df1ae3a8cd1c15a94ec0744a064d9a92b4f4f480584618993409affaa9e6` |
| Evidence index SHA-256 | `e83315c24911f8d3ac9325c5fcd29424a1789fc32d134a89db75463f9cb329a8` |

Execution used WSL Ubuntu/Linux x86_64 and the Docker Linux engine. Doctor observed 32 CPUs, 16,611,237,888 bytes of memory, an available NVIDIA GPU and sufficient disk; the selected task requires 1 CPU, 4 GiB, no GPU. Pinned task/runner/image identities and Docker inventories are retained in material/preparation/baseline records. The admitted local inventory contains one task, `batched-eval-parity`; it is not the entire upstream TB3 corpus.

Both subjects used `zhipu:glm-5.3-flash`, `anthropic-messages`, `https://open.bigmodel.cn/api/anthropic`. The operator read only the already-selected `.env` credential at launch; no value is included here. Required controls were context window **1,000,000** and per-response output **131,072**, with temperature/top_p/reasoning omitted. The two exact binaries each passed first-request and tool-continuation loopback checks with numeric `max_tokens=131072`; [proofs](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/output-control-r2) are bound to the frozen material. Official [model](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/glm-5.3-flash-recheck.md), [API](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/chat-completion-recheck.md) and [endpoint](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/official-docs/anthropic-endpoint-recheck.md) documentation was rechecked. Declared context capacity is not a measurement of successful 1M-token server consumption.

One repetition per subject, concurrency 1, max_trials 2; pi ran first. The unchanged profile limits the **whole native job to 900 seconds per trial**, including preparation, Agent and grading. The task itself declares **14,400 seconds for its Agent** and **900 for its verifier**; these are distinct from the outer profile budget. Native stdout/stderr caps are 1 MiB each; separate Agent captures can be larger. Opi retained max_iterations/max_turns=50; stock pi 0.85.1 had no equivalent fixed turn cap. No total token, cost or total wall-time guarantee was claimed. [Execution record](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/commands/run-terminal-bench-3.0.json): `2026-09-11T00:23:36.037394+00:00`–`2026-09-11T00:43:57.897748+00:00`, 1221.86 seconds.

## Outcomes and coverage

| TB3 subject / task | Planned / attempted | Native reward | Companion valid / invalid | Observed stop |
|---|---|---|---|---|
| pi 0.85.1 / batched-eval-parity | 1 / 1 | Unavailable; denominator 0 | 0 / 1 | Outer job timeout; unsealed `grader/verifier-timeout`, graded=false |
| Opi local HEAD 0.8.2 / batched-eval-parity | 1 / 1 | 0; denominator 1 | 1 / 0; Agent failure=1 | Exit 4, `provider stream failed`, native `NonZeroAgentExitCodeError` |

These counts reproduce the Companion JSON: corpus=1, selected=1, planned=2, attempted=2, valid=1, invalid=1, unexecuted=0. The sole comparison pair is excluded because pi is invalid. The native zero is preserved, but the unresolved Opi stream failure prevents interpreting it as normal Agent completion or evidence of comparative capability. No new acceptance-ready handoff is claimed.

- [pi settlement](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/runs/terminal-bench-3.0/trials/trial-baseline-pi-tb3-rerun-batched-eval-parity-r1/bundle/artifacts/control/settlement.json): 16 completed model responses and 32 tools (11 bash, 19 read, 1 write, 1 edit) are visible in the retained unsealed diagnostic log. Collection/grading did not finish; no score was reconstructed from attempted edits.
- [Opi native result](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/runs/terminal-bench-3.0/trials/trial-candidate-opi-working-tree-tb3-rerun-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json): 10 provider turns, 23 tools (6 bash, 17 read), Agent 277.44 s, total native trial 317.14 s, verifier 30.69 s. Only declared `/app/evalbench/` crossed to the separate verifier; collection digest `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7`. All five native tests failed. The sealed native result equals the original runner result byte-for-byte.

Neither stop was caused by the 50-turn limit. Opi's trace records only the generic stream failure; transport, server and parser causes remain unresolved. No deterministic source-bug claim or fix is made.

## Usage and comparison limits

| Subject | Completed-response input | Output | Cache read | Telemetry scope |
|---|---:|---:|---:|---|
| pi | 67,998 | 34,908 | 254,976 | 16 complete responses; interrupted-response usage unknown |
| Opi | 23,291 | 4,297 | 81,600 | Terminal session summary; failed stream may have unreported usage |

pi completed a single **20,719-token** response with tool continuation, establishing that the live run passed the former 8,192-token limit. Larger response allowance did not ensure completion within the unchanged outer time budget. Transport retries, compaction, full interrupted-response usage and billed cost remain unknown; configured zero prices are not zero billing.

The previous TB3 pair graded both subjects zero, with Agent durations pi 262.98 s / Opi 343.26 s. This rerun changed controls and ended differently; its partial timings and scores do not establish speed or capability superiority. [Earlier report](20260910t175447z-opi-head-pi0851-glm53flash-proxy-fixed-acceptance-live.md) remains historical and unchanged.

## Verification, privacy and cleanup

```sh
# cwd for native/check commands:
R=/home/luiz/opi-eval-runs/20260911t001700z-tb3-rerun
EVAL="$R/builds/opi-eval"
CONFIG="$R/inputs/terminal-bench-3.0.toml"
MATERIAL="$R/materials/terminal-bench-3.0/material.json"
PLAN="$R/plans/terminal-bench-3.0.json"
ROOT="$R/runs/terminal-bench-3.0"
"$EVAL" doctor --config "$CONFIG"
"$EVAL" prepare --config "$CONFIG" --out "$R/materials/terminal-bench-3.0"
"$EVAL" validate --config "$CONFIG" --native-material "$MATERIAL"
"$EVAL" plan --config "$CONFIG" --native-material "$MATERIAL" --out "$PLAN"
"$EVAL" run --plan "$PLAN" --root "$ROOT" --fixtures "$R/unused-fixtures" --canaries "$R/private/live-canaries.txt"
"$EVAL" verify --root "$ROOT"
"$EVAL" report --root "$ROOT" --out "$R/reports/terminal-bench-3.0-a.json" --canaries "$R/private/live-canaries.txt"
"$EVAL" report --root "$ROOT" --out "$R/reports/terminal-bench-3.0-b.json" --canaries "$R/private/live-canaries.txt"
# cwd=/mnt/d/Luiz/Odradek/opi; shared external Cargo cache, CARGO_INCREMENTAL=1
cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
cargo build --locked --release --target x86_64-unknown-linux-gnu -p opi-eval --bin opi-eval --message-format=json-render-diagnostics
```

Builds, doctor, prepare, native validation and plan each exited **0**. The first local probe exited **4** without sending a request because its operator D-Bus pathname address prevented credential fallback; changing only that probe address to the repository test's isolated abstract form made both exact-client checks pass. No Opi source change was needed. The initial summary-print helper also encountered the absent pi native result; [diagnostic record](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/checks/operator-helper-diagnostic.json) explains the read-only presentation correction.

Native `run` exited **1**, reflecting incomplete coverage. Original and staged `verify` each exited **1**: Opi's sealed bundle verified, while pi's missing manifest yielded `io`; the generic `mutation-detected` label is not evidence of modified Opi bytes. Four `report` invocations exited **0** and produced identical bytes, SHA-256 `b8692d3e53fdf0dc76600027449565b8819839a97526850c50f552b48e9840ae`. [Staging checks](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/staging-verification.json) bind their exact argv/cwd/exits and byte comparison. Verification did not regrade or rerun a provider. The repository documentation check is recorded separately in [documentation-check](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/documentation-check.json).

The selected-key canary scan covered 177 settled-run/launch files with zero hits. Captures/source snapshots retain personal paths and private data, so this root is **private, not approved for public sharing**. Sealed bytes were not redacted or rewritten. [Final cleanup](E:/opi/evaluations/20260911t001700z-tb3-rerun/checks/cleanup-final.json) is completed: projected canary and disposable operator scratch removed, zero owned containers remaining, build lease released, loopback listeners stopped. A 792-byte Docker client log was retained privately for diagnostics; reusable images, shared caches and original/staged evidence remain. Reclaimed space is unknown. Post-copy run inventories matched, and the previous acceptance manifest digest remained unchanged.

Test impact: **none**; no Rust or test files changed. Only this bilingual eval report was added to the repository. [Eval findings](E:/opi/evaluations/20260911t001700z-tb3-rerun/evidence/reports/findings.jsonl) are unverified operator evidence outside the active audit/remediation set. Remaining work is to diagnose the Opi stream failure and, if another live pair is requested, explicitly reconcile the outer job budget with the task's longer allowance before freezing a fresh plan. No automatic paid retry was launched.
