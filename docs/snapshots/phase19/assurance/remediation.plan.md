# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `7ae158dc8cfbdf0d4ec23497f2d5d34883878156f5eb1f6b83ae07d0ce98dd57`
**Remediation head**: `31b3ae11095a41f1cbec46c6886b8513de2a292c`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]; committed=none-for-this-task.
**Current continuation inventory**: staged=[]; unstaged=[the two task-owned fixed plan files plus six separately owned workflow-maintenance carry-in files listed below]; untracked=[]; HEAD unchanged.
**Unresolved decisions**: none
**Pending closure evidence**: C005 fresh acceptance/input-budget freeze; C006 executed native witnesses/regrade. Both remain explicit Cannot confirm exclusions; their authorization and handling decisions are resolved.

Planning owns only this fixed plan pair. The previously tracked clean pair was replaced without consuming its earlier content; this is a continuation of those owned edits. Current audit bytes, result pair, ledgers and production source are retained. This plan branch owns no production edit, branch or commit. The user separately authorized bounded no-model native witnesses/offline regrade with verified local caches, and prospective paid acceptance/retry with glm-5.3-flash under the frozen scope below. No further budget confirmation is required inside that scope. Source-repair and material admission gates remain; no fresh paid acceptance has started. Application of B1-B9 requires a new explicit apply invocation with this plan's exact validator-emitted digest. Ready status does not close C005/C006, change the audit verdict or admit Phase exit.

The six workflow-maintenance carry-in files are `.agents/skills/README.md`, `.agents/skills/README.zh.md`, `.agents/skills/opi-eval/SKILL.md`, `.agents/skills/opi-eval/references/fidelity-run.md`, `.agents/skills/opi-eval/references/local-acceptance.md` and `.agents/skills/opi-eval/references/native-run.md`. Preserve them unchanged. Their defaults guide prospective operator orchestration, but their uncommitted bytes are not remediation_head committed evidence, archive source applicability or criterion-closure proof. Human budget authority is recorded separately below.

Current committed code and plan-stage checks use a unique `git archive` export at `C:/Users/Luiz/AppData/Local/Temp/opi-remediate-p19-0219472cad1f4164a338d6dccbe577ec/src`, without a Git worktree. Synthetic red tests were added only to this export or its parent; production code was not edited. The existing persistent WSL Cargo cache `/home/luiz/.cache/opi/cargo-targets/opi-8abcf38f9871653e` was reused with leases. No per-session target, cleanup or cache deletion was used.

## Active sources and criterion alignment

| Alias | Full source run | Findings SHA-256 | Audit head |
|---|---|---|---|
| C | `phase19-codex-gpt61-2465aeb-20261001t055252z` | `90d9a494d59824cb99dfcf749cf6b0c30e7fa6eb79959b7e406fa2eeea64b5a1` | `2465aeb80ad84d06172acc142f6d5ffbceb201cc` |
| P | `phase19-pi-glm53-efe2f69-20261001t043357z` | `fb086ccb76ea53156b0600c6b46f8e8c1b9fb8842515f3acff8669ec69923b70` | `efe2f6906fa47003bf806bddec0b15f2d20dd3c3` |

The current run/digest controls admission. Title similarity cannot substitute for identity, and older sources cannot enter this strict union. Both peers remain independent sources; neither is canonical. All fourteen `(audit_run_id, id)` keys have exactly one disposition. Source severity is retained; shared fixture repair schedules at Major.

Both members register the same current parent and supplemental source bytes: `docs/opi-spec.md` SHA-256 `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`; supplemental English `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a`; Chinese `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194`. The supplemental path is `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` and its `.zh.md` counterpart. These registered sources supply authority; arbitrary unregistered designs and prior conclusions do not.

| Criterion alignment | Codex requirement/state | pi requirement/state | Current resolution |
|---|---|---|---|
| Same registered path/digest, section8 gates; exact citation strings differ | R45 not-met (008,009) | R-GATES not-met (002), R-AC011 partially-met (001) | Keep requirements separate. Both formatter and fixture failures reproduce at HEAD. |
| Same source/digest, AC-011/section8; independently stated obligations | R41 partially-met (007), R42 partially-met (004) | R-AC011 partially-met (001) | Fixture observation does not refute duplicate CI policy or root-admission defect. |
| Same source/digest, distinct AC-012/section8 and native-proof criteria | R44 partially-met (005), R07/R10/R13/R31 not-assessable (006) | R-AC012 met (Info003) | Credit genuine six trials/source applicability without inferring missing budget/witness/regrade proof. No vote or state overwrite. |
| Remaining independently stated AC-006/010/013 and PRIN-002 criteria | R16/R33/R34/R35/R49/R56 (001,002,003,010,011) | No matching source finding | Preserve Codex source rows and verify against current code/tests. |

## Current Finding Verification

| Source / finding | Verification | Source / final severity | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| C/P19-AUD-001 | Confirmed | Major / Major | `execution.explicit-capture-bound-admission` / `execution.authorized-bounds` | B1 | `fix:reject-unenforceable-captured-output-bound` |
| C/P19-AUD-002 | Confirmed | Major / Major | `credentials.private-host-stage-before-write` / `privacy.native-capture` | B2 | `fix:create-private-host-credential-stage` |
| C/P19-AUD-003 | Confirmed | Major / Major | `native-capture.private-thinking-boundary` / `privacy.native-capture` | B8 | `fix:filter-private-thinking-before-native-persistence` |
| C/P19-AUD-004 | Confirmed | Major / Major | `acceptance.one-admitted-root-for-all-consumers` / `evidence.acceptance-admission` | B3 | `fix:admit-trial-paths-and-reuse-selected-family-root` |
| C/P19-AUD-005 | Partially confirmed | Major / Major | `acceptance.contemporaneous-usage-time-freeze` / `evidence.operator-acceptance` | none | `no-action:Cannot confirm` |
| C/P19-AUD-006 | Cannot confirm | Major / Major | `native.executed-witnesses-and-real-artifact-regrade` / `evidence.native-conformance` | none | `no-action:Cannot confirm` |
| C/P19-AUD-007 | Confirmed | Minor / Minor | `ci.external-lock.shared-policy` / `ci.materialization-policy` | B9 | `fix:delegate-external-lock-ci-to-shared-cli` |
| C/P19-AUD-008 | Confirmed | Minor / Minor | `verification.rustfmt-native-lock-assertion` / `verification.formatting` | B5 | `fix:format-native-lock-assertion` |
| C/P19-AUD-009 | Confirmed | Minor / Minor | `fixture.output-control-hermetic-current-prepare` / `verification.script-fixtures` | B4 | `fix:update-synthetic-current-prepare-fixture` |
| C/P19-AUD-010 | Confirmed | Minor / Minor | `docs.native-run-and-explicit-oracle` / `documentation.native-contract` | B6 | `fix:align-native-run-rustdoc-with-current-contract` |
| C/P19-AUD-011 | Confirmed | Minor / Minor | `controls.reasoning-capability-truth` / `connection.control-projection` | B7 | `fix:record-unproven-reasoning-as-unsupported` |
| P/P19-AUD-001 | Confirmed | Major / Major | `fixture.output-control-hermetic-current-prepare` / `verification.script-fixtures` | B4 | `fix:update-synthetic-current-prepare-fixture` |
| P/P19-AUD-002 | Confirmed | Minor / Minor | `verification.rustfmt-native-lock-assertion` / `verification.formatting` | B5 | `fix:format-native-lock-assertion` |
| P/P19-AUD-003 | Confirmed | Info / Info | `registry.removed-report-history-resolution` / `evidence.registry` | none | `no-action:Info/No action` |

JSONL records retain each source's current evidence/rationale. C005 is partially confirmed only within supplied evidence; C006 is a missing executed-proof case and remains Cannot confirm. Neither is a demonstrated native runtime defect. Their explicit exclusions propose no production repair and do not waive the mandatory criteria. Under the disposition contract and closure matrix, these completed disposition decisions permit the independent B1-B9 fixes to proceed after approval; finding closure still needs the retained execution/evidence routes.

## Confirmed user decisions

- **D1**: remove private thinking before persistence; reject unknown formats. Preserve non-private terminal, tool and grading-binding evidence. This adopts the parent privacy obligation without rewriting existing immutable bundles. B8 implements this direction at the earliest capture sink, not only at later export classification.
- **D4**: preserve the manual workflow and transport receipt; delegate preparation and explicit conformance to the shared local CLI. B9 keeps receipt claims bound to what actually ran.
- **D3 execution scope**: bounded local-cache native witnesses/offline regrade remain authorized and contain no model/provider calls. The separate paid acceptance authorization below does not convert native diagnostics into model trials. Material and successful executed evidence remain missing inputs, not unresolved authority choices.
- **D2 operator choice**: use a fresh prospective acceptance run under the frozen retry budget below; the earlier no-paid-retry restriction is superseded. No historic freeze is inferred.

## Frozen prospective paid acceptance budget

This records current Human Authority for the upcoming acceptance batch. Recorded at 2026-10-01 09:29:05 UTC (17:29:05 Asia/Shanghai); this is the recording time, not an invented earlier approval timestamp. The user's exact instructions are "自动使用 glm-5.3-flash 付费重试" and "重试2次，最长1h，费用无限制，固定下来".

The user subsequently requested "固定到每次评测默认配置中". The separate workflow-maintenance edits persist these standing defaults; this plan consumes them only for its selected acceptance batch and does not broaden trial scope.

| Frozen item | Authorized value |
|---|---|
| Model | glm-5.3-flash; retain the selected Zhipu connection/protocol and freeze its actual runtime identity before dispatch |
| Logical acceptance scope | Six originals: one Opi and one pi trial for each selected TB2.1, TB3 and DeepSWE task |
| Retry unit | At most2 additional executions per failed logical trial, after its initial execution; at most3 executions per logical trial |
| Derived batch execution ceiling | At most18 actual trial executions for6 originals, including every trial dispatched by a replacement scope |
| Concurrency | 1, retaining the existing declared acceptance scope |
| Whole-batch time budget | 3600 seconds under one monotonic deadline, starting before the first execution of the new paid acceptance batch |
| Spend ceiling | None: the user explicitly authorizes unlimited fees within this model/count/time scope; no currency ceiling or unsupported max_cost is invented |
| Record preservation | Fresh experiment/trial/run identities; preserve all initial/retry attempts, commands, exits, timing, usage and native outcomes; never overwrite old sealed evidence or hide failed attempts |
| Scope covered by this plan | Prospective six-trial acceptance under standing defaults; no retrospective budget proof, new product retry feature, commit or release authority |

The planned six logical slots retain the existing candidate tasks openssl-selfsigned-cert (TB2.1), batched-eval-parity (TB3), and abs-module-cache-flags (DeepSWE). Their final selected task/build/material/config digests, supported model controls and credential references must be frozen before the batch starts. Runtime inputs are not guessed from this budget table; material availability and the admitted privacy repair remain prerequisites.

Operator orchestration must check the actual dispatched scope before every retry. Standing authorization covers additional attempts only for failed logical trials; completed valid native results, including zero, are not retry triggers. The current --replacement-for path may execute the entire declared scope with fresh identities and is not inherently one failed-trial retry. If that scope includes an already completed trial or other expansion, do not dispatch without separate explicit scope authorization. Prepare a fresh failed-trial-only scope only where supported; otherwise record a preparation gap. Count every actual execution against its logical trial's three-attempt ceiling and the aggregate18 ceiling; those ceilings do not grant additional scope. Preserve every attempt and replacement link without selecting only favorable scores or changing frozen controls. Provider-request retries remain distinct from whole-trial attempts and share the original batch deadline, including grading.

Current time_limit_seconds is per Agent process and does not implement the whole-batch3600s limit. The operator must use one monotonic deadline, check it before each subsequent run/retry, cancel active owned execution when it expires, stop new attempts and retain cleanup evidence. Merely setting time_limit_seconds=3600 is not proof of the batch limit. Current cancellation is cooperative and the process supervisor checks after spawn; do not claim an unverified absolute pre-spawn guarantee for every internal model/grader request. If the existing operator path cannot demonstrate the frozen count/time bounds, no model-containing run begins; stricter source enforcement remains subject to its owning admission gate. Fee permission is already settled and must not be requested again.

This budget record precedes the upcoming run. C005 remains partially confirmed for the original supplied evidence: the new authorization cannot establish a historical freeze. Closure still requires an attributable fresh run and its contemporaneous complete input/budget freeze, all attempt records and independently verified actual boundaries. No new mandatory acceptance schema or configuration key is introduced by this record.

## Unresolved Decisions

none.

D1 and D4 have concrete B8/B9 designs and red-before evidence. D2 budget authority and D3 no-model execution authority are fixed. The closure matrix permits Cannot confirm exclusions, and plan-stage green observations must remain not-run; successful final execution proof is not a prerequisite for approving these nine source-repair batches. This corrects the earlier draft's treatment of D3 missing material as an unresolved decision. No criterion is revised, deferred by invented authority or claimed met.

## Evidence prerequisites and follow-up

| Finding | Missing input or evidence | Required route and boundary |
|---|---|---|
| C005 | A fresh source/material/input-bound acceptance run with contemporaneous complete usage/time freeze and all attempts. | After B8 privacy admission and verified materials, execute only the frozen prospective scope with proven count/deadline supervision. Prior accepted trials and current authorization cannot supply a historical freeze. |
| C006 | Runnable verified local material, three controlled pinned-grader witnesses and genuine saved-artifact regrade. | The authorized TB2.1 witness failed before dispatch on its absent pinned static-lock path; referenced pi/tasks/Harbor/Pier/uv caches are also absent. Supply exact verified material/cache or source-applicable raw executed proofs, then use the no-model route below. No download or model call is authorized by the cache-only diagnostic branch. |

These prerequisites block their respective native execution and closure claims, and the remaining mandatory gaps block Phase conformance. They do not block approved edits for the independently reproduced B1-B9 defects. A pre-dispatch refusal cannot close C006. No additional budget approval is outstanding. Any newly supplied proof must be checked for source/material applicability; changed plan bytes require a new digest and approval.

## Closure Batches

Nine bounded designs cover eleven source records; C005/C006 retain Cannot confirm exclusions and P003 retains Info/No action. Distinct closure keys remain separate even where files overlap. Dependent edits are sequential: B1 before B7; B2 and B7 before B8; B8 before B6 and B9; B4 before B9; B5 after test-source edits. The plan is ready for review; production application still requires its explicit digest-bound invocation.

### Batch B1: Refuse unenforceable requested capture bounds

**Closure predicate**: explicit positive max_output_bytes cannot resolve into an executable experiment without enforcement; omission retains existing fixed capture protection.
**Dependencies**: none.
**Verification union**: isolated bound regression, experiment_contract, affected clippy, formatter, bilingual docs and final union.

#### Fix B1.1: Reuse early unsupported-bound rejection

- **Finding source(s)**: C/P19-AUD-001, exact full run/digest in Active sources and JSONL.
- **Decision**: `fix:reject-unenforceable-captured-output-bound`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/experiment_contract.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: after numeric validation, reuse existing UnenforceableExecutionBound beside max_total_tokens/max_cost admission to reject explicit positive max_output_bytes in every mode before effects. Keep the existing field/serialized shape and fixed runtime limits; synchronize current Rustdoc, positive connection fixture and bilingual README. Record scoped 0.x tightening in Unreleased. No new cap implementation/flag/schema version.
- **Closure predicate**: positive requested bound rejects by name in all three modes; omission is admitted.
- **Red-before**: `cargo test -p opi-eval --lib explicit_capture_bound_rejects_during_resolution --locked` on export plus synthetic test: exit 101,1 executed failure,0 ignored; first hermetic case resolves max_output_bytes=1. All modes use the same unguarded parser branch.
- **Green-after**: same command passes its full mode loop, plus `cargo test -p opi-eval --test experiment_contract --locked`. Promote the temporary assertion into existing behavior-based contract coverage.

### Batch B2: Protect the original host credential stage

**Closure predicate**: credentials enter only trial-owned 0700 directories / 0600 files created privately before the first secret byte, independent of caller root/umask.
**Dependencies**: none; B8 owns separate capture-content privacy.
**Verification union**: Unix mode/preexisting-path refusal and cleanup, native_driver, affected clippy.

#### Fix B2.1: Make private creation precede credential writes

- **Finding source(s)**: C/P19-AUD-002, exact C identity/digest in JSONL.
- **Decision**: `fix:create-private-host-credential-stage`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: reuse current private-creation pattern at runner lines 3629-3635/3747 without extracting unrelated capture machinery. For supported Unix native execution create the owned integration directory 0700 before writes and config/handoff0600 with exclusive creation; fail closed on preexisting/link targets. Preserve cleanup at 3767-3794, selected credential identity and caller-root permissions. No global umask mutation or Windows ACL expansion. Add child-only umask022/searchable-root case and unsafe-path refusal to existing test binary; add the user-visible security fix to Unreleased.
- **Closure predicate**: both selected clients' host stage is private before dispatch and cleaned on failure.
- **Red-before**: `cargo test -p opi-eval --test security_plan_probe --locked plan_security_host_credentials_private_on_searchable_root -- --nocapture`: exit 101,1 executed intended failure,0 ignored; both clients show directory 0755/config 0644/handoff 0644 with fake secret-present booleans. A separate two-case run also reproduces D1 without merging its predicate.
- **Green-after**: the identical red command and host-mode assertion in the retained temporary probe pass 0700/0600, then promoted native_driver permission/refusal filter and native_settlement_removes_actual_credentials pass. The separate thinking predicate belongs to B8.

### Batch B3: Admit one root identity before all evidence reads

**Closure predicate**: unsafe trial IDs reject before reads; every Python/Companion consumer uses the same contained manifest-selected family root.
**Dependencies**: none; budget D2 is separate.
**Verification union**: three structural probes, acceptance Python suite, public verifier subprocess negative path, full script discovery.

#### Fix B3.1: Share admitted paths across consumers

- **Finding source(s)**: C/P19-AUD-004, exact C identity/digest in JSONL.
- **Decision**: `fix:admit-trial-paths-and-reuse-selected-family-root`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`.
- **Change kind**: behavioral.
- **Change**: admit nonempty single-component trial IDs before reads, rejecting absolute/dot/parent paths and either platform separator. Resolve contained family roots once; require paired trial declarations agree. Reuse those roots for native/bundle/material/control/report and Companion argv, replacing hardcoded canonical reads at lines 252/382/457/931. Retain contained custom roots. Cover absolute/traversal/symlink escape, conflicting pair roots and benign alternate roots with existing recording fixture and public CLI refusal. Preserve no-Agent/no-credential/no-provider boundary.
- **Closure predicate**: all byte and semantic consumers share one admitted identity; malformed paths refuse before access.
- **Red-before**: `python3 ../acceptance-probes.py` from export src: exit 1,3 assertion failures,0 skipped; external harmless absolute/traversal canaries admitted, Companion chooses runs/family instead of selected/family. No full invalid false acceptance claimed.
- **Green-after**: same three probes pass; `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py -v` passes existing/new positive and negative cases.

### Batch B4: Make the current preparation fixture self-contained

**Closure predicate**: both existing output-control fixture tests reach their ordering/refusal assertions at the current shared prepare seam without installed uv or external effects.
**Dependencies**: none.
**Verification union**: focused fixture, native-smoke CI verifier, full Python discovery on WSL/Linux.

#### Fix B4.1: Model actual shared preparation

- **Finding source(s)**: C/P19-AUD-009 and P/P19-AUD-001, two separate run/digest keys.
- **Decision**: `fix:update-synthetic-current-prepare-fixture`; schedule at Major while retaining source Minor/Major.
- **Verification status**: Confirmed for both.
- **File(s)**: `crates/opi-eval/scripts/test_native_smoke_output_control.py` only.
- **Change kind**: test-only.
- **Change**: create actual synthetic executable uv bytes in fresh fixture roots; mock shared opi-eval prepare with explicitly synthetic per-family material/6 outputs. Check cached Opi/pi executable/package byte transport, compare cwd via Path, and validate actual --native-material argument. Retain exact-client proof-before-validation/freeze, failed/missing proof, credential exclusion, fresh retries and no routing/final receipt after failure. Keep the original two scenario-owning methods. No production producer change.
- **Closure predicate**: original methods/refusal subcases pass with only fixture bytes under current prepare behavior.
- **Red-before**: `python3 -m unittest discover -s crates/opi-eval/scripts -p test_native_smoke_output_control.py -v`: WSL exit 1,2 methods/3 errors (uv file missing and partial retry). Outside-source uv-only prototype next fails at unexpected prepare; current-fixture-only prototype passes both original methods. Prototype is design evidence, not production green.
- **Green-after**: same focused command plus `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_native_smoke_ci.py -v` and full discovery pass.

### Batch B5: Restore the formatter gate

**Closure predicate**: rustfmt accepts the admitted-static-lock assertion without changing condition/message.
**Dependencies**: B2 and any other test-source edits first.
**Verification union**: cargo fmt --check --all and diff whitespace check.

#### Fix B5.1: Format the baseline assertion

- **Finding source(s)**: C/P19-AUD-008 and P/P19-AUD-002, separate current keys.
- **Decision**: `fix:format-native-lock-assertion`.
- **Verification status**: Confirmed for both.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`.
- **Change kind**: test-only; assertion semantics retained.
- **Change**: use rustfmt's single-line admitted-static-lock assertion at original line 10054; format only task-owned changed sources.
- **Closure predicate**: formatter exits 0 on final relevant source.
- **Red-before**: `cargo fmt --check --all` on original committed export, Windows exit 1, sole original-source hunk native_driver.rs:10054.
- **Green-after**: same command exits 0. No added test assertion mirrors formatting.

### Batch B6: Correct current native run Rustdoc

**Closure predicate**: Rustdoc distinguishes hermetic/native execution and describes oracle diagnostics as explicit, matching current call sites.
**Dependencies**: B2 and B8 first for overlapping runner file.
**Verification union**: source comparison, documentation checker, formatter, warning-free opi-eval rustdoc.

#### Fix B6.1: Replace superseded fixture/oracle claims

- **Finding source(s)**: C/P19-AUD-010, exact C identity/digest in JSONL.
- **Decision**: `fix:align-native-run-rustdoc-with-current-contract`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/cli/run.rs`.
- **Change kind**: documentation.
- **Change**: align module line 13/preparation comment near 1167 with actual oracle=None near 1212; align CLI module lines 10-13 with native-material dispatch at 152-190. Describe selected real Agent/provider/native grader and explicit diagnostics. Rustdoc has no Chinese counterpart; bilingual README executable meaning is already accurate for this correction. Retain behavior/tests.
- **Closure predicate**: cited comments agree with hermetic/native paths and explicit conformance selection.
- **Red-before**: N/A: semantic prose contradiction statically proved; no runtime behavior change.
- **Green-after**: source comparison, `python scripts/opi-doc-check.py`, `git diff --check`, `cargo fmt --check --all`, and `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps --locked` pass.

### Batch B7: Keep reasoning capability verdicts truthful

**Closure predicate**: requested optional reasoning is retained as Unsupported when selected native execution cannot prove it effective; required refusal and settlement agree.
**Dependencies**: B1 first for experiment.rs/tests/bilingual README/Unreleased ownership.
**Verification union**: isolated projection matrix, required-control refusal, config/contract tests, native_driver and affected clippy.

#### Fix B7.1: Use the existing unsupported verdict

- **Finding source(s)**: C/P19-AUD-011, exact C identity/digest in JSONL.
- **Decision**: `fix:record-unproven-reasoning-as-unsupported`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/experiment.rs`, `crates/opi-eval/tests/experiment_contract.rs`, both opi-eval README counterparts, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: use existing ProjectedControl::Unsupported for supplied reasoning, retaining request/reason. Synchronize current projection/render assertions and Rustdoc, optional admission/required early rejection and unsupported settlement/comparison. Preserve output/context controls; do not add a new enum/config flag or invent effectiveness. Record deliberate 0.x projection tightening and both README counterparts. Serialized shape/schema version is retained.
- **Closure predicate**: both clients/all three protocols preserve high as requested Unsupported without contradictory effectiveness claims.
- **Red-before**: `cargo test -p opi-eval --lib optional_reasoning_projection_records_unsupported --locked`: exit 101,1 executed failure,0 ignored; first Opi/Anthropic case claims Effective. Shared branch and pi --thinking off corroborate scope.
- **Green-after**: same command passes all 6 client/protocol cases; retain required-refusal matrix and run affected experiment_contract/native_driver tests.

### Batch B8: Enforce privacy at the first native capture sink

**Closure predicate**: fresh Opi/Pi captures persist only validated privacy-projected bytes, under full and terminal-only policies; unsupported or unsafe streams refuse without a native score, while non-private terminal/tool/grading binding remains truthful.
**Dependencies**: B2/B7 first. B6/B9 follow; refresh shared producer pins after their final bytes. Existing read-only offline evidence obligations remain separate from fresh dispatch.
**Verification union**: persistence red, actual integration first-write observer, parser/helper subprocess cases, native_driver and controlled fixtures, material/build identity tests, changed script suites, bilingual docs and the final workspace union.

#### Fix B8.1: Stage one bounded shared capture helper before credentials

- **Finding source(s)**: C/P19-AUD-003, exact C run/digest in Active sources and JSONL.
- **Decision**: `fix:filter-private-thinking-before-native-persistence` (D1 user-confirmed).
- **Verification status**: Confirmed; Major.
- **File(s)**: exact JSONL union: binary-local `src/native_capture.rs` plus private main dispatch; experiment/material/prepare/plan/native runner; benchmark process/regrade/conformance consumers; both integration modules; build/materializer/native-smoke/artifact/acceptance scripts and their changed test owners; native_driver/support; two new synthetic thinking fixtures; static producer pins; both README counterparts; root Unreleased. No library entry or Agent Core edit.
- **Change kind**: behavioral.
- **Closure predicate**: both capture policies persist no private thinking at actual first sinks or later artifacts; recognized streams retain non-private native bindings and unknown/unsafe streams refuse without a score.
- **Change**: add a binary-local bounded Rust parser and hidden internal mode of the existing Companion executable. The private dispatch exposes no new SDK contract, crate, dependency, trait or configuration flag. Keep host proof/partial validation in its existing owners. Reuse the build producer's Cargo compiler-artifact locator to build and pin an x86_64-unknown-linux-musl helper with executable digest, source/Cargo.lock identity and target. Transport it alongside the selected Agent and verify task-side preflight before credentials. Refuse unavailable targets, dynamic loader dependencies, incompatible platforms or failed preflight; do not assume the GNU host CLI or an ambient Python/jq/Node parser runs in every task image. The helper spawns the Agent through bounded stdout/stderr pipes, preserving owned process-group cancellation and observed Agent exit. Replace direct raw shell redirects at Harbor lines587-595 and Pier626-630. Never spool raw contents or fall back to direct writes. Apply equivalent closed handling to version preflight streams, exceptions and partial captures.
- **Admitted grammar**: cover exact Opi NDJSON/schema2 event variants and Pi session2/3 lifecycle/retry correlation already proved by current source/importers and the six admitted capture shape inventory. Transform assistant messages wherever repeated in message/turn/AgentEnd events and update partial/done/error snapshots. Remove thinking text, declared private signatures and redacted_thinking data; empty private thinking delta/end content, preserving placeholders and content_index/contentIndex. Preserve native model/API/provider, IDs, usage/cost, stop/retry/settlement and non-private tool IDs/names/arguments/results. Do not recursively remove arbitrary tool JSON properties named thinking. Unknown envelope/event/block fields or types, duplicate keys, invalid UTF-8/JSON, incomplete records or exceeded existing bounds refuse before that record is written. Use the existing64MiB stream ceiling and a bounded record buffer within it; the largest admitted stream/record is33,930,516/844,458 bytes, so do not add an arbitrary smaller ceiling. Safe unchanged records retain their exact original bytes, whitespace and newlines; only transformed records are serialized.
- **Stderr and traces**: admit empty stderr and explicitly validated diagnostic forms; opaque stderr retains only static refusal/status/count/digest metadata and excludes a score. Preserve Opi's typed redacted trace contract. Validate safe prefixes and known trace records before partial copies; replace the private-unredacted receipt path with truthful sanitized/incomplete status. Terminal-only uses the same first-sink boundary and derives terminal facts from safe bytes. Classification as LocalOnly never substitutes for non-persistence.
- **Fresh proof formats**: native-material/7 requires helper executable pin/target/protocol; native-handoff/4 carries the same identity; new native-capture/1 records closed policy/status and per-stream proof; native-agent-terminal/3 binds helper preflight/capture to existing native identity; partial-capture/2 records safe-prefix/bounds/proofs. Distinguish producer-attested raw source commitment from independently recomputed retained-file identity. Claim a complete raw_source_sha256 only after bounded full EOF; interruption/overflow/refusal records a prefix/incomplete commitment. Host checks compare retained sanitized bytes/digests before native_terminal_completion/client_message, before sealing and before partial copying. Missing, altered, incomplete-for-success or contradictory proof refuses without a fabricated terminal/score. Synchronize all fresh material/terminal producers and consumers, including native-smoke and offline artifact verification.
- **Legacy boundary**: the current README offline contract requires old sealed verify/report/regrade. Retain material/6 and terminal/2 admission only for read-only offline use with their original seals. Never rewrite or relabel legacy bytes as privacy-safe and never use those formats for fresh Agent dispatch. Regrade creates only fresh verifier derivatives from the original declared artifacts. No private payload migration or deletion is authorized. Record the deliberate0.x tightening and versions in Unreleased and both README counterparts.
- **Red-before**: `cargo test -p opi-eval --test security_plan_probe --locked plan_security_private_thinking_is_not_persisted -- --exact --nocapture` in the archive: exit101,0 passed/1 intended failure/0 ignored/123 filtered,14 marker-bearing task/host/bundle/terminal paths across both policies. Separate earlier classification red admits4 Exportable entries. `python3 security-first-sink-probe.py` in archive parent exits1:8 actual generated Harbor/Pier × Opi/Pi × mounted/downloaded launches complete without fixture errors, all observe stdout/stderr markers while emitter is active and after settlement. Existing integration loaders/shell environments execute the real launch path; no Docker/provider/native task runs. Only presence/path metadata is emitted. The two checks independently prove retained persistence and first-write leakage.
- **Green-after**: retain both red commands and their no-marker predicates, permitting valid sanitized success or a specifically identified unsafe-format refusal in the persistence case. Generic missing-helper/material refusal cannot count as first-sink green. Promote fixture emission through the actual helper seam with fresh pinned formats; the first-sink witness must observe an active emitter and a successful recognized stdout capture preserving terminal/retry/tool/native bindings, testing opaque stderr refusal separately. For both integration families and mounted/downloaded capture, poll files while the emitter is alive; thinking message/delta/signature markers never appear, and cancellation leaves only safe prefixes/static diagnostics. Add duplicate-key/type/unknown/malformed/incomplete/overflow/stderr refusal, byte-identical safe stream, arbitrary tool-JSON preservation, altered proof, missing/unrunnable helper, preflight-before-credentials and incomplete-source-commitment cases. Retain >1MiB safe capture, terminal-only omission, native identity/cancellation/partial bounds and legacy offline verify/regrade. Run every new/modified Rust filter/binary (including helper binary tests), `test_native_integrations.py`, `test_native_smoke_output_control.py`, `test_verify_native_smoke_artifact.py`, `test_verify_native_smoke_ci.py`, `test_verify_local_acceptance.py`, relevant build/materializer identity cases and the final union.


### Batch B9: Delegate external-lock CI policy while preserving transport

**Closure predicate**: the manually dispatched default producer publishes a receipt only after successful shared prepare and explicit bound native conformance; its transport, oracle and network claims describe the actual shared evidence.
**Dependencies**: B4 and B8 first; B8 changes the fresh native-material/integration contract consumed by preparation. Update static byte pins last after workflow/scripts stabilize.
**Verification union**: exact default-shell synthetic delegation/refusal probe, external-lock CI/artifact/acquisition Python suites, full script discovery, offline receipt identity/refusal checks, bilingual docs and final union.

#### Fix B9.1: Preserve dispatch and receipt, use shared preparation/conformance

- **Finding source(s)**: C/P19-AUD-007, exact C run/digest in Active sources and JSONL.
- **Decision**: `fix:delegate-external-lock-ci-to-shared-cli` (D4 user-confirmed).
- **Verification status**: Confirmed; Minor.
- **File(s)**: `.github/workflows/opi-eval-external-lock-materialization.yml`; `crates/opi-eval/scripts/materialize-external-locks.sh`, `verify-external-lock-ci.py`, `test_verify_external_lock_ci.py`, `verify-external-lock-artifact.py`, `test_verify_external_lock_artifact.py`; `crates/opi-eval/external-locks/static/linux-x86_64.json`; both opi-eval README counterparts; `CHANGELOG.md`. Fixture shape/pin updates belong to the two existing Python test files. The unused static valid-static/valid-resolved fixtures and independent local-cache fixture remain outside the required path union.
- **Change kind**: behavioral.
- **Closure predicate**: default CI uses shared preparation and explicit conformance authority; shared refusal prevents receipt publication, and successful /2 transport claims bind the actual reports/material/raw artifacts.
- **Change**: keep candidate_sha-only manual dispatch, byte admission, 30-day upload and transport provenance. Build/locate the candidate Companion from Cargo compiler-artifact output using the existing native-smoke locator pattern; reuse `dtolnay/rust-toolchain@889fac408b4da0905346410f253f0c55fbcb6613`, toolchain `1.97.0`, already declared in the static lock. Seed the declared cache with the static lock/profile/provider/integrations. Generate one TB2.1 diagnostic config selecting the pinned task and both clients, with unused loopback fixture connection and no credentials. Invoke shared `prepare --config <config> --out <prepared> --opi-source <candidate> --opi-commit <candidate_sha>`, then explicit `conformance --suite benchmark --adapter terminal-bench-2.1 --case oracle-witness --root <oracle> --fixtures <fixtures> --provider <scripted-provider> --native-material <prepared/material.json>`. Prepare's existing acquisition backend uses its separate prepare mode, preventing default recursion. Both clients are pinned material inputs because current conformance requires them; this diagnostic does not execute either Agent or call a provider. Remove only duplicate default task/runner/uv acquisition, image pull/tag policy and inline Harbor Trial driver. Keep pinned closure archives/shims labelled transport-only, unapplied to the trial; inspect/export the already prepared OCI graph. Shared task preparation preserves lifecycle/network declarations while resolving immutable images. Shared oracle conformance selects official Harbor OracleAgent and labels the outcome DiagnosticWitness/no Agent score.
- **Receipt contract**: introduce `opi-eval-materialization-receipt/2` and derived `opi-eval-external-lock/resolved/2` for required shared provenance. Retain common candidate/static-lock/workflow/producer/run/time/expiry/artifact/closure/OCI fields. Add contained, relative, SHA-bound references for the Companion, diagnostic config, prepare command/report/material and conformance command/report; bind the final fresh material digest/schema after B8 and require suite/adapter/case, `met: true`, native reward origin and `agent_score: false`. Keep material origin `local`; do not insert GitHub provenance into NativeMaterial. Copy oracle status/reward and uniquely bound raw result/CTRF digests from the successful shared report, without a second reward threshold or first-recursive-match fallback. Refuse missing/ambiguous artifacts or wrong task/material bindings before final receipt publication. Distinguish task-declared network mode (`basis: pinned-task-policy`) from an unobserved effective mode; never claim offline enforcement from host UV_OFFLINE. The current verifier rejects legacy receipt/resolved /1; retained /1 bytes and their original candidate-bound producer/verifier are preserved without relabelling or migration. Record the scoped 0.x receipt change in Unreleased and synchronize both README counterparts. No new Rust seam/dependency or partial-material loader.
- **Red-before**: `PYTHONDONTWRITEBYTECODE=1 python3 tooling-external-route-red.py` from archive parent: exit1, 2 executed failures / 0 errors. Exact committed default Bash SHA-256 `edaae15faa05eb21b237f763af5d552ab43bc7c0e90f434cb23982f4bf1883cd` writes a receipt without shared prepare/conformance, and writes one despite configured shared conformance refusal. Fake tools and synthetic intercepted URL bytes only; no actual HTTP/Docker/uv/Cargo/Agent/grader run.
- **Green-after**: the same delegation/refusal predicates pass on the final default script with complete synthetic /2 outputs; extend the fixture's locator output to the actual build seam without weakening assertions. Promote them into existing CI tests, adding prepare refusal, exit0/non-met conformance, wrong binding and missing/ambiguous raw artifact cases. Every refusal retains safe diagnostics and publishes no final receipt. Run `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_external_lock_ci.py -v`, corresponding `test_verify_external_lock_artifact.py` and `test_prepare_acquisition.py`, then full discovery. Offline /2 verification passes exact byte/identity bindings; real CI/native execution remains separate from these hermetic checks.


## Current proof inventory and applicability

Only the validated indexed advisory appendix's exact candidate raw proofs were used for C005/C006. No history directory, prior remediation conclusion or alternate finding source was searched. Independent rechecks establish all ten named files' bytes/digests, commands/results/counts, source/build/client/material identity and predicate. An appendix prose lead without exact raw path was not reconstructed. Absence is bounded to inspected references.

- Supplied manifest SHA-256 `9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce`; genuine six native trials and source-applicable accepted:true proof are credited for actual loop, not budget/witness/regrade criteria.
- Raw Opi source manifest SHA-256 `8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53`; independent in-memory archive comparison at remediation_head matches 694/694 relevant runtime paths, zero added/missing/mismatched. Retained executable SHA-256 `0ccc2b7f5be5257068c1d3bb8cb73dfeaa0e482c16b468872179a0b1720e2ad2` matches all three Opi terminals. Relevant current integration/material identities match. Synthetic export instrumentation/__pycache__ does not enter committed source applicability.
- All six raw stdout digests match their bundle entries and have one AgentEnd; Opi/pi tool start/end counts TB2.1=4/4, TB3=95/78, DeepSWE=100/92. No raw reasoning text is included here.
- Six sealed time_limit_seconds/agent_timeout_seconds/verifier_timeout_seconds/max_output_tokens/max_output_bytes are null; concurrency 1 / max_trials 2. Their exact output-control proof, defaults and finite count do not establish explicit pre-run total usage/time scope.
- Applicable default Rust proof 369 passed,0 failed,11 ignored across 14 result blocks. Other exact-client/cancellation/cleanup/provider-failure proofs each execute 1 pass; none executes all controlled pinned grader witnesses or successful genuine saved-artifact regrade.
- Docker Linux engine admission succeeds (linux/x86_64). Retained material/6 files exist and match the manifest digests. Their referenced `/tmp/opi-p19-b6-cache-20260927/pins/static/linux-x86_64.json` is absent; pi/task/runner/uv dependencies in the same cache root are also absent. The native loader reads the pinned static lock before task dispatch. Its generic IO message says "cannot read native material manifest"; the retained manifest itself exists, so this is a missing cache prerequisite, not a demonstrated runtime bug.
- The explicitly authorized `native_tb21_artifact_witness` was executed with offline Cargo, filtered credential environment, the persistent Cargo cache, exact retained TB2.1 material and a 60-second bound: exit101, 0 passed / 1 failed / 0 ignored, CLI exit2 at material admission. No Agent, provider or Docker task launched; no download or credential-file read occurred. The other native filters/regrade were not repeated against the same missing prerequisite. The diagnostic trial root is `/tmp/opi-eval-terminal-bench-2.1-oracle-witness-lqeJkm`.

Fresh local proof files below are relative to the unique archive parent in the header. They pin review evidence, not registered sources or progress state:

| Local proof | SHA-256 |
|---|---|
| `acceptance-probes.py` | `484ab82488ece902e02d92ebcc564e9afb7f94179ba83744a95cb38dd00d5fe1` |
| `acceptance-red.log` | `1d796dc1dd8084896d716352e39e4162c70ad4c9b6317b4baac13848393ea950` |
| `capture-bound-red.log` | `e4abd6fc8b3f9272e21e7c28fdb79c9ef95bce35c19d7750b9b755ac2f4756da` |
| `reasoning-projection-red.log` | `2702bedd631006f0e02352e4c0293144ec56fae15805d42cdf8ce82e93e8c970` |
| `security-red-before.log` | `c3248ab9283c663a9d55dea9fc5882bf9a275fc6f2c699b91674e0b3e7afdede` |
| `host-credential-red.log` | `157b80eed2bbc42e5480cb055e365d34c4c4167e65e2b01fe0bd131fd4fd2522` |
| `security-persistence-red-before.log` | `cd9af9bf994ace255035ca92effeea5865c093c83aa90e4e274a099b69a9acfd` |
| `security-persistence-red.sh` | `c7d335de62c9d6f2e6ee05cfa3f129f72ce1fe50ffca30e12ded370f230083c1` |
| `security-first-sink-probe.py` | `76ae7bd4fe541454fd0a8fc2a60f337555207c15aec86590d2b5af0e504adc53` |
| `security-first-sink-red-before.log` | `106f1ee914d63b87a956d25846f09ff7d9b3bbc4a2cd7869a4ce8e54ab90c3ff` |
| `security-shape-metadata.json` | `d41a7c0515ef6df0b97c43669422e7283bf510b542e166e21e72367951f54edb` |
| `security-shapes-current.py` | `39bd9f04fd28d025c78abfb40942752549652b11ffe979a9a689a5f561f08c22` |
| `src/crates/opi-eval/src/experiment.rs` | `090d814e0557ff9daec8cd60a2966ac8ffdf32e45c8a1c6565ef08bf29314f0a` |
| `src/crates/opi-eval/tests/security_plan_probe.rs` | `3dc6570b9f179b7e9d11a92f724b33de326ccc975daeab29d39cc41627cf9d6a` |
| `tooling-uv-only-probe.py` | `e7b070f4314503daf26db8c025f4bb448c12cfbd32115633d2e920ad4900431a` |
| `tooling-current-fixture-probe.py` | `f941840cfea0535ae15d20475241228374258735d8823c6165257c014caeb7d1` |
| `tooling-external-route-red.py` | `c209e4b6e2e2bfaaaa5b83ab6809764775ca8ff64bfda1f4fb47f0012cb099f2` |
| `tooling-external-route-red.log` | `6a70f6b2742bfee8b82ffba8d74a48d7c91d189435da65bc1433143de7c944fa` |
| `native-cache-admission.sh` | `a1eef3bc7111d3b328fbf8114c79c19cbe8d798ecd37f18cf79736a70cb025ab` |
| `native-cache-admission.log` | `8d98b2853ce29168339e118a67260fbd2ac02a6b5d0a73465d8695228646382b` |
| `native-cache-inventory.py` | `ac3df79632af90658a66472065ba8f07f04f8e5307bff1c77d6798d923234b96` |
| `evidence-verification.json` | `6ce0f9c57244c9c6861995144856cc6211903005e6e8bf534542cb0cce2641f8` |

UV/baseline formatter observations are in the planning tool transcript and exact observed fields. Source bytes remain bound by remediation_head. Production green-after remains not-run for every source, including the successful temporary fixture prototype.

## Authorized native evidence route pending usable material

Existing opt-in filters remain retained. The TB2.1 admission attempt above is executed and failed; no witness success is claimed. The user authorized these no-model commands using verified local caches. Supply verified material/6 with pinned runner/task/runtime/integration identities and cached immutable images/dependencies; drift/missing prerequisites refuse. Do not fetch missing materials or run models within this authority. Existing exact filters: native_tb21_artifact_witness, native_tb3_artifact_witness, native_deepswe_patch_witness, native_verifier_only_regrade_witness.

Material/6 here describes the committed baseline. After B8, fresh controlled native jobs use its admitted material/7 and capture/helper bindings; recheck all affected witness predicates against that final source. Retained material/6 remains eligible only for original-seal offline verify/report and verifier-only regrade under B8's legacy boundary. Never relabel retained material or treat its old producer proof as fresh dispatch admission.

```sh
OPI_EVAL_NATIVE_WITNESS_MATERIAL="$VERIFIED_MATERIAL" cargo test -p opi-eval --offline --locked --test native_driver "$EXACT_FILTER" -- --ignored --exact --nocapture --test-threads=1
```

Require 1 executed pass per filter and retain controlled oracle 1 / nop 0 grader/collected identities; regrade witness needs material covering all 3 families. That controlled regrade proves mechanisms. R31 also needs production regrade of genuine immutable saved Agent artifacts:

```sh
"$BUILT_OPI_EVAL" regrade --root "$IMMUTABLE_ACCEPTANCE_FAMILY_ROOT" --native-material "$VERIFIED_FAMILY_MATERIAL" --out "$NEW_DERIVATIVE_ROOT"
```

Require exit 0, nonempty expected regraded trial records, source/artifact/task/runner/verifier identity, fresh derivatives, unchanged original bytes and zero Agent/provider calls. These native filters remain no-model diagnostics. The fresh paid acceptance/retry scope is separately authorized above; D2 requires its source-bound prospective freeze and actual execution evidence.

## Final Verification

Plan-only completion requires dispositions/plan validators, documentation checker and whitespace check. Intentional red checks verify current defects; no production green or Phase conformance is claimed. Recheck HEAD and staged/unstaged/untracked inventories at handoff.

After a validated digest-bound apply invocation, each batch's change is the step and its stated check is `step -> verify`. Run every added/modified test binary/filter and the deduplicated final union on relevant source. B9 additionally requires exact external-lock CI/artifact suites, a fake-CLI Bash delegation/refusal subprocess and lock/fixture conformance with synchronized transport documentation before resealing. C005/C006 follow-up remains separately subject to its actual material/privacy/count/deadline admission; source-fix green checks cannot substitute for that evidence.

```sh
python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

Workspace union is the registered final-state gate, distinct from D2/D3 operator/native proof. Default suites remain offline and cannot activate ignored/live cases because credentials exist. Repeat successful evidence only after changes/failure/applicability gaps. Read complete Unreleased before a future edit, reuse subsections, preserve releases. B1/B7/B8/B9 update both README counterparts. Fixed assurance protocol defines no Chinese plan sibling, so this pair has no counterpart to synchronize.

## Exclusions

| Source finding | Disposition | Current evidence/authority |
|---|---|---|
| C/P19-AUD-005 | Cannot confirm closure; no production repair proposed | Original freeze remains missing. Prospective paid scope is explicitly fixed: glm-5.3-flash,2 retries per failed logical trial,3600s, unlimited fees. No fresh run or complete source/material/input freeze is claimed; Major/conformance gap remains. |
| C/P19-AUD-006 | Cannot confirm; no demonstrated runtime defect | No-model cache execution approved, but TB2.1 admission fails on absent pinned cache; required successful native witnesses/genuine regrade remain missing. Major/conformance gap remains. |
| P/P19-AUD-003 | Info/No action | Exact removed targets confirmed; documented history resolution/null normative criterion requires no edit. |

## Test impact and materialization

- **add**: future bound/reasoning, host-mode/refusal, acceptance-root, early capture privacy/refusal and CI delegation/refusal cases in existing scenario owners; planning adds only temporary probes outside repository.
- **update**: future connection/projection assertions, capture/material/handoff/terminal-proof shapes and original 2 current-prepare fixture methods; CI receipt/fixture consumers preserve negative/order and transport intent.
- **retain**: formatter assertion, native identity/cleanup/credential tests, four opt-in witnesses and immutable evidence.
- **delete**: duplicate CI materialization/grading policy only in B9; retain intentional manual dispatch/upload/receipt behavior. No legacy bundle deletion or rewrite.
- **none**: current repository runtime/test changes, documentation-only B6, evidence-only Cannot confirm exclusions and registry Info.

This remediation task owns edits only to remediation.plan.md and its JSONL sibling; the six workflow-maintenance carry-in files remain separate and unchanged. No ledger, production source, audit, result or retained bundle is materialized. Committing remains a separate explicit request; plan-only commits preserve remediation_head under the shared baseline rule. A commit touching the separate workflow files or any other non-plan path requires a new plan baseline. Validate this ready plan and approve only its emitted digest before apply. C005/C006 remain unclosed unless separately attributable successful evidence satisfies their retained predicates; apply results must preserve Cannot confirm where that evidence is unavailable. Any index/member/plan byte change invalidates earlier approval.
