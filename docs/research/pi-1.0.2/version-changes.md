# Pi 1.0.0 → 1.0.2 升级分析

> 核查日期：2026-10-05。当前对象：本地 `.repo/pi-1.0.2`。这是一份 Pi 自身的版本分析，不包含其他项目的迁移建议。

## 1 结论与证据边界

1.0.2 保留 13 个一级包、默认 AgentSession/JSONL 主链和实验 Chord/durable 服务链。没有证据表明默认产品已切换为 durable。变化集中在请求参数、provider 会话身份、MCP 配置与渲染、受限脚本输出、终端图片和可复现安装。

旧源码目录已不在工作区，旧版 `source-manifest.json` 已逐字节迁入本目录的历史证据附件，保留路径、大小、行数及 SHA-256。本轮用它与新快照逐文件比较，再依据新源码、测试源码和发行记录解释语义：**这是摘要差异驱动的复核，不是旧新源码的完整逐行 diff**。发行记录可以说明作者归属的版本；没有记录的变化仅归于这两个快照之间，不臆测具体提交、原因或修复时间。

| 指标 | 1.0.0 | 1.0.2 / 差异 |
|---|---:|---:|
| 快照文件数 | 1,848 | 1,859 |
| 一级包 | 13 | 13，manifest 均为 1.0.2 |
| 字节一致 | — | 1,705 |
| 相同路径内容改变 | — | 138 |
| 新增 / 删除 | — | 16 / 5 |
| 文档 | 125 / 33,468 行 | 125 / 33,781 行 |
| 变化 / 一致文档 | — | 26 / 99 |

摘要算法及逐文件数据分别见 [旧清单](evidence/pi-1.0.0/source-manifest.json) 和 [新清单](source-manifest.json)。新快照摘要：

```text
f0309990522b4c47b96e19ec684c53fd1c8fd8a5bc7918ba70ec3289bf3f13fb
```

## 2 各包变化范围

“未变化”仅指包内文件内容，不保证升级依赖后的所有运行行为相同。包版本统一更新本身也不能当作每个包新增能力的证据。

| 包 | 新增 | 修改 | 删除 | 一致 | 本版复核重点 |
|---|---:|---:|---:|---:|---|
| [agent](packages/agent.md) | 0 | 2 | 0 | 18 | 生产源码、测试和 README 一致；版本与发行标题更新 |
| [ai](packages/ai.md) | 1 | 23 | 0 | 355 | 分 thinking level 采样、inline tools、Bedrock thinking、重试/OAuth、模型数据生成 |
| [chord](packages/chord.md) | 0 | 1 | 0 | 63 | 仅 manifest 改变；规划矩阵与 runtime 机制保持 |
| [client](packages/client.md) | 0 | 2 | 0 | 17 | 版本与发行标题更新，连接/订阅代码一致 |
| [codemode](packages/codemode.md) | 0 | 6 | 0 | 12 | 输出字符/项数限制与不可恢复的失败结算 |
| [coding-agent](packages/coding-agent.md) | 5 | 59 | 4 | 796 | MCP 项目覆盖/CIMD、独立 tool renderer、采样配置、登录 URL、交付物 |
| [durable](packages/durable.md) | 2 | 13 | 0 | 140 | 持久 provider session、旧会话补建、fork/checkpoint、compaction/view |
| [evals](packages/evals.md) | 0 | 2 | 0 | 28 | 容器入口要求与更新后受测产品；统计协议一致 |
| [mcp](packages/mcp.md) | 0 | 7 | 0 | 25 | CIMD document 回调接口、非持久化 client identity、OAuth callback 路径绑定 |
| [protocol](packages/protocol.md) | 0 | 2 | 0 | 15 | 版本与发行标题更新，wire 类型/codec/tests 一致 |
| [server](packages/server.md) | 0 | 2 | 0 | 27 | 版本与发行标题更新，router/lease/listener/tests 一致 |
| [telemetry](packages/telemetry.md) | 0 | 2 | 0 | 10 | 版本与发行标题更新，契约/实现/测试/README 一致 |
| [tui](packages/tui.md) | 0 | 8 | 0 | 113 | 同步图片转码、缓存、Kitty 图片 placement 与滚动 |

## 3 实现变化及其含义

### 3.1 AI 请求配置不再只有一组固定 sampling 参数

`samplingParamsByThinkingLevel` 为模型提供按有效 thinking level 选择的参数组。共同解析函数先 clamp level，再按“模型基础参数 → level 参数 → 请求参数”逐键合并；这与单独设置 reasoning 强度不同。OpenAI Completions、Responses、Azure Responses 的 payload 构造接入了这个机制，coding-agent 的 `models.json` schema、模型组合与使用文档同步支持。具体 raw/simple 调用差异、temperature 与 null 的处理见 [AI 报告](packages/ai.md) 和 [应用报告](packages/coding-agent.md)。

[共同解析](../../../.repo/pi-1.0.2/packages/ai/src/api/simple-options.ts#L24)、[产品配置说明](../../../.repo/pi-1.0.2/packages/coding-agent/docs/models.md#L101)。

1.0.1 的 provider 修正还涉及 Anthropic 的 inline tool definitions、工具同名重定义后的缓存、Bedrock stale thinking、容量错误重试、ChatGPT OAuth 回调端口占用、Cloudflare classifier 与模型元数据。必须区分“生成器会产生的新目录”与“快照已经包含的生成文件”，也不能把厂商请求格式的代码修改直接当作本轮外部服务实测。`hasToolRedefinitions()` 已标记弃用但未删除。

### 3.2 durable 区分 conversation ID 与 provider session ID

新增 `pi.provider` document 持久保存 UUIDv7，scope 为 conversation，history 为 latest，fork 为 initial，并在 checkpoint 保留该身份。fork/child 初始化新的身份；同一 conversation 的重开、重试和恢复沿用已保存身份。旧 conversation 首次需要发送 provider 请求时，通过一次提交补建身份，提交在发请求之前完成。

[ProviderDoc 与补建](../../../.repo/pi-1.0.2/packages/durable/src/harness/provider.ts#L12)、[发行说明](../../../.repo/pi-1.0.2/packages/durable/CHANGELOG.md#L7)。

generation 和 compaction 将这个 sessionId 传给 AI 层；它用于 provider 的 prompt-cache/session affinity，不能解释成新建一套会话存储协议，也不能证明每次请求命中缓存。compaction 仍有 `cacheRetention: "none"`，文档所提“精确重发上一请求以复用缓存”的候选方案仍不在 v1。view 投影及相关生成、会话、压缩测试也发生变化，当前具体状态语义见 [durable 报告](packages/durable.md)。

### 3.3 MCP 注册、配置和 UI 生命周期分别扩展

底层 OAuth 的新接口把 `clientMetadataUrl` 替换为 `clientMetadataDocument(metadata)` 回调，返回 document URL 与独立 redirect URL，document 身份不持久化为动态注册凭据；callback 增加 `extraPaths` 和每次授权的预期路径绑定。CIMD 允许使用外部 client metadata document URL 作为 client ID，应用通过显式 `oauth.clientRegistration: "cimd"` 选择。callback 返回 issuer 和 exchange 的 `iss` 校验已在旧版存在；本版复核调用方仍需正确转交该值，不能将这项已有能力重复计为新增。

可信项目的 `.pi/mcp.json` 可以用没有 command/url/type 的条目覆盖用户级 server 的 enabled、exposure 和 toolExposure；toolExposure 是整表浅替换，不代表复制一个独立连接定义。`registerToolRenderer()` 则让工具渲染不再依赖工具已经接入，解决恢复会话与 HTML 导出中离线 MCP 工具的展示问题；renderer 的存在不授予执行能力。OAuth 对话框新增 URL copy，属于登录交互，不能与协议注册或凭据隔离混为一谈。

详见 [MCP 包](packages/mcp.md) 与 [coding-agent](packages/coding-agent.md)。本版保留两个不同的文档问题：包 README 的回调示例仍遗漏 issuer 转交；测试说明中“不使用 CIMD”的旧总括已失效。

### 3.4 codemode 为宿主输出累计量设置限制

每次脚本输出最多 16 × 1024 × 1024 字符和 100,000 项，覆盖 text、console 和图片 base64 数据。字符按 JavaScript `string.length` 的 UTF-16 code units 累计，不是字节或用户感知字符数，也不是整个宿主进程的 RSS 上限。超过限制先结算为失败再抛异常，脚本 catch 不能将该次执行改回成功。此前报告中无输出累计上限的表述已更新；VM 内存、宿主工具结果和输出队列仍须分别理解。

[输出常量与意图](../../../.repo/pi-1.0.2/packages/codemode/src/runtime/prelude-source.ts#L36)；详细失败顺序见 [codemode 报告](packages/codemode.md)。

### 3.5 TUI 图片协议修正不等于布局路线图完成

TUI 增加同步 ImageTranscoder 接线与缓存，为需要 PNG 的 Kitty 路径转换 JPEG/GIF/WebP；Kitty 图片传输和 placement 的组合也有调整，以处理全屏滚动后图片缩成单行的问题。库的同步回调与产品的图片处理实现分别归属，不能统称异步转码。

基础 ScrollView、scrollbar、selection 的既有能力仍保留；transcript virtualization、独立 overlay layout 等文档方向没有因此自动兑现。`contain` 的剩余 wheel delta 回退到 primary 的静态差距也单独复核，详见 [TUI 报告](packages/tui.md)。

## 4 安装、构建与发布变化

### 4.1 三种依赖固定方式

| 路径 | 当前固定方式 | 分析边界 |
|---|---|---|
| monorepo 开发 | 根 package-lock，直接外部依赖精确版本 | 仅有 manifest 不能替代依赖树检查 |
| 普通 npm 安装 | package manifest；已移除 npm-shrinkwrap | 传递依赖不再由发布包整体锁住，库消费者可覆盖 |
| 托管 installer / Nix runtime | 独立 install-lock，Nix 以本地构建 tarballs 替换内部包 | 构建输入还包括 Nix pin、模型目录及原生资产 |

`packages/coding-agent/npm-shrinkwrap.json` 与其生成器已删除，根 scripts/check/release 不再生成或检查它。原报告把所有安装途径都描述为锁定传递依赖的表述不能延续。新版 README 主推托管安装并说明 npm 差异；这只是分析仓库说明，不执行其中的联网安装命令。

[安装契约](../../../.repo/pi-1.0.2/README.md#L37)、[Nix install lock 替换](../../../.repo/pi-1.0.2/nix/package.nix#L82)。

### 4.2 Nix 构建与运行时模型更新是不同阶段

flake 声明 Linux/macOS 各两种架构；Intel macOS 使用单独 nixpkgs pin。构建以 root lock 安装 workspace、用固定内容摘要的模型 catalog hydrate 数据、执行 offline build，再把八个 CLI 运行依赖包打成 tarball，替换 install-lock 中对应的 registry 包。wrapper 提供 Node、fd、ripgrep；Linux 另处理剪贴板工具和原生动态库。

“offline build”描述模型数据已由固定输入准备后的构建过程，不表示首次取得 npm/Nix/catalog 输入不需要网络，也不表示运行时禁止拉取新模型目录。原生预构建资产仍参与交付，Nix metadata 明确同时列出 fromSource 和 binaryNativeCode。

[flake 系统集](../../../.repo/pi-1.0.2/flake.nix#L20)、[构建与打包](../../../.repo/pi-1.0.2/nix/package.nix#L32)、[安装后检查](../../../.repo/pi-1.0.2/nix/package.nix#L142)、[来源类型](../../../.repo/pi-1.0.2/nix/package.nix#L175)。

### 4.3 catalog pin 的“过时”有具体规则

`--if-stale` 校验现有 revision 内容摘要和 hydrate 可行性，并比较当前 checkout 的 provider/type 组是否覆盖 live catalog 中的同类组；同组内新增一个模型不必强制换 pin。live body、immutable revision body 都要与 SHA-256 一致，新 pin 在可 hydrate 后才写入。旧 revision 网络不可达会报错，不能悄悄当作内容过时替换。

因此这个命令仍会访问 live catalog，即使最终保留旧 pin；它不是纯离线 freshness 检查。本轮只阅读脚本和 mock 测试，没有调用端点或改动 pin。

[更新决策](../../../.repo/pi-1.0.2/scripts/update-model-catalog-pin.mjs#L76)、[保留及失败用例](../../../.repo/pi-1.0.2/scripts/update-model-catalog-pin.test.mjs#L97)。

### 4.4 CI 声明与实际证据

Nix workflow 的构建矩阵为 x86_64-linux、aarch64-linux、aarch64-darwin；x86_64-darwin 仅包括在全系统求值中。tag 构建使用 tag 的 pin，成功后才 fast-forward stable；普通 main catalog 更新在需要更换 pin 时构建并提交 pin。此处描述工作流条件，不声称远端 job 已成功或 stable 当前指向某个提交。

release 脚本在版本递增前按需刷新 Nix pin，以便 pin 进入 tag。npm audit workflow 改用本地解析脚本，以 moderate 为失败阈值，带按 advisory ID 与包名匹配的接受列表；当前 node-forge 例外的理由来自仓库作者说明，本轮没有验证公告或执行漏洞扫描。registry signatures 仍是独立步骤。

[Nix build 矩阵](../../../.repo/pi-1.0.2/.github/workflows/nix.yml#L92)、[stable 更新条件](../../../.repo/pi-1.0.2/.github/workflows/nix.yml#L135)、[release pin 顺序](../../../.repo/pi-1.0.2/scripts/release.mjs#L231)、[audit 接受策略](../../../.repo/pi-1.0.2/scripts/npm-audit.mjs#L13)。

入口图预算也明确计入 durable 的 provider-session UUID 依赖，当前根入口预算为 62 个可达文件，仍禁止拉入 AI root barrel。锁文件提交检查允许仅 workspace package metadata 的变化，或显式允许标记；不是对任何依赖改动的自动豁免。

[入口预算](../../../.repo/pi-1.0.2/scripts/check-entry-graphs.mjs#L47)、[锁文件 gate](../../../.repo/pi-1.0.2/scripts/check-lockfile-commit.mjs#L54)。

## 5 文档与规划如何继承

26 份文档变化包含 README、CHANGELOG、产品说明、durable spec 和仓库工作约定；没有新增或删除匹配清单口径的文档。六份核心设计材料中只有 durable spec 变化。所有旧方向仍按“已兑现、部分、未实现、被替代、非目标、历史实验、证据不足”分别判断，不把 patch 版本号当作规划完成度。

重点更正包括 provider session 的现行契约、MCP CIMD 的旧测试说明、codemode 输出上限，以及 npm 与托管安装的区别。Chord symmetric RpcPeer、structural reload、TUI virtualization、durable 额外工具与缓存友好摘要等尚未完成方向仍有明确范围；同一份 spec 的新段落并不能让其中旧的“尚未实现”状态句变成正确。

逐条结论见 [设计文档核查](documentation-audit.md)；每篇材料的继承/重新阅读深度见 [文档索引](documentation-index.md)。报告没有访问外部 RFC，因此不承诺穷尽上游长期规划。

## 6 逐文件变化清单

下面是完整摘要差异。列入清单只证明文件身份变化，不代表逐行安全审计；锁文件和生成数据按结构、关联 manifest 与消费路径检查，不宣称逐个验证所有传递依赖。删除文件仅记路径，避免生成指向不存在源码的链接。

### 新增（16）

- [.github/workflows/nix.yml](../../../.repo/pi-1.0.2/.github/workflows/nix.yml)
- [flake.lock](../../../.repo/pi-1.0.2/flake.lock)
- [flake.nix](../../../.repo/pi-1.0.2/flake.nix)
- [nix/model-catalog.json](../../../.repo/pi-1.0.2/nix/model-catalog.json)
- [nix/package.nix](../../../.repo/pi-1.0.2/nix/package.nix)
- [packages/ai/scripts/hydrate-model-catalog.ts](../../../.repo/pi-1.0.2/packages/ai/scripts/hydrate-model-catalog.ts)
- [packages/coding-agent/src/modes/interactive/components/auth-url.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/auth-url.ts)
- [packages/coding-agent/src/modes/interactive/components/easter-egg-3d.lazy.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/easter-egg-3d.lazy.ts)
- [packages/coding-agent/src/modes/interactive/components/easter-egg-3d.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/easter-egg-3d.ts)
- [packages/coding-agent/test/auth-url-copy.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/auth-url-copy.test.ts)
- [packages/coding-agent/test/suite/regressions/10285-mcp-tool-renderers.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/suite/regressions/10285-mcp-tool-renderers.test.ts)
- [packages/durable/src/harness/provider.ts](../../../.repo/pi-1.0.2/packages/durable/src/harness/provider.ts)
- [packages/durable/test/provider-session-cache-e2e.test.ts](../../../.repo/pi-1.0.2/packages/durable/test/provider-session-cache-e2e.test.ts)
- [scripts/npm-audit.mjs](../../../.repo/pi-1.0.2/scripts/npm-audit.mjs)
- [scripts/update-model-catalog-pin.mjs](../../../.repo/pi-1.0.2/scripts/update-model-catalog-pin.mjs)
- [scripts/update-model-catalog-pin.test.mjs](../../../.repo/pi-1.0.2/scripts/update-model-catalog-pin.test.mjs)

### 删除（5）

- `packages/coding-agent/npm-shrinkwrap.json`
- `packages/coding-agent/src/modes/interactive/components/daxnuts.ts`
- `packages/coding-agent/src/modes/interactive/components/pi-logo-animation.lazy.ts`
- `packages/coding-agent/src/modes/interactive/components/pi-logo-animation.ts`
- `scripts/generate-coding-agent-shrinkwrap.mjs`

### 修改（138）

- [.github/workflows/npm-audit.yml](../../../.repo/pi-1.0.2/.github/workflows/npm-audit.yml)
- [.pi/skills/release.md](../../../.repo/pi-1.0.2/.pi/skills/release.md)
- [AGENTS.md](../../../.repo/pi-1.0.2/AGENTS.md)
- [README.md](../../../.repo/pi-1.0.2/README.md)
- [package-lock.json](../../../.repo/pi-1.0.2/package-lock.json)
- [package.json](../../../.repo/pi-1.0.2/package.json)
- [packages/agent/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/agent/CHANGELOG.md)
- [packages/agent/package.json](../../../.repo/pi-1.0.2/packages/agent/package.json)
- [packages/ai/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/ai/CHANGELOG.md)
- [packages/ai/README.md](../../../.repo/pi-1.0.2/packages/ai/README.md)
- [packages/ai/package.json](../../../.repo/pi-1.0.2/packages/ai/package.json)
- [packages/ai/scripts/generate-models.ts](../../../.repo/pi-1.0.2/packages/ai/scripts/generate-models.ts)
- [packages/ai/scripts/model-data.ts](../../../.repo/pi-1.0.2/packages/ai/scripts/model-data.ts)
- [packages/ai/src/api/anthropic-messages.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/anthropic-messages.ts)
- [packages/ai/src/api/azure-openai-responses.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/azure-openai-responses.ts)
- [packages/ai/src/api/bedrock-converse-stream.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/bedrock-converse-stream.ts)
- [packages/ai/src/api/cloudflare-workers-ai-system-one.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/cloudflare-workers-ai-system-one.ts)
- [packages/ai/src/api/openai-completions.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/openai-completions.ts)
- [packages/ai/src/api/openai-responses.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/openai-responses.ts)
- [packages/ai/src/api/simple-options.ts](../../../.repo/pi-1.0.2/packages/ai/src/api/simple-options.ts)
- [packages/ai/src/auth/oauth/openai-chatgpt.ts](../../../.repo/pi-1.0.2/packages/ai/src/auth/oauth/openai-chatgpt.ts)
- [packages/ai/src/types.ts](../../../.repo/pi-1.0.2/packages/ai/src/types.ts)
- [packages/ai/src/utils/retry.ts](../../../.repo/pi-1.0.2/packages/ai/src/utils/retry.ts)
- [packages/ai/src/utils/transcript.ts](../../../.repo/pi-1.0.2/packages/ai/src/utils/transcript.ts)
- [packages/ai/test/bedrock-thinking-payload.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/bedrock-thinking-payload.test.ts)
- [packages/ai/test/cloudflare-workers-ai-system-one.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/cloudflare-workers-ai-system-one.test.ts)
- [packages/ai/test/model-data-validation.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/model-data-validation.test.ts)
- [packages/ai/test/sampling-options.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/sampling-options.test.ts)
- [packages/ai/test/stream.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/stream.test.ts)
- [packages/ai/test/together-models.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/together-models.test.ts)
- [packages/ai/test/transcript-tool-changes.test.ts](../../../.repo/pi-1.0.2/packages/ai/test/transcript-tool-changes.test.ts)
- [packages/chord/package.json](../../../.repo/pi-1.0.2/packages/chord/package.json)
- [packages/client/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/client/CHANGELOG.md)
- [packages/client/package.json](../../../.repo/pi-1.0.2/packages/client/package.json)
- [packages/codemode/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/codemode/CHANGELOG.md)
- [packages/codemode/README.md](../../../.repo/pi-1.0.2/packages/codemode/README.md)
- [packages/codemode/package.json](../../../.repo/pi-1.0.2/packages/codemode/package.json)
- [packages/codemode/src/index.ts](../../../.repo/pi-1.0.2/packages/codemode/src/index.ts)
- [packages/codemode/src/runtime/prelude-source.ts](../../../.repo/pi-1.0.2/packages/codemode/src/runtime/prelude-source.ts)
- [packages/codemode/test/sandbox.test.ts](../../../.repo/pi-1.0.2/packages/codemode/test/sandbox.test.ts)
- [packages/coding-agent/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/coding-agent/CHANGELOG.md)
- [packages/coding-agent/README.md](../../../.repo/pi-1.0.2/packages/coding-agent/README.md)
- [packages/coding-agent/docs/cli.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/cli.md)
- [packages/coding-agent/docs/codemode.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/codemode.md)
- [packages/coding-agent/docs/extensions.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/extensions.md)
- [packages/coding-agent/docs/keybindings.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/keybindings.md)
- [packages/coding-agent/docs/mcp.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/mcp.md)
- [packages/coding-agent/docs/models.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/models.md)
- [packages/coding-agent/docs/quickstart.md](../../../.repo/pi-1.0.2/packages/coding-agent/docs/quickstart.md)
- [packages/coding-agent/examples/extensions/custom-provider-anthropic/package-lock.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/custom-provider-anthropic/package-lock.json)
- [packages/coding-agent/examples/extensions/custom-provider-anthropic/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/custom-provider-anthropic/package.json)
- [packages/coding-agent/examples/extensions/custom-provider-gitlab-duo/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/custom-provider-gitlab-duo/package.json)
- [packages/coding-agent/examples/extensions/gondolin/package-lock.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/gondolin/package-lock.json)
- [packages/coding-agent/examples/extensions/gondolin/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/gondolin/package.json)
- [packages/coding-agent/examples/extensions/sandbox/package-lock.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/sandbox/package-lock.json)
- [packages/coding-agent/examples/extensions/sandbox/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/sandbox/package.json)
- [packages/coding-agent/examples/extensions/with-deps/package-lock.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/with-deps/package-lock.json)
- [packages/coding-agent/examples/extensions/with-deps/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/examples/extensions/with-deps/package.json)
- [packages/coding-agent/install-lock/package-lock.json](../../../.repo/pi-1.0.2/packages/coding-agent/install-lock/package-lock.json)
- [packages/coding-agent/install-lock/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/install-lock/package.json)
- [packages/coding-agent/package.json](../../../.repo/pi-1.0.2/packages/coding-agent/package.json)
- [packages/coding-agent/src/cli/args.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/cli/args.ts)
- [packages/coding-agent/src/config.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/config.ts)
- [packages/coding-agent/src/core/agent-session.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/agent-session.ts)
- [packages/coding-agent/src/core/export-html/tool-renderer.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/export-html/tool-renderer.ts)
- [packages/coding-agent/src/core/extensions/index.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/extensions/index.ts)
- [packages/coding-agent/src/core/extensions/loader.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/extensions/loader.ts)
- [packages/coding-agent/src/core/extensions/runner.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/extensions/runner.ts)
- [packages/coding-agent/src/core/extensions/types.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/extensions/types.ts)
- [packages/coding-agent/src/core/mcp-servers.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/mcp-servers.ts)
- [packages/coding-agent/src/core/model-config.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/model-config.ts)
- [packages/coding-agent/src/core/model-resolver.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/model-resolver.ts)
- [packages/coding-agent/src/core/provider-composer.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/provider-composer.ts)
- [packages/coding-agent/src/core/tools/renderers/index.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/core/tools/renderers/index.ts)
- [packages/coding-agent/src/extensions/mcp/cli.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/cli.ts)
- [packages/coding-agent/src/extensions/mcp/config.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/config.ts)
- [packages/coding-agent/src/extensions/mcp/index.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/index.ts)
- [packages/coding-agent/src/extensions/mcp/oauth.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/oauth.ts)
- [packages/coding-agent/src/extensions/mcp/runtime.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/runtime.ts)
- [packages/coding-agent/src/extensions/mcp/tools.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/tools.ts)
- [packages/coding-agent/src/extensions/mcp/ui.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/extensions/mcp/ui.ts)
- [packages/coding-agent/src/index.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/index.ts)
- [packages/coding-agent/src/modes/interactive/components/armin.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/armin.ts)
- [packages/coding-agent/src/modes/interactive/components/index.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/index.ts)
- [packages/coding-agent/src/modes/interactive/components/login-dialog.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/login-dialog.ts)
- [packages/coding-agent/src/modes/interactive/components/tool-execution.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/components/tool-execution.ts)
- [packages/coding-agent/src/modes/interactive/interactive-mode.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/interactive-mode.ts)
- [packages/coding-agent/src/package-manager-cli.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/package-manager-cli.ts)
- [packages/coding-agent/src/utils/image-convert.ts](../../../.repo/pi-1.0.2/packages/coding-agent/src/utils/image-convert.ts)
- [packages/coding-agent/test/args.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/args.test.ts)
- [packages/coding-agent/test/export-html-whitespace.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/export-html-whitespace.test.ts)
- [packages/coding-agent/test/extensions-runner.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/extensions-runner.test.ts)
- [packages/coding-agent/test/image-processing.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/image-processing.test.ts)
- [packages/coding-agent/test/mcp-extension.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/mcp-extension.test.ts)
- [packages/coding-agent/test/mcp-oauth-refresh.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/mcp-oauth-refresh.test.ts)
- [packages/coding-agent/test/model-registry.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/model-registry.test.ts)
- [packages/coding-agent/test/suite/mcp-oauth-server.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/suite/mcp-oauth-server.ts)
- [packages/coding-agent/test/suite/regressions/7027-credential-refresh-hang.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/suite/regressions/7027-credential-refresh-hang.test.ts)
- [packages/coding-agent/test/tool-execution-component.test.ts](../../../.repo/pi-1.0.2/packages/coding-agent/test/tool-execution-component.test.ts)
- [packages/durable/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/durable/CHANGELOG.md)
- [packages/durable/README.md](../../../.repo/pi-1.0.2/packages/durable/README.md)
- [packages/durable/docs/spec.md](../../../.repo/pi-1.0.2/packages/durable/docs/spec.md)
- [packages/durable/package.json](../../../.repo/pi-1.0.2/packages/durable/package.json)
- [packages/durable/src/harness/compaction.ts](../../../.repo/pi-1.0.2/packages/durable/src/harness/compaction.ts)
- [packages/durable/src/harness/generation.ts](../../../.repo/pi-1.0.2/packages/durable/src/harness/generation.ts)
- [packages/durable/src/harness/harness.ts](../../../.repo/pi-1.0.2/packages/durable/src/harness/harness.ts)
- [packages/durable/src/harness/view.ts](../../../.repo/pi-1.0.2/packages/durable/src/harness/view.ts)
- [packages/durable/src/index.ts](../../../.repo/pi-1.0.2/packages/durable/src/index.ts)
- [packages/durable/test/harness-compaction.test.ts](../../../.repo/pi-1.0.2/packages/durable/test/harness-compaction.test.ts)
- [packages/durable/test/harness-conversations.test.ts](../../../.repo/pi-1.0.2/packages/durable/test/harness-conversations.test.ts)
- [packages/durable/test/harness-generation.test.ts](../../../.repo/pi-1.0.2/packages/durable/test/harness-generation.test.ts)
- [packages/durable/test/harness-view.test.ts](../../../.repo/pi-1.0.2/packages/durable/test/harness-view.test.ts)
- [packages/evals/docker/entrypoint.ts](../../../.repo/pi-1.0.2/packages/evals/docker/entrypoint.ts)
- [packages/evals/package.json](../../../.repo/pi-1.0.2/packages/evals/package.json)
- [packages/mcp/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/mcp/CHANGELOG.md)
- [packages/mcp/package.json](../../../.repo/pi-1.0.2/packages/mcp/package.json)
- [packages/mcp/src/oauth/callback.ts](../../../.repo/pi-1.0.2/packages/mcp/src/oauth/callback.ts)
- [packages/mcp/src/oauth/flow.ts](../../../.repo/pi-1.0.2/packages/mcp/src/oauth/flow.ts)
- [packages/mcp/src/oauth/index.ts](../../../.repo/pi-1.0.2/packages/mcp/src/oauth/index.ts)
- [packages/mcp/src/oauth/provider.ts](../../../.repo/pi-1.0.2/packages/mcp/src/oauth/provider.ts)
- [packages/mcp/test/oauth.test.ts](../../../.repo/pi-1.0.2/packages/mcp/test/oauth.test.ts)
- [packages/protocol/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/protocol/CHANGELOG.md)
- [packages/protocol/package.json](../../../.repo/pi-1.0.2/packages/protocol/package.json)
- [packages/server/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/server/CHANGELOG.md)
- [packages/server/package.json](../../../.repo/pi-1.0.2/packages/server/package.json)
- [packages/telemetry/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/telemetry/CHANGELOG.md)
- [packages/telemetry/package.json](../../../.repo/pi-1.0.2/packages/telemetry/package.json)
- [packages/tui/CHANGELOG.md](../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md)
- [packages/tui/package.json](../../../.repo/pi-1.0.2/packages/tui/package.json)
- [packages/tui/src/components/image.ts](../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts)
- [packages/tui/src/index.ts](../../../.repo/pi-1.0.2/packages/tui/src/index.ts)
- [packages/tui/src/terminal-image.ts](../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts)
- [packages/tui/src/tui-alt-screen.ts](../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)
- [packages/tui/test/terminal-image.test.ts](../../../.repo/pi-1.0.2/packages/tui/test/terminal-image.test.ts)
- [packages/tui/test/tui-alt-screen.test.ts](../../../.repo/pi-1.0.2/packages/tui/test/tui-alt-screen.test.ts)
- [scripts/check-entry-graphs.mjs](../../../.repo/pi-1.0.2/scripts/check-entry-graphs.mjs)
- [scripts/check-lockfile-commit.mjs](../../../.repo/pi-1.0.2/scripts/check-lockfile-commit.mjs)
- [scripts/release.mjs](../../../.repo/pi-1.0.2/scripts/release.mjs)

## 7 本轮验证范围

报告验证覆盖全部 13 包、来源链接存在性与行号范围、文档索引完整性、文件摘要一致性及文档规范。不运行 Pi 的 build、测试、benchmark、Nix、Docker、模型请求、OAuth 或发布脚本。因此测试源码中的期望、作者的 release 声明和本轮实际执行结果始终分开；测试影响为 `none`。

当前以本目录为分析基准，旧报告目录已移除。版本比较所需的旧清单和完整文档审读记录已迁入 [历史证据](evidence/pi-1.0.0/README.md)，旧报告全文可从其中记录的固定 Git 提交恢复。当前源码链接全部指向 1.0.2；历史记录仍保留原版本语义。
