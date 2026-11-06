# Mechanical version-document preparation

Shared procedure for an explicitly invoked release or documentation workflow;
this file is not a skill invocation or a separate approval state machine.

Precondition: `opi-release` has already updated the workspace version and
internal dependency version fields. Match the requested version to Cargo
metadata. Manifests and lockfile remain release-owned.

1. Locate maintained current-version claims in root/crate READMEs, install
   examples and current references. Use the document checker's source-derived
   version checks as a discovery aid, not a repository-wide text replacement.
2. Read owning source and update only claims that refer to the current Opi
   version. Keep English/Chinese counterparts synchronized. Preserve external
   versions, protocol identities, historical changelog sections and snapshots.
3. Report exact inspected/changed paths into the caller's existing manifest.
   Broader semantic drift is follow-up, not authority to expand this operation.
4. Run `python scripts/opi-doc-check.py` and `git diff --check` after the caller's
   final version/changelog edits; the caller reuses this result until those
   inputs change. No Rust build is required for prose-only version claims.

`opi-document scope=version-bump` owns this bounded documentation operation
when invoked directly. `opi-release` may perform the same bounded preparation
under its own explicit invocation without invoking the user-only document
skill. Neither grants broad README audit or normative-spec/ledger mutation.
