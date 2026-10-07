# Pi 1.0.4 文档索引与实际审读范围

> 核查日期：2026-10-07。对象为本地固定1.0.4快照；版本差异见 [升级分析](version-changes.md)，主张状态见 [文档时效核查](documentation-audit.md)。

## 1 口径和证据

128份文本共33,270行：包内115份32,276行，根及隐藏目录13份994行。扫描.md/.mdx/.rst/.txt，排除.git/node_modules/dist；不把LICENSE、schema、代码注释或外部网页混进此计数。

相对1.0.2，4新增、18修改、1删除、106不变。每条当前记录绑定文件SHA-256及树摘要；机器文件见 [documentation-reading.json](documentation-reading.json)。1.0.2原索引的125项已逐条提取到 [历史审读记录](evidence/pi-1.0.2/documentation-reading.json)，继续继承的1.0.0证据也已迁入。

“全文重读”“重点段落”“继承”是不同深度，全文不等于执行示例或逐项验证全部历史条目。下面范围均按实际阅读记录；没有本轮精确范围的继承项不补造范围。机器枚举不算文本阅读，文档状态也不是由哈希决定。

根tui-plan.md已删除，旧方向按 [历史计划证据](evidence/pi-1.0.2/README.md#tui-plan) 保全，不冒充当前文档或仍受承诺的路线图。当前五份长设计共7,213行，新增env协议/语义另179行。

## 2 覆盖汇总

| 范围 | 份数 | 行数 | 分析入口 |
|---|---:|---:|---|
| 根及隐藏目录 | 13 | 994 | [整体](README.md) |
| agent | 2 | 1,332 | [agent](packages/agent.md) |
| ai | 2 | 4,202 | [ai](packages/ai.md) |
| chord | 3 | 1,314 | [chord](packages/chord.md) |
| client | 2 | 119 | [client](packages/client.md) |
| codemode | 2 | 239 | [codemode](packages/codemode.md) |
| coding-agent | 79 | 14,458 | [coding-agent](packages/coding-agent.md) |
| durable | 6 | 7,030 | [durable](packages/durable.md) |
| env | 4 | 231 | [env](packages/env.md) |
| evals | 1 | 129 | [evals](packages/evals.md) |
| mcp | 3 | 209 | [mcp](packages/mcp.md) |
| protocol | 2 | 90 | [protocol](packages/protocol.md) |
| server | 2 | 146 | [server](packages/server.md) |
| telemetry | 2 | 507 | [telemetry](packages/telemetry.md) |
| tui | 5 | 2,270 | [tui](packages/tui.md) |

## 3 逐文件记录

### 根及隐藏目录

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [.pi/prompts/cl.md](../../../.repo/pi-1.0.4/.pi/prompts/cl.md#L1) | 54 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/prompts/deslop.md](../../../.repo/pi-1.0.4/.pi/prompts/deslop.md#L1) | 67 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/prompts/is.md](../../../.repo/pi-1.0.4/.pi/prompts/is.md#L1) | 28 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/prompts/pr.md](../../../.repo/pi-1.0.4/.pi/prompts/pr.md#L1) | 37 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/prompts/sa.md](../../../.repo/pi-1.0.4/.pi/prompts/sa.md#L1) | 163 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/prompts/wr.md](../../../.repo/pi-1.0.4/.pi/prompts/wr.md#L1) | 41 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/skills/add-llm-provider.md](../../../.repo/pi-1.0.4/.pi/skills/add-llm-provider.md#L1) | 57 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/skills/interactive-testing.md](../../../.repo/pi-1.0.4/.pi/skills/interactive-testing.md#L1) | 19 | 安全规范／仓库工作流资源 | 内容与1.0.2一致；继承原审读：字节一致；继承旧版：标题与关键词筛查；不作为功能规划；当前相关结论按包报告复核，不宣称本轮重新全文阅读 |
| [.pi/skills/release.md](../../../.repo/pi-1.0.4/.pi/skills/release.md#L1) | 50 | 安全规范／仓库工作流资源 | 本轮全文重读；对照包集合、实现边界、发布流程和外部规划可见性；不执行其中命令；本次范围：1–50 |
| [AGENTS.md](../../../.repo/pi-1.0.4/AGENTS.md#L1) | 125 | 仓库工作约定 | 本轮全文重读；对照包集合、实现边界、发布流程和外部规划可见性；不执行其中命令；本次范围：1–125 |
| [CONTRIBUTING.md](../../../.repo/pi-1.0.4/CONTRIBUTING.md#L1) | 102 | 仓库说明／贡献规则 | 本轮全文重读；对照包集合、实现边界、发布流程和外部规划可见性；不执行其中命令；本次范围：1–102 |
| [README.md](../../../.repo/pi-1.0.4/README.md#L1) | 164 | 仓库说明／贡献规则 | 本轮全文重读；对照包集合、实现边界、发布流程和外部规划可见性；不执行其中命令；本次范围：1–164 |
| [SECURITY.md](../../../.repo/pi-1.0.4/SECURITY.md#L1) | 87 | 安全规范／仓库工作流资源 | 本轮全文重读；对照包集合、实现边界、发布流程和外部规划可见性；不执行其中命令；本次范围：1–87 |

### agent

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/agent/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md#L1) | 768 | changelog | 重读最新 1–70 行，核对空的 1.0.3/1.0.4 标题及 1.0.0 范围收缩。；本次范围：1–70 |
| [packages/agent/README.md](../../../.repo/pi-1.0.4/packages/agent/README.md#L1) | 564 | readme | 重点复核角色说明、prepareRequest/finishTurn、State 与低层 system 过滤示例、订阅结算。；本次范围：38–58, 124–142, 303–325, 532–564 |

### ai

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/ai/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/ai/CHANGELOG.md#L1) | 2,264 | changelog | 重读最新发行 1–90 行；旧发行历史分析从旧证据保留。；本次范围：1–90 |
| [packages/ai/README.md](../../../.repo/pi-1.0.4/packages/ai/README.md#L1) | 1,938 | readme | 重点复核 Azure 双协议、跨模型历史与 compat；另读取 quick start 片段。；本次范围：126–162, 1517–1558, 1810–1844 |

### chord

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/chord/PLANNING.md](../../../.repo/pi-1.0.4/packages/chord/PLANNING.md#L1) | 859 | 设计规划及历史交接 | 文档与包内源码摘要未变，继承旧报告全文审读；本次重读 generation replacement、工作包及 Definition of Done，重新判断未兑现方向。；本次范围：196–272, 636–752, 835–859 |
| [packages/chord/README.md](../../../.repo/pi-1.0.4/packages/chord/README.md#L1) | 264 | 现行 API 与设计说明 | 本次全文重新阅读；新 env Context 消费者及旧规划边界另按当前源码复核。；本次范围：1–264 |
| [packages/chord/src/delta/README.md](../../../.repo/pi-1.0.4/packages/chord/src/delta/README.md#L1) | 191 | delta 算法设计与取舍 | 文档与对应源码摘要未变；继承旧报告第 1–191 行完整审读，本轮未重复阅读全文。 |

### client

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/client/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/client/CHANGELOG.md#L1) | 47 | 发行历史 | 本次全文重新阅读，区分已交付变更、空版本标题与历史记录；未作运行验证。；本次范围：1–47 |
| [packages/client/README.md](../../../.repo/pi-1.0.4/packages/client/README.md#L1) | 72 | 现行包说明与 API 契约 | 本次全文重新阅读，结合当前关键路径或内容摘要核对现版；具体实现/规划结论见本包报告。；本次范围：1–72 |

### codemode

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/codemode/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/codemode/CHANGELOG.md#L1) | 49 | 发行历史 | 本次全文重新阅读，区分已交付变更、空版本标题与历史记录；未作运行验证。；本次范围：1–49 |
| [packages/codemode/README.md](../../../.repo/pi-1.0.4/packages/codemode/README.md#L1) | 190 | 现行包说明与 API 契约 | 本次全文重新阅读，结合当前关键路径或内容摘要核对现版；具体实现/规划结论见本包报告。；本次范围：1–190 |

### coding-agent

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/coding-agent/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md#L1) | 6,148 | 发行历史 | 本轮详读 1.0.4/1.0.3 新段与 1.0.2/1.0.1 背景；旧历史主题沿用证据，不冒称全文；本次范围：1–107 |
| [packages/coding-agent/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/README.md#L1) | 95 | 现行包说明、API 与设计边界 | 字节一致；继承 1.0.2 实际审读深度：全文复核；对变化契约追踪关联实现；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/cli-integration.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/cli-integration.md#L1) | 106 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/cli.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/cli.md#L1) | 333 | 当前 CLI 契约 | 全文阅读；工具模式、MCP 特例与 no-mcp 的 SDK/loader/会话接线复核；本次范围：1–333 |
| [packages/coding-agent/docs/codemode.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/codemode.md#L1) | 208 | 当前脚本工具契约 | 全文阅读；read 图片结果、落盘及 guidelines 对照实现；本次范围：1–208 |
| [packages/coding-agent/docs/compaction.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/compaction.md#L1) | 463 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/configuration.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/configuration.md#L1) | 47 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/containerization.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/containerization.md#L1) | 183 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/custom-provider.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/custom-provider.md#L1) | 173 | Provider 扩展设计与边界 | 字节未变；本轮全文复读；API 标识与 provider 标识分开，未把 Responses API 列举误读为 provider 唯一能力；本次范围：1–173 |
| [packages/coding-agent/docs/environment-variables.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/environment-variables.md#L1) | 98 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/extensions.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/extensions.md#L1) | 274 | 当前扩展 API 与边界 | 全文阅读；ToolLoadout/getPromptGuidelines、隐藏声明及同进程权限复核；本次范围：1–274 |
| [packages/coding-agent/docs/how-pi-works.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/how-pi-works.md#L1) | 49 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/index.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/index.md#L1) | 39 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/json.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/json.md#L1) | 226 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/keybindings.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/keybindings.md#L1) | 194 | 当前按键参考 | 全文阅读；Home/End 与 transcript 首尾动作语义；底层表由 TUI 包核对；本次范围：1–194 |
| [packages/coding-agent/docs/llama-cpp.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/llama-cpp.md#L1) | 114 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/mcp.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md#L1) | 272 | 当前 MCP 使用与集成契约 | 全文阅读；MCP allowlist、曝光、禁用与 SDK 的调用链复核；本次范围：1–272 |
| [packages/coding-agent/docs/message-types.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/message-types.md#L1) | 261 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/models.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md#L1) | 205 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：全文复核；对变化契约追踪关联实现；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/packages.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/packages.md#L1) | 129 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/prompt-templates.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/prompt-templates.md#L1) | 59 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/providers.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/providers.md#L1) | 200 | 当前 Provider 认证参考 | 字节未变；本轮全文复读；Azure 环境配置仍有效，未列 provider key 不推成旧名；本次范围：1–200 |
| [packages/coding-agent/docs/quickstart.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/quickstart.md#L1) | 136 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：全文复核；对变化契约追踪关联实现；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/rpc-commands.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/rpc-commands.md#L1) | 860 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/rpc-extension-ui.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/rpc-extension-ui.md#L1) | 200 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/rpc.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/rpc.md#L1) | 193 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/sdk.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L1) | 147 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/security.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L1) | 99 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/session-format.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/session-format.md#L1) | 293 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/sessions.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/sessions.md#L1) | 66 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/settings.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/settings.md#L1) | 169 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/shell-aliases.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/shell-aliases.md#L1) | 93 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/skills.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/skills.md#L1) | 93 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/slash-commands.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/slash-commands.md#L1) | 60 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/terminal-setup.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/terminal-setup.md#L1) | 219 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/termux.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/termux.md#L1) | 117 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/themes.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/themes.md#L1) | 136 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/tmux.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/tmux.md#L1) | 55 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/tui.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/tui.md#L1) | 125 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/usage.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/usage.md#L1) | 94 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/virtual-models.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/virtual-models.md#L1) | 114 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；关联实现核对；本轮不宣称重读全文。 |
| [packages/coding-agent/docs/windows.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/windows.md#L1) | 67 | 当前使用／集成契约 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：标题、首段与规划关键词筛查；命中处局部阅读；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/README.md#L1) | 39 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/README.md#L1) | 214 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/doom-overlay/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/doom-overlay/README.md#L1) | 46 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md#L1) | 8 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md#L1) | 5 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/plan-mode/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/plan-mode/README.md#L1) | 66 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/README.md#L1) | 177 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/agents/planner.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/agents/planner.md#L1) | 37 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/agents/reviewer.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/agents/reviewer.md#L1) | 35 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/agents/scout.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/agents/scout.md#L1) | 50 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/agents/worker.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/agents/worker.md#L1) | 24 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/prompts/implement.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/prompts/implement.md#L1) | 10 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md#L1) | 9 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/plugins/pi-example-plugin/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L1) | 38 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/examples/sdk/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/examples/sdk/README.md#L1) | 141 | 示例说明／示例 Agent 或 prompt | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；示例能力不视为项目路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/src/experimental/durable/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L1) | 66 | 实验设计、集成说明与 TODO | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；逐条判断 TODO 的前提和剩余范围；本轮不宣称重读全文。 |
| [packages/coding-agent/src/experimental/services/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L1) | 45 | 实验服务设计与后续方向 | 字节未变；本轮全文复读；逐项沿用并核对旧迁移前提、root-only、授权与分页边界；本次范围：1–45 |
| [packages/coding-agent/src/experimental/vacation/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L1) | 53 | 实验设计、集成说明与 TODO | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；逐条判断 TODO 的前提和剩余范围；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md#L1) | 9 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/long-name/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/long-name/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md#L1) | 7 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md#L1) | 11 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md#L1) | 3 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md#L1) | 10 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md#L1) | 8 | 解析器测试夹具 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；测试输入，非路线图；本轮不宣称重读全文。 |
| [packages/coding-agent/test/mcp-conformance/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-conformance/README.md#L1) | 45 | 测试运行指南 | 字节一致；继承 1.0.2 实际审读深度：字节一致；本轮全文重读，重核 CIMD 陈旧说明及对应实现；本轮不宣称重读全文。 |
| [packages/coding-agent/test/suite/README.md](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/README.md#L1) | 10 | 测试运行指南 | 字节一致；继承 1.0.2 实际审读深度：字节一致；继承旧版：全文；当前验证方法；本轮不宣称重读全文。 |

### durable

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/durable/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md#L1) | 63 | changelog | 本轮全文阅读。；本次范围：1–63 |
| [packages/durable/README.md](../../../.repo/pi-1.0.4/packages/durable/README.md#L1) | 613 | readme | 本轮全文阅读，特别核对新 progress、argv、reader、watch 和输出窗口；已补读中段以免截断冒称全文。；本次范围：1–613 |
| [packages/durable/docs/chord-delta-findings.md](../../../.repo/pi-1.0.4/packages/durable/docs/chord-delta-findings.md#L1) | 358 | research-findings | 重读复现实验和 future attempt 条件，核对其为历史研究、非当前性能或路线图。；本次范围：308–358 |
| [packages/durable/docs/pico-v5-chord-usage.md](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-chord-usage.md#L1) | 387 | design-guide | 重读 built-in view、commit delivery、生命周期和最终指南收尾；前面 canvas/diff/task 示例分析继承。；本次范围：334–387 |
| [packages/durable/docs/pico-v5-handoff.md](../../../.repo/pi-1.0.4/packages/durable/docs/pico-v5-handoff.md#L1) | 917 | implementation-plan | 重读阶段 19–23 及最终 API/包拆分说明，保持前期各阶段的继承分析。；本次范围：681–917 |
| [packages/durable/docs/spec.md](../../../.repo/pi-1.0.4/packages/durable/docs/spec.md#L1) | 4,692 | design-spec | 改动长文按列明范围重读：地位/旧 PR 状态、Pico 状态、新工具 API/输出窗口/argv、reader/watch 约束、规划尾部与开发顺序；其余前版详细分析保留。不是全文新读，也不是旧新逐行 diff。；本次范围：1–80, 289–313, 2655–2880, 3020–3088, 3270–3284, 3470–3503, 4630–4692 |

### env

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/env/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/env/CHANGELOG.md#L1) | 8 | changelog | 本轮首次全文阅读；1.0.4 首次发行（不是 1.0.3）。；本次范围：1–8 |
| [packages/env/README.md](../../../.repo/pi-1.0.4/packages/env/README.md#L1) | 44 | readme | 本轮首次全文阅读；包边界、SSH 示例、平台声明和测试资产前提均核对实现。；本次范围：1–44 |
| [packages/env/docs/protocol.md](../../../.repo/pi-1.0.4/packages/env/docs/protocol.md#L1) | 114 | protocol | 本轮首次全文阅读，逐项核对 framing、cancel、调度、心跳、文件/exec/watch 操作。；本次范围：1–114 |
| [packages/env/docs/semantics.md](../../../.repo/pi-1.0.4/packages/env/docs/semantics.md#L1) | 65 | semantics | 本轮首次全文阅读，核对 client/daemon 分工、Node 文件/进程语义与 watcher。；本次范围：1–65 |

### evals

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/evals/README.md](../../../.repo/pi-1.0.4/packages/evals/README.md#L1) | 129 | 评测协议与使用说明 | 字节未变；本轮全文复读；实验配对/隔离/缺失数据契约沿用，模型示例不是默认值；本次范围：1–129 |

### mcp

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/mcp/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/mcp/CHANGELOG.md#L1) | 52 | 发行历史 | 本次全文重新阅读，区分已交付变更、空版本标题与历史记录；未作运行验证。；本次范围：1–52 |
| [packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt](../../../.repo/pi-1.0.4/packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt#L1) | 21 | 第三方来源与许可 | 本次全文重新阅读；仅作来源/许可材料，不作为运行依赖或未来计划。；本次范围：1–21 |
| [packages/mcp/README.md](../../../.repo/pi-1.0.4/packages/mcp/README.md#L1) | 136 | 现行包说明与 API 契约 | 本次全文重新阅读，结合当前关键路径或内容摘要核对现版；具体实现/规划结论见本包报告。；本次范围：1–136 |

### protocol

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/protocol/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/protocol/CHANGELOG.md#L1) | 49 | 发行历史 | 本次全文重新阅读，区分已交付变更、空版本标题与历史记录；未作运行验证。；本次范围：1–49 |
| [packages/protocol/README.md](../../../.repo/pi-1.0.4/packages/protocol/README.md#L1) | 41 | 现行包说明与 API 契约 | 本次全文重新阅读，结合当前关键路径或内容摘要核对现版；具体实现/规划结论见本包报告。；本次范围：1–41 |

### server

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/server/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/server/CHANGELOG.md#L1) | 83 | 发行历史 | 本次全文重新阅读，区分已交付变更、空版本标题与历史记录；未作运行验证。；本次范围：1–83 |
| [packages/server/README.md](../../../.repo/pi-1.0.4/packages/server/README.md#L1) | 63 | 现行包说明与 API 契约 | 本次全文重新阅读，结合当前关键路径或内容摘要核对现版；具体实现/规划结论见本包报告。；本次范围：1–63 |

### telemetry

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/telemetry/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/telemetry/CHANGELOG.md#L1) | 43 | 发行历史 | 本次全文重新阅读，确认新增版本标题为空；历史设计判断沿用原分析。；本次范围：1–43 |
| [packages/telemetry/README.md](../../../.repo/pi-1.0.4/packages/telemetry/README.md#L1) | 464 | 观测设计契约与集成示例 | 摘要未变，继承旧报告全文审读；本次重新阅读 schema/Agent 示例，核对 agent 当前 exports 与既有移除记录。；本次范围：321–400 |

### tui

| 文档 | 行数 | 类型 | 本轮及继承口径 |
|---|---:|---|---|
| [packages/tui/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/tui/CHANGELOG.md#L1) | 1,247 | 发行历史 | 本次阅读第 1–36 行，核对 Home/End 调整、空 1.0.4 标题和已有图片修复；其余历史条目仅继承旧报告的分类筛查和重点章节阅读，不声称本轮全文重读。；本次范围：1–36 |
| [packages/tui/README.md](../../../.repo/pi-1.0.4/packages/tui/README.md#L1) | 947 | 终端 UI API 与平台用法 | 摘要未变，继承旧报告全文审读；本次重新阅读图片章节，核对无 setImageTranscoder 说明的已知遗漏。；本次范围：651–689 |
| [packages/tui/native/darwin/README.md](../../../.repo/pi-1.0.4/packages/tui/native/darwin/README.md#L1) | 22 | native 构建与平台说明 | 本次全文重新阅读；说明构建、平台降级和条件测试，没有独立路线图。；本次范围：1–22 |
| [packages/tui/native/linux/README.md](../../../.repo/pi-1.0.4/packages/tui/native/linux/README.md#L1) | 23 | native 构建与平台说明 | 本次全文重新阅读；说明构建、平台降级和条件测试，没有独立路线图。；本次范围：1–23 |
| [packages/tui/native/win32/README.md](../../../.repo/pi-1.0.4/packages/tui/native/win32/README.md#L1) | 31 | native 构建与平台说明 | 本次全文重新阅读；说明构建、平台降级和条件测试，没有独立路线图。；本次范围：1–31 |

## 4 使用限制

示例plan-mode/subagent/prompts是可加载的使用资源，测试fixtures是输入数据，名称含plan不意味着项目规划。长期方向需要结合README、spec、handoff、PLANNING和当前调用方逐主张判断；分类筛查的fixture不提高到设计审计。

所有当前材料都有记录，不表示每个源码分支、所有历史CHANGELOG和每个平台都已审计或运行。外部RFC未读取；旧源码原文缺失的比较限制与当前分析可独立完成的部分分开记录。
