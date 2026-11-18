# 确定 opi-ai 的模块结构与公开入口
Type: grilling
Label: wayfinder:grilling
Status: resolved
Owner: Codex for user
Parent: [opi-ai 1.5.0 独立库与能力对齐路线](../map.md)
Blocked by: [核实 pi-ai 1.5.0、telemetry 与使用文档的真实边界](01-upstream-contracts.md), [核实 opi-ai 工程、测试与独立接入的现状](02-current-structure-tests.md), [核实 Rust 依赖复用与成熟项目的测试策略](03-rust-ecosystem.md), [确定独立接入与行为对齐的公共契约](04-public-contract.md), [确定 telemetry 的归属与依赖方向](05-telemetry-boundary.md), [确定第三方能力的复用与自研边界](07-dependency-boundary.md)

## Question

怎样按能力所有者、provider 配置与协议实现组织 Rust 模块，消除重复入口和宽泛的杂项模块？哪些内部模块需要独立契约，哪些只需私有实现；如何组织 catalog/auth/stream/compat/session/telemetry 与公开 facade，并保持独立 crate 接入？

## Comments

2026-11-06 候选逻辑结构，目录名和公开程度尚未确认。

借鉴 Pi 的 provider 工厂与 API 协议分离。provider 是服务配置、鉴权和目录组合；API adapter 拥有 wire 请求/解析与协议状态机；transport 拥有网络机制。一个 provider 可提供多协议，一个协议可供多个 provider 使用。

候选目录如下；它表示职责，不要求一目录一 crate，也不要求全部 pub：

```text
crates/opi-ai/
  src/
    lib.rs                 # 精选公共入口与文档导航
    types/                 # message/model/options/error 等稳定数据契约
    models/                # 注册、路由、刷新与调用编排
    api/                   # 按 wire 协议：anthropic/openai/bedrock/...
    providers/             # endpoint、auth、catalog、compat 的工厂组合
    stream/                # 事件、终态、result、取消和资源生命周期
    transcript/            # 历史与跨模型投影
    tools/                 # schema/参数校验/增量 JSON
    auth/                  # 凭据契约、刷新、OAuth、云来源
    catalog/               # 静态目录、动态源、store
    transport/             # http/sse/websocket/tls 的内部适配边界
    telemetry/             # 仅 AI 域 instrumentation
    compat/                # 确有消费者需要的兼容入口
  examples/
  tests/
  README.md
  README.zh.md
crates/opi-telemetry/        # 已确认拆分；尚未实施
```

不是先搬文件再决定依赖。先给现有入口和状态找唯一所有者，重点拆 models.rs 的注册/刷新/类型/调用状态；保留一套正常用户路径和有需求的高级入口。内部 crypto/TLS/codec/并发实现不因“将来可能复用”全部公开。

features 按真实依赖成本和平台能力设计，优先隔离 core+Faux、目录资产、网络、云认证和可选 SDK。不要为每个品牌机械增加 feature，也不要创建一组互相冲突的 feature。当前 manifest 无 features/optional dependency，目录重排无法单独满足最小依赖要求。

重构验证用外部消费探针：新增一个协议和两个复用它的 provider，无需改既有领域类型和其他 adapter；以真实 Cargo 包验证，不只在工作区 examples 通过。

已确认的[遥测生命周期约束](11-telemetry-lifecycle.md)要求请求执行者与消费句柄分离。模块/公开 API 设计必须明确谁持有执行、取消、result 和清理责任；不要把所有这些职责继续交给接收流对象。具体 Rust 入口还需区分纯构造、启动请求与读取事件，启动动作按 Pi 入口映射。

### 继续讨论：建议采用的结构

2026-11-06，依赖策略已确认。本节是当时提交讨论的结构建议，修订前述概略树；后续确认及公共 Rust 接口迁移政策见 Answer。使用 codebase-design 检查 Module 的 Interface、状态所有者和真实消费者，目录只表达职责，不作为结构合格证据。

#### 包与公共入口

本轮保持一个 opi-ai crate，加上已经确认的独立 opi-telemetry。先用 Rust Module 和可选能力隔离依赖，不预先增加 opi-ai-core、opi-http、opi-ws 等发布单元。

普通调用者以 Models、消息/模型类型、选项、事件/result、错误为入口。高级调用者通过职责明确的 Interface 扩展 Provider/API、Store/认证来源、调用执行和必要的宿主能力。lib.rs 精选常用 re-export；不再把所有实现模块当作同等公共入口。完整 Pi 能力映射仍是约束，减少入口学习成本不能删掉原有公开能力。

候选目录如下；不要求小模块立即建目录，Rust 的 foo.rs + foo/ 子模块形式也可使用。公开程度按 Interface 逐符号确定，树本身不是一张全 pub 清单。

```text
crates/
  opi-telemetry/             # 已确认：中立遥测合同与可选适配
  opi-ai/
    src/
      lib.rs                # 常用入口与 rustdoc 导航
      types/                # message/model/usage；含图像、分类、deferred 值类型
      errors.rs             # 通用错误合同；专用错误仍归所属 Module
      models/               # provider 集合、查询、路由、鉴权/刷新编排
      provider.rs           # Provider：服务对象的公共合同
      providers/            # 内置 provider 工厂与正式 Faux
      api/                  # 操作合同、原生选项、各 wire 协议 Adapter
      execution/            # 高级调用控制；内部 driver/resource 状态机私有
      stream/               # 事件、读取/result 等公开流合同
      auth/                 # 凭据、Store/修改协议、OAuth、云来源
      catalog/              # 目录合同与来源；内嵌资产隔离
      transcript/           # 历史、正规化与重放规则
      schema/               # schema 值/方言/构造等自有语义
      tools/                # 工具参数、增量 JSON、受约束输出
      transport/            # 必要的配置/注入接口；HTTP/WS/TLS backend 私有
      telemetry.rs          # AI 域 schema 与 instrumentation
      utils/                # 已有公开通用辅助能力；不作私有杂项堆
      compat/               # 有真实合同的 Pi/既有接口兼容
    examples/
    tests/
    README.md
    README.zh.md
```

types 只装确定性数据合同，不把当前含 cancel、回调、transport 的整个 Request 塞进去；运行控制归 execution/调用选项，协议专属选项归 api。消息/模型类型不得依赖具体 SDK、CLI、内嵌目录数据或网络客户端。图像、分类、deferred 保留各自类型与能力判断，不被统一成任意 JSON 的聊天请求。

#### Provider、API 与 Models 的区别

- Provider 是带身份、目录、鉴权与操作能力的服务对象，可以组合多个协议。不能只剩一行 endpoint 配置。
- API Adapter 处理一个 wire 协议及其选项、请求/响应、错误和流状态；多个 provider 可以复用它。
- Models 是集合与调用门面：管理选择/查询/认证/刷新编排并路由请求，不手写供应商 wire 规则。
- 内置 provider 工厂是装配位置；Faux 是正式可独立使用的 provider，不能与 AWS 测试运输替身捆在一起。
- 对外 provider/API 标识保留扩展空间；内建枚举可作实现细节，不能阻断新协议 ID 或自定义 provider。

当前 Provider trait 同时带目录、prepared dispatch、attempt 和 resource 方法；typed ChatApi/ImageApi/ClassifierApi 又有独立入口。建议共享底层执行与状态所有者，按消费者保留清楚的服务对象和协议扩展 Interface；不通过新增另一套万能 trait 再叠一层。

#### 请求执行和消费的放置

execution 内部的请求执行者拥有 setup、协议源的推进、转发、结果和请求级清理责任；它调用 AI instrumentation，普通 consumer 通过 stream Interface 读事件或等待结果。消费句柄 Drop 不丢弃执行者，不把 channel closed 变成取消。具体 spawn、调度和跨任务句柄是 Implementation，统一合同不要求每条操作使用相同 task 数量。

保留 agent 真正需要的高级控制：冻结路由、准备调用、attempt、retry/strict admission、资源清理。普通 Models 路径与高级控制路径复用同一套规则，避免 SDK 自动重试与 agent 重试组合后重复发送。部分业务要求的提交/轮换不得因普通消费 Drop 误杀。

#### 允许依赖方向

1. types 与纯语义 Module 位于底部；错误文本/脱敏/费用逻辑不能通过 HTTP 或某一供应商实现反向拉入网络。
2. API 合同依赖领域值、选项与流合同；具体 API Implementation 才依赖 transport/SDK。Models 通过合同调用，不直接导入 Bedrock 等实现的规则。
3. providers 的工厂组合 API、auth、catalog；Models 不为支持自定义 provider 强制加载内置工厂全集。
4. auth/catalog 的合同类型与云后端、文件实现、HTTP 来源、内嵌资产分离。给核心调用传入自定义 Store 不要求带所有内置来源。
5. execution 拥有调用生命周期；AI telemetry 只观察其已有路径，通用 opi-telemetry 不依赖 execution 或 AI。
6. 私有 helper 留在所属 Module；utils 只承载真正已有的公共辅助能力。测试专用状态操纵回到 crate 内，正式 Faux 能力保留为公共 Interface。

结构验收按逻辑职责和生产/构建/测试依赖图进行，不能仅凭上述目录或 Rust 允许的模块引用宣布无环。

#### 真实消费者与迁移依据

| 当前表面 | 已核实消费者 | 建议归属与限制 |
| --- | --- | --- |
| Models/stream/complete、记录与简单选项 | examples/quick_start.rs:25,75；quick_start_stream.rs:70；tool_loop.rs:133 | 保留主入口，内部拆注册、刷新和执行所有者 |
| Provider/ProviderRegistry/ApiMappedProvider | opi-agent/src/extension.rs:197；opi-coding-agent/src/provider_factory.rs:141,168,1212 | 保留扩展能力，先迁移调用者，再合并重叠实现 |
| PreparedRoute/PreparedProviderCall/重试许可 | opi-agent/src/agent_loop.rs:539,624,1132,1211；evidence.rs:1319 | execution 的真实高级 Interface；不能误当测试泄漏删除 |
| MutationQueue/Handle/Receipt/Decision/SlotView/Barrier | opi-coding-agent/src/credential_store.rs:1048,1132–1177,1251–1266 | auth/store 的持久化实现者合同，迁移 keychain 后才调整公开路径 |
| HttpClient 与云来源 Adapter | opi-coding-agent/src/provider_factory.rs:39–40,358 | 保留必要配置/注入能力，backend 不进入通用消息类型 |

相反，test_support::force_sequence_exhausted 属已核实的私有状态测试探针；它与正式 MockProvider/Faux 不能一并处理。工作区有无搜索命中只提供局部证据，外部第三方消费者仍未知。

已核实的结构阻碍还包括：provider.rs:673 的通用错误脱敏依赖 http::safe_excerpt，:963 的错误分类依赖 Bedrock retry；models.rs:3986 直接调用 Bedrock 模型规则；catalog/mod.rs:50 无条件 include_bytes；auth.rs:1028,1043 常开 ADC/AWS；test_support.rs:574 起混入 AWS 替身。先调整这些责任归属，再验证 feature 闭包。

#### 功能配置与测试位置

建议保留现有 Tokio 路线，本轮不自动扩展为多运行时或 no_std 工程。最小配置应可运行 Models+Faux 与所需纯语义能力，排除可选网络/云 SDK/内嵌目录/具体遥测后端；单协议/常用配置按真实依赖打开对应能力，完整配置覆盖总能力范围。具体 feature 名称、default 集合与发布验证集合在验收/交付方案中冻结。

Cargo feature 是依赖图中的加法合并：能力开关不能意外改变无关 provider 的语义，也不能只验证 all-features。这个规则来自 [Cargo features](https://doc.rust-lang.org/cargo/reference/features.html)，不代表当前配置已编译验证。

私有机制测试靠近所属 Module；公开合同与消费者测试在 tests；fixture/peer helper 集中复用。此次只确定归属原则，测试去重清单由测试决策处理，避免边搬源码边凭名称删测。

#### 推荐迁移顺序

公共能力/消费者映射 → 值类型和依赖归属 → auth/catalog 合同与后端拆开 → 统一请求执行/消费合同 → provider/API 和内置工厂收敛 → 功能配置隔离 → 外部消费与文档。每步保留行为验证，协议 backend 替换单独审查，不借目录整理顺手更换。

#### 本轮征询（已确认，见 Answer）

是否允许本次针对 0.x Rust 公共路径和 Interface 进行有计划的集中调整，同时保留完整 Pi 能力、同步迁移仓库消费者、提供明确迁移说明；只对存在实际外部兼容需要的旧入口提供过渡，而不永久维持所有旧路径和重复表面？

推荐允许。候选例子：把 PreparedRoute/调用控制集中到 execution，把 Store mutation 协议归到 auth/store，把分散的协议入口归到 api；具体符号有完整迁移表，不能借收缩公共面删能力。若存在必须保持源码兼容的外部项目，需要将其加入消费者清单并限定哪些路径需兼容。此选择与正式发布版本号分别处理，本轮仍不实施代码。

## Answer

2026-11-06，用户以“同意，继续推进”确认本轮结构方案及有计划的 0.x 公共 Rust 接口迁移政策。

- 采用本票“继续讨论：建议采用的结构”中的职责划分：一个 opi-ai crate 加独立 opi-telemetry；Models 为常用门面，Provider、API Adapter、执行控制分别承担服务对象、wire 协议与请求生命周期职责。目录可随规模采用文件或子模块，具体名称和逐符号签名不因本次确认全部冻结。
- 保留完整 Pi 能力及已核实的高级消费者合同，包括 Provider 扩展、prepared call/attempt/retry admission、Store mutation 和必要宿主注入。执行与消费分离；正式 Faux 保留，测试专用私有状态探针收回所属模块。
- 纯值与通用语义不依赖具体 SDK、网络、CLI 或目录资产；auth/catalog 合同与后端分离。保留 Tokio 路线，最小配置支持核心与 Faux；具体 feature/default/发布矩阵在验收方案中冻结。
- 允许集中调整 0.x 公共路径和 Interface，同步迁移工作区消费者并提供逐符号迁移说明；旧入口只在实际外部兼容需要下提供明确过渡。外部消费者是否存在仍须调查，不能用仓库内无引用代替兼容性证据。发布版本号另行确定。
- 重构先建立公共能力与消费者映射，再按本票建议顺序调整责任、执行、适配和配置。backend 替换与测试删除各自需要证据，不随搬目录自动批准。

此结论是后续实施依据；本轮不改生产代码、README、依赖或现有测试，不声称已实现独立接入或通过行为验收。

## 后续实施记录

2026-11-17 的 #158 将内置 provider 装配模块从 crate 根迁移到
`providers::collection`。该模块负责 profile、catalog、auth resolver 与已构造
wire route 的组合，不负责任何 wire 序列化，因此归入 `providers`；旧
`provider_collection` 根入口已删除，没有保留兼容 shim。此实现遵循本票的
“内置 provider 工厂是装配位置”结论，并由 builtins 行为测试、coding-agent
调用方测试和实际归档包外部 consumer 验证。

2026-11-17 的 #159 将 provider/model/request header 合同从 crate 根迁移到
`api::shared::headers`。该模块只负责跨 wire 的 header 语法、保留名保护和
确定性合并，不负责 provider identity、endpoint 或 catalog，因此归入
`api::shared`；`ProviderHeaders` 的根级 re-export 仍是精选 facade，旧
`provider_headers` 模块路径已删除。协议 mapper 继续保持 crate-private，实际
归档包 consumer 直接解析并运行 `api::shared::headers`。

2026-11-17 的 #160 将 provider environment lookup 从 crate 根迁移到
`api::shared::env`。该 helper 只做 scoped-first/ambient fallback 的惰性解析，
不选择 provider、不构造 endpoint、不处理鉴权或 proxy，也不序列化 wire；因此
它属于共享 API adapter contract，而不是 `providers` profile 或 composition。
旧 `provider_env` 根模块已删除，实际归档包 consumer 直接解析并运行
`api::shared::env`。

2026-11-17 的 #161 将增量 UTF-8 `StreamTextDecoder` 从 crate 根迁移到
`api::shared::text_decode`。它只维护跨网络 chunk 的字节尾部和 pinned
replacement 语义，被多个 API wire adapter 复用，不负责 provider profile、
endpoint、鉴权或 transport；因此保留为 `api::shared` 下的 crate-private
shared seam，不扩成公共 consumer API。旧 `text_decode` 根模块已删除，协议
adapter 与归档包 consumer 验证均使用新实现。

2026-11-17 的 #162 将 `join_endpoint` 从 crate 根迁移到
`api::shared::endpoint`。它只对已解析的 endpoint 字符串和 API path 做确定性
斜杠拼接，被多个 API wire adapter 复用，不选择或验证 provider endpoint，不处理
鉴权、catalog 或 transport；因此归入 `api::shared` 下的 crate-private shared
seam，不扩成公共 consumer API。旧 `endpoint` 根模块已删除，调用方与实际归档
包 consumer 均验证 canonical 路径。详见 [#162](../issues/162-migrate-endpoint-to-api-shared.md)。

2026-11-17 的 #163 完成的是剩余 crate 根模块的保留审计。`codex_resources`
虽然起源于 Codex 资源场景，但它是 `Models`、`Provider` 和多个 adapter 共同
使用的公开 resource-generation/lease/cleanup 合同；`time` 被 agent、Models、
stream 和 adapter 共同使用；私有 `execution` 拥有 chat 执行状态。三者均不属于
`api::shared`。其余根模块分别保留为公共领域合同、transport/backend、auth/
catalog、跨 provider 语义或私有 substrate。这一步没有发现仅因根目录位置就
应迁移的模块，但它没有证明跨层职责已经解耦。复核后 `ChatApi`/`DeferredApi`
已分别收口到 `api::contract`，但 API adapter 仍依赖具体 provider profile，
profile 仍保存 adapter-native options，且 `models.rs` 继续拥有多类注册、
auth/catalog 编排、调用执行与 wire-specific 规则。详见
[#164](../issues/164-audit-unresolved-ownership-boundaries.md)；其中 OpenAI Chat
profile 的 native options 已由 [#167](../issues/167-decouple-openai-chat-profile-options.md)
移出，OpenAI Responses profile 也已由 [#168](../issues/168-decouple-openai-responses-profile-options.md)
移出；[#169](../issues/169-decouple-openai-codex-profile-options.md) 又将 Codex
profile 的 native options 与 API 依赖移出，并把其会话级 WebSocket 状态归入
canonical `transport::codex`；[#170](../issues/170-decouple-anthropic-profile-options.md)
随后将 Anthropic 的 native options 收拢到 canonical wire adapter，并把纯
  `ProviderHeaders` 值下沉到 `types::headers`，移出 profile 的 API 依赖；[#171](../issues/171-decouple-azure-openai-profile-options.md)
  又将 Azure 的 per-call options 与 dual-wire dispatch state 收拢到
  `api::azure_openai::AzureOpenAIWireAdapter`，移出 profile 的 API 依赖。其余
  profiles 仍未完成。2026-11-18 的结构复核进一步确认
  `provider.rs` 反向导入 `api::contract`，而 `stream.rs` 与 `execution.rs`
  互相依赖实现类型；这两条边界不能由目录存在或根模块保留审计代替。
  [#172](../issues/172-freeze-layer-graph-and-lower-provider-contract.md)
  把 lower provider contract、stream/execution owner 和结构检查列为后续
  收敛门槛。结构阶段保持开放，必须先完成有证据的
contract、adapter/profile、stream/execution 和 Models 内部拆分，再
并行推进 #143 及其余平台、能力、资源、性能和 fuzz 验收。详见
[#163](../issues/163-audit-root-module-ownership.md)。

2026-11-18 的第一条 G3 切片将 `SimpleOptions`、Deferred call values 和
`DeferredApi` backend contract 的实现移动到 `provider::operation`。`api::contract`
继续提供公开重导出，工作区调用方不需要同步改路径；`provider.rs` 不再导入
`api::*`，请求头保留名校验也改为直接依赖 `types::headers`。该切片只处理
lower contract 方向，剩余 API adapter/profile route context、profile native
state、Models 内拆和 stream/execution owner 仍按 [#172](../issues/172-freeze-layer-graph-and-lower-provider-contract.md)
推进。
2026-11-18 的 #173 将 Mistral native options 收拢到
`api::mistral::MistralConversationsWireAdapter`，移除 profile 的 API-native
state。Mistral fixture、repeated-finish 回归和 builtins check 均通过，剩余
Bedrock、Gemini、Pi Messages、Vertex profile 继续按 #172 推进。

2026-11-18 的 #174 将 Pi Messages native options 收拢到
`api::pi_messages::PiMessagesWireAdapter`，移除 Radius profile 的 API-native
state。Pi Messages Chat 和 terminal 回归均通过，剩余 Bedrock、Gemini、Vertex
profile 继续按 #172 推进。

2026-11-18 的 #175 将 Bedrock native options 收拢到
`api::bedrock::BedrockWireAdapter`，移除 Bedrock profile 的 API-native state。
Bedrock fixture 的 179 项测试和 builtins check 均通过，剩余 Gemini、Vertex
profile 继续按 #172 推进。

2026-11-18 的 #176 将 Gemini native options 收拢到
`api::gemini::GeminiWireAdapter`，并把 body engine 改为显式接收 per-call
options，供 Vertex 复用。Gemini fixture 71 项与 Vertex fixture 73 项均通过。
Vertex 暂时保留自己的 native option 值，下一条 #177 将其迁移到 Vertex
wire adapter 并关闭最后的 G1 profile-state 引用。

2026-11-18 的 #177 将 Vertex native options 收拢到
`api::vertex::VertexWireAdapter`，同时把 project/location override 绑定到
adapter-owned route snapshot。Vertex fixture 的 73 项测试通过，结构报告的
`profile_native_state` 达到 0，G1 收口。后续结构工作转向 API/profile route
context、provider-to-provider edges、stream/execution owner 和 Models 内部
owner 拆分。

2026-11-18 的 #178 把 `ProviderRegistry`、provider resource namespaces、
cleanup owner list 与 registration gate 收拢到 `Models` 私有的
`RegistryOwner`。Models facade 的公开路径保持不变，lib 374 项测试通过，
结构报告新增 `models_owner_modules=1`。这是 G5 的第一条 owner slice，auth、
catalog、operation registration 与 prepared execution 继续拆分。

2026-11-18 的 #179 把 route auth descriptors、resolver/source maps、OAuth
bindings、builtin auth factories、ambient auth context 与 credential store
收拢到 `Models` 私有的 `AuthOwner`。lib 374 项测试与结构检查继续通过，
`models_owner_modules` 达到 2；catalog、operation registration 与 prepared
execution 继续拆分。

2026-11-18 的 #180 把 persisted models store、dynamic catalog sources、
publisher provenance、acknowledged catalog entries 与 publication generation
收拢到 `Models` 私有的 `CatalogOwner`。catalog personalization 5 项回归与
结构检查通过，`models_owner_modules` 达到 3；operation registration 与
prepared execution 继续拆分。

2026-11-18 的 #181 把 chat、image、classifier 与 deferred registration maps
收拢到 `Models` 私有的 `OperationOwner`。现有 `PreparedProviderCall` 被记录为
prepared execution owner，catalog personalization 5 项回归与结构检查继续通过，
`models_owner_modules` 达到 4，`models_prepared_execution_owner=1`。

随后 #200 开始物理模块拆分，将 `RegistryOwner` 放入
`models/registry_owner.rs`。结构报告同时保留 owner 定义总数，并新增独立 owner
文件数和 facade 中的剩余定义数，避免把“已定义”误当作“已物理隔离”。

随后 #201 将 `AuthOwner` 放入 `models/auth_owner.rs`，继续保持 facade 的公开
接口与 auth 生命周期行为不变。物理拆分指标达到 2 个独立 owner 文件，facade
中剩余 2 个 owner 定义。

随后 #202 将 `CatalogOwner` 放入 `models/catalog_owner.rs`，保留 refresh 与
publication generation 的一致性状态。物理拆分指标达到 3 个独立 owner 文件，
facade 中剩余 1 个 owner 定义。

随后 #203 将 `OperationOwner` 放入 `models/operation_owner.rs`，四类 registration
map 完成物理迁移。`models.rs` 中 owner 定义数降至 0，registration record types
仍作为 facade 内部的 crate-private 数据合同保留。

随后 #204 将 `PreparedProviderCall`、`PreparedRoute` 和 attempt stream 放入
`models/prepared_call.rs`。prepared execution 通过 crate-private 构造 seam 进入，
执行字段保持在模块内，Models facade 只保留 route preparation 与 orchestration。

随后 #205 将五个 registration record、typed dispatch adapter、native option
handle 和 catalog-invisible chat route 放入 `models/registration.rs`。结构报告新增
registration record 文件数和 facade 剩余数，`FrozenChatBinding` 保留到 ChatCall
orchestration 切片。

2026-11-18 的 #182 将 Gemini route state 抽取到 lower
`provider::routes::GeminiRouteContext`。API adapter 直接依赖该中性上下文，
`providers::gemini::GeminiProvider` 保留为 canonical 类型别名，因此现有
构造与 fixture 调用不变，API/profile 结构边界开始以正向
`api_route_context_edges` 指标记录。

2026-11-18 的 #183 将 Vertex route state 抽取到
`provider::routes::VertexRouteContext`，并让其复用 neutral
`GeminiRouteContext` body engine。Vertex adapter 不再导入具体 Gemini 或
Vertex profile，`provider_to_provider_edges` 同步减少一条。

2026-11-18 的 #184 将 Bedrock 的 endpoint、client、catalog 与 Smithy
retry route state 抽取到 `provider::routes::BedrockRouteContext`，并把纯
endpoint 校验放进 lower contract。现有 provider helper 通过 re-export 保持
兼容，API adapter 不再导入 Bedrock profile。

2026-11-18 的 #185 将 OpenAI-compatible route state 抽取到
`provider::routes::OpenAiRouteContext`，由 OpenAI Chat adapter 直接消费。
Mistral、OpenRouter 与 Azure 的复用方向继续通过该 context 收口，避免把
一个 concrete profile 当成另一个 profile 的 owner。

2026-11-18 的 #186 将 Mistral 与 OpenRouter 的 OpenAI-compatible factory
改为直接构造 `OpenAiRouteContext`。Azure 仍保留一条 dual-wire profile
复用边，等待 Responses route context 一并收口。

2026-11-18 的 #187 将 native Mistral Conversations route state 抽取到
`provider::routes::MistralRouteContext`。其 API adapter 与 OpenAI-compatible
Mistral route 现在共享 lower route contract，但保留各自 wire owner。

2026-11-18 的 #188 将 Pi Messages Radius route state 抽取到
`provider::routes::PiMessagesRouteContext`，保留默认 catalog 与 client
配置在 profile 构造路径，wire/native options 继续归 API adapter。

2026-11-18 的 #189 将 Anthropic Messages route state 与 federation cache
抽取到 `provider::routes::AnthropicRouteContext`，OAuth/Copilot route policy
继续属于 route context，native wire options 保持在 adapter 内。

2026-11-18 的 #190 将 OpenAI Responses route state 与 affinity policy
抽取到 `provider::routes::OpenAiResponsesRouteContext`，为 Azure dual-wire
route 复用提供了中性上下文。

2026-11-18 的 #191 将 Azure dual-wire endpoint、deployment 与 client state
抽取到 `provider::routes::AzureRouteContext`，同时以 neutral Chat/Responses
contexts 组装两个 wire，profile-to-profile edge 收口为零。

2026-11-18 的 #192 将 Codex endpoint、catalog、HTTP client 与 WebSocket
resource state 抽取到 `provider::routes::OpenAiCodexRouteContext`，主 wire
与 native subadapter 共享该 context，API/profile inversion 收口。
