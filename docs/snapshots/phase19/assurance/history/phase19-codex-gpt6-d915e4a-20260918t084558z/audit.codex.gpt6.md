# Phase 19 Audit

**Audit run ID**: `phase19-codex-gpt6-d915e4a-20260918t084558z`
**Audit head**: `d915e4a22509c0f60c2651dbccfd054f0abc73d4`
**Reviewer ID**: `codex`
**Model ID**: `gpt6`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt6`
**Model identity source**: operator-declared
**Independence**: unknown — 本任务从当前 committed baseline 独立检查，未读取既往或同级 audit/remediation 结论；未验证实现者模型家族，不把新上下文宣称为跨家族独立性。
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

本成员发现 8 项当前实现缺陷和 1 项必需动态证据不可评估项，均为 Major/blocks。模型标识逐字采用操作员声明 `gpt6`，不推断精确 runtime 版本。只读生产审计，无实现修改、提交或 live provider 调用。

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `d9b84dd2d85d64f0fa7b3462dbfd00f6f0a464b02393684e70840a9ecdc642a0` | committed source；registered source 的 stored hash 一致 |
| `docs/snapshots/phase19/opi-impl-state.json` | `35b3378bd902cc976bb348d794dd1b5d3c5f160a58769c3aeb62fea175a7b7a9` | committed source；registered source 的 stored hash 一致 |
| `docs/opi-spec.md` | `1c26136df87f0f8e7c39f248cdd5b313393d343e58b2932b54682b6530f245db` | committed source；registered source 的 stored hash 一致 |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md` | `5be1b609145556c90c33487a30025cb2d93829123243d5e8cbf5586bd151e91a` | committed source；registered source 的 stored hash 一致 |
| `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md` | `d41c826412b0d71802327d60f953708f6cb2083436a2a2b0d99a2e52d851f194` | committed source；registered source 的 stored hash 一致 |

先封存 71 个独立 proof obligations，再读取 production。所有身份、criterion hash 和 observable behavior 保持不变；当前仅填充证据、checks、state 和 reciprocal findings。
代码与检查均来自 `git archive d915e4a22509c0f60c2651dbccfd054f0abc73d4` 的私有 export，不使用 worktree 或未提交源码。

## Requirement Conformance

状态计数：met=49, not-assessable=7, not-met=1, partially-met=14。全部为 mandatory。表格是机器记录的索引；完整逐项 surfaces、测试证据和 observed checks 见同 stem 的 requirements.jsonl。

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| `P19-AC-001-01` | P19-AC-001; sections 3–10 | 本地 origin 与按选定 subject 装载的 CLI 路径不要求 GitHub receipt；独立工作目录/无 fixtures 的合成 native CLI 案例通过。此项证明入口解耦，非真实 native 验收。 | met | — |
| `P19-AC-001-02` | P19-AC-001; sections 3–10 | doctor 使用 Docker engine 观测并保留 unknown 原因；读取资源不会启动 Agent。requirements/resource 检查有负向测试。 | met | — |
| `P19-AC-001-03` | P19-AC-001; sections 3–10 | 已缓存材料的 pin、package、image 身份校验通过；缺失 Agent/corpus/runner 的生产 acquisition 分支仍未接通。 | partially-met | P19-AUD-001 |
| `P19-AC-001-04` | P19-AC-001; sections 3–10 | 仓库外原始目录保存实际 run/report、frozen controls 和 command exits，当前 verifier 接受；本次未独立闭合完整 prepare provenance 或全部授权/失败尝试历史。 | partially-met | P19-AUD-009 |
| `P19-AC-002-01` | P19-AC-002; sections 3–10 | 结果入口比对新 job 集、唯一 trial、task/config、选定 Agent、executable 和 model；错配、缺失、oracle 不能成为选定 Agent 成绩。 | met | — |
| `P19-AC-002-02` | P19-AC-002; sections 3–10 | native/verifier-input 绑定声明的 collection、已收集文件和相应 delivered 摘要；合成替换、遗漏和未声明输入案例被拒绝。真实 handoff 另列动态义务。 | met | — |
| `P19-AC-002-03` | P19-AC-002; sections 3–10 | Harbor/Pier 的选定 integration 在 upstream task environment 安装/执行客户端；结果由同一 job/trial reader 接纳，无独立 host Agent 加默认 job 的 native 分数路径。 | met | — |
| `P19-AC-002-04` | P19-AC-002; sections 3–10 | oracle/nop 只通过显式 conformance diagnostic 入口，正常 native admission 要求实际选定 Agent terminal。 | met | — |
| `P19-AC-003-01` | P19-AC-003; sections 3–10 | 共享 verifier task lifecycle 传给 Harbor，integration 在返回前移除私有凭据；dummy 凭据删除和 environment 契约测试通过。 | met | — |
| `P19-AC-003-02` | P19-AC-003; sections 3–10 | 真实 TB2.1 正确/错误 witness 是显式 ignored 案例，本次未运行，也未提供对应 raw job/grader 证据。 | not-assessable | P19-AUD-009 |
| `P19-AC-003-03` | P19-AC-003; sections 3–10 | 已直接检查保存的 TB2.1 Opi/pi live terminal、tool/provider streams 和 native reward；两侧 reward=1、terminal exit0，build SHA 与 manifest 一致。证明该保存试验，不将其替代当前正误 witness。 | met | — |
| `P19-AC-004-01` | P19-AC-004; sections 3–10 | task lifecycle 与 collection 由 task package 提供；separate verifier handoff 只接受声明的 artifact，合成正负向 cases 通过。 | met | — |
| `P19-AC-004-02` | P19-AC-004; sections 3–10 | multi-step/unsupported lifecycle、missing/undeclared/substituted collection 被生产 admission 拒绝；native-driver 负向案例执行。 | met | — |
| `P19-AC-004-03` | P19-AC-004; sections 3–10 | 真实正误 witness 和无关 canary 的 Docker handoff 证据不在本次输入；合成覆盖不能补足。 | not-assessable | P19-AUD-009 |
| `P19-AC-004-04` | P19-AC-004; sections 3–10 | TB3 两实际客户端有工具事件与 native result，收集 /app/evalbench/ 的 bound digest 一致；两侧合法 reward=0、无 native exception。 | met | — |
| `P19-AC-005-01` | P19-AC-005; sections 3–10 | Pier 路径验证 task base、collect hook、pristine verifier 与声明网络策略；model.patch 作为 collection 而非工作区任意文件进入 verifier。 | met | — |
| `P19-AC-005-02` | P19-AC-005; sections 3–10 | wrong base、patch substitution、错配与未声明 collection 的合成生产入口负例通过。 | met | — |
| `P19-AC-005-03` | P19-AC-005; sections 3–10 | 真实正确/错误 committed patch witness 未运行，未获得原始收集及 grader 输出。 | not-assessable | P19-AUD-009 |
| `P19-AC-005-04` | P19-AC-005; sections 3–10 | DeepSWE 两实际客户端有工具事件与 native result，原始 collect 保存 empty committed patch 摘要；两侧合法 reward=0（未承诺成功修复）。 | met | — |
| `P19-AC-006-01` | P19-AC-006; sections 3–10 | 选定 connection 的 provider/api/base URL/model/credential reference 投影到各原生配置，临时隔离环境由 integration 消费；无隐式 endpoint/model fallback 分支。实际 client matrix 另列。 | met | — |
| `P19-AC-006-02` | P19-AC-006; sections 3–10 | requested/effective projection 存在，但 required sampling 只验证值类型；Opi Unsupported 仍获 comparable。 | partially-met | P19-AUD-003 |
| `P19-AC-006-03` | P19-AC-006; sections 3–10 | 真实锁定客户端三协议 fake-server matrix 是 ignored 测试，本次仅运行 stand-in / projection / server 单元测试，不能替代客户端 wire/tool-loop。 | not-assessable | P19-AUD-009 |
| `P19-AC-006-04` | P19-AC-006; sections 3–10 | 缺失凭据和 required max-output proof 可前置拒绝；required sampling 的 capability gate 未闭合。 | partially-met | P19-AUD-003 |
| `P19-AC-006-05` | P19-AC-006; sections 3–10 | Agent terminal typed provider causes 进入单一 native classification；exit-zero provider error 不能计分，retriable intermediate 与 terminal outcome 分离；对应 native provider-error 回归通过。 | met | — |
| `P19-AC-006-06` | P19-AC-006; sections 3–10 | native facts 保留 source/unknown reason；缺失服务模型、usage、cost 不从 config 捏造；不可强制的 monetary cap 在 resolve 拒绝。 | met | — |
| `P19-AC-006-07` | P19-AC-006; sections 3–10 | 六个 sealed experiments 均为 live，connection=zhipu-anthropic、Anthropic base URL、glm-5.3-flash；pi tool_execution_start 分别4/26/16，Opi provider/tool evidence events 均非空。 | met | — |
| `P19-AC-007-01` | P19-AC-007; sections 3–10 | native inventory 从完整 pinned corpus 枚举；effective requirements 分 Agent/verifier 保留 CPU/memory/storage/GPU types/network/lifecycle/time/default origins。未拉全量镜像。 | met | — |
| `P19-AC-007-02` | P19-AC-007; sections 3–10 | selection full/subset、task aliases、explicit reason、repetitions/seeds 和 max_trials 经 resolver/plan expansion 绑定；拒绝静默缩集。 | met | — |
| `P19-AC-007-03` | P19-AC-007; sections 3–10 | native preflight 使用 Docker/WSL 观测，remote storage/GPU 未证实保留 unknown，required unknown 与 unmet 拒绝；合成 resource cases 通过。 | met | — |
| `P19-AC-007-04` | P19-AC-007; sections 3–10 | plan/2 digest 绑定 source、材料 identity、inventory、exact trials 与 preview；run 验证相同展开结果与 pins，篡改拒绝。 | met | — |
| `P19-AC-007-05` | P19-AC-007; sections 3–10 | normal frozen run 按 task-indexed material 派发，task/user stage timeouts 取更窄值；material/plan drift 负例通过。replacement 另列 AC01007。 | met | — |
| `P19-AC-008-01` | P19-AC-008; sections 3–10 | graded-native zero、Agent failure 和 benchmark-valid timeout 保留 native reward/denominator；不将合法零分归为 infrastructure。 | met | — |
| `P19-AC-008-02` | P19-AC-008; sections 3–10 | provider/access/identity/infrastructure/grader failure 进入 invalid 或 unsealed attempted coverage，不生成有效分数；合成失败矩阵通过。 | met | — |
| `P19-AC-008-03` | P19-AC-008; sections 3–10 | run-control exact planned trials 与 unsealed metadata 保留 attempted/unexecuted/effect-unknown；offline report 不自动重跑。recovery 命令另有 F005。 | met | — |
| `P19-AC-008-04` | P19-AC-008; sections 3–10 | report/2 分开 corpus/selected/planned/attempted/valid/failed/invalid/unexecuted、native metric 和原始 test-count provenance；report regressions 通过。 | met | — |
| `P19-AC-008-05` | P19-AC-008; sections 3–10 | scope 和 incomplete 状态保留；按 benchmark 聚合 native metric，test-case fraction 单独显示且缺失为 unknown。 | met | — |
| `P19-AC-008-06` | P19-AC-008; sections 3–10 | task/group/manifest、missing sides、protocol guard 存在；但 output capability 在 offline coverage 被丢弃，required sampling 在 run 也未约束 pair。 | partially-met | P19-AUD-003, P19-AUD-004 |
| `P19-AC-008-07` | P19-AC-008; sections 3–10 | single-subject/pi-only 不要求人工 edge 或 Opi executable；未知 native telemetry 保留 reason。 | met | — |
| `P19-AC-008-08` | P19-AC-008; sections 3–10 | outcome 用 sealed settlement 重建，但 comparability 仍在 report::coverage 单独推断，漏掉缺失 output-control proof。 | partially-met | P19-AUD-004 |
| `P19-AC-009-01` | P19-AC-009; sections 3–10 | bundle verify 对 sealed bytes/hash fail closed；verify、report、native regrade 入口分离，旧 conformance 不自动变为 native score。 | met | — |
| `P19-AC-009-02` | P19-AC-009; sections 3–10 | report 输出路径需 fresh/outside source；读取 sealed inputs 离线且同输入输出稳定；output containment/immutability 测试通过。比较含义错误另列 F004。 | met | — |
| `P19-AC-009-03` | P19-AC-009; sections 3–10 | regrade 经 verifier-only integration 直接调用原生 Verifier，恢复保存 collection/patch，fresh root，cached image 与 offline uv；不构造 Agent job。此项为机制与合成回归证据。 | met | — |
| `P19-AC-009-04` | P19-AC-009; sections 3–10 | 缺失 cache/artifact、digest drift、不可离线的 verifier network/service 需求前置拒绝；未凭空补交付。 | met | — |
| `P19-AC-009-05` | P19-AC-009; sections 3–10 | 派生 observation 绑定 source bundle、恢复输入、fresh verifier/image/raw result；输出隔离原始证据且 cleanup 不确定停止派发。 | met | — |
| `P19-AC-009-06` | P19-AC-009; sections 3–10 | 三个真实家族 artifact-sensitive fresh regrade + original byte preservation 见证为 ignored external-material 测试，本次无 raw 证据。 | not-assessable | P19-AUD-009 |
| `P19-AC-010-01` | P19-AC-010; sections 3–10 | 凭据仅按所选 reference 解析，Agent HOME/config 隔离；生产 grader-only 路径剔除凭据，dummy credential 生命周期测试通过。 | met | — |
| `P19-AC-010-02` | P19-AC-010; sections 3–10 | integration 在共享评分之前移除 handoff/config 凭据，采用精确 task/log 挂载；cleanup/containment 单元与合成集成证据通过，真实 canary 见证另列。 | met | — |
| `P19-AC-010-03` | P19-AC-010; sections 3–10 | concurrency/turn output limits 和 unsupported cost/total caps 有准入；required sampling 与 prepare materializer 无界 Command::output 仍是缺口。 | partially-met | P19-AUD-002, P19-AUD-003 |
| `P19-AC-010-04` | P19-AC-010; sections 3–10 | 信号取消和 owned container 机制存在；privacy canary 失败后仍派发第二个串行 native trial，prepare 获取进程也未纳入同一有界监督。 | partially-met | P19-AUD-002, P19-AUD-007 |
| `P19-AC-010-05` | P19-AC-010; sections 3–10 | owned container 清理用精确 Compose label/ID/root，失败阻止封存，shared cache/邻居资源保留；清理不确定后继续派发的问题另列 AC01004。 | met | — |
| `P19-AC-010-06` | P19-AC-010; sections 3–10 | recover 本身不启动模型，却覆盖 run-report.json 并将 trials 清空；仅遍历已存在目录，不完整分类 planned unstarted。 | not-met | P19-AUD-005 |
| `P19-AC-010-07` | P19-AC-010; sections 3–10 | replacement 从未展开 source 查 trial，selection plan target 被拒绝；固定 .r2 后缀在重复请求中复用身份。 | partially-met | P19-AUD-006 |
| `P19-AC-010-08` | P19-AC-010; sections 3–10 | 真实 Docker cancellation/credential-canary isolation witness 未提供；默认测试中的模拟 Docker 清理不等价。 | not-assessable | P19-AUD-009 |
| `P19-AC-011-01` | P19-AC-011; sections 3–10 | 默认测试显式使用 dummy credentials 与 hermetic/native-fixture；native/live 不由 ambient credentials 激活；默认 CI 与 workflow_dispatch 入口分开。 | met | — |
| `P19-AC-011-02` | P19-AC-011; sections 3–10 | native tests 可本地显式运行；CI producer 仍拥有本地 prepare 无法取得的 Agent/corpus/runner acquisition，尚非同一公开准备策略。 | partially-met | P19-AUD-001 |
| `P19-AC-011-03` | P19-AC-011; sections 3–10 | verifier 能验证结构和 byte bindings，但 fixture native mode、scripted-local 和 synthetic binaries 也获 accepted=true，不能证明六个真实 trial。 | partially-met | P19-AUD-008 |
| `P19-AC-011-04` | P19-AC-011; sections 3–10 | scoped capture view 必须含三个 Opi capture 且 byte-bound/exhaustive；upstream repo URL+commit 从缓存配对，empty/substitution/mismatch 负例通过。 | met | — |
| `P19-AC-011-05` | P19-AC-011; sections 3–10 | 仓库外原始目录保存实际 run/report、frozen controls 和 command exits，当前 verifier 接受；本次未独立闭合完整 prepare provenance 或全部授权/失败尝试历史。 | partially-met | P19-AUD-009 |
| `P19-AC-012-01` | P19-AC-012; sections 3–10 | 找到六个保存的真实 trial，三个 benchmark 各 Opi/pi 一次；直接核对 raw terminal、工具轨迹与 native result，current verifier exit0。 | met | — |
| `P19-AC-012-02` | P19-AC-012; sections 3–10 | raw root 的 manifest、sealed experiment/output-control、material、bundle digest、upstream identity cache 经当前 verifier 验证，raw terminal executable SHA 与 manifest 对应；源为55fbc950。 | met | — |
| `P19-AC-012-03` | P19-AC-012; sections 3–10 | native runner 本身拒绝多类 invalid execution，但独立 acceptance verifier 接受 native-fixture 全套证据；不足以拒绝 fixture substitution。 | partially-met | P19-AUD-008 |
| `P19-AC-012-04` | P19-AC-012; sections 3–10 | 本次 WSL opi-eval scoped tests 与 doc check 已执行；未运行整 workspace gates，也未获得可独立复核的当前 native acceptance/gate 原始证据。 | not-assessable | P19-AUD-009 |
| `P19-AC-013-01` | P19-AC-013; sections 3–10 | publish=false、无 Opi crate dependency；pi-only test 通过；没有产品启动/registry hook 或 Minimal Runtime 激活。 | met | — |
| `P19-AC-013-02` | P19-AC-013; sections 3–10 | 外部 corpus/runner/image 由 pins/material 引用，不内嵌 graders/images；pub cli/experiment 为 unpublished 同包 facade，内部 execution/bundle contracts 保持私有。 | met | — |
| `P19-AC-013-03` | P19-AC-013; sections 3–10 | 当前 help/callers/fixtures/schemas 与双语 README 的新 material/5、plan/2、report/2、regrade/2 路径均存在；WSL opi-doc-check PASS。 | met | — |
| `P19-AC-013-04` | P19-AC-013; sections 3–10 | Unreleased 记录 deliberate 0.x break；旧 sealed bundles 读取不改写，legacy conformance 明确区分。recover 的 report 覆盖另列 F005。 | met | — |
| `P19-AC-013-05` | P19-AC-013; sections 3–10 | 回滚契约为停用本地执行或恢复已验证构建；当前 normal native route 没有静默 fallback/default oracle/第二评分 facade。 | met | — |
| `P19-PRIN-002` | PRIN-002 | 私有 binding/settlement 有三个 family 与 run/report/regrade 实际 consumers；CLI/experiment 同包公开 facade 有冻结状态价值。无新 core seam/crate/trait。 | met | — |
| `P19-PRIN-004` | PRIN-004 | 多数身份/byte 边界拒绝式校验已实现；control comparison、privacy 后续派发、acceptance fixture 边界仍未闭合。 | partially-met | P19-AUD-003, P19-AUD-004, P19-AUD-007, P19-AUD-008 |
| `P19-PHASE-001` | PHASE-001 | 当前 registered supplemental source 引用父规范并有战略优先级、placement/non-goals、可观察验收、风险/回滚与 Linux/WSL 边界；stored hashes 与 committed bytes 一致。 | met | — |
| `P19-AUTH-004` | AUTH-004 | 逐节检视英中 registered specs，P19 acceptance IDs、provider identity、风险与 handoff 范围一致；doc checker PASS。 | met | — |

## Standards Review

Agent-neutral Companion 仍 `publish=false`，没有 Opi crate dependency 或产品启动激活。新功能保留在现有 CLI/experiment、私有 runner/benchmark/bundle/report 模块；闭集控制使用 Rust enum 和 typed errors。跨语言 integration 继续调用 pinned Harbor/Pier，未发现新嵌入 grader、镜像或 sandbox engine。局部 probe 只在 export 外的临时测试副本追加/捕获证据，未改动 production 或 repository tests。

文档、schemas、fixtures 与 deliberate 0.x break 在双语 README 和 Unreleased 有对应记录；WSL documentation contract 通过。本次 assurance 产物没有英中 counterpart，故不存在需同步的另一个文档文件。Rust/Python 测试均是实际运行，不能将 9 个 ignored native tests 和 1 个 Python Docker skip 算作通过。

## Spec Review

主要偏差集中在本地冷缓存获取、有效控制比较、恢复/替换、隐私失败后的调度和真实验收身份。计划/材料的 hash、task-indexed resolution、完整资源 preflight、native result binding、独立 verify/report/regrade、诚实分母等机制已有生产实现和区分正负输入的离线测试证据。声明实际执行的义务单独保留 not-assessable，未用静态类型或 stand-in 推导真实客户端/容器/外部 grader 的成功。

## Security, Invariants, Integration, Test Quality, and Residuals

- Security：临时凭据清理、隔离 config/HOME、精确 owned-resource 清理与 canary publication guard 有实际离线回归；F007 表明 privacy violation 尚未反馈到 run stop owner。
- Invariants：bundle/plan/material digest 与 containment guards 经负例验证；F005 的 recover 覆盖 report 和 F006 的 replacement identity 重用破坏 durable evidence/identity 契约。F002 是静态证明的无界 acquisition 分支，未将未执行的网络 hang 描述为实测事故。
- Integration：F003/F004 通过 public native CLI 与随后 report 的组合探针重现，说明分开测试 resolver、run 和 report 不足以证明共享 controls。F001 的 mock materializer 正例不能覆盖生产冷缓存路径。
- Test quality：309 Rust tests 通过、9 ignored；Python 216 项中 1 skip、其余通过。三项临时精确 probe 也通过预期反例断言。F008 直接复用已存在 acceptance 测试的 synthetic fixture，保存了 accepted=true 与 native-fixture 相冲突的证据；测试名中的 genuine 不作为事实。
- Residuals：production native route 不再默认启动 oracle；旧 conformance bundle 阅读仍明确为旧意义。仍存在 run/report 分别构造 unsupported control 的重复策略，以及 CI-only acquisition 胶水；没有建议扩大重构到其他 crates。

## Minimum-change Conformance

核对 committed Phase task 的 `reuse_search`、`surface_necessity`、`simplification_ceiling`。以下状态评价当前落点，不复述历史审计结论。`net_deletion` 的量化行数不作追溯推断；仅核对当前声明的旧路线是否还参与新 native 分数。未新增生产接口。

| Task | reuse_search / surface_necessity / simplification_ceiling | Current evidence |
|---|---|---|
| 19.1 | conforming / conforming / triggered | 复用 family adapters、NativeTrialBinding、单一 settlement；当前结果 reader 绑定 same job/trial，旧 host-Agent/default-job native score 不在该分支。report controls policy 残留见 19.7。 |
| 19.2 | conforming / conforming / triggered | 两个实际产品 adapter 消费选定 connection；unsupported sampling 已触发 declared revisit_when，F003 尚未拒绝/收窄。 |
| 19.3 | conforming / conforming / conforming | TB3 沿同一 binding、声明 artifacts 与 separate verifier；超出 admitted lifecycle 明确拒绝。实际 witness 证据另为 not-assessable。 |
| 19.4 | conforming / conforming / conforming | Pier 保留 committed patch/base/pristine verifier；未新增独立评分规则。实际 witness 另缺证据。 |
| 19.5 | conforming / conforming / drifted | local-origin 与 pin 验证复用现有 material；公开 acquisition 未闭合且未使用 bounded supervisor（F001/F002）。 |
| 19.6 | conforming / conforming / conforming | inventory/selection/plan 有冻结状态价值；完整 native resources 走同一 preflight，未增加远程调度或全部 TB3 执行承诺。 |
| 19.7 | conforming / conforming / triggered | byte verify 正确独立于 regrade；report 仍另推 controls comparability，F004 未满足删除平行策略的边界。 |
| 19.8 | conforming / conforming / conforming | verifier-only fresh native path 复用 material/binding/bundle；offline 不足时拒绝。实际三家族 artifact-sensitive witness 另缺证据。 |
| 19.9 | conforming / conforming / triggered | 复用 supervisor/lifecycle/ledger，但 recover、replacement、privacy stop 未闭合（F005–F007）。 |
| 19.10 | conforming / conforming / drifted | workflow 仍有公开 prepare 无法替代的 acquisition；offline verifier 结构复用正确但允许 fixture 真实验收（F008）。 |
| 19.11 | conforming / conforming / not-assessable | 已独立复核保存的六次 live raw handoff；当前相关 native witnesses/final workspace gates 仍有证据缺口，不能以任务状态补足（F009）。 |

| Seam / replacement | production_consumers | nonproduction_consumers | net_deletion | residual_glue |
|---|---|---|---|---|
| Selected connection + native binding | Opi/Pi projections；Harbor TB2.1/TB3、Pier DeepSWE；run、report、regrade | experiment_contract/native_driver、Python integration tests | 当前 normal native 路径不以 newest-job/default oracle 供分；未量化历史 diff | 私有 typed binding/terminal handoff 必要；controls admission 漏洞见 F003 |
| Material/5 + plan/2 | prepare、plan、run、regrade、CI material consumer | native_driver resource/multi-task/material negative tests、docs/schema | local origin 不再强制 GitHub receipt；CI acquisition 尚未被公共入口替代 | pinned external scripts 与 task profiles 必要；F001/F002 限制 |
| Settlement/report/verify/regrade | native run；offline verify/report；fresh native verifier | report_contract、report_output_containment、native_driver regrade cases | byte-only regrade 已分离；legacy conformance 不变为 native score | output comparability 仍重复重构（F004），应回到 durable owner |
| Recovery/replacement | CLI run、TrialLifecycle、run-control planned identity | assembled_run 与本次 public-CLI counterexamples | 没有新增并行 ledger；旧简单 TOML rewrite 仍参与 replacement | F005/F006/F007 显示 existing state seam 尚未完整连接 |
| Offline acceptance | 独立 operator verifier、built Companion verify/report | Python acceptance tests、native_driver 6-trial synthetic fixture | 未新增 core/public SDK；无产品 activation | byte/provenance checks 有用，但 fixture/live 区分缺失（F008） |

## Findings

### P19-AUD-001: 本地 prepare 不能取得所需 Agent、corpus 和 runner 材料

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-03, P19-AC-011-02
- Claim: 在只有声明的 pin/producer 材料且缺少实际缓存内容时，公开 prepare 调用的 production prepare-mode 仅接线 tools archive；Agent、corpus、runner 一律返回 not wired，要求操作员另行运行 producer stages。它没有交付规范约定的单一本地材料准备入口。
- Evidence: `crates/opi-eval/src/runner/prepare.rs:919` — prepare 将 missing 清单传给 seeded materializer。；`crates/opi-eval/scripts/materialize-external-locks.sh:99` — 仅 tools/<id> 分支执行获取；其他身份在 149–156 返回 not wired for local acquisition。；`reproduction/acquire.stderr` — 实际 production script 对 agents/pi、corpora/terminal-bench-2.1、runners/harbor 均 exit 2；无网络或模型调用。
- Refutation attempted: ready-cache 与 supervised_acquisition_runs_the_pinned_materializer 测试通过；后者使用测试写入的 acquisition shell，前者预先 seed 缺失内容。生产 script 的明确失败虽 fail-closed 且 README/CHANGELOG 如实写 unwired，却不能满足规范 §3 禁止以手工串联工具替代本地入口的要求。
- Suggested closure: 让公开 prepare 对选定且支持的 pinned Agent/corpus/runner 完成获取或构建，并以真实生产 producer 的冷缓存 public-CLI 测试证明；仍保留 digest 验证与可操作失败。

Reproduction:

1. 准备仅含 committed pins/static/linux-x86_64.json 的临时 cache，plan JSON 列出 agents/pi、corpora/terminal-bench-2.1、runners/harbor。
2. bash crates/opi-eval/scripts/materialize-external-locks.sh prepare --cache <cache> --plan <missing-plan.json>

### P19-AUD-002: 材料获取进程缺少有界监督

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-03, P19-AC-010-04
- Claim: prepare 直接同步调用 Command::output 等待 materializer，未配置 timeout、取消、进程树监督或输出上限；生产下载使用无 timeout 的 urlretrieve。挂住的获取路径无法按声明的监督契约结算，日志也要等子进程退出后才落盘。
- Evidence: `crates/opi-eval/src/runner/prepare.rs:919` — Command::new(...).arg(...).output() 直接等待；未使用现有 ProcessSupervisor。；`crates/opi-eval/scripts/materialize-external-locks.sh:106` — urllib.request.urlretrieve 无 timeout 参数或 bounded enclosing supervisor。；`crates/opi-eval/tests/native_driver.rs:4120` — 现有 acquisition 测试只覆盖快速退出的 seeded fake materializer，没有 hang/cancel/output-budget 负例。
- Refutation attempted: native job 的 ProcessSupervisor、stage timeouts 和 signal cancellation 测试确实通过，但 prepare 没调用它；已有 acquisition log 仅是完成后写入的输出，不是进程监督。此项未执行无限等待探针，不声称观察到真实网络挂起。
- Suggested closure: 复用已有有界进程 owner 监督 acquisition，提供等待、取消和输出边界；添加快成功与超时/取消两类生产入口证据。

Reproduction:

1. 静态追踪 prepare cache-miss 分支 890–961 → materializer prepare-mode；确认从 CLI 到子进程没有 deadline/token/output budget。
2. 以会持续等待的 pinned acquisition fixture 触发 cache miss，应在有界 deadline/cancel 后结束并清理 owned tree；当前调用没有这个结算路径。

### P19-AUD-003: required sampling 未按选定客户端能力准入

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-006-02, P19-AC-006-04, P19-AC-008-06, P19-AC-010-03, P19-PRIN-004
- Claim: required=["temperature"] 且 temperature=0.0 会通过 resolver；虽然 Opi projection 明确 Unsupported，native run 仍派发并将 Opi/pi pair 标为 comparable。比较共享的是请求值，不是两侧有效控制。
- Evidence: `crates/opi-eval/src/experiment.rs:760` — required 仅检查 ControlValue::Value，未检查各选定 client capability。；`crates/opi-eval/src/experiment.rs:1151` — Opi 的 sampling 无原生配置字段，被投影为 Unsupported。；`crates/opi-eval/src/runner/experiment.rs:2335` — unsupported_controls 只收 reasoning/max_output_tokens，遗漏 temperature/top_p。；`probe-controls.log` — current production CLI + synthetic native job：required temperature 的 run exit 0，两个 trial sealed，pair comparable。
- Refutation attempted: required omitted/unknown 解析负例和 required max-output exact-client admission 确实通过；它们分别只检查值是否提供和 output proof。不能否定本例中 Opi sampling 已知 Unsupported 却被派发和比较。该探针只证明准入/报告缺陷，不宣称真实模型执行。
- Suggested closure: 对每个选定客户端的有效控制统一准入；必需控制不支持时在效果前拒绝，optional unsupported 必须保持不可严格比较并进入 sealed/recomputed comparability。

Reproduction:

1. 在 native_driver.rs 的 stage("ok")/write_config 数据中加入 required=["temperature"]；重新生成与该 frozen config 相符的 synthetic output proof。
2. 通过 run_native 调用当前 opi-eval 二进制；观察 exit 0 和 pairs[0].comparability=comparable；外部临时探针 audit_sampling_required_and_offline_comparability 已执行。

### P19-AUD-004: 离线 report 将未证实的输出控制升级为可比较

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-008-06, P19-AC-008-08, P19-PRIN-004
- Claim: native run 在缺少 optional max_output_tokens exact-client proof 时正确记录 unsupported-control:max_output_tokens；对相同 sealed bundle 执行 report 却发布 coverage.comparability=comparable。离线重建丢失控制能力边界，错误制造严格比较。
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2339` — run 将 output_control.is_none() 转为 UnsupportedControl。；`crates/opi-eval/src/report.rs:1537` — coverage 仅重建 reasoning unsupported，未消费 output-control snapshot 的缺失状态。；`crates/opi-eval/tests/native_driver.rs:2287` — 现有 optional-output regression 只断言 run 输出，未执行随后的 report。；`reproduction/optional-output/offline-report.json` — 同一 run exit 1 且 pair unsupported；report exit 0、outcome published、coverage comparable。
- Refutation attempted: report 已正确读取 sealed outcome class、完整分母和缺失 trial；bundle integrity 也通过。但 task grading valid 不等于 controls comparable，当前重算从 requested controls 再推断且漏掉本项。
- Suggested closure: 把比较能力事实纳入单一 durable owner，run/report 共同消费；为 optional-unproven output 的 run→verify→report 添加一致性断言。

Reproduction:

1. 用 stage("ok") / write_config，移除 material.agents[*].output_control_evidence 后运行 native public CLI。
2. opi-eval report --root <same-optional-run>；对照 run-report.json 的 unsupported-control:max_output_tokens 与 offline coverage 的 comparable。

### P19-AUD-005: recover 覆盖原始报告且丢失 trial 明细

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-06
- Claim: 对已完成 run 执行 --recover 会调用 persist_run_report 覆盖同一 run-report.json，将原先两个 trial 清空为 trials=[] 并标为 incomplete；它只扫描已存在的 trial 目录，也不从 planned inventory 补齐未开始项。恢复操作不是只读证据分类。
- Evidence: `crates/opi-eval/src/runner/experiment.rs:508` — recover 分支将新 recovery_report 持久化到原 run 根。；`crates/opi-eval/src/runner/experiment.rs:930` — persist_run_report 使用 std::fs::write 覆盖 run-report.json。；`crates/opi-eval/src/runner/experiment.rs:4118` — recovery_report 只遍历已有目录并输出 trials=[]。；`reproduction/original-run-report.json` — 已保存 recover 前 bytes；recover 后原路径 bytes 变化，trial 数 2→0，exit 1。
- Refutation attempted: 原始 sealed bundles 尚在，正常 report 能从 bundle/run-control 重建一部分信息，且 recover 不调 provider。但规范 §9 要求原始 bundles 和 reports 逐字节保留；可重建不能授权覆盖原报告。
- Suggested closure: recover 从 frozen planned inventory 和 journal 生成只读独立视图，保留原报告 bytes，准确区分 completed/cancelled/failed/effect-unknown/unstarted。

Reproduction:

1. opi-eval run --config crates/opi-eval/tests/fixtures/experiment/local-paired.toml --fixtures crates/opi-eval/tests/fixtures --root <fresh-root>
2. 保存 run-report.json bytes，再对同 root 执行相同 run 参数加 --recover；比较 SHA-256/trials 数。

### P19-AUD-006: replacement 不支持冻结 selection 且重复使用 .r2 身份

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-07
- Claim: replacement 在 selection 展开前从 source.trials 查找目标，因此合法 run-plan 中生成的 trial 被误判为未声明；显式 trials 的 replacement 每次固定追加 .r2，同原 config 在不同新 root 的两次执行会重复 trial/group/experiment identities。
- Evidence: `crates/opi-eval/src/runner/experiment.rs:4075` — replace_group 重新解析未展开 TOML，再按文本行加固定 .r2。；`reproduction/selection-replacement.stderr` — 先成功 plan，再 run --plan --replacement-for trial-baseline-pi-fixture-task-r1：exit 2，not a declared trial。；`reproduction/replacement-ok-1.stdout` — 显式 trial replacement 第一次 exit 0，ids 为 trial-{opi,pi}-1.r2。；`reproduction/replacement-ok-2.stdout` — 第二次独立执行仍是相同 experiment、trial 和 group ids。
- Refutation attempted: 现有 whole-group replacement/recovery tests 对一次显式 [[trials]] 的 double-quoted TOML 可以通过，且使用新 root 保留旧目录；这不覆盖 selection expansion，也不保证多次 replacement identity 新鲜。
- Suggested closure: 基于已冻结/展开的 planned trials 创建独立 replacement group，生成真正新身份并绑定原组和未决效果；拒绝改变 controls/material 的隐式替代。

Reproduction:

1. 生成只有 [selection] 的 local-paired config，再 plan；对生成的确切 trial id 执行 run --plan P --replacement-for ID --root fresh-root。
2. 对显式 local-paired.toml 的 trial-pi-1 分别使用两个 fresh roots 执行 --replacement-for，比较返回 identities。

### P19-AUD-007: 隐私边界失败后继续派发后续 native trial

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-010-04, P19-PRIN-004
- Claim: native seal 检测到声明的 canary 后只记录当前 trial failed，没有触发 run-level stop。串行实验中的下一个 trial 仍启动 Agent/native job；这违背隐私违约后停止受影响派发的零容忍边界。
- Evidence: `crates/opi-eval/src/runner/experiment.rs:2300` — canary_leak 分支使 seal failed，但未 cancel 或返回停止派发的 outcome。；`crates/opi-eval/src/runner/experiment.rs:814` — 下一 wave 只在 run_cancel.is_cancelled() 时停止，非 seal/security failure。；`probe-privacy.log` — 宽度1：trial-opi-native settled=failed 后仍出现 trial-pi-native stage=start；两条 receipt seal=failed:at-evidence。
- Refutation attempted: canary scanner 和 publication guard 确实阻止 seal/report，信号取消也能停止后续 trial；本例检查的是检测违约后的自动 stop-dispatch，当前失败没有连接到 run cancellation owner。没有使用或暴露真实秘密。
- Suggested closure: 把 privacy/authority/provenance 的停止原因反馈给 run supervisor，停止受影响后续派发并保留既有失败/未执行覆盖；使用串行双 trial 的负向测试。

Reproduction:

1. native stage("ok") 的合成环境，将已知会进入 staged evidence 的 dummy canary native-driver-test 写入 --canaries 文件。
2. 执行双 subject native run；外部探针 audit_stop_dispatch_on_privacy_failure 观察到第一个 seal privacy failure 后第二个 native job 仍执行。

### P19-AUD-008: 真实验收校验器接受 native-fixture 合成证据

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-011-03, P19-AC-012-03, P19-PRIN-004
- Claim: verify-local-acceptance.py 将存在 native/verifier-input.json 加 seal 一致性当作真实执行证据，没有拒绝 sealed execution.mode=native-fixture 或 scripted-local connection。当前生产测试完整构造 fake runner/binaries 的六个 trial，校验器返回 accepted=true；这能替代本应要求的真实验收。
- Evidence: `crates/opi-eval/scripts/verify-local-acceptance.py:165` — check_native_evidence 仅检查 gate 文件存在；check_controls 比较配置相等但不检查真实 mode/首次连接。；`crates/opi-eval/tests/native_driver.rs:6909` — local_acceptance_verifier_accepts_genuine_native_evidence 使用 stage("ok") 的 native-fixture，断言 accepted=true。；`reproduction/accepted-fixture-verification.json` — 重新执行原测试并保存中间产物：accepted=true，三个 family 六 trial。；`reproduction/accepted-fixture/runs/terminal-bench-2.1/trials/trial-opi-tb21/bundle/artifacts/control/experiment.json` — execution.mode=native-fixture；provider=scripted，base_url=http://127.0.0.1:48127/v1，model=eval。
- Refutation attempted: 校验器确实拒绝缺少 gate、oracle attribution、篡改 bundle、空/替换 capture、错误 repository/commit 和不匹配 output proof。但结构自洽的 fixture 具备这些字段，测试证明 built Companion verify/report 同样接受；这些 guards 不能证明实际 Agent/原生环境/live tool-loop。
- Suggested closure: 从封存的执行与实际调用原始证据验证 native-live/首次连接、真实 builds、工具循环与原生 handoff；将当前 fixture acceptance 改为负例，独立保留结构校验的 unit seam。

Reproduction:

1. 运行已有 local_acceptance_verifier_accepts_genuine_native_evidence 精确测试；其 stage("ok") 明确是 deterministic stand-in。
2. 在临时测试副本中于 accepted=true 断言后复制 evidence root 和 verification report；对照 sealed experiment 的 native-fixture 和 scripted connection。

### P19-AUD-009: 当前 native witness 与 Phase exit 证据仍有缺口

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P19-AC-001-04, P19-AC-003-02, P19-AC-004-03, P19-AC-005-03, P19-AC-006-03, P19-AC-009-06, P19-AC-010-08, P19-AC-011-05, P19-AC-012-04
- Claim: 已找到并独立复核仓库外保存的六个 live trial，不能再称其缺失。但实际客户端完整三协议矩阵、三个家族正误与 fresh-regrade witness、真实 Docker cancellation/canary、冷缓存准备全流程及当前相关 workspace gates，仍缺少本次可复核的原始证据。六次保存试验来自 55fbc950，当前涉及这些边界的 production bytes 已变更，不能用旧结果自动证明当前动态边界。
- Evidence: `docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md:244` — Phase 验收除六次实际试验外，还要求正误 witness、真实客户端矩阵、regrade、取消隔离与最终 gates。；`crates/opi-eval/tests/native_driver.rs:1` — 默认 native tests 为 deterministic stand-ins；实际客户端/环境 witnesses 需 external material。；`rust-tests.log` — 309 passed、9 ignored；ignored witnesses 没有在本次执行。；`D:/Luiz/Odradek/phase19-operator/evidence-root/manifest.json` — 六个 live trial 原始目录已找到；manifest 绑定 Opi source 55fbc95009e6f47145196dbc539454ebcb1cf692。；`operator-current-verification.json; operator-raw-summary.json` — 当前 verifier exit 0；独立检查 terminal build identities、live configs、tool/provider events、native outputs，不采纳既往报告结论。；`git diff --stat 55fbc95009e6f47145196dbc539454ebcb1cf692 d915e4a22509c0f60c2651dbccfd054f0abc73d4 -- crates/opi-eval/src crates/opi-eval/integrations` — 22 个相关生产文件变更，包括 Harbor/Pier integration、material、prepare、regrade、native reader/supervision；不能假定旧动态见证覆盖当前实现。
- Refutation attempted: 最强反证是本次新发现的真实 operator 原始目录：两个 Agent 均有工具事件且 reward 合法（TB2.1 均1，TB3/DeepSWE 均0），current verify/report 接受封存证据。因此已将对应六项 preserved-live obligations 改为 met。它仍不能替代故障注入/正误/regrade/cancellation 的独立动态义务，也未证明变更后路径和最终 workspace gates。
- Suggested closure: 补齐当前相关边界缺失的原始动态与 gate 证据；对已有六个 live trial 保留其真实来源和合法零分，不要求为提高分数重跑或否认旧试验。

Reproduction:

1. 检查本次保留的 operator-raw-summary.json 和当前 verifier 结果，再区分保存的六个 live trials 与尚未观察的 native witness/gate。
2. 补齐与当前 relevant source/material identity 绑定的实际三协议 client matrix、正误 native/regrade/cancellation 证据和 final gates；不得只提供过往结论。

## Verification Commands

所有 production checks 在 sealed export 的 WSL Ubuntu 执行，`CARGO_INCREMENTAL=1`，共享外部 cache `/mnt/e/opi/cargo-targets/opi-8abcf38f9871653e` 并使用 lease；没有 cargo clean 或 session target。

| Command | Result | Requirement/finding |
|---|---|---|
| `python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase19` | PASS，开始时 live paths admitted | workflow admission |
| `python scripts/opi-cargo-cache.py status` / WSL `python3 ... status` | exit 0；检查已有外部 cache | cache policy |
| `cargo test -p opi-eval --locked --all-targets` | exit 0；309 passed、0 failed、9 ignored | 当前 Companion 全部默认 targets |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | exit 0；216 tests，OK (skipped=1) | Python regressions；未运行显式 Docker check |
| `python scripts/opi-doc-check.py`（Windows export） | exit 1；WinError 5，exported .claude/skills symlink 不可跟随 | 环境限制，随后改在 WSL 执行 |
| `python3 scripts/opi-doc-check.py`（WSL export） | PASS，exit 0 | documentation contracts |
| `python3 <audit-temp>/reproduce.py` | recovery 2→0/bytes changed；selection plan exit 0、replacement exit 2；最初显式 replacement target 拼错，后续 corrected cases 覆盖 | F005/F006 |
| `python3 <audit-temp>/probe-more.py` | corrected replacements 均 exit 0、identities 相同；真实 acquisition script exit 2。最初 probe rustc 选错缓存依赖而 compile 失败，不作为产品结果 | F001/F006 |
| `cargo test -p opi-eval --locked --test native_driver --no-run --message-format=json` | exit 0，用于获得当前匹配 rlib identities；修正外部 probe compile setup | probe infrastructure |
| `python3 <audit-temp>/compile-probe.py` | 1 exact probe passed；确认 required sampling 与 offline comparability 反例 | F003/F004 |
| `python3 <audit-temp>/compile-privacy.py` | 1 exact probe passed；privacy failure 后第二 trial 仍派发 | F007 |
| `python3 <audit-temp>/compile-acceptance.py` | 1 exact copied existing test passed；保存 synthetic accepted=true evidence | F008 |
| `git merge-base --is-ancestor d915e4a22509c0f60c2651dbccfd054f0abc73d4 HEAD` | exit 0，install 前 HEAD 仍为同一 commit | baseline ancestry |
| workspace fmt/clippy/all-targets/doc-tests/rustdoc gates；ignored real clients/native witnesses；重新发起 live six-trial acceptance | NOT RUN；已独立复核既有六次原始 live 试验，但不能代替这些不同义务/当前 gates | F009 |
| `opi-eval --help` | exit 0；列出 validate/run/verify/regrade/report/plan/doctor/prepare/conformance | CLI contract |
| `python3 crates/opi-eval/scripts/verify-local-acceptance.py --root /mnt/d/Luiz/Odradek/phase19-operator/evidence-root --out <audit-temp>/operator-current-verification.json --companion /mnt/e/opi/cargo-targets/opi-8abcf38f9871653e/debug/opi-eval` | exit 0、accepted=true；当前导出版本执行，未读取既往验收报告 | 保存的六次 live 试验 |
| `python3 <audit-temp>/inspect-operator.py` | exit 0；直接读取 raw terminal、native result 和 provider/tool streams，保存摘要和哈希 | 对 acceptance verifier 的独立复核 |
| `git diff --stat 55fbc95009e6f47145196dbc539454ebcb1cf692 d915e4a22509c0f60c2651dbccfd054f0abc73d4 -- crates/opi-eval/src crates/opi-eval/integrations` | 22 个相关 production files 变更 | 区分旧 live 证据与当前 witness 义务 |

`<audit-temp>` = `C:/Users/Luiz/AppData/Local/Temp/opi-audit-phase19-codex-gpt6-y1s_et7g`（WSL 下为 `/mnt/c/Users/Luiz/AppData/Local/Temp/opi-audit-phase19-codex-gpt6-y1s_et7g`）。外部探针通过 public CLI 运行已编译的 sealed-baseline 代码；测试副本只追加反例或复制中间 artifacts，未改变 production policy。Rustc 初始 setup 错误（rlib features 不配对、相对 include_str 路径）均修正后才采纳结果。没有实际服务调用、付费试验或真实 credential。

本地留存证据（安装成员本身恰好为四文件；下列是独立 temp evidence，不是额外 ledger）：

| Artifact | SHA-256 |
|---|---|
| `rust-tests.log` | `6605a2bca7c69dbaa3bb194bcd70da1cf493214fab1aeaf0e68bfdcc98faf246` |
| `python-tests.log` | `0742335665784f0d0616587300f7225adafae37ace9ae71105d5948e80ed5ffa` |
| `probe-controls.log` | `e2cd628b06f991b95e00c6ffa334220ef8876bb7c85d22391ed4bd8f4d434fda` |
| `probe-privacy.log` | `f15c92741a4bd1e542a2d16619d9d405c502030d7fcefa1753147382efd691c1` |
| `probe-acceptance.log` | `ef02a124f23655112909c250fc95b4f047b56d79e547594044a34631b0b66d30` |
| `operator-current-verification.json` | `3ecb1fb77662e4d0285777b1574a80dc4084dd66812fcccf137c6b74a983647b` |
| `operator-raw-summary.json` | `e3dc29d31c259f420e0a83e4b62d9929c235e3fbdf422d34d057032068b0824d` |
| `inspect-operator.py` | `1e6e140a54d692d777cdfa5f6670f6a3eb771833c074f4b0e6029953e7949fb0` |
| `reproduction/original-run-report.json` | `113d39839946347ac25680d85e8ed96fd63f360ec5c4b7fc73587833dbac3261` |
| `reproduction/initial/run-report.json` | `e7efaed24e1cd905e7c309646ba4483ceaa6b3af5e1ed48044d17e92ff29d7c8` |
| `reproduction/optional-output/offline-report.json` | `609bd15f2691b52f75b278ad8c3b62d03ced6b956961ad915a32d13432c973fa` |
| `reproduction/accepted-fixture-verification.json` | `39a936572cd8b56f9d7d24d5b7c6d8d0c2eaff5bba1d3ad539908438753c0447` |
| `reproduce.py` | `72931895388aa00338690438a00e81ad557974ec708fd488754a20b0891832ab` |
| `probe-more.py` | `954aeb518e2680c6d0ea9f52d2bc0c776ac6e331f8c97e8235831a6fccb9afe8` |
| `compile-probe.py` | `aa804d05a62e0d234f3c2728f14acfd6190b5ff7b1dadd878b45db483380a81e` |
| `compile-privacy.py` | `38df3127f6a12967b995e98479ff271c8f796abab9f7191b2b3bf8e5949fd351` |
| `compile-acceptance.py` | `37aa3496759460aaf664e66e95d9eea336c1e4a92ab484bdc745b6516a518b38` |
| `native-audit-probes.rs` | `479d3dd7aa509737a62f02c500d3d9f9093a4ab37ef060d75a6859b344e3e683` |

原始 operator evidence 位于 `D:/Luiz/Odradek/phase19-operator/evidence-root`，由 current committed ledger 的路径片段定位；没有打开该目录旁的 `reports` 或 repo 中既往验收/audit 文档。当前 verifier 的成功只是结构与 seal 验证证据，另行读取 raw data 以避开 F008 的盲点。

| Family | pi raw evidence | Opi raw evidence | Native reward |
|---|---|---|---|
| TB2.1 openssl-selfsigned-cert | 4 tool_execution_start，terminal exit0，5 delivered identities | 28 provider / 38 tool evidence events，terminal exit0，5 delivered identities | pi=1、Opi=1 |
| TB3 batched-eval-parity | 26 tool_execution_start，terminal exit0 | 22 provider / 50 tool evidence events，terminal exit0 | pi=0、Opi=0；声明的 /app/evalbench/ collection digest 保留 |
| DeepSWE abs-module-cache-flags | 16 tool_execution_start，terminal exit0 | 26 provider / 36 tool evidence events，terminal exit0 | pi=0、Opi=0；两侧保存 empty committed patch，合法零分 |

六个 sealed experiments 均为 `execution.mode=live`、`zhipu-anthropic` / `glm-5.3-flash`；terminal executable SHA 与 operator manifest 一致，native exception 为空。Opi event 数是 evidence events，不冒充独立工具调用数。该材料证明六次保存试验及原始调用/评分存在，不自动证明改动后的 native supervision/regrade 等不同路径。原始 trial source commit 为 `55fbc95009e6f47145196dbc539454ebcb1cf692`；本次审计 baseline 仍为 d915e4a。

Test impact: **retain** — 未修改仓库测试；新增审计探针只在私有临时目录。Production changes: **none**。文档/metadata 修改仅为当前 audit member 和安装器拥有的 index/history。

## Verdict Rationale

机械规则：任一 mandatory obligation 非 met 即 FAIL。本次 71 个 mandatory 中 49 met、14 partially-met、1 not-met、7 not-assessable；9 项 reciprocal Major finding 均 blocks。默认测试通过不能覆盖已重现的组合路径错误，也不能把缺失的实际 native/live 证据升级为通过。

此结论属于本成员；live set aggregate 只以安装后的 `audit-set` validator 为准。不在本报告推断或引用其他成员观点。发现状态按 contract 统一为 `unverified`，表示尚未被 remediation 关闭，不表示反例未执行。
