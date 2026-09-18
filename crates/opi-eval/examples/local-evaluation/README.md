# Local evaluation walkthrough (P19-D05)

Independent-directory preparation of native materials from a seeded
local cache. Nothing here calls a model or reads real credentials; the
provider connection is declared in the experiment document and resolved
only for the selected Agent processes at run time.

## 1. Seed the local cache once

From a repository checkout, copy the pin set into the cache directory
your experiment document declares under `[storage].cache`:

```sh
cache=/path/to/cache
mkdir -p "$cache/pins/static" "$cache/pins/scripts" \
         "$cache/pins/profiles/benchmarks" "$cache/pins/integrations"
cp crates/opi-eval/external-locks/static/linux-x86_64.json \
   "$cache/pins/static/linux-x86_64.json"
cp crates/opi-eval/scripts/materialize-external-locks.sh \
   crates/opi-eval/scripts/build-agent-artifacts.sh \
   crates/opi-eval/scripts/verify-external-lock-ci.py \
   crates/opi-eval/scripts/scripted-provider.py "$cache/pins/scripts/"
cp crates/opi-eval/profiles/benchmarks/*.toml "$cache/pins/profiles/benchmarks/"
cp -r crates/opi-eval/integrations/harbor "$cache/pins/integrations/"
cp -r crates/opi-eval/integrations/pier  "$cache/pins/integrations/"
```

The public prepare entry acquires the selected Agent packages, complete pinned
corpus and selected Harbor/Pier runner directly from this pin-only cache.
For a cold Opi build, pass the local repository and exact committed revision:
prepare exports only that commit and verifies its Cargo.lock against the static
workspace input before building. Uncommitted source bytes never enter the build.
A missing local commit rejects without fetching. Pi-only selections require no
Opi source, and verified warm packages require no source arguments.

Pi preparation verifies its source blobs, locked dependencies, model-data archive
against the committed shrinkwrap integrity and pinned Node distribution. Its
complete runtime package includes Node, bundle, metadata, docs and assets.
Harbor/Pier are installed with pinned uv and locked dependencies at their final
cache paths, preserving interpreter and console-script paths. No manual producer
stage is required. An unverifiable source, archive or build input rejects before
material publication; existing verified cache entries remain reusable.

`prepare` checks the runner's Git root, exact commit, clean source, lock
blob and SHA-256, then freezes its source tree, project file, lock,
interpreter and entrypoint identities in the inventory. Local material also
requires a `runner_project` with an absolute path and a source manifest
digest over tracked Git entries and their working-file bytes. Material
loading rechecks the source against the static commit and lock pins,
including after preparation and when loading a frozen plan. The emitted
`verifier_env` binds `UV_PROJECT`, `UV_PROJECT_ENVIRONMENT` and `UV_PYTHON`
to the absolute cached project, virtual environment and its prepared Python
interpreter, while preserving the trial's working directory for native
job evidence. It uses a cache-local home and uv cache, offline dependency
resolution, disabled Python downloads and dotenv loading, and an explicit
system `PATH`; development dependencies are excluded with `UV_NO_DEV=1`,
and no ambient provider credentials enter this environment.
Populate all required dependencies before freezing: an offline run cannot
fetch missing dependencies. No host wrapper is needed to select the runner.
The source manifest excludes `.git`, `.venv` and ignored generated files.
Interpreter and console-script digests are checked during preparation;
material loading does not yet verify the installed virtual environment's
complete dependency bytes. The source and lock binding is not a complete
runtime dependency closure.

Oracle diagnostics use the same pinned uv executable and runner environment;
the explicit witness dispatch selects the upstream oracle Agent. No separately
invented `wrappers/*-oracle.sh` is needed for local preparation.

## 2. Diagnose

```sh
opi-eval doctor --config experiment.toml
```

Reports engine reachability, architecture, WSL/Docker-observed
CPU/memory/GPU/disk, and missing materials. Unknown observations carry
reasons; diagnosis starts no model.

## 3. Prepare

```sh
opi-eval prepare --config experiment.toml --out /path/to/material \
  --opi-source /path/to/local/opi --opi-commit <full-40-hex-commit>
```

Verifies the seeded cache against the pinned static lock (producer
scripts over LF-normalized bytes, packages and integration modules over
sorted-manifest digests), freezes the selected benchmark, subject
executables, and every requested image identity, checks the frozen
task's resource requirements against the local observations (an unmet
or unobservable requirement rejects the whole request - the set never
silently shrinks), and emits `material/material.json` with truthful
local origin. The manifest carries no GitHub run/artifact metadata; the
upload receipt stays a CI transport claim. Missing material triggers
supervised acquisition through the pinned materializer when it is
seeded (`prepare --cache ... --plan ...`, log captured under the output
root); without it, the failure names the missing identity and how to
acquire it.

If registry access needs a host proxy, configure it before freezing the plan.
Native runner processes receive only the material's explicit environment;
exporting `HTTPS_PROXY` in the launching shell does not reach Pier's Docker
client. Add the required credential-free proxy endpoint to the selected
benchmark's `verifier_env` and `oracle_env` in a fresh, unfrozen material copy,
then re-run native validation and plan generation with that copy. Keep proxy
passwords and provider credentials out of these persisted maps. Preserve the
pinned runner variables and the Agent's separate inference-egress policy.
Check registry access and the runner's container startup with that exact
environment and no model invocation; a successful build in the interactive
shell or with warm registry caches does not establish native-run readiness.
Already frozen material and failed trials remain unchanged.

## 4. Preview, freeze, and run a selection plan

```sh
opi-eval plan --config experiment.toml --out /path/to/plan.json
cat /path/to/plan.json
opi-eval run --plan /path/to/plan.json --root /path/to/run-root
```

The document's `[selection]` (scope, explicit task ids, reason,
repetitions, optional seeds) expands against the enumerated corpus at
freeze time; unmet or unobservable selected requirements reject before
any plan is written. The plan is digest-bound over the experiment
source, material identity, corpus entries, and exact trials;
`run --plan` re-verifies all of it and executes exactly those trials
(stage/settlement progress on stderr, structured result saved under the
run root). A pi-only experiment needs no artificial comparison edge.
For native material, selection accepts the profile's canonical task ID or the
exact upstream task name from its admitted package. Plans and generated trial
IDs use the canonical ID; unbound aliases and duplicate selections reject.

## 5. Verify seals and read the offline report

```sh
opi-eval verify --root /path/to/run-root
opi-eval report --root /path/to/run-root --out /path/to/report.json
```

`verify` re-checks the original seals of every sealed bundle (exit 1
names the typed drift; bytes are never repaired). `report`
reconstructs the normalized report offline from verified sealed
evidence: per-benchmark metric denominators (native zero and
benchmark-defined timeouts stay graded), coverage counts
(planned/attempted/valid/agent-failure/invalid/unexecuted), scope
labels, comparison pairs with missing sides, repetition control pairs,
and typed unknown telemetry.

`regrade --root R --native-material M --out D` is the distinct actual
native regrading: it re-grades the preserved Agent artifacts with the
pinned native verifier in fresh offline environments, persisting one
derived grading record per trial without rewriting any original score
and without any Agent, credential, or provider involvement.

## 6. Validate and run from the independent directory

```sh
opi-eval validate --config experiment.toml --native-material /path/to/material/material.json
opi-eval run --config experiment.toml --root /fresh/root \
  --native-material /path/to/material/material.json ...
```

Neither command needs the repository checkout: every identity the
runtime consumes lives inside the cache or the output tree.

## 7. Hand the operator evidence to the offline verifier

After the real operator run, redact and stage the evidence root (manifest
with the six attributable trials, preserved artifacts, the
`opi-artifact-audit` scoped view, the cached upstream identities) and
verify it offline - the verifier launches no Agent, reads no credentials,
and contacts no provider:

```sh
python3 scripts/verify-local-acceptance.py --root /redacted/evidence-root   --out /tmp/verification-report.json --companion ./target/release/opi-eval
```

See the crate README section "Local operator acceptance and its offline
verifier" for the full contract.
