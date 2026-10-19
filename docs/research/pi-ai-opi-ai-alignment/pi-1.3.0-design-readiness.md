# Pi 1.3.0 对齐：设计就绪判定

> 2026-10-16，**设计 READY**（非规范性）；独立复审确认无未决设计阻塞，七组产品实施与平台验收仍 OPEN。Opi 输入 `8cb0fd68db528ae3d3c61874458d45e46bffa29c`，沿用上一轮未提交研究文档／资产；不改产品代码、Cargo、产品测试、CI、正式规范或实施账本。中文技术附件没有既有英文 counterpart；双语 spec 同步其决定。

> 2026-10-19 取代记录：用户批准将 Schema 的固定 Unicode16／vendor 数据路线改为 [RDF-21 上游 profile](pi-1.3.0-rust-differences.md#rdf-21-upstream-data-profile)。下表同步现行选择；原实验、独立复审与历史摘要保留原窗口，不作为本次实现结果。20.11、20.34、20.57 的完整验收仍 OPEN。

> HTTP 单独修订：采用 [RDF-22 上游接受范围 profile](pi-1.3.0-rust-differences.md#rdf-22-upstream-http-profile)，退役 Hyper 源码补丁／HTTP namespace 必建义务；WS 受控 Ratchet 与 crypto 不变。旧原始结果／patch／复审保持历史身份，本次合同与 plan 不构成运行证明。

> crypto 后续单独修订：采用 [RDF-23 原版 rustls＋AWS-LC profile](pi-1.3.0-rust-differences.md#rdf-23-upstream-crypto-profile)，退役私有 OpenSSL EVP／本地 FFI／crypto namespace 和 `third-party/pi-crypto` 必建义务。WS 的受控 codec 方案与 RDF-18 trust 选择不变；[新增固定摘要](offline-probes/upstream-crypto/summary.json)保留 Windows scratch 研究窗口，不关闭产品、包消费者、MSRV或三平台验收。

## READY 的含义与不变门槛

用户本轮明确选择：“设计 READY：关键选型和合同有依据、无未决设计阻塞；产品实施及平台验收继续明确标为 OPEN”。因此设计结论与产品覆盖是两种判定，不修改原七组覆盖的关闭条件，也不把它们重命名为通过。正式 `opi-spec` §8.1／PRIN-002／GATE-002、Placement Review 和 `opi-implement plan` 的准入权威不变；本页不审批依赖、实施或功能进入 Agent Core。

设计 READY 要求固定来源／运行时，关键可表达性有源码和可运行机制依据，决定不依赖私有 API、虚构预算、静默能力删减、typed Unsupported 或 SSE 成功遮蔽，且每组有明确 owner、状态／失败、维护及既有 V 验证入口。RDF-21/22/23 明示的上游接受范围不冒称 Pi 接受域；完整 SDK 组装、公开 consumer、声明范围的正常及拒绝输入、平台／MSRV／供应商和产物真实性，必须由实施后的真实验收证明。

本轮先复用[已有离线反例](pi-1.3.0-schema-ws-offline-validation.md)，再比较／验证补救，不用旧“尚无替代选择”句子限制有证据的新设计。旧报告／独立复审／摘要固定其原输入，不改写历史。新选择只在本候选内取代被证明不足的机制，不引入第二公共框架或第二实施 ledger。

## 七组的设计决定和产品覆盖

| 组 | 设计决定／依据 | 产品覆盖与唯一验收入口 |
|---|---|---|
| Schema | 私有、JSON-only 的 TypeBox dialect IR evaluator；normalization、strict wire projection 与校验分离。ECMA regex 采用 crates.io regress0.12.0；URL 采用普通 url2.5.8，Unicode／IDNA 数据随受支持上游 profile（RDF-21）；JS String 仍用 ryu-js1.0.2。逐分支传播执行错误，不用标准 draft／meta admission／全 pattern guard 代替源码语义。详见 [Schema 选型](pi-1.3.0-readiness-schema.md) | **OPEN**：完整格式／refs／normalizer／公开 utility／consumer／cache/cancel；V-11/34/35/39/41/58，RDF-09/21 |
| WS | owned 严格 Upgrade／动态 headers／tail 保留＋Ratchet1.2.1 PMD codec、消息状态适配及受控frame/length/storage修正，明确版本和边界。TW 无压缩组合退出完整目标，SSE fallback 不作 WS 验收。详见 [WS 选型](pi-1.3.0-readiness-ws.md) | **OPEN**：完整 Pi/opi provider、continuation／pool／cleanup／proxy／TLS／平台；V-02/30/49/58，RDF-13/18 |
| HTTP／proxy／H2／redirect | 单一 source-qualified transport／retry owner；实际 H1/H2、proxy、redirect 的方法／body／敏感 header 行为逐来源区分，不继承 reqwest 默认当 Pi 默认；普通crates.io reqwest0.12.28/Hyper1.9.0的head/buffer/trailer/chunk-extension边界按RDF-22明示，不维护HTTP fork/namespace或预分配巨大固定值。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md) | **OPEN**：所有公开路径／source 和完整 transport 组装、取消、代理、parser 接受集；V-01/05/09/29/30/51/55/58 |
| TLS／trust／crypto | 原版rustls0.23.40＋aws-lc-rs1.18.2／aws-lc-sys0.46.0，私有safe KeyProvider／SigningKey／Signer glue，AWS-LC实例provider显式选择，不安装global或以union默认代替冻结TLS/cipher/scheme/KX/ALPN。RDF-23明示RSA1024／multiprime拒绝、P-521签名／验证／mTLS与不支持P-521 KX的区别，不要求完整Node算法。default trust仍选固定Node参考bundle，scoped extra/explicit roots按source，host roots显式选择。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md) | **OPEN**：声明密钥／协商接受范围、完整trust来源、真实ADC/mTLS/ALPN、宿主隔离／取消、native来源／普通union包consumer/MSRV／三平台；V-09/49/55/58，RDF-18/23 |
| Auth／ADC／AWS | 保留 owned 协议／来源 adapter、明确冻结身份和 source retry／redirect／metadata timeout／进程与 store ACK 所有权；不只实现 standard 模式，不叠 generation retry。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md)和现有后端合同 | **OPEN**：完整 source chain、九 OAuth、DPoP／SSO／MFA／adaptive、进程树、rotation／并发／副作用；V-05/09/12/43/55/58 |
| Telemetry native carrier | 保留 opi-ai 小模块；safe `Option<Pin<Box<T>>>` 在 frozen Span＋源 Dispatch 下分别 poll／drop，producer 本身也被包住；noop 不采 ambient。七场景真实编译／运行见 [runtime](pi-1.3.0-readiness-runtime.md) | **OPEN**：全部 SDK 入口和 consumer 传播、Serde／wire／secret canary；V-46–48/58，RDF-14 |
| Cargo／platform／catalog | root 管理 lockstep／features／锁图，native／unsafe 边界单独审查；固定版本化 artifact、schema／身份／摘要，staging 验全后原子 publication。人工目录 helper 验证不冒充原产物。见 [runtime](pi-1.3.0-readiness-runtime.md) | **OPEN**：产品 graph、MSRV1.97、三平台 native build/run、外部 package、真实 hydrated artifact；V-20/42/49/56/58/59 |

此表是现有合同的设计决定导航，不新增 AIR／V，不记录实施进度或关闭百分比。即使某个机制探针全部通过，也不把对应 V 标 PASS。

## 维护、成本和重新打开设计的条件

研究中的受控数据／源码补丁必须有原始版本／源码摘要、最小 diff、生成输入／命令、补丁后摘要、license 和回归语料。它们不是未经修改的上游发布包；实现不得只声明原 crate 版本而遗漏补丁身份。使用私有模块隐藏差异；无推测性公共 trait／crate／feature 配置体系。

当前受控源码修正保留在PMD／frame／length／storage的WS有限边界；Schema数据、HTTP parser/storage和crypto分别按RDF-21/22/23的明示上游profile验证，不再承担其vendor／patch／本地FFI必建义务。算法或安全修复一旦越过声明边界、需要新通用解析器／验证器、引入第二transport/retry owner，或用固定cap／未经登记的能力删减才能通过，必须重开本设计，不能实施中静默扩大补丁、恢复私有EVP或收窄声明的正常范围。上游更新先重放固定corpus并审接受集／算法／来源差异；版本升高本身不关闭缺口。

修改过的Ratchet继续采用受控registry可解析的独立第三方package身份、精确version/source/checksum；HTTP采用RDF-22普通crates.io，crypto采用RDF-23普通rustls/AWS-LC，不依赖普通consumer的root patch或opi-ai lock。Schema用普通crates.io regress／URL/IDNA/ICU，退役固定Unicode16分叉与10包namespace；crypto不新增替代目录／namespace。旧共图／privateEVP实测保留历史身份，不冒充新图证明。普通上游更新仍须审graph／features／checksum、native来源、算法／数据差异、许可和回归。WS namespace仍沿真实Ratchet core/ext/deflate最小类型闭包统一引用；HTTP／crypto审真实registry解析图和feature union，公共custom transport不泄漏私有类型。WS driver须链接修正版，HTTP／crypto真实使用声明上游graph/provider，不增加无消费者fork。最终发布身份、registry产物与真实普通／union包consumer必须实际验证；库lock不固定所有消费者graph或native输入。Schema子集或crypto scratch不关闭完整分发，不publish，不新增Opi领域crate／公共trait。

原生压缩与AWS-LC的上游C／assembly／FFI／build工具成本须逐项声明，SDK只用safe API，不新增本地FFI。crypto标准分发为非FIPS源码构建；须控制并记录native来源、真实工具链／flags／链接身份，拒绝未审system来源／bindings／skip-version／target override，不把crate checksum当native来源证明。允许上游CPU一次初始化、OPENSSL_ia32cap能力输入与OS entropy，不承诺零native初始化／零环境读取；仍禁止ambient身份／trust／proxy发现、passphrase UI和全局rustls／OpenSSL配置或provider mutation。构造及静态读仍零auth/source I/O/network。设计READY不授予Minimal Runtime、供应链／平台准入；未测native-tls/default-tls/native-roots/FIPS组合不继承union证明，不得临时换更窄后端后仍声称符合声明profile。

## 验证和独立复审

`固定输入 → verify: HEAD、carried-in 归属、Pi tree、runtime／source／archive／lock SHA`；`机制选择 → verify: 原反例、正例、控制错误与真实退出码`；`同步文档 → verify: 双语合同、RDF-01–23、67 AIR／59 V 身份与测试处置`；`独立复审 → verify: 新作者以外的 reviewer 核验原始结果和设计／实施边界，重跑须另记真实执行窗口`；`完成 → verify: 文档检查、diff、来源摘要及未关闭项`。

本轮产品测试影响 **none**；研究探针与固定证据 **add**，既有 fixture 复用／历史保留 **retain**。没有删除产品测试，不运行付费 provider／OAuth 登录／真实 metadata／generator；依赖／数据的准备联网与实际 loopback／离线运行分开记录。只报告实际发生的副作用，不继承上一轮“完全离线”的错误概括。
