# Phase 15 Safety & Sandbox Design

Historical note: under the 2026-07-10 roadmap redesign
(`docs/superpowers/plans/2026-07-10-phase-roadmap-redesign-map.md`), Phase 15 is
**Safety & Sandbox** (cluster K, with the cluster L Operations seam folded in).
The prior Phase 14 (TUI product polish,
`2026-06-24-phase14-tui-product-polish-design.md`) is renumbered to Phase 17.
This doc synthesizes tickets T4 (OS-native sandbox), T5 (per-tool Operations
seam), and T6 (project-trust gate), all resolved 2026-07-11. Phase 15 is fully
resolved: T4 + T5 + T6 are the only tickets.

## Overview

Phase 15 closes the safety/sandbox cluster (cluster K) identified by the
pi-0.80.6 realignment under posture B (strategic gap-closing). It introduces an
OS-native subprocess-tree sandbox for `bash`, a per-tool `Operations` seam
giving the sandbox a structurally correct home, and a project-trust gate that
closes the native-child-process blast-radius gap opi's adapter architecture
opens relative to pi's in-process jiti extensions.

All three subsystems are Rust-native and preserve the construction-ownership
invariant. The sandbox is positioned as opt-in defense-in-depth, explicitly not
a security boundary; untrusted code belongs in a container or VM (pi parity).

The phase is implementation-ready: each subsystem lists concrete types,
signatures, crate placement, and verified source touch points. `opi-implement`
breaks this doc into tasks; it is not itself a task list.

## Goals

- An OS-native subprocess-tree sandbox for `bash` covering Linux (seccomp +
  Landlock), macOS (`sandbox-exec` + process group), and Windows (Job Object),
  confining only the spawned subprocess tree, not opi itself.
- An L0 baseline (process-group / Job-Object tree-kill) that ships always-on as
  a correctness fix — bash has neither `process_group(0)` nor a Job Object today
  (`crates/opi-coding-agent/src/tool/bash.rs:86-94`, only `kill_on_drop(true)`
  at `:90`) — plus opt-in L1/L2/L3 `strict` layers default-off.
- A per-tool `Operations` seam (`FileOperations` + `BashOperations` traits)
  layered below `PathPolicy` pre-flight, with `Arc<dyn>` constructor injection
  and local default impls; the T4 sandbox lives inside local
  `BashOperations::exec`.
- A project-trust gate that gates loading of project-local resources —
  including project-local adapter declarations, so an untrusted project's native
  adapter children never start — backed by a pi-style `ProjectTrustStore` and a
  full ask UX.
- Fallback diagnostics that default to fail-open-with-diagnostic, with opt-in
  `require = true` for fail-closed behavior in CI/untrusted contexts.
- No `unsafe` block in opi code: the only unsafe is the irreducible std
  `pre_exec` *contract*, delegated to audited libraries.

## Non-Goals

- No confinement of opi itself (rejected in T4 D1; a partial in-process sandbox
  must not be mistaken for a security boundary; pi `security.md:35` parity).
- No strict-confinement of adapters in Phase 15 (T4 D7): adapters get L0 only;
  per-adapter capability declarations are a deferred follow-up.
- No confinement of in-process file tools (read/write/edit): they stay
  `PathPolicy`-guarded (T4 confines only the bash subprocess tree; T5
  `FileOperations` is unsandboxed).
- No SSH/container remote backends in Phase 15 (T5 D5): the seam is shipped
  with local impls only; remote delegation is future/examples.
- No nav-tool `Operations` (grep/find/ls/glob) in Phase 15 (T5 D3): the
  `ignore`-crate `WalkBuilder` traversal cannot be cleanly redirected to a
  remote backend without losing gitignore semantics.
- No trust-gating of tool execution (T6 D1, pi parity): once in a session,
  tools run with the user's privileges; trust decides what project resources
  *load*, not what the model asks tools to *do*.
- No auto-injection of project-local `AGENTS.md`/`CLAUDE.md` for untrusted
  projects (T6 D2, deliberate pi divergence): the files remain readable via the
  `read` tool.
- No schema versioning or metadata for `trust.json` in Phase 15 (T6 D3).
- No new OAuth providers, credential store changes, or session-schema changes
  (those belong to Phases 14 and 13 respectively).

## Relationship to pi

pi's sandbox (`security.md`) is positioned as defense-in-depth, explicitly not a
security boundary, and confines subprocess trees rather than the pi process
itself. opi matches this posture. The justification is stronger for opi than for
pi: opi's adapters are native child processes
(`crates/opi-coding-agent/src/adapter_host.rs:166-187`, `Command::new` with
`.spawn()` at `:187`), strictly higher blast radius than pi's in-process jiti
extensions. Phase 15 still confines only bash (D7), but the threat-model split
T4 establishes (subprocess tree in scope, opi itself out of scope) is the same.

pi splits the Operations seam the same way opi's T5 chooses to: pi's
`agent` package (`packages/agent`) has no `tools/` dir, only the generic
`AgentTool` trait; all 7 per-tool `Operations` interfaces
(`BashOperations`/`EditOperations`/`FindOperations`/`GrepOperations`/
`LsOperations`/`ReadOperations`/`WriteOperations`) live in pi's
`coding-agent` package (`packages/coding-agent/src/core/tools/`). opi matches
the split: the `FileOperations`/`BashOperations` traits live in
`opi-coding-agent`, and `opi-agent`'s `Tool` trait is unchanged. (pi has an
eighth `FileOperations` interface in `coding-agent/src/core/compaction/utils.ts`
for compaction plumbing, outside the `tools/` dir, so the "7 Operations in
tools/" count is correct as scoped.)

pi has a `ProjectTrustStore` (`trust-manager.ts:43-57`) with an ancestor walk
and always/never/ask decision, gating loading of project-local resources. opi
matches the store design (flat `Map<canonical_path, bool>`, ancestor walk,
realpath key, `fs4` lock, acquire-then-reread). opi deviates on two points:
project-local `AGENTS.md`/`CLAUDE.md` are not auto-injected for untrusted
projects (prompt-injection channel), and trust-gating extends to project-local
adapter declarations (opi's native adapters are higher-blast-radius than pi's
in-process jiti, so gating at declaration-load is load-bearing for opi where it
is not for pi).

## Load-bearing invariant

The construction-ownership invariant (map line 33-42: `opi-agent` must not
construct providers or own provider/auth configuration; `opi-agent` calls
`provider.stream` at `agent_loop.rs:118` but builds nothing) extends to Phase
15's new surfaces: `opi-agent` must not gain sandbox, trust, UI, or Operations
code. The abstract types live nowhere in opi-agent; the concrete
implementations, IO, env, and TUI/prompt surfaces live in `opi-coding-agent`.

Unlike Phase 14 (whose abstract `CredentialStore`/`OAuthProvider` traits live
in `opi-ai` because `opi-ai`'s provider runtime invokes them), Phase 15's
abstract types live in `opi-coding-agent`: `agent_loop` never invokes
`Operations`, `PathPolicy`, or trust resolution (deciding principle in the T5
Location section), so they have no `opi-ai` home.

- The `Tool` trait (`crates/opi-agent/src/tool.rs:21-27`, 4-arg `execute`) is
  unchanged; `agent_loop.rs:802-803` calls `tool.execute` with zero references
  to `Operations` or `PathPolicy`.
- `PathPolicy` already lives in `opi-coding-agent` (verified: zero `PathPolicy`
  hits in `crates/opi-agent/`); the symlink-escape detection lives at
  `crates/opi-coding-agent/src/tool/mod.rs:83`, and the nav walker is at
  `tool/mod.rs:199`.
- The `Extension` trait (`crates/opi-agent/src/extension.rs:185-326`) has zero
  `project_trust`/`trust` methods across its entire surface; the
  `set_trace_collector` method at `:325` is the trait's final member.
- opi has no sandbox crate, no sandbox config, no `ProjectTrustStore`, no
  `trust.json`, no `/trust` slash command, no `--trust`/`--no-trust`/`--approve`
  flag, and no `AwaitingTrust` `AppState` variant today.
- `before_tool_call` fails open by default (`crates/opi-agent/src/hooks.rs:88`
  returns `BeforeToolCallResult::Allow`; the production override at
  `crates/opi-coding-agent/src/runner.rs:744` also defaults `Allow`).

Phase 15 honors this split. The `FileOperations`/`BashOperations` traits,
the local + sandboxed impls, the T4 sandbox mechanism, the `ProjectTrustStore`,
the trust prompt, and the `project_trust` extension hook are all defined in
`opi-coding-agent`. opi-tui gains the new `AppState::AwaitingTrust` variant and
prompt widget. opi-agent is unchanged.

## Implementation Priority and Crate Boundaries

| Priority | Scope | Owner | Requirement |
|---|---|---|---|
| P0 | L0 bash correctness fix (`process_group(0)` Unix, Job Object Windows) | `opi-coding-agent` | Always-on subprocess-tree tree-kill; bash spawn (`bash.rs:86-94`) currently has neither — entirely new work. Adapter spawn (`adapter_host.rs:180`) already has `process_group(0)` on Unix; Job-Object assign is the D4 add on top. |
| P0 | L1/L2/L3 `strict` sandbox for bash | `opi-coding-agent` | Linux: `extrasafe` + `landlock` via `pre_exec` (parent pre-build, child raw-apply); macOS: `sandbox-exec -p` deny-overlay; Windows: L0-only, degrades to L0 with diagnostic. |
| P0 | `[sandbox]` config + `--sandbox` flag | `opi-coding-agent` | `mode = "off"\|"strict"` (default `off`, ships L0 always); `require = false`; optional `fs`/`network`/`syscalls` per-layer toggles; CLI override mirrors `--allow-mutating`. |
| P0 | `FileOperations` trait + local impl | `opi-coding-agent` | read+write+edit; `read_file`/`write_file`/`mkdir`/`metadata`/`access`; atomic temp+rename is an impl detail. Layers below `PathPolicy` pre-flight. |
| P0 | `BashOperations` trait + local impl (sandboxed) | `opi-coding-agent` | Bash exec; the T4 sandbox lives inside local `BashOperations::exec` (the bash spawn moves out of `bash.rs:86-94` into the impl, which deletes that block from the tool). |
| P0 | `Arc<dyn FileOperations>` / `Arc<dyn BashOperations>` constructor injection | `opi-coding-agent` | `build_tools` (`harness.rs:762`) wires the local default; `ReadTool`/`WriteTool`/`EditTool` gain `ops: Arc<dyn FileOperations>`; `BashTool` gains `ops: Arc<dyn BashOperations>`; grep/find/ls/glob constructors unchanged (4 of 8 tools widen). |
| P0 | `ProjectTrustStore` + `trust.json` + `fs4` lock | `opi-coding-agent` | `{user_config_dir}/trust.json`, flat `Map<canonical_path, bool>`, ancestor walk, realpath key, acquire-then-reread. No schema version. Consulted once at session-start before `discover_resources` consumes project layers. |
| P0 | TUI trust prompt + `AppState::AwaitingTrust` | `opi-tui` (variant + widget) / `opi-coding-agent` (state transitions) | Trust / Trust-parent / Trust-session / Deny / Deny-session, mirroring pi; fired only on first cd into a project with trust-requiring resources. |
| P0 | `--trust` / `--no-trust` + `default_project_trust` + `/trust` | `opi-coding-agent` | CLI flags; `[defaults] default_project_trust = "ask"\|"always"\|"never"` (global-only, default `ask`); slash command for mid-session change. |
| P1 | `project_trust` extension hook trait | `opi-coding-agent` | User-global + CLI `-e` resolvers only; minimal pre-trust UI surface (`select`/`confirm`/`input`/`notify`); first yes/no wins, `Undecided` falls through. |
| P1 | Sandbox fallback diagnostics | `opi-coding-agent` | Additive `CODE_SANDBOX_DEGRADED` and `CODE_SANDBOX_UNAVAILABLE` (`&'static str`) + `SOURCE_SANDBOX`, produced from opi-coding-agent; structured `{layer, reason}` rides in `Diagnostic.details` as `serde_json::Value`. No opi-agent Diagnostic struct change. |

Phase 15 must not satisfy acceptance with the abstract traits alone. Each P0
item needs a production path from config through `build_tools` into the local
impls and the bash spawn, exercised by mock-`Operations` integration tests
(mirrors `opi-ai::test_support::MockProvider`). Sandbox tests must not require
elevated privileges or a specific kernel version; degrade paths are covered by
the fallback diagnostics.

## Design

### T4 — OS-native subprocess sandbox

**Posture.** Confine the bash subprocess tree only; opi itself stays
unconfined. Positioned as opt-in defense-in-depth, explicitly not a security
boundary — untrusted code belongs in a container or VM (pi `security.md:35`
parity). ADR-019 is a permission-*popup* subsystem decision, not in conflict;
T4 complements it rather than duplicates it.

**Layers.** L0 baseline (subprocess-tree lifecycle: process group / Job Object
tree-kill) is **default-on**; L1 (filesystem), L2 (network), L3 (syscalls) are
**opt-in under `strict`**, default off. Per-platform matrix:

| Platform | L0 | L1 (FS) | L2 (net) | L3 (syscalls) |
|---|---|---|---|---|
| Linux 5.13+ | process group | Landlock | Landlock (6.2+ net) | extrasafe/seccomp |
| macOS | process group | sandbox-exec | sandbox-exec | n/a |
| Windows | Job Object | n/a | n/a | n/a |

**Linux mechanism.** `extrasafe` 0.5.1 (builds on `seccompiler`,
`#![deny(unsafe_code)]`, no C deps) for syscalls, composed with `landlock` 0.4.5
(MSRV 1.71, kernel 5.13+ for FS / 6.2+ for net; 4 contained unsafe syscall
wrappers inside the crate). Both crate MSRVs sit below the workspace floor of
1.97; the 5.13/6.2 kernel floors are covered by the D6 fail-open-with-diagnostic
path on older kernels. Applied in the child's `pre_exec` on the bash spawn.
The **parent** pre-builds the ruleset/BPF; the **child** runs only the raw apply
syscalls (`landlock_restrict_self`, `seccomp(SECCOMP_SET_FILTER)`) — both
async-signal-safe, satisfying the `pre_exec` contract. There is no `unsafe`
block in opi code: the only unsafe is the irreducible std `pre_exec` *contract*,
delegated to audited libs. Rejected alternatives: a helper binary (ships a
second artifact across six release targets) and `bwrap` (external runtime dep,
violates the Rust-native posture).

**macOS mechanism.** L1/L2 via direct `sandbox-exec -p <templated profile>`
(inherently child-only — `sandbox-exec` IS the helper, so no `pre_exec`/`unsafe`
in opi); L0 is `process_group(0)`. The profile is a deny-only overlay on the
allow-all default: `(deny file-write* (subpath "/"))` with workspace+temp
exceptions plus `(deny network*)`. The `sandbox-run` crate is rejected (provenance
unclear — the linked repo is a PHP toolchain manager, ~631 downloads). macOS is
the highest-fallback-probability target (`sandbox-exec` soft-deprecated since
Sierra 2016, still functional on macOS 15, no removal date; MDM-block status
unverified) and degrades to L0 + diagnostic when unavailable.

**Windows mechanism.** L0 only — `win32job` 2.0.3 with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` set and `JOB_OBJECT_LIMIT_BREAKAWAY_OK`
intentionally unset (deny escape via `CREATE_BREAKAWAY_FROM_JOB`), assigned
after spawn via `child.raw_handle()` for bash + adapters. The
CreateProcess-suspended → Assign → Resume race is accepted (std/tokio discard
the thread handle, so a `CREATE_SUSPENDED` child cannot be `ResumeThread`'d; a
hard guarantee needs reimplementing spawn via `CreateProcessW`, disproportionate
for a non-security-boundary net) and documented as a residual. No resource caps
in Phase 15. `strict` degrades to L0 with a startup diagnostic.

**L0 precision.** The Phase-15 L0 deliverable adds `process_group(0)` and
Job-Object confinement to the bash spawn (`bash.rs:86-94`), which has only
`kill_on_drop(true)` today. The adapter spawn (`adapter_host.rs:180`) already
sets `process_group(0)` on Unix; L0 for adapters is mostly already there, with
the Job-Object assign being the D4 add. The D7 parenthetical "(process_group(0)
already present)" applies to the adapter spawn only, not to bash. The D8 framing
that "off still ships L0" is accurate as Phase-15 design intent, not as current
state.

**Fallback.** Default fail-open-with-diagnostic: a configured layer that cannot
engage proceeds at the engaged baseline. Opt-in `require = true` fails closed
(abort the turn) for CI/untrusted use, composing with T6. Permanent platform
gaps (Windows L1-L3, macOS L3, seccomp-unsupported architectures) emit a
one-time startup diagnostic, not a per-command one. New diagnostic codes
`CODE_SANDBOX_DEGRADED` and `CODE_SANDBOX_UNAVAILABLE` (stable `&'static str`
literals) with `SOURCE_SANDBOX`, produced from `opi-coding-agent`; the
structured `{layer, reason}` payload rides in `Diagnostic.details` as
`serde_json::Value`, consistent with the existing pattern (the unified
`Diagnostic` struct lives in `crates/opi-agent/src/diagnostic.rs:78` but is
constructed by-value from the consumer crate with a stable `code: &'static str`
and `details: Option<serde_json::Value>` split, so opi-agent's struct is
unchanged). No schema break. Example construction:

```rust
Diagnostic::new(
    CODE_SANDBOX_DEGRADED,
    SOURCE_SANDBOX,
    Some(serde_json::json!({ "layer": "landlock", "reason": "kernel < 5.13" })),
)
```

**Surfaces.** `strict` confines **bash only** (`bash.rs:86-94`, the primary
LLM-driven arbitrary-command surface). **Adapters get L0 only** — adapter
`strict`-confinement is deferred until a per-adapter capability-declaration
mechanism exists (T5/T6 territory; the sandbox *mechanism* is generic, so wiring
it to adapters is a later config step). In-process file tools (read/write/edit)
are not a T4 surface: they do I/O in-process and stay on `PathPolicy` (T5
revisits and confirms). `bedrock credential_process`
(`crates/opi-ai/src/bedrock/credentials.rs:330`, parsed exclusively from AWS
profile files `~/.aws/credentials` and `~/.aws/config`) is out of T4 scope — it
is config-driven, not LLM-driven. The only T6-reachable bedrock field from
`.opi/config.toml` is `providers.bedrock.profile`; see the T6 corrected
cross-ref for why `credential_process` itself is outside T6's threat model.

**Verified spawn-surface audit (`wf_d3373e28-329`, completeness COMPLETE).** opi
has no subagent/`Task` tool (8 built-in tools only; the nested-agent example at
`tests/sub_agent_example.rs` is explicitly non-core per `CLAUDE.md`). `opi-tui`
and `opi-agent` have zero spawn sites. There is no stdio-MCP-server launch; the
adapter `process-jsonl` protocol is the only external-process surface besides
bash. All three T4 surfaces are therefore exhaustive.

**Config schema.** A new `SandboxConfig` in `crates/opi-coding-agent/src/config.rs`
(sibling to `DefaultsConfig` at `config.rs:36`):

```rust
pub struct SandboxConfig {
    pub mode: SandboxMode,
    pub require: bool,
    pub fs: Option<bool>,
    pub network: Option<bool>,
    pub syscalls: Option<bool>,
}

pub enum SandboxMode {
    Off,
    Strict,
}
```

TOML form:

```toml
[sandbox]
mode = "strict"
require = false
fs = true
network = true
syscalls = true
```

`require` lives under `[sandbox]` (not as a top-level flag) because it modifies
the configured layer's degrade policy. CLI override: `--sandbox off|strict`
mirrors `--allow-mutating`; `--sandbox-require` toggles the degrade policy.
Both wire into `cli.rs` alongside `--allow-mutating`. `off` still ships L0 as an
always-on correctness baseline.

**Seccomp shape.** L2 is an INET-block with an AF_UNIX carve-out: `socket()`/
`connect()`/`sendto()`/`recvfrom()`/`accept()`/`bind()` are arg-filtered on
domain; AF_UNIX local IPC survives, INET/INET6/NETLINK are denied. L3 is a
danger-blocklist, not a strict allowlist: deny kernel-handle escapes
(`open_by_handle_at`, `bpf`, `perf_event_open`, `ptrace`, `kexec*`, `reboot`,
`init_module`/`finit_module`/`delete_module`, `swapon`/`swapoff`, `iopl`/
`ioperm`, `acct`, `settimeofday`); `clone`/`unshare` are allowed (user-namespace
unshare is more confinement, not escape).

**Scope boundary.** T4 delivers the OS-native subprocess sandbox for bash (L0
baseline + opt-in L1/L2/L3 `strict`) across Linux/macOS/Windows, the `[sandbox]`
config + `--sandbox` flag, the fallback diagnostics, and the L0
`process_group(0)` / Job-Object correctness fix for bash.

### T5 — Per-tool Operations seam

**Contract + layering.** Operations is a pure FS/exec backend: it receives an
already-resolved path, performs the op, and does no path resolution and no
confinement. `PathPolicy` stays the pre-flight (expand → canonicalize →
symlink-escape → workspace-containment → resolved path; the symlink-escape check
lives at `crates/opi-coding-agent/src/tool/mod.rs:83`) and runs **first**;
Operations layers **below** it, replacing the raw `tokio::fs::*`/`Command::new`
calls. Operations neither replaces `PathPolicy` nor sits beside it; it replaces
the *op backend under* `PathPolicy`. Confinement is authoritative for the
**local** backend (PathPolicy + local Ops share the same FS); for a **remote**
backend, opi does only lexical normalization and the remote backend owns
workspace confinement (pi parity — pi's Operations has no confinement at all).

**Location.** The `FileOperations` and `BashOperations` traits, the local
impls, and the T4-sandbox wrapping all live in `opi-coding-agent`
(`src/tool/`, alongside `PathPolicy` and the concrete tools). `opi-agent`'s
`Tool` trait is unchanged and unaware of Operations. This corrects the original
ticket premise ("defined in opi-agent like PathPolicy") — `PathPolicy` is itself
in `opi-coding-agent`, not `opi-agent`. The deciding principle: a trait lives in
`opi-agent` only if `opi-agent`'s runtime invokes it. `agent_loop` calls
`AgentHooks`/`CompactionHooks` (so those are in `opi-agent`) but never
Operations or `PathPolicy` (both internal to how concrete tools execute).

**Tools in scope.** Phase-15 Operations cover **read/write/edit/bash**. Nav
tools (grep/find/ls/glob) stay local-walk: their `ignore`-crate `WalkBuilder`
(`nav_walk_builder` at `tool/mod.rs:199`) traverses the local FS directly and
cannot be cleanly redirected to a remote backend without a remote-aware walker
redesign that loses gitignore semantics. pi's nav Operations are narrow and
never overridden in practice (ssh/sandbox/gondolin examples override only
Read/Write/Edit/Bash). Nav Operations are deferred to fog. `grep`/`find`/`ls`/
`glob` constructors are therefore unchanged; `build_tools` widens for exactly 4
of the 8 built-in tools.

**Granularity.** Two grouped traits. `FileOperations` (read+write+edit) and
`BashOperations` (bash exec). A remote FS backend delegates the FS as a unit,
not per-tool; bash is process-exec, orthogonal. Rejected: one-trait-per-tool
(pi's model — duplicates `read_file`/`write_file` across Read/Edit and
Write/Edit; backends are full-FS anyway) and a single fat trait (interface
segregation; bash-only backends such as the T4 sandbox should not implement file
methods).

```rust
pub trait FileOperations: Send + Sync {
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, FsOpError>;
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), FsOpError>;
    async fn mkdir(&self, path: &Path, recursive: bool) -> Result<(), FsOpError>;
    async fn metadata(&self, path: &Path) -> Result<OpMetadata, FsOpError>;
    async fn access(&self, path: &Path, mode: AccessMode) -> Result<(), FsOpError>;
}

pub trait BashOperations: Send + Sync {
    async fn exec(&self, command: BashRequest) -> Result<BashResult, BashOpError>;
}

pub struct BashRequest {
    pub command: String,
    pub cwd: PathBuf,
    pub timeout: Duration,
    pub signal: CancellationToken,
    pub env: Vec<(String, String)>,
}

pub struct BashResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub diagnostics: Vec<ToolDiagnostic>,
}
```

`FsOpError`/`BashOpError`/`OpMetadata`/`AccessMode`/`ToolDiagnostic` are
defined alongside the traits. The atomic write (temp+rename) is an impl detail
of the local `FileOperations`; the bounded `StreamCapture` at `bash.rs:119`
stays impl-internal to the local `BashOperations` (the trait returns drained
bytes). `CancellationToken` is part of the request struct so the caller threads
the existing agent-loop cancellation token; the local impl races it against
`tokio::time::sleep(timeout)`.

**Injection.** Constructor injection — `Arc<dyn FileOperations>` flows to
read/write/edit; `Arc<dyn BashOperations>` flows to bash; `build_tools`
(`harness.rs:762`) constructs the local impls internally and threads them to the
tool constructors. `Arc` is chosen for shared local impls, cheap clones, and
lifetime parity with the tool. Phase 15 ships the seam + local impls only — no
SSH/container backends (future/examples, pi parity). The seam's value is
enabling future remote delegation and giving T4's sandbox a home. Testable via
mock impls mirroring `opi-ai::test_support::MockProvider`.

**Sandbox composition (T4 integration).** The T4 sandbox lives inside the local
`BashOperations::exec`, driven by `[sandbox]` config; `pre_exec`/
`sandbox-exec`/Job-Object are applied to the `Command` before spawn. The bash
spawn therefore moves out of `bash.rs:86-94` into the local `BashOperations`
impl: `BashTool::execute` becomes a thin caller of the injected
`Arc<dyn BashOperations>`, and the `Command`-construction + spawn block at
`bash.rs:86-94` is deleted from the tool and lives wholly in the impl.
`FileOperations` is unsandboxed — file tools stay `PathPolicy`-guarded, since
T4 confines only the bash subprocess tree. Remote or custom `BashOperations`
impls bypass the sandbox and own their own confinement. The Operations seam is
exactly where "local + T4-sandboxed" plugs in distinct from "remote" — the
separation the contract layering established. This matches pi's model: the
sandbox IS a `BashOperations` impl, not a wrapper.

**Scope boundary.** T5 delivers the Operations seam — `FileOperations` +
`BashOperations` traits in `opi-coding-agent`, local impls, `Arc<dyn>`
constructor injection in `build_tools`, `PathPolicy` confirmed as the pre-flight
above Operations, and the T4 sandbox relocated into local `BashOperations::exec`.

### T6 — Project-trust gate

**Scope.** Trust gates **loading** of project-local resources, including (the
pi-deviating key point) **project-local adapter declarations** — an untrusted
project's `.opi/packages.toml` adapter entries do not load, so their native
child processes do not start. This closes the native-child-process blast-radius
gap T4 flagged: opi's adapters are native children
(`crates/opi-coding-agent/src/adapter_host.rs:166-187`, with the spawn coupled
to declaration-load via `runtime_packages.rs:21-54` → `adapter_extension.rs:810-878`),
higher blast radius than pi's in-process jiti. The gate is at declaration-load,
not at process spawn, because for opi the two are coupled in a way they are not
for pi. The principle "gate loading, not execution" is preserved. User-global
resources, including user-global adapters, are ungated (user scope, always
trusted). Tool execution (bash/read/write/edit/grep/find/ls/glob) is ungated —
pi parity: once in a session, tools run with the user's privileges.

**Gated list (untrusted project ⇒ skipped):** `.opi/config.toml` (skipped
entirely at config-merge time, the project-TomlDefaults layer at `config.rs:288`
— not loaded-then-filtered — so `providers.bedrock.profile` and every other
project-config key are inert together); `.opi/{skills,fragments,themes}`;
`.opi/extensions` (D6 anchor — not loaded until after trust resolves, so a
project extension cannot influence its own trust decision; see the Extension
hook section); project `AGENTS.md`/`CLAUDE.md` (D2 deviation); project
`.opi/packages.toml` adapter declarations (D1).

**Not gated (always loaded):** user-global config/extensions/skills/fragments/
themes/adapters/context-files; CLI `-e` extensions; all tool execution. opi has
no project-config auto-install, so no auto-install gate is needed (simpler than
pi).

**Gating seam.** `CodingHarness::build` receives a resolved `TrustDecision`
before `discover_resources` consumes project layers; a new
`trust_decision: TrustDecision` field on `BuildOptions` is produced by the
resolver at session-start. When `Untrusted`, `Self::discover_resources` skips
the project layer (the `config.rs:288` project-TomlDefaults merge,
`runtime_packages.rs:21-54` project `.opi/packages.toml`, `harness.rs:2041/2057`
project skills/fragments). This same seam is the Phase-16 T7 dependency: T6's
trust gate must filter `layers.skills`/`layers.fragments` at
`discover_resources` time so an untrusted project's skills/fragments cannot
resolve via `/skill:`/`/fragment:`.

**Context-file deviation.** Project-local `AGENTS.md`/`CLAUDE.md` are not
auto-injected into the system prompt for untrusted projects — they are a direct
prompt-injection channel. This deliberately deviates from pi, which loads them
regardless. User-global context files still load. The files remain readable via
the `read` tool (this is tool execution, ungated).

**Store.** pi-style `ProjectTrustStore` at `{user_config_dir}/trust.json` — flat
`Map<canonical_path, bool>` (true/false only; absent entry = no decision = ask),
ancestor walk (`Path::parent()`, nearest true/false wins), `std::fs::canonicalize`
realpath key, `fs4` sidecar `trust.json.lock` (consistency with the Phase 14 T1
credential lock; acquire-then-reread). The store is consulted once at
session-start in `opi-coding-agent`, before `discover_resources` consumes
project layers at `harness.rs:775/1949`, so trust resolution precedes any
project-resource load. The concrete struct lives in `opi-coding-agent` — no
`opi-agent` trait, because the store is consulted at session-start in
`opi-coding-agent`, not by `opi-agent`'s runtime, per the T5 deciding
principle. No schema version or metadata in Phase 15.

```rust
pub struct ProjectTrustStore {
    entries: BTreeMap<PathBuf, bool>,
    path: PathBuf,
}

pub enum TrustDecision {
    Trusted,
    Untrusted,
    Undecided,
}

impl ProjectTrustStore {
    pub fn load(user_config_dir: &Path) -> Result<Self, TrustError>;
    pub fn decide(&self, project_path: &Path) -> TrustDecision;
    pub fn record(&self, path: &Path, trusted: bool) -> Result<(), TrustError>;
}
```

On-disk JSON shape (`{user_config_dir}/opi/trust.json`):

```json
{
  "/home/user/project-a": true,
  "/home/user/project-b": false
}
```

Lock path: `{user_config_dir}/opi/trust.json.lock`.

**Ask UX.** Full TUI trust prompt — Trust / Trust-parent / Trust-session / Deny /
Deny-session, mirroring pi — backed by a new `AppState::AwaitingTrust` variant
(the current `AppState` enum is exactly `{Idle, Thinking, Streaming, ToolExecuting}`
at `crates/opi-tui/src/lib.rs:138-142`). The variant lives in `opi-tui`:

```rust
pub enum AppState {
    Idle,
    Thinking,
    Streaming,
    ToolExecuting,
    AwaitingTrust(AwaitingTrustState),
}

pub struct AwaitingTrustState {
    pub project_path: PathBuf,
    pub response_tx: oneshot::Sender<TrustChoice>,
}

pub enum TrustChoice {
    Trust,
    TrustParent,
    TrustSession,
    Deny,
    DenySession,
}
```

The prompt widget renders in `opi-tui`; the trust resolver in `opi-coding-agent`
awaits the oneshot. The transport mirrors the T13 `PendingUiRequest`+oneshot
pattern (interactive via `Arc<Mutex<TuiState>>`, RPC via a dedicated
`AgentEvent::UiRequest`/`SdkCommand::ui_response` pair) but is a dedicated
field on `TuiState` polled by the event loop, since trust resolution is
session-start, not mid-tool. The prompt fires on first cd into a project with
**trust-requiring resources** (mirrors pi's `hasTrustRequiringProjectResources`
— a bare `.opi` dir does not count; only `.opi` config/skills/fragments/themes/
extensions/project packages/project `AGENTS.md` trigger the prompt).
Non-interactive and RPC modes cannot ask, so they default untrusted (skip
project resources) unless `--trust` or `default_project_trust=always` is set
(mirrors pi's "ask + no-UI → false"). When a project has no trust-requiring
resources, no gate fires in any mode (interactive, non-interactive, RPC) — the
absence of triggering resources is the no-op path.

**Overrides.** `--trust` / `--no-trust` flags (clearer than pi's `--approve`);
`[defaults] default_project_trust = "ask"|"always"|"never"`, global-only (the
setting must load before trust resolution, and project `.opi/config.toml` is
itself gated or not-yet-loaded at that point; default `ask`); a `/trust` slash
command to change trust mid-session. No env var in Phase 15.

Precedence chain (pi-style): CLI → `project_trust` hook (extension event) →
store → default → ask. The `project_trust` hook sits at precedence position 2;
if it returns `Trust` or `Deny` that is authoritative (first yes/no wins), and
an `Undecided` return falls through to store/default/ask. In non-interactive/RPC
modes the chain terminates at default-untrusted since ask is unreachable without
a TUI. `/trust` with no argument opens a picker
(Trust/Trust-parent/Trust-session/Deny/Deny-session mirroring the prompt);
`/trust <choice>` applies and persists to `trust.json` (except Trust-session/
Deny-session which are session-only and not persisted).

**Extension hook.** A `project_trust` extension hook mirrors pi — only
user-global and CLI `-e` resolvers may own the decision. This is structurally
enforced: project `.opi/extensions` are not loaded until after trust resolves,
so a project extension cannot influence its own trust decision (D6 anchor). The
hook exposes a minimal pre-trust UI surface (`select`/`confirm`/`input`/`notify`
— no full TUI access), preventing privilege escalation before consent. First
yes/no wins; `Undecided` falls through to the store/default/ask path. The trait
lives in `opi-coding-agent` (not on `opi-agent`'s `Extension` trait —
`opi-agent`'s runtime never invokes it; trust resolution is session-start,
coding-agent-local). It is the ADR-019-consistent seam for custom trust policy
(signed manifests, team allowlists, org policy).

```rust
pub trait ProjectTrustResolver: Send + Sync {
    async fn resolve(&self, ctx: TrustContext, ui: &dyn PreTrustUi) -> TrustVote;
}

pub struct TrustContext {
    pub project_path: PathBuf,
    pub triggering_resources: Vec<TrustResource>,
}

pub enum TrustVote {
    Trust,
    Deny,
    Undecided,
}

pub trait PreTrustUi: Send + Sync {
    async fn select(&self, prompt: &str, options: &[&str]) -> Result<usize, UiError>;
    async fn confirm(&self, prompt: &str) -> Result<bool, UiError>;
    async fn input(&self, prompt: &str) -> Result<String, UiError>;
    async fn notify(&self, msg: &str);
}
```

User-global and CLI `-e` `ProjectTrustResolver` registrations are collected at
harness build, before the resolver runs.

**T4 credential_process cross-ref — corrected.** T4 flagged
`bedrock credential_process` as "config-trust for T6". The accurate framing:
`credential_process` is not a field in opi's `.opi/config.toml`
`[providers.bedrock]` schema
(`crates/opi-coding-agent/src/config.rs:124-141` — the schema carries `profile`,
`region`, `access_key_id`, `secret_access_key_env`, `session_token_env`,
`base_url`, `proxy`, with no `credential_process` field). It is parsed
exclusively by `opi-ai`'s bedrock credentials loader from AWS profile files
(`~/.aws/credentials`, `~/.aws/config`) at
`crates/opi-ai/src/bedrock/credentials.rs:178-182,212,241-242,307,330`. The
only T6-reachable bedrock field from `.opi/config.toml` is
`providers.bedrock.profile` (an AWS profile NAME selector, `config.rs:136`).
The real T4 vector that T6 closes is narrower than "credential_process never
executed": an untrusted project cannot select an arbitrary AWS profile name via
`providers.bedrock.profile`, because the project `.opi/config.toml` is skipped
entirely (not loaded-then-filtered) for an untrusted project. The
`credential_process` directive itself lives in the user-trusted `~/.aws/config`
surface under whatever profile name the user already created, and opi-ai reads
it directly from the AWS file — outside `.opi/config.toml`'s gating surface and
outside T6's threat model.

**Scope boundary.** T6 delivers the project-trust gate — `ProjectTrustStore`,
the gated/not-gated resource lists, the TUI ask prompt + `AppState::AwaitingTrust`,
`--trust`/`--no-trust` + `default_project_trust` + `/trust`, the `project_trust`
extension hook, and adapter-declaration gating.

## Sequencing

T5 is the substrate for T4: the bash spawn must move from `bash.rs:86-94` into
the local `BashOperations::exec` before the sandbox can be applied at the
structurally correct attach point. T4 therefore follows T5 (or the two land
together with T5's `BashOperations` trait and T4's sandbox wrapping in the same
implementation step). T6 is independent of T4 and T5 — it gates resource
*loading* at session-start, not tool execution — and can proceed in parallel.
Phase 15 has no hard dependency on Phase 14 (auth) or Phase 16 (agent
intelligence). Phase 16 T7 (skills/templates runtime) depends on T6: T6's trust
gate must filter `layers.skills`/`layers.fragments` at `discover_resources` time
(`harness.rs:775/1949/2041/2057`) so an untrusted project's skills/fragments
cannot resolve via `/skill:`/`/fragment:`.

## Cross-ticket interactions

- **T4 sandbox lives inside local `BashOperations::exec` (T5 D6).** The bash
  spawn moves out of `bash.rs` into the local `BashOperations` impl;
  `BashTool::execute` becomes a thin caller of the injected
  `Arc<dyn BashOperations>`, and the `Command`-construction + spawn block at
  `bash.rs:86-94` is deleted from the tool. Sandbox filters are applied to the
  `Command` before spawn there. `FileOperations` is unsandboxed: file tools stay
  `PathPolicy`-guarded, and T4 confines only the bash subprocess tree. Remote
  and custom `BashOperations` impls bypass the sandbox and own their confinement.
- **T6 gates loading of project-local resources including project-local adapter
  declarations.** Untrusted project `.opi/packages.toml` adapter entries do not
  load, so their native child processes do not start. This closes the
  native-child-process blast-radius gap T4 flagged. User-global resources and
  tool execution are ungated (pi parity on execution).
- **T6 deviates from pi on context files.** Project-local `AGENTS.md`/
  `CLAUDE.md` are not auto-injected for untrusted projects (prompt-injection
  channel). User-global context files still load. The files remain readable via
  the `read` tool.
- **T4 credential_process cross-ref resolved by T6 (with correction).** An
  untrusted project cannot select an arbitrary AWS profile name via
  `providers.bedrock.profile`, because the project `.opi/config.toml` is skipped
  entirely (not loaded-then-filtered). The `credential_process` directive itself
  lives in the user-trusted `~/.aws/config` surface, outside T6's threat model.

## Residuals / follow-ups

- **Adapter `strict`-confinement** (T4 D7) — fog; depends on a per-adapter
  capability-declaration schema and on T5/T6 landing. The sandbox *mechanism*
  is generic; wiring it to adapters is a later config step.
- **Windows hard no-escape via direct `CreateProcessW`** (T4 D4 option b) —
  deferred follow-up, not Phase 15. The `CreateProcess-suspended → Assign →
  Resume` race is accepted and documented for the L0 Job-Object assign.
- **Nav-tool Operations** (grep/find/ls/glob) — fog; the `ignore`-crate
  `WalkBuilder` cannot be cleanly redirected to a remote backend without losing
  gitignore semantics.
- **Remote backends** (SSH/container) — future/examples, not Phase 15; the seam
  ships with local impls only.
- **`Operations` method signatures** (read windowing for large files, streaming
  reads) — design-doc-level detail for the implementation phase; the Phase-15
  contract names the base signatures, and impls extend with windowing as needed.
- **`trust.json` schema version / metadata** — deferred; the v1 flat
  `Map<canonical_path, bool>` is sufficient.
- **§15 roadmap rewrite.** Batched with the Phase 16 design doc landing. Editing
  `opi-spec.md` triggers the phase4 + phase6 specification-hash ledger re-sync
  plus the live-ledger raw-hash re-sync (per project convention; see memory
  `spec-edit-breaks-phase4-ledger`). This is a separate, guard-affecting step,
  not part of authoring this design doc. The known §15 drift (Phase 14 TUI
  polish, `opi-spec.md:1595-1604`, defers branch-summary to "Phase 14") is
  reconciled by that rewrite, not by this doc.

## Out of scope (cross-ref map)

- opi-self confinement (external runner like Docker/Gondolin) — explicitly out
  of T4's threat model; untrusted code belongs in a container or VM.
- Resource caps on Windows Job Objects (CPU/memory limits) — not in Phase 15.
- Full `seccomp` strict-allowlist (T4 L3 is a danger-blocklist only).
- Process-config auto-install gating (opi has no auto-install; simpler than pi).
- Provider/auth changes (Phase 14), agent intelligence changes (Phase 16),
  extension-surface changes (Phase 17).