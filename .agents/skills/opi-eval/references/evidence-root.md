# Evidence root and privacy review

Read before any native run to plan capture, and again when staging or verifying
evidence. The [offline acceptance verifier](../../../../crates/opi-eval/scripts/verify-local-acceptance.py)
owns the acceptance schema and current field checks. The Companion owns its
run/bundle formats. This reference explains how to populate the handoff from
real outputs; no template or synthetic test tree is evidence.

## Privacy without breaking seals

Use three distinct locations: private execution storage, a fresh shareable
evidence root, and verification/report output outside that evidence root. Keep
credentials, canary files, personal configuration and private diagnostics out
of the shareable root. Configure isolated Agent homes, capture policy and
non-personal paths before freezing plans; inspect the pinned integrations'
credential cleanup and capture behavior before starting the provider.

The current CLI supplies leak rejection through `--canaries` and credential
cleanup, not an arbitrary post-run redaction/export command. `capture_policy`
is not a guarantee that all personal/sensitive data has been removed. Review
exportable commands, configuration references, logs, tool args/results, native
captures, delivered files/patches and report excerpts for secrets, identifying
paths and unrelated user data. A scan result reports only paths/categories,
never the matched secret. Unknown privacy status blocks sharing.

Copy accepted sealed evidence byte-for-byte. Redact only separately derived
summaries/log copies outside seals and label their origin; a redacted derivative
cannot replace a native artifact that a seal or verifier-input digest binds.
If a sealed artifact itself contains sensitive data, keep it private and block
the handoff. Record the affected path and missing capability, then use a newly
authorized safe capture/run or seek a separately admitted fix. Never edit,
rehash or re-seal the old bundle to pretend the original passed.

For ordinary benchmarks, retain each original run tree, configs/plans/materials,
build/source identities and command logs, plus the Companion reports and a
privacy-reviewed human report. Do not force single-subject or other-model runs
into the special six-trial acceptance schema.

## Acceptance layout

Stage after qualifying actual trials exist. All paths inside `manifest.json`
are relative to the evidence root and must resolve within it. Copy the complete
run trees, including receipts, journals, native job/artifact trees and original
bundle manifests; the outline shows required entry points, not a deletion list.

```text
evidence-root/
  manifest.json
  upstream-identity-cache.json
  runs/
    terminal-bench-2.1/
      run-control.json
      run-report.json
      material.json
      trials/<actual-trial-id>/bundle/
        manifest.json
        artifacts/control/experiment.json
        artifacts/native/verifier-input.json
        artifacts/native/<preserved native captures>
    terminal-bench-3.0/<same required run structure>
    deepswe-v1.1/<same required run structure>
  opi-artifact-audit/
    <first Opi trial id>/<complete native capture tree>
    <second Opi trial id>/<complete native capture tree>
    <third Opi trial id>/<complete native capture tree>
  inputs/<frozen configs, plans, doctor/prepare outputs>
  builds/<verified build and runtime identities>
  commands/<sanitized command records and preserved exits>
  reports/<original native JSON reports>
  attempts/<retained earlier failed runs and replacement records, if any>
```

Copy each exact prepared material manifest to `runs/<family>/material.json`
without editing embedded paths or claims. Keep original local material/cache
available where native regrading needs it. Offline `verify`/`report` evidence
portability does not imply that moving an absolute-path plan enables a new run.

## Manifest field sources

Populate these existing verifier fields with real records. Hash final preserved
file bytes with SHA-256 (64 lowercase hex characters, no invented values).
Keep digests of redacted derivatives distinct from their private originals.

| Field | Required contents and source |
|---|---|
| `schema` | `opi-eval-local-acceptance/1`. |
| `operator` | Non-sensitive operator label, actual run authorization/capture scope and privacy review result. `redacted: true` only after review. |
| `opi_repository` | `url` and full `commit` from the actual Opi source/build record, with dirty-state disclosure alongside. |
| `sources` | `pi`, `harbor`, `pier`, each with `repository_url` and full lowercase 40-hex `commit` from the corresponding pinned source. |
| `builds` | One row per `agent` (`opi`, `pi`), `executable_sha256` matching the exact executable in native material/terminal attribution; link version, source, toolchain, runtime/bundle and dependency identities. |
| `controls` | Positive `execution_concurrency`; `connection_controls` object derived from frozen configs and effective observations, including provider/API/endpoint/model, controls and their origins. Preserve per-family bounds, trial counts, usage/time scope and credential **references**, never values. |
| `trials` | Exactly six rows with actual `id`, `agent`, `family`, `run_root`, `bundle_digest`, and `attribution.agent_executable_sha256`. Bind actual task/repetition/job, connection/tool-loop/native handoff evidence alongside. |
| `trials[].bundle_digest` | SHA-256 of that trial's `bundle/manifest.json` bytes, not the manifest's internal bundle identity or a directory digest. |
| `material` | Exactly three rows with `family` and `material_sha256`, computed from each preserved `runs/<family>/material.json`. |
| `command_exits` | Real records with `family`, `command`, `exit`; include `run`, `verify`, `report` for every family plus preparation/plan records. `verify` and `report` must exit 0. `run` may exit 0, or exit 1 only when that family's `run-report.json` has a nonempty `trials` list whose entries all have `status = "sealed"` and a `verifier.completion` string starting with `verified`. Other verifier checks still apply; other run exits reject. Retain nonzero attempts and report any blocker. |
| `preserved` | Exhaustive inventory of referenced preserved files using `path` and `sha256`, with optional `role`. Include controls, plans, builds, logs, native inputs/outputs, reports and upstream cache; exclude `manifest.json` itself to avoid self-hashing. |

Fill fields from native material, original `run-control.json`, sealed
settlements and integration terminal/binding artifacts, not just the operator's
requested config. Check six unique trial IDs and the exact three families.
Subject IDs need not use fixed labels: derive each subject's `product` from
`bundle/artifacts/control/experiment.json`, whose bytes must match its entry
digest in the bound bundle manifest. Require the run-control experiment ID
and manifest digest to match that sealed experiment, and the planned trial's
subject, task and group to match its unique sealed declaration. The unique
subject record's product must match the acceptance row's Agent; missing,
duplicate or substituted bindings reject. A missing actual service-model
observation remains unknown with a reason; do not manufacture one from the
requested selector.

For the upstream cache, use schema `opi-eval-upstream-identity-cache/1` and
`repositories[repository_url].commits[full_commit] = <observed metadata>`.
Before going offline, record commit/tree identity from each verified upstream
checkout or a retained authoritative acquisition response. Preserve that
source evidence too. Merely copying manifest claims into this object is not
independent provenance. A pi/Harbor/Pier commit is checked in its own repository,
never resolved as though it belonged to the Opi checkout.

Build the generic-audit view by recursively copying each of the three Opi
trials' entire `bundle/artifacts/native` directory into
`opi-artifact-audit/<trial-id>`. Compare complete directory inventories and
file bytes, including nested entries. Include exactly those three capture
directories, no pi captures and no hand-picked excerpts. The verifier checks
non-emptiness, exhaustiveness and byte identity; generic auditor exit zero by
itself can be an empty scan and is insufficient.

## External records and legacy references

Return the external evaluation ID and `record.json`, exact root/report/check
paths, manifest SHA-256 and schema versions when applicable, material schema,
Companion identity and current consumability gaps. The operator's external
catalogue reads JSON records, immutable import references and lifecycle receipts
directly, without a database or persistent query cache. These files are owned
only by the evaluation operator. Reviewers query them
or use the supplied immutable references, then verify the original evidence.
Index metadata cannot establish a seal or score.

`docs/eval/evidence-roots.jsonl` was a historical project index; it has been
removed and its records stay in Git history. External JSONL indexes are still
imported only by an explicit operator
`import-index --config <profile> --index <legacy-jsonl>` request, read-only,
without copying or rewriting originals. New runs neither require nor write one.

Archive/GC decisions and receipts stay outside sealed native trees. Restoration
returns verified archive bytes to their original canonical path without editing
absolute material references or seals. Storage unavailability, retirement and
freshness are separate from historical result validity. Preserve required
material/runtime/image dependencies; a portable report does not imply portable
native regrading. Missing dependencies remain explicit gaps.

Finished operations keep the latest three per retention group, protecting pins,
retained references, active operations and unknown cleanup/dependencies. Protected
exceptions remain locatable and are reported. Rolling cleanup never adopts
caller-owned standalone output or shared material/build caches.

## Offline checks

Resolve `REPO` to the Opi checkout used for the generic repository audit,
`EVIDENCE` to the staged root, `EVAL` to the built Companion, and `CHECKS` to a
fresh directory outside that root. Record verifier/Companion identities and
each command's real exit and stdout/stderr. Never load credential values for
these checks; keep credentials out of subprocess environments.

```sh
python3 "$REPO/crates/opi-eval/scripts/verify-local-acceptance.py" --root "$EVIDENCE" --out "$CHECKS/acceptance.json" --companion "$EVAL"
python3 "$REPO/scripts/opi-artifact-audit.py" "$EVIDENCE/opi-artifact-audit" --workspace-root "$REPO" --json
```

The first command accepts only with exit 0 and `accepted: true`; exit 2 is a
rejection, including a missing root, existing output, output inside evidence,
or invalid evidence. It invokes only Companion `verify` and `report` per family.
Preserve the second command's JSON and verify audited content is non-empty in
addition to the first command's exhaustive capture binding. Retain separate
semantic checks for the real tool loops, declared task handoffs and six valid
native settlements; the structural verifier is not an all-criteria oracle.

Also reconstruct each family report twice with Companion `report` into fresh
paths under `CHECKS`, compare bytes, and compare with the preserved original
native report. Record both file hashes and the comparison exit. Do not place
verification output into the sealed run roots or overwrite prior reports.

For an explicitly requested native regrade, use `regrade --root R
--native-material M --out D` with fresh `D` outside source evidence and cached
offline verifier materials. Retain derived grading records separately; missing
offline materials/network equivalence are rejection reasons, not permission
for an Agent/provider rerun.

**Completion criterion**: privacy-reviewed originals still verify; the
acceptance structure, repository identities, non-empty byte-bound Opi view,
native semantics and deterministic reports all have recorded results. Hash
`manifest.json` last and return its digest with external verification reports.
