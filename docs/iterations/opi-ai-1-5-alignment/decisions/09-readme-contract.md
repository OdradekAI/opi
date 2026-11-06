# 确定面向第三方开发者的 README 信息结构
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](01-upstream-contracts.md), [核实 opi-ai 工程、测试与独立接入的现状](02-current-structure-tests.md), [确定独立接入与行为对齐的公共契约](04-public-contract.md), [确定 opi-ai 的模块结构与公开入口](06-module-structure.md), [确定测试归属、差分证据与去重规则](08-test-boundaries.md)

## Question

如何借鉴 pi-ai README 的用户任务顺序，设计 opi-ai 的安装、最小完整示例、模型/provider、流、工具、auth、扩展、features 和生命周期文档？哪些细节进入 rustdoc/专题，如何以外部消费工程和可执行示例证明文档足够接入，并诚实标注现有能力？

## Comments

2026-11-06 候选信息结构，尚未改写产品 README。

建议复用 pi-ai 的开发者任务顺序：定位/支持范围 → 安装与 features → 最小 complete 示例 → streaming 与最终结果 → Models/providers → auth/store → tools → 图像/分类 → reasoning/原生选项 → 错误/取消/清理 → 自定义 provider/API → Faux 测试 → telemetry → 跨模型历史/序列化 → 迁移与 API 参考。

README 承担“能接入、知道支持条件和继续去哪里找”的任务；具体类型和方法进入 rustdoc，复杂 OAuth/代理/WS/生命周期进入专题，私有维护与历史验收留在开发文档。AIR/RDF/task 编号不进入普通接入流程。中英镜像一起维护，提取仍有效的内容，避免直接丢弃行为合同。

当前 Quick Start 暴露了较多初始化、Request 字段与生命周期细节，说明需要同时审查公开 API 的接入成本。若只能用现有 API，则诚实给出完整例子，不能编造尚未实现的 builder 或便利方法；若要简化 API，单独决策和实施后再更新示例。

接入验收：空目录外部 consumer 使用打包产物，仅按 README 增加声明依赖；无仓库路径、私有模块、开发依赖泄漏；离线 Faux 可执行；真实 provider 示例至少可编译，联网 smoke 显式启用。取消、工具循环、错误处理各有独立可执行示例。

### 继续讨论：以开发者任务为主线的使用手册

2026-11-06，测试政策已确认。本节提交 README 重构的信息结构与验证方案，尚未修改产品 README，也没有冻结未来 Rust API 的逐符号签名。

#### 从 Pi 借鉴什么

本地 Pi README 的主线是“安装与调用 → Providers/Models → Auth → Tools → 图像/分类/推理 → 错误与遥测 → 扩展 → 测试与上下文 → 平台与迁移”。多数主题同时提供用途说明和调用例子，读者能按自己的任务进入，而不是先理解仓库工程史。

opi-ai 的混乱主要来自同一份文件混合了接入教程、公共行为合同、私有实现说明和历史验收记录。当前英文 README 的 Quick Start 已有约百行代码；后面不断插入 AIR/RDF/task 标题，License 后仍追加 AWS/ADC/OAuth 大段内容。重构应重新组织整篇，而非在旧目录顶部再增加一段简介。

建议保留 Pi 风格的完整能力导航和常用示例，不设 README 行数指标。正文足以完成常见接入，专题承担长流程与协议差异，rustdoc 承担精确 API。关键行为必须在使用位置出现；维护记录移出时，不连同用户需要的约束一起删除。

Pi 的 JS imports、Browser Usage 和 Bundling/Tree Shaking，分别换成 Rust 的依赖声明、已验证平台/运行时、Cargo features 与最小依赖配置。不能据此承诺 WASM、浏览器、多运行时或 no_std；也不能照抄 npm 子路径作为 Rust 公共目录。Pi README 的“所有内置 provider”第一例可调整为单 provider 的最小真实调用，全部注册放在 provider 章节。

#### 推荐目录与每节交付

以下是新 README 的内容顺序，英文/中文保持语义与示例同步；实施时可合并相邻短节，但不丢掉能力入口。

| 顺序/章节 | 开发者要完成的任务 | 正文必须包含 |
| --- | --- | --- |
| 定位、支持范围与目录 | 判断是否适合自己的项目 | 独立 Rust 模型调用库的职责；聊天、图像、分类等实际支持范围；版本对应的兼容状态与限制链接；API 文档和中文/英文切换 |
| Installation | 在自己的 Cargo 项目中安装 | 已核实的依赖版本、MSRV/运行时要求、推荐功能配置、示例额外依赖；完整可复制 manifest 片段；最小配置入口 |
| Quick Start | 完成一次真实调用 | 单 provider、明确凭据来源、模型选择、消息、complete、结果/错误；所需 import、运行入口与命令；紧邻提供无需凭据的 Faux 运行路径 |
| Streaming and Results | 显示增量内容并获取最终结果 | 可用流式示例、常用事件、终态与 result；启动时点、停止消费、显式取消的区别，并链接完整事件表 |
| Providers and Models | 注册、选择与查询模型 | 单 provider/全部内置注册、静态目录/动态刷新、自定义模型；支持表区分 provider、协议、操作能力、auth 与必要配置 |
| Authentication | 将自己的凭据接入 SDK | 显式/环境/Store 等实际优先序的清楚说明、API key 示例、OAuth 入口；内存 Store 与持久化职责；云认证和宿主交互指向专题 |
| Tools | 完成工具调用续轮 | schema、最终参数校验、执行工具、tool result 回填与下一次调用；partial JSON 仅用于预览；受约束输出入口 |
| Image Input / Image Generation / Classification | 使用非普通文本能力 | 三种独立能力分别给最小示例，注明模型能力检查与返回类型；不能用聊天图片输入示例代替图像生成 |
| Reasoning and Native Options | 配置通用参数或协议特性 | 通用/原生选项的选择方式、支持条件、应用/忽略/拒绝行为的查找入口；协议细表放专题 |
| Errors, Cancellation and Resources | 处理失败并结束工作 | 返回前失败/流内失败/终态的实际区分、partial/usage 的保留、取消与 Drop、清理方式；重试责任及工具副作用说明 |
| Usage, Cost and Telemetry | 读取用量、费用与观测请求 | usage 与估算/供应商费用的区别；无观测配置可正常调用；显式 context/可选适配的简例；宿主 exporter/init/shutdown 职责 |
| Context, Handoffs and Serialization | 保存与恢复上下文、更换模型 | system/tool 时间顺序、跨 provider 重放、序列化示例与边界；上下文不等同于持久化完整运行中请求 |
| Custom Providers and APIs | 接自己的服务或新协议 | 已有协议+自定义服务的短例；新协议扩展入口；直接调用 API、Provider 与 Models 的适用场景；高级执行/Store 接口导航 |
| Faux and Deferred Operations | 离线测试或处理延迟结果 | Faux 的正式公共用法；deferred 的能力检测、fetch/wait/cancel 入口及明确支持范围；本地取消与远端任务取消的区别 |
| Migration / Further Reading / Development / License | 升级并找到更多信息 | 0.x 路径迁移索引、Pi 能力对应与已知差异链接、贡献/测试入口、许可证及必要第三方声明 |

支持表不以“有一个 provider 工厂”推断聊天、图像、分类、OAuth 或 deferred 全部可用。主表写开发者选型所需的能力与条件；详细验证矩阵放兼容性专题。源码存在、离线协议通过和某日期真实端点可用不是同一种证据，不能统一写成“已完全支持”。当前快照缺目录数据或有未决行为差异时，不以 README 文案补齐结论。

#### 第一段接入体验

推荐主路径是“单 provider + API key + 一次 complete”，延续现有 Anthropic 示例及其受控服务验证基础；具体 provider/model 名称、版本和所需 feature 以实施时选定且验证的版本为准，不在本票假定线上模型可用。

第一例只需要读者做这几件事：创建普通 Cargo 项目 → 复制依赖 → 设置该例明确使用的凭据 → 复制完整 main → cargo run。命令面向用户自己的项目，不要求执行仓库中的 cargo run -p opi-ai --example，也不要求先安装 opi CLI。提供 PowerShell 与 POSIX 的环境变量设置方式，命令不内嵌真实密钥。

完整第一例应展示依赖、import、运行时入口、构建调用与读取结果；避免为测试便利将 setup/start/finish 三套辅助函数完整暴露给初学者。确实必要的配置必须显示，不能用省略号、未定义变量、隐藏的全局初始化或虚构 builder 把未解决的 API 成本藏掉。若当前接口仍需要长 Request 字面量，应在已批准的公共接口迁移范围内先完成真实简化和调用者验证，再生成新例子；本票不凭文案批准任何具体便利 API。

Faux 紧邻 Quick Start 提供无凭据、确定输出的第二路径，用来验证安装和运行时配置。它的最小依赖闭包也需要外部包验证；不能通过全部 features 编译后宣称最小接入已证明。Faux 通过不代表真实 provider 的鉴权和协议通过。

第二个主要例子讲 streaming/result，第三个讲工具续轮。错误、取消、图像、分类、自定义 provider/API、认证与 telemetry 各有可发现的用法；长场景放专题和完整 examples。第一屏不把所有事件分支、工具循环、全部 provider 和遥测配置堆进同一个程序。

#### 内容的唯一归属

| 内容 | 拥有者 | README 中保留什么 |
| --- | --- | --- |
| 常见开发任务与能力导航 | README.md / README.zh.md | 可直接使用的核心例子、关键约束与专题链接 |
| 类型、字段、方法、错误、取消/并发合同 | 对应公共项 rustdoc | 必要概念和关键规则摘要，链接精确接口；不手写重复的全量方法列表 |
| 多步骤/平台/协议差异 | crate 内 docs 专题及对应 examples | 入口、最小用法与影响选择的限制 |
| 私有 codec/crypto、测试执行、架构选择、历史验收 | 现有工程/迭代/ADR 或测试说明 | 简短贡献入口；用户确实需要知道的传输能力仍进产品专题 |

拟新增产品专题按实际内容建立，不先铺大量空文档。可从 auth、providers/transport、streaming/lifecycle、tools/schema、extension、compatibility/migration 几组起步；它们是内容分组，不要求六个固定文件。既有 topic 内容先核实再迁移，每段都有新归属或失效理由。现有中英文产品文档继续同步，新增产品专题也提供对应语言入口；代码来源共享，必要时翻译旁边的说明，避免两套语义漂移。

需要从 README 移出的典型内容：task/AIR/RDF 编号、私有 PMD/crypto 实现细节、测试关闭记录、历史 proof、仓库 cache 与内部门禁命令。需要保留并改写为用户语言的内容：代理/TLS 的实际配置、重试可能影响重复请求、谁持有凭据持久化、哪些 native options 有效、会话资源如何清理、哪些能力尚未支持。许可证和第三方 NOTICE 保留，不能作为“工程噪音”删除。

#### 不能在整理中抹掉的行为

- 请求何时开始、消费句柄 Drop 做什么、如何明确取消、何时可获取结果与完成清理。README 现有 first-poll lazy 行为与[已批准的目标生命周期](11-telemetry-lifecycle.md)须随实现迁移更新；不能只换标题后保留全部旧预期，也不能提前宣布目标已落地。
- 事件中的 partial 是事件时点快照还是会继续变化的对象，保留后再次读取会看到什么。Pi 的真实 Adapter 与 Faux 也存在细节差异，不能直接复制 README 的 shared live partial 描述，也不能把 Rust 当前快照语义当作已批准的等价映射；未决差异由验收契约逐入口处理。
- 工具参数预览与最终可执行参数的区别；失败 tool result、ID 配对和历史追加责任。opi-ai 提供模型/工具调用合同，宿主负责工具执行和完整 agent 策略。
- 认证的实际优先序、刷新及持久化，取消一个等待者不等于撤销已经开始的凭据轮换；内存 Store 不承诺持久保存，宿主 keychain/交互方式不应成为基本接入必需品。
- transport 重试、显式 whole-call helper 与宿主 agent 重试的不同责任；用户选择影响重复发送与副作用时，正文必须给出简明说明，精确判定留在对应接口文档。
- transcript/序列化/跨模型 handoff 的可保留内容与限制；不能暗示 serialize context 就能恢复连接、认证上下文或进行中的远端作业。
- telemetry 的中立 context、AI instrumentation 与宿主采集策略边界；默认无观测后端仍可工作。现有 frozen tracing carrier/noop 实现不能未经迁移就作为未来 opi-telemetry 的已发布合同。
- 图像、分类和 deferred 各自的能力限制；尤其 scripted/custom deferred 接口不能写成所有内置远端 provider 都支持异步任务。

目录章节还要明确静态目录、Models 注册集合、鉴权可用性与显式刷新之间的区别。代码存在离线目录，不等于所有操作都无需联网；查询能返回模型，也不等于已配置调用凭据。

这些是文档需要覆盖的行为主题；每项对目标快照的精确判定依照契约索引，README 不重新定义规范。

本轮有界核验的源码锚点包括：src/models.rs:632 的默认凭据 Store、:634 的目录来源配置；src/credential.rs:303、:334 的 mutation 生命周期；src/retry.rs:249、:537 的显式重试与交接；src/message.rs:290、:315、:426 的工具参数 final/preview；src/models.rs:4918 的 partial 描述；src/openai_chat.rs:3168 的 receiver-closed 行为；src/telemetry.rs:34、:73、:113 的当前 carrier。它们均相对 D:/Luiz/Odradek/opi/crates/opi-ai，绑定地图 HEAD，仅证明当前实现，不替代未来验收。

#### 示例与发布包的验证方式

复用已存在的 scripts/pi-sdk-conformance/run_readme_examples.py。源码 :55–70 检查打包 examples 与两份 README 嵌入一致；:645–669 将打包示例放进外部 consumer，编译 standalone bins，并通过受控服务执行它们共享的 setup/start/finish。它已包含外部执行路径，不应误写为“仅有仓库内编译”；本轮只读了脚本，没有重新执行，也不把已有报告当作未来重构后的通过结果。

现有校验要求整份示例逐字嵌入两份 README，会把测试辅助结构也带进接入文档。建议保留“展示代码就是受测代码”的保证，允许从一个完整 examples 源提取具名片段；Quick Start 仍给完整可复制程序，其余片段明确上下文并链接完整源码。检查提取片段、打包源码、英文/中文展示一致，不另维护三套手写示例。

后续文档验收包括：

1. 新建工作区外 consumer，消费实际发布包，仅增加 README 明示的普通依赖和 features；不继承仓库私有路径、补丁、测试探针或 dev-dependency 闭包。打包测试中指向已提取 .crate 的受控路径是验证手段，不写进用户安装步骤。
2. Quick Start 的完整程序可编译；Faux 路径实际执行。真实 provider 通过受控服务执行请求、流/结果、错误和工具续轮；需要控制端点时使用正式配置接口。另核对完整 main 和运行命令，不能只运行测试辅助函数便覆盖所有接入步骤。
3. 每个标成可运行的主要例子有对应外部消费证据；片段注明所属示例，真实服务调用只在显式 live 入口运行。不能因未提供 key 就把网络例子标通过。
4. rustdoc 中 no_run 仅证明编译，ignore 不提供编译保证；网络例子可 no_run，但必须保留受控协议验证。README 普通代码围栏不会因 Cargo 指定 readme 字段自动成为 doctest，应由现有提取/消费 harness 或明确纳入 rustdoc 的方式检查。[rustdoc 示例规则](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)、[Cargo readme 字段](https://doc.rust-lang.org/cargo/reference/manifest.html#the-readme-field)。
5. 对应版本的 README、docs、examples、rustdoc 与 crate 元数据一致；文档链接在仓库、实际包及发布文档入口可达。面向已发布版本使用匹配版本的 API/专题链接，不把私有本地路径或持续变化的主分支当作唯一说明。
6. 迁移文档包括旧入口→新入口、依赖/feature 调整和经批准的行为变化；已有外部兼容要求按已确认的迁移政策处理。版本号、default features、平台/MSRV 与完整发布门禁由验收决策冻结。

当前 Cargo.toml 声明 readme=README.md、未有 features 表；根 workspace 的版本/MSRV 只是当前源码事实。未来安装片段必须基于届时实际可获取且验证的产物，不从徽章、拟定目录或包名推定发布可用性。本轮不改 Cargo，也不发布 crate。

#### 推荐迁移顺序

清点现有章节并标记新归属 → 确认公开入口和示例的真实实现 → 写通安装、complete、stream/result 与工具续轮 → 按能力补正文短例与专题 → 迁移有效合同并清理历史残留 → 同步中英文、rustdoc、包内资源和迁移说明 → 外部消费/链接检查。发生接口变更时，代码、示例和 owning docs 同步更新；不让一份正式 README 同时混合现状与未实现目标。

#### 本轮征询（已确认，见 Answer）

是否采用上述 Pi 风格的完整开发者手册结构：README 正文保留常用能力的可用示例与关键行为，长流程/协议细节进入专题，精确 API 进入 rustdoc；快速开始以单 provider 的真实最小调用为主，紧邻提供离线 Faux 路径，再依次展开流与工具？

推荐采用。这保留能力全貌，也让首次接入只处理当前需要的概念；具体 Rust 签名和 feature 名称仍以实施后的真实 API 为准。确认后推进最后的验收标准与发布判定，不直接改写产品 README。

## Answer

2026-11-07，用户以“同意，继续推进”确认本票 README 方案。

- 采用 Pi 风格、按开发任务组织的完整开发者手册。README 保留常用能力的可用示例、关键行为与导航；复杂流程/协议差异放产品专题，精确公共合同放 rustdoc，私有实现与历史验收放工程/迭代资料。
- 快速开始以单 provider、API key 和一次完整真实调用为主，紧邻提供无凭据的 Faux 路径，再分别展开 streaming/result 与工具续轮。安装与命令面向普通外部 Cargo 项目，完整程序不依赖未定义辅助函数、私有路径或隐式全局初始化。
- 保留聊天、图像输入、图像生成、分类、认证、目录、原生选项、错误/取消/资源、用量/遥测、上下文、扩展、Faux/deferred 和迁移的能力入口。删去工程历史时，保留会影响调用结果的约束；目标行为随实现迁移，不提前写成已交付。
- 中英文同步，示例共用真实源码并与打包产物绑定。复用现有外部消费 harness，验证完整程序、受控协议执行与文档链接；精简展示不削弱“文档代码就是受测代码”的保证。
- 本票目录表达信息结构，不冻结逐符号 API、feature 名称、发布版本或线上模型可用性；具体配置与发布验收由验收决策确定。

本轮仅完成文档规划，未改写产品 README、代码或测试，没有执行新的外部消费或联网验证。
