# opi-ai 对齐 Pi 1.5.0：重构方案与实施交接
Status: active
Owner: user
Baseline: opi 90f953d5ee288c165dc295bfdfb7cb7a05d7cecd; initial worktree clean; upstream D:/Luiz/Odradek/opi/.pi/pi-1.5.0; SHA256 content manifest 959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072

日期：2026-11-07。本文件承接已完成的[决策地图](map.md)，作为实施的单一方案入口。active 表示已确认的目标范围仍有效。第一阶段已完成并提交，证据见[第一阶段报告](phase1.md)。用户随后授权继续推进；第二阶段首个切片[建立中立遥测基础与可选 tracing 适配](issues/02-neutral-telemetry-foundation.md)已完成，结果及全量回归限制见[切片报告](phase2.md)。随后完成第二个切片[分离纯值、公共错误与认证目录契约](issues/03-values-errors-auth-catalog.md)，结果见[基础边界报告](phase2-foundation.md)。两票完成第二阶段基础整理。第三阶段已完成首个切片[显式启动 chat 请求并分离执行与消费](issues/04-started-chat-execution.md)，见[交付与剩余工作](phase3-started.md)；随后完成[迁移 agent 请求消费与显式中止](issues/05-agent-execution-consumers.md)，状态见[消费者迁移报告](phase3-consumers.md)；接着完成[让普通 chat 入口独立执行](issues/06-default-chat-execution.md)，交付、检查与边界见[默认入口迁移报告](phase3-default.md)；随后完成限定范围的[AI telemetry seam 与 source/result 尾部](issues/07-ai-telemetry-source-tail.md)，见[切片报告](phase3-ai-telemetry.md)；当前又完成第四阶段首个[可选 telemetry tracing feature](issues/08-optional-telemetry-feature.md)，见[feature 报告](phase4-features.md)，以及[普通 Models chat 自动安装 AI observer](issues/09-ordinary-chat-ai-telemetry.md)，见[切片报告](phase4-ordinary-ai-telemetry.md)。普通 Models chat 已自动安装 observer；随后 deferred fetch 的 `ChatStream` 与
`pi.ai.poll` 自动观测、prepared attempt 的 observed helper，以及供独立 provider
使用的公开 `ChatStream::new_with_telemetry` 均已落地；其 `ChatStream::start()` 正常
后台终态也会完成同一 observer。无观测的 raw ChatStream、
prepared raw attempt、非 chat 其他路径仍开放；prepared observed helper 的跨 attempt
聚合已明确为调用方责任，不自动合并；完整 provider
覆盖、feature 矩阵与 Pi 差分仍开放。各票独立维护检查范围，不代表完整能力对齐
通过。答案细节以各决策票的 Answer 为准，研究报告只提供证据。

最新的普通 chat telemetry 切片又把 `pi.ai.request` observer 的安装边界对齐到
`ChatCall` 构造：显式 context 在 setup/authentication 开始前即可看到未结束的
span，setup failure 使用规范 `CallFailure.message()` 结算；后台执行仍保持异步，
不会把构造时点误写成同步 provider 调用。

随后修正 AI telemetry `provider_error` 状态消息的长度单位：错误文本现在按
UTF-16 code unit 限制为 500，非 BMP 文本不会因 Rust `char` 计数而超出 Pi 的
边界；孤立代理对无法由 Rust `String` 表示，已作为明确的 Rust-native 差异单独
记录。随后补齐 `StopReason::Pending`，将各 provider 的初始 partial 从 `Stop`
改为非终态 `Pending`；AI telemetry 对 pending/deferred poll 均不累计
token/cost，并将 `toolUse` 与完整 stop vocabulary 对齐。canonical Rust
`StopReason::as_str()` 的 `tool_use` 仍保留，telemetry 通过独立映射使用 Pi
词汇。随后修正可选的 `response.id`：provider 没有报告 response id 时保持属性缺失，
不把它改写为空字符串。request duration metric 也改为只携带 Pi 定义的
`provider`、`model`、`stop`，不复用 span 的高基数和 usage 结束属性。

当前 pricing/cost 切片已完成：`Usage.cost` 区分 catalog 分量、catalog total
和 provider-reported total；tier 选择、cache-write 1h 子集以及 OpenRouter
非 BYOK/BYOK 账单规则按 Pi 1.5.0 固定。通用 OpenAI/Anthropic 解析器保留
OpenRouter 扩展字段但不应用它们，只有对应 provider route 才进行账单覆盖。
专项测试、两个 provider fixture 集合和 `model_wire_metadata` 通过；Radius OAuth
固定回调端口的并行竞争已修复，platform delivery feature 契约也已更新，schema-engine
的 sibling resource identity 回归也已收敛。全量 `opi-ai` 包测试通过。详情见
[pricing/cost 切片报告](phase4-cost-pricing-semantics.md)
和[检查证据](evidence/phase4-cost-pricing-semantics-checks.json)。

随后完成独立消费者补充验收：实际 `opi-ai` `.crate` 在 workspace 外部消费者中
分别以 `default-features = false` 和显式 `telemetry-tracing` 编译并执行 README
三条 onboarding 流程；metadata、controlled-peer 回归、执行前启动、最终结果和
tool-result continuation 均通过。证据见[独立消费者报告](phase4-readme-package-profiles.md)
和 `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`。
随后补上双语 README 的任务导航，将独立接入路径与后续 provider/协议参考分开；
实施记录见[README 导航票](issues/27-readme-task-guide.md)。
随后补上公共 API 总览，固定中立入口与 Provider/Telemetry 的责任边界；实施记录见
[README API 总览票](issues/28-readme-public-api-map.md)。
随后补上双语高级能力参考索引，将 Provider、认证、Telemetry、cost/pricing、
WebSocket、Deferred 与测试包证明集中成入口；实施记录见
[README 参考索引票](issues/29-readme-reference-map.md)。
随后补上双语 feature profile 矩阵，固定 `telemetry-tracing` 与
`default-features = false` 的真实语义；实施记录见
[README feature profile 票](issues/30-readme-feature-profile-matrix.md)。
随后同步 `opi-telemetry` 独立 README 的 AI instrumentation 边界，修正过时的
future-migration 描述；实施记录见
[telemetry README 边界票](issues/31-telemetry-readme-ai-boundary.md)。
随后把长篇 cost/pricing 规则拆成 crate 内双语专题文档，主 README 仅保留稳定摘要、
旧标题锚点和包内链接；实施记录见
[README cost/pricing 专题票](issues/32-readme-cost-pricing-topic.md)。
随后把 Provider、Catalog、认证与 Refresh 参考拆成 crate 内双语专题文档，主 README
保留稳定锚点和 Core API 总览；实施记录见
[README Provider/Auth 专题票](issues/33-readme-provider-auth-topic.md)。
随后清理公开手册中的历史任务、阶段和验收快照标签，使 README 与包内专题回到面向
能力和行为的开发者手册；内部 issue、report、evidence 继续保留完整追溯。实施记录见
[公开手册历史标签清理票](issues/34-remove-internal-labels-from-public-manuals.md)。
随后按 `decisions/09-readme-contract.md` 的任务顺序重构双语主 README，并把传输、
公共工具、Provider wire/operation 和测试/包消费细节拆为四对包内专题；三份真实
onboarding 示例继续与中英文 README 逐字绑定。实施记录见
[README 公共手册重构票](issues/35-readme-public-manual-restructure.md)。

## 目标与范围

让第三方 Rust 开发者按 opi-ai 文档，仅依赖 opi-ai 及其声明的库依赖，就能在普通项目中使用与指定 Pi 1.5.0 对应的能力。无需 opi-agent、opi CLI、仓库私有路径或隐式全局初始化；可以保持同一 workspace，不要求独立仓库。

采用 Rust 原生接口，保持完整能力映射，可观察行为逐项验收；有意差异单独讨论并记录，不能算严格等价通过。目标不仅是聊天，也包括图像、分类、deferred、Faux、认证、目录、工具/schema、transcript、扩展、compat、公开辅助能力及包级工具的对应关系。

本方案定义重构范围、依据和迁移次序。基准与映射保持冻结；当前开始按后续阶段迁移产品源码、Cargo 配置、文档与相关测试。第二阶段按可独立验证的切片推进，中立遥测与纯值/错误、auth/catalog 分离均已实施。已增加显式 start 的独立执行者，并迁移 agent 主循环和 preparation summary 消费者；普通 chat 入口现已调度独立执行者，受影响回归与实际包消费检查通过，工作区仍有两项既有失败；随后建立了 AI telemetry 手动 seam、`ChatStream::result()` 的 source 尾部结算，在普通 Models chat 路径自动安装 observer，并把 `opi-telemetry` tracing 适配改成默认开启但可关闭的 `opi-ai` feature。deferred fetch 现在复用同一 `ChatStream` 结算并自动产生 `pi.ai.poll`；prepared attempt 保留原始 raw seam，并新增 observed ChatStream helper；调用方持有 raw `EventStream` 时可通过公开 `ChatStream::new_with_telemetry` 复用同一 observer。自动接入每条 provider、raw/prepared/non-chat 其他路径、跨 attempt 聚合、完整 feature 矩阵和完整 Pi 差分仍未完成。本次没有发布、真实 provider 调用或整个 pi-coding-agent 的实现。opi-agent/opi-coding-agent 中已有消费者要随本次公共接口调整迁移，完整 coding-agent 能力对齐另行安排。

## 已确认的设计

| 议题 | 实施方向 | 权威决策 |
| --- | --- | --- |
| 独立接入与对齐 | Rust 原生表达；完整能力与可观察行为分别举证；没有预先批准的行为例外 | [公共契约](decisions/04-public-contract.md) |
| telemetry 拆分 | 新增中立 opi-telemetry，AI 埋点留 opi-ai；宿主拥有后端初始化与输出策略 | [遥测边界](decisions/05-telemetry-boundary.md) |
| 请求生命周期 | 执行与消费分离；释放消费句柄不隐式取消；请求执行者负责资源与遥测结算 | [生命周期与失败语义](decisions/11-telemetry-lifecycle.md) |
| 工程结构 | 一个 opi-ai 加一个 opi-telemetry；按责任组织模块，保留实际高级入口并迁移调用者 | [模块结构与公开入口](decisions/06-module-structure.md) |
| 第三方依赖 | 成熟依赖、薄适配优先；必要补充与最小自研须有具体反例及同一契约验证 | [复用与自研边界](decisions/07-dependency-boundary.md) |
| 测试 | 按访问边界和契约确定主要验证位置；用替代证据去重，保留能发现不同错误的贯通验证 | [测试归属与去重](decisions/08-test-boundaries.md) |
| README | Pi 风格任务手册；常用例子在正文，长流程在专题，精确合同在 rustdoc；中英文共用受测示例 | [开发者文档](decisions/09-readme-contract.md) |
| 默认配置与验收 | 默认完整内置能力、可选最小构建、显式注册；本次建立完整基线，后续按影响补证 | [验收政策](decisions/10-acceptance-policy.md) |

### telemetry 的实际边界

opi-ai 依赖 opi-telemetry；中立 crate 不反向依赖 AI、agent 或 CLI。中立范围覆盖 AI 所需的 context/span、属性/事件/状态、metrics、schema 描述、no-op、轻量记录器与共享一致性验证。tracing 是可选适配，不成为所有用户必须接受的公共类型。

AI 专属 schema、请求/poll 的观测入口、token/cost 和结算规则仍在 opi-ai。宿主拥有 exporter、采样、脱敏、初始化和 flush/shutdown。Store/query/render、数据库和全部 pi-telemetry 能力不因拆 crate 自动纳入；传播与 links 按已映射的公开能力和实际消费者落实。

“消费者不再读取”与“请求终结”是两件事：错误、显式取消、超时和停止消费分别验证，不能在 stream 返回、消费 Drop 或最后一个 context clone 释放时一律结束 span。原版异常路径与工程资源要求发生冲突时保留两份证据，回到具体差异决策。

### 建议目录与依赖

以下是责任布局，目录名、文件粒度和逐符号签名可在实施中调整，并非每个目录都公开：

```text
crates/
  opi-telemetry/       中立观测合同、no-op、记录器、可选适配
  opi-ai/
    src/
      lib.rs          常用公共门面
      types/          消息、模型、图像、分类、deferred 等纯值
      errors.rs       通用错误合同
      models/         集合、查询、路由与认证/刷新编排
      provider.rs     服务对象与扩展合同
      providers/      内置工厂、Faux
      api/            操作合同、原生选项、协议适配
      execution/      高级控制；私有执行驱动与资源所有者
      stream/         事件、消费与结果合同
      auth/           凭据、Store mutation、OAuth 与云认证
      catalog/        合同、来源与资产分离
      transcript/
      schema/
      tools/
      transport/      必要配置与宿主注入；backend 默认私有
      telemetry.rs    AI 专属 schema 与 instrumentation
      utils/          确有公共契约的辅助能力
      compat/
    tests/            公共合同、协议、跨模块与消费验证
    examples/         文档共用的完整程序
```

纯值和通用语义不反向依赖 SDK、网络或内嵌目录。auth/catalog 合同与具体后端分离；Models 通过操作合同路由，不直接包含某个 provider 的特殊规则。execution 拥有执行期资源，stream 表达消费关系；不能把旧的巨大 Models 搬成新的巨大通用 trait。

PreparedRoute/PreparedProviderCall、attempt/retry admission、Store mutation 及必要 transport 注入均有真实消费者，迁移时保留能力。测试专用状态探针回到 crate 内，正式 Faux 继续公开。保留 Tokio 路线，本次不增加多运行时、no_std 或预先拆分 ai-core/http/ws 等 crate。

### 默认安装与运行时

默认构建包含支持平台上的完整内置模型调用能力，包括内置协议/provider、目录和必要认证。调用者可关闭默认 features，获得核心＋Faux 的最小配置，再按需启用能力。最小配置排除可选网络 backend、云认证 SDK、内嵌目录与具体遥测适配，但不等于零依赖或 Tokio-free。

普通 Models 构造为空；单 provider 注册和显式全内置构造并存。编译进某个 provider 不意味着自动注册、联网或登录。初始化时点按各入口实际合同验收。默认完整不等于开启 exporter、CLI、测试设施和所有互斥后端，也不消除 Rust 编译依赖成本。具体 feature 名称、有效组合及成本通过实现验证，参见[Pi 默认策略的源码依据](evidence/default-configuration.md)。

## 测试与 README 的组织方式

| 验证对象 | 主要位置与方式 | 去重约束 |
| --- | --- | --- |
| 私有解析、状态机与只能内部观察的不变量 | 所属模块内测试；按需要参数化、属性验证 | 不为每个私有函数机械配测试；精准联合测试可替代简单委派单测 |
| 公共值、公开小工具和 API 合同 | 外部契约测试 | 小能力也须有明确断言，不能以“贯通时调用到了”代替 |
| 请求、流事件、错误、取消、Store 与跨模块协作 | 受控 peer 的协议/贯通测试，必要时独立 TS/Rust 差分 | 保留独有的接线、时序与副作用反例，不重复完整用例矩阵 |
| 安装、features、真实包、文档程序 | 复用并扩展现有外部包消费 harness | 仓库内编译不能代替实际打包产物的独立消费 |
| 不可信输入、资源、性能、真实服务 | 专项入口，按影响及里程碑预算执行 | 已发现失败样本转成确定性回归；live 显式启用 |

用一份契约索引连接源能力、Rust 入口、主要/互补测试、原始差分记录与旧案例替代关系，不另建重复验收台账。删除测试前证明输入类别、时序、错误阶段、副作用和资源风险仍可被发现；关键风险用具名错误变异检验。先整理 target/helper 并保留断言，再逐批合并或删除，不能凭测试数量、文件大小或旧注释直接删测。

README 从“安装 → 单 provider 的完整调用 → 离线 Faux → streaming/result → 工具续轮”开始，再展开图像/分类、Models/Provider、认证与目录、原生选项、错误与取消、用量与遥测、transcript、扩展/deferred/compat、配置与迁移。常用能力保留可用短例和关键限制，复杂协议细节进入专题，工程历史离开接入正文。篇幅不设任意上限，能力入口不能丢失。

文档例子必须对应真实公开 API。中英文、examples、rustdoc 和包内代码保持一致；快速开始包含可复制的完整 manifest、imports、main 和命令。复用已有 scripts/pi-sdk-conformance 的实际外部执行路径，允许由完整示例提取正文片段，但不以片段编译替代完整接入证明。源码/示例/文档随各阶段同步调整，不把未实现目标提前写成现状。

## 实施顺序与每步完成条件

以下按阶段实施，第一阶段的状态和产物通过上方实施票与报告追溯。每个切片先确定受影响的观察与回归，再调整结构；测试和文档随接口迁移更新，最后一阶段才给完整里程碑结论。

| 阶段 | 工作及产物 | 该阶段完成的证据 |
| --- | --- | --- |
| 基准与映射 | 固定源数据/依赖/运行时身份；建立单一契约索引；清点公开入口、真实消费者和现有案例 | 旧标准与当前快照差异有归属；已知缺口被标记；关键生命周期/别名/异常有可执行观察设计；不把静态结论当原版运行记录 |
| 值类型与基础边界 | 拆纯值/通用错误，分离 auth/catalog 合同与后端；建立中立 opi-telemetry 及必要适配 | 依赖方向可验证；中立 crate 可单独消费；保留认证/Store/目录行为；测试无额外公共探针 |
| 执行与协议组织 | 分离执行者和消费者；接入 AI instrumentation；调整 Models、Provider、API、工厂；同步高级调用者 | 停止消费、全不消费、显式取消、异常、重复身份等关键路径保持合同；业务与遥测分别结算；工作区消费者迁移并通过受影响验证 |
| 功能配置与依赖适配 | 实现默认完整/最小/定制构建；冻结有效配置集合；必要时探证第三方 backend 候选 | 各配置有真实依赖图和外部消费结果；构造与首次调用副作用可观察；候选与现有实现跑同一合同。无证据不顺带替换 backend |
| 测试整理与接入文档 | 按已建立的归属整合 target/helper，补真实入口缺口，逐批去重；完成双语 README、专题、示例与迁移表 | 每次删除有替代映射；关键反例仍被检测；两 crate 的实际包可被普通外部项目使用；完整程序及受控协议执行可复现 |
| 完整对齐里程碑 | 校准并确认数值预算；固定候选/平台/配置；执行或复用仍适用的六门证据 | 给出逐项结果、证据身份、差异与缺口；符合条件才声明对应级别通过；发布动作另行安排 |

基础边界中的中立遥测提取与后续生命周期迁移分别验证，移动文件本身不算语义对齐。已缺源数据只阻断受影响的目录/价格/原版整包结论，其他可独立验证的工作可推进。每阶段的源码改动仍按仓库规则选择检查，不用本表替代具体变更的影响分析。

第三方 backend 的研究是有条件工作：先为真实不兼容点给出输入和观察，再评估复用、适配、补充或自研。当前没有批准某个 WS/AWS 包版本、移除 PMD、重写官方 SDK 或删除现有实现；也不要求为每个 npm 包建立同名 Rust 子模块。未来确有独立消费者和维护收益时，再讨论提取 crate。

## 验收频率与结论

| 时点 | 要求 |
| --- | --- |
| 日常修改 | 仓库要求的检查，加受影响的确定性回归；文档修改不触发完整协议/性能矩阵 |
| 集成与本次中间版本 | 按影响运行离线组合、协议、差分、平台与包消费验证；已有证据经适用性检查可复用 |
| 本次独立重构＋Pi 1.5.0 完整对齐里程碑 | 建立范围、行为、结构、测试、运行、交付六门的完整基线；按冻结配置与已确认预算执行 |
| 后续普通发布 | 沿用已有 CI/发布要求，补因代码、依赖、平台等变化失效的证据；不默认全量重跑 fuzz、耐久和性能 |
| 后续更换基准、重大架构变化或影响无法界定 | 重新建立完整基线；影响有界的 backend 更新只扩大到相关合同及专项 |

行为事实与工程预算分开。30 CPU 分钟 fuzz、10000 次耐久/属性案例、性能倍率等仍是[验收政策](decisions/10-acceptance-policy.md)中的候选值；先明确适用范围、负载、计数、测量环境和预计成本，校准后由用户确认，再冻结正式要求。校准不等于通过，不能在验收失败后改低 expected 并覆盖原记录。

每项区分通过、失败、未执行、受阻、不适用和有意差异。范围限定通过、行为严格对齐、完整六门通过、包含已批准差异的交付、真实服务当前可用，分别给结论。关闭某项 feature 只影响特定配置，不能从总体能力分母删除该能力；缺凭据/数据/工具及 skip 都不能计为通过。

三 OS 原生验证沿用仓库重大里程碑政策。配置集合覆盖最小、默认、单独能力、有效两两与已知高阶组合，以及完整能力；all-features 只有合法时才是一行。记录实际 OS/架构、runner 镜像、工具链、依赖、命令和产物，区分原生运行与仅交叉编译。时间敏感的服务可用声明另需相应新证据。

## 实施前置条件与重新讨论的触发点

这些事项有明确处理方式，不把“规划完成”解释为材料、预算或兼容证据已经齐全。

| 待落实项 | 下一步及阻断范围 |
| --- | --- |
| 目标快照缺实际模型 JSON | 查找与固定快照匹配且可追溯的源数据；补全前目录/价格及依赖该数据的原版运行证据受阻。不得用 Rust 数据或当前最新线上目录冒充 |
| 原版执行环境与实际包材料 | 解析并记录目标 lockfile、SDK/运行时、生成数据与打包入口；对照驱动分别运行真实 TS/Rust 实现，保留原始记录 |
| JS 共享/启动/异常的 Rust 映射 | 优先探证 partial 延后读取/别名、eager 与 first-poll、observer 异常及 pending 等场景；若不能同时满足兼容与工程要求，提出具体差异供用户决策 |
| 逐符号与消费者迁移 | 形成旧入口→新入口表，覆盖现有 agent/coding-agent 和可确认的外部使用方；不能以仓库无引用推定没有外部消费者 |
| feature、平台和工具链精确集合 | 根据实际依赖闭包冻结配置集合与支持条件；MSRV 先按当前声明 1.97 评估，变更需更新支持承诺；验证加法合并下的外部消费 |
| 定量预算与维护成本 | 做标注清楚的校准，提交负载/方法/成本/推荐门槛；数值正式确认前不能签署相关验收通过 |
| 独立包与发布版本 | 按届时版本状态确定适合公共接口迁移的 0.x 版本与两 crate 版本依赖；尚未发布时用仅含候选产物的受控源验证，不能依赖仓库源码 patch |
| 具体第三方替换选择 | 仅在发现真实缺口时做候选兼容探针；新增语义例外、公开宿主要求或维护负担时重新讨论，普通私有实现细节由实施者处理 |

基准身份变更、能力删减、行为例外或支持承诺改变，需要更新对应决策；一般目录名、私有函数布局和不改变合同的实现选择无需反复确认。没有出现这些冲突时按已确认方案推进，不新增一层无依据的审批流程。

## 规划交接记录（历史）

本轮已完成三项研究、八项人机决策及本方案，共十一项决策票；所有回答均保留在各自票中。研究主要为静态源码/文档核查，不能当作原版运行、包消费或性能结果。

规划交接时只改动规划文档及 glossary 中 Model Request 的执行/观察生命周期定义；当时产品测试影响为 none。没有生产源码、Cargo、产品 README 或测试的重构，没有构建、提交、推送、发布或付费调用。

规划交接检查：

- `python scripts/opi-doc-check.py`：PASS。
- `git diff --check`：PASS；新增规划文件另外检查行尾空白。
- 规划与阅读快照的 177 个本地链接：PASS；十一张决策票均有 Answer，地图各索引一次，阻塞关系无环。
- 工作树核对：HEAD 与上述基线一致；committed=none-for-this-task，staged=none，unstaged 为任务中的 docs/GLOSSARY.md，untracked 为本迭代目录。先前各轮留下的同任务文档继续保留，没有其他产品变更。

这些检查只证明规划文件的一致性。实现、TS/Rust 差分、外部包消费、协议、资源、性能和真实服务验收均未运行。

第一阶段的实际观察、检查和剩余材料缺口见[第一阶段报告](phase1.md)，不以以上历史规划检查代替实施验证。后续从“值类型与基础边界”切入，复核消费者和相关源码是否漂移，优先保留已定位的生命周期反例；仍缺的源数据只阻断其实际影响范围。
