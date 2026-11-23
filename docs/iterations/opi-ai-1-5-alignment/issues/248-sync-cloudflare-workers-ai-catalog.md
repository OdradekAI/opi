# #248 同步 Cloudflare Workers AI 目录与图片分类请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 `cloudflare-workers-ai` 的 19 条
聊天记录和 5 条分类器记录。参考 artifact 的 SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
作者源、bundle、Models builtin 入口和运行工厂均继续由同一份 opi 目录驱动。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Cloudflare Workers AI 聊天和分类器 | 21 → 24 | 3 | 0 | 5 |

新增聊天模型是 `@cf/deepseek-ai/deepseek-v4.1-flash-1017`。新增分类器是
`@cf/cloudflare/clef-omni` 和 `typesafe/jev-1.14.0`。聊天更新收敛
Gemma 4 和 Nemotron 3 的推理等级声明。分类器更新收敛 Clef、Clef Flash
和 Jev 的图片输入、价格与上下文窗口。

Pi 的 System One 合同要求 Cloudflare Clef 使用 `input.images` data URL
数组，Jev 使用 `assets`。因此 Rust 目录 wire codec 和共享 System One
transport 增加了 `ClassifierImageInput::Images`，只用于表达固定目录已经
声明的能力。没有添加旧模型别名、目录回退或其他兼容行为。本批固定目录
没有下线 ID，入口集合验收以 24 条闭合集合和未知 ID 派发前拒绝为准。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Cloudflare 记录逐条
   相等。`Catalog::builtin`、Models 聊天列表、分类器列表、可用列表和按 ID
   查询分别暴露 19 条聊天与 5 条分类器记录。
2. **实际请求路径、认证和参数一致。** 聊天回环服务验证 Bearer
   `POST /accounts/account-id/ai/v1/chat/completions`，模型和
   `max_completion_tokens` 由目录路由。分类器回环服务验证 Bearer
   `POST /accounts/account-id/ai/run`，Cloudflare envelope、`noul` 问题和
   `input.images` PNG data URL 均符合 Pi System One 请求形状。Jev 记录继续
   通过同一路径发送 `input.assets`，且两条 Cloudflare 图片路径都不发送
   TypeSafe 图片 opt-in 头。
3. **入口集合封闭。** 本批没有 Pi 下线 ID。合成未知聊天和分类器 ID 不会出现在
   内置目录、列表或查询中，派发前返回 typed failure，回环服务收到零请求。
   `build_catalog`、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Cloudflare 作者源 SHA-256：
  `27aca4d12011bca9212eefc81815b63fa16cc458f14299c731e4a9f693d70244`
- 全库 bundle SHA-256：
  `497be764242174191d94ab1c463ff187c145c5c638c137b9b9a5496b0cbca87b`
- manifest SHA-256：
  `88b316b8ae826a3e93e44584003cc4d938518dd2883d3fce679ac8a02b465fb3`

固定 Pi 源码参考文件 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/cloudflare-workers-ai.ts`：
  `384632bc3a320df2c14664d226acaf7301e9d6731e34329b560dd2e39f22c1a5`
- `.pi/pi-1.5.0/packages/ai/src/providers/cloudflare-workers-ai.ts`：
  `ad4d97721b01048d10b3e7ea6d2d0755dcd087f6623c1497c6f06c70c8511711`
- `.pi/pi-1.5.0/packages/ai/src/api/cloudflare-workers-ai-system-one.ts`：
  `f8bf74222a675fee5c6bdcee8ed70c0be006165417940cae47098f650f2ae900`
- `.pi/pi-1.5.0/packages/ai/src/api/system-one-shared.ts`：
  `4be07d266a37a2906adb8d6776a3e4fff4c100a199eb677e3b500b57fef7400f`

固定目录全库比较结果为 1,707 条 opi、1,712 条参考记录，1,683 条完全相等，
10 条仅 opi，15 条仅参考，14 条共有记录仍有字段差异。Cloudflare 已从差异
provider 集合中移除，剩余 16 个 provider 有差异。

## 测试回执

- `provider_collection::cloudflare_workers_ai`：4 项通过。
- `provider_collection` 全目标：147 项通过。
- `classifier_contract`：84 项通过。
- `catalog_artifact`：14 项通过。
- `catalog_operations`：27 项通过。
- `openai_chat_fixtures`：116 项通过。
- `external_sdk`：11 项通过。
- `build_catalog` example：9 项通过。
- `build_catalog` check、归档 catalog consumer、目录差分、fmt、文档检查、架构边界检查：通过。

Test impact — add Cloudflare catalog, `images` System One codec and chat/classifier
wire coverage; update builtin catalog bundle and catalog authoring validation;
retain shared classifier, catalog, OpenAI wire and archive consumer regression;
none coding-agent entry.
