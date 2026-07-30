# Phase 15 Remediation Plan

**Date**: 2026-07-29
**Audit sources**: `audit.codex.md`, `audit.glm5.2.md`
**Commit range**: `11d4c28e8d3d4d0f85ff0d53f2bdf9795c95cf4c..d88980f8eb703ceb0ce22a39bad25f42fd21c80c`
**Verification target**: `bfa80d92607f4ca5399448927e60c186e910f094`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md`
**Authoritative correctives consulted**: `docs/research/2026-07-24-phase15-linux-l2-feasibility.md`, `docs/research/2026-07-24-project-trust-semantics-pi-claude-code-codex-cli.md`

---

## Audit cross-reference summary

The two reports contain 29 raw findings. They normalize into 27 behavioral
clusters. The GLM report is pinned to the Phase 15 exit commit `d88980f`; every
cluster was therefore re-verified against current HEAD. Eleven GLM-only
findings were already remediated or refuted by current behavior and do not
produce fix items.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C1 | Trusted staged config reverses project/explicit/env/CLI precedence and validates custom-provider fragments too early | Codex 2.1 | Unique (1/2) | Major | Confirmed |
| C2 | `doctor` and `--list-models` consume project config before the trust gate | Codex 2.2 | Unique (1/2) | Major | Confirmed |
| C3 | Workspace file operations remain vulnerable to an ancestor symlink/junction swap after `PathPolicy` | Codex 2.3 | Unique (1/2) | Major | Confirmed |
| C4 | Dropping bash execution can detach capture tasks and leak spill files | Codex 2.4 | Unique (1/2) | Major | Confirmed |
| C5 | Linux ABI 1-3 drops the independent seccomp socket gate; user-facing strict degradation remains incomplete | Codex 3.1, GLM m3 | Full (2/2) | Major (Codex Major / GLM Minor) | Partially confirmed: Linux remains; GLM's macOS/all-layer portion was remediated |
| C6 | macOS probes one `sandbox-exec` helper but launches a new bare-name resolution | Codex 3.2 | Unique (1/2) | Major | Confirmed |
| C7 | Linux can claim seccomp engagement on unverified `riscv64` | Codex 3.3, GLM m1 | Full (2/2) | Minor | Confirmed residual after partial architecture remediation |
| C8 | Sandbox diagnostic helpers serialize arbitrary secret-bearing reason strings | Codex 4.1 | Unique (1/2) | Minor | Confirmed |
| C9 | `TrustParent` at a filesystem root returns trusted without persisting a durable choice | Codex 4.2 | Unique (1/2) | Minor | Confirmed |
| C10 | Linux L2 native acceptance omits AF_UNIX datagram and distinct TCP-connect probes | Codex 5.1 | Unique (1/2) | Minor | Confirmed |
| C11 | Linux L1 native acceptance omits the temp-directory write carve-out | Codex 5.2 | Unique (1/2) | Minor | Confirmed |
| C12 | Linux L3 danger syscalls are structurally pinned but never denied by a runtime child probe | Codex 5.2, GLM m4 | Full (2/2) | Minor | Confirmed |
| C13 | Phase 15 documentation guards search entire files instead of heading-bounded sections | Codex 5.3 | Unique (1/2) | Minor | Confirmed |
| C14 | Windows clean-exit `disarm` kills survivors while Unix preserves them | GLM M1 | Unique (1/2) | Major | Refuted at HEAD: clean exit now terminates the remaining tree intentionally on every platform |
| C15 | Per-layer `Some(false)` toggles are ignored while building confinement | GLM m2 | Unique (1/2) | Minor | Refuted at HEAD: builders consume the engaged subset |
| C16 | macOS `build_wrapped_argv` is dead and its test misses production composition | GLM m5 | Unique (1/2) | Minor | Refuted at HEAD: helper removed and production composer tested |
| C17 | Legacy trust helper maps `Undecided` to `Trusted` | GLM m11 | Unique (1/2) | Minor | Refuted at HEAD: helper now fails closed |
| C18 | Trust prompt lacks RAII terminal restoration | GLM m12 | Unique (1/2) | Minor | Refuted at HEAD: step-aware RAII guard and failure tests are present |
| C19 | SC1 cites the adapter process-group test under the wrong binary | GLM m14 | Unique (1/2) | Minor | Refuted at HEAD: EN/ZH citations and guard are corrected |
| C20 | CHANGELOG omits the `AppState` derive removals | GLM m13 | Unique (1/2) | Minor | Confirmed |
| C21 | macOS module documentation says the shipped runtime is deferred | GLM m10 | Unique (1/2) | Minor | Refuted at HEAD: module documentation describes the shipped runtime |
| C22 | Atomic-write residue test searches for a stale temp tag | GLM m7 | Unique (1/2) | Minor | Confirmed |
| C23 | Write/Edit lack rejected-path-before-backend regression tests | GLM m8 | Unique (1/2) | Minor | Confirmed test gap; production order is currently correct |
| C24 | Untrusted themes/extensions are not behaviorally asserted | GLM m9 | Unique (1/2) | Minor | Refuted at HEAD: global/project positive and negative assertions are present |
| C25 | Seatbelt ordering comment says first-match-wins | GLM m6 | Unique (1/2) | Minor | Refuted at HEAD: comment and assertion say last-match-wins |
| C26 | RPC trust test hardcodes trusted state and misses headless policy | GLM i1 | Unique (1/2) | Info | Partially confirmed: original cause fixed; only optional full-binary coverage remains |
| C27 | Adapter trust fixture has no real `[adapter]` and cannot prove spawn suppression | GLM i2 | Unique (1/2) | Info | Refuted at HEAD: real marker adapters exercise production startup |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1 | Retain parsed raw user/explicit layers and CLI/env overlays, expose only trust inputs before the decision, then finalize exactly once with the authorized project layer in canonical order and validate custom providers once. | Avoids configuration rereads, duplicated precedence logic, and premature validation. The user selected the recommended staged-raw-layer design. | user |
| D2 | C2 | Route all early config-consuming commands through the reusable headless trust preflight while continuing to honor explicit `--config`. | `--no-trust` must suppress project input consistently; explicit config remains user-authorized. | auto |
| D3 | C3 | Preserve the public `FileOperations` trait and harden `LocalFileOperations` with a held workspace-root capability plus component-safe, handle-relative traversal and same-parent replacement. | Closes the shipped local race with the smallest API surface change. Custom/remote backends continue to own their filesystem semantics. The user selected this approach. | user |
| D4 | C4 | Give capture tasks and spill paths RAII ownership; dropping execution aborts owned tasks and dropping a spill removes it. | Cleanup must not depend on reaching the normal join path. | auto |
| D5 | C5 | Represent the seccomp new-socket gate and Landlock TCP rights as distinct Linux network sub-capabilities; retain the socket gate under fail-open and report the TCP gap. | This is the normative strongest-engaged-baseline behavior and the auditors' convergent recommendation. | auto |
| D6 | C6 | Probe and launch the canonical absolute `/usr/bin/sandbox-exec` path, retaining that exact identity in the backend/confinement plan. | A second PATH lookup or pass-through shim invalidates capability evidence. | auto |
| D7 | C7 | Whitelist Linux x86_64 and aarch64 before `seccompiler::TargetArch` conversion. | Only those architectures have normative build/runtime acceptance. | auto |
| D8 | C8 | Replace arbitrary diagnostic reason strings with a closed, redaction-safe reason type and static serialized text. | A best-effort string sanitizer cannot guarantee that arbitrary credentials are absent. The user accepted the 0.x API change. | user |
| D9 | C9 | Remove/disable `TrustParent` when the canonical project root has no parent; reject a forged direct selection with a named error. | Avoids both silent non-persistence and a dangerously broad root fallback. The user selected this behavior. | user |
| D10 | C10-C12 | Add native runtime probes for the missing Linux L1/L2/L3 cases. | These are additive acceptance tests with one clear direction. | auto |
| D11 | C13 | Extract heading-bounded EN/ZH Phase 15 and README sandbox/trust sections before applying claim/stale-text guards. | Makes the tests prove claim placement as well as presence. | auto |
| D12 | C20 | Add an Unreleased breaking-change note naming the lost `Copy`, `Clone`, `PartialEq`, and `Eq` derives. | The public 0.x break already occurred and needs truthful release documentation. | auto |
| D13 | C22, C23 | Correct the residue predicate and add Write/Edit rejected-path zero-call tests. | Both are direct, additive test corrections. | auto |
| D14 | C14-C19, C21, C24-C27 | Do not carry stale/refuted findings into implementation; retain their current regression evidence. | Current source either fixes the cited behavior or invalidates the original premise. | auto |

## Remediation layers

Workspace dependency order:

```text
Layer 1: opi-ai, opi-tui
Layer 2: opi-agent -> opi-ai
Layer 3: opi-coding-agent -> opi-ai, opi-agent, opi-tui
Layer 4: documentation
```

`opi-ai` and `opi-agent` require no Phase 15 remediation changes. The affected
substrate is `opi-tui`; all runtime and acceptance changes are owned by
`opi-coding-agent`.

### Layer 1: `opi-tui` trust-prompt substrate

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-tui --all-targets -- -D warnings
    cargo test -p opi-tui --all-targets

#### Fix 1.1: Make `TrustParent` availability explicit

- **Audit source**: Codex 4.2
- **Cluster**: C9
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-tui/src/trust_prompt.rs` ~L41-L69, ~L183-L235
- **Change**: Let `TrustPrompt` receive the available choice set (or an equivalent `allow_parent` input), omit `TrustParent` at filesystem roots, and keep selection/index navigation correct for both four- and five-choice prompts.
- **Test plan**: Add deterministic widget/input tests for the normal five-choice prompt and the root four-choice prompt, including number-key and up/down selection behavior.

### Layer 3: `opi-coding-agent` product behavior and acceptance

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Finalize trusted configuration in canonical order

- **Audit source**: Codex 2.1
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/config.rs` ~L1553-L1644; `crates/opi-coding-agent/src/main.rs` ~L228-L387; `crates/opi-coding-agent/tests/config_tests.rs`; `crates/opi-coding-agent/tests/trust_resource_gating.rs`
- **Change**: Replace incremental `OpiConfig` project merging with a staged raw-layer resolver. Parse user and explicit inputs before trust, retain CLI/env overlays and the unread project path, expose only the global trust default before authorization, then merge `user -> authorized project -> explicit config -> env -> CLI` and validate the completed provider namespace once. An untrusted project file must remain unread.
- **Test plan**: Compare staged trusted output with `resolve_config` for CLI/env/explicit model precedence, sandbox fields, OpenAI-compatible entries, and custom-provider fragments split across user/project/explicit layers. Prove malformed project TOML is ignored when untrusted and fails only when trusted.

#### Fix 3.2: Trust-gate early config consumers

- **Audit source**: Codex 2.2
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/main.rs` ~L39-L77, ~L528-L560; `crates/opi-coding-agent/src/cli.rs` ~L111-L122; `crates/opi-coding-agent/tests/doctor_cli.rs`; `crates/opi-coding-agent/tests/list_models.rs`
- **Change**: Reuse the staged resolver and `HeadlessPreTrustUi` before `doctor` or `--list-models` receives an `OpiConfig`. Preserve early/no-provider execution and explicit `--config` behavior.
- **Test plan**: Add subprocess cases showing `--no-trust` ignores malformed and provider-defining project config, `--trust` consumes it, headless ask defaults untrusted, and explicit/user config still affects both commands.

#### Fix 3.3: Make local workspace file access handle-relative

- **Audit source**: Codex 2.3
- **Cluster**: C3
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/mod.rs` ~L64-L109; `crates/opi-coding-agent/src/tool/operations.rs` ~L331-L384, ~L482-L575; `crates/opi-coding-agent/src/tool/read.rs` ~L131-L147; `crates/opi-coding-agent/src/tool/write.rs` ~L84-L100; `crates/opi-coding-agent/src/tool/edit.rs` ~L110-L127; `crates/opi-coding-agent/src/harness.rs`
- **Change**: Construct `LocalFileOperations` with a canonical workspace-root handle/capability. For paths lexically inside that root, traverse components without following symlinks/reparse points and perform read, metadata, mkdir, staging, and rename relative to held directory handles. Keep explicitly allowed external interactive reads on the ambient path. Use safe platform abstractions; do not add `unsafe` to `tool/operations.rs`.
- **Test plan**: Add deterministic barriers that swap a checked ancestor after `PathPolicy` returns. Read, Write, and Edit must neither read nor modify an outside sentinel; atomic replacement must remain relative to the verified parent. Run equivalent Unix symlink and Windows junction/reparse tests.

#### Fix 3.4: Own bash capture tasks and spill cleanup with RAII

- **Audit source**: Codex 2.4
- **Cluster**: C4
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/operations.rs` ~L785-L839, ~L952-L983, ~L1083-L1166
- **Change**: Add an owned capture-task guard whose `Drop` aborts unfinished join handles, and a spill owner whose `Drop` closes/removes the private file. Normal completion consumes/disarms task ownership only after both captures are drained.
- **Test plan**: Exceed the in-memory cap, observe spill creation, drop the execution future, wait for L0 teardown, and assert both capture tasks end and no process-tagged spills remain. Unit-test spill cleanup on normal, error, and aborted capture paths.

#### Fix 3.5: Retain the Linux socket gate below Landlock ABI 4

- **Audit source**: Codex 3.1, GLM m3
- **Cluster**: C5
- **Decision**: D5
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox.rs` ~L313-L433; `crates/opi-coding-agent/src/sandbox/linux.rs` ~L140-L169, ~L694-L770, ~L870-L896; `crates/opi-coding-agent/src/tool/operations.rs`
- **Change**: Track `seccomp_socket_creation` separately from `landlock_tcp_bind_connect`. With network requested on a verified architecture, fail-open ABI 1-3 must still build/apply the socket-family deny rules while emitting the TCP capability gap; `require = true` must refuse before spawn.
- **Test plan**: Inject ABI 1 and ABI 3 through the production preparation path and assert AF_INET/AF_INET6/AF_NETLINK socket creation returns `EPERM`, AF_UNIX remains available, the degraded diagnostic names only the TCP gap, and `require = true` produces no child side effect.

#### Fix 3.6: Bind macOS execution to the probed helper

- **Audit source**: Codex 3.2
- **Cluster**: C6
- **Decision**: D6
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox/macos.rs` ~L65-L89, ~L233-L300; `crates/opi-coding-agent/src/tool/operations.rs` ~L643-L673; `crates/opi-coding-agent/tests/sandbox_strict.rs`
- **Change**: Probe `/usr/bin/sandbox-exec`, retain that absolute path in `SandboxExecStatus::Available` and `Confinement`, and launch exactly it. Do not accept a PATH shim as engagement evidence.
- **Test plan**: Add host-independent resolver/composer tests with a pass-through PATH shim and PATH reordering after the probe; the production launcher must remain `/usr/bin/sandbox-exec`. Retain native macOS engaged subprocess tests.

#### Fix 3.7: Limit Linux seccomp engagement to verified architectures

- **Audit source**: Codex 3.3, GLM m1
- **Cluster**: C7
- **Decision**: D7
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox/linux.rs` ~L238-L242, ~L600-L714; `crates/opi-coding-agent/tests/sandbox_linux_backend.rs` ~L212-L217
- **Change**: Reject every architecture name except `x86_64` and `aarch64` before converting to `TargetArch`; use the same helper for availability and confinement construction.
- **Test plan**: Table-test x86_64/aarch64 engagement and riscv64/mips64/unknown permanent unavailability, including `require` fail-open/fail-closed behavior.

#### Fix 3.8: Make sandbox diagnostic reasons closed and redaction-safe

- **Audit source**: Codex 4.1
- **Cluster**: C8
- **Decision**: D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/diagnostics.rs` ~L29-L66; `crates/opi-coding-agent/src/sandbox.rs`; `crates/opi-coding-agent/src/sandbox/linux.rs`; `crates/opi-coding-agent/src/sandbox/macos.rs`; `crates/opi-coding-agent/src/sandbox/windows.rs`; `crates/opi-coding-agent/tests/sandbox_config.rs` ~L366-L380
- **Change**: Introduce a closed `SandboxReason`-style value whose variants serialize to curated static text. Map raw backend/probe errors to a variant before diagnostic construction; never accept arbitrary `String`/`Into<String>` at the public helper boundary.
- **Test plan**: Feed raw probe/build errors containing credential, command, and absolute-path canaries through every mapper and assert the serialized diagnostic contains none of them while preserving exact `{layer, reason}` shape.

#### Fix 3.9: Reject root-level `TrustParent` defensively

- **Audit source**: Codex 4.2
- **Cluster**: C9
- **Decision**: D9
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/interactive.rs` ~L74-L130, ~L237-L291; `crates/opi-coding-agent/src/project_trust.rs` ~L712-L749; `crates/opi-coding-agent/tests/interactive_trust.rs`; `crates/opi-coding-agent/tests/project_trust_store.rs`
- **Change**: Canonicalize the project root before prompt construction, pass parent availability to `TrustPrompt`, and return a named trust error if a direct/stale `TrustParent` choice reaches `apply_ui_choice` without a parent.
- **Test plan**: Cover Unix `/` and a Windows drive root with isolated stores. The prompt must omit `TrustParent`, no record may be written accidentally, and a forged direct selection must fail without returning `Trusted`.

#### Fix 3.10: Complete native Linux L1/L2/L3 operation coverage

- **Audit source**: Codex 5.1, Codex 5.2, GLM m4
- **Cluster**: C10, C11, C12
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/sandbox_strict.rs` ~L862-L953, ~L1042-L1129; `crates/opi-coding-agent/tests/sandbox_linux_backend.rs` ~L20-L172; `.github/workflows/ci.yml`
- **Change**: Extend the native Linux child probe with AF_UNIX datagram round-trip, a distinct TCP-connect attempt against a reachable loopback listener, a temp-directory write, and a safe L3 danger syscall. Keep structural tests as complementary evidence.
- **Test plan**: Prove the unconfined L3 baseline succeeds before expecting strict `EPERM` (prefer `ptrace(PTRACE_TRACEME)` if stable on the runner), assert L3-disabled behavior separately, and require CI filters to execute nonzero tests for every new case.

#### Fix 3.11: Scope Phase 15 documentation guards to their sections

- **Audit source**: Codex 5.3
- **Cluster**: C13
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs` ~L27-L35, ~L166-L264
- **Change**: Add heading-slice helpers for the Phase 15 sections of `docs/opi-spec.md` and `docs/opi-spec.zh.md` and the sandbox/trust sections of `README.md` and `README.zh.md`; run presence and stale-claim checks only against those slices.
- **Test plan**: Add mutation-style fixtures showing that moving a required marker to another section fails and that EN/ZH heading boundaries are both recognized.

#### Fix 3.12: Repair Operations regression tests

- **Audit source**: GLM m7, GLM m8
- **Cluster**: C22, C23
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/tests/tools_read_write_edit_bash.rs` ~L542-L570; `crates/opi-coding-agent/tests/tool_operations.rs` ~L689-L801, ~L948-L980
- **Change**: Scan for the production `opi-ops-tmp` tag and compare directory state before/after writes. Add WriteTool and EditTool outside-workspace cases using a recording backend.
- **Test plan**: Assert successful, failed, and cancelled atomic operations leave no new production-tag residue. For rejected Write/Edit calls assert zero metadata, mkdir, read, and write calls plus the outside-workspace diagnostic.

### Layer 4: Documentation

**Verification**:

    cargo test -p opi-coding-agent --test phase15_safety_sandbox_docs

#### Fix 4.1: Document Linux strict partial capability behavior

- **Audit source**: GLM m3
- **Cluster**: C5
- **Decision**: D5
- **Verification status**: Partially confirmed
- **File(s)**: `README.md` ~L336-L340; `README.zh.md` corresponding sandbox table/section
- **Change**: State that Linux ABI 1-3 retains the seccomp new-socket gate but lacks Landlock TCP bind/connect, emits a degraded diagnostic under fail-open, and fails closed with `require = true`. Keep the wording defense-in-depth and do not claim complete network isolation.
- **Test plan**: Extend the section-scoped EN/ZH guard with the exact partial-capability and residual markers.

#### Fix 4.2: Record the `AppState` derive break

- **Audit source**: GLM m13
- **Cluster**: C20
- **Decision**: D12
- **Verification status**: Confirmed
- **File(s)**: `CHANGELOG.md` ~L8-L19; evidence in `crates/opi-tui/src/lib.rs` ~L138-L151
- **Change**: Under `## [Unreleased]` / `### Breaking Changes`, state that `AppState` lost `Copy`, `Clone`, `PartialEq`, and `Eq` because `AwaitingTrustState` carries a oneshot sender.
- **Test plan**: Review the rendered changelog entry against the current public derive set; no localized changelog counterpart exists.

## Final verification

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps

Native and target-matrix verification:

- Linux x86_64: ABI 1/3 partial-network composition, AF_UNIX stream/datagram,
  denied new INET/INET6/NETLINK sockets, Landlock bind/connect where ABI 4+,
  temp write, and L3 runtime denial.
- Linux aarch64: compile the same filter and capability paths; retain native or
  emulated coverage required by CI.
- macOS x86_64/aarch64: probe and launch the absolute system helper and rerun
  engaged L1/L2 product tests.
- Windows x86_64/aarch64: run junction/reparse ancestor-swap coverage and
  retain Job-Object L0 tests.
- Cross-compile all six release triples from `.github/workflows/ci.yml`.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| GLM M1 / C14 | Refuted at current HEAD | Clean exit deliberately terminates remaining descendants on Unix and Windows while retaining the direct child's exit status; `disarm` is no longer the normal completion path. |
| GLM m2 / C15 | Refuted at current HEAD | Requested/engaged layer subsets now drive Linux and macOS confinement construction. |
| GLM m5 / C16 | Refuted at current HEAD | `build_wrapped_argv` was removed and the production command composer is tested. |
| GLM m11 / C17 | Refuted at current HEAD | `resolve_project_trust_decision` now maps resource-bearing `Undecided` projects to `Untrusted`. Removing the remaining compatibility helper is unrelated cleanup. |
| GLM m12 / C18 | Refuted at current HEAD | `TrustPromptTerminalGuard` restores each entered terminal state in `Drop`, with injected failure-path tests. |
| GLM m14 / C19 | Refuted at current HEAD | Both specifications cite `sandbox_l0::adapter_process_group_contract`, and the guard rejects the old citation. |
| GLM m10 / C21 | Refuted at current HEAD | macOS module documentation describes the shipped, cfg-gated runtime. |
| GLM m9 / C24 | Refuted at current HEAD | Untrusted/ trusted tests separately assert project themes and extensions absent/present while global resources remain. |
| GLM m6 / C25 | Refuted at current HEAD | The comment and ordering assertion now correctly state Seatbelt last-match-wins behavior. |
| GLM i1 / C26 | Info/No action | `RpcRunner::new` now receives explicit `Untrusted`; separate preflight coverage proves headless ask. A full-binary RPC startup test would be additive confidence, not remediation of the original claim. |
| GLM i2 / C27 | Refuted at current HEAD | A real `[adapter]` marker fixture runs through installed-package startup and proves only the global adapter spawns. |
| Complete Linux network isolation | Deferred/out of scope | Inherited descriptors, non-TCP traffic, and `io_uring` remain documented Phase 15 residuals. |
| Public `FileOperations` capability redesign | Deferred by user decision | The selected remediation hardens the shipped local backend without changing the public trait. Revisit only when a concrete remote backend requires a typed verified-path contract. |
