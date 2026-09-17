# DeepSWE registry proxy repair

The Docker Hub connection blocker is repaired in a separate native-material
candidate. This is preparation evidence, with zero new benchmark trials,
Agent launches, grader launches, provider calls, or real credential reads.
The earlier six-slot evaluation remains 4/6 valid; its two DeepSWE failures
are unchanged. The user's “继续修复” authorizes this infrastructure repair.

## Cause and change

The frozen runner environment omitted the host's proxy. Companion intentionally
clears ambient variables; Pier passes the resulting explicit environment to
Docker. Exporting a proxy only in the outer launcher cannot bridge that boundary.
With the original environment, the unchanged generated proxy Dockerfile failed
after 30.977 seconds while fetching an anonymous Docker Hub token. Adding only
`HTTPS_PROXY=http://127.0.0.1:19828` made the build pass in 2.563 seconds and a
repeat pass in 0.973 seconds. The endpoint has no embedded credentials.

The candidate adds that one key to DeepSWE's existing `verifier_env` and
`oracle_env` maps. All other material values are identical. No Rust code,
upstream runner, task package, image recipe, global proxy/DNS setting, or
existing plan was changed. The local walkthrough now explains explicit host
proxy preparation and checks under the actual runner environment. That file
has no Chinese counterpart; this repair report has matching English/Chinese
versions.

## Verification

All command records and outputs are retained under `commands/` in the repair
root. Paths below abbreviate:

```text
N=/home/luiz/opi-eval-runs/20260910t170439z-deepswe-network-repair
P=/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
```

| Actual check | Result |
|---|---|
| Public anonymous-token `curl`, response body discarded, original isolated environment versus explicit existing proxy | Direct: exit 28 / HTTP 000; proxy: exit 0 / HTTP 200. No model endpoint called. |
| `python3 N/reproduce-build.py`, running `docker build --progress=plain --tag opi-eval-pier-egress-proxy:20260910t170439z N/proxy-context` under the recorded environments | Original: exit 1; explicit proxy: exit 0 twice. |
| `python3 N/run-bootstrap-checks.py`, invoking pinned `uv run --locked --no-sync python N/bootstrap-check.py direct\|proxy` | Both exit 0 after cache warming. The repaired environment's native Pier `DockerEnvironment.start(force_build=False)` completed Compose startup and health checks in 5.114 seconds. |
| Bootstrap default-task and Agent-process environment assertions | Passed: the host proxy is absent from default task execution; Agent execution still receives Pier's authenticated restricted proxy. These are shell assertions, not Agent or grader runs. |
| Recorded native Compose files with `docker compose ... build --pull pier-egress-proxy` | Both environments exit 0 after warming. This check does not establish direct-path reliability. |
| `docker buildx imagetools inspect --raw docker.io/library/ubuntu:24.04`, separate clients under original versus repaired environment | Direct: exit 1 after 30.110 seconds; explicit proxy: exit 0 after 1.960 seconds. Returned manifest SHA-256 matches `224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254`. |
| `P/builds/opi-eval validate --config P/inputs/deepswe-v1.1.toml --native-material N/material/material.json` | Exit 0; derived native integrity `0474341d58723fc8f56bf42a3fb999370dc465104610ce55490e6d21891ec82c`. No new plan or execution. |
| `python3 scripts/opi-doc-check.py`; `git diff --check` | See retained final check records. |

The warm-cache successes are retained alongside the negative results. They do
not weaken the observed proxy requirement for fresh Docker client requests.
The checks use the public registry network; “no model calls” does not mean
all checks were disconnected from the internet. A future run still depends
on the local proxy being available.

## Identities and retained evidence

- Original material SHA-256: `a2cf5cd83a577b76f1d070785e74cd9b47170ad1a0dbf8f7128d02faf42f169f`.
- Repaired material SHA-256: `b725588bff93c28458e09fae2feaa23920758627961a187ff6aa7659c6f9f18e`.
- Companion executable SHA-256: `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`, unchanged, built from `55fbc95009e6f47145196dbc539454ebcb1cf692`.
- Retained reusable proxy image: `sha256:cab836d1ae2453f3b0dc60a25423f884238c92abc8eb49759bfe23c95b4cdce4`.
- Authoritative repair root: `N` above. Private Windows-accessible copy: [evidence inventory](E:/opi/evaluations/20260910t170439z-deepswe-network-repair/inventory.json).
- Prepared candidate: [material.json](E:/opi/evaluations/20260910t170439z-deepswe-network-repair/material/material.json). Its pinned Linux paths remain authoritative; the copy is not a relocated runnable cache.

Hash checks found zero changes across four prior staged inventories
(694, 276, 586, and 124 files) and two protected Linux inventories
(549 and 39 files). No acceptance manifest was created or upgraded.

Cleanup completed: both bootstrap projects removed their containers, volumes
and networks; three proven duplicate proxy images were removed after matching
their filesystem/startup to the retained reusable image. All 11 baseline
image IDs and shared caches remain. Final inspection found zero containers,
zero volumes, and no repair-owned networks. Reclaimed disk bytes are unknown
because layers are shared. Private generated proxy configurations remain as
evidence for stopped synthetic bootstrap instances.

Worktree-only scope: committed=none-for-this-task; staged=none; one tracked
walkthrough edit and this new report pair. Eight carried-in evaluation reports
remain untouched. `HEAD` is unchanged. Repository test impact: **none**;
no runtime/test source changed, and no Rust test or workspace gate is claimed.

Further paid evaluation needs a fresh plan using the repaired material and
authorization for its exact scope. This repair does not establish DeepSWE
task success, a new six-slot acceptance result, independent audit admission,
or Phase completion.
