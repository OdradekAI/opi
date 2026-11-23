# #223 消除 OpenAI Chat 与同族 API 的命名歧义

Status: done
Owner: api/openai_completions.rs 与 Chat Completions 消费者
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Prerequisite: [#211](211-restore-single-provider-feature-builds.md) 已验收

OpenAI Chat、Responses、Codex Responses 与 Decisions 并列暴露，只有 Chat 使用整个品牌名。它的外部 import、静态检查和 feature 消费面明显大于 #210 的三项改名。

## 验收

Chat Completions 的公开入口是 `api::openai_completions`，OpenAI 服务商 profile 使用 `providers::openai`。wire ID 为 `openai-completions`，单独通过 feature `provider-openai-chat` 选择。移动前后 adapter 文件字节相同。

更新 Azure 的共享 SSE 导入、Rust 测试、双语指南、Unreleased、测试目标与归档 runner。活跃调用方旧接口引用为零。旧字符串在归档拒绝测试和 #104 冻结回放中提供检查输入。

## 验证

迁移阶段的默认配置合并运行 25 个 Rust 目标，共 793 项通过。Radius 兼容性复核后，9 个受影响目标的 247 项通过。`protocol-adapters` 下 12 个目标的 13 项通过，单 `provider-openai-chat` 的 profile 测试通过。

同一实际 `.crate` 的单 feature、`protocol-adapters` 和 Mistral + OpenAI Chat 消费者均编译运行。临时外部消费者使用旧导入得到缺少模块的编译诊断。默认及单 feature rustdoc、结构门禁与 77 项脚本回归通过。[验收回执](../evidence/issue-223-openai-completions-module.json)

Test impact: update 测试 import 与归档消费者；retain OpenAI Chat 的协议、共享消费和 profile 断言。
