**Status**: COMPLETE
**Audit index SHA-256**: `e10b49f77361516f750166624f02ec0bb508be4018ac93efaef30297d871baa0`
**Plan SHA-256**: `40120473d670dc437082d655c989058fb5da70393da8bbe872920e4b50045779`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/src/agent/process.rs", "crates/opi-eval/src/authority.rs", "crates/opi-eval/src/benchmark/process.rs", "crates/opi-eval/src/bundle/mod.rs", "crates/opi-eval/src/cli/report.rs", "crates/opi-eval/src/comparison.rs", "crates/opi-eval/src/integrity.rs", "crates/opi-eval/src/process.rs", "crates/opi-eval/src/process/tree.rs", "crates/opi-eval/src/regrade.rs", "crates/opi-eval/src/report.rs", "crates/opi-eval/src/runner/experiment.rs", "crates/opi-eval/src/trajectory/mod.rs", "crates/opi-eval/tests/authority_boundaries.rs", "crates/opi-eval/tests/bundle_recompute.rs", "crates/opi-eval/tests/end_to_end_report.rs", "crates/opi-eval/tests/fixtures/experiment/phase18-multi-edge.toml", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/phase18_assembled_smoke.rs", "crates/opi-eval/tests/report_contract.rs", "scripts/phase18-native-smoke.sh", "scripts/test_verify_phase18_native_ci.py"]

Apply bound to remediation head `b9af27fd7944b00566d8dd7443936d9a5031f0e2`,
approved plan digest `40120473d670dc437082d655c989058fb5da70393da8bbe872920e4b50045779`,
and index digest `e10b49f77361516f750166624f02ec0bb508be4018ac93efaef30297d871baa0`.
Every red-before predicate was re-observed failing in this apply run before
production edits. All ten source findings are Closed; one bounded
incidental repair (I1) was accepted under batch B1.

## Closure batch outcomes

- **B1 `bundle.retained-byte-closure` - Closed.** The pre-effect intent
  reservation now names the complete artifact closure (control evidence,
  trajectory, normalized expected output, agent streams, answer, authority
  ledger); sealing enforces staged == reserved plus the declared produced
  native evidence and requires the expected output; verification compares
  the manifest intent/settlement with the durable sidecars, requires every
  reserved artifact among the entries, enumerates the artifact tree so
  unmanifested, missing, non-file, and digest-mismatched entries all fail
  at `TrialDurability`, and stays read-only and byte-stable. Agent and
  verifier artifact reads fail closed.
- **B2 `report.sealed-input-and-output-isolation` - Closed.** The offline
  report reconstructs trial views, coverage, integrity provenance,
  rewards, and diagnostics from verified sealed bundles only; a covered
  byte mutation or sealed-input parse failure returns the typed
  `verification-failed` outcome with a non-zero exit; `--out` is opened
  create-new outside the run root.
- **B3 `agent-failure.native-graded-outcome` - Closed.** A closed failure
  classification keeps the Agent's own non-zero exit and budget timeout in
  the graded Agent outcome class (one native grade dispatch, comparable
  `AgentFailure` pairing in the denominator) while spawn, cancellation,
  adapter/evidence, and infrastructure failures retain their mechanical
  stops; the ledger seals the observed completion class.
- **B4 `process.natural-exit-tree-cleanup` - Closed.** Natural exit tracks
  per-stream EOF, probes the tree guard, and terminates/verifies
  descendants and inherited-pipe holders; `NotRequired` is reported only
  after observed tree emptiness.
- **B5 `deepswe.reward-domain-and-positive-oracle` - Closed.** Every Pier
  reward is domain-checked in `0..=1` before any `u64` conversion, and the
  DeepSWE oracle preflight requires an explicitly known positive native
  reward.
- **B6 `trial.intent-edge-identity` - Closed.** Each trial's durable
  `PairIdentity` is its unique owning edge; zero or multiple owners reject
  before any process effect.
- **B7 `verifier-artifact.source-identity` - Closed.** Verifier streams
  and native reports carry the pinned grader source; the offline headline
  requires the grader source and Native role together.
- **B8 `native-conformance.receipt-count` - Closed.** The conformance
  receipt count derives from the successful-case loop counter (13 at
  runtime), and the CI contract test binds the declared case list to the
  counter path.

## Incidental repairs

- **I1 (B1)**: the planned `BundleError` extension made the shared offline
  regrade classifier in `crates/opi-eval/src/regrade.rs` non-exhaustive,
  failing compilation of B1's verification command. The minimal repair
  maps the five new variants to typed regrade tokens; all guardrails hold
  (no public API, durable format, dependency, spec, authority, manifest,
  ledger, or schema change).

## Verification

Applied in dependency order with each batch's verification union green at
its boundary (B1 -> B3 -> B4 -> B5 -> B6 -> B7 -> B8 -> B2). Final union,
all run at the final tree:

    cargo test -p opi-eval --lib                          # 144 passed
    cargo test -p opi-eval --test bundle_recompute        # 4 passed
    cargo test -p opi-eval --test end_to_end_report       # 5 passed
    cargo test -p opi-eval --test report_contract         # 3 passed
    cargo test -p opi-eval --test authority_boundaries    # 4 passed
    cargo test -p opi-eval --test phase18_assembled_smoke # 9 passed
    cargo test -p opi-eval --test experiment_contract     # 14 passed
    cargo test -p opi-eval --test native_driver           # 8 passed
    python scripts/test_verify_phase18_native_ci.py       # 34 tests OK
    python scripts/test_verify_phase18_native_artifact.py # 15 tests OK
    python scripts/opi-doc-check.py                       # PASS
    cargo fmt --check --all                               # clean
    cargo clippy --workspace --all-targets -- -D warnings # 0 errors
    cargo test --workspace --all-targets                  # see note
    cargo test --workspace --doc                          # 8 suites OK
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps # PASS

Notes:

- `cargo test --workspace --all-targets`: 131 suites pass. One
  `opi-coding-agent` failure (`build_http_client_without_proxy_succeeds`)
  is environmental - this session exports `HTTP_PROXY`/`HTTPS_PROXY`, and
  the test passes with the proxy variables cleared. Two to three
  `opi-coding-agent` `oauth_auth` PKCE deadline tests fail
  non-deterministically run-to-run on this host; the changed surface of
  this remediation contains no `opi-coding-agent` file or any crate it
  depends on, and the failing set varies between identical runs. Both are
  reported as non-blocking observations outside the approved scope and
  were not fixed.
- The original B8 red-before command compared the executed case list with
  a `cases_run` literal in the script text; the approved change removes
  that second independent literal by design, so the literal-regex form no
  longer matches. The green observation is carried by the extended script
  contract test plus the derived counter path, per the approved change.

## Materialization boundary

Fixes plus the current live audit set are not yet committed. A fresh audit
or reviewer re-run may be requested only after the fixes, the fixed
remediation plan/result group, and the current live set are committed and
the assurance directory is clean. No Phase conformance is claimed and no
other skill was invoked.
