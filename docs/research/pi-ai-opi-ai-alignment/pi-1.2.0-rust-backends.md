# Pi 1.2.0：Rust 后端与五项工程合同

> 2026-10-16，非规范性候选附件。双语 [spec](spec-draft.md)／[中文](spec-draft.zh.md)明确采纳本页；[验收矩阵](acceptance-matrix.md)仍是唯一 V 索引。本页记录有条件机制选择，不表示依赖已加入 Cargo、已编译、已实施或已经获得正式准入。中文技术附件没有既有英文 counterpart；双语正文包含对应义务并共同引用本页。
>
> 输入见 [backend revision baseline](pi-1.2.0-backend-revision-baseline.json)；本页处理[架构审查](pi-1.2.0-rust-architecture-review.md)的 R-01–08。选型采用精确发布源码；参数/新增拒绝按[来源审计](pi-1.2.0-parameter-audit.md)修正，旧固定值不能作为默认实现依据；新增依赖的 Cargo 解析、完整传递闭包、平台和运行 conformance 都留给未来准入后的实施。没有安装、编译或执行下载的第三方代码。

## 1. 选择原则与适用层

保留一个 Models、一个 frozen call、一个 acquisition owner、一个最终 collector。所选后端负责明确的协议机制；来源优先级、取消、预算、cache publication 和资源所有权仍在 opi-ai。不能因为引入成熟库，就委托它自行发现另一身份、重试、代理、重定向、启动进程或换一个全局 runtime。

本页中的类型／签名是待实施的 interface 合同，不是可复制即编译的示例。保持 67 AIR、59 V 与 RDF-01–20，不建立第二矩阵或实施账本。正式 opi-spec §8.1、PRIN-002、GATE-002 保留；本页不准入新能力、不豁免单 image adapter 的 placement 证据。

## 2. 五项工程合同：来源先于默认值

### 2.1 流：有界调度不等于限制合法内容

本轮撤回的容量值逐项见[参数来源审计](pi-1.2.0-parameter-audit.md)。省略 caller resource policy 时，不施加那些 SDK bytes/count/depth/总时长上限；也不承诺 runtime/provider 无限容量。保留现 opi 队列64作为内部调度差异：Pi EventStream 没有该 count check。队列数不是内存字节证明。

| 对象 | 当前合同与来源分类 |
|---|---|
| SSE | Pi Codex/Mistral/Pi Messages 增量累积事件，没有草案的8MiB cap。按所选 adapter 的 TextDecoder replacement 语义跨 chunk 解码，完整 JSON 再解析；不能把逐 chunk lossy decode 当增量解码，也不默认新增 fatal UTF-8 拒绝 |
| AWS EventStream | 保留 unsigned32 长度、12-byte prelude、至少16-byte total、CRC 与 headers≤total−16 的 checked 关系；尽早校验 prelude。未证明的8MiB/128KiB本地 cap撤回；协议/service/runtime极限另按出处记录，不转成任意客户端拒绝 |
| WS | 按 RFC 保留 control≤125bytes/不分片、Text合法UTF-8；Binary按 Pi TextDecoder replacement 再 JSON parse。接收frame/message、outgoing JSON不另设无来源默认值；receive配置不能误用为send上限 |
| compact／retained partial／tool／opaque | 保留 Pi frame index/kind/order/closed-state 检查和原数据；无新增默认 bytes/blocks/depth cap。实际分配/解析失败 typed报告，不以“完整partial太大”静默截断 |
| progress queue／producer | 有界事件数、增量读取、最多一个in-progress event、可取消背压；无默认logical-byte cap或single-event过大拒绝。合法大事件可用单独owned rendezvous/credit debt等内部机制继续交付，具体实现须证明，不新增公共接口或磁盘spill |
| terminal／result | 一次最终Result与队列进度分离，满队列不阻断完整partial/已知usage。无独立256KiB envelope cap；诊断只保留受控code/path，不保存raw secret/body。result主动drain，不保存无限事件archive |

保留不可变event-time snapshots与内部结构共享；不规定每delta深clone。buffer增长前checked arithmetic、真实分配失败、cancel/drop/receiver close、唯一终态与producer/socket退出均可观察；handoff后不重发。合法retained message随内容增长与固定queue/transport缓冲分别计量；不称固定RSS或固定时间/token限制。显式caller policy只有被选定才冻结/执行；不能成为省略时的隐藏默认。

### 2.2 凭据：完成性不靠猜测deadline

保留对象安全、owned Send+'static 的 CredentialStore::mutation_barrier(options)／Models::credential_mutation_barrier(options)，同步捕获同store跨Models的cutoff/active/unconfirmed receipts。按provider/source-generation/resource-key/revision区分；同provider另一key成功不得覆盖未知效果，只有同key明确reconcile才清除。已确认且无引用记录可回收，不保留无限成功历史。

撤回默认30s等待、256 unresolved keys准入上限，以及九OAuth从ownership到ACK总15s/ADC总60s。Pi auth/resolve.ts给refresh网络调用15s独立signal，store lock/modify/save ACK不在该timer内；保留准确网络范围。调用方可以取消barrier等待或显式给wait deadline，得到known/pending aggregate；没有默认wait deadline。此行为不取消已开始rotation、不发auth/logout、不关闭store。不得因回收额度淘汰unknown或在无政策时拒新provider/key。

等锁仍可取消；已开始rotation仍store-owned且成功前ACK。宿主停止新auth→await barrier/核报告→关Tokio；Handle不维持Runtime存活。未知ACK保留fence，不能让仍会晚写的操作覆盖logout/replace。外部persistent adapter明确跨进程scope，任意不合作backend不能假称可强杀/quiescent；runtime/crash/remote/store failure不保证回滚或旧token有效。连接cleanup不等待/撤销凭据事务。

### 2.3 Transport：按路径复现redirect与retry

保留唯一acquisition/source owner，library low-level reqwest显式retry(never)以关闭叠加重试；真实重试次数按Pi具体generation/auth source调度。不能把所有auth协议改成0，也不统一增加总send budget。redirect不是普通retry；其实际network legs、method/body/headers与来源分别记录。

撤回所有generation/token/catalog/metadata一概redirect:none、ADC source GET仅同origin且5跳的默认政策。Pi fetch、OpenAI委托fetch、Google Gaxios/node-fetch、AWS raw HTTP的行为不同。已核Google node-fetch默认follow20，301/302/303的method/body改写及跨域敏感header stripping必须保留准确scope；不能把该20跳推广到Node global fetch/Bun/AWS。真正运行时行为未知即证据缺口，不能自填新cap。

来源身份/权限/TLS保护不撤回：不能redirect后重选账户、把target credential添加到subject source请求、绕验证或向未授权目的地发秘密/model data。若必要保护与Pi某运行时不同，必须列为RDF差异并以同意/拒绝正反例验证，不能称默认等价。custom transport不二次包，opaque client有显式conformance责任，不保证任意外部代码。

### 2.4 Schema：撤回未证明的引擎等价

jsonschema0.28.3(default-features=false)保留为有条件native引擎候选；选型尚不能关闭Pi TypeBox1.3.27验证义务。Pi普通JSON路径不按$schema切换dialect，未知format默认通过，已知format仍校验，unknown keyword不一概拒绝；contentEncoding/contentMediaType在其dispatch中未执行。原root/metadata/refs/normalization保持。strict wire副本与本地validation分开。

| 方面 | 修正后要求 |
|---|---|
| 数量/容量/工作 | 撤回schema/resource/instance bytes、nodes、targets、depth、pattern长度、全局work、validator cache容量、job/queue/error数量等无来源默认值。具体旧值逐项列在来源审计；不换另一组猜测 |
| backend默认 | jsonschema默认2020-12，支持Draft4/6/7/2019/2020；unknown formats默认ignore，format assertion随draft。fancy-regex每match默认1000000 backtracks、regex cache10均是backend事实，不是Pi预算。TypeBox默认maxErrors8是诊断设置，不是schema接受上限 |
| dialect/keyword/format | 撤回强制Draft7/仅三dialect及unknown/混合dialect、keyword、format blanket拒绝。补齐Pi实际有意义的约束/format；不能通过全部AnnotationsOnly掩盖差异，也不能把jsonschema Draft7额外content assertions加给Pi |
| ref/recursion | Pi仅在内存解析，无隐式schema HTTP/file I/O；可用explicit static retriever保持该性质，即使feature union变化。URI只是identity；不按scheme拒绝。撤回non-descending cycle blanket拒绝；实际ref/cycle终止与匹配结果必须语料验证，不假称任意cycle安全 |
| regex/error | 撤回所有pattern×所有string/key guard、复制private ECMA canonicalizer及allOf碰撞重写。它们可能拒绝无关分支且新增未证算法。锁定engine吞掉某些嵌套regex错误的事实仍是blocker；必须按实际求值分支正确处理或重选机制，不得伪成功/全局拒绝替代等价 |
| 调度/cache | 内部有界并发、等待可取消、cache可淘汰/不缓存，而不因缓存/worker tuning拒绝合法schema。实际同步计算不能被async timeout强杀；caller离开不提前释放仍运行worker的permit；不承诺CPU毫秒上限 |

验证语料至少包含unknown format、uuid/url/json-pointer-uri-fragment、unknown $schema、混用旧items/prefixItems、inert contentEncoding、ECMA whitespace U+1680、metadata及irrelevant regex branch；期望来自固定TypeBox/Pi源码，不是候选engine自证。八项源码导出正例在来源审计；尚未运行。保留clone-before-normalize、JS numeric/String语义、原schema完整验证与typed脱敏错误；修正Pi root-conversion验证漏洞须单独标记正确性差异，不借此添加全局资源拒绝。

### 2.5 Custom API：类型安全不等于实例准入门槛

保留closed OperationKind Chat/Image/Classifier、开放byte-exact ApiId；不添加Pi未定义的nonempty/trim/case/Unicode/length语法。固定O:Send+Sync+'static的object-safe ChatApi<O>/ImageApi<O>/ClassifierApi<O>与opaque native options为Rust接口表达：map_simple/validate_native/start_attempt进入同一普通/advanced driver，owned boxed Future/Stream保留。

Models::register_chat_api<O>(ApiId,Arc<dyn ChatApi<O>>)产生RegisteredChatApi<O>；handle.options(O)只封装API/type tag，不授予instance capability。兼容注册的Models可复用同一合法option值；兼容type替换不使选项仅因revision过期。不增加duplicate-ID错误，沿Pi/current opi upsert语义。真正错operation/type或实际unregistered API typed失败；不做blanket foreign-namespace/stale-handle拒绝。

call创建时冻结完整record/adapter/options；后来替换/删除不改旧call。内部revision用于捕获隔离/late-publication，不变成任意新call拒绝。Prepared request含选定身份、context/projection/compat/transport/options，不重lookup model ID；custom adapter仍trusted非sandbox，functional callbacks/cancel/telemetry仍可达。外部consumer须证明相同option跨兼容Models、同type替换、换type失败、旧call稳定与无第二driver。

## 3. 源级机制与尚未关闭的backend约束

### 3.1 HTTP、TLS、proxy

保留reqwest0.12.28、现Tokio1.52.3/futures0.3.32，以及rustls0.23.40/tokio-rustls0.26.4/ring0.17.14、webpki-roots1.0.7/httparse1.10.1的机制意图；本轮不改Cargo/features。后续root workspace声明、完整锁图/feature/native/MSRV/平台审查仍必需。原仅H1与webpki-only配置不能被称为完成Pi默认对齐。

WS classic Upgrade及明确ProxyPlaceholderSigning需要H1；Pi普通Bedrock默认H2，proxy/FORCE_HTTP1才改H1，须保留该能力或公开未关闭差异，不能因当前features不启H2就默认剥夺。TLS1.2/1.3、SNI/hostname/server验证、禁止bypass及实例显式crypto provider保留；不安装global provider。webpki与host DER roots不能冒称Node/OS/runtime根证书默认等价；企业CA/系统trust覆盖须证明或保持backend差异，不自动读取越权ambient目录。

构造/静态读取保持零source I/O，首次需要时冻结route/TLS并按有效配置共享pool。scoped>ambient/lowercase>uppercase/protocol>ALL_PROXY、NO_PROXY host/domain/port/IPv6按Pi纯函数语料，空值沿非空fallback；Direct显式表达。Pi helper确实只HTTP(S)proxy，SOCKS/PAC拒绝有来源。custom transport不重复包。proxy/root/identity/cache不会随每次credential无界新建；由配置/owner释放而不是虚构cache容量拒绝请求。

### 3.2 Codex WS

tokio-tungstenite/tungstenite0.28.0 provided-stream API保留为条件候选；default-features=false，handshake/rustls-tls-webpki-roots，不启自动connect/native-tls/url。已有route stream→client_async_tls_with_config＋instance rustls；不让后端隐式proxy、重选auth、reconnect或fallback。

HTTP proxy CONNECT；HTTPS proxy先outer TLS验证proxy，再CONNECT/inner TLS验证target；不把target key/body发给proxy，保留CONNECT后overread tunnel bytes。Pi connect15s保留其连接作用域；不另猜own CONNECT64KiB/124-header cap。clear ws也CONNECT的既有候选机制可能不同于运行时proxy，保持待证明覆盖差异，不以proxy失败自动旁路。

后端默认read/write各128KiB、max write usize::MAX、接收frame16MiB/message64MiB是backend默认，不是Pi限制或SDK推荐。撤回研究者64KiB/32MiB/8MiB/16MiB设置。receive frame/message可通过WebSocketConfig选择不加backend size cap，以免引入额外接受上限；这不等于无限RSS或无限排队。Sink readiness/一个owned driver提供背压；配置关系max_write>write_buffer由checked验证，不能触发assertion。不要用新的写缓冲cap拒绝合法outgoing message。

不可配置的Upgrade限制仍存在：64KiB response、124 headers、512 reads，以及64次read后平均至少128bytes启发式；所选版本没有permessage-deflate。这些不是RFC/Pi上限，可能拒绝合法握手或缺协商能力；须适配/重选或保持未关闭coverage，不能称调整WebSocketConfig即可完全消除。Text按RFC、Binary按Pi replacement，Ping/Pong/Close不伪装内容。3xx/Upgrade跟随取决于目标运行时，不能一概套none。

取消/cleanup禁止新内容及continuation发布，drop socket并取消/await owned本地driver/timer；撤回2s flush/peer-close默认deadline。可显式采用host shutdown deadline并诚实报告pending，不把close发起当远端ACK/退款。Pi idle5min/max-age55min保留来源：Pi有idle timer且busy跳过；候选next-idle-acquire eviction机制是明确Rust生命周期差异，不是原timer完全等价。源级未运行local proxy/TLS/fragment/cleanup conformance。

### 3.3 AWS：owned认证协议adapter＋无I/O codec

选opi-ai内的full来源协调器和STS/SSO/SSOOIDC/SignIn固定协议adapters，统一受控reqwest；保留现owned SigV4 canonicalizer和成熟hmac **0.12.1**／sha2 **0.10.9**计算。官方协议源码/签名vectors仅作固定oracle，不调用AWS服务Client/Config、runtime orchestration、aws-sigv4 HTTP signer或其private serde。

独立复审否决了前一“官方认证client＋guards”候选：Config::from_conf会在endpoint override之前eager构造会读真实partition env/fs的DefaultResolver；Smithy trace及LOG_SENSITIVE_BODIES也可能暴露token body。aws-sigv4 1.6.0的http_request::sign会trace request/canonical并通过Debug读取LOG_SIGNABLE_BODY；为两个纯HMAChelper引入整树也无价值。因此不维护环境哨兵/UA guard，不把这些库当运行依赖；不借此删full chain/MFA/login/DPoP。

| 新增直接候选 | default-features=false后的features／角色 |
|---|---|
| aws-smithy-query=0.62.1 | []；公开QueryWriter::new/prefix/finish与QueryValueWriter::string，固定schema字段/Action/Version的成熟form/query encoding |
| aws-smithy-xml=0.62.1 | []；只在下面严格预检后的bounded XML上用Document/ScopedDecoder读取固定shape；不能把decoder当validator |
| xmlparser=0.13.5 | std；无I/O Tokenizer作完整lexical/结构/DTD/entity/预算预检，树/namespace/实体规则仍由owned有限stack验证 |
| p256=0.13.2 | ecdsa/pem/std；SEC1 key→ES256 DPoP/JWK，注入clock/RNG；不自写椭圆曲线算法 |
| sha1=0.10.6 | std；标准SSO cache文件名兼容散列，不用于认证签名 |

不增加aws-config、aws-sdk-sts/sso/ssooidc/signin/bedrockruntime、aws-runtime/aws-types、aws-sigv4/credential-types的直接运行依赖。Query/XML的schema/runtime-api/types传递类型依赖不等于启动HTTP/default resolver/orchestrator；必须未来审真实锁图，不能声称整个闭包不存在这些类型包。选定codec调用路径只处理受控内存，无env/fs/network/trace/clock或全局初始化。reqwest池、clock/RNG和来源I/O仍由已有seam显式注入，auth-source retry由单source owner调度，不与generation counter混合。

| 必需操作 | 固定wire／身份 | 响应／失败合同 |
|---|---|---|
| STS AssumeRole | POST /，application/x-www-form-urlencoded，Action=AssumeRole、Version=2011-06-15；QueryWriter编码；source credentials SigV4、service=sts | RoleArn/RoleSessionName/ExternalId/DurationSeconds/MFA SerialNumber+TokenCode等profile字段保留；XML Credentials的AccessKeyId/SecretAccessKey/SessionToken/Expiration、AssumedRoleUser/provenance必需校验；ErrorResponse Code/Type/RequestId按固定source分类 |
| STS AssumeRoleWithWebIdentity | 同version Query，Action=AssumeRoleWithWebIdentity；injected WebIdentityToken、RoleArn/session/duration；无source SigV4 | token文件失败/STS拒绝按已定规则终止；credentials/expiry/subject/audience/provider元数据保真，不发现另一身份 |
| SSO GetRoleCredentials | GET /federation/credentials?role_name=…&account_id=…，x-amz-sso_bearer_token，selected region/endpoint | JSON roleCredentials，expiration是epoch milliseconds；缺值/无效/服务拒绝typed终止，不能当unconfigured |
| SSOOIDC refresh | POST /token JSON，clientId/clientSecret/grantType=refresh_token/refreshToken | accessToken/expiresIn/refreshToken/tokenType、OAuth错误/slowdown/throttle和cache写回；不自动login |
| SignIn login_session refresh | POST /v1/token JSON，clientId/grantType=refresh_token/refreshToken，owned ES256 DPoP（JWK/x/y/jti/iat/htu/POST） | accessToken中的accessKeyId/secretAccessKey/sessionToken三项AWS凭据，account仅沿冻结cache/session ARN来源保留；expiresIn、rotated refreshToken；AccessDeniedException及TokenExpired/UserCredentialsChanged/InsufficientPermissions按固定链规则，不能把login当SSO |

纯endpoint规则以选定partition/region/fips/dual-stack/明确override为输入，使用已读官方发布的endpoint数据语料；不调用SDK默认resolver、不读取SMITHY_CLIENT_SDK_CUSTOM_PARTITION。未知region处理沿现source合同，不能静默选us-east-1。各operation/source的auth retry沿固定Node机制，不能在缺证据时设统一0；已有ECS/IMDS/SSO指定重试/节流按source限定；所有实际sends由同source owner/clock/cancel调度，不增加虚构总send cap，不叠reqwest或vendor retry。错误保留source/operation/status/标准code/requestId，去掉secret/raw message，unknown code typed失败仍经Continue/Terminal分类。

**XML/Query边界：** 撤回body/query1MiB、depth32、tokens16384、attributes4096/per-element256等无Pi依据本地默认值。保留完整lexical/结构、长度checked与所选STS固定shape；不读取外部entity资源或默默截断credential。Smithy next_tag可能略过token错误、try_data可能只取首段，故不能把它当validator：完整收集text/comment间text/CDATA，保留unknown可忽略字段，必要credential字段按真实schema处理。DTD/namespace/duplicate/scalar策略须对照固定Node XML行为，不把任意更严格XML子集冒充协议要求；未证明差异保持coverage blocker。拒绝真实畸形/歧义credential且不把秘密写诊断，不等于授权添加容量阈值。

来源顺序保持Env（explicit/scoped profile时跳过）→explicit SSO→INI→process→web identity→ECS **or** IMDS。每段返回Continue/Terminal/Success，不复制Rust默认链ECS失败后继续IMDS；选中服务401/403不重开发现。typed/scoped pair缺半不混ambient；profile、source_profile、role/MFA、web/source互斥和cycle先验证。

| 来源 | 真实实现与状态／副作用 |
|---|---|
| INI／role | owned读取注入config/credentials并字段overlay，STS执行role；MFA由注入presenter给token_code，无presenter typed终止，不自动terminal提示 |
| process | 既有ProcessRunner seam；选择命令/child env/cwd冻结，Pi锁定provider-process直接execPromise且无显式timeout/maxBuffer；现opi10s/stdout64KiB为已实现差异，stderr现为null，不推广为Pi默认。候选默认须对应选定运行时机制/显式host政策；保留进程树清理；同一发现轮同profile执行一次；继续资格沿固定Node语料 |
| web identity | 注入token文件→STS；实际token I/O/服务失败终止，不能换账号 |
| SSO | 标准cache路径/hash/schema；现代OIDC refresh/SSO role credentials、legacy有效token；5min提前/30s节流和写回真实发生，不自动login；失败可保留尚有效token但expiry不伪造 |
| login_session | 标准session.trim SHA256文件名/cache，SignIn CreateOAuth2Token，owned ES256 DPoP的typ/alg/jti/iat/htu/POST；按固定来源规则处理refresh失败/未过期旧token与cache ACK |
| ECS | owned HTTP metadata，relative优先full、token-file优先token、host/protocol校验；1s attempt与modern≤4 sends/legacy1沿固定语料，已选ECS最终失败不转IMDS |
| IMDS | owned有界v2 token/role/credential，保留明确v1-disable、IPv4/IPv6与401/token分支；不在构造探测，不由neutral resolver延长effective expiry |

来源缓存按选定source/account/revision，single-flight；近expiry后台refresh须登记进§2.2同一credential mutation owner，原caller离开也可await其completion。SSO/login子token cache与外层temporary credentials是不同key/revision，写回receipt不能混为“都已保存”。文件改变可显式invalidate，不保留永恒module promise。

Bedrock generation保留受控reqwest＋增量EventStream和现owned签名；HMAC/SHA来自成熟已锁crate，以官方SigV4向量/精确HTTP oracle验证规范化而非重写crypto。headers签名前冻结，每次实际send只同凭据重签timestamp。Bearer/SigV4互斥，ProxyPlaceholderSigning用固定非secret credentials、明确nonstandard endpoint/region和强制H1，零真实source读取。默认日志/host TRACE/LOG_*环境都不能输出auth body/key/canonical秘密；选定纯codec/owned adapter没有vendor隐藏日志开关。

V-05/09/20/39/49/51/55/58涵盖所有来源、MFA/SSO/login/DPoP/cache、多key ACK、scope canary、evil XML/Query及实际签名；source级选择不表示解析/编译或这些实验已经通过。采用owned协议增加维护成本，换取来源/transport/日志隔离并减少runtime guard；后续版本跟进必须同步官方operation schema/error/endpoint和协议语料，不能只更新crate版本号。

依据：[Query公共codec](https://docs.rs/aws-smithy-query/0.62.1/aws_smithy_query/)、[XML codec源码](https://docs.rs/crate/aws-smithy-xml/0.62.1/source/src/decode.rs)、[xmlparser](https://docs.rs/xmlparser/0.13.5/xmlparser/)、[STS Query schema](https://docs.rs/crate/aws-sdk-sts/1.119.0/source/src/protocol_serde/shape_assume_role_input.rs)、[SSO operation](https://docs.rs/crate/aws-sdk-sso/1.114.0/source/src/operation/get_role_credentials.rs)、[OIDC schema](https://docs.rs/crate/aws-sdk-ssooidc/1.116.0/source/src/protocol_serde/shape_create_token_input.rs)、[SignIn schema](https://docs.rs/crate/aws-sdk-signin/1.27.0/source/src/protocol_serde/shape_create_o_auth2_token_request_body.rs)。这些服务包只是只读固定源码证据，不成为运行依赖。

### 3.4 Vertex ADC：owned有界协议adapter＋成熟签名

选opi-ai私有ADC adapter，复用reqwest0.12.28/rustls0.23.40和现有serde_json/base64/time，直接使用已经锁定的ring **0.17.14**、rustls-pki-types **1.14.1**（两者default-features=false，std）。不引Google auth/service SDK、JWT聚合依赖树或JS runtime，不fork/vendoring官方源码，不自写密码学原语。

排除官方google-cloud-auth **1.17.0**：其MSRV1.91不是问题，实际user/external-account/STS交换内部Client::new/builder无公开通用transport注入，reqwest0.13.4/default aws-lc还会引第二套配置路径；仅包装default constructor不能修复内部proxy/env/redirect。gcp_auth0.12.7覆盖不全且有gcloud路径，也不采用。代价是SDK维护下面各条Google标准HTTP exchange与source状态；以固定google-auth-library10.6.2/@google/genai2.21.0源码与官方协议语料跟进，不能称选一个库已自动得到完整ADC。

RSA采用RsaKeyPair::from_pkcs8或from_der(PKCS#1)、RSA_PKCS1_SHA256和SystemRandom；PEM用PemObject/PrivateKeyDer解析。固定JWT alg=RS256、typ=JWT、可选kid，iss=client_email、scope=cloud-platform、aud=canonical token endpoint、iat/exp=clock/clock+3600，以base64url无padding签名精确header.payload；不默加用户delegation。ring实际key构造范围/非RSA/encrypted-key不支持是backend能力边界，不是Pi通用拒绝规则；typed报告并保持覆盖缺口，不能猜另一算法或称caller增预算可修复。JSON服务账户的PKCS#1/PKCS#8 PEM和mTLS cert/key均支持；bare私钥文件缺client_email不能冒称有效ADC。锁定Node的P12/PFX本已拒绝，不为虚构的“Pi旧p12能力”新增OpenSSL或转换命令。

| Pi实际ADC家族／source | 选定Rust机制与边界 |
|---|---|
| service_account | bounded JSON身份/私钥→标准RS256 OAuth assertion exchange，保留real expiry/key/source；不把JSON token_uri任意值作为新凭据目的地 |
| authorized_user | client_id/client_secret/refresh_token form刷新canonical Google token endpoint；quota_project独立保存，无交互登录或gcloud前提 |
| impersonated_service_account | 递归解析source_credentials，不增默认4层cap，同adapter取得source token；IAM generateAccessToken保留target principal/delegates/cloud-platform/lifetime及RFC3339 expireTime，source与target身份分开 |
| external_account file/URL | text或JSON subject_token_field_name，显式URL headers/format，RFC8693 Google STS，audience/type/workforce_pool_user_project/confidential-client字段，optional IAM impersonation/lifetime；只限trusted所选config，未知/坏来源终止 |
| external_account aws1 | Google supplier自己的region env→AZ metadata、complete env pair→配置IMDS（含可选v2token）；用同一owned SigV4形成serialized GetCallerIdentity subject token＋x-goog-cloud-target-resource；不因此实际调用AWS STS，不查Bedrock profile/SSO/process，不扩张身份发现 |
| external_account certificate | use_default_certificate_config/certificate_config_location互斥，scoped证书配置路径与well-known配置；注入fs读cert/key/trust_chain，按Node顺序/leaf规则构成DER数组；显式mTLS client只用于其auth链，不污染其他provider/model client；仍验证server TLS |
| external_account executable | 保留SDK-native有界ProcessRunner，不以callback占位；所选trusted config/既有host执行授权及selected GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1仍必需；不再虚构另一个独立默认拒绝开关，不能借不提供runner把已发布native能力变成Unsupported。env gate在读output_file/spawn前检查。解析兼容quoted argv，直接spawn，不插入sh/powershell；冻结child env，version1/token-type/id_token或saml/expiry验证。保留Node默认30s、config允许5–120s，不再用60s总事务截断；撤回stdout/stderr各64KiB的默认cap，取消/超时kill-reap进程树。output_file只读有效cache，missing/stale才执行，SDK不自动写输出 |
| external_account_authorized_user | HTTP Basic client auth＋refresh_token到validated token_url/default STS oauthtoken；保留quota、expires_in与返回的新refresh_token，在owner内原子更新；默认不写原ADC文件，需持久化时使用已选store ACK合同 |
| explicit direct metadata | scoped标准metadata endpoint、Metadata-Flavor:Google、token/expiry与有界HTTP；只在明确direct/default-metadata source可达，不把Models available/check改成metadata健康探测 |

Pi Models factory要求ADC文件存在且project/location才报告configured；实际direct/default GoogleAuth还能metadata。Rust保留这两个入口的区别。无显式文件时portable well-known路径为Windows scoped APPDATA/gcloud、Unix scoped HOME/.config/gcloud；这是对Pi factory硬编码~/.config与Node底层平台路径不一致的明确修正。没有可信scoped home/path时不偷偷读真实用户目录。显式selected文件读/parse失败终止，不转metadata/API key。Models缺文件检查零metadata网络。

model project优先options→stored/scoped GOOGLE_CLOUD_PROJECT→GCLOUD_PROJECT；location优先options→stored/scoped GOOGLE_CLOUD_LOCATION。ADC JSON的project_id/quota_project不替代缺失的模型target，API key与bearer互斥；quota header与target项目分开。Rust直接使用已解析target，不做gcloud config config-helper或CloudResourceManager项目探测。Node的default/well-known分支在auth自身缺项目时可能触发gcloud；显式keyFilename分支绕开该helper，不能把所有Pi ADC都说成会shell。

ADC auth-source retry按锁定Google/Gaxios具体exchange：STS/refresh/service-account显式允许POST，enabled默认额外3、no-response2，各状态/方法/backoff准确对应，不套generation counter。撤回统一0、总60s、16sends、config/subject/token1MiB、chain4和same-origin5跳；metadata1s未直接读到gcp-metadata对应来源时保持证据缺口，不从AWS推导。Google source GET使用其Gaxios/node-fetch默认follow20与method/header规则；其他source/protocol分别核实。配置source身份/endpoint不重选，caller可取消或显式给policy，不代表省略policy就自动变成无限服务容量。

同source的access cache按config/resource digest、principal/target/delegates/audience/scopes/project/quota/endpoint/TLS/mTLS identity分key，单flight、真实expiry加一次5min提前窗口；短TTL有效token可服务当前call，不循环刷新至“超过5min”。source替换/失效时晚refresh不发布；输入subject path相同不能证明实际principal相同，automatic whole-call recovery仍要求source提供稳定账号证据，否则identity-unverified。provider普通retry复用冻结token。

普通可取消token acquisition不获得rotation完整性例外；可能旋转refresh_token或写cache的工作才通过既有source/store mutation owner登记，并由§2.2屏障观察，网络refresh保持其source timeout，store成功须ACK，未设置虚构ownership-to-ACK总时限，不随原caller消失丢唯一token。background cache refresh有selected source owner、来源限定网络timeout及generation，Models最终owner释放时取消普通refresh；已开始的唯一token rotation则按屏障合同。若source store只在内存确认，报告只称内存ACK，不冒充文件/跨进程持久化。

V-09/39/49/55/58覆盖五JSON家族、每个external子类、metadata入口、native harmless executable fixture、mTLS隔离、project/quota、expiry/shortTTL、hop/retry/cancel、跨Models与source替换、无ambient/gcloud/额外AWS身份。下载与本轮选型不运行任何进程、ADC探测、真实token或metadata请求。

依据：[官方Rust auth源码](https://docs.rs/crate/google-cloud-auth/1.17.0/source/)、[锁定Node auth archive](https://registry.npmjs.org/google-auth-library/-/google-auth-library-10.6.2.tgz)、[Google ADC规范](https://cloud.google.com/docs/authentication/application-default-credentials)、[外部credential验证](https://cloud.google.com/docs/authentication/external/externally-sourced-credentials)、[ring RSA](https://docs.rs/ring/0.17.14/ring/signature/struct.RsaKeyPair.html)、[PEM解析](https://docs.rs/rustls-pki-types/1.14.1/rustls_pki_types/pem/trait.PemObject.html)。

## 4. 供应链、平台与验证边界

所有新增依赖仅为有条件文档意图；旧source-evidence的selected清单固定于上一轮，不自动授予本轮Schema guard/其他已撤回配置有效地位，未来从root workspace声明并用Cargo生成锁图，不手改Cargo.lock。所选Query/XML codec manifest MSRV为1.94.1、p256为1.65、WS为1.63；xmlparser/sha1精确manifest未声明MSRV，不将缺失当0，已声明的上述MSRV低于workspace1.97；未声明项及完整closure/三个平台都待未来解析/编译证明。

禁AWS默认HTTPS避免额外aws-lc C/CMake栈；现ring 0.17.14仍含native C/assembly/build.rs，rustls也有build script。最终方案不引入aws-types/service runtime的直接build路径，也不为签名升级hmac/sha2；codecs传递schema/runtime-api仍可能引proc macro，最终build脚本/原生闭包数量须实际审锁图。必须审manifest/features/build/native/proc-macro/transitive与来源，而非声称“全是Rust所以无供应链代价”。不为42个profile预建42个feature/crate，不以feature flag豁免正式准入。

本轮只验证源码、公开签名和文档一致性。future Cargo解析、外部consumer编译、Windows/macOS/Linux构建、真实本地adapter conformance、内存/性能实验、OAuth/vendor接受性都未执行。下载source不是安装/执行，source-only审查也不是compile proof；测试影响none。
