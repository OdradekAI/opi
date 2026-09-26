# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt6-e079780-20260926t093335z`
**Audit head**: `e079780bda6681ec466f79bd15f3248baf0f2eb8`
**Reviewer ID**: `codex`
**Model ID**: `gpt6`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt6`
**Model identity source**: operator-declared
**Independence**: unknown — 本轮未读取历史 audit/remediation 或其他 peer 结论；实现者及模型家族关系未获得可验证证明。
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

本报告仅审查封存提交。审计前固定了 53 项要求，随后只填入证据与判定；没有修改生产源码、测试、实现账本或规范。用户明确授权重新取得六次真实 trial，使用 E:\opi 和仓库 .env 的凭据引用。gpt6 是用户声明的审计模型身份，不是被评估 Agent 的服务模型。

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | `c65c10635c4bb8b1754625b94e8a0a15459eef6d65634610ba9b65ea9ba5337e` | committed bytes; stored hash mismatch: none |
| docs/snapshots/phase19/opi-impl-state.json | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | committed bytes; stored hash mismatch: none |
| docs/opi-spec.md | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | committed bytes; stored hash mismatch: none |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | committed bytes; stored hash mismatch: none |
| docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | committed bytes; stored hash mismatch: none |

完整原始来源字节由 scope reader 计算摘要；账本只读 allowlist projection。封存要求原稿 SHA-256: `4b793cea2fe930698b2d16d5010292987db5384b1d3dc2e089edd4601434b89a`。

## Requirement Conformance

每项的完整生产链、判别性测试和精确命令保存在同名 requirements.jsonl。本表只索引当前证据；默认通过的 stand-in 测试没有被当作真实原生运行。

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P19-AC-002 | Section 7; task 19.1 / P19-AC-002 | 328 passed, 9 ignored; Production CLI uses one bound native job; substitution, absent terminal, invalid grading and failed capture are discriminated. Native zero and task timeout remain scored; provider/identity/infra are unscored. Stand-ins establish binding/settlement, not real native fidelity. | met | — |
| P19-AC-003-native | Section 7; task 19.1 / P19-AC-003-native | Fresh exact-argv confirmation PASS; no model; native zero from nop. | met | — |
| P19-AC-008-outcome | Section 7; task 19.1 / P19-AC-008-outcome | 328 passed, 9 ignored; Production CLI uses one bound native job; substitution, absent terminal, invalid grading and failed capture are discriminated. Native zero and task timeout remain scored; provider/identity/infra are unscored. Stand-ins establish binding/settlement, not real native fidelity. | partially-met | P19-AUD-008 |
| P19-AC-006-fixture | Section 7; task 19.2 / P19-AC-006-fixture | PASS: actual Opi/pi clients completed the three-protocol matrix and distinct auth, rate-limit, protocol and retry cases (30 cases, 199.81 s); direct client conformance only, not public run/report failure acceptance. | met | — |
| P19-AC-010-credentials | Section 7; task 19.2 / P19-AC-010-credentials | 328 passed, 9 ignored; Missing credentials and required unsupported controls reject before effects; dummy canary leakage blocks seal and subsequent dispatch. Credential staging is removed at settlement and before native shared verification. Python integration suite passed (228 tests total, 1 Docker-dependent skip). | met | — |
| P19-AC-004-native | Section 7; task 19.3 / P19-AC-004-native | PASS: actual pinned Harbor, oracle and nop, distinct declared artifact digests and native rewards 1/0; 459.46 s. No model call. | met | — |
| P19-AC-005-native | Section 7; task 19.4 / P19-AC-005-native | PASS: actual pinned Pier, oracle and nop, distinct committed patch digests and native rewards 1/0, pristine separate verifier; 828.01 s. No model call. | met | — |
| P19-AC-001-prepare | Section 7; task 19.5 / P19-AC-001-prepare | PASS for TB2.1, TB3 and DeepSWE after explicit POSIX permission restoration and stable-path Harbor environment materialization; all three validate/plan commands also exit 0. Initial permission/path rejections are retained and are not counted as passes. | met | — |
| P19-AC-007-material | Section 7; task 19.5 / P19-AC-007-material | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-AC-007-selection | Section 7; task 19.6 / P19-AC-007-selection | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-AC-001-plan | Section 7; task 19.6 / P19-AC-001-plan | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-AC-008 | Section 7; task 19.7 / P19-AC-008 | 328 passed, 9 ignored; Reports verify seals and use the settled interpretation, retain unstarted/unsealed diagnostic coverage, separate family denominators and missing/control-mismatched pairs. The full-selection selected count is separately reproduced as incorrect. | partially-met | P19-AUD-005, P19-AUD-007 |
| P19-AC-009-verify | Section 7; task 19.7 / P19-AC-009-verify | 328 passed, 9 ignored; Covered-file mutation is rejected without rewriting source, and reports are deterministic. A symlink replacing a whole trial bypasses verify enumeration; see current finding. | partially-met | P19-AUD-003 |
| P19-AC-009-regrade | Section 7; task 19.8 / P19-AC-009-regrade | 328 passed, 9 ignored; Fresh verifier-only request binds source artifact/base/image and derives new records; source bytes and credentials are not reused for Agent execution. Paired stand-in cases pass; pi-only material fails before dispatch. Actual offline native witness is tracked separately. | partially-met | P19-AUD-004 |
| P19-AC-010-lifecycle | Section 7; task 19.9 / P19-AC-010-lifecycle | 328 passed, 9 ignored; Signal cancellation and unknown command termination stop dispatch; owned resource labels/full IDs are checked and shared images retained. Recovery is read-only and replacement uses fresh paired identities. Ordinary integrity/cleanup failures fail to cancel subsequent dispatch. | partially-met | P19-AUD-002 |
| P19-AC-011 | Section 7; task 19.10 / P19-AC-011 | 328 passed, 9 ignored; Unpublished crate has no Opi crate dependency or product activation. Explicit live mode keeps defaults credential-free; CI calls shared producer/verifier paths. Bilingual documentation and metadata doc check pass; pi-only regrade exception is linked separately. | met | — |
| P19-AC-013 | Section 7; task 19.10 / P19-AC-013 | 328 passed, 9 ignored; Unpublished crate has no Opi crate dependency or product activation. Explicit live mode keeps defaults credential-free; CI calls shared producer/verifier paths. Bilingual documentation and metadata doc check pass; pi-only regrade exception is linked separately. | partially-met | P19-AUD-004 |
| P19-AC-001-live | Section 7; task 19.11 / P19-AC-001-live | Six fresh real client tool loops and six native results retained; each family verify/report exited 0. Unmodified acceptance checker exited 2, accepted=false: agent evidence: 'trial-baseline-pi-portable-abs-module-cache-flags-r1' terminal, stream, or native model attribution differs; independent full-capture probe also proves Opi ApiKind mismatch. | partially-met | P19-AUD-010 |
| P19-AC-003-live | Section 7; task 19.11 / P19-AC-003-live | Two newly executed real clients, normal process exit and native grading observed. pi: responses=8, paired_tools=7, native=known:1(native-trial-result), raw_bytes=757683, truncated=False, report_class=graded-native; opi: responses=6, paired_tools=7, native=known:1(native-trial-result), raw_bytes=2269104, truncated=True, report_class=graded-native. Original offline acceptance rejects the sealed evidence; this row is not a conformance pass. | partially-met | P19-AUD-009, P19-AUD-010 |
| P19-AC-004-live | Section 7; task 19.11 / P19-AC-004-live | Two newly executed real clients, normal process exit and native grading observed. pi: responses=15, paired_tools=28, native=known:0(native-trial-result), raw_bytes=2002525, truncated=True, report_class=invalid-execution; opi: responses=11, paired_tools=24, native=known:0(native-trial-result), raw_bytes=2942536, truncated=True, report_class=graded-native. Original offline acceptance rejects the sealed evidence; this row is not a conformance pass. | partially-met | P19-AUD-009, P19-AUD-010 |
| P19-AC-005-live | Section 7; task 19.11 / P19-AC-005-live | Two newly executed real clients, normal process exit and native grading observed. pi: responses=7, paired_tools=12, native=known:0(native-trial-result), raw_bytes=1587986, truncated=True, report_class=invalid-execution; opi: responses=14, paired_tools=13, native=known:0(native-trial-result), raw_bytes=2996723, truncated=True, report_class=graded-native. Original offline acceptance rejects the sealed evidence; this row is not a conformance pass. | partially-met | P19-AUD-009, P19-AUD-010 |
| P19-AC-006-live | Section 7; task 19.11 / P19-AC-006-live | Six fresh real client tool loops and six native results retained; each family verify/report exited 0. Unmodified acceptance checker exited 2, accepted=false: agent evidence: 'trial-baseline-pi-portable-abs-module-cache-flags-r1' terminal, stream, or native model attribution differs; independent full-capture probe also proves Opi ApiKind mismatch. | partially-met | P19-AUD-009 |
| P19-AC-012 | Section 7; task 19.11 / P19-AC-012 | Six fresh real client tool loops and six native results retained; each family verify/report exited 0. Unmodified acceptance checker exited 2, accepted=false: agent evidence: 'trial-baseline-pi-portable-abs-module-cache-flags-r1' terminal, stream, or native model attribution differs; independent full-capture probe also proves Opi ApiKind mismatch. | partially-met | P19-AUD-006, P19-AUD-009, P19-AUD-010 |
| P19-001-doctor | Section 4 / task 19.5 | PASS: Linux Docker reachable; x86_64, CPU=32, engine RAM=16611233792 bytes, GPU unknown with reason, local disk and missing cold Agent material reported; no model. | met | — |
| P19-001-acquire | Section 4-5 / task 19.5 | PASS for TB2.1, TB3 and DeepSWE after explicit POSIX permission restoration and stable-path Harbor environment materialization; all three validate/plan commands also exit 0. Initial permission/path rejections are retained and are not counted as passes. | met | — |
| P19-002-settlement | Section 5-6 / task 19.1 | 328 passed, 9 ignored; Production CLI uses one bound native job; substitution, absent terminal, invalid grading and failed capture are discriminated. Native zero and task timeout remain scored; provider/identity/infra are unscored. Stand-ins establish binding/settlement, not real native fidelity. | partially-met | P19-AUD-008 |
| P19-002-failed-capture | Section 6 / task 19.1 | 328 passed, 9 ignored; Production CLI uses one bound native job; substitution, absent terminal, invalid grading and failed capture are discriminated. Native zero and task timeout remain scored; provider/identity/infra are unscored. Stand-ins establish binding/settlement, not real native fidelity. | met | — |
| P19-003-no-legacy-route | Section 5 / task 19.1 | 328 passed, 9 ignored; Production CLI uses one bound native job; substitution, absent terminal, invalid grading and failed capture are discriminated. Native zero and task timeout remain scored; provider/identity/infra are unscored. Stand-ins establish binding/settlement, not real native fidelity. | met | — |
| P19-004-declarations | Section 5 / task 19.3 | 328 passed, 9 ignored; Declared original-path collection and patch/base identities are inspected and rejected on substitution; deterministic upstream shapes exercise the production importer. | met | — |
| P19-005-patch | Section 5 / task 19.4 | 328 passed, 9 ignored; Declared original-path collection and patch/base identities are inspected and rejected on substitution; deterministic upstream shapes exercise the production importer. | met | — |
| P19-006-config | Section 4 / task 19.2 | 328 passed, 9 ignored; Selected API/model/endpoint and explicit live mode drive projection. Evidence pin, wire limits and required-control rejection are discriminating; unsupported sampling, service model and prices remain explicitly unobserved. File-credential exception is linked separately. | partially-met | P19-AUD-001 |
| P19-006-controls | Section 4 / task 19.2 | 328 passed, 9 ignored; Selected API/model/endpoint and explicit live mode drive projection. Evidence pin, wire limits and required-control rejection are discriminating; unsupported sampling, service model and prices remain explicitly unobserved. File-credential exception is linked separately.; PASS for original exact clients and again for the rebuilt musl Opi with the same pi; initial and tool-continuation wire requests carry max_output_tokens=4096. | met | — |
| P19-006-terminal | Section 4,6 / task 19.2 | PASS: actual Opi/pi clients completed the three-protocol matrix and distinct auth, rate-limit, protocol and retry cases (30 cases, 199.81 s); direct client conformance only, not public run/report failure acceptance. | met | — |
| P19-006-unknown | Section 4,6 | 328 passed, 9 ignored; Selected API/model/endpoint and explicit live mode drive projection. Evidence pin, wire limits and required-control rejection are discriminating; unsupported sampling, service model and prices remain explicitly unobserved. File-credential exception is linked separately. | met | — |
| P19-007-inventory | Section 5 / task 19.6 | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-007-plan | Section 4-5 / task 19.6 | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-007-bounds | Section 4 / tasks 19.2,19.6,19.9 | 328 passed, 9 ignored; Plan/material digest drift, unknown or unmet required resources, duplicate aliases, subset boundaries and unenforceable cost/token caps reject. Full inventory stays visible, stage caps narrow task deadlines. Native stand-ins are used by default; real preparation/plan observations supplement below. | met | — |
| P19-008-coverage | Section 6 / task 19.7 | 328 passed, 9 ignored; Reports verify seals and use the settled interpretation, retain unstarted/unsealed diagnostic coverage, separate family denominators and missing/control-mismatched pairs. The full-selection selected count is separately reproduced as incorrect. | met | — |
| P19-008-metrics | Section 6 / task 19.7 | 328 passed, 9 ignored; Reports verify seals and use the settled interpretation, retain unstarted/unsealed diagnostic coverage, separate family denominators and missing/control-mismatched pairs. The full-selection selected count is separately reproduced as incorrect. | partially-met | P19-AUD-005, P19-AUD-008 |
| P19-008-pairs | Section 6 / task 19.7 | 328 passed, 9 ignored; Reports verify seals and use the settled interpretation, retain unstarted/unsealed diagnostic coverage, separate family denominators and missing/control-mismatched pairs. The full-selection selected count is separately reproduced as incorrect. | met | — |
| P19-009-derived | Section 6 / task 19.8 | 328 passed, 9 ignored; Fresh verifier-only request binds source artifact/base/image and derives new records; source bytes and credentials are not reused for Agent execution. Paired stand-in cases pass; pi-only material fails before dispatch. Actual offline native witness is tracked separately. | partially-met | P19-AUD-004 |
| P19-010-isolation | Section 4 / tasks 19.2,19.9 | 328 passed, 9 ignored; Missing credentials and required unsupported controls reject before effects; dummy canary leakage blocks seal and subsequent dispatch. Credential staging is removed at settlement and before native shared verification. Python integration suite passed (228 tests total, 1 Docker-dependent skip). | met | — |
| P19-010-cancel | Section 9 / task 19.9 | 328 passed, 9 ignored; Signal cancellation and unknown command termination stop dispatch; owned resource labels/full IDs are checked and shared images retained. Recovery is read-only and replacement uses fresh paired identities. Ordinary integrity/cleanup failures fail to cancel subsequent dispatch. | partially-met | P19-AUD-002 |
| P19-010-recovery | Section 4,6 / task 19.9 | 328 passed, 9 ignored; Signal cancellation and unknown command termination stop dispatch; owned resource labels/full IDs are checked and shared images retained. Recovery is read-only and replacement uses fresh paired identities. Ordinary integrity/cleanup failures fail to cancel subsequent dispatch. | met | — |
| P19-011-ci-policy | Section 8 / task 19.10 | 328 passed, 9 ignored; Unpublished crate has no Opi crate dependency or product activation. Explicit live mode keeps defaults credential-free; CI calls shared producer/verifier paths. Bilingual documentation and metadata doc check pass; pi-only regrade exception is linked separately. | met | — |
| P19-012-offline-verifier | Section 8 / tasks 19.10-19.11 | 328 passed, 9 ignored; Offline verifier checks six products/families, sealed first connection, tool starts/ends, exact-client output wire proof, immutable capture view and repository-qualified cached source identities. Unit positives are synthetic, with malformed/missing/substitution negatives; real acceptance remains a separate obligation.; Six fresh real client tool loops and six native results retained; each family verify/report exited 0. Unmodified acceptance checker exited 2, accepted=false: agent evidence: 'trial-baseline-pi-portable-abs-module-cache-flags-r1' terminal, stream, or native model attribution differs; independent full-capture probe also proves Opi ApiKind mismatch. | partially-met | P19-AUD-009 |
| P19-012-provenance | Section 8 / tasks 19.10-19.11 | 328 passed, 9 ignored; Offline verifier checks six products/families, sealed first connection, tool starts/ends, exact-client output wire proof, immutable capture view and repository-qualified cached source identities. Unit positives are synthetic, with malformed/missing/substitution negatives; real acceptance remains a separate obligation. | met | — |
| P19-012-final-gates | Section 8 / task 19.11 |  | partially-met | P19-AUD-006 |
| P19-013-placement | Section 3 / P19-AC-013 | 328 passed, 9 ignored; Unpublished crate has no Opi crate dependency or product activation. Explicit live mode keeps defaults credential-free; CI calls shared producer/verifier paths. Bilingual documentation and metadata doc check pass; pi-only regrade exception is linked separately. | met | — |
| P19-013-contracts | Section 9 / P19-AC-013 | 328 passed, 9 ignored; Unpublished crate has no Opi crate dependency or product activation. Explicit live mode keeps defaults credential-free; CI calls shared producer/verifier paths. Bilingual documentation and metadata doc check pass; pi-only regrade exception is linked separately. | met | — |
| P19-OPTIONAL-ORACLE | Section 5 | 328 passed, 9 ignored; Oracle is a named conformance diagnostic rejected as Agent attribution. Optional real OpenAI endpoints are not exercised or claimed; mandatory native-client loopback matrix has its own row. | met | — |
| P19-OPTIONAL-CONNECTIONS | Section 4 | 328 passed, 9 ignored; Oracle is a named conformance diagnostic rejected as Agent attribution. Optional real OpenAI endpoints are not exercised or claimed; mandatory native-client loopback matrix has its own row. | met | — |
| P19-ARCH-MINIMUM | PRIN-001, PRIN-002, PRIN-003, PLACE-001, PLACE-004 | 328 passed, 9 ignored; All 11 task reuse/surface/ceiling notes were checked against current consumers. Score authority remains in shared settlement/binding, mechanisms stay private except same-package CLI/experiment APIs; no new Agent Core seam. Behavioral defects are linked at their owning obligation rather than inferred historical intent. | met | — |

## Standards Review

当前 opi-eval 为 unpublished Independent Companion，Cargo 不依赖 Opi 内部 crate；Agent Core 没有评测策略入口。CLI/experiment 的可见性服务同包二进制，native material、binding、settlement 和 lifecycle 留在私有机制层。直接审查了 CLI、持久格式、Harbor/Pier Python 适配、脚本与双语 README 的对应消费者。格式、Clippy、文档检查、doctest 和 rustdoc 均已运行；完整 workspace test 的失败单独记为 P19-AUD-006。

依赖与锁文件没有修改。生产 Rust/Python 测试没有增加或更改，test impact: **none**；独立探针及运行脚本均位于仓库之外。报告按 assurance 四文件契约输出，没有独立中英 counterpart。

## Spec Review

结算从真实 native terminal、任务/产物身份、评分与生命周期导出一个结果；valid zero 与 invalid execution 在默认生产入口测试中得到区分。三个家族的 native handoff 与离线 verify/report/regrade 都经过调用链追踪。当前可复现的缺口集中在文件凭据投影、失败后调度、verify 枚举、单产品 regrade、full 覆盖统计、真实客户端失败矩阵的组装证据、加载器故障归类、客户端流封存、离线验收器 API 形状及最终门禁，见对应 finding。

真实 trial 是本次重新采集的单任务双产品 smoke，不能据此声称统计优势或完整 benchmark 完成。输出上限来自同一原生客户端二进制的本地 wire 观测；未观测到的服务端实际模型、价格及采样效果不被配置值冒充。

## Security, Invariants, Integration, Test Quality, and Residuals

- Security：credential staging/canary/seal admission 与缺失凭据前置拒绝有反例测试；符号链接替换整个 trial 的枚举绕过被独立复现。
- Invariants：signal/effect-unknown/privacy 分支会停止后续调度，但普通 identity/cleanup 拒绝没有同样的运行级取消效果。
- Integration：Env 凭据和双产品材料的正常路径通过，分别不能反驳有效 File 凭据和 pi-only regrade 的失败。
- Test quality：默认 9 个 ignored native 测试不计入默认通过数；实际执行的 ignored filters 单独列出。反证探针通过触发失败断言揭示缺口，不作为绿色测试呈现。
- Residuals：仍保留明确的 hermetic fixture 路径和显式 oracle conformance；它们不是旧 host-agent/default-oracle 评分路径。没有以 TODO 搜索结果替代生产链审查。

## Minimum-change Conformance

下表记录当前消费者及机制边界；net_deletion 指当前已不存在的旧行为路线或重复机制，不推断历史删行数量。全部 11 项均核对其登记的 reuse_search、surface_necessity 与 simplification_ceiling。

| Task | Reuse / surface / ceiling | production_consumers | nonproduction_consumers | net_deletion | residual_glue |
|---|---|---|---|---|---|
| 19.1 | conforming / conforming / conforming | run_trial; Opi/Pi settle; three benchmark adapters; sealed ReportBuilder | native_driver; benchmark_integration_conformance; authority_boundaries | native run 不再走 host Agent + 独立默认/oracle grader；单一结算拥有者 | 现有 family adapter、native binding 与 lifecycle |
| 19.2 | conforming / conforming / conforming | connection resolver; Opi/pi config projection; terminal importer | native protocol/output-control matrix | 无额外 Eval LLM client 或隐式 endpoint fallback | 两个原生客户端配置形状；File 分支缺陷见 001 |
| 19.3 | conforming / conforming / conforming | TB3 adapter; shared Harbor lifecycle; declared collection admission | TB3 artifact witness; substitution negatives | 共享 19.1 importer，不另造 score authority | TB3 task-declared collection/network 规则 |
| 19.4 | conforming / conforming / conforming | DeepSwe adapter; Pier lifecycle; patch/base binding | DeepSWE witness; base drift/uncommitted negatives | 复用 settlement；不引入第二套 patch scorer | pristine verifier 的必要家族差异 |
| 19.5 | conforming / conforming / conforming | doctor; prepare; material loader; pinned acquisition producer | prepare Python tests; native independent-directory acquisition | CI 与本地不各有一套下载策略 | 固定平台 producer 与 cache provenance |
| 19.6 | conforming / conforming / conforming | plan; existing experiment resolver; material inventory; run | tampered/frozen plan; bounds/resource tests | 无新 scheduler/hosted-worker seam | full/subset 的冻结表示；统计缺陷见 005 |
| 19.7 | conforming / conforming / conforming | ReportBuilder; sealed interpretation; comparison | bundle_recompute; end_to_end_report; coverage tests | 不由 report 再猜测 Agent outcome | 显式旧 bundle 解读与缺失证据展示 |
| 19.8 | conforming / conforming / conforming | verify; NativeRegrader; saved handoff; native integration | offline regrade; tamper; source-preservation tests | 重用原生 verifier，不新增 Agent/provider 路径 | 派生记录及 source binding；单产品加载缺陷见 004 |
| 19.9 | conforming / conforming / conforming | existing supervisor; run/cancel/recover/replace | signal/cleanup/effect-unknown/replacement cases | 单一生命周期所有者，未增加通用调度平台 | cleanup/integrity 失败应触发统一取消；见 002 |
| 19.10 | conforming / conforming / conforming | thin native CI transport; producer; offline acceptance verifier | Python negative acceptance/provenance tests | CI 不拥有替代 runtime policy | 外部源身份按仓库分别验证 |
| 19.11 | conforming / conforming / conforming | explicit operator run; same Companion verify/report | six fresh attributable trials and acceptance validator | 无新增产品功能/激活/排行榜 | 只保留本次脱敏证据和审计结论 |

## Findings

### P19-AUD-001: Opi 丢弃 credential_file 解析出的凭据

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-006-config
- Claim: 当所选连接使用有效 credential_file 时，Opi 配置引用 OPI_EVAL_AGENT_KEY，但投影环境为空；实际 Agent 因而无法使用已经解析的文件凭据。
- Evidence: crates/opi-eval/src/experiment.rs:1135 — agent_env 仅在 ResolvedCredential.variable() 为 Some 时写入；FromFile 返回 None。; crates/opi-eval/src/experiment.rs:1263 — 文件路径分支生成 api_key_env = OPI_EVAL_AGENT_KEY，却未为该变量提供值。; crates/opi-eval/src/runner/experiment.rs:1234 — 运行入口支持解析 File 引用，然后将投影的 agent_env 写入原生 handoff。; audit probe audit_file_credential_projection — 匹配的 File/FromFile 输入得到 env_names=[]；应包含 OPI_EVAL_AGENT_KEY 的断言失败。
- Refutation attempted: projected_config_bytes_carry_the_exact_request_identity 验证环境引用及 pi 的配置内凭据，projection_rejects_credential_not_matching_the_selected_reference 验证引用不匹配；它们均未覆盖匹配的 Opi File 分支。Harbor/Pier 只消费 handoff.agent_env，没有补入该值的替代入口。
- Suggested closure: 让文件引用的解析值与 Opi 配置引用的私有环境变量一致，保留清理与不导出约束；以文件引用运行实际 Opi 的本地鉴权回环。

### P19-AUD-002: 完整性或清理失败后仍继续原生 trial 调度

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-lifecycle, P19-010-cancel
- Claim: 串行原生运行在 Agent 身份被替换或 owned-resource-cleanup-unconfirmed 后，仍启动下一个计划 trial；这些失败未触发运行级取消。
- Evidence: crates/opi-eval/src/runner/experiment.rs:769 — 下一批调度只检查 run_cancel；失败 receipt 不会自动取消。; crates/opi-eval/src/runner/experiment.rs:2012 — 清理失败记录 infrastructure failure，但不取消运行。; crates/opi-eval/src/runner/experiment.rs:2300 — 此处仅 canary_leak 设置 run_cancel，拒绝 seal 的其他边界失败不设置。; audit probes audit_cleanup_stops_dispatch / audit_provenance_stops_dispatch — 两次独立探针都观察到首个 trial settled=failed 后第二个 stage=start，trials=2；身份替换用例的第二个 trial 甚至成功封存。
- Refutation attempted: native_settlement_cleanup_uncertainty_blocks_success 确认受影响 trial 不封存；p19_plan_privacy_failure_stops_dispatch 与 native_unobserved_command_exit_stops_dispatch_and_sealing 确認隐私泄漏及 effect-unknown 特例会停止。它们没有覆盖清理不确定或普通身份绑定拒绝后的后续调度。离线 regrade 已有清理失败停止逻辑，也不保护 run。
- Suggested closure: 由统一结算所有者把规范 §9 的完整性/来源/清理边界失败传播到运行级取消，保留已发生效果和未启动覆盖；新增只发生一次 dispatch 的断言。

### P19-AUD-003: verify 将符号链接 trial 静默排除后仍返回成功

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-009-verify
- Claim: 将已经封存的 trial 目录替换为指向其原始字节的符号链接后，verify 不报告篡改，而是减少 bundles 数并继续返回 verified/exit 0。
- Evidence: crates/opi-eval/src/regrade.rs:113 — read_trial_ids 只接受 DirEntry.file_type().is_dir()，符号链接和其他非目录条目被静默丢弃。; crates/opi-eval/src/regrade.rs:96 — 空 failures 决定 verified，未与已有计划 inventory 校对。; audit probe audit_verify_trial_symlink — 同一个 hermetic CLI 运行篡改前有两个 verified bundles；篡改后只有一个，failures=[]，outcome=verified。
- Refutation attempted: RunBundle::verify 会拒绝 bundle 内链接和字节漂移，ReportBuilder 也有 checked_input_path 链接检查。但被 read_trial_ids 跳过的 trial 从未进入这些检查，report 的较强检查不能兑现独立 verify 的契约。
- Suggested closure: 在 trial 枚举边界拒绝链接和不支持条目，保留被篡改/缺失的计划 trial 的失败证据；验证命令应非零且不改写原始内容。

### P19-AUD-004: pi-only 原生运行无法使用自己的材料重评分

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-009-regrade, P19-009-derived, P19-AC-013
- Claim: 有效 pi-only run 可完成，但 regrade 用默认双产品材料加载器强制要求 opi 和 pi；只含实际所需 pi 的材料被拒绝。
- Evidence: crates/opi-eval/src/regrade.rs:312 — NativeRegrader 在读取原始 trial 需求前调用 NativeMaterial::load。; crates/opi-eval/src/runner/material.rs:257 — load 硬编码 load_for(path, &[opi, pi])；run 则按实际 subjects 使用 load_for。; audit probe audit_pi_only_regrade — 移除未选择 Opi subject/material 后，原生 stand-in run exit 0；同一 source/material 的 regrade exit 2，报 agent set must be exactly [opi, pi], found [pi]。
- Refutation attempted: pi_only_without_opi_runtime_and_packaging 只执行 hermetic run/report；native_offline_regrade 用双 Agent 材料。生产 run 的按需 load_for 能接受本用例，证明不是实验输入无效，但 regrade 没有采用该路径。
- Suggested closure: 重评分根据已验证源证据选择必要材料，pi-only 和 opi-only 均不要求无关产品；保留错误材料、来源漂移和离线约束的拒绝行为。

### P19-AUD-005: full 选择的离线报告将 selected 计为零

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008, P19-008-metrics
- Claim: 合法 scope=full 使用空 task_ids 表示完整语料；运行扩展并完成任务后，report 仍按原始空数组计算 selected，产生错误覆盖统计。
- Evidence: crates/opi-eval/src/runner/experiment.rs:705 — run-control 保存原始 selection.task_ids，而不是展开后的选中任务。; crates/opi-eval/src/report.rs:842 — coverage_counts.selected 直接取该原始数组长度。; audit probe audit_full_selection_report — 一个任务的 full 语料，两产品运行成功；离线 coverage_counts 为 corpus=1、selected=0、planned=2、attempted=2、valid=2。
- Refutation attempted: 现有 plan/run/report 端到端测试验证 scope=subset 和显式 task_ids 的 selected=2；完整语料的 plan 扩展也正确。这些反例仅说明执行和 subset 统计正确，无法保护 full 的不同表示。
- Suggested closure: 从经过绑定的展开计划或计划 trial 的唯一任务集合确定 selected；验证 full、subset、重复次数和未开始运行的统计一致。

### P19-AUD-006: 最终工作区测试门禁未通过

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-012-final-gates, P19-AC-012
- Claim: 在封存源码的 WSL/Linux 审计副本中，完整工作区测试未通过；bash_tool_env_inheritance_reporting 在独立精确重跑中仍失败，因此本次证据不能确认 Phase 19 所要求的最终工作区门禁。
- Evidence: crates/opi-coding-agent/tests/tools_read_write_edit_bash.rs:2121 — 生产 BashTool 执行 env 后 result 非错误，但返回文本为空，should have env output 断言失败；完整工作区和独立重跑均复现。; workspace-all.log — cargo test --offline --workspace --all-targets --no-fail-fast 汇总 4505 passed、3 failed、14 ignored；失败来自 artifact_audit_script、oauth_auth、tools_read_write_edit_bash。; commit-object-test.log / oauth-recheck.log / bash-env-recheck.log — Git 对象依赖测试在只读对象库可见后通过；OAuth 时限用例精确重跑通过；Bash 环境输出用例仍为 0 passed、1 failed。
- Refutation attempted: 格式、Clippy、12 个 doctest、rustdoc 和 328 个 opi-eval 默认测试均通过。两个附加失败分别有审计 export 环境解释或未在单例复现，但这些正向检查不能替代完整门禁，也不能反驳仍可复现的 Bash 用例失败。此 finding 不把尚未定位的根因归咎于 Phase 19 生产代码。
- Suggested closure: 定位并消除 Bash 环境输出测试失败，在具有所需只读 Git 对象的封存测试环境重跑完整工作区门禁；保留本次失败记录，不用单例通过改写原始完整命令结果。

### P19-AUD-007: 真实客户端失败矩阵尚未贯通公开 run-to-report 验收链

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008
- Claim: 任务 19.7 的 P19-AC-008 要求实际 Opi/pi 配合本地 fixture provider 经公开 run/report 保留最终 provider failure、native zero 和 transient 429 后的有效结果；当前真实客户端矩阵直接启动客户端，公开 run/report 测试则使用 stand-in。二者没有形成该项要求的同一次组装验收。
- Evidence: crates/opi-eval/tests/native_driver.rs:2748 — native_protocol_client_matrix 使用生产 project_connection，但 run_opi_client/run_pi_client 后仅检查进程、消息及 request log，没有调用 opi-eval run/report。此次精确 ignored filter 已通过。; crates/opi-eval/tests/native_driver.rs:5719 — native_provider_failure_report 的 run/report 断言确实区分 invalid 与 native zero，但输入来自 stage(provider-error/zero)。; crates/opi-eval/tests/native_driver.rs:669 — stage 写入 synthetic profile、stand-in integration 和 shell Agent/runner；没有实际 Opi/pi 请求本地 provider。后续三家族 terminal/report 矩阵也使用该 stage。; docs/snapshots/phase19/opi-impl-state.json#tasks/19.7/acceptance_scenarios/P19-AC-008 — 通过审计 scope projection 读取的当前验收要求明确指定 actual Opi/pi clients、local fixture provider 和 same public run-to-report path。
- Refutation attempted: 本轮真实协议矩阵证明两产品三协议的 auth/rate/protocol/retry 行为；默认公共 CLI 测试证明预制 terminal 事实的结算和报告。实际正向 live trial 也可证明正常装配，但三者都没有在同一次公开路径中观察真实客户端的这些失败及重试事实，因此不能替代任务 19.7 明确要求的组装证据。这是验收缺口，不是声称已发现相应生产映射错误。
- Suggested closure: 用固定实际 Opi/pi 与本地 fake provider，至少把一次最终 provider failure、一次有效零分及一次 429 后成功的结果贯通公开 run/report；断言封存解释和 denominator，并保持默认套件离线、无真实凭据。

### P19-AUD-008: Opi 加载器失败被计为有效原生零分

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-outcome, P19-002-settlement, P19-008-metrics
- Claim: 按当前构建路径取得的 GNU Opi 在官方 TB2.1 容器中因缺少 GLIBC_2.39 而尚未启动；Companion 仍把此 trial 归为 graded-native / reward=0 / agent_failed=true，污染有效评分分母。
- Evidence: crates/opi-eval/src/runner/experiment.rs:1659 — 普通非零退出统一变成 agent-non-zero-exit；没有区分尚未进入客户端的加载器/环境启动失败。; crates/opi-eval/src/runner/experiment.rs:1453 — 无 runner exception 的已绑定评分进入 GradedNative，非零 Agent completion 只设置 agent_failed。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-evidence/runs/terminal-bench-2.1/trials/trial-candidate-opi-fresh-openssl-selfsigned-cert-r1/bundle/artifacts/native/agent-stderr.log — 实际 stderr 明确报告 GLIBC_2.39 not found；stdout 为零字节，terminal.exit_code=1。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-evidence/runs/terminal-bench-2.1/run-report.json — 同一 trial sealed，classification.class=graded-native，reward=0，agent_failed=true。
- Refutation attempted: 合法 Agent 崩溃与有效超时确实应保留 native zero，默认分类测试也覆盖这些情况。但本次失败由动态加载器产生，客户端未执行、无模型/工具事件；同一源码的静态 musl 构建已在完全相同的任务容器中成功执行 --version，证实阻断位于运行环境兼容性而非任务求解。独立验收脚本会拒绝这份材料，但 report 的有效零分仍然错误。
- Suggested closure: 在实际任务环境验证所选运行包可启动，并将已观测的加载/启动故障归为不评分的执行无效；保持真正任务失败和有效超时的原生零分。增加该区别贯通公开 run/report 的反例。

### P19-AUD-009: 离线验收器把 Opi 原生 API 枚举误当作 pi 协议字符串

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-006-live, P19-AC-012, P19-012-offline-verifier, P19-AC-003-live, P19-AC-004-live, P19-AC-005-live
- Claim: 真实 Opi Anthropic 消息序列化 api 为 "Anthropic"；verify-local-acceptance.py 却对 Opi 和 pi 都要求 "anthropic-messages"。因此即便提供完整、未截断的真实 Opi 工具循环，该原始验收器仍拒绝正确首连接证据。
- Evidence: crates/opi-ai/src/lib.rs:62 — ApiKind 直接派生 Serialize，Anthropic 变体没有 rename；原生 JSON 使用 Anthropic。; crates/opi-eval/scripts/verify-local-acceptance.py:445 — 对两个产品逐条比较 FIRST_CONNECTION 的 provider/api/model，缺少 Opi ApiKind 到连接协议的明确解释。; crates/opi-eval/scripts/test_verify_local_acceptance.py:190 — 合成的 Opi/pi assistant message 均直接复制 CONNECTION[api]，没有保留实际 Opi 事件的原生形状。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/api-verifier-probe/result.json — 保持本轮真实 Opi stdout/stderr/native result 字节，单独构造只用于隔离截断因素的临时输入；未改原 seal。原始 check_agent_observations 返回 provider response differs from the first connection。
- Refutation attempted: 本轮 Opi 已以指定 zhipu:glm-5.3-flash 完成真实工具调用且原生评分为 1；配置投影和实际客户端的初次/续轮输出上限回环都通过。现有验收单测通过，但其合成 Opi 消息采用了 pi 的字符串。隔离探针使用完整原始流，排除了 1 MiB 截断这一独立原因；不是把错误 endpoint 或模型放宽成成功。
- Suggested closure: 按各自固定的原生序列化解释协议身份，并从实际客户端保留的消息形状构造判别性回归；继续拒绝确实不匹配的协议、provider、model 和缺少工具循环。重新执行未修改的六次材料验收。

### P19-AUD-010: 固定 1 MiB 封存上限使真实客户端循环无法完整进入验收材料

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-live, P19-AC-003-live, P19-AC-012, P19-AC-004-live, P19-AC-005-live
- Claim: 本轮普通 TB2.1 真实工具循环即超过 read_native_agent_capture 的固定 1 MiB 上限。full 捕获仍只封存前缀，验收器要求未截断并含结束事件，因此实际完成和已评分的 Opi/pi 尝试无法稳定生成可验收的原生流证据。
- Evidence: crates/opi-eval/src/runner/experiment.rs:1715 — CAP 固定为 1024*1024；仅读 CAP+1，超过便截断到 CAP。; crates/opi-eval/src/runner/experiment.rs:2077 — capture_policy=full 时封存的仍是上述 capture.bytes 前缀。; crates/opi-eval/scripts/verify-local-acceptance.py:418 — stdout_truncated 必须为 false，随后还要求 AgentEnd 和成对工具事件。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-evidence/runs/terminal-bench-2.1 — 第一次真实 pi 完整流 1304530 bytes、exit 0、原生 reward 1；封存仅 1048576 bytes，import-evidence-incomplete。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/terminal-bench-2.1-actual-observations.json — 重新构建后 Opi 完整流 2269104 bytes，6 条匹配响应、7 次成对工具调用、exit 0、reward 1；封存仍被截断，原始验收函数拒绝。同轮 pi 的 757683-byte 流通过，构成低于上限的反证。; E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/terminal-bench-3.0-actual-observations.json — pi actual bytes=2002525, sealed_truncated=True, matched responses=15, settled tool calls=28, native=known:0(native-trial-result); opi actual bytes=2942536, sealed_truncated=True, matched responses=11, settled tool calls=24, native=known:0(native-trial-result); E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/deepswe-v1.1-actual-observations.json — pi actual bytes=1587986, sealed_truncated=True, matched responses=7, settled tool calls=12, native=known:0(native-trial-result); opi actual bytes=2996723, sealed_truncated=True, matched responses=14, settled tool calls=13, native=known:0(native-trial-result)
- Refutation attempted: 有界内存和截断后拒绝不完整 pi 导入本身符合 fail-closed；低于上限的新 pi trial 也成功通过。但这不能兑现 Phase 19 要求的真实循环封存：一次仅 7 次工具调用的 Opi 流和先前 9 次调用的 pi 流已超过无配置入口的上限。完整 upstream 日志虽被另存，并不自动成为验收器所核对的封存流；不能用操作者重写 terminal 标志修复原证据。
- Suggested closure: 保留有界读取/解析，同时让可验证的原生流或必要的完整终态与工具事实能以字节绑定的形式封存；同步验收器契约并用实际长于 1 MiB 的两个客户端流验证。不要删除截断标志或伪造结束事件来通过。

## Verification Commands

所有 Rust 命令在封存 Linux export 执行，复用外置 Cargo cache，incremental 开启。完整日志与临时探针留在外部 evidence 根目录；没有执行 cargo clean。

| Command | Result | Requirement/finding |
|---|---|---|
| python scripts/opi-doc-check.py | PASS | documentation contracts |
| cargo fmt --check --all | PASS | final gate |
| cargo clippy --offline --workspace --all-targets -- -D warnings | PASS | final gate |
| cargo test --offline -p opi-eval --all-targets | PASS: 328 passed; 9 ignored; 14 binaries | focused production integration |
| python -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py' | PASS: 228 tests; 1 Docker-dependent skip | Python integration/producer/acceptance boundaries |
| cargo test --offline --workspace --all-targets | FAIL: artifact_audit_script requires Git objects missing from export | 006; original failure retained |
| cargo test --offline --workspace --all-targets -- --skip artifact_audit_accepts_real_declared_commit_objects | FAIL: harness=false binary rejects test-harness arguments | not a passing gate; superseded coverage by no-fail-fast command |
| cargo test --offline --workspace --all-targets --no-fail-fast | FAIL: 4505 passed; 3 failed; 14 ignored; 229 binaries | 006 |
| GIT_DIR=/mnt/d/Luiz/Odradek/opi/.git cargo test --offline -p opi-coding-agent --test artifact_audit_script artifact_audit_accepts_real_declared_commit_objects -- --exact --nocapture | PASS: 1; read-only Git object lookup | export-dependent failure refutation |
| cargo test --offline -p opi-coding-agent --test oauth_auth pkce_total_budget_bounds_token_response_body_after_headers -- --exact --nocapture | PASS: 1 on exact retry | initial failure not repeated; no rewrite of full gate |
| cargo test --offline -p opi-coding-agent --test tools_read_write_edit_bash bash_tool_env_inheritance_reporting -- --exact --nocapture | FAIL: 1; reproduced empty env output | 006 |
| cargo test --offline --workspace --doc | PASS: 12 doctests | final gate |
| RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --no-deps | PASS | final gate |
| cargo package --offline -p opi-eval --list | exit 0; no publication; export-generated __pycache__ visible | packaging limitation; not proof of clean tracked package |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_file_credential_projection -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-001 |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_cleanup_stops_dispatch -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-002 |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_provenance_stops_dispatch -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-002 |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_verify_trial_symlink -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-003 |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_pi_only_regrade -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-004 |
| cargo test --offline --manifest-path /mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-p19-codex-gpt6-jn4kz92u/probe/Cargo.toml --test probes audit_full_selection_report -- --exact --nocapture --test-threads=1 | FAIL as intended by behavioral assertion; actual defect observed | P19-AUD-005 |
| cargo test --offline --locked -p opi-eval --test native_driver native_protocol_client_matrix -- --exact --ignored --nocapture --test-threads=1 | PASS: 1 ignored filter explicitly executed; 30 client cases | P19-AC-006-fixture; P19-006-terminal |
| cargo test --offline --locked -p opi-eval --test native_driver native_output_control_client_matrix -- --exact --ignored --nocapture --test-threads=1 | PASS twice: original and portable Opi binaries separately bound to proof | P19-006-controls |
| cargo test --offline --locked -p opi-eval --test native_driver native_client_turn_budgets -- --exact --ignored --nocapture --test-threads=1 | PASS: default 52 requests, explicit two-turn boundary, projected native configurations and omitted-output rejection | P19-006-controls |
| opi-eval doctor --config <external-evidence>/tb21.toml | exit 0; actual cold-cache resource observations retained | P19-001-doctor |
| opi-eval prepare --config <external-evidence>/<family>.toml --out <external-evidence>/material-stable-<family> --opi-source /mnt/d/Luiz/Odradek/opi --opi-commit e079780bda6681ec466f79bd15f3248baf0f2eb8 | PASS: each of three family commands exit 0; fixed source, runner, tool and image identities | P19-AC-001-prepare; P19-001-acquire |
| opi-eval validate/plan --config <external-evidence>/<family>.toml --native-material <external-evidence>/<family>-ready-material.json | PASS for all three original materials and separately for portable runtime materials | P19-001-acquire; P19-007-plan |
| cargo test --offline --locked -p opi-eval --test native_driver native_tb21_artifact_witness -- --exact --ignored --nocapture --test-threads=1 | FAIL initially: oracle passed, nop infrastructure failure; preserved as failed command | P19-AC-003-native |
| cargo test --offline --locked -p opi-eval --test native_driver native_tb3_artifact_witness -- --exact --ignored --nocapture --test-threads=1 | PASS: 1 ignored filter explicitly executed | P19-AC-004-native |
| python3 <external-audit-scratch>/probe-native-api-verifier.py | PASS as refutation probe: unchanged full real Opi stream is rejected specifically for the API representation; not an acceptance bundle | P19-AUD-009 |
| cargo test --offline --locked -p opi-eval --test native_driver native_deepswe_patch_witness -- --exact --ignored --nocapture --test-threads=1 | PASS: 1 ignored filter explicitly executed | P19-AC-005-native |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval run --plan /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/tb21-plan.json --root /tmp/opi-p19-live-tb21-uc16tclh/terminal-bench-2.1 --canaries '<private-canary-file>' | exit 0; native/receipt facts and original classification retained | terminal-bench-2.1 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval verify --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/terminal-bench-2.1 | exit 0; native/receipt facts and original classification retained | terminal-bench-2.1 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval report --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/terminal-bench-2.1 --out /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/reports/terminal-bench-2.1.json | exit 0; native/receipt facts and original classification retained | terminal-bench-2.1 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval run --plan /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/tb30-plan.json --root /tmp/opi-p19-live-tb30-lnufqek_/terminal-bench-3.0 --canaries '<private-canary-file>' | exit 1; native/receipt facts and original classification retained | terminal-bench-3.0 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval verify --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/terminal-bench-3.0 | exit 0; native/receipt facts and original classification retained | terminal-bench-3.0 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval report --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/terminal-bench-3.0 --out /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/reports/terminal-bench-3.0.json | exit 0; native/receipt facts and original classification retained | terminal-bench-3.0 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval run --plan /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/deepswe-plan.json --root /tmp/opi-p19-live-deepswe-zus30w36/deepswe-v1.1 --canaries '<private-canary-file>' | exit 1; native/receipt facts and original classification retained | deepswe-v1.1 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval verify --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/deepswe-v1.1 | exit 0; native/receipt facts and original classification retained | deepswe-v1.1 live / offline |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval report --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/runs/deepswe-v1.1 --out /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/reports/deepswe-v1.1.json | exit 0; native/receipt facts and original classification retained | deepswe-v1.1 live / offline |
| python3 /tmp/opi-audit-p19-e079780-7hFVlg/crates/opi-eval/scripts/verify-local-acceptance.py --root /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable --out /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable-acceptance-verification.json --companion /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval | exit 2 | P19-AC-012; acceptance |
| python3 /tmp/opi-audit-p19-e079780-7hFVlg/scripts/opi-artifact-audit.py /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable/opi-artifact-audit --workspace-root /mnt/d/Luiz/Odradek/opi --json | exit 0 | P19-AC-012; artifact-audit |
| /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/bin/opi-eval conformance --suite benchmark --adapter terminal-bench-2.1 --case nop-witness --root /tmp/opi-audit-p19-tb21-nop-confirmed-20260926 --fixtures /tmp/opi-audit-p19-e079780-7hFVlg/crates/opi-eval/tests/fixtures --provider /tmp/opi-audit-p19-e079780-7hFVlg/crates/opi-eval/scripts/scripted-provider.py --native-material /mnt/opi-audit-e-20260926/opi/evaluations/phase19-codex-gpt6-e079780-20260926/tb21-ready-material.json | exit 0; met=true, verified-native, reward=0 | P19-AC-003-native |
| native_verifier_only_regrade_witness / native_owned_resource_cancellation / remediation_native_stage_deadlines | NOT RUN in this audit; default suite ignores these Docker diagnostics; no claim of passing | P19-AC-009-regrade; P19-AC-010-lifecycle |
| python .agents/skills/_shared/scripts/assurance_set.py complete docs/snapshots/phase19 <private-member> --reviewer codex --model gpt6 | FAIL: history target exists with different bytes | workflow installation; AUDIT-INCOMPLETE |
| python .agents/skills/_shared/scripts/assurance_set.py recover docs/snapshots/phase19 | FAIL: same pre-existing history collision | workflow recovery |
| python <external-audit-scratch>/rollback-own-assurance-install.py | PASS: exact prior active bytes restored; existing history unchanged; only own transaction removed | safe rollback; no source edit |
| python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19 | PASS after rollback; prior paths clean | installation remains incomplete |

## Fresh Local Evidence

新证据：`E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable`。六次均为本轮真实 Opi/pi 调用，审计源码固定 `e079780bda6681ec466f79bd15f3248baf0f2eb8`，不是旧材料改日期。完整 manifest SHA-256 为 `1470fd8273630cc3b2478e3ea79d29c5a8fdadf015b4d5548c6c9cd0212f9ccb`。原生结果归属、工具循环和输出上限的来源分别记录，不把本地回环冒充真实 provider 试验。

所选连接为 zhipu / anthropic-messages / glm-5.3-flash；并发 1，max_output_tokens=4096，context_window=128000，temperature/top_p/reasoning omitted。凭据仅由用户授权的 .env 引用解析，六次材料中所用 API 密钥精确字节扫描均为 0 命中，grader 前清理记录保留。未声称服务端内部模型或未观测采样参数获得独立证明。

实际客户端：pi 0.85.1 固定源；Opi 0.8.2 从同一封存源码重新构建 x86_64-unknown-linux-musl。保留完整 Cargo 命令、工具链、Cargo.lock 与二进制摘要；原任务容器内 --version 和新二进制初次/工具续轮输出上限验证均通过。

| Family | Agent | Matching responses | Paired tools | Native reward | Report class | Sealed stdout truncated |
|---|---|---:|---:|---|---|---|
| terminal-bench-2.1 | pi | 8 | 7 | known:1(native-trial-result) | graded-native | False |
| terminal-bench-2.1 | opi | 6 | 7 | known:1(native-trial-result) | graded-native | True |
| terminal-bench-3.0 | pi | 15 | 28 | known:0(native-trial-result) | invalid-execution | True |
| terminal-bench-3.0 | opi | 11 | 24 | known:0(native-trial-result) | graded-native | True |
| deepswe-v1.1 | pi | 7 | 12 | known:0(native-trial-result) | invalid-execution | True |
| deepswe-v1.1 | opi | 14 | 13 | known:0(native-trial-result) | graded-native | True |

表中 native reward 是固定原生 grader 的事实，不把 invalid-execution 重新算作有效 Agent 分数。TB2.1 的两个有效原生结果均为 1；其余任务的原生结果和报告分类按表保留。一个任务、一次重复只证明调用路径，不支持总体成功率、产品优劣或跨家族平均。

原始验收器结果为 accepted=false（exit 2），并未通过 Phase exit。逐项检查发现两个独立障碍：封存的客户端 stdout 被固定 1 MiB 上限截断；Opi 原生 ApiKind=Anthropic 与验收器要求的 pi 风格字符串不符。完整 upstream 原生流另存且按 SHA-256 绑定，未删除截断标志、未改 seal、未补造 AgentEnd。

三个 family 的 Companion verify/report 均 exit 0；这只证明原 seal 和报告重建成功，不能替代上述真实验收。Opi-only artifact audit 结果见 `E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/fresh-portable-artifact-audit-command.json`，exit=0。

早期失败全部保留：E-drive 运行目录的 Compose getwd 故障；GNU Opi 的 GLIBC_2.39 加载失败；先前 pi 流截断；诊断期间停止的 TB3 调度。索引及原 run-report 摘要见 fresh-portable/attempt-history.json。运行目录转到 Linux /tmp 后，逐个结果树按字节复制回 E 盘；source/destination tree SHA-256 一致。

原生正反例与这六次 Agent trial 分开记录：TB2.1 oracle=1/nop=0（初始 nop 命令失败保留，Linux 路径确认通过）；TB3 native witness PASS；DeepSWE committed-patch witness PASS。未执行的 verifier-only regrade、owned cancellation、stage-deadline ignored filters 明确标记 NOT RUN。

默认测试、探针及工作区门禁的原始日志与外部探针副本保留在 `E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/audit-checks-final`。当前审计没有修复生产问题、改写规范/实现账本或提交 Git。test impact: none。

正式安装状态：AUDIT-INCOMPLETE。complete 在切换后因 pre-existing history target exists with different bytes 失败，常规 recover 同样失败。本轮通过安装工具原有 rollback 分支恢复了精确 prior 字节，并核对既有归档未改动，随后仅清理本轮事务。Git 工作区恢复干净；本候选报告的 FAIL 是私有要求判定，不是新的 live-set aggregate。安装/回滚记录见 E:/opi/evaluations/phase19-codex-gpt6-e079780-20260926/assurance-installation.json。


## Verdict Rationale

FAIL 按 mandatory requirement state 机械推导。当前有 10 个 actionable findings；所有非 met 的 mandatory 行均有 reciprocal finding。已通过的协议矩阵、原生 witness 和逐 family 的 verify/report，不能覆盖这些边界失败或最终 workspace 门禁的缺口。

本轮四文件候选尚未正式安装。历史归档冲突使安装未完成，状态为 AUDIT-INCOMPLETE；已恢复 prior active 字节，不宣称新的 live-set aggregate verdict。
