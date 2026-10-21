# opi-ai 测试组织

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
tests/transcript_contract/provider_projection.rs
tests/replay_contract.rs           # 一个 integration target
tests/replay_contract/object_wire.rs
tests/openai_codex_responses.rs    # 一个 integration target
tests/openai_codex_responses/replay.rs
tests/tls_contract.rs
tests/ws_upgrade.rs
tests/support/tls_fixtures.rs
tests/support/ws_peer.rs
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

使用现有[共享 Cargo 缓存](../../../scripts/opi-cargo-cache.py)，保持增量编译，
使用本地 fixture/peer，不使用实时凭据：

```powershell
cargo test --locked --offline -p opi-ai --lib crypto::tests::
cargo test --locked --offline -p opi-ai --lib crypto::tests::worker::
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test openai_codex_responses
cargo test --locked --offline -p opi-ai --test http_acceptance --test tls_contract --test ws_upgrade
```

这些是逻辑测试范围，不是独立构建/发布单元。保留 WS 三种依赖图及
[实际 SDK 打包验收](../../../scripts/pi-sdk-conformance/README.md)。
私有机制通过，不能关闭尚未接入的生产 codec 或 JWT/ADC 调用方验收。
