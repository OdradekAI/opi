# 第五阶段补充：材料身份复核与有屏障的生命周期差分

日期：2026-11-08。对应[实施票](issues/36-supplemented-materials-and-gated-differential.md)。
工作树基点为 `3f2ff874af08a82108f2326281f579808b6dd25a`。
用户确认依赖、dist 和生成目录已补充后继续本切片。本次只修改验收工具、Rust
观察示例、测试和内部文档，不修改生产行为或 `.pi/pi-1.5.0` 内容，不安装依赖、
重新 hydrate、联网调用 provider、提交或发布。

> 本文件记录修复前候选。用户随后批准的[普通调用尾部修复](phase5-ordinary-source-tail.md)
> 已关闭下述两例 source EOF 缺口；旧 FAIL 证据保留。材料身份阻断尚未清除，当前
> 验收状态以修复报告及其新证据为准。

## 结论

工具补强已交付；**当前生命周期差分为 FAIL，完整材料身份仍有两个阻断项**。
“已安装”“内部校验通过”“与指定快照相同”是不同结论，不能合并成通过。

- [补充材料证据](evidence/phase5-supplemented-materials.json)
- [有屏障的差分证据](evidence/phase5-gated-lifecycle-differential.json)
- [检查记录](evidence/phase5-hardening-checks.json)

原[首次材料与差分报告](phase5-materials-and-differential.md)及 JSON 原始记录保留；
其中目录缺失是首次记录时的事实，不再是当前状态。初版探针对 source 尾部的证明
不足，由本报告的直接观察修正，不能继续沿用“三例无意外 mismatch”作为当前结论。

## 材料：哪些已确认，哪些仍受阻

### 安装依赖与 dist

从 AI 的实际 importer 解析全部十个直接 runtime dependency，安装版本和所选
lock record 均匹配第一阶段保存的声明与摘录。OpenAI 7.19.0 和
https-proxy-agent 9.1.0 位于 `packages/ai/node_modules`，不能只查看根 node_modules；
本地 schema/telemetry 链接也已核对。

AI/schema/telemetry 的 dist 分别记录 859/172/32 个文件的逐文件摘要；
204/43/8 个源文件均具有对应 JS、声明与 map，JS map 内嵌源与当前源一致。
这些证明实际产物存在、完整并具有可核对的内容身份，**不证明 JS 是由所附 map
可重复构建出来的，也不是上游 npm 实际包的外部消费验收**。

### 原始 lock 身份未恢复

2949 个冻结源文件仍全部存在，只有 `package-lock.json` 字节发生变化：

- 冻结：185524 bytes，SHA-256
  `802adbf0a5e093830226723f37b8971ce42eee442181a1471e0b70b1396b244a`。
- 当前：185020 bytes，SHA-256
  `79490cd0acd238e0ad915d96c77bfddb5a07bf73fefcadc46cc13ec25785c721`。

直接依赖摘录相同不能证明整个传递依赖解析相同。没有取得原始完整 lock bytes，
因此保留 `frozen upstream source/lock provenance drift` 阻断，未覆盖旧基准。

### 生成模型数据不是 pinned 数据

保存的原始响应仍是 940702 bytes，SHA-256 与 pin
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e` 一致。

当前 source/dist 模型目录逐字一致，两者都通过上游内部校验。但使用上游纯
`groupProviderModelData` 对该 pinned 响应投影并逐模型比较，得到：

- pinned 1712 项；当前 1738 项；均覆盖 42 个 provider。
- 9 个 provider 不同：baseten、fireworks、nvidia、opencode、opencode-go、
  openrouter、radius、together、vercel-ai-gateway。
- 按 provider/API/type/id 区分：缺失 20 项、额外 46 项、共同身份中内容变化 63 项。

上游内部 checker 校验当前数据与当前 manifest 的一致性，不校验是否来自该 pin。
因此 `generated catalog differs from pinned response` 仍受阻；本次不替换用户补充
的目录。下一步应在可追溯工作副本中从已验证的原始响应 hydrate，再对 source/dist
进行同一比较，不能用当前线上数据或 Rust 内嵌目录替代。

## 差分：观察方法与真实失败

Rust source 在发出 Start 后进入明确的 oneshot 屏障，直到消费动作结束才允许产生
终态。记录构造时 setup 次数、消费前后的 source 状态、终态次数、source EOF、
source Drop，并从真正的 `ChatCall::result()` 取得 stop。保留结果句柄的场景与
直接 Drop 消费句柄的两个 Rust 测试分别验证，不虚构已经丢弃的公共 result。

仍使用未修改的上游源码探针；每个 TS 场景运行三次并比较原始观察一致性。
runner 保存命令、stdout/stderr、轨迹摘要、编译器报告的 Rust 可执行文件及其摘要，
执行前后复核材料。driver 超时与 Cargo 编译耗时分离；构建使用原有共享 target
和 cache lease，不清理缓存。

| 场景 | 实际观察 | 当前判定 |
| --- | --- | --- |
| 不消费 | 两侧均先启动、屏障放行后产生一个 stop；Pi 执行 source 尾部，Rust 释放 source 但未 poll 到 EOF | **FAIL**：`source_completed` 不同 |
| 消费 Start 后停止 | 放行前确认 source 尚无终态；后续 stop 与结果均真实观察到，但 source 尾部仍不同 | **FAIL**：`source_completed` 不同 |
| setup failure | 两侧都是一个 error；Rust event 与 result 相同 | **difference**：Pi rejected setup 与 Rust 未注册 route 的失败输入、同步 throw 与 typed async 错误通道不同，未批准 |

失败定位于 `crates/opi-ai/src/models.rs` 的
`drive_resolved_call_registered`：`stream::unfold` 在业务终态后设置 `finished`，
下一次 poll 直接 `return None`，没有继续 poll `current` attempt。外层
`ChatCallDriver::result`、ProjectionStream 和 AttemptStream 虽有尾部处理，仍被
这一层截断。初次加强示例时的 EOF 断言在两例都失败；随后改为输出真实 EOF/Drop
观察，由比较器保留 **FAIL**，不是改低 expected。构造时的同步 setup 与 Tokio
调度差异也继续保留在原始步骤中。

此切片不顺带修改生产生命周期，也未批准该差异。下一项产品修复应针对这个普通
调用 wrapper，保留首终态吸收和 retry/admission 语义，同时验证尾部、副作用、
取消与资源结算；先运行本差分作为红灯，再补/运行受影响的产品回归。

## 检查与范围

检查命令、原始日志身份和结果见[检查记录](evidence/phase5-hardening-checks.json)。
选用 worktree-only 检查：只验证本轮刻意修改的工具、示例和文档；不把既有 88 个
领先提交当作本轮审计范围。

- Python 材料测试 9 项、Node inspector 测试 4 项。
- Python 差分工具测试 13 项，包括三个实际 Rust 场景和未知场景 CLI 负例。
- Rust 示例内两个消费 Drop 测试。
- 受影响 example 的 Clippy（warnings denied）、workspace fmt check、文档检查、
  JSON/索引解析与 diff 空白检查。
- 实际差分命令退出 **1 / FAIL**，材料清单退出 0 只表示成功记录，两个阻断仍保留。

测试影响：`add` 新 inspector/runner 负例、材料漂移检查与两个 Drop 测试；
`update` 原 freezer/comparator/driver 测试；`retain` 全部产品测试和历史证据；
`delete` none。内部报告无既存英文镜像；公共双语手册不变，因为未改变产品合同。

本次没有运行完整 Cargo workspace、完整 provider 差分、feature/platform/MSRV、
性能/耐久预算或真实服务。后续顺序：修复已复现的普通 source 尾部截断；恢复原始
lock 身份并准备匹配 pin 的工作副本；随后继续真正的最小 feature 构建和按契约扩展
差分。恢复材料与不依赖这些材料的确定性产品修复可以独立推进。
