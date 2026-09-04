# Phase 18 Audit

**Audit run ID**: `phase18-codex-gpt56-92ee1d9-20260904t000000z`
**Audit head**: `92ee1d9ddaa23bbc5b55455d3c76b0b4dd2e6995`
**Reviewer ID**: `codex`
**Model ID**: `gpt56`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt-5.6-sol`
**Model identity source**: request-config
**Independence**: fresh-context-same-family; sealed export excluded every assurance directory at archive creation.
**Baseline policy**: latest-committed-spec
**Verdict**: PASS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | current committed source; registered hashes match for both specs |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | current committed source; registered hashes match for both specs |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | current committed source; registered hashes match for both specs |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | current committed source; registered hashes match for both specs |


## Requirement Conformance

149 independently decidable records are in `audit.codex.gpt56.requirements.jsonl`: all 131 valid `P18-*` IDs from the registered supplemental specification plus 18 individually sealed unnumbered risk thresholds. Set difference between registered IDs and sealed IDs was empty. Every mandatory record is met at `92ee1d9ddaa23bbc5b55455d3c76b0b4dd2e6995`; no record relies on the historical `phase_exit.criteria_trace` alone.

## Standards Review

The Independent Companion remains unpublished, outside `[workspace.dependencies]`, and has no Opi crate dependency or reverse product dependency. The Rust surface remains a small provisional entry seam; fallible boundaries use typed errors, external commands use structured argv/environment, and process supervision is platform-specific with explicit Windows limitations. Workspace fmt and clippy pass.

## Spec Review

The assembled crate covers frozen experiment identity, real-process Opi/pi adapters, three pinned benchmark families, durable lifecycle and bundle sealing, integrity/pairing, trajectory projection, offline regrade/report, and the generic N-subject seam. Current `opi-eval` domain names and paths contain no active project-phase naming. Historical `P18-*` identities remain confined to registered delivery/snapshot evidence.

## Security, Invariants, Integration, Test Quality, and Residuals

Both manually dispatched workflows are read-only, candidate-SHA bound, action-pin checked, fail-closed, and statically verified. Canary, path/symlink, digest mutation, schema drift, missing terminal, output bound, trial-identity reuse, authority-stop, and no-fallback tests observe production paths. On Windows, Unix-only integration binaries report zero tests and were not treated as universal evidence; Linux native behavior is supported by the committed digest-bound receipt and producer verifier, not by this Windows run. No actionable residual, compatibility alias, phase-named active path, or duplicate execution mechanism was found.

The full `scripts/opi-doc-check.py` cannot run faithfully in an `Expand-Archive` export on Windows because committed Git symlinks (`CLAUDE.md` and `.claude`) are materialized as placeholder text rather than links. Its failures were recorded as an export-environment limitation, not as passing evidence. The focused naming checker showed no active naming violation, and its 26 unit tests passed.

## Minimum-change Conformance

All Phase 18 ledger tasks retain `reuse_search`, `surface_necessity`, and `simplification_ceiling` traces. The current implementation has one production consumer path through the same-package CLI and integration tests, no Opi production consumer, no reverse dependency, and coherent deletion/rollback at the workspace-member boundary. Status: conforming; no simplification trigger observed.

## Findings

No current Blocker, Major, Minor, or Info finding survived refutation against production callers, tests, fixtures, locked dependencies, and workflow verifiers.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `cargo fmt --check --all` | PASS | Standards |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | Standards/workspace |
| `cargo test -p opi-eval --all-targets` | PASS with explicit Windows platform limitation | Phase 18 functional records |
| five `crates/opi-eval/scripts/test_*.py` suites | PASS (53 + 15 + 15 + 34 tests plus scripted-provider suite) | workflow/artifact contracts |
| external-lock workflow static verifier | PASS | pin/trigger/permission contract |
| native-smoke workflow static verifier | PASS | pin/producer contract |
| `cargo tree -p opi-eval --all-features --target all --edges normal,build,dev` and inverse tree | PASS; no Opi edge/reverse consumer | placement |
| `cargo test --workspace --all-targets` | ENVIRONMENT-LIMITED: one unrelated Git-object test fails because archive export intentionally has no `.git`; Phase 18 focused tests pass | workspace gate not claimed as PASS |
| `cargo test --workspace --doc` | PASS | documentation compile |
| `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps` | PASS | rustdoc |
| `python scripts/opi-doc-check.py` | ENVIRONMENT-LIMITED: Windows ZIP extraction does not preserve committed symlinks | no PASS claim |

## Verdict Rationale

All 149 mandatory records are met, the focused implementation and workflow checks pass, platform/native claims are scoped to their pinned evidence, and there are no non-Info findings. Mechanical member verdict: PASS.
