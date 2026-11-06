# 当前 opi-ai 结构、测试与消费边界调查

日期：2026-11-06。性质：规划证据，不是实现或测试通过报告。仓库路径相对 D:/Luiz/Odradek/opi。

## 基准与结论

根代理记录初始基准 90f953d5ee288c165dc295bfdfb7cb7a05d7cecd、初始工作树干净。本调查核对 HEAD 相同，status 只显示根代理新建 docs/iterations/opi-ai-1-5-alignment/。本代理未改仓库，未构建、运行测试或调用服务。

验收依据：C:/Users/Luiz/Documents/Codex/2026-11-03/r/outputs/pi-ai-rust-acceptance.md，229–265 行规定职责与 R01–R12，267 行起规定 L0–L8/T。它要求职责、依赖与独立消费可验证，不规定 crate 数或照搬 TS 目录。

主要结论：现有 tests/README.md:10 已有正确的私有机制、公开契约、共享 helper、实际包消费者分层，应该继续采用。确认的结构问题是缺少最小能力配置、Models 内部责任集中、语义与调用层互相依赖，以及 public 测试探针。已确认过时的是部分说明与旧基准归属；本次没有证明任何整份大型测试应删除。

## 可复现规模

清点方法为 Python pathlib，UTF-8 decode 后 splitlines，字节取 stat().st_size。src 包含 inline tests、数据与 test_support，不能称为生产 LOC。可复跑：

    python -c "from pathlib import Path; r=Path('crates/opi-ai'); groups={'src':list((r/'src').rglob('*.rs')),'tests':list((r/'tests').rglob('*.rs')),'targets':list((r/'tests').glob('*.rs')),'src_no_tests_dir':[p for p in (r/'src').rglob('*.rs') if 'tests' not in p.parts]}; [print(k,len(v),sum(len(p.read_text(encoding='utf-8').splitlines()) for p in v),sum(p.stat().st_size for p in v)) for k,v in groups.items()]"
    python -c "from pathlib import Path; import tomllib; p=tomllib.loads(Path('crates/opi-ai/Cargo.toml').read_text()); print(len(p['dependencies']),len(p['dev-dependencies']),p.get('features'),[k for k,v in p['dependencies'].items() if v.get('optional')])"
    rg --files crates/opi-ai
    rg -n '#\[(tokio::)?test' crates/opi-ai/src crates/opi-ai/tests

| 范围 | 文件 | 行 | 字节 |
|---|---:|---:|---:|
| src 全部 .rs | 139 | 98,997 | 3,954,448 |
| tests 全部 .rs | 83 | 127,285 | 4,782,755 |
| tests 根 .rs（集成 target） | 73 | 123,584 | 4,627,566 |
| src 排除名为 tests 的目录 | 120 | 94,597 | 3,798,639 |

最后一行仍含 inline tests。子目录 auth 33文件/20,310行、schema 16/9,425、ws 17/5,792、catalog 5/3,665、crypto 9/1,508、api 5/3,873；ws/crypto 数字含私有测试。正则静态 test attribute 数为 src 369、tests 2,540，不是运行后的案例数。

集中点：models.rs 5,221行且没有 cfg(test)；anthropic.rs 3,540（inline test 从3051起）；openai_chat.rs 3,515；tls/node_trust.rs 3,344（含证书数据）；bedrock/mod.rs 2,618；schema/eval.rs 2,347。最大测试文件 bedrock_fixtures 10,276、oauth_flows 8,304、anthropic_fixtures 7,507、provider_collection 7,087、auth_contracts 5,617。README.md 3,249行。大小只选择审查顺序，不证明拆分或删除必要。

## 结构事实与建议

### 最小配置、领域依赖与公共面

事实：crates/opi-ai/Cargo.toml:12 有33个无条件普通依赖；:50 有Windows条件依赖；:53 有13个dev-dependencies；没有 features 表或 optional 普通依赖。reqwest、AWS codecs、p256、regress、rustls/aws-lc、libz/flate2 都常开。src/lib.rs:7 起有45个public root modules；catalog/mod.rs:50 无条件 include_bytes 纳入目录数据。它证明编译闭包没有最小能力隔离，不能证明运行时一定联网或加载全部目录。Models 的 lazy 构造与明确注册已有价值（models.rs:579、:590）。

建议：先记录核心、运行入口、协议、宿主、目录数据允许依赖，再决定 features 或少数 crate。R01/R12 的核心+Faux 配置必须确实不引入可选网络/SDK/目录数据；完整主配置同样保留在发布验证集合V。不要先承诺固定 crate 数。

事实：model_info.rs:15 → stream.rs:11 → provider.rs:12 → model_info 是源码依赖环；provider.rs:7 使用 reqwest headers；transcript.rs:245 实现到 ProviderError 的转换，:724 接收 provider::Request。文件边界不是无环职责边界。可以把相互依赖的数据类型归一个逻辑域，但不能仅重命名目录宣称 R01 已过。建议把值类型、usage/cost、通用错误与纯 transcript 规则收进不依赖传输的语义边界，把 Request/Provider 适配与转换留调用侧。

事实：lib.rs:57 的 doc-hidden public test_support 没有 cfg-gate；test_support.rs:719 的 force_sequence_exhausted 调 identity.rs:130 直接改私有 sequence；tests/public_utilities.rs:898 调它。这是确切测试专用公共控制面。建议 exhaustion 机制证明移到 crate-local，保留公共正常生成/格式/错误类型断言。MockProvider/Faux 有消费者及验收能力，不能连整个 test_support 删除；应分清正式 Faux、外部测试支持和私有探针。

### Models 与协议 Adapter

事实：models.rs:534 同时持有 registry/resource owners、多套auth map、OAuth flows、四类API注册表、credential store、catalog source/store/publisher和publication generation；:3469 起还有prepared route/attempt；:4205 定义ChatApi，:4361/:4507定义image/classifier handles，:4923定义ChatCall。

建议：保留一个 Models 公共入口，内部按“注册与冻结绑定、鉴权编排、目录刷新发布、调用生命周期”设置 owner。状态与提交点一起移动；不要按 impl 长度切片。现有 profile/协议分离应保留：provider_collection.rs:3 以 profile+Catalog 构造 ApiMappedProvider/既有wire route，providers/mod.rs:34 是数据profile，api_mapped.rs:165 实现Provider。typed ChatApi/ImageApi/ClassifierApi/DeferredApi已有真实适配器，不是虚设接口。

通用 ChatApi/Models 的资源接口使用 codex_resources 名称（models.rs:536、:4236、:4247）。应判定其通用生命周期意义，再决定局部重命名/放置，不应为统一外观抽象所有协议状态。

事实：Anthropic parse_frames（anthropic.rs:134）与OpenAI Chat parse_frames（openai_chat.rs:84）重复CRLF、注释、单空格、多data行拼接机制；Anthropic还保留event字段，Gemini gemini.rs:34是另一简化器。建议先用1.5 fixture证明相同词法部分，再抽私有SSE机制；保留各协议terminal、event与错误规则。这里不是“三个解析器完全重复”的结论。

### 自实现 WebSocket、Transport 与 Store

事实：ws.rs:22 的frame/io/pmd为私有实现；Codex ws_transport.rs:47将Driver绑定reqwest::Upgraded，:669构造Upgrade，:704创建私有Driver，并拥有池/continuation/cleanup。它已有生产调用，不能当未接通代码删除。

当前 HttpClient 是 reqwest wrapper（http.rs:210、:234），UpgradedWs公开reqwest::Upgraded（ws.rs:218）；这是宿主/协议表面，不应传进通用消息模型。建议保留“HTTP/Upgrade wire、私有frame/PMD/IO、Codex会话与cleanup”三个证明目标。选替代WS库前应让候选跑相同握手、PMD、close、partial IO、取消恢复、错误/资源fixture，先比较再决定；本次没有作替代库评估。

已有实际seam：CredentialStore（credential.rs:311），memory实现（mutation.rs:702），产品keychain实现（crates/opi-coding-agent/src/credential_store.rs:1212）；AwsSourceTransport/Reqwest（auth/aws/source_http.rs:102/:138）；AdcSourceTransport/Reqwest（auth/adc/native.rs:329/:385）；FakeSourceTransport（test_support.rs:630）。ModelsStore是同步文件owner（catalog/store.rs:1/:24），不是Store trait。R07应逐一登记真实/测试Adapter共用的契约；不能为画图给所有具体类添trait。本次未完整证明它们已共享同一contract harness。

### Telemetry

事实：telemetry.rs:34把subscriber/exporter/sampling/flush留给宿主；carrier冻结Span+Dispatch（:54/:73），每次poll/drop使用该上下文（:136/:149），Noop仍执行业务（:82）。Request显式可选携带（provider.rs:302）；Codex producer spawn使用context.instrument（openai_codex_responses.rs:876）；产品agent构造Request时填None（crates/opi-agent/src/agent_loop.rs:525）。

结论边界：已有carrier与传播，不能声称完整pi-telemetry语义等价。建议carrier/观测语义属于SDK，导出策略与全局初始化属于host，禁止反向依赖CLI；Pi计数、时机、字段仍需独立1.5映射。

## 已证实过时与需重新冻结的证据

| 项目 | 确切证据 | 处置建议 |
|---|---|---|
| WS/ADC pending说明 | README.md:2194称无生产Codex caller；tests/README.md:87–88称codec/JWT ADC caller pending；与ws_transport.rs:704、auth/adc/native.rs:912实际调用矛盾；README.md:1725本身也描述接通。lib.rs:25–27仍留首次消费者前allow注释 | update说明、审核allow必要性；retain私有/公开测试 |
| 旧Schema oracle/有意差异 | schema_public.rs:3–15明确Pi1.3.0；:453–469 root-correctness保留记录Pi字符串“2”，而Rust expected为Invalid/minimum | retain旧录制及regression，1.5实测后update分类；现状不能计严格等价，不能为全绿批量重录golden |
| backend数据差异 | scripts/pi-sdk-conformance/current-profile.json:4/:8明确两项URL改true，而历史Node为false；README.md:22–34明确差异和历史proof退休 | retain冲突证据，按1.5/固定环境重判；accepted difference不是严格通过 |
| 旧发布冻结 | platform_delivery.rs:234–280固定七成员/0.8.3/MSRV1.97，:368固定安全checksum | retain身份约束，随批准的新V update；workspace图归release owner，SDK消费不依赖仓库布局 |
| migration名称 | model_capabilities_migration.rs:14真实测public builder/default，:60测nested字段，:85测builtin cache capability | retain行为，必要时改职责名称；名字旧不是过时证据 |

## 测试重整方法与独立消费

1. 保留 tests/README.md:10 的层次。每case登记契约ID、原1.5版本/证据、输入/调度、入口、实际断言、L层、owner、add/update/delete/retain。展开用户目标快照的183份TS测试参数案例（旧验收文档为185份，基准迁移见上游证据），不能用Rust文件数或test数当覆盖率。
2. 去重比较入口、输入/受控时序、观察面、失效分支。全部相同才是候选；若保护不同入口、协议或任务边界应保留。纯fixture/peer helper放tests/support。只有所有原断言有明确接替和运行证据后才列delete。本次delete=none。
3. 遥测抽样确认不同证明目标：telemetry_carrier.rs:125/:140/:177/:205测冻结dispatch、一次poll、未poll的drop；telemetry_operations.rs:557经Models::stream验证组合传播/drop，:1258/:1457/:1587覆盖图像/分类/deferred；codex_telemetry.rs:192/:334–338验证真实producer、GET→POST与无telemetry序列化。三者应retain。
4. WS亦如此：src/ws/tests/io.rs:79测取消read恢复，:172测backpressure offset；tests/ws_upgrade.rs:256测真实握手、:388测重复header拒绝；Codex suite测池/续接与模型语义。不能按“都涉及WS”去重。
5. external_sdk.rs:1虽叫workspace-external matrix，实际是Cargo集成target，使用public API及工作区依赖/dev-deps（:19–40），有L5价值但不独自证明独立包/依赖图。现有 scripts/pi-sdk-conformance 已有真实package consumer（README.md:16、:35–46、:79–94），应扩展复用，不能写“没有外部消费测试”。
6. 新增完整外部contract consumer：提取实际.crate到fresh独立Cargo workspace/lockfile，禁止仓库私有路径与测试探针，覆盖核心+正式Faux、各方法族、compile-pass/fail、自定义协议ID+两个同协议provider注册/调用/替换（R06）、最小/默认/单provider/混合矩阵；归档artifact/lock/features/命令/退出码/hash。既有WS/schema/http/crypto/README示例的package harness保留；workspace发布图检查与SDK包API证明分开。
7. 真实调用者：opi-agent/Cargo.toml:13、opi-coding-agent/Cargo.toml:27直接依赖SDK；agent.rs:547持有Arc<Models>，agent_loop.rs:539调用prepare_call_with_policy；provider_factory.rs:172–190构造registry/Models，产品层还拥有keychain/OAuth/host policy。内部重构需保留真实入口组合测试，不应让调用者管理wire解析和锁。

## 顺序、未知与检查结果

推荐先冻结1.5 oracle/公共能力表、依赖与生命周期图；保留现有测试保护下整理domain和最小构建闭包；随后拆Models内部owner与局部机制；最后扩展实际包consumer/发布矩阵。不要先搬迁上千测试或先固定crate数。

未完成：全量AST公共符号/重载分母；目标快照183份测试逐case审计；Rust/TS执行；所有Store/Transport共享契约证明；全量重复检测；所有源码环跨逻辑职责判定；替代WS库行为/性能比较；全部1.3差异在1.5是否仍存在。

本次仅执行只读rg、文件读取、Python统计、git HEAD/status；没有构建和行为验证。test impact：add/update/delete=none，retain=现有全部（未来建议见上）。仓库修改=none。不得把缺口写成测试失败，也不得把静态调查写成验收通过。
