# Pi 1.3.0 后端覆盖缺口专项核查

> 2026-10-16；输入 Opi `a37ed708dfdac3528bc7dfeba18f0df3af68cadc`、本地 `.repo/pi-1.3.0`。只修订研究文档，未实施、未更新 Cargo/测试/CI/正式规范。七组覆盖缺口全部 **OPEN**；源码事实与方案可行性不等于运行验收。唯一要求、验收和差异索引仍为双语 spec、[V 矩阵](acceptance-matrix.md)及 [RDF](pi-1.3.0-rust-differences.md)，本页不新增 AIR/V 或实施账本。
>
> [冻结证据](pi-1.3.0-backend-gap-evidence.json)保存输入、决定性源文件/归档摘要、实际检查和独立报告摘要；[专项复审](pi-1.3.0-backend-gap-review.md)只对本轮输入有效。中文技术附件没有既有英文 counterpart；双语正文同步约束并共同引用本页。正式 `opi-spec` §8.1、PRIN-002/GATE-002 继续决定准入，研究方案不授予实施权限。

## 1. 证据与判定口径

固定 Pi tree `08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2`；沿用[升级来源清单](pi-1.3.0-source-evidence.json)，本轮重新核对决定性源文件。实际外部依赖未变，不从“1.3.0”推导后端问题已解决。本地快照没有 Git 身份及 hydrated provider JSON；没有核验上游 tag/npm 发布身份。缓存归档只作为惰性文本读取，没有重新联网、安装、import 或执行其中代码。

源码简称用于下面的 `路径:行`。归档 URL、完整摘要、实际恢复路径和选中文件字节对照在冻结证据中，不以 TEMP 文件作为唯一长期依据。

| 简称 | 固定来源 |
|---|---|
| PI | `.repo/pi-1.3.0`；路径从快照根开始 |
| TB | `typebox-1.3.27`，路径从 `package/build/` 开始 |
| JS | `jsonschema-0.28.3/src/`；`fancy-regex-0.14.0`另行标明 |
| TW／TTW | `tungstenite-0.28.0`／`tokio-tungstenite-0.28.0`发布源码 |
| RW | `reqwest-0.12.28`发布源码 |
| GA | `google-auth-library-10.6.2/package/build/src/` |
| NF／GX／SM | `node-fetch-3.3.2/package/`、`gaxios-7.1.4/package/`、`@smithy/core-3.33.3/package/` |
| AWS | 表内具名的精确 AWS/Smithy 发布版本，不是当前 Rust 运行依赖 |
| REG | Cargo.lock 对应的 registry 源码：ring0.17.14、rustls0.23.40、tracing0.1.44 等 |

区分四种结论：**已证源码限制**、**参考运行时未知**、**候选补救但未验证**、**明确 Rust 差异**。例如 TW 不支持压缩是已证事实；所选 Pi global WebSocket 是否协商压缩尚未证明，不能把二者写成已执行的双端失败。一个组只在对应公开能力、正反例、失败/取消/副作用及平台证据齐全后才可关闭。

本轮不增设 bytes/count/depth/CPU/总时长默认上限，不恢复已撤回的配额；backend 默认与不可配置限制照实列出。`Unsupported`、可覆盖配置、文档选型、源码下载或一次最终成功都不能关闭接受集缺口。更严格的来源/秘密保护仍保留，并以 RDF 差异证明其影响。

## 2. Schema：公开机制可表达性先于选型

**源码依据。** PI `packages/ai/src/utils/validation.ts:317–350` 先拒绝 truthy argumentsError、clone并处理optional-null/Value.Convert；取得root `Compile(schema)` validator之后才做plain-JSON coercion及Check；root export在`packages/ai/src/index.ts:47`。TB `compile/compile.mjs:6–11`、`compile/validator.mjs:10–14`、`schema/build.mjs:105–117`直接构造/求值，没有draft metaschema预检。JS `compiler.rs:283–299`却在keyword编译之前执行所选draft metaschema；关闭方法在`options.rs:346–353`为 **pub(crate)**。公开`with_keyword`在`:434–446`，到`compiler.rs:366–376`才调用，不能越过前面的schema拒绝。

以下是**源码推导的未执行语料**。每条未来都须记录Pi公开utility、直接后端、真实Rust公开utility三层结果，含compile/Check/validation/error阶段；直接后端失败不是完整适配不可行的证明。

| 细项／决定性源码 | 具体验证语料与预期 → verify | 可行方案／关闭条件 |
|---|---|---|
| keyword混用/metaschema：TB `schema/engine/schema.mjs:134–249`，JS `compiler.rs:334–376` | `items:[{const:1}]`＋`prefixItems:[{const:1},{const:2}]`对`[1,2]`/`[1,3]`：Pi通过/失败；native旧draft忽略prefixItems，新draft拒绝array-items。`{type:"string",minLength:"2"}`对`""`：TB将错误形状keyword视为inert，native各draft meta拒绝。`$ref`旁minimum的实际约束也须验 | 不能以选Draft7、unknown拒绝或调用私有disable方法关闭。公开prefixItems hook可补简单子集；有条件的private lowering须证明原schema、ref bases/evaluated sets不变。不声称所有转换不可能，也不把猜想转换写成已选定机制 |
| 字符长度/数值leaf：TB `schema/engine/minLength.mjs:6–13`、`guard/string.mjs:34–39,64–81,139–167`、`guard/guard.mjs:86–96`；JS `keywords/min_length.rs`/`max_length.rs:42–58`、`multiple_of.rs` | `{type:"string",maxLength:1}`对`"e\u0301"`：Pi的近似grapheme helper通过、native scalar-count失败；minLength2反向。再测ZWJ/flag/combining、multipleOf边界 | 用公开leaf hook有机制可能，但须匹配该版本helper，不泛称JS.length或标准Unicode grapheme；数值tolerance是源码算法，不是工作预算。真实嵌套/union/normalization前后都需验证 |
| formats/content：TB `format/_registry.mjs:64–94`、`schema/engine/format.mjs:7–14`；JS `keywords/format.rs:756–807`、`keywords/mod.rs:318–325`、`options.rs:152–155,244–251,360–373` | unknown format对`"bad"`通过；21个已知format各正反例，uuid/url/json-pointer-uri-fragment不能因draft被忽略。`contentEncoding:"base64"`对`"%"`、contentMediaType JSON对`"not-json"`在Pi inert，Draft7 native可额外拒绝 | 公开assert-known/ignore-unknown＋with_format、关闭具体content支持/受控keyword hook可补局部。全部AnnotationsOnly不行；URL.canParse需固定runtime，21个算法而非同名注册必须比较。默认registry Reset与自定义全局registry影响分开记录，不暗中采ambient修改 |
| refs/metadata/$schema：TB `schema/resolve/resolve.mjs:130–154,179–219,265–269`；referencing0.28.3 `specification/subresources.rs`、JS `options.rs:147–149,273–311` | local `$defs` pointer正反例；放在`x-meta`中的`$id`目标与正常target；HTTP/file/urn identity在内存可解析时零I/O。unresolved ref的Pi Check-false与native compile error分阶段记录；`$schema`在/不在、nested/duplicate IDs、encoded pointers | 显式static-only retriever＋已知内存resources防隐式I/O；不能只靠feature关闭或URI scheme拒绝。TypeBox遍历metadata可发现ref target，且`$schema`存在影响legacy frame，不能删所有unknown/meta/$schema做简化。resource索引/优先级及错误映射必须证明 |
| recursion：TB `schema/engine/_functions.mjs:13–21,57–64`；JS `keywords/ref_.rs:32–56,82–98,124–147` | descending/mutual/dynamic/recursive refs、sibling和无关分支；`{"$ref":"#"}`源侧递归无已证有限返回，native可省略self-ref得到true；独立子进程记录实际返回/exception/nontermination | 不默认拒全部cycle，也不假称都安全。harness containment timeout仅控制实验，不是SDK预算。需要保持ref状态机的补救；若适配器变成另一个validator则重开选型，不能以成功/typed拒绝吞掉问题 |
| regex语义/错误通道：TB `schema/engine/_regexp.mjs:1–3`；JS `ecma.rs:40–47,120–126`、`keywords/pattern.rs:117–145`、`not.rs:25–35`、`any_of.rs:57–74`、`pattern_properties.rs:49–95`、`additional_properties.rs:550–565` | `^\s$`对U+1680：源侧通过、native翻译缺该空白。合法ECMA-u昂贵pattern先找到native真实backtrack Err再固定case；比较bare/not/anyOf/if/contains/patternProperties/additionalProperties/propertyNames及未选中的合法branch。编译期invalid语法与运行期match失败分开 | native有的bool路径吞Err成false，not可能变成功，validate/iter_errors也不足。fancy-regex默认1000000是backend机制；不改成Pi fuel。with_keyword仍要求bool is_valid，leaf替换不修复parent通道。需branch-correct机制/上游改动/重选；不复制私有canonicalizer、不cross-product guard或全局拒绝 |
| normalize/raw与调度：PI `validation.ts:58–124,173–237,317–340`，JS `validator.rs`同步入口 | required integer `"0x10"`→16、null→0；optional不收null字段删除；已匹配string union不先转数。raw不变；2^53附近、指数/-0/String语义另验。cache miss/eviction/等待取消和worker持permit直到真实退出 | 小型owned JSON normalizer有必要，但不重建TypeBox私有~kind/codec。root schema integer minimum10、args`"2"`源侧可原样返回invalid；Rust最终完整验证为明确RDF09正确性差异。同步计算不能被async timeout强杀；有界调度/缓存不得添合法schema拒绝或猜CPU/RSS上限 |

**选型建议。** jsonschema0.28.3继续为**尚未证明可完整适配的候选**。先在未来获准的最小机制验证中列出每个public knob、leaf override和必要lowering，证明mixed keywords、refs、meta-admission与fallible regex组合都可维护。不能以“需要adapter”概括性越过这些阻碍。若必须重写applicator/ref evaluator或引入未经证明的error side channel，应比较上游改动/维护中的其他engine；本轮没有选择fork、替代库、JS runtime或自研全量validator。

**验证方法。** 固定JSONL `{schema,arguments,expected,source_refs}`和原文本bytes，在隔离离线Pi包的公共`validateToolArguments`下得到真实state/normalized/raw_unchanged；补直接`typebox/compile`的Compile/Check/Errors仅作定位。native baseline分别记录build、validate、is_valid、iter_errors，逐项记录全部options/静态retriever；未来真实opi公开consumer再跑同case，区分invalid instance、schema resolution、backend error及RDF差异。涉及ref cycle/昂贵regex的case放独立harness进程；先取得源侧真实结果，不能预填未知错误阈值或用后端自证。

本组 **OPEN**；V-11/34/35/39/41/58、RDF-09。argumentsError前置拒绝、root correctness修正、已存在validator类或源文件数均不关闭引擎接受集。schema immutable/raw与strict wire副本分离仍保留。


## 3. WS：运行时、握手与压缩分别证明

**源码依据。** PI `packages/ai/src/api/openai-codex-responses.ts:995–1025` 使用 `globalThis.WebSocket`，仅 Bun 分支添加其代理选项；`:1089–1097` 删除 OpenAI-Beta 后，以第二参数 `{headers}` 构造。`packages/ai/package.json:98–100` 只要求 Node >=22.19.0，没有固定实际 executable/build/global constructor。lock 中 npm `ws8.21.0`、`undici` 不是这条 global 路径的行为证明；类型断言也不能证明某 Node/Bun 构造器支持该 headers overload。

PI 同文件 `:302–375` 对非 SSE 路径的 eligible pre-start 失败可以回落 SSE，没有只限 Auto 的 guard，**显式 websocket 也可能回落**。因此最终有回答不足以证明 WS 成功；须观察真实 Upgrade、帧、header canary 和 fallback 分支。参考运行时本身不能握手或不发送 auth 时，记录参考失败与待核合同，不能要求 Rust 模仿意外失败。

| 细项及源码 | 未来验证方法 → 机械结果 | 可行方案与尚未关闭项 |
|---|---|---|
| TW `src/handshake/machine.rs:44–59,163–185`：成功 read 后、解析前检查累计 bytes>65536、reads>512、reads>64 且 reads×128>bytes | native 私有 supplied-AsyncRead 精确控制 read chunking；双端 loopback 验接受集。含大合法响应、微小读取和 101+首帧同批返回；必须保留 tail 一次 | 这是累计 read 返回值，不是 TCP 包数或纯 header 长度；可能包含首帧 tail。WebSocketConfig 无法解除；buffer coalescing 不构成普遍补救。实际 Pi 限制与差异仍待运行时证明 |
| TW `src/handshake/headers.rs:10`、`client.rs:301–308`：124 header 槽 | 合法 header 数/大小分别跨 backend 阈值，记录双方接受与失败；不能要求参考已拒绝的输入必须通过 | 可选完整自有 Upgrade 解析后接 codec，或 scoped upstream patch/重选；不能把124变成 opi 公共拒绝政策 |
| TW `client.rs:244–268`：Connection 用整值匹配 Upgrade，extension validation 留 TODO | `Connection: keep-alive, Upgrade`、大小写、重复字段、非法/缺失 Accept、未请求 subprotocol/extension；正确 token/协商验证，失败零内容发布 | token-list 接受风险与未协商 extension 校验风险分开。直接 native handshake 不解决；自有握手必须完整验证，不能只检查101 |
| TW `Cargo.toml:54–79`无压缩 feature，`protocol/mod.rs:636–645`拒绝 RSV；同文件`:78–102,128–136`接收大小可配置 | 首先记录实际 Pi offer/协商/帧。若参考确实使用 PMD，再验压缩/未压缩、fragment/control、context takeover 与参数组合；同时测无 extension 正例 | 接收 size 设置为 None 不提供压缩，也不证明固定 RSS。不向无压缩 codec 宣告 PMD；不在 message 外随意套 deflater。参考支持面未固定，codec/后端选择仍 OPEN |
| PI connect/idle/cache/recovery 路径；TW codec、TTW driver | 连接前/握手中/fragment/背压时取消；busy-overflow、sessionless、cleanup、idle busy-skip、age-on-acquire；唯一终态、零晚 continuation、owned socket/driver/timer 退出 | 保留15s连接作用域、5min idle/55min age来源；Rust next-acquire eviction 与 awaitable cleanup 是既有 RDF 差异。不加2s close默认预算，不声称 peer ACK/退款 |

**可行路径，尚未选择实现：**

1. 保留 TTW `src/tls.rs:189–236` provided-stream native handshake，可维持 route/instance TLS 所有权，但继续暴露上述固定检查和验证问题。
2. 完整解析并验证 Upgrade，再调用 TTW `src/lib.rs:204–230` 公共 `WebSocketStream::from_partially_read(stream, tail, Role::Client, config)`。保留 overread 一次；只有确实无 tail 或已有正确前缀 stream 时才用 `from_raw_socket`。owned 验证须覆盖 HTTP/version/status101、Upgrade/Connection token、fresh key/精确 Accept、subprotocol、extensions、重复/无效 header、redirect、取消与 checked 增长。此公共接入点仅证明机制可达，尚未设计/实施解析器；它仍不增加 PMD。
3. scoped upstream patch 或后端重选，以实际参考需要为依据。先审精确源码、feature/native/供应链、协议维护成本和共同语料；本轮未审替代库，不把任何新库写成最终选择。

本组 **OPEN**；V-02/30/49/58，RDF-13/18。测试自身的 loopback `write()` 次数不能控制 TCP `read()` 次数；私有确定性 reader 的机制证明与公开 SDK loopback 的组装证明必须都有。SSE fallback 记录而不掩盖 WS 结果。

## 4. HTTP／proxy／H2／redirect：按实际消费路径

**源码依据。** PI `packages/ai/src/api/bedrock-converse-stream.ts:220–231` 在 proxy/FORCE_HTTP1 选择 NodeHttpHandler，普通默认是 NodeHttp2Handler；`:249–251` 每流新 client。`packages/ai/src/utils/node-http-proxy.ts:13–21,74–157` 给出 scoped/lowercase/nonempty、NO_PROXY/domain/port/IPv6、protocol/ALL_PROXY 及仅 HTTP(S) 代理的行为。GX `build/esm/src/gaxios.js:373–405,522–524` 另有 ambient proxy/agent/fetch 选择，不能拿 Pi helper 代替 Google 内部规则。

NF `src/request.js:126,134` 的 follow20 只属于该 node-fetch 路径；`src/index.js:171–237` 有 subdomain/protocol 敏感 header 规则、POST301/302及303转 GET、body replay 限制。RW `src/redirect.rs:161–163,239–249` 默认10，按 host/port 去敏感 header，不能直接充当等价实现。Smithy raw Node handler 返回 status/body 不能证明整个 AWS middleware/operation 的最终 redirect 行为；Node global fetch/Bun WS 的默认也仍未固定。

| 细项 | 未来验证方法 → 机械结果 | 可行方案／剩余缺口 |
|---|---|---|
| HTTP协议与有效features | 普通 Bedrock 对 loopback H2 ALPN/wire 成功；proxy、placeholder、WS经典Upgrade走所需H1；核真实 resolved feature paths | reqwest public配置可用；root未显式启http2而lock有h2均不能证明有效feature union。未来正确启用/审查实际graph，再证明公开入口；不把所有Bedrock降为H1 |
| route/proxy/TLS tunnel | 每路径测 scoped/ambient/空值/NO_PROXY 正反例，HTTP/HTTPS proxy、outer TLS→CONNECT→inner TLS、overread；proxy只见自身auth，target key/body只在合法内层；proxy失败零直接旁路 | 私有冻结route/identity和唯一acquisition owner，RW `async_impl/client.rs:1402,1427,1546,2158`有retry/no_proxy/H1/preconfigured TLS机制。clear-ws是否CONNECT须实际参考验证；不伪称所有runtime代理相同 |
| redirect/retry/callback | 两个本地 origin 产生301/302/303/307/308、relative/invalid Location、host/subdomain/port/scheme变化、body replay/取消/穷尽；观察每leg的method/body/headers和真实send次数 | 显式按source复现，关闭reqwest叠加retry并保留source调度；follow20不推广到所有路径。不越权发送秘密/重发现身份；必要收窄记RDF正反例。custom client零二次包装仍须其真实conformance |

本组 **OPEN**；V-01/05/09/29/30/51/55/58。方法/headers、redirect与retry数量、callback时点分别观察，不归一化成同一最终字符串。

## 5. TLS／trust／签名／mTLS：不要混成一个能力

**源码依据。** RW `src/async_impl/client.rs:693起,761–770`涉及root与default crypto provider；默认builder可能先用已安装global provider，不能证明实例隔离。REG `rustls-0.23.40/src/client/builder.rs:136–152`和`src/crypto/ring/sign.rs:21–78`支持匹配client chain及RSA/P256/P384/Ed25519机制；不等于默认trust与Node/Bun/OS相同。

| 细项及来源 | 未来验证方法 → 机械结果 | 可行方案／剩余缺口 |
|---|---|---|
| server trust/default roots | 固定真实runtime/启动配置/OS根，分别测公共/enterprise/scoped roots、错误host/过期/不可信及proxy-target双TLS；三个native平台记录 | source-scoped根/identity和实例显式TLS，保留验证；webpki/host DER并非默认等价。不能读任意ambient目录、安装global provider或bypass补洞。需要真实支持矩阵 |
| service-account RS256 | GA `gtoken/jwsSign.js:28–52`显式header仅alg=RS256；未来固定clock与synthetic key，验证精确header.payload及签名。先核jws4.0.1/jwa2.0.1与实际Node/OpenSSL，再测参考支持的PKCS1/PKCS8/模数/指数/多素数/encrypted/非RSA边界 | ring `src/rsa/keypair.rs:98–105,176–180,280–285`是PKCS8 V1Only、两素数等限制，私钥max4096；`public_modulus.rs:63–70`下界按byte-rounded位长判断，不能简写精确2048。不得将verification8192当signing能力。撤回typ/kid默认；jws依赖源码缺失，不能断言Pi支持所有ring拒绝key |
| ADC certificate/mTLS | GA `auth/certificatesubjecttokensupplier.js:55–73,152–211`与`identitypoolclient.js:113–125`：互斥source、leaf顺序及auth mTLS。loopback双向TLS验匹配/错配key-chain、RSA与参考支持EC/Ed、PEM/DER/SEC1/PKCS8、leaf/trust-chain顺序、池隔离和server验证 | 将subject-token的trust_chain与server trust roots分开；mTLS不等同RS256签名。以成熟库处理已证参考支持而候选拒绝的key，先审方案再改依赖；不自写密码学、猜算法或新增OpenSSL转换命令 |

GA `gtoken/getCredentials.js`拒绝P12/PFX的源码事实保留；没有passphrase来源不能凭想象增加encrypted-key能力。PEM envelope被识别也不证明算法可签。local验签成功与供应商接受分开。本组 **OPEN**；V-09/49/55/58，RDF-18。

## 6. Auth／ADC／AWS：来源、模式和副作用各有owner

**AWS源码。** `@aws-sdk/credential-provider-node3.972.82 dist-cjs/index.js:78–160`按Continue/Terminal运行 Env（profile时跳过）→SSO→INI→process→web identity→remote；`credential-provider-ini3.973.15:56–103,195–220`含role、cycle、MFA。当前Rust的explicit/env/profile局部实现不能证明full-chain。AWS `aws-sdk-sts-1.119.0/src/config.rs:1460–1467`及`src/endpoint_lib.rs:4–9`的resolver/partition env/fs，Smithy runtime1.16.0 HTTP trace和aws-sigv4 1.6.0的secret-body trace路径，仍支持前轮不采用默认SDK client的理由；不用它们不代表owned协议已正确。

SM `dist-cjs/submodules/retry/index.js:368–382,432–496,722–778`有模块加载时2026 switch、shared/env max_attempts/retry_mode；`:256–340,528–555`的adaptive含CUBIC/token-bucket与首次send等待。Pi新client每流的quota生命周期有来源，**standard-only仍不覆盖adaptive/实验模式**。

**ADC源码。** GA `auth/googleauth.js:244–284,335–362,476–509,748–765`区分default文件/平台路径/GCE、五JSON家族与explicit keyFilename；PI `packages/ai/src/api/google-vertex.ts:424–426,442–459`将auth文件与model project/location分开。GA `auth/authclient.js:276–282`、`gtoken/getToken.js:28–38`允许source POST retry，GX `build/cjs/src/retry.js:17–64,94–163`有具体次数/status/delay。GA `auth/computeclient.js:45–57`委托gcp-metadata；lock虽固定8.1.2，本轮缓存缺该实现，**metadata1s仍UNKNOWN**。

| 细项 | 未来验证方法 → 机械结果 | 可行方案／剩余缺口 |
|---|---|---|
| full AWS chain/operation与authority | 隔离HOME/APPDATA/files/process/env＋fake STS/SSO/OIDC/SignIn/ECS/IMDS；完整/半对key、profile优先、role/cycle/MFA、SSO/login/DPoP、旋转cache及ACK；记录每次source读取和网络 | 扩展既有私有source协调器、固定operation adapter＋Query/XML内存codecs。Continue才推进；选中source服务拒绝零后续身份发现。分区endpoint/错误shape/XML词法结构由固定vector验证；无隐式SDK resolver/secret trace。缺来源不能Unsupported充完成 |
| AWS retry modes | fake clock/RNG＋失败序列，分别记录send、quota、hint、redirect、初始send-token等待；legacy/2026参考进程隔离，adaptive throttling→恢复验rate state | 保持standard-only缺口显式；未来需要时在既有owner内证明具体模式，不造通用public策略trait。真实源重试与generation counter分开，不新增统一send cap或将helper hint上限套Bedrock |
| ADC来源/target/retry | 五JSON/external子类/direct metadata分别跑public入口；invalid所选file终止，Models缺文件零metadata；Google aws1零额外Bedrock身份发现；POST503→success、no-response、cancel/redirect/mTLS/短TTL/source replacement | 私有五家族adapter、source-scoped route/真实expiry/单flight/身份分key。portable well-known与无gcloud作为RDF差异。metadata timeout先补exact源码/options，再跑loopback；不从AWS或通用Gaxios猜值 |
| ADC executable | GA `auth/pluggable-auth-client.js:25–33,168–216`真实30s及5–120s、env gate/cache/字段；`auth/pluggable-auth-handler.js:68–87,115–169`quoted argv、继承env、stdout+stderr合并、直接child.kill。未来harmless fixture测gate先于cache/spawn、valid cache零spawn、missing/stale与malformed区别、argv、stderr、timeout/后代退出、无SDK写output_file | 保留source的合并输出解析，stdout-only不能默默替换；scoped frozen child env、kill/reap tree为明确Rust安全/生命周期差异，不冒称Node保证。无第二默认deny、无新64KiB cap；process只在既有trusted config/host授权下可达 |
| rotation/unknown effect | fake source/store gates：等待锁取消、已开始rotation后caller离开、ACK失败/unknown、logout/replace/多key/跨Models cutoff；检查晚写fence、唯一凭据保全与实际ACK | 使用既有store-owned mutation owner/barrier，不把普通access acquisition都变不可取消。Pi resolve.ts15s仅refresh网络；10s网络＋6s ACK不因伪总15s失败。runtime/crash/外部不合作store限制照实报告 |

本组 **OPEN**；V-05/09/12/43/55/58。来源协议、模式与副作用的局部case通过不能推导full chain或任意账户可用，真实服务接受另需相应证据。

## 7. Native telemetry：安全的poll/drop方案仍需真实组装

**源码依据。** PI `packages/ai/src/types.ts:160`、`api/simple-options.ts:48`和`test/telemetry-options.test.ts:68–165`证明显式option传播意图，不是Rust跨Dispatch证明。REG `tracing-0.1.44/src/span.rs:1363–1371`有 `Span::with_subscriber`，可同步取得匹配Span的Dispatch，disabled返回None；`src/instrument.rs:266–286,315–322`覆盖Instrumented Future poll/drop，`:380–389`的WithDispatch覆盖Future poll而非完整inner Drop，也没有对应Stream impl。`tracing-core-0.1.36/src/dispatcher.rs`的with_default/Dispatch::none是同步机制。

**可行方案。** 继续私有Noop或owned Span＋源Dispatch carrier；不在poll时再采ambient。每次poll/poll_next/同步观察/Drop都在captured Dispatch与parent scope中短暂执行；不留guard跨await。可用owned `Option<Pin<Box<F/S>>>`，Drop时取出整个pinned box并在同scope释放，避免移动pinned inner；这是安全Rust的候选机制，不是已编译实现。producer task自身的poll/drop也须覆盖，仅在spawn时in_scope不足。Noop在Dispatch::none下运行业务一次；不创建默认domain spans/status/exporter，不替host判断语义结束。

**验证方法 → verify：** 记录subscriber A/B；A下创建parent并capture，B下首poll/Pending/resume/跨线程及未poll Drop，Future、Stream、producer分别在poll/Drop发marker/child并计数。预期都归A、B零误归、scope后恢复B、无多poll/业务重跑；disabled/omitted/noop不接ambient且原业务仍一次。再越过SDK native/simple/complete/advanced/image/classifier/deferred及重试/cleanup，验原Result/partial/usage/功能callback错误和send数不变，serde/wire无carrier/default secret canary，可信recorder失败不改业务。任意subscriber panic/block/abort不在隔离保证内。

本组 **OPEN**；V-46–48/58，RDF-14。Future组合器源码可用不等于Stream/Drop/producer接线通过；Span clone存活不是业务未结束或已完成的证据。

## 8. Cargo／native平台／catalog产物：源码与交付面分层

**源码依据。** root Cargo MSRV1.97；reqwest0.12.28的http2为独立feature，root默认feature关闭；但未解析实际feature union，Cargo.lock里有h2不能替代解析。所选Smithy Query/XML0.62.1声明MSRV1.94.1且build=false，仍依赖schema/runtime-api；ring0.17.14与rustls0.23.40含build.rs，ring含native C/assembly。p2560.13.2的rust-version=1.65等manifest下界不能证明完整closure和三平台。WS候选尚不在本项目lock，不能称现有locked依赖。

| 细项 | 未来验证方法 → 机械结果 | 可行方案／剩余缺口 |
|---|---|---|
| 完整graph/feature/native/MSRV | 正式准入后由Cargo生成lock，记录 `cargo metadata --locked`、`cargo tree --locked -e features`和reqwest/rustls反向feature路径；审每个build/native/proc-macro/来源/checksum；按native平台编译和本地conformance | root集中声明精确候选，避免第二套TLS/provider/default resolver；离线缺归档照实报告。不能手改lock、称default-features=false即全闭包纯Rust，或借本轮文档准入依赖 |
| 独立package/native可用性 | workspace外consumer只用已发布包/资源/文档host依赖，覆盖公开入口和offline资源；Windows/macOS/Linux各记录目标/toolchain/实际命令及H1/H2/TLS行为 | 不用monorepo dev依赖凑成功，不宣称WASM/browser/任意executor。保留external Cargo cache；不cargo clean、不造session target或自动下载补证 |
| hydrated catalog | PI `.gitignore:11`忽略data，`packages/ai/src/providers/google-vertex.models.ts:4`导入本地缺失JSON；`scripts/generate-models.ts:3495–3503,3537–3539`生成拓扑，另有live fetch | 未来固定匹配revision的惰性release/hydration artifact与schema/hash/minimum-version，再用offline目录/store/public consumer证明记录、selector、key isolation、晚publication。不能运行generator悄悄联网、从测试或defaults重建后冒称原产物；wire/供应商接受仍分开 |

本组 **OPEN**；V-20/42/49/56/58/59。CI已有三平台路线只证明流程存在，不证明本候选在那些平台通过。

## 9. 未来验证的最小证据链

1. `固定输入 → verify: source/归档/lock/hash、Pi实际runtime与build/构造器、OS/trust/env/启动参数`。缺runtime/source时先标未知，不能借别的npm包补结论。
2. `固定语料 → verify: Pi公开动作产生expected及真实失败，再对Rust公开动作`。Schema可补直接TypeBox定位，但最终必须经过Pi normalization/validation；HTTP/WS必须真实adapter，MockProvider不能证明wire。保留原始输入、effective值、raw错误来源与脱敏外部错误的区别。
3. `观察状态/副作用 → verify: accepted/rejected/error阶段、回调顺序与次数、真实sends/redirect/retry、cancel、publication、socket/process/store完成`。只归一化无关clock/request ID；不抹掉Unicode、身份、backend拒绝、knownness或SSE回落。
4. `验证补救 → verify: 公共扩展点可达＋局部机制＋外部consumer/组装/平台`。候选修补/重选先审供应链与维护成本，再按既有准入流程实施；不先造完整兼容引擎或公共框架。
5. `判断关闭 → verify: 对应既有V实际记录及仍缺输入`。局部已证事实可收窄问题；一个未实现、未运行或参考未知的细项仍使对应组OPEN。若补救最终需要牺牲目标能力或扩大正式权限，再提交具体取舍给用户，当前无需预先选择未经验证的fork/库。

本轮产品测试影响 **none**：add/update/delete均0。实际执行只有文档/源码身份和结构检查，命令结果以冻结证据为准；上述loopback、consumer、差分runner、Cargo解析/编译/平台、进程、OAuth/metadata/live、hydration均未执行。源码推导的预期不标成测试通过。
