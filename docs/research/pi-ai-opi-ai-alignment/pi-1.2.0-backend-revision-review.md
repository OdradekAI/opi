# Rust 工程合同与后端修订：独立复审结论

> **本轮参数审计后的适用限制：** 本页保留原输入的审查历史；“五项已补齐/已选型/无剩余must-fix”不再适用于未经来源校准的固定值和新增拒绝。其结论已由[参数来源审计](pi-1.2.0-parameter-audit.md)及[新独立复审](pi-1.2.0-parameter-revision-review.md)收窄。旧JSON hash/verification只证明旧输入；不能作为本轮默认值的核准。

> 2026-10-16，仅文档。输入为 [backend revision baseline](pi-1.2.0-backend-revision-baseline.json) 固定的 `6b816580f7500bbeca6b2bf32ec04737edc3a1e9` 与携入的技术审查。最终目标由双语 spec、[后端合同](pi-1.2.0-rust-backends.md)、SDK/RDF 附件和唯一验收矩阵共同表达。本页不授予实施或正式规范准入，不是运行测试报告。

## 结论

**五项设计合同已补齐，Rust 后端已完成源级选型；三路独立复审及修后重读无剩余必须修正项。** 67 AIR（58 SDK＋9宿主）、59 V（55 SDK＋4宿主）与20 RDF的分类保持。全部运行验收仍未实施/未执行；精确依赖版本可定位不等于Cargo闭包已解析、接口已编译、三个平台通过或服务接受。

本轮没有正式规范取舍要立即决定：没有变更 `docs/opi-spec.md`、Agent Core admission或宿主权限。新能力、尤其单image adapter的real-seam/placement证据，仍须后续正式shaping独立核准；不能以本报告或Pi parity豁免PRIN-002、§8.1、GATE-002。若届时需要改规范或放弃已定公开能力，再提交具体取舍，不预先请求空白批准。

## 已修订的可观察合同

| 主题 | 最终合同与未来证明 |
|---|---|
| 内存与终态 | 64 progress events加64MiB logical charge，frame/message/retained-state分别有界；独立state-budget＋256KiB terminal slot交付完整partial，不受满队列或小queue override阻塞；known producer usage保留；V-02/04/05/25/40/41/52 |
| 凭据completion | 对象安全owned mutation_barrier，同store跨Models的cutoff同步捕获；receipt按provider/source-generation/resource-key/revision保留，未确认不得被另一key成功覆盖；容量满先拒新effect；wait30s、九OAuth总15s与ADC60s区分；V-12/42/43/55 |
| 传输身份 | library-owned retry:never/redirect:none/no_proxy，受控scoped proxy/TLS；仅有界同origin ADC subject GET手工redirect；custom transport不二次包，真实双origin/proxy canary与实际send计数；V-01/05/09/29/30/36/49/51/55/58 |
| Schema | jsonschema0.28.3固定dialect/format/static resources/work/cache/jobs；明确AnnotationsOnly；独立fancy-regex guard阻止内部吞错fail-open，同canonical pattern/subject覆盖normalization每个candidate，碰撞pattern keys保留allOf约束；V-11/34/35/39/58 |
| Custom API | closed operation/open ApiId，RegisteredChatApi<O>与opaque typed options/fixed-O dyn adapters、namespace/type/revision校验；完整frozen record直达adapter、四入口/单driver保持；V-01/21/39/42/49/58 |

## 最终后端与被否决候选

| 层 | 选定方案 |
|---|---|
| HTTP/TLS | 现reqwest0.12.28、rustls0.23.40/ring0.17.14，显式instance ClientConfig/roots/H1/proxy，构造零env/I/O，配置后pool复用 |
| WS | tokio-tungstenite/tungstenite0.28.0 provided-stream；tokio-rustls0.26.4、httparse1.10.1；SDK窄CONNECT dialer、显式frame/write/close预算，无自动proxy/reconnect |
| AWS | owned full来源与STS/SSO/OIDC/SignIn协议，官方aws-smithy-query/xml 0.62.1加xmlparser 0.13.5完整结构预检；现owned SigV4＋hmac 0.12.1／sha2 0.10.9；p256 0.13.2做DPoP、sha1 0.10.6仅SSO文件名。完整来源/MFA/SSO/login/cache写回保留 |
| ADC | 私有有界五JSON家族与external各source/metadata adapter，受控reqwest、ring RSA与rustls-pki-types1.14.1；无gcloud/默认Google client，executable真实native机制但双显式gate；source/crypto不是空trait占位 |
| Schema | jsonschema0.28.3＋fancy-regex0.14.0显式guard；有限静态资源解析，不引JS runtime |

前一AWS“官方认证clients＋runtime guards”方案在复审中被否决并撤销：默认resolver在override前已有环境/文件副作用，Smithy还可能trace认证body。另查到aws-sigv4 HTTP signer的trace/Debug可触发LOG_SIGNABLE_BODY；最终也不调用该路径。采用纯codec/owned协议后不再依靠内部空trace header哨兵、全局日志关闭或真实env修改维持隔离。原下载源码保留为反证/协议oracle，不能从source-evidence列有某包推导它是运行依赖。

owned协议实现的维护成本明确：版本跟进必须核官方operation schema/error/endpoint与全部来源语料；不能只改版本号。源级选型与新增直接依赖意图见 [source evidence](pi-1.2.0-backend-source-evidence.json)；38个官方发布archive及8个锁定本地source文件有独立复算摘要。所有源码只读，没有安装/import/执行。

## 独立复审与问题闭环

选型研究者为 `select_aws_backend`、`select_adc_backend`、`select_ws_backend`；以下reviewer均未参与其设计或仓库写入，主审根据源码纠正，不以“建议”冒充人类逐项批准。

| Reviewer | 独立范围 | 修后结论 |
|---|---|---|
| independent_backend_contract_review | exact backend/public API、默认env/fs/日志、AWS/ADC/WS协议与source hashes | B-01–04全部关闭，无剩余必须修正项；只证明文档/API选择，不证明Cargo/运行 |
| independent_five_contract_review | bytes/terminal、store cutoff/receipt、Schema异常、typed custom接口与单driver | 原3项P2、15s预算及RDF同步项全部关闭，无剩余must-fix |
| independent_backend_bilingual_review | 双语/数字/公开动作、AIR/V/RDF分类、正式权威、历史/新选型、测试迁移与实际none | 原3项P2及format/MSRV导航残留关闭；未发现双语或权限范围漂移 |

原始独立报告在各自TEMP文件；其全部实质发现、修正与最终结果集中保存如下，不把TEMP路径当长期唯一证据：

| 合并问题 | 级别 | 修正与对应判据 |
|---|---|---|
| BR-01 官方client eager partition env/fs | P2 | 撤销服务runtime/default resolver，选owned协议与纯codec；V-09/39/49/55/58实际source隔离 |
| BR-02 vendor认证body/HTTP signer日志 | P2 | 不运行Smithy orchestration或aws-sigv4 HTTP signer，保留现成熟HMAC/SHA与owned canonicalizer；TRACE/LOG_*不产生隐藏auth日志 |
| BR-03 engine regex错误内部吞掉 | P2 | 每candidate/final的Result guard、同canonical与潜在subject、work计量，key碰撞allOf保约束；V-11/35/58 |
| BR-04 完整partial超queue无法交付终态 | P2 | 单独有限terminal slot/cache，明确drain/drop/cancel与唯一交付；V-04/40/41/52小queue/大partial/超限 |
| BR-05 per-provider latest receipt遗漏其他key | P2 | provider/source-generation/resource-key/revision保留unknown，256key满额先拒新effect、同keyreconcile；V-43/55双key ACK未知+另一成功 |
| BR-06 15s仅网络与ownership→ACK口径冲突 | P2（分项亦标P3） | Pi事实与Rust总事务预算分开；V-43/55加入refresh10s＋ACK6s越15s、known/pending/fence与不伪造rollback |
| BR-07 全构造format=true与AnnotationsOnly | P2 | 默认assertion未知拒绝，显式AnnotationsOnly对known/unknown都仅注释；V-35/58同schema/instance两模式正反例 |
| BR-08 SignIn错误要求response account | P3 | response只有三项AWS凭据；account沿冻结cache/session ARN provenance保留，不能虚构字段 |
| BR-09 选型/导航/精度残留 | P3 | V-09改“已选择，未解析/编译/运行”；README技术附件/本轮报告入口更新，未知MSRV不冒称低于1.97；RDF单progress event与terminal slot分清 |

复审没有要求删除能力或修改当前正式规范。HTTP安全默认、内部codec与普通状态机选择属于本次已授权文档设计；没有把后续G-01证据缺口静默关闭。

## 实际验证与剩余证明

文档检查和静态结构校验由本轮 [verification](pi-1.2.0-backend-revision-verification.json)记录最终命令、退出码和文件摘要；没有沿用前轮冻结verification冒充新稿证明。范围包含AIR/V/RDF唯一性与分类、双语新增段数字/动作、当前local links/line ranges、38 archive摘要、UTF-8/LF/whitespace和仅研究docs的工作区差异。README最终导航由主审在review后收尾，不改变被审合同。

测试影响为 **none**：未增加/更新/删除测试，未执行Rust/Node runtime、compile、live、OAuth、metadata、进程fixture或差分runner。未来测试按既有41+7索引与补充判据add/update，真实有效行为retain，只有替代证明后delete；没有自动运行Pi credentials-aware工作流。

未证明：Cargo解析/完整transitive/features/MSRV/native闭包；所有目标接口的编译/对象安全运行；Windows/macOS/Linux；proxy/TLS/进程树/持久ACK/签名/evil XML/regex/内存性能的执行结果；供应商接受、账户注册、费用/退款；正式能力准入。只要这些仍缺，不能称SDK已经与Pi1.2.0等价或本候选已经准入实施。
