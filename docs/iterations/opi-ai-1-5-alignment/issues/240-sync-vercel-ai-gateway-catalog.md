# #240 同步 Vercel AI Gateway 的聊天与分类目录

Status: done

## Vertex 完成后的目录基线

以 [#239](239-sync-google-vertex-catalog.md) 完成后的工作树为起点。用 `scripts/pi-catalog/pack.py` 按 `(provider, type, id)` 比较完整字段。opi bundle SHA-256 为 `cbb59f1490ef66fd52c5807f16ab08f01a4c2b2ba221d9d12bc4649fc20b6417`。Pi 1.5.0 的 `.pi/pi-1.5.0/nix/model-catalog.json` 将发布文件固定在 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。42 份 opi 作者源与各自的 bundle 数组相等，仍有差异的 25 份也与 #226 的旧发布基线相等。 [固定发布文件](../evidence/phase5-model-catalog.json) · [三方同步依据](226-sync-provider-catalog-three-way.md)

opi 共有 1,696 条，Pi 共有 1,712 条。17/42 个 provider 的目录逐字段一致，1,593 条记录身份与字段一致。下表从 opi 看向 Pi，更新按身份相同、字段有差异的记录计数。目录进度和协议验收分别计数。

| 顺序 | Provider | opi → Pi | 新增 | 撤下 | 更新 | 排序依据 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| 1 | `vercel-ai-gateway` | 305 → 317 | 24 | 12 | 9 | 45 条差异，两种操作，聊天请求还涉及查询参数 |
| 2 | `baseten` | 22 → 14 | 2 | 10 | 10 | 10 个失效聊天 ID，同时涉及推理档位和推理字段序列化 |
| P2 | `typesafe` | 1 → 1 | 0 | 0 | 1 | 同一 System One 协议的图片输入、assets 和 opt-in 头 |
| P2 | `cloudflare-workers-ai` | 21 → 24 | 3 | 0 | 5 | 聊天和分类输入、推理及上下文 |
| P2 | `radius` | 29 → 28 | 3 | 4 | 9 | 初始目录、缓存与推理映射，需区分已验收的网关刷新 |
| P2 | `together` | 17 → 15 | 3 | 5 | 4 | 下线、推理与输出上限 |
| P2 | `fireworks` | 22 → 29 | 7 | 0 | 1 | 新增路由及 Qwen 自适应推理 |
| P2 | `huggingface` | 78 → 78 | 3 | 3 | 4 | 模型替换及输出上限 |
| P2 | `xai` | 4 → 4 | 0 | 0 | 2 | 两条 Grok 记录开放 xhigh |
| P3 | `xiaomi` | 6 → 3 | 0 | 3 | 0 | 下线 |
| P3 | `xiaomi-token-plan-ams` | 4 → 2 | 0 | 2 | 0 | 区域下线 |
| P3 | `xiaomi-token-plan-cn` | 4 → 2 | 0 | 2 | 0 | 区域下线 |
| P3 | `xiaomi-token-plan-sgp` | 4 → 2 | 0 | 2 | 0 | 区域下线 |
| P3 | `nvidia` | 19 → 21 | 3 | 1 | 1 | 新增、下线及价格 |
| P3 | `cloudflare-ai-gateway` | 57 → 59 | 2 | 0 | 1 | 新增及价格 |
| P3 | `kimi-coding` | 4 → 6 | 2 | 0 | 1 | K3.1 及价格 |
| P3 | `moonshotai` | 4 → 6 | 2 | 0 | 0 | K3.1 |
| P3 | `moonshotai-cn` | 4 → 6 | 2 | 0 | 3 | K3.1 及区域价格 |
| P3 | `minimax` | 3 → 4 | 1 | 0 | 1 | M3.1 Flash 及输出上限 |
| P3 | `minimax-cn` | 3 → 4 | 1 | 0 | 3 | 同系列模型、输出上限及区域价格 |
| P3 | `qwen-token-plan` | 20 → 21 | 1 | 0 | 0 | 新增模型 |
| P3 | `qwen-token-plan-cn` | 20 → 21 | 1 | 0 | 0 | 同 ID 的区域路由 |
| P4 | `groq` | 7 → 7 | 0 | 0 | 2 | 上下文及价格 |
| P4 | `cerebras` | 2 → 2 | 0 | 0 | 1 | 价格 |
| P4 | `zai-coding-cn` | 4 → 4 | 0 | 0 | 1 | 价格 |
| 合计 | 25 个 provider | | 60 | 44 | 59 | |

## Vercel 的实施范围

只同步 `vercel-ai-gateway` 作者文件及派生目录，沿 SDK 的内置装配验收。聊天由共享 Anthropic Messages 工厂承接，分类使用 System One。opi 没有 Vercel 专属独立工厂，coding-agent 的 11 个内置 provider 也未注册 Vercel。Baseten 经 Vercel 提供的分类器归本票，`provider=baseten` 的直连聊天目录排在下一票。若有额外兼容需求，须先取得项目所有者同意。 [SDK 装配](../../../../crates/opi-ai/src/providers/collection.rs) · [分类路由](../../../../crates/opi-ai/src/providers/catalog_routes.rs) · [CLI 内置清单](../../../../crates/opi-coding-agent/src/provider_factory.rs)

| 操作 | opi → Pi | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Anthropic Messages 聊天 | 297 → 303 | 16 | 10 | 7 |
| System One 分类 | 8 → 14 | 8 | 2 | 2 |

九条共有记录只涉及价格与上下文。例如，`mistral/mistral-large-4` 的上下文从 524,288 增至 1,048,576，分类器 `liquid/d1` 的输入单价从 0.04 降到 0.02。14 个发布分类器均声明纯文本输入。

## 三项验收条件

1. **目录和 SDK 入口一致。** 用 #226 的三方流程取得 317 条候选记录。作者源和 bundle 的 Vercel 数组与固定发布文件逐字段、逐条同序相等。通过 `Catalog::builtin`、注册后的 `Models` 总列表、分操作列表、按 ID 查询和配置凭据后的可用列表核对 303 个聊天及 14 个分类器的身份与公开类型化元数据。两次离线生成字节一致，`build_catalog check` 通过，其他 41 份作者源逐字节一致。
2. **实际请求与固定 Pi 一致。** 通过 SDK 的内置 `Models`，仅把发布 URL 的 origin 指向本地 HTTP 服务，保留分类前缀。用同一枚测试 `AI_GATEWAY_API_KEY` 覆盖下表五个调用，核对完整路径、查询串、凭据头、请求体、返回的 provider 与模型 ID。现有 Rust 聊天适配器拼接 `/v1/messages`，实施时要处理已观察到的查询串差异。修改共享适配器时按受影响的调用方补验其协议。 [聊天适配器](../../../../crates/opi-ai/src/api/anthropic.rs) · [分类适配器](../../../../crates/opi-ai/src/api/system_one.rs)
3. **逐个证明 12 个 ID 已撤下。** 从旧、新发布记录的差集取得下列清单。在 Vercel 的全部内置列表及查询入口逐个确认缺席。10 个聊天 ID 经正常 SDK 选择并发起调用应在 HTTP 前失败，2 个分类 ID 经 `Models::classifier_model` 返回 `NotAClassifierModel`。回环服务收到的下线请求数为零。调用方自行提供完整记录的显式 API 仍按其自身契约验收。 [公开选择与分类入口](../../../../crates/opi-ai/src/models.rs)

下表数值来自冻结源码的五次离线 fetch 捕获，均返回 `stop`。使用 `.pi/pi-1.5.0/packages/ai/src/providers/vercel-ai-gateway.ts` 注册所指定的 `api/anthropic-messages.ts`、`api/typesafe-system-one.ts`，输入来自发布文件，Anthropic SDK 版本为 0.129.0。Rust 回环验收使用同样的输入。

| 模型与输入 | 完整 POST 目标及认证 | 请求和结果断言 |
| --- | --- | --- |
| `openai/gpt-6.1-terra`，xhigh，显式上限 4,096 | `/v1/messages?beta=true`，`x-api-key` | `max_tokens=20480`，`thinking.type=enabled`，`budget_tokens=16384`，`display=summarized`，省略 `output_config.effort`。`anthropic-beta: interleaved-thinking-2025-05-14`，JSON 无 `betas` 字段 |
| 同一 Terra，省略推理，显式上限 4,096 | 同一聊天目标和头 | `max_tokens=4096`，`thinking.type=disabled` |
| `moonshotai/kimi-k3.1-pro`，省略推理与输出上限，短上下文 | 同一聊天目标和头 | `max_tokens=131072`，`thinking.type=disabled`。三次聊天均有 `stream=true` 和 `anthropic-version: 2023-06-01` |
| `typesafe-ai/jev-1.14.0`、`baseten/elephant-1`，各发一次文本布尔问题 | `/typesafe/v1/systemone`，`Authorization: Bearer` | 正文含各自完整 ID、`state`、`questions`。公开布尔问题编码为 `noul`，结果按布尔概率解码 |

| 撤下的操作 | 旧 ID |
| --- | --- |
| 聊天（10） | `anthropic/claude-sonnet-4`、`arcee-ai/trinity-large-thinking`、`openai/gpt-3.5-turbo`、`openai/gpt-4-turbo`、`openai/gpt-4.1-nano-batch`、`openai/gpt-4.1-nano-fast`、`openai/o4-mini-fast`、`poolside/laguna-s-2.1-free`、`spacexai/grok-4.1-fast-non-reasoning`、`spacexai/grok-4.1-fast-reasoning` |
| 分类（2） | `convaiinnovations/laya-free`、`perplexity/pplx-decider-v1.1-27b` |

## 实施结果

- 三方同步零冲突。317 条记录的字段和顺序与发布文件一致。已有 41 份其他作者源逐字节通过比对。两次离线生成的 bundle、manifest 和 OpenRouter 派生产物逐字节相等，`build_catalog check` 通过。Vercel 作者源 SHA-256 为 `8d86d0a0fe0caa9758d8653ae372e755567878e6735f8409a3ac7a5ba04da721`，bundle 为 `d9c4b44ad69a48ed66a42517651c024a17812ebb76edceceaa9dde403241dcbe`。
- 三次聊天实际请求验证了查询串、认证、推理和输出上限。修复前能复现 `beta=true` 缺失、`betas` 写入 JSON、关闭推理时省略 `thinking`。在 Anthropic Messages 适配器的 Vercel HTTP 发送点添加查询参数并将 beta 功能移到请求头。simple 路径对支持 Off 的推理模型发送 `thinking.type=disabled`。Terra 的数值预算复用已有的共享准备逻辑。 [回环测试](../../../../crates/opi-ai/tests/provider_collection/vercel.rs) · [发送与推理逻辑](../../../../crates/opi-ai/src/api/anthropic.rs)
- Jev 和 Elephant 各发送一次 System One 请求。测试在选中发布记录后仅替换 origin，保留 `/typesafe/v1`，验证 Bearer 认证、正文、布尔概率 `0.91` 及结果的 provider、模型和 API 身份。聊天与分类均从同一个 `AI_GATEWAY_API_KEY` 测试来源取凭据。
- 12 个下线 ID 均未出现在目录、总列表、可用列表及分操作查询。10 个聊天 ID 的 `prepare_call` 返回 `RegistryError::UnknownModel`，2 个分类 ID 的选择返回 `NotAClassifierModel`。回环服务收到零个下线请求。 [下线验收](../../../../crates/opi-ai/tests/provider_collection/vercel.rs)

本次请求参考的源码位于 `.pi/pi-1.5.0/packages/ai/src`。 `api/anthropic-messages.ts` 的 SHA-256 为 `f1348f1919aa9c4d2ed69767e4f64727e29df0455e8998a7da55773197fce48b`，`api/typesafe-system-one.ts` 为 `3c73e0877e7f44a9ae4a25f481e1e752f585e4d3f8078464a68cdb093d30a965`。全部五次 Pi 捕获使用离线 fetch 夹具，Rust 通过回环 HTTP 请求验收。

全库现有 1,708 条，含 1,590 个聊天、54 个分类器和 64 个图像模型。18/42 个 provider 与固定发布目录完全一致，剩余 24 个。待新增／撤下／更新为 36／32／50。下一批为 Baseten 直连。

## 测试回执

- SDK 12 个定向目标共 459 项通过。包括 `provider_collection` 122 项、`anthropic_fixtures` 155 项、`classifier_contract` 84 项、`catalog_artifact` 14 项。其余覆盖 Anthropic usage、流终态、thinking 回放，以及 API 映射、内置认证、外部 SDK 和调用准备。
- 目录生成器 8 项、独立 `catalog-bundled` 14 项、独立 `provider-anthropic` 1 项通过。三方同步和目录比较脚本共 22 项，两个归档 runner 的 Python 测试共 40 项通过。定向 Clippy 通过并报告三条既有库警告。格式、架构、测试清单、文档及 diff 检查通过。
- `catalog-bundled` 和 `provider-anthropic` 的两个独立消费者均通过。实际 `.crate` 的 SHA-256 为 `d02c7c0200e6bbc891e718366c406f025a7bcb91dbb207f4d654f317db47d8bf`。两份归档中的 Vercel 作者源、bundle、manifest、协议源码、回归用例和中英文指南均与工作树逐字节一致。
- 附加 `test_hermeticity` 为 14 项通过、1 项失败。静态守卫将两个既有的 OpenCode `opencode.ai` 前缀字面量判断为外网调用。两处请求在发出前均用回环地址替换 origin。 [OpenCode 夹具](../../../../crates/opi-ai/tests/provider_collection.rs)

Test impact — `add` 四项 Vercel 入口回归，覆盖目录、聊天、分类和下线。 `retain` 共用 Anthropic Messages 与 System One 合同及归档测试。

本批验收基于工作树 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0`。暂存 0 项、未暂存 161 项、未跟踪 128 项。本批 11 个路径中 10 个带入已有改动，1 个仅归本批。其他带入文件的字节哈希与开始时相同。
