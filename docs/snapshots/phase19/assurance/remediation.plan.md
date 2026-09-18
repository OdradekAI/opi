# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Remediation head**: `2b6538a1b0bca0146ebaeac2d80352a5929fe22b`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]
**Unresolved decisions**: none

## Admission and scope

The current schema-2 index, revision 4, validated with `audit-set: PASS` before
semantic inputs were read. Aggregate verdict remains `FAIL`. The strict union
contains 11 source keys (9 Codex and 2 pi); the repeated textual IDs 001 and
002 remain distinct. No reviewer is canonical. The two member audit heads are
committed ancestors of the remediation head (enforced by audit-set validation).
All source finding records are retained unchanged in the disposition sidecar's
`source_record`; current verification is recorded separately.

Worktree-only inventory: `committed=none-for-this-task`. At admission all four
fixed remediation outputs were tracked and clean; their earlier contents were
not consulted. This plan replaces only the two fixed plan files. The existing
result files are untouched and are not closure evidence for this plan.

Committed source was inspected in unique `git archive` exports, not worktrees:

- Windows: `C:\Users\Luiz\AppData\Local\Temp\opi-p19-remediation-plan-f05600b20e304b76a44dc968edf649f8`.
- WSL ext4: `/tmp/opi-p19-remediation-plan.uxwgp3hu`.

Plan-stage runtime checks ran in the WSL export. Five temporary probe tests
were appended there only; production bytes and modes remained those of the
remediation head. Tests used fake Agents/runners, dummy credentials and local
fixtures; no paid provider or real auth file was used. The owned cancellation
probe process group was explicitly terminated after observing the failure.
Cargo reused `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e` with
`CARGO_INCREMENTAL=1`; no per-session target or cache deletion was used.

The registered English Phase19 delivery source owns meaning. Existing source
requirements, not historical ledger verdicts or peer agreement, govern the
decisions below. This plan neither changes the implementation ledger nor
declares Phase conformance. The fixed plan has no localized counterpart; its
schema-prescribed single Markdown/JSONL pair needs no duplicate `.zh` file.

## Indexed source bindings

| Reviewer | Exact audit run | Audit head | Findings SHA-256 |
|---|---|---|---|
| codex/gpt6 | `phase19-codex-gpt6-d915e4a-20260918t084558z` | `d915e4a22509c0f60c2651dbccfd054f0abc73d4` | `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` |
| pi/glm53 | `phase19-pi-glm53-ad67cb8-20260918t101920z` | `ad67cb8e1d9e824f270b549e4fdea0072cd7b539` | `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` |

## Current Finding Verification

All source and final severities remain Major; schedule each at that severity.
The full run binding above applies to the reviewer labels below.

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-001 | Confirmed | Major → Major: mandatory closure unproved or violated | `prepare.acquire-selected-materials` / `prepare.materials` | B9 | `fix:acquire-selected-pinned-materials-with-explicit-opi-source` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-002 | Confirmed | Major → Major: mandatory closure unproved or violated | `prepare.bounded-acquisition-settlement` / `prepare.supervision` | B8 | `fix:supervise-bounded-material-acquisition` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-003 | Confirmed | Major → Major: mandatory closure unproved or violated | `controls.selected-client-capability` / `controls.comparability` | B3 | `fix:reject-required-unsupported-sampling` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-004 | Confirmed | Major → Major: mandatory closure unproved or violated | `report.preserve-output-control-comparability` / `controls.comparability` | B4 | `fix:derive-offline-comparability-from-sealed-control-evidence` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-005 | Confirmed | Major → Major: mandatory closure unproved or violated | `recovery.immutable-complete-classification` / `lifecycle.recovery` | B5 | `fix:read-only-recovery-from-frozen-planned-inventory` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-006 | Confirmed | Major → Major: mandatory closure unproved or violated | `replacement.fresh-resolved-paired-attempt` / `lifecycle.recovery` | B6 | `fix:replace-resolved-pair-under-fresh-identities` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-007 | Confirmed | Major → Major: mandatory closure unproved or violated | `privacy.stop-affected-native-dispatch` / `lifecycle.authority` | B2 | `fix:cancel-run-on-native-privacy-breach` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-008 | Confirmed | Major → Major: mandatory closure unproved or violated | `acceptance.reject-fixture-substitution` / `acceptance.evidence` | B7 | `fix:require-live-first-connection-in-sealed-acceptance` |
| `phase19-codex-gpt6-d915e4a-20260918t084558z` / P19-AUD-009 | Partially confirmed | Major → Major: mandatory closure unproved or violated | `acceptance.current-dynamic-verification-union` / `acceptance.evidence` | B11 | `verify:retain-current-native-verification-union` |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-001 | Confirmed | Major → Major: mandatory closure unproved or violated | `packaging.direct-exec-script-modes` / `packaging.unix` | B1 | `fix:restore-executable-mode-on-two-direct-entrypoints` |
| `phase19-pi-glm53-ad67cb8-20260918t101920z` / P19-AUD-002 | Partially confirmed | Major → Major: current witness retention is defective; global historical absence is unproved | `acceptance.three-family-native-artifact-witness` / `acceptance.evidence` | B10 | `verify:retain-three-family-native-artifact-witnesses` |

## Unresolved Decisions

none. D1 has an explicit owner decision. D2 and D3 use existing product and
verification authority with concrete implementation choices below. Readiness
means the closure work is specified, not that its green checks have run.
Unavailable external materials, failed native checks or missing retained raw
evidence during apply leave the corresponding finding Not closed; they never
become a waiver or fabricated PASS.

### D1: owner-selected local source contract

The user explicitly selected "采用本地源码＋固定 commit（推荐）" in this
planning session. That resolves the public-input choice for B9; it does not
authorize apply or a model call.

Add paired optional prepare arguments `--opi-source <local-git-directory>` and
`--opi-commit <full-40-hex-commit>`. Require both when a selected Opi package
must be built, reject a half-specified pair, and never infer either from the
caller's directory or current HEAD. A verified warm Opi package keeps the
existing path. A pi-only experiment needs no Opi input or Opi build.

Resolve the exact declared local commit without changing its checkout, export
only that commit into preparation-owned staging, and verify its Cargo.lock
against the declared workspace-input hash. Build from that exported source;
bind the declared commit and resulting executable/package hashes into existing
local-origin build/material identity records. Uncommitted source files do not
enter the build. Do not fetch a missing Opi commit implicitly.

The same public prepare invocation acquires selected pi, corpus and runner
sources using their existing committed repository/revision pins, invokes only
the selected producer stages, validates complete inventory and image identities,
and publishes verified cache entries. The current script's unconditional
two-Agent inputs and caller-cwd Cargo build must be removed from that path.
No manual producer-stage sequence is an acceptable closure.

This choice introduces one local source input, with no new remote Opi locator,
registry, dependency or experiment-schema field. The alternative is an explicit
remote Opi repository plus exact commit; that has a larger public acquisition
contract and is not selected. B9 enumerates CLI/help/callers, producer pins,
fixtures, bilingual documentation and negative/positive cold-cache checks.

### D2: resolved implementation choice

Use the existing crate-private `ProcessSupervisor` for the one acquisition
invocation, with a fixed 3,600-second wall-clock budget and 1,048,576 retained
bytes per stdout/stderr stream. The time budget encloses clone, download and
build descendants together; it is not restarted per stage. These conservative
internal limits bound a stalled preparation without borrowing trial deadlines.
They are not performance promises and successful large preparations may need
a later invocation that reuses already verified cache entries.

No new flag, configuration field, environment override, public Rust argument
or durable schema is needed. Keep the existing public synchronous facade and
use the established CLI runtime/signal-cancellation pattern internally. Capture
overflow is drained with explicit truncation diagnostics; it does not itself
invalidate a correctly verified material. Timeout/cancellation and unverified
cleanup fail preparation and prevent manifest publication. B8 specifies the
repair and its proof. Selecting private constants under the existing bounded
supervision requirement does not require a separate product decision.

### D3: fresh no-model material and retained verification

A read-only WSL check observed Docker Server `29.7.2`, Linux, x86_64. The
engine is available; material completeness and native behavior remain unproved.
The active environment does not bind `OPI_EVAL_NATIVE_WITNESS_MATERIAL`,
`OPI_EVAL_NATIVE_CLIENT_MATERIAL` or `OPI_EVAL_OUTPUT_CONTROL_OUT`. This says
nothing about whether material files exist elsewhere.

Use fresh materials produced after B9, under a unique WSL ext4 directory,
rather than require the user to locate an earlier output. This is an
implementation choice under the existing no-model preparation/verification
scope, not presumed approval of the unanswered optional material-path question.
The registered source's sections 8 and 10 permit deterministic development
without an API key and identify credentials as runtime inputs. Existing public
source/image downloads are ordinary acquisition; model calls remain excluded.

Seed the committed pin/producer/profile/integration set into that private cache,
including B9's updated producer digests. Explicitly select the local Opi source
`/mnt/d/Luiz/Odradek/opi` at commit
`2b6538a1b0bca0146ebaeac2d80352a5929fe22b` for its unchanged Opi client build;
the repaired Companion is separately built from the approved changes and bound
by its executable hash plus source-head/change hashes. Invoke public prepare
for the three existing pinned corpus families from an independent directory.
Use only the registered candidate tasks after their unchanged resource/image
checks admit them; an infeasible task blocks that check rather than causing
silent task substitution. Record the exact resulting paths and hashes before
native execution. Preserve the existing image-admission semantics of prepare;
any required image acquisition uses explicit immutable pins outside task
dispatch, without modifying shared images or relaxing resource checks.

Construct the existing client JSON from those verified local packages: `opi`,
`node`, `pi_bundle`, `experiment`, `opi_package` and `pi_package`. A fresh
absolute output path supplies `OPI_EVAL_OUTPUT_CONTROL_OUT`. Bind each official
family's material JSON separately to `OPI_EVAL_NATIVE_WITNESS_MATERIAL` for B10.
B11's controlled lifecycle fixtures derive private material from the same
verified runner/tool/integration inputs; they do not overwrite official
corpora, frozen manifests or benchmark results. The test-side builder records
their distinct source/image/content identities and rejects runner pin drift.

Retain each ignored test's unique root before its first external effect, using
the existing tempfile retention facility. Set the standard TMPDIR to the
private evidence directory; do not add a product flag or environment contract.
Write command argv, exit code, stdout/stderr and input hashes before assertions,
retain failed attempts too, and print the retained path. Resource cleanup still
removes only owned processes/containers and temporary credentials. No successful
test may delete its raw evidence. B10 owns the three official native contrasts;
B11 owns the remaining current dynamic union and final gates.

The current static inspection establishes that all three official witness
filters remove their successful roots and the protocol matrix drops its
temporary root. This independently confirms a present retention gap, so the pi
finding is Partially confirmed. It does not establish that no witness ever ran
elsewhere. The indexed source's six saved live trials remain acknowledged.
No historical eval output is searched or loaded, and no new real-provider run
is part of these batches. Current real acceptance and Phase conformance remain
separate claims requiring the registered operator handoff.

## Evidence and disagreement resolution

The peers used different criterion granularity. Exact criterion triples
`(path, sha256, citation)` are retained in Appendix A; citations that differ
remain separate rows. A broad `met` row does not override a reproduced failing
sub-obligation. In particular:

- pi AC-001 versus Codex AC-001-03: production acquisition actually returns
  `not wired`; warm-cache fixture preparation is valid counter-evidence for
  verification, but does not prove cold acquisition.
- pi AC-006/008 versus Codex control findings: exact required-sampling and
  run→offline probes fail; passing request-projection tests do not establish
  effective control comparability.
- pi AC-010 versus Codex recovery/privacy findings: probes observe original
  report overwrite, repeated replacement identities, and post-breach dispatch.
- pi AC-012 versus Codex fixture-admission finding: the production verifier
  accepts synthetic evidence; this does not refute that real saved trials exist.
- pi EXIT-GATES versus Codex AC-012-04: faithful Linux modes reproduce both
  direct-exec failures. No current workspace PASS is claimed.

No history, alternate audit source, recurrence classification, previous task
conclusion or external eval artifact is used to decide a disposition.

## Closure Batches

Each batch below has one falsifiable closure predicate and one root repair.
Shared files/family keys do not merge batches. B3/B4 remain separate because
pre-dispatch capability admission and offline preservation require distinct
observations. B5/B6 remain separate because recovery and replacement have
different effects. B10's official-family contrasts and B11's broader native
verification union also remain separate closure batches.
All batches are planned, not implemented; every green-after remains not-run.

### Batch B1: Restore executable delivery modes

**Closure predicate**: Both direct-entry scripts can be invoked by production paths on a faithful Linux checkout.
**Dependencies**: none
**Verification union**: Run Q1/Q2 in a new Linux source export carrying the proposed modes. Verify the Git index mode with `git ls-files --stage` on the two explicit paths when materializing this approved fix. No new tests are needed. Include affected-target clippy and the final union below for runtime changes.

#### Fix B1.1: Restore executable delivery modes

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` + `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` + `P19-AUD-001`.
- **Decision**: `fix:restore-executable-mode-on-two-direct-entrypoints`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/scripts/scripted-provider.py`, `crates/opi-eval/tests/fixtures/external-locks/local-cache/scripts/materialize-external-locks.sh`.
- **Change kind**: metadata.
- **Change**: Change only the two Git file modes from 100644 to 100755; preserve script bytes. Retain existing shebang/direct-exec behavior. Do not bypass the defect by chmodding fixture copies inside tests. In a Windows checkout, materialize the approved mode changes in the Git index using the two explicit paths and preserve unrelated staged content. Committing still requires an explicit user request.
- **Closure predicate**: Both direct-entry scripts can be invoked by production paths on a faithful Linux checkout.
- **Red-before**: `cargo test -p opi-eval --locked --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --exact --nocapture; cargo test -p opi-eval --locked --test native_driver supervised_acquisition_runs_the_pinned_materializer -- --exact --nocapture` — FAIL: both tracked modes are 100644. Provider-fixture expected 0, got 1; acquisition expected 0, got 2 with Permission denied (os error 13). Each exact test failed on the pristine archive.
- **Green-after**: `cargo test -p opi-eval --locked --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --exact --nocapture; cargo test -p opi-eval --locked --test native_driver supervised_acquisition_runs_the_pinned_materializer -- --exact --nocapture` — PASS in a Linux export with intended 100755 modes; staged/committed Git modes, when later explicitly authorized, also equal 100755. Observed: not-run.
- **Test impact**: retain.

### Batch B2: Stop native dispatch after a privacy breach

**Closure predicate**: A native canary breach cancels affected dispatch before the next trial and prevents publication while retaining private observed evidence.
**Dependencies**: none
**Verification union**: Port the privacy probe into native_driver; add the no-leak control and assert the first seal failure plus absence of a second job/Agent dispatch. Run existing native signal/partial-capture cancellation filters and the owning native_driver binary. Include affected-target clippy and the final union below for runtime changes.

#### Fix B2.1: Stop native dispatch after a privacy breach

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-007`.
- **Decision**: `fix:cancel-run-on-native-privacy-breach`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: At the native canary detection boundary, cancel the existing run-level token; preserve the failed seal receipt and let the existing owned-process/resource supervision settle. Check cancellation before starting a subsequent wave. Keep planned missing coverage visible. Do not turn ordinary task failure or score zero into a privacy event.
- **Closure predicate**: A native canary breach cancels affected dispatch before the next trial and prevents publication while retaining private observed evidence.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver p19_plan_privacy_failure_stops_dispatch -- --exact --nocapture` — FAIL: width=1; trial-opi-native settled failed:at-evidence, then trial-pi-native emitted stage=start and also executed. Corrected probe assertion independently failed.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver p19_plan_privacy_failure_stops_dispatch -- --exact --nocapture` — PASS: no second native dispatch; owned in-flight execution settles, invalid evidence remains private/unsealed, and planned missing coverage stays visible. Observed: not-run.
- **Test impact**: add/update.

### Batch B3: Admit required controls against selected client capabilities

**Closure predicate**: Unsupported requested sampling cannot become a strict native comparison: required values reject before effects; optional unsupported values remain explicit.
**Dependencies**: none
**Verification union**: Port the required-sampling probe; cover temperature/top_p, omitted/unknown, Opi, pi Anthropic, pi OpenAI and pi-only positive admission. A required capability check belongs at native admission, so fixture proof construction stays independent and cannot mask the behavioral assertion. Run experiment_contract and native_driver, including output-control positive/negative cases. Include affected-target clippy and the final union below for runtime changes.

#### Fix B3.1: Admit required controls against selected client capabilities

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-003`.
- **Decision**: `fix:reject-required-unsupported-sampling`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/experiment_contract.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Use the existing Opi/pi projection for the selected protocol in native validate/plan/run admission; do not add sampling support to Opi. Reject required temperature/top_p when any selected client cannot express them, before material execution/Agent launch. Include optional requested unsupported sampling in native comparison facts. Preserve hermetic conformance semantics and value/omitted/unknown distinctions. Reuse the same small crate-private capability logic at the real consumers. Native test scenarios unrelated to sampling must declare omitted sampling rather than depend on the defect.
- **Closure predicate**: Unsupported requested sampling cannot become a strict native comparison: required values reject before effects; optional unsupported values remain explicit.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver p19_plan_required_sampling_rejects_before_dispatch -- --exact --nocapture` — FAIL: run exited 0, dispatched both subjects, sealed two trials, and reported comparable despite Opi Unsupported temperature.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver p19_plan_required_sampling_rejects_before_dispatch -- --exact --nocapture` — PASS: typed native-admission rejection before any trial directory/job; temperature/top_p and both native protocols/clients receive discriminating coverage. Observed: not-run.
- **Test impact**: add/update.

### Batch B4: Preserve output-control restrictions during offline reporting

**Closure predicate**: For the same verified sealed trial set, offline comparability never upgrades an absent/unsupported output control to comparable.
**Dependencies**: B3
**Verification union**: Extend the optional-output regression through public report. Cover genuine proven output controls, absent proof, tampered proof/seal, and legacy evidence without proof. Run report_contract, report_output_containment, end_to_end_report and the focused native regression. Include affected-target clippy and the final union below for runtime changes.

#### Fix B4.1: Preserve output-control restrictions during offline reporting

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-004`.
- **Decision**: `fix:derive-offline-comparability-from-sealed-control-evidence`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/report_contract.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Read verified sealed control/output-control.json presence and bound configuration/subject facts; reuse current control meaning where needed. Missing required evidence stays unknown/unsupported. Do not infer capability from requested numeric equality or trust mutable run-report.json. Preserve prior bundles byte-for-byte, deterministic offline operation and known native-zero semantics. No new schema is proposed.
- **Closure predicate**: For the same verified sealed trial set, offline comparability never upgrades an absent/unsupported output control to comparable.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver p19_plan_offline_preserves_unsupported_output -- --exact --nocapture` — FAIL: run exited 1 with unsupported-control:max_output_tokens; report exited 0 and coverage[0].comparability was comparable.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver p19_plan_offline_preserves_unsupported_output -- --exact --nocapture` — PASS: offline coverage preserves the same unsupported reason without modifying source bundles or trusting mutable run-report.json. Observed: not-run.
- **Test impact**: update.

### Batch B5: Make recovery an immutable classification view

**Closure predicate**: Recovery changes no original run/control/bundle/report bytes and classifies every frozen planned trial including unstarted ones.
**Dependencies**: none
**Verification union**: Port both recovery CLI cases into assembled_run; compare complete original-file digests before/after, include a missing planned root, completed and effect-unknown states, and keep pairing_and_integrity coverage. Update expectations that accidentally depended on overwrite. Include affected-target clippy and the final union below for runtime changes.

#### Fix B5.1: Make recovery an immutable classification view

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-005`.
- **Decision**: `fix:read-only-recovery-from-frozen-planned-inventory`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/tests/pairing_and_integrity.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Remove persistence into run-report.json from the recover branch. Build the recovery view from the durable frozen planned inventory when present and the verified resolved input otherwise; reject inconsistent identity rather than relabel evidence. Join observed roots with planned trials, retaining effect-unknown/cancelled/failed states. Return the existing CLI classification view on stdout, with no replay or fresh durable execution record.
- **Closure predicate**: Recovery changes no original run/control/bundle/report bytes and classifies every frozen planned trial including unstarted ones.
- **Red-before**: `python3 cli-probes.py (Appendix B; recover-preserves-report and recover-enumerates-unstarted)` — FAIL: run-report SHA changed from 38bc645be79871cf902da2a3ffe05d9628597d17cc8a06e0796e7379b5818695 to e7efaed24e1cd905e7c309646ba4483ceaa6b3af5e1ed48044d17e92ff29d7c8; trial count 2 -> 0. A fresh planned root returned recovery=[]. Both recover calls exited 1.
- **Green-after**: `cargo test -p opi-eval --locked --test assembled_run recovery_ -- --nocapture` — PASS: port Appendix B cases to focused recovery_* regressions; classify completed/cancelled/failed/effect-unknown/unstarted and preserve all original bytes. Observed: not-run.
- **Test impact**: add/update.

### Batch B6: Replace the resolved paired group under fresh identities

**Closure predicate**: Each replacement derives one new attempt identity for the whole resolved pair, preserving original evidence and frozen scope, including selection-generated trials.
**Dependencies**: B5
**Verification union**: Port both replacement CLI cases into assembled_run. Cover two independent replacements from one original config/plan, selection-generated IDs, exact paired membership, unchanged task/protocol/controls, unrelated groups and old-file digest preservation, tampered original plan rejection and fresh-root ownership. Include affected-target clippy and the final union below for runtime changes.

#### Fix B6.1: Replace the resolved paired group under fresh identities

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-006`.
- **Decision**: `fix:replace-resolved-pair-under-fresh-identities`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/runner/plan.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Verify the original plan/source digest and perform the existing selection expansion before locating the target. Replace the target group structurally using the resolved trial list, not TOML line rewriting. Generate a fresh collision-resistant attempt token using existing dependencies/identity utilities and derive new experiment/group/trial identities for both sides. Recheck normal preflight and planned bounds; never compare the derived replacement identity with the old manifest as if it were the original run. Do not add a second replacement engine or identity registry.
- **Closure predicate**: Each replacement derives one new attempt identity for the whole resolved pair, preserving original evidence and frozen scope, including selection-generated trials.
- **Red-before**: `python3 cli-probes.py (Appendix B; replacement-new-identities and replacement-frozen-selection)` — FAIL: two fresh roots each exited 0 but reused trial-opi-1.r2/trial-pi-1.r2, group-hello.r2, local-paired-hermetic and identical digest 0e62b270c6cc6705e4e7110c6f42985698336ad9d31a0350d322f590174e2b4e. Frozen selection replacement exited 2: target not a declared trial.
- **Green-after**: `cargo test -p opi-eval --locked --test assembled_run replacement_ -- --nocapture` — PASS: port both Appendix B cases to replacement_* regressions; unique paired attempt identities, preserved old roots, unchanged task/connection/controls, and rejection of tampered source plans. Observed: not-run.
- **Test impact**: add/update.

### Batch B7: Reject fixture substitution at real-acceptance verification

**Closure predicate**: Real acceptance rejects fixture or mismatched first-connection evidence even when its seals and six-trial structure are internally consistent.
**Dependencies**: B3, B4
**Verification union**: Port the fixture-rejection probe and add script unit negatives for missing/native-fixture mode and wrong provider/protocol/endpoint/model. Retain valid-zero and integrity/capture-view negative coverage. Run the exact Python unit file and the production subprocess negative via native_driver. A genuine operator handoff, if separately supplied, needs offline verification before any real-acceptance claim; it is not replaced by B10/B11's no-model evidence. Include affected-target clippy and the final union below for runtime changes.

#### Fix B7.1: Reject fixture substitution at real-acceptance verification

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-008`.
- **Decision**: `fix:require-live-first-connection-in-sealed-acceptance`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: After verifying sealed experiment bytes, require execution.mode=live and the registered first connection: Zhipu provider, anthropic-messages, https://open.bigmodel.cn/api/anthropic, glm-5.3-flash. Cross-check actual preserved Agent/terminal/provider/tool evidence with existing build/source identities. Keep loopback exact-client output-control probe evidence as conformance evidence; do not mistake it for the live trial connection. Turn the existing synthetic acceptance-positive e2e into a rejection regression; pure checker unit positives may remain structurally synthetic but cannot claim real acceptance. Genuine acceptance requires its separate operator handoff. No bypass flag or fabricated provenance.
- **Closure predicate**: Real acceptance rejects fixture or mismatched first-connection evidence even when its seals and six-trial structure are internally consistent.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver p19_plan_acceptance_rejects_fixture_evidence -- --exact --nocapture` — FAIL: production verifier exited 0 and accepted=true for all six deterministic native-fixture/scripted-local trials.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver p19_plan_acceptance_rejects_fixture_evidence -- --exact --nocapture` — PASS: exit 2 for fixture/missing mode or wrong first connection; the script unit suite preserves valid-zero and structurally valid control cases, without claiming live acceptance from a stand-in. Observed: not-run.
- **Test impact**: update.

### Batch B8: Bound and settle material acquisition

**Closure predicate**: Every started acquisition settles under its deadline or cancellation, retains bounded diagnostic output, reports owned-tree cleanup, and never publishes a manifest after unsuccessful settlement.
**Dependencies**: B1 for the executable fixture. B9 producer stages remain descendants of this supervised invocation.
**Verification union**: Port Q4 into native_driver and exercise both SIGINT and SIGTERM on Linux. Add focused runner tests with an internally injected short duration and small capture limits for a stalled child, a noisy successful child, spawn failure and ordinary nonzero exit. No public testing knob. Check absent manifest on failure, bounded acquisition.log with truncation markers, unchanged verified cache bytes, and no surviving owned child. Run the modified library test filters and native_driver; include affected-target clippy and the final union below.

#### Fix B8.1: Bound and settle material acquisition

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-002`.
- **Decision**: `fix:supervise-bounded-material-acquisition`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/cli/prepare.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Replace only the acquisition `Command::output` path with existing structured `SpawnSpec`/`ProcessSupervisor` execution and the CLI's existing signal-cancellation helper. Use D2's fixed whole-invocation timeout and per-stream caps. Project the required build-tool environment over an empty base without forwarding provider credentials. Record exit/timeout/cancel, truncation and observed cleanup in the existing acquisition diagnostic log before returning. Add no public limit arguments or result schema; B9 separately extends PrepareArgs for the owner-selected source input. Do not publish material.json after a failed or uncertain settlement; retain already verified shared-cache entries and remove only invocation-owned staging under the producer's ownership rules. Do not add a second process supervisor or change unrelated doctor observations.
- **Closure predicate**: Every started acquisition settles under its deadline or cancellation, retains bounded diagnostic output, reports owned-tree cleanup, and never publishes a manifest after unsuccessful settlement.
- **Red-before**: `cargo test -p opi-eval --locked --test native_driver p19_plan_acquisition_cancel_cleans_child -- --exact --nocapture` — FAIL: after SIGINT to prepare and a one-second observation window, its pinned materializer remained alive; the probe then terminated only its owned process group. Current source also has no deadline or bounded capture around that invocation.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver p19_plan_acquisition_cancel_cleans_child -- --exact --nocapture` — PASS: prepare settles cancellation and the owned child is gone. Focused acquisition tests additionally prove timeout, bounded output, failure diagnostics and warm-cache preservation under the same production supervision path. Observed: not-run.
- **Test impact**: add/update.

### Batch B9: Acquire selected pinned materials from one local entrypoint

**Closure predicate**: From an independent directory and a pin-only filesystem cache, public prepare obtains and verifies the selected Agent packages, complete corpus and runner environment without manual producer stages, an implicit Opi source, unselected Agent builds or any model call.
**Dependencies**: B1, B8; owner-selected D1 above. Existing explicit pin seeding and image/resource admission remain prerequisites; no task image or Agent is started by prepare.
**Verification union**: Add a production-materializer subprocess test using local pinned Git repositories/archives and bounded fake build tools to prove orchestration without downloads or inference. Exercise each corpus/runner family and Opi-only, pi-only and paired selections. Port the public cold-cache path into native_driver, including an unrelated cwd, source checkout with uncommitted sentinel bytes, unavailable/wrong commit, mismatched Cargo.lock, half-specified arguments, package-resource drift, producer-pin drift and a warm-cache repeat. Only declared committed bytes may reach the Opi build; pi-only must work without an Opi source or Cargo invocation. Run these tests plus the existing independent-directory, runner-source and runtime-package cases. After D3 material preparation, replay the same public entrypoint with real pins from an independent directory and retain the raw acquisition evidence; fixture success is not that native witness.

#### Fix B9.1: Wire selected acquisition and explicit Opi source

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-001`.
- **Decision**: `fix:acquire-selected-pinned-materials-with-explicit-opi-source`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/main.rs`, `crates/opi-eval/src/cli/prepare.rs`, `crates/opi-eval/src/runner/prepare.rs`, `crates/opi-eval/scripts/materialize-external-locks.sh`, `crates/opi-eval/scripts/build-agent-artifacts.sh`, `crates/opi-eval/scripts/native-smoke.sh`, `crates/opi-eval/external-locks/static/linux-x86_64.json`, `crates/opi-eval/scripts/test_prepare_acquisition.py`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/examples/local-evaluation/README.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement the D1 CLI arguments and equivalent typed facade fields, validate them before acquisition, and pass a structured selected-material plan to the pinned materializer. Export and build the exact local Opi commit in owned staging, checking its lock hash; take the executable only from Cargo's compiler-artifact stream. Make the existing builder run only requested products and use the supplied source as its actual build directory. Carry the explicit Opi commit through its existing native-smoke caller and recorded invocation. Record true source/build identities with existing identity formats. For pi, obtain its exact committed source and pinned Node distribution, verify declared blobs and locked dependencies, preserve the complete runtime package and its resources, and reject unverifiable build inputs. For each selected corpus, acquire the full pinned checkout and validate commit/tasks-tree before task selection. For Harbor/Pier, acquire the pinned source, check the uv.lock blob/hash and use pinned uv with locked dependency installation at the final canonical runner path so virtualenv shebangs remain valid. Reuse the existing repository mapping for those two admitted runners. Deduplicate missing identities by owning producer so executable/project/lock/interpreter entries trigger one build. Recompute required executable/runtime paths after acquisition, since a cold Agent path becomes a package directory. Reuse normal final verification and inventory freezing; no missing identity may be silently dropped or covered by an unverified cache entry.
- **Pin and caller impact**: Recompute the materializer's LF-normalized digest and add the reused builder to the existing static-lock producer list; verify both seeded bytes before executing them. Keep the static-lock schema, upstream commits, Cargo.lock and image pins unchanged. Seed the builder alongside existing pins in the walkthrough. Reuse existing build-input integrity checks and fail on drift; do not silently introduce a mutable dependency source. Preserve the native-smoke two-product path and its identity records while allowing selected-product local builds. Add CLI help and bilingual README examples, replace the walkthrough's manual Agent/corpus/runner steps with one prepare invocation, and record the deliberate PrepareArgs/CLI change under Unreleased. The walkthrough has no Chinese counterpart; the two crate READMEs carry the synchronized user-facing contract.
- **Closure predicate**: From an independent directory and a pin-only filesystem cache, public prepare obtains and verifies the selected Agent packages, complete corpus and runner environment without manual producer stages, an implicit Opi source, unselected Agent builds or any model call.
- **Red-before**: `python3 cli-probes.py` (Appendix B, acquisition case) — FAIL: the production prepare-mode exits 2 and reports agents/pi, corpora/terminal-bench-2.1 and runners/harbor as not wired. The source also confirms unconditional two-product builder inputs and a caller-cwd Cargo invocation.
- **Green-after**: `python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py` and `cargo test -p opi-eval --locked --test native_driver prepare_ -- --nocapture` — PASS: the production script/public CLI acquire selected material from pinned local fixture inputs and reject source/pin mismatch before publication; the owning full native_driver binary also passes. The separate real-material cold-cache replay remains required under D3. Observed: not-run.
- **Test impact**: add/update.

### Batch B10: Retain and execute the three official native artifact contrasts

**Closure predicate**: Each of TB2.1, TB3 and DeepSWE has a current, retained correct/incorrect contrast through its production handoff and unchanged pinned native grader, with bound inputs, changed submitted artifact and responsive native outcome.
**Dependencies**: B1, B8, B9 and D3's fresh pinned material preparation. Keep this predicate distinct from B11's larger verification union.
**Verification union**: Execute the three exact ignored artifact-witness filters below against their corresponding freshly prepared material. Read their retained reports, native job/result trees, command records and bound input/artifact identities; exit zero alone is insufficient. TB2.1 must preserve the real graded environment; TB3 must show only its declared collected artifact entering the separate verifier; DeepSWE must show the committed patch and base/pristine-verifier identity. The correct and incorrect native results must differ as the registered witnesses require. Recheck retained files after each test process exits.

#### Fix B10.1: Preserve official witness evidence and run each family

- **Finding source(s)**: `phase19-pi-glm53-ad67cb8-20260918t101920z` + `fa4d61c10efa6cb9d68146fab9f5f28eaf970393f941ba97119a1c68e5fd8301` + `P19-AUD-002`.
- **Decision**: `verify:retain-three-family-native-artifact-witnesses`.
- **Verification status**: Partially confirmed: current committed filters delete successful evidence; the source's universal historical-absence claim is not established.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/examples/local-evaluation/README.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`.
- **Change kind**: test-only.
- **Change**: Replace PID-derived reusable/deleted witness roots with unique retained temporary roots in the three existing ignored filters. Persist commands, exits and captured stdout/stderr before assertions; preserve all native trees and material hashes, including on failure. Keep the original oracle/nop behavior, benchmark definitions and outcome assertions. Document exact opt-in commands, per-family material binding and retained-root discovery in the walkthrough and both crate READMEs. Execute the official contrasts after material verification; do not substitute controlled lifecycle tasks or deterministic stand-in runners for these checks.
- **Closure predicate**: Each of TB2.1, TB3 and DeepSWE has a current, retained correct/incorrect contrast through its production handoff and unchanged pinned native grader, with bound inputs, changed submitted artifact and responsive native outcome.
- **Red-before**: Static inspection at the exact remediation head, reproduced in Appendix C, finds successful-path `fs::remove_dir_all(&root)` in all three filters (functions at lines 2059, 2108, 2181). This is a test/evidence-retention repair, so no failing live-model or native-grader run is required before the test-only edit. No global absence assertion is made.
- **Green-after**: `cargo test -p opi-eval --locked --test native_driver native_tb21_artifact_witness -- --ignored --exact --nocapture`; repeat exact filters `native_tb3_artifact_witness` and `native_deepswe_patch_witness`, each with its own official material binding — all PASS, retained raw outputs survive process exit, the correct/incorrect native outcomes and artifact provenance satisfy the predicate, and no remote inference request occurs. Observed: not-run.
- **Test impact**: update; execute all three modified ignored filters and the owning native_driver binary.

### Batch B11: Retain the remaining current native verification union

**Closure predicate**: Current exact clients, native lifecycle/offline operations and independent cold preparation have attributable retained no-model execution evidence, and every required final workspace gate passes for the final relevant source state.
**Dependencies**: B1-B10. B10's official-family contrasts are referenced by hashes without being merged into this source disposition. No workspace gate can substitute for an ignored native check.
**Verification union**: Execute the real-client three-protocol and output-control matrices, real Docker owned-resource cancellation, pinned-runner stage deadlines, verifier-only native regrade, a real grader/export canary-isolation case and an independent-directory cold prepare/run/report replay. Use loopback scripted provider responses and synthetic secrets only. Retain all raw evidence and run the final workspace gates below. Test helpers cannot read real credentials or become live merely because credentials exist.

#### Fix B11.1: Make native verification reproducible and retain its raw results

- **Finding source(s)**: `phase19-codex-gpt6-d915e4a-20260918t084558z` + `01c32b1bc87f7a4a3cb42c965b817f2c4c6d229343303673ddeb149d3e0b77ac` + `P19-AUD-009`.
- **Decision**: `verify:retain-current-native-verification-union`.
- **Verification status**: Partially confirmed: Q1/Q2 fail on the current head and the current protocol test drops its raw root; no claim that all external evidence is absent.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`, `crates/opi-eval/examples/local-evaluation/README.md`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`.
- **Change kind**: test-only.
- **Change**: Retain protocol/cancellation test roots and persist raw command/exit/stream records before assertions, using the same existing tempfile mechanism as B10. Keep these tests explicitly ignored. Add one shared test-only material builder for the existing deadline/regrade filters and the missing real canary scenario: consume verified prepared material, keep Harbor/Pier/uv/integration pins unchanged, and create private, byte-bound controlled task/image inputs for the exact cases already asserted by those filters. The deadline cases are positive, time-cap, agent-cap, verifier-cap, both-agent, both-verifier, minimum-cap, strict-task and unbounded-task; regrade uses positive, negative and timeout. Reuse the documented stage limits and immutable Python image pin; reject unresolved images and record separate fixture identities. Never rewrite official corpus/cache entries or label these controlled cases as benchmark acceptance. Add ignored `native_credential_canary_isolation` for actual pinned runner/Docker grader and export isolation using a dummy Agent credential: prove that it reaches only the Agent/request boundary and is absent from the grader environment, collected artifact, sealed export and post-settlement credential files. Keep private diagnostic inputs separate from scanned export content. The shared test helper is exercised through those exact ignored filters; no new runtime crate, schema or public configuration is introduced.
- **Independent CLI evidence**: After B9, use a fresh pin-only filesystem cache and unrelated cwd, prepare each selected family, then run a bounded native experiment with actual pinned clients against the loopback scripted provider and report offline through the public CLI without a `--fixtures` argument. A benchmark-valid zero is permitted; invalid attribution, missing effects or grader failure is not. Preserve declared scope, all failed attempts, effective controls, material/source/build hashes and the exact preparation/run/report commands. The Opi-only, pi-only and paired orchestration distinctions have deterministic coverage in B9; the current native replay verifies the production cold path.
- **Closure predicate**: Current exact clients, native lifecycle/offline operations and independent cold preparation have attributable retained no-model execution evidence, and every required final workspace gate passes for the final relevant source state.
- **Red-before**: Q1/Q2 are observed FAIL on the current head. Appendix C statically establishes that `native_protocol_client_matrix` drops its root at exit. Missing execution evidence is not an invented failing behavioral test; these edits retain and execute test evidence without changing product behavior. The six saved live trials acknowledged by the current source remain acknowledged.
- **Green-after**: Execute all B11 exact ignored filters in Final Verification and the independent CLI replay, retain their commands/results/input hashes, then execute the exact final workspace union. Every required command passes its stated predicate; source bundles remain byte-identical through regrade, provider request logs do not grow during verifier-only operations, and owned cleanup leaves neighboring resources/shared images intact. Observed: not-run.
- **Test impact**: add/update; the new helper executes via the modified deadline/regrade tests and new canary test. Preserve default deterministic/credential-free coverage.

## Final Verification

Scope-derived layers: native admission/execution and CLI recovery/replacement;
sealed-evidence interpretation and offline reports; acceptance-checker script
behavior; Unix package modes; affected fixtures/tests; bilingual current-product
documentation and Unreleased entries; supervised preparation lifecycle;
test-only native material setup, evidence retention and explicit local checks.
Existing durable formats are reused. B9 adds the owner-selected public source
input, updates its facade/help/callers/fixtures and refreshes affected producer
digests within the existing static-lock format. No Cargo/lock/dependency,
Agent Core, benchmark semantics, registered spec or ledger changes are approved.

For each batch: implement the minimal change -> verify the exact discriminating
green check and the affected existing test binaries. Every modified test file
must be executed, not merely compiled. Preserve source bundle bytes for all
offline/recovery paths. Default test runs remain credential-free. Test impact
for this planning change is `none`; temporary red probes are not repository tests.

For the union of B1-B11 after approved implementation:

```sh
cargo test -p opi-eval --locked --test agent_integration_conformance
cargo test -p opi-eval --locked --test native_driver
cargo test -p opi-eval --locked --test experiment_contract
cargo test -p opi-eval --locked --test assembled_run
cargo test -p opi-eval --locked --test pairing_and_integrity
cargo test -p opi-eval --locked --test report_contract
cargo test -p opi-eval --locked --test report_output_containment
cargo test -p opi-eval --locked --test end_to_end_report
cargo test -p opi-eval --locked --lib runner::prepare
cargo clippy -p opi-eval --all-targets -- -D warnings
python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py
python3 -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py
python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_ci.py
python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py
python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_artifact.py
python scripts/opi-doc-check.py
git diff --check
```

Run a focused command once at its completed relevant state; later broader gates
need not duplicate it unless dependencies or evidence changed. These are future
requirements, not claims of green execution in this plan.

B10/B11's final relevant-state evidence, separate from deterministic regressions.
Bind each artifact witness to its official family material; bind controlled
lifecycle checks to D3's verified material inputs and private test-only derived
materials. Use a fresh TMPDIR under the owned evidence root and record each
printed retained path. Do not reuse a manifest with a different meaning merely
because the filters share an environment-variable name.

```sh
# Requires explicit pinned material bindings and retained raw native outputs.
cargo test -p opi-eval --locked --test native_driver native_protocol_client_matrix -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_output_control_client_matrix -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_tb21_artifact_witness -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_tb3_artifact_witness -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_deepswe_patch_witness -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_verifier_only_regrade_witness -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver native_owned_resource_cancellation -- --ignored --exact --nocapture
cargo test -p opi-eval --locked --test native_driver remediation_native_stage_deadlines -- --ignored --exact --nocapture --test-threads=1
cargo test -p opi-eval --locked --test native_driver native_credential_canary_isolation -- --ignored --exact --nocapture --test-threads=1
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The apply operator records D3's newly prepared absolute material paths and
hashes, then executes B10/B11 with the registered profile/test bounds. Both
successful and failed raw roots remain private outside the repository, with
their paths and hashes referenced by the fixed remediation result; no competing
implementation ledger or new acceptance schema is created. Any unmet host,
download, image or resource prerequisite is an explicit failed/incomplete check,
never permission to weaken pins, thresholds or evidence requirements. This plan
does not authorize a paid run or declare P19-AC-012/Phase exit satisfied.
Only a separately supplied real-acceptance handoff can be checked with
`verify-local-acceptance.py --root <supplied-redacted-root> --out <fresh-output>
--companion <current-exact-binary>` after B7. No substitute eval artifact is
silently made a remediation source.

## Exclusions

none. Every one of the 11 current source keys has one closure batch. All
remain open until their specified evidence is observed during apply. The pi
native-witness finding is only Partially confirmed: its broad historical claim
is neither adopted nor refuted. Its concrete current retention defect and
required native contrasts are handled by B10. No historical source or peer
vote is used to change a source identity or severity.

## Plan-stage checks actually observed

| Check | Exact command | Result |
|---|---|---|
| Admission | `python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase19/assurance` | PASS |
| Cache inspection | `python scripts/opi-cargo-cache.py status` | exit 0; existing external caches inspected, not cleaned |
| Q1 | `cargo test -p opi-eval --locked --test agent_integration_conformance agent_conformance_matrix_settles_every_pinned_case -- --exact --nocapture` | FAIL, 0 passed/1 failed; provider-fixture exit 1 vs 0 |
| Q2 | `cargo test -p opi-eval --locked --test native_driver supervised_acquisition_runs_the_pinned_materializer -- --exact --nocapture` | FAIL, 0 passed/1 failed; EACCES |
| Q3 | `cargo test -p opi-eval --locked --test native_driver p19_plan_ -- --nocapture --test-threads=1` | Three valid behavioral reds: sampling, offline control, fixture acceptance. Acquisition probe initially hit pin drift and privacy assertion initially used the wrong progress spelling; neither initial result counts for those two findings. |
| Q4 | `cargo test -p opi-eval --locked --test native_driver p19_plan_acquisition_cancel_cleans_child -- --exact --nocapture` | Corrected seeded pin; FAIL with live owned child after SIGINT |
| Q5 | `cargo test -p opi-eval --locked --test native_driver p19_plan_privacy_failure_stops_dispatch -- --exact --nocapture` | Corrected progress assertion; FAIL with second trial dispatch |
| Q6 | `python3 cli-probes.py` (Appendix B) | exit 1; five named cases fail: acquisition, two recovery properties, two replacement properties |
| Documentation baseline | `python3 scripts/opi-doc-check.py` in the WSL source export | PASS |
| Docker availability | `wsl -d Ubuntu --exec bash -c 'timeout 10s docker info --format "{{.ServerVersion}} {{.OSType}} {{.Architecture}}"'` | exit 0; `29.7.2 linux x86_64`; read-only prerequisite check, not a native witness |
| Plan dispositions | `python .agents/skills/_shared/scripts/validate_assurance_artifact.py dispositions docs/snapshots/phase19/assurance/remediation.plan.dispositions.jsonl` | PASS; all 11 source keys covered exactly once |
| Witness retention | Appendix C static inspection in the committed-source archive | Current retention contract FAIL: three success-path deletions and one auto-dropped protocol root; no native test was executed by this check |
| Fixed plan | `python .agents/skills/_shared/scripts/validate_assurance_artifact.py plan docs/snapshots/phase19/assurance/remediation.plan.md` | PASS as READY-FOR-APPLY; the emitted digest is reported outside this file |
| Planning documentation | `python scripts/opi-doc-check.py` | PASS |
| Planning whitespace | `git diff --check` | PASS |

Completion inventory is worktree-only: HEAD remains the remediation head;
committed=none-for-this-task; staged=[]; untracked=[]; unstaged consists only
of the two fixed plan files. Comparing 59 exported production/script/test
files with the archive found only the intended temporary native_driver test
addition; the cancellation probe child was absent after owned cleanup.

Q3-Q5 invoke production compiled from the exact remediation head with only
test-side probe additions. The Windows workspace runtime source is unchanged.
Raw logs retained in `/tmp/opi-p19-remediation-plan.uxwgp3hu`:

| File | SHA-256 |
|---|---|
| probes.log | `95ed52a0bc6eda99c3ed9ba1fbbdd1154350a3c4b43e2fca6065b25c807ce400` |
| acquisition-cancel.log | `8ee338d9c2dcba36e5dd31ad1c29a692903498cb23c2ae774702327ca440dd33` |
| privacy.log | `c03f79a8637a4fd1ecbb54b769ee0f900ae204bdb7c087a49571efa3a434cc9e` |
| cli-probes-results.json | `bd6f76f63e2d07203e5271d1045114e49fdb937b6df346dcc894b5ee7405ada6` |

The fixed plan preserves observations and the executable probe recipe below;
temporary logs are supporting evidence, not an alternate audit/plan source.

## Appendix A: criterion identity matrix

Every indexed requirement sidecar was consumed. Rows retain original states;
no peer's record is rewritten. Each citation belongs to the exact path/hash
bound in that member's metadata. Different citation strings are left unaligned.
This matrix is coverage/context, not a new audit or a vote.

| Reviewer/run binding | Requirement | Criterion path @ SHA-256 : citation | Source state | Source finding IDs |
|---|---|---|---|---|
| codex/gpt6 (run above) | P19-AC-001-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-001; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-001-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-001; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-001-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-001; sections 3–10 | partially-met | P19-AUD-001 |
| codex/gpt6 (run above) | P19-AC-001-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-001; sections 3–10 | partially-met | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-002-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-002; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-002-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-002; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-002-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-002; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-002-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-002; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-003-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-003; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-003-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-003; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-003-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-003; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-004-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-004; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-004-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-004; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-004-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-004; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-004-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-004; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-005-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-005; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-005-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-005; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-005-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-005; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-005-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-005; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-006-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-006-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | partially-met | P19-AUD-003 |
| codex/gpt6 (run above) | P19-AC-006-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-006-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | partially-met | P19-AUD-003 |
| codex/gpt6 (run above) | P19-AC-006-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-006-06 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-006-07 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-006; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-007-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-007; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-007-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-007; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-007-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-007; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-007-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-007; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-007-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-007; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-06 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | partially-met | P19-AUD-003, P19-AUD-004 |
| codex/gpt6 (run above) | P19-AC-008-07 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-008-08 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-008; sections 3–10 | partially-met | P19-AUD-004 |
| codex/gpt6 (run above) | P19-AC-009-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-009-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-009-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-009-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-009-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-009-06 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-009; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-010-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-010-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-010-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | partially-met | P19-AUD-002, P19-AUD-003 |
| codex/gpt6 (run above) | P19-AC-010-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | partially-met | P19-AUD-002, P19-AUD-007 |
| codex/gpt6 (run above) | P19-AC-010-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-010-06 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | not-met | P19-AUD-005 |
| codex/gpt6 (run above) | P19-AC-010-07 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | partially-met | P19-AUD-006 |
| codex/gpt6 (run above) | P19-AC-010-08 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-010; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-011-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-011; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-011-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-011; sections 3–10 | partially-met | P19-AUD-001 |
| codex/gpt6 (run above) | P19-AC-011-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-011; sections 3–10 | partially-met | P19-AUD-008 |
| codex/gpt6 (run above) | P19-AC-011-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-011; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-011-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-011; sections 3–10 | partially-met | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-012-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-012; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-012-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-012; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-012-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-012; sections 3–10 | partially-met | P19-AUD-008 |
| codex/gpt6 (run above) | P19-AC-012-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-012; sections 3–10 | not-assessable | P19-AUD-009 |
| codex/gpt6 (run above) | P19-AC-013-01 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-013; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-013-02 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-013; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-013-03 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-013; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-013-04 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-013; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-AC-013-05 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : P19-AC-013; sections 3–10 | met | none |
| codex/gpt6 (run above) | P19-PRIN-002 | `docs/opi-spec.md` @ `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` : PRIN-002 | met | none |
| codex/gpt6 (run above) | P19-PRIN-004 | `docs/opi-spec.md` @ `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` : PRIN-004 | partially-met | P19-AUD-003, P19-AUD-004, P19-AUD-007, P19-AUD-008 |
| codex/gpt6 (run above) | P19-PHASE-001 | `docs/opi-spec.md` @ `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` : PHASE-001 | met | none |
| codex/gpt6 (run above) | P19-AUTH-004 | `docs/opi-spec.md` @ `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` : AUTH-004 | met | none |
| pi/glm53 (run above) | P19-AC-001 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-001 | met | none |
| pi/glm53 (run above) | P19-AC-002 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-002 | met | none |
| pi/glm53 (run above) | P19-AC-003 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-003 | partially-met | P19-AUD-002 |
| pi/glm53 (run above) | P19-AC-004 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-004 | partially-met | P19-AUD-002 |
| pi/glm53 (run above) | P19-AC-005 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-005 | partially-met | P19-AUD-002 |
| pi/glm53 (run above) | P19-AC-006 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-006 | met | none |
| pi/glm53 (run above) | P19-AC-007 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-007 | met | none |
| pi/glm53 (run above) | P19-AC-008 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-008 | met | none |
| pi/glm53 (run above) | P19-AC-009 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-009 | met | none |
| pi/glm53 (run above) | P19-AC-010 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-010 | met | none |
| pi/glm53 (run above) | P19-AC-011 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-011 | met | none |
| pi/glm53 (run above) | P19-AC-012 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-012 | met | none |
| pi/glm53 (run above) | P19-AC-013 | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 7, table row P19-AC-013 | met | none |
| pi/glm53 (run above) | P19-EXIT-GATES | `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` @ `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` : Section 8, verification union | not-met | P19-AUD-001 |

## Appendix B: reproducible red probes

Use another unique source export of the remediation head with faithful Git
modes. Reuse the repository's external Cargo cache. Save the following Rust
block as `native-probes.rs` outside the export, append it only to the exported
`crates/opi-eval/tests/native_driver.rs`, and copy the existing
`local_acceptance_verifier_accepts_genuine_native_evidence` function under the
name `p19_plan_acceptance_rejects_fixture_evidence`. In that copy only, replace
the final verifier exit-0/accepted assertions with an exit-2 assertion and stop
the copied function there (the setup is unchanged). The exact transformation
used is included after the Rust block. Do not alter exported production files.

```rust
#[test]
fn p19_plan_required_sampling_rejects_before_dispatch() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let source = fs::read_to_string(&config).unwrap().replace(
        "temperature = 0.0", "temperature = 0.0\nrequired = [\"temperature\"]");
    fs::write(&config, source).unwrap();
    stage_output_control(&staged, &config);
    let root = staged.root.join("required-run");
    let (code, stdout, stderr) = run_native(&staged, &config, &root);
    eprintln!("code={code}; report={stdout}; stderr={stderr}");
    assert_eq!(code, 2, "unsupported required sampling must reject before dispatch");
    assert!(!root.join("trials").exists(), "no trial effect is admitted");
}

#[test]
fn p19_plan_offline_preserves_unsupported_output() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let mut material: serde_json::Value = serde_json::from_slice(&fs::read(&staged.material).unwrap()).unwrap();
    for agent in material["agents"].as_object_mut().unwrap().values_mut() {
        agent.as_object_mut().unwrap().remove("output_control_evidence");
    }
    fs::write(&staged.material, serde_json::to_vec(&material).unwrap()).unwrap();
    let root = staged.root.join("optional-run");
    let (code, stdout, stderr) = run_native(&staged, &config, &root);
    assert_eq!(code, 1, "{stderr}");
    let live: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(live["pairs"][0]["comparability"], "unsupported-control:max_output_tokens");
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval")).arg("report").arg("--root").arg(&root).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let offline: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("run={}; offline={}", live["pairs"], offline["coverage"]);
    assert_eq!(offline["coverage"][0]["comparability"], live["pairs"][0]["comparability"]);
}

#[test]
fn p19_plan_privacy_failure_stops_dispatch() {
    let staged = stage("ok");
    let config = write_config(&staged);
    let root = staged.root.join("privacy-run");
    let canaries = staged.root.join("canaries.txt");
    fs::write(&canaries, "native-driver-test\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("run").arg("--config").arg(&config).arg("--root").arg(&root)
        .arg("--fixtures").arg(fixtures()).arg("--native-material").arg(&staged.material)
        .arg("--canaries").arg(&canaries)
        .env("OPI_EVAL_SCRIPTED_KEY", "<dummy-scripted-credential>")
        .env("PATH", stage_cleanup_docker(&staged.root)).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("report={report}; stderr={stderr}");
    assert_eq!(report["trials"][0]["status"], "failed");
    assert!(!stderr.contains("trial trial-pi-native: stage=start"), "second native trial started after privacy failure");
}

#[test]
fn p19_plan_acquisition_cancel_cleans_child() {
    use std::os::unix::process::CommandExt;
    use std::time::{Duration, Instant};
    let staged = tempfile::tempdir().unwrap();
    let cache = staged.path().join("cache");
    seed_local_cache(&cache);
    fs::remove_file(cache.join("agents/pi")).unwrap();
    let marker = staged.path().join("acquirer.pid");
    write_exec(&cache.join("pins/scripts/materialize-external-locks.sh"),
        &format!("#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec sleep 30\n", marker.display()));
    let lock_path = static_lock_in(&cache);
    let mut lock: serde_json::Value = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    lock["authority"]["producers"][0]["sha256"] = sha256_hex(&fs::read(cache.join("pins/scripts/materialize-external-locks.sh")).unwrap()).into();
    fs::write(&lock_path, serde_json::to_vec(&lock).unwrap()).unwrap();
    let config = experiment_with_cache(staged.path(), &cache);
    let mut child = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("prepare").arg("--config").arg(&config).arg("--out").arg(staged.path().join("out"))
        .process_group(0).spawn().unwrap();
    let pid = child.id();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !marker.exists() {
        let _ = Command::new("kill").args(["-KILL", "--", &format!("-{pid}")]).status();
        let _ = child.wait();
        panic!("acquirer did not reach the probe marker");
    }
    let acquired_pid = fs::read_to_string(&marker).unwrap();
    Command::new("kill").args(["-INT", &pid.to_string()]).status().unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let state = fs::read_to_string(format!("/proc/{acquired_pid}/stat")).unwrap_or_default();
    let alive = !state.is_empty() && !state.contains(") Z ");
    eprintln!("after SIGINT: materializer pid={acquired_pid}, live={alive}, state={state}");
    let _ = Command::new("kill").args(["-KILL", "--", &format!("-{pid}")]).status();
    let _ = child.wait();
    assert!(!alive, "prepare cancellation left its acquisition process alive");
}

```

Exact test installer used (adjust only temporary export/probe paths on replay):

```python
from pathlib import Path
root = Path('/tmp/opi-p19-remediation-plan.uxwgp3hu')
source = Path('/mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediation-plan-f05600b20e304b76a44dc968edf649f8')
target = root / 'crates/opi-eval/tests/native_driver.rs'
text = target.read_text()
text = text.split('\n#[test]\nfn p19_plan_', 1)[0]
start = text.index('#[test]\nfn local_acceptance_verifier_accepts_genuine_native_evidence()')
end = text.index('\n/// P19-AC-013:', start)
clone = text[start:end].replace('fn local_acceptance_verifier_accepts_genuine_native_evidence()', 'fn p19_plan_acceptance_rejects_fixture_evidence()')
needle = '    assert_eq!(\n        output.status.code(),\n        Some(0),\n        "stderr: {stderr} stdout: {stdout}"\n    );'
assert needle in clone
clone = clone[:clone.index(needle)] + '    assert_eq!(output.status.code(), Some(2), "fixture acceptance must reject: {stdout} {stderr}");\n}\n'
target.write_text(text + '\n' + (source / 'native-probes.rs').read_text() + '\n' + clone)
print('Installed five probe tests in archive only; production bytes unchanged')

```

CLI probe used (adjust only exported source and compiled binary paths on replay):

```python
import hashlib,json,pathlib,shutil,subprocess,sys,tempfile
root=pathlib.Path('/tmp/opi-p19-remediation-plan.uxwgp3hu')
binary='/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval'
room=pathlib.Path(tempfile.mkdtemp(prefix='p19-cli-probes.'))
fixtures=root/'crates/opi-eval/tests/fixtures'
config=fixtures/'experiment/local-paired.toml'
def invoke(*args):
    p=subprocess.run([binary,*map(str,args)],capture_output=True,text=True,timeout=30)
    return p.returncode,p.stdout,p.stderr
results=[]
def record(case,good,detail):
    row={'case':case,'contract_result':'PASS' if good else 'FAIL',**detail}
    results.append(row)
    print(json.dumps(row),flush=True)
cache=room/'cache'
(cache/'pins/static').mkdir(parents=True)
shutil.copyfile(root/'crates/opi-eval/external-locks/static/linux-x86_64.json',cache/'pins/static/linux-x86_64.json')
plan=room/'missing.json'
plan.write_text(json.dumps([{'id':x} for x in ['agents/pi','corpora/terminal-bench-2.1','runners/harbor']]))
p=subprocess.run(['bash',str(root/'crates/opi-eval/scripts/materialize-external-locks.sh'),'prepare','--cache',str(cache),'--plan',str(plan)],capture_output=True,text=True,timeout=15)
record('acquire',p.returncode==0,{'exit':p.returncode,'stderr':p.stderr})
run=room/'original'
code,out,err=invoke('run','--config',config,'--fixtures',fixtures,'--root',run)
assert code==0,(code,err)
before=(run/'run-report.json').read_bytes()
code,out,err=invoke('run','--config',config,'--fixtures',fixtures,'--root',run,'--recover')
after=(run/'run-report.json').read_bytes()
record('recover-preserves-report',before==after,{'exit':code,'before_sha256':hashlib.sha256(before).hexdigest(),'after_sha256':hashlib.sha256(after).hexdigest(),'before_trials':len(json.loads(before)['trials']),'after_trials':len(json.loads(after)['trials'])})
code,out,err=invoke('run','--config',config,'--fixtures',fixtures,'--root',room/'unstarted','--recover')
recovered=json.loads(out)
record('recover-enumerates-unstarted',len(recovered['recovery'])==2,{'exit':code,'recovery':recovered['recovery']})
replacements=[]
for n in [1,2]:
    code,out,err=invoke('run','--config',config,'--fixtures',fixtures,'--root',room/f'replace-{n}','--replacement-for','trial-pi-1')
    assert code==0,(code,err)
    r=json.loads(out)
    replacements.append({'experiment':r['experiment'],'digest':r['manifest_digest'],'trials':[x['id'] for x in r['trials']],'groups':[x['group'] for x in r['trials']]})
record('replacement-new-identities',replacements[0]!=replacements[1],{'runs':replacements})
selection=room/'selection.toml'
selection.write_text(config.read_text().split('[[trials]]')[0]+'\n[selection]\nscope = "subset"\ntask_ids = ["fixture-task"]\nreason = "plan probe"\nrepetitions = 1\n')
frozen=room/'run-plan.json'
code,out,err=invoke('plan','--config',selection,'--fixtures',fixtures,'--out',frozen)
assert code==0,(code,err)
code,out,err=invoke('run','--plan',frozen,'--fixtures',fixtures,'--root',room/'selection-replacement','--replacement-for','trial-baseline-pi-fixture-task-r1')
record('replacement-frozen-selection',code==0,{'exit':code,'stderr':err})
(root/'cli-probes-results.json').write_text(json.dumps(results,indent=2))
print('probe_root='+str(room))
sys.exit(any(x['contract_result']=='FAIL' for x in results))

```

For green-after implementation, port the recovery and replacement assertions
to the named focused test filters in B5/B6; use equivalent success predicates,
including before/after byte equality and distinct paired IDs. The acquisition
branch maps to B9: its deterministic green check uses bounded pinned local
fixtures. The separate real-material replay uses D3's fresh pinned preparation.

## Appendix C: current witness-retention inspection

The following read-only Python check ran against the original Windows source
archive recorded in Admission, with no test probes appended to that copy.
It inspects committed source rather than claiming a native execution. The
LF-normalized native_driver.rs SHA-256 was
`d0b0425acf099d2644a25ee88b22fe8988f823de28b9880fad92723726b4aa47`;
its Git blob equals HEAD's `27d8ab515b65b9a9332ed4f35023ed66233bf3c1`.

```python
import hashlib, json, re
from pathlib import Path
root = Path(r'C:\Users\Luiz\AppData\Local\Temp\opi-p19-remediation-plan-f05600b20e304b76a44dc968edf649f8')
p = root / 'crates/opi-eval/tests/native_driver.rs'
text = p.read_text(encoding='utf-8')
rows = []
for name in ('native_tb21_artifact_witness', 'native_tb3_artifact_witness',
             'native_deepswe_patch_witness', 'native_protocol_client_matrix'):
    match = re.search(r'^fn ' + name + r'\(\) \{.*?^\}', text, re.M | re.S)
    assert match, name
    body = match.group()
    deletes = body.count('fs::remove_dir_all(&root)')
    autodrop = ('let root = tempfile::tempdir().unwrap();' in body
                and 'root.keep()' not in body)
    assert deletes == 2 or autodrop, name
    rows.append({'test': name, 'line': text[:match.start()].count('\n') + 1,
                 'success_root_cleanup_calls': deletes, 'tempdir_drop': autodrop,
                 'retained_raw_evidence': 'FAIL: successful execution does not retain the root'})
report = {'source_head': '2b6538a1b0bca0146ebaeac2d80352a5929fe22b',
          'source_lf_sha256': hashlib.sha256(p.read_bytes().replace(b'\r\n', b'\n')).hexdigest(),
          'observation_kind': 'static; no native test executed', 'rows': rows}
out = root / 'plan-native-retention.json'
out.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps(report, indent=2))
print('report_sha256=' + hashlib.sha256(out.read_bytes()).hexdigest())
```

The three official witness bodies each contain two removals, one before and
one after execution; the protocol matrix has the auto-dropped tempfile root.
The saved JSON SHA-256 is
`092fa957e1e0e9d700645c3fff778a8bc6b583130e7719dbc8f3190328b0bd3b`.
This is evidence of the current retention defect, not a statement about all
historical runs. B10/B11 green checks inspect surviving raw outputs after the
real test processes exit instead of testing for source-text wording.

## Materialization boundary

Planning changes only the fixed Markdown plan and JSONL dispositions. Nothing
is staged or committed. No runtime fix, result declaration, ledger update,
release or audit rerun is performed. Planning is complete with a validated
READY-FOR-APPLY plan; its native checks remain future work and no finding is
declared Closed here. A new explicit apply invocation must contain the exact
validator-emitted digest. Any active-index or plan byte change invalidates
approval. Apply records actual native prerequisite failures and incomplete
checks truthfully; this plan does not authorize model spending or Phase exit.
