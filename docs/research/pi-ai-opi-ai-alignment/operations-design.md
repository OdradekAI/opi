# 图片生成与分类：SDK 候选契约及归属比较

> **后续重审：** 用户已将本轮基准明确为“pi 的 Rust 实现”，opi-spec 仅作可能过时的参考。本页源码事实保留；涉及原 Q1–Q21 的目标建议和确认状态须先读 [决策重审](decision-reassessment.md)。旧规范不能继续单独否决 pi 已落地能力。

状态：**Candidate / 非规范性设计研究**。研究日期：2026-10-03。本文不修改现行规范、不授予 Agent Core 准入、不新增 crate/trait/config，不构成实施授权。

人类已确认：优先修正现有调用链；图片生成与 classification 纳入候选目标；第一批调用方是 **SDK / 嵌入式 Agent 的显式调用**，不默认新增 Reference Product 工具。初稿时未确认具体provider、类型、媒体、生命周期与归属；这些现在已由Q10/Q13/Q14/Q21回答，见下方访谈后状态。下文未采纳备选不构成新未决项或实施要求。

沿用已实际读取的 research 与 codebase-design 技能。证据来自本地 pi 和 opi 源码、测试原文及 `docs/opi-spec.md` §8.1；没有联网、安装、生成 catalog、执行 pi 测试或 live 调用。既有研究仍保持独立；本新中文专题没有既有双语 counterpart。正式候选 spec 若吸收内容，由主任务同步中英版本。

> **当前状态**：Q3/Q10/Q13保留SDK消费者、opi-ai归属及首批OpenRouter图片／TypeSafe与OpenRouter System One。Q27/Q28替代首轮Q14/Q21：image retry0/SystemOne2、attempt timeout＋可选total、有序blocks/first-choice、有限可覆盖预算及无sum/argmax默认拒绝。下文初轮方案仅保存推理历史；不再要求重选。当前契约见SDK target、decisions D-24/25及44 AIR；依赖链/timeout补证在SDK annex A–D。

## 结论先行

图片和分类适合共享 provider 身份、凭据解析、HTTP 安全边界、取消/错误分类、已知 usage 的会计规则；**不适合复用聊天消息、工具调用、thinking、文本 delta 或“最大输出 token”作为所有 operation 的必选契约**。pi 的三类 model/result 是这种区别的源码依据；Rust 的类型化请求与结果可以更直接表达差异。

图片在当前 pi 快照只有 **一个具体生成 adapter：OpenRouter**。legacy image registry、lazy wrapper 和两个模型名称都不是第二个 adapter。分类确有 TypeSafe/OpenRouter 共享 System One wire、Cloudflare System One envelope 和 llama.cpp label-logprob 实现。上游已有这些实现只是参考证据，不是 opi 已有两适配器或两消费者的证据。

近期结构建议优先比较“小的 operation catalog 判别 + 分开的 typed methods”与“Agent-neutral companion 的两个 typed client”；先明确行为和用户选择，再做 placement case。不要先把 chat `Provider` 改成任意 JSON operation 框架，再寻找消费者。

## 证据索引与覆盖

| ID | 直接证据 | 覆盖 / 限制 |
|---|---|---|
| <a id="OP01"></a>OP01 | [pi types:302–338](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L302)、[types:615–689](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L615)、[types:1096–1168](../../../.repo/pi-1.0.0/packages/ai/src/types.ts#L1096) | operation接口、模型身份、request/result；不证明runtime输入校验 |
| <a id="OP02"></a>OP02 | [Models.generateImages/classify](../../../.repo/pi-1.0.0/packages/ai/src/models.ts#L948)、[model-operations.ts](../../../.repo/pi-1.0.0/packages/ai/src/utils/model-operations.ts#L16)、[createProvider dispatch](../../../.repo/pi-1.0.0/packages/ai/src/models.ts#L1146) | model type与operation dispatch、auth、error-as-value |
| <a id="OP03"></a>OP03 | [openrouter-images.ts:42](../../../.repo/pi-1.0.0/packages/ai/src/api/openrouter-images.ts#L42)、[:138](../../../.repo/pi-1.0.0/packages/ai/src/api/openrouter-images.ts#L138)、[:167](../../../.repo/pi-1.0.0/packages/ai/src/api/openrouter-images.ts#L167) | 图片完整adapter；只此一个具体生成wire |
| <a id="OP04"></a>OP04 | [system-one-shared.ts:63](../../../.repo/pi-1.0.0/packages/ai/src/api/system-one-shared.ts#L63)、[:131](../../../.repo/pi-1.0.0/packages/ai/src/api/system-one-shared.ts#L131)、[:175](../../../.repo/pi-1.0.0/packages/ai/src/api/system-one-shared.ts#L175) | 分类parser、usage、HTTP与retry；不验证所有语义范围 |
| <a id="OP05"></a>OP05 | [typesafe-system-one.ts:8](../../../.repo/pi-1.0.0/packages/ai/src/api/typesafe-system-one.ts#L8)、[cloudflare-workers-ai-system-one.ts:23](../../../.repo/pi-1.0.0/packages/ai/src/api/cloudflare-workers-ai-system-one.ts#L23)、[cloudflare-stream.ts:7](../../../.repo/pi-1.0.0/packages/ai/src/providers/cloudflare-stream.ts#L7) | 两种真实transport envelope；共享parser不等于相同部署/auth |
| <a id="OP06"></a>OP06 | [llama-cpp-classify.ts:16](../../../.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.ts#L16)、[:104](../../../.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.ts#L104)、[:180](../../../.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.ts#L180)、[:227](../../../.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.ts#L227)、[:393](../../../.repo/pi-1.0.0/packages/ai/src/api/llama-cpp-classify.ts#L393) | 不依赖自由生成JSON的分类、多个HTTP子请求、校验与缓存 |
| <a id="OP07"></a>OP07 | [codemode input checks](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/codemode/execute.ts#L119)、[显式model调用](../../../.repo/pi-1.0.0/packages/coding-agent/src/extensions/codemode/execute.ts#L612) | pi真实产品入口的校验；SDK直调不经过这里 |
| <a id="OP08"></a>OP08 | [opi auth](../../../crates/opi-ai/src/auth.rs#L1)、[ModelInfo](../../../crates/opi-ai/src/model_info.rs#L474)、[ModelInfo.validate](../../../crates/opi-ai/src/model_info.rs#L538)、[Usage](../../../crates/opi-ai/src/stream.rs#L40)、[ProviderError](../../../crates/opi-ai/src/provider.rs#L479)、[PreparedProviderCall](../../../crates/opi-ai/src/provider_collection.rs#L756) | 可复用与chat专属现有机制；没有现成通用operation call |
| <a id="OP09"></a>OP09 | [openrouter image tests](../../../.repo/pi-1.0.0/packages/ai/test/openrouter-images.test.ts#L67)、[System One tests](../../../.repo/pi-1.0.0/packages/ai/test/typesafe-system-one.test.ts#L50)、[Cloudflare tests](../../../.repo/pi-1.0.0/packages/ai/test/cloudflare-workers-ai-system-one.test.ts#L59)、[llama tests](../../../.repo/pi-1.0.0/packages/ai/test/llama-cpp-classify.test.ts#L116) | 全部离线设计/预期；本次未运行 |
| <a id="OP10"></a>OP10 | [spec §8.1](../../opi-spec.md#81-agent-core-admission) | 所有admission gates；pi parity不能豁免 |

模型目录完整性限制：`providers/openrouter.models.ts` 引用 `./data/openrouter.json`，但本次读取的快照不存在 `providers/data/`。不执行hydrate。因此无法由这份目录宣称当前完整可用图片模型名单；测试中的模型是明确的fixture，不当作已核验真实catalog或live capability。模型身份的结构与factory接线仍可直接读源码。

## 图片：实际能力、差异和弱契约

### 模型与请求

OP01 的 `ImageModel` 有 `type=image`、provider/id/api/baseUrl、input modalities、output modalities与token价格；不要求chat的contextWindow、maxTokens、reasoning。输入是有序text/image blocks，image由base64+MIME组成。没有公开的尺寸、数量、quality、seed、aspect ratio或透明背景类型字段；`ImagesOptions.metadata`仅是通用声明，OP03实际buildParams不读取它。可通过onPayload任意改wire不等于这些参数已经成为稳定公共能力。

OP02 在 Models 层检查type=image、provider存在、generateImages方法存在，随后复用applyAuth。具体factory再按api分发。该检查不会验证输入blocks、base64、MIME、model.input相容性或所有目录limits；这些责任不能因TypeScript类型存在而略过。

### 两种图片模型路径与“第二adapter”缺口

| 已有证据 | 真正不同 | 不能据此声称 |
|---|---|---|
| fixture `google/gemini-3.1-flash-image-preview`，`openrouter-images.test.ts:67` | output声明text+image，因此wire modalities=[image,text]；返回text及image | 已验证Google原生adapter；真实模型能力已live确认 |
| fixture `black-forest-labs/flux.2-pro`，同文件`:100,124` | output声明image，因此wire modalities=[image]；用于pre-abort与最终结果测试 | 第二个图片供应商adapter或两adapter conformance |
| `providers/images/register-builtins.ts:37–56` | legacy global registry包装同一个 `api/openrouter-images.ts` | 独立实现或未来应保留双入口 |

在当前源码范围，要求“图片至少两真实provider/adapters差异”**无法满足**。这是明确证据缺口；应等待第二具体adapter被人类选为目标并完成官方协议调查，而不是虚构另一adapter或把OpenRouter底层模型商当作本库独立wire实现。该缺口影响real-seam论证和跨adapter验收，不妨碍设计一个窄的单adapter Candidate。

### 输出和失败

OP03 一次nonstream `POST chat/completions`，把全部输入映射为一个user message，stream=false。只读取 `choices[0]`，先加text，再遍历images。仅接受匹配 `data:<mime>;base64,<payload>` 的值；HTTP(S) URL、缺URL、未匹配data URL被跳过，不下载、不写文件、不返回remote URL。它不验证base64是否能解码、MIME是否真实、尺寸/像素上限或decoded size。

结果初始stopReason=stop。没有choice、只有文字、所有image条目被跳过都可能返回成功且没有图片。`ImageModel.output`声明“始终包含image”不保证一次响应确实有image。对此不能机械对齐；SDK用户要求图片时，空媒体成功很容易被误判成完成。对于明确的provider拒绝/内容过滤，当前adapter也没有独立的refusal类型映射。

catch把error转result.stopReason=error或aborted，并保留已写入output的字段。Models外层发生setup错误则用empty output helper。类型注释“never rejects”由实现catch支持常规failure，但外部自定义provider仍可能throw，Models捕获后损失其未返回的partial metadata。Rust可以用typed `Result`，但failure必须能携带已经报告的usage/response id；否则仅迁移语法会丢已付费失败的证据。

### 取消、超时、重试和usage

OP03 将signal与timeout传SDK、关闭SDK retry并使用通用helper；不显式传maxRetries时helper为0。错误时若caller signal.aborted标aborted，否则error；不能把这视为独立typed Timeout。没有独立逻辑call total deadline；其精确SDK默认timeout未在本次读取第三方locked源码核验。

没有检查success返回前signal是否刚被取消，所以取消与已收到success的竞争不是由本adapter显式规定。取消HTTP不证明服务器终止生成、退款或不会返回billable usage。这些必须由新operation契约说清。

usage整个对象缺失时结果usage缺省；有usage时字段缺值用0。cache-write从reported cached中扣除，再从prompt扣cache buckets；费用直接使用model.cost四个base rate，并未调用通用calculateCost，所以不应用ModelCost.tiers。仅有token价格，不能表达按张数、分辨率或质量计费；无价格或无token时不应伪造0成本。图片真实计费模型未核验，不纳入已确认估价范围。

## 分类：三个实现族及可共享语义

### 请求与答案不是“任意JSON生成”

OP01 输入是一个JSON object `state`及question-id→question map。每题有instructions和criteria，类型为choice、score、bool。输出保留question ids并对应typed answers：choice为chosen key+probability map+confidence；score为number+confidence；bool为true的probability。没有threshold、业务action、自动选路或执行授权。SDK调用者应决定如何将probability变成业务判断；分类输出不能批准工具。

`state`是被判断的数据，`instructions/criteria`是题目描述，不能把两者合并成一个随意prompt而丢失角色含义。llama实现的system prompt显式说state中的指令不能被执行；这是提示意图，不是prompt injection防护证明。

### 真实adapter比较

| 合约点 | TypeSafe / OpenRouter System One | Cloudflare System One | llama.cpp |
|---|---|---|---|
| wire | baseUrl/systemone；body={model,state,questions} | account-scoped ai/run；body={model,input:{state,questions}} | /tokenize、/apply-template、/completion多个请求 |
| auth/route | Bearer key；TypeSafe与OpenRouter不同provider/baseUrl，同一adapter | Bearer token+解析account endpoint；provider wrapper替换placeholder | 可无key本地server；有key则Bearer；baseUrl剥/v1 |
| operation状态 | 预期一次body={answers,usage} | response.success与result.state=Completed且nested result；Queued视error，无polling | 串行各question；每题可能多次readout升级 |
| bool编码 | 公共bool↔wire noul | 同System One；外层envelope不同 | 读取Yes/No label token概率 |
| temperature | 忽略option；测试明确断言不发送 | 共享实现，同样忽略 | finite positive，除logprob后softmax，默认1 |
| score含义 | 直接接收server score；本地未验证范围/算法 | 同一shared parser | ordered criteria level index的期望值∑index·p，可非整数 |
| choice confidence | 服务端返回finite值，未验证数学定义 | 同一shared parser | `(n*max(p)-1)/(n-1)`并clamp[0,1] |
| usage | 有token字段时采集，failure后仍可保留 | nested result usage同样解析 | 不返回usage，即使多个HTTP请求发生 |
| 默认HTTP重试 | 每HTTP请求maxRetries=2 | 同shared helper | 每个tokenize/template/readout HTTP最多2重试 |

OP05 两transport共享 OP04 answer转换；TypeSafe和OpenRouter还有同wire两个provider的测试。它们提供“共享conformance值得存在”的上游证据；不是两个不同分类算法相同数值校准的证明，更不是opi gate已经通过。llama分类是label概率算法，不是把模型生成的choice字符串再转JSON。

### System One校验边界

OP04 `parseAnswers`要求每个请求question有object答案、类型匹配、数字finite。它不检查：choice是否属于criteria、probability keys是否恰等于labels、每个概率是否[0,1]、sum≈1、confidence是否[0,1]、choice是否argmax、score是否位于criteria索引范围。未知额外答案被忽略。SDK直调也没有codemode的非空questions/type/criteria形状检查；codemode check只是产品边界，不能承担SDK库全部validation。

因此Rust候选不能仅以“服务端返回JSON结构正确”为成功条件。建议以typed request constructors和统一result validation实现这些必要不变量；choice=argmax是否强制、tie如何稳定处理、score解释及阈值交给调用者仍需明确（下文Q-OP2）。输入invalid应在解析auth/network之前失败；不从chat tool schema校验偷渡coercion。

usage在parseAnswers之前赋值；malformed answer的错误result仍保留已报告token counts，这一行为值得保留。负数、非数、非finite token count变0，缺整个usage或无token字段时None；不应照抄“坏usage字段等于已知0”的处理。异常usage可独立标为unknown/invalid，不必抹去有效分类结果，也不能冒充准确会计。

### llama.cpp的必要差异与不宜公开的偶然

OP06 每题要求单token labels且无重复token：choice 2–62选项、score 2–10等级、bool Yes/No；在首次HTTP之前校验问题数量范围与temperature。tokens和template可并发，题与题串行。起始n_probs=max(256,16*labels)，缺label尝试4096和32768；最终仍缺任何label则error，**不把缺失label概率编成0**。所有label低于underflow门槛也error。保留这一failure原则比复制具体readout深度数字更重要。

同分choice取最早criteria顺序；score是期望值不是argmax等级。若后题失败，先前answers全部丢弃为{}。操作本身没有partial success result；不能不经讨论让Rust返回已完成子集然后声称完全成功。

module-global labelTokenCache按server-root/model/label缓存Promise，失败evict；未看到容量、TTL、显式资源所有者或模型权重变更失效。并发调用共享pending tokenization，而该pending绑定首个request.signal；在首调用取消、次调用共享pending的条件下可能共同失败。此为源码推导风险，未重现；不应照抄为SDK公共缓存契约。

onPayload/onResponse仅观察completion readouts，tokenize/template被observe=false隐藏。每次HTTP attempt新建timeout；多题+readout升级+重试的总时间可能远大于timeoutMs。未来SDK契约应区别logical operation、provider attempt和内部subrequest，不要求把全部模型操作伪装成chat turn。

## Rust可以复用哪些现有契约

| 现有opi module | 可复用行为 | 必须先解决的边界 |
|---|---|---|
| `AuthResolver` / `ResolvedAuth` | 每logical call解析一次、secret保密、provenance、account/base-url | 现resolver返回ProviderError；新operation若在companion不能反向依赖产品；跨attempt使用冻结凭据可保留，但no-auth local route不能用假secret表达未定义契约 |
| `HttpClient` / proxy / bounded error body | 已有连接池、代理设置、错误body限制与脱敏工具 | 不要假设bounded error reader等于成功图片body也bounded；新增媒体download属于I/O授权和limits决定 |
| `ProviderError` taxonomy | Cancelled、Timeout、RateLimited、auth、network、unsupported区分 | 目前MissingWireRoute带chat WireApi；新operation input/result validation与failure metadata不能强塞StreamError；不能仅为复用扩张全部chat enum |
| `Usage` / pricing functions | unknown-vs-reported、reasoning/cache child不重复计数、已知token成本 | input/output目前u32；images可能无tokens/按图片收费；不得伪造tokens或套用token价格；更广会计单位待实际provider证明 |
| `ModelInfo` / registry | provider+model路由思想、价格与兼容性验证原则 | `ModelInfo.validate`强制context_window和max_output_tokens>0；chat WireApi/WireCompat/thinking是必需字段，不能给image填虚构窗口 |
| `PreparedProviderCall` | route/request/auth冻结、single active attempt、credential terminal | 持有chat Request与EventStream；不是现成通用one-shot调用。可先复用设计不变量，不复制另一个重叠prepared-call框架 |
| `Message/InputContent` | text与图片输入可有共享值表示 | 不把图片输出塞assistant text或toolResult来绕过operation类型；Rust拥有bytes不必对外强制base64字符串 |

共享是一条行为和所有权论证，不能仅因字段重合新建public trait。若两个operation先在不同placement验证，有共享conformance后再评估最小提取；禁止新旧同职责入口无限并存。

## 三个 Candidate 结构方案

下表只比较结构，没有选择crate归属；所有方案都面向已确认的SDK/embedded显式调用，不默认创建终端工具。

| 方案 | 公共interface形状 | 得失与进入条件 |
|---|---|---|
| A. 受控operation catalog + typed methods | model catalog用闭合Chat/Image/Classifier判别；分别chat stream、generate_images、classify；各自请求与结果；内部按实际adapter分发，不公开任意JSON execute | 适合共享身份/auth/目录、生命周期已有耦合证据；最多让catalog公共，不把结果合并成全optional struct。若放入Agent Core，必须逐gate通过；当前semantic necessity/coupled lifecycle/两opi适配器证据不足 |
| B. Agent-neutral companion 的typed clients | ImageClient/ClassifierClient作为独立操作，显式注入auth与transport；不要求Agent、tools、session；embedding应用组合 | 保留较小chat接口与独立发布，适合初期尚无core invariant需要。新crate本身仍需真实interface价值，不能因“叫companion”免去维护/依赖评审；会产生如何共享错误/auth版本的实际成本 |
| C. SDK中的具体可选adapter模块先验证 | 先公开目标provider最小typed方法与结果，共用现有安全/HTTP机制；不承诺统一所有operation/provider，后续第二adapter以同一conformance评估合并 | 最小I/O/依赖增量，适合单image adapter；多provider SDK目标较快增长时可能暴露不同调用方式，需要明确替代出口，不能再叠一层同职责永久facade |

研究者当前偏好：**先把A的typed contract形状写成placement-neutral Candidate，并用B/C比较实际首次交付位置**。这一偏好不等于选择“进入opi-ai Core”，也不等于授权新crate。用户若选择首批只有OpenRouter图片+一个System One classifier，不能为未知未来提前建立plugin-style operation registry或支持自定义字符串operation的trait。

### §8.1 gate检查（截至本文）

| Gate | 当前证据 | 状态 |
|---|---|---|
| Semantic necessity | 没有证据表明移除images/classifier会破坏当前chat或Agent durable invariant | 未满足Core准入 |
| Product neutrality | 用户已选SDK/embedded显式调用；typed operation可以不含终端和用户policy | 有可行设计，待目标审阅 |
| Depth/locality | protocol解析、结果validation、取消/会计可藏于小interface | 有设计依据，无opi多caller证据 |
| Real seam | pi classifier有多个adapter/sharedparser；pi image仅一个；opi新operation尚无adapter | 待验证，不能用上游计作opi消费者 |
| Coupled lifecycle | 可以独立client；尚未证明独立version会更脆弱 | 未满足Core准入 |
| Mechanical verification | 本文提供验收设计；现有pi测试未执行，新opi测试尚未存在 | 未满足 |
| Minimal authority | 目标provider与deps未选；不得默认添加I/O、SDK/native依赖、工具或网络download | 待审；user scope确认不自动满足 |

[GATE-001/002](../../opi-spec.md#81-agent-core-admission) 明确失败的capability保留在Reference Product、Extension Ecosystem或Independent Companion，pi parity/feature flags不能豁免。故本文保持Candidate与placement未决。如拟修改该规范，必须具体指出要修订哪个gate及影响，另请人类确认。

## 候选目标契约：哪些稳定，哪些必须先决定

以下是给spec作者的可执行契约候选，不是已批准规范性要求。采用时必须关联主决策ID、owner和对应验收ID。

| 候选ID | 触发/行为及失败 | 责任候选 | 依据 / 验收 |
|---|---|---|---|
| <a id="OC-01"></a>OC-01 | 明确operation+provider+model后查类型；image不能误入chat，classifier与chat可同provider/model-id共存；未知operation/route无I/O失败 | SDK model lookup与operation入口 | OP01/02/08；OV-01 |
| <a id="OC-02"></a>OC-02 | 首批为nonstream、一个await返回typed terminal结果；不伪造chat start/delta/done。成功、timeout、cancel、auth、malformed-output可区分 | operation runtime/adapter | OP03–06；OV-02/03 |
| <a id="OC-03"></a>OC-03 | logical call开始前校验请求并解析一次auth；确定route/request/凭据。任何retry不能静默切provider/model/account | 调用准备层，具体placement待定 | opi prepared-call原则；OV-03 |
| <a id="OC-04"></a>OC-04 | 图片由SDK返回拥有的媒体值；文件写入/命名/持久化由调用者显式决定。URL与inline bytes支持集待Q-OP1，不能暗中联网下载URL | image SDK / embedder | OP03；OV-04 |
| <a id="OC-05"></a>OC-05 | 只有满足已选success条件的媒体响应才成功；零image、损坏base64、unsupported MIME、超limits不得静默丢弃后成功；是否允许text-only拒绝待Q-OP1 | image result validator | OP03弱契约；OV-04/05 |
| <a id="OC-06"></a>OC-06 | 分类按原question ID给typed答案；unknown/missing labels、非finite/out-of-range概率、超score范围被明确拒绝。概率总和容差与tie规则随Q-OP2固定 | classifier result validator | OP04/06；OV-06 |
| <a id="OC-07"></a>OC-07 | 不用缺失label=0替代未观测结果；confidence不被称作跨model已校准可靠度；阈值/行动policy由调用者持有 | classifier adapter / embedder | OP06；OV-06 |
| <a id="OC-08"></a>OC-08 | 分类整体原子成功或failure；是否另行公开partial answers待Q-OP2。即使最终输出invalid也保留已报告usage及来源；未知usage/cost保持unknown | operation outcome | OP04、OP08；OV-07 |
| <a id="OC-09"></a>OC-09 | 取消停止等候和后续子请求/重试；资源释放与期限范围明确；不宣称服务端已撤回/未收费。deadline是整个operation还是per-attempt由Q-OP3固定 | runtime与transport | OP03–06；OV-02/03 |
| <a id="OC-10"></a>OC-10 | 不从目录字段或方法存在推断已交付operation；model capability只宣告实际adapter验证的输入/输出/options；不支持的请求设置明确拒绝或按已确认规则忽略 | operation metadata/validation | OP01、temperature差异；OV-01/06 |

除placement/provider选择外，可根据研究直接推荐：typed输出；nonstream优先；inline媒体校验；请求前输入验证；错误携带已知usage；不默认自动retry（尤其生成图片可能收费）；不隐藏HTTP子请求；不公开业务threshold。推荐仍需主决策文档记录确认或保留Candidate。

## 首轮关键决策问题（已回答，部分被替代）

Q3首批调用方已确认，**不再询问**。以下是初轮问题记录；Q13、Q27、Q28已经收敛其范围，不作为当前访谈前沿。

1. **Q-OP1 图片结果与首批provider**：首批是否选择OpenRouter这个已有一手adapter基线，并以“prompt+reference images → 至少一个经过校验的owned image bytes，可附text”为完成？推荐SDK返回bytes+MIME，由embedder保存；remote URL先拒绝为unsupported而不自动下载。主要代价：暂不支持URL-only provider、尺寸/quality等未建模参数。若要URL-only/文件直接输出，需要额外network/file authority与生命周期决策。第二image provider尚未选择，不为满足数量虚构。
2. **Q-OP2 classification的业务语义与首批adapter**：需要choice/bool/ordered-score三者，还是先choice+bool？建议保持pi的typed问题，而非任意JSON；score明确为有序criteria索引域中的实数，不能把llama期望分数和server未知算法承诺为相同校准。threshold/abstain由caller；整体失败时不公开可误用partial答案。TypeSafe/OpenRouter共同System One实现是较窄起点；若选llama则必须承担tokenizer/cache/readout多请求生命周期，不能作为“免费等价fallback”。
3. **Q-OP3生命周期与placement**：建议nonstream、显式operation总期限与取消、默认不自动重试；failure保留已报告usage，已收到完整valid success与cancel竞争的优先级写清。画像生成与多请求classifier的重试是否允许不同策略，以及先B/C placement验证还是申请A进入Core，需要人类确认。仅凭Q1“纳入目标”不能替代placement case。

可由implementation设计解决但必须在最终spec固定：实际bytes/像素/request-size限额、概率sum容差、稳定label/tie顺序、结果大小上限、invalid usage表示、deadline时钟、最大子请求并发。不得留给执行Agent临场猜测，也不为每项新增用户配置键。

## 离线验收设计（均未执行）

| ID | 输入/边界 | 可机械断言 | 不能证明 |
|---|---|---|---|
| <a id="OV-01"></a>OV-01 | 三operation身份、同provider/id不同type、unsupported options、unknown route | wrong-type/no-route/invalid input在mock transport计数0时失败；chat现有lookup不被image污染 | provider真实可用性 |
| <a id="OV-02"></a>OV-02 | fake response pending、timeout、pre/mid cancel、transport drop | terminal种类互斥；cancel后不发下一子请求/重试；resources归零；outcome按固定race规则 | 远端取消/退款 |
| <a id="OV-03"></a>OV-03 | 两允许attempt、auth resolver计数、retry-after、classifier多subrequest | auth只一次、route/request不漂移、总deadline不被每retry刷新绕过、按D-24区分image0与SystemOne2；每种retry记录身份 | 公共网络延迟表现 |
| <a id="OV-04"></a>OV-04 | data URL、HTTP URL、bad base64、MIME不符、空choices、text-only、多个images、多choices | 选定媒体策略确定且有界；不暗中download/write；坏entry不静默变成功；保留顺序规则 | 内容质量/图像含义 |
| <a id="OV-05"></a>OV-05 | encoded/decoded bytes限额、巨大尺寸/压缩图、超数量、unsupported reference modality | 预网络输入失败与有限读取；结果超限停止读取并释放；不把错误body/凭据写日志 | 所有解码器安全性 |
| <a id="OV-06"></a>OV-06 | System One native/Cloudflare envelope/llama标签fixture、missing/extra答案、NaN/负/大于1概率、未知choice、错误sum、tie | 按ID/labels/type验证；预定score范围、tie/temperature/unsupported-option规则；不靠归一化掩盖坏值 | 不同classifier校准一致/真实准确率 |
| <a id="OV-07"></a>OV-07 | valid usage+malformed answers、missing usage、reported zero、invalid count、token价格未知 | outcome失败仍保留已知usage；missing≠zero；无报价≠0成本；不用image count伪造tokens | 供应商账单一致 |
| <a id="OV-08"></a>OV-08 | 现有chat SDK compile/conformance、无新默认tool/dependency/网络启动 | typed operation加入不改变chat请求/工具authority；静态清查无chat包装假事件；仅任务owned callers迁移 | Core admission自动通过 |

差分只比较同一语义输入、所选答案字段与operation outcome；归一化timestamp/request-id，不归一化错误类别、missing-vs-zero、label集合、概率范围、输出媒体数量、timeout和cancel。System One两个transport可共享wire-independent fixtures；llama softmax用确定logprobs验算，但不得用概率强制等于独立System One服务。图片目前只有一个adapter，不能声称跨adapter差分完成。

## 实际执行与交接

本专题实际执行为 `Get-Content`/`rg` 源码与测试读取、catalog路径存在性检查和新增本文；未执行pi或opi运行时测试，未联网。原笔记 `pi-contracts-notes.md` 未修改。测试影响：`none`。

范围边界：第二image adapter不在首批；generated catalog原始数据、live限制/计费未验证。首批协议和锁定依赖已补核；新Rust实现、离线conformance及正式文档协调留给后续获准实施。原Q-OP1/2/3已由访谈及重审收敛，见D-13/24/25。本文不能在这些缺口未解决时标为ready-for-agent。主任务负责将获确认的部分纳入中英候选spec并运行文档检查。
