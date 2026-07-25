# pi、Claude Code 与 Codex CLI 的项目信任语义

日期：2026-07-24

## 结论

- pi 0.80.6、Claude Code、Codex CLI 都有“进入项目后是否采纳项目提供的配置/扩展能力”的信任语义。
- 只有 pi 0.80.6 提供内置 `/trust`。Claude Code 和 Codex CLI 的官方命令面都没有内置 `/trust`。
- pi 0.80.6 的 `/trust` 不是“中途改变当前会话信任”。它只修改持久化决策，明确要求重启后生效。
- pi 0.80.6 只识别精确的 `/trust`，没有 `/trust <choice>` 解析；该命令的选择器也不提供 session-only 选项。
- 因此，Phase 15 设计中的“`/trust <choice>` + 当前会话生效 + 下一次资源发现采用新决策”并不是 pi、Claude Code 或 Codex CLI 的现有语义。

## pi 0.80.6 如何决策

`resolveProjectTrusted()` 的优先级如下：

1. 一次性 CLI override：`--approve` / `--no-approve`。
2. 如果项目没有需要信任的资源，直接返回 trusted。
3. 依次询问预信任阶段已加载的 user/global 与 CLI 扩展；第一个 yes/no 决策生效，可选择记住。
4. 查询 `~/.pi/agent/trust.json`；当前 canonical path 或最近父目录的决策优先。
5. 应用 global-only `defaultProjectTrust = ask | always | never`。
6. `ask` 且有交互 UI 时显示选择器；无 UI 或取消时返回 untrusted。

实现依据：

- [project-trust.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/core/project-trust.ts#L46) 直接编码了 override → 无受保护资源 → extension → store → default → UI/headless 的顺序。
- [trust-manager.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/core/trust-manager.ts#L65) 定义启动选择：Trust、Trust parent、Trust this session、Do not trust、Do not trust this session。
- [security.md](../../.repo/pi-0.80.6/packages/coding-agent/docs/security.md#L18) 说明 canonical path、最近父目录决策、全局默认以及 headless 行为。

pi 的 trust 只决定项目资源是否加载，不是工具执行授权或 OS sandbox。受保护面包括 `.pi/settings.json`、项目扩展、skills、prompts、themes、system prompt fragments、项目 packages；pi 0.80.6 的 `AGENTS.md`/`CLAUDE.md` 仍不受 trust gate 约束，这是 Opi Phase 15 有意偏离 pi 的地方。

## pi 0.80.6 的 `/trust` 实际语义

源码与文档一致：

- 命令分发只匹配 `text === "/trust"`；没有参数解析。[interactive-mode.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L2708)
- `/trust` 调用 `getProjectTrustOptions(cwd)`，没有传 `includeSessionOnly: true`，所以只显示 Trust、Trust parent、Do not trust 三个持久化选项。[interactive-mode.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L4401)
- 选择后只写 trust store，并提示 “Restart pi for this to take effect”。[interactive-mode.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L4410)
- 命令注册文字就是“Save project trust decision for future sessions”。[slash-commands.ts](../../.repo/pi-0.80.6/packages/coding-agent/src/core/slash-commands.ts#L34)
- 用户文档明确说明当前会话不 reload，需重启。[usage.md](../../.repo/pi-0.80.6/packages/coding-agent/docs/usage.md#L127)

启动 prompt 与 `/trust` 不能混为一谈：启动 prompt 有五个选择并可做 session-only 决策；`/trust` 只是未来启动所用的持久化决策编辑器。

## Claude Code

Claude Code 有 workspace trust，但没有官方文档化的内置 `/trust`：

- 首次进入 codebase 时显示 workspace trust；项目目录的接受结果会持久化，直接在 home 目录启动则只在当前会话有效。[Security safeguards](https://code.claude.com/docs/en/security#additional-safeguards)
- trust 控制项目配置所提供的能力，例如项目 allow rules、hooks、skills 的 `allowed-tools`、MCP/插件相关配置。[Workspace trust](https://code.claude.com/docs/en/permissions#project-allow-rules-and-workspace-trust)
- `/cd` 进入尚未信任的目录时会触发信任提示。[Commands](https://code.claude.com/docs/en/commands)
- 官方完整命令表没有 `/trust`；`/permissions` 管理工具 allow/ask/deny，不负责切换 workspace trust。[Commands](https://code.claude.com/docs/en/commands)
- 非交互 `-p` 禁用首次 trust verification；`--worktree` 是例外，要求先在目录中交互接受 trust。[Security safeguards](https://code.claude.com/docs/en/security#additional-safeguards), [Worktrees](https://code.claude.com/docs/en/worktrees)

所以 Claude Code 的语义是“进入/切换 workspace 时建立信任”，不是“在会话中通过 `/trust` 改写加载状态”。

## Codex CLI

Codex CLI 同样有 project trust，但没有内置 `/trust`：

- TUI onboarding 的 trust 页面只有 “Yes, continue” 与 “No, quit”；信任目标会提升到 Git repository root。[trust_directory.rs](https://github.com/openai/codex/blob/main/codex-rs/tui/src/onboarding/trust_directory.rs)
- 接受后写入用户 `config.toml` 的 `[projects."<path>"].trust_level = "trusted"`。[config_update.rs](https://github.com/openai/codex/blob/main/codex-rs/tui/src/config_update.rs)
- 未 trusted 的 project layer 会以 disabled layer 存在；当前源码明确把 project-local config、hooks 与 exec policies 列为 gated features。[config loader](https://github.com/openai/codex/blob/main/codex-rs/config/src/loader/mod.rs)
- 当前内置 slash-command enum 没有 `Trust` 变体；有 `/permissions`，但它是权限配置面。[slash_command.rs](https://github.com/openai/codex/blob/main/codex-rs/tui/src/slash_command.rs)

Codex 的 trust 还参与默认 approval policy/sandbox 配置选择，因此它并不等同于 Phase 15 的“只 gate 项目资源、绝不影响工具权限”模型。[core config](https://github.com/openai/codex/blob/main/codex-rs/core/src/config/mod.rs)

## 对 Opi Phase 15 的建议

保留“启动前 trust gate”是合理的：三者都有这种概念，而且在加载项目提供的可执行或扩权配置前取得用户同意是明确的安全边界。

不应按当前草案实现 live `/trust`：

- `/trust <choice>` 没有 pi 0.80.6、Claude Code、Codex CLI 的先例。
- 在当前 session 中改变 effective trust 会产生未定义的 unload/reload 问题：已经加载的 extension、package adapter、context、config 和 provider 不能安全地假装尚未加载。
- “下一次 project-resource discovery 生效”在 Opi 当前只在 harness 构造时 discovery 的架构中没有可验证入口。

建议在 task graph 确认前二选一：

1. **Claude/Codex 对齐（推荐）**：Phase 15 不实现 `/trust`；只保留启动 prompt、持久化 project decision、CLI override 和 headless policy。
2. **严格 pi 兼容**：保留无参数 `/trust`，只提供 Trust / Trust parent / Do not trust，写入未来会话决策，并明确提示 restart required；不提供 session-only、`/trust <choice>` 或 live reload。

无论选择哪一种，都应从 Phase 15 的 DoD 和 acceptance scenario 中删除“当前会话生效”和“下一次 discovery 采用新决策”的表述。
