# Pi 1.3.0：设计就绪独立复审

> 2026-10-16，独立 reviewer；不是选型附件或原探针作者。复审口径是用户明确选择的“关键选型和合同有依据、无未决设计阻塞；产品实施及平台验收继续 OPEN”。研究与产品验收分别记录，本文不建立另一套 V 或实施账本。输入 HEAD 为 `8cb0fd68db528ae3d3c61874458d45e46bffa29c`；上一轮未提交的研究资产属于 carried-in，本文没有替它们重新声明产品通过。

## 最终结论

**核准设计 READY，没有剩余 must-fix 设计阻塞。** 七组选型、所有权／失败／维护合同和可表达机制已有源码或运行依据。复审发现的 R-01／R-02 已按下述限定修正并独立复验；Schema URL 私有依赖隔离和registry分发合同、TLS私有EVP扩展、真实依赖类型闭包及双语／RDF／唯一验收矩阵已经同步。该结论不是产品实施、依赖或正式规范准入，也不是任何 V 通过。

本报告保留原始失败和修正后的不同证据窗口。完整 normalizer、resolver、formats、provider lifecycle、平台、真实 catalog、最终registry产物和外部 consumer 继续实施／覆盖 OPEN；不要求本轮实现它们。曾经出现的 WS 进程失败及依赖语义数据串图也没有通过改名为产品 OPEN 略过，均有具体机制和回归依据。

## 发现与闭环

### R-01 · WS 声明长度可触发进程失败：设计层已关闭

先前六行 PMD patch 没有触及 Ratchet 1.2.1 的长度解析：`protocol/frame.rs:185–194` 将 64-bit wire 长度直接转为 `usize`，没有验证 RFC 长度最高位和最短编码；`framed/mod.rs:96` 直接加 header/payload 长度，`:161` 按宣告的不足字节一次 resize。使用当时选定的 owned Upgrade、`max_message_size=usize::MAX`，独立冻结 exe 后输入：

| 实际帧输入 | 实际结果 |
|---|---|
| `817f8000000000000000` | exit101，`capacity overflow` |
| `817fffffffffffffffff` | exit101，`framed/mod.rs:96` 加法 overflow |
| `817e000178` 后正常 Close | exit0，收到 `x`；未拒绝非最短126长度编码 |

原始命令、exe SHA、stdout/stderr 和退出码在 [独立反例](offline-probes/readiness-independent/ws-extra-results.json)；[脚本](offline-probes/readiness-independent/ws-extra.py)仅用于独占 TEMP 研究副本，没有更改产品。

最终选择已扩大为三个runtime文件的有限修正：协议最高位／最短编码、checked conversion/arithmetic、实际到达字节增长、fallible bulk reservation与typed表示错误；preferred扩容失败回退required精确容量，不新增MiB cap，不捕获panic来伪装解码。精确身份见 [最终输入](offline-probes/readiness-independent/ws-memory-final/inputs-and-negative.json)。

按用户最新约束，后续独立复验**只用纯内存、无socket／连接／请求／下载**。固定最终 `verify.py` SHA `31527137d391be33a390b7c4dfc20e1fb7661e0f37ccb502b94a88347736aa6a`、`source-files.json` SHA `a108bcc685e39d47fc67dd6bd400026cb3dd103bb4b96c3808c354563c0bf88c`，新TEMP执行 `cargo test --locked --offline` 的指定 `readiness_memory` 测试：13内核边界＋1个public codec测试（遍历12个冻结fixture），**14 tests PASS**；[命令](offline-probes/readiness-independent/ws-memory-final/commands.json)和[实际输出](offline-probes/readiness-independent/ws-memory-final/memory-test.txt)保留。错误source pin另以exit1、workspace未创建结束。

结果证明两个原panic输入得到typed非成功、非最短编码被拒、1TiB合法声明不提前预留payload、Pending／EOF／tail顺序以及PMD／fragment关键内存回归。8192只作内部read粒度；isize／usize是物理表示边界，不是SDK配额。可恢复bulk分配与常规allocator全进程OOM/abort分开，不承诺隔离Bytes shared metadata／PMD/native的一切分配。**旧80条网络观察没有重跑，也不移植为最终扩大patch的结果**；新patch的网络组装、完整provider及平台继续OPEN。

### R-02 · WS 摘要入口指向作者 TEMP：已关闭

旧 [summarize.py](offline-probes/readiness-ws/summarize.py)第2行把 `p` 固定为作者 TEMP。独立 reviewer 发现后没有运行旧入口。当前修正为 `Path(__file__).resolve().parent`，SHA `99fcd24dc658ec5b47c2ab77de1dd226e9d20ba92df087a80cf48a28208c4c8b`。

独立本地文件fixture从repo cwd执行新脚本：80-row复制正例exit0、缺本地results负例exit1；只读cached archive，历史作者和仓库results/summary的SHA均不变，详见 [本地正负证据](offline-probes/readiness-independent/ws-r02-local-summary.json)。最终memory-only入口另跑删一row负例，非零正确。没有启动Node/Bun或旧网络runner，也没有为修正重写历史raw。

## 已独立执行的验证

复用既有 `E:/opi/cargo-targets/windows-direct`，没有 clean、新会话 target、产品代码／根 Cargo／CI 修改、真实账户或外网服务。研究依赖和数据已由作者明确准备，本复审只使用已缓存归档、固定源码、synthetic fixtures 和 loopback。Bun 子进程均为 `--no-install`。

| 面 | 本复审实际执行 | 结果与范围 |
|---|---|---|
| Telemetry／catalog | 持久 `readiness-runtime/run.py` 全6命令；七 tracing 场景、七 original-helper synthetic catalog 观察 | 全 exit0；不会把 carrier 或人工126记录当公共 SDK／真实 catalog 已交付 |
| 输入负例 | runtime `test_run.py` 四项；WS 错 source pin | 四测试 OK；WS exit1 且 workspace 未建立 |
| Schema 数据生成 | 持久官方生成入口在新 TEMP 连续两次 | 表 SHA `475cb03589f0deb859b9a2ccfb34e3f33026a1e89500524024ce575a135e5daf`、测试 SHA `ae24799bc5c8ef417b72de6cbc2808fb581c28e6412284fa00c8f034cd21497c` 均与作者一致 |
| Schema 主机制 | 固定 Unicode16 后端持久 runner 全81命令；64个 TypeBox 同输入比较＋11合成 Result 控制 | 全 exit0、64／64同意、11控制满足；不代表 normalizer／完整42机制已完成 |
| Schema exe 隔离 | 比对 run 后 SHA，再复制 exe 至独占 TEMP，重跑64＋11控制／65命令 | 全部与上述逐对象一致；排除共享 Cargo target 的 binary 覆盖歧义 |
| Number／URL | `run_leaves.py --profile icu16` 完整7命令 | exit0，14 Number String＋29 URL 共43／43；数据／依赖隔离合同另审，不自动证明 consumer 图 |
| URL 依赖隔离 | `namespace_probe.py` 四命令，10项私有 package identity 与普通 url2.5.8／idna_adapter1.2.2／ICU2.2 同图 | 全 exit0，43／43；逐归档比对仅10份Cargo.toml变化，算法文件不变。不是已经发布可供外部解析的artifact |
| WS | cached archives 独立 pin／patch／locked offline build，再执行全20×4＝80观察；独立 RFC1951 扫描和两个 whole-owner Drop 观察 | 80主结果的 PMD、takeover、window、fragment、headers 与 owned Upgrade 接受／拒绝复现；其中非正常参考退出保留。Drop两例仅证明销毁，不证明取消后复用 |
| WS 最终修正 | 最新限定后的纯内存入口14 tests、12个public codec fixtures；两类local summary负例、错误source pin | 最终source pin与真实退出码均符合；没有socket/请求/下载，旧80不冒充新patch结果 |
| HTTP／TLS／auth | 持久 transport runner 全6命令，独立生成新 synthetic inputs | 全 exit0，112 records＋16实际 Pi proxy 调用；H2／H1／mTLS、TLS negatives、redirect legs 和 metadata timeout 符合断言 |
| 私有 OpenSSL context | 10个新 synthetic RSA key encodings、3负例和default-context poison后继续private signing | 14观察 exit0；10 RSA 的 JWS 签名与固定 Node 相同；未复用作者 multiprime fixture，不声称独立重跑第11把 key |
| Transport／crypto 精确来源 | 69个 source files 全部重新核 SHA；读取 rustls SigningKey／CryptoProvider／SupportedKxGroup／WebPkiSupportedAlgorithms 公开接口 | 全69与摘要一致；支持确定私有 EVP 扩展机制，不证明安全 Rust wrapper 和全部TLS算法已完成 |

原始独立结果保存在 [readiness-independent](offline-probes/readiness-independent/README.md)。命令、退出码、固定输入／raw摘要以及包装的完整输出可重核；没有将实验 containment、header采样数或消息大小变成 SDK 限制。

## 七组审查与后续边界

- Schema：私有源定义 IR／fallible 分支机制和固定 Unicode 数据是可以表达的路线；有限 prototype 不是完整 evaluator。URL／IDNA 的 Unicode16 数据选择通过私有 package family 与普通 consumer 图隔离已独立复现；附件明确选择10个受控registry后端包，以精确版本／source／checksum可解析地分发，承担许可证、漏洞和升级维护责任，不把TEMP纯path依赖写成已发布artifact。完整格式、resolver、normalizer、source guard、registry产物和 consumer 保留原 V OPEN。
- WS：PMD adapter、公开from-upgraded及取消后销毁owner路线明确；R-01／R-02均已按最终冻结源独立复验。有限三文件长度／storage／RSV修正取代旧六行边界，产品V／新patch网络差分、provider、pool、proxy／TLS、生命周期和平台仍OPEN。
- HTTP／proxy／H2／redirect：source-owned leg 的表达机制和 ALPN 实测成立；Hyper 动态 storage 的有限 patch选择可继续，完整 trailer/buffer／framing／pool实现不冒充已验证。source audience保护和来源 method/header差异必须在最终合同保持明确。
- TLS／trust／crypto：私有 libctx memory-only signing、无全局配置修改有源码和真实 poison控制；C API不等于安全 Rust wrapper。作者因复审补充实际P521/e3 mTLS差异后，确定公开 SigningKey、WebPKI算法映射与CryptoProvider/KX＋同私有成熟EVP扩展路线；源码可表达性成立，ring默认范围不再冒充Pi默认。全部TLS算法、FFI安全与平台仍OPEN，不能把RSA签名探针当TLS扩展已验收。
- Auth／ADC／AWS：owned coordinator、full source和standard/adaptive/2026 scope已给出确定算法来源；metadata3000／0替代猜测值符合实际。未运行真实全来源、rotation副作用及服务验收，不影响这些机制的设计可表达性；原 V保持OPEN。
- Native telemetry：source Span＋Dispatch、safe pinned inner poll/drop／producer均独立成立；没有任意subscriber隔离或自动领域span保证。公共入口传播仍OPEN。
- Cargo／platform／catalog：真实版本化artifact及原子last-known publication合同明确；synthetic helper不替代缺失原产物。研究锁图、Windows运行及两目标类型检查不是产品MSRV／三平台／package验收。

## 最终整合复审与机械状态更新

已对 [13份冻结输入及结论](offline-probes/readiness-independent/final-document-review.json)中的双语spec、四份机制报告、design decision、backend、RDF、SDK annex、README、唯一矩阵和tracking的当前合同／变更／相互引用进行终审。AIR-008结构guard、AIR-024新机制、AWS完整mode与RDF-17／SDK表一致；旧WS MSRV、源码轮“未运行”限定及metadata猜值均已纠正。Hyper／Ratchet namespace沿真正消费修正的最小类型闭包，custom transport不泄漏private类型；10包URL family有registry可解析分发路线与维护成本。没有新增猜测cap、隐式权限、第二retry owner或公共backend框架。

67 AIR、59 V及RDF-01–20身份保留；54 active SDK／4 host-impact／1 retired V分层不变。原产品41＋7测试处置不被研究fixture替代，七组产品覆盖全部OPEN；有意Rust UTF8／exact integer／root-validation／RS256算法正确性差异保留。研究source/数据/patch/native及FFI安全／正式准入／发布门禁没有被READY豁免。

核准root在冻结正文基础上仅做四处机械状态替换：English顶部 `READY — DESIGN ONLY, NON-NORMATIVE, 2026-10-16`；中文顶部 `READY — 设计就绪，非规范性，2026-10-16`；design-readiness首段标设计READY但保持非规范／产品OPEN；matrix首段标设计就绪而非实施准入或通过。该范围不批准其他合同变更。本文记录读过的freeze摘要和明确允许的替换，不反向声称读过root稍后生成的最终evidence字节；状态替换后的最终摘要／links检查由root另存。

本复审产品测试影响 **none**；研究证据 **add**，历史 fixture／输出 **retain**。未 commit／push，未改正式规范或实施 ledger。
