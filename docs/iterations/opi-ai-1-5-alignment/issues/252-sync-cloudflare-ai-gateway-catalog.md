# #252 同步 Cloudflare AI Gateway 目录与三条请求路径

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 Cloudflare AI Gateway 的聊天目录。
参考 artifact 的 SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
SDK builtin、Models 列表、查询和运行路由继续读取同一份 opi 作者目录。

此前 opi 目录有 57 条记录，固定 Pi 1.5 artifact 实际包含 59 条记录。
本批以固定 artifact 为权威，新增两条记录，不保留旧目录数量限制。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Cloudflare AI Gateway 聊天模型 | 57 → 59 | 2 | 0 | 1 |

新增 gpt-6.1-terra 和
workers-ai/@cf/deepseek-ai/deepseek-v4.1-flash-1017。更新
claude-sonnet-5-5 的 cacheRead 价格。固定目录包含 14 条 Anthropic
Messages、19 条 OpenAI Completions 和 26 条 OpenAI Responses 记录。

Cloudflare Gateway 认证使用 cf-aig-authorization，并从
CLOUDFLARE_API_KEY、CLOUDFLARE_ACCOUNT_ID 和 CLOUDFLARE_GATEWAY_ID
解析凭据和端点。实际请求路径分别是
/anthropic/v1/messages、/compat/chat/completions 和 /openai/responses。
本票不新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Cloudflare AI
   Gateway 记录逐条相等。Catalog::builtin、Models 列表、可用列表和按 ID
   查询均只暴露 59 条固定记录，三种 API 数量分别为 14、19 和 26。
2. **三条实际请求路径、认证和推理参数一致。** 回环服务验证
   Anthropic Messages、OpenAI Completions 和 OpenAI Responses 三条请求
   路径及 cf-aig-authorization。gpt-6.1-terra 发送 reasoning.effort=xhigh，
   DeepSeek 发送 max_tokens 且省略 max_completion_tokens 和
   reasoning_effort，claude-sonnet-5-5 发送 adaptive thinking 和
   output_config.effort=xhigh。
3. **入口集合封闭。** 固定目录之外的合成模型不出现在内置目录、列表或查询
   中，prepare_call 在派发前返回 UnknownModel，回环服务收到零请求。
   build_catalog、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Cloudflare AI Gateway 作者源 SHA-256：
  71173aff37abe3e3c02e5eba8e97f48a6f834e4eda2b1e747a1b55ea4879c869
- 全库 bundle SHA-256：
  36b74f8a812872c0578229621758d53b798d1c2591fdd0c33149a23c3e168a1e
- manifest SHA-256：
  38b21667128e8dfa210020a6972134eae5ffb645c996672cd591d072e30ef711

固定 Pi 源码参考文件 SHA-256：

- .pi/pi-1.5.0/packages/ai/catalog/providers/cloudflare-ai-gateway.ts：
  ed132820ffd8f67bb1573c68384219aa561db4cee52f422e09b18589b5c6f00b
- .pi/pi-1.5.0/packages/ai/src/providers/cloudflare-ai-gateway.ts：
  6f19bdcb965750aa1367f1c09eb883da16526644ad3a3cbcb60da8b53ca418ff
- .pi/pi-1.5.0/packages/ai/src/providers/cloudflare-auth.ts：
  d0532efb4f00126286d71dbce6fcb95e581cd9477467dc4bd20f1df1a466f03e
- .pi/pi-1.5.0/packages/ai/src/api/cloudflare.ts：
  08b69898fff115d1cd11339feb12ba35b3694f879e278838f3a13f6b22594d1e

固定目录全库比较结果为 1,714 条 opi、1,712 条参考记录，1,699 条完全
相等，9 条仅 opi，7 条仅参考，6 条共有记录仍有字段差异。Cloudflare AI
Gateway 已从差异 provider 集合中移除，剩余 12 个 provider 有差异。

## 测试回执

- provider_collection::cloudflare_ai_gateway：3 项通过。
- provider_collection 全目标：159 项通过。
- catalog_artifact：14 项通过。
- catalog_operations：27 项通过。
- anthropic_fixtures：155 项通过。
- openai_chat_fixtures：116 项通过。
- openai_responses_fixtures：84 项通过。
- external_sdk：11 项通过。
- build_catalog：9 项通过。
- build_catalog check、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。

Test impact — add Cloudflare AI Gateway catalog, three-wire request-path,
authentication and reasoning coverage; update the builtin catalog bundle;
retain Anthropic, OpenAI, catalog and archive consumer regression; none
coding-agent entry.
