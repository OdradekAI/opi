# Phase 15 Remediation Plan

**Date**: 2026-07-27
**Audit sources**: `audit.gpt5-codex.md`, `audit.glm5.2.md`
**Commit range**: `11d4c28e8d3d4d0f85ff0d53f2bdf9795c95cf4c..d88980f8eb703ceb0ce22a39bad25f42fd21c80c`
**Design specs**: `docs/opi-spec.md`, `docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md`
**Authoritative correctives consulted**: `docs/research/2026-07-24-phase15-linux-l2-feasibility.md`, `docs/research/2026-07-24-project-trust-semantics-pi-claude-code-codex-cli.md`

---

## Audit cross-reference summary

Two independent reports produced 49 findings. They normalize into 37
behavioral clusters. Both auditors reported a cluster only where the underlying
behavior overlapped; findings that merely cited the same file remain separate.

| Cluster | Theme | Auditors | Consensus | Unified severity | Verification |
|---|---|---|---|---|---|
| C1 | Nested CWD bypasses ancestor context trust | gpt5-codex 2.1 | Unique (1/2) | Blocker | Confirmed |
| C2 | Fail-open drops engaged layers; macOS default strict and layer toggles do not produce the requested partial plan | gpt5-codex 2.2, gpt5-codex 3.6; glm5.2 3.1 | Full (2/2) | Blocker | Confirmed |
| C3 | Linux plan/ruleset construction failures bypass `require`, including ABI 1-3 FS loss and unsupported architectures | gpt5-codex 2.3; glm5.2 3.2, glm5.2 4.1 | Full (2/2) | Blocker | Confirmed |
| C4 | Landlock TCP capability has two divergent ABI predicates | glm5.2 3.5 | Unique (1/2) | Minor | Confirmed |
| C5 | Compiled BPF is cloned per spawn despite the shared-`Arc` contract | glm5.2 4.7 | Unique (1/2) | Minor | Confirmed |
| C6 | Linux `pre_exec` error handling allocates and errno translation is under-tested | gpt5-codex 3.7; glm5.2 5.5 | Full (2/2) | Major | Confirmed |
| C7 | `BashTool` discards backend sandbox diagnostics | gpt5-codex 3.1 | Unique (1/2) | Major | Confirmed |
| C8 | Release/debug production code honors L0 fault-injection environment variables | gpt5-codex 3.2; glm5.2 2.1 | Full (2/2) | Major | Confirmed |
| C9 | Degraded timeout/cancellation can wait forever on inherited pipes | gpt5-codex 3.3 | Unique (1/2) | Major | Confirmed |
| C10 | Clean shell exit does not enforce one cross-platform tree-lifecycle contract | gpt5-codex 3.4; glm5.2 4.3 | Full (2/2) | Major | Confirmed |
| C11 | Adapter L0 attach failures are swallowed | gpt5-codex 3.5; glm5.2 2.3 | Full (2/2) | Major | Confirmed |
| C12 | Public resource-loading APIs default to trusted and treat `Undecided` as trusted | gpt5-codex 3.8; glm5.2 4.6 | Full (2/2) | Major | Partially confirmed: standard CLI is preflighted; public/direct paths are not |
| C13 | Project discovery layers are identified by a `.opi` string prefix instead of provenance | glm5.2 3.3 | Unique (1/2) | Minor | Confirmed |
| C14 | Untrusted project package declarations are resolved before filtering | gpt5-codex 3.9 | Unique (1/2) | Major | Confirmed |
| C15 | Fresh-install persistence failures are silent; non-UTF-8 keys cannot round-trip | gpt5-codex 3.10; glm5.2 4.5 | Full (2/2) | Major | Confirmed |
| C16 | `WriteTool` probes the host filesystem after an injected backend error | gpt5-codex 3.11 | Unique (1/2) | Minor | Partially confirmed: abstraction violation is real; shipped local behavior is not presently incorrect |
| C17 | Windows strict emits three diagnostics; the associated unit-test name is misleading | gpt5-codex 3.12; glm5.2 5.2 | Full (2/2) | Major | Confirmed |
| C18 | Required strict+`require` test assumes every host must fail closed | gpt5-codex 3.13 | Unique (1/2) | Major | Confirmed |
| C19 | Atomic staging does not exclusively create its temporary file | gpt5-codex 4.1 | Unique (1/2) | Minor | Confirmed |
| C20 | Atomic staging/rename errors lose typed filesystem identity | gpt5-codex 4.2 | Unique (1/2) | Minor | Confirmed |
| C21 | Trust-prompt errors can leave raw/alternate-screen terminal state active | gpt5-codex 4.3 | Unique (1/2) | Minor | Confirmed |
| C22 | Trust acceptance and localized-document tests overstate their coverage | gpt5-codex 4.4; glm5.2 5.1 | Full (2/2) | Minor | Confirmed; glm5.2's README count was corrected to 4/5 |
| C23 | Construction-ownership guard misses most Phase 15 trust surface names | glm5.2 3.4 | Unique (1/2) | Minor | Confirmed |
| C24 | Layered sandbox precedence lacks an end-to-end test | gpt5-codex 4.5 | Unique (1/2) | Minor | Confirmed |
| C25 | Retained Linux artifact omits several native product assertions | gpt5-codex 4.6 | Unique (1/2) | Minor | Confirmed |
| C26 | SC1 cites the adapter test under the wrong test binary | gpt5-codex 4.7 | Unique (1/2) | Minor | Confirmed |
| C27 | Phase-exit artifact records a nonexistent start commit and the validator misses it | gpt5-codex 4.8 | Unique (1/2) | Minor | Confirmed |
| C28 | macOS probe places raw helper stderr/I/O text into diagnostic `reason` | glm5.2 2.2 | Unique (1/2) | Minor | Confirmed |
| C29 | `build_wrapped_argv` tests dead code rather than the production launcher path | glm5.2 4.8 | Unique (1/2) | Minor | Confirmed |
| C30 | macOS profile test comment states the opposite Seatbelt ordering rule | glm5.2 5.3 | Unique (1/2) | Minor | Confirmed |
| C31 | macOS network test description claims `socket()` denial but tests `bind()` denial | glm5.2 5.4 | Unique (1/2) | Minor | Confirmed |
| C32 | macOS module documentation still says the runtime is deferred | glm5.2 6.1 | Unique (1/2) | Minor | Confirmed |
| C33 | `--sandbox-require` documentation implies a two-way override | glm5.2 6.2 | Unique (1/2) | Minor | Confirmed |
| C34 | Conflicting trust flags are not tested through the public preflight | glm5.2 5.6 | Unique (1/2) | Info | Confirmed |
| C35 | `child.id().unwrap_or(0)` creates a latent process-group self-kill footgun | glm5.2 4.2 | Unique (1/2) | Minor | Partially confirmed: the fallback is unsafe, but Tokio currently returns `Some` on this path |
| C36 | Explicit `--config` can select project input before trust | glm5.2 2.4 | Unique (1/2) | Info | Confirmed behavior; accepted by design |
| C37 | Windows `fs4` locking does not serialize same-process writers | glm5.2 4.4 | Unique (1/2) | Minor | Refuted |

## Decision record

| ID | Finding cluster(s) | Decision | Rationale | Decided by |
|---|---|---|---|---|
| D1 | C1 | Use one shared ancestor-context candidate walk for trust detection and context loading; do not widen the harness/file-tool workspace to the Git root. | It closes the prompt-injection bypass while preserving the existing workspace boundary. | auto |
| D2 | C2, C3 | Represent requested, engaged, and unavailable layers separately. Backends receive the enabled/engaged set and return typed construction results; fail-open keeps every successfully engaged independent layer, while `require = true` refuses before spawn on any required-layer failure. | This is the normative “strongest engaged baseline” and the convergent audit recommendation. | auto |
| D3 | C4, C5 | Use one rights-derived TCP predicate and store the compiled BPF in an `Arc` outside the per-spawn closure. | Removes divergent capability truth and makes implementation match its reuse contract. | auto |
| D4 | C6 | Convert all child-side confinement failures to allocation-free stable errno values; add parent-side context only. | Heap allocation after `fork` is unsafe, even on an error path. | auto |
| D5 | C7, C11 | Preserve and convert every backend sandbox diagnostic into agent-facing diagnostics; retain adapter attach errors until the diagnostic store exists. | Diagnostics are part of the Phase 15 observable safety contract. | auto |
| D6 | C8 | Remove environment-driven production fault injection. Use an injected fault strategy/test constructor that is not activated by process environment, including debug builds. | `cfg(debug_assertions)` would leave ordinary debug binaries vulnerable. | auto |
| D7 | C9 | Use owned, abortable drain tasks with a bounded post-termination grace period. | Timeout/cancellation must complete even if a degraded descendant retains pipe handles. | auto |
| D8 | C10 | Terminate the remaining command process tree after the direct shell exits cleanly, while preserving its exit status. | Selected recommendation. It gives Unix and Windows one contract and satisfies the normative L0 claim. | user |
| D9 | C12, C13 | Require `ProjectStartupPlan` or an explicit decided trust state on public resource-loading constructors; remove implicit-trusted defaults, default-close `Undecided`, and structurally tag discovery-layer provenance. | Selected recommendation. It intentionally accepts a public API break instead of retaining a permissive compatibility path. | user |
| D10 | C14 | Make package resolution scope-aware and omit the project store entirely for untrusted startup. | “Skipped, not filtered” must hold before parsing, hashing, precedence, or errors. | auto |
| D11 | C15 | Create the config directory only when recording a durable decision, return persistence failures to the UI/startup caller, and return a named error for non-UTF-8 paths rather than changing the flat JSON schema. | Minimum fix that preserves the Phase 15 on-disk contract and never writes an unusable key. | auto |
| D12 | C16, C19, C20 | Keep every filesystem observation behind `FileOperations`; use exclusive temporary-file creation with collision retry and preserve typed I/O identity. | Restores the abstraction and atomic replacement contract without broadening it. | auto |
| D13 | C17 | Aggregate the Windows L1-L3 permanent platform gap into exactly one startup diagnostic. | The task DoD and public specification both promise one bounded startup event. | auto |
| D14 | C18, C22-C34 | Correct tests, CI, source documentation, paired public docs, and retained artifact validation to exercise the production behavior they claim. | These are additive or unambiguous truthfulness fixes. | auto |
| D15 | C21 | Use a step-aware RAII terminal guard. | Guarantees cleanup on every error return without duplicating cleanup branches. | auto |
| D16 | C35 | Reject PID 0 as an L0 attach failure and degrade diagnostically; do not panic. | Preserves the panic-free L0 policy and removes the latent negative-PGID hazard. | auto |
| D17 | C36 | No code change. Explicit `--config` remains user-authorized input. | This behavior is explicitly retained by the Phase 15 specification and acceptance tests. | auto |
| D18 | C37 | No change. | `fs4`/Windows two-handle locking does contend, and the repository concurrency test passes on Windows. | auto |

## Remediation layers

The workspace dependency graph is:

```text
Layer 1: opi-ai, opi-tui
Layer 2: opi-agent -> opi-ai
Layer 3: opi-coding-agent -> opi-ai, opi-agent, opi-tui
Layer 4: workspace acceptance/CI/artifact tooling
Layer 5: documentation and retained evidence
```

Phase 15 runtime findings are all owned by `opi-coding-agent`; Layers 1 and 2
therefore require no changes.

### Layer 3: `opi-coding-agent` product behavior

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --all-targets

#### Fix 3.1: Preserve partial strict engagement and enforce layer toggles

- **Audit source**: gpt5-codex 2.2, gpt5-codex 3.6; glm5.2 3.1
- **Cluster**: C2
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox.rs` ~L195-L205, ~L295-L395; `crates/opi-coding-agent/src/sandbox/linux.rs` ~L321-L380, ~L487-L524; `crates/opi-coding-agent/src/sandbox/macos.rs` ~L117-L149, ~L309-L327; `crates/opi-coding-agent/src/tool/operations.rs` ~L556-L567
- **Change**: Replace the all-or-nothing `Engaged`/`FailOpen` representation with requested, engaged, temporary-gap, and permanent-gap layer state. Pass the enabled/engaged set into backend construction, attach a partial plan under fail-open, and build Linux/macOS mechanisms only for enabled layers. Default macOS strict must retain L1/L2 while reporting its L3 gap.
- **Test plan**: Extend `sandbox_strict` and inline resolver tests for all-false, each single layer, mixed engaged/unavailable layers, ABI<4 filesystem retention, default macOS strict, and `require` true/false matrices.

#### Fix 3.2: Make Linux construction typed and fail-closed when required

- **Audit source**: gpt5-codex 2.3; glm5.2 3.2, glm5.2 4.1
- **Cluster**: C3
- **Decision**: D2
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox/linux.rs` ~L321-L380, ~L487-L524; `crates/opi-coding-agent/src/sandbox.rs` ~L386-L395; `crates/opi-coding-agent/src/tool/process_tree.rs` ~L450-L481; `crates/opi-coding-agent/src/tool/operations.rs` ~L556-L633
- **Change**: Replace `Option`, `.ok()?`, and `.ok()` construction erasure with typed per-layer results. Validate `TargetArch` before claiming engagement, skip empty Landlock network rights on ABI 1-3, and route filter/ruleset construction failures through the common `require` policy before command side effects.
- **Test plan**: Add injected unsupported-architecture, filter-build, Landlock-build, and ABI V1/V3 tests. Assert `require = true` never spawns and `require = false` retains independent seccomp/filesystem layers.

#### Fix 3.3: Unify Linux capability truth and actual BPF reuse

- **Audit source**: glm5.2 3.5, 4.7
- **Cluster**: C4, C5
- **Decision**: D3
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox/linux.rs` ~L236-L241, ~L362-L391, ~L508-L519
- **Change**: Implement TCP support through `!AccessNet::from_all(abi).is_empty()` everywhere. Wrap the compiled `BpfProgram` in `Arc` before constructing the reusable closure and clone only the `Arc` per spawn.
- **Test plan**: Table-test current/unsupported ABI capability equivalence and assert the production confinement path retains a shared BPF handle.

#### Fix 3.4: Remove allocation from the post-fork error path

- **Audit source**: gpt5-codex 3.7; glm5.2 5.5
- **Cluster**: C6
- **Decision**: D4
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/process_tree.rs` ~L445-L480; `crates/opi-coding-agent/src/sandbox/linux.rs` ~L194-L204; `crates/opi-coding-agent/tests/sandbox_linux_backend.rs` ~L179-L187
- **Change**: Return allocation-free errno-based `io::Error` values from `pre_exec`; move diagnostic formatting to the parent side. Audit the child closure for other locks/allocations.
- **Test plan**: Cover `Prctl`, `Seccomp`, `ThreadSync`, `Backend`, and empty-filter mappings, plus a confinement spawn-failure regression and a source guard excluding formatting in the child closure.

#### Fix 3.5: Deliver backend and adapter sandbox diagnostics

- **Audit source**: gpt5-codex 3.1, 3.5; glm5.2 2.3
- **Cluster**: C7, C11
- **Decision**: D5
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/operations.rs` ~L538-L567, ~L639-L768; `crates/opi-coding-agent/src/tool/bash.rs` ~L124-L231; `crates/opi-coding-agent/src/adapter_host.rs` ~L198-L244
- **Change**: Convert every non-context backend diagnostic into an agent `ToolDiagnostic` and append it on success, nonzero exit, timeout, cancellation, and backend error paths. Preserve adapter `AttachError` until host construction and then enqueue one redacted degradation diagnostic. Retain the adapter tree guard on both supported OS families for the host lifetime.
- **Test plan**: Add production `BashTool` tests with diagnostic-bearing success/error mocks and forced adapter attach failure tests that assert one exact `{layer, reason}` payload and fail-open continuation.

#### Fix 3.6: Make L0 fault injection non-production and tree teardown bounded

- **Audit source**: gpt5-codex 3.2, 3.3, 3.4; glm5.2 2.1, 4.3
- **Cluster**: C8, C9, C10
- **Decision**: D6, D7, D8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/process_tree.rs` ~L46-L54, ~L163-L254; `crates/opi-coding-agent/src/tool/operations.rs` ~L639-L723; `crates/opi-coding-agent/tests/sandbox_l0.rs`
- **Change**: Remove `OPI_TEST_L0_ATTACH_FAIL` and `OPI_TEST_L0_TERMINATE_FAIL` reads from runtime code and introduce an injected test fault strategy. Run stdout/stderr drains as abortable tasks with bounded grace after termination. On clean direct-child exit, terminate remaining group/job members before releasing L0 while preserving the direct exit status.
- **Test plan**: Add a release-built binary test proving both environment names are inert; injected attach/terminate-failure tests with descendants retaining both pipes; and Unix/Windows clean-exit marker tests proving descendants die while the tool reports exit 0.

#### Fix 3.7: Reject an unavailable child PID safely

- **Audit source**: glm5.2 4.2
- **Cluster**: C35
- **Decision**: D16
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/src/tool/operations.rs` ~L639; `crates/opi-coding-agent/src/adapter_host.rs` ~L203-L204; `crates/opi-coding-agent/src/tool/process_tree.rs` ~L176-L205
- **Change**: Remove `unwrap_or(0)`. Treat a missing/zero PID as a named L0 attach failure and continue only through the existing degraded path.
- **Test plan**: Assert `TreeGuard::attach(0)` never signals and produces a stable redacted reason; cover both bash and adapter callers.

#### Fix 3.8: Restore the `FileOperations` and atomic-write contracts

- **Audit source**: gpt5-codex 3.11, 4.1, 4.2
- **Cluster**: C16, C19, C20
- **Decision**: D12
- **Verification status**: Confirmed except C16's impact downgrade
- **File(s)**: `crates/opi-coding-agent/src/tool/write.rs` ~L146-L168, ~L218-L233; `crates/opi-coding-agent/src/tool/operations.rs` ~L59-L77, ~L416-L469
- **Change**: Classify `NotADirectory` through `FsOpError` and perform any ancestor metadata checks through the injected backend. Open staging files with `create_new(true)`, use a strong unique suffix with bounded collision retry, never follow a pre-existing sibling symlink, and map real staging/rename errors through the typed I/O mapper.
- **Test plan**: Use a virtual backend that disagrees with the host filesystem; add pre-existing regular-file/symlink collision tests and portable typed permission/not-found/not-directory mapping tests.

#### Fix 3.9: Make context trust and context loading inspect the same ancestors

- **Audit source**: gpt5-codex 2.1
- **Cluster**: C1
- **Decision**: D1
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/main.rs` ~L112-L115; `crates/opi-coding-agent/src/project_trust.rs` ~L409-L414, ~L639-L650; `crates/opi-coding-agent/src/context_files.rs` ~L47-L59; `crates/opi-coding-agent/src/harness.rs` ~L838-L843
- **Change**: Extract the exact context ancestor/candidate walk and use it both to enumerate trust-requiring context resources and to load them. Keep the harness workspace and file-tool root unchanged.
- **Test plan**: Add nested-CWD Git-root and no-Git ancestor-context cases. Headless startup must skip injection; interactive startup must ask; trusted startup must inject the same enumerated files.

#### Fix 3.10: Require explicit trust on every public resource-loading path

- **Audit source**: gpt5-codex 3.8; glm5.2 3.3, 4.6
- **Cluster**: C12, C13
- **Decision**: D9
- **Verification status**: Partially confirmed for C12; confirmed for C13
- **File(s)**: `crates/opi-coding-agent/src/harness.rs` ~L354-L381, ~L461-L528, ~L812-L843, ~L2164-L2178; `crates/opi-coding-agent/src/runner.rs` ~L111-L202; `crates/opi-coding-agent/src/rpc.rs` ~L127-L220; `crates/opi-coding-agent/src/runtime_packages.rs` ~L27-L42, ~L76-L80; `crates/opi-coding-agent/src/resource.rs` ~L127-L249
- **Change**: Change public builders/direct constructors to require `ProjectStartupPlan` or an explicit decided trust state. Remove implicit `Trusted` defaults, treat only `TrustDecision::Trusted` as trusted, and add a structural `DiscoveryLayerKind`/origin used by trust filtering.
- **Test plan**: Test direct builder, non-interactive, RPC, package runtime, explicit `Undecided`, custom project layers, and lookalike `.opi-backup` layers. Every path must default closed unless explicitly trusted.

#### Fix 3.11: Skip untrusted package stores before resolution

- **Audit source**: gpt5-codex 3.9
- **Cluster**: C14
- **Decision**: D10
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/runtime_packages.rs` ~L55-L84; `crates/opi-coding-agent/src/package_resolver.rs` ~L82-L155, ~L219-L245; `crates/opi-coding-agent/src/harness.rs` ~L2193-L2218
- **Change**: Add scope-aware package resolution. Untrusted startup constructs/resolves only the global store, so project parse failures and same-name precedence cannot suppress global packages.
- **Test plan**: Cover malformed project declarations with a valid global package, global/project same-name collisions, a blocked project marker adapter, an allowed global marker adapter, and harness fallback discovery.

#### Fix 3.12: Make durable trust persistence reliable and observable

- **Audit source**: gpt5-codex 3.10; glm5.2 4.5
- **Cluster**: C15
- **Decision**: D11
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/project_trust.rs` ~L330-L403, ~L688-L706; `crates/opi-coding-agent/src/interactive.rs` ~L80-L100
- **Change**: Create the user config directory inside `record` only for a durable choice, propagate record failures through `apply_ui_choice`/interactive startup, and return a named non-UTF-8-path error instead of serializing a lossy key.
- **Test plan**: Test Trust, Deny, and TrustParent from a nonexistent config root and reload the stored decisions; inject write/permission failure and assert visible error; on Unix, assert invalid-byte paths return the named error and do not write an unusable entry.

#### Fix 3.13: Guarantee terminal restoration on trust-prompt errors

- **Audit source**: gpt5-codex 4.3
- **Cluster**: C21
- **Decision**: D15
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/interactive.rs` ~L144-L188
- **Change**: Introduce a guard that records which raw/alternate-screen states were successfully entered and restores them best-effort in `Drop`.
- **Test plan**: Inject failures at alternate-screen entry, terminal construction, draw, poll, and read; assert exactly the applicable cleanup operations run.

#### Fix 3.14: Aggregate the Windows permanent strict gap

- **Audit source**: gpt5-codex 3.12; glm5.2 5.2
- **Cluster**: C17
- **Decision**: D13
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox.rs` ~L319-L345, ~L546-L577; `crates/opi-coding-agent/src/sandbox/windows.rs`; `crates/opi-coding-agent/tests/sandbox_strict.rs` ~L351-L459
- **Change**: Represent Windows L1-L3 as one permanent platform capability gap and emit exactly one startup diagnostic for either require mode. Rename/rewrite the misleading generic permanent-gap unit test.
- **Test plan**: Assert startup diagnostic count is one for `require = false` and true, repeated commands emit no duplicate unavailable diagnostic, and fail-open remains L0-only.

#### Fix 3.15: Sanitize macOS probe reasons and test the production launcher

- **Audit source**: glm5.2 2.2, 4.8, 5.3, 5.4, 6.1, 6.2
- **Cluster**: C28-C33
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/sandbox/macos.rs` ~L1-L38, ~L221-L327; `crates/opi-coding-agent/src/cli.rs` ~L124-L133; `crates/opi-coding-agent/tests/sandbox_strict.rs` ~L619-L629, ~L1123-L1147, ~L1263-L1281
- **Change**: Replace raw macOS probe stderr/I/O display with static bounded reasons; remove the dead wrapper-argv helper and test `Confinement::launcher` plus the real spawn composition; correct Seatbelt ordering, bind-denial, shipped-runtime, and one-way `--sandbox-require` documentation.
- **Test plan**: Inject a path/secret canary into probe failure text and assert it is absent; pin launcher prefix and verbatim shell args on the production composition path; retain profile ordering and bind-denial assertions with corrected descriptions.

### Layer 4: Workspace acceptance, CI, and artifact tooling

**Verification**:

    cargo fmt --all
    cargo clippy -p opi-coding-agent --all-targets -- -D warnings
    cargo test -p opi-coding-agent --test sandbox_config
    cargo test -p opi-coding-agent --test sandbox_l0
    cargo test -p opi-coding-agent --test sandbox_strict
    cargo test -p opi-coding-agent --test sandbox_linux_backend
    cargo test -p opi-coding-agent --test project_trust_startup
    cargo test -p opi-coding-agent --test project_trust_store
    cargo test -p opi-coding-agent --test trust_resource_gating
    cargo test -p opi-coding-agent --test phase15_safety_sandbox_docs
    cargo test -p opi-coding-agent --test artifact_audit_script

#### Fix 4.1: Make strict acceptance host-independent and retain full Linux evidence

- **Audit source**: gpt5-codex 3.13, 4.5, 4.6
- **Cluster**: C18, C24, C25
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-coding-agent/src/main.rs` ~L2567-L2658; `crates/opi-coding-agent/tests/sandbox_config.rs` ~L208-L252; `.github/workflows/ci.yml` ~L44-L96
- **Change**: Replace the “all hosts fail closed” assertion with an injected capability outcome or an independently verified engaged/fail-closed branch. Add one four-layer user/project/explicit/CLI precedence test. Expand the Linux `sandbox_product` matrix to retain filesystem denial, socket-family denial, AF_UNIX preservation, Landlock TCP, and alternate-surface audit results.
- **Test plan**: Run the named local tests above; on native Linux, require all named product filters to execute at least one test with zero failures/ignored tests and upload the complete log.

#### Fix 4.2: Make trust/documentation guards match their acceptance names

- **Audit source**: gpt5-codex 4.4; glm5.2 3.4, 5.1, 5.6
- **Cluster**: C22, C23, C34
- **Decision**: D14
- **Verification status**: Confirmed, with corrected glm5.2 assertion count
- **File(s)**: `crates/opi-coding-agent/tests/trust_resource_gating.rs` ~L42-L98, ~L270-L373, ~L439-L517; `crates/opi-coding-agent/tests/phase15_safety_sandbox_docs.rs` ~L68-L138, ~L316-L341; `crates/opi-coding-agent/tests/project_trust_startup.rs` ~L126-L163
- **Change**: Add project/global theme and extension fixtures plus a real marker adapter side-effect assertion. Expand construction-ownership checks to module paths and the complete trust surface. Mirror critical EN claims in ZH guards. Drive conflicting CLI flags through `prepare_project_startup` and assert validation occurs before registry/store/UI side effects.
- **Test plan**: Run the three affected integration binaries and mutation-style negative fixtures for every structural guard.

#### Fix 4.3: Validate commit references in retained artifacts

- **Audit source**: gpt5-codex 4.8
- **Cluster**: C27
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `scripts/opi-artifact-audit.py`; `crates/opi-coding-agent/tests/artifact_audit_script.rs`; `target/opi-artifacts/phase15-phase-exit/RUN_SUMMARY.md` ~L4
- **Change**: Teach the artifact audit to parse declared commit references and verify them with local Git object lookup. The validator must reject the existing `...a4a` typo and accept the real `...a4b` object. Regenerate the Phase 15 summary rather than silently editing only the preserved line.
- **Test plan**: Add synthetic valid/missing commit fixtures to `artifact_audit_script`; run the validator against the regenerated Phase 15 artifact and require `ok = true`.

### Layer 5: Documentation and retained evidence

**Verification**:

    cargo test -p opi-coding-agent --test phase15_safety_sandbox_docs
    python scripts/opi-artifact-audit.py target/opi-artifacts/phase15-phase-exit --workspace-root . --json

#### Fix 5.1: Correct the paired SC1 acceptance citation

- **Audit source**: gpt5-codex 4.7
- **Cluster**: C26
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `docs/opi-spec.md` ~L2011; `docs/opi-spec.zh.md` ~L1568
- **Change**: Replace `adapter_host_mock::adapter_process_group_contract` with `sandbox_l0::adapter_process_group_contract` in both localized specifications.
- **Test plan**: Run `phase15_safety_sandbox_docs` and execute the cited test filter to prove it selects the intended test.

#### Fix 5.2: Regenerate Phase 15 acceptance evidence after remediation

- **Audit source**: gpt5-codex 4.6, 4.8
- **Cluster**: C25, C27
- **Decision**: D14
- **Verification status**: Confirmed
- **File(s)**: `.github/workflows/ci.yml`; `target/opi-artifacts/phase15-phase-exit/*`
- **Change**: Rebuild the phase-exit artifact from the remediated commit, include the complete native Linux/macOS/Windows product evidence, use resolvable start/head commits, and rerun the strengthened artifact audit.
- **Test plan**: Require the local artifact audit to pass, all three native sandbox-product jobs to execute their named tests with zero failures/ignored tests, and all six target-check jobs to pass.

## Final verification

After every layer passes:

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS="-D warnings"; cargo doc --workspace --no-deps

Native acceptance additionally requires:

- Linux: all engaged FS, new-socket, AF_UNIX, Landlock TCP, and alternate-surface tests.
- macOS: default strict L1/L2 engagement plus the three production subprocess assertions.
- Windows: one permanent strict diagnostic, L0 fail-open, `require` fail-closed, and adapter/bash tree lifecycle.
- Cross-compilation: all six release triples from `.github/workflows/ci.yml`.

## Scope exclusions

| Finding | Status | Reason |
|---|---|---|
| glm5.2 2.4 / C36 | Info/No action | Explicit `--config` is intentionally user-authorized input and is retained by the normative spec. |
| glm5.2 4.4 / C37 | Refuted | Windows `fs4` two-handle exclusive locking contends; the repository concurrency test also passes on the current Windows host. |
| glm5.2 4.3 survivor-preservation recommendation | Refuted direction; underlying cluster fixed by C10 | The normative L0 contract takes precedence. The user selected command-tree termination on clean shell exit. |
| glm5.2 5.2 statement that three Windows diagnostics are correct | Refuted direction; underlying test mismatch fixed by C17 | The task DoD and public spec require one aggregated Windows startup diagnostic. |
| Adapter strict confinement | Deferred/out of scope | Phase 15 explicitly gives adapters L0 only; this plan repairs that L0 lifecycle but does not add L1-L3 adapter confinement. |
| Complete Linux network isolation | Deferred/out of scope | Inherited descriptors, non-TCP traffic, and `io_uring` remain documented residuals. |
| Remote/nav `Operations` backends | Deferred/out of scope | Phase 15 ships local read/write/edit/bash Operations only. |
