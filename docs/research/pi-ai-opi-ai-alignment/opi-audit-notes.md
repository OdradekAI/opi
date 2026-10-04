# opi-ai 独立设计审计证据笔记

> **目标解释更新：** 本页基线事实与一手定位保留；首轮目标建议须与 [决策重审](decision-reassessment.md)及 decisions 的第二轮确认一起阅读。D-18以 pi 的 Rust 实现为方向，不能用旧准入或当前 opi 未接线单独否决已落地行为。


> 非规范性研究笔记；2026-10-03；仅记录源码事实、审计判断与候选处置。
> 本文不授予产品实施准入，不修改现行规范。所有候选处置均未获人类确认。

本子研究按 `research` 与 `codebase-design` 的一手证据、真实消费者、状态所有权和 deletion test 方法执行；实际读取两份技能。范围是 `crates/opi-ai` 及解释其调用关系必需的 `opi-agent`、`opi-coding-agent` 路径。主研究负责 pi 对照、版本基线与流生命周期的交叉审计。本笔记未运行测试；下文的“测试证据”均指已阅读的测试设计，不能据此声称测试通过。

本文件是新建中文研究材料，没有既有英文 counterpart；不修改独立 pi 报告或权威文档。最终候选 spec 的双语同步由主研究负责。

## 证据与范围

权威已读取：`docs/opi-spec.md` §3、INV-001/INV-002、§8，`docs/CONTEXT.md`。本次相关约束：

- PRIN-001/002：Agent Core 需有删除测试价值；新公开 seam 应有内在状态机价值或两个真实 adapter/consumer 与共享 conformance。
- PRIN-003：mechanism 在 policy 之下；Reference Product policy 不流入 Agent Core。
- INV-001：模型选择必须解析并保留对应 provider/wire，不能仅因启动 provider 不同而拒绝跨 provider。
- INV-002：供应商 wire 在中立 request/stream/usage/capability interface 之后。
- §8.1：中立性、深度、真实 seam、机械验证与最小 authority；GATE-002 明确 unstable 标记或 pi parity 不能豁免。
- §8.2：跨层移动需要 ADR；本次只提出候选，不建立未确认 ADR。

Cargo 源事实：root workspace 声明 `0.8.2`、edition 2024、MSRV 1.97；`opi-ai/Cargo.toml` 无内部 crate 依赖，HTTP 使用 workspace `reqwest`，root 关闭 reqwest default features 并启用 rustls/json/stream。未核验全部锁定第三方实现，不能从清单推断任何 SDK 性能优势。

主要文件导航：

| 证据组 | 实际阅读或定位范围 |
|---|---|
| 中立入口 | `lib.rs`、`provider.rs`、`provider_collection.rs`、`registry.rs`、`api_mapped.rs` |
| 元数据与消息 | `model_info.rs`、`message.rs`、`stream.rs` usage 部分、`config.rs`、`model.rs` |
| Wire 与重放 | Anthropic serializer/thinking、OpenAI Chat constructor/compat/serializer/prepared dispatch、Responses shared serializer、Gemini serializer、Bedrock serializer/credentials |
| 实际产品装配 | `provider_factory.rs`、`harness.rs` 模型选择/collection 装配、`model_listing.rs`、`picker.rs` |
| 鉴权与刷新 | `auth.rs`、`credential.rs`、product `credential_store.rs` 的 API-key/OAuth resolver、product `oauth.rs` registry |
| Agent 消费 | `agent.rs`、`loop_types.rs`、`agent_loop.rs` 的 prepared call 与工具参数校验、product `compaction.rs` |
| 离线测试设计 | provider_collection、api_mapped_provider、model_wire_metadata、per_request_auth、anthropic_thinking_roundtrip、image_provider_serialization、output_content_image、provider_factory、extension_mapped_catalog、oauth_auth、session_runtime 的相关用例 |

除非特别注明，“无生产消费者”限定为对本仓库 Rust 源码及相关文档的搜索结果，不代表未知外部使用者不存在。没有进行全量外部依赖者调查。

## 当前职责和默认产品路径

```text
Reference Product configuration / OS credential store / OAuth flows
          │ factory assembles Provider + lazy AuthResolver
          ▼
ProviderCollection
  ├─ ProviderRegistry: provider:model + overrides + dynamic catalog
  ├─ AuthResolver: resolve once per logical call
  └─ PreparedProviderCall: request + auth + provider Arc + route facts
          │ start_attempt()
          ▼
ApiMappedProvider (where one provider has multiple wires)
          │ own model catalog selects WireApi
          ▼
concrete Provider::stream_prepared → HTTP/SSE mapper → neutral events
          ▲
opi-agent Agent / loop and opi-coding-agent compaction
```

| Module / interface | 状态 owner | 实际消费者 | 当前接线结论 |
|---|---|---|---|
| ProviderRegistry | `Vec<Arc<dyn Provider>>`、override map、dynamic catalog map | collection；factory；picker/model listing；extension registry tests | 生产使用；不是全局 singleton |
| ProviderCollection | registry、lookup-only auth/probe metadata、dispatch resolver/source maps | Agent、harness、compaction；CLI listing 独立 lookup-only collection | 生产主路径已使用 |
| PreparedProviderCall | frozen request、auth、provider Arc、route facts、attempt active/credential-terminal flags | Agent logical call；compaction；SDK tests | 生产使用；不是仅测试 abstraction |
| ApiMappedProvider | public provider ID、catalog、`WireApi -> Box<dyn Provider>` | GitHub Copilot；自定义多 wire 配置；兼容 profiles 的产品 factory | 有多个真实消费者；不可按“薄包装”直接删除 |
| Provider::refresh_models / collection.refresh | provider 返回快照；registry 安装目录 | collection 离线 mock tests；mapped provider 返回 None | 有库 substrate；未找到默认产品触发路径 |
| AuthDescriptor / AuthStatus / probe | collection 的非实时 listing metadata | factory 构建 listing，doctor 的 credential metadata；并非 live dispatch 的 auth authority | 生产 read-side 使用，和 resolver 不是相同保证 |
| OAuthProvider / CredentialStore | opi-ai 定义对象安全契约；product 实现流程/OS IO | Anthropic、Copilot、Codex；product keychain 与测试 fake | seam 已有真实 variation，应保留中立契约 |

精确导航：`registry.rs:120-128`；`provider_collection.rs:297-312,416-550,634-701,764-775`；`api_mapped.rs:26-29,174-200`；`provider_factory.rs:135-195,1215-1286,1442-1512,2300-2425`；`harness.rs:1731-1761`；`opi-agent/src/agent.rs:543-574`；`opi-coding-agent/src/compaction.rs:178-193`。

## O-001：保留 collection、registry、mapped-provider 的不同职责

**分类 / 置信度：** 合理且必要的不同设计取舍；保留职责，高。不能因有三层或 pi 接口名称不同就要求合并。

**当前职责、消费者、owner：** 见上图。registry 隐藏目录优先级；collection 隐藏鉴权一次解析、路由校验和 logical-call 生命周期；mapped provider 隐藏多 wire 的完整性验证和分派。`ApiMappedProvider::try_new` 验证 route/catalog 完整性，真实产品 Copilot 同一 ID 跨 Anthropic Messages、Chat、Responses，custom profile 也使用它。

**证据 / 可观测价值：** `registry.rs:218-258` 的 override/dynamic/builtin 优先级；`provider_collection.rs:416-550` 的准备逻辑；`api_mapped.rs:67-146` 拒绝重复/缺 route/隐藏 provider ID/catalog 不一致；product `provider_factory.rs:1512` 装配 Copilot。删除 collection 会把 auth 与冻结责任返回 Agent、compaction 等多个消费者，满足 deletion test。

**反证检查：** `from_registry` 与 `register` 确有 lookup-only 路径，但 product listing 需要不触发 credential IO；并非所有“非 dispatch”对象都是遗留废物。`Provider::stream_prepared` 虽公开，仍是 adapters 和 fixture tests 必需 seam；不能简单把它删为只有 collection 一个类型。

**保留理由与代价：** 保留 Rust 显式所有权与 trait objects；代价是须解释 metadata-only/dispatchable 区别，且多份 catalog 需解决 O-002。候选替代是把数据存储内聚到一个实现内，而不是把全部语义摊给调用方。

**建议与验证设计：** 保留三种职责；是否收窄公共可变接口与删除历史 facade 需分项决定。共享 conformance 应跨两个真实 wire 和 mapped provider 覆盖路由、preflight、auth、终止。保留 `api_mapped_provider` 的图验证与 `phase17_provider_runtime` 的产品路由测试。

## O-002：有效模型元数据有两个 authority，非启动 route override 可与实际 wire 脱节

**分类 / 置信度：** 静态可证明的数据流不一致风险；修正，高（运行时触发尚未隔离重现）。

**当前职责和 owner：** collection 从 registry effective model 得到 wire、capabilities 和默认输出上限；concrete/mapped provider 再从自身 constructor catalog 选择 wire/compat/thinking/base URL。prepared call 保存 route 的字符串/enum，却未保存并传递 effective `ModelInfo`。

**精确证据：**

- `provider_collection.rs:421-449,450-499,538-550`：使用 registry model 做 preflight、上限，构造 `PreparedRoute.wire_api`；交给 provider 的是 `Request`，无 model metadata 字段。
- `api_mapped.rs:174-200`：再次查 `self.models`，根据其 wire 分派。
- `openai_chat.rs:988-1007`：兼容选项来自 provider 自有 catalog 或 constructor config；`:1076-1080` thinking map 同样来自自身 models；`:1468-1470` endpoint/path 来自自身 model 与 compat。
- `provider_factory.rs:156-182`：只把 active provider overrides 预先 materialize；非 active overrides 进入 registry。
- `harness.rs:1504-1519,1746-1761`：只 materialize 启动 active provider，然后把 extra routes 加入 collection。
- `harness.rs:2153-2166`：模型切换做 resolution/dispatchability/thinking 校验，未 materialize 被选中的 extra route。

**成立条件和影响：** 用户安装的 extension 给一个已注册、可 dispatch 的非启动 provider 添加或修改模型，然后跨 provider 选择它。registry 可接受新增 ID，但 mapped provider 查自身 catalog 后返回 UnknownModel；对已有 ID 改 wire 时，prepared route 可记载新 wire，而 dispatch 仍走旧 wire；改 compat/endpoint/thinking map 也可能 preflight 与网络行为不同。仅改上限的 override 恰好由 request 把结果传递下去，不能据此证明所有字段一致。

动态 catalog 也只安装 registry，存在同类接口风险；但是所有当前内置 provider 静态、mapped refresh 明确返回 None，未找到生产 refresh trigger，不能把这写成“默认产品运行中的刷新故障”。

**反证：** `materialize_active_overrides` 已修复启动 active provider；`extension_mapped_catalog.rs:140-286` 测试 active additions、缺 wire 拒绝和回滚；`provider_factory.rs:1984` 测试产品 profile model override 的 wire 字段。它们都不是“extra route + extension override + 随后跨 provider dispatch”的测试。constructor `try_new` 的一致性验证不涵盖注册后独立 registry override。

**保留理由与代价：** registry overlay 允许无需改 adapter 的模型目录扩展，产品 listing 也有价值；立即删除 overlay 会破坏 extension 行为。原样保留则造成审计 route facts 与实际 request 不一致，触及 INV-001，属落实既有契约的候选修正，不必仅为此扩大正式规范。

**候选方案：** 推荐在一次 route preparation 中生成唯一 effective-model binding，并要求 wire 消费同一 binding；API 具体形状待人类确认。最小替代是对所有 dispatchable route 在共享前物化 overrides，拒绝无法物化的 route，并暂时禁止动态 catalog 改可执行字段。这减少破坏面，但保留刷新缺口。不得只加一个新 model 参数而永远保留两个权威路径。

**机械验收设计：** offline 伪传输观测 endpoint/wire/body，fixture 覆盖初始/extra route、原 ID/新 ID、wire/compat/effort/base URL/能力/上限变化；断言 picker、prepared facts、wire 来自同一结果。无 concrete wire 的覆盖在 auth/HTTP 前失败。旧 prepared call 继续使用旧 binding，后续 call 使用新 binding。保留 active override rollback 测试，新增跨 route 用例。测试尚未执行。

## O-003：公开旧 Config/Model/Error 壳缺少当前生产职责

**分类 / 置信度：** 已证明仓库内冗余公开 surface；候选删除，高；外部消费者不明。

**职责、消费者与证据：** `config.rs:6-26` 只有 `Config { providers }`、`ProviderConfig { name, api_key: Option<String>, base_url }`、单独字符串 Error/Result；`model.rs:3-9` 只有 `Model { id, ProviderKind, context_window: u32 }`；`provider.rs:623` 的 ProviderKind 仅被旧 Model 与 `provider_trait.rs:312-328` 构造测试消费。root `lib.rs:44,48` re-export 这些壳。实际产品使用 product `OpiConfig`、`ModelInfo`、`ProviderError` 和 `ResolvedAuth`。仓库搜索未找到旧 Config/Model 的生产数据流。

**可观测影响条件：** 下游按 root 导出选择旧类型，会得到与当前路径不连通的模型/配置/错误对象；旧 ProviderConfig 的 API key 是明文 String 并派生 Debug/Serialize，和当前 secret wrapper/closed error contract 不同。此处不是证明运行产品已经泄密，因为未找到产品消费。

**反证：** 公共导出意味着未知外部 embedder 可能编译依赖它；不能因仓库无消费者就称“没人用”。ApiKind 与 WireApi 也有名称重叠，但 ApiKind 被 AssistantMessage 持久化与签名重放使用，不在本项删除列表。

**原样保留理由/代价：** 保留能避免外部源代码破坏；代价是无状态机价值、错误推荐路径和不同 secret discipline。没有发现 owning contract 要求继续兼容这些旧壳。

**推荐处置：** 作为需确认的 0.x scoped breaking change 删除旧 Config/ProviderConfig/Error/Result、Model、ProviderKind 及 root re-export；替代为已有 ModelInfo/ProviderError/collection+resolver，不创造新 facade。必须同步 crate README/docs/examples 和 Unreleased，禁止历史 released changelog 改写。外部是否给一个版本 deprecated 过渡由用户决定；过渡若获准必须有明确版本退出条件。

**测试影响设计：** 删除纯枚举存在测试；保留 current public interfaces 的行为测试；静态确认旧符号在生产/文档/样例无残留（历史研究、release history 可保留有标签引用），compile existing workspace consumers。不得把“所有类型都能 Debug”作为新业务验收。

## O-004：部分 compat 配置是无 wire 行为的 metadata，需作明确退出决定

**分类 / 置信度：** 当前能力声明/配置 surface 与执行语义分离；候选删除/修正，高。不是所有 compat 字段都无效。

**owner、消费者、证据：** `model_info.rs:290-303` 的 Chat compat 字段包含 `reasoning_effort`、`require_assistant_after_tool_result`、`supports_store`、`supports_developer_role`、`supports_reasoning_effort`。product `config.rs:614-621,1622-1632,1688-1725` 接受、合并这些值；factory `:1185-1196` 继续传递。搜索 runtime wire 未见后三项的读取；前两项 README 明言 legacy metadata / metadata-only。Chat body `openai_chat.rs:1076-1080` 以 request thinking + model map 为 authority。Responses `reasoning_effort` 同样留在 compatibility/config 中，但 request/model map 驱动 wire。

**可观测影响条件：** 用户配置 `require_assistant_after_tool_result = true` 不能得到补齐 assistant turn 的行为；配置静态 reasoning_effort 不等于请求启用 thinking。supports 字段虽可以被保存，但并未约束实际 wire，不能当 capability enforcement。

**反证：** `strict_tool_schema` 确实在 `openai_chat.rs:1056-1058` 发 strict；system role、max tokens key、usage、cache affinity、endpoint path 有真实实现。不能以本发现删除整个 compat 类型。现有 README 已诚实解释两个 legacy 字段，因此这不构成未披露默认产品缺陷。

**保留理由/代价：** 输入格式兼容可能有外部使用者；metadata 可支持诊断。但未找到真实诊断 consumer，仅接受并保留值造成配置意图含糊。没有证据证明需等待 pi 后续设计才能解决。

**候选选择：** 推荐删除无效配置及对应公开字段，旧键显式报告迁移诊断而非静默忽略；thinking 转到唯一 request/model map authority。若用户确认保留某支持字段，则必须指定 enforce/wire consumer 与 offline behavior test；不能仅命名为 capability 就继续无行为保留。这个破坏性输入格式选择须确认，候选 spec 不得写成无条件 MUST。

**验收/退出：** inventory TOML parsing、public structs、factory lowering、README 中英、配置示例与文档 checker；相同非 legacy 输入的 wire 不变。旧 key 负例产生可机械断言的诊断，旧构造/读取/配置示例静态归零。更新相关 parsing tests，保留真实 compat wire fixtures。

## O-005：目录刷新不是已交付的动态模型产品能力

**分类 / 置信度：** substrate 与产品接线必须区分；延后产品激活，高。

**职责与证据：** `Provider::refresh_models` 默认 None (`provider.rs:65-74`)；`ApiMappedProvider` 明确 None (`api_mapped.rs:284-285`)。collection `:634-701` 按排序 ID 顺序 await、校验非空/重复 ID 与 model validity，全部成功才 replace all；任一失败保留旧目录；None 清掉该 provider 旧 dynamic snapshot。`&mut self` 在安全 Rust 下排斥同一 collection 的并发 refresh，没有当前 epoch race 必须靠 generation 系统解决的证据。

**消费者/测试：** `provider_collection.rs` tests `refresh_models_is_atomic_substrate`、rollback、ordering、repeated/None/empty 验证目录替换语义；未找到 default CLI/harness 的 refresh 调用，README 明确 substrate-only。没有生产 dynamic provider 的 freshness/TTL/credential policy 证据。

**反证和代价：** 不应说“没调用所以没有能力”；可供库调用方使用，离线测试定义了有效目录状态机。但公共 `Registry::set_dynamic_catalog/replace_all_dynamic_catalogs/clear_dynamic_catalogs` (`registry.rs:339-357`) 绕过 collection 校验，除 replace-all 被 collection 使用外，其他仓库消费者仅测试；这使“动态安装都 fail closed”的文档主张不能无条件成立。

**推荐：** 暂不新增 timer/background refresh/cache generations；先处理 O-002 的实际 binding 契约。候选收窄直接目录 mutator 为私有，或让它们执行同一验证；不要同时保留可信与未校验的公开替换路径。是否完全移除无生产 dynamic adapter 的 refresh seam 证据不足：它有目录替换状态机价值，应单独应用 PRIN-002，不能仅因测试用例存在就准入或删除。

**重评触发/验证：** 出现真实动态 provider 与产品刷新需求后，明确失败保留、取消、过期、并发、预算、鉴权、旧 call 的 snapshot lifetime。现有 atomicity/None/empty tests 应 retain；新增 invalid metadata bypass 与 dispatch 一致性测试。未实测。

## O-006：保留鉴权分层与每 logical-call 一次冻结

**分类 / 置信度：** 值得保留的 Rust 选择；高。

**职责/owner/真实 variation：** opi-ai 定义 AuthResolver/ResolvedAuth、OAuthProvider、CredentialStore；product 选择来源、保存凭据、登录/刷新。collection `prepare_call` `:514-550` 在可取消解析之后冻结 auth；provider trait `:29-44` 不允许 adapter 再解析。opaque secrets 与 redacted provenance 分离。Anthropic API key/OAuth、OpenAI bearer、AWS SigV4 不应强行压成无类型 string。

**刷新证据：** product `credential_store.rs:1146-1245`：near expiry 的 OAuth 在 mutation lock 内重读、刷新、成功才写；默认 30s refresh timeout (`:1087-1098`)，避免旋转 refresh token 并发；失败后再读但不写半成品。`oauth_auth.rs:786-906` 有对应离线 mock 测试。锁是 store 级而非 provider 级，可能串行化不同 provider 的刷新，但未见性能证据要求细分锁。

**反证：** 不应把所有 refresh 都塞进 collection；OS credential lifecycle 与模型目录 lifecycle 不同。`AuthDescriptor::StoreCredential.resolve` 返回 Configured 不代表真实可用 (`provider_collection.rs:175-180,594-601`)；probe 是单独状态。factory 真实 listing 路径需要这类不读取 secret 的 metadata，而 dispatch resolver 才是 authority。

**保留理由/代价：** 单次冻结使重试不悄然换身份；代价是 call 中凭据过期不自动重解，也不自动重新登录。这是明确可审计的边界，应保持 credential failure 终止 logical call、新 call 才重新 prepare。auth_status 的非 authoritative 性应在所有消费处明确，不合并为一个含糊“authenticated”bool。

**建议：** 保留 typed scheme、resolver/store/login 分层、冻结与 fail-closed credential errors；若要改“同 call 刷新”需重要决策，不能照搬 pi 凭据重试行为。保留 per_request_auth、auth_contracts、oauth_auth 与 prepared-call tests；新增能力无需新泛化 credential framework。部分 provider resource lifecycle 问题由主审计单独记录。

## O-007：AWS credential discovery/process 的层级值得 Placement Review

**分类 / 置信度：** 已存在行为与现行层级原则的张力；候选移动，事实高、处置中；需人类确认。

**职责/消费者/证据：** `opi-ai/src/bedrock/credentials.rs` 不只是 SigV4 wire：`:220-250` 读 profile files、`:286-294` 执行 credential_process；`:580-601` Windows 选 PowerShell，其他系统选 sh，启动 shell；`:536-544,632-660` 有 10s/64KiB 与清理约束；`:734-745` 环境读取 helper。product `provider_factory.rs:2037-2041,2094-2107` 的 BedrockAuthResolver 实际使用它。它不是测试孤岛。

**规范关系：** §8.1 product neutrality 明示无 operating-system opinion，minimal authority 限制 optional 行为扩大的 IO/dependency surface；PRIN-003 mechanism/policy 分离。AWS 签名与 protocol adapter 的存在仍有正当核心价值，不能从此推导“Bedrock 不应在 opi-ai”。需要讨论的是文件/环境来源优先级、shell 执行与产品配置 policy 的 owner。

**反证：** helper 可接受由 caller 注入的 env/profile paths，未在模块加载时自动执行；credential_process 是显式选定 AWS profile 的真实用户功能，有 bounded output/timeout/cancel cleanup 和 typed redaction-safe errors。既有用户工作流不是可随意删除的“历史负担”。本次没有触发环境读取或执行 credential process。

**保留理由/代价：** 留在库使 AWS embedder 复用已有 local chain；代价是 Agent Core 带 OS/process opinion 与权能，和 OAuth product-only 路径不同。要移动跨层需 ADR（§8.2），若人类要继续认定为核心例外，则需明确是否修订现行 admission 条款或证明它符合条款。

**推荐候选：** 保留 SigV4/typed credentials/concrete wire 在 opi-ai；把凭据发现、profile IO/process executor 放到 product AuthResolver，使用已有 AuthResolver seam，无新增 crate/trait 默认要求。外部 agent-neutral复用需求出现时再评估 Companion；当前没有两个外部消费者证据。

**迁移/验收设计：** 冻结来源优先级、partial source fail closed、safe errors、process timeout/output bound/cancel cleanup、provenance；迁移前后相同 isolated fixture 与 fake process 输出等价。把相关 tests 随 owner 更新，旧 `opi_ai::bedrock::credentials` discovery exports 退出需 scoped breaking-change 确认与明确过渡。不能只复制实现到 product 并保留两个 authority。

## O-008：工具 schema 执行校验属于 Agent；partial JSON 不应当作调用授权

**分类 / 置信度：** 保留现有责任边界，部分 wire 能力需澄清；高。

**事实与 owner：** `message.rs:140-150` 的 ToolCall arguments 是原始 String，ToolDef schema 是 JSON Value；stream mapper 增量累积字符串。`opi-agent/src/agent_loop.rs:1767-1797` terminal 参数 JSON 解析；`:2251-2289` hook 后对最终参数做 schema validation，再授权/执行。Chat `openai_chat.rs:1045-1063` 的 strict 只加 provider 字段，没有本地自动把任意 schema 转成严格闭合 schema。

**消费者/反证：** 工具执行与 authority 是 Agent 真实语义；library direct users也可拿完整 raw arguments，但 library 不应执行工具。现有 `opi-agent/tests/tool_validation.rs` 等测试位于真实执行 seam。不能凭 pi 有 partial JSON helper 就断言 opi 必须增加工具校验框架；也不能把 strict:true 当作已经验证 schema 可被服务接受。

**保留理由/代价：** raw 字符串保留 malformed/truncated 输入证据，避免 partial parse 可用对象被误执行。UI 若要实时显示 partial tree，需要明确 preview-only consumer；目前没有证据准入新的公开 parser seam。

**推荐：** 保留 Agent 中最终参数校验与授权；定义 provider strict 支持的输入条件和本地拒绝/转换契约后才加行为。使用相同 tool schema + invalid JSON + length terminal 的 offline 验收，不复制 pi 测试数量。Partial JSON preview 与 deferred tool loading 延后到真实消费和权限契约确认。

## O-009：工具返回图片仅有中立表示，当前 wire 多数只发送占位文本

**分类 / 置信度：** 能力部分实现/语义缺口；修正或明确有意差异，事实高、目标待确认。

**事实、owner与消费者：** `OutputContent::Image` 存在并能 serde round-trip；`Request::contains_image_content` (`provider.rs:136-149`) 将 tool-result image 计入 vision preflight。但 serializers：Anthropic `:1309-1317`、Chat `:1419-1420`、Responses shared `:769-778`、Gemini `:600-601`、Bedrock `:1020-1021` 都把 tool image 转为 `[image: MIME]` 文本。真正 image bytes/URL 入 wire 的覆盖主要是 UserMessage 的 InputContent::Image。

**成立条件 / 影响：** 工具向模型返回图片时，即使模型 supports_images=true，模型也可能仅收到描述 MIME 的占位符。文本模型则可能因该未发送的图像被 capability preflight 拒绝。不能把这等同于 image generation：这是现有工具结果的视觉输入，而生成图片是另一项操作/范围。

**反证和测试局限：** `image_provider_serialization.rs` 验证的是 user image；`output_content_image.rs` 明确只验证 variant 与 serde；这些测试不证明 tool image round-trip 到模型。对两个生产 crate 的 `OutputContent::Image` 搜索仅找到 compaction 消费，没有直接构造图片的默认内置工具；`ToolResult.content` (`opi-agent/src/tool.rs:92`) 仍允许 SDK/extension 返回它，不能据此说能力不存在。已确认的生产图片 producer 是 CLI/TUI attachment：product `image.rs:23-57`、`main.rs:958,1321`、`interactive.rs:1535`，产生 User InputContent，而非 tool output。因此本项不是默认内置工具已经普遍出错的断言。

**保留理由/代价：** placeholder 避免不支持 wire 的格式错误并可保留部分文本上下文；代价是 silent data loss，能力声明不足以解释。候选保持 placeholder 的方案必须被定义为有意降级并可观测，不能继续被描述为完整 tool-image 支持。

**推荐与验证：** 人类确认对齐“工具图片可见”范围后，按每 wire 真实能力发送、合法重排或明确拒绝；不得强制所有 wire 原位同构。机械验收使用 bytes/base64/URL×user/tool×supported/unsupported 模型，伪传输断言图片内容与tool call/result关联、顺序、失败标记，失败不能静默只留 MIME。保留现有 serde 与用户图 fixtures，新增 tool image wire fixtures。

## O-010：跨模型重放需要独立契约，不宜先复制整个 transcript 框架

**分类 / 置信度：** 有合理现存机制，也有明确未覆盖语义；保留已证行为并修正必要缺口，中高。

**事实与 owner：** `Request` 持有单一 current system、tools、messages (`provider.rs:102-106`)，Message enum 无逐时点 system/tool-change entries。Anthropic `:1265-1303` 只在同 ApiKind/provider/requested-model 时保留签名/opaque thinking；换 route 把有 signature 的 readable thinking 改为 text，opaque block 丢弃。OpenAI Chat `:1397-1398`、Responses shared `:760-761`、Gemini `:583-584` 省略 thinking；Bedrock `:1004-1007` 可保留 readable、丢弃 opaque。

**真实价值与反证：** `anthropic_thinking_roundtrip.rs:71,160,224,285` 设计覆盖 HTTP→storage→tool continuation、跨 route剥离、UTF-8与多block signature。当前模型 alias/requested model 与 response_model 已明确分离。ApiKind 虽较粗，不能无审计直接删除，因为持久化和重放消费者仍用它。

**需要确认的缺口：** current system/tools 适用于请求整体；没有 pi transcript 时间线等价保证。Anthropic serializer 将 malformed/non-object tool arguments 变为 `{}` (`:1277-1281`)，Gemini/Bedrock 也有 parse fallback；Responses 则保留字符串。换模型的 tool IDs、未配对结果、failed/aborted assistant 是否可重放需矩阵核验。单凭 serialization 函数存在不能断言已完整跨 provider repair。

**保留理由与代价：** 现有简单 Request 适合 stateless HTTP，完整 transcript 迁移会影响 Agent/session/hooks/compaction 和持久化格式；收益必须由“中途 system/tools 的历史语义”真实使用场景证明。签名保真与不泄漏 opaque block 则已经是 durable contract，应保留。

**推荐：** 先确认对齐的是 current-context 请求还是历史 transcript；可以先为合法 history 的 route转换定义共同输入/输出与失败契约，不先新增通用转换 trait 或另一个持久化账本。任何不兼容历史类型替换需 migration 与 exit test，不无限双轨。

**验收设计：** same/cross provider+wire+model；signed/unsigned/redacted thinking；aliased response model；完整/不完整tool JSON；orphan/duplicate tool ID；failed/length/cancelled assistant；system/tools 变更之前和之后。只归一化时间/request ID，不归一化错误类型、顺序或终止。现有 signature tests retain，新语义未确认前不宣称同 pi 等价。

## O-011：Usage 的未知值处理应保留；细分字段完整性仍需明确

**分类 / 置信度：** 既有正确方向；保留，高；对子集聚合的“完整总数”表述须谨慎。

**事实与 owner：** `stream.rs:40-115` 区分 `Usage::unknown`、reported=true 的零值、optional child subsets；父桶总计不重复加 reasoning/cache-1h；`:197-241` 累积并用 unknown_turns 保留整次usage缺失；ModelPricing 验证非负/有限值与 tier 顺序，`:442` 采用严格高于门槛。product harness `:2347` 与 session_coordinator 消费 pricing，成本不是 adapter 成功的前置条件。

**反证：** child subset 的 None 与 0 保留在单次 Usage，但累计过程中某一 turn None、另一 turn Some 时，累积 Some 仅代表“已报告子集之和”，不是所有 turn 的完整子集总数；unknown_turns 只看 turn.reported，并不跟踪每个 child 的缺失。不能把 CumulativeUsage 所有 optional 字段都写成完整会计事实。

**保留理由/代价：** 比默认0更可信；未知成本留空不会阻断成功流。若将每个桶升级 Option 或增加 completeness metadata，需先有下游展示/预算使用场景与格式迁移理由，不能为“比 pi 更完整”盲增框架。

**推荐/验证：** 保留整次unknown、zero/absent子集与父桶一次计费；候选文档明确部分聚合的含义。保留 usage_cost 测试与 session cost tests；加入 mixed present/absent 子集的语义测试仅在该行为确认后。不得用未知计费数据作可靠成本限额。

## O-012：crate README 的“无 environment IO”主张部分失效

**分类 / 置信度：** 文档主张部分过时/过宽；修正，高。

`README.md:84-90` 把 OAuth/keychain分层概括为 opi-ai 不做 keychain/environment/presenter IO。源码的 `AuthDescriptor::EnvApiKey.resolve` (`provider_collection.rs:157`) 读取 env，proxy helper (`http.rs:270-283`) 读取 env，Bedrock credentials 又有 env/profile/process IO。README 同时在 Proxy 节承认 env fallback，所以自身表述也有范围冲突。

不是建议删除环境代理：它是现有 provider HTTP 适配选择。推荐把声明限缩到具体 seam：“CredentialStore/OAuth/LoginPresenter 的接口不固定 OS backend；产品提供 keychain/login 实现；某些 provider/transport helpers 仍有显式 env/local IO”，或在 O-007 获准移动后按结果同步。此任务只在研究目录记录，不擅改 crate README。无需为文档句子写运行时测试；最终落地由 doc checker 和源代码核验。

## 有意不作的结论

- 不把 catalog size、provider module 数量或代码行数作为质量排序。
- 不把 image generation/classification/deferred tools/WebSocket 缺少入口直接升级为实施目标；这些改变 model/provider 抽象的范围需分别确认。
- 不将 Request API 中的 provider-neutral options 与实际产品 producer 混为一谈：README 指 timeout/extra_headers/cache retention 多为 substrate，session_id 已由 harness 生产。
- 不因 `ApiKind` 与 `WireApi` 并存即要求合并：前者在序列化消息中，后者在路由元数据中；若统一须先完成 persisted format / history migration 调查。
- 不把所有流生命周期实现认定一致；主研究对 drop、backpressure、terminal、retry 做单独交叉审计。
- 不自动新增 background刷新、缓存 generation、操作通用 trait、插件wire协议或 provider SDK features。

## 推荐送交人类讨论的设计问题

1. **Effective model authority：** 选择统一 prepared model binding，还是先对所有 route 物化并限制动态字段？前者更彻底，后者破坏面更小但保留将来迁移成本。
2. **明确破坏性清理：** 是否删除 O-003 旧壳、O-004 无行为 compat 字段，并选择显式诊断与退出版本？不能将未回答写为批准。
3. **范围与层级：** 工具图片 wire 消费、跨模型重放、historical system/tools transcript 各纳入到哪一级；AWS local credential discovery/process 是否迁往 product AuthResolver 或需修订规范解释？这些不是同一个“pi parity”开关。

可独立继续的研究：为 O-002 补最小 offline reproduction；核验 pi 当前模型 binding 与迁移 history；精查 tool-image 真实生产 producer；细化 replay矩阵；最终审阅候选 requirements 的 owner/trigger/failure/acceptance。以上均无须开始产品实施。

## 本子研究实际检查

实际执行：只读 `rg` 搜索、PowerShell `Get-Content` 带行号源码检查；读取所列技能、规范、清单和测试。没有运行 Cargo tests，没有访问凭据，没有请求模型或付费服务，没有安装依赖，没有修改运行时代码/测试/配置。测试影响：**none**（本文提出将来的 add/update/delete/retain 分类不代表已变更）。最终 repo doc checker 与 diff check 由主研究统一执行。
