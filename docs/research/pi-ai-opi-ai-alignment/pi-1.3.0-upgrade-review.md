# Pi 1.2.0 → 1.3.0：AI／telemetry 对齐升级复核

> 2026-10-16；仅文档、非规范性候选。本轮输入Opi `HEAD c5ab42ab8888b93f9969652323caee87b7550392`，来源为两个本地普通目录，均无`.git`。实际AI／telemetry manifests分别声明1.2.0和1.3.0；据此将用户同句“1.12.0→1.13.0”视为版本笔误。没有验证官方tag/npm产物身份或远端部署。

## 结论及可交付边界

需要跟进，已将当前双语候选和三份技术附件推进到Pi1.3.0。最重要的修正是退出compact frames、禁止把跨模型thinking转成answer、保留最终工具参数失败事实、对齐effort／图片工具结果／provider路由。它们改善可观察调用行为与安全性；cache改善和供应商接受性仍仅是来源意图，未作运行测量。

Schema／WS等所有后端覆盖缺口继续 **OPEN**，具体可关闭证据在[后端附件§5](pi-1.3.0-rust-backends.md#5-pi-130-后端覆盖缺口全部-open)，对应唯一[验收矩阵](acceptance-matrix.md)。TypeBox版本与WS代码没变；argumentsError的新增前置验证不会修复native Schema接受集，移除可配置WS cap不会修复不可配置handshake／compression差异。

67 AIR追踪身份＝57 active SDK＋9 host-impact＋1 retired（AIR-060）；59 V追踪身份＝54 active SDK＋4 host-impact＋1 retired（V-52）。退役不复用ID，不算测试通过。RDF-01–20保留定位，RDF-05只保留不可变普通事件差异。无runtime／Cargo／tests／CI／ledger／正式规范变更，无实施准入、安装、编译、live、commit或push。

## 固定来源与复核深度

| 对象 | 固定身份／变化 | 实际阅读与限制 |
|---|---|---|
| Pi1.2.0 | 1902 files，tree a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10 | 本次重新清点；同摘要事实按前轮有日期审计继承，不冒充全仓重读 |
| Pi1.3.0 | 1925 files，tree 08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2 | 排除.git/node_modules/dist，工具拒绝symlink/junction；包/文本资料全量清点，语义重读限本任务 |
| AI | 394→396；新增4／修改41／删除2，351同摘要 | 全部47变化路径/hunks，包括被删除490行实现与606行测试，公开入口/必要消费者、README变化和发行条目；同摘要文件仅相关链复核 |
| Telemetry | 12→12；修改2，10同摘要 | 全包旧新摘要；README/source/两个test文件与关键AI/Agent/Durable/Chord传播链核对。仅metadata变化 |
| 全仓机器差异 | 新增30／修改192／删除7／同摘要1703 | 只用于发现跨包依赖，不称所有229变化路径已做语义审查 |
| 外部依赖与目录产物 | 外部node_modules lock entry整体无增删改；AI telemetry依赖range→exact1.3.0 | 逐entry JSON比较，不只比较版本字符串。两快照均缺hydrated provider JSON，未import/hydrate、未取得所pin远端catalog bytes |

机器文件摘要、49个目标包变化路径、361项同摘要继承、文档输入和共享工作区边界见[source evidence](pi-1.3.0-source-evidence.json)。三个并行来源审计分别覆盖AI全差异、telemetry/context、backend gap；正文整合其结果，不能把并行审读称为三次独立运行实验。

## 逐项行为变化、收益与处置

以下SOURCE均指新快照；旧实现仍可在`.repo/pi-1.2.0`及证据JSON中定位。Pi源码事实、测试预期、generator意图和本研究选择分开。

| 项 | Pi1.3.0事实与边界 | 对opi-ai目标的具体修订／验证 |
|---|---|---|
| 1 compact公开接口删除 | [CHANGELOG:5–7](../../../.repo/pi-1.3.0/packages/ai/CHANGELOG.md#L5)、[root exports](../../../.repo/pi-1.3.0/packages/ai/src/index.ts#L35)；frame union/encoder/reducer与整份test删除，README段落删除。移除动机“仅实验harness使用”来自changelog，不当作本轮运行证明 | AIR-060/V-52退役；从current spec/SDK/RDF/动作索引删现行compact义务。保留普通events、immutable partial、final Result；没有新的持久化格式或Rust实现删除 |
| 2 model-switch thinking | [transformMessages:122–137](../../../.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts#L122)：provider/API/model三元身份任一不同则drop全部thinking；同身份保留signed/redacted空块，普通空白省略。text/tool仍沿原变换 | AIR-010/038、V-07/32：撤回“可读thinking降级text”。same-provider/API不同model也要drop；canonical原件和projection fact保留，不称lossless |
| 3 final tool arguments | 新[ToolCall字段](../../../.repo/pi-1.3.0/packages/ai/src/types.ts#L442)、[final parser:106–120](../../../.repo/pi-1.3.0/packages/ai/src/utils/json-parse.ts#L106)、[前置validation](../../../.repo/pi-1.3.0/packages/ai/src/utils/validation.ts#L317)。空白→{}；完整repair后必须非null/nonarray object；坏/截断→{}＋argumentsError和原文，不再接受已解析前缀。非法\u必须4hex才当Unicode，\usepackage保留为literal | AIR-048/049/066、V-35/41/49/58：临时展示和最终失败分离；非空error先于coercion，显式owned原文可取而默认错误脱敏。五改动位点为Anthropic/Bedrock/Mistral/Chat/sharedResponses，shared再覆盖Azure/CodexHTTP/WS；Google对象、grammar、PiMessages/custom各循自己的合同，不普遍加strictJSON拒绝 |
| 4 Responses mid-convo effort | [guard:101–115](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-responses.ts#L101)、[marker:224–231/347–376](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-responses-shared.ts#L224)。flag默认false＋truthy native effort，排除精确directChatGPT分支（provider=openai、标准baseURL、apiKey已定义（含空串）且非sk-前缀）。same-identity且有stamp/非空reply才插历史marker；当前marker始终追加，包括tooloutput后及同effort重复；顶层effort仅在model.reasoning分支发出、在sampling/onPayload覆盖前以medium起始 | AIR-029/038/057/062、V-16/32/49/54：保留映射level/stamp/order，不自行去重或取消最终marker。summary-only/simple off/省略/flagfalse走旧路径；metadata在onPayload前记录，不担保任意callback重写与stamp一致。generator仅4精确ID：gpt-6-astra/sol/luna、gpt-6.1-sol；无普遍GPT6／cache-hit保证 |
| 5 Anthropic thinking off | [buildParams:1205–1221/1278–1290](../../../.repo/pi-1.3.0/packages/ai/src/api/anthropic-messages.ts#L1205)：reasoning且显式false且off map非null时优先disabled，不发marker/output_config/block_binding/stamp；off不支持仍managed adaptive。reply stamp改由实际variant build返回 | AIR-029/053/057/062、V-11/16/32/45/49/54：明确supported/unsupported off两个分支，numeric budget truthy1024不改。generator Opus5/Haiku5.5 whitelist是metadata来源意图，不等于所有Claude已支持off |
| 6 error-tool images | 新[split helper:14–59](../../../.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts#L14)、[Anthropic:1500–1523](../../../.repo/pi-1.3.0/packages/ai/src/api/anthropic-messages.ts#L1500)、[Bedrock:1093–1131](../../../.repo/pi-1.3.0/packages/ai/src/api/bedrock-converse-stream.ts#L1093)。error且有图才split，连续image run→placeholder，caption＋images放同turn全部连续toolresults后；Bedrock仅Claude | AIR-014/057、V-07/08/49：保留normalized call ID/tool caption、图像次序和text，成功/nonClaude/noimage负例；nonvision先走原placeholder。不是统一改变所有tool-result schema |
| 7 Chat delta order／Google wire | [Chat:591–653](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts#L591)同chunk reasoning/reasoning_details先于content，再tools；不保证全部跨chunk thinking早于answer。[Google:75–80](../../../.repo/pi-1.3.0/packages/ai/src/api/google-shared.ts#L75)新增Gemini4及以后regex到原level分支，map仍限制可用level | V-16/32/41/49/54：补同chunk排序及Gemini/Vertex实际payload；不从model名字授予所有level，不改已有Gemini3/Gemma/latest分支 |
| 8 Kimi／OpenCode | [Kimi factory](../../../.repo/pi-1.3.0/packages/ai/src/providers/kimi-coding.ts#L1)A→C、/coding→/coding/v1，key/OAuth来源不变。[Go:10–21](../../../.repo/pi-1.3.0/packages/ai/src/providers/opencode-go.ts#L10)新增Google；[generator:2253–2288](../../../.repo/pi-1.3.0/packages/ai/scripts/generate-models.ts#L2253)Zen/Go→inference[/go]，保留protocol suffix；MiniMax M2.7的forceChat特例删除，按源npm provider为Anthropic，不是给任意MiniMax强改A | V-11/44/49/56：当前factory/path更新，旧explicitoverride需带/v1，不能静默改任意customendpoint或洗历史A。K3/K3-256K/KimiForCoding的inline tools/effort与Highspeed不同；旧k2p5/k2p6/k2p7 alias归一化/过滤撤回，reasoning取m.reasoning，旧kimi-k2-thinking impliedprice移除，k3-256k无隐含k3同价。classifier固定root也改/inference/v1；无hydratedcatalog不得虚构记录 |
| 9 metadata generator | [Xiaomi:2487–2503/2540](../../../.repo/pi-1.3.0/packages/ai/scripts/generate-models.ts#L2487)reasoning模型仅off/high且不用reasoning_effort；[Codex window:3172](../../../.repo/pi-1.3.0/packages/ai/scripts/generate-models.ts#L3172)只有openai-codex/gpt-6.1-sol从272000→500000，其他所列GPT6仍272000 | V-11/44/49/54：保留toggle、source-specificmodel metadata和exactID，不增加SDK输入上限或伪称本地materializedcatalog/真实服务接受 |
| 10 whole-call error classifier | [retry.ts:47/58](../../../.repo/pi-1.3.0/packages/ai/src/utils/retry.ts#L43)补service is temporarily unavailable与unable to forward upstream；quota/billing排除仍优先。provider-retry.ts、请求次数／delay/cancel／transport字节未变 | AIR-065/V-57补两措辞正例及非retry负例；只有显式helper可开新logical calls，不变成AIR-007低层默认重试。source新增service文本tests，但没有找到独立Unable-to-forward新增test，故Rust仍要补 |
| 11 telemetry／context | [telemetry interface](../../../.repo/pi-1.3.0/packages/telemetry/src/index.ts#L14)、[AI type](../../../.repo/pi-1.3.0/packages/ai/src/types.ts#L157)、[simple forwarding](../../../.repo/pi-1.3.0/packages/ai/src/api/simple-options.ts#L36)同摘要；仅version/changelog与exact内部依赖。AI无新增spans/exporter；Pi[auth await后spread](../../../.repo/pi-1.3.0/packages/ai/src/models.ts#L849)及[lazy立即setup](../../../.repo/pi-1.3.0/packages/ai/src/api/lazy.ts#L46)不保证创建时freeze | AIR-054–056/V-46–48继承explicitparent职责，Rust frozenSpan＋sourceDispatch、首次poll、Stream/Drop为明确语言/所有权差异。不恢复过时Agent schema/import，不将Chord invocation Context或Durable model Context混成trace carrier |

## 包外工作流、文档意图与未实现规划

AI外部SDK精确版本不变：Anthropic0.129.0、AWS3.1127.0、Google2.21.0、OpenAI7.19.0、partial-json0.1.7、TypeBox1.3.27。完整lock比较只有16个localworkspace entries变化；外部entry对象（含resolved/integrity/transitive deps）不变，因而可继承前轮精确依赖阅读，但不把本机运行时global WebSocket等同lock中的ws8.21.0。

[publisher最低版本](../../../.repo/pi-1.3.0/scripts/publish-model-catalog.mjs#L28)由0.80.7升为**1.2.1**，理由是Kimi的新wire，不是1.3.0，也不是Rust crate semver。distribution schema v1、hydration data v6未变。[update pin](../../../.repo/pi-1.3.0/scripts/update-model-catalog-pin.mjs#L33)先取不带pi-version的最新catalog，按provider/type/api精确分组检查missing/extra，并返回选定bytes；[release:98–131](../../../.repo/pi-1.3.0/scripts/release.mjs#L98)在version bump前hydrate该bytes并build:offline，失败恢复pin。[pinned-deps](../../../.repo/pi-1.3.0/scripts/check-pinned-deps.mjs#L48)/[sync-versions](../../../.repo/pi-1.3.0/scripts/sync-versions.js#L44)要求内部依赖精确pin并覆盖optionalDependencies，脚本tests随改。这里只同步测试/发布工作流意图，没有运行或照搬这些脚本到Rust CI。 [test.sh:72–76](../../../.repo/pi-1.3.0/test.sh#L72)新增显式保留NODE_EXTRA_CA_CERTS供代理CA，其余隔离env策略保留；不把此TS测试环境机制当成Rust trust-store已对齐或默认离线网络封锁。nix catalog pin变化不证明远端bytes。

新源码的AI文档仅README和CHANGELOG；telemetry文档仅README和CHANGELOG。AI删frames并修正thinking连续性说法；telemetry README全464行同摘要，但[323–338/365–383](../../../.repo/pi-1.3.0/packages/telemetry/README.md#L323)仍引用当前Agent没有导出的AGENT_TELEMETRY_SCHEMAS/AI_TELEMETRY_SCHEMA/HARNESS_TELEMETRY_SCHEMA与starter，属于过时示例，不是待实现承诺。Agent src同摘要；Durable output向Chord移动不恢复旧Agent/harness schemas，不能由包名/Context同词推导统一trace机制。

本轮没有新的独立AI／telemetry本地路线图。Responses effort等上表有实现＋测试源码；Lite、真实内置remote-deferred、默认SDKspans/exporter仍未在所审入口交付。更早RFC/PR的在线Discussion/closed状态保留原访问窗口，本轮未联网刷新，不能据本地1.3.0推断上游所有后续状态或未来承诺。[tracking](upstream-tracking.md)记录明确重评触发。

## 过时内容清理与历史保全

- README重写为当前入口，移除旧1.0.0长正文中的“当前／已回答／已通过／排除能力”等陈旧现行叙述；历史完整字节从Git `c5ab42a`恢复。
- 三份现行SDK/RDF/backend正文迁入1.3.0文件；原1.2.0路径只保留简短历史恢复入口，旧hash/review不追认新稿。旧参数审计继续保留其来源日期与已撤回值，只有本轮变化项被替代。
- 从current spec／动作索引／RDF撤回compact和跨模型thinking转text；AIR-060/V-52留tombstone，历史审计/人类Q答案不是默认实现授权。全部旧review/JSON属于固定输入证据，无需复制其要求或追改执行记录。
- 只清理本目录现行契约；共享工作区中其他研究删除/规范导航改动属于并行任务，本轮未stage/改写。正式实施仍受opi-spec/AGENTS/既有工作流约束。

## 全部目标包变化路径与处置

下表覆盖49条变化路径（AI47＋telemetry2），逐行new/old链接和摘要在证据JSON；删除源不伪造新路径。T为测试源码预期、G为generator意图、S为实现／接口、D为文档、M为metadata。所有T的add/update/delete均为来源事实与未来Rust设计，实际本轮Rust test impact **none**。

| Path（package内） | 变化／类别 | 处置与证据边界 |
|---|---|---|
| [packages/ai/test/anthropic-error-tool-result-images.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-error-tool-result-images.test.ts) | added／T | source tests映射至唯一矩阵；未执行；V-07/08/49 |
| [packages/ai/test/json-parse.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/json-parse.test.ts) | added／T | source tests映射至唯一矩阵；未执行；V-35/41/49/58 |
| [packages/ai/test/openai-responses-mid-convo-effort.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-responses-mid-convo-effort.test.ts) | added／T | source tests映射至唯一矩阵；未执行；V-16/32/49/54 |
| [packages/ai/test/transform-messages-cross-model-thinking.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/transform-messages-cross-model-thinking.test.ts) | added／T | source tests映射至唯一矩阵；未执行；V-49/58/59 |
| [packages/ai/CHANGELOG.md](../../../.repo/pi-1.3.0/packages/ai/CHANGELOG.md) | modified／D | 版本/依赖或文档事实；不证明运行变化；V-49/58/59 |
| [packages/ai/README.md](../../../.repo/pi-1.3.0/packages/ai/README.md) | modified／D | 版本/依赖或文档事实；不证明运行变化；V-49/58/59 |
| [packages/ai/package.json](../../../.repo/pi-1.3.0/packages/ai/package.json) | modified／M | 版本/依赖或文档事实；不证明运行变化；V-49/58/59 |
| [packages/ai/scripts/generate-models.ts](../../../.repo/pi-1.3.0/packages/ai/scripts/generate-models.ts) | modified／G | metadata意图；缺hydrated输出，不称catalog证明；V-11/44/49/50/54/56 |
| [packages/ai/src/api/anthropic-messages.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/anthropic-messages.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-07/08/16/35/49 |
| [packages/ai/src/api/bedrock-converse-stream.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/bedrock-converse-stream.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-07/08/35/49 |
| [packages/ai/src/api/google-shared.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/google-shared.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-16/49/54 |
| [packages/ai/src/api/mistral-conversations.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/mistral-conversations.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-35/41/49 |
| [packages/ai/src/api/openai-completions.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-completions.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-32/35/41/49 |
| [packages/ai/src/api/openai-responses-shared.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-responses-shared.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-16/32/35/41/49 |
| [packages/ai/src/api/openai-responses.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/openai-responses.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-16/32/49/54 |
| [packages/ai/src/api/transform-messages.ts](../../../.repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-07/32/49 |
| [packages/ai/src/index.ts](../../../.repo/pi-1.3.0/packages/ai/src/index.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-52 retired; V-58 |
| [packages/ai/src/providers/compat-schema.ts](../../../.repo/pi-1.3.0/packages/ai/src/providers/compat-schema.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-11/16/49/54 |
| [packages/ai/src/providers/kimi-coding.ts](../../../.repo/pi-1.3.0/packages/ai/src/providers/kimi-coding.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-44/49/56 |
| [packages/ai/src/providers/opencode-go.ts](../../../.repo/pi-1.3.0/packages/ai/src/providers/opencode-go.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-44/49/50 |
| [packages/ai/src/types.ts](../../../.repo/pi-1.3.0/packages/ai/src/types.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-35/41/58 |
| [packages/ai/src/utils/json-parse.ts](../../../.repo/pi-1.3.0/packages/ai/src/utils/json-parse.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-35/41/49/58 |
| [packages/ai/src/utils/retry.ts](../../../.repo/pi-1.3.0/packages/ai/src/utils/retry.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-57 |
| [packages/ai/src/utils/validation.ts](../../../.repo/pi-1.3.0/packages/ai/src/utils/validation.ts) | modified／S | 按上表精确分支更新contract；无Rust实施；V-35/58 |
| [packages/ai/test/anthropic-adaptive-thinking-models.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-adaptive-thinking-models.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-11/45/49/54 |
| [packages/ai/test/anthropic-empty-thinking-signature-compat.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-empty-thinking-signature-compat.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-49/58/59 |
| [packages/ai/test/anthropic-force-adaptive-thinking.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-force-adaptive-thinking.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-11/45/49/54 |
| [packages/ai/test/anthropic-mid-conversation-effort.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-mid-conversation-effort.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-16/32/49/54 |
| [packages/ai/test/bedrock-convert-messages.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/bedrock-convert-messages.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-07/08/49 |
| [packages/ai/test/classifier-models.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/classifier-models.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-49/50 |
| [packages/ai/test/context-overflow.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/context-overflow.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-44/58 |
| [packages/ai/test/google-thinking-level-map.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/google-thinking-level-map.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-16/49/54 |
| [packages/ai/test/openai-completions-reasoning-details.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-completions-reasoning-details.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-32/41/49 |
| [packages/ai/test/openai-completions-tool-choice.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-completions-tool-choice.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-21/41/49 |
| [packages/ai/test/openai-responses-message-id.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-responses-message-id.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-41/49 |
| [packages/ai/test/openai-responses-partial-json-cleanup.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-responses-partial-json-cleanup.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-35/41/49 |
| [packages/ai/test/openai-responses-reasoning-replay-e2e.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/openai-responses-reasoning-replay-e2e.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-07/32/49 |
| [packages/ai/test/retry.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/retry.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-57 |
| [packages/ai/test/stream.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/stream.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-44/49 |
| [packages/ai/test/supports-xhigh.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/supports-xhigh.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-49/54 |
| [packages/ai/test/total-tokens.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/total-tokens.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-03/49/54 |
| [packages/ai/test/transcript-tool-changes.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/transcript-tool-changes.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-16/44/49 |
| [packages/ai/test/transform-messages-copilot-openai-to-anthropic.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/transform-messages-copilot-openai-to-anthropic.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-49/58/59 |
| [packages/ai/test/validation.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/validation.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-35/58 |
| [packages/ai/test/xiaomi-models.test.ts](../../../.repo/pi-1.3.0/packages/ai/test/xiaomi-models.test.ts) | modified／T | source tests映射至唯一矩阵；未执行；V-11/49/54 |
| [packages/telemetry/CHANGELOG.md](../../../.repo/pi-1.3.0/packages/telemetry/CHANGELOG.md) | modified／D | 版本/依赖或文档事实；不证明运行变化；V-46–48/59 |
| [packages/telemetry/package.json](../../../.repo/pi-1.3.0/packages/telemetry/package.json) | modified／M | 版本/依赖或文档事实；不证明运行变化；V-46–48/59 |
| [packages/ai/src/utils/assistant-message-frame.ts](../../../.repo/pi-1.2.0/packages/ai/src/utils/assistant-message-frame.ts) | removed／S | retire compact专属要求；保留普通event/Result；V-52 retired; V-40/41 retain |
| [packages/ai/test/assistant-message-frame.test.ts](../../../.repo/pi-1.2.0/packages/ai/test/assistant-message-frame.test.ts) | removed／T | retire compact专属要求；保留普通event/Result；V-52 retired; V-40/41 retain |

## 复审与实际检查

最终独立复审、输入/输出摘要和文档检查见[复审结论](pi-1.3.0-revision-review.md)。来源比较和全部文档一致性是静态证据；未运行任何Pi/Rust产品测试、编译、差分runner、live/OAuth、metadata/process fixtures。当前Schema/WS等OPEN项不因文档可行性而关闭。
