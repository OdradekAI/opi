# Pi1.3.0文档修订：独立复审结论

> 2026-10-16；非规范性文档候选。输入为Opi c5ab42a及两个本地固定快照。完整逐项变化和清理见[upgrade review](pi-1.3.0-upgrade-review.md)，文件身份见[source evidence](pi-1.3.0-source-evidence.json)。本结论不授予正式规范准入或实施权限，不证明Rust已经等价。

## 当前结论

本轮文档修订完成。三路独立reviewer均未参与前置来源审计或候选写入；最终有限范围重读后无剩余必须修正项。67 AIR与59 V身份保留，AIR-060/V-52退役且不算通过；active SDK分别57/54项，host-impact分别9/4项。当前三份技术附件、双语spec、唯一矩阵和tracking已按1.3.0更新，过时现行要求退出而历史原字节可恢复。

Pi AI的47条变化路径逐项处置；telemetry仅2条metadata变化，原实现/两个测试/README不变。完整外部lock entry无增删改，版本pin不代表新runtime。两快照缺hydrated provider JSON，generator和测试源码不能证明本地目录记录、完整编译或服务接受。

Schema／WS／HTTP-H2-proxy-redirect／TLS-trust-crypto／auth-source-retry／native tracing／Cargo-platform-catalog共七组机制与证据缺口均继续 **OPEN**，详见[后端附件](pi-1.3.0-rust-backends.md#5-pi-130-后端覆盖缺口全部-open)。新arguments_error前置验证、可覆盖配置、typed Unsupported或SSE fallback均不能关闭对应接受集差异。

## 独立复审范围与结果

| Reviewer | 独立范围 | 最终结果及限制 |
|---|---|---|
| pi130_ai_independent_review | 全部16个改动current源码文件hunks、generator、25个新增/修改测试hunks、公开面/文档/manifest，以及必要同摘要消费者 | 三项P2与一项中文表格问题修订关闭；补充model.reasoning分支限定经再次独立核对。无剩余P1/P2；未重审所有未变外部SDK实现 |
| pi130_backend_independent_review | 完整依赖lock、Schema/WS及其余OPEN边界、native tracing与Pi callback DSL/settlement区别、历史恢复 | 无必须修正项；不将有条件选型或新增字段视为闭环。未重新推导全部继承AWS/ADC operation及native/transitive闭包 |
| pi130_docs_independent_review | 双语数值/编号/scope、当前契约残留、表格/链接/anchor、49变化与361继承摘要、Git历史字节恢复 | 无剩余必须修正项；最终复审报告本身由主审整合并机械验证，不伪记reviewer提前审过该报告 |

原始临时报告hash在[最终verification](pi-1.3.0-revision-verification.json)保全。实质发现保存在下表，不以TEMP路径作为唯一长期证据。

## 发现与修订闭环

| 发现 | 来源证据／影响 | 最终修正 |
|---|---|---|
| P2：frames backlog旧义务残留 | 初稿SDK仍有encoder offsets/catch-up行，指向1.3.0已删除源并引用退役V-52 | 删除残留行及frames节名；普通event-time partial/end/Result保留在V-40/41，不恢复compact API |
| P2：sign-in将defined误写truthy | openai-responses.ts:42–48实际apiKey!==undefined，空串＋authorization header与省略key＋header的分支不同 | SDK/升级表/双语正文写明含空串；V-16增加对应正反例。truthy仍仅用于native effort guard |
| P2：medium写成无条件固定值 | request builder先设medium，随后samplingParams与onPayload可覆盖；输出stamp来自变换前mapped effort | 双语/SDK/升级表/V-16明确构造基线和override/stamp分离，不新增override禁令；补充只有model.reasoning分支发该顶层字段，marker guard本身没有该条件 |
| 中文AIR-049句子越过表格末列 | 参数失败/raw文本义务初稿追加在最后pipe之后，可能不渲染 | 移入目标行为列，保持英中同义；静态检查增加每行closing separator，避免仅宽度检查漏报 |
| 来源整合精度 | Responses当前及重复effort marker不得优化去重；Anthropic off使用严格!==null；统计区分1.2.0与1.3.0 | 两版与附件精确限定simple off/旧路径、始终当前marker、包含未配置off；保留25 changed/new AI测试hunks与174文件数量的区别 |

## 实际验证及未证明部分

[最终verification](pi-1.3.0-revision-verification.json)记录精确命令、退出码、文件/报告摘要、Git各状态及结构检查。已运行 `python scripts/opi-doc-check.py` 与 `git diff --check`；另按固定清单重算1902/1925源文件摘要、比较完整lock对象、核对双语480行数值、AIR/V/RDF、表格/链接和历史恢复。文件数、链接通过和源码预期均不是产品测试结果。

本轮只写本目录文档。共享工作区另有其他任务的规范导航、研究／快照清理与runtime测试文件改动；其中研究清理形成独立提交c652192，经路径比较不含本轮目录。以上均未由本轮修改或stage；本轮文档无commit/push。测试影响 **none**：add/update/delete均0，未执行Pi/Rust产品测试、编译、差分runner、安装/import/hydration、live/OAuth或metadata/process fixtures。

仍未证明：Schema接受集/错误传播、WS不可配置限制的补救或重选、HTTP/TLS/crypto/source chain的实际平台conformance、native tracing poll/drop与跨Dispatch、完整Cargo/features/native/MSRV/三平台、hydrated catalog及供应商接受、性能/费用和正式准入。未来只能用对应V的实际证据改变OPEN状态，不能用版本号、文档可行性或后端名称关闭。
