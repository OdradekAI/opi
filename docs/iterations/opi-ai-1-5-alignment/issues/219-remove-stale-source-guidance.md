# #219 清除生产代码和当前指南中的过期来源指引

Status: done
Owner: opi-ai rustdoc、双语 provider 文档与文档检查
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

以 .repo/pi-1.3.0、Pinned TypeScript reference、已删除的根模块链接为入口。Bedrock reasoning 的能力声明要与实际事件解析核对。

## 验收

生产注释说明当前契约、时序和失败；比较使用的版本/修订放入迭代证据。修复已确认的错误路径和 provider-auth 双语能力说明。卫生检查仅覆盖活跃源码/指南，给合法历史材料保留位置。

## 验证

旧引用扫描和检查器正反例（过期源码注释报错、历史 evidence 合法）；`python scripts/opi-doc-check.py`、受影响 feature 下 rustdoc broken-intra-doc-links 检查，并选跑 `bedrock_fixtures` 中 reasoning 用例验证文档声明。

Test impact: add 卫生检查样例；retain 已有 Bedrock 行为测试。

[#215](215-own-integration-test-targets.md) 已修复 minimum 归档 rustdoc 实际报出的 13 个 `Models`、`Provider`、`ChatStream` 路径链接。此票继续处理版本指引、其余 feature 的文档和 Bedrock 能力表。

## 实施与复核 · 2026-11-20

OAuth、replay、transcript、identity 等源码注释改为说明当前入口、输入规则和失败阶段。直接核对 Pi 1.5.0 快照的目录数据，Cloudflare classifier、TypeSafe、Decisions、OpenRouter Images 和 Radius 分别有 5、1、1、66、34 项。对应注释改为描述 Opi 返回的 fallback 和调用方的覆盖入口。Radius 的动态刷新接受条件留在 [#222](222-refresh-radius-builtin-catalog.md)，静态目录比较留在 [#221](221-reconcile-pi-1-5-catalog.md)。

双语 provider-auth 现在记录 Bedrock ConverseStream 的 reasoning 行为。stream mapper 保留文本签名，redacted 分片聚合为不透明的 `RedactedThinking`。用例 `reasoning_content_deltas_stream_as_thinking_blocks`、`redacted_content_accumulates_into_redacted_thinking` 和 `thinking_replay_projects_converse_shapes` 在 179 项 Bedrock 目标中全部通过。

严格 rustdoc 初次运行定位了三处链接。默认配置的 Mistral 与 Pi Messages 文档链接已指向实际 wire adapter。单独启用 `provider-bedrock` 时可读取 profile 表，文档明确 `providers::collection` 的装配入口由 `builtins` 提供。两个配置重跑 `-D rustdoc::broken_intra_doc_links` 均通过。

`opi-doc-check.py` 扫描 SDK 的 Rust 源码、双语 README 和接入指南，检查旧 Pi 路径、版本及来源标签，并校验当前指南的本地文件链接。当前扫描为零命中。测试将旧注释放进现行源码并验证 CLI 退出 1，同一旧标记放在迭代 evidence 中通过。比较用的版本和文件摘要见回执。[Pi 1.5.0 来源、链接与 Bedrock 用例回执](../evidence/issue-219-current-source-guidance.json)
