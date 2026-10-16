# Phase 20 来源注册与上位路线独立复审

结论：**REGISTRATION-REVIEW-PASS**。没有存留的文档注册阻塞。此结论只覆盖本次来源注册与上位路线修订；不是 `opi-implement` P.0 准入、P.2 任务图复审、P.4 整图确认、依赖批准或产品完成判定。

本报告是独立 fresh-context、same-family 复审，不声称 cross-model 独立性。它是中文单独审计记录，没有既有英文 counterpart；受审的上位规范和 Phase 正文均保持双语。本报告及配套 JSON 只记录审计输入与限制，不进入 `spec_files`，不充当实施台账。

## 任务合同与检查边界

用户明确要求把 `spec-draft.md` 注册为 Phase 交付来源，并把 `docs/opi-spec.md` 更新到选定路线。作者任务的 DoD 显式拥有 `docs/opi-spec.md`、`docs/opi-spec.zh.md`、`.agents/skills/opi-implement/SKILL.md`、双语 Phase 正文、当前三份附件／矩阵和 README 的来源身份与边界说明；仅限文档／技能维护，禁止 runtime、Cargo、产品测试、CI、历史证据、ledger、stage、commit 或网络操作。该显式拥有的文档／对齐合同符合 `.agents/skills/opi-implement/SKILL.md:282` 的上位规范编辑例外；它没有启动产品任务或制造已确认任务图。

复审者只拥有本报告与 `phase20-registration-review.json`。未编辑作者文件、台账或历史制品，未递归调用 workflow／reviewer，未发请求、建立 socket、下载或重跑原网络实验。

- 固定输入 → verify: HEAD、18 个 CRLF→LF SHA-256、原始 ledger SHA 和起止 Git inventory。
- 检查来源注册 → verify: 单一注册表的 16 个未来 live 文件、精确章节采纳、非递归证据边界和准入分离。
- 检查上位路线 → verify: 双语含义、61 个条款／战略 ID 的顺序、现存 MUST 门禁、独立 Eval 及全项目使命。
- 检查 Phase 合同 → verify: 条款映射、归属、分解、成功／退出、风险、回滚、平台、AIR/V 处置与无 PASS 晋升。
- 完成记录 → verify: `python scripts/opi-doc-check.py` 与 `git diff --check` 均 exit 0，受审输入 hash 未漂移。

## 受审输入与结果

基线 HEAD 为 `5e5282e020f4d746576a593627ff7cac93417b52`。完整复核了相对该 HEAD 的 10 个变更文件 diff，并沿注册／采纳表检查相关来源章节。18 个冻结文件的实际 LF hash 全部匹配；精确映射和复审限制见 `phase20-registration-review.json`。

| 检查项 | 结果与依据 |
|---|---|
| 上位权威与项目使命 | `docs/opi-spec.md:34` 的 AUTH-001–005、`:60` 的使命和 `:120` 的 PRIN 门禁保留；`:94` 的包级目标例外及 `:140` 的 SDK 方向不把全项目改成 Pi 移植。AUTH、GOAL、PRIN、PLACE、CTRL、GATE、PHASE 规范条款未改变。 |
| 上位条款强度 | 双语各 61 个条款／战略 ID 与 HEAD 同序。仅 CAP-001、INV-001、INV-002 表行变化；INV-001 加入 operation／选定 auth，INV-002 增加公开 caller／adapter、失败／取消／资源与非 substrate 证明义务（`docs/opi-spec.md:405`）。其余规范行原文保留。 |
| SDK 优先级与 Eval | `docs/opi-spec.md:657` 将 opi-ai 基础置于其下游 Agent 扩展之前；`:667` 与 `:686` 继续允许独立 Eval 依靠经证明的最小 evidence seam 推进，未把完整 SDK parity 设成其无条件前置条件。中文对应含义一致。 |
| 注册集合 | `.agents/skills/opi-implement/SKILL.md:41` 明确 Phase 20／STRAT-001；`:49`–`:67` 恰有 14 份 supplemental，加两份父规范共 16 份未来 live 来源。18 份复核输入还包含注册表和 README；不是 18 份 live 来源。没有 ledger 安装或 hash 刷新。 |
| 精确采纳与证据隔离 | `spec-draft.md:55`–`:67` 及中文同位表限定 readiness 机制段、TP-02/03/13/14、选定 RP SDK 部分、SDK 单一归一化规则及附录 A–D；明确排除 RP-03 旧跨 route thinking 转 text、旧宿主范围／猜测预算／未决措辞。整文件 hash 不采纳历史段落；其他链接、tracker、审计、raw probe／archive 不递归获得权威。 |
| Phase 合同可注册性 | `spec-draft.md:19`–`:32` 包含条款映射、优先理由、placement、来源、三平台／MSRV边界、成功／退出、停止条件、回滚与准入边界；`:34`–`:53` 的 DEL-01–14 是待提取来源分解，要求真实公开场景及 substrate 的后续 owner 闭包，不伪装 ledger task ID。中文对应含义一致。 |
| AIR/V 身份与处置 | 两份正文各 67 个 AIR 表行与 HEAD 逐行相同：57 active SDK、9 host-only、AIR-060 retired。矩阵 59 个 V 表行与 HEAD 逐行相同：54 active SDK、4 host-impact、V-52 retired；没有编号复用、行删除或 PASS 晋升。处置边界见 `spec-draft.md:79` 与 `acceptance-matrix.md:7`、`:15`。 |
| 设计 READY 与产品 OPEN | `spec-draft.md:15`、`pi-1.3.0-design-readiness.md:13` 和 `acceptance-matrix.md:3` 保留七组产品／平台覆盖 OPEN；旧机制结果、日期和通过计数仍限于原输入。注册不证明 public consumer、package、native 或完整验收。 |
| 非任务文件保全 | Git inventory 只有原来的 10 个作者文档／技能变更及本复审新文件；产品源码、Cargo、CI、测试、旧报告／原始 probes、`docs/snapshots/` 和 `.opi-impl-state.json` 没有相对 HEAD 变更。ledger raw SHA 仍为 `dd914532dfaaefb4ec340801fc88e1aba784afb71ffbf1e9925dd8c7cdc7e96f`，current_phase=19、tasks=[]，Phase 16–19 历史保留。 |

## 检验后不成立的候选发现

| 候选发现 | 驳回依据 |
|---|---|
| research 路径中的文件不能注册 | AUTH-003 允许下位 Phase 交付来源；`docs/opi-spec.md:46` 与注册表明确限定它的权威。目录不是自动准入来源，也不是禁止受审来源注册的规则。 |
| 注册或设计 READY 已跳过 P.0／P.2／P.4 | 注册表 `:46`、正文 `:32` 和 initializer 的 P.0–P.4 均分离这些动作。当前没有 draft graph，不能要求本次出具 P.2 READY，更不能安装 ledger。 |
| CAP-001 删除 “bounded failure” 等于移除门禁 | 旧表项不是独立总 deadline／大小 cap。`docs/opi-spec.md:66`、`:158`、`:425` 的有界并发／队列、provider retry 与外部 deadline 仍在；新增 INV-002 公开验证失败／取消／资源所有权。`:165` 进一步区分调度与接受集，未豁免任何现存 MUST。 |
| 14 个 supplemental 均全篇采纳，旧宿主政策成为 SDK 门槛 | 注册表 `:69` 与双语采纳表限定历史三文件的具名算法部分，正文 scope、host-only 行和矩阵明确排除新宿主政策；当前正文／RDF 优先，当前来源冲突须返回复审。 |
| 单语技术附件造成双语合同缺失 | 各附件声明无既有英文 counterpart，双语正文同步同一合同和采纳表；本次两份父规范及两份正文变化逐项等价。没有删除或遗留失配的已有 counterpart。 |
| 父规范更新应顺带刷新旧 ledger hash | AUTH-003／PHASE-001 与技能 drift 规则要求后续 source admission、图复审和人工整图 gate；本次保留旧 hash 可见漂移，恰好避免未确认图被追认。 |

## 验证与尚未证明事项

独立执行 `python scripts/opi-doc-check.py`：`opi documentation contracts: PASS`，exit 0；`git diff --check`：无输出，exit 0。报告落盘后再次执行相同检查并复核冻结输入。中途一个临时只读 inventory Python 片段因缩进错误 exit 1，修正后完整运行成功；它不是产品或文档合同失败。

测试影响：产品 **none**，既有产品测试 **retain**；本次新增两个审计文件，不新增或改写测试、机制探针、原始结果。`committed=none-for-this-task`、`staged=none`；作者的 10 个原有 unstaged 文件为 carried-in，复审者未触及；untracked 为本报告与 JSON。

剩余事项属于后续正式 plan／实施：P.0 来源准入、详细任务／验收／最小变更 trace、P.2 对抗图复审、P.4 整图确认，以及真实产品 consumer、依赖／package、MSRV／三平台与 Phase-exit 证据。此注册复审没有验证或宣称这些事项通过。
