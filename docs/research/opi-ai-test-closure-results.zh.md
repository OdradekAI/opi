# opi-ai 证据闭合与测试目录整理

[English](opi-ai-test-closure-results.md)

## 范围与来源绑定

2026-10-21 继续推进[首轮执行](opi-ai-test-acceptance-results.zh.md)：
关闭通过现有接口可验证的已映射证据缺口，改善私有测试和公开接口测试的可读性。
属于限定范围的测试/文档维护；没有新增或修改生产调用方、公开接口、依赖、feature、
共享 fixture 内容或 Phase 状态。新增的内联测试输入用于区分相应行为场景。

worktree-only HEAD 为 `7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`；
本任务 committed=none、staged=none。上轮修改及其他账本、README、WS、harness 修改均为带入状态。
基线文件及字节哈希保留于 `E:/opi/acceptance/opi-ai-test-closure-20261021/baseline/`，
仅在这些输入上叠加任务所属改动。

carried-in-and-touched 包括 crate README 对、crypto owner、Gemini fixture、
公开 WS target、三个 harness 文件及上轮计划/结果文档对。任务新增改动包括
HTTP/TLS 装配、Codex/replay/transcript 父文件、Vertex fixture 及下文新文件。
其他带入修改（包括 `.opi-impl-state.json`）保留；最终 index/worktree/新文件清单
分别记录于 `final-{staged,unstaged,untracked}.txt`。

## Rust 约定与实际目录调整

Rust 不要求私有测试必须内联在 `mod.rs` 中。
`#[cfg(test)] mod tests;` 可以加载仍具有同等私有访问权限的子模块。
Cargo 根 `tests/` 下的 target 则是跨越库公开接口的独立 crate。
多个私有模块的组合仍是内部证据；一个公开函数也可以是 integration test 的对象。
官方出处和当前目录树记录在双语[测试组织说明](../../crates/opi-ai/tests/README.zh.md)中。

crypto 的生产文件由 874 行降到 256 行；15 个既有测试搬到
`src/crypto/tests/{admission,signing,worker,jwt}.rs`，由小型 `tests/mod.rs` 索引。
测试属性和归一化函数体相同，生产前缀逐字节相同，原私有 fixture 和 verification owner 保留。
`crypto-test-layout-proof.json` 记录全部前后名称及哈希。

TLS 常量从根 `tests/tls_fixtures.rs` 逐字节搬到 `tests/support/tls_fixtures.rs`，
由 HTTP/TLS/WS target 声明内部模块。Cargo metadata 现在发现 47 个 integration target，
不再有空 fixture target。新验收主题均是既有契约 target 的子模块，没有新增 executable。
package guard 及其测试只同步 fixture 路径，没有放宽包内 path/include 校验。

## 已闭合的证据单元

| 单元 | 实际接口及有区分力的证据 |
| --- | --- |
| Transcript/current tools 与完整 schema 投影 | 两个 [provider_projection](../../crates/opi-ai/tests/transcript_contract/provider_projection.rs) 测试实调六个独立 serializer、四个真实 facade/factory。同一 normalized chronology 包含 system 追加、section 替换/删除、工具删除/重定义/新增；断言独立 prompt 字面量、当前名称/顺序/描述、完整嵌套 schema 及 canonical 不变。 |
| Codex 独立 replay | 五个 [Codex replay](../../crates/opi-ai/tests/openai_codex_responses/replay.rs) 案例实调专用 builder：逐维 provider/wire/requested-model/legacy 身份反例、双 ID、成功/错误结果、原始 malformed 字符串、缺失结果的未知执行效果、歧义 batch 的类型拒绝。不以普通 Responses 代替。 |
| Codex typed usage/生命周期 | [openai_codex_responses](../../crates/opi-ai/tests/openai_codex_responses.rs) 中强化两个 HTTP 案例并新增一个表：Stop/Length、精确 input/output/cache/reasoning、未报告与报告零、唯一终态后 EOF、来源、一次 HTTP、canonical 不变。 |
| Gemini/Bedrock 对象参数 replay | 两个 [object_wire](../../crates/opi-ai/tests/replay_contract/object_wire.rs) 测试覆盖 malformed JSON/非对象数组、有无解析诊断、可信非执行事实、精确脱敏上下文及无效关联结果消费；同一 batch 有效调用作为正控，错误文字不能替代可信事实，仍须 Config。断言真实 role 和 call 先于 result。 |
| Bedrock wire ID | 先通过公开 builder 取得别名，再把它作为第二个 canonical ID，持续制造真实碰撞，不复制私有 hash 算法。独立断言合法/有界/唯一、结果同步、保留有效原拼写、role/order、重复构建稳定及 canonical 不变。 |
| Gemini/Vertex HTTP usage | 强化既有 [Gemini](../../crates/opi-ai/tests/gemini_fixtures.rs)、[Vertex](../../crates/opi-ai/tests/vertex_fixtures.rs) 正向 HTTP 测试，断言完整 typed Usage 和精确请求数，不吞掉错误。 |

投影矩阵覆盖 Anthropic、Chat、Responses、Codex、Gemini、Bedrock，及 Azure、Vertex、Mistral、OpenRouter 的真实公开构造。
后四者仍复用底层 serializer；这里新增的是公开 builder 证据，不是新增 HTTP 证据。
矩阵显式使用 Disabled cache，分别验证默认 strict 缺失与 Codex `strict:null`，
不推定原生 mid-conversation placement 或所有 strict/cache/profile 组合。

测试影响：**add 11** 个函数、**update 4** 个既有行为测试、搬移后 **retain 15** 个私有 crypto 用例、
**delete 0** 个测试用例。另更新模块装配/helper 路径，移除一个零测试 helper target。

## 验证与反例

原始命令、来源哈希、精确退出码、实际名称/计数和打包证据保留于
`E:/opi/acceptance/opi-ai-test-closure-20261021/`。
`verified-source-sha256.json`、`windows-execution.json`、`wsl-execution.json`
及各 package 的 `evidence.json` 绑定实际输入与结果。

Windows 使用 Rust 1.99.1（`x86_64-pc-windows-msvc`），WSL 使用 Rust 1.97.1
（`x86_64-unknown-linux-gnu`）；沿用平台既有 target，并保持增量编译开启。

| 实际检查 | Windows MSVC | WSL GNU |
| --- | --- | --- |
| `cargo test --locked --offline -p opi-ai --all-targets` | 1340 个顶层用例 + 2 次 TLS 子进程执行通过 | 1341 个顶层用例 + 2 次 TLS 子进程执行通过 |
| `cargo test --locked --offline -p opi-ai --doc` | 4 通过 | 4 通过 |
| `cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings` | PASS | PASS |
| `python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py'` | 70 通过 | 70 通过 |
| 全新 scratch/稳定 target 下执行 `package_slice.py --family ws --offline` | 三种图每图：私有 93、公开 Upgrade 16、外部消费者 6；clippy 和包内 rustdoc PASS | 同样数量与门禁全部 PASS |
| 实际解包 SDK `--lib crypto::tests::` | 15 通过 | 15 通过 |
| 实际解包 SDK 的 transcript/replay/Codex/Gemini/Vertex 五个公开 target | 111 通过（12/21/20/30/28） | 111 通过（12/21/20/30/28） |

所选命令均以 0 退出；实际 Rust 用例 0 失败/忽略。精确 argv、scratch/manifest 路径及
日志摘要记录在两份 execution JSON 中；`final-checks.json` 记录格式、文档、空白及
最终来源绑定检查。这是单 crate 维护所需的检查并集，不声称工作区/Phase/发行门禁通过。

WSL 多一个既有 Unix 专用代理环境变量用例。计数将两次隔离 TLS 子进程执行与顶层
用例分开。[上轮报告](opi-ai-test-acceptance-results.zh.md)也更正了旧 WSL 正则聚合
漏掉一条交错子进程结果的问题；`historical-count-audit.json` 绑定保留的原始日志。

本轮两个归档均有 152 个文件，逐文件 payload 哈希完全相同。归档 SHA-256 分别为
Windows `a6c5059fede7432585b7c0b5712798d5e229350bacc0659d984f0f9d5f854879`、
WSL `bf7055c9a603b649dbed4ada027b549c98ff21e2ee8d7623e81b1d143ffa5c69`。
149 处元数据差异为文件 mode（`0644` 与 `0755`）；逐成员比较记录于
`artifact-comparison.json`。这些是绑定各平台的证据产物，不声称归档字节可重现。

2026-10-22 准备提交时，将 `tests/support/ws_peer.rs` 的 119 个 CRLF 换行统一为 LF。
解码后的逐行内容相同，前后字节哈希记录于 `commit-line-ending-normalization.json`。
上述归档哈希绑定此前已测试的内容，不代表此次纯换行调整后重新打包的产物。

四个受控反例均在预期断言以 101 退出：丢失嵌套 schema、把 Bedrock 误当字符串参数 wire、
绕过 ID 碰撞分配、改变 Codex fixture 的 token 数值；源码均按字节还原。
复核把硬编码 hash 别名改为公开 probe 并补 role/order 后，重新执行了碰撞反例。
证据见 `counterexamples.json`、`replay-object-wire-review.json`，交付源码没有残留变异。

harness CLI 也在构建前以 2 拒绝了已存在的 scratch。实际打包使用全新 scratch 和既有、带 marker 的平台缓存，
不使用 `cargo clean`、每会话 target、实时 provider 或环境凭据。
先前 crypto/HTTP/Schema 消费者证明继续作为历史证据；本轮刷新受目录调整影响的 WS/package 证明，
并在实际解包 SDK 上执行新闭合的公开测试组，不声称所有未改变的历史 family 都已针对新归档重跑。

## 仍存在的生产与平台边界

表列证据单元仅在上述场景内闭合，不代表所有 adapter/profile 组合、最终 SDK 分发或 Phase 出口。
MSVC、WSL GNU 结果不证明精确 Rust 1.97.0、其他 target triple 或原生 macOS。

仍缺少两条实际生产链路：Models→Codex→WS codec（已注册任务 20.16），以及
JWT/ADC→signing worker/Vertex（20.21.1）。账本两项状态仍为 failing，
不能用测试虚构调用方或标记 passing。
仓库[工作约定](../../AGENTS.md#skills-and-workflow)要求通过显式调用 `opi-implement` 进行实施准入和账本执行；
生产接线仍需自身任务/依赖门禁，不会被静默纳入本次测试维护的闭合范围。
