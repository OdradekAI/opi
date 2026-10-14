# Phase 19 local native agent acceptance audit (P19-D08)

[简体中文](phase19-local-native-acceptance.zh.md)

## Current supplied evidence and apply outcome (2026-10-14)

**Operator evidence: verified. Native closure batch B1: Not closed.** The
current registered criterion requires four real trials across TB2.1/TB3;
DeepSWE AC-005/D04 is deferred/reserved. The current original evidence root is
`/home/luiz/phase19-acceptance-r2`, with the preserved four-trial view at
`/mnt/c/Users/Luiz/AppData/Local/Temp/opi-phase19-evidence-continuation-lcprjvy7/operator-f2-evidence`.
Its manifest SHA-256 is
`936e722ae9d2c54a5940a9d83c48a5db10eeceab7bccf1a913d4f01ace787e14`.
The historical six-trial record below remains attributed to its original date
and contracts; it does not determine the current scope or native closure.

A fresh source-bound Companion at committed source
`9ef3768968a559e0bc0366c1f3c66752d499595a` (production bytes unchanged by this
test-only apply) has SHA-256
`1406e98d638354899fab559d24a75dc313f1bf47448f08b48dd9806253f258e2`.
The locked/offline musl build, public acceptance verifier, two family verifies,
two report reconstructions per family, and exhaustive two-Opi artifact audit
all exited 0. Repeated report bytes match. Actual pi/Opi rewards are TB2.1
1/1 and TB3 1/0, with completed tool pairs 13/7 and 61/75 respectively; both
pairs are comparable. These are four attributable operator trials, with no new
real-provider execution authorized or performed by this apply.

All fresh commands, original exits and raw hashes are retained at
`/home/luiz/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d/operator-verification`; `receipts.json` binds each invocation. The
117 declared preserved view hashes were rechecked. Original attempts remain
separate: 28 sealed trials, 24 nonprimary trials and six scoped-out DeepSWE
trials; source bundles and original scores are retention targets.

## Current native execution and concrete blocker

The approved plan digest is
`571c76317637ab7acca5661f3c381dd8e7482648a9b8a8008d40c42e2a90e237`, bound to
live audit index
`23b39dcdc52f71dc1b17b4f87678f39aa3d86289c3753eb0b5f55373ad82873e`.
Independent public `doctor`/`prepare`, copied exact Opi 0.8.2/pi 1.0.4 packages
and build receipts, Harbor 0.22.0/uv 0.9.7, current capture helper and pinned
TB2.1/TB3 inputs are retained under `/home/luiz/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d`.
The final selected-family material/7 composite SHA-256 is `c29d45781e10a271df1399edc964b5baf4bb6b212c1634b6a9fc07cb3a66274a`.
Task/base/build-context/mode/argv/output image identities are retained under
`image-preparation` and `image-preparation-sidecar`.

The complete inventories contain 138 tests and 14 ignored native tests;
twelve exact native filters were selected. Serial dispatch stopped after the
second filter failed. Current selected executions are:

| Exact filter | Cargo exit | Complete transcript SHA-256 |
|---|---:|---|
| `native_tb21_artifact_witness` | 0 | `87cefd490d1ec5617b560113e1798a75421603f09bb6d5d235e5f13b813e2ee5` |
| `native_tb3_artifact_witness` | 101 | `52026101f4423d2d99fc4838676e5a642dbd2ff573a05867906862e5329cf829` |

TB2.1 executes one libtest case and both official oracle/nop jobs, with native
rewards 1/0. The anti-vacuity checker reports one passed, zero failed, zero
ignored, 137 filtered. Each invocation preserves complete independent stdout
and stderr and their hashes; the complete transcript concatenates both byte
streams without trimming. The earlier merged-stream run and checker refusal
remain retained separately.

TB3 executes one libtest case and both official jobs. Oracle grades 1 and
collects the task's sole `/app/results.txt`; the test now binds that exact
canonical declaration. Nop grades 0 upstream but submits no declared file, so
the production reader refuses it with
`native-binding-declared-artifact-missing`. The case exits 101. The required
valid incorrect-artifact observation and distinct collected digest therefore
remain absent. Original baseline receipts also retain the earlier runner-cache
setup refusal and the reproduced obsolete `/app/evalbench/` assertion failure.

Current native conformance admits only `oracle-witness` and `nop-witness`.
Treating the missing file as a valid artifact would contradict AC-002's
missing/substituted-artifact rejection. A correctly submitted negative witness
needs a new plan outside the approved four-path test/documentation scope;
production, public schemas, task scripts, ledgers, lockfile and client packages
were not edited. The remaining ten native filters, including the newly added
`native_real_client_offline_regrade`, were not executed after this stop. Its
four-source/eight-request/four-fresh-grade predicate has no green claim.

The result pair in `docs/snapshots/phase19/assurance/` records exact source-key
coverage, final offline workspace checks and the materialization boundary.
B1 remains Not closed; this document does not grant Phase conformance. Test
impact: add one ignored actual-client regrade case; update the TB3 source
assertion; retain existing ordinary/native/compatibility cases; delete none.

## Preserved historical six-trial record (2026-09-15)

The following text retains the earlier acceptance, attempts and limitations
under their original identities. Its verdict and old gate claims are historical.

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

### Final evidence and exact identity

- Redacted evidence root `E`:
  `/mnt/d/Luiz/Odradek/phase19-operator/evidence-root` (the
  `phase19-operator` operator workspace beside this repository checkout;
  Windows path `D:/Luiz/Odradek/phase19-operator/evidence-root`), manifest
  schema `opi-eval-local-acceptance/1`, manifest SHA-256
  `c7c0a2556b4e5d1dbe5626165590f5e831a218499fdc1dbc379d7029f214731a`.
  Registered in the historical `docs/eval/evidence-roots.jsonl` index (since
  removed from the checkout; preserved in Git history).
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

### Frozen scope

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

### Native outcomes

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

### Retained failed attempts and replacements

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

### Operator setup repairs (pre-trial, no model calls)

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

### Verification performed

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

### Limits

One task per family, one repetition, six trials total; native zeros are
valid graded outcomes and count in the denominator. The actual backend
model behind the `glm-5.3-flash` selector remains unknown; live wire bytes
were not captured. The output-control claim is proven by the local loopback
probe against the pinned clients, not against the live endpoint. Nothing
here is a corpus-level or statistical comparison, and Phase F still
independently reconstructs the criteria trace with its own human archive
gate.
