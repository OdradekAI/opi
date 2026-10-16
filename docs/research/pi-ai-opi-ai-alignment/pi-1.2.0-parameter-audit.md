# Pi 1.2.0 参数、预算与新增限制来源审计

> 2026-10-16，文档修订证据。输入固定于[本轮baseline](pi-1.2.0-parameter-revision-baseline.json)；精确依赖发布归档与报告摘要在[source evidence](pi-1.2.0-parameter-source-evidence.json)。本页逐项解释来源和处置，不增加AIR/V或实施账本；[验收矩阵](acceptance-matrix.md)仍是唯一验收索引。双语spec共同采纳本页，附件没有既有英文counterpart；下面保留英文只读取证原稿以便复查。

## 结论与适用顺序

前轮“固定有限默认值／后端选型完成／无必须修正项”不能证明阈值合理或Pi语义等价。本轮撤回无Pi/协议依据的固定值和新增拒绝；不另猜替代数字。Pi显式参数只在真实作用域保留；backend默认与不可配置上限不能冒称Pi上限。资源政策省略时不施加撤回的SDK默认限制，但runtime/provider/实际分配仍有边界；不承诺无限资源。

本页最终处置与当前双语spec/RDF/backend/唯一矩阵优先于旧SP/TP/D/Q和前轮复审中的冲突预算。历史回答/测试结果与发布源码不重写。原有runtime/Cargo/tests/CI/正式规范/implementation ledger全部未改；测试影响none。正式§3.3有界buffers/backpressure与安全/权限/准入保持，未规定这批MiB/秒/数量值。移除研究稿的任意内容cap不等于取消并发/队列边界，也不要求修改正式规范。

## 分类与判据

| 分类 | 有效证明 | 当前处置 |
|---|---|---|
| Pi显式行为 | 固定adapter/helper消费代码、测试及其选定配置 | 保留准确单位、scope、0/省略/重试含义；不能只凭类型注释推导runtime拒绝 |
| 协议结构 | 长度字段、CRC、WS control/UTF-8等真实wire规则 | 保留必要校验；service建议或backend策略不自动成为客户端必须拒绝 |
| 后端默认／硬约束 | 精确版本/feature的源码 | 记录出处及可配置性；配置消除额外接受限制或保持未关闭coverage，不能自动照搬 |
| 既有opi选择 | 当前Rust代码、原确认及其版本窗口 | 不改当前runtime；候选若不同于Pi须明示理由/影响，不能称其为Pi默认 |
| Rust接口／安全差异 | 所有权/类型/权限与真实观察差异 | 保留必要机制及明确的失败/信息差异；“Rust更严格”不能授权任意新拒绝 |
| 无依据提案 | 没有上面来源、没有语料/测量标定 | 撤回默认值和拒绝；显式caller政策不是默认等价证明，不增加配置框架 |
| 实际coverage缺口 | 后端不能表达Pi已支持语义、平台行为未知 | 保持未完成；不以typed Unsupported/整体拒绝或忽略约束关闭义务 |

## 最终处置总表

以下按参数对象分组；同组所有具体数字/新增条件及逐项出处见后续取证表。版本号、协议日期、OAuth client ID/端口、能力枚举数量、test输入和来源行号各有身份含义，不当成可调资源预算。

| 对象 | 保留／撤回／未完成 |
|---|---|
| SSE/AWS/WS/compact/partial/tool/opaque | 撤回8/16/32MiB、4096blocks/depth64、AWS headers128KiB等草案cap。保留真正EventStream12/16/4-byte结构与CRC、WS control125；Pi SSE增量TextDecoder replacement保留，不新增fatal byte拒绝 |
| 队列／terminal | 现opi64事件为内部调度差异，保留背压；撤回64MiB logical charge与single-event超cap失败、256KiB terminal envelope。合法大event须可继续，唯一终态保留完整partial/known usage；不保证固定RSS |
| WS传输 | Pi connect15s、idle5min/max-age55min有来源；idle timer与next-acquire候选机制须区别。撤回own64KiB/124header、buffer64KiB/maxwrite32MiB、close2s。tungstenite默认recv16/64MiB/readwrite128KiB/usize::MAX与握手硬64KiB/124/512/small-read、无compression单列backend事实；不可配置差异保持gap |
| image | 撤回8图、16MiB/图、32MiB总、64KiB text、48MiB body、四MIME/strict base64/签名/非空bytes/必须一图/坏entry全拒绝。保留Pi first-choice/text顺序、空成功及跳过不匹配URI，不下载/写文件；owned结果需保留encoded原值/可选decode状态 |
| classifier | 撤回1–64题、1MiB/depth32/2MiB、统一2–255labels/score>=2/unique-levels和概率range/choice-membership/full-probability-set/score-range拒绝。保留各parser真实required/type/finite；llama choice2–62/score2–10、Decisions images128及算法只作用该协议 |
| image/classifier/chat timeout | image OpenAI7.19.0的600000ms覆盖非流式JSON body/解析的remaining attempt时间；Mistral60000ms仅fetch-to-headers。两者不可混同。classifier/helper/Codex/native按各源码，省略不加新总generation deadline |
| OAuth/store/barrier | Pi15s仅refresh网络signal；撤回ownership→ACK总15s、barrier30s、256-key准入及ADC60s。保持ACK/serialized ownership/cutoff与unknown receipt/fence；显式wait deadline只停止等待，10s refresh+6s ACK为不应新增失败的正例 |
| AWS/ADC资源 | 撤回XML/Query1MiB/depth32/tokens16384/attrs4096/per-element256；ADC总60s/16sends/depth4/1MiB、stdio64KiB。现opi AWS process10s/stdout64KiB是既有差异，Node exec未设置这些值；不外推ADC。Google executable真实默认30s/config5–120s保留，不截为60s |
| retry | 保留helper/Codex/Smithy真实次数/指数/jitter/quota/协议分支。helper/Codex hint cap60000与0 sentinel不推广Bedrock；撤回Bedrock新增60s hint拒绝与新header优先级、ADC auth全0，保留Google POST retry3/no-response2与source机制。独立Codex一次missing-previous及一次connection-limit不能合为总一次 |
| redirect/proxy/TLS | 撤回blanket none、ADC同origin5跳；Google node-fetch实际follow20只作用该路径，准确method/body/header规则，其他runtime未知保留gap。HTTP(S)-only/SOCKS-PAC拒绝有Pi helper来源。受保护来源/权限/secret/TLS验证保留并列RDF；H1-only、roots默认、ring key范围是backend差异，不是Rust强制 |
| Schema | 撤回全部草案bytes/nodes/depth/resources/work/cache/jobs/errors数字、强制Draft7/三draft/unknown-format-keyword/跨dialect/cycle拒绝、所有patterns×subjects guard及private canonical改写。Pi TypeBox忽略unknown format/keyword，保留known格式与实际内存ref；无隐式schema I/O。jsonschema选择降为conditional，ECMA/format/keyword/content/ref/吞错缺口未关闭；backend每match1e6/regex cache10、TypeBox诊断maxErrors8不是SDK接受预算 |
| model/token/cost | cost threshold初始-1/仅>-1及重复取首也保持源码scope，不增正数准入；保留源码4096/min1、thinking1024/2048/8192/16384、Google表、4800/3.5 UTF16估算、native floor16及cost公式的准确scope；撤回通用M正整数预检/Claude零BudgetInsufficient，按Anthropic truthy-fallback1024/Bedrock `??`/Chat省略。inputLimits是metadata，缺runtime reader时不新增SDK默认cap/resize。所有来源derived表见附录D |
| custom API／cleanup | 保留Rust类型安全/owned捕获/dyn/统一driver，撤回ApiId grammar/trim/新length、重复注册拒绝和仅namespace/revision不符即拒绝兼容options。保留upsert/可兼容跨Models和oldcall。空cleanup ID不扩大为All是明确安全差异，非Pi事实；有界所有权不需要2s新deadline |
| 其他helper/metadata/端口 | promptcache64Unicode scalar截断、UUID48-bit timestamp/41-bit序列、32-bit hash、overflow0.99、1h subset/price tiers/multipliers、cache TTL、flow端口/interval/expiry皆按对应源码/算法保留；注释里的Bedrock requestMetadata50/64/256不是Pi preflight实现证明。4h产品freshness、retry3/2s/60s、deferred fixture5s不成为SDK新增默认 |

## 仍有理由保留的差异与证明缺口

Rust的owned/lazy Future、Result携partial、immutable snapshots、active result drain、typed secret-safe错误、known/unknown usage与原子ACK是明确接口/信息/生命周期差异；它们不是新增容量/内容拒绝的理由。single near-expiry5min是对flow预减+resolver重复窗口的已标明修正，须单独比较时点。保护header/来源/权限、ProxyPlaceholderSigning零真实凭据、空cleanup不扩为All等安全差异仍记录真实影响；不由本次审计扩大正式授权。

Schema TypeBox对应机制、WS不可配置handshake/compression、普通BedrockH2、runtime trust roots/crypto范围、全平台/供应商接受性依然需要实施准入后证据；本轮不制造第二套validator、fork或配置体系。若必需backend替换或正式规范变更，后续以具体方案核准，不把这些gap自动算入完成。

## 只读取证附录的解释

附录A–E整合调查者的参数表、准确发布源码路径/行号与反例；后续独立复审纠正取证遗漏处已明确更新，不是逐字报告副本。候选文档旧行号只定位其调查时输入，后续父审已修订；源代码行号仍对应固定snapshot/archive。附录中的建议不高于上方最终处置，特别是旧“保留已选安全policy”不能复活已撤回默认值。临时绝对路径只说明调查过程，长期身份以source evidence内URL/SRI/hash和相对member path为准。

## 取证附录A：流、媒体与classifier

## Read-only audit: streaming, media and classifier budgets

Date: 2026-10-16. Scope: candidate research documents versus pinned `.repo/pi-1.2.0` and current Opi. No repository files edited; no tests/build/install/live requests executed. Test impact: **none**.

### Classification and authority

- **Pi-explicit**: enforced by pinned Pi adapter/helper; not necessarily a universal provider hard maximum.
- **Protocol-hard**: structural/wire validity; distinguish from a discretionary resource ceiling.
- **Backend-default/constraint**: exact selected dependency behavior; not Pi policy and not an optimal value.
- **Existing-opi**: already implemented current contract; retaining it needs an explicit parity difference where observable.
- **Unsupported proposed Rust difference**: researcher-selected number/new rejection without Pi/protocol/current-code justification. Historical D/Q approval is historical candidate provenance, not evidence that the value comes from Pi.

Formal `docs/opi-spec.md` §1.1 leaves wire constants to authoritative implementation sources; §3.3 prefers bounded buffers/explicit backpressure, but assigns no 8/16/32/64 MiB values. Queue backpressure and checked allocation are compatible with the formal direction. Removing invented size defaults from this non-normative candidate does not itself require changing the formal spec. A demand for literally unlimited memory or removal of backpressure would be a distinct design decision; lack of an explicit Pi cap does not establish unlimited Node/Bun/browser capacity.

### Source abbreviations

All Pi paths below are relative to `.repo/pi-1.2.0/packages/ai/`.

- COD = `src/api/openai-codex-responses.ts`.
- IMG = `src/api/openrouter-images.ts`.
- SYS = `src/api/system-one-shared.ts`.
- CLS = `src/api/classifier-shared.ts`.
- DEC = `src/api/openai-decisions.ts`.
- LLA = `src/api/llama-cpp-classify.ts`.
- FRM = `src/utils/assistant-message-frame.ts`.
- EVT = `src/utils/event-stream.ts`.
- RUSTWS = `%TEMP%/opi-ws-backend-source/tungstenite-0.28.0/src/`.
- NODE = `%TEMP%/opi-parameter-npm-source/` (root agent restored exact official npm archives; pinned lock integrity manifest at `manifest.json`).
- OAI = NODE + `openai-7.19.0/package/src/`.
- SMITHY = NODE + `@smithy__core-3.33.3/package/dist-es/submodules/event-streams/`.

The exact locked OpenAI is `.repo/pi-1.2.0/package-lock.json:5105–5108` **7.19.0**, not root 6.40.0. Pi Codex calls runtime `globalThis.WebSocket` (COD:1023–1025), with a Bun-specific proxy wrapper (COD:1000–1020), so neither the npm `ws` package default nor undici's npm version proves the effective runtime WebSocket limit.

### 1. Streaming numerical inventory and disposition

Candidate locations: `pi-1.2.0-rust-backends.md:17–38,137–143`; `spec-draft.md:83–84,355,366,383–385`; `pi-1.2.0-rust-differences.md` RDF-04/05/06. Every value below needs synchronized English/Chinese spec, matrix, SDK annex and RDF cleanup when withdrawn.

| Candidate rule/value | Classification | Evidence and recommended disposition |
|---|---|---|
| SSE event and unfinished buffer **8 MiB**, counting encoded lines/separators | Unsupported proposed Rust difference | COD:800–849 incrementally appends decoder output and parses events, with no explicit length cap; Mistral `src/api/mistral-conversations.ts:460–493` and Pi Messages `src/api/pi-messages.ts:273–305` also have no such cap. Withdraw fixed default. A documented caller-selected host resource policy may cap it; do not replace 8 with another guessed number. |
| AWS total frame **8 MiB** | Unsupported proposed Rust difference | SMITHY `eventstream-serde-universal/getChunkedStream.js:6–14,40–47` reads unsigned 32-bit size then allocates that declared size, without 8 MiB admission. Current Opi `crates/opi-ai/src/bedrock/event_stream.rs:27–40` also has no such cap. Withdraw fixed default. |
| AWS headers **128 KiB** | Unsupported as this client's mandatory rejection/default | SMITHY `eventstream-codec/splitMessage.js:11–29` reads u32 headerLength and slices after CRC checks; no 128 KiB client check. Do not infer hard client requirement from a service-format maximum; that requires its own protocol source and client applicability. Withdraw mandatory local cap absent that evidence. |
| AWS prelude **12 bytes**, minimum total **16 bytes**, final CRC **4 bytes**, headers <= total−16; checked arithmetic; prelude/message CRC | Protocol-hard + existing-opi | SMITHY `eventstream-codec/splitMessage.js:2–29` establishes 8-byte length fields +4 prelude checksum +4 message checksum, declared length equality and both checksums. Current Opi `bedrock/event_stream.rs:43–76` already checks structural length and CRC, but waits for the whole declared frame before parsing. Retain structure/CRC/overflow checks; moving prelude validation earlier is safer timing, not justification for a new cap. |
| WS received data frame **8 MiB**, reassembled message **16 MiB** | Unsupported proposed Rust difference | COD runtime constructor above and `connectWebSocket` COD:1091–1097 set headers only, no SDK max frame/message. Selected tungstenite defaults are **16 MiB frame /64 MiB message**, RUSTWS `protocol/mod.rs:95–104`; those defaults are backend evidence, not proof Pi accepts exactly those maxima. Withdraw 8/16 defaults; separately document unavoidable backend constraints and tune only on real compatibility evidence. |
| WS outgoing JSON **16 MiB** | Unsupported proposed Rust difference | COD:1543 directly sends JSON.stringify of request; no send-byte budget. Tungstenite receive max_message_size is not an outgoing-message maximum. Withdraw mandatory outgoing cap. |
| WS control payload <=**125 bytes**, no fragmentation, text valid UTF-8 | Protocol-hard / selected backend enforcement | RUSTWS `protocol/mod.rs:656–665` states RFC control constraints; `protocol/message.rs:39–71` rejects invalid/incomplete UTF-8. Retain protocol rules. Do not transfer text-frame UTF-8 rejection to binary transport bytes. |
| Binary UTF-8 replacement expansion capped at **16 MiB** | Replacement is Pi-explicit; number unsupported | COD:1291–1304 uses TextDecoder for ArrayBuffer/views/blob bytes. Preserve per-path replacement semantics and parse JSON afterward; withdraw added expansion cap absent a selected host policy. |
| Compact decoded frame **16 MiB** | Unsupported proposed Rust difference | FRM:328–486 clones/appends/reduces content with index/kind/ended checks, no byte ceiling. Withdraw fixed cap. |
| Retained assistant canonical partial **32 MiB**, **4096 blocks**, JSON depth **64** | Unsupported proposed Rust difference | FRM:340 pushes next block; :400/:424/:462 append deltas; :470 clones complete arguments. No such Pi cap. These can reject legitimate long output even with continuous consumption; they are not queue backpressure. Withdraw all three defaults. |
| Tool raw/parsed JSON each **8 MiB**, signature/opaque block **8 MiB** | Unsupported proposed Rust difference | Pi FRM:453–470 keeps JSON scratch/arguments and end metadata without those limits. Withdraw fixed values. Retain preservation of original raw/effective facts and checked encoding. |
| Progress queue **64 events** | Existing-opi resource behavior | Current `crates/opi-ai/src/openai_chat.rs:1513`, `anthropic.rs:1422`, `openai_responses.rs:563`, `bedrock/mod.rs:238`, `gemini.rs:884` use mpsc::channel(64). Pi EVT:3–22,43–57 is a FIFO with no count check. Retain existing Opi backpressure as declared difference; do not claim a count cap is an RSS bound or a Pi maximum. |
| Queue logical canonical charge **64 MiB** including full partial, single event >whole budget rejected | Unsupported proposed Rust difference | EVT has no byte admission. Logical encoding of shared snapshots is not actual allocation; rejecting a large event creates an extra valid-output restriction. Withdraw default and rejection. Retain cancellation-aware backpressure without inventing a byte threshold. |
| At most **1** producer event under construction/waiting; no spawn per event or parse-all archive | Internal ownership/concurrency mechanism | Does not itself impose a new payload acceptance limit. Retain as an implementation invariant if useful, without treating 1 as a Pi numeric budget or promising total RSS. |
| Independent terminal/result slot **1**, plus retained state +**256 KiB** envelope | Single settlement is semantic; envelope number unsupported | Pi EVT:46–48 resolves final result independently of queue; :86–87 returns same Promise. Retain unique result and terminal availability even when queue is full; withdraw 256 KiB fixed envelope and dependency on unsupported state cap. Avoid truncating partial/usage or recursive queue failure. |
| Positive-only overrides; **0** and unlimited sentinels always InvalidInput | Unsupported blanket public restriction | This follows newly invented budgets, not Pi. Preserve checked overflow and option-specific invalidity; represent absent optional host policy directly. Never infer that all Pi zero-valued options are invalid. |
| WS read/write buffers each **64 KiB**, max write buffer **32 MiB**, derived write_buffer + outgoing +**14** | Unsupported tuning / protocol overhead partly real | RUSTWS `protocol/mod.rs:98–102` defaults read/write128KiB, max write usize::MAX, message64MiB/frame16MiB. :148–150 asserts max_write>write_buffer. A maximum client frame header can cost14 bytes, but that does not prove 32MiB or64KiB choices; remove tuning defaults/derived16MiB dependency. Preserve config relation validation and Sink backpressure. |
| WebSocket handshake **65536 bytes /124 headers /512 reads**, after **64** reads average >=**128 bytes** | Selected backend constraint | RUSTWS `handshake/machine.rs:163–185`; `handshake/headers.rs:10,33`; not RFC maxima or Pi policy. Disclose exact inability to configure these through WebSocketConfig. The small-read heuristic can reject a legal fragmented handshake; compatibility limitation must remain visible, not normalized away. |
| Owned CONNECT parser **64 KiB /124 headers** | Proposed Rust dialer policy, distinguish backend constraint | Backend's Upgrade parser has those limits, but that does not automatically justify independently imposing them on proxy CONNECT. Withdraw arbitrary own-parser limits or source them separately; if consciously inheriting a parser constraint, identify it as a backend limitation. |
| Close wait <=**2s** | Unsupported new Rust deadline | Pi COD:1057–1060 simply invokes socket.close; no awaited2s guarantee. Retain owned cleanup/finalization contract, but remove this invented parity default or isolate as explicit host shutdown policy. |
| Connect timeout **15s** | Pi-explicit | COD:57,1080,1141–1147. Retain scope accurately; not whole generation deadline. |
| Cached WS idle **5min**, max age **55min** | Pi-explicit values with differing lifecycle mechanism | COD:867–868; :1053–1054 tests max age; :1063–1073 schedules idle expiry timer and skips busy entries. Candidate next-idle-acquire eviction is a deliberate mechanism difference, not literal timer parity. Keep source-proven values, describe lifecycle honestly; never kill active generation from max age. |
| No permessage-deflate | Selected backend capability limitation | Tungstenite has no compression negotiation in selected release and rejects unnegotiated RSV bits, RUSTWS `protocol/mod.rs:635–644`; not a Pi policy. Pi global runtime may support extensions. Disclose backend support difference without adding an independent application refusal. |

**独立复审核正（OpenAI timeout）：** 原只读取证仅追到fetchWithTimeout清理timer，遗漏了client.ts:1008–1015→1033–1088的parseResponseWithTimeout；非流式JSON包括image stream=false会按startTime+timeout剩余预算约束body/解析。不得把这项真实后端限制当研究者新增值撤回；Mistral的headers-only结论不受影响。

OpenAI **64 KiB** in OAI `internal/decoders/line.ts:6–7,102–118` is a backing-buffer reclamation threshold. It does **not** reject long lines/events: :126–143 grows to fit active data. Do not cite it as an SSE event maximum. Likewise backend memory/allocation ceilings and JavaScript string/typed-array limits remain runtime dependent; no claim of unlimited real memory follows.

#### Streaming qualitative rejection inventory

1. Invalid JSON in a relevant Codex/Mistral/Pi event: Pi-explicit parse failure, COD:835–842, Mistral:516–520, Pi Messages:320. Keep adapter-specific parsing and unknown/control handling; do not reject legal keepalives merely for carrying no provider content.
2. Universal fatal SSE invalid UTF-8: **new acceptance difference**. COD:804,820; Mistral:464,476; Pi Messages:277,284 all create TextDecoder without fatal:true and therefore replace invalid byte sequences. Rust string representation/serde parsing can justify a declared language/backend difference, but cannot establish that Pi itself rejects those bytes. Preserve valid split UTF-8/CRLF and per-adapter decode semantics; do not impose stricter preflight just from a generic 'fail closed' slogan.
3. FRM safe nonnegative integer index, duplicate/gap start, wrong kind, absent/ended block, duplicate start/out-of-order start: Pi-explicit. FRM:94–98,328–365,374–388. Keep these state-machine checks; remove only arbitrary byte/count/depth maxima.
4. ResourceLimit retains last admitted partial and known producer usage, stops owned read/task, no post-handoff retry: useful failure/ownership semantics for an actually selected policy, not proof of the numeric policy. Keep known information and single result. Never replace a removed cap with hidden truncation.

### 2. Image numerical and qualitative inventory

Candidate locations: `spec-draft.md:119–124,359–364`; `sdk-target-proposal.md:102–146`; SDK contracts:121; RDF-06.

| Rule | Classification/evidence | Disposition |
|---|---|---|
| At most **8 images/direction**, decoded **16MiB/image**, **32MiB/direction**, text **64KiB/direction**, encoded request and decoded success JSON each **48MiB** | Unsupported researcher-selected resource values. IMG:138–164 maps every input item; :91–108 iterates first-choice images; no counts/bytes/text caps. | Withdraw **all**, including older64KiB/48MiB engineered values. Do not retain them merely because they predate new streaming values. |
| Shared default finite positive/no unlimited and pre-encode/decode caps | New Rust host policy, not Pi baseline | Explicit optional host budgets can be frozen and checked with typed errors. Omission must not secretly select these invented ceilings. |
| PNG/JPEG/GIF/WebP-only | Existing Opi **chat** MediaType enum: `crates/opi-ai/src/message.rs:55–66`; broader image-generation rejection is proposed policy. Pi `src/types.ts:430–433` has MIME string; IMG:101 captures any nonsemicolon MIME. | Do not extend existing chat enum restriction to a claim of Pi image wire hard limit. New image operation can preserve MIME metadata; format-specific codec requirements must be demonstrated. |
| Nonempty input-block list | Unsupported construct restriction | IMG:139 maps input and serializes empty list without local failure. Preserve mapping; do not invent a minimum prompt/images requirement. Image-only and empty text are accepted by mapping. |
| Strict base64, decoded bytes nonempty, MIME/container signature match | Proposed stricter behavior, partly representation issue, **not Pi validation** | IMG:101 only regex-validates data-URL envelope and forwards captured MIME/base64 text. Pi test `test/openrouter-images.test.ts:84–88` succeeds with PNG MIME + `ZmFrZS1wbmc=` ('fake-png'), so signature checking breaks a pinned passing fixture. Preserve MIME and encoded content/normalization by default. Decoding required solely to return owned bytes may fail malformed base64; identify that unavoidable representation difference rather than using it to justify signature/MIME allowlist rejection. Consider retaining encoded payload alongside optional bytes to preserve the observable result. Opt-in host strict-media validation can be separate. |
| At least **1 valid image**, no choices/text-only/empty/URL-only result failure | New Rust success criterion | IMG:91–111 returns stop even with no choice/no output/text-only; URI skipping can produce empty success. Remove as parity-default failure. Return what pinned parser returns with known usage; caller decides whether image absence fulfills its application goal. |
| Bad consumed image makes entire response fail; no skipping | New Rust rejection | IMG:99–102 explicitly continues for nondata URL or nonmatching data URI. Preserve skip semantics for those entries; accepted siblings and text remain. Do not redownload URLs or silently switch APIs. |
| Consume first choice only; output text then reported image order | Pi-explicit | IMG:91–108. Retain; extra choices don't invalidate first choice. |
| No automatic URL downloads/pixel decode/file writes | Pi-explicit absence of those effects / owned SDK boundary | Retain; preserving a URL string/skip result does not authorize downloading it. |

Historical candidate provenance: `decisions.md:238–242` D16/Q21 explicitly calls first resource boundaries **researcher design values**, not supplier limits; :242 calls64KiB/48MiB engineering detail. `decisions.md:271` D25/Q28 later confirms overridable budgets/no unlimited and base domain validation; `sdk-target-proposal.md:142,165` records stricter media policy. Do not rewrite this history as if it never happened. Record current user-directed reassessment as superseding unsupported parity-default values/extra rejections, while preserving historical approvals with their original scope/date. These are non-normative research choices, not formal mandatory image-validation clauses.

### 3. Classifier inventory

| Rule | Classification/evidence | Disposition |
|---|---|---|
| Shared **1–64 questions**, request/state **1MiB**, depth **32**, success JSON **2MiB** | Unsupported proposed defaults | SYS:86–95 serializes question entries, SYS:45–82 iterates requested keys; DEC:51–70 maps questions; no shared question/count/body/depth checks. CLS:89 calls next.json without this success cap. Withdraw all fixed defaults, including minimum1. |
| Shared choice **2–255 labels**, score min**2**, unique level strings | Unsupported across protocols | Pi types `src/types.ts:662–689` use Record/string[] and SYS/DEC mapping forwards; no global min/max/uniqueness check. Object map keys are inherently unique representation, not an extra label rejection. Withdraw. Do not force ordered score descriptions to be distinct. |
| Llama choice **2–62**, score **2–10** | Pi-explicit adapter algorithm limits | LLA:39–40,104–119. Retain only for llama; never transfer to System One/Decisions. |
| Decisions images <=**128** | Pi-explicit adapter check with service comment | DEC:32–33,73–88; test `openai-decisions.test.ts:168–182` rejects129 before send. Retain exact modality-specific scope. |
| Models image support check before auth; TypeSafe/Cloudflare/llama images unsupported | Pi-explicit | `src/models.ts:984–990`; SYS:116 applies both System One transports; LLA:437. Retain actual modality rejection even if caller forges metadata; don't add stricter bytes/MIME check to image inputs. |
| Bool/choice/score type, required requested answers, probabilities container shape, finite numeric checks | Pi-explicit | CLS:30–38; SYS:32–82; DEC:94–143. Retain typed/finite/missing/refusal checks with adapter-specific shape; these are **not** proof of range constraints. |
| probability/confidence in **[0,1]**, score within level index range, selected choice belongs to request criteria, exact probability labels/full distribution | Unsupported extra default validation | SYS:49–78 accepts any string choice and finite numbers; probability map parses its received entries without matching criteria. DEC:94–128 does likewise. Withdraw added domain/label-set rejection from parity baseline; optional host validation only. |
| Sum=1, selected choice=argmax, score mean/distribution requirement and tolerance constants | Unsupported extra validation; candidate had already partly withdrawn | Keep withdrawn. Pi finite parser has no such tests. Score needs only score/confidence shape for SYS/DEC; llama computes its own mathematically defined result. |
| Extra question IDs ignored | Pi-explicit | SYS:45 iterates request keys; DEC:133–143 builds map then reads request keys. Retain. DEC duplicate named answers are last-wins (:135); duplicate returned probability labels Object.fromEntries last-wins (:96–102). New duplicate rejection would also narrow Pi. |
| Llama temperature default**1**, finite>**0**; all question shape checked before first request | Pi-explicit | LLA:438–443. Retain; do not globally force temperature to affect System One where ignored. |
| Llama readout min**256**, **16** per label, retry depths**4096/32768**, missing/all logprob <=**−1e30**, single-token labels/distinct token IDs | Pi-explicit algorithm, not resource policy | LLA:43–48,303–332,405–421. Retain exact algorithm and typed failure. These cannot be removed as arbitrary Rust limits. |
| No usage -> unknown; already parsed usage preserved when answers invalid | Pi-explicit preservation, knownness has existing Rust differences | SYS:124–128 saves usage before answer parse; DEC does likewise at :185–190. Retain facts; no raw diagnostics needed. |

#### Timeout and retry scopes

- Image default **600000ms**: **backend-default adopted by Pi**, OAI `client.ts:608,1992`; IMG:67–81 passes the request timeout and disables vendor retries. Initial fetch timer clears at headers, but `responsePromise:1008–1015` invokes `parseResponseWithTimeout:1033–1088`, which races nonstream JSON body/parse against `startTime+timeout` remaining time. **Independent-review correction:** the initial headers-only claim was incomplete and is withdrawn. Keep the real body timeout; do not extend it to auth/hooks/all retries or to streaming paths that skip this wrapper.
- Shared classifier timeout: **Pi-explicit optional**, no default600000/60000ms. CLS:76 only creates AbortSignal.timeout if supplied; :82–89 wraps fetch plus body json in same attempt's signal. Retries refresh that timeout. Do not impose image semantics or total deadline.
- Mistral **60000ms** is source-proven headers default in its own adapter, irrelevant to image/classifier global policy.
- Image extra retry default **0**: IMG:70 disables SDK retry; outer provider helper default0. Classifier shared extra **2**: CLS:96; Decisions **504 exclusion** is DEC protocol-specific. Retain protocol-qualified rules.

### 4. Input-limit metadata is not blanket SDK rejection proof

Pi `src/providers/model-schema.ts:48–82` defines optional positive maxWidth/maxHeight/maxBytes/maxPerMessage/maxPerRequest/maxRequestBytes and JPEG quality **1–100**; maxBytes comment is **base64-encoded payload**. `src/types.ts:828` stores inputLimits. Search of `packages/ai/src` finds field definitions but no runtime enforcement of these request/image numeric metadata. Product/durable consumers preprocess images (e.g. `packages/durable/src/tools/read.ts:171–209` and its own DEFAULT_IMAGE_LIMITS2000x2000/base644.5MiB in `tools/image-processor.ts:11`). This is host behavior, not SDK image-generation defaults.

Therefore AIR-062/V-54 language requiring every metadata limit to reject before dispatch is a **new SDK behavioral policy**, unless its exact consumer enforcement is separately sourced. Preserve typed metadata/validation of supplied metadata and supported-modality selection; do not infer global mandatory image/request rejection or automatic resize from metadata field existence. No source-grounded need to revise formal authority for this documentation correction was found.

### 5. Recommended rewrite rule

Keep existing64-event backpressure, exact protocol structure/CRC/RFC/state-machine checks, pinned adapter algorithms and source-proven timeout/retry values. List selected dependency defaults/constraints accurately without upgrading them to Pi or protocol maxima. Withdraw every unsupported fixed number and newly strict media/classifier success criterion from the default alignment contract; optional explicit host policies can remain conceptually distinct without inventing a new configuration framework or selecting new numbers. Do not substitute arbitrary finite numbers merely to satisfy the word 'bounded'. Describe incremental parsing, cancellation-aware backpressure, one ownership path and checked allocation as mechanisms; acknowledge remaining backend/runtime limitations honestly.

## 取证附录B：鉴权与transport

## Auth, source, and transport parameter reaudit (source-only)

2026-10-16. No repository writes, installs, builds, runtime/live tests, credentials or metadata calls. Test impact: none. This is a bounded read-only handoff; recommendations do not grant runtime permissions or alter formal admission.

Source aliases (all exact local primary source):
- `P` = D:/Luiz/Odradek/opi/.repo/pi-1.2.0/packages/ai/src
- `D` = D:/Luiz/Odradek/opi/docs/research/pi-ai-opi-ai-alignment
- `O` = D:/Luiz/Odradek/opi/crates/opi-ai/src
- `G` = C:/Users/Luiz/AppData/Local/Temp/opi-adc-source/google-auth-library-10.6.2/package/build/src
- `N` = C:/Users/Luiz/AppData/Local/Temp/opi-parameter-npm-source (restored exact locked npm archives, parent verified integrity)
- `R` = C:/Users/Luiz/AppData/Local/Temp/opi-ws-backend-source
- `Ring` = C:/Users/Luiz/AppData/Local/Temp/opi-adc-source/ring-0.17.14/ring-0.17.14

Authority: docs/opi-spec.md:119 PRIN-004 protects selection/authority boundaries; :393-397 assigns explicit execution budgets to product/embedder while preserving bounded queues, provider retry limits and external deadlines. PRIN-003, GATE-001/002 and C3 authority remain unchanged. Bounded operation ownership and strict identity validation do not establish any particular resource constant or generic reject-all-3xx policy.

### Conclusions for the rewrite

1. Keep 15s as the Pi stored-OAuth refresh network signal, not a transaction ownership-to-persistence-ACK deadline. Keep real expiry, cancellable lock waiting, one rotation owner, cancellation-independent started rotation and honest ACK/unconfirmed outcomes. Remove invented universal deadlines for auth persistence and for all ADC.
2. Withdraw ADC total60s/16sends/depth4/1MiB and mutation wait30s/256keys as aligned defaults. Preserve optional caller policy, bounded concurrency/backpressure, checked arithmetic, cancellation and explicit pending state. Do not treat unbounded caller policy as an integer sentinel accidentally rejected by positive-only validation. Concrete memory-engine limits need a separate justified differential, not Pi attribution.
3. Preserve actual source-local defaults: executable30s and valid5-120s range, SSO5min/30s throttle, ECS1s/4send and legacy1send, OAuth network15s, Codex connect15s/idle5min/max-age55min. Do not put an invented60s outer ADC timer over a valid120s executable.
4. Blanket redirect:none (including generation, catalog, all token HTTP) and ADCsubjectGET same-origin5 are extra behavioral rejection. Keep redirect handling explicit and counted by the one owner; recover source-specific Pi/default behavior wherever within the already-authorized route/identity contract. An opt-in compatibility override does not make a restrictive default equivalent. Never re-resolve credentials, leak target secrets/model input to an unauthorized target, downgrade TLS verification, or treat an observed redirect as an ordinary transient retry.
5. H1-only, no OS roots, WS fixed handshake ceilings and lack of compression are backend/platform coverage differences. Document them as capability gaps or reconsider backend pairing/configuration; do not describe them as intrinsic Rust or Pi resource requirements. SOCKS/PAC rejection actually is Pi behavior in its explicit proxy helper.

### Parameter inventory

| Candidate location and rule | Classification and primary evidence | Correction |
|---|---|---|
| D/spec-draft.md:281; rust-backends.md:56; sdk-contracts.md:150,228: default15s rotation through ACK | Mixed. Pi-explicit `P/auth/resolve.ts:103,140` supplies independent15_000ms signal only to `oauth.refresh`. `:132-146` awaits store.modify outside that timer; `:110-114` ignores original cancellation after refresh starts. Store implementation `P/auth/credential-store.ts:14-27,40-55` serializes and awaits task with no15s ACK cap. | Preserve refresh HTTP scope; do not time out store ACK using remaining network budget. User/operator-specified persistence deadlines report unconfirmed effects and do not silently discard rotated tokens. Pi comments are less exact than the executable scope; cite code. |
| Same rows: one5min near-expiry window | Pi-explicit resolver5min at `P/auth/resolve.ts:102,169-182`; flow-level margins additionally exist, e.g. anthropic.ts:136,274 and github-copilot.ts:345. Current candidate already calls single-window an intentional difference. `G/auth/authclient.js:31,48` has Google5min window too. | Retain existing deliberate single-window semantics/real expiry and shortTTL behavior. Do not label it exact expiry parity or shorten a real expiry. |
| D/rust-backends.md:54: max256 unresolved resource keys; positive override; reject new mutation at capacity | Unsupported new Rust resource/rejection policy. Pi store maps provider credentials and promise chains without fixed numeric cap (`P/auth/credential-store.ts:9-27`). Pi has no barrier/receipt public API. | Withdraw fixed256 as default admission rule. Preserve keyed ownership and non-eviction of unknown outcomes; any bound must be caller policy or separately justified host/resource differential. Backpressure before effect may protect resources without pretending unconfirmed effects disappeared. |
| D/rust-backends.md:56-58; spec:389; sdk:228: barrier wait30s and relation to ADC60s | Unsupported new wait default. No Pi barrier. Correctness value is owned completion, not30s. | Keep owned completion/pending/aggregate results and cancel-only-wait semantics. Caller chooses wait deadline; no arbitrary default cutoff or claim that runtime Handle keeps runtime alive. |
| D/rust-backends.md:222 (ADC budget paragraph): total60s,16 HTTP sends, chain<=4, all files/subject/token bodies<=1MiB; positive-only override | Unsupported new Rust differences. `G/auth/googleauth.js:413` recursively calls fromJSON without depth4; `:545-554` accumulates credential input then parses without1MiB cap. `G/auth/urlsubjecttokensupplier.js:47-62` reads via transport without this body cap. Google STS/auth retry code below has no60s/16send whole-chain policy. | Withdraw all four fixed aligned defaults. Preserve cancellation and logical ownership; accept valid supported source chains/configs subject to explicit caller policy and actual parser/backend limits. Do not copy unbounded allocation internally; streaming/ownership is distinct from rejecting valid input at a guessed byte/depth cutoff. |
| D/rust-backends.md:215 external executable:30s default and5-120s configured | Pi backend-explicit: `G/auth/pluggable-auth-client.js:25,29,33,132-148`. | Retain. Remove truncation by unsupported ADC60s total. |
| Same row: stdout/stderr each64KiB | Unsupported new ADC restriction, not Google executable default. `G/auth/pluggable-auth-handler.js:71-79` accumulates stdout and stderr into one output string without64KiB. | Withdraw64KiB default for ADC; preserve pre-growth accounting and explicit host policy if separately requested. Do not infer Google behavior from AWS's existing-opi process cap. |
| Same row: envALLOW_EXECUTABLES=1 plus trusted opt-in/runner, deny before cache read | Env gate and cache-before-spawn ordering are backend-explicit: `G/auth/pluggable-auth-client.js:168-181` checks env before output_file. Extra independent trusted opt-in/default-missing runner is a Rust authority policy, not Pi behavior. Direct argv spawn/quoted parsing is actual Node behavior (`G/auth/pluggable-auth-handler.js:68-70,157-165`). | Retain required formal trusted-runtime/host authorization; document source selection plus the required env gate without manufacturing another independent parity gate. A standard supported native runner must be reachable within authorized execution; default Unsupported is a capability gap. Do not automatically grant execution for untrusted credential documents. |
| D/rust-backends.md:182 AWS credential_process10s/64KiB, stdout/stderr each | Existing-opi: `O/bedrock/credentials.rs:536-543` documents10s and64KiB **stdout**, implementation `:643` bounds stdout; stderr is null in current implementation, so each-stream claim is new. Exact Pi Node provider `N/@aws-sdk__credential-provider-process-3.972.70/package/dist-cjs/index.js:41-46` calls `execPromise(credentialProcess)` with no explicit timeout/maxBuffer. Node exec defaults are runtime defaults, not these values. | Preserve existing intentional behavior unless separately authorized to change; classify correctly. Do not extrapolate to ADC. If stderr becomes collected, its handling is new resource policy. |
| D/rust-backends.md:184 SSO5min advance/30s throttle; valid-old-token fallback | Backend-default, exact `N/@aws-sdk__token-providers-3.1116.0/package/dist-cjs/index.js:20,105-111,117-139` (3.1127.0 same). New token cache write errors swallowed at :122-131, unlike candidate explicit ACK. | Retain actual timing; retain honest cache ACK as intentional correctness strengthening, not Pi proof of persistence. A failed refresh may preserve truly unexpired oldtoken per source; not fake expiry. |
| D/rust-backends.md:186 ECS1s attempt and modern<=4 sends/legacy1 | Backend-default: modern `N/@aws-sdk__credential-provider-http-3.972.72/package/dist-cjs/index.js:145-162` uses timeout1000, maxRetries3; loop :94-105 performs3+1 calls and waits1000 between failures. Legacy/IMDS `N/@smithy__credential-provider-imds-4.5.2/package/dist-cjs/index.js:19-21` default1000/maxRetries0; :57-79 runs1+N. | Retain source-qualified budgets, including wait scope. Do not turn every AWS auth call into generation retry policy; one scheduler can own separate source-local retry episodes. |
| D/rust-backends.md:222 metadata1s attempt | Not proven by generic GoogleAuth source itself; exact metadata authority is pinned gcp-metadata8.1.2 (Pi root package-lock:2903-2909). AWS1s is not Google proof. | Preserve only if exact gcp-metadata source validates the relevant token request scope; otherwise mark evidence gap, not measured/aligned fixed default. |
| D/rust-backends.md:173,175 XML body/query1MiB,depth32,tokens16384,attributes4096/256 | Unsupported new numeric SDK rejection. XML well-formedness/UTF-8, checked lengths, no external entity I/O and fixed STS field/schema requirements are separate legitimate boundary validation. No primary Pi evidence for these numeric ceilings. | Withdraw fixed caps as aligned defaults. Retain malformed/ambiguous credentials rejection, no external resource reads and checked preallocation; source protocol constraints remain owned by exact STS schema. |
| D/rust-backends.md:203 JWTclock+3600,RS256, canonical token endpoint | Backend-explicit: `G/gtoken/jwsSign.js:20-22,29-34,48-51` and `G/gtoken/getToken.js:19,27-38`. Ignores JSON token_uri here. P12 rejection is genuine `G/gtoken/getCredentials.js:84-94,114-116`. | Retain these specific facts; do not invent P12 compatibility. NonRSA/encrypted/oversize key rejection must be classified by actual selected parser/crypto capability. |
| Same: ring rejects weak/oversize RSA, PKCS#1/8 | Backend limitation: `Ring/src/rsa/keypair.rs:177-180` modulus at least2047/no larger4096 and exponent constraints; :269-284 stricter actual accepted construction. These are ring limits, not generic Google protocol numbers. | Retain honest backend errors but record potential key-family/range capability gap; an opt-in budget cannot lift a crypto backend hard limit. |

### Transport behavior and backend limitations

| Candidate restriction | Exact evidence/classification | Recommendation |
|---|---|---|
| All owned generation/token/AWS/ADC/metadata/catalog redirect:none and other3xx terminal (D/rust-backends.md:64-72; spec:395; sdk:232) | Extra Rust blanket rejection. Existing-opi `O/http.rs:202-214` builds reqwest without redirect override; exact reqwest0.12.28 `R/reqwest-0.12.28/src/redirect.rs:160-164` defaults limited10. Pi direct fetch Mistral `P/api/mistral-conversations.ts:308-320` and SDK delegation OpenAI `P/api/openai-responses.ts:178-184,297` do not opt out. OpenAI exact client `N/openai-7.19.0/package/src/client.ts:1628-1642` delegates request init to selected fetch without generic redirect:error. ADC Gaxios evidence next. AWS Node rawhttp source normally has no fetch redirect; protocol-qualified behavior must remain distinct. | Remove universal no3xx default as alignment requirement. Keep manual/owned redirects visible to send accounting where needed, with method/body/auth semantics from selected backend. Never use a redirect to select a different account/source. Legitimate protected destination/secret constraints may require a declared differential; do not remove them merely to reproduce unsafe cross-origin body forwarding. An explicit restrictive redirect policy can be caller/host policy, not unacknowledged baseline. |
| ADC subject GET same-origin only,5 redirects, no userinfo/fragment | New Rust policy. `G/auth/urlsubjecttokensupplier.js:47-62` passes GET URL+explicit headers to Gaxios. `N/gaxios-7.1.4/package/build/cjs/src/gaxios.js:109-117,329-330,526-530` uses node-fetch and only sets follow when maxRedirects supplied. `N/node-fetch-3.3.2/package/src/request.js:126,134` defaults follow/20; index.js:199-212 strips selected sensitive headers on other domain/protocol, :222-226 converts POST301/302/303 toGET. Node rule even allows exact/subdomain auth preservation, not strict same-origin/port. | Withdraw guessed5 and sole-origin parity claim. Source-owned transport should preserve actual authorized subject source redirect semantics/default, not wrap only one narrow exception. Explicit headers can contain secret source tokens: protecting them remains a real boundary. No target-model credential may be added to subject supplier requests. |
| Disable nested reqwest retry | Legitimate one-owner mechanism, exact reqwest `R/reqwest-0.12.28/src/retry.rs:11-14,81-84` retries known-safe protocolNACK by default; disabling it avoids double accounting. H2-feature dependent paths at :270ff; current independent H1 build need not have exercised them. | Retain retry(never) on the lowlevel engine, while moving source-specific actual retries into the selected owner. No implication that every auth source is retry0. |
| ADC default0 auth retries, only opt-in finite auth policy | Capability/reliability narrowing. `G/auth/oauth2common.js:139-144` enables retries includingPOST; STS uses it at `G/auth/stscredentials.js:76-88`; OAuth refresh `G/auth/oauth2client.js:252-259`; service account `G/gtoken/getToken.js:36-37` supplies retryConfigPOST. Exact Gaxios retry.js:18-24 defaults enabled retry3; no-response2 :32-35; total/max delay MAX_SAFE_INTEGER :42-47; retry predicate/status ranges :50-64. | Preserve source-specific auth retry defaults under same owner; generation remains separately0/2/etc as applicable. Remove unsupported total16/60s. Keep cancellation/frozen identity, do not retry callbacks or arbitrary definitive auth failures. |
| GlobalH1 ALPN; H2 enablement forbidden through consumer features (backend:125) | Initial chosen transport capability difference, not Pi default. Pi Bedrock `P/api/bedrock-converse-stream.ts:220-231` explicitly says defaultH2, selectsH1 for proxy or FORCE_HTTP1. reqwest provides HTTP version config; backend is capable ofH2 with selected features. | Reconsider H2 support for ordinary Bedrock/default transport or document unresolved parity gap. Preserve H1 for explicitly chosen proxy-placeholder mode and selected classic WS Upgrade; those do not justify globalH1. No automatic authority expansion from unrelated cargo feature unification. |
| HTTP(S) proxy only; rejectSOCKS/PAC; scope/lowercase/ALL_PROXY/NO_PROXY rules | Actual Pi helper contract `P/utils/node-http-proxy.ts:13-21,74-135,138-157`; Pi Bedrock and BunWS use it. | Retain; not an accidental extra restriction. Do not blanket-claim every ambient HTTP client uses that helper; actual HTTP/WS/AWS/ADC wiring needs conformance. Frozen scoped resolver and no double wrapping custom transport are legitimate isolation. |
| webpki plus explicitDER roots; never OS roots; no ambientextraCA (backend:125) | Selected roots/backend policy; not Pi-wide behavioral equivalence. Pi delegates TLS to Node/global fetch/Node HTTPS agents; no explicit root pinning in the inspected Pi adapter. ADCcert `G/auth/certificatesubjecttokensupplier.js:69-73` uses newhttps.Agent(key,cert) with Node root handling. Rust explicit per-instanceconfig/global-provider avoidance is sound ownership, but root source restriction can reject corporateTLS endpoints accepted by runtime. | Preserve certificate/hostname validation and no secret/identity fallback. Make the trust-store policy explicit, support required host-provided roots through existing proven seam, and mark OS/runtime root coverage as real default difference until implemented. Do not claim a manual override makes default equal or silently read ambient roots contrary to selected trust policy. |
| TLS1.2/1.3, SNI/server verification, no earlydata | Selected backend/current-safe defaults: `R/rustls-0.23.40/src/versions.rs:24-46` supports1.2/1.3; reqwest default SNItrue/verificationtrue/noearlydata in async_impl/client.rs:293-340. | Retain validation and supported secure protocol choices; classify as backend behavior. No evidence that older insecure versions are needed for Pi parity; no widening proposed. |
| CONNECT <=64KiB/124headers/15s coveringTCP/TLS/CONNECT/Upgrade (backend:137) |15s connect is Pi-explicit (`P/api/openai-codex-responses.ts:57,1080`), but a customCONNECT64KiB/124 header cap and using WS attack counters as proxy response limits are new choice. Backend WS itself has hard64KiB/512reads and smallread heuristic (`R/tungstenite-0.28.0/src/handshake/machine.rs:169-185`) and124headers (handshake/headers.rs:10, client.rs:303). | Retain source-qualified connect deadline; withdraw new proxy-parser numeric limits as aligned policy. Document backend hardlimits as capability gap that cannot be lifted with WebSocketConfig. Avoid promising configurable compatibility the backend cannot provide. |
| WS read/write64KiB,maxwrite32MiB,receive8/16MiB,outgoing16MiB,close2s | New resource/timing defaults. Backend defaults are read/write128KiB,maxwriteusize::MAX,recvmessage64MiB/frame16MiB (`R/tungstenite-0.28.0/src/protocol/mod.rs:98-102`). Pi closes immediately with close() helper and no2s flush guarantee (`P/api/openai-codex-responses.ts:1057-1060`). RFC control<=125 and nofragment/UTF8 are protocol-hard, distinct from resource defaults. | Stream-budget auditor owns final numeric policy; these are not Pi constants. Cleanup localownership/awaitdriver must remain honest; arbitrary2s close cap not required for Pi parity. |
| No compression, explicit3xx/non101 handshakefailure | tungstenite selected backend capability; Pi uses runtimeglobalWS (`P/api/openai-codex-responses.ts:1023-1025`) so runtime-qualified negotiation, not universally a checkedPi no-compression fact. | Record backend gap, not universal Rust requirement. Do not advertise unsupported extensions. Non101 is correct classicUpgrade failure, but whether a runtime follows redirects before Upgrade is separate. |
| Custom transport single-send frozenrequest, not wrapped | Legitimate adapter responsibility/one-owner design, not proof of opaque third-party behavior. | Retain explicit conformance responsibility; support source-specific owned retries/redirects above it instead of forcing all valid SDK workflows into single-wire-send with no authority to continue. |

### Qualitative source and authority boundaries to retain

- Frozen account/source/config, no source substitution after token rotation or generation retry; source file/path/parse failure cannot silently switch an explicitly selected identity. Distinguish Node chain Continue from a completed source-selection boundary: `N/@aws-sdk__credential-provider-process-3.972.70/.../index.js:53-64` wraps process failures as chain provider errors; do not indiscriminately convert every discovery failure into terminal once the profile is merely inspected.
- ECSorIMDS is a branch, not fallback after selectedECS exhaustion; actual chain source in restored credential-provider-node is the authority. Partial scopedcredential pair cannot be combined with ambient secrets. Bearer/SigV4 exclusivity, placeholder mode's zero realcredential discovery and explicit endpoint/region remain deliberate safety contracts.
- Google externalcertificate source is real existing capability, not an optional future callback: `G/auth/identitypoolclient.js:70-100`; supplier mutually exclusive config at :55-60, standard path fallback :94-118, key/cert read at :124 onward. Keep mTLSidentity isolated by selected source, normalserverTLS validation, and no accidental identity pooling.
- Canonical token endpoint for ordinary serviceaccount/authorizeduser is not evidence to reject selected external-account token_url/universe/impersonation endpoint. External configurable URL fields in `G/auth/baseexternalclient.js:113-131` and externalAccountAuthorizedUserClient.js:117 are real contract data; apply trustedconfig validation without silently replacing supported endpoints.
- Models configuration check requiring file/project/location is separate from directGoogleAuth metadata capability. Pi's provider `P/providers/google-vertex.ts:74-85` checks configured prerequisites; `G/auth/googleauth.js:145-162` has project discovery, includinggcloud. Candidate explicitlyresolved modeltarget can avoid unnecessarygcloud, but do not rename missing direct/default entry points as parity. Hostpermission and trust gates are retained; audit does not authorize reading ambient homes/spawninggcloud automatically.

Open evidence gap: Google metadata1s exact runtime source not read in this bounded audit. Some Pi-global-fetch/WS behavior depends on the selected Node/Bun runtime rather than an npm archive; do not invent one universal limit/default. Exact AWSlogin/SSOprocess standard source details beyond the rows above remain the main audit's responsibility. Old opi-sdk-source directory was empty; restored N archives supplied the evidence used here.

## 取证附录C：Schema、自定义API与生命周期

## Read-only audit: schema semantics, custom registration, telemetry and lifecycle

Date: 2026-10-16. Repository: D:/Luiz/Odradek/opi. No repository files were written. No build, installation, live call or test execution was performed. Test impact: none. This report is source-derived evidence for the parent documentation revision, not implementation admission.

### Sources and authority

Read AGENTS.md, the formal docs/opi-spec.md and the check-selection reference. Formal GOAL-003/§3.2–3.3 permits Rust-native ownership/typed interfaces but does not make arbitrary extra input rejection a language requirement. PRIN-002/003 still applies to newly proposed seams. docs/opi-spec.md:257–259 (CTRL-001/003) supports exporter independence and classified/redacted evidence. Research annexes do not create parent authority.

Abbreviations used below:

- `S` = docs/research/pi-ai-opi-ai-alignment/spec-draft.md; its Chinese counterpart has the same relevant line numbers.
- `B` = docs/research/pi-ai-opi-ai-alignment/pi-1.2.0-rust-backends.md.
- `R` = docs/research/pi-ai-opi-ai-alignment/pi-1.2.0-rust-differences.md.
- `PI` = .repo/pi-1.2.0/packages/ai/; `PT` = .repo/pi-1.2.0/packages/telemetry/.
- `JS` = C:/Users/Luiz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/jsonschema-0.28.3/.
- `FR` = the same registry root, fancy-regex-0.14.0/.
- `REF` = the same registry root, referencing-0.28.3/.
- `TB` = C:/Users/Luiz/AppData/Local/Temp/opi-parameter-npm-source/typebox-1.3.27/package/build/. Parent restored the exact official npm artifact and reported verification against pinned package-lock integrity. This agent inspected the restored sources; it did not independently download or execute them.

Candidate line references describe the pre-correction content read during this audit; the parent began concurrent documentation corrections near report completion, so review final anchors against the outgoing diff. The parent should synchronize S/Chinese, B, R, acceptance matrix and any current summary references. Historical evidence should preserve its original state and identify the superseding correction.

### Numeric inventory and disposition

| Candidate parameter | Current location | Classification and evidence | Correction |
|---|---|---|---|
| schema + resource bytes 256 KiB | B:87, S/Chinese:403 | Unsupported SDK policy. PI/src/utils/validation.ts:271–279 passes the schema directly to Compile; no byte ceiling. TB/compile/compile.mjs:6–11 and schema/build.mjs:105–117 impose no such byte ceiling. | Withdraw fixed value and boundary tests; do not call it Pi or backend default. |
| schema nodes 16384 | B:87, S:403 | Unsupported SDK policy; same evidence. | Withdraw. |
| 256 resources/ref targets | B:87, S:403 | Unsupported SDK policy. TB/schema/resolve/resolve.mjs resolves supplied in-memory context/root; no count256. | Withdraw. |
| schema and instance depth64 | B:87, S:403 | Unsupported SDK policy. No such validator bound in the cited paths. TypeBox maxInstantiationCount128 is a different type-construction mechanism, not this. | Withdraw; do not replace with 128. |
| instance8 MiB | B:87, S:403 | Unsupported SDK policy; no validation instance-byte maximum in Pi/TypeBox entry path. | Withdraw. |
| instance262144 JSON nodes | B:87, S:403 | Unsupported SDK policy. | Withdraw. |
| pattern4 KiB | B:87, S:403 | Unsupported SDK policy. TypeBox constructs native RegExp from supplied pattern (TB/schema/engine/_regexp.mjs:1–4); FR does not specify a 4KiB pattern-string maximum. | Withdraw. |
| work admission1000000 total units | B:88,91,93; S:403 | Unsupported SDK policy/algorithm. No engine-wide fuel/cancel callback in ValidationOptions. This is unrelated to FR regex backtracking. A conservative-bound-required admission rule rejects inputs for inability to prove cost even when valid/fast. | Withdraw value and compulsory proof/prewalk rejection, leaving an implementation evidence gap rather than claiming equivalence. |
| cache128 compiled validators | B:86; S:403 | Unsupported SDK policy. Pi cache is WeakMap keyed by schema object (PI validation.ts:6,271–279). JS pattern.rs:15 has a *global regex cache10*, not a per-Models compiled-validator cache128. | Leave cache optimization/eviction implementation-specific; correctness must not depend on a stale mutable address. No invented fixed capacity. |
| cache retained source32 MiB | B:86; S:403 | Unsupported SDK policy, distinct from opaque engine heap. | Withdraw. |
| active jobs4 / queue16 | B:89; S:403 | Unsupported SDK scheduler policy; no Pi/TypeBox or jsonschema numeric source. | Withdraw fixed values; describe cancellation honestly if an eventual worker implementation is selected. |
| collected errors32 | B:88; S:403 | Unsupported SDK policy. TB/system/settings/settings.mjs:5,16 has maxErrors8; schema/engine/_context.mjs:113–119 stops appending at that count without making a valid input invalid. Current Opi Agent reports one masked error (crates/opi-agent/src/validation.rs:15–19). | Withdraw32. If diagnostic parity is intended, evaluate actual8 default separately; avoid conflating error truncation with resource rejection. |
| regex backtrack1_000_000 | B:93 | Real *backend default*, FR/src/lib.rs:529–537 and569–576. Limits backtracking only for fancy patterns, per matching execution; not total schema work, total matches, bytes or CPU milliseconds. | May document precisely as backend difference; does not support work1million or a global guard. |
| TypeBox maxErrors8 | TB system/settings/settings.mjs:5,16 | Real upstream dependency diagnostic default. Can be changed by TypeBox Settings. | Preserve source fact, not a new mandatory universal SDK count. |
| TypeBox maxInstantiationCount128 | TB system/settings/settings.mjs:6,17; type/engine/call/instantiate.mjs:23 | Real type-engine generic instantiation guard; not used by schema validation as a node/depth/cache/work limit. | Do not repurpose. |
| jsonschema regex cache10 | JS src/keywords/pattern.rs:15 | Real backend internal optimization, global process-local static. | May record implementation fact, not a user-input maximum or validator capacity. |
| queue64 events | AIR-005/S:85; R:74–81 | Existing Opi choice, not Pi. Current opi-ai adapters use mpsc::channel(64), e.g openai_chat.rs:1513, anthropic.rs:1422, openai_responses.rs:563. Pi FifoQueue/EventStream unbounded (PI src/utils/event-stream.ts:3–22,43–57). | Retain as existing Opi resource behavior with differential impact. Does not justify new byte limits. |
| WS idle5min/max-age55min/connect15s | B:143 | Pi explicit constants PI src/api/openai-codex-responses.ts:57,867–868. | Retain source-qualified. |
| WS close grace2s | B:143; R:159 | Unsupported SDK policy; Pi helper simply close()s, PI Codex:950–963. | Withdraw fixed grace until source/measurement/user policy warrants it. Awaited local cleanup can remain separate ownership design. |
| credential barrier wait30s | S:182,226,389; B:56 | New SDK policy, no Pi mutation_barrier public API. Pi refresh timeout15s is a different operation. | Withdraw fixed default; parent auth audit owns full receipt/barrier correction. |
| unresolved receipt keys256 | B:54 | New SDK rejection policy; no Pi counterpart. | Withdraw fixed value/new rejection absent demonstrated need. |
| OAuth refresh15s | PI auth/resolve.ts:102–103,140 | Pi explicit *refresh network* timeout; Rust total transaction through ACK15s changes the covered interval (R:132). | Retain only exact original boundary or label source-backed lifecycle difference; never derive barrier30s from it. |

The general 'positive values only / zero and unlimited forbidden for every budget' rule (B:17; S:355) is an additional SDK policy, not proved by these sources. Checked arithmetic and safe failure on actual overflow are sound correctness requirements, but do not authorize arbitrary ceilings or reject a protocol-defined zero sentinel. This distinction also matters outside this sub-audit.

### Schema qualitative restrictions

#### Default draft, allowed dialects and mixed keywords

Candidate B:76,80 and S:379,401 impose Draft7 without `$schema`, only Draft7/2019-09/2020-12, conflict/unknown URI/mixed-resource rejection. These are not jsonschema defaults: REF/src/specification/mod.rs:18–30 defaults to Draft202012 and supports Draft4/6 as well; JS options.rs:58–80 explicitly prefers selected draft, then detects/defaults, possibly using retriever for unknown specifications. JS compiler.rs:283–299 checks draft-specific metaschemas.

Pi does not select a JSON Schema draft in validateToolArguments. It calls Compile(schema) without context/options (PI validation.ts:271–279). TB/compile/compile.mjs:6–11 uses empty context; TB/schema/build.mjs:105–117 builds directly, with no metaschema validation. TB/schema/engine/schema.mjs:134–249 and257–302 dispatches recognized keywords from multiple eras together (legacy tuple items/dependencies as well as prefixItems/dependentRequired/unevaluated/dynamic refs), without a dialect-wide allowlist. `$schema` affects a narrow legacy-reference framing case (TB/schema/resolve/resolve.mjs:211–219); that is not a general draft assertion/keyword switch.

Therefore withdraw the selected backend's conformance/equality claim and blanket UnsupportedSchemaDialect/UnsupportedSchemaKeyword policy. Keep actual differences explicit pending a source-defined corpus and mechanism that preserves the selected plain-JSON behavior. Do not silently ignore a meaningful Pi constraint just because the replacement backend ignores it; that is a capability gap. Unsupported is not automatically permission for a new default rejection.

#### Unknown keywords and annotations

B:81 restricts preserved annotations to standard annotations/x-* and rejects unsupported/other-dialect validation keywords. TypeBox schema dispatch enumerates recognized checks and does not reject arbitrary extra keyword names (TB engine/schema.mjs:134–249). JS compiler.rs:377–379 treats unknown keywords as annotations by default. Blanket rejection changes both upstream and selected backend behavior. Leave annotations inert without falsely claiming a constraint executed; identify actual known keywords with unavailable semantics as unresolved coverage.

#### Formats

B:76,82/S:401 default should_ignore_unknown_formats(false) is an invented stricter policy. Exact TypeBox Format.Test is `formats.get(format)?.(value) ?? true` (TB/format/_registry.mjs:64–66), so unknown formats pass. Defaults include uuid, url, json-pointer-uri-fragment (same file:85,93–94), independent of a declared draft.

Jsonschema default ignores unknown formats (options.rs:47,367–377). Its format validation default is draft-dependent (compiler.rs:131–135); candidate always-on assertion is an override. JS keywords/format.rs:775–805 includes uuid only >=2019-09 and has no url/json-pointer-uri-fragment. Blanket annotation mode is also not full Pi equivalence because it turns off known constraints TypeBox does assert. Suppressing every format is not an acceptable way to hide a validation gap. Preserve known/unknown semantics in the corpus; withdraw universal UnsupportedSchemaFormat and the default assertion/AnnotationsOnly policy as if upstream required it.

#### Content keywords

B:81 says contentEncoding/contentMediaType/contentSchema are annotations. Exact TypeBox schema dispatch has no content handlers (TB engine/schema.mjs:134–249), so this observation is consistent with TypeBox. But selected jsonschema Draft6/7 *does execute* contentEncoding/contentMediaType (JS keywords/mod.rs:318–324); keywords/content.rs:275–278 contains failing base64/JSON-content cases. Thus the candidate's selected Draft7 constructor does not implement its annotation statement unless it explicitly disables those checks. Treating content keywords as assertions would add rejection relative to Pi; dropping other actual constraints is the opposite error. This needs explicit compatibility handling/evidence, not a blanket statement about engine equality.

#### References and implicit I/O

Retain no implicit schema I/O. TypeBox resolution consults in-memory context/root only: TB/schema/resolve/resolve.mjs:179–200,265–269,303–313; the Pi Compile(schema) invocation supplies empty context. No HTTP or file fetch occurs there. Remote-looking identities are still legal local `$id`/ref identities. Restricting by the URI scheme alone would break valid in-document identity resolution.

JS DefaultRetriever may perform blocking HTTP/file reads when features are enabled (src/retriever.rs:15–45); explicit static-only retriever is a good mechanism to preserve no hidden I/O even after Cargo feature unification. Caller-supplied immutable resources can be a Rust extension; do not require an invented256-target cap, forbid mixed resources without evidence, or claim all TypeBox resolution details are reproduced. Resolve failures must retain facts, not silently skip constraints or download schemas.

#### Recursion and graph admission

B:84/88 and S:401 impose productive-only recursion and blanket non-descending-cycle admission rejection. TypeBox has explicit local, recursive and dynamic ref support (TB schema/engine/ref.mjs:23–38, recursiveRef.mjs:8–17, dynamicRef.mjs:8–17), but no such productive-cycle admission pass. Jsonschema also has recursive support; src/keywords/ref_.rs:606–608 specifically accepts `{"$ref":"#"}` with42, and compiler.rs:358–359 handles same-location infinite ref as true for older-draft branch. A cycle can also be irrelevant to the instance/selected branch; no blanket rejection is justified by Rust. Withdraw it as parity, retain actual undefined/nontermination/safety evidence separately; do not claim every arbitrary recursive schema is safe.

#### Regex guard and canonicalization

The backend error-loss finding is real. JS keywords/pattern.rs:117–134 returns a typed regex error in validate(), but is_valid() uses unwrap_or(false) at143. pattern_properties.rs uses unwrap_or(false) at56,73,89,104,159,174,188,201; any_of.rs:57–58 uses boolean nested is_valid; one_of.rs:46,62 does likewise. Therefore simply wrapping top-level validate does not establish reliable propagation of every nested regex runtime failure.

The proposed all reachable patterns × all candidate strings/keys guard (B:91–93, S:403, R:122) is not a valid general fix. It matches patterns against unrelated properties and branches, including branches the validator never needs to evaluate. A backtracking failure on such a pair would reject an otherwise valid input. It also adds an unproved graph/cost engine and a second private ECMA translator, while claiming exact engine coverage and idempotence not shown by tests. Backend ecma conversion is private best-effort code (JS ecma.rs:5–8,10–69), not a public canonicalization promise. Its actual translation differs from ECMAScript in some cases; sharing a crate version alone cannot prove semantic equality.

Withdraw the guard/canonical-copy/allOf collision-rewrite contract and backend semantic-equivalence claim. Preserve the error-loss finding as an implementation blocker; the eventual validator needs a proven relevant-evaluation error strategy or a different conformant mechanism. Do not treat inability to prove safety of an arbitrary pattern as a newly authorized global rejection. Keep FR's actual per-match backtracking limit as a backend fact only.

#### Plain normalization and final validation

Pi evidence supports clone, optional-null special handling, preserved already-valid union arm, ordered per-arm copies and primitive conversions (PI validation.ts:59–268,317–338; tests validation.test.ts:64–211). Exact integer preservation is a Rust representation difference; numeric strings use ECMAScript Number/String conversion. Strict wire conversion is a different operation and its restricted subset must not be imposed on local validation (S:333/B:85).

PI validation.ts:332 can return original root args when a changed root fails validation. S:331's mandatory full final original-schema validation fixes that bug; classify this as an explicit correctness change with a negative fixture rather than claiming byte-for-byte upstream behavior. Current Opi Agent already fully validates and masks an error (crates/opi-agent/src/validation.rs:10–20). The new library utility does not grant execution or authorization.

### Source-derived positive incompatibility corpus (not dynamically executed)

Use these as future evidence cases, preserving complete schemas/values. They are derived from exact source; this audit does not claim a differential run passed.

1. `{type:'string', format:'unknown-example'}` with `'ok'`: TypeBox passes unknown formats by registry fallback; candidate rejects at admission.
2. `{type:'string', format:'uuid'}` with a valid UUID and no `$schema`: TypeBox default uuid check is available; candidate defaultDraft7 declares it unsupported.
3. `{type:'string', format:'url'}` with `'https://example.test/'`, or json-pointer-uri-fragment with a valid fragment: TypeBox has these default formats; selected jsonschema known-format table does not.
4. `{$schema:'urn:custom', type:'string'}` with `'ok'`: TypeBox build dispatches type and does not select/reject a dialect; candidate rejects unknown URI.
5. `{type:'array', items:[{type:'number'}], prefixItems:[{minimum:1}]}` with `[2]`: TypeBox executes both old tuple and new prefixItems keywords; candidate forbids cross-dialect keyword use. See TB schema/engine/items.mjs:8–32 and prefixItems.mjs:7–23. A single selected jsonschema draft cannot be presumed to apply both.
6. `{type:'string', contentEncoding:'base64'}` with `'asd'`: TypeBox has no contentEncoding check; selected defaultDraft7 jsonschema rejects (its content.rs:275 has precisely this failing vector).
7. `{type:'string', pattern:'^\\s$'}` with U+1680 OGHAM SPACE MARK: TypeBox uses `new RegExp(pattern,'u')` (TB schema/engine/_regexp.mjs:1–4), whose ECMAScript whitespace includes U+1680. JS ecma.rs:122–125 rewrites whitespace to an explicit incomplete class without U+1680; source-derived backend mismatch. This illustrates why a copied translator/guard is not an equality proof.
8. Unknown metadata names in a schema with otherwise valid type/value: TypeBox ignores them; global unknown-keyword rejection would fail them. Distinguish such inert metadata from recognized constraints with unavailable semantics.

### Custom API/typed registration restrictions

| Candidate behavior | Evidence/classification | Recommended treatment |
|---|---|---|
| Closed Chat/Image/Classifier, extensible API identity | S:407; B:99. Pi has ModelType-specific operations and open Api/ImageApi/ClassifierApi strings, PI types.ts:61–85,311–320. | Retain typed operation vocabulary/open identity; do not close extension by builtin enum alone. |
| ApiId nonempty and 'normalized validated' | B:99；Pi Api is arbitrary string intersection; registration Map.set(provider.api) has no normalization/empty check (compat.ts:128–139; images-api-registry.ts:38–48). Current Opi rejects whitespace-empty provider ID only (registry.rs:147–154), not arbitrary ApiId grammar. | Withdraw unspecified normalized-ID acceptance policy. Preserve identity byte-exactly; specify only necessary dispatch association. Do not infer case/Unicode/whitespace restrictions. |
| O:Send+Sync+'static, fixed-O dyn adapter, opaque options | B:101–111/S:407；Rust interface/ownership design, not Pi protocol restriction. Pi custom options fall back to StreamOptions & Record<string,unknown> (types.ts:295–301). | Can retain as explicit language/API choice while proving a real custom adapter and borrowed/owned caller migration; no sandbox claim. |
| Type/API/operation mismatch pre-auth failure | B:113/S:409；Actual Pi runtime API mismatch check exists in compat.ts:104–124 and images-api-registry.ts:26–35. Earlier failure/typed error is Rust boundary change. | Retain associated type safety without broadening into unsupported content rejection. |
| Namespace tag/foreign handle rejection | B:111–113；New instance-scoping policy, not Pi registry behavior. Pi ordinary Models use provider values; compat/image registry is global. | Mark as Rust ownership choice only if needed; do not treat safe same-type reuse as inherently invalid or claim Pi demands it. |
| Stale handle always fails new call after replace/delete | B:113/S:409/R:50；New rejection policy. Pi registries return plain provider closures (compat.ts:142–143); previously obtained closure can still be invoked after Map replacement/removal. Models set/delete merely mutate Map (models.ts:410–425). | Withdraw blanket stale rejection. Freeze route/options consistently when a call is created; decide handle capture/current-lookup semantics from actual interface ownership, not an extra global freshness gate. |
| Duplicate registration policy unspecified | B:113；Pi registration upserts by Map.set; current Opi provider registration also replaces (registry.rs:30–34,156–158; provider_collection.rs:378). | Do not introduce duplicate-ID rejection absent explicit owning contract. |
| Only handle-built opaque custom options/no Serde raw executor | B:111；Valid native Rust representation decision, not TypeScript source/API equivalence. | Retain if dynamic functionality remains reachable and independent consumer proves it. |
| Full frozen prepared record, no model-ID relookup | B:115/S:409；Consistent with formal INV-001 and selected owned binding; complete custom models should not require catalog membership. | Retain; no arbitrary extra model-record rejection. |
| Adapter is trusted code, all public callbacks/cancel/telemetry still reachable | B:117；Correct honesty boundary. Pi functions are trusted process code too. | Retain; do not claim SDK can sandbox arbitrary Rust adapter I/O. |

### Telemetry and lifecycle audit

- Explicit telemetry option propagation is Pi-backed: PI types.ts:157–160 and test/telemetry-options.test.ts:68–165 prove identity preservation across simple mapping, Models/native/deferred/image paths. They do not prove automatic SDK spans, exporters, mutation protection or semantic settlement.
- PT/src/index.ts:14–22 defines callback-based startSpan, events/attributes/status; PT/src/noop.ts:3–8 executes business callback once and forwards success/failure. Rust `from_span/noop/in_scope/instrument`, captured Dispatch, poll/drop wrappers and no enter guard across await (S:238–248/R:165–172) are Rust-native mechanism choices, not extra input rejections. Retain explicit propagation, host subscriber/export ownership and business result preservation. Do not claim source utility equality or silently delete functional callback failures. No numeric telemetry queue/error/storage ceiling is specified here; do not invent one.
- Process-local telemetry excluded from serialized model/auth/history is an ownership/privacy contract, supported by distinct Pi types and formal CTRL-003. Fixed domain schemas/automatic spans remain out of candidate scope. The 'trusted nonblocking/nonpanicking subscriber' conformance boundary is an honest limit, not a promise that the SDK contains arbitrary host failure.
- Owned captured input/revision, lazy first poll and Send+'static (S:206/R:59) are language lifecycle differences. Pi lazyStream immediately calls setup() (PI src/api/lazy.ts:46–59); Pi Models normalizes synchronously then invokes it (models.ts:883–895). Do not describe Pi lazy as waiting for first consumption. Rust first-poll capture must not delay owned-input freezing.
- Immutable event-time snapshots, exactly one terminal/failure and active result drain are explicit Rust contract changes. Pi events can alias live accumulators; its unbounded EventStream resolves final Promise at push without drain (PI event-stream.ts:43–57,86–88). Current Opi64 queue is a real retained choice. New state/frame/byte budgets and terminal-envelope256KiB are not implied by immutable ownership or64 events; parent transport audit owns those numeric withdrawals.
- Awaited instance cleanup and generation cutoff (S:230–234/R:159–163) intentionally strengthen Pi's module-global synchronous callback loop (PI session-resources.ts:3–23). Ownership isolation, waiting for local tasks, no remote refund/revoke promise, no parent-token cancellation and keeping started credential rotation distinct are coherent resource contracts. They do not require a new2s deadline, hard byte ceilings or generic shutdown framework.
- `Some("")` rejection is a visible Rust API change (S:230/R:159). Pi passes the string through and Codex cleanup uses truthiness, so empty selects all (PI Codex:950–963). Do not call empty-ID rejection Pi parity. A typed explicit All/Session selector can prevent accidental broad cleanup without treating arbitrary byte-exact IDs as malformed; preserve the chosen safety change only as an acknowledged ownership decision.
- Independent Models namespace, final-public-owner cleanup, owned calls surviving Models replacement and no background self-ownership are Rust lifetime guarantees, not Pi rules. They are not grounds for arbitrary reject-on-freshness of otherwise well-typed custom options.
- Duration from response-stream first activation rather than construction is explicit language timing difference: Pi starts at EventStream construction (PI event-stream.ts:91–99,126–128). Do not invent exact latency/clock guarantees; own backpressure can extend local duration.
- Credential barrier/cutoff/receipt ledger is a new SDK lifecycle seam, not an upstream public action; need a real consumer/ownership case under PRIN-002. Pi independently times refresh at15s and persists under store lock (auth/resolve.ts:105–146). This does not prove total transaction15s, wait30s, keys256, or cross-Models aggregate reporting requirements. Parent auth audit owns full scope decision.

### Handoff

Primary correction: withdrawing hard-coded schema numbers must be accompanied by withdrawal of blanket dialect/keyword/format/cycle rejection, all-patterns guard and unproved selected-backend semantic equality. Exact sources establish substantial differences. Retain zero implicit schema I/O, raw preservation/normalization behavior, honest typed errors and Rust ownership mechanisms without turning them into arbitrary acceptance restrictions. Backend error loss and format/dialect/regex gaps remain open implementation evidence, not silently deleted constraints.

Checks performed: source reads and targeted ripgrep searches only. No repo changes, no test/build result claims, no dependencies installed, no live credentials/network calls. Test impact: none.


## 取证附录D：模型参数、估算、成本及公共helper

## Read-only model numeric parameter audit

2026-10-16. Scope: current candidate AIR-053/AIR-062, spec model-budget paragraphs and SDK annex §§3/7 versus pinned Pi1.2.0. No repository edits; no tests/build/install/live. Test impact **none**. Paths below relative to `.repo/pi-1.2.0/packages/ai/` unless `crates/` specified.

### Result

The major budget/estimate/UUID/cache/cost constants in this group are source-derived and should remain, with their adapter-specific scopes. Two remaining preflight rules are **proposed Rust differences**, not Pi behavior: blanket positive-integer maxTokens validation and numeric-Claude effective-zero `BudgetInsufficient`. Do not portray either as protocol-hard or Rust-language-required. Input-limit metadata validation is source-grounded, but universal SDK dispatch rejection from those fields is not.

### 1. Simple and native output/thinking budgets

| Parameter/formula | Classification | Exact evidence/scope |
|---|---|---|
| Context safety **4096**, minimum available/max **1** | Pi-explicit | `src/api/simple-options.ts:15–21`: W<=0 -> max(1,M); W>0 -> min(M,max(1,W−estimate−4096)). `:46` M=request max ?? model max. These are **simple builder** defaults, not native or all-operation global clamps. |
| Explicit max is not additionally capped to model.max | Pi-explicit | `simple-options.ts:46` uses explicit option directly before context clamp. Numeric Claude separately applies model.max in thinking adjustment. |
| Default thinking minimal**1024**/low**2048**/medium**8192**/high**16384**; xhigh/max ->high | Pi-explicit | `simple-options.ts:68–89`; custom budgets spread over defaults `:82`; clampReasoning `:77–78`. Retain exact table only for consumers of this helper/token-budget family. |
| **1024** reserved answer room, B'=min(B,max(0,ceiling−1024)) | Pi-explicit arithmetic | `simple-options.ts:87–89`; does not guarantee actual1024 answer tokens when response ceiling<1024. Do not add an unconditional minimum-response rejection. |
| Numeric Claude simple C=base clamp; adjusted=min(C+B,model cap), final F=context clamp; effective=min(adjusted B,max(0,F−1024)) | Pi-explicit | `simple-options.ts:92–107`; Anthropic `src/api/anthropic-messages.ts:970–1007`; Bedrock `src/api/bedrock-converse-stream.ts:535–570`. Helper's intermediate B clamp is conditional when adjusted max<=B, but final formula in current spec is equivalent for valid nonnegative B. Adaptive model branches `anthropic:983–989`, `bedrock:545–550` bypass numeric addition. |
| Common Chat numeric budget uses actual serialized max_tokens/max_completion_tokens ?? model cap and omits when result<=0 | Pi-explicit | `src/api/openai-completions.ts:1020–1031`; only when reasoningEffort present and model.reasoning. No Claude budget addition. Template values can have separate omission rules; do not invent universal0 serialization. |
| M is always validated positive finite integer before auth | **Proposed Rust input restriction**, not Pi explicit | `simple-options.ts:18–21` has no such validator; M=0 yields0 with W>0 and1 with W<=0. Native Chat/Responses use truthy check, Google uses !==undefined, Anthropic uses ??, so zero behavior differs. Current Rust `crates/opi-ai/src/provider.rs:112,248` uses Option<u64>, which excludes negative/nonintegral/nonfinite representation but still represents0. No global0 rejection found in current source. Scope validation to actual typed representation/selected contract, not guessed protocol positivity; explicitly record any intended stricter behavior. |
| Numeric-Claude B'=0 -> typed `BudgetInsufficient` before auth | **Intentional proposed Rust safety difference**, not source parity | Anthropic simple `:1007` computes0; native `:1293` uses `options.thinkingBudgetTokens \|\| 1024` (backfills1024). Bedrock simple `:570` computes0; native `:1301–1306` uses ?? and sends0. Therefore current candidate changes both failure timing/dispatch and wire behavior, not just a Rust type. Distinguish from arbitrary constants; if current user wants no extra local rejection, withdraw as baseline or mark explicit unresolved safety candidate. Do not silently copy this difference while claiming full Pi normalization equivalence. |
| Native Anthropic max default=model cap; thinking enabled omitted numeric default**1024** | Pi-explicit | `anthropic-messages.ts:1213,1293`; explicit0 thinking also backfills1024 because logical-or. Native does not use simple context clamp. |
| Native Bedrock Claude max default=model cap, other models absent unless caller explicit | Pi-explicit | `bedrock-converse-stream.ts:263–271`; numeric Claude native table1024/2048/8192/16384 (xhigh/max16384) `:1290–1306`; custom budget0 preserved by ??. |
| Native Chat absent max omitted; explicit truthy max to chosen field | Pi-explicit | `openai-completions.ts:842–848`. Native0 is omitted, not validation failure. |
| Native Responses floor**16**, supportsMaxOutputTokens + non-ChatGPT-signin gates | Pi-explicit adapter behavior; comment cites service limitation | `src/api/openai-responses.ts:32–33,328–341`; only truthy explicit max. Keep actual gates and omission. Source comment alone is not a reason to impose minimum16 across all wires. |
| Native Azure Responses floor**16** without ordinary Responses gates | Pi-explicit | `src/api/azure-openai-responses.ts:18–19,206–207`. |
| Native Codex no max_output_tokens | Pi-explicit omission | `src/api/openai-codex-responses.ts:525–600` request body builder has no max field despite simple builder usage `:512`. Don't turn a computed option into a new serialized field. |
| Native Google/Vertex max only when explicit, including0 | Pi-explicit | `src/api/google-generative-ai.ts:382–383`; `src/api/google-vertex.ts:475–476`. Simple paths **do still use common base4096 clamp**, GenAI `:315–318`, Vertex `:319`; exemption is only common thinking table/answer-room clamp. |
| Pi Messages native/simple passthrough | Pi-explicit | `src/api/pi-messages.ts:378–384,432–443`: explicit max unchanged, absent omitted, only temperature/maxTokens/reasoning/cacheRetention/sessionId/toolChoice serialized. No builder/default max/context clamp. |
| Native Mistral explicit max only; simple common builder | Pi-explicit | `src/api/mistral-conversations.ts:198,390,542`; no extra global max or thinking token table invented. |

#### Google budgets, order minimal/low/medium/high

| Adapter/model selector | Source values | Evidence |
|---|---|---|
| GenAI + Vertex id includes2.5-pro | **128/2048/8192/32768** | GenAI `:440–447`; Vertex `:533–540`. |
| GenAI id includes2.5-flash-lite | **512/2048/8192/24576** | GenAI `:450–457`; checked before generic flash. |
| GenAI id includes2.5-flash | **128/2048/8192/24576** | GenAI `:460–467`. |
| Vertex id includes2.5-flash (includes flash-lite) | **128/2048/8192/24576** | Vertex `:543–550`; no separate flash-lite selector. Preserve this actual difference. |
| Other budget models | **−1** dynamic | GenAI `:470`; Vertex `:553`; protocol option, not unlimited-memory sentinel. |
| Custom budget supplied | Exact supplied value precedes tables | GenAI `:436–437`; Vertex `:529–530`, using !==undefined, including0/−1. No1024 answer-room clamp. |
| Google level-mode models | No numeric budget | GenAI `:329–336`; Vertex analogous `:338–345`; native level takes precedence over numeric budget GenAI `:405–408`/Vertex `:498–501`. |

### 2. Estimator/usage anchors

All numbers here are **Pi-explicit** heuristics, not billing/tokenizer facts. Source `src/utils/estimate.ts`:

- **3.5 chars/token**, **4800 chars/image**, `:15–16`; `text.length` means UTF-16 code units (`:31,34,39,61–65`), so a valid astral character contributes2. Rust must not substitute UTF-8 length or scalar count.
- User/toolResult aggregate all blocks before one ceil (`:30–43,56–57`); assistant aggregates text/thinking/tool-name+JSON args before one ceil (`:59–68`). No per-short-block rounding. Safe stringify fallback is literal `undefined` or `[unserializable]` (`:22–27`) if those unsupported JS cases are applicable to Rust's typed inputs.
- System body+sections goes through getSystemMessageText once, then separate JSON-tool-addition/removal rounded estimates (`:49–54,114–116`). `src/utils/text.ts` owns exact section rendering.
- Context anchor uses totalTokens truthy else input+output+cacheRead+cacheWrite (`:18–19`); latest applicable assistant must have timestamp>=max timestamp of its **preceding** messages, not Error/Aborted, and resulting usage>0 (`:71–94`). After anchor add trailing per-message estimates (`:97–111`). No same-model gate/full-transcript lower bound.
- Current Rust known-positive-total or fully-known parent-sum rule is an **explicit information-model difference** because Pi numeric buckets are not independently optional. Keep unknown,0 and positive distinct; do not misattribute this to Pi.

### 3. Cost/tier and sampling groups

| Rule | Classification/evidence |
|---|---|
| USD/**1,000,000** tokens; tier selected by highest strictly exceeded threshold on input+cacheRead+cacheWrite; unsorted list allowed | Pi-explicit `src/models.ts:1206–1224`. Exact threshold equality stays lower tier; matchedThreshold starts−1. No requirement for integer/nonnegative threshold in `src/providers/model-schema.ts:25–37` (Type.Number without min); adding one is a new metadata restriction. |
| Long1h cache write charged **2×input**; short write=total write−long write; avoid adding children twice | Pi-explicit `models.ts:1217–1224`. Pi helper doesn't clamp malformed negative shortWrite. Rust subset-domain validation/unknown handling is a declared difference; not evidence for rejecting ordinary generation solely because usage metadata is invalid. |
| OpenRouter image uses base rates, no tier helper | Pi-explicit `src/api/openrouter-images.ts:167–196`. Keep even when same model has tiers. |
| Responses flex**0.5**, priority/fast**2**, exact idgpt-5.5**2.5**, other**1** | Pi-explicit `openai-responses.ts:391–418`; response.service_tier ?? request in shared `openai-responses-shared.ts:578–582`. Azure doesn't supply callback; no multiplier inferred. |
| Codex flex**0.5**, priority**2**/exactgpt-5.5**2.5**, fast/other**1** | Pi-explicit `openai-codex-responses.ts:603–629`; response default preserves requested flex/priority `:632–639`; used HTTP`:675–678` and WS`:1558–1561`. |
| OpenRouter per-token ->million rates with **6 decimal** rounding; missing/invalid/negative base->**0**; override fallback to base; time-of-day/day overrides skipped | Pi-explicit `src/providers/openrouter-api.ts:40–76`. Proposed Rust unknown instead of0 is intentional information difference. Do not invent fixed tier-count/rate bounds. Pi perMillion's >=0 test itself doesn't explicitly rejectInfinity; Rust finite representation/validation should be described honestly. |
| Sampling model base < effective-thinking-level < caller keys | Pi-explicit `simple-options.ts:24–33`; supported thinking is clamped via `models.ts:1241–1259`; native merges Chat`:1004–1006`, Responses`:383–385`, Azure`:243–245`. No new arbitrary numeric sampling ranges are derived by this merge. |

### 4. Model limits/cache/UUID/hash/overflow

| Numeric group | Classification and exact scope |
|---|---|
| maxWidth/maxHeight/maxBytes/maxPerMessage/maxPerRequest/maxRequestBytes optional integer >=**1**; JPEG quality integer**1–100** | Pi schema-explicit `src/providers/model-schema.ts:48–82`; maxBytes means **base64 encoded payload bytes**. No standard default numeric maxima. AI SDK contains schema/metadata, no dispatch reader of these fields; host image preprocessing is a separate consumer. A blanket SDK preflight rejection is a new policy, not proof from field existence. |
| promptCache short/long lifetime in seconds, positive (>**0**) | Pi schema-explicit `model-schema.ts:17–22`; Type.Number permits fractional seconds, no arbitrary max or implicit default; absent lifetime unknown/no warm. Don't require integers. |
| OpenAI prompt cache key <=**64** Unicode code points, preserve prefix; absent staysundefined | Pi-explicit normalization `src/api/openai-prompt-cache.ts:1–7`. Uses Array.from, so valid Unicode code points correspond to Rust scalar chars; **truncate**, don't reject overlength. Not UTF16 code units/bytes/graphemes. Lone-surrogate JS strings are a separate representation difference. |
| UUIDv7 supplied timestamp integer **0…2^48−1**; ordinary timestamps nondecreasing; **16 bytes**, version7/variant bits; sequence max **2^41−1** | Pi-explicit `src/utils/uuid.ts:1–44`; validation`:9–11`; ordinary rollback max`:14–16`; random initialized sequence increments across calls, not reset per timestamp`:20–29`. Supplied timestamp remains exact. These are algorithm/wire constraints; retain. |
| ShortHash32bit wrapping/UTF16, output base**36**; seed/multiply constants | Pi-explicit `src/utils/hash.ts:2–12`: seeds0xdeadbeef/0x41c6ce57, multiplies2654435761/1597334677/2246822507/3266489909 and unsigned shifts16/13; output h2-base36+h1-base36. Keep exact only when reproducing compatible IDs; not security hash. |
| Overflow successful Stop input+cacheRead>window; Length output=**0** and input+cacheRead>=**0.99×window** | Pi-explicit detector `src/utils/overflow.ts:137–170`, after error-pattern/nonoverflow checks`:139–149`. No cacheWrite in these detector sums; no automatic rejection/retry/compaction.0.99 is heuristic, not safety limit. |
| RecoverableLength output<original desired max and desired max>**0** | Pi-explicit helper `overflow.ts:173–180`; detector only. Comment mentions caller bounded retry but helper doesn't execute one. |
| Allowed Anthropic fallback models max**3** | Pi schema-explicit, not generic retry limit；`src/providers/compat-schema.ts:485–487` allowedFallbackModels maxItems3; preserve adapter-specific metadata, don't conflate fallback candidates with attempt count. |

### Recommended corrections

Retain source-derived formulas and number groups above; label heuristics, normalizations and algorithm requirements accurately. Rewrite blanket maxTokens positivity as actual typed/adapter rules or a separately acknowledged Rust difference. Keep numeric-Claude zero-budget behavior explicitly separate; it is currently a deliberately chosen local rejection with no matching Pi preflight. Do not add minimum budget checks to Google−1/0, cache-key truncation, price tiers or metadata solely for uniformity. No evidence in this audit requires changing formal `opi-spec.md`; these candidate research corrections do not authorize runtime work.

## 取证附录E：retry与九OAuth参数

## Retry and OAuth parameter audit

2026-10-16. Read-only audit. No repository edits, installs, builds, runtime tests, live calls, credentials, or metadata probes. Test impact: none. Candidate docs are under parent revision; section names identify affected text more reliably than current line numbers.

Exact local source aliases:
- P = D:/Luiz/Odradek/opi/.repo/pi-1.2.0/packages/ai/src
- C = D:/Luiz/Odradek/opi/.repo/pi-1.2.0/packages/coding-agent/src
- S = C:/Users/Luiz/AppData/Local/Temp/opi-parameter-npm-source/@smithy__core-3.33.3/package/dist-cjs/submodules/retry/index.js
- N = C:/Users/Luiz/AppData/Local/Temp/opi-parameter-npm-source

### Findings that affect the rewrite

Most retry numbers are source-backed and should remain: helper 500ms/8s/25% jitter, Codex 1s base, Smithy 100/500ms bases, 20s jitter cap, 5s server-hint adjustment, 500 quota, costs 5/10, default 0 extra helper/Codex and 2 extra Bedrock, at most 2 strict variants, and two independent once-only Codex recovery branches.

Four qualifications matter:
1. Applying max_retry_delay_ms=60000 admission to Bedrock introduces rejection absent from Pi. Pi ignores this generic option there; Smithy already bounds hint influence to jitter+5s. Preserve helper/Codex 60s and explicit 0; make additional Bedrock admission explicit caller policy if retained.
2. Standard-only operation and ignoring AWS_MAX_ATTEMPTS/AWS_RETRY_MODE/SMITHY_NEW_RETRIES_2026 are configuration coverage differences. The 2026 switch changes actual constants, not merely naming.
3. Giving Retry-After fixed priority over x-amz-retry-after changes Smithy's header enumeration behavior. It is a deliberate normalization, not source parity.
4. Per-logical-call quota matches effective Pi Bedrock behavior: Pi constructs a fresh AWS client per stream call. Do not claim that preserving this changes Pi from provider-global to per-call quota.

### Low-level retry inventory

| Group | Classification and exact source | Required distinction |
|---|---|---|
| Extra-send count and helper default 0 | Pi-explicit: P/utils/provider-retry.ts:107-125 uses maxRetries ?? 0 and at most N eligible caught retries after the initial request. Its comment requires vendor SDK maxRetries=0 (:100-105); Anthropic does so at P/api/anthropic-messages.ts:639-642. | Preserve 1+N per acquisition episode, explicit 0, and one owner. Do not stack a vendor or Agent acquisition loop. |
| Helper error classification | Pi-explicit: P/utils/provider-retry.ts:16-35,120-121 requires the expected Error/status/headers shape, honors x-should-retry true/false, then no-status or 408/409/429/>=500; explicit excluded statuses still block. Google adds missing headers before delegating: P/api/google-shared.ts:488-514. | Preserve typed equivalents and exclusions. Callback exceptions are not transport errors merely because they are Error objects. |
| 500ms, 2^i, 8s, 25% jitter | Pi-explicit: P/utils/provider-retry.ts:67-68 computes min(500*2^i,8000)*(1-U*.25), with i=0 at the first retry (:123). | Retain. This is not old opi's 1s/no-jitter behavior or an unsupported Rust resource estimate. |
| Helper server-hint syntax | Pi-explicit: P/utils/provider-retry.ts:53-64 uses nonempty retry-after-ms first, parseFloat in milliseconds; otherwise Retry-After parseFloat seconds or Date.parse. | Preserve protocol-specific syntax fixtures; Codex and AWS use different parsers. |
| Helper/Codex 60000ms and sentinel 0 | Pi-explicit helper P/utils/provider-retry.ts:1,39-50,100-105; Codex P/api/openai-codex-responses.ts:56,168-175. A positive cap rejects an original valid server hint above it. Zero disables admission checking. Helper sleep clamps negative delay to 0 (:93); Codex clamps during parsing (:144,155,160). | This is server-delay admission, not a computed-backoff cap, a total deadline, or sleep=0. Never reject valid sentinel 0 through a generic positive-only validator or shorten a long instruction into permission to retry early. |
| Codex default 0 and 1000ms base | Pi-explicit: P/api/openai-codex-responses.ts:54-56,389-391,425-432,456-457. Delay is 1000*2^attempt without jitter absent a valid hint. | Retain with its own counter and optional header timeout; no invented total deadline. |
| Codex statuses/body predicates | Pi-explicit predicate: P/api/openai-codex-responses.ts:129-136 retries 429/500/502/503/504 or named transient body predicates, excluding terminal quota-related 429. Parser uses Number for retry-after-ms/Retry-After then Date.parse (:139-163). | Candidate's terminal 400/401/callback classification is a deliberate safety correction: Pi's outer catch (:443-460) can retry generic thrown parsed HTTP errors beyond the predicate. Do not call this exact failure parity or generalize to every 5xx. |
| Bedrock default 3 total attempts = 2 extras | Dependency default: S:432-433,458,513-518. Pi leaves SDK maxAttempts/retryMode selection intact: P/api/bedrock-converse-stream.ts:163 onward,251,288. | Retain the standard default. Pi's common maxRetries is not wired to Bedrock, so applying it is an explicit Rust API change. |
| Smithy retry classes | S:19-38,57-81,231-238,523-524: throttling is status 429, named errors or throttling trait; transient includes retryable trait, corrected clock skew, expired signature, named/network errors, 500/502/503/504, H2 refusal and bounded nested causes. Standard retries only THROTTLING/TRANSIENT. | Bare 408/409/501/505 are insufficient, although a real retryable trait can still admit them. The source cause-depth check <=10 is not a new global JSON-depth budget. Owned signing must actually provide any claimed clock-skew correction after removing the vendor runtime. |
| Smithy 100/500ms, 20s, full jitter | Dependency default: S:84-86,367-395,471-476. Legacy standard computes floor(U*min(base*2^i,20000)), with base 100 transient / 500 throttling. | Retain this mode's constants, and qualify the separate 2026 mode. |
| Smithy +5000ms hint allowance | S:475-476 computes max(b,min(hintRemaining,b+5000)); the strategy waits directly at :488. | Actual wait can approach 25s when b approaches 20s. Do not incorrectly infer every wait <=20s from DefaultRetryToken.getRetryDelay (:417); its returned token already follows the wait in this implementation. |
| AWS hint units/priority | S:94-139 iterates response headers. Retry-After supports seconds/date; x-amz-retry-after is Number milliseconds. First matching recognized header wins; a malformed first match may return undefined. | Fixed Retry-After priority is a deliberate semantic normalization. Preserve source-qualified expectations or explicitly record the accepted difference. |
| Quota 500; costs 5/10; return cost or 1 | S:87-90,443,485-496,513-524. In the legacy mode, TRANSIENT costs 10 and other retryable classes cost 5 (:375-382). Initial attempt does not spend retry cost; success returns the acquired cost or 1. | Admission requires capacity >= cost, not merely capacity > 0. Values 1-4 also block a cost-5 retry. |
| Quota lifetime | Strategy owns capacity: S:440-460; resolved strategy is memoized per client: S:747-770. Pi constructs a new BedrockRuntimeClient each stream at P/api/bedrock-converse-stream.ts:249-251 and sends one command at :288. | Resetting quota per logical Pi stream is effective source parity. HTTP pooling may be independent. A reused standalone AWS client would have a longer quota lifetime. |
| Bedrock 60s admission extension | No corresponding Pi behavior: its config/send path does not consume maxRetryDelayMs; Smithy applies bounded hint influence above without a 60s admission error. | Withdraw this additional default rejection, or label it explicit caller/host policy. Fixing an ignored option is not proof the new default restriction aligns with Pi. |
| AWS_MAX_ATTEMPTS / max_attempts | S:722-745 reads env/config using parseInt, with default 3. | Ignoring these is a configuration coverage difference. Preserve frozen trusted resolution; do not silently read ambient settings after capture. |
| AWS_RETRY_MODE / retry_mode | S:427-433,747-778 contains standard/adaptive modes and actually constructs AdaptiveRetryStrategy. | Standard-only support is a concrete coverage limitation. This audit does not require speculative frameworks, but an opt-in standard override does not make ignored configured adaptive behavior equivalent. |
| SMITHY_NEW_RETRIES_2026 | S:368 tests ambient value exactly true. It changes base to 50ms, throttling base to 1000ms, costs to 14/5, and modified cost class to THROTTLING (:369-382); long-poll behavior also differs (:463,478-481). | Fixed 100/500/5/10 describes the ordinary legacy standard mode, not every behavior of the locked SDK. Selecting or ignoring the switch must remain explicit and documented. |
| Anthropic at most 2 variants | Pi-explicit P/api/anthropic-messages.ts:644-681 uses one downgrade flag. Only rejected 400 invalid_request_error plus three exact message predicates admits retry without preferred strict tools. onPayload runs per variant (:648-652), onResponse after acceptance (:690); require stays strict (:1624-1628). | Preserve at most 2*(1+N) sends, including 2 at N=0. No third variant. There is no invented numeric local grammar/tool threshold: a specific server rejection triggers the transition. |
| Codex two separate once-only branches | P/api/openai-codex-responses.ts:304-305 tracks connection limit and missing previous response separately; :344-352 permits each once. | Can reach 3 WS attempts (initial + one each), then permitted pre-output SSE has its separate 1+N acquisition budget. A shared single-retry cap would narrow Pi. Candidate's missing-previous-before-output guard is deliberate strengthening: source :346 lacks !websocketStarted; the connection-limit branch does check before start (:344). |
| WS connect 15s / idle 5min / age 55min | Pi explicit connect at P/api/openai-codex-responses.ts:57,1080; same file's session constants and expiry logic :1053-1073 own idle/age. | Retain source scope; age is checked on acquire, while idle5min schedules a timer on release that skips busy entries; neither is an active-generation deadline. These facts do not justify a new close=2s or frame cap. |
| Classifier 2 extras; Decisions no 504 retry | P/api/classifier-shared.ts:76-100 and llama-cpp-classify.ts:265; P/api/openai-decisions.ts:146-151 sets NO_RETRY_STATUSES=[504]. | Retain per underlying HTTP attempt and optional fresh attempt timeout. Comments about approximately 5s gateway/~600K tokens are observations, not new SDK admission limits. |
| Image 0 extras | P/api/openrouter-images.ts:69-79 disables vendor retry and delegates maxRetries to helper, whose omitted default is 0. | Do not restore OpenAI standalone SDK's default 2 retries; Pi explicitly disabled them. |
| Whole-call helper omitted/disabled 0; 60s cap | P/utils/retry.ts:127-132,192-225. Enabled policy supplies maxRetries/baseDelay; omitted/disabled performs no extra call. Computed delay is baseDelay*2^(attempt-1), with integer guard then maxAgentDelay default 60s. Executable code has no jitter despite an interface comment mentioning it. | Keep explicit helper, fresh logical calls and separate counters. This 60s caps computed delays, unlike the provider hint-admission meaning. No SDK default 3 or 2s exists here. |
| Product enabled / 3 / 2000ms / 60000ms | C/core/settings-defaults.ts:17-24 sets enabled, maxRetries=3, baseDelayMs=2000, SDK default maxAgentDelay=60000 and provider.maxRetryDelayMs=60000. | Keep host policy; do not automatically enable these defaults in Models or the SDK whole-call helper. |

### OAuth numeric and callback inventory

Every value below is Pi-explicit unless otherwise noted. Stored OAuth's 15s refresh signal is distinct from login waiting and persistence ACK (P/auth/resolve.ts:103,132-146). There is no universal nine-flow 30s login deadline or body-size quota.

| Flow/group | Exact values/source | Scope and discrepancy |
|---|---|---|
| Shared device poller | P/auth/oauth/device-code.ts:5-9: minimum 1000ms, omitted interval 5s, slow_down +5000ms. Deadline is server expiry or Infinity if absent (:47-54). Optional wait before first poll :57-60; valid new server interval wins, otherwise +5s (:81-86); sleep bounded by remaining time (:94). | Retain source/protocol values. Comments attribute 5s/+5s to RFC8628; 1s minimum is Pi behavior. No fixed poll-count limit or universal 15min expiry. Do not reject an interval 0 accepted by the selected flow merely through positive-only budget validation. |
| Anthropic | anthropic.ts:17-23 prefers 53692 /callback. It next tries port 0, then manual fallback if both binds fail (:142-157). HTTP JSON requests use 30000ms (:76-87). Token expiry subtracts 5min (:136,274). | Do not freeze the only allowed port or add a 30s total browser-login deadline. Existing Rust real-expiry/single-window behavior differs intentionally from Pi's flow margin plus resolver margin. |
| OpenAI Codex legacy | openai-codex.ts:26 uses localhost:1455/auth/callback; bind failure allows manual input (:364-373). Device timeout is 15*60s (:31,232-235). Device interval accepts numeric/string finite >=0 (:213-227), then shared poller enforces 1s minimum. Token expiry has no flow subtraction (:141). | Retain legacy bind fallback and the 15min device scope. There is no module-wide direct HTTP 30s constant. Stored refresh gets the resolver's 15s signal separately. |
| OpenAI ChatGPT | openai-chatgpt.ts:22-25 uses fixed 127.0.0.1:1455/auth/callback; bind failure is terminal (:125-129,239-247). Expiry subtracts 3min (:29,177). Random state/nonce use 32 bytes (:50,239), with required UUID host-id pattern (:18). | Do not copy legacy free-port/manual-on-bind-failure behavior. Preserve stable host identity and agentName. No explicit total-login/per-request timeout constant is present here; stored 15s refresh composes separately. |
| GitHub Copilot | github-copilot.ts:229-259,270-271 uses server device interval/expiry. Access expiry is expires_at*1000 minus 5min (:334-345). Specific catalog/policy helper has 5000ms per request (:148-150), retries only 429 (:152), starts at 500*2^retry, uses Retry-After, clamps negative to 0 and stops if delay >= remaining budget (:154-164). Login catalog/policy uses maxRetries=2, maxElapsedMs=5000 (:398,462-469); refresh catalog uses 0/0 (:359-362). | Do not apply 5s/2 retries to all Copilot requests. Device authorization and raw token fetch pass the caller signal only (:208-220,321-328). Stored refresh's outer 15s includes its sequence. |
| OpenRouter | openrouter.ts:21-22 has login 5min and exchange 30000ms. Callback uses port 0 and random path (:115-123). Exchange timer covers fetch and response JSON, cleared after parsing (:67-93). Credentials use expiry Number.MAX_SAFE_INTEGER and empty refresh (:105-109). | Preserve free port and actual timer scope. MAX_SAFE_INTEGER represents a non-expiring key, not a TTL resource limit; a typed NonExpiring equivalent need not trigger refresh. |
| Kimi Coding | kimi-coding.ts:16-19 defines fallback device 15min, poll 5s, request 30s, refresh 3 extras. Valid server interval/expiry overrides fallback (:102-116). Refresh is at most 4 attempts with 1s/2s/4s waits (:214-218); network errors retry (:239-241); 429/>=500 retry (:210-211); 401/403/invalid_grant terminates (:249-252). Service expires_in has no subtraction (:138). | Retain this flow-specific refresh retry even though chat helper defaults 0. Stored resolver 15s may truncate the composed refresh sequence; this is actual Pi behavior, not grounds to add an ADC/ACK total timer. |
| Meta | meta.ts:25,173 assigns minted API key 24h. Per-HTTP timeout is 30s (:26,36-37). Device interval/expiry are optional positive server fields (:97-105), with no fixed 15min fallback. | Missing expiry can leave the shared poller pending until caller cancellation. Preserve cancellation without inventing a finite admission default. 24h is Pi's key-lifetime assumption, not a remote expires field. |
| xAI | xai.ts:13-14,135-141 subtracts 5min and uses fallback token lifetime 3600s only when expires_in is omitted. Device interval 0/malformed falls back to shared 5s (:109-124); expiry is positive server data. postForm passes caller signal (:64-77). | Invalid present expiry is not the same as omitted. No direct HTTP 30s constant; stored refresh's 15s is separate. |
| Radius | radius.ts:18-22 uses 127.0.0.1:1456/oauth/callback and 60000ms token expiry skew. Browser bind has no free-port fallback (:153-172). Device interval/expiry come from server (:214-235); shared slow_down applies (:253-257). Token request uses caller signal (:101-106). | Radius's 60s is expiry skew, not an HTTP deadline. Do not invent 30s requests or 15min device expiry. |
| Generic callback server | callback-server.ts:17,30,120,133-134: port 0 selects a free port, timeout is optional and installed only when supplied. | No hidden generic callback timeout. Positive-only resource validation must not reject valid port 0. Flow-specific fixed-port failures and manual fallback are distinct. |
| PKCE | pkce.ts:23-31 generates 32 random verifier bytes, base64url without padding and SHA256 challenge. | Source-backed security mechanism, not an arbitrary input cap. Keep secure RNG and state binding; withdrawing resource tuning numbers does not justify weakening them. |

Formal identity/trust/cancellation boundaries remain unchanged: retries do not authorize tool effects or account substitution; callbacks cannot masquerade as transport loss; cancellation of a waiter cannot discard an already rotated credential. This audit makes no runtime parity claim. Global fetch/WS defaults remain runtime-qualified wherever Pi delegates Node/Bun rather than the restored dependency code.

Additional timing evidence: P/auth/oauth/github-copilot.ts:272, kimi-coding.ts:150, meta.ts:106 and xai.ts:165 explicitly wait before the first device poll. Codex and Radius do not set that option and therefore poll immediately. Preserve this observable timing difference. Exact WS constants are P/api/openai-codex-responses.ts:867-868 (5min/55min).

Another valid zero sentinel: Codex normalizeTimeoutMs accepts finite value >=0 and floors it (P/api/openai-codex-responses.ts:192-197). HTTP headers install a timer only when >0 (:397-398), WS connect only when >0 (:1143-1146), and WS idle wait only when >0 (:1395-1403). Thus explicit timeoutMs=0/websocketConnectTimeoutMs=0 disables these source timers; a positive-only validator would narrow the existing contract. Omitted WS connect retains15s; omitted HTTP/idle has no default timer in those paths.

补充源码边界（独立复审）：calculateCost在models.ts:1209以matchedThreshold=-1起始，仅选threshold>-1且严格超过者，重复同threshold取首；model-schema并未给threshold正数下限。Responses/Azure均只在truthy max发送floor16，显式0省略；SDK contextWindow<=0分支是max(1,M)，不是原值透传。它们是来源语义，不得添加新的正数拒绝来掩盖。

复审补充的zero语义：helper/Codex max_retry_delay_ms非正值关闭的是server-hint上限检查，不是禁用/忽略hint。Image timeout遵OpenAI positive-integer规则；classifier supplied0交AbortSignal.timeout立即到期，不能统一提前InvalidInput；Codex connect/header/idle0则关闭对应timer。不同0的作用不得合并。
