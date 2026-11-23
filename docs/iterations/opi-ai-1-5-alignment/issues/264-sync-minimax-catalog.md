# #264 同步 MiniMax 目录与 Anthropic Messages 请求

Status: done

本票按固定 Pi 1.5.0 artifact（`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`）
同步 MiniMax 的 SDK 内置聊天目录。固定目录从 3 条增加为 4 条，新增
`MiniMax-M3.1-Flash`，并将 `MiniMax-M3.maxTokens` 从 512000 修正为 524288。
Pi 的 MiniMax 入口使用 API key Anthropic Messages 协议，因此本批沿用
`x-api-key`，没有改成 Bearer。

## 三项验收条件

1. 作者源、bundle、Catalog、Models 列表、可用列表、按 ID 查询和运行工厂逐条
   暴露固定 4 条 Anthropic Messages 记录，新增模型的 adaptive thinking、价格、
   上下文和输出上限与发布目录一致。
2. 回环服务验证 `POST /anthropic/v1/messages`、`x-api-key`、
   `anthropic-version=2023-06-01`、`max_tokens`，以及
   `thinking.type=adaptive` 和 `output_config.effort=xhigh`。
3. 目录外模型在所有入口返回 `UnknownModel`，派发前不产生请求。

## 实施结果

- 作者源：`e6beb677e25634658cc2e1b98444739a48fc5fd3c49d1e6cd3fb13864d2f9e7e`
- 全库 bundle：`3b2481f38cf5e0532d12962b25617aee9458ea7463c83aaf8a34af99c5af277e`
- manifest：`276f2ae1c5d45b0ca6a12a8e686691adb5f8bb7211adf1f6a085b394edaa6672`
- 固定 Pi 源：catalog `0f3a5bb7b7669c25915bad364617b1c2dfdde50eb19df15d558368453a716566`，
  provider `d571a6a8b5a47daf40b034a2dae4ac902a252fbd44b073c4caee8eeb1d7035b5`，
  Anthropic adapter `f1348f1919aa9c4d2ed69767e4f64727e29df0455e8998a7da55773197fce48b`，
  data `1e0459dda3858ea24151182106fc4179b95f43a0a8dffc1c3d2e7db4c2d9206e`。
- 全库目录差分已归零：1,712 条 opi、1,712 条参考，1,712 条相等，0 条新增、下线或字段差异。

## 测试回执

- `provider_collection::minimax`：3 项通过。
- 全量 provider_collection、catalog artifact、catalog operations、OpenAI chat fixtures、
  build_catalog、build_catalog check、归档 consumer、pack、fmt、文档和架构边界检查通过。
- clippy lib 通过，保留 3 个既有警告。

Test impact — `add` MiniMax 目录、API-key Anthropic adaptive 请求和新增模型回归；
`update` bundle、M3 输出上限和双语 provider 文档；`retain` Anthropic Messages、
catalog 和归档消费者回归；`none` 兼容行为和旧模型别名。
