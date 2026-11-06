---
name: opi-release
description: Publish a verified lockstep Opi release through explicit GitHub and crates.io gates.
arguments: "<version> [--fix] [--skip-cross]"
disable-model-invocation: true
---

# Opi Release

Publish every publishable workspace crate at one semver, then finalize its
GitHub release. Invoke explicitly with a version without the `v` prefix.
`opi-eval` is an unpublished development calibration tool, not a release gate.

## Inputs and authority

- `<version>`: required target semver.
- `--fix`: permits only formatter and narrowly applicable clippy repairs during
  preflight. Leave them uncommitted and stop for their own reviewed delivery;
  this option does not bypass candidate CI.
- `--skip-cross`: source/crate-only release, with no attached binary assets.
  The current automatic tag workflow builds binaries: this option therefore
  stops before a tag push unless an explicitly reviewed workflow strategy
  supports it. It does not silently disable CI.

Default to CI-built assets; use local cross-build only when explicitly
requested. Read [CI and platform policy](../../../docs/ci.md) when selecting
verification. Ordinary patches need Linux product CI plus affected-platform
smoke. New `0.N` series (after 1.0, a new major), major milestones, or changes
with unbounded cross-platform impact need full Linux/Windows/macOS regression.
Version size never exempts a platform-specific fix from that platform's proof.

Local preparation is authorized and reversible, not side-effect free. Keep an
exact task-owned changed-file manifest and preserve unrelated work. The two
fresh confirmations are: public Git/Actions/draft operations, then irreversible
crates.io publication. No broad staging, force-push, history rewriting, automatic
revert, workspace-wide `cargo clean`, or cache deletion.

## Entry: fresh release or recovery

Inspect `.opi-release-state.json` before preflight. For a matching incomplete
release, read [recovery](references/recovery.md), reconcile remote facts, and
continue at the first unfinished transition rather than reapplying fresh-entry
requirements. A different version, changed candidate or conflicting remote
identity stops for a decision.

The ignored file is temporary recovery state, not a progress ledger. Write it
atomically after each verified external transition. Record version, phase,
base/candidate commit, exact changed paths, asset strategy, selected platform
suite/run URLs, tag/draft identity, verified published/pending crates, and final
GitHub publication state. A lost checkpoint is reconciled from remote truth;
the state file alone never proves success. Do not change `.opi-impl-state.json`.

Record elapsed preparation, build/test, packaging, registry waiting, consumer
verification and human waiting separately. Report unmeasured time as unknown.

## 1. Preflight -> verify the baseline and release plan

Before local Cargo work, read the shared
[Cargo cache lifecycle](../_shared/references/cargo-cache-lifecycle.md),
inspect the selected persistent target once and coordinate writers. Preserve
versioned release artifacts and failure evidence; maintenance is separate.

For a fresh release:

1. Require clean `main`; compare `HEAD` with the actual remote `refs/heads/main`,
   not merely a possibly stale tracking ref. Neither local nor remote target
   tag may exist. Do not switch branches or discard changes.
2. Confirm license, root README, lockfile, changelog and publishable manifests.
3. Query `ci.yml` for this exact baseline SHA and inspect its selected checks,
   including the stable `required` gate. Missing, pending or failed required
   checks block entry. Record URLs and conclusions, not only a green badge.
4. Check package metadata/content, publish flags, internal dependency versions,
   secret-shaped tracked files, MSRV, registry authentication, crate ownership
   and target-version availability. `cargo audit` is required only if declared
   by repository policy; otherwise report its optional status.

Use existing applicable CI for format, clippy, tests, doctests and rustdoc.
Run only missing checks locally. Do not run the same five gates again simply
because this is a release. `--fix` repairs invalidate affected proof and stop
before release; do not commit those repairs here.

## 2. Prepare -> verify one final local delta

Read the complete Unreleased section before editing it. Update the workspace
version, publishable internal dependency version fields, and Cargo-generated
lockfile; review unexpected transitive changes. Refresh documented versions
using the shared [version-document procedure](../_shared/references/version-documents.md).
This bounded preparation does not invoke `opi-document`; broader semantic
README work requires its own explicit scope.

Promote curated Unreleased content to `## <version> - YYYY-MM-DD`, retaining an
empty Unreleased heading for future work. Commits since the previous tag only
help identify omissions; they do not replace curated notes. Keep released
sections immutable. Historical detail compression is separate, explicitly
reviewed documentation maintenance, not an automatic release operation.
Generate transient release notes and include every intentionally tracked path
in the manifest. Check affected English/Chinese counterparts together.

Run the documentation checker and `git diff --check` once after these final
edits. Run only affected checks again if their inputs change. No standalone
workspace build or full local test rerun is required by a mechanical bump.

## 3. Package plan -> verify the graph and pending obligations

Read `cargo metadata --format-version 1 --no-deps`, select publishable workspace
members, and topologically sort internal path dependencies. Use this one graph
for dry-run, live publication and final version checks; do not keep a second
hand-written order. Exclude `publish = false` members.

Run `cargo publish --dry-run --allow-dirty --locked -p <crate>` in graph order.
Only an unavailable exact new internal version may defer a dependent dry-run.
Record it as **not yet verified**, not PASS; metadata, content, or unrelated
resolution failures block. Recheck each deferred package once its dependencies
are visible, before publishing that package.

For default CI assets, plan the supported Opi archives and native sandbox
archives from `release.yml`. Full regression and binary distribution are
separate: a patch may still build all advertised assets without repeating full
three-platform regression. Release builds and extracted-artifact smoke prove
the shipped bytes; debug CI does not replace them. Additional full release-profile
tests are needed only for an identified profile-sensitive behavior.

For explicitly requested local assets, first establish a single upload owner.
The automatic tag uploader must not race a local uploader. If that cannot be
established within the reviewed strategy, stop before pushing the tag.
Keep local assets under `release-artifacts/v<version>/`; package only expected
files, inspect foreign file formats, extract/native-smoke archives, reject
missing/duplicate/unexpected assets and generate/verify `SHA256SUMS.txt` over
the exact archive set.

## 4. Public boundary -> verify the actual release candidate

Show the complete diff, exact staging manifest, parent commit, proposed tag,
asset plan, selected platform suite and planned Actions/draft operations.
Explain the public boundary and require fresh confirmation.

After confirmation:

1. Stage reviewed paths explicitly. Verify cached names equal the manifest and
   `git diff --cached --check` passes. Commit `chore: release v<version>`.
2. Record the candidate SHA, push `main` without force and checkpoint.
3. Wait for this candidate's required `ci.yml` checks. The baseline's CI is not
   candidate evidence. Do not push the tag or publish crates while this fails.
4. Reuse exact-candidate platform results where applicable; otherwise dispatch
   `platform-verify.yml` at that full SHA for the selected missing platforms and
   suite. A large iteration needs full regression on all three; successful
   final-candidate Linux product CI supplies Linux coverage, so normally dispatch
   only Windows/macOS with `suite=full`. An affected-platform patch needs at
   least its smoke. Inspect actual selected coverage;
   unselected platforms are not a passing result. Record URLs and checkpoint.
5. Create annotated tag `v<version>`, push it without force and checkpoint.
6. Create the GitHub draft from the final notes and checkpoint. The tag uploader
   waits a bounded interval for that draft; if creation was interrupted, retain
   build artifacts and recover rather than substituting a public release.
7. Wait for the tag-triggered build and mandatory sandbox evidence audit.
   Download the exact assets and checksum file, verify every checksum and
   confirm the declared asset set. Reuse CI extracted-artifact smoke only when
   the downloaded bytes match its recorded evidence; otherwise extract/smoke
   on a native host. Checkpoint verified asset identities. A missing required
   target or failed audit blocks.

`release.yml` is the default upload owner. It serializes runs for the tag,
requires a draft and does not clobber existing assets. Replacing an asset is a
separate reviewed operation; do not overwrite bytes after verification.

## 5. Irreversible boundary -> verify each published crate

Show graph batches, verified/deferred dry-runs, already-published versions and
remaining crates. Warn that crates.io versions cannot be deleted and require
fresh confirmation immediately before the first live `cargo publish`.

Publish graph batches with `--locked`; independent crates may run concurrently
only with unambiguous results. Verify each version through the registry before
advancing and checkpoint each verified crate. Allow bounded index propagation
waits. Re-run deferred dry-runs after dependency visibility and before that
package's publication.

Automatically retry at most three times for transient network/5xx/propagation
failures. Authentication, validation, version conflict, missing dependency or
uncertain partial success requires reconciliation and a user decision.
Yank-and-abort requires separate explicit confirmation and never deletes bytes.

## 6. Finalize -> verify registry consumers and publish the draft

Reconfirm candidate/tag identity, expected registry versions and the asset
identities verified above; download/reverify again only if assets changed or
proof is incomplete. Source-only strategies must claim and attach no binaries.

Install the published `opi-coding-agent` and `opi-sandbox` into isolated Cargo
roots with the exact version and `--locked`. Verify `opi --version` and the
sandbox's documented CLI/doctor posture. Registry existence alone is not
consumer build proof. Report docs.rs as eventual/non-blocking unless policy
says otherwise.

Publish the draft only after consumer and remote checks pass. Checkpoint
completion, then remove the temporary state and notes; keep non-empty artifacts
by default. Report candidate/tag, GitHub/crate URLs, build strategy, actual
platform/suite coverage, required/optional target outcomes, checksum and consumer
results, measured stage times and state-file disposition. Command exit zero
alone is not completion.

On any failure, retain the exact diff, original proof and verified remote set.
Use [recovery](references/recovery.md); never restart already published work or
silently widen authority.
