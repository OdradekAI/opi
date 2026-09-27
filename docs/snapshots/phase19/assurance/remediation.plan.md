# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `40c48da48a5192a8cfce2b32ce80dc91bf33e5435442e9facff169c0f4ec7b00`
**Remediation head**: `075a046ebc585265c28a6c3f778dfcc501338ddc`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: committed=none-for-this-task; staged=[]; unstaged=[]; untracked=[]; worktree-only planning.
**Unresolved decisions**: none

## Admission and source boundary

The complete active audit set passed `validate_assurance_artifact.py audit-set` before semantic consumption. Index schema 2, revision 6, aggregate verdict FAIL; 15 strict-union source keys (10 C, 5 P). The current validator prints `audit-set: PASS`; the exact raw index SHA-256 above was computed separately because this implementation does not emit it. Each member head is an ancestor of the remediation head. Current source/spec bytes relevant to these findings are unchanged since the members' audit head; no historical conclusion is used as a new proof.

| Alias | Exact source run | Owning findings SHA-256 | Member audit head |
|---|---|---|---|
| C | `phase19-codex-gpt6-e079780-20260926t093335z` | `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` | `e079780bda6681ec466f79bd15f3248baf0f2eb8` |
| P | `phase19-pi-glm53-e079780-20260926t192024z` | `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` | `e079780bda6681ec466f79bd15f3248baf0f2eb8` |

`C001` means the C run's `P19-AUD-001`; `P001` means the P run's separate `P19-AUD-001`. This notation never merges source identity. Indexed metadata, requirements and findings remain unchanged and retain their source titles, claims, severities, evidence and independence fields. Only the current run/digest admits a finding; title similarity cannot substitute for identity, and no older source was consulted.

Committed code was inspected in a unique `git archive` export, not a Git worktree. History, earlier remediation conclusions, eval artifacts and alternate source/plan paths were not inputs. Ledger scope was read only through `opi-audit/scripts/read_audit_scope.py`'s allowlisted committed projection. Source registration and exact hashes match both member metadata; it confirms the supplemental English/Chinese design is registered, rather than treating an arbitrary design file as authority.

This continuation reseals the current fixed plan against the committed HEAD above after apply admission detected a head mismatch. Starting inventory: staged=[], unstaged=[], untracked=[]; committed=none-for-this-task. The current plan's closure decisions, batches, predicates and disposition bytes are retained. The only committed differences from the recorded evidence state are the fixed plan pair; all other tracked bytes, including source, tests, specifications, ledgers and indexed audit artifacts, are identical. Result artifacts are not inputs. Only the fixed plan Markdown is an outgoing repository edit; the paired JSONL is retained byte-identically. No product code, ledger, manifest, lockfile, branch or commit is changed.

## Observable completion

1. Admit source set and clean-output baseline -> verify: whole-set validator, HEAD and separate staged/unstaged/untracked inventories.
2. Verify each current source key -> verify: committed call paths, counter-evidence and isolated discriminating probes below.
3. Seal the complete plan -> verify: 15/15 exact dispositions, dispositions validator, fixed-plan validator, documentation checker and whitespace check.

Planning completion is a validated READY-FOR-APPLY plan, not execution or a Phase-conformance verdict. This reseal retains the accepted D1 direction and the existing artifact limit in B11 without a new design decision. The new validator-emitted digest needs a separate explicit `opi-remediate mode=apply phase=19 plan_sha256=<digest>` invocation.

## Current Finding Verification

| Source | Finding ID | Verification | Source → final severity and rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|---|
| C001 | `P19-AUD-001` | Confirmed | Major → Major: Major retained: an admitted credential reference cannot authenticate the selected actual client. | `connection.opi-file-credential-projection` / `connection.projection` | B1 | `fix:project-resolved-file-credential` |
| C002 | `P19-AUD-002` | Confirmed | Major → Major: Major retained: provenance and unconfirmed cleanup must stop new affected execution under registered design Section 9. | `native.boundary-failure-stops-dispatch` / `native.supervision` | B2 | `fix:cancel-after-unsettled-boundary` |
| C003 | `P19-AUD-003` | Confirmed | Major → Major: Major retained: byte verification must not silently discard tampered whole trials. | `offline.verify.reject-trial-links` / `offline.evidence` | B3 | `fix:reject-linked-trial-inventory` |
| C004 | `P19-AUD-004` | Confirmed | Major → Major: Major retained: pi-only evaluation has independent value and must regrade without absent Opi material. | `offline.regrade.selected-products` / `offline.regrade` | B4 | `fix:load-only-source-selected-agent-material` |
| C005 | `P19-AUD-005` | Confirmed | Major → Major: Major retained: selected=0 misstates a valid full experiment and its coverage. | `report.full-selected-count` / `report.coverage` | B5 | `fix:count-expanded-full-task-selection` |
| C006 | `P19-AUD-006` | Partially confirmed | Major → Major: Major retained until all required final gates pass; the isolated failure is an environment defect, not established Bash product breakage. | `verification.final-workspace-gates` / `verification.environment` | B10 | `fix:reverify-with-known-system-tools` |
| C007 | `P19-AUD-007` | Confirmed | Major → Major: Major retained: the registered actual-client public run-to-report acceptance seam has no assembled proof. | `native.report.actual-client-failure-matrix` / `native.report-integration` | B6 | `fix:add-real-client-public-report-matrix` |
| C008 | `P19-AUD-008` | Confirmed | Major → Major: Major retained: infrastructure startup failure currently contributes a valid Agent zero. | `native.outcome.reject-unobserved-client-startup` / `native.outcome-classification` | B7 | `fix:separate-client-startup-failure-from-task-zero` |
| C009 | `P19-AUD-009` | Confirmed | Major → Major: Major retained: correct first-connection native Opi evidence is rejected. | `acceptance.native-api-identity` / `acceptance.native-evidence` | B8 | `fix:interpret-api-by-native-product` |
| C010 | `P19-AUD-010` | Confirmed | Major → Major: Major retained: truncation prevents actual completed trials from supplying required acceptance evidence. | `native.capture.complete-settled-streams` / `native.evidence-capture` | B11 | `fix:stream-complete-native-capture-with-existing-artifact-bound` |
| P001 | `P19-AUD-001` | Cannot confirm | Major → Major: Major retained: successful synthetic checks cannot satisfy six real Agent trials; the historical external root was not read or re-adjudicated. | `acceptance.six-valid-real-trials` / `acceptance.operator-evidence` | — | `no-action:deferred-by-registered-source` |
| P002 | `P19-AUD-002` | Partially confirmed | Major → Major: Major retained for the reproduced truncation/invalid-result effect. The profile-cap explanation is not the native-path root cause. | `native.capture.complete-settled-streams` / `native.evidence-capture` | B11 | `fix:stream-complete-native-capture-with-existing-artifact-bound` |
| P003 | `P19-AUD-003` | Confirmed | Minor → Minor: Minor retained: declared commands have drifted while substantive behavior has separate test coverage. | `ledger.nonvacuous-verification-filters` / `verification.ledger` | — | `no-action:returned-to-shaping-ledger-owner` |
| P004 | `P19-AUD-004` | Partially confirmed | Minor → Minor: Minor retained for opaque test-helper failure; the historical >25-minute and missing-log claims are unconfirmed. | `native.witness.material-prerequisite-diagnostics` / `native.witness-tests` | B9 | `fix:validate-witness-material-before-staging` |
| P005 | `P19-AUD-005` | Cannot confirm | Minor → Minor: Minor retained; the historical orphan incident is not reproduced, and its proposed label mechanism conflicts with current code. | `native.witness.owned-resource-cleanup` / `native.supervision` | — | `no-action:cannot-confirm-diagnostic-cleanup-incident` |

### Current evidence and counter-evidence

- **C001**: experiment.rs:1135 inserts environment values only for variable(); FromFile returns None, while render_opi_config at :1263 references OPI_EVAL_AGENT_KEY. The current probe observes that exact missing value; configuration bytes remain secret-free. The existing environment-reference path is counter-evidence against a general projection failure.
- **C002**: runner/experiment.rs:769 only checks the shared cancellation token; cleanup handling at :2000-2012 records a failure without cancelling; :2303 cancels for canary leakage only. Two public CLI probes each start the second serial trial after the first fails. Counter-evidence: benchmark/process.rs:1410 already cancels unknown native command termination; preserve that path.
- **C003**: regrade.rs:113-125 filters DirEntry types with is_dir() before verification; :97 treats an empty failure list as verified. report.rs:291-310 already includes and rejects linked inputs. Bundle artifact-ancestor checks cannot validate a trial omitted before opening its bundle.
- **C004**: regrade.rs:312 calls NativeMaterial::load before discovering sealed products at :491-505. runner/material.rs:257 fixes load to [opi,pi], while load_for at :265 and cli/run.rs:134 already support exact selected sets. The public pi-only source run exits 0; regrade exits 2 demanding both products.
- **C005**: experiment.rs:152,203,858 intentionally encodes full scope with empty task_ids; runner/plan.rs:295-303 expands tasks correctly. report.rs:841-843 counts the original empty array. Validated planned_trials already carries expanded tasks (:592,1071). Changing run-control selection alone would violate its equality to the sealed experiment at :1065-1071.
- **C006**: Exact WSL cargo filter reproduces 0 passed/1 failed at tools_read_write_edit_bash.rs:2121. Current login PATH resolves env to /mnt/e/opi/pi-debug/uv-bin/env; sh -c env exits 0 with zero bytes. The identical compiled test binary with PATH=/usr/bin:/bin, HOME=/tmp and a harmless canary passes 1/0 in 0.02s. BashTool faithfully inherits PATH and forwards output (tool/operations.rs:908-920,1220-1237; tool/bash.rs:298-315). No unrelated gate is inferred green from this control.
- **C007**: native_driver.rs:2748-2959 launches actual clients directly (:2826-2834) without public run/report; :5719 uses stage(provider-error/zero), whose setup at :669,:709-809 installs stand-ins. The broader synthetic matrix at :6259 does not supply native-client provider behavior. Scope-reader projection of task 19.7 explicitly requires the same assembled path.
- **C008**: runner/experiment.rs:1659 maps ordinary nonzero Opi exit to agent-non-zero-exit; classifier :1453 can therefore accept bound reward zero. An isolated runner injects empty stdout and a loader diagnostic and observes exit 0, sealed graded-native, reward=0 and agent_failed=true. A valid-zero control passes. The precise historical GNU binary/container incident is source testimony only; no external eval artifacts were read.
- **C009**: opi-ai/src/lib.rs:62-67 serializes ApiKind::Anthropic as Anthropic; message.rs:30 carries it and anthropic.rs:683 produces it. verify-local-acceptance.py:439 compares both products to anthropic-messages; synthetic fixture :190 uses that string for both products. The isolated probe rejects native-shaped Opi, accepts native pi, and rejects unknown/wrong-product API controls.
- **C010**: runner/experiment.rs:1715,1748-1751 hard-caps completed native capture; :2077-2090 seals that prefix in full mode; pi.rs:395-396 rejects truncated input and acceptance verifier :418-447 requires complete terminal/tool evidence. The current test at runner/experiment.rs:4523 asserts the cap, so silently raising/removing it is not a complete resource decision.
- **P001**: Current verifier enforces six attributable trials and calls verify/report (verify-local-acceptance.py:check_trials and :run). New independent probes confirm capture and native-API blockers, but do not establish a current external acceptance outcome. Registered design Section 8 reserves real acceptance to a separately authorized local operator; P19-AC-012 remains unmet until supplied evidence is independently verified.
- **P002**: The native handoff at runner/experiment.rs:1554-1571 carries no profile capture cap. Host adapters use profile caps (agent/pi.rs:225; agent/opi.rs:197), but Harbor :416-417 and Pier :455-456 redirect native streams to files. The independent capture probe confirms the later hard reader cap. Therefore C010/P002 share the B11 root repair and predicate, while their source identities and rationales remain separate.
- **P003**: The committed allowlisted scope projection for 19.6 names assembled_run independent_plan_run, absent from that source; existing candidates are frozen_subset_execution (:1424) and hermetic_paired_group_runs_end_to_end (:242). Task 19.8 names native_offline_regrade -- --ignored --exact, but native_driver.rs:5838-5839 has #[test] without #[ignore]. This is checked without reading ledger conclusions.
- **P004**: native_witness_material.rs:94-102 unconditionally unwraps family/tasks/profile data after side effects at :13-73. Both callers already declare three-family input (:8405,8964); missing-family support is not required. run_cli at helper :381-410 already saves command/output/digest/elapsed evidence and both long callers use it. No historical operator logs were read.
- **P005**: runner/experiment.rs:3651-3663 discovers standard com.docker.compose.project.working_dir labels and :3806-3869 validates/removes exact immutable IDs, not an opi-eval-only label. Diagnostic witnesses do omit OwnedDockerEnvironment (benchmark/process.rs:1280-1284), which identifies a separate path needing a controlled failure reproduction. The cited stage-neighbor is explicitly outside trial cleanup (native_driver.rs:8438-8479), so it surviving an externally killed test does not prove a trial-owned leak.

## Unresolved Decisions

none

### D1 resolution: complete files within the existing artifact bound

The accepted file-backed, bounded-memory direction remains unchanged. Inspection of the committed archive establishes an existing concrete bound: `bundle/mod.rs:1164-1166` defines `MAX_ARTIFACT_BYTES = 64 * 1024 * 1024`; insertion at `:1202-1206` rejects larger artifacts with `ArtifactTooLarge`. B11 reuses that **per-artifact** limit and its existing rejection meaning. It introduces no new numeric limit, total-trial quota, CLI/config key, schema version, dependency or public adapter seam. The existing raw source log can exceed the bound, but it cannot be admitted as a complete sealed artifact by truncating it.

Complete settled stdout/stderr up to that limit is copied, hashed and checked through bounded file IO. Above the limit, read/write exhaustion, malformed terminal evidence or unsafe/changing input fails the affected evidence/settlement explicitly; private raw diagnostics remain unmodified where available, and no successful partial artifact/seal or fabricated Agent score is published. This is a repair of evidence handling inside the existing Companion contract. Changing the bound, public evidence shape, authority or new resource policy is outside B11 and would require a new plan.

Normal complete capture and the separate timeout diagnostic-copy path remain distinct. The latter's 1 MiB/file, 16 MiB-total, 256-file private-copy limits are not changed. Host process-adapter profile caps are not changed. C010 and P002 have one root repair, one combined predicate and shared before/after evidence, while retaining separate source records.

P003 has a resolved ownership disposition (Returned to shaping), not permission for this workflow to write an implementation ledger. P001 has a registered operator-evidence deferral; no spending or real credentials are authorized. P005 remains an explicit Cannot confirm exclusion with a concrete evidence gap.

## Closure Batches

These bounded batches are the sealed plan scope. Production edits require the separate exact-digest apply invocation; the user's D1 agreement resolves the design choice and does not replace that invocation. Severity order is Major batches first, then the Minor helper repair; final verification follows every admitted source change. Shared files do not merge predicates. B6 may reuse B9's validated helper when that creates a real dependency, so the execution order can put B9 immediately before B6.

### Batch B1: connection.opi-file-credential-projection

**Closure predicate**: A matching File/FromFile connection supplies its resolved value under the same private environment key named in Opi configuration; the value never enters config, grader or exported evidence.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B1.1: project-resolved-file-credential

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-001` (C001).
- **Decision**: `fix:project-resolved-file-credential`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Reuse the existing fallback name when building the Opi agent_env map. Add file-reference projection and selected-client CLI coverage; retain mismatch, privacy and credential cleanup cases. No new credential source or config field.
- **Closure predicate**: A matching File/FromFile connection supplies its resolved value under the same private environment key named in Opi configuration; the value never enters config, grader or exported evidence.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` — FAIL: config references OPI_EVAL_AGENT_KEY; projected env names=[]; actual None instead of the dummy resolved value.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver file_credential_projection -- --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B2: native.boundary-failure-stops-dispatch

**Closure predicate**: After an integrity/provenance rejection or unconfirmed owned/credential/partial-capture cleanup, settlement preserves the failure and the next queued trial never starts; valid Agent zero and settled provider outcomes retain their specified treatment.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B2.1: cancel-after-unsettled-boundary

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-002` (C002).
- **Decision**: `fix:cancel-after-unsettled-boundary`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Propagate the existing run cancellation token from the failed boundary after recording observed settlement; reuse supervision for in-flight owned work. Cover both failure reasons in one predicate matrix, with queued and already-running behavior. Do not cancel merely because a valid Agent outcome is zero.
- **Closure predicate**: After an integrity/provenance rejection or unconfirmed owned/credential/partial-capture cleanup, settlement preserves the failure and the next queued trial never starts; valid Agent zero and settled provider outcomes retain their specified treatment.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` — FAIL: cleanup-unconfirmed probe starts 2 trials; provenance-mismatch probe starts 2 trials and the second seals. Expected only the first dispatch. Valid-zero control PASS.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver boundary_failure_stops_dispatch -- --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B3: offline.verify.reject-trial-links

**Closure predicate**: An intact two-trial source verifies; replacing a trial directory with a symlink to identical bytes yields an explicit nonzero verification failure without following the link or reducing a successful inventory.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B3.1: reject-linked-trial-inventory

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-003` (C003).
- **Decision**: `fix:reject-linked-trial-inventory`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Reject unsafe trial directory entries at discovery and retain existing source containment semantics for verify/regrade. Add directory-link, file-replacement and ordinary-directory controls at the public CLI seam; preserve source bytes.
- **Closure predicate**: An intact two-trial source verifies; replacing a trial directory with a symlink to identical bytes yields an explicit nonzero verification failure without following the link or reducing a successful inventory.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_offline_probe plan_offline_verify_rejects_trial_symlink -- --exact --nocapture --test-threads=1` — FAIL: intact source verifies 2 bundles; after replacing trial-opi-native with a symlink to preserved identical bytes, verify exits 0 with outcome=verified, 1 bundle and failures=[]. Corrected exact filter: 0 passed/1 failed.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver verify_rejects_trial_links -- --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B4: offline.regrade.selected-products

**Closure predicate**: A verified pi-only run produces one source-linked derived grade from its unchanged pi-only material, invokes zero Agents/providers and preserves the original source bytes; missing selected material still rejects.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B4.1: load-only-source-selected-agent-material

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-004` (C004).
- **Decision**: `fix:load-only-source-selected-agent-material`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/regrade.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Determine the required product set from verified source control/subject identities before using existing load_for. Retain both-product loader semantics for unrelated consumers and retain selected identity validation.
- **Closure predicate**: A verified pi-only run produces one source-linked derived grade from its unchanged pi-only material, invokes zero Agents/providers and preserves the original source bytes; missing selected material still rejects.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` — FAIL: source run exit 0; regrade exit 2, agent set must be exactly [opi, pi], found [pi].
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver pi_only_native_regrade -- --exact --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B5: report.full-selected-count

**Closure predicate**: Full selected counts distinct expanded task identities, independent of subject/repetition count; corpus, scope, planned, attempted and valid coverage remain consistent for full, partial-full and explicit subset runs.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B5.1: count-expanded-full-task-selection

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-005` (C005).
- **Decision**: `fix:count-expanded-full-task-selection`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Derive full selected count from validated planned trial task identities in ReportBuilder. Keep the full task_ids=[] durable representation and existing subset semantics; no schema or format change is needed.
- **Closure predicate**: Full selected counts distinct expanded task identities, independent of subject/repetition count; corpus, scope, planned, attempted and valid coverage remain consistent for full, partial-full and explicit subset runs.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_offline_probe plan_offline_full_selection_reports_selected_tasks -- --exact --nocapture --test-threads=1` — FAIL: valid full run has corpus=1, planned=2, attempted=2, valid=2, but selected=0 instead of 1. Corrected exact filter: 0 passed/1 failed.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver full_selection_reports_selected_tasks -- --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B7: native.outcome.reject-unobserved-client-startup

**Closure predicate**: A non-timeout process failure with no native Opi startup evidence is invalid infrastructure and unscored; a real started client task failure/zero and benchmark-valid timeout remain scored with their native meanings.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B7.1: separate-client-startup-failure-from-task-zero

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-008` (C008).
- **Decision**: `fix:separate-client-startup-failure-from-task-zero`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/agent/opi.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Require positive existing native Opi event/startup evidence before classifying an ordinary nonzero exit as a task failure. Preserve typed provider-error precedence and timeout semantics; use the native Agent/AgentStart envelope already emitted by Opi. Do not key classification solely on GLIBC stderr strings or make every nonzero exit invalid. Update synthetic fixtures to carry truthful native startup evidence and add absence/malformed/provider/zero/timeout controls. No new launch schema or Opi product serialization is proposed.
- **Closure predicate**: A non-timeout process failure with no native Opi startup evidence is invalid infrastructure and unscored; a real started client task failure/zero and benchmark-valid timeout remain scored with their native meanings.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` — FAIL: empty client stdout plus loader-only stderr produces sealed graded-native/reward=0/agent_failed=true and run exit 0; valid native-zero control PASS.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver native_client_startup_classification -- --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B8: acceptance.native-api-identity

**Closure predicate**: Offline acceptance recognizes Opi Anthropic and pi anthropic-messages as their exact native representations of the selected protocol, while provider/model and invalid-product/API cases remain strict.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B8.1: interpret-api-by-native-product

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-009` (C009).
- **Decision**: `fix:interpret-api-by-native-product`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Make interpretation product-specific in the existing verifier and give fixtures actual native shapes. Retain all tamper, endpoint/control and evidence-completeness checks. Preserve Opi serialization and the selected first-connection policy. Update the public CLI subprocess negative test and reject unknown representations.
- **Closure predicate**: Offline acceptance recognizes Opi Anthropic and pi anthropic-messages as their exact native representations of the selected protocol, while provider/model and invalid-product/API cases remain strict.
- **Red-before**: `python C:\Users\Luiz\AppData\Local\Temp\opi-phase19-remediation-plan-hldp4vqx\probe-native-api-offline.py C:\Users\Luiz\AppData\Local\Temp\opi-phase19-remediation-plan-hldp4vqx\source` — FAIL exit 1: Opi Anthropic rejected as provider response differs; pi anthropic-messages accepted; unknown Opi API and pi Anthropic rejected.
- **Green-after**: `python crates/opi-eval/scripts/test_verify_local_acceptance.py` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B11: native.capture.complete-settled-streams

**Closure predicate**: For both products, a completed native stdout/stderr stream within the existing 64 MiB per-artifact limit is preserved byte-for-byte with complete digests and terminal interpretation, including streams above 1 MiB; copy/hash/scan/verification use fixed-size buffers and parsing retains bounded current/terminal state rather than a whole event history. Oversize, unsafe, changing, unreadable or unwritable evidence cannot become a truncated successful seal or an invented Agent score; existing privacy and terminal-only rules hold.
**Dependencies**: B2 (failed boundary propagation), B7 (startup classification) and B8 (product-specific API predicate); B6 consumes the corrected capture path. This Major batch precedes B9/B6 and the final B10 gates.
**Verification union**: public `full_capture_` regression matrix; focused pi/bundle/scanner unit tests; bundle recomputation/tamper tests; offline-verifier unit and CLI-negative tests; retained terminal-only, native-zero, timeout, privacy and ownership controls; final workspace gates. The unchanged committed red is reused rather than rerun without a source change.

#### Fix B11.1: stream-complete-native-capture-with-existing-artifact-bound

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-010` (C010); `phase19-pi-glm53-e079780-20260926t192024z` + `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` + `P19-AUD-002` (P002).
- **Decision**: `fix:stream-complete-native-capture-with-existing-artifact-bound`
- **Verification status**: Confirmed for C010; Partially confirmed for P002 (effect confirmed; profile-cap attribution corrected).
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/agent/pi.rs`, `crates/opi-eval/src/bundle/mod.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/bundle_recompute.rs`, `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Replace native prefix-as-evidence handling with safe, complete, file-backed capture under the existing per-artifact bound; carry full-file digest/length separately from any private diagnostic preview; stream through the necessary consumers below. Reuse existing failure boundaries, artifact keys, native terminal fields and manifest identity rules.
- **Closure predicate**: For both products, a completed native stdout/stderr stream within the existing 64 MiB per-artifact limit is preserved byte-for-byte with complete digests and terminal interpretation, including streams above 1 MiB; copy/hash/scan/verification use fixed-size buffers and parsing retains bounded current/terminal state rather than a whole event history. Oversize, unsafe, changing, unreadable or unwritable evidence cannot become a truncated successful seal or an invented Agent score; existing privacy and terminal-only rules hold.
- **Red-before**: `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` — FAIL in `plan_full_capture_preserves_large_settled_streams`: both 1,050,479-byte source streams seal only 1,048,576 bytes; pi becomes `import-evidence-incomplete`, and valid report coverage is 1/2. The 1,966-byte controls retain complete bytes and valid 2/2 coverage. Exact archive/probe/log hashes remain listed below.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver full_capture_ -- --nocapture --test-threads=1` — PASS with the same complete-byte and final-outcome assertions for both products, plus the failure matrix below; not run during planning.
- **Transfer rule**: Port the existing isolated capture assertion into maintained `native_driver.rs`; use genuine per-product native fixtures when extending the matrix. The earlier synthetic probe proves byte retention and pi settlement, not actual Opi/pi-client acceptance. B6 remains separate.

Implementation and failure requirements are part of this single closure batch:

1. **Acquire settled files safely.** In `runner/experiment.rs`, preserve directory containment, no-follow open, regular-file/single-link validation and stopped-owned-writer precondition. Track the opened file's identity/size and reject observed growth, replacement or drift; do not trust a size precheck alone. Read at most the shared artifact limit plus a checked overflow observation. Missing or malformed required terminal evidence cannot be upgraded to a completed client. Keep raw sources unchanged.
2. **Stage atomically without raw-stream Vec copies.** Add only a private file/reader insertion path to the existing `RunBundle` state machine, reusing artifact-role/source/classification/causal-edge checks, duplicate checks, sealed-state rejection, atomic temporary-file publication and durability syncing. Compute size and SHA-256 during fixed-buffer copy; enforce the existing 64 MiB per-artifact limit before publishing the artifact. Preserve the existing bytes API for its current consumers; do not add a trait, public storage abstraction or second bundle format. An incomplete write/sync/rename cannot register a successful artifact or publish a manifest. Remove only transaction-owned temporary files.
3. **Interpret the whole stream with bounded retained state.** Refactor pi parsing to share one event/state validation core between the current in-memory host input and native file input. Retain the current schema/terminal/provider/model checks and count exactly one terminal; process one event at a time instead of collecting all lines/events. Bound decoded current/terminal payload and total input by the existing artifact admission bound; explicitly reject overflow/parse failure rather than truncating a line or inventing a smaller event-limit configuration. Opi startup and typed provider interpretation retain B7's semantics. `stdout_sha256`/`stderr_sha256` describe the complete validated file, and truncation fields must truthfully distinguish full evidence from a private preview. Terminal-only mode still omits raw streams while preserving its existing terminal facts, diagnostic and classification.
4. **Scan every byte before seal.** Replace the staged canary scanner's whole-file read in `runner/experiment.rs:404-425` with bounded chunk scanning; retain enough overlap for canaries spanning chunks, including a canary longer than a buffer. Memory for matching is bounded by configured canary inputs, not stream length. File/read/identity errors fail evidence admission rather than being treated as a clean scan. A leak or failed privacy check blocks seal/export and propagates the already-approved affected-run cancellation; no secret bytes enter error text. Preserve behavior of other callers of the shared scanner.
5. **Hash sealed evidence incrementally.** Change only the covered-artifact hashing passes in `bundle/mod.rs` (seal `:1291-1294`, verify `:1435-1443`) to bounded readers; retain reservation equality, sidecar comparison, unmanifested/missing-file rejection, immutable source handling and exact digest/manifest semantics. Native newly admitted stream files must not be reloaded wholesale by these passes. Existing unrelated sidecar/metadata parsing and old bundle identity semantics remain unchanged.
6. **Keep offline acceptance bounded.** In `verify-local-acceptance.py`, stream hashes for the native logs and walk validated native events without building the whole `rows/events/messages` history. Preserve B8's API interpretation, complete-terminal/tool matching, digest checks and strict rejection of malformed/ambiguous evidence. Tool correlation state and each event/terminal payload remain bounded by the admitted artifact size; reuse the existing per-artifact contract rather than inventing a new public cutoff. Bound/check the native inputs before decoding, and cover >1 MiB acceptance and tamper through the existing Python test file. Do not broaden this into unrelated upstream-material or build artifact processing.
7. **Preserve explicit failures and coverage.** Oversize, disk exhaustion, permission/read failures, malformed/unsafe input or failed flush/scan/seal are evidence/infrastructure failures, never graded Agent failures. If evidence sufficient for an invalid diagnostic seal remains, label it invalid; otherwise keep an explicit unsealed failure and private diagnostics. Never claim complete capture or successful seal for a prefix, and never rewrite an old sealed bundle or repair its hash. B2 governs cancellation for privacy/integrity/unknown-cleanup breaches; an ordinary settled resource rejection must not be misreported as a breach or an Agent score.

Required tests add behavior absent from the current suite:

- Public full capture for both products below 1 MiB, above 1 MiB, and at the existing artifact-size boundary; verify exact raw/sealed digest and length, final terminal/valid-zero/provider-error interpretation, `verify` and deterministic `report`. Cover both many small events and a large single event within the admitted bound.
- Reject a stream just above 64 MiB and a source that grows during reading; retain unsafe symlink/hardlink/nonregular-file negatives and missing/malformed-terminal cases. The negative must exercise the stream path, not merely assert the limit constant.
- Inject read/write/sync failure into a bounded reader/writer or isolated staging path and assert no successful partial artifact, manifest or score. Use deterministic fault injection rather than filling the real disk or changing host quotas.
- Exercise a canary after the old 1 MiB boundary, across chunk boundaries, longer than one buffer and in stderr; assert privacy failure, no exported secret and no subsequent affected dispatch. A late reader error must not be accepted as a clean privacy scan.
- Cover streamed bundle insertion/seal/verify and source tamper with existing bundle recomputation cases; retain the bytes-insertion conformance cases. Test bounded read requests/state directly with a recording reader, rather than a brittle timing or RSS-only assertion.
- In Python, verify complete native-shaped >1 MiB observations, malformed/truncated/oversize/tampered negatives and the public verifier's rejection exit. Run the entire modified Python test file. Preserve separately authorized six-real-trial acceptance as P001's exclusion.

Document the complete-capture and resource-failure behavior in both Companion READMEs, including the existing per-artifact bound and the distinct timeout-copy contract. Read the complete Unreleased section and add the visible fix there. Test impact: **add/update** capture/parser/bundle/scanner/verifier regressions; **retain** host-profile, privacy, strict-identity, terminal-only, timeout and native-outcome guards; **delete** none.

### Batch B9: native.witness.material-prerequisite-diagnostics

**Closure predicate**: Missing family, empty tasks and malformed profile inputs fail before staging with a diagnostic naming the invalid prerequisite; valid three-family inputs still support existing witnesses.
**Dependencies**: none
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B9.1: validate-witness-material-before-staging

- **Finding source(s)**: `phase19-pi-glm53-e079780-20260926t192024z` + `f2e7acaffe724a0fa6dbf28ca767b163a3cd664a905a17e6b17924d1fedd582c` + `P19-AUD-004` (P004).
- **Decision**: `fix:validate-witness-material-before-staging`
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-eval/tests/support/native_witness_material.rs`, `crates/opi-eval/tests/native_driver.rs`
- **Change kind**: test-only
- **Change**: Validate the existing three-family prerequisite before tempdir/git/material effects and reuse the result in the helper. Add a focused default deterministic negative matrix; retain both opt-in real native witnesses. Do not introduce arbitrary witness timeouts or a second logging feature.
- **Closure predicate**: Missing family, empty tasks and malformed profile inputs fail before staging with a diagnostic naming the invalid prerequisite; valid three-family inputs still support existing witnesses.
- **Red-before**: N/A — Test-helper-only repair; the unchecked as_object()/next()/as_str() chain supplies a direct static proof of the missing input diagnostic. No production runtime behavior is changed.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver native_witness_material_rejects_incomplete_families -- --exact --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B6: native.report.actual-client-failure-matrix

**Closure predicate**: Both actual Opi/pi clients traverse public native run, settlement and offline report against a loopback fixture provider: final auth/rate/protocol failures are attempted/invalid/unscored, native zero stays valid, and transient 429 followed by success yields a final valid outcome.
**Dependencies**: B1, B2, B7, B11, and B9 if its helper is reused
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: add/update; retain existing guards.

#### Fix B6.1: add-real-client-public-report-matrix

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-007` (C007).
- **Decision**: `fix:add-real-client-public-report-matrix`
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`
- **Change kind**: test-only
- **Change**: Add one explicitly ignored native_provider_failure_client_report_matrix using existing pinned native witness and scripted-provider helpers, with distinct terminal/retry cases rather than a duplicate protocol cross-product. Keep default tests credential-free and do not invoke remote inference. Reuse helper improvements from B9 where necessary.
- **Closure predicate**: Both actual Opi/pi clients traverse public native run, settlement and offline report against a loopback fixture provider: final auth/rate/protocol failures are attempted/invalid/unscored, native zero stays valid, and transient 429 followed by success yields a final valid outcome.
- **Red-before**: N/A — Test-only missing integration proof: current source call-path inspection establishes the absent assembled seam; existing production behavior is not edited by this batch.
- **Green-after**: `cargo test --offline --locked -p opi-eval --test native_driver native_provider_failure_client_report_matrix -- --exact --ignored --nocapture --test-threads=1` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.
### Batch B10: verification.final-workspace-gates

**Closure predicate**: All final workspace gates pass for the final relevant source using known system tools, with real Unix tests executed and no environment-shim or archive-metadata false failures.
**Dependencies**: all source batches, including B11
**Verification union**: the matching regression below plus the deduplicated final verification union. Test impact: retain; none to production tests.

#### Fix B10.1: reverify-with-known-system-tools

- **Finding source(s)**: `phase19-codex-gpt6-e079780-20260926t093335z` + `7be02185ae78c5c6b6ffcc508c231e1c42a7614e92cd79526d751eff2bc607a8` + `P19-AUD-006` (C006).
- **Decision**: `fix:reverify-with-known-system-tools`
- **Verification status**: Partially confirmed
- **File(s)**: none; validation environment/evidence only.
- **Change kind**: metadata
- **Change**: Correct the validation command environment, resolve and lease the existing external Cargo cache, and run the verification union after source repairs. Do not change Bash runtime behavior or the existing test just to accommodate the shadowing executable. For tests requiring commit objects, supply read-only Git object access while preserving the archive as the source under test.
- **Closure predicate**: All final workspace gates pass for the final relevant source using known system tools, with real Unix tests executed and no environment-shim or archive-metadata false failures.
- **Red-before**: `cargo test --offline --locked -p opi-coding-agent --test tools_read_write_edit_bash bash_tool_env_inheritance_reporting -- --exact --nocapture` — FAIL: 0 passed/1 failed under inherited login PATH; same binary PASS: 1 passed/0 failed with PATH=/usr/bin:/bin. Full final workspace union not run during planning.
- **Green-after**: `env PATH=/home/luiz/.cargo/bin:/usr/bin:/bin bash scripts/opi-impl-smoke.sh full` — PASS with the same discriminating assertion and retained controls; not run during planning.
- **Transfer rule**: for an archive-only probe, port the minimal new assertions into the named maintained test seam, without copying the whole probe scaffold. Preserve its before/after predicate; record both commands and exact outcomes.

## Exclusions

Every excluded source is still represented exactly once in the disposition sidecar. Exclusion is not closure of its requirement.

| Source | Finding ID | Disposition | Current evidence/authority |
|---|---|---|---|
| P001 | `P19-AUD-001` | Deferred by registered source | Explicit exclusion from source apply: retain as Deferred by registered source, never Closed. B8 and D1 do not automatically close P001. No provider dispatch, credential access or evidence rewrite is authorized by this plan. |
| P003 | `P19-AUD-003` | Returned to shaping | Return exact correction evidence to opi-implement, the sole ledger writer. No ledger edit or new competing ledger in this remediation. Do not add a meaningless test alias or mark a default deterministic test ignored merely to satisfy stale prose. |
| P005 | `P19-AUD-005` | Cannot confirm | No speculative cleanup change. Retain Cannot confirm pending a fresh no-model fault reproduction on the current diagnostic path with exact pre/post container IDs. Do not broaden discovery, prune Docker globally, or merge this finding into the proven run-dispatch cancellation fix merely because the family matches. |

## Plan-stage checks and evidence identity

Current committed source was exported with `git archive 075a046ebc585265c28a6c3f778dfcc501338ddc` into a unique temporary archive, without a Git worktree. Its faithful Linux extraction is `/tmp/opi-remediate-p19-reseal-nlhf3rj2/source`; `python3 scripts/opi-doc-check.py` there returned exit 0, `opi documentation contracts: PASS`. The initial Windows extraction could not preserve the `.claude/skills` directory alias and its doc check returned exit 1; the Linux extraction preserves the archive-declared symlink and resolves that export defect without changing source. The current index raw-byte digest matches the header. The committed scope reader reports no stored-hash mismatches and confirms both registered supplemental sources.

Evidence reuse check: `git diff --quiet eb930fbcfc16384555edcbae6e6026ba7b763c37..075a046ebc585265c28a6c3f778dfcc501338ddc -- . ':(exclude)docs/snapshots/phase19/assurance/remediation.plan.md' ':(exclude)docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl'` returned exit 0. All 15 planned source paths also match the retained probe archive byte-for-byte, and all nine probe/log SHA-256 entries below were rechecked successfully. The change-scope contract requires retaining evidence against the same relevant state rather than repeating unchanged checks. Accordingly, behavioral red-before commands and outcomes below are retained evidence, not newly executed tests; the dispositions keep their truthful `verified_at=eb930fbcfc16384555edcbae6e6026ba7b763c37`. No conclusion from a history run, earlier result, eval artifact or alternate plan is used as proof. Every source remains bound to the current index and owning findings digest.

Retained red-before source archive (evidence state `eb930fbcfc16384555edcbae6e6026ba7b763c37`): `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-remediation-plan-hldp4vqx/source`.
Its native Linux path is `/mnt/c/Users/Luiz/AppData/Local/Temp/opi-phase19-remediation-plan-hldp4vqx/source`.
The initial ZIP extraction materialized symlinks as pointer files; reconstructing archive-declared symlinks and executable modes fixed that export artifact without changing committed source bytes. The archive documentation checker then passed. This setup issue is not a product finding.

Cargo reused and leased `/mnt/opi-audit-e-20260926/opi/cargo-targets/opi-8abcf38f9871653e`, whose marker identifies Linux rustc 1.97.1 and the original workspace. `python scripts/opi-cargo-cache.py status` passed; no cache clean/delete or per-session target was used.

The following probes exist only in the archive/scratch and use dummy credentials, local fixtures and isolated temporary run roots. They do not prove actual native-client or paid-provider acceptance. They copy existing test scaffolding solely to access private fixture helpers; production integration must port only their new cases. All commands use the recorded source state; matching green assertions are still pending source repair.

| Check | Observed planning outcome |
|---|---|
| `cargo test --offline --locked -p opi-eval --test plan_native_probe --test plan_offline_probe --test plan_capture_probe --no-fail-fast plan_ -- --nocapture --test-threads=1` | Native: 11 pass/4 intended failures. Capture: 10 pass/1 intended failure. Initial offline: 10 pass/1 intended pi-only failure plus 2 probe-setup failures; those setup failures are not accepted as product red evidence. The corrected exact checks below supersede only their two setup results. The broad filter also ran retained plan-related controls. |
| Corrected `plan_offline_verify_rejects_trial_symlink` and `plan_offline_full_selection_reports_selected_tasks` | See each disposition's observed red; exact corrected filters were rerun after fixing only the probe's trial ID and required selection.reason. |
| Python native API predicate probe | Expected red, exit 1; product-shaped positive/negative controls recorded above. |
| Exact Bash env filter | Reproduced red with inherited PATH; identical binary passed with system PATH. Whole final workspace gates remain unverified. |
| `python scripts/opi-doc-check.py` in faithful source archive | PASS. |

Probe and diagnostic artifacts are listed with exact SHA-256 below so their test-only provenance is inspectable. Temporary artifacts are reproduction aids, not new authoritative findings or a competing progress ledger. If they are unavailable later, reconstruct the named assertions against a fresh committed archive and observe red again before editing production.

| Scratch-relative artifact | SHA-256 |
|---|---|
| `source/crates/opi-eval/tests/plan_native_probe.rs` | `aa8836b3eaa260ab12cae39fe98140937d18d1e7f9b3409b5ce756b2a90f6ea1` |
| `source/crates/opi-eval/tests/plan_offline_probe.rs` | `ecb785f83ef4a76442ee583c089bdde8b5137398b431e96bba8d0ed8aeb32782` |
| `source/crates/opi-eval/tests/plan_capture_probe.rs` | `cb20fafef411294743b764da65a03d3ce928b66f81e15fdecc07747edd6f904e` |
| `probe-native-api-offline.py` | `9ac6b315572baa45f3678dcf9e4462d231535fbf5d6ab5589a28f1f6a1aaa3fe` |
| `probe-native-api-offline.log` | `d23f6823c801fe843410268c0d19669faa27ab7c021ad601d60df8e77ba2fac0` |
| `regression-probes.log` | `fe54877f215f26560c36392749011c4301b307c858f51058ddc5885dd9c440d0` |
| `offline-corrected-probes.log` | `db32ac6ac68985b691fd3494b5bddb9c34a343c15cc546918011383a5bb133d9` |
| `native-test_inherited.log` | `3737601b24defc6a6799072df6c22e38c655403f876a57c2aef92b1d0c041b49` |
| `native-test_minimal.log` | `0d34d1eb757d03c62be0309837c0a33349c8555518c59cf6a92e180ab985b7d5` |

## Change inventory and final verification

Current outgoing change: documentation/assurance metadata only, the fixed plan Markdown; the paired JSONL is retained byte-identically. This continuation changes the head binding and records its verification without changing D1 or any approved closure scope. Reused red-before evidence remains bound to its original verified commit and proven identical relevant source bytes at the new remediation head. No English/Chinese maintained product document is edited; the assurance contract fixes a single Markdown path and has no localized plan counterpart. Future user-visible changes require Unreleased entries after reading the complete existing Unreleased section. If a repair changes maintained explanatory documentation, edit its English/Chinese counterpart in the same batch. No manifests, feature defaults, dependency edges or lockfile changes are proposed.

Derived apply surfaces: local Companion runtime; public CLI run/verify/regrade/report; evidence/seal containment; Python offline-verifier behavior; native witness/test helper; gate environment. Their union requires focused CLI/integration/conformance regressions, Python unit and subprocess negatives, and the final workspace gates explicitly required by C006. B11 adds bounded streaming through native capture, pi parsing, privacy scan, bundle insertion/seal/verify and the offline acceptance reader; its exact causal paths and failure tests below are included in this union. No broader storage or parser refactor is admitted.

```sh
# Planning artifact checks, from the repository:
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance
python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl
python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md
python scripts/opi-doc-check.py
git diff --check

# Apply-stage union, after the final relevant source changes; not run by this plan:
python crates/opi-eval/scripts/test_verify_local_acceptance.py
cargo test --offline --locked -p opi-eval --test native_driver native_provider_failure_client_report_matrix -- --exact --ignored --nocapture --test-threads=1
cargo test --offline --locked -p opi-eval --test native_driver remediation_native_stage_deadlines -- --exact --ignored --nocapture --test-threads=1
cargo test --offline --locked -p opi-eval --test native_driver native_verifier_only_regrade_witness -- --exact --ignored --nocapture --test-threads=1
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Use known system-tool resolution and the repository's external cache workflow for these commands. The two existing ignored witness checks are required if B9 changes their shared prerequisite path; supply verified three-family material, no live model, and preserve logs. B6's actual client matrix additionally needs exact native clients and the local fixture provider; missing material is a concrete verification gap, never a skipped pass. Workspace all-targets covers every newly ported default Rust case; do not repeat default binaries once equivalent evidence has passed on the same relevant state. The new Python tests must include a real CLI negative invocation, not only helper calls. B11's default tests and unit tests are included in the same workspace all-targets pass; this includes the modified native_driver and bundle_recompute binaries and pi/bundle/scanner library cases. Run each new or modified exact regression filter as its matching green proof when not already covered by an equivalent recorded final-state run.

Test impact now: **none** to repository test files; **retain** all repository tests and the identified isolated probes. Planned test impact: **add/update** the named regressions/native-shape fixtures, streaming capture/bundle/privacy cases and test-helper diagnostics; **retain** privacy, unsafe-path, paired/subset, native-zero, timeout, native-witness and strict-identity guards; **delete** none.

After required checks pass, rerun only invalidated evidence. Preserve unrelated work; recheck the head and all three worktree inventories at handoff. A final result must not declare Phase 19 conformant: P001, P003 and P005 have explicit remaining ownership/evidence boundaries, and the live audit aggregate remains FAIL until a later independently admitted audit.

## Criterion cross-reference matrix

This table projects every current requirement record without changing its state. Criterion identity is the exact `(path, sha256, citation)` tuple. Unequal citation strings remain separate rows even when acceptance IDs resemble each other; the table makes no voting or consensus inference. Source aliases above resolve full run IDs and sidecar digests. Related state disagreements (notably outcome, report, verify, regrade, lifecycle and independent placement) are decided by the current evidence above, not by which peer reports met.

| Source | Requirement | Criterion path | SHA-256 | Exact citation | Source state | Source finding IDs |
|---|---|---|---|---|---|---|
| C | P19-AC-002 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.1 / P19-AC-002 | met | none |
| C | P19-AC-003-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.1 / P19-AC-003-native | met | none |
| C | P19-AC-008-outcome | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.1 / P19-AC-008-outcome | partially-met | P19-AUD-008 |
| C | P19-AC-006-fixture | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.2 / P19-AC-006-fixture | met | none |
| C | P19-AC-010-credentials | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.2 / P19-AC-010-credentials | met | none |
| C | P19-AC-004-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.3 / P19-AC-004-native | met | none |
| C | P19-AC-005-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.4 / P19-AC-005-native | met | none |
| C | P19-AC-001-prepare | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.5 / P19-AC-001-prepare | met | none |
| C | P19-AC-007-material | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.5 / P19-AC-007-material | met | none |
| C | P19-AC-007-selection | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.6 / P19-AC-007-selection | met | none |
| C | P19-AC-001-plan | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.6 / P19-AC-001-plan | met | none |
| C | P19-AC-008 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.7 / P19-AC-008 | partially-met | P19-AUD-005, P19-AUD-007 |
| C | P19-AC-009-verify | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.7 / P19-AC-009-verify | partially-met | P19-AUD-003 |
| C | P19-AC-009-regrade | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.8 / P19-AC-009-regrade | partially-met | P19-AUD-004 |
| C | P19-AC-010-lifecycle | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.9 / P19-AC-010-lifecycle | partially-met | P19-AUD-002 |
| C | P19-AC-011 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.10 / P19-AC-011 | met | none |
| C | P19-AC-013 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.10 / P19-AC-013 | partially-met | P19-AUD-004 |
| C | P19-AC-001-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-001-live | partially-met | P19-AUD-010 |
| C | P19-AC-003-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-003-live | partially-met | P19-AUD-009, P19-AUD-010 |
| C | P19-AC-004-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-004-live | partially-met | P19-AUD-009, P19-AUD-010 |
| C | P19-AC-005-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-005-live | partially-met | P19-AUD-009, P19-AUD-010 |
| C | P19-AC-006-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-006-live | partially-met | P19-AUD-009 |
| C | P19-AC-012 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7; task 19.11 / P19-AC-012 | partially-met | P19-AUD-006, P19-AUD-009, P19-AUD-010 |
| C | P19-001-doctor | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 / task 19.5 | met | none |
| C | P19-001-acquire | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4-5 / task 19.5 | met | none |
| C | P19-002-settlement | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5-6 / task 19.1 | partially-met | P19-AUD-008 |
| C | P19-002-failed-capture | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 6 / task 19.1 | met | none |
| C | P19-003-no-legacy-route | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5 / task 19.1 | met | none |
| C | P19-004-declarations | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5 / task 19.3 | met | none |
| C | P19-005-patch | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5 / task 19.4 | met | none |
| C | P19-006-config | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 / task 19.2 | partially-met | P19-AUD-001 |
| C | P19-006-controls | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 / task 19.2 | met | none |
| C | P19-006-terminal | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4,6 / task 19.2 | met | none |
| C | P19-006-unknown | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4,6 | met | none |
| C | P19-007-inventory | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5 / task 19.6 | met | none |
| C | P19-007-plan | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4-5 / task 19.6 | met | none |
| C | P19-007-bounds | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 / tasks 19.2,19.6,19.9 | met | none |
| C | P19-008-coverage | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 6 / task 19.7 | met | none |
| C | P19-008-metrics | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 6 / task 19.7 | partially-met | P19-AUD-005, P19-AUD-008 |
| C | P19-008-pairs | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 6 / task 19.7 | met | none |
| C | P19-009-derived | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 6 / task 19.8 | partially-met | P19-AUD-004 |
| C | P19-010-isolation | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 / tasks 19.2,19.9 | met | none |
| C | P19-010-cancel | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 9 / task 19.9 | partially-met | P19-AUD-002 |
| C | P19-010-recovery | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4,6 / task 19.9 | met | none |
| C | P19-011-ci-policy | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 8 / task 19.10 | met | none |
| C | P19-012-offline-verifier | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 8 / tasks 19.10-19.11 | partially-met | P19-AUD-009 |
| C | P19-012-provenance | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 8 / tasks 19.10-19.11 | met | none |
| C | P19-012-final-gates | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 8 / task 19.11 | partially-met | P19-AUD-006 |
| C | P19-013-placement | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 3 / P19-AC-013 | met | none |
| C | P19-013-contracts | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 9 / P19-AC-013 | met | none |
| C | P19-OPTIONAL-ORACLE | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 5 | met | none |
| C | P19-OPTIONAL-CONNECTIONS | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 4 | met | none |
| C | P19-ARCH-MINIMUM | `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | PRIN-001, PRIN-002, PRIN-003, PLACE-001, PLACE-004 | met | none |
| P | P19-AC-002 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-002; task 19.1 acceptance scenario P19-AC-002 | met | none |
| P | P19-AC-003-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-003; task 19.1 acceptance scenario P19-AC-003-native | met | none |
| P | P19-AC-008-outcome | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-008; task 19.1 acceptance scenario P19-AC-008-outcome | met | none |
| P | P19-AC-006-fixture | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-006; task 19.2 acceptance scenario P19-AC-006-fixture | met | none |
| P | P19-AC-010-credentials | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-010; task 19.2 acceptance scenario P19-AC-010-credentials | met | none |
| P | P19-AC-004-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-004; task 19.3 acceptance scenario P19-AC-004-native | met | none |
| P | P19-AC-005-native | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-005; task 19.4 acceptance scenario P19-AC-005-native | met | none |
| P | P19-AC-001-prepare | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-001; task 19.5 acceptance scenario P19-AC-001-prepare | met | none |
| P | P19-AC-007-material | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-007; task 19.5 acceptance scenario P19-AC-007-material | met | none |
| P | P19-AC-007-selection | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-007; task 19.6 acceptance scenario P19-AC-007-selection | met | none |
| P | P19-AC-001-plan | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-001; task 19.6 acceptance scenario P19-AC-001-plan | met | P19-AUD-003 |
| P | P19-AC-008 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-008; task 19.7 acceptance scenario P19-AC-008 | met | none |
| P | P19-AC-009-verify | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-009; task 19.7 acceptance scenario P19-AC-009-verify | met | none |
| P | P19-AC-009-regrade | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-009; task 19.8 acceptance scenario P19-AC-009-regrade | met | P19-AUD-003, P19-AUD-004 |
| P | P19-AC-010-lifecycle | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-010; task 19.9 acceptance scenario P19-AC-010-lifecycle | met | P19-AUD-005 |
| P | P19-AC-011 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-011; task 19.10 acceptance scenario P19-AC-011 | met | none |
| P | P19-AC-013 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-013; task 19.10 acceptance scenario P19-AC-013 | met | none |
| P | P19-AC-001-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-001; task 19.11 acceptance scenario P19-AC-001-live | partially-met | P19-AUD-001 |
| P | P19-AC-003-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-003; task 19.11 acceptance scenario P19-AC-003-live | partially-met | P19-AUD-001 |
| P | P19-AC-004-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-004; task 19.11 acceptance scenario P19-AC-004-live | partially-met | P19-AUD-001 |
| P | P19-AC-005-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-005; task 19.11 acceptance scenario P19-AC-005-live | partially-met | P19-AUD-001 |
| P | P19-AC-006-live | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-006; task 19.11 acceptance scenario P19-AC-006-live | partially-met | P19-AUD-001, P19-AUD-002 |
| P | P19-AC-012 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Section 7 P19-AC-012; task 19.11 acceptance scenario P19-AC-012 | not-met | P19-AUD-001, P19-AUD-002 |
