# pi 对齐矩阵

## 范围

本文按语义行为、Rust crate 归属、产品工作流和持久证据锚点对比 `opi` 与 `.repo/pi-0.80.2`。它不是 TypeScript API、package ABI、配置文件或会话文件兼容性清单。

本文也是 `pi` 0.80.2 的持久证据和对齐基线。当前结论是：`opi` 的核心语义对等度较高，产品对等度中等，生态对等度有意保持较低，直到现有能力做深、做稳。

## 文档控制

| 字段 | 值 |
|---|---|
| 上游路径 | `.repo/pi-0.80.2` |
| 上游包版本 | `@earendil-works/pi-ai`、`@earendil-works/pi-agent-core`、`@earendil-works/pi-tui` 和 `@earendil-works/pi-coding-agent` 均为 `0.80.2` |
| Opi workspace 版本 | `0.6.4` |
| 采样日期 | 2026-06-24 |
| 证据范围 | `.repo/pi-0.80.2` 下的本地文件、当前 `docs/opi-spec.md`、当前 `docs/pi-alignment-matrix.md` 和当前 `crates/*` 布局 |
| 更新策略 | 当研究的 `pi` 基线变化，或 `opi` 关闭本文列出的缺口时更新本文。保留有价值的旧证据作为历史上下文，不要静默重写。 |

## 执行摘要

`opi` 仍然与 `pi` 方向一致：它保留了终端优先的 coding-agent 形态、provider streaming、tool calling、session persistence、compaction、JSON/RPC 表面、package/process-adapter 思路和 extension hooks。当前偏移主要不是功能名称，而是架构归属：`pi` 0.80.2 已经把重要职责移动到 `pi-ai` 的 `Models/Auth` 和 `pi-agent-core` 的 `AgentHarness`/session repo 原语中，而 `opi` 仍有大量可比编排逻辑位于 `opi-coding-agent::CodingHarness`，provider 构造策略也更靠近 CLI/config 层。

正确调整不是把 TypeScript 包结构照搬到 Rust。正确方向是在扩张生态宽度前深化 Rust 原生缝合点：

- `opi-ai` 应增加受 `pi-ai` `Models` 启发的 provider collection/auth 缝合点，但近期核心不纳入 OAuth 和图像生成。
- `opi-agent` 应拥有 embedders 所需的通用 harness/session facade 语义，`opi-coding-agent` 保持为 CLI、工具、配置、packages 和交互命令的产品包装层。
- `opi-tui` 应继续使用 `ratatui`/`crossterm`；自定义 extension UI 和 message renderers 属于未来生态候选，不属于第 14 阶段范围。
- 产品对等度中等，核心语义对等度较高但不完整；生态对等度在现有产品做深、做稳前有意保持较低。

## Pi 架构

### `@earendil-works/pi-ai`

`pi-ai` 0.80.2 已不只是 wire adapters 集合。它把 provider 视为运行时单元，由 provider 拥有 model catalog、auth 和 stream behavior，而 `Models` collection 把请求路由到拥有该 model 的 provider（`.repo/pi-0.80.2/packages/ai/README.md:227-231`）。Provider factories 按 provider 拆分以支持选择性导入，同时通过显式的重型 `providers/all` 入口提供全部内置 provider（`README.md:233-261`）。

认证由 provider 拥有。`Models` collection 在请求路径中通过 owning provider 解析 auth，并暴露 `getAuth()` 供状态 UI 使用（`README.md:321-348`）。持久凭据位于小型 `CredentialStore` 契约之后，写入串行化；OAuth refresh 在该锁内运行，并且已有持久凭据时 provider 不会在 refresh 失败后静默回退到 env（`README.md:350-362`）。Anthropic、OpenAI Codex 和 GitHub Copilot 都已有 OAuth providers（`README.md:1361-1369`）。

图像生成通过独立的 `ImagesModels`/`ImagesProvider` 表面镜像聊天侧架构（`README.md:634-663`）。这说明图像生成与 `pi` 方向一致，但它依赖同一套 collection/auth 思路，不应早于聊天侧 provider 缝合点稳定进入 `opi`。

### `@earendil-works/pi-agent-core`

`pi-agent-core` 导出低层 agent、loop 函数、`AgentHarness`、harness messages、prompt templates、session repos、skills、system prompt helpers、harness types 和 utilities（`.repo/pi-0.80.2/packages/agent/src/index.ts:1-40`）。

`AgentHarness` 是低层 loop 之上的编排层。它拥有 session persistence、runtime config、resource resolution、operation locking 和面向 extension 的 mutation semantics（`.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:1-5`）。Harness 区分 config、session、pending writes 和 turn snapshots；turn snapshot 是一次 LLM turn 使用的具体状态（`agent-harness.md:34-60`）。Save points 在 agent-emitted messages 之后 flush pending writes，为未来 turn 创建新 snapshot，并避免修改已在途的 provider request（`agent-harness.md:140-150`）。

持久化方向是半持久化，而不是把完整运行时状态序列化：session log 是持久 state tree，host 在 resume 时重建 runtime dependencies（`.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,42-44`）。

### `@earendil-works/pi-tui`

`pi-tui` 是带自有 renderer 和 component model 的 TypeScript 终端 UI 库。`opi-tui` 有意不复制该技术栈，而是使用 Rust 原生 `ratatui`/`crossterm` widgets。因此对齐目标是产品行为和终端人体工学，不是 renderer API 兼容。

### `@earendil-works/pi-coding-agent`

`pi-coding-agent` 是产品层：CLI/TUI modes、tools、sessions、extensions、package workflows、export/share/update surfaces，以及丰富的 extension integration。其 extension 文档展示了比当前 `opi` 更宽的表面：user prompts、custom UI components、custom commands、event interception、session lifecycle hooks、provider hooks、message renderers 和 example extensions（`.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14,297-299,438-440,2177-2185,2397-2403,2524-2526,2628-2632`）。

对 `opi` 来说，这些是未来生态证据，不是当前范围。现有 Rust process-adapter 路径是好的基底，但不意味着已经对等 TypeScript custom UI、npm/gallery 行为、provider payload hooks 或 session publishing。

## 版本演进信号

以下是相较较早 `.repo/pi-0.75.3` 基线，`pi` 0.80.2 对 `opi` 规划产生实质影响的信号。

| 信号 | 证据 | 对 `opi` 的影响 |
|---|---|---|
| `pi-ai` 中的 `Models` runtime | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:81` | 在 provider correctness 前插入第 10 阶段，让 `opi-ai` 先具备可测试的 provider collection/model/auth 缝合点。 |
| Provider-owned auth substrate | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:82`; `packages/ai/README.md:321-362` | 把 provider auth 语义从 CLI env/config parsing 中拆出；OAuth 等缝合点稳定后再做。 |
| Provider factories 和 built-in catalog | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:83`; `packages/ai/README.md:233-261` | Rust profiles 应保持显式且由 registry 支撑；不要把所有 compatible provider 都硬编码进 core。 |
| OAuth providers | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:84`; `packages/ai/README.md:1361-1369` | 与 `pi` 方向一致，但需要 credential store、login UX、redaction、doctor 和 revocation 语义，因此属于未来生态范围。 |
| Image generation collection | `.repo/pi-0.80.2/packages/ai/CHANGELOG.md:86`; `packages/ai/README.md:634-663` | 聊天侧 provider collection/auth correctness 稳定后再作为未来生态候选。 |
| `AgentHarness` 导出和文档 | `.repo/pi-0.80.2/packages/agent/src/index.ts:5,28-40`; `packages/agent/docs/agent-harness.md:3` | `opi-agent` 应在拥有 Rust 原生通用 harness 缝合点前标为 Partial。 |
| Turn snapshot/save-point 语义 | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:58-60,140-150` | 第 10 阶段应先定义 snapshot/save-point 契约，再进入第 13 阶段 session 工作。 |
| Pending session writes 和计划中的 facade | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-90,176-196` | 第 13 阶段应建立在 session facade 上，而不是临时 CLI-only writes。 |
| Semi-durable harness | `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,38-44,118-121` | 长期上下文应属于 session entries；除非 sidecar 有明确持久引用模型，否则不引入隐藏全局记忆。 |
| Extension UI 和 lifecycle 宽度 | `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14,297-299,438-440,2177-2185,2397-2403,2524-2526` | 不声明 extension UI 对等；内置 TUI 稳定后再作为未来生态工作。 |
| Provider hook 宽度 | `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:443`; `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:1646-1681` | Provider request/response adapter hooks 应等 provider seam、trace 和 redaction 契约稳定后再做。 |

## 证据索引

| 来源 | 证据摘要 | 影响的 `opi` 区域 | 路线图含义 |
|---|---|---|---|
| `docs/superpowers/specs/2026-06-24-phase9-pi-0-80-2-baseline-realignment-design.md:59-61` | 第 9 阶段设计记录了重校准前文档曾使用 `.repo/pi-0.75.3` 作为研究基线。 | 文档基线 | 第 9 阶段把当前比较重校准到 `.repo/pi-0.80.2`。 |
| `docs/opi-spec.md:342-345` | 既有 `opi` 规则已经说明 generic harness primitives 属于 `opi-agent`，coding behavior 属于 `opi-coding-agent`。 | Crate 边界 | 第 10 阶段沿用既有 Rust 归属指导，而不是发明新拆分。 |
| `docs/opi-spec.md:248,1462` | 规范和 ADR-003 拒绝共享 `opi-types` crate。 | Crate 边界 | 跨 crate 类型应保留在语义拥有者中。 |
| `.repo/pi-0.80.2/packages/ai/README.md:229-231` | Provider 拥有 catalog/auth/stream behavior；`Models` 路由请求。 | `opi-ai` | Provider collection/auth 缝合点属于 `opi-ai`。 |
| `.repo/pi-0.80.2/packages/ai/README.md:323-348` | Auth 通过 owning provider 解析，并可在无请求时检查。 | `opi-ai`、diagnostics、TUI status | 第 10 阶段应暴露 missing/available auth 状态，同时避免泄露 secret。 |
| `.repo/pi-0.80.2/packages/ai/README.md:350-362` | Credential store 写入串行化，并在持久凭据 refresh 失败后避免静默 env fallback。 | 未来 auth 生态 | OAuth 需要明确的 credential-store 和 revocation 设计。 |
| `.repo/pi-0.80.2/packages/ai/README.md:634-663` | 图像生成使用独立 collection，镜像聊天侧设计。 | 未来 `opi-ai` 生态 | 聊天 provider collection/auth 未稳定前不要加入图像生成。 |
| `.repo/pi-0.80.2/packages/agent/src/index.ts:1-40` | Agent core 导出 harness、messages、prompt templates、session repos、skills、system prompt 和 utilities。 | `opi-agent` | 当前 `opi-agent` 对齐为 Partial，直到具备 generic harness/session repo 宽度。 |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:3` | Harness 拥有 persistence、runtime config、resources、locking 和 mutation semantics。 | `opi-agent`、`opi-coding-agent` | `CodingHarness` 应成为 generic runtime seams 上的产品包装层。 |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:58-60,140-150` | Turn snapshot 和 save points 防止未来状态更新影响已在途的 provider request。 | `opi-agent` | 实现第 10 阶段 harness seam 时应增加显式合约测试。 |
| `.repo/pi-0.80.2/packages/agent/docs/agent-harness.md:84-90,176-196` | Pending writes 被排队，且计划进入 session facade。 | `opi-agent` sessions | 第 13 阶段依赖第 10 阶段 session facade 定义。 |
| `.repo/pi-0.80.2/packages/agent/docs/durable-harness.md:19-28,38-44,118-121` | Session log 是 durable state tree；host 重建 runtime dependencies。 | `opi-agent` sessions | 避免为长期 workflow state 引入隐藏全局记忆。 |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:10-14` | Extensions 包含 custom tools、event interception、user interaction、custom UI 和 commands。 | `opi-coding-agent`、process adapters、TUI | 现有 adapter substrate 是 Partial；custom UI 属于未来生态范围。 |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:297-299,438-440` | Session compaction/tree hooks 可自定义或观测流程。 | Sessions、extensions | 保留未来 session lifecycle hooks 路径，但不复制 TS API。 |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:2177-2185,2397-2403,2524-2526` | UI prompts 和 custom components 接收 TUI/theme/keybinding 集成。 | `opi-tui`、extension UI | 第 14 阶段应打磨内置 TUI，不承诺 custom extension UI 对等。 |
| `.repo/pi-0.80.2/packages/coding-agent/docs/extensions.md:1646-1681` | Custom providers 可包含 OAuth login 支持。 | Provider 生态 | 未来 provider extension 路径依赖 auth seam 和产品 login 设计。 |

## 对齐等级

| 等级 | 含义 |
|---|---|
| 完整 | 已实现等价的用户可见或库可见行为，并有测试覆盖。 |
| 部分 | 已实现为基底或更窄的 Rust 原生等价物。 |
| 有意偏离 | Opi 有意使用不同的 Rust 原生接口、存储格式、渲染器或 package 模型。 |
| 缺失 | `pi` 有该能力，但 `opi` 尚无等价物。 |
| 范围外 | 不应进入核心，除非后续设计改变范围。 |

## 对齐仪表盘

| 层级 | 当前等级 | 摘要 | 下一步调整 |
|---|---|---|---|
| 核心语义对等 | 高但不完整 | Agent loop 语义、provider streaming、tool scheduling、compaction、session tree 基础、JSON/RPC、extension hooks 和 package adapters 已存在。主要缺口是 `Models/Auth`、通用 `AgentHarness`、session facade 和显式 save-point 语义。 | 在继续加固前插入第 9 和第 10 阶段。 |
| 产品对等 | 中等 | `opi` 二进制在 TUI、非交互、JSON、RPC、sessions、packages、providers、diagnostics 和 image input 上可用。`pi` 在 provider auth UX、extension UI、package lifecycle、export/share 和 session workflows 打磨上仍更宽。 | 把旧第 9-12 阶段重排为第 11-14 阶段，并放在核心缝合点之后。 |
| 生态对等 | 有意较低 | OAuth/subscription auth、广泛 provider catalog、图像生成、npm/gallery/update/enable/disable、自定义 extension UI/message renderers、web/share、provider payload hooks 和 pi session import 都不是当前产品声明。 | 作为带进入条件的未来候选跟踪。 |

## 包级对齐

| pi 包 | opi crate | 等级 | 已实现 | 缺口 / 调整 |
|---|---|---|---|---|
| `@earendil-works/pi-ai` | `opi-ai` | 部分 | Provider trait、provider adapters、provider registry、model metadata、image input、usage/cost、retry/backoff、proxy config、OpenAI-compatible profiles，以及 custom provider/model registration。 | 第 10 阶段增加 Rust 原生 provider collection/auth 缝合点。OAuth、图像生成和广泛 catalog 作为未来候选。 |
| `@earendil-works/pi-agent-core` | `opi-agent` | 部分 | Agent loop、有状态 `Agent`、hooks、tool batching、queues、sessions、compaction、SDK types、extension trait、diagnostics、streaming proxy primitives 和 runtime contract tests。 | Generic `AgentHarness`/session facade 缝合点已存在，但产品 turn/session 采用仍是部分完成。 |
| `@earendil-works/pi-tui` | `opi-tui` | 部分 | Rust 原生 `ratatui`/`crossterm` widgets、transcript rendering、markdown/code、diff、pickers、branch/session picker snapshots、themes、keybindings、terminal images 和 CJK display-width 覆盖。 | Renderer API 兼容是有意偏离。自定义 extension UI/message renderers 仍是未来生态工作；第 14 阶段应打磨内置产品 UI。 |
| `@earendil-works/pi-coding-agent` | `opi-coding-agent` | 部分 | CLI modes、built-in tools、config、sessions、context files、images、JSON/RPC、resources、packages、skills、prompt fragments、themes、custom provider registration、extension commands、branch/tree/fork/clone flows、package CLI、process-jsonl adapter hosting、diagnostics 和 doctor checks。 | `pi` 在 custom extension UI、provider hooks/login、npm/gallery lifecycle、export/share 和 update surfaces 上仍更宽；这些保持为未来生态设计。 |

## 细分功能对齐

### `pi-ai` / `opi-ai`

| 功能 | Opi 等级 | 证据 / 当前状态 | 调整 |
|---|---|---|---|
| Provider stream lifecycle | 完整 | Provider-neutral stream events 和 adapters 已实现并测试。 | 保持 start/delta/end/done/error fixture 覆盖。 |
| Provider registry 和 model metadata | 部分 | Registry 与 custom provider/model registration 已存在。 | 演进为 provider collection seam，避免 construction policy 分散。 |
| `Models` collection | 部分 | `opi` 有 registry/profile construction，但没有直接等价于 `createModels()` 的 runtime owner。 | 第 10 阶段定义 model lookup、refresh、auth 和 dispatch 归属。 |
| Provider-owned auth | 部分 | 每个 provider 已有静态 env/config credentials。 | 将 auth 语义移入 `opi-ai`；CLI/env/package config 作为构造输入。 |
| OAuth/subscription auth | 缺失 | 未实现。 | credential store、redaction、doctor、login UX、refresh 和 revocation 设计完成后作为未来候选。 |
| Image input | 部分 | Image attachments 和 provider serialization 已实现。 | 第 12 阶段继续覆盖 provider-specific image input correctness。 |
| 图像生成 | 缺失 | 未实现。 | 聊天侧 provider collection/auth 稳定后作为未来候选。 |
| 广泛 provider catalog | 部分 | 已有多个一等 provider 和 OpenAI-compatible providers，但不达到 `pi` 宽度。 | 优先使用 profiles；只有 wire/auth 语义不同才新增一等 adapter。 |

### `pi-agent-core` / `opi-agent`

| 功能 | Opi 等级 | 证据 / 当前状态 | 调整 |
|---|---|---|---|
| 低层 agent loop | 完整 | Event order、tool scheduling、hooks、queues 和 cancellation 语义已实现并测试。 | 以第 8 阶段契约作为回归门。 |
| 有状态 `Agent` wrapper | 完整 | Prompt/continue/abort/subscribe 和 queue behavior 已存在。 | 除非后续稳定化，否则保持 0.x API。 |
| 通用 `AgentHarness` | 部分 | `CodingHarness` 目前拥有大量可比的编排行为。 | 第 10 阶段在 `opi-agent` 定义 generic harness phases、snapshots、save points、busy guards 和 runtime mutation semantics。 |
| Session storage | 部分 | Append-only JSONL、resume/list/delete/fork、branch `parent_id`、`leaf`、compaction、extension state，以及第 13 阶段在 v1 头部上的增补类型条目（`session_info`、`model_change`、`thinking_level_change`、`label`、`branch_summary`）已存在；`custom_message` 推迟。 | 第 10 阶段定义 session facade；第 13 阶段已实现更丰富的 context entries。`branch_summary` 生成 UX 与交互式 `/export` 推迟到第 14 阶段。 |
| Pending session write ordering | 部分 | Generic harness/session facade 缝合点已暴露有序 pending-write 行为，但产品 session 采用仍是部分完成。 | 第 13 阶段建立在该缝合点之上增加更丰富 entries，并保持 ordering contract tests。 |
| Compaction | 完整 | Threshold/manual/overflow primitives 和 session events 已存在。 | 保持 branch-aware compaction tests。 |
| Extension trait/hooks/state | 部分 | Rust in-process extension API 和 process adapter bridge 已存在。 | 保持狭窄；未来 provider/UI/session lifecycle hooks 需要单独设计。 |

### `pi-tui` / `opi-tui`

| 功能 | Opi 等级 | 证据 / 当前状态 | 调整 |
|---|---|---|---|
| Terminal renderer | 有意偏离 | `opi-tui` 使用 `ratatui`/`crossterm`，不复制 `pi` TypeScript renderer。 | 除非单独批准可复用 TUI 产品，否则保持 Rust 原生 renderer。 |
| Transcript 和 markdown/code rendering | 部分 | 内置 transcript、markdown、code 和 snapshots 已存在。 | 第 14 阶段打磨高密度终端工作流。 |
| Diff 和 image rendering | 部分 | Diff rendering 和 terminal image primitives 已存在。 | 保持产品聚焦，只在 CLI 工作流需要时扩展。 |
| Pickers 和 keybindings | 部分 | Model/session/branch pickers、themes、keybindings 和 CJK-width snapshots 已存在。 | 第 14 阶段改善发现性、状态和可访问性。 |
| Custom extension UI/message renderers | 缺失 | 不支持。 | 内置 TUI 与 UI/RPC protocol 设计稳定后作为未来候选。 |

### `pi-coding-agent` / `opi-coding-agent`

| 功能 | Opi 等级 | 证据 / 当前状态 | 调整 |
|---|---|---|---|
| CLI modes | 完整 | Interactive、non-interactive、JSON、RPC、model listing、completions、sessions、doctor 和 package commands 已存在。 | 保持 command contracts 文档化和测试覆盖。 |
| Built-in tools | 部分 | `read`、`write`、`edit`、`bash`、`grep`、`find`、`ls` 和 `glob` 已存在，并有 mode-aware policy。 | 第 11 阶段加固 paths、encodings、truncation、cancellation 和 diagnostics。 |
| Config/resource discovery | 部分 | TOML layers、provider profiles、context files、resources、skills、prompt fragments、themes、packages 和 extensions 已存在。 | 保持 precedence 和 diagnostics 显式。 |
| Sessions 和 branch workflows | 部分 | Resume/list/delete/fork、`/tree`、`/branch`、`/fork`、`/clone`、active branch continuation 和 compaction 已存在。 | 第 13 阶段增加稳定 metadata、summaries、labels 和 export。 |
| Package/process adapter substrate | 部分 | Local/git package sources、manifest V2、`process-jsonl`、adapter tools/commands/hooks/events/state/cancellation 和 examples 已存在。通过 `opi-extension-jsonl-v1` 运行的 process-JSONL adapter 会把 package command、tool、hook、event、state 和 cancellation 桥接进 runtime。 | 稳定后再考虑 npm/gallery/update/enable/disable。 |
| Provider hooks/login UX | 缺失 | Custom provider registration 已存在；provider request/response hook parity 和 login flows 不存在。 | 第 10/12 阶段 provider seam 与 redaction/trace 设计后作为未来候选。 |
| Export/share/web surfaces | 缺失 | 本地 session/export 方向计划中；web/share 未实现。 | 第 13 阶段 sensitivity 和 redaction 规则之后作为未来候选。 |

## Phase 对齐

| Phase | 范围 | Crate | 当前等级 | 备注 / 调整 |
|---:|---|---|---|---|
| 1 | MVP foundation：Anthropic、core loop、basic tools、TUI、config | 全部 crate | 完整到部分 | 核心已交付；read-only tool 宽度后来通过 `find`/`ls` 和额外 `glob` 补齐。 |
| 2 | Multi-provider、sessions、compaction、JSON mode、retry/cost/thinking | `opi-ai`、`opi-agent`、`opi-coding-agent`、`opi-tui` | 部分 | 核心已存在；provider collection/auth 和更丰富 sessions 仍是缺口。 |
| 3 | Production hardening：enterprise providers、image input、context files、tool policy、completions、proxy | 全部 crate | 部分 | Image input 已存在；图像生成仍是未来。 |
| 4 | Extensibility substrate：RPC、SDK、extensions、resources、skills、themes、packages、custom providers、branch UI、streaming proxy | 全部 crate | 部分 | 基底较强，但不是 TypeScript extension 或 custom UI 对等。 |
| 5 | Package store / process-adapter MVP | `opi-coding-agent`、`opi-agent` | 部分 | Local/git/process-jsonl 路径已存在，包含 process-JSONL adapter hosting（`opi-extension-jsonl-v1`）；npm/gallery/update/enable/disable 仍是未来。 |
| 6 | Alignment and reliability hardening | workspace | 部分 | 文档和 runtime integration 加固，但不扩张生态范围。 |
| 7 | Reliability and observability | `opi-agent`、`opi-coding-agent`、`opi-ai` | 部分 | Diagnostics、redaction、trace envelopes、doctor checks 均为本地且显式。 |
| 8 | Runtime stabilization | `opi-agent`、`opi-coding-agent` | 部分 | Event order、hooks、tool scheduling、cancellation、SDK/RPC contracts 和 API surface classification 已测试。 |
| 9 | pi 0.80.2 baseline realignment | docs | 计划中 | 文档/证据门；不改变 runtime。 |
| 10 | Core architecture deepening | `opi-ai`、`opi-agent`、`opi-coding-agent` | 部分 | `Models/Auth`、generic `AgentHarness`、session facade、runtime hook boundaries。 |
| 11 | Tooling quality | `opi-coding-agent`、`opi-agent`、`opi-tui` | 计划中 | 由旧第 9 阶段重排；依赖第 10 阶段边界。 |
| 12 | Provider correctness | `opi-ai`、`opi-coding-agent` | 部分 | 由旧第 10 阶段重排；通过 provider collection/auth seam 已交付 fixture lifecycle/error/taxonomy/profile-flags/cache/response-id/retry/cancel/auth-diagnostic 覆盖。`usage_in_stream` 现在会请求 `stream_options.include_usage`，OpenAI Chat 会从任何携带 `id` 的 chunk 捕获 response ID，缺失 usage 会保持为显式未知，因此会话费用汇总可以被省略，而 `previous_response_id` 与请求侧 cache-control 断点仍被明确推迟。 |
| 13 | Session tree and context reconstruction | `opi-agent`、`opi-coding-agent`、`opi-tui` | 计划中 | 由旧第 11 阶段重排；依赖 generic harness/session facade。 |
| 14 | TUI product polish | `opi-tui`、`opi-coding-agent` | 计划中 | 由旧第 12 阶段重排；只打磨内置 TUI，不做 custom extension UI 对等。 |

## 路线图含义

| Phase | 名称 | 原因 |
|---:|---|---|
| 9 | pi 0.80.2 Baseline Realignment | 之前的文档基于更早的上游快照，并且高估了 `opi-agent` 对等程度。 |
| 10 | Core Architecture Deepening | `pi` 0.80.2 中 `Models/Auth` 和 `AgentHarness` 已足够核心，工具、provider、session 和 TUI 工作应建立在这些缝合点上。 |
| 11 | Tooling Quality | 旧第 9 阶段仍有价值，但应依赖第 10 阶段的 harness/tool scheduling 和 diagnostics 契约。 |
| 12 | Provider Correctness | 旧第 10 阶段仍有价值，但应通过 provider collection/auth seam 测试，而不是只测试孤立构造器。 |
| 13 | Session Tree and Context Reconstruction | 旧第 11 阶段应依赖 generic harness/session facade。 |
| 14 | TUI Product Polish | 旧第 12 阶段应聚焦内置终端产品打磨，并明确排除 custom extension UI 对等。 |
| Future | Ecosystem Candidates | OAuth、广泛 provider catalog、图像生成、custom extension UI、npm/gallery/update、web/share、provider hooks 和 pi session import 只有在前置条件明确后才进入。 |

## 当前修复优先级

| 优先级 | 领域 | 状态 | 下一步 |
|---|---|---|---|
| P0 | 基线真实性 | 当前文档描述 `0.6.4` workspace，并使用 `.repo/pi-0.80.2` 作为研究的上游基线。 | 保持本矩阵和 `opi-spec` 同步；保留内嵌证据锚点。 |
| P1 | `Models/Auth` | Registry/profile/provider construction 已存在，但 `opi-ai` 尚未拥有完整 collection/auth runtime。 | 第 10 阶段先设计/实现，再进入第 12 阶段 provider correctness。 |
| P1 | Generic harness | `CodingHarness` 拥有过多通用编排行为。 | 在 `opi-agent` 中移动或定义 generic phase/snapshot/save-point/session facade 语义。 |
| P1 | Session facade | Sessions 可用，但 richer context 不应通过临时 CLI-only writes 增加。 | 先第 10 阶段缝合点，再第 13 阶段 entries。 |
| P2 | Tooling quality | 内置工具可用。 | 第 11 阶段加固 normalization、diagnostics、cancellation、truncation 和 policy。 |
| P2 | TUI polish | 产品 TUI 可用。 | 第 14 阶段改善内置工作流，不承诺 custom extension UI。 |

## 未来生态候选

| 候选 | 当前矩阵等级 | 进入条件 |
|---|---|---|
| Provider OAuth / subscription auth | 缺失 | `Models/Auth` 稳定；credential store、login UX、refresh、redaction、doctor 和 revocation 已设计。 |
| 广泛 provider catalog | 部分 | 第 12 阶段 provider correctness 稳定；compatibility-profile quirks 已文档化。 |
| 图像生成 | 缺失 | Chat provider collection/auth/model metadata/cost/error semantics 稳定。 |
| Custom extension UI / message renderer | 缺失 | 第 14 阶段内置 TUI 稳定；UI/RPC 子协议已设计。 |
| npm/gallery/update/enable/disable | 缺失 | Package adapter lifecycle、trust/source model、diagnostics 和 lock/update policy 稳定。 |
| Web/share/session publishing | 缺失 | 第 13 阶段 export、redaction 和 session sensitivity 规则稳定。 |
| Provider request/response adapter hooks | 缺失 | Provider seam、hook ordering、redaction 和 trace semantics 稳定。 |
| `pi` session import/migration | 缺失 | `opi` session v2 稳定且用户价值明确。 |

## 维护规则

- 当阶段完成或研究的上游基线变化时更新本矩阵。
- 保守使用状态。没有工作中的用户路径或库路径加测试时，不标记为“完整”。
- 区分语义对齐和 TypeScript API/文件兼容。
- 优先遵循 Rust 原生 crate 归属，而不是复制 `pi` 包内部结构。
- 保留研究上游副本的证据锚点。若未来 `pi` 快照中的行号变化，应增加新锚点，而不是删除有价值的旧理由。
- 不要把未来生态候选写成当前产品声明。
- 保持英文和中文版本同步。
