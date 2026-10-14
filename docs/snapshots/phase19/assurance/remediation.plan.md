# Phase 19 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `9c9d9c3487f895bf0486359091b0119599caae432d9f7053a3c297d117df6c2a`
**Remediation head**: `f9806a775a20767bd3ccbc2d6233164fbff2447f`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=[]; unstaged=[]; untracked=[]; committed=none-for-this-task.
**Unresolved decisions**: none

## Admission and scope

The live schema-2 index revision 29 validates. Its two indexed runs contribute exactly 12 source keys (11 Major, 1 Minor). Reports do not replace machine records; no peer is canonical. All source severities are retained. Findings with related families remain separate unless one root change and one falsifiable closure predicate closes both; no such cross-source closure batch is asserted here.

| Member | Full run | Findings SHA-256 | Source verdict |
|---|---|---|---|
| audit.codex.gpt61 | `phase19-codex-gpt61-ae6dff6-20261014t044949z` | `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` | FAIL |
| audit.pi.glm53 | `phase19-pi-glm53-7a0fe85-20261014t073512z` | `b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356` | FAIL |

The current committed planning baseline is `f9806a775a20767bd3ccbc2d6233164fbff2447f`, exported using `git archive HEAD` into `C:/Users/Luiz/AppData/Local/Temp/opi-p19-plan-refresh-62vp1n1y/source`; no Git worktree was created. The original plan-stage builds, fixture probes and tests ran at source head `4514b8c0169c39e8263a1dcb53c5c636ffb95a63` in `C:/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992`. Their original commands, exits and executed counts are retained, rather than described as new executions at the refreshed head.

The user removed the four archived files for `phase19-pi-glm53-2e82c99-20261002t204027z`, which contain the previous repair's audit content; commit `f9806a775a20767bd3ccbc2d6233164fbff2447f` records that deletion and the plan pair. That archived run is outside the current indexed finding union. This adjustment does not restore or reconstruct its content, use it as closure authority, or require its paths during apply. The installed current members and their validated advisory appendices remain byte-identical. Removal of that audit snapshot does not supply the missing original operator/native evidence for006/007/102.

The baseline refresh independently compares all 982 tracked path/mode/blob records outside `docs/snapshots/phase19/assurance/` between the original probe head and the new remediation head: all are identical. Only the fixed plan pair and those four history deletions differ. Source, tests, manifests, locks, clients and registered specifications are unchanged; all 22 listed temporary proof hashes and all eight active member file hashes also match. This proves applicability of the retained plan-stage evidence without rerunning unchanged checks. The read-only refresh record is `C:/Users/Luiz/AppData/Local/Temp/opi-p19-plan-refresh-62vp1n1y/baseline-refresh.json`, SHA-256 `2bdb5ec9ab0ac8a73194d0d54bafb838ca4c53efc7091e71a06a6d4722cb64de`. The existing result pair remains outside this adjustment. No ledger or specification changes are admitted.

The supplied location is `\\wsl.localhost\Ubuntu\home\luiz\phase19-acceptance-r2`. Registered Phase19 Sections 3/7/8 admit TB2.1/TB3 and require four attributable Opi/pi trials. Existing DeepSWE evidence remains scoped out and immutable. This planning adjustment performs no remote provider calls, private auth reads, material downloads, production edits, commits or branches. Isolated no-model native Docker diagnostics and actual-client loopback tests are permitted plan-stage checks; the prior blanket exclusion of those checks was too broad and is withdrawn. Future production apply still needs a ready exact plan and explicit apply invocation; real-provider replacement runs remain separately authorized operator work.

The user selected the concrete /3 contract and explicit independent expected-identity input in this continuation. The user also confirmed only the supplied operator directory is available and selected code-first planning with evidence gaps retained. These decisions resolve the code-plan authority/scope: 9 repair batches plus explicit Cannot confirm exclusions for006/007/102. Mandatory Phase criteria and their original severities are unchanged, so Phase acceptance remains blocked by those exclusions. D4 has current native red evidence and bounded B8 ownership repair under existing AC-008/010. This ready plan does not authorize production edits until a new exact-digest apply invocation. The user-requested adjustment starts from a clean worktree at the refreshed head and modifies only the fixed plan pair; no unowned overlap exists.

## Current requirement comparison

All 71 indexed requirement records were retained as source records: codex has 52 and pi has 19. Each criterion identity is `(path, sha256, citation)`. The linked citations differ between the broad pi rows and split codex rows, so these rows remain separate; apparent agreement is not an exact-criterion merge and a met broad row cannot refute a narrower defect. The following matrix contains every requirement linked to a current finding; the unlinked records remain in the pinned sidecars.

| Member | Requirement | Original state | Original current findings | Exact citation (same registered source digest) |
|---|---|---|---|---|
| audit.codex.gpt61 | P19-AC-001-independent | not-assessable | P19-AUD-007 | P19-AC-001; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-002-artifact | not-met | P19-AUD-001 | P19-AC-002; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-003-witness | not-assessable | P19-AUD-006 | P19-AC-003; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-004-witness | not-assessable | P19-AUD-006 | P19-AC-004; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-006-clients | not-assessable | P19-AUD-006 | P19-AC-006; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-008-coverage | not-met | P19-AUD-002 | P19-AC-008; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-008-unknown | not-met | P19-AUD-004 | P19-AC-008; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-009-regrade | not-assessable | P19-AUD-006 | P19-AC-009; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-010-privacy | not-assessable | P19-AUD-006 | P19-AC-010; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-012-freeze | partially-met | P19-AUD-003 | P19-AC-012; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-012-attempts | not-assessable | P19-AUD-007 | P19-AC-012; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-012-platform | not-assessable | P19-AUD-007 | P19-AC-012; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-012-gates | not-assessable | P19-AUD-007 | P19-AC-012; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-013-pi-only | not-assessable | P19-AUD-006 | P19-AC-013; Sections 3-10; mapped parent clauses in Section 1 |
| audit.codex.gpt61 | P19-AC-013-contracts | partially-met | P19-AUD-005 | P19-AC-013; Sections 3-10; mapped parent clauses in Section 1 |
| audit.pi.glm53 | P19-AC-004 | partially-met | P19-AUD-103 | #7. Requirements and acceptance scenarios row P19-AC-004; #5. Native execution and material contract TB3 row |
| audit.pi.glm53 | P19-AC-008 | partially-met | P19-AUD-104, P19-AUD-105 | #7. Requirements and acceptance scenarios row P19-AC-008; #6. Evidence, failure, and reporting |
| audit.pi.glm53 | P19-AC-011 | partially-met | P19-AUD-101 | #7. Requirements and acceptance scenarios row P19-AC-011; #8. Verification and local acceptance handoff |
| audit.pi.glm53 | P19-AC-012 | partially-met | P19-AUD-102 | #7. Requirements and acceptance scenarios row P19-AC-012; #8. Verification and local acceptance handoff; #3 Phase exit paragraph (at least four real trials, TB2.1/TB3 only) |
| audit.pi.glm53 | P19-GATES | not-met | P19-AUD-101 | #8. Verification and local acceptance handoff (repository verification union and actual WSL/Linux acceptance requirement) |

The registered English design digest is `f507f29d91ddfc919ead6fa84df8d5f912b04af1d84ed5e68777e20c80c135c5`; Chinese is `7d97f00f3a06723765e8cb2c8ea7ed13933defc8d7c230bdc0d165ad1284e7dc`; parent spec is `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db`. These current registered obligations, parent CTRL-004/005/006/007 and domain authority boundaries govern decisions. In particular pi AC-002/003/006/009/010/013 met states do not dispose codex 001/006, and pi broad AC-008 remains partially-met independently of codex 002/004.

## Current Finding Verification

| Source run / Finding ID | Verification | Source -> final severity and current rationale | Closure key / family | Batch | Decision |
|---|---|---|---|---|---|
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-001 | Confirmed | Major -> Major; Retain Major. A source-bound current stand-in traversed the production result reader and CLI: omission of observation admitted substituted declared bytes and a native headline. This is fixture evidence, not an actual native grader/Agent claim. | native-binding.nonempty-declared-observation-completeness / native-binding.delivered-artifacts | B1 | fix:reject-unobserved-nonempty-declared-inputs |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-002 | Confirmed | Major -> Major; Retain Major. Current production CLI and saved-report paths independently admit invalid single-subject completion; no-edge pair all() is insufficient. Actual invalid counts and repetition state are observed, not inferred from another reviewer. | report.single-subject-valid-completion / report.outcome-coverage | B2 | fix:require-valid-single-subject-completion |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-003 | Confirmed | Major -> Major; Current structural acceptance admits omitted operator/source records and malformed Opi source. Source-to-build, original command provenance and usage/time/platform freeze admission are absent; the registered operator handoff requires those semantics. | acceptance.required-execution-provenance-and-freeze / acceptance.evidence-admission | B9 | fix:enforce-approved-acceptance-provenance-v3 |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-004 | Confirmed | Major -> Major; Retain Major. A fresh private fixture changed only Opi token availability; public run/report preserved unknown diagnostics but advertised measured usage with no usage-bearing records. Fixture numeric control alone is not native provenance. | report.native-usage-supported-by-measurement / report.native-facts | B3 | fix:derive-native-usage-from-measured-evidence |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-005 | Confirmed | Minor -> Minor; The independent walkthrough's final handoff contradicts four-trial registered scope and uses script/binary paths never installed by its cache seed instructions. Primary English/Chinese crate README is synchronized. | walkthrough.independent-four-trial-offline-verifier / acceptance.user-handoff | B7 | fix:synchronize-independent-walkthrough |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-006 | Partially confirmed | Major -> Major; Partially confirmed within the admitted current proof selection. Fresh actual-client protocol matrix now qualifies all three local wire paths for Opi/pi: 1 executed test PASS, 30 client invocations, 6 wire logs, 75 requests and 12 verified real bash file effects. Original client/source/package/binary identities are byte-bound. This is loopback conformance, not optional real-service qualification or all finding closure. Native artifact witnesses, both-family real-artifact fresh regrade, grader/export canaries and pi-only native qualification remain distinct proof work. Fixture/package summaries and four real native results are useful counterevidence to absence of execution; no global absence or severity reduction is claimed. A bounded current original pi-only TB3 regrade verified the source seal, saved results and cached official image, then exited1 with verifier-network-required and 0 fresh grades. Pi-only material admission is established; successful official regrade/canary qualification is not. User explicitly chose code-first planning with these evidence gaps retained. | acceptance.qualified-native-conformance-proof / acceptance.evidence-handoff | none | no-action:cannot-confirm-qualified-native-proof-unavailable |
| phase19-codex-gpt61-ae6dff6-20261014t044949z / P19-AUD-007 | Partially confirmed | Major -> Major; Current /1 supplied manifest is rejected by the current /2 verifier (fresh exit 2/accepted=false). Exact admitted records do not establish complete original independent preparation/failed-attempt/platform/final workspace handoff. Preserved four-outcome reports are counterevidence to absence of real trials; they are not complete current operator proof. User confirms only the supplied directory is available and chooses code-first planning with missing original freeze/build/command facts retained as unclosed evidence; no retrospective fact or new provider run is authorized. | acceptance.current-assembled-handoff / acceptance.evidence-handoff | none | no-action:cannot-confirm-original-operator-handoff-unavailable |
| phase19-pi-glm53-7a0fe85-20261014t073512z / P19-AUD-101 | Confirmed | Major -> Major; The deterministic current workspace pin check fails, and cold preparation compares the source-export Cargo.lock to the same stale pin before building. The admitted regression gate is blocked. | material.current-workspace-cargo-lock-pin / material.input-identity | B4 | fix:synchronize-current-workspace-lock-pin |
| phase19-pi-glm53-7a0fe85-20261014t073512z / P19-AUD-102 | Confirmed | Major -> Major; Fresh schema-admission CLI rejects the exact supplied /1 root at exit 2; four newer raw-outcome reports do not replace the prescribed /2 handoff. Static material identity changed and requires a new derived view, never mutation or relabelling of old seals. User confirms only the supplied directory is available and chooses code-first planning with missing original freeze/build/command facts retained as unclosed evidence; no retrospective fact or new provider run is authorized. | acceptance.current-assembled-handoff / acceptance.evidence-handoff | none | no-action:cannot-confirm-original-operator-handoff-unavailable |
| phase19-pi-glm53-7a0fe85-20261014t073512z / P19-AUD-103 | Confirmed | Major -> Major; Retain source Major: required TB3 declared-artifact/native reward diagnostic has a critical-path test expectation gap. Production missing-artifact fail-closed behavior is correct. Confirmation is static expectation/binding mismatch; this worker did not reproduce native execution. | tb3.declared-artifact-native-witness-reward / native-conformance.witnesses | B5 | fix:test-only-declared-tb3-witness-baseline |
| phase19-pi-glm53-7a0fe85-20261014t073512z / P19-AUD-104 | Confirmed | Major -> Major; Retain Major: current ignored P19-AC-008 report acceptance filter rejects intentionally supported typed retry exhaustion while production classification remains invalid/unscored. Source-critical test gap, no production behavior change required. | native-provider-report.pi-retry-exhaustion-reason / native-conformance.provider-report | B6 | fix:test-only-exact-pi-exhaustion-assertion |
| phase19-pi-glm53-7a0fe85-20261014t073512z / P19-AUD-105 | Partially confirmed | Major -> Major; Retain Major. A current source-bound real TB3 diagnostic reproduced invalid native timeout settlement, but refuted the claimed 64 MiB cause for these cases. Positive has valid2/invalid0; timecap has valid0/invalid2 despite native AgentTimeoutError and reward0. Retained streams are only51/140B with stderr0. Group SIGKILL kills the capture helper (exit137), leaving no finalized receipt. Missing evidence still correctly refuses; the root repair is owned Agent cancellation plus helper finalization, not cap relaxation. | native-stage-deadline.admitted-capture-graded-timeout / native.supervised-capture | B8 | fix:terminate-owned-agent-group-and-preserve-capture-finalizer |

## Current proof and counterevidence

- Current WSL CLI build: `cargo build --locked --offline -p opi-eval --bin opi-eval`, exit 0. Shared external cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`; CLI digest `eef65fb7aae5775399d179881167921d7852204d9b7f1bbb90bc9281248c17f4`. No cache cleanup or session-specific target.

- B1 executed one export-only regression (4 differential public CLI cases), failed on the two observation-omission substitutions. B2/B3 executed 6 fixture cases and 12 original run/report subprocesses with saved argv/stdout/stderr/exits. D1 executed 3 Python tests: accepted structural control passed, 2 expected rejections failed; original family bytes stayed unchanged. B4 executed 1 pin test and failed it. These are local structural/fixture facts, not native operator acceptance.

- Exact appendix candidates were independently hash-checked before use. All 104 candidate inventory byte identities/lengths still match. The inventory is a bounded proof selection, not a global absence assertion. The saved four-outcome TB2.1/TB3 reports and unchanged source dependencies support limited applicability; selected saved report bytes cannot prove unrecorded original workspace or native execution. Package summaries are not workspace gates. No source/key/severity/closure assignment comes from an old claim.

- Fresh external schema admission: current verifier invoked against the exact supplied `/1` manifest returns original exit 2, accepted=false, no families. The Companion sentinel was never reached; this proves schema rejection only. Original record `p19-parent-probes/schema-red-record.json`, SHA-256 `2d88cf88d2c5679c30a6ad175f77f64c8e9a359140241a9501541df7208a44aa`.

- Continued D4 native red: exact TB3 input, unchanged current Harbor module and cached pinned image were independently checked. Four real diagnostic trials ran: positive valid2/invalid0, timecap valid0/invalid2; both timeout results contain AgentTimeoutError and real native reward0.0. Helper exit137 and unreadable/missing receipt precede invalid settlement; retained stdout51/140B, stderr0. Cargo original exit101, one executed/failed test,139 filtered, no ignored execution. The 64 MiB attribution is refuted for these cases; the outcome gap is partially confirmed and has bounded B8 ownership repair. Overflow/privacy refusal is retained.

- Fresh actual-client matrix PASS: one selected/executed native_protocol_client_matrix,30 client invocations,6 wire logs,75 requests and12 real bash file effects. Opi/pi binaries/packages/source receipts are hash-checked; every selected endpoint is127.0.0.1 with dummy credentials and an isolated environment. Optional real-service qualification is not claimed.

- Six exact inventory-pinned original run stdout/stderr/exit records were checked: both tb21-z2 and tb30-z3 original exits0 and two trials per run. Current read-only platform observations show WindowsX64,UbuntuWSL2 and local Linuxx86_64 Docker29.7.2. Current facts do not fabricate the old operator freeze. The prior summary-only limitation is narrowed, not removed for all original command/freeze/workspace facts.

- Fresh original pi-only official TB3 regrade: all21 candidate source pins and source seal verify; actual saved results.txt and exact cached verifier image exist. The private operational material contains only pi and is admitted. Current CLI rejects before new grading with verifier-network-required (exit1,17.226s,0 fresh grades), preserving task network policy. It proves the negative boundary and pi-only material admission, not positive regrade/canary conformance. No synthetic grader or source policy rewrite substitutes for this result.

Matching green checks B2/B3 copy the same discriminating cases into report_contract, asserting production CLI and offline report parity; export probe argv is retained as red provenance, not reused against occupied output roots. Current raw-probe paths below are relative to the original probe export `C:/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992`, not the refreshed source export or repository deliverables. Before reuse, check their exact bytes, original command exits and the applicable source/build/client/material identities. These files are temporary evidence and must be preserved with an eventual approved evidence handoff.

| Current proof | Exact SHA-256 | Limit |
|---|---|---|
| `.remediation-probes/script-findings/handoff.json` | `987544c0c13be547549995ed87d6597b63bb8fa93850e41acdbb1f7f2f3007f1` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-findings.json` | `cf7c008f825c074d56589593fa892e77efff0bd569c26e5dde66c33105bd46f5` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-parent-probes/schema-red-record.json` | `2d88cf88d2c5679c30a6ad175f77f64c8e9a359140241a9501541df7208a44aa` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-runtime-probes/runtime-report-summary.json` | `1821e5b9137ab71d1a36a42488adc5a0d3ad70c7501c2bef0ae120dcb9ab20c9` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-runtime-probes/runtime-report-probe.py` | `e52cbebc37502d742ff3f3994c159b4e9c74acd3751ca48f2b824caacc7223e7` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-runtime-probes/native-artifact-test.log` | `c6518e9635f3f5f5daa7284b2191d6c0c4455f13869a6ca3c90bcba4ab508890` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-delay-probe.py` | `bc2be7ab2c79af44123d1a5dc20ed2d8adf89ef3f8e4210dd8241fa80fcdb3fd` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-delay-probe.json` | `15ec60f0b8960f77138594b22b7e86a59f98d837b833cb02a088895e523170eb` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-stage-probes/timecap-handoff.json` | `2b1ba5ffe69a5e877fd5fae377b4d68fd3e79f63708306fa4323690c0ba1ec8c` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-stage-probes/test.log` | `438b734001d3e244fb6c7a487e208ae339b93d30ae38ea7ec437467e826bfe6a` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-d3-protocol-proof/execution.json` | `7ec533e7fae5d86b364eb7674b19dd7d19bc1883a14d98a8cb3e4b878b3ef860` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-d3-protocol-proof/payload-counts.json` | `98a7f91ca739a242ec17378cc5feaa5f9fc34d2e30696c700154d4f40e67e082` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-parent-probes/continue/original-run-log-projection.json` | `6bab606f1a0adcf6717aaaa086da5d2f13be9d6ca10ee61f3884d8b00a837ab3` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-parent-probes/continue/current-platform.json` | `60b7671a9f1cfb6350f10b005e9d3c7d1cbbbc893e1e71f698b41885e22ae730` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-d3-handoff.json` | `eff63f27ed1347b144f06543200bb6ac14242ce2cd57ecb1ffd7df63d6460d31` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `native-test-d3-official-regrade-proof/execution.json` | `721ba97f3e766532e3b636fe682db03b5804539cb8beff61758186a1979aaa36` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `.remediation-probes/script-findings/continue-D1/proposal-minimal.md` | `5b17aa8c7e0506c40b8a718dfe7d068241fb6bbde70bcb0740d67221fd97a6da` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `p19-runtime-probes/runtime-findings-handoff.json` | `90f2ef34f40d96e548b0d870a2ee5b3ec339d1f16553fda4426abfd8c7731744` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `.remediation-probes/script-findings/test_acceptance_admission_red.py` | `ed90003f3b86f9f5631e8851c02d0546e648c96b2e2375ea0559e49195121e78` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `.remediation-probes/script-findings/acceptance-admission-red.log` | `e36457f7c61029b811ab4232653d06d6e10c81b26e10e9b7f7ec251206c26e8b` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `.remediation-probes/script-findings/workspace-pin.log` | `02d16bd6663a91a232d2cee777e1c4b1913f374e94b5d6b431e973a35c4a6b36` | Fresh plan-stage evidence; scope described above and in the source handoff. |
| `.remediation-probes/script-findings/producer-pins.json` | `0a9505b3ff7647284a7afaeaaf78fe00aece8db2116c7486655703b4cf21fb4d` | Fresh plan-stage evidence; scope described above and in the source handoff. |

## Unresolved Decisions

none.

## Recorded scope decisions and remaining evidence

D1 /3 was approved by the user; B9 implements that exact bounded contract. D2/D3 evidence gaps are explicit exclusions under the user-selected code-first scope. They are not unresolved implementation choices and remain blockers to Phase conformity. The following record retains their actual evidence boundaries, not a new source of implementation progress.

| ID | Decision/input record | Evidence boundary | Recorded route | Owning authority |
|---|---|---|---|---|
| D1 | User approved acceptance /3 and the proposed independent expected-identities input. | Identity/byte binding is enforced; cryptographic build causality and completeness beyond recorded scope are not claimed. Actual old operator qualification remains outside B9 code closure. | B9 implements the inline freeze/commands/attempts contract below with existing build receipts, strict typed admission and synchronized consumers/docs. | Explicit user decision in this continuation; registered AC-013 and Sections4/6/8/9. |
| D2 | Qualify original retained attempts, independent prepare/run/report handoff, platform freeze and final applicable workspace proof; choose a new derived evidence view for 007/102 after D1 and final fixes. | Exact current manifest remains /1 and binds older selections; raw operator summary and four saved outcomes do not supply every original proof or current source/material linkage. B4 changes the static-lock digest. | Prefer offline assembly from byte-verified original proofs into a new view with current per-family admission, fresh material identities and separately labelled DeepSWE. Never rewrite old evidence or fabricate source/build/freeze claims. If essential originals are unavailable, the operator must choose separately authorized replacement evidence. Run final workspace gates once for final relevant state. | Evaluation product owner/local operator under AC-001/012 and Section 8. |
| D3 | Qualify remaining source/build/client/material-bound native artifact witnesses, fresh real-artifact offline regrade, grader/export canaries and pi-only native execution for006. | All six local protocol combinations now have fresh actual-client proof; this does not establish the other independent native predicates.103/104 require bounded test corrections and B8 owns the reproduced timeout correction. | Continue isolated no-model proof work with current admitted/explicitly derived material. Preserve original hashes and all execution evidence; no additional permission is needed merely because a local Docker grader or fake-server client runs. Failed prerequisites remain explicit. Real-provider replacement acceptance is separate. | Native integration/Evidence/Test owners; current source and test assertions determine qualification. |

D2/D3 disposition decision: the user chose code-first planning and preservation of missing/unqualified evidence. These requirements are excluded from this apply as Cannot confirm, with no fabricated facts or positive native qualification. The source006 official saved-pi regrade now has a concrete verifier-network-required refusal even though artifacts/cache are present; any future offline-equivalent policy or alternate qualifying task must respect its declared verifier rules and receive the required scope/contract admission. No such policy change belongs to these9 code batches.

There are no pending dispositions.003 belongs to B9 and105 to B8;006/007/102 have explicit no-action:cannot-confirm dispositions and will remain unclosed. They cannot be labelled Closed by a passing code gate. A new public contract or a source change outside the approved9 batches requires a new plan. No production fix is claimed during planning.

D3 current diagnostic execution routes (not executed in this plan): `native_tb21_artifact_witness`, corrected `native_tb3_artifact_witness`, `native_protocol_client_matrix`, `native_output_control_client_matrix` where effective-control proof requires it, corrected `native_provider_failure_report`, `native_real_client_offline_regrade` and `native_offline_regrade` with both admitted families, plus pi-only native execution with no Opi executable/material. For each selected ignored filter capture full and ignored inventories and use `.agents/skills/opi-audit/scripts/check_test_execution.py --filter <exact identity> --exact --selection ignored --expected-count 1`. Assertions must establish the linked predicate, not only execution count. Shared source finding 006 retains separate proof requirements; DeepSWE/all-families filters do not qualify current acceptance.

## Closure Batches

### Batch B1: native-binding.nonempty-declared-observation-completeness

**Closure predicate**: The same substituted nonempty declared input is unscored and unsealed with complete, empty, or partial delivered observation; valid observed input still grades, and unrelated files stay excluded.
**Dependencies**: none
**Verification union**: new observation-omission regression; existing native_result_binding_rejects_every_substitution; native_tb30_rejects_undeclared_and_substituted_collection; full native_driver normal selection; focused lib tests; affected clippy; applicable workspace union below

#### Fix B1.1: P19-AUD-001

- **Finding source(s)**: `phase19-codex-gpt61-ae6dff6-20261014t044949z` + `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` + `P19-AUD-001`.
- **Decision**: `fix:reject-unobserved-nonempty-declared-inputs`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/benchmark/process.rs`, `crates/opi-eval/tests/native_driver.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Add the reverse consumer cross-check for each ordinary file in a nonempty declared collection: its normalized source and digest must be covered by terminal delivered observations. Preserve existing digest/substitution/undeclared and native identity rejection. Copy the export-only discriminating case into the existing native_driver surface. Do not broaden the observation scan or invent an observation schema. This closes admitted substituted collected bytes under empty/partial observations; it does not claim proof of omitted Agent-side files or truly empty directory contents. Read the complete Unreleased section before its future edit; record the user-visible correction under the existing Fixed subsection and preserve released sections.
- **Closure predicate**: The same substituted nonempty declared input is unscored and unsealed with complete, empty, or partial delivered observation; valid observed input still grades, and unrelated files stay excluded.
- **Red-before**: cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_observation_omission_rejects_substitution -- --exact; FAIL, original Cargo exit 101; 1 test executed/failed, 0 passed/ignored, 138 filtered; all 4 differential CLI cases executed. Happy and observed-substitution rejection controls behaved correctly; empty and unrelated-only observations admitted the substituted file, exit 0, completed, sealed, graded-native reward 1. Raw log: p19-runtime-probes/native-artifact-test.log.
- **Green-after**: cargo test --locked -p opi-eval --test native_driver p19_plan_probe_observation_omission_rejects_substitution -- --exact; expected The same substituted nonempty declared input is unscored and unsealed with complete, empty, or partial delivered observation; valid observed input still grades, and unrelated files stay excluded. Observed: not-run.
- **Test impact**: add observation-omission regression; retain existing native binding/collection negatives; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_observation_omission_rejects_substitution -- --exact.
2. Apply only the root change above -> verify: cargo test --locked -p opi-eval --test native_driver p19_plan_probe_observation_omission_rejects_substitution -- --exact.
3. Verify the complete changed surface -> verify: new observation-omission regression; existing native_result_binding_rejects_every_substitution; native_tb30_rejects_undeclared_and_substituted_collection; full native_driver normal selection; focused lib tests; affected clippy; applicable workspace union below. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B2: report.single-subject-valid-completion

**Closure predicate**: A single-subject run with only invalid trials exits 1/incomplete and reconstructs incomplete offline; invalid/unknown repeated outcomes are not consistent. Happy and benchmark-valid timeout/zero controls remain completed without an artificial comparison edge.
**Dependencies**: none
**Verification union**: fresh CLI invalid/single/zero/timeout cases; assembled_run and report_contract; affected clippy; applicable workspace union below

#### Fix B2.1: P19-AUD-002

- **Finding source(s)**: `phase19-codex-gpt61-ae6dff6-20261014t044949z` + `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` + `P19-AUD-002`.
- **Decision**: `fix:require-valid-single-subject-completion`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/runner/experiment.rs`, `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/assembled_run.rs`, `crates/opi-eval/tests/report_contract.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Require all planned trials to have valid graded outcomes as well as durable settlement and comparable declared edges for completed. Apply the same predicate to live run and sealed offline report. Treat repeated absent/invalid headlines as existing incomplete repetition state instead of equality of nulls. Retain valid zero, graded Agent timeouts and no-edge single-subject success. Add public CLI regressions for malformed Agent streams and invalid verifier outcomes and assert offline parity. Read the complete Unreleased section before its future edit; record the user-visible correction under the existing Fixed subsection and preserve released sections. Synchronize the primary English/Chinese exit-0/completed and repeated-outcome contract descriptions; all other declared report fields remain current.
- **Closure predicate**: A single-subject run with only invalid trials exits 1/incomplete and reconstructs incomplete offline; invalid/unknown repeated outcomes are not consistent. Happy and benchmark-valid timeout/zero controls remain completed without an artificial comparison edge.
- **Red-before**: python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992/p19-runtime-probes/runtime-report-probe.py; FAIL assertions in combined 002/004 probe selected and executed 6 cases and 12 public CLI run/report invocations. Each invalid single-subject case ran exit 0/completed, reconstructed completed, planned=attempted=invalid=2, valid=0, repeated null headlines consistent. Happy and graded-timeout controls had valid=2/invalid=0; timeouts retained 2 valid Agent failures.
- **Green-after**: cargo test --locked -p opi-eval --test report_contract; expected A single-subject run with only invalid trials exits 1/incomplete and reconstructs incomplete offline; invalid/unknown repeated outcomes are not consistent. Happy and benchmark-valid timeout/zero controls remain completed without an artificial comparison edge. Observed: not-run.
- **Test impact**: add invalid single-subject and offline-parity cases; update incomplete repeated-outcome assertions; retain valid-zero/timeout/single-subject controls; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992/p19-runtime-probes/runtime-report-probe.py.
2. Apply only the root change above -> verify: cargo test --locked -p opi-eval --test report_contract.
3. Verify the complete changed surface -> verify: fresh CLI invalid/single/zero/timeout cases; assembled_run and report_contract; affected clippy; applicable workspace union below. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B3: report.native-usage-supported-by-measurement

**Closure predicate**: An Opi manifest with both token measurements Unknown(not_reported) reconstructs honest unknown usage; any measured marker cites sealed bytes containing the measurement. Partial known/unknown reasons and pi unknown facts remain visible.
**Dependencies**: B2 for final report parity
**Verification union**: all-unknown and partial-known source-bearing usage cases; report_contract; affected clippy; applicable workspace union below

#### Fix B3.1: P19-AUD-004

- **Finding source(s)**: `phase19-codex-gpt61-ae6dff6-20261014t044949z` + `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` + `P19-AUD-004`.
- **Decision**: `fix:derive-native-usage-from-measured-evidence`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/src/report.rs`, `crates/opi-eval/tests/report_contract.rs`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Derive usage availability from actual known native token measurements and their usage-bearing sealed source, not records-entry presence. When both measurements are unknown, retain their reasons and do not claim measured. Mixed known/unknown coverage stays explicit. Cite bytes that contain the measured facts; Provider/Diagnostic-only records are not usage provenance. Update the existing asymmetric native-fact contract and add the all-unknown case without adding report fields or inventing pi telemetry. Read the complete Unreleased section before its future edit; record the user-visible correction under the existing Fixed subsection and preserve released sections.
- **Closure predicate**: An Opi manifest with both token measurements Unknown(not_reported) reconstructs honest unknown usage; any measured marker cites sealed bytes containing the measurement. Partial known/unknown reasons and pi unknown facts remain visible.
- **Red-before**: python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992/p19-runtime-probes/runtime-report-probe.py; FAIL all-unknown assertion; 6 executed cases/12 CLI invocations, with each original CLI exit retained. Both unknown Opi measurements preserved not_reported diagnostics, but native_facts.usage was measured and cited native/evidence/records digest 9a7ecd3a7b3109053d524421dbe1532672b4fcacd490209925d7d42da4413302. The Provider/Diagnostic-only records and digest were identical to the known-input control.
- **Green-after**: cargo test --locked -p opi-eval --test report_contract; expected An Opi manifest with both token measurements Unknown(not_reported) reconstructs honest unknown usage; any measured marker cites sealed bytes containing the measurement. Partial known/unknown reasons and pi unknown facts remain visible. Observed: not-run.
- **Test impact**: add all-unknown provenance case; update native-fact assertion; retain pi unknown and mixed-known diagnostics; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: python3 /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992/p19-runtime-probes/runtime-report-probe.py.
2. Apply only the root change above -> verify: cargo test --locked -p opi-eval --test report_contract.
3. Verify the complete changed surface -> verify: all-unknown and partial-known source-bearing usage cases; report_contract; affected clippy; applicable workspace union below. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B4: material.current-workspace-cargo-lock-pin

**Closure predicate**: The static lock workspace_input.cargo_lock_sha256 equals the committed LF-normalized Cargo.lock digest; unchanged pinned producers still match; missing/wrong source-lock inputs continue to reject before build/publication.
**Dependencies**: none; its updated identity precedes native material refresh
**Verification union**: python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' on WSL/Linux; existing subprocess source-lock/producer drift negative path; doc/diff checks

#### Fix B4.1: P19-AUD-101

- **Finding source(s)**: `phase19-pi-glm53-7a0fe85-20261014t073512z` + `b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356` + `P19-AUD-101`.
- **Decision**: `fix:synchronize-current-workspace-lock-pin`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/external-locks/static/linux-x86_64.json`.
- **Change kind**: metadata.
- **Change**: Update only workspace_input.cargo_lock_sha256 from 261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3 to e2adf702d60f2cc69c5a815e28067657f053f2b5e0c2b97b0a6727b61db8d3f3. Cargo.lock itself requires no regeneration. All three current producer hashes match LF-normalized source, so none needs changing for this fix. A changed static-lock digest requires fresh future material/run-plan identities; preserve existing immutable material/bundle/evidence bytes. If an earlier independently approved producer change occurs, that batch owns its producer digest refresh; this batch still owns only the Cargo.lock field. A shared static file is not a shared closure predicate.
- **Closure predicate**: The static lock workspace_input.cargo_lock_sha256 equals the committed LF-normalized Cargo.lock digest; unchanged pinned producers still match; missing/wrong source-lock inputs continue to reject before build/publication.
- **Red-before**: python -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py -k test_repository_workspace_input_matches_static_pin -v; Executed 1, failed 1, no skip/ignore, exit 1, 0.028s. Current Cargo.lock e2adf702d60f2cc69c5a815e28067657f053f2b5e0c2b97b0a6727b61db8d3f3 != pin 261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3. Cold acquisition itself was not run; its fail-closed implication is current source proof at materialize-external-locks.sh:324.
- **Green-after**: python -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py -k test_repository_workspace_input_matches_static_pin -v; expected Focused pin test passes, unchanged producer/artifact checks pass, wrong lock inputs still reject before build/publication. No network, Docker or real build acquisition is necessary. Observed: not-run.
- **Test impact**: none; retain existing pin/producer and subprocess negative tests; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: python -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py -k test_repository_workspace_input_matches_static_pin -v.
2. Apply only the root change above -> verify: python -m unittest discover -s crates/opi-eval/scripts -p test_prepare_acquisition.py -k test_repository_workspace_input_matches_static_pin -v.
3. Verify the complete changed surface -> verify: python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' on WSL/Linux; existing subprocess source-lock/producer drift negative path; doc/diff checks. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B5: tb3.declared-artifact-native-witness-reward

**Closure predicate**: Exactly one declared /app/results.txt enters separate verifier in each of two native cases; unrelated canary is absent; correct/incorrect collected byte digests differ; unchanged real native grader returns oracle1/nop0; both conformance reports met=true; source material and verifier bytes unchanged.
**Dependencies**: B1, B4; fresh private diagnostic material
**Verification union**: native_tb3_artifact_witness ignored exact (1 executed, 2 real native cases); native_tb30_run_grades_declared_artifacts_across_the_separate_verifier and native_tb30_rejects_undeclared_and_substituted_collection normal exact; existing binding lib negative tests; affected clippy

#### Fix B5.1: P19-AUD-103

- **Finding source(s)**: `phase19-pi-glm53-7a0fe85-20261014t073512z` + `b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356` + `P19-AUD-103`.
- **Decision**: `fix:test-only-declared-tb3-witness-baseline`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/tests/support/native_witness_material.rs`.
- **Change kind**: test-only.
- **Change**: Add a private retained TB3 diagnostic material builder around a copied foodstuff task. Seed a known incorrect but PRESENT /app/results.txt (candidate: empty file) and an unrelated canary in Agent environment Dockerfile; oracle replaces declared file, nop preserves incorrect file. Rebind private corpus/profile/task-package/static-lock identity using existing fixture helpers; preserve native runner, task.toml collection/separate verifier, tests/native grader, solution bytes and original material unchanged. Both cases traverse existing public conformance handoff. Record fixture identity honestly. Preserve existing missing-artifact rejection tests.
- **Closure predicate**: Exactly one declared /app/results.txt enters separate verifier in each of two native cases; unrelated canary is absent; correct/incorrect collected byte digests differ; unchanged real native grader returns oracle1/nop0; both conformance reports met=true; source material and verifier bytes unchanged.
- **Red-before**: N/A: No production/runtime behavior is changed. The current test expectation and declared-artifact binding mismatch are independently shown by committed source. This specific native witness was not executed during planning; the source auditor failure is attributed evidence, not worker-observed red. Future native green must discriminate present incorrect artifact from absent artifact.
- **Green-after**: OPI_EVAL_NATIVE_WITNESS_MATERIAL=<freshly admitted TB3 foodstuff material> cargo test -p opi-eval --test native_driver native_tb3_artifact_witness -- --ignored --exact; expected 1 test executed (never 0); 2 real native diagnostic cases oracle1/nop0, both bound declared artifacts. Observed: not-run.
- **Test impact**: update native_tb3_artifact_witness/private witness material; retain missing/substituted collection negatives; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: current source contract and unchanged production boundary.
2. Apply only the root change above -> verify: OPI_EVAL_NATIVE_WITNESS_MATERIAL=<freshly admitted TB3 foodstuff material> cargo test -p opi-eval --test native_driver native_tb3_artifact_witness -- --ignored --exact.
3. Verify the complete changed surface -> verify: native_tb3_artifact_witness ignored exact (1 executed, 2 real native cases); native_tb30_run_grades_declared_artifacts_across_the_separate_verifier and native_tb30_rejects_undeclared_and_substituted_collection normal exact; existing binding lib negative tests; affected clippy. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B6: native-provider-report.pi-retry-exhaustion-reason

**Closure predicate**: All five actual-client scripted cases run to report for two Agents; pi proven exhaustion is accepted under exact intentional reason + agent-process boundary, failure trials remain attempted/invalid with no score or Agent failure, while retry recovery and native zero remain graded.
**Dependencies**: B2, B4; exact current clients/material
**Verification union**: native_provider_failure_report ignored exact (1 executed, 5 cases x 2 Agents); importer_pinned_v3_exhausted_chain_binds_provider_failure; native_provider_failure_report_fixture exact; affected clippy

#### Fix B6.1: P19-AUD-104

- **Finding source(s)**: `phase19-pi-glm53-7a0fe85-20261014t073512z` + `b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356` + `P19-AUD-104`.
- **Decision**: `fix:test-only-exact-pi-exhaustion-assertion`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/tests/native_driver.rs`.
- **Change kind**: test-only.
- **Change**: Extend the test allowlist only for product=pi, case=rate-limit, reason=agent-provider-failure:connection and require agent-process boundary. Preserve attempted=invalid=2, valid=Agent-failure=0, no native score and report denominator checks for all invalid provider cases. Keep recovery/native-zero and actual provider status/cleanup evidence. No wildcard reason acceptance or production rename.
- **Closure predicate**: All five actual-client scripted cases run to report for two Agents; pi proven exhaustion is accepted under exact intentional reason + agent-process boundary, failure trials remain attempted/invalid with no score or Agent failure, while retry recovery and native zero remain graded.
- **Red-before**: N/A: Test-only assertion correction. The exact current production reason is an intentional, existing unit-tested typed failure outside the current test allowlist. No production edits or behavioral red are required. This specific ignored native provider-report filter was not executed during planning.
- **Green-after**: OPI_EVAL_NATIVE_CLIENT_MATERIAL=<exact admitted clients> OPI_EVAL_NATIVE_WITNESS_MATERIAL=<fresh admitted TB2.1+TB3 material> cargo test -p opi-eval --test native_driver native_provider_failure_report -- --ignored --exact; expected 1 test executed, 5 cases × 2 Agents = 10 native actual-client scripted trials; reasons/classes/denominators/credential cleanup/provider status evidence all pass. Observed: not-run.
- **Test impact**: update native_provider_failure_report only; retain importer and failure fixture tests; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: current source contract and unchanged production boundary.
2. Apply only the root change above -> verify: OPI_EVAL_NATIVE_CLIENT_MATERIAL=<exact admitted clients> OPI_EVAL_NATIVE_WITNESS_MATERIAL=<fresh admitted TB2.1+TB3 material> cargo test -p opi-eval --test native_driver native_provider_failure_report -- --ignored --exact.
3. Verify the complete changed surface -> verify: native_provider_failure_report ignored exact (1 executed, 5 cases x 2 Agents); importer_pinned_v3_exhausted_chain_binds_provider_failure; native_provider_failure_report_fixture exact; affected clippy. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B7: walkthrough.independent-four-trial-offline-verifier

**Closure predicate**: The independent evaluation directory explicitly installs the built executable and verifier, every command uses those declared paths, and the final offline handoff invokes acceptance /3 with the independently selected --expected-identities file outside the evidence root. The minimum remains four TB2.1/TB3 Opi/pi trials and DeepSWE stays separately scoped out.
**Dependencies**: B9 defines the final /3 verifier invocation; implement B9 before this bounded documentation batch
**Verification union**: python scripts/opi-doc-check.py; git diff --check; inspect installed command paths and registered four-trial scope

#### Fix B7.1: P19-AUD-005

- **Finding source(s)**: `phase19-codex-gpt61-ae6dff6-20261014t044949z` + `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` + `P19-AUD-005`.
- **Decision**: `fix:synchronize-independent-walkthrough`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/examples/local-evaluation/README.md`.
- **Change kind**: documentation.
- **Change**: Replace six attributable trials with at least four TB2.1/TB3 Opi/pi trials and truthfully separated DeepSWE evidence. Before leaving the source checkout, copy the offline verifier to evaluation/bin and install the built opi-eval executable there using its resolved Cargo artifact path; add that bin directory to PATH for Steps 2-6 and use its explicit installed paths in Step 7. Resolve the actual Cargo target/artifact path rather than assume ./target/release because this repository uses external Cargo caches. No material acquisition or real model run is needed for this docs repair. After B9, document the /3 verifier invocation with --expected-identities and a separately reviewed file path outside the evidence root. The walkthrough must not invent expected identities, old original command records or a retrospective freeze.
- **Closure predicate**: The independent evaluation directory explicitly installs the built executable and verifier, every command uses those declared paths, and the final offline handoff invokes acceptance /3 with the independently selected --expected-identities file outside the evidence root. The minimum remains four TB2.1/TB3 Opi/pi trials and DeepSWE stays separately scoped out.
- **Red-before**: N/A: Documentation-only change. Current source lines 211 and 217 directly prove the six-trial claim and undeclared scripts/ and target/release/ paths; no prose-mirroring test is needed.
- **Green-after**: python scripts/opi-doc-check.py; expected Checks pass; installed artifact/script paths match every independent-directory command; at least four TB2.1/TB3 Opi/pi trials and excluded DeepSWE treatment agree with the registered source. Observed: not-run.
- **Test impact**: none; retain all runtime checks; no test that mirrors prose; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: current source contract and unchanged production boundary.
2. Apply only the root change above -> verify: python scripts/opi-doc-check.py.
3. Verify the complete changed surface -> verify: python scripts/opi-doc-check.py; git diff --check; inspect installed command paths and registered four-trial scope. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B8: native-stage-deadline.admitted-capture-graded-timeout

**Closure predicate**: For an AgentTimeoutError with bound native reward and admitted bounded evidence, stop only the selected Agent and its owned descendants, let the first-sink helper drain complete safe records and finalize a validated sanitized capture/1 receipt, and preserve the timeout native reward/valid graded denominator. Overflow, trailing partial record, opaque/privacy refusal, missing/unreadable receipt, or uncertain termination remains invalid and visible.
**Dependencies**: B4; derive native diagnostic material from the corrected current Harbor module before green
**Verification union**: python -m unittest discover -s crates/opi-eval/scripts -p test_native_integrations.py; bounded positive/timecap native probe exact (1 executed,4 diagnostic trials); remediation_native_stage_deadlines exact with both admitted families; native_capture_artifact_limit and credential/privacy/owned-cleanup regressions; applicable workspace union

#### Fix B8.1: P19-AUD-105

- **Finding source(s)**: `phase19-pi-glm53-7a0fe85-20261014t073512z` + `b37a36c99d81ea6dce462f067147abfeef0eb9d1519436bcf2052c5ffac75356` + `P19-AUD-105`.
- **Decision**: `fix:terminate-owned-agent-group-and-preserve-capture-finalizer`.
- **Verification status**: Partially confirmed.
- **File(s)**: `crates/opi-eval/integrations/harbor/opi_eval_harbor.py`, `crates/opi-eval/tests/support/native_witness_material.rs`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/scripts/test_native_integrations.py`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Keep helper as the first sink and receipt producer. Under it, use an integration-private Bash wait wrapper to launch the exact selected Agent in a separate job process group and write that actual Agent-group PID to the existing private stage agent.pid. On deadline, kill only that Agent group, suppress only wrapper job-notification stderr, await wrapper/helper EOF/finalization under the existing finite settlement bound, then let existing terminal/3 and receipt validation decide admission. Failed finalization remains unreadable/invalid. Do not change limits, capture policy, receipt status meanings or reader validation. Native witness helper currently preserves original cached integration module. For green checks derive a private material/module snapshot from current changed source and recompute the existing module_manifest_sha256 field; keep original source material/cache bytes immutable. This is test material derivation, no material/7 schema change. Copy the bounded positive/timecap red probe into the existing native_driver surface and retain full stage-deadline, capture overflow/privacy and owned-resource cleanup assertions. Read the complete Unreleased section before documenting the user-visible timeout correction. The private PID must be authenticated against the owned launch/parent and process start identity; do not trust an Agent-editable PID file alone or allow PID reuse to target an unrelated group. Stop only owned descendants, including any separately created child groups, and prove an unrelated task baseline process survives. If complete owned termination cannot be established within the existing bound, retain an explicit invalid/uncertain outcome rather than grading. These are private supervision guards, not a new public field or broadened cleanup authority.
- **Closure predicate**: For an AgentTimeoutError with bound native reward and admitted bounded evidence, stop only the selected Agent and its owned descendants, let the first-sink helper drain complete safe records and finalize a validated sanitized capture/1 receipt, and preserve the timeout native reward/valid graded denominator. Overflow, trailing partial record, opaque/privacy refusal, missing/unreadable receipt, or uncertain termination remains invalid and visible.
- **Red-before**: OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/phase19-acceptance-r2/materials-r9/tb30/material.json cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_tb3_timecap_capture_settlement -- --ignored --exact --nocapture; FAIL: cargo exit101, 1 selected/executed test failed,0 passed/ignored,139 filtered; 153.94s. Positive run exit0 valid2 invalid0; timecap run exit1 valid0 invalid2 reason native-capture-refused with missing receipts, retained stdout51B/140B, stderr0 and native reward0.0.
- **Green-after**: OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/phase19-acceptance-r2/materials-r9/tb30/material.json cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_tb3_timecap_capture_settlement -- --ignored --exact --nocapture; expected Same positive and timecap cases both valid2/invalid0 with genuine sanitized receipt/digest commitments, timecap native reward0 and Agent-failure/timed_out flags retained. No leaked record or missing receipt is admitted; no owned Agent descendant survives. Observed: not-run.
- **Test impact**: add or integrate the discriminating positive/timecap case and private group/helper finalization negatives; update current-module witness material derivation; retain overflow/privacy/missing-receipt and owned-resource cleanup checks; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/phase19-acceptance-r2/materials-r9/tb30/material.json cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_tb3_timecap_capture_settlement -- --ignored --exact --nocapture.
2. Apply only the root change above -> verify: OPI_EVAL_NATIVE_WITNESS_MATERIAL=/home/luiz/phase19-acceptance-r2/materials-r9/tb30/material.json cargo test --locked --offline -p opi-eval --test native_driver p19_plan_probe_tb3_timecap_capture_settlement -- --ignored --exact --nocapture.
3. Verify the complete changed surface -> verify: python -m unittest discover -s crates/opi-eval/scripts -p test_native_integrations.py; bounded positive/timecap native probe exact (1 executed,4 diagnostic trials); remediation_native_stage_deadlines exact with both admitted families; native_capture_artifact_limit and credential/privacy/owned-cleanup regressions; applicable workspace union. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

### Batch B9: acceptance.required-execution-provenance-and-freeze

**Closure predicate**: A required execution-provenance/freeze record is preserved, byte-bound and validated before accepted=true; missing, malformed or mismatched source/build, original command, usage/time/platform freeze evidence rejects without claiming authentic provenance solely from mutual agreement of supplied records.
**Dependencies**: B1, B2, B3, B4, B8 for final source/material/report identity; public field contract already approved
**Verification union**: python -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py; full affected Python union; p19_plan_acceptance_rejects_fixture_evidence exact (1 executed); approved synthetic structural control plus independent-identity/original-proof negative CLI cases; doc/diff and applicable workspace gates

#### Fix B9.1: P19-AUD-003

- **Finding source(s)**: `phase19-codex-gpt61-ae6dff6-20261014t044949z` + `85c2bbf73b8b452603fea1969b777219b6f84e05af161230ab8f23c5905f5ad2` + `P19-AUD-003`.
- **Decision**: `fix:enforce-approved-acceptance-provenance-v3`.
- **Verification status**: Confirmed.
- **File(s)**: `crates/opi-eval/scripts/verify-local-acceptance.py`, `crates/opi-eval/scripts/test_verify_local_acceptance.py`, `crates/opi-eval/tests/native_driver.rs`, `crates/opi-eval/README.md`, `crates/opi-eval/README.zh.md`, `crates/opi-eval/examples/local-evaluation/README.md`, `CHANGELOG.md`.
- **Change kind**: behavioral.
- **Change**: Implement the user-approved /3 contract below in the existing offline verifier. Reuse current material/control/trial and Agent identity receipts; require an explicitly selected independent expected-identity file, original argv/exits/both-stream/input-output records, inline frozen task/control/bounds/platform/credential-reference facts and reciprocal retained attempts. Match approved identities and bytes, not a claim of cryptographic build causality. Missing/malformed/tampered facts reject before Companion calls. Update the existing Rust acceptance producer and script fixtures with separately supplied synthetic expected identities; preserve fixture/native/old-evidence distinctions. Synchronize primary EN/ZH docs, independent walkthrough invocation and the existing Unreleased Breaking Changes subsection after reading its complete contents. No paid calls, source-registry/ledger or core edits.
- **Closure predicate**: A required execution-provenance/freeze record is preserved, byte-bound and validated before accepted=true; missing, malformed or mismatched source/build, original command, usage/time/platform freeze evidence rejects without claiming authentic provenance solely from mutual agreement of supplied records.
- **Red-before**: wsl.exe -d Ubuntu --exec bash -lc 'cd /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992 && TMPDIR="$PWD/.remediation-probes/script-findings/tmp" PYTHONDONTWRITEBYTECODE=1 python3 .remediation-probes/script-findings/test_acceptance_admission_red.py'; FAIL: Python 3.12.3 on WSL Ubuntu: executed 3, passed 1 control, failed 2 rejection assertions, no skipped/ignored tests. Both negative cases exited 0 accepted=true with 4 structural trials. Probe command exit 1; elapsed 41.491s. Original run redirected stdout/stderr to acceptance-admission-red.log.
- **Green-after**: python -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py; expected Approved /3 structural control passes with independently selected synthetic expectations; omitted/malformed/tampered original source/build/command/freeze/attempt facts and coordinated internal substitutions reject. Existing native/fixture provenance distinctions and typed unknown/not-requested bounds remain intact. This is contract conformance, not closure of the excluded actual operator evidence. Observed: not-run.
- **Test impact**: add missing/tampered original proof and independent-expectation negative cases; update existing accepted structural fixture and Rust acceptance producer; retain native/fixture/old-evidence distinctions and unknown/not-requested bounds; delete=none.

1. Install the discriminating case or bounded metadata/test/doc correction -> verify: wsl.exe -d Ubuntu --exec bash -lc 'cd /mnt/c/Users/Luiz/AppData/Local/Temp/opi-p19-remediate-8c3e25f8ccfa45e59af5e6de2ec1b992 && TMPDIR="$PWD/.remediation-probes/script-findings/tmp" PYTHONDONTWRITEBYTECODE=1 python3 .remediation-probes/script-findings/test_acceptance_admission_red.py'.
2. Apply only the root change above -> verify: python -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py.
3. Verify the complete changed surface -> verify: python -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py; full affected Python union; p19_plan_acceptance_rejects_fixture_evidence exact (1 executed); approved synthetic structural control plus independent-identity/original-proof negative CLI cases; doc/diff and applicable workspace gates. Preserve original logs/exits/counts and stop for a new plan if closure requires new public meaning or authority.

## Final Verification

This is the selected union for the approved-scope code repair after exact apply admission; it is not a claim product greens already passed. Execute B1/B2/B3/B4/B5/B6/B8/B9, then B7 (the numeric labels preserve planning traceability, not a conflicting execution order). Behavioral Rust, public CLI/report seams, script/platform negative paths and native test-only edits contribute their checks. Native greens use freshly verified private materials and no real provider. Actual operator acceptance and the excluded missing proof remain separate. Do not repeat unchanged successful evidence without a relevant state change.

```text
python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'  # WSL/Linux, isolated temp inputs
cargo test --locked -p opi-eval --lib
cargo test --locked -p opi-eval --test assembled_run
cargo test --locked -p opi-eval --test report_contract
cargo test --locked -p opi-eval --test native_driver  # normal selection; ignored != executed
# Exact admitted offline native filters/counts listed in B5/B6/B8 and D3, with original inventories/logs
python scripts/opi-doc-check.py
git diff --check
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The existing default suite remains deterministic and credential-free. Final workspace gates also supply D2 only when original commands/logs/source seals/executed counts and actual operator platform facts are preserved for the final relevant state. A temporary plan probe is not a committed regression until deliberately added in a ready apply. No live-provider action follows automatically from a plan digest.

## Exclusions

| Exact source key | Current disposition | Evidence / required authority |
|---|---|---|
| `phase19-codex-gpt61-ae6dff6-20261014t044949z/P19-AUD-006` | Cannot confirm; explicit code-first evidence exclusion | Partially confirmed within the admitted current proof selection. Fresh actual-client protocol matrix now qualifies all three local wire paths for Opi/pi: 1 executed test PASS, 30 client invocations, 6 wire logs, 75 requests and 12 verified real bash file effects. Original client/source/package/binary identities are byte-bound. This is loopback conformance, not optional real-service qualification or all finding closure. Native artifact witnesses, both-family real-artifact fresh regrade, grader/export canaries and pi-only native qualification remain distinct proof work. Fixture/package summaries and four real native results are useful counterevidence to absence of execution; no global absence or severity reduction is claimed. A bounded current original pi-only TB3 regrade verified the source seal, saved results and cached official image, then exited1 with verifier-network-required and 0 fresh grades. Pi-only material admission is established; successful official regrade/canary qualification is not. User explicitly chose code-first planning with these evidence gaps retained. |
| `phase19-codex-gpt61-ae6dff6-20261014t044949z/P19-AUD-007` | Cannot confirm; explicit code-first evidence exclusion | Current /1 supplied manifest is rejected by the current /2 verifier (fresh exit 2/accepted=false). Exact admitted records do not establish complete original independent preparation/failed-attempt/platform/final workspace handoff. Preserved four-outcome reports are counterevidence to absence of real trials; they are not complete current operator proof. User confirms only the supplied directory is available and chooses code-first planning with missing original freeze/build/command facts retained as unclosed evidence; no retrospective fact or new provider run is authorized. |
| `phase19-pi-glm53-7a0fe85-20261014t073512z/P19-AUD-102` | Cannot confirm; explicit code-first evidence exclusion | Fresh schema-admission CLI rejects the exact supplied /1 root at exit 2; four newer raw-outcome reports do not replace the prescribed /2 handoff. Static material identity changed and requires a new derived view, never mutation or relabelling of old seals. User confirms only the supplied directory is available and chooses code-first planning with missing original freeze/build/command facts retained as unclosed evidence; no retrospective fact or new provider run is authorized. |

There are no Refuted, Info/No action or registered deferral conclusions. The no-action prefix records only explicit Cannot confirm exclusions. Cannot confirm is not Refuted. Current run/digest controls admission; similar titles cannot substitute for identity; older source keys never enter the union. These three metadata exclusions authorize no source edit, remain Major and block Phase acceptance.

## Planning handoff and materialization

Planning changes only the fixed Markdown/JSONL pair. Test impact for this task: none in the repository; temporary fixture/native probes supplied red-before and counterevidence. Planned test impact is add/update/retain as recorded per batch; delete=none. No English/Chinese product counterpart was changed. The sole English example has no Chinese counterpart, and primary EN/ZH docs already agree on four trials. This fixed English assurance artifact has no translated companion contract.

Validate exact disposition coverage, then the fixed ready plan; retain the validator-emitted digest outside the plan. The user approved contract/scope decisions, not an exact apply invocation. Production apply therefore still requires the new explicit mode=apply phase=19 plan_sha256 invocation for the final emitted digest. Any plan/index bytes change that identity. Only plan-pair commits may follow remediation_head without resealing unchanged source; no commit is requested. Apply must keep006/007/102 at Cannot confirm with no fabricated originals, unauthorized provider replacement, or Phase-conformity claim. No other workflow or ledger is invoked.

## User-approved acceptance /3 contract

User-approved choice: `opi-eval-local-acceptance/3` with inline provenance facts and one explicit `--expected-identities PATH` input to the existing offline verifier. Keep current material/control/trial semantics, existing Agent build receipts, native evidence checks, Companion verify/report entry, and artifact ceilings. The user chose `/3`; the breaking in-place `/2` alternative is not admitted. `/3` makes changed admission visible; the source does not mandate a number bump. A narrow `/2` repair rejecting malformed existing source/operator fields is routine validation, but cannot close the missing build/command/freeze evidence portion of 003.

The independent input is a routine part of the proposed contract, not another authority choice. The human selects its exact bytes outside the supplied evidence root and approves its digest. Outside-root location alone is not approval. The verifier records the selected digest for comparison with that approval; it does not accept a fallback file or identity from the producer root. One new argument is necessary because current root/out/companion inputs provide no independent static-lock/source/Agent identity. No other flag, Companion command, signing infrastructure, reproducible-build service or provenance SDK is proposed.

## Exact changed manifest fields

The `/3` top-level record retains `/2` `operator`, `opi_repository`, `controls`, `trials`, `sources`, `builds`, `material`, and `preserved`. Add `expected_identities_sha256`, inline `freeze`, inline `commands`, and inline `attempts`; replace the weak `command_exits` tuples with `commands`. Add `attempt_id` to each accepted trial. DeepSWE remains outside primary trial coverage. Each accepted trial still passes all current native/seal/tool/control/material rules.

An artifact reference is `{path, sha256}`. A command stream reference additionally requires `redaction: {kind: "none"}` or `{kind: "redacted", policy: nonempty string, reason: nonempty string}`. It binds the exported bytes and truthfully labels derivatives; it does not claim reconstruction of unavailable secret-bearing originals. Empty stdout/stderr files are preserved and hashed. Original host paths in receipts/argv/cwd are informational and never followed. New inline objects reject unknown fields and duplicate keys; digests are lowercase SHA-256, commits full lowercase 40-hex, timestamps UTC RFC3339, IDs single contained components, and integers exclude booleans. Contained references occur once in the existing preserved inventory; reject traversal, links, special files and over-ceiling artifacts.

`builds[]` keeps `agent` and `executable_sha256`; add required `receipt: ArtifactRef`, `executable: ArtifactRef`, `runtime_package: {path, manifest_sha256}|null`, and `build_command_ids: nonempty Id[]`. There is one Opi and one pi row. Reuse `opi-eval-agent-identity/1` unchanged. Verify its source/lock/toolchain/target/profile/features/dirty-state fields as actually recorded and independently approved; do not impose new clean/release/default-feature policy. A dirty-state receipt cannot be described as committed-source-only. Package digest reuses current `runtime_package_manifest_digest` exactly: sorted relative-file-path/LF/file-digest/LF rows, existing no-link/no-special-file rules; no new mode/directory commitment. Pi wrapper, Node, bundle and package identities remain distinct.

`freeze` is exactly `{scope_id, frozen_at_utc, operator_label, platform, credential_references, families, budgets}`:

For inline freeze identity, hash UTF-8 `json.dumps(freeze, sort_keys=True, separators=(",", ":"), ensure_ascii=False)`: array order is retained, strings are not Unicode-normalized, and the new budget quantities use integers or decimal strings rather than floating point. The independent expected input and accepted run records must carry that same digest; assembling a final manifest cannot retroactively create its original pre-run freeze.

- `platform = {host, execution, distribution, engine_id, evidence}`; for the registered acceptance platform, `host="windows-x86_64"` and `execution="wsl2-linux"`. Other values are not silently qualified as this Phase platform. `evidence` references original redacted doctor/preflight observations, not a fresh claim about the past host.
- `credential_references[] = {connection_id, kind, label}`; kind is `environment` or `private-file`, label is redacted, and no credential is read or exported. Existing actual selected account/model/billing-access evidence remains required where it affects real execution; an endpoint alone does not prove access.
- `families[] = {family, run_root, config, plan, material_sha256, trial_ids, bounds}`; exactly TB2.1 and TB3, matching existing material/control/trial identity. `config` and non-null `plan` are artifact references; `plan=null` permits existing direct `run --config` when the exact saved config, material, selected IDs and existing run-control freeze determine the same trials. `bounds` is the **existing** run-config/plan bounds object: concurrency, max_trials, Agent/verifier/process time limits, response output cap and byte cap retain their actual declared/effective meanings and conditional nulls. Do not add experiment configuration knobs or treat a response cap as cumulative usage.
- `budgets = {time, usage, spend}`; each is `{requested, effective, state, evidence, revision}`. `requested` is `null` or `{value, unit, hard}`; `effective` is `null` or `{value, unit}`. Time uses positive seconds, usage positive tokens, and spend a positive decimal string with an uppercase currency unit. `state` is `not-requested`, `enforced`, `unsupported`, or `revised`. `evidence` is a reference or `null`; an enforced claim requires original evidence of the **actual** mechanism/scope, not just its name. `not-requested` allows both requested/effective to be null and needs no revision. An actually requested **hard** bound that is unsupported must either reject or carry `revision={approved_by, approved_at_utc, reason, revised_requested}` made before execution; this records the already allowed explicit scope revision. Soft/monitoring limitations and unknown observations stay visible. Unknown price/usage cannot satisfy a guaranteed currency cap. No new budget enforcer or mandatory currency cap is proposed.

`commands[]` is exactly `{id, phase, family, attempt_id, argv, cwd_label, executable_sha256, started_at_utc, ended_at_utc, exit_code, stdout, stderr, freeze_sha256, inputs, outputs}`. Phase is doctor/build/prepare/validate/plan/run/verify/report. Family is TB2.1/TB3, scoped-out DeepSWE, or null for shared builds. Attempt ID is required for run, otherwise null. Preserve exact launched argv order, original integer exit, timestamps, both stream references and `{role, artifact}` input/output identities. Current original verify/report require zero; original run keeps the current valid nonzero-comparability rule. An accepted run's freeze digest matches the independently approved freeze; doctor/build/prepare before final freeze may use null. For build, pin the producer-script input to the admitted lock and link the original receipt/binary/package outputs. An original builder invocation capture is sufficient; do not pretend a printed command summary is original argv/stdout/stderr/exit capture. The fresh offline verifier's own subprocess successes never replace original records.

Required original command coverage: each accepted family has successful `prepare`, then `plan` for the saved-plan path or `validate` for the direct-config path, plus `run`, `verify` and `report` linked to the corresponding family/config/material/attempt. Every `builds.build_command_ids` entry resolves to an original `build` record binding that build receipt and outputs; shared build records may serve both families. Missing any required phase rejects, even if retained build+run records and fresh verifier subprocesses succeed. Add a separate negative case for each omitted required phase. Original `prepare`/`plan`/`validate`/`verify`/`report` exits are zero; accepted original `run` retains the existing valid nonzero rule and failed predecessor commands retain their original exits.

`attempts` is exactly `{scope_id, declared_by, declared_at_utc, scope_description, complete_within_scope, records}`. The boolean is an operator's **bounded completeness attestation**, not proof that no private invocation was omitted. Each record is `{id, family, run_id, trial_ids, disposition, replacement_of, command_ids, artifacts, missing_evidence_reason}`. Disposition is accepted/failed/cancelled/effect-unknown/unstarted/scoped-out; nullable predecessor and unique trial/run identities preserve replacement chains. Empty run/command/trial identities are permitted only for unstarted work. All accepted trials map reciprocally to an attempt and original run record. Retain declared failed/unknown/cancelled predecessors and separately labelled DeepSWE; do not count them as valid scores. Missing essential original records reject/incomplete the handoff, even when a reason truthfully preserves the gap.

## Independent expected file

The one external record is `opi-eval-acceptance-expected-identities/1` with exactly `{schema, scope_id, static_lock_sha256, freeze_sha256, companion, agents}`. `companion={execution_executable_sha256, verification_executable_sha256}` binds the original operator CLI and current `--companion`; different identities require explicit source-applicability qualification. Each agent row is `{repository_url, source_commit, receipt_sha256, executable_sha256, runtime_package_manifest_sha256}`; Opi's package digest may be null only when its actual material has no package. Pi additionally requires `node_sha256` and `bundle_sha256`. The approved raw receipt digest fixes all existing receipt fields without adding build policy. Match `source_commit` to the existing receipt `checkout_commit` and the manifest source commit. Match `repository_url` to the manifest `opi_repository`/`sources` and independent expected input; pi also matches the independently admitted static lock. The existing Opi/pi `opi-eval-agent-identity/1` receipts contain no repository URL: do not add a receipt field or reject them for that absence. An Opi previous build may be admitted only through D2/D3's actual applicability review, never by declaring its old Cargo.lock to be the new one.

Expected identities come from the actual operator-selected source/build/material/plan and independently reviewed bytes, not from copying untrusted root assertions into a second file. `--expected-identities` records the file digest and checks equality; it does not certify how that file was approved. The external expected lock anchors supplied material locks before qualified pi/Harbor identities are derived. The approved source/build receipt, binary/package and original builder records prove **approved identity and byte binding**. They are not a cryptographic proof that an honest compiler causally built those bytes from that source. This limited human trust model is the recommended default; no PKI is required.

## Validation order

| Order | Predicate | Rejection example |
|---|---|---|
| 1 | Strict schemas/types/duplicate keys, contained IDs/paths/output separation. | Malformed source URL/commit, absolute artifact path, duplicate build. |
| 2 | Load selected independent expected file once; record digest; compare manifest expected digest and hash current Companion before spawn. | Expected input missing/root fallback, changed CLI bytes. |
| 3 | Hash required receipts/executables/packages, stream captures and inventory; admit all references. | Missing original stderr including empty capture, replaced receipt/package, linked artifact. |
| 4 | Match supplied lock to expected digest before deriving upstream identity; keep current material/integrity checks. | Coordinated supplied lock/source/cache replacement. |
| 5 | Match independently approved receipt source and actual Agent binary/package/Node/bundle to material/trial attribution and builder captures. | Source commit differs despite all supplied maps agreeing. |
| 6 | Match freeze to expected digest, saved config/plan/run-control bounds and selected task/control/platform facts; enforce only actual requested hard-bound/revision semantics. | Late freeze, silent task change, unsupported requested hard cap without prior revision. |
| 7 | Original command phase coverage, recorded argv/exits/logs/input-output/freeze/CLI identities; preserve failures. | New verifier success substituted for missing original run/verify/report capture. |
| 8 | Reciprocal accepted trial/attempt coverage, retained predecessor chains and bounded attestation. | Replacement reuses trial ID or hides declared failed attempt. |
| 9 | Current native/sealed/control/tool-loop/scoped-view/coverage checks. | Fixture-only, oracle, invalid handoff, omitted frozen pair. |
| 10 | Current hash-matched Companion verify/report only; accepted after all predicates. | Companion consumer rejection; never a Phase-conformity claim. |

`opi-eval-local-acceptance-verification/3` retains current output fields and adds the selected expected-file digest, approved freeze digest and explicit limit: operator-approved identity/byte consistency does not cryptographically establish build causality or completeness beyond the declared invocation scope. No secrets, original host paths, Agent/build/grader execution, network or material acquisition follow from verification.

Before: current `/2` fixture with source/operator deleted is accepted at exit 0 while family bytes remain unchanged. After: it rejects before Companion invocation. A complete structural fixture matches a **separately supplied synthetic expectation**; resealing all internal records with a substituted source commit still rejects when the independent expected identity remains unchanged. A complete native zero remains eligible; structural success is never real-provider/native proof.

`acceptance-manifest-example.json` is an approximately 100-line exact-field illustration and `expected-identities-example.json` shows its independently selected expectation shape. Their hashes and source URLs are deliberately synthetic; referenced artifacts do not exist and these examples are not acceptable evidence or build approval. `proposal-minimal.json` inventories the proposal artifacts and states this limitation.

Preserve original old manifests/materials/bundles/failures byte-for-byte. A new derived view can bind copied original bytes, never fabricate original logs/exits or retroactive freezes. D2/D3 retain ownership of actual old-build qualification and missing/replacement evidence. Candidate changes after approval: verifier/Python tests, existing Rust acceptance producer, help/invocation, primary EN/ZH READMEs, independent walkthrough and Unreleased; a producer script outside the listed batch paths requires a new plan before editing, even if original capture would benefit from it; no such producer edit is admitted here. No specification/ledger/core edit is authorized. Deterministic tests cover missing/tampered fields, independent expected mismatch, paths/duplicates, honest unknown/not-requested bounds, unsupported hard-bound revision, retained attempts and intended assembled fixture rejection. Required workspace/native/operator evidence remains separate.


Executable/runtime-package identities reuse existing material/package verification and bounded streamed hashing. The native capture stream ceiling is not a new blanket executable/package-size ceiling; retain each existing verification family's bounds without widening capture. Source arrays expose only credential references, never credential values or private reads.

The /3 format and independent expected-identity input were explicitly selected by the user. Actual external expected-file bytes and old execution facts are not fabricated or approved by this code-design decision. The broader temporary proposal is superseded and is not part of this plan.
