# Opi Pluggable Extension Architecture

Status: approved on 2026-07-28.

This document records the product-level rationale for optional Opi extensions.
The canonical Phase 16 specification is
`2026-07-28-phase16-pluggable-extension-command-execution-design.md`.

## Decision Summary

Opi keeps one standard binary with a minimal built-in runtime and dormant
extension-host code. Optional mechanisms such as host-native command
restriction, containers, VMs, remote execution, and future capabilities are
installed as independent Extension Packages.

The architecture uses a hybrid contribution model:

- different implementations of an existing stable Capability register as
  Capability Adapters;
- genuinely new capabilities register independent tools;
- extensions cannot replace a core tool by registering the same name.

For command execution, the stable model-callable tool remains `bash`.
`local`, `opi-sandbox`, future `opi-docker`, and future remote executors are
adapters for `command.execute`, not separate aliases for `bash`.

The standard binary includes package discovery, policy, routing, permission,
and process-host implementations. It does not link concrete external
extensions. When no extension is enabled, those implementations remain dormant
under the default fixed-local strategy, and the built-in local path incurs no
extension runtime.

## Motivation

Opi's Phase 15 implementation places native sandbox mechanisms inside the
coding-agent binary. That couples an optional platform-specific mechanism to
the core product and makes "sandbox" an umbrella term for concerns that are not
all sandboxes.

Pi 0.80.6 demonstrates the desired product shape: its core exposes replaceable
Operations seams, while sandbox, SSH, and Gondolin integrations live outside
the core. Opi adopts that optionality but does not copy Pi's dynamic same-name
tool replacement. Opi's process packages need explicit identity, trust,
compatibility, routing, permission, and failure contracts.

The resulting architecture must support all of these product modes:

- no extension installed or enabled: run directly on the local host;
- a user-selected fixed extension;
- deterministic user-defined routing;
- model selection among user-approved adapters;
- independent use of an extension through its SDK or CLI;
- reuse of the same extension by Pi or another agent.

## Goals

- Keep the standard Opi binary useful without any extension.
- Preserve a direct local fast path suitable for edge and constrained systems.
- Keep concrete extension implementations outside the Opi dependency graph.
- Make installation, code trust, enablement, invocation permission, and
  selection explicit.
- Let users, deterministic policy, or the model choose adapters without letting
  the model expand authority.
- Use declarative discovery that does not execute extension code.
- Provide stable, product-neutral wire contracts only where a process seam
  requires them.
- Let official extensions be independent SDK and CLI products.
- Fail closed after an external adapter has been selected.

## Non-Goals

- Dynamically loading Rust shared libraries.
- Defining one universal protocol for every kind of extension.
- Migrating RPC, NDJSON, trace, or `opi-extension-jsonl-v1` opportunistically.
- Implementing every future Capability or adapter in Phase 16.
- Allowing project configuration to install, trust, enable, or persistently
  authorize executable code.
- Proving publisher authenticity from an in-package hash.
- Automatically composing multiple execution adapters.
- Letting the model install packages, grant permissions, or edit User Policy.

## Domain Model

The canonical glossary is in the repository root `CONTEXT.md`.

### Separate state gates

An executable Extension Contribution is governed by five separate gates:

1. **Installed**: its artifact and manifest exist in the user package store.
2. **Trusted**: the user has established Package Trust for the exact locked
   artifact.
3. **Enabled**: the user has made its contributions available to normal runs.
4. **Selected**: the Capability Router has chosen it for an invocation.
5. **Permitted**: Capability Permission authorizes that invocation.

These are not one linear state machine. Installation, trust, and enablement are
package state; selection and permission are invocation state. A persistent
permission may exist before selection, while an `ask` permission is requested
after selection. Each gate still records a distinct decision:

- installation does not enable a package;
- enablement does not grant invocation permission;
- model selection does not grant permission;
- permission does not select an adapter;
- a changed locked executable invalidates Package Trust.

Selection may precede the final permission prompt for an `ask` candidate. A
selected but unpermitted invocation does not start extension code.

The built-in `local` adapter has no package installation or Package Trust
state. It remains subject to tool policy, User Policy, routing, and the
ordinary mutating-tool gate.

### Capability and tool contributions

A Capability is a stable action owned by the Opi host. A Capability Adapter
implements that action through a particular mechanism or destination.

Initial stable identity:

```text
command.execute
```

Core rejects an adapter contribution for an unknown Capability. Adding a new
stable Capability requires its own design because its request, result,
permission, and routing semantics form a new interface.

An extension that introduces a genuinely new action contributes a new tool.
Its tool name must not collide with a core tool or another enabled contribution.
Core tools cannot be shadowed or replaced by package load order.

This independent new-tool contribution syntax is a Phase 19 design topic.
Phase 16 implements only adapters for the existing `command.execute`
Capability and must not treat the conceptual declaration below as executable.

## Architectural Seams

```text
Model tool call
  -> stable Tool interface
  -> Capability Router
  -> built-in or external Capability Adapter

External process adapter
  -> protocol host
  -> independent extension CLI
  -> extension SDK implementation
```

Core owns:

- built-in tools and local adapters;
- package metadata resolution and lock validation;
- Package Trust and enablement state;
- User Policy and Capability Permission;
- the Capability Router;
- extension process supervision;
- protocol framing and validation;
- stable failure codes and redacted remediation.

An Extension Package owns:

- its mechanism-specific configuration;
- its implementation and platform dependencies;
- target setup and effective guarantees;
- invocation-scoped state and cleanup;
- its SDK and standalone CLI behavior.

`opi-protocol` owns:

- wire data types;
- protocol identifiers and negotiation types;
- framing codecs and bounded validation helpers;
- normative schemas and shared fixtures.

`opi-protocol` does not own package discovery, Package Trust, enablement, User
Policy, routing, permission prompts, process lifecycle, or an extension
implementation.

The Capability Router is product policy and initially remains inside the coding
agent. A separate host crate is not introduced until a second in-process
consumer makes that seam real.

## Minimal Runtime

The standard Opi binary contains dormant extension-host code so the same
artifact can become extensible through configuration. Concrete packages such
as `opi-sandbox` are not linked into it.

Normal startup uses two paths:

```text
no enabled contributions and default fixed-local routing
  -> construct built-in tools and LocalBashOperations directly
  -> no package-store scan
  -> no Capability Router runtime
  -> no permission-state runtime
  -> no protocol host or extension process

enabled contributions exist
  -> resolve only named enabled packages
  -> validate trust, lock, manifest, target, and compatibility
  -> construct the required router and host state
  -> start an extension only when its contribution is first invoked
```

`opi package list`, `opi package doctor`, and top-level `opi doctor` may scan
the complete package store because the user explicitly requested diagnostics.
An installed but disabled package has no effect on an ordinary default agent
run. Explicit non-default extension or routing configuration may require host
validation even when it ultimately names only `local`.

The product claim is **zero extension runtime overhead**, not absolute zero
cost. The dormant host contributes binary size and startup checks whether the
enabled set is empty. The Minimal Runtime performs no extension directory
scan, process launch, background task creation, protocol initialization,
dynamic routing, or permission-state allocation.

## Declarative Contribution Manifest

Extension Contributions are declared as data. Discovery never executes package
code.

Conceptual adapter declaration:

```toml
[[contributions.adapters]]
capability = "command.execute"
id = "opi-sandbox"
transport = "process-jsonl"
protocol = "command-execution-jsonl-v1"
command = "bin/opi-sandbox"
args = ["backend", "--stdio"]
target = "x86_64-unknown-linux-gnu"
sha256 = "<locked executable digest>"
```

Conceptual new-tool declaration:

```toml
[[contributions.tools]]
name = "example_tool"
transport = "process-jsonl"
protocol = "opi-extension-jsonl-v1"
command = "bin/example-provider"
```

The concrete schema may share package-level target and compatibility fields,
but it must preserve these invariants:

- adapter identity is unique within its Capability;
- tool names cannot shadow core or enabled tool names;
- command paths are relative, canonicalized, and contained by the package;
- target, package compatibility, protocol, and executable hash are hard gates;
- extension-specific configuration remains namespaced and bounded;
- discovery reads declarations only;
- a declaration cannot grant itself Package Trust or permission.

Package names describe distributions, not capabilities. Capability membership
comes only from manifest data. The official package and adapter are both named
`opi-sandbox`, but no `opi-*` prefix is required for third-party packages.

## Package Trust and Scope

Executable packages are trusted code. The extension process initially runs
with the user's operating-system authority even if it later restricts a child
command. Its declared permissions are not a sandbox around the extension
itself.

Initial policy:

- executable contributions are installable only in the user-global package
  store;
- project-local packages may contribute static resources such as skills,
  prompt fragments, and themes, but not executable adapters or tools;
- a trusted project may request a globally available adapter, but cannot
  install, trust, or enable it;
- installing an executable package records its artifact but leaves its
  contributions disabled;
- first enablement requires an explicit user trust decision;
- disabling a package prevents execution but may retain trust for the same
  locked artifact;
- artifact or lock drift invalidates trust and fails closed;
- removal deletes its enablement and trust records.

SHA-256 and package locks detect drift. They do not establish publisher
identity. Signed registries, update workflows, and project-level executable
packages require a separate design.

## Routing and Permission

### Routing strategies

The Capability Router supports:

- `fixed`: User Policy names exactly one adapter;
- `rules`: ordered deterministic rules choose an adapter from host-known facts
  and explicit structured invocation metadata;
- `model`: the tool schema exposes eligible adapter identities and the model
  selects one per call.

Rules must not infer safety by heuristically parsing a shell-command string.
Each Capability design defines the structured fields that its rules may match.

For `model`, the adapter field appears only when model routing is active. It is
required, its enum contains only eligible candidates, and its descriptions may
state placement, effective contract, and whether interactive approval is
required.

### User Policy decisions

User Policy assigns each candidate one decision:

- `deny`: exclude it from routing and model-visible schemas;
- `ask`: allow selection, then require an interactive Capability Permission;
- `allow`: authorize invocation through explicit persistent user
  configuration.

Interactive `ask` grants have only two scopes:

- one invocation;
- the current session.

The current-session grant is memory-only. It is not written to session JSONL
and does not survive process restart, resume, or fork.

Persistent grants are written only through an explicit user configuration
action. Non-interactive, NDJSON, and RPC runs cannot prompt; selecting an `ask`
candidate returns a structured `permission_required` failure.

The model cannot:

- install or remove packages;
- establish or revoke Package Trust;
- enable or disable contributions;
- change Routing Strategy;
- change `deny`, `ask`, or `allow`;
- grant a one-shot, session, or persistent permission;
- invoke a hidden authorization tool.

Once an external adapter is selected, missing permission, unavailability,
protocol failure, execution failure, timeout, or cleanup failure never causes
an implicit retry through `local`.

## Modular Shared Protocol

The shared crate is named `opi-protocol`, but it is not a universal plugin
protocol. It contains independently versioned modules added only after a
concrete process seam has at least two sides.

Phase 16 adds only:

```text
opi_protocol::execution::v1
wire id: command-execution-jsonl-v1
```

The Cargo crate version and the wire identifier are independent. A crate
release never silently changes the meaning of an existing wire id.

Static resources use no protocol. In-process Rust embedders may use Rust
interfaces. Existing long-running tool adapters may continue using
`opi-extension-jsonl-v1`. RPC, NDJSON, trace envelopes, and other protocols are
not moved into `opi-protocol` without separate schema reviews.

Protocol modules are feature- or module-scoped so a command-execution consumer
does not need unrelated contracts added later.

## Extension Failure Interface

All host surfaces render the same structured failure:

```text
ExtensionFailure
  code
  phase
  capability
  adapter
  retryable
  user_action_required
  remediation[]
  redacted_details
```

Context fields such as capability and adapter may be absent when a failure is
reported before contribution resolution. Codes and phase meanings remain
stable across surfaces.

Stable initial codes include:

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

TUI, text, NDJSON, RPC, package doctor, and top-level doctor may render
different presentation but do not invent different semantics. Policy and
permission failures are not retryable. Only explicitly transient adapter
failures may be marked retryable.

Phase implementations use each surface's existing diagnostic/result envelope.
If a new public wire field is required, that surface follows its own schema
versioning rules; its protocol is not folded into `opi-protocol`.

Failures and remediation must not expose command text, environment values,
credentials, raw extension stderr, or unnecessary absolute paths. Unconfirmed
cleanup remains visible and is never rewritten as successful cancellation.

## Delivery Slices

This architecture is broader than one implementation phase.

### Phase 16 vertical slice

Phase 16 proves one complete path:

```text
Capability: command.execute
Tool: bash
Adapters: local and opi-sandbox
Protocol: opi-protocol::execution::v1
```

It implements the Minimal Runtime fast path, executable package trust and
enablement needed by this slice, routing and permission for
`command.execute`, the one-shot protocol, and the independent `opi-sandbox`
SDK/CLI/package.

### Separate future designs

Future work may add:

- Docker, VM, SSH, and remote command adapters;
- additional stable Capabilities;
- broader deterministic rule inputs;
- independent new-tool contribution redesign;
- signed registries and update workflows;
- project-level executable packages;
- adapter composition;
- Windows AppContainer or restricted-token command restriction;
- additional `opi-protocol` modules.

Those efforts reuse this architecture but receive separate specs and plans.

## Architecture Acceptance

The implementation must prove:

- with no enabled extension and default fixed-local routing, ordinary startup
  does not inspect an intentionally invalid package store or start a sentinel
  extension process;
- the `bash` schema and local execution path are unchanged in the default
  `local + fixed` mode;
- the Opi binary dependency graph contains no concrete external package;
- `opi-sandbox` depends on `opi-protocol`, not `opi-agent` or
  `opi-coding-agent`;
- installation, Package Trust, enablement, selection, and permission tests fail
  independently;
- executable project contributions, core-tool shadowing, duplicate adapters,
  incompatible targets, and lock/hash drift fail closed;
- `fixed`, `rules`, and `model` cannot escape User Policy;
- `deny` candidates are not model-visible and `ask` cannot execute without a
  user grant;
- no model-facing path can mutate trust, enablement, routing, or permission;
- all user surfaces preserve stable failure codes and redaction;
- no external-adapter failure retries through `local`.

The Phase 16 spec adds the command-protocol, native sandbox, SDK, and standalone
CLI acceptance criteria.
