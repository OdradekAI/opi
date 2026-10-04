# Pi 1.0.0 文档索引与审读范围

> 核查日期：2026-10-04。对象与 [整体报告](README.md) 相同，是本地固定快照；本清单不代表上游全部设计资料。

## 1 清单口径

共 **125 份、33,468 行**：`packages/` 下 111 份，根目录及 `.pi/` 下 14 份。扫描扩展名为 `.md`、`.mdx`、`.rst`、`.txt`，排除 `.git`、`node_modules`、`dist`；这个口径不包括无扩展名 LICENSE、代码注释、JSON schema 和外部网页。后几类在相关实现分析中按需使用，不计入这里的文档数量。

全量清点和规划信号筛查已覆盖清单内文件，但**清点、关键词筛查、全文阅读、逐项与实现复验是不同深度**。尤其较长的 CHANGELOG 和 22 份 coding-agent 使用文档没有宣称逐条全文复验。表内逐文件记录实际审读口径；“全文”也不代表执行了每个示例或验证了每项平台声明。

核心设计材料六份均全文阅读：Chord PLANNING、durable 的四份 docs 和根 tui-plan，合计 8,123 行。示例里的 plan-mode、planner、scout-and-plan 描述示例行为；测试 SKILL 文件是输入数据。它们不能仅因含有 plan/skill 等词而归入项目路线图。

时效状态与证据见 [专项核查](documentation-audit.md) 及各包报告末尾章节。表中的文档类型不是整份文件的有效／过时判决；同一文件可以混合仍有效的设计、已完成的计划和已被替代的接口。

## 2 覆盖汇总

| 范围 | 文件数 | 行数 | 分析入口 |
|---|---:|---:|---|
| 根目录及 .pi/ | 14 | 1,945 | [专项核查](documentation-audit.md) |
| agent | 2 | 1,324 | [agent 报告](packages/agent.md) |
| ai | 2 | 4,156 | [ai 报告](packages/ai.md) |
| chord | 3 | 1,314 | [chord 报告](packages/chord.md) |
| client | 2 | 111 | [client 报告](packages/client.md) |
| codemode | 2 | 219 | [codemode 报告](packages/codemode.md) |
| coding-agent | 79 | 14,246 | [coding-agent 报告](packages/coding-agent.md) |
| durable | 6 | 6,867 | [durable 报告](packages/durable.md) |
| evals | 1 | 129 | [evals 报告](packages/evals.md) |
| mcp | 3 | 189 | [mcp 报告](packages/mcp.md) |
| protocol | 2 | 82 | [protocol 报告](packages/protocol.md) |
| server | 2 | 138 | [server 报告](packages/server.md) |
| telemetry | 2 | 499 | [telemetry 报告](packages/telemetry.md) |
| tui | 5 | 2,249 | [tui 报告](packages/tui.md) |

## 3 逐文件清单

### 根目录及 .pi/

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [.pi/prompts/cl.md](../../../.repo/pi-1.0.0/.pi/prompts/cl.md#L1) | 54 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/deslop.md](../../../.repo/pi-1.0.0/.pi/prompts/deslop.md#L1) | 67 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/is.md](../../../.repo/pi-1.0.0/.pi/prompts/is.md#L1) | 28 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/pr.md](../../../.repo/pi-1.0.0/.pi/prompts/pr.md#L1) | 37 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/sa.md](../../../.repo/pi-1.0.0/.pi/prompts/sa.md#L1) | 163 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/wr.md](../../../.repo/pi-1.0.0/.pi/prompts/wr.md#L1) | 41 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/skills/add-llm-provider.md](../../../.repo/pi-1.0.0/.pi/skills/add-llm-provider.md#L1) | 57 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/skills/interactive-testing.md](../../../.repo/pi-1.0.0/.pi/skills/interactive-testing.md#L1) | 19 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [.pi/skills/release.md](../../../.repo/pi-1.0.0/.pi/skills/release.md#L1) | 48 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [AGENTS.md](../../../.repo/pi-1.0.0/AGENTS.md#L1) | 125 | 仓库工作约定 | 全文；工作边界上下文，非路线图 |
| [CONTRIBUTING.md](../../../.repo/pi-1.0.0/CONTRIBUTING.md#L1) | 102 | 仓库说明／贡献规则 | 标题与关键词筛查；外部规划入口局部核对 |
| [README.md](../../../.repo/pi-1.0.0/README.md#L1) | 116 | 仓库说明／贡献规则 | 标题与关键词筛查；外部规划入口局部核对 |
| [SECURITY.md](../../../.repo/pi-1.0.0/SECURITY.md#L1) | 87 | 安全规范／仓库工作流资源 | 标题与关键词筛查；不作为功能规划 |
| [tui-plan.md](../../../.repo/pi-1.0.0/tui-plan.md#L1) | 1,001 | 设计计划、非目标、后续方向 | 全文；逐项实现与时效对照 |

### agent

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/agent/CHANGELOG.md#L1) | 760 | 发行历史 | 全篇标题／关键词筛查；重点详读 1–260、359–495 |
| [README.md](../../../.repo/pi-1.0.0/packages/agent/README.md#L1) | 564 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### ai

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/ai/CHANGELOG.md#L1) | 2,218 | 发行历史 | 全篇标题／关键词筛查；重点详读 1–250、650–733 |
| [README.md](../../../.repo/pi-1.0.0/packages/ai/README.md#L1) | 1,938 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### chord

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [PLANNING.md](../../../.repo/pi-1.0.0/packages/chord/PLANNING.md#L1) | 859 | 设计计划与工作包 | 全文；WP0–WP8 与现行实现逐项对照 |
| [README.md](../../../.repo/pi-1.0.0/packages/chord/README.md#L1) | 264 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |
| [src/delta/README.md](../../../.repo/pi-1.0.0/packages/chord/src/delta/README.md#L1) | 191 | 现行 delta 设计与使用契约 | 全文；与旧计划／实验文字对照 |

### client

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/client/CHANGELOG.md#L1) | 39 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/client/README.md#L1) | 72 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### codemode

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/codemode/CHANGELOG.md#L1) | 29 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/codemode/README.md#L1) | 190 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### coding-agent

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/coding-agent/CHANGELOG.md#L1) | 6,044 | 发行历史 | 全篇标题／关键词筛查；重点详读 1–88、107–185、357–374、714–731、748–760、1283–1308 |
| [README.md](../../../.repo/pi-1.0.0/packages/coding-agent/README.md#L1) | 70 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |
| [docs/cli-integration.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/cli-integration.md#L1) | 106 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/cli.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/cli.md#L1) | 319 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/codemode.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/codemode.md#L1) | 207 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/compaction.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/compaction.md#L1) | 463 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/configuration.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/configuration.md#L1) | 47 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/containerization.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/containerization.md#L1) | 183 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/custom-provider.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/custom-provider.md#L1) | 173 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/environment-variables.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/environment-variables.md#L1) | 98 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/extensions.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/extensions.md#L1) | 270 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/how-pi-works.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/how-pi-works.md#L1) | 49 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/index.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/index.md#L1) | 39 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/json.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/json.md#L1) | 226 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/keybindings.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/keybindings.md#L1) | 194 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/llama-cpp.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/llama-cpp.md#L1) | 114 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/mcp.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/mcp.md#L1) | 248 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/message-types.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/message-types.md#L1) | 261 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/models.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/models.md#L1) | 179 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/packages.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/packages.md#L1) | 129 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/prompt-templates.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/prompt-templates.md#L1) | 59 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/providers.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/providers.md#L1) | 200 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/quickstart.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/quickstart.md#L1) | 122 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/rpc-commands.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/rpc-commands.md#L1) | 860 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/rpc-extension-ui.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/rpc-extension-ui.md#L1) | 200 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/rpc.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/rpc.md#L1) | 193 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/sdk.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/sdk.md#L1) | 147 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/security.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/security.md#L1) | 99 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/session-format.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/session-format.md#L1) | 293 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/sessions.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/sessions.md#L1) | 66 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/settings.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/settings.md#L1) | 169 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/shell-aliases.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/shell-aliases.md#L1) | 93 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/skills.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/skills.md#L1) | 93 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/slash-commands.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/slash-commands.md#L1) | 60 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/terminal-setup.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/terminal-setup.md#L1) | 219 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/termux.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/termux.md#L1) | 117 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/themes.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/themes.md#L1) | 136 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/tmux.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/tmux.md#L1) | 55 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/tui.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/tui.md#L1) | 125 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/usage.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/usage.md#L1) | 94 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/virtual-models.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/virtual-models.md#L1) | 114 | 当前使用／集成契约 | 全文；关联实现核对 |
| [docs/windows.md](../../../.repo/pi-1.0.0/packages/coding-agent/docs/windows.md#L1) | 67 | 当前使用／集成契约 | 标题、首段与规划关键词筛查；命中处局部阅读 |
| [examples/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/README.md#L1) | 39 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/README.md#L1) | 214 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/doom-overlay/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/doom-overlay/README.md#L1) | 46 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/dynamic-resources/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md#L1) | 8 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/dynamic-resources/dynamic.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md#L1) | 5 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/plan-mode/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/plan-mode/README.md#L1) | 66 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/README.md#L1) | 177 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/planner.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/agents/planner.md#L1) | 37 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/reviewer.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/agents/reviewer.md#L1) | 35 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/scout.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/agents/scout.md#L1) | 50 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/worker.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/agents/worker.md#L1) | 24 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/implement-and-review.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/implement.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/prompts/implement.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/scout-and-plan.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md#L1) | 9 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/plugins/pi-example-plugin/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L1) | 38 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [examples/sdk/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/examples/sdk/README.md#L1) | 141 | 示例说明／示例 Agent 或 prompt | 全文；示例能力不视为项目路线图 |
| [src/experimental/durable/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/src/experimental/durable/README.md#L1) | 66 | 实验设计、集成说明与 TODO | 全文；逐条判断 TODO 的前提和剩余范围 |
| [src/experimental/services/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/src/experimental/services/README.md#L1) | 45 | 实验设计、集成说明与 TODO | 全文；逐条判断 TODO 的前提和剩余范围 |
| [src/experimental/vacation/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/src/experimental/vacation/README.md#L1) | 53 | 实验设计、集成说明与 TODO | 全文；逐条判断 TODO 的前提和剩余范围 |
| [test/fixtures/skills-collision/first/calendar/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills-collision/second/calendar/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/consecutive-hyphens/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/disable-model-invocation/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md#L1) | 9 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/invalid-name-chars/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/invalid-yaml/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/long-name/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/long-name/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/missing-description/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md#L1) | 7 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/multiline-description/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md#L1) | 11 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/name-mismatch/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/nested/child-skill/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/no-frontmatter/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md#L1) | 3 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/root-skill-preferred/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md#L1) | 3 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md#L1) | 3 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/unknown-field/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md#L1) | 10 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/fixtures/skills/valid-skill/SKILL.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 全文；测试输入，非路线图 |
| [test/mcp-conformance/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/mcp-conformance/README.md#L1) | 45 | 测试运行指南 | 全文；当前验证方法 |
| [test/suite/README.md](../../../.repo/pi-1.0.0/packages/coding-agent/test/suite/README.md#L1) | 10 | 测试运行指南 | 全文；当前验证方法 |

### durable

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/durable/CHANGELOG.md#L1) | 7 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/durable/README.md#L1) | 597 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |
| [docs/chord-delta-findings.md](../../../.repo/pi-1.0.0/packages/durable/docs/chord-delta-findings.md#L1) | 358 | 历史实验分析 | 全文；原意、阶段更替与当前实现对照 |
| [docs/pico-v5-chord-usage.md](../../../.repo/pi-1.0.0/packages/durable/docs/pico-v5-chord-usage.md#L1) | 387 | 设计规范／实现交接 | 全文；原意、阶段更替与当前实现对照 |
| [docs/pico-v5-handoff.md](../../../.repo/pi-1.0.0/packages/durable/docs/pico-v5-handoff.md#L1) | 917 | 设计规范／实现交接 | 全文；原意、阶段更替与当前实现对照 |
| [docs/spec.md](../../../.repo/pi-1.0.0/packages/durable/docs/spec.md#L1) | 4,601 | 设计规范／实现交接 | 全文；原意、阶段更替与当前实现对照 |

### evals

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [README.md](../../../.repo/pi-1.0.0/packages/evals/README.md#L1) | 129 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### mcp

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/mcp/CHANGELOG.md#L1) | 32 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [LICENSES/modelcontextprotocol-typescript-sdk.txt](../../../.repo/pi-1.0.0/packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt#L1) | 21 | 第三方许可证 | 清点与类型筛查；不作为功能规划 |
| [README.md](../../../.repo/pi-1.0.0/packages/mcp/README.md#L1) | 136 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### protocol

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/protocol/CHANGELOG.md#L1) | 41 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/protocol/README.md#L1) | 41 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### server

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/server/CHANGELOG.md#L1) | 75 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/server/README.md#L1) | 63 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### telemetry

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/telemetry/CHANGELOG.md#L1) | 35 | 发行历史 | 全文；核对相关演进，非当前待办 |
| [README.md](../../../.repo/pi-1.0.0/packages/telemetry/README.md#L1) | 464 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |

### tui

| 文档 | 行数 | 类型 | 本次审读口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.0/packages/tui/CHANGELOG.md#L1) | 1,226 | 发行历史 | 全篇标题／关键词筛查；重点详读 1–190（0.84.0 至 1.0.0 相关条目） |
| [README.md](../../../.repo/pi-1.0.0/packages/tui/README.md#L1) | 947 | 现行包说明、API 与设计边界 | 全文；核查设计意图、限制与具体示例 |
| [native/darwin/README.md](../../../.repo/pi-1.0.0/packages/tui/native/darwin/README.md#L1) | 22 | 原生构建与验证说明 | 全文；当前平台边界，非路线图 |
| [native/linux/README.md](../../../.repo/pi-1.0.0/packages/tui/native/linux/README.md#L1) | 23 | 原生构建与验证说明 | 全文；当前平台边界，非路线图 |
| [native/win32/README.md](../../../.repo/pi-1.0.0/packages/tui/native/win32/README.md#L1) | 31 | 原生构建与验证说明 | 全文；当前平台边界，非路线图 |

## 4 使用边界

此索引回答“哪些材料存在、以什么深度检查过”，不单独证明某项能力成立。能力与规划状态必须回到包报告中的文档原句、实现、导出和测试证据。CHANGELOG 的旧版本条目保留历史价值，不能一律改读为当前接口；也不能根据文件修改时间或标题将整篇文档作废。

根 README 与 CONTRIBUTING 提到外部 RFC，其中部分不公开。本轮没有访问这些网页或调查后续版本，因此没有承诺已覆盖 Pi 的全部长期计划。文档所列测试、构建、benchmark、容器和 provider 调用也没有在本轮执行。
