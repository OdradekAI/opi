# #214 迁出 lower Provider 中的 Copilot 请求头规则

Status: done
Owner: 三个 chat wire 的共享 header 策略
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: [#213](213-measure-lower-layer-dependencies.md)

provider.rs 的 github_copilot_route_headers 与 github_copilot_initiator 被 Anthropic、OpenAI Chat 和 Responses 三处 wire 调用。

## 验收

两个规则由一个供三种 wire 复用的 API 实现拥有。原认证来源、优先级、initiator 分类及失败阶段保持兼容。门禁验证 `provider.rs` 中的两条规则均已迁出，更新 #213 的调用链记录。

## 验证

以真实工厂的 `cargo test --locked --offline -p opi-coding-agent --test github_copilot_provider` 核对 Authorization、静态头、`Openai-Intent`、视觉头和 initiator。这个目标已捕获 Anthropic、Chat、Responses 三种请求。再选三个 adapter 的直接 fixture、SDK 装配的相关用例与独立 feature，核对 protected header 拒绝的阶段。各组记录实际命中数。

[#213 的调用链复核](213-measure-lower-layer-dependencies.md)显示，CLI 工厂显式启用三种 wire 的 Copilot 请求头，SDK builtin 工厂保留默认关闭的 flag。单独记录这个行为差异。若后续统一两种装配，以真实 `register_builtin` 请求捕获作为独立验收。

## 实施与复核 · 2026-11-20

`api::shared::copilot_headers` 统一生成 `Openai-Intent`、按图片内容生成视觉头，并根据最后一条消息生成 `X-Initiator`。三个 adapter 在原来的头部合并位置调用同一个函数，沿用各自的 auth、affinity 和合并顺序。共享实现仅在三个消费者任一 feature 启用时编译。`Request::contains_image_content` 同时供公共请求能力验证使用，属于 Provider 请求合同。

原 CLI 工厂测试通过 13 项。扩展后在改前和改后都通过同一组 14 项。这组测试经实际工厂捕获三个协议的 URL、Bearer 凭据、静态头、intent、视觉与 initiator。三种协议各尝试覆盖 7 个受管头，共 21 种输入。每种输入均返回原 `RequestFailed` 原因和诊断，且 header transform 调用数与 HTTP 请求数均为 0。

| SDK 目标 | 通过 | 覆盖点 |
| --- | ---: | --- |
| `anthropic_fixtures` | 2 | Copilot affinity、受管 auth 与普通头覆盖 |
| `openai_chat_fixtures` | 2 | profile 头发送、native 与 simple 路径 |
| `openai_responses_fixtures` | 2 | Copilot 路由选项、回调前的 affinity 合并 |
| `provider_collection` | 2 | 三种 wire 的来源凭据、内置装配的头优先级 |
| `oauth_flows` | 1 | OAuth store 写入后实际 Chat 请求 |

底层门禁明确跟踪两个原函数名和受管头常量。生产定义从 3 处降为 0 处，临时恢复这些符号会使 CLI 退出 1。#213 的直接依赖配额仍按原来的符号边统计。SDK builtin 的 Copilot 配置仍是单独的接入验收项。

关闭默认 feature 后，分别运行 `provider_anthropic_profile`、`provider_openai_chat_profile`、`provider_openai_responses_profile`，每个目标通过 1 项。minimum 库检查通过。Codex 独立配置同样通过 1 项，覆盖使用 Responses 共享实现且无需 Copilot 策略的消费者。单开内部 `provider-openai-responses-shared` 时，改前改后的日志逐字节相同地报告缺少 `crate::http` 和 `reqwest`。两组受影响目标的普通 Clippy 完成，仍报告已记录的 3 项库警告。[精确用例、feature 和门禁回执](../evidence/issue-214-copilot-header-ownership.json)

Test impact: add 三种 wire 的受管头阶段回归与门禁负例。update CLI header 测试。retain 原请求与凭据断言。
