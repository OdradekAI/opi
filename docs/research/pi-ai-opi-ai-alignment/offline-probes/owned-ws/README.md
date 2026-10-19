# 自有 WS：已执行隔离研究的固定摘要

[summary.json](summary.json)封存先前已完成的 Windows/Rust1.99.1、内存/Tokio duplex 原型；[独立复审](independent-review.md)保留其来源、最终 receipts 和局限。不是本次 plan 的新运行、产品实现或验收。

最终76个测试函数在 base/defaultunion/C-zlibunion 各跑一次，共228次通过；同一72,413输入在三图重放零差异，不是217,239个不同输入。5个错误实现变体均被运行时断言识别；旧实现2个行为失败和非Sync/allocator互斥2个预期编译失败单列。中间轮最初Clippy失败已修正、辅助harness/验证器设置错误与compare额外留档重跑均在原窗口明确记录。

根 manifest 列出的269个旧文件在本次封存前逐个重算SHA一致；本资产保留该完整文件清单及外部原始目录，不复制产品源码、不改历史原始结果。三图Rust源码/manifests一致；29个registry archives/2,545成员已对照，独立审查另核3个关键包131成员。不能据此声称产品Cargo图、所有平台/MSRV或供应商有效。

依赖语义、Send/Close/取消/调度的当前交付义务见[WS合同§8](../../pi-1.3.0-readiness-ws.md#8-自有中立-ws-模块合同rdf-24)与[RDF-24](../../pi-1.3.0-rust-differences.md#rdf-24-owned-ws-profile)。只有主英中合同和登记章节提供规范性含义；本摘要不进入spec_files、不生成第二矩阵/ledger。技术资产此前没有英文counterpart，英中主合同共同引用。

测试影响：本次计划产品none，历史研究add/update/retain；旧Ratchet patch/wire fixtures retain。无删除产品测试、无网络/provider/socket、无构建/测试/fetch或缓存清理；本轮独立审查误调用1次只读cargo metadata --no-deps，未改跟踪文件，不作产品证据。
