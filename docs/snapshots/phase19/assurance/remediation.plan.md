# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Remediation head**: `2e2bd806d25e95a722b72ca7f4482753cb6d048b`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[], unstaged=[], untracked=[] at admission; only the fixed plan pair changes during planning.
**Unresolved decisions**: none

## Admission and scope

This plan independently verifies the current strict union of two indexed members
and 11 findings at the committed head above. All 85 requirement records were
consumed. The aggregate audit verdict remains FAIL; no member is canonical and
no source record or severity is rewritten. The normative parent and registered
Phase source retain authority; no specification or implementation ledger changes
are planned.

The user explicitly authorized committing the 29 reviewed task files to main
without pushing, then generating this new plan. That materialization commit is
the head above. This planning authorization does not approve application of its
new digest. The clean fixed plan pair is replaced without using earlier plan
content. Existing clean result siblings are not results for this plan; apply
will replace them after a new exact-digest invocation. Prior remediation
conclusions, historical audit runs and external eval/operator outputs are not
current verification inputs.

Verification used a unique faithful git archive at
`/tmp/opi-p19-replan-k4zbchfg/source`, not a Git worktree. Archive SHA-256:
`2c9a20d17b70623f606d39dd319882c833d5b4103fc0cdefed6d7b9e5da370a9`.
Rust tests used CARGO_INCREMENTAL=1 and the existing shared external target
`/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`. No cache was deleted.

The scope is three batches: align one workspace-input pin, retain official
native artifact contrasts, and retain/execute the remaining current native
verification union. Eight current defect claims have committed code and fresh
executable counter-evidence. Refuted means refuted at this remediation head,
not that the original observation was false at its audit head. Their original
Major severities remain unchanged; they require no repair.

## Indexed source bindings

| Member | audit_run_id | findings_sha256 |
| --- | --- | --- |
| codex / gpt6 | `phase19-codex-gpt6-d915e4a-20260918t084558z` | `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` |
| pi / glm53 | `phase19-pi-glm53-ad67cb8-20260918t101920z` | `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` |

## Current Finding Verification

| Source run / finding | Verification | Source / final severity | Closure key / family | Batch | Decision |
| --- | --- | --- | --- | --- | --- |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-001 | Partially confirmed | Major / Major; mandatory closure remains unproved | `prepare.current-source-pin-and-real-cold-materials` / `prepare.materials` | B1 | `fix:refresh-workspace-input-pin-and-prove-real-cold-preparation` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-002 | Refuted | Major / Major; current defect refuted only | `prepare.acquisition-supervision` / `prepare.lifecycle` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-003 | Refuted | Major / Major; current defect refuted only | `controls.required-native-sampling` / `controls.native` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-004 | Refuted | Major / Major; current defect refuted only | `report.sealed-output-control-coverage` / `report.evidence` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-005 | Refuted | Major / Major; current defect refuted only | `run.immutable-recovery` / `run.lifecycle` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-006 | Refuted | Major / Major; current defect refuted only | `run.fresh-resolved-pair-replacement` / `run.lifecycle` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-007 | Refuted | Major / Major; current defect refuted only | `run.stop-on-privacy-failure` / `run.privacy` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-008 | Refuted | Major / Major; current defect refuted only | `acceptance.reject-fixture-substitution` / `acceptance.evidence` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-009 | Partially confirmed | Major / Major; mandatory closure remains unproved | `acceptance.current-native-verification-union` / `acceptance.evidence` | B3 | `verify:retain-current-native-client-lifecycle-and-final-gate-evidence` |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-001 | Refuted | Major / Major; current defect refuted only | `packaging.direct-executable-script-modes` / `packaging.unix` | none | `no-action:refuted-by-current-committed-evidence` |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-002 | Partially confirmed | Major / Major; mandatory closure remains unproved | `acceptance.retained-official-family-contrasts` / `acceptance.evidence` | B2 | `verify:retain-and-run-three-official-native-artifact-contrasts` |

## Unresolved Decisions

none. The selected repair refreshes a stale declaration to match an exact local
commit; it changes no Cargo.lock, dependency graph, public contract or native
benchmark semantics. Runtime prerequisites below are execution checks, not
permission to weaken a closure predicate.

### D1: Exact local source and fresh no-model materials

The production materializer compares the archived Opi Cargo.lock before build.
The exact committed source and its LF-normalized bytes have SHA-256
`261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`.
The current workspace_input.cargo_lock_sha256 declares
`d19619f4c05b0bb00f324345f8103d615be6bcae0462cff5b9b8dbf3cce0cd2a`.
The fresh input-equivalence probe fails with exit 1. B1 changes only that field
to the former digest; Cargo.lock, producer bytes, upstream commits, package
locks and image pins remain unchanged. Source validation is not relaxed.

After approved B1 edits, create a fresh private WSL ext4 root and seed only the
current declared pin/producer/profile/integration set into a new cache. Record
its pre-acquisition inventory. Build the Companion from the approved source and
bind its compiler-selected executable hash, remediation head and change hashes.
Build the Opi client from `/mnt/d/Luiz/Odradek/opi` at explicit commit
`2e2bd806d25e95a722b72ca7f4482753cb6d048b`, using its committed archive rather
than working-tree files or implicit HEAD. Its Cargo.lock must match the new pin.

Prepare both actual clients for terminal-bench-2.1, terminal-bench-3.0 and
deepswe-v1.1, using the existing candidate tasks openssl-selfsigned-cert,
batched-eval-parity and abs-module-cache-flags after unchanged resource/image
admission. From an independent directory, run for each family:

```sh
<exact-companion> prepare --config <frozen-family-experiment.toml> --out <fresh-family-output> --opi-source /mnt/d/Luiz/Odradek/opi --opi-commit 2e2bd806d25e95a722b72ca7f4482753cb6d048b
```

The first attempt has a pin-only cache. Later families may reuse verified common
Agent/tool entries, but their first acquisition must not use prepopulated corpus
or runner contents; retain inventories demonstrating that condition. Repeat each
prepare into a new output directory with the warm cache and compare verified
identities. Keep the pi-only and mismatch rejection regressions. Manual producer
stages cannot replace this public replay, and failed attempts stay retained.

Read-only planning observed Docker Server 29.7.2, Linux x86_64, 32 CPUs and
16,611,233,792 bytes of engine-visible memory. This does not prove task/material
readiness. Inspect required immutable images and, if absent, acquire only their
declared immutable pins outside task dispatch. Retain commands/results. Reject
infeasible tasks instead of substituting candidates or loosening limits. Keep
existing acquisition bounds, cache ownership and image admission. Do not change
upstream revisions or shared images to make a check pass.

No model runs during D1. Native clients below use isolated synthetic credentials
and local scripted endpoints; no real auth file or paid provider is accessed.
The registered source sections 8 and 10 permit this deterministic/native work.
The source finding's acknowledgement of six saved live trials is retained, but
those files are not consumed or treated as current dynamic closure. Real
acceptance and Phase exit remain separate claims requiring the registered
operator handoff.

### D2: Retained evidence and distinct controlled material

Retain each check's unique root before its first external effect with the
existing tempfile facility. Use standard TMPDIR under the private evidence root;
add no product flag/environment contract. Save argv, cwd, exit, stdout/stderr,
input/build/material hashes and job/result paths before assertions. Print the
retained root. Successful and failed raw outputs survive process exit. Cleanup
settles owned children/containers and temporary credentials only, preserving
neighboring resources, shared images and evidence. Provider children must settle
on failure as well as success.

B2 uses original official pinned corpora and graders. B3's controlled lifecycle
tasks are private derivatives of D1's byte-verified runner/tool/integration set,
with explicit derived source/image/content identities. They cannot substitute
for B2, change official corpora, or claim real Agent acceptance. A test-private
helper at tests/support/native_witness_material.rs may build these inputs for
existing witnesses through the native-material/5 contract and normal validation;
it introduces no public materialization API or durable schema.

Construct the existing client JSON from D1 packages with opi, node, pi_bundle,
experiment, opi_package and pi_package keys. OPI_EVAL_OUTPUT_CONTROL_OUT is a
fresh absolute directory. Retain and bind exact wire/projection/package proof
before freezing new plans; never rewrite a frozen material, bundle or score.

## Closure Batches

### Batch B1: Align workspace input and prove real material preparation

**Closure predicate**: the selected source passes pin admission and public
prepare acquires/verifies all selected real families, while mismatches still
reject before publication and warm repeats preserve verified identities.
**Dependencies**: none.
**Verification union**: repository-input regression, production acquisition
Python suite, public prepare regressions, external-lock checker suite,
documentation/diff checks, and D1 real cold/warm replay.

#### Fix B1.1: Refresh the stale workspace-input digest

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `P19-AUD-001`; owning findings SHA-256 is the exact codex binding above.
- **Decision**: `fix:refresh-workspace-input-pin-and-prove-real-cold-preparation`.
- **Verification status**: Partially confirmed: orchestration exists, input admission fails.
- **File(s)**: `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/scripts/test_prepare_acquisition.py`, `CHANGELOG.md`.
- **Change kind**: metadata, with an executable input-admission regression.
- **Change**: replace only workspace_input.cargo_lock_sha256 with `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`. Add test_repository_workspace_input_matches_static_pin in the existing Python suite, comparing repository Cargo.lock LF bytes with the shipped pin. Retain source/pin/producer rejection cases. Read the full Unreleased section and record this user-visible preparation repair in its existing appropriate subsection. No producer digest refresh is needed.
- **Closure predicate**: the new regression, retained mismatch tests and D1 real public cold/warm replays all pass, with Cargo.lock and unrelated pins unchanged.
- **Red-before**: `python3 /tmp/opi-p19-replan-k4zbchfg/inputs/check-workspace-input.py /tmp/opi-p19-replan-k4zbchfg/source` — FAIL, exit 1; exact source 261a3ec5... differs from declared d19619f4.... Appendix B retains both full hashes and raw stderr.
- **Green-after**: `python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py`; `cargo test -p opi-eval --locked --test native_driver prepare_ -- --nocapture`; D1 three-family real public cold/warm replay — expected PASS. Fixture success alone cannot close this batch. Observed: not-run.

### Batch B2: Retain the three official native artifact contrasts

**Closure predicate**: each official family retains attributable correct/incorrect
production-handoff/native-grader evidence after its test process exits.
**Dependencies**: B1 and D1's verified materials.
**Verification union**: owning native_driver binary, affected-target clippy,
three exact ignored official filters, documentation checks.

#### Fix B2.1: Preserve official witness roots and raw results

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` / `P19-AUD-002`; owning findings SHA-256 is the exact pi binding above.
- **Decision**: `verify:retain-and-run-three-official-native-artifact-contrasts`.
- **Verification status**: Partially confirmed; current retention loss is proved, universal historical absence is not.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/examples/local-evaluation/README.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`.
- **Change kind**: test-only retention/evidence execution, with synchronized instructions.
- **Change**: replace PID-based/pre-deleted roots in native_tb21_artifact_witness, native_tb3_artifact_witness and native_deepswe_patch_witness with uniquely claimed retained temp roots. Remove success-time deletion; write argv/exits/stdout/stderr/input hashes before assertions and print paths. Preserve production conformance calls and official identity/reward/collection predicates. Retain native jobs, verifier-input/artifact digests and native outcomes. Update both READMEs and the unpaired walkthrough with reproducible no-model commands and retention instructions.
- **Closure predicate**: three official native contrasts pass with surviving correct/incorrect raw evidence, differing required artifacts/outcomes and unchanged declared-only collection semantics; no remote inference occurs.
- **Red-before**: FAIL (static): current functions at native_driver.rs:2253/2302/2375 each remove their root after successful assertions. Source bodies/hashes are retained; no model/grader failure is fabricated for a test-only change.
- **Green-after**: the three exact B2 ignored filters listed in Final Verification, each bound to its own official D1 material — expected PASS with retained raw evidence. Observed: not-run.

### Batch B3: Retain current native verification and final gates

**Closure predicate**: the entire listed current no-model dynamic union and final
gates have retained raw commands/results/source/material identities, without
mutating original evidence or substituting controlled tasks for B2 contrasts.
**Dependencies**: B1/D1 and B2.
**Verification union**: seven remaining native filters, independent CLI replay,
modified owning tests and final workspace gates.

#### Fix B3.1: Complete native input setup, retention and canary evidence

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` / `P19-AUD-009`; owning findings SHA-256 is the exact codex binding above.
- **Decision**: `verify:retain-current-native-client-lifecycle-and-final-gate-evidence`.
- **Verification status**: Partially confirmed: current roots are dropped and required native checks remain unobserved here.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`, `crates/opi-eval/examples/local-evaluation/README.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`.
- **Change kind**: test-only setup, retention and missing canary coverage, with synchronized instructions.
- **Change**: retain protocol/turn-budget matrix roots and real per-client outputs; preserve the output-control proof root. Retain the cancellation witness root instead of dropping it. Bind its existing OPI_EVAL_WITNESS_IMAGE fixture override to `python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc` and remove its mutable busybox pull. Use existing production supervision and verify owned PIDs/containers are gone while neighbors/shared images stay unchanged.
- **Change**: add a test-private material builder from D1's verified inputs for real-runner delay cases (positive, time-cap, agent-cap, verifier-cap, both-agent, both-verifier, minimum-cap, strict-task, unbounded-task) and fresh Git/Python grading cases (positive, negative, timeout). Preserve Harbor/Pier source/lock identities and real Docker lifecycle; retain distinct derived task/image digests. Keep existing positive controls, timing limits and cleanup predicates. These private fixtures never replace official benchmark contrasts.
- **Change**: add ignored native_credential_canary_isolation through actual pinned runner/production integration/grader paths with synthetic child-only credentials. Agent-side positive control must observe its intended dummy value; grader environment/files and exported evidence must not contain it. Retain credential-handoff removal and unchanged material/neighbor checks. No real auth, product knob or remote model call is introduced.
- **Closure predicate**: all seven B3 filters, independent CLI replay and final gates pass with retained raw evidence. Fresh regrade invokes no Agent/provider and preserves source bundles; cancellation/canary checks prove owned cleanup and grader/export isolation; B2 evidence remains distinct.
- **Red-before**: FAIL (static): native_protocol_client_matrix and native_client_turn_budgets drop tempfile roots; all 9 existing native tests remain ignored by default and native_credential_canary_isolation is absent. This is current retention/coverage evidence, not a claim of global historical absence. No model/grader red failure is invented.
- **Green-after**: run all seven exact B3 filters, independent replay and final workspace gates below — expected PASS with commands/exits/input hashes retained and no remote inference. Observed: not-run.

For independent CLI replay use D1 materials, both actual clients, explicit task
selection, fresh outputs and an isolated scripted provider through the runner's
existing supported inference route. Exercise public validate/plan/run/verify/report
from an independent cwd across the three selected families, retaining exact
commands and valid zero outcomes. Compare source bundle bytes before/after offline
operations. This local scripted execution does not establish first real-provider
connection fidelity or the six-live-trial criterion.

## Final Verification

Run a focused check once at its final relevant state. Execute every modified
test binary, not merely compile it. New failures stop dependent batches. Changes
to other pins, public contracts, dependencies, specifications or authority need
a new plan; unrelated prerequisite repair is not silently added here.

```sh
python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py
python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_ci.py
cargo test -p opi-eval --locked --test native_driver prepare_ -- --nocapture
cargo test -p opi-eval --locked --test native_driver
cargo clippy -p opi-eval --all-targets -- -D warnings
python scripts/opi-doc-check.py
git diff --check
```

B2 uses each official family material separately. B3 uses the D1 client JSON and
separate controlled lifecycle materials; no one manifest is relabelled to fit a
different meaning. TMPDIR is inside the fresh evidence root. Exact native and
final workspace commands follow:

```sh
cargo test -p opi-eval --locked --test native_driver native_tb21_artifact_witness -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_tb3_artifact_witness -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_deepswe_patch_witness -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_protocol_client_matrix -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_client_turn_budgets -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_output_control_client_matrix -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_owned_resource_cancellation -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_verifier_only_regrade_witness -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_credential_canary_isolation -- --ignored --exact --nocapture --test-threads=1
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The ten native commands are nine existing ignored filters plus the planned
canary filter. They are future requirements, not planning PASS claims. Missing
resources/downloads/images remain explicit failed/incomplete checks.

Test impact now: **none**, apart from a private standalone red probe. Planned
apply impact: **add** workspace-input regression, native canary and test-private
material support; **update** relevant witness retention; **retain** current
behavioral regressions. No tests are deleted. The two crate READMEs are updated
together; the walkthrough and fixed assurance plan have no Chinese counterpart.

## Exclusions

| Source run / finding | Disposition | Current evidence |
| --- | --- | --- |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-002 | Refuted at current head; no action | runner/prepare.rs uses SpawnSpec with a 3600-second deadline and 1 MiB stream caps, settle_acquisition and cancellation. The cancellation regression and 7 prepare unit tests pass, covering owned cleanup, bounded output, failure diagnostics and cache preservation. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-003 | Refuted at current head; no action | experiment.rs:1017 admits required controls against selected client capabilities. Required temperature/top_p reject before dispatch; current required and optional sampling regressions pass. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-004 | Refuted at current head; no action | report.rs:932 reconstructs output_control_proven from sealed evidence; coverage consumes it at 1587. The offline regression passes for absent/present proof, mutable report replacement, seal rejection and unchanged source bytes. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-005 | Refuted at current head; no action | runner/experiment.rs:500 returns recovery_report without writing over the original report. Current frozen-inventory and byte-preservation regressions pass, including unstarted trials and identity drift. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-006 | Refuted at current head; no action | runner/experiment.rs:587 replaces the resolved experiment; replace_group creates a fresh token. Current selection/config replacement regression passes for distinct attempts, original roots, unrelated groups and tampered-plan rejection. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-007 | Refuted at current head; no action | runner/experiment.rs:2303 cancels the run token on canary detection. Current privacy regression passes: the second job is not dispatched and planned missing coverage stays visible. |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-008 | Refuted at current head; no action | verify-local-acceptance.py:352 requires sealed live mode and FIRST_CONNECTION, plus native client/provider/tool observations. Fixture rejection and 42 Python checker tests pass; synthetic structural tests are not live acceptance evidence. |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-001 | Refuted at current head; no action | Exact committed archive modes are 0755 for both scripts; inputs/executable-modes.json records them. |

Every other current source key is in one closure batch. Current run/digest
identity controls admission; similar titles cannot substitute, and no older
source is consulted.

## Planning checks actually observed

| Exact command / observation | Result | Raw evidence relative to the private root |
| --- | --- | --- |
| `cargo test -p opi-eval --locked --test agent_integration_conformance --test native_driver --test assembled_run -- --nocapture` | PASS: 2 + 15 + 74; 9 native tests ignored | `logs/current-regressions.log` |
| `cargo test -p opi-eval --locked --lib runner::prepare` | PASS: 7 | `logs/prepare-unit.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py` | PASS: 42 | `logs/acceptance-checker.log` |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py` | PASS: 2, with family/product and mismatch subcases | `logs/prepare-acquisition.log` |
| `python3 /tmp/opi-p19-replan-k4zbchfg/inputs/check-workspace-input.py /tmp/opi-p19-replan-k4zbchfg/source` | Expected FAIL, exit 1 | `logs/workspace-input-red.stderr`, `inputs/source-lock-comparison.json` |
| Both direct-entrypoint modes in exact committed archive | PASS: 0755/0755 | `inputs/executable-modes.json` |
| Current witness source body inspection | FAIL retention predicates; no native execution inferred | `inputs/witness-source-observations.json` |
| `python scripts/opi-cargo-cache.py status` | Existing external cache inspected; no deletion | task command output |
| WSL Docker version/info read-only observation | 29.7.2; linux x86_64; 32 CPUs; 16611233792 bytes | evidence index observation |

All evidence above belongs to `/tmp/opi-p19-replan-k4zbchfg` and the exact head
in this plan. inputs/commands.json records argv, cwd, exit codes and log hashes.
The first non-login WSL launcher could not find cargo before spawning a test;
execution then used the observed /home/luiz/.cargo/bin on PATH. That was an
environment correction, not a repository repair. No native/model execution or
production edit occurred during planning.

Evidence index: `/tmp/opi-p19-replan-k4zbchfg/inputs/evidence-index.json`; SHA-256 `66d44e7c4389288a4a022dffb4709ebff8411f645405cc89ea1ed7627396d8b6`.

## Appendix A: exact criterion identity matrix

Only exact (path, sha256, citation) keys align. Each column preserves source
requirement IDs/states/finding links; differing citations stay separate. Full
records remain in the indexed sidecars and inputs/indexed-requirements.json.
This matrix is descriptive and never overrides evidence by reviewer vote.

| Exact identity: path / SHA-256 / citation | codex source records | pi source records |
| --- | --- | --- |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-001; sections 3–10 | P19-AC-001-01: met; findings=none<br>P19-AC-001-02: met; findings=none<br>P19-AC-001-03: partially-met; findings=P19-AUD-001<br>P19-AC-001-04: partially-met; findings=P19-AUD-009 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-002; sections 3–10 | P19-AC-002-01: met; findings=none<br>P19-AC-002-02: met; findings=none<br>P19-AC-002-03: met; findings=none<br>P19-AC-002-04: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-003; sections 3–10 | P19-AC-003-01: met; findings=none<br>P19-AC-003-02: not-assessable; findings=P19-AUD-009<br>P19-AC-003-03: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-004; sections 3–10 | P19-AC-004-01: met; findings=none<br>P19-AC-004-02: met; findings=none<br>P19-AC-004-03: not-assessable; findings=P19-AUD-009<br>P19-AC-004-04: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-005; sections 3–10 | P19-AC-005-01: met; findings=none<br>P19-AC-005-02: met; findings=none<br>P19-AC-005-03: not-assessable; findings=P19-AUD-009<br>P19-AC-005-04: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-006; sections 3–10 | P19-AC-006-01: met; findings=none<br>P19-AC-006-02: partially-met; findings=P19-AUD-003<br>P19-AC-006-03: not-assessable; findings=P19-AUD-009<br>P19-AC-006-04: partially-met; findings=P19-AUD-003<br>P19-AC-006-05: met; findings=none<br>P19-AC-006-06: met; findings=none<br>P19-AC-006-07: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-007; sections 3–10 | P19-AC-007-01: met; findings=none<br>P19-AC-007-02: met; findings=none<br>P19-AC-007-03: met; findings=none<br>P19-AC-007-04: met; findings=none<br>P19-AC-007-05: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-008; sections 3–10 | P19-AC-008-01: met; findings=none<br>P19-AC-008-02: met; findings=none<br>P19-AC-008-03: met; findings=none<br>P19-AC-008-04: met; findings=none<br>P19-AC-008-05: met; findings=none<br>P19-AC-008-06: partially-met; findings=P19-AUD-003,P19-AUD-004<br>P19-AC-008-07: met; findings=none<br>P19-AC-008-08: partially-met; findings=P19-AUD-004 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-009; sections 3–10 | P19-AC-009-01: met; findings=none<br>P19-AC-009-02: met; findings=none<br>P19-AC-009-03: met; findings=none<br>P19-AC-009-04: met; findings=none<br>P19-AC-009-05: met; findings=none<br>P19-AC-009-06: not-assessable; findings=P19-AUD-009 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-010; sections 3–10 | P19-AC-010-01: met; findings=none<br>P19-AC-010-02: met; findings=none<br>P19-AC-010-03: partially-met; findings=P19-AUD-002,P19-AUD-003<br>P19-AC-010-04: partially-met; findings=P19-AUD-002,P19-AUD-007<br>P19-AC-010-05: met; findings=none<br>P19-AC-010-06: not-met; findings=P19-AUD-005<br>P19-AC-010-07: partially-met; findings=P19-AUD-006<br>P19-AC-010-08: not-assessable; findings=P19-AUD-009 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-011; sections 3–10 | P19-AC-011-01: met; findings=none<br>P19-AC-011-02: partially-met; findings=P19-AUD-001<br>P19-AC-011-03: partially-met; findings=P19-AUD-008<br>P19-AC-011-04: met; findings=none<br>P19-AC-011-05: partially-met; findings=P19-AUD-009 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-012; sections 3–10 | P19-AC-012-01: met; findings=none<br>P19-AC-012-02: met; findings=none<br>P19-AC-012-03: partially-met; findings=P19-AUD-008<br>P19-AC-012-04: not-assessable; findings=P19-AUD-009 | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>P19-AC-013; sections 3–10 | P19-AC-013-01: met; findings=none<br>P19-AC-013-02: met; findings=none<br>P19-AC-013-03: met; findings=none<br>P19-AC-013-04: met; findings=none<br>P19-AC-013-05: met; findings=none | unaligned |
| `docs/opi-spec.md`<br>`1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`<br>PRIN-002 | P19-PRIN-002: met; findings=none | unaligned |
| `docs/opi-spec.md`<br>`1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`<br>PRIN-004 | P19-PRIN-004: partially-met; findings=P19-AUD-003,P19-AUD-004,P19-AUD-007,P19-AUD-008 | unaligned |
| `docs/opi-spec.md`<br>`1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`<br>PHASE-001 | P19-PHASE-001: met; findings=none | unaligned |
| `docs/opi-spec.md`<br>`1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`<br>AUTH-004 | P19-AUTH-004: met; findings=none | unaligned |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-001 | unaligned | P19-AC-001: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-002 | unaligned | P19-AC-002: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-003 | unaligned | P19-AC-003: partially-met; findings=P19-AUD-002 |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-004 | unaligned | P19-AC-004: partially-met; findings=P19-AUD-002 |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-005 | unaligned | P19-AC-005: partially-met; findings=P19-AUD-002 |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-006 | unaligned | P19-AC-006: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-007 | unaligned | P19-AC-007: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-008 | unaligned | P19-AC-008: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-009 | unaligned | P19-AC-009: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-010 | unaligned | P19-AC-010: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-011 | unaligned | P19-AC-011: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-012 | unaligned | P19-AC-012: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 7, table row P19-AC-013 | unaligned | P19-AC-013: met; findings=none |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md`<br>`5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`<br>Section 8, verification union | unaligned | P19-EXIT-GATES: not-met; findings=P19-AUD-001 |

## Appendix B: reproducible current red and retention probes

The retained private unittest wrapper computes SHA-256 of Cargo.lock LF bytes
in the exact archive, then asserts equality with the shipped static lock's
workspace_input.cargo_lock_sha256. Command, full values and failing stderr are
recorded above; its source is inputs/check-workspace-input.py. No archive file
was modified for this probe.

The retention probe reads the current native_driver.rs function bodies and
records lines/text/SHA-256 in inputs/witness-source-observations.json. The three
official witness bodies call fs::remove_dir_all(&root) after successful assertions.
The protocol and turn-budget matrices own tempfile::tempdir without keep().
These are static retention failures, not executions of a native grader or model.

## Materialization boundary

Planning changes only the fixed plan Markdown/sidecar. It does not change the
static pin, production/test code, result siblings, audit members, specification
or ledger. Validate dispositions, plan, audit-set, documentation and diff, then
present the emitted plan digest. A new apply invocation with that exact digest
is required before B1/B2/B3 edits or native execution.

Apply must preserve all source records and closure predicates, record actual
green evidence or explicit nonclosure, and retain current success/failure raw
artifacts. No further commit/push, new audit, live acceptance, release or Phase
PASS is automatic. A fresh reviewer run requires committed fixes/live set,
a clean assurance directory, resolved external evidence and completion of the
owning-workflow return.
