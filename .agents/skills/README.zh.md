# Opi 技能用户手册

Opi 有十个项目技能。受 Git 跟踪的真实来源位于 `.agents/skills/`；
`.claude/skills` 是指向同一目录的兼容符号链接。只编辑
`.agents/skills/` 下的正式文件。所有技能都必须由用户显式调用，例如 Codex
中的 `$opi-workflow` 或 Claude Code 中的 `/opi-workflow`。

不确定入口时使用 `opi-workflow`。它只推荐一个下一条命令并停止，不会创建
另一套工作流账本。

## 30 秒选择入口

1. 你在决定“应该构建什么”吗？
   - 对照固定的 pi 修订版检查 opi，并纳入其目标设计视野：使用 `opi-realign`。
   - 调研外部能力或生态方案：使用 `opi-research`。
   - 已有证据但产品决策未收敛：使用路由器推荐的人主导塑形命令，不要开始实现。
2. 是否已有评审并登记过的 Phase 交付来源？
   - 没有：返回证据或塑形。
   - 有：先运行 `opi-implement plan`；通过准入后才能运行 `opi-implement`。
3. 你在检查或修正已交付行为吗？
   - 静态需求符合性：`opi-audit`。
   - 带凭据的真实 provider 行为：`opi-eval`。
   - 验证并按需修复统一发现：`opi-remediate`。
   - 文档、发布或测试链接清理：使用对应的具名技能。

## 生命周期与返回循环

实线表示工作流推进；虚线表示必须经过人工决策或将决策落入规范来源。
`READY` 既不代表已经实现，也不代表授权提交。

```mermaid
flowchart TD
    RI[opi-realign<br/>向内证据]
    RO[opi-research<br/>向外证据]
    SH[人主导塑形<br/>评审并落地决策]
    SRC[已登记的 Phase<br/>交付来源]
    PLAN[opi-implement plan<br/>准入与任务图评审]
    EXEC[opi-implement<br/>交付与 Phase exit]
    ASSURE[opi-audit / opi-eval<br/>独立保障]
    FIX[opi-remediate<br/>验证并按需修复]
    DOC[opi-document]
    REL[opi-release]
    SLIM[opi-slim-tests<br/>独立测试链接优化]

    RI --> SH
    RO --> SH
    SH -. 人工批准并登记 .-> SRC
    SRC --> PLAN
    PLAN -->|READY + 任务图关卡| EXEC
    PLAN -. RESEARCH_REQUIRED .-> RI
    PLAN -. DESIGN_DECISION_REQUIRED .-> SH
    PLAN -->|GRAPH_REVISION_REQUIRED| PLAN
    EXEC --> ASSURE
    ASSURE -->|已确认发现| FIX
    FIX --> ASSURE
    ASSURE -->|需求已满足| DOC
    DOC -. 公开与不可逆关卡 .-> REL
    EXEC -. 当前测试图 .-> SLIM
```

## 边界规则

- 证据不是需求。`opi-realign` 与 `opi-research` 不能授权产品实现。
- 塑形由人主导。Tracker 地图与候选 spec 在经人工评审并落入
  `docs/opi-spec.md` 或已登记 Phase 交付来源前不具规范性。
- `opi-implement plan` 只检查就绪度，不修补缺失的产品含义。任务图确认与
  Git commit 授权是两个不同关卡。
- `opi-audit` 与 `opi-eval` 只诊断，不编辑生产代码。每次 audit 只使用最新已提交
  的登记来源，绝不读取先前 audit 或 remediation 结论。
- `opi-remediate mode=plan` 验证当前活动 indexed audit set 的每个成员，并为
  findings 严格并集制定计划；`mode=apply` 必须显式批准固定计划摘要与精确活动
  index 摘要。apply 只允许契约限定的顺手修复，两种模式都不写入正式
  `.opi-impl-state.json`。
- 默认情况下，`opi-document` 会依据实现证据独立审计每份受维护的当前产品 README；
  targeted 与 version-bump scope 必须显式指定。它可以修正文档，但不授权发布。
- `opi-release` 是唯一公开发布流程；crates.io 发布还有独立的最后时刻
  不可逆关卡。

## 技能参考

副作用标记：`RO` 只读，`W` 写仓库文件，`C` 经显式关卡后可能创建本地
commit，`$` 可能使用凭据或付费 provider，`P` 改变公开状态，`I` 包含不可逆步骤。

| 技能 | 角色与必要输入 | 归属产物 | 停止点、关卡与通常下一步 | 副作用 |
|---|---|---|---|---|
| `opi-workflow` | 为不确定请求选择入口 | 一条推荐调用 | 在每个显式技能边界停止 | RO |
| `opi-realign` | 对照固定 pi 修订版的当前状态与目标设计视野检查 opi | `docs/realign/*.md` | 只是证据；经登记后进入塑形或 `opi-implement plan` | W |
| `opi-research` | 基于一手资料调研向外能力 | `docs/research/*.md` | 只是证据；下一步为塑形 | W |
| `opi-implement` | 准入 Phase 来源、执行任务图并归档 Phase 证据 | `.opi-impl-state.json`、Phase 快照、任务变更 | 任务图、任务 commit、账本 commit 和失败关卡相互独立 | W、C |
| `opi-audit` | 以显式 reviewer/model 身份，按最新已提交的登记来源独立核实一个 Phase | `docs/snapshots/phase<N>/assurance/` 活动索引集合中一个带后缀的四文件报告组 | 不修复；各报告独立入集，当前发现进入 `opi-remediate` | W |
| `opi-eval` | 运行显式、隔离的真实 provider 保真度用例 | `docs/eval/` 报告与历史 | 可能需要凭据；变更工具还需额外确认 | W、$ |
| `opi-remediate` | `mode=plan` 验证活动 indexed set 并为 findings 严格并集生成 closure batch；`mode=apply` 执行精确批准的 plan 与 index 摘要 | 集合级固定计划、disposition、结果和用户批准的修复 | `READY-FOR-APPLY` 与两个精确摘要绑定共同控制执行；意图变化返回塑形 | W |
| `opi-document` | 默认依据实现证据审计每份受维护的当前产品 README；按显式 targeted/version-bump scope 同步文档 | 文档与 doc-check 变更 | 不发布 | W |
| `opi-release` | 为六个 crate 与 GitHub 产物执行七阶段发布 | Git tag/release 与 crates.io 版本 | 公开 Git 关卡之后仍有独立 crates 不可逆关卡 | W、C、P、I |
| `opi-slim-tests` | 在不丢失行为的前提下删除重复或已取代测试二进制 | 已验证、未提交的测试图缩减 | 从不自动提交 | W |

## 保障模型

`opi-audit` 与 `opi-eval` 使用 `_shared/references/finding-contract.md` 的统一
发现词汇，但只有当前活动 indexed set 能进入 `opi-remediate`。每个 Phase 只有一个
活动 assurance set，其中包含独立入集的 reviewer/model 报告组；各报告地位相等，
不存在 canonical report，每个成员携带自己的已提交 `audit_head`，单成员集合即合法。
聚合 verdict 采用 fail-dominant 规则，remediation 消费全部 indexed finding 的严格
并集，并把批准绑定到精确 `audit.index.json` 摘要。

同一 reviewer/model 重跑会替换自己的条目；被替代的 run 移入
`assurance/history/<audit-run-id>/`，永不成为语义输入。一个行为证明可以关闭多个
source key，但每个来源仍有独立 disposition；结果只允许带独立 red/green 证明、阻塞
验证且受限的顺手修复。只有修复与活动集合已提交且 assurance 目录干净后，才能开始
新的独立 audit 或重跑。通用 provider-fidelity canary 只是运行时信号；只有登记过的
runtime-fidelity 用例才能关闭产品条件。

条件允许时使用独立模型或评审器，并如实披露独立性降级。reviewer/model ID 只披露
实际 runtime 身份，绝不选择 provider model。项目契约不固定某个 provider 或模型。

## 持久产物归属

| 产物 | 所有者与生命周期 |
|---|---|
| `docs/realign/*.md` | `opi-realign`；非规范向内证据 |
| `docs/research/*.md` | `opi-research`；非规范向外证据 |
| Tracker 地图、票据和候选 spec | 人主导塑形；落入并登记前不具规范性 |
| `docs/opi-spec.md` 与已登记 Phase 交付来源 | 人主导塑形；规范来源 |
| `.opi-impl-state.json` | `opi-implement`；唯一正式、受跟踪实现账本 |
| `.opi-impl-state.draft.json` | `opi-implement plan`；仅在评审/恢复需要时保留的忽略草稿 |
| `docs/snapshots/phase<N>/` | 冻结的实现账本快照与非保证类 Phase 证据 |
| `docs/snapshots/phase<N>/assurance/` | 一个含独立入集报告组的活动 indexed audit set 与一组 remediation；被替代 run 位于 `history/`，不参与语义输入 |
| `docs/eval/` | `opi-eval` 报告与历史 |
| `.opi-release-state.json` | `opi-release`；仅在未完成发布期间保留的忽略恢复状态 |
| `_shared/references/finding-contract.md` | 共享发现格式 |
| `_shared/references/audit-set-contract.md` | indexed 报告路径、身份、聚合 verdict、锁/恢复、独立入集、轮换与历史规则 |
| `_shared/references/remediation-disposition-contract.md` | 严格并集来源绑定、index 摘要批准、决策、顺手修复与关闭格式 |

只有 `opi-implement` 能写正式实现账本。不得创建第二套任务账本，也不得在
`docs/opi-spec.md` 中记录实现进度。

## 维护者组合说明

项目本地技能拥有 Opi 产物和生命周期边界。调用策略允许时，Matt 技能可以
提供证据、领域建模、设计质询、测试 seam 设计、TDD 或评审视角；Superpowers
可以提供“完成前先验证”等窄操作原语。两者都不得替换
`.opi-impl-state.json`、暗中调用仅用户可调用的塑形命令，或在
`opi-implement` 内建立第二套计划/执行状态机。

选定技能后，完整阅读其 `SKILL.md`，并只加载它路由到的 references。破坏性、
付费、使用凭据、产生 commit 或发布的动作始终需要显式用户关卡。
