# Pluggable Extension Command Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Opi's built-in native command sandbox with a minimal-runtime extension host, a policy-bounded `command.execute` router, the shared `opi-protocol::execution::v1` contract, and an independently usable `opi-sandbox` SDK/CLI/package.

**Architecture:** The default `fixed + local` path constructs `LocalBashOperations` directly and does not scan packages, allocate a router or permission state, or start an extension. Non-default routing resolves only enabled, trusted global contributions and routes the stable `bash` tool through `RoutedBashOperations`; process adapters use a one-shot product-neutral protocol. `opi-sandbox` is a separate library/binary that depends on `opi-protocol`, owns native restriction and invocation state, and can be validated or reused without Opi.

**Tech Stack:** Rust 2024, Tokio, serde/serde_json, schemars/jsonschema, clap, SHA-256, TOML, ratatui/crossterm, existing `BashOperations` and package store, Unix process groups, Windows Job Objects, Linux Landlock/seccompiler, macOS `sandbox-exec`, GitHub Actions.

---

## Binding Sources

- Canonical:
  `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
- Supporting rationale:
  `docs/superpowers/specs/2026-07-28-pluggable-extension-architecture-design.md`
- `CONTEXT.md`
- `docs/opi-spec.md` and `docs/opi-spec.zh.md`, after Task 16.1 aligns them
- Historical implementation evidence only:
  `docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md`
- Source comparison:
  `docs/research/2026-07-27-sandbox-comparison.md`

If this plan and an approved design disagree, stop for design review. Do not
silently reinterpret trust, enablement, permission, fallback, package scope,
protocol, platform guarantees, or the Minimal Runtime claim.

## Execution Preconditions

1. Use `opi-implement plan` to bind the canonical Phase 16 source, reconcile
   `.opi-impl-state.json`, and obtain task-graph approval before code work.
2. Use an isolated worktree if the implementation baseline contains unrelated
   changes. Never copy, stash, reset, or discard the current dirty workspace.
3. Preserve the Phase 15 L0 fixes: descendant cleanup after clean parent exit,
   bounded output drain, dropped-future cleanup, typed supervision failure, and
   Windows Job Object termination.
4. Every executable task uses red-green-refactor and the verification gates in
   `.claude/skills/opi-implement/skill.md`.
5. The commit blocks below are instructions for an authorized
   `opi-implement` Phase E only. Do not commit merely because this plan is
   approved. Each task still requires the Phase B commit gate.
6. Do not bump the workspace version as part of this phase. New workspace
   dependencies use the current lockstep `0.7.1` version; the release workflow
   later changes all crate versions together.

## Scope Boundaries

Phase 16 implements only:

- Capability `command.execute`;
- built-in adapter `local`;
- external adapter `opi-sandbox`;
- stable tool `bash`;
- strategies `fixed`, ordered run-mode `rules`, and `model`;
- decisions `deny`, `ask`, and `allow`;
- executable package trust/enablement required by this vertical slice;
- `opi-protocol::execution::v1`;
- standalone `opi-sandbox` SDK, CLI, protocol backend, and package.

Do not implement:

- dynamic Rust libraries;
- Docker, VM, SSH, remote, or composed adapters;
- command-text rules or model risk labels;
- project-local executable contributions;
- publisher signatures, registries, or automatic updates;
- a generalized independent-tool contribution redesign;
- Windows AppContainer;
- target stdin in `command-execution-jsonl-v1`;
- new session/RPC/NDJSON protocols merely to carry execution diagnostics.

The existing package-level `[adapter]` process extension remains supported only
as a legacy global executable contribution. It becomes trust/enablement gated
and is rejected at project scope. New `[[contributions.tools]]` support is not
implemented in this phase.

## File and Module Structure

### Shared protocol crate

```text
crates/opi-protocol/
  Cargo.toml
  src/lib.rs
  src/execution/mod.rs
  src/execution/v1/mod.rs
  src/execution/v1/codec.rs
  src/execution/v1/frames.rs
  src/execution/v1/schema.rs
  schemas/command-execution-jsonl-v1.schema.json
  fixtures/valid/
  fixtures/invalid/
  tests/execution_v1_contract.rs
  tests/execution_v1_schema.rs
```

This crate contains serde types, lossless OS-string/path encoding, bounded
JSONL codecs, schema generation, and fixtures. It has no Opi package, config,
provider, session, process-launch, routing, permission, or sandbox dependency.

### Opi product host

```text
crates/opi-coding-agent/src/execution/
  mod.rs
  failure.rs
  permission.rs
  protocol_host.rs
  registry.rs
  router.rs
  runtime.rs

crates/opi-coding-agent/src/package_activation.rs

crates/opi-coding-agent/tests/
  execution_config.rs
  execution_failures.rs
  execution_minimal_runtime.rs
  execution_package_lifecycle.rs
  execution_permission.rs
  execution_protocol_host.rs
  execution_routing.rs
  execution_product.rs
  fixtures/execution_backend_mock.rs
```

`fixtures/execution_backend_mock.rs` is a harnessless binary declared with
`test = false`, `bench = false`, and
`required-features = ["execution-backend-test-fixture"]`. It supports
`normal`, `malformed`, `crash-before-ready`, `stall`, `duplicate-terminal`,
`stdout-contamination`, and `child-tree` scenarios.

### Independent sandbox crate

```text
crates/opi-sandbox/
  Cargo.toml
  src/lib.rs
  src/backend.rs
  src/cli.rs
  src/helper.rs
  src/main.rs
  src/platform/mod.rs
  src/platform/linux.rs
  src/platform/macos.rs
  src/platform/windows.rs
  src/policy.rs
  src/process_tree.rs
  src/runner.rs
  tests/cli_contract.rs
  tests/protocol_conformance.rs
  tests/sdk_contract.rs
  tests/linux_policy.rs
  tests/macos_policy.rs
  tests/fixtures/protocol_client.py
```

`opi-sandbox` depends on `opi-protocol`, never on `opi-agent` or
`opi-coding-agent`. It contains both the SDK and the `opi-sandbox` binary.

### TUI permission prompt

```text
crates/opi-tui/src/permission_prompt.rs
crates/opi-coding-agent/src/interactive.rs
crates/opi-coding-agent/tests/interactive_permission.rs
```

The router sends an interactive permission request over an in-memory channel.
The TUI returns exactly one `AllowOnce`, `AllowSession`, or `Deny` response.
Session grants live only in the runtime `PermissionManager`; they are never
serialized into sessions or package/config files.

### Packaging and standalone acceptance

```text
packaging/opi-sandbox/package.toml.template
scripts/package-opi-sandbox.sh
scripts/package-opi-sandbox.ps1
scripts/opi-sandbox-smoke.sh
scripts/opi-sandbox-smoke.ps1
```

The smoke scripts accept an explicit binary path, invoke that executable
directly, and never invoke `opi`.

## Fixed Contracts

### Default and routed configuration

```toml
[execution]
strategy = "fixed"
backend = "local"
```

```toml
[execution]
strategy = "rules"

[[execution.rules]]
modes = ["non-interactive", "rpc"]
backend = "opi-sandbox"

[[execution.rules]]
backend = "local"

[execution.permissions]
local = "allow"
"opi-sandbox" = "ask"
```

Rules are declaration-ordered. Exactly one catch-all rule is required and must
be last. Phase 16 rule inputs are only `interactive`, `non-interactive`, and
`rpc`; command text is never inspected.

For `strategy = "model"`, `bash` gains a required `backend` enum containing
only eligible `allow` or `ask` adapters. `deny` adapters are absent. Default,
fixed, and rules schemas remain byte-for-byte equal to the current schema.

### Executable package lifecycle

```text
package add -> installed, untrusted, disabled
package enable -> display identity/hash/contributions -> TTY trust confirmation
package disable -> disabled, unchanged matching trust retained
package remove -> package, enablement, and trust removed
artifact drift -> trust mismatch -> fail closed
```

No non-TTY path infers first trust. No model path installs, trusts, enables,
disables, grants, or edits policy.

The global state file is `package-state.toml`. Its records bind:

```rust
pub struct PackageActivationRecord {
    pub name: String,
    pub identity_kind: String,
    pub identity_value: String,
    pub manifest_sha256: String,
    pub trust_fingerprint: String,
    pub enabled: bool,
}
```

`trust_fingerprint` is SHA-256 over deterministic serialized trust material:
package identity, manifest hash, and sorted executable contribution fields
including capability/id/transport/command/args/protocol/target/executable hash.

### Declarative execution adapter contribution

```toml
[[contributions.adapters]]
capability = "command.execute"
id = "opi-sandbox"
transport = "process-jsonl"
command = "bin/opi-sandbox"
args = ["backend", "--stdio"]
protocol = "command-execution-jsonl-v1"
target = "x86_64-unknown-linux-gnu"
sha256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
handshake_timeout_ms = 5000
```

The command must be relative, canonicalized, contained by the package, and a
regular executable file. Capability, adapter id, package version, Opi version,
target, protocol, SHA-256, duplicate identity, and reserved identity are hard
gates. Discovery never starts the executable.

### Product-neutral protocol

```text
initialize -> ready -> execute -> accepted -> started
           -> stdout | stderr | diagnostic*
           -> completed -> clean backend exit
```

- Every frame carries one host request id.
- Command/configuration data is sent only after a valid `ready`.
- Command output chunks are base64.
- One process handles one command.
- Protocol stdin is never target stdin.
- The deadline covers handshake, setup, execution, output drain, and cleanup.
- No malformed/out-of-order/oversized frame falls back to `local`.

### Stable execution failures

```rust
pub struct ExtensionFailure {
    pub code: ExtensionFailureCode,
    pub phase: ExtensionPhase,
    pub capability: String,
    pub adapter: Option<String>,
    pub retryable: bool,
    pub user_action_required: bool,
    pub remediation: Vec<String>,
    pub redacted_details: serde_json::Value,
}
```

Stable codes:

```text
package_not_installed
package_untrusted
contribution_disabled
policy_denied
permission_required
permission_denied
no_eligible_adapter
adapter_not_selected
adapter_unavailable
protocol_incompatible
protocol_violation
execution_failed
execution_timed_out
cleanup_unconfirmed
```

Public diagnostics never contain command text, environment values, credentials,
unnecessary absolute paths, PIDs, or raw backend stderr.

### Standalone CLI

```text
opi-sandbox run --workspace <PATH> --profile workspace-write \
  --network deny|allow -- <PROGRAM> [ARGS...]
opi-sandbox doctor --json
opi-sandbox backend --stdio
```

Direct `run` inherits stdin and passes target stdout/stderr bytes through.
Exit mapping is:

```text
target normal exit -> target code
usage -> 2
timeout -> 124
pre-start platform/policy/setup failure -> 125
interactive cancellation -> 130
Unix signal -> 128 + signal
```

`doctor --json` returns zero after a completed diagnostic even when
`supported=false`, with stable fields:
`schema_version`, `supported`, `target`, `mechanisms`, `profiles`, and
`limitations`.

## Candidate Task Graph

| Task | Depends on | Checkpoint |
|---|---|---|
| 16.1 Verify the canonical source and reconcile the ledger | Phase 15 exit | normative graph approved |
| 16.2 Pin L0 supervision and remove sandbox naming from L0 | 16.1 | local and host lifecycle green |
| 16.3 Add `opi-protocol::execution::v1` | 16.1 | neutral types/schema/fixtures green |
| 16.4 Parse and hard-gate executable contributions | 16.3 | declarative metadata, no execution |
| 16.5 Add Package Trust and enable/disable lifecycle | 16.4 | installed/trusted/enabled independent |
| 16.6 Add execution config, failures, routing, and permission policy | 16.5 | fixed/rules/model decisions deterministic |
| 16.7 Implement one-shot protocol host | 16.2, 16.3, 16.6 | fake adapter lifecycle/cancel green |
| 16.8 Build routed operations and Minimal Runtime split | 16.5, 16.6, 16.7 | direct local fast path or routed host |
| 16.9 Wire dynamic `bash` schema and all headless surfaces | 16.8 | model enum and structured failures |
| 16.10 Add interactive permission broker and TUI | 16.9 | once/session/deny, memory only |
| 16.11 Build standalone sandbox SDK/CLI and smoke scripts | 16.2, 16.3 | direct isolated CLI green without Opi |
| 16.12 Add atomic helper/start gate and protocol backend | 16.11 | no target before `started` |
| 16.13 Port Linux native restriction | 16.12 | direct native Linux contract |
| 16.14 Port macOS restriction and truthful Windows posture | 16.13 | macOS direct contract; Windows refuses |
| 16.15 Package release artifacts and run extracted smoke | 16.5, 16.13, 16.14 | installable Linux/macOS archives |
| 16.16 Remove core native sandbox, run product vertical slices, update docs | all previous | Phase 16 acceptance and Phase F |

Tasks 16.2 and 16.3 may run in parallel after graph approval. Task 16.11 may
run alongside 16.5 or 16.6 after its own dependencies pass. Tasks 16.5 then
16.6 then 16.7 are serialized because they overlap `cli.rs` or
`execution/mod.rs`; tasks 16.13 then 16.14 are serialized because they overlap
platform dispatch, helper, smoke-script, and CI files. The parent must
serialize every shared path, including `Cargo.toml`, `Cargo.lock`, `lib.rs`,
workflow, docs, and ledger edits.

## Acceptance Scenario Ownership

| Scenario | Owner | Production evidence |
|---|---:|---|
| SC16-01 default Minimal Runtime | 16.8, 16.16 | startup ignores invalid package store; unchanged schema/results |
| SC16-02 local L0 lifecycle | 16.2, 16.16 | local clean-exit/timeout/cancel/drop/wait paths |
| SC16-03 package gates | 16.4, 16.5 | add/enable/disable/remove/list/doctor and startup registry |
| SC16-04 fixed/rules/model routing | 16.6, 16.9 | production `bash` definitions and selected adapter |
| SC16-05 permission policy | 16.6, 16.9, 16.10 | allow/deny/ask in TUI, text, NDJSON, RPC |
| SC16-06 neutral protocol | 16.3, 16.7, 16.12 | shared fixtures, real host, mock process, and non-Rust fixture client |
| SC16-07 fail-closed external execution | 16.7, 16.9 | no fallback after selection |
| SC16-08 independent SDK | 16.11, 16.12 | repeated SDK invocations and cleanup |
| SC16-09 independent CLI | 16.11, 16.15 | installed/extracted binary smoke with no Opi |
| SC16-10 Linux restriction | 16.13 | direct CLI filesystem/network sentinels |
| SC16-11 macOS restriction | 16.14 | direct CLI filesystem/network sentinels |
| SC16-12 Windows truthfulness | 16.14, 16.15 | doctor unsupported; run refuses; no artifact |
| SC16-13 install-to-execute workflow | 16.15, 16.16 | archive -> add -> enable -> permission -> real `bash` turn |
| SC16-14 diagnostics/redaction | 16.6, 16.9, 16.16 | stable codes/remediation across all public surfaces |
| SC16-15 migration, dependency, and repository gates | 16.16 | no native sandbox in Opi; crate graph, paired docs, README/changelog, artifact-truth, six-target, workspace test/doc guards |

---

### Task 16.1: Verify the Canonical Source and Reconcile the Ledger

**Files:**

- Create: `crates/opi-coding-agent/tests/phase16_extension_docs.rs`
- Reference:
  `docs/superpowers/specs/2026-07-28-phase16-pluggable-extension-command-execution-design.md`
- Reference: `docs/superpowers/plans/2026-07-28-pluggable-extension-command-execution.md`

- [ ] **Step 1: Run the absent guard target**

```powershell
cargo test -p opi-coding-agent --test phase16_extension_docs -- --nocapture
```

Expected: FAIL because the guard test has not been created.

- [ ] **Step 2: Add the canonical-source and roadmap guard**

Create `phase16_extension_docs.rs` with exact source names, phase placement,
and boundary terms:

```rust
use std::fs;
use std::path::Path;

#[test]
fn phase16_registry_and_specs_bind_the_extension_vertical_slice() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let specs = root.join("docs/superpowers/specs");
    let skill = fs::read_to_string(root.join(".claude/skills/opi-implement/skill.md"))
        .expect("read opi-implement registry");
    let en = fs::read_to_string(root.join("docs/opi-spec.md")).expect("read English spec");
    let zh = fs::read_to_string(root.join("docs/opi-spec.zh.md")).expect("read Chinese spec");
    let phase16 = "2026-07-28-phase16-pluggable-extension-command-execution-design.md";
    let phase18 = "2026-07-11-phase18-agent-intelligence-design.md";
    let canonical_path = specs.join(phase16);
    let canonical = fs::read_to_string(&canonical_path).expect("read canonical Phase 16 spec");
    assert!(canonical_path.is_file());
    assert!(!specs.join("2026-07-11-phase16-agent-intelligence-design.md").exists());
    assert!(skill.contains(&format!("| 16 | `docs/superpowers/specs/{phase16}` |")));
    assert!(skill.contains(&format!("| 18 | `docs/superpowers/specs/{phase18}` |")));
    assert!(!skill.contains("2026-07-11-phase16-agent-intelligence-design.md"));
    for term in [
        "command.execute",
        "opi-protocol",
        "opi-sandbox",
        "Minimal Runtime",
        "Package Trust",
    ] {
        assert!(en.contains(term), "English spec missing {term}");
    }
    for term in ["command.execute", "opi-protocol", "opi-sandbox", "最小运行时", "包信任"] {
        assert!(zh.contains(term), "Chinese spec missing {term}");
    }
    assert!(en.contains("### Phase 17 - Benchmark and Regression Evaluation"));
    assert!(en.contains("### Phase 18 - Agent Intelligence"));
    assert!(en.contains("### Phase 20 - UI Productization"));
    assert!(zh.contains("### 第十七阶段 - Benchmark 与回归评估"));
    assert!(zh.contains("### 第十八阶段 - Agent Intelligence"));
    assert!(zh.contains("### 第二十阶段 - 界面产品化"));

    for contract in [
        "**Installed**",
        "**Trusted**",
        "**Enabled**",
        "**Selected**",
        "**Permitted**",
        "No router, permission, or protocol task is created.",
        "No selected external failure retries through `local`.",
        "The standalone CLI smoke suite described above is mandatory.",
        "Independent new-tool contributions remain a Phase 19 design topic.",
        "specification will be discussed only after Phase 16 exits",
    ] {
        assert!(canonical.contains(contract), "canonical spec missing {contract}");
    }

    let premature_phase17_specs: Vec<_> = fs::read_dir(&specs)
        .expect("read specs directory")
        .map(|entry| entry.expect("read spec entry").file_name())
        .filter(|name| {
            let name = name.to_string_lossy().to_ascii_lowercase();
            name.contains("phase17") || name.contains("benchmark")
        })
        .collect();
    assert!(
        premature_phase17_specs.is_empty(),
        "Phase 17 spec must wait for Phase 16 exit: {premature_phase17_specs:?}"
    );
}
```

- [ ] **Step 3: Run the completed docs guard**

```powershell
cargo test -p opi-coding-agent --test phase16_extension_docs -- --nocapture
```

Expected: PASS. The guard proves the canonical Phase 16 source and its
load-bearing contract, the absence of a premature Phase 17 benchmark spec, the
renamed Phase 18 Agent Intelligence source, and Phase 20 UI placement remain
synchronized.

- [ ] **Step 4: Commit the guard after authorization**

The registry and paired roadmap docs are planning prerequisites, not files
owned by this implementation task. After the task commit gate:

```powershell
git add crates/opi-coding-agent/tests/phase16_extension_docs.rs
# opi-implement Phase E subject:
# test(opi-coding-agent): guard Phase 16 source alignment
```

- [ ] **Step 5: Reconcile through the guarded plan path**

```powershell
opi-implement plan
```

Expected: the plan path binds the canonical Phase 16 source hash, creates or
reconciles tasks 16.2 through 16.16 and acceptance scenarios SC16-01 through
SC16-15, runs its plan-stage review, presents the `A.init.3` graph gate, and
pauses. It must not pull the Phase 17 benchmark or Phase 18 Agent Intelligence
tasks into Phase 16. Review task-owned paths and production call-site traces
before approval. Do not hand-edit `.opi-impl-state.json`.

- [ ] **Step 6: Approve and checkpoint the reconciled graph separately**

After the user approves the `A.init.3` graph, let the `opi-implement` guarded
flow atomically write and commit only `.opi-impl-state.json` as its own ledger
checkpoint. Do not combine the canonical ledger with the docs bootstrap or any
implementation commit.

---

### Task 16.2: Pin L0 Supervision and Separate It from Sandbox Policy

**Files:**

- Modify: `crates/opi-coding-agent/src/tool/process_tree.rs`
- Modify: `crates/opi-coding-agent/src/tool/operations.rs`
- Modify: `crates/opi-coding-agent/src/adapter_host.rs`
- Modify: `crates/opi-coding-agent/src/diagnostics.rs`
- Modify: `crates/opi-coding-agent/tests/sandbox_l0.rs`
- Modify: `crates/opi-coding-agent/tests/adapter_host.rs`
- Modify: `crates/opi-coding-agent/tests/bash_backend_diagnostics.rs`

- [ ] **Step 1: Add or retain failing lifecycle assertions**

Pin all of these observable paths:

```rust
#[test]
fn clean_parent_exit_still_terminates_descendants() {}

#[test]
fn timeout_terminates_the_process_tree_and_bounds_pipe_drain() {}

#[test]
fn cancellation_terminates_the_process_tree_and_bounds_pipe_drain() {}

#[test]
fn dropping_execution_future_terminates_the_process_tree() {}

#[test]
fn wait_failure_reports_supervision_without_leaking_command() {}

#[test]
fn adapter_host_uses_the_same_tree_cleanup_contract() {}
```

Use the existing sentinel helpers and platform-specific skips. Do not weaken a
passing Phase 15 assertion merely to manufacture a red test.

- [ ] **Step 2: Run the baseline tests**

```powershell
cargo test -p opi-coding-agent --test sandbox_l0 -- --nocapture
cargo test -p opi-coding-agent --test adapter_host -- --nocapture
cargo test -p opi-coding-agent --test bash_backend_diagnostics -- --nocapture
```

Expected: any lost Phase 15 remediation is RED. If all assertions already pass,
record the existing production call sites as folded evidence and avoid a no-op
refactor.

- [ ] **Step 3: Keep one policy-neutral process-tree API**

The retained API is:

```rust
pub fn configure_tree(command: &mut tokio::process::Command);

impl TreeGuard {
    pub fn attach_child(child_pid: Option<u32>) -> Result<Self, AttachError>;
    pub fn terminate(&mut self) -> TerminationOutcome;
}

impl Drop for TreeGuard {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}
```

Clean direct-child completion calls `terminate` before guard release. Timeout,
cancellation, dropped futures, wait failure, and host shutdown converge on the
same primitive.

- [ ] **Step 4: Rename L0 diagnostics without moving native policy yet**

Define:

```rust
pub const CODE_PROCESS_SUPERVISION_FAILED: &str =
    "opi.execution.supervision_failed";
pub const SOURCE_EXECUTION: &str = "execution";
```

The public details contain only `operation` and a stable `reason_code`. Do not
include command, environment, paths, PID, or raw OS errors.

- [ ] **Step 5: Re-run focused tests**

```powershell
cargo test -p opi-coding-agent --test sandbox_l0 -- --nocapture
cargo test -p opi-coding-agent --test adapter_host -- --nocapture
cargo test -p opi-coding-agent --test bash_backend_diagnostics -- --nocapture
cargo test -p opi-coding-agent --lib tool::operations -- --nocapture
```

Expected: PASS with local command behavior unchanged.

- [ ] **Step 6: Commit only if code or tests required changes**

After the task commit gate:

```powershell
git add crates/opi-coding-agent/src/tool/process_tree.rs
git add crates/opi-coding-agent/src/tool/operations.rs
git add crates/opi-coding-agent/src/adapter_host.rs
git add crates/opi-coding-agent/src/diagnostics.rs
git add crates/opi-coding-agent/tests/sandbox_l0.rs
git add crates/opi-coding-agent/tests/adapter_host.rs
git add crates/opi-coding-agent/tests/bash_backend_diagnostics.rs
# opi-implement Phase E subject:
# refactor(opi-coding-agent): isolate process supervision
```

If baseline code already satisfies the DoD, fold evidence during reconciliation
and create no task commit.

---

### Task 16.3: Add `opi-protocol::execution::v1`

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/opi-protocol/Cargo.toml`
- Create: `crates/opi-protocol/src/lib.rs`
- Create: `crates/opi-protocol/src/execution/mod.rs`
- Create: `crates/opi-protocol/src/execution/v1/mod.rs`
- Create: `crates/opi-protocol/src/execution/v1/codec.rs`
- Create: `crates/opi-protocol/src/execution/v1/frames.rs`
- Create: `crates/opi-protocol/src/execution/v1/schema.rs`
- Create:
  `crates/opi-protocol/schemas/command-execution-jsonl-v1.schema.json`
- Create: `crates/opi-protocol/fixtures/valid/normal.jsonl`
- Create: `crates/opi-protocol/fixtures/invalid/out-of-order.jsonl`
- Create: `crates/opi-protocol/fixtures/invalid/oversized.jsonl`
- Create: `crates/opi-protocol/tests/execution_v1_contract.rs`
- Create: `crates/opi-protocol/tests/execution_v1_schema.rs`

- [ ] **Step 1: Add the workspace member and dependency**

Use the current lockstep version:

```toml
[workspace]
members = [
    "crates/opi-ai",
    "crates/opi-agent",
    "crates/opi-coding-agent",
    "crates/opi-protocol",
    "crates/opi-tui",
]

[workspace.dependencies]
opi-protocol = { version = "0.7.1", path = "crates/opi-protocol" }
```

`crates/opi-protocol/Cargo.toml` uses `version.workspace = true`,
`edition.workspace = true`, `rust-version.workspace = true`, and workspace
serde/schemars/jsonschema/base64 dependencies.

- [ ] **Step 2: Write failing neutral-frame tests**

```rust
use opi_protocol::execution::v1::{
    BackendFrame, Guarantee, HostFrame, Placement, PROTOCOL_ID,
};

#[test]
fn initialize_contains_no_opi_product_fields() {
    let frame = HostFrame::Initialize {
        request_id: "r1".into(),
        protocols: vec![PROTOCOL_ID.into()],
        deadline_unix_ms: 1_800_000_000_000,
        config: serde_json::json!({"profile": "workspace-write"}),
    };
    let value = serde_json::to_value(frame).unwrap();
    assert_eq!(value["type"], "initialize");
    for forbidden in ["opi_version", "package", "session", "tool_call_id"] {
        assert!(value.get(forbidden).is_none(), "found {forbidden}");
    }
}

#[test]
fn started_reports_the_effective_contract() {
    let frame = BackendFrame::Started {
        request_id: "r1".into(),
        placement: Placement::Host,
        guarantee: Guarantee::Restricted,
        effective_policy: serde_json::json!({"network": "deny"}),
        limitations: vec!["host-reads-unrestricted".into()],
    };
    assert_eq!(serde_json::to_value(frame).unwrap()["guarantee"], "restricted");
}
```

- [ ] **Step 3: Run the new crate tests**

```powershell
cargo test -p opi-protocol --all-targets
```

Expected: FAIL because the `execution::v1` types and schema are absent.

- [ ] **Step 4: Implement closed v1 frames**

Export:

```rust
pub const PROTOCOL_ID: &str = "command-execution-jsonl-v1";

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostFrame {
    Initialize {
        request_id: String,
        protocols: Vec<String>,
        deadline_unix_ms: u64,
        config: serde_json::Value,
    },
    Execute {
        request_id: String,
        program: WirePath,
        args: Vec<WireOsString>,
        workspace: WirePath,
        cwd: WirePath,
        timeout_ms: u64,
        inherit_env: bool,
        env: Vec<WireEnv>,
    },
    Cancel {
        request_id: String,
        reason: CancelReason,
    },
}
```

```rust
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendFrame {
    Ready {
        request_id: String,
        protocol: String,
        implementation: String,
        implementation_version: String,
        target: String,
    },
    Accepted { request_id: String },
    Started {
        request_id: String,
        placement: Placement,
        guarantee: Guarantee,
        effective_policy: serde_json::Value,
        limitations: Vec<String>,
    },
    Stdout { request_id: String, data_base64: String },
    Stderr { request_id: String, data_base64: String },
    Diagnostic { request_id: String, diagnostic: WireDiagnostic },
    Completed { request_id: String, result: ExecutionResult },
    Unavailable { request_id: String, reason_code: String },
    Failed {
        request_id: String,
        phase: FailurePhase,
        reason_code: String,
        cleanup: CleanupState,
    },
}
```

Define closed enums for `CancelReason`, `FailurePhase`, `CleanupState`,
`Placement`, and `Guarantee`. `ExecutionResult` has `exit_code`, `signal`,
`timed_out`, `cancelled`, `cleanup`, and final diagnostics.

- [ ] **Step 5: Implement lossless native strings**

```rust
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "encoding", content = "value", rename_all = "snake_case")]
pub enum WireOsString {
    Utf8(String),
    UnixBytesBase64(String),
    WindowsUtf16LeBase64(String),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct WirePath(pub WireOsString);

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct WireEnv {
    pub key: WireOsString,
    pub value: WireOsString,
}
```

Conversions reject the wrong platform encoding and never use lossy
`to_string_lossy()` for executable paths, arguments, or environment entries.

- [ ] **Step 6: Add bounded JSONL codecs**

Use explicit limits:

```rust
pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_CONFIG_BYTES: usize = 64 * 1024;
pub const MAX_DIAGNOSTIC_BYTES: usize = 16 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
```

`JsonlReader` rejects oversized lines before deserialization, blank lines,
unknown required frame variants, invalid base64, and trailing non-whitespace.
`JsonlWriter` serializes exactly one UTF-8 JSON value plus `\n` and flushes
when used for `ready`, `started`, or terminal frames.

- [ ] **Step 7: Generate and pin the schema**

`schema.rs` produces one deterministic schema for the host/backend envelope.
The schema-sync test serializes with two-space indentation and a final newline:

```rust
#[test]
fn checked_in_schema_matches_generator() {
    let generated = opi_protocol::execution::v1::schema::render().unwrap();
    let checked_in = std::fs::read_to_string(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schemas/command-execution-jsonl-v1.schema.json"
        ),
    )
    .unwrap();
    assert_eq!(checked_in, generated);
}
```

- [ ] **Step 8: Validate fixtures and dependency purity**

```powershell
cargo test -p opi-protocol --all-targets
cargo tree -p opi-protocol
```

Expected: PASS. The dependency tree contains no `opi-agent`,
`opi-coding-agent`, Tokio process, Landlock, seccompiler, or platform sandbox
crate.

- [ ] **Step 9: Commit after authorization**

```powershell
git add Cargo.toml
git add Cargo.lock
git add crates/opi-protocol/Cargo.toml
git add crates/opi-protocol/src/lib.rs
git add crates/opi-protocol/src/execution/mod.rs
git add crates/opi-protocol/src/execution/v1/mod.rs
git add crates/opi-protocol/src/execution/v1/codec.rs
git add crates/opi-protocol/src/execution/v1/frames.rs
git add crates/opi-protocol/src/execution/v1/schema.rs
git add crates/opi-protocol/schemas/command-execution-jsonl-v1.schema.json
git add crates/opi-protocol/fixtures/valid/normal.jsonl
git add crates/opi-protocol/fixtures/invalid/out-of-order.jsonl
git add crates/opi-protocol/fixtures/invalid/oversized.jsonl
git add crates/opi-protocol/tests/execution_v1_contract.rs
git add crates/opi-protocol/tests/execution_v1_schema.rs
# opi-implement Phase E subject:
# feat(opi-protocol): add command execution protocol v1
```

---

### Task 16.4: Parse and Hard-Gate Executable Contributions

**Files:**

- Modify: `crates/opi-coding-agent/Cargo.toml`
- Modify: `crates/opi-coding-agent/src/package_discovery.rs`
- Modify: `crates/opi-coding-agent/src/package_store.rs`
- Modify: `crates/opi-coding-agent/src/package_resolver.rs`
- Create: `crates/opi-coding-agent/src/execution/mod.rs`
- Create: `crates/opi-coding-agent/src/execution/registry.rs`
- Create: `crates/opi-coding-agent/tests/execution_package_lifecycle.rs`

- [ ] **Step 1: Add `opi-protocol` to the product crate**

```toml
[dependencies]
opi-protocol = { workspace = true }
```

- [ ] **Step 2: Write failing manifest and scope tests**

```rust
#[test]
fn parses_command_execute_process_adapter_contribution() {}

#[test]
fn project_package_with_execution_adapter_is_rejected() {}

#[test]
fn project_package_with_legacy_process_adapter_is_rejected() {}

#[test]
fn project_package_with_only_static_resources_remains_valid() {}

#[test]
fn adapter_command_must_be_relative_canonical_and_contained() {}

#[test]
fn unknown_capability_duplicate_id_reserved_id_and_core_name_fail() {}

#[test]
fn target_version_protocol_and_sha256_are_hard_gates() {}

#[test]
fn validation_does_not_start_the_declared_executable() {}
```

Use `tempfile::tempdir()` and a sentinel executable that writes a marker if
started.

- [ ] **Step 3: Run the focused test**

```powershell
cargo test -p opi-coding-agent --test execution_package_lifecycle -- --nocapture
```

Expected: FAIL because `contributions.adapters` is unknown and executable scope
is not enforced.

- [ ] **Step 4: Add declarative manifest types**

Add:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterContributionManifest {
    pub capability: String,
    pub id: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub protocol: String,
    pub target: String,
    pub sha256: String,
    pub handshake_timeout_ms: u64,
}
```

`PackageManifest` gains `adapter_contributions:
Vec<AdapterContributionManifest>`. Parsing uses
`[[contributions.adapters]]`; absence is an empty vector. Do not implement
`[[contributions.tools]]`.

- [ ] **Step 5: Add lock material and validation**

Persist normalized contribution metadata in each package lock entry so runtime
does not trust mutable manifest data alone. Also persist whether the locked
package contributes static resources, so startup can distinguish a disabled
executable-only package from a package whose static resources still need
composition without scanning its directory. Validate:

```text
capability == command.execute
transport == process-jsonl
protocol == command-execution-jsonl-v1
id != local
id is a valid non-reserved package-style identifier
target == current compilation target
command is relative and contained after canonicalization
resolved command is a regular executable file
sha256 is lowercase 64-hex and matches file bytes
handshake_timeout_ms is within 100..=30000
opi_version contains the running version
```

Duplicate adapter ids fail across all enabled candidates; project package
precedence never shadows a global executable contribution.

- [ ] **Step 6: Keep executable discovery declarative**

`ExecutionContributionRegistry::inspect` reads lock, manifest, metadata, and
hash only. It never calls `Command::spawn`, `doctor`, `--version`, or protocol
handshake.

- [ ] **Step 7: Re-run tests**

```powershell
cargo test -p opi-coding-agent --test execution_package_lifecycle -- --nocapture
cargo test -p opi-coding-agent --lib package_discovery -- --nocapture
cargo test -p opi-coding-agent --lib package_resolver -- --nocapture
```

Expected: PASS. Existing static package resolution remains green.

- [ ] **Step 8: Commit after authorization**

```powershell
git add crates/opi-coding-agent/Cargo.toml
git add crates/opi-coding-agent/src/package_discovery.rs
git add crates/opi-coding-agent/src/package_store.rs
git add crates/opi-coding-agent/src/package_resolver.rs
git add crates/opi-coding-agent/src/execution/mod.rs
git add crates/opi-coding-agent/src/execution/registry.rs
git add crates/opi-coding-agent/tests/execution_package_lifecycle.rs
git add Cargo.lock
# opi-implement Phase E subject:
# feat(opi-coding-agent): validate execution adapter contributions
```

---

### Task 16.5: Add Package Trust and Enable/Disable Lifecycle

**Files:**

- Modify: `crates/opi-coding-agent/src/lib.rs`
- Modify: `crates/opi-coding-agent/src/cli.rs`
- Modify: `crates/opi-coding-agent/src/package_cli.rs`
- Modify: `crates/opi-coding-agent/src/package_store.rs`
- Modify: `crates/opi-coding-agent/src/package_resolver.rs`
- Modify: `crates/opi-coding-agent/src/runtime_packages.rs`
- Create: `crates/opi-coding-agent/src/package_activation.rs`
- Modify: `crates/opi-coding-agent/tests/execution_package_lifecycle.rs`
- Modify: `crates/opi-coding-agent/tests/package_cli.rs`
- Modify: `crates/opi-coding-agent/tests/harness_resource_integration.rs`

- [ ] **Step 1: Write failing five-gate lifecycle tests**

Add tests that distinguish each state:

```rust
#[test]
fn package_add_installs_executable_contributions_disabled_and_untrusted() {}

#[test]
fn first_enable_requires_tty_confirmation_after_showing_trust_material() {}

#[test]
fn non_tty_first_enable_refuses_with_remediation() {}

#[test]
fn enablement_does_not_grant_capability_permission() {}

#[test]
fn disable_retains_trust_only_for_the_unchanged_artifact() {}

#[test]
fn remove_deletes_enablement_and_trust_records() {}

#[test]
fn manifest_lock_or_executable_drift_invalidates_trust() {}

#[test]
fn legacy_global_process_adapter_is_also_disabled_until_trusted() {}
```

Inject a `PackageTrustPrompt` test double; do not manipulate the real terminal
or user config directory.

- [ ] **Step 2: Run the lifecycle tests**

```powershell
cargo test -p opi-coding-agent --test execution_package_lifecycle -- --nocapture
cargo test -p opi-coding-agent --test package_cli -- --nocapture
```

Expected: FAIL because `package enable`, `package disable`, and
`package-state.toml` do not exist.

- [ ] **Step 3: Implement a global activation store**

Use these domain types:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageActivationRecord {
    pub name: String,
    pub identity_kind: String,
    pub identity_value: String,
    pub manifest_sha256: String,
    pub trust_fingerprint: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageActivationFile {
    #[serde(default)]
    pub package: Vec<PackageActivationRecord>,
}
```

`PackageActivationStore` is rooted only at the user config directory and
writes `package-state.toml` atomically through a sibling temporary file and
rename. It rejects duplicates, malformed fingerprints, records without a
matching global lock identity, and any project-scope path.

- [ ] **Step 4: Compute deterministic trust fingerprints**

Use a serde-serializable `PackageTrustMaterial` containing:

```rust
pub struct PackageTrustMaterial {
    pub identity_kind: String,
    pub identity_value: String,
    pub manifest_sha256: String,
    pub executable_contributions: Vec<ExecutableTrustMaterial>,
}
```

Sort contributions by `(kind, capability, id)` and include every executable
field: transport, command, args, protocol, target, SHA-256, and handshake
timeout. Serialize compact JSON and SHA-256 the bytes. Tests pin ordering
independence and one-field drift.

- [ ] **Step 5: Add CLI commands and an injectable prompt**

Extend:

```rust
pub enum PackageCommand {
    Add { source: String, local: bool },
    Remove { name_or_source: String, local: bool },
    Enable { name: String },
    Disable { name: String },
    List { json: bool },
    Doctor { json: bool },
}
```

`PackageTrustPrompt` has one method:

```rust
pub trait PackageTrustPrompt {
    fn confirm(&mut self, review: &PackageTrustReview) -> std::io::Result<bool>;
}
```

The production implementation first checks both stdin and stderr are terminals,
prints package identity, version, manifest hash, executable hash, and every
executable contribution, then defaults to deny. There is no `--yes` or
machine-facing trust bypass in Phase 16.

- [ ] **Step 6: Change runtime startup to honor activation**

`runtime_packages` may resolve and start a legacy global `[adapter]` only when
its activation record is enabled and its current trust fingerprint matches.
Project legacy adapters fail before startup. Static resources retain their
existing behavior and project-trust gate; an executable package's disabled
adapter does not suppress unrelated static resources from the same package.

Execution adapter contributions are not started here; Task 16.8 resolves them
on demand.

- [ ] **Step 7: Extend list and doctor without executing packages**

Human and JSON output reports:

```text
installed
trusted
enabled
trust_status = matching | missing | stale
executable contribution ids and capabilities
target/protocol/hash/compatibility status
```

Neither command starts a process or probes native sandbox mechanisms.

- [ ] **Step 8: Run focused tests**

```powershell
cargo test -p opi-coding-agent --test execution_package_lifecycle -- --nocapture
cargo test -p opi-coding-agent --test package_cli -- --nocapture
cargo test -p opi-coding-agent --test harness_resource_integration -- --nocapture
cargo test -p opi-coding-agent --lib package_activation -- --nocapture
```

Expected: PASS. Install, trust, enablement, selection, and permission remain
distinct assertions.

- [ ] **Step 9: Commit after authorization**

```powershell
git add crates/opi-coding-agent/src/lib.rs
git add crates/opi-coding-agent/src/cli.rs
git add crates/opi-coding-agent/src/package_cli.rs
git add crates/opi-coding-agent/src/package_store.rs
git add crates/opi-coding-agent/src/package_resolver.rs
git add crates/opi-coding-agent/src/runtime_packages.rs
git add crates/opi-coding-agent/src/package_activation.rs
git add crates/opi-coding-agent/tests/execution_package_lifecycle.rs
git add crates/opi-coding-agent/tests/package_cli.rs
git add crates/opi-coding-agent/tests/harness_resource_integration.rs
# opi-implement Phase E subject:
# feat(opi-coding-agent): gate executable package activation
```

---

### Task 16.6: Add Execution Configuration, Failures, Routing, and Permission Policy

**Files:**

- Modify: `crates/opi-coding-agent/src/config.rs`
- Modify: `crates/opi-coding-agent/src/cli.rs`
- Modify: `crates/opi-coding-agent/src/execution/mod.rs`
- Create: `crates/opi-coding-agent/src/execution/failure.rs`
- Create: `crates/opi-coding-agent/src/execution/permission.rs`
- Create: `crates/opi-coding-agent/src/execution/router.rs`
- Create: `crates/opi-coding-agent/tests/execution_config.rs`
- Create: `crates/opi-coding-agent/tests/execution_failures.rs`
- Create: `crates/opi-coding-agent/tests/execution_permission.rs`
- Create: `crates/opi-coding-agent/tests/execution_routing.rs`

- [ ] **Step 1: Write failing default and validation tests**

```rust
#[test]
fn default_execution_config_is_fixed_local() {}

#[test]
fn fixed_requires_exactly_one_backend() {}

#[test]
fn rules_require_one_final_catch_all() {}

#[test]
fn rules_reject_unknown_modes_and_command_matchers() {}

#[test]
fn selected_rule_failure_does_not_continue_to_later_rules() {}

#[test]
fn deny_is_ineligible_and_model_invisible() {}

#[test]
fn ask_requires_interactive_grant_after_selection() {}

#[test]
fn allow_does_not_enable_or_select_an_adapter() {}

#[test]
fn external_failure_never_falls_back_to_local() {}

#[test]
fn trusted_project_cannot_set_persistent_permission() {}

#[test]
fn untrusted_project_permission_request_is_ignored_and_never_grants() {}

#[test]
fn explicit_config_file_cannot_set_persistent_permission() {}

#[test]
fn execution_backend_cli_override_implies_fixed_without_granting_permission() {}

#[test]
fn execution_strategy_cli_override_uses_configured_strategy_only() {}

#[test]
fn backend_and_non_fixed_strategy_overrides_conflict() {}
```

- [ ] **Step 2: Run the new tests**

```powershell
cargo test -p opi-coding-agent --test execution_config -- --nocapture
cargo test -p opi-coding-agent --test execution_routing -- --nocapture
cargo test -p opi-coding-agent --test execution_permission -- --nocapture
```

Expected: FAIL because the current config exposes `[sandbox] off|strict` and no
router or capability permission exists.

- [ ] **Step 3: Replace config types, retaining migration errors**

Define routing and user-owned policy as separate resolved values:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStrategy {
    Fixed,
    Rules,
    Model,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionDecision {
    Deny,
    Ask,
    Allow,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct ExecutionRule {
    #[serde(default)]
    pub modes: Vec<ExecutionRunMode>,
    pub backend: String,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ExecutionRoutingConfig {
    pub strategy: ExecutionStrategy,
    pub backend: Option<String>,
    #[serde(default)]
    pub rules: Vec<ExecutionRule>,
    #[serde(default)]
    pub backends: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct UserExecutionPolicy {
    #[serde(default)]
    pub permissions: std::collections::BTreeMap<String, PermissionDecision>,
}

pub struct ResolvedExecutionConfig {
    pub routing: ExecutionRoutingConfig,
    pub user_policy: UserExecutionPolicy,
}
```

The default is `fixed` + `local`; built-in local permission defaults to
`allow`.

Parse persistent `[execution.permissions]` only from the user configuration
layer before ordinary routing layers are merged. Project and explicit
`--config` layers may contribute routing fields but must reject
`[execution.permissions]`; an untrusted project layer remains skipped by the
existing trust gate and can never grant. Preserve this provenance through
resolution rather than deserializing permissions from the final merged TOML.

Add:

```text
--execution-backend <local|ADAPTER-ID>
--execution-strategy <fixed|rules|model>
```

Explicit CLI routing overrides win over `--config`, trusted project, user, and
default routing. `--execution-backend` implies `fixed`; combining it with
`--execution-strategy rules|model` is a configuration error. Strategy-only
overrides select an already configured strategy and its existing rules/backend.
Neither flag can set Package Trust, enablement, User Policy, or Capability
Permission.

Reject simultaneous old `[sandbox]` and new `[execution]` with a targeted
migration message. Remove `--sandbox` and `--sandbox-require`; if clap must
retain hidden parsing for one release, they return a configuration error that
points to `[execution]` and never activate old behavior.

- [ ] **Step 4: Implement the stable failure envelope**

Use serde snake-case enums:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionFailureCode {
    PackageNotInstalled,
    PackageUntrusted,
    ContributionDisabled,
    PolicyDenied,
    PermissionRequired,
    PermissionDenied,
    NoEligibleAdapter,
    AdapterNotSelected,
    AdapterUnavailable,
    ProtocolIncompatible,
    ProtocolViolation,
    ExecutionFailed,
    ExecutionTimedOut,
    CleanupUnconfirmed,
}
```

`ExtensionPhase` distinguishes discovery, trust, enablement, selection,
permission, handshake, setup, execution, and cleanup. Constructors set
`retryable`, `user_action_required`, and redacted remediation consistently.
Snapshot tests pin every code and prohibit secret-like keys and supplied
sentinel command/environment strings.

- [ ] **Step 5: Implement deterministic selection**

`CommandExecutionRouter::select` receives:

```rust
pub struct SelectionContext<'a> {
    pub run_mode: ExecutionRunMode,
    pub model_backend: Option<&'a str>,
    pub eligible: &'a [EligibleAdapter],
}
```

Behavior:

```text
fixed -> configured backend
rules -> first matching rule; catch-all last
model -> required model_backend
```

After a name is selected, that exact candidate passes installation, trust,
enablement, compatibility, and policy checks. A failure is terminal; it does
not select another rule or `local`.

- [ ] **Step 6: Implement permission management without UI coupling**

Define:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionBrokerDecision {
    AllowOnce,
    AllowSession,
    Deny,
    InteractionRequired,
}

pub trait PermissionBroker: Send + Sync {
    fn request(
        &self,
        request: PermissionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = PermissionBrokerDecision> + Send>
    >;
}
```

`PermissionManager` owns a memory-only `HashSet<PermissionKey>`. It checks
`deny`, `allow`, then `ask`; `AllowSession` inserts only into that set.
`HeadlessPermissionBroker` returns `InteractionRequired`, which maps to the
stable `permission_required` failure rather than `permission_denied`; it never
prompts and never grants. The decision type has no serde implementation and no
session-coordinator reference.

- [ ] **Step 7: Run focused tests**

```powershell
cargo test -p opi-coding-agent --test execution_config -- --nocapture
cargo test -p opi-coding-agent --test execution_failures -- --nocapture
cargo test -p opi-coding-agent --test execution_routing -- --nocapture
cargo test -p opi-coding-agent --test execution_permission -- --nocapture
```

Expected: PASS, including one-invocation consumption, session-memory reuse, and
fresh-manager denial after simulated resume/fork.

- [ ] **Step 8: Commit after authorization**

```powershell
git add crates/opi-coding-agent/src/config.rs
git add crates/opi-coding-agent/src/cli.rs
git add crates/opi-coding-agent/src/execution/mod.rs
git add crates/opi-coding-agent/src/execution/failure.rs
git add crates/opi-coding-agent/src/execution/permission.rs
git add crates/opi-coding-agent/src/execution/router.rs
git add crates/opi-coding-agent/tests/execution_config.rs
git add crates/opi-coding-agent/tests/execution_failures.rs
git add crates/opi-coding-agent/tests/execution_permission.rs
git add crates/opi-coding-agent/tests/execution_routing.rs
# opi-implement Phase E subject:
# feat(opi-coding-agent): add command execution policy
```

---

### Task 16.7: Implement the One-Shot Execution Protocol Host

**Files:**

- Modify: `crates/opi-coding-agent/Cargo.toml`
- Modify: `crates/opi-coding-agent/src/execution/mod.rs`
- Create: `crates/opi-coding-agent/src/execution/protocol_host.rs`
- Create: `crates/opi-coding-agent/tests/execution_protocol_host.rs`
- Create:
  `crates/opi-coding-agent/tests/fixtures/execution_backend_mock.rs`

- [ ] **Step 1: Register the test-only fixture binary**

```toml
[features]
execution-backend-test-fixture = []

[[bin]]
name = "execution_backend_mock"
path = "tests/fixtures/execution_backend_mock.rs"
test = false
bench = false
required-features = ["execution-backend-test-fixture"]
```

The fixture accepts scenario selection through a test-only environment variable
and writes protocol frames only on stdout.

- [ ] **Step 2: Add failing process-contract tests**

```rust
#[tokio::test]
async fn command_is_not_disclosed_before_valid_ready() {}

#[tokio::test]
async fn normal_sequence_maps_output_and_terminal_status() {}

#[tokio::test]
async fn malformed_oversized_duplicate_or_out_of_order_frames_fail_closed() {}

#[tokio::test]
async fn stdout_contamination_is_a_protocol_violation() {}

#[tokio::test]
async fn timeout_and_cancellation_send_cancel_then_kill_after_grace() {}

#[tokio::test]
async fn dropping_the_host_future_kills_backend_descendants() {}

#[tokio::test]
async fn backend_crash_evidence_is_bounded_and_redacted() {}
```

- [ ] **Step 3: Run the host test**

```powershell
cargo test -p opi-coding-agent --features execution-backend-test-fixture `
  --test execution_protocol_host -- --nocapture
```

Expected: FAIL because the protocol host does not exist.

- [ ] **Step 4: Implement one process per invocation**

`ExecutionProtocolHost::execute`:

1. creates a host request id;
2. configures the existing L0 process-tree primitive;
3. starts the backend with only manifest-declared command/args;
4. reserves stdin/stdout for protocol and stderr for bounded crash evidence;
5. sends `initialize`;
6. waits for `ready` under the minimum of request deadline and manifest
   handshake timeout;
7. validates protocol, implementation metadata, target, and request id;
8. sends `execute` only after readiness;
9. enforces `accepted -> started -> events -> terminal`;
10. closes stdin, requires a clean backend exit, then terminates remaining
    descendants.

- [ ] **Step 5: Map shell text before protocol disclosure**

The host receives an already resolved `ProgramInvocation`:

```rust
pub struct ProgramInvocation {
    pub program: std::ffi::OsString,
    pub args: Vec<std::ffi::OsString>,
}
```

On Unix the product maps a `bash` command to `sh -c <command>`; on Windows it
maps to `cmd /C <command>`. The neutral protocol never carries an Opi shell
string field.

- [ ] **Step 6: Bound output and cancellation**

Use protocol constants for line, message, diagnostic, and cumulative output
limits. On cancel/timeout:

```text
send cancel
wait bounded grace for terminal cleanup
kill supervised backend tree
return cleanup_unconfirmed if cleanup was not confirmed
```

The request deadline includes handshake, setup, execution, drain, and cleanup.

- [ ] **Step 7: Run focused tests**

```powershell
cargo test -p opi-coding-agent --features execution-backend-test-fixture `
  --test execution_protocol_host -- --nocapture
cargo test -p opi-coding-agent --test sandbox_l0 -- --nocapture
```

Expected: PASS. The mock never sees an `execute` frame in pre-ready failure
scenarios.

- [ ] **Step 8: Commit after authorization**

```powershell
git add crates/opi-coding-agent/Cargo.toml
git add crates/opi-coding-agent/src/execution/mod.rs
git add crates/opi-coding-agent/src/execution/protocol_host.rs
git add crates/opi-coding-agent/tests/execution_protocol_host.rs
git add crates/opi-coding-agent/tests/fixtures/execution_backend_mock.rs
# opi-implement Phase E subject:
# feat(opi-coding-agent): host execution protocol adapters
```

---

### Task 16.8: Build Routed Operations and the Minimal Runtime Split

**Files:**

- Modify: `crates/opi-coding-agent/src/lib.rs`
- Modify: `crates/opi-coding-agent/src/main.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/runner.rs`
- Modify: `crates/opi-coding-agent/src/rpc.rs`
- Modify: `crates/opi-coding-agent/src/runtime_packages.rs`
- Modify: `crates/opi-coding-agent/src/tool/operations.rs`
- Create: `crates/opi-coding-agent/src/execution/runtime.rs`
- Modify: `crates/opi-coding-agent/src/execution/router.rs`
- Create: `crates/opi-coding-agent/tests/execution_minimal_runtime.rs`
- Create: `crates/opi-coding-agent/tests/execution_product.rs`

- [ ] **Step 1: Write failing Minimal Runtime tests**

```rust
#[test]
fn default_startup_does_not_read_an_invalid_package_store_sentinel() {}

#[test]
fn default_startup_does_not_start_an_extension_sentinel() {}

#[test]
fn default_startup_constructs_local_operations_without_router_or_permission() {}

#[test]
fn non_default_strategy_leaves_the_minimal_path() {}

#[test]
fn enabled_external_contribution_leaves_the_minimal_path() {}
```

Use injected filesystem/process probes or a deliberately invalid package-store
path. Do not inspect timing as the correctness signal.

- [ ] **Step 2: Pin the pre-extension schema and local behavior**

Capture the current `bash` `ToolDef.input_schema` as a checked-in JSON fixture
or exact `serde_json::Value`, then assert default mode equality. Add a local
command fixture asserting exit code, timeout, cancellation, stdout/stderr
preview, details, diagnostics, and L0 behavior remain unchanged.

- [ ] **Step 3: Run the Minimal Runtime test**

```powershell
cargo test -p opi-coding-agent --test execution_minimal_runtime -- --nocapture
```

Expected: FAIL because current startup prepares built-in sandbox policy and
scans/starts installed process packages independently of activation.

- [ ] **Step 4: Extend `BashRequest` with internal selection input**

Add:

```rust
pub struct BashRequest {
    pub command: String,
    pub cwd: std::path::PathBuf,
    pub timeout: std::time::Duration,
    pub signal: tokio_util::sync::CancellationToken,
    pub env: Vec<(String, String)>,
    pub backend: Option<String>,
}
```

The field is internal routing input. It does not alter local command
construction or result shape.

- [ ] **Step 5: Add `RoutedBashOperations`**

```rust
pub struct RoutedBashOperations {
    router: CommandExecutionRouter,
    local: std::sync::Arc<LocalBashOperations>,
    registry: ExecutionContributionRegistry,
    host: ExecutionProtocolHost,
    permissions: PermissionManager,
}
```

`exec` selects exactly one adapter. `local` delegates to the existing local
implementation. A process adapter revalidates package activation, trust, lock,
target, compatibility, contained executable path, and SHA-256 immediately
before spawning, then checks capability permission and calls the host.

No failure after external selection calls `local.exec`.

- [ ] **Step 6: Add an explicit bootstrap enum**

```rust
pub enum ExecutionRuntime {
    Minimal {
        operations: std::sync::Arc<dyn BashOperations>,
    },
    Routed {
        operations: std::sync::Arc<dyn BashOperations>,
        model_backends: Vec<ModelBackendChoice>,
    },
}
```

`ExecutionRuntime::build` first checks:

```text
execution config is exactly default fixed + local
global package-state file is absent or has no enabled executable contribution
lock/index metadata reports no configured static package resources or legacy enabled adapter
no explicit execution override was requested
```

Only that branch constructs `LocalBashOperations` directly and returns an empty
`RuntimePackageStartup`. It does not scan package directories, resolve
manifests, allocate router/permission state, create background tasks, or
construct a protocol host. Reading the small declaration/lock/activation index
files needed to decide whether the set is empty is allowed; following package
roots or reading package manifests is not.

- [ ] **Step 7: Replace sandbox-aware harness construction**

Replace `build_tools_with_sandbox` with:

```rust
pub fn build_tools_with_operations(
    workspace_root: &std::path::Path,
    tool_config: &ToolRuntimeConfig,
    bash_operations: std::sync::Arc<dyn BashOperations>,
    bash_schema: BashBackendSchema,
) -> Vec<Box<dyn opi_agent::tool::Tool>>;
```

Every interactive, non-interactive, JSON, and RPC startup path obtains one
`ExecutionRuntime` and injects its operations. Remove calls to
`sandbox::prepare_production`, but retain native sandbox files until Task 16.16
ports and deletes them.

- [ ] **Step 8: Run focused production tests**

```powershell
cargo test -p opi-coding-agent --test execution_minimal_runtime -- --nocapture
cargo test -p opi-coding-agent --test execution_product -- --nocapture
cargo test -p opi-coding-agent --test sandbox_l0 -- --nocapture
cargo test -p opi-coding-agent --test harness_resource_integration -- --nocapture
```

Expected: PASS. The default branch proves no package scan, router, permission
state, protocol host, or extension process exists.

- [ ] **Step 9: Commit after authorization**

```powershell
git add crates/opi-coding-agent/src/lib.rs
git add crates/opi-coding-agent/src/main.rs
git add crates/opi-coding-agent/src/harness.rs
git add crates/opi-coding-agent/src/runner.rs
git add crates/opi-coding-agent/src/rpc.rs
git add crates/opi-coding-agent/src/runtime_packages.rs
git add crates/opi-coding-agent/src/tool/operations.rs
git add crates/opi-coding-agent/src/execution/runtime.rs
git add crates/opi-coding-agent/src/execution/router.rs
git add crates/opi-coding-agent/tests/execution_minimal_runtime.rs
git add crates/opi-coding-agent/tests/execution_product.rs
# opi-implement Phase E subject:
# feat(opi-coding-agent): route optional command adapters
```

---

### Task 16.9: Wire the Dynamic `bash` Schema and Headless Surfaces

**Files:**

- Modify: `crates/opi-coding-agent/src/tool/bash.rs`
- Modify: `crates/opi-coding-agent/src/tool/operations.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/main.rs`
- Modify: `crates/opi-coding-agent/src/runner.rs`
- Modify: `crates/opi-coding-agent/src/rpc.rs`
- Modify: `crates/opi-coding-agent/tests/tools_read_write_edit_bash.rs`
- Modify: `crates/opi-coding-agent/tests/non_interactive.rs`
- Modify: `crates/opi-coding-agent/tests/json_mode.rs`
- Modify: `crates/opi-coding-agent/tests/rpc_jsonl.rs`
- Modify: `crates/opi-coding-agent/tests/doctor_cli.rs`
- Modify: `crates/opi-coding-agent/tests/execution_product.rs`

- [ ] **Step 1: Add failing schema tests**

```rust
#[test]
fn fixed_local_bash_schema_is_byte_identical_to_the_baseline() {}

#[test]
fn rules_bash_schema_has_no_backend_field() {}

#[test]
fn model_bash_schema_requires_a_bounded_backend_enum() {}

#[test]
fn model_schema_omits_denied_untrusted_disabled_and_incompatible_backends() {}

#[test]
fn ask_model_choice_is_described_as_requiring_user_approval() {}
```

Compare canonical serialized JSON values, not key iteration order.

- [ ] **Step 2: Add failing headless permission/failure tests**

With `MockProvider`, request `bash` using an enabled trusted `ask` backend in:

```text
non-interactive text
--json NDJSON
--rpc JSONL
```

Assert no backend sentinel starts and each existing public envelope carries:

```json
{
  "code": "permission_required",
  "phase": "permission",
  "capability": "command.execute",
  "adapter": "opi-sandbox",
  "user_action_required": true,
  "remediation": ["Set execution.permissions.opi-sandbox = \"allow\" in the user config or use interactive approval."]
}
```

The exact JSON nesting follows the existing surface: tool-result details and
diagnostic context for agent events; RPC/NDJSON schema versions change only if
their existing generic details field cannot carry the object.

- [ ] **Step 3: Run the focused tests**

```powershell
cargo test -p opi-coding-agent --test tools_read_write_edit_bash -- --nocapture
cargo test -p opi-coding-agent --test execution_product -- --nocapture
```

Expected: FAIL because `bash` cannot parse/model-select a backend and the
headless paths do not surface `ExtensionFailure`.

- [ ] **Step 4: Add explicit schema policy**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BashBackendSchema {
    Hidden,
    Required(Vec<ModelBackendChoice>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBackendChoice {
    pub id: String,
    pub description: String,
    pub requires_approval: bool,
}
```

`BashArgs` gains an internal optional field:

```rust
#[serde(default)]
#[schemars(skip)]
pub backend: Option<String>,
```

`Hidden` uses the current `schema_for!(BashArgs)` output after the skipped field
and must match the pinned baseline. `Required` inserts one string property with
an enum and adds `"backend"` to `required`; it never accepts a free-form id.

- [ ] **Step 5: Pass model selection into `BashRequest`**

The tool passes `args.backend` unchanged. The router rejects:

```text
backend supplied outside model strategy
missing backend in model strategy
id absent from the exact schema candidate set
selected candidate that became invalid before spawn
```

The final case fails closed with the matching gate code and never retries.

- [ ] **Step 6: Map execution failures into existing tool results**

Add a `BashOpError::Extension(ExtensionFailure)` variant or an equivalent typed
carrier. `BashTool` maps it to:

```rust
result.details = Some(serde_json::json!({
    "extension_failure": failure,
}));
```

and an execution diagnostic whose context contains the same redacted object.
Text output shows the stable code and remediation, not backend stderr or
command text.

- [ ] **Step 7: Wire all headless startup modes**

Non-interactive, NDJSON, and RPC use `HeadlessPermissionBroker`. Their startup
must not allocate an interactive channel. `--allow-mutating` remains a separate
outer gate: enabling `bash` does not authorize an external adapter, and
authorizing an adapter does not enable `bash`.

Top-level doctor and package doctor report matching stable package/adapter
failure codes without launching the adapter.

- [ ] **Step 8: Run surface tests**

```powershell
cargo test -p opi-coding-agent --test tools_read_write_edit_bash -- --nocapture
cargo test -p opi-coding-agent --test non_interactive -- --nocapture
cargo test -p opi-coding-agent --test json_mode -- --nocapture
cargo test -p opi-coding-agent --test rpc_jsonl -- --nocapture
cargo test -p opi-coding-agent --test doctor_cli -- --nocapture
cargo test -p opi-coding-agent --test execution_product -- --nocapture
```

Expected: PASS. Default schema is unchanged; model schema is bounded; every
headless `ask` path fails with `permission_required`.

- [ ] **Step 9: Commit after authorization**

```powershell
git add crates/opi-coding-agent/src/tool/bash.rs
git add crates/opi-coding-agent/src/tool/operations.rs
git add crates/opi-coding-agent/src/harness.rs
git add crates/opi-coding-agent/src/main.rs
git add crates/opi-coding-agent/src/runner.rs
git add crates/opi-coding-agent/src/rpc.rs
git add crates/opi-coding-agent/tests/tools_read_write_edit_bash.rs
git add crates/opi-coding-agent/tests/non_interactive.rs
git add crates/opi-coding-agent/tests/json_mode.rs
git add crates/opi-coding-agent/tests/rpc_jsonl.rs
git add crates/opi-coding-agent/tests/doctor_cli.rs
git add crates/opi-coding-agent/tests/execution_product.rs
# opi-implement Phase E subject:
# feat(opi-coding-agent): expose bounded execution routing
```

---

### Task 16.10: Add the Interactive Permission Broker and TUI Prompt

**Files:**

- Modify: `crates/opi-tui/src/lib.rs`
- Modify: `crates/opi-tui/src/status_bar.rs`
- Create: `crates/opi-tui/src/permission_prompt.rs`
- Modify: `crates/opi-coding-agent/src/execution/permission.rs`
- Modify: `crates/opi-coding-agent/src/execution/runtime.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: `crates/opi-coding-agent/src/main.rs`
- Create: `crates/opi-coding-agent/tests/interactive_permission.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_mock.rs`

- [ ] **Step 1: Write failing prompt-state tests**

In `opi-tui`:

```rust
#[test]
fn permission_prompt_starts_on_allow_once() {}

#[test]
fn permission_prompt_cycles_allow_once_session_and_deny() {}

#[test]
fn awaiting_permission_projects_the_status_bar_state() {}
```

In the product test:

```rust
#[tokio::test]
async fn allow_once_authorizes_exactly_one_invocation() {}

#[tokio::test]
async fn allow_session_authorizes_later_calls_in_the_same_process() {}

#[tokio::test]
async fn deny_returns_permission_denied_without_starting_adapter() {}

#[tokio::test]
async fn cancel_or_closed_tui_denies_without_grant() {}

#[tokio::test]
async fn resume_and_fork_build_fresh_permission_state() {}
```

- [ ] **Step 2: Run the prompt tests**

```powershell
cargo test -p opi-tui permission_prompt -- --nocapture
cargo test -p opi-coding-agent --test interactive_permission -- --nocapture
```

Expected: FAIL because there is no permission prompt or interactive broker.

- [ ] **Step 3: Add TUI domain types**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    AllowOnce,
    AllowSession,
    Deny,
}

pub struct AwaitingPermissionState {
    pub capability: String,
    pub adapter: String,
    pub summary: String,
    pub response_tx: tokio::sync::oneshot::Sender<PermissionChoice>,
}
```

Add `AppState::AwaitingPermission` and `AppStatus::AwaitingPermission`.
`PermissionPrompt` renders only:

```text
Allow once
Allow for this session
Deny
```

It explains that the extension is already installed/trusted/enabled and that
this prompt grants invocation permission only. It never offers install, trust,
enable, or persistent allow.

- [ ] **Step 4: Add a channel-backed broker**

```rust
pub struct PermissionPromptRequest {
    pub request: PermissionRequest,
    pub response_tx: tokio::sync::oneshot::Sender<PermissionBrokerDecision>,
}

pub fn interactive_permission_channel(
) -> (
    std::sync::Arc<dyn PermissionBroker>,
    tokio::sync::mpsc::UnboundedReceiver<PermissionPromptRequest>,
);
```

The broker sends one request and awaits one response. A dropped receiver,
dropped sender, Esc, or TUI shutdown maps to deny. The adapter process is not
started while the request is pending.

- [ ] **Step 5: Integrate the outer TUI event loop**

Create the channel before interactive `ExecutionRuntime` construction. Pass the
receiver into `run_interactive_tui`. While the harness task owns its mutex and
awaits the broker, the outer TUI loop continues polling the permission channel,
renders `AwaitingPermission`, and sends exactly one choice.

Do not implement this through `AgentHooks::before_tool_call`: the permission
decision depends on the router-selected adapter, not merely the `bash` tool
name.

- [ ] **Step 6: Keep session grants memory-only**

The in-memory `PermissionManager` lives for one running interactive harness.
It is absent from:

```text
session JSONL
extension state
package-state.toml
config TOML
resume metadata
fork metadata
```

Tests inspect the resulting files for adapter ids and grant markers and then
construct a new harness to prove a new prompt is required.

- [ ] **Step 7: Run TUI and integration tests**

```powershell
cargo test -p opi-tui permission_prompt -- --nocapture
cargo test -p opi-coding-agent --test interactive_permission -- --nocapture
cargo test -p opi-coding-agent --test interactive_mock -- --nocapture
cargo test -p opi-coding-agent --test execution_product -- --nocapture
```

Expected: PASS. The adapter sentinel starts only after `AllowOnce` or
`AllowSession`; no choice persists across a new harness.

- [ ] **Step 8: Commit after authorization**

```powershell
git add crates/opi-tui/src/lib.rs
git add crates/opi-tui/src/status_bar.rs
git add crates/opi-tui/src/permission_prompt.rs
git add crates/opi-coding-agent/src/execution/permission.rs
git add crates/opi-coding-agent/src/execution/runtime.rs
git add crates/opi-coding-agent/src/harness.rs
git add crates/opi-coding-agent/src/interactive.rs
git add crates/opi-coding-agent/src/main.rs
git add crates/opi-coding-agent/tests/interactive_permission.rs
git add crates/opi-coding-agent/tests/interactive_mock.rs
# opi-implement Phase E subject:
# feat(opi-tui): prompt for adapter invocation permission
```

---

### Task 16.11: Build the Standalone `opi-sandbox` SDK, CLI, and Smoke Scripts

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `crates/opi-sandbox/Cargo.toml`
- Create: `crates/opi-sandbox/src/lib.rs`
- Create: `crates/opi-sandbox/src/cli.rs`
- Create: `crates/opi-sandbox/src/main.rs`
- Create: `crates/opi-sandbox/src/platform/mod.rs`
- Create: `crates/opi-sandbox/src/platform/windows.rs`
- Create: `crates/opi-sandbox/src/policy.rs`
- Create: `crates/opi-sandbox/src/process_tree.rs`
- Create: `crates/opi-sandbox/src/runner.rs`
- Create: `crates/opi-sandbox/tests/sdk_contract.rs`
- Create: `crates/opi-sandbox/tests/cli_contract.rs`
- Create: `scripts/opi-sandbox-smoke.sh`
- Create: `scripts/opi-sandbox-smoke.ps1`

- [ ] **Step 1: Add the workspace crate**

Add `"crates/opi-sandbox"` to workspace members and:

```toml
[workspace.dependencies]
opi-sandbox = { version = "0.7.1", path = "crates/opi-sandbox" }
```

The crate uses:

```toml
[dependencies]
opi-protocol = { workspace = true }
base64 = { workspace = true }
clap = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tempfile = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tokio-util = { workspace = true }
```

It has `[lib]` plus `[[bin]] name = "opi-sandbox"`.

- [ ] **Step 2: Write failing SDK state tests**

```rust
#[tokio::test]
async fn request_contains_all_policy_and_process_inputs() {}

#[tokio::test]
async fn sequential_runs_share_no_invocation_state() {}

#[tokio::test]
async fn completion_and_drop_remove_invocation_temporary_state() {}

#[tokio::test]
async fn fake_platform_refuses_before_target_start() {}

#[test]
fn crate_public_api_has_no_opi_agent_or_coding_agent_types() {}
```

Use an injected fake platform and a marker target. Do not require native
Landlock or `sandbox-exec` yet.

- [ ] **Step 3: Write failing CLI-dispatch tests**

Against an injectable runner:

```rust
#[test]
fn run_preserves_explicit_program_and_argument_vector() {}

#[test]
fn run_passes_stdin_stdout_and_stderr_as_bytes() {}

#[test]
fn run_maps_usage_timeout_setup_cancel_exit_and_signal_codes() {}

#[test]
fn doctor_json_has_the_stable_six_fields() {}

#[test]
fn help_and_version_do_not_read_opi_state() {}
```

- [ ] **Step 4: Run the crate tests**

```powershell
cargo test -p opi-sandbox --all-targets -- --nocapture
```

Expected: FAIL because the SDK and CLI do not exist.

- [ ] **Step 5: Implement the independent SDK**

Export:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProfile {
    WorkspaceWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPolicy {
    Deny,
    Allow,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub profile: SandboxProfile,
    pub network: NetworkPolicy,
}

pub struct SandboxRequest {
    pub program: std::ffi::OsString,
    pub args: Vec<std::ffi::OsString>,
    pub workspace: std::path::PathBuf,
    pub cwd: std::path::PathBuf,
    pub env: Vec<(std::ffi::OsString, std::ffi::OsString)>,
    pub inherit_env: bool,
    pub timeout: std::time::Duration,
    pub stdin: StdinMode,
    pub cancellation: tokio_util::sync::CancellationToken,
    pub policy: SandboxPolicy,
}
```

`SandboxRunner<P: PlatformSandbox>` owns no cross-call state. Each `run`
canonicalizes explicit paths, allocates one temporary root and process-tree
guard, delegates restriction setup to `P`, emits structured events, and removes
owned temporary state on terminal completion or guard drop.

- [ ] **Step 6: Port policy-neutral L0 supervision**

Port the verified Phase 15 process-tree behavior into
`opi-sandbox/src/process_tree.rs`. The crate does not import private
`opi-coding-agent` modules. Pin clean-exit descendant cleanup, timeout,
cancellation, drop, bounded pipe drain, and platform termination behavior in
SDK tests.

- [ ] **Step 7: Implement the human CLI shell**

Use clap subcommands:

```rust
pub enum Command {
    Run(RunArgs),
    Doctor(DoctorArgs),
    Backend(BackendArgs),
    #[command(hide = true)]
    Helper(HelperArgs),
}
```

At this checkpoint `Backend` and `Helper` may return a typed unavailable error
until Task 16.12, but parsing is complete. `run` uses the SDK and an explicit
program vector; it never interprets a shell expression. Direct mode inherits
stdin and forwards output bytes without UTF-8 conversion.

- [ ] **Step 8: Implement stable doctor output**

```rust
#[derive(serde::Serialize)]
pub struct DoctorReport {
    pub schema_version: u32,
    pub supported: bool,
    pub target: String,
    pub mechanisms: Vec<String>,
    pub profiles: Vec<String>,
    pub limitations: Vec<String>,
}
```

On Windows, `supported=false`; `run` returns 125 before the target marker can
start. A completed `doctor --json` returns zero.

- [ ] **Step 9: Create standalone smoke scripts**

Both scripts accept exactly one required binary path plus an optional work
directory. They:

```text
copy the binary to a fresh temporary directory
construct a PATH containing required system tools but no opi
set OPI_CONFIG, OPI_SESSIONS_DIR, and OPI_PACKAGES_DIR to invalid sentinels
run from an empty cwd
check --help, --version, and doctor --json
on Windows, prove run returns 125 before a marker target starts
on supported Unix, run the full IO/exit/backend/native suite after Tasks 16.12-16.14
assert no .opi, session, package, daemon, or durable sandbox files appear
```

The scripts must fail if their own text contains an invocation of `opi` other
than the provided `opi-sandbox` path.

- [ ] **Step 10: Run independent checks**

```powershell
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo tree -p opi-sandbox
cargo build -p opi-sandbox --bin opi-sandbox
.\scripts\opi-sandbox-smoke.ps1 `
  -BinaryPath .\target\debug\opi-sandbox.exe
```

On Windows, expected: tests PASS; tree has no `opi-agent` or
`opi-coding-agent`; help/version/doctor PASS; `run` refuses before target start.
On Linux/macOS, run the `.sh` script; native run assertions become mandatory
after Tasks 16.13/16.14.

- [ ] **Step 11: Commit after authorization**

```powershell
git add Cargo.toml
git add Cargo.lock
git add crates/opi-sandbox/Cargo.toml
git add crates/opi-sandbox/src/lib.rs
git add crates/opi-sandbox/src/cli.rs
git add crates/opi-sandbox/src/main.rs
git add crates/opi-sandbox/src/platform/mod.rs
git add crates/opi-sandbox/src/platform/windows.rs
git add crates/opi-sandbox/src/policy.rs
git add crates/opi-sandbox/src/process_tree.rs
git add crates/opi-sandbox/src/runner.rs
git add crates/opi-sandbox/tests/sdk_contract.rs
git add crates/opi-sandbox/tests/cli_contract.rs
git add scripts/opi-sandbox-smoke.sh
git add scripts/opi-sandbox-smoke.ps1
# opi-implement Phase E subject:
# feat(opi-sandbox): add independent SDK and CLI
```

---

### Task 16.12: Add the Atomic Helper Gate and Protocol Backend

**Files:**

- Create: `crates/opi-sandbox/src/backend.rs`
- Create: `crates/opi-sandbox/src/helper.rs`
- Modify: `crates/opi-sandbox/src/cli.rs`
- Modify: `crates/opi-sandbox/src/lib.rs`
- Modify: `crates/opi-sandbox/src/main.rs`
- Modify: `crates/opi-sandbox/src/platform/mod.rs`
- Modify: `crates/opi-sandbox/src/runner.rs`
- Create: `crates/opi-sandbox/tests/protocol_conformance.rs`
- Create: `crates/opi-sandbox/tests/fixtures/protocol_client.py`
- Modify: `crates/opi-sandbox/tests/sdk_contract.rs`
- Modify: `scripts/opi-sandbox-smoke.sh`
- Modify: `scripts/opi-sandbox-smoke.ps1`

- [ ] **Step 1: Write failing atomic-start tests**

```rust
#[tokio::test]
async fn accepted_is_emitted_before_setup_and_target_start() {}

#[tokio::test]
async fn started_is_flushed_before_the_target_gate_is_released() {}

#[tokio::test]
async fn setup_failure_never_releases_the_target() {}

#[tokio::test]
async fn backend_protocol_stdin_is_never_target_stdin() {}

#[tokio::test]
async fn one_backend_process_accepts_only_one_execute() {}

#[tokio::test]
async fn cancellation_before_and_after_start_confirms_cleanup() {}
```

Use a marker target and a test writer whose `flush` call is observable.

- [ ] **Step 2: Add a non-Rust fixture client**

`tests/fixtures/protocol_client.py` uses only Python stdlib. It:

```text
spawns the supplied opi-sandbox binary with backend --stdio
sends initialize and validates ready
sends execute with a portable explicit program vector
if doctor reports supported, validates accepted, started, output, completed, and clean EOF
if doctor reports unsupported, validates accepted then a pre-start unavailable/failed terminal frame
rejects any non-JSON stdout
```

It contains no Opi configuration or package metadata.

- [ ] **Step 3: Run the conformance tests**

```powershell
cargo test -p opi-sandbox --test protocol_conformance -- --nocapture
```

Expected: FAIL because backend and helper modes are not implemented.

- [ ] **Step 4: Implement the helper handshake**

The helper receives command/policy data through an invocation-owned control
channel, not ordinary command-line arguments. Its sequence is:

```text
read bounded request
close nonessential inherited descriptors
apply platform restriction
create target process at a blocked start gate
send Armed(effective contract) to parent
wait for one Release byte
exec/release target
```

Only control-handle identifiers may appear in the hidden helper's arguments.
The helper remains single-threaded until restrictions and the gate are
established. Any required `unsafe` is isolated in the platform module, has a
safety comment, and is covered by the marker test.

- [ ] **Step 5: Make SDK events and protocol use the same runner**

`SandboxRunner` emits:

```rust
pub enum SandboxEvent {
    Accepted,
    Started(EffectiveSandbox),
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Diagnostic(SandboxDiagnostic),
    Completed(SandboxResult),
}
```

Direct `run` releases the gate after its event sink observes `Started`.
Protocol mode serializes and flushes the `started` frame before sending
`Release`. There is no separate native-policy implementation for backend mode.

- [ ] **Step 6: Implement the one-shot backend state machine**

Use `opi_protocol::execution::v1` codecs and validate:

```text
initialize is first
request ids match
selected protocol is command-execution-jsonl-v1
execute arrives once
program/workspace/cwd/env decode losslessly
deadline and all size limits hold
cancel is accepted only for the active request
terminal frame is unique
stdin closes and process exits after completion
```

Protocol stdout contains frames only. Bounded internal crash evidence goes to
stderr.

- [ ] **Step 7: Extend smoke scripts with backend mode**

Invoke the Python fixture with the explicit copied binary path. If Python is
unavailable in a release job, compile and use a tiny neutral fixture host from
`opi-protocol` tests; never substitute `opi`.

- [ ] **Step 8: Run direct acceptance**

```powershell
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo build -p opi-sandbox --bin opi-sandbox
.\scripts\opi-sandbox-smoke.ps1 `
  -BinaryPath .\target\debug\opi-sandbox.exe
```

On Unix:

```bash
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo build -p opi-sandbox --bin opi-sandbox
./scripts/opi-sandbox-smoke.sh ./target/debug/opi-sandbox
```

Expected: protocol conformance PASS. Platform-native run assertions report
unsupported until the next tasks; the neutral fixture validates the structured
pre-start refusal and proves the target did not start.

- [ ] **Step 9: Commit after authorization**

```powershell
git add crates/opi-sandbox/src/backend.rs
git add crates/opi-sandbox/src/helper.rs
git add crates/opi-sandbox/src/cli.rs
git add crates/opi-sandbox/src/lib.rs
git add crates/opi-sandbox/src/main.rs
git add crates/opi-sandbox/src/platform/mod.rs
git add crates/opi-sandbox/src/runner.rs
git add crates/opi-sandbox/tests/protocol_conformance.rs
git add crates/opi-sandbox/tests/fixtures/protocol_client.py
git add crates/opi-sandbox/tests/sdk_contract.rs
git add scripts/opi-sandbox-smoke.sh
git add scripts/opi-sandbox-smoke.ps1
# opi-implement Phase E subject:
# feat(opi-sandbox): add atomic protocol execution
```

---

### Task 16.13: Port the Linux Native Restriction Contract

**Files:**

- Modify: `crates/opi-sandbox/Cargo.toml`
- Create: `crates/opi-sandbox/src/platform/linux.rs`
- Modify: `crates/opi-sandbox/src/platform/mod.rs`
- Modify: `crates/opi-sandbox/src/helper.rs`
- Modify: `crates/opi-sandbox/src/runner.rs`
- Create: `crates/opi-sandbox/tests/linux_policy.rs`
- Modify: `scripts/opi-sandbox-smoke.sh`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Move Linux-only dependencies to the extension crate**

```toml
[target.'cfg(target_os = "linux")'.dependencies]
landlock = { workspace = true }
libc = { workspace = true }
seccompiler = { workspace = true }
```

Do not remove them from `opi-coding-agent` until Task 16.16 deletes the old
native implementation.

- [ ] **Step 2: Write failing pure policy tests**

Pin:

```rust
#[test]
fn danger_blocklist_includes_required_syscalls_and_not_clone_unshare() {}

#[test]
fn network_deny_blocks_inet_inet6_netlink_and_preserves_unix() {}

#[test]
fn network_deny_blocks_io_uring_setup() {}

#[test]
fn workspace_write_allows_only_workspace_and_invocation_temp_mutations() {}

#[test]
fn requested_contract_is_unavailable_if_any_required_mechanism_cannot_engage() {}
```

The danger blocklist includes `open_by_handle_at`, `bpf`,
`perf_event_open`, `ptrace`, `kexec_load`, `kexec_file_load`, `reboot`,
`init_module`, `finit_module`, `delete_module`, `swapon`, `swapoff`, `acct`,
and `settimeofday`; x86_64 also includes `iopl` and `ioperm`.

- [ ] **Step 3: Write failing direct CLI native tests**

Run the built binary, not an in-process helper, and assert:

```text
write/create/remove/rename inside workspace succeeds
write/create/remove/rename outside workspace fails
invocation temporary root is writable and removed afterward
host reads and execution of a system toolchain remain allowed
network=deny cannot create/connect/bind INET/INET6/NETLINK
network=deny retains AF_UNIX IPC
network=allow connects to a local TCP listener
inherited nonessential descriptors are closed
target marker cannot run on setup failure
doctor reports the actually observed mechanisms and limitations
```

Use only local listeners and temporary directories; never require internet.

- [ ] **Step 4: Run the Linux tests**

On Linux:

```bash
cargo test -p opi-sandbox --test linux_policy -- --nocapture
```

Expected: FAIL because `NativePlatform` has no Linux restriction
implementation.

- [ ] **Step 5: Port Landlock filesystem mutation policy**

Canonicalize the workspace and invocation temporary root before helper start.
Landlock allows host reads/execution and limits mutation operations to those
roots. The helper applies the ruleset before it reports `Armed`.

If the kernel/ABI cannot enforce every promised workspace-write rule, return a
pre-start unavailable result. Do not downgrade to supervision-only.

- [ ] **Step 6: Port seccomp and network policy**

Install:

```text
fixed danger-syscall blocklist
io_uring_setup denial
socket(AF_INET/AF_INET6/AF_NETLINK) denial for network=deny
Landlock TCP bind/connect restrictions where supported
close nonessential inherited descriptors
```

Keep AF_UNIX. Report cooperating descriptor-transfer limitations without
claiming confidentiality or isolation. The effective guarantee is
`restricted`, never `isolated`.

- [ ] **Step 7: Keep the atomic start contract**

All Landlock/seccomp/fd setup occurs in the helper before `Armed`. The backend
flushes `started` before releasing the target. Any setup failure returns 125 in
direct CLI mode or a structured pre-start failure in protocol mode.

- [ ] **Step 8: Make Linux smoke mandatory in CI**

Add a native Linux job that:

```bash
cargo build --release -p opi-sandbox --bin opi-sandbox
./scripts/opi-sandbox-smoke.sh ./target/release/opi-sandbox
```

The smoke script performs the direct filesystem/network tests itself. Preserve
its complete log as an artifact on failure.

- [ ] **Step 9: Run Linux verification**

```bash
cargo test -p opi-sandbox --test linux_policy -- --nocapture
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo build --release -p opi-sandbox --bin opi-sandbox
./scripts/opi-sandbox-smoke.sh ./target/release/opi-sandbox
```

Expected: PASS on supported CI Linux. Unsupported host kernels fail the native
contract test rather than silently reporting success.

- [ ] **Step 10: Commit after authorization**

```bash
git add crates/opi-sandbox/Cargo.toml
git add crates/opi-sandbox/src/platform/linux.rs
git add crates/opi-sandbox/src/platform/mod.rs
git add crates/opi-sandbox/src/helper.rs
git add crates/opi-sandbox/src/runner.rs
git add crates/opi-sandbox/tests/linux_policy.rs
git add scripts/opi-sandbox-smoke.sh
git add .github/workflows/ci.yml
# opi-implement Phase E subject:
# feat(opi-sandbox): enforce Linux workspace policy
```

---

### Task 16.14: Port macOS Restriction and Pin the Windows Posture

**Files:**

- Create: `crates/opi-sandbox/src/platform/macos.rs`
- Modify: `crates/opi-sandbox/src/platform/mod.rs`
- Modify: `crates/opi-sandbox/src/platform/windows.rs`
- Modify: `crates/opi-sandbox/src/helper.rs`
- Create: `crates/opi-sandbox/tests/macos_policy.rs`
- Modify: `crates/opi-sandbox/tests/cli_contract.rs`
- Modify: `scripts/opi-sandbox-smoke.sh`
- Modify: `scripts/opi-sandbox-smoke.ps1`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Write failing macOS direct CLI tests**

Against the built `opi-sandbox` binary:

```text
workspace and invocation-temp writes succeed
outside writes/removes/renames fail
host reads and executable toolchains remain available
network=deny cannot reach a local TCP listener
network=allow can reach the listener
missing/rejected sandbox-exec fails before marker start
doctor labels sandbox-exec legacy/experimental and lists limitations
```

- [ ] **Step 2: Write Windows posture tests**

```rust
#[cfg(target_os = "windows")]
#[test]
fn doctor_reports_restriction_unsupported_but_returns_zero() {}

#[cfg(target_os = "windows")]
#[test]
fn run_returns_125_before_target_marker_starts() {}

#[cfg(target_os = "windows")]
#[test]
fn help_and_version_succeed_without_opi_state() {}
```

Do not describe Job Objects as a restriction mechanism. They provide only
supervision for Opi's built-in `local` path.

- [ ] **Step 3: Run native tests**

On macOS:

```bash
cargo test -p opi-sandbox --test macos_policy -- --nocapture
```

On Windows:

```powershell
cargo test -p opi-sandbox --test cli_contract -- --nocapture
```

Expected: macOS RED until the profile launcher exists; Windows posture tests
should become GREEN without starting any target.

- [ ] **Step 4: Implement the macOS profile**

Generate a `sandbox-exec` profile with escaped literal canonical paths:

```text
(version 1)
(allow default)
(deny file-write*)
(allow file-write* (subpath "<workspace>"))
(allow file-write* (subpath "<invocation-temp>"))
```

For `network=deny`, add `(deny network*)`. For `network=allow`, do not add a
network deny. The helper must prove it is inside the profile before reporting
`Armed`; missing/rejected `sandbox-exec` is a pre-start failure.

- [ ] **Step 5: Preserve truthful guarantee reporting**

Report:

```text
placement=host
guarantee=restricted
mechanism=sandbox-exec
limitations include legacy/experimental and no syscall-filter claim
```

Never report `isolated`.

- [ ] **Step 6: Make native smoke mandatory**

Add native macOS release-binary smoke using
`scripts/opi-sandbox-smoke.sh`. Keep Windows smoke limited to
help/version/doctor and pre-start refusal. Preserve logs on failure.

- [ ] **Step 7: Run verification**

On macOS:

```bash
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo build --release -p opi-sandbox --bin opi-sandbox
./scripts/opi-sandbox-smoke.sh ./target/release/opi-sandbox
```

On Windows:

```powershell
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo build --release -p opi-sandbox --bin opi-sandbox
.\scripts\opi-sandbox-smoke.ps1 `
  -BinaryPath .\target\release\opi-sandbox.exe
```

Expected: native contract PASS on macOS; Windows reports unsupported and never
starts the target marker.

- [ ] **Step 8: Commit after authorization**

```powershell
git add crates/opi-sandbox/src/platform/macos.rs
git add crates/opi-sandbox/src/platform/mod.rs
git add crates/opi-sandbox/src/platform/windows.rs
git add crates/opi-sandbox/src/helper.rs
git add crates/opi-sandbox/tests/macos_policy.rs
git add crates/opi-sandbox/tests/cli_contract.rs
git add scripts/opi-sandbox-smoke.sh
git add scripts/opi-sandbox-smoke.ps1
git add .github/workflows/ci.yml
# opi-implement Phase E subject:
# feat(opi-sandbox): enforce macOS workspace policy
```

---

### Task 16.15: Package Release Artifacts and Smoke the Extracted Archives

**Files:**

- Create: `packaging/opi-sandbox/package.toml.template`
- Create: `scripts/package-opi-sandbox.sh`
- Create: `scripts/package-opi-sandbox.ps1`
- Modify: `scripts/opi-sandbox-smoke.sh`
- Modify: `scripts/opi-sandbox-smoke.ps1`
- Modify: `.github/workflows/release.yml`
- Create: `crates/opi-coding-agent/tests/opi_sandbox_package.rs`

- [ ] **Step 1: Write failing package-layout tests**

Given an extracted package directory, assert:

```rust
#[test]
fn package_contains_manifest_binary_schema_license_and_no_opi_binary() {}

#[test]
fn manifest_target_protocol_and_executable_hash_match_archive_bytes() {}

#[test]
fn package_add_accepts_the_archive_directory_disabled_and_untrusted() {}

#[test]
fn no_windows_opi_sandbox_package_is_declared() {}
```

- [ ] **Step 2: Run the package test**

```powershell
cargo test -p opi-coding-agent --test opi_sandbox_package -- --nocapture
```

Expected: FAIL because no package template or artifact generator exists.

- [ ] **Step 3: Define the package layout**

```text
opi-sandbox-<target>/
  package.toml
  LICENSE
  bin/opi-sandbox[.exe]
  share/command-execution-jsonl-v1.schema.json
```

The package must not contain `opi`, Opi configuration, a package-state file, a
session directory, or a daemon launcher.

- [ ] **Step 4: Generate a complete manifest**

The packaging scripts:

1. read the workspace/package version from Cargo metadata;
2. compute the compatible Opi minor range;
3. copy the target binary;
4. compute lowercase SHA-256 over copied bytes;
5. substitute target, version, range, executable path, and hash in the
   template;
6. copy the checked-in schema byte-for-byte;
7. create `.tar.gz` on Linux/macOS.

The generated contribution is:

```toml
[[contributions.adapters]]
capability = "command.execute"
id = "opi-sandbox"
transport = "process-jsonl"
command = "bin/opi-sandbox"
args = ["backend", "--stdio"]
protocol = "command-execution-jsonl-v1"
handshake_timeout_ms = 5000
```

Target and SHA-256 are generated fields. There is no Windows archive branch.

- [ ] **Step 5: Make release builds native and independently smokeable**

Add release matrix entries:

```text
x86_64-unknown-linux-gnu on native x64 Linux
aarch64-unknown-linux-gnu on native arm64 Linux
x86_64-apple-darwin on native x64 macOS
aarch64-apple-darwin on native arm64 macOS
```

Each entry:

```text
builds only opi-sandbox
packages it
extracts the archive into a new directory
runs the standalone smoke script against the extracted binary
uploads archive and complete smoke log
```

Do not publish an `opi-sandbox-windows-*` artifact. The ordinary Opi release
matrix continues to publish its existing Windows binaries.

- [ ] **Step 6: Test install state without granting trust**

Run `opi package add <extracted-dir>` against isolated config paths. Assert the
package appears installed, disabled, and untrusted; no adapter process starts.
The first `package enable` remains a separate interactive action.

- [ ] **Step 7: Run extracted-archive smoke locally**

On a supported Unix host:

```bash
cargo build --release -p opi-sandbox --bin opi-sandbox
./scripts/package-opi-sandbox.sh \
  --target "$(rustc -vV | sed -n 's/^host: //p')" \
  --binary ./target/release/opi-sandbox \
  --output ./target/opi-sandbox-package
mkdir -p ./target/opi-sandbox-extracted
tar -xzf ./target/opi-sandbox-package/*.tar.gz \
  -C ./target/opi-sandbox-extracted
./scripts/opi-sandbox-smoke.sh \
  ./target/opi-sandbox-extracted/*/bin/opi-sandbox
```

Expected: PASS using only the extracted binary and its runtime files.

- [ ] **Step 8: Run package tests**

```powershell
cargo test -p opi-coding-agent --test opi_sandbox_package -- --nocapture
```

Expected: PASS. Hash and target drift are detected before any process starts.

- [ ] **Step 9: Commit after authorization**

```powershell
git add packaging/opi-sandbox/package.toml.template
git add scripts/package-opi-sandbox.sh
git add scripts/package-opi-sandbox.ps1
git add scripts/opi-sandbox-smoke.sh
git add scripts/opi-sandbox-smoke.ps1
git add .github/workflows/release.yml
git add crates/opi-coding-agent/tests/opi_sandbox_package.rs
# opi-implement Phase E subject:
# build(opi-sandbox): package standalone native artifacts
```

---

### Task 16.16: Remove Core Native Sandbox and Prove the Complete Product

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/opi-coding-agent/Cargo.toml`
- Modify: `crates/opi-coding-agent/src/lib.rs`
- Modify: `crates/opi-coding-agent/src/config.rs`
- Modify: `crates/opi-coding-agent/src/cli.rs`
- Modify: `crates/opi-coding-agent/src/main.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/tool/operations.rs`
- Modify: `crates/opi-coding-agent/src/tool/process_tree.rs`
- Delete: `crates/opi-coding-agent/src/sandbox.rs`
- Delete: `crates/opi-coding-agent/src/sandbox/linux.rs`
- Delete: `crates/opi-coding-agent/src/sandbox/macos.rs`
- Delete: `crates/opi-coding-agent/src/sandbox/windows.rs`
- Delete: `crates/opi-coding-agent/tests/sandbox_config.rs`
- Delete: `crates/opi-coding-agent/tests/sandbox_strict.rs`
- Delete: `crates/opi-coding-agent/tests/sandbox_linux_backend.rs`
- Modify: `crates/opi-coding-agent/tests/sandbox_l0.rs`
- Modify: `crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs`
- Modify: `crates/opi-coding-agent/tests/phase16_extension_docs.rs`
- Modify: `crates/opi-coding-agent/tests/execution_product.rs`
- Modify: `crates/opi-coding-agent/tests/non_interactive.rs`
- Modify: `crates/opi-coding-agent/tests/json_mode.rs`
- Modify: `crates/opi-coding-agent/tests/rpc_jsonl.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_permission.rs`
- Modify: `crates/opi-coding-agent/tests/doctor_cli.rs`
- Modify: `README.md`
- Modify: `README.zh.md`
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Modify: `AGENTS.md`
- Modify: `CLAUDE.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add failing dependency and residue guards**

The replacement docs/architecture test asserts:

```rust
#[test]
fn opi_binary_does_not_depend_on_opi_sandbox_or_native_policy_crates() {}

#[test]
fn opi_sandbox_depends_on_opi_protocol_not_agent_product_crates() {}

#[test]
fn coding_agent_has_no_sandbox_module_config_flags_or_prepared_types() {}

#[test]
fn default_bash_schema_and_local_result_fixture_are_unchanged() {}

#[test]
fn docs_do_not_claim_builtin_native_command_restriction() {}

#[test]
fn docs_name_package_trust_enablement_selection_and_permission_separately() {}

#[test]
fn phase15_archived_acceptance_history_remains_citable() {}
```

Adapt the existing Phase 15 documentation guard rather than deleting it. It
must continue to validate archived Phase 15 specs/snapshots and L0 history,
while current product claims move to the Phase 16 guard and must not describe
native restriction as built into Opi.

Use `cargo metadata --no-deps` or manifest parsing for crate edges and source
walks for prohibited symbols:

```text
PreparedSandbox
SandboxConfig
SandboxMode
StrictBackend
--sandbox
--sandbox-require
CODE_SANDBOX_DEGRADED
CODE_SANDBOX_UNAVAILABLE
```

Do not prohibit the independent crate/package name `opi-sandbox`.

- [ ] **Step 2: Add complete production vertical slices**

Using isolated package/config/session directories, a generated package, the
real protocol host, and `MockProvider`, prove:

```text
default fixed-local ignores invalid package store and runs local
package add installs disabled/untrusted
package enable displays exact trust material and explicit prompt controls trust
disable and remove prevent execution
hash drift invalidates trust before process start
fixed opi-sandbox routes a real bash tool turn
rules choose by run mode and do not fall through after selected failure
model enum contains only eligible non-denied adapters
interactive ask allows once, session, or deny
text/NDJSON/RPC ask returns permission_required
external crash/protocol/setup/execution/cleanup failures never invoke local
all surfaces carry the same stable code and redacted remediation
```

The install-to-execute scenario must reach these production call sites:

```text
package CLI dispatch
PackageActivationStore
ExecutionRuntime::build
CommandExecutionRouter::select
PermissionManager
ExecutionProtocolHost::execute
BashTool::execute
```

- [ ] **Step 3: Run red tests before deletion**

```powershell
cargo test -p opi-coding-agent --test phase16_extension_docs -- --nocapture
cargo test -p opi-coding-agent --features execution-backend-test-fixture `
  --test execution_product -- --nocapture
```

Expected: FAIL while old core sandbox symbols/dependencies/docs remain or any
vertical slice is incomplete.

- [ ] **Step 4: Remove built-in native restriction**

Delete the native sandbox modules and sandbox-specific integration tests only
after their L0 assertions and native policy assertions are green in their new
owners:

```text
L0 -> coding-agent process supervision tests
Linux/macOS restriction -> opi-sandbox tests
Windows restriction posture -> opi-sandbox CLI tests
```

Remove `landlock`, `seccompiler`, and sandbox-only `libc` uses from
`opi-coding-agent`. Do not remove process-tree supervision or Windows Job
Objects used by local/adapter host lifecycle.

- [ ] **Step 5: Complete migration behavior**

Old `[sandbox]`, `--sandbox`, and `--sandbox-require` return targeted migration
errors pointing to:

```toml
[execution]
strategy = "fixed"
backend = "opi-sandbox"
```

and the package workflow:

```text
opi package add <package-directory>
opi package enable opi-sandbox
```

Existing project-local executable/process packages are rejected with
remediation to reinstall globally and explicitly enable/trust them. Static
project resources remain unchanged.

- [ ] **Step 6: Update paired documentation**

Document in both English and Chinese:

```text
minimal core and zero extension runtime overhead claim
dormant generic host still contributes binary size and one enablement check
local is supervised, not OS-restricted
opi-sandbox is optional and independently runnable
fixed/rules/model and deny/ask/allow
Package Trust vs Capability Permission
global-only executable contributions
no external fallback
Linux/macOS guarantees and limitations
Windows package not published
standalone CLI commands and exit codes
package add/enable/disable/remove workflow
```

Update `AGENTS.md` and `CLAUDE.md` in lockstep. Add `CHANGELOG.md` entries only
under `Unreleased`; do not edit released sections.

- [ ] **Step 7: Run all scenario owners**

```powershell
cargo test -p opi-protocol --all-targets -- --nocapture
cargo test -p opi-sandbox --all-targets -- --nocapture
cargo test -p opi-coding-agent --test phase16_extension_docs -- --nocapture
cargo test -p opi-coding-agent --test execution_minimal_runtime -- --nocapture
cargo test -p opi-coding-agent --test execution_package_lifecycle -- --nocapture
cargo test -p opi-coding-agent --test execution_routing -- --nocapture
cargo test -p opi-coding-agent --test execution_permission -- --nocapture
cargo test -p opi-coding-agent --features execution-backend-test-fixture `
  --test execution_protocol_host -- --nocapture
cargo test -p opi-coding-agent --features execution-backend-test-fixture `
  --test execution_product -- --nocapture
cargo test -p opi-coding-agent --test interactive_permission -- --nocapture
cargo test -p opi-coding-agent --test non_interactive -- --nocapture
cargo test -p opi-coding-agent --test json_mode -- --nocapture
cargo test -p opi-coding-agent --test rpc_jsonl -- --nocapture
cargo test -p opi-coding-agent --test doctor_cli -- --nocapture
```

Expected: PASS. Preserve output artifacts for every runtime/CLI claim.

- [ ] **Step 8: Run the independent CLI suite**

Build and invoke the sandbox binary directly:

```powershell
cargo build --release -p opi-sandbox --bin opi-sandbox
.\scripts\opi-sandbox-smoke.ps1 `
  -BinaryPath .\target\release\opi-sandbox.exe
```

On supported Unix:

```bash
cargo build --release -p opi-sandbox --bin opi-sandbox
./scripts/opi-sandbox-smoke.sh ./target/release/opi-sandbox
```

Expected:

```text
no opi executable on smoke PATH
invalid Opi config/session/package sentinels ignored
help/version/doctor pass
direct run IO/args/exit/timeout pass on Linux/macOS
backend protocol fixture passes
native restriction sentinels pass on Linux/macOS
Windows run refuses before target marker
no durable Opi or sandbox state created
```

- [ ] **Step 9: Run extracted release archive smoke**

On each supported native release target, generate, extract, and pass the same
standalone smoke script. Preserve:

```text
packaging command
archive SHA-256
extracted tree listing
smoke stdout/stderr
exit code
doctor JSON
native sentinel results
```

Run `scripts/opi-artifact-audit.py` against the evidence directory. Only
`verified` evidence closes SC16-09 through SC16-12.

- [ ] **Step 10: Run workspace quality gates**

```powershell
cargo fmt --all
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
$env:RUSTDOCFLAGS='-D warnings'
cargo doc --workspace --no-deps
Remove-Item Env:RUSTDOCFLAGS
```

Expected: every command exits zero. Review any snapshot changes manually; do
not auto-accept them.

- [ ] **Step 11: Run residue and dependency checks**

```powershell
rg -n "PreparedSandbox|SandboxConfig|SandboxMode|StrictBackend|--sandbox-require|--sandbox" `
  crates/opi-coding-agent README.md README.zh.md docs/opi-spec.md docs/opi-spec.zh.md
cargo tree -p opi-coding-agent | rg "opi-sandbox|landlock|seccompiler"
cargo tree -p opi-sandbox | rg "opi-agent|opi-coding-agent"
```

Expected: each `rg` returns no matches except intentionally quoted migration
tests/docs; inspect those explicitly. The Opi tree contains no concrete sandbox
crate/native policy dependencies. The sandbox tree contains neither agent
crate.

- [ ] **Step 12: Commit after authorization**

Stage only the files changed by this task. Deletions use explicit paths:

```powershell
git add Cargo.toml
git add Cargo.lock
git add crates/opi-coding-agent/Cargo.toml
git add crates/opi-coding-agent/src/lib.rs
git add crates/opi-coding-agent/src/config.rs
git add crates/opi-coding-agent/src/cli.rs
git add crates/opi-coding-agent/src/main.rs
git add crates/opi-coding-agent/src/harness.rs
git add crates/opi-coding-agent/src/tool/operations.rs
git add crates/opi-coding-agent/src/tool/process_tree.rs
git add -u crates/opi-coding-agent/src/sandbox.rs
git add -u crates/opi-coding-agent/src/sandbox/linux.rs
git add -u crates/opi-coding-agent/src/sandbox/macos.rs
git add -u crates/opi-coding-agent/src/sandbox/windows.rs
git add -u crates/opi-coding-agent/tests/sandbox_config.rs
git add -u crates/opi-coding-agent/tests/sandbox_strict.rs
git add -u crates/opi-coding-agent/tests/sandbox_linux_backend.rs
git add crates/opi-coding-agent/tests/sandbox_l0.rs
git add crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs
git add crates/opi-coding-agent/tests/phase16_extension_docs.rs
git add crates/opi-coding-agent/tests/execution_product.rs
git add crates/opi-coding-agent/tests/non_interactive.rs
git add crates/opi-coding-agent/tests/json_mode.rs
git add crates/opi-coding-agent/tests/rpc_jsonl.rs
git add crates/opi-coding-agent/tests/interactive_permission.rs
git add crates/opi-coding-agent/tests/doctor_cli.rs
git add README.md
git add README.zh.md
git add docs/opi-spec.md
git add docs/opi-spec.zh.md
git add AGENTS.md
git add CLAUDE.md
git add CHANGELOG.md
# opi-implement Phase E subject:
# feat(opi): complete pluggable command execution
```

- [ ] **Step 13: Perform the Phase F audit**

The phase-exit evaluator must rebuild every architecture and Phase 16
acceptance criterion from both approved specs. It must independently inspect:

```text
Minimal Runtime production call path
five distinct executable-package/invocation gates
routing and permission authority bounds
no-fallback behavior
protocol conformance and redaction
independent SDK/CLI behavior
native Linux/macOS restriction
Windows truthful posture
extracted release artifacts
paired documentation and non-goals
```

Any criterion without preserved production evidence is `not-met`; do not
archive the phase. If all criteria pass, use the normal explicit
`opi-implement` Phase F archive gate. Do not hand-edit or prune the ledger.

## Final Verification Matrix

| Claim | Required command/evidence | Expected |
|---|---|---|
| Protocol independence | `cargo tree -p opi-protocol` | no Opi product/process policy deps |
| Sandbox independence | `cargo tree -p opi-sandbox` | no `opi-agent`/`opi-coding-agent` |
| Opi has no concrete sandbox | `cargo tree -p opi-coding-agent` | no `opi-sandbox`/Landlock/seccompiler |
| Minimal Runtime | `execution_minimal_runtime` | no store scan/router/permission/process |
| Package gates | `execution_package_lifecycle` | install/trust/enable/select/permit distinct |
| Routing | `execution_routing` + product test | fixed/rules/model bounded |
| Permission | headless + interactive tests | deny/ask/allow exact |
| Fail closed | protocol/product failure scenarios | no local fallback |
| Direct SDK | `cargo test -p opi-sandbox --test sdk_contract` | stateless and cleanup green |
| Direct CLI | platform smoke script | never invokes Opi |
| Protocol backend | Python neutral fixture | one-shot sequence green |
| Linux | native direct smoke | workspace/network contract green |
| macOS | native direct smoke | workspace/network contract green |
| Windows | PowerShell smoke | unsupported/refusal truthful |
| Release archive | extracted archive smoke | same direct suite green |
| Workspace | fmt/clippy/test/doc gates | all zero exit |

## Execution Handoff

After this plan is reviewed, choose one execution mode:

1. **Subagent-driven in this task:** use
   `superpowers:subagent-driven-development`, one ledger task at a time, with
   review after every task.
2. **Inline sequential execution:** use `superpowers:executing-plans` through
   `opi-implement`, preserving its task and ledger commit gates.

In either mode, start with `opi-implement plan`; do not start Task 16.2 until
the reconciled graph is explicitly approved.
