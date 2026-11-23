# #268 按协议 seam 拆分超大 fixture target

Status: complete for the 49 fixture batches and structure closeout
Owner: opi-ai protocol and fixture tests
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

## 证据

拆分前的基线记录中，Bedrock fixture 420,281 字节、OAuth flow 309,117 字节、
Anthropic fixture 295,336 字节、provider collection 285,855 字节、Codex
Responses 204,537 字节。它们同时包含 request serialization、stream terminal、
auth source、retry 和 lifecycle 场景，失败定位与按改动选测仍依赖大文件内的模块名。

## 目标和范围

- 先按真实 seam 把 request、stream、auth/retry、lifecycle 和 fixture data 分成
  可定位的子模块或 target，不按文件大小机械删测。
- 每个子模块保留 owner、layer、cost 和已有 runner 映射，业务 wire 期望与稳定
  测试工具分开。
- 删除或合并前记录旧用例到替代断言的输入、时序、错误阶段和资源条件映射。

## 独立验收

1. 每个迁移主题能通过 target/selector 直接选中，失败输出不再要求运行同一协议
   的全部大文件。
2. `targets.tsv`、Cargo metadata、selector 和 package/differential runner 的
   映射一致，迁移前后具名用例都有去向。
3. 受影响 wire、lifecycle、archive 和 differential 检查通过，未授权删除唯一
   public/parser/terminal assertion。

## 实施批次

### 第一批 Bedrock

2026-11-22 先处理最大的 Bedrock fixture。原 `bedrock_fixtures` target 中的
Smithy retry、strict-tool admission、bounded rebuild、retry header 和 cancellation
用例已移入独立的 `tests/bedrock_retry.rs`，保留 17 个具名断言。原 target 继续拥有
Converse serialization、stream decoding、auth source 和 lifecycle fixture。Cargo
manifest 与 `tests/targets.tsv` 新增同名 `bedrock_retry` owner，两个 target 的
package consumer 都归 `provider-bedrock` feature owner。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `api/bedrock` wire fixture | `bedrock_fixtures` | 请求、事件流、认证来源、生命周期 | `provider-bedrock` |
| `api/bedrock` retry seam | `bedrock_retry` (`provider-bedrock`) | Smithy retry、strict-tool admission、取消与限额 | `provider-bedrock` |
| `provider-bedrock` feature profile | `provider_bedrock_profile` | provider public construction boundary | `provider-bedrock` |

第一批没有删除 public/parser/terminal assertion，也没有加入兼容路径。旧
`bedrock_fixtures` 仍保留一个 carried-in 失败
`bedrock_42::persisted_redacted_bytes_follow_source_forgiving_base64_acceptance`，
失败与 target 拆分无关，已在证据文件中单独记录。
完整回执见 [`268-bedrock-retry-fixture-20261122.json`](../evidence/268-bedrock-retry-fixture-20261122.json)。

### 第二批 OAuth

2026-11-22 第二批处理 OAuth flow。OpenRouter 的 callback、manual-code、
exchange、refresh identity 和 permanent-key 生命周期已移入独立的
`oauth_openrouter` target。共享 scripted presenter 和 Models route assembly
fixture 归入 `tests/support`，其余 provider flow 与跨 provider regression 仍由
`oauth_flows` 负责。旧文件从 309,117 字节降到 278,723 字节，新的 OpenRouter
target 为 22,488 字节，fixture 和 assembly support 各自有单独 owner。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter` | scripted presenter、callback URL 记录、manual/device reply 与 presenter ACK | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter` | Models OAuth route assembly request fixture 与 identity resolution | `oauth-store` |
| `auth/oauth/openrouter` fixture | `oauth_openrouter` (`builtins`) | OpenRouter callback 路由、code exchange、permanent key 和 identity refresh | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Anthropic、Codex、ChatGPT、Copilot、Kimi、Meta、xAI、Radius 与共享回归 | `oauth-store` |

OAuth 批次的三项验收均通过。`oauth_openrouter` 17 项、保留的
`oauth_flows` 178 项、`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和
`oauth_wire_shape` 12 项通过。inventory、Cargo target、selector 与两个 OAuth
package consumer 的 owner 映射一致。`oauth-store` 外部
consumer 的 feature union 现明确包含由 `http-transport` 传递的 `tls-backend`，
同时保留 catalog 和 cloud protocol owner 的排除断言。没有删除唯一的
callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。完整回执见
[`268-oauth-openrouter-20261122.json`](../evidence/268-oauth-openrouter-20261122.json)。

### 第三批 Radius

2026-11-22 第三批继续按 OAuth provider seam 拆分 Radius。Radius discovery、
browser/device login、token exchange、refresh 和 custom identity lifecycle 的 24 项
断言迁移到独立的 `oauth_radius` target。Radius 专属端口锁、wiremock mount、token
success/device-start fixture、callback driver 和 provider factory 归入
`tests/support/oauth_radius.rs`。共享 presenter、expiry 和 URL 解码仍由
`oauth_fixtures.rs` 拥有，Models route assembly 仍由 `oauth_assembly.rs` 拥有。
`oauth_flows` 保留 154 项跨 provider flow、共享 regression 和 metadata 断言。
旧 OAuth target 在 OpenRouter 批次后的 278,723 字节降到 237,362 字节，新的
Radius target 为 39,240 字节，Radius support 为 3,276 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius` | scripted presenter、expiry 检查和 callback URL 解码 | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius` | Models OAuth route assembly request fixture 与 identity resolution | `oauth-store` |
| `tests/support/oauth_radius.rs` | `oauth_flows`, `oauth_radius` | Radius 端口锁、discovery/token/device mount、callback driver 与 Radius factory | `oauth-store` |
| `auth/oauth/radius` fixture | `oauth_radius` (`builtins`) | Radius discovery、browser/device login、token exchange、refresh 与 custom identity | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | 其余 provider flow、共享 regression 与 metadata | `oauth-store` |

Radius 批次的三项验收均通过。`oauth_radius` 24 项、保留的 `oauth_flows` 154 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_radius` 可由 selector 单独选中，support selector 同时扩展到
`oauth_flows` 与 `oauth_radius`，inventory、Cargo target 与 package consumer owner
映射为 116/116 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-radius-20261122.json`](../evidence/268-oauth-radius-20261122.json)。

### 第四批 Anthropic

2026-11-22 第四批处理 Anthropic OAuth。browser/copy-code login、callback
validation、token exchange、refresh 和 preferred-port lifecycle 的 15 项断言迁移到
独立的 `oauth_anthropic` target。Anthropic 专属端口锁、端口占用重试、authorize
redirect、token mount 和 OAuth provider factory 归入
`tests/support/oauth_anthropic.rs`。通用的 `stored_credential` fixture 归入
`oauth_fixtures.rs`，供 Anthropic 与剩余共享 regression 使用。`oauth_flows` 保留
139 项其它 provider flow、Models store checks、共享 regression 和 metadata 断言。
旧 OAuth target 在 Radius 批次后的 237,362 字节降到 213,915 字节，新的
Anthropic target 为 20,469 字节，Anthropic support 为 2,873 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_anthropic.rs` | `oauth_flows`, `oauth_anthropic` | Anthropic 端口锁、端口占用重试、token mount、redirect helper 与 provider factory | `oauth-store` |
| `auth/oauth/anthropic` fixture | `oauth_anthropic` (`builtins`) | browser/copy-code login、callback validation、token exchange、refresh 与 preferred-port lifecycle | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Codex、ChatGPT、Copilot、Kimi、Meta、xAI、Radius 与共享 regression | `oauth-store` |

Anthropic 批次的三项验收均通过。`oauth_anthropic` 15 项、保留的 `oauth_flows` 139
项、`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape`
12 项通过。`oauth_anthropic` 可由 selector 单独选中，support selector 同时扩展到
`oauth_flows` 与 `oauth_anthropic`，inventory、Cargo target 与 package consumer owner
映射为 117/117 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-anthropic-20261122.json`](../evidence/268-oauth-anthropic-20261122.json)。

### 第五批 Codex

2026-11-22 第五批处理 OpenAI Codex OAuth。browser/device login、固定 1455
loopback fallback、token exchange、account-id extraction、refresh 和 Models store ACK
的 11 项断言迁移到独立的 `oauth_codex` target。Codex provider factory、JWT account
fixture、token/device wire mounts 和 presenter 归入 `tests/support/oauth_codex.rs`。
Codex 与 ChatGPT 共用的 1455 端口锁和占用 fixture 归入
`tests/support/oauth_loopback.rs`。`oauth_flows` 保留 128 项 ChatGPT、Copilot、
Kimi、Meta、xAI、Radius、Models store checks、共享 regression 和 metadata 断言。
旧 OAuth target 在 Anthropic 批次后的 213,915 字节降到 192,946 字节，新的
Codex target 为 15,574 字节，Codex support 为 3,572 字节，loopback support 为
833 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_loopback.rs` | `oauth_flows`, `oauth_codex` | 1455 loopback lock 与占用重试 fixture | `oauth-store` |
| `tests/support/oauth_codex.rs` | `oauth_flows`, `oauth_codex` | Codex factory、JWT account fixture、token/device mounts、presenter 与 form encoder | `oauth-store` |
| `auth/oauth/codex` fixture | `oauth_codex` (`builtins`) | browser/device login、fixed-loopback fallback、token exchange、account-id extraction、refresh 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | ChatGPT、Copilot、Kimi、Meta、xAI、Radius 与共享 regression | `oauth-store` |

Codex 批次的三项验收均通过。`oauth_codex` 11 项、保留的 `oauth_flows` 128 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_codex` 可由 selector 单独选中，Codex 与 loopback support selector 均
扩展到 `oauth_flows` 与 `oauth_codex`，inventory、Cargo target 与 package consumer
owner 映射为 118/118 一致。OAuth store package consumer 和 websocket/OAuth
shared-owner pair 均通过，没有删除唯一的 callback、parser、terminal 或 store ACK
断言，也没有增加兼容行为。完整回执见
[`268-oauth-codex-20261122.json`](../evidence/268-oauth-codex-20261122.json)。

### 第六批 ChatGPT

2026-11-22 第六批处理 OpenAI ChatGPT OAuth。dynamic client registration、browser
callback、manual paste validation、1455 occupied-port terminal、device identity、token
contract、refresh 和 Models store ACK 的 11 项断言迁移到独立的 `oauth_chatgpt` target。
ChatGPT provider factory、authorize/token wire mount、form decoder、callback driver 和
authorize presenter helper 归入 `tests/support/oauth_chatgpt.rs`。与 Codex 共用的 1455
loopback fixture 继续由 `tests/support/oauth_loopback.rs` 拥有，通用 presenter、expiry、
URL 解码和 stored credential 继续由 `oauth_fixtures.rs` 拥有，Models route assembly
assertion 继续由 `oauth_assembly.rs` 拥有。`oauth_flows` 保留 117 项 Copilot、Kimi、
Meta、xAI、Radius、Models store checks、共享 regression 和 metadata 断言。旧 OAuth
target 在 Codex 批次后的 192,946 字节降到 172,209 字节，新的 ChatGPT target 为
18,481 字节，ChatGPT support 为 3,069 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex`, `oauth_chatgpt` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex`, `oauth_chatgpt` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_loopback.rs` | `oauth_flows`, `oauth_codex`, `oauth_chatgpt` | 1455 loopback lock 与占用重试 fixture | `oauth-store` |
| `tests/support/oauth_chatgpt.rs` | `oauth_flows`, `oauth_chatgpt` | ChatGPT factory、authorize/token mount、form decoder、callback driver 与 presenter helper | `oauth-store` |
| `auth/oauth/chatgpt` fixture | `oauth_chatgpt` (`builtins`) | dynamic registration、browser callback、manual paste、device identity、token contract、refresh 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Copilot、Kimi、Meta、xAI、Radius 与共享 regression | `oauth-store` |

ChatGPT 批次的三项验收均通过。`oauth_chatgpt` 11 项、保留的 `oauth_flows` 117 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_chatgpt` 可由 selector 单独选中，ChatGPT、loopback 与 assembly support
selector 只扩展到实际消费者，inventory、Cargo target 与 package consumer owner
映射为 119/119 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-chatgpt-20261122.json`](../evidence/268-oauth-chatgpt-20261122.json)。

### 第七批 Copilot

2026-11-22 第七批处理 GitHub Copilot OAuth。device login、GitHub token exchange、
Copilot token exchange、model discovery、enterprise domain、policy activation、refresh、
Models store ACK 以及 source timeout clamp 的 22 项断言迁移到独立的 `oauth_copilot`
target。Copilot 专属 token/model wire mount、enterprise URL factory、presenter、TLS
inference peer 和模型 helper 归入 `tests/support/oauth_copilot.rs`。通用 presenter、
expiry、URL 解码、stored credential 和 assembly assertion 继续由对应 support owner
维护，TLS 证书材料继续由 `tests/support/tls_fixtures.rs` 作为共享 transport fixture
拥有。`oauth_flows` 保留 95 项 Kimi、Meta、xAI、Radius、Models store checks、跨 provider
regression 和 metadata 断言。旧 OAuth target 在 ChatGPT 批次后的 172,209 字节降到
129,738 字节，新的 Copilot target 为 36,162 字节，Copilot support 为 8,019 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_copilot.rs` | `oauth_flows`, `oauth_copilot` | Copilot factory、device/token/model mount、enterprise URL、presenter、TLS inference peer 与模型 helper | `oauth-store` |
| `tests/support/tls_fixtures.rs` | `bedrock_fixtures`, `http_acceptance`, `oauth_flows`, `oauth_copilot`, `tls_contract`, `ws_upgrade` | 共享本地 TLS 证书和信任根材料 | shared transport consumers |
| `auth/oauth/copilot` fixture | `oauth_copilot` (`builtins`) | device login、GitHub/Copilot token exchange、model discovery、enterprise URL、policy activation、refresh、source clamp 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Kimi、Meta、xAI、Radius、Models store checks、跨 provider regression 和 metadata | `oauth-store` |

Copilot 批次的三项验收均通过。`oauth_copilot` 22 项、保留的 `oauth_flows` 95 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_copilot` 可由 selector 单独选中，Copilot support selector 只扩展到
`oauth_flows` 与 `oauth_copilot`，TLS support selector 保留其全部实际 transport
消费者，inventory、Cargo target 与 package consumer owner 映射为 120/120 一致。
OAuth store package consumer 和 websocket/OAuth shared-owner pair 均通过，没有删除
唯一的 callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。完整回执见
[`268-oauth-copilot-20261122.json`](../evidence/268-oauth-copilot-20261122.json)。

### 第八批 Kimi

2026-11-22 第八批处理 Kimi Coding OAuth。device authorization、polling、slow_down、
strict token triple、refresh、cancellation、typed error mapping、expiry 和 Models store
ACK 的 24 项断言迁移到独立的 `oauth_kimi` target。Kimi 专属 device/token wire mount、
provider factory、token request collector 和 stored credential fixture 归入
`tests/support/oauth_kimi.rs`。通用 presenter、expiry、URL 解码和 Models route assembly
继续由对应 support owner 维护。`kimi_stored_credential` 仍由旧 `oauth_flows` 通过
同一 support 消费，用于跨 provider overflow 和共享 store regression。`oauth_flows`
保留 71 项 Meta、xAI、Models store checks、跨 provider regression 和 metadata 断言。
旧 OAuth target 在 Copilot 批次后的 129,738 字节降到 103,895 字节，新的 Kimi target
为 24,556 字节，Kimi support 为 2,487 字节。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_kimi.rs` | `oauth_flows`, `oauth_kimi` | Kimi provider factory、device/token mount、token request collector 与 stored credential | `oauth-store` |
| `auth/oauth/kimi_coding` fixture | `oauth_kimi` (`builtins`) | device authorization、polling、slow_down、strict token triple、refresh、cancellation、typed errors、expiry 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Meta、xAI、Models store checks、跨 provider regression 和 metadata | `oauth-store` |

Kimi 批次的三项验收均通过。`oauth_kimi` 24 项、保留的 `oauth_flows` 71 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_kimi` 可由 selector 单独选中，Kimi support selector 只扩展到
`oauth_flows` 与 `oauth_kimi`，inventory、Cargo target 与 package consumer owner
映射为 121/121 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-kimi-20261122.json`](../evidence/268-oauth-kimi-20261122.json)。

### 第九批 Meta

2026-11-22 第九批处理 Meta OAuth。device authorization、verification URI 选择、polling、
slow_down、Model API key mint、refresh、cancellation、typed error mapping、expiry 和
Models store ACK 的 25 项断言迁移到独立的 `oauth_meta` target。Meta 专属 device/token
和 mint wire mount、provider factory、请求路径过滤器及 stored credential fixture 归入
`tests/support/oauth_meta.rs`。通用 presenter、expiry、URL 解码和 Models route assembly
继续由对应 support owner 维护。`meta_flow`、`meta_stored_credential` 仍由旧
`oauth_flows` 通过同一 support 消费，用于共享 store、metadata 和多 provider regression。
`oauth_flows` 保留 46 项 xAI、Models store checks、跨 provider regression 和 metadata 断言。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi`, `oauth_meta` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi`, `oauth_meta` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_meta.rs` | `oauth_flows`, `oauth_meta` | Meta provider factory、device/token/mint mount、路径过滤器与 stored credential | `oauth-store` |
| `auth/oauth/meta` fixture | `oauth_meta` (`builtins`) | device authorization、polling、slow_down、Model API key mint、refresh、cancellation、typed errors、expiry 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | xAI、Models store checks、跨 provider regression 和 metadata | `oauth-store` |

Meta 批次的三项验收均通过。`oauth_meta` 25 项、保留的 `oauth_flows` 46 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_meta` 可由 selector 单独选中，Meta support selector 只扩展到
`oauth_models_store` 与 `oauth_regressions`，inventory、Cargo target 与 package consumer
owner 映射为 124/124 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal、polling 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-shared-20261122.json`](../evidence/268-oauth-shared-20261122.json)。

### 第十批 xAI

2026-11-22 第十批处理 xAI OAuth。device authorization、verification URI 选择、polling、
slow_down、refresh、cancellation、typed error mapping、expiry 和 Models store ACK 的
25 项断言迁移到独立的 `oauth_xai` target。xAI 专属 device/token wire mount、provider
factory、token request collector、refresh credential 和 merge helper 归入
`tests/support/oauth_xai.rs`。通用 presenter、expiry、URL 解码和 Models route assembly
继续由对应 support owner 维护。`xai_flow`、`xai_stored_credential` 仍由旧
`oauth_flows` 通过同一 support 消费，用于共享 store、metadata 和多 provider regression。
`oauth_flows` 保留 21 项 Models store checks、跨 provider regression 和 metadata 断言。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_fixtures.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_anthropic`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi`, `oauth_meta`, `oauth_xai` | scripted presenter、expiry 检查、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | `oauth_flows`, `oauth_openrouter`, `oauth_radius`, `oauth_codex`, `oauth_chatgpt`, `oauth_copilot`, `oauth_kimi`, `oauth_meta`, `oauth_xai` | Models OAuth route assembly、identity 与实际 adapter request assertion | `oauth-store` |
| `tests/support/oauth_xai.rs` | `oauth_flows`, `oauth_xai` | xAI provider factory、device/token mount、token request collector、stored credential 与 xAI helper | `oauth-store` |
| `auth/oauth/xai` fixture | `oauth_xai` (`builtins`) | device authorization、polling、slow_down、refresh、cancellation、typed errors、expiry 与 Models store ACK | `oauth-store` |
| `auth/oauth` remaining flows | `oauth_flows` (`builtins`) | Models store checks、跨 provider regression 和 metadata | `oauth-store` |

xAI 批次的三项验收均通过。`oauth_xai` 25 项、保留的 `oauth_flows` 21 项、
`oauth_store_profile` 2 项、`oauth_store_integration` 10 项和 `oauth_wire_shape` 12 项
通过。`oauth_xai` 可由 selector 单独选中，xAI support selector 只扩展到
`oauth_flows` 与 `oauth_xai`，inventory、Cargo target 与 package consumer owner
映射为 123/123 一致。OAuth store package consumer 和 websocket/OAuth shared-owner
pair 均通过，没有删除唯一的 callback、parser、terminal、polling 或 store ACK 断言，也没有
增加兼容行为。完整回执见
[`268-oauth-xai-20261122.json`](../evidence/268-oauth-xai-20261122.json)。

### 第十一批共享 OAuth regression

2026-11-22 第十一批收敛剩余共享 OAuth regression。旧 `oauth_flows` 中的 21 项断言按
职责拆为 `oauth_models_store` 的 9 项 Models store 生命周期断言和
`oauth_regressions` 的 12 项跨 provider、错误边界、取消与 metadata 断言。FakeStore、
StoreMode、ACK 结果和并发序列化观察器归入新的 `tests/support/oauth_store.rs`，由两个
target 共同消费。provider-specific fixture 继续由各自 support owner 维护，旧
`oauth_flows` target 和 source 已退役，没有留下空壳 target。

边界如下：

| owner | target | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/support/oauth_store.rs` | `oauth_models_store`, `oauth_regressions` | FakeStore、StoreMode、ACK outcome、mutation barrier 和并发序列化观察 | `oauth-store` |
| `tests/oauth_models_store.rs` | `oauth_models_store` (`builtins`) | default memory isolation、explicit store replacement、Models login ACK、失败类型、check_auth 和并发写入 | `oauth-store` |
| `tests/oauth_regressions.rs` | `oauth_regressions` (`builtins`) | 九流注册与共享 store、provider error boundary、取消、Codex originator、Radius browser 和 OAuth metadata | `oauth-store` |
| `tests/support/oauth_fixtures.rs` | provider OAuth targets、`oauth_models_store`, `oauth_regressions` | scripted presenter、expiry、URL 解码和通用 stored credential | `oauth-store` |
| `tests/support/oauth_assembly.rs` | provider OAuth targets、`oauth_models_store`, `oauth_regressions` | Models route assembly 与真实 adapter request assertion | `oauth-store` |
| retired `tests/oauth_flows.rs` | none | 旧聚合 target 已删除，所有 21 项断言均有新 owner | none |

共享 OAuth 批次的三项验收均通过。`oauth_models_store` 9 项、`oauth_regressions` 12 项、
所有 provider OAuth target、`oauth_store_profile`、`oauth_store_integration` 和
`oauth_wire_shape` 均通过。两个新 target 可分别由 selector 选中，引用
`tests/support/oauth_store.rs` 只扩展到这两个 target，inventory、Cargo target 与
package consumer owner 映射为 124/124 一致。OAuth store package consumer 和
websocket/OAuth shared-owner pair 均通过，没有删除旧 21 项断言，也没有增加兼容行为。
完整回执见
[`268-oauth-shared-20261122.json`](../evidence/268-oauth-shared-20261122.json)。

### 第十二批 Anthropic protocol fixture

2026-11-22 第十二批处理原 `anthropic_fixtures` 的协议职责混杂问题。保留一个
`anthropic_fixtures` Cargo target，按真实 seam 拆成四个 target 内子模块，并由
`tests/anthropic_fixtures/fixtures.rs` 拥有跨主题共用的 Anthropic SSE 响应 fixture：
`sse` 负责 SSE 解析、stream 映射、usage/thinking/tool-call、prepared request、回调和
取消；`constrained_sampling` 负责 strict schema/grammar projection、Prefer/Require
语义和唯一的 strict-limit downgrade variant；`adapter` 负责 Models/direct adapter
生命周期、native options、cache、inline tools、fallback、affinity、federation 与
source-compat regression。没有新增 executable，也没有复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/anthropic_fixtures/fixtures.rs` | `anthropic_fixtures::{sse, constrained_sampling, adapter}` | 单一 Anthropic SSE 成功响应 fixture，避免各主题维护重复 wire 文本 | `anthropic` package consumer 不直接复制此 fixture |
| `tests/anthropic_fixtures/sse.rs` | `anthropic_fixtures::sse` | SSE parser/mapper、text/tool/usage/thinking、prepared HTTP、回调与取消 | 无直接绑定；`anthropic_usage_updates` 由独立 package runner 验证 |
| `tests/anthropic_fixtures/constrained_sampling.rs` | `anthropic_fixtures::constrained_sampling` | strict JSON/grammar projection、wire schema、Require preflight 与 bounded downgrade | 无直接绑定 |
| `tests/anthropic_fixtures/adapter.rs` | `anthropic_fixtures::adapter` | Models/direct adapter、native lifecycle、cache、inline tools、fallback、affinity 与 federation | 无直接绑定 |
| `tests/anthropic_fixtures.rs` | `anthropic_fixtures` (`builtins`) | target index，保留单 executable 和 feature owner | 无额外 package source copy |
| `scripts/pi-sdk-conformance/run_anthropic_package.py` | `anthropic_usage_updates` package consumer | 实际打包产物的 Anthropic usage consumer，维护 5 个具名 contract tests 与 two feature profiles | `opi-ai` package |

第十二批的三项验收均通过。三个 topic selector 分别只选中 `anthropic_fixtures` 的
`sse::`、`constrained_sampling::` 和 `adapter::` 过滤范围，fixture selector 则保留
整个 target。完整 target 共 154 项，按 `sse`、`constrained_sampling`、`adapter` 分别为 56、27、71
项。Cargo metadata、`tests/targets.tsv`、selector 和 module owner 一致，inventory
保持 124/124。Anthropic package consumer 两个 feature profile 和 clippy 均通过。
所有原有 154 项断言保留，旧的 3 项 package 硬编码计数检查改为校验 5 个具名 contract tests，未增加
兼容行为。完整回执见
[`268-anthropic-protocol-20261122.json`](../evidence/268-anthropic-protocol-20261122.json)。

### 第十三批 Codex Responses protocol fixture

2026-11-22 第十三批处理原 `openai_codex_responses` target 的协议职责混杂问题。
保留一个 Cargo target，将请求 wire、strict constrained sampling、WebSocket path、
continuation、fallback/recovery、cancellation、resource cleanup 和 protocol regression
拆为可单独过滤的子模块。`ws_path.rs` 继续拥有本地 WebSocket peer、帧读写和 SSE
响应 fixture；`deflate_audit.rs` 是被 WebSocket 主题共享的 decoder helper，本身不拥有
独立断言。已有 replay、terminal、idle、session cleanup 子模块保留各自 owner。
没有新增 executable、生产逻辑复制或兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/openai_codex_responses.rs` | `openai_codex_responses` | target index、auth/model/request 构造、共享 stream helper 和跨主题 fixture | `provider-openai-codex-responses` package consumer 不复制 fixture |
| `tests/openai_codex_responses/wire.rs` | `openai_codex_responses::wire` | Codex `/codex/responses` SSE 请求、headers、错误、usage、callbacks 和 source compatibility | 无直接绑定 |
| `tests/openai_codex_responses/constrained_sampling.rs` | `openai_codex_responses::constrained_sampling` | strict tool projection、grammar fallback 和诊断 | 无直接绑定 |
| `tests/openai_codex_responses/ws_path.rs` | `openai_codex_responses::ws_path` | local peer、upgrade、frame codec、Auto/SSE/WebSocket path 和 pooled response capture | `provider-openai-codex-responses` package consumer 验证 feature closure |
| `tests/openai_codex_responses/ws_continuation.rs` | `openai_codex_responses::ws_continuation` | pooled continuation、full-input fallback、response identity 和 sessionless connection | 无直接绑定 |
| `tests/openai_codex_responses/ws_branches.rs` | `openai_codex_responses::ws_branches` | connection limit、response failure、missing previous、fallback/recovery budgets | 无直接绑定 |
| `tests/openai_codex_responses/ws_cancellation.rs` | `openai_codex_responses::ws_cancellation` | active/idle cancellation 与 terminal Cancelled 语义 | 无直接绑定 |
| `tests/openai_codex_responses/ws_resources.rs` | `openai_codex_responses::ws_resources` | Models session resource、generation、pool cleanup、consumer Drop 和 release barriers | 无直接绑定 |
| `tests/openai_codex_responses/ws_protocol_regressions.rs` | `openai_codex_responses::ws_protocol_regressions` | compression、masked frames、ping/pong、raw observer、fallback sealing 和 transport diagnostics | 无直接绑定 |
| `tests/openai_codex_responses/{replay,terminal,websocket_idle,websocket_idle_timeout,session_fallback_cleanup}.rs` | existing topic modules | 已有 replay、terminal、idle 和 session cleanup regression owner | 无直接绑定 |
| `tests/openai_codex_responses/deflate_audit.rs` | shared helper | RFC 1951 distance audit，供 WebSocket compression 主题消费 | `provider-openai-codex-responses` package consumer 保持 websocket-compression closure |
| `scripts/pi-sdk-conformance/run_provider_protocol_package.py` | `provider-openai-codex-responses` | 实际打包产物的 feature graph、canonical namespace、WebSocket compression 和 forbidden package 检查 | `opi-ai` package |

第十三批的三项验收均通过。`wire`、`constrained_sampling`、`ws_path`、
`ws_continuation`、`ws_branches`、`ws_cancellation`、`ws_resources` 和
`ws_protocol_regressions` 分别通过 19、3、4、4、10、2、13、20 项过滤测试，
完整 target 共 87 项通过。共享 `deflate_audit.rs` selector 正确回退到完整 target。
Cargo metadata、`tests/targets.tsv`、selector 和 module owner 保持 124/124 一致。
`provider-openai-codex-responses` 独立 package consumer 通过，feature closure、
WebSocket compression 依赖和 canonical namespace 检查均通过。所有原有 87 项断言
保留，没有增加兼容行为。完整回执见
[`268-codex-protocol-20261122.json`](../evidence/268-codex-protocol-20261122.json)。

### 第十四批 provider collection assembly fixture

2026-11-22 第十四批处理原 `provider_collection` target 的跨 provider 主题职责混杂
问题。保留一个 Cargo target，因为这些断言共同验证 builtin composition root、Models
注册/装配和 provider collection 的公开边界；按真实 seam 拆出 phase-17 执行状态机、
registry preflight、catalog store refresh、retry hints 和 selected-owner management 五个
可精确过滤的子模块。既有 27 个 provider-specific 子模块继续由各 provider 文件拥有，
根文件只保留 target index、共享构造/断言 helper 和 builtin 装配断言。没有新增
executable、复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/provider_collection.rs` | `provider_collection` | target index、共享 auth/model/request 构造、公共 helper、builtin composition root 与 provider-specific module 声明 | `catalog-bundled` 与已有 provider package consumers 保持实际包边界；不复制 fixture |
| `tests/provider_collection/phase17.rs` | `provider_collection::phase17` | route/auth 一次性解析、attempt 生命周期、取消、重试、能力/模型/route preflight 与诊断 | 无直接 package fixture 绑定 |
| `tests/provider_collection/registry_preflight_and_refresh.rs` | `provider_collection::registry_preflight_and_refresh` | registry model preflight 和 refresh 聚合错误语义 | 无直接 package fixture 绑定 |
| `tests/provider_collection/catalog_store_refresh_20_33.rs` | `provider_collection::catalog_store_refresh_20_33` | ModelsStore、动态 catalog source、刷新原子性、取消与恢复发布 | `catalog-bundled` package consumer 保持 catalog archive owner |
| `tests/provider_collection/helper_retry_hints.rs` | `provider_collection::helper_retry_hints` | retry hint 上限、零上限和已封存语义 | 无直接 package fixture 绑定 |
| `tests/provider_collection/selected_owner_management.rs` | `provider_collection::selected_owner_management` | selected owner 选择、恢复/发布、资源 namespace 与重注册隔离 | 无直接 package fixture 绑定 |
| `tests/provider_collection/{baseten,cerebras,cloudflare_ai_gateway,cloudflare_workers_ai,fireworks,groq,huggingface,kimi_coding,minimax,minimax_cn,moonshotai,moonshotai_cn,nvidia,qwen_token_plan,qwen_token_plan_cn,radius,together,typesafe,vercel,xai,xiaomi*,zai_coding_cn}.rs` | provider-specific modules | 各 provider 的目录、认证、请求 wire、推理参数和下线模型断言 | 对应 provider package consumer 或 builtin target；本批不改变其 owner |
| `crates/opi-ai/tests/support/catalog_store.rs` | consuming catalog targets | 可复用 ModelsStore fixture，不拥有 provider collection 断言 | `catalog_lifecycle`、`node4_preparation`、`radius_builtin_catalog`、`registry` 与 `provider_collection` 共同消费 |

第十四批的三项验收均通过。`phase17`、`registry_preflight_and_refresh`、
`catalog_store_refresh_20_33`、`helper_retry_hints` 和 `selected_owner_management` 五个
selector 分别通过 26、2、9、4、7 项，完整 `provider_collection` target 共 194 项通过。
Cargo metadata、`tests/targets.tsv`、selector 和 module owner 保持 124/124 一致。
`catalog-bundled` 与已有 provider package consumer 仍由各自 runner 负责，本批未改变
package feature graph，也没有增加兼容行为。完整回执见
[`268-provider-collection-20261122.json`](../evidence/268-provider-collection-20261122.json)。

### 第十五批 Bedrock protocol fixture

2026-11-22 第十五批处理原 `bedrock_fixtures` target 的协议、builtin 装配和 AWS
credential-source 主题混杂问题。保留一个 Cargo target，抽出 `builtin_routes.rs`、
`constrained_guard.rs` 和 `converse_adapter.rs` 三个可精确过滤的主题模块。根文件
继续拥有 credential-source、stream/error、共享 request/response helper 和 target
index，`aws_source_transport.rs` 继续是本地 transport helper。`bedrock_42` 这一历史
任务编号改为语义化的 `converse_adapter`，移除模块文件中的旧 Pi 版本提示。没有新增
executable、复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures.rs` | `bedrock_fixtures` | credential-source 链、stream/error mapping、共享 helper 与 target index | `provider-bedrock` package consumer 保持生产 feature owner；不复制 fixture |
| `tests/bedrock_fixtures/builtin_routes.rs` | `bedrock_fixtures::builtin_routes` | builtin 注册、catalog admission、SigV4/bearer 路由、本地 endpoint、strict tools 与 effort | `provider-bedrock` package consumer 不复制 fixture |
| `tests/bedrock_fixtures/constrained_guard.rs` | `bedrock_fixtures::constrained_guard` | Require/Prefer constrained sampling 的 typed preflight 与 wire projection | 无直接 package fixture 绑定 |
| `tests/bedrock_fixtures/converse_adapter.rs` | `bedrock_fixtures::converse_adapter` | Converse body、reasoning、tool/history projection、usage、replay 与 Models lifecycle | `provider-bedrock` feature profile/package consumer 验证公开生产边界 |
| `tests/bedrock_fixtures/aws_source_transport.rs` | shared local transport helper | AWS source 交换的本地 HTTP/TLS fixture，不拥有独立断言 | 被 `bedrock_fixtures` 消费 |

第十五批的主题 selector 均能独立生成，`builtin_routes` 和 `constrained_guard` 分别
通过 8、2 项，`converse_adapter` 通过 27 项，完整 `bedrock_fixtures` target 编译并执行
168 项，其中 167 项通过。唯一失败仍是迁移前已记录的
`persisted_redacted_bytes_follow_source_forgiving_base64_acceptance` carried-in fixture，
其断言差异与本次文件拆分无关，本批没有修改该行为。Cargo metadata、
`tests/targets.tsv`、selector 和 module owner 保持 124/124 一致，已有
`provider-bedrock` package consumer owner 保持不变，没有增加兼容行为。完整回执见
[`268-bedrock-protocol-20261122.json`](../evidence/268-bedrock-protocol-20261122.json)。

### 第十六批 OpenAI Chat protocol fixture

2026-11-22 第十六批处理原 `openai_chat_fixtures` target 的 wire、Models lifecycle 和
共享 fixture 混杂问题。保留一个 Cargo target，抽出 `wire.rs`、`lifecycle.rs` 和
`constrained_guard.rs` 三个可精确过滤的测试模块，并将 response、request、HTTP
chunk/server 与模型注册 helper 归入 `fixtures.rs`。根文件只保留 target index、公共
imports 和模块声明。没有新增 executable、复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/openai_chat_fixtures.rs` | `openai_chat_fixtures` | target index、公共 imports 和四个模块声明 | `provider-openai-chat` package consumer 保持生产 feature owner；不复制 fixture |
| `tests/openai_chat_fixtures/fixtures.rs` | shared helper owner | OpenAI Chat SSE response、request builder、tool/usage fixture、local HTTP server、Models registration 与 duration helper | 被 `wire` 与 `lifecycle` 共同消费；无独立 package fixture copy |
| `tests/openai_chat_fixtures/wire.rs` | `openai_chat_fixtures::wire` | SSE parser、Chat request body、compat/projection、tool/result、callbacks、HTTP transport 和 cancellation | `provider-openai-chat` package consumer 验证公开 wire owner |
| `tests/openai_chat_fixtures/lifecycle.rs` | `openai_chat_fixtures::lifecycle` | Models native/simple/direct adapter、duration、source compatibility、model/profile precedence 与 required constraints | `provider-openai-chat` package consumer 保持 feature closure |
| `tests/openai_chat_fixtures/constrained_guard.rs` | `openai_chat_fixtures::constrained_guard` | Require/Prefer constrained sampling 的 typed admission 与非 strict projection | 无直接 package fixture 绑定 |

第十六批的三项验收均通过。`wire`、`lifecycle` 和 `constrained_guard` 分别通过 90、24、
2 项，完整 `openai_chat_fixtures` target 共 116 项通过。共享 `fixtures.rs` selector
正确回退到完整 target。Cargo metadata、`tests/targets.tsv`、selector 和 module owner
保持 124/124 一致，没有增加兼容行为。完整回执见
[`268-openai-chat-20261122.json`](../evidence/268-openai-chat-20261122.json)。

### 第十七批 Bedrock wire/lifecycle protocol fixture

2026-11-22 第十七批继续收敛原 `bedrock_fixtures` target。根文件只保留 target
index、公共 imports、已有主题声明和共享 fixture 导入。新增 `wire.rs`、`lifecycle.rs`
和 `fixtures.rs` 三个 owner。`wire.rs` 收纳事件流 framing、model/error、credential
redaction、tool/image projection 等 28 项断言。`lifecycle.rs` 收纳 HTTP stream、
callback、Models route、AWS credential source、STS/ECS/IMDS、SSO/login、refresh 和
cancellation 等 102 项断言。`fixtures.rs` 统一 request、event-stream bytes、auth、
response 和 lifecycle helper，继续供 builtin routes、constrained guard 和 Converse
adapter 消费。没有新增 executable、复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures.rs` | `bedrock_fixtures` | target index、公共 imports、五个主题声明和 shared fixture 导入 | `provider-bedrock` package consumer 保持生产 feature owner；不复制 fixture |
| `tests/bedrock_fixtures/fixtures.rs` | shared fixture owner | Bedrock request、event-stream bytes、auth、response、lifecycle helper | 被 wire、lifecycle、builtin routes、constrained guard 和 Converse adapter 共同消费；无独立 package fixture copy |
| `tests/bedrock_fixtures/wire.rs` | `bedrock_fixtures::wire` | event-stream framing、model/error、credential redaction、tool/image projection | 无直接 package fixture 绑定；由 `provider-bedrock` package consumer 间接验证生产 wire |
| `tests/bedrock_fixtures/lifecycle.rs` | `bedrock_fixtures::lifecycle` | HTTP stream、callback、Models route、AWS credential source、STS/ECS/IMDS、SSO/login、refresh 和 cancellation | `provider-bedrock` package consumer 保持 feature closure |
| `tests/bedrock_fixtures/{builtin_routes,constrained_guard,converse_adapter}.rs` | existing topic modules | builtin route 8 项、strict sampling 2 项、Converse adapter 28 项 | 各自保持现有 Bedrock package/builtin owner |
| `tests/bedrock_fixtures/aws_source_transport.rs` | shared local transport helper | AWS source 交换的本地 HTTP/TLS fixture，不拥有独立断言 | 被 `bedrock_fixtures` 消费 |

第十七批的三项边界验收均通过。`wire` 28 项、`lifecycle` 102 项、`builtin_routes`
8 项和 `constrained_guard` 2 项均通过，`converse_adapter` 保留 28 项，其中唯一
失败仍是迁移前已记录的 `persisted_redacted_bytes_follow_source_forgiving_base64_acceptance`
carried-in fixture。完整 target 共 168 项，167 项通过，未删除任何原有断言。共享
`fixtures.rs` selector 回退到完整 target。Cargo metadata、`tests/targets.tsv`、selector
和 module owner 保持 124/124 一致，`provider-bedrock` package consumer 通过，没有增加
兼容行为。完整回执见
[`268-bedrock-wire-lifecycle-20261122.json`](../evidence/268-bedrock-wire-lifecycle-20261122.json)。

### 第十八批 Bedrock lifecycle credential source and SSO/login fixture

2026-11-22 第十八批继续拆分第十七批留下的 `lifecycle.rs`。AWS environment、
profile、credential process、STS/ECS/IMDS、prepared credential 和 cancellation
断言迁移到 `aws_credentials.rs`，SSO、login、被动 refresh、过期处理和 DPoP rotation
断言迁移到 `aws_sso.rs`。`lifecycle.rs` 保留 25 项 HTTP、Models route、request-header、
callback 和 prepared-call 断言。三个文件继续共享同一个 `bedrock_fixtures` Cargo target，
`aws_sso` 通过 `aws_credentials` 暴露的 source fixture 使用同一套测试数据，没有复制
生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures/lifecycle.rs` | `bedrock_fixtures::lifecycle` | HTTP stream、Models route、request headers、prepared-call 和 callback 的 25 项断言 | `provider-bedrock` package consumer 保持 feature closure |
| `tests/bedrock_fixtures/aws_credentials.rs` | `bedrock_fixtures::lifecycle::aws_credentials` | AWS environment/profile/process、STS/ECS/IMDS、prepared credential 和 cancellation 的 46 项断言与 source fixture | `provider-bedrock` package consumer 间接验证；不复制 fixture |
| `tests/bedrock_fixtures/aws_sso.rs` | `bedrock_fixtures::lifecycle::aws_sso` | SSO、login、refresh、expiry、cache rotation 和 DPoP 的 31 项断言 | `provider-bedrock` package consumer 间接验证；复用 `aws_credentials` source fixture |
| `tests/bedrock_fixtures/fixtures.rs` | shared fixture owner | request、event-stream bytes、auth、response 和 lifecycle helper | 被 lifecycle、aws_credentials、aws_sso 及其它 Bedrock topic 共同消费 |
| `scripts/pi-sdk-conformance/run_provider_protocol_package.py` | `provider-bedrock` | package feature graph、metadata、build/run 和 forbidden package checks | `opi-ai` package |

第十八批的三个验收均通过。`lifecycle::aws_credentials` 46 项、`lifecycle::aws_sso`
31 项和剩余 lifecycle 断言均通过。完整 target 排除既有 carried-in base64 fixture 后
167 项通过，package consumer 通过，selector 能生成嵌套模块过滤器，inventory 保持
124/124，没有增加兼容行为。完整回执见
[`268-bedrock-lifecycle-20261122.json`](../evidence/268-bedrock-lifecycle-20261122.json)。

### 第十九批 Bedrock Converse adapter fixture

2026-11-22 第十九批继续拆分第十八批之后仍然较大的 `converse_adapter.rs`。
`projection.rs` 接管 request、reasoning、message、tool 和 cache projection 的 13 项断言，
`stream.rs` 接管 stream terminal、stop-reason、reasoning delta、usage 和 strict-tool 的
6 项断言，`replay.rs` 接管 persisted history、opaque block、foreign API route 和 model
capability binding 的 5 项断言，`lifecycle.rs` 接管 Models 完整调用、GovCloud route、wire
record admission、simple options 和 telemetry 的 4 项默认断言及 1 项可选 telemetry 断言。
`support.rs` 只拥有 adapter-local helper，`converse_adapter.rs` 只保留模块索引。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures/converse_adapter.rs` | `bedrock_fixtures::converse_adapter` | adapter topic index，装配四个主题模块 | `provider-bedrock` package consumer 保持同一 feature closure |
| `tests/bedrock_fixtures/converse_adapter/support.rs` | shared adapter helper | `model_42`、`req_42`、`StaticAwsResolver42`、body/replay helper；本身无断言 | 被四个 adapter 主题共同消费，修改时运行完整 target |
| `tests/bedrock_fixtures/converse_adapter/projection.rs` | `bedrock_fixtures::converse_adapter::projection` | 13 项 request 与 message projection 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/converse_adapter/stream.rs` | `bedrock_fixtures::converse_adapter::stream` | 6 项 stream、stop-reason、usage 与 strict-tool 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/converse_adapter/replay.rs` | `bedrock_fixtures::converse_adapter::replay` | 5 项 persisted-history 与 opaque-block replay 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/converse_adapter/lifecycle.rs` | `bedrock_fixtures::converse_adapter::lifecycle` | 4 项 Models lifecycle、route contract、telemetry 断言 | `provider-bedrock` package consumer 保持 feature closure |
| `tests/bedrock_fixtures/fixtures.rs` | shared fixture owner | request、event-stream bytes、auth、response 和 lifecycle helper | 被所有 Bedrock topic 共同消费 |

第十九批的三个验收均通过。四个主题 selector 可分别生成，默认 feature 下
`projection` 13 项、`stream` 6 项、`replay` 排除既有 carried-in base64 fixture 后 4 项、
`lifecycle` 4 项通过，`replay` 的第 5 项保留同一已知失败。完整 target 仍为 168 项，
排除该失败后 167 项通过。package consumer 通过，inventory 保持 124/124，没有新增
executable 或兼容行为。完整回执见
[`268-bedrock-converse-adapter-20261122.json`](../evidence/268-bedrock-converse-adapter-20261122.json)。

### 第二十批 Bedrock AWS credential-source fixture

2026-11-22 第二十批继续拆分第十八批遗留的 `aws_credentials.rs`。文件现在只保留
credential-source 主题索引，`support.rs` 集中承接 source fixture、解析输入、共享响应
和断言辅助。`chain.rs` 接管 environment/profile/process 优先级，`role_sources.rs`
接管 assume-role、MFA、STS denial 和 web identity，`metadata.rs` 接管 ECS/IMDS，
嵌套 `lifecycle.rs` 接管 Models、真实本地 HTTP、取消、凭据脱敏和冻结 process context。
`aws_sso.rs` 继续通过索引重导出复用同一套 source helper，没有复制 fixture、生产逻辑
或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures/aws_credentials.rs` | `bedrock_fixtures::lifecycle::aws_credentials` | credential-source topic index，装配四个主题模块并重导出 shared support | `provider-bedrock` package consumer 保持同一 feature closure |
| `tests/bedrock_fixtures/aws_credentials/support.rs` | shared credential-source fixture | `MapAuthContext`、process runner、resolver、source response、STS XML、签名断言和冻结 context；本身无断言 | 被四个 credential topic 与 `aws_sso` 共同消费，修改时运行完整 Bedrock target |
| `tests/bedrock_fixtures/aws_credentials/chain.rs` | `lifecycle::aws_credentials::chain` | 15 项 environment/profile/process precedence 与 region/bearer 选择断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_credentials/role_sources.rs` | `lifecycle::aws_credentials::role_sources` | 11 项 assume-role、MFA、STS denial、web identity 与 nested profile 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_credentials/metadata.rs` | `lifecycle::aws_credentials::metadata` | 9 项 credential-source、ECS 和 IMDS 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_credentials/lifecycle.rs` | `lifecycle::aws_credentials::lifecycle` | 11 项 Models route、真实本地 HTTP、取消、prepared auth 与 frozen process 断言 | `provider-bedrock` package consumer 保持 feature closure |
| `tests/bedrock_fixtures/aws_sso.rs` | `bedrock_fixtures::lifecycle::aws_sso` | 31 项 SSO/login/refresh/expiry/DPoP 断言，复用索引导出的 source helper | `provider-bedrock` package consumer 保持 feature closure |

第二十批的三个验收均通过。四个 credential 子主题 selector 分别通过 15、11、9、11
项，合计保留 `aws_credentials.rs` 原有 46 项断言。`aws_credentials.rs` 索引与
`support.rs` helper selector 均正确回退到完整 target。完整 `bedrock_fixtures` target
仍为 168 项，排除迁移前已记录的 `persisted_redacted_bytes_follow_source_forgiving_base64_acceptance`
后 167 项通过，package consumer 通过，inventory、Cargo target 和 selector owner 映射
一致，没有新增 executable 或兼容行为。完整回执见
[`268-bedrock-aws-credentials-20261122.json`](../evidence/268-bedrock-aws-credentials-20261122.json)。

### 第二十一批 Bedrock AWS SSO/login fixture

2026-11-22 第二十一批继续拆分第十八批遗留的 `aws_sso.rs`。文件现在只保留 SSO
主题索引，`support.rs` 集中承接时钟、token/cache、expiry probe、login refresh transport、
DPoP 和通用断言 helper。`discovery.rs` 接管显式 SSO、profile federation、SSO source
profile 和 denial，`expiry.rs` 接管 near-expiry、refresh、expiry validation、cache write
失败与旧 token 保留，`login.rs` 接管 login cache、refresh rotation、DPoP wire、typed
denial 和本地真实 HTTP，`chain.rs` 接管环境/profile 优先级与 passive-refresh throttle。
四个主题仍属于同一个 `bedrock_fixtures` executable，并继续复用 `aws_credentials`
索引导出的 source fixture，没有复制生产逻辑或增加兼容路径。

边界如下：

| owner | target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/bedrock_fixtures/aws_sso.rs` | `bedrock_fixtures::lifecycle::aws_sso` | SSO/login topic index，装配四个主题模块并重导出 shared support | `provider-bedrock` package consumer 保持同一 feature closure |
| `tests/bedrock_fixtures/aws_sso/support.rs` | shared SSO/login fixture | 时钟、cache/token 文件、expiry probe、login refresh transport、DPoP verifier 与通用 helper；本身无断言 | 被四个 SSO topic 共同消费，修改时运行完整 Bedrock target |
| `tests/bedrock_fixtures/aws_sso/discovery.rs` | `lifecycle::aws_sso::discovery` | 9 项 explicit SSO、profile federation、SSO/login source profile 和 denial 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_sso/expiry.rs` | `lifecycle::aws_sso::expiry` | 11 项 near-expiry、refresh、expiry validation、cache write failure 和旧 token 保留断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_sso/login.rs` | `lifecycle::aws_sso::login` | 8 项 login cache、refresh rotation、DPoP、typed denial 和真实 HTTP 断言 | `provider-bedrock` package consumer 保持 feature closure |
| `tests/bedrock_fixtures/aws_sso/chain.rs` | `lifecycle::aws_sso::chain` | 3 项环境/profile 优先级与 passive-refresh throttle 断言 | `provider-bedrock` 间接验证 |
| `tests/bedrock_fixtures/aws_credentials.rs` | shared credential-source index | 为 SSO topic 提供 source helper re-export，边界由第二十批维护 | `provider-bedrock` package consumer 保持同一 feature closure |

第二十一批的三个验收均通过。四个 SSO 子主题 selector 分别通过 9、11、8、3 项，
合计保留 `aws_sso.rs` 原有 31 项断言。`aws_sso.rs` 索引与 `support.rs` helper selector
均正确回退到完整 target。完整 `bedrock_fixtures` target 排除迁移前已记录的
`persisted_redacted_bytes_follow_source_forgiving_base64_acceptance` 后 167 项通过，
package consumer 通过，inventory、Cargo target 和 selector owner 映射一致，没有新增
executable 或兼容行为。完整回执见
[`268-bedrock-aws-sso-20261122.json`](../evidence/268-bedrock-aws-sso-20261122.json)。

## 第一批独立验收结果

1. `bedrock_retry` target 的 17 项通过，精确 selector 可单独运行，失败不再要求
   编译和执行整个 Bedrock fixture target。
2. Cargo metadata、`tests/targets.tsv`、Cargo target 和 package consumer owner
   一致，`provider_bedrock_profile` 通过，`provider-bedrock` package consumer
   通过。
3. 原 `bedrock_fixtures` 的 168 项通过，唯一失败是迁移前已存在的 carried-in
   base64 fixture。新的 retry、wire profile 和 package checks 均通过。

Test impact: update。新增 `bedrock_retry` target 和 inventory owner，保留原
Bedrock fixture assertions，未增加兼容行为。

## 第三批 Radius 独立验收结果

1. `oauth_radius` target 的 24 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_radius.rs` 时，selector 只扩展到 `oauth_flows` 与
   `oauth_radius`，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 116 个 target、116 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 154 项通过，24 项 Radius 断言均有独立去向，未删除
   callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_radius` target 和 `support/oauth_radius.rs`，update
`oauth_flows`、共享 OAuth fixture owner 与 inventory，保留全部 24 项 Radius
断言和 154 项剩余 OAuth flow 断言。

## 第四批 Anthropic 独立验收结果

1. `oauth_anthropic` target 的 15 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_anthropic.rs` 时，selector 只扩展到 `oauth_flows` 与
   `oauth_anthropic`，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 117 个 target、117 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 139 项通过，15 项 Anthropic 断言均有独立去向，未删除
   callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_anthropic` target 和 `support/oauth_anthropic.rs`，update
`oauth_flows`、通用 OAuth fixture owner 与 inventory，保留全部 15 项 Anthropic
断言和 139 项剩余 OAuth flow 断言。

## 第五批 Codex 独立验收结果

1. `oauth_codex` target 的 11 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_codex.rs` 或 `tests/support/oauth_loopback.rs` 时，selector
   只扩展到 `oauth_flows` 与 `oauth_codex`，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 118 个 target、118 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 128 项通过，11 项 Codex 断言均有独立去向，未删除
   callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_codex` target、`support/oauth_codex.rs` 和
`support/oauth_loopback.rs`，update `oauth_flows`、OAuth assembly owner 与
inventory，保留全部 11 项 Codex 断言和 128 项剩余 OAuth flow 断言。

## 第六批 ChatGPT 独立验收结果

1. `oauth_chatgpt` target 的 11 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_chatgpt.rs` 时，selector 只扩展到 `oauth_flows` 与
   `oauth_chatgpt`，引用 `oauth_loopback.rs` 扩展到 `oauth_flows`、`oauth_codex` 与
   `oauth_chatgpt`，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 119 个 target、119 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 117 项通过，11 项 ChatGPT 断言均有独立去向，未删除
   callback、parser、terminal 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_chatgpt` target 和 `support/oauth_chatgpt.rs`，update
`oauth_flows`、OAuth fixture、loopback、assembly owner 与 inventory，保留全部 11 项
ChatGPT 断言和 117 项剩余 OAuth flow 断言。

## 第七批 Copilot 独立验收结果

1. `oauth_copilot` target 的 22 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_copilot.rs` 时，selector 只扩展到 `oauth_flows` 与
   `oauth_copilot`，引用 `tests/support/tls_fixtures.rs` 时仍扩展到全部实际 TLS
   消费者，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 120 个 target、120 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 95 项通过，22 项 Copilot 断言均有独立去向，未删除
   callback、parser、terminal、TLS inference 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_copilot` target 和 `support/oauth_copilot.rs`，update
`oauth_flows`、OAuth assembly owner、TLS fixture consumer mapping 与 inventory，保留
全部 22 项 Copilot 断言和 95 项剩余 OAuth flow 断言。

## 第八批 Kimi 独立验收结果

1. `oauth_kimi` target 的 24 项通过，精确 selector 可单独运行；引用
   `tests/support/oauth_kimi.rs` 时，selector 只扩展到 `oauth_flows` 与
   `oauth_kimi`，不要求运行其它 OAuth target。
2. Cargo metadata、`tests/targets.tsv`、selector 与 package consumer owner
   一致，inventory 为 121 个 target、121 个 source；`oauth_store_profile`、
   `oauth_store_integration`、`oauth_wire_shape` 与两个 OAuth package consumer
   均通过。
3. `oauth_flows` 保留的 71 项通过，24 项 Kimi 断言均有独立去向，未删除
   callback、parser、terminal、polling 或 store ACK 断言，也没有增加兼容行为。

Test impact: add `oauth_kimi` target 和 `support/oauth_kimi.rs`，update
`oauth_flows`、OAuth fixture、assembly owner 与 inventory，保留全部 24 项 Kimi
断言和 71 项剩余 OAuth flow 断言。

## 第二十二批 Bedrock carried-in base64 fixture 修复

本批只处理第二十一批明确记录的 carried-in 失败，不改变 fixture owner、target 数量或
请求协议。失败样本包含回车符，Pi 的 `atob` 会把它视为空白并继续解码，Rust 投影此前
只过滤制表、换行、换页和空格，因而错误地把该 redacted signature 丢弃。生产代码只在
`decode_redacted_content` 补充回车过滤，并删除重复的换行分支，继续保留无填充、尾位校验、
非法输入和空输入丢弃规则。

边界与验收如下：

1. `converse_adapter::replay::persisted_redacted_bytes_follow_source_forgiving_base64_acceptance`
   聚焦回归通过 1 项，`converse_adapter::replay::` selector 通过 5 项。
2. `check_test_inventory.py` 保持 124/124，`check_architecture_boundaries.py --enforce`
   无 violations，`provider-bedrock` 实际包消费者通过，owner 映射没有新增 target 或
   package fixture copy。
3. 完整 `bedrock_fixtures` target 通过 168/168，`cargo fmt --all --check` 通过，未增加
   兼容行为。详细回执见 [`268-bedrock-base64-20261122.json`](../evidence/268-bedrock-base64-20261122.json)。

Test impact: update。没有新增测试文件或删除断言，仅恢复既有 carried-in base64 回归并更新
验收记录。

## 第二十三批 OpenAI Responses protocol fixture 拆分

本批处理剩余大型 protocol fixture `tests/openai_responses_fixtures.rs`。它原本把
Responses payload/SSE wire、Models lifecycle、constrained sampling、reasoning replay、
callback 和共享 request/model/response builder 放在一个约 190 KB 文件中。现保留同一个
`openai_responses_fixtures` Cargo executable，按实际职责拆为四个主题模块，不复制生产逻辑，
也不改变现有独立的 `openai_responses_lifecycle` target。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/openai_responses_fixtures/fixtures.rs` | shared fixture owner | request、model、response、reasoning builder；无独立断言 | 被三个主题模块消费；修改时运行完整 fixture target |
| `tests/openai_responses_fixtures/wire.rs` | `openai_responses_fixtures::wire` | payload/SSE mapping、terminal/usage、route、acquisition sealing 和 transport | `provider-openai-responses` package consumer 间接验证公开 wire |
| `tests/openai_responses_fixtures/constrained_guard.rs` | `openai_responses_fixtures::constrained_guard` | grammar、strict admission、custom-tool stream projection | `provider-openai-responses` package consumer 保持 feature closure |
| `tests/openai_responses_fixtures/lifecycle.rs` | `openai_responses_fixtures::lifecycle` | Models registration、callbacks、native options、reasoning replay、timing 和 source compatibility | `provider-openai-responses` package consumer 保持 feature closure |
| `tests/openai_responses_lifecycle.rs` | separate `openai_responses_lifecycle` target | HTTP、data-only SSE、missing terminal、deferred submit/fetch/cancel | 既有 target owner，不由本批迁移 |

三项验收均通过。`wire::`、`constrained_guard::`、`lifecycle::` selector 分别通过 40、12、
31 项，完整 fixture target 通过 83/83。`check_test_inventory.py`、`check_support_inventory.py`、
architecture gate、`provider-openai-responses` 实际 package consumer 和格式检查通过，
没有新增 executable、删除断言或兼容行为。共享 `tests/data/responses-error.json` 与
`responses-failed.json` 继续由原有 test-data owner 持有，wire 模块只调整相对 include 路径。
独立的 `openai_responses_lifecycle` target 也复验通过 15/15，确认本批没有改变其边界。
详细回执见 [`268-openai-responses-20261122.json`](../evidence/268-openai-responses-20261122.json)。

Test impact: update。新增四个 topic/helper 文件，更新 root index、target owner 文本和中英测试
布局文档，保留原 target 的 83 项断言与独立 lifecycle target。


## 第二十四批 Vertex protocol fixture 拆分

本批处理剩余大型 `tests/vertex_fixtures.rs` protocol target。原文件同时承载 builtin route、
Vertex wire、HTTP lifecycle、三类 ADC credential source、callback、strict sampling 和
Google 4.1 adapter 断言。现保留单一 `vertex_fixtures` executable，按 owner 拆出主题模块，
不复制生产逻辑，不改变 package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/vertex_fixtures/fixtures.rs` | shared fixture owner | request、stream、provider 与本地 response helper；无独立断言 | 被全部主题模块消费；修改时运行完整 target |
| `tests/vertex_fixtures/builtin_routes.rs` | `vertex_fixtures::builtin_routes` | builtin 注册、跨 provider route 与 endpoint/auth selection | `provider-vertex` |
| `tests/vertex_fixtures/wire.rs` | `vertex_fixtures::wire` | endpoint、auth、catalog/model resolution、request body、SSE、tool、error、usage projection | `provider-vertex` |
| `tests/vertex_fixtures/lifecycle.rs` | `vertex_fixtures::lifecycle` | 本地 HTTP、timeout、cancellation、body limits 与 header routing | `provider-vertex` |
| `tests/vertex_fixtures/adc_vertex.rs` | `vertex_fixtures::adc_vertex` | Vertex ADC source 与 refresh contract | `provider-vertex` |
| `tests/vertex_fixtures/adc_external.rs` | `vertex_fixtures::adc_external` | external account ADC source contract | `provider-vertex` |
| `tests/vertex_fixtures/adc_impersonation.rs` | `vertex_fixtures::adc_impersonation` | service-account impersonation ADC contract | `provider-vertex` |
| `tests/vertex_fixtures/callbacks.rs` | `vertex_fixtures::callbacks` | callback transform 与 provider callback contract | `provider-vertex` |
| `tests/vertex_fixtures/constrained_guard.rs` | `vertex_fixtures::constrained_guard` | strict sampling admission | `provider-vertex` |
| `tests/vertex_fixtures/google_41.rs` | `vertex_fixtures::google_41` | Google 4.1 adapter contract | `provider-vertex` |

三项验收均通过。focused selector 分别通过 `builtin_routes::` 11、`wire::` 20、
`lifecycle::` 11、`adc_vertex::` 7、`adc_external::` 4、`adc_impersonation::` 4、
`callbacks::` 3、`constrained_guard::` 2 与 `google_41::` 13 项，合计 75 项。完整
`vertex_fixtures` target 通过 75/75。`check_test_inventory.py` 保持 124/124，
`check_support_inventory.py` 为 16 helpers/60 edges，architecture gate 无 violations，
`provider-vertex` 实际 package consumer 通过，Vertex 文件 rustfmt 与文档门禁通过。全量 `cargo fmt --all --check` 仍报告此前 OpenAI Responses 批次遗留的格式 diff，本批不改动该范围。没有新增 executable、
删除断言或兼容行为。共享 `tests/data` 未迁移。

Test impact: add。新增十个 Vertex topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，保留原 target 的全部 75 项断言和单一 executable。


## 第二十五批 Gemini protocol fixture 拆分

本批处理当前剩余最大的未拆分 provider protocol target `tests/gemini_fixtures.rs`。原文件约
115 KB，同时承载 Gemini provider/model wire、GenerateContent SSE、HTTP auth/lifecycle、
callbacks、strict sampling、Google 4.0 adapter 与共享本地 server helper。现保留单一
`gemini_fixtures` executable，按实际 owner 拆出主题模块，不复制生产逻辑，不改变
`provider-gemini` package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/gemini_fixtures/fixtures.rs` | shared fixture owner | provider、request、stream 与 stalled-server helper；无独立断言 | 被全部主题模块消费；修改时运行完整 target |
| `tests/gemini_fixtures/wire.rs` | `gemini_fixtures::wire` | provider/model resolution、GenerateContent request body、SSE、usage、error 与 stop reason | `provider-gemini` |
| `tests/gemini_fixtures/lifecycle.rs` | `gemini_fixtures::lifecycle` | HTTP auth classification、request enrichment、timeout、cancellation 与 chunk split | `provider-gemini` |
| `tests/gemini_fixtures/callbacks.rs` | `gemini_fixtures::callbacks` | payload、raw、async observer 与 cancellation callback contract | `provider-gemini` |
| `tests/gemini_fixtures/constrained_guard.rs` | `gemini_fixtures::constrained_guard` | strict sampling admission | `provider-gemini` |
| `tests/gemini_fixtures/google_40.rs` | `gemini_fixtures::google_40` | Google 4.0 chat adapter、native thinking/tool choice、signed replay 与 telemetry | `provider-gemini` |

三项验收均通过。focused selector 分别通过 `wire::` 20、`lifecycle::` 12、`callbacks::` 6、
`constrained_guard::` 2 与 `google_40::` 31 项，合计 71 项。完整 `gemini_fixtures` target
通过 71/71。`check_test_inventory.py` 保持 124/124，`check_support_inventory.py` 保持
16 helpers/60 edges，architecture gate 无 violations，`provider-gemini` 实际 package consumer
通过，Gemini 文件 rustfmt 与文档门禁通过。全量 `cargo fmt --all --check` 仍报告此前
OpenAI Responses 批次遗留的格式 diff，本批不改动该范围。没有新增 executable、删除断言或
兼容行为。

Test impact: add。新增六个 Gemini topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，保留原 target 的全部 71 项断言和单一 executable。

## 第二十六批 Mistral protocol fixture 拆分

本批处理剩余大型 `tests/mistral_fixtures.rs` protocol target。原文件约 113 KB，同时承载
OpenAI-compatible Chat wire、取消与 callback、native Mistral Conversations adapter、replay/cache、
错误和本地 stalled-server helper。现保留单一 `mistral_fixtures` executable，按实际 owner
拆出共享 Chat 主题和 native Conversations 主题，不复制生产逻辑，不改变 `provider-mistral`
package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/mistral_fixtures/fixtures.rs` | shared fixture owner | shared provider、request、stream 与 stalled-server helper；无独立断言 | 被全部 Mistral 主题消费；修改时运行完整 target |
| `tests/mistral_fixtures/chat_wire.rs` | `mistral_fixtures::chat_wire` | OpenAI-compatible model/request、SSE、usage、custom endpoint、HTTP error 与 compat wire | `provider-mistral` |
| `tests/mistral_fixtures/chat_lifecycle.rs` | `mistral_fixtures::chat_lifecycle` | shared Chat cancellation lifecycle | `provider-mistral` |
| `tests/mistral_fixtures/chat_callbacks.rs` | `mistral_fixtures::chat_callbacks` | shared Chat callback contract | `provider-mistral` |
| `tests/mistral_fixtures/native_conversations.rs` | native adapter topic index | native Mistral Conversations topic boundaries 与 re-export；不拥有断言 | `provider-mistral` |
| `tests/mistral_fixtures/native_conversations/fixtures.rs` | native fixture owner | native model、request、SSE、mount 与 response builders；无独立断言 | 被 native 主题消费 |
| `tests/mistral_fixtures/native_conversations/wire.rs` | `mistral_fixtures::native_conversations::wire` | native auth、reasoning/effort、sampling、tool schema、cache affinity、SSE 与 HTTP error | `provider-mistral` |
| `tests/mistral_fixtures/native_conversations/lifecycle.rs` | `mistral_fixtures::native_conversations::lifecycle` | timeout、message conversion、wire mismatch、Models lifecycle 与 tool choice | `provider-mistral` |
| `tests/mistral_fixtures/native_conversations/replay.rs` | `mistral_fixtures::native_conversations::replay` | capture/reload replay、parser partial failure、direct delta 与 cache usage | `provider-mistral` |
| `tests/mistral_fixtures/native_conversations/callbacks.rs` | `mistral_fixtures::native_conversations::callbacks` | telemetry 与 raw/header callback contract | `provider-mistral` |

三项验收均通过。`chat_wire::`、`chat_lifecycle::`、`chat_callbacks::`、
`native_conversations::wire::`、`native_conversations::lifecycle::`、`native_conversations::replay::` 和
`native_conversations::callbacks::` selector 分别通过 15、1、1、24、8、4、1 项，合计 54 项，
完整 `mistral_fixtures` target 在默认 `builtins` 下通过 54/54，开启 `telemetry-tracing` 后通过 55/55。`check_test_inventory.py`、
`check_support_inventory.py`、architecture gate、`provider-mistral` 实际 package consumer
和 Mistral 文件 rustfmt 检查通过。全量 `cargo fmt --all --check` 仍报告此前 OpenAI
Responses 批次遗留的格式 diff，本批不改动该范围。没有新增 executable、删除断言或兼容行为。

Test impact: add。新增十个 Mistral topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，保留原 target 的全部 54 项断言和单一 executable。

## 第二十七批 OpenRouter protocol fixture 拆分

本批处理剩余大型 `tests/openrouter_fixtures.rs` protocol target。原文件约 93 KB，同时承载
OpenRouter-compatible Chat、OpenRouter Images、retry/timeout、Models registration、auth
生命周期、callback/observer 和 telemetry。现保留单一 `openrouter_fixtures` executable，按
Chat 与 Images 的真实职责拆出主题模块，不复制生产逻辑，不改变 `provider-openrouter`
package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/openrouter_fixtures/fixtures.rs` | shared Chat fixture owner | OpenRouter Chat provider、stream collector、stalled-server 与 chunk helper；无独立断言 | 被 Chat 主题消费；修改时运行完整 target |
| `tests/openrouter_fixtures/chat_wire.rs` | `openrouter_fixtures::chat_wire` | provider/model resolution、request body、SSE、usage、custom endpoint 与实际 Chat HTTP request | `provider-openrouter` |
| `tests/openrouter_fixtures/chat_lifecycle.rs` | `openrouter_fixtures::chat_lifecycle` | Chat cancellation、compatibility policy 与 custom registration/store lifecycle | `provider-openrouter` |
| `tests/openrouter_fixtures/images.rs` | Images topic index | Images imports、topic registration 与 child-module re-export；不拥有断言 | `provider-openrouter` |
| `tests/openrouter_fixtures/images/fixtures.rs` | Images fixture owner | image record、Models registration builder、retry/timeout local peer 与 request helper；无独立断言 | 被 Images 主题消费 |
| `tests/openrouter_fixtures/images/wire.rs` | `openrouter_fixtures::images::wire` | image request/response、modalities、usage、cost、endpoint 与 protected-header projection | `provider-openrouter` |
| `tests/openrouter_fixtures/images/lifecycle.rs` | `openrouter_fixtures::images::lifecycle` | image registration、retry、timeout、preflight admission、binding 与 auth cancellation | `provider-openrouter` |
| `tests/openrouter_fixtures/images/callbacks.rs` | `openrouter_fixtures::images::callbacks` | payload/response callbacks、observer result retention、callback timing 与 telemetry | `provider-openrouter` |
| `tests/support/pending_auth.rs` | `openrouter_fixtures` consumer | pending auth fixture remains shared-support owned and is consumed by image auth lifecycle | `oauth-store` shared support |

三项验收均通过。`chat_wire::`、`chat_lifecycle::`、`images::wire::`、
`images::lifecycle::` 和 `images::callbacks::` selector 分别通过 13、3、14、15、10 项，
合计 55 项。完整 `openrouter_fixtures` target 通过 55/55。`check_test_inventory.py`、
`check_support_inventory.py`、architecture gate、`provider-openrouter` 实际 package consumer
和 OpenRouter 文件 rustfmt 检查通过。没有新增 executable、删除断言或兼容行为。

Test impact: add。新增八个 OpenRouter topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，保留原 target 的全部 55 项断言和单一 executable。


## 第二十八批 Azure OpenAI protocol fixture 拆分

本批处理剩余大型 `tests/azure_openai_fixtures.rs` protocol target。原文件约 83 KB，同时承载
Azure OpenAI Chat 与 native Responses 两条 wire、HTTP lifecycle、取消、callback、部署与 endpoint
解析、Azure Responses options、projection、reasoning replay 和 duration 语义。现保留单一
`azure_openai_fixtures` executable，按 Chat 与 Responses 的真实职责拆出主题模块，不复制生产逻辑，
不改变 `provider-azure-openai` package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/azure_openai_fixtures/fixtures.rs` | shared Chat fixture owner | Chat provider、request、SSE、stream collector、stalled-server 与错误/投影 helper；无独立断言 | 被 Chat 主题消费；修改时运行完整 target |
| `tests/azure_openai_fixtures/chat_wire.rs` | `azure_openai_fixtures::chat_wire` | provider/config、deployment URL、model advertisement、Chat SSE、usage、错误、secret redaction 与 request body | `provider-azure-openai` |
| `tests/azure_openai_fixtures/chat_lifecycle.rs` | `azure_openai_fixtures::chat_lifecycle` | Chat HTTP drain、auth/error body、request enrichment、timeout、reserved-header admission、compat 与 cancellation | `provider-azure-openai` |
| `tests/azure_openai_fixtures/chat_callbacks.rs` | `azure_openai_fixtures::chat_callbacks` | Chat payload/response/raw callback ordering through the production path | `provider-azure-openai` |
| `tests/azure_openai_fixtures/responses.rs` | Responses topic index | native Responses child-module registration 与 fixture re-export；不拥有断言 | `provider-azure-openai` |
| `tests/azure_openai_fixtures/responses/fixtures.rs` | Responses fixture owner | Responses model/request/SSE/auth builders、mock mount、body collector、Models registration、history 与 duration peer | 被 Responses 主题消费 |
| `tests/azure_openai_fixtures/responses/wire.rs` | `azure_openai_fixtures::responses::wire` | endpoint/resource/api-version/deployment resolution、Responses URL、SSE error classification 与 wire-record guard | `provider-azure-openai` |
| `tests/azure_openai_fixtures/responses/lifecycle.rs` | `azure_openai_fixtures::responses::lifecycle` | text/tool execution、callback failure、setup failure、body cancellation 与 duration accounting | `provider-azure-openai` |
| `tests/azure_openai_fixtures/responses/options.rs` | `azure_openai_fixtures::responses::options` | native options、per-call record、thinking projection、endpoint override、signed reasoning replay 与 source compat | `provider-azure-openai` |

三项验收均通过。`chat_wire::`、`chat_lifecycle::`、`chat_callbacks::`、
`responses::wire::`、`responses::lifecycle::` 和 `responses::options::` selector 分别通过
12、9、1、8、7、9 项，合计 46 项。完整 `azure_openai_fixtures` target 通过 46/46。
`check_test_inventory.py`、`check_support_inventory.py`、architecture gate、
`provider-azure-openai` 实际 package consumer 和 Azure OpenAI 文件 rustfmt 检查通过。
没有新增 executable、删除断言或兼容行为。

Test impact: add。新增九个 Azure OpenAI topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，删除 0 个测试文件，保留原 target 的全部 46 项断言和单一
executable。

## 第二十九批 Pi Messages protocol fixture 拆分

本批处理剩余大型 `tests/chat_pi_messages.rs` protocol target。原文件 81,369 字节，同时承载
Radius gateway/catalog、Pi Messages request wire、SSE stream、HTTP/auth/Models lifecycle、
callbacks 与 Node 5 replay。现保留单一 `chat_pi_messages` executable，按实际 owner 拆出主题
模块，不复制生产逻辑，不改变 `provider-pi-messages` package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/chat_pi_messages/fixtures.rs` | shared fixture owner | model、request、SSE、local-peer 与 projection helper；无独立断言 | 被所有 Pi Messages 主题消费；修改时运行完整 target |
| `tests/chat_pi_messages/gateway.rs` | `chat_pi_messages::gateway` | Radius gateway baseline/catalog 与 curated catalog | `provider-pi-messages` |
| `tests/chat_pi_messages/wire.rs` | `chat_pi_messages::wire` | endpoint、option whitelist、cache retention 与 context projection | `provider-pi-messages` |
| `tests/chat_pi_messages/stream.rs` | `chat_pi_messages::stream` | SSE block、稀疏 index、terminal marker 与 typed stream failure | `provider-pi-messages` |
| `tests/chat_pi_messages/lifecycle.rs` | `chat_pi_messages::lifecycle` | HTTP failure、cancellation、auth/model admission、simple bypass、wire mismatch 与 Models completion | `provider-pi-messages` |
| `tests/chat_pi_messages/callbacks.rs` | `chat_pi_messages::callbacks` | callback ordering | `provider-pi-messages` |
| `tests/chat_pi_messages/node5.rs` | replay topic index | Node 5 child-module registration 与 fixture re-export；无独立断言 | `provider-pi-messages` |
| `tests/chat_pi_messages/node5/fixtures.rs` | Node 5 fixture owner | replay builder、terminal/complete helper；无独立断言 | 被 Node 5 replay 消费 |
| `tests/chat_pi_messages/node5/replay.rs` | `chat_pi_messages::node5::replay` | producer/reload、UTF-8、terminal accounting、tool-call 与 simple replay regression | `provider-pi-messages` |

三项验收均通过。`gateway::`、`wire::`、`stream::`、`lifecycle::`、`callbacks::` 与
`node5::replay::` selector 分别通过 6、10、10、10、1、5 项，合计 42 项。完整
`chat_pi_messages` target 通过 42/42，`provider-pi-messages` 实际 package consumer、
test inventory 124/124、support inventory 16/60、architecture gate 与 Pi Messages 文件
Rust 2024 rustfmt 均通过。没有新增 executable、删除断言或兼容行为。原根文件的过时
Pi 版本来源注释随 index 重写一并移除，`tests/support` 共享 helper 未迁移。

Test impact: add。新增九个 Pi Messages topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，删除 0 个测试文件，保留原 target 的全部 42 项断言和
单一 executable。

## 第三十批 authentication contract fixture 拆分

本批处理剩余最大的认证契约 target `tests/auth_contracts.rs`。原文件 206,578 字节，同时承载
typed credential failure、object-safe auth contract、resolver/store source selection、serialized
credential mutations、ADC native/external/impersonation source families、token cache 与 scoped
credential snapshot。现保留单一 `auth_contracts` executable，按认证边界拆出主题模块，不复制
生产逻辑，不改变 `opi-ai` auth package consumer 边界，也不增加兼容行为。

| owner | test target/module | 负责内容 | package consumer |
| --- | --- | --- | --- |
| `tests/auth_contracts/errors.rs` | `auth_contracts::errors` | typed credential errors、object-safe `AuthResolver`/`OAuthProvider`/`LoginPresenter` 与 secret redaction | `opi-ai` auth owner |
| `tests/auth_contracts/fixtures.rs` | shared auth fixture owner | `AuthContext`、`CredentialStore`、OAuth 替身、存储 API key 构造函数与合成 service-account key | 被 errors、resolution、mutations、ADC native 和 impersonation 消费；修改时运行完整 target |
| `tests/auth_contracts/resolution.rs` | `auth_contracts::resolution` | lazy source selection、explicit override、store precedence、logout、available providers 与 descriptor resolution | `opi-ai` auth owner |
| `tests/auth_contracts/mutations.rs` | `auth_contracts::mutations` | serialized credential mutation、barrier、receipt、runtime handoff 与 passive refresh registration | `opi-ai` auth owner |
| `tests/auth_contracts/adc_native.rs` | `auth_contracts::adc_native` | ADC native file selection、service-account/authorized-user exchange、refresh/retry 与 metadata token | `opi-ai` auth ADC owner |
| `tests/auth_contracts/adc_external.rs` | `auth_contracts::adc_external` | external account file/URL/AWS/certificate/executable suppliers、STS exchange 与 process lifecycle | `opi-ai` auth ADC owner |
| `tests/auth_contracts/adc_impersonation.rs` | `auth_contracts::adc_impersonation` | impersonation composition、universe validation、recursive source resolution 与 keyed cache | `opi-ai` auth ADC owner |
| `tests/auth_contracts/scoped.rs` | `auth_contracts::scoped` | scoped credential value ownership、resolved snapshot isolation 与 redaction | `opi-ai` auth owner |

三项验收均通过。`errors::`、`resolution::`、`mutations::`、`adc_native::`、`adc_external::`、
`adc_impersonation::` 与 `scoped::` selector 分别通过 7、15、14、20、32、19、1 项，合计
108 项。完整 `auth_contracts` target 通过 108/108，`opi-ai` auth package consumer、test
inventory 124/124、support inventory 16/60、architecture gate 与 authentication 文件
Rust 2024 rustfmt 均通过。`oauth-store` 与 `cloud-auth` 两个实际 external package
consumer 均通过。没有新增 executable、删除断言或兼容行为。

Test impact: add。新增八个 authentication topic/helper 文件并更新 root index、targets owner 文本、
中英文测试布局文档、spec 与 evidence，删除 0 个测试文件，保留原 target 的全部 108 项断言和
单一 executable。

### 认证共享 fixture 选测修复

`stored_api_key` 原来归在 `mutations.rs`，而 `resolution.rs` 也会调用。
`SERVICE_ACCOUNT_PEM` 原来归在 `adc_native.rs`，而 `adc_impersonation.rs`
也会读取。原路径只能选中单个主题。两项共享输入已迁入 `auth_contracts/fixtures.rs`，
该路径选中完整 `auth_contracts` target，七个主题继续各自选测。合成密钥的字节、108 项
测试名和行为断言均保留。完整 target 和七个 selector 通过，外部 `oauth-store` 与
`cloud-auth` 打包消费者通过。证据见[验收记录](../evidence/268-collection-root-and-auth-selection-20261123.json)、
[`oauth-store` 归档](../evidence/268-auth-shared-fixture-oauth-package-20261123.json)和
[`cloud-auth` 归档](../evidence/268-auth-shared-fixture-cloud-package-20261123.json)。

## 第三十一批 provider collection 根文件收敛

第十四批抽出了五个跨 provider 主题，根文件仍有 159,610 字节和 69 项独立测试。
本批保留 `builtins` 的单一 Cargo target，将根文件收成索引与公共导入，原根测试按负责的
公开合同分配给九个可选测主题。跨主题的请求、脚本化响应、认证、刷新和投影输入归入
`provider_collection/fixtures.rs`。跨 target 的 catalog store helper 继续由
`tests/support/catalog_store.rs` 提供。

| 文件（`tests/provider_collection/`） | libtest filter | 主要验证位置 | 本次通过 |
| --- | --- | --- | ---: |
| `auth_resolution.rs` | `auth_resolution::` | descriptor、credential probe 和脱敏 | 14 |
| `dispatch.rs` | `dispatch::` | 准入、stream slot 释放、terminal outcome 和公开入口等价 | 10 |
| `registry_preflight_and_refresh.rs` | `registry_preflight_and_refresh::` | registry 查询、预检和内存刷新 | 10（其中 8 项迁入） |
| `builtin_registration.rs` | `builtin_registration::` | 内置装配、provider 元数据、目录投影和自定义记录优先级 | 6 |
| `builtin_auth.rs` | `builtin_auth::` | AWS、环境、store 与 OAuth 认证来源及调用绑定 | 6 |
| `builtin_chat.rs` | `builtin_chat::` | chat 请求路径、目标 wire API、工具选项和请求头 | 8 |
| `builtin_operations.rs` | `builtin_operations::` | 分类、图像记录及实际请求 | 8 |
| `azure.rs` | `azure::` | Azure endpoint、deployment 和重试绑定 | 3 |
| `opencode.rs` | `opencode::` | Zen、Go 退役 ID，实际请求、推理参数和 prepared session | 6 |
| `fixtures.rs` | 完整 target | `provider_collection` 内主题共用的搭建 | 194 |

三项验收均通过。

1. 九个主题都能独立选中。根索引和目标内共享 fixture 都选择完整 target。
2. Cargo、`targets.tsv`、selector 与归档入口一致。迁移前后各有 194 项通过，迁移的
   69 项测试体原样保留，其余 125 项的完整测试名一致。七条带具名过滤器的生产路径仍选中
   同一组回归。规范路径检查覆盖移动后的子模块。现行 differential runner 的输入映射不涉及该 target。
3. 本地 HTTP/wire、认证、刷新和执行生命周期回归通过。README 示例 runner 从实际归档执行
   `quick_start`、`quick_start_stream` 和 `tool_loop`，在 `builtins-no-tracing` 和
   `defaults-on` 各通过三项。独立 `catalog-bundled` 归档验证嵌入目录。两者分别记录在
   [README 示例归档](../evidence/268-provider-collection-readme-package-20261123.json)与
   [目录归档](../evidence/268-provider-collection-catalog-package-20261123.json)。

统一门禁通过，清单仍为 124 个 Cargo target 对应 124 个根源文件，16 个跨 target helper
对应 60 条消费边。严格 Clippy 仍报告生产库已有的三项诊断。测试影响为
`retain`（302 项现有 Rust 用例）和 `update`（Python 规范路径和选测回归），其中新增一项
共享 fixture 选测用例。本批仅涉及测试及验收记录。完整映射和检查结果见
[本批验收证据](../evidence/268-collection-root-and-auth-selection-20261123.json)。

## 第三十二批目录操作测试

`catalog_operations.rs` 从 83,690 字节缩为 278 字节的索引，继续作为单一
`builtins` Cargo target。原文件有 19 项根用例和 8 项嵌套的 Anthropic 回归。
Anthropic 字段校验、恢复和拒绝提交后的原子性归入发布主题，fallback 费率和价格回读归入价格主题。

| 文件（`tests/catalog_operations/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `publication.rs` | `publication::` | 解码、来源新鲜度、取消与失效、发布失败后保留旧快照、路由元数据恢复 | 11 |
| `dispatch.rs` | `dispatch::` | 按操作选模、可用性探测、认证前检查、adapter 替换及实际 Chat、Images、Classifier 请求 | 8 |
| `pricing.rs` | `pricing::` | 未知价格、阶梯顺序、分数与大整数阈值、fallback 费率的解码、回读和费用计算 | 8 |
| `fixtures.rs` | 完整 target | 混合操作目录、认证替身、请求构造及供发布和价格共用的 Anthropic 记录 | 27 |

目录的外部打包消费者由 `catalog-bundled` runner 持有，检查实际归档内的嵌入目录及独立依赖图。
两条具名生产路径继续绑定 `protocol-adapters` 归档，验证 adapter 导出与依赖图。
动态发布、实际分发和持久化均由 `catalog_operations` 验证。使用解包后的原有 target
补验这三个边界，归档内五个 Rust 文件与工作树逐字节相同。临时解包目录的构建锁只将
`opi-telemetry` 指向配套归档。对应证据见[目录归档](../evidence/268-catalog-operations-catalog-package-20261123.json)
和[协议归档](../evidence/268-catalog-operations-protocol-package-20261123.json)。

三项验收均通过。

1. 发布、分发和价格各自独立选测。根索引及共享 fixture 运行完整 target。
2. Cargo、`targets.tsv`、selector 和归档 owner 一致。27 项测试体保留原样。
   OpenAI Decisions 和共享 System One 的具名筛选分别仍命中原来的 1 项和 8 项。
   当前 differential runner 的输入映射不涉及该 target。
3. 工作树和实际归档内的 target 均通过 27/27，两个外部归档消费者与统一结构门禁通过。
   清单仍为 124 个 Cargo target 对应 124 个根源文件，16 个共享支持文件对应 60 条消费边。

测试影响为 `retain`（27 项 Rust 用例）和 `add`（1 项 Python 选测回归）。
本批只调整测试组织和验证记录。严格 Clippy 报告三个既有生产诊断。
[完整验收证据](../evidence/268-catalog-operations-20261123.json)。

## 第三十三批执行生命周期测试

`execution_lifecycle.rs` 从 72,538 字节收成 336 字节的索引。Cargo 继续登记单一
`execution_lifecycle` target，由 `execution` 持有，层级为 `public-contract`，
成本为 `in-process`，可用 `--no-default-features` 运行。

| 文件（`tests/execution_lifecycle/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `ordinary.rs` | `ordinary::` | 普通调用与裸 `ChatStream::start` 的独立进度、初始化、事件顺序、partial、panic 及缓存结果 | 18 |
| `deferred.rs` | `deferred::` | 首个结果何时可见、何时等待源释放、本地取消、fetch 的 poll 结算和失败分类 | 12 |
| `terminal_release.rs` | `terminal_release::` | started、普通和 prepared 调用的 EOF、abort 等待、源释放及 attempt 封存 | 11 |
| `fixtures.rs` | 完整 target | 目标内共享的受控 channel、EOF/Drop 屏障、chat 请求及 provider 替身 | 41 |

Deferred 的尾部用例和替身保留在 `deferred.rs`，因为 fetch 的可见结果与释放时机需要在同一
条路径断言。重试后尾部的 provider 替身由 `terminal_release.rs` 持有。

三项验收均通过。

1. 三个主题路径分别选中 18、12、11 项并全部通过。根索引与共享 fixture 路径选中完整
   target。缺失的主题路径触发扩检提示，没有选出空筛选命令。
2. Cargo、`targets.tsv`、selector 和归档映射一致。41 个测试保留原测试体和末级名称，
   映射记录了新模块前缀。`run_minimal_package.py` 的外部 `core_profile` 拥有普通调用与
   Faux Deferred fetch 的最小归档检查。`execution_lifecycle` 自身拥有 EOF、Drop、
   abort 和首个结果时序的细粒度断言。生命周期差分由独立的 SDK example driver 持有。
3. 工作树和实际解包后的 target 各通过 41 项，归档的五个测试源与工作树逐字节相同。
   外部 `core` 与 `core-tracing` 均通过。当前 Rust 生命周期驱动对已记录的 Pi 对照通过
   2 项，比较器通过 71 项。统一门禁检查 124 个 Cargo target、310 个 Rust 测试源和
   16 个跨 target 支持文件，共 60 条消费边，未出现违规。

测试影响为 `retain`（41 项）。本批限定在测试组织、登记和文档。目标格式检查通过，
严格 Clippy 报告生产库两项既有诊断。全库格式检查在 `openai_responses_fixtures` 的三个文件中
仍发现既有格式差异。
#268 已验收 33 批。下一候选 `telemetry_operations.rs` 同时负责 Chat、prepared、图像、
分类和 Deferred 的上下文归属，拆分时需要保留 System One 的具名选测。
[验收证据](../evidence/268-execution-lifecycle-20261123.json) ·
[外部最小归档](../evidence/268-execution-lifecycle-minimal-package-20261123.json)

## 第三十四批遥测操作测试

`telemetry_operations.rs` 从 69,128 字节收成 416 字节的索引。单一 Cargo target 由
`telemetry` 持有，层级为 `assembly`，成本为 `local-io`，需要 `builtins,telemetry-tracing`。

| 文件（`tests/telemetry_operations/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `chat.rs` | `chat::` | native、simple 与 direct Chat、任务推进、跨线程 poll、Drop 和 no-op 上下文 | 8 |
| `prepared.rs` | `prepared::` | prepared Anthropic 请求、严格工具准入、重试与回调父 span | 1 |
| `images.rs` | `images::` | OpenRouter 图像请求、回调和上下文隔离 | 1 |
| `classifier.rs` | `classifier::` | System One 两个公开入口、回调和上下文隔离 | 1 |
| `deferred.rs` | `deferred::` | 提交、fetch、取消、放弃 fetch 和句柄序列化 | 1 |
| `fixtures.rs` | 完整 target | 目标内共用的记录器、ambient poll、脱敏标记、序列化断言及 Chat/Deferred 流探针 | 12 |

prepared 的 SSE 与重试响应、图像和分类的请求素材及 Deferred backend 各归对应主题。
外部 README runner 持有实际归档中的内置 SDK 特性矩阵。冻结父 span、回调作用域和
请求隔离由 `telemetry_operations` 持有，并在解包后以相同特性执行。

三项验收均通过。

1. 五条主题路径独立通过 8、1、1、1、1 项。共享 fixture 和根索引选中完整 target，
   缺失主题路径要求扩检。
2. Cargo、`targets.tsv`、selector 和归档映射一致。System One 的六条 Cargo 命令及 runner
   集合一致，`classifier_calls_freeze_context` 迁移前后只命中同一项分类回归。现有 selector
   单测现在也核对该参数。12 个 Rust 测试体和末级名称全部保留。
3. 工作树和解包后均通过 12 项，归档的七个测试源与工作树逐字节一致。外部
   `builtins-no-tracing` 和 `defaults-on` 各通过三个 README 例程。生命周期差分的
   owner 是独立 example driver，上一批的对照仍适用。两次归档的 249 个生产和差分输入
   哈希一致。统一门禁通过，覆盖 124 个 Cargo target、316 个 Rust 测试源和 60 条共享支持消费边。

测试影响为 `retain`（12 项 Rust）和 `update`（1 项 Python 选测回归）。严格 Clippy 报告
三项既有生产诊断，目标的常规 Clippy 与格式检查通过。全库格式仍报告 OpenAI Responses
fixture 的既有差异。#268 已验收 34 批，下一候选 `request_enrichment.rs` 集中了请求标量、
超时、受保护请求头、affinity 和 payload/response 回调。
[验收证据](../evidence/268-telemetry-operations-20261123.json) ·
[外部 README 归档](../evidence/268-telemetry-operations-readme-package-20261123.json)

## 第三十五批请求扩展测试

`request_enrichment.rs` 从 68,697 字节收成 422 字节的索引。单一 Cargo target 由
`provider` 持有，层级为 `wire`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/request_enrichment/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `scalars_and_timeout.rs` | `scalars_and_timeout::` | 请求默认值、显式字段、采样与 Anthropic 超时终止 | 7 |
| `protected_headers.rs` | `protected_headers::` | 请求头校验、每次 dispatch 转换和认证、路由标识保护 | 11 |
| `session_affinity.rs` | `session_affinity::` | Chat 与 Responses 的会话请求头、缓存键、保留策略和调用方覆盖 | 17 |
| `callbacks.rs` | `callbacks::` | payload 形状及预算、响应及 raw 回调顺序、重试复用冻结请求字节 | 10 |
| `fixtures.rs` | 完整 target | 共用请求构造、流排空、HTTP 请求记录读取和请求获准后的失败投影断言 | 45 |

affinity 保留原内嵌矩阵的 13 个用例和名称后缀。跨回调排序用例由 `callbacks` 持有，验证
请求头转换、payload、response 和 raw 的顺序。各主题的专用响应输入由主题持有。
外部 `protocol-adapters` runner 拥有实际归档的公开导出和特性依赖图验证。带 `builtins`
的 `request_enrichment` 拥有四个主题的行为断言，解包后执行同一 target。

三项验收均通过。

1. 四条主题路径独立通过 7、11、17、10 项。根索引和共享 fixture 选中完整 target，
   缺失主题路径要求扩检。
2. Cargo、`targets.tsv`、selector 和归档映射一致。45 个测试体保持原样并记录新名称。
   现有生产路径 selector 没有绑定此 target 的具名过滤。OpenAI Chat 响应观察差分使用
   自己的用例源，当前差分 runner 的输入映射不依赖本 target。
3. 工作树和解包后的 target 各通过 45 项。归档的六个测试源与工作树逐字节相同，
   外部 `protocol-adapters` 消费者通过。和上一批相比，归档的 249 个生产和差分输入哈希一致。
   统一门禁通过，覆盖 124 个 Cargo target、321 个 Rust 测试源及 60 条跨 target 支持消费边。

测试影响为 `retain`（45 项）。目标的常规 Clippy 和格式检查通过。严格 Clippy 报告
三条既有生产诊断，全库格式仍报告 OpenAI Responses fixture 的既有差异。
#268 已验收 35 批。下一候选 `node4_preparation.rs` 集中了调用限额、目录恢复和投影时序，
并消费跨 target 的 `catalog_store` fixture。
[验收证据](../evidence/268-request-enrichment-20261123.json) ·
[外部协议归档](../evidence/268-request-enrichment-protocol-package-20261123.json)

## 第三十六批 Models 请求准备测试

`node4_preparation.rs` 从 66,159 字节收成 327 字节的索引。单一 Cargo target 由
`models` 持有，层级为 `assembly`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/node4_preparation/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `call_limits.rs` | `call_limits::` | Chat 和 Responses 的 native/simple 限额、默认值、上下文预算和 wire 最小值 | 5 |
| `catalog_restore.rs` | `catalog_restore::` | 发布者归属、store 离线重开、刷新失败后保留和发布者撤下 | 6 |
| `projection.rs` | `projection::` | 各调用入口的请求投影、重试归属，以及认证、发送、EOF 和取消时的失败事实 | 15 |
| `fixtures.rs` | 完整 target | 限额和投影共用的请求构造、Chat SSE 与 HTTP 请求体读取 | 26 |

目录恢复直接引用由 catalog 共享测试支持持有的 `tests/support/catalog_store.rs`。外部
`catalog-bundled` 消费者持有归档嵌入目录及特性依赖图验证。三个行为主题由带 `builtins`
的 `node4_preparation` 持有，解包后执行同一 target。

三项验收均通过。

1. 三个主题路径分别独立通过 5、6、15 项。根索引和目标内共享 fixture 选中完整 target，
   缺失主题路径要求扩检。
2. Cargo、`targets.tsv`、selector 和归档映射一致。`catalog_store` 及所有权清单的 SHA-256
   与拆分前一致。其路径生成相同的 5 条完整 Cargo 命令，涵盖 `catalog_lifecycle`、
   `node4_preparation`、`provider_collection`、`radius_builtin_catalog` 和 `registry`。
   selector 单测核对这组命令都没有模块过滤。26 个测试体原样保留并记录新名称。
   #105 的限额差分继续由独立的 `openai_chat_output_limits` 持有，现有 wire 测试链接已更新。
3. 工作树和实际解包后的 target 各通过 26 项。归档中的根索引、四个子文件和
   `catalog_store` 与工作树逐字节一致。外部目录消费者及统一门禁通过。归档的 249 个生产和
   差分输入与上一批一致，独立限额差分的 Rust 测试源也逐字节一致。结构门禁覆盖
   124 个 Cargo target、325 个 Rust 测试源及 60 条跨 target 支持消费边。

测试影响为 `retain`（26 项 Rust）和 `update`（1 项 Python 选测回归）。目标格式及
常规 Clippy 通过。严格 Clippy 报告三条既有生产诊断，全库格式仍报告 OpenAI Responses
fixture 的既有差异。#268 已验收 36 批。下一候选 `tls_contract.rs` 覆盖 TLS 协商与信任、
mTLS、取消清理和进程级 provider 隔离，并消费跨 target 的 `tls_fixtures`。
[验收证据](../evidence/268-node4-preparation-20261123.json) ·
[外部目录归档](../evidence/268-node4-preparation-catalog-package-20261123.json)

## 第三十七批 TLS 公共合同测试

`tls_contract.rs` 从 64,622 字节收成 456 字节的索引。单一 Cargo target 由 `tls`
持有，层级为 `public-contract`，成本为 `extended`，需要 `builtins`。

| 文件（`tests/tls_contract/`） | libtest filter | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `negotiation_and_trust.rs` | `negotiation_and_trust::` | TLS 1.2/1.3、H1/H2 ALPN、密钥交换和信任失败 | 11 |
| `mtls.rs` | `mtls::` | 客户端身份准入、验证和逐连接隔离 | 6 |
| `cancellation.rs` | `cancellation::` | 握手和响应取消、EOF、资源释放及 H2 拒流后单次发送 | 5 |
| `provider_isolation.rs` | `provider_isolation::` | rustls 进程全局状态、宿主 ring 及实例密钥交换配置 | 3 |
| `fixtures.rs` | 完整 target | 共用本地 TLS peer、观测计数和 HttpClient 到 Models Chat 的调用驱动 | 25 |

证书和密钥继续由 TLS 测试支持持有，位于 `tests/support/tls_fixtures.rs`。
外部 `consumer-crypto` 从实际 Cargo 归档验证公开 TLS/Models 调用，使用独立的
测试 peer 和信任材料。完整握手矩阵、取消和 H2 终止行为由解包后的 `tls_contract`
验证。共享证书涉及的 Upgrade/WSS 归档映射由 `ws` 系列持有。

1. 四个主题分别独立通过 11、6、5、3 项。根索引和目标内 fixture 选中完整 target。
   缺失主题路径触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和归档映射一致。共享 fixture 及其清单字节保持一致，
   且仍选中 `bedrock_fixtures`、`http_acceptance`、`oauth_copilot`、`oauth_regressions`、
   `tls_contract` 和 `ws_upgrade` 六个完整 target。保留 `ws` runner，并接入
   `tls_contract` 的 `crypto` runner。25 个旧用例都有名称映射，23 个测试体原样保留，
   另两个跟随模块路径构造精确子进程测试名。
3. 工作树和实际解包后的 target 各通过 25 项，两个隔离子进程各实际执行 1 项。
   外部 `crypto` 的 ordinary 和 caller-union 各通过 6 项。归档内索引、五个子文件
   和共享证书源均与工作树逐字节一致。TLS 测试由本地 peer 验证。与上一批归档
   比对的 276 个输入一致，包含生产源码、Cargo 输入、共享 fixture、独立 WSS 和
   Chat 限额差分用例。统一门禁覆盖 124 个 Cargo target、330 个 Rust 测试源及
   60 条跨 target 支持消费边。

测试影响为 `retain`（25 项 Rust）和 `update`（1 项 Python 选测回归）。本批改动限于
测试、选测和验收记录。目标格式与常规 Clippy 通过。严格 Clippy 仍有三条既有
生产诊断，全库格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 37 批。
下一候选 `external_sdk.rs` 中的 Chat 协议矩阵、分类与图像、Models 公共合同可分别收敛。
[验收证据](../evidence/268-tls-contract-20261123.json) ·
[外部 TLS 归档](../evidence/268-tls-contract-crypto-package-20261123.json)

## 第三十八批外部 SDK 测试

`external_sdk.rs` 从 62,583 字节拆为 332 字节的索引和三个主题。`models` 持有唯一
Cargo target，层级为 `assembly`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/external_sdk/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `chat_matrix.rs` | `chat_matrix::` | 单个 Models 实例驱动十条 Chat 路由，覆盖内置注册、注入认证和本地协议 peer | 1 |
| `classifier_and_images.rs` | `classifier_and_images::` | 四种分类适配器的类型化分发及 OpenRouter 图像生成 | 1 |
| `models_contract.rs` | `models_contract::` | 跨协议选项投影和准入、生命周期、工具续接、实例隔离、零 I/O、大集合与 Deferred fetch | 9 |
| `fixtures.rs` | 完整 target | Chat 与 Models 共享的认证记录器、请求构造和 SSE 应答 | 11 |

专用 peer、响应和断言由对应主题持有。外部 `consumer-readme` 用自己的受控 peer
执行归档附带的 onboarding 示例。解包后的 `external_sdk` target 负责完整矩阵和
公共合同。外部 `protocol-adapters` 验证三个分类适配器的公开 ID 和协议 feature 图。

1. 三个主题分别独立通过 1、1、9 项。索引和共享 fixture 选中完整 target。原有
   `assembled_matrix_drives` 和
   `classifier_and_image_apis_answer_typed_through_one_instance` 各实际选中并通过
   1 项。缺失主题触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 README 归档映射一致。11 个用例均有旧名到
   新路径的映射，测试体逐项一致。统一门禁覆盖 124 个 Cargo target、334 个 Rust
   测试源及 60 条共享支持消费边。现有生产选择命令和协议归档绑定保留。
3. 工作树与实际解包后的 target 各通过 11 项，归档内五个测试源与工作树逐字节一致。
   README 外部消费者在 `builtins-no-tracing` 和 `defaults-on` 下各通过三个示例和
   一项延迟 peer 回归。协议适配器包验证复用此前通过的回执，核对了发布输入、
   runner、工具链和消费者文件。新旧归档的生产源码、Cargo 输入、Pi 差分驱动和
   独立 Chat 限额测试一致，沿用对应差分证据。

测试影响为 `retain`（11 项 Rust）和 `add`（1 项选测回归）。改动限于测试、
清单和文档。目标格式与常规 Clippy 通过。严格 Clippy 仍有三条既有生产诊断，全库
格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 38 批。下一候选
`http_acceptance.rs` 涉及响应解析、重定向与中断，以及 HTTPS 代理。
[验收证据](../evidence/268-external-sdk-20261123.json) ·
[外部 README 归档](../evidence/268-external-sdk-readme-package-20261123.json)

## 第三十九批 HTTP 公共合同测试

`http_acceptance.rs` 从 60,553 字节拆为 530 字节的索引和三个主题。`http` 持有唯一
Cargo target，层级为 `public-contract`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/http_acceptance/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `response_parsing.rs` | `response_parsing::` | 响应头大小与数量、chunk framing、trailer 及 EOF 接受边界 | 10 |
| `redirects_and_disconnects.rs` | `redirects_and_disconnects::` | 同源与跨源重定向、发送预算、认证头和响应前后连接失败 | 9 |
| `https_proxy.rs` | `https_proxy::` | 外层 TLS、CONNECT、内层 TLS、限定范围的 `NO_PROXY` 绕行 | 2 |
| `fixtures.rs` | 完整 target | TCP/TLS 共用的 HTTP/1 脚本 peer、请求记录和 Models Chat 驱动 | 21 |

响应边界、重定向脚本及 TLS/CONNECT 组装由对应主题持有。共享的
`tests/support/tls_fixtures.rs` 由 TLS 测试支持持有，提供证书和密钥。
外部 `consumer-http` 用自己的 loopback peer 验证公开请求、重定向和 HTTP 代理。
细粒度解析及双层 TLS 隧道由解包后的 `http_acceptance` 验证。

1. 三个主题独立通过 10、9、2 项。根索引及目标内 fixture 选中完整 target，缺失主题
   路径触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 HTTP 归档映射一致。共享 TLS 材料及清单逐字节
   一致，保留 `bedrock_fixtures`、`http_acceptance`、`oauth_copilot`、
   `oauth_regressions`、`tls_contract` 和 `ws_upgrade` 六条完整 target 命令，
   保留 crypto 和 WS 归档绑定。HTTP 归档绑定由 `http_acceptance` 提供。统一门禁
   覆盖 124 个 Cargo target、338 个 Rust 测试源及 60 条共享支持消费边。
3. 工作树与实际解包后的 target 各通过 21 项。六个归档测试源与工作树逐字节一致，
   原用例及断言逐项一致。TCP 与 TLS 测试 peer 复用相同的请求解析、回写和关闭条件。
   外部 `consumer-http` 的 ordinary 和 caller-union 各通过 6 项。新旧归档中的
   生产源码、Cargo 输入、Pi 差分驱动和独立 Chat 限额测试一致，沿用已有差分证据。

测试影响为 `retain`（21 项 Rust）、`add`（1 项主题选测回归）和
`update`（1 项共享 TLS 选测回归）。改动限于测试、选测和文档。目标格式与常规
Clippy 通过。严格 Clippy 仍有三条既有生产诊断，全库格式仍有 OpenAI Responses
fixture 的既有差异。#268 已验收 39 批。下一候选 `chat_capture_replay.rs` 涉及
生产者输出捕获、持久化恢复及向目标 wire 的投影。
[验收证据](../evidence/268-http-acceptance-20261123.json)

## 第四十批 Chat 捕获与重放测试

`chat_capture_replay.rs` 从 60,207 字节收成 357 字节的索引。`replay` 持有唯一 Cargo
target，层级为 `assembly`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/chat_capture_replay/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `producer_capture.rs` | `producer_capture::` | 单次真实生产者请求的身份、推理内容、用量和终止事实 | 1 项，覆盖五组输入 |
| `persistence_restore.rs` | `persistence_restore::` | 真实输出恢复后的对象及规范 JSON 字节往返 | 1 项，覆盖同五组输入 |
| `target_projection.rs` | `target_projection::` | 真实输出捕获、恢复后发送到目标协议的完整链路 | 13 项 |
| `fixtures.rs` | 完整 target | 内置场景清单、路由组装、SSE peer、请求和 JSON 恢复 | 15 项 |

捕获与恢复两个主题的每组输入各验证一次生产者请求。投影主题持有原有同模型、
异模型、异 provider 或 wire、无效及未知元数据、grammar 和原生工具 ID 合同。
外部 `consumer-readme` 负责归档示例的公开调用，包括捕获工具调用和带结果续接。
JSON 恢复及细粒度目标请求由解包后的 `chat_capture_replay` 验证。

1. 三个主题分别独立通过 1、1、13 项。投影中的 `replay_boundaries` 和
   `persisted_producers` 分别独立通过 3、4 项。根索引及共享 fixture 选中完整 target，
   缺失主题触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 README 归档映射一致。13 条原用例均有新名称
   对应，8 项用例体按新文件缩进后逐项一致，5 项引用共享清单中等价的输入。原矩阵的
   捕获断言分配给单请求主题，原始终止事实的 JSON 断言由恢复主题验证，目标请求断言
   保留。统一门禁覆盖 124 个 Cargo target、344 个 Rust 测试源及 60 条共享支持边。
3. 工作树和实际解包后的 target 各通过 15 项，归档内七个测试源与工作树逐字节一致。
   外部 README 消费者的 `builtins-no-tracing` 和 `defaults-on` 各通过三个示例和
   一项延迟 peer 回归。生产源码、Cargo 输入、Pi 差分驱动及独立 Chat 限额测试在
   新旧归档中一致，沿用既有差分证据。

测试影响为 `retain`（13 项原有 Rust 回归）和 `add`（2 项阶段专测、1 项选测回归）。
目标格式与常规 Clippy 通过。严格 Clippy 仍报三条既有生产诊断，全库格式仍有
OpenAI Responses fixture 的既有差异。#268 已验收 40 批。下一候选
`retry_backoff.rs` 涉及响应头解析、退避策略以及 acquisition 和 Smithy 重试。
[验收证据](../evidence/268-chat-capture-replay-20261123.json)

## 第四十一批 SDK 重试合同测试

`retry_backoff.rs` 从 57,998 字节收成 318 字节的索引。`retry` 持有唯一 Cargo
target，层级为 `public-contract`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/retry_backoff/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `response_headers.rs` | `response_headers::` | Retry-After 秒数、HTTP-date、限流重置和解析优先级 | 10 |
| `backoff_policy.rs` | `backoff_policy::` | 指数退避、最大延迟、尝试预算和 provider 错误分类 | 17 |
| `acquisition.rs` | `acquisition::` | 一项 mock 类型化错误和十二项 PreparedCall 发送、接收、封闭及取消合同 | 13 |
| `smithy.rs` | `smithy::` | AWS 重试配置、错误分类、抖动、提示上界、配额和自适应限流 | 17 |

HTTP-date 助手、获取阶段的 Chat/SSE peer 与 scripted provider，以及 Smithy 的时钟与
输入都由各自主题持有。获取阶段复用 `opi_ai::test_support`。外部 `consumer-http` 验证
默认获取路径上的类型化连接失败，以及接收响应后的 EOF 只发送一次。归档解包后的
`retry_backoff` 验证包括获准重试及 Smithy 策略在内的四个主题。`bedrock_retry`
负责 Converse 请求重试与重建，`whole_call_retry` 负责调用终止后的显式整次重试。

1. 四个主题按路径分别选中并实际通过 10、17、13、17 项。根索引选中完整 target，
   缺失主题触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 HTTP 归档映射一致。57 个原用例迁移后一一对应，
   仅调整缩进，既有 `acquisition::` 和 `smithy::` 用例全名保留。统一门禁覆盖 124 个
   Cargo target、348 个 Rust 测试源及 60 条共享支持边。
3. 拆分前、拆分后及实际解包后的 target 各通过 57 项。五个归档测试源与工作树逐字节
   一致，外部 HTTP 消费者的 ordinary 和 caller-union 各通过 6 项。生产源码、Cargo
   输入、Pi 差分驱动及相邻 retry target 在新旧归档中一致，沿用既有差分证据。

测试影响为 `retain`（57 项 Rust）和 `add`（1 项选测回归）。目标格式和常规 Clippy
通过。严格 Clippy 仍有三条既有生产诊断，全库格式仍有 OpenAI Responses fixture 的
既有差异。#268 已验收 41 批。下一候选 `whole_call_retry.rs` 负责整次调用的重试合同。
[验收证据](../evidence/268-retry-backoff-20261123.json)

## 第四十二批整次调用重试测试

`whole_call_retry.rs` 从 54,754 字节收成 366 字节的索引。`retry` 持有唯一 Cargo
target，层级为 `public-contract`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/whole_call_retry/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `classification.rs` | `classification::` | 配额、上下文溢出、瞬时诊断和规范助手消息的重试判定 | 6 |
| `call_and_callbacks.rs` | `call_and_callbacks::` | 新逻辑调用、预算、回调顺序、生产者返回的终止状态和真实发送 | 14 |
| `backoff_and_cancellation.rs` | `backoff_and_cancellation::` | 延迟上限、取消退避保留部分输出及用量、取消未读完的 429 响应体并释放连接 | 3 |
| `fixtures.rs` | 完整 target | 目标内共用的规范消息、请求构造、策略和结束回调记录 | 23 |

整次调用主题内的 `frozen_target_wire` 和 `quota_transport` 分别持有 Anthropic SSE 和
OpenAI 配额响应脚本。`tests/data/responses-failed.json` 归 OpenAI Responses 失败协议
fixture，由本 target、Responses wire target 和上游探针共用。外部 `consumer-http`
验证 OpenAI Chat 的公开请求获取及 EOF 单次发送边界。解包后的 `whole_call_retry`
验证显式整次重试以及回调合同。

1. 三个主题独立通过 6、14、3 项。整次调用内的两个 wire 子主题可再独立运行 2、4 项。
   根索引和共享 fixture 选中完整 target，缺失路径触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 HTTP 归档映射一致。23 项原用例迁移后一一对应，
   5 项只需调整引用路径，四处具名源码链接同步到新位置。统一门禁覆盖 124 个 Cargo
   target、354 个 Rust 测试源和 60 条共享支持边。
3. 拆分前、拆分后及实际解包后的 target 各通过 23 项，外部 HTTP 消费者两种配置各通过
   6 项。共享 Responses JSON 在归档、工作树和既有差分证据中逐字节一致，另一个 Rust
   消费者的具名 wire 用例通过。23 条分类观测和 11 条 Responses 失败观测与拆分前及
   固定 Pi 1.5 的既有验收记录一致，生产源码、Cargo 输入和生命周期差分驱动字节一致。

测试影响为 `retain`（23 项 Rust）和 `add`（1 项选测回归）。目标格式与常规 Clippy
通过。严格 Clippy 仍有三条既有生产诊断，全库格式仍有 OpenAI Responses fixture 的
既有差异。#268 已验收 42 批。下一候选是 `provider_lifecycle.rs`。
[验收证据](../evidence/268-whole-call-retry-20261123.json)

## 第四十三批 provider 生命周期测试

`provider_lifecycle.rs` 从 49,528 字节收成 372 字节的索引。`provider` 持有唯一 Cargo
target，层级为 `wire`，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/provider_lifecycle/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `request_and_errors.rs` | `request_and_errors::` | Anthropic POST 认证头和实际 JSON、401/429/500 错误分类及终止 | 4 |
| `sse_decoding.rs` | `sse_decoding::` | 文本和工具事件、EOF、逐字节拆帧、UTF-8 替换、工具参数及失败时的部分状态 | 11 |
| `cancellation_and_release.rs` | `cancellation_and_release::` | 观察者退出、等待时取消，以及 Gemini/Bedrock 的连接释放 | 7 |
| `fixtures.rs` | 完整 target | 目标内共用的请求构造和成功文本 SSE | 22 |

类型化错误断言、逐字节 SSE peer 和阶段化连接 peer 分别由对应主题维护。
外部 `consumer-readme` 验证公开 Anthropic 请求、消费前启动及工具续接。细粒度错误、
解码和连接释放由解包后的 `provider_lifecycle` 验证。

1. 三个主题分别独立通过 4、11、7 项。根索引与共享 fixture 选中完整 target，
   缺失主题触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和 README 归档映射一致。22 项原用例均有名称映射，
   21 项用例体与 12 个 fixture 函数相同。请求用例保留原检查，增加一次 mock 命中及
   模型、输出上限、流式开关和消息内容的实际 JSON 匹配。统一门禁覆盖 124 个 Cargo
   target、358 个 Rust 测试源和 60 条共享支持边。
3. 拆分前、拆分后及实际解包后的 target 各通过 22 项。五个归档测试源与工作树逐字节
   一致，外部消费者两种 feature 配置各通过三个示例及一项延迟请求检查。生产源码、
   Cargo 输入、生命周期差分驱动和相邻测试入口在新旧归档中一致，沿用既有差分证据。

测试影响为 `retain`（21 项 Rust）、`update`（1 项现有请求测试）和 `add`（1 项选测回归）。
目标格式与常规 Clippy 通过。严格 Clippy 仍有三条既有生产诊断，全库格式仍有 OpenAI
Responses fixture 的既有差异。#268 已验收 43 批。下一候选是 `deferred_contract.rs`。
[验收证据](../evidence/268-provider-lifecycle-20261123.json)

## 第四十四批 Deferred 合同测试

`deferred_contract.rs` 从 49,376 字节收成 534 字节的索引。`provider/operation` 持有
单一 `public-contract` target，成本为 `in-process`，可关闭全部默认 feature 执行。

| 文件（`tests/deferred_contract/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `handle_validation.rs` | `handle_validation::` | API 注册、句柄序列化和到期、路由拒绝及取消后检查原任务 | 8 |
| `submission_and_fetch.rs` | `submission_and_fetch::` | 从普通生成提交、`wait0` 单次检查、未知任务、认证失败与重试分类 | 5 |
| `cancellation.rs` | `cancellation::` | 本地中止、显式远端取消与再次获取、取消认证等待 | 3 |
| `reopen.rs` | `reopen::` | 调用方持有检查点和时钟，按保存的句柄恢复获取，报告模型缺失和取消失败 | 2 |
| `telemetry.rs` | `telemetry::` | 最小 SDK 对显式遥测 carrier 和默认调用的转发 | 1 |
| `fixtures.rs` | 完整 target | 目标内共用的句柄、请求、记录和脚本化路由构造 | 19 |

检查点和调度器归 `reopen.rs`。可阻塞的认证 resolver 归共享 `pending_auth.rs`，
选中 `classifier_contract`、`deferred_contract` 和 `openrouter_fixtures` 三个完整 target。
外部 `opi-minimum-consumer` 负责公开 Faux 的提交与显式获取。完整 Deferred 合同由
实际解包后的 target 执行。

1. 五个主题独立通过 8、5、3、2、1 项。根索引和目标内共用 fixture 选中完整
   target。32 项 selector 测试通过，缺失路径在严格门禁中退出 1 并提示扩检。
2. Cargo、`targets.tsv`、selector 和最小特性归档映射一致。19 个原用例各有具名
   路径，断言体逐项一致，七处源码引用已定位到新 owner。统一门禁覆盖 124 个
   Cargo target、364 个 Rust 测试源和 60 条共享支持边。
3. 工作树拆分前、拆分后及实际解包各通过 19 项。外部消费者 core 和 core-tracing
   两种配置分别通过 21、22 项，其中各自包含具名 Faux submit/fetch 用例。
   精确选测的五条 cancel 路由观测与拆分前和 Pi 1.5 既有对照记录一致。新旧归档的
   生产源码、Cargo 输入和独立生命周期差分驱动按字节核对，沿用既有差分回执。

测试影响为 `retain`（19 项 Rust）、`add`（1 项主题选测回归）和 `update`（共享
fixture 选测回归）。目标格式和常规 Clippy 通过。严格 Clippy 仍有两条既有 Models
诊断，全库格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 44 批。
下一候选是 `public_utilities.rs`。
[验收证据](../evidence/268-deferred-contract-20261123.json)

## 第四十五批公共工具测试

`public_utilities.rs` 从 48,734 字节收成 629 字节的索引。`utilities` 持有单一
`public-contract` target，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/public_utilities/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `rendering.rs` | `rendering::` | 可见文本、系统段落历史与只读渲染 | 9 |
| `tool_arguments.rs` | `tool_arguments::` | JSON 修复、流式预览、最终参数与字符串枚举 schema | 10 |
| `context_overflow.rs` | `context_overflow::` | 诊断、结束原因、用量和窗口驱动的溢出及可恢复长度判断 | 8 |
| `provider_diagnostics.rs` | `provider_diagnostics::` | 类型化错误码与脱敏诊断 | 3 |
| `identity.rs` | `identity::` | 短哈希、UUIDv7 布局、序列和时钟回退 | 8 |
| `provider_configuration.rs` | `provider_configuration::` | 请求头合并、删除、局部环境回退 | 5 |
| `cancellation.rs` | `cancellation::` | 取消定时器、释放 future、父子 token 的单向传播 | 6 |
| `grammar_property_names.rs` | `grammar_property_names::` | grammar helper 和 Responses 请求构造中的字面键 | 4 |

各主题持有自己的 fixture，环境锁随配置用例维护。外部 `opi-protocol-adapters-consumer`
覆盖公开 header 和 env API 的导出及依赖图，完整的配置行为与其他主题由实际解包后的
`public_utilities` 验证。

1. 八个主题按文件独立运行，根索引选中整个 target。四个 grammar 用例保留原有完整
   名称，旧的 Responses builder 精确筛选在工作树和解包归档中可用。33 项选测回归
   通过，缺失路径触发严格门禁的扩检提示。
2. Cargo、`targets.tsv`、selector 和协议归档映射一致。两个 namespace 检查随配置
   用例迁移，分别能拒绝旧命名空间，相关 Python suite 通过 36 项。三处文档引用指向
   对应主题文件。统一门禁覆盖 124 个 Cargo target、372 个 Rust 测试源和 60 条共享支持边。
3. 拆分前、拆分后和实际解包各通过 53 项。原断言和 12 个 fixture 函数逐项核对。
   外部协议消费者通过。实际回环 Chat 探针的 10 个场景、65 个检查点与 #111 的固定
   Pi 1.5 对照一致，同时覆盖增量预览和公开 JSON helper。新旧归档的生产和构建输入
   按字节核对。

测试影响为 `retain`（53 项 Rust）、`update`（2 项 namespace 检查）和 `add`（1 项
主题选测回归）。目标格式与常规 Clippy 通过。严格 Clippy 仍有三条既有生产诊断，
全库格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 45 批。
下一候选是 `schema_engine.rs`。
[验收证据](../evidence/268-public-utilities-20261123.json)

## 第四十六批 Schema engine 测试

`schema_engine.rs` 从 48,502 字节收成 532 字节的索引。`schema` 持有单一
`public-contract` target，成本为 `extended`，可关闭默认 feature 执行。

| 文件（`tests/schema_engine/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `corpus.rs` | `corpus::` | 64 条冻结差分输入的编译或检查结果 | 1 |
| `value_keywords.rs` | `value_keywords::` | 值检查、标量限制及 JavaScript 等值与数值语义 | 9 |
| `container_keywords.rs` | `container_keywords::` | 数组与对象的数量、元素、键和依赖 | 15 |
| `applicators.rs` | `applicators::` | 分支选择及已校验属性与元素的传播 | 10 |
| `reference_resolution.rs` | `reference_resolution::` | 指针、锚点、资源身份、枚举顺序及求值递归 | 12 |
| `admission.rs` | `admission::` | 编译诊断、资源选择及递归 frame | 23 |
| `stack_safety.rs` | `stack_safety::` | 子进程 2 MiB 线程栈上的深层编译、检查、查找和释放 | 3 |
| `fixtures.rs` | 完整 target | 公共 compile/check 组合 helper | 73 |

`corpus.rs` 持有 `schema-cases.jsonl` 和 `schema-expected.json` 的差分断言。数据改动
时可运行 `corpus::`，选择器对两个数据路径给出扩检提示。外部 `opi-sdk-conformance-consumer`
验证公开 compile/check、工具参数校验及两种依赖图。解包后的 `schema_engine` 执行本 target 的
完整测试。

1. 七个主题分别独立通过 1、9、15、10、12、23、3 项。根索引与共用 helper 选中
   完整 target。34 项选测测试通过，缺失主题触发严格门禁扩检。
2. Cargo、`targets.tsv`、selector 和 Schema 归档映射一致。73 项具名用例各有迁移
   路径。旧的无模块名 `--exact` 实测匹配零项并退出 0。深栈 helper 传递模块限定名，
   等待子进程的执行完成标记再通过。统一门禁覆盖 124 个 Cargo target、380 个 Rust
   测试源和 60 条共享支持边。
3. 拆分前、拆分后和实际解包各通过 73 项，包括 encoded-pointer 精确回归及三个深栈
   子进程。72 个用例体相同，语料用例仅更新引入路径。64 条输入及预期在工作树与新旧
   归档中逐字节一致。外部消费者两种配置各通过 73 项具名检查，归档生产与构建输入
   逐字节核对。

测试影响为 `retain`（73 项 Rust 断言）、`update`（3 项共用子进程 harness 的用例）
和 `add`（1 项选测回归）。目标格式与常规 Clippy 通过。严格 Clippy 仍有两条既有
Models 诊断，全库格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 46 批。
下一候选是 `custom_provider_registration.rs`。
[验收证据](../evidence/268-schema-engine-20261123.json)

## 第四十七批自定义 Provider 注册测试

`custom_provider_registration.rs` 从 47,787 字节收成 644 字节的索引。`models`
持有单一 `public-contract` target，成本为 `in-process`，可关闭默认 feature 执行。

| 文件（`tests/custom_provider_registration/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `registry_providers.rs` | `registry_providers::` | Provider 注册、替换、查询和模拟流式调用 | 7 |
| `registry_models.rs` | `registry_models::` | 模型覆盖校验、优先级、能力和列表 | 15 |
| `chat_registration.rs` | `chat_registration::` | API ID、目录可见性和自有模型记录查询 | 3 |
| `chat_dispatch.rs` | `chat_dispatch::` | 显式记录、native/simple 选项及直接认证 | 4 |
| `chat_lifecycle.rs` | `chat_lifecycle::` | 绑定快照、消费前执行和自有调用生命周期 | 3 |
| `chat_admission.rs` | `chat_admission::` | 能力、API ID、选项类型、provider 归属和取消 | 3 |
| `fixtures.rs` | 完整 target | 本 target 共用的注册表 mock、类型化适配器和认证解析器 | 35 |

1. 六个主题可分别选测，根索引和 fixture 选中完整 target。35 项选测回归通过，
   缺失主题触发严格门禁的扩检提示。统一门禁覆盖 124 个 Cargo target、387 个 Rust
   测试源和 60 条共享支持边。
2. Cargo、`targets.tsv`、selector 和最小特性归档 runner 映射一致。35 项具名用例
   各有迁移路径。外部 `opi-minimum-consumer` 直接使用归档中的八个测试源文件，
   core 和 core-tracing 各通过本 target 的 35 项，消费者总计分别通过 56 和 57 项。
3. 拆分前、拆分后和实际解包各通过 35 项。34 个用例体相同，认证前拒绝用例区分
   选项类型、未注册 API 与 provider 归属三种失败，并观察替换后的 resolver 调用。
   两条 Models setup failure 探针与 #42 固定 Pi 1.5 对照各匹配十项共享观测，保留已
   记录的错误交付差异。归档生产与构建输入逐字节核对。

测试影响为 `retain`（35 项具名用例）、`update`（1 项认证前拒绝用例与最小包消费者）
和 `add`（1 项选测回归）。目标格式、常规 Clippy 和外部消费者严格 Clippy 通过。
工作树严格 Clippy 仍有两条既有 Models 诊断，全库格式仍有 OpenAI Responses fixture
的既有差异。#268 已验收 47 批。下一候选是 `replay_contract.rs`。
[验收证据](../evidence/268-custom-provider-registration-20261123.json)

## 第四十八批重放请求合同测试

`replay_contract.rs` 从 46,956 字节收成 599 字节的索引。`replay` 持有单一
`wire` target，成本为 `in-process`，需要 `builtins`。

| 文件（`tests/replay_contract/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `association.rs` | `association::` | 批次配对、校验及流式失败终止 | 3 |
| `execution_effects.rs` | `execution_effects::` | 缺失结果、失败轮次、未知影响与持久化未执行事实 | 6 |
| `malformed_arguments.rs` | `malformed_arguments::` | 字符串和对象参数、可信未执行事实及脱敏上下文 | 3 |
| `wire_ids.rs` | `wire_ids::` | Responses item ID、请求内 ID 配对和冲突 | 3 |
| `content_projection.rs` | `content_projection::` | 推理身份、可见文本和工具结果图片 | 3 |
| `object_wire.rs` | `object_wire::` | Gemini 和 Bedrock 对象参数及合法配对 | 3 |
| `responses_grammar.rs` | `grammar_replay_tests::` | Responses 自定义调用、结果和 `ctc_` item ID | 1 |
| `fixtures.rs` | 完整 target | 本 target 共用的历史、请求及 Anthropic 回环 fixture | 22 |

1. 七个主题分别执行。根索引和 fixture 选中完整 target。36 项选测回归通过，
   缺失主题触发严格门禁扩检。统一门禁覆盖 124 个 Cargo target、394 个 Rust
   测试源和 60 条共享支持边。
2. Cargo、`targets.tsv`、selector 和 README 归档映射一致。22 个具名用例各有去向。
   `object_wire::*` 与 `grammar_replay_tests::*` 共四个完整名称保留，分别实跑
   Bedrock ID 冲突和 Responses grammar 的 `--exact` 筛选。外部 `consumer-readme`
   两种配置各通过三个示例与一项 peer 回归，第二次 Anthropic 请求的调用和结果
   同用 `weather-1`。
3. 拆分前、拆分后和实际解包各通过 22 项。配对矩阵逐批校验 Anthropic 和 Chat
   的调用、结果 ID 和顺序。分发拒绝用例使用回环地址并保留类型化失败及 EOF。
   现行探针的两组 Chat 请求体和 Anthropic 助手块与 #104 固定 Pi 1.5 观测一致。
   新旧归档的生产、构建和差分输入逐字节核对。

测试影响为 `retain`（22 项具名用例）、`update`（1 项配对断言和回环 fixture）和
`add`（1 项选测回归）。目标格式与常规 Clippy 通过。严格 Clippy 仍有三条既有生产
诊断，全库格式仍有 OpenAI Responses fixture 的既有差异。#268 已验收 48 批。
下一候选是 `model_wire_metadata.rs`。[验收证据](../evidence/268-replay-contract-20261123.json)

## 第四十九批模型 Wire 元数据测试

`model_wire_metadata.rs` 从 42,291 字节收成 689 字节的索引。`types/model` 持有单一
`public-contract` target，成本为 `local-io`，需要 `builtins`。

| 文件（`tests/model_wire_metadata/`） | libtest 筛选 | 负责验证 | 通过 |
| --- | --- | --- | ---: |
| `model_contract.rs` | `model_contract::` | wire 名称、模型校验、路由 JSON、八个内置目录及配置式 Azure | 6 |
| `thinking_admission.rs` | `thinking_admission::` | provider 流在 HTTP 前拒绝不支持的推理，保留模型元数据 | 4 |
| `sampling_parameters.rs` | `sampling_parameters::` | 缺省和显式空值、浅合并与优先级 | 3 |
| `sampling_wire.rs` | `sampling_wire::` | Chat、Responses、Azure 和 Anthropic 请求投影及 payload 回调 | 3 |
| `pricing_usage.rs` | `pricing_usage::` | 费率、阶梯边界及用量 service tier | 3 |
| `anthropic_complete_metadata.rs` | `anthropic_complete_metadata::` | 完整 Anthropic 目录记录的默认值 | 1 |
| `anthropic_response_cost.rs` | `anthropic_response_cost::` | fallback 声明和归因、缓存阶梯与未知成本 | 4 |
| `fixtures.rs` | 完整 target | 本 target 的请求构造和采样参数数据 | 24 |

1. 七个主题分别执行，根索引与 fixture 选中完整 target。37 项选测回归通过，缺失主题
   触发严格门禁扩检。统一门禁覆盖 124 个 Cargo target、402 个 Rust 测试源和 60 条
   共享支持边。
2. Cargo、`targets.tsv`、selector 和目录归档映射一致。24 个原用例都有迁移路径，
   两个 Anthropic 模块的五个限定名保留，分别实跑一条 `--exact`。外部 `catalog-bundled`
   消费者加载并重新解析归档内 42 个 provider 的 1,712 条记录。
3. 拆分前、拆分后和实际解包各通过 24 项。四项推理用例通过真正的 provider 流核对
   `StreamFailure` 内的 `UnsupportedCapability` 和 HTTP 计数，原有公共预校验用例也
   独立通过。固定 Pi 1.5 发布目录的 16 条采样用例及一条 Anthropic fallback SSE 用例
   通过。8 组存在性、2 组浅合并、5 组阶梯与 2 组 fallback 费率计算的同输入断言对应。
   新旧归档的生产、构建及内置目录输入逐字节核对。

测试影响为 `retain`（24 项具名用例）、`update`（4 项准入断言、请求 fixture 及一项
目录测试名称）和 `add`（1 项选测回归）。目标格式与常规 Clippy 通过。严格 Clippy
仍有三条既有生产诊断，全库格式仍有 OpenAI Responses fixture 的既有差异。

#268 已验收 49 批。现有大型根文件扫描中，`bedrock_retry.rs` 已独立持有 Bedrock
重试与重建场景。下一步按职责检查剩余测试边界并完成结构门禁收尾，重点复核既有格式差异和
[#269 的 WS 归档严格 Clippy](269-classify-shared-test-support.md)。
[验收证据](../evidence/268-model-wire-metadata-20261123.json)

## 结构收尾验收

2026-11-23 复核剩余测试根文件及其职责。`bedrock_retry.rs` 继续持有首批独立的
Bedrock 请求获取与重建场景，17 项用例共用重试入口。后续拆分依据是职责混杂、
fixture ownership 或独立选测的困难。

OpenAI Responses 的三个 fixture 文件与对原文件单独运行 rustfmt 的结果逐字节一致。
`openai_responses_fixtures` 的 83 个具名测试保留，修改三处路径选中完整 target。
`cargo fmt --all --check` 及受影响目标的严格 Clippy 通过。统一门禁覆盖 124 个 Cargo
target、402 个 Rust 测试源及 60 条共享支持边，零未选中目标。生产路径提示扩大
验证范围，已通过实跑调用方和归档阶段覆盖。

[#269 的 WS 归档收尾](269-classify-shared-test-support.md#归档收尾验收)在三种依赖图下
通过私有协议、公开 Upgrade、严格 Clippy 和外部消费者验证。测试影响为 `retain`。
[收尾证据](../evidence/269-ws-archive-closeout-20261123.json)
