# Acceptance execution: Opi 0.8.2 and pi 0.85.1

[简体中文](20260910t104726z-opi082-pi0851-glm53flash-acceptance-live.zh.md)

**Acceptance handoff: blocked. Execution: incomplete; all six planned slots settled.** Four actual Agent runs received native grading and sealed bundles. Both DeepSWE attempts failed during environment setup before Agent execution. TB2.1 rewards are pi 1 / Opi 0; TB3 rewards are pi 0 / Opi 0. DeepSWE has no valid score. These are one-task native results, not full-corpus or statistical comparisons.

Two pi trials have qualifying real loops and native settlements; both Opi trials are excluded from acceptance by provider-stream failures, and both DeepSWE slots by infrastructure failure. The Companion's four valid graded outcomes remain intact; semantic acceptance eligibility is a separate, stricter check. No replacement trial, source repair, native regrade, commit, implementation-ledger update or Phase-exit claim was made.

## Evidence and exact identity

- Private WSL operator root `R`: `/home/luiz/opi-eval-runs/20260910t100917z-acceptance-opi082-pi0851`.
- Local diagnostic copy `E`: `E:\opi\evaluations\20260910t104726z-opi082-pi0851-glm53flash-acceptance-live`; [inventory.json](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/inventory.json) SHA-256 `dfde6308817c65cfc95ce672fd331a1a294d72b0fbcd30daa34e062bf3d4e87a`; 694 byte-inventoried files.
- External checks `C`: `E:\opi\evaluations\20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks`; [check commands](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/commands.json), [report equality](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/report-comparisons.json), [cleanup](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/cleanup/cleanup.json).
- No acceptance `manifest.json` exists: the evidence does not meet the six-trial semantic requirement. The inventory is a private diagnostic inventory, not an acceptance manifest.
- [frozen operator scope](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/operator-scope.json) SHA-256 `16f9a80798479e574f5f4e9b4c2278940b3dcd2ecd3e18811c0f2fd82559d553`. The original preview stays unchanged; [authorization](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/authorization.json) records the user's `go` at 2026-09-10T10:46:31.965304Z.

- opi: `https://github.com/OdradekAI/opi`, commit `765608a55a78db47e8b41d2639c29ef8c90e0f31`, tree `a12e0b6877d2217f7c21a3b1102c8309d90dae7d`, clean.
- pi: `https://github.com/earendil-works/pi`, commit `d981de1229ef899957bbe968bc8dcda02a21f477`, tree `346294a615d2d0ad4f6e5fbccb4cee4ccd7b2d6c`, clean.
- harbor: `https://github.com/harbor-framework/harbor`, commit `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`, tree `cc4e705e7d439332ba8562f8f9c9707630bb73f7`, clean.
- pier: `https://github.com/datacurve-ai/pier`, commit `c1ebc6d145b40fae8425215e3fca528945065124`, tree `b8cae7a6624893d5757f6c0d0e302ef982a390ca`, clean.
- opi executable SHA-256: `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee`.
- pi executable SHA-256: `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403`.
- pi runtime package manifest SHA-256: `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`; official standalone Linux x64 0.85.1 distribution.
- Opi v0.8.2 uses the retained static musl executable, not current source rebuilt as a substitute. Original build records remain under `builds/source-records/`; source checkouts and client/dependency packages remain in the original WSL cache locations.
- Companion separately: package 0.8.2, clean source `55fbc95009e6f47145196dbc539454ebcb1cf692`, executable SHA-256 `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`, Cargo.lock SHA-256 `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`; Rust/Cargo 1.97.1. [Companion build identity](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/builds/companion-identity.json) and [independent upstream observations](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/builds/source-identities.json) retain toolchain and tree evidence.

## Frozen scope and platform

| Family | Selected task | Plan digest | Material SHA-256 | Native job cap |
|---|---|---|---|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | `02215f12563dbebbec7ca91258383613f780f878a78d8d9e3edbceb69a51aa45` | `7a765b62a9637d082a8928fbd441e7d271b5b3e8679c60ac334479cfb9654e1e` | 900 s |
| terminal-bench-3.0 | `batched-eval-parity` | `62b5f4b11ddff301f8415f4c14b8e03e73e3ad156bc7f7789b8bbf81bfcfcbc6` | `b671b87603fdf25fc2a642e58b085d4ff882d068e0bd621bad9cae1adb6e0db3` | 900 s |
| deepswe-v1.1 | `abs-module-cache-flags` | `5a0c47671489748610b29f258c793700ad1dc9a13fc7e0d98d33372dfbf753fc` | `e87b0bc804053fe1fa519978cb95e7d312567d4dd4f0909fef88643442f7129b` | 1800 s |

One admitted task package per family, one repetition per software, no seed override, `max_trials=2` per family, concurrency 1, six slots total. The package corpus count 1 is not the full upstream corpus size. These researched candidates were selected for feasible local acceptance coverage. Platform: WSL2 Ubuntu, Linux x86_64, Docker context `default` at `unix:///var/run/docker.sock`, 32 logical CPUs and 16611237888 memory bytes; task resource requests 1/1/2 CPUs and 2/4/8 GiB.

Both products used provider `zhipu`, selector `glm-5.3-flash`, API `anthropic-messages`, endpoint `https://open.bigmodel.cn/api/anthropic`; only `OPI_BIGMODEL_API_KEY` was selected from the repository `.env` at authorized launch. Context 131072 and output 8192 were required; temperature, top-p and reasoning were omitted. The exact-client synthetic loopback probe established `max_tokens=8192` on initial and tool-continuation requests. This is proof of the selected limit only. Live wire requests were not separately captured; the actual backend model behind the selector remains unknown. pi received real responses and completed tool loops; Opi received responses but ended with stream failures.

Each native job cap includes setup, Agent execution and grading. Six caps sum to 7200 s; no guaranteed total wall-clock, total-token or monetary cap. Configurable timeout overrides lack native enforcement in this build. No paid retry was authorized or run. The first family started 2026-09-10T10:47:26.588650Z; the final family settled 2026-09-10T11:03:02.877674Z. Operator/offline intervals are included between those timestamps.

Exact image identities and preparation commands are retained in [prepared images](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/checks/prepared-image-builds.json) and the Docker baselines. TB2.1 base digest `4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12`; Python base `272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`; DeepSWE base `3a4d47f5281269305343c83729836ac2f3172811aee72681e472a4196178eda1`. Preparation covered task and verifier images but did not successfully prebuild Pier's runtime-generated Ubuntu/Squid proxy image; that gap surfaced during DeepSWE startup.

## Native outcomes and coverage

The metric is the benchmark-defined native grader reward over valid graded trials. Native zero and benchmark-valid Agent failures stay in its denominator. Columns below use the Companion's native classification, not acceptance eligibility.

| Family | Subject/version | Reward | Graded denominator | Package corpus / selected | Planned / attempted | Native valid / Agent failure | Invalid / unexecuted |
|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi 0.85.1 | 1 | 1 | 1 / 1 | 1 / 1 | 1 / 0 | 0 / 0 |
| terminal-bench-2.1 | opi 0.8.2 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| terminal-bench-3.0 | pi 0.85.1 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| terminal-bench-3.0 | opi 0.8.2 | 0 | 1 | 1 / 1 | 1 / 1 | 1 / 1 | 0 / 0 |
| deepswe-v1.1 | pi 0.85.1 | unknown | 0 | 1 / 1 | 1 / 1 | 0 / 0 | 1 / 0 |
| deepswe-v1.1 | opi 0.8.2 | unknown | 0 | 1 / 1 | 1 / 1 | 0 / 0 | 1 / 0 |

TB2.1 and TB3 each retain one paired comparison. DeepSWE has neither valid comparison side, so its pair is excluded; its normalized metric label is `unknown/unknown` with denominator 0 because no native result was admitted. Two pi rows qualify semantically for this operator review; zero Opi rows qualify. The six required acceptance rows cannot be selected. Full corpus and broader protocol/native-witness coverage remain unverified.

- TB2.1 pi completed nine task-side tool calls and delivered certificate/key/PEM/verification/script artifacts; native reward 1. Opi received one provider turn, completed zero tools, then exited 4 with `provider stream failed`; native reward 0 remains preserved.
- TB3 pi completed 24 calls (6 bash, 18 read). Its final response had `rawStopReason=max_tokens`, `stopReason=length`, output usage 8192, and only a thinking block. No task changes reached collection. Opi completed three calls (2 bash, 1 read), then exited 4 with `provider stream failed`. Both independently collected `/app/evalbench/` with digest `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7` and used the declared separate verifier; both native rewards are 0. Opi's provider failure excludes its acceptance row despite those tool calls.
- Both DeepSWE native job exceptions show a TCP timeout to `auth.docker.io` while obtaining anonymous registry metadata for `ubuntu:24.04` in the Pier egress-proxy build. `agent_execution` and `verifier_result` are null. This is Docker registry infrastructure failure, not observed Zhipu authentication failure. The Companion classified both as `invalid-execution` / `native-binding-aggregate-invalid`; their unsealed bytes and original exceptions remain intact. No committed patch, collection or pristine grading was observed.

## Trial traceability

[Machine trial summary](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/reports/trial-summary.json) retains event counts, native hashes, timing and source paths. The four sealed `native/trial-result.json` files were compared byte-for-byte with their actual job results; all four matched. A terminal file inventory can include original task files and is not by itself proof of Agent modification.

| Trial / family / task / repetition | Product / actual job / upstream trial | Settlement / native source | Tool loop / effect | Official handoff | Bundle manifest SHA-256 |
|---|---|---|---|---|---|
| `trial-baseline-pi-openssl-selfsigned-cert-r1`<br>terminal-bench-2.1<br>`openssl-selfsigned-cert`; r1 | pi<br>`2026-09-10__18-47-27` / `task-package__4g4NTjP` | sealed; reward 1; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | 9 calls: {'bash': 7, 'write': 1, 'edit': 1} | shared; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/manifest.json)<br>`4a202ef0eb5892ce49cb9d2a2abe81489ecd656b13fb48617f22f4a12d020dc6` |
| `trial-candidate-opi-openssl-selfsigned-cert-r1`<br>terminal-bench-2.1<br>`openssl-selfsigned-cert`; r1 | opi<br>`2026-09-10__18-49-25` / `task-package__yiQiRX3` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | 0 calls: {} | shared; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-openssl-selfsigned-cert-r1/bundle/manifest.json)<br>`0a249c347eef45ab0d98459fe5177003e03c9d4726d380e32b631e2529903c2e` |
| `trial-baseline-pi-batched-eval-parity-r1`<br>terminal-bench-3.0<br>`batched-eval-parity`; r1 | pi<br>`2026-09-10__18-53-32` / `task-package__vTmwXgc` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | 24 calls: {'bash': 6, 'read': 18} | separate; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-batched-eval-parity-r1/bundle/manifest.json)<br>`6bbb3ae86ea56d13c6675eaab48a0718c68e3dc2faa652d6bd5f1f6566abdfc4` |
| `trial-candidate-opi-batched-eval-parity-r1`<br>terminal-bench-3.0<br>`batched-eval-parity`; r1 | opi<br>`2026-09-10__18-59-34` / `task-package__eXQcqMD` | sealed; reward 0; [native result](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | 3 calls: {'bash': 2, 'read': 1} | separate; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/artifacts/control/settlement.json) | [bundle](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-batched-eval-parity-r1/bundle/manifest.json)<br>`547838d8d9966358d50f765d15bdc35e1812b53b768a49768ffae8fe956fa739` |
| `trial-baseline-pi-abs-module-cache-flags-r1`<br>deepswe-v1.1<br>`abs-module-cache-flags`; r1 | pi<br>`2026-09-10__19-01-49` / `task-package__h6zmBKk` | failed; unsealed; no score; [native exception](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-abs-module-cache-flags-r1/verifier-trace/jobs/2026-09-10__19-01-49/task-package__h6zmBKk/result.json) | Agent did not start | no patch or verifier handoff; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-abs-module-cache-flags-r1/bundle/artifacts/control/settlement.json) | absent; original unsealed files retained |
| `trial-candidate-opi-abs-module-cache-flags-r1`<br>deepswe-v1.1<br>`abs-module-cache-flags`; r1 | opi<br>`2026-09-10__19-02-30` / `task-package__BHqYSsA` | failed; unsealed; no score; [native exception](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-abs-module-cache-flags-r1/verifier-trace/jobs/2026-09-10__19-02-30/task-package__BHqYSsA/result.json) | Agent did not start | no patch or verifier handoff; [original trial files](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-abs-module-cache-flags-r1/bundle/artifacts/control/settlement.json) | absent; original unsealed files retained |

## Usage, time and cost

Durations come from original native timestamps. pi usage is the sum of completed assistant-response usage records, deduplicated by response ID, and is diagnostic rather than billed usage. Input/output/cache-read/cache-write order is shown. Opi error-path zero counters are not treated as zero consumption; trace manifests report usage unknown. DeepSWE never reached Agent launch.

| Family / product | Native trial seconds | Agent seconds | Provider turns / completed pi responses | Tool calls | Input / output / cache read / cache write tokens |
|---|---|---|---|---|---|
| terminal-bench-2.1 / pi | 116.595 | 89.378128 | 10 | 9 | 31340/4119/6272/0 |
| terminal-bench-2.1 / opi | 31.025 | 6.740122 | 1 | 0 | unknown; error-path zero counters are not usage evidence |
| terminal-bench-3.0 / pi | 361.119 | 311.540159 | 13 | 24 | 26822/9679/147072/0 |
| terminal-bench-3.0 / opi | 52.294 | 13.787313 | 3 | 3 | unknown; error-path zero counters are not usage evidence |
| deepswe-v1.1 / pi | 31.776 | not started | not started | not started | not called |
| deepswe-v1.1 / opi | 32.290 | not started | not started | not started | not called |

The normalized reports mark usage, retries, compaction and cost unknown. pi's configured cost fields contain zero placeholders, not an invoice. Billed cost and complete Opi token use are unknown. There were no operator replacement trials or native regrades; internal provider retry/compaction counts are not established.

## Verification and privacy

| Check / command (absolute substitutions below) | Cwd | Exit / observed result |
|---|---|---|
| `$R/builds/opi-eval run --plan $R/plans/$F.json --root $R/runs/$F --fixtures $R/unused-fixtures --canaries $R/private/live-canaries.txt` | `$R` | TB2.1: 0; TB3: 0; DeepSWE: 1 |
| `$R/builds/opi-eval verify --root $R/runs/$F` | `$R` | TB2.1: 0; TB3: 0; DeepSWE: 1 (`mutation-detected`, two `io` failures because bundle manifests were never sealed; not evidence of a later edit) |
| `$R/builds/opi-eval report --root $R/runs/$F --out $R/reports/$F-a.json --canaries $R/private/live-canaries.txt` and fresh `-b.json` | `$R` | All six commands: 0; each A/B pair identical |
| `$R/builds/opi-eval verify --root $E/runs/$F` | `$R` | 0 / 0 / 1, same original outcomes |
| `$R/builds/opi-eval report --root $E/runs/$F --out $C/$F-a.json` and fresh `-b.json` | `$R` | All 0; A = B = original for each family |
| `python3 $REPO/crates/opi-eval/scripts/verify-local-acceptance.py --root $E --out $C/acceptance-result.json --companion $R/builds/opi-eval` | `$R` | 2; `accepted: false`; rejects absent acceptance manifest before deeper acceptance checks |
| `python3 $REPO/scripts/opi-artifact-audit.py $E/opi-artifact-audit --workspace-root $REPO --json` | `$R` | 0; 17 preserved files in three exact native directories; no detected issues. The third is an unsealed failure fragment; no DeepSWE Agent capture exists. |
| `python3 $R/verify-staged.py` supplemental `analyze_ndjson` from the same repository auditor | `$R` | 0; exactly two actual Opi stdout captures analyzed, zero reported issues; does not validate provider behavior or prove acceptance |
| Post-cleanup `verify` plus fresh `report --out $C/post-cleanup/$F-report.json` on original roots | `$R` | Verify 0 / 0 / 1; reports all 0 and equal to originals |

`F` is each of the three exact family names above. `R`, `E`, `C` denote their WSL paths; `REPO=/mnt/d/Luiz/Odradek/opi`. Exact expanded argv, timestamps and exits are in [original commands](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/commands/run-deepswe-v1.1.json), the other `commands/run-*.json` / `verify-*.json` / `report-*.json`, and the external check records. Credential variables were absent from offline check environments. The disposable canary path was removed after privacy checks.

Preparation build/probe and all three doctor/prepare/validate/plan commands exited 0, with exact original records retained. The earlier bilingual [preparation report](20260910t100917z-opi082-pi0851-glm53flash-acceptance-preview.md) remains historical. Its synthetic probe command was `cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact`: 1 passed, 0 failed. No Rust/source/test file changed in this operation; test impact **retain**. Workspace runtime gates and new three-protocol witnesses were not run.

The entire staged local diagnostic selection was scanned against the selected live credential; zero matches, no edited or resealed native bytes. [Privacy review](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/checks/privacy-final.json) records scope and limits. Seventeen files exactly match the three available Opi native directories, but generic audit found no NDJSON/session files in that view; the supplemental direct audit covers the two actual Opi stdout files. Neither audit substitutes for missing DeepSWE traces or native acceptance semantics.

This evidence remains local/private: it contains operator filesystem paths, public task author metadata, and a task-generated TLS private-key marker in the pi TB2.1 transcript. No provider key, dotenv, personal credential store or live canary file was copied. Public sharing was not approved. The acceptance verifier's rejection is expected and only establishes the missing manifest; it did not validate deeper six-trial requirements.

Documentation verification: `python3 scripts/opi-doc-check.py` passed; `git diff --check` exited 0. Exact outputs and command records are retained in [documentation checks](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/documentation-checks.json). Only the two historical preview reports and two new live reports are untracked; runtime source and test files are unchanged.

## Cleanup and acceptance handoff

Cleanup **completed**. The selected credential canary copy and four empty owned directories were removed after all writers and privacy checks finished. Docker inspection shows zero containers, zero volumes, only default networks. Reusable task/base/verifier/runner images, material/source/client/dependency caches, frozen executables and all failed/sealed evidence remain. The DeepSWE base image was absent after native cleanup; exact-digest pull and alias restoration both exited 0 and recovered the original identity. No global pruning, operator image removal or Cargo cache deletion occurred. Reclaimed bytes are unknown.

Cleanup compared 404 protected original files and all 694 staged files with their prior hashes: no changes. Post-cleanup verification remains 0 / 0 / 1 and all rebuilt reports match their originals. See [cleanup record](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/cleanup/cleanup.json) and [post-cleanup commands](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live-checks/post-cleanup/commands.json).

| Criterion | Operator evidence status | Evidence / missing part |
|---|---|---|
| P19-AC-001 | evidenced for TB2.1/TB3; missing end-to-end DeepSWE | Independent-root commands, four seals and reconstructed reports; DeepSWE startup failed |
| P19-AC-003 real portion | rejected/incomplete | pi task effects and shared-environment grading preserved; Opi missing real tool loop |
| P19-AC-004 real portion | evidenced mechanism; rejected complete acceptance pair | Two declared equal collections and separate native grading; Opi provider failure excludes its acceptance row; broader correct/incorrect witnesses not run here |
| P19-AC-005 real portion | missing | Neither Agent launched; no base-to-committed-patch/pristine-verifier trace |
| P19-AC-006 real portion | partially evidenced; rejected complete requirement | Same projected connection, real pi loops and Opi TB3 tools; Opi stream failures and backend-model observation gap; broader three-protocol tests not run here |
| P19-AC-012 | rejected | 6 slots attempted; 4 actual Agent runs, 4 native grades/seals, 2 semantically qualifying pi rows; 6 qualifying rows absent |

[Diagnostic findings](/E:/opi/evaluations/20260910t104726z-opi082-pi0851-glm53flash-acceptance-live/reports/findings.jsonl) records EVAL-001 (Opi stream failures; root cause unresolved) and EVAL-002 (DeepSWE proxy-image startup failure) as `source_kind=eval`, `axis=runtime-fidelity`, `status=unverified`, outside the active audit set. The evaluator's exact model ID/independence is unknown and no independent judge was used.

The remaining actions are offline diagnosis of the retained Opi failures and preparation of the original Pier proxy image/dependencies. Any source repair needs its concrete scope approved; any replacement real trials need new frozen identities and explicit execution authorization. Original evidence stays at R and E. Do not resume task 19.11 as an accepted handoff from this inventory.
