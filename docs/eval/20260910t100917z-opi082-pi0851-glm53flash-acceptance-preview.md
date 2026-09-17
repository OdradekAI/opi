# Acceptance preparation: Opi 0.8.2 and pi 0.85.1

[简体中文](20260910t100917z-opi082-pi0851-glm53flash-acceptance-preview.zh.md)

**Prepared; live authorization pending.** Three native plans passed material
validation and resource preflight. No paid trial has started. Acceptance remains
blocked pending authorization, six qualifying native results, privacy review,
seal verification and the offline acceptance checks. This is a preparation
report, not a benchmark result or Phase completion claim.

## Evidence and build identity

- Private operator root: `/home/luiz/opi-eval-runs/20260910t100917z-acceptance-opi082-pi0851` in WSL Ubuntu.
- Scope: `operator-scope.json`, SHA-256 `16f9a80798479e574f5f4e9b4c2278940b3dcd2ecd3e18811c0f2fd82559d553`.
- No acceptance `manifest.json`, native normalized report or sealed run exists for this invocation.
- Opi: released `v0.8.2` source `765608a55a78db47e8b41d2639c29ef8c90e0f31`, clean; cached musl build SHA-256 `f1bd9bbf82d03120427e4de42318f31e939c0a94dde99705f4fdde9eabf063ee`. The retained build command and selected bytes agree; `--version` returned `opi 0.8.2`.
- pi: official standalone Linux x64 `0.85.1` package, source `d981de1229ef899957bbe968bc8dcda02a21f477`; executable SHA-256 `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403`. The complete package is bound by the material/probe; `--version` returned `0.85.1`.
- Companion, separately built from clean current source `55fbc95009e6f47145196dbc539454ebcb1cf692`: package `0.8.2`, executable SHA-256 `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`.
- Toolchain: Rust `1.97.1`, Linux x86_64. Build records, lock hashes and compiler-artifact locator are under `builds/` and `commands/`.
- Harbor: `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`; Pier: `c1ebc6d145b40fae8425215e3fca528945065124`. Both final cached checkouts and their locked virtual environments passed preparation.

## Frozen scope and coverage

Each task has one pi trial and one Opi trial, one repetition, concurrency 1,
and `max_trials = 2` per family. The plans bind one admitted package per family;
their corpus count of 1 is not the size of the complete upstream benchmark.

| Family | Selected task | Planned / attempted / valid | Native job timeout | Plan digest |
|---|---|---|---|---|
| Terminal-Bench 2.1 | `openssl-selfsigned-cert` | 2 / 0 / 0 | 900 s | `02215f12563dbebbec7ca91258383613f780f878a78d8d9e3edbceb69a51aa45` |
| Terminal-Bench 3.0 | `batched-eval-parity` | 2 / 0 / 0 | 900 s | `62b5f4b11ddff301f8415f4c14b8e03e73e3ad156bc7f7789b8bbf81bfcfcbc6` |
| DeepSWE v1.1 | `abs-module-cache-flags` | 2 / 0 / 0 | 1800 s | `5a0c47671489748610b29f258c793700ad1dc9a13fc7e0d98d33372dfbf753fc` |

All six trials are unexecuted; both sides of all three comparisons are missing.
There is no score or graded denominator. Trial IDs, plan-file SHA-256 values,
material hashes and proof references are preserved in `operator-scope.json`.

The selected connection is Zhipu `glm-5.3-flash`, API `anthropic-messages`,
endpoint `https://open.bigmodel.cn/api/anthropic`. The user selected the repository
`.env`; only the name `OPI_BIGMODEL_API_KEY` was identified. Authorized execution
must load that variable into the Companion's environment, not pass the entire
dotenv file as a raw credential. No real credential value has been resolved.

Proposed controls: output limit 8192, declared context window 131072, temperature,
top-p and reasoning omitted. Output and context controls are required. The
exact-client loopback probe observed `max_tokens: 8192` on the initial request and
tool continuation for both clients. This proves that selected value only, not
arbitrary output-limit support in released Opi 0.8.2. Context metadata is projected
into both clients; service capacity and actual reasoning behavior remain unobserved.

Native job timeouts include setup, Agent execution and grading. Their six-trial
sum is 7200 seconds; preparation and cleanup are additional. Each job's supervisor
stdout/stderr cap is 1 MiB. Configurable timeout overrides are not applied by this
native dispatch, so none is promised. There is no total-token, monetary or overall
wall-clock cap. Provider access, actual usage, billed cost and live durations are
unknown because no live execution occurred. No paid retry is authorized.

## Platform, materials and capture

WSL2 Ubuntu and Docker's default Linux engine reported x86_64, 32 logical CPUs and
16611237888 memory bytes. The selected task environments require 1/1/2 CPUs and
2/4/8 GiB respectively. Resource preflight reported no unmet or unknown selected
requirements. This does not prove a successful live task or grader run.

The exact task/base image digests were initially absent from this engine. They
were pulled by the static-lock digests and their local aliases checked. The
unmodified TB3 Agent/verifier and DeepSWE verifier Dockerfiles built successfully;
identities and command logs are in `checks/prepared-image-builds.json` and
`commands/pull-*.json`. Provenance-only/build-tool lock entries are not claimed as
downloaded runtime images. No oracle or native grader was invoked.

The initial full local Harbor clone failed on an unreadable historical object.
Fetching only the pinned commit into a fresh independent shallow repository
succeeded; its source/lock and offline dependency installation were then validated.
Original caches and the failed command record remain intact.

Capture is full and private. Agents use the integration's isolated
`/opi-eval-agent` stage and `/logs/agent` capture mount. Inspection confirmed the
integration's credential-stage cleanup and Companion settlement cleanup paths;
their live operation is still unverified here. Any later sharing requires review
of original sealed bytes; sensitive sealed artifacts cannot be edited or resealed.

## Executed verification

Exact argv, cwd and exits for build, acquisition, preparation, validation, planning
and the probe are retained under `commands/`. With `R` denoting the operator root
and `F` each family from the table, the executed commands were:

| Command | Cwd | Result |
|---|---|---|
| `cargo build --locked --release -p opi-eval --bin opi-eval --message-format=json-render-diagnostics` | Opi checkout | Exit 0 |
| `cargo test -p opi-eval --test native_driver native_output_control_client_matrix -- --ignored --exact` | Opi checkout | Exit 0; 1 passed, 0 failed; synthetic loopback authentication only |
| `$R/builds/opi-eval doctor --config $R/inputs/$F.toml` | `$R` | All 3 exit 0 |
| `$R/builds/opi-eval prepare --config $R/inputs/$F.toml --out $R/materials/$F` | `$R` | All 3 exit 0 |
| `$R/builds/opi-eval validate --config $R/inputs/$F.toml --native-material $R/materials/$F/material.json` | `$R` | All 3 exit 0 for integrity derivation, pin confirmation and final validation |
| `$R/builds/opi-eval plan --config $R/inputs/$F.toml --native-material $R/materials/$F/material.json --out $R/plans/$F.json` | `$R` | All 3 exit 0; 2 trials each |

The probe's exact environment references are in `commands/output-control.json`.
Native `run`, `verify`, `report`, acceptance verification and the Opi capture audit
are not run: no trial evidence exists. Native regrade was not requested. Workspace
runtime gates are not claimed. Test impact: **retain**; no test/source files changed.
Documentation-check results are retained separately with this preparation handoff.

## Cleanup and remaining acceptance work

Preparation cleanup completed; see `cleanup-preparation.json`. Two empty version
probe homes were removed. All 48 protected files matched their prior hashes,
final material validation passed for all three families, and no containers remained.
Reusable images, runner/task/client caches, proof artifacts and command records
are retained for the pending run. Space reclaimed is unknown. Shared caches and
historical evidence were preserved; build/probe Cargo leases were released.

`P19-AC-001` live assembly, real portions of `P19-AC-003`–`P19-AC-006`, and
`P19-AC-012` are **missing** for this invocation. Prepared inputs and a passing
loopback probe do not close them. The next action is operator authorization of
this exact six-trial scope, followed by native execution, preservation of every
settlement, privacy review and offline evidence verification. No implementation
ledger, final Phase audit, commit or publication was created.
