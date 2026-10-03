# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt61-2465aeb-20261001t055252z`
**Audit head**: `2465aeb80ad84d06172acc142f6d5ffbceb201cc`
**Reviewer ID**: `codex`
**Model ID**: `gpt61`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt61`
**Model identity source**: operator-declared
**Independence**: unknown — fresh audit context and fresh evidence-only helpers; implementation-author exact model provenance unavailable. Same-family helpers do not imply independent-family review.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

The sealed 56-obligation inventory yields met=42, not-assessable=4, not-met=3, partially-met=7. Findings: Major=6, Minor=5. R55 is optional. Genuine six-trial native evidence is credited; missing controlled proofs and current defects still block Phase exit.

This audit makes no production/test/spec/ledger changes and no branch, commit, provider call, credential read or native task execution. Test impact: **retain**. Exactly one report language is emitted because the audit-set contract owns exactly four suffixed files; no separate localized report counterpart is required.

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | Latest committed bytes; ledger context projected through read_audit_scope.py. |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | Latest committed bytes; ledger context projected through read_audit_scope.py. |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | Latest committed bytes; ledger context projected through read_audit_scope.py. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | Latest committed bytes; ledger context projected through read_audit_scope.py. |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | Latest committed bytes; ledger context projected through read_audit_scope.py. |

No stored-hash mismatches. Requirements were sealed before production inspection; immutable identity projection SHA-256 `e6028cab251281b063a7b54cb131cb78063dfc50024d282926dd0f18aedbff58`. Source was exported with git archive, never a worktree. Prior audit/remediation conclusions, raw ledgers, history records and sibling member output were excluded throughout independent-core work.

## Requirement Conformance

The sidecar owns the sealed criterion identities and detailed production/test/check evidence; this table adds the current interpretation. Passing fixture checks are not substituted for ignored actual-native/client gates.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-R01 | P19-AC-001; Section 4 | Doctor resolves config and reports Linux/engine/resources/missing selected material without credential resolution or Agent/model launch; unknown resources retain reasons. | met |  |
| P19-R02 | P19-AC-001; Section 4 | Selected pinned source/cache bytes, runner uv.lock, Agent/runtime, module and OCI identities are reverified. Local origin is independent of GitHub receipts; supervised external acquisition has an explicit deadline. Fixtures demonstrate the public preparation/refusal path, not a new live download. | met |  |
| P19-R03 | P19-AC-001; Section 4 | Public local workflow works without repository fixtures in the native branch. Independent-directory tests and genuine external operator material/runs complement each other; the present audit performs offline replay only. | met |  |
| P19-R04 | P19-AC-002; Section 5 | Explicit selected import path, task package, fresh one-job/one-trial result, exact Agent/model and runtime proof bind logical trial/build. Six raw real trials have selected actual Agent identity and matching build hashes. | met |  |
| P19-R05 | P19-AC-002; Section 6 | Declared collection manifests, observed delivered digests, lifecycle/base/image, verifier-input and raw native result are admitted from one bound trial before sealing. Direct raw artifact hashing matched both TB3 collected directories and both DeepSWE patches; all six stream and bundle-manifest hashes matched. Limitation: Collected-file recursive reader residual described separately; pinned runner source needed to decide whether unsupported filesystem shapes reach it. | met |  |
| P19-R06 | P19-AC-003; Section 5 | Current integration awaits private-stage credential cleanup before publishing the terminal or returning for grading. TB2.1 source task default shared mode is honored. Raw Opi/pi TB2.1 results both declare shared mode and passed preflight; their distinct native outcomes are preserved. | met |  |
| P19-R07 | P19-AC-003; Section 5 | Discriminating current opt-in witness implementation exists at production conformance/BenchmarkExecution seam. Its actual executed artifacts are not available in the permitted current acceptance inventory. Limitation: Parent forbids real native task execution in audit. Default suites exclude this ignored filter. Six real provider trials do not substitute for a controlled correct/incorrect witness. | not-assessable | P19-AUD-006 |
| P19-R08 | P19-AC-003; Section 8 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R09 | P19-AC-004; Section 5 | Task-derived collection and mode are frozen, unsupported lifecycle declarations reject, returned native mode is cross-checked, declared inputs must be present and digest-bound, and undeclared manifest sources reject. Raw TB3 inputs contain one /app/evalbench/ separate-verifier collection for each Agent. | met |  |
| P19-R10 | P19-AC-004; Section 5 | Discriminating current opt-in witness implementation exists at production conformance/BenchmarkExecution seam. Its actual executed artifacts are not available in the permitted current acceptance inventory. Limitation: Parent forbids real native task execution in audit. Default suites exclude this ignored filter. Six real provider trials do not substitute for a controlled correct/incorrect witness. | not-assessable | P19-AUD-006 |
| P19-R11 | P19-AC-004; Section 8 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R12 | P19-AC-005; Section 5 | Pinned task collect hook diffs declared base against HEAD, excluding uncommitted state. Pier owns pristine separate verification and no-network task contract; native import cross-checks declared base/image/mode/patch. Wrong base and undeclared collection reject. Raw task.toml line 31 confirms the base-to-HEAD collect command and both preserved patch hashes match sealed inputs. | met |  |
| P19-R13 | P19-AC-005; Section 5 | Discriminating current opt-in witness implementation exists at production conformance/BenchmarkExecution seam. Its actual executed artifacts are not available in the permitted current acceptance inventory. Limitation: Parent forbids real native task execution in audit. Default suites exclude this ignored filter. Six real provider trials do not substitute for a controlled correct/incorrect witness. | not-assessable | P19-AUD-006 |
| P19-R14 | P19-AC-005; Section 8 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R15 | P19-AC-006; Section 4 | Selected exact provider/api/base_url/model and credential env/file reference resolve into production configs. Opi api_key_env and Pi inline apiKey remain private launch projections; adapters receive exact provider:model and config bytes verbatim. No global config mutation/fallback found. Limitation: Complete actual-client matrix execution withheld under R17; static Opi native API/config source supports exact protocol mappings. | met |  |
| P19-R16 | P19-AC-006; Section 4 | Required unsupported reasoning/sampling reject before effects, exact output proof and final unsupported markers protect native comparison. Public project_controls still labels optional requested reasoning Effective while pinned Pi launch forces thinking off and settlement records it unsupported. | partially-met | P19-AUD-011 |
| P19-R17 | P19-AC-006; Section 4 | Current-source-applicable raw test log records native_protocol_client_matrix:1 passed,0 failed,0 ignored. Its current body requires supplied actual Opi/Node/Pi, exercises both clients across all three local protocols with success/auth401/rate429/retry/protocol-error/tool continuation/output controls; missing credentials/proof mismatch covered by public owning tests. Raw request directory is now absent; scheme-level auth assertions are not exact-secret-value assertions. No real-service OpenAI qualification claimed. | met |  |
| P19-R18 | P19-AC-006; Section 8 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R19 | P19-AC-007; Section 5 | Enumerates complete pinned inventories and resource/unknown metadata without models or all task images; selection materialization is separate from full inventory identity. | met |  |
| P19-R20 | P19-AC-007; Section 5 | Task-derived CPU/memory/storage/GPU/type/network/lifecycle/collection/verifier inputs retain origin and timeout constraints. Unsupported GPU/storage/network/lifecycle shapes fail closed; shared Pier networking contradictions reject. | met |  |
| P19-R21 | P19-AC-007; Section 5 | Actual selected-engine resource admission rejects unmet or unknown required resources and immutable image mismatch; selected full/subset is not silently reduced or widened. | met |  |
| P19-R22 | P19-AC-007; Section 4 | Canonical material/corpus/task/trial/repetition/selection/control/bound identities are frozen and rechecked before dispatch. R35 separately assesses enforcement of a serialized output bound. | met |  |
| P19-R23 | P19-AC-007; Section 5 | Selected task packages, exact runner sources, Agent runtime, integrations and immutable images must match admitted bytes at load/run; substitution negatives reach the public path. | met |  |
| P19-R24 | P19-AC-008; Section 6 | One runner-owned classifier preserves valid native zero/task failure/benchmark timeout metrics while excluding provider/auth/access/identity/adapter/infra/invalid grading. Exit or failure token alone cannot confer score eligibility; report consumes sealed interpretation. | met |  |
| P19-R25 | P19-AC-008; Section 6 | Full/subset corpus and selected totals, planned/attempted/valid/Agent failure/invalid/unexecuted counts survive missing bundles. Native task metrics and test-case fractions retain separate meanings; no synthetic cross-family headline. | met |  |
| P19-R26 | P19-AC-008; Section 6 | Pairing requires same task/repetition/effective controls/protocol and exposes missing pairs and unsupported required controls; tests discriminate protocol mismatch instead of accepting a nominal subject label. | met |  |
| P19-R27 | P19-AC-008; Section 6 | Single-subject reports retain per-task/repetition coverage without inventing a comparison edge or requiring Opi runtime for pi-only value. | met |  |
| P19-R28 | P19-AC-008; Section 6 | Unknown telemetry remains state plus reason; estimates, reported usage and cost are distinct and unknown prices do not become a billed amount. | met |  |
| P19-R29 | P19-AC-009; Section 6 | Verify checks immutable bundle bytes and rejects tampering offline without source mutation, Agent execution or credential resolution; mutable derived reports do not rewrite the source seal. | met |  |
| P19-R30 | P19-AC-009; Section 6 | Report recomputation is a deterministic offline derivative of verified source; create_new output outside run root preserves original evidence and unsealed diagnostics cannot invent scores. | met |  |
| P19-R31 | P19-AC-009; Section 6 | Fresh native verifier-only implementation and discriminating fixture behavior exist, but no current valid executed pinned native regrade proof was located. The available raw earlier regrade output is rejected (bundle-io/task-package-drift, trials=[]); it cannot prove successful actual grading. No native task or Agent/provider was launched in this audit. | not-assessable | P19-AUD-006 |
| P19-R32 | P19-AC-009; Section 6 | All sources verify before effects; missing cache/material, preserved artifact drift, unsupported offline services or base drift reject. Fresh verifier request binds source+artifact+runner+verifier; no Agent entrypoint/provider config is consumed. | met |  |
| P19-R33 | P19-AC-010; Section 4 | Selected credentials are isolated from sealed identity/grader and cleaned before grading; original host handoff/config files have no private permissions admission under supported Unix ordinary umask/public-root conditions. | partially-met | P19-AUD-002 |
| P19-R34 | P19-AC-010; Sections 4 and 9 | Canary redaction, scoped task mounts and credential cleanup are covered, but metadata-only raw inspection shows thinking-bearing terminal/stdout captures sealed as Exportable; stage() has no sensitivity choice. | not-met | P19-AUD-003 |
| P19-R35 | P19-AC-010; Sections 4 and 9 | Positive concurrency/time/trial controls and unenforceable currency/usage refusal exist. max_output_bytes is accepted and frozen but has no runtime reader/enforcement/revision/refusal; fixed unrelated caps do not honor a smaller request. | not-met | P19-AUD-001 |
| P19-R36 | P19-AC-010; Section 9 | Run token stops subsequent dispatch; container removal requires daemon-observed full IDs and exact private Compose roots, then rechecks absence. Temporary credentials and bounded partial captures have separate cleanup admission; uncertainty blocks sealing. Signal/process fixture checks distinguish actual prior live effects and no orphan. | met |  |
| P19-R37 | P19-AC-010; Sections 4 and 6 | Recovery short-circuits before run execution, uses planned inventory and durable intent/settlement/seal only, verifies sealed bundles, attaches original settlement interpretation, preserves all files, and never calls Agent/provider or manufactures scores. Durable status and scoring interpretation remain separate. | met |  |
| P19-R38 | P19-AC-010; Section 6 | Replacement is explicit, resolves target group, creates fresh experiment/manifest and fresh ID for every reexecuted trial; target pair group takes new identity, unrelated frozen groups retained. Exclusive create_new run-control rejects reused/concurrent roots; old attempts are read-only and cannot silently rerun. | met |  |
| P19-R39 | P19-AC-010; Section 9 | Current-source-applicable raw test log records native_owned_resource_cancellation:1 passed,0 failed,0 ignored. Exact current body observes a live owned process and real Docker container before signal, then asserts exact removal/neighbor-image-cache survival. Retained temporary Docker/debug directory is now absent, limiting independent ID-level replay; recorded executed assertions and complete unchanged-source identity establish this no-model witness. | met |  |
| P19-R40 | P19-AC-011; Section 8 | Default resolver hermetic, remote requires explicit live mode, selected credential presence never switches route. Default local fixtures use dummy credential, no real provider. Native/material/real client matrix tests ignored and require explicit user-selected material; script docker check explicit --docker. Limitation: Parent owns current centralized test outcomes. No credential/provider access or real/native model task by subtask. | met |  |
| P19-R41 | P19-AC-011; Section 8 | Current source and discriminating assertions traced; limitations below. Limitation: Native-smoke caller satisfies the shared policy; the other extant workflow remains independently materializing. | partially-met | P19-AUD-007 |
| P19-R42 | P19-AC-011; Section 8 | Actual canonical root replays accepted with current Companion and no auth/Agent/provider. Manifest run_root resolves Python evidence while Companion always validates runs/family; appended trial ID is unchecked before Python file reads. Static target-equivalence gap proved; complete invalid-shadow acceptance not established. | partially-met | P19-AUD-004 |
| P19-R43 | P19-AC-012; Sections 7 and 8 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R44 | P19-AC-012; Section 8 | Actual source/material/task/model/runtime hashes and concurrency/trial counts preserved. Retained binary and canonical build-tree record independently bind all694 unchanged current crate/manifests/cache-helper paths. All six usage/time execution limit fields are null and no explicit operator budget freeze was found; six real trials and source applicability are credited. | partially-met | P19-AUD-005 |
| P19-R45 | P19-AC-012; Section 8 | Current required cargo fmt fails at native_driver.rs:10054; Python discovery has three errors in two output-control fixture methods. Narrow Companion clippy/tests/doc tests/rustdoc and WSL doc checker pass; broader final workspace and ignored native union are not claimed complete. | not-met | P19-AUD-008, P19-AUD-009 |
| P19-R46 | P19-AC-012; Section 8 | Current local verifier derives repository-qualified pi/Harbor/Pier authority from preserved static-lock bytes and verifies nonempty exhaustive byte-identical three Opi captures. Separate required generic artifact audit passed on 36 actual capture files: no issues and no commit-reference tokens. The Python verifier does not independently scan nested tokens; the separate generic gate supplies that check. | met |  |
| P19-R47 | P19-AC-013; Sections 3 and 8 | Current source and discriminating assertions traced; limitations below. Limitation: pi-only test uses fixture helpers and still has two pi subjects with protocol mismatch; it proves independent path value and no Opi trial, not real pi native acceptance. Package listing broad content review and unchanged Minimal Runtime gates are parent checks. | met |  |
| P19-R48 | P19-AC-013; Section 3 | Current source and discriminating assertions traced; limitations below. Limitation: Synthetic task/grader fixture packages exist under tests and support hermetic tests; their presence is not an embedded upstream production grader. R41 separate duplicate CI policy persists, but no second sandbox engine found. | met |  |
| P19-R49 | P19-AC-013; Sections 4 and 9 | Changed format identifiers, fixtures, help, bilingual README and Unreleased are largely synchronized; current Rust module docs still assert automatic oracle preflight/fixture-only no-provider behavior contrary to native execution. | partially-met | P19-AUD-010 |
| P19-R50 | Section 9 | Local native scoring is explicitly selected and fail closed; disabling the optional Companion/native path does not activate invalid scores or alter the Minimal Runtime. No release rollback or runtime activation was performed. | met |  |
| P19-R51 | Section 5 | Normal native run uses only Integration selection; no run_oracle_preflight exists and native integrity oracle=None. Only explicit benchmark conformance witness selects registered oracle/nop; output labels no Agent score and cannot satisfy Integration binding or enter normal report pair denominator. | met |  |
| P19-R52 | Section 6 | Six genuine preserved trials qualify the claimed first-connection/tool/native path. TB2.1 tool pairs pi/Opi=4/4, TB3=78/95, DeepSWE=92/100; exactly one AgentEnd each; stream and bundle digests match. Integration pins match. Retained Opi executable hash and canonical source manifest verified; all694 crate/manifests/cache-helper source paths match this archive, with no added/missing paths. Current-source applicability is established without adopting prior conclusions. | met |  |
| P19-R53 | Section 3; PRIN-002; CTRL-007 | Current source and discriminating assertions traced; limitations below. Limitation: Ordinary conformance matrices use helper fixtures and are #[cfg(unix)]; actual native integration evidence is parent check/current raw six-trial material. No stable SDK admission is claimed. | met |  |
| P19-R54 | Section 8; PHASE-001 through PHASE-006 | Latest committed parent and registered supplemental sources govern obligations; implementation declarations do not waive criteria. No ledger/spec changes, new authority or Phase completion mutation made by this audit. | met |  |
| P19-R55 | Section 4; optional OpenAI connections | Optional OpenAI Completions/Responses remain separately named, fixture-only connections. Source-applicable actual-client matrix raw log executes both clients/all three local protocols successfully; no real-service qualification or fallback claimed. Exact secret value is not asserted by the scheme-only fake-server check. | met |  |
| P19-R56 | Section 3; PRIN-002 | All eleven task contracts inventoried. Existing adapters/lifecycle/report/bundle retained with no extra Cargo/Core seam. Drift: separate default CI materialization/oracle policy survives; public reasoning and native capability verdicts differ. Numerical historical deletion not inferred from prohibited history. | partially-met | P19-AUD-007, P19-AUD-011 |

## Standards Review

The Companion remains unpublished with no Opi crate dependency; dependency directions and Minimal Runtime activation are unchanged. Existing private adapters, one outcome owner, safe typed boundary checks, bounded supervision and external cache are retained. Current Companion clippy/tests/docs pass; required rustfmt fails (AUD-008), and two output-control fixture methods fail before their intended assertions (AUD-009). Public/current Rust docs retain superseded behavior claims (AUD-010). No Cargo.lock regeneration, cache deletion or changes to historical artifacts occurred.

## Spec Review

Production native binding rejects absent/wrong/oracle/stale Agent/job/task/artifact identities. TB3 declaration-only collection and separate-verifier binding, DeepSWE base/patch/pristine-verifier binding, selected protocol/model/runtime projection and truthful failure/coverage reporting have discriminating current source/tests. The current six raw trials prove real Zhipu Anthropic glm-5.3-flash execution by both Agents for all three families. Some valid native outcomes are task failures/zero; that is allowed and not confused with provider/infra invalidity.

Current actual evidence manifest SHA-256 `9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce` lives at `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root`. Exported Harbor/Pier integration canonical manifests exactly match raw material pins: `4afea57c233cb8164d826dcb65c596b608095fad7440f6972a066730e1fcafef` and `702902c24dcc51b25af9a2ee81185283cbd642a43d570f0c5fa653ee6979cba9`. Retained Opi executable independently hashes to `0ccc2b7f5be5257068c1d3bb8cb73dfeaa0e482c16b468872179a0b1720e2ad2`. Raw build identity (`48409ba1dfb4c74a4ec8c1c1dad153bb5343518d0d2c5122d212cac042b7501a`) links canonical source manifest (`8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53`); recomputed tree digest `5e2f82209a0573de8f3e92970a56274992f231495e9b4c0d54f97a497f6e1d54` matches. All694 crate/manifests/cache-helper source files match current archive bytes with no added/missing paths. Thus frozen source applicability is established despite different repository commit labels; no prior report/conclusion was adopted.

The selected output-byte limit is not enforced (AUD-001), operator usage/time freeze remains incomplete (AUD-005), and four controlled pinned-native witness/regrade predicates remain not-assessable (AUD-006). Raw actual-client matrix and Docker cancellation logs each record1 executed pass and apply to the unchanged current source, so those requirements are credited. Their retained temporary request/Docker directories are absent now, limiting ID-level replay; this is disclosed. Final full-workspace gate union is not claimed after current required failures.

## Security, Invariants, Integration, Test Quality, and Residuals

Credential reference resolution is selected-client-only; current native modules sanitize/clean before grading and task mounts exclude broad host directories/socket. Original host staging lacks private permission admission (AUD-002). Exact canary redaction does not address private thinking export classification (AUD-003); only metadata/type counts/digests were recorded, never reasoning text.

Run/recovery state is durable and fail closed: journaled intent, one settled interpretation and seals keep effect-unknown/cancelled/unstarted truthful; replacements take fresh IDs without mutating previous attempts. Actual Docker cancellation exact-filter raw log and unchanged test-body predicate are credited. Offline byte verification and report reconstruction pass; successful real pinned native regrade was not proved. Current regrade source validation rejects missing cache/base/artifact/service contradictions and uses verifier-only entrypoints with no Agent/provider calls.

Acceptance root admission and semantic-target equivalence have a current static gap (AUD-004). Existing canonical supplied evidence passes the whole verifier; structural-only probes do not prove full invalid acceptance. The unsuccessful shadow falsification was explicitly refuted. Generic artifact audit passed on a nonempty/exhaustive byte-bound 36-file Opi view; its zero commit references do not establish additional runtime message quality beyond the file shapes it actually parses.

Residual collection-filesystem concern: collected_digest recursively follows directory checks and uses file reads; a link-cycle/FIFO problem would require pinned upstream extraction admitting those shapes. Current guarded material/result readers and absent locked extraction source prevent a proven reachability claim. It is not a finding. No speculative public seam/crate/config compatibility layer or embedded native dataset/grader/image/sandbox engine was found.

## Minimum-change Conformance

Task admission notes were read only from the allowlisted scope projection. Historical numeric added/deleted counts are intentionally not inferred: the audit evaluates current removal of superseded authorities and surviving glue. All eleven tasks record reuse, necessity, ceiling, consumers, deletion and residuals below.

### Task 19.1 — conforming

- Reuse search: conforming
- Surface necessity: conforming
- Simplification ceiling: conforming
- Production consumers: runner::experiment::run_native_trial; TerminalBench21Adapter::settle; TerminalBench30Adapter::settle; DeepSweAdapter::settle; ReportBuilder; native-smoke.sh.
- Nonproduction consumers: native_driver; benchmark_integration_conformance; agent_integration_conformance; authority_boundaries; end_to_end_report.
- Net deletion: Current structural inspection confirms automatic run_oracle_preflight and newest_job_result authority absent; host AgentExecution remains only in hermetic fixture branch. Numeric historical net line count not inferred or inspected. No Git history used.
- Residual glue: Private adapter/lifecycle glue and handoff/result persistent schemas; fixture answer.txt helper remains confined to hermetic path; stale oracle/fixture-only rustdoc is NAT-DOC1.

### Task 19.2 — drifted

- Reuse search: conforming
- Surface necessity: conforming
- Simplification ceiling: drifted
- Production consumers: ResolvedExperiment::resolve + project_connection; runner prepare_native/run_native_trial/stage_native_handoff; NativeMaterial::load and output_control::admit; Harbor OpiAgent/PiAgent and Pier OpiAgent/PiAgent; current public run/validate/plan; provider-neutral existing Opi native config/request clients.
- Nonproduction consumers: experiment projection unit tests; experiment_contract; native_driver actual-client matrix + stand-in fixtures; agent_integration_conformance; scripts/test_scripted_provider.py; scripts/test_native_integrations.py; profiles/agents and selected connection examples/current README.
- Net deletion: Current native AgentMaterial has no inline credential payload or fixed OpenAI-only launch configuration; current native adapters take selected projections and scripted configuration is confined to fixtures. Historical net line deletion is not established because no Git history/prior outputs were read; current removal-intent surfaces are satisfied.
- Residual glue: Shared private ProcessSupervisor/AgentAdapter/importers are justified existing lifecycle seams.; Harbor/Pier integration modules repeat process/staging mechanics for pinned different runner APIs; no standalone speculative public crate extracted.; Unused LocalOnly production classification and public Effective reasoning versus runtime Unsupported require current simplification, not historical compatibility.

### Task 19.3 — conforming

- Reuse search: conforming
- Surface necessity: conforming
- Simplification ceiling: conforming
- Production consumers: TerminalBench30Adapter::settle; runner::experiment::run_native_trial; regrade::regrade_one; cli::conformance::run_benchmark_case.
- Nonproduction consumers: native_driver TB3 cases; benchmark::task_lifecycle unit tests; benchmark integration conformance.
- Net deletion: Current structural inspection confirms automatic run_oracle_preflight and newest_job_result authority absent; host AgentExecution remains only in hermetic fixture branch. Numeric historical net line count not inferred or inspected. No Git history used.
- Residual glue: Task-declared lifecycle/collection fields extend existing NativeTrialBinding; no second container runner or stable SDK.

### Task 19.4 — conforming

- Reuse search: conforming
- Surface necessity: conforming
- Simplification ceiling: conforming
- Production consumers: DeepSweAdapter::settle; runner::experiment::run_native_trial; regrade::regrade_one; cli::conformance::run_benchmark_case.
- Nonproduction consumers: native_driver DeepSWE cases; test_native_integrations loops over Harbor/Pier.
- Net deletion: Current structural inspection confirms automatic run_oracle_preflight and newest_job_result authority absent; host AgentExecution remains only in hermetic fixture branch. Numeric historical net line count not inferred or inspected. No Git history used.
- Residual glue: Pier integration mirrors selected-Agent setup/run around the runner-specific base/environment/network API; shared discriminating conformance exercises both adapters; upstream owns collect/apply/grading.

### Task 19.5 — conforming

- Reuse search: NativeMaterial/verify_pinned/package digest and supervised external producer reused; doctor/prepare consumers remain private.
- Surface necessity: CLI doctor/prepare and local-origin verified inventory fit existing Companion; no Cargo/Core seam.
- Simplification ceiling: Local selected-source Linux x86_64 preparation only; upload receipt and repository fixture requirements removed from local path. Surviving CI duplication is task19.10 drift.
- Production consumers: cli::prepare; NativeMaterial::load; run_experiment; materialize-external-locks prepare mode.
- Nonproduction consumers: native_driver independent_directory_prepare and cache/resource refusal; external-lock fixtures and Python transport tests.
- Net deletion: Current local path has no mandatory upload-receipt check; numerical historical line deletion not inspected.
- Residual glue: Pinned static/resolved lock transport, source acquisition/cache/OCI admission; upstream tools own acquisition mechanics.

### Task 19.6 — conforming with R35 behavior defect

- Reuse search: ResolvedExperiment/NativeMaterial/DeclaredTrial/ModelControls and comparison identity reused.
- Surface necessity: Plan CLI plus inventory/selection/frozen plan justify intrinsic immutable state boundary; no speculative trait or dependency.
- Simplification ceiling: No full TB3 completion promise; unsupported resources/lifecycle and infeasible selection reject. Accepted captured-output cap violates enforcement trigger (AUD-001).
- Production consumers: cli::plan; prepare_native; run --plan; ReportBuilder planned coverage.
- Nonproduction consumers: native_driver plan/resource/full/subset/material drift and report-after-loss cases; experiment_contract positive-bound tests.
- Net deletion: No second native classifier or material source added; numerical historical deletion not inspected.
- Residual glue: Full metadata enumeration versus selected materialization, canonical path/digest source, task-specific requirements and engine observation.

### Task 19.7 — conforming

- Reuse search: RunBundle::verify/OfflineRegrader/ReportBuilder/comparison/coverage and runner sealed interpretation reused.
- Surface necessity: Verify accurately names byte verification; report derives immutable evidence with family-native schemas and planned scope.
- Simplification ceiling: No synthetic metric headline or independent failure classifier; conformance-only legacy read classification remains for old source.
- Production consumers: cli::verify; cli::report; ReportBuilder; runner sealed settlement.
- Nonproduction consumers: report_contract; bundle_recompute; end_to_end_report; authority_boundaries.
- Net deletion: Byte-only regrade behavior replaced by verify and explicit native regrade; report consumes owner interpretation. Historical numeric diff not inspected.
- Residual glue: Family-native metric mappings, verified run-control versus surviving trials, read-only legacy conformance reader.

### Task 19.8 — structurally conforming; native proof not-assessable

- Reuse search: RunBundle verification/BenchmarkExecution/family adapters/NativeTrialBinding/native cleanup and reporter reused.
- Surface necessity: Fresh native regrade CLI requires explicit material/output and binds derivative identity to immutable source; no Agent/Core/provider seam.
- Simplification ceiling: Reject absent materials/services/base/artifacts; no synthesized grade or remote cache/provider fallback. Actual native proof missing (AUD-006).
- Production consumers: cli::regrade; NativeRegrader::regrade_one; verifier-only Harbor/Pier integration; native_cleanup.
- Nonproduction consumers: native_driver native_offline_regrade fixtures/refusals; ignored native_verifier_only_regrade_witness.
- Net deletion: Distinct byte verifier and fresh regrade are present; historical numerical deletion not inspected.
- Residual glue: Material/source validation, private verifier request and owned container lifecycle; original bundles immutable.

### Task 19.9 — conforming

- Reuse search: conforming
- Surface necessity: conforming
- Simplification ceiling: conforming
- Production consumers: runner::experiment::run_native_trial; cli::conformance native settlement; runner::experiment::recovery_report; process::ProcessSupervisor; regrade supervised process.
- Nonproduction consumers: native_driver cancellation/cleanup cases; assembled_run recovery/replacement cases; test_native_integrations command/cleanup tests.
- Net deletion: Current structural inspection confirms automatic run_oracle_preflight and newest_job_result authority absent; host AgentExecution remains only in hermetic fixture branch. Numeric historical net line count not inferred or inspected. No Git history used.
- Residual glue: Private native_cleanup module bound to daemon IDs and task roots, process-group/JobObject platform mechanism, bounded cleanup capture-helper, existing intent/settlement/seal journal; no extra public knobs.

### Task 19.10 — drifted

- Reuse search: Existing native-smoke workflow, materializer and CLI prepare/conformance/run/verify/report reused; native smoke shared prepare replacement is present.
- Surface necessity: Conforming: offline script and explicitly supplied root/out/companion only, no new Cargo dependency or Agent Core seam.
- Simplification ceiling: Triggered/drifted: independent default external-lock materialization/oracle path remains beside local prepare mode.
- Production consumers: native-smoke workflow and public local CLI; external-lock materialization workflow; separately authorized local operator offline verifier.
- Nonproduction consumers: Python fixtures/regressions; Rust native_driver public CLI acceptance tests; bilingual README/examples.
- Net deletion: Not assessable numerically without prohibited Git/history; current native-smoke no longer synthesizes task material, but independent default external-lock orchestration persists.
- Residual glue: Cache/client transport and exact-client loopback proof routing in native-smoke; CI dispatch/upload transport validators; default legacy external-lock materialization path.

### Task 19.11 — partially-met acceptance freeze/proof gates

- Reuse search: Reuses same local verifier and current Companion verify/report; permitted raw actual root exists and current replay accepts six trials.
- Surface necessity: Conforming current admitted scope: redacted evidence/report handoff, no new runtime/API/activation.
- Simplification ceiling: Source task forbids provider/auth/live harness calls and autoarchive; those limits preserved in this audit. Historical delivery/report artifacts excluded, so net implementation changes not inferred.
- Production consumers: authorized local operator preserved evidence; offline verifier/Companion verify/report.
- Nonproduction consumers: current audit replay; existing Python structural tests and Rust fixture-rejection test.
- Net deletion: not-assessable; no historical diff permitted
- Residual glue: Preserved raw native evidence, repository identity cache, Opi scoped view and raw build/source manifest. Source applicability proved; operator usage/time freeze and pinned native witnesses/regrade remain incomplete.

## Findings

All findings publish with status `unverified` as required by the finding contract; remediation owns subsequent verification/closure. Blocking evidence gaps describe absence within the inspected current raw scope, not proof of a broken implementation.

### P19-AUD-001: Requested captured-output byte limit is accepted but never enforced

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R35
- Claim: A positive execution.max_output_bytes is accepted and serialized even though no execution consumer applies it or rejects/revises the request before effects.
- Evidence: `crates/opi-eval/src/experiment.rs:384`: ResolvedExecution calls the field an enforceable captured-output limit; parser at706 accepts a positive value. `crates/opi-eval/src/runner/plan.rs:611`: Complete runtime source search finds only declaration, parser/default and plan metadata; no runtime read. `crates/opi-eval/README.md:441`: Documentation acknowledges recorded-only value alongside fixed outer-job and 64MiB limits; design§4:160-161 instead requires rejection or explicit revision.
- Refutation attempted: Positive validation, fixed profile capture bounds and native 64MiB importer caps are real. They cannot enforce max_output_bytes=1 (or any smaller accepted cap). README disclosure is not a per-request explicit revision; model_controls.required applies different keys. This is a current static missing-consumer invariant, not a claim of unlimited Rust memory.
- Reproduction/check: `rg -n max_output_bytes crates/opi-eval/src`; `Resolve/plan a valid native config with execution.max_output_bytes=1; trace accepted value to execution (no runtime consumer).`
- Suggested closure: Apply the requested byte limit at the owned capture boundary with bounded settlement, or refuse it before effects; add a public execution/refusal case that observes the requested smaller limit.
- Confidence: high

### P19-AUD-002: Host temporary credential files inherit ordinary output-root permissions

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R33
- Claim: On supported Unix with umask022 and a searchable caller-supplied run root, native host handoff/config files containing resolved credentials are created readable by other users for the job lifetime.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:723`: fresh_run_root creates directories with default modes; root claim imposes no private-root permissions precondition. `crates/opi-eval/src/runner/experiment.rs:1560`: integration directory uses create_dir_all; config and handoff use fs::write at1565/1585 before any mode protection. `crates/opi-eval/src/experiment.rs:1157`: Opi secret enters agent_env copied into handoff; Pi inline apiKey at1357 enters models configuration.
- Refutation attempted: Native container stage0700, runtime-preflight files0600, pre-verifier cleanup, redacted handoff identity and host partial-diagnostic0700/0600 are present. None protects the original host secret file. Tempfile test roots are private and mask the supported public-root condition; no runtime mode admission/umask/chmod covers it. Static POSIX creation invariant is sufficient; no dynamic secret/mode probe and no Windows ACL claim.
- Reproduction/check: `Inspect fresh_run_root and stage_native_handoff with a caller-supplied searchable root under Unix umask022: create_dir_all gives0755 and new fs::write gives0644.`; `Search runtime permission guards and credential projection consumers; no private host-stage guard exists.`
- Suggested closure: Create/admit private host staging before writing credentials and preserve cleanup on partial failure; verify POSIX modes using synthetic credentials and a nonprivate ancestor.
- Confidence: high

### P19-AUD-003: Native private thinking content is classified as exportable evidence

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R34
- Claim: The native capture path seals private thinking-bearing Agent terminal/stdout bytes as Sensitivity::Exportable instead of excluding or privately classifying that content.
- Evidence: `crates/opi-eval/src/bundle/mod.rs:875`: Sensitivity contract explicitly forbids classifying private raw reasoning Exportable; LocalOnly is available. `crates/opi-eval/src/runner/experiment.rs:3495`: Shared stage helper unconditionally sets Exportable at3510; no production LocalOnly consumer or thinking normalization found. `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/native/agent-terminal.json`: Metadata-only probe: digest aacb815813bf869aee9a1b0c17b355860e373cea262caa5f64c220ae61dad25b, Exportable, one client_message thinking object. `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/runs/terminal-bench-2.1/trials/trial-baseline-pi-openssl-selfsigned-cert-r1/bundle/native/agent-stdout.log`: Digest cab0d55d7d493bd0c50c19013eed3644935a6ad2e916c6e2794888fbe05612d6, Exportable, 15 thinking objects including12 nonempty strings. Opi TB2.1 stdout has20 nonempty strings; all six stdout captures have thinking type tags. No reasoning text quoted or copied into audit.
- Refutation attempted: Exact credential/canary scanning and full-capture selection do not classify private reasoning. Pi --thinking off did not prevent the actual terminal object. Terminal-only suppresses raw streams but still stages full terminal client_message; LocalOnly exists only as a type, not a production insertion choice. Actual capture hashes match bundle manifests and current integration hashes match raw material pins.
- Reproduction/check: `Inspect type tags/counts and SHA-256 only in allowed native captures; compare manifest sensitivity without printing content.`; `Trace terminal/full capture callers to stage() and search all production sensitivity classifications.`
- Suggested closure: Preserve required attributable tool/terminal facts while omitting private thinking from exportable artifacts or retaining it only under explicit private classification; test actual native-format thinking input in full and terminal-only policies.
- Confidence: high

### P19-AUD-004: Acceptance verifies a different run root from the one it inspects

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R42
- Claim: Manifest-selected trial roots drive Python native/artifact checks, but Companion semantic verification always targets root/runs/family; trial IDs also reach early reads without containment admission.
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:168`: Only run_root itself is contained; unchecked trial.id is appended at185/195 and can replace the prefix if absolute. `crates/opi-eval/scripts/verify-local-acceptance.py:925`: check_companion hardcodes root/runs/family at931, independent of resolve_run_root used by native/material/capture checks. `C:\Users\Luiz\AppData\Local\Temp\opi-p19-audit-8ozcu3oo\assurance-probe-results.json`: Already-run harmless temporary structure-only probe accepted external canary native gate/bundle hash via absolute trial ID. Companion omitted; this proves early read-boundary gap only.
- Refutation attempted: Later sealed() checks and Rust RunBundle verification reject unsafe inputs at their own seam; they cannot undo Python reads or establish root equivalence absent an admission rule. Genuine supplied roots are canonical and current full replay passes. A complete temporary shadow-root attempt did NOT establish invalid false acceptance: added schema field was ignored and direct Companion also accepted. No end-to-end invalid-shadow claim is made; static semantic-target mismatch remains independently proved.
- Reproduction/check: `Trace resolve_run_root/check_native_evidence/check_bundle_digests against check_companion at the current head.`; `Structural-only harmless canary probe in assurance-probes.py; consult explicit limitations in assurance-probe-results.json.`
- Suggested closure: Admit canonical contained trial paths before all reads and use those same distinct admitted run roots for Companion verify/report; reject conflicting family roots. Test actual semantic verification target and benign out-of-root-read refusal.
- Confidence: high

### P19-AUD-005: Real acceptance freeze lacks explicit usage/time scope

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R44
- Claim: Inspected accepted evidence establishes genuine source-applicable six-trial execution, but does not establish the operator-frozen usage/time budget required for final-state Phase acceptance.
- Evidence: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:297`: Operator must freeze tasks/builds/material/control/concurrency/usage/time budget before real run; dynamic claims require raw proof at306-309. `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/manifest.json`: SHA2569897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce. Exact material/executable/task/upstream/control/concurrency digests exist; no explicit usage/time-budget record in inspected preserved inventory. Build pointer was followed only to raw identity/source records and successfully verified. `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/runs/*/trials/*/bundle/control/experiment.json`: All six execution time_limit_seconds, agent_timeout_seconds, verifier_timeout_seconds, max_output_tokens and max_output_bytes are null; max_trials2/concurrency1. These are not an explicit total usage/time freeze. `crates/opi-eval/scripts/verify-local-acceptance.py:445`: Controls check validates concurrency/model/output-control identity but has no required pre-run operator usage/time budget record. Structural fixture without that record accepted before Companion.
- Refutation attempted: Six native Agent trials, exact build/material/task pins, selected Anthropic model, concurrency1, max_trials2 and exact output-control proof are credited. Current integration bytes match pins. Retained Opi binary hash and canonical source-tree manifest were independently checked; all694 relevant current source paths match with no new/missing files, closing build applicability. These positives still do not establish explicit operator usage/time scope; individual model defaults/fixed timeouts or a finite six-trial count are not that required record. Missing freeze proof is an acceptance gap, not fabricated execution.
- Reproduction/check: `Verify manifest hash and inspect only allowed raw preserved inventory/control fields, no adjacent remediation conclusions.`; `Replay current local verifier and contrast its accepted:true result with Section8 freeze obligations.`
- Suggested closure: Supply independently verifiable pre-run freeze including authorized usage/time scope and retained failure/replacement inventory; validate it offline without new provider calls.
- Confidence: medium

### P19-AUD-006: Controlled pinned-grader witnesses and fresh native regrade lack executed proof

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-R07, P19-R10, P19-R13, P19-R31
- Claim: Current deterministic tests, source-applicable raw client/Docker logs and genuine six-trial inventory do not close the three controlled positive/negative pinned-grader witnesses or successful fresh native regrade criteria.
- Evidence: `crates/opi-eval/tests/native_driver.rs:3486`: native_tb21_artifact_witness, native_tb3_artifact_witness3508 and native_deepswe_patch_witness3552 are ignored; actual pinned correct/incorrect grader outputs not located. `crates/opi-eval/tests/native_driver.rs:7651`: native_offline_regrade default case uses helper fixtures. Native verifier-only witness is ignored; located raw c011 regrade stdout has rejections and trials=[], not successful native grades. `C:\Users\Luiz\AppData\Local\Temp\opi-p19-audit-8ozcu3oo\rust-tests.log`: Current central suite:369 passed,11 ignored. Three pinned witnesses and native regrade did not execute here; no raw exact-filter results located within current acceptance/indexed raw-proof scope.
- Refutation attempted: Native conformance shares production BenchmarkExecution/binding; six genuine trials prove real native loop; fixture regrade proves derivative immutability/zero calls. Raw actual-client matrix and Docker-cancellation logs each show1 executed pass and are applicable to unchanged694 current source files, so R17/R39 are credited. Those positives do not discriminate each controlled real grader correct/incorrect witness or establish fresh pinned offline grading. Exact runner paths in raw material are absent locally; no credentials/provider/native task execution occurred in this audit. This is a bounded evidence gap, not a reproduced runtime defect.
- Reproduction/check: `cargo test -p opi-eval --test native_driver <native_tb21_artifact_witness/native_tb3_artifact_witness/native_deepswe_patch_witness/native_verifier_only_regrade_witness> -- --ignored --exact (NOT RUN; needs pinned materials)`; `Inspect current raw acceptance inventory, source-bound native logs and exact rejected regrade stdout; default suite excludes these required filters.`
- Suggested closure: Supply byte-bound source-applicable raw results for the three pinned witness filters and successful native regrade, or run them with prepared local WSL/Linux pinned materials and no live model/credentials. Keep missing claims not-assessable until then.
- Confidence: medium

### P19-AUD-007: External-lock CI retains a separate materialization and oracle policy

- Axis: residuals
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-R41, P19-R56
- Claim: The surviving external-lock workflow selects default script mode, which independently acquires/materializes and oracle-grades instead of delegating the same local prepare/conformance policy.
- Evidence: `.github/workflows/opi-eval-external-lock-materialization.yml:58`: Calls materialize-external-locks.sh without prepare mode. `crates/opi-eval/scripts/materialize-external-locks.sh:42`: Local prepare mode42-420 exits at423; default425-1000 separately fetches TB2.1 files, creates closures, pulls OCI, invokes Harbor oracle and requires reward. `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:284`: Thin regression callers must not own second materialization/scoring policy; task19.10 simplification trigger replaces duplicate paths.
- Refutation attempted: native-smoke materialize-configs1024-1039 now delegates shared prepare and its negative delegation tests pass. External-lock transport/static producers are pinned. These checks preserve safety of each route but do not remove the separate default policy; no production score misattribution is claimed. Severity Minor for duplicate-policy/maintenance drift.
- Reproduction/check: `Trace default workflow invocation versus script prepare-mode exit and default Harbor Trial oracle code.`; `Inspect test_verify_native_smoke_ci.py delegation/refusal assertions and test_verify_external_lock_ci.py workflow contract.`
- Suggested closure: Delegate the surviving workflow to shared local materialization/conformance, retaining transport-only packaging and digest checks; remove now-unused policy after shared refusal/conformance proof.
- Confidence: high

### P19-AUD-008: Required rustfmt gate fails on the committed test source

- Axis: standards
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-R45
- Claim: cargo fmt --check --all fails at the admitted-static-lock assertion in native_driver.rs.
- Evidence: `crates/opi-eval/tests/native_driver.rs:10054`: Assertion multiline wrapping is rejected; formatter expects assert!(stderr.contains("admitted static lock"), "{stderr}");. `C:\Users\Luiz\AppData\Local\Temp\opi-p19-audit-8ozcu3oo\fmt.log`: Fresh exported committed-source check exit1; only reported diff is10054-10059.
- Refutation attempted: Companion clippy and all Rust tests pass; neither substitutes for the explicitly required formatting gate. No source was formatted in audit.
- Reproduction/check: `cargo fmt --check --all`
- Suggested closure: Apply rustfmt to task-owned changed test source and rerun the required format check.
- Confidence: high

### P19-AUD-009: Output-control fixtures depend on a nonexistent fixed uv executable

- Axis: test-quality
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-R45
- Claim: Two deterministic Python fixture tests mock command -v uv as /usr/bin/uv, but current producer copies that real path before the fake subprocess boundary, causing three errors on ordinary local WSL installations.
- Evidence: `crates/opi-eval/scripts/test_native_smoke_output_control.py:81`: Mock returns fixed /usr/bin/uv rather than a fixture executable; materialize executes extracted producer. `crates/opi-eval/scripts/native-smoke.sh materialize-configs embedded MATEOF`: Current producer copies selected uv executable into pinned cache before proof/validation; mock path does not exist. `C:\Users\Luiz\AppData\Local\Temp\opi-p19-audit-8ozcu3oo\python-linux.log`: 251 tests,3 errors,1 skipped. test_proof_precedes_material_validation_and_config_freeze and test_failed_or_incomplete_probe_never_freezes_material fail FileNotFoundError; second subcase hits existing partially staged static directory.
- Refutation attempted: Other Python cases pass and actual production can use an installed uv from PATH. These tests promise synthetic external command fixtures but now require an undeclared machine-specific file, preventing their intended ordering/refusal assertions. This is a test-seam portability defect, not proof that production uv acquisition is broken.
- Reproduction/check: `python3 -m unittest discover -s crates/opi-eval/scripts -p test_*.py (WSL Ubuntu; export)`
- Suggested closure: Use a real synthetic fixture uv file and reset per-subcase staging as needed; rerun both exact methods and all affected script tests without global executable installation.
- Confidence: high

### P19-AUD-010: Current Rust documentation describes superseded oracle and fixture-only behavior

- Axis: standards
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-R49
- Claim: Rust module docs claim automatic per-task oracle preflight and deny real native Opi/pi/provider operations although the public native-material path does both selected Agent execution and native grading without implicit oracle.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:13`: Module contract and native preparation comment near1167 still describe automatic oracle preflight. `crates/opi-eval/src/cli/run.rs:10`: Public run rustdoc says fixture-generated helpers never invoke real Opi/pi/provider or upstream task paths. `crates/opi-eval/src/runner/experiment.rs:1212`: Actual normal native path explicitly selects integration; integrity oracle=None; separate conformance selects explicit diagnostic oracle.
- Refutation attempted: Current executable behavior, README, oracle diagnostic tests and native result-binding tests implement the revised contract. Documentation checker/rustdoc build passing checks formatting/links, not semantic truth. This finding does not reduce R51 runtime conformance.
- Reproduction/check: `Read cited module docs alongside run_native_trial/native_integrity_review and oracle_witnesses_are_native_only_conformance_diagnostics test.`
- Suggested closure: Describe explicit hermetic versus selected-native run, and oracle diagnostic-only policy in current rustdoc; synchronize any corresponding user-facing bilingual prose if changed.
- Confidence: high

### P19-AUD-011: Public reasoning effectiveness contradicts the native capability verdict

- Axis: spec
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P19-R16, P19-R56
- Claim: project_controls labels any optional requested reasoning value Effective while native admission and settlement say effectiveness cannot be proved and pinned Pi launch forces thinking off.
- Evidence: `crates/opi-eval/src/experiment.rs:1193`: Requested reasoning Value becomes ProjectedControl::Effective unconditionally; public contract1586 calls it exact effective value. `crates/opi-eval/src/experiment.rs:1017`: Required reasoning rejects explicitly because pinned Pi disables thinking and current native execution cannot prove effective value. `crates/opi-eval/src/runner/experiment.rs:2582`: Settlement records every supplied reasoning Value unsupported; Harbor810/Pier855 Pi launch --thinking off.
- Refutation attempted: Required unsupported reasoning/sampling reject before effects and final report protects strict comparison with Unsupported. Effective rustdoc partly narrows meaning to projected config expression. That does not justify a public effective verdict for a value overridden by selected launch; consumers see inconsistent capability truth. Minor because native strict comparisons are protected.
- Reproduction/check: `Inspect optional reasoning projection versus required admission and native settlement; compare pinned Pi --thinking off launch.`
- Suggested closure: Use one truthful supported/effective projection for public config, admission and settlement, retaining requested value plus reason when actual effectiveness cannot be established.
- Confidence: high

## Verification Commands

All implementation checks ran on the sealed export. WSL Ubuntu was used for Unix/product checks, with the existing leased persistent target `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e`. Cache status was inspected; no clean/new session target/cache deletion. Current raw acceptance replay used the Companion built from this export.

| Command | Result | Scope / limit |
|---|---|---|
| `cargo fmt --check --all` | FAIL exit1 | AUD-008; committed assertion formatting. |
| `cargo clippy -p opi-eval --all-targets --locked -- -D warnings` | PASS exit0 | Companion all targets; not workspace-wide Phase-exit clippy. |
| `cargo test -p opi-eval --all-targets --locked` | PASS exit0 | 369 passed,0 failed,11 ignored;14 result blocks. |
| `cargo test -p opi-eval --doc --locked` | PASS exit0 | No doctest examples; not runtime proof. |
| `RUSTDOCFLAGS=-Dwarnings cargo doc -p opi-eval --no-deps --locked` | PASS exit0 | Companion warnings denied. |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p "test_*.py"` | FAIL exit1 | 251 tests,3 errors,1 skipped; two output-control fixture methods (AUD-009). |
| `python3 scripts/opi-doc-check.py` (WSL export) | PASS exit0 | Windows-export tar symlink failures excluded as platform artifacts. |
| `verify-local-acceptance.py --root <current preserved root> --out <fresh audit output> --companion <export build>` | PASS exit0,accepted:true | Exact six genuine preserved native trials; current per-family offline verify/report. Does not close AUD-004/005/006. |
| `python scripts/opi-artifact-audit.py <actual-root>/opi-artifact-audit --workspace-root D:/Luiz/Odradek/opi --json` | PASS exit0 | Script/cwd in export;36 files,0 issues,0 commit-reference tokens. |
| Full-workspace clippy/test/doc gates and ignored native/client/Docker filters | NOT RUN | Audit selected narrow current Companion checks; established failures already block final union. Missing dynamic proof remains explicit. |
| Raw exact-filter native_protocol_client_matrix and native_owned_resource_cancellation logs | VERIFIED APPLICABLE | Each1 passed,0 failed,0 ignored; current relevant source694 files identical to canonical build manifest. Temporary detailed evidence directories absent; no fresh native/client task execution. |
| Raw build identity/binary/source-manifest verification | PASS | Canonical tree hash and retained executable digest match; no source file additions/deletions/drift in relevant closure. |
| Harmless current structural probes | BOUNDED | Early outside-root canary reads accepted; no Agent/auth/provider. Full invalid-shadow false acceptance not established. |

Fresh raw check logs are external under `C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo`. Their SHA-256 identities are recorded below to distinguish current observations from any historical claim.

| Raw check artifact | SHA-256 |
|---|---|
| `fmt.log` | `6265747c0da3d95201a205e608d8df84ae0ca6d276145a12e6f9dff437dab42b` |
| `clippy.log` | `e7f3e13c43332a4d6063df8d591c33f1818f506485958a513e7b8848c32488da` |
| `rust-tests.log` | `20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f` |
| `doc-tests.log` | `78ddb567ab50d2efea76ef4c67cd9bc3bde0538fba4b5869dd96413b051cfb31` |
| `rustdoc.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `python-linux.log` | `9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79` |
| `doc-linux.log` | `f6daa323bc29663fca184eebe13f625b82f750ee6dc2f9a70be199bc9397309c` |
| `acceptance-current.json` | `83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06` |
| `artifact-audit.json` | `2cadfd15e25314372b8ae725429aa8af62ce1f1023a316c9fd11a9852485b89a` |
| `connection-sensitive-metadata.json` | `6bfa16fd2dbcc323176b81658d989ff1d9ecdfefa11f1c55f964c60973c17c45` |
| `assurance-probe-results.json` | `e65613c993665bbc54a6866e6c022a74490595e8e7f59da41529747c5918af49` |
| `assurance-shadow-reproduction.json` | `7be2710f91d1ef215344e7aa53e61772ae16916b39dc7123c2d8db73689411ce` |
| `build-applicability.json` | `857e5b492345b798b879e3664bb7f356cfaaf64297768d4169e4f5b2332fe6f5` |
| `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native_protocol_client_matrix.log` | `02aeb9b006c72fc55815ff09830f47da83c30c0a1cdafba834db2c9347d392ff` |
| `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native_owned_resource_cancellation.log` | `b5482323a6defe3c1890cc95b032f8902e189ff1287afbbae1f9578712939d49` |
| `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native_conformance_owned_cleanup_witness.log` | `79752570d04a252d31d2a571dc1f17def443abc89c802da783fa97974ebae455` |
| `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native_client_provider_failure_report.log` | `4d6965d60d0cb21b67bafe0d2b0e399e103881f636e6a11e5aa1fb60d08f9929` |

## Verdict Rationale

FAIL follows mechanically from mandatory non-met states. Six actual trials and broad deterministic tests are retained as positive evidence; they do not waive independent privacy/boundary/budget, missing native proof or required final gate obligations. No claim of Phase completion, performance superiority or successful full verification union is made. The report/requirements/findings/metadata are installed only through assurance_set; final validator results govern active membership and aggregate verdict.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "2465aeb80ad84d06172acc142f6d5ffbceb201cc",
  "audit\u005frun\u005fid": "phase19-codex-gpt61-2465aeb-20261001t055252z",
  "comparison": {
    "comparisons": [
      {
        "cause": "Automatic selected-key enrollment closes the specific opt-in-only seal guard. Current host-file permissions and private-thinking classification are separate boundaries, not recurrence of this exact hole.",
        "current\u005fevidence": "FIXED at current audit head: runner/experiment.rs:1865-1886 automatically enrolls the resolved selected env/file credential in private seal\u005fscan\u005fsecrets; the seal scan at2545 consumes it. native\u005fdriver::remediation\u005fp19\u005fselected\u005fcredential\u005fecho\u005fis\u005fnever\u005fsealed passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found selected credentials could be echoed into Exportable captures and sealed when --canaries was omitted.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain the no-canaries selected-credential echo refusal test for both credential sources."
      },
      {
        "cause": "Same-scope replacement remains allowed; every real reexecution now receives a fresh trial identity.",
        "current\u005fevidence": "FIXED: runner/experiment.rs:3967 refreshes every rerun trial ID before target-group naming changes. assembled\u005frun::replacement\u005ffresh\u005fresolved\u005fpairs\u005fpreserve\u005foriginal\u005fscope passed and now checks fresh identities for reexecuted unrelated groups.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found replacement reexecuted non-target groups under old trial IDs.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain multi-group replacement checks covering all reexecuted IDs and immutable prior bundles."
      },
      {
        "cause": "Installed executable bytes are now admitted by the durable material loader, rather than being preparation-only inventory.",
        "current\u005fevidence": "FIXED: material/6 carries interpreter/entrypoint PinnedFile values; runner/material.rs:133-176 checks expected paths and verify\u005fpinned byte identities. native\u005fdriver::remediation\u005fp19\u005frunner\u005finstalled\u005fbytes\u005fare\u005freverified passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found material/5 did not bind installed runner interpreter and console-entrypoint bytes.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-003",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain interpreter/entrypoint byte-tamper rejection before plan/run admission."
      },
      {
        "cause": "Observed package reference is bound to dispatch before sealing; an unchanged result.task\u005fname cannot substitute it.",
        "current\u005fevidence": "FIXED: benchmark/process.rs:1643-1654 compares observed task.path with the exact dispatched tasks[0].path. remediation\u005fp19\u005fobserved\u005ftask\u005fsubstitution\u005frejected and remediation\u005fp19\u005fsubstituted\u005ftask\u005freference\u005fnever\u005fseals passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found observed config.task.path could substitute another task package.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-004",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain substituted absolute package-path rejection with otherwise plausible lifecycle/result fields."
      },
      {
        "cause": "Admitted preserved locks own repo/commit authority. Current run-root equivalence and early unchecked trial-ID reads are different missed seams.",
        "current\u005fevidence": "FIXED for that authority path: verify-local-acceptance.py:222-350 binds one material per family and preserved admitted lock bytes to sealed upstream identity;402-441 requires unique preserved inventories;844-899 makes cache corroborative. Coordinated-substitution, empty-material, empty-preserved, duplicate-path and tamper tests passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found coordinated source/cache substitutions and empty material/preserved inventories could pass acceptance.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local-acceptance and delegation suites passed.",
            "limitations": "Two output-control test methods produced three errors; the overall Python check is not a pass.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/python-linux.log",
            "sha256": "9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain coordinated source/cache, empty inventory, cross-family lock and duplicate path negative cases."
      },
      {
        "cause": "The required-control pre-effect gate is repaired. The optional public Effective projection remains a separate inconsistency documented by current011.",
        "current\u005fevidence": "FIXED for required controls: experiment.rs:1023-1043 rejects required reasoning before native admission; native\u005fdriver::required\u005freasoning\u005frejects\u005fbefore\u005fnative\u005feffects and current\u005frequired\u005freasoning\u005frejects\u005fbefore\u005fnative\u005fadmission passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found required unsupported reasoning=high allowed native effects before a non-comparable result.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain supplied required reasoning values and assert zero native effects before refusal."
      },
      {
        "cause": "Frozen plan carries the admitted absolute material identity, avoiding plan-directory reinterpretation.",
        "current\u005fevidence": "FIXED: runner/plan.rs:539-557 canonicalizes material before loading, hashing and freezing. native\u005fdriver::current\u005frelative\u005fmaterial\u005fsurvives\u005fnested\u005ffrozen\u005fplan passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found relative native material changed its base when a frozen plan was written to a nested directory.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-007",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain relative input plus nested plan output, then execute from another working directory."
      },
      {
        "cause": "The cited duplicate producer was removed. A different default external-lock branch survives as current007; closure of one branch does not close the whole CI policy requirement.",
        "current\u005fevidence": "FIXED for the named native-smoke branch: native-smoke.sh:1024-1039 delegates current public opi-eval prepare and consumes emitted family materials. Shared-prepare delegation and negative missing-delegation/duplicate-transform tests passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found native-smoke materialize-configs independently transformed tasks and generated verifier/material policy.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local-acceptance and delegation suites passed.",
            "limitations": "Two output-control test methods produced three errors; the overall Python check is not a pass.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/python-linux.log",
            "sha256": "9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain shared-prepare delegation plus negative reintroduction checks; inspect the default external-lock path separately."
      },
      {
        "cause": "The exact walkthrough and English/Chinese guidance were synchronized. Current stale Rust module/run documentation is a separate missed documentation surface.",
        "current\u005fevidence": "FIXED: examples/local-evaluation/README.md:156-158 supplies --native-material; README.md:540-559,622 and README.zh.md:461-475,532 describe shared public preparation/delegation and regeneration. Documentation check passed in the audit.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "The prior report found the local walkthrough plan omitted native material and bilingual main READMEs retained CI-owned materialization guidance.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-009",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [
          "daaf2cfb540cb9890d930973cff725802a742a93"
        ],
        "verification": "Retain documented material admission and English/Chinese ownership parity; check Rust API prose separately."
      },
      {
        "cause": "New raw proof closes bounded client/cancellation evidence gaps without converting ordinary stand-ins or controlled\u005fmaterial diagnostic answer.txt graders into pinned benchmark-native evidence. Some detailed temporary artifacts are absent, so exact auth-value/shared-cleanup replay remains limited. No runtime regression is inferred.",
        "current\u005fevidence": "PARTIALLY SUPPLEMENTED: raw actual-client matrix, provider-failure and Docker cancellation logs each contain one executed pass; raw build/694-file source equivalence makes them applicable now. Six preserved native trials establish actual three-family lifecycle/handoff. REMAINING current006: controlled pinned correct/incorrect grader outputs for all three families and successful fresh pinned offline regrade were not located; current all-targets run still ignores them. Located c011 regrade stdout has rejected task-package drift and trials=[], not a successful witness.",
        "current\u005ffinding\u005fid": "P19-AUD-006",
        "prior\u005fclaim": "Prior010 lacked executed actual-client/Docker/shared-cleanup proof; prior011 lacked pinned native offline regrade; prior012 lacked pinned lifecycle and positive/negative witnesses. These were evidence limitations, not reproduced runtime failures.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-010",
            "source\u005fid": "prior-findings"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-011",
            "source\u005fid": "prior-findings"
          },
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-012",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          },
          {
            "checked": "Six attributable Opi/pi trials across TB2.1, TB3 and DeepSWE. Offline current Companion replay accepted:true; native/tool/terminal events and digest bindings were checked.",
            "limitations": "No new provider calls. This proves actual native trials, not controlled correct/incorrect graders or successful fresh offline regrade. No explicit pre-run usage/time budget was located.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/manifest.json",
            "sha256": "9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce"
          },
          {
            "checked": "Fresh current Companion offline verification accepted all six preserved native trials.",
            "limitations": "Verifier acceptance cannot supply a missing operator budget freeze or eliminate current verifier boundary findings.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/acceptance-current.json",
            "sha256": "83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06"
          },
          {
            "checked": "Retained Opi executable hash, raw build identity and canonical source manifest checked; all 694 relevant current source paths match, with no extra or missing paths.",
            "limitations": "Source/build applicability is established; it does not recreate absent temporary per-case detail files.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/build-applicability.json",
            "sha256": "857e5b492345b798b879e3664bb7f356cfaaf64297768d4169e4f5b2332fe6f5"
          },
          {
            "checked": "Raw executable/source identity followed from the acceptance build pointer and checked against retained bytes.",
            "limitations": "Credited as raw identity evidence, not a historical conclusion.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/clients/opi/build-identity.json",
            "sha256": "48409ba1dfb4c74a4ec8c1c1dad153bb5343518d0d2c5122d212cac042b7501a"
          },
          {
            "checked": "Canonical tree digest 5e2f82209a0573de8f3e92970a56274992f231495e9b4c0d54f97a497f6e1d54 matches current relevant source.",
            "limitations": "The audit/report metadata added afterward is outside this runtime-source projection.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/source-final-manifest.json",
            "sha256": "8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53"
          },
          {
            "checked": "Exact-client protocol matrix: 1 passed, 0 failed, 0 ignored.",
            "limitations": "Applicable to unchanged current source via raw identity/manifest checks. Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared-cleanup case artifacts were not independently replayed.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native\u005fprotocol\u005fclient\u005fmatrix.log",
            "sha256": "02aeb9b006c72fc55815ff09830f47da83c30c0a1cdafba834db2c9347d392ff"
          },
          {
            "checked": "Actual Docker owned-resource cancellation: 1 passed, 0 failed, 0 ignored.",
            "limitations": "Applicable to unchanged current source via raw identity/manifest checks. Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared-cleanup case artifacts were not independently replayed.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native\u005fowned\u005fresource\u005fcancellation.log",
            "sha256": "b5482323a6defe3c1890cc95b032f8902e189ff1287afbbae1f9578712939d49"
          },
          {
            "checked": "Native conformance/shared-cleanup entry point: 1 passed, 0 failed, 0 ignored.",
            "limitations": "Applicable to unchanged current source via raw identity/manifest checks. Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared-cleanup case artifacts were not independently replayed.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native\u005fconformance\u005fowned\u005fcleanup\u005fwitness.log",
            "sha256": "79752570d04a252d31d2a571dc1f17def443abc89c802da783fa97974ebae455"
          },
          {
            "checked": "Actual client retry/auth/rate/protocol/native-zero cases: 1 passed, 0 failed, 0 ignored.",
            "limitations": "Applicable to unchanged current source via raw identity/manifest checks. Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared-cleanup case artifacts were not independently replayed.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/final-native\u005fclient\u005fprovider\u005ffailure\u005freport.log",
            "sha256": "4d6965d60d0cb21b67bafe0d2b0e399e103881f636e6a11e5aa1fb60d08f9929"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Run and retain three pinned positive/negative native witnesses and successful verifier-only regrade of preserved pinned artifacts, with zero Agent/provider calls; retain detailed exact-client and cleanup captures."
      },
      {
        "cause": "Actual execution and build applicability are credited. A successful verifier replay does not prove an explicit prior operator budget freeze; this is missing acceptance proof, not fabricated trials or missing build identity.",
        "current\u005fevidence": "SIX-TRIAL PORTION SUPPLEMENTED: current offline replay accepted all six attributable trials, with real tool counts TB2.1 Opi/pi4/4, TB3 78/95, DeepSWE92/100 and one AgentEnd each. Raw material/task/build/control digests and exact current source applicability were checked. Current005 retains a different acceptance gap: all six frozen execution time/token/output bounds are null and no explicit operator pre-run usage/time-budget record was located; max\u005ftrials2/concurrency1 and model/profile defaults do not establish that record.",
        "current\u005ffinding\u005fid": "P19-AUD-005",
        "prior\u005fclaim": "Prior013 could not assess six real trials or final Phase-exit evidence from admitted input; it did not claim the runs never occurred.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Six attributable Opi/pi trials across TB2.1, TB3 and DeepSWE. Offline current Companion replay accepted:true; native/tool/terminal events and digest bindings were checked.",
            "limitations": "No new provider calls. This proves actual native trials, not controlled correct/incorrect graders or successful fresh offline regrade. No explicit pre-run usage/time budget was located.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root/manifest.json",
            "sha256": "9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce"
          },
          {
            "checked": "Fresh current Companion offline verification accepted all six preserved native trials.",
            "limitations": "Verifier acceptance cannot supply a missing operator budget freeze or eliminate current verifier boundary findings.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/acceptance-current.json",
            "sha256": "83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06"
          },
          {
            "checked": "Retained Opi executable hash, raw build identity and canonical source manifest checked; all 694 relevant current source paths match, with no extra or missing paths.",
            "limitations": "Source/build applicability is established; it does not recreate absent temporary per-case detail files.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/build-applicability.json",
            "sha256": "857e5b492345b798b879e3664bb7f356cfaaf64297768d4169e4f5b2332fe6f5"
          },
          {
            "checked": "Raw executable/source identity followed from the acceptance build pointer and checked against retained bytes.",
            "limitations": "Credited as raw identity evidence, not a historical conclusion.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/clients/opi/build-identity.json",
            "sha256": "48409ba1dfb4c74a4ec8c1c1dad153bb5343518d0d2c5122d212cac042b7501a"
          },
          {
            "checked": "Canonical tree digest 5e2f82209a0573de8f3e92970a56274992f231495e9b4c0d54f97a497f6e1d54 matches current relevant source.",
            "limitations": "The audit/report metadata added afterward is outside this runtime-source projection.",
            "path": "C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-apply-20260929/b11/source-final-manifest.json",
            "sha256": "8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Retain a timestamped pre-run operator freeze binding total usage/time scope to tasks, exact builds/materials, controls and concurrency, then validate its applicability to the acceptance run."
      },
      {
        "cause": "Current008 is an independently reproduced formatter failure. Old missing full-gate evidence does not establish a historical passing baseline; no regression claim is made.",
        "current\u005fevidence": "STILL NOT CLOSED: fresh current cargo fmt --check --all fails at native\u005fdriver.rs:10054-10059. Companion clippy,369 deterministic tests, doc tests and rustdoc passed, but these are narrower than workspace clippy/test/doc gates and cannot replace formatting or the complete exit union.",
        "current\u005ffinding\u005fid": "P19-AUD-008",
        "prior\u005fclaim": "Prior013 also lacked the full workspace Phase-exit verification union.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export cargo fmt --check --all exits1 with one committed assertion wrapping diff.",
            "limitations": "No source was formatted by the audit or this documentation maintenance.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/fmt.log",
            "sha256": "6265747c0da3d95201a205e608d8df84ae0ca6d276145a12e6f9dff437dab42b"
          },
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "After fixing formatting, retain the complete applicable workspace Phase-exit gate outputs for the final relevant source identity."
      },
      {
        "cause": "A different default CI branch remains outside the native-smoke repair. Shared downstream validation and pinned transport do not remove its independent policy. This is branch coverage, not proof that the repaired native-smoke path regressed.",
        "current\u005fevidence": "Current007: .github/workflows/opi-eval-external-lock-materialization.yml:58-68 calls materialize-external-locks.sh without prepare mode. Its42-420 shared-prepare mode exits at423; default425-1000 still separately acquires/materializes and invokes Harbor oracle grading.",
        "current\u005ffinding\u005fid": "P19-AUD-007",
        "prior\u005fclaim": "Prior008 specifically identified duplicate native-smoke materialization, repaired above.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-008",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local-acceptance and delegation suites passed.",
            "limitations": "Two output-control test methods produced three errors; the overall Python check is not a pass.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/python-linux.log",
            "sha256": "9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Trace every default workflow caller to the shared prepare/grading policy and add a discriminating duplicate-policy refusal check."
      },
      {
        "cause": "Required-control and final comparison gates protect native strict comparison. Public projection remained outside the repair and exposes contradictory capability truth; current severity stays Minor.",
        "current\u005fevidence": "Current011: experiment.rs:1193 still projects any requested optional reasoning Value as Effective, while experiment.rs:1017 rejects required reasoning and runner/experiment.rs:2582 settles supplied reasoning Unsupported; pinned Pi launches --thinking off.",
        "current\u005ffinding\u005fid": "P19-AUD-011",
        "prior\u005fclaim": "Prior006 objected to required unsupported reasoning entering native execution; that effect gate is repaired.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-006",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current sealed export: cargo test -p opi-eval --all-targets --locked; 369 passed, 0 failed, 11 ignored. Named repair tests below executed and passed.",
            "limitations": "Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/rust-tests.log",
            "sha256": "20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f"
          }
        ],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Verify public optional projection matches admitted native capability and settlement, while retaining required-control refusal before effects."
      },
      {
        "cause": "The trust-authority repair did not establish semantic run-root equivalence or safe early trial-ID reads. The attempted complete invalid-shadow acceptance was NOT established: its extra schema field was ignored and direct Companion also accepted. No end-to-end invalid-shadow bypass is claimed.",
        "current\u005fevidence": "Current004: verify-local-acceptance.py:925-931 verifies root/runs/family while earlier native/material/capture checks use resolved manifest roots;168-195 appends unchecked trial IDs before later sealing checks. Harmless structure-only external-canary probe establishes an early read gap.",
        "current\u005ffinding\u005fid": "P19-AUD-004",
        "prior\u005fclaim": "Prior005 addressed source/cache authority and empty inventories, repaired above.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-005",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Reject unsafe IDs before any read, bind all checked paths to one admitted canonical root and discriminate a truly invalid alternative bundle."
      },
      {
        "cause": "Container0700, preflight0600, cleanup and seal scanning cover other stages; they do not protect the original host file during its lifetime. No dynamic secret probe or Windows ACL claim is made.",
        "current\u005fevidence": "Current002: runner/experiment.rs:723,1560-1585 creates host directories and writes secret-bearing config/handoff using default permissions. Under an ordinary Unix umask022 and publicly searchable output root, original host secret files lack private permissions.",
        "current\u005ffinding\u005fid": "P19-AUD-002",
        "prior\u005fclaim": "Prior001 concerned selected secrets promoted into sealed captures; automatic selected-key scan fixes that exact route.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Use a dummy credential with explicit Unix umask022 and a searchable root; assert private creation before write and cleanup on every outcome."
      },
      {
        "cause": "Selected-key/canary rejection is preserved, but it does not classify reasoning. This is a newly identified content-classification omission; no known repaired reasoning boundary or regression baseline exists.",
        "current\u005fevidence": "Current003: bundle Sensitivity forbids Exportable private raw reasoning, but runner/experiment.rs:3495-3510 always stages Exportable. Metadata-only checks of actual digest-bound native captures found thinking objects, including a terminal client\u005fmessage; no private reasoning text is quoted or copied.",
        "current\u005ffinding\u005fid": "P19-AUD-003",
        "prior\u005fclaim": "Prior001 identified selected credential leakage. It did not establish privacy classification for client thinking content.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Verify private thinking content is excluded from export or classified LocalOnly, including terminal-only capture mode."
      },
      {
        "cause": "The current test-seam failure is reproduced, but no applicable old execution of this exact producer/fixture proves a pass-to-fail transition. Production may use installed uv from PATH; this does not prove production acquisition failure.",
        "current\u005fevidence": "Current009: Linux Python discovery251 tests,3 errors,1 skipped; test\u005fnative\u005fsmoke\u005foutput\u005fcontrol.py:81 returns nonexistent fixed /usr/bin/uv. Two test methods fail before their ordering/refusal assertions; a second subcase encounters a partially staged static directory.",
        "current\u005ffinding\u005fid": "P19-AUD-009",
        "prior\u005fclaim": "Prior013 recorded missing final-gate evidence without establishing this exact output-control fixture result.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local-acceptance and delegation suites passed.",
            "limitations": "Two output-control test methods produced three errors; the overall Python check is not a pass.",
            "path": "C:/Users/Luiz/AppData/Local/Temp/opi-p19-audit-8ozcu3oo/python-linux.log",
            "sha256": "9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79"
          }
        ],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Use a real temporary synthetic executable and isolated staging for each subcase, then execute both affected methods and the Python suite."
      },
      {
        "cause": "The README repair left Rust API prose outside its scope. Passing rustdoc/doc-check confirms build/link structure, not behavioral truth; runtime native behavior is credited.",
        "current\u005fevidence": "Current010: runner/experiment.rs:13 and comment near1167 still describe automatic oracle preflight; cli/run.rs:10 describes fixture-only execution even though normal native1212 selects real integration and integrity oracle=None.",
        "current\u005ffinding\u005fid": "P19-AUD-010",
        "prior\u005fclaim": "Prior009 identified stale walkthrough and bilingual README directions, repaired above.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
            "finding\u005fid": "P19-AUD-009",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Update both Rust module contracts to current native and explicit diagnostic-oracle behavior and check generated documentation."
      },
      {
        "cause": "This is a newly identified missing enforcement consumer. History supplies no repaired matching path or proven regression baseline; the independent current Major finding remains unchanged.",
        "current\u005fevidence": "Current001: accepted positive max\u005foutput\u005fbytes is persisted in plan metadata but has no production runtime consumer. Fixed profile/importer64MiB limits cannot enforce a smaller explicitly requested cap.",
        "current\u005ffinding\u005fid": "P19-AUD-001",
        "prior\u005fclaim": "No matching specific defect was identified among the prior report's thirteen findings.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Exercise a deliberately small requested cap and assert enforcement or refusal before native effects, with the same effective bound in the sealed controls."
      }
    ],
    "limitations": "Historical finding numbers are scoped by owning audit\u005frun\u005fid; identical current numbers do not denote identical defects. Exact old001-009 paths are repaired. Old010-013 evidence is partially supplemented, with remaining pinned controlled-grader/regrade and Phase-exit gaps. Current core, all three sidecars, requirement identities, severities and FAIL verdict stay byte-identical. No persistent-defect or regression label is asserted without evidence of the same affected path and an applicable prior fixed baseline. Raw proof is credited only within the stated source/build applicability and detailed-artifact limitations; no paid provider or Docker/native task was rerun. This maintenance does not repair product code, close sidecar findings, change the implementation ledger or satisfy the full Phase-exit gate union.",
    "schema": "opi-audit-history/1",
    "scope": "Explicit operator-requested post-install maintenance compares only the thirteen findings of phase19-codex-gpt6-3e8fee6-20260928t050232z (codex/gpt6; audit head 3e8fee6075c719ca0c3192790b800c13421e4fc4) with the byte-preserved codex/gpt61 core at 2465aeb80ad84d06172acc142f6d5ffbceb201cc. The old four-file member is archived at docs/snapshots/phase19/assurance/history/phase19-codex-gpt6-3e8fee6-20260928t050232z/. Historical source references retain their original committed paths and exact-byte commits. The named older history directory phase19-codex-gpt6-e079780-20260926t093335z is removed at the operator's explicit request and is not a comparison source. This explicitly requested cross-model comparison replaces the earlier no-history advisory; it does not resume independent audit work.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
        "commit": "ac802811f2f0216ceddc9d802906570d9f4d67e8",
        "id": "prior-report",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt6.md",
        "sha256": "4925287b3cf391be8784b2cf1dfe73c6b72daed0c90704ccf140428fbbd3fc52"
      },
      {
        "audit\u005frun\u005fid": "phase19-codex-gpt6-3e8fee6-20260928t050232z",
        "commit": "ac802811f2f0216ceddc9d802906570d9f4d67e8",
        "id": "prior-findings",
        "path": "docs/snapshots/phase19/assurance/audit.codex.gpt6.findings.jsonl",
        "sha256": "ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "90d9a494d59824cb99dfcf749cf6b0c30e7fa6eb79959b7e406fa2eeea64b5a1",
    "meta\u005fsha256": "d67dc8f6ade4b8516624518e905e8b62d0788854eb1350a7b3373e46e8c3881d",
    "report\u005fbytes": 66707,
    "report\u005fsha256": "f87efb09af2144209004e4652b2c5fb9219e37ed96ba18f06211634fd27ef654",
    "requirements\u005fsha256": "7a6bbb95a72c38ae15bf5b4bca6acf07970cbb1c956673309165b3aa27e2a5af"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase19&#45;codex&#45;gpt61&#45;2465aeb&#45;20261001t055252z
- Audit head: 2465aeb80ad84d06172acc142f6d5ffbceb201cc
- Core report bytes: 66707
- Core report SHA-256: f87efb09af2144209004e4652b2c5fb9219e37ed96ba18f06211634fd27ef654
- Current findings SHA-256: 90d9a494d59824cb99dfcf749cf6b0c30e7fa6eb79959b7e406fa2eeea64b5a1
- Comparison status: compared
- Inspected history: Explicit operator&#45;requested post&#45;install maintenance compares only the thirteen findings of phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z &#40;codex/gpt6; audit head 3e8fee6075c719ca0c3192790b800c13421e4fc4&#41; with the byte&#45;preserved codex/gpt61 core at 2465aeb80ad84d06172acc142f6d5ffbceb201cc&#46; The old four&#45;file member is archived at docs/snapshots/phase19/assurance/history/phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z/&#46; Historical source references retain their original committed paths and exact&#45;byte commits&#46; The named older history directory phase19&#45;codex&#45;gpt6&#45;e079780&#45;20260926t093335z is removed at the operator&#39;s explicit request and is not a comparison source&#46; This explicitly requested cross&#45;model comparison replaces the earlier no&#45;history advisory; it does not resume independent audit work&#46;
- Limitations: Historical finding numbers are scoped by owning audit&#95;run&#95;id; identical current numbers do not denote identical defects&#46; Exact old001&#45;009 paths are repaired&#46; Old010&#45;013 evidence is partially supplemented, with remaining pinned controlled&#45;grader/regrade and Phase&#45;exit gaps&#46; Current core, all three sidecars, requirement identities, severities and FAIL verdict stay byte&#45;identical&#46; No persistent&#45;defect or regression label is asserted without evidence of the same affected path and an applicable prior fixed baseline&#46; Raw proof is credited only within the stated source/build applicability and detailed&#45;artifact limitations; no paid provider or Docker/native task was rerun&#46; This maintenance does not repair product code, close sidecar findings, change the implementation ledger or satisfy the full Phase&#45;exit gate union&#46;

### Pinned historical sources

- prior&#45;report: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt6&#46;md; commit ac802811f2f0216ceddc9d802906570d9f4d67e8; SHA-256 4925287b3cf391be8784b2cf1dfe73c6b72daed0c90704ccf140428fbbd3fc52; audit run phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z
- prior&#45;findings: docs/snapshots/phase19/assurance/audit&#46;codex&#46;gpt6&#46;findings&#46;jsonl; commit ac802811f2f0216ceddc9d802906570d9f4d67e8; SHA-256 ccf574271b308b2bb72e13df1721824a9bda153e08b813e0679320b65ceb4ca1; audit run phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z

### Comparison 1: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found selected credentials could be echoed into Exportable captures and sealed when &#45;&#45;canaries was omitted&#46;
- Current evidence: FIXED at current audit head: runner/experiment&#46;rs:1865&#45;1886 automatically enrolls the resolved selected env/file credential in private seal&#95;scan&#95;secrets; the seal scan at2545 consumes it&#46; native&#95;driver::remediation&#95;p19&#95;selected&#95;credential&#95;echo&#95;is&#95;never&#95;sealed passed&#46;
- Cause / missed boundary: Automatic selected&#45;key enrollment closes the specific opt&#45;in&#45;only seal guard&#46; Current host&#45;file permissions and private&#45;thinking classification are separate boundaries, not recurrence of this exact hole&#46;
- Suggested current verification: Retain the no&#45;canaries selected&#45;credential echo refusal test for both credential sources&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;001; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 2: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found replacement reexecuted non&#45;target groups under old trial IDs&#46;
- Current evidence: FIXED: runner/experiment&#46;rs:3967 refreshes every rerun trial ID before target&#45;group naming changes&#46; assembled&#95;run::replacement&#95;fresh&#95;resolved&#95;pairs&#95;preserve&#95;original&#95;scope passed and now checks fresh identities for reexecuted unrelated groups&#46;
- Cause / missed boundary: Same&#45;scope replacement remains allowed; every real reexecution now receives a fresh trial identity&#46;
- Suggested current verification: Retain multi&#45;group replacement checks covering all reexecuted IDs and immutable prior bundles&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;002; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 3: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found material/5 did not bind installed runner interpreter and console&#45;entrypoint bytes&#46;
- Current evidence: FIXED: material/6 carries interpreter/entrypoint PinnedFile values; runner/material&#46;rs:133&#45;176 checks expected paths and verify&#95;pinned byte identities&#46; native&#95;driver::remediation&#95;p19&#95;runner&#95;installed&#95;bytes&#95;are&#95;reverified passed&#46;
- Cause / missed boundary: Installed executable bytes are now admitted by the durable material loader, rather than being preparation&#45;only inventory&#46;
- Suggested current verification: Retain interpreter/entrypoint byte&#45;tamper rejection before plan/run admission&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;003; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 4: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found observed config&#46;task&#46;path could substitute another task package&#46;
- Current evidence: FIXED: benchmark/process&#46;rs:1643&#45;1654 compares observed task&#46;path with the exact dispatched tasks&#91;0&#93;&#46;path&#46; remediation&#95;p19&#95;observed&#95;task&#95;substitution&#95;rejected and remediation&#95;p19&#95;substituted&#95;task&#95;reference&#95;never&#95;seals passed&#46;
- Cause / missed boundary: Observed package reference is bound to dispatch before sealing; an unchanged result&#46;task&#95;name cannot substitute it&#46;
- Suggested current verification: Retain substituted absolute package&#45;path rejection with otherwise plausible lifecycle/result fields&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;004; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 5: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found coordinated source/cache substitutions and empty material/preserved inventories could pass acceptance&#46;
- Current evidence: FIXED for that authority path: verify&#45;local&#45;acceptance&#46;py:222&#45;350 binds one material per family and preserved admitted lock bytes to sealed upstream identity;402&#45;441 requires unique preserved inventories;844&#45;899 makes cache corroborative&#46; Coordinated&#45;substitution, empty&#45;material, empty&#45;preserved, duplicate&#45;path and tamper tests passed&#46;
- Cause / missed boundary: Admitted preserved locks own repo/commit authority&#46; Current run&#45;root equivalence and early unchecked trial&#45;ID reads are different missed seams&#46;
- Suggested current verification: Retain coordinated source/cache, empty inventory, cross&#45;family lock and duplicate path negative cases&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;005; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/python&#45;linux&#46;log; SHA-256 9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79; checked: Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local&#45;acceptance and delegation suites passed&#46;; limits: Two output&#45;control test methods produced three errors; the overall Python check is not a pass&#46;

### Comparison 6: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found required unsupported reasoning=high allowed native effects before a non&#45;comparable result&#46;
- Current evidence: FIXED for required controls: experiment&#46;rs:1023&#45;1043 rejects required reasoning before native admission; native&#95;driver::required&#95;reasoning&#95;rejects&#95;before&#95;native&#95;effects and current&#95;required&#95;reasoning&#95;rejects&#95;before&#95;native&#95;admission passed&#46;
- Cause / missed boundary: The required&#45;control pre&#45;effect gate is repaired&#46; The optional public Effective projection remains a separate inconsistency documented by current011&#46;
- Suggested current verification: Retain supplied required reasoning values and assert zero native effects before refusal&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;006; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 7: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found relative native material changed its base when a frozen plan was written to a nested directory&#46;
- Current evidence: FIXED: runner/plan&#46;rs:539&#45;557 canonicalizes material before loading, hashing and freezing&#46; native&#95;driver::current&#95;relative&#95;material&#95;survives&#95;nested&#95;frozen&#95;plan passed&#46;
- Cause / missed boundary: Frozen plan carries the admitted absolute material identity, avoiding plan&#45;directory reinterpretation&#46;
- Suggested current verification: Retain relative input plus nested plan output, then execute from another working directory&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;007; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 8: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found native&#45;smoke materialize&#45;configs independently transformed tasks and generated verifier/material policy&#46;
- Current evidence: FIXED for the named native&#45;smoke branch: native&#45;smoke&#46;sh:1024&#45;1039 delegates current public opi&#45;eval prepare and consumes emitted family materials&#46; Shared&#45;prepare delegation and negative missing&#45;delegation/duplicate&#45;transform tests passed&#46;
- Cause / missed boundary: The cited duplicate producer was removed&#46; A different default external&#45;lock branch survives as current007; closure of one branch does not close the whole CI policy requirement&#46;
- Suggested current verification: Retain shared&#45;prepare delegation plus negative reintroduction checks; inspect the default external&#45;lock path separately&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;008; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/python&#45;linux&#46;log; SHA-256 9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79; checked: Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local&#45;acceptance and delegation suites passed&#46;; limits: Two output&#45;control test methods produced three errors; the overall Python check is not a pass&#46;

### Comparison 9: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): The prior report found the local walkthrough plan omitted native material and bilingual main READMEs retained CI&#45;owned materialization guidance&#46;
- Current evidence: FIXED: examples/local&#45;evaluation/README&#46;md:156&#45;158 supplies &#45;&#45;native&#45;material; README&#46;md:540&#45;559,622 and README&#46;zh&#46;md:461&#45;475,532 describe shared public preparation/delegation and regeneration&#46; Documentation check passed in the audit&#46;
- Cause / missed boundary: The exact walkthrough and English/Chinese guidance were synchronized&#46; Current stale Rust module/run documentation is a separate missed documentation surface&#46;
- Suggested current verification: Retain documented material admission and English/Chinese ownership parity; check Rust API prose separately&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;009; source prior&#45;findings
- Repair commits: daaf2cfb540cb9890d930973cff725802a742a93

### Comparison 10: P19&#45;AUD&#45;006

- Relationship: evidence&#45;gap
- Prior claim (attributed): Prior010 lacked executed actual&#45;client/Docker/shared&#45;cleanup proof; prior011 lacked pinned native offline regrade; prior012 lacked pinned lifecycle and positive/negative witnesses&#46; These were evidence limitations, not reproduced runtime failures&#46;
- Current evidence: PARTIALLY SUPPLEMENTED: raw actual&#45;client matrix, provider&#45;failure and Docker cancellation logs each contain one executed pass; raw build/694&#45;file source equivalence makes them applicable now&#46; Six preserved native trials establish actual three&#45;family lifecycle/handoff&#46; REMAINING current006: controlled pinned correct/incorrect grader outputs for all three families and successful fresh pinned offline regrade were not located; current all&#45;targets run still ignores them&#46; Located c011 regrade stdout has rejected task&#45;package drift and trials=&#91;&#93;, not a successful witness&#46;
- Cause / missed boundary: New raw proof closes bounded client/cancellation evidence gaps without converting ordinary stand&#45;ins or controlled&#95;material diagnostic answer&#46;txt graders into pinned benchmark&#45;native evidence&#46; Some detailed temporary artifacts are absent, so exact auth&#45;value/shared&#45;cleanup replay remains limited&#46; No runtime regression is inferred&#46;
- Suggested current verification: Run and retain three pinned positive/negative native witnesses and successful verifier&#45;only regrade of preserved pinned artifacts, with zero Agent/provider calls; retain detailed exact&#45;client and cleanup captures&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;010; source prior&#45;findings
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;011; source prior&#45;findings
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;012; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;acceptance&#45;20260929/evidence&#45;root/manifest&#46;json; SHA-256 9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce; checked: Six attributable Opi/pi trials across TB2&#46;1, TB3 and DeepSWE&#46; Offline current Companion replay accepted:true; native/tool/terminal events and digest bindings were checked&#46;; limits: No new provider calls&#46; This proves actual native trials, not controlled correct/incorrect graders or successful fresh offline regrade&#46; No explicit pre&#45;run usage/time budget was located&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/acceptance&#45;current&#46;json; SHA-256 83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06; checked: Fresh current Companion offline verification accepted all six preserved native trials&#46;; limits: Verifier acceptance cannot supply a missing operator budget freeze or eliminate current verifier boundary findings&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/build&#45;applicability&#46;json; SHA-256 857e5b492345b798b879e3664bb7f356cfaaf64297768d4169e4f5b2332fe6f5; checked: Retained Opi executable hash, raw build identity and canonical source manifest checked; all 694 relevant current source paths match, with no extra or missing paths&#46;; limits: Source/build applicability is established; it does not recreate absent temporary per&#45;case detail files&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/clients/opi/build&#45;identity&#46;json; SHA-256 48409ba1dfb4c74a4ec8c1c1dad153bb5343518d0d2c5122d212cac042b7501a; checked: Raw executable/source identity followed from the acceptance build pointer and checked against retained bytes&#46;; limits: Credited as raw identity evidence, not a historical conclusion&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/source&#45;final&#45;manifest&#46;json; SHA-256 8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53; checked: Canonical tree digest 5e2f82209a0573de8f3e92970a56274992f231495e9b4c0d54f97a497f6e1d54 matches current relevant source&#46;; limits: The audit/report metadata added afterward is outside this runtime&#45;source projection&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/final&#45;native&#95;protocol&#95;client&#95;matrix&#46;log; SHA-256 02aeb9b006c72fc55815ff09830f47da83c30c0a1cdafba834db2c9347d392ff; checked: Exact&#45;client protocol matrix: 1 passed, 0 failed, 0 ignored&#46;; limits: Applicable to unchanged current source via raw identity/manifest checks&#46; Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared&#45;cleanup case artifacts were not independently replayed&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/final&#45;native&#95;owned&#95;resource&#95;cancellation&#46;log; SHA-256 b5482323a6defe3c1890cc95b032f8902e189ff1287afbbae1f9578712939d49; checked: Actual Docker owned&#45;resource cancellation: 1 passed, 0 failed, 0 ignored&#46;; limits: Applicable to unchanged current source via raw identity/manifest checks&#46; Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared&#45;cleanup case artifacts were not independently replayed&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/final&#45;native&#95;conformance&#95;owned&#95;cleanup&#95;witness&#46;log; SHA-256 79752570d04a252d31d2a571dc1f17def443abc89c802da783fa97974ebae455; checked: Native conformance/shared&#45;cleanup entry point: 1 passed, 0 failed, 0 ignored&#46;; limits: Applicable to unchanged current source via raw identity/manifest checks&#46; Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared&#45;cleanup case artifacts were not independently replayed&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/final&#45;native&#95;client&#95;provider&#95;failure&#95;report&#46;log; SHA-256 4d6965d60d0cb21b67bafe0d2b0e399e103881f636e6a11e5aa1fb60d08f9929; checked: Actual client retry/auth/rate/protocol/native&#45;zero cases: 1 passed, 0 failed, 0 ignored&#46;; limits: Applicable to unchanged current source via raw identity/manifest checks&#46; Referenced /tmp evidence roots are absent; detailed request IDs, exact dummy auth value and all shared&#45;cleanup case artifacts were not independently replayed&#46;

### Comparison 11: P19&#45;AUD&#45;005

- Relationship: evidence&#45;gap
- Prior claim (attributed): Prior013 could not assess six real trials or final Phase&#45;exit evidence from admitted input; it did not claim the runs never occurred&#46;
- Current evidence: SIX&#45;TRIAL PORTION SUPPLEMENTED: current offline replay accepted all six attributable trials, with real tool counts TB2&#46;1 Opi/pi4/4, TB3 78/95, DeepSWE92/100 and one AgentEnd each&#46; Raw material/task/build/control digests and exact current source applicability were checked&#46; Current005 retains a different acceptance gap: all six frozen execution time/token/output bounds are null and no explicit operator pre&#45;run usage/time&#45;budget record was located; max&#95;trials2/concurrency1 and model/profile defaults do not establish that record&#46;
- Cause / missed boundary: Actual execution and build applicability are credited&#46; A successful verifier replay does not prove an explicit prior operator budget freeze; this is missing acceptance proof, not fabricated trials or missing build identity&#46;
- Suggested current verification: Retain a timestamped pre&#45;run operator freeze binding total usage/time scope to tasks, exact builds/materials, controls and concurrency, then validate its applicability to the acceptance run&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;013; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;acceptance&#45;20260929/evidence&#45;root/manifest&#46;json; SHA-256 9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce; checked: Six attributable Opi/pi trials across TB2&#46;1, TB3 and DeepSWE&#46; Offline current Companion replay accepted:true; native/tool/terminal events and digest bindings were checked&#46;; limits: No new provider calls&#46; This proves actual native trials, not controlled correct/incorrect graders or successful fresh offline regrade&#46; No explicit pre&#45;run usage/time budget was located&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/acceptance&#45;current&#46;json; SHA-256 83e921d9c83bf4737bed0b959438d4789512ce789727d9546f2bb4a728db6c06; checked: Fresh current Companion offline verification accepted all six preserved native trials&#46;; limits: Verifier acceptance cannot supply a missing operator budget freeze or eliminate current verifier boundary findings&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/build&#45;applicability&#46;json; SHA-256 857e5b492345b798b879e3664bb7f356cfaaf64297768d4169e4f5b2332fe6f5; checked: Retained Opi executable hash, raw build identity and canonical source manifest checked; all 694 relevant current source paths match, with no extra or missing paths&#46;; limits: Source/build applicability is established; it does not recreate absent temporary per&#45;case detail files&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/clients/opi/build&#45;identity&#46;json; SHA-256 48409ba1dfb4c74a4ec8c1c1dad153bb5343518d0d2c5122d212cac042b7501a; checked: Raw executable/source identity followed from the acceptance build pointer and checked against retained bytes&#46;; limits: Credited as raw identity evidence, not a historical conclusion&#46;
- Candidate raw proof: C:/Users/Luiz/&#46;local/state/opi&#45;remediation&#45;evidence/phase19&#45;apply&#45;20260929/b11/source&#45;final&#45;manifest&#46;json; SHA-256 8bd27e06e0690f5248031dfdabd91c78bc7418633a954c43801fd796d894ed53; checked: Canonical tree digest 5e2f82209a0573de8f3e92970a56274992f231495e9b4c0d54f97a497f6e1d54 matches current relevant source&#46;; limits: The audit/report metadata added afterward is outside this runtime&#45;source projection&#46;

### Comparison 12: P19&#45;AUD&#45;008

- Relationship: evidence&#45;gap
- Prior claim (attributed): Prior013 also lacked the full workspace Phase&#45;exit verification union&#46;
- Current evidence: STILL NOT CLOSED: fresh current cargo fmt &#45;&#45;check &#45;&#45;all fails at native&#95;driver&#46;rs:10054&#45;10059&#46; Companion clippy,369 deterministic tests, doc tests and rustdoc passed, but these are narrower than workspace clippy/test/doc gates and cannot replace formatting or the complete exit union&#46;
- Cause / missed boundary: Current008 is an independently reproduced formatter failure&#46; Old missing full&#45;gate evidence does not establish a historical passing baseline; no regression claim is made&#46;
- Suggested current verification: After fixing formatting, retain the complete applicable workspace Phase&#45;exit gate outputs for the final relevant source identity&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;013; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/fmt&#46;log; SHA-256 6265747c0da3d95201a205e608d8df84ae0ca6d276145a12e6f9dff437dab42b; checked: Current sealed export cargo fmt &#45;&#45;check &#45;&#45;all exits1 with one committed assertion wrapping diff&#46;; limits: No source was formatted by the audit or this documentation maintenance&#46;
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 13: P19&#45;AUD&#45;007

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior008 specifically identified duplicate native&#45;smoke materialization, repaired above&#46;
- Current evidence: Current007: &#46;github/workflows/opi&#45;eval&#45;external&#45;lock&#45;materialization&#46;yml:58&#45;68 calls materialize&#45;external&#45;locks&#46;sh without prepare mode&#46; Its42&#45;420 shared&#45;prepare mode exits at423; default425&#45;1000 still separately acquires/materializes and invokes Harbor oracle grading&#46;
- Cause / missed boundary: A different default CI branch remains outside the native&#45;smoke repair&#46; Shared downstream validation and pinned transport do not remove its independent policy&#46; This is branch coverage, not proof that the repaired native&#45;smoke path regressed&#46;
- Suggested current verification: Trace every default workflow caller to the shared prepare/grading policy and add a discriminating duplicate&#45;policy refusal check&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;008; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/python&#45;linux&#46;log; SHA-256 9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79; checked: Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local&#45;acceptance and delegation suites passed&#46;; limits: Two output&#45;control test methods produced three errors; the overall Python check is not a pass&#46;

### Comparison 14: P19&#45;AUD&#45;011

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior006 objected to required unsupported reasoning entering native execution; that effect gate is repaired&#46;
- Current evidence: Current011: experiment&#46;rs:1193 still projects any requested optional reasoning Value as Effective, while experiment&#46;rs:1017 rejects required reasoning and runner/experiment&#46;rs:2582 settles supplied reasoning Unsupported; pinned Pi launches &#45;&#45;thinking off&#46;
- Cause / missed boundary: Required&#45;control and final comparison gates protect native strict comparison&#46; Public projection remained outside the repair and exposes contradictory capability truth; current severity stays Minor&#46;
- Suggested current verification: Verify public optional projection matches admitted native capability and settlement, while retaining required&#45;control refusal before effects&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;006; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/rust&#45;tests&#46;log; SHA-256 20097a23b23dc74cf6efe3ef2cc5fc9ac9d0d674dfdd20650a4c7a0cf764931f; checked: Current sealed export: cargo test &#45;p opi&#45;eval &#45;&#45;all&#45;targets &#45;&#45;locked; 369 passed, 0 failed, 11 ignored&#46; Named repair tests below executed and passed&#46;; limits: Deterministic tests establish the specific repaired paths; they do not execute ignored native witnesses or full workspace gates&#46;

### Comparison 15: P19&#45;AUD&#45;004

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior005 addressed source/cache authority and empty inventories, repaired above&#46;
- Current evidence: Current004: verify&#45;local&#45;acceptance&#46;py:925&#45;931 verifies root/runs/family while earlier native/material/capture checks use resolved manifest roots;168&#45;195 appends unchecked trial IDs before later sealing checks&#46; Harmless structure&#45;only external&#45;canary probe establishes an early read gap&#46;
- Cause / missed boundary: The trust&#45;authority repair did not establish semantic run&#45;root equivalence or safe early trial&#45;ID reads&#46; The attempted complete invalid&#45;shadow acceptance was NOT established: its extra schema field was ignored and direct Companion also accepted&#46; No end&#45;to&#45;end invalid&#45;shadow bypass is claimed&#46;
- Suggested current verification: Reject unsafe IDs before any read, bind all checked paths to one admitted canonical root and discriminate a truly invalid alternative bundle&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;005; source prior&#45;findings

### Comparison 16: P19&#45;AUD&#45;002

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior001 concerned selected secrets promoted into sealed captures; automatic selected&#45;key scan fixes that exact route&#46;
- Current evidence: Current002: runner/experiment&#46;rs:723,1560&#45;1585 creates host directories and writes secret&#45;bearing config/handoff using default permissions&#46; Under an ordinary Unix umask022 and publicly searchable output root, original host secret files lack private permissions&#46;
- Cause / missed boundary: Container0700, preflight0600, cleanup and seal scanning cover other stages; they do not protect the original host file during its lifetime&#46; No dynamic secret probe or Windows ACL claim is made&#46;
- Suggested current verification: Use a dummy credential with explicit Unix umask022 and a searchable root; assert private creation before write and cleanup on every outcome&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;001; source prior&#45;findings

### Comparison 17: P19&#45;AUD&#45;003

- Relationship: new&#45;omission
- Prior claim (attributed): Prior001 identified selected credential leakage&#46; It did not establish privacy classification for client thinking content&#46;
- Current evidence: Current003: bundle Sensitivity forbids Exportable private raw reasoning, but runner/experiment&#46;rs:3495&#45;3510 always stages Exportable&#46; Metadata&#45;only checks of actual digest&#45;bound native captures found thinking objects, including a terminal client&#95;message; no private reasoning text is quoted or copied&#46;
- Cause / missed boundary: Selected&#45;key/canary rejection is preserved, but it does not classify reasoning&#46; This is a newly identified content&#45;classification omission; no known repaired reasoning boundary or regression baseline exists&#46;
- Suggested current verification: Verify private thinking content is excluded from export or classified LocalOnly, including terminal&#45;only capture mode&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;001; source prior&#45;findings

### Comparison 18: P19&#45;AUD&#45;009

- Relationship: insufficient&#45;information
- Prior claim (attributed): Prior013 recorded missing final&#45;gate evidence without establishing this exact output&#45;control fixture result&#46;
- Current evidence: Current009: Linux Python discovery251 tests,3 errors,1 skipped; test&#95;native&#95;smoke&#95;output&#95;control&#46;py:81 returns nonexistent fixed /usr/bin/uv&#46; Two test methods fail before their ordering/refusal assertions; a second subcase encounters a partially staged static directory&#46;
- Cause / missed boundary: The current test&#45;seam failure is reproduced, but no applicable old execution of this exact producer/fixture proves a pass&#45;to&#45;fail transition&#46; Production may use installed uv from PATH; this does not prove production acquisition failure&#46;
- Suggested current verification: Use a real temporary synthetic executable and isolated staging for each subcase, then execute both affected methods and the Python suite&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;013; source prior&#45;findings
- Candidate raw proof: C:/Users/Luiz/AppData/Local/Temp/opi&#45;p19&#45;audit&#45;8ozcu3oo/python&#45;linux&#46;log; SHA-256 9f8365ce4bef13615be384050121e49876d9b6689cab44c52cca4b3a22349c79; checked: Current Linux Python discovery: 251 tests, 3 errors, 1 skipped; local&#45;acceptance and delegation suites passed&#46;; limits: Two output&#45;control test methods produced three errors; the overall Python check is not a pass&#46;

### Comparison 19: P19&#45;AUD&#45;010

- Relationship: missed&#45;branch
- Prior claim (attributed): Prior009 identified stale walkthrough and bilingual README directions, repaired above&#46;
- Current evidence: Current010: runner/experiment&#46;rs:13 and comment near1167 still describe automatic oracle preflight; cli/run&#46;rs:10 describes fixture&#45;only execution even though normal native1212 selects real integration and integrity oracle=None&#46;
- Cause / missed boundary: The README repair left Rust API prose outside its scope&#46; Passing rustdoc/doc&#45;check confirms build/link structure, not behavioral truth; runtime native behavior is credited&#46;
- Suggested current verification: Update both Rust module contracts to current native and explicit diagnostic&#45;oracle behavior and check generated documentation&#46;
- Prior finding: phase19&#45;codex&#45;gpt6&#45;3e8fee6&#45;20260928t050232z / P19&#45;AUD&#45;009; source prior&#45;findings

### Comparison 20: P19&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): No matching specific defect was identified among the prior report&#39;s thirteen findings&#46;
- Current evidence: Current001: accepted positive max&#95;output&#95;bytes is persisted in plan metadata but has no production runtime consumer&#46; Fixed profile/importer64MiB limits cannot enforce a smaller explicitly requested cap&#46;
- Cause / missed boundary: This is a newly identified missing enforcement consumer&#46; History supplies no repaired matching path or proven regression baseline; the independent current Major finding remains unchanged&#46;
- Suggested current verification: Exercise a deliberately small requested cap and assert enforcement or refusal before native effects, with the same effective bound in the sealed controls&#46;
