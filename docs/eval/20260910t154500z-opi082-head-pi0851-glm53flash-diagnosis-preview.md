# Offline diagnosis and proposed acceptance rerun

[中文](20260910t154500z-opi082-head-pi0851-glm53flash-diagnosis-preview.zh.md)

The release v0.8.2 signature-event failure is reproducible offline. Existing HEAD `55fbc950` already handles this case; its separately built candidate passes the same reproduction. No source repair was made. A new six-trial acceptance preview is prepared, with all six trials **unexecuted and awaiting new authorization**. The original acceptance remains blocked; offline results do not change native scores or complete Phase 19.

## Evidence and scope

- Private Linux diagnosis root: `/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082`; proposed execution root: `/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance`.
- Review copy: [E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082); this contains diagnostic records and preview inputs, while retained executables and runner caches stay at their original Linux paths.
- [Operator scope](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/operator-scope.json) SHA-256: `dbe94817a77673b2673ba59b3c643d71d946d068a881875bb5e2a816144c5a6e`. This is a preview digest, **not an acceptance-manifest digest**. No new acceptance manifest or real run exists.
- Previous [live report](20260910t104726z-opi082-pi0851-glm53flash-acceptance-live.md) and its six failed/successful slots remain unchanged. The earlier `go` covered that original scope only. This proposed experiment replaces the Opi subject with a different source commit and adds six new live trials, including a fresh pi baseline.
- This invocation resolved no real credential and made no live-provider request. Proxy image preparation accessed Docker registries; Agent reproductions used synthetic loopback responses, and container checks used `--network none`.

## Diagnosis and its limits

`v0.8.2`'s Anthropic `RawDelta` lacks `signature_delta`. The recorded [source diff](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/anthropic-source-diff.patch) shows current HEAD adds signature handling and signed/redacted thinking preservation. A synthetic stream sends a thinking block, a signature event, a bash tool call, then a final response. Success requires two provider requests, `proof.txt=offline-ok`, and the signature retained in the tool-continuation request.

| Executable / case | Child exit | Requests | Tool effect / signature replay | Observation |
|---|---:|---:|---|---|
| release Opi v0.8.2, signed, attempt 1 | 4 | 1 | absent / absent | `provider stream failed`; reproduction harness exit 1 |
| release Opi v0.8.2, signed, attempt 2 | 4 | 1 | absent / absent | same failure; harness exit 1 |
| release Opi v0.8.2, unsigned control | 0 | 2 | present / not applicable | harness exit 0 |
| official pi 0.85.1, signed | 0 | 2 | present / preserved | harness exit 0 |
| current HEAD GNU build, signed | 0 | 2 | present / preserved | harness exit 0 |
| current HEAD musl release build, signed | 0 | 2 | present / preserved | harness exit 0 |

Exact command arguments, binary hashes, timestamps, synthetic requests/responses, stdout, stderr and effects are retained under [repro](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/repro); the [reproduction script](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/stream-repro.py) contains no real credentials.

```sh
# Synthetic loopback only. --out must be a fresh directory.
python3 /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/stream-repro.py --binary /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance/cache/agents/opi --product opi --case signed --out <fresh-private-directory>
```

The original pi live captures contain signed thinking in 7/7 TB 2.1 and 8/8 TB 3.0 observed thinking blocks; only counts and source identities are recorded in [metadata](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/live-signature-metadata.json). This supports the signature hypothesis for the original Opi failures, but those Opi trials did not capture raw provider wire events. Their exact failing wire event therefore remains unproven. Their native zeros and unknown telemetry remain as originally reported. The original pi TB 3.0 failure reached its 8192 output limit and did not establish the expected task effect; it remains a separate failure.

Both original DeepSWE slots failed while building the generated egress proxy, before Agent startup, because Docker Hub metadata/auth access timed out. The exact retained Dockerfile and startup script were copied without changes. `docker build --progress=plain --tag opi-eval-pier-egress-proxy:20260910t154500z /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/proxy-context` now exited 0, and a synthetic-token, network-isolated health check became ready on localhost:8080. The [build and smoke records](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/proxy-smoke.json) identify image `sha256:95c565d09bbf73b2990489346866f719d90566d775524b73c040a11ddfda22ea`.

The build log resolved Ubuntu to `sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254`. BuildKit cache is retained; no local `ubuntu:24.04` tag exists. The pinned native runner still generates/builds its own unchanged proxy. A warmed cache does not guarantee future registry availability and does not repair the two original invalid trials.

## Proposed subjects and controls

| Component | Source / version | Executable SHA-256 |
|---|---|---|
| Original Opi, retained only | v0.8.2; `765608a55a78db47e8b41d2639c29ef8c90e0f31` | `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee` |
| Proposed Opi candidate | unreleased `55fbc95009e6f47145196dbc539454ebcb1cf692`; package 0.8.2; x86_64 musl | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| Proposed pi baseline | official Linux x64 0.85.1; `d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| Companion, separate from evaluated Opi | opi-eval package 0.8.2; `55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |

The candidate is **not release tag v0.8.2**, despite identical package-version output. Tracked source is clean; no source was edited this invocation. pi runtime-package manifest SHA-256 is `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`; Cargo.lock SHA-256 is `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`. Rust/Cargo are 1.97.1. Harbor is pinned at `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`, Pier at `c1ebc6d145b40fae8425215e3fca528945065124`; independent copied checkouts have clean tracked source and offline-rebound virtual environments. Full build and runner identities are in [builds](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/builds).

Connection remains `zhipu:glm-5.3-flash`, `anthropic-messages`, `https://open.bigmodel.cn/api/anthropic`. Only repository `.env` variable `OPI_BIGMODEL_API_KEY` will be selected at an authorized Agent launch. Common controls: output 8192 per response, context window 131072; temperature, top_p and reasoning omitted. The context value is a declared client setting, not proof of server capacity. The actual candidate/pi loopback output-control probe passed for initial and tool-continuation requests; proofs were bound before plans were frozen.

Three families, one locally admitted pinned task package per family, two subjects, one repetition, concurrency 1, six new trials. This is the same acceptance subset as before, not the full upstream corpora or a statistical comparison. CPU/memory requirements per task are 1 CPU/2 GiB, 1 CPU/4 GiB, and 2 CPU/8 GiB respectively; no GPU.


| Family / 家族 | Task / 任务 | Trials / 次数 | Native job cap (s) / 原生作业上限 | Plan digest / 计划摘要 |
|---|---|---:|---:|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | 2 | 900 | `25edf422fd80ada6843311f0df0d7c530cb27f783677df34b9b5d0e20a993509` |
| terminal-bench-3.0 | `batched-eval-parity` | 2 | 900 | `e47cbc4bd1c8d6d0b0641f14744ebb4c2f3c3252d0d9e13199095c97d95535d1` |
| deepswe-v1.1 | `abs-module-cache-flags` | 2 | 1800 | `a616c7ed6222bb247a259145f932bda7e26524dcacc5c81f0773a6889913dec0` |

| Family / 家族 | Material SHA-256 / 材料摘要 |
|---|---|
| terminal-bench-2.1 | `865548a1fc487b61ecaa923651190db247566b9e877fc32edd52c891c900bc93` |
| terminal-bench-3.0 | `8798fc9e817c36a933de361472746b074b87a89e5ca7005fe2140ac6a4ad69bc` |
| deepswe-v1.1 | `a2cf5cd83a577b76f1d070785e74cd9b47170ad1a0dbf8f7128d02faf42f169f` |

| Trial / 试验 | Family / 家族 | Subject / 被测对象 | State / 状态 |
|---|---|---|---|
| `trial-baseline-pi-next-openssl-selfsigned-cert-r1` | terminal-bench-2.1 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-openssl-selfsigned-cert-r1` | terminal-bench-2.1 | candidate-opi-head | unexecuted / 未执行 |
| `trial-baseline-pi-next-batched-eval-parity-r1` | terminal-bench-3.0 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-batched-eval-parity-r1` | terminal-bench-3.0 | candidate-opi-head | unexecuted / 未执行 |
| `trial-baseline-pi-next-abs-module-cache-flags-r1` | deepswe-v1.1 | baseline-pi-next | unexecuted / 未执行 |
| `trial-candidate-opi-head-abs-module-cache-flags-r1` | deepswe-v1.1 | candidate-opi-head | unexecuted / 未执行 |

Per-trial native job caps are 900/900/1800 seconds by family, including setup, Agent and grading; their sum across six serial trials is 7200 seconds. Native stdout/stderr each have a 1 MiB cap. There is **no enforced total cost, total token or total wall-clock cap**. Acquisition/cleanup overhead is outside that timeout sum; configured timeout overrides have no native enforcement route in this build. No paid retry is authorized. Full private capture is retained; sharing requires a later privacy review without editing sealed bytes. Invalid, cancelled, unstarted and effect-unknown trials stay visible.

## Verification and preparation history

```sh
# cwd: /mnt/d/Luiz/Odradek/opi; retained external Cargo target; incremental enabled
cargo test --locked -p opi-ai --test anthropic_thinking_roundtrip
cargo build --locked -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
CC_x86_64_unknown_linux_musl=gcc cargo build --locked --release --target x86_64-unknown-linux-musl -p opi-coding-agent --bin opi --message-format=json-render-diagnostics
cargo test --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact
```

All four commands exited 0: six existing signature tests passed, both builds succeeded, and the one output-control test passed. Exact cwd, selected cache/probe environment, argv, timing and outputs are retained in [commands](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/commands) and [new-plan commands](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/commands). Test impact: **retain**; no repository tests added, updated or deleted. The standalone reproduction script is diagnostic evidence outside the repository. No workspace gate is claimed for this documentation-only handoff.

```sh
# cwd: /home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
# f = terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
./builds/opi-eval doctor --config inputs/$f.toml
./builds/opi-eval prepare --config inputs/$f.toml --out materials/$f
./builds/opi-eval validate --config inputs/$f.toml --native-material materials/$f/material.json
./builds/opi-eval plan --config inputs/$f.toml --native-material materials/$f/material.json --out plans/$f.json
```

All final doctor/prepare/validate/plan commands exited 0 for all three families; exact absolute arguments and derive/pinned validation passes are in [preparation records](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/checks/preparation-commands.json). All three actual task-image `opi --version` checks exited 0 without networking; [image identities and commands](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/next-acceptance/checks/task-image-executable-smoke.json) are retained.

Preparation failures are preserved: initial runner symlinks were rejected (exit 2), so independent runner copies were used; initial offline uv sync missed a build backend in the old per-run cache (exit 1 for both runners), then succeeded using the existing global uv cache (exit 0 for both). `ubuntu:24.04` image-tag inspection failed because the base exists only in BuildKit cache. Full `opi-cargo-cache.py status` was interrupted with exit -15 after a lengthy scan; narrow cache resolution and the owned lease release exited 0. None of these is recorded as a benchmark attempt.

New native `run`, seal `verify`, report reconstruction, acceptance verification, capture audit and native regrade are **not run** because no new live trial exists; regrade was not requested. New real-task coverage is 0/6, six unexecuted, no native score/denominator or live usage/cost measurement. P19-AC-001, the real portions of P19-AC-003 through P19-AC-006, and P19-AC-012 remain missing for this new candidate. The previous acceptance findings remain diagnostic/unverified; no independent audit admission or implementation-ledger update occurred.

## Preservation and next action

[Cleanup](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/cleanup.json) completed for owned disposable containers: the proxy container and three version-smoke containers are absent. Four reusable task/proxy image identities were verified present. Reproduction files, candidates, plans, caches and original evidence are retained. The pre-existing lease 133324 belongs to an unestablished earlier owner and remains intact; this invocation's lease 214323 ended and is absent. Reclaimed bytes are unknown; no image or shared cache was pruned.

[Preservation checks](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/preservation.json) match all 694 files in the prior private evidence inventory plus 48 original preparation records. The new 39-file protected inventory also matches after cleanup. No real credential was loaded or staged; diagnostic copies contain synthetic values and private operator paths, and remain local-only. Final `python3 scripts/opi-doc-check.py` and `git diff --check` results are recorded in [final checks](E:/opi/evaluations/20260910t154500z-offline-diagnosis-opi082/checks/final-checks.json).

The next action requires approval of this exact new candidate and six paid trials. After approval, run each recorded `proposed_execution` command in the operator scope, preserve all settlements, then verify, reconstruct reports, review privacy and attempt acceptance admission. No additional code repair, commit or publication is proposed.
