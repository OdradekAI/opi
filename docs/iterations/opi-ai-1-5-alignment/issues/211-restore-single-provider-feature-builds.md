# #211 修复单独启用 provider feature 的构建

Status: done
Owner: provider/routes 与 stream 单测
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

仅开 `provider-openai-chat` 时，`OpenAiRouteContext` 引用的 `Arc`、`ModelInfo`、`WireApi` 的导入受 Gemini/Bedrock cfg 控制。#210 的只读探针以 E0425 退出 101。另一个需要覆盖的入口是 `stream.rs` 中在 common feature 下直接调用 Anthropic adapter 的单测。

## 验收

让 route 类型的定义和导入在同一组 feature 下可用。隔离测试的实际依赖。每个公开单 provider 配置都能独立构建，不通过启用额外 provider 掩盖缺失。

## 验证

复现并转绿 `cargo check --locked --offline -p opi-ai --lib --no-default-features --features provider-openai-chat`。同一配置跑 `cargo test -p opi-ai --lib --no-default-features --features provider-openai-chat`，保留取消分支的真实覆盖。对 Cargo.toml 中 11 个公开 provider feature 分别跑上述独立 lib check，并用现有包验证器复核受影响 owner。

Test impact: update 条件编译用例；retain 现有协议与取消断言。

## 实施与复核 · 2026-11-20

复现 OpenAI Chat 单 feature 的 lib test 构建时，编译器报告三处缺失的路由类型、五处 Anthropic 引用和一处 Responses profile 引用。根因分别在生产导入、使用真实 Anthropic mapper 的取消测试、catalog 测试最后一条 Responses 配置断言。

- `provider/routes.rs` 的 `Arc` 和 `ModelInfo` 随协议共用 feature 导入，`WireApi` 随实际使用它的 Gemini 或 OpenAI Chat 导入。
- `stream.rs` 的 silent producer 取消用例由 `provider-anthropic` 启用；普通取消测试仍在 OpenAI Chat 配置实际执行。`catalog/wire.rs` 中的附加断言由 `provider-openai-responses` 启用；旧 catalog codec 的主体在 minimum 下仍可测试。
- Cargo 的 provider feature 依赖表无改动。原声明的 11 个 owner 均在关闭默认 feature 后单独通过库检查。OpenAI Chat 单 feature 的 182 项 lib 测试通过；Anthropic 的 silent producer 用例和 Responses 的 legacy source 默认值用例分别在自己的 feature 下通过。

重跑库检查时，从 Cargo.toml 的 `features.protocol-adapters` 逐个取值给 `cargo check --locked --offline -p opi-ai --lib --no-default-features --features <owner>`。归档验收使用 `scripts/pi-sdk-conformance/run_provider_protocol_package.py`，传入新的外部 `--scratch`、已标记的 `--target-dir` 和新的 `--evidence-output`，省略 `--owner` 即覆盖全部 11 项。[逐配置与外部编译执行回执](../evidence/issue-211-single-provider-checks.json)

严格 Clippy 对 OpenAI Chat 配置仍报告 `models.rs:2749` 的 needless borrow 和 `models/prepared_call.rs:200` 的参数数量。默认配置另有 `api/openai_responses.rs:699` 的参数数量。这些诊断已见 #210，普通 Clippy 在相应目标完成。
