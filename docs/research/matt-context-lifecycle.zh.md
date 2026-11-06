# Matt 技能的轻量上下文机制：Opi 迁移研究

> 研究证据，不是规范。基于本机已安装技能的只读调查，未验证 slash-command 运行兼容性。
> 下文 Opi 对照指迁移前状态（`dcbd92f38e119b93f17a8b31a584afc18cb89bc2`）；
> 当前文档布局与工作约定见[文档导航](../README.zh.md)和[任务级上下文](../agents/context.md)。
> 此报告保留调查时的建议与限制，不作为第二份当前工作流指导。

## 结论

**可以借鉴 Matt 的机制，把长期工程约束、领域术语、重要决策和本轮工作上下文分开；不应把旧总规格机械拆成四份新的永久规格。** 防止过期的核心不是文件名，而是：事实留在可查询的一手来源、每个意思只有一个权威位置、按条件加载、决策即时记录、短期工作有明确结束边界。

**`/wayfinder` 不是每个功能的必经流程；`/setup-matt-pocock-skills` 也不会自动生成一套完整知识库。** 原版技能对根目录 `GLOSSARY.md`、`.scratch/` 等有明确假设，因此“所有上下文都放在 `docs/`”需要项目级适配，不能只移动文件后宣称兼容。

本研究按 `/ask-matt` 的规则，先读实际技能再提出路由建议；没有执行访谈、setup、tracker 发布或实现流程。`ask-matt` 是路由器，不是写文件命令，也不自动启动其他用户调用型技能。〔`ask-matt/SKILL.md` → `# Ask Matt`；`writing-for-agents/SKILL-MECHANICS.md` → `## Router skills`〕

### 证据范围

- Matt 一手来源根目录：`C:/Users/Luiz/.cc-switch/skills/`。下文技能路径均相对此目录，引用给出实际标题；无 Markdown 标题的文件明确注明。
- 完整阅读 18 份已安装技能/配套文件，包括题目指定的全部 SKILL，以及格式、tracker、阶段边界、调用机制和审查发现规则。
- Opi 对照仅阅读 `AGENTS.md`、`docs/CONTEXT.md`、`.agents/skills/README.md`；没有读取总规格、运行时代码或检查器。因此，“旧规格已过期”是用户提出的问题，不是本研究独立核实的审计结论；下面的迁移布局是建议，不是已批准的新契约。

## 1. 哪些技能实际创建什么

| 技能 | 已安装文本的行为 | 不应误解为 |
|---|---|---|
| `ask-matt` | 解释路径、推荐下一技能。 | 自动执行整条工作流或生成文档。 |
| `setup-matt-pocock-skills` | 探索、逐项确认、展示草稿后，更新 steering 文件的 `Agent skills` 块；写 `docs/agents/issue-tracker.md`、`domain.md`，安装了 triage 时还写 `triage-labels.md`。GitHub/GitLab 分支还可能创建标签、配置 remote。 | 自动写出术语定义、ADR、工程规范或迭代规格。 |
| `grill-with-docs` | 正文仅调用 `grilling` 与 `domain-modeling`；文件产出来自后者。 | 每次访谈必有 ADR，或访谈结束自动产出 spec。 |
| `domain-modeling` | 首个术语被解决时才创建 glossary；术语达成共识即更新。首个符合条件的决策出现时才创建 ADR 目录/文件。 | 预先搭满空目录、把所有讨论都保存为长期规范。 |
| `wayfinder` | 在配置的 tracker 中创建 map 与决策票；本地 tracker 才表现为 Markdown 文件。研究票还有独立研究分支等操作。 | 默认创建 `docs/` 规格，或默认执行产品实现。 |
| `to-spec` | 综合已有对话与代码理解；确认测试 seam 后将 spec 发布到配置的 tracker。 | 自动访谈补齐未知，或默认更新项目级总规格。 |
| `to-tickets` | 用户确认切片及依赖后，发布一票一文件/issue；本地分支明确写到 `.scratch/<feature-slug>/issues/`。 | 单个永远增长的任务总文档。 |
| `retro` | 读会话一手材料，按严重程度提出环境改进候选。 | 自动改代码、全局规则或仓库文件的授权。 |
| `writing-for-agents` | 写作参考，指导指针、分层、裁剪。 | 自带生成器、同步器或过期检测器。 |

依据：

- `setup-matt-pocock-skills/SKILL.md` → `### 1. Explore`、`### 2. Present findings and ask`、`### 3. Confirm and edit`、`### 4. Write`。
- `grill-with-docs/SKILL.md` → 无 Markdown 标题，frontmatter 后全文为一条调用指令；`domain-modeling/SKILL.md` → `## File structure`、`### Update GLOSSARY.md inline`、`### Offer ADRs sparingly`。
- `wayfinder/SKILL.md` → `## The Map`、`### Chart the map`；`setup-matt-pocock-skills/issue-tracker-local.md` → `## Wayfinding operations`。
- `to-spec/SKILL.md` → `## Process`；`to-tickets/SKILL.md` → `### 4. Quiz the user`、`### 5. Publish the tickets to the configured tracker`。
- `retro/SKILL.md` → `## Steps`；`writing-for-agents/SKILL.md` → `## Information hierarchy`、`## Pruning`。

## 2. 实际减少陈旧上下文的机制

**以下是来源事实，不是自动化保障。**

1. **不复制容易查询的事实。** manifests、配置、目录和 `--help` 也是权威来源；文档复述它们是缓存，只有查询昂贵时才值得。文档应保留代码看不出的约定、理由和陷阱。每个意思只保留一个权威位置，并主动清理过期、无关和不改变行为的句子。〔`writing-for-agents/SKILL.md` → `## Pruning`〕
2. **加载有触发条件，而非让 agent 每次读全库。** 指针要说明材料是什么、哪类决策应读取它；通用信息留在主层，分支专用材料再展开。减少文件篇幅与注意力负担，而不是单纯追求更少 token。〔`writing-for-agents/SKILL.md` → `## Context pointers`、`## The two loads`、`## Information hierarchy`〕
3. **术语和实现分离，边讨论边校正。** glossary 只放项目特有术语，定义一两句，列 `_Avoid_`；不放实现细节、规格或草稿。遇到术语矛盾立即指出；用户描述的行为应对照代码，发现冲突交由人明确。〔`domain-modeling/GLOSSARY-FORMAT.md` → `## Rules`；`domain-modeling/SKILL.md` → `### Challenge against the glossary`、`### Cross-reference with code`、`### Update GLOSSARY.md inline`〕
4. **ADR 稀疏而非全量记事。** 只在“难逆转、无背景会令人意外、确有取舍”三项同时成立时提出。正文可只有一段；重访时可加 `deprecated` / `superseded by ADR-NNNN`，状态是可选格式，不是自动生命周期。消费者遇到冲突应显式提示。〔`domain-modeling/ADR-FORMAT.md` → `## Template`、`## Optional sections`、`## When to offer an ADR`；`setup-matt-pocock-skills/domain.md` → `## Flag ADR conflicts`〕
5. **短期计划降低对易变实现的绑定。** spec 和票通常不写具体路径/代码片段，避免快速过期；例外是原型中比文字更准确表达决策的状态机、schema 等精简片段。票有可验证验收和阻塞关系，每票适合一个新上下文窗口。〔`to-spec/SKILL.md` → `## Implementation Decisions`；`to-tickets/SKILL.md` → `### 3. Draft vertical slices`、`### 5. Publish the tickets to the configured tracker`〕
6. **不要把摘要误当成完整历史。** grilling→spec→tickets 尽量保留同一上下文；实现票可分别新开上下文。靠近上下文容量边界时，优先在阶段边界处理；压缩和交接会损失一手推理。retro 应在清空前运行，或重新读取会话日志。〔`ask-matt/SKILL.md` → `### Context hygiene`；`ask-matt/PHASE-BOUNDARIES.md` → `## The tree`、`## Primary and secondary sources`〕
7. **复盘把机械错误变成检查。** 路径规则、禁用 API 等机械错误优先进入已有 lint/hook/CI；真正依赖判断的规范才交给审查者。不是每次失败就往 AGENTS 增加一条永久指令。〔`retro/SKILL.md` → `## Steps`、`### Implementation vs Review`、`### Files`〕

**限制：** 本次读取的来源没有提供通用“过期时间”、自动语义同步或 ADR 冲突检测程序。其效果依赖维护行为、清楚的权威边界和项目自己的确定性检查；只换目录不能消除腐化。

## 3. 为什么 wayfinder 不适合每个功能

`ask-matt` 明确把它限定为“一次会话容纳不了、路径仍迷雾重重的大工作”，并明确说不要用于边界清楚的功能。普通可在一次会话中澄清的想法走 `grill-with-docs`；是否需要 spec/tickets，则取决于是否跨会话实现。〔`ask-matt/SKILL.md` → `## On-ramps`、`## The main flow: idea → ship`〕

`wayfinder` 的对象是**决策图，不是交付任务图**：

- map 是索引；详细答案只在各决策票中，避免双份状态。
- frontier 只包含未关闭、无阻塞、未认领的子票；每会话通常只解决一票，研究票例外。
- 不可精确表达的问题留在 `Not yet specified`；已明确超出范围的事项不应再自动进入 frontier。
- 解决一票后更新失效票、毕业新的问题；地图清晰便交接，不默认继续造产品。
- `Notes` 可显式覆盖“只计划”默认，但这是例外，不能把默认描述成执行流程。

〔`wayfinder/SKILL.md` → `## Plan, don't do`、`## The Map`、`## Fog of war`、`## Out of scope`、`## Invocation`、`### Work through the map`〕

**对本任务的建议：** 先用有明确边界的设计讨论决定权威拆分与迁移范围，不因为“正在改文档体系”就建立 wayfinder 地图。只有拆分后仍存在互相阻塞、跨会话才能解决的真实治理问题，才考虑它。此建议不授权启动任何技能。

## 4. `docs/` 自定义布局：可行，但有精确的集成风险

| 风险点 | 来源中的明确假设 | 对 Opi 的影响 / 建议 |
|---|---|---|
| Glossary 发现与创建 | `domain-modeling`、格式说明和 setup consumer 模板都查根目录 `GLOSSARY.md` / `GLOSSARY-MAP.md`；不存在时会在根目录惰性创建。 | 仅把词表放进 `docs/GLOSSARY.md` 不够，可能出现“未发现旧词表→创建第二份根词表”。项目调用约定必须明确覆盖读取和写入路径，验证不能意外创建根词表。 |
| 多上下文布局 | map 在根目录，示例词表及 ADR 在 `src/<context>/`。setup 的探索信号主要是 pnpm/npm/packages 布局。 | Rust workspace 不等于多个领域上下文；不能只因为多 crate 就复制一套多词表结构。默认一个领域词表最小。 |
| ADR | `docs/adr/`，顺序编号 `0001-slug.md`；多上下文另有局部 ADR。 | 用户要 `docs/` 与默认兼容，优先直接使用该路径，无需再改名。 |
| 本地迭代/票 | tracker 模板用 `.scratch/<feature>/spec.md`、`issues/`；`to-tickets` 正文本身也重复写死 `.scratch/`。wayfinder 本地 map 同样在 `.scratch/`。 | 只改 `docs/agents/issue-tracker.md` 为 `docs/iterations/`，仍与 `to-tickets` 的直接指令冲突。需明确的项目适配和验证，不能声称存在统一路径参数。 |
| setup 修改入口 | 只要 `CLAUDE.md` 存在便优先编辑它；探索 legacy `CONTEXT.md` 是根路径。 | 与 Opi“只编辑 `AGENTS.md`，`CLAUDE.md` 为别名”直接冲突；也不能假定它会发现 `docs/CONTEXT.md`。不应原样执行 setup。 |
| setup 副作用 | 可能 `git init`、添加 remote、创建远程标签；GitHub tracker 的 publish 就是创建远程 issue。 | 不是单纯本地文档初始化。配置草稿、远程发布和 Git 权限必须分别受项目安全边界约束。 |
| 工程规范位置 | retro 提到 `CODING_STANDARDS.md`；但实际 `code-review` 会搜索仓库中所有规范文件，并非只允许根目录标准文件。 | `docs/engineering.md` 可以成为审查来源；提供明确指针即可，不必为“兼容 Matt”再复制一份根文件。 |
| 路由和执行权限 | router 只能提示用户调用型技能；wayfinder charting 又明确带研究子代理、研究分支及 push。 | 本次只读、禁止子代理的授权不能容纳直接执行这些流程；研究技能文本不等于许可副作用。 |

依据（对应上表）：

- `domain-modeling/SKILL.md` → `## File structure`；`domain-modeling/GLOSSARY-FORMAT.md` → `## Single vs multi-context repos`；`setup-matt-pocock-skills/domain.md` → `## Before exploring, read these`、`## File structure`。
- `setup-matt-pocock-skills/SKILL.md` → `### 1. Explore`、`### 2. Present findings and ask`、`### 4. Write`。
- `domain-modeling/ADR-FORMAT.md` → `# ADR Format`、`## Numbering`。
- `setup-matt-pocock-skills/issue-tracker-local.md` → `## Conventions`、`## Wayfinding operations`；`to-tickets/SKILL.md` → `### 5. Publish the tickets to the configured tracker`。
- `setup-matt-pocock-skills/issue-tracker-github.md` → `## Conventions`、`## When a skill says "publish to the issue tracker"`。
- `retro/SKILL.md` → `### Files`；`code-review/SKILL.md` → `### 2. Identify the spec source`、`### 3. Identify the standards sources`。
- `writing-for-agents/SKILL-MECHANICS.md` → `## Router skills`；`wayfinder/SKILL.md` → `### Chart the map`。
- Opi `AGENTS.md` → `## Authority and context`、`## Git and release safety`。

**定制能力的准确边界：** setup 是 prompt-driven，不是确定性脚本；会展示草稿并允许用户修改，完成后也允许直接编辑 `docs/agents/*.md`，因此项目级规则有定制入口。但读取的技能没有声明通用 `docs_root` 配置，也没有证明所有直接路径都会服从这些规则。应区分“可按项目覆盖”与“原版零改动透明支持”。〔`setup-matt-pocock-skills/SKILL.md` → `# Setup Matt Pocock's Skills`、`### 3. Confirm and edit`、`### 5. Done`〕

## 5. 建议的最小迁移：改变权威分工，保留安全边界

以下是研究建议，**不是 Matt 默认目录，也不是本次已执行的变更**。

```text
AGENTS.md                          # 短路由 + 必要授权/安全约束
docs/engineering.md                # 长期工程原则、边界；不抄代码/manifest
docs/GLOSSARY.md                   # 从现有 docs/CONTEXT.md 提炼/迁移的单一词表
docs/adr/NNNN-<decision>.md         # 少量高价值决策及理由
docs/iterations/<slug>/context.md   # 本轮目标、范围/非目标、验收、关键决策与证据指针
```

`docs/agents/domain.md` / `issue-tracker.md` 是**接入 Matt 消费者时才需要的可选适配文件**，不应为了整齐预先创建整套空配置。也不建议同时维护 `context.md` 和内容重复的 `spec.md`；若接入本地 tracker，选一个为该迭代的唯一规格。

### 迁移顺序与可观察完成条件

1. **先确定新权威表，而非先改名。** 长期约束归 engineering；术语归 glossary；已接受的重要取舍归 ADR；本轮授权范围与验收归迭代上下文；当前行为仍以源码、schema、测试、help 等为准；历史仍是历史。明确迭代草稿何时成为人审后的交付依据。
   **verify：** 每类决策只有一个当前权威来源，读者能分辨“事实、已接受约束、候选、历史”。
2. **复用现有内容，逐项分流。** `docs/CONTEXT.md` 已采用术语 + `_Avoid_` 的词表形式，不能当作杂乱上下文整份丢弃。术语是否仍相关，需要后续对照当前产品；只重命名不能证明其新鲜。旧 spec 中能由代码直接回答的内容不再复制；不可丢失的约束迁移；有价值的历史归档并显式退出当前权威。
   **verify：** 保留的规范条款有唯一去向；失效内容不再被当前入口加载；没有新旧两份活词表/规格。〔对照：Opi `docs/CONTEXT.md` → `# Opi` 及各术语分组〕
3. **同步实际消费者与授权边界，再撤旧入口。** 目前 `AGENTS.md` 和技能手册明确依赖总规格/已注册 Phase 来源；仅换 AGENTS 链接会留下另一套权威。父任务需盘点技能、检查器、模板和双语引用，将其一并改到新契约。
   **verify：** 无悬空权威指针；检查器验证新结构而非继续强制旧结构；迁移不会让研究、tracker 候选或 Matt 的 `ready-for-agent` 自动取得实现权限。〔Opi `AGENTS.md` → `## Authority and context`、`## Skills and workflow`；`.agents/skills/README.md` → `## Boundary rules`、`## Durable artifact ownership`〕
4. **只加必要的 Matt 路径适配，暂不换执行引擎。** 在项目级明确 `docs/GLOSSARY.md` 的规范路径、ADR 路径，以及有无本地 tracker；不要改全局安装技能。没决定 tracker 时，不引入票据目录。既有 `.opi-impl-state.json` 仍由原 owner 管，迭代文档不复制完成百分比/运行账本。
   **verify：** 通过受控试用确认消费者读到唯一词表，不创建根 `GLOSSARY.md` 或意外 `.scratch/`；没有第二套计划执行状态机。〔Opi `AGENTS.md` → `## Skills and workflow`；`.agents/skills/README.md` → `## Maintainer composition note`〕
5. **按一次真实迭代验证维护负担。** 完成时复核本轮上下文；仅把新产生、值得长期保留的术语/决策提升到 glossary/ADR，其余随迭代结束成为历史。根据实际失败补指针/检查，而不是添一份更大的总规则。
   **verify：** 新 agent 能从 AGENTS 定位本轮范围和相关规则；旧迭代不会被当成全局有效要求；文档改动通过 `python scripts/opi-doc-check.py` 和 `git diff --check`，双语文件同步或说明不适用。〔检查要求来自 Opi `AGENTS.md` → `## Verification`、`## Authority and context`；增量改进依据 `retro/SKILL.md` → `## Steps`〕

**不要借迁移偷换安全契约。** 保留共享工作树保护、明确 Git/远程/付费授权、无凭据的隔离验证、工具/模型不能自增权限等边界。若用户还希望简化旧 Phase/准入机制，可以明确提出替代方案；“换文档体系”既不意味着必须永久保留旧流程，也不等于默许旁路现有 gate。该部分需父任务作产品/治理决定，本研究没有扩权。

## 6. 尚未验证的事项

- 未读取 `docs/opi-spec.md`，不能给出逐条保留/删除清单；未扫描全仓，不能声称引用迁移已完整。
- 未执行任何 skill，也未测试当前 harness 的 slash-command、Skill tool、路径覆盖或自动发现行为；引用的是安装文本，不是运行兼容性证明。
- 未检查 Git remote、全局技能版本/更新机制和工作树状态；不建议据此直接执行 setup 或远程发布。
- 本轮没有仓库或全局技能改动，没有创建子代理；仅保存这份外部研究报告。测试影响：`none`。仓库文档检查留给实际迁移完成后执行。
