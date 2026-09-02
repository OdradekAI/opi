# opi-eval Domain Naming Design

Status: approved design awaiting implementation planning

Authority: non-normative implementation design. `docs/opi-spec.md` remains the
normative source for product direction and architecture invariants.

## Goal

Remove project-management phase terminology from the active `opi-eval` module
and its live repository integrations. Retain current evaluation capabilities
under domain names, retire delivery-only Phase 18 assets, and leave completed
delivery history in the repository's historical sources of truth.

The resulting active module must be understandable as an Independent Companion
without knowing the implementation phase that created it.

## Design principles

`opi-eval` is a module whose interface includes more than Rust types and CLI
commands. Configuration schemas, diagnostic codes, fixture identities,
workflow names, artifact names, script entry points, failure behavior, and
verification commands are also part of the caller-facing interface.

The design applies the deep-module deletion test:

- Keep a file when deleting it would remove a current capability or force its
  verification logic into workflows or callers.
- Retire a file when it only proves a completed delivery step and has no
  current caller or independently useful interface.
- Do not add aliases or compatibility shims for unpublished 0.x contracts.
- Keep completed Phase 18 records in `docs/snapshots/phase18/`, the
  implementation ledger, registered historical material, and Git history.
  Those records are not active `opi-eval` module interfaces and are not
  rewritten by this change.

## Scope

The active migration covers:

- `crates/opi-eval/**`;
- live `opi-eval` workflows under `.github/workflows/`;
- the `opi-eval` attestation job in `.github/workflows/ci.yml`;
- root and crate README descriptions, Cargo metadata, `.gitattributes`, current
  Unreleased changelog references, and source-derived documentation checks;
- every active caller, test, fixture, lock binding, digest, and command that
  names a migrated interface.

The migration does not edit:

- `docs/snapshots/phase18/**`;
- `.opi-impl-state.json` or its archived copies;
- completed research, realignment, audit, remediation, or design evidence;
- released changelog sections or Git history.

## Retention decisions

### Retain as current capabilities

The following files hide substantial behavior behind narrow script or test
interfaces and therefore earn their maintenance cost:

| Current responsibility | Domain-named interface |
|---|---|
| Materialize pinned external inputs | `scripts/materialize-external-locks.sh` |
| Verify the external-lock workflow contract | `scripts/verify-external-lock-ci.py` |
| Verify a downloaded materialization artifact | `scripts/verify-external-lock-artifact.py` |
| Produce the native cross-Agent smoke artifact | `scripts/native-smoke.sh` |
| Verify the native-smoke workflow contract | `scripts/verify-native-smoke-ci.py` |
| Verify a downloaded native-smoke artifact | `scripts/verify-native-smoke-artifact.py` |
| Build and identify evaluated Agent executables | `scripts/build-agent-artifacts.sh` |
| Provide deterministic local model responses | `scripts/scripted-provider.py` |
| Exercise assembled run behavior through the CLI | `tests/assembled_run.rs` |

Their tests remain colocated in `crates/opi-eval/scripts/` and use matching
domain names, for example `test_verify_external_lock_ci.py` and
`test_verify_native_smoke_artifact.py`.

The two manually dispatched workflows remain current operational interfaces:

- `.github/workflows/opi-eval-external-lock-materialization.yml`;
- `.github/workflows/opi-eval-native-smoke.yml`.

### Retire as completed delivery machinery

The following assets have no continuing caller or duplicate durable behavior
covered at a deeper interface:

- the Minimal Runtime pre-delivery baseline capture script, its Python test,
  its `pre-phase18` evidence fixture, and the historical replay assertions in
  `tests/phase18_acceptance.rs`;
- the one-time seam-evidence matrix derivation script, its Python test, and the
  active copy of `docs/seam-evidence-matrix.md`;
- the Phase 18 pull-request attestation verifier, its Python test, the
  `phase18_attestation` CI job, and its active receipt/artifact names;
- the unreferenced POSIX and PowerShell eval-smoke wrappers and their Python
  test, because `tests/assembled_run.rs` exercises the production run seam;
- `tests/rollback_contract.rs`, which asserts completed delivery non-goals
  rather than current `opi-eval` behavior;
- the historical resolved external lock and real materialization receipt
  fixture, which have no production consumer and remain represented by the
  completed Phase 18 snapshot and Git history.

The generic, Agent-neutral experiment resolution assertion from
`tests/phase18_acceptance.rs` remains valuable. It moves into
`tests/experiment_contract.rs` before the phase acceptance file is removed.

## Path and fixture naming

Retained paths use responsibilities, not chronology:

| Old path | New path |
|---|---|
| `scripts/phase18-materialize-locks.sh` | `scripts/materialize-external-locks.sh` |
| `scripts/verify-phase18-materialization-ci.py` | `scripts/verify-external-lock-ci.py` |
| `scripts/verify-phase18-materialization-artifact.py` | `scripts/verify-external-lock-artifact.py` |
| `scripts/phase18-native-smoke.sh` | `scripts/native-smoke.sh` |
| `scripts/verify-phase18-native-ci.py` | `scripts/verify-native-smoke-ci.py` |
| `scripts/verify-phase18-native-artifact.py` | `scripts/verify-native-smoke-artifact.py` |
| `scripts/phase18-build-agent-artifacts.sh` | `scripts/build-agent-artifacts.sh` |
| `scripts/phase18-scripted-provider.py` | `scripts/scripted-provider.py` |
| `scripts/fixtures/phase18-native-ci/` | `scripts/fixtures/native-smoke-ci/` |
| `tests/phase18_assembled_smoke.rs` | `tests/assembled_run.rs` |

Experiment fixtures use behavior or benchmark identities:

| Old fixture | New fixture |
|---|---|
| `phase18-local.toml` | `local-paired.toml` |
| `phase18-deepswe.toml` | `deepswe.toml` |
| `phase18-tb30.toml` | `terminal-bench-3.0.toml` |
| `phase18-multi-edge.toml` | `multi-edge.toml` |
| `phase18-duplicate-pair.toml` | `duplicate-pair.toml` |
| `phase18-integrity-exclusion.toml` | `integrity-exclusion.toml` |
| `phase18-invalid-task.toml` | `invalid-task.toml` |
| `phase18-unsupported-control.toml` | `unsupported-control.toml` |

Test functions lose `p18_` and task-number prefixes. Each name describes the
observable behavior it verifies.

## Interface identity migration

Path-only renaming would leave project chronology in the module interface.
Therefore all active interface identities migrate in the same change:

- schema prefixes change from `phase18-` to `opi-eval-`;
- diagnostic and invariant codes change from `P18-` to `EVAL-`;
- experiment IDs, model IDs, lock IDs, artifact names, temporary-directory
  prefixes, workflow job names, concurrency groups, and environment variables
  receive domain names;
- current comments, rustdoc, README text, and Cargo descriptions state the
  present contract rather than the delivery phase or task that introduced it.

Representative schema mappings are:

| Old identity | New identity |
|---|---|
| `phase18-experiment/1` | `opi-eval-experiment/1` |
| `phase18-agent-profile/1` | `opi-eval-agent-profile/1` |
| `phase18-benchmark-profile/1` | `opi-eval-benchmark-profile/1` |
| `phase18-run-report/1` | `opi-eval-run-report/1` |
| `phase18-conformance-report/1` | `opi-eval-conformance-report/1` |
| `phase18-native-material/1` | `opi-eval-native-material/1` |
| `phase18-external-lock/static/1` | `opi-eval-external-lock/static/1` |
| `phase18-external-lock/resolved/1` | `opi-eval-external-lock/resolved/1` |

The same prefix rule applies to active receipt, bundle, report, trajectory,
provider-log, artifact-manifest, and upload-identity schemas.

This is a deliberate breaking change to unpublished 0.x contracts. Every
repository caller changes atomically. Old schemas and paths fail closed as
unknown inputs; there is no alias, dual-read path, or migration shim. The
breaking change is recorded under `CHANGELOG.md` `## [Unreleased]`.

## Evidence and data flow

The retained external-lock flow is:

```text
static lock
  -> workflow/static-contract verifier
  -> materializer
  -> downloaded artifact verifier
  -> resolved lock produced for the new domain-named contract
```

The retained native-smoke flow is:

```text
workflow dispatch
  -> native producer contract
  -> Agent builder + scripted provider
  -> assembled trials and native graders
  -> sealed artifact
  -> downloaded artifact verifier
```

Renaming any producer, workflow, or verifier changes the bound bytes. The
static lock and synthetic fixtures must therefore be regenerated from the new
paths and exact SHA-256 values. Historical Phase 18 artifacts are not rewritten
to claim they were produced by the renamed files; they leave the active module
and remain historical evidence.

## Failure behavior

All existing fail-closed behavior remains:

- a workflow or producer path/digest mismatch rejects before materialization
  or Agent execution;
- an unknown old schema rejects rather than silently upgrading;
- missing, expired, malformed, or identity-mismatched artifacts reject;
- renamed fixtures cannot fall back to old paths;
- generated locks and receipts must contain only the new canonical paths and
  identities.

No behavioral fallback is introduced by the naming migration.

## Verification

Success requires all of the following observable results:

1. A case-insensitive scan of `crates/opi-eval` finds no `phase18`, `phase_18`,
   `phase-18`, `P18-`, or `p18_` token in active paths or file contents.
2. The two live workflows and `.github/workflows/ci.yml` contain no active
   Phase 18 job, path, environment, artifact, or schema identity.
3. Every retained Python test file passes by exact path, and representative
   negative-path CLI tests still reject drifted schemas, paths, and digests.
4. Every retained shell script passes `bash -n`; the PowerShell wrapper removal
   leaves no PowerShell entry point to validate.
5. The domain-named native and external-lock static verifiers accept their live
   workflows and reject deliberately drifted fixtures.
6. `cargo test -p opi-eval --all-targets` passes, including the moved generic
   experiment contract and renamed assembled-run test.
7. `cargo clippy -p opi-eval --all-targets -- -D warnings`,
   `cargo fmt --check --all`, `python scripts/opi-doc-check.py`, and
   `git diff --check` pass.
8. Static lock producer/workflow digests match the exact renamed bytes.
9. The final worktree inventory distinguishes task changes from any carried-in
   user changes, and no historical snapshot or implementation ledger is
   modified.

Test impact is `update` and `delete`: current behavior tests are renamed or
moved, completed delivery tests are deleted, and no unrelated test surface is
changed.
