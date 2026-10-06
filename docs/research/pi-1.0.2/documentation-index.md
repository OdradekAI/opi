# Pi 1.0.2 文档索引与审读范围

> 核查日期：2026-10-05。对象：本地 `.repo/pi-1.0.2`；版本差异口径见 [升级核查](version-changes.md)。

## 1 清单口径

共 **125 份、33,781 行**：`packages/` 下 111 份，根目录及 `.pi/` 下 14 份。扫描 `.md/.mdx/.rst/.txt`，排除 `.git/node_modules/dist`；不含无扩展名 LICENSE、源码注释、JSON schema 和外部网页。

相对旧摘要，**26 份文档修改，99 份字节一致，无文档新增或删除**。对字节一致材料保留上一轮已完成的审读证据；表中明确标为“继承”，不能理解成本轮重新逐行阅读。变化文档按新内容复核，相关实现改变时重新判断原结论。

六份核心设计材料合计 **8,150 行**。其中 durable `spec.md` 发生变化，另外五份字节一致；没有仅凭规划文件不变就推断实现状态不变。文档类型、阅读深度与时效判定是不同维度，同一篇文档可以混合有效设计、已兑现计划与陈旧状态句。

以下“全文”只表示阅读文本，不表示执行示例、测试、模型调用或平台验证。长 CHANGELOG 只重新阅读新版本条目及相关历史段落；旧版详细行号范围已提取保存在 [历史文档审读记录](evidence/pi-1.0.0/documentation-reading.json)，不把旧范围当成新文件行号。

## 2 覆盖汇总

| 范围 | 文件数 | 行数 | 变化文档数 | 分析入口 |
|---|---:|---:|---:|---|
| 根目录及 .pi/ | 14 | 1,995 | 3 | [专项核查](documentation-audit.md) |
| agent | 2 | 1,328 | 1 | [agent](packages/agent.md) |
| ai | 2 | 4,182 | 2 | [ai](packages/ai.md) |
| chord | 3 | 1,314 | 0 | [chord](packages/chord.md) |
| client | 2 | 115 | 1 | [client](packages/client.md) |
| codemode | 2 | 227 | 2 | [codemode](packages/codemode.md) |
| coding-agent | 79 | 14,393 | 9 | [coding-agent](packages/coding-agent.md) |
| durable | 6 | 6,904 | 3 | [durable](packages/durable.md) |
| evals | 1 | 129 | 0 | [evals](packages/evals.md) |
| mcp | 3 | 201 | 1 | [mcp](packages/mcp.md) |
| protocol | 2 | 86 | 1 | [protocol](packages/protocol.md) |
| server | 2 | 142 | 1 | [server](packages/server.md) |
| telemetry | 2 | 503 | 1 | [telemetry](packages/telemetry.md) |
| tui | 5 | 2,262 | 1 | [tui](packages/tui.md) |

## 3 逐文件清单

### 根目录及 .pi/

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [.pi/prompts/cl.md](../../../.repo/pi-1.0.2/.pi/prompts/cl.md#L1) | 54 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/deslop.md](../../../.repo/pi-1.0.2/.pi/prompts/deslop.md#L1) | 67 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/is.md](../../../.repo/pi-1.0.2/.pi/prompts/is.md#L1) | 28 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/pr.md](../../../.repo/pi-1.0.2/.pi/prompts/pr.md#L1) | 37 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/sa.md](../../../.repo/pi-1.0.2/.pi/prompts/sa.md#L1) | 163 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/prompts/wr.md](../../../.repo/pi-1.0.2/.pi/prompts/wr.md#L1) | 41 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/skills/add-llm-provider.md](../../../.repo/pi-1.0.2/.pi/skills/add-llm-provider.md#L1) | 57 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/skills/interactive-testing.md](../../../.repo/pi-1.0.2/.pi/skills/interactive-testing.md#L1) | 19 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [.pi/skills/release.md](../../../.repo/pi-1.0.2/.pi/skills/release.md#L1) | 50 | 安全规范／仓库工作流资源 | 全文复核；对变化契约追踪关联实现 |
| [AGENTS.md](../../../.repo/pi-1.0.2/AGENTS.md#L1) | 125 | 仓库工作约定 | 全文复核；对变化契约追踪关联实现 |
| [CONTRIBUTING.md](../../../.repo/pi-1.0.2/CONTRIBUTING.md#L1) | 102 | 仓库说明／贡献规则 | 字节一致；继承旧版：标题与关键词筛查；外部规划入口局部核对 |
| [README.md](../../../.repo/pi-1.0.2/README.md#L1) | 164 | 仓库说明／贡献规则 | 全文复核；对变化契约追踪关联实现 |
| [SECURITY.md](../../../.repo/pi-1.0.2/SECURITY.md#L1) | 87 | 安全规范／仓库工作流资源 | 字节一致；继承旧版：标题与关键词筛查；不作为功能规划 |
| [tui-plan.md](../../../.repo/pi-1.0.2/tui-plan.md#L1) | 1,001 | 设计计划、非目标、后续方向 | 字节一致；继承旧版：全文；逐项实现与时效对照；本轮另复读滚轮段 524–532 |

### agent

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/agent/CHANGELOG.md#L1) | 764 | 发行历史 | 详读 1.0.2 / 1.0.1 与涉及的历史段；其他历史条目不逐项重审 |
| [README.md](../../../.repo/pi-1.0.2/packages/agent/README.md#L1) | 564 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### ai

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/ai/CHANGELOG.md#L1) | 2,244 | 发行历史 | 详读 1.0.2 / 1.0.1 与涉及的历史段；其他历史条目不逐项重审 |
| [README.md](../../../.repo/pi-1.0.2/packages/ai/README.md#L1) | 1,938 | 现行包说明、API 与设计边界 | 全文复核；对变化契约追踪关联实现 |

### chord

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [PLANNING.md](../../../.repo/pi-1.0.2/packages/chord/PLANNING.md#L1) | 859 | 设计计划与工作包 | 字节一致；继承旧版：全文；WP0–WP8 与现行实现逐项对照 |
| [README.md](../../../.repo/pi-1.0.2/packages/chord/README.md#L1) | 264 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |
| [src/delta/README.md](../../../.repo/pi-1.0.2/packages/chord/src/delta/README.md#L1) | 191 | 现行 delta 设计与使用契约 | 字节一致；继承旧版：全文；与旧计划／实验文字对照 |

### client

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/client/CHANGELOG.md#L1) | 43 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/client/README.md#L1) | 72 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### codemode

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/codemode/CHANGELOG.md#L1) | 37 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/codemode/README.md#L1) | 190 | 现行包说明、API 与设计边界 | 全文复核；对变化契约追踪关联实现 |

### coding-agent

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/coding-agent/CHANGELOG.md#L1) | 6,098 | 发行历史 | 详读 1.0.2 / 1.0.1 与涉及的历史段；其他历史条目不逐项重审 |
| [README.md](../../../.repo/pi-1.0.2/packages/coding-agent/README.md#L1) | 95 | 现行包说明、API 与设计边界 | 全文复核；对变化契约追踪关联实现 |
| [docs/cli-integration.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/cli-integration.md#L1) | 106 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/cli.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/cli.md#L1) | 321 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/codemode.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/codemode.md#L1) | 207 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/compaction.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/compaction.md#L1) | 463 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/configuration.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/configuration.md#L1) | 47 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/containerization.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/containerization.md#L1) | 183 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/custom-provider.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/custom-provider.md#L1) | 173 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/environment-variables.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/environment-variables.md#L1) | 98 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/extensions.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/extensions.md#L1) | 274 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/how-pi-works.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/how-pi-works.md#L1) | 49 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/index.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/index.md#L1) | 39 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/json.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/json.md#L1) | 226 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/keybindings.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/keybindings.md#L1) | 194 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/llama-cpp.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/llama-cpp.md#L1) | 114 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/mcp.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/mcp.md#L1) | 270 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/message-types.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/message-types.md#L1) | 261 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/models.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/models.md#L1) | 205 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/packages.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/packages.md#L1) | 129 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/prompt-templates.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/prompt-templates.md#L1) | 59 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/providers.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/providers.md#L1) | 200 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/quickstart.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/quickstart.md#L1) | 136 | 当前使用／集成契约 | 全文复核；对变化契约追踪关联实现 |
| [docs/rpc-commands.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/rpc-commands.md#L1) | 860 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/rpc-extension-ui.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/rpc-extension-ui.md#L1) | 200 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/rpc.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/rpc.md#L1) | 193 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/sdk.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/sdk.md#L1) | 147 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/security.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/security.md#L1) | 99 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/session-format.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/session-format.md#L1) | 293 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/sessions.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/sessions.md#L1) | 66 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/settings.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/settings.md#L1) | 169 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/shell-aliases.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/shell-aliases.md#L1) | 93 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/skills.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/skills.md#L1) | 93 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/slash-commands.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/slash-commands.md#L1) | 60 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/terminal-setup.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/terminal-setup.md#L1) | 219 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/termux.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/termux.md#L1) | 117 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/themes.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/themes.md#L1) | 136 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/tmux.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/tmux.md#L1) | 55 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/tui.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/tui.md#L1) | 125 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/usage.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/usage.md#L1) | 94 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [docs/virtual-models.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/virtual-models.md#L1) | 114 | 当前使用／集成契约 | 字节一致；继承旧版：全文；关联实现核对 |
| [docs/windows.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/windows.md#L1) | 67 | 当前使用／集成契约 | 字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读 |
| [examples/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/README.md#L1) | 39 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/README.md#L1) | 214 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/doom-overlay/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/doom-overlay/README.md#L1) | 46 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/dynamic-resources/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md#L1) | 8 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/dynamic-resources/dynamic.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md#L1) | 5 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/plan-mode/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/plan-mode/README.md#L1) | 66 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/README.md#L1) | 177 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/planner.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/agents/planner.md#L1) | 37 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/reviewer.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/agents/reviewer.md#L1) | 35 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/scout.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/agents/scout.md#L1) | 50 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/agents/worker.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/agents/worker.md#L1) | 24 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/implement-and-review.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/implement.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/prompts/implement.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/extensions/subagent/prompts/scout-and-plan.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md#L1) | 9 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/plugins/pi-example-plugin/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L1) | 38 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [examples/sdk/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/examples/sdk/README.md#L1) | 141 | 示例说明／示例 Agent 或 prompt | 字节一致；继承旧版：全文；示例能力不视为项目路线图 |
| [src/experimental/durable/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/durable/README.md#L1) | 66 | 实验设计、集成说明与 TODO | 字节一致；继承旧版：全文；逐条判断 TODO 的前提和剩余范围 |
| [src/experimental/services/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/services/README.md#L1) | 45 | 实验设计、集成说明与 TODO | 字节一致；本轮全文重读，逐条复核 TODO 前提和剩余范围 |
| [src/experimental/vacation/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/src/experimental/vacation/README.md#L1) | 53 | 实验设计、集成说明与 TODO | 字节一致；继承旧版：全文；逐条判断 TODO 的前提和剩余范围 |
| [test/fixtures/skills-collision/first/calendar/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills-collision/second/calendar/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/consecutive-hyphens/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/disable-model-invocation/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md#L1) | 9 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/invalid-name-chars/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/invalid-yaml/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/long-name/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/long-name/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/missing-description/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md#L1) | 7 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/multiline-description/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md#L1) | 11 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/name-mismatch/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/nested/child-skill/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/no-frontmatter/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/root-skill-preferred/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/unknown-field/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md#L1) | 10 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/fixtures/skills/valid-skill/SKILL.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承旧版：全文；测试输入，非路线图 |
| [test/mcp-conformance/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/mcp-conformance/README.md#L1) | 45 | 测试运行指南 | 字节一致；本轮全文重读，重核 CIMD 陈旧说明及对应实现 |
| [test/suite/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/test/suite/README.md#L1) | 10 | 测试运行指南 | 字节一致；继承旧版：全文；当前验证方法 |

### durable

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/durable/CHANGELOG.md#L1) | 15 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/durable/README.md#L1) | 599 | 现行包说明、API 与设计边界 | 全文复核；对变化契约追踪关联实现 |
| [docs/chord-delta-findings.md](../../../.repo/pi-1.0.2/packages/durable/docs/chord-delta-findings.md#L1) | 358 | 历史实验分析 | 字节一致；继承旧版：全文；原意、阶段更替与当前实现对照 |
| [docs/pico-v5-chord-usage.md](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-chord-usage.md#L1) | 387 | 设计规范／实现交接 | 字节一致；继承旧版：全文；原意、阶段更替与当前实现对照 |
| [docs/pico-v5-handoff.md](../../../.repo/pi-1.0.2/packages/durable/docs/pico-v5-handoff.md#L1) | 917 | 设计规范／实现交接 | 字节一致；继承旧版：全文；原意、阶段更替与当前实现对照 |
| [docs/spec.md](../../../.repo/pi-1.0.2/packages/durable/docs/spec.md#L1) | 4,628 | 设计规范／实现交接 | 全文复核；重点对照 ProviderSession 与恢复、view、compaction 实现 |

### evals

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [README.md](../../../.repo/pi-1.0.2/packages/evals/README.md#L1) | 129 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### mcp

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/mcp/CHANGELOG.md#L1) | 44 | 发行历史 | 全文；核对新增发行标题和条目 |
| [LICENSES/modelcontextprotocol-typescript-sdk.txt](../../../.repo/pi-1.0.2/packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt#L1) | 21 | 第三方许可证 | 字节一致；继承旧版：清点与类型筛查；不作为功能规划 |
| [README.md](../../../.repo/pi-1.0.2/packages/mcp/README.md#L1) | 136 | 现行包说明、API 与设计边界 | 字节一致；本轮仍全文重读，核查 issuer 示例和 CIMD 说明缺口 |

### protocol

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/protocol/CHANGELOG.md#L1) | 45 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/protocol/README.md#L1) | 41 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### server

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/server/CHANGELOG.md#L1) | 79 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/server/README.md#L1) | 63 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### telemetry

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/telemetry/CHANGELOG.md#L1) | 39 | 发行历史 | 全文；核对新增发行标题和条目 |
| [README.md](../../../.repo/pi-1.0.2/packages/telemetry/README.md#L1) | 464 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例 |

### tui

| 文档 | 行数 | 类型 | 1.0.2 复核口径 |
|---|---:|---|---|
| [CHANGELOG.md](../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md#L1) | 1,239 | 发行历史 | 详读 1.0.2 / 1.0.1 与涉及的历史段；其他历史条目不逐项重审 |
| [README.md](../../../.repo/pi-1.0.2/packages/tui/README.md#L1) | 947 | 现行包说明、API 与设计边界 | 字节一致；继承旧版：全文；核查设计意图、限制与具体示例；本轮另复读图片段 658–686 |
| [native/darwin/README.md](../../../.repo/pi-1.0.2/packages/tui/native/darwin/README.md#L1) | 22 | 原生构建与验证说明 | 字节一致；继承旧版：全文；当前平台边界，非路线图 |
| [native/linux/README.md](../../../.repo/pi-1.0.2/packages/tui/native/linux/README.md#L1) | 23 | 原生构建与验证说明 | 字节一致；继承旧版：全文；当前平台边界，非路线图 |
| [native/win32/README.md](../../../.repo/pi-1.0.2/packages/tui/native/win32/README.md#L1) | 31 | 原生构建与验证说明 | 字节一致；继承旧版：全文；当前平台边界，非路线图 |

## 4 使用边界

本索引说明存在的材料及审读来源，不独立证明某项能力已实现。具体结论见 [文档时效核查](documentation-audit.md) 和包报告。示例的 planner/plan-mode 是示例功能，测试 fixtures 是输入数据；两者均不能因名称包含 plan 而当成项目路线图。

外部 RFC 及网站最新文档没有纳入本地快照核查。源码身份由 [快照清单](source-manifest.json) 固定；本轮没有运行文档中的构建、benchmark、容器或 provider 命令。
