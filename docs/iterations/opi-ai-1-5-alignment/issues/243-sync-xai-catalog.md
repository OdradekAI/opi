# #243 同步 xAI 目录与 Responses 推理映射

Status: done

## 范围与固定基线

本票只处理 `provider=xai` 的 SDK builtin 目录和 OpenAI Responses 请求验证。
固定 Pi 1.5 发布 artifact SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型集合以该 artifact 为准，Pi 源码只用于核对 Responses 行为。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| xAI 聊天模型 | 4 → 4 | 0 | 0 | 2 |

两项更新为 `grok-4.3` 与 `grok-4.5` 的
`thinkingLevelMap.xhigh = "xhigh"`。本票不引入固定 artifact 之外的模型，
也不保留旧参数回退。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle、manifest 和固定 Pi 1.5 文件逐条相等。
   `Catalog::builtin`、注册后的 `Models` 列表、可用列表、按 ID 查询和运行工厂均只
   暴露 4 个固定模型，`grok-4.3` 与 `grok-4.5` 的 XHigh 映射为 `xhigh`。
2. **实际 Responses 请求一致。** 通过 builtin xAI 入口向本地回环服务发送
   `grok-4.3` 和 `grok-4.5` 请求，验证 `/v1/responses`、Bearer 认证、模型 ID、
   `reasoning.effort=xhigh` 和 `reasoning.encrypted_content`。
3. **入口集合封闭。** 固定发布目录没有新增或下线 xAI ID。未知旧 ID 在 HTTP
   派发前返回 `UnknownModel`，回环服务收到零个请求。`build_catalog check` 和归档
   catalog consumer 通过。

## 实施结果

- xAI 作者源 SHA-256：
  `a50749af1cff8fd1f68f1b7e20a124220ab739b3cfff97cb4fba65768590fcc3`
- 全库 bundle SHA-256：
  `57304daf0dccc4cce4e8ab9f24b9114c88a3d86ee5682da13f11b40a51e6a296`
- manifest SHA-256：
  `c4a70c05cd3332ed61c0c07cb06974bf99e6db8c535d06a516e41e1ec33e0d80`

固定 Pi 行为参考文件的 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/xai.ts`：
  `1c939eccaedbbf50b5f8ec9c1e86a83c3cb61bfce458e6afd40d1862385cbd08`
- `.pi/pi-1.5.0/packages/ai/src/providers/xai.ts`：
  `841c361fc5ebbb2d91c3f6aa46ce008f354ea287cce36ed2105ae815b7e5e9831`
- `.pi/pi-1.5.0/packages/ai/src/api/openai-responses.ts`：
  `20a2a355191df9c171e3e91c462fc771ff09cdfa40bf671eaf5a6cabd68b549b`

## 测试回执

- `cargo test -p opi-ai --offline --locked --test provider_collection --features builtins xai`：3 项通过。
- 受影响回归 union：`provider_collection` 131 项、`catalog_artifact` 14 项、
  `catalog_operations` 27 项、`openai_responses_fixtures` 84 项全部通过。
- `run_catalog_bundled_package.py`：实际归档包外部消费者通过。
- `cargo fmt --all`：通过。

Test impact — `add` xAI 目录、Responses 请求和入口集合回归；`update` builtin
catalog bundle；`retain` OpenAI Responses 通用回归和 xAI 加密推理行为；`none`
其他 provider 入口。
