# Opi

Opi is a terminal-first AI agent toolkit. This glossary records the domain
language used to describe product architecture, evidence and change control,
extension runtime, command execution, and safety boundaries.

## Product architecture

**Agent Core**:
The product-neutral model and agent runtime semantics that remain valid across
harnesses, user interfaces, and products.
_Avoid_: Opi core, workspace core

**Reference Product**:
A first-party Opi product that assembles the Agent Core with product-specific
interaction, defaults, and distribution policy.
_Avoid_: Agent Core, core harness

**Extension Ecosystem**:
The optional packages and contributions that extend a Reference Product without
becoming Agent Core requirements.
_Avoid_: Optional core, built-in ecosystem

**Independent Companion**:
An independently versioned, agent-neutral product that can interoperate with
Opi but is not required by the Minimal Runtime.
_Avoid_: Opi core package, built-in companion

**Standalone Project**:
An Independent Companion whose mission, governance, releases, and public
identity remain complete without Opi.
_Avoid_: Independent crate, external package

**Placement Review**:
An evidence-backed decision to keep or move a capability among the Agent Core,
Reference Product, Extension Ecosystem, and independent product layers.
_Avoid_: Graduation, promotion to core

## Evidence and change control

**Evidence Producer**:
An independent role that creates immutable, reproducible evaluation evidence
without proposing or approving a runtime change.
_Avoid_: Evaluator approval, learning judge

**Candidate Producer**:
A role that derives a versioned change candidate from finalized evidence but
cannot activate the candidate or alter its evidence.
_Avoid_: Self-approving learner, promotion worker

**Promotion Controller**:
The policy-enforcing owner of staged candidate activation and rollback. It
cannot rewrite evidence, candidates, or the policy that authorizes it.
_Avoid_: Learning worker, deployment approver

**Human Authority**:
The human owner of User Policy, risk classification, automation scope, and
approval requirements for runtime changes.
_Avoid_: Model approval, evaluator authority

**External Knowledge Sync**:
The replication of verified Source-of-Record state under provenance, revision,
permission, and withdrawal rules. It is not learning.
_Avoid_: Continual Learning, knowledge generation

**Continual Learning**:
The derivation of evaluated candidates from finalized experience without giving
the derivation process authority to activate them.
_Avoid_: External Knowledge Sync, online self-modification

**Activation Class**:
A Human-Authority-owned risk category that sets the maximum automated lifecycle
actions permitted for a change candidate.
_Avoid_: Confidence score, evaluation grade

**Runtime Input Binding**:
The immutable provenance binding for the material runtime inputs used by one
run. It is either a Direct Runtime Input assembled without Promotion authority
or an Active Snapshot selected by the Promotion Controller.
_Avoid_: Active configuration, mutable runtime state

**Direct Runtime Input**:
The immutable, digest-addressed runtime inputs assembled directly by a product
or embedder when no Promotion Controller selected them. It must not be described
as an Active Snapshot.
_Avoid_: Active Snapshot, promoted configuration

**Active Snapshot**:
The immutable set of versioned runtime inputs selected for new work by the
Promotion Controller. Existing work remains bound to its original snapshot.
_Avoid_: Latest configuration, mutable active state

**Control Baseline**:
A frozen Active Snapshot and execution manifest used for a paired comparison
with one change candidate.
_Avoid_: Latest run, external leaderboard score

**Promotion Lifecycle**:
The ordered offline-candidate, shadow, opt-in-canary, active, rejection, and
rollback states enforced without skipping evidence or approval gates.
_Avoid_: Deployment status, learning confidence

**Delegated Promotion Policy**:
A time-bounded Human Authority grant that fixes the candidate types, scope,
evidence gates, budgets, and limits within which promotion may be automated.
_Avoid_: Model permission, self-approved policy

**Controlled Self-Iteration**:
The policy-bounded lifecycle in which a behavior candidate is independently
evaluated, staged, activated, monitored, and rolled back without per-candidate
human approval or any authority to widen its own policy.
_Avoid_: Continual Learning, online self-modification, autonomous authority

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
