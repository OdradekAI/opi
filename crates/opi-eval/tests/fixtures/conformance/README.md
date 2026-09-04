# Conformance fixtures

Synthetic, self-authored fixtures for the `opi-eval conformance` suites.
These are **not** official benchmark or agent bytes: they exist so the two
integration binaries can drive the production conformance facade
hermetically (`EVAL-BMK-009`, `EVAL-AGT-006`).

Rules for everything under this directory:

- Bytes here are synthetic unless the case explicitly routes to a committed
  revision-local fixture under `tests/fixtures/benchmarks/<revision>/` or
  `tests/fixtures/agents/<product>/`; a conformance pass never claims a
  real official-task run, a real native verifier, or a real provider call.
- No network, no credentials, no user-global or project resources.
- The deterministic helper processes that stand in for real agent binaries
  and native verifiers are generated at runtime inside the run root; their
  behavior is selected by `OPI_EVAL_CONFORMANCE_BEHAVIOR` and is bounded by
  construction.
- The local scripted provider fixture is `crates/opi-eval/scripts/scripted-provider.py`
  (schema `opi-eval-scripted-provider/1`), tested by
  `crates/opi-eval/scripts/test_scripted_provider.py`.

## Contents

- `benchmarks/ctrf-unknown-schema.json` — schema-drift bytes for the
  Terminal-Bench 3.0 importer: a valid-JSON CTRF report whose summary
  carries an unknown key, so the closed 8-key summary contract must reject
  it as unsupported instead of parsing it as success. (Terminal-Bench 2.1
  has its own revision-local `ctrf/unknown-schema.json`; DeepSWE uses
  `pier-report/drift.json`.)

The native smoke reruns these suites through the committed exact executables and
replaces stand-ins where real pinned bytes are admitted.
