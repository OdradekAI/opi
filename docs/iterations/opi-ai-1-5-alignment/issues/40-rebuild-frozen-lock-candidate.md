# Rebuild an isolated candidate from the frozen npm lock

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [bind the isolated Pi candidate to lifecycle replay](39-bind-isolated-candidate-lifecycle-replay.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` plus the #39 worktree changes

## Installation and build

On 2026-11-11, [the restoration tool's source-copy helper](../tools/restore_pinned_catalog.py)
copied the frozen snapshot into `C:/Temp/opi-ai-frozen-lock-20261111`. The root lock
was written from the verified Pi `v1.5.0` Git blob
`a0749b93b38ead1c75351751883ed95dec5cb32f` at commit
`f9a26365568127461c47a07a37dcb42c140a3ac5`. All 2949 frozen source inputs matched
before and after installation. The root lock kept SHA-256
`802adbf0a5e093830226723f37b8971ce42eee442181a1471e0b70b1396b244a`.

Node 24.13.0 ran its bundled npm 11.6.2 directly with
`npm ci --offline --ignore-scripts --no-audit --no-fund` and dev, optional, and
peer dependencies included. The cache had one missing tarball,
`typebox@1.1.38`. It was fetched from the lock's registry URL and verified
against the lock's SHA-512. npm skipped the three frozen extraneous entries
that reference it. The clean offline install added 285 packages and generated
a candidate-local `node_modules/.package-lock.json`.

| Installation check | Result |
| --- | --- |
| Registry packages on disk | 269 locked names and versions. All 268 distinct cached tarballs match the lock's SRI |
| Workspace packages | 16 links resolve to the matching manifests inside the candidate |
| Platform selection | 112 incompatible optional entries omitted on Windows x64 |
| Frozen extraneous entries | Four rows in the installed lock have no matching package on disk |
| `npm ls --all --json --offline` | Exit 0. npm reported two installed, locked dev-optional packages as extraneous, `@emnapi/runtime@1.11.3` and `@img/sharp-wasm32@0.35.5` |

The audit compared installed-lock identities with the frozen root lock and
checked actual package manifests and link targets before building and after
replay. npm 11.6.2 warned about the project `min-release-age` setting. This run
used offline installation with the fixed lock and cached artifacts.

The candidate's own TypeScript 7.0.2 built telemetry, schema and AI after the
offline hydrator restored the saved pinned response. All four generated tree
manifests match the reviewed #38 candidate byte for byte. Importing the
compiled catalog confirmed 42 providers and 1712 model values.

## Bound replay

The [v3 material manifest](../evidence/phase5-frozen-lock-materials.json)
records the two lock hashes, the generated tree identities, and
`blocked_gates: []`. The [runner](../../../../scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py)
verified the same manifest before and after replay and used its candidate path
for all nine TS invocations. The cached Cargo build succeeded and each of the
three Rust invocations returned exit 0.

All three cases passed their shared observations. The overall verdict was
`DIFFERENCES_REQUIRE_DECISION` / exit 0. The first two cases still observe
one TS setup call at construction and zero on the Rust side. The failure
case still compares a single error terminal across different fault injection
points and error channels. Native install hook execution, upstream npm package
consumption and complete feature, platform and behavior acceptance remain open.

## Verification records

- [Install, build and installed-package audit](../evidence/phase5-frozen-lock-rebuild.json), SHA-256 `754130ae5a249e50ae2f8f4aafa386fe0b6a8d9c8a6bdacce8dc328ed7e494f9`.
- [Frozen material manifest](../evidence/phase5-frozen-lock-materials.json), SHA-256 `fc188607ef588d33208dadf5696c600029840229dc96a6bcb8d2a7b233104d70`.
- [Replay commands and raw traces](../evidence/phase5-frozen-lock-lifecycle-differential.json), SHA-256 `d1204a6f87c3751dae452fbbc811b8a2ff487591b36e06cf26d745f1b7b2a4bd`.

Post-replay checks matched the installation audit, the reviewed #38 generated
trees, the original supplied source and generated trees, and all three #39
comparisons. The existing #39 material manifest also revalidated. Test impact
is `retain` for the existing runtime catalog probe, freezer and replay.
