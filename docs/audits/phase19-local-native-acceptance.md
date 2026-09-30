# Phase 19 local native agent acceptance audit (P19-D08)

[简体中文](phase19-local-native-acceptance.zh.md)

**Verdict: accepted.** Six real local-operator trials — one Opi and one pi per
family across terminal-bench-2.1, terminal-bench-3.0 and deepswe-v1.1 — ran
through the Zhipu `glm-5.3-flash` Anthropic-messages connection, received
attributable native grading, sealed their bundles, and were reconstructed by
the offline local acceptance verifier (`accepted: true`, exit 0). The
Opi-only scoped artifact audit passes non-empty, exhaustive and byte-bound.
All failed attempts stay retained under the operator run roots with their
replacement identities. These are one-task-per-family native results with one
repetition; they are not corpus-level or statistical comparisons.

This audit records redacted findings only. No credential value, live wire
payload or private key appears here or in the staged evidence root.

## Final evidence and exact identity

- Redacted evidence root `E`:
  `/mnt/d/Luiz/Odradek/phase19-operator/evidence-root` (the
  `phase19-operator` operator workspace beside this repository checkout;
  Windows path `D:/Luiz/Odradek/phase19-operator/evidence-root`), manifest
  schema `opi-eval-local-acceptance/1`, manifest SHA-256
  `c7c0a2556b4e5d1dbe5626165590f5e831a218499fdc1dbc379d7029f214731a`.
  Registered in [docs/eval/evidence-roots.jsonl](../eval/evidence-roots.jsonl).
  The verifier of record at acceptance time (2026-09-15, companion
  SHA-256 `f8bf959c1428c3e10c88a7f6ebcc37166043b3d1f3f74c40809a0f30775a861b`)
  accepted this root; the current verifier rejects it because its
  material/4 manifests and /1 native terminals predate the material/6 and
  terminal-preflight /2 contracts - it is retained as historical
  acceptance evidence, not as currently consumable input.
- Run roots (retained failures included):
  `/home/luiz/.cache/opi/phase19-setup-repair/runs/{tb21,tb30,deepswe}`.
- opi agent: static musl executable, SHA-256
  `c89ac4a103c6cf0f433469ae45dc6baea03328c3937b8c412f67e8f0457ff8de`,
  locked offline release build of this repository at commit
  `55fbc95009e6f47145196dbc539454ebcb1cf692`.
- pi agent: SHA-256
  `f4db98b5f0ec4720a9be6d8e04971076f84050d365b066edca1e3cb84c6ed6ec`,
  `@earendil-works/pi-coding-agent` 0.85.1 runtime package at source commit
  `d981de1229ef899957bbe968bc8dcda02a21f477`, self-contained node 22.19.0.
- Sources (repository-qualified in the manifest): pi
  `https://github.com/earendil-works/pi` @ `d981de12`, harbor
  `https://github.com/harbor-framework/harbor` @ `4407eb52`, pier
  `https://github.com/datacurve-ai/pier` @ `c1ebc6d1`.
- Materials (`opi-eval-native-material/4`, origin `local`), SHA-256:
  terminal-bench-2.1 `b02ad3dd3b948bb044c1ba4ee89560a83905afb3d76e7ef85f05964a3bc9c129`,
  terminal-bench-3.0 `03cc326f8e8831f626795822712ffd3d84e299df97e14bde694f60e407bbd760`,
  deepswe-v1.1 `35ebd3dc119d7da7d72bda6615bf66d6824f0309284284c8704b3f71fb4bab7a`.
- Preserved run reports, SHA-256: terminal-bench-2.1
  `15bb5a4bb50498f6784201b4db9054d00bf0ad75376747d8162ce8013f31ca61`,
  terminal-bench-3.0
  `7ea5abb767d3632f4bb392c80fa4f3458eff30b80279fdc7f4e56ab1772db39a`,
  deepswe-v1.1
  `674b198cb77ba7c01c723c719f4cc66cec2210f97c4ee1e7d34066ad0d08e5d2`.

## Frozen scope

| Item | Value |
|---|---|
| Connection | `zhipu-anthropic`: provider `zhipu`, api `anthropic-messages`, `https://open.bigmodel.cn/api/anthropic`, model `glm-5.3-flash`, credential env `ZHIPU_API_KEY`, mode `live` |
| Model controls | context_window 131072, max_output_tokens 4096, reasoning/temperature/top_p omitted, no required controls |
| Execution | concurrency 1, 900 s per native job, 1 MiB stdout/stderr caps, one repetition per product, six slots total |
| Tasks | tb21 `openssl-selfsigned-cert`, tb30 `batched-eval-parity`, deepswe `abs-module-cache-flags` (one admitted task package per family, pinned upstream) |
| Output-control witness | sealed `control/output-control.json` inside every final bundle |

The credential was exported by the operator in their own shell only. No
harness run read the `.env` file, and no staged file contains a credential
value.

## Native outcomes

The metric is the benchmark-defined native grader reward. Native zero and
benchmark-valid agent failures stay in the denominator.

| Family | Task | pi trial / reward | opi trial / reward | Pair |
|---|---|---|---|---|
| terminal-bench-2.1 | openssl-selfsigned-cert | `trial-baseline-pi-openssl-selfsigned-cert-r1.r2` / 1 | `trial-candidate-opi-openssl-selfsigned-cert-r1.r2` / 1 | comparable |
| terminal-bench-3.0 | batched-eval-parity | `trial-baseline-pi-batched-eval-parity-r1.r2.r2` / 0 | `trial-candidate-opi-batched-eval-parity-r1.r2.r2` / 0 | comparable |
| deepswe-v1.1 | abs-module-cache-flags | `trial-baseline-pi-abs-module-cache-flags-r1.r2.r2` / 0 | `trial-candidate-opi-abs-module-cache-flags-r1.r2.r2` / 0 | comparable |

All six trials report completion `verified-native`. Both tb30 trials and
both deepswe trials are native zeros — valid graded outcomes, not auth,
infrastructure, tool-loop or handoff failures. Each agent record binds the
executing product, `zhipu:glm-5.3-flash`, the executable digest and the
delivered/collected artifact digests; TB3.0 used the declared separate
verifier with declared collection; deepswe bound the committed patch digest
against the declared base in a pristine verifier.

## Retained failed attempts and replacements

Every failed attempt stays durable in the run roots; replacements used
`--replacement-for` under fresh identities.

1. r1 (2026-09-15T09:43Z): tb21 ran both agents; pi exited 1 on a missing
   theme resource and the pair settled `unsupported-control:max_output_tokens`;
   tb30/deepswe were not started in that batch.
2. r1 (2026-09-15T12:09Z): tb30/deepswe attempts died at the `integrity`
   boundary — the runner invoked `docker compose cp` with relative
   `jobs/...` operands after its working directory was deleted; agents were
   never invoked; trials unsealed.
3. r1.r2 (2026-09-15T12:50Z, tb30/deepswe only): both agents sealed and
   graded natively, but the pair comparability stayed
   `unsupported-control:max_output_tokens` (no sealed output-control witness
   yet), so the runs exited 1 incomplete.
4. Final (2026-09-15T19:56Z): three replacements — tb21 `r1 -> r1.r2`,
   tb30 `r1.r2 -> r1.r2.r2`, deepswe `r1.r2 -> r1.r2.r2` — after the
   output-control witness was proven and pinned into all three materials.
   All three runs exited 0 with outcome `completed`; every pair comparable.

A manifest correction is recorded for honesty: during staging the
`command_exits` run entries still carried the stale exit 1 of attempt 3;
they were corrected to the final batch's actual 0 and the offline verifier
re-accepted (exit 0).

The published report view was reconstructed by read-only per-execution
staging: for each family, a fresh root received byte-exact copies of exactly
`run-control.json` plus the trial directories its `planned_trials` declares
for the final execution; `verify` and `report` ran against each staged copy
with outputs written outside both the staged and original roots. The whole
mixed run roots remain fail-closed (`report` exits 1 on them), every
superseded attempt stays retained in the original roots, and the original
`run-control.json` / `run-report.json` bytes are unchanged.

## Operator setup repairs (pre-trial, no model calls)

Recorded in `phase19-operator/setup-repair/`: DrvFs small-file upload
exceeded Harbor's 360 s setup cap (pi package moved to WSL-native ext4;
setup then 2.3 s); Docker Compose started from a deleted cwd (stable-cwd
launcher with redacted diagnostics); pi launcher host-absolute node path
(node 22.19.0 placed inside the package); opi GNU binary needed glibc 2.39
vs container 2.36 (locked offline musl static build); `compose cp` relative
operands recovered by walking to the `verifier-trace` ancestor (end-to-end
regression tests 3/3); pi package completed its missing `dist/` resources;
and the local loopback output-control probe (zero credentials) proved both
pinned clients honor `max_output_tokens` on initial and tool-continuation
requests before that control was accepted as comparable evidence.

## Verification performed

- `python crates/opi-eval/scripts/verify-local-acceptance.py --root <E>
  --out <out> --companion phase19-operator/bin/opi-eval` → `accepted: true`,
  exit 0; all six trials and three families listed; companion invoked only
  against the supplied family roots under a credential-stripped environment.
- `python scripts/opi-artifact-audit.py --workspace-root .
  <E>/opi-artifact-audit` → PASS; the Opi-only generic-audit view is
  non-empty, exhaustive for the three Opi trial captures, and byte-bound to
  the originals.
- `cargo fmt --check --all` and
  `cargo clippy --workspace --all-targets -- -D warnings`: green.
- `cargo test --workspace --all-targets`: green except the pre-existing
  `spec_ledger` hash check, which pins the ledger to HEAD while an
  unrelated parallel-repair session holds an uncommitted `docs/opi-spec.md`
  edit; not caused by this evidence. The full workspace gate re-runs after
  that edit lands.
- WSL2 Ubuntu operator platform, Docker engine at `unix:///var/run/docker.sock`,
  images pulled by pinned manifest digest.

## Limits

One task per family, one repetition, six trials total; native zeros are
valid graded outcomes and count in the denominator. The actual backend
model behind the `glm-5.3-flash` selector remains unknown; live wire bytes
were not captured. The output-control claim is proven by the local loopback
probe against the pinned clients, not against the live endpoint. Nothing
here is a corpus-level or statistical comparison, and Phase F still
independently reconstructs the criteria trace with its own human archive
gate.
