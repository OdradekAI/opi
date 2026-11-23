# #245 同步 Radius 目录与 Pi Messages 动态目录

Status: done

## 范围与固定基线

本票只处理 `provider=radius` 的 SDK builtin 目录、Pi Messages 请求和已有动态目录刷新。
固定 Pi 1.5 发布 artifact SHA-256 为
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
模型集合以该 artifact 为准，Pi 源码用于核对 Radius baseline、动态 `/v1/config`
刷新和 `/v1/messages` 请求行为。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| Radius Pi Messages 聊天模型 | 29 → 28 | 3 | 4 | 9 |

新增记录为 `gpt-6.1-sol`、`minimax-m3.1-flash` 和 `recursive/flint`。
撤下记录为 `gpt-5.3-codex`、`gpt-5.4`、`gpt-5.4-mini` 和 `gpt-5.5`。
8 条 Claude 记录新增 prompt cache 声明，`claude-haiku-5-5` 和
`claude-opus-5` 的 off 等级按固定目录收敛。`glm-5.3` 的 Radius 上游声明和
推理等级映射同步更新。`recursive/flint` 的 `free=true` 属于 Radius 原始目录
元数据，authoring 校验器只在 Radius 记录上接受该布尔字段。

Radius 已有动态认证目录。默认 gateway 使用固定 baseline，认证后的 `/v1/config`
结果按现有合同替换 baseline。本票没有新增旧模型别名、参数回退或额外兼容路径。

## 三项验收条件

1. **目录和入口一致。** 作者源、bundle、manifest 和固定 Pi 1.5 Radius 记录逐条
   相等。`Catalog::builtin`、注册后的 `Models` 列表、可用列表、按 ID 查询和运行
   工厂均只暴露 28 个固定模型，并保留 Radius 的原始 provider、lab、enabled 和
   free 元数据。
2. **动态目录和实际请求一致。** 通过回环 Radius gateway 验证带 Bearer 的
   `GET /v1/config` 刷新和带 Bearer 的 `POST /v1/messages`。刷新后的新模型能够
   完成 Pi Messages SSE 请求，body 包含模型 ID、context、options 和 reasoning。
3. **入口集合封闭。** 4 个下线 ID 在 builtin 目录、列表、查询和运行派发前均不可见。
   未知 ID 返回 `UnknownModel`，回环服务收到零请求。`build_catalog check`、目录
   差分和归档 catalog consumer 通过。

## 实施结果

- Radius 作者源 SHA-256：
  `384ce93de730efa169485d3b511146a3081a62e2357d4bef86b6a41370f109a1`
- 全库 bundle SHA-256：
  `2c9c1d059fb7752c277054ebe6260454ef1569c5233bd3fc4301a6cda539ff96`
- manifest SHA-256：
  `d4185138a6e12a015c039f0de5e7e858333c9826fa7c3decfe290dcdefaaeac5`

固定 Pi 行为参考文件的 SHA-256：

- `.pi/pi-1.5.0/packages/ai/catalog/providers/radius.ts`：
  `0ee6db47ce94d1ff0f592dcdafc9f8e5c4cfb60f31a3cd8fb5b137e3826ddc91`
- `.pi/pi-1.5.0/packages/ai/src/providers/radius.ts`：
  `40acce75f4f5c99cbeb03247cbf0fbdde9e911b097af3c25132cdf1f75346c32`
- `.pi/pi-1.5.0/packages/ai/src/providers/radius-config.ts`：
  `0e8c55d29039d760ba4cb8a763e61ea742ac13b3acdf42891c82486a610ab4f4`
- `.pi/pi-1.5.0/packages/ai/src/api/pi-messages.ts`：
  `d521d2227b622fc624a75124c56c730c07a8d33a42b2f920a5dedc5a47ce9907`

## 测试回执

- `cargo test -p opi-ai --offline --locked --test provider_collection --features builtins radius`：3 项通过。
- 受影响回归 union：240 项通过，包含 provider collection 137、catalog artifact 14、catalog operations 27、Pi Messages 42、external SDK 11 和 build_catalog example 9。
- `build_catalog check`、Clippy 和归档 catalog consumer：通过。
- 格式、文档和架构边界检查在本票收尾阶段执行并记录在 evidence。

Test impact — `add` Radius 目录、动态刷新、Pi Messages 请求和入口集合回归；`update`
authoring validator、builtin catalog bundle；`retain` Pi Messages、Radius OAuth 和
catalog store 回归；`none` coding-agent 入口。
