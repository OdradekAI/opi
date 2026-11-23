# #238 统一 Google Gemini 的发布目录和内置入口

Status: done

接续 #237，在 opi 工作树 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0` 的 Azure 批次结果上同步 Google Gemini。Google Vertex 独立排在下一批。 [#237 Azure OpenAI](237-sync-azure-openai-catalog.md)

## Azure 批次后的 provider 排序

用 `scripts/pi-catalog/pack.py` 按 `(provider, type, id)` 比较完整记录。Pi 1.5.0 的 `nix/model-catalog.json` 固定发布目录的 SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。同步前 opi bundle 的 SHA-256 为 `6a06b38969313002a0fb1c38c1c83e65d5b7179fd2df23e0c862a8e777e3088b`，42 份作者源都与当时的 bundle 数组相等。opi 有 1,697 条，Pi 有 1,712 条，1,591 条身份与字段一致。同步前 15 个 provider 已无目录差异。下表按从 opi 到 Pi 的方向列出其余 27 个，更新按共有记录数计数。 [固定发布目录](../evidence/phase5-model-catalog.json)

| 优先级 | Provider | 当前 → 发布 | 新增 | 撤下 | 更新 | 验收重点 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| P0 | `google` | 23 → 23 | 2 | 2 | 0 | CLI 和独立工厂只列 3 条，其中 1 条失效。验证 Gemini 请求 |
| P1 | `google-vertex` | 14 → 13 | 0 | 1 | 0 | 默认工厂只列 2 条。分别验证 Express 与项目／区域路径 |
| P1 | `vercel-ai-gateway` | 305 → 317 | 24 | 12 | 9 | 45 条差异，跨 Anthropic Messages 聊天和 System One 分类 |
| P1 | `baseten` | 22 → 14 | 2 | 10 | 10 | 单一 Chat Completions 路径，10 个失效身份及推理字段变化 |
| P2 | `typesafe` | 1 → 1 | 0 | 0 | 1 | 分类器图片输入格式及 opt-in 头 |
| P2 | `cloudflare-workers-ai` | 21 → 24 | 3 | 0 | 5 | 聊天和分类器的输入、推理及上下文 |
| P2 | `radius` | 29 → 28 | 3 | 4 | 9 | 静态初始目录、独立工厂、缓存与推理映射；#222 已验网关刷新 |
| P2 | `together` | 17 → 15 | 3 | 5 | 4 | 下线、推理与输出上限 |
| P2 | `fireworks` | 22 → 29 | 7 | 0 | 1 | 新路由及 Qwen 推理字段 |
| P2 | `huggingface` | 78 → 78 | 3 | 3 | 4 | 模型替换及 4 条输出上限 |
| P2 | `xai` | 4 → 4 | 0 | 0 | 2 | Grok 4.3、4.5 的 Responses 请求新增 xhigh |
| P3 | `xiaomi` | 6 → 3 | 0 | 3 | 0 | 主目录下线 |
| P3 | `xiaomi-token-plan-ams` | 4 → 2 | 0 | 2 | 0 | 阿姆斯特丹端点下线 |
| P3 | `xiaomi-token-plan-cn` | 4 → 2 | 0 | 2 | 0 | 中国端点下线 |
| P3 | `xiaomi-token-plan-sgp` | 4 → 2 | 0 | 2 | 0 | 新加坡端点下线 |
| P3 | `nvidia` | 19 → 21 | 3 | 1 | 1 | 新增模型、下线与价格 |
| P3 | `cloudflare-ai-gateway` | 57 → 59 | 2 | 0 | 1 | 新增 Terra、Workers AI 模型及调价 |
| P3 | `kimi-coding` | 4 → 6 | 2 | 0 | 1 | K3.1 及 K3 价格 |
| P3 | `moonshotai` | 4 → 6 | 2 | 0 | 0 | K3.1 两条 |
| P3 | `moonshotai-cn` | 4 → 6 | 2 | 0 | 3 | K3.1 两条及中国区价格 |
| P3 | `minimax` | 3 → 4 | 1 | 0 | 1 | M3.1 Flash、M3 输出上限 |
| P3 | `minimax-cn` | 3 → 4 | 1 | 0 | 3 | 同系列新模型、输出上限及中国区价格 |
| P3 | `qwen-token-plan` | 20 → 21 | 1 | 0 | 0 | DeepSeek 新模型 |
| P3 | `qwen-token-plan-cn` | 20 → 21 | 1 | 0 | 0 | 同 ID 的中国区路由 |
| P4 | `groq` | 7 → 7 | 0 | 0 | 2 | 上下文与价格 |
| P4 | `cerebras` | 2 → 2 | 0 | 0 | 1 | 价格 |
| P4 | `zai-coding-cn` | 4 → 4 | 0 | 0 | 1 | 价格 |
| **合计** | **27 个 provider** | | **62** | **47** | **59** | |

建议先 Google，再 Google Vertex、Vercel、Baseten。Google 和 Vertex 是这 27 个中已列入 coding-agent 内置 provider 的两家。Google 手写表只命中发布目录 2/23，Vertex 默认只列 2/13；把它们前移可以直接修复用户列表和运行选择。[CLI 内置注册与列表](../../../../crates/opi-coding-agent/src/provider_factory.rs) 与 [SDK 内置装配](../../../../crates/opi-ai/src/providers/collection.rs) 分别界定入口。

Vercel 是 SDK 中目录差异最多的一家。其 303 个聊天、14 个分类器分别通过 `/v1/messages` 和 `/typesafe/v1/systemone` 验收。Baseten 在一个协议内有 22 条差异。两家的 CLI 首次接入属于独立产品范围，现有 SDK 可分别验收。后续按表从请求行为变化推进到同厂商地区目录，再到价格、上下文修订。每个 provider 分别核对身份和进度。

## Google 的实施结果

使用 #226 的旧发布 blob 和本次固定发布归档三方比较。上表涉及的 27 家在同步前均与旧基线相同。临时目录中的 `sync.py --provider google` 得到零冲突、23 条候选记录，完整字段及顺序与发布归档相等。已采纳候选源。 [#226 三方同步流程](226-sync-provider-catalog-three-way.md)

- 新增 `gemini-pro-latest`、`gemini-robotics-er-2-preview`，撤下 `gemini-3.1-flash-lite-image`、`gemini-3.1-flash-live-preview`。其余 21 条发布记录字段一致。
- SDK 内置装配、独立工厂、CLI 列表和运行工厂共用审核后的 Google 目录及类型化投影。独立工厂读取单个作者文件，`provider-gemini` 可独立使用。原手写表中的 `gemini-2.0-flash` 也已撤下。CLI 使用 `google_provider`，请求结果和回放使用 `google` 身份。 [Gemini 工厂](../../../../crates/opi-ai/src/providers/gemini.rs)
- CLI 显式代理以主机或路径前缀配置，工厂补上 `/v1beta` 并绑定到本次路由。显式模型记录可以提供完整端点。Vertex 的请求体由 Vertex 自己的记录驱动，避免混用 Google 的能力数据。 [Gemini 请求适配器](../../../../crates/opi-ai/src/api/gemini.rs) · [Vertex profile](../../../../crates/opi-ai/src/providers/vertex.rs)

## 三项验收

1. **目录与入口。** 23 条 `google-generative-ai` 聊天记录与固定发布归档相等。SDK 查询、独立 Gemini 工厂、coding-agent 静态列表及运行工厂的类型化记录一一相等。静态列表读取凭据零次。两次离线生成的 bundle 字节一致，`build_catalog check` 通过，其他 41 份作者源通过逐字节比较。
2. **实际请求。** SDK 和 coding-agent 生产工厂各向回环服务发出 8 次请求，验证两个新模型的 `POST /v1beta/models/{id}:streamGenerateContent?alt=sse`、`x-goog-api-key`、返回模型 ID、`google` 身份及 CLI 显式代理前缀。省略推理发送 `thinkingBudget=0`，启用 High、Minimal 或 Max 且使用默认预算时发送 `includeThoughts=true` 和 `thinkingBudget=-1`。这些请求均省略 `thinkingLevel`。模型映射将 Pro 的 Minimal 取为 Low，Robotics 取为 Minimal，两者的 Max 均取 High。
3. **下线。** 旧 Google bundle 和手写表的并集减发布目录得到三个失效 ID。逐个验证 SDK 查询、独立工厂和 CLI 列表均查不到。SDK、CLI 运行工厂及独立工厂的直接请求在发出 HTTP 前返回 `UnknownModel`。CLI 非交互与 RPC 启动返回配置错误，交互入口使用同一个启动校验。回环服务对失效 ID 收到零个请求。

固定 Pi 1.5.0 的 `streamSimple` 已向回环服务实发 8 次，两个 ID 各 4 次，均返回选定的模型身份。另两组请求给 minimal、low、high 配置 128、512、1024，以观测档位选择。请求 Minimal 时分别收到 512 和 128，Max 时均为 1024。Rust 的通用 simple 选项当前没有这张逐档预算表，验收使用现有等级选择和 HTTP 能观察到的字段，无需增加公共参数。

Google 作者源 SHA-256 为 `6f48c01e13475329cdeca653c5acca8c1fa8e8ae49fb32813a8388f0f28f874b`，bundle 为 `589344a2eda2a0251743b9cc1bc353635c1888a8b8c6672bee0982bec3521d81`。全库有 1,697 条记录，16 个 provider 已与固定发布目录一致。剩余 26 个 provider，待新增／撤下／更新为 60／45／59。

## 测试回执

- 初次回归分别复现独立工厂只列 3 条、CLI 成功结果写入 `gemini` 身份、下线 ID 可绕过独立工厂的目录检查。修复后，SDK 14 个相关目标共 454 项通过。涵盖 `gemini_fixtures` 72 项、`gemini_lifecycle` 19 项、`gemini_repeated_finish` 3 项、`vertex_fixtures` 74 项、`provider_collection` 118 项及图像序列化 15 项。图像夹具静态注册自己的 `test-model`，沿用原有的 HTTP 断言。
- coding-agent 的 `provider_factory` 62 项、`list_models` 17 项、binary 40 项通过。库测试 234 项通过，1 项忽略。目录工厂用例验证完整类型化元数据与身份。回环测试另外验证带 `google-vertex:` 前缀的请求无法通过 Google 直接工厂发送。
- `catalog_artifact` 14 项、生成器 8 项、独立 `provider-gemini` 与 `provider-vertex` feature 各 1 项通过。三方同步脚本 14 项、归档脚本 36 项通过。定向 Clippy 通过并报告三条既有库警告。格式、架构、测试清单、文档与 diff 检查通过。
- `catalog-bundled` 和 `provider-gemini` 的两个独立消费者均通过。实际 `.crate` 的 SHA-256 为 `0dcc568227ae4af2379807a2a25d3af3e3ab9ea2bec7b5138adbb7b885897d66`。两份归档内的 Google 作者源、bundle 与 manifest 均和工作区逐字节一致。目录消费者读取到 1,585 条 chat、48 条 classifier 和 64 条 image 记录。

Test impact — `add` Google 跨入口目录、回环请求、三个失效 ID 的派发与启动回归。 `update` 独立 feature 和归档消费、Gemini 结果身份和图像夹具。 `retain` 显式模型端点、Vertex 与 OpenCode 共用协议回归。

本批在当前工作区验收，暂存 0 项、未暂存 157 项、未跟踪 125 项。25 个本批路径中，21 个带入已有改动，4 个仅归本批。其他带入改动的字节哈希与开始时一致。
