# 历史重放契约专项研究

> **第二轮目标已同步：** 按[D-19/20/22/28](decisions.md#第二轮目标决策d-18之后)更新replay与参数目标；R01–R08仍是基线源码事实。首轮Q16/修订Q17的Length统一文本投影、未知缺result拒绝已被替代。非规范性研究，不是实施准入。

状态：非规范性研究及候选设计；不是实施准入。2026-10-03 对本地源码作静态阅读，未执行本页列出的测试，未访问 live provider。本页事实以本次研究基线为准，用户确认统一见 [decisions.md](decisions.md)。只增加本研究目录文档，不改变代码、fixture、既有测试或正式规范。本页没有既有英文 counterpart；最终候选 spec 的双语同步由主文档承担。

范围：opi-ai 全部现有 chat wire、profile/委托关系、opi-agent 的生产循环、Reference Product 默认消息转换；pi 的已落地replay意图作为对齐目标，区分Rust表达、存量缺口与已知错误。图片生成和 classification 的非 transcript 操作不在本页范围。

## 核心结论

- opi 已有 Anthropic signed/opaque thinking 的同路由 roundtrip；其余 wire 没有统一 replay 语义。真实差异包括忽略 thinking、转 text、坏 tool arguments 变 `{}`、变 `null` 或原样发送。不是命名差异。
- tool recovery 是真实消费者：模型产生坏 JSON 或 Length 时，Core 保存 proposal 和明确未执行的 error result 并继续。直接对所有历史坏 JSON fail-closed 会截断已有恢复路径；把它变 `{}` 则伪造历史。需要可验证的失败上下文投影。
- 同一 tool batch 中的重复 ID 会造成关联歧义；不同已闭合 batch 重用字符串不能一概视为损坏。Gemini 当前每个 response 都从 `fc_0` 开始。
- replay 是从原始历史到请求的投影，不能改写 canonical history、执行工具、授予权限、假装缺失结果意味着“没有副作用”。同 provider + wire + requested model 是最小 continuation identity；`ApiKind` 和 reported model 不能代替它。

## 一手证据

<a id="R01"></a>

### R01 — 身份字段与继续调用资格

[AssistantMessage](../../../crates/opi-ai/src/message.rs#L28) 保存 `api/provider/model/response_model`；[ApiKind](../../../crates/opi-ai/src/lib.rs#L64) 只有 Anthropic/OpenAi/Google/Mistral，而 [WireApi](../../../crates/opi-ai/src/model_info.rs#L81) 区分八种 wire。`ApiKind::OpenAi` 不能区分 Completions/Responses/Codex；Bedrock 构造器也用 `ApiKind::Anthropic`（[bedrock/mod.rs:930](../../../crates/opi-ai/src/bedrock/mod.rs#L930)）。不能仅凭这枚 enum 批准 opaque continuation。

Anthropic [初始化:1045](../../../crates/opi-ai/src/anthropic.rs#L1045) 将 request model 写入 `model`，把未报告的 actual model 表达为 `response_model=Some("")`；[mapper:440](../../../crates/opi-ai/src/anthropic.rs#L440) 保持二者区分。[roundtrip test:119](../../../crates/opi-ai/tests/anthropic_thinking_roundtrip.rs#L119) 明确断言 requested alias 用于 replay identity。

相比之下，Chat [update_metadata:442](../../../crates/opi-ai/src/openai_chat.rs#L442) 与 Responses [Created:419](../../../crates/opi-ai/src/openai_responses_shared.rs#L419) 把供应商 reported model 写入 `model`；Gemini [empty_assistant_message:427](../../../crates/opi-ai/src/gemini.rs#L427) 和 Bedrock 同构函数的 model 初值为空，本次已读 mapper 不填 requested identity。由此只能判断这些历史现有数据不保证足够的 replay provenance，不能推断过去 actual model 相同即 continuation 合法。置信度：高；旧持久化数据中各 profile 的实际分布未采样。

<a id="R02"></a>

### R02 — 所有 adapter 的当前投影矩阵

下表是代码行为，不是对各远端最新协议的保证。所有所列 serializer 均未检查 `AssistantMessage.stop_reason`，也没有跨历史的 call/result 配对校验。

| wire / 实际复用者 | thinking / opaque | 历史 ToolCall arguments | ID 与混合内容 | 定位 |
|---|---|---|---|---|
| Anthropic Messages；custom mapped Anthropic route | `api==Anthropic && provider==target && model==requested` 时 signed thinking、redacted 原样；foreign 且 signature=Some 时可读 thinking→text，空块删；foreign unsigned 仍发 thinking；foreign redacted 删 | JSON object 保留；语法错或非object→`{}` | call/result ID 原样；保留 Text/ToolCall/Thinking 次序；空 assistant 删 | [anthropic.rs:1265–1339](../../../crates/opi-ai/src/anthropic.rs#L1265) |
| OpenAI Completions；Mistral、OpenRouter、Azure Chat、custom mapped Chat | thinking 与 redacted 全部丢弃，同路由也如此 | JSON object 重新编码为字符串；语法错或非object→`"{}"` | ID 原样；存在任何 tool call 就令 content=null，丢掉同 assistant 的 Text | [openai_chat.rs:1373–1441](../../../crates/opi-ai/src/openai_chat.rs#L1373) |
| OpenAI Responses / Codex Responses；custom mapped Responses | thinking 与 redacted 全部丢弃 | 原始字符串，不解析 | call `id` 和 `call_id` 都设为同一公共 ID；先合并所有Text输出，再输出所有calls，改变交错块顺序 | [openai_responses_shared.rs:745–789](../../../crates/opi-ai/src/openai_responses_shared.rs#L745) |
| Gemini / Vertex | thinking 与 redacted 全部丢弃；空 assistant 删 | parse 成任意JSON value；失败→null | functionCall 不发ID；functionResponse发公共ID；Text/call相对次序保留 | [gemini.rs:563–625](../../../crates/opi-ai/src/gemini.rs#L563) |
| Bedrock Converse | 所有可读thinking→普通text，忽略signature；redacted删，空assistant删 | JSON object；语法错或非object→`{}` | toolUseId 与result原样；Text/ToolCall/readable thinking次序保留 | [bedrock/mod.rs:982–1038](../../../crates/opi-ai/src/bedrock/mod.rs#L982) |

复用证据：[Mistral profile:22](../../../crates/opi-ai/src/mistral.rs#L22)、[OpenRouter profile:23](../../../crates/opi-ai/src/openrouter.rs#L23)、[Azure build:169](../../../crates/opi-ai/src/azure_openai.rs#L169)、[Vertex build:117](../../../crates/opi-ai/src/vertex.rs#L117)、[ApiMapped dispatch:200](../../../crates/opi-ai/src/api_mapped.rs#L200)。这些不是新增五套独立 serializer。OpenAI/Codex 共用 Responses serializer，不能因公共前缀相同就视为与 Completions 等价。

反证：各 adapter 已有明确 error-result 编码（Anthropic `is_error`、Chat/Responses marker、Gemini `error`、Bedrock status）；因此不是完全没有恢复语义。问题是 proposal 及关联的转换不一致。置信度：高；是否被特定 provider 接受需 adapter fixture/官方协议核验，不能由静态 payload 宣称 live 通过。

<a id="R03"></a>

### R03 — 不完整的 continuation 数据不能靠新 serializer 恢复

当前 [AssistantContent](../../../crates/opi-ai/src/message.rs#L109) 只有 thinking 的 signature 和 redacted data，Text、[ToolCall](../../../crates/opi-ai/src/message.rs#L140) 无 signature/opaque extension。Gemini [Part enum:79](../../../crates/opi-ai/src/gemini.rs#L79) 只解析 Text/FunctionCall，无 thought/signature；其 mapper [330](../../../crates/opi-ai/src/gemini.rs#L330) 生成 `fc_{response内序号}`。Responses [OutputItemAdded:428](../../../crates/opi-ai/src/openai_responses_shared.rs#L428) 只接收 function_call，公共ID优先用call_id，其次item_id，不同时保留二者；后续 serializer 却把同一字符串写入两个字段。

目标若纳入这些 wire 的 continuation parity，必须一起修复 parser→typed history→storage→projection 链，不能只加 serializer 字段；已丢失的老历史不能合成签名。pi 中 [google-shared:230–274](../../../.repo/pi-1.0.0/packages/ai/src/api/google-shared.ts#L230) 同时支持 text/thinking/toolCall signature，显示该需求与内容块绑定；不代表 opi 已实现。置信度：高。

<a id="R04"></a>

### R04 — 路由 preflight 不验证 replay graph

[prepare_call:421–431](../../../crates/opi-ai/src/provider_collection.rs#L421) 确认 canonical target route，[:499](../../../crates/opi-ai/src/provider_collection.rs#L499) 调 capability validator，随后才解析 auth。[validate_request_for_model:296](../../../crates/opi-ai/src/provider.rs#L296) 管 model/capability/thinking，不校验历史 orphan、duplicate call/result、missing result 或 arguments。现有 serializer 直接遍历 Messages；本次定向检索未找到统一 replay normalizer。结论限定为已检查的八种内建 wire 与 mapped/profile 路径；不是声称不存在所有外部 extension 自定义处理。

缺失的不是“全历史ID唯一”检查：Gemini 每轮重用 `fc_0` 是已证实反例。关联需要考虑 assistant entry / batch，不允许后续一个同字符串覆盖先前关联。置信度：高。

<a id="R05"></a>

### R05 — 默认真实消费者保留原始失败信息

Core [agent_loop:609–684](../../../crates/opi-agent/src/agent_loop.rs#L609) 把完整 Done 或 in-band Error payload 放入 context；in-band Error不执行tools、不重试并使run失败，但保留 partial assistant。默认 Product [agent_messages_to_llm:4568–4595](../../../crates/opi-coding-agent/src/harness.rs#L4568) 直接clone所有Llm，不筛Error/Aborted；coding/interactive hooks [:4601](../../../crates/opi-coding-agent/src/harness.rs#L4601)、noninteractive [runner:1024](../../../crates/opi-coding-agent/src/runner.rs#L1024) 共用它。自定义 `AgentHooks::convert_to_llm` 可改变投影，但不能当成默认保证。

中途 token cancellation 是另一条路径：[agent_loop:588–598](../../../crates/opi-agent/src/agent_loop.rs#L588) 立即结束，没有把尚未terminal的stream partial加入context。不能把“in-band Error保留”推广成“所有cancel都保留Aborted消息”。外部SDK仍能提供具有Aborted stop_reason的历史，adapter目前不筛它。

<a id="R06"></a>

### R06 — Length / malformed 是正常恢复场景，不能用 `{}` 掩盖

Core [Length branch:714–758](../../../crates/opi-agent/src/agent_loop.rs#L714) 不执行整个 response 中任一 tool call，包括看似完整的早期call；为每个call产生“达到输出限制、未执行、请重发”的error result。[complete_tool_call:2176–2204](../../../crates/opi-agent/src/agent_loop.rs#L2176) 保存原始ID关联的result。工具arguments [parse:1767](../../../crates/opi-agent/src/agent_loop.rs#L1767) 解析失败后走 [malformed result:1782](../../../crates/opi-agent/src/agent_loop.rs#L1782)，不会把 `{}` 当实际可执行参数。

测试证据（均未运行）：[malformed_tool_arguments:719–817](../../../crates/opi-agent/tests/tool_validation.rs#L719) 明确预期连续两次provider call、零工具执行、持久化error result；[Length batch:1168–1267](../../../crates/opi-agent/tests/tool_validation.rs#L1168) 预期三次provider call，只执行后续fresh call，前一Length batch零授权/执行。这证明现有Mock测试设计了恢复，不证明真实wire重放坏JSON正确。Anthropic单元测试 [1523、1539](../../../crates/opi-ai/src/anthropic.rs#L1523) 反而明确固化坏JSON/非object变空对象；它与“准确复述被拒绝proposal”不等价。

<a id="R07"></a>

### R07 — tool permissions / durable truth 的 owner 不变

[preflight_tool:2235–2295](../../../crates/opi-agent/src/agent_loop.rs#L2235) 从 trusted registry解析定义、hook只deny/continue、执行前验证schema；[:2297](../../../crates/opi-agent/src/agent_loop.rs#L2297) 才进入可信authorizer。这是当前代码事实；D-22新增可信hook mutation，模型输入先规范化/校验，最终authorizer审实际execution值。replay的普通失败文本、合法化后的wire ID、历史tool declarations仍不能授予执行权限。旧[INV-005](../../opi-spec.md#73-tools-cancellation-and-backpressure)需未来协调，不能否决已确认目标。

旧[§7.4](../../opi-spec.md#74-sessions-and-artifacts)记录committed prefix + immutable binding；当前目标用每call revision与projection识别动态材料，原始事实不回写。D-19允许可信动态schema/实现及active/visible分离，User Policy实时撤销；Q11 child只用于legacy迁移。未来同步旧规范，不能将其作为固定整个run registry的理由。恢复语义同步fixture/迁移，不新增第二权威。

<a id="R08"></a>

### R08 — pi 参考机制及不应复制的部分

pi [transformMessages:95–144](../../../.repo/pi-1.0.0/packages/ai/src/api/transform-messages.ts#L95) 用 provider+api+requested model 判断same：same保留signed/opaque，包括空可读文本但有signature；cross移除opaque，非空readable thinking转text，删除Text/ToolCall的signature。跨route才调adapter ID normalizer，并用旧ID→新ID map改写result。

第二遍 [:158–232](../../../.repo/pi-1.0.0/packages/ai/src/api/transform-messages.ts#L158) 为缺result合成error `No result provided`，把夹在call/results间的system留到results后；跳过Error/Aborted assistant，但不删除任意orphan result，不验证duplicate result，Length不跳过。不能把这段函数当成健全的history validator。若坏历史含被排除assistant的result，需要opi独立确定闭包投影。

pi Anthropic [normalizer:1299](../../../.repo/pi-1.0.0/packages/ai/src/api/anthropic-messages.ts#L1299) 和 Bedrock [:908](../../../.repo/pi-1.0.0/packages/ai/src/api/bedrock-converse-stream.ts#L908) replace非法字符并截64字符，没有在该函数内解决碰撞；不同原始ID可变相同。不能直接拷贝作为Rust唯一性保证。

pi `tool-call-id-normalization.test.ts` [:19](../../../.repo/pi-1.0.0/packages/ai/test/tool-call-id-normalization.test.ts#L19) 会模块级解析OAuth；`tool-call-without-result.test.ts` [:13](../../../.repo/pi-1.0.0/packages/ai/test/tool-call-without-result.test.ts#L13) 也解析凭据并发live请求。名称不等于离线fixture，本次未运行。其“filter orphan”注释 [:69](../../../.repo/pi-1.0.0/packages/ai/test/tool-call-without-result.test.ts#L69) 与当前代码“合成result”不同，应以源码判断，不能把该注释写成现行契约。

## 已确认方向下的目标矩阵

D-19/Q22确认动态transcript与自动适配；D-20/Q23确认中性恢复、Length结构和规范化；D-22/Q25确认可信hook执行值；D-28记录主Agent技术收敛。下表是这些目标的契约细化，不声称已实现。RP编号保留；首轮被替代方向的历史见[决策重审](decision-reassessment.md)和[RT复核](reassessment-transcript-notes.md)。

| ID | 条件 / owner | 当前目标行为 | 失败与机械验收 |
|---|---|---|---|
| RP-01 | 每次prepared logical call；opi-ai准备 | 从冻结的route/revision/history构造一次request projection；同call attempts复用，canonical不改；高层恢复另建关联call由产品owner管理 | 源序列化前后相同，attempt有效payload相同；后继revision不改旧call；不把D-24产品恢复误判成同流隐式重播 |
| RP-02 | continuation metadata；typed历史与adapter | provider+exact wire+requested model作为最小identity；reported model为另一个fact。只有匹配且adapter支持的metadata按block回放；foreign opaque不发送 | 分别改变三个字段；alias与reported不同；legacyunknown不猜same；opaque canary不外流 |
| RP-03 | readable thinking与混合内容；adapter | 同route按具体wire表达；跨route可读thinking转text，signature/opaque不跟随。Text/Thinking/ToolCall的可表示原序保持；缺必要metadata明确降级或unsupported | 全adapter捕获mixed blocks；同模型fixture覆盖实际metadata；空opaque投影不留下非法空assistant；更多已确认continuation目标按D-23及主spec验收 |
| RP-04 | Error/Aborted assistant；projection | canonical保留，wire省略；同批非执行结果成闭包处理，避免orphan。本投影不执行工具，也不自己决定高层retry；D-24产品恢复是独立策略 | Error/Aborted含Text/tool/signature均不wire；与真实执行事实矛盾的历史报错，不静默称未执行；产品重试有独立身份 |
| RP-05 | Length；Core及projection | 全部Length proposals零执行，包括schema-valid的早期call。保留可表达的结构化proposal+可信NotExecuted error result和可读内容；不能表达的坏参数才局部ordinary failure context。产品特定恢复可省略整个attempt view | 完整+截断batch零tool授权/执行；第二次真实wire保留合法pair；坏JSON不伪造{}；产品恢复不留half-pair；后续fresh call正常权限检查 |
| RP-06 | batch关联；opi-ai | 以assistant/batch位置+source ID建有序关联；已闭合batch复用fc_0合法；同batch重复call ID、重复/orphan result、name矛盾报错；缺result按RP-09而非一律拒绝 | 多闭合batch成功；歧义preflight typed error且零auth/network；禁止last-wins；system插在pair间不错误结束batch |
| RP-07 | wire ID限制；adapter | 请求局部稳定一一映射，call/result同步，canonical与execution/evidence identity不改；同prepared call固定 | 非法字符、超长、合法化碰撞、跨batch复用；不归一化掉顺序/关联差异 |
| RP-08 | malformed proposal+可信NotExecuted；Agent事实、opi-ai投影 | canonical保存原proposal/result；wire不能忠实表达时转明确普通失败上下文，不替换{}、null或伪成功call。NotExecuted仅来自可信Agent路径，不靠is_error/文本猜测 | 坏JSON、array/null与实际wire要求的矩阵；第二call可继续且坏提案零执行；普通文本脱敏；合法可表达pair优先保留 |
| RP-09 | 关联明确、result缺失；opi-ai replay | request-local error result准确表达ResultUnavailable，效果UnknownEffect；只补缺项，不写成真正canonical结果，不声称NotExecuted/回滚，不自动重执行。缺失是缺失，不能以未知副作用为理由阻断pi恢复 | trailing/partial batch、下一user/assistant、夹system、跨route ID；补齐可发请求；副作用计数0、canonical不变；歧义仍按RP-06拒绝 |
| RP-10 | 已完成历史pair与新工具执行；projection/Agent | 历史参数不按当前schema再次coerce或执行，保留真实success/error语义；当前registered/active/visible变化不抹旧事实。新调用用其冻结revision，raw→plain-JSON normalized校验→可信hook execution→最终authorizer；不复验hook后的模型schema | 改schema后历史pair不变；新call使用新revision；旧call稳定但实时policy撤销生效；authorizer与执行值相等，raw/normalized/execution可辨 |
| RP-11 | continuation持久化迁移；parser/storage/adapter | 必要item ID/call ID、签名/opaque与block和来源绑定，覆盖parser→typed history→storage→projection；不预建任意JSON扩展袋，不合成丢失老签名 | parse→persist→reload→same-route roundtrip，cross-route剥离；legacy可读且未知明确；各已选adapter真实第二request fixture |

ResultUnavailable/UnknownEffect描述结果是否可得及副作用未知，不预定Rust类型名称。NotExecuted是更强的可信事实；`is_error=true`也可能伴随部分副作用，不能替代它。缺result恢复不会授权工具执行，更不会自动把旧tool proposal重新运行。

RP-03不承诺模型收到相同文本就产生相同行为。RP-11只收已确认adapter所需metadata。RP-06不扩张成任意业务历史的全能validator；采用pi恢复机制也不要求复制缺少ID碰撞防护等局部实现弱点。

## 规范化、动态工具和持久化的接口

精确转换规则由[TP-12–16](transcript-target-proposal.md#tp-13plain-json-规范化的精确参照)统一记载：plain-JSON optional-null/coercion、原可信schema校验、wire strict副本；D-28不移植TypeBox私有metadata分叉。重复回放旧call不重新规范化其历史参数；仅新调用从raw生成normalized，再经可信hook得到execution。当前opi hook没有mutation返回通道，D-22要求公开接口扩展及观察阶段迁移。

D-19使registered、active/callable、wire-visible和authorized各负其责。历史声明、failed marker、模型提供的schema均不注册执行器；冻结revision包含实现/schema来源，但当前User Policy仍能撤销。运行中的旧call不因为后继动态更新切换执行器；正常更新不创建child。

session按[迁移研究](transcript-migration.md)保存原始因果事实与unknown。Q11 legacy child继续有效；普通可信更新不是迁移。D-28压缩为当前有效checkpoint+summary+retained非system+后续变化，request context有损，不能用它证明retained旧call逐时点保真。原始记录仍可审查。

## 首轮方向的退出与完成条件

| 首轮方向/旧路径 | 当前替代与退出证明 |
|---|---|
| Length统一丢全部proposal/result | 合法可表达pair保留，不执行；替换只断言普通文本的fixture，用真实第二adapter请求证明恢复 |
| 所有未知missing result拒绝 | RP-09中性request-local补齐；canonical不改、无副作用、仅缺项补齐的fixture替代零network旧预期 |
| {} / null历史参数fallback | 真实合法值或明确失败上下文；各serializer旧fallback分支退出，保留malformed输出到Core的测试 |
| replay等于tool execution/schema重验证 | replay只转换历史；新call执行经绑定revision、normalized、trusted hook和final authorizer；保留权限负向测试 |
| 同一ID字符串永久唯一 | batch identity与request-local一一映射；闭合fc_0重复是正例，batch内歧义是负例 |
| 原Q16/Q17仍待选择 | 已由Q23明确更新，不重复询问。ordinary failure的闭合来源/字段/role/脱敏依主spec固定，不给Agent自行选择 |

ID编码、内部batch索引和私有serializer合并无需追加访谈。原始缺失metadata不可猜；新protocol事实若与目标矛盾再报告，不静默改变选择。

## 测试影响与交接

| 影响 | 现有边界 / 拟补内容 | 能证明与不能证明 |
|---|---|---|
| retain | [anthropic_thinking_roundtrip.rs:71、160、285](../../../crates/opi-ai/tests/anthropic_thinking_roundtrip.rs#L71) | 同route signed/opaque、foreign剥离、empty signed；不能证明其他wire/live接受 |
| update | [anthropic.rs:1523、1539](../../../crates/opi-ai/src/anthropic.rs#L1523)；Chat mixed text/tool、Responses block order/双ID、Gemini signature | 替换失真fallback为真实pair/失败投影；不删除raw malformed接收路径 |
| retain/update | [tool_validation.rs:719、834、916、1251](../../../crates/opi-agent/tests/tool_validation.rs#L719)、[evidence_runtime.rs:2395](../../../crates/opi-agent/tests/evidence_runtime.rs#L2395)、[cancellation:1868](../../../crates/opi-agent/tests/agent_loop_semantics.rs#L1868) | 保留malformed/Length零执行、partial/cancel事实；修改无coercion/旧hook顺序与旧投影预期，按D-22验证实际授权值 |
| add | RP-01–11 fixture、ResultUnavailable、NotExecuted区别、跨batch fc_0、duplicate/orphan、跨wire opaque canary、第二次真实mock HTTP请求 | 验证保留/剥离/拒绝；不归一化status、顺序、关联或把error改success |
| delete | 仅替代fixture覆盖后删除{} / null伪造历史、缺result一律拒绝、Length一律丢pair的旧断言 | 不以删除测试掩盖malformed provider输出或权限检查；本次无运行时代码删除 |

本页实际执行仍只有源码/测试静态研究与文档编辑，运行时测试未执行；本轮实际测试影响 **none**。文档检查由主任务统一运行。最终主spec负责固定ordinary failure marker等易产生猜测的细节，本页不另建竞争规范。
