# Restore the pinned catalog in an isolated upstream candidate

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [ordinary source-tail settlement](37-ordinary-source-tail-settlement.md)

## Scope and acceptance

Continue the approved alignment work without changing the frozen baseline or
user-supplied `.pi/pi-1.5.0` tree. Use the verified saved catalog response and the
snapshot's offline hydrator in a fresh working copy. Rebuild schema, telemetry
and AI outputs; prove generated source/dist values and actual generated-module
exports match the pinned response. Keep full-lock provenance, npm consumption,
strict TS/Rust alignment and minimal features explicitly separate.

The restoration tool refuses existing destinations/evidence, path overlap and
unexpected frozen-source drift. Copy only baseline-listed source files; share
installed dependencies through explicit links, redirecting local schema and
telemetry packages to the candidate. Do not install packages, execute online
model generators, inherit credentials/proxy variables, or silently clean up a
failed candidate. Windows links use CPython's native junction primitive rather
than a shell command.

## Current result

The initial, native-link, path-validated and review-hardened candidates all
restore 42 providers / 1712 models, with identical dist/data output bytes. All 255 source/output pairs, generated data and emitted catalog runtime values
validate. Source and generated-tree identities in the supplied snapshot are
unchanged, and its direct-dependency/material inspection is unchanged.

Nine Python tests and two Node tests pass, including subprocess negatives,
failed-build evidence retention, source-drift detection, nested dependency
selection, literal shell-character directory names and dangling-junction refusal.
Independent review findings were addressed: fail early on all existing link
objects, and explicitly scope the immutability evidence rather than claiming all
shared dependency files are monitored. The owning report records checks and limits.

## Evidence and limits

[Restoration report](../phase5-isolated-catalog-restoration.md),
[initial candidate](../evidence/phase5-restored-catalog-candidate.json),
[native-link candidate](../evidence/phase5-restored-catalog-native-candidate.json),
[path-validated candidate](../evidence/phase5-restored-catalog-validated-candidate.json),
[review-hardened final candidate](../evidence/phase5-restored-catalog-reviewed-candidate.json).

This is not a replacement material manifest for the lifecycle runner. The
original tree still has its recorded catalog mismatch; the isolated candidate
removes that mismatch but still carries the changed root lock. Existing installed
transitive dependencies are not re-established from the frozen lock. No Cargo
build, upstream npm package consumer, new TS/Rust differential, or full feature/
platform/MSRV/resource acceptance is claimed.

Test impact: `add` restoration and runtime-export tests; `update` none;
`retain` existing inspector/freezer and product regressions; `delete` none.
