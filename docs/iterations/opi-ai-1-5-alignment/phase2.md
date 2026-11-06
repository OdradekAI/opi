# 第二阶段首个切片：中立 telemetry 基础

本次交付独立的 `opi-telemetry`，并迁移 `opi-ai` 现有 tracing carrier。实施状态由[实施票](issues/02-neutral-telemetry-foundation.md)维护；迁移记录追加到[契约索引](contract-index.json)的 `implementation_slices`，第一阶段冻结的能力、源码和测试清单保持原样。

这完成了第二阶段的 telemetry 基础部分。纯值/通用错误、auth/catalog 合同与后端的分离仍待后续切片；AI instrumentation 和请求执行/消费生命周期属于后续执行层迁移。此处没有给出 Pi 1.5.0 严格等价、六门通过或发布结论。

## 边界与公开接口

`crates/opi-telemetry` 默认不启用任何 feature，普通依赖只有 serde，不依赖 async runtime、网络、其他 Opi crate 或隐式全局初始化。模块按责任分为 `context`、`backend`、`metrics`、`schema`、`noop`、`memory` 和可选 `tracing`。工作区为八个成员；没有新增外部锁定包或变更现有外部包版本/checksum。

- `TelemetryContext::start_span` 同步开始，返回唯一 `SpanOwner`；只有 owner 可以显式 `finish`，可克隆的 `TelemetrySpan` 只持有观察能力。
- 丢弃 context/观察句柄不结算；未结算就丢弃 owner 会留下未完成记录。迟到属性、事件、状态修改无效；已完成上下文不再接纳有效子 span，但保留 metrics 能力。
- 内存记录器返回独立快照，保留父关系、属性合并、事件/计量顺序、开始顺序和结算顺序。显式状态优先于自动错误状态。
- metrics 的工厂、记录及 span 完成各自同步返回后端错误，不统一吞错、重试或自动转成业务错误。schema 是可序列化描述，没有增加运行时校验或 AI 领域词汇。
- 可选 tracing 适配固定源 span 的 dispatcher，保留跨线程、poll 和 Drop 的传播。逻辑结算与原生 tracing close 分开；内部关联身份不复用原生 tracing ID，避免旧 context 的迟到 measurement 关联到新 span。

`instrument` 仍是惰性 future 包装：每次外层 poll 对应一次内层 poll，guard 不跨 await。丢弃包装会在捕获的作用域中丢弃 inner future，业务取消遵循该 future 的合同；它不会替调用者分离或后台启动业务执行，也不会替请求执行者结算 span。

`opi-ai::telemetry::{TelemetryContext, Instrumented}` 保留重导出，现有请求字段和高级消费者继续使用同一类型。`opi-ai` 目前启用新 crate 的 tracing feature；这不代表 AI 的最小 feature 配置已完成。

## 行为与文档迁移

按已确认的中立 no-op 决策，`noop()` 和 disabled span 不再安装静音 dispatcher，回调、poll 和 Drop 中宿主已有 tracing 保持可见。对应旧测试改为精确检查宿主 marker、业务调用次数、Pending、结果和父作用域恢复，其余断言保留，没有删测。该变化已列入 Unreleased Breaking Changes。

新 crate 提供双语 README 和完整离线示例 `examples/record_request.rs`；工作区双语入口和 opi-ai 双语 telemetry 说明同步。README 明确 crate 尚未发布、后端由宿主初始化、AI 自动 span/metrics 尚未接入。整个 opi-ai README 的任务式重构仍在后续阶段。

## 真实包接入

现有 conformance harness 现在同时打包 `opi-ai` 和 `opi-telemetry`。未发布的 telemetry 依赖仅通过候选 `.crate` 解压目录解析；检查归档字节、源码、配置和 Cargo metadata 身份，禁止把仓库源码路径当作候选包。没有依赖 agent、CLI 或仓库私有初始化。

- [telemetry 最小外部项目](evidence/phase2-telemetry-package-current.json)：关闭默认 feature，直接消费实际包，运行包内完整示例；依赖图没有 tracing、Tokio、网络库或其他 Opi crate。
- [SDK README 外部项目](evidence/phase2-readme-package-current.json)：运行实际包内三个 onboarding 程序，验证 lazy setup、未 poll、普通结果/流和工具续轮。
- 为避免改写历史证据，README runner 新增拒绝覆盖已有路径的 `--evidence-output`。

验证中发现 Windows 本地 peer 接受的 socket 可能继承非阻塞模式，导致请求字节到达前读失败。已先复现失败，再以显式 blocking 模式修复；延迟发送回归先红后绿，加入每次 README proof，固定后的实际消费者连续十次重放通过。它是验证工具修复，没有改变产品网络实现。首次失败记录保留在 [failed attempt](evidence/phase2-readme-package-final.json)，未用重跑结果覆盖。

## 检查结果及限制

完整命令、退出码、日志身份、失败片段及尝试分类见[检查记录](evidence/phase2-checks.json)。本机为 Windows x86_64，Rust/Cargo 1.99.1；声明 MSRV 1.97 未在本次执行，Linux/macOS 未验证。

| 检查 | 结果 |
| --- | --- |
| 新 crate 无默认 feature / all-features 合同 | 8 / 12 tests PASS，两个配置有重叠，不相加为独立覆盖数 |
| 现有 carrier、operations、Codex、platform 回归 | 32 tests PASS |
| 包验证 Python 回归 | 73 package + 13 README tests PASS |
| 工作区 Clippy（排除 opi-eval，all-targets，告警为错） | PASS |
| 工作区 doctest / rustdoc（排除 opi-eval） | 11 doctests PASS；rustdoc 告警为错 PASS |
| 最小 telemetry 与 SDK 实际包消费 | PASS，限定上述本机/配置/受控 peer |
| fmt、文档、diff 和索引冻结检查 | PASS |
| 扩展工作区回归 | FAIL，原因与补跑结果见下文；不能作为全量通过 |

扩展回归有三个需要后续跟踪的观察：

1. `schema_engine::encoded_pointer_targets_preserve_selected_resource_identity` 在当前工作树失败；从提交 `dbb5954b68a096ec6c9b7aed67eb11e7a9ac1081` 提取的未修改源码上，同一用例出现相同断言失败。该 schema 失败在本切片前已存在。
2. `spec_ledger::live_ledger_spec_hash_matches_current_spec` 读取 `docs/opi-spec.md` 失败。该测试未修改，所需文件在基准 Git 树和当前工作树都不存在；此结论来自源身份及缺文件核查，没有声称运行了基准的该用例。
3. Vertex 的 `adc_external::external_executable_streams_bearer` 在扩展执行中失败一次；同一候选二进制隔离重跑及基准源码隔离执行均通过。仍保留为未定位的间歇性失败，不将隔离通过当作原失败已修复。

原工作区命令因 schema 失败提前结束；另行补跑剩余 AI targets 和剩余工作区包。剩余包的 Cargo 执行只有上述 spec_ledger 失败。一次补跑编译被中止，后以 `-j 4` 完成；补充二进制执行器误给三个 `harness=false` 辅助程序传入 libtest 参数，相关退出被单列为执行器错误，正式 Cargo 补跑已正确执行这些辅助程序。所有尝试保留，不通过累加重跑次数或忽略失败得到“全绿”。

本切片通过了自身受影响合同和接入验收，但全量 gate 仍为 FAIL。没有运行真实服务、完整 TS/Rust 差分、全平台/完整 feature 矩阵或六门里程碑，也没有校准/冻结数值预算。现有 schema 失败、失效的 ledger 测试和 Vertex 间歇性失败须分别诊断，不能靠本次 telemetry 重构绕过。

## 下一步

继续第二阶段的纯值/通用错误与 auth/catalog 边界切片，沿既有消费者映射保持高级入口和行为。随后由执行层统一处理 AI schema/metrics 埋点、eager 启动、请求执行与停止消费、observer 异常的 TS/Rust 对照；本次基础提取不能替代这些证明。

实现与验证交接时尚未提交；用户随后授权本地提交。未推送或发布。两 crate 仍沿用工作区 0.8.3 的未发布候选身份；正式版本与发布顺序另行安排。
