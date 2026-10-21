# opi-ai 测试与验收方案

[English](opi-ai-test-acceptance-plan.md)

执行证据：[2026-10-21 执行结果](opi-ai-test-acceptance-results.zh.md)。

后续执行：[证据闭合与测试目录整理](opi-ai-test-closure-results.zh.md)。
缺口表保留最初检查状态，后续记录说明已闭合内容，不为新生产调用方提供实施准入。
当前目录约定见[测试组织](../../crates/opi-ai/tests/README.zh.md)。

## 范围与结论

私有机制测试保留在实现附近，调用者行为通过既有公开接口验收。一个验收单元可以
跨越多个 Rust 模块，不要求新增 crate、trait、feature 或测试二进制。

这是面向 `opi-ai` 的非规范性测试组织方案，日期为 2026-10-21。盘点基线是 HEAD
`7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf` 加当前工作区。其中，
`tests/ws_upgrade.rs`、`tests/support/ws_peer.rs` 是已有未跟踪文件，WS 测试、crate
README 和打包脚本也已有本地修改。下文“已有覆盖”指存在相应测试代码，不表示
本轮执行通过；执行前需要重新核对这些输入。

规范依据仍为 [PRIN-002、PRIN-005](../opi-spec.md#31-minimal-and-deep)、
[SDK 方向](../opi-spec.md#32-alignment-with-pi)，以及
[组装产品验收和迁出条件](../opi-spec.md#42-reference-product-ownership)。本方案
不构成实现准入，不改变已注册交付门禁，不写实现账本，也不授权拆包或发布。

## 测试层次与完成条件

| 层次 | 测试面 | 完成证据 |
| --- | --- | --- |
| 私有机制 | crate 内状态、解析器、算法、调度和资源所有权 | 确定性的成功、非法输入和状态迁移断言；保留 fixture/oracle；实际执行的测试 |
| 公开模块 | Schema、transcript、replay 等受支持函数和值 | 可观察结果、类型化失败、输入不变性和已声明语义差异 |
| 公开接口组合 | `Models`/prepared call、真实 adapter、`HttpClient`、本地 peer 和具体 store | 正确 wire 字节和发送次数、来源选择、失败阶段、终止结果、取消和资源回收 |
| SDK 包 | 实际 `opi-ai.crate`、解包后的测试和独立外部公开消费者 | 包/lock/toolchain 身份、依赖 feature、原生构建来源及各场景执行记录 |
| 工作区与产品 | 跨 crate 调用方和组装后的 Reference Product | 适用工作区门禁，以及所属已注册端到端验收场景 |

crate 内测试和解包后的私有测试不能替代公开消费者。公开 Upgrade 成功不能替代
生产 codec 调用链。Cargo filter 仍会构建所在 crate 和依赖，不形成独立构建或发布单元。

每个选定验收范围都应记录具体契约、命令、源码状态、选中测试名/数量、结果和
未覆盖范围。退出码为零但选中零个测试不能验收。失败/忽略项和平台证据缺失必须
保留。只有确认源码、fixture、feature、工具链和平台仍适用，才复用已有执行证据。

## 模块归属与现有覆盖

命令组名称对应后文命令。保留现有测试布局；逻辑分组不要求移动或复制测试。

| 验收单元 | 保留私有 | 公开验收与现有覆盖 |
| --- | --- | --- |
| WS | [src/ws/tests](../../crates/opi-ai/src/ws/tests/mod.rs) 的 frame/PMD/Close/IO 状态、部分进度、取消、背压、压缩 oracle，以及内部 Upgrade→Driver 组合 | **Transport**：[ws_upgrade](../../crates/opi-ai/tests/ws_upgrade.rs) 覆盖严格 101、协商、WSS/H1、trust、mTLS、CONNECT 和 detached raw bytes。公开 bridge 不启动私有 codec。保留三种已声明 feature graph。 |
| Crypto | [crypto 测试](../../crates/opi-ai/src/crypto/tests/mod.rs) 中的密钥准入、签名、独立验签、JWT 向量、worker 取消和密钥生命周期 | **Transport**：TLS identity admission 和 mTLS 消费密钥准入 owner。TLS 的握手签名交给 rustls，不经过 `WorkerSigning`。将来的真实签名调用方需提供自身公开链路验收。 |
| TLS | 冻结的 rustls 配置、显式 provider 和 root bundle 保持私有，不导出测试专用 verifier/config | **Transport**：[tls_contract](../../crates/opi-ai/tests/tls_contract.rs) 经公开 TLS options、`HttpClient` 和 Models/Chat 验收。已有 TLS 1.2/1.3、H1/H2、支持的 identity、隔离、I/O 前类型化拒绝、trust/hostname/expiry/EKU/KX 负例、取消、partial usage/EOF、全局 provider 独立性。没有已建立的 `--lib tls::` 测试组。 |
| HTTP 与 retry | [retry](../../crates/opi-ai/src/retry.rs) 内的 acquisition lease/readiness/seal、endpoint 构造和传输内部细节 | **Transport**：[http_acceptance](../../crates/opi-ai/tests/http_acceptance.rs)、[retry_backoff](../../crates/opi-ai/tests/retry_backoff.rs)、[proxy_support](../../crates/opi-ai/tests/proxy_support.rs)、[connection_pooling](../../crates/opi-ai/tests/connection_pooling.rs)。已有重定向、受保护 header、代理分层、精确发送数、冻结 auth/payload、body 失败和交接后禁止重试。连接池配置/Arc identity 与实际 socket 复用分开验收。 |
| Schema | IR、求值栈、递归证明、formats、Number/URL/IDNA/hash helper | **Schema**：[schema_engine](../../crates/opi-ai/tests/schema_engine.rs)、[schema_public](../../crates/opi-ai/tests/schema_public.rs) 经 `compile/check/validate_tool_arguments/validate_tool_call` 覆盖冻结语料、无进展失败、归一化、最终按原 schema 校验、输入不变和脱敏报告。工具声明/provider 投影仍需 **Canonical** 和 **Adapters**。 |
| Credential store 与 mutation | 队列调度和 bookkeeping 保持实现细节，不为测试额外导出接口 | **Credentials**：[auth_contracts](../../crates/opi-ai/tests/auth_contracts.rs) 验收公开 store/queue/receipt/barrier 契约、per-provider FIFO、runtime handoff、同步 admission、waiter cancellation、共享 store cutoff 和 unknown/late outcome。它们是 integration tests，没有已建立的 `--lib mutation` 测试组。具体持久 store 还需后文跨 crate 检查。 |
| Models 与 auth 生命周期 | auth 来源解析、secret formatting、binding 和 lease 实现 | **Credentials**：[provider_collection](../../crates/opi-ai/tests/provider_collection.rs)、[custom_provider_registration](../../crates/opi-ai/tests/custom_provider_registration.rs)、[per_request_auth](../../crates/opi-ai/tests/per_request_auth.rs)、`auth_contracts`、`oauth_wire_shape`。已有优先级/失败不回退、resolve once、冻结重试、lazy dispatch、单 active attempt、取消、typed call 入口，以及 provider 替换后旧 binding 继续有效。 |
| Transcript | section 顺序 patch、工具 delta、collapse/rendering helper | **Canonical**：[transcript_contract](../../crates/opi-ai/tests/transcript_contract.rs) 覆盖规范化 chronology、serde admission、工具声明、投影和 canonical 不变性。`adapters_consume_only_the_normalized_transcript` 实际只运行 Anthropic，复数测试名不能证明全部 adapter 一致性。 |
| Replay | batch association、loss fact、identity/thinking 规则和 ID legalization | **Canonical**：[replay_contract](../../crates/opi-ai/tests/replay_contract.rs) 检查真实 request body、缺失结果的 unknown effect、严格模式失败、畸形参数和 ID 双射。`association_matrix_through_real_adapters` 实际运行 Anthropic 和 OpenAI Chat。tool-result wire/leak 测试补充覆盖，但不直接证明所有 wire/capability 组合。 |
| Context、usage 与 cost | 估算/算术、饱和累加和定价计算不需要新公开接口 | **Accounting**：[context_budget](../../crates/opi-ai/tests/context_budget.rs)、[usage_cost](../../crates/opi-ai/tests/usage_cost.rs)、[stream_events](../../crates/opi-ai/tests/stream_events.rs)。还需组合 `provider_collection` 证明按当前 context 限制 output/thinking；算术正确不能替代 prepare 实际消费估算结果的验收。 |
| Registry、metadata 与 catalog | model spec 解析、metadata 校验可局部检查 | **Catalog**：[registry](../../crates/opi-ai/tests/registry.rs)、[model_wire_metadata](../../crates/opi-ai/tests/model_wire_metadata.rs)、migration tests、[api_mapped_provider](../../crates/opi-ai/tests/api_mapped_provider.rs)。与 `provider_collection` 一起，已有替换、wire/compat 校验、原子 refresh/rollback，以及三个真实 wire adapter 的 dispatch。Refresh 当前是调用者负责的 substrate，不是 coding-agent 生产刷新功能。 |
| Bedrock event stream | [event_stream](../../crates/opi-ai/src/bedrock/event_stream.rs) 内 CRC、帧边界、残帧保留和逐字节分片测试 | **Bedrock**：[bedrock_fixtures](../../crates/opi-ai/tests/bedrock_fixtures.rs) 还覆盖 HTTP draining、terminal mapping、truncated trailer 和类型化取消。filter 用整个 `bedrock::event_stream::`，同时包含 `tests` 与 `split_frame_tests`。fixture-only cancellation 明确为 no-op；signed HTTP 路径已有取消测试。 |
| Bedrock SigV4 | [sigv4](../../crates/opi-ai/src/bedrock/sigv4.rs) 内固定签名/派生向量、canonical header、空白和 session-token 规则 | **Bedrock**：`stream_prepared` 测试检查 signature/session-token header 保护，以及跨 retry 的 resolve-once frozen auth。纯签名通过不等于实际 path/header/body 构造正确。它与私有 `crypto` owner 是不同模块。 |
| Provider adapter 与共享 stream 类型 | SSE/event mapper、UTF-8 decoder、producer/queue 取消保留局部测试 | **Adapters**、**Fixtures**、**Wire**、**Accounting**：fixture 证明 mapping；本地 HTTP lifecycle 证明 dispatch、wire shape、typed error、terminal/EOF 和取消。`ChatStream` 测试约束 iteration 与 `result` 共享一个 outcome。保留 provider trait/error/diagnostic 和 test-support 测试。 |

## 缺口与限定的后续工作

以下区分生产链路缺失、测试判别力不足，以及仍待映射的证据；都不是本轮新发现的
运行时测试失败。

| 缺口或局限 | 需要的验收证据与下一步 |
| --- | --- |
| WS 生产链路尚不存在 | [ws.rs](../../crates/opi-ai/src/ws.rs) 明确 codec 没有生产调用方；[Codex](../../crates/opi-ai/src/openai_codex_responses.rs) 当前使用 HTTP/SSE。在已准入调用方落地前，Models→Codex→Upgrade→codec、continuation/pool 和完整 cleanup 验收保持开放。增加私有测试不能关闭它。 |
| Crypto worker 没有生产调用方 | `sign_on_worker` 当前只有私有测试调用。保留这些测试；已准入 JWT/ADC 调用方落地后，再验收其公开请求、取消、所有权和 late-result suppression。TLS 握手取消是另一种证据。 |
| 连接池测试未证明 socket 复用 | `connection_pooling` 检查 settings 和 Arc identity；主要 HTTP/TLS helper 禁用了 idle pooling。只补有区别的公开 H1 keep-alive 场景：一个共享 client、两次完整消费的调用、两个请求、恰好一次 socket accept。保留原配置测试。 |
| 公开 Upgrade 拒绝分支在已检查测试组中缺少对应案例 | 增加 missing/wrong/duplicate `Upgrade`、重复 selected subprotocol 的表驱动公开测试，断言具体类型化终止拒绝及精确发送数。已有 unsolicited Version、Accept、307、trust、mTLS/CONNECT、PMD 案例保留，不重复补。 |
| Worker 取消时点没有被确定区分 | 已有测试 poll 后 drop，或预设取消。私有层用受控 signing key 和进入/释放 barrier 加强：证明进入同步调用后取消、持钥直至完成、抑制发布；用调用计数证明进入前取消没有调用。不能声称 native 调用可被抢占。 |
| Adapter 覆盖需要明确消费者映射 | 将 transcript/replay/schema/auth/usage 场景映射到真实消费者和支持能力。复用已有 fixture、wire、lifecycle；只给缺失的契约/adapter 格补能区分错误行为的反例。不能仅凭测试名称认定完整矩阵。 |
| 当前 artifact/platform 执行证据尚未建立 | 现有打包脚本覆盖限定的 WS/HTTP/TLS 和 early Schema 切片。新运行需绑定实际 package/consumer lock、feature，区分 native host 与 WSL，记录 target/toolchain。历史 Schema 证据不代表当前树或全平台通过；缺少运行是证据缺口，不自动等于代码缺陷。 |

持久凭据已有具体 [KeychainCredentialStore](../../crates/opi-coding-agent/src/credential_store.rs)
和 [credential_store 测试](../../crates/opi-coding-agent/tests/credential_store.rs)，
包括 lock、rotation cancellation、logout ordering 和 barrier，不能列为整体缺测。
跨进程/具体 store 验收属于该 adapter 和组装调用方；`InMemoryCredentialStore`
通过不能替代它。完整平台执行仍需核实。

## 验收命令

从仓库根目录执行。以下是计划命令，不是运行结果。按变更面选择检查并集，通过后
不要因为目录里列出了其他命令而重复跑未失效的测试。范围涉及整个 crate 时，最终
crate 门禁可以替代默认依赖图的各个聚焦运行。

### 环境与既有共享缓存

使用本地 fixture、scripted loopback peer、隔离临时数据和 `MockProvider`；本方案
不包含付费 provider、环境凭据触发或 live endpoint。Cargo offline 只控制依赖解析，
不控制测试本身联网。保留环境变量修改的串行化约束。

```powershell
python scripts/opi-cargo-cache.py status
$sharedTarget = (python scripts/opi-cargo-cache.py resolve).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cargo cache resolution failed' }
$env:CARGO_TARGET_DIR = $sharedTarget
$env:CARGO_INCREMENTAL = '1'
```

`status` 只读；`resolve` 可能初始化按 workspace/toolchain 稳定定位的 cache marker。
不创建 per-session target，不清理/删除共享缓存。离线运行要求依赖/registry archive
和原生工具链已可用。manifest 声明 edition 2024、MSRV 1.97；较新编译器运行成功
不能证明 MSRV 支持。

### 私有机制

选择下面的相关 filter。Cargo 使用子串匹配，这些限定串避免因名称包含相同单词
而选中其他模块。它们已有测试，仍需确认实际选中名称和数量。不能用并未建立的
TLS、mutation 私有 filter 替代其 integration target。

```powershell
cargo test --locked --offline -p opi-ai --lib ws::
cargo test --locked --offline -p opi-ai --lib crypto::
cargo test --locked --offline -p opi-ai --lib retry::
cargo test --locked --offline -p opi-ai --lib schema::
cargo test --locked --offline -p opi-ai --lib transcript::
cargo test --locked --offline -p opi-ai --lib replay::
cargo test --locked --offline -p opi-ai --lib bedrock::event_stream::
cargo test --locked --offline -p opi-ai --lib bedrock::sigv4::
cargo test --locked --offline -p opi-ai --lib stream::cancellation_tests::
cargo test --locked --offline -p opi-ai --lib text_decode::
cargo test --locked --offline -p opi-ai --lib endpoint::
```

### 公开模块与组合命令组

执行整个 integration target，包含分散在嵌套模块里的案例。
`tests/support/tls_fixtures.rs` 及其他 `tests/support` 模块是 helper，不是验收测试组。

```powershell
# Transport
cargo test --locked --offline -p opi-ai --test ws_upgrade --test tls_contract --test http_acceptance --test proxy_support --test connection_pooling --test retry_backoff
# Schema
cargo test --locked --offline -p opi-ai --test schema_engine --test schema_public
# Credentials and Models lifecycle
cargo test --locked --offline -p opi-ai --test auth_contracts --test per_request_auth --test oauth_wire_shape --test provider_collection --test custom_provider_registration
# Canonical transcript and replay
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test tool_result_wire --test tool_result_no_leak
# Accounting and stream outcomes
cargo test --locked --offline -p opi-ai --test context_budget --test usage_cost --test stream_events
# Catalog
cargo test --locked --offline -p opi-ai --test registry --test model_wire_metadata --test model_capabilities_migration --test api_mapped_provider
# Bedrock
cargo test --locked --offline -p opi-ai --test bedrock_fixtures
# Adapter dispatch and lifecycle
cargo test --locked --offline -p opi-ai --test provider_trait --test provider_error_classes --test provider_diagnostics --test provider_lifecycle --test openai_compat_lifecycle --test openai_responses_lifecycle --test gemini_lifecycle --test openai_codex_responses
# Provider fixtures
cargo test --locked --offline -p opi-ai --test anthropic_fixtures --test anthropic_thinking_roundtrip --test openai_chat_fixtures --test openai_responses_fixtures --test gemini_fixtures --test azure_openai_fixtures --test vertex_fixtures --test mistral_fixtures --test openrouter_fixtures
# Additional wire contracts and test infrastructure
cargo test --locked --offline -p opi-ai --test request_enrichment --test image_input --test image_provider_serialization --test output_content_image --test test_support_tests
```

### WS feature graph

ordinary、caller-default、C-zlib union 分开验收，不能用 `--all-features` 替代。
C-zlib union 需要对应原生构建工具。

```powershell
cargo test --locked --offline -p opi-ai --lib ws:: --features flate2/default
cargo test --locked --offline -p opi-ai --test ws_upgrade --features flate2/default
cargo test --locked --offline -p opi-ai --lib ws:: --features flate2/zlib
cargo test --locked --offline -p opi-ai --test ws_upgrade --features flate2/zlib
```

### SDK artifact 与外部消费者

每个 runner 使用全新的外部 scratch，既有带 marker 的 target 必须属于当前
workspace。独立消费者离线解析自身 lock；workspace lock 本身不能证明其依赖可用。

```powershell
$acceptanceRoot = Join-Path $env:TEMP ('opi-ai-acceptance-' + [guid]::NewGuid().ToString('N'))
python -B scripts/pi-sdk-conformance/package_slice.py --family ws --offline --scratch (Join-Path $acceptanceRoot 'ws') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/package_slice.py --family crypto --offline --scratch (Join-Path $acceptanceRoot 'crypto') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/package_slice.py --family http --offline --scratch (Join-Path $acceptanceRoot 'http') --target-dir $sharedTarget
python -B scripts/pi-sdk-conformance/run_early_package_proof.py --scratch (Join-Path $acceptanceRoot 'schema') --target-dir $sharedTarget
```

[Family runner](../../scripts/pi-sdk-conformance/package_slice.py) 只支持 `ws`、
`crypto`、`http`。WS 在三种依赖图中分别运行解包后私有测试、包内公开 Upgrade 测试
和独立外部公开消费者。Crypto/HTTP 有 ordinary、caller-union 公开消费者；crypto
family 验证 TLS，不是完整私有签名/worker 契约。
[Early runner](../../scripts/pi-sdk-conformance/run_early_package_proof.py)
内部固定离线，不接受 `--offline` 参数，在两种依赖图检查 early public Schema。
其 `ICU4X_DATA_DIR` 前置条件要求变量完全不存在。两个 runner 都不接受
`--output`/`--json`，日志和 `evidence.json` 固定写在 scratch 中。遵守
[harness 前置条件](../../scripts/pi-sdk-conformance/README.md)，包括原生构建来源
和 source/config override 拒绝规则。这些切片不证明每个模块的包外行为，也不完成
最终 SDK 发行验收。

修改 harness 本身时，还需运行其测试；它们不能替代真实 package/consumer 运行：

```powershell
python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py' -v
```

### Crate 与跨 crate 门禁

局部 runtime 修改使用聚焦测试和 affected-target clippy，例如下列首条命令；
不能给 clippy 传测试名 filter。整个 crate 范围使用后面的 crate 命令。

```powershell
cargo clippy --locked --offline -p opi-ai --lib --test ws_upgrade -- -D warnings
cargo test --locked --offline -p opi-ai --all-targets
cargo test --locked --offline -p opi-ai --doc
cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings
```

涉及具体 credential adapter 时，加入
`cargo test --locked --offline -p opi-coding-agent --test credential_store`
和对应 affected-target clippy，然后执行适用工作区门禁。跨 crate 检查选择遵循
既有[变更范围契约](../../.agents/skills/_shared/references/change-scope-and-check-selection.md)。
涉及跨 crate runtime、Phase exit、CI 或 release 时，保留下列工作区门禁及所属
产品场景，模块结果不能替代它们。

```powershell
python scripts/opi-doc-check.py
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
$savedRustdocFlags = $env:RUSTDOCFLAGS
try {
    $env:RUSTDOCFLAGS = '-D warnings'
    cargo doc --workspace --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'cargo doc failed' }
} finally {
    if ($null -eq $savedRustdocFlags) {
        Remove-Item Env:RUSTDOCFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTDOCFLAGS = $savedRustdocFlags
    }
}
```

## 执行顺序与交接

1. 绑定当前状态和相关契约到既有测试 -> verify: HEAD、staged/unstaged/untracked
   清单、具体 consumer/capability 映射、测试发现及检查并集；重新核对已有 WS 输入。
2. 执行选中的私有/公开测试组和依赖 profile -> verify: 实际非零执行、可观察的
   正反例结果和 affected-target clippy。只有实现准入后才为确证缺口补有判别力的案例，
   保持既有测试/fixture 归属。
3. 执行适用 artifact/platform 和跨 crate 验收 -> verify: 实际包身份、外部消费者
   结果、host/target/MSRV、feature/native provenance 和已注册产品场景。
4. 交接限定结果 -> verify: 精确命令/退出码/数量、材料身份、已接受差异、未关闭的
   生产/证据缺口，以及测试影响（`add`、`update`、`delete`、`retain`、`none`）。
   较窄层通过不能关闭较宽层失败。

本轮仅整理文档，测试影响为 `retain`：不提出立即修改 runtime、测试、manifest、
fixture、harness 或账本。新增文档对使用 `python scripts/opi-doc-check.py` 和
`git diff --check` 验证。
