# Opi

Opi is a terminal-first AI agent toolkit. This glossary records the domain
language used to describe extension runtime, command execution, and safety
boundaries.

## Extension runtime

**Minimal Runtime**:
The operating state of the standard Opi distribution when no extension is
enabled and built-in local capabilities use their direct default path.
Extension host code may be present but no extension runtime is active.
_Avoid_: Minimal build, extension-free binary

**User Policy**:
The user-owned hard limits on which enabled extension capabilities may
participate. A model decision cannot broaden or weaken these limits.
_Avoid_: Model preference, routing hint

**Capability Permission**:
A user grant that authorizes an enabled extension capability for a defined
scope. The model may request this permission but cannot grant it.
_Avoid_: Extension enabled, model approval

**Permission Scope**:
The lifetime of a Capability Permission: one invocation, the current in-memory
running harness session, or an explicit persistent User Policy entry. An
in-memory session grant never survives process restart, session resume, or
fork.
_Avoid_: Permanent model approval

**Extension Package**:
An independently installed distribution that may contribute Capability
Adapters or new tools. Installation alone does not enable, authorize, or select
its contributions.
_Avoid_: Built-in extension, enabled plugin

**Package Trust**:
The user's authorization for Opi to execute code from an Extension Package. It
is distinct from permission to invoke a capability provided by that package.
_Avoid_: Capability Permission, installed state

**Extension Contribution**:
A Capability Adapter or new tool declared by an Extension Package without
executing package code.
_Avoid_: Dynamically registered plugin code

**Capability**:
A stable action Opi can offer independently of the implementation that performs
it.
_Avoid_: Extension feature, backend type

**Capability Adapter**:
A built-in or extension-provided implementation of one Capability.
_Avoid_: Capability, extension tool

**Capability Router**:
The policy-enforcing selector that resolves one invocation to an allowed
Capability Adapter.
_Avoid_: Model router, backend fallback

**Routing Strategy**:
The user-selected method by which the Capability Router chooses an adapter:
fixed selection, deterministic rules, or model choice within User Policy.
_Avoid_: Model policy, backend mode

## Command execution

**Execution Backend**:
A Capability Adapter for model-requested shell-command execution. Local
execution, a native command sandbox, a container, a VM, and a remote executor
are different execution backends.
_Avoid_: Sandbox backend as the umbrella term

**Command Sandbox**:
An execution backend that runs commands on the host while restricting their
capabilities through operating-system policy. It is not the general term for
containers, VMs, or remote execution.
_Avoid_: Isolation for host-native defense-in-depth

**Supervision**:
Lifecycle control that applies timeout, cancellation, and process-tree cleanup
without restricting what a command may access.
_Avoid_: Sandbox, isolation
