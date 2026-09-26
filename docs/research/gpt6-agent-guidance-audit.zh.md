# GPT-6 Agent 指令审计

日期：2026-09-27。基线：`5803c8758006593efd4efeba214ae8b73c903580`。
[English](gpt6-agent-guidance-audit.md)

本文是非规范性的指令维护证据，不是 Phase 审计、实现台账或模型性能实测报告。

## 范围与来源

审阅根规则、全部十个项目 `SKILL.md`、Codex sidecar、相关工作流 references 及双语
技能手册；检查 CI/文档契约的衔接。本次不修改工作流脚本、Rust/Cargo、CI YAML、
规范、运行时模型配置或正式台账。`CLAUDE.md` 与 `.claude/skills` 保持符号链接。
AGENTS 和各技能没有中文对应正文；已有双语手册及本报告同步更新。

审计当日读取的官方来源：

- [GPT-6 Astra 的 skills 与提示词指导](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)：
  精确描述、按需上下文、移除无必要的流程脚手架。
- [GPT-6 模型指南](https://developers.openai.com/api/docs/guides/latest-model)：
  完整执行、权限边界、分工和适量验证。
- [GPT-6 Sol](https://developers.openai.com/api/docs/models/gpt-6-sol)：
  确认编码/Agent 工作流定位；手册的任务分工是本仓库的操作建议，不是官方强制分配。

## 发现与处理

| 范围 | 发现 | 修改 / 保留的约束 |
|---|---|---|
| AGENTS.md | 重复工作流清单和通用步骤增加常驻上下文 | 按决策加载资料，授权内完成任务，压缩安全与验证规则 |
| 路由/手册 | “eval 不修复”与已存在的确认修复分支冲突 | 链接所属修复契约，源码修复与真实试次分别授权 |
| 塑形入口 | 可选 Matt/tracker 缺失可能阻断产品讨论 | 检查实际可用性，允许当前对话人主导塑形，保留来源登记 |
| Implement | A/B/F 总是执行，混淆 status/plan/docs 与代码执行 | 按模式路由；文档/status/plan 不跑 Cargo bootstrap；A–F 按需读取 |
| Implement 并行 | 每合入一个子单元全量验证，重复整体验证 | 按台账顺序整合，验证组装结果，仅重跑失效证据 |
| Audit | 重复 scope reader 与共享入集契约 | 记录/渲染阶段再加载对应资料，保留独立上下文、完整已提交范围、摘要和独立入集 |
| Eval | 无关分支阅读及重复保真度评分 | 按模式加载，评分由 evaluator prompt 统一拥有；保留证据、预算与隐私停止条件 |
| Remediate | 已批准批次内能否连续执行不够明确 | 批准范围内连续执行，plan/index 绑定变化仍在编辑前阻塞 |
| Document | 每次 targeted 编辑均强制读无关发布历史 | 按版本/发布声明触发；默认仍完整审计所有维护中 README |
| Realign | 根入口重复分层证据与 worker 规则 | 路由到所属 reference；保留固定身份、Layer A/H 覆盖和独立核验 |
| Research | 归属候选遗漏 Independent Companion | 对齐 spec 第 4/8 章；复用已明确范围，保留非规范证据边界 |
| Release | 缓存 crate 数量/顺序及 skip-cross 终验假设可能漂移 | 从可发布依赖图推导，按构建策略核验产物；保留公开和不可逆关卡 |
| Slim-tests | 重复共享测试维护契约 | 复用证明规则；保留行为/平台覆盖，不自动提交 |

既有共享保障/检查选择契约、校验器和输出 schema 继续拥有其权限。涉及产物完整性、
权限和复现的细节予以保留。这是贡献者指令修改，不改变已发布运行时行为，故无产品
changelog 条目。

## 精简量

以 CRLF 归一化为 LF 后的 UTF-8 字节数比较 Git 基线与工作文件；
这不是模型 tokenizer 计数，也不是延迟测量。

| 入口 | 修改前 | 修改后 | 减少 |
|---|---:|---:|---:|
| AGENTS.md | 13,193 | 8,643 | 34.5% |
| Implement 入口 | 29,290 | 13,785 | 52.9% |
| 英文手册 | 13,785 | 9,911 | 28.1% |
| 中文手册 | 12,603 | 9,006 | 28.5% |

AGENTS.md、十个技能入口与两份手册合计从 133,331 减至 98,010 字节（26.5%）。
这是文件清单汇总，不表示每次请求都会加载全部十三个文件。

移到 `opi-implement/references/execution.md` 的细节在执行时仍需加载；
减少的是无关模式的上下文，不是执行义务。Research 为修正归属语义略增文字。
文件越短并非验收标准。

## 验证与限制

- `python scripts/opi-doc-check.py`：集成后 PASS。
- `git diff --check`：PASS。
- `python .agents/skills/opi-implement/scripts/skill-contract.tests.py`：3 项通过。
- 独立只读场景推演覆盖六类请求：根文档笔误、实现状态查询、无可选工具时塑形、
  已确认 eval 修复、默认全量 README 审计、index 漂移后的 remediation。
  推演发现实现流程仍残留强制 Matt 返回路径，已与路由器同步。
  这是指令解释验证，不是实际工作流执行成功证明。
- 已安装的通用 skill-creator 校验器不支持项目既有 frontmatter
  （`disable-model-invocation`、release 的 `arguments`）；
  一次原生 Windows 调用还遇到默认编码失败。仓库校验器通过；
  没有把外部工具失败记为成功，也没有为适配它而削弱项目调用策略。

测试影响：`none`；保留并运行现有检查。没有付费 provider 试次、真实发布、
正式 Phase audit 或 Astra/Sol A/B 性能运行。本次 Markdown/YAML 指令修改无需编译 Rust。

既有发布限制：`.github/workflows/release.yml` 对每个 `v*` tag 都构建/上传二进制，
因此 source-only `--skip-cross` 仍需单独决定 CI/发布策略。技能现在会在公开推送关卡前
识别该不匹配；本审计不声称已实现这一发布模式。

工作区验证范围：`committed=none-for-this-task`、`staged=none`；
受跟踪修改未暂存，新增执行 reference 和双语审计记录未跟踪。
初始工作区干净，HEAD 未变；未创建 commit 或发布。

## 性能后续验证

基线/候选使用相同代表任务、源码状态、宿主/工具、模型及有效 reasoning 设置。
先验证完成率和权限，再比较不必要停顿、工具调用、耗时、token/费用覆盖及波动。
分别比较 Astra、Sol，缺失遥测保持 unknown。在配对运行完成前，能证实的是指令冗余
减少、路由更明确，不能声称速度、质量或费用已经改善。
