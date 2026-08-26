# Phase 18 external execution locks and Linux native-smoke contract

Date: 2026-08-26

Status: research evidence; not a design, ADR, implementation ledger, or completion record

Scope: Opi/pi executable identity, Terminal-Bench 2.1, Terminal-Bench 3.0,
DeepSWE v1.1, native verifier provenance, and Linux x86_64 CI evidence

## Executive finding

Phase 18 now has enough primary evidence to replace floating product names with
immutable source candidates and an executable CI contract, but it is **not yet
fully admitted for implementation**.

The source revisions, tool source locks, selected smoke-task subtrees, and two
prebuilt image manifests can be frozen now. Opi/pi executable hashes and the
Terminal-Bench 3.0 environment/verifier image hashes are build products and must
be resolved and sealed by the Linux job. Terminal-Bench 2.1 has an additional
blocker: its 89 official task verifiers all perform runtime package acquisition
through paths such as `apt`, `curl`, `pip`, or `uvx`. A repository commit and
task-image digest therefore do not yet satisfy Phase 18's exact-integrity rule
for every native tool and package.

The implementation plan should add an external-lock materialization gate before
the Agent and benchmark adapters, and should make one Linux x86_64 Phase-exit
job the sole owner of the real native-smoke artifact. It should not repeat that
smoke in the assembly, reporting, and final-assurance tasks.

## 1. Research question

Which immutable inputs and run-produced identities are required to execute real
Opi and earendil pi processes against one complete official task from each of
Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1, preserve their native
verifier authority, and produce one reviewable Linux x86_64 Phase-exit artifact
without paid providers or hosted benchmark services?

The answer must distinguish:

- immutable source inputs that can be checked in now;
- image, executable, and output digests that exist only after an admitted Linux
  build/run;
- unresolved supply-chain closure that must block admission rather than be
  represented as complete; and
- conformance evidence from public benchmark-score or leaderboard claims.

## 2. Relation to pi

Phase 18 targets the current earendil coding-agent process, not pi Harness v2.
The exact upstream source is tag `v0.84.3`, commit
[`4e58f324fae8ebfa98a3d45181fb248072a2afac`](https://github.com/earendil-works/pi/commit/4e58f324fae8ebfa98a3d45181fb248072a2afac).
Its package manifest declares version `0.84.3`, package
`@earendil-works/pi-coding-agent`, and the installed `pi` entry
`dist/bundle/cli.js`; the normal build creates that bundle, while
`build:binary` is a separate Bun-compiled path. See the immutable
[`package.json`](https://github.com/earendil-works/pi/blob/4e58f324fae8ebfa98a3d45181fb248072a2afac/packages/coding-agent/package.json)
and the repository's pinned local evidence at
[`packages/coding-agent/package.json`](../../.repo/pi-0.84.3/packages/coding-agent/package.json).

The Phase 18 executable identity for pi is therefore composite rather than a
single native file:

1. source commit and package manifest digest;
2. root `package-lock.json` and package `npm-shrinkwrap.json` digests;
3. exact Node/npm versions and Node executable digest;
4. built `dist/bundle/cli.js` digest;
5. installed package-tree/lock identity; and
6. structured argv, isolated configuration, and adapter digest.

The source build is preferable to a floating global npm install because it can
prove the Git revision and build recipe directly. An npm tarball remains a
viable alternative only if its registry integrity is locked and its unpacked
manifest and bundle are verified against this source revision.

## 3. Primary findings

### 3.1 Lock taxonomy

| Identity | Lock time | Required evidence |
|---|---|---|
| Git source and task package | Before dispatch | Repository URL, full commit, root/tasks/task Git trees, canonical package manifest digest |
| Python/Node/Rust tool input | Before build | Exact version, immutable action/source commit, upstream archive or lock digest |
| Registry image input | Before build/run | Repository, original tag for provenance, Linux/amd64 manifest digest; never tag alone |
| Opi/pi executable | After Linux build, before Agent start | Resolved absolute path, SHA-256, file/runtime metadata, source and lock identities |
| Built task/verifier image | After build, before trial | OCI manifest/config digest, base-image digests, build context digest, BuildKit identity |
| Native verifier result | After verification | Raw stdout/stderr, exit status, native reward/CTRF files, and per-file plus manifest digests |
| Uploaded Phase artifact | After sealing | Canonical artifact manifest, GitHub artifact ID/URL, and upload action's SHA-256 digest |

Git SHA-1 tree identities below are authoritative immutable source locators, but
they do not replace Phase 18's SHA-256 artifact manifests. After checkout, the
lock materializer should hash each task package as a sorted manifest of relative
path, file role/mode, size, and SHA-256, then hash the canonical manifest.

### 3.2 Opi and pi build, location, and digest contracts

#### Opi

The current product manifest declares binary `opi` in
[`crates/opi-coding-agent/Cargo.toml`](../../crates/opi-coding-agent/Cargo.toml).
The repository MSRV is Rust `1.97`; Phase 18 should use that exact toolchain for
its first admitted lock rather than floating `stable`.

The Linux job should build with a machine-readable artifact stream:

```sh
cargo build --locked --release \
  -p opi-coding-agent --bin opi \
  --message-format=json-render-diagnostics
```

It must select the `compiler-artifact` whose target name is `opi` and use its
non-null `executable` field. It must not assume `target/release/opi`, because the
workspace supports an external Cargo target/cache path. Before any Agent starts,
record `readlink -f`, `sha256sum`, `file`, `ldd`, `rustc -Vv`, `cargo -V`, the
checkout commit/dirty state, target triple, profile, feature set, and
`Cargo.lock` SHA-256. The currently observed `Cargo.lock` digest is
`d19619f4c05b0bb00f324345f8103d615be6bcae0462cff5b9b8dbf3cce0cd2a`;
the CI checkout must recompute and compare it rather than trusting this report.

The actual invocation shape is:

```text
<resolved-opi> --config <isolated-config> --model scripted:phase18
  --json --json-compact --allow-mutating --no-trust
  --trace <fresh-trace-root> --tools <reviewed-list> <prompt>
```

The exact current flags are owned by
[`cli.rs`](../../crates/opi-coding-agent/src/cli.rs) and documented in
[`README.md`](../../crates/opi-coding-agent/README.md). Each trial gets fresh
`HOME`, XDG config/data/state, trace, session, and task directories. Only the
explicit config and dummy scripted-provider credential may enter the allowed
environment projection.

#### pi

At the pinned source revision, the immutable input digests are:

| Input | Git blob | SHA-256 |
|---|---|---|
| `package-lock.json` | `37842d14c45e87e34a19e0f79dbbed54a71cce23` | `abd9982ce837df8e2775196cde3b5c5584c30e66b232c26b7c55aafd58ab2e1b` |
| `packages/coding-agent/npm-shrinkwrap.json` | `f65c23df9a50204b11484d91ccbb4d2618534acf` | `df18751951cef578a1e5d809b33774bb385f69539f197bade0e70f1944aa3de7` |
| `packages/coding-agent/package.json` | `02f45a37b3aa707af5bf7c99e8c3dc041c88cba5` | `6c0bfa4558a0de32fb54fa97ca73397753ec7d4ad7d0ba607790920168de751f` |

The pinned runtime candidate is Node `22.19.0`, the minimum declared by the pi
source. The official Linux x64 archive SHA-256 is
`c0649af18e6a24f6fe5535a3e86b341dd49a8e71117c8b68bde973ef834f16f2`
for `node-v22.19.0-linux-x64.tar.xz`, from Node's
[`SHASUMS256.txt`](https://nodejs.org/dist/v22.19.0/SHASUMS256.txt).

Build and locate with:

```sh
npm ci --ignore-scripts
npm run build
test -f packages/coding-agent/dist/bundle/cli.js
```

Record the Node executable and bundle SHA-256 values after the build. Invoke the
bundle explicitly with the resolved Node executable so the runtime is part of
the command identity:

```text
<resolved-node> <pi-source>/packages/coding-agent/dist/bundle/cli.js
  --print --mode json --no-session --no-approve
  --provider scripted --model scripted/phase18 --api-key <redacted-dummy>
  --thinking off --tools <reviewed-list> <prompt>
```

Use a fresh `PI_CODING_AGENT_DIR`, `HOME`, and XDG projection. The isolated
`models.json` selects `openai-completions`, the same loopback endpoint and model
identity as Opi, and contains no command substitutions or ambient credential
fallback. pi's custom-model contract is documented in the pinned
[`docs/models.md`](https://github.com/earendil-works/pi/blob/4e58f324fae8ebfa98a3d45181fb248072a2afac/packages/coding-agent/docs/models.md).

#### Shared scripted provider

Both products can use an explicit OpenAI-compatible Chat Completions endpoint.
The provider fixture must itself be a checked-in executable with a source and
executable digest, bind only to the admitted local/Docker network, and emit
deterministic streaming tool-call responses. Request order, normalized request
digests, response-script identity, and server logs are native smoke evidence.
No paid key, live-provider fallback, or user configuration is allowed.

The native benchmark smoke should not embed reference solutions in the scripted
provider. Run each official oracle first and require its native passing reward;
then let Opi and pi make the same harmless, non-oracle workspace mutation. A
native reward of zero is still a valid integration result if the verifier ran
to completion and its zero remains authoritative. Requiring the scripted Agents
to score one would turn the smoke fixture into an oracle leak unless a separately
reviewed non-secret task solution is admitted.

### 3.3 Exact benchmark source and task-package candidates

| Benchmark | Execution source lock | Complete tasks tree | Count | Recommended smoke task and subtree |
|---|---|---:|---:|---|
| Terminal-Bench 2.1 | `harbor-framework/terminal-bench-2-1` commit [`7131e4375048a0e408a8fb404b5f499d726b695b`](https://github.com/harbor-framework/terminal-bench-2-1/commit/7131e4375048a0e408a8fb404b5f499d726b695b) | `2f0f5fdc68f0befd9b4745386eb8698264b00d8a` | 89 | `openssl-selfsigned-cert`, tree `4c5b1214db4b807f2bc4c1bff8803402e36b648b` |
| Terminal-Bench 3.0 | canonical repository `harbor-framework/terminal-bench`, tag `v3.0.0`, commit [`2b0442c3c583b710ca8da14c8e601b99f2f1f244`](https://github.com/harbor-framework/terminal-bench/releases/tag/v3.0.0) | `a10dbfde7cd4d1c3ceaf22ed39b52a98dc775d54` | 74 | `batched-eval-parity`, tree `729916599199dd1de6be0e0c543da9788d6129b5` |
| DeepSWE v1.1 | `datacurve-ai/deep-swe` commit [`435ee89ec2f2e2289f33b0da4f992f0b7b7266b9`](https://github.com/datacurve-ai/deep-swe/commit/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9) | `66df25a1b382017d0ae014d94cadb2698baaed48` | 113 | `abs-module-cache-flags`, tree `8ee76a3e4a876a9bd24f34edabcdde4e47257db4` |

Terminal-Bench 2.1 has no Git tag or GitHub release. The full commit is therefore
the version lock; `main`, `latest`, and the Harbor Hub `latest` dataset are not
admissible identities. Its official README describes 2.1 and the complete local
task package at the selected
[`README.md`](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/README.md).

The Phase design's `harbor-framework/terminal-bench-3` URL redirects to the
canonical `harbor-framework/terminal-bench` repository. Tag `v3.0.0` is the
immutable 3.0 lock; current default-branch names and later Frontier-Bench/v4
content must not be substituted.

DeepSWE also has no `v1.1` Git tag. Commit
[`8cae5984d5dd0ee37445beff0e928dc10c331116`](https://github.com/datacurve-ai/deep-swe/commit/8cae5984d5dd0ee37445beff0e928dc10c331116)
is the explicit `DeepSWE V1.1` version anchor and already has 113 tasks, but it
uses the older `pre_artifacts.sh` collection path. The recommended execution
lock is `435ee89...`: it retains 113 tasks and adds the `[[verifier.collect]]`
patch handoff required by the Phase design and current DeepSWE README. The
immutable
[`README.md`](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/README.md)
states that v1.1 uses a separate pristine verifier and Pier newer than 0.3.0.
The implementation plan should record both the semantic version anchor and the
chosen executable revision; it must not pretend that `435ee89...` is a tag.

The proposed tasks are deliberately small smoke representatives, not a score
subset:

- Terminal-Bench 2.1 `openssl-selfsigned-cert` uses 1 CPU, 2 GiB memory, and the
  older shared native-verifier lifecycle.
- Terminal-Bench 3.0 `batched-eval-parity` declares `/app/evalbench/`, a separate
  verifier container, 1 CPU, and 4 GiB per environment.
- DeepSWE `abs-module-cache-flags` declares a collected binary patch, separate
  no-network verifier, immutable upstream base commit, and 2 CPU/8 GiB limits.

These IDs remain implementation-plan outputs until human review pre-registers
them. Selection must not be changed after seeing Opi/pi results.

### 3.4 Exact external runner locks

Terminal-Bench 2.1 and 3.0 can run from their local task directories through
Harbor. DeepSWE's own current instruction selects Pier for its patch collection
and pristine verifier path.

| Runner | Source/version | Source commit | Lock input | SHA-256 |
|---|---|---|---|---|
| Harbor | `v0.22.0` | [`4407eb5227a2ff4f0d3f16b2eb48849382fdf276`](https://github.com/harbor-framework/harbor/releases/tag/v0.22.0) | `uv.lock` Git blob `1c3995feda2d52cbf822a7d378294065bfd64e09` | `bfc3b39202ec3a04a379a2ff9a8c887c1cd55a5cdf3e7ede7f0d657d35a4d95a` |
| Pier | `v0.3.1` | [`df89f994623a0a6a57229103b6fe910766693c30`](https://github.com/datacurve-ai/pier/commit/df89f994623a0a6a57229103b6fe910766693c30) | `uv.lock` Git blob `c479fa26584b198250f4ebba68aac8941ebe158d` | `6261c632e80ee65e327c23f450c7cc5c38e34f2ee941d3ebe96ff72fc91f6de9` |

Run each tool from an exact checkout with `uv run --locked`; do not install
`harbor`, `datacurve-pier`, or their transitive dependencies from unconstrained
PyPI resolution. Pin uv `0.9.7`; its official Linux x86_64 GNU archive digest is
`b26fcc8dfa1c39b5a5613445af3be3eefda45d9a39359bee271eafe34913583e`
([official release](https://github.com/astral-sh/uv/releases/tag/0.9.7)).

Harbor's immutable 0.22 documentation confirms local execution with
`harbor run -p <path>` and that separate verifiers receive only declared
artifacts at their original paths. See
[`run-evals.mdx`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/docs/content/docs/run-jobs/run-evals.mdx)
and
[`tasks/index.mdx`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/docs/content/docs/tasks/index.mdx).
Pier 0.3.1 documents local task paths and Docker execution in its immutable
[`README.md`](https://github.com/datacurve-ai/pier/blob/df89f994623a0a6a57229103b6fe910766693c30/README.md).

### 3.5 Image locks and build-produced images

The following manifests were resolved from their registries on 2026-08-26.
They are candidate lock values, not claims that their original tags are
immutable.

| Role | Original reference | Linux/amd64 or single manifest digest | Admission |
|---|---|---|---|
| TB 2.1 task image | `alexgshaw/openssl-selfsigned-cert:20251031` | `sha256:4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12` | Can be frozen now; resolved config/layers must also be captured |
| TB 3.0 Dockerfile base | `python:3.11-slim` | `sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc` | Input only; both final task and verifier OCI digests are run-produced |
| DeepSWE task image and verifier base | `public.ecr.aws/d3j8x8q7/swe-bench-202605:kh75679ajj3b8dtd7se3h7z0a1833y6r-v1.1` | `sha256:3a4d47f5281269305343c83729836ac2f3172811aee72681e472a4196178eda1` | Can be frozen now; final verifier image is run-produced |
| DeepSWE source-build provenance | `public.ecr.aws/x8v8d7g8/mars-base:latest` | `sha256:9c024d5dc46f32cee946353232ef99d8f714841ed08bba9b8fb9bb237c77f1ba` | Provenance only; prefer the task's prebuilt image |
| BuildKit | `moby/buildkit:buildx-stable-1` | `sha256:040d34121c27906c4ff9ac152a30d52bf2c5d328d3bb748916bb3d2743c02528` | Pin by digest; observed version v0.32.2 |

The resolver should pull by digest and either supply the digest directly in the
resolved execution profile or seed the exact local tag before an offline build.
The official task package remains byte-identical. Every built image is exported
to OCI form and its manifest/config/layer digests are sealed before any trial.
A Docker image ID or mutable tag alone is insufficient.

#### Terminal-Bench 2.1 integrity blocker

The selected task's immutable
[`tests/test.sh`](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/tests/test.sh)
runs `apt-get update`, installs `curl`, downloads the uv 0.9.5 installer, and
resolves pytest packages with `uvx`. A repository-wide inspection found the
same class of runtime installer in every one of the 89 `tests/*/test.sh` files;
no candidate avoided dynamic acquisition.

This leaves three honest options:

1. materialize an exact verifier dependency closure, including apt snapshot or
   `.deb` hashes, uv installer/binary hashes, Python wheel hashes, and a
   controlled offline cache, then prove the unmodified native verifier consumes
   only that closure;
2. use an upstream-published immutable verifier image/lock if Terminal-Bench
   adds one; or
3. explicitly revise the Phase integrity requirement.

Option 1 is the only current route consistent with the Phase source, but it
requires a Linux dry run and should be a separate lock-materialization task.
Recording only the versions printed after an online run proves what happened
once; it does not make the input closure exact or reproducible.

### 3.6 Native verifier identity

For each selected task, `native_verifier_identity` should include all of:

- benchmark commit, complete tasks tree, selected task tree, and canonical
  package SHA-256 manifest;
- exact `task.toml`, verifier entrypoint, tests/verifier Dockerfile, grader and
  held-out file Git blob identities;
- Harbor/Pier source commit, executable digest, Python executable/version, and
  locked environment digest;
- task and verifier image OCI identities, platform, resources, network mode,
  timeouts, artifact declarations, and collect hooks;
- structured argv and allowed environment digest;
- collected input artifact manifest and digest;
- native stdout/stderr, exit status, reward/CTRF/report files, and their
  individual and aggregate digests; and
- failure owner: task, Agent, collection, verifier, runner, image, timeout,
  cancellation, or infrastructure.

The normalized Opi report may project the native reward, but it must retain the
native bytes and native metric names. A cached reward or previously recorded
parser fixture never satisfies real native verification.

### 3.7 Linux x86_64 native-smoke CI contract

The native smoke should be a dedicated, manually authorized workflow or job,
not a normal Opi startup path and not an every-PR benchmark run.

Minimum static contract:

```yaml
name: Phase 18 Linux native smoke

on:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: phase18-linux-native-smoke
  cancel-in-progress: false

jobs:
  native_smoke:
    if: github.repository == '<owner>/opi'
    runs-on: ubuntu-24.04
    timeout-minutes: 360
```

Pin every action by full commit. Candidate audited inputs are:

| Action | Full commit |
|---|---|
| `actions/checkout` v4.2.2 | `11bd71901bbe5b1630ceea73d27597364c9af683` |
| `dtolnay/rust-toolchain` 1.97.0 | `889fac408b4da0905346410f253f0c55fbcb6613` |
| `actions/setup-node` v4.4.0 | `49933ea5288caeca8642d1e84afbd3f7d6820020` |
| `astral-sh/setup-uv` v6.7.0 | `b75a909f75acd358c2196fb9a5f1299a9a8868a4` |
| `docker/setup-buildx-action` v3.11.1 | `e468171a9de216ec08956ac3ada2f0791b6bd435` |
| `actions/upload-artifact` v4.6.2 | `ea165f8d65b6e75b540449e92b4886f43607fa02` |

Set Node `22.19.0`, uv `0.9.7` with its release checksum, Rust `1.97.0`, and
BuildKit by the digest above. GitHub's `ubuntu-24.04` label still receives runner
image updates, so the job must capture `ImageOS`, `ImageVersion`, `uname -a`,
`/etc/os-release`, CPU, memory, disk, cgroup mode, Docker client/server/buildx
versions, and BuildKit image digest. If byte-repeatable host identity is needed
across later runs, use an immutable self-hosted VM image; a hosted-runner label
alone is evidence of one run, not a permanent host lock.

The job runs sequentially to bound memory and disk:

1. checkout Opi and verify an expected clean commit;
2. fetch each external repository at its full commit and reject any tree drift;
3. install exact Node, Rust, uv, Harbor, Pier, Docker/BuildKit inputs and record
   executable plus lock digests;
4. resolve/pull/build every image, export OCI metadata, and freeze the resolved
   experiment before starting a product process;
5. build and locate Opi and pi as specified above, then freeze their executable
   identities;
6. start the content-addressed local scripted provider and verify that both task
   containers can reach only the admitted endpoint during the Agent phase;
7. run one upstream oracle preflight per task and require a valid passing native
   result;
8. run exactly six paired Agent trials: Opi and pi for each of the three tasks;
9. require each native verifier to complete, preserve a zero reward as a valid
   outcome, and reject any fallback, cached result, heuristic, or model judge;
10. seal bundles and the conformance-only report, run canary/redaction and
    package/artifact audits, and generate a sorted SHA-256 manifest; and
11. upload safe diagnostics on failure and the sealed artifact only after
    redaction/sealing succeeds.

The planned repository-level command can retain the Phase source's shape:

```sh
cargo run --locked -p opi-eval -- \
  conformance phase18-linux-native \
  --lock <resolved-native-lock.json> \
  --artifact-dir <fresh-private-staging>
```

The exact final flags remain an implementation-plan output because `opi-eval`
does not yet exist. The command must consume a pre-resolved lock; it must not
resolve `latest`, select tasks, or widen network/tool authority at runtime.

Do not cache verifier outputs, sealed bundles, resolved experiment files, or
native rewards. Dependency caches are optional only when keyed by every exact
lock and revalidated before use; the simpler Phase-exit job should omit them.
Upload action v4 exposes an `artifact-digest` SHA-256 output; retain it together
with artifact ID/URL and the in-artifact manifest. Raw traces and task workspaces
stay private staging unless their export classification passes.

One successful, unexpired artifact closes the native-smoke evidence. Assembly,
offline report, seam review, and final assurance consume and verify it; they do
not rerun it unless an Agent, benchmark, task, runner, image, provider, adapter,
authority, or host lock changed.

## 4. Alternatives considered

### Published pi npm tarball instead of source build

This shortens CI, but a package version alone loses source/build identity. It is
acceptable only with exact npm integrity, unpacked tree digest, package manifest
check, bundle digest, and a proved mapping to pi commit `4e58f324...`. Source
build is the clearer initial contract.

### Harbor Hub datasets instead of Git task packages

Rejected for Phase 18. Hub `latest`, login, download metadata, and hosted upload
add authority and mutable identities that the Phase explicitly excludes. Local
complete packages at immutable Git revisions satisfy the native task contract.

### One runner for all three benchmarks

Harbor 0.22 can describe the task shapes, but DeepSWE explicitly selects Pier
for its agent network and patch/pristine-verifier lifecycle. Using Pier for
DeepSWE and Harbor for the two Terminal-Bench revisions preserves upstream
ownership and avoids claiming unproved parity. A later shared runner is possible
only after native conformance proves it.

### Reimplement the task protocol in Rust

Rejected. It would duplicate complex environment, artifact, network, and
separate-verifier semantics and create a new authority surface. Rust should
validate identities and supervise exact external executables, not become a
second partial Harbor/Pier.

### Build all images online every run

This preserves source instructions but leaves mutable tags and repositories in
the build path. Prefer one admitted materialization that exports final OCI
images and exact base/provenance metadata, followed by digest-only use. Rebuild
only when a lock changes.

### GitHub-hosted versus immutable self-hosted Linux

`ubuntu-24.04` is sufficient for a one-run Phase-exit artifact if its exact
runner image/version and all outputs are disclosed. An immutable self-hosted VM
is stronger for later byte-repeatability and predictable disk, but it adds
infrastructure ownership and should not be required before the hosted dry run
proves that resource limits are inadequate.

## 5. Rust implementation feasibility

The work is feasible without a new public Rust abstraction or a native
benchmark reimplementation.

The independent Companion needs package-private mechanisms for:

- canonical external lock parsing and validation;
- source/task-tree and canonical file-manifest hashing;
- structured process argv/environment, bounded output, timeout, cancellation,
  and process-tree cleanup;
- executable/image/native-result admission and typed failure ownership; and
- immutable bundle sealing and offline verification.

The existing workspace already uses `serde`, `serde_json`, `sha2`, `hex`,
`tokio`, and `tokio-util`; Phase planning should verify whether the independent
Companion may use workspace dependencies without depending on an Opi crate.
External Git, Node, npm, Docker, Harbor, and Pier remain process dependencies.
No live provider SDK or benchmark crate belongs in the Rust dependency graph.

The largest feasibility risk is operational, not algorithmic: image size,
Docker disk pressure, provider reachability through no-network task policies,
and the Terminal-Bench 2.1 verifier dependency closure. The Linux dry run must
resolve these before claiming the job contract is green.

## 6. Fit with the existing extension and evidence model

The Evaluation runtime remains an Independent Companion. Agent-neutral
experiment resolution, external artifact identity, task execution, native
grading, pairing, bundles, and reports do not belong in the Reference Product
or Agent Core.

Opi's current process surface already supplies the required product seam:
one-shot NDJSON plus explicit Phase 17 trace/evidence capture. pi supplies its
own JSON process output and package/session facts. Their adapters keep native
bytes namespaced and project only proved common facts.

The existing Agent Core `EvidenceSink` and Reference Product file sink need no
widening. The Companion reads Opi's finalized evidence as a versioned private
adapter concern and captures pi's native stream separately. No benchmark image,
grader, scheduler, dataset, provider, or report policy should enter an Opi core
crate.

## 7. Smallest missing core seam

No missing Agent Core seam is demonstrated by this research.

Two internal Companion modules are justified because they have multiple real
callers:

1. an external artifact identity module shared by Opi, pi, benchmark, runner,
   image, provider, and native-result locks; and
2. an external process supervision module shared by Agent and native-runner
   invocations.

They should remain package-private and provisional through the full two-Agent,
three-revision matrix. Adding a public trait, crate, feature flag, or Agent Core
API would be premature.

## 8. Placement candidates and implementation-plan implications

| Concern | Placement | Confidence |
|---|---|---|
| Agent and benchmark profiles/locks | Independent `opi-eval` Companion | High |
| External artifact identity and process supervision | Package-private deep modules in the Companion | High |
| Opi trace import and pi JSON import | Adapter-private Companion modules | High |
| Real native smoke | Dedicated manually authorized Linux x86_64 workflow/job | High |
| Final bulky/private evidence | CI artifact plus Phase snapshot reference/digest, not repository source | High |
| Benchmark images, tests, graders, tools | Externally resolved artifacts | High |
| Agent Core or Reference Product | No new seam; process and evidence consumption only | High |

The next `opi-implement plan` revision should:

1. add an external-lock materialization/dry-run predecessor before the current
   Agent and benchmark adapter tasks;
2. move artifact identity ownership early enough to cover Opi/pi executables as
   well as benchmarks, tools, images, and verifier outputs;
3. record the exact revision/task/tool candidates above and make unresolved
   run-produced digests explicit outputs of the Linux materialization job;
4. block Terminal-Bench 2.1 admission until its dynamic verifier dependency
   closure is exact;
5. keep the local assembled run hermetic, but assign the actual Linux native
   smoke solely to the Phase-exit CI task and reuse its artifact for reporting
   and assurance;
6. ensure any benchmark or trajectory module enters the real crate graph when
   introduced rather than compiling only through test-local paths; and
7. keep task selection, public interface naming, and complete-matrix seam
   contraction provisional until the native evidence exists.

This report does not edit `.opi-impl-state.json`; only `opi-implement` may apply
those implications.

## 9. Unresolved questions and blockers

1. **Terminal-Bench 2.1 verifier closure:** exact apt repositories/packages, uv
   installer/binary, Python wheels, and transitive hashes still need a reviewed
   offline materialization.
2. **DeepSWE admission wording:** the plan owner must explicitly approve
   `435ee89...` as the executable v1.1 revision while retaining `8cae598...` as
   the semantic version anchor.
3. **Final image digests:** Terminal-Bench 3.0 task/verifier and the DeepSWE
   verifier image must be built/exported on Linux before their final OCI digests
   can be checked in.
4. **Executable digests:** Opi, Node, and pi bundle digests are necessarily
   produced by the admitted Linux build and cannot be copied from this Windows
   research host.
5. **Provider networking:** the exact Docker/Pier allowlist and host/service
   address must prove that DeepSWE's no-network Agent phase can reach only the
   scripted provider.
6. **Smoke success semantics:** Phase owners should confirm that an oracle pass
   plus authoritative Opi/pi native rewards, including zero, proves integration;
   otherwise they must admit a non-leaking task-specific solution source.
7. **Runner resources:** the first Linux dry run must measure disk, memory, and
   wall time. If GitHub-hosted capacity is insufficient, an immutable
   self-hosted runner becomes required.
8. **Artifact retention:** the Phase snapshot must preserve artifact ID/URL and
   digest without committing hidden, bulky, or export-restricted native data.
9. **Final CLI flags:** the exact `opi-eval` command cannot be frozen until its
   internal CLI exists; its authority and lock inputs are fixed above.

## 10. Limitations

- Research was performed from Windows with Docker manifest inspection but no
  running Linux Docker daemon. No official task, oracle, Agent, or native
  verifier was executed.
- Opi and pi were not built during this research, so no executable digest is
  claimed.
- Registry digests were observed on 2026-08-26. The report records them as
  digest candidates precisely because their source tags may move.
- GitHub-hosted runner capacity and exact image version were not exercised.
- The three proposed task IDs were inspected for contract shape and resource
  size, not for benchmark performance, statistical representativeness, or
  leaderboard eligibility.
- No Phase design, ADR, implementation task, ledger state, CI workflow, or
  runtime code was changed by this research.

## Appendix A. Terminal-Bench 2.1 offline closure addendum

Addendum date: 2026-08-26

Verdict: **the static dependency-research blocker is resolved for planning, but
`P18-SEC-003` is not yet satisfied by delivery evidence**. Every external byte
needed by the selected verifier now has an exact candidate version, first-party
URL, and SHA-256. A from-zero Linux x86_64 no-network feasibility run also
completed the official-oracle path with all six tests passing and the upstream
verifier bytes unchanged. The remaining evidence is run-produced:
Harbor/Docker execution locks, the final sealed closure artifact or OCI identity,
and an admitted native-smoke artifact from the Phase Linux job.

### A.1 Question

Can the complete official Terminal-Bench 2.1 task
`openssl-selfsigned-cert` at commit `7131e437...` run its native verifier with
network egress disabled, while every Debian, uv, Python, image, task, and
verifier input is pinned by exact provenance and SHA-256?

This addendum is intentionally limited to that one pre-registered task. It does
not generalize the closure to the other 88 Terminal-Bench 2.1 tasks.

### A.2 Relation to pi

This closure is independent of pi. Opi and pi are producers of the task
workspace; neither owns the benchmark's `tests/test.sh`, dependency installer,
pytest plugin, task image, or reward semantics. The benchmark adapter must
therefore preserve the official task/verifier bytes and record the closure as
external native-verifier provenance rather than packaging it as an Agent
feature.

### A.3 Primary-source findings

#### Complete official task package

The selected Git tree is still
[`4c5b1214db4b807f2bc4c1bff8803402e36b648b`](https://github.com/harbor-framework/terminal-bench-2-1/tree/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert).
It contains eight files; no hidden verifier Dockerfile, lock file, vendored
wheel, or alternate test entrypoint exists.

| Relative path | Git mode | Bytes | SHA-256 | Git blob |
|---|---:|---:|---|---|
| `.gitignore` | `100644` | 177 | `9a8dd8d9b6c3adb8e9e56328df27deef859a3e98fffc9b8133c8b22207c25953` | `ecd089e8ac7767b9bdb6f3ac247c8d033f7565ab` |
| `environment/Dockerfile` | `100644` | 259 | `f590b523038be43afcaf5291d7b9e8ed1333c48d9994e77b8dee2f2c352f0f40` | `96e6c9aeca68d196bea1f3b66d485e941abf13e2` |
| `instruction.md` | `100644` | 1,381 | `38054a3371f7c2c853478a2f9af0bff858437d5b28a9a07483238f06e566921f` | `8ed50b24d2b05ab1516f539891def9abed8c54a3` |
| `README.md` | `100644` | 2,404 | `5dfa88651890cda06fa2a25babd3b813818767635e88bbabecc8305d5bb3942f` | `dca818c6e9d00792adfd293e58c58f2c633cb0da` |
| `solution/solve.sh` | `100644` | 3,135 | `58cf3c844e907745cd81fb4bd75efb3d080e2f0e82b589c56c954ae6e1756510` | `8058f1efb55ce83e8be4f8e8a40a4baa5ac3c361` |
| `task.toml` | `100644` | 939 | `75c8913f0fa15890786c11999cde920b8bfc10d4ce8a14edf1e76985a2ac5800` | `9860901f6ec77f8fef9bc0c6056d007339dfab57` |
| `tests/test.sh` | `100644` | 608 | `4770437ea96c3cc84684b4f99d55fb148fcac09f9ea1e8ef49de487716e6c334` | `cb4a41a45e4b23f4d1f1392fd68f6e47c5dbbf57` |
| `tests/test_outputs.py` | `100644` | 5,878 | `90970bb3c372e8e7fb6bb9ac5f649ba73eb38e1a5ea2e922a9057a72b6b4192d` | `e81aaade16184e40e7e3543b404222a723464613` |

For reproducibility of this addendum's audit, hashing the UTF-8 bytewise-sorted
records
`path<TAB>mode<TAB>size<TAB>sha256<LF>` produces package-manifest SHA-256
`1fc946ef9efe7a223d6009a96bdaa411bb6e356cea09d64f0b959a69ae4944d9`.
This is an audit-manifest digest, not an upstream-published digest or the final
Phase canonicalization; the admitted manifest must additionally retain file
roles as section 3.1 requires. The oracle remains excluded from Agent-visible
mounts even though its bytes are covered by the complete source-package
identity.

The exact
[`test.sh`](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/tests/test.sh)
has four runtime-resolution effects:

1. `apt-get update` resolves mutable Debian indexes;
2. `apt-get install -y curl` resolves curl and its dependency/recommendation
   graph;
3. `curl https://astral.sh/uv/0.9.5/install.sh | sh` resolves the installer and
   the installer-selected platform archive; and
4. `uvx -p 3.13` may resolve a Python runtime, then resolves
   `pytest==8.4.1`, `pytest-json-ctrf==0.3.5`, and their transitive packages.

The Python test file itself only imports the standard library and invokes
`openssl` and `python`; it has no additional network or package acquisition.
See the exact
[`test_outputs.py`](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/tests/test_outputs.py).

#### Task image identity

The task declares
`alexgshaw/openssl-selfsigned-cert:20251031`. The Docker Hub registry resolved
that tag on 2026-08-26 to this single Linux/amd64 manifest:

| OCI object | Digest |
|---|---|
| Manifest | `sha256:4c948a4e630af2435ae0a19108fc0814a946ac2fa29a512469e0fc77b38c8c12` |
| Config | `sha256:7082de4dab44888b5b5e9c0f9de09b30bc8cd28b329edc6d153fb5ce3dca5f24` |
| Layer 1 | `sha256:b1badc6e50664185acfaa0ca255d8076061c2a9d881cecaaad281ae11af000ce` |
| Layer 2 | `sha256:7e88bf276da094b3c1c5d62775d85ae6e63c58165a5c759570bcecf8b52a4ce3` |
| Layer 3 | `sha256:d55dfff74d98ac204fa78f18533eecc39a8bd69bd9384a73572c6d09f06b11fc` |
| Layer 4 | `sha256:e97853488aa5b877aa1248a8d146c75752543b00dced77518e40164d1badafd3` |
| Layer 5 | `sha256:63ff33ad641b9f09475780c18fcae5aa35c92fed4207585d4e93c6070d6c2e0e` |
| Layer 6 | `sha256:0d61612c452df55617b5222f791e20a624942b2476f92af02c613054f80bb17e` |

The config identifies Linux/amd64, Python `3.13.7`, and a base-rootfs creation
record anchored to Debian snapshot `20250811T000000Z`. The unpacked image's
`debian.sources` retains that base snapshot in comments even though its active
URI is mutable `deb.debian.org`. The Opi candidate materialization instead uses
`20250822T180000Z`, immediately before the final task-image construction
recorded by the image history, so the resolved apt state matches the completed
task image rather than only its earlier base layer. The image and tag are visible
in the official [Docker Hub repository](https://hub.docker.com/r/alexgshaw/openssl-selfsigned-cert/tags);
admitted execution must pull by the manifest digest above and retain the
config/layer graph, not re-resolve the tag.

#### Debian apt closure

The candidate apt universe is the Opi materialization epoch
`20250822T180000Z`. All three indexes participate in candidate selection and
must be retained even though the 19 selected files below come from `bookworm`
main:

| Index | SHA-256 |
|---|---|
| [`bookworm/main amd64 Packages.xz`](https://snapshot.debian.org/archive/debian/20250822T180000Z/dists/bookworm/main/binary-amd64/Packages.xz) | `c9bb33f1dc3e679fead4b8c53cac7a1ef99a26b06f48f56ba84510b9c4bc6625` |
| [`bookworm-updates/main amd64 Packages.xz`](https://snapshot.debian.org/archive/debian/20250822T180000Z/dists/bookworm-updates/main/binary-amd64/Packages.xz) | `87e7e94047fb7fb6f4ceecc7022d4bee55b66031cc2a7666d3196f3e0aabb846` |
| [`bookworm-security/main amd64 Packages.xz`](https://snapshot.debian.org/archive/debian-security/20250822T180000Z/dists/bookworm-security/main/binary-amd64/Packages.xz) | `1b348f05bbc69c5daf572a9e7a7c59e904759066d148bfd8f256a2138373dcaf` |

Resolving `apt-get install -y curl` with recommendations enabled against the
unmodified image package database installs exactly these 19 new packages:

| Package | Exact version | Official snapshot file | SHA-256 |
|---|---|---|---|
| `curl` | `7.88.1-10+deb12u12` | [`curl_7.88.1-10+deb12u12_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/c/curl/curl_7.88.1-10+deb12u12_amd64.deb) | `fa9e0c7bb1e00607c060ce533844c3a465105d7db13685b75508514aa8a2c999` |
| `krb5-locales` | `1.20.1-2+deb12u3` | [`krb5-locales_1.20.1-2+deb12u3_all.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/krb5/krb5-locales_1.20.1-2+deb12u3_all.deb) | `59d13f7dda06638202011a5ec740dad591a663b3f68df1e2349662dc9efdacc5` |
| `libbrotli1` | `1.0.9-2+b6` | [`libbrotli1_1.0.9-2+b6_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/b/brotli/libbrotli1_1.0.9-2+b6_amd64.deb) | `563b4caec1aa5e876bd3355b36e7a38e1484baf5a293b48d1e8bd22db786e4d7` |
| `libcurl4` | `7.88.1-10+deb12u12` | [`libcurl4_7.88.1-10+deb12u12_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/c/curl/libcurl4_7.88.1-10+deb12u12_amd64.deb) | `deeae4b7dd7235c88376b2f5ad08c22b225c5112133ee0d9bff8412cb29dd104` |
| `libgssapi-krb5-2` | `1.20.1-2+deb12u3` | [`libgssapi-krb5-2_1.20.1-2+deb12u3_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/krb5/libgssapi-krb5-2_1.20.1-2+deb12u3_amd64.deb) | `34a3fcd0e5aeeb01e895b8ce563b06b12b0d763f1b5943b97feffed07a3f56f1` |
| `libk5crypto3` | `1.20.1-2+deb12u3` | [`libk5crypto3_1.20.1-2+deb12u3_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/krb5/libk5crypto3_1.20.1-2+deb12u3_amd64.deb) | `6aff44ea189ad062eb7a83b9fac99861f1efa35bf66bc7605c2ade644a08bb94` |
| `libkeyutils1` | `1.6.3-2` | [`libkeyutils1_1.6.3-2_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/keyutils/libkeyutils1_1.6.3-2_amd64.deb) | `cfac89e6a7a54ff3c6a4f843310e25efeddaa771baeae470bd98bd588c373563` |
| `libkrb5-3` | `1.20.1-2+deb12u3` | [`libkrb5-3_1.20.1-2+deb12u3_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/krb5/libkrb5-3_1.20.1-2+deb12u3_amd64.deb) | `e3c228ea202bf5a278329f5a96cbe3ca48927e0ed7fea849680b0996fece4879` |
| `libkrb5support0` | `1.20.1-2+deb12u3` | [`libkrb5support0_1.20.1-2+deb12u3_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/k/krb5/libkrb5support0_1.20.1-2+deb12u3_amd64.deb) | `8862ae9677f2361d888fcefb2734e4ddfb2678f82535d21c4c0cc77d8e406e9f` |
| `libldap-2.5-0` | `2.5.13+dfsg-5` | [`libldap-2.5-0_2.5.13+dfsg-5_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/o/openldap/libldap-2.5-0_2.5.13+dfsg-5_amd64.deb) | `4b6c30f6554149c594628d945edc6003f0eea8d0cc1341638c0e71375db147ed` |
| `libldap-common` | `2.5.13+dfsg-5` | [`libldap-common_2.5.13+dfsg-5_all.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/o/openldap/libldap-common_2.5.13+dfsg-5_all.deb) | `72a6c113801a0f307f3a9ab9fe7a7f9559d9164af990494ed2c50617a0e20452` |
| `libnghttp2-14` | `1.52.0-1+deb12u2` | [`libnghttp2-14_1.52.0-1+deb12u2_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/n/nghttp2/libnghttp2-14_1.52.0-1+deb12u2_amd64.deb) | `dbe3eb8c831e654ac7d5cf045d605d53fd763e8ebc238607c25f301996103bb9` |
| `libpsl5` | `0.21.2-1` | [`libpsl5_0.21.2-1_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/libp/libpsl/libpsl5_0.21.2-1_amd64.deb) | `4f0d35610204e4e754b057748719744114621f2f6f4202d846c314860a981afb` |
| `librtmp1` | `2.4+20151223.gitfa8646d.1-2+b2` | [`librtmp1_2.4+20151223.gitfa8646d.1-2+b2_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/r/rtmpdump/librtmp1_2.4+20151223.gitfa8646d.1-2+b2_amd64.deb) | `e1f69020dc2c466e421ec6a58406b643be8b5c382abf0f8989011c1d3df91c87` |
| `libsasl2-2` | `2.1.28+dfsg-10` | [`libsasl2-2_2.1.28+dfsg-10_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/c/cyrus-sasl2/libsasl2-2_2.1.28+dfsg-10_amd64.deb) | `11ee190ad39f8d7af441d2c8347388b9449434c73acc67b4b372445ac4152efa` |
| `libsasl2-modules` | `2.1.28+dfsg-10` | [`libsasl2-modules_2.1.28+dfsg-10_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/c/cyrus-sasl2/libsasl2-modules_2.1.28+dfsg-10_amd64.deb) | `b1966bea9832686a0fd5ddba9787dce5816ebe02218a4a8f7472a1628d73451b` |
| `libsasl2-modules-db` | `2.1.28+dfsg-10` | [`libsasl2-modules-db_2.1.28+dfsg-10_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/c/cyrus-sasl2/libsasl2-modules-db_2.1.28+dfsg-10_amd64.deb) | `3ac4fd6cbe3b3b06e68d24b931bf3eb9385b42f15604a37ed25310e948ca0ee6` |
| `libssh2-1` | `1.10.0-3+b1` | [`libssh2-1_1.10.0-3+b1_amd64.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/libs/libssh2/libssh2-1_1.10.0-3+b1_amd64.deb) | `d20a3ee34fa84ad8bd381e8be6e9c2c2ea32347cff5e1169c10e978d43f54f24` |
| `publicsuffix` | `20230209.2326-1` | [`publicsuffix_20230209.2326-1_all.deb`](https://snapshot.debian.org/archive/debian/20250822T180000Z/pool/main/p/publicsuffix/publicsuffix_20230209.2326-1_all.deb) | `791c92c681a3cefcc9721445dc8a301a1a3cb3eef40ac2c16a4d9dd9ad5a42d7` |

The closure size is 2,489 KiB of archives and 6,813 KiB installed. `krb5-locales`,
`libldap-common`, `libsasl2-modules`, and `publicsuffix` appear because the
native `apt-get install -y` keeps apt recommendations enabled. Removing them to
minimize the closure would no longer reproduce the upstream command.

#### uv installer and executable closure

The hardcoded versioned endpoint returned exactly the official uv 0.9.5
installer asset on 2026-08-26:

| Artifact | First-party source | SHA-256 |
|---|---|---|
| Installer shell, 65,983 bytes | [`uv-installer.sh`](https://github.com/astral-sh/uv/releases/download/0.9.5/uv-installer.sh) | `8402ab80d2ef54d7044a71ea4e4e1e8db3b20c87c7bffbc30bff59f1e80ebbd5` |
| Linux x86_64 GNU archive | [`uv-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/astral-sh/uv/releases/download/0.9.5/uv-x86_64-unknown-linux-gnu.tar.gz) | `2cf10babba653310606f8b49876cfb679928669e7ddaa1fb41fb00ce73e64f66` |
| Extracted `uv` | output of the archive above | `d3dc3ca8e29337dd602a9a4df9e6edb15ebc52aed91461debeedb96939dc4ce0` |
| Extracted `uvx` | output of the archive above | `421b1bb58429180a04ade98da066b5a5b6763e8ed74edef97359701c4cb42db8` |

The archive digest is also published by uv's official
[`0.9.5` checksum asset](https://github.com/astral-sh/uv/releases/download/0.9.5/uv-x86_64-unknown-linux-gnu.tar.gz.sha256),
and the release maps to source commit
[`d5f39331a73d5042e70ab770463dff632e20c127`](https://github.com/astral-sh/uv/commit/d5f39331a73d5042e70ab770463dff632e20c127).

The task image already supplies CPython `3.13.7`. Setting
`UV_PYTHON_DOWNLOADS=never` makes `-p 3.13` fail closed instead of resolving a
managed Python. The feasibility adapter installs a reviewed `curl` shim that
returns the pinned installer only for the exact upstream argument vector
`-LsSf https://astral.sh/uv/0.9.5/install.sh`, rejects any other use of that URL,
and delegates all unrelated calls to `/usr/bin/curl`. Its SHA-256 is
`37a68181e83ae7c30286d3048148c55bb2a842af89ec97fd04d78c11a6e2aae0`.
The unchanged installer then consumes the pinned archive through
`UV_DOWNLOAD_URL=file:///opt/uv`. This shim is Opi adapter policy, not an
upstream artifact; its bytes therefore need the same review and lock treatment
as the closure inputs.

#### Python wheel closure

On Linux CPython 3.13, the two exact top-level requirements resolve to six pure
Python wheels. Markers for Windows `colorama` and Python before 3.11
`exceptiongroup`/`tomli` do not apply. Upstream historically locks only
`pytest==8.4.1` and `pytest-json-ctrf==0.3.5`; it does **not** lock the four
transitive distributions. On the 2026-08-26 research epoch, Opi materialized
the current compatible pure-Python candidates from official PyPI metadata and
then froze these exact file URLs and hashes. Thus `iniconfig`, `packaging`, and
`pygments` below are Opi candidate policy locks, not claims about upstream's
historical environment. A different compatible transitive closure is a
different experiment identity.

| Distribution | Exact wheel | SHA-256 |
|---|---|---|
| `pytest==8.4.1` | [`pytest-8.4.1-py3-none-any.whl`](https://files.pythonhosted.org/packages/29/16/c8a903f4c4dffe7a12843191437d7cd8e32751d5de349d45d3fe69544e87/pytest-8.4.1-py3-none-any.whl) | `539c70ba6fcead8e78eebbf1115e8b589e7565830d7d006a8723f19ac8a0afb7` |
| `pytest-json-ctrf==0.3.5` | [`pytest_json_ctrf-0.3.5-py3-none-any.whl`](https://files.pythonhosted.org/packages/51/a8/d58637d10f70ae14a1ad18cb37f4aa32d5593510213d5f929892cb1c9c6d/pytest_json_ctrf-0.3.5-py3-none-any.whl) | `e82fd1d69be2f92385bc33540063e5ad7b17b36de67764c84f3ceb9815a895e9` |
| `iniconfig==2.3.0` | [`iniconfig-2.3.0-py3-none-any.whl`](https://files.pythonhosted.org/packages/cb/b1/3846dd7f199d53cb17f49cba7e651e9ce294d8497c8c150530ed11865bb8/iniconfig-2.3.0-py3-none-any.whl) | `f631c04d2c48c52b84d0d0549c99ff3859c98df65b3101406327ecc7d53fbf12` |
| `packaging==26.3` | [`packaging-26.3-py3-none-any.whl`](https://files.pythonhosted.org/packages/63/34/ba1c580383c9eada3711951fef0795c80b829a078d72188184bcab9dd527/packaging-26.3-py3-none-any.whl) | `d7193f7c8e4e93f444fde0262bf90af30e16fa0ad0ad44cb553c87339b23cd1c` |
| `pluggy==1.6.0` | [`pluggy-1.6.0-py3-none-any.whl`](https://files.pythonhosted.org/packages/54/20/4d324d65cc6d9205fabedc306948156824eb9f0ee1633355a8f7ec5c66bf/pluggy-1.6.0-py3-none-any.whl) | `e920276dd6813095e9377c0bc5566d94c932c33b27a3e3945d8389c374dd4746` |
| `pygments==2.21.0` | [`pygments-2.21.0-py3-none-any.whl`](https://files.pythonhosted.org/packages/71/46/17f022dd3e953bf20a04a028a21ec746d942f8d2af30fa0f124fa0e6a684/pygments-2.21.0-py3-none-any.whl) | `2363c69b61c4a97c838da3b130dcd6468f4848992b21a82f2a63ec34377137d9` |

The runtime environment must set all of:

```text
UV_DOWNLOAD_URL=file:///opt/uv
UV_FIND_LINKS=file:///opt/wheels
UV_OFFLINE=1
UV_PYTHON_DOWNLOADS=never
```

`UV_OFFLINE` is uv's official no-network setting; in a fresh rootfs with no uv
cache, `UV_FIND_LINKS` makes the six reviewed wheels the available resolver
universe. `UV_NO_INDEX` is also an official uv environment variable, but this
proof did not set it and does not claim otherwise. `UV_DOWNLOAD_URL` belongs to
the official uv installer and redirects its release archive lookup to the
sealed local directory. See uv's official
[`environment` reference](https://docs.astral.sh/uv/reference/environment/)
and the pinned installer asset above.

#### Harbor environment and network contract

Harbor `v0.22.0` accepts job-level verifier environment variables. The exact
path is `VerifierConfig.env` -> `Trial._run_shared_verifier` as `override_env`
-> `Verifier.verify`'s merged verifier environment -> the Docker environment
adapter's `docker compose exec -e KEY=value` arguments.
See the immutable
[`VerifierConfig`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/models/trial/config.py#L319-L347),
[`Verifier.verify`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/verifier/verifier.py),
and
[`Trial._run_shared_verifier`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/trial/trial.py#L641-L672).
This is sufficient to inject the `APT_CONFIG` and `UV_*` values without editing
`tests/test.sh` or `task.toml`.

Network policy is different. The official task's legacy
`allow_internet = true` resolves to a public environment baseline, and it has no
explicit verifier-phase override. Harbor's exact
[`network_policy.py`](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/trial/network_policy.py)
therefore keeps the shared verifier public. The Phase execution profile must
apply and record an exact no-network Docker/Harbor overlay, or a reviewed
effective-config projection, and then prove the actual container policy with
runtime inspection. Silently claiming that the unchanged task already asks
Harbor for no-network verification would be false.

#### Linux x86_64 no-network feasibility result

The official OCI layers were unpacked from zero on WSL2 Linux x86_64. The exact
upstream oracle and verifier files were hash-checked; all 19 Debian packages,
both uv release assets, and all six wheels were materialized from the locks
above; mutable apt sources were disabled; and the verifier ran under
`unshare -n`.

Before verifier execution, the runner proved that the namespace had no default
route and that a direct `/usr/bin/curl https://example.com/` probe failed. The
exact official `solution/solve.sh` then produced reward `1`; all six tests
passed; and the CTRF SHA-256 was
`c82e566e8a424f8dfda437ed1fcb4dfa4758a490c34e5a91db250e539d8ea41b`.
The reviewed offline runner SHA-256 was
`006becdd01cbde807e5dcd6050e07c9b2ac46f65ecbf8a3805c6e1f8e52ebfaf`.

The resulting research proof lock JSON has SHA-256
`6512737e7eb5c4cc9f19855f1eed233684e775db9ac34dea42def02ccdad06fc`.
That digest identifies this feasibility observation only: it is not the final
closure artifact, verifier OCI image, Harbor trial lock, or CI admission
artifact. Those identities remain outputs of the Phase Linux materializer and
native-smoke job.

### A.4 Alternatives

| Approach | Result |
|---|---|
| Leave the verifier online and merely record installed versions afterward | Rejected: future apt/PyPI state remains an input and a one-run inventory is not reproducibility. |
| Edit `tests/test.sh` to remove installers | Rejected: this changes native verifier bytes and weakens upstream authority. |
| Preseed exact installer outputs, use a local apt repo/wheelhouse, and run the unchanged script with no network | Feasible; proved in the Linux chroot experiment. |
| Build a derivative task image containing the closure | Feasible but changes the task image identity and exposes an extra build product; prefer a sealed read-only closure mount unless the real Harbor smoke proves mounting insufficient. |
| Wait for an upstream verifier image | Strongest provenance if published, but none exists in the selected package/revision. |

### A.5 Rust feasibility

No Debian or Python package manager needs to be reimplemented in Rust. The
Companion can validate a canonical closure manifest, hash each file, validate
the allowed environment projection, supervise the exact Harbor process, and
reject an observed image/executable/result not named by the resolved lock.

The Linux materializer remains an external, bounded build step. Its own
executable/tool identity must be captured because generated `Packages` indexes,
archive layout, and OCI output otherwise add an unreviewed supply-chain input.

### A.6 Existing extension fit

The closure fits the planned Independent Companion:

- `ExternalArtifactLock` can cover the task tree, task image graph, snapshot
  indexes, `.deb` files, uv release files, wheels, and closure manifest;
- the process adapter can pass Harbor's verifier environment projection;
- the native benchmark adapter can preserve `test.sh`, CTRF, reward, stdout,
  stderr, and failure ownership; and
- one resolved execution profile can distinguish official package identity
  from the additional no-network closure policy.

No Opi Agent Core or Reference Product change is demonstrated.

### A.7 Smallest missing core seam

None. The remaining work is external lock materialization and process evidence
inside `opi-eval`. A public package-manager abstraction, task-image trait, or
Agent Core API would not help close the missing run-produced facts.

### A.8 Placement and planning implication

The next `opi-implement plan` may treat the prior Terminal-Bench 2.1
**external-fact research blocker as closed**, provided it creates an early
Linux lock-materialization task with these outputs:

1. canonical closure artifact manifest and SHA-256;
2. actual pulled task image manifest/config/layer identities;
3. actual Harbor executable/environment lock and effective task configuration;
4. actual Docker network inspection proving verifier no-network enforcement;
5. exact upstream tests and oracle hashes checked before execution; and
6. one oracle pass plus authoritative Opi/pi verifier outputs in the sole Phase
   native-smoke artifact.

Planning must not mark `P18-SEC-003` complete merely because this report lists
candidate inputs. Completion belongs to the implementation/assurance artifact.

### A.9 Minimal remaining facts before `P18-SEC-003` can close

No unresolved online package version or checksum remains for this selected
task. The minimal missing facts are all produced by the admitted Linux run:

1. the final materializer/tool identity and canonical closure artifact digest;
2. the actual Harbor trial lock plus resolved local task-image identity;
3. the actual container network mode/egress-control evidence during verifier
   execution; and
4. the Docker/Harbor oracle result proving the exact native package consumes
   only the sealed closure.

Until those four facts exist, the honest status is:

```text
research_status = static_closure_resolved
planning_gate = may_resume
P18-SEC-003 = open_pending_linux_materialization_and_native_smoke
```

### A.10 Limitations

- The no-network run used a Linux x86_64 chroot and network namespace over the
  official image rootfs, not a live Docker daemon or Harbor trial.
- WSL2 host, chroot mount, and generated apt-index identities are experimental
  and are not Phase artifacts.
- The task's public network baseline means Harbor/Docker no-network enforcement
  still needs an explicit reviewed execution-profile mechanism and runtime
  proof.
- Only `openssl-selfsigned-cert` is closed. The other 88 Terminal-Bench 2.1
  verifiers retain their own independent dependency closures.
- Registry values were observed on 2026-08-26; digest-only use prevents later
  tag movement from changing the admitted input.
- No specification, design, implementation ledger, runtime code, workflow, or
  CI file was modified by this addendum.
