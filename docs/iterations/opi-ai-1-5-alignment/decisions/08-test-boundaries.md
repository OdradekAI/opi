# 确定测试归属、差分证据与去重规则
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](01-upstream-contracts.md), [核实 opi-ai 工程、测试与独立接入的现状](02-current-structure-tests.md), [核实 Rust 依赖复用与成熟项目的测试策略](03-rust-ecosystem.md), [确定独立接入与行为对齐的公共契约](04-public-contract.md), [确定 opi-ai 的模块结构与公开入口](06-module-structure.md), [确定第三方能力的复用与自研边界](07-dependency-boundary.md), [确定遥测生命周期与失败语义](11-telemetry-lifecycle.md)

## Question

哪些不变量由子模块测试独占，哪些由公共接口集成/协议契约/TS 差分测试拥有？怎样证明删除重复测试不丢失失败、竞态、清理和切片边界覆盖；如何分离 fixtures/support、离线主门禁、属性/fuzz、资源测试与显式 live 检查？

## Comments

2026-11-06 首轮建议，尚未授权删除测试。

现有 tests/README 已按私有机制、私有组合、公开契约、SDK 消费者区分，方向可保留。组织边界首先取决于是否跨公开 API，而不是调用了多少模块；多个私有模块的组合仍可在 crate 内测试，一个公开函数也可有集成契约测试。

每项契约设一个主要证明位置：纯语义/私有状态在 src；公开职责和协议在 tests；跨模块选择代表性贯通路径；TS 差分共享原始输入并独立驱动两端；实际发布包消费、fuzz、性能、live 各有明确入口。减少根 tests/*.rs target 数量与删除断言是两件独立的事，可先把相关案例收进少量职责级 target 的子模块。

示例：WS 私有层穷举分帧/压缩/取消；transport 契约验证握手/TLS/代理；Models→Codex 只证明生产链路确实调用它及 session/关闭正确，无需在整条链重复全部压缩语料。反例：遥测冻结 Dispatch 的 poll/drop 测试、业务操作计量测试、spawned producer 传播测试，证明不同风险，不因都叫 telemetry 就去重。

删除必须记录“旧案例 → 契约/风险 → 替代断言 → 失败可检测性”，比较输入类、时序、错误阶段、副作用、清理条件。只覆盖相同代码行或最终文本不构成替代；关键阈值/竞态可用故意变异确认替代测试仍会失败。真正过时的预期绑定旧基准后重判，不能直接删掉不合新实现的测试。

注意给定验收 G 系列包含公开小 helper 的直接行为承诺：被联合测试顺带调用不代表已覆盖。小不是删除理由，重复证明且无独立价值才是。此轮仅静态抽样，不给出“全部旧测已审完”或通过结论。

### 继续讨论：按契约组织测试，按反例判断重复

2026-11-06，模块结构与公共入口迁移政策已确认。本节为待用户确认的测试方案。调查仍是静态阅读和有限抽样，没有执行 Rust/Pi 测试；不构成具体删除清单。

#### 两种位置、四类证明

“子模块测试”与“联合测试”容易混用：Rust 的放置边界是能否访问私有实现；测试覆盖一个还是多个模块，是另一维度。建议直接称为 crate 内测试、公共契约测试、协议测试和贯通场景，避免把所有跨模块组合都搬到 tests。

| 要证明什么 | 主要位置 | 保留什么，避免什么 |
| --- | --- | --- |
| 私有机制的规则与状态转移正确 | src 中所属模块的 cfg(test) tests；需要多个私有模块的组合也放在有合理访问权限的共同所有者处 | 增量解析、提交屏障、资源所有权、边界和竞态。无需为每个私有函数配测试；简单委派若已被上层精确断言覆盖，可不单测 |
| 开发者可依赖的公开行为正确 | tests 的公共契约主题 | 公开入口的输入/输出、错误阶段、副作用和类型约束。一个公开小函数也可有直接契约测试，不为接触私有字段扩大 API |
| 真实 Adapter 与传输接线正确 | tests 的协议主题；本地可控 HTTP/WS/认证服务 | 实际请求、headers、wire 事件、回调顺序、失败/取消/清理；仅 mock SDK 返回事件不能证明 SDK 配置与真实 wire 正确 |
| 多职责组成用户场景后仍正确 | tests 的贯通主题；独立包消费者另用现有 harness | Models→auth→真实 Adapter→stream/result→telemetry 的代表性链路及已知高风险交互。Faux 可验证编排，不能代替真实协议路径 |

差分、属性测试、fuzz、mutation 是用于上述职责的验证方法，不再各自复制一整套业务用例。文档/发布包消费和资源预算另有执行入口。参考验收文档的 L0–L8 是证据分类，不必照抄成九层源码目录。

一个契约选择一个主要验证位置，负责其完整输入类别、边界与状态表；其他位置只补能发现不同缺陷的证据。这不等于每项行为只能断言一次，也不允许用一个“最终成功”测试替代各公开入口的行为承诺。direct API、Provider、Models、compat 之间存在不同鉴权/错误/默认值行为时，分别保留适用行。

#### 建议布局与 target 整理

示意布局如下；目录、target 个数和分组不是发布承诺，实施时依据 feature、进程隔离和编译/运行成本确定：

```text
crates/opi-ai/
  src/<owner>/tests.rs       # 或 tests/，靠近私有职责
  tests/
    contracts.rs            # 公共行为主题索引
    contracts/              # usage、schema、auth、catalog、stream、Faux 等
    protocols.rs            # 协议主题索引；必要时按 feature/成本拆多个 target
    protocols/              # 按 wire 协议，provider 特例放对应协议之下
    flows.rs                # 跨职责交接、真实高级消费者场景
    flows/
    support/                # 可控 peer、时钟/屏障、记录器、测试数据构造
    data/                   # 已有数据目录；固定输入、来源与预期分开标记
  examples/                 # 第三方使用方式
  benches/                  # 适用的性能/分配预算
scripts/pi-sdk-conformance/  # 扩展已有发布包消费与 README 验证
```

当前有 73 个根 integration target，可先按职责分组、复用 setup 和数据构造，再评估删断言。不要直接指定必须缩到三个可执行文件；大型协议、不同 feature 或依赖进程隔离的测试可以单列。合并 target 会改变测试共享进程和并行关系，必须检查环境变量、全局 subscriber、静态缓存、固定端口和一次性初始化；优先注入依赖或用隔离子进程，不能靠将全部测试串行掩盖污染。

共享 helper 放子目录，不新增零用例的根 target；不复制生产源码到外部测试，不通过 doc(hidden) public 探针访问私有状态。正式 Faux 属产品能力，仍保留公共 API 和自身契约测试。

Rust/Cargo 的上述访问和 target 规则见 [Cargo tests](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#tests)。结构整理本身不证明更快，需在实施时比较编译、链接、运行和定位失败的成本。

#### 三个具体例子

**费用。** 公开费用函数测试拥有阈值前/等/后、各费用分项和 service tier 倍率的完整矩阵；协议测试证明响应 usage、响应模型和 tier 正确传入计算；贯通测试证明 Models 路径与遥测计量没有漏接或重复计费。无需在每个贯通场景重跑全部数值组合，但不能删掉协议接错字段的反例。

有限抽样确认 usage_cost.rs:808 直接检查费用与倍率；service_tier_cost.rs:218 起检查真实请求路径、请求/响应 tier 和序列化，二者不能整体互相替代。特别是 usage_cost.rs:856 的 openrouter_image_parse_usage_uses_base_rates_without_tiers，没有调用图像解析或图像入口；:872 将 base 与相同参数的 calculate_cost 再比较。它只额外确认 fixture 跨阈值，不能作为图像实际路径的证据。建议补真正图像入口反例，或把数据自检合入该反例；不能据此推断仓库其他位置完全没有图像费用覆盖。

**Schema。** schema_engine.rs:31 的 compile/check 与 schema_public.rs:416 的参数归一化/结构化错误是不同契约；两者都走公开 API，不能仅因 engine 文件名把它迁到私有测试。可以共享关键字/边界输入表，但 bool 验证、归一化结果、错误位置各保留对应断言。schema_public.rs:453 的旧 Pi 1.3 差异说明须对当前快照重判，不按“年代久”直接删除。
[#268 拆分后的 compile/check 语料用例](../../../../crates/opi-ai/tests/schema_engine/corpus.rs#L7)

**WS 与遥测。** 自有 codec 的分帧/取消/恢复机制、本地 Upgrade/代理/TLS、Codex session/fallback 是不同故障面；依赖替换后，机制测试随实现责任退役，公开行为契约由新旧 backend 共同通过后保留。遥测中立行为转到 opi-telemetry；AI schema/费用/入口埋点仍归 opi-ai；跨任务传播与真实协议路径保留少量贯通案例。telemetry_carrier.rs:177 的精确 poll 行为和 codex_telemetry.rs:286 的回调/父 span 行为不同。涉及“consumer Drop 停止 producer”的旧预期须按已确认生命周期重写，不能只搬位置后继续冻结错误目标；中立 noop 与 tracing 适配的现有差异也不能照搬成新合同。

以上源定位均相对 D:/Luiz/Odradek/opi/crates/opi-ai，绑定地图 HEAD。根代理复核了所列片段；仍不是全量测试审计。

#### 合并、迁移与删除的准则

不要求一函数一测试，不设删测比例或 unit/integration 数量比例。首先判断测试保护什么，再处理组织形式：

| 处理 | 适用条件 | 必须留下的证据 |
| --- | --- | --- |
| 保留 | 能发现其他案例发现不了的输入、阶段、交错或接线错误 | 对应契约/回归风险和实际断言 |
| 迁移 | 断言有价值，但访问边界、职责归属或 target 不合适 | 移动前后案例标识、适用配置、语义一致；检查并行污染 |
| 合并/参数化 | setup 或断言结构相同，输入行及契约仍各自成立 | 每一参数行保持可辨认 ID 与失败输出，不能压成一个无来源的大 bool |
| 替换 | 旧预期基线失效、只验证替身、或未真正触达声称的行为 | 目标来源、旧风险由何处接管、真正行为断言；有意差异另记 |
| 删除 | 同一输入类、时序/错误阶段、副作用、资源条件已被替代断言覆盖，或职责已明确退役 | 旧案例→目标契约→替代案例/退役决定；没有独特风险遗失 |

删除审查应问：“将这里的真实错误重新引入，替代测试能否失败？”只看相同行覆盖率、都断言 OK、名字相似或同样调用某 helper，不足以删除。简单委派可以由公共集成测试直接覆盖；公开辅助函数本身承诺的语义仍需要明确断言，不接受顺带运行过即算覆盖。

优先复用最终验收的契约索引来记录映射，不另造一套长期维护的测试台账。均一的参数案例可按族记录映射规则和展开列表；异常单列。高风险阈值、终态、鉴权 fallback、旧刷新发布、重复计费需要具名错误变异验证；不强制为每个简单重复测试制造 mutation。既有参考标准的关键 mutation 清单与定量执行预算留待验收决策冻结。

明确可迁移的测试探针例子是 src/test_support.rs:719 的 force_sequence_exhausted，直接改变 UUID 私有 sequence；耗尽机制转入 crate 内测试，公开正常生成与错误类型合同保留。不能把整个 test_support/Faux 都按测试泄漏删除。

#### Pi、成熟 Rust 项目与差分证据

Pi 当前快照有 183 个 *.test.ts；它们提供回归族和输入样本，不能按文件名一对一生成 Rust 测试，也不是全部能力的唯一分母。除展开参数化案例外，还需补公开能力、状态迁移及错误阶段。旧验收文档的 185 文件基线及已发现的 13 个源码变化需重基准，保留来源差异。

本地 Pi 样本提供三种互补思路：

- streaming-tool-arguments.test.ts 通过多个协议路径复用长工具参数场景，并在 :367–371 检查规模增长；相同数据用于不同 Adapter，不自动算重复。
- bedrock-stream-state.test.ts:6 起替换 SDK 事件源，精确验证自身状态与临时字段；它不能单独证明 SDK 实际序列化/签名/传输配置正确。
- models-entry.test.ts:11–59 在子进程限制重型依赖加载，并从源码和包入口运行 Faux。Rust 应验证实际 feature/依赖图与发布包消费者，而不是照抄 JS loader hook。

成熟项目同样按风险组合：reqwest 既测私有代理规则，又通过本地服务器测公共 Client；Tokio 既有私有 Loom 交错探索，也有公共 oneshot 行为测试。出处与固定源码见[已核实的 Rust 生态与测试研究](../evidence/rust-ecosystem.md#成熟项目的实际测试布局)。借鉴其职责划分，不复制其全部测试设施；只有 opi 自有并发状态机需要对应模型验证，不重测 Tokio 本身。

差分共享原始输入、切分计划、可控时钟/顺序和已批准的比较规范，TS/Rust 各自驱动真实入口；不得共享被验证的解析/转换逻辑。预期保留来源快照、运行时/依赖版本、数据 hash 和原始轨迹。比较覆盖请求、事件、回调、result、Store 与资源观察；不能仅比最终文本，也不能通过统一排序/丢字段把差异洗掉。比较器自身必须能拒绝删事件、丢 usage、改工具 ID、null 变缺失等反例。

快照缺模型 JSON 仍是相关原版执行的证据缺口。旧 golden、Rust 自产 expected 或手抄结果不能冒充当前 Pi 运行记录；阻塞的行明确标记，后续补源并重放。私有状态无法外部表达的断言留在机制测试；公开语义的观测不足不能靠私有探针伪装完成。

#### 执行入口与可重复性

- 默认检查使用合成凭据、局部临时存储和受控本地 peer；测试运行环境拒绝非回环网络。Cargo 的 --offline 只限制依赖获取，不会阻止测试程序自行联网，运行隔离必须单独验证。
- 时序测试使用屏障、显式推进、可控时间或协议握手确认状态；超时负责防止挂起，不以长 sleep 推测“已经开始”。保留错误、显式取消与仅释放消费句柄的差别；已知 Pi pending 路径在边界内观察，不无限等候，也不自动改为完成。
- 属性/fuzz 针对自有解析、边界与状态组合；最小化失败样本进入确定性回归。性能/资源专项固定输入规模和环境，不能用输出正确代替泄漏/分配检查。
- 外部消费者扩展 scripts/pi-sdk-conformance，实际消费打包产物，用独立依赖图验证核心+Faux、必要扩展与 README 示例；工作区 tests/external_sdk.rs 不能独立提供这份证据。doc-test 的 no_run 仅表示编译；可离线的示例还要运行。
- live 只在明确启用且有授权的入口运行，机器上碰巧存在 key 不触发调用。缺凭据、工具或数据标未验证/受阻；不把 skip 计为通过。开发、合并和发布的执行频率、features/平台集合与耗时预算由验收票统一冻结。

#### 建议实施顺序

固定来源与案例清单 → 映射主要契约和真实消费者 → 先整理 target/helper 并保留断言 → 补真实入口与已确认生命周期的覆盖缺口 → 小批迁移/合并并验证替代案例能发现原风险 → 最后删除已证明冗余或退役的案例。任何原失败先记录原因，不允许通过批量重录预期或移除不合新实现的测试变绿。

#### 本轮征询（已确认，见 Answer）

是否采用以上测试政策：按契约确定主要验证位置，保留能发现不同错误的少量贯通验证；允许精确的联合测试替代简单私有单测，不要求每个函数各配测试；合并/删除以输入、时序、副作用和反例可检测性为依据，同时保留公开小能力的明确行为断言？

推荐采用。取舍是先做有限的案例映射和缺口修补，再逐批减少重复，不能立即按文件大小/数量做大规模删测。确认后进入 README 信息结构，具体运行矩阵和定量门槛在验收决策收敛；本轮不执行测试重构。

## Answer

2026-11-06，用户以“同意，继续推进”确认本票测试政策。

- 测试按访问边界和契约职责组织：私有机制/私有组合在 crate 内，公共契约、真实协议和贯通场景从外部验证。每项契约有一个主要验证位置，其他位置保留能发现不同错误的互补证据。
- 不要求每个函数各配单测，不设测试数量或删测比例。简单私有委派可由精确的联合测试覆盖；公开小能力仍须有明确的行为断言，不能以被顺带调用作为完成证据。
- 先整理 target、共享搭建与数据，再逐批迁移/合并/替换/删除；删除以输入类别、时序/错误阶段、副作用、资源条件和反例可检测性为依据，保留旧案例到替代验证的映射。关键风险使用具名错误变异，不为所有简单测试机械增加 mutation。
- 复用最终验收的契约索引；Pi 当前快照的测试按案例与回归族映射，不逐文件照搬。TS/Rust 差分共享输入与比较规范，不共享被验证的语义实现；来源、原始轨迹、比较器负例及证据缺口保留。
- 默认离线、受控本地服务与合成凭据；时序用可控屏障和明确推进。外部包消费者、可执行示例、属性/fuzz、资源及显式 live 各有适用入口；执行频率、配置矩阵与定量预算由验收决策统一冻结。
- 当前具体缺口及迁移建议进入实施清单，包括图像费用测试未触达声称入口、UUID 私有测试探针、旧生命周期预期。有限抽样不构成整组测试可删的证明；本次确认不授权跳过替代验证。

本轮仅确定后续测试重构依据，没有修改或执行生产测试；产品测试影响为 none。

2026-11-19 复核发现，历史的 73 个根 integration target 已不再是当前
基线。当前工作树有 102 个根级 `tests/*.rs`，Cargo metadata 有 112 个 test
target，其中 19 个仍由自动发现产生，11 个 provider profile target 复用同一
源文件。该数字只用于维护性盘点，不构成删测目标。后续按
[#209](../issues/209-maintain-module-test-and-catalog-boundaries.md) 先生成
target/feature inventory，补齐 `required-features` 和语义 manifest，再处理
同一公开 seam 的子模块聚合。`test_support` 的正式 Faux API 与私有状态探针
也必须分别计量，不能因为 `lib.rs` 的 doc-hidden 导出而整体删除或整体当作
正式公共能力。

2026-11-20 的落地顺序为 [#215 target 归属](../issues/215-own-integration-test-targets.md)、
[#216 按改动选测](../issues/216-select-tests-by-changed-owner.md) 和
[#217 classifier 分组试点](../issues/217-partition-classifier-contract-tests.md)。
本轮改名命中的五个目标及各自保护的失败模式已列入
[#210 的测试矩阵](../issues/210-audit-owners-and-name-classifier-adapters.md)。

#215 的[测试归属清单](../../../../crates/opi-ai/tests/targets.tsv)已覆盖全部 Cargo 集成目标。
Cargo 管理源路径和 feature 准入条件，清单管理主要职责与具名 runner 绑定。
维护者可用 `python scripts/pi-sdk-conformance/check_test_inventory.py --list` 查看合并结果。

日常改动使用[按路径选测入口](../../../../crates/opi-ai/tests/README.zh.md#按改动选测)。规则随其验证的行为维护。调整协议断言时，同时核对直接 wire、共享分派和确实调用该 adapter 的跨层用例；迁移测试时，逐个核对原用例名与 runner。共享 helper 按物理模块引用找到集成目标，私有库测试和归档消费者的额外绑定在选择器中显式维护。改变 feature、装配根或未覆盖路径时，计划会列出扩大检查的原因。实施证据见 [#216](../issues/216-select-tests-by-changed-owner.md)。

[#217](../issues/217-partition-classifier-contract-tests.md)保留一个 classifier Cargo 目标，并为四种 wire、Models 生命周期、跨协议行为和声明顺序提供各自的 libtest 模块。后续按同一方式分组时，先在原 feature 下保存完整用例名，再核对旧名到新名的映射与所有具名选测入口。共享搭建留在其实际消费范围内，协议响应与行为断言随各自模块维护。

[#218](../issues/218-separate-faux-and-private-probes.md) 用真实归档验证公开 Faux 的工具调用、deferred 提交与 fetch。UUID 耗尽用例在 `identity` 的 `cfg(test)` 模块直接设置私有序列。测试 fixture 按消费范围放置，Bedrock 的 AWS transport 替身由单个 integration 目标拥有。

2026-11-22 的复核发现，provider feature-profile 仍是唯一的多 target 单源映射。
[#265](../issues/265-split-provider-profile-test-sources.md) 已将 11 个 profile target
拆为 11 个独立源文件。当前清单为 124 个 target、124 个源文件。后续新增 profile
默认沿用一 target 一源，真正共享的源码只允许作为显式记录的例外。

[#268](../issues/268-partition-oversized-protocol-fixtures.md) 的 Anthropic protocol
批次保留一个 `anthropic_fixtures` executable，把 154 项公开协议断言拆为 target 内的
`sse`、`constrained_sampling`、`adapter` 三个可过滤 topic，并让
`tests/anthropic_fixtures/fixtures.rs` 成为唯一共享 SSE fixture owner。目录子模块用于
失败定位和职责边界，Cargo target 仍由 `tests/anthropic_fixtures.rs` 统一管理。没有为
每个 topic 增加 executable，也没有把没有直接 consumer 绑定的 fixture 复制到 package。

剩余结构工作按 [#266](../issues/266-separate-route-context-contracts.md)、
[#267](../issues/267-split-models-facade-implementation.md)、
[#268](../issues/268-partition-oversized-protocol-fixtures.md)、
[#269](../issues/269-classify-shared-test-support.md) 和最后的
[#270](../issues/270-enforce-structure-and-test-inventory.md) 顺序推进。这个顺序先
收敛 adapter 与 provider contract 的依赖，再处理 facade locality，随后才移动大
fixture 和 shared helper，避免测试重排掩盖生产结构变化。

本批 all-features Clippy 还发现两个 image fixture 仍初始化已删除的 `compat`
字段，分别由 [#271](../issues/271-repair-stale-image-fixture.md) 和
[#272](../issues/272-repair-stale-openrouter-image-fixture.md) 修复。两项受影响
target 共 67 个测试通过，all-features test Clippy 只保留 3 个既有生产 warning。
