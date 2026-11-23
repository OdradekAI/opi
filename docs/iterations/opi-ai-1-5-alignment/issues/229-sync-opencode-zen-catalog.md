# #229 同步 OpenCode Zen 目录与请求路由

Status: done

按 [#226 三方同步流程](226-sync-provider-catalog-three-way.md) 更新 `opencode`。上次基线是 Pi 1.3 的发布目录，目标是 [Pi 1.5 固定发布归档](../evidence/phase5-model-catalog.json)。本批验收以归档中的身份、完整字段和顺序为准。通过同一版本的 `packages/ai/src/providers/opencode.ts`、`packages/ai/catalog/providers/opencode.ts` 与 `packages/ai/src/models.ts` 核对路由和推理等级选择。

## 目录同步

三方合并无冲突，85 条变为 81 条。其中 79 条为聊天模型，2 条为分类模型。新增 5 个聊天模型，撤下 9 个聊天模型，76 个共有模型的基址更新。共有身份的协议保持一致。其他字段变化涉及兼容参数、价格和推理等级映射。两条 Jev 记录继续使用相同的模型 ID。

检查全部 9 个下线 ID 在 `Catalog::model` 和 builtin `Models::get_model` 中均查询不到。Rust SDK 的 builtin 由目录装配，coding-agent 的内置 provider 列表没有 Zen 入口。新数据通过现有的作者校验和适配器装配。

## 公开请求验收

用默认 builtin 装配获取真实模型记录，保留端点路径并将协议、主机和端口替换为本地 HTTP 服务，再从 `Models::complete_simple` 或 `Models::classify` 发起请求。校验完整请求路径、有效模型身份和响应。聊天请求核对 session 头，各协议核对其认证头。

| 协议 | 模型 | 发出的请求路径 | 请求体核对 |
| --- | --- | --- | --- |
| Chat Completions | `kimi-k3`、`qwen3.8-max` | `/inference/openai/v1/chat/completions` | 分别请求 Medium、High，发送 `reasoning_effort` 为 `high`、`xhigh` |
| Responses | 新增的 `muse-wonder-1.0` | `/inference/openai/v1/responses` | 模型 ID |
| Anthropic Messages | `claude-opus-4-8` | `/inference/anthropic/v1/messages` | 模型 ID |
| Gemini | `gemini-3.1-pro` | `/inference/google/v1beta/models/gemini-3.1-pro:streamGenerateContent?alt=sse` | 模型 ID 位于请求路径 |
| System One | `jev-1.13`、`jev-1.13-free` | `/inference/systemone/v1/systemone` | 各自的模型 ID 和输入 state |

新增的 `builtin_opencode_zen_` 两项测试在旧目录上分别因下线 ID 仍可查询和 Kimi 请求旧路径而失败，应用候选后均通过。

## 产物验收

- 81 条完整记录及顺序与固定发布归档一致。其余 41 份 provider 源文件逐字节一致，包含 OpenCode Go。
- 两次离线生成结果逐字节一致。`build_catalog check` 通过。
- 源目录 `opencode.json` 的 SHA-256 为 `8fbfcf05d4e15f71eb7aad960cf838a71d66b4f262dc68cd293edb410f448fbe`。聚合 `catalog.json` 的 SHA-256 为 `bbf86a4702cede64fa86d870841b357f69cdbccfefc1c0b586c383ea9337b3e3`。
- 从实际 `opi-ai-0.8.3.crate` 安装的独立 `catalog-bundled` 消费者读出了相同摘要及计数。归档 SHA-256 为 `98a0d27696da398b0789d9e301fe31efbe44608f5690c2569965e4465071fb59`。

## 执行检查

| 本地检查 | 结果 |
| --- | --- |
| `cargo test -p opi-ai --offline --locked --no-default-features --features builtins --test provider_collection --test catalog_operations` | 116 + 27 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --features catalog-bundled --test catalog_artifact` | 14 项通过 |
| `cargo test -p opi-ai --offline --locked --no-default-features --example build_catalog` | 8 项通过 |
| `python -m unittest discover -s scripts/pi-catalog` | 14 项通过 |
| `run_catalog_bundled_package.py` | 独立归档消费通过 |
| 定向 Clippy、`cargo fmt --all --check`、`opi-doc-check.py`、`git diff --check` | 通过，Clippy 仍有 3 项既有库 lint |

## 下次上游基线

Mistral、OpenRouter 和 OpenCode Zen 已采纳固定的 Pi 1.5 发布基线，SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。本票结束时其余 39 个 provider 使用 [#226 记录的 Pi 1.3 Git blob](226-sync-provider-catalog-three-way.md#mistral-基线)。当时全库 42 个 provider 共 1,704 条记录。9 个 provider 的身份与字段相同，33 个仍有差异。只读比较得到 80 条仅参考有、72 条仅 opi 有、139 条共有记录字段不同。[采纳 OpenCode Go 后的基线](230-sync-opencode-go-catalog.md#下次上游基线)

Test impact — add 公开 builtin 的下线查询、五种协议路径与推理参数回归。retain 目录生成、类型化目录、provider 装配与归档消费检查。
