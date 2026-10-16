# ai：Provider 集合、转录协议与多种模型操作

分析日期：2026-10-16。对象是 Pi 1.2.0 本地固定源码。源码、测试与文档做静态核查；没有安装依赖、生成目录数据、构建、运行测试、联网或调用 provider。测试影响 `none`。

## 职责、公开边界与资产

`@earendil-works/pi-ai` 将 provider 身份、目录与鉴权，同 API wire 实现分层：`Models` 集合拥有注册与 request orchestration，`Provider` 拥有 auth/catalog/具体操作，API adapter 转换 provider 协议。它不执行模型提出的工具，不保存聊天记录，不承担 agent turn 调度或应用权限 UI。chat catalog 筛选支持 tool calling 的模型；image/classifier 按各自能力入目录。 [README 职责](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1)、[Provider 契约](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L144)。

生产依赖精确声明 Anthropic SDK `0.129.0`、AWS Bedrock Runtime `3.1127.0`、Google GenAI `2.21.0`、OpenAI `7.19.0`、Smithy HTTP handler `4.12.1`、HTTP(S) proxy agents `9.1.0`、partial-json `0.1.7`、TypeBox `1.3.27`，内部 telemetry `^1.2.0`；Node engine `>=22.19.0`。lockfile 中 ai 实际解析的 OpenAI 是 `packages/ai/node_modules/openai:7.19.0`；根提升的另一副本不能代替它。这里仅核对声明、锁定记录与调用方，未取得全部外部依赖内部源码，不能声称审计 SDK 内部行为。源码统计 200 个 TS 文件/27420 行，测试 171 文件/43168 行。 [manifest](../../../../.repo/pi-1.2.0/packages/ai/package.json#L1)、[锁文件](../../../../.repo/pi-1.2.0/package-lock.json)。

exports 有根、`./models`、`./compat`、`./providers/*`、`./api/*`、`./utils/*`、`./oauth`、Bedrock 与 Bun OAuth 包装子路径；CLI 为 `pi-ai`。根导出 TypeBox、types、Models/auth/stores、faux、transcript/frame/validation/retry utilities，不导入 built-in catalogs 或 SDK 实现；`./models` 提供更轻运行时；单 provider factory 只引用自己的 catalog 与 lazy API；`providers/all` 显式拉入全内建集。`compat` 保留 global registry/catalog/env-key 旧 API 且有注册 side effect；`oauth` 只留 extension compatibility 类型，不能据旧名称推断公开底层 login。 [index.ts](../../../../.repo/pi-1.2.0/packages/ai/src/index.ts#L1)、[exports](../../../../.repo/pi-1.2.0/packages/ai/package.json#L14)、[README bundling](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1665)、[迁移边界](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1819)。

编译前需要 model-data schema v6 的 ignored JSON assets；脚本将结构、生成时间、结构 hash 与各数据 hash 写入 `.manifest.json` 并校验。package `build` 先严格 generate（可能联网），`build:offline` 则校验已存在数据、tsc、复制 data。当前本地快照没有 `src/providers/data/`，因此 TS wrappers/源码存在不等于目录值或离线构建依赖齐备；本次没有 hydrate，不能验证实际 catalog 全量值。 [构建脚本](../../../../.repo/pi-1.2.0/packages/ai/package.json#L65)、[model-data 身份](../../../../.repo/pi-1.2.0/packages/ai/scripts/model-data.ts#L5)、[README generation](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1881)。

## 类型与状态归属

模型是普通可序列化数据，以 provider+type+id 标识；chat 的 type 可以省略（等效 chat），image/classifier 必须显式区分。同一个 upstream id 可在多个操作类型出现。默认 getModels/getModel/getAvailable 仍只返 chat；`*OfType` 与 getAll* 才含 image/classifier。`hasApi` 只对 chat 成立。持久化 unknown model type 在 restore/fetch 时被过滤，避免新版类型破坏旧 runtime。 [类型与操作检查](../../../../.repo/pi-1.2.0/packages/ai/src/utils/model-operations.ts#L17)、[unknown 类型过滤](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L129)、[读取](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L435)。

`ModelsImpl` 拥有 provider map、注入或内存 credential/model stores、AuthContext、每 provider 的 refresh generation/controller 与 publication chain。Provider closure 持有自身动态目录；对话 messages/context 仍由 caller 持有。替换、删除、clear provider 会 supersede 旧 refresh，不是删除 credential/catalog 持久记录。默认 stores 仅进程内，应用才能提供文件/数据库持久性及跨进程串行保证。 [state](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L395)、[Provider 构造](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1047)、[credential store](../../../../.repo/pi-1.2.0/packages/ai/src/auth/credential-store.ts#L4)。

Messages 允许 system/user/assistant/toolResult。assistant 有 text/thinking/toolCall、provider/model/api、usage/cost、stop reason、responseId/responseModel、签名、诊断与可选 duration；tool result 支持 text/image、JSON-compatible details、isError、usage、duration。`stopReason` 不只有 README 列举的 pending/stop/length/toolUse/error/aborted，源码还包含 `deferred`。 [types.ts:408](../../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L408)、[stop reason/JSON](../../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L469)、[assistant/result](../../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L565)。

## 一次 chat request 的执行链

`Models.stream*` 同步把 shorthand Context 归一成 TranscriptContext，然后返回外层 lazy stream；异步阶段 require chat/provider → resolve auth → merge headers/env/endpoint → provider.stream* → 选择 model.api implementation/lazy SDK → request conversion → onPayload → 网络 → onResponse → provider event observer → normalized events → terminal assistant。`complete*` 只等待 stream.result。auth/unknown provider/lazy load 失败编码为预生成 error；direct API 绕过 Models 的 provider auth 与归一化，某些 streamSimple 缺 auth 会同步 throw，不能混用两层契约。 [Models stream](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L849)、[lazyStream](../../../../.repo/pi-1.2.0/packages/ai/src/api/lazy.ts#L31)、[direct API 指南](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1362)。

header 优先序是 provider auth/model headers → explicit options.headers → Models-only transformHeaders；case-insensitive merge，null 表达下层支持的删除。transform 完成后从 provider options 移除；auth 可派生 baseUrl，request env 覆盖 auth/env 配置，但并不意味着清空全部 process env，overlay 对未给定或空值仍可回退。`getAuth` 也可能刷新 token，预先 getAuth 再 stream 会重复鉴权；transformHeaders 提供最终修改点。onPayload 可替换 wire payload，onResponse/provider-event callbacks 被 await，慢 callback 延迟消费，throw 可使 request 失败。 [applyAuth](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L849)、[env overlay](../../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L95)、[observer 文档](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1201)。

API 入口共享统一 Message/Event，但能力不同：

| API 家族 | 当前转换与传输边界 |
|---|---|
| anthropic-messages | SDK 建 client/request，自己消费 SSE；签名/effort/inline tool changes、strict sampling 与兼容元数据控制 payload；start 在收到成功 headers 后发出 |
| openai-responses / azure-openai-responses | SDK request，shared Responses conversion；保留 encrypted reasoning、item/tool namespace/phase 等 replay metadata，错误 terminal 与未完成工具必须区别成功工具 |
| openai-completions | 多个 provider 共用；URL detection+model.compat 决定角色、thinking、cache/affinity、tool schema 与 token 字段，而不是 provider 名字等于唯一协议 |
| google-generative-ai / google-vertex | Google SDK，各自鉴权/config；collapse system messages，共享 tool/image/reasoning conversion；拒绝自定义非 global fetch，而非静默绕过 |
| bedrock-converse-stream | Node-only lazy AWS implementation，ConverseStreamCommand，SigV4/Bearer/ambient profile，proxy 可切 HTTP/1 handler；SDK abortSignal 与 AWS diagnostics |
| mistral-conversations | 名称保留历史，当前用 native HTTP `/v1/chat/completions` + SSE，不再依赖 Mistral SDK；Mistral 专用 message/reasoning/id 规则 |
| openai-codex-responses | 独立 legacy subscription transport，SSE/WS auto fallback、session connection/delta replay；不是普通 OpenAI API-key route |
| pi-messages | Radius gateway 的 Pi transcript JSON/SSE；POST `/messages`，compact Pi event 重建为统一事件；无 terminal EOF 必须报错 |

入口证据：[Anthropic request](../../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L610)、[Responses](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-responses.ts#L127)、[Google](../../../../.repo/pi-1.2.0/packages/ai/src/api/google-generative-ai.ts#L59)、[Bedrock](../../../../.repo/pi-1.2.0/packages/ai/src/api/bedrock-converse-stream.ts#L249)、[Mistral HTTP](../../../../.repo/pi-1.2.0/packages/ai/src/api/mistral-conversations.ts#L298)、[Codex](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-codex-responses.ts#L238)、[Pi messages](../../../../.repo/pi-1.2.0/packages/ai/src/api/pi-messages.ts#L355)。这张表不是逐行审计全部 adapter 的声明；本次深入重读核心 orchestration、升级相关 adapter 与被文档主张触及的路径。

## Transcript：系统提示、工具声明与跨模型 replay

`normalizeContext` 将 shorthand prompt/tools 加为首条 system message，provider-facing Context 只剩 messages。replay 顺序应用 content 追加、sections 按名字替换/null 删除，以及 toolsRemoved 后 toolsAdded（同名替换）。`resolveTranscript` 对支持 mid-conversation system 的模型保留原位置，否则 collapse 成一个最新 leading system message。声明工具只有 schema/description 等 JSON 数据，剥除 executable/display fields；工具实际执行在 agent/应用。 [归一与 replay](../../../../.repo/pi-1.2.0/packages/ai/src/utils/transcript.ts#L10)、[collapse](../../../../.repo/pi-1.2.0/packages/ai/src/utils/transcript.ts#L104)、[tool declaration/diff](../../../../.repo/pi-1.2.0/packages/ai/src/utils/transcript.ts#L122)。

native tool replay 按能力分支：Anthropic 支持 inline tool additions/removals/同名重定义并稳定顶层 prefix；OpenAI Responses 可锚定 additive tools，非 additive 情况回退当前顶层工具；不支持 mid-system 的模型统一 collapse。旧 `hasToolRedefinitions` 保留但 deprecated，不是当前 transport 的必需准入检查。 [transcript tools](../../../../.repo/pi-1.2.0/packages/ai/src/utils/transcript.ts#L179)、[README 当前 native 规则](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1601)。

`transformMessages` 判定同 provider+api+model 才是同模型；跨模型把普通 thinking 变纯 text、清除不适用 signature，redacted opaque thinking 仅同模型保留。非 vision 输入被替换为说明占位，并非简单静默丢弃。error/aborted assistant 会被过滤，孤立 tool calls 生成 `No result provided` error result；跨协议 ID 正规化同时更新对应 result。此变换只构建模型请求视图，不能据此认为 caller 的原始 transcript 已被删除或持久化修改。 [transformMessages](../../../../.repo/pi-1.2.0/packages/ai/src/api/transform-messages.ts#L12)、[thinking](../../../../.repo/pi-1.2.0/packages/ai/src/api/transform-messages.ts#L92)、[孤立调用/失败过滤](../../../../.repo/pi-1.2.0/packages/ai/src/api/transform-messages.ts#L158)。

## Stream、frame 与耗时

`EventStream` 用双栈 FIFO 分别存事件与 waiter，push 不等待消费；终止事件先 resolve result，再进入消费队列，done 后忽略后续 push。队列无容量或生产者背压，消费者停止迭代不会自动 abort provider。`partial` 是共享的 live response，保留旧 event 不等于保留 event-time snapshot；按 contentIndex 处理 interleaving。 [EventStream](../../../../.repo/pi-1.2.0/packages/ai/src/utils/event-stream.ts#L3)、[README 事件契约](../../../../.repo/pi-1.2.0/packages/ai/README.md#L701)。

1.1.0 起 `AssistantMessageEventStream` 成为独立 class，私有 monotonic 起点使手写 generic EventStream 不再满足同一类型。terminal push/end 仅在 message 尚无 duration 且 timestamp 未早于 stream 时计算 round 毫秒；已有 duration 保留，早于 stream 的 deferred result 不补计时。转发流保留内层测量，因此 duration 不是 Models 外层 auth/loading/观察者/持久化全链 wall time。 [timing](../../../../.repo/pi-1.2.0/packages/ai/src/utils/event-stream.ts#L91)、[保持 inner timing 的测试](../../../../.repo/pi-1.2.0/packages/ai/test/event-stream.test.ts#L120)。

`AssistantMessageFrameEncoder` 每流一个状态机，捕获 start metadata、text/thinking 增量与权威 end、tool JSON checkpoint/delta；解决生产者已前进而 queued start.partial 已变长时的重复字符。frame 不含 terminal settlement，需单独保存最终 assistant；reducer 重建 pending partial 并拒绝坏顺序、kind/index 等，不验证工具参数。当前 frame metadata 也不等于完整 final assistant（duration/thinkingLevel 等终态数据应由最终 message 保存）。 [frame 类型/克隆](../../../../.repo/pi-1.2.0/packages/ai/src/utils/assistant-message-frame.ts#L4)、[README frame](../../../../.repo/pi-1.2.0/packages/ai/README.md#L726)。

## 鉴权、目录刷新与事务边界

显式 request apiKey（provider 有 API-key handler 时）优先；否则 stored credential 拥有 provider：存 OAuth 则走 OAuth、存 API key 则走该 resolver，失败不静默回退环境。无存量才尝试 ambient env/profile/ADC。CredentialStore.list 只列 provider/type，不能解析 secret/运行命令。API-key credential 可带 provider-scoped env。 [resolve](../../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L27)、[store](../../../../.repo/pi-1.2.0/packages/ai/src/auth/credential-store.ts#L30)。

OAuth 少于默认五分钟 validity 时进入 serialized modify，并在锁内 double-check；一旦 refresh 真正开始，caller abort 只让外部停止等待，refresh/persist 以独立 15,000ms timeout 继续，防止丢失已轮转 refresh token。是否跨进程排他由注入 store 保证，内存实现只有每 provider promise chain。refresh 失败保留旧 credential 并报 ModelsError，登录/注销操作才拥有写删语义。 [refreshStoredOAuthCredential](../../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L102)、[stored OAuth resolution](../../../../.repo/pi-1.2.0/packages/ai/src/auth/resolve.ts#L157)。

refresh 对所选 dynamic providers 并行：先 read credential、offline restore cached catalog，再在允许 network 且有有效 credential 时 fetch。每次 refresh 或 provider replacement 递增 generation 并 abort 前代；publication 按 provider 串行，先选定 persist mutation，再检查 generation/signal，最后同步 update in-memory。错误以 provider→error map 汇总，caller abort 能及时返回 aborted，即使自定义 provider 不合作；这不能保证其底层工作已停止。store 必须合作 signal 才能防止不合规旧写长期占链。 [refresh orchestration](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L491)、[publication](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L509)、[两阶段刷新](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L558)。

通用 createProvider 将 dynamicModels 以 type+id 覆盖/添加到 static baseline，不能表达“远端移除静态模型”。Radius 特别实现 `dynamicModels ?? baseline`：获得缓存/新 catalog 后整体替换，空数组也权威，因此组织 owner 禁用模型不会重新出现；这就是 1.1.0 改动，README 368 的“overlay”已不准确。 [通用 merge](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1062)、[Radius replacement](../../../../.repo/pi-1.2.0/packages/ai/src/providers/radius.ts#L21)。

1.2.0 的 `personalizeOpenRouterCatalog(catalog, RefreshModelsContext)` 从 `/models/user` 收窄 chat catalog，并更新该 key 的 context/output limits/prices；image/classifier 不过滤，401/403/404 保留原 catalog，无 key 也不联网，调用合并 signal 与 15 秒 timeout，使用 context.baseUrl 支持 regional/proxy endpoint。它是显式 helper，`openrouterProvider()` 本身仍静态，没有自动 refresh 接线；coding-agent 的 model-runtime wrapper 才引用该 helper。不能写成所有 Models 用户默认得到 key-specific catalog。 [helper](../../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter.ts#L16)、[factory](../../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter.ts#L62)、[实际 caller](../../../../.repo/pi-1.2.0/packages/coding-agent/src/core/model-runtime.ts#L234)。

## 图像、分类与 deferred 操作

image/classifier 与 chat 共享 provider/credential/catalog，但操作互斥。`generateImages`/`classify` try/catch type/provider/auth/adapter 错误，返回 error/aborted result，不 reject；分类先按 model.input 检查图片，再交 API。OpenRouter 是当前内建 dedicated image provider；单次返回 base64 output，不参与 tool calling。 [Models operation](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L960)、[能力检查](../../../../.repo/pi-1.2.0/packages/ai/src/utils/model-operations.ts#L45)。

分类主链：structured JSON state+questions → operation adapter → normalized choice/score/bool answers。System One 把 bool 转 wire noul；当前 System One 与 llama-cpp classifier 明确拒绝图片，即使公共 ClassifierContext 可以带 images。llama-cpp 每次预验所有 question，然后顺序单题生成 label token logprobs/softmax，复用 prompt prefix；它没有 service usage。 [System One](../../../../.repo/pi-1.2.0/packages/ai/src/api/system-one-shared.ts#L99)、[llama-cpp](../../../../.repo/pi-1.2.0/packages/ai/src/api/llama-cpp-classify.ts#L425)。

1.1.0 新 OpenAI Decisions adapter POST `/decisions`：纯文本 state JSON；有图片时构造同一 user message，最多 128 images；bool→predicate，并把 true/false 含义附入 instructions。按 answer name 对齐，缺失、错型或 refusal 使整个 result error；先记录 usage 再解析 answer，失败仍保留已计费 usage。HTTP helper 默认 2 retries、每次独立 timeout，Decisions 特别禁止重试 504（文档解释约 5 秒 gateway/大输入历史观察，不是本次测量）。OpenAI OAuth availability filter 隐去 classifier，但这是 listing policy，手工传 model 仍会派发并由服务拒绝。 [Decisions](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-decisions.ts#L73)、[解析与 504](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-decisions.ts#L106)、[request helper](../../../../.repo/pi-1.2.0/packages/ai/src/api/classifier-shared.ts#L60)、[OpenAI filter](../../../../.repo/pi-1.2.0/packages/ai/src/providers/openai.ts#L23)。

deferred 通过持久 JSON handle（provider/modelId/api/id/可选 expiry/poll/data）与 `streamDeferred/fetchDeferred/cancelDeferred` 提供机制；retrieval 也重新鉴权。ProviderStreams/Provider 的 fetch/cancel 都是可选能力，createProvider 按实际 implementation 暴露；faux 模拟 pending/ready/failed/cancelled 用于测试。不能把类型存在推断为所有真实 provider 支持或自动轮询/恢复；本包没有 durable scheduler，handle 不应包含运行时 telemetry context。 [DeferredHandle](../../../../.repo/pi-1.2.0/packages/ai/src/types.ts#L519)、[Models dispatch](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L924)、[可选 capability](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1134)。

## 工具采样、验证、thinking 与成本

TypeBox schemas 定义模型工具，但 `toolcall_end` 只表示 parser 完成，不是 schema 或安全校验通过。`validateToolArguments` 维护 WeakMap schema validator cache，对 TypeBox/普通 JSON schema 做 coercion，union 先保留已经匹配 arm，再尝试复制后转换，避免 null 被错转。严格 JSON schema 与 grammar constrained sampling 是 provider-side 生成约束，仍不能替代执行前 validation；执行权限在上层。 [validation](../../../../.repo/pi-1.2.0/packages/ai/src/utils/validation.ts#L1)、[union](../../../../.repo/pi-1.2.0/packages/ai/src/utils/validation.ts#L175)、[README sampling](../../../../.repo/pi-1.2.0/packages/ai/README.md#L541)。

1.2.0 Anthropic strict prefer fallback 只在成功 headers 前，遇指定 400 invalid_request_error 的 grammar-size/tool-count/union-limit 文本才重建一次 request，把 prefer 取消 strict、保留 require，加入 diagnostic；onPayload 会在重建请求上再次执行。已开始 SSE、hook exception、abort 或第二次拒绝都不再 fallback；该降级只影响本次请求，不改 model/tool 原对象策略。 [fallback 状态机](../../../../.repo/pi-1.2.0/packages/ai/src/api/anthropic-messages.ts#L639)、[测试矩阵](../../../../.repo/pi-1.2.0/packages/ai/test/anthropic-strict-tool-fallback.test.ts#L128)。

thinking level 是模型 metadata，不是全 provider 恒定能力。standard off/minimal/low/medium/high 可显式 null 排除，xhigh/max 必须有非 null map 才 opt in；clamp 先向更高可用 level 查找，再向下。simple API 将其映射成各 provider budget/effort，不抹平原生 semantics。cost 统一 USD/1M token，按 input+cacheRead+cacheWrite 找最高“严格大于”阈值，全请求套该 tier，1h cache write 单独按 2x input；reasoning 是 output 子集，不能再相加。 [thinking/cost](../../../../.repo/pi-1.2.0/packages/ai/src/models.ts#L1206)、[model schema](../../../../.repo/pi-1.2.0/packages/ai/src/providers/model-schema.ts#L25)。

新 pricing helpers 从 models.dev/OpenRouter/Gateway 引入 prompt-length tiers；OpenRouter -1 router placeholder 价变 0，缺失 tier rate 沿用 base，time-of-day overrides 明确跳过，因为 ModelCost 无法表达。数值是 catalog estimate 而非账单承诺。输入估算从 4 改为 3.5 chars/token，结合最后有效 assistant usage 与尾部估算；它不是 tokenizer，对多语言/图片/外部 provider 正式 token 限制只是启发式。 [OpenRouter pricing](../../../../.repo/pi-1.2.0/packages/ai/src/providers/openrouter-api.ts#L35)、[estimate](../../../../.repo/pi-1.2.0/packages/ai/src/utils/estimate.ts#L15)。

## 错误、取消、重试与资源

区分三层：adapter 对网络/request 构建失败编码 stream error；`retryProviderRequest` 模仿 pinned SDK 的 pre-response retry（默认 maxRetries 0），带可取消 sleep 与 Retry-After，server delay 超默认 60 秒直接失败，0 关闭该 cap；`retryAssistantCall` 是调用者显式采用的 bounded outer policy，按 errorMessage transient classifier 重做 assistant-producing call，agent core 本身不自动安装它。quota/billing/subscription exhausted 不重试，abort terminal；新 server_busy/upstream provider returned an error wording 加入 classifier。 [provider retry](../../../../.repo/pi-1.2.0/packages/ai/src/utils/provider-retry.ts#L24)、[outer retry](../../../../.repo/pi-1.2.0/packages/ai/src/utils/retry.ts#L110)、[classification](../../../../.repo/pi-1.2.0/packages/ai/src/utils/retry.ts#L253)。

timeout 单位与覆盖依 API 不同，不能一概写成全流 deadline：Mistral 1.2.0 的默认 60,000ms 只等待 response headers，随后移除 timeout 的组合监听，body 使用 caller abort；classifier helper timeout 包含 fetch/JSON body，每次 retry 新建；SDK adapter 依其 request options；Codex 还有独立 WS connect/header/idle 控制。pre-generation lazyStream 错误固定编码 error（包括某些 setup abort），因此“所有取消必为 aborted”过强。 [Mistral](../../../../.repo/pi-1.2.0/packages/ai/src/api/mistral-conversations.ts#L298)、[classifier timeout](../../../../.repo/pi-1.2.0/packages/ai/src/api/classifier-shared.ts#L74)、[lazy error](../../../../.repo/pi-1.2.0/packages/ai/src/api/lazy.ts#L4)。

legacy Codex 默认 auto 先 WS，未开始生成的连接失败可落 SSE，并有会话 fallback 标记；cache retention none 关闭 session cache，正常有 session 的连接按 session+credential 隔离复用，维护 delta/previous-response state 与 idle cleanup。应用结束 session 要调用 cleanupSessionResources，coding-agent dispose 有实际调用；JSON context 本身不拥有 socket。 [Codex transport selection](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-codex-responses.ts#L271)、[cache/cleanup](../../../../.repo/pi-1.2.0/packages/ai/src/api/openai-codex-responses.ts#L911)、[consumer cleanup](../../../../.repo/pi-1.2.0/packages/coding-agent/src/core/agent-session.ts#L1416)。

## 测试预期与未执行范围

测试分离了 mocked/faux contract tests、payload/序列化回归和 credentials/env-gated provider E2E；vitest 30 秒 test timeout 不等于 provider API 默认 timeout。没有运行任何 test，也未通过现有 credentials 激活 live case。静态证据包括：

| 升级/契约 | 可复查测试 |
|---|---|
| 专用 stream duration、inner 保留、旧 deferred 不计时 | [event-stream.test.ts:120](../../../../.repo/pi-1.2.0/packages/ai/test/event-stream.test.ts#L120) |
| strict fallback 一次、require 保留、abort/hook/SSE 不降级 | [anthropic-strict-tool-fallback.test.ts:68](../../../../.repo/pi-1.2.0/packages/ai/test/anthropic-strict-tool-fallback.test.ts#L68) |
| Decisions question mapping、图片上限、refusal usage、504 不重试 | [openai-decisions.test.ts:80](../../../../.repo/pi-1.2.0/packages/ai/test/openai-decisions.test.ts#L80) |
| key-specific OpenRouter chat limits/pricing 与 fallback | [openrouter-user-models.test.ts:63](../../../../.repo/pi-1.2.0/packages/ai/test/openrouter-user-models.test.ts#L63) |
| prompt-length tier 与 time-of-day 排除 | [model-cost-tiers.test.ts:14](../../../../.repo/pi-1.2.0/packages/ai/test/model-cost-tiers.test.ts#L14) |
| orchestration/auth/refresh generations 与 stores | [models-runtime.test.ts](../../../../.repo/pi-1.2.0/packages/ai/test/models-runtime.test.ts) |
| frame replay、跨模型工具链、abort/overflow/provider compat | [assistant-message-frame.test.ts](../../../../.repo/pi-1.2.0/packages/ai/test/assistant-message-frame.test.ts)、[cross-provider-handoff.test.ts](../../../../.repo/pi-1.2.0/packages/ai/test/cross-provider-handoff.test.ts) |

faux provider 的 queue 在 request start order 消费，可模拟 token/cache/delta/deferred，适合确定性编排测试；其估算成本与 cache 命中不是生产 benchmark，独立并发脚本应使用不同 provider handle。 [README faux 边界](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1517)。

## 全部文本的设计/规划时效核查

本包文本只有 README（1949 行）与 CHANGELOG（2310 行），均全文语义阅读；长历史条目部分以剥离重复链接目标的呈现阅读，未把文件枚举当阅读。没有独立 PLANNING/TODO/spec。重要方向及当前证据如下：

| 主张/动机 | 状态与剩余边界 |
|---|---|
| 从全局 API 到显式 Provider/Models、选择性 bundle | 已兑现，root/models/providers/all/compat 接线清楚；compat “未来删除”是未定期方向，未发现本版移除 |
| transcript 承载系统/工具历史，保持缓存 prefix | 已兑现，native 与 collapse 并存；不同 provider 不保证语义完全无损 |
| chat/image/classifier 合并目录、按操作区分 | 已兑现；旧 ImagesModels 是已删除历史，不能作为当前 API；图片分类仅部分 adapter 支持 |
| dynamic generation-checked publication 与取消 | 已兑现，custom provider/store 仍需遵守 signal/commit 合同；不是任意实现的强制事务回滚 |
| provider-owned auth、serialized rotated-token refresh | 已兑现，内存 store 不提供跨进程持久锁；CLI auth.json 只是 CLI 的存储选择 |
| deferred handle 与 fetch/cancel | 机制已实现、adapter 可选；没有自动 durable scheduler 或全真实 provider 支持承诺 |
| server strict prefer 降级、价格 tiers、Decisions | 本轮新增/修复已对应实现/测试，外部可用性未运行验证 |

需更正文档的具体局部主张：README 368 仍说 Radius overlay（当前 replace）；756 说非 vision 图片静默忽略（当前 placeholder）；1535–1537 说同 provider/API assistant 原样、跨 provider thinking 包 `<thinking>`（当前按完整 model identity，纯 text，无 tags）；1827 说 Azure Responses only（1.0.3 起 azure factory 同时提供 Chat Completions）；1089–1098 stop list 缺 deferred。它们不是未实现计划，源码/后续 CHANGELOG 已替代旧说明。README 1940 要写 Unreleased 是 contributor 指导，本快照实际从 1.2.0 开始的 release ledger 不表示漏做运行能力。 [README cross-provider](../../../../.repo/pi-1.2.0/packages/ai/README.md#L1528)、[Azure 变更](../../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md#L55)、[当前 Azure factory](../../../../.repo/pi-1.2.0/packages/ai/src/providers/azure.ts#L1)。

历史 CHANGELOG 中 SDK 重试默认启用/禁用、Mistral SDK 转换、旧 transport 默认 SSE、旧 Google CLI/Antigravity、旧全局 auth 都有后续替代，不能拼接成当前架构。价格、模型名与 provider upstream 可用性仅代表快照作者当时资料；本次没有联网确认，也不把历史 image-limit 测量或 Decisions 大输入阈值冒称新测试结果。

## 1.0.4 → 1.2.0 的语义变化与阅读建议

以旧有效 manifest 的 path/hash 比较定位变更，旧 source 目录未提供，因此没有完整旧新逐行 diff；当前报告从实际 source 重新证实，不机械继承旧正文。新增/修改主要落在 classifier shared/Decisions、OpenRouter pricing/personalization、model/compat schema、stream duration、Radius catalog replacement、Anthropic strict fallback、Mistral headers-only timeout、cost generation、OAuth login agentName/callback fallback、估算与 retry wording。相关 manifest/export/types/callers/tests 均需一起解释；如 OpenRouter helper 的导出不等于默认 Models 行为，classifier 可用性也不等于 agent 自动执行分类。

静态风险/证据缺口：ignored model data 缺失；SDK/upstream service 未执行验证；EventStream 无容量限制与 consumer 自动取消；原始 provider callbacks 可改变 payload 或让请求失败；catalog pricing/availability 是估计；deprecated/legacy 文档存在局部冲突；typed schema/compat 值不是应用权限或运行安全边界。阅读顺序：types/Models → auth resolve/stores → transcript → lazy/event stream → 所选 provider/API → 升级测试 → generator/ignored-data 校验。主链已解释，未逐行阅读所有模型包装、全部长 adapter、所有测试或外部库内部实现。
