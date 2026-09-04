# opi-eval

> 未发布的 Independent Companion，用于 Agent 中立的跨 Agent 评测。

[English](README.md) | [opi workspace](../../README.zh.md)

`opi-eval` 是 Agent 中立的 workspace 成员，用于跨 Agent 评测实验：在任何
Agent 进程启动前，将规范的、按摘要寻址的实验契约（N 个 harness subject、
有向 baseline/candidate 对比边、完全显式的共享模型控制、环境身份和声明的
trial）冻结。解析过程 fail-closed；不存在隐式控制默认值，也没有回退。

## Independent Companion 边界

本 crate 为 `publish = false`，且在任何依赖表（normal、dev、build、
optional、target-specific）中都不依赖 Opi crate。没有任何 Opi 产品链接它，
它也不在 `opi` 中注册 provider、tool、package、command、extension、启动
hook 或默认捕获路径。普通 `opi` 运行时行为不因它的存在而改变；现有的
session、原生 evidence、本地 Eval 报告、配置、凭据和用户工件永远不会被本
crate 读取、改写或迁移。

## 稳定性

本 crate 尚未发布，仍是 `0.x` workspace 成员。其 CLI、schema、磁盘格式和
库入口可以被有意调整；破坏性变更记录在 `CHANGELOG.md` 的 Unreleased 部分。
旧版实验或报告 schema 不隐含任何兼容层。

## 用法

```sh
cargo run -p opi-eval -- validate --config crates/opi-eval/tests/fixtures/experiment/minimal.toml
```

`validate` 解析实验文档并输出一行摘要（实验 id、schema、规范 manifest
摘要，以及 subject、edge、trial 数量）。无效文档以退出码 1 失败，并在
stderr 上给出类型化诊断。

当前 crate 版本：`0.8.1`，继承自 workspace package 版本。
