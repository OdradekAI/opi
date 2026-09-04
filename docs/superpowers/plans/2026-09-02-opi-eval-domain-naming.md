# opi-eval Domain Naming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Phase 18 project-management terminology from active
`opi-eval` interfaces, retire completed delivery machinery, and preserve the
remaining evaluation capabilities under domain names.

**Architecture:** Treat Rust types, schemas, diagnostic codes, scripts,
workflows, fixtures, and artifact identities as one module interface. Retire
files that fail the deep-module deletion test, rename retained entry points by
responsibility, and migrate all repository callers atomically without aliases.
Completed Phase 18 evidence remains only in historical sources of truth.

**Tech Stack:** Rust 2024, Cargo, Python 3 `unittest`, POSIX shell, GitHub
Actions YAML, TOML, JSON, PowerShell host commands.

## Global Constraints

- `docs/opi-spec.md` and `docs/CONTEXT.md` remain authoritative; do not edit
  product direction or architecture terminology in this migration.
- Do not edit `.opi-impl-state.json`, `docs/snapshots/phase18/**`, completed
  research/design/audit/remediation evidence, or released changelog sections.
- `opi-eval` remains `publish = false`, depends on no Opi crate, and has no
  reverse dependency from an Opi product crate.
- Replace active `phase18-` schema prefixes with `opi-eval-` and `P18-`
  diagnostic prefixes with `EVAL-`; do not add aliases, dual-read behavior, or
  compatibility shims.
- Old active inputs fail closed as unsupported schemas or mismatched identities.
- Preserve executable Git modes for retained `.sh` files and
  `scripts/scripted-provider.py`.
- Use explicit paths for every stage/status operation; never use `git add .` or
  `git add -A`.
- Do not create a commit unless the user explicitly authorizes committing this
  task. Each task ends with an uncommitted review checkpoint by default.
- Test impact is `update` and `delete`; no live credentials, paid providers, or
  network-dependent test is permitted.

---

### Task 1: Retire completed delivery machinery and preserve the durable experiment assertion

**Files:**

- Modify: `crates/opi-eval/tests/experiment_contract.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `scripts/opi-doc-check.py`
- Modify: `scripts/test_opi_doc_check.py`
- Delete: `crates/opi-eval/scripts/capture-phase18-minimal-runtime-baseline.py`
- Delete: `crates/opi-eval/scripts/test_capture_phase18_minimal_runtime_baseline.py`
- Delete: `crates/opi-eval/tests/fixtures/minimal-runtime/pre-phase18/**`
- Delete: `crates/opi-eval/scripts/derive-phase18-seam-matrix.py`
- Delete: `crates/opi-eval/scripts/test_derive_phase18_seam_matrix.py`
- Delete: `crates/opi-eval/docs/seam-evidence-matrix.md`
- Delete: `crates/opi-eval/scripts/verify-phase18-ci.py`
- Delete: `crates/opi-eval/scripts/test_verify_phase18_ci.py`
- Delete: `crates/opi-eval/scripts/phase18-eval-smoke.sh`
- Delete: `crates/opi-eval/scripts/phase18-eval-smoke.ps1`
- Delete: `crates/opi-eval/scripts/test_phase18_eval_smoke.py`
- Delete: `crates/opi-eval/tests/phase18_acceptance.rs`
- Delete: `crates/opi-eval/tests/rollback_contract.rs`
- Delete: `crates/opi-eval/external-locks/resolved/linux-x86_64.json`
- Delete: `crates/opi-eval/tests/fixtures/external-locks/materialization/receipt-linux-x86_64.json`

**Interfaces:**

- Consumes: `ResolvedExperiment::resolve(&str) -> Result<ResolvedExperiment, ResolveError>` and `opi_eval::cli::validate(&Path)`.
- Produces: one durable generic multi-subject/multi-benchmark contract test in `experiment_contract.rs`; no active delivery-phase entry point remains.

- [ ] **Step 1: Record the worktree baseline and Cargo cache state**

Run:

```powershell
git rev-parse HEAD
git status --short
git diff --cached --name-status
git diff --name-status
git ls-files --others --exclude-standard
python scripts/opi-cargo-cache.py status
```

Expected: `HEAD` is recorded; the approved design and plan are the only
task-created untracked files; Cargo cache status completes without cleaning or
creating a per-session target directory.

- [ ] **Step 2: Move the durable generic resolution assertion before deleting the acceptance suite**

Add this behavior to `crates/opi-eval/tests/experiment_contract.rs`, using the
existing `generic-three-subject-fourth-benchmark.toml` fixture:

```rust
#[test]
fn generic_schema_resolves_without_product_hard_coding() {
    let path = workspace_root().join(
        "crates/opi-eval/tests/fixtures/experiment/\
         generic-three-subject-fourth-benchmark.toml",
    );
    let source = std::fs::read_to_string(&path).unwrap();
    let resolved = ResolvedExperiment::resolve(&source).unwrap();

    assert_eq!(resolved.subjects().len(), 3);
    assert_eq!(resolved.edges().len(), 3);
    assert_eq!(resolved.trials().len(), 3);
    assert_eq!(cli::validate(&path).unwrap().subject_count, 3);
}
```

Reuse or introduce one `workspace_root() -> PathBuf` helper in this test file;
do not copy the historical baseline-replay helpers.

- [ ] **Step 3: Run the moved assertion while the old suite still exists**

Run:

```powershell
cargo test -p opi-eval --test experiment_contract generic_schema_resolves_without_product_hard_coding
```

Expected: PASS. This is a move of an existing behavior assertion, so the
characterization test is expected to pass before deletion.

- [ ] **Step 4: Remove the completed-only files and CI attestation job**

Delete the listed files/directories. Remove the complete
`phase18_attestation:` job from `.github/workflows/ci.yml`, including its
receipt construction and upload steps. Do not alter the ordinary merge-ref CI
jobs.

Remove these active historical-roadmap symbols and their tests:

```python
PHASE18_ROADMAP_CONTRACT
check_phase18_roadmap_contract
write_phase18_roadmap_docs
test_phase18_roadmap_contract_passes
test_phase18_roadmap_contract_requires_every_token
```

Also remove the call to `check_phase18_roadmap_contract()` from
`scripts/opi-doc-check.py::main`.

- [ ] **Step 5: Verify the deeper retained interfaces still cover current behavior**

Run:

```powershell
cargo test -p opi-eval --test experiment_contract
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

Expected: all commands PASS; no test reads the retired design source or
Minimal Runtime baseline.

- [ ] **Step 6: Check for stale references to retired paths**

Run:

```powershell
rg -n 'capture-phase18|minimal-runtime/pre-phase18|derive-phase18|verify-phase18-ci|phase18_attestation|phase18-eval-smoke|phase18_acceptance|rollback_contract|external-locks/resolved/linux-x86_64|external-locks/materialization/receipt-linux' crates/opi-eval .github scripts README.md README.zh.md CHANGELOG.md --glob '!docs/snapshots/**'
```

Expected: no active caller remains. `CHANGELOG.md` may still contain prose in
historical Unreleased entries until Task 5, but it must not name a now-active
entry point.

- [ ] **Step 7: Review checkpoint**

Run `git status --short` and inspect only the Task 1 diff. Do not commit unless
the user explicitly authorizes a checkpoint commit.

---

### Task 2: Rename retained scripts, workflows, tests, and fixtures by responsibility

**Files:**

- Rename: `.github/workflows/phase18-lock-materialization.yml` -> `.github/workflows/opi-eval-external-lock-materialization.yml`
- Rename: `.github/workflows/phase18-native-smoke.yml` -> `.github/workflows/opi-eval-native-smoke.yml`
- Rename: `crates/opi-eval/scripts/phase18-materialize-locks.sh` -> `crates/opi-eval/scripts/materialize-external-locks.sh`
- Rename: `crates/opi-eval/scripts/verify-phase18-materialization-ci.py` -> `crates/opi-eval/scripts/verify-external-lock-ci.py`
- Rename: `crates/opi-eval/scripts/verify-phase18-materialization-artifact.py` -> `crates/opi-eval/scripts/verify-external-lock-artifact.py`
- Rename: `crates/opi-eval/scripts/phase18-native-smoke.sh` -> `crates/opi-eval/scripts/native-smoke.sh`
- Rename: `crates/opi-eval/scripts/verify-phase18-native-ci.py` -> `crates/opi-eval/scripts/verify-native-smoke-ci.py`
- Rename: `crates/opi-eval/scripts/verify-phase18-native-artifact.py` -> `crates/opi-eval/scripts/verify-native-smoke-artifact.py`
- Rename: `crates/opi-eval/scripts/phase18-build-agent-artifacts.sh` -> `crates/opi-eval/scripts/build-agent-artifacts.sh`
- Rename: `crates/opi-eval/scripts/phase18-scripted-provider.py` -> `crates/opi-eval/scripts/scripted-provider.py`
- Rename: matching retained `test_*.py` files to the names in the design
- Rename: `crates/opi-eval/scripts/fixtures/phase18-native-ci/` -> `crates/opi-eval/scripts/fixtures/native-smoke-ci/`
- Rename: `crates/opi-eval/tests/phase18_assembled_smoke.rs` -> `crates/opi-eval/tests/assembled_run.rs`
- Rename: all eight experiment fixture files listed in the design
- Modify: every caller of these paths under `crates/opi-eval`, `.github/workflows`, `.gitattributes`, and current documentation

**Interfaces:**

- Consumes: the retained script CLI arguments and workflow inputs unchanged.
- Produces: canonical domain-named file paths; callers have exactly one path for each interface.

- [ ] **Step 1: Add a temporary path-contract check and observe failure**

Run before renaming:

```powershell
$bad = rg --files crates/opi-eval .github/workflows | Where-Object { $_ -match '(?i)phase[-_]?18' }
if ($bad) { $bad; exit 1 }
```

Expected: FAIL and print the retained paths that still encode the phase.

- [ ] **Step 2: Rename every retained path with Git-aware moves**

Use the exact mapping from the design. The retained Python test mapping is:

```text
test_phase18_scripted_provider.py
  -> test_scripted_provider.py
test_verify_phase18_materialization_ci.py
  -> test_verify_external_lock_ci.py
test_verify_phase18_materialization_artifact.py
  -> test_verify_external_lock_artifact.py
test_verify_phase18_native_ci.py
  -> test_verify_native_smoke_ci.py
test_verify_phase18_native_artifact.py
  -> test_verify_native_smoke_artifact.py
```

The experiment mapping is:

```text
phase18-local.toml               -> local-paired.toml
phase18-deepswe.toml             -> deepswe.toml
phase18-tb30.toml                -> terminal-bench-3.0.toml
phase18-multi-edge.toml          -> multi-edge.toml
phase18-duplicate-pair.toml      -> duplicate-pair.toml
phase18-integrity-exclusion.toml -> integrity-exclusion.toml
phase18-invalid-task.toml        -> invalid-task.toml
phase18-unsupported-control.toml -> unsupported-control.toml
```

Verify each source exists and each destination stays under the intended
`crates/opi-eval` or `.github/workflows` directory before moving it.

- [ ] **Step 3: Update direct path callers without changing schemas yet**

Update `Path.with_name(...)`, Rust fixture joins, workflow commands, producer
pin paths, verifier defaults, help examples, README fixture references, and
`.gitattributes`. Use these canonical repository paths:

```text
.github/workflows/opi-eval-external-lock-materialization.yml
.github/workflows/opi-eval-native-smoke.yml
crates/opi-eval/scripts/materialize-external-locks.sh
crates/opi-eval/scripts/verify-external-lock-ci.py
crates/opi-eval/scripts/verify-external-lock-artifact.py
crates/opi-eval/scripts/native-smoke.sh
crates/opi-eval/scripts/verify-native-smoke-ci.py
crates/opi-eval/scripts/verify-native-smoke-artifact.py
crates/opi-eval/scripts/build-agent-artifacts.sh
crates/opi-eval/scripts/scripted-provider.py
```

Do not leave fallback probes for old paths.

- [ ] **Step 4: Restore and verify executable modes**

Run:

```powershell
git ls-files --stage crates/opi-eval/scripts/*.sh crates/opi-eval/scripts/scripted-provider.py
```

Expected: retained shell entry points and `scripted-provider.py` have mode
`100755`. If Windows lost a mode, restore only the affected explicit path with
`git update-index --chmod=+x <path>`.

- [ ] **Step 5: Run path-sensitive focused tests**

Run:

```powershell
python crates/opi-eval/scripts/test_scripted_provider.py
python crates/opi-eval/scripts/test_verify_external_lock_ci.py
python crates/opi-eval/scripts/test_verify_native_smoke_ci.py
cargo test -p opi-eval --test assembled_run
```

Expected: path resolution succeeds. Digest assertions may still require Task 4
updates, but no failure may be a missing old file.

- [ ] **Step 6: Re-run the path-contract check**

Run the Step 1 command again.

Expected: PASS with no phase-numbered path under `crates/opi-eval` or the live
workflow directory.

- [ ] **Step 7: Review checkpoint**

Inspect `git diff --summary` and confirm moves are detected as renames and no
executable mode drift remains. Do not commit without explicit authorization.

---

### Task 3: Migrate the Rust, TOML, and JSON module interface identities

**Files:**

- Modify: all `crates/opi-eval/src/**/*.rs` files reported by the phase-token scan
- Modify: all retained `crates/opi-eval/tests/**/*.rs` files reported by the phase-token scan
- Modify: `crates/opi-eval/profiles/agents/*.toml`
- Modify: `crates/opi-eval/profiles/benchmarks/*.toml`
- Modify: retained benchmark, experiment, conformance, and static-lock fixtures under `crates/opi-eval/tests/fixtures/`
- Modify: `crates/opi-eval/external-locks/static/linux-x86_64.json`

**Interfaces:**

- Consumes: the existing closed schema and diagnostic families.
- Produces: the same behavior under `opi-eval-*` schema identities and `EVAL-*` diagnostic identities.

- [ ] **Step 1: Change the experiment contract test first and observe the red state**

In `crates/opi-eval/tests/experiment_contract.rs`, assert:

```rust
assert_eq!(EXPERIMENT_SCHEMA, "opi-eval-experiment/1");
```

Add an explicit fail-closed check:

```rust
#[test]
fn legacy_phase_schema_is_rejected() {
    let legacy = MINIMAL_FIXTURE.replace(
        "opi-eval-experiment/1",
        "phase18-experiment/1",
    );
    assert!(matches!(
        ResolvedExperiment::resolve(&legacy),
        Err(ResolveError::UnsupportedSchema(_))
    ));
}
```

Run:

```powershell
cargo test -p opi-eval --test experiment_contract legacy_phase_schema_is_rejected
```

Expected: FAIL before the production schema constant and fixture are migrated.

- [ ] **Step 2: Apply the closed schema mapping**

Use this exact prefix mapping throughout active Rust, profiles, and fixtures:

```text
phase18-experiment/             -> opi-eval-experiment/
phase18-agent-profile/          -> opi-eval-agent-profile/
phase18-benchmark-profile/      -> opi-eval-benchmark-profile/
phase18-run-report/             -> opi-eval-run-report/
phase18-conformance-report/     -> opi-eval-conformance-report/
phase18-native-material/        -> opi-eval-native-material/
phase18-external-lock/          -> opi-eval-external-lock/
phase18-run-bundle-intent/      -> opi-eval-run-bundle-intent/
phase18-provisional-trajectory/ -> opi-eval-provisional-trajectory/
phase18-regrade-report/         -> opi-eval-regrade-report/
phase18-normalized-report/      -> opi-eval-normalized-report/
phase18-adapter-manifest/       -> opi-eval-adapter-manifest/
phase18-benchmark-report/       -> opi-eval-benchmark-report/
phase18-trial-receipt/          -> opi-eval-trial-receipt/
phase18-scripted-provider/      -> opi-eval-scripted-provider/
phase18-scripted-provider-log/  -> opi-eval-scripted-provider-log/
```

For every other active schema beginning `phase18-`, preserve its suffix and
version and change only the prefix to `opi-eval-`.

- [ ] **Step 3: Migrate diagnostic and invariant identities**

Replace active codes using the mechanical category-preserving rule:

```text
P18-EXP-008 -> EVAL-EXP-008
P18-BMK-003 -> EVAL-BMK-003
P18-SEAM-001 -> EVAL-SEAM-001
```

Apply the same `P18-` -> `EVAL-` prefix replacement to every active code. Do
not renumber suffixes. Rename `p18_*` Rust tests to behavior names without a
phase/task prefix.

- [ ] **Step 4: Replace phase-derived fixture and identity values**

Use stable domain values:

```text
phase18-minimal-pairing       -> minimal-pairing
phase18-local-hermetic        -> local-paired-hermetic
phase18-multi-edge-hermetic   -> multi-edge-hermetic
phase18-tb30-hermetic         -> terminal-bench-3.0-hermetic
phase18-deepswe-hermetic      -> deepswe-hermetic
phase18-linux-x86_64          -> opi-eval-linux-x86_64
phase18-native-material       -> opi-eval-native-material
reviewer = "phase18-native-material" -> reviewer = "opi-eval-native-material"
```

Replace temporary-directory prefixes and test-only IDs with a descriptive
`opi-eval-` or behavior prefix. Do not change benchmark-native identities.

- [ ] **Step 5: Update current source comments and rustdoc**

Remove task/phase history from comments and rustdoc. Describe current
contracts, for example:

```rust
/// Supervises the complete child process tree on Unix and Windows.
```

Do not replace Phase 18 prose with a different delivery-history narrative.

- [ ] **Step 6: Make the focused test green and run the full crate suite**

Run:

```powershell
cargo test -p opi-eval --test experiment_contract
cargo test -p opi-eval --all-targets
```

Expected: PASS; the new schema is accepted and the explicitly supplied legacy
schema is rejected.

- [ ] **Step 7: Scan the Rust/profile/fixture interface**

Run:

```powershell
rg -n '(?i)phase[-_ ]?18|P18-|p18_' crates/opi-eval/src crates/opi-eval/profiles crates/opi-eval/tests --glob '!**/*.log'
```

Expected: no match.

- [ ] **Step 8: Review checkpoint**

Review schema/diagnostic diffs as breaking interface changes and confirm
`CHANGELOG.md` has not yet been edited outside `## [Unreleased]`. Do not commit
without explicit authorization.

---

### Task 4: Migrate retained script, workflow, and artifact contracts

**Files:**

- Modify: all retained, renamed files under `crates/opi-eval/scripts/`
- Modify: `crates/opi-eval/scripts/fixtures/native-smoke-ci/**`
- Modify: `.github/workflows/opi-eval-external-lock-materialization.yml`
- Modify: `.github/workflows/opi-eval-native-smoke.yml`
- Modify: `crates/opi-eval/external-locks/static/linux-x86_64.json`
- Modify: `crates/opi-eval/tests/fixtures/external-locks/static/valid-static.json`
- Modify: `crates/opi-eval/tests/fixtures/external-locks/static/valid-resolved.json`

**Interfaces:**

- Consumes: renamed script/workflow paths and `opi-eval-*` schemas from Tasks 2–3.
- Produces: fail-closed workflow, receipt, artifact, and lock contracts with exact domain-named producer bindings.

- [ ] **Step 1: Update one test to require the new canonical paths and observe failure**

In `test_verify_external_lock_ci.py`, set:

```python
WORKFLOW_PATH = ".github/workflows/opi-eval-external-lock-materialization.yml"
MATERIALIZER = "crates/opi-eval/scripts/materialize-external-locks.sh"
CI_VERIFIER = "crates/opi-eval/scripts/verify-external-lock-ci.py"
```

Run:

```powershell
python crates/opi-eval/scripts/test_verify_external_lock_ci.py
```

Expected: FAIL until the verifier, workflow, static lock, and digests all name
the new contract.

- [ ] **Step 2: Migrate external-lock workflow identities and paths**

Use these active values:

```text
workflow name: opi-eval external-lock materialization
concurrency group: opi-eval-external-lock-materialization
artifact name: opi-eval-linux-lock-materialization
stage directory: opi-eval-lock-materialization
static schema: opi-eval-external-lock/static/1
resolved schema: opi-eval-external-lock/resolved/1
lock id: opi-eval-linux-x86_64
```

Update the workflow, materializer, both verifiers, tests, static lock, and
synthetic fixtures together.

- [ ] **Step 3: Migrate native-smoke workflow identities and paths**

Use these active values:

```text
workflow name: opi-eval Linux native smoke
concurrency group: opi-eval-linux-native-smoke
stage env: OPI_EVAL_NATIVE_STAGE
stage directory: opi-eval-native
provider network: opi-eval-provider-net
artifact name: opi-eval-native-smoke
upload receipt artifact: opi-eval-native-smoke-upload-receipt
```

Update every producer command, receipt, verifier constant, help example, test
fixture, temporary prefix, and embedded wrapper comment. Apply the schema rule
`phase18-` -> `opi-eval-` to active native stage, artifact, receipt, provider,
and upload identities.

- [ ] **Step 4: Recompute bound SHA-256 values from exact LF bytes**

After all script/workflow content is final, calculate SHA-256 using the same LF
normalization as the verifier:

```powershell
$paths = @(
  '.github/workflows/opi-eval-external-lock-materialization.yml',
  'crates/opi-eval/scripts/materialize-external-locks.sh',
  'crates/opi-eval/scripts/verify-external-lock-ci.py'
)
foreach ($path in $paths) {
  $bytes = [IO.File]::ReadAllBytes((Resolve-Path $path))
  $text = [Text.Encoding]::UTF8.GetString($bytes).Replace("`r`n", "`n")
  $hash = [Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))
  "{0}  {1}" -f ([Convert]::ToHexString($hash).ToLowerInvariant()), $path
}
```

Write the resulting exact values into the production static lock and regenerate
matching synthetic lock fixtures. Never copy a stale digest or hand-edit
`Cargo.lock`.

- [ ] **Step 5: Run retained Python tests**

Run:

```powershell
python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'
```

Expected: every retained Python test passes; no retired test is discovered.

- [ ] **Step 6: Run representative negative paths explicitly**

Run the exact test methods that mutate paths/digests:

```powershell
python -m unittest crates.opi-eval.scripts.test_verify_external_lock_ci
python -m unittest crates.opi-eval.scripts.test_verify_native_smoke_ci
```

If dotted module loading is invalid because of the hyphenated crate directory,
run the corresponding two test files directly instead. Expected: PASS,
including rejection of floating actions, wrong producer paths, and digest
drift.

- [ ] **Step 7: Validate script syntax and live workflow contracts**

Run:

```powershell
Get-ChildItem crates/opi-eval/scripts -Filter '*.sh' | ForEach-Object { bash -n $_.FullName }
python crates/opi-eval/scripts/verify-external-lock-ci.py --workflow .github/workflows/opi-eval-external-lock-materialization.yml --script crates/opi-eval/scripts/materialize-external-locks.sh
python crates/opi-eval/scripts/verify-native-smoke-ci.py --workflow .github/workflows/opi-eval-native-smoke.yml --script crates/opi-eval/scripts/native-smoke.sh --build-script crates/opi-eval/scripts/build-agent-artifacts.sh --provider crates/opi-eval/scripts/scripted-provider.py
```

Expected: syntax checks and both static verifiers PASS.

- [ ] **Step 8: Scan active scripts and workflows**

Run:

```powershell
rg -n '(?i)phase[-_ ]?18|P18-|p18_' crates/opi-eval/scripts .github/workflows/opi-eval-external-lock-materialization.yml .github/workflows/opi-eval-native-smoke.yml .github/workflows/ci.yml
```

Expected: no match.

- [ ] **Step 9: Review checkpoint**

Review producer path/digest changes as supply-chain-sensitive code. Confirm
each hash was computed after the final content edit. Do not commit without
explicit authorization.

---

### Task 5: Update current documentation, metadata, and the permanent naming contract

**Files:**

- Modify: `crates/opi-eval/Cargo.toml`
- Modify: `crates/opi-eval/README.md`
- Modify: `crates/opi-eval/README.zh.md`
- Modify: `README.md`
- Modify: `README.zh.md`
- Modify: `.gitattributes`
- Modify: `CHANGELOG.md` only under `## [Unreleased]`
- Modify: `scripts/opi-doc-check.py`
- Modify: `scripts/test_opi_doc_check.py`

**Interfaces:**

- Consumes: the final paths and identities from Tasks 2–4.
- Produces: synchronized current documentation and a source-derived guard that rejects future phase-numbered active `opi-eval` names.

- [ ] **Step 1: Add the failing naming-contract test**

Add a test fixture that creates an active file such as
`crates/opi-eval/scripts/phase18-example.py` and asserts that the checker emits
an error. The checker interface is:

```python
OPI_EVAL_ACTIVE_NAMING_ROOTS = (
    "crates/opi-eval",
    ".github/workflows/opi-eval-external-lock-materialization.yml",
    ".github/workflows/opi-eval-native-smoke.yml",
    ".github/workflows/ci.yml",
)

OPI_EVAL_PHASE_TOKEN = re.compile(
    r"(?i)(phase[-_ ]?18|P18-|p18_)"
)

def check_opi_eval_domain_naming() -> None:
    """Reject delivery-phase terminology from active opi-eval interfaces."""
```

Run the new exact test before implementing the checker.

Expected: FAIL because `check_opi_eval_domain_naming` does not exist.

- [ ] **Step 2: Implement the source-derived naming checker**

The checker must:

- recurse through `crates/opi-eval` files;
- inspect both relative path text and UTF-8-decodable file content;
- inspect the three explicit workflow files;
- ignore `docs/snapshots`, implementation ledgers, historical designs, and Git
  internals because they are outside `OPI_EVAL_ACTIVE_NAMING_ROOTS`;
- append one deterministic error per offending path/token to `ERRORS`;
- run from `scripts/opi-doc-check.py::main`.

Add passing and failing unit tests in `scripts/test_opi_doc_check.py`.

- [ ] **Step 3: Update English and Chinese current descriptions together**

Use this English package description:

```toml
description = "Unpublished Independent Companion for cross-agent evaluation"
```

Replace “provisional Phase 18 seam” sections with a current “Unpublished 0.x
interface” section stating that interfaces may still break before publication,
without tying instability to a delivery phase. Apply equivalent Chinese text
in both Chinese README files.

- [ ] **Step 4: Update comments and changelog**

Change `.gitattributes` comments to describe digest-pinned `opi-eval` evidence
bytes. Under `CHANGELOG.md` `## [Unreleased]`, add one `Changed` entry recording:

```markdown
- `opi-eval`: active scripts, workflows, schemas, diagnostic codes, fixtures,
  and artifact identities now use domain names instead of Phase 18 delivery
  names; completed delivery-only helpers and acceptance assets were retired.
  This deliberately breaks the unpublished 0.x machine-facing identities
  without compatibility aliases.
```

Update stale active file references elsewhere in the Unreleased section, but
do not rewrite released history.

- [ ] **Step 5: Make the documentation contract green**

Run:

```powershell
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

Expected: PASS.

- [ ] **Step 6: Run the complete active-token scan**

Run:

```powershell
rg -n '(?i)phase[-_ ]?18|P18-|p18_' crates/opi-eval .github/workflows/opi-eval-external-lock-materialization.yml .github/workflows/opi-eval-native-smoke.yml .github/workflows/ci.yml README.md README.zh.md .gitattributes
```

Expected: no match. Historical sources outside this command remain unchanged.

- [ ] **Step 7: Review checkpoint**

Compare the English/Chinese README changes side by side and confirm the entire
Unreleased section still has no duplicate subsection headings. Do not commit
without explicit authorization.

---

### Task 6: Run the verification union and hand off the uncommitted change

**Files:**

- Verify: all files changed in Tasks 1–5
- Verify: `docs/superpowers/specs/2026-09-02-opi-eval-domain-naming-design.md`
- Verify: `docs/superpowers/plans/2026-09-02-opi-eval-domain-naming.md`

**Interfaces:**

- Consumes: the complete worktree migration.
- Produces: evidence that active naming, script behavior, Rust behavior,
  workflow contracts, documentation, formatting, and supply-chain bindings are
  coherent.

- [ ] **Step 1: Run script and syntax verification**

Run:

```powershell
python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'
Get-ChildItem crates/opi-eval/scripts -Filter '*.py' | ForEach-Object {
  python -c "import ast,pathlib,sys; ast.parse(pathlib.Path(sys.argv[1]).read_text(encoding='utf-8'))" $_.FullName
}
Get-ChildItem crates/opi-eval/scripts -Filter '*.sh' | ForEach-Object { bash -n $_.FullName }
python scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
```

Expected: all commands PASS.

- [ ] **Step 2: Run Rust checks for the affected crate**

Run:

```powershell
cargo fmt --check --all
cargo test -p opi-eval --all-targets
cargo clippy -p opi-eval --all-targets -- -D warnings
```

Expected: all commands PASS with no warning.

- [ ] **Step 3: Run integration checks for shared script/audit consumers**

Run:

```powershell
cargo test -p opi-coding-agent --test artifact_audit_script
python scripts/opi-artifact-audit.py --help
```

Expected: PASS; the shared artifact auditor remains callable after retiring the
old `opi-eval` evidence directories.

- [ ] **Step 4: Re-run live workflow contract verification**

Run:

```powershell
python crates/opi-eval/scripts/verify-external-lock-ci.py --workflow .github/workflows/opi-eval-external-lock-materialization.yml --script crates/opi-eval/scripts/materialize-external-locks.sh
python crates/opi-eval/scripts/verify-native-smoke-ci.py --workflow .github/workflows/opi-eval-native-smoke.yml --script crates/opi-eval/scripts/native-smoke.sh --build-script crates/opi-eval/scripts/build-agent-artifacts.sh --provider crates/opi-eval/scripts/scripted-provider.py
```

Expected: both report PASS.

- [ ] **Step 5: Run final structural and diff checks**

Run:

```powershell
$matches = rg -n '(?i)phase[-_ ]?18|P18-|p18_' crates/opi-eval .github/workflows/opi-eval-external-lock-materialization.yml .github/workflows/opi-eval-native-smoke.yml .github/workflows/ci.yml README.md README.zh.md .gitattributes
if ($LASTEXITCODE -eq 0) { $matches; exit 1 }
git diff --check
git diff --summary
git status --short
```

Expected: no active phase token, no whitespace error, intended renames/deletes
only, and no unexpected mode change.

- [ ] **Step 6: Inventory the final worktree-only delivery**

Run:

```powershell
git rev-parse HEAD
git diff --cached --name-status
git diff --name-status
git ls-files --others --exclude-standard
```

Expected: `HEAD` matches the Task 1 baseline; `committed=none-for-this-task`;
staged, unstaged, and untracked states are reported separately. The design and
plan documents are task-created files, and historical snapshots/ledgers are
unchanged.

- [ ] **Step 7: Report the handoff**

Report:

- retained and retired interfaces;
- the deliberate `phase18-*` -> `opi-eval-*` and `P18-*` -> `EVAL-*` breaking
  identity migration;
- exact commands and results;
- test impact: `update` and `delete`;
- worktree inventory and any remaining unverified Linux-only manual workflow
  execution;
- that no commit was created unless the user separately authorized it.
