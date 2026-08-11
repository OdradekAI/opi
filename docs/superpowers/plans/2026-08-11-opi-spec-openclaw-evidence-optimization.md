# Opi Spec OpenClaw Evidence Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Strengthen the bilingual Opi parent specification with durable resolved-execution, benchmark-integrity, authority, Package Trust, and C1 evidence contracts while preserving its existing design doctrine and stable clause identity.

**Architecture:** Revise existing `CTRL`, `INV`, gate, and `STRAT` text instead of adding chapters or clause identifiers. Keep Agent Core propagation minimal, place Package Trust in the Reference Product, and protect the new durable semantics with focused bilingual documentation-contract checks.

**Tech Stack:** Markdown, Python 3 standard-library `unittest`, `scripts/opi-doc-check.py`.

---

## File map

- Modify `docs/opi-spec.md`: normative English clauses and evolution route.
- Modify `docs/opi-spec.zh.md`: identifier-preserving equivalent Chinese clauses and route.
- Modify `scripts/opi-doc-check.py`: require a small set of durable semantic tokens in both parent specifications.
- Modify `scripts/test_opi_doc_check.py`: prove every new semantic token is mechanically required.
- Reference only `docs/superpowers/specs/2026-08-11-opi-spec-openclaw-evidence-optimization-design.md`: approved design; do not edit.
- Reference only `docs/research/2026-08-11-openclaw-agent-practices-opi-spec.md`: non-normative evidence; do not edit.
- Do not modify `docs/CONTEXT.md`: the existing terms already cover Active Snapshot, User Policy, Capability Permission, Permission Scope, and Package Trust.

### Task 1: Add a bilingual semantic regression contract

**Files:**
- Modify: `scripts/test_opi_doc_check.py:10-328`
- Modify: `scripts/opi-doc-check.py:15-170`
- Modify: `scripts/opi-doc-check.py:684-696`

- [ ] **Step 1: Confirm the pre-change documentation baseline**

Run:

```powershell
python scripts/opi-doc-check.py
python -m unittest scripts.test_opi_doc_check -v
```

Expected: `opi documentation contracts: PASS` and fourteen passing unit tests.

- [ ] **Step 2: Add the required-token fixture and failing tests**

Add this constant after `EVAL_BEHAVIOR_BASELINE_REQUIRED` in `scripts/test_opi_doc_check.py`:

```python
OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED = {
    "docs/opi-spec.md": (
        "resolved execution",
        "benchmark integrity",
        "model-visible content",
        "exact immutable package artifact digest",
        "ordinary-context/no-memory baseline",
        "Proactive or scheduled Agent behavior",
        "Multi-Agent orchestration",
    ),
    "docs/opi-spec.zh.md": (
        "已解析执行",
        "基准完整性",
        "模型可见内容",
        "精确的不可变 package artifact digest",
        "普通上下文/无记忆基线",
        "主动式或定时 Agent 行为",
        "多 Agent 编排",
    ),
}
```

Add this fixture method after `write_eval_behavior_baseline_docs()`:

```python
    def write_opi_spec_evidence_refinement_docs(self) -> None:
        for rel, tokens in OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")
```

Add these tests after the Eval behavior-baseline contract tests:

```python
    def test_opi_spec_evidence_refinement_contract_passes(self) -> None:
        self.write_opi_spec_evidence_refinement_docs()
        checker = getattr(
            doc_check,
            "check_opi_spec_evidence_refinement_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi spec evidence checker must exist")
        checker()
        self.assertEqual([], doc_check.ERRORS)

    def test_opi_spec_evidence_refinement_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_opi_spec_evidence_refinement_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi spec evidence checker must exist")
        for rel, tokens in OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_opi_spec_evidence_refinement_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token)
                        + "\n",
                    )
                    checker()
                    self.assertIn(
                        f"{rel}: Opi spec evidence refinement contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )
```

- [ ] **Step 3: Run the new tests and confirm the missing checker**

Run:

```powershell
python -m unittest scripts.test_opi_doc_check.SkillContractTests.test_opi_spec_evidence_refinement_contract_passes scripts.test_opi_doc_check.SkillContractTests.test_opi_spec_evidence_refinement_requires_every_token -v
```

Expected: both tests fail with `Opi spec evidence checker must exist`.

- [ ] **Step 4: Add the checker implementation**

Add this constant next to the other semantic contract constants in `scripts/opi-doc-check.py`:

```python
OPI_SPEC_EVIDENCE_REFINEMENT_CONTRACT = {
    "docs/opi-spec.md": (
        "resolved execution",
        "benchmark integrity",
        "model-visible content",
        "exact immutable package artifact digest",
        "ordinary-context/no-memory baseline",
        "Proactive or scheduled Agent behavior",
        "Multi-Agent orchestration",
    ),
    "docs/opi-spec.zh.md": (
        "已解析执行",
        "基准完整性",
        "模型可见内容",
        "精确的不可变 package artifact digest",
        "普通上下文/无记忆基线",
        "主动式或定时 Agent 行为",
        "多 Agent 编排",
    ),
}
```

Add this function beside the other semantic contract checkers:

```python
def check_opi_spec_evidence_refinement_contract() -> None:
    for rel, tokens in OPI_SPEC_EVIDENCE_REFINEMENT_CONTRACT.items():
        require_tokens(
            rel,
            "Opi spec evidence refinement contract",
            tokens,
        )
```

Call it from `main()` immediately before `check_top_level_spec()`:

```python
    check_opi_spec_evidence_refinement_contract()
    check_top_level_spec()
```

- [ ] **Step 5: Run the focused unit tests**

Run the command from Step 3 again.

Expected: both tests pass. Do not run the repository documentation checker until Tasks 2 and 3 have supplied the protected text.

### Task 2: Strengthen the normative English specification

**Files:**
- Modify: `docs/opi-spec.md:228-280`
- Modify: `docs/opi-spec.md:318-413`
- Modify: `docs/opi-spec.md:477-507`
- Modify: `docs/opi-spec.md:539-594`

- [ ] **Step 1: Strengthen resolved-execution provenance in Section 6.1**

Replace the evidence paragraph before the `CTRL-001` table with:

```markdown
Evidence artifacts are immutable and content-addressed. A reproducible claim
binds evidence to the resolved execution that produced it: harness, runtime,
adapter, and material configuration identity; requested and actual model route;
effective policy and snapshot; trigger provenance; and measurement origin.
Exact field schemas remain with their authoritative evidence contracts.

Missing measurements remain `unknown` with their reason; they are never
silently converted to zero. Estimated, provider-reported, quota, and billed
costs remain distinguishable. Sensitive prompts, tool arguments, results, and
environment data require explicit capture and redaction policy.
```

Replace `CTRL-002` with:

```markdown
| CTRL-002 | Evidence **MUST** retain resolved harness/runtime/adapter identity and configuration, requested and actual provider/model/wire/authentication/fallback route, source, permission, time, environment, prompt, tool, budget, trigger, measurement origin, and artifact provenance sufficient for offline verification. | Evidence Producer | Manifest/schema validation and offline recomputation of the resolved execution. |
```

- [ ] **Step 2: Add benchmark integrity without moving benchmark ownership**

Insert this paragraph before the `CTRL-004` table:

```markdown
Native graders own benchmark outcome semantics; they do not prove benchmark
integrity. Evaluation admits and retires benchmark revisions through an
integrity record that distinguishes valid Agent outcomes from broken,
unsatisfiable, ambiguous, prompt/test-misaligned, and infrastructure-failed
trials. Coverage and every exclusion reason remain visible.
```

Replace `CTRL-004` and `CTRL-005` with:

```markdown
| CTRL-004 | Headline benchmark results **MUST** come from the benchmark's native grader under an admitted benchmark revision; an LLM judge **MAY** provide a separately labelled diagnostic only. | Evidence Producer | Grader provenance, benchmark integrity record, and report-schema validation. |
| CTRL-005 | Baseline and candidate runs **MUST** be paired by task and trial under one frozen manifest; missing pairs or telemetry, exclusions, task-integrity decisions, and infrastructure-failure classifications **MUST** remain visible. Invalid or infrastructure-failed trials **MUST NOT** be scored as Agent success or failure. | Evaluation orchestrator | Pairing, coverage, adjudication, and failure-classification checks. |
```

- [ ] **Step 3: Make model-visible data non-authoritative**

Insert this paragraph in Section 6.4 after the paragraph beginning `User Policy sets hard limits`:

```markdown
Content from a tool, retrieval adapter, channel, memory item, extension package,
or another Agent remains untrusted for authority even when it becomes
model-visible content. A model, classifier, prompt label, or risk score may deny,
mark risk, or escalate to Human Authority; it cannot grant permission, weaken
policy, or widen scope.
```

Replace `INV-005` with:

```markdown
| INV-005 | Authority, capability, scope, and schema validation **MUST** derive from User Policy and trusted runtime state and complete before a tool causes side effects; model-visible content or a model decision **MUST NOT** grant permission, weaken policy, or widen scope. | Agent runtime owner | Negative source-to-sink permission, capability, scope, and schema tests. |
```

- [ ] **Step 4: Bind finalized evidence to the effective execution state**

Replace `INV-008` with:

```markdown
| INV-008 | Finalized run evidence **MUST** identify the session branch, Active Snapshot, resolved harness/runtime/adapter configuration, and effective User Policy that produced it. | Agent runtime owner | Artifact-schema, resume/fork, and offline resolved-execution tests. |
```

- [ ] **Step 5: Make Package Trust object-specific without adding a Core permission system**

Insert this paragraph after `Neither implies the other.` in Section 7.5:

```markdown
Package Trust is object-specific: it binds an exact immutable package artifact
digest and declared capability footprint. A changed artifact or expanded
footprint remains Installed but is a new trust object; it does not automatically
inherit Trusted or affected Capability Permission. Signatures, scans, registry
provenance, and review results are evidence for the user's decision, not sources
of Trust. The declared footprint remains metadata and does not enforce an
operating-system sandbox.
```

Replace `INV-009` with:

```markdown
| INV-009 | Installed, Trusted, Enabled, Selected, and Permitted **MUST** remain independently observable and enforceable lifecycle states. Package Trust **MUST** bind to an exact immutable package artifact digest and declared capability footprint; a changed artifact or expanded footprint **MUST NOT** inherit Trusted or affected Capability Permission without user reauthorization. | Reference Product owner | Package install/update, footprint-expansion, execution-routing, and lifecycle-diagnostic tests. |
```

- [ ] **Step 6: Strengthen C1 controls without selecting a memory implementation**

Replace the memory/skill ablation bullet in Section 8.4 with:

```markdown
- source episode, ownership, permission snapshot, expiry, contradiction, and
  withdrawal state for memory and skill candidates;
- an ordinary-context/no-memory baseline and a no-learning ablation for memory
  and skill candidates;
```

Replace the Evidence promotion gate with:

```markdown
| Evidence | Resolved manifest, digests, adapter conformance, benchmark-revision admission, source deduplication, privacy scan, holdout isolation, failure classification, and offline recomputation all pass. |
```

- [ ] **Step 7: Adjust strategic emphasis and parallel routes**

Replace the `STRAT-001` body with:

```markdown
Give runtime provider dispatch real ownership; make next-turn state replacement
atomic and correctly ordered; establish the minimum product-neutral evidence
and observability seam for resolved-execution provenance; and validate authority
before side effects. Adding more catalogue entries is lower priority than making
the existing abstraction true at runtime.
```

Add this sentence to the first `STRAT-002` paragraph:

```markdown
Benchmark integrity admission, retirement, coverage, and task/infrastructure
failure classification precede any Learning or Promotion claim that consumes
the report.
```

Replace the `STRAT-004` body with:

```markdown
Validate episodic memory before reusable skills, shadow before activation, and
retention/privacy/withdrawal before scale. Bind candidates to source and
permission state, and compare them with an ordinary-context/no-memory baseline
and no-learning ablation. The current knowledge/learning research document
remains evidence, not an implementation specification.
```

Add these bullets to `Parallel routes` before model-weight training:

```markdown
- Proactive or scheduled Agent behavior may be explored through the Reference
  Product or Extension Ecosystem only with trigger provenance, snapshot/policy
  binding, budget, interruption/delivery policy, and dedicated Eval; it does not
  create a Gateway or scheduler seam in Agent Core.
- Multi-Agent orchestration and Agent-to-Agent protocols remain Extension
  Ecosystem or Independent Companion experiments until real consumers, shared
  conformance, and frozen evaluation evidence justify a Placement Review.
```

- [ ] **Step 8: Confirm the English document satisfies only its half of the new contract**

Run `python scripts/opi-doc-check.py`.

Expected: non-zero exit with one semantic-token error for `docs/opi-spec.zh.md`; the English token set must not appear in the error.

### Task 3: Apply the equivalent Chinese specification revision

**Files:**
- Modify: `docs/opi-spec.zh.md:181-216`
- Modify: `docs/opi-spec.zh.md:244-304`
- Modify: `docs/opi-spec.zh.md:352-374`
- Modify: `docs/opi-spec.zh.md:392-427`

- [ ] **Step 1: Add equivalent resolved-execution and benchmark-integrity text**

Use the same placement as Task 2:

```markdown
证据制品不可变且按内容寻址。可复现声明会把证据绑定到产生它的已解析执行：harness、runtime、adapter 及实质配置身份，请求与实际模型路由，有效策略与快照，触发来源，以及度量来源。精确字段 schema 保留在其权威证据契约中。

缺失度量连同原因保持为 `unknown`，绝不能静默转换为零。估算成本、Provider 报告成本、quota 和实际账单保持可区分。敏感提示词、工具参数、结果和环境数据需要显式的采集与脱敏策略。

| CTRL-002 | 证据**必须（MUST）**保留已解析的 harness/runtime/adapter 身份与配置、请求与实际 Provider/model/wire/authentication/fallback 路由，以及足以离线验证的来源、权限、时间、环境、prompt、工具、预算、触发、度量来源和制品来源信息。 | 证据生产者 | manifest/schema 验证和已解析执行的离线重计算。 |

原生 grader 拥有基准结果语义，但不证明基准完整性。Eval 通过完整性记录准入和停用基准修订，并将有效 Agent 结果与损坏、不可解、歧义、prompt/test 不一致及基础设施失败的 trial 区分开。覆盖率和每项排除理由保持可见。

| CTRL-004 | 对外发布的基准主结果**必须（MUST）**来自已准入基准修订的原生 grader；LLM 裁判**可以（MAY）**仅提供单独标记的诊断信息。 | 证据生产者 | grader 来源、基准完整性记录和报告 schema 验证。 |
| CTRL-005 | 基线运行和候选运行**必须（MUST）**在同一份冻结 manifest 下按任务和 trial 配对；缺失配对或 telemetry、排除项、任务完整性裁决和基础设施失败分类**必须（MUST）**保持可见。无效或基础设施失败的 trial **不得（MUST NOT）**计为 Agent 成功或失败。 | Eval 编排器 | 配对、覆盖率、裁决和失败分类检查。 |
```

- [ ] **Step 2: Add equivalent authority and Active Snapshot requirements**

```markdown
来自工具、retrieval adapter、channel、memory item、扩展 package 或其他 Agent 的内容，即使成为模型可见内容，也仍不具有权威可信性。模型、classifier、prompt label 或 risk score 可以拒绝、标记风险或升级到人类权威；它不能授予 permission、削弱策略或扩大 scope。

| INV-005 | 权威、capability、scope 和 schema 验证**必须（MUST）**仅从 User Policy 与可信 runtime state 导出，并在工具产生副作用前完成；模型可见内容或模型决策**不得（MUST NOT）**授予 permission、削弱策略或扩大 scope。 | Agent 运行时责任方 | source-to-sink permission、capability、scope 和 schema 负向测试。 |

| INV-008 | 最终运行证据**必须（MUST）**标识生成它的会话分支、Active Snapshot、已解析 harness/runtime/adapter 配置和有效 User Policy。 | Agent 运行时责任方 | 制品 schema、resume/fork 和已解析执行离线测试。 |
```

- [ ] **Step 3: Add equivalent object-specific Package Trust text**

```markdown
Package Trust 以对象为单位：它绑定精确的不可变 package artifact digest 和声明的 capability footprint。artifact 变化或 footprint 扩大后仍保持 Installed，但会形成新的信任对象；它不会自动继承 Trusted 或受影响的 Capability Permission。签名、扫描、registry 来源和评审结果只作为用户决策的证据，不产生 Trust。声明的 footprint 仍是 metadata，不执行操作系统 sandbox。

| INV-009 | Installed、Trusted、Enabled、Selected 和 Permitted **必须（MUST）**保持为可独立观测和强制执行的生命周期状态。Package Trust **必须（MUST）**绑定精确的不可变 package artifact digest 和声明的 capability footprint；变化的 artifact 或扩大的 footprint **不得（MUST NOT）**在没有用户重新授权时继承 Trusted 或受影响的 Capability Permission。 | 参考产品责任方 | package 安装/更新、footprint 扩张、执行路由和生命周期诊断测试。 |
```

- [ ] **Step 4: Add equivalent C1 baseline and Evidence-gate text**

```markdown
- 记忆和技能候选项的来源 episode、所有者、permission snapshot、expiry、contradiction 和 withdrawal 状态；
- 记忆和技能候选项的普通上下文/无记忆基线及无学习消融；

| Evidence | 已解析 manifest、digest、adapter 一致性、基准修订准入、来源去重、隐私扫描、留出集隔离、失败分类和离线重计算全部通过。 |
```

- [ ] **Step 5: Apply equivalent strategic route adjustments**

```markdown
赋予运行时 Provider 分派真正的所有权；使下一轮状态替换具有原子性并保持正确顺序；为已解析执行的来源追溯建立产品中立的最小证据与可观测性 seam；并在副作用前验证权威。让现有抽象在运行时名副其实，优先级高于增加更多目录条目。

基准完整性准入与停用、覆盖率检查，以及任务/基础设施失败分类，先于任何使用报告的学习或晋级声明。

先验证情景记忆，再验证可复用技能；先进行 shadow，再激活；先验证保持性/隐私/撤回，再扩大规模。候选项绑定来源和 permission 状态，并与普通上下文/无记忆基线及无学习消融比较。当前的知识/学习研究文档仍是证据，而非实施规范。

- 主动式或定时 Agent 行为只能在绑定触发来源、快照/策略、预算、打断/交付策略和专用 Eval 后，通过参考产品或扩展生态进行探索；它不会在 Agent 核心中建立 Gateway 或 scheduler seam。
- 多 Agent 编排和 Agent-to-Agent 协议仍属于扩展生态或独立伴生产品实验，直到真实消费者、共享合规验证和冻结评测证据足以支持归属复审。
```

- [ ] **Step 6: Run the full documentation contract**

Run `python scripts/opi-doc-check.py`.

Expected: `opi documentation contracts: PASS`.

### Task 4: Verify clause identity, semantic equivalence, and scope

**Files:**
- Verify: `docs/opi-spec.md`
- Verify: `docs/opi-spec.zh.md`
- Verify: `scripts/opi-doc-check.py`
- Verify: `scripts/test_opi_doc_check.py`

- [ ] **Step 1: Run all documentation-checker unit tests**

Run:

```powershell
python -m unittest scripts.test_opi_doc_check -v
```

Expected: sixteen tests pass, including both Opi spec evidence-refinement tests.

- [ ] **Step 2: Verify stable identity and prohibited-content constraints**

Run:

```powershell
python scripts/opi-doc-check.py
rg -n "^(## [0-9]+\.|### (AUTH|GOAL|PRIN|PLACE|CAP|CTRL|INV|GATE|STRAT|PHASE)-)|\b(AUTH|GOAL|PRIN|PLACE|CAP|CTRL|INV|GATE|STRAT|PHASE)-[0-9]{3}\b" docs/opi-spec.md docs/opi-spec.zh.md
rg -n -i "\b(T[B]D|TO[D]O)\b|^#{2,6}\s+.*(progress|status|进度|状态)\s*$" docs/opi-spec.md docs/opi-spec.zh.md
```

Expected: documentation contracts pass; both documents preserve the same eleven chapter headings and sixty-one stable identifiers in the same order; the prohibited-content scan has no matches attributable to the revision.

- [ ] **Step 3: Review the focused diff against the approved design**

Run:

```powershell
git diff --check -- docs/opi-spec.md docs/opi-spec.zh.md scripts/opi-doc-check.py scripts/test_opi_doc_check.py
git diff -- docs/opi-spec.md docs/opi-spec.zh.md scripts/opi-doc-check.py scripts/test_opi_doc_check.py
```

Confirm from the diff:

- Agent Core still propagates only minimal correlation and finalized-artifact references;
- Package Trust remains Reference Product policy and declares no sandbox;
- no Gateway, scheduler, multi-Agent coordinator, memory store, or A2A protocol enters Agent Core;
- English and Chinese clauses carry equivalent owners and verification routes; and
- no unrelated dirty-worktree file is staged.

- [ ] **Step 4: Commit only the approved specification revision**

Run:

```powershell
git add -- docs/opi-spec.md docs/opi-spec.zh.md scripts/opi-doc-check.py scripts/test_opi_doc_check.py
git diff --cached --check
git diff --cached --stat
git commit -m "docs: strengthen opi evidence and trust invariants"
```

Expected: one commit containing only the two parent specifications and their documentation-contract implementation/tests. Preserve every unrelated existing working-tree change.
