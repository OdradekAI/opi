# Phase 18 Remediation Plan

**Status**: DRAFT-UNRESOLVED
**Audit index SHA-256**: `325a4665c863139394767d3ce3454e79528b77d0301a2dad621c7330761c987c`
**Remediation head**: `5c0642a7e5af51529c18be47311fa17679d44c8b`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]
**Unresolved decisions**: D1

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-001` | Confirmed by a Windows junction probe in the committed archive: `RunBundle::insert` returned success and wrote through `artifacts/native` to an external directory. | Major -> Major. Artifact staging can escape the reserved bundle tree through an existing ancestor alias, widening artifact authority. | `bundle.artifact-ancestor-containment` / `bundle.filesystem-boundary` | B1 | `fix:reject-bundle-ancestor-aliases` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-002` | Confirmed by current-source proof: `atomic_write` synchronizes the temporary file and returns directly from `fs::rename` without a containing-directory durability barrier. | Major -> Major. `DurableIntentProof` can be returned before the renamed directory entry is crash-durable. | `bundle.intent-directory-durability` / `bundle.durability` | B2 | `fix:durably-sync-parent-after-publication` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-003` | Confirmed by a saved-trace mutation probe in the committed archive: replacing the first record's run with a different valid UUID still settled as complete. Current source also omits strict sequence, call, parent, and kind/payload correlation. | Major -> Major. Phase 17-invalid evidence graphs cross the Opi adapter boundary as complete evidence. | `agent.opi-phase17-evidence-graph` / `agent.opi-import` | B3 | `fix:validate-complete-phase17-evidence-graph` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-004` | Confirmed on Windows by the production CLI: the happy hermetic run exits 1 with `expected agent output is unreadable` / OS error 2 after staging `helper-agent.sh`. | Major -> Major. The committed cross-platform wrapper cannot execute its Windows happy path. | `runner.hermetic-windows-execution` / `runner.cross-platform-hermetic` | B4 | `fix:generate-native-windows-hermetic-helpers` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-005` | Confirmed by a Windows junction probe in the committed archive: a lexical `--out` outside the run root resolved through the junction and created `report.json` inside the run root. | Major -> Major. Report publication can add an unmanifested file under sealed input through an ancestor alias. | `report.output-resolved-containment` / `report.filesystem-boundary` | B5 | `fix:resolve-report-output-parent-before-containment` |
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-006` | Confirmed: the native artifact is bound to `27344e3aaf03d38eaa53c7af19c777efbe9be213`, the terminal CI receipt to `0f5a3fa152b12d7be4036b2a08ae7a195f8c2107`, and the remediation head changes twenty native-exercised runtime/producer paths relative to the artifact candidate. | Major -> Major. Current-head real-runtime and three-platform claims are not supported by the recorded artifacts. | `phase18.current-head-runtime-evidence` / `phase18.assurance-binding` | pending D1 | `pending:D1-current-head-evidence-authority` |
| `phase18-pi-glm53-68d74ec-20260830t200548z / P18-AUD-001` | Confirmed: with ambient `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, the focused test fails because the client correctly adopts the ambient proxy while the test asserts no proxy URL. | Minor -> Minor. This is an environment-isolation defect in the test, not a product proxy defect. | `config-test.proxy-environment-isolation` / `tests.environment-isolation` | B6 | `fix:remove-proxy-free-assumption-from-test` |

## Unresolved Decisions

| ID | Required decision | Why evidence cannot decide | Alternatives | Authority needed |
|---|---|---|---|---|
| D1 | Select the registered authority path for current-head Phase 18 native and terminal-CI evidence after B1-B6 are materialized. | The live evidence is stale, but the registered task 18.15 source requires one sole human-authorized native dispatch, task 18.16 consumes it without rerunning native work, and task 18.16.1 owns a terminal receipt with no runtime/test implementation. A remediation run cannot silently revise those authority and execution boundaries. | (A) Revise the registered supplemental source and owning task authority to admit one post-remediation replacement native dispatch plus replacement three-platform terminal receipt, then derive the matrix from those exact artifacts. (B) Retain the sole/no-rerun boundary, return this finding to shaping, and explicitly withdraw current-head Phase-conformance claims while retaining the old artifacts only as historical evidence. | Human owner of the registered Phase 18 supplemental source and implementation-state workflow; use the owning shaping/`opi-implement` route for any source or ledger revision. |

The decision determines the closure predicate and changed paths for
`P18-AUD-006`; no placeholder fix is planned before D1 is resolved.

## Closure Batches

### Batch B1: Keep staged bundle artifacts inside the reserved tree

**Closure predicate**: Every existing ancestor of a logical artifact path is a real directory inside the canonical bundle artifact root; a symlink, Windows junction/reparse alias, non-directory, or resolved escape is rejected before any byte is written.
**Dependencies**: none
**Verification union**: focused bundle ancestor-alias tests on Unix and Windows; `cargo test -p opi-eval bundle::tests`; affected-target clippy; documentation check; `git diff --check`.

#### Fix B1.1: Reject artifact ancestor aliases before staging

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-001`
- **Decision**: `fix:reject-bundle-ancestor-aliases`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/bundle/mod.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Make artifact-path preparation walk and validate each component without following an alias outside the canonical artifact root, fail closed before `atomic_write`, and add Unix-symlink plus Windows-junction negative tests that prove no external byte is created.
- **Closure predicate**: Insertion and later covered reads reject every ancestor alias/escape and leave both the external target and bundle entry map unchanged.
- **Red-before**: `cargo test -p opi-eval insertion_rejects_ancestor_directory_alias -- --nocapture` -> FAIL in the test-only archived probe: insertion returned success through the junction.
- **Green-after**: The same focused ancestor-alias test passes on Windows, its Unix symlink counterpart passes, and `cargo test -p opi-eval bundle::tests` passes.

### Batch B2: Make durable publication include the directory entry

**Closure predicate**: `publish_intent` cannot return `DurableIntentProof` until the temporary bytes, rename, and containing-directory durability barrier (or a fail-closed platform equivalent) have all succeeded.
**Dependencies**: none
**Verification union**: focused publication-order/failure tests; `cargo test -p opi-eval bundle::tests`; affected-target clippy; documentation check; `git diff --check`.

#### Fix B2.1: Synchronize the parent after atomic rename

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-002`
- **Decision**: `fix:durably-sync-parent-after-publication`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/bundle/mod.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Extend the existing private atomic-publication helper with the narrow platform-specific parent-directory durability operation after rename; propagate failure so no durable proof/state transition is issued when directory durability is unproved. Add an instrumented ordering/failure test without adding a public seam or dependency.
- **Closure predicate**: A successful intent publication proves write -> file sync -> rename -> parent durability in order; an injected parent-durability failure returns an error and withholds `DurableIntentProof`.
- **Red-before**: `rg -n -A 18 "fn atomic_write" crates/opi-eval/src/bundle/mod.rs` -> FAIL: the current helper syncs only the temporary file and returns directly from `fs::rename`.
- **Green-after**: `cargo test -p opi-eval intent_publication_requires_parent_directory_durability -- --nocapture` passes and `cargo test -p opi-eval bundle::tests` passes.

### Batch B3: Enforce the complete Phase 17 Opi evidence graph

**Closure predicate**: An imported Opi trace is complete only when every record has one manifest-bound run, strictly increasing sequence, stable call identity, valid earlier parent, non-self-parent, producer-equivalent kind/payload pairing, and exact terminal run/turn/call/parent/sequence correlation.
**Dependencies**: none
**Verification union**: table-driven saved-trace adversaries; Opi adapter unit tests; `cargo test -p opi-eval --test agent_integration_conformance`; affected-target clippy; documentation check; `git diff --check`.

#### Fix B3.1: Validate graph correlation before accepting completion

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-003`
- **Decision**: `fix:validate-complete-phase17-evidence-graph`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/agent/opi.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Parse the private saved schema into the minimum typed correlation view, mirror the current producer's closed graph invariants (including retry/diagnostic payload rules and Artifact payload admission), compare the complete terminal correlation with the manifest, and add one table-driven adversary for every rejected graph class.
- **Closure predicate**: Each mixed-run, non-increasing-sequence, unstable-call, missing/late/self-parent, kind/payload mismatch, or terminal-correlation mutation settles as a typed import failure; the exact complete fixture still settles successfully.
- **Red-before**: `cargo test -p opi-eval importer_rejects_phase17_invalid_evidence_graphs -- --nocapture` -> FAIL in the test-only archived probe: a mixed-run trace was accepted.
- **Green-after**: The same table-driven graph test passes, all Opi importer tests pass, and `cargo test -p opi-eval --test agent_integration_conformance` passes.

### Batch B4: Execute hermetic helpers natively on Windows

**Closure predicate**: The production hermetic runner stages and executes behavior-equivalent agent and verifier helpers for the host platform, and the committed PowerShell smoke happy/failure/offline paths complete with their declared exits and artifacts.
**Dependencies**: B1, B2, B3
**Verification union**: direct Windows happy run; `python scripts/test_phase18_eval_smoke.py`; assembled-run focused tests on supported hosts; affected-target clippy; documentation check; `git diff --check`.

#### Fix B4.1: Generate host-native bounded helper programs

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-004`
- **Decision**: `fix:generate-native-windows-hermetic-helpers`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Keep the Unix shell helpers, add exact Windows command helpers for the same pinned behaviors and argv guards, select the host-native suffix/content at staging, and cover both agent products plus verifier success/failure without introducing a new public CLI, dependency, or live provider path.
- **Closure predicate**: A Windows `happy` run produces readable `answer.txt`, complete Opi/pi native evidence, sealed bundles, and a published report; declared crash/timeout/verifier-failure behaviors retain their existing typed outcomes.
- **Red-before**: `cargo run -q -p opi-eval -- run --config crates/opi-eval/tests/fixtures/experiment/phase18-local.toml --root <unique-temp> --fixtures crates/opi-eval/tests/fixtures --behavior happy` -> FAIL: `expected agent output is unreadable` with OS error 2.
- **Green-after**: `python scripts/test_phase18_eval_smoke.py` passes on Windows and the existing Unix smoke path remains green.

### Batch B5: Resolve report-output containment before creation

**Closure predicate**: The existing output parent is resolved to its canonical location before containment is checked, and the final create-new target cannot land within the canonical run root through a symlink, junction, or other ancestor alias.
**Dependencies**: B4
**Verification union**: cross-platform subprocess containment test; report contract suite; Windows smoke; affected-target clippy; documentation check; `git diff --check`.

#### Fix B5.1: Canonicalize the output parent and reject aliases into the run

- **Finding source(s)**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 + 099fa8c019f1af3f6362c726895a67b45e83691b9d48b75d4a3e8466724b842b + P18-AUD-005`
- **Decision**: `fix:resolve-report-output-parent-before-containment`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/cli/report.rs`, `crates/opi-eval/tests/report_output_containment.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Require and canonicalize the existing output parent, join only the final absent filename to that resolved parent, compare the resolved target against the canonical run root, preserve create-new behavior, and add a real-binary Unix-symlink/Windows-junction regression that proves no in-root file appears.
- **Closure predicate**: Direct, relative, symlinked, and junction-aliased paths that resolve inside the run root exit 2 without creating bytes; a fresh target under a resolved external parent remains append-only and succeeds once.
- **Red-before**: `cargo test -p opi-eval output_rejects_ancestor_alias_into_run_root -- --nocapture` -> FAIL in the test-only archived probe: `open_output` created the file through the junction.
- **Green-after**: `cargo test -p opi-eval --test report_output_containment` passes on Unix and Windows and `cargo test -p opi-eval --test report_contract` passes on its supported hosts.

### Batch B6: Remove the proxy-free assumption from the config test

**Closure predicate**: The no-explicit-proxy test verifies successful client construction without asserting that ambient proxy policy is absent, and it passes with or without ambient proxy variables.
**Dependencies**: none
**Verification union**: focused test with ambient proxy variables; complete `config_tests` binary; affected test-target clippy; `git diff --check`.

#### Fix B6.1: Assert the product contract instead of ambient state

- **Finding source(s)**: `phase18-pi-glm53-68d74ec-20260830t200548z + 7ea0bd6518d4169f1e597a6d52588038bae9fc1d74a64c5f4d4600c30570da37 + P18-AUD-001`
- **Decision**: `fix:remove-proxy-free-assumption-from-test`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/config_tests.rs`
- **Change kind**: test-only
- **Change**: Rename the test to distinguish absence of explicit configuration from absence of ambient proxy policy and remove only the invalid `proxy_config().url.is_none()` assertion; retain product behavior and all explicit-proxy tests.
- **Closure predicate**: Client construction without an explicit proxy succeeds under both clean and proxy-populated environments, while explicit invalid proxy configuration remains rejected.
- **Red-before**: `$env:HTTP_PROXY='http://127.0.0.1:19828'; $env:HTTPS_PROXY='http://127.0.0.1:19828'; $env:ALL_PROXY='http://127.0.0.1:19828'; cargo test -p opi-coding-agent --test config_tests build_http_client_without_proxy_succeeds -- --exact --nocapture` -> FAIL at the assertion that the proxy URL is none.
- **Green-after**: The renamed focused test passes with the same ambient variables and `cargo test -p opi-coding-agent --test config_tests` passes.

## Final Verification

After B1-B6, run the deduplicated local union:

    cargo test -p opi-eval bundle::tests
    cargo test -p opi-eval importer
    cargo test -p opi-eval --test agent_integration_conformance
    cargo test -p opi-eval --test report_output_containment
    cargo test -p opi-eval --test report_contract
    python scripts/test_phase18_eval_smoke.py
    cargo test -p opi-coding-agent --test config_tests
    cargo clippy -p opi-eval --all-targets -- -D warnings
    cargo clippy -p opi-coding-agent --test config_tests -- -D warnings
    python scripts/opi-doc-check.py
    git diff --check

If D1 authorizes replacement evidence, a revised fixed plan must name the
exact materialized commit, native dispatch/verification commands, derived
matrix paths, terminal three-platform receipt path, and the required workspace
gates. This draft does not authorize or pre-approve those external writes.

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710 / P18-AUD-006` | Returned to shaping pending D1 | The current evidence mismatch is confirmed. The live registered source does not admit a second native run or replacement terminal receipt, so `opi-remediate` cannot choose or execute an authority revision. Current run/digest admission controls; title similarity cannot substitute for identity; no older source or history run was consulted. |
