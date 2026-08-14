# 边缘 Agent 与本地—云端分级协同计算：对 Opi 技术路线的研究判断

> 日期：2026-08-13  
> 性质：非规范性 outward research evidence；不修改 `docs/opi-spec.md` 的权威。  
> 范围：边缘 Linux/桌面/移动级设备上的 Agent runtime，以及本地小模型、近端边缘模型与云端大模型的分级协同。本文不把 MCU/裸机、模型训练或自研推理内核纳入 Opi 当前范围。  
> 事实边界：Opi 当前代码与规范、本地固定的 pi 0.84.1 快照，以及截至本文日期可访问的一手论文、标准和官方产品文档。

## 结论先行

用户的设想成立，而且它与 Opi 使用 Rust、保持小而深的 Agent Core、把 provider routing、authority、evidence 和 command execution 做成明确边界的技术路线具有较高一致性。但要把设想说准确：

1. **pi 正在从 coding-agent monorepo 向 Agent infrastructure 过渡，但当前更准确的名称是“durable/remote Agent platform foundation”，还不是成熟的 edge Agent infrastructure。** pi 0.84.1 已有真实的 session/storage、protocol/client/server、telemetry 基础；决定产品闭环的 `AgentHarness` durable operation 仍多处返回 `HarnessNotImplemented`，server 也没有完成 coding-agent service assembly。来源见 [`harness-v2.md`](../../.repo/pi-0.84.1/packages/agent/docs/harness-v2.md)、[`agent-harness.ts`](../../.repo/pi-0.84.1/packages/agent/src/harness/agent-harness.ts)、[`server/README.md`](../../.repo/pi-0.84.1/packages/server/README.md) 和[逐包盘点](2026-08-13-pi-0.84.1-package-architecture-realignment.md)。
2. **Opi 最有价值的角色不是在 Rust 中重新实现模型推理引擎，而是成为靠近数据、工具和设备的可信 Agent runtime。** 本地、近端和云端模型均作为可替换 provider；Opi 在边缘侧持有会话、权限、工具执行、路由约束和证据。这样既利用 Rust 的部署和系统正确性优势，又避免把 CUDA、NPU、量化格式和模型生命周期塞进 Agent Core。
3. **CPU cache 类比在“减少数据移动、把控制环放近数据”这一层成立，但在一致性和时延模型上不成立。** CPU cache 有硬件维护的一致性、固定层级和纳秒级互联；edge/cloud 没有共享地址空间，网络会断开、重排和抖动，两侧模型可能具有不同 tokenizer、能力和输出语义，而且跨越新的隐私与权限边界。正确借鉴不是先做 transformer layer split，而是先把 **Agent、工具和数据处理放在边缘**，再用显式、可测的 provider routing 决定哪些推理上送。
4. **Rust 不保证“没有内存泄漏”。** Rust 在 safe code 中大幅消除 use-after-free、double free 和数据竞争等内存安全问题，也没有 GC 停顿；但官方 Rust Book 明确说明 `Rc` 引用环可以泄漏，防止所有泄漏不是 Rust 的保证，[`mem::forget`](https://doc.rust-lang.org/core/mem/fn.forget.html) 甚至是 safe API。长期运行的 edge Agent 仍需 RSS/句柄/任务泄漏、allocator fragmentation、OOM、磁盘耗尽和热降频的 soak/fault tests。[Rust Book：Reference Cycles Can Leak Memory](https://doc.rust-lang.org/stable/book/ch15-06-reference-cycles.html)
5. **Opi 当前不需要新增 edge/server/scheduler/model-distribution crates。** 现在应完成 Phase 17；随后以 Reference Product/Extension prototype 证明一个本地模型 + 一个云模型的请求级分级路由。在真实硬件、真实网络和 frozen Eval 上证明收益后，再为 durable operation、device identity、remote protocol、模型分发和 fleet control 分别做 Placement Review。

推荐的长期形态不是“云中心拥有一切”，而是：

```text
可选的中心 Fleet / Policy Control Plane
  - 人类策略、设备注册、签名部署、撤销、模型目录
                         │ signed immutable inputs
                         ▼
边缘 Opi Agent（可信执行与权限边界）
  - session / durable intent（未来）
  - local tools / local data
  - ToolAuthorizer / User Policy
  - evidence + store-and-forward
  - provider selection mechanism
       ├── device-local model endpoint
       ├── LAN / regional-edge model endpoint
       └── cloud model endpoint
```

即：**控制面可以集中，执行面和数据面下沉，权限面在本地闭合，证据面可最终汇聚。**

## 1. 这个设想真正包含的三个不同问题

“边缘 Agent + 本地/云端模型协同”容易把三个生命周期不同的问题揉在一起：

| 问题 | 核心对象 | Opi 应承担什么 |
|---|---|---|
| Agent placement | 会话、工具、权限、状态和用户交互放在哪里 | Opi 可以直接成为边缘侧 Agent runtime |
| Inference placement | 一次请求在哪个模型/设备执行，是否分割生成 | Opi 只拥有 provider-neutral dispatch 和可验证选择；推理引擎保持外部 |
| Fleet infrastructure | 设备身份、部署、升级、模型分发、监控和撤销 | 未来独立 control-plane/companion；不能提前进入 Agent Core |

这一分离很重要。把 Agent 放在边缘，不要求把所有模型权重放在边缘；同样，在设备上运行一个小模型，也不等于已经具备可管理、可升级、可恢复的 edge Agent infrastructure。

## 2. 外部一手证据：哪些协同方式已经成立

### 2.1 五种计算放置模式

| 模式 | 工作方式 | 外部证据 | 对 Opi 的判断 |
|---|---|---|---|
| 完全本地 | 模型和 Agent 都在设备运行 | Google 的 [LiteRT-LM](https://developers.google.com/edge/litert-lm/overview) 已支持 Android/iOS/Linux/IoT 等端侧执行；官方公开的特定 Raspberry Pi 5 测试也显示 2.58 GB 模型仍只有约 8 tok/s decode、约 7.8 s TTFT，说明“能跑”不等于满足交互 SLO | 可作为 offline/privacy baseline；推理 runtime 应是外部 provider |
| 请求级 cascade | 小/本地模型先处理，按任务、质量、成本、网络或隐私约束升级到强模型 | Apple PCC 的生产架构明确“能本地就本地，更复杂任务进入云端”；[RouteLLM](https://proceedings.iclr.cc/paper_files/paper/2025/hash/5503a7c69d48a2f86fc00b3dc09de686-Abstract-Conference.html) 证明弱/强模型 routing 可以改善成本—质量权衡 | **Opi 第一优先实验**；边界清楚、失败可回退、无需共享 KV/cache |
| token 级协作 | edge SLM 生成大部分 token，只让 cloud LLM 处理关键 token 或验证草稿 | ACL 2025 的 [Token Level Routing](https://aclanthology.org/2025.acl-demo.16/) 在一个窄场景报告小模型质量提升且少量 token 上云 | 有前景但仍是研究/特定 workload；要求 tokenizer、状态、取消和网络管线高度兼容，不应先进入 Opi |
| layer / shard split | 将一个模型的层或分片分布到设备和云端，传输中间激活 | [Neurosurgeon](https://doi.org/10.1145/3037697.3037698) 对早期 DNN 证明了动态 partition 的价值；[EdgeShard](https://arxiv.org/abs/2405.14371) 把这一思路扩展到 LLM 的多设备分片 | 最接近 CPU/GPU 分级计算类比，但耦合模型版本、网络和硬件最深；只有 profiling 证明优于 cloud-only/local-only 才值得做 |
| Agent/task 级拆分 | 边缘处理感知、检索、过滤和工具，云端负责复杂规划/生成 | Apple PCC 将本地与云端执行环境分开记录；AWS Greengrass 的官方定位也是靠近数据做 local compute、offline operation，再与云管理/存储协作 | 最符合 Opi：把数据和 side effects 留在 edge，只发送最小必要上下文 |

以上论文数字都只证明其各自模型、硬件、任务和网络条件下的结果，不是 Opi 的性能承诺。特别是 learned router 的“置信度”不能成为 permission 或 data-egress authority；它最多在 User Policy 已允许的候选集合内做选择。

### 2.2 Apple PCC 验证了分级模型，也揭示了安全成本

[Apple Private Cloud Compute Security Guide](https://security.apple.com/documentation/private-cloud-compute/) 是目前最完整的生产级本地—云端模型分级案例之一：可本地处理的请求留在设备，更复杂任务使用云端大模型。它同时要求：

- 请求只加密给经过验证和 attestation 的计算节点；
- 节点运行的代码和模型资产受完整性保护；
- 云端处理 personal data 后不保留数据；
- 用户设备只向公开 transparency log 中可验证的软件版本释放请求密钥；
- 本地/云端执行环境可以在 [Apple Intelligence Report](https://security.apple.com/documentation/private-cloud-compute/appendix_appleintelligencereport) 中区分。

这说明本地—云端协同的真正难点不是一个 `if local { ... } else { ... }`。一旦数据出设备，系统就必须拥有节点身份、软件/模型度量、请求数据分类、可验证 route、留存策略和最小化 telemetry。Opi 不需要复制 PCC，但应吸收这个边界思维。

### 2.3 Edge fleet 的基础工作远超“编译一个 ARM binary”

AWS Greengrass 的一手文档提供了成熟 edge runtime 的对照：设备使用 X.509 identity 和 policy；components 可以按单设备或 group 连续部署；断网时可以使用有期限的本地 credential cache；edge 应用能在本地响应并与云端做管理/持久化。[Device authentication](https://docs.aws.amazon.com/greengrass/v2/developerguide/device-auth.html)、[offline authentication](https://docs.aws.amazon.com/greengrass/v2/developerguide/offline-authentication.html)、[deployments](https://docs.aws.amazon.com/greengrass/v2/developerguide/create-deployments.html)。

这组能力应被理解为独立的 fleet/control-plane 产品，不是 `opi-agent` 应该吸收的 API 清单。

## 3. CPU/GPU/cache 类比：哪里对，哪里需要修正

### 3.1 类比成立的部分

- 数据移动可能比计算更贵；把感知、检索、tool I/O、权限判断和预处理放到数据附近，能够减少带宽、隐私暴露和交互往返。
- 计算应该按照 latency、capacity、energy 和 locality 分级；不是所有请求都要进入最强模型。
- 每一层都需要 admission、eviction/rollback、measurement 和 miss/escalation policy。
- 边缘可以成为 local working set：短期 session、工具输出、embedding/index、局部事实和小模型常驻；大模型保留在近端或云端。

### 3.2 类比失效的部分

| CPU cache | Edge/cloud Agent |
|---|---|
| 硬件提供一致性协议 | 没有共享内存；状态必须显式 version、digest、reduce 和 reconcile |
| 命中/缺失是字节地址问题 | 路由是语义质量、能力、隐私、预算和网络的联合决策 |
| 层级和互联可控 | RTT、带宽、掉线、拥塞、服务排队动态变化 |
| 同一 ISA/内存语义 | 不同模型可能有不同 tokenizer、tool schema、system prompt 和输出分布 |
| 单一硬件信任域 | 设备、LAN edge、云是不同 identity、attestation 和 data-retention 边界 |

因此更准确的类比是 **分层服务 + 显式一致性 + policy-constrained routing**，不是透明 cache。Opi 需要把每次 route 当成可验证的运行时状态转换，而不是把 local/cloud 当作隐藏 fallback。

端到端时延也应该按完整路径计算：

```text
T_total = T_admission + T_local_preprocess + T_queue
        + T_upload + RTT + T_remote_inference + T_download
        + T_reconcile + T_tool_effect
```

只有当 `T_total` 的 P50/P95/P99、质量、能耗和成本同时优于基线时，“计算靠近数据”才是实证结论。token/layer split 还会引入更频繁的同步、rollback 和状态一致性成本。

## 4. Rust 的真实优势与边界

### 4.1 Rust 给 Opi 的真实优势

- `Agent`、session、cancellation、permission 和 protocol 可以用所有权、closed enums、typed errors 和 bounded queues 表达，适合长期运行的控制 runtime。
- 没有 tracing GC，通常更容易获得稳定 latency 和受控 memory footprint。
- Rust 官方将 `aarch64-unknown-linux-gnu` 列为 Tier 1 host target；Opi 当前 CI 已对六个 release triples 做 compile gate，并发布 Linux ARM64 artifact。[`ci.yml`](../../.github/workflows/ci.yml)、[`release.yml`](../../.github/workflows/release.yml)、[Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- 一个 CLI 可以打包为单个 executable，减少 Python/Node runtime 的现场依赖和版本漂移。
- 对跨语言 inference engine、sandbox 或 device adapter，可以通过 bounded process protocol 保持故障和依赖隔离；Opi 已在 `command-execution-jsonl-v1` 上实践这一模式。

### 4.2 必须纠正的四个预期

1. **Rust 不是 leak-free。** `Rc/Arc` cycle、`mem::forget`、永不结束的 task、无界 cache/channel、句柄遗失和 FFI 都可能导致内存或资源不释放。
2. **单 executable 不等于 universal/static binary。** Opi 当前 Linux artifact 使用 `*-unknown-linux-gnu`，需要相应 kernel/glibc；musl 可以支持更独立的静态部署，但当前项目没有声明 musl target，而且 TLS、keyring、DNS、证书和原生依赖仍需逐项验证。[Rust Arm Linux support](https://doc.rust-lang.org/stable/rustc/platform-support/arm-linux.html)
3. **支持 ARM64 编译不等于支持 edge device。** 还缺真实设备启动、keyring/证书/文件系统行为、低内存、断电、热降频、网络抖动和长时间 soak evidence。
4. **当前 Opi 是 `std` + Tokio + Reqwest + OS/TUI 的 hosted runtime，不是 MCU firmware。** Rust Embedded Book 明确区分有 POSIX/OS 的 hosted environment 与 `no_std` bare metal；当前 Opi 不应宣称可直接进入 MCU/RTOS。[Embedded Rust `no_std`](https://docs.rust-embedded.org/book/intro/no-std.html)

建议把近期 edge platform scope 明确为：**Linux ARM64/x86_64 hosted edge node，具有文件系统、网络、进程和至少数百 MB 可用内存；模型可以运行在外部 accelerator/runtime。** 这包括 Raspberry Pi/Jetson/工业 PC/家庭服务器一类设备，但不自动覆盖手机 App sandbox、浏览器、MCU 或实时控制器。

## 5. pi 是否正在转向 Agent infra

答案是“是，但要加两个限定词”：**试探性、非 edge-specific。**

pi 0.84.1 的动作已经覆盖了 Agent infra 的若干平面：

| Infra 平面 | pi 0.84.1 信号 | 当前缺口 |
|---|---|---|
| State plane | v4 session tree、lanes、operation records、facts、Memory/JSONL/SQLite conformance | broad AgentHarness operation runtime 仍是 scaffold |
| Control plane | `pi-protocol`、`pi-client`、`pi-server`，多 session、lease、snapshot | server experimental；无完整 coding-agent service/CLI |
| Observation plane | 独立 `pi-telemetry`、explicit context、noop/in-memory adapter | domain spans 依赖未完成 harness 主路径 |
| Evaluation plane | private `pi-evals` 驱动真实 `AgentSession` | 仍是 pi-specific regression，不是 cross-Agent infra |

它尚未覆盖 edge infra 的关键面：设备/工作负载身份、硬件 attestation、fleet inventory、signed OTA/model distribution、hardware capability discovery、offline authority cache、network-aware scheduling、energy/thermal budget 和 edge data-egress policy。

因此 pi 是有价值的 durable/remote platform 观察源，却不能被当作 Opi 的 edge roadmap。尤其不能因为 pi 新增 package 就在 Opi 中复制 server/client/sqlite/telemetry crates。

## 6. Opi 当前路线与这个设想的匹配度

### 6.1 已经走对的底层

| Opi 能力 | 对 edge hierarchy 的价值 | 当前证据 |
|---|---|---|
| `ProviderCollection` + canonical `provider:model` | 将 local/LAN/cloud inference 都表示为真实可派发 route，而不是 metadata | [`provider_collection.rs`](../../crates/opi-ai/src/provider_collection.rs)；Phase 17 `P17-PRV-*` |
| 完整 `NextTurnState` 原子替换 | 一次 turn 后可以显式改变模型/推理配置而不产生混合状态 | [`loop_types.rs`](../../crates/opi-agent/src/loop_types.rs)；Phase 17 `P17-NXT-*` |
| trusted `ToolAuthorizer` | 云端/本地模型都只能提出 tool call，不能授予 side-effect 权限 | [`authority.rs`](../../crates/opi-agent/src/authority.rs)；`INV-005` |
| `EvidenceSink`、route/auth/policy/runtime binding | 可以证明某次请求实际在哪一层执行、以何种权限和配置执行 | [`evidence.rs`](../../crates/opi-agent/src/evidence.rs)；`CTRL-001`—`CTRL-003` |
| session branch/reconstruction/crash tail recovery | 为 offline/resume 提供会话基础 | [`session.rs`](../../crates/opi-agent/src/session.rs)、[`harness.rs`](../../crates/opi-agent/src/harness.rs) |
| 独立 command-execution protocol/sandbox | edge Agent 可以通过 process boundary 调本地/容器/远程 execution backend | [`opi-protocol`](../../crates/opi-protocol/README.md)、[`opi-sandbox`](../../crates/opi-sandbox/Cargo.toml) |
| Linux ARM64 build/release | 已有 hosted edge binary 的交付起点 | [`ci.yml`](../../.github/workflows/ci.yml)、[`release.yml`](../../.github/workflows/release.yml) |

这也是为什么 Phase 17 应继续完成，而不是转向 broad AgentHarness：local/cloud 分级首先需要“选择的模型真的对应实际 provider”“state 切换原子”“远端输出无权授予工具权限”“route/effect 有证据”。

### 6.2 当前还不能做出的声明

- 当前生产 `CodingHarness` 仍有同-provider model switch 限制，Phase 17 的 cross-provider production cutover 仍在进行；不能宣称已经可在一次 session 中自动 local/cloud 跳转。[`harness.rs`](../../crates/opi-coding-agent/src/harness.rs)
- `evidence.rs` 当前自称 additive substrate，尚不能把 contract 存在等同于 provider/tool/retry/compaction/finalization 全路径 coverage。
- JSONL RPC 输入和内部 channel 仍有无界路径；`opi-protocol` command execution 已有 capped reader，但普通 RPC 使用 `BufRead::lines()` 和 unbounded channels。它适合作为当前本地自动化入口，不应直接暴露为 edge network service。[`rpc.rs`](../../crates/opi-coding-agent/src/rpc.rs)、[`streaming_proxy.rs`](../../crates/opi-agent/src/streaming_proxy.rs)、[`codec.rs`](../../crates/opi-protocol/src/execution/v1/codec.rs)
- 当前 session 能恢复 conversation branch，不等于 durable accepted operation：断电发生在“工具 intent 已接受但 effect/result 未落盘”时，系统还没有完整恢复语义。
- Opi 没有 device identity、attestation、fleet deployment、model artifact distribution 或 resource-aware scheduler；这些不是 bug，而是尚未被真实消费者 admission 的产品能力。

## 7. 若沿这条路线继续，必须考虑的十个系统面

### 7.1 明确 edge device class 和 SLO

先定义支持层级，而不是使用模糊的“edge”：

```text
E0: MCU / no_std / RTOS                 — 当前明确不支持
E1: hosted Linux edge (ARM64/x86_64)    — 最合适的第一目标
E2: desktop/mobile OS sandbox           — 需要各平台 adapter 与分发形态
E3: regional edge server                — 可作为近端 provider 或 execution node
Cloud: hyperscale/provider APIs         — 现有 provider 路径
```

每一层需要明确最低 RAM/disk、架构/ABI、网络、可信硬件、accelerator、文件系统和升级能力。没有 target profile，就无法解释“一份 Rust binary 可运行”的范围。

### 7.2 设备身份、工作负载身份和用户权限必须分离

- Device identity：这是哪一台 edge node？
- Workload identity：这台机器上的哪个已度量 Opi/runtime 正在请求？
- User/tenant authority：谁允许它处理哪些数据、调用哪些工具？
- Model/service identity：响应来自哪个 model artifact/provider endpoint？

[SPIFFE/SPIRE](https://spiffe.io/docs/latest/spire-about/spire-concepts/) 展示了 node attestation 与 workload attestation 分层的成熟模式；AWS Greengrass 展示了 X.509 device identity 与 data-plane policy。它们是设计参考，不意味着 Opi 应依赖 SPIRE/AWS。Opi 的关键约束是：**attestation 只证明身份/度量，不能自动授予工具或数据权限；授权仍由 User Policy/Capability Permission 决定。**

### 7.3 Routing 必须是 policy-constrained scheduling

一次 local/edge/cloud 选择至少受到以下输入约束：

- 任务所需能力：context、image、tool use、reasoning、structured output；
- 数据分类：local-only、edge-allowed、cloud-allowed、redaction-required；
- 当前资源：RAM、accelerator、queue、thermal、battery、disk；
- 网络：reachable、RTT、bandwidth、loss、metered；
- 预算：deadline、tokens、money、energy、cloud quota；
- 质量证据：固定任务族上的 router calibration，而不是模型自报 confidence；
- User Policy：哪些 fallback/escalation 被允许。

Agent Core 只应拥有“resolved selection 必须真实 dispatch、失败可区分、route 有证据”的机制。默认 route、privacy threshold、成本策略和 learned router 都属于 Reference Product/Extension policy，不能进入 `opi-agent`。

### 7.4 数据最小化比单纯减少 RTT 更重要

edge Agent 可以在本地完成文件检索、传感器聚合、PII/secret redaction、tool schema projection 和 context compaction，只把完成任务所需的最小 prompt/artifact reference 交给云模型。云模型返回的文本始终是 untrusted content；实际 tool effect 在本地再次经过 schema + authority。

这比将完整 session 同步到云端更接近用户的“把内存放到计算附近”设想：不是复制全部 working set，而是让数据的主要拥有者和 side-effect boundary 留在边缘。

### 7.5 Offline、resume 和外部 side effect 需要 operation 语义

真正 unattended edge Agent 必须回答：

- prompt 在返回 accepted 前是否已经 durable？
- 断电后哪些操作可自动重放，哪些只能标记 `outcome_unknown`？
- tool effect 前是否先写 intent，effect 后是否写 result？
- retry 是否带稳定 idempotency key？目标系统不支持幂等时怎么办？
- 谁拥有 single-writer lease，过期 writer 如何 fencing？
- 网络恢复后如何 upload evidence，如何处理 quota 和重复数据？

这正是 pi durable harness 最值得观察的部分，但 Opi 应在真实 edge prototype 证明需求后再进入单独 Phase。不能宣称“exactly once side effects”；现实目标应是 durable intent、可识别的 at-least-once/at-most-once policy、幂等适配器，以及无法证明时显式 `partial/cleanup_unknown`。

### 7.6 Protocol 必须在读取前有界，并有 backpressure

未来 remote session/device protocol 至少需要：

- transport authentication before application bytes；
- protocol/version/capability negotiation；
- frame、depth、container、message 和 artifact size limits；
- per-stream/per-session flow control 与 bounded queues；
- cancellation、deadline、terminal outcome 和 final byte accounting；
- stable request/operation ID、dedup/idempotency 和 reconnect snapshot；
- authoritative snapshot 与 transient progress 分离；
- explicit corruption、overflow、stale lease、partial effect errors。

[QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html) 将 flow control、stream cancellation 和 final size 作为 transport 状态；[MQTT 5.0](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html) 将 receive quota、session/message expiry 和 maximum packet size 显式协议化。这些标准说明 backpressure/resume 不是“换成 CBOR/WebSocket”就自动获得。Opi 未来可选任意 transport，但必须保留上述语义。

### 7.7 Binary、配置、模型和 adapter 要作为不同更新对象

一个 edge deployment 至少有四种独立 artifact：

1. Opi binary；
2. User Policy/runtime configuration；
3. local model/tokenizer/quantization/runtime bundle；
4. extension/execution adapter。

它们需要各自 digest、target compatibility、signature、dependency、rollback 和 activation record。模型权重不是普通配置；tokenizer、chat template、tool schema compatibility 和 accelerator engine cache 都应绑定版本。

[TUF specification](https://github.com/theupdateframework/specification/blob/master/tuf-spec.md) 提供 threshold roles、version/expiry、consistent snapshot 和 compromise-resilient update 的标准参考；[IETF RFC 9019 SUIT architecture](https://www.rfc-editor.org/rfc/rfc9019.html) 明确要求设备基于 trust anchor、signature/MAC、vendor/class/device ID 和 sequence 判断固件是否适用且不是 rollback。Opi 不应自创一套未经证明的 OTA crypto protocol。

若未来 central fleet controller 选择 runtime/model bundle，还需要审查 Opi 的 `RuntimeInputBinding` 术语：当前 `ActiveSnapshot` 专属于 Promotion Controller，普通 fleet deployment 不能冒充这一 authority。届时应做 domain/spec revision，增加正确的 deployment binding，或继续记录为 Direct Runtime Input；不能偷偷复用名称。

### 7.8 Evidence 必须支持断网、配额和分层 route

edge evidence 至少要保留：

- device/workload identity 与 attestation reference；
- binary、policy、model/tokenizer/runtime/adapter digests；
- requested/resolved/actual tier、provider/model/wire；
- routing policy/version、触发原因和候选集合；
- network/resource measurement 及 measurement origin；
- local preprocessing/redaction 与实际上送 artifact 的 digest；
- tool authority、side-effect intent/result/unknown；
- offline buffer、drop/overflow、upload/finalization 状态。

但 raw prompt、sensor data、tool result 和 environment 不应因 fleet telemetry 自动上传。应使用 bounded local spool、redaction-before-sink、retention quota 和 store-and-forward；磁盘满时不能伪装为 complete evidence。Phase 17 的 EvidenceHealth/fail-closed 方向适合成为这一基础。

### 7.9 Local model 不是免费资源

local inference 仍消耗 DRAM bandwidth、battery、热预算、disk 和交互时间。长期服务需要：

- 模型加载/卸载和 cold-start policy；
- memory high-water 与并发 admission；
- thermal/battery aware shedding；
- accelerator exclusivity 和其他 workload fairness；
- quantization/quality drift evidence；
- OOM/panic 后 supervisor restart 和 session recovery。

因此选择 local 不能只因为 cloud 有 token cost；必须同时计算 quality、wall time、energy、device wear 和用户体验。

### 7.10 集中控制面不能获得隐式执行权限

Fleet controller 可以下发经过签名和用户授权的版本、撤销和预算，却不能因为“它是 central controller”就发送任意 shell/tool call。模型 route、软件 deployment、tool permission 和 package trust 是不同 authority。edge Agent 应先验证 immutable artifact 与 effective policy，再允许执行；控制面离线时只能在缓存且未过期的授权范围内继续。

## 8. 对 Opi 的分阶段建议

### 8.1 Now：不改变 Phase 17 范围

| 行动 | Placement / admission 判断 | 验证 |
|---|---|---|
| 完成真实 multi-provider dispatch、atomic next-turn、trusted tool authority 和 end-to-end evidence | 已由 `STRAT-001`、`INV-001`—`INV-006` admission；是 local/cloud hierarchy 的必要底层 | 两个 mock provider 跨 route；actual route evidence；远端内容无法授予 tool authority |
| 保持六 crate 拓扑，不新增 edge/server/scheduler/model crate | 新 seam 尚无两个真实消费者；避免违反 `PRIN-001/002` | Cargo graph 无新增 speculative seam |
| 把 edge hypothesis 记录为研究/实验路线，不改 mission | 当前还没有用户、SLO、真实硬件 Eval | 本报告作为 evidence；规范仍由 human shaping 决定 |
| 将“Rust 无内存泄漏”“通用 edge binary”从产品论证中移除 | 这些不是 Rust/Opi 已证明的保证 | 文档只声明实际 target/ABI 与测试证据 |
| 在任何 network exposure 前登记普通 RPC 的 bounded-read/unbounded-channel hardening | `INV-006` 已要求 bounded queue；但不应混入 Phase 17 未授权的 remote product | oversized line 在完全 buffering 前拒绝；bounded queues 有 overflow outcome |

### 8.2 Next：Phase 17 后做一个窄 prototype，不先做 infra

建议的最小实验：

```text
一台 Linux ARM64 edge device
  Opi Reference Product/headless mode
  ├── provider A: localhost/LAN OpenAI-compatible SLM endpoint
  ├── provider B: existing cloud provider
  ├── deterministic product-owned route policy
  └── local read/tool execution + complete evidence
```

可以先复用现有 custom OpenAI-compatible `base_url` 路径；当前 profile 需要 API-key env/auth scheme，若真实 local endpoint 不使用认证，只能在 product prototype 中做最窄适配，不应为了一个实验扩展 Agent Core。[`config.rs`](../../crates/opi-coding-agent/src/config.rs)、[`provider_factory.rs`](../../crates/opi-coding-agent/src/provider_factory.rs)

Prototype 只做请求级选择，建议固定三条规则：

1. `local-only` 数据永不上送；local 不可用则明确失败；
2. cloud-allowed 且 local 能力/quality/SLO 满足时使用 local；
3. 只有策略显式允许且 route reason 被记录时才 escalation 到 cloud；不做 silent fallback。

真实设备矩阵至少包含 Raspberry Pi/普通 ARM SBC 与带 accelerator 的 Jetson/mini-PC 中两类。比较 local-only、cloud-only、hierarchical 三组，记录：native task outcome、P50/P95 TTFT、P50/P95 wall time、tok/s、peak RSS、disk、energy/thermal、上/下行 bytes、cloud token/cost、escalation rate、route error、tool authority violations 和 evidence coverage。

故障矩阵至少包含：启动时离线、stream 中断、cloud auth 失效、local model OOM、磁盘满、edge process crash、tool effect 后 result 前断电、router 选择错误、远端 prompt injection、模型 update 中断。

只有这个实验出现可重复收益，才产生下一步真实 consumer evidence。

### 8.3 Later：由证据触发 Placement Review

| 能力 | 触发条件 | 初始 placement |
|---|---|---|
| Durable accepted operations / crash resume | unattended edge run 在 effect-boundary crash matrix 中证明 current session 不足 | intrinsic Agent semantics 的候选；需单独 Phase，不等于恢复 broad harness |
| Remote session protocol/client | 一个 edge service + 至少两个真实 clients/transports，且 auth/backpressure/conformance 完整 | Extension Ecosystem 或 Independent Companion |
| Device/workload identity | 两种实际部署环境/attestor，证明确有共同 contract | Agent-neutral Independent Companion/integration；permission 仍在产品 |
| Fleet deployment/model distribution | 多设备 inventory、签名 update、staged rollout 和 rollback 成为真实产品 | 独立 control-plane product，不进入 Agent Core |
| Resource-aware/learned routing | deterministic baseline 已有 frozen Eval，learned route 在质量/安全/成本上稳定优于基线 | Reference Product/Extension policy；Agent Core 只保留 dispatch mechanism |
| SQLite/search/materialized state | JSONL 在 bounded recovery/query/writer fencing 上出现可测瓶颈，且第二 backend 成为真实消费者 | Session adapter；共享 conformance 后再扩大 seam |
| Regional edge scheduler/leases | 多 edge nodes、容量、租约与迁移成为真实需求 | Independent serving companion；不把 Kubernetes/fleet policy放入 core |
| Token/layer collaborative inference | 请求级 cascade 已不足，兼容模型对、网络和硬件 profiling 证明稳定收益 | 外部 inference runtime/provider，不进入 `opi-agent` |

### 8.4 Do not do

1. 不在 `opi-agent` 内集成 llama.cpp/ONNX/TensorRT/CUDA/NPU runtime 作为默认模型执行层。
2. 不因为 Rust 可交叉编译就宣称支持 MCU、手机 App sandbox 或任意 Linux 发行版。
3. 不把 learned router 或 cloud model 输出当作 privacy、permission、fallback 或 tool authority。
4. 不做 silent local→cloud 或 cloud→local fallback；每次实际 route 和数据上送必须可见。
5. 不先做 token/layer split。先证明请求级 cascade 和 Agent/tool locality 的收益。
6. 不把 `opi-protocol` 的 command-execution wire 扩展成 remote session/device-management 协议。
7. 不复制 pi 的 npm package 拓扑，不恢复当前没有生产消费者的 broad `AgentHarness`。
8. 不自己发明 OTA/model-update 密码学；使用经审查的 TUF/SUIT 类设计或成熟平台能力。
9. 不承诺 exactly-once 外部 side effects；不能证明时保留 `partial`/`unknown`。
10. 不为证明“edge vision”跳过 Opi 规范的 Agent Core admission gate 和 cross-Agent Eval 顺序。

## 9. 对现有 Opi 规范路线的最终判断

当前不需要修改 [`docs/opi-spec.md`](../opi-spec.md) 才能探索这个设想：

- `GOAL-003` 已要求 Rust 用于 correctness、state、testability、portability，而不是复制 npm；
- `INV-001/002` 的真实 provider routing 是 local/cloud 分层的必要机制；
- `INV-005` 保证边缘/云模型都不能扩大工具权限；
- `CTRL-001/002` 要求 resolved route、runtime、policy 和 artifact provenance；
- `INV-006` 要求 bounded queues 和可见 backpressure/failure；
- `STRAT-003` 允许在 frozen evidence 上深化 model/tool decisions；
- Parallel routes 已把 remote、多-Agent 和 proactive behavior 留在产品/生态实验，避免过早污染 core。

需要保持的路线纪律是：

```text
Phase 17 semantic closure
  → independent/frozen Eval foundation
  → narrow edge local/cloud prototype
  → real hardware + failure evidence
  → Placement Review for the one proven missing seam
```

如果未来人类决定把“edge Agent runtime”从一个 Reference Product/Extension route 提升为 Opi 的首要 mission，那将是一次规范层的战略修订：需要新增明确 platform scope、支持设备级别、authority/fleet ownership 和 measurable success criteria。现在证据还不足以做这次转向，但足以把它确认为一条与 Opi 既有底层高度兼容、值得用窄实验验证的长期路线。

## 10. 决策摘要

| 问题 | 判断 |
|---|---|
| pi 是否在向 Agent infra 过渡？ | 是；重点是 durable state + remote control + telemetry，但纵向产品尚未闭合，也不是 edge fleet infra |
| Rust 是否适合 edge Agent？ | 适合 hosted edge control runtime；有内存安全、无 GC、typed concurrency 和交付优势，但绝非 leak-free/universal binary |
| Opi 当前路线要转弯吗？ | 不需要；Phase 17 正在建设 edge hierarchy 必需的正确底层 |
| 最先验证什么？ | edge Agent + local SLM endpoint + cloud provider 的请求级、policy-constrained routing |
| 最重要的架构边界？ | Agent/authority/evidence 在 edge；inference engine 外置；control plane 不能隐式获得 tool authority |
| 最大风险？ | 把 package/协议/模型分发提前塞进 core，或在没有真实设备 Eval 时把“减少传输”当作自动性能收益 |
| 何时建设 durable/remote/fleet infra？ | 当 unattended edge product、第二 adapter/consumer 和 crash/network/fleet conformance 共同证明 seam |

## 一手来源索引

- Opi normative direction: [`docs/opi-spec.md`](../opi-spec.md)、[`docs/CONTEXT.md`](../CONTEXT.md)
- Opi Phase 17: [`2026-08-12-phase17-deep-agent-core-semantic-closure-design.md`](../superpowers/specs/2026-08-12-phase17-deep-agent-core-semantic-closure-design.md)
- pi pinned source: [`.repo/pi-0.84.1`](../../.repo/pi-0.84.1)
- Apple: [Private Cloud Compute Security Guide](https://security.apple.com/documentation/private-cloud-compute/)、[PCC architecture introduction](https://security.apple.com/blog/private-cloud-compute/)
- Google: [Google AI Edge](https://developers.google.com/edge)、[LiteRT-LM Overview](https://developers.google.com/edge/litert-lm/overview)
- Edge/cloud inference research: [RouteLLM, ICLR 2025](https://proceedings.iclr.cc/paper_files/paper/2025/hash/5503a7c69d48a2f86fc00b3dc09de686-Abstract-Conference.html)、[Token Level Routing, ACL 2025](https://aclanthology.org/2025.acl-demo.16/)、[Neurosurgeon, ASPLOS 2017](https://doi.org/10.1145/3037697.3037698)、[Hybrid SLM/LLM, EdgeFM 2024](https://doi.org/10.1145/3662006.3662067)、[EdgeShard](https://arxiv.org/abs/2405.14371)
- Rust: [reference-cycle leaks](https://doc.rust-lang.org/stable/book/ch15-06-reference-cycles.html)、[`mem::forget`](https://doc.rust-lang.org/core/mem/fn.forget.html)、[platform support](https://doc.rust-lang.org/rustc/platform-support.html)、[`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html)
- Edge operations and identity: [AWS Greengrass device auth](https://docs.aws.amazon.com/greengrass/v2/developerguide/device-auth.html)、[offline auth](https://docs.aws.amazon.com/greengrass/v2/developerguide/offline-authentication.html)、[SPIFFE/SPIRE concepts](https://spiffe.io/docs/latest/spire-about/spire-concepts/)
- Secure distribution: [TUF specification](https://github.com/theupdateframework/specification/blob/master/tuf-spec.md)、[IETF RFC 9019](https://www.rfc-editor.org/rfc/rfc9019.html)
- Protocol behavior: [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html)、[MQTT 5.0](https://docs.oasis-open.org/mqtt/mqtt/v5.0/mqtt-v5.0.html)
