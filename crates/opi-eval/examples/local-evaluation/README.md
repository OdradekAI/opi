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
   crates/opi-eval/scripts/verify-external-lock-ci.py \
   crates/opi-eval/scripts/scripted-provider.py "$cache/pins/scripts/"
cp crates/opi-eval/profiles/benchmarks/*.toml "$cache/pins/profiles/benchmarks/"
cp -r crates/opi-eval/integrations/harbor "$cache/pins/integrations/"
cp -r crates/opi-eval/integrations/pier  "$cache/pins/integrations/"
```

Build the selected Agent executables into the cache (pi-only selections
need no Opi):

```sh
bash crates/opi-eval/scripts/build-agent-artifacts.sh ...   # agents/<product>
```

## 2. Diagnose

```sh
opi-eval doctor --config experiment.toml
```

Reports engine reachability, architecture, WSL/Docker-observed
CPU/memory/GPU/disk, and missing materials. Unknown observations carry
reasons; diagnosis starts no model.

## 3. Prepare

```sh
opi-eval prepare --config experiment.toml --out /path/to/material
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
and typed unknown telemetry. `regrade` answers with an explicit
unavailable-operation diagnostic until native regrading lands.

## 6. Validate and run from the independent directory

```sh
opi-eval validate --config experiment.toml --native-material /path/to/material/material.json
opi-eval run --config experiment.toml --root /fresh/root \
  --native-material /path/to/material/material.json ...
```

Neither command needs the repository checkout: every identity the
runtime consumes lives inside the cache or the output tree.
