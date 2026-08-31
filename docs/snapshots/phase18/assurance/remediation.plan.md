# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `325a4665c863139394767d3ce3454e79528b77d0301a2dad621c7330761c987c`
**Remediation head**: `fe6501a389031aa15252f1931b6dbcfe86c4434a`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged: none; unstaged: none; untracked: none
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-001` | Confirmed by an archive-only Windows junction probe; `fe6501a` changes only fixed remediation artifacts, so the production failure remains current. | Major -> Major: ancestor aliasing widens bundle write authority. | `bundle.artifact-ancestor-containment` / `bundle.filesystem-boundary` | B1 | `fix:reject-bundle-ancestor-aliases` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-002` | Confirmed by source proof: `atomic_write` syncs the temporary file, renames, and returns without a containing-directory durability barrier. | Major -> Major: durable proof can precede crash-durable namespace publication. | `bundle.intent-directory-durability` / `bundle.durability` | B2 | `fix:durably-sync-parent-after-publication` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-003` | Confirmed by an archive-only saved-trace mutation: a mixed-run graph was accepted as complete. | Major -> Major: invalid Phase 17 evidence crosses the adapter boundary as complete. | `agent.opi-phase17-evidence-graph` / `agent.opi-import` | B3 | `fix:validate-complete-phase17-evidence-graph` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-004` | Confirmed by the direct committed Windows run: POSIX `helper-agent.sh` staging ended with unreadable expected output and OS error 2. | Major -> Major: the declared Windows smoke happy path cannot execute. | `runner.hermetic-windows-execution` / `runner.cross-platform-hermetic` | B4 | `fix:generate-native-windows-hermetic-helpers` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-005` | Confirmed by an archive-only real-binary Windows junction probe; the report was created inside the run tree through an outside lexical path. | Major -> Major: report creation bypasses the sealed-run boundary. | `report.output-ancestor-containment` / `report.filesystem-boundary` | B5 | `fix:resolve-report-output-parent-before-containment` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-006` | Confirmed: the matrix binds native candidate `27344e3aaf03d38eaa53c7af19c777efbe9be213`, the terminal receipt binds `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`, and 20 native/producer paths differ at `fe6501a`. Human Authority selected D1 option A on 2026-08-31. | Major -> Major: current implementation lacks same-candidate native and three-platform evidence. | `phase18.current-candidate-external-evidence` / `phase18.exit-evidence-binding` | B7 | `fix:authorize-one-replacement-evidence-cycle` |
| `phase18-pi-glm53-68d74ec-20260830t200548z / P18-AUD-001` | Confirmed under populated `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`: product construction succeeded but the proxy-free test assertion failed. | Minor -> Minor: test-only ambient-environment assumption. | `config-test.ambient-proxy-independence` / `config-test.environment-isolation` | B6 | `fix:remove-proxy-free-assumption-from-test` |

## Unresolved Decisions

none

D1 is resolved by the user's explicit selection of option A. This fixed,
audit-index-bound plan records a remediation-only authority exception: after
B1-B6 and the complete local gate are materialized as one exact candidate,
one replacement Linux x86_64 native-smoke dispatch and one same-candidate
three-platform same-repository pull-request CI run may be executed. The
exception becomes operational only through a later explicit
`mode=apply phase=18 plan_sha256=<exact validator digest>` invocation. It does
not reopen tasks 18.15, 18.16, or 18.16.1; change product meaning; revise a
registered source; modify `.opi-impl-state.json` or the frozen Phase snapshot;
authorize another rerun; or make the earlier artifacts disappear. The old
artifact and receipt remain historical evidence but cease to support a
current-implementation claim.

## Closure Batches

### Batch B1: Reject bundle artifact ancestor aliases

**Closure predicate**: Every existing ancestor below the reserved bundle root
is inspected without following aliases before a target is created; a symlink,
junction, reparse-point alias, or non-directory ancestor rejects insertion and
no byte is written outside the bundle.
**Dependencies**: none
**Verification union**: focused ancestor-alias tests on Windows and Unix;
`cargo test -p opi-eval bundle::tests`; affected-target clippy; documentation
check; `git diff --check`.

#### Fix B1.1: Walk the reserved artifact path without following aliases

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-001`
- **Decision**: `fix:reject-bundle-ancestor-aliases`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/bundle/mod.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Add a private, component-by-component containment check for the
  existing artifact ancestor chain before directory creation or file
  publication. Reject symlinks and Windows reparse-point directory aliases,
  retain the existing normalized key/path rules, and add real filesystem
  regressions without adding a public seam or dependency.
- **Closure predicate**: Unix symlink and Windows junction ancestors fail before
  insertion; ordinary absent directories are created under the canonical
  bundle root and retain byte-stable sealing.
- **Red-before**: `cargo test -p opi-eval insertion_rejects_ancestor_directory_alias -- --nocapture`
  failed in the test-only archive: insertion succeeded through the Windows
  junction. `git diff 5c0642a..fe6501a` contains only the fixed remediation
  artifacts, so this production observation remains valid.
- **Green-after**: The same focused test passes on Windows, its Unix symlink
  counterpart passes, and `cargo test -p opi-eval bundle::tests` passes.

### Batch B2: Make intent directory publication durable

**Closure predicate**: Successful intent publication proves ordered write,
file sync, rename, and containing-directory durability before
`DurableIntentProof` is returned; a durability failure returns an error and
withholds the proof.
**Dependencies**: none
**Verification union**: focused ordering and failure injection;
`cargo test -p opi-eval bundle::tests`; affected-target clippy; documentation
check; `git diff --check`.

#### Fix B2.1: Sync the containing directory after rename

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-002`
- **Decision**: `fix:durably-sync-parent-after-publication`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/bundle/mod.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Extend the private atomic-publication helper with the narrow
  platform-specific containing-directory durability operation after rename,
  propagate its failure, and add an instrumented ordering/failure test. Do not
  add a public seam or dependency.
- **Closure predicate**: A successful publication proves the complete ordered
  durability sequence; an injected directory-durability failure returns an
  error before any durable proof or later authority transition.
- **Red-before**: `rg -n -A 18 "fn atomic_write" crates/opi-eval/src/bundle/mod.rs`
  shows file sync followed by direct `fs::rename` return and no directory
  barrier; `fe6501a` changes no production path.
- **Green-after**: `cargo test -p opi-eval intent_publication_requires_parent_directory_durability -- --nocapture`
  and `cargo test -p opi-eval bundle::tests` pass.

### Batch B3: Enforce the complete Phase 17 Opi evidence graph

**Closure predicate**: An imported Opi trace is complete only when every
record has one manifest-bound run, strictly increasing sequence, stable call
identity, valid earlier non-self parent, producer-equivalent kind/payload
pairing, and exact terminal run/turn/call/parent/sequence correlation.
**Dependencies**: none
**Verification union**: table-driven saved-trace adversaries; Opi adapter unit
tests; `cargo test -p opi-eval --test agent_integration_conformance`;
affected-target clippy; documentation check; `git diff --check`.

#### Fix B3.1: Validate graph correlation before accepting completion

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-003`
- **Decision**: `fix:validate-complete-phase17-evidence-graph`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/agent/opi.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Parse the private saved schema into the minimum typed correlation
  view, mirror the current producer's closed graph invariants including
  retry/diagnostic payload rules and Artifact admission, compare complete
  terminal correlation with the manifest, and add one table-driven adversary
  for every rejected graph class.
- **Closure predicate**: Mixed-run, non-increasing sequence, unstable call,
  missing/late/self-parent, kind/payload mismatch, and terminal-correlation
  mutations settle as typed import failures; the exact complete fixture still
  succeeds.
- **Red-before**: `cargo test -p opi-eval importer_rejects_phase17_invalid_evidence_graphs -- --nocapture`
  failed in the test-only archive because a mixed-run trace was accepted;
  `fe6501a` changes no production path.
- **Green-after**: The same table-driven test, all Opi importer unit tests, and
  `cargo test -p opi-eval --test agent_integration_conformance` pass.

### Batch B4: Execute hermetic helpers natively on Windows

**Closure predicate**: The production hermetic runner stages and executes
behavior-equivalent agent and verifier helpers for the host platform, and the
committed PowerShell smoke happy/failure/offline paths complete with their
declared exits and artifacts.
**Dependencies**: B1, B2, B3
**Verification union**: direct Windows happy run;
`python scripts/test_phase18_eval_smoke.py`; assembled-run focused tests on
supported hosts; affected-target clippy; documentation check; `git diff --check`.

#### Fix B4.1: Generate host-native bounded helper programs

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-004`
- **Decision**: `fix:generate-native-windows-hermetic-helpers`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Keep the Unix shell helpers, add exact Windows command helpers
  for the same pinned behaviors and argv guards, select the host-native
  suffix/content at staging, and cover both Agent products plus verifier
  success/failure without a new public CLI, dependency, or live-provider path.
- **Closure predicate**: A Windows `happy` run produces readable `answer.txt`,
  complete Opi/pi native evidence, sealed bundles, and a published report;
  declared crash, timeout, and verifier-failure behaviors retain their typed
  outcomes.
- **Red-before**: `cargo run -q -p opi-eval -- run --config crates/opi-eval/tests/fixtures/experiment/phase18-local.toml --root <unique-temp> --fixtures crates/opi-eval/tests/fixtures --behavior happy`
  exited 1 on Windows with `expected agent output is unreadable` / OS error 2;
  `fe6501a` changes no production path.
- **Green-after**: `python scripts/test_phase18_eval_smoke.py` passes on Windows
  and the existing Unix assembled smoke remains green.

### Batch B5: Resolve report-output containment before creation

**Closure predicate**: The existing output parent is resolved to its canonical
location before containment is checked, and the final create-new target cannot
land within the canonical run root through a symlink, junction, or other
ancestor alias.
**Dependencies**: B4
**Verification union**: cross-platform subprocess containment test; report
contract suite; Windows smoke; affected-target clippy; documentation check;
`git diff --check`.

#### Fix B5.1: Canonicalize the output parent and reject aliases into the run

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-005`
- **Decision**: `fix:resolve-report-output-parent-before-containment`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/cli/report.rs`, `crates/opi-eval/tests/report_output_containment.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Require and canonicalize the existing output parent, join only
  the final absent filename to the resolved parent, compare that target with
  the canonical run root, preserve create-new behavior, and add a real-binary
  Unix-symlink/Windows-junction regression proving no in-root file appears.
- **Closure predicate**: Direct, relative, symlinked, and junction-aliased
  targets that resolve inside the run root exit 2 without creating bytes; a
  fresh target under a resolved external parent succeeds exactly once.
- **Red-before**: `cargo test -p opi-eval output_rejects_ancestor_alias_into_run_root -- --nocapture`
  failed in the test-only archive because `open_output` created the file through
  the junction; `fe6501a` changes no production path.
- **Green-after**: `cargo test -p opi-eval --test report_output_containment`
  passes on Unix and Windows and `cargo test -p opi-eval --test report_contract`
  passes on supported hosts.

### Batch B6: Remove the proxy-free assumption from the config test

**Closure predicate**: The no-explicit-proxy test verifies successful client
construction without asserting that ambient proxy policy is absent, and it
passes with or without ambient proxy variables.
**Dependencies**: none
**Verification union**: focused test with ambient proxy variables; complete
`config_tests` binary; affected test-target clippy; `git diff --check`.

#### Fix B6.1: Assert the product contract instead of ambient state

- **Finding source(s)**: `phase18-pi-glm53-68d74ec-20260830t200548z + 7ea0bd6518d4169f1e597a6d52588038bae9fc1d74a64c5f4d4600c30570da37 + P18-AUD-001`
- **Decision**: `fix:remove-proxy-free-assumption-from-test`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/config_tests.rs`
- **Change kind**: test-only
- **Change**: Rename the test to distinguish absence of explicit
  configuration from absence of ambient proxy policy and remove only the
  invalid `proxy_config().url.is_none()` assertion. Retain product behavior
  and all explicit-proxy tests.
- **Closure predicate**: Client construction without an explicit proxy
  succeeds under clean and proxy-populated environments, while explicit
  invalid proxy configuration remains rejected.
- **Red-before**: `$env:HTTP_PROXY='http://127.0.0.1:19828'; $env:HTTPS_PROXY='http://127.0.0.1:19828'; $env:ALL_PROXY='http://127.0.0.1:19828'; cargo test -p opi-coding-agent --test config_tests build_http_client_without_proxy_succeeds -- --exact --nocapture`
  failed at the proxy-URL-is-none assertion; `fe6501a` changes no product or
  test path.
- **Green-after**: With the same variables,
  `cargo test -p opi-coding-agent --test config_tests build_http_client_without_explicit_proxy_succeeds -- --exact --nocapture`
  and `cargo test -p opi-coding-agent --test config_tests` pass.

### Batch B7: Rebind Phase 18 external evidence to one replacement candidate

**Closure predicate**: One exact materialized candidate containing B1-B6 and
the approved plan passes the full local gate, one replacement Linux x86_64
native-smoke artifact and one same-repository three-platform PR CI run both
bind that identical full SHA, the regenerated seam matrix and terminal receipt
retain those identities, and any later evidence commit changes no runtime,
producer, verifier, or workflow byte.
**Dependencies**: B1, B2, B3, B4, B5, B6 and the complete local verification
union below
**Verification union**: native verifier contract tests; matrix derivation
contract tests; terminal CI verifier tests; exact replacement native artifact
verification; matrix derivation with `--verify`; same-candidate terminal CI
verification; candidate-to-evidence-commit path-drift proof; documentation
check; `git diff --check`.

#### Fix B7.1: Execute the one-time, same-candidate replacement evidence cycle

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-006`
- **Decision**: `fix:authorize-one-replacement-evidence-cycle`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/docs/seam-evidence-matrix.md`, `docs/snapshots/phase18/ci-receipt.json`
- **Change kind**: metadata
- **Change**: After local closure, materialize exactly one candidate commit and
  bind `$replacementCandidate` to its full `git rev-parse HEAD`. With separate
  explicit authorization for commit/push/workflow/PR operations, publish that
  unchanged candidate, dispatch `.github/workflows/phase18-native-smoke.yml`
  once with `candidate_sha=$replacementCandidate`, verify and download its
  receipt/artifact, derive the matrix, and run ordinary same-repository PR CI
  with the candidate as the PR head. Verify terminal metadata and write the
  replacement receipt. Do not amend or advance the candidate until both
  external runs have captured it. Commit the two evidence files only after
  both verifiers pass, then prove that this evidence-only commit has no drift
  in the candidate's runtime, producer, verifier, or workflow paths.
- **Closure predicate**: The native verifier, matrix derivation, and terminal
  verifier accept one identical 40-hex candidate; the matrix and receipt name
  it; all required jobs succeed with ordinary merge-ref semantics; and the
  final candidate-to-evidence diff is limited to the approved evidence and
  remediation-result paths.
- **Red-before**: `python -c "import json,pathlib,subprocess; h=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(); m=pathlib.Path('crates/opi-eval/docs/seam-evidence-matrix.md').read_text(encoding='utf-8'); n=next(x.split(chr(96))[1] for x in m.splitlines() if x.startswith('| candidate_commit |')); c=json.loads(pathlib.Path('docs/snapshots/phase18/ci-receipt.json').read_text(encoding='utf-8'))['candidate_head']; assert n==h and c==h,(h,n,c)"`
  fails with `fe6501a`, `27344e3`, and `0f5a3fa`; the native candidate also
  differs from 20 current native/producer paths.
- **Green-after**: Set exact paths returned by the single authorized runs and
  execute:

      $replacementCandidate = (git rev-parse HEAD).Trim()
      python scripts/verify-phase18-native-artifact.py --criterion all-native --expected-commit $replacementCandidate --receipt $nativeReceipt --artifact $nativeArtifact --repo .
      python scripts/derive-phase18-seam-matrix.py --receipt $nativeReceipt --artifact $nativeArtifact --require-trajectory-spans --output crates/opi-eval/docs/seam-evidence-matrix.md --verify --repo . --expected-commit $replacementCandidate
      python scripts/verify-phase18-ci.py --terminal --expected-head $replacementCandidate --run-metadata $runMetadata --jobs-metadata $jobsMetadata --artifact-metadata $artifactMetadata --inner-receipt $innerReceipt --output docs/snapshots/phase18/ci-receipt.json --repo .

  Each command must pass. After the evidence-only commit, parse both committed
  outputs to assert their candidate is `$replacementCandidate`, and run
  `git diff --exit-code $replacementCandidate..HEAD -- .github/workflows/ci.yml .github/workflows/phase18-native-smoke.yml crates/opi-eval/src crates/opi-coding-agent/src scripts/phase18-native-smoke.sh scripts/phase18-build-agent-artifacts.sh scripts/phase18-scripted-provider.py scripts/verify-phase18-native-artifact.py scripts/verify-phase18-ci.py`
  with exit 0.

## Final Verification

Before materializing the replacement candidate, run the deduplicated local
union in this order:

    cargo test -p opi-eval insertion_rejects_ancestor_directory_alias -- --nocapture
    cargo test -p opi-eval intent_publication_requires_parent_directory_durability -- --nocapture
    cargo test -p opi-eval importer_rejects_phase17_invalid_evidence_graphs -- --nocapture
    cargo test -p opi-eval bundle::tests
    cargo test -p opi-eval importer
    cargo test -p opi-eval --test agent_integration_conformance
    cargo test -p opi-eval --test report_output_containment
    cargo test -p opi-eval --test report_contract
    python scripts/test_phase18_eval_smoke.py
    cargo test -p opi-coding-agent --test config_tests
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/test_derive_phase18_seam_matrix.py
    python scripts/test_verify_phase18_ci.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
    python scripts/opi-doc-check.py
    git diff --check

Then materialize one exact candidate, preserve it unchanged through the single
native dispatch and same-candidate PR CI, execute B7's exact external
verification commands, install only the derived matrix and terminal receipt,
and rerun:

    python scripts/opi-doc-check.py
    git diff --check

Commit, push, workflow dispatch, PR creation/update, metadata download, and
external execution are materialization boundaries. This plan defines their
identities and one-run ceiling but does not perform or pre-authorize Git or
remote writes; execution must stop for explicit user authorization at those
boundaries. No result may declare B7 closed or Phase 18 current-head conformant
until both external verifiers and the candidate-to-evidence drift proof pass.

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| none | none | All 7 current source identities are confirmed and assigned exactly once to B1-B7. |
