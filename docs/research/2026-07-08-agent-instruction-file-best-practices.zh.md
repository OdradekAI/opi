# Agent instruction file 最佳实践调研

日期：2026-07-08

目的：回答“有没有类似 `CLAUDE.md` 的最佳实践”，并把面向仓库级 AI
coding agent 指令文件的共识整理成可复用准则。这里的对象包括
`CLAUDE.md`、`AGENTS.md`、GitHub Copilot custom instructions、Cursor
rules、Cline rules、Gemini CLI `GEMINI.md`、Aider `CONVENTIONS.md` 等。

## 结论

有。当前最佳实践正在收敛到一个模式：用一个短小、版本控制、以项目为中心的
Markdown 文件描述 agent 必须长期记住的上下文，再把更细的规则拆到按路径、
语言、模式或工具加载的规则文件里。

如果要兼容多种 coding agent，优先把跨工具规则放在 `AGENTS.md`。Claude Code
官方文档说明 Claude 本身读取 `CLAUDE.md`，但如果仓库已有 `AGENTS.md`，可以
在 `CLAUDE.md` 里用 `@AGENTS.md` 导入，必要时再追加 Claude 专属规则。Codex
官方文档则以 `AGENTS.md` 作为项目指令文件，并支持全局、项目、子目录 override
的加载链。

类似 andrej-karpathy-skills 的 `CLAUDE.md` 可以看作“行为准则层”：它强调先想清楚、
保持简单、做外科手术式修改、定义可验证目标。这类规则适合保留，但还需要补上
项目特定内容：构建命令、测试命令、代码风格、架构边界、禁止修改的路径、发布和
提交流程。

## 已观察到的共识

### 1. 写给 agent 的不是 README 复制品

`AGENTS.md` 项目把它定义为给 coding agents 的专用、可预测位置，用来放 build
steps、tests、conventions 等会干扰人类 README 简洁度、但 agent 工作需要的上下文。
它给出的最小示例包含开发环境提示、测试指令和 PR 指令。

来源：
- https://agents.md/
- https://github.com/agentsmd/agents.md/blob/main/README.md

### 2. 内容应具体、可执行、可验证

Claude Code 文档建议把 `CLAUDE.md` 用于 build/test commands、coding standards、
architecture、naming conventions 和 common workflows，并强调具体、简洁、结构化的
指令更稳定。文档给出的反例方向是不要写“format code properly”这类抽象要求，而要写
可检查的规则，例如具体缩进宽度或具体测试命令。

VS Code / Copilot 文档也给出相同方向：指令应短、自包含；说明规则背后的原因；
用具体代码例子展示推荐和避免的模式；跳过 formatter/linter 已经能强制的显然规则。

来源：
- https://code.claude.com/docs/en/memory
- https://code.visualstudio.com/docs/agent-customization/custom-instructions

### 3. 控制长度，避免 always-on 噪声

Claude Code 文档建议每个 `CLAUDE.md` 目标控制在 200 行以内；文件过长会消耗上下文并
降低遵循度。它还提醒，`@path` import 有助于组织，但导入内容仍会在启动时进入上下文，
所以不能把 import 当作省 token 的手段。

VS Code 文档同样把“短、自包含”列为有效指令建议；Cline、Roo、Windsurf/Devin 等工具都
提供全局、项目、模式或路径级规则，目的也是减少不相关规则进入当前任务。

来源：
- https://code.claude.com/docs/en/memory
- https://code.visualstudio.com/docs/agent-customization/custom-instructions
- https://docs.cline.bot/customization/cline-rules
- https://roocodeinc.github.io/Roo-Code/features/custom-instructions/
- https://docs.devin.ai/desktop/cascade/memories

### 4. 分层：全局偏好、项目规则、本地私有规则分开

Claude Code 支持 managed、user、project、local 四类 `CLAUDE.md`，并建议项目规则提交到
版本控制，本地偏好放 `CLAUDE.local.md` 且加入 `.gitignore`。Codex 支持全局
`~/.codex/AGENTS.md`、项目 `AGENTS.md`、子目录 `AGENTS.override.md`，从根到当前目录合并。
Roo Code、Cline、Gemini CLI 也都有 global/project 或 workspace 层级。

实践含义：
- 公司或个人偏好不要塞进每个仓库的项目文件。
- 仓库级规则只写团队共享事实。
- 私人 URL、测试数据、个人快捷命令放 local 文件。

来源：
- https://code.claude.com/docs/en/memory
- https://developers.openai.com/codex/guides/agents-md
- https://roocodeinc.github.io/Roo-Code/features/custom-instructions/
- https://docs.cline.bot/customization/cline-rules
- https://geminicli.com/docs/cli/gemini-md/

### 5. 大仓库用路径或主题规则，不要把所有规则塞进根文件

Claude Code 支持 `.claude/rules/`，可以按主题和路径拆分；Gemini CLI 支持 workspace
context 与 just-in-time context；GitHub Copilot 支持 `.github/instructions/*.instructions.md`
并用 `applyTo` glob 指定适用路径；Windsurf/Devin 的 workspace rules 每条规则带 activation
mode，`AGENTS.md` 在根目录是 always-on，在子目录可按目录自动作用。

实践含义：
- 根 `AGENTS.md` / `CLAUDE.md` 只放所有任务都需要的规则。
- Rust、前端、数据库、测试、发布等局部规则拆到路径级文件。
- 只有当前任务相关时才加载的流程，优先做成 skill、rule 或 prompt fragment。

来源：
- https://code.claude.com/docs/en/memory
- https://geminicli.com/docs/cli/gemini-md/
- https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions
- https://docs.devin.ai/desktop/cascade/memories

### 6. 指令不是强制安全机制

Claude Code 明确说明 `CLAUDE.md` 是上下文，不是强制配置；如果必须阻止某类动作，要用
permissions、hooks 或管理策略。它还建议把“必须在固定生命周期执行”的动作写成 hook。
这对所有 agent instruction 文件都适用：规则文件能影响行为，但不能替代 sandbox、CI、
pre-commit hook、权限系统或代码所有权保护。

来源：
- https://code.claude.com/docs/en/memory
- https://code.claude.com/docs/en/debug-your-config

### 7. 兼容多工具时，维护单一事实源

Claude Code 文档建议已有 `AGENTS.md` 的仓库用 `CLAUDE.md` 导入它，避免重复维护。Codex
以 `AGENTS.md` 为主；Cline、Roo Code、Windsurf/Devin 都支持或识别 `AGENTS.md`；Gemini CLI
允许通过设置把 context filename 改成 `AGENTS.md`、`CONTEXT.md`、`GEMINI.md` 等列表。

实践含义：
- 用 `AGENTS.md` 做跨工具主文件。
- `CLAUDE.md` 内容可以是：

```md
@AGENTS.md

## Claude Code
- 只写 Claude 专属的加载、plan mode、hook 或 memory 规则。
```

- `.github/copilot-instructions.md`、`.cursor/rules/`、`.clinerules/` 等只放工具专属差异。

来源：
- https://code.claude.com/docs/en/memory
- https://developers.openai.com/codex/guides/agents-md
- https://docs.cline.bot/customization/cline-rules
- https://roocodeinc.github.io/Roo-Code/features/custom-instructions/
- https://docs.devin.ai/desktop/cascade/memories
- https://geminicli.com/docs/cli/gemini-md/

## 推荐结构

下面是一套适合多数工程仓库的主文件结构：

```md
# AGENTS.md

## Project
- 项目是什么，不是什么。
- 关键架构边界和模块职责。

## Commands
- 安装依赖、构建、测试、lint、格式化、文档生成。
- 说明哪些命令是 release gate，哪些只是局部验证。

## Working Principles
- 先回答问题再改代码。
- 最小改动。
- 不改无关文件。
- 遇到歧义先确认。

## Code Style
- 非 formatter 能自动表达的风格。
- 首选库、禁止库、错误处理、日志、测试 fixture 约定。

## Testing
- 新增测试的位置。
- 需要序列化或隔离的测试。
- 外部网络、真实密钥、快照测试规则。

## Git / PR
- 是否允许 commit。
- staging 规则。
- commit message / changelog / release 规则。

## Safety
- 不要触碰的文件、目录、命令。
- 秘钥、生产数据、迁移、发布、force push 等策略。

## Tool-specific Notes
- 只保留跨工具无害的说明；工具专属细节放对应文件。
```

## 应避免的内容

- 把 README、设计文档、API 文档整篇复制进 always-on 文件。
- 写“写高质量代码”“保持整洁”这类不可验证规则。
- 同一件事在 `AGENTS.md`、`CLAUDE.md`、Copilot instructions 和 Cursor rules 里重复维护。
- 把私人路径、token、账号、测试 URL 提交到共享指令文件。
- 期待规则文件强制禁止危险动作；安全边界应交给权限、hook、CI 和 sandbox。
- 把一次性需求、当前 issue 的验收标准写进长期项目规则。

## 针对 opi 仓库的建议

opi 已经有内容较完整的 `AGENTS.md` 和同步的 `CLAUDE.md`。下一步最值得做的不是再加一个
大文件，而是减少重复和漂移风险：

1. 保留 `AGENTS.md` 作为跨工具主文件。
2. 让 `CLAUDE.md` 尽量导入或镜像 `AGENTS.md`，只追加 Claude Code 专属内容。
3. 把 release、implementation、remediation 这类长流程继续放 skill 或专项文档，不塞进根
   always-on 文件。
4. 若未来接入 Copilot/Cursor/Cline/Roo/Windsurf，工具文件只写该工具加载机制需要的差异，
   不复制项目规则全文。
5. 定期检查 `AGENTS.md` 和 `CLAUDE.md` 是否漂移；本仓库已经明确要求二者锁步更新。

## 来源

- andrej-karpathy-skills `CLAUDE.md`: https://github.com/multica-ai/andrej-karpathy-skills/blob/main/CLAUDE.md
- Claude Code memory / CLAUDE.md docs: https://code.claude.com/docs/en/memory
- Claude Code configuration debugging: https://code.claude.com/docs/en/debug-your-config
- Codex `AGENTS.md` docs: https://developers.openai.com/codex/guides/agents-md
- AGENTS.md open format: https://agents.md/
- AGENTS.md repository README: https://github.com/agentsmd/agents.md/blob/main/README.md
- GitHub Copilot repository instructions: https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions
- VS Code custom instructions: https://code.visualstudio.com/docs/agent-customization/custom-instructions
- Cursor agent best practices: https://cursor.com/blog/agent-best-practices
- Cline rules: https://docs.cline.bot/customization/cline-rules
- Gemini CLI `GEMINI.md`: https://geminicli.com/docs/cli/gemini-md/
- Aider conventions: https://aider.chat/docs/usage/conventions.html
- Aider conventions repository: https://github.com/Aider-AI/conventions
- Roo Code custom instructions: https://roocodeinc.github.io/Roo-Code/features/custom-instructions/
- Windsurf/Devin rules and memories: https://docs.devin.ai/desktop/cascade/memories
