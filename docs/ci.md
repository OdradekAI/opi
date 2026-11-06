# Product CI and platform verification

[简体中文](ci.zh.md)

## Routine checks

`ci.yml` always runs a cheap scope/documentation job and a stable `required`
result. Branch protection should require that result instead of permanently
requiring every optional matrix job. Changing repository rules is a separate
maintainer operation; editing YAML does not update remote branch protection.

The scope job runs `scripts/opi-doc-check.py` and its Python regression suite,
plus the scope-selector and smoke-wrapper tests. Documentation checks cover
files, local links, metadata and source-derived values; prose meaning and
runtime behavior require their owning review/tests.

The scope selector compares exact Git commits, includes both sides of renames,
and fails closed when history cannot be read. Markdown/license/ignore-only
changes and development-only eval changes do not start Rust product builds.
Other changes run Linux product clippy, tests, feature-gated execution
acceptance, doctests/rustdoc and a separate declared-MSRV check. Formatting
remains workspace-wide and cheap. There is no repeated Phase-named acceptance
job: default workspace tests already execute those binaries.

Product Cargo gates use `--workspace --exclude opi-eval`. `opi-eval` is currently
an unpublished development calibration tool, not product runtime, CI or release
prerequisite. Its changed logic still needs scoped local offline tests; live
trials require their own scope, budget and authorization. This CI policy does
not cancel the long-term independent-evaluation direction in the specification.
The old eval workflow bytes are [inert development contracts](../crates/opi-eval/development/workflow-contracts/README.md), not Actions entrypoints.

## Choose native evidence separately

`platform-verify.yml` supports manual dispatch and reusable workflow calls:

- `ref`: full lowercase 40-character commit SHA; branch names are rejected.
- `linux`, `windows`, `macos`: independent booleans; select at least one.
- `suite=smoke`: sandbox lib, auth-process lifecycle, native keyring, cross-mode
  and recovery tests, plus feature-gated execution acceptance.
- `suite=full`: all product workspace test targets plus the feature-gated
  execution acceptance, without replaying the smoke subset a second time.

Each selected runner records the checked-out candidate, platform and suite in
the Actions summary. Inspect the checkout/input SHA, not just the workflow
definition's branch SHA. Missing or unselected platforms are not passing proof.
Linux clippy, docs and MSRV remain routine product checks; full platform runs
add native regression, not another copy of those platform-neutral gates.
Successful exact-candidate Linux product CI already supplies full Linux
regression; a major iteration normally dispatches only Windows/macOS full suites.

| Change or delivery | Default evidence |
|---|---|
| Documentation/skills prose | Document checker and diff check; no Rust build |
| Ordinary product change | Linux product CI |
| Platform-sensitive patch | Linux CI plus affected native smoke |
| New `0.N` series, post-1.0 major, major milestone, unbounded platform impact | Full Linux/Windows/macOS regression at the release candidate |

The selector adds Windows/macOS smoke for manifests/lockfile, workflows,
sandbox/TUI, execution and known process/keyring/platform paths. PowerShell
changes select Windows, where native development-wrapper parity also runs. Path selection
is an aid, not semantic impact analysis. Owners must request further platforms
when shared process, terminal, path, locking, native dependency or build behavior
changes outside these names. Version size sets a default, never an exemption.

The former macOS stabilization entry is replaced by selecting macOS here.
Foreign-architecture compilation now belongs to release builds rather than six
extra checks on every PR. Native three-OS regression is not a claim that every
foreign architecture executed tests.

## Release and evidence reuse

A patch may retain all advertised binary downloads without full three-platform
regression. `release.yml` still builds six Opi targets (Windows ARM64 is Tier 2),
produces four native sandbox archives, and requires the complete sandbox
evidence audit. Build, extracted archive smoke and checksums prove delivery
bytes; workspace tests prove different behavior and do not replace packaging.

The release skill waits for its final candidate's required CI, requests only
missing native evidence, then pushes the tag. One tag-scoped uploader waits
boundedly for the draft and never clobbers existing assets. Source-only/local
asset strategies must resolve the automatic uploader before any tag push.
Public Git/Actions and irreversible registry publication keep separate gates.

Use the existing [scope/check-selection contract](../.agents/skills/_shared/references/change-scope-and-check-selection.md)
for local work. Reuse executed proof only while source/build inputs, toolchain,
features, target and command remain applicable. An audit may inspect raw proof
but does not inherit an implementer's verdict. Changed inputs invalidate only
affected proof. No new global verification ledger or cache is introduced.

## Timing and verification limits

Historical successful [CI run 41300159581](https://github.com/OdradekAI/opi/actions/runs/41300159581)
took about 51m47s; its Linux workspace test step took 51m29s, including 38m39s
in eval's `native_driver`. Windows/macOS test jobs took 32m38s/15m43s. The
[v0.8.3 artifact workflow](https://github.com/OdradekAI/opi/actions/runs/38840442512)
took about 6m22s. These are baseline observations, not post-change measurements
or latency guarantees. Local release preparation, registry waits and consumer
installation need separate timing; do not infer them from tag-build duration.

Run offline selector/subprocess tests, smoke-wrapper parity tests, document
checks and the owning Rust CI/release topology binaries after workflow edits.
Hosted runner/toolchain behavior and end-to-end publication require separately
authorized remote execution; a static test does not prove those outcomes.
