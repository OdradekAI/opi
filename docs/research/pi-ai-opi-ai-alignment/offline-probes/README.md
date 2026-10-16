# Schema／WS 离线研究资产

这些脚本和结果属于研究材料，不是 opi-ai 实现、产品测试、CI job、实施计划或验收账本。总判定见[主报告](../pi-1.3.0-schema-ws-offline-validation.md)；两组及其他五组覆盖缺口全部 OPEN。

- [Schema说明](schema/README.md)：真实 Pi 公开 utility 的85 cases／276 native observations；[固定来源](schema/source-pins.json)、[隔离锁图](schema/results/cargo-graph.json)、[语义结果](schema/summary.json)、[资产摘要](schema/artifact-list.json)。
- [WS说明](ws/ws-summary.md)：60参考构造器／15 native loopback／16 supplied-reader／15 partial-codec observations；[固定来源](ws/ws-source-pins.json)、[证据](ws/ws-evidence.json)、[资产摘要](ws/ws-durable-assets.json)。

原始结果保留阶段、错误与退出码，不能把运行器 exit0 解释为两端对齐通过；其中有 native 假成功、参考运行时异常退出及因缺依赖未执行的 public provider。仅 `from_partially_read` 不构成完整握手或PMD支持。没有供应商、OAuth或metadata请求。

## 安全重放与输入固定

先取得相同的既有 Pi source、TypeBox package／官方归档和 Cargo cache。不要从仓库根运行 Bun 默认导入，不运行 `provider-import-initial-diagnostic.mjs`；它只保全一次已废弃的未受控诊断。需要 Bun 的任何诊断都必须显式 `--no-install`。工作目录、输出与第三方提取内容应在新 TEMP scratch；不要把 vendor、binary、target 或生成输出写回本目录。

Schema 的 `run.py --scratch <TEMP>` 会复制自有输入、核对 runtime/source/fixture/lock hashes，并强制 `--offline --locked`；完整命令在该组说明。缺缓存／pin不匹配应明确失败，不自动下载／安装／切换版本。其 `pi_input_js` 和 `pi_normalized` 是参考观察，不是已实现的 Rust normalizer。

WS 先用 `prepare-ws-source.py` 在新 TEMP 解包校验过的 TW／TTW 归档，再用该组说明中的 Cargo 与 loopback 命令。该准备脚本只读取既有归档，不下载；其 path patch 指向完整原发布源码。重放前用 `Get-FileHash -Algorithm SHA256` 对照记录的 Node／Bun exe SHA，并核 `--version`／Bun `--revision`、rustc／Cargo版本；WS runner本身允许显式传入其他exe，换runtime的输出必须另存并标新实验，不能覆盖本轮结果。Cargo.lock固定版本／checksum，`cargo-features.txt`是该实验有效features，不能外推为产品图。

保留已有外部共享 Cargo target 和 incremental，不建会话 target，不清理缓存。harness timers／大header／长消息／regex subject长度只是fixture或实验containment，不产生SDK预算。正式受控运行仅离线／127.0.0.1；前置两个官方crate获取与已隔离的Bun自动解析副作用分别记录，不声称整个任务从未联网。

只在需要固定新输入／修复harness时新增一轮结果与来源；不覆盖既有raw记录。产品测试影响 **none**；研究harness与fixture／证据 **add**。本目录的Cargo manifest是隔离实验crate，不属于Opi workspace。
