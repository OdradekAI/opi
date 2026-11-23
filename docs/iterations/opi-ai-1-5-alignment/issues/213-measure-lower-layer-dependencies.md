# #213 给底层语义依赖建立可回归的基线

Status: done
Owner: provider / stream / telemetry / transport / auth
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: [#212](212-guard-profile-to-api-imports.md)

现有零违规报告未覆盖 provider↔stream、telemetry/http→stream、transcript↔provider、Codex transport→profile/Responses 投影、auth builtin→profile 元数据等方向。纯文本 import 数还会漏过限定路径、别名和协议分支。

## 验收

逐条记录消费符号、生产/测试配置、变更原因和归属。检查器对可机械识别的违规拒绝新增，并对已存在的例外精确列名；对 thinking、strict downgrade、Azure URL、Bedrock 校验等按调用链人工判定，给需迁出的行为指定 owner、回归用例和后续任务。分别报告已知例外、新违规与未检查项。

## 验证

扩充 `test_architecture_boundaries.py` 的限定路径、pub use、测试引用和负例，运行 `check_architecture_boundaries.py --enforce`。逐条对照 #210 台账，至少用一次故意插入的非法生产引用证明门禁非空。这张票交付测量，不将仍在基线中的依赖算作已消除。

## 底层依赖基线 · 2026-11-20

原定的方向仍适合作为目标。当前底层有事件、投影和内置装配的反向引用。单看 `src/api/` 和 `src/providers/` 无法衡量这些依赖。下表按当前生产引用位置计数，从 #210 的文件归属表逐项复核。每个文件、符号、引用方式、最大位置数和 feature 记录在可执行的[基线](../../../../scripts/pi-sdk-conformance/lower_layer_baseline.json)。

| 引用方向 | 现存例外 | 当前用途与后续工作 |
| --- | ---: | --- |
| provider → stream | 1 | `EventStream` 的元素来自 `stream::AssistantStreamEvent`。事件值下沉后保留原公开导出，生命周期与 poll 顺序用 `provider_trait`、`execution_lifecycle` 和 `ai_telemetry` 核对。 |
| provider / stream → transcript | 13 | `Request` 携带时间线输入，经 transcript 编译；stream 用 `ProjectionFacts` 记录结果。将值引用导向 `types::transcript`，在投影入口使用最小的借用输入。保留 `transcript_contract`、`replay_contract` 的结果与失败类型。 |
| transcript → provider | 7 | 投影编译器接收整个 `Request` 并生成 `ProviderError`。在 provider 请求边界保留 typed error 转换，投影层只处理必要输入。沿用上行回归。 |
| stream → models | 1 | `CallFailure::from_setup` 经 crate 根导出获取 `ChatModelRecord`，由 `models/chat_call.rs` 调用。以失败输出需要的模型信息替代整个登记记录，验收 setup 失败仍保留 partial 与分类。 |
| telemetry → stream | 21 | Observer 消费事件、失败和终态，其中 `StopReason` 定义已在 `types::usage`。事件值与 producer 的状态分离后，用 `ai_telemetry` 核对首块、失败与终态只上报一次。 |
| http → stream | 1 | `http::openai_rate_limit_detail` 解析 OpenAI/Zhipu 429 响应并附加 stream failure。将 quota/billing 识别交给 Chat 与 Responses 共享的 API 实现，HTTP 保留 body 读取、取消和超时。用 `whole_call_retry` 的 quota veto、transient 429、payload freeze 和取消用例验收。 |
| Codex transport → api / providers / stream | 16 | 7 处 Responses wire 和 URL helper、1 处 Codex profile ID、8 处 stream 引用。把消息投影、WS 重放语义及事件解码归到 Codex adapter，由装配注入服务身份。传输层负责连接与帧。用 `openai_codex_responses`、`replay_contract` 与单独 Codex feature 核对 continuation、重连、压缩和释放。 |
| auth builtin → models / providers | 16 | 15 处构造 Models auth bindings 的类型，1 处读取 `BuiltinProviderProfile`。让内置装配消费 auth resolver 和 source 配置，保留 credential 读取与取消责任。用 `builtin_auth_ownership`、`auth_contracts` 和单独 cloud/provider feature 核对。 |

另有 4 处明确允许的执行接口。execution 使用 `AssistantStreamEvent`、`CallFailure`、`ProducerSnapshot` 驱动或观察流；`stream` 公开转出由 execution 拥有的 `StartedChatStream`。这四处单独记为已审查合同。stream 从 provider 获取公共错误与事件流合同。

### 需要沿调用链核对的行为

| 行为 | 当前调用链 | 后续交付与回归 |
| --- | --- | --- |
| Thinking 默认预算 | Models 的 `compile_prepared_request` 应用通用输出上限，再调用无 feature 门槛的 `Provider::prepare_thinking_request`。默认实现分支识别 Anthropic adaptive 和 Bedrock numeric Claude；Bedrock adapter 也使用同一模型判断。 | 将 wire 默认预算交给对应 adapter 的路由能力，保留通用上限与认证前的失败阶段。迁移需覆盖 `CanonicalRoute`、`ApiMappedProvider`、`ChatApiRouteProvider` 和自定义 Provider 默认行为。回归用 `provider_collection` 的 `prepare_call_anthropic_thinking_keeps_answer_room_and_rejects_tiny_context`、`prepare_call_adds_bedrock_thinking_only_for_numeric_claude` 及两种实际 wire fixture。 |
| Anthropic strict downgrade | Anthropic adapter 根据实际发出的 payload 和 400 标记生成 `StrictToolLimits`，`models/prepared_call.rs` 与 `retry.rs` 控制一次重建、请求次数和封口。通用 `provider::PayloadVariant` 仍有 Anthropic 专用变体。 | 让 adapter 拥有重建条件及 variant 标识，通用 retry 保留预算、一次重建和终止。迁移时单独处理公开错误、回调标签和已公开的 admission 类型。用 `anthropic_fixtures` 的 Models/direct、post-payload、二次标记和 per-variant budget 用例验收。 |
| Azure URL | `provider::routes::resolve_azure_base_url` 处理来源优先级和 endpoint 规范化。Azure adapter 还调用在 profile 文件定义的 `AzureRouteContext::build_azure_url`，这是 import 数看不见的调用。 | 把 endpoint 选择与共享 Azure wire URL 投影归到各自入口，保留已公开方法。用 `azure_openai_fixtures` 的 endpoint 优先级、host 规范化、Responses URL 和 native override，外加 `provider_azure_openai_profile`。 |
| Bedrock endpoint 校验 | profile 构造检查 AWS endpoint 与占位签名，adapter 在认证准备期间及选定实际 endpoint 后再次检查。调用方共用 `provider::routes` 的 Bedrock helper。 | 保留一个 Bedrock 路由校验 owner。若拆分 route 实现，保留各阶段检查与公开 helper，用 `standard_aws_endpoints_reject_proxy_placeholder_signing` 和 `node5_placeholder_signing_rejects_selected_standard_model_endpoint_before_payload` 验收。 |
| Copilot 请求头 | 基线时 `provider.rs` 定义 `github_copilot_route_headers` 与 `github_copilot_initiator`。Anthropic、Chat、Responses 在已解析 auth 的请求上调用。 | [#214](214-move-copilot-header-policy.md) 已迁至 `api::shared::copilot_headers`。门禁确认下层两处函数和受管头常量均已迁出。CLI 工厂启用三种 wire，SDK `register_builtin` 走默认关闭的构造参数。这一装配差异需要独立的行为验收。 |

### 门禁与收敛条件

运行 `python scripts/pi-sdk-conformance/check_architecture_boundaries.py --enforce --json`。当前 **76 处现存例外、4 处已审查合同、0 处新增依赖、0 处待删基线、15 处测试引用被排除**。配额按文件、引用方式和符号计数，行号用来定位。新增一个位置或删除最后一处引用后留下额度，两种情况均报告失败。修复对应依赖时同步删减其基线。

临时 CLI 样例中新增了一个分组别名导入、一个限定路径及一个经 crate 根别名的公开转出，三处均被报告，退出码为 1。通过 `cfg(test)` 载入的子模块引用仍被排除。词法检查覆盖表中路径和指定方向。间接转出、宏展开后的语义、所有 feature 组合和按方法分派的依赖，仍由上述调用链与后续定向验收负责。

默认配置的 `provider_trait`、`execution_lifecycle`、`ai_telemetry`、`replay_contract`、`transcript_contract`、`whole_call_retry` 六个目标共 153 项通过。[门禁与回归回执](../evidence/issue-213-lower-layer-dependencies.json)

Test impact: add 底层门禁、已知例外和负例；retain 生命周期、遥测、replay 及投影行为。
