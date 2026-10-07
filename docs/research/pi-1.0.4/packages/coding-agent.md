# coding-agent：默认编码会话与实验性持久化运行时

> 分析对象：本地 `pi-1.0.4` 源码快照的 `packages/coding-agent`；核查日期 2026-10-07。
> 方法：静态阅读 manifest、真实入口、实现、测试及随包文档；未运行构建、测试、CLI、模型请求或外部服务。
> 文中“已确认”表示可由引用源码直接确定；“观察/推断”表示基于实现形成的解释，不代表实测结果。测试章节描述测试意图与断言，不声称测试通过。

## 0. 1.0.2 → 1.0.4：范围、变化与证据继承

本包当前有 **861 个文件、299 个 src TypeScript 文件（85,290 行）、311 个测试文件（67,245 行）**。相对 1.0.2 清单：**新增 1、修改 56、删除 0、未变 804**；6 份 Markdown 改变、73 份未变。旧源码不在工作区，比较依据是双份清单、当前实现和版本记录，不声称取得完整逐行历史 diff。[当前清单](../source-manifest.json)、[历史证据](../evidence/pi-1.0.2/README.md)。

| 变化组 | 当前契约及正文位置 |
|---|---|
| 工具选择 | 星号模式、MCP 注册保留与直接声明分别判断、no-mcp 禁用内置工厂（5.1、12.2、14.1） |
| 提示一致性 | hidden 工具离开默认 rules；间接 reader 的 skills 提示不点名；guidelines 经 ToolLoadout/codemode 传递（10.3、12.2） |
| 图片与输出 | read 的结构化图片、codemode 图片落盘、统一独占创建及 0600 输出文件（11.2、11.5、13.2） |
| Provider/认证 | Azure provider 改名、当前默认值、token 刷新独立取消域、MCP native 注册与连接关闭（9.2、9.5、14.2） |
| TUI/安装 | Home/End 分工、逐行高亮、失效终端退出、安装变化提示和 worker 内存副本（16.1） |
| 实验调用方 | 仍由旧 NodeExecutionEnv 连接 durable Harness，新 env 包没有自动替代默认或实验入口（17.3） |

作者版本归属见 [1.0.3/1.0.4 发布段](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md#L3)。本报告改写相关正文，未变子系统按历史证据保留完整解释。全文/选段/继承范围见第 24–25 节；文件摘要一致不证明依赖行为也未变。

## 1. 核心结论与边界

`coding-agent` 是把通用 Agent、模型目录与认证、文件及命令工具、会话存储、扩展机制和终端产品组合起来的应用包，同时暴露可嵌入 SDK。默认产品的中心对象是 `AgentSession`，而不是实验目录中的 durable `Harness`。
默认路径沿 `cli.ts → main.ts → createAgentSessionServices → createAgentSession → AgentSession → Agent` 工作，持久化格式是带父指针的 JSONL v3。
另一条明确隔离的源码实验路径沿 `experimental/cli.ts → server/client → Session worker → durable Harness + SQLite + Chord facets` 工作。
二者会复用模型配置、设置、终端组件等代码，但运行状态、存储格式、工具实现和远程协议不能混为一谈。
依据：[src/cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/cli.ts#L1)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L784)。

本版本尤其需要区分以下概念：

| 概念 | 默认路径的含义 | 不能据此推导的结论 |
|---|---|---|
| Agent | 通用工具调用循环与流式状态 | 不负责完整 CLI、项目信任或 JSONL 文件生命周期 |
| AgentSession | 会话、队列、扩展、压缩、工具负载、模型与事件协调 | 不等于 durable conversation |
| AgentSessionRuntime | 管理“当前会话及其 cwd 绑定服务”的可替换宿主 | 不是远程服务进程 |
| SessionManager | 追加式会话树及投影 | 不是数据库事务引擎 |
| 稳定 RPC | 单进程 stdin/stdout JSONL 命令与事件 | 不使用实验性 Chord 服务目录 |
| extension | 同进程 TypeScript/JavaScript 工厂及 hooks | 不等于隔离插件或 Chord facet |
| codemode sandbox | 受限 JavaScript VM，通过显式桥接调用工具 | 不把被调用的 shell、文件操作变成 OS 沙箱 |
| 项目信任 | 控制大多数项目资源是否加载 | 不限制工具访问路径，也不逐次批准工具执行 |

接口边界可从 [src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L74)、[src/modes/rpc/rpc-types.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-types.ts#L20)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L3) 交叉确认。

## 2. Manifest、导出面与交付形态

### 2.1 包与入口

包名为 `@earendil-works/pi-coding-agent`，版本 `1.0.4`，ESM，Node 要求 `>=22.19.0`。
命令 `pi` 指向 `dist/bundle/cli.js`；SDK 主入口和声明文件分别是 `dist/index.js`、`dist/index.d.ts`。
`./rpc-entry` 有正式 `import` 条件；`./client` 与 `./experimental/plugin` 仅提供 `source` 条件，并指向源码。
发布文件列表明确排除 `dist/client`、`dist/experimental`、`dist/cli/experimental`，因此源码存在不等于发布产品已开放该能力。
依据：[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)。

SDK 根入口导出面相当宽：会话和运行时创建、模型运行时、资源加载器、SessionManager、扩展类型、工具工厂、压缩、主题及部分终端组件、图片处理等。
它不仅是 `createAgentSession()` 的薄封装，也允许调用方替换资源和工具操作实现。
内置 codemode/MCP/tool-search 的工厂是显式导出项；调用 SDK 并不自动复制 CLI 注入全部内置扩展的策略。
依据：[src/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/index.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)。

### 2.2 依赖与跨包关系

| 依赖 | 本包职责中的位置 |
|---|---|
| `pi-agent-core` | 默认 Agent、消息/工具接口、工具调用流水线 |
| `pi-ai` | 模型、Provider、流式请求、认证与内容类型 |
| `pi-tui` | 终端、编辑器、布局和渲染基础 |
| `pi-codemode` | QuickJS 脚本沙箱、源解析与工具声明渲染 |
| `pi-mcp` | MCP 客户端、stdio/HTTP transport 及协议类型 |
| `chord` | 本包声明的运行依赖；实验服务与 facets 使用它 |
| `pi-client`、`pi-protocol`、`pi-server` | manifest 的开发依赖；源码实验客户端/服务端接线使用 |
| `pi-durable` | 实验实现直接导入 Harness、SQLite、CodingTools；未列入本包 dependencies/devDependencies，源码解析由仓库 source resolver 支撑 |

最后一行是 manifest 与源码分层的事实，不在缺乏构建验证时判断为打包缺陷。
依据：[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L1)、[src/experimental/source-resolver.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/source-resolver.ts#L45)。

### 2.3 构建与资源定位

常规构建先 `tsc` 生成未打包输出及 assets，再由仓库级 bundle 脚本生成 CLI/SDK/RPC 入口。
TypeScript 构建配置排除实验目录；bundle 以 Node 22.19 为目标，对指定原生/外部包保留 external，并校验意外残留的外部依赖。
codemode worker、OAuth 和部分 Provider 采用独立或延迟入口，避免所有可选能力进入启动热路径。
依据：[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[tsconfig.build.json](../../../../.repo/pi-1.0.4/packages/coding-agent/tsconfig.build.json#L19)、[scripts/build-coding-agent-bundle.mjs](../../../../.repo/pi-1.0.4/scripts/build-coding-agent-bundle.mjs)、[scripts/build-coding-agent-bundle.mjs](../../../../.repo/pi-1.0.4/scripts/build-coding-agent-bundle.mjs)。

单文件 binary 路径额外先构建若干兄弟包，再使用 Bun compile，将图片 worker 和 codemode worker 纳入入口并复制主题、导出模板、文档与 WASM。
`config.ts` 集中解决源码、常规 npm、bundle、binary 的资源位置；QuickJS WASM 和 worker specifier 并非到处硬编码相对路径。
默认用户目录是 `~/.pi/agent`，可由环境变量覆盖；模型、认证、设置、prompts、sessions 等路径由同一模块生成。
依据：[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[src/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/config.ts)。

### 2.4 npm、managed installer 与 Nix 的锁定边界

1.0.1 已删除发布包的 `npm-shrinkwrap.json`；当前普通 npm 安装允许重新解析传递依赖，库消费者也可覆盖它们。`brace-expansion:5.0.12` 被直接固定，manifest 仍有 `protobufjs:7.6.6` override；这不等于为任意消费者锁定整个传递图。`install-lock/package.json` 则固定安装根的 `pi-coding-agent:1.0.4`，与独立安装锁配合用于 managed installer/updater。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[install-lock/package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/install-lock/package.json)。

普通 npm 自更新命令带 `--ignore-scripts --min-release-age=0`；源码注释明确指出 npm 无单包 age gate，此参数也放行新的传递依赖。成功更新后的 npm 分支提示改用 managed installer；managed 路径下载 package/lock、执行 npm ci、验证版本、再激活版本目录，不能与普通 npm 路径混称。[src/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/config.ts)、[src/package-manager-cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/package-manager-cli.ts#L171)、[src/package-manager-cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/package-manager-cli.ts#L1096)。

README/quickstart 现以 managed installer 为主入口，同时说明普通 npm 与 Nix。Nix 在 macOS/Linux 从源码构建，可用 tag 固定发行版；`pi update` 不负责更新 Nix 提供的程序，packages 与模型目录更新仍可使用。quickstart 的 `v1.0.0` 是 pin 示例，不是声称当前版本仍为 1.0.0。此处只核对本包文档/入口，仓库 Nix 构建与发布接线见[版本变化报告](../version-changes.md)；未运行任何安装器。[README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/README.md#L37)、[docs/quickstart.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/quickstart.md#L23)、[docs/cli.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/cli.md)。

## 3. 源码目录地图

| 区域 | 关键文件 | 主要责任 |
|---|---|---|
| 启动 | `src/cli.ts`、`src/main.ts`、`src/cli/*` | 进程配置、参数、会话选择、模式分派 |
| SDK 装配 | `core/sdk.ts`、`agent-session-services.ts` | cwd 服务与 AgentSession 创建 |
| 会话宿主 | `core/agent-session-runtime.ts` | 新建/切换/分叉/导入以及销毁次序 |
| 会话协调 | `core/agent-session.ts` | prompt、队列、hooks、工具状态、压缩、树导航 |
| 历史与上下文 | `core/session-manager.ts`、`messages.ts`、`compaction/*` | JSONL 树、投影、摘要、模型消息转换 |
| 模型与认证 | `model-runtime.ts`、`model-registry.ts`、`auth-storage.ts` | Provider 组合、目录、路由、认证同步 |
| 配置与资源 | `settings-manager.ts`、`resource-loader.ts`、`package-manager.ts` | 多层设置、资源发现、包来源和优先级 |
| 项目信任 | `trust-manager.ts`、`project-trust.ts` | 资源检测、信任记录、决策顺序 |
| 工具 | `core/tools/*` | 八类内置工具、截断、变更队列、输出累积 |
| 扩展基础 | `core/extensions/*` | 工厂加载、运行时 API、事件派发、工具包装 |
| 默认扩展 | `extensions/{llama,codemode,tool-search,mcp}` | 本地模型管理、程序化工具组合、发现、MCP |
| 交互 | `modes/interactive/*` | 命令、编辑器、对话、树、主题、工具呈现 |
| 非交互 | `modes/print-mode.ts`、`modes/rpc/*` | 文本/JSON 输出、JSONL RPC、子进程客户端 |
| 实验 | `experimental/*` | server/client、worker、Chord services、durable、vacation |
| 辅助 | `utils/*`、`core/export-html/*` | shell/进程、图片、工具下载、HTML 导出 |
| 验证材料 | `test/*`、`test/suite/*`、`examples/*` | 单元/行为/集成测试与 SDK/扩展示例 |

阅读时可以把 `AgentSession` 看作协调中心，但模型目录、JSONL 投影和资源装配具有各自的独立状态与生命周期；不能只读这个大类就理解全部行为。
上述地图的公开边界集中展示在 [src/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/index.ts)，工具全集则在 [src/core/tools/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/index.ts#L114)。

## 4. 默认 CLI 启动链

### 4.1 进程准备与提前返回的命令

`cli.ts` 先调用 `setupCli()`，再运行 `main(process.argv.slice(2))`。
setup 设置进程名、`PI_CODING_AGENT`/`AI_AGENT`，配置 HTTP dispatcher，并统一处理运行时 warning。
`main()` 先合成内置和调用方 extension factories，再处理 auth、包管理、config、MCP 等可提前返回的子命令、参数诊断、版本和导出。
依据：[src/cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/cli.ts#L1)、[src/cli/setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/cli/setup.ts#L4)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)。

模式判断优先尊重显式 RPC/JSON；`-p` 或 stdin/stdout 任一非 TTY 时进入 print，否则 interactive。
机器输出模式安装 stdout guard，避免提示文本与 JSON 混流；RPC 不接受启动 `@file` 参数。
这说明默认模式由终端环境参与决定，而非只有一个布尔开关。
依据：[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)。

### 4.2 会话选择先于最终服务装配

`createSessionManager()` 处理无持久化、显式 session、ID/路径解析、fork、continue、resume picker 等分支。
显式 ID 需要验证；跨目录会话在相关交互路径中可转成新分支；会话头的 cwd 又会参与后续服务创建。
因此 `cwd` 既是 CLI 启动上下文，也是恢复会话的资源边界，切换会话不能只替换 `agent.state.messages`。
依据：[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L268)。

### 4.3 信任、资源和模型的次序

启动阶段先以 `projectTrusted:false` 创建 bootstrap SettingsManager，然后由目标 cwd 的 runtime factory 解析信任。
factory 把信任后的 SettingsManager、资源选项和内置 factories 交给 `createAgentSessionServices()`。
资源加载过程中注册的普通/native Provider 与虚拟模型会先进入 pending 列表，再应用到 ModelRuntime，最后进行不允许网络的 refresh，并收集诊断。
随后 CLI 才解析最终模型/思考等级/工具名单并创建 session。
依据：[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/core/agent-session-services.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-services.ts#L135)。

最后根据模式交给 `runRpcMode`、`InteractiveMode.run` 或 `runPrintMode`。
RPC 可以在启动完成后后台刷新模型目录，带超时且受 offline 控制；启动时本地可用性与后续网络发现是不同阶段。
依据：[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)。

## 5. SDK 与运行时所有权

### 5.1 `createAgentSession()` 的默认值和恢复规则

SDK 接受 cwd、agentDir、ModelRuntime、SettingsManager、ResourceLoader、SessionManager、模型、思考等级、工具允许/排除清单和自定义工具等注入点。
缺省时自行创建这些服务；已有 SessionManager 的 cwd 可以成为默认 cwd。
注意：直接调用 SDK 的 SettingsManager 默认项目信任为 true；CLI 的交互信任启动协议并非自动附着在 SDK 上。
依据：[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L442)。

模型选择次序为：显式传入模型；已有上下文分支中记录的模型且认证已配置；再交给初始模型解析器依据设置和 Provider 默认选择。
恢复虚拟模型必须查看 `model_change`，因为 assistant 消息记载的是实际回答的物理模型。
思考等级恢复也区分“会话确有 thinking entry”和“旧会话没有该条目”；最后按模型能力裁剪，无模型则 off。
依据：[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)。

默认激活工具是 read、bash、edit、write；显式工具、设置、`noTools`、排除清单决定最终负载。
SDK 通过 `new Agent()` 注入 ModelRuntime 的 stream 函数、消息转换、上下文变换、重试/队列配置和请求 hooks，再交给 AgentSession 装配工具与系统提示。
模块还设置 Agent 的默认 streamFn，供没有显式 streamFn 的扩展代码使用；这不改变 Agent 核心自身的 Provider 中立接口。
依据：[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L215)。

当前允许/排除项由 createToolNameMatcher 解释：只有星号是通配符，其余正则字符转义，匹配全名且区分大小写，排除优先。SDK tools 与 CLI 均进入同一会话选择逻辑；空 allowlist（包括 noTools:"all"）清空全部工具。MCP 另有保留政策，见 12.2，不能把 allowlist 简写成删除所有未列出的 MCP 注册。[匹配器](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/mcp-servers.ts#L12)、[SDK](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts#L270)、[构造](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts#L490)。

### 5.2 services 与 runtime 的拆分

`createAgentSessionServices()` 只创建 cwd 绑定的服务，不创建会话；`createAgentSessionFromServices()` 再调用 SDK。
这种拆分使调用方在创建 Agent 之前就能用最终目录的模型、资源与设置解析参数。
`AgentSessionRuntime` 持有当前 session，负责把它整体替换，而 UI/RPC 持有 runtime 这个稳定宿主。
依据：[src/core/agent-session-services.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-services.ts#L135)、[src/core/agent-session-services.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-services.ts#L214)、[src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L74)。

切换次序具有行为意义：先 abort 并等待旧运行落稳，使部分 assistant/工具结果得到记录；发送 session_shutdown；同步通知旧 UI 引用即将失效；dispose 旧 session；按新 cwd 创建和重新绑定服务。
`switchSession()` 在破坏旧会话前检查目标头部 cwd 是否可访问；new/fork/import 也通过宿主维护一致生命周期。
导入采用复制到独占目标文件再切换，不把原文件直接当成当前输出。
依据：[src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L196)、[src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L226)、[src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L262)、[test/agent-session-runtime-events.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/agent-session-runtime-events.test.ts#L183)。

## 6. AgentSession：输入、事件与结束语义

### 6.1 一次 prompt 的前置检查

`prompt()` 不是直接调用 `agent.prompt()`：它先识别扩展命令，在运行中的会话也允许立即处理命令；拒绝不允许的压缩并发；派发 input hooks；展开 skill 和模板；处理运行中 steer/follow-up；确认模型与认证；必要时压缩；再派发 before_agent_start。
随后进行图片归一化、工具负载与系统提示变化处理、加入待发送消息，才报告 preflight `started` 并启动 Agent。
`streamingBehavior` 必须明确指定运行中输入如何排队，避免默认把第二次 prompt 当成新的并发模型循环。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

`handled`、`queued`、`started` 描述输入的接收结果，不能解释为整次任务已经完成。
前置检查之后的 `_runAgentPrompt()` 还需处理重试、自动压缩后的续跑、后续队列以及 `agent_before_settle` 请求的继续执行。
最终 `agent_settled` 才表达该会话运行边界已经稳定；`agent_end` 可以带 `willRetry:true`。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

### 6.2 事件、持久化和队列

AgentSession 订阅底层 Agent 事件，再增加队列变化、条目追加、压缩开始/结束、重试、shell 输出、名称和思考等级等产品事件。
在内部事件处理路径中，扩展监听器先于公开 session listeners，消息完成随后写入 SessionManager；这给 hooks 和呈现层明确的顺序，但消费者不能把任何事件都当成“已经 durable 写入”的确认。
`turn_end` 后才追加某些仅上下文 custom 消息，避免插入 assistant tool calls 与对应 tool results 之间。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

steering 与 follow-up 是分别管理的队列，可按设置选择一次取一个或全部。
显示队列在 user message_start 时同步出队；queue_update 供 UI/RPC更新；清空队列、abort、waitForIdle 各有独立职责。
abort 传播到当前循环与有关任务，waitForIdle 用来等待运行落稳，不能仅观察某次 provider stream 已结束。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

### 6.3 嵌套工具调用仍经过统一管线

AgentSession 安装 `beforeToolCall/afterToolCall`，而非让每个 wrapper 各自重复拦截。
前置 tool_call hook 可以阻止执行；hook 抛错也阻止执行。后置 tool_result 可修改内容、结构化结果、usage、错误状态，之后再归一化图片。
`ctx.executeTool()` 通过 callable tools 集合和 Agent 的 `runToolCall` 执行，增加 parentToolCallId，继续应用相同 hooks 与取消信号。
因此 codemode 内的子调用不是绕开工具校验的私有旁路。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

## 7. JSONL 会话树、恢复与分支

### 7.1 文件格式表达的不只是聊天消息

SessionHeader 记录 version、id、timestamp、cwd 和可选 parentSession；v3 条目具有 id、parentId、timestamp。
每条记录一行 JSON，但其逻辑结构是树，不是简单数组聊天记录。
当前 leaf 向 parentId 回溯构成当前分支；追加顺序和当前上下文顺序需要区分。
依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L41)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L57)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L390)。

| 条目类别 | 内容与用途 |
|---|---|
| message | user/assistant/toolResult/system 等实际消息 |
| model_change / thinking_level_change | 可恢复的模型选择与思考状态 |
| usage | 非普通消息路径的使用量记录 |
| compaction | 摘要、firstKeptEntryId、压缩前 tokens、details、usage、system checkpoint |
| branch_summary | 从旧分支带回的摘要及 fromId |
| custom | 扩展私有状态；本身不进入模型上下文 |
| custom_message | 可展示且可转成模型上下文的扩展消息 |
| label / session_info | 标签、名称等会话树元数据 |
| context_edit | 对目标条目的投影替换或省略，不破坏原记录 |

字段定义依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L64)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L91)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L128)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L175)。

### 7.2 恢复是投影计算

`buildSessionProjection()` 先取分支，再结合最近 compaction 和 context_edit 计算模型应看到的条目，保留 sourceEntry 关联。
最近一次 compaction 提供摘要和系统提示 checkpoint；保留区间从 firstKeptEntryId 起，接上压缩之后的条目。
同一目标的后续 context edit 覆盖先前 edit；替换为 null 表示从模型上下文省略，原历史仍可用于树与审计。
模型和思考等级从完整分支恢复，不因截掉旧上下文而丢失。
依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L476)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L543)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L576)。

`messages.ts` 扩展 AgentMessage 的 custom、bashExecution、branchSummary、compactionSummary 形态。
传给 Provider 时，shell 执行转成 user 文本，摘要也转成带提示边界的 user 消息；custom 的显示元数据不直接成为 Provider 字段。
用户以 `!!` 执行的 shell 消息可记录在会话但排除出模型上下文。
依据：[src/core/messages.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/messages.ts#L29)、[src/core/messages.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/messages.ts#L148)。

### 7.3 文件写入、容错与恢复限制

新 SessionManager 可以先只有内存头部和设置条目；出现第一条 user 或 assistant 后才创建文件，并一次刷出之前积累的条目。
初次创建使用排他写入，后续追加 JSONL；“第一条用户输入已经持久化”不需要等模型成功返回。
载入时按块扫描并处理不完整尾行；解析会跳过非法 JSON 行，迁移可以重写文件。
解析入口不是对每个字段执行完整 schema 校验，后续代码也对部分旧/异常内容做归一化。因此这里提供的是可恢复日志策略，不能推断具有数据库式事务和完整恶意输入验证。
依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L355)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L439)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L627)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L1166)。

### 7.4 同文件导航与新文件分叉

`branch()` 只移动 leaf；后续 append 的 parentId 指向新位置，使原路径继续保留在同一文件。
`createBranchedSession()` 则提取到目标 leaf 的路径，重建 parent 链和标签，调整压缩引用，生成新 session ID 与 parentSession，并按同样的首次对话规则落盘。
这解释了“回到某处继续”与“复制成另一个会话”的不同所有权。
依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L1579)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L1632)。

`AgentSession.navigateTree()` 拒绝运行或压缩中的导航，计算旧 leaf 与目标的公共祖先，向 before_tree hook 提供取消/摘要指令/标签的控制机会。
选择用户或 custom 消息时可回到其父节点，把文本交还编辑器；选择其他条目则保留该节点。
需要摘要时，在新位置追加 branch_summary，然后刷新模型上下文、模型、思考等级与工具状态。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/core/compaction/branch-summarization.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/branch-summarization.ts#L108)。

## 8. 压缩：预算、切点与重试

默认 compaction 启用，reserveTokens 为 16,384，keepRecentTokens 为 20,000；阈值根据当前模型上下文窗口计算。
token 估计优先利用有效 assistant usage，再估计未被 usage 覆盖的后缀；error、aborted、全零 usage 不应成为可靠锚点。
字符估算与图片估值是启发式，不是对任一 Provider tokenizer 的等价复现。
依据：[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L126)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L140)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L196)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L267)。

切点算法保护 assistant tool calls 与 tool results 的结构，必要时对长 turn 分出 prefix，而不是任意截断消息数组。
`prepareCompaction()` 使用已经应用 context edits 的 canonical projection；默认摘要路径合并之前摘要、旧片段、拆分 turn 的前缀与文件操作记录。
总结请求本身通过当前会话的模型/认证与专门 summary 路由运行，虚拟模型也有 summary 用途的解析。
依据：[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L446)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L872)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L965)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

手工 `compact()` 先 await abort，然后建立独立取消控制器，派发 compaction start 和 before_compact，允许扩展取消或提供结果。
成功后追加 compaction entry 并重建上下文；清除“正在压缩”状态再发结束事件，避免结束监听器看到陈旧 busy 状态。
失败也发送结束状态；摘要不成功不能当成已提交的上下文替换。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

自动压缩区分超出上下文造成的错误、成功完成但过阈值、prompt 前检测等原因。
overflow 路径可压缩后重试；正常阈值压缩不重放已经完成的回答。
代码还排除被取消的运行、已被较新 compaction 覆盖的旧 usage、不同模型上下文中的不适用估计，防止凭旧计数反复压缩。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

## 9. 模型、认证与请求控制

### 9.1 ModelRuntime 是规范运行时

ModelRuntime 合成内置或 native Provider、models 配置、扩展覆盖、远程目录和虚拟模型，提供全部/可用模型与认证状态快照。
创建默认禁止模型目录网络请求，refresh 显式决定是否允许；offline 又限制自动网络发现。
`ModelRegistry` 保留同步外观以服务现有扩展 API，但内部权威数据源是 ModelRuntime。
依据：[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L107)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L215)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L300)、[src/core/model-registry.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-registry.ts#L42)。

虚拟模型是有名字的路由选择，resolver 可根据用途、之前模型、失败情况和状态选择物理模型。
结果必须是已注册物理模型且认证已配置；思考等级按实际模型能力限制。
跨 Provider 路由时丢弃不能安全继承的 apiKey、headers、env 覆盖，防止把原 Provider 的敏感参数直接带到另一个 Provider。
依据：[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L715)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L955)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L994)。

### 9.2 认证提交与模型同步

认证存储有文件与内存实现；文件修改在 proper-lockfile 保护下完成，创建目录/文件采用受限权限，reload 失败时保留最近有效快照。
同一 Provider 的认证变更按队列串行，不同 Provider 可并行。
认证写入成功而后续目录/可用性同步失败有专门 `CredentialSynchronizationError`，保留“提交已发生”的事实，不能误导调用方重新执行认证写入。
依据：[src/core/auth-storage.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/auth-storage.ts#L41)、[src/core/auth-storage.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/auth-storage.ts#L347)、[src/core/auth-storage.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/auth-storage.ts#L377)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L135)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L571)。

认证/请求配置中的字符串可以解析环境变量，也支持 `!command` 执行 shell 取得值；普通路径有进程级缓存，严格路径在适用场景中抛错。
该能力属于本地配置执行，不是纯字符串替换；MCP headers/env 同样会触及配置求值边界。
依据：[src/core/resolve-config-value.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resolve-config-value.ts#L159)。

交互登录完成后还要区分“凭据已保存”与“模型已发现”。若之前是 unknown 模型、Provider 有默认模型规则但当前快照还没有该默认模型，界面先确认保存，再发起带 15 秒取消信号的后台 refresh；完成后尝试选默认模型，Radius 还允许退到该账户目录的首个模型。仍没有模型、默认模型缺失或选模失败会分别提示。回调同时检查 session 和 previousModel 身份，避免覆盖用户刷新期间切换的会话/模型。这是沿用 1.0.2 审读的时序；没有对应历史正文 diff，不把全部细节都认定为本次首次加入。[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)、[test/suite/regressions/7027-credential-refresh-hang.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/7027-credential-refresh-hang.test.ts#L127)。

当前 token 刷新与请求取消分开：pi-ai 的 refreshStoredOAuthCredential 在锁内重查凭据，请求取消只中止等待锁；刷新一旦开始，改收独立的 15 秒 timeout signal，成功后在释放锁前保存，防止服务端已轮换 refresh token 而丢弃新值。及时结束依赖刷新实现响应 signal，保存流程也没有统一的 15 秒硬上限。本包旧式 extension OAuth 也经过该链，测试检查请求取消后 refresh 的信号仍未取消。这不表示整个模型请求失去取消能力。[刷新事务](../../../../.repo/pi-1.0.4/packages/ai/src/auth/resolve.ts#L119)、[扩展适配](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/provider-composer.ts#L369)、[静态断言](../../../../.repo/pi-1.0.4/packages/coding-agent/test/model-runtime-auth-options.test.ts#L266)。

### 9.3 Provider hooks 与图片处理

SDK 在请求上组合用户/模型配置、归因 headers 和扩展 hooks，包括 before_provider_headers、before_provider_request、after_provider_response、provider_stream_event。
图片输入在会话和工具结果边界按模型限制归一化；禁止图片的设置还在 LLM 转换处过滤，避免仅依赖 UI 路径。
cache warmer 在 settled 时观察会话，但不应把它与用户显式 prompt 的完成信号混同。
依据：[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

### 9.4 按有效思考等级采样与模型选择

1.0.2 的 `samplingParamsByThinkingLevel` 不是 UI 设置别名。`model-config.ts` 在自定义 model 和 `modelOverrides` 两种 schema 中接受它，键是 Pi 的 `off/minimal/low/medium/high/xhigh/max`，每个值是自由键值记录。Provider composer 将字段带入模型；覆盖已有模型时，对每个等级的参数按键合并，保留未覆盖等级和键，而不是整体替换等级表。[src/core/model-config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-config.ts#L66)、[src/core/model-config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-config.ts#L213)、[src/core/model-config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-config.ts#L237)、[src/core/provider-composer.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/provider-composer.ts#L164)、[src/core/provider-composer.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/provider-composer.ts#L244)。

运行链继续进入 pi-ai 的 `resolveSamplingParams()`：先按模型支持范围 clamp 思考等级，再取该有效等级的覆盖，并依次展开 **模型 samplingParams → 等级覆盖 → 请求 samplingParams**，后者逐键胜出。等级键不是 `thinkingLevelMap` 映射后的 Provider 字符串。OpenAI completions、OpenAI responses、Azure OpenAI responses 在请求组装最后应用这些参数，因此也可覆盖同名的显式请求字段；其它 API 不消费这组自由参数。配置被接受不保证远端接受任意参数，尤其不能把它当成通用于 Anthropic/Google 的温度控制。[docs/models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md#L101)、[pi-ai simple-options.ts](../../../../.repo/pi-1.0.4/packages/ai/src/api/simple-options.ts#L24)、[openai-completions.ts](../../../../.repo/pi-1.0.4/packages/ai/src/api/openai-completions.ts#L1003)、[openai-responses.ts](../../../../.repo/pi-1.0.4/packages/ai/src/api/openai-responses.ts#L382)、[azure-openai-responses.ts](../../../../.repo/pi-1.0.4/packages/ai/src/api/azure-openai-responses.ts)。

本包测试通过自定义模型、已有模型 override 断言参数保留与逐等级合并；实际 payload、clamp 与命名字段优先级由 pi-ai 的 sampling 测试表达。这里仅核对源码，不声称已请求远端端点。[test/model-registry.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/model-registry.test.ts#L765)、[pi-ai test/sampling-options.test.ts](../../../../.repo/pi-1.0.4/packages/ai/test/sampling-options.test.ts)。

模型选择另有两项 1.0.1 明示变化：`--models` 分割后 trim 并过滤空项，尾逗号不再成为额外 pattern；NVIDIA 的默认 ID 是 `nvidia/nemotron-3-ultra-550b-a55b`。它们分别影响 cycle scope 和初始默认选择，不改变 JSONL 的模型恢复优先级。[src/cli/args.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/cli/args.ts)、[test/args.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/args.test.ts)、[src/core/model-resolver.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-resolver.ts)。

Cloudflare Clef / Clef Flash 以 classifier model 加入可用能力：`@cf/cloudflare/clef` 和 `@cf/cloudflare/clef-flash` 通过 codemode 的 `models.classify()` 或扩展的 `ctx.modelRegistry.classify()` 调用，不出现在聊天 `/model` picker。它们复用既有分类及 usage 汇总路径，文档中的新增型号不是新的会话模型类型或 roadmap。[docs/models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md#L137)、[docs/models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md#L143)、[docs/models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md#L164)、[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)。

### 9.5 Azure 身份与当前默认模型

1.0.3 将 provider 从 azure-openai-responses 改名为 azure。发布段要求同步 auth.json、models.json、settings 的 provider 键及 patterns，或重新登录；旧 provider 会话恢复退到其它模型且不复用其 prompt cache。当前默认表键为 azure，ID 为 gpt-5.4，AZURE_OPENAI_* 变量不改名。azure-openai-responses 仍是 Responses 的 API ID，Foundry Chat Completions 由同一 provider 的另一 API 分派承担。[迁移说明](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md#L26)、[当前默认表](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-resolver.ts#L20)、[AI 报告](ai.md)。

当前默认表还包含 anthropic/claude-opus-4-8、openai/gpt-5.5、openai-codex/gpt-6.1-sol 和 NVIDIA/nvidia/nemotron-3-ultra-550b-a55b。这是静态表，不是每次启动的结果：显式选择、恢复、已保存配置及认证可用性仍影响选择。eval 示例的 gpt-5.6-sol 是显式示例，不是默认承诺。[findInitialModel](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-resolver.ts#L631)。

## 10. 设置、包、上下文与 skills

### 10.1 设置合并

SettingsManager 合并全局与项目层设置，对对象做深合并、对数组通常替换；defaultTools 另有只含 `+name/-name` 时增减继承清单的规则。
保存只更新脏字段，配合文件存储合并，减少不同操作互相覆盖；解析错误通过诊断暴露并保留适当的已有状态。
未信任项目不能写项目设置，但读写规则由调用方是否正确传入 projectTrusted 决定。
依据：[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L195)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L215)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L442)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L662)。

### 10.2 资源重新加载是装配动作

DefaultResourceLoader 汇总 extensions、skills、prompt templates、themes、context files、system/append prompt。
它支持全局/项目/命令行资源、包解析、过滤及覆盖函数；reload 清理工厂缓存并重新完成发现和加载。
pretrust 阶段只加载可用于信任决策的全局/CLI 扩展，信任决定后重新加载设置和项目资源。
noExtensions/noSkills 等开关与显式 CLI 路径有分别处理，不能简单解释成“把所有资源源头全部清空”。
依据：[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)、[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)、[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)。

PackageManager 处理 npm、Git 和本地来源、包资源 manifest、作用域与重复资源优先级。
它负责把“安装了什么”转换成可供 ResourceLoader 使用的路径；真正 extension factory 的执行仍归扩展加载器。
本次重点核查资源解析与信任接线，未逐分支审计所有包下载、安装、卸载和平台错误处理。
依据：[src/core/package-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/package-manager.ts#L114)、[src/core/package-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/package-manager.ts#L156)、[src/core/package-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/package-manager.ts#L563)、[src/core/package-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/package-manager.ts#L812)。

### 10.3 上下文文件与系统提示

每个目录按 `AGENTS.override.md → AGENTS.md → AGENTS.MD → CLAUDE.md → CLAUDE.MD` 选择首个可用上下文文件。
先加载用户 agentDir 的上下文，再从文件系统祖先到 cwd 继承；嵌套 linked worktree 会处理主工作树同一逻辑位置的遮蔽，避免重复应用。
system prompt 以分节状态构造，并能计算后续变化，把工具和系统提示状态变化纳入可恢复的上下文，而不是只保存在一块可变字符串里。
依据：[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)、[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)、[src/core/system-prompt.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/system-prompt.ts)、[src/core/system-prompt.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/system-prompt.ts)。

buildSystemPromptSections 以 selectedTools 减 hiddenTools 形成声明集，再生成工具列表与 rules。read/bash 被隐藏但仍在 selectedTools 时，skills 提示使用 indirect 不点名 reader；两者都不在 selectedTools 时不加入 skills 节。隐藏工具规则交编排工具呈现，分节 diff/持久化机制仍保留。[提示构造](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/system-prompt.ts#L128)、[skills 提示](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts#L358)。

### 10.4 Skills 与 prompt templates

Skill 是目录中的 SKILL.md/frontmatter 资源：名称、描述和 disable-model-invocation 等元数据先用于发现与提示。
模型可见列表过滤禁用模型调用的 skill；显式 `/skill:name` 展开才读取正文、剥离 frontmatter，并附带名称和位置，说明引用路径相对于该 skill。
它是渐进加载的指令内容，不是具备独立权限的代码插件。
模板则从 Markdown 解析、根据命令名和参数替换占位符；参数解析考虑引号，最终仍进入正常 prompt 管线。
依据：[src/core/skills.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts)、[src/core/skills.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/core/prompt-templates.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/prompt-templates.ts#L25)、[src/core/prompt-templates.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/prompt-templates.ts#L304)。

## 11. 内置工具：输入、输出和执行边界

### 11.1 共同契约

工具全集是 read、bash、powershell、edit、write、grep、find、ls；默认仅激活前述四个编码工具。
工具定义以 TypeBox schema 描述参数，可提供 prepareArguments、outputSchema、执行模式和渲染元数据，再适配成 AgentTool。
运行上下文的 cwd 优先于工厂创建时 cwd；路径允许绝对路径，resolve 行为不构成目录 confinement。
部分工具暴露 Operations 接口，允许替换底层操作，但各工具可替换范围不同，不能假设存在统一远程文件系统抽象。
依据：[src/core/tools/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/index.ts#L114)、[src/core/tools/tool-definition-wrapper.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/tool-definition-wrapper.ts#L8)、[test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)。

| 工具 | 主要输入 | 模型可见输出 | 关键限制/语义 |
|---|---|---|---|
| read | path、offset、limit | 文本或图片 content | offset 从 1 开始；文本取头，默认 2,000 行/50 KiB |
| bash | command、timeout 秒 | 输出尾部；另有结构化 output/exit_code/wall_time_seconds | 无默认命令超时；取消/超时终止进程树 |
| powershell | command、timeout 秒 | 与 bash 相同契约 | PowerShell 启动与 UTF-8 前置配置 |
| edit | path、edits 数组 | 成功提示及 diff/patch/firstChangedLine details | 多个编辑对同一原文匹配，必须唯一且互不重叠 |
| write | path、content | 成功写入提示 | 创建父目录并覆盖文件；没有回滚事务 |
| grep | pattern、path、glob、ignoreCase、literal、context、limit | 匹配及上下文文本 | 默认 100 matches；每行 500 字符；总输出 50 KiB |
| find | pattern、path、limit | 相对路径清单 | 默认 1,000 个；遵循 fd/Git 忽略策略；50 KiB |
| ls | path、limit | 排序目录项，目录带 `/` | 默认 500 项，含点文件；50 KiB |

通用截断数值定义在 [src/core/tools/truncate.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/truncate.ts#L11)；具体工具可以关闭行限制或增加各自限制，不能把这张表简化成“所有结果都统一截前 2,000 行”。

### 11.2 read：文本窗口与图片

read 首先检查可访问性并探测内容类型；图片识别不只依赖扩展名，也支持内容 magic。
图片走格式处理和按模型限制缩放，输出 ImageContent 与必要说明；无图像能力的模型情形会返回解释信息。
文本实现读取完整文件后按行选取 offset/limit，再做头部截断，报告后续 offset；超过文件范围是显式错误。
第一行本身超过字节限额时提供特殊说明，避免返回看似完整但实际上无法定位的空窗口。
依据：[src/core/tools/read.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/read.ts)、[src/core/tools/read.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/read.ts)。

取消监听可及时拒绝工具 Promise，但本地文件读取并非所有步骤都具有底层 I/O 中断能力。
观察：输出窗口有界不等于读取内存有界；超大文本文件仍要先完整读取。这是静态实现特征，未进行内存基准测量。
测试验证 offset/limit、无扩展名图片、BMP 等行为，见 [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)、[test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)。

当前 read 带 outputSchema：文本为字符串，图片为 {type:"image", data, mimeType, note}，structuredContent 从处理后同一 content 生成，包含缩放后的图像和提示。codemode 可将 tools.read 返回值交给 image()，不再只能获得图片旁文字；模型可见 content、脚本返回值和模型本身图像能力仍是不同层。[readOutputSchema/toReadOutput](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/read.ts#L27)、[静态会话断言](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-codemode.test.ts#L524)。

### 11.3 edit：一次性验证原文上的多个变更

正式输入为 `edits:[{oldText,newText}, ...]`。
prepareArguments 兼容 JSON 字符串形式 edits、单个对象和旧 oldText/newText 字段，再执行实际参数验证。
读取文件后剥离 BOM、将 CRLF 归一化为 LF；匹配优先 exact，再用包含尾部空白、Unicode NFKC/标点归一化的 fuzzy 规则。
每个 oldText 必须唯一，所有修改范围不能重叠；全部匹配和冲突检查完成后才写入，保持未修改行内容，恢复原换行及 BOM。
依据：[src/core/tools/edit.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit.ts#L113)、[src/core/tools/edit.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit.ts#L145)、[src/core/tools/edit.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit.ts#L174)、[src/core/tools/edit-diff.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit-diff.ts#L34)、[src/core/tools/edit-diff.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit-diff.ts#L300)。

输出 details 同时含面向呈现的 diff、标准 unified patch 和首个变更行。
“验证失败不会只应用前几个 edits”有测试断言；这不意味着外部程序同时修改文件时具备跨进程事务隔离。
依据：[src/core/tools/edit-diff.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/edit-diff.ts#L365)、[test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)、[test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)。

### 11.4 write 与文件变更队列

write 将 path 解析为目标文件，递归创建父目录，再执行 UTF-8 覆盖写入。
edit/write 共用 `withFileMutationQueue()`：已有文件按 realpath 归一化键，缺失文件按解析后的路径键；同一目标包括符号链接别名串行，不同文件可并行。
键注册本身经过队列，避免获取 key 的异步步骤打乱先后顺序。
依据：[src/core/tools/write.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/write.ts#L39)、[src/core/tools/file-mutation-queue.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/file-mutation-queue.ts#L29)。

取消只在关键 await 边界检查；若底层写入已开始，调用可以最终报告取消但文件已经发生变化。
队列直到实际操作 settle 才释放，防止后一个写操作越过仍在执行的前一个写操作。
这是进程内互斥和取消顺序保证，不是临时文件+rename 的原子替换，也不是操作系统文件锁。
依据：[src/core/tools/write.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/write.ts#L39)、[test/file-mutation-queue.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/file-mutation-queue.test.ts#L77)、[test/file-mutation-queue.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/file-mutation-queue.test.ts#L176)。

### 11.5 bash / powershell：进程、取消和双重输出

shell 输入 timeout 可省略，但若给出必须是有限正数，转换后的毫秒不能超过计时器上限。
本地实现检查 cwd，spawn shell，组合 stdout/stderr；非 Windows 使用 detached，Windows 隐藏子进程窗口。
abort 和 timeout 调用进程树终止，退出等待还处理“主进程已退出但继承 pipe 的后代未释放句柄”的情况。
signal 结束会映射退出码；非零退出返回 isError 及结构化结果，取消/超时走带已收集输出的错误路径。
依据：[src/core/tools/bash.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/bash.ts#L22)、[src/core/tools/bash.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/bash.ts#L99)、[src/core/tools/bash.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/bash.ts#L247)。

spawn 环境先移除继承来的会话标识字段，再按当前上下文注入 `PI_SESSION_ID/FILE/PROVIDER/MODEL/REASONING` 等，防止父 shell 的过期值误代表新会话。
可替换 spawn hook 可以调整 command、cwd、env；用户 `!` 命令另有 user_bash hook，但也运行在本地权限下。
PowerShell 复用同一执行/输出实现，仅负责 shell 配置和 UTF-8 编码前置内容。
依据：[src/core/tools/bash.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/bash.ts#L190)、[src/core/tools/powershell.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/powershell.ts#L30)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

输出有两层：给模型阅读的文本保留末尾 2,000 行/50 KiB；结构化 output 允许最多 1 MiB，并在超出时保留头尾。
OutputAccumulator 流式解码 UTF-8，保留有界内存尾部，超过限制时将完整原始输出写入临时文件；最终结果给出完整文件位置。
stream 更新受节流，finalize 之后停止接纳迟到 chunks，避免工具结束后结果继续变动。
结构化错误结果使 codemode 能检查 exit_code，而不必把所有非零退出都变成无法处理的脚本异常。
依据：[src/core/tools/bash.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/bash.ts#L24)、[src/core/tools/output-accumulator.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/output-accumulator.ts)、[src/core/tools/output-accumulator.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/output-accumulator.ts)、[test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts)。

截断全文、MCP 二进制资源及 codemode 图片共用 output-files.ts：系统 temp 目录、8 字节随机后缀、wx 独占创建、mode 0600。调用方包括 OutputAccumulator、用户 bash executor、MCP tools 和 codemode execute，不只图片。mode 表达 POSIX 用户权限，不能替代 Windows ACL 或证明平台已实测；这些文件留供后续回合使用，未建立随会话删除的自动清理契约。[输出工厂](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/output-files.ts#L14)、[累积器](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/output-accumulator.ts#L249)、[用户 shell](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/bash-executor.ts#L66)。

### 11.6 grep / find / ls：有界发现

grep 默认调用外部 rg，使用 JSON 输出、行号、hidden、禁色等参数，并用 `--` 隔开用户 pattern 与命令选项。
达到 match limit 后终止进程；退出码 1 按无匹配处理；context 行从文件读取并缓存，每行另做字符截断。
其 Operations 只替换 stat/read 等文件操作，不把搜索进程整体抽象掉；abort 会终止子进程。
依据：[src/core/tools/grep.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/grep.ts#L21)、[src/core/tools/grep.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/grep.ts#L126)。

find 默认调用 fd，处理包含目录分隔符的 glob、平台路径分隔符、hidden、Git 忽略和非 Git 目录行为，限制输出个数并返回相对路径。
如果调用方提供 glob 操作实现，可绕过默认 fd；因此 find 的替换范围比 grep 更广。
ls 使用目录读取和 stat，包含点文件，按不区分大小写排序；无法 stat 的条目跳过，目录追加斜杠。
ls/read 的 Promise 可取消不等于所有已发出的文件系统异步操作立即停止。
依据：[src/core/tools/find.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/find.ts#L27)、[src/core/tools/ls.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/ls.ts#L12)。

若机器缺少 rg/fd，`ensureTool()` 可以发现系统工具或下载工具；offline 时跳过下载并返回诊断。
这意味着“执行一个本地搜索工具”可能先触发工具安装网络路径；本次未执行该路径，也没有验证远端版本或下载内容。
依据：[src/utils/tools-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/tools-manager.ts#L258)、[src/utils/tools-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/tools-manager.ts#L349)。

## 12. 扩展与 hooks

### 12.1 加载和生命周期

稳定扩展是接受 ExtensionAPI 的工厂，可注册工具、命令、快捷键、flags、Provider、虚拟模型、MCP servers 和事件处理器。
加载器延迟创建 Jiti，在源码、npm 和 binary 环境使用相应 aliases/virtual modules；工厂缓存与每次 session 的运行时实例分开。
工厂执行成功才提交收集到的注册；抛错则丢弃该轮注册并报告错误，但任意工厂已经产生的外部副作用无法由注册回滚机制撤销。
依据：[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts)。

ExtensionRuntime 在绑定 session 前提供明确不可用的动作；失效后阻止旧闭包继续操作已替换会话，并清理事件总线订阅。
`reload()` 会重新解析资源、Provider 与工具，旧上下文失效，随后重新绑定。
默认内置 llama.cpp 以及可替换的 codemode/tool-search/MCP 都通过同一扩展面进入 CLI。
如果第三方已注册相应工具或命令，可替换内置扩展跳过加载，以避免同名双重运行。
依据：[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/extensions/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/index.ts#L7)、[src/core/resource-loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts)。

### 12.2 工具的声明、可调用性和暴露策略

本版本将“已注册”“可供嵌套调用”“在模型请求里声明”“当前 active”分离，而非只有一个 enabled 标志。

| exposure | 对模型直接声明 | codemode/嵌套调用 |
|---|---|---|
| direct | active 时声明 | active 时可调用 |
| model-only | active 时声明 | 不可调用，避免例如 codemode 自递归 |
| codemode | active 时可声明；可在 codemode 说明中列出 | 注册后可调用，不要求 active |
| deferred | active 时可声明；初始不内联目录 | 注册后可调用，可通过发现工具找到 |
| hidden | 不声明 | 不可调用 |

`prepareLoadout` 可以只改变 wire 上的模型声明，保留会话内部的 active 状态以便恢复。
工具 annotations 是作者提供的描述性 hint，不能当成真实只读或安全保证；namespace 则帮助目录组织、搜索和脚本名称映射。
依据：[src/core/extensions/types.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/types.ts)。

选择在曝光表之外还有两道 gate：_isAllowedTool 决定进入 registry，_isActivatable 决定直接声明。非空 allowlist 不含 mcp__ 前缀项时，未匹配 MCP 工具仍注册；codemode/deferred 可供脚本调用，只有注册了 tool_search 才能将这种未匹配工具激活，direct 工具不会因此声明。恢复与扩展 setActiveTools 同样受限。一旦 allowlist 含 mcp__ 项，就同时过滤 MCP；exclude 总优先。资源三工具也算 MCP，但 mcp__* 字面 pattern 不匹配 list_mcp_resources 等名称；筛选服务器不会自动把公共资源工具的访问范围缩到该服务器。[两道选择](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts#L1519)、[MCP 名称](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/mcp-servers.ts#L27)、[静态边界断言](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts#L231)。

ToolLoadout 新增 getPromptGuidelines(name)，codemode 把规则合入 inline 声明、describeTool 和 ALL_TOOLS；零 inlineBudget 仍可由发现接口取得。hiddenDeclarations 改变请求声明及相应默认提示，保留内部 active/callable 状态，不是撤销权限。[API](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/types.ts#L549)、[loadout](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts#L342)、[工厂接线](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/index.ts#L38)。

### 12.3 不同 hooks 有不同失败政策

tool_call 按注册顺序执行，首个 block 即停止；异常沿执行路径传播并阻止工具运行。
tool_result 是结果补丁链，一个 handler 失败会报告，但其他 handlers 仍有机会处理；仅修改 content 而未提供对应 structuredContent 时移除旧结构化数据，避免两份结果矛盾。
user_bash hook 的异常也采用阻止 shell 执行的策略。
依据：[src/core/extensions/runner.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/runner.ts)。

普通 context hook 收到剥离 system 的消息副本，处理后恢复系统消息；context_with_system 才获得完整请求上下文控制，若移除 system 头会有提示。
边界 hooks 使用 draft/proposal/preview 方式串联，最后由 session 提交，而非让每个 hook 任意即时变更 Agent 状态。
这些是扩展一致性设计，不是对恶意同进程代码的安全隔离。
依据：[src/core/extensions/runner.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/runner.ts)。

### 12.4 Renderer resolver 与执行注册分离

`pi.registerToolRenderer()` 增加的是呈现解析链：`ToolRenderers` 只取 `renderShell/renderCall/renderResult`，不要求参数 schema 或 execute。loader 按扩展收集 resolver，runner 按加载顺序调用；resolver 的 `next()` 才继续后续解析器，最终落到调用方提供的基础工具定义。`next() ?? mine` 是补缺，直接返回自己的对象则可覆盖；这里没有自动逐字段合并所有 resolver 的规则。类型从 extensions 和 SDK 根入口导出。[src/core/extensions/types.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/types.ts)、[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts#L367)、[src/core/extensions/runner.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/runner.ts#L789)、[src/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/index.ts#L182)。

内置 MCP 在工厂加载时注册 `mcp__…__…` fallback，连接和工具注册尚未发生也能为历史调用生成 server/tool 标签及折叠结果。InteractiveMode 将 resolver 套在工具/内置 renderer 的基础回退外；AgentSession 的 HTML export 同样通过 resolver。该能力使 resume/replay 和导出不必等待服务器重新连接，也不因此让已撤回工具重新可执行。回归测试专门在没有服务器配置、工具未注册时检查 renderCall 与 HTML 导出。[src/extensions/mcp/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/index.ts#L362)、[src/extensions/mcp/tools.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts)、[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[test/suite/regressions/10285-mcp-tool-renderers.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/10285-mcp-tool-renderers.test.ts#L14)。

## 13. Codemode 与工具发现

### 13.1 从模型调用到程序化组合

codemode 接收 `code:string`，支持顶层 await/return 及可选源首部 options；外部模型声明可使用 raw JavaScript grammar。
默认扩展注册 codemode 时 `defaultActive:false`，是否实际进入工具负载由设置/MCP 自动激活等策略决定。
on 模式保留直接工具；only 模式通过 prepareLoadout 隐藏直接工具的 wire 声明，使模型主要从脚本桥接调用，同时保留会话 active 清单。
依据：[src/extensions/codemode/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/index.ts)、[src/extensions/codemode/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts)、[src/extensions/codemode/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts)。

可调用目录排除 codemode 自己；有限说明预算按 namespace 轮询，各组先选择成本低的工具，避免一个大 namespace 占满提示。
deferred 工具不进入内联目录，使 MCP 在后台连接或改变工具列表时不会无谓改变 codemode 的整体描述。
脚本中的发现能力和 tool_search 允许按需找到其他工具。
依据：[src/extensions/codemode/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts)、[src/extensions/codemode/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts)、[src/extensions/codemode/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts)。

### 13.2 沙箱和返回值边界

执行模块按需加载 pi-codemode，使用 QuickJS WASM/worker；脚本没有直接 Node、文件系统、网络和 timer 接口，只获得注册的桥接 API。
VM 堆限制为 256 MiB；单脚本 classify/generateImages 的模型调用并发最多 4 个，超出的调用排队。
脚本 timeout 未设置时执行层传 Infinity，因此存在取消机制和内存限额不意味着所有脚本自动有固定时长上限。
1.0.1 起下游 pi-codemode prelude 另限制累计输出 **16,777,216 个字符**（文本和图片 base64）或 **100,000 个输出条目**；空字符串仍消耗条目额度。超过任一上限先将脚本标记失败再抛错，捕获异常不能恢复输出。这个限额在输出进入宿主累积前生效，不是 `max_output_tokens` 的估算截断，也不是 256 MiB VM 堆限制。[pi-codemode prelude-source.ts](../../../../.repo/pi-1.0.4/packages/codemode/src/runtime/prelude-source.ts)、[prelude-source.ts](../../../../.repo/pi-1.0.4/packages/codemode/src/runtime/prelude-source.ts)、[docs/codemode.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/codemode.md)。
依据：[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)、[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)、[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)。

子调用通过 `ctx.executeTool()`，收集状态、调用 ID、usage，遵循 hooks 与执行模式。
若工具有 outputSchema 且提供 structuredContent，脚本直接获得该结构化值，包括可供检查的 isError 结果；没有该结构化契约时，普通错误结果转换为脚本异常。
脚本失败可保留已经产生的显示输出，但不会回滚已经完成的 shell、文件或外部服务副作用。
依据：[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)、[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)。

store 从当前会话分支的 custom 条目恢复；只有成功脚本才提交 `codemode-store` 更新，失败脚本丢弃暂存写入。
脚本结束总会关闭沙箱，把未结束调用标为取消；结果经过输出截断，完整文本可写临时文件，并附带嵌套 usage。
这里的 store 成功提交与工具副作用是不同事务边界。
依据：[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)、[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)、[src/extensions/codemode/execute.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts)。

saveImages 在文本预算截断之后保存显示输出图片，将路径标签放在每张图片前，避免路径被截掉；同次结果中重复 base64 数据只写一次。写失败成为图片旁说明，仍保留图片和已执行工具结果；只有交 image() 的图片被显示/保存，generateImages 返回图却完全未显示时另加提示。[saveImages](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts#L281)、[截断/保存次序](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts#L458)。

models 按 provider/id 回查当前目录并用宿主认证，脚本其余模型字段不授权；分类/图片并发最多四个，usage 归工具结果，参数错误与 Provider error/aborted 结果分别处理。下游 codemode 还冻结 VM 内建对象/全局并检查 worker 消息，避免原型篡改破坏序列化；不把这些局部防护扩大为所有输入与宿主副作用的完整安全证明。[模型桥接](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts#L566)、[codemode](codemode.md)。

### 13.3 tool_search

tool_search 用 BM25 排序，文本包含工具名、说明、namespace、schema 属性等，分词处理 camelCase、英文停用词和简单单数化。
候选主要是尚未激活的 codemode/deferred 工具；命中后通过 setActiveTools 使其可直接声明，并由会话状态保存这个变化。
默认返回 8 个；拒绝空查询和不合法 limit。它是本地目录发现，不是调用外部搜索引擎。
依据：[src/extensions/tool-search/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/tool-search/tool.ts#L21)、[src/extensions/tool-search/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/tool-search/tool.ts#L76)、[src/extensions/tool-search/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/tool-search/tool.ts#L108)、[src/extensions/tool-search/tool.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/tool-search/tool.ts#L222)。

## 14. 内置 MCP 适配

### 14.1 配置、连接与工具负载

MCP 扩展装配全局 mcp.json、可信项目 mcp.json 和扩展注册；完整同名项目定义替换全局定义，文件配置又优先于扩展注册。
1.0.1 增加局部覆盖：项目条目不含 `command/url/type` 时，只能设置 `enabled/exposure/toolExposure`，必须已有同名全局服务器；含其它字段或没有全局基底时报告并跳过。实现以浅展开合并，故 `toolExposure` 若给出会替换整张表。这样可继承全局 env/headers/auth，同时禁止用 override 修改 command/URL 后挟带全局凭据；完整项目定义仍不能设置 `auth.provider`。未信任项目不读取 override。[src/extensions/mcp/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/config.ts#L74)、[src/extensions/mcp/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/config.ts#L108)、[src/extensions/mcp/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/config.ts#L145)。
`/mcp` 只在可信项目里给尚无 override 的全局服务器提供项目启停操作；保存后 entry 保留全局 source/scope 并另记 override 路径，之后曝光/启停修改写回该项目文件。override 中的 `enabled:true` 和 `exposure:codemode` 不像完整定义那样删除为默认，否则会意外继承全局值。CLI list 也输出 override 来源。[src/extensions/mcp/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/index.ts#L600)、[src/extensions/mcp/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/index.ts#L743)、[src/extensions/mcp/config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/config.ts#L169)、[src/extensions/mcp/cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/cli.ts#L91)。
配置名称、工具原名经过编码生成 `mcp__server__tool` 一类可用名字，冲突另加稳定区分信息。
连接在 session_start 后后台建立，session_shutdown 清理；generation 标识使旧连接晚到的工具列表不能污染新会话。
依据：[src/extensions/mcp/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/index.ts)、[src/extensions/mcp/tools.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts)。

MCP 的配置 exposure 是 codemode、deferred、direct、hidden 等策略；默认 codemode 映射到核心 deferred，刻意不把每个远程工具声明内联到脚本提示中。
codemode/deferred 策略可分别带动 codemode/tool_search 激活，direct 则直接向模型声明；每个工具可覆盖服务器默认。
首个 prompt 仅为 direct servers 进行有上限等待，默认 10 秒；脚本、搜索或资源调用再根据实际需要等待相关服务器，避免所有间接工具拖慢启动。
依据：[src/extensions/mcp/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/index.ts)、[src/extensions/mcp/tools.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts)、[test/suite/agent-session-mcp.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts)、[test/suite/agent-session-mcp.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts)。

--no-mcp 经 main 传 disabledBuiltinExtensions:["mcp"]，loader 合并配置与显式 -e builtin:mcp 后过滤该项，故二者不能覆盖本次禁用；第三方替代 MCP 扩展和独立 pi mcp 子命令是不同边界。--no-tools 只清空工具，并不禁止 MCP 连接。[CLI](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts#L785)、[loader](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts#L573)、[静态优先级断言](../../../../.repo/pi-1.0.4/packages/coding-agent/test/resource-loader.test.ts#L1113)。

### 14.2 Transport 与重试

runtime 通过 pi-mcp 创建 Streamable HTTP 或 stdio transport；headers、command args、cwd/env 经严格配置求值。
每个 ServerConnection 管理 connecting/connected/disconnected/needs-auth/failed/closed 状态，多个调用共享正在建立的 client。
默认请求 timeout 为 60 秒，建立连接有有限退避；OAuth 需求体现在连接状态与管理 UI。
依据：[src/extensions/mcp/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts)。

有副作用的 callTool 不对一般传输失败盲目重试；只读 list/read 路径可在特定瞬时 HTTP 错误后重试一次。
失效 session 的恢复有单独判断：确认旧 session 不会执行该请求时才允许重新建立并重试。
tools/list_changed notification 触发目录更新，工具生命周期由连接状态与当前 generation 共同约束。
依据：[src/extensions/mcp/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts)。

当前关闭边界收敛正在建立的连接：shutdown AbortController 取消 initialize/discovery client 和退避等待；close 等已连接 client 关闭及 opening 完成，再等 authProvider.settled 保存可能轮换的 tokens。connectOnce 记忆 close promise，避免二次 close 过早返回；晚到结果检查 closed，不能重置为 connected。[连接建立](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts#L378)、[close](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts#L479)、[静态关闭断言](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-extension.test.ts#L586)。

共享 pi-mcp 动态注册依据 redirect URI 推导 OIDC application_type；Pi loopback callback 得到 native，避免服务器按 web 默认拒绝 HTTP loopback。此变化在 OAuth 层，不新建本包登录框架，也不改变显式 CIMD 选择规则。[applicationType](../../../../.repo/pi-1.0.4/packages/mcp/src/oauth/flow.ts#L106)、[发布段](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md#L19)。

### 14.3 结果、资源与脚本结构化视图

MCP 文本输出按 20 KiB 做中间截断，并保存完整输出；图片保留为图片，resource link 转摘要，嵌入文本资源按 MIME 转换，二进制内容可以保存临时文件。
给模型看的 content 和供脚本读取的 structuredContent 有意不同：后者保留 CallToolResult 结构但去除 `_meta`，并保留 isError。
传入 signal/timeout/progress，使远程取消和进度能接上 session 工具事件。
依据：[src/extensions/mcp/tools.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts)。

除服务器工具外，还生成 list_mcp_resources、list_mcp_resource_templates、read_mcp_resource，按服务器能力调用客户端。
这些资源工具也使用统一工具输出转换和错误边界；资源内容与 annotations 仍是外部输入，不能提升为本地可信策略。
`/mcp` 与 CLI MCP 子命令承担配置、连接、授权和可见性管理，不是另一套 Agent 循环。
依据：[src/extensions/mcp/resources.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/resources.ts)、[src/extensions/mcp/resources.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/resources.ts)、[src/main.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts)。

### 14.4 CIMD、callback URI 与 issuer

`clientRegistration` 的默认策略仍为动态注册（预配置 clientId 另行使用）；显式 `oauth.clientRegistration:"cimd"` 才选择 Pi 的 Client ID Metadata Document。配置层禁止它与 clientId/clientName 组合，并限制自定义 callback 为 localhost/127.0.0.1 的 `/callback`；runtime 将配置原样交给 MCP OAuth provider。授权服务器必须同时广告 CIMD 支持和公开客户端认证方式 `none`，否则登录失败，不静默退回动态注册。[src/core/mcp-servers.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/mcp-servers.ts)、[src/extensions/mcp/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts)、[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L240)。

广告 RFC 9207 issuer 响应支持时，client ID 是 `https://pi.dev/oauth/client.json`，回调用共享 `/callback`；否则使用 MCP URL（去掉 fragment）的 SHA-256 前 9 字节 base64url 作为 ID，令 metadata URL 和 `/callback/<id>` 都绑定服务器。回调服务器只接受本次授权选择的路径；手工粘贴 URL 还比较 origin、pathname、state，再取 code 与 iss。两种返回路径都将 `iss` 传给 `authorizeMcp()`，由下层在兑换 code 前验证 issuer。注册模式切换时，旧动态 client 与其 tokens 不会被拿来当成 CIMD client 刷新。[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L227)、[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L383)、[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L471)、[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L518)。

issuer 防护已在 1.0.0 发行记录中出现；1.0.1 新增重点是 CIMD 选择与服务器专属 redirect，不能把已有 issuer 检查误记成 1.0.2 首次支持。当前测试断言默认 DCR、CIMD 两种 issuer 情形、模式切换、错误路径拒绝及不支持 CIMD 时失败；这是静态断言证据，未验证 pi.dev 文档服务或真实 OAuth 提供方。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[test/mcp-oauth-refresh.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-oauth-refresh.test.ts#L113)、[test/mcp-oauth-refresh.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-oauth-refresh.test.ts#L163)。

## 15. llama.cpp 扩展

内置 llama 扩展注册 native Provider，并通过 `/llama` 管理 router 中模型的列出、加载、卸载、下载及选择。
连接地址来自 Provider 认证配置/环境；目录同步直接查询配置服务器，再刷新该 Provider。
加载新模型时可选择先卸载其他模型；失败或取消在对应分支尝试恢复之前已加载模型，但恢复失败不会掩盖原始错误。
依据：[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L29)、[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L42)、[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L66)。

下载 UI 查询 Hugging Face 模型详情和量化版本，真正加载/下载由配置的 llama.cpp 服务完成；相关 API 与 token 解析封装在相邻 client/provider/huggingface 模块。
`/llama` 限交互模式。用户已经明确操作服务器后，其显式刷新允许网络，即使 PI_OFFLINE 已设置；因此 offline 应理解为限制自动网络行为，而不能当作全进程网络防火墙。
依据：[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L48)、[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L140)、[src/extensions/llama/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/llama/index.ts#L185)。

## 16. 三种默认产品模式

### 16.1 Interactive

InteractiveMode 持有 runtime 宿主与 TUI renderer，初始化编辑器、对话区、状态区、主题、终端颜色检测和启动输入缓冲。
renderer 根据设置选择 alternate-screen 或 main-screen 实现；共享 transcript/dock viewport 将滚动历史与固定输入区域分开。
会话替换时重绑事件和扩展 UI，而不是重新创建整个终端宿主。
依据：[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)、[src/modes/interactive/tui-renderer.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/tui-renderer.ts#L22)、[src/modes/interactive/chat-viewport.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/chat-viewport.ts#L22)。

提交路径先处理内置命令/扩展命令/模板/skill 与 `!` shell，再把普通输入送往 session。
事件 handler 将 message_start/update/end、tool execution、队列和压缩状态映射成已有组件；大部分模型/工具业务逻辑仍在 AgentSession。
树选择、分叉、session picker、模型选择和 export 属于这一呈现层的命令工作流。
普通退出先停 TUI 再 dispose runtime；SIGTERM/SIGHUP 先 dispose 扩展/session 再恢复终端；真正失效终端走紧急退出，不能承诺所有错误都完成常规清理。
依据：[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)。

观察：InteractiveMode 是大型协调类，组合很多命令与组件；这是功能集中度的事实，不凭文件大小直接判断质量。
本次追到初始化、提交、事件、导航和 shutdown 主路径，没有逐个审计所有快捷键、终端协议和视觉状态。

1.0.1 的登录 URL 复制用共享 `AuthUrlComponent` 显示 hyperlink、点击提示与当前 `app.message.copy` 绑定（默认 Ctrl+X）；复制成功或失败替换提示。`/login` 的 auth URL 页面和 MCP TUI 的 sign-in 页面先消费复制键，避免送进 code/redirect 输入框。device-code 页面清除 authUrl，因此这里的复制动作并不涵盖所有认证界面，也不能扩大为 shell `pi mcp login` 一定具备同样组件。[src/modes/interactive/components/auth-url.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/auth-url.ts#L10)、[src/modes/interactive/components/login-dialog.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/login-dialog.ts#L117)、[src/modes/interactive/components/login-dialog.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/login-dialog.ts#L230)、[src/extensions/mcp/ui.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/ui.ts#L179)、[test/auth-url-copy.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/auth-url-copy.test.ts#L23)。

图片呈现与 Provider 输入缩放是两条链。Kitty 协议要求 PNG；本包 `loadPngTranscoder()` 延迟加载 Photon，应用 EXIF 方向后同步编码 PNG，`ensurePngTranscoder()` 只在 Kitty 能力下注册到 pi-tui。InteractiveMode 初始化/设置应用后触发注册，使扩展直接创建的 Image 也可受益；ToolExecutionComponent 自行保证注册，回调 invalidate/render，供其它 TUI host 使用。测试断言旧 partial 图片不复活、转换后的 Image ID 在 invalidate 后保持；真实终端效果未实测。[src/utils/image-convert.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/image-convert.ts#L7)、[src/utils/image-convert.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/image-convert.ts#L61)、[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)、[src/modes/interactive/components/tool-execution.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/tool-execution.ts#L359)、[test/tool-execution-component.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tool-execution-component.test.ts#L48)。

1.0.0 → 1.0.2 的历史清单还显示旧 daxnuts 与 pi-logo-animation 两个实现/延迟入口共三个文件移除，新增 `easter-egg-3d.ts/.lazy.ts`。当前入口按需 import；只有 fullscreen renderer 可以播放，已有 overlay 时不叠加；Armin 在普通模式回退 inline 组件。动画以 overlay 拿到焦点，结束隐藏 overlay，计时器在不再渲染时清理。此处仅复核入口、生命周期与绘制结构，没有审计全部 raster 数学或测量帧率；源码更替不能无旧文本地认定为纯重命名。[src/modes/interactive/components/easter-egg-3d.lazy.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/easter-egg-3d.lazy.ts#L8)、[src/modes/interactive/components/easter-egg-3d.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/easter-egg-3d.ts#L842)、[src/modes/interactive/components/easter-egg-3d.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/components/easter-egg-3d.ts#L908)、[src/modes/interactive/interactive-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts)。

Home/End 默认保留给编辑行首/行尾，全文 transcript 首尾改为 Ctrl+Home/Ctrl+End；绑定来自 pi-tui。多行高亮对每个非空行分别应用 formatter 后再拆渲染行，避免首行后颜色丢失。[按键](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/keybindings.md#L61)、[formatter](../../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/syntax-highlight.ts#L153)、[theme 调用](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/theme/theme.ts#L1004)。

stdin/stdout/stderr 与 uncaught handler 将 EIO/EPIPE/ENOTCONN/ENOTTY 识别为失效终端：紧急路径杀跟踪子进程、退出 129，不再写恢复序列或建议 /bug。错误后的安装检测读取启动时 package.json 路径，更新/删除时给重启与恢复提示。bundled Node 的 codemode worker 首次读取成 data URL 缓存，后续 worker 不再依赖被更新删除的文件；不代表任意未加载代码均能热更新。[生命周期](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L4251)、[紧急退出](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L4292)、[安装提示](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts#L2184)、[worker 缓存](../../../../.repo/pi-1.0.4/packages/coding-agent/src/config.ts#L518)。

### 16.2 Print / JSON

runPrintMode 顺序处理初始输入和附加 prompts，复用同一 session；文本模式只输出最终 assistant 文本，error/aborted 设退出码 1。
JSON 模式先输出 session header，再输出会话事件；对 stdout backpressure 设置等待，结束时 flush 并 dispose runtime。
SIGTERM/SIGHUP 路径会清理跟踪的 detached children 并结束运行。
依据：[src/modes/print-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/print-mode.ts#L34)。

`toJsonEvent()` 删去每个 message_update 携带的累计消息快照，只保留必要 delta、tool-call identity 和 usage，最终以 message_end 提供权威完整消息。
这避免长流在 JSON 输出中反复序列化增长中的前缀造成近似平方级体积。
这里的 JSON 事件流与磁盘 SessionManager JSONL 是不同协议：前者包含生命周期/增量事件，后者是可回溯持久化条目。
依据：[src/modes/json-event.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/json-event.ts#L48)。

### 16.3 稳定 stdin/stdout RPC

RPC mode 在一条 stdin JSONL 上接收命令，在 stdout JSONL 返回 response 和异步事件；请求 ID 关联响应，事件不要求与单个请求一一对应。
同一模式也绑定 extensions，不过选择/确认/输入/编辑等 UI 能力转换为 `extension_ui_request`，客户端用响应 ID 回答。
通知、状态、widget、标题和编辑器文本等更新也可以跨此桥传递，不能假设所有扩展 UI 都是本地 TUI 组件。
依据：[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L54)、[src/modes/rpc/rpc-types.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-types.ts#L20)。

| RPC 命令族 | 代表能力 |
|---|---|
| 输入与队列 | prompt、steer、follow_up、abort、clear_queue |
| 状态 | get_state、get_messages、最后回答、统计、命令列表 |
| 模型 | set_model、cycle_model、thinking 与队列模式 |
| 生命周期 | new_session、switch_session、fork、clone、树条目查询 |
| 上下文 | 手工 compact、自动压缩/重试设置 |
| 其他 | bash、abort_bash、会话命名、HTML 导出、扩展 UI 响应 |

命令和 response union 的规范位置为 [src/modes/rpc/rpc-types.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-types.ts#L20)；处理分派位于 [src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L394)。

prompt response 只在前置检查完成后返回一次 success，disposition 为 started/queued/handled；前置失败返回一次 error。
已经 started 后的 provider/工具失败由事件和会话状态呈现，不能再把同一请求回成第二次失败 response。
客户端等待完整回答需要继续观察事件；RpcClient 的 `promptAndWait()` 与普通 `prompt()` 因此是不同方法。
依据：[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L394)、[src/modes/rpc/rpc-client.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-client.ts#L198)、[src/modes/rpc/rpc-client.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-client.ts#L513)。

framing 按 LF 分割、支持 CRLF 和最后无换行的记录，保留 JSON 字符串里的 U+2028/U+2029；不使用会把这些字符当行分隔符的通用文本接口。
输入先 JSON.parse 并检查基本对象/type，随后交由命令分派；其静态 TypeScript union 不等于对每一命令字段统一应用严格 runtime schema。
RpcClient 自行 spawn RPC 入口、管理 pending IDs、解析响应并在进程退出时拒绝未决请求。
依据：[src/modes/rpc/jsonl.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/jsonl.ts#L10)、[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L753)、[src/modes/rpc/rpc-client.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-client.ts#L56)、[src/modes/rpc/rpc-client.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-client.ts#L597)。

**已确认：这套 RPC 没有通过 pi-protocol/pi-client/pi-server 实现。** `src/client/index.ts` 只是实验 client 子路径的重新导出，不能拿它替换稳定 RpcClient 的职责。
依据：[src/client/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/client/index.ts#L1)、[src/modes/rpc/rpc-client.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-client.ts#L1)。

## 17. 实验性 server/client 与 services

### 17.1 入口和发布条件

`experimental/cli.ts` 先尝试实验命令，未处理则回落默认 main。
只有 `PI_EXPERIMENTAL === "1"` 且首参数为 server/client 时才命中；命令分派代码明确标注 development-only，正式入口不得导入。
源码启动脚本预加载 source-resolver；该 resolver 根据根 tsconfig aliases 定位源码，找不到映射即报错，不静默退回可能陈旧的 dist。
依据：[src/experimental/cli.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/cli.ts#L1)、[src/core/experimental.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/experimental.ts#L1)、[src/experimental/commands.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/commands.ts#L85)、[src/experimental/source-resolver.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/source-resolver.ts#L45)、[pi-test.sh](../../../../.repo/pi-1.0.4/pi-test.sh#L58)。

这条路径使用 protocol/client/server 包处理连接和 transport，使用 Chord 的服务 token、facet graph、远程服务目录与复制状态。
SessionDirectory/SessionManagement 等应用 DTO 定义在本包 services 中；底层 transport 将 service payload 当作不透明数据，不拥有编码产品语义。
依据：[src/experimental/client-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-runtime.ts#L53)、[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L1)。

### 17.2 服务进程与 Session worker

服务 profile 有逻辑 server ID、目录锁和默认 ID 文件；默认 server 目录为 `~/.pi/server`，默认实验会话目录在 agentDir 的 experimental/sessions 下。
本地 socket 目录要求当前 POSIX user 所有并设为 0700；`process.getuid` 缺失则明确报错，所以不能据此声称本地 server 在所有平台上等价可用。
client 可以连接本地 Unix 地址或 Radius 路由，配置插件路径的能力受本地路由条件约束。
依据：[src/experimental/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/server.ts#L51)、[src/experimental/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/server.ts#L58)、[src/experimental/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/server.ts#L79)、[src/experimental/client-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-runtime.ts#L53)。

每个 Session 目录包含 meta.json 和 session.sqlite；目录列表/创建依赖元数据，worker 才持锁打开数据库并拥有 Harness。
worker 启动参数使用 StrictObject TypeBox schema，进一步要求 sessionDir/cwd/path 是绝对路径、provider/model 组合合法；内部入口还要求正确的进程 role。
控制通道使用专门 token 和 session key；服务请求带 serverConnectionId/attachmentId scope，防止旧 attachment 继续向新绑定发送请求。
依据：[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L47)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L64)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L739)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L830)。

worker 生命周期同时考虑 attachment demand、进行中的 request hold、Harness task graph 活动和失联后的宽限。
仅没有连接不一定退出：后台有活任务时 worker 继续拥有数据库；仅请求已经返回也不意味着后台任务完成。
worker 在初始化服务和 task graph 后调用 harness.resume()，恢复被中断的 durable 工作。
依据：[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L184)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L237)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L509)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L594)。

### 17.3 Harness 装配与工具实现差异

createCodingAgentHarness 创建 ModelRuntime、SettingsManager、HTTP 配置和按 cwd 缓存的 NodeExecutionEnv，然后以 SQLite storage 打开 Harness。
`createCodingRegistry()` 安装的是 **pi-durable 的 CodingTools** 和本包的 coding prompt extension；并未把 `core/tools/read.ts` 等默认 AgentTool 实现直接接入。
Harness settings 用 getters 读取启动时已加载的压缩、重试、队列和 stream 设置。
已有 root conversation 保留持久化模型配置；CLI 初始模型只在 root 初次创建时应用。
依据：[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L773)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L25)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L52)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L59)。

默认核心中的工具取消、截断、file-mutation queue、codemode/MCP exposure 等结论不能自动套用到这个 CodingTools registry。
这里共享的是若干产品配置和提示构造，其任务调度、提交、恢复、工具回放契约来自 durable 包。
本报告定位接线，不重复把另一个包全部内部实现纳入本包结论。

独立 env 包并未替换当前应用装配：ExecutionEnvs 仍导入 pi-durable/env/node 的 NodeExecutionEnv，以 new NodeExecutionEnv({cwd}) 创建；server worker 与独立 durable runtime 把 envs.env 传 Harness。默认 SDK 继续装配 core/tools，没有 pi-env 接线。新增包与旧 NodeExecutionEnv 内部的变化须分别解释，不能由源码调用方未变推断底层行为未变。[ExecutionEnvs](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L59)、[worker](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L790)、[runtime](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L144)、[durable](durable.md)、[env](env.md)。

### 17.4 Chord services 与投影

worker 建立 built-in AgentController、SessionPlugins、Models、Transcript facets，再合并独立插件 generation，创建 FacetHost。
服务目录由实际提供的非 local tokens 生成，客户端按消费需求绑定，没有一份另行维护的内置 RPC 服务清单。
每个连接/attachment pair 有自己的 RemoteServiceEndpoint 和订阅状态，detach 时按 scope 清理。
依据：[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L47)、[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L3)。

AgentController 只包 root conversation：prompt 使用 whenBusy:reject；steer/followUp 入队；cancelQueued 使用 conversation 约束的 abortSubmission；compact 返回 operation ID。
waitForPrompt 解析 submission ID，等待该 submission，再读取其具体 assistant answer，区别于“等待全局最新 assistant”。
Transcript 直接提供 conversation.viewState 的 replicated state；worker 不维护第二个 reducer，Chord 负责状态同步与副本恢复机制。
依据：[src/experimental/services/agent-controller-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/agent-controller-provider.ts#L19)、[src/experimental/services/transcript-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/transcript-provider.ts#L6)。

服务版 TUI 复用稳定版 alternate-screen renderer、viewport 和消息/工具/主题组件，但通过 AgentController 发操作、订阅 Transcript，不直接驱动 AgentSession。
本地 SlashCommands 和 PresentationUI token 提供窄化的命令与选择/状态能力，不把原始 TUI 对象作为跨端服务。
依据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L28)、[src/experimental/plugin.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/plugin.ts#L1)。

### 17.5 实验 plugins 不是稳定 extensions

实验 plugin package 约定 `src/session.ts`、`src/tui.ts` 等 facet 入口，由 Chord 构建成不同 host 的 bundle。
服务器默认选择与某个 Session 分支选择分别保存；客户端拿到与选中 Session 匹配的 presentation artifacts。
worker 热重载先加载 candidate，FacetHost.reload 完成切换后才 dispose retired generation；失败则清理 candidate，保留已运行代。
依据：[src/experimental/plugins/package.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/plugins/package.ts#L69)、[src/experimental/plugins/bundled.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/plugins/bundled.ts#L12)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L103)。

这套依赖图、服务 shape 校验和 lifecycle disposal 属于 Chord facets；稳定 extension 的 input/tool_call hooks、命令注册和资源发现不是同一 API 的不同拼写。
不能因为二者都叫插件便推导第三方稳定扩展可原样在 worker 服务中运行。

## 18. 两个独立 durable 演示

### 18.1 单进程 durable coding agent

`experimental/durable/main.ts` 是另一个独立源码入口，单进程拥有 ModelRuntime、Harness、SQLite 与 TUI。
会话路径是 experimental/durable-sessions 下的 cwd hash/session 目录，与默认 JSONL、server sessions 分开；锁保护同一数据库所有权。
runtime 打开 Harness 后将 ConversationView 与 taskGraph 转成普通 view/controller，UI 只观察并发出操作。
close 不写入“已中止”的终局结果，使下次 continue 能恢复未完成工作；显式 abort 则是不同操作。
依据：[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L1)、[src/experimental/durable/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L124)、[src/experimental/durable/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L332)。

该演示额外安装 Subagent extension。每次 subagent 调用创建由当前 tool task 拥有的 child conversation，复制 agent 配置但不继承完整聊天历史，并移除 Subagent 防止继续递归委派。
它用 ownerTaskId 找回已创建 child，用 `subagent:${taskId}` requestId 复用提交，从而支持 replay-safe 重跑。
取消调用会取消 child 的相关工作；child conversation 保留供用户继续查看/交谈。
依据：[src/experimental/durable/subagent.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/subagent.ts#L19)、[src/experimental/durable/subagent.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/subagent.ts#L24)。

它明确没有默认产品完整的 session picker、fork/tree、稳定 extensions、prompt templates、图片和 `/login`；认证复用现有用户存储。
这些是演示文档声明的范围，不是本次未运行而推测出来的缺陷。
依据：[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L66)。

### 18.2 Vacation：后台任务与回报恢复

vacation 以旅行计划提示替换 coding prompt，以 search/research 替换编码工具，复用 durable TUI 外观。
search 返回源码中的 canned 天气、博物馆和列车结果，通过不同延迟展示并行任务；它不是实时互联网研究器。
search 标记 replay:safe，允许中断后重新执行未完成搜索。
依据：[src/experimental/vacation/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L1)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L1)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L44)。

research 创建 conversation-owned background task 后立即返回；Research task 按 deliver/report 两阶段推进。
deliver 使用 `research:${task.id}` 向 child 发送请求，得到文本后 checkpoint 到 report 状态；report 再以 followUp 向主 conversation 投递，并使用 `research-report:${task.id}` 去重。
持久化 checkpoint 与稳定 request ID 将“完成研究”和“向主线程回报”分开，使恢复时不必重新发送已提交的请求/报告。
这是演示由状态机表达后台工作，而非默认编码产品已经内置同名后台 research 功能。
依据：[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L62)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L79)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L108)。

## 19. 权限与信任边界

产品安全文档明确：Pi 按启动账户权限读写和执行，不在每次工具调用前请求批准；扩展、安装器、language server 和子进程继承相同权限，除非外部环境限制。
工具 schema、截断、取消与项目 trust 各解决不同问题，不能叠加推导成文件/网络权限隔离。
文件内容、注释、上下文指令、工具输出、MCP 结果和模型输出均可能影响后续模型行为。
依据：[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L3)。

项目 protected resources 包括设置、MCP、extensions、skills、prompts、themes、SYSTEM/APPEND 等；只有存在相应资源才需要决策，裸 `.pi` 目录本身不强制询问。
信任决策顺序包含显式 CLI 覆盖、用户/CLI 扩展的 project_trust 回答、已存记录、全局 always/never/ask 策略及是否有交互 UI。
信任存储对规范化目录记录决定，并支持最近祖先继承；非交互无适用授权时不能弹内置确认。
依据：[src/core/trust-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/trust-manager.ts#L30)、[src/core/trust-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/trust-manager.ts#L192)、[src/core/trust-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/trust-manager.ts#L210)、[src/core/project-trust.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/project-trust.ts#L48)。

两个明确例外必须保留在安全理解中：启动选择会话时已读取项目 sessionDir，发生在最终信任决策前；上下文 AGENTS/CLAUDE 文件即使拒绝项目 trust 仍加载，除非禁用 context。
SDK 默认信任与实验 worker 直接创建 SettingsManager 的路径，也不能套用默认 CLI 的交互确认保证。
codemode VM 限制的是脚本运行环境，通过 tools 调用的进程、文件和远程操作仍由宿主权限执行。
依据：[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L31)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L57)、[src/core/settings-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/settings-manager.ts#L442)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L775)。

实验 server 的本地目录权限、worker token、attachment scope 是可观察到的局部边界；不能据此宣称已具备多租户 workspace/plugin 授权模型。
services 文档仍将 authenticated per-client projection、workspace authorization、plugin policy 列为后续工作；本报告不把未完成设计假写成当前保证。
依据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L15)。

## 20. 三条完整流程追踪

### 20.1 默认 CLI：输入 → 工具 → 写入 → 回答 → JSONL

```mermaid
sequenceDiagram
    participant CLI as main / mode
    participant S as AgentSession
    participant A as Agent
    participant M as ModelRuntime / Provider
    participant T as edit / write
    participant J as SessionManager
    CLI->>S: prompt(text)
    S->>S: input、模板、认证、压缩前检查
    S->>S: before_agent_start + loadout
    S->>A: prompt(messages)
    A-->>S: user message_end
    S->>J: appendMessage（必要时首次建文件）
    A->>M: streamSimple(context)
    M-->>A: assistant tool call
    A->>S: beforeToolCall
    S-->>A: hooks 允许或阻止
    A->>T: validated args + signal
    T->>T: 目标文件队列 + 执行
    T-->>A: content + details
    A->>S: afterToolCall / tool_result
    A-->>S: toolResult message_end
    S->>J: appendMessage
    A->>M: 下一次请求含工具结果
    M-->>A: 最终 assistant
    A-->>S: message_end / agent_end
    S->>J: appendMessage
    S->>S: 重试/压缩/后续队列/before_settle
    S-->>CLI: agent_settled
```

这个流程的观察点是：工具执行先通过 hooks 与参数管线；文件互斥只保护本进程中的目标文件变更；Provider 的流事件不逐个写成 JSONL message；最终消息完成才追加对应 message entry。
若工具取消，进程/文件实现的取消边界决定副作用是否已经发生；如果会话切换，runtime 等待旧运行结束和结果记录后才失效旧引用。
依据：[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/core/tools/file-mutation-queue.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/file-mutation-queue.ts#L29)。

### 20.2 恢复旧会话 → 导航分支 → 压缩后继续

1. CLI 选择 JSONL，SessionManager 扫描记录、迁移旧格式、重建 id 索引与 leaf。
2. buildSessionProjection 沿 parent 链选择分支，应用最新 compaction 和 context edits；SDK 根据分支状态恢复模型/思考等级。
3. 用户选择树目标；AgentSession 在 idle 状态计算分支差异，允许 before_tree 取消或修改摘要策略。
4. 需要时将离开分支的相关内容交给 summary 模型，得到 branch summary；移动 leaf 并追加摘要条目，不删除旧分支。
5. 新输入前检查当前上下文预算。若超阈值，prepareCompaction 选出安全切点并生成摘要。
6. 成功追加 compaction，保存 firstKeptEntryId 与 system checkpoint，更新 Agent 消息投影；然后按正常 prompt 管线开始新运行。
7. 下次恢复仍由同一树与摘要投影重建，而不是依赖上一次进程的内存消息数组。

关键依据：[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L1085)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L543)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/core/compaction/compaction.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/compaction/compaction.ts#L872)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)。

### 20.3 实验客户端：服务调用 → durable 提交 → 复制状态

1. 源码实验入口解析 server/client；客户端发现、激活或连接所选 server，并获得远程服务目录。
2. SessionManagement 创建/选择 Session，server 加载元数据并保证对应 worker；attach 建立 scope，返回服务/呈现资源。
3. worker 打开 SQLite 和 Harness，装配 CodingTools、prompt、ModelRuntime 及 settings，创建或恢复 root conversation。
4. worker facets 提供 AgentController、Transcript 等，RemoteServiceEndpoint 按当前 attachment 接受调用。
5. 客户端调用 AgentController.prompt，实际提交 durable input，得到 operationId；忙时 reject 或使用 steer/followUp 方法排队。
6. Harness 运行模型和工具，更新 conversation view；Transcript 直接复制该 view，客户端边等控制器结果边渲染状态。
7. waitForPrompt 等待特定 submission 的 answer，而非读取一个易竞争的“最后消息”。
8. 客户端断开释放 demand；如果 task graph 仍有活动，worker 不退休。进程恢复时 Harness.resume 接续持久化工作。

关键依据：[src/experimental/client-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-runtime.ts#L53)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L509)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L47)、[src/experimental/services/agent-controller-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/agent-controller-provider.ts#L19)、[src/experimental/services/transcript-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/transcript-provider.ts#L6)。

## 21. 测试证据矩阵

以下是**静态阅读的测试源码与断言主题，未执行**。未变文件的原有选段分析按哈希继承；本轮重新核对的变更用例单列于表末。没有将测试存在换算成通过率、平台兼容承诺或端到端服务认证。
`test/suite/harness.ts` 创建临时目录、内存设置/认证与 FauxProvider，适合在不调用真实 Provider 的条件下表达 session 行为；个别独立测试仍使用 subprocess、临时文件或本地 fixture server。
依据：[test/suite/harness.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/harness.ts#L150)。

| 测试文件与锚点 | 覆盖主题 | 能支持的静态判断 |
|---|---|---|
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | read offset/limit、图片 magic、BMP | 文本窗口和图片分支有具体行为断言 |
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | 多 edits 对原文匹配、重叠、空 edits、失败不部分写入 | multi-edit 契约被明确测试 |
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | fuzzy、Unicode、空白、CRLF、BOM | 容错匹配与原文保真有测试材料 |
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | shell 非零退出、结构化结果、输出限额、timeout | 非零退出保留可检查结果 |
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | 输出合并、UTF-8 chunk、进度节流 | 流式输出不只是字符串拼接的 happy path |
| [test/tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | grep/find/ls 及上下文 cwd | 发现工具与 cwd override 有行为覆盖 |
| [test/file-mutation-queue.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/file-mutation-queue.test.ts#L38) | 同文件串行、不同文件并行、symlink、abort 后队列保持 | 进程内写入排序的关键竞争受到测试 |
| [test/default-tools-setting.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/default-tools-setting.test.ts#L58) | 默认工具、PowerShell/custom、显式覆盖、reload/services | 工具选择不是仅 CLI 字符串解析 |
| [test/session-manager/build-context.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/session-manager/build-context.test.ts#L127) | 摘要先于保留尾部、最近压缩、全路径设置、复杂分支 | 投影与树模型有独立测试 |
| [test/session-manager/file-operations.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/session-manager/file-operations.test.ts#L83) | 非法行、尾部换行、超扫描限/大文件、保留非会话文件 | 恢复器考虑损坏/非预期文件 |
| [test/session-manager/file-operations.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/session-manager/file-operations.test.ts#L447) | setup 不建文件、首 user 建文件、后续 append | 首次持久化时点由测试明确约束 |
| [test/session-context-edit.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/session-context-edit.test.ts#L37) | 只改投影、后 edit 覆盖、分支作用域、压缩后 edit、usage 失效 | 原记录与可见上下文分离被测试 |
| [test/compaction.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/compaction.test.ts#L194) | usage、零值排除、预算、切点、custom/大工具/重复压缩 | 预算和摘要输入选取存在算法级断言 |
| [test/agent-session-runtime-events.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/agent-session-runtime-events.test.ts#L115) | new/resume、取消切换、shutdown→invalidate→rebind、fork | runtime 生命周期顺序可验证 |
| [test/extensions-runner.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/extensions-runner.test.ts) | user_bash 出错阻止执行、结果补丁链、context、边界草案 | hooks 失败策略各异且有专门用例 |
| [test/extensions-runner.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/extensions-runner.test.ts) | 延迟 Provider 注册、headers 等请求 hooks | 动态扩展接线不只测试注册存在 |
| [test/agent-session-dynamic-tools.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/agent-session-dynamic-tools.test.ts#L29) | shell 会话环境、动态工具、来源元数据 | 当前 session 上下文能传入工具执行 |
| [test/model-runtime-credential-sync.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/model-runtime-credential-sync.test.ts#L63) | 提交后同步、同 Provider 串行、跨 Provider 并行、已提交失败 | 认证写入与同步错误有区分 |
| [test/trust-manager.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/trust-manager.test.ts#L24) | 祖先信任继承、保护资源检测 | 项目信任是路径/资源层政策 |
| [test/rpc-prompt-response-semantics.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/rpc-prompt-response-semantics.test.ts#L197) | preflight 单一失败、started/handled/queued、清队列 | RPC 接收确认与运行完成分离 |
| [test/rpc-jsonl.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/rpc-jsonl.test.ts#L6) | LF、CRLF、Unicode separator、末行无 LF | framing 的特殊字符策略有明确断言 |
| [test/suite/agent-session-codemode.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-codemode.test.ts) | 并行子调用、hooks、usage、结构化 patch、图片 | 脚本桥接仍通过 session 工具管线 |
| [test/suite/agent-session-codemode.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-codemode.test.ts) | timeout 参数、内存限、输出溢出、bash 错误、store 成败/分支 | 沙箱和持久 store 有异常路径测试 |
| [test/suite/agent-session-codemode.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-codemode.test.ts) | models API 认证与图片生成 | 脚本不能以随意提供的字段替代 runtime 认证 |
| [test/suite/agent-session-mcp.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts) | exposure、树恢复、direct、resources、逐工具覆盖 | MCP 的可见性是 session 级行为 |
| [test/suite/agent-session-mcp.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts) | 间接服务器不拖慢首 prompt、按需等待、direct 超时 | 连接等待政策有边界断言 |
| [test/suite/agent-session-mcp.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts) | resume 注册、晚到结果、reload | generation 与恢复处理受到测试 |
| [test/experimental-agent-controller.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/experimental-agent-controller.test.ts#L11) | 经过服务目录 prompt/wait、busy、steer/followUp/cancel、compact | 服务不是仅类型声明，有 Harness 集成测试 |
| [test/experimental-transcript-provider.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/experimental-transcript-provider.test.ts#L11) | conversation view 变化复制 | Transcript adapter 不另造消息 reducer |
| [test/experimental-plugin-reload.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/experimental-plugin-reload.test.ts#L9) | 新 facet generation 激活、旧代与最终代 dispose | generation 切换及资源释放顺序有断言 |
| [test/experimental-session-worker-lifecycle.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/experimental-session-worker-lifecycle.test.ts#L22) | demand+activity、多个 attachment、ack hold、旧 generation、orphan grace | worker 退休不是简单的连接计数 |
| [test/experimental-server-profile.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/experimental-server-profile.test.ts#L23) | 同 profile 启动序列化、不同 ID 独立、损坏 ID 拒绝 | 服务实例身份和启动锁有测试材料 |

继承的 1.0.2 变化测试（当前变化文件使用相应主题的文件引用）：

| 测试文件与当前锚点 | 核对的断言 |
|---|---|
| [test/model-registry.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/model-registry.test.ts#L765) | sampling 默认、逐等级 override 按键合并、未配置字段保持 absent |
| [test/mcp-extension.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-extension.test.ts)、[同文件](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-extension.test.ts) | 项目 override 保留全局 env、拒绝额外字段/无基底；CIMD 与 callback 配置校验 |
| [test/mcp-oauth-refresh.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-oauth-refresh.test.ts#L163) | 默认 DCR、两种 CIMD redirect、旧 client 替换、浏览器和粘贴路径拒绝、不支持时失败 |
| [test/extensions-runner.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/extensions-runner.test.ts#L892)、[test/suite/regressions/10285-mcp-tool-renderers.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/10285-mcp-tool-renderers.test.ts#L14) | renderer resolver 顺序、next 补缺、未注册 MCP 工具和 HTML export |
| [test/auth-url-copy.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/auth-url-copy.test.ts#L23) | 登录/MCP URL 复制与无 authUrl 时不复制 |
| [test/image-processing.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/image-processing.test.ts#L85)、[test/tool-execution-component.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/tool-execution-component.test.ts#L48) | 同步 PNG transcoder、EXIF 方向、无效图片、partial 替换与稳定 Image ID |
| [test/args.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/args.test.ts)、[test/suite/regressions/7027-credential-refresh-hang.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/7027-credential-refresh-hang.test.ts#L127) | 空 model pattern 过滤；登录后默认模型/Radius 首模型选择、等待 refresh、避免覆盖并发选模和 15 秒边界 |
| [test/export-html-whitespace.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/export-html-whitespace.test.ts#L24)、[test/suite/mcp-oauth-server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/mcp-oauth-server.ts) | HTML renderer 新依赖接口接线；CIMD/issuer/redirect fixture 的测试支撑，均不是实网证据 |

本轮复读静态断言：MCP allowlist/exclude/恢复 gate（agent-session-mcp 231–324）、禁用内置优先级（resource-loader 1113–1133）、hidden guidelines/零预算（agent-session-codemode 141–205）、read 图片桥接（同文件 524–540）、initialize/退避关闭（mcp-extension 586–633）、OAuth 请求取消（model-runtime-auth-options 266–301）。其它变化测试核对主题目录，不冒充全部断言全文；未执行任何测试。

测试影响记录：`none`。本研究没有添加、更新或删除测试，也没有执行现有测试。
矩阵是围绕本报告论点的选取，不是完整测试清单；未列出的测试不能被解释成不存在。

## 22. 设计取舍与已观察限制

1. **通用循环与产品协调分层。** Agent 承担通用执行，AgentSession 承担产品上下文与生命周期；可复用边界明确，但产品协调类需要理解很多状态之间的时序关系。[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)
2. **JSONL 易检查、树历史可保留，投影复杂度相应增加。** compaction、context edit、branch summary、模型/工具状态不能靠简单截数组恢复。[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L543)
3. **持久化默认是消息级，实验 durable 则有另一套提交模型。** 默认流事件和最终 JSONL 条目不同；实验 view/task 恢复由 Harness 负责，两套恢复强度不能混称。[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)、[src/experimental/durable/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L332)
4. **工具输出有界，执行副作用不自动撤销。** 文件读取可能完整载入；write 取消不能撤回已写数据；shell 已执行部分命令不会因 abort 回滚。[src/core/tools/read.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/read.ts)、[src/core/tools/write.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/write.ts#L39)
5. **统一嵌套工具管线减小绕过面。** hooks、取消、usage 在 codemode 子调用上仍适用，但它们只约束通过 API 的调用，同进程扩展本身能使用 Node 权限。[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts)
6. **发现与激活分离控制提示体积。** MCP/deferred/codemode 降低初始声明体积，代价是按需等待与工具集动态变化需要专门恢复/生命周期逻辑。[src/extensions/mcp/tools.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts)
7. **schema 和类型不能替代信任模型。** 工具参数、worker scope、配置文件分别使用不同校验策略；安全结论需要对应实际边界，不能因为 TypeScript union 或 TypeBox 存在就假设所有输入完全验证。[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L753)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L739)
8. **实验 services 当前只暴露 root conversation。** 文档明确暂无 tree navigate、nextRun、显式 resume 方法；subagent 需要 keyed service 扩展，Transcript 仅含最近 reset/compaction 后 active context，旧历史仍需分页接口。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L42)
9. **实验 plugins 采用代际替换。** 服务形状兼容时保留消费者 facade，替换实现并释放旧代；这有利于热重载一致性，但需要严格尊重 host 的连接/释放次序。[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L103)
10. **本地目录安全不等于跨用户授权。** 本地 POSIX 所有权/token/scope 有实现证据，而多用户资源授权仍是文档中的后续项；本次没有对远端 Radius 服务进行协议或部署安全验证。[src/experimental/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/server.ts#L58)、[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L15)

## 23. 分层阅读顺序

| 阅读轮次 | 推荐源码 | 要回答的问题 |
|---|---|---|
| 1：边界 | package.json、tsconfig.build.json、cli.ts、main.ts、index.ts | 哪些是发布入口，哪些只是源码实验？ |
| 2：装配 | agent-session-services.ts、sdk.ts、agent-session-runtime.ts | 谁创建服务，切 cwd/会话时谁负责释放？ |
| 3：一次运行 | agent-session.ts 的 prompt、事件、工具 hooks、settled | 输入何时接受，消息何时写入，运行何时结束？ |
| 4：可恢复历史 | session-manager.ts、messages.ts、compaction/* | 分支与压缩如何重建模型上下文？ |
| 5：实际副作用 | tools/index.ts、read/edit/write/bash、queue/accumulator | 参数、取消、输出上限和文件权限在哪里生效？ |
| 6：配置与信任 | settings/model-runtime/auth-storage/resource-loader/trust | 哪些配置会执行代码或启动网络？ |
| 7：可扩展性 | extensions/types/loader/runner、codemode、MCP | 声明、激活、可调用与生命周期如何分开？ |
| 8：呈现 | interactive 主路径、print、rpc-types/rpc-mode/rpc-client | 接受确认与完整回答如何呈现给人或程序？ |
| 9：实验集成 | experimental/cli、server、worker、services、durable/vacation | 另一套持久化和服务模型复用了什么、替换了什么？ |
| 10：验证意图 | 上述各层相应 test 与 suite harness | 哪些异常、竞争和恢复行为已有可执行规格？ |

最小闭环可以先按第 20.1 节逐符号追踪，再阅读第 7–8 节的投影；想理解实验服务时，应从 worker 的 Harness 创建继续，而不是从默认 AgentSession 推测。

## 24. 覆盖范围与未验证项

本报告覆盖包交付边界、默认启动与 SDK、AgentSession 主状态流、JSONL 树与压缩、八类工具、资源与模型认证、扩展 hooks、codemode、MCP、三种默认模式，以及实验 server/services/durable/vacation 的真实接线。
主要结论同时引用实现和针对性测试；源码事实与设计观察已在相关章节分开。

本轮全文阅读 5 份变化 docs（cli、codemode、extensions、keybindings、mcp），CHANGELOG 详读 1–107；另全文复读 providers、custom-provider 和 services README。按符号链核对 SDK/AgentSession 的工具选择、hidden loadout 与 prompt/skills、read 结构化输出、输出文件所有调用方、codemode 输出/模型桥接、MCP close/认证、UI 退出/安装提示和高亮。测试只读相关断言及主题目录，不声称所有变更测试全文审计。manifest 全文复核；安装与示例锁文件纳入路径/摘要差异，未逐项审计传递记录。

804 个未变文件按 1.0.2 清单和审读证据继承，包括默认会话树、压缩、稳定 RPC 与实验目录主体。NodeExecutionEnv 调用方仍未改用独立 env 包；依赖影响结合 durable、env、ai、tui、codemode 报告判断。[继承证据](../evidence/pi-1.0.2/README.md)。改变文件不沿用未经核对的旧数字行号；新主张重新定位到当前符号与调用方。

未逐分支覆盖：全部交互快捷键与视觉组件、所有主题/图片协议、HTML 导出模板的完整安全审计、包安装器所有来源和平台分支、所有 OAuth Provider 的登录细节、MCP UI/OAuth 全流程、Radius relay 内部实现、全部示例实现和每一个回归测试。
这些是研究范围边界，不应解释为上述能力缺失或存在缺陷。

未执行：依赖安装、build、typecheck、lint、测试、CLI、任何 Provider/模型/MCP/外部服务请求、真实 shell 工具和文件编辑工具调用、实验进程恢复演示、性能或跨平台基准。
本次只进行本地静态读取和报告引用检查；行为是否在具体环境实际通过，需要另行运行对应验证。


## 25. 设计文档、规划与时效核查（2026-10-07）

本节核对文档中的设计意图、未来方向和历史承诺，当前基准是 **Pi 1.0.4 本地快照**；未变材料继承 1.0.2 及其保全的更早静态阅读记录，变化文档以当前正文重新判断。
日期是本次静态核查日期，不代表已查询此日的上游最新状态；没有访问文档中的外链，也没有执行示例、构建或测试。
“已实现”表示找到对应实现链；“部分实现”表示机制已有但文档列出的延伸尚未接入；“尚无实现”限定在当前包的可见契约与接线。
“非目标”表示文档明确排除或只是示例范围；“被替代”表示旧机制的意图由不同机制承接；“文档冲突/过时”需要当前源码的正面证据，不能只由搜索无结果推定。
**没有实现、存在 TODO 和公开承诺交付，是三个不同判断。**

### 25.1 文档地图与实际阅读范围

包内仍有 79 份文本资料：根目录 2、docs 40、examples 16、实验 README 3、测试指南 2、skill fixtures 16。六份变化文档均有本轮阅读；其余按 1.0.2 索引原深度继承，只有下表注明者追加本轮全文复读。逐文件记录见[文档索引](../documentation-index.md)，身份及传递继承见[历史证据](../evidence/pi-1.0.2/README.md)。枚举、同哈希及“全文阅读”均不等于运行验证。

| 本轮阅读 | 范围与重点 |
|---|---|
| CHANGELOG | 1–107：1.0.4/1.0.3 新段及 1.0.2/1.0.1 背景；更早段沿用历史审读，不冒称 6,148 行全文 |
| docs/cli.md | 全文 1–333；工具 patterns、MCP 例外与 no-mcp |
| docs/codemode.md | 全文 1–208；图片落盘、read 返回值、guidelines |
| docs/extensions.md | 全文 1–274；ToolLoadout、hidden 声明、同进程权限 |
| docs/keybindings.md | 全文 1–194；Home/End 与 Ctrl+Home/End |
| docs/mcp.md | 全文 1–272；选择/曝光、禁用、SDK |
| docs/providers.md | 未变全文复读 1–200；Azure 环境变量和 endpoint 说明仍有效 |
| docs/custom-provider.md | 未变全文复读 1–173；Responses 作为 API 实现的列举仍有效，不等于整个 Azure provider 仅支持 Responses |
| src/experimental/services/README.md | 未变全文复读 1–45；旧迁移前提及 root-only 服务边界 |

models.md 的 azure-openai-responses 是 API 标识，不能随 provider 改名而机械判为过时。SDK compat、服务授权/导航、conformance 的历史归因在后续章节逐项保留。没有现成双语研究报告，故不新增对应译文。

### 25.2 默认产品文档表达的设计动机

默认 README 将产品定位为小型、可扩展的终端 coding harness，复杂工作流通过扩展、skills、prompt templates、themes 和 packages 组合。
这是一项**现行设计原则**，不是“未来准备加入扩展”的愿景：当前 ResourceLoader、ExtensionRunner、工具注册和 SDK 已经承担对应能力。
CLI integration 把 interactive、print、JSON、RPC 解释为共享 agent/session 机制的不同输入输出方式；SDK 则改变进程边界，不另造会话语义。
这个概括的适用范围是默认产品入口，不能扩大成实验 SQLite/Chord 服务也使用 AgentSession。
证据：[README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/README.md)、[docs/cli-integration.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/cli-integration.md#L3)、[docs/how-pi-works.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/how-pi-works.md#L33)、[src/core/sdk.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts)。

| 设计主题 | 文档动机与当前实现判断 | 时效及边界 |
|---|---|---|
| 可持久化的分支上下文 | SessionManager 保留树历史，默认模型上下文是 active branch 的投影；压缩释放上下文，branch summary 保留离开分支的工作信息 | 已实现。compaction 文档中的 Goal / Progress / Next Steps 是给模型生成摘要的格式，不是仓库开发计划。[docs/compaction.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/compaction.md#L16)、[docs/compaction.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/compaction.md#L234)、[src/core/session-manager.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/session-manager.ts#L543) |
| 恢复不篡改原始历史 | 溢出/length 失败后以 context_edit 排除失败尝试，再压缩并另起 retry；旧消息仍可导出与计费 | 已实现的当前顺序说明。文档明确失败或取消压缩时不自动再重试，不能把“可恢复”概括成所有错误都保证重做。[docs/compaction.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/compaction.md#L85)；实现及测试见第 8、21 节 |
| 可嵌入的会话生命周期 | SDK 区分 AgentSession 的运行管理和 AgentSessionRuntime 的 cwd 相关服务重建；强调持久化权威在 SessionManager | 已实现。不是另一个远程服务 SDK；运行结束应看 agent_settled，不能只看 agent_end。[docs/sdk.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L38)、[docs/sdk.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L58)、[docs/sdk.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L92)、[src/core/agent-session-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session-runtime.ts#L43) |
| 扩展拥有行为，skills/templates 提供指令 | 工具/事件/命令需要可执行 ExtensionAPI；skills 先暴露元数据再读取内容，templates 复用文本 | 已实现且分工仍有价值。不能把 skill 内容当成进程隔离或执行权限。[docs/extensions.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/extensions.md)、[docs/skills.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/skills.md#L43)、[src/core/skills.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts) |
| 默认工具发现控制提示体积 | codemode 把大量工具调用/过滤留在脚本内；MCP 默认间接发现，tool_search 按需声明工具 | 已实现，且 0.99.2/1.0.0 文档记录了最近的提示缓存及声明体积调整。并非“未来 MCP 支持”。[docs/codemode.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/codemode.md)、[docs/mcp.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md)、[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[src/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/index.ts) |
| 安全边界放在进程/OS 环境 | 文档区分完整 Pi 在隔离环境中运行与仅工具委派；项目 trust 只控制部分资源加载 | 当前明确的威胁模型，不是未完成内置 sandbox 的承诺。sessionDir 的提前读取、context files 始终加载都有文档限定。[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L3)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L31)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L57)、[docs/containerization.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/containerization.md#L9) |

这里“默认/公开支持路径”不表示所有扩展 API 都已获永久稳定性保证。文档中仍有局部实验 API 和兼容层，其分界要逐项判断。

### 25.3 services README：真实 roadmap、已完成底座与仍然开放的方向

这份 README 的核心动机，是把 Session worker 拥有的 durable conversation 与 presentation 分开：
控制通过服务 facade，状态通过 ConversationView/Chord 复制；工具执行、持久化和恢复不归 TUI。
它还试图让插件以 Session/TUI 等不同 host 的 facets 组织，通过服务 token 建立依赖与生命周期，避免把整个 Harness 或原始 TUI 当成远程服务暴露。
这些已有对应源码：worker 构建 AgentController、Models、Transcript 和 SessionPlugins facets；TUI 以 private bridge 连接这些服务，提供窄的本地 PresentationUI。
证据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L26)、[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L32)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L51)、[src/experimental/client-tui.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-tui.ts#L267)。

README 的 Continuation point 共 9 项。下表分别保留当前切片、尚未完成的部分和条件，不把全部句子压成一个笼统的“服务化尚未实现”。

| 规划/动机 | 当前源码与状态 | 对未来含义的判断 |
|---|---|---|
| SessionDirectory 按已认证客户端投影，避免所有客户端看到同一目录 | **当前目录已实现；身份投影尚无实现。** server 建立一份 directory replicated state，并把同一 state 提供给各 attachment。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L16)、[src/experimental/services/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/server.ts#L37)、[src/experimental/services/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/server.ts#L70) | 明确以 identity 落地为前提；不等同于已有 per-user ACL，也没有给出完成时间 |
| SessionManagement 的 workspace authorization | **创建/删除/attach/detach 已实现；该层 workspace 授权尚无实现。** 当前 provider 直接调用管理回调和 presentation 操作。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L17)、[src/experimental/services/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/server.ts#L90) | 保留为文档明确后续方向。现有本地目录权限、worker token/scope 不能替代这个要求 |
| PresentationPlugins 的认证 plugin policy | **分支插件选择和构建/重载已实现；认证策略尚无实现。** prepare/reload 在 server 回调链上执行，没有此 README 所述的身份策略参数。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L18)、[src/experimental/services/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/server.ts#L71) | 本地路径选择限制和分支隔离是已有边界，不应宣称已经形成多用户插件授权 |
| SessionPlugins 的跨 worker 协调重载及结果汇报 | **单 worker generation reload 已实现；多 worker 协调尚无实现。** 每个 worker 有独立 facetHost 与 reloadTail；/reload 顺序处理选中 presentation、Session 和当前 TUI。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L19)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L91)、[src/experimental/services/slash-commands-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/slash-commands-provider.ts#L104) | 现有能力不能读成全体 Session 同步切换，也不能从“atomically rebuilds”推导跨进程事务 |
| Models 将 provider/auth composition 下移到插件 facets | **部分实现。** Models 自身已有 facet，但 ModelRuntime/SettingsManager 由 host 创建并直接传入；服务操作再调用这些对象。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L20)、[src/experimental/services/models-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/models-provider.ts#L139)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L778) | “已有 Models facet”并不表示 Provider/认证的组成权已经插件化；此延伸仍是明确后续方向 |
| AgentController 仅在 presentation 需要时添加新 conversation 操作 | **现有 root facade 已实现；新增操作是条件性方向。** 当前 interface 包含 prompt、steer、followUp、cancelQueued、abort、compact、waitForPrompt。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L21)、[src/experimental/services/agent-controller.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/agent-controller.ts#L38) | 这是一项限制抽象扩张的设计约束，不是列出一批已承诺 API |
| Transcript 仅在第二种 presentation 需要时增添投影 | **当前 root ConversationView 已实现；额外投影无具体承诺。** provider 直接取得 conversation.viewState()，没有 worker 自有 transcript reducer。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L22)、[src/experimental/services/transcript-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/transcript-provider.ts#L6) | 需求驱动的扩展点；不能把其它包中的 canvas/diff-review 示例当成本包待交付界面 |
| SlashCommands 随具体 plugin slice 增加 hookpoints | **本地贡献注册表和内置命令已实现。** 定义为 local token；命令注册、替换和释放由 registry/facet 生命周期承担。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L23)、[src/experimental/services/slash-commands.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/slash-commands.ts#L22)、[src/experimental/services/slash-commands-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/slash-commands-provider.ts#L77) | 明确按实际插件需要增加，不表示默认 ExtensionAPI 全部 hooks 会被照搬 |
| PresentationUI 随命令需要添加窄能力 | **select/showStatus 已实现；其它能力未列出。** token 为 local，TUI bridge 提供实现。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L24)、[src/experimental/services/presentation-ui.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/presentation-ui.ts#L9)、[src/experimental/client-tui.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-tui.ts#L270) | 这是保持 UI 服务表面较窄的原则，不是通用远程 TUI API 规划 |

**TODO 段落中，迁移前提已过时，功能差距不能一并删去。**
README 第 40 行仍把后续项描述为等待 worker 从已删除 AgentHarness 移到 pi-durable；当前 worker 已调用 Harness.open(SQLite)、取得 root conversation，且启动时调用 harness.resume()。
因此迁移底座应判为**已实现，旧延期原因失效**，而不能据此说 services 整体过时或四个 TODO 全部完成。
证据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L40)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L784)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L804)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L594)。

| TODO 中的项目 | 分项核查 | 分类 |
|---|---|---|
| Tree navigation / navigate() | 当前 AgentController 无 navigate。文档描述需要 fork 新 conversation、summary 起点和 Session 当前 conversation 指针；当前服务仍固定 root，未连接这套导航政策 | **尚无服务实现；明确后续设计。** 不能把 durable 底层可 fork 等同于 TUI 服务导航已完成。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L42)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L24) |
| nextRun() | 当前 controller 只提供 steer/followUp；文档说旧方法已 dropped，并说明 durable 的队列分类 | **旧接口已移除/语义收窄。** 没有足够证据把它记成等待恢复的承诺。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L43)、[src/experimental/services/agent-controller.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/agent-controller.ts#L39) |
| 显式 resume() | 当前服务无此方法；worker 在打开 Session 时自动 resume | **被自动恢复机制替代。** 这是已完成机制变化，不是遗漏 API。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L43)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L593) |
| Subagents 的 keyed services | 当前 worker 只向 services 传一个 root conversation，Transcript 与 AgentController 都是 root facade；durable 独立演示已有子 conversation 也不会自动暴露为远程 keyed service | **本服务切片尚无实现。** keyed 机制存在不等于已有该功能的实例目录。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L44)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L51) |
| Transcript 旧历史分页 | interface 只有 replicated state，没有 entries paging 方法；它服务 active view，非全历史查询 API | **尚无服务实现；文档仍准确。** 文档建议以 Conversation.entries() 建分页，但没有页大小、游标或发布计划。[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L45)、[src/experimental/services/transcript.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/transcript.ts#L4) |

该 README 还有一处**范围过强**：开头称每个 host 的 RPC catalogue 都由 facet setup 生成、“没有手写内置清单”。
worker/TUI 的 FacetHost 装配支持这种方向，但当前 server 仍直接创建 RemoteServiceProvider 并列出三个 singleton token；非交互客户端也显式选择所需服务。
README 第 36 行自己承认 server provider 还在直接组装，等待完整 host environment。
因此应将首段读作 facets 主路径的机制说明及目标，不能作为 server 已全面统一的事实；后者是**部分实现**。
证据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L3)、[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L36)、[src/experimental/services/server.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/server.ts#L65)、[src/experimental/client-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-runtime.ts#L185)。

### 25.4 durable/vacation README：已实现的设计实验与明确非目标

两个 README 的价值是提供可运行入口和最小集成切片，观察 durable 语义如何穿过工具、子任务和 TUI。
它们既不是默认产品功能清单，也没有承诺把 SQLite/恢复机制整体替换到 AgentSession。
两者复用默认模型认证、设置和界面组件，与复用默认 agent/session 实现是不同事实。
证据：[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L3)、[src/experimental/vacation/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L3)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L52)、[src/experimental/vacation/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/harness-setup.ts#L42)。

| 文档设计 | 当前接线与判断 | 保留的边界 |
|---|---|---|
| durable：提交状态驱动一个 view，TUI 不处理恢复 | runtime 安装 CodingTools 和 Subagent，打开 Harness，订阅 view/task graph，调用 resume；README 的恢复设计有具体集成支撑 | **已实现的演示主链；原文概括需要限定。** README 的“every streamed partial/tool output”不能理解为每个输入 chunk 都立即完成独立 durable commit；底层 generation/output 有节流与在途提交，崩溃只能恢复已提交状态，也不保证固定 100 ms 丢失上限。本次未做 crash 或掉电演练。[原文](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L20)、[runtime](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L124)、[generation](../../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts)、[output](../../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts)、[durable 详细核查](durable.md) |
| durable：foreground subagent 由工具调用拥有 | registry 安装 Subagent；工具创建 child conversation，和调用生命周期绑定；/agents 可切换观察 | **已实现。** 与默认 examples/subagent 的独立 pi 进程是另一种实现。[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L26)、[src/experimental/durable/runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/runtime.ts#L134)、[src/experimental/durable/subagent.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/subagent.ts#L24) |
| durable：会话列表、resume picker、fork/tree、extensions、templates、images、/login “Not here” | README 明确排除，而 runtime 只装专用 registry；认证从 Pi 已有存储共享 | **演示的明确非目标。** “extensions”在此指默认扩展发现/ExtensionAPI，不是说 durable 没有 defineExtension 机制。不能据此推断主产品失去这些能力，或承诺以后补齐。[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L15)、[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L66)、[src/experimental/durable/harness-setup.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/harness-setup.ts#L52) |
| vacation：research 立即返回，后台 child 完成后把报告送回主 conversation | Research 任务以 conversation ownership、background:true 启动；deliver/report checkpoint 分开；报告作为 followUp 输入提交 | **已实现的后台任务模式。** 不是“规划中的旅行产品”。[src/experimental/vacation/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L27)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L62)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L112) |
| vacation：并行搜索、恢复和重复提交防护 | search 明确 replay:safe；不同 topic 使用模拟结果和延迟；research 与 report 采用由 task.id 生成的稳定 requestId | **已实现的幂等演示。** canned search 不是缺失真实搜索的自动承诺；只读源码也不代表实际恢复实验通过。[src/experimental/vacation/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L30)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L35)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L79)、[src/experimental/vacation/vacation.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/vacation.ts#L97) |

services README 引用的 canvas、diff-review、task-observation 被原文明确定位为扩展模式，而非内置 coding-agent services。
它们的保留价值在于验证跨包接口能承载哪些 presentation，不足以证明本包已有这些界面、排期或默认启用计划。
证据：[src/experimental/services/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/README.md#L36)。

### 25.5 扩展、SDK、MCP 与 plugins 的支持边界

| 文档中的名称 | 当前所属路径 | 时效判断 |
|---|---|---|
| Extensions、Pi Packages、skills/templates | 默认同进程 ExtensionAPI 和资源加载；packages 使用 pi 资源 manifest | **当前公开产品机制。** docs/packages 的 package 不等于 chord.facets 插件包。[docs/extensions.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/extensions.md)、[docs/packages.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/packages.md#L3)、[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts) |
| SDK / stdio RPC | npm 根 export 和 RpcClient；CLI 的 RPC 是同一默认会话的 JSONL 子进程接口 | **当前公开支持入口。** CHANGELOG 明确在收回实验发布时保留它们；支持不意味着每个 example snippet 都无过时内容。[docs/sdk.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L3)、[docs/rpc.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/rpc.md#L3)、[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json) |
| MCP / codemode / tool_search | 默认 CLI 内置 extensions；SDK 需显式加 factories 并绑定生命周期 | **已实现，不属于 source-only services。** SDK 未自动装入不是 roadmap 缺口，而是当前装配契约。[docs/sdk.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/sdk.md#L116)、[docs/mcp.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md)、[src/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/index.ts) |
| Virtual models | ModelRuntime 与 AgentSession 的默认本地链，扩展可注册；0.99.0 发布说明仍称 experimental | **已实现但被标实验的 API。** 源码有注册、路由、会话状态接线，不能把“experimental”统一解释为需要 server/client 或 PI_EXPERIMENTAL 才可用；也不能仅凭 docs 页面不再重复标签就断言已正式稳定。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[docs/virtual-models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/virtual-models.md#L61)、[src/core/model-runtime.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-runtime.ts#L955)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts) |
| experimental/plugin、client、Session/TUI Chord facets | source 条件入口，normal build/publish 排除相关源码产物 | **已实现的源码实验，未进入默认 npm API。** 示例展示的是当前构建/分发/热重载模式，不是未来“将支持 plugins”的空设计。[examples/plugins/pi-example-plugin/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L3)、[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[tsconfig.build.json](../../../../.repo/pi-1.0.4/packages/coding-agent/tsconfig.build.json#L19) |
| durable/vacation main | 独立源文件入口及 source resolver；各自 SQLite 会话目录 | **设计演示。** 不通过默认 CLI SDK 自动获得其恢复语义。[src/experimental/durable/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/durable/README.md#L7)、[src/experimental/vacation/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/vacation/README.md#L14) |

插件示例的设计动机有三个已经实现的部分：同一包拆成不同 host facet；Session 分支保存插件选择，避免改变其它 worker；/reload 生成候选代并在 host 内替换、释放旧代。
服务的未来方向是认证策略与多 worker 协調，不是重新实现上述基础。
当前 /reload 有 presentation package rebuild、Session reload、TUI reload 三次顺序等待，因此文档的原子构建/代际替换表述不能扩大为跨所有进程同时提交、失败全部回滚。
证据：[examples/plugins/pi-example-plugin/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L21)、[examples/plugins/pi-example-plugin/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/plugins/pi-example-plugin/README.md#L25)、[src/experimental/services/slash-commands-provider.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/slash-commands-provider.ts#L104)、[src/experimental/services/worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/services/worker.ts#L94)、[src/experimental/client-tui.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/client-tui.ts#L246)。

### 25.6 旧说明与当前源码冲突、未完成声明的逐项判断

**明确过时的 SDK README 片段。**
examples/sdk/README 的 Quick Reference 从 pi-ai 根导入 getModel 并直接调用；当前 pi-ai 根入口只导出 core，旧全局 getModel 位于 compat。
实际 SDK 例子 02-custom-model 已通过 ModelRuntime.getModel 获取模型，因此 README 片段落后于相邻源码。
ExtensionLoader 将 pi-ai 根映射到 compat 是扩展专属运行时别名，不能据此认定普通 SDK 进程的根 import 正确。
这是静态可确认的文档/API 冲突；本次没有运行该片段。
证据：[examples/sdk/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/sdk/README.md#L36)、[examples/sdk/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/sdk/README.md#L51)、[examples/sdk/02-custom-model.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/sdk/02-custom-model.ts#L12)、[packages/ai/src/index.ts](../../../../.repo/pi-1.0.4/packages/ai/src/index.ts#L4)、[packages/ai/src/compat.ts](../../../../.repo/pi-1.0.4/packages/ai/src/compat.ts#L65)、[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts)。

| 文档条目 | 与现状对照 | 判定 |
|---|---|---|
| 0.80.0 说明 compat 入口及 loader alias 将在未来删除并提供迁移指南 | 当前 ai/package.json 仍有 ./compat；coding-agent loader 仍把根映射到 compat | **历史未来承诺仍未完成；无日期。** 可以记录为未落地方向，不能宣称 1.0 已移除，也不能替维护者承诺下一版一定完成。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[packages/ai/package.json](../../../../.repo/pi-1.0.4/packages/ai/package.json)、[src/core/extensions/loader.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/loader.ts) |
| 0.84.0 的 AgentHarness v2 scaffold、lane v4 Session/Repo 与旧 JSONL API removal | 当前 agent 根入口只导出 Agent/loop/proxy/stream/types；实验 worker 已用 pi-durable Harness；默认 SessionManager 仍为自身 JSONL 树 | **历史架构已被替代。** 这些旧发行记录不能当成当前 AgentHarness 尚待完成的设计，也不能推导默认 coding-agent JSONL 已弃用。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[packages/agent/src/index.ts](../../../../.repo/pi-1.0.4/packages/agent/src/index.ts#L1)、[src/experimental/session-worker.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/experimental/session-worker.ts#L784) |
| 0.85.1 收回误发布的 experimental 子路径 | 当前 manifest 只给 source 条件，build 排除 client/experimental；没有正常 import fallback | **发布范围修正仍有效。** 是明确支持边界，不是实现缺失。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[package.json](../../../../.repo/pi-1.0.4/packages/coding-agent/package.json)、[tsconfig.build.json](../../../../.repo/pi-1.0.4/packages/coding-agent/tsconfig.build.json#L19) |
| 0.99.0–1.0.0 的 MCP/codemode、默认间接曝光、提示精简与图片模型 | 当前 MCP/codemode 文档和工厂导出均描述实际接线，服务 README 另属实验栈 | **已交付的默认扩展能力。** 历史版本“Added”不能继续留在未来列表；CHANGELOG 的性能数字是项目记录，本次没有复测。[CHANGELOG.md](../../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md)、[docs/codemode.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/codemode.md) |
| models.md：maxRequestBytes / images.maxPerMessage / maxPerRequest 尚不重写或拒绝历史 | 配置 schema 接受这些 metadata；当前新图片处理取 images.resize，检索未找到上述 hard-limit 字段的历史执法路径 | **明确尚未实施的策略，不是已排期功能。** 保留 metadata ≠ enforce；现有 resize 限制与整体 request 限制应分开。[docs/models.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/models.md)、[src/core/model-config.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-config.ts)、[src/core/agent-session.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts) |
| skills.md 将 allowed-tools 描述为 experimental pre-approved tool list | 该表说明 Agent Skills 规格；本包 SkillFrontmatter 允许额外字段，但构建 Skill 对象只提取 name/description/disableModelInvocation 等，未建立允许工具清单 | **规格/实现边界需补充，不宜认定已实现预批准。** 未发现本包执法路径，也没有本包承诺何时实现；不把规格字段自动纳入 roadmap。[docs/skills.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/skills.md#L69)、[docs/skills.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/skills.md#L78)、[src/core/skills.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts)、[src/core/skills.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts) |
| RPC UI 的 custom、terminal input、themes、header/footer 等降级 | rpc-mode 对相应方法返回 undefined、空值/no-op 或 setTheme 失败 | **当前明示非支持范围。** 没有证据说明将把完整 terminal component API 远程化。[docs/rpc-extension-ui.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/rpc-extension-ui.md#L14)、[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L228)、[src/modes/rpc/rpc-mode.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/rpc/rpc-mode.ts#L290) |
| MCP legacy SSE、MCP Apps UI 资源不支持 | 文档明确仅 stdio/streamable HTTP，省略需 UI 渲染的资源；conformance 指南另列 elicitation、stateless 等已知范围；其 CIMD 描述需单独更正，见下行 | **现行限制/测试范围，不自动构成未来承诺。** protocol 日期和 baseline 只代表固定套件对象，不能推成覆盖任意更新规范。[docs/mcp.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md)、[test/mcp-conformance/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-conformance/README.md#L13)、[test/mcp-conformance/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-conformance/README.md#L39) |
| conformance README 把 `auth/basic-cimd` 归因为 Pi 仅动态注册、未用 CIMD | 当前 `oauth.clientRegistration: "cimd"` 已实现，支持检查、metadata document、callback 及注册方式切换均有实现和静态测试证据；未显式启用时仍使用 DCR | **旧说明已不能表示整个产品的支持边界。** 该 README 哈希未变，baseline 在默认 DCR 配置下仍可能有预期失败；本次没有运行 conformance，也不宣称该 case 已通过。应说明具体测试配置，而非继续断言不支持 CIMD。[test/mcp-conformance/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-conformance/README.md#L42)、[docs/mcp.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md)、[src/extensions/mcp/oauth.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/oauth.ts#L240)、[test/mcp-oauth-refresh.test.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-oauth-refresh.test.ts#L186) |

概念概述也必须保留详细文档的例外：how-pi-works 说 trust 决定后才加载 project settings/resources，security/configuration 则明确 sessionDir 在 trust 前读取，context files 不受此 gate 限制。
这属于**概述压缩了范围**，以详细说明和实际启动链为准；不足以说整个 trust 文档已失效。
证据：[docs/how-pi-works.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/how-pi-works.md#L49)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L31)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L57)、[docs/configuration.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/configuration.md#L3)。

### 25.7 示例中的“plan”不是项目路线图

plan-mode README 描述的是用户可切换的只读探索、Plan: 步骤提取和 DONE 标记追踪。
实现关闭 edit/write，保留其它 active tools，并在 tool_call 上过滤 bash；它是当前扩展示例。
“只读”不能扩展成无条件系统沙箱：该实现保留其它 active tools，而 hook 只匹配 bash，第三方工具或其它执行通道不会因为这个名称自动受限。
这些边界与 security 文档的同进程权限模型相符，不能因为示例列出安全命令就提升为内置安全保证。
证据：[examples/extensions/plan-mode/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/plan-mode/README.md#L5)、[examples/extensions/plan-mode/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/plan-mode/README.md#L39)、[examples/extensions/plan-mode/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/plan-mode/index.ts#L24)、[examples/extensions/plan-mode/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/plan-mode/index.ts#L164)、[docs/security.md](../../../../.repo/pi-1.0.4/packages/coding-agent/docs/security.md#L3)。

subagent 的 planner.md 是委派角色提示，scout-and-plan.md 是把用户 $@ 输入送入 scout→planner 的 workflow template。
其中“创建 implementation plan”“Do NOT implement”等是被演示 Agent 的任务指令，并非维护者对 Pi 下一阶段的工作承诺。
subagent README 的并行 8 项/并发 4 项、独立 pi 进程、信任确认与取消说明也有具体实现，并不是 durable keyed services roadmap 已完成的证据。
证据：[examples/extensions/subagent/agents/planner.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/agents/planner.md#L8)、[examples/extensions/subagent/prompts/scout-and-plan.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md#L4)、[examples/extensions/subagent/README.md](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/README.md#L55)、[examples/extensions/subagent/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/index.ts#L33)、[examples/extensions/subagent/index.ts](../../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/subagent/index.ts#L346)。

同理，vacation 中的 planning 是旅行用户目标；compaction 文档中的 Next Steps 是摘要模板；quickstart 中的 Next steps 是阅读导航。
标题含 plan/future/next 只能作为检索线索，不能单独决定文件性质。

### 25.8 1.0.4 未变规划的再判断

services README 全文未变，worker/services 合约与演示装配未变：认证目录投影、workspace/plugin 授权、多 worker reload、provider/auth 下沉仍是原有方向；root-only、旧历史分页、子 conversation 服务化仍无完整接线。新 env 包没有兑现这些服务政策。旧“等 worker 迁移到 durable”前提依旧失效；自动 resume 是已有替代机制，不能整体宣布 TODO 完成。

默认 JSONL、session-format、compaction、稳定 RPC 文档和核心实现未变；新工具/prompt 语义经 AgentSession 生效，无证据改成 SQLite。旧 SDK getModel、compat 别名、skills allowed-tools 与 MCP conformance CIMD 归因的边界继续成立；本轮未改变相关 exports/loader/conformance 文件。Azure providers 环境配置仍有效，custom-provider 的 Responses 实现名仍有效；pi-ai README 的“Azure 只用 Responses”已不足以描述整个 provider。新增工具选择/guidelines/图片/按键说明有当前调用方，是已兑现行为而非路线图。


### 25.9 这些设计方向仍保留什么价值

以下是根据文档动机与当前实现作出的**研究判断**，不新增项目承诺、交付日期或开发任务：

- **认证后的会话投影与 workspace/plugin 授权**仍是服务架构最明确的未完成方向：当前共享目录和宿主权限模型已经存在，文档要求解决的是多客户端身份进入后的资源可见性和操作授权，不能靠现有 token/scope 替代。
- **conversation 作为服务实例边界**仍有解释力：root-only controller/view 已能工作，而子 conversation、tree 导航和旧历史分页尚未通过远程契约暴露；独立 durable 演示可以证明底层的可能性，却不能替代 service contract。
- **提交驱动的呈现与 checkpoint/requestId**是两份 durable 演示的主要设计证据：主界面不重建恢复状态；后台任务把提交与报告分开。该价值已经能从真实集成看见，不需要包装成未来产品清单。
- **窄的 facet 服务与按需求扩张的 UI hooks**仍是 README 明示的设计约束；当前 server 直接组装的例外表明统一 host 模型只完成一部分，不能用目标架构覆盖现状。
- **资源描述与执行政策分开**有持续的解释价值：hard request metadata、allowed-tools 规格字段、MCP annotations 都不自动成为授权或限额执法；状态应由实际消费它们的路径判断。
- **历史 compat 的删除意图仍可追溯，但日期与范围未重新确认**。当前兼容别名同时解释了旧扩展示例为何可能继续工作，以及普通 SDK README 为什么不能沿用同样假设。

本节把文档清单、服务 roadmap 的条件、SDK 说明片段、迁移前提及历史未来承诺分别核对。第 24 节未验证项仍有效，文档全文阅读不等于全部示例实现或每个回归分支都完成审计。
测试影响为 `none`；新增的验证仅为报告链接目标/行号和文本自检，没有执行项目测试。
