# #249 同步 Moonshot AI CN 目录与 OpenAI Completions 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 moonshotai-cn 的聊天目录。
参考 artifact 的 SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
SDK builtin、Models 列表、查询和运行路由继续读取同一份 opi 作者目录。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Moonshot AI CN OpenAI Completions 聊天模型 | 4 → 6 | 2 | 0 | 3 |

新增 kimi-k3.1-pro 和 kimi-k3.1-pro-highspeed。三条既有 K2.6、
K2.7 Code 和 K2.7 Code HighSpeed 记录只更新 Pi 发布目录中的价格字段。
新增 K3.1 Pro 记录使用 max_tokens、OpenAI 风格推理映射和
thinkingLevelMap.high = high、max = max。

Moonshot AI CN 使用现有 OpenAI Completions 适配器和
MOONSHOT_API_KEY Bearer 认证。本票不新增旧模型别名、参数回退或额外
兼容路径。固定目录没有下线 ID，入口集合验收以 6 条闭合集合和未知 ID
派发前拒绝为准。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 Moonshot AI CN
   记录逐条相等。Catalog::builtin、Models 列表、可用列表和按 ID 查询
   均只暴露 6 条固定记录，两个新增 ID 均可查询。
2. **实际请求路径、认证和推理参数一致。** 回环服务验证 Bearer
   POST /v1/chat/completions。两个 K3.1 Pro 模型发送所选模型 ID、
   max_tokens 和 reasoning_effort=high，不发送
   max_completion_tokens。
3. **入口集合封闭。** 本批没有 Pi 下线 ID。合成未知 ID 不会出现在内置
   目录、列表或查询中，prepare_call 在派发前返回 UnknownModel，回环服务
   收到零请求。build_catalog、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- Moonshot AI CN 作者源 SHA-256：
  29348f6a1e29a46a15eb90dbab9eeea0f393798bd79ddaa894de37b505757c60
- 全库 bundle SHA-256：
  d39291fc45eb4b77803d08bb9fd7cc8f9040fd0b3e8e606a57711ae9cbea33b3
- manifest SHA-256：
  48d8449ce1913cf908cf7d2db057c92d8206cb20dd7696070f970614d72a7b7f

固定 Pi 源码参考文件 SHA-256：

- .pi/pi-1.5.0/packages/ai/catalog/providers/moonshotai-cn.ts：
  5437054d5bc6ce74ada782051a6989073015fe4619da28407cba910d4cd2c9fa
- .pi/pi-1.5.0/packages/ai/src/providers/moonshotai-cn.ts：
  da8ec0ed30a2cf42972cead61a93b1fcb64380095f9d062ee4ed9e4807ebd5bc
- .pi/pi-1.5.0/packages/ai/src/providers/moonshotai.ts：
  2e269cca0231ebdb3d2a58848ab3d19b0a43697e132b48a679e3c1b0cd4897665
- .pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts：
  e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a

固定目录全库比较结果为 1,709 条 opi、1,712 条参考记录，1,688 条完全相等，
10 条仅 opi，13 条仅参考，11 条共有记录仍有字段差异。Moonshot AI CN 已从
差异 provider 集合中移除，剩余 15 个 provider 有差异。

## 测试回执

- provider_collection::moonshotai_cn：3 项通过。
- provider_collection 全目标：150 项通过。
- 其他受影响的 catalog、OpenAI wire、external SDK 和 build_catalog
  回归沿用本轮固定检查清单。
- build_catalog check、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。

Test impact — add Moonshot AI CN catalog, request-path, reasoning and closed
entrypoint coverage; update the builtin catalog bundle; retain OpenAI
Completions, catalog and archive consumer regression; none coding-agent entry.
