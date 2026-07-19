# opi 技能（skills）

`.claude/skills/opi-*` 技能构成了 opi 项目的产品生命周期：从 spec 出发，经过实现、
独立审计、修复、运行时回归、文档，到发布。本 README 描述端到端工作流以及每个技能的
使用方式。

共有七个 opi-* 技能：**opi-realign**、**opi-implement**、**opi-audit**、
**opi-remediate**、**opi-eval**、**opi-document**、**opi-release**。它们是彼此独立的产物，边界严格——
每个技能都明确说明自己做什么，以及同样重要地，**拒绝**做什么。

> 范围说明：本 README 仅覆盖七个 `opi-*` 产品技能。本目录下的其他技能
>（`caveman`、`grill-me`、`tdd`、`to-prd`、`prototype` 等）是通用工具，与 opi 生命周期
> 无关。

---

## opi 工作流

生命周期是一条七阶段主干，外加两条侧支。每个阶段都有进入条件和退出关卡（gate），不要
跳过关卡。

| 阶段 | 技能 | 发生什么 |
|---|---|---|
| Phase 0（可选） | `opi-realign` | 对照参考/上游项目做战略再对齐 |
| Phase 1 | 手动 | 撰写 `docs/opi-spec.md`；把该阶段注册进 `opi-implement` |
| Phase 2 | `opi-implement` | `--reinit` 台账；评估其合理性 |
| Phase 3 | `opi-implement` | 逐任务 TDD 循环；每任务 `compact`（操作约定：ultracode + GLM-5.2） |
| Phase 4 | `opi-audit` | 多模型、独立审计员 |
| Phase 5 | `opi-remediate` | 验证 + 修复；循环至通过 |
| （eval 关卡） | `opi-eval` | 运行时回归（真实 provider 金额） |
| Phase 6 | `opi-document` | 文档 + EN/ZH 同步（guard 验证） |
| Phase 7 | `opi-release` | GitHub Releases + crates.io |

### 两条贯穿全局的模式

- **模型独立性。** *评估* 或 *审计* 某个产物的模型应当与 *构建* 它的模型 **不同**。
  Phase 2（台账评估）、Phase 4（审计）、Phase 5（验证）都依赖这一点。目前切换模型是手工
  的——你在阶段之间手动切换 agent/model。自动化是后续工作。
- **带 `compact` 的上下文受限循环。** Phase 3 和 Phase 5 是超出单上下文窗口的循环。模式
  是：做一个工作单元，提交/记录它，**`compact`**（或清空上下文），只重新加载关键内容，对照
  目标（spec / 审计报告）验证，重复直至退出关卡通过。

### Phase 0 —— 战略再对齐（可选，偶尔）

**技能：** `opi-realign`。将 opi 实现与参考 / 上游项目（如 `earendil-works/pi`）对比，
*在* 规划新工作 *之前* 发现架构、功能、设计哲学、包边界、路线图层面的漂移。当 spec 或
路线图需要对照上游做一次现实校验时使用——不必每个周期都跑。

- **进入：** 存在目标项目路径（或你提供一个）。
- **退出：** 带有 P0–P3 优先级的漂移报告；可选的 spec 调整附录进入 Phase 1。

### Phase 1 —— 需求与 spec 撰写（手工，与技能无关）

没有技能负责这一步。用户手工撰写 `docs/opi-spec.md`（以及任何 PRD），然后 **注册** 新的
或变更的阶段工作进 `opi-implement`：Phase 1–4 用 §15 路线图表，Phase 5–14 用
`opi-implement/skill.md` 中已评审的补充来源注册表。迭代 spec 至稳定。

- **进入：** 一个产品需求。
- **退出：** 一个包含成功标准、退出标准、任务路线图的 spec 小节；该阶段工作已注册进
  `opi-implement`。

### Phase 2 —— 台账初始化与评估循环

**技能：** `opi-implement`（`--reinit` 或首次 init）。把 spec 解析成 `.opi-impl-state.json`
任务台账（含推断的 tier / commit 类型 / 依赖、复合行拆分、以及任务图评审关卡）。然后用
**与实现不同的模型** 评估台账的合理性：任务拆解是否对齐 spec？边界是否覆盖清晰？是否有
冗余、遗漏、过度？每个产品成功标准是否都有验收场景归属？优化台账并 re-init，直至图稳定。

- **进入：** Phase 1 注册的 spec。
- **退出关卡：** 任务图评审已确认；每个产品成功标准都映射到归属任务并带验收场景；
  `spec_files` 哈希已固定。

### Phase 3 —— 逐任务实现循环

**技能：** `opi-implement`，反复运行（它会自动挑选 ID 最小的未阻塞任务）。本项目的操作约定
是用 Claude Code **ultracode** + **GLM-5.2** 实现——这是工作流/模型选择，并非技能所编码；
`opi-implement` Phase C 本身组合 `superpowers:test-driven-development`（red-green-refactor），
外加可选并行派发与第 3 次尝试起的 `systematic-debugging`。每个任务跑 harness 阶段 A→F：
bootstrap、plan、用 TDD 实现、verify（分层 gate + Artifact Truthfulness Gate）、带 `Opi-*`
footer 提交、以独立提交检查点化受 Git 跟踪的台账、phase-exit 检查。两个提交完成后
**`compact`**，再继续。

- **进入：** Phase 2 定稿的台账。
- **退出关卡：** 该阶段所有任务 `passing`；phase-exit 评估器把每条成功/退出标准追踪到
  `met` / `deferred-by-updated-design` / `not-met`；阶段归档至 `docs/snapshots/phase<N>/`。
- **关键护栏：** harness 永不 push 提交，永不发布，永不调用 provider API，永不编辑
  `opi-spec.md`，永不通过削弱测试来使其通过，永不执行破坏性 git 操作。

### Phase 4 —— 独立审计（多模型）

**技能：** `opi-audit`。一个或多个 **独立模型** 各自审计已完成的阶段：读快照台账 + spec，
按推断出的维度（Correctness、Security、Test quality、Spec compliance、Invariants、
Integration、Residuals）审计，写出 `docs/snapshots/phase<N>/audit.<model-id>.md`，含
Blocker/Major/Minor/Info 问题集与 PASS / PASS-WITH-FINDINGS / FAIL 结论。

- **进入：** Phase 3 归档的阶段快照。
- **独立性规则：** 审计员在完成自己的报告前，不得阅读该阶段的其他审计报告或完整评估器
  记录。
- **退出关卡：** 至少存在一份审计报告；理想情况是 2+ 份来自不同模型，重叠验证真实问题、
  分歧暴露盲点。

### Phase 5 —— 修复循环（上下文受限）

**技能：** `opi-remediate`。交叉引用全部审计报告，归一化并统一严重度，按共识聚类
（全量 / 多数 / 单一），对照真实代码逐条验证（Confirmed / Partially confirmed /
Cannot confirm / Refuted），产出按依赖分层的 `remediation-plan.md`，并在显式 opt-in 后逐层
执行修复，每层带验证关卡。

由于问题集可能很大且上下文受限，**操作者** 在多个上下文窗口间循环：跑 `opi-remediate`，
**清空上下文**，重新加载审计报告与当前代码，再次验证，重复直至验证结论为通过。单次
`opi-remediate` 调用是按依赖顺序的一次正向遍历（每层由 `cargo fmt`/`clippy`/`test` gate），
最后跑工作区 smoke 脚本。

- **进入：** Phase 4 的审计报告。
- **退出关卡（工作流层面）：** 无未决 Blocker 或 Major；每个问题要么由修复项处理，要么列入
  计划的 Scope exclusions（Refuted / Deferred / Info / Duplicate）。

### （eval 关卡）—— 运行时回归

**技能：** `opi-eval`。发布前跑端到端回归 eval：编译 opi、对真实 LLM provider 跑结构化用
例、收集 NDJSON trace、派发只读评估器、写出 `docs/eval/<version>-<date>-<model>.md` 并追加
`docs/eval/history.jsonl`。它捕捉静态审计无法发现的运行时保真度退化。消耗真实 API 金额，
永不自动触发。

- **进入：** 运行时改动已合并；provider 凭据已配置。
- **退出：** 报告已写；退化（若有）回馈 Phase 5。

### Phase 6 —— 文档与 EN/ZH 同步

**技能：** `opi-document`。刷新 opi 文档，使其与已发布代码保持一致，并保持 EN/ZH 镜像同步——在
八个 doc-guard 套件 **之内** 编辑，而非绕过它们。用于一次完整的阶段文档刷新、一次改动后的定点
更新，或版本号 bump 的文档重同步。

- **进入：** Phase 3–5（实现 / 审计 / 修复）通过；或一次临时的文档 / 翻译请求。
- **退出关卡：** 每个 guard 套件（`productized_packages_docs`、`phase11_tooling_quality_docs`、
  `phase12_provider_correctness_docs`、`phase13_session_context_docs`、`observability_docs`、
  `runtime_contract_docs`、`transport`）EN + ZH 均报告 `0 failed`；无遗留的内部阶段术语；若发生
  版本号 bump，所有含版本号的行已同步更新。
- **关键护栏：** 永不删除 guard 固定的 token；永不引入非否定形态的禁止性过度声明；若触碰
  `docs/opi-spec.md` 则重新同步 phase4 spec-hash 台账；不改代码、`Cargo.toml` 或版本号（那是
  `opi-release` 的职责）；不弱化 guard 测试。

### Phase 7 —— 发布

**技能：** `opi-release <version> [--fix] [--skip-cross]`。跑七阶段发布流水线——pre-flight、
版本号 bump、changelog、构建、提交/打 tag/push + 草稿 GitHub Release、crates.io 发布、发布
草稿 + 验证——发布到 GitHub Releases 和 crates.io。可逆性随进度递减：Phase 1–4 本地可逆，
Phase 5 部分可逆（push 时提交/tag 即公开），Phase 6（crates.io）不可逆。全程有显式用户确认
关卡。

- **进入：** 处于可发布状态的工作区（Phase 3–5 通过，eval 干净）。
- **退出：** 已发布 GitHub Release + crates.io 版本；发布报告。

---

## 共享契约

- **`.opi-impl-state.json`**（Git 跟踪，仓库根）—— `opi-implement` 的规范活动任务台账。
  任务提交与台账检查点提交相互独立；临时、草稿和恢复副本继续忽略。`opi-remediate` 与
  `opi-audit` 只 *读* 它——且两者读的都是 `docs/snapshots/phase<N>/opi-impl-state.json`
  处的 **冻结按阶段快照**，而非仓库根的活动文件。其他技能不写它。
- **`docs/snapshots/phase<N>/`** —— 冻结的按阶段归档：`opi-impl-state.json` 快照、
  `audit.<model-id>.md` 报告、`remediation-plan.md`。
- **`Opi-*` 提交 footer**（`Opi-Task`、`Opi-DoD-SHA256`、`Opi-Verification`、
  `Opi-Evaluator`、`Opi-Acceptance`）—— 使任务完成情况可从 git 历史重建，无需依赖台账。
- **`.opi-release-state.json`**（仓库根）—— `opi-release` 的恢复状态，与实现台账相互独立。
- **`docs/eval/`** —— `opi-eval` 报告与 `history.jsonl`。

---

## 各技能参考

### opi-realign

将当前实现与目标 / 参考项目对比，产出架构、功能、设计哲学、包边界、路线图层面的再对齐评审。

- **何时调用：** "realign"、"audit drift"、"compare a port/reimplementation"、"check whether
  planned phases match an upstream project"、"evaluate cross-language architecture against a
  target project path"，或提供一个目标项目路径以供对比。
- **输入：** `target=<path>`（必需；省略则询问）。可选：`current=<path>`、`current_label`、
  `target_label`、`scope=<text>`。
- **做什么：** 为两个项目构建证据清单；对比 *语义*（而非文件形状），覆盖架构、运行时、数据
  格式、provider/集成面、扩展模型、测试、文档；归类漂移（Aligned / Intentional divergence /
  Partial / Missing / Overreach / Risk）；给出 P0–P3 调整建议；大型审计写本地报告文件，在
  对话里只汇总最高信号项。
- **不做什么：** 无证据不声称兼容；不把目标项目的广度当作必然 desirable；不在与当前语言规范
  冲突时照搬目标语言架构；未经显式要求不修改源码/spec/路线图、不提交。
- **产物：** 读两项目的指南文件、manifest、源码拓扑、测试、路线图产物；写报告文件（HTML 或
  markdown），仅在你要求修改时才写 spec 文件 + spec 调整附录。
- **在工作流中：** Phase 0（可选侧支）。

### opi-implement

长期运行 agent harness，逐任务驱动 `docs/opi-spec.md` 任务与已评审的 Phase 5–14 补充 spec
的实现，使用 TDD、分层验证、文档 guard、JSON 台账 checkpoint。它是一个 **harness**，不是编码
助手——它固化了关于状态、证据、失败恢复、升级的规则，若任何规则将被违反则拒绝执行。

- **何时调用：** "implement"、"resume"、"verify"、"progress"，或查询状态、重新初始化台账、
  恢复中断的实现、清除阻塞、自动选下一个未阻塞任务。不用于仅仅阅读或讨论 spec。
- **输入 / 命令：**
  - `opi-implement` —— 自动挑选下一个未阻塞任务。
  - `opi-implement <task-id>` —— 跑指定任务（校验依赖）。
  - `opi-implement --status` —— 台账摘要（任务表、阶段、漂移、阻塞）。
  - `opi-implement --reinit` —— 重新解析 spec 并对账台账。
  - `opi-implement <task-id> --resume-from-manual` —— 验证一次手工提交。
  - `opi-implement <task-id> --extend-cap <N>` —— 调高迭代上限。
  - `opi-implement --clear-blocker <id> --because <text>` —— 解除阻塞。
  - 需要 `cargo`（Rust ≥ 1.97）与 `git`。
- **做什么：** 每任务六阶段——A Bootstrap、B Plan（打印 DoD + tier + 验收场景 + 调用点 +
  禁止范围 guard，用户 gate）、C Implement（TDD red-green-refactor，可选并行派发，第 3 次尝试
  起用 systematic debugging）、D Verify（产品验收 D.0、Artifact Truthfulness Gate D.0a、分层
  gate D.1、风险评估器 D.2、横切 gate D.3）、E 任务提交 + 独立的受跟踪台账检查点
  （Conventional 任务提交带 `Opi-*` footer）、F Phase-Exit 检查。init/reinit 时推断任务元数据；
  强制 spec 哈希对齐 guard；
  跑分层验证——**六个 tier**（workspace / documentation / library / cli-tool / cli-runtime /
  tui）外加叠加在其上的**条件 addenda**（provider-contract、multimodal、product acceptance）。
- **不做什么：** 不编辑 `opi-spec.md`（除非一个已评审、拥有它的文档任务）、不 push 提交/tag、
  不发布或开 PR/release、不调用 provider API、不删除或削弱测试、不 crate 级 bypass clippy、不
  自动接受 TUI 快照、不运行 `git restore`/`clean`/`reset`/`--no-verify`/`--force`/
  `git add -A`、不以 stub 或 TODO 满足 DoD。
- **产物：** 读 `docs/opi-spec.md` §15 + 已评审 Phase 5–14 来源；写受跟踪的
  `.opi-impl-state.json`、忽略的临时台账文件、`docs/snapshots/phase<N>/` 下的阶段快照、
  带 `Opi-*` footer 的任务提交和独立的台账检查点提交。
- **在工作流中：** Phase 2（init）与 Phase 3（实现循环）。
- **备注：** 全工作区 smoke 开销大且曾撑满本机磁盘——库层任务优先用每任务 library gate 加
  `CARGO_INCREMENTAL=0`。本 Windows 主机用 `python` 而非 `python3`。

### opi-audit

对指定 opi 实现阶段做独立、阶段级代码审计：对照设计 spec 与实际实现，产出带严重度分类的
结构化问题报告。

- **何时调用：** "audit"、"code review"、"review phase N"、"compare spec and implementation"、
  "check spec compliance"、"find implementation gaps"，或中文触发词 审计 / 审查。
- **输入：** `phase=<N>`（必需；省略则询问）。可选：`focus=<text>`，在仍覆盖基础项的同时加权
  指定维度。
- **做什么：** 读 **快照** 台账 `docs/snapshots/phase<N>/opi-impl-state.json`（非活动文件）及
  其引用的 spec 文件；推断适用审计维度并与用户简短确认（含焦点区域与增删）；完整读相关源/测试/
  文档（大阶段建议并行子 agent）；逐维度审计；按 Blocker / Major / Minor / Info 分类；写
  `docs/snapshots/phase<N>/audit.<model-id>.md`，含执行摘要与 PASS / PASS-WITH-FINDINGS /
  FAIL 结论。
- **不做什么：** 未经要求不修改代码/spec/测试/文档（是审计，不是修复）；完成前不读该阶段其他
  `audit.*.md` 报告或完整评估器记录——`phase_exit` 中结构性的 `evaluator_summary` 字段是唯一
  例外（独立性）；不因存在其他报告而降低深度；不把每条 spec 偏差都当作缺陷。
- **产物：** 读快照台账 + spec 文件 + `CLAUDE.md`/`AGENTS.md` + `docs/opi-spec.md`；写
  `audit.<model-id>.md`。
- **在工作流中：** Phase 4。
- **备注：** 文件名中的 `<model-id>` 由审计模型身份自行确定（如 `opus4.6`、`codex`、
  `glm5.2`、`gpt5.5`），不确定则询问。同时支持快照 schema v1（`spec_path`）与 v2
  （`spec_files`）。

### opi-remediate

交叉引用、对照真实代码验证、并修复独立审计报告中的问题，产出带可选用户 gated 执行的分层
修复计划。

- **何时调用：** "remediate phase N"、"verify audit findings"、"fix audit issues"、"confirm
  audit"，或中文触发词 修复审计 / 验证审计发现 / 审计修复，或对 `docs/snapshots/phase<N>/
audit.*.md` 采取行动的请求。
- **输入：** `phase=<N>`（必需）。可选：`scope=<text>`；`execute=<bool>`（默认 `false`——仅出
  计划；`true` 或审阅后 opt-in 时继续执行）。
- **做什么：** Phase A 获取（全部 `audit.*.md` + 快照台账 + spec）；Phase B 交叉引用（归一化
  问题、统一严重度、按共识聚类；仅一份审计时进入单报告模式）；Phase C 逐条对照完整源文件验证
  （Confirmed / Partially confirmed / Cannot confirm / Refuted）；Phase D 设计决策（显然者自动
  决断，其余带标签选项上报）；Phase E 推导按依赖分层的 `remediation-plan.md`；Phase F（gated）
  逐层执行修复，每层带 `cargo fmt`/`clippy`/`test` gate，最后跑工作区 smoke。
- **不做什么：** 不写 `.opi-impl-state.json`；不在问题范围外重构或改进代码；不加功能；不运行
  `git reset --hard`/`checkout .`/`clean -fd`/`add -A`；前层连续两次修复失败时不推进下一层；
  Phase F 永不自动执行。
- **产物：** 读 `audit.*.md` + 快照台账 + spec + `cargo metadata`；写 `remediation-plan.md`；
  Phase F 仅修改经验证问题指向的源/测试/文档文件。
- **在工作流中：** Phase 5。
- **备注：** 文档若有 `.zh.md` 对应版本，EN 与 ZH 必须同一变更内一起更新。

### opi-eval

opi 运行时的端到端回归 eval：编译 opi、对真实 LLM provider 跑结构化用例、收集 NDJSON 运行时
trace、派发只读评估器子 agent 以检测保真度退化。

- **何时调用：** 仅用户显式调用——frontmatter 设 `disable-model-invocation: true`，且技能消耗
  真实 API 金额。自然触发词："eval"、"regression"、"runtime fidelity"。
- **输入：** `model=<provider:model>`（可选；默认 opi 的默认解析；始终记录）；`cases=<name,...>`
  或 `all`（默认）。需要真实 provider 凭据。
- **做什么：** Step 1 release 模式 clean-build `opi-coding-agent`；Step 2 在隔离 temp 工作区跑
  每个用例并捕获 `output.ndjson`；Step 3 解析信号（工具调用、压缩、重试、最终答案、token、
  cost、诊断）；Step 4 派发 **只读** 评估器，按六维（答案正确性、工具调用正确性、上下文完整
  性、链路效率、资源消耗、错误处理）打分；Step 5 写 `docs/eval/<version>-<date>-<model-short>.md`
  并追加 `docs/eval/history.jsonl`，含版本差分小节。内置三个用例（`candy`、`tool_chain`、
  `context_retention`）。
- **不做什么：** 不修改 opi 源码；不在评估器子 agent 中执行任何东西；不把 fixture 写进工作区
  根；不建议代码改动（仅诊断）；无显式调用不触发；单用例崩溃不中止整个 eval。
- **产物：** 写 `docs/eval/<version>-<date>-<model-short>.md` 与 `docs/eval/history.jsonl`；读
  `docs/eval/history.jsonl` 与可选的 `pi-baseline.jsonl`。
- **在工作流中：** eval 关卡（侧支，发布前）。
- **备注：** 任一用例首次运行建立资源基线且不能失败；只有后续运行才按 1.5×/3× 阈值打分。新增
  用例只需在 `test-cases.md` 追加 `## Case N:` 小节。

### opi-document

刷新 opi 文档，使其与已发布代码保持一致，同时保持 EN 与简体中文镜像同步、doc-guard 套件全
绿。这是专门的 Phase 6 技能（此前为手动）。

- **何时调用：** "update the docs/README"、"refresh the README"、"sync EN/ZH"、"translate the
  README"、"fix doc drift"，中文触发词 文档更新 / 文档同步 / 更新 README / 翻译，或 opi 工作流
  的 Phase 6（实现之后、发布之前）。
- **输入：** `scope=<full|targeted|version-bump>`（默认 targeted）；`files=<...>`；版本号 bump 时
  `version=<X.Y.Z>`。
- **做什么：** 七阶段——发现文档 delta；对照源码审计漂移 / 噪声 / 缺口；载入 doc-guard 约束；
  决定 guard-safe 范围；编辑 EN 文档；外科式镜像到 ZH（对新增文案组合 `baoyu-translate`，逐字
  保留固化的 ZH token）；跑八个 guard 套件加一次阶段术语 grep 来验证。
- **不做什么：** 不改代码 / `Cargo.toml` / 版本号；不提交或发布；不撰写 `opi-spec.md` 规范内容
  （只做 doc-sync 编辑，且会重新同步 phase4 台账）；不弱化 guard 测试；不整篇重生成中文文档。
- **产物：** 读相关文档 + `CHANGELOG.md` + crate `src/` + guard 测试文件 + baoyu-translate
  `EXTEND.md`；写文档（EN + ZH），并在编辑 `opi-spec.md` 时重新同步 phase4 台账哈希。
- **在工作流中：** Phase 6。
- **备注：** doc-guard 约束位于 `opi-document/references/doc-guards.md`。

### opi-release

编排 opi Rust 工作区的完整发布流程——通过七个带安全 gate 的阶段发布到 GitHub Releases 和
crates.io，每个阶段都需要用户确认。

- **何时调用：** "release <version>"、"opi-release <version>"、"ship version <version>"、
  "publish opi <version>"。
- **输入：** `<version>`（必需 semver）；`--fix`（pre-flight 时自动修 fmt/clippy）；
  `--skip-cross`（仅源码发布）。需要 `main` 上干净的工作区、`cargo`/`git`/`gh`、以及 crates.io
  认证（`~/.cargo/credentials.toml` 或 `$CARGO_REGISTRY_TOKEN`）。
- **做什么：** Phase 1 pre-flight（文件、git 状态、CI、fmt/clippy/test/doc、audit、密钥扫描、
  元数据、包内容、版本语义、`--version` 命令）；Phase 2 bump 工作区版本 + 内部依赖版本 + 干跑
  发布；Phase 3 从 Conventional Commits 生成 CHANGELOG + release notes；Phase 4 构建
  （推荐 CI 驱动，或本地 `cross`，或 `--skip-cross`）+ 打包产物 + 校验和 + 自检；Phase 5 提交/
  tag/push + 草稿 GitHub Release（仅 stage `Cargo.toml`/`Cargo.lock`/`CHANGELOG.md`）；Phase 6
  按拓扑序发布到 crates.io；Phase 7 发布草稿 + 验证安装。通过 `.opi-release-state.json` 支持
  恢复。
- **不做什么：** 不跑实时 provider/dogfood 检查（pre-flight 是确定性的）；不上传
  `SHA256SUMS.txt` 到 release；不自动重试显式 cargo 错误；未经批准不自动进入 crates.io；不硬
  编码发布顺序；不用 `git reset --hard` + force-push 回滚（用 `git revert` + 删 tag）；不把
  yank 当作删除。
- **产物：** 读写 `Cargo.toml`、`Cargo.lock`、`CHANGELOG.md`、release notes、
  `release-artifacts/v$VERSION/`、`.opi-release-state.json`、release 提交 + tag、GitHub Release、
  crates.io 版本。
- **在工作流中：** Phase 7（终点）。
- **备注：** 不可逆边界——push 时提交/tag 即公开（Phase 5）；crates.io 发布永久（Phase 6）。本
  主机上 `git push`/`cargo publish` 可能需要重试（代理上 SSL 掉线）；用 `git ls-remote` 而非
  `gh api` 验证 push。

---

## 缺口与后续工作

- **模型 / agent 切换** 在阶段之间（Phase 2 评估、Phase 4 审计、Phase 5 验证）目前是手动的。
  自动化多模型编排是后续工作。
