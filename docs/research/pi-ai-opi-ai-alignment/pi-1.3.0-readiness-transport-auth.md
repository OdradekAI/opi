# Pi 1.3.0：HTTP、TLS 与 Auth 的设计收敛

> 2026-10-16，非规范性研究。本文确定三个组的后端／owner／运行合同，供双语 candidate 采用；**设计决定已收敛，产品实现与验收继续 OPEN**。没有改产品代码、根 Cargo、产品测试、CI 或实施 ledger。本文不是完整 Pi adapter、跨平台交付、真实身份／供应商接受性证明。原始结果、pins 与可重放脚本在 [readiness-transport-auth](offline-probes/readiness-transport-auth/README.md)。旧[源码专项](pi-1.3.0-backend-gap-audit.md#4-httpproxyh2redirect按实际消费路径)仍是历史依据，新实际结果优先。

> 现行 crypto 选择按 [RDF-23](pi-1.3.0-rust-differences.md#rdf-23-upstream-crypto-profile) 改为普通 crates.io rustls＋AWS-LC 的明示接受范围；§4.2 的 OpenSSL 私有 libctx 选择及其原始结果保留历史身份，由 §4.3 取代。该crypto协调当时不改WS；后续RDF-24另选自有中立WS与原版压缩。RDF-18的trust source／reference bundle不变。新增 [上游 crypto 证据摘要](offline-probes/upstream-crypto/summary.json)只记录历史 Windows 研究，不能关闭产品、外部包或平台验收。

## 1. 三项确定决定

| 组 | 选定方案 | 被排除的捷径 | 后续产品状态 |
|---|---|---|---|
| HTTP／proxy／H2／redirect | crates.io 原版 reqwest 0.12.28／Hyper 1.9.0，采用 RDF-22 明示 parser/storage 接受范围；保留 source-owned route／redirect／retry 与实例 TLS 显式 ALPN。普通 Bedrock 保留 H2，明确 proxy／placeholder／WS classic 路径用 H1；不维护 HTTP fork/namespace | 把 reqwest 默认 redirect10 当来源规则；隐藏后端边界或伪称无限接受；“启 http2 feature 就得到 preconfigured TLS 的 H2”；由 SDK 二次发现 ambient proxy | V-01/05/09/29/30/51/55/58 OPEN；旧对照只证明其原 patch，当前 profile／产品组装须独立验证 |
| TLS／trust／crypto | 原版 crates.io rustls0.23.40＋aws-lc-rs1.18.2／aws-lc-sys0.46.0；SDK 私有 safe Rust 模块使用公开 KeyProvider／SigningKey／Signer，RS256 与 TLS 明确选择 AWS-LC 实例 provider。按 RDF-23 声明 RSA／P-521／KX 接受范围，不补本地 FFI、fork 或 crypto namespace；按源冻结 roots／proxy／identity | ring-only 或 AWS-LC 等于完整 Node/OpenSSL；隐式 builder／global provider install；将没有本地 FFI 说成没有上游 C／FFI／初始化；以替换 crypto 改 trust | V-09/49/55/58、RDF-18/23 OPEN。所测 safe 机制有证据；产品接线、普通／union 包消费者、MSRV／三平台／native 来源仍未验收 |
| Auth／ADC／AWS | 保留私有 full-source coordinator＋固定协议 adapter，统一受控 transport、已有 SigV4／成熟 HMAC/SHA、Query/XML codecs、p256 DPoP；**同时实现 standard、adaptive 与固定 Smithy 2026 分支**，不删 source／MFA／SSO／login／executable | vendor 默认 Client/Config 的 eager resolver、隐式 fs/env／trace；standard-only；选中 source 的服务失败后改选另一账户；metadata1s猜值 | V-05/09/12/43/55/58 OPEN。来源与模式是必须实施的合同，未把几个实测点提升为 full chain 通过 |

这些是实际技术选择，不再留“未来选择另一引擎／补 Unsupported”作为设计完成条件。source-scoped secrets、操作模式、取消／mutation owner 均是 SDK 私有机制；没有新增 public crypto／transport／retry 框架、scope 或默认预算。正式准入仍由 `opi-spec` 与既有工作流决定。

## 2. 固定输入和实际边界

Pi 使用已有固定 `.repo/pi-1.3.0`；package-lock SHA-256 是 `b686f30bc97e99aee6bb63c78714535ada1d307639db7f4e8972144059fc501d`。Node executable 24.13.0 SHA-256 为 `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`，其 OpenSSL 为 3.5.4。Bun 1.3.5 的 executable SHA-256 为 `0025a8bc31f136745d102212ddb192a1876c721b71deeb43123d18375fcf030c`，Bun 进程显式 `--no-install`。不是用 npm lock 中其他 fetch/WS 包替代 global runtime 行为。

官方 registry 的 **23 个 npm 生产依赖归档**由 Pi 原 lock 的 URL 与 SHA-512 integrity 固定后显式获取／展开；无 install scripts、npm install、Bun 自动解析。准备阶段使用网络；运行阶段只用固定源码、一次性 synthetic keys/certs 和 `127.0.0.1` 服务。另取只读 crypto 源码的 URL／bytes／SHA 在 acquisition manifests；不能把整个研究任务称为“从未联网”。

隔离 Rust crate 使用 rustc/Cargo 1.99.1、Windows x86_64-msvc；最终 Cargo lock SHA-256 为 `563a6181f6a498864bb7cd9393189b6c98e723870a4a465ebfeeae8feaa4a76e`，metadata 120 packages（119 registry＋本地研究 crate）。直接候选版本精确固定，传递图是本研究自己的已解析图，不冒充产品 root lock／MSRV／三平台。复用现有 `E:/opi/cargo-targets/windows-direct`，无 clean、target cache 删除或 session target。

最终可重放主 runner 六条命令全部 exit0，产生 **112 records（含一条 finish）＋16 Pi proxy observations**。另有 **20 header paired records、15 libctx observations（11 RSA＋3 invalid-input／algorithm cases＋1 isolation negative control）、3 supplemental mTLS observations**。最终raw与结构化结果保留。首次诊断中未配置 provider 的 native builder panic、未给 preconfigured TLS 配 ALPN 的 H1、fixture 偶然落到 fetch blocked service port、runner summary 的 None handling，都属于研究 harness 问题，已修复再跑；这些诊断不计最终成功证据，其记录区分保存的raw和transcript summary。

## 3. HTTP 的决定和明确差异

### 3.1 每 source 的 redirect owner

`probe.mjs` 对 Node global fetch、node-fetch 3.3.2、Gaxios 7.1.4 和 reqwest 0.12.28 执行五种 status（301/302/303/307/308）× GET/POST/PUT；另外验证 11/20/21 redirects。Google 的 reqwest 替代不能只用 `Policy::limited(20)`：方法／body／header／referer 也不同。

- 当前固定 Node global fetch、node-fetch、Gaxios 对 20 redirects 成功（21 sends），21 redirects 失败；reqwest 默认10，对 11 redirects 已失败，实际11 sends。这个实测 Node 值只属于本 executable/build；不能推广到任意 Node/Bun/WS/AWS。
- 在同 host、不同 port 的 302 POST→GET 上，Node global fetch 和 reqwest 去掉 Authorization/Cookie；node-fetch/Gaxios 保留。reqwest 还自动加入 referer。raw 逐 leg 保留 method/body/headers，没有只比较最后文本。
- `redirect:none + retry:never + no_proxy` 实际只发送一 leg、原样返回3xx，证明统一 owner 能获得 status/Location 处理权；没有实现或证明整个 Rust source-specific redirect driver。

**选定合同：** 保存每个来源实际的 hop/method/body replay/status 规则、header 变换及 retry 计数；库层不再自动 follow/retry。每 leg 只使用冻结 source/identity，既不刷新 source，也不追加另一账户的 key。Header/body 的 credential audience 以 prepared source 的既有信任范围为准；安全收窄必须列入既有 Rust authority/secret 差异和同一 V 的正反例，不能把“所有来源只能 same-origin”作为新的默认能力边界。**跨 port 也是 origin 变化；node-fetch 的行为不能被悄悄归一化成 reqwest 行为。** 不向未授权目的地发 secret/model data；custom transport 的能力与责任仍显式，零二次包装。必要且尚未由既有 authority 合同覆盖的权限取舍必须在实施前提交具体决定。

Pi proxy helper 的 **16 次真实源码调用**覆盖 scoped lower/upper、empty fallback、ambient lower、protocol/ALL_PROXY、domain boundary、port、IPv6、wildcard、SOCKS、invalid URL 和 bypass-before-parse。包括 `ws_proxy=proxy:80` 会按 `ws://` 补 scheme 再拒绝的实际细节；不能把 helper 当所有 Google agent/runtime 的默认代理规则。Rust source owner 复制实际路径的 resolver 语义，冻结 scoped env；owned CONNECT 先验证 HTTPS proxy，再 target TLS，并保留 overread，不在失败后直连旁路。完整 tunnel/proxy/redirect/cancel 产品 conformance 继续 OPEN。

### 3.2 上游 Hyper 接受范围与历史 patch

精确来源是 Hyper `src/proto/h1/role.rs:31,1017–1030` 的默认100 header 槽；public `client/conn/http1.rs:450–465` 的 `max_headers(n)` 按 n 直接分配，不能填 `usize::MAX` 来假装无 cap。buffer 的默认与 `max_buf_size` 是另一约束；reqwest 本版本未公开相应 H1 header/buffer builder。

普通合法200响应含99/100/101/125/200 headers：固定 Node/Bun 都通过；原 reqwest/Hyper 对101/125/200拒绝。TEMP prototype **只改变 Client response parser 的两个 storage arrays**：None 时依已缓冲 head 的实际行数取必要槽数，在第一空行停止，coalesced body 不增加槽数；Some 显式policy保留。原 httparse、status、framing、body decoding 完全保留。该原始 patch 已 `cargo build --locked --offline`，同20条 paired records 的 patched native 全部通过。

**现行生产实现合同（RDF-22）：** 上述 dynamic-head/trailer/buffer patch 选择由用户明确修订为普通 crates.io 上游 profile；历史 patch 与 paired records 保持原样。允许已声明的 response-head/read-buffer、trailer count/bytes 和累计 chunk-extension 边界拒绝其他方面合法的 Pi 输入，不再要求 HTTP fork、namespace 或 `third-party/pi-http` 必建目录。不承诺后端全部内部 allocation 可恢复，也不放大为 SDK body/SSE/WS cap 或任意资源保证豁免。固定版本、实际 graph/features/checksum 和边界须由真实公共 adapter、普通／feature-union 包消费者验证；B−1/B/B+ 与逐字节／合并输入不能只测 is_err。失败保留 typed 脱敏 cause／阶段、完整 partial/usage、终态及重试封口。完整 framing／conflicting CL-TE／obs-fold／interim responses／trailers／callbacks／EOF／cancel／pool 生命周期继续是正确性验收，不因 profile 差异放宽。TLS/crypto 按独立 RDF-23 修订，WS 受控方案不变，完整验收 OPEN，详见 [RDF-22](pi-1.3.0-rust-differences.md#rdf-22-upstream-http-profile)。

### 3.3 H2 与 TLS 的实际组装

`http2` feature 与 preconfigured TLS 本身没有为该 rustls config 填 ALPN。研究先观察到H1，显式写入 `h2,http/1.1` 后真实服务端报告 `httpVersion=2.0, alpn=h2`；H1 profile 报 `1.1,http/1.1`。因此每个冻结 route/TLS profile 必须显式设置协议与 ALPN：普通 Bedrock H2，来源明确要求的 proxy/FORCE_HTTP1/placeholder/classic WS H1。不能因经典 WS 选 H1 就把普通 generation 全降为 H1。真实 Bedrock adapter／proxy 场景尚未执行。

## 4. TLS/trust 与服务账户签名

### 4.1 TLS、根与 mTLS 不混为一个能力

实际 loopback 验证实例 `ClientConfig::builder_with_provider(Arc<ring provider>)`，无全局 provider 安装；显式 CA 后 h2、h1 均成功，未信任／wrong host／expired／缺 client certificate 都失败且服务端零 HTTP 请求。RSA、P-256、Ed25519 的三个 synthetic mTLS client 都在服务端授权并走 H2。这个局部证明没有覆盖所有证书链顺序、所有 key encoding、完整 Google certificate supplier、OS roots 或三个平台。

独立复审指出 ring 默认key范围不是Node完整范围后，追加同一TLS1.3/H2 wire的三个实际case：**P521和2048/e3客户端身份均被Node服务端授权，ring默认key loader拒绝；1024/e65537则Node自身先以ee key too small拒绝**，不当作有效mTLS差分。研究Rust helper在key-loader Err处unwrap而exit101，记录为显式的研究错误路径，不是产品panic合同。ADC的RSA签名1024正例不能推广为Node mTLS也接受1024。

**当前公开接入机制（RDF-23）：** rustls0.23.40 的 `CryptoProvider.key_provider`／`KeyProvider::load_private_key`、`SigningKey::choose_scheme`／`Signer::sign`、`CertifiedKey`／client certificate resolver 为 safe 私有 glue 提供接入点；AWS-LC 上游实现承担密钥解码、RSA／P-521 签名和相应验证。**继续由 WebPKI 验证 chain、hostname、time 和 trust，不改成 dangerous verifier 绕过**。不再补 OpenSSL EVP signer／verification／KX；旧公开 seam 可表达性并不要求本地维护另一套算法实现。

TLS source owner 冻结选定 RDF-23 profile 的 TLS1.2/1.3、cipher／signature scheme／group 集与优先级、ALPN，不能把依赖默认列表冒充 Pi 列表，也不能因 feature union 改变冻结配置。候选 KX 次序明确为 `[X25519MLKEM768, X25519, secp256r1, secp384r1]`；P-521 签名／验证／mTLS 支持不等于 P-521 KX 支持。仅提供 P-521 KX 的 peer 必须 typed TLS 失败、零业务 HTTP，不以静默换源或另造 KX 实现掩盖。完整 Node cipher／scheme／group／指纹等价义务由 RDF-23 明示上游接受范围取代；正常协商、证书／签名错误、协议／ALPN、host provider 不变与平台仍是实施验收。

**选定 trust 合同：** root source 与 source owner／revision 绑定，获取时冻结并按有效 trust/proxy/client identity 隔离 pool。固定 Node 的 bundled/default CA fingerprints 可以作为参考来源 fixture；本次干净启动实际都为146条。webpki 与任意 OS roots 都不自动等于该集合。native默认选择这一固定Node reference bundle；按source消费scoped extra/explicit roots，host-root source由caller显式选择。不把此Rust native默认冒充所有Bun／OS默认；scope 内的文件由 owned source 读取。不得让无 scoped-env 注入的后端任意读取真实 `SSL_CERT_FILE/DIR`、home、OpenSSL config 或把 subject token trust_chain 加入 server trust。默认集合及平台差异在 RDF-18 清楚记录；参考 bundle／host source 的正式打包和各平台验证保持 OPEN，不能写“默认系统根已完全等价”。

### 4.2 历史：ring-only 的排除与原私有 libctx 选择

本节保留 2026-10-16 的比较、机制选择、计数与原生责任，记录当时完整源接受域要求下为何选择私有 EVP。以下“选择／计划／必须”属于该历史方案；当前 RDF-23 已取代私有 libctx／本地 FFI 实施义务，不继承本节证据作为上游 AWS-LC 的运行结果。

固定实际 `google-auth-library/build/src/gtoken/jwsSign.js`→jws4.0.1→jwa2.0.1，用 synthetic key 生成精确 `alg=RS256` 的 header.payload。Node 对 PKCS#1/PKCS#8 的1024/e65537、2048/e65537、2048/e3、4096/e65537、8192/e65537全部签名且验签成功；ring 只接受2048/4096、e65537的四项，对其余六项在 key 阶段拒绝。不能用 signing4096/verification8192 混淆范围，也不能把弱 key 的拒绝称作 Pi 本有预算。

选择 **OpenSSL3 memory-only 私有 libctx＋成熟 RSA/EVP**。计划以 `openssl-sys=0.9.117` 的 raw ABI／native链接支持为基础（default features关闭；native包须正式准入时固定受维护的OpenSSL3 security patch与构建来源），仅为该版本缺少的 ctx-aware `EVP_DigestSignInit_ex` 做最窄私有 ABI 声明；不引高层 openssl 默认 PKey/Signer 初始化路径。ABI／init声明已读精确源码，未把其 Cargo vendored build、Rust wrapper 或传递闭包声称编译通过。实验 DLL 的3.0.15不是生产安全版本推荐；只证明 C API 机制。3.0.15与Pi Node同系3.5.4的关键初始化来源分别固定。

实际调用链：`OSSL_LIB_CTX_new` → 此 context 的 built-in `default` provider → `OSSL_DECODER_CTX_new_for_pkey(PEM,RSA,provider=default)` → 显式返回失败的 passphrase callback → `OSSL_DECODER_from_data` → `EVP_DigestSignInit_ex(SHA256,private_ctx,provider=default)` → PKCS1 padding → Update/Final。不用 BIO/fs、legacy PEM fallback、provider模块名、全局 ctx setter、`OPENSSL_INIT_NO_LOAD_CONFIG`、UI／TTY 或外部转换进程。crypto entropy 是必要的 OS CSPRNG 能力，不是身份发现。

实际 Windows DLL OpenSSL3.0.15 SHA-256 `630b0735e0a3ec5824084698a78d2043326bc133a5d579aa0f41cf0b5a92ddb6`。原10项加实际生成的2048三素数 key，**11项签名 SHA 与 Node 的精确 JWS 签名一致，且 Node 全部验签成功**。malformed／encrypted-no-passphrase 双方都失败；EC key 被 Node 错以 RS256 header 生成 ECDSA 签名，私有 RSA decoder拒绝，是明确的算法正确性差异，不是新增“允许换算法”选项或 RSA 能力收窄。

隔离负例把 `OPENSSL_CONF` 的默认属性设为不存在 provider，`OPENSSL_MODULES` 指向不存在目录：私有11项先成功；随后刻意对 default ctx 的 SHA256 fetch失败；私有签名再次成功。证明这个机制不靠全局禁config抹掉宿主策略。源码佐证：两个版本 `OSSL_LIB_CTX_new` 仅初始化 context；`ossl_provider_find`／provider枚举仅 `ossl_lib_ctx_is_default` 才 `LOAD_CONFIG`；`provider_predefined.c` 的 default 为内建 init pointer，`provider_init` 仅没有 init pointer 才读 MODULES／DSO。memory decoder和签名传同一私有ctx。此局部源码与poison实验不是全面FS syscall审计，但不允许把未来 wrapper 换回默认／legacy路径后继承本证据。

Rust safe外观必须隔离不可避免unsafe：非空句柄、完整 return检查、输入/输出长度与 checked转换、不可越界 callback、显式所有权、每次digest独立；先释放digest/decoder/key，再provider，再libctx；没有证明前不为裸指针宣称 Send/Sync。保持 host error queue／thread policy不被随意清空，外部 typed error 不调用会输出 key/body 的 Debug／日志。同步 native签名不能被async timeout硬杀；caller离开不提前释放仍被使用的key/context。实现时的 FFI 安全审查和平台构建仍是门禁。

该选择增加native C、build script及平台工具链／供应链成本；已解释并限制在私有EVP签名／verification／KX适配，不引另一套TLS协议栈／默认provider／auth resolver。aws-lc-rs1.18.1仍2048..8192，boring5.2.0只two-prime，均不能作为完整上述输入面的一键替代；RustCrypto rsa 的 [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html) 在本次核验的2026-09修订仍无修复，因此未选用它来规避native成本。不自写密码学原语。

### 4.3 现行：原版 rustls＋AWS-LC 接受范围与原生边界

采用普通 crates.io **rustls0.23.40＋aws-lc-rs1.18.2／aws-lc-sys0.46.0**，SDK 以公开 safe `KeyProvider`／`SigningKey`／`Signer` 使用上游密码学；不维护源码 patch、独立 namespace、`third-party/pi-crypto` 必建目录或本地 `extern`／unsafe FFI。ring 可以因宿主或传递 feature union 共存，但实际 SDK 路径明确选择 AWS-LC 实例 provider；不得调用 `install_default` 或依赖隐式 provider 推断。现有 HMAC／SHA、SigV4、DPoP 和 source coordinator 合同不因换后端改动。

接受范围按 RDF-23 记录：正常 two-prime RSA2048/3072/4096/8192、2048/e3、PKCS#1／PKCS#8 与 P-521 签名／验证／mTLS 须保留；RSA1024、多素数 RSA 允许 typed 拒绝，不能称 Pi 原本不接受。所测 P-521 PKCS#8／SEC1 与 PEM／DER、RSA 编码一致性只证明对应语料，不泛称全部密钥／证书编码。错误算法、无密码 encrypted key、public-only、损坏／截断密钥和坏签名继续失败；不允许 algorithm confusion、passphrase UI、secret Debug 或新增任意 payload cap。ADC JWT 保留仅 `alg=RS256` 的 header、来源限定 iss／scope／aud／iat／exp 与原 retry／endpoint；真实 service-account token→Bearer wire 机制不能关闭完整 ADC 工厂。

标准分发基线为**非 FIPS、上游源码构建**；上游仍含 C／FFI／平台汇编、build scripts、CPU 一次初始化、`OPENSSL_ia32cap` 能力输入及 OS entropy。“无本地 FFI”不表示纯 Rust、零原生初始化或零环境读取。允许这些已声明的后端机制，不允许读取 ambient 身份／roots／proxy／passphrase，或更改全局 rustls provider／OpenSSL config／provider。模型构造和静态查阅仍零身份发现／网络；crypto CPU 初始化不能被用作 source I/O 例外。

标准分发和实际普通／feature-union 包消费者须控制并记录 native 来源、工具链、实际 build flags／output 和静态链接身份；本轮研究明确 `AWS_LC_SYS_USE_SYSTEM=0`、毒化 `OPENSSL_DIR` 后通过真实 CC 源码构建。不能仅凭 registry checksum 或 opi lock 固定所有消费者的 native 输入；system directory／system bindings／skip-version-check／target-specific overrides 等未审替换必须在该分发／consumer 门禁中拒绝，不能静默继承源码构建证据。上游 C／FFI／license／安全更新、MSRV1.97、Windows／Linux／macOS 的构建与运行继续审查。native-tls／default-tls／native-roots 附加组合及 FIPS 未执行，不声明任意 feature union 皆通过。

取消边界保留 owned 生命周期：同步 `Signer::sign` 不可硬杀，worker 持有真实上游 key，caller drop／abort 后抑制 late publish，工作完成 join 后才释放；握手／等待响应／已读 partial 的异步取消须终止 owner 并观察连接清理，不把 Future drop 当全部清理完成。研究 partial-body 证据不代替产品 CallFailure／完整 partial／usage、终态／永久 EOF、无 handoff 后 retry 的验收。

[固定研究摘要](offline-probes/upstream-crypto/summary.json)区分实际计数：共9次 Rust 测试函数执行；RSA／P-521 为每 profile 43项，独立验签48个签名／96个篡改反例；45个 TLS／ADC／取消场景在五条件共225次观察，另10份 JWT、8个受限 KX 对照。宿主预装 ring 与毒化环境均验证显式实例选择，源码身份审查是7个 registry crate／2403文件。以上为 Windows rustc1.99.1 的历史 scratch 研究，不是本轮重跑，更不关闭真实 Models→ADC、完整 HTTP／WS adapter、普通／union opi 包消费者、三平台、MSRV，或20.11／20.34／20.57。

## 5. Auth 全来源、attempt 和副作用 owner

**确定架构：** 一个 frozen source/acquisition owner，来源读取与网络腿由其控制；来源必须报告 Continue／Selected／Terminal、identity evidence、实际 expiry、receipt knownness。选中来源的服务拒绝不触发另一账户发现。provider generation、source exchange、redirect腿、全call recovery分别计数，reqwest层全部关闭额外retry。私有protocol adapters复用现有calldriver，没有第二public auth/strategy trait。

| 来源／模式 | 必须保留的动作及 owner | 副作用／attempt／redirect边界与验收义务 |
|---|---|---|
| API key／explicit OAuth／九OAuth flow | frozen provider/source revision、presenter、clock、store与scopedenv；刷新仅同身份 | refresh网络独立15s来源保留；lock/modify/save ACK不计入伪造总15s。已开始rotation归store mutation owner，barrier跨Models按同store cutoff和key观察 |
| AWS Env／profile／INI | profile存在时跳过env；env complete pair、配置／凭据文件、role链／cycle／source_profile／credential_source／MFA按精确supplier分类 | 缺失是否Continue由各source真实error决定，不把所有parse失败都fallback；MFA presenter显式，无构造期读真实HOME。region/endpoint解析与account选择分开 |
| AWS SSO／SSOOIDC／login | SSO cache与session/profile、GetRoleCredentials、CreateToken、SignIn OAuth/DPoP操作齐全 | token子缓存与外层temporary credentials不同key/revision；写回要ACK、失败／unknown不丢唯一refresh token。注入clock/RNG；不触发第二默认SDK resolver或secret body trace |
| AWS process／web identity | trusted选定命令、token file、AssumeRoleWithWebIdentity | ProcessRunner显式authority／env、kill/reap；无shell插入。已选token文件或服务拒绝不发现另一身份。读/执行/交换分别记录，不能以Unsupported完成 |
| AWS ECS／IMDS | ECS relative/full＋tokenfile/token、host/protocol校验；IMDSv2/v1-disable、IPv4/IPv6、token/role/credential branches | 保留固定source真实timeout/attempt；选ECS失败不转IMDS。无构造期探测或无来源总deadline；中断与实际send计数可见 |
| AWS retry standard/adaptive／2026 | 完整standard quota/backoff/hint、adaptive CUBIC/rate bucket与首次send-token等待；固定Smithy2026 bool分支 | 读取scoped retry_mode/max_attempts/shared config；legacy/2026 constants各来自固定源码，不是研究预算。Bedrock每stream新client的quota生命周期不能扩大为跨实例global；auth source自己的retry不并入generation额度 |
| ADC service_account／authorized_user | canonical endpoint的RSA JWT／form refresh；source与quota_project保留 | standardPOST允许集／额外3retry、no-response2、status/delay按固定Gaxios调用配置；不在transport再叠一层。JWT header没有擅加typ/kid，scope/aud/issuer/claims来源明确 |
| ADC impersonated／external authorized user | source_credentials递归、target/delegates/lifetime；Basic refresh与可能返回新refresh_token | 无新默认chain-depth cap；source/target身份分离，latepublication fenced；真实expiry，唯一token写回走同mutation owner，默认不写回原ADC文件 |
| ADC external file/URL／aws1 | text/JSON subject field、RFC8693STS、optional IAM；aws1自己的region/env/IMDS＋GetCallerIdentity subject签名 | file/URL source headers/retry/redirect逐source；target凭据不加到subject请求。aws1不调用实际AWS STS、不顺便查Bedrock profile/SSO/process |
| ADC certificate | mutually exclusive config、leaf与trust_chain、source专用mTLS | trust_chain不成为server roots；auth mTLS pool不污染generation。rawkey/chain匹配与算法属于TLS/crypto合同，不从RS256直接推演所有mTLS能力 |
| ADC executable | 原env gate在cache/spawn之前；quoted argv、有效cache零spawn、malformed/stale区别、version/token_type/expiry校验 | 保留stdout+stderr合并解析、默认30s/允许5–120s；无另一默认deny或64KiBcap。scoped child env、kill/reap tree是Rust生命周期差异；SDK不写output_file |
| ADC direct/default metadata | 原factory的file/project/location检查与真正direct metadata分开；metadata headers和expiry | **本次已补精确来源及实测3000/0**，详下；Models缺file检查不得调用metadata、GCE/BIOS/网络健康探测或gcloud |

本表归并已有要求，不创建新V或新ledger。固定owner能表达完整来源，不意味着它们全部已经实现。参数／operationschema／error／partition endpoint／mode公式的来源更新仍由[跟进记录](upstream-tracking.md)维护；不能仅升级crate版本。

### 5.1 metadata 1s 的证据缺口已改成准确合同

新读到的 gcp-metadata **8.1.2** `build/src/index.js:118–178,398–400`：metadataAccessor调用 `requestTimeout()`，**getGCPResidency()为true时0，否则3000ms**；不是固定1s。residency由serverless env、Linux BIOS和MAC规则决定，默认cache在该模块。`isAvailable`的探测策略／fastFail并行DNS-IP与直接instance token请求不同，不能把探测的 DETECT_GCP_RETRIES 或source timeout扩大到所有ADC操作。

实际固定setter false/true分别得3000/0；loopback instance GET带 Metadata-Flavor:Google，合法响应成功；错误Flavor RangeError；4s延迟在约3000ms失败，实际一send。该一次timeout没有重试，不能从源码 noResponseRetries3 推导本失败必有4 sends；保留真实error/cancel规则和每次source配置。

Rust metadata owner冻结所选residency来源：明确值或**仅当metadata路径实际被选中后**按scoped environment／host提供的系统观察得到的结果；不读宿主真实不相关环境、也不在Models构造/available/check探测。0表达该source没有此timeout，不转换成默认SDK总deadline；3000仅metadata单请求范围，其他source timeout各按出处。真实探测分支与direct token入口分别验收。

## 6. 后续验收与交接

`选定机制 → verify: 本文pins、历史C API对照、现行safe上游crypto profile与parser接入证据` 已有局部实际依据；历史 probe 的执行窗口与计数分开保留。`产品接线／完整语料 → verify: 既有V与Rust外部consumer` 全部仍OPEN。尤其没有执行 full AWS chain／adaptive实现、OAuth真实服务、complete ADC工厂、代理TLS双隧道、全redirect与跨origin权限策略、上游HTTP／crypto profile完整公共边界验证、三平台、MSRV、完整native来源／供应链审查或真实opi外部package smoke。本地Rust FFI已退出计划，不能继续将其安全审查列为必建实现；上游原生边界审查保留。不得用本文状态覆盖这些OPEN。

测试影响：研究 **add**（脚本／isolated crate／source pins／raw结果）；产品 **none**（add/update/delete均0，现有产品测试retain）。没有paid provider、真实credential、非loopback metadata网络或用户目录发现；准备网络与crypto CLI都已显式分开记录。没有 commit/push。本页技术附件没有既有英文counterpart；双语spec须同步采纳选择与差异。
