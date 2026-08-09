---
name: opi-release
description: Release the opi Rust workspace through explicit local, public GitHub, and irreversible crates.io gates.
arguments: "<version> [--fix] [--skip-cross]"
disable-model-invocation: true
---

# Opi Release

Release all publishable workspace crates at one lockstep semver, then publish
the corresponding GitHub release. Invoke explicitly with a target version.

## Inputs

- `<version>`: required semver without the `v` prefix.
- `--fix`: permits only the pre-flight formatter/linter fixes described below.
  It does not permit a commit before Phase 5.
- `--skip-cross`: source/crate release with no locally built cross-platform
  binaries. It does not weaken crate or documentation checks.

Use the host's native progress and user-interaction mechanisms when available.
The workflow must not depend on product-specific task or question tool names.
Shell examples are illustrative: use equivalent PowerShell commands on Windows
and POSIX commands on Unix without changing their safety semantics.

## Boundaries

| Phase | Boundary |
|---|---|
| 1 | Read-only unless `--fix` was supplied; fixes remain uncommitted. |
| 2-4 | Reversible local preparation with an exact changed-file manifest. |
| 5 | Public: release commit and tag are pushed. Requires fresh confirmation. |
| 6 | Irreversible: crates.io versions cannot be deleted. Requires fresh confirmation. |
| 7 | The GitHub draft becomes public; crate publication remains irreversible. |

Never describe Phases 1-4 collectively as side-effect free. Never use
`git reset --hard`, `git checkout --`, `git clean`, force-push, broad staging,
or automatic workspace-wide `cargo clean`.

## Resume state

Before Phase 1, inspect `.opi-release-state.json`. If it matches the requested
version, validate every recorded remote fact before offering to resume. Write
state atomically after each public or irreversible transition:

```json
{
  "version": "0.0.0",
  "release_commit": null,
  "tag_pushed": false,
  "draft_url": null,
  "published": [],
  "pending": [],
  "github_published": false
}
```

Do not infer success from the state file alone. Query GitHub/crates.io and stop
on disagreement.

## Phase 1: pre-flight

Run from a clean `main` whose `HEAD` exists on `origin/main` and whose required
CI checks for that exact SHA succeeded.

Required gates:

1. Confirm `LICENSE`, `README.md`, `Cargo.lock`, `CHANGELOG.md`, and every
   publishable manifest exist.
2. Confirm `git status --porcelain` is empty, the current branch is `main`,
   local `HEAD` equals `origin/main`, and neither local nor remote `v<version>`
   exists.
3. Query required check runs for the exact `HEAD`; pending, missing, or failed
   checks block release.
4. Treat the exact-SHA required CI checks as the evidence for format, all-target
   clippy, workspace tests, doctests, and rustdoc. Record their job URLs and
   conclusions; do not repeat the same five gates locally. If repository CI no
   longer covers one of them, run only the missing gate locally and record why.

5. Check package metadata, publish flags, internal dependency version fields,
   package contents, tracked secret-shaped files, MSRV, registry auth, crate
   ownership, and whether the target version already exists.
6. Run `cargo run -p opi-coding-agent -- --version` and verify the current
   version before the bump.
7. Treat `cargo audit` as required only when the repository/release policy
   declares it installed; otherwise report the missing optional check.

With `--fix`, run only `cargo fmt --all` and the narrowly applicable clippy fix.
Record the resulting paths, run `git diff --check`, and repeat only affected
local gates. Because the clean exact-SHA CI evidence no longer covers a dirty
fix, stop before release and require the resulting commit's required CI to pass.
Do not stage or commit the fixes.

Report PASS/FAIL/WARN with evidence. Any required failure stops the workflow.

## Phase 2: version and packaging preparation

### Version and document ownership

`opi-release` owns the mechanical workspace version mutation: update
`workspace.package.version`, every publishable internal dependency's exact
version field, and the resulting `Cargo.lock` changes. Show and record those
paths, then invoke:

```text
opi-document scope=version-bump version=<version>
```

`opi-document` owns documented version surfaces and localized counterparts; it
does not edit Cargo manifests or the version itself. Record its exact changed
paths in the same release manifest, show the combined diff, then run
`cargo check --workspace`.

### One dependency graph

Read `cargo metadata --format-version 1 --no-deps`, select publishable workspace
members, build internal path-dependency edges, and topologically sort them.
Reuse this graph in both dry-run and live publication; do not maintain a second
hand-written order.

Current expected batches are an assertion to verify, not the source of truth:

- Batch 1: `opi-ai`, `opi-tui`, `opi-protocol`
- Batch 2: `opi-agent`, `opi-sandbox`
- Batch 3: `opi-coding-agent`

The expected edges include `opi-agent -> opi-ai`,
`opi-sandbox -> opi-protocol`, and
`opi-coding-agent -> opi-ai, opi-agent, opi-protocol, opi-tui`.

Run `cargo publish --dry-run --allow-dirty -p <crate>` in computed order.
Before the new internal versions exist on crates.io, a dependent crate may fail
only because that exact internal version is unavailable; classify that as an
expected ordering constraint. Any metadata, package-content, or unrelated
dependency failure blocks release.

On failure, retain and show the recorded preparation diff. Do not silently
revert it; ask whether to fix forward or revert only the known files.

## Phase 3: changelog and release notes

Promote the existing `## [Unreleased]` content into
`## [<version>] - YYYY-MM-DD`. Never modify an already released section. Use
commits since the previous tag only to detect omissions or category mistakes,
not to overwrite curated changelog text.

Keep the repository's allowed headings and Conventional Commit mapping. Create
an untracked/transient release-notes file from the finalized version section.
Add `CHANGELOG.md` and any intentionally tracked note to the exact release
manifest.

Run `opi-document scope=targeted` if changelog edits create or invalidate a
declared documentation claim; otherwise record why no additional doc surface
was affected.

## Phase 4: build and artifact evidence

Ask the user to select one strategy:

1. CI-driven builds (recommended): local native smoke only; `release.yml`
   builds supported `opi` and `opi-sandbox` archives.
2. Local cross-build: build only targets supported by the host/toolchain.
3. `--skip-cross`: no binary release artifacts.

Always run a release build and release tests appropriate to the selected
strategy. For every locally produced archive:

- package only expected binaries/files;
- extract into a temporary directory;
- run the native binary's `--version` when executable on the host;
- inspect foreign binaries using a file-format tool;
- reject missing, duplicate, or unexpectedly large assets;
- generate `SHA256SUMS.txt` over the exact archive set and verify it locally.

Keep artifacts under `release-artifacts/v<version>/`. Build failures retain
narrow evidence for diagnosis. Do not automatically run workspace-wide
`cargo clean`.

## Phase 5: public Git boundary

Show the full release diff, exact staging manifest, target commit, tag, and
planned GitHub assets. Warn that the commit/tag push is public, then require
explicit confirmation.

After confirmation:

1. Stage each reviewed tracked path explicitly. Never use `git add -A`,
   `git add .`, globs, or command substitution over `git diff`.
2. Run `git diff --cached --name-only` and `git diff --cached --check`; verify
   the staged set equals the manifest.
3. Commit `chore: release v<version>` and create annotated tag `v<version>`.
4. Push `main` and the tag without force.
5. Create a draft GitHub release from the transient notes.
6. For local artifacts, upload the archives and their `SHA256SUMS.txt` together.
   For CI-driven builds, initially upload notes only.
7. Record and atomically checkpoint the release commit, tag, and draft URL.

For CI-driven builds, wait for the tag-triggered `release.yml`. It must build
the declared targets, run the release evidence audit, generate
`SHA256SUMS.txt`, and upload archives plus checksums to the draft. A missing
asset, failed required target, or failed audit blocks Phase 6.

## Phase 6: crates.io publication

Show the validated dependency batches, already-published set, and the warning
that crates cannot be deleted. Require explicit confirmation immediately before
the first `cargo publish`.

Publish one computed batch at a time. Independent crates within a batch may run
in parallel only if output and result attribution remain unambiguous. Verify
each version through the registry before advancing, allowing bounded index
propagation waits.

Retry automatically at most three times only for transient network/5xx/index
propagation failures. Authentication, validation, version conflict, missing
dependency, or uncertain partial success requires a user decision.

After every verified crate, atomically update `.opi-release-state.json`. On a
mid-release stop, offer retry, bounded wait, resume later, or yank-and-abort.
Yanking is itself destructive and requires explicit confirmation; it does not
delete the published version.

## Phase 7: finalize and verify

Before publishing the draft:

1. Download the GitHub release assets into a fresh temporary directory.
2. Verify every downloaded archive using the downloaded `SHA256SUMS.txt`.
3. Extract and smoke the host-native `opi` binary; audit the declared
   `opi-sandbox` platform assets.
4. Verify all six crates at `<version>`:
   `opi-ai`, `opi-tui`, `opi-agent`, `opi-protocol`, `opi-sandbox`, and
   `opi-coding-agent`.
5. Install `opi-coding-agent --version <version>` into an isolated cargo root
   and verify `opi --version`.
6. Report docs.rs state as eventual/non-blocking unless repository policy says
   otherwise.

Publish the draft only after these checks pass. Mark the resume state complete,
then remove the transient release-notes file. Retain release artifacts by
default. Disk-cache deletion is a separate user decision with an explicit,
resolved target path.

## Failure recovery

- Before Phase 5: stop with the exact local diff and evidence. Fix forward or
  revert only after the user chooses the known paths.
- After Phase 5 but before Phase 6: retry draft/asset operations when safe. To
  abandon, delete the draft/tag only after confirmation and revert the public
  release commit; never rewrite history.
- During Phase 6: preserve the verified published set. Resume from registry
  truth, not memory. Published versions can only be yanked.
- After Phase 7: amend the GitHub release through a new auditable action; never
  mutate crates.io contents in place.

## Completion report

Report the release commit/tag, GitHub URL, all six crate URLs/versions, artifact
count, checksum verification, selected build strategy, any tier-2 omissions,
and the final state-file disposition. A release is not complete merely because
the commands exited zero; remote versions and downloaded assets must agree.
