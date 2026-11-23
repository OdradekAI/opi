# #222 让 builtin Radius 从已存和远端配置刷新目录

Status: done
Owner: Radius provider、Models refresh/store
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

Pi 1.5.0 的 `packages/ai/src/providers/radius.ts` 与 `radius-config.ts` 提供网关目录参考。其 revision 和文件摘要记录在验收回执中。opi 通过 builtin 工厂接入自身的认证、目录验证和 store 发布合同。

## 验收

- 默认 gateway 从 opi bundle 提供初始模型列表。自定义 gateway 以空目录注册 Pi Messages 路由，显式模型继续使用配置的 chat 路径。
- 选择 `radius` 并允许联网时，Models 使用当前的路由凭据向配置的 origin 请求 `GET /v1/config`。过期 OAuth 使用该 origin 的 token 端点。opi 在具备凭据时才请求目录，远端声明的 chat base URL 也须同源。
- 目录通过验证和 store ACK 后完整替换，包括空列表。离线刷新零凭据读取、零 HTTP 请求，从 store 恢复当前 gateway 的 builtin 发布。旧版未带 scope 的 native 记录可回读。
- 网关拒绝、无凭据、无效记录、重复模型或跨源 chat 地址时保留已知目录。取消等待中的请求保留先前文件字节和 revision。同一个 `ModelsStore` 的两个 clone 中，较晚发起的刷新先提交后，较早的结果为 `Superseded`。重开文件恢复获准版本。

Pi 历史 OAuth 凭据中的 `gatewayConfig` 属于跨产品导入数据。opi 的凭据格式无此字段，当前发布与恢复走 `ModelsStore`。

## 验证

改前实跑复现了自定义 gateway 继承静态模型的问题。最终复跑 `radius_builtin_catalog`、`catalog_lifecycle`、`catalog_operations`、`provider_collection`、`external_sdk`、`builtin_auth_ownership`、`chat_pi_messages`、`pi_messages_terminal` 和 `registry`，共 247 项通过，其中 8 项经真实 builtin 入口验证本地 gateway 和临时 store。另跑 Radius OAuth 26 项、单 `provider-pi-messages` 1 项、minimum 21 项和受影响目标的 Clippy。[验收回执](../evidence/issue-222-builtin-radius-catalog.json)

Test impact: add `radius_builtin_catalog`；update 工厂与外部 SDK 的 Radius fixture、目标清单和选测规则；retain 通用刷新合同测试。
