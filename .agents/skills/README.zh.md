# Opi 技能用户手册

正式技能位于 `.agents/skills/`，`.claude/skills` 是别名。所有 `opi-*` 技能
均须显式调用；阅读或维护技能不等于执行。打开选定的 `SKILL.md`，只加载当前模式
需要的参考。手册只负责路由，不建立第二套契约。

## 选择入口

- 新功能或普通修复：确认目标和验收后遵循 [任务级上下文](../../docs/agents/context.md)。
  小而明确的任务直接做；跨会话或含糊的工作使用简报。不要求全局 spec、Phase 登记
  或 `opi-implement plan`。
- 需要证据：指定 pi 快照比较用 `opi-realign`；外部研究与当前 Opi 对照用 `opi-research`。
- 文档、评测、测试清理或发布：选择对应技能，不把这些步骤强加给每次迭代。
- 旧 Phase 任务或审计集合：先读 [旧流程边界](_shared/references/legacy-phase-workflows.md)，
  再选择 `opi-implement`、`opi-audit` 或 `opi-remediate`。
- 不确定：显式调用 `opi-workflow` 获取一个建议。

正常循环：用户目标 → 检查与澄清 → 小切片实施 → 验证 → 同步受影响的上下文。
证据和 tracker 状态不能代替产品决策批准。提交、发布、凭据和付费试次保留独立授权；
只在实质变化时重新确认，不反复确认常规步骤。

## 技能参考

| 技能 | 结果 / 归属产物 | 入口或停止边界 |
|---|---|---|
| `opi-workflow` | 一个下一步动作 | 只读，不调用其他技能 |
| `opi-realign` | `docs/realign/` 中的快照比较、架构建议和验收 | 使用实际提供的来源身份，不自动采用 |
| `opi-research` | `docs/research/` 中的一手研究和当前 Opi 对照 | 证据不是实施授权 |
| `opi-implement` | 旧 Phase 任务图、交付和台账 | 明确旧范围；登记输入及原有评审/提交关卡 |
| `opi-audit` | 旧 Phase 独立报告组与历史附录 | 选定的已提交 Phase 契约；不修复生产代码 |
| `opi-eval` | 外部评测证据、报告和已确认修复 | 真实范围/预算、维护、源码修复分别授权 |
| `opi-remediate` | 旧索引发现集的计划与批准修复 | 精确审计集合及批准计划摘要 |
| `opi-document` | 审计并同步文档 | 默认审计每份受维护的当前产品 README；targeted/version-bump 范围须明确 |
| `opi-release` | 版本、包、公开 Git 和 crates.io 发布 | 公开 Git 与最后时刻不可逆发布分别授权 |
| `opi-slim-tests` | 已验证的冗余测试二进制缩减 | 行为替代证明；保留为未提交变更 |

## 证据选择

`opi-realign target=<snapshot-path>` 接受没有 Git 的本地 pi 目录。所给内容决定目标，
其他文档的版本不能覆盖它。`opi-research <来源或问题>` 追踪一手来源和当前 Opi 对应能力，
借鉴、保持、观察和拒绝的依据见 [借鉴标准](opi-research/references/comparison-guide.md#transfer-standard)。
在线 latest 与本地快照身份分开记录。不用旧规划、台账或报告豁免真实差异，也不自动采用新版上游设计。

## 评测与验证

`opi-eval/SKILL.md` 定义 action/mode 输入和持续授权的默认值；
[操作契约](opi-eval/references/operator.md) 定义离线准备、执行、比较、外部证据存储和保留。
缺少基线不启动真实运行。已确认修复走 [源码修复分支](opi-eval/SKILL.md#user-confirmed-source-repairs)，
不隐式调用 Phase 计划。证据本身也不使旧任务自动完成。

[检查选择](_shared/references/change-scope-and-check-selection.md) 与 [CI/平台策略](../../docs/ci.zh.md)
决定适用检查；复用仍有效的证明，不因技能名称不同重跑。开发用 `opi-eval` 不自动成为产品
CI/发布准入。构建或维护缓存前读 [Cargo 缓存生命周期](_shared/references/cargo-cache-lifecycle.md)。
release 与 document 复用有界版本文档流程，不隐式调用另一个用户专用技能。

## 旧 Phase 保障

只有明确处理旧工作时才加载本节。[边界](_shared/references/legacy-phase-workflows.md)
优先于各自程序，负责来源适用性及缺失输入处理。

- 只有 `opi-implement` 写 `.opi-impl-state.json`；
  [任务完成证明](opi-implement/references/task-completion.md) 与
  [台账 schema](opi-implement/references/ledger-schema.md) 定义检查点。
- Audit 使用 [审计集合契约](_shared/references/audit-set-contract.md)，维护独立安装、摘要绑定
  和安装后的有限历史参考。旧结论不能替代独立证明。
- Remediation 使用 [处置契约](_shared/references/remediation-disposition-contract.md)，定义发现并集、
  计划批准、范围和关闭。原有双模式及精确摘要关卡不变，也不写台账。
- 旧报告、冻结快照和来源哈希仍是证据；新简报既不复制旧进度，也不暗示旧 Phase 已通过。

## Matt 适配与维护

按请求使用 Matt 流程时，先读 [tracker 配置](../../docs/agents/issue-tracker.md) 和
[领域配置](../../docs/agents/domain.md)，将根词表及 `.scratch/` 默认位置适配到项目的 `docs/`。
`ask-matt` 负责路由，`wayfinder` 解决大型决策空间，都不是每个功能的必经步骤。
宣称兼容前核对已安装技能；缺少可选技能不妨碍当前对话中的澄清。

技能专属辅助脚本/测试留在所属 `scripts/`，共享保障辅助放在 `_shared/scripts/`，
Companion 运行代码归 crate，仓库级检查、缓存和发布辅助留在根 `scripts/`。
每条规则只保留一个维护位置，以条件指针代替复制。保留用户所选模型；通过代表性任务结果
评估指令变更，而不只比较文字长度。[文档导航](../../docs/README.zh.md) 负责上下文路由。
