# opi-ai test organization

[中文](README.zh.md)

Rust supports both inline test modules and separate test files. What matters is
the interface crossed, not the file size or number of modules exercised.
Descendant modules can access ancestor private items; integration targets are
separate crates using the public interface. See [Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html).

| Purpose | Location | Evidence |
| --- | --- | --- |
| Private mechanisms | `src/<module>/tests.rs` or `src/<module>/tests/` | Algorithms, parsers, state transitions and resource ownership |
| Private combinations | The same crate-local test module, grouped by contract | Internal handoff, such as Upgrade-to-codec; no public provider caller is implied |
| Public module/combination acceptance | `tests/<contract>.rs`, optionally with `tests/<contract>/*.rs` submodules | Caller interfaces, actual adapters, request bytes, typed failures and lifecycle |
| Shared integration helpers | `tests/support/*.rs`, declared by consuming targets | Local peers and synthetic fixtures; no separate test executable |
| SDK consumers | `scripts/pi-sdk-conformance/` and external scratch | Actual package, independent dependency graph and public consumer behavior |

The focused layout is:

```text
src/crypto/mod.rs                  # production; cfg(test) mod tests
src/crypto/tests/mod.rs            # private test index/shared helpers
src/crypto/tests/admission.rs
src/crypto/tests/signing.rs
src/crypto/tests/worker.rs
src/crypto/tests/jwt.rs
src/ws/tests/                      # private codec/internal combinations
tests/transcript_contract.rs       # one integration target
tests/transcript_contract/provider_projection.rs
tests/replay_contract.rs           # one integration target
tests/replay_contract/object_wire.rs
tests/openai_codex_responses.rs    # one integration target
tests/openai_codex_responses/replay.rs
tests/tls_contract.rs
tests/ws_upgrade.rs
tests/support/tls_fixtures.rs
tests/support/ws_peer.rs
```

The crypto owner keeps only this test declaration:

```rust,ignore
#[cfg(test)]
mod tests;
```

Small private tests may remain inline. Crypto's admission, signing, worker and
JWT contracts justify separate topics. Its fixture constants remain in the
existing private, test-only `src/crypto/fixtures.rs`.

The [Rust Reference](https://doc.rust-lang.org/reference/items/modules.html#outlined-modules)
supports both `tests.rs` and `tests/mod.rs` and encourages named module files to
avoid many identically named `mod.rs` files. Here the existing WS/crypto
directory convention is retained; the production module tree is not renamed.

Cargo gives each discovered integration target a separate executable. Group
related cases as submodules of an existing contract target before adding another
executable; [Cargo documents the compilation/scheduling tradeoff](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#tests).
Shared helpers go under `tests/support`, rather than becoming root `tests/*.rs`
targets that run zero tests. Do not widen production visibility or copy
production source into integration tests to reach private mechanisms.

A root integration file explicitly declares its topic submodule, for example:

```rust,ignore
#[path = "transcript_contract/provider_projection.rs"]
mod provider_projection;
```

A single public Schema function can be tested by an integration target.
Conversely, several private modules working together remain crate-local
evidence. Directory organization communicates that access boundary; it does not
replace assertions about the actual behavior.

Use the existing [shared Cargo cache](../../../scripts/opi-cargo-cache.py), with
incremental compilation enabled, local fixtures/peers and no live credentials:

```powershell
cargo test --locked --offline -p opi-ai --lib crypto::tests::
cargo test --locked --offline -p opi-ai --lib crypto::tests::worker::
cargo test --locked --offline -p opi-ai --test transcript_contract --test replay_contract --test openai_codex_responses
cargo test --locked --offline -p opi-ai --test http_acceptance --test tls_contract --test ws_upgrade
```

These are logical test scopes, not independent build/release units. Retain the
WS three-graph and [actual SDK package checks](../../../scripts/pi-sdk-conformance/README.md).
Private mechanism success does not close the pending production codec or
JWT/ADC caller.
