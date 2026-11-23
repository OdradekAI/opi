# #251 同步 MiniMax CN 目录与 Anthropic Messages 请求

Status: done

## 范围与固定基线

本票按固定 Pi 1.5.0 发布 artifact 同步 MiniMax CN 的聊天目录。参考
artifact 的 SHA-256 为
f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e。
SDK builtin、Models 列表、查询和运行路由继续读取同一份 opi 作者目录。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| MiniMax CN Anthropic Messages 聊天模型 | 3 → 4 | 1 | 0 | 3 |

新增 MiniMax-M3.1-Flash。MiniMax-M2.7、MiniMax-M2.7-highspeed 和
MiniMax-M3 的价格字段按固定目录更新，MiniMax-M3 的 maxTokens 从
512,000 更新为 524,288。MiniMax-M3.1-Flash 声明 adaptive thinking，
并将 XHigh 映射为 output_config.effort=xhigh。

MiniMax CN 使用现有 Anthropic Messages 适配器和 MINIMAX_CN_API_KEY
API key 认证。请求基址为 https://api.minimaxi.com/anthropic，实际路径为
/anthropic/v1/messages。本票不新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle 和固定 Pi 1.5 MiniMax CN 记录逐条
   相等。Catalog::builtin、Models 列表、可用列表和按 ID 查询均只暴露
   4 条固定记录，新模型可查询，adaptive metadata 完整保留。
2. **实际请求路径、认证和推理参数一致。** 回环服务验证 API key
   POST /anthropic/v1/messages、anthropic-version: 2023-06-01、模型 ID、
   max_tokens、thinking.type=adaptive 和 output_config.effort=xhigh。
   请求不发送 authorization 或 reasoning_effort。
3. **入口集合封闭。** 固定目录之外的 MiniMax-M2.5 不出现在内置目录、
   列表或查询中，prepare_call 在派发前返回 UnknownModel，回环服务收到
   零请求。build_catalog、归档 catalog consumer、目录差分和受影响回归通过。

## 实施结果

- MiniMax CN 作者源 SHA-256：
  33c15c71d4d98446e0d479650f6bc61f14a6da985807799d9bd1c57407a3b323
- 全库 bundle SHA-256：
  2b8d2816f710a8c8d9f14239072463b0581906d58f3589e6f55aae15645e5bfc
- manifest SHA-256：
  837da46ef441647e8286afebc59519a1fc17423b48da36ee74a8f13e0d3c31c2

固定 Pi 源码参考文件 SHA-256：

- .pi/pi-1.5.0/packages/ai/catalog/providers/minimax-cn.ts：
  eb17f5c6eeec6736640e3f2bcf9dd8ad57450557a19f40cc5458ced465b2f748
- .pi/pi-1.5.0/packages/ai/src/providers/minimax-cn.ts：
  ae3170234556ef355f7cffcfc666888992ae50caa8daf07352f22bac69cca0ba
- .pi/pi-1.5.0/packages/ai/catalog/providers/minimax.ts：
  0f3a5bb7b7669c25915bad364617b1c2dfdde50eb19df15d558368453a716566
- .pi/pi-1.5.0/packages/ai/src/api/anthropic-messages.ts：
  f1348f1919aa9c4d2ed69767e4f64727e29df0455e8998a7da55773197fce48b

固定目录全库比较结果为 1,712 条 opi、1,712 条参考记录，1,696 条完全
相等，9 条仅 opi，9 条仅参考，7 条共有记录仍有字段差异。MiniMax CN 已从
差异 provider 集合中移除，剩余 13 个 provider 有差异。

## 测试回执

- provider_collection::minimax_cn：3 项通过。
- provider_collection 全目标：156 项通过。
- catalog_artifact：14 项通过。
- catalog_operations：27 项通过。
- anthropic_fixtures：155 项通过。
- external_sdk：11 项通过。
- build_catalog：9 项通过。
- build_catalog check、归档 catalog consumer、目录差分、fmt、文档检查和
  架构边界检查：通过。

Test impact — add MiniMax CN catalog, request-path, authentication, adaptive
reasoning and closed entrypoint coverage; update the builtin catalog bundle;
retain Anthropic Messages, catalog and archive consumer regression; none
coding-agent entry.
