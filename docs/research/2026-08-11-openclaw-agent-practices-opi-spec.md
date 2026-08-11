# OpenClaw 与 2026 Agent 工程实践对 `opi-spec` 的启发

> 日期：2026-08-11
>
> 性质：非规范性研究证据；不修改 `docs/opi-spec.md` 的权威。
>
> 范围：OpenClaw 官方实现/文档、2026 年 Agent 与 coding-agent 一手论文和机构工程报告，以及 Bilibili 访谈的结构化转录。
>
> 截止：2026-08-11；2025 年资料只用于补足 2026 年研究直接依赖的工程背景。

## 结论先行

`opi-spec` 的主方向得到 2026 年证据的明显支持：Agent Core 应保持小而稳定，模型能力必须按“模型 + harness/runtime + 配置”归因，工具权限与操作系统隔离必须分层，Eval/Learning/Promotion 必须分权，记忆与技能必须经冻结评测、留存、安全、效率和回滚门禁后才能激活。OpenClaw 的实际工程也证明 Gateway、heartbeat、channel、multi-agent、memory、skills 和 plugins 很有产品价值，但同时证明它们带有强烈的产品策略、信任模型和运维假设，不应因此直接进入 Agent Core。

当前规范最值得补强的不是新增 Gateway 或多 Agent，而是五类证据合同：

1. `CTRL-002` 还应记录 harness/runtime/adapter 的身份、版本和配置摘要，以及“请求模型/实际模型/认证 profile/wire/fallback”链路。2026 年的 Harness-Bench 明确发现能力随 model-harness 配对显著变化，并同时保留 artifact、trace、usage 和 validator 结果，而不是只报模型名与最终通过率（[2026-05-27, arXiv:2605.27922](https://arxiv.org/abs/2605.27922)）。
2. Eval 还需要“基准本身是否有效”的准入与停用门：任务可解性、提示与测试一致性、覆盖率、污染/评测感知、基础设施失败分类和人工裁决。OpenAI 对 SWE-Bench Pro 的审计估计约 30% 任务损坏（自动流程 27.4%，人工 34.1%），说明冻结 manifest 不能替代数据集质量审计（[2026-07-08, OpenAI](https://openai.com/index/separating-signal-from-noise-coding-evaluations/)）。
3. 权限判断必须增加“影响来源到敏感 sink”的证据视角：LLM 风险判断、提示注释和分类器只能给出风险信号，不能授予权限或替代 sandbox/policy。AgentSecBench 把 prompt text 与真正关闭模型可见信道的 provenance projection、capability restriction、output validation 明确区分（[2026-05-25, arXiv:2605.26269](https://arxiv.org/abs/2605.26269)）；ARGUS 的单一预印本结果可作为后续实验候选，而不能直接成为 Core 机制（[v2, 2026-07-08, arXiv:2605.03378](https://arxiv.org/abs/2605.03378)）。
4. C1 memory/skill 候选应补齐写入侧 provenance、taint、owner、expiry、withdrawal、contradiction 和跨模型解释漂移测试，并使用真正存在可复用结构的连续任务流。CL-Bench 中 naive ICL 胜过专用 memory 系统（[2026-06-04, arXiv:2606.05661](https://arxiv.org/abs/2606.05661)）；AgentCL 也显示 naive stream 难以区分 memory 设计，held-out 场景会出现 memory-induced degradation（[2026-06-01, arXiv:2606.02461](https://arxiv.org/abs/2606.02461)）。
5. 若未来做 ambient/proactive agent，应走 Reference Product/Extension 路线，而非给 Core 增加 Gateway。任何 heartbeat/scheduled/situation-aware trigger 都应绑定 trigger provenance、Active Snapshot、User Policy、预算、静默/打断策略、会话与交付目标，并接受单独 Eval。Google Research 将 proactive coding 分成 Reactive、Scheduled、Situation-Aware，并提出 Insight Decision Quality、Context Grounding、Learning Lift 三类评测目标（[2026, Google Research / arXiv:2605.06717](https://research.google/pubs/agentic-coding-needs-proactivity-not-just-autonomy/)）。

### 两个可确认的规范缺口

第一个缺口不只是 `CTRL-002` 少记一类 provenance，而是当前 `INV-005` 尚未明确规定 **model-visible 数据流不能升级为 authority**。候选规范措辞如下，需经人类 shaping 后才有权威：

> 来自 tool、retrieval、channel、memory、package 或其他 Agent 的不可信内容，即使进入 model-visible context，也 **MUST NOT** 因此获得 instruction authority 或 action authority。有效 capability projection 与副作用前 validation **MUST** 仅从 User Policy 和可信 runtime state 导出；delimiter、prompt label、classifier 或 LLM risk judgment **MAY** 触发拒绝/升级，但 **MUST NOT** 授予 permission 或扩大 scope。

这条是 AgentSecBench/ARGUS 所揭示的执行不变量，而不只是离线审计字段：如果 authority 只在 evidence bundle 中事后记录，攻击已经发生。机械验收应覆盖恶意 tool result、retrieval record、skill text、child-agent output 和 memory item，证明它们不能改变 effective policy，且 action argument 在执行前仍受 schema、capability 和 scope 验证。

第二个缺口位于 Package Trust：当前五态分离正确，但 Trust 尚需绑定不可变对象。候选措辞为：

> Package Trust **MUST** 绑定精确 immutable artifact digest 和 declared capability footprint。artifact/version/digest 变化或 capability footprint 扩张 **MUST** 使既有 Trust/Permission 对新对象失效并要求重新授权；signature、scan、registry provenance 和 review result 只构成 evidence，**MUST NOT** 自动产生 Trust、Enablement 或 Permission。

这可防止“已审旧版本”被解释成“自动信任更新版本”，也防止包在不改变名字时扩大 filesystem/network/process/secrets/tool-shadowing 能力。

## 研究方法与证据等级

- **A（强合同证据）**：固定版本的官方源代码/规范/产品文档，可核查“系统声明或实现了什么”。它不自动证明效果。
- **B（中强经验/实验）**：官方工程报告或带公开方法、数据/代码的定量论文。若仅是预印本、自我报告或单一实验，结论只用于设计与实验候选，不视为普遍规律。
- **C（观点/待核验）**：访谈自述、路线计划或缺少可复现 artifact 的主张。只能生成假设，不能通过 Core/Promotion 门禁。

OpenClaw 以 2026-08-11 的主分支固定提交 [`2731dc24e58a94059b063df8583e5eb01a939484`](https://github.com/openclaw/openclaw/commit/2731dc24e58a94059b063df8583e5eb01a939484) 为审阅快照（A，合同证据）；发布参照为读取时 GitHub 标记 Latest 的 [`v2026.7.1-2`](https://github.com/openclaw/openclaw/releases/tag/v2026.7.1-2)，发布于 2026-08-04。下文所有 OpenClaw 源码文档链接都固定到该提交，避免 `main` 漂移。

## 视频证据：观点、核验与边界

本节引用本地的[带时间码结构化转录整理](./2026-08-11-openclaw-vincent-koc-video-notes.md)。该文件来自公开视频音轨的 Whisper 识别与人工章节复核，不是逐字稿；时间码约有数秒误差。以下“视频称”均为 C 级受访者/节目观点，和外部一手核验严格分开。

| 时间码 | 视频命题（C） | 外部一手核验 | 对 Opi 的判断 |
|---|---|---|---|
| 23:25 | Gateway 作为常驻中枢，通过 heartbeat 周期触发会话。 | OpenClaw 固定提交文档确认 heartbeat 是由 Automations scheduler 持有的定期 main-session turn，默认 30 分钟；它会因 busy、active hours、route 和 visibility 条件跳过，并明确要求不要从旧对话推断任务（[heartbeat](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/heartbeat.md)，A）。 | 产品机制得到核验；“它是个人 Agent 的决定性分界”仍是观点。若 Opi 实验，必须把 trigger provenance、预算、snapshot、目标和静默策略放在 Reference Product/Extension。 |
| 27:00 | `auto mode` 可由另一模型判断风险并决定是否请求批准。 | OpenClaw 自身把 sandbox、tool policy、elevated 分为三个独立控制，并说明 elevated 不授予工具、不绕过策略；OpenAI 也把 prompt injection 防御建模为 source-to-sink 控制而非单纯内容分类（[OpenClaw controls](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/sandbox-vs-tool-policy-vs-elevated.md)，[OpenAI, 2026-03-11](https://openai.com/index/designing-agents-to-resist-prompt-injection/)，A/B）。 | LLM 结果只能是风险信号或“升级到人工”的输入；不得改变 `INV-005` 的外部权限/模式/范围事实，更不能自授予 `Permitted`。 |
| 28:26 | Agent 表现来自模型与 harness 的组合；模型可能对特定 harness 有偏好。 | Harness-Bench、VS Code 官方工程文档都把工具、上下文、循环、状态与模型共同视为被测系统（[Harness-Bench](https://arxiv.org/abs/2605.27922)，[VS Code, 2026-05-15](https://code.visualstudio.com/blogs/2026/05/15/agent-harnesses-github-copilot-vscode)，B/A）。 | 支持 provider-neutral，但暴露 `CTRL-002` 的缺口：证据不能只记模型，应完整记 model-harness-runtime 配置。 |
| 32:09 | 持久记忆、跨模型语义与共享 Agent 的多租户隔离远未解决。 | OpenClaw 官方文档限定一个 Gateway 是一个 trusted-operator domain，session ID 只是路由而非授权；互不信任租户必须分 cell/Gateway（[multi-tenant hosting](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/multi-tenant-hosting.md)，A）。2026 memory benchmarks也显示专用 memory 并不稳定优于 ICL（[CL-Bench](https://arxiv.org/abs/2606.05661)，[AgentCL](https://arxiv.org/abs/2606.02461)，B）。 | 强烈支持 memory 继续按 C1 候选处理；Gateway/session ownership 不应被描述成租户安全边界。 |
| 34:33 | 多数个人任务一个 Agent 足够；拆分必须由独立上下文、周期和重复负载证明。 | Google 的 180 组配置研究发现 multi-agent 在可并行任务上可增益，但在顺序依赖任务上下降 39%–70%，并观察到独立式错误放大 17.2 倍（[Google Research, 2026-01-28](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)，B）；Anthropic 的研究系统报告多 Agent 在其 breadth-first research 内部评测提升 90.2%，但 token 约为 chat 的 15 倍（[2025-06-13](https://www.anthropic.com/engineering/multi-agent-research-system)，B）。 | 不存在“多 Agent 普遍更强”。维持 package/example 路线；准入应看可并行性、上下文隔离、fan-out、协调税、失败传播和成本。 |
| 41:38 | 高自治开发需要可复现环境、截图/录像、测试证据、人工接管和回滚组成“Agent 工厂”。 | OpenAI 的 harness engineering 自述为每个 worktree 提供隔离 app、日志/指标、浏览器操作并把结构约束机械化，同时明确这些结果依赖特定仓库投资、不可直接泛化（[2026-02-11](https://openai.com/index/harness-engineering/)，B）；VS Code 将 harness PR 构建成精确 SHA 版本的 eval agent，再运行评测（[2026-05-15](https://code.visualstudio.com/blogs/2026/05/15/agent-harnesses-github-copilot-vscode)，A/B）。 | 这不是新 Core 模块，而是现有 `CTRL-*`、Phase admission 和 implementation ledger 的工程实现方向：环境冻结、原生测试、artifact、人工接管、失败保留、回滚证据缺一不可。 |
| 01:03:16 | Agent 间互操作会成为下一阶段问题。 | A2A v1.0 于 2026-03-12 发布，定义独立/不透明 Agent 的发现、版本协商、任务和多 binding 语义（[v1.0 规范](https://a2a-protocol.org/v1.0.0/specification/)，[release](https://github.com/a2aproject/A2A/releases/tag/v1.0.0)，A）。 | 证明协议已出现真实第二生态，但不证明 Opi Core 应实现 A2A。先做独立 adapter/package；只有两个真实消费者和共享 conformance 后再评估 seam。 |

## OpenClaw 工程实践核查

### 1. Gateway、会话与触发器

OpenClaw 的 [architecture](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/architecture.md) 和 [Gateway runbook](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/index.md) 把单个长驻 Gateway 作为消息面、控制客户端、nodes、HTTP/WS API、插件与 UI 的中枢；连接先握手，side-effect request 可带 idempotency key，事件 gap 要求客户端刷新而非假装已回放（A）。这是优秀的 Reference Product 控制面设计，但“一个 Gateway/一个 session authority”的共享所有权不是租户隔离。

[Heartbeat](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/heartbeat.md) 的优点不是“定时问一次模型”，而是它显式区分 desired config 与 persisted schedule，带 active hours、busy deferral、route、timeout、delivery suppression、isolated session/light context 和 token 成本提示；默认 prompt 还禁止从旧聊天自行恢复任务（A）。对 Opi 的可借鉴合同是“触发器是带来源、预算、快照和交付策略的运行输入”，而不是新增 Gateway Core。

[Multi-tenant hosting](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/multi-tenant-hosting.md) 明确声明默认安全模型是每 Gateway 一个 trusted operator boundary；session ID 选择路由，不进行租户授权；不互信租户要使用完整独立 cell，并分别拥有 state、credential、workspace、channel accounts、token 和 loopback port（A）。其 operator/host 仍被所有租户信任，host compromise 是非目标。这个诚实的 threat model 值得借鉴；共享 Gateway、workspace、session 或 process 不能被 Opi 文档误称为多租户安全。

### 2. Agent loop、工具与失败语义

[Agent loop](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/agent-loop.md) 为每个 session 串行化 turn，立即返回 `runId`，用 state-directory lock、防重复 Gateway、`activeWriterRunId` 和 generation fencing 保护 transcript write；它还把运行区分为 long-running/stalled/stuck，并在 timeout/recovery 时保留可观测状态（A）。这直接支持 `INV-003/006/007`，但 writer claim、generation fence、liveness taxonomy 更适合作为后续 Phase acceptance，而非现在立即增加永久 Core API。

[Tools](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/tools/index.md) 在模型调用前应用 profile、allow/deny、provider、sandbox、channel 和 plugin 过滤；模型看不到被禁工具的 schema（A）。[Sandbox vs tool policy vs elevated](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/sandbox-vs-tool-policy-vs-elevated.md) 则明确：sandbox 决定“在哪里”，tool policy 决定“哪些工具”，elevated 只是 exec 的 sandbox escape；`deny` 优先，禁止 write/edit 不会把 exec 变成只读，tool policy 也不能推断命令语义副作用（A）。这与 `INV-005`、`INV-009..011` 同向，并说明“审批 UI”“工具 allowlist”“OS sandbox”不能合并为一个布尔值。

### 3. 记忆与持续学习

OpenClaw 的 [memory](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/memory.md) 使用人可读 Markdown 区分 profile、curated memory、daily notes、dream review；action-sensitive memory 可携带 permission/timing/expiry/owner/safe-to-act，但文档明确 memory 本身不执行 policy（A）。[Memory search](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/memory-search.md) 采用 embedding+BM25、recency/importance/MMR，并限制只有 promoted trusted root memory 自动注入；显式远程 embedding 失败时不会静默降级为另一语义路径（A）。这些是好的实验基线。

不能照搬的是默认“dreaming”后的自动 promotion：即使有 taint gate 和 review surface，owner/agent-derived 内容仍可能直接写入长期 memory。2026 年 `When Continual Learning Moves to Memory` 指出 memory 只是把 stability-plasticity 瓶颈转移到 representation/retrieval，细粒度组织甚至可能增加 forgetting（[2026-04-29, arXiv:2604.27003](https://arxiv.org/abs/2604.27003)，B）；`ContinualSkillBench` 的 500 个互联子任务显示显式技能的收益具有选择性，较弱模型会产生碎片化技能，稳健跨任务 consolidation 仍困难（[2026-08-04, arXiv:2608.03874](https://arxiv.org/abs/2608.03874)，B）。因此 Opi 的 C1 promotion、retention、withdrawal 和 no-learning ablation 比 OpenClaw 的便利默认更适合作为规范底线。

`MemoryArena` 进一步把 memory 与 agent/environment 的跨 session 状态联动起来，指出在静态长上下文基准趋近饱和的系统仍可能无法处理这种相互依赖（[2026-02-18, arXiv:2602.16313](https://arxiv.org/abs/2602.16313)，B）。Opi 不应只评 recall；要评“检索后采取什么行动、是否基于已撤回/冲突/越权记忆行动”。

### 4. 多 Agent 与互操作

OpenClaw [multi-agent](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/multi-agent.md) 默认隔离 workspace、agentDir 和 session store，bindings 显式匹配，跨 Agent memory 默认不共享，agent-to-agent messaging 默认关闭/allowlist；但 workspace 只是 cwd 而非 sandbox，插件可能共享全局 store，主 Agent 的 auth profile fallback 仍可能合并，完全 auth isolation 尚不支持（A）。[Subagents](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/tools/subagents.md) 使用独立 context、限深/限宽/并发/成本、父级交付、结果视作数据而非用户指令、级联停止和 fail-fast sandbox requirement（A）。

这些设计适合 Opi 的示例 package：默认隔离 context，父 Agent 独占面向用户的最终交付，child output 作为不受信证据，fan-out/depth/concurrency/cost 有上限，权限不能向下或横向扩大。它们不满足 `PRIN-002` 的 Core seam 准入，也不解决跨租户凭据隔离。A2A v1 可作为未来互操作 adapter 的真实外部消费者，但不能以“标准存在”为由绕过 `GATE-001/002`。

### 5. Provider、model、harness/runtime 与模型偏见

OpenClaw [agent runtimes](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/agent-runtimes.md) 明确区分 Provider、Model、Agent runtime/harness 与 Channel，并列出谁拥有 loop、canonical thread、tools/hooks、context/compaction 和 unsupported behavior；显式 runtime 不兼容时 fail closed，仅 `auto` 可做兼容 fallback（A）。[Model failover](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/model-failover.md) 先轮换 auth profile 再轮换 model，fallback 只影响本 turn、不改写用户选项；只有在 tool execution/assistant output 前才可整链重试，以避免重复副作用（A）。

这说明“模型偏见”至少有三类必须拆开：基础模型能力差异、harness 对某模型的适配偏好、provider/auth/wire fallback 实际改变了执行配置。不能把模型得分当作 provider-neutral 成功。VS Code 还会按模型调整 tool exposure 和 system prompt，并在 2026-07 的官方实验中用 offline eval + live traffic control 测试 prompt 变体（[2026-07-06](https://code.visualstudio.com/blogs/2026/07/06/optimizing-vscode-coding-harness-model-providers)，B）。Opi 应保留 provider-neutral 接口，但证据必须精确暴露实际运行组合。

### 6. 安全、生态与供应链

OpenClaw [security](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/security/index.md) 明确假定一个 Gateway 对应单一 trusted operator，优先顺序是 identity、scope、model，并把 prompt injection 视为不能靠 prompt 消除的问题；plugins 是进程内 trusted code，sandbox 仍为 opt-in，workspace 不是边界，agent browser 接近 operator-level capability（A）。这份边界声明很有价值，但其个人 Agent 默认与 Opi 的通用 toolkit 目标不同，不能照搬默认 full-host trust。

[Skills](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/tools/skills.md) 和 [plugin architecture](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/plugins/architecture.md) 提供 precedence、allowlist、版本/来源、metadata-first discovery、typed capability registry 和 immutable snapshot replacement（A）。安全扫描、registry trust envelope、pinning 只提供证据；它们不能自动把包从 Installed 提升到 Trusted/Enabled/Selected/Permitted。`Skill-Inject` 在 202 个 injection-task 配对上报告前沿 Agent attack success rate 最高 80%，模型放大和通用过滤都不足，必须做 context-aware authorization（[2026-02-23, arXiv:2602.20156](https://arxiv.org/abs/2602.20156)，B）。因此技能生态是扩展价值，也是供应链与 prompt 权限面，不能成为 C2/C3 的旁路。

OpenClaw 的 [formal verification](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/security/formal-verification.md) 声称用 TLA+/TLC 覆盖 exposure、node exec、pairing、ingress、routing、concurrency，却也明确 model 不等于完整实现；其链接的独立 formal-model 仓库在本次核验时不可达且未见 CI（A 用于核验“文档声称什么”，C 用于有效性）。这是 `PRIN-005` 的反例提醒：没有可访问 artifact、固定版本和自动重放，形式化语言本身不是证据。

### 7. 可观测性与成本

[OpenTelemetry](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/gateway/opentelemetry.md) 把稳定 diagnostics event 留在进程内，exporter 作为可选 plugin；默认不捕获内容，并记录 provider/model、harness lifecycle、queue/liveness/recovery、failover、tool blocked/loops、exec、diagnostic queue dropped 和 exporter health（A）。这直接支持 `CTRL-001/003`，并提示 Evidence coverage 必须包含“丢了多少 telemetry/为何丢”，而不是只有成功 span。

[API usage costs](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/reference/api-usage-costs.md) 与 [usage tracking](https://github.com/openclaw/openclaw/blob/2731dc24e58a94059b063df8583e5eb01a939484/docs/concepts/usage-tracking.md) 区分本地估算与 provider quota/实际组织账单，并盘点 core call、media、image/video、embedding、web、provider snapshot、compaction、model scan、speech、skills 等隐性支出（A）。Opi 的 efficiency gate 应标记 cost origin/estimate/billing，覆盖非主模型 side call；未知价格应保持 unknown，不应填零或和实际账单混合。

## 2026 一手研究与工程证据矩阵

| 日期/版本 | 一手来源与关键证据 | 对 Opi 的具体启发 | 强度 |
|---|---|---|---|
| 2026-08-04, v1 | [Self-Evolving Coding Agents, arXiv:2608.03392](https://arxiv.org/abs/2608.03392)：综述把可进化对象分为 framework、memory、skill、tool、model、collaboration，并列出反馈可靠性、benchmark overfitting、安全、维护、成本与泛化风险。 | 可用作 C1/C2/C3 对象分类索引；它是 survey，不是 promotion 效果证据，不能为自修改授权。 | C/B（地图） |
| 2026-08-04, v1 | [ContinualSkillBench, arXiv:2608.03874](https://arxiv.org/abs/2608.03874)：5 个领域、每域 100 个互联子任务；显式技能有选择性收益，弱模型易碎片化。 | 支持 `CAP-004` 的多 season、retention、source-family holdout、no-learning ablation；否定“自动产技能即持续提升”。 | B |
| 2026-06-04, v1 | [CL-Bench, arXiv:2606.05661](https://arxiv.org/abs/2606.05661)：6 个专家验证领域，naive ICL 优于专用 memory。 | memory 候选必须和 ICL/no-memory 基线配对；要隔离基础能力与 learning gain。 | B |
| 2026-06-01, v1 | [AgentCL, arXiv:2606.02461](https://arxiv.org/abs/2606.02461)：用 compositional stream 检查可复用结构；naive/held-out stream 会暴露退化。 | `CTRL-005` manifest 应记录 task-stream topology 和 reuse relation；不能只随机排列独立任务。 | B |
| 2026-04-29, v1 | [When Continual Learning Moves to Memory, arXiv:2604.27003](https://arxiv.org/abs/2604.27003)：memory 把 stability-plasticity 问题移到表示/检索，负迁移仍存在。 | `CAP-004` 需要 contradiction、negative-transfer、withdrawal 与 retrieval-policy 评测。 | B |
| 2026-02-18, v1 | [MemoryArena, arXiv:2602.16313](https://arxiv.org/abs/2602.16313)：跨 session 的 memory-agent-environment 状态互相依赖。 | 从静态 recall 迁到 action-coupled memory Eval；记录检索证据如何影响工具与 artifact。 | B |
| 2026-05-27, v1 | [Harness-Bench, arXiv:2605.27922](https://arxiv.org/abs/2605.27922)：106 个离线任务、5,194 条轨迹；同时记录 artifact、trace、usage、validator，model-harness 配对差异显著。 | P0 补强 `CTRL-002`；headline claim 必须命名 model+harness config，不能只命名模型。 | B |
| 2026-05-15 | [VS Code harness](https://code.visualstudio.com/blogs/2026/05/15/agent-harnesses-github-copilot-vscode)：官方说明 harness 拥有 context、tool exposure/execution、loop；变更按 commit SHA 发布 eval agent。 | 将 harness build/version/config 纳入 Active Snapshot；任何 prompt/tool/loop PR 都触发 Eval。 | A/B |
| 2026-06-19 | [VS Code 50,974-run smoke eval](https://code.visualstudio.com/blogs/2026/06/19/what-50000-runs-taught-us)：固定空 workspace、工具、prompt 的五行任务可暴露模型、harness 与 infra 波动。 | 保留极小而稳定的 canary/smoke season，和复杂 benchmark 并列；不能用其代表广泛能力。 | B |
| 2026-02-11 | [OpenAI Harness Engineering](https://openai.com/index/harness-engineering/)：隔离 worktree app、日志/指标、浏览器证据、结构 lints 与持续 doc gardening；作者明确结果依赖具体 repo 投资。 | “Agent 工厂”映射到可复现环境、机械边界、证据 artifact、人工 judgment；不把内部吞吐自述推广为普遍 ROI。 | B（自述） |
| 2026-07-08 | [OpenAI coding eval audit](https://openai.com/index/separating-signal-from-noise-coding-evaluations/)：约 30% SWE-Bench Pro 任务损坏，问题含过严测试、欠规范 prompt、低覆盖、误导 prompt。 | P0 增加 dataset integrity/adjudication gate 和 benchmark withdrawal；原生 grader 也可能错。 | B（强审计） |
| 2026-02-05 | [Anthropic Infrastructure Noise](https://www.anthropic.com/engineering/infrastructure-noise)：Terminal-Bench 2.0 单一 infra 配置差异可改变约 6 分，基础设施错误率随资源策略显著变化。 | `CTRL-005/006` 记录 CPU/RAM/time/network/image，并把 infra failure 与 agent failure 分开；小分差需重复试验。 | B |
| 2026-05-29 | [OpenAI trustworthy third-party evaluations](https://openai.com/index/trustworthy-third-party-evaluations-foundations/)：区分 capability、safeguard、comparison claim，并要求检查 reward hacking、refusal、contamination、broken task、sandbagging 与 harness 固定。 | Eval report schema 增加 claim type、integrity audit 与适用边界；第三方身份本身不保证独立或正确。 | B |
| 2026-04-13, v1 | [HORIZON, arXiv:2604.11978](https://arxiv.org/abs/2604.11978)：3,100+ trajectories、四领域，分析 horizon-dependent degradation；trajectory judge 与人工有较高一致性。 | 可用于 failure attribution 诊断；依 `CTRL-004`，LLM judge 仍不能变 headline/native grader。 | B |
| 2026-05-28, v1 | [LongDS-Bench, arXiv:2605.30434](https://arxiv.org/abs/2605.30434)：68 个任务、2,225 turns，后期准确率大幅下降，更多步骤不必然提升。 | `CAP-003` 长程能力要评 state update/rollback/composition，不以 step 数或运行时长替代结果。 | B |
| 2026-01-28 | [Google multi-agent scaling](https://research.google/blog/towards-a-science-of-scaling-agent-systems-when-and-why-agent-systems-work/)：并行任务可获益，顺序依赖任务显著退化，coordination topology 决定误差放大。 | 多 Agent 只做按任务结构准入的 package；要求 single-agent baseline、fan-out/cost/failure propagation 指标。 | B |
| 2025-06-13 | [Anthropic multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)：其研究任务内部评测提升但 token 成本约为 chat 的 15 倍，并承认不适合强依赖/共享上下文任务。 | 作为 2025 奠基工程证据；支持“广度优先可并行”而不是普适 multi-agent。 | B（自述） |
| 2026, to appear | [Google Agentic Coding Needs Proactivity](https://research.google/pubs/agentic-coding-needs-proactivity-not-just-autonomy/)：Reactive/Scheduled/Situation-Aware；IDQ/CGS/Learning Lift。 | ambient trigger 是独立产品/extension 路线；需 trigger provenance、silence/interrupt、budget、snapshot、authority 与单独 Eval。 | B/C（框架） |
| 2026-03-12, v1.0.0 | [A2A specification](https://a2a-protocol.org/v1.0.0/specification/) 与 [release](https://github.com/a2aproject/A2A/releases/tag/v1.0.0)：分离 canonical semantics 与 JSON-RPC/gRPC/HTTP bindings，含版本协商、Agent Card、task lifecycle。 | 可作为独立 companion/adapter 的 conformance target；不因协议成熟而进入 Core。 | A |
| 2026-02-23, v1 | [Skill-Inject, arXiv:2602.20156](https://arxiv.org/abs/2602.20156)：技能注入可导致泄露、破坏与勒索式动作，单靠 scaling/filtering 不够。 | 技能内容是 untrusted input，技能安装/选中不授予工具；C2/C3 仍需人类授权与外部 capability gate。 | B |
| 2026-05-25, v1 | [AgentSecBench, arXiv:2605.26269](https://arxiv.org/abs/2605.26269)：形式化 instruction/retrieval/capability integrity，区分 prompt annotation 与 enforcing projection。 | `INV-005` 测试应覆盖来源影响秘密/能力 sink；LLM auto-approval 只能升级风险，不能 grant。 | B（模型规模有限） |
| 2026-07-08, v2 | [ARGUS, arXiv:2605.03378](https://arxiv.org/abs/2605.03378)：影响 provenance graph、action argument grounding 和 invariant check；单一 benchmark 报告 attack success 28.8%→3.8%、clean utility 87.5%。 | 适合 Phase 实验 influence-provenance adapter；目前不足以定义 Core 通用契约。 | B（单一预印本） |
| 2026-03-11 | [OpenAI prompt-injection design](https://openai.com/index/designing-agents-to-resist-prompt-injection/)：把问题类比 social engineering，强调 source/sink、敏感数据/动作确认与限制。 | 安全评测按 source→sink 数据/能力流组织；sanitization/classifier 只是 defense-in-depth。 | B |
| 2026-05-08 | [Running Codex safely](https://openai.com/index/running-codex-safely/)：sandbox、approval、managed config、network policy 与 agent-native OTel 并用。 | 支持 `CTRL-001..003`、`INV-005`、`INV-011`；审批事件和 network block 进入证据但不取代边界。 | B（自述） |
| 2026-04-15 | [OpenAI Agents SDK evolution](https://openai.com/index/the-next-evolution-of-the-agents-sdk/)：把 harness 与 compute 分离，credential 位于模型代码环境外，并支持 snapshot/rehydrate。 | 支持 standalone execution adapter、fail-closed 选择、durable snapshot；不要求 Opi Core 托管 compute。 | B/A |

## 与 `docs/opi-spec.md` 的逐条映射

“支持”表示外部证据强化现条款；“补强”表示条款方向正确但 Phase/spec evidence contract 缺字段；“保持/不照搬”表示研究不足以改变现有放置或明确反对复制。

### Authority、目标、原则与放置

| 条款 | 结论 | 证据映射与缺口 |
|---|---|---|
| `AUTH-001` | 支持 | 快速变化的 OpenClaw 产品面和 2026 预印本不适合写成永久事实；规范只保留 durable direction。 |
| `AUTH-002` | 支持 | OpenClaw formal-verification 文档的外部 artifact 不可达，说明没有 owner+mechanical route 的 MUST 不可审计。 |
| `AUTH-003` | 支持 | Agent factory/eval 证据应先进入 Phase delivery spec 和 ledger，不能由研究文档直接覆盖规范。 |
| `AUTH-004` | 保持 | 未发现需要改变英文规范/中文等义副本的证据。 |
| `AUTH-005` | 支持 | OpenClaw 的 release/roadmap 高频变化说明日期、进度和路线历史应留在 snapshot/research。 |
| `GOAL-001` | 强支持 | OpenClaw 同时有可复用 runtime 与强意见 Gateway/channel 产品；二者价值并存但边界不同。 |
| `GOAL-002` | 强支持 | Memory、multi-agent、skills、heartbeat、A2A 都有价值，但都携带独立 policy/authority/infra；留在外部直到 Chapter 8 gate。 |
| `GOAL-003` | 支持 | OpenClaw 的 TS/npm 结构不是 Rust 设计理由；Opi 应借合同、失败语义和可验证性，不照抄目录/运行时。 |
| `GOAL-004` | 强支持 | OpenClaw 的 Gateway/memory/automation 复杂度反向证明 Minimal Runtime 必须在无 Eval/Learning/remote/extension 时独立有用。 |
| `PRIN-001` | 支持 | Gateway/heartbeat/channel 只服务特定 Reference Product，不通过 deletion test。 |
| `PRIN-002` | 强支持 | A2A 可成为未来真实 adapter，但单一 OpenClaw/单一插件仍不足；至少两个消费者+共享 conformance 不变。 |
| `PRIN-003` | 强支持 | OpenClaw 将 sandbox、tool policy、elevated 分开；机制/产品默认不能混入 Core。 |
| `PRIN-004` | 强支持 | 显式 runtime/backend/fallback、embedding、外部 adapter 都应 fail closed；OpenClaw 的明确失败优于静默语义变化。 |
| `PRIN-005` | 强支持/补强 | 2026 benchmark 审计和 infra noise 证明 immutable bundle 必须再加 dataset integrity、infra classification 和 claim type。 |
| `PLACE-001` | 支持 | Eval、Learning、Promotion 和 Gateway 都应依赖最小 runtime contract，不能反向被 Core 依赖。 |
| `PLACE-002` | 支持 | Sandbox、A2A、telemetry exporter、remote scheduler 都是 Agent-neutral；先独立 contract。 |
| `PLACE-003` | 保持 | OpenClaw Foundation/品牌治理是组织选择，不决定 crate/repo 放置。 |
| `PLACE-004` | 强支持 | OpenClaw Fleet 标记 experimental 并不把它变成安全 Core；feature flag 不能代替 gate。 |
| `PLACE-005` | 保持 | 研究不能证明现有代码已造成 material harm，不建议借机搬迁。 |

### Capability ladder 与控制面

| 条款 | 结论 | 证据映射与缺口 |
|---|---|---|
| `CAP-001` | 补强 | Provider-neutral 方向正确；evidence/profile 需加 harness/runtime、requested/actual model、wire/auth/fallback provenance。 |
| `CAP-002` | 支持/补强 | OpenClaw compaction、model switching、memory flush、liveness 可观测；继续禁止依赖 private raw reasoning。建议记录 context transform/compaction policy digest。 |
| `CAP-003` | 强支持 | OpenClaw writer fencing、session queue、partial failure；HORIZON/LongDS 证明长程要评状态正确性而非更多 steps。 |
| `CAP-004` | 强支持/补强 | CL-Bench/AgentCL/ContinualSkillBench 支持独立冻结评测与 retention；补 provenance/taint/expiry/contradiction/withdrawal 和 stream topology。 |
| `CAP-005` | 强支持 | Self-evolving survey 只说明研究对象，不提供授权；C2 staged activation、monitor、rollback、Delegated Policy 不应放松。 |
| `CAP-006` | 强支持 | 自动记忆/技能或 agent factory 吞吐不能绕过早期 runtime/eval/safety 证据。 |
| `CAP-007` | 支持 | OpenClaw memory import/remote knowledge 与“从 finalized experience 学习”不同；External Knowledge Sync 继续平行。 |
| `CTRL-001` | 强支持/补强 | OpenClaw OTel 支持稳定事件/可插 exporter；建议 coverage 包含 dropped diagnostic、exporter health、harness lifecycle。 |
| `CTRL-002` | **P0 补强** | 增加 harness/runtime/adapter id+version+config digest；requested/actual provider/model/wire/auth profile/fallback；cost origin；trigger provenance。 |
| `CTRL-003` | 强支持 | OpenClaw/OpenAI 默认 content capture off、显式开启/有界/脱敏；保持 private content 最小化。 |
| `CTRL-004` | 支持/补强 | HORIZON 的 LLM judge 仅适合 failure diagnostic；另需 native grader 自身的有效性/覆盖率审计。 |
| `CTRL-005` | **P0 补强** | 除 paired trial，还需 task validity、stream topology、infra/resource class、repeated-run policy、missing telemetry reason。 |
| `CTRL-006` | 强支持 | VS Code 精确 SHA eval agent、Harness-Bench bundle 支持 offline reproduce；同环境重跑仍必须新 trial id。 |
| `CTRL-007` | 强支持 | OpenClaw exporter/scheduler/Gateway 与 A2A 都证明基础设施应在 eval module 外。 |
| `CTRL-008` | 强支持 | OpenClaw dreaming/LLM auto-approval 是反例风险；生成候选者不能兼任 promotion/authorization。 |
| `CTRL-009` | 强支持 | OpenClaw snapshot replacement、separate tenant cell 支持 immutable reference 和单写者所有权。 |
| `CTRL-010` | 强支持 | External runtime/backend、embedding、permission 和 telemetry 不确定时应停止 forward transition，不回退成本地/旧安全语义。 |

### Runtime invariants 与准入

| 条款 | 结论 | 证据映射与缺口 |
|---|---|---|
| `INV-001` | 强支持/补强 | OpenClaw provider→model→runtime ownership matrix 与 turn-local failover 支持运行时解析；Evidence 必须保留实际选择。 |
| `INV-002` | 强支持 | A2A 也分 canonical semantics 与 binding；provider wire 继续藏在中立接口后。 |
| `INV-003` | 强支持 | OpenClaw serialized agent loop、writer claim 和 retry-before-side-effect 强化精确 turn order 测试。 |
| `INV-004` | 支持 | Heartbeat isolated session、runtime switch、compaction 都需要完整 next-request snapshot，不只是追加消息。 |
| `INV-005` | **P0 真缺口** | 增加“model-visible data cannot upgrade to authority”：LLM auto-approval 只可 signal/escalate；permission、schema、source→sink capability gate 必须在副作用前由外部机制完成。 |
| `INV-006` | 强支持/补强 | 引入 long-running/stalled/stuck、queue busy/skip、telemetry dropped 的显式状态作为 Phase 候选。 |
| `INV-007` | 强支持 | OpenClaw transcript writer fencing、state lock、recovery；Opi 的 branch/leaf/crash contract 更明确，保持。 |
| `INV-008` | **P0 补强** | Active Snapshot 应再绑定 harness/runtime、trigger、actual model/fallback 和 effective User Policy。 |
| `INV-009` | **P0 补强** | 技能扫描、allowlist、安装和 sandbox 都不是同一 gate；五态独立保持。Package Trust 还应绑定 artifact digest + declared capability footprint，版本/权限扩张重新授权。 |
| `INV-010` | 强支持 | OpenClaw explicit runtime/backend/embedding 失败不应静默换语义；selected external backend 不回 local。 |
| `INV-011` | 强支持 | OpenAI SDK 也把 harness 与 compute/credential 分离；`opi-sandbox` 独立与显式平台降级正确。 |
| `GATE-001` | 强支持 | Gateway、heartbeat、multi-agent、memory、A2A、influence graph 当前都应先留 Reference Product/Extension/Companion。 |
| `GATE-002` | 强支持 | “OpenClaw 已做”“A2A 已 v1”“论文显著提升”均不能免 gate。 |

### 战略次序

| 条款 | 结论 | 证据映射与缺口 |
|---|---|---|
| `STRAT-001` | 最高优先、补证据字段 | 先闭合 provider/runtime/next-turn/observability；尤其 model-harness provenance 和 retry-before-effects。 |
| `STRAT-002` | 最高优先、补 benchmark QA | Cross-Agent Eval 先于 learning；加入 task integrity、infra noise、claim type、harness identity。 |
| `STRAT-003` | 保持 | 用 HORIZON/LongDS 类状态与失败指标深化可测能力，不以“长时间运行/多 Agent”替代。 |
| `STRAT-004` | 谨慎推进 | 先 episodic memory，再 skill；必须与 naive ICL/no-learning 配对，做 retention/withdrawal/negative transfer。 |
| `STRAT-005` | 后置 | OpenClaw memory promotion/agent-generated skills 不足以授权 C2；保持 human approval 或窄 Delegated Policy。 |
| `STRAT-006` | 最后 | Agent factory 是证据生产与执行基础设施，不是自迭代获得 authority 的理由。 |

## 建议优先级

### P0：下一次规范 shaping 应解决

1. **扩充 Evidence/Active Snapshot provenance。** 在 `CTRL-002`、`INV-008` 的 Phase 规范中加入 harness/runtime/adapter identity、版本、配置 digest，requested/actual model/provider/wire/auth profile/fallback、context/compaction policy、effective User Policy、trigger provenance 与 cost origin。验收：同一 bundle 可离线解释“谁拥有 loop、用何工具/权限、为何实际调用该模型、是否发生 fallback”。
2. **增加 Eval integrity admission/withdrawal。** 在 `CTRL-004..007` 的实现前置规范中加入 task solvability、prompt-test alignment、test coverage、contamination/eval-awareness、reward hacking、infra failure 分类、人工 adjudication 与 benchmark retirement。验收：broken/ambiguous/infra-failed 不计 agent failure，报告仍保留覆盖率和排除理由。
3. **把 LLM approval 明确降为 risk signal。** `INV-005` 的 Phase 负测覆盖 untrusted source→secret/action sink；模型/分类器只能 `allow-to-ask`、`deny` 或标风险，不能生成 `Permitted`，不能扩大 scope，不能替代 schema/policy/sandbox。验收：恶意 skill/retrieval/tool output 即使说服守门模型，也无法越过有效 User Policy。
4. **补 C1 写入侧与动态 Eval 合同。** 任何 memory/skill candidate 记录 source episode、owner、taint、permission snapshot、expiry、contradiction、withdrawal；比较 naive ICL/no-memory，并对 compositional/held-out/source-family stream 做 retention、negative-transfer、action-coupled safety。验收：候选生产者不能选 cohort/grader/threshold；默认不自动 promotion。
5. **为 proactive trigger 预留外部路线而非 Core seam。** 先写 Reference Product/Extension Phase proposal：trigger provenance、schedule/event identity、budget、active hours、silence/interrupt/delivery policy、session/Active Snapshot、authority snapshot、busy/duplicate/idempotency、rollback 和专用 IDQ/CGS/Learning Lift Eval。没有该合同，不新增 heartbeat/Gateway。
6. **把 Package Trust 绑定到不可变 artifact 与能力足迹。** 信任记录包含 digest、来源、版本和 declared filesystem/network/process/secrets/tool capability footprint；digest 或能力扩张使旧授权不适用于新对象，必须重新授权。scan/signature 只进入 evidence。验收：同名包升级或扩大能力时保持 Installed，但不自动继承 Trusted/Permitted。

### P1：Phase/Reference Product 可采用

1. 定义 session writer claim/generation fence 与 `long-running/stalled/stuck` liveness taxonomy，做 crash/recovery/duplicate-write 负测。
2. 成本证据区分估算、provider-reported usage、quota、实际 billing；盘点 embedding、compaction、web/media、judge、monitor、subagent 等 side call，未知价格保持 unknown。
3. 做 multi-agent 示例 package：single-agent baseline、isolated context 默认、父级独占最终交付、child result 视作不受信证据、深度/宽度/并发/预算上限、级联取消、权限不扩大。
4. 做 Gateway/control-plane companion prototype 时借鉴 typed handshake、schema validation、idempotency、accepted/final completion、gap/resync、health/readiness；不得改变 Agent Core ownership。
5. Skill/plugin package 增加来源、精确版本、digest、签名/scan evidence、allowlist 与 effective lifecycle diagnostics；扫描结果不自动授予 Trusted/Permitted。

### P2：有证据后再考虑

1. 以 Markdown 分层 memory、hybrid retrieval、MMR/recency、review surface 作为 C1 实验变量，不固化为 Core API。
2. 对 permission state machine、session writer fencing、promotion state machine 做少量 TLA+/model-checking；前提是 artifact 固定、公开可访问、CI 重放，且结果与实现 conformance 相连。
3. 做 A2A v1 adapter 与第二个独立 agent protocol adapter；只有出现两个真实消费者和共享 conformance 后才重新做 `PRIN-002` Placement Review。
4. 实验 influence-provenance graph；先证明在 Opi 工具/记忆/skill 注入场景下的 clean utility、attack reduction、成本和 false-block，再讨论稳定 seam。

## 明确不建议

- 不把 OpenClaw Gateway、heartbeat、channel、DM routing、browser、multi-agent、memory 或 A2A 直接加入 Agent Core。
- 不把共享 Gateway、session key、workspace、agent ID 或 SQLite session ownership 称为租户安全边界；不互信租户至少需要独立进程/凭据/状态/OS 或 VM 边界。
- 不复制 OpenClaw 面向单一 trusted operator 的默认 full-host 风险姿态作为通用 Opi 默认；workspace 不是 sandbox，approval 也不是 sandbox。
- 不允许 LLM auto-approval、prompt sanitizer、input classifier、risk score 或 LLM judge授予权限；它们最多拒绝、标风险或升级到人工。
- 不根据 owner/agent 自述、局部阈值或一次 LLM consolidation 自动 promotion 长期 memory/skill；不把 retrieval 命中率当持续学习成效。
- 不把 registry scan、签名、来源可信、安装成功或插件 allowlist 合并成 `Trusted`/`Permitted`；维持五态独立。
- 不把多 Agent 当默认提效器，不按组织图复制 manager/planner/subagent 树；没有单 Agent baseline、可并行性和协调税证据就不拆分。
- 不报告“模型 X 的 Agent 能力”而省略 harness/runtime/config；不把 estimated cost 与 provider billing 合并，不把 missing telemetry 当零。
- 不因为 OpenAI 的内部监控案例使用 raw chain-of-thought 而改变 Opi 的私有原始推理非目标。安全证据应优先来自动作、工具、权限、来源、sink、artifact 和可公开解释的结果；内部私有 CoT 监控不能成为 portability 或 Eval 前提（对比 [OpenAI, 2026-03-19](https://openai.com/index/how-we-monitor-internal-coding-agents-misalignment/)）。
- 不把 agentic factory 的吞吐、自修复或 agent-to-agent review 当成 `CTRL-008` 的独立授权；candidate producer、grader、promotion controller 与 Human Authority 继续分离。

## 来源清单与核验说明

本报告实际引用 **50 项一手 artifact**：

- 22 项 OpenClaw 固定版本材料：1 个主分支提交、1 个 release，以及 architecture、Gateway、heartbeat、multi-tenant、agent loop、tools、sandbox/policy/elevated、memory、memory search、multi-agent、subagents、security、formal verification、model failover、agent runtimes、plugin architecture、skills、OpenTelemetry、API cost、usage tracking 等 20 份官方文档。
- 1 项视频结构化转录（本地音轨识别+复核；只作 C 级观点证据）。
- 27 项外部一手材料：2026 论文/预印本、OpenAI/Anthropic/Google/VS Code 官方工程或研究报告、A2A v1 规范与 release。A2A 规范与 release 合并计为一个版本化来源；同一研究的项目页/论文不重复计数。

核验方式：`opi-spec.md` 已全文读取并逐个覆盖全部 `AUTH/GOAL/PRIN/PLACE/CAP/CTRL/INV/GATE/STRAT` 标识；OpenClaw 文档以固定 commit 阅读，GitHub release 另行核验；论文日期和版本以 arXiv 记录为准；机构实践只使用发布机构自己的文章/规范/仓库。A 级产品文档只证明合同/实现声明，B 级定量资料保留预印本、单 benchmark、自我报告和可泛化性限制，视频数字及未来计划不作为独立事实。
