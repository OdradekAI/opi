# #234 同步 GitHub Copilot 的发布目录、coding-agent 入口与三条请求路径

Status: done

Owner: opi-ai 内置目录与 coding-agent Copilot 工厂。前置工作 [#233](233-bedrock-preferred-strict-tool-limit-retry.md) 已完成。

## 批次选择

以 Pi 1.5.0 的 `nix/model-catalog.json` 校验[发布归档](../evidence/phase5-model-catalog.json)，SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。同步前 opi bundle 的 SHA-256 为 `4c323ee874c62a9dc2a7e58e545a9fe8c6914c1ef8c094f591ec24bba8b1c1d6`。按 `(provider, type, id)` 比较完整记录，42 个 provider 中 11 个目录一致、31 个有差异。opi 有 1,707 条，Pi 有 1,712 条。共有 1,578 条完全一致、64 条字段不同，另有 70 条仅 Pi 有、65 条仅 opi 有。统计范围为目录数据。

优先处理会让用户列出、选择或发送到不同模型的入口分歧，再处理可复用验收的协议和大批次。下表的新增、撤下、更新均从当前 opi 源看向固定发布归档。

| 优先级 | Provider | 当前 → 目标 | 新增 | 撤下 | 更新 |
| --- | --- | ---: | ---: | ---: | ---: |
| P0 | `github-copilot` | 36 → 27 | 1 | 10 | 1 |
| P1 | `openai-codex` | 9 → 8 | 1 | 2 | 0 |
| P1 | `openai` | 45 → 45 | 3 | 3 | 4 |
| P1 | `azure` | 45 → 45 | 3 | 3 | 0 |
| P1 | `vercel-ai-gateway` | 305 → 317 | 24 | 12 | 9 |
| P1 | `baseten` | 22 → 14 | 2 | 10 | 10 |
| P2 | `typesafe` | 1 → 1 | 0 | 0 | 1 |
| P2 | `cloudflare-workers-ai` | 21 → 24 | 3 | 0 | 5 |
| P2 | `radius` | 29 → 28 | 3 | 4 | 9 |
| P2 | `together` | 17 → 15 | 3 | 5 | 4 |
| P2 | `fireworks` | 22 → 29 | 7 | 0 | 1 |
| P2 | `google` | 23 → 23 | 2 | 2 | 0 |
| P2 | `google-vertex` | 14 → 13 | 0 | 1 | 0 |
| P2 | `huggingface` | 78 → 78 | 3 | 3 | 4 |
| P2 | `xai` | 4 → 4 | 0 | 0 | 2 |
| P3 | `nvidia` | 19 → 21 | 3 | 1 | 1 |
| P3 | `kimi-coding` | 4 → 6 | 2 | 0 | 1 |
| P3 | `moonshotai` | 4 → 6 | 2 | 0 | 0 |
| P3 | `moonshotai-cn` | 4 → 6 | 2 | 0 | 3 |
| P3 | `minimax` | 3 → 4 | 1 | 0 | 1 |
| P3 | `minimax-cn` | 3 → 4 | 1 | 0 | 3 |
| P3 | `cloudflare-ai-gateway` | 57 → 59 | 2 | 0 | 1 |
| P3 | `qwen-token-plan` | 20 → 21 | 1 | 0 | 0 |
| P3 | `qwen-token-plan-cn` | 20 → 21 | 1 | 0 | 0 |
| P3 | `xiaomi` | 6 → 3 | 0 | 3 | 0 |
| P3 | `xiaomi-token-plan-ams` | 4 → 2 | 0 | 2 | 0 |
| P3 | `xiaomi-token-plan-cn` | 4 → 2 | 0 | 2 | 0 |
| P3 | `xiaomi-token-plan-sgp` | 4 → 2 | 0 | 2 | 0 |
| P3 | `groq` | 7 → 7 | 0 | 0 | 2 |
| P3 | `cerebras` | 2 → 2 | 0 | 0 | 1 |
| P3 | `zai-coding-cn` | 4 → 4 | 0 | 0 | 1 |

- P0 先处理 GitHub Copilot。SDK 有 36 条，coding-agent 的静态列表和 OAuth 工厂共用另一份 25 条手写表。后者只有 7 个 ID 命中发布目标，且旧测试固定了 25 条和已撤下模型的请求路径。
- P1 建议依次处理 OpenAI Codex、OpenAI、Azure、Vercel AI Gateway、Baseten。Codex 也有独立的 CLI 手写目录。OpenAI 的 4 个模型撤去附加工具和工具搜索声明，Azure 与 OpenAI 具有同一组模型增删，部署路径须独立验收。Vercel 的 45 条差异跨 chat 与 classifier，Baseten 的 22 条差异集中于单一 Chat Completions 协议。
- P2 优先覆盖有请求行为差异的协议。Typesafe 的图像输入和 opt-in 请求头可先验，再核查 Cloudflare Workers AI 的图像分类路径。Radius 涉及缓存、推理映射和 provider 路由数据。Google 与 Vertex 分别验证各自的上下线入口。
- P3 可按厂商家族复用测试用例，每个 provider 单独推进发布基线，分别核对价格、额度及下线身份。

## 本批范围

只推进 `github-copilot`。依照 [#226 三方同步规则](226-sync-provider-catalog-three-way.md)，使用其中固定的 Pi 1.3 Git blob 作旧基线。本轮规划已在临时目录运行 `sync.py`，得到无冲突的 27 条，记录值及顺序与发布归档一致，其他 41 份源文件逐字节一致。

新增 `gpt-6.1-terra`，更新 `gpt-5.4-nano.thinkingLevelMap`。源目录需撤下 `claude-opus-4.7`、`gemini-3.5-flash`、`gemini-3.6-flash`、`gemini-3.7-flash`、`gpt-5-mini`、`gpt-5.4`、`gpt-5.4-mini`、`gpt-5.5`、`kimi-k2.7-code`、`mai-code-1-flash-picker`。

将 coding-agent 的 `github_copilot_catalog()`、列表和运行工厂接到 opi 已有目录及类型化 chat 模型投影，撤掉手写 `RECORDS` 和旧版 25 条 fixture 对当前行为的约束。Copilot 身份、OAuth 端点选择和请求头仍由现有 owner 负责。协议依据为固定版本的 `packages/ai/src/providers/github-copilot.ts`、`packages/ai/src/models.ts` 和 `packages/ai/src/api/openai-responses.ts`。需要为下线模型增加别名、保留入口或回退行为时，须先取得项目所有者同意。

## 三项验收

1. **目录与入口同源。** 离线重建并校验 bundle，对照 27 条完整发布记录，验证 SDK、CLI 静态列表和运行工厂的查询、协议分配及类型化元数据。应为 14 个 Responses、10 个 Messages 和 3 个 Chat Completions 模型。CLI 静态列举保持零凭据读取。已有账户可用性过滤入口用测试凭据验证与当前目录取交集。全库应为 1,698 条，12 个 provider 的目录完全一致。
2. **请求行为。** 通过 SDK 内置装配和 coding-agent 工厂向本地 HTTP 服务发送真实请求。`gpt-6.1-terra` 走 `POST /responses`，simple 请求 Max 时发 `reasoning.effort="max"`。`gpt-5.4-nano` 的 Minimal 发 `"low"`，Max 按 simple 规则收敛为 `"xhigh"`。用 `claude-sonnet-4.6` 核对 `POST /v1/messages`，用 `gemini-4-argon` 核对 `POST /chat/completions`。抓取模型 ID、实际 URL、Bearer 认证、Copilot 管理的 initiator 与图像请求头。检查 OAuth 解析的企业端点确实收到请求。
3. **下线闭环。** 取「旧 SDK 源 ∪ 旧 CLI 手写表」减目标目录，得到 **20 个**失效 ID。SDK 源须撤下 10 个，CLI 须撤下 18 个，两者有重合。逐个验证 catalog、注册表查询、CLI 列表和运行配置选择，失效 ID 应在发送前报告未知模型，本地服务收到零次请求。对 `gpt-4.1`、`claude-sonnet-4.5` 等仅留在 CLI 的旧 ID 也要覆盖。

定向执行 opi-ai 的 `catalog_artifact`、`provider_collection` 和 `catalog_personalization` 中相关用例，以及 coding-agent 的 `github_copilot_provider`、`list_models`、`provider_factory`。修改了使用旧 ID 的 `oauth_auth` 或 `doctor_cli` 用例时，按名执行。用实际 `catalog-bundled` 归档消费者验证离线产物。完成后同口径全库应剩 69 个待新增、55 个待撤下、63 个字段更新，共 30 个有差异的 provider。

## 验收结果

- 27 条记录与发布归档逐条同值同序，其他 41 份 provider 源逐字节一致。两次离线生成产物一致，`build_catalog check` 通过。源文件 SHA-256 为 `3ba4e7f43afbdb6f8bedfce7ac0bb231c26dcb39d834718680efeca4280feb3c`，bundle 为 `f08a368cb25d1cdd023935539f9374e944bcbf39819dae13021d257aefdb0188`。
- SDK 内置、CLI 列表、运行工厂及 OAuth 登录策略读取同一目录。请求夹具在两个实际工厂各抓取 5 次本地 HTTP 请求，验证三条路径、企业端点前缀、Bearer 与 Copilot 请求头，并检查 Terra 和 Nano 的三种 effort 投影。
- 重新从旧 SDK 与旧 CLI 的记录计算下线集合，确认 20 个 ID。它们在目录、静态列表和两条运行查询路径均不可选，`prepare_call` 返回 `UnknownModel`，本地服务收到零次请求。把这 20 个 ID 和 Terra 同时写入账户可用名单后，SDK 只返回 Terra。非交互与 RPC 复用交互模式的启动校验，使用已下线默认 ID 时在创建 runner 前返回配置错误。
- 已通过 `provider_collection` 118 项、`catalog_personalization` 5 项、`model_wire_metadata` 24 项和 `catalog_artifact` 14 项。CLI binary、`github_copilot_provider`、`list_models` 和 `provider_factory` 分别通过 37、13、14、58 项。改动涉及的 `oauth_auth` 4 项及 `doctor_cli` 1 项通过。
- 实际 `.crate` 的独立 `catalog-bundled` 消费通过，归档 SHA-256 为 `cc34c17af12052652361b08165fd65d8afa1560fae8f4c00e046b45287ec4b88`。格式、架构、测试清单、文档和 diff 检查通过。定向 Clippy 通过并报告 3 项既有库 lint。

同步后全库有 1,698 条，包含 1,586 条 chat、48 条 classifier 和 64 条 image。12 个 provider 的目录与固定发布数据一致。剩余 30 个 provider 有差异，待新增 69 条、撤下 55 条、更新 63 条。

Test impact — `add` 目录一致性、真实请求、下线集合及启动报错回归。 `update` 现有列表、工厂、OAuth 和诊断夹具。 `delete` 旧 Copilot 目录快照。 `retain` 授权、企业端点和请求头回归。
