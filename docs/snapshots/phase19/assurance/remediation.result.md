# Phase 19 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `fe9e6af9cd26f69556c0224f219500626181ae2829d7099af99500e73faff5db`
**Plan SHA-256**: `db42006b85fd392267c76dcf7362bb1d1478e369d25f353a5e4ec4f4fcd88d24`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/scripts/test_verify_local_acceptance.py", "crates/opi-eval/scripts/verify-local-acceptance.py", "crates/opi-eval/src/agent/pi.rs", "crates/opi-eval/src/cli/prepare.rs", "crates/opi-eval/src/experiment.rs", "crates/opi-eval/src/native_capture_runtime.rs", "crates/opi-eval/src/regrade.rs", "crates/opi-eval/src/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/src/runner/plan.rs", "crates/opi-eval/src/runner/prepare.rs", "crates/opi-eval/tests/agent_integration_conformance.rs", "crates/opi-eval/tests/assembled_run.rs", "crates/opi-eval/tests/bundle_recompute.rs", "crates/opi-eval/tests/experiment_contract.rs", "crates/opi-eval/tests/fail_closed_cli.rs", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/report_output_containment.rs", "crates/opi-eval/tests/support/native_witness_material.rs"]

## Summary

All ten approved closure batches executed in dependency order with the
planned minimum changes. Every batch's discriminating green-after check
was observed against the final repaired build
(`opi-eval` sha256 `8e483c6f8535a5e1924b6f39e531010657431e5cb144c6c7c76dd633584f81ac`)
and the final repaired source; the recorded observations live in the
result dispositions. Ten fix findings closed; the two no-action findings
keep their planned statuses (`Cannot confirm` for the retained final-state
Phase-exit gap, `Info/No action` for the advisory registry navigation
limitation).

One bounded incidental repair (`I1`) was accepted during the B8 final
native evidence: the regrade witness's diagnostic stand-in agent emitted
unlisted stderr and plain stdout that B1's closed capture grammar
correctly refuses; the stand-in now emits no stderr and the recognized
Agent event shape. All guardrails hold (test-support only).

## Final verification union

Executed once after the last invalidating edit, in the repository
working tree with the external Cargo cache:

- Ten matching discriminating probes rerun with the final binary and
  fresh proof directories: privacy 32/32, protocol 8/8,
  quoted-bounds 21/21, explicit-max-trials (exit 2, 0 sealed, no root),
  doctor-stall (bounded 10.1s report), zero-seal (both cases),
  acceptance 6/6, doctor-materials (all seven categories), the B6
  containment regression, and the B8 desktop regression - all PASS.
- `cargo test -p opi-eval --all-targets --locked --offline`: all test
  binaries green (lib 175, native_driver 117 + the B6 containment
  regression, fail_closed_cli 8, agent_integration_conformance 3,
  assembled_run 16, bundle_recompute 7, report_contract 6,
  report_output_containment 14, experiment_contract 35, others green).
- `cargo clippy -p opi-eval --all-targets --locked --offline -- -D warnings`:
  clean.
- `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'`:
  273 tests OK (1 skipped) with the final Companion at the workspace
  target fallback.
- `python3 scripts/opi-doc-check.py`: PASS; `git diff --check`: clean on
  every task-owned path (an unrelated, unowned mid-session rewrite of
  `docs/research/pi-1.0.0/source-manifest.json` and an untracked
  research document are preserved untouched).
- `cargo fmt --check --all`: OK.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`:
  clean.
- `cargo test --workspace --all-targets --locked --offline`: exit 0,
  232/232 test-result lines OK.
- `cargo test --workspace --doc --locked --offline`: exit 0 (8 result
  lines OK).
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --offline`:
  exit 0.
- Exact six-trial raw root re-verified offline with the final Companion:
  accepted=true, six attributable trials, three families.
- B8 native evidence with final identities: Docker engine
  `Docker Desktop (containerized)` (f7640520-9a9f-4acb-bd58-62c5b388982a)
  admitted through the existing namespace observer; three public
  prepares exit 0 publishing `opi-eval-native-material/7`; each config
  and the derived three-family witness manifest validate exit 0; the
  final musl helper (sha256 `28b0d74c0e208fefd63c5576d2020cfa85f4e454c4cab8c147719439c2421b4c`)
  repinned; the six exact native filters execute/pass
  (6 passed, 0 failed, 122 filtered) with no owned residual container or
  process.

## Notes

- The retained warm cache had been wiped externally between planning and
  apply; it was re-seeded from the verified pinned inputs (12 lock
  seeds, the retained Opi client copy) and re-acquired under supervision
  through the pinned materializer (uv, three corpora, harbor/pier
  runners, pi package). Corpus working-tree exec bits were normalized to
  the git-recorded modes after drvfs-staged acquisition, and one
  deferred-uv console-script shebang rewrite of the pier entrypoint was
  restored to its pinned identity before the final re-freeze; content
  digests verified unchanged throughout.
- `cargo test --workspace` initially reused stale export-compiled unit
  binaries from the shared external Cargo cache (a `git`-dependent
  artifact audit binary and one OOM-corrupted unit); the affected units
  were rebuilt from the repository and the complete workspace union then
  ran green. These are cache/workflow effects, not source defects.
- C/P19-AUD-009's final-state Phase-exit gap is retained as planned: the
  exact six-trial root passes the final Companion and verifier, while
  final repaired-state workspace gates are now green; any remaining
  Phase-conformance assertion stays with the owning workflow.

This result claims finding closure and executed native evidence only; it
does not declare the Phase conformant.
