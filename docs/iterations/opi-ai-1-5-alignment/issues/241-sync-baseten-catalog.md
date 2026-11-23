# #241 同步 Baseten 直连目录与 OpenAI Completions 路由

Status: done

## 范围与固定基线

本票接续 [#240 Vercel AI Gateway](240-sync-vercel-ai-gateway-catalog.md)，只处理
`provider=baseten` 的 SDK 内置直连路由。Baseten 使用现有 OpenAI Completions
适配器和 `https://inference.baseten.co/v1`，不新增独立协议适配器，也不为已经下线的
模型保留兼容入口。coding-agent 没有 Baseten 内置注册，本票不扩展其产品范围。

三方同步使用当前工作树的 Pi 1.3 发布 bundle
`sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678`、固定的
Pi 1.5 发布文件
`sha256-f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`，以及
`crates/opi-ai/resources/catalog/providers/baseten.json`。候选零冲突，14 条发布记录由
22 条变更而来。

固定发布 artifact 是本票的权威输入。`.pi/pi-1.5.0` 源数据当前还含有
`moonshotai/Kimi-K3.1-Pro` 这一第 15 条记录，但它不在固定 Pi 1.5 发布文件中，
因此没有把这条源数据旁路带入 opi。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| OpenAI Completions 聊天 | 22 → 14 | 2 | 10 | 10 |

新增记录为 `deepseek-ai/DeepSeek-V4.1-Flash-1017` 和
`deepseek-ai/DeepSeek-V4.1-Flash-1017-Fast`。字段更新包括 DeepSeek 推理等级、
NVIDIA Nemotron 的 OpenAI 推理格式、GLM-5.3-Fast 的 Off 映射以及多条缓存价格。

## 三项验收条件

1. **目录和 SDK 入口一致。** 作者源、bundle 和固定 Pi 1.5 文件逐条同序相等。通过
   `Catalog::builtin`、注册后的 `Models` 总列表、按操作列表、按 ID 查询及可用列表
   核对 14 条聊天记录及其类型化元数据。`build_catalog check` 通过，其他 provider
   作者源保持字节不变。
2. **实际请求与 Pi 路由一致。** 通过内置 `Models` 只把 Baseten 的 origin 替换为
   本地 HTTP 服务，保留 `/v1` 路径。请求必须是 `POST /v1/chat/completions`，使用
   `Authorization: Bearer`，发送 `stream_options.include_usage`，并按发布记录验证
   DeepSeek XHigh 收敛到 `max`、Nemotron 的 `reasoning_effort=high`、GLM-5.3-Fast
   Off 省略 effort，以及 GLM-5.2 的 `chat_template_args.enable_thinking` 与 effort。
3. **逐个证明十个 ID 已撤下。** 下列模型在 catalog、总列表、可用列表和按 ID 查询中
   均不存在。正常 SDK 选择在 HTTP 发出前返回 `UnknownModel`，回环服务收到零个
   下线请求。

| 撤下 ID |
| --- |
| `deepseek-ai/DeepSeek-V4-Pro` |
| `moonshotai/Kimi-K2.5` |
| `moonshotai/Kimi-K2.6` |
| `moonshotai/Kimi-K2.7-Code` |
| `nvidia/Nemotron-120B-A12B` |
| `thinkingmachines/inkling` |
| `thinkingmachines/inkling-small` |
| `zai-org/GLM-4.7` |
| `zai-org/GLM-5` |
| `zai-org/GLM-5.1` |

## 实施结果

- Baseten 作者源 SHA-256 为
  `f48eb92f89b3546e91ed7855e1299bf7b32563c1080838541caf285f657bf54a`。全库 bundle
  SHA-256 为 `051968d5e703d9b7a2859e5df048beef17b52bea3ef7e19c1fe7570f6fffeefe`。
  `build_catalog check` 通过，Baseten 的作者源和 bundle 共 14 条记录逐条相等。
- 四次回环聊天请求均命中 `/v1/chat/completions` 并使用 Bearer 认证。DeepSeek
  XHigh 经 neutral simple clamp 后发送 `reasoning_effort=max`；Nemotron 发送
  `reasoning_effort=high` 且不发送 `chat_template_args`；GLM-5.3-Fast Off 不发送
  `reasoning_effort`；GLM-5.2 发送 `chat_template_args.enable_thinking=true` 与
  `reasoning_effort=high`。
- 十个下线 ID 均在内置 catalog、列表、查询和正常选择入口缺席，回环服务收到零个
  请求。Baseten 直连没有额外兼容行为。

固定 Pi 源文件用于行为核对的 SHA-256 如下：

- `.pi/pi-1.5.0/packages/ai/src/providers/baseten.ts`：
  `c3833be0c64c4d5c96810403e23e966bb779bf2c045619b6fbac18ff27ac92f5`
- `.pi/pi-1.5.0/packages/ai/src/providers/baseten.models.ts`：
  `3d322e618fa3999635218a4d38d3bb188470112b8413117a101962fbe15139bb`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-completions.ts`：
  `e765179e356b4828a38954cda4cec762a851bb1d3a8fa28cdaf05edd12ca5c8a`

## 测试回执

- `cargo test -p opi-ai --offline --locked --test provider_collection baseten`：3 项通过。
- `cargo test -p opi-ai --offline --locked --no-default-features --example build_catalog`：
  8 项通过，`build_catalog check` 通过。
- `catalog-bundled` 与 `provider-openai-chat` 的实际 `.crate` 外部消费者均通过，归档
  `opi-ai-0.8.3.crate` SHA-256 为
  `9428683d2ae8656f76835550d4bfe55a633816d59f3568c1407e1e2a6509bce8`。
- `opi-coding-agent` 的 `test_hermeticity` 保持既有 14 项通过、1 项静态守卫误报失败。
  失败仍来自 `provider_collection.rs` 中两个 OpenCode `opencode.ai` 前缀字面量，实际
  请求在发送前替换为回环 origin，与本票新增 Baseten 测试无关。
- 本票新增 `provider_collection/baseten.rs`，覆盖目录入口、四条实际请求和十条下线
  选择。既有 OpenAI Completions、catalog、认证和归档消费者测试保持通过。

Test impact — `add` 三个 Baseten 目录、请求和下线回归；`retain` 共享 OpenAI
Completions 适配器与 catalog builder 测试；`none` 未变更独立 coding-agent 入口。
