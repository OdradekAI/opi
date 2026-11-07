# 让普通 chat 入口独立执行
Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [迁移 agent 请求消费与显式中止](05-agent-execution-consumers.md)

## Scope

将 Models 的 stream/complete、simple、direct 六个普通 chat 入口改为在构造时
冻结输入并调度独立执行者。ChatCall 保留公共消费接口，start 取出同一个执行
句柄而不启动第二次请求，result 和显式 abort 复用执行者。构造要求 Tokio
runtime；实际 setup 在该 runtime 调度任务时执行，不保证在构造返回前完成。

原始 EventStream、ChatStream collector 和 advanced prepared attempt 的启动
合同不在本票更改；其他操作与 AI instrumentation、source.result 尾部对齐仍开放。

## Acceptance

- 六个普通入口的执行不依赖消费者 poll；丢弃未 poll 的调用或结果等待者不取消。
- 构造冻结路由/认证计划；已取消和配置失败仍是 typed outcome，不重复 dispatch。
- 部分消费、start、result、abort 共享同一个执行者和结果。
- 无 runtime 构造按文档失败；runtime 退出可观察 typed failure；跨线程消费有效。
- 保留显式 telemetry context、清理 cutoff 和 provider 资源清理的相关验证，
  用执行进度屏障替换旧消费 poll 的隐式同步假设。
- 双语文档、受测示例、测试归属、能力映射与实际检查记录同步更新。

## Delivery

六个普通 chat 入口已在构造时调度独立执行者，消费句柄 Drop 不取消请求；
start、result、显式 abort 共享同一执行者。双语文档、三个完整示例及
README 外部执行报告已迁移。详情见[默认入口迁移报告](../phase3-default.md)，
实际命令见[检查证据](../evidence/phase3-default-checks.json)，独立接入见
[实际包证据](../evidence/phase3-default-package.json)。

受影响的最终回归、Clippy、文档和外部包检查通过。全量运行中六项旧时长
假设已修正并通过所属 target 重跑；两项既有失败仍开放，未声明工作区全绿。
本票不包含 AI instrumentation、source/result 尾部或完整 Pi 等价验收。
