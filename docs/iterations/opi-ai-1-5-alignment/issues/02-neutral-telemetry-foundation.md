# 建立中立遥测基础与可选 tracing 适配
Status: done
Owner: Codex for user
Blocked by: [固定基准并建立能力、消费者与测试映射](01-baseline-and-mapping.md)

## Scope

承接用户“继续推进”，实施[方案](../spec.md)第二阶段的首个可独立验证切片。
工作树基点为 dbb5954b68a096ec6c9b7aed67eb11e7a9ac1081，开始时干净；
它只增加第一阶段文档，产品基准仍是第一阶段冻结的 90f953d。
不改写冻结 manifest，也不将文档提交导致的 HEAD 差异当作产品漂移。

新增 opi-telemetry：后端中立的 context/span、属性/事件/状态、同步 metrics、
schema 描述、no-op 和内存记录器；将已有 tracing carrier 移到可选适配。
opi-ai 依赖新 crate，并保留有真实调用者的旧路径重导出。
中立 no-op 不再静音无关宿主 tracing，按已确认决策更新对应断言。

本票不改变 AI 请求启动、停止消费/取消、observer 异常或 token/cost 计量。
Pi callback/promise 与 Rust 执行者的完整映射仍由执行层切片举证；
此处同步开始、显式结算不是完整等价声明。auth/catalog、纯值/通用错误
整理是第二阶段后续切片，不将本票完成写成整个第二阶段完成。

## Acceptance

- 中立公开合同不要求 tracing；默认 crate 不包含网络、AI 或宿主初始化依赖。
- 同步 begin，独立结算权；克隆/丢弃观察句柄不结算；迟到修改无效。
- 记录器保留父关系、属性合并、事件/计量顺序和脱离内部状态的快照。
- metrics factory/record/end 的后端失败可分别观察，不统一吞错；schema 不新增运行时验证。
- no-op 下业务结果、Pending 及宿主 tracing 可见性保留；可选 tracing 保留冻结 dispatch 的 poll/drop/thread 传播。
- 旧 carrier 调用者、相关 AI 操作回归通过；中英文当前文档同步。
- 无默认 feature 的普通外部 Rust 项目可消费实际打包的新 crate。

## Checks planned

新 crate 外部合同测试、tracing feature 测试、现有 telemetry carrier/operations/Codex 回归；
受影响 clippy、fmt、doctest/rustdoc、依赖图与外部包消费；仓库文档和 diff 检查。
交叉 crate 表面只保留类型重导出，按实际改动扩展必要 workspace 编译/回归。

## Test impact

add：中立 crate 公共合同与包消费验证。update：显式 no-op 的旧静音断言。
retain：其余现有断言与第一阶段原始证据。delete：none。

## 交付与检查

已交付新 crate、可选 tracing 适配、旧路径重导出、双语文档、完整示例和实际包消费验证。
具体合同、变更和下一步见[切片报告](../phase2.md)，完整命令与失败记录见
[检查记录](../evidence/phase2-checks.json)。第一阶段原有索引保持冻结，迁移信息仅追加。

- 新 crate 的无默认 feature / all-features 合同分别 8 / 12 tests PASS；既有 AI 遥测与交付回归 32 tests PASS。
- 实际 telemetry 最小外部消费和 SDK 三条 README 接入流程 PASS；Python package/README 回归 73 + 13 tests PASS。
- 工作区 Clippy、11 个 doctest、rustdoc、fmt、文档与 diff 检查 PASS。新 tracing ID 重用反例已加入适配测试。
- README peer 在 Windows 的非阻塞 socket 继承问题先复现、再修复；具名延迟发送回归先红后绿，固定后的实际消费者十次重放 PASS。
- 全量 gate 仍为 FAIL：schema 用例在未修改基准复现；spec_ledger 所需文件在基准已不存在；一次 Vertex 失败的隔离候选/基准重跑通过，但原因仍未定位。原始失败未覆盖。

done 仅关闭本票受影响合同与独立接入验收，不代表整个第二阶段或 Pi 1.5.0 对齐完成。
未验证 MSRV 1.97、其他 OS、真实服务、完整 TS/Rust 差分或六门里程碑。
实现与验证交接时尚未提交；用户随后授权本地提交。未推送或发布。下一切片为纯值/错误、auth/catalog 合同与后端的边界整理。
