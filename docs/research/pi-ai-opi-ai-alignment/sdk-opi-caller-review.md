# opi-ai 独立 Rust 调用方审计

> 非规范性研究，2026-10-04。只审计本地公开源码、crate 文档、测试、manifest 与当前候选 spec；不授权实施，不改变已确认目标。本轮未编译外部消费项目、未运行运行时测试、未联网或读取真实凭据。下文测试定位是静态证据，验收项是后续验证设计。

## 结论与阅读范围

外部 Rust 项目不依赖任何其他 opi crate，即可用现有 `opi-ai` 完成模型选择、注入鉴权、stream、流汇总 complete，以及自行维护消息和工具结果的多轮循环。现有 `ProviderCollection → PreparedProviderCall → EventStream` 已有真实状态机价值，应作为收敛基础；不需要为了独立 SDK 另造一个产品事务、会话或 Agent 外壳。

“仅依赖 opi-ai”需要准确表述为“只依赖这一个 opi crate”。README 的可读示例还显式使用 Tokio、tokio-util、futures-util、secrecy；具体 transport 调用 `tokio::spawn`。这不能被宣传为只在 Cargo.toml 添加一个依赖、仅凭标准库即可运行 HTTP 的证明，也不能反过来证明 SDK 依赖 opi-agent。部分外部类型可以靠类型推断构造，本轮没有编译穷尽最小依赖写法，不断言额外依赖的严格最小集合。

当前候选 spec 的 owner 表、sessionless 段和 AIR-042 已表达 SDK 独立性，但第 60 行持久化 MUST 的范围，以及 AIR-024 的“all operations”和工具授权叙述仍可能被实施 Agent 误读。建议明确划分 **SDK 请求/投影验证、调用方工具执行、产品持久化** 三种职责。这里需要澄清适用条件，不需要撤销已确认的持久化确认或最终授权保证。

基线：Git HEAD `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`；本轮 `git status --short -- crates/opi-ai` 无输出，目标 spec 为未跟踪研究文档。crate 版本 `0.8.2`、edition `2024`、rust-version `1.97` 来自 `Cargo.toml:13–16`。`crates/opi-ai/Cargo.toml:13–33` 无其他 opi crate 依赖。`crates/opi-ai/examples/` 不存在；README 有一个最小示例，tests 下存在公开 seam 的独立集成测试。

本轮实际读取的主要 Module：`lib`、`provider`、`provider_collection`、`registry`、`auth`、`message`、`model_info`、`stream`、`openai_chat`、`http`、旧 `config/model`；检索 concrete adapter 构造入口，并阅读 collection、provider trait、custom registration、tool result 和 lifecycle 测试的相关部分。未穷尽每个 Provider 的所有序列化器，不重新判定本轮既有协议研究。

## 外部调用路径与证据地图

| 任务 | 当前公开组合 | 能证明什么及限制 |
|---|---|---|
| 选模型 | concrete `model_catalog()` 或 `Provider::models()`；`ProviderRegistry::all_models/resolve`；collection 的 `registry/resolve/capabilities` | 可不鉴权读取有限模型元数据；不是已验证 entitlement。`openai_chat.rs:809–846`、`registry.rs:210–262,291`、`provider_collection.rs:553–582`。 |
| 配鉴权 | `StaticAuthResolver::new`，或调用方实现 object-safe `AuthResolver` 返回 `ResolvedAuth`；通过 `register_route` 组装 | 静态 API/Bearer 与自定义来源不需要产品 crate。SDK 没有因此自动提供产品登录 UI/OS store。`auth.rs:135–156,288–330`、`provider_collection.rs:372–397`。 |
| Stream | 构造 `Request`，`prepare_call(spec, request).await`，`start_attempt()`，消费 `EventStream` | preparation 冻结鉴权，attempt 拥有独立活动租约；需处理两层错误。`provider.rs:78–133`、`provider_collection.rs:416–550,853–899`。 |
| Complete | 对上述 attempt 调用公开 `provider_collection::drain_to_completion` | 已有能力，不应误报缺少 complete；没有独立 `Provider::complete` 也不是缺陷。返回 `Result<CompletedRequest, ProviderError>`，成功 drain 可携带 `CompletedRequest::Error`。`provider_collection.rs:218–232,712–735`。 |
| 工具上下文循环 | 请求 `ToolDef`；读最终 `AssistantMessage/ToolCall`；调用方执行后追加 `Message::Assistant` 与 `Message::ToolResult`，准备下一调用 | 消息表示足够供宿主手写循环；SDK 自己不执行工具、也不要求 ToolRegistry。当前 `ToolCall.arguments` 是原始字符串，不能声称已验证。`message.rs:9–52,109–150`、`provider.rs:296–351`。 |
| 模型覆盖 | `ProviderRegistry::register_model` → `ProviderCollection::from_registry` → `register_route` | 已有实际可组装顺序；不是“覆盖必须产品 factory”。但 collection 建成后只暴露只读 registry，且 metadata 与 dispatch 一致性仍待 AIR-001 修正。`registry.rs:165–207`、`provider_collection.rs:327–397,554`。 |
| 兼容 profile | `OpenAiChatProvider::new_for_profile`；或 route + concrete model catalog + `with_model_overrides` | SDK 有可复用 concrete 构造，不需读产品配置文件。多个 compat 状态来源需要按 AIR-022 统一，不能新增一套 SDK 专用 profile 状态。`openai_chat.rs:774–807,910–1007`。 |
| 多实例 | 每个 collection 自有 registry/resolver/source/compat map；每个 prepared call 自有 request/auth/租约，Provider/client 仅显式 Arc 共享 | 静态所有权支持隔离；不等于已执行同名 Provider/不同账户/取消交叉隔离测试。`provider_collection.rs:297–324,543–550,764–774`、`openai_chat.rs:794–806`。 |
| 取消/错误/结果 | caller-owned `CancellationToken`、`Request.timeout`、`CollectionError`、`ProviderError`、terminal message/Usage | pre-auth 和 attempt 错误不同；Error 事件不是 Rust Err；Usage::unknown 不是计费零。当前 raw stream 终态漏洞不能靠调用方习惯性停止代替修正。`provider_collection.rs:514–521,718–735,793–823,871–899`、`stream.rs:15–25,46–79,325–395`。 |

上述路径均为静态可达性判断。本轮没有将仓库 dev-dependencies 环境下的测试编译等同于独立发布 crate 的消费证明。

## 发现与处置建议

### SDK-01 — SDK 独立性成立，但还缺完整、可重现的外部消费者验收

- **类型/优先级：** 已有合理设计 + 验收缺口；中。**置信度：高（公开组合），中（外部最小依赖和打包结果）**。
- **事实与影响：** `lib.rs:7–61` 暴露必要 modules；manifest 无 opi-agent/opi-coding-agent；README `:316–387` 仅调用 collection 和 concrete adapter。`tests/provider_collection.rs:287–336` 的测试静态展示 stream 和 complete；`tests/provider_collection.rs:404–501` 覆盖终止 Error、transport Err 和无终止流。现有示例只输出 Debug 事件，未展示终止失败判断或工具循环。
- **已查反证：** complete 不是缺失能力；不能因为根模块未 re-export `drain_to_completion` 就判定不可用。tests 是外部于 lib 的 integration targets，但仍继承 workspace/dev-dependencies；没有现存 examples 目录不等于外部消费者不能使用。
- **建议：保留 + 补齐使用证明。** 未来离线验收用隔离消费 fixture，只依赖 opi-ai 及明确列出的通用 Rust 依赖，不依赖其他 opi crate。经公开入口运行静态 resolver、伪 transport、流/complete、手写工具往返、两个独立实例和失败取消。README 提供同一条小路径的递进示例，避免引导所有用户先学产品 binding/session。
- **机械验收：** 对依赖图断言无其他 opi crate；无环境凭据参与；捕获两次模型请求，第二次含第一响应与关联 tool result；断言 complete 的 terminal Error 仍保留 message/usage。此验收尚未执行。

### SDK-02 — 候选 spec 应限定 durable/Agent gate 的适用范围

- **类型/优先级：** 候选契约范围歧义；高，建议合稿前澄清。**置信度：高**。
- **证据：** `spec-draft.md:48–53` 已将事务交给 Session/product owner；`:60` 却在通用职责正文使用“same transaction owner MUST durably acknowledge ... before first transport dispatch”，没有就地限定持久化调用；`:62` 用“Sessionless ... same validation”排除伪会话，但没有枚举排除的 gate。AIR-017 `:96`、AIR-019 `:98`、AIR-042 `:131` 的 durable caller 表述则提供正确反证。AIR-024 `:108` 的 owner 为 Agent/errors/usage、触发条件“all operations”，混合工具参数验证、执行授权和所有操作的用量。
- **成立条件/可观测影响：** 新 Agent 把“一切 SDK dispatch 需 durable receipt/可信工具 registry/authorizer”落实到 `prepare_call`、image/classifier 或自定义 host，外部单 crate 调用无法使用，或被迫伪造 session/trust receipt。当前运行时代码没有该依赖，本轮也没有把歧义当作已交付缺陷。
- **保留现状理由及代价：** 正文上下文和 owner 能支持正确解释，已有 sessionless 反例；但靠跨节推断才能决定每个 SDK call 要不要持久化，不满足无猜测实施目标。
- **建议：修正适用域。** 在通用状态段明确：“持久化会话模式才要求 AIR-017 receipt；独立 SDK 在 owned memory 完成 request/model/auth/payload freeze，不创建事务 owner、session、trusted execution registry、authorizer 或 Promotion。”AIR-024 把“模型提出工具调用时的纯规范化/验证”与“宿主实际执行时的最终授权”分开；image/classifier 只应用其 typed input/output、usage/error 规则。AIR-039 的默认恢复为产品/Agent 策略，不自动安装到 SDK complete。为 raw tool proposals 保留返回路径，不能因没有 executor 就拒绝模型响应。
- **机械验收：** 一个没有持久化/Agent dependency 的 SDK fixture 能 stream/complete/submit/fetch/image/classify（按各自目标范围）；注入一个永不被调用的持久化探针，独立模式计数零；产品持久化模式确认失败则 transport 计数零，已接受输入仍保留。这样同时证明两个保证。

### SDK-03 — 保留 prepared-call 深度，收敛构造和汇总体验

- **类型/优先级：** Interface 深度与误用机会；中。**置信度：高**。
- **当前职责/所有者：** collection 管模型查找/鉴权；prepared call 管冻结状态/attempt 租约；drain 管 terminal 汇总，责任有价值。它们不是仅传参的冗余 Module。`Request` 为 多个公开字段且无 Default/builder（`provider.rs:102–156`）；调用还要重复给 `prepare_call` 路由字符串，`:425–431` 不匹配即报错。根 `Error` 指向旧 config 外壳，实际调用错误位于 `CollectionError/ProviderError`（`lib.rs:44,53–57`）。
- **影响与反证：** 外部调用方需理解路由两份输入、取消 token、两种终态和多处 import。重复路由检查是有意 fail-closed，当前不会偷偷采用另一值；不能把字段多或 import 多直接等同设计错误。原生 Future/Stream 和 typed enums 应保留。
- **建议：保留 + 最小收敛。** 继续只有一个 prepare/attempt 执行状态机；便利 stream/complete 只能委托同一条路径。用初始化构造/选项默认值降低短调用负担，并让模型身份只有一个权威输入：可从 request 的已选 model 准备，不要求外部重复同一字符串。确需 route token 的方案应绑定 collection/revision 且避免第二 authority；不为此预建新泛型框架。根导出指向真实请求/错误/结果类型；AIR-021 旧 Error/Model 外壳退出时同步文档，不再加新通用 Error 外壳。
- **验收：** 公开示例一条输入选择模型；stream 和 convenience complete 捕获同一 effective binding/auth/payload；失配如果仍允许表达，必须在鉴权前失败。complete 不额外实现第二次 adapter dispatch。具体函数签名本报告没有冻结，也不伪称这些便利入口已经存在。

### SDK-04 — “注册成功”与“可调用”是合理状态区分，但当前命名/组合容易误用

- **类型/优先级：** 有意差异 + 易误用 Interface；中。**置信度：高**。
- **事实：** `register` 是 lookup-only，且替换同 ID 时清除已有 resolver；`register_route` 才可执行（`provider_collection.rs:346–397`）。`from_registry` 不继承可调用鉴权（`:327–343`）。`auth_status` 也不能代替 `validate_dispatchable_route`。测试 `provider_collection.rs:1934–1976` 明确证明 metadata replacement 后禁止沿用旧 resolver。
- **影响条件：** SDK 用户照名称先 register 一个 Provider，再假定 descriptor 是鉴权解析器；或用 metadata update 意外把可调用路线降为 lookup-only。正确结果是 typed error，不是凭据混用。
- **反证/保留理由：** 列出尚未配置凭据的模型、渐进配置和显示 auth status 都是真实用途；删除所有 lookup-only 状态反而令模型选择依赖网络/凭据。
- **建议：保留状态，收敛表达。** 查询/listing 与 executable assembly 可以共用 collection，但操作名称/类型清楚表达“仅目录注册”或“可执行注册”。模型元数据更新不隐式等价于清除账户鉴权；显式 route replacement 则原子替换。可收敛现有 register/from_registry 编排，不另设长期第二 collection。
- **验收：** lookup-only 能枚举，prepare 在 resolver 前明确失败；转为 executable 后只解析选定来源；再显式降为 lookup-only 不复用旧 resolver。若只改模型元数据，不误丢同一条 route 的来源配置。

### SDK-05 — 模型覆盖已有可用组合，但没有完成“一个有效记录控制实际执行”

- **类型/优先级：** 部分能力 + 已知正确性风险；高。**置信度：高**。
- **事实：** `register_model` 支持未来 Provider 的覆盖，见 `registry.rs:165–207`；测试 `provider_collection.rs:1556–1604` 先注册覆盖，再 from_registry，再 register_route，验证输出预算采用覆盖。不能报告“SDK 无模型覆盖”。但 collection 仅暴露 `&ProviderRegistry`（`:554`），不能通过同一公共 facade 在运行中注册新 override。`prepare_call` 保存 route wire 和 request，但 prepared call 不保存完整 ModelInfo（`:538–550,764–774`）。Chat `resolve_compat` 仍读取 adapter 自己的 models/profile overrides（`openai_chat.rs:986–1007`）；mapped adapter 也有独立目录/路由选择。
- **影响条件：** 覆盖改变 wire/compat/模型身份时，仅在 registry 中列出或改变预算，不证明 concrete adapter 按同一记录执行。AIR-001 已选定修正方向。当前 refresh 能被显式调用，但它的 `&mut self` 和全体成功后发布语义（`:634–701`）不自动提供后台一致性。
- **反证：** construction-time `Provider::replace_model_catalog` 为 mapped materialization 提供真实 hook（`provider.rs:46–62`）；某些 factory 能正确预先装配。不能因此推断所有外部 assembly 都已统一。
- **建议：修正/合并到唯一 owner。** 明确公开可写 catalog/override 方法由同一 collection 的 revision owner 验证并发布；旧 prepared call 保留旧完整绑定，新调用使用新记录；adapter 消费绑定，不再独立查找。底层 registry 是否保持公开可作为兼容处置讨论，不能为了 wrapper 简洁删除 lookup-only 能力。
- **验收：** 外部 fixture 通过公开 facade 添加自定义模型、覆盖 compat/wire 和替换 revision；preflight 与捕获的实际请求一致；旧 call 不漂移；坏候选不发布。单看 `all_models` 或 mock 的预算字段不足。

### SDK-06 — 工具循环需 SDK 数据/验证支持，但不应被迫采用 opi-agent 执行器

- **类型/优先级：** 可复用能力和执行职责需分开；高（目标澄清），中（示例）。**置信度：高**。
- **事实/状态：** `ToolDef.input_schema` 为 JSON、`ToolCall.arguments` 为 String、ToolResult 带 ID/name/content/error，消息都可序列化（`message.rs:9–52,139–150`）。当前 `validate_request_for_model` 检查图片、model compat、thinking（`provider.rs:296–351`），不验证未来生成的工具参数。`tool_result_wire.rs:53–87,286–338` 证明 serializer 如何保留错误标记，不证明调用方执行已受可信 schema/权限约束。
- **影响：** 当前外部宿主可以手写工具往返；它须自己解析/验证/执行。候选 transcript/normalization 若只实现于 Agent 内部，SDK 用户就得复制 pi 风格规范化规则；若把 Agent registry/政策系统塞到 SDK，则失去可复用的 SDK Module。
- **建议：保留手写循环，提取有实际语义的纯能力。** opi-ai 拥有 transcript 的声明/内容/投影与无 I/O 的 plain-JSON normalize/validate helper；输出原始/解析/有效值及类型化失败，不执行工具、不授予权限、不自行假造 NotExecuted。实际宿主或 opi-agent 决定执行，并负责执行前最终授权和受信任 hooks。收到 ToolCallEnd 不代表整个响应成功，示例等待最终成功/ToolUse 后才按调用方策略执行；Length/Error/Aborted 遵守候选非执行与重放规则。
- **验收：** 无 Agent 依赖的两轮离线 fixture 得到工具提议、调用公开纯 helper、显式宿主处理、追加结果再调用；错误/截断提议不会触发 fixture 执行器；schema 转换不修改原始声明；未装执行器仍能获得工具提议，不因缺 registry 拒绝完整模型响应。

### SDK-07 — 多实例是现有 Rust 所有权优势，需要增加调用方层面的隔离证明

- **类型/优先级：** 合理设计保留 + 验收缺口；中。**置信度：高（所有权），中（全路径隔离）**。
- **事实：** collection 各自拥有地图；concrete Provider 各自保存 endpoint/models/compat/client；prepared call 拥有 Arc Provider 和私有 request/auth（前述证据）。没有在本轮查到全局 model/provider singleton；HTTP 的静态 regex 缓存不属于账户状态。显式共享一个自定义 resolver/client 的内部状态是调用方的选择，不能承诺它仍彼此独立。
- **已查反证：** 已有同一 collection 多 Provider、同一 prepared call 多 attempt 和取消测试，但检索 tests 未找到以两个 collection、同 Provider ID、不同账户/endpoint/model override 组成的隔离场景；“没找到”不证明绝对不存在其他覆盖。相同 Provider ID 在一个 collection 内有意 replacement，不是多账户容器。
- **建议：保留实例所有权。** 不引入进程全局默认目录/凭据/WS cache；多账户独立 runtime 实例可使用相同 Provider/model ID，只有显式共享依赖才共享。目标 Codex cache 必须继续按 runtime/session/account/endpoint 隔离，不要求 sessionless 用户创建产品 session。
- **验收：** 两个独立 collection 同时使用同 ID，不同静态凭据、endpoint、override；取消/替换/刷新 A 不影响 B，旧 prepared A 保持原绑定。只验证 Arc 类型或 `Send + Sync` 不足。

### SDK-08 — 返回语义要在最短路径中正确呈现，不增加隐式重试或工具执行

- **类型/优先级：** 已有正确结果建模 + 流修正/文档缺口；高（流），中（教学）。**置信度：高**。
- **事实：** complete helper 的 Rust Ok 可以是 terminal Error，terminal Done 的 reason 又区分 Stop/Length/ToolUse（`provider_collection.rs:222–232,718–735`、`stream.rs:15–25`）。`ProviderError` 和 `CollectionError` 分层反映准备/调用失败，不宜合并成丢失 message/usage 的通用字符串。`AttemptStream::poll_next` 当前 terminal 时释放 active，但继续 poll 内部，未 fuse（`:793–816`）；drain 遇到 terminal 则立即返回，因而两条消费路径目前不等价。AIR-003–006 已要求修正。
- **影响条件：** README 的 `event?` 仅传播 Rust Err，打印 `AssistantStreamEvent::Error` 后仍可返回 main Ok。外部循环若以“drain 返回 Ok”作为成功，会把 Provider terminal Error 当成功；工具循环若在工具增量结束即执行，可能执行后来整体失败的提议。
- **建议：保留 result+terminal distinction，统一终态。** Stream 是主行为，complete 汇总相同终态并保留 partial message/known usage。明确失败不会自动创建新 logical call，不默认继承产品 AIR-039 恢复策略。调用方若手动 start_attempt 重试，必须了解 frozen auth/cancel 和一个活动 attempt 的契约。
- **验收：** 相同 fixture 通过 raw stream 和 complete 得到一致 terminal/reason/message/usage；提前 EOF 一次 typed error 后永久 EOF；取消与缓冲 terminal 的同步竞态按 AIR-004；failed completion 不触发宿主工具执行或自动下一 call。

## Rust-native 小而深的 Interface 候选

这是供下一轮技术选择的 Interface 轮廓，不是已存在 API 签名，也不授权新增 trait/crate。

1. **一个拥有状态的 collection。** 接收已选 concrete adapter、可注入 resolver、有效 catalog/compat 配置；负责 lookup-only 与 executable 状态、修订和 preparation。公共查询无需凭据。注册/修改都交给同一 owner，而不是再包一层同时可变的 facade。
2. **一个 owned request/transcript 和 options。** 便利初始 system/tools 构造立刻规范化；模型身份只有一个权威来源。取消、timeout、sampling 等有明确默认值，复杂选项按需暴露；它不是 product session。
3. **一个 prepared logical call。** 冻结 model/request/auth/payload，公开无秘密的 effective facts/projection；原生 Stream 表示 attempt，complete 是同一流的汇总。普通调用方不用先创建 run、trusted execution revision 或持久化 receipt；产品自己在 prepare 与 dispatch 之间落实其 ACK gate。
4. **公开数据和纯 helper，不内嵌 Agent executor。** Transcript、工具参数规范化/验证、usage、错误均可由任何 host 使用。工具实现、用户权限、持久化/恢复策略属于调用方；SDK 不因调用方没有 opi-agent 就不返回模型工具提议。
5. **以现有 object-safe Provider/AuthResolver 作为实际变化 seam。** 内部继续允许具体 adapters 和 mock；不引入为未来所有 operation/plugin 都服务的通用 trait。类型化 image/classifier/deferred 可共享 model/auth/预算/错误所有权，不能为了共享而抹平结果和状态区别。

推荐先保留这些有状态职责，再在同一次候选破坏性迁移中收敛便利入口。是否增加一个 root-level prelude、给 helper 改名或提供 builder 不是本轮已确认目标；这些小选择不能替代上述行为验收。Rust async runtime 和可选通用依赖也不应被包装成新的不透明执行器，只为追求表面“单依赖”。

## 对候选 spec 的最小补充建议

| 建议 | 涉及契约 | 完成条件 |
|---|---|---|
| 说明 SDK、外部执行宿主、Agent、持久化产品四种使用角色；在持久化 MUST 就地限定 durable callers | 职责段、AIR-017/019/042 | 无 session/transaction/authorizer 的 SDK fixture 可准备和调用；持久化模式仍有 dispatch 前 ACK |
| 明确 SDK pure normalization 与宿主执行授权的区别 | AIR-024/040/041、责任表 | 外部循环复用纯规则，无隐式执行；Agent 仍授权实际最终值 |
| 把独立消费组合纳入验收，而不只测产品接线和 adapter 内部函数 | AIR-001/003/007/020/021/022/032；V-01/03/05/06/20/26 | 外部依赖图、完整工具往返、双实例隔离、完整错误模式、实际 override wire 均有 fixture |
| 明确便利 stream/complete 不带默认高层恢复和产品 startup 网络 | AIR-007/026/034/039 | 无显式选择时只做该调用；只构造 SDK 不访问目录，重试所有者可追踪 |
| 在旧壳淘汰同时修正公开文档与 root 错误导航 | AIR-021 | 当前文档指向可执行类型；不把 `Config/Error/Model` 构造证明当完整 SDK 能力 |

无需因为这些澄清退回旧 opi-spec 的产品归属结论。候选已选 AWS SDK 来源、多操作类型、全 transcript 等能力仍应独立可用。也不应为了展示“可只用 opi-ai”把会话迁移、产品权限或工具实现搬进 opi-ai。

## 本轮检查与证据限度

- 实际执行：只读 `Get-Content`、`rg`、文件/目录盘点，以及 `git rev-parse HEAD`、限定路径 `git status --short`。没有 credentials、live、网络、安装、Cargo build/test 或 probe 执行。
- 静态测试证据包括 collection 公共 preparation/complete/error/cancel/override、custom Provider 注册、tool-result serializer；这里只说明测试断言和使用形状，不声称本轮通过。
- 未证明：独立发布包消费者的实际编译、最小 Cargo 依赖集合、全部 adapter 多实例隔离、所有账户来源、完整工具循环 fixture。应在未来实施的公共 Interface 验收中落实，不让研究者据名称宣布已经交付。
- 测试影响：本轮 **none**。建议未来 **add** 独立 SDK consumer/双实例/工具往返组合测试，**update** public examples/终态与旧入口迁移测试，**retain** 现有 auth/error/usage/cancel 行为测试；本报告不单独授权删除测试。
- 本文为新增中文研究笔记，没有既有英文 counterpart；英中同步目标 spec 由主 Agent 统一处理，本轮未修改共享 spec。

## 合稿更新：Q32 的明确替代

本笔记SDK-03/08中的“保留”指prepared状态机、完整partial/usage/typed cause，不是永久保留旧ProviderCollection、CompletedRequest或event Error名称。Q32已确认统一Rust Result；以spec AIR-045–050、D-30/31及V-39–42为目标。全部旧consumer迁移及历史reader退出界限由spec规定。现产品三OAuth公开协议仍需核pi1.0，不能只搬结构即称等价。
