# Phase 16: Pluggable Extensions and Command Execution

Status: approved on 2026-07-28.

This is the canonical Phase 16 specification. It consolidates the approved
extension architecture in
`2026-07-28-pluggable-extension-architecture-design.md`, the command-execution
vertical slice, and the reviewed implementation contract in
`../plans/2026-07-28-pluggable-extension-command-execution.md`.

Phase 16 specifies the extension state model, Package Trust, routing,
permission, Minimal Runtime, failure interface, the `command.execute`
Capability, the `local` and `opi-sandbox` adapters, and the first
`opi-protocol` module. If a supporting design note or the implementation plan
conflicts with this document, this document wins.

It supersedes the narrower draft that selected one backend before tool
construction. Phase 16 now includes fixed, deterministic-rule, and model
routing under User Policy.

## Roadmap Position

Phase 16 establishes the first complete pluggable-extension vertical slice and
must finish before the project writes the Phase 17 specification.

- Phase 17 is reserved for a benchmark and regression-evaluation phase. Its
  specification will be discussed only after Phase 16 exits; this document
  does not pre-design its corpus, metrics, providers, or gates.
- Agent Intelligence moves to Phase 18. Its existing design remains separate
  and will use the future Phase 17 baseline to prioritize skills/fragments,
  compaction, branch summaries, and inline-image work.
- Broader extension architecture is reserved for Phase 19.
- TUI and graphical-interface productization are deferred to Phase 20.

Phase 16 neither implements benchmark infrastructure nor pulls Phase 18-20
features forward.

## Architecture Commitments

Opi uses a hybrid contribution model:

- implementations of a stable Capability register as Capability Adapters;
- genuinely new actions register independent tools;
- extensions cannot shadow or replace core tools.

Phase 16 implements only the first adapter slice. The stable tool is `bash`,
the stable Capability is `command.execute`, and `local` plus `opi-sandbox` are
adapters. Independent new-tool contributions remain a Phase 19 design topic.

An executable contribution passes five separate gates:

1. **Installed**: package artifact and manifest exist.
2. **Trusted**: Package Trust matches the exact locked artifact.
3. **Enabled**: the user made the contribution available.
4. **Selected**: the router chose it for this invocation.
5. **Permitted**: Capability Permission authorized this invocation.

Installation does not imply trust or enablement. Enablement does not grant
permission. Selection does not grant permission. Permission does not select an
adapter. A model may select only from a user-bounded set and cannot mutate any
of these gates.

## Overview

Phase 16 replaces the built-in Phase 15 native sandbox with an official,
optional `opi-sandbox` Extension Package.

The stable tool remains `bash`:

```text
Capability: command.execute
Tool: bash
Built-in adapter: local
External adapter: opi-sandbox
Wire module: opi_protocol::execution::v1
Wire id: command-execution-jsonl-v1
```

`opi-sandbox` is one independently usable Rust package with:

- a library SDK;
- a human-facing CLI;
- a one-shot protocol backend for Opi and other agents.

The Opi binary does not link `opi-sandbox`. Without an enabled external
adapter, `bash` uses `LocalBashOperations` directly and creates no extension
runtime.

## Motivation

Phase 15 combines two separate concerns:

1. L0 process lifecycle supervision; and
2. optional operating-system command restriction.

Supervision belongs in the core because every local child process needs
timeout, cancellation, output bounds, and process-tree cleanup. Native
restriction is platform-specific and optional.

Pi 0.80.6 keeps sandbox, SSH, and Gondolin implementations in extensions that
replace tool Operations. Opi follows that product shape while adding explicit
package identity, trust, routing, permission, protocol negotiation, integrity,
and fail-closed behavior suitable for out-of-process adapters.

The source comparison is recorded in
`docs/research/2026-07-27-sandbox-comparison.md`.

## Goals

- Preserve `local` as the built-in default.
- Preserve always-on L0 supervision without describing it as sandboxing.
- Remove Landlock, seccomp, `sandbox-exec`, and sandbox helper implementations
  from the Opi binary.
- Add the first declarative Capability Adapter contribution.
- Support `fixed`, `rules`, and `model` routing for `command.execute`.
- Enforce `deny`, `ask`, and `allow` without granting the model authority.
- Add `opi-protocol` with only `execution::v1`.
- Make `opi-sandbox` reusable without Opi through its SDK and CLI.
- Validate the standalone CLI as a first-class Phase 16 acceptance surface.
- Fail closed after an external adapter is selected.

## Non-Goals

- Building Docker, VM, SSH, Gondolin, or remote adapters.
- Routing file, navigation, or other built-in tools.
- Letting extensions replace `bash` or another core tool by name.
- Defining a universal extension protocol.
- Migrating `opi-extension-jsonl-v1`, RPC, NDJSON, or trace envelopes.
- Dynamically loading native libraries.
- Composing multiple execution adapters for one invocation.
- Providing host-read or environment-variable confidentiality.
- Sandboxing the extension process itself.
- Authenticating publishers with an in-package checksum.
- Supporting project-local executable contributions.
- Implementing Windows AppContainer or restricted-token restriction.
- Preserving unreleased Phase 15 sandbox configuration aliases.

## Product Contract

### Execution Backend

An Execution Backend is a Capability Adapter for `command.execute`.

| Identity | Placement | Effective guarantee |
|---|---|---|
| `local` | host | `supervised` |
| `opi-sandbox` | host | `restricted` |
| future Docker adapter | container | adapter-dependent |
| future VM adapter | VM | adapter-dependent |
| future remote adapter | remote host | adapter-dependent |

Adapter identity never establishes a guarantee by itself. Each invocation
reports its effective placement, guarantee, policy, and limitations after setup
has succeeded.

### Scope

Only the model-callable `bash` tool uses `command.execute` in Phase 16:

| Surface | Phase 16 behavior |
|---|---|
| `bash` | Routed through `command.execute` |
| `write`, `edit` | Existing workspace `PathPolicy` |
| `read`, `grep`, `find`, `ls`, `glob` | Existing mode-aware path policy |
| Existing extension tools/hooks | Existing extension implementation |
| Opi/provider network | Unaffected by command backend network policy |

Opi has no separate Pi-style `!` command surface. A future surface must
explicitly decide whether it shares this Capability.

### Supervision

L0 supervision applies to:

- local target processes;
- external backend processes;
- descendants of both.

It controls timeout, cancellation, process-tree cleanup, and bounded output
draining. It does not restrict files, network, syscalls, credentials, or model
provider access and is reported only as `supervised`.

## Configuration and Routing

### Default and fixed routing

The default is equivalent to:

```toml
[execution]
strategy = "fixed"
backend = "local"
```

This path constructs `LocalBashOperations` directly. It does not create a
Capability Router, inspect the package store, alter the `bash` schema, or
initialize protocol state.

An explicit fixed external selection is:

```toml
[execution]
strategy = "fixed"
backend = "opi-sandbox"
```

`--execution-backend <local|ADAPTER-ID>` is an invocation override that selects
the `fixed` strategy. `--execution-strategy <fixed|rules|model>` may select a
configured strategy, but cannot supply Package Trust or permission.

### Deterministic rules

Phase 16 rules intentionally match only the host-known run mode:

```toml
[execution]
strategy = "rules"

[[execution.rules]]
modes = ["non-interactive", "rpc"]
backend = "opi-sandbox"

[[execution.rules]]
backend = "local"
```

Rules are evaluated in declaration order. Exactly one catch-all rule is
required and must be last. Allowed Phase 16 modes are `interactive`,
`non-interactive`, and `rpc`.

Rules do not inspect or classify command text. Command regexes, shell parsing,
and model-generated risk labels are outside this phase because they could
create a false safety boundary.

The selected rule result must still pass Package Trust, enablement, User
Policy, and Capability Permission. Failure does not continue to the next rule.

### Model routing

For `strategy = "model"`, the `bash` JSON Schema adds a required `backend`
field. Its enum contains only adapters that are:

- built in or installed;
- trusted where applicable;
- enabled;
- target- and version-compatible;
- not denied by User Policy.

An `ask` candidate is visible with a description that it requires interactive
approval. A `deny` candidate is absent. The model cannot provide an unknown
identity or select a disabled package.

`fixed` and `rules` do not expose `backend` in the tool schema. The default
`local + fixed` schema therefore remains byte-for-byte equivalent to the
pre-extension schema.

### User Policy and permission

Conceptually:

```toml
[execution.permissions]
local = "allow"
"opi-sandbox" = "ask"
```

The actual configuration follows Opi's layered TOML conventions, but these
decisions are normative:

- `deny`: adapter is ineligible and model-invisible;
- `ask`: adapter may be selected but needs a one-invocation or current-session
  interactive grant;
- `allow`: persistent user configuration authorizes invocation.

The current-session grant is memory-only and does not survive process restart,
session resume, or fork.

Project configuration may request a strategy or adapter only after the
existing project-trust gate. It cannot create persistent `allow`, establish
Package Trust, or enable a package.

Persistent permission policy is resolved from the user-owned configuration
layer before routing layers are merged. Project `[execution.permissions]` is
rejected even when the project is trusted; trust permits routing requests, not
authorization. CLI routing overrides likewise cannot provide permission.

In non-interactive, NDJSON, and RPC runs, `ask` returns
`permission_required` with remediation. It never prompts or silently uses
`local`.

The existing non-interactive `--allow-mutating` gate remains outside this
policy. Enabling the `bash` tool does not authorize an external adapter, and
authorizing an adapter does not enable the `bash` tool.

## Executable Package Lifecycle

### Install, trust, and enable

Execution-adapter packages are global-only.

Initial lifecycle:

1. `opi package add <local-directory>` validates and installs the package in a
   disabled state.
2. `opi package enable <name>` displays package identity, version, locked
   executable hash, and executable contributions.
3. The user explicitly confirms Package Trust; only then are the contributions
   enabled.
4. `opi package disable <name>` makes contributions unavailable without
   deleting an unchanged trust record.
5. `opi package remove <name>` removes the package, enablement, and trust
   record.

First enablement requires an interactive user confirmation. A non-TTY or
machine-facing invocation refuses to infer trust and returns remediation.
Future signed or managed installation workflows require a separate design.

If the manifest, lock, or executable changes, Package Trust no longer matches.
The package fails closed until the user reviews the new artifact. An adapter
process is never started merely to establish trust or availability.

Project-local packages declaring executable adapter or tool contributions are
rejected. Static project resources continue to follow their existing scope and
trust rules.

### Contribution manifest

The existing `package.toml` gains a declarative adapter contribution:

```toml
name = "opi-sandbox"
description = "Official host-native command restriction backend."
version = "0.8.0"
opi_version = ">=0.8,<0.9"

[[contributions.adapters]]
capability = "command.execute"
id = "opi-sandbox"
transport = "process-jsonl"
command = "bin/opi-sandbox"
args = ["backend", "--stdio"]
protocol = "command-execution-jsonl-v1"
target = "x86_64-unknown-linux-gnu"
sha256 = "<lowercase 64-character hex digest>"
handshake_timeout_ms = 5000
```

Phase 16 requires:

- `capability = "command.execute"`;
- unique adapter identity;
- `transport = "process-jsonl"`;
- `protocol = "command-execution-jsonl-v1"`;
- required package and Opi compatibility versions;
- exact target match;
- a relative command path contained by the canonical package root;
- fixed manifest arguments;
- a regular executable file;
- exact SHA-256 match;
- bounded handshake timeout and extension configuration.

Absolute, bare `PATH`, drive-relative, traversal, and symlink-escape commands
are rejected. The package lock records the manifest hash, executable path and
hash, package version, target, Opi range, protocol, and adapter identity.

The stored hash detects drift but does not authenticate the publisher.

### Discovery

Ordinary startup reads the configured enabled-package identities. If the set is
empty and routing is default local, it does not scan the package store.

When an external adapter is enabled or requested, core resolves only the named
package metadata and revalidates its trust and lock before each process start.
Discovery does not execute package code or probe platform mechanisms.

`opi package list`, `opi package doctor`, and top-level `opi doctor` may inspect
all installed packages. They report installation, trust, enablement, adapter
identity, target, compatibility, protocol, and lock/hash status without
starting adapter executables.

## Core Architecture

`BashOperations` remains the public command-execution trait:

```text
BashTool
  -> Arc<dyn BashOperations>
       +-- LocalBashOperations
       |    `-- ProcessSupervisor
       `-- RoutedBashOperations
            +-- CommandExecutionRouter
            +-- LocalCommandAdapter
            +-- ProcessCommandAdapter
            |    `-- ExecutionProtocolHost
            `-- ProcessSupervisor
```

The tool-construction plan supplies both:

- the `BashOperations` implementation;
- the optional model-routing schema fragment.

`BashTool` therefore knows the optional `backend` request field but does not
know package paths, trust records, protocol frames, or sandbox mechanisms.

`LocalBashOperations` is constructed directly for the default Minimal Runtime.
`RoutedBashOperations` exists only when configured routing requires it.

There is no closed enum for sandbox, Docker, VM, or SSH. The router uses stable
adapter identities from validated contributions. Adding a future adapter does
not add a core enum variant.

After routing:

- `local` executes under L0;
- an external adapter creates one supervised backend process for that
  invocation;
- any selected-adapter failure becomes a tool failure;
- no failure path invokes another adapter.

## `opi-protocol`

### Crate boundary

Phase 16 adds the publishable shared crate:

```text
opi-protocol
  `-- execution::v1
```

Consumers:

```text
opi-coding-agent ----+
opi-sandbox ---------+--> opi-protocol::execution::v1
other agents --------+
```

`opi-protocol` contains protocol types, bounded codecs, schemas, and fixtures.
It has no dependency on `opi-agent` or `opi-coding-agent` and owns no process
launch, package policy, routing, permission, or sandbox behavior.

The wire identity remains `command-execution-jsonl-v1`; it is independent of
the Cargo crate version. Later modules require separate designs and do not
change `execution::v1`.

### Process and transport

The host starts:

```text
opi-sandbox backend --stdio
```

with:

- stdin reserved for host-to-backend UTF-8 JSONL;
- stdout reserved for backend-to-host UTF-8 JSONL;
- stderr reserved for bounded backend crash evidence;
- the backend process under L0 supervision.

Command and configuration data are sent in protocol messages, never command
arguments. Every frame carries one host-generated request id. Command stdout
and stderr chunks are base64. Core bounds line size, message size,
configuration, diagnostics, and cumulative output.

The protocol is product-neutral. It contains no Opi package path, lock,
manifest hash, Opi version, session id, tool-call id, or extension-hook field.

### State machine

```text
host starts backend
  -> initialize
  <- ready
  -> execute
  <- accepted
  <- started
  <- stdout | stderr | diagnostic  (zero or more)
  <- completed
  -> host closes stdin
  -> backend exits successfully
```

Normative behavior:

- `initialize` carries deadline, adapter configuration, and the host's ordered
  supported protocol list.
- `ready` selects a protocol and reports implementation identity, version, and
  target.
- The command is not disclosed until `ready` validates.
- `execute` carries explicit program and arguments, canonical workspace,
  working directory, timeout, environment inheritance policy, and bounded
  environment additions.
- Opi maps the `bash` shell string to an explicit platform shell program and
  argument vector before sending `execute`.
- `accepted` means the request is valid and the target has not started.
- `started` means setup established the reported placement, guarantee, policy,
  and limitations at an atomic target-start gate.
- The backend flushes `started` before releasing the target.
- `completed` is terminal and reports exit/signal, timeout/cancellation, cleanup
  state, and final diagnostics.
- One backend process accepts at most one execution.

Before `started`, the backend may terminate with `unavailable` or `failed`.
After `started`, failure identifies execution or cleanup phase. Malformed,
oversized, duplicate, unknown-required, or out-of-order messages, stdout
contamination, premature EOF, timeout, and unexpected process exit are protocol
failures.

The backend never inherits protocol stdin as target stdin. Phase 16's `bash`
contract has no target stdin stream.

### Cancellation and cleanup

The request deadline covers startup, handshake, setup, execution, output drain,
and cleanup.

On timeout or cancellation:

1. the host sends `cancel` with request id and reason;
2. the backend cancels the target and reports a terminal cleanup state within a
   bounded grace period;
3. the host kills the supervised backend process tree after grace expiry;
4. unconfirmed destination cleanup is returned as `cleanup_unconfirmed`.

Dropping the execution future follows the same local kill path.

## Independent `opi-sandbox` Product

### Package shape

`opi-sandbox` is one Rust package with library and binary targets:

```text
opi-sandbox
  +-- library SDK
  |    +-- SandboxPolicy
  |    +-- SandboxRequest
  |    +-- SandboxRunner
  |    `-- SandboxEvent / SandboxResult
  `-- binary
       +-- run
       +-- backend --stdio
       `-- doctor --json
```

Exact Rust signatures follow workspace conventions, but the library interface
must remain independent of Opi configuration, sessions, package storage, and
agent types.

The crate depends on `opi-protocol`, not `opi-agent` or `opi-coding-agent`.
The binary is a thin adapter over the same runner used by the SDK.

### State model

The module is invocation-stateful but cross-invocation stateless:

- no daemon, session, history, credential, or package database;
- policy and configuration are explicit inputs;
- each call owns its temporary root, restriction setup, child tree, output,
  cancellation, and cleanup guard;
- sequential SDK calls cannot observe state from earlier calls;
- invocation-owned resources are removed at terminal completion or guard drop.

### Human CLI

The independent command is:

```text
opi-sandbox run \
  --workspace <PATH> \
  --profile workspace-write \
  --network <deny|allow> \
  -- <PROGRAM> [ARGUMENTS...]
```

It executes an explicit program and argument vector. Users who want a shell
expression explicitly invoke their shell. Target stdout and stderr pass through
as bytes. Direct `run` inherits terminal stdin by default; `backend --stdio` never
passes protocol stdin to the target.

Exit behavior:

- target normal exit returns the target exit code;
- CLI usage errors return `2`;
- timeout returns `124`;
- pre-start policy, platform, or setup failure returns `125`;
- interactive cancellation returns `130`.

After target start, an ordinary target exit takes precedence and is returned
verbatim even when it equals a reserved pre-start code. On Unix, signal
termination follows the conventional `128 + signal` mapping. The protocol and
SDK retain the unambiguous structured status.

Rich integrations should use the SDK or protocol result rather than infer all
semantics from an exit code.

Capability inspection is:

```text
opi-sandbox doctor --json
```

It reports platform support, observed mechanisms, available profile/network
contracts, and limitations without reading Opi configuration or modifying
project state. Its stable object includes `schema_version`, `supported`,
`target`, `mechanisms`, `profiles`, and `limitations`. A completed diagnostic
returns zero even when `supported` is false; malformed invocation or internal
diagnostic failure returns nonzero.

Agent integrations use:

```text
opi-sandbox backend --stdio
```

This mode writes protocol frames only to stdout, processes one invocation, and
exits after its terminal result.

The binary also provides ordinary `--help` and `--version` without Opi.

### Standalone CLI acceptance

Phase 16 is not complete merely because Opi can launch the package. CI must
validate the built `opi-sandbox` executable directly.

The standalone smoke suite:

1. builds `opi-sandbox` as its own binary target;
2. copies or installs that binary into an isolated temporary directory;
3. runs with no `opi` executable on the test `PATH`;
4. points Opi-specific configuration, session, and package environment
   overrides at invalid sentinel locations and uses an empty working directory,
   proving those inputs are ignored;
5. verifies `--help`, `--version`, and `doctor --json`;
6. invokes `run` with an explicit target path and verifies target arguments,
   stdin, stdout, stderr, and exit mapping;
7. invokes `backend --stdio` through a product-neutral protocol fixture host;
8. verifies no Opi files or durable sandbox state are created.

On supported native Linux and macOS, the direct CLI suite also verifies the
documented filesystem and network contract. It must exercise the installed
binary, not `cargo run -p opi-coding-agent` and not an in-process test helper.

On Windows, the workspace binary must still provide `--help`, `--version`, and
`doctor --json`. `doctor` reports unsupported restriction, and `run` refuses
before a target sentinel starts. Phase 16 publishes no official Windows
`opi-sandbox` package artifact.

Release packaging must smoke-test the extracted target archive in the same
standalone manner so a missing runtime file or incorrect package layout cannot
pass through workspace-only tests.

The repository provides `scripts/opi-sandbox-smoke.sh` and
`scripts/opi-sandbox-smoke.ps1`. Each accepts an explicit built or extracted
binary path, launches that binary directly, and never invokes `opi`. Native CI
and release jobs call the appropriate script rather than duplicating the
acceptance flow.

### Reuse outside Opi

Rust hosts call the SDK directly. Other agents use
`command-execution-jsonl-v1`.

A Pi extension can implement Pi's `BashOperations` by translating a Pi request
to the neutral protocol and mapping events back to Pi results. It does not need
Opi, Opi configuration, or Opi package metadata.

The Opi package archive is only a distribution wrapper containing the
standalone executable, manifest, schemas, licenses, and required runtime
assets.

## `opi-sandbox` Restriction Contract

### Common profile

Initial policy:

```text
profile = workspace-write
network = deny | allow
```

`workspace-write` means:

- host reads remain unrestricted;
- host binaries and toolchains remain executable;
- writes, creates, removes, and renames are restricted to the canonical
  workspace and an invocation-owned temporary root;
- network policy is enforced separately.

The extension inherits the command environment needed for current local tool
behavior. It is not a confidentiality boundary for environment variables,
credentials, readable host files, or data later sent to a model provider.

The package reports `restricted`, never `isolated`.

### Linux

The Linux implementation:

- uses Landlock for filesystem mutation restriction;
- uses a fixed seccomp danger-syscall blocklist;
- for `network = deny`, blocks new INET, INET6, and NETLINK sockets, closes
  inherited nonessential descriptors, denies `io_uring` setup, and adds
  Landlock TCP bind/connect restrictions where supported;
- preserves AF_UNIX for ordinary local IPC;
- installs all required mechanisms before releasing the target-start gate;
- fails before target execution when the requested contract cannot be
  established.

Host reads remain allowed. Known limitations, including cooperating descriptor
transfer outside the non-malicious-command threat model, are reported without
upgrading the guarantee.

### macOS

The macOS implementation uses `sandbox-exec` with:

- host reads and execution allowed;
- writes denied outside workspace and invocation temporary roots;
- `(deny network*)` for `network = deny`;
- no syscall-filter claim.

The mechanism is labeled legacy/experimental. A missing or rejected
`sandbox-exec`, profile failure, or inability to prove in-profile target start
fails before target execution.

### Windows

Job Objects provide L0 supervision, not command restriction. Phase 16 has no
official Windows `opi-sandbox` artifact. Selecting an absent or target-mismatched
package fails before command execution.

Windows `local` continues to report only `supervised`.

## Failure and Diagnostics

Phase 16 uses the architecture-level `ExtensionFailure` envelope. Relevant
stable codes include:

- `package_not_installed`;
- `package_untrusted`;
- `contribution_disabled`;
- `policy_denied`;
- `permission_required`;
- `permission_denied`;
- `no_eligible_adapter`;
- `adapter_not_selected`;
- `adapter_unavailable`;
- `protocol_incompatible`;
- `protocol_violation`;
- `execution_failed`;
- `execution_timed_out`;
- `cleanup_unconfirmed`.

Interactive mode may intercept `permission_required` and present the
one-invocation/current-session grant prompt. Installation, trust, and
enablement failures only guide the user; they do not mutate state.

Text, TUI, NDJSON, RPC, package doctor, and top-level doctor preserve the same
codes and remediation fields. Public diagnostics omit command text,
environment values, credentials, unnecessary absolute paths, and raw backend
stderr.

Phase 16 uses each existing public surface's diagnostic/result envelope. Any
new public field follows that surface's schema-version rules; RPC, NDJSON, and
trace protocols are not migrated into `opi-protocol`.

No `degraded` success state exists. The adapter either reports its effective
contract or the command fails.

## Migration from Phase 15

The built-in Phase 15 native sandbox is unreleased and is replaced directly:

| Phase 15 | Phase 16 |
|---|---|
| Built-in native sandbox code | Optional `opi-sandbox` package |
| `[sandbox] mode = "off"` | `execution.strategy = "fixed"`, backend `local` |
| `[sandbox] mode = "strict"` | fixed backend `opi-sandbox` |
| `require = false` fail-open | Removed |
| `require = true` fail-closed | All selected external adapters fail closed |
| `fs`, `syscalls` | Package implementation details |
| `network = bool` | Package policy `network = "allow"|"deny"` |
| `--sandbox off|strict` | `--execution-backend local|ADAPTER-ID` |
| `--sandbox-require` | Removed |
| Project-local executable/process package | Rejected; install globally, review, and enable |

No compatibility aliases are added. Corrected L0 supervision remains in core.
Native confinement and its helper/capability-selection code leave core.

Historical Phase 15 technical claims, snapshots, and implementation-ledger
history remain immutable evidence of that phase. Explicit roadmap
cross-references in the Phase 15 design may be renumbered, and its documentation
guard may be adapted to distinguish archived Phase 15 evidence from current
Phase 16 product claims; neither action rewrites Phase 15 acceptance history.

## Testing and Acceptance

### Minimal Runtime

- Default startup constructs `LocalBashOperations` directly.
- An invalid or unreadable package-store sentinel is not touched when no
  extension is enabled.
- No extension process sentinel starts.
- No router, permission, or protocol task is created.
- The default `bash` schema matches the pre-extension schema.
- Local command results and L0 behavior remain unchanged.

### Package state and trust

- Install does not trust or enable.
- Trust does not grant Capability Permission.
- Enablement does not select an adapter.
- Selection does not grant permission.
- Disable and remove prevent execution.
- Lock or executable drift invalidates Package Trust.
- Project-local executable contributions are rejected.
- Reserved/colliding tool and adapter identities fail.
- Target, Opi version, path containment, executable, protocol, and hash gates
  fail closed.

### Routing and permission

- `fixed`, ordered `rules`, and `model` select only eligible adapters.
- Rules validate mode values, ordering, and final catch-all.
- Rule failure never falls through after selection.
- Model routing adds a required bounded `backend` enum.
- Fixed/rules/default modes do not add the field.
- `deny` is absent from model schemas.
- Interactive `ask` supports only one-invocation and current-session grants.
- Non-interactive, NDJSON, and RPC `ask` return `permission_required`.
- No model action can mutate install, trust, enablement, policy, or grants.
- No selected external failure retries through `local`.

Use `opi_ai::test_support::MockProvider`; tests never contact a real model.

### Protocol contract

Shared valid and invalid fixtures verify Rust types and the normative schema.
At least one non-Rust fixture client proves the protocol has no Rust- or
Opi-specific representation requirement.

A fake adapter proves:

- readiness precedes command disclosure;
- one process accepts one execution;
- the target cannot start before `started`;
- binary stdout/stderr round-trip;
- nonzero exit and signal remain in-band results;
- malformed, oversized, duplicate, unknown-required, and out-of-order frames
  fail;
- startup/handshake timeout, crash, EOF, and version mismatch fail closed;
- cancellation, timeout, and dropped futures kill the local backend tree;
- cleanup failure remains observable;
- diagnostics are redacted;
- no failure invokes `local`.

### L0 supervision

For local targets and external adapter processes:

- timeout kills child and descendants;
- cancellation kills child and descendants;
- dropping the execution future kills child and descendants;
- normal direct-child exit kills surviving background descendants;
- descendants holding output pipes cannot exceed bounded drain grace.

### SDK and direct CLI

SDK tests prove:

- no Opi configuration, package store, session, or process is required;
- sequential calls share no invocation state;
- cleanup removes invocation-owned temporary state;
- `run` and `backend --stdio` use the same policy and runner implementation.

The standalone CLI smoke suite described above is mandatory. It verifies:

- the isolated executable's `--help`, `--version`, and `doctor --json`;
- direct `run` argument, byte-output, stdin, exit, timeout, and setup-failure
  behavior;
- direct `backend --stdio` protocol behavior;
- operation without `opi` on `PATH`;
- absence of Opi configuration/session/package access;
- absence of durable cross-invocation state;
- the extracted release archive, not only a workspace binary.

### Native platform contract

Supported Linux runs verify workspace/temp writes, outside-write denial,
outside-read allowance, required Landlock/seccomp setup, network deny/allow,
reported limitations, and no silent mechanism loss.

Supported macOS runs verify the in-profile start gate, workspace/temp writes,
outside-write denial, outside-read allowance, network deny/allow, and
fail-closed missing/rejected `sandbox-exec`.

Windows runs verify unsupported doctor output, pre-start `run` refusal, absent
official package acceptance, and `local` reporting only `supervised`.

### Repository gates

- `cargo build -p opi-sandbox --bin opi-sandbox` succeeds independently.
- `cargo test -p opi-protocol` and `cargo test -p opi-sandbox` pass.
- `scripts/opi-sandbox-smoke.sh` or `scripts/opi-sandbox-smoke.ps1` passes
  against the isolated binary and extracted release artifact.
- All six target checks pass.
- Workspace tests and doctests pass.
- `cargo fmt --check --all` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- warning-free workspace documentation builds.
- English and Chinese product documentation change together.
- README, changelog, and platform claims match published artifacts.
- Phase 15 acceptance history remains immutable.

## Phase Integration

The normative implementation path is:

- Phase 15 remains archived Safety & Sandbox implementation history.
- Phase 16 becomes this Pluggable Extensions and Command Execution slice.
- later phase numbering is shifted as already approved.

Before implementation:

1. register this canonical spec as the reviewed Phase 16 source through the
   guarded Opi implementation workflow; keep the architecture document as a
   supporting rationale, not a second normative ledger source;
2. update `docs/opi-spec.md` and `docs/opi-spec.zh.md` together;
3. reconcile phase references and canonical source hashes without changing
   archived Phase 15 snapshots;
4. review the Phase 16 implementation plan against this canonical source after
   written spec approval.

Suggested plan order:

1. isolate and verify L0 supervision;
2. add `opi-protocol::execution::v1` and contract fixtures;
3. add disabled-install, Package Trust, enable/disable, and declarative adapter
   contribution state;
4. add Minimal Runtime selection and the `command.execute` router;
5. add fixed/rules/model routing, permission, and stable failures;
6. implement the execution protocol host and fake adapter suite;
7. remove native confinement from Opi core;
8. implement the standalone `opi-sandbox` SDK and CLI;
9. run direct CLI acceptance before Opi integration acceptance;
10. package and test native Linux/macOS artifacts;
11. update diagnostics, product documentation, and phase metadata.

## Follow-Ups

Separate designs may add:

- `opi-docker`, `opi-gondolin`, `opi-ssh`, or other adapters;
- more deterministic rule inputs;
- adapter composition;
- additional stable Capabilities;
- signed registries and package updates;
- project-level executable contributions;
- Windows AppContainer feasibility;
- read-confined profiles and toolchain allowlist UX;
- additional public SDK/session events for routing decisions.
