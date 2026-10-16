# Phase 20 验收设计与追踪

> **Phase 20 注册来源的唯一验收矩阵：仅 `opi-ai`，对照固定 Pi 1.3.0 公开能力及可观察语义。** 本页保留[版本审查](pi-1.2.0-review.md)和[五问补核](pi-1.2.0-followup.md)的来源窗口，保留 AIR-001–056 / V-01–48 的追踪身份，追加 AIR-057–067 / V-49–59，不复用旧编号。共 **67 个 AIR 追踪项＝57 个 active SDK 义务＋9 个下游影响＋1 个退役项；59 个 V＝54 个 active SDK 验收设计＋4 个仅下游影响＋1 个退役项**。注册不等于来源／任务图准入、实施或执行通过，七组产品覆盖仍 OPEN。

> Phase 交付目标的验收设计；2026-10-03 原稿、2026-10-04 SDK 复审及 2026-10-16 研究修订均保留原证据窗口。[正式规范](../../opi-spec.md)继续决定范围、归属与实施准入，[双语交付来源](spec-draft.md)固定本页采纳边界。Q22–37（含修订 Q31）与 [D-19–37](decisions.md)是历史确认；选定包内目标限定其适用范围。SDK 机制和必要破坏性变更调用方适配属于验收；独立产品政策仅记录影响，不能作为 SDK 完成门槛。
>
> 本页是本目录唯一综合验收矩阵。AIR 为双语候选要求，V 为验收设计；[SDK 契约附件](pi-1.3.0-sdk-contracts.md)、[Rust 差异附件](pi-1.3.0-rust-differences.md)、TP/SP/EP 及旧 TM-V/OV 只细化合同或提供证据，不建立第二验收权威或进度账本。**54个active SDK与4个host-impact均未完成实施／完整执行验收；V-52退役，不是通过**；历史 70 个离线测试与研究探针单列，不能证明新目标通过。本页为无既有英文 counterpart 的中文研究材料；双语候选共同引用此表。

> 前置最小窗口的[Schema／WS离线差分](pi-1.3.0-schema-ws-offline-validation.md)执行的是独立研究harness：Schema越过真实Pi公开utility，WS仅固定global构造调用形状与直接Rust机制；没有执行opi-ai新公开utility／真实provider组装。保留原V行，实测观察在本页末节映射，不能将研究case数量换算为V通过。

## 验证边界及证据分层

active SDK 验收优先越过 Models/native/simple/complete/result、公开直接 adapter、typed operation、管理/store、具体 adapter＋loopback 伪传输以及 workspace 外 consumer。MockProvider 证明状态/调度，不证明 HTTP；payload fixture 证明编码，不证明供应商接受；只调用内部 serializer 不等于公共入口已组装。测试专用 fake 不要求新增公开 trait。SDK 破坏性 API 所需的 Agent/产品编译、机械调用方适配及原有安全回归仍在范围，不能据“仅 SDK”留下失编译的 workspace。

**scope 规则：** V-17/18/19/33 是 `host-impact`，不计 SDK 完成。V-06/12/15/16/20/26/28/31/35/36/38/42/45/48 内的产品/Agent 场景只说明下游影响；每行明确的 SDK 子边界才是 active gate。AIR-009、015、016、017、018、019、037、039、041 共 9 个原要求保留为 `host-impact`，其产品授权、hook、session、compaction、tool_search、启动刷新、高层恢复或 UI 行为不要求本轮实现。V-57 独立验证显式 SDK whole-call retry helper，不让旧 V-33 的产品默认恢复反向成为 SDK 前提。

每个 V 给出 owner/公开边界、输入、机械结果与失败、证据限制和 test impact。`add/update/delete/retain` 是未来建议；本次仓库测试变更为 `none`。`active` 表示范围内的未来验收，绝不表示通过。只有有来源的默认限制或显式选择的caller policy才测B−1/B/B+1；没有默认policy时必须补大合法输入不被额外拒绝的正例。旧预算及新增拒绝的修正以当前双语正文和后端合同为准；[参数来源审计](pi-1.2.0-parameter-audit.md)保留其依据，旧默认值不再自动继承。时间竞争用 barrier/假时钟及可观察提交点，不靠任意 sleep 证明。完整公开能力覆盖的逐项索引和 AIR→V scope/state 在下文；测试迁移表继续保留 41＋7 全文件输入。

## 本轮 Rust 合同与后端的补充判据（不新增 V）

明确采纳[后端附件](pi-1.3.0-rust-backends.md)；下表细化既有V，不建立第二矩阵/进度或已执行结果。原41+7文件处置保留；未来add/update在对应binary/filter落地，现有有效行为retain，只有replacement proof后delete。本轮测试影响none。

| 处置 | 既有 V | 必须新增到同组的机械判据 |
|---|---|---|
| R-01 调度与接受集 | V-02/04/05/25/40/41 | 对照Pi事件/帧正例，超过旧草案阈值仍可通过：大SSE event、single/fragment WS、完整context outgoing、大tool/opaque/partial；不新增默认bytes/count/depth拒绝。现queue64仅背压、合法大event不能因credit失败；单terminal独立交付完整partial/已知usage，满queue/cancel/小policy并发有task/socket barrier。RFC control/prelude/CRC/真实malformed JSON保留；Pi TextDecoder replacement跨chunks对应。caller显式policy测B−1/B/B+1/overflow/省略；backend不可配置约束单列gap，不归一化 |
| R-02 store completion | V-12/42/43/55 | 两Models共dyn内存/persistent store，同步barrier捕获cutoff/active/source-generation/resource-key/revision，oldunknown key不被另一成功key覆盖。无默认wait30s/keys256/ADC60s；caller cancel/显式wait deadline只停等待并报告pending。10s refresh＋6s ACK成功不触发虚构15s-total失败；网络超过来源15s则typed失败，ACK未知/latewrite fence/跨进程scope/Runtime不活保证分别验证。停止新auth→await报告→关runtime；无sleep代替ACK |
| R-03 路径transport | V-01/05/09/29/30/36/49/51/55/58 | 关闭nested retry，按Pi source/HTTP机制核实际status/method/body/header stripping/retry/backoff。允许redirect正例及拒绝真实未授权credential传播负例同时覆盖，不能以统一3xx失败/ADC sameorigin5或16send通过。proxy scope/lowercase/NO_PROXY/HTTP(S)、custom零二次包、实例TLS/roots与Bedrock普通H2/明确H1分开；backend和运行时差异未证明保持gap |
| R-04 schema接受集 | V-11/34/35/39/58 | 以来源审计及[专项Schema语料](pi-1.3.0-backend-gap-audit.md#2-schema公开机制可表达性先于选型)为oracle，先证公开扩展点：metaschema前置准入/private bypass不能用、mixed applicator/ref sibling、近似grapheme而非scalar、metadata ref/$schema frame与fallible regex父节点；直接engine/适配后真实public utility分别记录。unknown format/$schema/metadata、known uuid/url/json-pointer-uri-fragment、旧items+prefixItems、content annotations、in-memory refs/cycles、ECMA U+1680及irrelevant regex branch。撤回任意shape/work/cache/jobs/error默认配额和全pattern guard。cache淘汰/worker背压不拒合法schema；branch-correct regex错误处理与backend差异未证明不关闭。无schema隐式I/O、raw/normalization/最终原schema验证保留；不是engine自己验证自己 |
| R-05 typed custom API | V-01/21/39/42/49/58 | builtin+nonbuiltin经固定O dyn adapter进入四入口；ApiId精确、不添grammar；upsert、同type替换、兼容Models共享options正例，换type/错operation/真unregistered负例；旧call冻结完整record/adapter。撤回仅因namespace/revision拒绝；无第二registry/driver，public consumer真实编译/执行待实施 |
| R-06–08 backend条件 | V-09/20/26/29/30/39/42/49/51/55/58 | Cargo closure/features/native/MSRV/平台仍未来。WS可配置size上限移除与不可配置handshake/无compression分开；own CONNECT/readwrite/close不添猜测阈值。AWS完整来源/MFA/SSO/login/DPoP/cache与source retry、XML/Query真实结构/credential/无secret日志，不按虚构XML容量拒绝。ADC五家族/external/metadata/authorized native runner/真实30s与5–120s/executable env gate、source POST retries/redirect/mTLS/expiry，撤回总60s/16sends/depth4/1MiB和64KiB stdio。后端无法保留接受集即gap，不用typed Unsupported充完成 |
| G-01 admission | V-26/58（证据边界） | 本表不豁免正式§8.1/PRIN-002/GATE-002。单image adapter/新能力placement在正式shaping另核；mock consumer不冒充真实第二consumer，不造wire/CLI以凑门槛，本轮不改正式spec |

## 已有要求的修订验收（V-01–26）

| ID／决策／证据 | Owner／触发输入 | 结果与失败的机械判据 | 既有证明及缺口 | 未来测试影响 |
|---|---|---|---|---|
| V-01／D-03/21/26/28；O-002、SP-01–03、route probe | Registry→collection→adapter；override/refresh、非启动provider切换、auth-derived endpoint、高层effort请求或native/exact | 同一选定完整模型贯穿picker/prepared/wire；pre-auth固定身份与显式数据，auth后用合法最终endpoint补齐依赖其的compat默认，不重新lookup。未知route/已知非法输入在auth前失败，auth后才能判定的问题在HTTP前失败。effort记录requested/effective，高层按已确认pi支持等级规则选择，native/exact按目标能力验证；旧call不受目录变化影响 | active override/rollback及route probe已有；尚未运行产品extra-route切换、完整compat与effort corpus。probe证明当前反例，不证明目标已修复 | add跨route/binding/effort；update prepared conformance；retain active/rollback |
| V-02／D-04；A-02、P06 | SSE framing；同一合法UTF-8/CRLF fixture在每个字节边界切块，另含无效UTF-8/JSON | 合法UTF-8/CRLF跨每字节分块保持文本/参数/事件；Pi TextDecoder replacement的非法byte输入按adapter对应，而非新增fatal拒绝。真实坏JSON/framing按Pi路径typed失败；合法keepalive与超过旧默认阈值的大event不拒绝。R-01调度/协议检查同组适用。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add共同chunk fixtures；update受影响parser；retain正常provider fixtures |
| V-03／D-04；A-04 | Prepared attempt；empty/prefix EOF、terminal+tail、重复terminal、Err+tail | 无terminal EOF产生一次typed protocol error后永久EOF；首Done/Err(CallFailure)吸收且停止读取inner，释放attempt；完整terminal/partial message/usage保留。旧provider Error映射Err，stream.result/complete、Agent、直接SDK同一结果规则 | drain empty拒绝与Agent guard已有；prepared尾事件/empty探针证明当前缺口；目标公共conformance未跑 | add adversarial MockProvider；retain消费者防御与terminal payload |
| V-04／D-04；A-03 | 每个adapter producer；queue满、connect/header/read/idle、drop/cancel | barrier确认等待点后取消一次可见且buffer不穿透；drop停止本地工作并释放HTTP读取/队列等待。允许旧attempt仅cleanup与新attempt短暂共存，不允许旧请求继续执行/交付。已交付terminal不撤回 | 既有取消测试通过；Gemini drop屏障后socket观察timeout只证明该观察窗口未见关闭，未证明永久泄漏或所有adapter行为。任务计数/全部wire待补 | add逐wire lifecycle与drop计数；update仅drain型断言；retain有界队列 |
| V-05／D-04/24/36；A-04/05、P08、UX-16 | **SDK active：** Models/prepared call→真实 adapter/伪 transport；acquisition 失败、retry override、headers、deadline、quota、strict 变体 | 每 call 至多一个可执行 attempt；同 call auth/model/account 冻结。**同 payload 变体内**普通 HTTP retry 复用已验证 payload，不重调 onPayload；Anthropic 受控 strict 降级另按 V-34/36，每变体重建并重调 onPayload，不能混称普通 retry。helper/Codex 默认 0 额外，Bedrock standard 2；ordinary retry 在每 payload variant 内按冻结 max_retries=N，最多 1+N 次 send；通常只有一个变体，Anthropic 受控降级最多两个，因此总计至多 2×(1+N)，N=0 仍可有两次 send。advanced 共享每变体 counter/总分支数，不能重置预算；唯一 owner、调用方不得叠加。按锁定协议分类、hint 优先级、指数/jitter/quota；Bedrock 裸 501/505/408/403 不重试，THROTTLING/TRANSIENT 才准入，U=0.5 首次 transient 50ms、10s hint 5050ms、90s hint按Smithy bounded hint调整，不以helper/Codex60s新增拒绝；Codex 1000/2000/4000 无 jitter，400/401/quota 不由 catch 复活。cancel/drop/deadline 使全部等待点零后续 send，handoff 后 body/EOF/parser/observer 失败零低层重发。advanced 共享累计 budget/index/quota，RetryNotReady/AlreadyActive/CallSealed 可观察；未 dispatch drop 不收费，dispatch 后未知效果封存；两个 call quota 各从 500 开始。旧配置键必要迁移报错；产品恢复只属 V-33 影响，显式 SDK helper 见 V-57  R-06显式关闭reqwest/vendor嵌套retry，R-03实际redirect/transport发送计数同验。| 历史 58 collection tests 仅证明旧 lease/auth/fallback；新分类、quota、timer、变体及边界未运行。静态依赖源码不是请求次数 proof；不声称 adaptive/ambient 实验 parity | add 协议失败序列/假 clock-RNG/quota/取消/变体计数；update retry/advanced 预算；retain 冻结与无 fallback |
| V-06／D-19/28；TP-01–06 | **SDK active：** 公共 Context→normalize→adapter；initial shorthand、content/section/tool add/remove/redefine；**host-impact：** 实现 revision 安装 | SDK 初始便利输入一次归入 leading entry；content 追加，section replace 保序、null 删除、delete+add 移末尾；工具声明按值保序，真实变更才记 delta；call 捕获 owned revision，raw 不被投影改写，无效名称/schema 在 auth 前失败。Anthropic 使用 inline-tools-2026-09-15 与 by-value tool_addition，同名重定义不得套旧 collapse 限制。宿主下一 call 原子更换实现/schema、旧工具批次继续旧实现属 AIR-009 下游义务，不计 SDK 完成 | 当前 owned Request 是有限基础；Pi 1.2.0 新 chronology fixtures 仅静态读取，未跑 timeline/native corpus 或宿主更新 | add SDK timeline/by-value corpus；update 输入消费及必要 callers；retain call 稳定性；host 测试本轮 none |
| V-07／D-19/20/23；TP-09–11、R01–08、EP-09/10 | replay/adapter；same/cross/unknown来源、重复ID、缺result、mixed text/tool、failed/Length历史 | 按批次位置＋source ID关联；闭合批次可复用ID，批内歧义/重复或orphan result失败；request-local双射同步call/result。可关联缺项补ResultUnavailable/UnknownEffect，不改canonical、不声称NotExecuted、不重执行；Error/Aborted及Length细则见V-15。mixed次序、Responses双ID和签名来源保真 Pi1.3新增：provider/API/model任一不同即drop全部thinking，不转可见文本；同身份signed/redacted空块保留，canonical不变。 | Anthropic signed roundtrip及既有serde存在；完整batch/缺项/跨wire矩阵未跑。opaque全族另见V-32，不能声称只有Anthropic就完成 | add已确认replay corpus；update旧缺项全拒绝；retain原始history与真实副作用事实 |
| V-08／D-06；O-009、M-12 | tool-result图像preflight/serializer；bytes/base64/URL×user/tool×model/wire | 支持的wire收到实际图像并保留tool关联、顺序及失败标志，否则在适当preflight边界typed拒绝；不以MIME占位冒充图；private details不进wire，URL不自动下载 | user image与ToolResult serde存在；默认内置producer无调用证据不能证明库能力不存在，也不证明tool-image wire已交付 | add工具图wire/拒绝；retain user/serde/no-leak；update明确投影行为 |
| V-09／D-21/28；SP-05–08及SDK附录A/B/D、O-007/012 | SDK Bedrock resolver＋产品装配；typed/Bearer/explicit/scoped/ambient profile、完整/部分env pair、process/metadata及近过期cache | SDK独立使用同resolver，产品注入来源/策略；Bearer与SigV4不混发。explicit/scoped profile与ambient profile按已核实规则区别；已选typed/scoped不完整来源失败关闭，不拼ambient。以固定source corpus验证Env→显式SSO→INI→process→web identity→ECS或IMDS及INI子来源，只有typed continue-eligible发现失败前进；已选HTTP401/403不另寻身份。有效近过期凭据可与单一同源refresh并存，过期须resolve；原expiry/provenance、file invalidation、refresh迟到屏障准确。公共SDK DefaultChain经fake STS/OIDC/metadata装配可达；显式 ProxyPlaceholderSigning＋非标准 endpoint＋HTTP/1.1 经普通 SDK/direct 真实 adapter→本地代理，固定非秘密 SigV4、冻结region，无 env/store/profile/process/metadata/bearer 探测；普通 AWS endpoint 拒绝，ambient 真实 credential canary 不泄露，不声称 unsigned/真实 AWS 身份；取消/进程树释放/secret canary有效 | 锁定Node发布源码已补齐chain/expiry/cache/skip-auth证据，见SDK附录A/B/D；当前opi local chain及SigV4不证明全部目标。Rust后端已选定，但未完成Cargo闭包/编译/平台/运行conformance，本轮未执行credential corpus；不再要求移出opi-ai，不复制dummy SigV4为NoAuth | add SDK-only/source/expiry/fake-HTTP覆盖；update来源顺序/绑定；retain安全失败、SigV4、进程保护 |
| V-10／D-02/21；O-003、SP-16 | SDK public surface＋所有当前调用方；旧Config/Model/Error/ProviderKind壳退出 | 明确退役symbol/re-export/当前docs/examples归零，编译全部当前消费者；新typed catalog/error/auth入口保留实际SDK配置能力与ApiKind持久化。无无限期同职责shim；未知外部consumer不能由仓库搜索否认 | 仓库内旧壳无生产consumer证据较强；外部兼容未知，需发布迁移说明，非本轮发布授权 | delete构造存在型测试；update编译/docs；retain真实route/auth/历史reader |
| V-11／D-21/26/28；O-004、SP-01–04/17/18 | 单一compat normalizer＋配置parser；supports/role/store/bridge/effort格式与旧键迁移 | 先用公共wire fixture证明store省略/SendFalse、reasoning条件role、tool→user/image-user bridge及已声明thinking格式的on/off/null/省略；显式false与未配置不同。有效语义补齐或合并后旧键才能退出；各配置位置旧键明确migration error，静态无作用reasoning_effort退出而请求effort保留 | 当前部分字段无wire读点是缺口而非删除能力依据。现有role/strict/endpoint有效；完整pi推断及迁移corpus未跑，默认不能全部false冒充对齐 | add行为及旧键负例；update schema/config；delete纯字段存在测试；retain有效compat |
| V-12／D-09/24/28；O-006、SP-08、EP-12；R3 | **SDK active：** AuthResolver/store/Models；并发近过期、失败、waiter cancel、低层 retry；**host-impact：** 产品恢复 | 同 prepared logical call 一次解析并冻结 auth；低层 retries 不重新选来源。等待 store-owned 锁的 caller 可取消；**宿主 runtime 存活时，refresh一旦开始仅网络使用独立15s signal，store ACK不受该timer，caller 停止等待不能中断其刷新/原子保存**；Handle 不维持 Runtime 存活，shutdown/abort/timeout/remote或store failure 不保证保存或远端回滚，不冒称旧token仍有效。其他 waiter 复用已提交新 token，不重复 refresh；失败无半凭据/账户 fallback；logout/replace 与刷新同事务序列化，迟到 refresh 不复活已替换身份。取消生成零后续 dispatch，不因已保存刷新自动恢复。显式 V-57 新 call 可重解析同源 auth；产品 recovery identity/持久化政策只属 V-33 | 历史 collection tests 仅低层冻结；新 OAuth rotation/store transaction 仅源码证据，尚未实现、未验证多进程。不能用 Models-local mutex 充 shared store proof | retain 冻结/fail-closed；update 旧取消终止 rotation 断言；add 多 waiter/ack/logout/replace races |
| V-13／D-09/27；P17、A-05、SP-14 | 错误/诊断producer与只读observer；恶意body/status/URL/header/stack、transform错误 | canary不在Display/Debug/serialized公开诊断/Agent事件；保留typed类别、允许status、脱敏provenance。观察接口只读且不改模型/鉴权，raw媒体不默认日志。未知error不转成功 | 当前closed summary比pi更严格；本轮未跑完整canary矩阵；新producer与observer需补测试 | retain已有；add新producer/observer；update错误映射 |
| V-14／D-10；O-011、M-11、SP-12 | Usage/cost消费者；absent/zero/subset/tier/溢出、多attempt | absent与明确0可辨；child不双加；mixed missing只标已报告部分；tier边界与精确累计成立，未知不能充可靠限额。重试前未报告usage不补0，估算不称账单 | 现有usage_cost仅读；此前70测试不能证明成本全覆盖；不承诺全部provider字段完整或账单准确 | retain；add恢复多attempt已知性；update受影响序列化 |
| V-15／D-20/22/28；TP-08–16 | **SDK active：** 公共工具参数 normalize/validate、replay projection；坏 JSON/schema、stop reason；**host-impact：** hook→授权→执行 | SDK raw 不变，plain-JSON normalized 按原 schema 校验；失败为 typed 可区分结果，不产生执行许可；Length/Error/Aborted 的 request projection 依 V-07，不能虚构 NotExecuted 或抹掉已执行事实。SDK 不执行工具。宿主只读 rejected proposal、trusted hook 可改实际值且 final authorizer 绑定实际值、deny/cancel/Length 零执行，是 AIR-009/041 的保留影响，不计 SDK gate；必要 API caller 适配仍须编译及保留已有安全回归 | 现 raw 参数/schema seam 不证明新 normalizer；Pi corpus 仅读未跑。hook/permission 的产品实现未由本候选授权 | add SDK validation/replay corpus；update 公共返回与必要 callers；retain 原授权安全测试；host 新行为 none |
| V-16／D-19/22；TP-02–07、EP-07 | **SDK active：** transcript compiler→各 adapter；S0→user→S1、sections/tools、native 不足；**host-impact：** active/visible/implementation revision | 默认 native-or-collapse 记录 mode/loss 且 canonical 不变；Strict 在 auth/network 前拒绝不可保时序。adapter-specific 逐变更类别证明，不以单一能力 bool 代替。Anthropic 按值 addition/redefinition；其他目标仅按真实能力 native/collapse。宿主 call 间 revision 更新、hidden declaration 不撤销合法 execution、inactive/unknown/policy-denied 零执行是 AIR-009/037 下游影响，不计 SDK 完成 Responses flag/native effort/精确sign-in负例（apiKey省略、空串加authorization header、header-only分别测）、same-identity历史marker、相同effort重复marker保留、空reply跳过、tooloutput后始终final marker与顶层medium基线／sampling和onPayload覆盖不追改stamp；Anthropic off无marker/output_config/block_binding/stamp。 | 旧 Strict/run-fixed/visible gate 已被替换；新 Pi1.3 fixtures 及 SDK 完整投影矩阵未运行，原授权回归保留 | add SDK native/collapse/Strict；update 旧默认与必要 caller；retain 授权；host 新测试 none |
| V-17／AIR-015/016/019；D-14/19/28；TP-17 | **host-impact，仅追踪：** Session read/resume/fork/export；v1/v2/v3、旧 tip/sibling、unknown、capture 关闭 | 保留原产品候选影响：旧 bytes 不变、legacy child 与 unknown、required/version fail-closed、单支及脱敏；普通动态工具更新不强制 child。它们不进入 SDK acceptance；SDK 的 serde/replay 自有格式另由 V-41/58 验收。必要 SDK API 更改造成的旧 reader/caller 失编译须机械适配，但不据此交付 session v3 | 旧 session tests 只是既有事实；新 v3/delta/revision 未实施、未运行，不计 SDK coverage | retain 历史测试输入；未来 host add/update 待其独立授权；本轮 none |
| V-18／AIR-017；D-17/19/27/28；TP-06/17、SP-14 | **host-impact，仅追踪：** 唯一 session/compaction 事务 owner；输入 commit/ack、final projection、crash/cancel | 保留 accepted input/ack 与首次 dispatch 前 projection 持久确认的产品影响；失败不捏造 dispatch，单 owner 不要求固定两 append。普通 SDK 没有 durable ACK/session 前提。SDK 自身 binding/auth/变体冻结与零 dispatch 负例由 V-01/36 验收；不得以此项要求本轮重写 session 事务 | 原 owner/ack 有历史依据；新 durable projection barrier 未运行且不是 SDK gate | retain 原持久化安全测试；未来 host add/update 独立；本轮 none |
| V-19／AIR-018/019；D-19/28；TP-04/17 | **host-impact，仅追踪：** compaction/reconstruction；旧 system/tool delta、多次压缩、sibling | 保留 checkpoint＋summary＋retained 非 system 历史＋后续 entries 的有损重建影响，不重复吸收 delta、不声称旧 call 逐时点等价，raw/unknown/来源保真。SDK 不拥有 compaction/session 引擎；本轮只保持必要 API caller 适配，公开 transcript 投影由 V-06/16 验收 | Pi 源码支持 checkpoint 语义；新 session/compaction fixture 未运行，不计 SDK 完成 | retain 原分支/脱敏；未来 host add/update 独立；本轮 none |
| V-20／D-19/21/28；TP-01/17、SP-16–18 | 全调用方迁移及退役检查；SDK/adapters/mocks/hooks/compaction/config | 下游无独立Request.system/tools解释权；initial便利builder一次规范化可保留。无第二catalog选择、重复compat解释及退役壳；必要compat wire能力先完成。AWS resolver仍可独立SDK使用，不扫描要求其从opi-ai消失。当前生产 caller 退出旧请求写入路径，历史 reader 保留；session v3/迁移政策仅 V-17 下游影响，不新增产品完成义务 | 静态清除不能证明语义保留，须与V-01/09/11及公共编译一起验收；未知外部consumer仍是发布迁移边界 | add旧键负例；update所有当前消费者；delete已被替代壳/错误断言；retain必要能力 |
| V-21／D-13/23/25；SP-09 | typed catalog/collection；相同provider/id的chat/image/classifier、错operation/缺route、远端更新 | operation-qualified身份不碰撞；chat默认lookup只chat；image不虚构token window。已知错type/route在auth前失败，旧call冻结新call取完整新记录；共用auth/catalog owner，不造parallel ImagesModels | 现有chat路由有基础；多operation与远端typed目录未执行。显式注册有效但不是唯一长期目录来源 | add typed lookup/public conformance；retainchat路由 |
| V-22／D-24/27；SP-13/14及SDK附录C | Public image/classify→resolver→真实adapter→loopback；500→200、429、headers返回后body挂起、attempt timeout、可选total、cancel/drop | image默认0额外retry，SystemOne默认2，可显式覆盖；同call auth/transform一次，retry用同payload/binding，禁换provider/account。image默认600000ms逐attempt预算覆盖完整HTTP body，SystemOne默认不设attempt deadline；override按各adapter来源：image timeout由OpenAI要求positive integer，classifier supplied0交AbortSignal.timeout立即到期而不是统一InvalidInput；attempt预算每次独立、total可选且不重置。语义invalid-response/bad media不retry；cancel/deadline/result按已确认交付点处理，drop释放本地工作 | 锁定OpenAI7.19.0默认600000ms已静态核实；完整body计时来自精确OpenAI7.19.0 responsePromise/parseResponseWithTimeout；只读fetch timer会遗漏后续body timer。首轮“一次HTTP＋强制总deadline”已撤回；新end-to-end及body-hang假时钟验收未跑 | add counts/backoff/timeout/body-hang/drop；update旧统一zero-retry；retainchat独立层级 |
| V-23／D-13/25；SP-10/15、WEB01 | OpenRouter Chat-wire image；有序Text/InlineImage、image-only、first/multiple choice、媒体/预算边界 | 请求/首choice的text和data-URL图像保序，额外choice不拒绝。按Pi保留text-only/空image成功、跳过非匹配URI、fake-png成功fixture；无默认图数/bytes/MIME/signature/strict-base64/必须有图拒绝。owned表示保留encoded原值及必要decode差异，不丢Pi已返回结果；无下载/写盘/像素decode。省略policy的大合法输入和显式policy边界分测。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add Pi正例/差分/边界与公开consumer；update旧强拒绝断言；retain协议/取消/安全回归 |
| V-24／D-25；SP-11/15、WEB02 | TypeSafe/OpenRouter System One；choice/score/bool、缺/额外ID、label、非finite及domain | System One两binding真实bool↔noul/parser；缺answer/错type/非finite失败，额外question IDs忽略。没有统一2–255 labels/score>=2/unique levels、概率[0,1]/membership/完整probability keys/score domain/sum/argmax额外拒绝；按Pi有限provider值保留。llama choice2–62/score2–10、Decisions image128及各modality拒绝只作用本协议；test正例不能只测严格负例。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add Pi正例/差分/边界与公开consumer；update旧强拒绝断言；retain协议/取消/安全回归 |
| V-25／D-10/24/25；SP-12/15 | operation outcome及预算reader；usage存在但答案坏、absent/0/invalid bucket、未知价格、超限 | 保留typed outcome/known usage/secret-safe；absent/invalid不充0，坏答案保留已知usage。省略policy时无统一body/depth/count/总deadline，较大合法输入不拒绝；显式policy才冻结并测B边界/checked overflow/真实allocation失败，不静默截断。0/omission/sentinel按各来源，不能统一InvalidInput；image timeout600000覆盖非流式JSON body/解析剩余attempt时间，不能误按headers-only；Mistral60000仍仅headers。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add Pi正例/差分/边界与公开consumer；update旧强拒绝断言；retain协议/取消/安全回归 |
| V-26／D-13/23/27；SP-09–15、placement | **SDK active：** typed image/classifier 公共入口→resolver→真实 adapter/伪 transport；依赖/外部 consumer；**host-impact：** 终端工具与产品目录 | 四 classifier＋image 必须按 V-50 各协议真实组装；复用 wire 不冒充新算法。SDK 不自动下载写盘/执行工具；终端目录发现、权限、启动刷新仅下游影响。依赖按 exact lock/feature/native/build 影响审查，保持无内部 Opi runtime 依赖；缺缓存依赖记录 gap，不自动安装 | 归属已明确 opi-ai；新的实际 adapter、公共组装、依赖 conformance 未运行，不能只数入口 | add 全协议 public→fake HTTP/外部 consumer；update 必要 callers；retain 现有安全/无产品依赖 |

## 新增已确认能力的验收（V-27–38）

V-27–38 保留与 AIR-033–044 的原主关联；本轮依据 scope 将 V-33 标为纯 host-impact，V-28/31/35/36 分清 SDK 子边界与下游影响。真实 remote deferred 和完整 MCP/codemode 不由离线 fixture 推定交付。锁定 Node SDK 来源链是历史静态证据；Rust 机制及同契约 conformance 仍待实施验证。

| ID／AIR／决策／证据 | Owner／触发输入 | 结果与失败的机械判据 | 既有证明及缺口 | 未来测试影响 |
|---|---|---|---|---|
| V-27／033／D-23；EP-01/03、EPT-01 | opi-ai catalog publisher/store；A挂起→B新revision→A迟到，provider替换/删除、auth可用性更新 | 内存与重开store的当前发布不回滚；在fetch/锁前/持久提交点插barrier，区分合法旧提交与新revision后的stale写。owned旧request稳定，新read/call看到完整snapshot；取消不迟到publish，一provider失败不清其他有效目录 | pi generation/read/store有源码与测试断言，本轮未跑；现opi独占refresh不能证明新异步产品场景。自定义store忽略取消需目标事务/owner保证而非照抄双if | add publication race/reopen/partial-provider；update目录owner；retain稳定snapshot |
| V-28／AIR-034；D-23/35；EP-02/03、EPT-02/03、UX-13 | **SDK active：** new/register/lookup、显式 refresh(false/true)、取消/混合结果；**host-impact：** 产品启动/缓存协调 | SDK 构造/注册/静态 lookup 零 store/env/network；refresh(false) 仅显式恢复缓存、零 OAuth refresh/网络。refresh(true) 选定 configured dynamic providers，unknown/static/unconfigured 不联网；typed report 包含 aborted/各 provider errors，混合成功发布，失败保先前有效目录。configured baseURL、OpenRouter 显式 personalization、Radius 空目录权威及版本协议见 V-56。产品 UI/RPC 后背景刷新、4h freshness/4s attempt/2额外 GET retry/15s startup、ETag/缓存 tombstone/logout/coordinator 仅 AIR-034 下游政策说明，不计 SDK gate | 旧产品源码/测试仅读；SDK 显式 refresh 机制与 fake store/HTTP 尚未验证，不把产品 wrapper 当裸 factory 默认 | add SDK 零 I/O/显式 refresh/混合错误/取消；update SDK 管理；retain 离线 snapshot；产品新测试 none |
| V-29／035／D-23/30/35；EP-04、EPT-04、UX-15 | Codex owned WS pool＋public Models.cleanup_session_resources；同/跨实例/账号/endpoint、connecting/cached/temporary/busy-overflow、active fallback、drop/cleanup重入 | 相同身份串行复用、busy overflow仍owned；sessionless/no-cache临时WS。Pi idle5min有release后timer且busy跳过，age55min在acquire检查；候选next-acquire-only idle轮换为明确Rust生命周期差异，不定时杀active generation；connect15s。cleanup首次poll为cutoff，Some精确完整非空ID、None含sessionless、Some空typed失败不清全部、有效无匹配成功无操作；Future结果typed/unit，清理步骤故障仍尝试其他资源、失败报告已知效果/未确认释放不伪装quiescence；首次poll在auth/transform等待前登记generation，cleanup取消旧matching calls含准备；caller父token只单向联动、cleanup只cancel内部call，不取消共享父token/其他namespace/后续calls，未交付终态Err/Aborted，已交付终态不变。连接/producer/timer/continuation/fallback/debug本地quiescence后await才返回，connecting取消/缓存busy/溢出/临时均覆盖，零晚publish/retry/SSE。截止点后及未pollhandle fresh generation，再调用可复用，drop cleanup不撤close标记。显式SSE/不持WS资源operations不受影响；相同ID两Models隔离、注入共享仅清本namespace，不删credential/catalog或外部clients。Models drop/remove/replace不取消owned call；最后公共owner/guard drop触发释放，background refs不能无限续命；Drop不冒充已await | pihelper公开但全局同步仅缓存socket，connecting/temporary不可见，可能关闭后SSE；目标有意加强，不复制。未跑RustWS/清理完成/性能试验；不能证明server停止/退款或用户外部tasks退出 | add假WS/屏障/假clock/两namespace共享clients/重入/最后owner；updatepool owner和dispose接线；retain显式SSE及冻结call |
| V-30／036／D-23/24/35；EP-05/06、EPT-05、UX-14 | SDK/产品Codex transport/continuation/fallback；省略与四显式值、无session/no-cache、prefix/state变更、missing ID、limit、EOF/JSON/cancel | 省略默认Auto与显式Auto先归一effective，WS选择及delta规则相同；Sse绕过WS，Websocket完整上下文，WebsocketCached/Auto仅精确旧full input+实际response items前缀且非input相等才delta，否则完整input。no-cache/sessionless仍temporary WS，不续缓存。store:false保留；仅成功terminal+responseID发布。missing continuation最多一次完整恢复，connection-limit开始前最多一次重连；普通transport只有首个有效provider事件前可降SSE，显式WS也不假定fail-only。Start后错误保partial单终态，abort/API/auth/protocol不fallback，cleanup cutoff后零fallback，账号A不污染B；EOF非success | pi省略transport时未统一delta eligibility是实现偶然，目标明确修正。WS/SSE语义及所有前缀矩阵未执行，性能无结论；不是照抄共享全局map | add SDK默认/四模式/grammar/delta/异常与transport差分；update旧产品独占auto/SSE-only；retaintyped终态 |
| V-31／AIR-037；D-19/23；EP-07/08、TP-03/05/07 | **SDK active 子边界：** transcript→adapter tool change 能力；**host-impact：** 产品 tool_search/loadout/resume | SDK 按真实 capability 编译 native additions/removals/redefinitions 或默认 collapse；保留 canonical/声明顺序与 projection loss，不能让声明授权执行。此子边界由 AIR-008/011 和 V-06/16/49 共同计 SDK。原 AIR-037 的 CLI 注册/启用 tool_search、发现未 active 工具、下一请求 loadout、resume 迟到注册及 session 关闭策略仅保留影响，不计 SDK 完成；完整 MCP/codemode 不在本轮 | Pi SDK inline tools 有静态源码；产品可选 tool_search 路径不证明 SDK native parity；二者均未运行 | add SDK tool-change wire corpus；update 必要 callers；retain 授权；产品新测试 none |
| V-32／038／D-23；EP-09/10、EPT-08/09 | 各adapter＋canonical persistence＋target replay；Responses/Codex/Azure、Google/Vertex、Chat reasoning_details、Bedrock与Anthropic | capture→terminal→serialize/reload→下一request保留opaque bytes/顺序/块绑定/来源；signed-empty和terminal-only回填不丢。跨provider/API/model剥离不能重用的签名/opaque，保可见内容与raw；能力变化不盲回stale thinking。invalid metadata有typed失败/明确投影，不伪造unknown来源 跨身份所有thinking含可读部分drop；新effort stamp按同身份重放，不能把旧模型reasoning暴露成answer。 | Anthropic已有部分proof，其余源码和fixture仅读，真实远端签名接受性未验证。tag后修复作为额外case，不混称1.0.0已行为 | add每族roundtrip/跨目标矩阵；update持久化格式；retainraw/脱敏 |
| V-33／AIR-039；D-24/28；EP-11/12、EPT-10/11 | **host-impact，仅追踪：** Reference Product recovery owner；partial transient、持久 omission、auth identity、tool 回合 | 产品默认 enabled/3额外/2s base/60s cap、run/turn/recovery events、失败历史/usage、omission ACK 后新 call、同源账号约束及不重复 tool 仅为原目标影响，不计 SDK 完成。独立 SDK 默认不自动恢复；公开显式 whole-call retry helper 必须由 V-57 另行实现和验收，不能用产品 wrapper 代替，也不要求 session | Pi 产品路径及测试仅读未跑；旧 Opi 输出后全停的策略是否变更由该产品另行决定，不将其作为新 SDK oracle | retain 原失败/授权回归；未来 host add/update 独立；SDK helper add 见 V-57；本轮 none |
| V-34／AIR-040；D-20/23/29；R4、strict fallback fixture | **SDK active：** Anthropic/Responses/Codex constrained sampling；prefer/require、指定三类 400、grammar/custom incremental | private wire schema optional→required nullable/递归转换不改 trusted schema，返回 null 按原 schema normalize。Anthropic 仅匹配 grammar 大小/strict tool 数量/union 限制的 400 invalid_request_error、public Start 前可创建一次降级变体：只移除 prefer strict，require 保留；其他 400、callback 错误、取消和 Start 后失败均不分支。auth/model/source 不变，每变体重建＋onPayload＋post-validation；普通 retry 只复用变体，max_retries=N 每变体 ≤1+N，总 ≤2×(1+N)，advanced 不重置，N=0 也可两次 send。记录一次降级诊断及理由。Responses/Codex lark 优先 regex；不支持 grammar 目标退普通 JSON 并记 loss，支持但无合法 variant/schema 在 dispatch 前失败；call_id/item_id/result 关联、raw 文本封装、追加增量及 closed-input 失败可观察 | Pi 新 strict fixture 仅静态读取；未执行。旧全 call transform 一次/全 call 1+N 不再是 oracle；本地 schema 不证明 grammar 等价或服务接受 | add 三类 positive/相邻 negative/require-only/两次 callback/计数/取消/grammar corpus；update strict-only 与 once-per-call 断言；retain 本地校验 |
| V-35／AIR-041；D-20/22/28；TP-08/12–16 | **SDK active 子边界：** plain-JSON normalize/validate 公共 utility；optional-null/primitive/union/nested/root；**host-impact：** hooks/final authorizer/execution | SDK raw 不变，normalized 通过原 schema；不填 default/偷删 extra、不依 TypeBox 私有标记；数值/string 转换固定 JS corpus，不以 Rust parse/Debug 猜测，返回 raw/normalized/validation failures 可辨，失败不授予执行权。此子边界由 AIR-024/049 与 V-15/58 计 SDK。原 AIR-041 的可信 hook mutation、阶段观察、最终授权绑定实际 execution/revision、deny/cancel 零执行仅下游影响，不要求本轮交付 最终malformed/nonobject工具参数保留arguments_error/raw text，schema coercion前拒绝；省略/空白合法{}、非法\u修复保留后续字段；partial展示不授予执行。 | Pi plain-JSON/hook 测试静态证据；Rust 新 normalizer 未运行，现 hook seam 也不是 SDK utility proof | add SDK 完整 corpus/失败；update utility 与必要 caller；retain 原 deny/cancel/secret 回归；host 新测试 none |
| V-36／AIR-042；D-17/27/28；SP-14、R4 | **SDK active：** Models/direct adapter trusted onPayload/onResponse→post-transform validation；普通 HTTP retry、strict 变体、callback failure/cancel；**host-impact：** durable projection ACK | 每个实际 payload 变体只调用一次 onPayload；max_retries=N 按变体计，普通 call ≤1+N、唯一 strict 分支后 ≤2×(1+N)，N=0 仍可两次 send；advanced 不可重置 counter/variant 上限。**普通 HTTP retry 冻结该变体并复用结果；strict 降级最多创建一个新变体，重建后再次 onPayload 并重新预算/JSON/身份校验**。同 call auth/model/provider/account 与已合并 headers 不变，strict 分支只重跑 onPayload、不重跑 transform_headers；改身份/endpoint/protected auth 失败且零后续 dispatch，非身份合法值保留。onResponse 只读，成功获取最终 response 后按支持协议一次；strict 被拒的初始响应不冒充成功 callback。transform/onResponse 抛错 typed 失败，开始后不降级/不重放；dispatch 后失败保留 partial/usage/已发生请求事实。动态 headers/raw observer 见 V-51。durable caller 的首次 send 前 ACK 仅 V-18 下游影响，裸 SDK 不建存储 | Pi 新 strict fixture 对 onPayload 两次/onResponse 一次仅静态读；公开 callbacks/保护/计数尚未执行。不能再用“全 call transform 一次”拒绝已选变体语义 | add public callback counts/失败/权限/每变体验证；update 旧 once-per-call 断言/必要 callers；retain binding/脱敏；host 新测试 none |
| V-37／043／D-23；EP-13、EPT-12 | opi-ai public deferred dispatcher＋faux式MockProvider；submit、pending→pending→ready/error、fetch/cancel、已带expiry且过期handle | Pending(handle)与正常最终结果可区分，route/provider/model/API/token+可选hint/data可序列化且无secret。wait=0一次check不隐式poll，鉴权按operation解析；不支持/错route/无效或已明确过期handle为typed失败，不当普通生成/换provider。无expiry不捏造远端寿命。fetch本地abort只停本次等待，remote cancel须显式best-effort | 已发布public＋faux＋durable消费者成立；仅内建remote adapter未证实。远端expiry含义/重复ready/cancel保证不能从faux推断；本地fixture只证明已给定契约 | add public生命周期/auth/unsupported/错route/显式expiry；retain普通stream terminal边界 |
| V-38／044／D-23；EP-14、EPT-13 | 可重开的最小deferred consumer fixture；保存handle/model/poll时间→重开→fetch、再次pending、cancel、model消失 | 重开不重submit，只按checkpoint fetch；无hint时caller fixture用5s默认，pending下次时间严格推进且无busy-loop，ready才commit assistant；选定model消失明确失败不切route。本地scheduler关闭，显式best-effort remote cancel失败可报告且不伪装ready；已aborted token不能让声称尝试的cancel根本未调用 | pi durable真实poll/cancel/reopen源码与测试仅阅读。5s仅属已确认fixture，不建立SDK全局poll。此验收不新增opi-durable、远端adapter或完整持久任务框架；真实远端协议缺口保持 | add temp-store/fake-clock重开/cancelfixture；retainscope与远端证明分界 |

## 完整 AIR→V 覆盖、scope 与 state

67 个 AIR 均保留独立身份。`SDK` 为本包未来验收义务；`SDK/必要 caller` 只包含保持新公共 API 的当前消费者可用及原有安全保证所需适配，不扩大为产品功能交付。`host-impact` 的旧条款保留历史因果和影响；同一 V 的 SDK 子边界由其他 active AIR 承担。全部 state 是候选设计状态，不是完成结果。下表的 V 辅关联不改变该 V 自己的 scope；例如 AIR-007 提到 V-33，只表示下游关系。

| AIR | 本轮合同/追踪面 | scope | 验收 V | state |
|---|---|---|---|---|
| AIR-001 | 唯一绑定/compat | SDK | V-01/V-11 | active 设计；未实施/未执行 |
| AIR-002 | 原子目录/冻结 | SDK | V-01/V-27 | active 设计；未实施/未执行 |
| AIR-003 | 吸收终态/EOF | SDK | V-03/V-05/V-40 | active 设计；未实施/未执行 |
| AIR-004 | 取消交付 | SDK | V-04/V-05 | active 设计；未实施/未执行 |
| AIR-005 | 有界 producer/资源 | SDK | V-04/V-05/V-29 | active 设计；未实施/未执行 |
| AIR-006 | 无损分帧 | SDK | V-02 | active 设计；未实施/未执行 |
| AIR-007 | 冻结 auth/分层 retry | SDK | V-05/V-12/V-39/V-57 | active 设计；未实施/未执行 |
| AIR-008 | chronology/声明变更 | SDK | V-06/V-16/V-31 | active 设计；未实施/未执行 |
| AIR-009 | Agent 可信执行 revision | host-impact（不计 SDK） | V-06/V-15/V-16 | 保留影响；未实施/未执行 |
| AIR-010 | 来源/replay | SDK | V-07/V-32 | active 设计；未实施/未执行 |
| AIR-011 | native-or-collapse/Strict | SDK | V-07/V-16/V-31 | active 设计；未实施/未执行 |
| AIR-012 | 工具配对/缺项投影 | SDK | V-07/V-15 | active 设计；未实施/未执行 |
| AIR-013 | 失败/Length 投影 | SDK | V-07/V-15 | active 设计；未实施/未执行 |
| AIR-014 | 工具结果图片 | SDK | V-08 | active 设计；未实施/未执行 |
| AIR-015 | session v3 | host-impact（不计 SDK） | V-17 | 保留影响；未实施/未执行 |
| AIR-016 | 旧 session 迁移 | host-impact（不计 SDK） | V-17 | 保留影响；未实施/未执行 |
| AIR-017 | 持久 ACK/事务 | host-impact（不计 SDK） | V-18 | 保留影响；未实施/未执行 |
| AIR-018 | compaction checkpoint | host-impact（不计 SDK） | V-19 | 保留影响；未实施/未执行 |
| AIR-019 | 分支/导出/证据 | host-impact（不计 SDK） | V-17/V-19 | 保留影响；未实施/未执行 |
| AIR-020 | 单一 normalize/必要 callers | SDK/必要 caller | V-06/V-20 | active 设计；未实施/未执行 |
| AIR-021 | 旧公共壳退役/迁移 | SDK/必要 caller | V-10/V-20 | active 设计；未实施/未执行 |
| AIR-022 | compat 行为/旧键 | SDK/必要 caller | V-11/V-20 | active 设计；未实施/未执行 |
| AIR-023 | 独立 AWS resolver | SDK | V-09/V-12/V-20 | active 设计；未实施/未执行 |
| AIR-024 | SDK 参数验证/errors/usage | SDK/必要 caller | V-12/V-13/V-14/V-15/V-35 | active 设计；未实施/未执行 |
| AIR-025 | typed operation 身份 | SDK | V-21/V-28 | active 设计；未实施/未执行 |
| AIR-026 | operation lifetime/retry | SDK | V-22/V-36/V-50 | active 设计；未实施/未执行 |
| AIR-027 | OpenRouter image | SDK | V-23/V-25/V-50 | active 设计；未实施/未执行 |
| AIR-028 | 四 classifier 协议与基线语义 | SDK | V-24/V-50 | active 设计；未实施/未执行 |
| AIR-029 | 答案 domain 验证 | SDK | V-24/V-50 | active 设计；未实施/未执行 |
| AIR-030 | typed failure/known usage | SDK | V-13/V-25/V-50 | active 设计；未实施/未执行 |
| AIR-031 | 有限预算/reader | SDK | V-23/V-24/V-25/V-36/V-54 | active 设计；未实施/未执行 |
| AIR-032 | 真实 adapter/public SDK | SDK | V-21/V-22/V-26/V-50 | active 设计；未实施/未执行 |
| AIR-033 | catalog publisher/store | SDK | V-27/V-56 | active 设计；未实施/未执行 |
| AIR-034 | SDK 显式 refresh/产品政策拆分 | SDK/必要 caller | V-28/V-39/V-56 | active 设计；未实施/未执行 |
| AIR-035 | 实例资源/async cleanup | SDK | V-29/V-42 | active 设计；未实施/未执行 |
| AIR-036 | Codex WS/continuation | SDK | V-30/V-39/V-42 | active 设计；未实施/未执行 |
| AIR-037 | 产品 tool_search/loadout | host-impact（不计 SDK） | V-31/V-16 | 保留影响；未实施/未执行 |
| AIR-038 | opaque thinking 全族 | SDK | V-32/V-07 | active 设计；未实施/未执行 |
| AIR-039 | 产品默认 recovery | host-impact（不计 SDK） | V-33/V-05/V-12 | 保留影响；未实施/未执行 |
| AIR-040 | strict/grammar/有界降级 | SDK | V-34/V-05/V-36 | active 设计；未实施/未执行 |
| AIR-041 | Agent hook/最终执行授权 | host-impact（不计 SDK） | V-35/V-15 | 保留影响；未实施/未执行 |
| AIR-042 | 受限 payload/response callback | SDK/必要 caller | V-36/V-13/V-51 | active 设计；未实施/未执行 |
| AIR-043 | deferred dispatcher | SDK | V-37 | active 设计；未实施/未执行 |
| AIR-044 | 可重开离线 consumer fixture | SDK | V-38/V-37 | active 设计；未实施/未执行 |
| AIR-045 | Models 四聊天入口 | SDK | V-39/V-42/V-01 | active 设计；未实施/未执行 |
| AIR-046 | 构造/管理/auth/store | SDK | V-39/V-42/V-43/V-09/V-12/V-27/V-28/V-55/V-56 | active 设计；未实施/未执行 |
| AIR-047 | 统一 Result/result collector | SDK | V-40/V-03/V-05 | active 设计；未实施/未执行 |
| AIR-048 | 不可变事件/终态内容 | SDK | V-41/V-07/V-32 | active 设计；未实施/未执行 |
| AIR-049 | Context/validate/manual loop | SDK | V-39/V-41/V-07/V-15/V-35/V-36/V-58 | active 设计；未实施/未执行 |
| AIR-050 | 独立 consumer/package/callers | SDK/必要 caller | V-39/V-40/V-41/V-42/V-20/V-26/V-58 | active 设计；未实施/未执行 |
| AIR-051 | 九类 OAuth 与 rotation | SDK | V-43/V-12/V-39/V-55 | active 设计；未实施/未执行 |
| AIR-052 | builtin ID/Azure 双 wire | SDK/必要 caller | V-44/V-01/V-20/V-42/V-49 | active 设计；未实施/未执行 |
| AIR-053 | 3.5 estimator/simple/native | SDK/必要 caller | V-45/V-39/V-01/V-54 | active 设计；未实施/未执行 |
| AIR-054 | 显式 telemetry 传播 | SDK | V-46/V-39/V-42/V-58 | active 设计；未实施/未执行 |
| AIR-055 | parent/Dispatch/poll/drop | SDK | V-47/V-42/V-58 | active 设计；未实施/未执行 |
| AIR-056 | 被动安全观测/evidence owner | SDK/必要 caller | V-48/V-13 | active 设计；未实施/未执行 |
| AIR-057 | 全 public/protocol/profile | SDK | V-49 | active 设计；未实施/未执行 |
| AIR-058 | 四 classifier＋images | SDK | V-50 | active 设计；未实施/未执行 |
| AIR-059 | headers/raw observer/callbacks | SDK | V-51 | active 设计；未实施/未执行 |
| AIR-060 | compact接口已由Pi1.3.0移除 | retired（不计 SDK） | V-52 retired；V-41保留 | 退役身份；不复用／不算通过 |
| AIR-061 | duration/lazy timing | SDK | V-53 | active 设计；未实施/未执行 |
| AIR-062 | sampling/pricing/input limits | SDK | V-54 | active 设计；未实施/未执行 |
| AIR-063 | 九 OAuth/store transaction | SDK | V-55 | active 设计；未实施/未执行 |
| AIR-064 | catalog/personalization/version | SDK | V-56 | active 设计；未实施/未执行 |
| AIR-065 | 显式 whole-call retry helper | SDK | V-57 | active 设计；未实施/未执行 |
| AIR-066 | 三 Context/package/custom APIs/utilities/platform | SDK | V-58 | active 设计；未实施/未执行 |
| AIR-067 | 版本追踪/test evidence | SDK | V-59 | active 设计；未实施/未执行 |

AIR-009/015–019/037/039/041 共 9 个 host-only；其编号不回收。除AIR-060退役外，其余57个AIR计active SDK。V-17/18/19/33 共 4 个纯 host-impact；V-52退役，其余54个V的明确SDK子边界组成active验收集合。此 scope 修订不追改历史 D/Q 或原测试结果。

## SDK 使用方补强验收（V-39–42）

| ID／决策／证据 | Owner／触发输入 | 结果与失败的机械判据 | 既有证明及缺口 | 未来测试影响 |
|---|---|---|---|---|
| V-39／D-30/31/35/36；UX-01/02/05/06/08/09/11–16、SC-01–10、SDK-01–08 | 外部公共SDK consumer；Models/builtins/选模、完整custom record、四入口、显式key、neutral/native、工具循环、管理方法 | 普通chat无需产品factory/Agent/prepare；单pipeline、native/simple各自mapping、auth一次。custom record含provider_id+bare id且不被同名catalog替换，错误owner/wire/options preauth零HTTP。默认空Models/内存store，显式builtin含惰性env来源；注入可替代ambient，typed key优先、stored失败不转env。构造/注册/静态查询env/auth/store/network零且无runtime。check_auth Option配置/expired OAuth不刷新；available chat/type/all Vec metadata过滤、unknown为空，真实store/check失败Err；get_auth选定来源可刷新。refresh报告catalog/混合结果，login返回ack Credential，logout本地删后env仍可configured。假store/presenter独立完成宣称flow；四入口与原生max/simple预算/Codex Auto/adapter retry默认经真实wire fixture验证。工具只proposal，caller验证/授权/执行/追加，Length不执行、Deferred pending，换模型按投影 | 当前manifest无internal deps/trait/prepared存在不证明目标；pi例子及新管理/默认分支只静态读。生成无需先管理调用，但确会在获准auth阶段I/O；不是构造零I/O延伸为全管理pure | add SDK caller/假env-store日志/管理返回与failure/默认transport-retry/normalization；update公开示例/消费者；retain来源与无secret |
| V-40／D-31；UX-03、SC-04/05、AIR-003 | 同fixture三种收集：complete、从未next直接result、部分/完整next后result；事件数量>64，setup/partial failure/cancel/EOF/各stop reason | 最终Result和canonical message/typedcause完全对应；失败只有一次Err后永久EOF。result主动推进不死锁，不多dispatch/resolve、无self-cancel；next已取terminal后result返回缓存。Stop/Length/ToolUse/Deferred是Ok但Ok不授予执行许可：Length/Error/Aborted不执行，Deferred pending，其余完整提议按校验/宿主授权；Error/Aborted是Err，Cancelled为Aborted；setup empty/unknown，partial/knownusage/来源保留；producer/socket/attempt释放分别验证。通过标准Stream和result相同seam | 现drain/CompletedRequest与生命周期测试证明旧路径，不证明统一error/owned collector；不以bounded队列或只有最终字符串断言代替结果一致 | add >64/三路径/缓存/失败等价；update所有Error/CompletedRequest断言；delete旧wrapper存在型测试；retain错误内容/usage/cleanup |
| V-41／D-30/31；UX-04/05/08、SC-06–09 | 公共事件/Context：交错多个text/thinking/tool blocks、partial JSON、signed-empty/terminal-only metadata、grammar、serde后跨模型/manual tool loop | Start先于update/Done；setup可直接Err，start后失败Err，block index/次序不变，end是权威内容。早期owned partial不会因后续生产变化；delta与end、done/result保留合法差异，不假设partialJSON已最终合法。完整terminal含stop/deferred/provenance/usage/签名；serde自有格式往返这些字段和chronology，无TS JSON字节承诺。pi差分在事件产生时采样live partial，仅归一化已有允许字段；匹配End而非文本相同 Pi1.3文本finalization不得截成可执行prefix，arguments_error经end/terminal/serde往返；同chunk reasoning先于content，retired frames不作此组前提。 | opi事件enum/serde和pi注释存在；未运行完整重建/签名/grammar语料。本轮不重新跑pi；future pair fixtures/真实adapter假传输必要 | add full-event/immutable partial/serde/tool-loop corpus；update公共类型/错误映射fixtures；retainopaque/replay顺序 |
| V-42／D-30/31/35；UX-01–15、SDK独立审查 | workspace外隔离consumer/离线package smoke；两Models同ID不同依赖、明确shared clients/stores、owned call后替换context/provider、spawn/drop/cleanup | opi-ai是唯一Opi runtime依赖，仅列出普通Tokio/Stream/serde deps，不靠monorepo dev-deps/私有资源。公开exports/资源离线编译运行；ChatStream和completion Future Send+'static可进host Tokio，Stream不需Sync。Call创建owned冻结、无mutable借用，未poll drop零来源网络；Models drop/remove/replace后原call可完成，新call用新revision。实例目录/auth/cancel/cache隔离，共享只按namespace/真实owner；公开Some/None cleanup达到V-29完成屏障，不清另一Models/外部owner，最终guard Drop不自建executor/无限后台存活。无全局注册/env探测/login/背景目录。旧并列ProviderCollection、CompletedRequest、当前Error双轨退出，Agent/product/summary/evidence与外部Provider实现迁移；历史reader仅合法旧格式 | 静态无internal deps不是publish package/外部consumer证明；SDK完整OAuth目前未实施，不能以trait充验收。依赖/feature精确版本审查，未缓存依赖报gap而非自动安装；不声称任意executor/WASM/未测性能 | add consumer/package/Send-lifetime/两instance/共享lease/cleanup/未poll/SDKauth组装；update exports/crate双语docs/生产consumer；delete旧并列壳测试；retainadvanced prepare/历史read |

## 已确认 SDK 协议、身份与预算（V-43–45）

| ID／决策／证据 | Owner／触发输入 | 结果与失败的机械判据 | 既有证明及缺口 | 未来测试影响 |
|---|---|---|---|---|
| V-43／AIR-051；D-32 Q33；OA-01–06、R3 | **SDK active：** Anthropic/Codex legacy/Copilot 旧三 flow→store/presenter；fake endpoints/ports/clock、并发/ack/cancel | 三旧flow为九flow子集，准确callback/ports/PKCE/state/metadata/expiry保持；等锁/结果caller可取消，已开始refresh网络15s与ACK分开，10s网络＋6s ACK不因新总budget失败。store-owned/两Models共享/logout/replace/barrier/latewrite按R-02；没有默认wait/keycount/ADC总时限。旧OA/30s等历史不成为oracle。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add SDK flow/ports/metadata/expiry/rotation races；update 旧 form/callback/cancel 与必要 product caller；retain keychain/locking/denial |
| V-44／AIR-052；D-33 Q34；BP-01–04、R2 | **SDK active：** builtin/get_model/native/simple；provider/API/deployment、旧引用迁移冲突/ack/reentry；历史 replay | canonical google/google-vertex/amazon-bedrock；openai 默认 Responses、同 provider 显式 Chat。**Azure provider 恒为 azure，同 provider 注册 azure-openai-responses 与 openai-completions 两 wire**，不能将 provider 改为 wire ID；Responses 默认 POST /openai/v1/responses?api-version=v1，api-key 与 body.model=deployment，无 /deployments path；Chat 显式 profile 及共享 deployment-name/override 规则各自 wire fixture。provider_id 与 bare catalog id/deployment 分开，历史 provider/API bytes 不洗掉。必要 config/catalog/credential caller 迁移先验证冲突、copy/ack/reference/finalization 可重入，确认前保 source，不覆盖/合并账号；不永久留旧 default aliases，user custom IDs/历史 reader 保留 Kimi新builtin为C+/coding/v1；旧override URL显式迁移、历史A保真；OpenCode Go注册G，generator新endpoint/Minimax路由单列缺hydrateddata。 | 当前 Azure Chat/provider azure 已有 proof，不是 provider rename 缺口；新 Responses/wire 迁移未运行。fake HTTP 不证 live 订阅或服务接受 | add 双 wire public 组装/deployment/迁移冲突失败；update 必要 factory/schema/config/docs/callers；delete 退役 default alias 测试；retain explicit Chat/custom/history |
| V-45／AIR-053；D-34 Q35；estimate/simple-options；R5 | **SDK active：** estimator、simple/native/advanced；UTF-16/images/sections/tools/rawargs、usage anchors、max/thinking；**host-impact：** compaction/UI | 无 anchor 按 message 合计 UTF-16 units 后 **ceil(chars/3.5)**；350→100、3500→1000，4800 image chars 及 system render/tool 数组各自粒度按固定 corpus。正 reported total 优先，否则仅完整已知正 parent 和；unknown 不补 0、不双加 child、total≠sum 保留；有效成功 anchor＋trailing，不限同 model、不 max 全文、prefix 时间戳失效/Error/Aborted 不 anchor。simple 精确 M/C/F/B′、4096/min1、显式 max 不再 min modelcap；common 1024/2048/8192/16384 家族/极小 ceiling 的 Claude zero按Anthropic truthy fallback1024/Bedrock `??`区分、Chat zero省略，不新增BudgetInsufficient；Google/Vertex 独立表，Pi Messages simple/native 仅转发六个白名单字段，省略max/cache保省略（仅显式long环境覆盖），显式max原值，无通用clamp/default；metadata/sampling/thinkingBudgets/deferred不进envelope。native 不套 simple clamp，逐 wire omission/default/gating/floor；两入口只在 effective 输入等价时比较。sampling/tiers/input limits 见 V-54。必要 API consumer 复用 estimator，产品 compaction/UI 重新设计不计 gate | 当前旧 /4/same-model/lowerbound 是实现事实；新 corpus 仅读未跑，不证明 tokenizer/账单/远端生成。Rust 精度/unknown 差异保留 | add 3.5/UTF16/anchor/每 mode/边界 corpus；update 旧 /4/max/same-model 断言及必要 caller；retain knownness/非账单 |


## Telemetry 补强验收（V-46–48）

以下是 Q37/D-37 的未来验收设计，只验证已选 context 接入和 owner 保留；不要求新增自动 SDK call/attempt spans，也不把 pi 的旧 schema 或未合并 proposal 当 oracle。

| ID／决策／证据 | Owner／触发输入 | 结果与失败的机械判据 | 既有证明及缺口 | 未来测试影响 |
|---|---|---|---|---|
| V-46／D-37；TC-01/07–09、TE-04/07/08、TO-03/08/09/16 | 外部 SDK caller→Models→真实 adapter+fake transport／MockProvider；native/simple stream/complete/result、custom model、advanced prepare、图片、分类、deferred submit/fetch/cancel | 每入口自定义 adapter 可观察到同一 frozen caller parent/来源 Dispatch，simple 转换与低 retry 不丢失/重选；四聊天 collector 结果仍一致。无 context／显式 noop 不自动产生 SDK spans、upload、env/auth/store/network；业务真实 call 按原授权正常执行。没有已批准的内置 call/attempt schema。serde、wire、session/checkpoint/model/credential/catalog/deferred handle 不含 context/backend object/trace-ID；auth/management 不自动扩新 options，caller 可外包 future | pi identity 测试未运行且缺 classifier专项；当前 opi 无此字段。仅 type/re-export存在不证明 adapter接线，不能因捕获 metadata断言就宣称 builtin emitted spans | add public options传播/序列化排除/外部 caller；update request/options/examples/自定义provider implementers；retain native/simple/typed operation behavior |
| V-47／D-37；TC-02–06/11、TO-10–12/17–20、tracing0.1.44 | Context/helper/producer／宿主 recording subscribers A/B；A下capture、B下首次poll/pending后迁移线程、spawn、Stream poll_next、drop；disabled parent/显式noop | from_span 保存 A 的匹配Span+Dispatch；in_scope同步body一次、返回恢复B；instrument保留lazy和output，未poll不执行body，poll/drop各仅短scope、不多poll、不建额外span。custom adapter在context helper内所发marker/child归A，B没有误归marker；独立同ID Models仍隔离。noop helpers在禁用dispatcher下仍运行业务一次，其marker不误接ambient A/B，结束恢复host。Future/Stream/Drop分别证明，不能拿Future combinator替Stream proof。native Span close可被host clone延后，但原Result/terminal/资源cleanup不等待它；不宣称generic callback schema/status/inert-after-settlement等价 | locked tracing只支持Future instrumentation，WithDispatch无通用Drop守卫；仅静态读。memory test subscriber是测试支持，不证明任何production exporter性能/相同merge逻辑。host自己拥有raw native parent的业务结束 | add cross-Dispatch/multithread/poll/drop/noop/result conformance；retain 原lifecycle/队列/drop/EOF；none自动schema或新production buffer tests |
| V-48／D-37；TC-02/05/10、TO-04–07/12–15/19/20、CTRL-001/003 | **SDK active：** SDK domain/custom adapter＋conforming trusted host recorder；**host-impact：** 既有 DiagnosticSink/AgentEvents/EvidenceSink/Session；recording/export失败、disabled/sampled、typed失败/cancel、canaries | no-op/合规recorder/可失败内部exporter下原Result/partial/usage/事件顺序/auth-send-retry次数/工具授权相同；record失败不重跑callback/poll、不触发业务重试，不等flush。模型返回失败由host domain显式标错，不伪称SDK会自动检测。telemetry关或丢失不吞功能observer/hook失败、原诊断/EvidenceSink failure/health/withheld manifest/ack，不能以span ID顶替durable ID。default fields不含raw prompt/completion/tool/payload/headers/credential/free error/URL query canary；未知usage不填0；库无global-init/backend网络/线程/清空外部subscriber。文档显式限制任意panic/block/abort/OOM不受保证；不以catch_unwind包业务掩盖错误 | 当前diagnostic/evidence/redaction测试有真实边界但本轮未运行，不证明tracing parent或任意backend故障隔离。pi memory对记录异常的防护不等于强制任意adapter合规；两生态信任边界声明而非假称语言必然 | add passive/safe-fields/host-ownership隔离；update SDK 接线与必要 caller；产品 verbose 新接线仅下游影响（不自动启用 subscriber）；retain 所有diagnostic、功能事件、evidence/RPC trace及ack tests；delete none |

## Pi 1.3.0 完整公开能力补齐（V-49–59）

下列 11 项追加覆盖旧候选排除或遗漏的本包公开能力；与旧项交叉引用以共享同一 oracle，不能双算为两份完成证明。协议/profile、公开 utility 的来源及 Rust 目标动作细节见 [SDK 契约附件](pi-1.3.0-sdk-contracts.md)；语言、安全、平台差异见 [Rust 差异附件](pi-1.3.0-rust-differences.md)。表中每项 scope 均为 active SDK、state 均为设计未实施/未执行。

| ID／AIR／证据 | 输入与公开边界 | 机械结果与失败 | 证明限制 | 未来 test impact |
|---|---|---|---|---|
| V-49／AIR-057；KnownApi、providers/all、R1/R2/R9 | 十个 chat API 的 direct/native/simple/complete/result；42 个实际 builtin factories、完整 custom record、同 provider 多 wire；各协议文本/tool/thinking/图片/失败 fixtures | 下方十协议及42 profile表每项经真实 adapter＋loopback，不仅注册/列名；provider/API/catalog ID/deployment 分开。检查 auth/baseURL/header/compat/effort、默认和显式 route，以及每协议成功、畸形/EOF、拒绝/cancel。未知或未注册 API typed pre-auth 失败，已注册 custom API 按 V-58 可用；不以复用 transport 免除 profile 差异、不静默缩为旧六 wire。Azure 为 azure 双 wire；Mistral 名称为 mistral-conversations，实际 /v1/chat/completions；Pi Messages 需实际流/投影/rewrite-impact，以及省略max/cache、显式max和六字段白名单，禁止误发metadata/sampling/thinkingBudgets/deferred；Bedrock显式placeholder proxy/H1与零真实来源读取由V-09/58同验，不以普通 Chat 替代 Anthropic/BedrockClaude error images在全turn toolresults后，caption/连续placeholder/图像次序与非Claude/成功负例；五finalization位点和shared Azure/Codex继承不能假称各有新增测试。 | 固定源码明确10/42；生成 model data 缺失，不能证明全 catalog 记录、全模型名称可离线恢复，不能把 fake HTTP 说成 vendor 接受。所有新 conformance 未运行 | add 十协议/42 profile 参数化公共 fixtures 与 negative cases；update 旧 exact wire/provider-count guards；retain 现有效 wire/secret/cancel |
| V-50／AIR-058；KnownImageApi/ClassifierApi、R6 | generate_images/classify→真实 adapter；openrouter-images＋四 classifier；state/questions/images、operation-qualified 同 ID、API-key/OAuth availability | 下方逐协议表的 payload、endpoint、bool 变换、答案/usage/errors/retry 各自成立；classifier 仅 model.input 含 image 才接受，unsupported image 在 provider 调用前失败（零 auth/dispatch）；Decisions 不能套 System One noul。TypeSafe/OpenRouter/OpenCode 同 System One 协议分别验证 bindings；Cloudflare 包装、llama-cpp 和 Decisions 独立。input/semantic invalid 不 retry；取消/timeout/drop 与失败 usage 保留共用 V-22/25；image 仍是一个非流式 Chat-wire，无 URL 自动抓取 | Pi 分类/images 测试仅读；没有四协议运行证明，不能以两 bindings 充两算法或假定所有 classifier 支持所有题型/图片；没有专用 OpenRouter Image API 或 image streaming 的交付证据，不虚构 parity 要求 | add 四协议及图像 accept/reject/availability/count fixtures；update 旧 System-One-only 覆盖与枚举；retain 图像 serde/无下载/knownness |
| V-51／AIR-059；ProviderRequestOptions/ModelsRequestTransforms、R4 | Models→header assembly→transform_headers；onPayload/onResponse/raw provider-event；native/simple/direct/image/classifier/deferred 支持路径及 callback 抛错 | 静态 header 按大小写合并、null 删除非保护默认；Pi 为 resolved auth→model.headers→per-call→transform，Provider.headers 元数据本身不自动并入。Rust 按 profile/static→resolved auth→per-call→transform 一次，保护 identity/auth/host 等变更，合法字段实际到 wire；此保护及显式 profile 默认为记录过的安全差异。Bedrock 自定义 header 在 SigV4 前纳入签名，reserved auth/x-amz/host 不可绕过保护。每 payload variant 一次 onPayload，普通 HTTP retry 不重跑；唯一 strict 降级第二 variant 重跑且重验。HTTP onResponse 按 adapter 固定：支持的 chat 在 body 消费前（Mistral 在 HTTP success 检查前），image/classifier 在成功完整 JSON 之后，错误 HTTP 不概括为一定回调；Google 两 API 无伪造 headers callback，Codex WS 无伪造 HTTP response。十 chat 的 raw parsed events 在 normalize 前依序只读，unsupported 操作明确无此 callback；callback failure typed 终止，保留 dispatch/partial，不变为 retry/fallback/成功。telemetry recorder 不吞功能 callback 失败；canary 仅显式可信 raw 回调可见，不进默认诊断 | 静态 callbacks 支持不证明公开接线；raw 默认日志与只读可信观察不同。保护身份是明确 Rust 差异，不能借保护取消整个回调能力；全部计数/失败未运行 | add header precedence/null/保护/signing、raw 顺序与支持矩阵、callback count/failure/cancel；update 旧 callbacks 排除；retain canaries/有效 header |
| V-52／AIR-060；Pi1.3.0 breaking removal | retired：不再新增compact encoder/reducer | AssistantMessageFrame/encoder/reducer及对应Pi测试均删除；ordinary事件／不可变partial／final Result仍由V-41/40证明。无新持久化格式或仅为历史parity的公开API | 退役不是测试通过；旧1.2.0行为仅由Git历史恢复 | delete未来compact专属实施要求；retain普通事件／Result设计；本轮测试none |
| V-53／AIR-061；event-stream duration、R7 | direct adapter/native/simple/complete/result、成功/partial error/abort、未 poll、fake monotonic clock、已有 duration/deferred fetch | adapter 响应流**首次 poll 到终态**计时，未 poll 无计时/任务；不包含外层 Models auth/loading，adapter 前 setup failure duration absent。complete/result/迭代/转发保留同一 duration，不重复补时或覆盖已存在值；较早 deferred ready 结果不加本次 fetch 等待。0 可表达，单调不负值，序列化保可选 duration；tool result 时长为 caller 提供 metadata，SDK 不执行工具或捏造时长 | 明示 Rust lazy 与 JS eager 计时差异；fake clock 证明边界，不证明真实性能/时钟精度；尚无新测试 | add public lazy/direct/fake-clock/已有值/deferred/collector 一致；update 公共 message serde；retain unknown/0 区别 |
| V-54／AIR-062；sampling resolver/pricing schema/estimate/input limits、R5/R7 | 模型 defaults、effective effort override、request sampling；catalog pricing tiers＋usage、image/media/token 输入限制；native/simple | sampling 按 model defaults→effective-level map→request keys 逐键覆盖，只给真实支持的 OpenAI-compatible APIs 应用；simple 先 resolve effort/native 不偷 clamp，named 与 arbitrary keys 优先级可观察。calculateCost/helper消费adapter的最高命中 **strict > threshold** tier 对整次请求收费（实际matchedThreshold起始-1，仅threshold>-1入选、同threshold保留首项；负阈值不新增准入拒绝），OpenRouter image parseUsage保留基价、不取tiers；同model基价input=1、tier input=2且超过阈值时image用1/helper用2，阈值基数 input+cacheRead+cacheWrite、等于/超过、缺/0/invalid/溢出均测；unknown price/usage 不补可靠0，负 placeholder 按显式 Rust unknown 差异且无负费用。Responses serviceTier按response优先/request后备；flex0.5、priority/fast为2（model ID恰gpt-5.5为2.5）、其余1；Codex HTTP/WS的fast为1，response default保留request flex/priority，其他沿response优先；Azure无倍率。倍率乘已知四成本bucket后合计，unknown保留；fixtures含default/省略/覆盖/精确模型ID，不能仅测发送tier。3.5 estimator 与 V-45 单一算法；inputLimits（maxRequestBytes、images maxPerMessage/maxPerRequest、resize maxWidth/maxHeight/maxBytes/jpegQuality）按原schema和Serde保留，Pi SDK没有该统一runtime执行点，不新增默认pre-dispatch cap或resize；选择该政策的宿主/显式caller才验执行。model modality真实拒绝保留，不套四MIME/container假硬限制 Gemini4+源码regex、Xiaomi off/high toggle和Codex500000为generator/changed-test事实；无hydrated目录不得报catalog通过。 | 现 Rust 已有 tiers，不称从零新增；字段存在不能证明 cost/catalog 接线。估算、成本和限额不是 tokenizer、账单或性能证明；未运行 | add merge/effort/tier ingestion+算法接线/limits；update /4 与旧 price placeholder 预期；retain u64精度/knownness/现 tiers |
| V-55／AIR-063；OAuth loaders/AuthContext/store、R3 | 九 flow 的 SDK login/get_auth/refresh/logout，fake presenter/endpoints/ports/clock；两个 Models 共享 store、refresh 后取消 waiter、logout/replace race | 九flow由公开login/get_auth/refresh/logout→fake endpoints/presenter→同store ACK→adapter可达。已开始refresh仅网络受来源15s signal，store lock/ACK不包该timer；caller停等不丢新token、不恢复generation。无默认barrier wait/key-count/ADC总预算；显式等待deadline返回pending不abort。按key保留unknown、跨Models/crossprocess/fence/denial/expiry/Runtime与已知效果分别证明；check/available不refresh。R-02包含10s+6s正例和真实网络超时负例。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add 九 flow/SDK-only/store-owned transaction/ack/rotation/logout/replace；update 旧三flow-only、固定 callback、cancel network 预期；retain PKCE/state/denial/keychain 安全 |
| V-56／AIR-064；catalog helpers/protocol、R8/followup | ModelsStore read/write/delete、refresh baseURL、显式 personalize_openrouter_catalog、Radius dynamic 空列表、schema/version fixtures、stale publish | configured endpoint 下 /models/user 仅 chat 按 key 更新限制/价格，image/classifier 不变；无 key/401/403/404 保原 catalog，取消不晚 publish。裸 OpenRouter factory 不隐式个性化；宿主默认 wrapper 是下游政策。Radius 首次已知 dynamic（含空）替代 baseline，不让被禁模型复活；通用 overlay 不强套。store snapshots owned、operation/type 过滤、provider generation commit/reopen 不陈旧；显式 source invalidation 可持久撤下无效 cache，即使没有 bundled generation，也不能重开后复活旧值。此机制不默认启用产品 HTTP404/501 触发政策。distribution schema v1、AI hydration v6、revision index/minimumPiVersion 及 UA 显式兼容版本分开；Rust crate version 不冒充 Pi version，unsupported/malformed/unknown-required fail closed、可兼容 revision 选择由固定本地语料证明 | model-data 目录缺失阻止全 bundled records proof；本地协议代码不证明 pi.dev 部署/字节一致。复用远端服务与具体兼容声明需独立证据；不默认联网。全部新 tests 未运行 | add configured endpoint/chat-only/key isolation/empty authority/store reopen/version negative corpus；update 把版本协商全写未来或 factory 自动 helper 的预期；retain 旧有效 snapshot |
| V-57／AIR-065；retryAssistantCall/retry classifier | 公开显式 retry_assistant_call(owned produce, optional policy, cancel, callbacks)；transient/busy/upstream/stalled、quota/billing/overflow/abort、partial usage | 无 policy/disabled/0 只调用 produce 一次；enabled 至多1+max_retries 个**新 logical calls**，单一指数 delay/base/cap，不串接旧流、不执行工具、不主动切换 provider/model/account、不隐式 session；任意 produce 闭包的目标由 caller 负责，若使用 Models 受控 call factory 则执行冻结目标/来源身份检查。配额/billing/overflow/abort/auth 与配置错误不重试；typed 新分类覆盖 Pi1.3 transient/stalled及gateway/temporarily-unavailable，不靠泄漏原 body 匹配。scheduled 在 sleep 前、attempt-start 在 sleep 后且新 call 前；finished 有重试时只一次，有效 Ok 的 Stop/Length/ToolUse/Deferred 对应 true，Error/Aborted 为 false；这不授予工具执行或 remote ready 许可。取消 delay 为 Err/Aborted 保留最近 partial/usage，零后续 produce/timer。callback 失败 typed 结束，不重跑 callback/业务、不吞；与低层 acquisition/strict variant/WS budget 独立可计数，返回 failure 不虚构完整重试账单 新增Unable to forward upstream stream与The service is temporarily unavailable匹配，保留quota/overflow/auth/abort排除和默认次数不变。 | 显式 SDK helper 与 V-33 产品默认恢复不同；source 仅读。fake-clock/producer 不证明真实网络；同源重新解析的受控 factory 按 V-12，凭据实变无法确认身份须 typed fail-closed；新实现未运行 | add SDK standalone helper/classifier/delay/callback/cancel/partial/auth corpus；update 旧“SDK完全不提供whole-call retry”排除；retain 默认不自动恢复 |
| V-58／AIR-066；root/subpath exports/package、三 Context、custom API | workspace 外 consumer；下方 public动作索引、三 Context、custom chat/image/classifier/deferred APIs、direct/lazy adapters、faux/test support、utility/CLI/platform 替代 | 每公开动作有Rust动作或明确差异/gap；workspace外consumer证明三Context、四调用入口、custom兼容options跨Models/upsert/oldcall、native typed failures、callbacks/telemetry/platform替代。R-01–08受来源审计约束，不能靠新增cap/unknownformat/stale拒绝或“Unsupported”关覆盖；backend配置不可解除的限制单列。包资源离线、构造零I/O、Tokio/Send条件明确；尚无新SDK公开consumer的编译/运行证明，独立研究机制不替代它。 当前后端OPEN表各gap须有source expected/native结果与真实public-boundary proof；Schema argumentsError前置拒绝、去WS可配置cap均不能关闭engine/handshake/compression差异。 | 本轮为源码/合同修订；新的公开consumer/本地conformance未实施或执行，历史测试不证明本目标。 | add external consumer/package/custom API/三Context/utilities/platform negative smoke；update exports/examples/必要 callers；delete 仅旧壳存在测试；retain 真行为 |
| V-59／AIR-067；version tracking/followup evidence | 新固定 Pi 版本/来源摘要、Opi HEAD/变更、public capability delta、test inventory/实际运行证据；缺 generated data/live 授权 | 后续每次对照在 existing upstream-tracking 和本矩阵更新 source/version/tree、前驱、公开动作/Context/telemetry/RFC accepted-vs-delivered、Rust差异、test add/update/delete/retain 与实际命令/case数/skip/gap；不修改旧70/probe基线，不将静态文件数当pass。Pi参考只运行审过的纯fixture范围，隔离HOME/env/defaultauth/非本机网络；不因有key启live、不自动hydrate/安装。测试证据逐 Pi路径/语义输入→Rust binary/filter→差异→actual outcome，缺runner/job明确未交付；无第二ledger/脚本或自动监控 本地1.2→1.3 fixedtree/pathdelta与test处置；catalog最低Pi1.2.1、发布hydrate/build:offline和内部exactpin检查仅作source工作流事实。 | 本轮只修订文档，无差分 runner/CI job。历史70只证原输入；171为1.2.0来源文件数（1.3.0为174）、41+7为Rust文件数，均非新目标执行数。版本跟踪不保证未来提案落地 | retain 全历史输入/41+7迁移；未来 add 受控fixtures/runner/CI由实施准入决定；本轮 none |

### 十个 chat API 的公共行为覆盖

来源为 [KnownApi](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)与各 [api 模块](../../../.repo/pi-1.3.0/packages/ai/src/index.ts)。每行均必须通过 V-49 的公共入口＋真实 adapter/伪 transport；native/simple 映射、四 collector、取消/EOF/secret 共同 gate 不因复用而豁免。每行状态均为未实施/未运行。

| API | 必须触发的协议差异与机械断言 | 关联 V |
|---|---|---|
| openai-completions | Chat path/role/store/bridge；tool_calls 关联、reasoning_details、usage、字节分块；兼容 provider defaults 和 arbitrary sampling 优先级到 wire | V-01/02/07/08/11/32/45/49/51/54 |
| mistral-conversations | 实际 POST /v1/chat/completions；自有 payload/thinking/tool/cache 与 HTTP headers timeout；body stream/EOF/cancel 不能套 image body deadline | V-02/04/07/32/45/49/51 |
| openai-responses | response/items/call_id 双 ID、encrypted reasoning、terminal metadata、grammar/custom tools、max omission/gating、raw events | V-07/32/34/41/45/49/51 |
| azure-openai-responses | provider azure、api-key、deployment 与 catalog ID分开、/openai/v1/responses、api-version/endpoint override、body.model、native floor | V-01/44/45/49/51 |
| openai-codex-responses | account headers、auto/四显式transport、WS full/delta、continuation/fallback、resource stats/cleanup；失败后不隐式播内容 | V-05/29/30/32/34/45/49/51 |
| anthropic-messages | inline-tools-2026-09-15、by-value tool_addition/同名重定义、signed/redacted/empty、工具图像、strict 两变体+callback次数 | V-06/07/08/16/32/34/36/45/49/51 |
| bedrock-converse-stream | AWS event-stream/CRC、Bearer或SigV4链、family effort/cache、reasoning签名/bytes、tool图像、standard retry与headers signing | V-02/05/08/09/32/45/49/51 |
| google-generative-ai | contents/functionResponse/tool parts、thinking level/budget、thoughtSignature、image工具桥接；raw chunks、无伪造onResponse | V-07/08/32/45/49/51 |
| google-vertex | 与Google shared wire规则＋project/location/baseURL/API-key/ADC或service-account 来源优先级，typed配置失败；不改写历史ApiKind | V-01/07/08/09/32/45/49/51 |
| pi-messages | Radius gateway endpoint、chronology原生输入、Pi事件→owned partial/Result、rewrite impact/diagnostics、六字段白名单、simple透传/省略max与cache/显式long、headers/raw callbacks和错误/取消；不以Chat序列化替代 | V-06/16/41/49/51/56 |

### 一个 image 与四个 classifier API 的公共行为覆盖

来源为 [KnownImageApi / KnownClassifierApi](../../../.repo/pi-1.3.0/packages/ai/src/types.ts)。下列均由 V-50 主验收；typed错误/usage/预算/资源的共同断言保留 V-22/25。每个 adapter 的 input支持/题型限制按模型元数据和真实协议分别测，不能一律照 System One。

| API | 输入/公开边界与必须观察的结果/失败 | 关联 V |
|---|---|---|
| openrouter-images | 有序text/image与首choice；保留Pi text-only/空image/0choice成功、跳过非匹配URI、fake-png可观察encoded结果；不增四MIME/signature/图数/bytes门槛；无下载。0额外retry、600000ms非流式body/parse remaining attempt scope有精确OpenAI来源 | V-22/23/25/50 |
| typesafe-system-one | object state＋ordered choice/score/bool，bool→noul，任何非空images均拒绝（custom metadata声称支持也不能绕过）；TypeSafe/OpenRouter/OpenCode binding endpoint/auth分别；请求ID齐全、额外ID忽略、真实required/type/finite与provider confidence（无额外domain拒绝）、不加sum/argmax业务拒绝 | V-24/25/50 |
| cloudflare-workers-ai-system-one | Cloudflare account/gateway endpoint及header包装、System One语义；非空images在请求前拒绝，API返回结构/error映射和retry不借其他binding proof | V-24/25/50/51 |
| llama-cpp-classify | 本地 /tokenize→/apply-template→/completion 独立算法：label单token/冲突与cache、n_probs升级、logprob/temperature/softmax及confidence；非空images拒绝，key可省略；payload/response只观察completion；不假造System One响应或已知usage | V-25/50/54 |
| openai-decisions | /decisions、bool→predicate、state+images的具体payload/答案；API-key-only availability，OAuth不列可用；504不重试，其他准入类别/计数独立验证 | V-21/22/25/50/55 |

### 42 个 builtin factory/profile 的逐项验收输入

完整枚举源为 [builtinProviders](../../../.repo/pi-1.3.0/packages/ai/src/providers/all.ts#L145)。这是42份profile组装义务，不是42种transport。**每行**通过对应实际 factory 构造零I/O、inject假env/key/store、get_model/availability、合法完整custom record、真实adapter→loopback：断言源文件声明的ID、有效baseURL、auth/header、默认compat/effort与已注册API；wrong-source/unsupported route零dispatch。下表列每个profile需加入共同corpus的差异；全部归 V-49/39/42，状态未运行。缺生成数据不能用捏造的完整builtin记录掩盖，完整catalog证明按 V-56/59 明示gap。

| factory / provider ID | 额外必须触发的 profile 差异 |
|---|---|
| amazonBedrockProvider / amazon-bedrock | bedrock-converse-stream；Bearer/profile/default-chain选择、region/endpoint与签名、standard retries |
| antLingProvider / ant-ling | openai-completions；其独立endpoint/env key及compat默认，不能用OpenAI默认代替 |
| anthropicProvider / anthropic | anthropic-messages；API-key/OAuth headers/betas、strict及inline tools |
| azureProvider / azure | 同provider的azure-openai-responses＋openai-completions；deployment/endpoint/version双wire |
| basetenProvider / baseten | openai-completions；独立endpoint/key/header/compat |
| cerebrasProvider / cerebras | openai-completions；独立endpoint/key、effort/compat |
| cloudflareAIGatewayProvider / cloudflare-ai-gateway | anthropic/openai-completions/responses三API常驻，不因catalog暂缺某API拒绝完整custom record；account/gateway包装 |
| cloudflareWorkersAIProvider / cloudflare-workers-ai | completions＋Cloudflare System One classifier；account/token/endpoint包装 |
| deepseekProvider / deepseek | completions；reasoning/thinking与工具兼容profile |
| fireworksProvider / fireworks | anthropic＋completions；/inference基址、同provider按API分发 |
| githubCopilotProvider / github-copilot | anthropic/completions/responses；OAuth endpoint/availableModelIds metadata过滤、API-key独立 |
| googleProvider / google | google-generative-ai；API-key/endpoint/Google thinking映射 |
| googleVertexProvider / google-vertex | google-vertex；API-key/ADC/service account、project/location及来源失败 |
| groqProvider / groq | completions；独立endpoint/key/compat与参数 |
| huggingfaceProvider / huggingface | completions；路由endpoint/key/模型ID与compat |
| kimiCodingProvider / kimi-coding | completions；API-key/OAuth、/coding/v1；K3类inline tool-change与Highspeed区别，旧override带/v1显式迁移 |
| metaProvider / meta | responses；API-key/OAuth、原生model profile |
| minimaxProvider / minimax | anthropic；国际endpoint/key与profile |
| minimaxCnProvider / minimax-cn | anthropic；中国endpoint/key，不能与国际来源拼接 |
| mistralProvider / mistral | mistral-conversations；自有API与headers timeout |
| moonshotaiProvider / moonshotai | completions；国际endpoint/key/effort |
| moonshotaiCnProvider / moonshotai-cn | completions；中国endpoint/key/effort隔离 |
| nvidiaProvider / nvidia | completions；独立NIM endpoint/key、typed transient分类 |
| openaiProvider / openai | responses默认、completions显式、Decisions classifier；新ChatGPT OAuth与API-key-only classifier过滤 |
| openaiCodexProvider / openai-codex | codex-responses；legacy OAuth/account、Auto/WS/SSE资源 |
| opencodeProvider / opencode | anthropic/google/completions/responses＋System One；inference各wire suffix、classifier固定inference/v1；session header、key、subscription/quota不可retry |
| opencodeGoProvider / opencode-go | anthropic/google/completions/responses；inference/go各wire suffix，MiniMax按源provider映射；共享key名但独立ID/catalog/session |
| openrouterProvider / openrouter | anthropic/completions＋image＋System One；OAuth/key，显式个性化不自动联网 |
| qwenTokenPlanProvider / qwen-token-plan | completions；独立plan endpoint/key/compat |
| qwenTokenPlanCnProvider / qwen-token-plan-cn | completions；CN plan endpoint/key/compat |
| qwenTokenPlanIndividualProvider / qwen-token-plan-individual | completions；individual plan metadata/endpoint |
| radiusProvider / radius（可自定义id/name/gateway） | pi-messages；动态空catalog权威、configured gateway、key/OAuth身份隔离 |
| togetherProvider / together | completions；独立endpoint/key/compat |
| typesafeProvider / typesafe | classifier-only；System One，chat查询不混classifier |
| vercelAIGatewayProvider / vercel-ai-gateway | anthropic；gateway endpoint/headers/auth与model profile |
| xaiProvider / xai | responses；API-key/OAuth与模型限制 |
| xiaomiProvider / xiaomi | completions；独立endpoint/key/compat |
| xiaomiTokenPlanAmsProvider / xiaomi-token-plan-ams | completions；AMS endpoint/plan隔离 |
| xiaomiTokenPlanCnProvider / xiaomi-token-plan-cn | completions；CN endpoint/plan隔离 |
| xiaomiTokenPlanSgpProvider / xiaomi-token-plan-sgp | completions；SGP endpoint/plan隔离 |
| zaiProvider / zai | completions；国际endpoint/key/effort |
| zaiCodingCnProvider / zai-coding-cn | completions；CN coding endpoint/key/effort |

### 九个 OAuth flow 的逐项验收输入

完整枚举源为 [OAuth loaders](../../../.repo/pi-1.3.0/packages/ai/src/auth/oauth/load.ts#L16)。每行均经 SDK login/get_auth/refresh、注入 presenter/store 与假 HTTP/loopback；不只构造 ResolvedAuth。每个 flow 保存真实 credential/metadata并验证 error/denied/expired/state/PKCE/回调失败等实际适用边界，返回 ACK 前不报成功；共同 rotation/secret/事务由 V-55。全部未实施/未运行，不代表拥有真实客户端注册或供应商接受证明。

| flow | 必须进入语料的特定动作与元数据 |
|---|---|
| Anthropic | JSON token请求、state/PKCE、53692→空闲loopback→手贴、expiry/refresh及OAuth wire headers |
| OpenAI Codex legacy | callback/device两路径、bind失败手贴、严格refresh_token/expires_in、account ID/endpoint |
| OpenAI ChatGPT | dynamic_agent_client 注册、PKCE/state、callback携issued client_id、1455路径/手贴URL校验、agentName、direct scope/refresh和真实expiry；与Codex legacy分开、Decisions API-key-only过滤 |
| GitHub Copilot | device denied/expired、enterprise domain/proxy endpoint、availableModelIds；登录policy与refresh不启用policy |
| OpenRouter | 自有PKCE/API-key兑换/metadata；无refresh能力按真实flow返回而不造token、个性化显式helper |
| Kimi Coding | 自有device/授权路径、poll间隔/过期/拒绝、credential与provider headers/metadata |
| Meta | RFC8628 device→identity token→mint Model API key；refresh为同identity重mint而非refresh_token grant，mint401/403需重新登录，不伪造identity可续期 |
| xAI | device-code/poll、HTTPS verification URL、denial/expiry；refresh可省新refresh_token时保旧值，metadata/expiry按协议及显式安全窗口差异 |
| Radius | id/name/gateway绑定、browser PKCE callback与device两登录方式、gateway discovery/refresh/credential/动态model metadata；不同gateway/account不混store |

### 其余 public 动作与导出覆盖

这张索引把导出对应到实际调用/失败语料，名称对照不等于语义通过。公开可消费的 `utils/*` 子路径也必须有合同映射；Rust可采用现有安全类型/模块，不能因不是root re-export便静默遗漏。每项 proof 均按具体 V 的限制，未实施/未运行。

| 公开动作/导出面 | 需要越过的边界与机械输入/结果 | V |
|---|---|---|
| createModels/builtinModels/builtinProviders、set/get/delete/clear provider | 空/替换/删除/清空、旧owned call继续、新lookup变化、构造零I/O、双实例；完整custom model不被catalog二次替换 | V-01/20/39/42/49/58 |
| getModel(s)、OfType、All、builtin typed getters/data generation | 同provider同id跨operation、unknown为空、chat-only、不支持type过滤、metadata完整；缺data显式gap | V-21/39/49/56/58 |
| checkAuth/getAvailable/OfType/All/getAuth/login/logout/refresh | Option/Vec/typed management failures/RefreshReport、store失败不吞空、expired check不refresh、source失败关闭、logout本地、refresh(false)不联网 | V-09/12/27/28/39/43/55/56 |
| native/simple stream/complete、direct/lazyStream/lazyApi、custom createProvider | 四入口相同collector、lazy setup failure、typed exact-options mismatch、自定义API字符串可注册并实际调度，无需builtin catalog/Agent/session | V-01/03/39/40/42/49/58 |
| generateImages/classify、deferred submit/fetch/cancel | typed操作及各协议；pending handle roundtrip、wait0、unsupported/expiry错route、显式remote cancel与本地abort分开、重开consumer | V-21–26/37/38/50 |
| Context/TranscriptContext、normalize/initial/current system/tools、collapse/resolve、declaration/state changes | shorthand一次折入、section顺序/删除/同名by-value、equal不增delta、Strict/collapse、serde不绕不变量、不更改canonical | V-06/07/16/31/41/58 |
| tool schema/grammar/plain-JSON validate、StringEnum/TypeBox替代 | JSON schema构造/enum与常规Rust schema机制；validateToolCall/Arguments纯机制、optional-null/转换失败；无执行授权或TypeBox私有依赖 | V-15/34/35/58 |
| event stream/createAssistantMessageEventStream | bounded producer、push/finish/Result三消费路径、不可变partial、异步取消/EOF/terminal；compact encoder/reducer已退役，不再形成public动作 | V-02–05/40/41/53 |
| calculateCost/estimate*/calculateContextTokens、thinking support/clamp、modelsAreEqual/hasApi/getModelType/assert* | 3.5/usageknownness/tiers/effort与sampling；typed identity/equality及错误operation/input限制、不仅测字段存在 | V-01/14/21/45/54/58 |
| retryAssistantCall/retryDelayMs/isRetryableAssistantError、provider retry、overflow/RecoverableLength/patterns | 明确整call与acquisition层、quota优先、delay cap、cancel释放；overflow与Length判断不自动执行/恢复 | V-05/30/40/57/58 |
| JSON repair/stream parsing、text/system render、unicode/hash/uuid/header/env/abort/sleep helpers | malformed/partial/合法边界、可见文本顺序、无损Unicode、稳定非秘密ID/哈希、headers case/null、注入env优先、combined cancel及listener/timer释放；错误采用typed脱敏差异 | V-02/06/07/13/41/51/58 |
| diagnostics/error formatting、faux/scripted/tool agent fixtures | typed redacted分类/附加诊断、不泄漏raw body；脚本化多block/tool/deferred可供外部SDK消费，Mock不当HTTP/live proof | V-03/13/37/41/48/58 |
| ModelsStore/CredentialStore/AuthContext/API-key/AWS/OAuth helpers | owned read/write/delete/list/事务、惰性env/file source、explicit优先、shared store并发/ACK、store failure不回退 | V-09/12/27/39/55/56/58 |
| session resource cleanup/Codex debug stats | 精确scope/namespace/异步本地quiescence、connecting/temporary/busy统计、无秘密；不复制module-global registry或声称server退款 | V-29/30/42/58 |
| TelemetryContext/options/helpers、三Context分工 | 全操作显式parent+Dispatch、cross-task/poll/drop/noop、serde/wire排除、被动性与域owner安全边界；不新增自动span/exporter | V-46/47/48/58 |
| root/models/providers/api/utils/oauth/bedrock-provider/bun-oauth/compat exports、catalog资源、pi-ai list/login | Rust modules/re-exports与可运行独立SDK示例保留list/login动作，legacyglobal入口不复制全局；Bunloader/Node callback/proxy/浏览器桥接由明确platform/presenter/runtime替代；package离线资源和不支持平台typed错误可检查 | V-10/20/42/55/58/59 |

## 差分语料、归一化与有意差异

共同语料使用人工构造、无真实秘密的请求及provider响应fixtures，固定pi源码版本。对齐的是已选择等价的可观察语义：文本/媒体块顺序、tool call/result关联、canonical最终内容、usage已知性、事件顺序和终态。相同供应商协议才比较wire payload；Rust/TS对象布局、所有权及私有helper不要求相同。

可归一化的只有不参与语义的clock、随机request ID；tool/response ID若参与引用/续接只能做保留双射及引用关系的统一替换。pi共享partial在事件产生时采样，不能事后读取已累积对象充当当时事件。

不能归一化：终止缺失/重复、cancel与success、是否重试/降级、usage缺失与零、UTF-8损坏、参数coercion、system/tools时间位置、section顺序、缺result的UnknownEffect与NotExecuted区别、签名归属、WS前缀/重复输出。不能只比较最终字符串或最终tool集合掩盖中间契约。

按 D 与 [Rust 差异附件](pi-1.3.0-rust-differences.md)记录的差异要作为双向有意 case：Rust有界背压/owned状态、typed脱敏错误和Q32 Result（failure保留完整message）；Rust owned collector/惰性poll明确；AWS选定不完整来源失败关闭；高层新prepared call重解析但账号实变终止；原始历史与有损request projection分离；trusted payload transform受身份约束；plain-JSON不模拟TypeBox私有路径。native-or-collapse是已确认默认，Strict为显式选择。compaction 的 D-28 有损重建仅保留 host impact，不进入 SDK 完成条件。

pi已知缺陷/实现偶然不作oracle（含D-35/36修正）：pending result不完成、usage未报置0、原样错误泄漏、旧generation晚到发布、WS全局资源残留、Codex省略/显式auto的delta不一致、HTTP400/401宽catch重播、Bedrock忽略retry override、不可取消SDK sleep、取消误报retry成功。每个修正都需局部反例/明确目标，不用“Rust不同”笼统豁免。score distribution/强制argmax等被撤回的研究者要求也不再作为oracle。

差分不能证明真实模型服从、缓存命中、远端签名接受、server已停止计算/退款、账单、外部consumer兼容、性能或跨进程事务。存储事务/资源释放用对应故障注入证明，remote事实另查；live只有另获授权才执行。

## 已执行验证与未来设计分界

此前成功运行的固定命令是：

```text
cargo test --offline --locked -p opi-ai --test gemini_lifecycle --test provider_lifecycle --test provider_collection
```

结果 **4＋8＋58＝70 passed，0 failed**，使用既有外部`E:/opi/cargo-targets/windows-direct`，详见[README基线验证](README.md)。它只覆盖当时已有mock HTTP、EOF、取消、prepared call与目录事务断言；不证明新增catalog并发、WS、high-level恢复、normalization、operations或deferred目标通过。

[流探针](offline-probes.md)与[原始JSON](offline-probe-result.json)实际运行成功，assert的是当前缺口：prepared terminal后可见尾事件、empty入口不一致、Chat UTF-8分块损坏，以及有drop后屏障的Gemini连接在约501ms窗口未见关闭。最后一项不能升级为永久泄漏/精确producer存活结论。[route probe](route-probe.md)实际用纯mock复现目录wire不一致与晚失败；它不证明产品交互已运行。probe exit0是研究观测符合断言，不是目标回归通过；探针源码不作为固化错误行为的正式测试。

未运行：有效V-01–51/53–59的未来add/update/delete（V-52退役）、Pi suite、完整跨 provider 差分、live、性能、全 workspace 测试。本轮不修改测试/运行时代码/依赖；环境有凭据不构成live授权，未来只运行已审查mock-only范围。

历史测试影响：**retain**（此前 70 个现有离线测试在原基线运行）。本轮测试影响：**none**（仓库测试 add/update/delete 均 0，未重跑历史测试）。研究探针独立保留，不注册为生产tests。文档检查与候选双语/ID检查由主Agent最终联合运行并报告；本页不把未来完成条件写成当前成功。

## 完成判据与剩余事实缺口

本轮**文档候选完成**须满足：67 个 AIR 的身份/scope/state/责任与 V 映射闭合；59 个 V 都有公开边界、输入、机械结果/失败、证明限制和 test impact；54 个 active SDK、4个 host-impact与1个retired 不混算；十 chat、一个 image、四 classifier、42 factories、九 OAuth 及全部导出动作有可追踪的等价行为或明确 Rust 差异。双语候选、两份新附件与本页不得在 Azure、3.5、rotation、strict 变体/预算/callback 时序上冲突。41＋7 文件处置、历史 70tests/probes 原始输入与版本限制完整保留。

未来**实施验收完成**须经本包公共边界、真实 adapter/fake transport、workspace 外 consumer/package 及适用平台证明，并完成破坏性 API 必需的当前 callers 适配。退役行为先有替代 proof 再静态退出扫描；SDK 函数存在、测试文件数、文档断言都不等于通过。产品 hook/授权/session/compaction/tool_search/默认恢复/启动刷新/UI 只保留影响，除机械 API 适配及现有安全回归外不要求本轮新交付。全部新运行记录真实命令、binary/filter、执行数/结果/skip；不拿历史70或probe追认。文档修订不是实施准入。

已确认的包内目标、Rust Result/owned/lazy/有界资源/安全来源及 telemetry 责任不重新列作待用户选择。当前待评审的是修订候选本身；本页没有新增必答产品问题。实际 SDK conformance、依赖/平台机制选择、生成 model data 缺失及真实 remote deferred 协议仍是**证据缺口**，不能由实现者猜测或把缺能力关为“Rust差异”。若未来选择直接复用 Pi 分发服务，客户端身份/兼容版本声明和部署合同须另有一手证据；live/性能/服务账户与预算另需明确授权，均不阻塞本次文档闭合，也不默认启动。

## 2026-10-16 测试迁移补核

本节保留两轮审查的静态逐文件处置输入，依据用户“仅 opi-ai / Pi 1.2.0 公开能力和可观察语义”的目标；本轮只同步其与 V-01–59 的关系，不追改历史执行结果。来源、调用体验、规划和当前缺口见 [五问补核](pi-1.2.0-followup.md)，完整清单见 [follow-up evidence](pi-1.2.0-followup-evidence.json)。此前历史结果不改写。

原1.2.0窗口的 inventory 是 **41 个 Rust integration 文件＋7 个含 cfg(test) 的 src 文件，Pi AI 171 个 .test.ts 文件**；1.3.0 Pi文件增量见本页末节，不是执行 case 数。没有运行 Cargo/Node/Pi 测试、读取真实凭据或修改测试；本轮实际 test impact 为 **none**。下表全部是未来在契约核准和实现变更后的处置，不授权现在删除或改写测试。

处置按断言而非文件名：retain 保留有价值行为，update 仅替换与新契约冲突的预期；delete 必须有退役依据和替代 proof，不能把 API 改名、阶段名或上游删测试作为整文件删除理由。原测试准确锁定旧实现不等于新合同 oracle。表中行号来自本次静态源码，代表输入/断言不是逐条 assertion 全审计；未来运行必须记录实际 binary/filter、执行数及结果。

### 现有 integration 文件的完整处置映射


路径均相对于 `crates/opi-ai/tests/`。行号为本次静态读取时位置。`update+retain` 不授权整文件删除；下面只列代表过滤器/触发输入，不声称逐条 assertion 全部复核。

| 文件 | 未来动作 | 代表函数/证据与保留/调整规则 |
|---|---|---|
| [anthropic_fixtures.rs](../../../crates/opi-ai/tests/anthropic_fixtures.rs) | update+retain | :150 text lifecycle、:394 absent/zero/1h subset、:708 CRLF 保留；:953 默认 thinking budget 按 native/simple 分开；:1496 drain cancellation 补实际任务/HTTP释放；新增 all-byte UTF-8、strict prefer/require fallback、inline chronology。 |
| [anthropic_thinking_roundtrip.rs](../../../crates/opi-ai/tests/anthropic_thinking_roundtrip.rs) | retain+update | :71 signed/opaque HTTP→storage→tool continuation、:160 foreign route、:224 byte chunks、:285 empty-signature 关系保留；迁新消息/来源字段，扩 opaque 全族，不能以本文件已覆盖全部 thinking。 |
| [api_mapped_provider.rs](../../../crates/opi-ai/tests/api_mapped_provider.rs) | update+retain | :225 三 wire、:257 auth once、:525/:563 catalog rollback 保留；单一完整绑定贯穿 adapter、完整 custom record、post-auth endpoint默认；:652 固定 custom endpoint 行为须与明确route contract比较，不盲目套 auth baseURL。 |
| [auth_contracts.rs](../../../crates/opi-ai/tests/auth_contracts.rs) | update+retain | :20/:31 nonretryable credential、:51/:66 redaction 保留；:167 object-safe stub 可证明调用但不能证明 SDK login/refresh/logout 真组装；补 store-owned transaction 与 flows。 |
| [azure_openai_fixtures.rs](../../../crates/opi-ai/tests/azure_openai_fixtures.rs) | update+retain | :279 provider ID `azure` **现有即正确，不改成 azure-openai-responses provider**；:285/:465 当前 Chat部署 URL/body留作显式 Chat profile；另补同provider Responses API/deployment body；auth-routing、bodyless errors、:804/:809 cancel保留。 |
| [bedrock_fixtures.rs](../../../crates/opi-ai/tests/bedrock_fixtures.rs) | update+retain | :629/:652 明确断言 `[image: image/jpeg]` 占位，未来改实际图或typed拒绝；:822/:887 固定不解析 reasoningContent 的限制需由新thinking语义替代；CRC :250、Auth canaries :1262、:1598 resolve once、:1634 fresh-call auth保留；family/retry/timeout扩充。 |
| [connection_pooling.rs](../../../crates/opi-ai/tests/connection_pooling.rs) | retain | :44/:100 shared client复用、:115 pool config保留；实例资源归属/async cleanup需要另加测试，不能从client共享断言推导cleanup完成。 |
| [context_budget.rs](../../../crates/opi-ai/tests/context_budget.rs) | update+retain | :63 的跨模型 fallback (:76–77 =2) 与 :114 全文lower-bound (:117=1001)是现行为；新目标需替换。保留 failed/unreported、prefix时间失效、child不双计性质；:90–110 UTF-16/图像/工具输入 corpus迁3.5公式。 |
| [custom_provider_registration.rs](../../../crates/opi-ai/tests/custom_provider_registration.rs) | update+retain | :71 replace、:330 deterministic listing、:404 override precedence保留；:153 invalid limits按Pi实际字段/零值/metadata有效性及明确Rust差异重审，不把现拒绝充作全局新限制；补 operation-qualified身份、remove/clear、typed get/all/available及custom API adapter扩展。 |
| [gemini_fixtures.rs](../../../crates/opi-ai/tests/gemini_fixtures.rs) | update+retain | :129 当前 `gemini` provider ID 随 builtin `google` 身份迁移；wire历史 ApiKind不洗掉。:28/:98 auth body安全、:513 CRLF、:929–939 cancel保留；thinking level/unsigned tool signature/工具图像扩充。 |
| [gemini_lifecycle.rs](../../../crates/opi-ai/tests/gemini_lifecycle.rs) | update+retain | :55 success、:126 auth、:161 rate-limit、:197 EOF保留输入；迁单Result与partial/usage，新增drop/队列/读取释放计数，不能只把stream drain结束当socket cleanup证明。 |
| [image_input.rs](../../../crates/opi-ai/tests/image_input.rs) | retain | :19/:30/:41 URL/base64/bytes、:52/:65/:78 serde和:128 bytes保真保留；inputLimits仅metadata透传/原schema有效性与显式caller policy分测；不补无Pi执行点的默认内容拒绝。 |
| [image_provider_serialization.rs](../../../crates/opi-ai/tests/image_provider_serialization.rs) | update+retain | :117/:198/:283/:342 user图片wire形状保留；工具结果图片、model+wire真实modality、limits metadata/显式policy分开、实际adapter公共preflight另加，不以user image支持证明tool image支持。 |
| [mistral_fixtures.rs](../../../crates/opi-ai/tests/mistral_fixtures.rs) | update+retain | :372 当前共享Chat profile HTTP fixture保留为合法profile；补 Pi原生 `mistral-conversations` public API语义及实际 /v1/chat/completions、headers timeout边界；:441 compat、:505 cancel保留。不要按API名称虚构 /conversations endpoint。 |
| [model_capabilities_migration.rs](../../../crates/opi-ai/tests/model_capabilities_migration.rs) | update; targeted delete | :14/:32/:44/:60 builder字段/迁移形状断言随退役API删或并入外部consumer编译；:85 cache能力元数据断言、:109 registry effective能力保留，并用独立wire行为测试验证接线，不能因文件名含migration整删。 |
| [model_wire_metadata.rs](../../../crates/opi-ai/tests/model_wire_metadata.rs) | update+retain | :38 exact WireApi旧集合扩新published API/custom契约；:73 mismatch负例、:90 strict `>` tiers (:100 200000=base/:101 200001=tier)、:105 invalid pricing按来源字段与明确unknown/typed差异重审，不新增正数metadata准入；补实际catalog+cost接线及input+cache读写阈值。 |
| [oauth_wire_shape.rs](../../../crates/opi-ai/tests/oauth_wire_shape.rs) | update+retain | :128 exact Anthropic beta核新基线；:191 API key/Bearer分离、:281 body canary、:479/:560 revocation matrix、:627 Codex account/header保留；ResolvedAuth直构不证明Sdk OAuth login/refresh。 |
| [openai_chat_fixtures.rs](../../../crates/opi-ai/tests/openai_chat_fixtures.rs) | update+retain; targeted delete | :577 名称有zero但 :587断言 !is_reported，保留unknown与明确zero区别，不能误删；:1015–1052纯字段存在、:1110静态无效reasoning_effort随旧键退役删除并改migration error；有效role/strict/token/header行为保留；补3.5、effort-dependent sampling、byte chunks与typed失败。 |
| [openai_codex_responses.rs](../../../crates/opi-ai/tests/openai_codex_responses.rs) | update+retain | :117 path/body/account、:524 empty account、:615/:650 revoked一次HTTP保留；:219 high-level effort与native validation分开；补WS/fallback/continuation、取消计时器、no post-handoff retry、资源清理。 |
| [openai_compat_lifecycle.rs](../../../crates/opi-ai/tests/openai_compat_lifecycle.rs) | update+retain | :59/:366/:608 等各profile实际HTTP success/error/EOF留住；:235 custom路径和:788 extra headers保留；迁Error Result及完整terminal/cleanup。 |
| [openai_responses_fixtures.rs](../../../crates/opi-ai/tests/openai_responses_fixtures.rs) | update+retain; targeted delete | :34 当前独立 openai-responses provider壳退出，但保留wire/API及历史reader；:374 previous_response_id永远deferred的旧限制不能在新continuation能力中继续当oracle；:908 interleaving、:997 content index、:1097 call_id、usage子集保留。 |
| [openai_responses_lifecycle.rs](../../../crates/opi-ai/tests/openai_responses_lifecycle.rs) | update+retain | :73 success、:138 data-only frames、:251 EOF输入保留；迁统一collector及资源drop proof。 |
| [openrouter_fixtures.rs](../../../crates/opi-ai/tests/openrouter_fixtures.rs) | update+retain | :323 real loopback Chat wire、:395 compat、:458 cancel保留；增显式 personalizeOpenRouterCatalog语义（configured endpoint/key chat-only、401/403/404保留）、动态catalog、placeholder price/tier差异。 |
| [output_content_image.rs](../../../crates/opi-ai/tests/output_content_image.rs) | retain+update | :72/:85/:98 serde、:145 bytes、:191 tool-result image消息往返保留；新message schema机械迁移，不能从serde证明wire支持。 |
| [per_request_auth.rs](../../../crates/opi-ai/tests/per_request_auth.rs) | update+retain | :91 missing credential、:225/:258/:290/:320 supplied auth到wire、:379/:403/:421/:440 wrong scheme零HTTP保留；新完整Models与store优先级/refresh边界扩充。 |
| [phase14_remediation.rs](../../../crates/opi-ai/tests/phase14_remediation.rs) | update+retain | :128 initial args必须在Start后、:329 exact u64费用、:394 refresh继续收集混合结果保留；:39/:227/:297 unsupported level的native与simple语义分开；:261 route catalog materialization迁单一binding。阶段命名不是删除理由。 |
| [provider_collection.rs](../../../crates/opi-ai/tests/provider_collection.rs) | update+retain | :404/:443–444当前 `Ok(CompletedRequest::Error)`改Err(CallFailure)且保留partial；:478 empty stream负例保留。:1556–1614默认modelcap+成功后可再attempt、:2180–2209 Done后任意sequential retry与新CallSealed/低层重试规则冲突，替换结果并保留auth once。:1077 atomic/:1166 rollback、:2090 active-slot、:2245 cancel forbids retry、:2688 no auth fallback保留。新增四入口/管理/lazy/result三路径/>64/冻结/实例与store conformance。 |
| [provider_diagnostics.rs](../../../crates/opi-ai/tests/provider_diagnostics.rs) | update+retain | :198–318 redaction corpus、:369 validation保留；:97 exactly-nine-phase12构造计数不是覆盖证明，随新typed错误更新；:422 `rate-limit/network only retryable`不能全局当所有wire的retry policy，应测selected protocol/status/phase。 |
| [provider_error_classes.rs](../../../crates/opi-ai/tests/provider_error_classes.rs) | update+retain | :94–161 各adapter HTTP500 bodyless分类性质保留；retry是否允许是另层判断，不可把ProviderSide分类一律nonretryable扩大到Bedrock transient。 |
| [provider_lifecycle.rs](../../../crates/opi-ai/tests/provider_lifecycle.rs) | update+retain | :107/:172 full events、:312 no-terminal、:399 HTTP request保留；:356 drain cancel补barrier任务/socket/queue，迁公共Result。 |
| [provider_trait.rs](../../../crates/opi-ai/tests/provider_trait.rs) | update; targeted delete; retain | :222/:272 untrusted error安全、:330 Send、对象安全实现保留；:315只数6个ProviderKind、:96字段存在随退役壳删/替代。:338仅证明DummyProvider可编译，不证明trait绝无带默认实现的complete；Models.complete也不强迫Provider改接口，应由真实custom adapter/consumer编译覆盖。 |
| [proxy_support.rs](../../../crates/opi-ai/tests/proxy_support.rs) | retain | :179–228 serial env tests、:239–263 redaction、:344真实HTTP经proxy保留；不以localhost mock名义绕过proxy/env隔离。 |
| [registry.rs](../../../crates/opi-ai/tests/registry.rs) | update+retain; targeted delete | :12源码字符串扫描不是API compile proof，退役后删或换实际consumer；:83 replace、:102 clears动态snapshot、:203 canonical parser、:248 exact ID保留；补operation identity/removed/cleared/available。 |
| [request_enrichment.rs](../../../crates/opi-ai/tests/request_enrichment.rs) | update+retain | :106 auth头保护、:132控制字符、:219 header到wire、:349 affinity映射保留；:165 timeout需按具体API deadline语义，headers回调/observer/retry payload变化次数新增。纯标量字段存在 :25/:31/:50/:75 可由实际wire/外部consumer覆盖后合并，勿与有效负例一起删。 |
| [retry_backoff.rs](../../../crates/opi-ai/tests/retry_backoff.rs) | update+retain | :102–106 统一3 attempts、:129–136把10s hint截5s旧预期按协议重审；:140 `should_retry(2)`是旧counter语义。保留HTTP-date/invalid/date-past、禁止auth/config/capability重试性质，新增wire默认/override/预算/准入/计时器/cancel/drop/strict downgrade变体。 |
| [stream_events.rs](../../../crates/opi-ai/tests/stream_events.rs) | update+retain | :44/:142 Error事件双通道迁Err(CallFailure)；:74/:87/:95/:113 serde、:155 stop reason、:184 tool关联保留；加event-time immutable partial、duration、terminal-only metadata与result等价。 |
| [test_support_tests.rs](../../../crates/opi-ai/tests/test_support_tests.rs) | update+retain | :180 queued responses顺序、:312 auth scheme、:331 Send+Sync保留；:137/:288 Error helper迁新failure；扩adversarial terminal/tail/EOF/partial但不把mock当HTTP证明。 |
| [tool_result_no_leak.rs](../../../crates/opi-ai/tests/tool_result_no_leak.rs) | retain | :63/:70/:77/:84 四wire private truncated/details不泄漏，迁schema仍须保留canary。 |
| [tool_result_wire.rs](../../../crates/opi-ai/tests/tool_result_wire.rs) | update+retain; targeted delete | :286跨provider文字/error语义保留，并加图像/关联/顺序；**删除 :352 provider_tool_result_error_no_phase12_breadth_guard 的阶段广度token/module冻结**（:357–388禁止catalog/image/oAuth依赖、:401–438冻结module列表），在新scope核准后由正式依赖/边界检查接管，不能让旧阶段guard拒绝批准的能力。 |
| [usage_cost.rs](../../../crates/opi-ai/tests/usage_cost.rs) | update+retain | :53 missing != known0、:74/:83 absent/Some(0)、:152–278 saturating/knownness、:451 1h subset、精度保留；补独立reported total≠sum、tiers实际计费接线和多attempt partial已知性，避免因上游数字类型不同丢Rust更准表达。 |
| [vertex_fixtures.rs](../../../crates/opi-ai/tests/vertex_fixtures.rs) | update+retain | :30当前vertex identity迁google-vertex；:40 URL/project/location、:100 token redact、:487 bodyless、:867–877 cancellation保留；补API-key auth、对应Google wire共享语义/来源限定选项，不加typed内容cap。 |


### 现有 src 单元测试文件的完整处置映射


| src 文件 | 未来动作 | 代表过滤/限制 |
|---|---|---|
| anthropic.rs（cfg(test):1482） | update+retain | :1523 malformed args、:1539 non-object被静默改{}不能直接当新validation目标；按批准canonical/replay契约改typed失败/显式投影。:1626–1805 cache marker/TTL/reverse scan保持有效性质并迁chronology。 |
| endpoint.rs（:16） | retain | :21 joins_without_double_slashes，仍是独立安全path join性质。 |
| openai_responses_shared.rs（:809） | retain+add | :814 split_sse_frame_is_retained_until_the_blank_line_arrives留；add UTF-8每字节、CRLF、无效byte按Pi增量replacement及真实坏JSON/terminal尾帧，单个split frame不证明全部chunk-safe。 |
| stream.rs（:450） | retain+update/add | :461 backpressured_producer_cancels_and_buffered_events_are_discarded、:511 pending consumer cancellation保留；公共result/partial/terminal结构迁移，再补取消与Done交付barrier、drop任务计数。 |
| bedrock/credentials.rs（:749） | update+retain/add | :786 explicit priority、:929/:1003/:1139/:1161/:1183 partial来源fail-closed、:1500 bounded process的现10s/64KiB默认断言按Pi来源/显式policy重审；:1630 real descendant cancellation保留；新的SDK resolver chain/expiry/store共享事务以fake endpoints/source fixtures另补，不拿现local chain声称全AWS conformance。 |
| bedrock/event_stream.rs（:219） | retain+add | :259/273/304/314 framed chunks、:345/364 bounds、:390 CRC、:403 header类型跳过保留；更复杂split/bounds corpus补。 |
| bedrock/sigv4.rs（:155） | retain | :181 signing key、:193/:236 request/session token、:318 header whitespace向量；auth来源重构不等于删签名算法证明；真实AWS接受另属live边界。 |


### Pi 1.2.0 新语料与修订 V 的衔接

下列是已有上游测试的静态输入/断言，不是本次执行通过，也不能用新增文件名或 V 编号称“测试同步完成”。本轮已在 V-49–59 追加完整覆盖，下表保留原关联并指向新增合同；静态来源不转化为执行结果。

| Pi 参考测试 | 需要转化的可观察断言 | 原 V 关联与补充 |
|---|---|---|
| [context-estimate](../../../.repo/pi-1.3.0/packages/ai/test/context-estimate.test.ts#L46) | 3,500字符=1,000估算；usage anchor＋trailing；prefix失效 | update V-45，保留 Rust unknown/UTF-16/message 粒度和 total≠sum |
| [strict fallback](../../../.repo/pi-1.3.0/packages/ai/test/anthropic-strict-tool-fallback.test.ts#L74) | 特定错误至多降级一次、保留require；onPayload两次/onResponse一次；callback错和开始后失败不降级 | update V-05/34/36/51；本轮按 payload variant 重跑 onPayload，普通 HTTP retry 冻结变体，≤2×(1+N) 与 require 保留 |
| [OAuth rotation](../../../.repo/pi-1.3.0/packages/ai/test/models-runtime.test.ts#L801) | caller停止等待，已开始rotation仍保存新token；其他call不重复refresh | update V-43/12/55；store-owned 串行化/ack，caller 停等不终止已开始 rotation |
| [inline tool changes](../../../.repo/pi-1.3.0/packages/ai/test/transcript-tool-changes.test.ts) | by-value同名重定义、固定prefix及新beta | update V-06/07/16/31；只保留属于SDK的chronology/projection，产品tool_search另列 |
| [event-stream](../../../.repo/pi-1.3.0/packages/ai/test/event-stream.test.ts#L121) | duration、已存在值不重计、较早deferred不补；terminal之后输出吸收 | update V-03/40/41/53，add adapter 首次 poll→terminal 的 Rust lazy 计时及 owned partial |
| [classifier models](../../../.repo/pi-1.3.0/packages/ai/test/classifier-models.test.ts) / [Decisions](../../../.repo/pi-1.3.0/packages/ai/test/openai-decisions.test.ts) | 不支持图像零provider调用；operation身份和API-key availability；四种协议各自payload/结果/失败 | add/update V-21–26/50；四协议与images各自payload/结果/失败，不能只测System One |
| [OpenRouter personalization](../../../.repo/pi-1.3.0/packages/ai/test/openrouter-user-models.test.ts#L64) / [cost tiers](../../../.repo/pi-1.3.0/packages/ai/test/model-cost-tiers.test.ts#L15) | configured endpoint、chat-only、key限额/价格与错误保留；tiers ingestion/阈值 | add/update V-01/13/27/28/54/56；显式 helper 与产品 wrapper 分开、unknown 价格差异 |
| [telemetry options](../../../.repo/pi-1.3.0/packages/ai/test/telemetry-options.test.ts#L68) | 同一context透传，simple/native/deferred/images路径 | retain/add V-46–48；补 classifier/complete、cross-Dispatch/poll/drop/序列化/取消，不能从identity测试推完整spans |

### 执行层次与流程同步要求

目前未发现已接通的 Pi/Opi paired runner 或 CI 共同语料；59组V身份中54 active SDK＋4 host-impact仍是设计、1 retired不算通过。Pi默认 `npm test` 包含scripts和workspaces；AI部分文件先读取home auth再按条件skip，环境key可激活live或本地LLM探测。Pi的隔离 `test.sh` 没被所读CI调用。未找到显式dotenv加载证据，不能宣称已证实默认自动读.env。

1. **机制 → verify：** 依实际修改选择 Rust lib filter，使用 fake clock/RNG/barrier。检查既有 Cargo cache，不clean、不新建session target。
2. **公共入口/真实adapter → verify：** 所有新增/修改的test binary/filter实际运行。若本包多处重构，覆盖本包all-targets，不只重跑旧70；分别断言lease、task、队列、HTTP/socket释放。
3. **受控Pi参考与差分 → verify：** 固定source/hash/锁定依赖/生成数据；先审顶层与imports，再选择mock-only文件/入口。隔离HOME/env/默认auth及非本机网络；不运行根全suite、不用npx自动安装或临时hydrate。共同人工fixtures越过两侧公开入口；只归一化非语义时钟和ID，不能抹平usage unknown/zero、cancel、回调/重试次数、chronology、signature归属。
4. **独立SDK/包装 → verify：** workspace外仅依赖opi-ai及正常Rust runtime/serde/Stream依赖；四入口、direct/custom adapter、tool loop、serde、Send/lifetime、两实例和shared-store/cleanup都可实际运行。单个manifest或内部测试不算外部消费proof。
5. **调用方迁移/CI → verify：** 适用workspace fmt/clippy/all-targets/doctest/rustdoc与多OS gate；SDK 改动所需调用方适配及原有安全回归保留，产品授权/session/UI 等新完成义务不进入本轮 SDK gate。新增差分job如何运行和缺数据如何报告需有具体实现，不能只写“已同步”。
6. **live/性能 → verify：** 另获明确授权及独立目标/预算/凭据选择，默认不被机器有key触发。fixture或差分成功不证明服务接受、真实计费、远端停止计算或退款。

按 `Pi测试路径/版本 → 语义输入/预期 → Rust路径与filter → 有意差异 → 实际运行/skip/缺失证据` 做后续映射，归属本矩阵。当前清单/处置是审查证据，不是第二测试执行账本。本轮没有新增测试、执行器、CI job或生产代码。

## Pi 1.2.0 → 1.3.0 测试来源迁移（仅设计）

本节以[逐项差异](pi-1.3.0-upgrade-review.md)的路径清单为输入，41 integration＋7 Rust source-test处置仍保留。当前Pi AI为174个test文件（前驱171；新增4、修改21、删除1、其余149同摘要），telemetry两个test文件同摘要。文件数不是用例数，未执行任何suite；test.sh/包脚本变化也不表示CI已同步实现。

| 处置 | Pi 来源路径／预期 | Rust未来证明／既有V |
|---|---|---|
| add | json-parse.test.ts；完整final object／非法escape／空白与malformed rawerror | normalize/validator utility＋五文本位点／shared继承的adapter fixture；V-35/41/49/58 |
| add | anthropic-error-tool-result-images.test.ts；Bedrock convert修改 | error-turn多results/连续图像/非Claude与成功结果负例；V-07/49 |
| add | openai-responses-mid-convo-effort.test.ts | flag/effort/map/sign-in/空reply/历史stamp/tooloutput/重复marker完整fixture；V-16/32/49/54 |
| add | transform-messages-cross-model-thinking.test.ts | same provider/API不同model与全跨identity、signed-empty/readonlycanonical；V-07/32 |
| update+retain | validation/Responses partial-json、Chat reasoning、Anthropic off、Google level、Xiaomi/模型metadata及retry等21文件逐路径见来源表 | 保留有效断言并补源修复；不沿用旧partial-prefix成功/旧thinking转text/KimiA/旧endpoint假设；V-11/35/41/44/49/54/57 |
| delete（仅设计） | assistant-message-frame.test.ts随public实现移除 | 不新增对应RustframeAPI或专属tests；AIR-060/V-52退役，V-40/41仍retain |
| retain | telemetry两个test文件与AI telemetry-options.test.ts同摘要；外部dependency与现有安全语义 | V-46–48新增Rustparent/Dispatch/poll/drop接受集，不恢复过时Agent schema或默认exporter |

本轮产品测试add/update/delete均0，test impact **none**。未来实际迁移必须逐Pi路径→Rustbinary/filter→允许差异→真实结果；新tests可以先用真实adapter/loopback和手工完整model，不能悄悄hydrate/联网/从HOME读取auth来填空目录。Schema/WS等仍OPEN。

## 后端覆盖缺口专项验证细化（既有V，无新增编号）

[逐项源码／验证方法／可行路径](pi-1.3.0-backend-gap-audit.md)绑定下表的既有验收。本表保留前置源码轮的未来判据与当时未执行状态，不覆盖下节的实际研究观察，也不建立第二验收索引或运行状态；七组全部OPEN。源码推导case须先运行固定Pi参考取得真实expected，再跑真实Rust公开consumer，不把文档写成通过。Schema／WS的产品未实施、完整V未验收，与已执行研究harness分开。

| 既有验收 | 专项补充判据与关闭边界 | 未来测试处置／本轮结果 |
|---|---|---|
| Schema · V-11/34/35/39/41/58 | 比较Compile/Check/Errors、native build/validate/is_valid/iter_errors、真实公开utility；meta-before-hook、malformed known keyword、mixed items/prefixItems/ref siblings、grapheme、21formats/content、metadata refs/$schema frames、cycle/regex分支错误均有正反例。保留raw与RDF09 root修正，无隐式I/O/新配额。私有bypass、blanket拒绝、leaf hook假定均不能关gap | add差分/外部consumer及worker持permit；update引擎假定；retain既有安全/normalization；未实施未执行 |
| WS · V-02/30/49/58 | 固定actual global constructor/runtime/build和headers可用性；观察真实Upgrade/帧/协商，显式WS/Auto的SSE fallback另记。确定性AsyncRead测累计read检查及coalesced tail、124header、Connection token-list、extension/subprotocol/Accept，公开adapter再验组装。压缩先证参考真实支持；不能只调size或SSE成功 | addnative机制＋SDK loopback/cancel/cleanup；update过度简化header/TCP说法；retaincontinuation/partial；未实施未执行 |
| HTTP · V-01/05/09/29/30/51/55/58 | 普通Bedrock H2与明确H1分别ALPN/wire；scoped proxy、双TLS CONNECT/overread、NO_PROXY、source-specific redirect方法/body/header、实际leg/retry/callback计数与取消，无越权秘密/身份重发现。root features/lock中h2不替代有效feature证明 | add各source loopback与canary；update统一redirect/proxy假定；retaincustom零二次包；未实施未执行 |
| Trust/crypto · V-09/49/55/58 | 真实runtime/platform roots正反例；固定参考接受key集合再验ring边界、精确JWT header.payload/独立验签；typ/kid无依据默认撤回。RS256签名与mTLS算法/leaf/trust roots/认证池隔离分别证明 | addkey/mTLS/企业CA语料；update签名与mTLS混同断言；retainhostname/server验证；未实施未执行 |
| Auth/source · V-05/09/12/43/55/58 | full AWS Continue/Terminal/priority/MFA/SSO/login/DPoP/ACK/source effects；standard/adaptive/2026发送与等待分别记录。ADC各家族/external/direct-metadata POST retry/redirect与模型target分离；缺gcp-metadata源码不填写1s。executable gate先于cache/spawn、invalid/stale区别、merged stdout+stderr、scoped env/tree cleanup差异；无新总budget | addisolated源/clock/store/process fixtures；update链/模式/stdio假定；retain无账号fallback与rotation；未实施未执行 |
| Telemetry · V-46–48/58 | matching Span+Dispatch捕获，A创建/B poll/Pending/resume/unpolled Drop/producer/Stream/Future跨线程，noop不接ambient、原Result/poll/send/功能callback错误不变，无wire/secret字段。不能仅Future组合器通过 | add真实public入口传播/Drop/recorder失败；retainhost领域和functional错误；未实施未执行 |
| Graph/platform/catalog · V-20/42/49/56/58/59 | exact Cargo graph/features/build/native/proc-macro/MSRV＋外部package consumer＋三native平台；匹配revision的hydrated数据/schema/hash/版本选择与offline资源真实存在。generator/test文本不补产物，CI路线不等于执行 | add允许目标的package/平台/数据fixtures；retain现有wire/目录隔离；未实施未执行 |

后端/差分harness观察deadline仅用于隔离实验，不变SDK拒绝政策。缺runtime/源码/产物、未实施、未执行及实际失败分开；不能以修改backend默认、typed Unsupported、source数量或caller override记完成。本轮test impact **none**，产品测试add/update/delete均0；表内处置均为未来建议。

## Schema／WS 最小离线实验观察（不是V通过）

[固定输入／全部结果／候选判定](pi-1.3.0-schema-ws-offline-validation.md)及[独立复审](pi-1.3.0-schema-ws-offline-review.md)对应本节。原59条V要求行、67条AIR映射和41＋7原测试处置保留；下面只记录证据层次与下一步，不构成第二验收表或实施账本。

| 既有V | 已运行观察与候选判定 | 尚缺证明／后续测试处置 |
|---|---|---|
| Schema／RDF-09 · V-11/34/35/39/41/58 | 85 Pi公开utility cases、276 native observations；meta-before-hook、grapheme／format／numeric／ref／regex双向错误实际复现。content knobs和prefix hook只修局部，选型重新比较；原始／JS parse／normalized分层，root修正与RangeError不作错误oracle | 完整Rust normalizer／公开utility／外部consumer、refs／formats全域、branch-correct regex补救、worker/cache/cancel未证；add复用固定反例／正例，update引擎假设，retain raw／安全／union。OPEN |
| WS／RDF-13/18 · V-02/30/49/58 | 60参考constructor＋15native loopback＋16reader＋15partial-codec观察；Node／Bun实际PMD与headercanary、Bun125header与native差分；当前TW完整组合不能原样保留。Node异常退出／Bun非法响应宽松单列；partial接入不补PMD | 完整Pi／opi provider、fallback／continuation／pool／cancel／cleanup、完整Upgrade与多帧tail、PMD参数／fragment／context takeover、proxy／TLS／平台未证；add共同wire／机制fixtures，update选型假设，retain资源／partial。OPEN |

所有正式实验已记录命令／exit／pins，仍不能把native对错误输入的假成功算通过。该最小差分窗口的未关闭组七组不变；其余五组在该窗口未增运行证据，后续新增见下节。harness timers与采样大小不是SDK预算。实际产品test impact **none**；研究脚本／fixtures／结果 **add**。废弃Bun自动解析诊断只作副作用记录，正式Bun --no-install已重跑；没有产品测试迁移／CI实施。

## 设计 READY 的机制证据与产品 OPEN（同一 V）

用户明确选择design-only READY；[设计决定](pi-1.3.0-design-readiness.md)与[独立复审](pi-1.3.0-design-readiness-review.md)只判断关键选择/合同/可表达性。所有54 active SDK组、4host-impact仍未完整产品验收；V-52退役不计通过。本节映射新增研究证据，不新增V/AIR、不把研究case当覆盖率，不改41＋7产品测试的add/update/delete/retain迁移义务。

| 既有入口 | 本轮机制证据与确定选择 | 实施验收仍OPEN |
|---|---|---|
| Schema · V-11/34/35/39/41/58 · RDF-09 | 私有TypeBox JSON方言IR、regress0.12.0算法/Unicode16数据；64/64固定实例与11合成错误控制、生成两次一致；ryu-js14点和URL29点，43/43叶，10包namespace与普通ICU2.2共图不串数据；source guard inert边界明确 | 全normalizer/42机制/21formats/ref/dynamic/evaluated/错误诊断/UTF8边界、publicutility/consumer/cache/cancel；保留原raw和root正确性修正，不复制JS parse损失 |
| WS · V-02/30/49/58 · RDF-13/18 | ownedUpgrade＋受控Ratchet PMD状态/帧修正；旧80观察及原缺陷保留。后续独立发现的长度/算术/分配边界，按最新WS纯内存探针另证；不自动继承旧网络运行记录到新patch | 全provider/continuation/pool/fallback/proxy/TLS/cleanup/platform，新增patch的网络组装未重跑；不得cap或SSE掩盖 |
| HTTP/Auth · V-01/05/09/29/30/51/55/58 | sourceowner＋Hyper动态存储；112主记录/16Pi proxy、20headers对照，ALPN/H2和TLS负例，metadata0/3000ms有来源。prepared身份/redirect腿/retry副作用不合并成最终字符串 | 全head/trailer/buffercap补救、framing/所有source/代理/取消/回调/公开driver、adaptive/SSO/MFA/DPoP/9OAuth/rotation/store/process树 |
| Crypto · V-09/49/55/58 · RDF-18 | OpenSSL3私有libctx成熟EVP，11RSA/三输入负例/poison共15观察，P521/e3mTLS差异3记录；rustls公开signer/WebPKI/verification/KX扩展明确机制，commonring不冒称完整 | 全源算法/优先级/协商/链/构建/平台、Rust FFI安全/包消费者；已测C API不当完整Rust TLS adapter |
| Telemetry · V-46–48/58 · RDF-14 | pinned safe poll/drop载体七场景，sourceDispatch/跨线程/Stream/producer/noop实际一致；不加domain/exporter | 全SDKnative/simple/direct/advanced/image/classifier/deferred入口、serde/wire/secretcanary、hostconsumer |
| Delivery · V-20/42/49/56/58/59 | 原Pi catalog helper人工42×3记录七观察；固定schema/身份/hash/staging/原子publication合同，隔离研究Cargo图和少量cross-targetcheck不冒充产品 | 真hydratedartifact与source关联、产品graph/namespace registry产物/packaging、MSRV1.97/三平台native build/run/consumer |

原始资产分别在[Schema](offline-probes/readiness-schema/README.md)、[WS](offline-probes/readiness-ws/README.md)、[transport/auth](offline-probes/readiness-transport-auth/README.md)、[runtime/catalog](offline-probes/readiness-runtime/README.md)。依赖/Unicode数据准备联网与运行时离线/loopback分开；用户最新限定后的WS修复只做纯内存无socket验证，若平台仍阻止便按指示跳过并保留OPEN，不尝试绕过。研究test **add**，产品test **none**，历史证据 **retain**，产品测试 **delete=0**。所有fixture timers/bytecount都是实验输入，未成为SDK预算。
