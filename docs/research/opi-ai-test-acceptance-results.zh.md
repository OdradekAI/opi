# opi-ai 测试与验收执行结果

[English](opi-ai-test-acceptance-results.md)

后续证据闭合和目录调整见[继续执行结果](opi-ai-test-closure-results.zh.md)。
下文区分顶层用例与 2 次隔离 TLS 子进程执行。旧 WSL 解析器漏掉了一条输出交错的
子进程结果：原聚合数 1331 更正为 1332 次执行（1330 个顶层用例加 2 次子进程执行）。

## 范围与来源绑定

2026-10-21 执行[统一测试与验收方案](opi-ai-test-acceptance-plan.zh.md)。
本文件记录验收证据和限定范围的测试维护，不代表 Phase 完成、实施准入或最终 SDK 分发验收。
本任务未修改生产行为、公开接口、依赖、保留的 fixture 数据文件或验收脚本实现；crypto 修改均位于 `cfg(test)` 内。

采用 worktree-only 模式，基线 HEAD 为
`7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`；本任务 committed=none、staged=none。
账本、README、WS 测试迁移和脚本修改均为带入状态。
`tests/ws_upgrade.rs` 原已是未跟踪文件，本任务在其上补测；其他带入实现文件未编辑。
实施账本仍由 `opi-implement` 独占维护。

下表九个已跟踪测试文件的 unstaged 修改由本任务产生。
`ws_upgrade.rs` 和方案双语文件属于 carried-in-and-touched 的未跟踪输入，方案只新增结果链接；
结果双语文件属于本任务新建的 untracked 输出。
`baseline-status.txt`、`final-status.txt` 分别保留完整清单。

采用本地 matt 技能库的 `codebase-design`、`tdd` 原则，在用户批准方案已经明确的接口上验收：
公开行为通过真实调用观察，私有机制保持原归属，用有区分力的反例补测，不新增公开测试接口或推测性抽象。

## 测试影响

| 影响 | 改动及可观察契约 |
| --- | --- |
| add | [connection_pooling](../../crates/opi-ai/tests/connection_pooling.rs)：同一公开 client 完整消费两次 H1 响应，断言两个请求、一个实际接受的 socket；同一 peer 配合 `max_idle_per_host(0)` 反控制必须产生两个 socket。保留配置和 Arc 测试。 |
| add | [ws_upgrade](../../crates/opi-ai/tests/ws_upgrade.rs)：一张表覆盖缺失、错误、重复 Upgrade，以及重复选择 subprotocol；断言精确类型错误、一次实际请求、EOF 和 peer 收尾。 |
| update、add | [crypto 私有测试](../../crates/opi-ai/src/crypto/mod.rs)：用包装真实已准入密钥的可控 rustls signer 强化两项取消测试，新增进入 signer 后取消且 receiver 存活、未轮询 future 直接丢弃两项测试。用进入/释放同步、调用计数、弱引用和 worker join 区分跳过、保留和抑制发布；控制器在展开时释放，并在断言/签名前释放 mutex guard。 |
| update | [Anthropic](../../crates/opi-ai/tests/anthropic_fixtures.rs)、[Chat](../../crates/opi-ai/tests/openai_chat_fixtures.rs)、[Responses](../../crates/opi-ai/tests/openai_responses_fixtures.rs)、[Gemini](../../crates/opi-ai/tests/gemini_fixtures.rs)、[Mistral](../../crates/opi-ai/tests/mistral_fixtures.rs)、[OpenRouter](../../crates/opi-ai/tests/openrouter_fixtures.rs) 六个既有 HTTP 文本测试：要求一次匹配、无流错误、成功 `Done(Stop)` 并消费至 EOF；Anthropic matcher 改为现有带缓存标记的 system 数组形状。 |
| update | [per_request_auth](../../crates/opi-ai/tests/per_request_auth.rs) 七个正例要求实际匹配；Anthropic Bearer 请求同时断言无 `x-api-key`；Codex 使用正确 WireApi 和合成 account ID，并验证 account header。空 SSE 继续只承担认证发送验收。 |
| retain | 其他测试/fixture 归属、真实签名独立验证、私有 WS codec 测试、具体凭据存储测试及生产调用方。 |
| delete | none |

共新增 4 个测试函数、更新 15 个既有函数。表中的场景行不单独计为 Rust 测试。
`.expect(1)` 约束的是匹配请求数，不能统一解读为服务器收到的所有请求总数。

## 执行证据

本机原始证据保留于 `E:/opi/acceptance/opi-ai-test-acceptance-20261021/`：
包括原文件、来源哈希、精确命令、退出码、实际测试数量、耗时、完整日志、变异反例和全新打包 scratch。
`windows-execution.json`、`wsl-execution.json` 记录逐项命令；各 package scratch 内的
`evidence.json` 记录产物、消费者锁文件及 features；传输证明还绑定 native provenance，
早期 Schema 证明绑定 regex/URL/data 依赖图。
这些本机路径不是可移植的仓库产物。

| 检查 | Windows：Rust 1.99.1，x86_64 MSVC | WSL：Rust 1.97.1，x86_64 GNU |
| --- | --- | --- |
| `cargo test --locked --offline -p opi-ai --all-targets` | 1329 个顶层用例 + 2 次 TLS 子进程执行通过，0 失败/忽略 | 1330 个顶层用例 + 2 次 TLS 子进程执行通过，0 失败/忽略 |
| `cargo test --locked --offline -p opi-ai --doc` | 4 通过 | 4 通过 |
| `cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings` | exit 0 | exit 0 |
| 最终 `--lib crypto::` 定向复验 | 15 通过 | 15 通过 |
| 默认 WS 图，已包含在整包 gate 中 | 93 私有 + 16 公开通过 | 93 私有 + 16 公开通过 |
| WS `flate2/default` 图 | 93 私有 + 16 公开通过 | 93 私有 + 16 公开通过 |
| WS `flate2/zlib` 图 | 93 私有 + 16 公开通过 | 93 私有 + 16 公开通过 |
| 实际 WS package，三种图 | 每图 93 私有 + 16 公开 + 6 外部检查及 clippy；package doc 通过 | 每图 93 私有 + 16 公开 + 6 外部检查及 clippy；package doc 通过 |
| 实际 crypto package，两种图 | 每图 6 项外部 TLS 检查 | 每图 6 项外部 TLS 检查 |
| 实际 HTTP package，两种图 | 每图 6 项外部检查 | 每图 6 项外部检查 |
| 实际早期 Schema package，两种图 | 每图 73 项外部检查 | 每图 73 项外部检查 |
| Harness `unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py'` | 70 通过 | 70 通过 |

两平台 runner 的 22 条命令均以 0 退出。`tls_fixtures.rs` 是零测试的辅助 target，
不计为验收 battery；表中的验收数量均非零。
仓库 `cargo fmt --check --all`、`python scripts/opi-doc-check.py`、`git diff --check` 通过，没有忽略测试。

每个平台的四个 family 使用相同的 `opi-ai-0.8.3.crate` 身份：

- Windows SHA-256：`765a41680ea968446945c0a0230a634069450db2f19fd7448ee00830129eed7f`。
- WSL SHA-256：`8e516a5aa058475dc833a111aa539a3a0a37d7f6ffd91ca783bd81a94201b8b0`。

两平台 142 个文件的内容相同；归档元数据仅有 139 个文件的权限不同，
Windows 为 0644，该 WSL 挂载环境为 0755，比较结果保留于 `cross-platform-package-comparison.json`。
`summary.json` 和各 family 的证据分别保留产物及消费者锁文件身份；文件内容相同不合并平台验收记录。

模型相关场景全部使用本地 fixture 或 loopback peer，没有实时 provider 调用或环境凭据激活。
WSL 结果属于 WSL 中的 Linux GNU 执行，不代表另一台物理 Linux 主机。
两组编译器结果均不能证明精确 Rust 1.97.0 或其他 target triple。

Windows 原始基线 `cargo test --locked --offline -p opi-ai --all-targets` 通过 1327 项。
强化断言后实际揭示两处旧测试误判：Anthropic 的旧字符串 system matcher 得到 HTTP 404；
Codex 认证用例因缺少 account 而在发送前被拒绝，服务器实际未收到请求。
修正测试数据后，测试才进入原本声称覆盖的当前契约。

三个可逆反例均在预期断言以 101 退出：允许 worker 发布迟到结果、跳过进入前取消检查、
在缺失 Upgrade 的拒绝行返回有效响应。`mutation-evidence.json` 记录变异前后哈希及逐字节还原。
连接池测试自带公开配置的正反控制。最终生产与测试源码没有残留变异。
复核修正辅助器 mutex 展开路径后，两平台又各跑了一次 crypto 私有测试；其他接口与断言未改变。

## 实际适配器消费者映射

D＝公开请求 body builder；H＝本地真实 HTTP；F＝fixture 事件映射；R＝真实 facade/factory 复用其他 serializer。
该表区分证据类型及能力边界，不把共享实现推定为完整 conformance。

| 消费者 | Transcript | Replay | 工具 schema 投影 | Auth | Usage |
| --- | --- | --- | --- | --- | --- |
| Anthropic | D chronology 折叠/current tools；H 初始 prompt | D 关联、错误对象、未知效果、ID、图片 | D 工具名；完整 schema 值未闭合 | H Bearer/API key 选择及缺失头 | F reported/cache/missing；H 非法子集和 partial |
| OpenAI Chat | D/H 初始 system；chronology 更新未闭合 | D 重用 batch、错误字符串参数、error marker | D strict 开关；完整参数未闭合 | H lifecycle/prepared/policy | F cache/reasoning/missing/zero；H invalid/partial |
| Responses | D/H 初始 instructions；chronology 未闭合 | D same/foreign 双 ID、error/no-leak | D strict；完整参数未闭合 | H lifecycle/prepared/policy | F usage/cache/reasoning；H 非法子集 |
| Codex | H 初始 instructions；chronology 未闭合 | 独立 Codex replay 断言未闭合 | H `strict:null`；完整参数未闭合 | H account/header policy 及修正后的 prepared 请求 | SSE 有输入数值；字段值断言未闭合 |
| Gemini | D/H 初始 system instruction；chronology 未闭合 | D error Struct/no-leak；错误对象反例未闭合 | 完整 declaration 参数未闭合 | H 文本成功、scheme 拒绝 | F usage/cache；H token 数值断言未闭合 |
| Bedrock | H 初始 system；chronology 未闭合 | D foreign thinking、status、图片占位；ID/错误对象反例未闭合 | 完整 `inputSchema.json` 值未闭合 | H SigV4、冻结重试、保护头 | H input/output；F cache；reasoning 不解析有显式测试 |
| Azure | H 初始 system；R Chat | D 真实 facade error marker；其他 R | D facade strict；完整参数未闭合 | H deployment/api-version/API key | F usage；其他 R Chat |
| Vertex | H 初始 system；R Gemini | D 真实 facade error Struct；其他 R | 完整参数未闭合；R Gemini | H project/location/Bearer | F usage；其他 R Gemini |
| Mistral | H 初始 system；R Chat | D 真实 factory error marker；其他 R | D profile strict；完整参数未闭合 | H 文本及 prepared policy | F usage；其他 R Chat |
| OpenRouter | H 初始 system；R Chat | D 真实 factory error marker；其他 R | D profile strict；完整参数未闭合 | H 文本、识别头及 policy | F usage；其他 R Chat |

具体锚点为 `transcript_contract::adapters_consume_only_the_normalized_transcript`（仅 Anthropic）、
`replay_contract::association_matrix_through_real_adapters`（Anthropic/Chat）、
`replay_contract::responses_dual_ids_round_trip_and_cross_drops_item_id`（普通 Responses），以及
`tool_result_wire::tool_result_error_semantics_across_providers`（九个真实 adapter/factory，不含 Codex）。
共享 Schema 编译、参数归一化、最终原 schema 验证由 `schema_engine`、`schema_public` 负责，
不需要为十个 adapter 重复引擎测试。

## 仍未闭合的验收边界

- WS 缺少生产 codec 调用方；Models→Codex→Upgrade→codec、continuation/pool、完整生产资源收尾仍开放。
- 签名 worker 缺少生产 JWT/ADC 调用方；TLS 签名和私有 worker 验证不能关闭该公开组合验收。
- 表中标出的 adapter 单元仍是证据缺口；整包通过不等于本次映射尚未证明的 chronology、完整 schema 投影、独立 Codex replay/usage 单元已经完成。
- 打包证据仅覆盖四个现有 harness family，不代表最终分发、每个模块消费者、所有平台或已注册 Phase 出口；MSVC 与 WSL GNU 分开报告。
- 具体持久化凭据存储及产品行为未改变，保留原测试。本次没有跨 crate runtime 变更，因此没有选择全 workspace、产品或 release gates。

重跑使用[方案中的命令](opi-ai-test-acceptance-plan.zh.md#验收命令)，按受影响范围取并集。
将来的生产调用方或缺失 adapter 单元需要自己的契约测试；本结果不提供新功能实施准入。
