# Pi 1.0.2 → 1.0.4 升级分析

> 比较日期：2026-10-07。本文是完整新版报告的变化说明，不能代替 [整体报告](README.md) 和 14 份包报告。只使用本地固定快照和已保全的历史报告证据。

## 1 对象、方法与证据强度

| 对象 | 1.0.2 基线 | 1.0.4 当前 |
|---|---|---|
| 一级包 | 13，均为1.0.2 | 14，均为1.0.4，新增env |
| 根private版本 | 0.0.3 | 0.0.3 |
| 清单文件 | 1,859 | 1,908 |
| 文本资料 | 125份 / 33,781行 | 128份 / 33,270行 |
| 源码树SHA-256 | `f0309990522b4c47b96e19ec684c53fd1c8fd8a5bc7918ba70ec3289bf3f13fb` | `7e74bf510f6a2ced24656f8a03f576e01067d66fbe2e89bafb9c33059593de02` |

路径与摘要比较得到 **52新增、148修改、3删除、1,708不变**。文本资料为 **4新增、18修改、1删除、106不变**；行数减少主要包含删除1,001行的根TUI计划，不代表分析范围收缩。

比较口径为 `.git/node_modules/dist` 排除，文档扩展名 `.md/.mdx/.rst/.txt`。当前清单拒绝 symlink/reparse point；旧清单没有同样的链接策略字段，不能补作历史保证。新版 [清单](source-manifest.json) 与 [旧清单](evidence/pi-1.0.2/source-manifest.json) 均经工具自洽验证。两次树摘要不能互换成 Git revision 或发布签名。

旧源码不在工作区，因此这里不是完整逐行 diff。路径变化由摘要证实；语义解释由旧报告结论、当前实现/测试源码与1.0.3、1.0.4发行条目共同支持。只有当前代码证据时写当前事实；发行记录能确定作者归属的版本，不证明本次测试通过。同内容文件继承审读深度，仍重查受变化依赖影响的关键调用链。恢复依据见 [历史证据](evidence/pi-1.0.2/README.md)。

## 2 主要语义变化

### 2.1 环境抽象成为独立远端实现的接缝

durable 1.0.3 要求自定义 FileSystem 实现 binary/dir reader、scanLines、watch，Shell.exec 支持字符串与 argv，onOutput 接收 stream/skipped 信息。1.0.4 又收紧 watch permission_denied 和一致性预期。版本都是patch并不说明契约没有破坏：CHANGELOG明确列 Breaking Changes，应按当前接口判断。[durable发行记录](../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md#L3)、[环境契约](../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts)。

新增env把这些机制放到远端：TS处理请求关联/路径/高层语义，Rust处理文件、进程、watch、spill和扫描，SSH helper探测并安装匹配daemon。其协议与Pi Session的CBOR服务协议不同；默认CLI与experimental worker仍用已有本地路径，未自动迁移。不能写成“Pi默认工具现在都远程执行”。[env报告](packages/env.md)、[应用接线](packages/coding-agent.md)。

### 2.2 有界输出与有界读取需要跨层一致

read从全量文件读取转为头部、一次环境内行扫描、实际展示区间读取；1.0.4容忍读取中的追加，截短/原位改写另处理。输出窗口允许远端省略调用方不会保留的内容，并用skipped维护总计；progress默认100ms与写入预算并行作用。它降低主机传输和保留成本，不等于每次I/O都为原子快照，也不等于100ms恢复点保证。

PowerShell是新导出的独立tool工厂，默认coding工具集合是否包含它须看工厂代码。durable read仍不支持图片；本版coding-agent/codemode图片桥接不能算成该方向已兑现。[durable报告](packages/durable.md)。

### 2.3 Azure身份与请求格式分开演进

新增providers/azure.ts与azure.models.ts、api/azure-openai-config.ts，删除旧azure-openai-responses provider文件；新provider key为azure，Responses协议标识仍为azure-openai-responses，Chat Completions为另一API。对旧Provider key的消费者，名称变化与协议名称保留需要分别理解，不能全局字符串替换。AI README的“only Responses”已落后，产品API配置示例中同名字串仍可正确。[AI报告](packages/ai.md)。

OAuth refresh获得锁并开始后使用独立timeout，保存轮换token不再随外层取消而丢弃；外层仍可及时结束等待。MCP产品关闭需等待auth后台工作等收束，避免关闭与凭据落盘竞争。retry classifier新增HTTP2 pending cancellation、暂时subscription信息不可用和额度耗尽的区别，实际重试仍由上层预算控制。

### 2.4 工具可见性、可执行性与提示必须一起更新

CLI支持tool星号模式和--no-mcp；--tools未带mcp__前缀时保留MCP集合。prepareLoadout隐藏声明之后，system prompt规则、skills hint以及codemode声明的guidelines同步变化。MCP连接关闭会等待仍在连接中的transport；DCR根据redirect类型选择application_type，显式配置可覆盖。[coding-agent](packages/coding-agent.md)、[MCP](packages/mcp.md)。

codemode通用库冻结内建对象、加强host消息收束；产品层补模型参数诊断、并发限制、图片结构化结果和私有输出文件。worker被缓存为内存data URL并单独bundle，减少运行中安装目录被更新移除造成的失败；它不能保住所有尚未加载的资源。[codemode](packages/codemode.md)、[应用](packages/coding-agent.md)、[bundle脚本](../../../.repo/pi-1.0.4/scripts/build-coding-agent-bundle.mjs#L221)。

### 2.5 不变包仍有受影响关系

Agent六个src文件未变，默认模型和auth行为仍受ai及coding-agent装配影响。Chord源码未变，env增加Context消费者并不实现planned RpcPeer。client/protocol/server当前代码不变，env另有stdio协议并未取代它们。evals的模型名称/版本变化不能推导新评测结果。telemetry旧Agent导出示例也没有因版本号更新而恢复有效。[各包报告](README.md#3-全部包与规模)。

## 3 逐包差异与复核重点

| 包 | 新增 | 修改 | 删除 | 不变 | 本轮语义判断 |
|---|---:|---:|---:|---:|---|
| [agent](packages/agent.md) | 0 | 2 | 0 | 18 | 核心 src/test 不变；manifest/发行标题跟随版本，ai 的鉴权和模型语义可影响实际运行。 |
| [ai](packages/ai.md) | 4 | 30 | 2 | 347 | Azure provider 改名与双 API；OAuth refresh 取消边界；retry classifier；生成目录及对应 provider 测试。 |
| [chord](packages/chord.md) | 0 | 1 | 0 | 63 | 仅 manifest；当前服务/delta/facet 实现继承，env 新消费者只用 Context/取消语义。 |
| [client](packages/client.md) | 0 | 2 | 0 | 17 | manifest/发行标题；当前 Session client 与服务边界未改。 |
| [codemode](packages/codemode.md) | 1 | 5 | 0 | 13 | 冻结 VM built-ins/globals，host 消息错误收束和 done/store 校验，raw-worker 反例。 |
| [coding-agent](packages/coding-agent.md) | 1 | 56 | 0 | 804 | MCP/tool pattern、loadout提示、图片/输出文件、更新后worker、OAuth关闭/UI/默认模型等。 |
| [durable](packages/durable.md) | 8 | 26 | 0 | 129 | 环境接口扩展与conformance；scan/watch、argv、输出窗口和进度；PowerShell工厂与读文件竞争修复。 |
| [env](packages/env.md) | 36 | 0 | 0 | 0 | 新增全部36文件；TS远端适配器、SSH bootstrap、Rust daemon、协议/语义说明及测试。 |
| [evals](packages/evals.md) | 0 | 1 | 0 | 29 | 仅 manifest；评测实现和README继承，依赖中的产品/模型变化不等于重新评测。 |
| [mcp](packages/mcp.md) | 0 | 5 | 0 | 27 | DCR application_type 默认选择及显式覆盖；OAuth回归测试。 |
| [protocol](packages/protocol.md) | 0 | 2 | 0 | 15 | manifest/发行标题；CBOR Session协议不因env另有协议而改变。 |
| [server](packages/server.md) | 0 | 2 | 0 | 27 | manifest/发行标题；SessionHandle、租约、listener仍由宿主接线。 |
| [telemetry](packages/telemetry.md) | 0 | 2 | 0 | 10 | manifest/发行标题；未恢复Agent已删除的schemas，README旧跨包示例仍陈旧。 |
| [tui](packages/tui.md) | 0 | 5 | 0 | 116 | Home/End保留编辑器行首尾，Ctrl+Home/End改为全屏transcript导航，相关测试同步；主渲染/布局代码继承。 |

这些计数按一级包路径归属，嵌套扩展示例的manifest/lock仍计入coding-agent。根目录/隐藏目录的构建、发布、配置和删除计划另在下节及完整路径清单呈现。未删除一级包；旧13份子报告均承接，新env有独立报告。

## 4 包外构建、发布与依赖输入

根package.json把env插入durable之后的build/build:offline序列，tsconfig增加pi-env源码路径；local-release将env纳入打包清单。CI先构建Rust daemon再运行workspace tests；release workflow等待env-daemons并收集8个平台/架构产物。普通env build仅tsc，不能推导源码树已有daemon可执行文件。[根manifest](../../../.repo/pi-1.0.4/package.json)、[CI](../../../.repo/pi-1.0.4/.github/workflows/ci.yml)、[local-release](../../../.repo/pi-1.0.4/scripts/local-release.mjs#L9)。

**静态前置缺口：** env测试helper要求daemon已存在；local-release/release没有显式构建daemon，test.sh又不转交PI_ENV_DAEMON。由干净源码按原流程运行时，还需预先构建默认debug路径的daemon；否则到该helper会报缺失。这里是调用链条件推导，没有运行失败记录，也不推断维护者已准备daemon的环境必然失败。[helper](../../../.repo/pi-1.0.4/packages/env/test/daemon.ts)、[test.sh](../../../.repo/pi-1.0.4/test.sh#L40)、[release](../../../.repo/pi-1.0.4/scripts/release.mjs#L256)。

新env-daemons发布8类资产，Environments运行矩阵覆盖6个非Android产物，另有Node/Rust一致性、SSH和pack清单检查。Android产物构建不等于Android运行测试，Intel macOS用Rosetta也不等于本机Intel硬件验证。[构建矩阵](../../../.repo/pi-1.0.4/.github/workflows/env-daemons.yml)、[测试矩阵](../../../.repo/pi-1.0.4/.github/workflows/env.yml)。

根/托管安装/嵌套示例lockfile摘要改变。按新旧manifest保存字段比较，旧13包的直接external dependencies/devDependencies没有版本增删；内部依赖跟随1.0.4，新env增加chord/durable。旧lock正文缺失，因此未宣称所有传递项都只是版本替换，也未审计外部依赖源码。新daemon拥有Cargo.lock、Rust toolchain和build.rs，是独立原生构建边界。[env构建](packages/env.md)。

`nix/model-catalog.json`新revision为`sha256-c5d5070c7592ca8e27743e892a7eec1d6c883be8034f888735238cdfdb3ab70f`；flake与Nix workflow字节未变。这是固定模型数据输入变更，不是重新实现Nix。root .gitignore增加env bin/target资产排除；本研究的清单排除口径仍是显式三个目录名，不能用.gitignore猜测源码范围。

## 5 旧结论与规划如何处置

| 原来可能采用的结论 | 1.0.4处理 |
|---|---|
| 只有13个一级包 | 改为14个，单独解释env的TS与Rust两部分 |
| Azure provider只有Responses | 局部陈旧；provider变为azure，旧API ID仍有意义 |
| caller abort应停止OAuth refresh全部工作 | 改为锁前取消与锁后轮换持久化分开 |
| default主请求依赖全局compat stream | SDK仍有兼容注册，但Agent显式注入ModelRuntime stream；不可只据import断言主链 |
| 自定义ExecutionEnv只实现原FS/shell接口 | 有明确Breaking Changes；新readers/watch/scan/argv/output info均为当前契约 |
| 固定100ms可以界定崩溃丢失窗口 | 不能保证；节流、在途commit及存储延迟共同影响 |
| durable只有四个工具所以没有PowerShell能力 | 默认coding工具集合与单独PowerShell工厂分开；图片read仍未支持 |
| 当前TUI路线图仍是根tui-plan.md | 路径已删；保全旧二手分析，只作历史方向，当前承诺未确认 |
| telemetry Agent schema示例有效 | 当前Agent导出仍没有这些schemas；版本标题不能证实示例 |
| 有env就代表CLI已远程化 | 未发现产品接线；NodeExecutionEnv仍是experimental宿主的实现 |

详细文档覆盖和逐主张时效见 [documentation-audit.md](documentation-audit.md)。106份未改文档不是自动“仍全部正确”，Azure旧说明、telemetry示例、experimental服务TODO都需要结合当前代码重新判定。4份env新文档也不是天然无误：协议描述中的队列/窗口保证仍要按具体实现和操作类型收窄。

`tui-plan.md`删除的直接事实由清单差异确认；原文无法从本地旧源码恢复。旧报告中关于virtualization、overlay、scrollbar与contain的分析已绑定固定Git对象和旧source hash，保存在[历史计划证据](evidence/pi-1.0.2/README.md#tui-plan)。不把“删除”解释为作者承诺的延续或撤回。

## 6 全量变化路径

下面覆盖全部203个变化路径，新增和修改链接到本版实际文件；删除路径只作历史标识。类别用于追踪复核范围，并不声称每个生成数据、测试用例和平台分支都逐行审计。具体深度见子报告与文档审读记录。1,708个不变路径由两份manifest的路径+SHA承载。

### 6.1 新增（52）

| 路径 | 复核类别 |
|---|---|
| [.github/workflows/env-daemons.yml](../../../.repo/pi-1.0.4/.github/workflows/env-daemons.yml) | 构建／发布／工作区输入 |
| [.github/workflows/env.yml](../../../.repo/pi-1.0.4/.github/workflows/env.yml) | 构建／发布／工作区输入 |
| [packages/ai/src/api/azure-openai-config.ts](../../../.repo/pi-1.0.4/packages/ai/src/api/azure-openai-config.ts) | 实现／调用链 |
| [packages/ai/src/providers/azure.models.ts](../../../.repo/pi-1.0.4/packages/ai/src/providers/azure.models.ts) | 模型目录／生成数据 |
| [packages/ai/src/providers/azure.ts](../../../.repo/pi-1.0.4/packages/ai/src/providers/azure.ts) | 实现／调用链 |
| [packages/ai/test/azure-openai-completions.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/azure-openai-completions.test.ts) | 测试源码／夹具 |
| [packages/codemode/test/fixtures/raw-worker.ts](../../../.repo/pi-1.0.4/packages/codemode/test/fixtures/raw-worker.ts) | 测试源码／夹具 |
| [packages/coding-agent/src/utils/output-files.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/output-files.ts) | 实现／调用链 |
| [packages/durable/src/env/decode.ts](../../../.repo/pi-1.0.4/packages/durable/src/env/decode.ts) | 实现／调用链 |
| [packages/durable/src/env/line-scan.ts](../../../.repo/pi-1.0.4/packages/durable/src/env/line-scan.ts) | 实现／调用链 |
| [packages/durable/src/env/node-watch.ts](../../../.repo/pi-1.0.4/packages/durable/src/env/node-watch.ts) | 实现／调用链 |
| [packages/durable/src/testing/env-conformance.ts](../../../.repo/pi-1.0.4/packages/durable/src/testing/env-conformance.ts) | 实现／调用链 |
| [packages/durable/test/env-line-scan.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/env-line-scan.test.ts) | 测试源码／夹具 |
| [packages/durable/test/env-node-conformance.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/env-node-conformance.test.ts) | 测试源码／夹具 |
| [packages/durable/test/harness-output-skip.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/harness-output-skip.test.ts) | 测试源码／夹具 |
| [packages/durable/test/tools-read-differential.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/tools-read-differential.test.ts) | 测试源码／夹具 |
| [packages/env/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/env/CHANGELOG.md) | 文档／规划／历史 |
| [packages/env/README.md](../../../.repo/pi-1.0.4/packages/env/README.md) | 文档／规划／历史 |
| [packages/env/daemon/Cargo.lock](../../../.repo/pi-1.0.4/packages/env/daemon/Cargo.lock) | 锁文件／依赖输入 |
| [packages/env/daemon/Cargo.toml](../../../.repo/pi-1.0.4/packages/env/daemon/Cargo.toml) | 包契约／构建配置 |
| [packages/env/daemon/build.rs](../../../.repo/pi-1.0.4/packages/env/daemon/build.rs) | 包契约／构建配置 |
| [packages/env/daemon/rust-toolchain.toml](../../../.repo/pi-1.0.4/packages/env/daemon/rust-toolchain.toml) | 包契约／构建配置 |
| [packages/env/daemon/src/decode.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/decode.rs) | 实现／调用链 |
| [packages/env/daemon/src/errors.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/errors.rs) | 实现／调用链 |
| [packages/env/daemon/src/exec.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/exec.rs) | 实现／调用链 |
| [packages/env/daemon/src/frame.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/frame.rs) | 实现／调用链 |
| [packages/env/daemon/src/fs.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/fs.rs) | 实现／调用链 |
| [packages/env/daemon/src/main.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/main.rs) | 实现／调用链 |
| [packages/env/daemon/src/output.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/output.rs) | 实现／调用链 |
| [packages/env/daemon/src/scan.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/scan.rs) | 实现／调用链 |
| [packages/env/daemon/src/sys/mod.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/sys/mod.rs) | 实现／调用链 |
| [packages/env/daemon/src/sys/unix.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/sys/unix.rs) | 实现／调用链 |
| [packages/env/daemon/src/sys/windows.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/sys/windows.rs) | 实现／调用链 |
| [packages/env/daemon/src/watch.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/watch.rs) | 实现／调用链 |
| [packages/env/daemon/src/window.rs](../../../.repo/pi-1.0.4/packages/env/daemon/src/window.rs) | 实现／调用链 |
| [packages/env/docs/protocol.md](../../../.repo/pi-1.0.4/packages/env/docs/protocol.md) | 文档／规划／历史 |
| [packages/env/docs/semantics.md](../../../.repo/pi-1.0.4/packages/env/docs/semantics.md) | 文档／规划／历史 |
| [packages/env/package.json](../../../.repo/pi-1.0.4/packages/env/package.json) | 包契约／构建配置 |
| [packages/env/src/connection.ts](../../../.repo/pi-1.0.4/packages/env/src/connection.ts) | 实现／调用链 |
| [packages/env/src/errors.ts](../../../.repo/pi-1.0.4/packages/env/src/errors.ts) | 实现／调用链 |
| [packages/env/src/index.ts](../../../.repo/pi-1.0.4/packages/env/src/index.ts) | 实现／调用链 |
| [packages/env/src/remote-env.ts](../../../.repo/pi-1.0.4/packages/env/src/remote-env.ts) | 实现／调用链 |
| [packages/env/src/ssh.ts](../../../.repo/pi-1.0.4/packages/env/src/ssh.ts) | 实现／调用链 |
| [packages/env/src/watch.ts](../../../.repo/pi-1.0.4/packages/env/src/watch.ts) | 实现／调用链 |
| [packages/env/test/conformance.test.ts](../../../.repo/pi-1.0.4/packages/env/test/conformance.test.ts) | 测试源码／夹具 |
| [packages/env/test/daemon.ts](../../../.repo/pi-1.0.4/packages/env/test/daemon.ts) | 测试源码／夹具 |
| [packages/env/test/differential.test.ts](../../../.repo/pi-1.0.4/packages/env/test/differential.test.ts) | 测试源码／夹具 |
| [packages/env/test/remote.test.ts](../../../.repo/pi-1.0.4/packages/env/test/remote.test.ts) | 测试源码／夹具 |
| [packages/env/test/ssh-external.test.ts](../../../.repo/pi-1.0.4/packages/env/test/ssh-external.test.ts) | 测试源码／夹具 |
| [packages/env/test/ssh.test.ts](../../../.repo/pi-1.0.4/packages/env/test/ssh.test.ts) | 测试源码／夹具 |
| [packages/env/tsconfig.build.json](../../../.repo/pi-1.0.4/packages/env/tsconfig.build.json) | 包契约／构建配置 |
| [packages/env/vitest.config.ts](../../../.repo/pi-1.0.4/packages/env/vitest.config.ts) | 包契约／构建配置 |

### 6.2 修改（148）

| 路径 | 复核类别 |
|---|---|
| [.github/workflows/build-binaries.yml](../../../.repo/pi-1.0.4/.github/workflows/build-binaries.yml) | 构建／发布／工作区输入 |
| [.github/workflows/ci.yml](../../../.repo/pi-1.0.4/.github/workflows/ci.yml) | 构建／发布／工作区输入 |
| [.gitignore](../../../.repo/pi-1.0.4/.gitignore) | 构建／发布／工作区输入 |
| [nix/model-catalog.json](../../../.repo/pi-1.0.4/nix/model-catalog.json) | 构建／发布／工作区输入 |
| [package-lock.json](../../../.repo/pi-1.0.4/package-lock.json) | 锁文件／依赖输入 |
| [package.json](../../../.repo/pi-1.0.4/package.json) | 构建／发布／工作区输入 |
| [packages/agent/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/agent/CHANGELOG.md) | 文档／规划／历史 |
| [packages/agent/package.json](../../../.repo/pi-1.0.4/packages/agent/package.json) | 包契约／构建配置 |
| [packages/ai/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/ai/CHANGELOG.md) | 文档／规划／历史 |
| [packages/ai/package.json](../../../.repo/pi-1.0.4/packages/ai/package.json) | 包契约／构建配置 |
| [packages/ai/scripts/generate-models.ts](../../../.repo/pi-1.0.4/packages/ai/scripts/generate-models.ts) | 实现／调用链 |
| [packages/ai/src/api/azure-openai-responses.ts](../../../.repo/pi-1.0.4/packages/ai/src/api/azure-openai-responses.ts) | 实现／调用链 |
| [packages/ai/src/auth/resolve.ts](../../../.repo/pi-1.0.4/packages/ai/src/auth/resolve.ts) | 实现／调用链 |
| [packages/ai/src/env-api-keys.ts](../../../.repo/pi-1.0.4/packages/ai/src/env-api-keys.ts) | 实现／调用链 |
| [packages/ai/src/models.generated.ts](../../../.repo/pi-1.0.4/packages/ai/src/models.generated.ts) | 模型目录／生成数据 |
| [packages/ai/src/models.ts](../../../.repo/pi-1.0.4/packages/ai/src/models.ts) | 实现／调用链 |
| [packages/ai/src/providers/all.ts](../../../.repo/pi-1.0.4/packages/ai/src/providers/all.ts) | 实现／调用链 |
| [packages/ai/src/types.ts](../../../.repo/pi-1.0.4/packages/ai/src/types.ts) | 实现／调用链 |
| [packages/ai/src/utils/retry.ts](../../../.repo/pi-1.0.4/packages/ai/src/utils/retry.ts) | 实现／调用链 |
| [packages/ai/test/abort.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/abort.test.ts) | 测试源码／夹具 |
| [packages/ai/test/azure-openai-base-url.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/azure-openai-base-url.test.ts) | 测试源码／夹具 |
| [packages/ai/test/azure-openai-responses-reasoning-replay.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/azure-openai-responses-reasoning-replay.test.ts) | 测试源码／夹具 |
| [packages/ai/test/azure-openai-tool-choice.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/azure-openai-tool-choice.test.ts) | 测试源码／夹具 |
| [packages/ai/test/context-overflow.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/context-overflow.test.ts) | 测试源码／夹具 |
| [packages/ai/test/cross-provider-handoff.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/cross-provider-handoff.test.ts) | 测试源码／夹具 |
| [packages/ai/test/empty.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/empty.test.ts) | 测试源码／夹具 |
| [packages/ai/test/image-tool-result.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/image-tool-result.test.ts) | 测试源码／夹具 |
| [packages/ai/test/models-runtime.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/models-runtime.test.ts) | 测试源码／夹具 |
| [packages/ai/test/openai-responses-namespace.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/openai-responses-namespace.test.ts) | 测试源码／夹具 |
| [packages/ai/test/openai-responses-tool-result-images.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/openai-responses-tool-result-images.test.ts) | 测试源码／夹具 |
| [packages/ai/test/responseid.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/responseid.test.ts) | 测试源码／夹具 |
| [packages/ai/test/retry.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/retry.test.ts) | 测试源码／夹具 |
| [packages/ai/test/stream.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/stream.test.ts) | 测试源码／夹具 |
| [packages/ai/test/supports-xhigh.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/supports-xhigh.test.ts) | 测试源码／夹具 |
| [packages/ai/test/tokens.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/tokens.test.ts) | 测试源码／夹具 |
| [packages/ai/test/tool-call-without-result.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/tool-call-without-result.test.ts) | 测试源码／夹具 |
| [packages/ai/test/total-tokens.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/total-tokens.test.ts) | 测试源码／夹具 |
| [packages/ai/test/unicode-surrogate.test.ts](../../../.repo/pi-1.0.4/packages/ai/test/unicode-surrogate.test.ts) | 测试源码／夹具 |
| [packages/chord/package.json](../../../.repo/pi-1.0.4/packages/chord/package.json) | 包契约／构建配置 |
| [packages/client/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/client/CHANGELOG.md) | 文档／规划／历史 |
| [packages/client/package.json](../../../.repo/pi-1.0.4/packages/client/package.json) | 包契约／构建配置 |
| [packages/codemode/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/codemode/CHANGELOG.md) | 文档／规划／历史 |
| [packages/codemode/package.json](../../../.repo/pi-1.0.4/packages/codemode/package.json) | 包契约／构建配置 |
| [packages/codemode/src/runtime/host.ts](../../../.repo/pi-1.0.4/packages/codemode/src/runtime/host.ts) | 实现／调用链 |
| [packages/codemode/src/runtime/prelude-source.ts](../../../.repo/pi-1.0.4/packages/codemode/src/runtime/prelude-source.ts) | 实现／调用链 |
| [packages/codemode/test/sandbox.test.ts](../../../.repo/pi-1.0.4/packages/codemode/test/sandbox.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/coding-agent/CHANGELOG.md) | 文档／规划／历史 |
| [packages/coding-agent/docs/cli.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/cli.md) | 文档／规划／历史 |
| [packages/coding-agent/docs/codemode.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/codemode.md) | 文档／规划／历史 |
| [packages/coding-agent/docs/extensions.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/extensions.md) | 文档／规划／历史 |
| [packages/coding-agent/docs/keybindings.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/keybindings.md) | 文档／规划／历史 |
| [packages/coding-agent/docs/mcp.md](../../../.repo/pi-1.0.4/packages/coding-agent/docs/mcp.md) | 文档／规划／历史 |
| [packages/coding-agent/examples/extensions/custom-provider-anthropic/package-lock.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/custom-provider-anthropic/package-lock.json) | 锁文件／依赖输入 |
| [packages/coding-agent/examples/extensions/custom-provider-anthropic/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/custom-provider-anthropic/package.json) | 包契约／构建配置 |
| [packages/coding-agent/examples/extensions/custom-provider-gitlab-duo/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/custom-provider-gitlab-duo/package.json) | 包契约／构建配置 |
| [packages/coding-agent/examples/extensions/gondolin/package-lock.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/gondolin/package-lock.json) | 锁文件／依赖输入 |
| [packages/coding-agent/examples/extensions/gondolin/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/gondolin/package.json) | 包契约／构建配置 |
| [packages/coding-agent/examples/extensions/sandbox/package-lock.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/sandbox/package-lock.json) | 锁文件／依赖输入 |
| [packages/coding-agent/examples/extensions/sandbox/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/sandbox/package.json) | 包契约／构建配置 |
| [packages/coding-agent/examples/extensions/with-deps/package-lock.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/with-deps/package-lock.json) | 锁文件／依赖输入 |
| [packages/coding-agent/examples/extensions/with-deps/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/examples/extensions/with-deps/package.json) | 包契约／构建配置 |
| [packages/coding-agent/install-lock/package-lock.json](../../../.repo/pi-1.0.4/packages/coding-agent/install-lock/package-lock.json) | 锁文件／依赖输入 |
| [packages/coding-agent/install-lock/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/install-lock/package.json) | 包契约／构建配置 |
| [packages/coding-agent/package.json](../../../.repo/pi-1.0.4/packages/coding-agent/package.json) | 包契约／构建配置 |
| [packages/coding-agent/src/cli/args.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/cli/args.ts) | 实现／调用链 |
| [packages/coding-agent/src/config.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/config.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/agent-session.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/agent-session.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/bash-executor.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/bash-executor.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/extensions/types.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/extensions/types.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/mcp-servers.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/mcp-servers.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/model-resolver.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/model-resolver.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/resource-loader.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/resource-loader.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/sdk.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/sdk.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/skills.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/skills.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/system-prompt.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/system-prompt.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/tools/output-accumulator.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/output-accumulator.ts) | 实现／调用链 |
| [packages/coding-agent/src/core/tools/read.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/core/tools/read.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/codemode/execute.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/execute.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/codemode/index.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/index.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/codemode/tool.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/codemode/tool.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/mcp/resources.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/resources.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/mcp/runtime.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/runtime.ts) | 实现／调用链 |
| [packages/coding-agent/src/extensions/mcp/tools.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/extensions/mcp/tools.ts) | 实现／调用链 |
| [packages/coding-agent/src/main.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/main.ts) | 实现／调用链 |
| [packages/coding-agent/src/modes/interactive/interactive-mode.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/interactive-mode.ts) | 实现／调用链 |
| [packages/coding-agent/src/modes/interactive/theme/theme.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/modes/interactive/theme/theme.ts) | 实现／调用链 |
| [packages/coding-agent/src/utils/syntax-highlight.ts](../../../.repo/pi-1.0.4/packages/coding-agent/src/utils/syntax-highlight.ts) | 实现／调用链 |
| [packages/coding-agent/test/args.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/args.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/config.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/config.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/interactive-mode-bug-report-hint.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/interactive-mode-bug-report-hint.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/mcp-extension.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/mcp-extension.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/model-resolver.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/model-resolver.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/model-runtime-auth-options.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/model-runtime-auth-options.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/resource-loader.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/resource-loader.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/suite/agent-session-codemode.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-codemode.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/suite/agent-session-mcp-oauth.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp-oauth.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/suite/agent-session-mcp.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/agent-session-mcp.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/suite/regressions/5080-signal-shutdown-extension-cleanup.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/5080-signal-shutdown-extension-cleanup.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/suite/regressions/5109-exclude-tools.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/suite/regressions/5109-exclude-tools.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/syntax-highlight.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/syntax-highlight.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/system-prompt.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/system-prompt.test.ts) | 测试源码／夹具 |
| [packages/coding-agent/test/tools.test.ts](../../../.repo/pi-1.0.4/packages/coding-agent/test/tools.test.ts) | 测试源码／夹具 |
| [packages/durable/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/durable/CHANGELOG.md) | 文档／规划／历史 |
| [packages/durable/README.md](../../../.repo/pi-1.0.4/packages/durable/README.md) | 文档／规划／历史 |
| [packages/durable/docs/spec.md](../../../.repo/pi-1.0.4/packages/durable/docs/spec.md) | 文档／规划／历史 |
| [packages/durable/package.json](../../../.repo/pi-1.0.4/packages/durable/package.json) | 包契约／构建配置 |
| [packages/durable/src/env/index.ts](../../../.repo/pi-1.0.4/packages/durable/src/env/index.ts) | 实现／调用链 |
| [packages/durable/src/env/node.ts](../../../.repo/pi-1.0.4/packages/durable/src/env/node.ts) | 实现／调用链 |
| [packages/durable/src/harness/agent.ts](../../../.repo/pi-1.0.4/packages/durable/src/harness/agent.ts) | 实现／调用链 |
| [packages/durable/src/harness/generation.ts](../../../.repo/pi-1.0.4/packages/durable/src/harness/generation.ts) | 实现／调用链 |
| [packages/durable/src/harness/output.ts](../../../.repo/pi-1.0.4/packages/durable/src/harness/output.ts) | 实现／调用链 |
| [packages/durable/src/harness/tool.ts](../../../.repo/pi-1.0.4/packages/durable/src/harness/tool.ts) | 实现／调用链 |
| [packages/durable/src/harness/types.ts](../../../.repo/pi-1.0.4/packages/durable/src/harness/types.ts) | 实现／调用链 |
| [packages/durable/src/index.ts](../../../.repo/pi-1.0.4/packages/durable/src/index.ts) | 实现／调用链 |
| [packages/durable/src/testing/index.ts](../../../.repo/pi-1.0.4/packages/durable/src/testing/index.ts) | 实现／调用链 |
| [packages/durable/src/testing/runner.ts](../../../.repo/pi-1.0.4/packages/durable/src/testing/runner.ts) | 实现／调用链 |
| [packages/durable/src/testing/types.ts](../../../.repo/pi-1.0.4/packages/durable/src/testing/types.ts) | 实现／调用链 |
| [packages/durable/src/tools/bash.ts](../../../.repo/pi-1.0.4/packages/durable/src/tools/bash.ts) | 实现／调用链 |
| [packages/durable/src/tools/image.ts](../../../.repo/pi-1.0.4/packages/durable/src/tools/image.ts) | 实现／调用链 |
| [packages/durable/src/tools/index.ts](../../../.repo/pi-1.0.4/packages/durable/src/tools/index.ts) | 实现／调用链 |
| [packages/durable/src/tools/read.ts](../../../.repo/pi-1.0.4/packages/durable/src/tools/read.ts) | 实现／调用链 |
| [packages/durable/src/truncate.ts](../../../.repo/pi-1.0.4/packages/durable/src/truncate.ts) | 实现／调用链 |
| [packages/durable/test/env-node.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/env-node.test.ts) | 测试源码／夹具 |
| [packages/durable/test/harness-generation.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/harness-generation.test.ts) | 测试源码／夹具 |
| [packages/durable/test/harness-output.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/harness-output.test.ts) | 测试源码／夹具 |
| [packages/durable/test/harness-registry.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/harness-registry.test.ts) | 测试源码／夹具 |
| [packages/durable/test/harness-tools.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/harness-tools.test.ts) | 测试源码／夹具 |
| [packages/durable/test/tools.test.ts](../../../.repo/pi-1.0.4/packages/durable/test/tools.test.ts) | 测试源码／夹具 |
| [packages/evals/package.json](../../../.repo/pi-1.0.4/packages/evals/package.json) | 包契约／构建配置 |
| [packages/mcp/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/mcp/CHANGELOG.md) | 文档／规划／历史 |
| [packages/mcp/package.json](../../../.repo/pi-1.0.4/packages/mcp/package.json) | 包契约／构建配置 |
| [packages/mcp/src/oauth/flow.ts](../../../.repo/pi-1.0.4/packages/mcp/src/oauth/flow.ts) | 实现／调用链 |
| [packages/mcp/src/oauth/types.ts](../../../.repo/pi-1.0.4/packages/mcp/src/oauth/types.ts) | 实现／调用链 |
| [packages/mcp/test/oauth.test.ts](../../../.repo/pi-1.0.4/packages/mcp/test/oauth.test.ts) | 测试源码／夹具 |
| [packages/protocol/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/protocol/CHANGELOG.md) | 文档／规划／历史 |
| [packages/protocol/package.json](../../../.repo/pi-1.0.4/packages/protocol/package.json) | 包契约／构建配置 |
| [packages/server/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/server/CHANGELOG.md) | 文档／规划／历史 |
| [packages/server/package.json](../../../.repo/pi-1.0.4/packages/server/package.json) | 包契约／构建配置 |
| [packages/telemetry/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/telemetry/CHANGELOG.md) | 文档／规划／历史 |
| [packages/telemetry/package.json](../../../.repo/pi-1.0.4/packages/telemetry/package.json) | 包契约／构建配置 |
| [packages/tui/CHANGELOG.md](../../../.repo/pi-1.0.4/packages/tui/CHANGELOG.md) | 文档／规划／历史 |
| [packages/tui/package.json](../../../.repo/pi-1.0.4/packages/tui/package.json) | 包契约／构建配置 |
| [packages/tui/src/keybindings.ts](../../../.repo/pi-1.0.4/packages/tui/src/keybindings.ts) | 实现／调用链 |
| [packages/tui/test/keybindings.test.ts](../../../.repo/pi-1.0.4/packages/tui/test/keybindings.test.ts) | 测试源码／夹具 |
| [packages/tui/test/tui-alt-screen.test.ts](../../../.repo/pi-1.0.4/packages/tui/test/tui-alt-screen.test.ts) | 测试源码／夹具 |
| [scripts/build-coding-agent-bundle.mjs](../../../.repo/pi-1.0.4/scripts/build-coding-agent-bundle.mjs) | 构建／发布／工作区输入 |
| [scripts/local-release.mjs](../../../.repo/pi-1.0.4/scripts/local-release.mjs) | 构建／发布／工作区输入 |
| [tsconfig.json](../../../.repo/pi-1.0.4/tsconfig.json) | 构建／发布／工作区输入 |

### 6.3 删除（3）

| 路径 | 复核类别 |
|---|---|
| `packages/ai/src/providers/azure-openai-responses.models.ts` | 模型目录／生成数据 |
| `packages/ai/src/providers/azure-openai-responses.ts` | 实现／调用链 |
| `tui-plan.md` | 文档／规划／历史 |

## 7 交付与限制

新版是完整正文：整体状态图、每个包的API/流程/失败/测试/规划/阅读顺序均承接当前实现；不是在1.0.2正文后仅追加差异。本轮未改Pi源码、未运行build/test/daemon/SSH/provider，也未确认上游tag或发布包字节。

旧报告作为历史对象保全精确恢复位置；新版继承依据位于自身evidence目录，并包含更早1.0.0依赖的必要证据。新导航在覆盖和验证完成后切换，历史比较研究的基线和运行日志不重写。最终机械与人工检查、旧目录退役、源树一致性、Git状态及未执行事项见 [verification.md](verification.md)。
