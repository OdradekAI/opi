# Bind the isolated Pi candidate to lifecycle replay

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [isolated pinned catalog restoration](38-isolated-pinned-catalog-restoration.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344`

## Material binding and replay

On 2026-11-11, the [freezer](../tools/freeze_acceptance_materials.py) recorded
`C:/Temp/opi-ai-pinned-catalog-reviewed-20261108` with `--snapshot`. The
schema-v3 manifest binds the upstream path separately from the Rust workspace.
Its frozen source comparison, four generated trees, raw catalog pin and
dependency/dist/catalog inspection match the [reviewed #38 record](../evidence/phase5-restored-catalog-reviewed-candidate.json).
The compiled catalog module exports all 42 providers and 1712 models from the
pinned response.

The [lifecycle runner](../../../../scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py)
takes the upstream path from the manifest and verifies both the manifest bytes
and material identities before and after replay. All nine TS invocations used
that path. The shared Cargo cache built the Rust example once and the runner
executed it once per case.

| Case | Shared observations | Open difference |
| --- | --- | --- |
| `lazy-no-consumption` | Pass, including source completion and final result | Setup calls at construction were one in TS and zero in Rust |
| `lazy-stop-consumption` | Pass, including source completion and final result | Same setup timing difference |
| `lazy-setup-failure` | Pass for one error terminal | Error channels and fault injection points differ |

The replay recorded `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit 2. The catalog
matches its pin. The root `package-lock.json` in the candidate still differs
from the frozen bytes. The candidate links installed package directories and
has no npm-generated `node_modules/.package-lock.json` of its own. Both lock
observations remain in `blocked_gates`.

- [Material manifest](../evidence/phase5-isolated-candidate-materials.json), SHA-256 `9fd92d85d9aa30bdcade0569ab39a77c61e8114f86e92e816caef9ab78db0cf9`.
- [Replay commands, raw traces and comparisons](../evidence/phase5-isolated-candidate-lifecycle-differential.json), SHA-256 `9afc91b18a0ed4c16acca860b68a18a6c15e6a4a61dbaeeb3aaefd09c3740c18`.

## Recovered original lock

The adjacent checkout at `D:/Luiz/Odradek/pi` contains the original
`v1.5.0:package-lock.json`. The tag resolves to commit
`f9a26365568127461c47a07a37dcb42c140a3ac5` and that path to blob
`a0749b93b38ead1c75351751883ed95dec5cb32f`. Reading the blob with `git cat-file blob`
through Python's binary subprocess output gave 185524 bytes and SHA-256
`802adbf0a5e093830226723f37b8971ce42eee442181a1471e0b70b1396b244a`, matching
the [frozen source manifest](../evidence/phase1-baseline.json).

The replay used the same installed material as #38. Its root lock was 185020
bytes with SHA-256 `79490cd0acd238e0ad915d96c77bfddb5a07bf73fefcadc46cc13ec25785c721`.
The verified original blob provides the input for a fresh install and rebuild
in a new candidate. Installed transitive provenance and npm package consumption
remain open.

## Verification

| Check | Result |
| --- | --- |
| `python -B docs/iterations/opi-ai-1-5-alignment/tools/test_acceptance_materials.py -v` | 10 passed |
| `python -B scripts/pi-sdk-conformance/differential/test_runner.py -v` | 6 passed, including the stale-material CLI path and mid-run manifest replacement |
| `python -B scripts/pi-sdk-conformance/differential/test_lifecycle_differential.py -v` | 7 passed |
| `node --experimental-import-meta-resolve --test docs/iterations/opi-ai-1-5-alignment/tools/test_inspect_acceptance_materials.mjs` | 4 passed |
| Freeze with `--snapshot` and replay with `--materials` | Freeze exit 0, replay exit 2 with all three shared observations passing |
| Recorded manifest hash, command paths, driver hashes and stdout hashes | Verified against the evidence and current files |

Test impact `add` for alternate snapshot binding and drift, manifest replacement
and the CLI negative case. `update` for schema and replay fixtures. `retain` for
product regressions, comparison rules and historical evidence. `delete` none.
