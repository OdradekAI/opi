# 智谱国内 provider 端点与本地评测配置核查

日期：2026-09-05。

状态：非规范性外部研究证据；不构成 Phase 19 规范、准入或真实 provider 验收。仅核查国内 `docs.bigmodel.cn` / `open.bigmodel.cn` 文档，不采用国际 Z.AI 账号、价格或 endpoint 规则。

语言：本报告响应本次中文规划讨论，没有既有英文对应文档；作为一次性 outward research，本次不新增英文副本。

## 1. 用户输入与已核实事实

用户提供了三个候选 Base URL；**三个地址及其协议映射均与智谱中文官方文档一致**。用户随后明确提供精确模型编码 **`glm-5.3-flash`**；凭据、账号产品类型和计费权益未提供。本报告保留用户的模型选择，不推断用户购买了任何套餐。[官方编程端点表](https://docs.bigmodel.cn/cn/coding-plan/tool/others#二、编程端点)、[Coding Plan 快速开始](https://docs.bigmodel.cn/cn/coding-plan/quick-start)

| 用户提供的 Base URL | 官方协议定位 | 完整请求路径的核实程度 |
|---|---|---|
| `https://open.bigmodel.cn/api/anthropic` | Anthropic Messages 兼容 | 官方 cURL 直接给出 `POST https://open.bigmodel.cn/api/anthropic/v1/messages`，已核实。|
| `https://open.bigmodel.cn/api/coding/paas/v4` | OpenAI Chat Completions 兼容、Coding 专用端点 | 协议与 Base URL 已核实。预期完整路径为 `/api/coding/paas/v4/chat/completions`；这是依据官方兼容声明及 Chat Completions 路由拼接的推导，本次未在抓取的官方页面找到该完整 Coding URL 的原始 HTTP 示例。|
| `https://open.bigmodel.cn/api/v1` | OpenAI Responses 兼容 | 官方 Codex 接入页明确指定该 Base URL，且 `wire_api` 必须为 `responses`。预期完整路径为 `/api/v1/responses`；本次未找到国内官方该路径的独立原始 HTTP API 示例，不能把配置文档当作已执行请求证据。|

主源：[Claude API 兼容说明](https://docs.bigmodel.cn/cn/guide/develop/claude/introduction)、[编程端点表](https://docs.bigmodel.cn/cn/coding-plan/tool/others)、[Codex 接入页](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)。

本报告没有对上述 API 地址发送任何模型请求。Base URL 与完整 request URL 必须在配置和本地 fake-server 测试中区分；检查 agent 客户端是否追加 `/v1/messages`、`/chat/completions` 或 `/responses`，避免重复追加版本段或把三个协议混用。此项是集成建议，不是现有 Opi/pi 行为结论。

## 2. 鉴权与模型编码

### Anthropic Messages

智谱官方原始 cURL 示例使用 `x-api-key: YOUR_API_KEY`，Anthropic SDK 示例使用智谱 API key 和用户提供的 Base URL。Claude Code 工具接入示例则使用 `ANTHROPIC_AUTH_TOKEN` 配合 `ANTHROPIC_BASE_URL`。因此不能把某个工具的环境变量名当成所有 Anthropic-compatible 客户端的唯一鉴权方式；Opi/pi adapter 应采用其实际客户端支持的方式。[原始 API 示例](https://docs.bigmodel.cn/cn/guide/develop/claude/introduction)、[Claude Code 配置](https://docs.bigmodel.cn/cn/coding-plan/tool/claude)

上述原始 cURL 没有列出 `anthropic-version` header；本次没有核实智谱对此 header 的必需性、可选性或默认行为，不据此声称应该删除客户端正常发送的 header。

### Chat Completions 与 Responses

智谱通用 HTTP 文档明确列出 `Authorization: Bearer YOUR_API_KEY` 与 JSON content type，通用完整路径为 `https://open.bigmodel.cn/api/paas/v4/chat/completions`。Coding 接入页面要求给 OpenAI Compatible 客户端配置 API Key 与专用 Base URL；将同一种 Bearer 鉴权用于 Coding Chat 路径是兼容接口配置层面的推导，尚未做真实请求验证。[HTTP API 文档](https://docs.bigmodel.cn/cn/guide/develop/http/introduction)、[Coding OpenAI Compatible 示例](https://docs.bigmodel.cn/cn/coding-plan/tool/others)

Responses 官方 Codex 示例使用 `experimental_bearer_token` 保存智谱 API Key，并要求 `wire_api = "responses"`。这证明官方给出的客户端 Bearer 配置方式；本次没有独立的国内 Responses 原始 HTTP 请求样例，不额外声称已核实全部 header、SSE event、tool-call 或 reasoning 字段兼容性。[Codex 配置](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)

### 模型与 provider 身份

- API 中的 `model` 应使用智谱模型编码；不能把 `anthropic` / `openai` 协议名字当成实际模型供应商，也不能自动保留 Claude/GPT 模型编码。官方 Claude 迁移说明明确要求调用智谱模型编码。[Claude 兼容迁移](https://docs.bigmodel.cn/cn/guide/develop/claude/introduction)
- 如果 harness 使用 `provider/model` 作为其内部选择语法，应由 adapter 按实际 Agent 配置解析；不能据此声称智谱 wire payload 需要同样的前缀。
- Claude Code 文档提供了 Opus/Sonnet/Haiku 到 GLM 模型的环境变量映射；Codex 文档要求模型在其模型目录中声明。这些是工具配置规则，不是三个 endpoint 自动使用同一个后端模型的证明。[Claude Code](https://docs.bigmodel.cn/cn/coding-plan/tool/claude)、[Codex](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)
- 智谱已提供 earendil Pi Coding Agent 的国内接入说明：使用 `zai-coding-cn`，或者 `ZAI_CODING_CN_API_KEY` 环境变量，再选择对应 GLM 模型。该文档指向最新 Pi；不自动证明本项目固定的 pi 版本具备同一内置 provider，固定版本事实应另外查其源码。[Pi 官方接入页](https://docs.bigmodel.cn/cn/coding-plan/tool/pi)

本次不把官方页面示例中的其他 GLM 型号替换为用户实验默认值。用户指定的 `glm-5.3-flash` 原样保留；本次有限端点调查没有独立核实该模型在三个接口上的可用性、账号权限、推理参数、上下文长度和价格，也没有证明三个协议对同一模型完全等价。主代理已说明首轮按先前建议以 Anthropic Messages 编写方案；该规划假设不是一次已通过的模型兼容性验收。

## 3. Coding Plan 与标准 API 的区别

### 已明确的官方条款

官方工具接入页对适用范围的原文短摘录为：

> “仅限在以下官方支持的指定工具与产品环境中使用”

该段下一句明确说订阅权益不得用于所列范围以外的工具或场景。因此，该页按其文字是带限制的范围表，不是随意举出的若干例子；其中明确列有 Pi Coding Agent。官方 FAQ 又说明，自建应用、网站、机器人、SaaS 等 API 集成场景应使用标准 API，按对应协议计费。[适用工具原文](https://docs.bigmodel.cn/cn/coding-plan/tool/others#一、适用工具)、[FAQ 套餐详情](https://docs.bigmodel.cn/cn/coding-plan/faq#一、套餐详情)

但本次查询没有找到明确讨论 **Opi、opi-eval、Terminal-Bench、DeepSWE 或自动 benchmark** 的官方资格条文。不能仅凭列表缺少 Opi 就断言所有 Opi 使用均违法或技术上不可用，也不能把“Pi 在列表内”外推为任意自动批量 benchmark 都享有套餐额度。Opi 或评测场景是否属于受支持环境、用户现有账号是否有额外约定，属于供应商或账号合同需要确认的事项。这是费用和权益身份边界，不自动否定用户的技术方案，也不阻止无凭据开发、离线验证或其他已授权工作。

### 端点与费用不能静默替换

| 事项 | 官方说明 | 对评测配置的含义 |
|---|---|---|
| 标准 API | 通用 Base URL 是 `https://open.bigmodel.cn/api/paas/v4/`。 | 它与用户提供的 `/api/coding/paas/v4` 不同；不能因请求失败而自动切过去。|
| Coding 套餐 | 使用专属 Coding endpoint；错配 Base URL 可能无法抵扣套餐。 | HTTP 协议兼容不等于计费身份相同；应记录 endpoint 和用户明确的产品类型。|
| 套餐额度耗尽 | FAQ 表明不会继续消耗其他资源包或账号余额，需要等待恢复。 | 不应实现隐式标准 API fallback；rate limit 或额度耗尽是可报告的实验失败/暂停原因。|
| 套餐以外集成 | 官方 FAQ 区分标准 API 计费与套餐适用场景。 | 不因为用户提供了 Coding URL 就假定已具备套餐资格或余额。|
| Key 类型 | 当前个人套餐文档从个人套餐概览创建 key；团队套餐文档说明团队套餐 key 与其他平台 API key 不通用。 | 凭据来源必须由用户本地明确；不能将团队 key、普通平台 key 或国际 key 混用。|

主源：[HTTP API 基础信息](https://docs.bigmodel.cn/cn/guide/develop/http/introduction)、[Coding Plan FAQ](https://docs.bigmodel.cn/cn/coding-plan/faq)、[Coding Plan 快速开始](https://docs.bigmodel.cn/cn/coding-plan/quick-start)。

另一个容易混淆的事实：`/api/anthropic` 同时出现在 Coding Plan 文档和通用 Claude API 兼容文档中；后者引导到普通开放平台 API key 页面。因此，仅凭该 URL 不能确定账户结算方式。`/api/v1` 在本次查到的国内官方资料中主要由 Codex Coding Plan 接入页解释；没有核实它对普通标准 API 账号的资格、独立价格或全部模型支持情况。不能猜测统一计费，也不能猜测一律只能用于套餐。[Claude API 兼容](https://docs.bigmodel.cn/cn/guide/develop/claude/introduction)、[Codex Coding Plan](https://docs.bigmodel.cn/cn/coding-plan/tool/codex)

## 4. 对 Phase 19 讨论的有限建议

1. 把这三个地址保存为用户选择的国内智谱协议候选；每次实验显式选择一个 `api_kind`、Base URL 和精确 model ID。
2. 使用本地 credential reference，报告只显示引用名及是否配置；不要要求用户把 key 发到对话或写入仓库。
3. 用本地 fake server 验证 Opi/pi 的实际 request path、鉴权注入和请求体映射；这不需要访问智谱，且能够暴露重复路径等确定性错误。
4. 用户提供模型与凭据后，再以明确预算做真实 provider preflight。记录实际模型、端点、stream/tool-call 兼容结果；不要把“鉴权成功”或“普通文本成功”当作完整 Agent 工具循环已兼容。
5. 套餐权益或标准 API 产品选择与技术协议分开处理。需要供应商澄清的资格事项作为显式未核实项；不伪装成已通过的 preflight，也不自动切换费用路径。

这些是研究建议，尚未进入正式实现准入，也不声明已有运行能力。

## 5. 取证范围与限制

通过已加载的 research、agent-reach 网页路由与 Firecrawl 流程，只搜索和读取智谱中文官方网页；Firecrawl search/scrape 结果暂存在系统临时目录 `.firecrawl`，不进入仓库。搜索结果用于定位后，引用实际读取的官方页面。读取过程中发现 FAQ 的旧工具 URL 说明没有覆盖新的 Responses 行，具体 Responses 配置采用更窄的 Codex 接入页为依据。

没有访问用户账户页面，没有读取 API key、`.env` 或其他凭据，没有给 provider 发请求，没有启动 WSL/Docker，没有购买套餐，没有修改 Agent 全局配置。官网可变文档代表本次查询时状态，不是固定版本协议保证。未核实的事项已在对应章节逐项列出，未扩大到国际 Z.AI 规则。
