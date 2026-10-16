# Pi 1.3.0：Schema／WS 最小离线差分

> 2026-10-16；输入 Opi `8cb0fd68db528ae3d3c61874458d45e46bffa29c`。本轮执行隔离研究 harness，不改产品代码、根 Cargo、产品测试、CI、正式规范或实施账本。七组后端缺口仍全部 **OPEN**。实际命令、退出码、锁图和摘要见[冻结证据](pi-1.3.0-schema-ws-offline-evidence.json)，复现入口见[研究脚本说明](offline-probes/README.md)，独立复核见[复审结论](pi-1.3.0-schema-ws-offline-review.md)。本页是没有既有英文 counterpart 的中文技术附件，双语 spec 同步引用结论。

## 1. 实测后的选型判断

- **Schema：重新打开后端选型。** `jsonschema 0.28.3` 的未适配方案已被反例排除，换 draft 或少数公开 options 不足以保留为完整对齐后端。它只保留为比较／受控适配实验候选，没有“默认继续采用”的地位。若要保留，须先证明较小、可维护的 schema admission／ref／格式与数值 leaf／ECMA regex 适配，特别是组合约束内的错误通道；如果需要重写 applicator/ref evaluator，则应比较维护中的其他引擎／上游修复。本轮没有选择替代库、fork、JS runtime 或全量自研 validator。
- **WS：当前 TTW／TW 0.28.0 组合不能按现状保留为完整后端。** 固定的两个参考运行时都实际请求 PMD 并解码压缩消息；该 codec 没有 PMD，公开 `from_partially_read` 也不能补上。必须重新比较支持 PMD 的成熟 codec／维护中的上游补救路线。provided-stream／tail attachment 可以保留为机制参考，不能据此保留整个组合；本轮未选择新库或实施完整自有 Upgrade。
- 这些是**候选淘汰／保留条件**，不是 opi-ai 已交付或 V 已通过。Schema／WS 以及其余五组继续 OPEN；不以新增拒绝、提高 regex fuel、调大 cap、typed Unsupported 或 SSE 成功规避差异。

## 2. 固定输入与运行边界

| 输入 | 本轮实际身份／范围 |
|---|---|
| Pi | 本地 1.3.0；1925 文件，tree SHA-256 `08a98ac282590eeb8255dd326e566b8b456212b4018a6d59cce44caf7f62e9d2`；仍未核验上游 tag／发布包身份，仍缺 hydrated provider JSON |
| Node | Windows x64 `24.13.0`；exe SHA-256 `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`；内置 Undici `7.18.2`、V8 `13.6.233.17-node.37`、ICU `77.1`、Unicode `16.0`；不是 package-lock 中 npm ws／undici 的替代名称 |
| Bun | Windows `1.3.5+1e86cebd7`；exe SHA-256 `0025a8bc31f136745d102212ddb192a1876c721b71deeb43123d18375fcf030c`；`process.version` 是 Node 兼容值 v24.3.0，不能据它识别 Bun；完整 versions／构造器摘要另存 |
| Schema JS | 原始 Pi `packages/ai/src/utils/validation.ts`，仅 loader 映射 bare TypeBox import；TypeBox **1.3.27**，tgz SHA-256 `5a288bcd3d68f44567de84b185e04d205ef7d5b53602846bc3db30fdd219dda3`，1385 个归档文件逐字节核对；不复制算法、不用 stub |
| Rust | rustc **1.99.1** `58cf8f9c3`、Cargo **1.99.1**；`x86_64-pc-windows-msvc`。jsonschema **0.28.3**／referencing **0.28.3**／fancy-regex **0.14.0**；TTW／TW **0.28.0**，tokio **1.52.3**，serde_json **1.0.149**；两个隔离 Cargo.lock／有效 features 单独冻结 |
| 依赖与目标 | Schema 图继承产品 lock 中相应版本后由 Cargo 离线裁剪；WS 发布源码完整解到 TEMP 并 path patch，逐归档 member 核对。只构建研究 crate，使用既有外部共享 Cargo target、保持 incremental；不改变产品图，不证明产品 feature union、MSRV 或三平台 |

这些运行时是本轮明确选定的参考构建，不能代表所有满足 `node >=22.19.0` 的版本或全部 Bun。Schema 只跑 Node；WS 同时跑 Node／Bun。运行环境白名单去掉真实 auth/proxy/NODE_OPTIONS 等注入；Schema 显式 deny retriever，WS 只绑定 `127.0.0.1`，使用固定假 token／canary，没有供应商／OAuth／metadata 调用。

依赖准备阶段从官方 static.crates.io 补取 `data-encoding 2.11.0`、`utf-8 0.7.6`，均与隔离 Cargo.lock checksum 一致；它们是测试图输入，不是新增产品依赖。另有一次 Bun 默认导入诊断**未关闭自动依赖解析**，实际生成工作区 `node_modules/.bun-cache`（含 partial-json 0.1.7，共10文件）；其来源／联网范围未被观测，不能声称整轮完全离线或只有两次归档获取。已记录每文件摘要并将整目录原封移到 TEMP 隔离，工作区目录已移除。该诊断废弃，不计 offline/provider 证据；正式 WS 全部以 `--no-install` 重跑。没有把自动解析的未知包加入参考图。

## 3. 方法与实际执行层次

| 层次 | 实际执行与限制 |
|---|---|
| Pi Schema 公开边界 | **85 cases＝74普通＋11隔离进程 case**。直接导入真实公开 `validateToolArguments`；另记 TypeBox Compile／Check／Errors、raw before/after、JS parse 后输入与 normalized。不是根 index 全包构建／Pi suite |
| Native Schema | **276条结构化观察**；Draft7 baseline、Draft202012 baseline、Draft7 content-off；已知 formats 开启断言、未知 formats 放行、显式 deny retriever。另对两个 mixed case 试公开 prefixItems hook，对 malformed minLength 试公开 hook。profiles 是显式实验配置，不能称全部是后端默认 |
| 输入阶段 | Rust 分别消费 fixture 原始 JSON、必要时 `pi_input_js`、必要时 Pi 返回的 `pi_normalized`。大整数的 JS parse 舍入不冒充 normalizer；把 Pi normalized 喂给 engine 仅隔离 validator 差异，**没有实现 Rust normalizer／opi-ai utility** |
| WS 参考 | **60 child observations＝2运行时×2构造形状×15 wire cases**。Pi 的 `new globalThis.WebSocket(url,{headers})` 形状与标准构造器分别测试，保留 Upgrade／假 auth／canary／PMD offer／帧与退出码。未执行 Pi 的完整 public provider，也未运行 Bun proxy wrapper／真实 TLS |
| WS Native | **15 TTW loopback＋16确定性 supplied-reader＋15 public from_partially_read codec observations**。同语料服务按各请求携带的 key 计算 Accept；native使用固定RFC sample key，便于确定性机制实验，不证明随机nonce生成。一字节 read 只在自定义 AsyncRead 中确定控制。TCP 服务 write 次数不代表 client read 次数 |
| 受控 provider 诊断 | 原始 Codex 模块的 Node import 与 Bun `--no-install` import 均因缺 **partial-json 0.1.7** 失败。保留原错误／退出码，没有安装整个 Pi 图；构造器 probe 不能记为 Models／stream／fallback／continuation 已组装 |

每个 child 的实际退出码与 stdout/stderr 分开保留；构建失败不是验证失败。早期 WS 缺两项 cache／部分旧提取目录缺模块的构建失败已保留，补齐官方归档并完整解包后 `cargo build --locked --offline` 成功，不能用那些环境失败判断候选行为。Schema runner 对关键命令失败／case缺行失败关闭，执行前后核对固定输入。

隔离进程的观察 deadline、服务关闭 timer、人工125／140 headers／66000字符／70000字节等均为**实验输入或 containment**，不进入 SDK 默认预算，也没有由采样点推导新的通用极限。没有全量测试通过率；下表的正反例只是固定语料的实际结果。

## 4. Schema：由源码推断升级为实际反例

精确输入与全部结果在 `offline-probes/schema/cases.jsonl`、`results/pi.jsonl`、`results/native.jsonl`、`summary.json`。源码解释延续[前置核查§2](pi-1.3.0-backend-gap-audit.md#2-schema公开机制可表达性先于选型)，实测优先于旧“待执行预期”。

| Case／方法 | Pi／TypeBox实际结果 | Native实际结果及判定 |
|---|---|---|
| normal-valid／normal-invalid | 通过／拒绝，raw不变 | 三个baseline profiles同意；有真实正反控制 |
| mixed-pass／mixed-fail：旧items数组＋prefixItems | `[1,2]`通过，`[1,3]`拒绝 | Draft7两者都通过，202012两者schema build拒绝；公开prefix hook修复这两个简单子集，不证明ref/evaluated sets等完整语义 |
| malformed-minlength：minLength为字符串`"2"` | 空字符串通过 | 三profile build拒绝；公开minLength hook factory调用**0**，证明不能靠该hook越过metaschema前置拒绝 |
| grapheme-max/min-combining | `e\u0301`在maxLength1通过、minLength2拒绝 | Native相反；另含ZWJ／flag case。不能以Unicode scalar count或标准grapheme直接替代该TypeBox helper |
| 21formats各一正一反＋unknown | unknown通过；已知url／json-pointer-uri-fragment／uuid反例拒绝 | Native未知format放行正确，但url／json-pointer-uri-fragment未被同名覆盖，Draft7还忽略uuid。42点不证明21个算法的全接受集 |
| content-base64／content-json | `%`／`not-json`通过 | Draft7默认content支持额外拒绝；公开content-off knobs修复两个case，202012也通过；是局部可用机制 |
| local/sibling refs与metadata-id-ref | 正常local refs正反例有效；metadata中`$id`可解析 | metadata target在native触发deny retriever后build失败；需要准确内存resource索引。unresolved ref源侧validation拒绝／native build失败，阶段不能合并；retriever被调用不等于网络发生 |
| regex-u1680：`^\s$`对U+1680 | 通过 | 三profile拒绝，实际ECMA whitespace差异 |
| multipleof-decimal：0.3、multipleOf0.1 | 通过 | 三profile拒绝，固定浮点算法差异，不是工作预算 |
| normalize-*／arguments-error | 记录hex／null／optional-null／union／number-string与truthy argumentsError前置失败；空argumentsError不触发 | Engine不提供Pi normalization。raw／JS输入／normalized分层留存；85cases的原输入均未被Pi utility改写 |
| root-correctness：integer minimum10，args字符串`"2"` | utility原样返回`"2"`，但返回值TypeBox Check为false | Native拒绝。保留RDF-09最终原schema验证的正确性修正，不仿效源漏洞；root-valid-coercion `"12"`→12单列正控制 |
| self-ref：`{"$ref":"#"}` | 本固定Node的Check／公开utility抛RangeError（stack overflow），不是harness timeout | Native build成功且true。递归与失败分类仍OPEN；不默认拒全部cycle，不要求Rust制造栈溢出 |

真正运行期 regex 错误语料为 `P = (?:(a|b|ab)*(?=c)|.*)`、subject 为 `"ab"` 重复20次（40个ASCII字符）。前置探索找到真实错误后才固定该case，不猜输入阈值。

| 组合case | Pi实际结果 | Native三profiles实际结果 |
|---|---|---|
| bare／anyOf／contains／propertyNames／allOf／additionalProperties | 成功 | 失败，含真实 `BacktrackLimitExceeded`／组合错误；additionalProperties把本应由pattern覆盖的key视为未覆盖。错误不等于普通不匹配 |
| not／if／patternProperties | 拒绝 | **is_valid=true、validate=Ok、iter_errors=[]**；错误被分支boolean通道转成不匹配后可产生假成功 |
| unselected-anyOf（前一合法分支已满足） | 成功 | 成功；不能预扫所有pattern后全局拒绝整个schema／instance |

因此只从 `validate` 换到 `iter_errors`、提高 fancy-regex 默认 fuel、在外层加 blanket guard 或替换一个仍返回bool的leaf，均没有证明修复组合语义。实验没有实现新regex后端／fallible evaluator。下一轮若走受控适配，须复用这些反例证明小型机制可表达并保持分支语义，而不是在规范里预先宣告“adapter会解决”。

## 5. WS：运行时需要、异常与后端缺失分开

源码解释见[前置核查§3](pi-1.3.0-backend-gap-audit.md#3-ws运行时握手与压缩分别证明)。实际wire与原始退出码见 `offline-probes/ws/ws-node-bun-results.json`、`ws-rust-loopback-results.json`，确定性reader／codec结果另存。15个 wire case 的全部结果都保留，不挑选最终字符串。

| Case／机制 | 固定参考实际结果 | Native实际结果／含义 |
|---|---|---|
| baseline101＋首帧／tail-text70000 | Node／Bun首帧成功，Pi形状发送假auth与canary | TTW loopback成功；codec attachment读出首帧。两个attachment控制还额外观察下一次next为EOF协议错误、没有重复text；多帧／取消的完整生命周期仍未证明 |
| headers-total125 | Bun两shape都成功且exit0；Node虽交付message，随后异常exit `3221226505` | TTW `TooManyHeaders`；稳定的Bun正例已证明124槽会额外拒绝。Node的交付后崩溃不能记作稳定成功或要求Rust仿效 |
| headers124／140 | Node相关多header试验同样保留交付后异常退出；Bun140拒绝 | 保留不同runtime差异与完整退出码；不推导Pi统一header count默认值 |
| header-value66000 | Bun成功，Node拒绝该采样 | Native `AttackAttempt`；不是opi SDK的64KiB政策，也不声称完整准确Pi阈值 |
| one-byte reader；tail-text70000确定性reader | 参考TCP写不能控制底层read粒度，没有对端同粒度证据 | 单字节reader第65次read触发`AttackAttempt`；70000 tail在两次read内首帧成功。累计字节检查已由66000 header触发；本轮未实际证明coalesced tail恰好跨该阈值或独立>512read路径 |
| Connection token-list | Node／Bun都拒绝此输入 | Native也拒绝；本轮**没有**证明这里是双端接受集差异。仍是owned RFC验证方案的待证项，不把旧源码风险当已证parity失败 |
| invalid Accept／unsolicited extension／subprotocol | Node拒部分非法响应；unsolicited subprotocol还触发uncaught exit1。Bun部分意外宽松接受 | Native拒invalid Accept，但意外接受unknown extension。安全验证负例保留；不要求Rust复制宽松接受／进程崩溃，不因此放弃完整协商验证 |
| PMD offer＋压缩消息 | 两runtime／两shape均实际offer `permessage-deflate`，在合法协商响应后解码RSV1帧成功 | TW不offer PMD／无该feature。若服务强发PMD，它的拒绝本身不是协商错误；但其当前codec在RSV1消息上 `NonZeroReservedBits`，公开partial接入仍同样失败，证明确少参考已具备的能力 |

`from_partially_read` 实验明确只接**已分离的tail**，没有实现／验证HTTP Upgrade解析器。它证明公共codec接入点能读出首帧，并在两个控制中额外观察EOF而没有重复text，不能证明Accept/token/extensions/subprotocol/redirect/tail多帧／取消安全，也不能让TW具备PMD。不允许向无压缩codec宣告协商PMD或在message外随意套deflater。

故当前“只移除size caps”“只改握手”“显式WS失败后SSE有回答”路线均不能保留原组合作为完整选型。后续比较应至少覆盖PMD协商参数、fragment/control、context takeover、无压缩回退、scoped proxy/TLS、背压／取消／cleanup、有效features与三平台，并越过真实Pi／opi provider公共入口；本轮没有替代库源码／编译比较。

## 6. 未关闭项与后续动作

本轮只把部分“参考未知／源码推断”升级为可重现观察：不是完整覆盖证明。唯一验收仍是 [V矩阵](acceptance-matrix.md)，对应 Schema V-11/34/35/39/41/58、WS V-02/30/49/58，差异 RDF-09/13/18。AIR67／V59／RDF20身份不新增，原测试迁移处置保留。

Schema剩余：完整formats／URI与mutable registry、`$schema`/legacy frames、encoded/duplicate/dynamic/mutual refs、真实Rustnormalizer、publicconsumer、worker/cache/cancel/resource行为与维护成本。WS剩余：完整provider、Auto/explicit fallback、continuation/pool/cleanup/cancel、多帧tail恰好一次、PMD完整参数与碎片、proxy/TLS、平台与产品feature图。其余HTTP、trust/crypto、auth/retry、telemetry、Cargo/platform/catalog五组未在本轮运行验证，全部OPEN。

`固定输入 → verify: exe/archive/source/fixture/lock hashes`；`重复实验 → verify: raw stdout/stderr/exit与正反控制`；`选型修订 → verify: 实测限制与未证能力分开`；`独立复审 → verify: 精选重跑、来源／双语／V一致性与文档检查`。新的补救／替代后端沿用同一语料，不覆盖历史原始结果。测试影响：产品 **none**；研究 harness／固定fixtures／执行证据 **add**。没有提交／推送或实施准入。
