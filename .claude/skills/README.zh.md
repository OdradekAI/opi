# Opi 技能

`.claude/skills/opi-*` 下的十个技能共同构成项目工作流，但不会把产品探索
强行变成机械流水线。

核心设计理念是：

> 遵循 pi 的设计思路，用 Rust 实现；通过插件与包的边界扩展 opi，使插件既
> 能单独调用，也能共同丰富 opi 生态。

所有 `opi-*` 技能都只能显式调用。Claude 侧使用
`disable-model-invocation: true`，Codex 侧使用
`policy.allow_implicit_invocation: false`。不确定入口时，显式调用
`opi-workflow`。

## 工作流地图

| 关注点 | 入口 | 产物或下一步 |
|---|---|---|
| 向内证据 | `opi-realign` | `docs/realign/` 下基于精确 pi 修订版的差异账本 |
| 向外证据 | `opi-research` | `docs/research/` 下基于一手资料的能力调研 |
| 高不确定性塑形 | 直接调用 Matt `wayfinder` | 决策地图；按需反复 research、realign、追问 |
| 有界设计质询 | 直接调用 Matt `grill-with-docs` | 明确的设计抉择与领域语言 |
| 设计已收敛 | 直接调用 Matt `to-spec` | 候选实现规格 |
| 准入与交付 | `opi-implement plan`，再调用 `opi-implement` | 已评审任务图与规范实现账本 |
| 静态保障 | `opi-audit` | 独立的 Standards/Spec 双轴发现 |
| 运行时保障 | `opi-eval` | 运行时保真度发现与 trace |
| 验证后修复 | `opi-remediate` | 保留来源的验证与可选修复 |
| 文档 | `opi-document` | 真实同步的中英文文档与快速、源派生检查 |
| 发布 | `opi-release <version>` | GitHub 产物与六个 crates.io crate |
| 测试链接优化 | `opi-slim-tests` | 已验证、未提交的集成测试二进制缩减 |

### 向内与向外必须分开

`opi-realign` 向内：固定 `earendil-works/pi` 的精确修订版，判断 opi 是否
仍然保留 pi 当前的语义与设计脉络，同时采用 Rust 原生的架构。它不负责提出
与 pi 无关的新功能。

`opi-research` 向外：研究 pi 没有或实现得不适合 opi 目标的能力，优先使用
一手资料，评估 Rust 可行性，并判断应放入现有插件/包、新插件，还是一个有
证据支持的最小核心 seam。它不写 spec，也不授权实现。

两类报告都只是塑形输入，不是需求本身。

### 需求塑形保持人为主导

把证据转化为功能点本来就不是固定流程。它包含澄清、实验、取舍、否决和
回到证据的循环。应按不确定性直接调用 Matt 技能：

- 大型、模糊、跨会话的设计空间使用 `wayfinder`；
- 边界较清楚、需要对抗性质询与领域模型维护时使用 `grill-with-docs`；
- 关键决策稳定后才使用 `to-spec`；
- 新暴露出的证据缺口重新进入 `research` 或 `opi-realign`。

`opi-workflow` 只负责路由，不新建账本，也不会把这个循环藏进自动转换。

### `opi-implement plan` 是对抗性准入关卡

plan 路径不替代产品设计。它只验证候选来源能否进入唯一的实现状态机：

1. 准入并固定规范来源；
2. 在不修改正式账本的前提下生成纵向切片任务图草案；
3. 分开质询设计就绪度与执行就绪度；
4. 确定性地返回一个结论：`READY`、`RESEARCH_REQUIRED`、
   `DESIGN_DECISION_REQUIRED` 或 `GRAPH_REVISION_REQUIRED`；
5. 只有结论为 `READY` 且用户确认任务图后，才修改
   `.opi-impl-state.json`。

缺少产品抉择时回到塑形；缺少证据时回到 `opi-research` 或
`opi-realign`。评审器不得暗中修改规范，也不得改写自己的草案来制造通过。

## Matt 与 Superpowers 的取舍

opi-* 内的推理与产物级子技能默认来自本地 Matt 技能包。Superpowers 只保留
不会与 opi 正式账本竞争的窄操作原语。

| 需求 | 选择 | 原因 |
|---|---|---|
| 向外证据 | Matt `research` | 一手资料优先，并产出仓库内调研文档 |
| 高不确定性塑形 | Matt `wayfinder` | 决策地图允许反复、回退与跨会话探索 |
| 有界对抗塑形 | Matt `grill-with-docs` | 将追问与领域语言维护结合 |
| Spec 合成 | Matt `to-spec` | 从已收敛上下文合成，而不是重新启动探索 |
| 实现切片 | Matt `tdd` | 先约定公共 seam，再做纵向 red/green 切片 |
| 困难诊断 | Matt `diagnosing-bugs` | 先建立可变红反馈环，再最小化与差分定位 |
| 审计视角 | Matt `code-review` | Standards 与 Spec 两个维度互不混淆 |
| 文档 | Matt `writing-for-agents` | 强调可缓存事实、指针和无需改动的结论 |
| 完成证明 | Superpowers `verification-before-completion` | 狭窄的“先证据、后声明”纪律 |
| 独立并行 | Superpowers `dispatching-parallel-agents` | 仅作为条件性并发原语 |

不组合进 `opi-implement` 的技能：

- Superpowers `brainstorming`、`writing-plans`、`executing-plans`、
  `subagent-driven-development` 会在正式账本旁形成第二套计划/执行流。
- Matt `to-tickets` 与 `implement` 含有可吸收的启发式规则，但不能替代
  `.opi-impl-state.json`；其中 tracer-bullet 拆分原则已进入 plan 准入。
- 这些技能仍可在实现 harness 之外直接用于塑形；不组合不代表它们普遍更差。

这套选择结合了 [AI Hero Skills 官方文档](https://www.aihero.dev/skills)、
本地固定版本的 [Matt 技能包](https://github.com/mattpocock/skills) 与
[Superpowers 技能包](https://github.com/obra/superpowers)。完整理由记录于
`docs/superpowers/specs/2026-08-09-opi-workflow-skill-system-optimization-design.md`。

## 保障契约

`opi-audit` 与 `opi-eval` 按 `_shared/references/finding-contract.md` 输出
统一发现。每条发现保留来源类型、路径、模型、独立性、轴、严重度、证据、
复现方式、置信度与“尚未验证”状态。

`opi-remediate` 可以直接消费任一来源，不再手工转录。它保留来源和严重度，
再用代码或运行时产物验证，并把修复验证与原始发现分开记录。如果发现实际
改变产品意图，remediate 必须停止并返回塑形。

条件允许时使用独立模型/评审器；无法完全独立时要如实标记。项目工作流不
固定某个 provider 或模型。

## 持久产物归属

| 产物 | 所有者 |
|---|---|
| `docs/realign/*.md` | `opi-realign`；非规范的向内证据 |
| `docs/research/*.md` | `opi-research`；非规范的向外证据 |
| `docs/opi-spec.md` 与登记过的补充 spec | 人主导塑形；规范来源 |
| `.opi-impl-state.json` | `opi-implement`；受 Git 跟踪的唯一实现账本 |
| `docs/snapshots/phase<N>/` | `opi-implement` 归档及 audit/remediation 证据 |
| `docs/eval/` | `opi-eval` 报告与历史 |
| `.opi-release-state.json` | `opi-release` 的可恢复公开/不可逆转换状态 |
| `_shared/references/finding-contract.md` | 跨技能发现格式 |
Git 安全规则只在始终加载的 `AGENTS.md` / `CLAUDE.md` 中定义，不在
`_shared` 下重复维护。

只有 `opi-implement` 可以写正式实现账本。research、realign、audit、eval、
remediation 计划、文档和发布都不得创建竞争性的任务账本。

## 技能索引

| 技能 | 契约 |
|---|---|
| `opi-workflow` | 薄路由器；无状态机，不实现功能 |
| `opi-realign` | 固定修订版的向内对齐；不提出向外功能 |
| `opi-research` | 一手资料优先的向外探索；不生成需求或实现 |
| `opi-implement` | 来源准入、对抗性任务图评审、TDD 交付、验证与账本检查点 |
| `opi-audit` | 独立审查固定提交区间的 Standards/Spec；不修复 |
| `opi-eval` | 显式、带凭据、隔离运行的运行时回归评估 |
| `opi-remediate` | 验证 audit/eval 的统一发现；执行仍需用户确认 |
| `opi-document` | 真实文档、中英文同步与无需编译的文档检查 |
| `opi-release` | 七阶段本地/公开/不可逆发布流程 |
| `opi-slim-tests` | 保留当前行为并删除重复/已取代的 Rust 测试二进制；不自动提交 |

调用后完整阅读对应 `SKILL.md`，只按其路由加载需要的 references。破坏性、
付费、使用凭据或发布类技能必须显式调用。
