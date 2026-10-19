# Pi 1.3.0 对齐：设计就绪判定

> 2026-10-16，**设计 READY**（非规范性）；独立复审确认无未决设计阻塞，七组产品实施与平台验收仍 OPEN。Opi 输入 `8cb0fd68db528ae3d3c61874458d45e46bffa29c`，沿用上一轮未提交研究文档／资产；不改产品代码、Cargo、产品测试、CI、正式规范或实施账本。中文技术附件没有既有英文 counterpart；双语 spec 同步其决定。

> 2026-10-19 取代记录：用户批准将 Schema 的固定 Unicode16／vendor 数据路线改为 [RDF-21 上游 profile](pi-1.3.0-rust-differences.md#rdf-21-upstream-data-profile)。下表同步现行选择；原实验、独立复审与历史摘要保留原窗口，不作为本次实现结果。20.11、20.34、20.57 的完整验收仍 OPEN。

> HTTP 单独修订：采用 [RDF-22 上游接受范围 profile](pi-1.3.0-rust-differences.md#rdf-22-upstream-http-profile)，退役 Hyper 源码补丁／HTTP namespace 必建义务；WS 受控 Ratchet 与 crypto 不变。旧原始结果／patch／复审保持历史身份，本次合同与 plan 不构成运行证明。

## READY 的含义与不变门槛

用户本轮明确选择：“设计 READY：关键选型和合同有依据、无未决设计阻塞；产品实施及平台验收继续明确标为 OPEN”。因此设计结论与产品覆盖是两种判定，不修改原七组覆盖的关闭条件，也不把它们重命名为通过。正式 `opi-spec` §8.1／PRIN-002／GATE-002、Placement Review 和 `opi-implement plan` 的准入权威不变；本页不审批依赖、实施或功能进入 Agent Core。

设计 READY 要求固定来源／运行时，关键可表达性有源码和可运行机制依据，决定不依赖私有 API、虚构预算、静默能力删减、typed Unsupported 或 SSE 成功遮蔽，且每组有明确 owner、状态／失败、维护及既有 V 验证入口。完整 SDK 组装、公开 consumer、所有合法输入、平台／MSRV／供应商和产物真实性，必须由实施后的真实验收证明。

本轮先复用[已有离线反例](pi-1.3.0-schema-ws-offline-validation.md)，再比较／验证补救，不用旧“尚无替代选择”句子限制有证据的新设计。旧报告／独立复审／摘要固定其原输入，不改写历史。新选择只在本候选内取代被证明不足的机制，不引入第二公共框架或第二实施 ledger。

## 七组的设计决定和产品覆盖

| 组 | 设计决定／依据 | 产品覆盖与唯一验收入口 |
|---|---|---|
| Schema | 私有、JSON-only 的 TypeBox dialect IR evaluator；normalization、strict wire projection 与校验分离。ECMA regex 采用 crates.io regress0.12.0；URL 采用普通 url2.5.8，Unicode／IDNA 数据随受支持上游 profile（RDF-21）；JS String 仍用 ryu-js1.0.2。逐分支传播执行错误，不用标准 draft／meta admission／全 pattern guard 代替源码语义。详见 [Schema 选型](pi-1.3.0-readiness-schema.md) | **OPEN**：完整格式／refs／normalizer／公开 utility／consumer／cache/cancel；V-11/34/35/39/41/58，RDF-09/21 |
| WS | owned 严格 Upgrade／动态 headers／tail 保留＋Ratchet1.2.1 PMD codec、消息状态适配及受控frame/length/storage修正，明确版本和边界。TW 无压缩组合退出完整目标，SSE fallback 不作 WS 验收。详见 [WS 选型](pi-1.3.0-readiness-ws.md) | **OPEN**：完整 Pi/opi provider、continuation／pool／cleanup／proxy／TLS／平台；V-02/30/49/58，RDF-13/18 |
| HTTP／proxy／H2／redirect | 单一 source-qualified transport／retry owner；实际 H1/H2、proxy、redirect 的方法／body／敏感 header 行为逐来源区分，不继承 reqwest 默认当 Pi 默认；普通crates.io reqwest0.12.28/Hyper1.9.0的head/buffer/trailer/chunk-extension边界按RDF-22明示，不维护HTTP fork/namespace或预分配巨大固定值。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md) | **OPEN**：所有公开路径／source 和完整 transport 组装、取消、代理、parser 接受集；V-01/05/09/29/30/51/55/58 |
| TLS／trust／crypto | rustls 的实例配置区分 source trust／额外 roots／mTLS／server identity；ADC RS256／TLS signer/verification/KX由private OpenSSL3 EVP和rustls公开seam补足；default trust选固定Node参考bundle，scoped extra/explicit roots按source，host roots显式选择，不把ring或webpki限制说成Pi限制。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md) | **OPEN**：完整信任来源、密钥／算法／构建和真实服务；V-09/49/55/58，RDF-18 |
| Auth／ADC／AWS | 保留 owned 协议／来源 adapter、明确冻结身份和 source retry／redirect／metadata timeout／进程与 store ACK 所有权；不只实现 standard 模式，不叠 generation retry。详见 [transport/auth](pi-1.3.0-readiness-transport-auth.md)和现有后端合同 | **OPEN**：完整 source chain、九 OAuth、DPoP／SSO／MFA／adaptive、进程树、rotation／并发／副作用；V-05/09/12/43/55/58 |
| Telemetry native carrier | 保留 opi-ai 小模块；safe `Option<Pin<Box<T>>>` 在 frozen Span＋源 Dispatch 下分别 poll／drop，producer 本身也被包住；noop 不采 ambient。七场景真实编译／运行见 [runtime](pi-1.3.0-readiness-runtime.md) | **OPEN**：全部 SDK 入口和 consumer 传播、Serde／wire／secret canary；V-46–48/58，RDF-14 |
| Cargo／platform／catalog | root 管理 lockstep／features／锁图，native／unsafe 边界单独审查；固定版本化 artifact、schema／身份／摘要，staging 验全后原子 publication。人工目录 helper 验证不冒充原产物。见 [runtime](pi-1.3.0-readiness-runtime.md) | **OPEN**：产品 graph、MSRV1.97、三平台 native build/run、外部 package、真实 hydrated artifact；V-20/42/49/56/58/59 |

此表是现有合同的设计决定导航，不新增 AIR／V，不记录实施进度或关闭百分比。即使某个机制探针全部通过，也不把对应 V 标 PASS。

## 维护、成本和重新打开设计的条件

研究中的受控数据／源码补丁必须有原始版本／源码摘要、最小 diff、生成输入／命令、补丁后摘要、license 和回归语料。它们不是未经修改的上游发布包；实现不得只声明原 crate 版本而遗漏补丁身份。使用私有模块隐藏差异；无推测性公共 trait／crate／feature 配置体系。

Unicode／regex、PMD／frame 校验与签名初始化的受控修正各有有限责任边界；HTTP parser/storage 改按 RDF-22 的明示上游接受范围验证，不再承担源码修正。算法或安全修复一旦越过声明边界、需要新通用解析器／验证器、引入第二 transport/retry owner，或用固定 cap／删减可观察能力才能通过，必须重开本设计，不能在实施中静默扩大补丁或降级目标。上游提供等价公开修复后先重放固定 corpus，再按来源审计替换；版本升高本身不关闭缺口。

修改过的 Ratchet 继续采用受控 registry 可解析的独立第三方 package 身份、精确 version/source/checksum；HTTP 采用 RDF-22 普通 crates.io profile，不依赖普通 consumer 的 root patch 或 opi-ai lock。Schema 改用普通 crates.io regress／URL/IDNA/ICU，退役固定 Unicode16 分叉与10包 namespace 发布路线；旧共图隔离实测保留历史身份，不冒充新图证明。普通上游数据升级仍须审图／features／checksum、算法与数据差异、许可和回归。WS 的 namespace 仍必须沿真实 Ratchet core/ext/deflate 最小依赖闭包统一引用；HTTP 的 reqwest/hyper-util/Hyper/TLS glue 改审普通 registry 解析图与 feature union，公共 custom transport 不泄漏私有类型；WS driver 必须真实链接修正版，HTTP driver 必须真实使用声明的上游图，不能只添加无消费者的 fork。最终发布身份、registry 可解析产物与 external consumer 必须实际固定并验证；本次 Schema 子集验证不关闭完整分发验收，不 publish，不新增 Opi 领域 crate 或公共 trait。

原生压缩／EVP签名及TLS verification/KX增加的 C／assembly／构建工具／unsafe FFI 成本需要逐项声明。它们不能因为设计 READY 就自动进入 Minimal Runtime 或豁免供应链／平台／准入；也不能临时回退一个更窄的 crypto／codec 后端而继续声称等价。构造及静态读仍零 auth/env/network，显式操作才按固定 source 使用所需能力；不得更改进程全局环境、TLS provider 或 subscriber。

## 验证和独立复审

`固定输入 → verify: HEAD、carried-in 归属、Pi tree、runtime／source／archive／lock SHA`；`机制选择 → verify: 原反例、正例、控制错误与真实退出码`；`同步文档 → verify: 双语合同、RDF-01–22、67 AIR／59 V 身份与测试处置`；`独立复审 → verify: 新作者以外的 reviewer 重跑关键机制、核验原始结果和设计／实施边界`；`完成 → verify: 文档检查、diff、来源摘要及未关闭项`。

本轮产品测试影响 **none**；研究探针与固定证据 **add**，既有 fixture 复用／历史保留 **retain**。没有删除产品测试，不运行付费 provider／OAuth 登录／真实 metadata／generator；依赖／数据的准备联网与实际 loopback／离线运行分开记录。只报告实际发生的副作用，不继承上一轮“完全离线”的错误概括。
