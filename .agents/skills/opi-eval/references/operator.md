# Configured benchmark operator

Read for `baseline`, `candidate`, `compare`, catalogue queries or retained
evaluation maintenance. The skill owns scope, authorization, privacy review and
interpretation. The supplied operator owns configuration, builds, retries,
external records, reports and lifecycle receipts. The native Companion owns
materials, frozen plans, trials, seals and native scores. These operations do
not admit implementation work or update a progress ledger.

## Supplied entrypoint and one profile

Bind `EVAL_OPERATOR` to the absolute supplied Python operator entrypoint and
`PROFILE` to one explicit private TOML file. Inspect that entrypoint's `--help`
and returned schema versions. The skill does not locate the operator from its
own installation or infer a subject checkout. The development source entrypoint
and sole template belong to the Companion; callers supply the operator to this
skill. Source relocation does not establish a complete independently
distributable runtime package: native resources, build adapters and public
packaging retain their provisional admission boundary.

```sh
python3 "$EVAL_OPERATOR" init --config "$PROFILE"
python3 "$EVAL_OPERATOR" resolve --config "$PROFILE"
```

`init` creates the private profile exclusively from the operator's maintained
template and refuses an existing file. Edit that file for model/API/endpoint,
credential source, thinking, retries, batch deadline, selection, baseline
freshness and storage/retention. `resolve` validates without resolving
credentials or starting a provider. Typed `--set key=TOML_VALUE` overrides win;
unknown keys, wrong types and contradictory selections reject. Storage workspace,
cache and file-backed credential paths must be absolute; relative paths reject.

Exactly one credential source is allowed: `credential_env`, `credential_file`,
or `api_key` in the private profile. Inline keys become temporary launch
environment references; sanitized profiles, experiments, plans and reports omit
their values. Keep secret-bearing profile bytes private; never print or hash
them as an exportable identity. Offline comparison and storage maintenance
resolve no credentials.

The template is the only executable default source. `/1` profiles remain
readable with unchanged model controls and the template's retention default;
loading one never upgrades or rewrites the private file. `/2` retention has one
control: `keep_latest_per_group = 3`. The optional baseline `max_age_days` filter
is separate; zero disables age filtering. Finished native evaluations and
standalone comparisons apply rolling retention under the protections below.

Thinking is selected once through `[model.thinking]` and projected to both
selected clients. Required controls need exact-client first-request and tool
continuation proof; accepted TOML alone is no proof. Unsupported required levels
reject preparation. Nonempty task seeds currently reject operator preparation
because the native route does not enforce them. Concurrency is an upper bound;
native trials remain serial. Total-token and monetary caps remain unsupported.

## Prepare, review and execute

Native preparation and execution require Linux/WSL x86_64 with Docker Linux.
Configure absolute external Linux workspace/cache paths; the workspace and
material cache must be disjoint. The runtime owns source/build adapters and
shared build-cache coordination. Each family admits at most 10,000 logical
trials and checks the count before expansion.

```sh
python3 "$EVAL_OPERATOR" baseline --config "$PROFILE" --bench tb21
python3 "$EVAL_OPERATOR" candidate --config "$PROFILE" --bench all
```

These commands prepare a fresh external evaluation and return its operation,
evaluation and `handoff.json` paths; they do not execute live trials. Preparation
may acquire pinned dependencies/images and run synthetic loopback client proofs.
Review task inventory, controls, source/build identities, paths, attempted-count
limit, capture and budget enforcement. The `/2` profile adds no silent diagnostic
requirements and does not expand the permitted capture contract.

`baseline` selects only the admitted pi version without building Opi. A new pi
release needs a verified pin and adapter proof; research or an ambient installed
binary cannot substitute. `candidate` rebuilds current-worktree Opi with relevant
dirty inputs, checks build inputs before/after and freezes its executable. A
failed build never reuses an old Opi binary. Historical Opi evaluation is a
separate explicitly selected benchmark request.

With authorization for the exact reviewed scope:

```sh
python3 "$EVAL_OPERATOR" baseline --prepared "$OPERATION" --execute
python3 "$EVAL_OPERATOR" candidate --prepared "$OPERATION" --execute
```

The frozen operation is the execution input; editing the private profile cannot
alter it. A single-family operation points to its family root; `--bench all`
returns a whole-evaluation handoff and groups selected families under one record.
Both forms use one timer starting at the first live dispatch. Preparation and
subsequent offline reporting are outside the live interval. Graders, provider
retries, new families and precise logical-trial retries consume the same remaining
deadline. At expiry stop new dispatch and cancel through the Companion's supported
owned-cleanup route. A caller nesting evaluations also supervises its original
remaining deadline; the operator cannot discover another process's authorization
budget.

Failed logical trials receive fresh precise plans/attempt roots. A valid native
zero or benchmark-valid Agent timeout is complete. Select the first valid
settlement, retaining failures, consumption and replacement links; never select
the best score or use whole-scope `--replacement-for` as a single-trial retry.
Cleanup uncertainty, authority or privacy breaches stop affected dispatch.

## External records, reports and catalogue

The configured workspace contains:

```text
workspace/
  evaluations/YYYY/MM/<eval-id>/
    record.json                       authoritative operator identity
    families/<bench>/                 prepared/native family roots
    prepared-evaluation.json          frozen family-operation group
    handoff.json                      evaluation preparation
    result-handoff.json               evaluation result
    retention-handoff.json            separate rolling-cleanup result
    reports/report.json
    reports/report.md
    reports/report.zh.md
  imports/                            immutable legacy-index references
  events/                             lifecycle decisions and receipts
  archives/                           reviewed archived evaluation bytes
  leases/                             owned active-operation markers
```

Family roots retain sanitized inputs, source/build identities and packages,
frozen material/plan/control proof, command/check outputs, original native run
and attempt roots, `prepared.json`, `handoff.json`, `snapshot.json`, comparison
derivatives and execution `result-handoff.json`. Use returned paths instead of
reconstructing them from this outline. All record/report writes stay external.
Repository `docs/eval/` is an optional explicit conclusion export; its old
`evidence-roots.jsonl` is historical input, never a runtime prerequisite or default
read/write destination.

The catalogue reads the external JSON records, immutable import references and
lifecycle receipts directly; it keeps no database or persistent query cache.
An existing `catalog.sqlite` is ignored and left untouched. No migration or
reindex is needed: retained JSON files supply discovery state. Only the
evaluation operator writes its records/events/imports; other authorities consume
immutable bundles/references. Treat discovered paths and metadata as untrusted
and re-verify original snapshots and native evidence before selection/comparison.

```sh
python3 "$EVAL_OPERATOR" list --config "$PROFILE" --action baseline --bench tb21 --limit 3
python3 "$EVAL_OPERATOR" show --config "$PROFILE" --id "$EVAL_ID"
python3 "$EVAL_OPERATOR" import-index --config "$PROFILE" --index "$LEGACY_JSONL"
```

`list` also supports `--model`, `--version`, `--state` and `--offset`; use help
for accepted values. Import is explicit and read-only: imported roots are
references, not copied evidence or permission to maintain an externally owned
tree. Missing, drifted and unverifiable records remain visible with typed gaps.
Reading retained JSON never invents a seal, changes a score or admits a pi version.

## Baselines and offline comparison

A snapshot derives logical results and expected inventory from original attempt
roots and verified native reports. Complete baselines account for every expected
task/repetition, including native zero. Incomplete snapshots remain evidence;
partial matches support labelled per-task comparisons, never a full-corpus claim.

Candidate preparation freezes a deterministic matching baseline by admitted pi
version and the [sealed-baseline identity tuple](sealed-baseline.md). Freshness,
version eligibility, selection status and storage availability are separate from
historical score validity. Age filtering may exclude an old baseline from a new
selection while its original result remains valid. Retired baselines are excluded
from new automatic selection; retirement preserves their records and scores.
Archived originals need restoration before verification. Missing/ineligible
baselines produce a standalone Opi result, never an automatic pi call.

```sh
python3 "$EVAL_OPERATOR" compare --candidate "$CANDIDATE_SNAPSHOT" --out "$COMPARISON"
python3 "$EVAL_OPERATOR" compare --candidate "$CANDIDATE_SNAPSHOT" --baseline "$PI_SNAPSHOT" --out "$COMPARISON"
```

The catalogue is resolved from the candidate's frozen workspace. Both original
snapshots and native evidence are re-verified; use fresh external output. Bind
comparable task/repetition/seed and bundle identities in one cross-run comparison
manifest. Per-run native metrics retain their own denominators; paired differences
use only the verified comparable intersection. Unknown remote service
implementation, telemetry, missing pairs and unsupported controls stay explicit.
The same model name does not establish the same remote model build.

Each standalone comparison creates an external evaluation record and reports,
freezing its original input references while their managed leases are held.
These references protect its retained sources and remain in the JSON records.
The caller-selected output is referenced rather than adopted for maintenance;
output inside retained evidence or operator metadata rejects.

## Retained-evaluation maintenance

After a native evaluation or standalone comparison finishes, rolling cleanup
keeps the latest three per group by recorded completion time, falling back to
creation time/identity. A just-finished evaluation is retained. A group combines Agent, benchmark-family combination,
model/API, model parameters/thinking and task scope. Each pi release
has an independent bucket; Opi source commits, Opi release versions and Companion
build identities do not reset its history.

Active/resumable operations, explicit pins, retained references and unknown
dependencies or cleanup status are protected. For pi, only excess unprotected copies
within the same pi release bucket are eligible. These protections may leave more
than three evaluations; returned retention results name removals and exceptions.
Completed preparation failures count when their grouped proof records no live
dispatch; stopped offline comparisons count when command/stream cleanup is
confirmed. Unknown or unsettled process cleanup remains protected.
When the command returns JSON, it contains the `retention` pin to `retention-handoff.json` and
`retention_status = applied|protected_excess|refused`. This separate receipt
preserves the already frozen `result-handoff.json`; a cleanup refusal does not
change the native result or erase its evidence.
Failure summaries and `retention-handoff.json` are also persisted when the CLI
rejects with an exception; inspect those files even when no result JSON is returned.
The operator maintains only its owned evaluation trees. A standalone comparison's
arbitrary `--out` remains caller-owned and is never adopted for deletion. Shared
materials, images and Cargo caches remain outside rolling cleanup.

Manual archive/GC remains available through reviewed plans:

```sh
python3 "$EVAL_OPERATOR" pin --config "$PROFILE" --id "$EVAL_ID" --reason "retain for review"
python3 "$EVAL_OPERATOR" unpin --config "$PROFILE" --id "$EVAL_ID" --reason "review complete"
python3 "$EVAL_OPERATOR" retire --config "$PROFILE" --id "$EVAL_ID" --reason "exclude from new selection"
python3 "$EVAL_OPERATOR" unretire --config "$PROFILE" --id "$EVAL_ID" --reason "selection restored"
python3 "$EVAL_OPERATOR" archive --config "$PROFILE" --id "$EVAL_ID" --plan "$FRESH_PLAN"
python3 "$EVAL_OPERATOR" gc --config "$PROFILE" --plan "$FRESH_PLAN"
python3 "$EVAL_OPERATOR" apply-plan --config "$PROFILE" --plan "$REVIEWED_PLAN"
python3 "$EVAL_OPERATOR" restore --config "$PROFILE" --id "$EVAL_ID"
```

Pin/unpin and retire/unretire record decisions outside sealed trees. Archive and
GC only preview an exact inventory/digest-bound plan; repeated `gc --id` restricts
its scope. Review selected entries and protected reasons before explicitly
applying that exact plan. Application rechecks paths, bytes, leases, references,
ownership and policy. A changed inventory refuses application. The latest-three
policy and the same protections bound selection; distinct pi releases stay in
independent buckets. Manual application does not broaden rolling cleanup to
shared materials, images or Cargo caches.

Restore extracts verified archive bytes to their original canonical path without
rewriting seals, absolute material paths or original scores. Missing cached images,
runtime packages or other dependencies remain typed portability gaps; restoration
does not promise offline regrading or a new executable run. Read
[environment cleanup](environment-cleanup.md) to distinguish disposable invocation
cleanup from explicit retained-evidence maintenance.

## Evidence and caller handoff

Read the actual schema/version and returned artifact references. Preparation
`handoff.json` and execution `result-handoff.json` have different meanings.
Original family contracts remain `opi-eval-prepared-operation/1`,
`opi-eval-baseline-snapshot/1`, `opi-eval-cross-run-comparison/1` and
`opi-eval-handoff/1`; evaluation contracts add `opi-eval-prepared-evaluation/1`,
`opi-eval-evaluation-handoff/1` and `opi-eval-evaluation-report/1` to bind family results.
An exit-zero process alone proves no native result or acceptance.

Keep full evidence external and apply [privacy review](evidence-root.md) before
sharing. `capture_policy = "full"` retains permitted streams, not private thinking,
complete sessions/final filesystems or telemetry absent from the clients. Use
[diagnostics](diagnostics.md) to assess score validity, diagnostic completeness
and comparison eligibility separately. Missing diagnostics cannot retroactively
invalidate an otherwise verified historical native score; proposed richer
diagnostic handoff fields remain unavailable until admitted and implemented.

Another explicitly authorized skill may consume this contract with supplied
scope/budget. A call does not authorize paid scope expansion, pi pin updates,
source repair, native regrade, commit or publication. Findings remain evidence
until their owning repair/audit workflow admits them; acceptance and Phase exit
retain their existing owners.
