# Opi domain language

Definitions used by current engineering conversations. These define meaning,
not a roadmap, a fixed crate assignment or proof a capability is implemented.
For changes, follow [domain rules](agents/domain.md).

## Product and runtime

**Agent Core**:
Product-neutral model and agent runtime semantics that remain valid across
harnesses, user interfaces and products.
_Avoid_: workspace core, core harness

**Reference Product**:
A first-party product assembling the Agent Core with product-specific
interaction, defaults and distribution policy.
_Avoid_: Agent Core

**Extension Ecosystem**:
Optional packages and contributions extending a product without becoming
requirements of its Agent Core.
_Avoid_: optional core

**Independent Companion**:
An agent-neutral product that has value without Opi and can interoperate with
it through an explicit contract.
_Avoid_: built-in companion, Opi core package

**Minimal Runtime**:
The operating state of the standard distribution with no extension enabled
and built-in local capabilities on their direct default path.
_Avoid_: extension-free binary, minimal build

## Authority and extensions

**User Policy**:
User-owned limits on which capabilities may participate. A model choice can
operate within these limits, not grant or widen them.
_Avoid_: model preference

**Capability Permission**:
A user grant authorizing an enabled extension capability for a defined scope.
_Avoid_: extension enabled, model approval

**Package Trust**:
The user's authorization to execute code from an extension package, distinct
from permission to invoke a capability it provides.
_Avoid_: installed state, Capability Permission

**Extension Package**:
An independently installed distribution contributing adapters or tools;
installation does not itself imply trust, enablement, selection or permission.
_Avoid_: enabled plugin

**Capability**:
An action the product can offer independently of the implementation performing it.
_Avoid_: backend type

**Capability Adapter**:
A built-in or extension-provided implementation of one capability.
_Avoid_: Capability

**Capability Router**:
The policy-enforcing selector resolving an invocation to an allowed adapter.
_Avoid_: model router, backend fallback

## Command execution

**Execution Backend**:
An adapter for model-requested shell-command execution; local execution,
command sandboxing, containers and remote execution are distinct choices.
_Avoid_: sandbox as the umbrella term

**Command Sandbox**:
An execution backend running commands on the host while restricting their
capabilities through operating-system policy.
_Avoid_: process supervision, generic remote execution

**Supervision**:
Lifecycle control applying timeout, cancellation and process-tree cleanup
without restricting what a command may access.
_Avoid_: sandbox, isolation
