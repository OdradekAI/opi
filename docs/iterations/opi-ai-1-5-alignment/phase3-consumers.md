# 第三阶段第二个切片：agent 消费者与显式中止

日期：2026-11-07。实施票：[迁移 agent 请求消费与显式中止](issues/05-agent-execution-consumers.md)。
本切片已完成，相关检查及已知失败记录如下；第三阶段及完整生命周期对齐尚未完成。

## 本轮实现

agent 主 turn/retry 和 preparation summary 两个生产调用点已改为消费独立启动的
`StartedChatStream`。丢弃 agent 操作 future 后，已经启动的模型调用继续执行；
没有把后续响应写入 agent 状态，也没有替丢弃的 run 伪造完成或证据结算。

`StartedChatStream::abort(&mut self).await` 是显式执行中止：释放积压，向 Tokio
请求中止，等待执行任务持有的源释放。任务已完成或在中止生效前完成时保留真实
outcome；中止未完成任务时返回 Cancelled，保留执行者已观察的最新 partial。
源 panic 继续传播，原 request token 不被隐式取消。`abort` 等待被丢弃时，句柄
仍持有任务和中止状态，后续 abort/result/迭代仍能观察结束。

agent 收到 caller token 取消后，保留原取消优先策略，等待执行者退出再返回
Cancelled。准备流程持有已启动 summary 的消费句柄：外层 select 丢弃 hook 时，
仍可取回并中止执行者；初始准备、续轮准备、reborrow 与 commit 结算共用这个所有权。
commit 继续遵循原来开始后等待确认的合同，没有加新的可中断提交点。

这里的确认范围是执行任务及它持有的源。provider 独立任务、外部请求效果、池和
会话连接仍遵循原清理合同，不能用 `abort().await` 代替
`Models::cleanup_session_resources`。Tokio 不能抢占不让出执行权的同步 poll；
强制中止也不能获得 provider 未发事件的内部 partial。已 dispatch 却未观察到
终态的 prepared attempt 仍记录 UnknownDispatch；不会伪装成已观察到取消或成功。

## 检查与证据

完整命令、日志身份和结果见[检查记录](evidence/phase3-consumers-checks.json)。
本轮先以失败测试锁定消费者 Drop 释放源，再实现独立执行；随后测试复现外层
preparation 取消抢先丢弃 summary 等待者的问题。借用式 abort 保留等待期间的
JoinHandle，避免内层中止等待再次被取消时丢失 join 所有权。

| 检查 | 实际结果与范围 |
| --- | --- |
| 执行与 agent 定向回归 | 109 项通过；包括执行生命周期、消费者、主循环、准备事务、重试、续轮和 wrapper。随后修正的两项测试另行复验 |
| 修正后目标复验 | 24 项通过：execution_lifecycle、tool_result_wire、request_execution；与上行重叠，不相加统计覆盖 |
| 工作区 Clippy | 最终源码通过，`--all-targets -- -D warnings` |
| 工作区文档示例 | 11 项通过，`--doc` |
| 工作区 rustdoc | 最终源码通过，`--no-deps`，`RUSTDOCFLAGS=-D warnings` |
| 工作区全部 target 回归 | FAIL：原运行四个目标失败，其中两项修正后定向复验通过，两项既有失败仍开放，详见下文 |

工作区命令均排除 opi-eval，采用离线依赖和 Windows / Rust 1.99.1 环境。
未运行其他平台、声明的 MSRV、真实服务、当前候选包独立消费或性能/内存预算验收。

工作区全量命令使用 `--workspace --exclude opi-eval --all-targets --no-fail-fast`，
没有跳过失败目标，约 18 分 44 秒后完成并返回 FAIL，共四个失败目标：

- 新增 abort 结果测试重新构造期望消息时采样了第二个时间戳，差 1 ms 导致失败。
  修正为单次构造输入和期望，不修改生产逻辑；同时用 Timeout 失败验证中止不会
  覆盖已完成错误。当前线程 Tokio 的调度假设不能代替稳定的输入值。
- `tool_result_wire` 的直接模块声明清单遗漏上一阶段已批准的基础模块移动，
  本轮更新清单；没有删除 guard 或工具结果协议断言。
- `schema_engine::encoded_pointer_targets_preserve_selected_resource_identity`
  仍是 `Ok(false)` 对 `Ok(true)` 的既有失败。schema 实现和该测试与先前
  `dbb5954` 基准一致；先前实际基准失败证据见[第二阶段报告](phase2.md)。
- `spec_ledger::live_ledger_spec_hash_matches_current_spec` 仍因缺少
  `docs/opi-spec.md` 失败。测试未改；基准 Git 树和当前工作树均缺少该文件。
  这里复用的是基准静态核查，没有声称重新执行基准用例。

修复前两项后，`execution_lifecycle` 20 项、`tool_result_wire` 2 项和 agent
`request_execution` 2 项全部复验通过。全量原始 FAIL 记录保留；其他目标的
运行时实现没有因这两项测试修复改变，因此没有重复整轮运行。运行期间的
其他源码文字变动仅澄清 rustdoc 的 token 取消与 abort 边界；最终 Clippy 和
文档构建另按最新源码检查。后两项既有失败仍未解决。
Vertex executable 用例本轮通过，但不据此关闭其先前未定位的间歇性失败。

测试影响：add 为 5 个 AI 执行案例和 2 个 agent 贯通案例；后者各参数化覆盖主
turn、初始 summary、续轮 summary（包含嵌套 reborrow）。update 为上一阶段遗漏的
`tool_result_wire` 直接模块声明清单：四个旧模块改为重导出，新增 types/errors
声明；工具结果 wire 行为断言和其余结构限制保留。retain 为其他原有案例及
断言；delete 为 none。确定性通知、限时等待和源 Drop 屏障验证实际资源时序，
没有为验证添加公共状态探针。

## 剩余范围

旧普通 ChatCall/ChatStream 仍按手动 poll 推进，raw EventStream 和内置 provider
底层 receiver-closed 合同未改。旧入口无 Tokio 手动 poll 和跨线程行为仍是明确
存在的迁移输入；本轮没有把它们默认为永久保留差异，也没有将其判定为 Pi eager
启动等价。默认入口迁移需同时明确构造、启动、runtime 和同步失败时点。

AI schema/instrumentation、first-terminal 后的 source/result、业务与遥测分别
结算、observer 故障通道继续开放，以上游[实际观测](evidence/phase3-ai-telemetry-observations.json)
为约束。队列/性能预算、配置、完整 README、实际包及六门验收尚未完成。

工作树基点仍为 `814c95f7ce575fb9be3df6b9d04ff6e2942706c1`，本轮开始时的 80 个
未提交路径保留。第一阶段及前序切片的证据对象不改写；新能力/消费者/测试关系
只追加实施映射。未提交、推送或发布。
