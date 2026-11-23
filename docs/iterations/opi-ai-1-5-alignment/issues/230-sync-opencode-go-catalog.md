# #230 同步 OpenCode Go 目录与请求路由

Status: done

沿用 [#229 OpenCode Zen](229-sync-opencode-zen-catalog.md) 的三项验收条件。用 [#226 三方同步流程](226-sync-provider-catalog-three-way.md) 将 `opencode-go` 从 Pi 1.3 发布基线更新到 [Pi 1.5 固定发布归档](../evidence/phase5-model-catalog.json)。请求行为核对同版本 `packages/ai/src/providers/opencode-go.ts`、`packages/ai/catalog/providers/opencode-go.ts`、`packages/ai/src/api/google-shared.ts` 和 `packages/ai/test/google-thinking-level-map.test.ts`。

## 目录同步与下线

三方合并无冲突，新增 6 条、下线 7 条、更新 24 条。30 条聊天记录的身份、完整字段和顺序与发布归档一致。24 条共有记录都更新基址，`kimi-k3` 同时将缓存写入价格由 0 调整为每百万 token 3 美元。共有记录的协议和推理映射一致。

7 个下线 ID 均通过 `Catalog::model`、`Models::get_model` 和 `Models::get_models_of_type` 验证。OpenCode Go 由 SDK 聚合目录装配，coding-agent 的内置 provider 列表没有独立的 Go 入口。

## 公开请求验收

经默认 builtin 装配取得模型记录，保留其路径并将 origin 指向本地 HTTP 服务。用 `Models::complete_simple` 验证完整请求路径、模型身份、认证头和 session 头。

| 协议 | 模型 | 请求路径 | 推理参数 |
| --- | --- | --- | --- |
| Chat Completions | 新增 `kimi-k3.1-pro` | `/inference/go/openai/v1/chat/completions` | Medium 发送 `reasoning_effort: "high"`，关闭时省略 effort |
| Responses | `gpt-5.6-luna` | `/inference/go/openai/v1/responses` | 默认 simple 请求 |
| Anthropic Messages | `qwen3.8-flash` | `/inference/go/anthropic/v1/messages` | 默认 simple 请求 |
| Gemini | 新增 `gemini-4-argon` | `/inference/go/google/v1beta/models/gemini-4-argon:streamGenerateContent?alt=sse` | XHigh 发出 `HIGH` 和 `includeThoughts: true`。默认选项只发出 `LOW` |

旧目录复现了下线 ID 仍能查到和 Responses 请求发到旧路径的问题。换用新目录后，Go 的 Gemini 默认请求还发出了零预算。Pi 对不支持 Off 的离散推理等级模型使用最低可用档。修正在 Gemini 和 Vertex 共用的请求构造处，沿用公开模型的等级映射。Vertex native 请求另有一项回归，核对最低可用档映射到的实际协议值。

## 产物验收

- 其余 41 份 provider 源文件逐字节一致，含 OpenCode Zen。
- 两次离线生成逐字节一致。`build_catalog check` 通过。
- 源目录 `opencode-go.json` 的 SHA-256 为 `10b2bc0a303dffd52ab8533f9768f995143ce2daf3d51da2fcec4d4b84d7c896`。聚合 `catalog.json` 的 SHA-256 为 `7cf39ab2c51030e6af9a799dfe90912594ba23623f894f4630efeefadf8be651`。
- 从实际 `opi-ai-0.8.3.crate` 安装的独立 `catalog-bundled` 消费者读出了相同摘要及计数。归档 SHA-256 为 `3cc3756559b4aeb9f75daa80fb9e885647535e005912635f120d048546818bbd`。

## 执行检查

| 本地检查 | 结果 |
| --- | --- |
| `cargo test -p opi-ai --offline --locked --no-default-features --features builtins --test provider_collection --test gemini_fixtures --test vertex_fixtures --test catalog_operations --test external_sdk` | 118 + 70 + 73 + 27 + 11 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --features catalog-bundled --test catalog_artifact` | 14 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --example build_catalog` | 8 项通过 |
| 禁用默认 feature 后分别运行 `provider_gemini_profile` 和 `provider_vertex_profile` | 各 1 项通过 |
| `python -m unittest discover -s scripts/pi-catalog` | 14 项通过 |
| `run_catalog_bundled_package.py` | 独立归档消费通过 |
| 定向 Clippy、`cargo fmt --all --check`、`opi-doc-check.py`、`git diff --check` | 通过，Clippy 仍有 3 项既有库 lint |

## 下次上游基线

Mistral、OpenRouter、OpenCode Zen 和 OpenCode Go 已采纳固定的 Pi 1.5 发布基线，SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。本票结束时其余 38 个 provider 使用 [#226 记录的 Pi 1.3 Git blob](226-sync-provider-catalog-three-way.md#mistral-基线)。当时全库 42 个 provider 共 1,703 条记录。10 个 provider 的身份与字段一致，32 个仍有差异。只读比较得到 74 条仅参考有、65 条仅 opi 有、115 条共有记录字段不同。[采纳 Amazon Bedrock 后的基线](231-sync-amazon-bedrock-catalog.md#下次上游基线)

Test impact — add Go 下线查询与四种协议请求回归、Vertex native 禁用推理回归。update Zen 与 Go 共用的本地聊天请求 fixture。retain Gemini、Vertex、目录装配和归档消费检查。
