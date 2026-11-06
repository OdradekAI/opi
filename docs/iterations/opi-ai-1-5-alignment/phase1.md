# 第一阶段：固定基准与建立映射

日期：2026-11-07。实施状态由[第一阶段实施票](issues/01-baseline-and-mapping.md)维护。本文件解释已建立的证据和后续使用方式；[契约索引](contract-index.json)是能力、消费者、测试与旧标准对照的唯一数据入口。

## 已建立的基准

[基准记录](evidence/phase1-baseline.json)固定了目标 Pi 快照的 2949 个文件、相关 opi 源码/资产/脚本/配置的 943 个文件、两个 lockfile、直接依赖的声明及锁记录、工具链和执行入口。逐文件原始字节摘要均保留，可以检测修改、删除和范围内新增文件。

- Pi 内容清单摘要仍为 `959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072`；该快照没有自身 Git 身份，不能将旧验收文档的 commit 写成它的 commit。
- opi HEAD 仍为 `90f953d5ee288c165dc295bfdfb7cb7a05d7cecd`；当前包版本 0.8.3，声明 MSRV 1.97，没有本次构建产物。
- 本机 Node 24.13.0、Rust/Cargo 1.99.1，Windows x86_64。Node 满足目标声明的最低版本；本机 Rust 运行环境不构成 MSRV 测试。
- npm SDK 版本记录来自目标 package/lock；目标 `node_modules` 和 `dist` 均缺失，不能称为已安装或已打包验证。
- 另有隔离的 Python 语法解析工具依赖，版本固定在 [requirements](tools/requirements.txt)。它们安装在工作目录，不加入产品 Cargo/npm 依赖。

快照保持原样。后续需要补依赖、恢复目录或构建时，在可追溯的工作副本中操作，记录新增材料；不能静默重写本阶段基准来消除漂移。

## 单一索引的范围与读法

| 索引字段 | 内容 | 它证明什么 |
| --- | --- | --- |
| `contracts` | 50 个能力族，含原入口、当前 Rust 候选、观察点、目标责任及风险 | 能力的实施归属；不等于已实现或等价 |
| `source_files` | 205 个 TS 源文件和 139 个 Rust 源文件 | 每个源文件均有能力族归属，包括内部机制及 crate-local 测试文件 |
| `entrypoints` / `upstream_symbols` | 172 个源码投影子路径、1188 个导出绑定 | 递归 re-export、别名、type-only、通配嵌套入口已清点；同一符号在不同入口分别记录 |
| `models_methods` / `provider_factories` | Models 24 个方法、MutableModels 3 个方法；43 个工厂 | getAuth 的重载保留在方法记录中；42 个内置工厂加 Faux |
| `rust_declarations` | 3469 个语法级声明候选 | 便于查现有类型、方法、字段、重导出；不是编译器确认的公开 API 分母 |
| `consumers` / `critical_consumers` | 161 个含直接引用的 Rust 文件；关键生产消费者与包消费路径单列 | 迁移影响的起点；文本引用可能位于 rustdoc/inline tests，不能全部计为生产调用 |
| `test_files` / `test_declarations` | 183 个 TS 测试文件、8 个 TS 辅助/手工探针文件、83 个 Rust tests 文件；4869 个测试声明 | 保留输入来源和案例定位；参数化、循环及 helper 生成的运行实例没有展开 |
| `reviewed_cases` | 38 个读取了具体断言的重点案例 | 已核查对应观察面，仍未运行原产品测试 |
| `acceptance_rebase_and_observation_design` | 172 条旧编号条款的逐条处置、8 组观察设计、目录恢复依据 | 原条文、行号、摘要、适用能力、变更理由和受阻范围可追溯 |

测试声明分为 TS 1960 个、Rust 2909 个。Rust `tests/` 中有 73 个根测试 target，不能把 83 个文件都当作独立 target。**以上数字不是通过数或覆盖率。** 全部现有产品案例保留；文件级候选族用于派发审查，不能据此声称每个案例都覆盖文件的全部能力，更不能直接删除测试。

导出清单来自 package exports 与源码，实际 `dist` 尚未生成。TS 成员记录包含声明自身成员与继承 header，继承成员仍在其声明类型下；没有运行 TypeScript 类型检查器。Rust 记录不展开 cfg、宏和完整重导出可达图。语法解析器的两类兼容处理仅作用于等长内存缓冲区，原始签名、位置和摘要仍取源字节；具体文件记录在 `extraction.parser_adjustments`。

后续迁移一个能力时，从 `contracts` 找责任和原入口，沿 `source_files`、`consumers` 找当前影响，再检查具体测试声明与 `reviewed_cases`。在实施中补新 Rust 精确入口、输入/时序/断言和执行记录。当前“同族候选”不构成逐符号替代证明，未确认的实际能力明确留作缺口。

## 会影响后续重构的发现

| 观察面 | 现状证据 | 后续处理 |
| --- | --- | --- |
| 中立 no-op | `telemetry_carrier.rs:251/276` 断言屏蔽宿主 tracing | 拆分中立层与 tracing 适配时改为精确责任验证，保留旧轨迹；不能把屏蔽宿主当作新合同 |
| 启动时点 | `telemetry_operations.rs:793` 断言未 poll 时无 adapter 构造；原版 lazyStream 会同步调用 setup | 分别记录创建、首次 poll、首次 I/O；Models/真实 provider 的启动映射仍需独立实验 |
| 停止消费 | `provider_lifecycle.rs:1084/1191/1198` 经共享 helper 验证 Drop 关闭 HTTP；原版 lazyStream 外层 return 后仍执行 | 执行所有者与消费者重组时分别验证停止观察、显式取消和资源终结 |
| partial 别名 | `stream_events.rs:745` 验证事件时不可变快照 | Faux 和真实 adapter 分别建立共享关系实验；不能用最终文本或 generic 容器实验替代 |
| Schema/旧 backend | 存在 Pi 1.3 oracle、旧 profile 的两项 URL 差异及旧说明 | 保留来源，按目标重录独立证据；旧例外不自动继承为 Pi 1.5 的已批准差异 |
| 高级消费者 | agent 的 PreparedRoute/attempt admission、coding-agent 的 Mutation*、HTTP/云配置等已有使用 | 基础边界阶段保留能力，接口变动与真实消费者同步迁移 |
| 能力候选缺口 | 正式 Faux、generic EventStream 异常、AI metrics/schema、运行时模型 schema、compat/包级工具等尚无逐项证明 | 逐族确认具体 Rust 对应，不以同名模块或 MockProvider 自动填为完成 |

上表中的短路径与完整行号关系见索引的 `migration_risks`、`reviewed_cases` 和 `critical_consumers`。仓库外消费者仍未知；`opi-agent` 的 ToolDef 重导出等间接使用不能被字面扫描排除。

旧标准的 172 条逐条对照保留在索引，原输入另作[历史归档](evidence/acceptance-original.md)。当前规则以已确认决策及各条 `target_rule` 为准：移除将旧 Anthropic continuation 当作目标能力的推导，重新绑定 max-token 4096 余量、Copilot 过滤及当前测试分母；保留有条件的宿主/工具映射。数值预算继续待校准和确认，没有在本阶段冻结为每次发布的硬门槛。

## 已运行的原版观察

[原始记录](evidence/phase1-observations.json)由[独立子进程探针](tools/observe_upstream.mjs)直接导入冻结的 TS 文件生成，没有复制其语义实现。9 个具名案例的源断言通过：

- extractResult 抛错时，两个既有 waiter 和 result 在具名检查点仍 pending；新 iterator 可结束；随后 end() 与 end(result) 有不同效果。
- 只 push 正常终态时先满足一个 waiter；显式 end 才释放另一 waiter。
- generic EventStream 保留传入对象身份，延后读取能看到原对象修改。此项只证明容器，不证明 Faux 或真实 adapter 的 partial 图。
- lazyStream 调用时同步启动 setup；完全不消费、读取一项后 return 两种情况下，放行源屏障后仍继续执行并产生 result。
- 终态 onEvent 抛错、onResult 抛错、重复 onEvent 抛错分别记录；业务 result、observer 结果和 unhandledRejection 可能不同。
- setup 同步抛错与返回拒绝 Promise 的错误通道不同。

每个案例有独立进程、原始步骤、源文件摘要、driver 摘要和命令。pending 在明确事件循环检查点观察，进程 watchdog 只用于失败退出，不作为 pending 证据。子进程采用显式环境，不读取真实 provider 凭据。

这些是**有限原版源码观察**，没有 Rust 比较结果，也没有执行完整原包、Models、Faux、真实 adapter 或 AI telemetry。索引中的其余观察设计列出 fixture、屏障调度、采集项、比较规则、当前候选入口和阻断范围，供后续直接补驱动。

## 已定位但未补齐的材料

目标 `nix/model-catalog.json` 固定了 revision `sha256-f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`，目标 Nix 文件给出精确版本 URL 和原始响应摘要规则。本地搜索没有找到相同归档；现有 Rust 目录来自 Pi 1.3.0，另一 Pi 检出的 pin 也不同，均不能替代。

已尝试该固定 URL，连接被远端重置（WinError 10054），[尝试记录](evidence/phase1-model-data-attempt.json)未获得可校验产物。恢复顺序为：在快照外下载固定响应 → 校验原始 SHA256 → 在工作副本执行目标 hydrate 脚本 → 执行目标 check-model-data → 记录生成数据与已安装依赖身份。

因此目录/价格精确比较、依赖该目录的原版整包结论仍受阻；SDK 依赖缺失也阻断完整原版运行。边界提取、已有消费者迁移与不依赖这些材料的受控实验可继续进行。实际包入口解析、Rust 对照、MSRV/多平台/features、资源/性能预算和发布验收尚未执行。

## 复核与维护

从仓库根目录运行基准与索引检查（仅 Python 标准库）：

```powershell
python docs/iterations/opi-ai-1-5-alignment/tools/verify_phase1.py --repo .
```

重新提取语法清单时，将 requirements 装入独立工作目录，并将结果写到工作目录，再给 verifier 传入 `--inventory`。提取器不会改写人工维护的契约索引。测试脚本需要 `--repo`、`--parser-path` 和 `--scratch`；具体已执行命令、退出码及输出见[检查记录](evidence/phase1-checks.json)。

工具验证覆盖别名/重导出、循环拒绝、祖先条件 suite、todo、Rust 属性、原始字节保留、文件增删改、基准防覆盖、删减声明及导航清单。CLI 负例确保损坏基准和缺失记录会失败；原版探针的未知案例也必须非零退出。

本阶段测试影响：产品测试 retain；新增的是调查/验证工具及其专项测试。产品源码、Cargo、产品 README 和既有测试没有重构。后续基础边界阶段出现预期源码变化时，本基准检查会报告漂移；应保留阶段一原证据并建立候选身份，不能覆盖基准或把旧轨迹当成新实现的通过记录。
