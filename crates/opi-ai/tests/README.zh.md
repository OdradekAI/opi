# opi-ai 测试组织

[targets.tsv](targets.tsv) 按集成目标记录主要 owner、测试层、验证的行为、运行成本及直接绑定的 runner。源路径和所需 feature 由 Cargo 提供。在仓库根目录可查看合并后的完整清单，或单独检查一致性。

```sh
python scripts/pi-sdk-conformance/check_test_inventory.py --list
python scripts/pi-sdk-conformance/check_test_inventory.py
```

测试层区分公共合同、协议 wire、服务装配和 feature 配置。成本按资源划分：`in-process` 包含进程内冻结 fixture，`local-io` 使用本地服务或文件，`extended` 覆盖子进程、TLS/WS 生命周期、显式性能测量或较大的组装场景。runner 列记录实际执行用例或从归档验证同一公开边界的入口，短横线表示仅依通用 Cargo 选择执行。合并视图中的 feature 是 Cargo 声明的准入条件。比如 `core_profile` 在 minimum 下运行四例，开启 `telemetry-tracing` 后增加一例。

TLS peer、catalog store、pending auth 等稳定搭建放在 `tests/support/`。协议预期和数据随业务测试族维护。新增根目标时同步填写显式 Cargo 声明和清单行。provider feature-profile target 各自拥有源文件，让 target 路径直接表达 owner 和 feature；确需共享源时必须记录独立理由和清单行。更改 target 或用例名时，先核对该行绑定的 runner。

`tests/support/OWNERS.tsv` 是共享 helper 的唯一归属清单，记录每个 helper 的角色、owner、覆盖 layer、成本、runner 和实际 Cargo consumer。检查器会从真实 Rust 模块图推导 consumer，因此手写清单无法掩盖过期或零消费者的 helper：

```sh
python scripts/pi-sdk-conformance/check_support_inventory.py --list
python scripts/pi-sdk-conformance/check_support_inventory.py
```

`stable-tool` 用于不携带协议策略的确定性搭建，`protocol-fixture` 用于绑定协议契约族的请求、认证或存储预期，`platform-fixture` 用于传输层或操作系统行为。跨多个 layer 的 helper 要完整列出这些 layer。共享 helper 只属于测试代码，不得复制到 `src/`。

统一的日常门禁会同时检查 architecture、Cargo target/source inventory、共享 helper 归属和 selector 覆盖：

```sh
python scripts/pi-sdk-conformance/check_structure_gate.py --json
```

变更 `crates/opi-ai` 路径时可以追加 `--strict-selection`，让未登记的路径映射立即失败。完整 workspace 回归仍显式运行 `cargo test --workspace --exclude opi-eval --all-targets --locked`。
根 target 下的命名主题模块在改动时由选测器生成限定模块过滤器。没有测试函数的共享 fixture 改动会保留整个 target。

### 按改动选测

在仓库根目录传入一个或多个改动路径。选择器给出每个目标的选择理由、带 feature 的 Cargo 命令及相关 runner。使用 `--json` 可取得命令参数数组。

```sh
python scripts/pi-sdk-conformance/select_tests.py crates/opi-ai/src/api/openai_decisions.rs
python scripts/pi-sdk-conformance/select_tests.py crates/opi-ai/tests/support/pending_auth.rs
python scripts/pi-sdk-conformance/select_tests.py --target openrouter_cost
python scripts/pi-sdk-conformance/select_tests.py --full
```

已核对的分类协议路径选择本 wire 用例、共享分派和跨层行为。共享测试源选中实际消费目标，包括以不同 feature 编译同一文件的目标。改到 Cargo feature、内置装配或尚未映射的路径时，会提示扩大检查。搬迁合同或重命名用例时，同步更新其选测规则，并实际执行生成的命令核对命中的用例。

### 单跑一种分类协议

`classifier_contract` 按协议组织 wire 断言，Models 生命周期、跨协议行为与声明顺序各有模块。单跑 Decisions wire 用例可执行以下命令。

```sh
cargo test --locked --offline -p opi-ai --test classifier_contract -- openai_decisions::
```

其他协议的模块前缀为 `typesafe_system_one::`、`cloudflare_workers_ai_system_one::` 和 `llama_cpp_classify::`。修改 adapter 源码时使用选测器，以包含实际经过该 wire 的跨协议与 SDK 用例。修改 System One 共享实现时执行整个 classifier 目标。协议响应留在对应模块。仅供 classifier 目标复用的 fixture 放在 `tests/classifier_contract/`，跨目标复用的 helper 放在 `tests/support/`。

[English](README.md)

Rust 同时支持内联测试模块和独立测试文件。关键在于跨越哪个接口，不在于文件大小或涉及几个模块。
后代模块能访问祖先的私有项；integration target 是使用公开接口的独立 crate。
见 [Rust Book：测试组织](https://doc.rust-lang.org/book/ch11-03-test-organization.html)。

| 目的 | 位置 | 证据 |
| --- | --- | --- |
| 私有机制 | `src/<module>/tests.rs` 或 `src/<module>/tests/` | 算法、解析、状态转换、资源所有权 |
| 内部组合 | 同一 crate 内的测试模块，按契约分组 | 例如 Upgrade→codec 内部交接，不意味着公开 provider 调用链已验收 |
| 公开模块/组合验收 | `tests/<contract>.rs`，可有 `tests/<contract>/*.rs` 子模块 | 调用接口、真实 adapter、请求字节、类型错误及生命周期 |
| 集成测试共享辅助代码 | `tests/support/*.rs`，由消费它的 target 声明 | 本地 peer、合成 fixture，不单独形成测试可执行文件 |
| SDK 消费者 | `scripts/pi-sdk-conformance/` 及外部 scratch | 实际产物、独立依赖图、公开消费者行为 |

本次相关目录为：

```text
src/crypto/mod.rs                  # 生产模块；cfg(test) mod tests
src/crypto/tests/mod.rs            # 私有测试索引/共享辅助函数
src/crypto/tests/admission.rs
src/crypto/tests/signing.rs
src/crypto/tests/worker.rs
src/crypto/tests/jwt.rs
src/ws/tests/                      # 私有 codec/内部组合
tests/transcript_contract.rs       # 一个 integration target
tests/anthropic_fixtures.rs         # 一个 Anthropic protocol target
tests/anthropic_fixtures/fixtures.rs
tests/anthropic_fixtures/sse.rs
tests/anthropic_fixtures/constrained_sampling.rs
tests/anthropic_fixtures/adapter.rs
tests/gemini_fixtures.rs          # 一个 Google Gemini protocol target
tests/gemini_fixtures/fixtures.rs
tests/gemini_fixtures/wire.rs
tests/gemini_fixtures/lifecycle.rs
tests/gemini_fixtures/callbacks.rs
tests/gemini_fixtures/constrained_guard.rs
tests/gemini_fixtures/google_40.rs
tests/mistral_fixtures.rs        # 一个 Mistral protocol target
tests/mistral_fixtures/fixtures.rs
tests/mistral_fixtures/chat_wire.rs
tests/mistral_fixtures/chat_lifecycle.rs
tests/mistral_fixtures/chat_callbacks.rs
tests/mistral_fixtures/native_conversations.rs
tests/mistral_fixtures/native_conversations/fixtures.rs
tests/mistral_fixtures/native_conversations/wire.rs
tests/mistral_fixtures/native_conversations/lifecycle.rs
tests/mistral_fixtures/native_conversations/replay.rs
tests/mistral_fixtures/native_conversations/callbacks.rs
tests/openrouter_fixtures.rs       # 一个 OpenRouter protocol target
tests/openrouter_fixtures/fixtures.rs
tests/openrouter_fixtures/chat_wire.rs
tests/openrouter_fixtures/chat_lifecycle.rs
tests/openrouter_fixtures/images.rs
tests/openrouter_fixtures/images/fixtures.rs
tests/openrouter_fixtures/images/wire.rs
tests/openrouter_fixtures/images/lifecycle.rs
tests/openrouter_fixtures/images/callbacks.rs
tests/azure_openai_fixtures.rs # 一个 Azure OpenAI protocol target
tests/azure_openai_fixtures/fixtures.rs
tests/azure_openai_fixtures/chat_wire.rs
tests/azure_openai_fixtures/chat_lifecycle.rs
tests/azure_openai_fixtures/chat_callbacks.rs
tests/azure_openai_fixtures/responses.rs
tests/azure_openai_fixtures/responses/fixtures.rs
tests/azure_openai_fixtures/responses/wire.rs
tests/azure_openai_fixtures/responses/lifecycle.rs
tests/azure_openai_fixtures/responses/options.rs
tests/chat_pi_messages.rs       # 一个 Pi Messages protocol target
tests/chat_pi_messages/fixtures.rs
tests/chat_pi_messages/gateway.rs
tests/chat_pi_messages/wire.rs
tests/chat_pi_messages/stream.rs
tests/chat_pi_messages/lifecycle.rs
tests/chat_pi_messages/callbacks.rs
tests/chat_pi_messages/node5.rs
tests/chat_pi_messages/node5/fixtures.rs
tests/chat_pi_messages/node5/replay.rs
tests/auth_contracts.rs          # 一个认证契约 target
tests/auth_contracts/errors.rs
tests/auth_contracts/fixtures.rs
tests/auth_contracts/resolution.rs
tests/auth_contracts/mutations.rs
tests/auth_contracts/adc_native.rs
tests/auth_contracts/adc_external.rs
tests/auth_contracts/adc_impersonation.rs
tests/auth_contracts/scoped.rs
tests/bedrock_fixtures.rs         # 一个 Bedrock protocol target
tests/bedrock_fixtures/fixtures.rs
tests/bedrock_fixtures/wire.rs
tests/bedrock_fixtures/lifecycle.rs
tests/bedrock_fixtures/aws_credentials.rs
tests/bedrock_fixtures/aws_credentials/support.rs
tests/bedrock_fixtures/aws_credentials/chain.rs
tests/bedrock_fixtures/aws_credentials/role_sources.rs
tests/bedrock_fixtures/aws_credentials/metadata.rs
tests/bedrock_fixtures/aws_credentials/lifecycle.rs
tests/bedrock_fixtures/aws_sso.rs
tests/bedrock_fixtures/aws_sso/support.rs
tests/bedrock_fixtures/aws_sso/discovery.rs
tests/bedrock_fixtures/aws_sso/expiry.rs
tests/bedrock_fixtures/aws_sso/login.rs
tests/bedrock_fixtures/aws_sso/chain.rs
tests/bedrock_fixtures/builtin_routes.rs
tests/bedrock_fixtures/constrained_guard.rs
tests/bedrock_fixtures/converse_adapter.rs
tests/bedrock_fixtures/converse_adapter/support.rs
tests/bedrock_fixtures/converse_adapter/projection.rs
tests/bedrock_fixtures/converse_adapter/stream.rs
tests/bedrock_fixtures/converse_adapter/replay.rs
tests/bedrock_fixtures/converse_adapter/lifecycle.rs
tests/openai_chat_fixtures.rs       # 一个 OpenAI Chat protocol target
tests/openai_chat_fixtures/fixtures.rs
tests/openai_chat_fixtures/wire.rs
tests/openai_chat_fixtures/lifecycle.rs
tests/openai_chat_fixtures/constrained_guard.rs
tests/openai_responses_fixtures.rs # 一个 OpenAI Responses protocol target
tests/openai_responses_fixtures/fixtures.rs
tests/openai_responses_fixtures/wire.rs
tests/openai_responses_fixtures/constrained_guard.rs
tests/openai_responses_fixtures/lifecycle.rs
tests/vertex_fixtures.rs       # 一个 Google Vertex protocol target
tests/vertex_fixtures/fixtures.rs
tests/vertex_fixtures/builtin_routes.rs
tests/vertex_fixtures/wire.rs
tests/vertex_fixtures/lifecycle.rs
tests/vertex_fixtures/adc_vertex.rs
tests/vertex_fixtures/adc_external.rs
tests/vertex_fixtures/adc_impersonation.rs
tests/vertex_fixtures/callbacks.rs
tests/vertex_fixtures/constrained_guard.rs
tests/vertex_fixtures/google_41.rs
tests/transcript_contract/provider_projection.rs
tests/replay_contract.rs           # 一个 integration target
tests/replay_contract/fixtures.rs
tests/replay_contract/association.rs
tests/replay_contract/execution_effects.rs
tests/replay_contract/malformed_arguments.rs
tests/replay_contract/wire_ids.rs
tests/replay_contract/content_projection.rs
tests/replay_contract/object_wire.rs
tests/replay_contract/responses_grammar.rs
tests/openai_codex_responses.rs    # 一个 integration target
tests/openai_codex_responses/wire.rs
tests/openai_codex_responses/constrained_sampling.rs
tests/openai_codex_responses/ws_path.rs
tests/openai_codex_responses/ws_continuation.rs
tests/openai_codex_responses/ws_branches.rs
tests/openai_codex_responses/ws_cancellation.rs
tests/openai_codex_responses/ws_resources.rs
tests/openai_codex_responses/ws_protocol_regressions.rs
tests/openai_codex_responses/replay.rs
tests/openai_codex_responses/deflate_audit.rs  # 共享 helper，本身不拥有断言
tests/catalog_operations.rs        # 一个 builtins 目录装配 target
tests/catalog_operations/fixtures.rs
tests/catalog_operations/publication.rs
tests/catalog_operations/dispatch.rs
tests/catalog_operations/pricing.rs
tests/provider_collection.rs        # 一个 builtins 装配 target
tests/provider_collection/fixtures.rs
tests/provider_collection/auth_resolution.rs
tests/provider_collection/dispatch.rs
tests/provider_collection/builtin_registration.rs
tests/provider_collection/builtin_auth.rs
tests/provider_collection/builtin_chat.rs
tests/provider_collection/builtin_operations.rs
tests/provider_collection/azure.rs
tests/provider_collection/opencode.rs
tests/provider_collection/phase17.rs
tests/provider_collection/registry_preflight_and_refresh.rs
tests/provider_collection/catalog_store_refresh_20_33.rs
tests/provider_collection/helper_retry_hints.rs
tests/provider_collection/selected_owner_management.rs
tests/oauth_models_store.rs        # Models store 生命周期 target
tests/oauth_regressions.rs         # 共享跨 provider regression target
tests/tls_contract.rs              # TLS 公共合同 target 索引
tests/ws_upgrade.rs
tests/support/tls_fixtures.rs
tests/support/ws_peer.rs
tests/support/OWNERS.tsv
tests/support/oauth_fixtures.rs
tests/support/oauth_assembly.rs
tests/support/oauth_anthropic.rs
tests/support/oauth_codex.rs
tests/support/oauth_chatgpt.rs
tests/support/oauth_copilot.rs
tests/support/oauth_kimi.rs
tests/support/oauth_meta.rs
tests/support/oauth_xai.rs
tests/support/oauth_loopback.rs
tests/support/oauth_radius.rs
tests/support/oauth_store.rs
```

crypto 的生产模块仅保留测试声明：

```rust,ignore
#[cfg(test)]
mod tests;
```

少量私有测试可以继续内联。crypto 的准入、签名、worker、JWT 契约适合按主题分文件；
fixture 常量仍保留在原有私有、仅测试使用的 `src/crypto/fixtures.rs`。

[Rust Reference](https://doc.rust-lang.org/reference/items/modules.html#outlined-modules)
支持 `tests.rs` 和 `tests/mod.rs` 两种形式，并鼓励使用模块名文件以减少同名 `mod.rs`。
这里沿用现有 WS/crypto 的目录习惯，不重命名生产模块树。

Cargo 为每个发现的 integration target 生成独立可执行文件。新增 executable 前，
优先把相关案例组织为既有契约 target 的子模块；[Cargo 说明了编译/调度取舍](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#tests)。
共享辅助代码放在 `tests/support`，不再以根目录 `tests/*.rs` target 形式执行零个测试。
不要为测试私有机制扩大生产可见性，也不要向 integration tests 复制生产源码。

根 integration 文件显式声明主题子模块，例如：

```rust,ignore
#[path = "transcript_contract/provider_projection.rs"]
mod provider_projection;
```

即使只测试一个公开 Schema 函数，也可以属于 integration target；
反过来，多个私有模块的联合验证仍属于 crate 内部证据。
目录传达的是访问边界，不能替代具体行为断言。

Codex Responses 保留一个 executable，同时为请求 wire、约束采样、WebSocket
连接与 continuation、恢复、取消、资源清理和协议 regression 提供精确过滤器。
`deflate_audit.rs` 是共享 decoder helper，本身没有独立断言，修改它会正确选中整个 target。

provider collection 同样保留一个 executable，因为它跨越 builtins 装配根和多个
provider。其主题文件分别拥有 phase-17 执行状态机、registry preflight、catalog store
刷新、retry hint 和 selected-owner 管理。各 provider 的请求检查继续由同目录的
provider 模块拥有。五个主题文件提供限定模块过滤器，修改根共享 helper 或 builtin
索引时仍应运行完整 target。

Bedrock fixture target 保留一个 executable，覆盖 AWS credential source 和 Converse
协议。`wire.rs` 拥有 28 项 parser、event-stream、model、error 和 projection 断言。
`lifecycle.rs` 保留 25 项 HTTP、callback、request-header 和 prepared-call 断言。
`aws_credentials.rs` 现在是 credential-source 主题索引。其 `support.rs` 拥有共享 source
fixture 与 helper contract，`chain.rs` 拥有 15 项 environment/profile/process 优先级断言，
`role_sources.rs` 拥有 11 项 assume-role/MFA/web-identity 断言，`metadata.rs` 拥有 9 项
ECS/IMDS 断言，嵌套的 `lifecycle.rs` 拥有 11 项 Models、HTTP、取消和 frozen-process 断言。
`aws_sso.rs` 现在是 SSO 主题索引。其 `support.rs` 拥有共享 SSO、login、refresh 和 DPoP
fixture，`discovery.rs` 拥有 9 项 source discovery 与 federation 断言，`expiry.rs` 拥有
11 项 expiry 与 passive-refresh 断言，`login.rs` 拥有 8 项 login 与 DPoP 断言，`chain.rs`
拥有 3 项 precedence 与 throttling 断言。SSO 主题通过索引复用 credential helper。
`fixtures.rs` 拥有根 target 与各主题共享的 request、event-stream bytes、auth 和
response helper。`builtin_routes.rs` 拥有 8 项 builtin 注册与本地 endpoint 断言，
`constrained_guard.rs` 拥有 2 项 strict sampling 断言，`converse_adapter.rs` 现在是
adapter 主题索引。`projection.rs` 拥有 13 项 request 与 message projection 断言，
`stream.rs` 拥有 6 项 stream、stop-reason、usage 与 strict-tool 断言，`replay.rs`
拥有 5 项 persisted-history 断言，`lifecycle.rs` 拥有 4 项 Models lifecycle 与
route-contract 断言及 1 项可选 telemetry 断言。`support.rs` 负责 adapter 私有 helper，
本身不拥有断言。因此默认 feature 下共有 28 项，telemetry-tracing 另有 1 项。
`aws_source_transport.rs` 继续作为共享本地 transport helper。嵌套 selector 包括
`lifecycle::aws_credentials::chain::`、`lifecycle::aws_credentials::role_sources::`、
`lifecycle::aws_credentials::metadata::`、`lifecycle::aws_credentials::lifecycle::`、
`lifecycle::aws_sso::discovery::`、`lifecycle::aws_sso::expiry::`、
`lifecycle::aws_sso::login::`、`lifecycle::aws_sso::chain::`、
`converse_adapter::projection::`、`converse_adapter::stream::`、
`converse_adapter::replay::` 和 `converse_adapter::lifecycle::`。修改 adapter helper、
共享 fixture 或根索引时运行完整 target。

OpenAI Chat target 保留一个 executable。`wire.rs` 拥有 parser、request body、projection
和 transport 断言，`lifecycle.rs` 拥有 Models、计时、callback 和 source compatibility
断言，`fixtures.rs` 拥有共享 response、request 与本地 server helper。
`constrained_guard.rs` 保留 strict sampling 准入用例，作为可单独选择的 wire 子模块。
修改共享 fixture source 时运行完整 target。

OpenAI Responses fixture target 同样保留一个 executable。`wire.rs` 拥有 payload 与 SSE
映射、terminal 和 usage projection、请求路由、acquisition sealing 及 transport 断言。
`constrained_guard.rs` 拥有 grammar 与 strict constrained-sampling 准入和 stream projection
断言。`lifecycle.rs` 拥有 Models 注册、callback、native options、reasoning replay、计时和
source compatibility 断言。`fixtures.rs` 拥有共享 request、model、response 和 reasoning
builder。现有 `openai_responses_lifecycle` target 继续拥有独立的 HTTP 及 deferred
submit/fetch/cancel lifecycle contract。修改 `fixtures.rs` 或根索引时运行完整
`openai_responses_fixtures` target。

Vertex fixture target 保留一个 executable。`fixtures.rs` 负责共享 request、stream、
provider 和本地 response helper，本身不拥有断言。`builtin_routes.rs` 负责 builtin 注册与
跨 provider route 用例。`wire.rs` 负责 endpoint、auth、catalog、model resolution、request
body、SSE、tool、error 与 usage projection。`lifecycle.rs` 负责本地 HTTP、timeout、
cancellation、body limit 与 header routing。`adc_vertex.rs`、`adc_external.rs` 和
`adc_impersonation.rs` 分别负责三类 ADC credential-source contract。`callbacks.rs`、
`constrained_guard.rs` 和 `google_41.rs` 分别负责 callback transform、strict sampling
准入与 Google 4.1 adapter contract。package consumer 仍由 `provider-vertex` 负责。
修改共享 fixture 或根索引时运行完整 `vertex_fixtures` target。

Gemini fixture target 保留一个 executable。`fixtures.rs` 负责共享 provider、request、stream
和本地 stalled-server helper，本身不拥有断言。`wire.rs` 负责 provider/model resolution、
GenerateContent request body、SSE mapping、usage、error 与 stop reason。`lifecycle.rs` 负责
HTTP auth classification、request enrichment、timeout、cancellation 与 chunk split 行为。
`callbacks.rs` 负责 payload/raw/async observer contract。`constrained_guard.rs` 负责 strict
sampling 准入，`google_40.rs` 负责 Google 4.0 chat adapter、native thinking/tool choice、
signed replay 与 telemetry 用例。package consumer 仍由 `provider-gemini` 负责。修改共享
fixture 或根索引时运行完整 `gemini_fixtures` target。

Mistral fixture target 保留一个 executable。`fixtures.rs` 负责共享 provider、request、stream
和 stalled-server helper，本身不拥有断言。`chat_wire.rs`、`chat_lifecycle.rs` 和
`chat_callbacks.rs` 分别负责共享 OpenAI-compatible Chat 的 request/SSE、取消与 callback
契约。`native_conversations.rs` 是 native adapter 主题索引，目录下的 `fixtures.rs` 负责 native
builder 与 mount，`wire.rs`、`lifecycle.rs`、`replay.rs` 和 `callbacks.rs` 分别负责 Mistral
options、SSE、timeout/conversion、replay/cache 与 callback 行为。package consumer 仍由
`provider-mistral` 负责。修改共享 fixture 或根索引时运行完整 `mistral_fixtures` target。

OpenRouter fixture target 保留一个 executable。`fixtures.rs` 负责共享 Chat provider 与本地
peer helper，本身不拥有断言。`chat_wire.rs` 负责 OpenRouter-compatible 的 model、request、
SSE、usage 与 endpoint 用例，`chat_lifecycle.rs` 负责取消、compatibility 与 custom
registration lifecycle。`images.rs` 是 Images 主题索引。其下的 `images/fixtures.rs` 负责
image record、request builder 与 retry/timeout peer，`images/wire.rs` 负责 image request、
response、modality、usage 与 header projection，`images/lifecycle.rs` 负责 registration、
retry、timeout、admission 与 auth cancellation，`images/callbacks.rs` 负责 callback、observer
与 telemetry 行为。package consumer 仍由 `provider-openrouter` 负责。修改共享 fixture 或根
索引时运行完整 `openrouter_fixtures` target。

Azure OpenAI fixture target 保留一个 executable。`fixtures.rs` 负责共享 Chat provider、request、
SSE 与本地 peer helper，本身不拥有断言。`chat_wire.rs`、`chat_lifecycle.rs` 和
`chat_callbacks.rs` 分别负责 Chat request/SSE、传输取消与 callback 契约。`responses.rs` 是
native Responses 主题索引，其 `responses/fixtures.rs` 负责 model、request、SSE 与本地 peer
builder，`wire.rs`、`lifecycle.rs` 和 `options.rs` 分别负责 endpoint/stream 映射、执行计时以及
Azure Responses options、projection 与 replay 行为。package consumer 仍由
`provider-azure-openai` 负责。修改共享 fixture 或任一主题索引时运行完整
`azure_openai_fixtures` target。

Pi Messages fixture target 保留一个 executable。`fixtures.rs` 负责共享 model、request、SSE、
local-peer 和 projection helper，本身不拥有断言。`gateway.rs` 负责 Radius gateway 与 catalog
行为，`wire.rs` 负责 endpoint、option whitelist、cache retention 和 context projection，
`stream.rs` 负责 SSE block、稀疏 index、terminal marker 与 typed stream failure。
`lifecycle.rs` 负责 HTTP failure、cancellation、auth/model admission、simple bypass、wire
mismatch 和 Models completion，`callbacks.rs` 负责 callback ordering。`node5.rs` 是 replay
topic index，`node5/fixtures.rs` 负责 replay builder，`node5/replay.rs` 负责 producer/reload
regression。package consumer 保持为 `provider-pi-messages`。修改共享 fixture 或主题索引时，
运行完整 `chat_pi_messages` target。

认证契约 target 保留一个 executable。`errors.rs` 负责 typed credential failure、object-safe
auth contract 与 redaction 断言。`fixtures.rs` 负责共享 resolver/store context、OAuth
替身、存储 API key 构造函数，以及 native 与 impersonated ADC 共同使用的合成 service-account key。
`resolution.rs` 负责 lazy source selection、explicit override、
store precedence、logout 与 descriptor resolution。`mutations.rs` 负责串行 credential
mutation、barrier、receipt 与 passive refresh handoff。`adc_native.rs`、`adc_external.rs` 与
`adc_impersonation.rs` 分别负责一个 ADC source family，并拥有各自的 local transport fixture
和 lifecycle 断言。`scoped.rs` 负责 credential snapshot 与 redaction。外部 `oauth-store` 和
`cloud-auth` 打包消费者验证公开认证入口。修改共享 fixture 或根索引时运行完整 `auth_contracts` target。

`provider_collection` 索引装配一个 `builtins` executable。`auth_resolution.rs` 负责
凭据声明和探测结果，`dispatch.rs` 负责请求准入和流终止，`registry_preflight_and_refresh.rs`
负责注册表查询、预检和内存目录刷新。内置装配主题分别负责注册及目录投影、凭据绑定、聊天路由与
非聊天操作。Azure 和 OpenCode 的专属装配路径由各自的模块负责。`fixtures.rs` 负责共享
请求、脚本化响应、认证、刷新和投影 helper。修改主题文件时选中该主题，修改共享 fixture 或根索引时
选中完整 target。外部消费者在两种 feature 配置下执行归档里的 README 示例，验证 `register_builtin`。
[README 示例 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`catalog_operations` 使用 `builtins` 验证公开 SDK 中按操作类型区分的目录。
`publication.rs` 负责解码、刷新、原子发布与 store 恢复后的路由元数据。
`dispatch.rs` 负责操作选择、可用性检查和实际的 Chat、Images、Classifier 调用。
`pricing.rs` 负责未知价格、精确阶梯阈值及 fallback 价格的解码、回读与费用计算。
`fixtures.rs` 负责共享的混合操作目录、Anthropic 记录、认证替身和请求构造。
修改主题文件时选中对应模块，修改根索引或共享 fixture 时选中完整 target。
外部 `catalog-bundled` 消费者从实际 Cargo 归档验证嵌入目录及依赖图。动态发布、
操作分发和价格持久化由 `catalog_operations` 持有，也可以从解包后的归档执行。
[目录归档 runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

`execution_lifecycle` 在 `--no-default-features` 下验证公开执行合同。
`tests/execution_lifecycle/` 下的三个主题各自可按模块选测。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `ordinary.rs` | `ordinary::` | 普通调用和裸 `ChatStream::start` 的执行进度、初始化、事件顺序与类型化结果 |
| `deferred.rs` | `deferred::` | Deferred fetch 的就绪及结果时序，以及该 fetch 的源释放与 poll 结算 |
| `terminal_release.rs` | `terminal_release::` | started、普通和 prepared 调用的 EOF 与源释放，包括 abort 等待和 attempt 封存 |

`fixtures.rs` 负责目标内共用的受控 channel、尾部屏障和 chat 调用搭建。修改它或根索引会选中三个主题。
可把主题路径交给 selector，也可在 `cargo test --locked --offline -p opi-ai --no-default-features --test execution_lifecycle` 后加 `--` 和对应筛选。

外部最小特性消费者从实际归档运行 `core_profile`，覆盖核心普通调用与 Faux Deferred fetch。
解包后的 `execution_lifecycle` target 验证 abort、尾部释放及缓存结果的时序。
[最小特性归档 runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

`telemetry_operations` 负责公开 SDK 操作的冻结上下文，需要 `builtins,telemetry-tracing`，
使用本地响应端核对实际请求。`tests/telemetry_operations/` 下的主题可独立选测。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `chat.rs` | `chat::` | native、simple 与 direct Chat、任务推进、poll、Drop 和 no-op 上下文 |
| `prepared.rs` | `prepared::` | prepared Anthropic 请求、严格工具准入、重试及回调父 span |
| `images.rs` | `images::` | OpenRouter 图像请求和回调、结果一致性与上下文隔离 |
| `classifier.rs` | `classifier::` | System One 普通和 native 分类、回调、结果一致性与上下文隔离 |
| `deferred.rs` | `deferred::` | 提交、fetch、取消、放弃 fetch 和任务句柄序列化 |

`fixtures.rs` 负责记录订阅者、ambient poll、脱敏标记以及 Chat 与 Deferred 共用的流。
根索引和共享 fixture 选中完整 target。各操作的输入与响应为对应主题持有。
修改 `src/api/system_one.rs` 时，此 target 只选中 `classifier_calls_freeze_context`。

外部 README 消费者验证归档里的 builtins 有无 tracing 的两种配置。解包后的
`telemetry_operations` target 验证冻结的 dispatch 和父 span、回调作用域及请求载荷隔离。
[README 归档 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`request_enrichment` 负责请求选项及其在 provider 请求中的实际表现，使用 `builtins` 和本地
HTTP 响应端。`tests/request_enrichment/` 下的四个主题各自可按模块选测。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `scalars_and_timeout.rs` | `scalars_and_timeout::` | 请求默认值、显式字段、采样和超时终止 |
| `protected_headers.rs` | `protected_headers::` | 请求头准入、转换，以及认证和路由标识保护 |
| `session_affinity.rs` | `session_affinity::` | 会话请求头、缓存键、保留策略及调用方覆盖 |
| `callbacks.rs` | `callbacks::` | payload 形状与预算、观察回调顺序、重试复用冻结请求字节 |

`fixtures.rs` 持有共用的请求构造、HTTP 请求记录读取工具，以及请求获准后的失败投影断言。
修改它或根索引会选中四个主题。Responses affinity 的参数矩阵归入 `session_affinity` 子模块。

外部 `protocol-adapters` 消费者从实际 Cargo 归档验证公开导出和特性依赖图。
解包后的 `request_enrichment` target 用 `builtins` 和本地响应端验证四个主题的行为。
[协议归档 runner](../../../scripts/pi-sdk-conformance/run_protocol_adapters_package.py)

`node4_preparation` 使用 `builtins` 和本地 fixture 验证 Models 请求准备。`tests/node4_preparation/`
下的三个主题各自可按模块选测。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `call_limits.rs` | `call_limits::` | Chat 和 Responses 的 native/simple 调用限额、默认值及上下文预算 |
| `catalog_restore.rs` | `catalog_restore::` | Models store 重开、目录发布归属、刷新失败及已发布目录撤下 |
| `projection.rs` | `projection::` | 各调用入口的请求投影、重试归属及认证、发出请求、EOF 和取消的失败时序 |

`fixtures.rs` 持有本 target 共用的请求、Chat SSE 和请求体读取工具。它和根索引选中完整
target。`catalog_restore.rs` 直接引用跨 target 的 `tests/support/catalog_store.rs`，该路径
会选中清单登记的所有完整消费者。[共享 fixture 清单](support/OWNERS.tsv)

外部 `catalog-bundled` 消费者从实际归档验证嵌入目录及特性依赖图。三个行为主题由解包后的
`node4_preparation` target 使用 `builtins` 验证。
[目录归档 runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

`http_acceptance` 负责 HTTP transport 公共合同，使用 `builtins`，通过 `HttpClient`
和 Models Chat 驱动本地 TCP 或 TLS peer。可向选择器传入 `tests/http_acceptance/`
下的主题路径，或直接使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `response_parsing.rs` | `response_parsing::` | HTTP/1 响应头大小与数量、chunk framing、trailer 和 EOF 接受边界 |
| `redirects_and_disconnects.rs` | `redirects_and_disconnects::` | 同源和跨源重定向、受保护请求头、发送预算及响应接收前后的失败 |
| `https_proxy.rs` | `https_proxy::` | 外层 TLS、CONNECT、内层 TLS 和限定范围的 `NO_PROXY` 绕行 |

`fixtures.rs` 持有共用的请求读取器、脚本 peer、请求记录和 Models Chat 调用驱动。
根索引和这个 fixture 都选中完整 target。解析边界响应、重定向脚本及 TLS/CONNECT
组装随各自主题维护。证书和密钥来自共享的 `tests/support/tls_fixtures.rs`，其路径
选中全部六个消费者。[共享 fixture 清单](support/OWNERS.tsv)

外部 `consumer-http` 在 ordinary 和 caller-union 依赖图下验证公开 Models 和 HTTP
transport 请求，包括 HTTP 代理和绕行。解包后的 `http_acceptance` 验证细粒度解析边界
和 HTTPS 隧道。[HTTP 归档 runner](../../../scripts/pi-sdk-conformance/package_slice.py)

`tls_contract` 负责 TLS transport 公共合同，使用 `builtins` 和本地真实 TLS peer。
可通过 `tests/tls_contract/` 下的主题路径选测，也可将筛选传给
`cargo test --locked --offline -p opi-ai --no-default-features --features builtins --test tls_contract`。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `negotiation_and_trust.rs` | `negotiation_and_trust::` | TLS 版本、H1/H2 ALPN、密钥交换、信任根、主机名和证书有效期 |
| `mtls.rs` | `mtls::` | 客户端身份准入、验证和逐连接隔离 |
| `cancellation.rs` | `cancellation::` | 握手和响应取消、EOF、传输释放及 H2 拒流后的发送次数 |
| `provider_isolation.rs` | `provider_isolation::` | rustls 进程全局状态，以及独立子进程中实例 TLS 与宿主 provider 的隔离 |

目标内的 `fixtures.rs` 负责本地 TLS peer 及 HttpClient 到 Models Chat 的调用驱动。
它和根索引都选中完整 target。共享的 `tests/support/tls_fixtures.rs` 负责证书和密钥
材料，其路径选中包括 `ws_upgrade` 在内的六个消费者。
[共享 fixture 清单](support/OWNERS.tsv)

外部 `crypto` 归档消费者使用自己的测试 peer 和信任材料，在 ordinary 和 caller-union
依赖图下验证 TLS/Models 公共请求。解包后的 `tls_contract` 验证四个主题，包括取消和
清理。`ws` 归档系列
验证共享证书在 Upgrade/WSS 边界的使用。[归档 runner](../../../scripts/pi-sdk-conformance/package_slice.py)

`external_sdk` 由 `models` 负责，使用 `builtins` 和受控测试 peer。可向选择器传入
`tests/external_sdk/` 下的主题路径，或直接使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `chat_matrix.rs` | `chat_matrix::` | 单个 Models 实例的十条 Chat 路由，覆盖内置注册、注入认证和连接本地 peer 的协议适配器 |
| `classifier_and_images.rs` | `classifier_and_images::` | System One、Workers AI、OpenAI Decisions 和 llama.cpp 分类的类型化分发，以及 OpenRouter 图像生成 |
| `models_contract.rs` | `models_contract::` | 选项准入、回调、计时、工具续接、实例隔离、构造、零 I/O 查询、合法大集合及显式 Deferred fetch |

`fixtures.rs` 负责 Chat 和 Models 共享的认证读取记录器、请求构造器和 SSE 应答。
专用 peer、应答和断言就地维护。根索引和共享 fixture 路径均选中完整 target。生产路径
仍可单独选中 `assembled_matrix_drives` 和
`classifier_and_image_apis_answer_typed_through_one_instance` 两条具名用例。

外部 `consumer-readme` 在两种调用方 feature 配置下执行 Cargo 归档附带的三个示例。
解包后的 `external_sdk` target 验证上述完整矩阵和公共合同。
[README 归档 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`chat_capture_replay` 由 `replay` 负责，使用 `builtins` 和本地受控 peer 验证真实
生产者输出与续接请求。可向选择器传入 `tests/chat_capture_replay/` 下的主题路径，
或直接使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `producer_capture.rs` | `producer_capture::` | 五组内置生产者的 provider 与 wire 身份、推理内容、用量和终止事实 |
| `persistence_restore.rs` | `persistence_restore::` | 同五组结果恢复后的 AssistantMessage 一致性和规范 JSON 字节往返 |
| `target_projection.rs` | `target_projection::` | 十三条捕获、恢复、续接链路，覆盖 Chat grammar、Azure、Responses、Codex 和 Anthropic |

捕获和恢复主题的每组场景均以一次生产者请求结束。`fixtures.rs` 负责共用场景、
路由组装、SSE peer、请求构造和 JSON 恢复助手，它与根索引都选中完整 target。
投影专用响应与断言随各场景维护，两块子主题可分别用
`target_projection::replay_boundaries::` 和
`target_projection::persisted_producers::` 选测。

外部 `consumer-readme` 负责执行归档示例，包括从公开 API 捕获工具调用并发送
带工具结果的续接请求。解包后的 `chat_capture_replay` 验证 JSON 恢复及内置、
Codex 和 Anthropic 路径的实际请求投影。
[README 归档 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`replay_contract` 由 `replay` 持有，以 `builtins` 验证重放请求。可向选择器传入
`tests/replay_contract/` 下的主题路径，或使用对应筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `association.rs` | `association::` | 工具调用和结果配对、分发前校验及失败终止 |
| `execution_effects.rs` | `execution_effects::` | 缺失结果、失败轮次、未知影响和已持久化的未执行事实 |
| `malformed_arguments.rs` | `malformed_arguments::` | 字符串与对象参数投影及脱敏失败上下文 |
| `wire_ids.rs` | `wire_ids::` | Responses item ID、请求内 ID 配对及冲突处理 |
| `content_projection.rs` | `content_projection::` | 推理身份、Chat 可见文本和工具结果图片 |
| `object_wire.rs` | `object_wire::` | Gemini 和 Bedrock 对象参数、可信事实及有效配对 |
| `responses_grammar.rs` | `grammar_replay_tests::` | Responses 自定义调用与结果及 `ctc_` item ID |

根索引和 `fixtures.rs` 选中完整 target。共享 fixture 提供本 target 的历史与请求。
`chat_capture_replay` 负责生产者输出的捕获和恢复。外部 `consumer-readme` 通过公开 API
在第二次请求中重放已记录的调用和关联结果。解包 SDK 可运行完整 `replay_contract`。
[README 归档 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`retry_backoff` 由 `retry` 负责，使用 `builtins` 验证公共重试合同。可向选择器传入
`tests/retry_backoff/` 下的主题路径，或使用下表的 libtest 筛选词。根索引选中完整 target。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `response_headers.rs` | `response_headers::` | Retry-After 秒数与 HTTP-date、限流重置时间、优先级和无效输入 |
| `backoff_policy.rs` | `backoff_policy::` | 指数退避、延迟上限、尝试次数及 provider 错误是否可重试 |
| `acquisition.rs` | `acquisition::` | 类型化获取失败、真实 OpenAI Chat 发送限额、payload 冻结、取消，以及响应接收或发送状态未知时的封闭 |
| `smithy.rs` | `smithy::` | AWS 重试模式与配置优先级、错误分类、抖动、提示上界、配额和自适应限流 |

响应头主题负责 HTTP-date 格式化助手。请求获取主题负责本地 SSE 脚本和脚本化 provider
组装，并复用既有的 `opi_ai::test_support` mock provider。Smithy 负责自身的确定性时钟、
输入和错误 fixture。

外部 `consumer-http` 在两种 feature 依赖图下验证类型化的默认获取失败，以及响应接收后
发生 EOF 时只发送一次。解包后的 `retry_backoff` 验证四个主题，包括获准的重试及 Smithy
策略。`bedrock_retry` 负责 Bedrock Converse 专属重试及请求重建。`whole_call_retry`
负责已经结束的调用的显式整次重试。
[HTTP 归档 runner](../../../scripts/pi-sdk-conformance/package_slice.py)

`whole_call_retry` 由 `retry` 持有，使用 `builtins` 验证整次调用重试的公共合同。
可向选择器传入 `tests/whole_call_retry/` 下的主题路径，或使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `classification.rs` | `classification::` | 规范助手消息的重试判定、配额与上下文溢出否决、瞬时错误匹配 |
| `call_and_callbacks.rs` | `call_and_callbacks::` | 新逻辑调用次数、重试预算、回调、生产者终止结果及 Anthropic 和 OpenAI 的实际发送 |
| `backoff_and_cancellation.rs` | `backoff_and_cancellation::` | 延迟上限、调用方取消退避，以及 OpenAI 系 429 响应体尚未读取完时的连接释放 |

根索引与 `fixtures.rs` 选中完整 target。这个 fixture 持有目标内共用的规范消息、请求
构造、重试策略和结束回调记录。Anthropic SSE 和 OpenAI 配额响应随各自 wire 模块维护，
可分别用 `call_and_callbacks::frozen_target_wire::` 和 `call_and_callbacks::quota_transport::`
选测。

`tests/data/responses-failed.json` 归 OpenAI Responses 失败协议 fixture，由
`call_and_callbacks::responses_failed_sse_reaches_the_explicit_whole_call_policy`、
`openai_responses_fixtures` 的 `wire::responses_sse_error_preserves_the_callers_recovery_decision`
和 Responses 差分探针共同消费。传入该数据路径会得到扩检提示。

外部 `consumer-http` 负责 OpenAI Chat 的公开请求获取及 EOF 单次发送边界。解包后的
`whole_call_retry` 验证三个主题的显式整次重试合同，包括回调时序。
[HTTP 归档 runner](../../../scripts/pi-sdk-conformance/package_slice.py)

`provider_lifecycle` 由 `provider` 持有，使用 `builtins` 验证协议 wire。可向选择器
传入 `tests/provider_lifecycle/` 下的主题路径，或使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `request_and_errors.rs` | `request_and_errors::` | 实际 Anthropic POST 的认证头及 JSON 字段、401/429/500 类型映射、重试提示与终止错误 |
| `sse_decoding.rs` | `sse_decoding::` | Anthropic 文本和工具事件、缺失终止事件、按字节拆帧、UTF-8 替换及最终工具参数 |
| `cancellation_and_release.rs` | `cancellation_and_release::` | 观察者退出后的已启动调用、等待期间的取消，以及 Gemini/Bedrock 在响应前和响应中的连接释放 |

请求构造和成功文本 SSE 由目标内共用的 `fixtures.rs` 持有。根索引与该 fixture
选中完整 target。类型化错误断言、逐字节 SSE peer 和按阶段阻塞的连接 peer 由各自
主题维护。

外部 `consumer-readme` 执行 Anthropic 公开示例，验证 POST 和认证、消费前启动调用
以及工具续接。精确的错误映射、解码边界及 socket 释放由解包后的
`provider_lifecycle` 验证。[README 归档 runner](../../../scripts/pi-sdk-conformance/run_readme_examples.py)

`deferred_contract` 由 `provider/operation` 持有，在关闭默认 feature 时验证公共合同。
可向选择器传入 `tests/deferred_contract/` 下的文件路径，或使用下表的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `handle_validation.rs` | `handle_validation::` | 精确 API 注册、句柄 JSON、身份与到期检查，以及认证或分发前拒绝非法 fetch/cancel |
| `submission_and_fetch.rs` | `submission_and_fetch::` | 普通生成、`wait0` 单次获取、未知任务和保留规范重试输入的类型化认证失败 |
| `cancellation.rs` | `cancellation::` | 本地中止、尽力远端取消与再次获取，以及认证前或认证期间的取消 |
| `reopen.rs` | `reopen::` | 调用方检查点与时钟、重开后获取且不重复提交、模型缺失和取消失败报告 |
| `telemetry.rs` | `telemetry::` | 最小 SDK 显式转发遥测 carrier，默认调用不隐式捕获 |

根索引和 `fixtures.rs` 选中完整 target。脚本化路由和请求构造只在目标内部复用，
检查点与调度器归 `reopen.rs`。共用的 `tests/support/pending_auth.rs` 分别选中完整的
`classifier_contract`、`deferred_contract` 和 `openrouter_fixtures`。

外部 `opi-minimum-consumer` 在 core 和 core-tracing 配置中验证公开 Faux 提交及显式获取
从 pending 到 ready。其余合同由实际解包后的 SDK target 验证。重放取消路由观测时，
用 `--exact --nocapture` 选测
`handle_validation::cancel_checks_all_deferred_route_fields_before_auth_or_dispatch`。
生命周期差分使用独立可执行程序和上游观测记录。
[最小特性归档 runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

`public_utilities` 由 `utilities` 持有，使用 `builtins`。每个主题可按
`tests/public_utilities/` 下的文件路径或 libtest 筛选独立运行。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `rendering.rs` | `rendering::` | 可见文本提取、系统段落顺序和只读渲染 |
| `tool_arguments.rs` | `tool_arguments::` | 流式 JSON 预览、最终参数修复和可移植字符串枚举 schema |
| `context_overflow.rs` | `context_overflow::` | 根据诊断、结束原因、用量和窗口判断上下文溢出及可恢复长度 |
| `provider_diagnostics.rs` | `provider_diagnostics::` | 类型化 provider 错误码和脱敏诊断 |
| `identity.rs` | `identity::` | 短哈希 fixture、UUIDv7 格式、取值范围、序列和时钟回退 |
| `provider_configuration.rs` | `provider_configuration::` | 请求头优先级与删除、局部环境取值与回退 |
| `cancellation.rs` | `cancellation::` | 可取消定时器、future 释放和 token 单向传播 |
| `grammar_property_names.rs` | `grammar_property_names::` | grammar helper 和 Responses 请求构造中的字面属性名 |

各主题持有自己的 fixture，环境锁随 provider 配置用例维护。根索引选中完整 target。
已有 `grammar_property_names::*` 具名路径仍可精确选测。

外部 `opi-protocol-adapters-consumer` 在仅协议适配器的 feature 依赖图中调用公开
header 和 env API。完整断言由实际解包后以 `builtins` 运行的 `public_utilities`
验证。回环 Chat 参数预览差分由 `chat_argument_preview` 使用独立 fixture 执行。
[协议适配器归档 runner](../../../scripts/pi-sdk-conformance/run_protocol_adapters_package.py)

`schema_engine` 由 `schema` 持有，可关闭默认 feature 运行。可传入
`tests/schema_engine/` 下的文件路径，或使用对应的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `corpus.rs` | `corpus::` | 冻结的 64 条差分输入及已批准的预期 |
| `value_keywords.rs` | `value_keywords::` | 值类型检查、标量限制和 JavaScript 等值及数值语义 |
| `container_keywords.rs` | `container_keywords::` | 数组与对象的数量边界、元素、属性及依赖 |
| `applicators.rs` | `applicators::` | 分支选择及已校验属性与元素的传播 |
| `reference_resolution.rs` | `reference_resolution::` | 指针、锚点、资源身份、枚举顺序和求值递归 |
| `admission.rs` | `admission::` | 编译诊断与资源遍历，包括递归 frame |
| `stack_safety.rs` | `stack_safety::` | 在子进程的 2 MiB 线程栈上验证深层编译、校验、引用查找和释放 |

根索引和 `fixtures.rs` 选中完整 target，共用 helper 使用公开 compile/check API。
`corpus.rs` 持有 `tests/data/schema-cases.jsonl` 和 `tests/data/schema-expected.json`
的断言。修改数据时可直接运行 `corpus::`，并查看选择器的扩检提示。深栈测试使用带模块
前缀的名称选中子用例，验证执行完成后才通过。

外部 `opi-sdk-conformance-consumer` 在两种 feature 配置下各运行 73 项具名 Schema
检查。实际 SDK 归档还可运行完整的 `schema_engine` 集成测试。工具参数归一化和公开
错误路径由 `schema_public` 持有。
[Schema 归档 runner](../../../scripts/pi-sdk-conformance/run_early_package_proof.py)

`custom_provider_registration` 由 `models` 持有，可关闭默认 feature 运行。
可传入 `tests/custom_provider_registration/` 下的文件路径，或使用对应的 libtest
筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `registry_providers.rs` | `registry_providers::` | Provider 注册、替换、查询和模拟流式调用 |
| `registry_models.rs` | `registry_models::` | 模型覆盖的校验、优先级、能力和确定性列表 |
| `chat_registration.rs` | `chat_registration::` | API ID 精确匹配、目录可见性和自有模型记录查询 |
| `chat_dispatch.rs` | `chat_dispatch::` | 显式记录、native/simple 选项和直接传入的认证 |
| `chat_lifecycle.rs` | `chat_lifecycle::` | 绑定快照、消费前执行和自有调用生命周期 |
| `chat_admission.rs` | `chat_admission::` | 认证前检查能力、API ID、选项类型、provider 归属和取消 |

根索引和 `fixtures.rs` 选中完整 target。fixture 提供本 target 共用的注册表 mock、
类型化 Chat 适配器及可观察的认证解析器。外部 `opi-minimum-consumer` 从实际 SDK
归档复制这组测试，在 core 和 tracing 两种配置下运行。解包 SDK 也可直接运行完整
集成测试。[最小特性归档 runner](../../../scripts/pi-sdk-conformance/run_minimal_package.py)

`model_wire_metadata` 由 `types/model` 持有，使用 `builtins`。可传入
`tests/model_wire_metadata/` 下的主题路径，或使用相应的 libtest 筛选词。

| 主题 | 筛选 | 负责验证 |
| --- | --- | --- |
| `model_contract.rs` | `model_contract::` | wire 名称、模型校验、路由 JSON、八个内置目录及配置式 Azure |
| `thinking_admission.rs` | `thinking_admission::` | provider 在 HTTP 前拒绝不支持的推理等级，并保留模型元数据 |
| `sampling_parameters.rs` | `sampling_parameters::` | 缺省和显式空值、浅合并及模型、等级、请求的优先级 |
| `sampling_wire.rs` | `sampling_wire::` | Chat、Responses、Azure 和 Anthropic 的请求投影与 payload 回调 |
| `pricing_usage.rs` | `pricing_usage::` | 费率校验、严格阶梯阈值及用量 service tier 往返 |
| `anthropic_complete_metadata.rs` | `anthropic_complete_metadata::` | Anthropic 完整目录记录的默认值 |
| `anthropic_response_cost.rs` | `anthropic_response_cost::` | fallback 声明和归因、缓存写入阶梯、未知与零成本 |

根索引和 `fixtures.rs` 选中完整 target。请求和采样数据构造器由本 target 持有。
外部消费者只启用 `catalog-bundled`，加载并重新解析归档内嵌的目录资源。解包 SDK 以
`builtins` 运行完整的元数据、回环请求和计价测试。
[目录归档 runner](../../../scripts/pi-sdk-conformance/run_catalog_bundled_package.py)

使用现有[共享 Cargo 缓存](../../../scripts/opi-cargo-cache.py)，保持增量编译，
使用本地 fixture/peer，不使用实时凭据：

```powershell
cargo test --locked --offline -p opi-ai --lib crypto::tests::
cargo test --locked --offline -p opi-ai --lib crypto::tests::worker::
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test openai_codex_responses
cargo test --locked --offline -p opi-ai --test http_acceptance --test tls_contract --test ws_upgrade
```

WS 归档验收在三种依赖图下执行私有协议测试、公开 Upgrade 测试和外部消费者的
Models → Codex → Upgrade → codec 调用。crypto 验收单独跟踪生产 JWT/ADC 调用方。
[SDK 归档检查](../../../scripts/pi-sdk-conformance/README.md)
